//! A consumer fixture supplies lexical/runtime context and uses the existing
//! expression API. The shared graph walker never parses or executes source.
use cem_ml::{
    import::import_data,
    parser::CemAstNode,
    schema::reference_policy::{
        ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy,
    },
    value::reference_resolution::{
        resolve_reference, ReferenceLinkEvaluation, ReferenceResolutionHost,
        ReferenceResolutionState,
    },
};
use cem_ql::{
    api::{evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{imported_cem_tree, values::ReferenceView, AtomValue, Item, ItemStream},
};

struct Consumer {
    context: Option<StandaloneExpressionContext>,
    ready: bool,
    policy: ReferenceScopePolicy,
    query_calls: usize,
}

impl Consumer {
    fn new(context: Option<StandaloneExpressionContext>) -> Self {
        Self {
            context,
            ready: true,
            policy: ReferenceScopePolicy::schema_defaults().unwrap(),
            query_calls: 0,
        }
    }
}

impl ReferenceResolutionHost for Consumer {
    type Node = Item;
    type Scope = ();
    fn scope(&self, _: &Item) {}
    fn scope_limits(
        &self,
        _: &(),
    ) -> cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
        self.policy.limits
    }
    fn reference_occurrence(&self, node: &Item) -> Option<ReferenceOccurrence> {
        let view = node.view()?;
        if view.field("kind")?.first()?.atom() != Some(AtomValue::String("reference".into())) {
            return None;
        }
        let expression =
            view.field("expression")
                .and_then(|values| match values.first()?.atom()? {
                    AtomValue::String(value) => Some(value),
                    _ => None,
                });
        Some(ReferenceOccurrence {
            identity: view.identity(),
            node_id: None,
            expression,
            source_map: view.source_map().unwrap_or_default(),
        })
    }
    fn unresolved_policy(&self, _: &Item) -> &ReferenceUnresolvedPolicy {
        &self.policy.unresolved
    }
    fn permits_edge(&self, _: &Item, _: &Item) -> bool {
        // This consumer explicitly grants one runtime scope to its source and
        // supplied data. A production host must map actual scopes here.
        true
    }
    fn evaluate(&mut self, node: &Item) -> ReferenceLinkEvaluation<Item> {
        let view = node.view().unwrap();
        if let Some(native) = view.downcast_ref::<ReferenceView>() {
            return ReferenceLinkEvaluation::Resolved(native.0.values().to_vec());
        }
        if !self.ready {
            return ReferenceLinkEvaluation::Pending("data-not-ready".into());
        }
        let Some(context) = &self.context else {
            return ReferenceLinkEvaluation::Unresolved("runtime-context-unavailable".into());
        };
        let Some(AtomValue::String(expression)) =
            view.field("expression").and_then(|v| v.first()?.atom())
        else {
            panic!("the fixture provides retained source references");
        };
        self.query_calls += 1;
        match evaluate_expression(&expression, context) {
            Err(error) => ReferenceLinkEvaluation::Invalid(error.diagnostics),
            Ok(evaluation) if evaluation.result.error.is_some() => {
                ReferenceLinkEvaluation::Invalid(evaluation.result.diagnostics)
            }
            Ok(evaluation) => {
                // This consumer associates the outer # constructor's selected
                // targets with the retained source occurrence for this run.
                // Nested reference operands remain targets and are not erased.
                let item = &evaluation.result.items[0];
                let reference = item
                    .view()
                    .unwrap()
                    .downcast_ref::<ReferenceView>()
                    .unwrap();
                ReferenceLinkEvaluation::Resolved(reference.0.values().to_vec())
            }
        }
    }
}

fn context(input: ItemStream) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default()
        .with_binding("datadom", StandaloneExpressionBinding::any(input))
}

fn data(source: &str) -> Item {
    imported_cem_tree(import_data(source, "xml", "cem", "data.xml").unwrap())
}

fn template() -> std::sync::Arc<cem_ml::parser::tree::RetainedCemTree> {
    use cem_ml::{
        events::cem::CemEventNormalizer,
        parser::{
            builder::CemAstBuilder,
            tree::{CemTreeSemantics, RetainedCemTree},
        },
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    let text = "{#datadom}";
    let tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), text.as_bytes().to_vec()));
    let ast = CemAstBuilder::new(CemEventNormalizer::new(tokenizer)).build();
    assert!(ast.diagnostics.is_empty());
    RetainedCemTree::new(ast, "template.cem", text, CemTreeSemantics::default(), None).unwrap()
}

#[test]
fn one_retained_template_evaluates_against_independent_data_without_writeback() {
    let source = template();
    let document = imported_cem_tree(source.clone());
    let reference = document
        .view()
        .unwrap()
        .field("children")
        .unwrap()
        .remove(0);
    let mut first = Consumer::new(Some(context(ItemStream::once(data("<a/>")))));
    let mut second = Consumer::new(Some(context(ItemStream::once(data("<b/>")))));
    let limits = first.policy.limits;
    let a = resolve_reference(reference.clone(), &mut first, limits).unwrap();
    let b = resolve_reference(reference.clone(), &mut second, limits).unwrap();
    assert!(a.is_complete() && b.is_complete());
    assert_ne!(
        a.nodes[0].view().unwrap().identity(),
        b.nodes[0].view().unwrap().identity()
    );
    // Native results retain their owners after the evaluator/context is dropped.
    drop(first);
    drop(second);
    for (result, expected) in [(&a, "a"), (&b, "b")] {
        let children = result.nodes[0].view().unwrap().field("children").unwrap();
        assert_eq!(
            children[0].view().unwrap().field("name").unwrap()[0].atom(),
            Some(AtomValue::String(expected.into()))
        );
    }
    assert!(source.ast().nodes.iter().any(|node| matches!(node,
        CemAstNode::Reference { expression, targets: None, .. } if expression == "#datadom")));
    assert!(reference.view().unwrap().field("targets").is_none());
}

#[test]
fn lifecycle_readiness_empty_and_invalid_results_remain_distinct() {
    let source = template();
    let reference = imported_cem_tree(source.clone())
        .view()
        .unwrap()
        .field("children")
        .unwrap()
        .remove(0);
    let mut host = Consumer::new(None);
    let limits = host.policy.limits;
    host.ready = false;
    assert_eq!(
        resolve_reference(reference.clone(), &mut host, limits)
            .unwrap()
            .state,
        ReferenceResolutionState::Pending
    );
    assert_eq!(host.query_calls, 0);
    host.ready = true;
    assert_eq!(
        resolve_reference(reference.clone(), &mut host, limits)
            .unwrap()
            .state,
        ReferenceResolutionState::Unresolved
    );
    host.context = Some(context(ItemStream::empty()));
    let empty = resolve_reference(reference.clone(), &mut host, limits).unwrap();
    assert!(empty.is_complete());
    assert!(empty.nodes.is_empty());
    host.context = Some(context(ItemStream::once(Item::Atomic(AtomValue::String(
        "#an-id".into(),
    )))));
    let invalid = resolve_reference(reference.clone(), &mut host, limits).unwrap();
    assert_eq!(invalid.state, ReferenceResolutionState::Invalid);
    assert!(invalid.failed);
    assert!(!invalid.diagnostics.is_empty());
    assert!(invalid
        .diagnostics
        .iter()
        .all(|d| d.code.starts_with("cem.ql.")));
    assert_eq!(
        invalid.issues[0].occurrence.expression.as_deref(),
        Some("#datadom")
    );
    assert!(!invalid.issues[0].occurrence.source_map.frames.is_empty());
    host.context = Some(context(ItemStream::once(data("<ready/>"))));
    assert!(resolve_reference(reference.clone(), &mut host, limits)
        .unwrap()
        .is_complete());
    assert!(reference.view().unwrap().field("targets").is_none());
}

#[test]
fn binary_reloaded_source_evaluates_with_fresh_contexts_without_persisting_runtime_results() {
    use cem_ml::{
        ast::{decode::DebugBinaryDecoder, encode::DebugBinaryEncoder},
        parser::tree::{CemTreeSemantics, RetainedCemTree},
    };
    let original = template();
    let original_source = match original.ast().get(1).unwrap() {
        CemAstNode::Reference { source, .. } => source.clone(),
        _ => unreachable!(),
    };
    let payload = DebugBinaryEncoder::new().encode(original.ast()).bytes;
    drop(original);
    let restored = DebugBinaryDecoder::new().decode(&payload).unwrap();
    let source = RetainedCemTree::new(
        restored,
        "template.cem",
        "{#datadom}",
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let inert_payload = DebugBinaryEncoder::new().encode(source.ast()).bytes;
    let reference = imported_cem_tree(source.clone())
        .view()
        .unwrap()
        .field("children")
        .unwrap()
        .remove(0);
    assert!(reference.view().unwrap().field("targets").is_none());
    assert_eq!(reference.source_map().unwrap(), original_source);
    let mut host = Consumer::new(None);
    host.ready = false;
    let limits = host.policy.limits;
    assert_eq!(
        resolve_reference(reference.clone(), &mut host, limits)
            .unwrap()
            .state,
        ReferenceResolutionState::Pending
    );
    assert_eq!(host.query_calls, 0);
    host.ready = true;
    let first_input = data("<first/>");
    let second_input = data("<second/>");
    let mut outcomes = vec![];
    for input in [first_input.clone(), second_input.clone()] {
        host.context = Some(context(ItemStream::once(input.clone())));
        let outcome = resolve_reference(reference.clone(), &mut host, limits).unwrap();
        assert!(outcome.is_complete());
        assert_eq!(outcome.nodes.len(), 1);
        assert_eq!(
            outcome.nodes[0].view().unwrap().identity(),
            input.view().unwrap().identity()
        );
        outcomes.push(outcome);
        assert_eq!(
            DebugBinaryEncoder::new().encode(source.ast()).bytes,
            inert_payload
        );
        assert!(reference.view().unwrap().field("targets").is_none());
    }
    drop(host);
    drop(first_input);
    drop(second_input);
    assert_ne!(
        outcomes[0].nodes[0].view().unwrap().identity(),
        outcomes[1].nodes[0].view().unwrap().identity()
    );
    for (outcome, name) in outcomes.iter().zip(["first", "second"]) {
        let children = outcome.nodes[0].view().unwrap().field("children").unwrap();
        assert_eq!(
            children[0].view().unwrap().field("name").unwrap()[0].atom(),
            Some(AtomValue::String(name.into()))
        );
    }
}

#[test]
fn reloaded_cyclic_graph_exposes_native_target_handles_without_expanding_children() {
    use cem_ml::{
        ast::{decode::DebugBinaryDecoder, encode::DebugBinaryEncoder},
        events::cem::CemEventNormalizer,
        parser::{
            builder::CemAstBuilder,
            tree::{CemTreeSemantics, RetainedCemTree},
        },
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    use cem_ql::eval::{retained_cem_node, RetainedCemNode};
    let text = "{section | {#first} {target} {#second}}";
    let mut document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    let references: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Reference { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect();
    let target = document
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "target" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let edges = [
        vec![target, references[1], target, references[0]],
        vec![references[0]],
    ];
    for (&id, targets) in references.iter().zip(&edges) {
        let CemAstNode::Reference {
            targets: stored, ..
        } = &mut document.nodes[id as usize]
        else {
            unreachable!()
        };
        *stored = Some(targets.clone());
    }
    let payload = DebugBinaryEncoder::new().encode(&document).bytes;
    let source = RetainedCemTree::new(
        DebugBinaryDecoder::new().decode(&payload).unwrap(),
        "graph.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    for (&id, edges) in references.iter().zip(&edges) {
        let item = RetainedCemNode::new(source.clone(), id)
            .unwrap()
            .query_item();
        let view = item.view().unwrap();
        assert_eq!(
            view.field("kind").unwrap()[0].atom(),
            Some(AtomValue::String("reference".into()))
        );
        assert!(view.field("children").unwrap().is_empty());
        let targets = view.field("targets").unwrap();
        assert_eq!(targets.len(), edges.len());
        for (target, &expected_id) in targets.iter().zip(edges) {
            let native = retained_cem_node(target).unwrap();
            assert_eq!(native.node_id(), expected_id);
            assert!(std::sync::Arc::ptr_eq(native.owner(), &source));
        }
    }
    assert_eq!(
        DebugBinaryEncoder::new().encode(source.ast()).bytes,
        payload
    );
}
