//! CEMQL-SORT-CALL-BUDGET: shared prerequisite exposed by portable XSLT sorting.
use cem_ml::operation_control::{
    ExecutionScopeKind, ExecutionScopeRegistration, OperationControl, ROOT_EXECUTION_SCOPE_ID,
};
use cem_ml::scheduler::ScopePolicy;
use cem_ql::{
    api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext},
    eval::{BudgetAxis, EvalError, ItemStream},
    native::{NativeQueryFunction, NativeQueryRequest},
};

fn sequential(calls: usize, workers: u32) -> ItemStream {
    let calls = vec!["tick()"; calls].join(",");
    let query = compile(
        &format!("declare function tick() {{ 1 }} ({calls})"),
        &CompileContext::default(),
    )
    .unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            scope_policy: ScopePolicy::host_root()
                .with_cpu_workers(workers)
                .with_queue_size(128),
            ..Default::default()
        },
    )
}

#[test]
fn completed_calls_do_not_exhaust_active_depth() {
    // Explicit policies reproduce low-core native hosts and the WASM default,
    // independently of the hardware running this regression.
    for workers in [1, 4] {
        let result = sequential(100, workers);
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(result.items.len(), 100);
    }
}

fn run(source: &str, queue: u32) -> ItemStream {
    let query = compile(source, &CompileContext::default()).unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            scope_policy: ScopePolicy::host_root()
                .with_cpu_workers(1)
                .with_queue_size(queue),
            ..Default::default()
        },
    )
}

#[test]
fn active_recursion_is_bounded_and_completed_recursion_releases_depth() {
    let declaration = "declare function down(n) { if n == 0 { 1 } else { down(n - 1) } }";
    let result = run(&format!("{declaration} (down(15), down(15))"), 128);
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items.len(), 2);
    let result = run(
        &format!("{declaration} try {{ down(16) }} catch (code, message) {{ 99 }}"),
        128,
    );
    assert_eq!(
        result.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::CallDepth))
    );
    assert!(result.items.is_empty());
}

#[test]
fn recovered_calls_release_depth_but_do_not_reset_total_call_budget() {
    let declaration = r#"declare function fail() { report:raise("test.fail", "recoverable") }"#;
    let calls = vec!["try { fail() } catch (code, message) { () }"; 100].join(",");
    let result = run(&format!("{declaration} ({calls})"), 128);
    assert!(result.error.is_none(), "{result:?}");
    let result = run(&format!("{declaration} ({calls})"), 1);
    assert_eq!(
        result.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::FunctionCalls))
    );
    assert!(result.items.is_empty());
}

#[derive(Debug)]
struct Identity;
impl NativeQueryFunction for Identity {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        request.arguments[0].clone()
    }
}

#[test]
fn repeated_native_calls_inside_lambdas_release_depth() {
    let mut functions = cem_ql::native::NativeFunctionRegistry::default();
    functions.register("test.identity", 1, Identity).unwrap();
    let items = vec!["1"; 100].join(",");
    let query = compile(
        &format!(r#"seq:map(({items}), fn(value) => native:call("test.identity", value))"#),
        &CompileContext::default(),
    )
    .unwrap();
    let result = evaluate(
        &query,
        &EvaluationContext {
            native_functions: functions,
            scope_policy: ScopePolicy::host_root()
                .with_cpu_workers(1)
                .with_queue_size(128),
            ..Default::default()
        },
    );
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items.len(), 100);
}

#[test]
fn explicit_execution_scopes_bound_context_call_budgets_and_isolate_siblings() {
    let root_policy = ScopePolicy::host_root()
        .with_cpu_workers(2)
        .with_queue_size(128);
    let control = OperationControl::with_root_policy(Default::default(), root_policy).unwrap();
    let context = EvaluationContext {
        scope_policy: root_policy,
        ..Default::default()
    };
    for (source, policy, axis) in [
        (
            "declare function down(n) { if n == 0 { 1 } else { down(n - 1) } } down(17)".to_owned(),
            root_policy.with_cpu_workers(1),
            BudgetAxis::CallDepth,
        ),
        (
            format!(
                "declare function tick() {{ () }} ({})",
                vec!["tick()"; 20].join(",")
            ),
            root_policy.with_queue_size(1),
            BudgetAxis::FunctionCalls,
        ),
    ] {
        let query = compile(&source, &CompileContext::default()).unwrap();
        let child = control
            .register_scope(
                ROOT_EXECUTION_SCOPE_ID,
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Template,
                    "limited",
                    policy,
                ),
            )
            .unwrap();
        let sibling = control
            .register_scope(
                ROOT_EXECUTION_SCOPE_ID,
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Template,
                    "ordinary",
                    root_policy,
                ),
            )
            .unwrap();
        let result = evaluate_with_control(&query, &context, &control, child);
        assert_eq!(result.error, Some(EvalError::BudgetExceeded(axis)));
        assert!(result.items.is_empty());
        for scope in [ROOT_EXECUTION_SCOPE_ID, sibling] {
            let result = evaluate_with_control(&query, &context, &control, scope);
            assert!(result.error.is_none(), "{result:?}");
        }
        // A stricter context also stays stricter beneath a broader host scope.
        let smaller = EvaluationContext {
            scope_policy: policy,
            ..context.clone()
        };
        assert_eq!(
            evaluate_with_control(&query, &smaller, &control, sibling).error,
            Some(EvalError::BudgetExceeded(axis))
        );
    }
}

#[test]
fn explicitly_shared_queries_keep_call_work_and_release_completed_depth() {
    let context = EvaluationContext {
        execution_budget: Some(Default::default()),
        scope_policy: ScopePolicy::host_root()
            .with_cpu_workers(1)
            .with_queue_size(8),
        ..Default::default()
    };
    let query = compile(
        "declare function down(n) { if n == 0 { 1 } else { down(n - 1) } } down(15)",
        &CompileContext::default(),
    )
    .unwrap();
    for _ in 0..8 {
        assert!(evaluate(&query, &context).error.is_none());
    }
    let failed = evaluate(&query, &context);
    assert_eq!(
        failed.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::FunctionCalls))
    );
    assert!(failed.items.is_empty());
}

#[derive(Debug)]
struct Reenter(cem_ql::ir::CompiledQuery);
impl NativeQueryFunction for Reenter {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        let context = EvaluationContext {
            execution_budget: Some(request.execution_budget.clone()),
            scope_policy: ScopePolicy::host_root()
                .with_cpu_workers(1)
                .with_queue_size(128),
            ..Default::default()
        };
        // A callback cannot turn an exhausted nested query into successful data.
        let _ = evaluate_with_control(&self.0, &context, request.control, request.scope);
        ItemStream::empty()
    }
}

#[test]
fn native_reentry_shares_active_depth_and_cannot_hide_a_nested_budget_failure() {
    let mut functions = cem_ql::native::NativeFunctionRegistry::default();
    functions
        .register(
            "test.reenter",
            0,
            Reenter(
                compile(
                    "declare function down(n) { if n == 0 { 1 } else { down(n - 1) } } down(15)",
                    &CompileContext::default(),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let query = compile(
        r#"try { native:call("test.reenter") } catch (code, message) { 99 }"#,
        &CompileContext::default(),
    )
    .unwrap();
    let result = evaluate(
        &query,
        &EvaluationContext {
            native_functions: functions,
            scope_policy: ScopePolicy::host_root()
                .with_cpu_workers(1)
                .with_queue_size(128),
            ..Default::default()
        },
    );
    assert_eq!(
        result.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::CallDepth))
    );
    assert!(result.items.is_empty());
}
