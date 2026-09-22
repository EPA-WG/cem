//! Native test-only candidate. Production input bindings remain owned copies.
use super::*;
use crate::api::{compile, evaluate, CompileContext};
use std::cell::Cell;

thread_local! {
    static BORROW_INPUTS: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn enabled() -> bool {
    BORROW_INPUTS.with(Cell::get)
}

pub(crate) fn with_borrowed_inputs<T>(enabled: bool, operation: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            BORROW_INPUTS.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(BORROW_INPUTS.with(|v| v.replace(enabled)));
    operation()
}

#[test]
fn borrowed_inputs_are_scoped_immutable_and_reads_remain_owned() {
    let input = ItemStream::once(Item::Record(BTreeMap::from([
        (
            "label".into(),
            vec![Item::Atomic(AtomValue::String("outer".into()))],
        ),
        (
            "extra".into(),
            vec![Item::Array(vec![Item::Atomic(AtomValue::Integer(9))])],
        ),
    ])));
    let context = EvaluationContext {
        policy_bindings: BTreeMap::from([("input".into(), input.clone())]),
        scope: QueryContextScope(17),
        current_item: Some(Item::Atomic(AtomValue::String("focus".into()))),
        ..Default::default()
    };
    let options = CompileContext {
        policy_bindings: context.policy_bindings.clone(),
        ..Default::default()
    };
    let query = compile("input", &options).unwrap();
    let control = OperationControl::default();
    let mut ctx = with_borrowed_inputs(true, || {
        EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID)
    });
    let binding = *query
        .policy_bindings
        .iter()
        .find(|(_, name)| name.as_str() == "input")
        .unwrap()
        .0;
    assert!(std::ptr::eq(
        ctx.borrowed_inputs[&binding],
        &context.policy_bindings["input"]
    ));
    assert!(ctx.scopes[0].is_empty());
    assert_eq!(ctx.query_scope, context.scope);
    let mut result = ctx.lookup_var(binding);
    let Item::Record(fields) = &mut result.items[0] else {
        panic!("owned record");
    };
    fields.clear();
    assert_eq!(ctx.lookup_var(binding), input);
    ctx.push_scope();
    ctx.bind(
        binding,
        ItemStream::once(Item::Atomic(AtomValue::Integer(3))),
    );
    assert_eq!(
        ctx.lookup_var(binding).items[0],
        Item::Atomic(AtomValue::Integer(3))
    );
    ctx.pop_scope();
    assert_eq!(ctx.lookup_var(binding), input);
    assert_eq!(context.policy_bindings["input"], input);

    for source in [
        "record:entries(input).key",
        "seq:map((1, 2), fn(n) => seq:map((n), fn(m) => input.label))",
        "declare function first() { second() } declare function second() { input.label } first()",
        "{ let input = 4; input + 1 }",
        "try { 1 / 0 } catch (code, message) { input.label }",
    ] {
        let query = compile(source, &options).unwrap();
        assert!(query.binding_dependencies().is_some(), "{source}");
        let expected = evaluate(&query, &context);
        let actual = with_borrowed_inputs(true, || evaluate(&query, &context));
        assert_eq!(actual, expected, "{source}");
        assert_eq!(actual.diagnostics, expected.diagnostics, "{source}");
        assert!(actual.error.is_none(), "{source}: {actual:?}");
    }
}

#[test]
fn borrowed_input_reads_preserve_full_stream_metadata_and_opaque_fallback() {
    let mut input = ItemStream::from_items(vec![
        Item::Atomic(AtomValue::Integer(1)),
        Item::Atomic(AtomValue::Integer(2)),
    ]);
    input.cursor = 1;
    input.chain = true;
    input.error = Some(EvalError::TypeError("retained error"));
    input.diagnostics.push(Diagnostic {
        code: "fixture.input".into(),
        severity: Severity::Warning,
        ..Default::default()
    });
    let context = EvaluationContext {
        policy_bindings: BTreeMap::from([("input".into(), input.clone())]),
        ..Default::default()
    };
    let options = CompileContext {
        policy_bindings: context.policy_bindings.clone(),
        ..Default::default()
    };
    let control = OperationControl::default();
    for source in [
        "input",
        "native:call(\"fixture.opaque\", input)",
        "seq:map((1), fn(n) => cemt:apply_templates(input))",
    ] {
        let query = compile(source, &options).unwrap();
        let binding = *query
            .policy_bindings
            .iter()
            .find(|(_, name)| name.as_str() == "input")
            .unwrap()
            .0;
        let mut ctx = with_borrowed_inputs(true, || {
            EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID)
        });
        let closed = query.binding_dependencies().is_some();
        assert_eq!(ctx.borrowed_inputs.contains_key(&binding), closed);
        assert_eq!(ctx.scopes[0].contains_key(&binding), !closed);
        let result = ctx.lookup_var(binding);
        assert_eq!(result, input);
        assert_eq!(result.diagnostics, input.diagnostics);
        assert_eq!(result.cursor, input.cursor);
        assert_eq!(result.chain, input.chain);
        assert_ne!(result.items.as_ptr(), input.items.as_ptr());
    }
}
