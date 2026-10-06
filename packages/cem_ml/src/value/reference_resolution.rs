//! Explicit bounded resolution over consumer-owned typed CEM reference nodes.
//! Evaluation, context, lexical scope selection and edge authorization stay
//! with the host. Loading, query construction and inspection never call this.

use crate::{
    diagnostics::Diagnostic,
    schema::{
        reference_policy::{
            ReferenceOccurrence, ReferenceUnresolvedPolicy, UnresolvedReferenceFact,
        },
        reference_traversal::ReferenceTraversalLimits,
    },
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
};

#[derive(Debug, Clone)]
pub enum ReferenceLinkEvaluation<N> {
    Resolved(Vec<N>),
    Pending(String),
    Unresolved(String),
    /// Original expression/type diagnostics, never suppressed by disposition.
    Invalid(Vec<Diagnostic>),
}

/// Nodes must be typed retained CEM handles, including their required storage
/// ownership. The host invokes its existing expression mechanism explicitly;
/// the resolver does not choose between saved edges and a fresh evaluation.
pub trait ReferenceResolutionHost {
    type Node: Clone;
    /// Runtime scope identity, independent of authored IDs and document owners.
    type Scope: Clone + Eq + Hash;
    fn scope(&self, node: &Self::Node) -> Self::Scope;
    /// Effective bounds must remain stable for a scope during this request.
    fn scope_limits(&self, scope: &Self::Scope) -> ReferenceTraversalLimits;
    fn reference_occurrence(&self, node: &Self::Node) -> Option<ReferenceOccurrence>;
    fn unresolved_policy(&self, reference: &Self::Node) -> &ReferenceUnresolvedPolicy;
    /// Required even for same-scope edges. The host knows the actual scopes;
    /// document ownership or ID spelling alone cannot authorize a crossing.
    fn permits_edge(&self, reference: &Self::Node, target: &Self::Node) -> bool;
    fn evaluate(&mut self, reference: &Self::Node) -> ReferenceLinkEvaluation<Self::Node>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceResolutionState {
    Resolved,
    Pending,
    Unresolved,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceResolutionIssueKind {
    Pending,
    Unresolved,
    Cycle,
    DepthLimit,
    WorkLimit,
    ScopeDenied,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct ReferenceResolutionIssue<N> {
    pub reference: N,
    pub kind: ReferenceResolutionIssueKind,
    pub occurrence: ReferenceOccurrence,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ReferenceResolution<N> {
    /// Ordered available terminal nodes. An incomplete result may have a
    /// useful prefix/other branches; it must not be treated as fully resolved.
    pub nodes: Vec<N>,
    pub state: ReferenceResolutionState,
    pub failed: bool,
    pub issues: Vec<ReferenceResolutionIssue<N>>,
    pub diagnostics: Vec<Diagnostic>,
    pub work_used: usize,
}

impl<N> ReferenceResolution<N> {
    pub fn is_complete(&self) -> bool {
        self.state == ReferenceResolutionState::Resolved
    }
}

/// Temporary consumer structure over original typed handles. Indices belong
/// only to this evaluation, never to the source arena or authored references.
#[derive(Debug, Clone)]
pub struct ReferenceStructureResolution<N> {
    pub resolution: ReferenceResolution<N>,
    pub roots: Vec<usize>,
    pub roots_complete: bool,
    pub parents: Vec<Option<usize>>,
    pub children: Vec<Vec<usize>>,
    pub children_complete: Vec<bool>,
    pub origins: Vec<ReferenceOccurrence>,
}
impl<N> ReferenceStructureResolution<N> {
    fn mark_incomplete(&mut self, parent: Option<usize>) {
        if let Some(parent) = parent {
            self.children_complete[parent] = false;
        } else {
            self.roots_complete = false;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceResolutionError {
    InvalidBounds,
    NotReference,
}
impl fmt::Display for ReferenceResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidBounds => "Reference resolution requires positive depth and work bounds",
            Self::NotReference => "Reference resolution requires a typed reference root",
        })
    }
}
impl std::error::Error for ReferenceResolutionError {}

#[derive(Clone)]
struct ScopeActivation<S> {
    scope: S,
    depth_ceiling: usize,
}
struct ScopeBudget {
    limits: ReferenceTraversalLimits,
    work_used: usize,
}
struct Frame<N, S> {
    nodes: std::vec::IntoIter<N>,
    reference: Option<(N, ReferenceOccurrence)>,
    depth: usize,
    scopes: Vec<ScopeActivation<S>>,
    structural_parent: Option<usize>,
    owns_reference: bool,
}

/// Resolve one reference under request-wide and effective destination bounds.
/// Each entered scope constrains its subtree; scope work is cumulative across
/// repeated entries within the request and never replenished on crossings.
/// Each visited node costs one work unit, including reference occurrences and
/// denied/cyclic edges. Only reference expansion costs depth.
/// Query execution has its own existing execution limits; it is not charged as
/// one work unit per query instruction by this graph traversal budget.
///
/// Cycle/depth failures stop their branch. Request work exhaustion stops the
/// request; scope work exhaustion prunes that scope subtree, preserving
/// enclosing siblings. Ancestor constraints remain active across crossings.
/// All keep the outcome incomplete, regardless of diagnostic disposition.
/// Repeated occurrences are evaluated at consumption time without a shared
/// cache or writeback, so the host retains control of context and timing.
pub fn resolve_reference<H: ReferenceResolutionHost>(
    root: H::Node,
    host: &mut H,
    limits: ReferenceTraversalLimits,
) -> Result<ReferenceResolution<H::Node>, ReferenceResolutionError> {
    Ok(walk_reference_structure(root, host, limits, |_, _| Ok(None), false, None)?.resolution)
}

/// Resolve a reference and descend into consumer-selected terminal children
/// under the same active reference stack, request budget and scope budgets.
/// Containment consumes work but does not itself increment reference depth.
/// Children are supplied as typed original handles, never synthetic references.
pub fn resolve_reference_structure<H, C>(
    root: H::Node,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    mut structural_children: C,
) -> Result<ReferenceStructureResolution<H::Node>, ReferenceResolutionError>
where
    H: ReferenceResolutionHost,
    C: FnMut(&H, &H::Node) -> Option<Vec<H::Node>>,
{
    walk_reference_structure(
        root,
        host,
        limits,
        |host, node| Ok(structural_children(host, node)),
        true,
        None,
    )
}

/// Internal consumer entry over an original owning container, such as a native
/// attribute with several authored value nodes. The origin is diagnostic metadata,
/// not a synthetic reference node. Containment costs work, never reference depth.
pub(crate) fn resolve_consumer_structure<H, C>(
    root: H::Node,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    origin: Option<ReferenceOccurrence>,
    mut children: C,
) -> Result<ReferenceStructureResolution<H::Node>, ReferenceResolutionError>
where
    H: ReferenceResolutionHost,
    C: FnMut(&H, &H::Node) -> Option<Vec<H::Node>>,
{
    walk_reference_structure(
        root,
        host,
        limits,
        |host, node| Ok(children(host, node)),
        true,
        origin,
    )
}

/// Enter consumer lifecycle stages only after the original structural node has
/// passed authorization and the current traversal budgets. Stages may prepare
/// schemas using stable caller contexts, but must not mutate those scope inputs.
/// Existing read-only child callbacks retain their public contracts above.
pub(crate) fn resolve_consumer_structure_with_stage<H, C>(
    root: H::Node,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    origin: Option<ReferenceOccurrence>,
    children: C,
) -> Result<ReferenceStructureResolution<H::Node>, ReferenceResolutionError>
where
    H: ReferenceResolutionHost,
    C: FnMut(&mut H, &H::Node) -> Result<Option<Vec<H::Node>>, ReferenceResolutionError>,
{
    walk_reference_structure(root, host, limits, children, true, origin)
}

fn walk_reference_structure<H, C>(
    root: H::Node,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    mut structural_children: C,
    record_structure: bool,
    consumer_origin: Option<ReferenceOccurrence>,
) -> Result<ReferenceStructureResolution<H::Node>, ReferenceResolutionError>
where
    H: ReferenceResolutionHost,
    C: FnMut(&mut H, &H::Node) -> Result<Option<Vec<H::Node>>, ReferenceResolutionError>,
{
    if limits.max_depth == 0 || limits.max_work == 0 {
        return Err(ReferenceResolutionError::InvalidBounds);
    }
    let owning_container = consumer_origin.is_some();
    let root_occurrence = consumer_origin
        .or_else(|| host.reference_occurrence(&root))
        .ok_or(ReferenceResolutionError::NotReference)?;
    let mut result = ReferenceResolution {
        nodes: vec![],
        state: ReferenceResolutionState::Resolved,
        failed: false,
        issues: vec![],
        diagnostics: vec![],
        work_used: 0,
    };
    let mut structure = ReferenceStructureResolution {
        resolution: result.clone(),
        roots: vec![],
        roots_complete: true,
        parents: vec![],
        children: vec![],
        children_complete: vec![],
        origins: vec![],
    };
    let root_scope = host.scope(&root);
    let root_limits = host.scope_limits(&root_scope);
    if root_limits.max_depth == 0 || root_limits.max_work == 0 {
        return Err(ReferenceResolutionError::InvalidBounds);
    }
    let mut budgets = HashMap::from([(
        root_scope.clone(),
        ScopeBudget {
            limits: root_limits,
            work_used: 0,
        },
    )]);
    let mut frames = vec![Frame {
        nodes: vec![root.clone()].into_iter(),
        reference: owning_container.then(|| (root.clone(), root_occurrence.clone())),
        structural_parent: None,
        owns_reference: false,
        depth: 0,
        scopes: vec![ScopeActivation {
            scope: root_scope,
            depth_ceiling: root_limits.max_depth,
        }],
    }];
    let mut active = HashSet::new();
    while let Some(frame) = frames.last_mut() {
        let Some(node) = frame.nodes.next() else {
            if frame.owns_reference {
                if let Some((_, occurrence)) = &frame.reference {
                    active.remove(&occurrence.identity);
                }
            }
            frames.pop();
            continue;
        };
        let depth = frame.depth;
        let structural_parent = frame.structural_parent;
        let parent = frame.reference.clone();
        let mut scopes = frame.scopes.clone();
        if result.work_used == limits.max_work {
            for frame in &frames {
                structure.mark_incomplete(frame.structural_parent);
            }
            let (reference, occurrence) =
                parent.unwrap_or_else(|| (root.clone(), root_occurrence.clone()));
            structure.mark_incomplete(structural_parent);
            unresolved(
                &mut result,
                host,
                reference,
                occurrence,
                ReferenceResolutionIssueKind::WorkLimit,
                "work-limit".into(),
            );
            break;
        }
        result.work_used += 1;
        let mut exhausted = None;
        for activation in &scopes {
            let budget = budgets.get_mut(&activation.scope).unwrap();
            if budget.work_used == budget.limits.max_work {
                exhausted = Some(activation.scope.clone());
                break;
            }
            budget.work_used += 1;
        }
        if let Some(scope) = exhausted {
            let (reference, occurrence) = parent
                .clone()
                .unwrap_or_else(|| (root.clone(), root_occurrence.clone()));
            structure.mark_incomplete(structural_parent);
            unresolved(
                &mut result,
                host,
                reference,
                occurrence,
                ReferenceResolutionIssueKind::WorkLimit,
                "scope-work-limit".into(),
            );
            while frames
                .last()
                .is_some_and(|frame| frame.scopes.iter().any(|a| a.scope == scope))
            {
                let pruned = frames.pop().unwrap();
                structure.mark_incomplete(pruned.structural_parent);
                if pruned.owns_reference {
                    if let Some((_, occurrence)) = pruned.reference {
                        active.remove(&occurrence.identity);
                    }
                }
            }
            continue;
        }
        if let Some((reference, occurrence)) = &parent {
            if !host.permits_edge(reference, &node) {
                structure.mark_incomplete(structural_parent);
                unresolved(
                    &mut result,
                    host,
                    reference.clone(),
                    occurrence.clone(),
                    ReferenceResolutionIssueKind::ScopeDenied,
                    "scope-denied".into(),
                );
                continue;
            }
        }
        let scope = host.scope(&node);
        if !scopes.iter().any(|activation| activation.scope == scope) {
            let budget = budgets.entry(scope.clone()).or_insert_with(|| ScopeBudget {
                limits: host.scope_limits(&scope),
                work_used: 0,
            });
            if budget.limits.max_depth == 0 || budget.limits.max_work == 0 {
                return Err(ReferenceResolutionError::InvalidBounds);
            }
            if budget.work_used == budget.limits.max_work {
                let (reference, occurrence) = parent
                    .clone()
                    .unwrap_or_else(|| (root.clone(), root_occurrence.clone()));
                structure.mark_incomplete(structural_parent);
                unresolved(
                    &mut result,
                    host,
                    reference,
                    occurrence,
                    ReferenceResolutionIssueKind::WorkLimit,
                    "scope-work-limit".into(),
                );
                continue;
            }
            budget.work_used += 1;
            scopes.push(ScopeActivation {
                scope,
                depth_ceiling: depth.saturating_add(budget.limits.max_depth),
            });
        }
        let Some(occurrence) = host.reference_occurrence(&node) else {
            if !record_structure {
                result.nodes.push(node);
                continue;
            }
            let index = result.nodes.len();
            structure.parents.push(structural_parent);
            structure.children.push(vec![]);
            structure.children_complete.push(true);
            structure.origins.push(
                parent
                    .as_ref()
                    .map(|(_, origin)| origin.clone())
                    .unwrap_or_else(|| root_occurrence.clone()),
            );
            if let Some(parent_index) = structural_parent {
                structure.children[parent_index].push(index);
            } else {
                structure.roots.push(index);
            }
            if let Some(children) = structural_children(host, &node)? {
                frames.push(Frame {
                    nodes: children.into_iter(),
                    reference: parent,
                    owns_reference: false,
                    structural_parent: Some(index),
                    depth,
                    scopes,
                });
            }
            result.nodes.push(node);
            continue;
        };
        if active.contains(&occurrence.identity) {
            structure.mark_incomplete(structural_parent);
            unresolved(
                &mut result,
                host,
                node,
                occurrence,
                ReferenceResolutionIssueKind::Cycle,
                "cycle".into(),
            );
            continue;
        }
        let request_depth_exhausted = depth >= limits.max_depth;
        if request_depth_exhausted || scopes.iter().any(|a| depth >= a.depth_ceiling) {
            structure.mark_incomplete(structural_parent);
            unresolved(
                &mut result,
                host,
                node,
                occurrence,
                ReferenceResolutionIssueKind::DepthLimit,
                if request_depth_exhausted {
                    "depth-limit"
                } else {
                    "scope-depth-limit"
                }
                .into(),
            );
            continue;
        }
        match host.evaluate(&node) {
            ReferenceLinkEvaluation::Resolved(nodes) => {
                active.insert(occurrence.identity.clone());
                frames.push(Frame {
                    nodes: nodes.into_iter(),
                    reference: Some((node, occurrence)),
                    owns_reference: true,
                    structural_parent,
                    depth: depth + 1,
                    scopes,
                });
            }
            ReferenceLinkEvaluation::Unresolved(reason) => {
                structure.mark_incomplete(structural_parent);
                unresolved(
                    &mut result,
                    host,
                    node,
                    occurrence,
                    ReferenceResolutionIssueKind::Unresolved,
                    reason,
                );
            }
            ReferenceLinkEvaluation::Pending(reason) => {
                structure.mark_incomplete(structural_parent);
                if result.state == ReferenceResolutionState::Resolved {
                    result.state = ReferenceResolutionState::Pending;
                }
                result.issues.push(ReferenceResolutionIssue {
                    reference: node,
                    kind: ReferenceResolutionIssueKind::Pending,
                    occurrence,
                    reason,
                });
            }
            ReferenceLinkEvaluation::Invalid(diagnostics) => {
                structure.mark_incomplete(structural_parent);
                result.state = ReferenceResolutionState::Invalid;
                result.failed = true;
                result.diagnostics.extend(diagnostics);
                result.issues.push(ReferenceResolutionIssue {
                    reference: node,
                    kind: ReferenceResolutionIssueKind::Invalid,
                    occurrence,
                    reason: "invalid-expression-or-result".into(),
                });
            }
        }
    }
    structure.resolution = result;
    Ok(structure)
}

fn unresolved<H: ReferenceResolutionHost>(
    result: &mut ReferenceResolution<H::Node>,
    host: &H,
    reference: H::Node,
    occurrence: ReferenceOccurrence,
    kind: ReferenceResolutionIssueKind,
    reason: String,
) {
    let fact = UnresolvedReferenceFact { occurrence, reason };
    let treatment = host.unresolved_policy(&reference).apply(&fact);
    if result.state != ReferenceResolutionState::Invalid {
        result.state = ReferenceResolutionState::Unresolved;
    }
    result.failed |= treatment.failed;
    result.diagnostics.extend(treatment.diagnostic);
    result.issues.push(ReferenceResolutionIssue {
        reference,
        kind,
        occurrence: treatment.fact.occurrence,
        reason: treatment.fact.reason,
    });
}
