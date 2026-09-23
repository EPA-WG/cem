//! Authored body attribution and a test-only inspection registry candidate.
use super::*;
use crate::{
    api::{compile, evaluate},
    compile_profile::{measure, Span},
    eval::inspection_profile_tests::{with_prepared, Prepared},
};
use std::{hint::black_box, time::Instant};

fn profile<T>(name: &str, mut run: impl FnMut() -> T, verify: impl Fn(&T)) {
    let mut samples = Vec::new();
    for _ in 0..6 {
        let (result, stages) = measure(|| {
            let _span = Span::new("total");
            black_box(run())
        });
        verify(&result);
        samples.push(stages);
    }
    for (label, first) in &samples[0] {
        let times: Vec<_> = samples[1..]
            .iter()
            .map(|sample| {
                sample
                    .get(label)
                    .map_or(0.0, |stage| stage.elapsed.as_secs_f64() * 1000.0)
            })
            .collect();
        let mut sorted = times;
        sorted.sort_by(f64::total_cmp);
        println!("{name}\t{label}\tfirst_calls={}\twarm_calls={}\tfirst_ms={:.3}\tmedian_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}", first.calls, samples[1].get(label).map_or(0, |stage| stage.calls), first.elapsed.as_secs_f64() * 1000.0, sorted[2], sorted[0], sorted[4]);
    }
    let mut samples = Vec::new();
    for _ in 0..6 {
        let start = Instant::now();
        let result = black_box(run());
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        verify(&result);
    }
    samples.remove(0);
    samples.sort_by(f64::total_cmp);
    println!(
        "{name}\tunprofiled_median_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}",
        samples[2], samples[0], samples[4]
    );
}

fn authored(tag: &str) -> &'static str {
    // Select the exact embedded authored template; no document data is decoded.
    let page = include_str!("../../../cem-elements/demo/cell-overrides.html");
    page.split_once(&format!("<cem-element tag=\"{tag}\">"))
        .unwrap()
        .1
        .split_once("<template type=\"text/cem-ml\">")
        .unwrap()
        .1
        .split_once("</template>")
        .unwrap()
        .0
}

fn hooks(tag: &str, controls: usize) -> (TemplateArtifact, TemplateData) {
    let owner = cem_ml::import::import_data(
        "<name>ivy<em>saur</em></name>",
        "xml",
        "cem",
        "memory:label",
    )
    .unwrap();
    let node = crate::xpath::functions::XPathQueryItem::from_node(
        cem_ml::validation::xpath::XPathNativeNode::cem_node(owner, 1).unwrap(),
    );
    let data = TemplateData::default()
        .with_binding(
            "island",
            ItemStream::once(copy_profile_tests::controls(controls)),
        )
        .with_binding(
            "count",
            ItemStream::once(Item::Atomic(AtomValue::Integer(2))),
        )
        .with_binding(
            "day",
            ItemStream::once(Item::Atomic(AtomValue::String("2024-02-29".into()))),
        )
        .with_binding("label", ItemStream::once(node));
    let artifact = compile_template(
        authored(tag),
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    (artifact, data)
}

#[test]
fn authored_hooks_and_inspection_keep_output_and_attribution() {
    for (tag, expected) in [
        ("cem-native-value-card", "Text-only label: ivysaur"),
        ("cem-native-value-parent", "count=\"2\""),
    ] {
        let (artifact, data) = hooks(tag, 2);
        let baseline = render_compiled_template(&artifact, &data);
        let (plan, stages) = measure(|| render_compiled_template(&artifact, &data));
        copy_profile_tests::verify_plan(&plan, &baseline);
        assert!(
            render_plan_to_html(&plan).contains(expected),
            "{}",
            render_plan_to_html(&plan)
        );
        assert!(stages.contains_key("copy/hook-capture"));
        assert!(stages.contains_key("copy/hook-caller"));
        assert!(!stages.contains_key("copy/try-snapshot"));
    }
    let (artifact, data) = input_profile_tests::tree_fixture(2);
    let (expected, baseline) = measure(|| render_compiled_template(&artifact, &data));
    assert_eq!(baseline["inspect/schema-registry"].calls, 1);
    assert_eq!(baseline["inspect/conversion-registry"].calls, 1);
    let prepared = Prepared::default();
    for cold in [true, false] {
        let (plan, stages) =
            measure(|| with_prepared(&prepared, || render_compiled_template(&artifact, &data)));
        copy_profile_tests::verify_plan(&plan, &expected);
        assert_eq!(stages.contains_key("inspect/prepared-registry-build"), cold);
        assert_eq!(stages["inspect/prepared-writer"].calls, 1);
        assert!(!stages.contains_key("inspect/schema-registry"));
        assert!(!stages.contains_key("copy/try-snapshot"));
        assert!(!stages.contains_key("copy/hook-capture"));
    }
}

fn recovery(source: &str, controls: usize) -> (TemplateArtifact, TemplateData) {
    let page = include_str!("../../../cem-elements/demo/xpath-functions.html");
    let expression = page
        .split_once("<p>Use <code>")
        .unwrap()
        .1
        .split_once("</code>")
        .unwrap()
        .0;
    assert!(expression.starts_with("try { data:parse(source, \"xml\") } catch"));
    let data = TemplateData::default()
        .with_binding(
            "island",
            ItemStream::once(copy_profile_tests::controls(controls)),
        )
        .with_binding(
            "source",
            ItemStream::once(Item::Atomic(AtomValue::String(source.into()))),
        );
    let artifact = compile_template(
        &format!("{{pre | {{${expression}}}}}"),
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    (artifact, data)
}

#[test]
fn documented_import_recovery_does_not_use_renderer_snapshots() {
    for source in ["<r>ok</r>", "<r>"] {
        let (artifact, data) = recovery(source, 2);
        let (plan, stages) = measure(|| render_compiled_template(&artifact, &data));
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        let html = render_plan_to_html(&plan);
        if source.ends_with("</r>") {
            assert_eq!(html, "<pre><r>ok</r></pre>");
        } else {
            assert!(!html.contains("<r>"));
            assert!(html.len() > "<pre></pre>".len());
        }
        assert!(!stages.contains_key("copy/try-snapshot"));
        assert!(!stages.contains_key("copy/hook-capture"));
    }
}

fn template_recovery(outcome: &str, controls: usize) -> (TemplateArtifact, TemplateData) {
    let readme = include_str!("../../README.md");
    let body = readme
        .split_once("CEMT provides scoped output recovery")
        .unwrap()
        .1
        .split_once("```cem-ml\n")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0;
    // Supply the example's named loader without changing its authored recovery.
    let source = format!("{{template @name=load-content | {outcome}}}{body}");
    let data = TemplateData::default().with_binding(
        "island",
        ItemStream::once(copy_profile_tests::controls(controls)),
    );
    let artifact = compile_template(
        &source,
        &CompileTemplateOptions {
            host_bindings: vec!["island".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    (artifact, data)
}

const RECOVERY_CASES: [(&str, &str, &str); 3] = [
    ("success", "{p | loaded}", "<p>loaded</p>"),
    (
        "first-catch",
        r#"{b | partial}{$report:raise("sample.invalid", "bad input")}"#,
        "<p role=\"alert\">bad input</p>",
    ),
    (
        "last-catch",
        r#"{b | partial}{$report:raise("sample.other", "bad input")}"#,
        "<p role=\"alert\">sample.other</p>",
    ),
];

#[test]
fn documented_template_recovery_profiles_success_and_both_handlers() {
    for (_, outcome, expected) in RECOVERY_CASES {
        let (artifact, data) = template_recovery(outcome, 2);
        let (plan, stages) = measure(|| render_compiled_template(&artifact, &data));
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(render_plan_to_html(&plan).trim(), expected);
        assert_eq!(stages["copy/try-snapshot"].calls, 1);
        assert!(!stages.contains_key("copy/hook-capture"));
    }
}

#[test]
#[ignore = "profiling fixture: --release --lib profile_authored_bodies -- --ignored --nocapture --test-threads=1"]
fn profile_authored_bodies() {
    for controls in [0, 256] {
        let (artifact, data) = input_profile_tests::tree_fixture(controls);
        let expected = render_compiled_template(&artifact, &data);
        let render = || render_compiled_template(&artifact, &data);
        profile(&format!("tree/{controls}/fresh"), render, |plan| {
            copy_profile_tests::verify_plan(plan, &expected)
        });
        let prepared = Prepared::default();
        profile(
            &format!("tree/{controls}/prepared"),
            || with_prepared(&prepared, render),
            |plan| copy_profile_tests::verify_plan(plan, &expected),
        );
        for tag in ["cem-native-value-card", "cem-native-value-parent"] {
            let (artifact, data) = hooks(tag, controls);
            let expected = render_compiled_template(&artifact, &data);
            profile(
                &format!("hooks/{tag}/{controls}"),
                || render_compiled_template(&artifact, &data),
                |plan| copy_profile_tests::verify_plan(plan, &expected),
            );
        }
        for (name, outcome, _) in RECOVERY_CASES {
            let (artifact, data) = template_recovery(outcome, controls);
            let expected = render_compiled_template(&artifact, &data);
            profile(
                &format!("template-recovery/{name}/{controls}"),
                || render_compiled_template(&artifact, &data),
                |plan| copy_profile_tests::verify_plan(plan, &expected),
            );
        }
        for (name, source) in [("valid", "<r>ok</r>"), ("invalid", "<r>")] {
            let (artifact, data) = recovery(source, controls);
            let expected = render_compiled_template(&artifact, &data);
            profile(
                &format!("query-recovery/{name}/{controls}"),
                || render_compiled_template(&artifact, &data),
                |plan| copy_profile_tests::verify_plan(plan, &expected),
            );
        }
    }
    // First use includes registry construction every time: no amortized claim.
    let (artifact, data) = input_profile_tests::tree_fixture(256);
    let expected = render_compiled_template(&artifact, &data);
    profile(
        "tree/256/cold-prepared",
        || {
            with_prepared(&Prepared::default(), || {
                render_compiled_template(&artifact, &data)
            })
        },
        |plan| copy_profile_tests::verify_plan(plan, &expected),
    );
    // Direct query measurements separate registry and writer work from CEMT.
    let owner = cem_ml::import::import_data(
        include_str!("../../../cem-elements/demo/tree-source.xml"),
        "xml",
        "cem",
        "memory:direct",
    )
    .unwrap();
    let context = EvaluationContext {
        policy_bindings: BTreeMap::from([(
            "doc".into(),
            ItemStream::once(crate::eval::imported_cem_tree(owner)),
        )]),
        ..Default::default()
    };
    let query = compile(
        "cemml:inspect(doc)",
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let expected = evaluate(&query, &context);
    let prepared = Prepared::default();
    for cached in [false, true] {
        profile(
            &format!("inspect/prepared={cached}"),
            || {
                if cached {
                    with_prepared(&prepared, || evaluate(&query, &context))
                } else {
                    evaluate(&query, &context)
                }
            },
            |result| {
                assert_eq!(result, &expected);
                assert_eq!(result.diagnostics, expected.diagnostics);
            },
        );
    }
}
