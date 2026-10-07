//! Opt-in, invocation-local namespace dependency coordination over original owners.
use super::{
    CemQlSchemaDeclarationHost, DeclarationScope, LexicalScopeHandoffError,
    NamespacePropertyPreparation, NamespaceScopePreparationIssue,
};
use crate::api::StandaloneExpressionContext;
use cem_ml::{
    parser::{AstNodeId, CemAstNode},
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        machine::LexicallyScopedDocument,
        namespace_references::{
            decode_native_namespace_property, NamespaceLexicalSnapshot, NamespaceNameCompletion,
            NamespaceNameCompletionError, NamespaceScopeTarget, PendingNamespaceValue,
        },
        reference_policy::ReferenceScopePolicyOverrides,
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::ReferenceResolutionError,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

type SourceKey = (usize, AstNodeId);
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceLifecycleError {
    Lexical(LexicalScopeHandoffError),
    InvalidRoot(AstNodeId),
    OverlappingRoots(AstNodeId),
    InvalidBounds,
    WorkLimit,
    Resolution(ReferenceResolutionError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceLifecycleIssue {
    Namespace(NamespaceNameCompletionError),
    ContextNotReady(AstNodeId),
    PropertyNotReady(AstNodeId),
    WorkLimit,
}
/// Immutable execution result. Ready roots are disjoint original selected forests;
/// unresolved siblings never borrow their names or input contexts. Property
/// reports preserve bounded outcomes/diagnostics independently of name readiness.
#[derive(Debug)]
pub struct NamespaceLifecycleSnapshot {
    pub completion: Arc<NamespaceNameCompletion>,
    pub ready_roots: Vec<AstNodeId>,
    pub incomplete_roots: Vec<(AstNodeId, NamespaceLifecycleIssue)>,
    pub properties: Vec<NamespacePropertyPreparation>,
    pub pending_properties: Vec<AstNodeId>,
    pub scopes: Vec<(AstNodeId, DeclarationScope)>,
    pub work_used: usize,
    pub work_exhausted: bool,
}
impl NamespaceLifecycleSnapshot {
    pub fn is_complete(&self) -> bool {
        !self.work_exhausted
            && self.incomplete_roots.is_empty()
            && self.pending_properties.is_empty()
            && self
                .properties
                .iter()
                .all(NamespacePropertyPreparation::is_ready)
    }
}
struct Invocation<'a> {
    host: &'a mut CemQlSchemaDeclarationHost,
    assignments: BTreeMap<SourceKey, DeclarationScope>,
    following: BTreeMap<SourceKey, BTreeMap<usize, DeclarationScope>>,
    names: BTreeMap<usize, Arc<NamespaceNameCompletion>>,
    publications: BTreeMap<SourceKey, super::namespace_publication::NamespacePublicationProof>,
    input_snapshot: u64,
}
impl Drop for Invocation<'_> {
    fn drop(&mut self) {
        self.host.node_scopes = std::mem::take(&mut self.assignments);
        self.host.following_scopes = std::mem::take(&mut self.following);
        self.host.namespace_name_completions = std::mem::take(&mut self.names);
        // Consumer changes to real input contexts expire earlier publications too.
        if self.host.namespace_input_snapshot == self.input_snapshot {
            self.host.namespace_publications = std::mem::take(&mut self.publications);
        }
    }
}
fn source(captured: &LexicallyScopedDocument, node: AstNodeId) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(captured.document().clone(), node).expect("original captured node")
}
fn forest(
    captured: &LexicallyScopedDocument,
    roots: &[AstNodeId],
    cap: usize,
) -> Result<(BTreeSet<AstNodeId>, Vec<BTreeSet<AstNodeId>>), NamespaceLifecycleError> {
    let mut all = BTreeSet::new();
    let mut regions = vec![];
    for root in roots {
        let mut nodes = BTreeSet::new();
        let mut pending = vec![*root];
        while let Some(id) = pending.pop() {
            let node = captured
                .document()
                .get(id)
                .ok_or(NamespaceLifecycleError::InvalidRoot(id))?;
            if !all.insert(id) {
                return Err(NamespaceLifecycleError::OverlappingRoots(id));
            }
            if all.len() > cap {
                return Err(NamespaceLifecycleError::WorkLimit);
            }
            nodes.insert(id);
            match node {
                CemAstNode::Document { root_children, .. } => pending.extend(root_children),
                CemAstNode::Element {
                    children,
                    attributes,
                    ..
                } => {
                    pending.extend(children);
                    pending.extend(attributes);
                }
                CemAstNode::Attribute { value_nodes, .. } => pending.extend(value_nodes),
                _ => {}
            }
        }
        regions.push(nodes);
    }
    Ok((all, regions))
}
/// Discover only lexical dependencies of selected source nodes and declarations
/// explicitly encountered by authorized selection. Dependency handles outside a
/// query forest supply bindings without becoming navigable query nodes.
fn dependencies(
    captured: &LexicallyScopedDocument,
    pending: &mut Vec<AstNodeId>,
    nodes: &mut BTreeSet<AstNodeId>,
    occurrences: &mut BTreeSet<AstNodeId>,
    properties: &mut BTreeSet<AstNodeId>,
    used: &mut usize,
    cap: usize,
) -> Result<(), NamespaceLifecycleError> {
    while let Some(id) = pending.pop() {
        if !nodes.insert(id) {
            continue;
        }
        *used = used.saturating_add(1);
        if *used > cap {
            return Err(NamespaceLifecycleError::WorkLimit);
        }
        let owner = captured.document();
        if captured.snapshot(owner, id).is_some() {
            occurrences.insert(id);
            if let Some(bindings) = captured.pending_namespace_bindings(owner, id) {
                pending.extend(bindings.values());
            }
        }
        if let Some(name) = captured.pending_namespace_name(owner, id) {
            pending.push(name.declaration);
        }
        if let Some(declaration) = captured.pending_namespace_declaration(owner, id) {
            match declaration.value {
                PendingNamespaceValue::Alias(alias) => pending.push(alias),
                PendingNamespaceValue::Native => {
                    properties.insert(id);
                    if let Ok(property) =
                        decode_native_namespace_property(source(captured, id), captured)
                    {
                        pending.push(property.value.node_id());
                    }
                }
            }
        }
    }
    Ok(())
}
// Copy only binding dependencies used by this view, rather than the entire
// publication table per occurrence/root. Charge metadata edges too, so many
// active prefixes cannot turn a bounded source walk into quadratic copying.
fn view_targets(
    captured: &LexicallyScopedDocument,
    nodes: impl IntoIterator<Item = AstNodeId>,
    targets: &BTreeMap<AstNodeId, NamespaceScopeTarget>,
    used: &mut usize,
    cap: usize,
) -> Option<BTreeMap<AstNodeId, NamespaceScopeTarget>> {
    let mut result = BTreeMap::new();
    let mut visited = BTreeSet::new();
    let owner = captured.document();
    for id in nodes {
        let mut pending = vec![];
        if let Some(name) = captured.pending_namespace_name(owner, id) {
            pending.push(name.declaration);
        }
        if let Some(bindings) = captured.pending_namespace_bindings(owner, id) {
            for declaration in bindings.values() {
                if *used >= cap {
                    return None;
                }
                *used += 1;
                pending.push(*declaration);
            }
        }
        if captured.pending_namespace_declaration(owner, id).is_some() {
            pending.push(id);
        }
        while let Some(declaration) = pending.pop() {
            if !visited.insert(declaration) {
                continue;
            }
            if *used >= cap {
                return None;
            }
            *used += 1;
            if let Some(target) = targets.get(&declaration) {
                result.insert(declaration, target.clone());
            } else if let Some(pending_value) =
                captured.pending_namespace_declaration(owner, declaration)
            {
                if let PendingNamespaceValue::Alias(alias) = pending_value.value {
                    pending.push(alias);
                }
            }
        }
    }
    Some(result)
}
impl CemQlSchemaDeclarationHost {
    /// Explicit opt-in lifecycle invocation. Caller contexts/policy overrides
    /// prepare original occurrences only after every captured lexical dependency
    /// is ready; preassigned original occurrence contexts remain authoritative.
    /// Missing inputs return incomplete roots for caller retry, without waiting
    /// for I/O or retaining an execution slot. Each property uses the shared
    /// authorized resolver; total metadata/selection work is bounded across retries.
    ///
    /// The consumer callback receives matching ready names and temporary scopes
    /// on this same host. Consume `ready_roots` for partial execution, or require
    /// `is_complete` before processing the whole forest. All source assignments,
    /// names and invocation publications restore on return/error/unwind. Retained
    /// output snapshots remain inspectable; no AST, original capture or source
    /// reference target is changed. Ordinary parse/load/query entry is unaffected.
    pub fn with_namespace_lifecycle<P, C, R>(
        &mut self,
        captured: Arc<LexicallyScopedDocument>,
        roots: &[AstNodeId],
        limits: ReferenceTraversalLimits,
        mut prepare: P,
        consume: C,
    ) -> Result<(NamespaceLifecycleSnapshot, R), NamespaceLifecycleError>
    where
        P: FnMut(
            &SchemaDeclarationNode,
            &NamespaceLexicalSnapshot,
            DeclarationScope,
            &Arc<NamespaceNameCompletion>,
        ) -> (
            Option<StandaloneExpressionContext>,
            ReferenceScopePolicyOverrides,
        ),
        C: FnOnce(&mut Self, &NamespaceLifecycleSnapshot) -> R,
    {
        if limits.max_depth == 0 || limits.max_work == 0 {
            return Err(NamespaceLifecycleError::InvalidBounds);
        }
        let (selected, regions) = forest(&captured, roots, limits.max_work)?;
        self.attach_captured_namespaces(captured.clone())
            .map_err(NamespaceLifecycleError::Lexical)?;
        self.attach_captured_names(&captured)
            .map_err(NamespaceLifecycleError::Lexical)?;
        let owner = Arc::as_ptr(captured.document()) as usize;
        let mut visited = BTreeSet::new();
        let mut occurrences = BTreeSet::new();
        let mut properties = BTreeSet::new();
        let mut used = 0;
        let mut pending: Vec<_> = selected.iter().copied().collect();
        dependencies(
            &captured,
            &mut pending,
            &mut visited,
            &mut occurrences,
            &mut properties,
            &mut used,
            limits.max_work,
        )?;
        let assignments = self.node_scopes.clone();
        let following = self.following_scopes.clone();
        let names = self.namespace_name_completions.clone();
        let publications = self.namespace_publications.clone();
        let input_snapshot = self.namespace_input_snapshot;
        let invocation = Invocation {
            host: self,
            assignments,
            following,
            names,
            publications,
            input_snapshot,
        };
        let host = &mut *invocation.host;
        let mut parents = BTreeMap::new();
        let mut installed = BTreeMap::new();
        let mut prepared = BTreeSet::new();
        let mut explicit = BTreeSet::new();
        let mut reports: BTreeMap<AstNodeId, NamespacePropertyPreparation> = BTreeMap::new();
        let mut attempts = BTreeMap::new();
        let mut targets: BTreeMap<AstNodeId, NamespaceScopeTarget> = host
            .namespace_publications
            .iter()
            .filter(|((key, _), proof)| *key == owner && proof.matches(host))
            .map(|((_, id), proof)| (*id, proof.target.clone()))
            .collect();
        let mut exhausted = false;
        loop {
            let mut progress = false;
            for id in &occurrences {
                if parents.contains_key(id) {
                    continue;
                }
                let occurrence = source(&captured, *id);
                let parent = host
                    .source_scope_with_assignments(&occurrence, &invocation.assignments)
                    .ok_or(NamespaceLifecycleError::Lexical(
                        LexicalScopeHandoffError::UnregisteredOwner,
                    ))?;
                parents.insert(*id, parent);
                if invocation.assignments.contains_key(&(owner, *id)) {
                    explicit.insert(*id);
                } else {
                    let scope = host
                        .register_lexical_scope_with_policy_overrides(
                            parent,
                            None,
                            Default::default(),
                        )
                        .unwrap();
                    host.node_scopes.insert((owner, *id), scope);
                    installed.insert(*id, scope);
                }
            }
            for id in &occurrences {
                if prepared.contains(id) {
                    continue;
                }
                let Some(local_targets) =
                    view_targets(&captured, [*id], &targets, &mut used, limits.max_work)
                else {
                    exhausted = true;
                    break;
                };
                let Ok(completion) =
                    NamespaceNameCompletion::new(captured.clone(), &[*id], local_targets)
                else {
                    continue;
                };
                let completion = Arc::new(completion);
                let occurrence = source(&captured, *id);
                let Ok(snapshot) = completion.lexical_snapshot(&occurrence) else {
                    continue;
                };
                if explicit.contains(id) {
                    prepared.insert(*id);
                    progress = true;
                    continue;
                }
                let parent = parents[id];
                let (context, policy) = prepare(&occurrence, &snapshot, parent, &completion);
                let scope = host
                    .register_lexical_scope_with_policy_overrides(parent, context, policy)
                    .unwrap();
                host.node_scopes.insert((owner, *id), scope);
                installed.insert(*id, scope);
                prepared.insert(*id);
                progress = true;
            }
            if exhausted {
                break;
            }
            let mut discovered = vec![];
            for id in &properties {
                if targets.contains_key(id) {
                    continue;
                }
                if let Some(previous) = reports.get(id) {
                    let waiting_for_binding = previous.preparation.as_ref().is_some_and(|p| {
                        p.issue == Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
                    });
                    if !waiting_for_binding || attempts.get(id) == Some(&targets.len()) {
                        continue;
                    }
                }
                attempts.insert(*id, targets.len());
                let declaration = source(&captured, *id);
                // Don't execute a selector until its complete original lexical
                // inputs are supplied. Pending scopes never inherit parent data.
                if let Ok(property) =
                    decode_native_namespace_property(declaration.clone(), &captured)
                {
                    if !prepared.contains(&property.value.node_id()) {
                        continue;
                    }
                }
                if used >= limits.max_work {
                    exhausted = true;
                    break;
                }
                let mut remaining = limits;
                remaining.max_work -= used;
                let report = host
                    .prepare_namespace_property(declaration, remaining)
                    .map_err(NamespaceLifecycleError::Resolution)?;
                if let Some(result) = &report.preparation {
                    used = used.saturating_add(result.selection.work_used);
                    exhausted |= used >= limits.max_work && result.selection.issues.iter().any(|issue|
                        issue.kind == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::WorkLimit);
                    if result.issue == Some(NamespaceScopePreparationIssue::TargetBindingNotReady) {
                        if let Some(target) = result
                            .selection
                            .nodes
                            .first()
                            .and_then(|n| host.declaration_node(n))
                        {
                            if Arc::ptr_eq(target.document(), captured.document())
                                && !visited.contains(&target.node_id())
                            {
                                discovered.push(target.node_id());
                            }
                        }
                    }
                }
                if report.is_ready() {
                    host.publish_namespace_property(&report)
                        .expect("fresh host preparation proof");
                    targets.insert(
                        *id,
                        report
                            .preparation
                            .as_ref()
                            .unwrap()
                            .target
                            .as_ref()
                            .unwrap()
                            .clone(),
                    );
                    progress = true;
                }
                reports.insert(*id, report);
            }
            if !discovered.is_empty() {
                match dependencies(
                    &captured,
                    &mut discovered,
                    &mut visited,
                    &mut occurrences,
                    &mut properties,
                    &mut used,
                    limits.max_work,
                ) {
                    Ok(()) => progress = true,
                    Err(NamespaceLifecycleError::WorkLimit) => exhausted = true,
                    Err(error) => return Err(error),
                }
            }
            if !progress || exhausted {
                break;
            }
        }
        let mut ready_roots = vec![];
        let mut incomplete_roots = vec![];
        for (root, nodes) in roots.iter().zip(&regions) {
            let Some(local_targets) = view_targets(
                &captured,
                nodes.iter().copied(),
                &targets,
                &mut used,
                limits.max_work,
            ) else {
                exhausted = true;
                incomplete_roots.push((*root, NamespaceLifecycleIssue::WorkLimit));
                continue;
            };
            let issue =
                match NamespaceNameCompletion::new(captured.clone(), &[*root], local_targets) {
                    Err(error) => Some(NamespaceLifecycleIssue::Namespace(error)),
                    Ok(completion) => {
                        let mut issue = properties
                            .iter()
                            .find(|id| nodes.contains(id) && !targets.contains_key(id))
                            .map(|id| NamespaceLifecycleIssue::PropertyNotReady(*id));
                        for id in occurrences.iter().filter(|id| nodes.contains(id)) {
                            if issue.is_some() {
                                break;
                            }
                            let occurrence = source(&captured, *id);
                            if let Err(error) = completion.lexical_snapshot(&occurrence) {
                                issue = Some(NamespaceLifecycleIssue::Namespace(error));
                                break;
                            }
                            if !host
                                .source_scope(&occurrence)
                                .and_then(|s| host.scope_record(s))
                                .is_some_and(|s| s.context.is_some())
                            {
                                issue = Some(NamespaceLifecycleIssue::ContextNotReady(*id));
                                break;
                            }
                        }
                        issue
                    }
                };
            if let Some(issue) = issue {
                incomplete_roots.push((*root, issue));
            } else {
                ready_roots.push(*root);
            }
        }
        let pending_properties = properties
            .iter()
            .filter(|id| !targets.contains_key(id))
            .copied()
            .collect();
        let completion = Arc::new(
            NamespaceNameCompletion::new(captured, &ready_roots, targets)
                .expect("each disjoint ready forest preflighted"),
        );
        host.namespace_name_completions
            .insert(owner, completion.clone());
        let snapshot = NamespaceLifecycleSnapshot {
            completion,
            ready_roots,
            incomplete_roots,
            properties: reports.into_values().collect(),
            pending_properties,
            scopes: installed.into_iter().collect(),
            work_used: used,
            work_exhausted: exhausted,
        };
        let result = consume(host, &snapshot);
        Ok((snapshot, result))
    }
}
