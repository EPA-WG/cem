use cem_ml::dependency_identity::DependencyVersion;
use cem_ql::api::{compile_expression, EvaluationContext, StandaloneExpressionContext};
use cem_ql::eval::dependencies::{evaluate_tracked, EvaluationFreshness};
use cem_ql::eval::{AtomValue, Item, ItemStream};

fn revision(value: u64) -> DependencyVersion {
    DependencyVersion::Revision {
        owner: "test-provider".into(),
        generation: 1,
        revision: value,
    }
}

#[test]
fn tracks_only_the_taken_branch_and_invalidates_its_inputs() {
    let mut expression = StandaloneExpressionContext::default();
    for (name, item) in [
        ("flag", Item::Atomic(AtomValue::Boolean(true))),
        ("left", Item::Atomic(AtomValue::Integer(1))),
        ("right", Item::Atomic(AtomValue::Integer(2))),
    ] {
        expression.bindings.insert(
            name.into(),
            cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(item)),
        );
    }
    let query = compile_expression("if flag { left } else { right }", &expression)
        .unwrap()
        .query;
    let context = EvaluationContext {
        policy_bindings: expression
            .bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect(),
        ..Default::default()
    };
    let mut freshness = EvaluationFreshness::default();
    freshness.environment = revision(1);
    for name in ["flag", "left", "right"] {
        freshness.bindings.insert(name.into(), revision(1));
    }
    let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
    assert!(result.error.is_none());
    let receipt = receipt.unwrap();
    assert_eq!(
        receipt.binding_names().collect::<Vec<_>>(),
        vec!["flag", "left"]
    );
    freshness.bindings.insert("right".into(), revision(2));
    assert!(receipt.matches(&query, &context, &freshness));
    freshness.bindings.insert("left".into(), revision(2));
    assert!(!receipt.matches(&query, &context, &freshness));
    let mut next_context = context.clone();
    next_context.policy_bindings.insert(
        "flag".into(),
        ItemStream::once(Item::Atomic(AtomValue::Boolean(false))),
    );
    freshness.bindings.insert("flag".into(), revision(2));
    assert!(!receipt.matches(&query, &next_context, &freshness));
    let next = evaluate_tracked(&query, &next_context, &freshness)
        .1
        .unwrap();
    assert_eq!(
        next.binding_names().collect::<Vec<_>>(),
        vec!["flag", "right"]
    );
}

fn setup(
    source: &str,
    values: Vec<(&str, Item)>,
) -> (
    cem_ql::ir::CompiledQuery,
    EvaluationContext,
    EvaluationFreshness,
) {
    let mut expression = StandaloneExpressionContext::default();
    for (name, item) in values {
        expression.bindings.insert(
            name.into(),
            cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(item)),
        );
    }
    let query = compile_expression(source, &expression).unwrap().query;
    let context = EvaluationContext {
        policy_bindings: expression
            .bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect(),
        ..Default::default()
    };
    let freshness = EvaluationFreshness {
        environment: revision(1),
        bindings: context
            .policy_bindings
            .keys()
            .map(|k| (k.clone(), revision(1)))
            .collect(),
        ..Default::default()
    };
    (query, context, freshness)
}

#[test]
fn optimized_record_reads_and_absent_fields_track_the_whole_provider_value() {
    for source in ["input.name", "input.missing"] {
        let input = Item::Record(
            [(
                "name".into(),
                vec![Item::Atomic(AtomValue::String("Ada".into()))],
            )]
            .into(),
        );
        let (query, context, mut freshness) = setup(source, vec![("input", input)]);
        let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
        assert_eq!(result, cem_ql::api::evaluate(&query, &context));
        let receipt = receipt.unwrap();
        assert_eq!(receipt.binding_names().collect::<Vec<_>>(), vec!["input"]);
        freshness.bindings.clear();
        assert!(!receipt.matches(&query, &context, &freshness));
        assert!(evaluate_tracked(&query, &context, &freshness).1.is_err());
    }
}

#[test]
fn opaque_and_invalid_freshness_never_prevent_ordinary_evaluation() {
    use cem_ql::eval::dependencies::EvaluationTrackingError::*;
    let (query, context, mut freshness) = setup(
        "input",
        vec![("input", Item::Atomic(AtomValue::Integer(7)))],
    );
    for stamp in [
        DependencyVersion::Opaque,
        DependencyVersion::Revision {
            owner: String::new(),
            generation: 0,
            revision: 0,
        },
        DependencyVersion::Content(cem_ml::content_cache::ContentHash {
            scheme: "bad".into(),
            hex: "0".repeat(64),
        }),
    ] {
        freshness.bindings.insert("input".into(), stamp);
        let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
        assert_eq!(result, cem_ql::api::evaluate(&query, &context));
        assert!(matches!(receipt, Err(MissingFreshness | InvalidFreshness)));
    }
    freshness.environment = DependencyVersion::Opaque;
    assert!(evaluate_tracked(&query, &context, &freshness).1.is_err());
}

#[test]
fn owner_generation_environment_scope_policy_and_actual_ir_are_identity_inputs() {
    let (query, context, freshness) = setup(
        "input + 1",
        vec![("input", Item::Atomic(AtomValue::Integer(7)))],
    );
    let receipt = evaluate_tracked(&query, &context, &freshness).1.unwrap();
    assert!(receipt.matches(&query, &context, &freshness));
    for stamp in [
        DependencyVersion::Revision {
            owner: "different".into(),
            generation: 1,
            revision: 1,
        },
        DependencyVersion::Revision {
            owner: "test-provider".into(),
            generation: 2,
            revision: 1,
        },
    ] {
        let mut next = freshness.clone();
        next.bindings.insert("input".into(), stamp);
        assert!(!receipt.matches(&query, &context, &next));
    }
    let mut next = freshness.clone();
    next.environment = revision(2);
    assert!(!receipt.matches(&query, &context, &next));
    let mut next_context = context.clone();
    next_context.scope.0 += 1;
    assert!(!receipt.matches(&query, &next_context, &freshness));
    next_context = context.clone();
    next_context.scope_policy.queue_size -= 1;
    assert!(!receipt.matches(&query, &next_context, &freshness));
    let mut next_query = query.clone();
    next_query.source.push(' ');
    assert!(!receipt.matches(&next_query, &context, &freshness));
    // Same source text is insufficient: native IR and provenance must match too.
    next_query = query.clone();
    let literal = next_query
        .tree
        .nodes
        .iter_mut()
        .find(|n| matches!(n, cem_ql::ir::IrNode::LitInt(_)))
        .unwrap();
    *literal = cem_ql::ir::IrNode::LitInt(2);
    assert!(!receipt.matches(&next_query, &context, &freshness));
    next_query = query.clone();
    next_query.tree.source_maps.clear();
    assert!(!receipt.matches(&next_query, &context, &freshness));
}

#[test]
fn focus_needs_explicit_freshness() {
    let query = compile_expression(
        ".",
        &StandaloneExpressionContext::default()
            .with_context_item(Item::Atomic(AtomValue::Integer(7))),
    )
    .unwrap()
    .query;
    let context = EvaluationContext {
        current_item: Some(Item::Atomic(AtomValue::Integer(7))),
        ..Default::default()
    };
    let mut freshness = EvaluationFreshness {
        environment: revision(1),
        ..Default::default()
    };
    assert!(evaluate_tracked(&query, &context, &freshness).1.is_err());
    freshness.current_item = revision(1);
    let receipt = evaluate_tracked(&query, &context, &freshness).1.unwrap();
    assert!(receipt.matches(&query, &context, &freshness));
    freshness.current_item = revision(2);
    assert!(!receipt.matches(&query, &context, &freshness));
}

#[test]
fn failed_and_cancelled_evaluations_never_issue_receipts() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ml::scheduler::AbortSignal;
    use cem_ql::eval::dependencies::{evaluate_tracked_with_control, EvaluationTrackingError};
    let (query, mut context, freshness) = setup(
        "input",
        vec![("input", Item::Atomic(AtomValue::Integer(7)))],
    );
    context.policy_bindings.clear();
    let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
    assert!(result.error.is_some());
    assert!(matches!(
        receipt,
        Err(EvaluationTrackingError::UnsuccessfulEvaluation)
    ));
    let abort = AbortSignal::new();
    abort.abort();
    let control = OperationControl::new(abort);
    let (result, receipt) = evaluate_tracked_with_control(
        &query,
        &context,
        &freshness,
        &control,
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(result.error.is_some());
    assert!(receipt.is_err());
}

#[test]
fn opaque_native_calls_execute_on_every_attempt_even_with_fresh_stamps() {
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    #[derive(Debug)]
    struct Counter(Arc<AtomicUsize>);
    impl NativeQueryFunction for Counter {
        fn call(&self, _: NativeQueryRequest<'_>) -> ItemStream {
            ItemStream::once(Item::Atomic(AtomValue::Integer(
                self.0.fetch_add(1, Ordering::SeqCst) as i64,
            )))
        }
    }
    let (query, mut context, freshness) = setup("native:call(\"counter\")", vec![]);
    let calls = Arc::new(AtomicUsize::new(0));
    context
        .native_functions
        .register("counter", 0, Counter(calls.clone()))
        .unwrap();
    for _ in 0..2 {
        let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
        assert!(result.error.is_none(), "{result:?}");
        assert!(matches!(
            receipt,
            Err(cem_ql::eval::dependencies::EvaluationTrackingError::OpaqueProvider)
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn bounded_metadata_refuses_receipts_without_changing_results() {
    use cem_ql::eval::dependencies::{EvaluationTrackingError, MAX_EVALUATION_RECEIPT_BYTES};
    let (query, context, mut freshness) = setup(
        "input",
        vec![("input", Item::Atomic(AtomValue::Integer(7)))],
    );
    freshness.bindings.insert(
        "input".into(),
        DependencyVersion::Revision {
            owner: "x".repeat(MAX_EVALUATION_RECEIPT_BYTES),
            generation: 1,
            revision: 1,
        },
    );
    let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
    assert_eq!(result, cem_ql::api::evaluate(&query, &context));
    assert!(matches!(receipt, Err(EvaluationTrackingError::Bounds)));
    freshness.bindings.insert("input".into(), revision(1));
    let mut large_query = query.clone();
    large_query.source = " ".repeat(MAX_EVALUATION_RECEIPT_BYTES);
    let (result, receipt) = evaluate_tracked(&large_query, &context, &freshness);
    assert_eq!(result, cem_ql::api::evaluate(&large_query, &context));
    assert!(matches!(receipt, Err(EvaluationTrackingError::Bounds)));
}

#[test]
fn receipts_do_not_retain_native_input_owners() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    use std::sync::Arc;
    #[derive(Debug)]
    struct View(Arc<()>);
    impl QueryItemView for View {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "test.pure-view"
        }
        fn identity(&self) -> String {
            format!("{:p}", Arc::as_ptr(&self.0))
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
    }
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let (query, context, freshness) = setup("input", vec![("input", Item::native(View(owner)))]);
    let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
    let receipt = receipt.unwrap();
    drop(result);
    drop(context);
    assert!(weak.upgrade().is_none());
    assert_eq!(receipt.binding_names().collect::<Vec<_>>(), vec!["input"]);
}

#[test]
fn read_count_is_bounded_and_rejected_without_losing_evaluation_output() {
    use cem_ql::eval::dependencies::{EvaluationTrackingError, MAX_EVALUATION_READS};
    use cem_ql::{
        ir::{IrId, IrNode},
        resolve::BindingId,
        types::Type,
    };
    let (mut query, mut context, mut freshness) = setup("1", vec![]);
    let mut children = Vec::new();
    for i in 0..=MAX_EVALUATION_READS {
        let name = format!("binding{i}");
        let binding = BindingId(i as u32);
        let id = IrId(query.tree.nodes.len() as u32);
        query.tree.nodes.push(IrNode::LocalVar(binding));
        query.tree.types.push(Type::Any);
        query.tree.resolutions.push(None);
        query.tree.source_maps.push(Default::default());
        query.policy_bindings.insert(binding, name.clone());
        context.policy_bindings.insert(
            name.clone(),
            ItemStream::once(Item::Atomic(AtomValue::Integer(i as i64))),
        );
        freshness.bindings.insert(name, revision(1));
        children.push(id);
    }
    query.tree.root = IrId(query.tree.nodes.len() as u32);
    query.tree.nodes.push(IrNode::Sequence(children));
    query.tree.types.push(Type::Any);
    query.tree.resolutions.push(None);
    query.tree.source_maps.push(Default::default());
    let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items.len(), MAX_EVALUATION_READS + 1);
    assert!(matches!(receipt, Err(EvaluationTrackingError::Bounds)));
}

#[test]
fn resource_and_resolver_modules_are_conservatively_ineligible() {
    for source in [
        "data:read(\"<root/>\", \"xml\")",
        "url:parse(\"https://example.com/\")",
    ] {
        let (query, context, freshness) = setup(source, vec![]);
        let (result, receipt) = evaluate_tracked(&query, &context, &freshness);
        assert_eq!(result, cem_ql::api::evaluate(&query, &context));
        assert!(receipt.is_err());
    }
}

#[test]
fn native_identity_preserves_nonfinite_float_bits() {
    let (mut query, context, freshness) = setup("1", vec![]);
    let root = query.tree.root.0 as usize;
    query.tree.nodes[root] = cem_ql::ir::IrNode::LitDouble(f64::INFINITY);
    let receipt = evaluate_tracked(&query, &context, &freshness).1.unwrap();
    assert!(receipt.matches(&query, &context, &freshness));
    query.tree.nodes[root] = cem_ql::ir::IrNode::LitDouble(f64::NAN);
    assert!(!receipt.matches(&query, &context, &freshness));
}
