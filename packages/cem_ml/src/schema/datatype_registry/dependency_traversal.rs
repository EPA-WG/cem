//! Bounded native and symbolic dependency selection, before executable compilation.
use super::{
    declaration_name, DatatypeDependency, DatatypeDependencyRole, DatatypeDependencyValue,
    DatatypeSource, DatatypeSourcePlan,
};
use crate::{
    parser::CemAstNode,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        reference_policy::{ReferenceOccurrence, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::{
        resolve_consumer_structure, ReferenceLinkEvaluation, ReferenceResolutionError,
        ReferenceResolutionHost, ReferenceStructureResolution,
    },
};

pub trait DatatypeDependencyHost: SchemaDeclarationHost {
    /// Original selected datatype and its given declaring lexical scope. Missing
    /// source context keeps dependency compilation deferred, never inferred from
    /// the consuming declaration's aliases or the target document's ownership.
    fn datatype_source(&self, target: &SchemaDeclarationNode) -> Option<DatatypeSource>;

    /// Resolve the unchanged literal QName using the original declaring source's
    /// lexical bindings. The host owns namespace admission and implementation
    /// lookup; matching a local name never implicitly grants a built-in contract.
    /// Returned targets pass the same bounds, grants and target checks as native
    /// selection. Missing lifecycle lookup remains pending, not an empty result.
    fn lookup_literal_type(
        &mut self,
        _source: &DatatypeSource,
        _dependency: &DatatypeDependency,
        _qname: &str,
    ) -> ReferenceLinkEvaluation<Self::Node> {
        ReferenceLinkEvaluation::Pending("datatype-literal-lookup-unavailable".into())
    }
}

/// Consumer containers retain original fields; they are not synthetic AST nodes
/// or references. Evaluated values remain the host's original typed handles.
#[derive(Debug, Clone)]
pub enum DatatypeTraversalNode<N> {
    Value(N, Option<DatatypeDependencyRole>),
    Field(DatatypeDependency, DatatypeSource),
    /// Compiler-owned symbolic link metadata over an original attribute. This
    /// does not construct an AST Reference or change the authored attribute.
    Literal(DatatypeDependency, DatatypeSource),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypeDeferredReason {
    MissingSource,
}
#[derive(Debug, Clone)]
pub struct DatatypeDeferredDependency {
    pub reason: DatatypeDeferredReason,
    pub source: SchemaDeclarationNode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypeDependencyIssueKind {
    Cardinality,
    WrongTarget,
}
#[derive(Debug, Clone)]
pub struct DatatypeDependencyIssue {
    pub kind: DatatypeDependencyIssueKind,
    pub attribute: SchemaDeclarationNode,
}
#[derive(Debug, Clone)]
pub struct DatatypeDependencySite {
    pub role: DatatypeDependencyRole,
    pub attribute: SchemaDeclarationNode,
    pub targets: Vec<SchemaDeclarationNode>,
    pub complete: bool,
}

/// Complete here means dependency selection, not executable datatype readiness.
/// Registered signatures, effective facets and converter/equality
/// selection remain compiler responsibilities. Available original sources survive
/// even when another branch is incomplete or invalid.
#[derive(Debug, Clone)]
pub struct NativeDatatypeDependencyTraversal<N> {
    pub walk: ReferenceStructureResolution<DatatypeTraversalNode<N>>,
    pub plans: Vec<DatatypeSourcePlan>,
    pub sites: Vec<DatatypeDependencySite>,
    pub deferred: Vec<DatatypeDeferredDependency>,
    pub issues: Vec<DatatypeDependencyIssue>,
}
impl<N> NativeDatatypeDependencyTraversal<N> {
    pub fn failed(&self) -> bool {
        self.walk.resolution.failed
            || !self.issues.is_empty()
            || self.plans.iter().any(|plan| !plan.issues.is_empty())
    }
    pub fn is_complete(&self) -> bool {
        self.walk.resolution.is_complete()
            && !self.failed()
            && self.deferred.is_empty()
            && self.sites.iter().all(|site| site.complete)
    }
}

struct TraversalHost<'a, H>(&'a mut H);
impl<H: DatatypeDependencyHost> TraversalHost<'_, H> {
    fn source_value(&self, node: &DatatypeTraversalNode<H::Node>) -> H::Node {
        match node {
            DatatypeTraversalNode::Value(value, _) => value.clone(),
            DatatypeTraversalNode::Field(edge, _) | DatatypeTraversalNode::Literal(edge, _) => {
                self.0.source_reference(edge.attribute.clone())
            }
        }
    }
}
impl<H: DatatypeDependencyHost> ReferenceResolutionHost for TraversalHost<'_, H> {
    type Node = DatatypeTraversalNode<H::Node>;
    type Scope = H::Scope;
    fn scope(&self, node: &Self::Node) -> Self::Scope {
        self.0.scope(&self.source_value(node))
    }
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits {
        self.0.scope_limits(scope)
    }
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence> {
        match node {
            DatatypeTraversalNode::Value(value, _) => self.0.reference_occurrence(value),
            DatatypeTraversalNode::Field(_, _) => None,
            DatatypeTraversalNode::Literal(edge, _) => {
                let CemAstNode::Attribute {
                    node_id, source, ..
                } = edge.attribute.node()
                else {
                    return None;
                };
                Some(ReferenceOccurrence {
                    identity: format!("datatype-literal:{}", edge.attribute.identity()),
                    node_id: Some(*node_id),
                    expression: None,
                    source_map: source.clone(),
                })
            }
        }
    }
    fn unresolved_policy(&self, node: &Self::Node) -> &ReferenceUnresolvedPolicy {
        self.0.unresolved_policy(&self.source_value(node))
    }
    fn permits_edge(&self, reference: &Self::Node, target: &Self::Node) -> bool {
        self.0
            .permits_edge(&self.source_value(reference), &self.source_value(target))
    }
    fn evaluate(&mut self, node: &Self::Node) -> ReferenceLinkEvaluation<Self::Node> {
        let (evaluation, role) = match node {
            DatatypeTraversalNode::Value(value, role) => (self.0.evaluate(value), *role),
            DatatypeTraversalNode::Literal(edge, source) => {
                let DatatypeDependencyValue::Literal(qname) = &edge.value else {
                    return ReferenceLinkEvaluation::Unresolved(
                        "invalid-literal-dependency".into(),
                    );
                };
                (
                    self.0.lookup_literal_type(source, edge, qname),
                    Some(edge.role),
                )
            }
            DatatypeTraversalNode::Field(_, _) => {
                return ReferenceLinkEvaluation::Unresolved(
                    "dependency-container-not-reference".into(),
                );
            }
        };
        match evaluation {
            ReferenceLinkEvaluation::Resolved(nodes) => ReferenceLinkEvaluation::Resolved(
                nodes
                    .into_iter()
                    .map(|node| DatatypeTraversalNode::Value(node, role))
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

fn selected_source<H: DatatypeDependencyHost>(
    root: &DatatypeSource,
    target: &SchemaDeclarationNode,
    host: &H,
) -> Option<DatatypeSource> {
    if target.identity() == root.declaration().identity() {
        Some(root.clone())
    } else {
        host.datatype_source(target)
            .filter(|source| source.declaration().identity() == target.identity())
    }
}

pub fn traverse_native_datatype_dependencies<H: DatatypeDependencyHost>(
    root: DatatypeSource,
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<NativeDatatypeDependencyTraversal<H::Node>, ReferenceResolutionError> {
    let CemAstNode::Element {
        node_id, source, ..
    } = root.declaration().node()
    else {
        return Err(ReferenceResolutionError::NotReference);
    };
    let origin = ReferenceOccurrence {
        identity: format!("datatype-dependencies:{}", root.declaration().identity()),
        node_id: Some(*node_id),
        expression: None,
        source_map: source.clone(),
    };
    let value =
        DatatypeTraversalNode::Value(host.source_reference(root.declaration().clone()), None);
    let mut adapter = TraversalHost(host);
    let walk = resolve_consumer_structure(
        value,
        &mut adapter,
        limits,
        Some(origin),
        |adapter, node| match node {
            DatatypeTraversalNode::Field(edge, source) => match &edge.value {
                DatatypeDependencyValue::Native(reference) => {
                    Some(vec![DatatypeTraversalNode::Value(
                        adapter.0.source_reference(reference.clone()),
                        Some(edge.role),
                    )])
                }
                DatatypeDependencyValue::Literal(_) => Some(vec![DatatypeTraversalNode::Literal(
                    edge.clone(),
                    source.clone(),
                )]),
            },
            DatatypeTraversalNode::Literal(_, _) => None,
            DatatypeTraversalNode::Value(value, role) => {
                let target = adapter.0.declaration_node(value)?;
                if role.is_some_and(|role| !valid_target(&target, role)) {
                    return None;
                }
                if !matches!(target.node(), CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "type")
                {
                    return None;
                }
                let source = selected_source(&root, &target, adapter.0)?;
                Some(
                    source
                        .plan()
                        .dependencies
                        .into_iter()
                        .map(|edge| DatatypeTraversalNode::Field(edge, source.clone()))
                        .collect(),
                )
            }
        },
    )?;
    let mut plans = vec![];
    let mut sites = vec![];
    let mut deferred = vec![];
    let mut issues = vec![];
    for (index, node) in walk.resolution.nodes.iter().enumerate() {
        match node {
            DatatypeTraversalNode::Value(value, role) => {
                let Some(target) = adapter.0.declaration_node(value) else {
                    continue;
                };
                if !role.is_some_and(|role| !valid_target(&target, role))
                    && matches!(target.node(), CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "type")
                {
                    match selected_source(&root, &target, adapter.0) {
                        Some(source) => plans.push(source.plan()),
                        None => deferred.push(DatatypeDeferredDependency {
                            reason: DatatypeDeferredReason::MissingSource,
                            source: target,
                        }),
                    }
                }
            }
            DatatypeTraversalNode::Literal(_, _) => {}
            DatatypeTraversalNode::Field(edge, _) => {
                let complete = walk.children_complete[index];
                if complete && walk.children[index].len() != 1 {
                    issues.push(DatatypeDependencyIssue {
                        kind: DatatypeDependencyIssueKind::Cardinality,
                        attribute: edge.attribute.clone(),
                    });
                }
                let mut targets = vec![];
                for child in &walk.children[index] {
                    let target = match &walk.resolution.nodes[*child] {
                        DatatypeTraversalNode::Value(value, _) => adapter.0.declaration_node(value),
                        DatatypeTraversalNode::Field(_, _)
                        | DatatypeTraversalNode::Literal(_, _) => None,
                    };
                    if !target
                        .as_ref()
                        .is_some_and(|target| valid_target(target, edge.role))
                    {
                        issues.push(DatatypeDependencyIssue {
                            kind: DatatypeDependencyIssueKind::WrongTarget,
                            attribute: edge.attribute.clone(),
                        });
                    }
                    targets.extend(target);
                }
                sites.push(DatatypeDependencySite {
                    role: edge.role,
                    attribute: edge.attribute.clone(),
                    targets,
                    complete,
                });
            }
        }
    }
    Ok(NativeDatatypeDependencyTraversal {
        walk,
        plans,
        sites,
        deferred,
        issues,
    })
}

fn valid_target(target: &SchemaDeclarationNode, role: DatatypeDependencyRole) -> bool {
    if role != DatatypeDependencyRole::ValidationRule {
        return declaration_name(target).is_some();
    }
    let CemAstNode::Element {
        expanded_name,
        attributes,
        ..
    } = target.node()
    else {
        return false;
    };
    if expanded_name.local_name != "behavior" {
        return false;
    }
    let name = attributes
        .iter()
        .rev()
        .find_map(|id| match target.document().get(*id) {
            Some(node @ CemAstNode::Attribute { expanded_name, .. })
                if expanded_name.local_name == "name" =>
            {
                Some(node)
            }
            _ => None,
        });
    matches!(name, Some(CemAstNode::Attribute {value: Some(value), value_nodes, ..}) if value_nodes.is_empty() && !value.trim().is_empty())
}
