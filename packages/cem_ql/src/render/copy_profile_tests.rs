//! Opt-in attribution only; no timing threshold or alternate production path.
use super::*;
use crate::{
    api::{compile, evaluate},
    compile_profile::{measure, Span, Stages},
};
use std::{hint::black_box, time::Instant};

thread_local! {
    static FORCE_CONTEXT_COPY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn with_context_copy<T>(operation: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            FORCE_CONTEXT_COPY.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(FORCE_CONTEXT_COPY.with(|v| v.replace(true)));
    crate::eval::pipeline::field_profile_tests::with_copied_fields(|| {
        crate::eval::binding_profile_tests::with_copied_inputs(operation)
    })
}

pub(super) fn force_context_copy() -> bool {
    FORCE_CONTEXT_COPY.with(|v| v.get())
}

const VIEW: &str = include_str!("../../../cem-elements/demo/data-table-view.cemt");

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: impl IntoIterator<Item = (&'static str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(k, v)| (k.into(), vec![v]))
            .collect(),
    )
}
fn controls(count: usize) -> Item {
    // Synthetic browser control metadata, never an imported document.
    Item::Record(
        (0..count)
            .map(|i| {
                (
                    format!("control-{i}"),
                    vec![record([
                        ("name", text(&format!("control-{i}"))),
                        (
                            "attributes",
                            record([
                                ("role", text("option")),
                                ("scope", text("fixture")),
                                ("value", text("number")),
                            ]),
                        ),
                        (
                            "state",
                            record([("revision", text("3")), ("selected", text("false"))]),
                        ),
                    ])],
                )
            })
            .collect(),
    )
}
fn report(case: &str, samples: &[Stages]) {
    for (label, first) in &samples[0] {
        let mut times: Vec<_> = samples[1..]
            .iter()
            .map(|s| {
                assert_eq!(s[label].calls, first.calls, "{case}/{label}");
                s[label].elapsed.as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        println!("{case}\t{label}\tcalls={}\tfirst_ms={:.3}\tmedian_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}", first.calls, first.elapsed.as_secs_f64()*1000.0, times[2], times[0], times[4]);
    }
}
fn profile<T>(case: &str, mut operation: impl FnMut() -> T, verify: impl Fn(&T)) {
    with_context_copy(|| {
        profile_strategy(&format!("{case}/copied-context"), &mut operation, &verify);
    });
    crate::eval::pipeline::field_profile_tests::with_copied_fields(|| {
        crate::eval::binding_profile_tests::with_copied_inputs(|| {
            profile_strategy(&format!("{case}/copied-inputs"), &mut operation, &verify);
        });
        profile_strategy(&format!("{case}/copied-fields"), &mut operation, &verify);
    });
    profile_strategy(&format!("{case}/borrowed-fields"), &mut operation, &verify);
}

fn profile_strategy<T>(case: &str, mut operation: impl FnMut() -> T, verify: impl Fn(&T)) {
    let mut samples = Vec::new();
    for _ in 0..6 {
        let (result, stages) = measure(|| {
            let _total = Span::new("total");
            black_box(operation())
        });
        verify(&result);
        samples.push(stages);
    }
    report(case, &samples);
    // Record-free medians separate the workload from tracing overhead.
    let mut times = Vec::new();
    for _ in 0..6 {
        let start = Instant::now();
        let result = black_box(operation());
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        verify(&result);
    }
    times.remove(0);
    times.sort_by(f64::total_cmp);
    println!(
        "{case}\tunprofiled_median_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}",
        times[2], times[0], times[4]
    );
}
fn verify_plan(plan: &RenderPlan, expected: &RenderPlan) {
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    verify_plan_including_errors(plan, expected);
}

fn verify_plan_including_errors(plan: &RenderPlan, expected: &RenderPlan) {
    assert_eq!(plan.diagnostics, expected.diagnostics);
    assert_eq!(plan.host_attribute_updates, expected.host_attribute_updates);
    let actual = render_plan_to_html_with_source_map(plan);
    let expected = render_plan_to_html_with_source_map(expected);
    assert_eq!(actual.rendered, expected.rendered);
    assert_eq!(actual.source_map, expected.source_map);
    assert_eq!(actual.diagnostics, expected.diagnostics);
    assert_eq!(
        rmp_serde::to_vec_named(&actual.output_spans).unwrap(),
        rmp_serde::to_vec_named(&expected.output_spans).unwrap()
    );
}

#[test]
fn default_render_borrows_proven_expression_context() {
    let data = TemplateData::default().with_binding(
        "datadom",
        ItemStream::once(record([("mode", text("fixed"))])),
    );
    let artifact = compile_template(
        "{span | {$datadom.mode}}",
        &CompileTemplateOptions {
            host_bindings: vec!["datadom".into()],
            ..Default::default()
        },
    );
    assert!(artifact.diagnostics.is_empty());
    let (plan, stages) = measure(|| render_compiled_template(&artifact, &data));
    assert!(plan.diagnostics.is_empty());
    assert_eq!(render_plan_to_html(&plan), "<span>fixed</span>");
    assert!(!stages.contains_key("copy/expression-context"));
    // Input setup borrows bindings, while value reads still return owned copies.
    assert_eq!(stages["eval/input-bindings"].calls, 1);
    assert_eq!(stages["eval/borrowed-input-bindings"].calls, 1);
    assert_eq!(stages["copy/local-value"].calls, 1);
    assert!(!stages.contains_key("copy/field-input"));
    assert_eq!(stages["eval/borrowed-field-input"].calls, 1);
    assert_eq!(stages["copy/field-selected"].calls, 1);
}

#[test]
fn borrowing_preserves_focus_records_recovery_and_callbacks() {
    use crate::native::{NativeQueryFunction, NativeQueryRequest};
    #[derive(Debug)]
    struct Echo;
    impl NativeQueryFunction for Echo {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            assert!(request.control.check_scope(request.scope).is_ok());
            assert!(!request.source_map.frames.is_empty());
            assert_eq!(
                request.current_item.unwrap().identity(),
                request.arguments[0].items[0].identity()
            );
            request.arguments[0].clone()
        }
    }
    let native = evaluate(
        &compile(
            r#"data:read("<name>ivy</name>", "xml").root.children"#,
            &Default::default(),
        )
        .unwrap(),
        &Default::default(),
    );
    assert!(native.error.is_none());
    let mut data = TemplateData::default()
        .with_binding("native", native.clone())
        .with_binding("island", ItemStream::once(controls(2)))
        .with_binding(
            "datadom",
            ItemStream::once(record([("mode", text("fixed"))])),
        );
    data.expression_scope.focus = Some(native.items[0].clone());
    data.native_functions
        .register("urn:profile:echo", 1, Echo)
        .unwrap();
    for (source, fallback) in [
        ("{$record:entries(datadom).key}", false),
        ("{$record:entries(island).key}", false),
        ("{$dom:text()} {$same_node(native, native)}", false),
        ("{cem:variable @name=label @select='\"outer\"'}{span | {cem:variable @name=label @select='label + \"-inner\"'}{$label}}{$label}", false),
        ("{try | {$1 / 0}{catch | {span | {$datadom.mode}}}}", false),
        ("{template @mode=probe @match=true | {$dom:text(node)} {$datadom.mode}}{$cemt:apply_templates(native, \"probe\")}", true),
        ("{template @mode=probe @match=true | {$dom:text(node)} {$datadom.mode}}{$seq:map((1), fn(n) => cemt:apply_templates(native, \"probe\"))}", true),
        ("{$native:call(\"urn:profile:echo\", native)}", true),
    ] {
        let options = CompileTemplateOptions { host_bindings: data.bindings.keys().cloned().collect(), ..Default::default() };
        // Exercise a portable artifact reload as well as direct compilation.
        use crate::template_artifact::{compile_template_artifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode};
        let portable = compile_template_artifact(source, &options, TemplateArtifactSourceMapMode::Dev);
        let artifact = portable.reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(portable.identity.source_hash.clone()),
            host_bindings: options.host_bindings,
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        }).unwrap();
        assert!(artifact.diagnostics.is_empty(), "{source}: {:?}", artifact.diagnostics);
        let baseline = with_context_copy(|| render_compiled_template(&artifact, &data));
        let (borrowed, stages) = measure(|| render_compiled_template(&artifact, &data));
        verify_plan(&borrowed, &baseline);
        assert_eq!(stages.contains_key("copy/expression-context"), fallback, "{source}");
        assert_eq!(data.bindings["native"], native);
        assert_eq!(data.expression_scope.focus.as_ref().unwrap().identity(), native.items[0].identity());
    }
}

#[test]
fn borrowing_preserves_reader_retention_across_renders() {
    use crate::xpath::functions::XPathQueryItem;
    use std::sync::Arc;
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
    let context = EvaluationContext::default();
    let source = r#"data:read("<name>ivy</name>", "xml", "xpath").root"#;
    let query = compile(source, &Default::default()).unwrap();
    let native = evaluate(&query, &context);
    assert!(native.error.is_none());
    let original = owner(&native.items[0]);
    let data = TemplateData {
        data_readers: context.data_readers.clone(),
        ..Default::default()
    }
    .with_binding("retained", native.clone());
    let artifact = compile_template(
        &format!("{{${source}}}"),
        &CompileTemplateOptions {
            host_bindings: vec!["retained".into()],
            ..Default::default()
        },
    );
    assert!(artifact.diagnostics.is_empty());
    let baseline = with_context_copy(|| render_compiled_template(&artifact, &data));
    for _ in 0..2 {
        let (plan, stages) = measure(|| render_compiled_template(&artifact, &data));
        verify_plan(&plan, &baseline);
        assert_eq!(render_plan_to_html(&plan), "<name>ivy</name>");
        let RenderPlanNode::Reference { reference, .. } = &plan.nodes[0] else {
            panic!("native result reference expected");
        };
        assert!(Arc::ptr_eq(&owner(&reference.values()[0]), &original));
        assert!(!stages.contains_key("copy/expression-context"));
    }
    // Clearing the shared capability invalidates future reads, while the input
    // binding independently retains its original tree and identity.
    data.data_readers.clear();
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty());
    let RenderPlanNode::Reference { reference, .. } = &plan.nodes[0] else {
        panic!("native result reference expected");
    };
    assert!(!Arc::ptr_eq(&owner(&reference.values()[0]), &original));
    assert_eq!(
        data.bindings["retained"].items[0].identity(),
        native.items[0].identity()
    );
}

#[test]
fn borrowing_preserves_protected_failures_and_recovery() {
    for (source, fallback, recovered) in [
        (
            r#"{b | prefix}{$report:raise("fixture.failure", "bad value")}{i | suffix}"#,
            false,
            false,
        ),
        (
            r#"{try | {b | prefix}{$native:call("fixture.missing")}{catch | must-not-recover}}"#,
            true,
            false,
        ),
        (
            r#"{cem:variable @name=label @select='"outer"'}{try | {b | prefix}{cem:variable @name=label @select='"inner"'}{$report:raise("fixture.failure", label)}{catch @as=e | {b | {$e.code}}{i | {$label}}}}"#,
            false,
            true,
        ),
    ] {
        let artifact = compile_template(source, &Default::default());
        assert!(
            artifact.diagnostics.is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        let data = TemplateData::default();
        let render = || render_compiled_template_internal(&artifact, &data, None, None, true);
        let baseline = with_context_copy(render);
        let (actual, stages) = measure(render);
        verify_plan_including_errors(&actual.plan, &baseline.plan);
        assert_eq!(stages.contains_key("copy/expression-context"), fallback);
        if recovered {
            assert!(actual.failure.is_none());
            assert!(actual.plan.diagnostics.is_empty());
            assert_eq!(
                render_plan_to_html(&actual.plan),
                "<b>fixture.failure</b><i>outer</i>"
            );
        } else {
            assert!(actual.plan.nodes.is_empty());
            let failure = actual.failure.expect("propagated failure");
            let baseline = baseline.failure.unwrap();
            assert_eq!(failure.error, baseline.error);
            assert_eq!(failure.diagnostic, baseline.diagnostic);
            assert!(!failure.diagnostic.source_map.unwrap().frames.is_empty());
            assert_eq!(failure.error.is_recoverable(), !fallback);
        }
    }
}

#[test]
fn borrowing_preserves_scoped_cancellation_and_budget_failure() {
    use crate::eval::{QueryItemView, QueryItemViewKind};
    use cem_ml::operation_control::{
        ExecutionScopeKind, ExecutionScopeRegistration, ROOT_EXECUTION_SCOPE_ID,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    // A native field can signal cancellation during an otherwise closed query.
    // It does not need (or receive) a mutable CEMT host.
    #[derive(Debug)]
    struct CancelField(OperationControl, ExecutionScopeId, Arc<AtomicUsize>);
    impl QueryItemView for CancelField {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.cancel-field"
        }
        fn identity(&self) -> String {
            "cancel-field".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            if name != "value" {
                return None;
            }
            self.2.fetch_add(1, Ordering::SeqCst);
            self.0.cancel_scope(self.1, None, None).unwrap();
            Some(vec![text("must-not-escape")])
        }
    }
    for cancelled in [false, true] {
        let expression = if cancelled {
            "trigger.value"
        } else {
            "declare function down(n) { if n == 0 { 1 } else { down(n - 1) } } down(17)"
        };
        let artifact = compile_template(
            &format!(
                "{{try | {{b | {{$\"prefix\"}}}}{{${expression}}}{{catch | must-not-recover}}}}"
            ),
            &CompileTemplateOptions {
                host_bindings: vec!["trigger".into()],
                ..Default::default()
            },
        );
        assert!(
            artifact.diagnostics.is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        let mut expected = None;
        for copy in [true, false] {
            let root_policy = ScopePolicy::host_root()
                .with_cpu_workers(2)
                .with_queue_size(128);
            let control =
                OperationControl::with_root_policy(Default::default(), root_policy).unwrap();
            let child = control
                .register_scope(
                    ROOT_EXECUTION_SCOPE_ID,
                    ExecutionScopeRegistration::inherited(
                        ExecutionScopeKind::Template,
                        "limited",
                        root_policy.with_cpu_workers(1),
                    ),
                )
                .unwrap();
            let sibling = control
                .register_scope(
                    ROOT_EXECUTION_SCOPE_ID,
                    ExecutionScopeRegistration::inherited(
                        ExecutionScopeKind::Template,
                        "sibling",
                        root_policy,
                    ),
                )
                .unwrap();
            let calls = Arc::new(AtomicUsize::new(0));
            let data = TemplateData::default().with_binding(
                "trigger",
                ItemStream::once(Item::native(CancelField(
                    control.clone(),
                    child,
                    calls.clone(),
                ))),
            );
            let render =
                || render_compiled_template_with_control(&artifact, &data, &control, child);
            let (plan, stages) = measure(|| {
                if copy {
                    with_context_copy(render)
                } else {
                    render()
                }
            });
            assert!(plan.nodes.is_empty(), "{:?}", plan.nodes);
            assert!(!plan.diagnostics.is_empty());
            assert_eq!(stages.contains_key("copy/expression-context"), copy);
            assert_eq!(calls.load(Ordering::SeqCst), usize::from(cancelled));
            assert_eq!(control.check_scope(child).is_err(), cancelled);
            if let Some(expected) = &expected {
                verify_plan_including_errors(&plan, expected);
            } else {
                expected = Some(plan);
            }
            assert_eq!(control.memory_charged(child).unwrap(), 0);
            // Neither cancellation nor the lower child budget poisons its peers.
            let query = compile(
                "declare function down(n) { if n == 0 { 1 } else { down(n - 1) } } down(17)",
                &Default::default(),
            )
            .unwrap();
            for scope in [ROOT_EXECUTION_SCOPE_ID, sibling] {
                assert!(control.check_scope(scope).is_ok());
                let result = crate::api::evaluate_with_control(
                    &query,
                    &EvaluationContext {
                        scope_policy: root_policy,
                        ..Default::default()
                    },
                    &control,
                    scope,
                );
                assert!(result.error.is_none(), "{result:?}");
                assert_eq!(result.items, vec![Item::Atomic(AtomValue::Integer(1))]);
            }
        }
    }
}

#[test]
fn copied_input_baseline_preserves_renderer_contracts() {
    crate::eval::binding_profile_tests::with_copied_inputs(|| {
        borrowing_preserves_focus_records_recovery_and_callbacks();
        borrowing_preserves_reader_retention_across_renders();
        borrowing_preserves_protected_failures_and_recovery();
        borrowing_preserves_scoped_cancellation_and_budget_failure();
    });
}

#[test]
fn copied_field_baseline_preserves_renderer_contracts() {
    crate::eval::pipeline::field_profile_tests::with_copied_fields(|| {
        borrowing_preserves_focus_records_recovery_and_callbacks();
        borrowing_preserves_reader_retention_across_renders();
        borrowing_preserves_protected_failures_and_recovery();
        borrowing_preserves_scoped_cancellation_and_budget_failure();
    });
}

#[test]
#[ignore = "profiling fixture: --release --lib profile_render_copies -- --ignored --nocapture --test-threads=1"]
fn profile_render_copies() {
    for count in [0, 256] {
        let bindings = BTreeMap::from([
            ("island".into(), ItemStream::once(controls(count))),
            (
                "datadom".into(),
                ItemStream::once(record([
                    ("mode", text("fixed")),
                    ("island", controls(count)),
                ])),
            ),
        ]);
        let context = EvaluationContext {
            policy_bindings: bindings.clone(),
            ..Default::default()
        };
        for source in [
            "datadom",
            "datadom.mode",
            "datadom.island",
            "(datadom.mode, datadom.mode)",
            "declare function label(value) { value.mode } label(datadom)",
        ] {
            let query = compile(
                source,
                &CompileContext {
                    policy_bindings: bindings.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
            let expected = evaluate(&query, &context);
            assert!(expected.error.is_none());
            match source {
                "datadom" => assert_eq!(expected, bindings["datadom"]),
                "datadom.island" => assert_eq!(expected, bindings["island"]),
                _ => assert!(expected.items.iter().all(|item| item == &text("fixed"))),
            }
            profile(
                &format!("query/{source}/{count}"),
                || evaluate(&query, &context),
                |result| {
                    assert_eq!(result, &expected);
                    assert_eq!(result.diagnostics, expected.diagnostics);
                },
            );
        }
        for (name, source) in [
            ("literal", "{span | fixed}".repeat(32)),
            ("selected-member", "{span | {$datadom.mode}}".repeat(32)),
            (
                "named-call",
                format!(
                    "{{template @name=probe | {{span | fixed}}}}{}",
                    "{call @template=probe}".repeat(32)
                ),
            ),
            (
                "match-call",
                format!(
                    "{{template @mode=probe @match=true | {{span | fixed}}}}{}",
                    "{apply-templates @select=1 @mode=probe}".repeat(32)
                ),
            ),
            (
                "shadow",
                "{span | {cem:variable @name=island @select=1}fixed}".repeat(32),
            ),
            (
                "try",
                "{try | {span | fixed}{catch | {span | error}}}".repeat(32),
            ),
            (
                "try-recover",
                "{try | {span | prefix}{$1 / 0}{catch | {span | fixed}}}".repeat(32),
            ),
            (
                "try-scan",
                "{try | {$1 / 0}{catch @test=false | wrong}{catch @test=false | wrong}{catch | {span | fixed}}}".repeat(32),
            ),
            (
                "eligible-false-hook",
                format!(
                    "{{template @on=expression @into=content @match=false | {{$value}}}}{}",
                    "{span | {$\"fixed\"}}".repeat(32)
                ),
            ),
            (
                "eligible-true-hook",
                format!(
                    "{{template @on=expression @into=content | {{$value}}}}{}",
                    "{span | {$\"fixed\"}}".repeat(32)
                ),
            ),
            (
                "eligible-hook-scan",
                format!(
                    "{{template @on=expression @into=content @match=false | wrong}}{{template @on=expression @into=content @match=false | wrong}}{{template @on=expression @into=content @priority=-1 | {{$value}}}}{}",
                    "{span | {$\"fixed\"}}".repeat(32)
                ),
            ),
        ] {
            let artifact = compile_template(
                &source,
                &CompileTemplateOptions {
                    host_bindings: bindings.keys().cloned().collect(),
                    ..Default::default()
                },
            );
            assert!(
                artifact.diagnostics.is_empty(),
                "{name}: {:?}",
                artifact.diagnostics
            );
            let input = TemplateData {
                bindings: bindings.clone(),
                ..Default::default()
            };
            let expected = render_compiled_template(&artifact, &input);
            assert_eq!(
                render_plan_to_html(&expected),
                "<span>fixed</span>".repeat(32),
                "{name}"
            );
            profile(
                &format!("template/{name}/{count}"),
                || render_compiled_template(&artifact, &input),
                |plan| verify_plan(plan, &expected),
            );
        }
    }

    let full = compile_template(
        VIEW,
        &CompileTemplateOptions {
            host_bindings: vec!["island".into()],
            ..Default::default()
        },
    );
    assert!(full.diagnostics.is_empty());
    let import = "{cem-data @name=document @select=source @type=\"{$format}\"}";
    assert_eq!(VIEW.matches(import).count(), 1);
    let retained = compile_template(
        &VIEW.replace(import, "{cem:variable @name=document @select=retained}"),
        &CompileTemplateOptions {
            host_bindings: vec!["island".into(), "retained".into()],
            ..Default::default()
        },
    );
    assert!(retained.diagnostics.is_empty());
    for (format, source, column) in [
        (
            "xml",
            "<r><row qty='10'>🍒</row><row qty='2'>🍋</row><row qty='3'>🍌</row></r>",
            "@qty",
        ),
        ("csv", "qty,fruit\n10,🍒\n2,🍋\n3,🍌", "qty"),
        (
            "yaml",
            "- qty: 10\n  fruit: 🍒\n- qty: 2\n  fruit: 🍋\n- qty: 3\n  fruit: 🍌",
            "qty",
        ),
        (
            "json",
            r#"[{"qty":10,"fruit":"🍒"},{"qty":2,"fruit":"🍋"},{"qty":3,"fruit":"🍌"}]"#,
            "qty",
        ),
    ] {
        let context = EvaluationContext {
            policy_bindings: BTreeMap::from([
                ("source".into(), ItemStream::once(text(source))),
                ("format".into(), ItemStream::once(text(format))),
            ]),
            ..Default::default()
        };
        let query = compile(
            "data:read(source, format)",
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        // The shared CEM-ML reader owns every external-format conversion.
        let document = evaluate(&query, &context);
        assert!(document.error.is_none());
        let root_query = compile(
            "document.root",
            &CompileContext {
                policy_bindings: BTreeMap::from([("document".into(), document.clone())]),
                ..Default::default()
            },
        )
        .unwrap();
        let root_context = EvaluationContext {
            policy_bindings: BTreeMap::from([("document".into(), document.clone())]),
            ..Default::default()
        };
        let root = evaluate(&root_query, &root_context);
        let identity = root.items[0].identity().expect("retained native root");
        for count in [0, 256] {
            let input = TemplateData::default()
                .with_binding("format", ItemStream::once(text(format)))
                .with_binding("island", ItemStream::once(controls(count)))
                .with_binding(
                    "datadom",
                    ItemStream::once(record([
                        (
                            "payload",
                            record([("nodes", record([("text", text(source))]))]),
                        ),
                        (
                            "slices",
                            record([("column", text(column)), ("mode", text("number"))]),
                        ),
                    ])),
                );
            let expected = render_compiled_template(&full, &input);
            let html = render_plan_to_html(&expected);
            let body = html.split_once("<tbody>").unwrap().1;
            assert!(body.find('🍋').unwrap() < body.find('🍌').unwrap());
            assert!(body.find('🍌').unwrap() < body.find('🍒').unwrap());
            profile(
                &format!("viewer/{format}/{count}"),
                || render_compiled_template(&full, &input),
                |plan| verify_plan(plan, &expected),
            );
            let input = input.with_binding("retained", document.clone());
            let reused = render_compiled_template(&retained, &input);
            assert_eq!(render_plan_to_html(&reused), html);
            profile(
                &format!("retained/{format}/{count}"),
                || render_compiled_template(&retained, &input),
                |plan| verify_plan(plan, &reused),
            );
            assert_eq!(
                evaluate(&root_query, &root_context).items[0]
                    .identity()
                    .as_deref(),
                Some(identity.as_str())
            );
        }
    }
}
