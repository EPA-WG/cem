//! Production direct record-read contracts and an opt-in complete-read baseline.
use super::*;
use crate::api::{compile, evaluate, CompileContext, EvaluationContext};
use crate::compile_profile::measure;
use crate::eval::EvalError;
use std::cell::{Cell, RefCell};

thread_local! {
    static FORCE_RECORD_READ_COPY: Cell<bool> = const { Cell::new(false) };
    static POINTS: RefCell<Option<Vec<(bool, IrId)>>> = const { RefCell::new(None) };
}

pub(crate) fn force_record_read_copy() -> bool {
    FORCE_RECORD_READ_COPY.with(Cell::get)
}

pub(crate) fn with_copied_record_reads<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            FORCE_RECORD_READ_COPY.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(FORCE_RECORD_READ_COPY.with(|v| v.replace(true)));
    run()
}

pub(crate) fn trace_point(force: bool, source: IrId) {
    POINTS.with(|trace| {
        if let Some(points) = trace.borrow_mut().as_mut() {
            points.push((force, source));
        }
    });
}

pub(crate) fn with_points<T>(run: impl FnOnce() -> T) -> (T, Vec<(bool, IrId)>) {
    struct Reset(Option<Vec<(bool, IrId)>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            POINTS.with(|trace| *trace.borrow_mut() = self.0.take());
        }
    }
    let _reset = Reset(POINTS.with(|trace| trace.replace(Some(vec![]))));
    let result = run();
    let points = POINTS.with(|trace| trace.borrow_mut().take().unwrap());
    (result, points)
}

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: impl IntoIterator<Item = (&'static str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), vec![value]))
            .collect(),
    )
}
fn context(input: ItemStream) -> EvaluationContext {
    EvaluationContext {
        policy_bindings: BTreeMap::from([("input".into(), input)]),
        ..Default::default()
    }
}
fn query(source: &str, context: &EvaluationContext) -> crate::ir::CompiledQuery {
    compile(
        source,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap()
}
fn same(actual: &ItemStream, expected: &ItemStream) {
    assert_eq!(actual, expected);
    assert_eq!(actual.diagnostics, expected.diagnostics);
    assert_eq!(actual.cursor, expected.cursor);
    assert_eq!(actual.chain, expected.chain);
}

#[test]
fn default_record_field_read_avoids_whole_read_and_keeps_selected_results_owned() {
    let input = ItemStream::once(record([
        ("label", record([("text", text("kept"))])),
        ("unselected", Item::Array(vec![text("extra")])),
    ]));
    let ctx = context(input.clone());
    let q = query("input.label", &ctx);
    let expected = with_copied_record_reads(|| evaluate(&q, &ctx));
    let (mut actual, stages) = measure(|| evaluate(&q, &ctx));
    same(&actual, &expected);
    let Item::Record(fields) = &mut actual.items[0] else {
        panic!("owned result")
    };
    fields.clear();
    assert_eq!(ctx.policy_bindings["input"], input);
    same(&evaluate(&q, &ctx), &expected);
    assert!(!stages.contains_key("copy/local-value"));
    assert_eq!(stages["eval/direct-record-read"].calls, 1);
    assert_eq!(stages["copy/direct-field-selected"].calls, 1);
}

#[test]
fn default_record_field_read_preserves_shadowing_parameters_and_safe_points() {
    let ctx = context(ItemStream::once(record([
        ("label", text("outer")),
        ("nested", record([("label", text("nested"))])),
        ("rows", Item::Array(vec![record([("label", text("row"))])])),
    ])));
    for source in [
        "input.label",
        "input.nested.label",
        "input.missing.label",
        "input.rows.label",
        "(input.label, input.label, input)",
        "{ let input = {label: \"inner\"}; input.label }",
        "declare function label(value) { value.label } label(input)",
        "seq:map((1, 2), fn(n) => input.label)",
        "declare function first() { second() } declare function second() { input.label } first()",
        "try { 1 / 0 } catch (code, message) { input.label }",
        "try { (input.label, 1 / 0, input.label) } catch (code, message) { input.nested.label }",
    ] {
        let q = query(source, &ctx);
        let (expected, expected_points) =
            with_copied_record_reads(|| with_points(|| evaluate(&q, &ctx)));
        let ((actual, points), stages) = measure(|| with_points(|| evaluate(&q, &ctx)));
        assert!(actual.error.is_none(), "{source}: {actual:?}");
        same(&actual, &expected);
        assert_eq!(points, expected_points, "{source}");
        assert!(stages.contains_key("eval/direct-record-read"), "{source}");
    }
}

#[test]
fn default_record_field_read_preserves_full_value_native_and_opaque_fallbacks() {
    let node = evaluate(
        &compile(
            r#"data:read("<name>ivy</name>", "xml").root"#,
            &Default::default(),
        )
        .unwrap(),
        &Default::default(),
    );
    assert!(node.error.is_none());
    let plain = record([("label", text("outer"))]);
    for (input, source) in [
        (ItemStream::once(plain.clone()), "input"),
        (ItemStream::once(plain.clone()), "seq:first(input).label"),
        (ItemStream::once(plain.clone()), "input.first"),
        (
            ItemStream::once(plain.clone()),
            "native:call(\"fixture.opaque\", input.label)",
        ),
        (
            ItemStream::once(plain.clone()),
            "seq:map((1), fn(n) => cemt:apply_templates(input.label))",
        ),
        (
            ItemStream::once(Item::Array(vec![plain.clone()])),
            "input.label",
        ),
        (
            ItemStream::from_items(vec![plain.clone(), plain]),
            "input.label",
        ),
        (ItemStream::once(text("scalar")), "input.label"),
        (node, "input.name"),
    ] {
        let ctx = context(input);
        let q = query(source, &ctx);
        let (expected, expected_points) =
            with_copied_record_reads(|| with_points(|| evaluate(&q, &ctx)));
        let ((actual, points), stages) = measure(|| with_points(|| evaluate(&q, &ctx)));
        same(&actual, &expected);
        assert_eq!(points, expected_points, "{source}");
        assert!(!stages.contains_key("eval/direct-record-read"), "{source}");
        assert!(stages.contains_key("copy/local-value"), "{source}");
    }
}

#[test]
fn default_record_field_read_preserves_metadata_and_failed_input_fallback() {
    use crate::eval::Diagnostic;
    for failed in [false, true] {
        let mut input = ItemStream::once(record([("label", text("value"))]));
        input.cursor = 3;
        input.chain = true;
        input.diagnostics.push(Diagnostic {
            code: "fixture.record-read".into(),
            severity: Severity::Warning,
            ..Default::default()
        });
        if failed {
            input.error = Some(EvalError::TypeError("retained input error"));
        }
        let ctx = context(input.clone());
        for source in [
            "input.label",
            "try { input.label } catch (code, message) { message }",
        ] {
            let q = query(source, &ctx);
            let (expected, expected_points) =
                with_copied_record_reads(|| with_points(|| evaluate(&q, &ctx)));
            let ((actual, points), stages) = measure(|| with_points(|| evaluate(&q, &ctx)));
            same(&actual, &expected);
            assert_eq!(points, expected_points);
            assert_eq!(stages.contains_key("eval/direct-record-read"), !failed);
            assert_eq!(ctx.policy_bindings["input"].cursor, 3);
            assert!(ctx.policy_bindings["input"].chain);
            assert_eq!(ctx.policy_bindings["input"].diagnostics, input.diagnostics);
        }
    }
}

#[test]
fn default_record_field_read_preserves_cancellation_and_child_budgets() {
    use crate::api::evaluate_with_control;
    use crate::eval::QueryItemView;
    use cem_ml::operation_control::{
        ExecutionScopeId, ExecutionScopeKind, ExecutionScopeRegistration, OperationControl,
        ROOT_EXECUTION_SCOPE_ID,
    };
    use cem_ml::scheduler::ScopePolicy;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    #[derive(Debug)]
    struct CancelField(OperationControl, ExecutionScopeId, Arc<AtomicUsize>);
    impl QueryItemView for CancelField {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.record-cancel"
        }
        fn identity(&self) -> String {
            "trigger".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            assert_eq!(name, "fire");
            self.2.fetch_add(1, Ordering::SeqCst);
            self.0.cancel_scope(self.1, None, None).unwrap();
            Some(vec![text("must-not-escape")])
        }
    }
    for cancelled in [false, true] {
        let source = if cancelled {
            "try { (input.label, input.trigger.fire) } catch (code, message) { \"must-not-catch\" }"
        } else {
            "declare function down(n, value) { if n == 0 { value.label } else { (value.label, down(n - 1, value)) } } try { down(17, input) } catch (code, message) { \"must-not-catch\" }"
        };
        let mut baseline = None;
        for copy in [true, false] {
            let policy = ScopePolicy::host_root().with_cpu_workers(2);
            let control = OperationControl::with_root_policy(Default::default(), policy).unwrap();
            let child = control
                .register_scope(
                    ROOT_EXECUTION_SCOPE_ID,
                    ExecutionScopeRegistration::inherited(
                        ExecutionScopeKind::Template,
                        "limited",
                        policy.with_cpu_workers(1),
                    ),
                )
                .unwrap();
            let sibling = control
                .register_scope(
                    ROOT_EXECUTION_SCOPE_ID,
                    ExecutionScopeRegistration::inherited(
                        ExecutionScopeKind::Template,
                        "sibling",
                        policy,
                    ),
                )
                .unwrap();
            let calls = Arc::new(AtomicUsize::new(0));
            let ctx = context(ItemStream::once(record([
                ("label", text("prefix")),
                (
                    "trigger",
                    Item::native(CancelField(control.clone(), child, calls.clone())),
                ),
            ])));
            let q = query(source, &ctx);
            let ((result, points), stages) = measure(|| {
                let run = || with_points(|| evaluate_with_control(&q, &ctx, &control, child));
                if copy {
                    with_copied_record_reads(run)
                } else {
                    run()
                }
            });
            assert!(result.error.is_some());
            assert!(result.items.is_empty());
            assert!(!result.diagnostics.is_empty());
            assert_eq!(stages.contains_key("eval/direct-record-read"), !copy);
            if let Some((expected, expected_points)) = &baseline {
                same(&result, expected);
                assert_eq!(&points, expected_points);
            } else {
                baseline = Some((result, points));
            }
            assert_eq!(calls.load(Ordering::SeqCst), usize::from(cancelled));
            assert_eq!(control.check_scope(child).is_err(), cancelled);
            assert!(control.check_scope(sibling).is_ok());
            assert_eq!(control.memory_charged(child).unwrap(), 0);
            let sibling_query = query("input.label", &ctx);
            let run = || evaluate_with_control(&sibling_query, &ctx, &control, sibling);
            let result = if copy {
                with_copied_record_reads(run)
            } else {
                run()
            };
            assert!(result.error.is_none());
        }
    }
}

#[test]
fn default_record_field_read_retains_selected_native_owners() {
    use crate::xpath::functions::XPathQueryItem;
    use std::sync::Arc;
    let imported = EvaluationContext::default();
    let root = evaluate(
        &compile(
            r#"data:read("<root><name>ivy</name></root>", "xml", "xpath").root"#,
            &Default::default(),
        )
        .unwrap(),
        &imported,
    );
    assert!(root.error.is_none());
    let owner = |item: &Item| {
        item.view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap()
            .source_owner()
            .unwrap()
    };
    let original_owner = owner(&root.items[0]);
    let weak = Arc::downgrade(&original_owner);
    let ctx = context(ItemStream::once(record([("node", root.items[0].clone())])));
    let q = query("input.node", &ctx);
    let expected = with_copied_record_reads(|| evaluate(&q, &ctx));
    let actual = evaluate(&q, &ctx);
    same(&actual, &expected);
    assert!(Arc::ptr_eq(&original_owner, &owner(&actual.items[0])));
    imported.data_readers.clear();
    std::mem::drop((ctx, expected, root, original_owner));
    assert!(
        weak.upgrade().is_some(),
        "selected result retains the native owner"
    );
    let read = context(actual.clone());
    assert_eq!(
        evaluate(&query("dom:text(input)", &read), &read).items,
        vec![text("ivy")]
    );
    std::mem::drop((read, actual));
    assert!(
        weak.upgrade().is_none(),
        "no hidden borrowed owner escapes evaluation"
    );
}
