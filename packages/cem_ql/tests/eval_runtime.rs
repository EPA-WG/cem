use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, BudgetAxis, EvalError, Item, QueryContextScope};

fn eval(source: &str, policy: ScopePolicy) -> cem_ql::eval::ItemStream {
    let query = compile(source, &CompileContext::default()).unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: policy,
            diagnostics: Vec::new(),
            policy_bindings: Default::default(),
        },
    )
}

fn default_policy() -> ScopePolicy {
    ScopePolicy::host_root().with_queue_size(128)
}

#[test]
fn evaluator_handles_literals_arithmetic_and_control_flow() {
    let stream = eval("if true then 1 + 2 * 3 else 0", default_policy());

    assert_eq!(stream.items, vec![Item::Atomic(AtomValue::Integer(7))]);
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn evaluator_applies_pipeline_lambda_with_current_item() {
    let stream = eval("(1, 2, 3).{. + 1}", default_policy());

    assert_eq!(
        stream.items,
        vec![
            Item::Atomic(AtomValue::Integer(2)),
            Item::Atomic(AtomValue::Integer(3)),
            Item::Atomic(AtomValue::Integer(4)),
        ]
    );
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn evaluator_deduplicates_union_in_source_order() {
    let stream = eval("(1, 2, 2) | (2, 3)", default_policy());

    assert_eq!(
        stream.items,
        vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(2)),
            Item::Atomic(AtomValue::Integer(3)),
        ]
    );
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn evaluator_materializes_intersect_difference_and_symmetric_difference() {
    let intersect = eval("(1, 2, 3) & (2, 3, 4)", default_policy());
    assert_eq!(
        intersect.items,
        vec![
            Item::Atomic(AtomValue::Integer(2)),
            Item::Atomic(AtomValue::Integer(3)),
        ]
    );

    let difference = eval("seq:difference((1, 2, 3), (2, 4))", default_policy());
    assert_eq!(
        difference.items,
        vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(3)),
        ]
    );

    let symmetric = eval("(1, 2, 3) ^ (2, 4)", default_policy());
    assert_eq!(
        symmetric.items,
        vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(3)),
            Item::Atomic(AtomValue::Integer(4)),
        ]
    );
}

#[test]
fn evaluator_emits_budget_exceeded_when_stage_materialization_crosses_scope_policy() {
    let stream = eval(
        "(1, 2, 3) | ()",
        ScopePolicy::host_root().with_queue_size(2),
    );

    assert_eq!(
        stream.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage))
    );
    assert!(stream
        .diagnostics
        .iter()
        .any(|diag| diag.code == "cem.ql.budget_exceeded"));
}
