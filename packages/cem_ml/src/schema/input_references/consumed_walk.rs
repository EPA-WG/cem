//! One traversal for selected structure, attribute chains and authorized source
//! subtrees. Tags are consumption roles over original handles, never AST nodes.
use super::*;
use crate::schema::attribute_references::{
    has_unconsumed_constraints, ConsumedAttributeValue, NativeAttributeTargetAccess,
};
use crate::schema::reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy};
use crate::value::reference_resolution::{
    ReferenceLinkEvaluation, ReferenceResolution, ReferenceResolutionIssue,
    ReferenceResolutionIssueKind,
};

#[derive(Clone, Copy)]
enum Role {
    Structural(Option<usize>),
    Value(usize),
    Authored(usize),
}
#[derive(Clone)]
struct Tagged<N> {
    node: N,
    role: Role,
}
struct Host<'a, H> {
    inner: &'a mut H,
    started: HashSet<usize>,
    denied_roles: std::cell::RefCell<Vec<Role>>,
}
impl<H: InputReferenceHost> ReferenceResolutionHost for Host<'_, H> {
    type Node = Tagged<H::Node>;
    type Scope = H::Scope;
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.inner.scope(&node.node)
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.inner.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        if matches!(node.role, Role::Authored(_)) {
            None
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
        if let Role::Value(site) = reference.role {
            self.started.insert(site);
        }
        match self.inner.evaluate(&reference.node) {
            ReferenceLinkEvaluation::Resolved(nodes) => ReferenceLinkEvaluation::Resolved(
                nodes
                    .into_iter()
                    .map(|node| Tagged {
                        node,
                        role: reference.role,
                    })
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
}
pub(super) struct ConsumedWalk<N> {
    pub structure: ReferenceStructureResolution<N>,
    pub attributes: Vec<(Option<usize>, ConsumedAttributeValue)>,
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
    let mut sites = vec![];
    let role = if let Some((attribute, element)) = attribute {
        sites.push(Site {
            owner: None,
            attribute,
            element,
            supported: true,
        });
        Role::Value(0)
    } else {
        Role::Structural(None)
    };
    let mut tagged_host = Host {
        inner: host,
        started: HashSet::new(),
        denied_roles: Default::default(),
    };
    let mut next_index = 0;
    let mut invalid_sites = HashSet::new();
    let mut resolved = resolve_reference_structure(
        Tagged { node: root, role },
        &mut tagged_host,
        limits,
        |host, entry| {
            let position = next_index;
            next_index += 1;
            let retained = host.inner.retained_node(&entry.node)?;
            match entry.role {
                Role::Structural(_) => {
                    let children = consumable_children(retained.node(), model)?;
                    let mut nodes: Vec<_> = children
                        .iter()
                        .filter_map(|id| {
                            SchemaDeclarationNode::new(retained.document().clone(), *id)
                        })
                        .map(|node| Tagged {
                            node: host.inner.source_node(node),
                            role: Role::Structural(Some(position)),
                        })
                        .collect();
                    if let CemAstNode::Element {
                        expanded_name,
                        attributes,
                        ..
                    } = retained.node()
                    {
                        let element = &model.elements[&expanded_name.local_name];
                        for id in attributes {
                            let Some(attribute) =
                                SchemaDeclarationNode::new(retained.document().clone(), *id)
                            else {
                                continue;
                            };
                            let CemAstNode::Attribute {
                                expanded_name: name,
                                value_nodes,
                                ..
                            } = attribute.node()
                            else {
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
                                single_reference(&attribute)
                            } else {
                                None
                            };
                            let site = sites.len();
                            sites.push(Site {
                                owner: Some(position),
                                attribute: attribute.clone(),
                                element: expanded_name.local_name.clone(),
                                supported: reference.is_some(),
                            });
                            if let Some(reference) = reference {
                                nodes.push(Tagged {
                                    node: host.inner.source_node(reference),
                                    role: Role::Value(site),
                                });
                            }
                        }
                    }
                    Some(nodes)
                }
                Role::Value(site) | Role::Authored(site) => {
                    let edges = owning_edges(retained.node());
                    if edges
                        .iter()
                        .any(|id| retained.document().get(*id).is_none())
                        || matches!(retained.node(), CemAstNode::Element{attributes,..} if attributes.iter().any(|id| !matches!(retained.document().get(*id), Some(CemAstNode::Attribute{..}))))
                    {
                        invalid_sites.insert(site);
                    }
                    Some(
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
                    )
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
                if matches!(entry.role, Role::Structural(_)) {
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
    let mut site_indices = vec![Vec::new(); sites.len()];
    let mut map = vec![None; resolved.resolution.nodes.len()];
    for (index, entry) in resolved.resolution.nodes.iter().enumerate() {
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
            Role::Value(site) => {
                site_issues[site] = true;
                value_issues[site] = true;
            }
            Role::Authored(site) => site_issues[site] = true,
            Role::Structural(Some(parent)) => {
                structural_failures.insert(parent);
            }
            Role::Structural(None) => roots_failed = true,
        }
    }
    let mut diagnostics = resolved.resolution.diagnostics.clone();
    let mut attributes = vec![];
    for (site_id, (site, indices)) in sites.into_iter().zip(site_indices).enumerate() {
        let mut access = NativeAttributeTargetAccess {
            nodes: vec![],
            roots: vec![],
            parents: vec![],
            children: vec![],
            complete: site.supported && tagged_host.started.contains(&site_id),
        };
        for global in &indices {
            let entry = &resolved.resolution.nodes[*global];
            let Some(retained) = tagged_host.inner.retained_node(&entry.node) else {
                access.complete = false;
                continue;
            };
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
        access.complete &= !site_issues[site_id];
        // A source slot which was never evaluated is not a resolved-empty value.
        let name = match site.attribute.node() {
            CemAstNode::Attribute { expanded_name, .. } => &expanded_name.local_name,
            _ => unreachable!(),
        };
        let selection_complete = site.supported
            && tagged_host.started.contains(&site_id)
            && !value_issues[site_id]
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
                resolved.children_complete[*global]
            } else {
                !structural_failures.contains(global)
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
        state: resolved.resolution.state,
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
fn single_reference(attribute: &SchemaDeclarationNode) -> Option<SchemaDeclarationNode> {
    let CemAstNode::Attribute { value_nodes, .. } = attribute.node() else {
        return None;
    };
    let [root] = value_nodes.as_slice() else {
        return None;
    };
    let root = SchemaDeclarationNode::new(attribute.document().clone(), *root)?;
    matches!(root.node(), CemAstNode::Reference { .. }).then_some(root)
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
