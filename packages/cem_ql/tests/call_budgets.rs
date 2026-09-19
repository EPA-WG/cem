//! CEMQL-SORT-CALL-BUDGET: shared prerequisite exposed by portable XSLT sorting.
use cem_ml::scheduler::ScopePolicy;
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
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
