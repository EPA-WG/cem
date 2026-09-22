//! Opt-in attribution only; no timing threshold or alternate production path.
use super::*;
use crate::{
    api::{compile, evaluate},
    compile_profile::{measure, Span, Stages},
};
use std::{hint::black_box, time::Instant};

thread_local! {
    static BORROW_CONTEXT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn with_candidate<T>(enabled: bool, operation: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            BORROW_CONTEXT.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(BORROW_CONTEXT.with(|v| v.replace(enabled)));
    operation()
}

pub(super) fn evaluate_candidate(
    renderer: &mut PlanRenderer<'_>,
    query: &CompiledQuery,
    control: &OperationControl,
    scope: ExecutionScopeId,
    protected: bool,
) -> ItemStream {
    if BORROW_CONTEXT.with(|v| v.get()) && query.binding_dependencies().is_some() {
        let _profile = Span::new("candidate/borrowed-evaluation");
        return crate::eval::Evaluator::evaluate_internal(
            query,
            &renderer.evaluation_context,
            control,
            scope,
            protected,
            None,
        );
    }
    // Identical to the production path. Opaque calls retain the mutable host
    // and complete context even while the test candidate is enabled.
    let context = renderer.evaluation_context.for_query(query);
    crate::eval::Evaluator::evaluate_internal(
        query,
        &context,
        control,
        scope,
        protected,
        Some(renderer),
    )
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
    for enabled in [false, true] {
        with_candidate(enabled, || {
            profile_strategy(
                &format!(
                    "{case}/{}",
                    if enabled {
                        "borrow-candidate"
                    } else {
                        "current"
                    }
                ),
                &mut operation,
                &verify,
            );
        });
    }
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
    assert_eq!(plan.diagnostics, expected.diagnostics);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
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
fn borrowing_candidate_preserves_focus_records_recovery_and_callbacks() {
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
        let baseline = render_compiled_template(&artifact, &data);
        let (candidate, stages) = measure(|| with_candidate(true, || render_compiled_template(&artifact, &data)));
        verify_plan(&candidate, &baseline);
        assert_eq!(stages.contains_key("copy/expression-context"), fallback, "{source}");
        assert_eq!(data.bindings["native"], native);
        assert_eq!(data.expression_scope.focus.as_ref().unwrap().identity(), native.items[0].identity());
    }
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
        for source in ["datadom.mode", "(datadom.mode, datadom.mode)"] {
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
            assert!(expected.items.iter().all(|item| item == &text("fixed")));
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
                "eligible-false-hook",
                format!(
                    "{{template @on=expression @into=content @match=false | {{$value}}}}{}",
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
