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
