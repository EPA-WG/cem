use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    parser::{document::CemDocument, AstNodeId, CemAstNode, ExpandedName},
    schema::{
        document_model::compile_schema_document_model,
        reference_policy::{ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy},
    },
    source::{ByteRange, SourceId},
    source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
    value::reference_resolution::{
        resolve_reference, ReferenceLinkEvaluation, ReferenceResolutionHost,
        ReferenceResolutionIssueKind, ReferenceResolutionState,
    },
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

struct Host {
    document: Arc<CemDocument>,
    outcomes: HashMap<AstNodeId, ReferenceLinkEvaluation<AstNodeId>>,
    policies: HashMap<AstNodeId, ReferenceUnresolvedPolicy>,
    default: ReferenceScopePolicy,
    denied: HashSet<(AstNodeId, AstNodeId)>,
    calls: usize,
    scopes: HashMap<u32, usize>,
    scope_bounds: HashMap<usize, cem_ml::schema::reference_traversal::ReferenceTraversalLimits>,
}

impl Host {
    fn graph(targets: &[Option<Vec<AstNodeId>>]) -> Self {
        let mut document = CemDocument::default();
        document.nodes.push(CemAstNode::Document {
            node_id: 0,
            root_children: (1..=targets.len() as u32).collect(),
            source: Default::default(),
        });
        for (index, edges) in targets.iter().enumerate() {
            let node_id = index as u32 + 1;
            let source = SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(1),
                    span: FrameSpan::Single(ByteRange::new(node_id as u64 * 10, 5)),
                    transform: TransformKind::CemTokenizer,
                }],
            };
            document.nodes.push(if let Some(edges) = edges {
                CemAstNode::Reference {
                    node_id,
                    expression: format!("#n{node_id}"),
                    context: 0,
                    targets: Some(edges.clone()),
                    source,
                }
            } else {
                CemAstNode::Element {
                    node_id,
                    expanded_name: ExpandedName {
                        namespace_uri: String::new(),
                        local_name: format!("n{node_id}"),
                        schema_id: None,
                    },
                    attributes: vec![],
                    children: vec![],
                    has_explicit_boundary: true,
                    source,
                }
            });
        }
        Self {
            document: Arc::new(document),
            outcomes: HashMap::new(),
            policies: HashMap::new(),
            default: ReferenceScopePolicy::schema_defaults().unwrap(),
            denied: HashSet::new(),
            calls: 0,
            scopes: HashMap::new(),
            scope_bounds: HashMap::new(),
        }
    }

    fn disposition(&mut self, node: u32, value: &str) {
        let model = compile_schema_document_model(
            "fixture:scope",
            &format!(
                r#"{{schema | {{constraints |
            {{constraint @kind="reference-unresolved-disposition" @value="{value}"}}
        }} }}"#
            ),
        );
        self.policies
            .insert(node, self.default.unresolved.for_scope(&model).unwrap());
    }
}

impl ReferenceResolutionHost for Host {
    type Node = AstNodeId;
    type Scope = usize;
    fn scope(&self, node: &u32) -> usize {
        *self.scopes.get(node).unwrap_or(&0)
    }
    fn scope_limits(
        &self,
        scope: &usize,
    ) -> cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
        self.scope_bounds
            .get(scope)
            .copied()
            .unwrap_or(self.default.limits)
    }
    fn reference_occurrence(&self, node: &u32) -> Option<ReferenceOccurrence> {
        match self.document.get(*node)? {
            CemAstNode::Reference {
                expression, source, ..
            } => Some(ReferenceOccurrence {
                identity: format!("fixture:{:p}:{node}", Arc::as_ptr(&self.document)),
                node_id: Some(*node),
                expression: Some(expression.clone()),
                source_map: source.clone(),
            }),
            _ => None,
        }
    }
    fn unresolved_policy(&self, node: &u32) -> &ReferenceUnresolvedPolicy {
        self.policies.get(node).unwrap_or(&self.default.unresolved)
    }
    fn permits_edge(&self, from: &u32, to: &u32) -> bool {
        !self.denied.contains(&(*from, *to))
    }
    fn evaluate(&mut self, node: &u32) -> ReferenceLinkEvaluation<u32> {
        self.calls += 1;
        if let Some(outcome) = self.outcomes.get(node) {
            return outcome.clone();
        }
        match self.document.get(*node).unwrap() {
            CemAstNode::Reference {
                targets: Some(targets),
                ..
            } => ReferenceLinkEvaluation::Resolved(targets.clone()),
            _ => ReferenceLinkEvaluation::Unresolved("dependency-unavailable".into()),
        }
    }
}

#[test]
fn order_multiplicity_and_source_graph_are_preserved() {
    let mut host = Host::graph(&[Some(vec![2, 3, 2]), None, Some(vec![4, 4]), None]);
    let limits = host.default.limits;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![2, 4, 4, 2]);
    assert_eq!(result.state, ReferenceResolutionState::Resolved);
    assert!(result.is_complete());
    assert!(!result.failed);
    assert_eq!(result.work_used, 6);
    assert!(
        matches!(host.document.get(1), Some(CemAstNode::Reference { targets: Some(edges), .. }) if edges == &[2, 3, 2])
    );
}

#[test]
fn cycles_stop_only_the_branch_and_follow_each_links_disposition() {
    for (value, failed, diagnostics) in [
        ("neutral", false, 0),
        ("mandatory", true, 1),
        ("warning", false, 1),
        ("ignore", false, 0),
    ] {
        let mut host = Host::graph(&[Some(vec![2, 3, 4]), Some(vec![2]), None, Some(vec![1])]);
        host.disposition(2, value);
        host.disposition(1, value);
        let limits = host.default.limits;
        let result = resolve_reference(1, &mut host, limits).unwrap();
        assert_eq!(result.nodes, vec![3]);
        assert_eq!(result.state, ReferenceResolutionState::Unresolved);
        assert!(!result.is_complete());
        assert_eq!(result.failed, failed);
        assert_eq!(result.issues.len(), 2);
        assert!(result
            .issues
            .iter()
            .all(|issue| issue.kind == ReferenceResolutionIssueKind::Cycle));
        assert_eq!(result.diagnostics.len(), diagnostics * 2);
        assert_eq!(result.issues[0].occurrence.node_id, Some(2));
        assert_eq!(
            result.issues[0].occurrence.source_map.frames[0].span,
            FrameSpan::Single(ByteRange::new(20, 5))
        );
        assert_eq!(host.calls, 3); // Cycle edges do not execute again.
    }
}

#[test]
fn depth_is_branch_local_and_work_is_one_execution_wide_budget() {
    let mut host = Host::graph(&[Some(vec![2, 3]), Some(vec![4]), None, None]);
    let mut limits = host.default.limits;
    limits.max_depth = 1;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![3]);
    assert_eq!(
        result.issues[0].kind,
        ReferenceResolutionIssueKind::DepthLimit
    );
    assert!(!result.is_complete());
    let mut host = Host::graph(&[Some(vec![2, 2, 2, 2]), None]);
    limits.max_depth = 128;
    limits.max_work = 3;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![2, 2]);
    assert_eq!(result.work_used, 3);
    assert_eq!(result.issues.len(), 1);
    assert_eq!(
        result.issues[0].kind,
        ReferenceResolutionIssueKind::WorkLimit
    );
    assert!(!result.is_complete());
    let mut host = Host::graph(&[Some(vec![2, 2]), None]);
    assert!(resolve_reference(1, &mut host, limits)
        .unwrap()
        .is_complete());
}

#[test]
fn explicit_edge_permissions_precede_target_evaluation() {
    let mut host = Host::graph(&[Some(vec![2, 3]), Some(vec![4]), None, None]);
    host.denied.insert((1, 2));
    host.disposition(1, "warning");
    let limits = host.default.limits;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![3]);
    assert_eq!(host.calls, 1);
    assert_eq!(
        result.issues[0].kind,
        ReferenceResolutionIssueKind::ScopeDenied
    );
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    assert_eq!(result.issues[0].occurrence.node_id, Some(1));
}

#[test]
fn empty_pending_unresolved_and_invalid_outcomes_are_distinct() {
    let mut host = Host::graph(&[Some(vec![])]);
    let limits = host.default.limits;
    assert!(resolve_reference(1, &mut host, limits)
        .unwrap()
        .is_complete());
    host.disposition(1, "mandatory");
    host.outcomes.insert(
        1,
        ReferenceLinkEvaluation::Pending("datadom-not-ready".into()),
    );
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.state, ReferenceResolutionState::Pending);
    assert!(!result.failed);
    assert!(result.diagnostics.is_empty());
    host.outcomes.insert(
        1,
        ReferenceLinkEvaluation::Unresolved("missing-binding".into()),
    );
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.state, ReferenceResolutionState::Unresolved);
    assert!(result.failed);
    host.disposition(1, "ignore");
    let diagnostic = Diagnostic {
        code: "cem.ql.type_error".into(),
        severity: Severity::Error,
        message: "Scalar operand".into(),
        ..Default::default()
    };
    host.outcomes.insert(
        1,
        ReferenceLinkEvaluation::Invalid(vec![diagnostic.clone()]),
    );
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.state, ReferenceResolutionState::Invalid);
    assert!(result.failed);
    assert_eq!(result.diagnostics, vec![diagnostic]);
}

#[test]
fn repeated_executions_receive_independent_context_and_do_not_store_outcomes() {
    let mut first = Host::graph(&[Some(vec![]), None, None]);
    if let CemAstNode::Reference {
        expression,
        targets,
        ..
    } = &mut Arc::get_mut(&mut first.document).unwrap().nodes[1]
    {
        *expression = "#datadom".into();
        *targets = None;
    }
    let shared_source = first.document.clone();
    let mut second = Host::graph(&[]);
    second.document = shared_source.clone();
    first
        .outcomes
        .insert(1, ReferenceLinkEvaluation::Resolved(vec![2]));
    second
        .outcomes
        .insert(1, ReferenceLinkEvaluation::Resolved(vec![3]));
    let limits = first.default.limits;
    let a = resolve_reference(1, &mut first, limits).unwrap();
    let b = resolve_reference(1, &mut second, limits).unwrap();
    assert_eq!(a.nodes, vec![2]);
    assert_eq!(b.nodes, vec![3]);
    assert!(
        matches!(shared_source.get(1), Some(CemAstNode::Reference { expression, targets: None, .. }) if expression == "#datadom")
    );
    assert_eq!(first.calls, 1);
    assert_eq!(second.calls, 1);
}

#[test]
fn deep_chains_use_an_explicit_stack_and_zero_bounds_are_rejected() {
    let depth = 10_000u32;
    let mut edges: Vec<_> = (1..=depth).map(|id| Some(vec![id + 1])).collect();
    edges.push(None);
    let mut host = Host::graph(&edges);
    let mut limits = host.default.limits;
    limits.max_depth = depth as usize;
    limits.max_work = depth as usize + 1;
    host.default.limits = limits;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![depth + 1]);
    assert!(result.is_complete());
    limits.max_work = 0;
    assert!(resolve_reference(1, &mut host, limits).is_err());
    let standard = host.default.limits;
    assert!(resolve_reference(depth + 1, &mut host, standard).is_err());
}

#[test]
fn destination_work_is_cumulative_across_reentry() {
    let mut host = Host::graph(&[Some(vec![2, 2, 4]), Some(vec![3]), None, None]);
    host.scopes.extend([(2, 1), (3, 1)]);
    let mut bounds = host.default.limits;
    bounds.max_work = 2;
    host.scope_bounds.insert(1, bounds);
    let limits = host.default.limits;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![3, 4]);
    assert_eq!(result.work_used, 5);
    assert_eq!(host.calls, 2);
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].reason, "scope-work-limit");
    assert_eq!(result.issues[0].reference, 1);
    assert!(!result.is_complete());
}

#[test]
fn destination_depth_constrains_subtree_without_relaxing_request() {
    let mut host = Host::graph(&[
        Some(vec![2, 6]),
        Some(vec![3]),
        Some(vec![4]),
        Some(vec![5]),
        None,
        None,
    ]);
    host.scopes.extend([(3, 1), (4, 1), (5, 1)]);
    let mut bounds = host.default.limits;
    bounds.max_depth = 1;
    host.scope_bounds.insert(1, bounds);
    let limits = host.default.limits;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![6]);
    assert_eq!(result.issues[0].reference, 4);
    assert_eq!(result.issues[0].reason, "scope-depth-limit");
    assert_eq!(host.calls, 3);

    bounds.max_depth = 100;
    host.scope_bounds.insert(1, bounds);
    let mut request = limits;
    request.max_depth = 1;
    let result = resolve_reference(1, &mut host, request).unwrap();
    assert_eq!(result.nodes, vec![6]);
    assert_eq!(result.issues[0].reason, "depth-limit");
    request.max_depth = limits.max_depth;
    request.max_work = 3;
    let result = resolve_reference(1, &mut host, request).unwrap();
    assert_eq!(result.work_used, 3);
    assert_eq!(result.issues[0].reason, "work-limit");
}

#[test]
fn exhausted_destination_prunes_wide_subtree_and_restores_parent() {
    let mut children = vec![3; 10_000];
    children.push(4);
    let mut host = Host::graph(&[Some(vec![2, 5]), Some(children), None, None, None]);
    host.scopes.extend([(2, 1), (3, 1), (4, 1)]);
    let mut bounds = host.default.limits;
    bounds.max_work = 2;
    host.scope_bounds.insert(1, bounds);
    let limits = host.default.limits;
    let result = resolve_reference(1, &mut host, limits).unwrap();
    assert_eq!(result.nodes, vec![3, 5]);
    assert_eq!(result.work_used, 5);
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].reference, 2);
    assert_eq!(host.calls, 2);
}
