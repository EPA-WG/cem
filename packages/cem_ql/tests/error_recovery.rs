use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::ir::{deserialize::IrDeserializer, serialize::IrSerializer};

fn run(source: &str) -> ItemStream {
    let compiled = compile(source, &CompileContext::default()).expect("query must compile");
    evaluate(&compiled, &EvaluationContext::default())
}

fn ok(source: &str) -> Vec<Item> {
    let result = run(source);
    assert!(result.error.is_none(), "{:?}", result);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.items
}

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

#[test]
fn try_returns_success_without_evaluating_catch_and_preserves_native_identity() {
    assert_eq!(
        ok("try { (1, 2) } catch (code, msg) { 1 / 0 }"),
        vec![
            Item::Atomic(AtomValue::Integer(1)),
            Item::Atomic(AtomValue::Integer(2))
        ]
    );
    assert!(ok("try { () } catch (code, msg) { 42 }").is_empty());
    assert_eq!(
        ok(
            r#"{ let node = data:read("<root/>", "xml").root; (try { node } catch (code, msg) { () }) is node }"#
        ),
        vec![Item::Atomic(AtomValue::Boolean(true))]
    );
}

#[test]
fn catch_discards_partial_sequence_and_binds_code_and_message() {
    assert_eq!(
        ok(
            r#"try { (1, report:raise("sample.invalid", "bad input"), 2) } catch (code, msg) { (code, msg) }"#
        ),
        vec![string("sample.invalid"), string("bad input")]
    );
    assert_eq!(
        ok(r#"try { 1 / 0 } catch (code, msg) { code }"#),
        vec![string("cem.ql.type_error")]
    );
    assert_eq!(
        ok(
            r#"try { if (1 / 0) == 0 { 1 } else { report:raise("wrong", "must not execute") } } catch (code, msg) { code }"#
        ),
        vec![string("cem.ql.type_error")]
    );
}

#[test]
fn catch_scope_is_lexical_and_handler_failure_reaches_only_outer_catch() {
    assert_eq!(
        ok(
            r#"{ let code = "outer"; (try { report:raise("inner", "bad") } catch (code, msg) { code }, code) }"#
        ),
        vec![string("inner"), string("outer")]
    );
    assert_eq!(
        ok(
            r#"try { try { report:raise("first", "one") } catch (code, msg) { report:raise("second", msg) } } catch (code, msg) { (code, msg) }"#
        ),
        vec![string("second"), string("one")]
    );
    for source in [
        "try { code } catch (code, msg) { 1 }",
        "(try { 1 } catch (code, msg) { code }, msg)",
        "try { 1 } catch (code, code) { 2 }",
        "try { 1 }",
        "try { 1 } catch (code) { 2 }",
    ] {
        assert!(
            compile(source, &CompileContext::default()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn errors_cross_function_calls_and_do_not_evaluate_later_arguments() {
    assert_eq!(
        ok(
            r#"declare function fail() { report:raise("sample.call", "bad") }
        try { str:concat(fail(), report:raise("wrong", "later argument")) } catch (code, msg) { code }"#
        ),
        vec![string("sample.call")]
    );
}

#[test]
fn uncaught_raise_has_source_map_and_fatal_emit_remains_diagnostic_only() {
    let failed = run(r#"report:raise("sample.invalid", "bad input")"#);
    assert!(failed.error.is_some());
    assert!(failed.items.is_empty());
    let diagnostic = failed
        .diagnostics
        .iter()
        .find(|d| d.code == "sample.invalid")
        .unwrap();
    assert_eq!(diagnostic.message, "bad input");
    assert!(diagnostic
        .source_map
        .as_ref()
        .and_then(|s| s.current())
        .is_some());
    let emitted = run(
        r#"try { (report:emit("sample.notice", "notice", "fatal"), 42) } catch (code, msg) { 0 }"#,
    );
    assert!(emitted.error.is_none());
    assert_eq!(emitted.items, vec![Item::Atomic(AtomValue::Integer(42))]);
    assert!(emitted
        .diagnostics
        .iter()
        .any(|d| d.code == "sample.notice"));
}

#[test]
fn recovery_survives_query_artifact_reload() {
    let query = compile(
        r#"try { 1 / 0 } catch (code, msg) { code }"#,
        &CompileContext::default(),
    )
    .unwrap();
    let reloaded = IrDeserializer::deserialize(&IrSerializer::serialize(&query)).unwrap();
    let result = evaluate(&reloaded, &EvaluationContext::default());
    assert!(result.error.is_none());
    assert!(result.diagnostics.is_empty());
    assert_eq!(result.items, vec![string("cem.ql.type_error")]);
}

#[test]
fn recovery_cannot_swallow_cancellation_or_reset_materialization_budgets() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ml::scheduler::ScopePolicy;
    use cem_ql::api::evaluate_with_control;
    use cem_ql::eval::EvalError;
    let query = compile(
        "try { (1, 2, 3) | () } catch (code, msg) { 42 }",
        &CompileContext::default(),
    )
    .unwrap();
    let context = EvaluationContext {
        scope_policy: ScopePolicy::host_root().with_queue_size(2),
        ..EvaluationContext::default()
    };
    let result = evaluate(&query, &context);
    assert!(matches!(result.error, Some(EvalError::BudgetExceeded(_))));
    assert!(result.items.is_empty());
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let cancelled = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert_eq!(cancelled.error, Some(EvalError::Cancelled));
    assert!(cancelled.items.is_empty());
}

#[test]
fn raise_validates_its_arguments_and_catch_does_not_reset_function_budgets() {
    for source in [
        r#"report:raise("", "bad")"#,
        r#"report:raise((), "bad")"#,
        r#"report:raise(1, "bad")"#,
    ] {
        let result = run(source);
        assert!(matches!(
            result.error,
            Some(cem_ql::eval::EvalError::TypeError(_))
        ));
    }
    let calls = std::iter::repeat_n("try { fail() } catch (code, msg) { () }", 17)
        .collect::<Vec<_>>()
        .join(", ");
    let query = compile(
        &format!(r#"declare function fail() {{ report:raise("sample.fail", "bad") }} ({calls})"#),
        &CompileContext::default(),
    )
    .unwrap();
    let result = evaluate(
        &query,
        &EvaluationContext {
            scope_policy: cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(1),
            ..EvaluationContext::default()
        },
    );
    assert!(matches!(
        result.error,
        Some(cem_ql::eval::EvalError::BudgetExceeded(
            cem_ql::eval::BudgetAxis::FunctionCalls
        ))
    ));
    assert!(result.items.is_empty());
}
