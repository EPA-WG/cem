//! Native-only experiment moving an owned let value into its local scope.
use super::*;
use crate::api::{compile, evaluate, CompileContext};
use crate::compile_profile::measure;
use crate::eval::pipeline::record_read_profile_tests::with_points;
use std::cell::Cell;

thread_local! {
    static MOVE_LET: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn enabled() -> bool {
    MOVE_LET.with(Cell::get)
}

pub(crate) fn with_moved_lets<T>(enabled: bool, run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            MOVE_LET.with(|value| value.set(self.0));
        }
    }
    let _reset = Reset(MOVE_LET.with(|value| value.replace(enabled)));
    run()
}

fn bind_moved(ctx: &mut EvalCtx<'_>, name: BindingId, value: ItemStream) -> ItemStream {
    let _profile = crate::compile_profile::Span::new("candidate/move-let-binding");
    // Reads of the binding still see its complete status and stream metadata.
    // The enclosing let needs only diagnostics/error after evaluating the body.
    let status = {
        let _status = crate::compile_profile::Span::new("copy/let-status");
        ItemStream {
            diagnostics: value.diagnostics.clone(),
            error: value.error.clone(),
            ..ItemStream::empty()
        }
    };
    ctx.bind(name, value);
    status
}

pub(crate) fn eval_let(
    ctx: &mut EvalCtx<'_>,
    name: BindingId,
    value: IrId,
    body: IrId,
) -> ItemStream {
    let value = ctx.eval_id(value);
    ctx.push_scope();
    let status = bind_moved(ctx, name, value);
    let mut out = ctx.eval_id(body);
    out.extend_diagnostics(status);
    ctx.pop_scope();
    out
}

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn context() -> EvaluationContext {
    EvaluationContext {
        policy_bindings: BTreeMap::from([(
            "input".into(),
            ItemStream::once(Item::Record(BTreeMap::from([
                ("label".into(), vec![text("outer")]),
                ("extra".into(), vec![Item::Array(vec![text("retained")])]),
            ]))),
        )]),
        ..Default::default()
    }
}

fn query(source: &str, context: &EvaluationContext) -> CompiledQuery {
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
fn let_move_candidate_avoids_redundant_binding_copy() {
    let context = context();
    let query = query("{ let local = input; local.label }", &context);
    let expected = evaluate(&query, &context);
    let (actual, stages) = measure(|| with_moved_lets(true, || evaluate(&query, &context)));
    same(&actual, &expected);
    assert_eq!(actual.items, vec![text("outer")]);
    assert!(!stages.contains_key("copy/let-binding"));
    assert_eq!(stages["candidate/move-let-binding"].calls, 1);
    assert_eq!(stages["copy/local-value"].calls, 1);
}

#[test]
fn let_move_candidate_moves_allocation_and_keeps_full_binding_metadata() {
    let context = context();
    let query = query("input", &context);
    let name = *query.policy_bindings.keys().next().unwrap();
    let mut value = context.policy_bindings["input"].clone();
    value.cursor = 3;
    value.chain = true;
    value.error = Some(EvalError::TypeError("retained error"));
    value.diagnostics.push(Diagnostic {
        code: "fixture.let".into(),
        ..Default::default()
    });
    let expected = value.clone();
    let allocation = value.items.as_ptr();
    let mut ctx = EvalCtx::new(
        &query,
        &context,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    );
    ctx.push_scope();
    let status = bind_moved(&mut ctx, name, value);
    let bound = &ctx.scopes.last().unwrap()[&name];
    same(bound, &expected);
    assert_eq!(bound.items.as_ptr(), allocation);
    assert!(status.items.is_empty());
    assert_eq!(status.diagnostics, expected.diagnostics);
    assert_eq!(status.error, expected.error);
    let mut owned_read = ctx.lookup_var(name);
    owned_read.items.clear();
    same(&ctx.lookup_var(name), &expected);
    ctx.pop_scope();
    same(&ctx.lookup_var(name), &context.policy_bindings["input"]);
}

#[test]
fn let_move_candidate_preserves_shadowing_reads_recovery_and_safe_points() {
    let context = context();
    for source in [
        "{ let local = input; local }",
        "{ let local = input; (local.label, local, local.extra) }",
        "{ let input = {label: \"inner\"}; input.label }",
        "{ let local = input; { let local = {label: local.label}; local.label } }",
        "declare function label(value) { let local = value; local.label } label(input)",
        "{ let local = input; seq:map((1,2), fn(n) => local.label) }",
        "try { let local = 1 / 0; local } catch (code, message) { input.label }",
        "try { let local = input; (local.label, 1 / 0) } catch (code, message) { input.label }",
        "{ let local = input; try { 1 / 0 } catch (code, message) { local.label } }",
    ] {
        let query = query(source, &context);
        let (expected, expected_points) = with_points(|| evaluate(&query, &context));
        let ((actual, points), stages) =
            measure(|| with_moved_lets(true, || with_points(|| evaluate(&query, &context))));
        assert!(actual.error.is_none(), "{source}: {actual:?}");
        same(&actual, &expected);
        assert_eq!(points, expected_points, "{source}");
        assert!(
            stages.contains_key("candidate/move-let-binding"),
            "{source}"
        );
        assert!(!stages.contains_key("copy/let-binding"));
    }
}

#[test]
fn let_move_candidate_preserves_diagnostic_order_and_failed_values() {
    for failed in [false, true] {
        let mut context = context();
        let value = context.policy_bindings.get_mut("input").unwrap();
        value.cursor = 2;
        value.chain = true;
        value.diagnostics.push(Diagnostic {
            code: "fixture.value".into(),
            ..Default::default()
        });
        if failed {
            value.error = Some(EvalError::TypeError("initializer failure"));
        }
        for source in [
            "{ let local = input; local }",
            "{ let local = input; local.label }",
            "{ let local = input; 1 / 0 }",
            "try { let local = input; local } catch (code, message) { message }",
        ] {
            let query = query(source, &context);
            let (expected, expected_points) = with_points(|| evaluate(&query, &context));
            let ((actual, points), stages) =
                measure(|| with_moved_lets(true, || with_points(|| evaluate(&query, &context))));
            same(&actual, &expected);
            assert_eq!(points, expected_points, "{source}");
            assert!(stages.contains_key("candidate/move-let-binding"));
        }
    }
}

#[test]
fn let_move_candidate_retains_fresh_native_owners_and_releases_them() {
    use crate::native::{NativeQueryFunction, NativeQueryRequest};
    use std::sync::{Mutex, Weak};
    type Log = Arc<Mutex<Vec<&'static str>>>;
    #[derive(Debug)]
    struct Fresh(Log, Arc<Mutex<Weak<()>>>);
    #[derive(Debug)]
    struct Owner(Arc<()>, Log);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.1.lock().unwrap().push("drop");
        }
    }
    impl QueryItemView for Owner {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.let-owner"
        }
        fn identity(&self) -> String {
            "fresh".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            assert_eq!(name, "label");
            assert_eq!(Arc::strong_count(&self.0), 1);
            self.1.lock().unwrap().push("field");
            Some(vec![text("fresh")])
        }
    }
    impl NativeQueryFunction for Fresh {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            assert!(request.control.check_scope(request.scope).is_ok());
            let owner = Arc::new(());
            *self.1.lock().unwrap() = Arc::downgrade(&owner);
            self.0.lock().unwrap().push("create");
            ItemStream::once(Item::native(Owner(owner, self.0.clone())))
        }
    }
    for return_node in [false, true] {
        for candidate in [false, true] {
            let log = Log::default();
            let weak = Arc::new(Mutex::new(Weak::new()));
            let mut context = context();
            context
                .native_functions
                .register("fixture.fresh", 0, Fresh(log.clone(), weak.clone()))
                .unwrap();
            let source = if return_node {
                "{ let local = native:call(\"fixture.fresh\"); local }"
            } else {
                "{ let local = native:call(\"fixture.fresh\"); local.label }"
            };
            let query = query(source, &context);
            let (result, stages) =
                measure(|| with_moved_lets(candidate, || evaluate(&query, &context)));
            assert!(result.error.is_none());
            assert_eq!(stages.contains_key("candidate/move-let-binding"), candidate);
            drop(context);
            if return_node {
                assert!(weak.lock().unwrap().upgrade().is_some());
                assert_eq!(*log.lock().unwrap(), ["create"]);
                assert_eq!(
                    result.items[0].view().unwrap().field("label"),
                    Some(vec![text("fresh")])
                );
            } else {
                assert_eq!(result.items, vec![text("fresh")]);
                assert!(weak.lock().unwrap().upgrade().is_none());
            }
            drop(result);
            assert!(weak.lock().unwrap().upgrade().is_none());
            assert_eq!(*log.lock().unwrap(), ["create", "field", "drop"]);
        }
    }
}

#[test]
fn let_move_candidate_preserves_cancellation_and_lower_child_budgets() {
    use crate::api::evaluate_with_control;
    use cem_ml::operation_control::{ExecutionScopeKind, ExecutionScopeRegistration};
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug)]
    struct CancelField(OperationControl, ExecutionScopeId, Arc<AtomicUsize>);
    impl QueryItemView for CancelField {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.let-cancel"
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
            "try { let local = input; (local.label, local.trigger.fire) } catch (code, message) { \"must-not-catch\" }"
        } else {
            "declare function down(n, value) { let local = value; if n == 0 { local.label } else { (local.label, down(n - 1, local)) } } try { down(17, input) } catch (code, message) { \"must-not-catch\" }"
        };
        let mut baseline = None;
        for candidate in [false, true] {
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
            let mut context = context();
            let Item::Record(fields) =
                &mut context.policy_bindings.get_mut("input").unwrap().items[0]
            else {
                unreachable!()
            };
            fields.insert(
                "trigger".into(),
                vec![Item::native(CancelField(
                    control.clone(),
                    child,
                    calls.clone(),
                ))],
            );
            let query = query(source, &context);
            let ((result, points), stages) = measure(|| {
                with_moved_lets(candidate, || {
                    with_points(|| evaluate_with_control(&query, &context, &control, child))
                })
            });
            assert!(result.error.is_some());
            assert!(result.items.is_empty());
            assert!(!result.diagnostics.is_empty());
            assert_eq!(stages.contains_key("candidate/move-let-binding"), candidate);
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
        }
    }
}
