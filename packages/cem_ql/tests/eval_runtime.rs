use std::collections::BTreeMap;

use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, BudgetAxis, EvalError, Item, QueryContextScope};

fn eval(source: &str, policy: ScopePolicy) -> cem_ql::eval::ItemStream {
    eval_with_bindings(source, BTreeMap::new(), policy)
}

fn eval_with_bindings(
    source: &str,
    policy_bindings: BTreeMap<String, cem_ql::eval::ItemStream>,
    policy: ScopePolicy,
) -> cem_ql::eval::ItemStream {
    let query = compile(
        source,
        &CompileContext {
            policy_bindings: policy_bindings.clone(),
            ..CompileContext::default()
        },
    )
    .unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: policy,
            diagnostics: Vec::new(),
            policy_bindings,
            current_item: None,
        },
    )
}

fn default_policy() -> ScopePolicy {
    ScopePolicy::host_root().with_queue_size(128)
}

#[test]
fn evaluator_handles_literals_arithmetic_and_control_flow() {
    let stream = eval("if true { 1 + 2 * 3 } else { 0 }", default_policy());

    assert_eq!(stream.items, vec![Item::Atomic(AtomValue::Integer(7))]);
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn compiler_rejects_implicit_numeric_promotion() {
    let error = compile("1 + 1.0", &CompileContext::default()).unwrap_err();

    assert_eq!(error.code, "cem.ql.compile_failed");
    assert!(
        error.message.contains("matching numeric operand types"),
        "{}",
        error.message
    );
}

#[test]
fn evaluator_uses_rust_integer_division_and_remainder() {
    let stream = eval("(5 / 2, -5 / 2, -5 % 2)", default_policy());

    assert_eq!(
        stream.items,
        vec![
            Item::Atomic(AtomValue::Integer(2)),
            Item::Atomic(AtomValue::Integer(-2)),
            Item::Atomic(AtomValue::Integer(-1)),
        ]
    );
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn evaluator_rejects_integer_division_and_remainder_by_zero() {
    let division = eval("1 / 0", default_policy());
    assert_eq!(
        division.error,
        Some(EvalError::TypeError(
            "operator `/` cannot divide integer by zero"
        ))
    );

    let remainder = eval("1 % 0", default_policy());
    assert_eq!(
        remainder.error,
        Some(EvalError::TypeError(
            "operator `%` cannot divide integer by zero"
        ))
    );

    let decimal_division = eval("1.0 / 0.0", default_policy());
    assert_eq!(
        decimal_division.error,
        Some(EvalError::TypeError(
            "operator `/` cannot divide decimal by zero"
        ))
    );

    let decimal_remainder = eval("1.0 % 0.0", default_policy());
    assert_eq!(
        decimal_remainder.error,
        Some(EvalError::TypeError(
            "operator `%` cannot divide decimal by zero"
        ))
    );
}

#[test]
fn evaluator_uses_exact_finite_decimal_arithmetic() {
    let stream = eval(
        "(1.25 + 2.50, 5.5 - 2.0, 2.5 * 4.0, 1.0 / 4.0, 5.5 % 2.0)",
        default_policy(),
    );

    assert_eq!(
        stream.items,
        vec![
            Item::Atomic(AtomValue::Decimal("3.75".to_owned())),
            Item::Atomic(AtomValue::Decimal("3.5".to_owned())),
            Item::Atomic(AtomValue::Decimal("10".to_owned())),
            Item::Atomic(AtomValue::Decimal("0.25".to_owned())),
            Item::Atomic(AtomValue::Decimal("1.5".to_owned())),
        ]
    );
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn evaluator_rejects_non_finite_decimal_division_without_double_conversion() {
    let stream = eval("1.0 / 3.0", default_policy());

    assert_eq!(
        stream.error,
        Some(EvalError::TypeError(
            "operator `/` decimal result is not finite; use num:double(...) for IEEE division"
        ))
    );
}

#[test]
fn evaluator_keeps_double_ieee_division_behavior() {
    let infinity = eval("1.0e0 / 0.0e0", default_policy());
    let Some(Item::Atomic(AtomValue::Double(value))) = infinity.items.first() else {
        panic!("expected double infinity, got {:?}", infinity.items);
    };
    assert!(value.is_infinite() && value.is_sign_positive());

    let nan = eval("0.0e0 / 0.0e0", default_policy());
    let Some(Item::Atomic(AtomValue::Double(value))) = nan.items.first() else {
        panic!("expected double NaN, got {:?}", nan.items);
    };
    assert!(value.is_nan());

    let remainder = eval("-5.0e0 % 2.0e0", default_policy());
    assert_eq!(remainder.items, vec![Item::Atomic(AtomValue::Double(-1.0))]);
}

#[test]
fn evaluator_requires_explicit_conversion_for_dynamic_mixed_numeric_operands() {
    let mut bindings = BTreeMap::new();
    bindings.insert(
        "left".to_owned(),
        cem_ql::eval::ItemStream::once(Item::Atomic(AtomValue::Integer(1))),
    );
    bindings.insert(
        "right".to_owned(),
        cem_ql::eval::ItemStream::once(Item::Atomic(AtomValue::Double(1.0))),
    );

    let mixed = eval_with_bindings("left + right", bindings, default_policy());

    assert_eq!(
        mixed.error,
        Some(EvalError::TypeError(
            "operator `+` requires matching numeric operand types; use an explicit num:* conversion"
        ))
    );

    let converted = eval("num:double(1) + 1.0e0", default_policy());
    assert_eq!(converted.items, vec![Item::Atomic(AtomValue::Double(2.0))]);
    assert!(converted.error.is_none(), "{:?}", converted.diagnostics);
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

    let difference = eval("(1, 2, 3) - (2, 4)", default_policy());
    assert_eq!(
        difference.items,
        vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(3)),
        ]
    );

    let alias = eval("seq:difference((1, 2, 3), (2, 4))", default_policy());
    assert_eq!(alias.items, difference.items);

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
fn evaluator_runtime_dispatches_minus_for_unknown_policy_bindings() {
    let mut bindings = BTreeMap::new();
    bindings.insert(
        "left".to_owned(),
        cem_ql::eval::ItemStream::from_items(vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(2)),
            Item::Atomic(AtomValue::Integer(3)),
        ]),
    );
    bindings.insert(
        "right".to_owned(),
        cem_ql::eval::ItemStream::from_items(vec![
            Item::Atomic(AtomValue::Integer(2)),
            Item::Atomic(AtomValue::Integer(4)),
        ]),
    );

    let stream = eval_with_bindings("left - right", bindings, default_policy());

    assert_eq!(
        stream.items,
        vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(3)),
        ]
    );
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
}

#[test]
fn evaluator_runtime_minus_rejects_numeric_stream_mixes() {
    let mut bindings = BTreeMap::new();
    bindings.insert(
        "left".to_owned(),
        cem_ql::eval::ItemStream::once(Item::Atomic(AtomValue::Integer(1))),
    );
    bindings.insert(
        "right".to_owned(),
        cem_ql::eval::ItemStream::from_items(vec![
            Item::Atomic(AtomValue::Integer(2)),
            Item::Atomic(AtomValue::Integer(3)),
        ]),
    );

    let stream = eval_with_bindings("left - right", bindings, default_policy());

    assert_eq!(
        stream.error,
        Some(EvalError::TypeError(
            "operator `-` cannot mix numeric and stream operands"
        ))
    );
    assert!(stream
        .diagnostics
        .iter()
        .any(|diag| diag.code == "cem.ql.type_error"));
}

#[test]
fn evaluator_materializes_computed_record_keys() {
    let stream = eval(r#"{ [str:concat(("na", "me"))]: "Ada" }"#, default_policy());

    let Some(Item::Record(record)) = stream.items.first() else {
        panic!("expected record, got {:?}", stream.items);
    };
    assert_eq!(
        record.get("name").and_then(|items| items.first()),
        Some(&Item::Atomic(AtomValue::String("Ada".to_owned())))
    );
    assert!(stream.error.is_none(), "{:?}", stream.diagnostics);
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
