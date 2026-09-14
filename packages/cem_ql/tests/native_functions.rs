//! XSLT-VIEW-NATIVE-CALL / XSLT-VIEW-NATIVE-CEMT: generic native invocation.
use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, BudgetAxis, EvalError, Item, ItemStream};
use cem_ql::ir::{deserialize::IrDeserializer, serialize::IrSerializer};
use cem_ql::native::{NativeFunctionRegistry, NativeQueryFunction, NativeQueryRequest};
use cem_ql::render::{
    compile_template, render_compiled_template, render_compiled_template_with_control,
    render_plan_to_html, CompileTemplateOptions, TemplateData,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

#[derive(Debug)]
struct Echo(Arc<AtomicUsize>);
impl NativeQueryFunction for Echo {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        self.0.fetch_add(1, Ordering::SeqCst);
        assert!(request.control.check_scope(request.scope).is_ok());
        assert!(!request.source_map.frames.is_empty());
        assert!(request.max_result_items > 0);
        if request.arguments.len() == 3 {
            assert!(request.arguments[0].items.is_empty());
            assert_eq!(request.arguments[1].items.len(), 2);
        }
        request.arguments.last().cloned().unwrap_or_default()
    }
}

fn registry() -> (NativeFunctionRegistry, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = NativeFunctionRegistry::default();
    for arity in [0, 1, 3] {
        registry
            .register("urn:test:echo", arity, Echo(calls.clone()))
            .unwrap();
    }
    (registry, calls)
}

fn run(source: &str, functions: NativeFunctionRegistry) -> ItemStream {
    let query = compile(source, &CompileContext::default()).unwrap();
    evaluate(
        &query,
        &EvaluationContext {
            native_functions: functions,
            ..Default::default()
        },
    )
}

#[test]
fn native_sequences_keep_empty_arguments_native_identity_and_provenance() {
    let (functions, calls) = registry();
    let result = run(
        r#"{ let node = data:read("<r/>", "xml").root;
        native:call("urn:test:echo", (), (1, 2), node) is node }"#,
        functions.clone(),
    );
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items, [Item::Atomic(AtomValue::Boolean(true))]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let result = run(
        r#"native:call("urn:test:echo", data:read("<r/>", "xml").root)"#,
        functions,
    );
    assert!(result.items[0].source_map().is_some());
    assert!(result.items[0].identity().is_some());
}

#[test]
fn registry_is_explicit_rejects_duplicates_and_does_not_serialize_callbacks() {
    let (mut functions, calls) = registry();
    assert!(functions
        .register("urn:test:echo", 1, Echo(calls.clone()))
        .is_err());
    assert!(functions.register("", 0, Echo(calls.clone())).is_err());
    let query = compile(
        r#"native:call("urn:test:echo", "🍒")"#,
        &CompileContext::default(),
    )
    .unwrap();
    let reloaded = IrDeserializer::deserialize(&IrSerializer::serialize(&query)).unwrap();
    let missing = evaluate(&reloaded, &EvaluationContext::default());
    assert!(missing.items.is_empty());
    assert!(missing
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.native_function_unavailable"));
    let result = evaluate(
        &reloaded,
        &EvaluationContext {
            native_functions: functions.clone(),
            ..Default::default()
        },
    );
    assert_eq!(result.items, [string("🍒")]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for source in [
        r#"native:call("urn:test:echo", 1, 2)"#,
        r#"native:call((), 1)"#,
        r#"native:call(1, 1)"#,
    ] {
        let result = run(source, functions.clone());
        assert!(result.error.is_some(), "{source}");
        assert!(result.items.is_empty());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.source_map.as_ref().is_some_and(|s| !s.frames.is_empty())));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn failing_arguments_stop_before_later_callbacks_and_preserve_catch_codes() {
    let (functions, calls) = registry();
    let result = run(
        r#"try { native:call("urn:test:echo", 1 / 0, native:call("urn:test:echo")) }
        catch (code, message) { code }"#,
        functions,
    );
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.items, [string("cem.ql.type_error")]);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[derive(Debug)]
struct Cancel;
impl NativeQueryFunction for Cancel {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        request.control.cancel_root(None, None).unwrap();
        ItemStream::once(string("must-not-escape"))
    }
}

#[test]
fn native_controls_are_shared_and_cannot_be_caught_or_expose_prefix_output() {
    let (mut functions, calls) = registry();
    functions.register("urn:test:cancel", 0, Cancel).unwrap();
    let query = compile(
        r#"try { ("prefix", native:call("urn:test:cancel")) } catch (code, message) { "caught" }"#,
        &CompileContext::default(),
    )
    .unwrap();
    let control = OperationControl::default();
    let context = EvaluationContext {
        native_functions: functions,
        ..Default::default()
    };
    let result = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert!(matches!(
        result.error,
        Some(EvalError::Cancelled | EvalError::Control(_))
    ));
    assert!(result.items.is_empty());
    let query = compile(
        r#"native:call("urn:test:echo")"#,
        &CompileContext::default(),
    )
    .unwrap();
    assert!(
        evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID)
            .error
            .is_some()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[derive(Debug)]
struct Oversized;
impl NativeQueryFunction for Oversized {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        ItemStream::from_items(vec![string("x"); request.max_result_items as usize + 1])
    }
}

#[test]
fn native_result_and_call_budgets_are_enforced_outside_catch_regions() {
    let (mut functions, _) = registry();
    functions.register("urn:test:large", 0, Oversized).unwrap();
    let context = EvaluationContext {
        native_functions: functions,
        scope_policy: ScopePolicy::host_root().with_queue_size(2),
        ..Default::default()
    };
    for (source, budget) in [
        (
            r#"try { native:call("urn:test:large") } catch (code, message) { 0 }"#.into(),
            BudgetAxis::ItemsPerStage,
        ),
        (
            format!(
                "({})",
                vec![r#"native:call("urn:test:echo")"#; 33].join(",")
            ),
            BudgetAxis::FunctionCalls,
        ),
    ] {
        let query = compile(&source, &CompileContext::default()).unwrap();
        let result = evaluate(&query, &context);
        assert_eq!(
            result.error,
            Some(EvalError::BudgetExceeded(budget)),
            "{result:?}"
        );
        assert!(result.items.is_empty());
    }
}

#[test]
fn cemt_calls_retain_native_functions_across_loops_named_calls_and_renders() {
    let (functions, calls) = registry();
    let source = r#"{template @name=row | {param @name=value}{body | {b | {$native:call("urn:test:echo", value)}}}}
        {for-each @select=values @as=item | {call @template=row @with:value='{item}'}}"#;
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["values".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for values in [vec![string("🍒"), string("🍋")], vec![string("new")]] {
        let expected = values
            .iter()
            .map(|item| match item {
                Item::Atomic(AtomValue::String(s)) => format!("<b>{s}</b>"),
                _ => unreachable!(),
            })
            .collect::<String>();
        let data = TemplateData {
            native_functions: functions.clone(),
            ..Default::default()
        }
        .with_binding("values", ItemStream::from_items(values));
        let plan = render_compiled_template(&artifact, &data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(
            render_plan_to_html(&plan)
                .split_whitespace()
                .collect::<String>(),
            expected
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn cemt_native_cancellation_cannot_render_a_catch_branch() {
    let mut functions = NativeFunctionRegistry::default();
    functions.register("urn:test:cancel", 0, Cancel).unwrap();
    let artifact = compile_template(
        r#"{try | {b | prefix}{$native:call("urn:test:cancel")}{catch @as=e | caught}}"#,
        &Default::default(),
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let plan = render_compiled_template_with_control(
        &artifact,
        &TemplateData {
            native_functions: functions,
            ..Default::default()
        },
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(plan.nodes.is_empty());
    assert!(!plan.diagnostics.is_empty());
}

#[derive(Debug)]
struct Fail;
impl NativeQueryFunction for Fail {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        request.raise("urn:test:bad-value", "invalid input")
    }
}

#[test]
fn native_failures_are_source_mapped_and_recover_without_later_invocation() {
    let (mut functions, calls) = registry();
    functions.register("urn:test:fail", 0, Fail).unwrap();
    let failed = run(r#"native:call("urn:test:fail")"#, functions.clone());
    assert!(matches!(failed.error, Some(EvalError::Raised { .. })));
    assert!(failed.items.is_empty());
    assert!(failed
        .diagnostics
        .iter()
        .any(|d| d.code == "urn:test:bad-value"
            && d.source_map.as_ref().is_some_and(|m| !m.frames.is_empty())));
    let result = run(
        r#"try { ("prefix", native:call("urn:test:fail"), native:call("urn:test:echo")) }
        catch (code, message) { (code, message) }"#,
        functions,
    );
    assert!(result.error.is_none(), "{result:?}");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(
        result.items,
        [string("urn:test:bad-value"), string("invalid input")]
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn cemt_binary_reload_requires_explicit_capabilities_and_preserves_recovery() {
    use cem_ql::template_artifact::{
        compile_template_artifact, CompiledTemplateArtifact, TemplateArtifactLoadContext,
        TemplateArtifactSourceMapMode,
    };
    let source =
        r#"{try | {b | prefix}{$native:call("urn:test:fail")}{catch @as=e | {p | {$e.code}}}}"#;
    let artifact = compile_template_artifact(
        source,
        &Default::default(),
        TemplateArtifactSourceMapMode::Dev,
    );
    let reloaded = CompiledTemplateArtifact::from_bytes(artifact.bytes)
        .unwrap()
        .reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(cem_ml::content_cache::ContentHash::from_blake3(
                source.as_bytes(),
            )),
            host_bindings: vec![],
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    let missing = render_compiled_template(&reloaded, &TemplateData::default());
    assert!(
        missing.nodes.is_empty(),
        "a missing capability is not a recoverable data error"
    );
    let mut functions = NativeFunctionRegistry::default();
    functions.register("urn:test:fail", 0, Fail).unwrap();
    let plan = render_compiled_template(
        &reloaded,
        &TemplateData {
            native_functions: functions,
            ..Default::default()
        },
    );
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(render_plan_to_html(&plan), "<p>urn:test:bad-value</p>");
}

#[test]
fn external_template_call_receives_the_native_registry_inside_recovery() {
    use cem_ml::source_map::SourceMapStack;
    use cem_ql::render::{
        render_compiled_template_with_calls, RenderPlanAttribute, TemplateArtifact,
        TemplateCallHandler, TemplateCallResult,
    };
    struct Calls(TemplateArtifact);
    impl TemplateCallHandler for Calls {
        fn handles(&self, tag: &str) -> bool {
            tag == "external"
        }
        fn call(
            &self,
            _: &[RenderPlanAttribute],
            _: &SourceMapStack,
            data: &TemplateData,
            protected: bool,
        ) -> TemplateCallResult {
            assert!(protected);
            render_compiled_template_with_calls(&self.0, data, None, self, protected)
        }
    }
    let calls = Calls(compile_template(
        r#"{b | partial}{$native:call("urn:test:fail")}"#,
        &Default::default(),
    ));
    let root = compile_template(
        "{try | {external}{catch @as=e | {p | {$e.code}}}}",
        &Default::default(),
    );
    let mut functions = NativeFunctionRegistry::default();
    functions.register("urn:test:fail", 0, Fail).unwrap();
    let result = render_compiled_template_with_calls(
        &root,
        &TemplateData {
            native_functions: functions,
            ..Default::default()
        },
        None,
        &calls,
        false,
    );
    assert!(result.failure.is_none(), "{:?}", result.failure);
    assert!(
        result.plan.diagnostics.is_empty(),
        "{:?}",
        result.plan.diagnostics
    );
    assert_eq!(
        render_plan_to_html(&result.plan),
        "<p>urn:test:bad-value</p>"
    );
}

#[test]
fn argument_diagnostics_are_retained_once_on_success_and_later_argument_failure() {
    let (functions, _) = registry();
    for source in [
        r#"native:call("urn:test:echo", report:emit("notice", "hello", "warning"))"#,
        r#"native:call("urn:test:echo", report:emit("notice", "hello", "warning"), 1 / 0)"#,
    ] {
        let result = run(source, functions.clone());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.code == "notice")
                .count(),
            1,
            "{result:?}"
        );
    }
}
