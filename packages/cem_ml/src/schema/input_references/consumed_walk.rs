//! One traversal for selected structure, attribute chains and authorized source
//! subtrees. Tags are consumption roles over original handles, never AST nodes.
use super::*;
use crate::schema::attribute_references::{
    has_unconsumed_constraints, ConsumedAttributeValue, NativeAttributeTargetAccess,
};
use crate::schema::reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy};
use crate::value::reference_resolution::{
    resolve_consumer_structure_with_stage, ReferenceLinkEvaluation, ReferenceResolution,
    ReferenceResolutionIssue, ReferenceResolutionIssueKind,
};

use std::collections::HashMap;

#[derive(Clone, Copy)]
enum Role {
    Structural(Option<usize>, usize),
    Value(usize),
    Expression(usize),
    Container(usize),
    Authored(usize),
}
#[derive(Clone)]
struct Tagged<N> {
    node: N,
    role: Role,
}
struct Host<'a, H> {
    inner: &'a mut H,
    started: std::cell::RefCell<HashSet<usize>>,
    denied_roles: std::cell::RefCell<Vec<Role>>,
}
impl<H: InputReferenceHost> ReferenceResolutionHost for Host<'_, H> {
    type Node = Tagged<H::Node>;
    type Scope = H::Scope;
    fn prepare_node(&mut self, node: &mut Self::Node) -> Result<(), ReferenceResolutionError> {
        // Navigable attribute targets retain authored descendant references.
        if !matches!(node.role, Role::Authored(_)) {
            self.inner.prepare_node(&mut node.node)?;
        }
        Ok(())
    }
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.inner.scope(&node.node)
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.inner.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        if matches!(node.role, Role::Authored(_) | Role::Container(_)) {
            None
        } else if matches!(node.role, Role::Expression(_)) {
            self.inner
                .retained_node(&node.node)
                .as_ref()
                .and_then(native_attribute_expression)
        } else {
            self.inner.reference_occurrence(&node.node)
        }
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.inner.unresolved_policy(&node.node)
    }
    fn permits_edge(&self, reference: &Self::Node, target: &Self::Node) -> bool {
        let allowed = self.inner.permits_edge(&reference.node, &target.node);
        if !allowed {
            self.denied_roles.borrow_mut().push(target.role);
        }
        allowed
    }
    fn evaluate(&mut self, reference: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        if let Role::Value(site) | Role::Expression(site) = reference.role {
            self.started.borrow_mut().insert(site);
        }
        let (evaluated, role) = if let Role::Expression(site) = reference.role {
            (
                self.inner.evaluate_input_expression(&reference.node),
                Role::Value(site),
            )
        } else {
            (self.inner.evaluate(&reference.node), reference.role)
        };
        match evaluated {
            ReferenceLinkEvaluation::Resolved(nodes) => ReferenceLinkEvaluation::Resolved(
                nodes
                    .into_iter()
                    .map(|node| Tagged { node, role })
                    .collect(),
            ),
            ReferenceLinkEvaluation::Pending(reason) => ReferenceLinkEvaluation::Pending(reason),
            ReferenceLinkEvaluation::Unresolved(reason) => {
                ReferenceLinkEvaluation::Unresolved(reason)
            }
            ReferenceLinkEvaluation::Invalid(diagnostics) => {
                ReferenceLinkEvaluation::Invalid(diagnostics)
            }
        }
    }
}
struct Site {
    owner: Option<usize>,
    attribute: SchemaDeclarationNode,
    element: String,
    supported: bool,
    model: usize,
}
pub(super) struct ConsumedWalk<N> {
    pub structure: ReferenceStructureResolution<N>,
    pub attributes: Vec<(Option<usize>, ConsumedAttributeValue)>,
    pub models: Vec<Option<usize>>,
}
/// Directly authored attribute requests and structurally selected attributes use
/// exactly this walk. The latter keep the enclosing active identities/budgets.
pub(super) fn walk<H: InputReferenceHost>(
    root: H::Node,
    model: &SchemaDocumentModel,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    attribute: Option<(SchemaDeclarationNode, String)>,
) -> Result<ConsumedWalk<H::Node>, ReferenceResolutionError> {
    let mut models = RegionModels::new(model, &[]).expect("no region descriptors");
    walk_regions_scheduled(
        root,
        &mut models,
        0,
        host,
        limits,
        attribute,
        &mut |_: &mut H, _: SchemaDeclarationNode, _: ReferenceTraversalLimits| Ok(None),
    )
}
pub(super) fn walk_regions_scheduled<H, D>(
    root: H::Node,
    models: &mut RegionModels<'_>,
    model_index: usize,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    attribute: Option<(SchemaDeclarationNode, String)>,
    scheduler: &mut D,
) -> Result<ConsumedWalk<H::Node>, ReferenceResolutionError>
where
    H: InputReferenceHost,
    D: FnMut(&mut H, SchemaDeclarationNode, ReferenceTraversalLimits)
        -> Result<Option<DiscoveredInputSchemaRegion>, ReferenceResolutionError>,
{
    let mut sites = vec![];
    let role = if let Some((attribute, element)) = attribute {
        sites.push(Site {
            owner: None,
            attribute,
            element,
            supported: true,
            model: model_index,
        });
        if matches!(
            host.retained_node(&root).as_ref().map(|node| node.node()),
            Some(CemAstNode::Attribute { .. })
        ) {
            Role::Container(0)
        } else {
            value_role(host, &root, 0)
        }
    } else {
        Role::Structural(None, model_index)
    };
    let mut tagged_host = Host {
        inner: host,
        started: Default::default(),
        denied_roles: Default::default(),
    };
    let mut next_index = 0;
    let mut invalid_sites = HashSet::new();
    let mut blocked_positions = HashSet::new();
    let mut region_diagnostics = vec![];
    let mut effective_models = HashMap::new();
    let mut structural_sources = HashMap::new();
    let origin = if matches!(role, Role::Container(_)) {
        let source = tagged_host.inner.retained_node(&root).unwrap();
        Some(ReferenceOccurrence {
            identity: source.identity(),
            node_id: Some(source.node_id()),
            expression: None,
            source_map: document_model::source_stack_for_node(source.node()).clone(),
        })
    } else {
        None
    };
    let mut resolved = resolve_consumer_structure_with_stage(
        Tagged { node: root, role },
        &mut tagged_host,
        limits,
        origin,
        |host, entry| {
            let position = next_index;
            next_index += 1;
            // Unsupported targets still reach original-node admission below.
            // Retain their incoming model metadata instead of indexing a missing
            // entry before the existing attributed invalid-target diagnostic.
            if let Role::Structural(_, model) = entry.role {
                effective_models.insert(position, Some(model));
            }
            let Some(retained) = host.inner.retained_node(&entry.node) else {
                return Ok(None);
            };
            match entry.role {
                Role::Structural(parent, current_model) => {
                    let effective = models.selected_child_model(
                        current_model,
                        &retained,
                        parent.and_then(|parent| structural_sources.get(&parent)),
                    );
                    structural_sources.insert(position, retained.clone());
                    effective_models.insert(position, effective);
                    let Some(current_model) = effective else {
                        blocked_positions.insert(position);
                        return Ok(Some(vec![]));
                    };
                    models.discover(&retained, host.inner, limits, scheduler)?;
                    let model = models.model(current_model);
                    let (children, child_model, complete) =
                        models.children(current_model, &retained);
                    if !complete {
                        blocked_positions.insert(position);
                        region_diagnostics.extend(models.blockers(&retained));
                    }
                    let mut nodes: Vec<_> = children
                        .iter()
                        .filter_map(|id| {
                            SchemaDeclarationNode::new(retained.document().clone(), *id)
                        })
                        .map(|node| Tagged {
                            node: host.inner.source_node(node),
                            role: Role::Structural(Some(position), child_model),
                        })
                        .collect();
                    if let CemAstNode::Element { attributes, .. } = retained.node() {
                        let Some(expanded_name) =
                            host.inner.input_expanded_name(&retained).cloned()
                        else {
                            blocked_positions.insert(position);
                            return Ok(Some(nodes));
                        };
                        let Some(element) = model.elements.get(&expanded_name.local_name) else {
                            return Ok(Some(nodes));
                        };
                        for id in attributes {
                            if models
                                .control_attributes(&retained)
                                .is_some_and(|attributes| attributes.contains(id))
                            {
                                continue;
                            }
                            let Some(attribute) =
                                SchemaDeclarationNode::new(retained.document().clone(), *id)
                            else {
                                continue;
                            };
                            let CemAstNode::Attribute { value_nodes, .. } = attribute.node() else {
                                continue;
                            };
                            let Some(name) = host.inner.input_expanded_name(&attribute).cloned()
                            else {
                                blocked_positions.insert(position);
                                continue;
                            };
                            if value_nodes.is_empty() {
                                continue;
                            }
                            let supported = element
                                .allows_attribute(&name.namespace_uri, &name.local_name)
                                && model
                                    .attributes
                                    .get(&name.local_name)
                                    .is_some_and(|contract| contract.is_node_valued());
                            let reference = if supported {
                                value_request_root(&attribute)
                            } else {
                                None
                            };
                            let site = sites.len();
                            sites.push(Site {
                                owner: Some(position),
                                attribute: attribute.clone(),
                                element: expanded_name.local_name.clone(),
                                supported: reference.is_some(),
                                model: current_model,
                            });
                            if let Some(reference) = reference {
                                let role =
                                    if matches!(reference.node(), CemAstNode::Attribute { .. }) {
                                        Role::Container(site)
                                    } else if native_attribute_expression(&reference).is_some() {
                                        Role::Expression(site)
                                    } else {
                                        Role::Value(site)
                                    };
                                nodes.push(Tagged {
                                    node: host.inner.source_node(reference),
                                    role,
                                });
                            }
                        }
                    }
                    Ok(Some(nodes))
                }
                Role::Container(site) => {
                    host.started.borrow_mut().insert(site);
                    let CemAstNode::Attribute { value_nodes, .. } = retained.node() else {
                        unreachable!()
                    };
                    let nodes: Vec<_> = value_nodes
                        .iter()
                        .filter_map(|id| {
                            SchemaDeclarationNode::new(retained.document().clone(), *id)
                        })
                        .collect();
                    if nodes.len() != value_nodes.len()
                        || value_nodes.iter().copied().collect::<HashSet<_>>().len()
                            != value_nodes.len()
                    {
                        invalid_sites.insert(site);
                    }
                    Ok(Some(
                        nodes
                            .into_iter()
                            .map(|source| {
                                let role = if native_attribute_expression(&source).is_some() {
                                    Role::Expression(site)
                                } else {
                                    Role::Value(site)
                                };
                                Tagged {
                                    node: host.inner.source_node(source),
                                    role,
                                }
                            })
                            .collect(),
                    ))
                }
                Role::Value(site) | Role::Expression(site) | Role::Authored(site) => {
                    let edges = owning_edges(retained.node());
                    if edges
                        .iter()
                        .any(|id| retained.document().get(*id).is_none())
                        || matches!(retained.node(), CemAstNode::Element{attributes,..} if attributes.iter().any(|id| !matches!(retained.document().get(*id), Some(CemAstNode::Attribute{..}))))
                    {
                        invalid_sites.insert(site);
                    }
                    Ok(Some(
                        edges
                            .into_iter()
                            .filter_map(|id| {
                                SchemaDeclarationNode::new(retained.document().clone(), id)
                            })
                            .map(|node| Tagged {
                                node: host.inner.source_node(node),
                                role: Role::Authored(site),
                            })
                            .collect(),
                    ))
                }
            }
        },
    )?;
    // Denied owning edges precede reference evaluation. Attribute failures
    // must not make already-selected structural children look unavailable.
    let denied = tagged_host.denied_roles.borrow();
    let mut denied = denied.iter();
    for issue in &mut resolved.resolution.issues {
        if issue.kind == ReferenceResolutionIssueKind::ScopeDenied {
            if let Some(role) = denied.next() {
                issue.reference.role = *role;
            }
        }
    }
    let budget_exhausted = resolved
        .resolution
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit);
    let structural_map: Vec<_> = {
        let mut index = 0;
        resolved
            .resolution
            .nodes
            .iter()
            .map(|entry| {
                if matches!(entry.role, Role::Structural(_, _)) {
                    let result = Some(index);
                    index += 1;
                    result
                } else {
                    None
                }
            })
            .collect()
    };
    // Partition once; per-attribute scans of the full traversal would be
    // quadratic even when every reference is within its work budget.
    let mut incomplete_containers = HashSet::new();
    let mut site_indices = vec![Vec::new(); sites.len()];
    let mut map = vec![None; resolved.resolution.nodes.len()];
    for (index, entry) in resolved.resolution.nodes.iter().enumerate() {
        if let Role::Container(site) = entry.role {
            if !resolved.children_complete[index] {
                incomplete_containers.insert(site);
            }
        }
        if let Role::Value(site) | Role::Authored(site) = entry.role {
            map[index] = Some(site_indices[site].len());
            site_indices[site].push(index);
        }
    }
    let mut site_issues = vec![false; sites.len()];
    let mut value_issues = vec![false; sites.len()];
    let mut structural_failures = HashSet::new();
    let mut roots_failed = false;
    for issue in &resolved.resolution.issues {
        match issue.reference.role {
            Role::Value(site) | Role::Expression(site) | Role::Container(site) => {
                site_issues[site] = true;
                value_issues[site] = true;
            }
            Role::Authored(site) => site_issues[site] = true,
            Role::Structural(Some(parent), _) => {
                structural_failures.insert(parent);
            }
            Role::Structural(None, _) => roots_failed = true,
        }
    }
    let mut diagnostics = resolved.resolution.diagnostics.clone();
    diagnostics.extend(region_diagnostics);
    let mut attributes = vec![];
    for (site_id, (site, indices)) in sites.into_iter().zip(site_indices).enumerate() {
        let model = models.model(site.model);
        let mut access = NativeAttributeTargetAccess {
            nodes: vec![],
            input_views: vec![],
            roots: vec![],
            parents: vec![],
            children: vec![],
            complete: site.supported && tagged_host.started.borrow().contains(&site_id),
        };
        for global in &indices {
            let entry = &resolved.resolution.nodes[*global];
            let Some(retained) = tagged_host.inner.retained_node(&entry.node) else {
                access.complete = false;
                continue;
            };
            let view = input_node_view(&retained, tagged_host.inner);
            access.complete &= view.is_some();
            access.input_views.push(view);
            access.nodes.push(retained);
            access
                .parents
                .push(resolved.parents[*global].and_then(|parent| map[parent]));
            access.children.push(
                resolved.children[*global]
                    .iter()
                    .filter_map(|child| map[*child])
                    .collect(),
            );
            access.complete &= resolved.children_complete[*global];
            if matches!(entry.role, Role::Value(_)) {
                access.roots.push(map[*global].unwrap());
            }
        }
        if access.nodes.len() != indices.len() {
            access.complete = false;
            diagnostics.push(invalid_target(
                "Expected an original native attribute target",
                document_model::source_stack_for_node(site.attribute.node()).clone(),
            ));
        }
        if access.nodes.len() == indices.len() {
            for root in &access.roots {
                let mut pending = vec![*root];
                let mut owned = HashSet::new();
                while let Some(index) = pending.pop() {
                    if !owned.insert((
                        Arc::as_ptr(access.nodes[index].document()) as usize,
                        access.nodes[index].node_id(),
                    )) {
                        invalid_sites.insert(site_id);
                        break;
                    }
                    pending.extend(access.children[index].iter().copied());
                }
            }
        }
        if invalid_sites.contains(&site_id) {
            access.complete = false;
            diagnostics.push(invalid_target(
                "Invalid original attribute target ownership",
                document_model::source_stack_for_node(site.attribute.node()).clone(),
            ));
        }
        access.complete &= !site_issues[site_id] && !incomplete_containers.contains(&site_id);
        // A source slot which was never evaluated is not a resolved-empty value.
        let name = match site.attribute.node() {
            CemAstNode::Attribute { expanded_name, .. } => &expanded_name.local_name,
            _ => unreachable!(),
        };
        let selection_complete = site.supported
            && tagged_host.started.borrow().contains(&site_id)
            && !value_issues[site_id]
            && !incomplete_containers.contains(&site_id)
            && !invalid_sites.contains(&site_id);
        if selection_complete {
            document_model::validate_native_attribute_count(
                model,
                &site.element,
                name,
                access.roots.len(),
                site.attribute.node(),
                &mut diagnostics,
            );
        }
        let complete = access.complete
            && !model
                .attributes
                .get(name)
                .is_some_and(has_unconsumed_constraints);
        attributes.push((
            site.owner.and_then(|owner| structural_map[owner]),
            ConsumedAttributeValue {
                attribute: site.attribute,
                access: Arc::new(access),
                complete,
            },
        ));
    }
    let structural_indices: Vec<_> = structural_map
        .iter()
        .enumerate()
        .filter_map(|(global, local)| local.map(|_| global))
        .collect();
    let children_complete = structural_indices
        .iter()
        .map(|global| {
            if budget_exhausted {
                resolved.children_complete[*global] && !blocked_positions.contains(global)
            } else {
                !structural_failures.contains(global) && !blocked_positions.contains(global)
            }
        })
        .collect();
    let roots_complete = if budget_exhausted {
        resolved.roots_complete
    } else {
        !roots_failed
    };
    let mut resolution = ReferenceResolution {
        nodes: structural_indices
            .iter()
            .map(|index| resolved.resolution.nodes[*index].node.clone())
            .collect(),
        state: if !blocked_positions.is_empty()
            && resolved.resolution.state == ReferenceResolutionState::Resolved
        {
            ReferenceResolutionState::Pending
        } else {
            resolved.resolution.state
        },
        failed: resolved.resolution.failed,
        issues: resolved
            .resolution
            .issues
            .into_iter()
            .map(|issue| ReferenceResolutionIssue {
                reference: issue.reference.node,
                kind: issue.kind,
                occurrence: issue.occurrence,
                reason: issue.reason,
            })
            .collect(),
        diagnostics,
        work_used: resolved.resolution.work_used,
    };
    resolution.failed |= resolution
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation());
    Ok(ConsumedWalk {
        models: structural_indices
            .iter()
            .map(|index| effective_models[index])
            .collect(),
        structure: ReferenceStructureResolution {
            roots: resolved
                .roots
                .iter()
                .filter_map(|root| structural_map[*root])
                .collect(),
            roots_complete,
            parents: structural_indices
                .iter()
                .map(|index| resolved.parents[*index].and_then(|parent| structural_map[parent]))
                .collect(),
            children: structural_indices
                .iter()
                .map(|index| {
                    resolved.children[*index]
                        .iter()
                        .filter_map(|child| structural_map[*child])
                        .collect()
                })
                .collect(),
            children_complete,
            origins: structural_indices
                .iter()
                .map(|index| resolved.origins[*index].clone())
                .collect(),
            resolution,
        },
        attributes,
    })
}
pub(super) fn value_request_root(
    attribute: &SchemaDeclarationNode,
) -> Option<SchemaDeclarationNode> {
    let CemAstNode::Attribute { value_nodes, .. } = attribute.node() else {
        return None;
    };
    if let [root] = value_nodes.as_slice() {
        if let Some(root) = SchemaDeclarationNode::new(attribute.document().clone(), *root) {
            if matches!(root.node(), CemAstNode::Reference { .. })
                || native_attribute_expression(&root).is_some()
            {
                return Some(root);
            }
        }
    }
    (!value_nodes.is_empty()).then(|| attribute.clone())
}
fn value_role<H: InputReferenceHost>(host: &H, node: &H::Node, site: usize) -> Role {
    if host
        .retained_node(node)
        .as_ref()
        .and_then(native_attribute_expression)
        .is_some()
    {
        Role::Expression(site)
    } else {
        Role::Value(site)
    }
}
fn owning_edges(node: &CemAstNode) -> Vec<crate::parser::AstNodeId> {
    match node {
        CemAstNode::Document { root_children, .. } => root_children.clone(),
        CemAstNode::Element {
            children,
            attributes,
            ..
        } => children.iter().chain(attributes).copied().collect(),
        CemAstNode::Attribute { value_nodes, .. } => value_nodes.clone(),
        _ => vec![],
    }
}
