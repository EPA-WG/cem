//! Authored storage templates: scalar startup versus imported native documents.
use cem_ml::value::artifact::CemValueArtifactLimits;
use cem_ql::{
    api::native_values,
    eval::{portable::decode_values, AtomValue, Item, ItemStream},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, render_template,
        CompileTemplateOptions, RenderPlan, RenderPlanNode, TemplateData,
    },
};

const PAGE: &str = include_str!("../../cem-elements/demo/local-storage.html");
const FRUIT: &str = "6. Fruit buttons and a storage watcher";
const JSON: &str = "3e. JSON validation";
const FRUITS: [&str; 4] = ["lemons", "cherries", "apples", "bananas"];

fn template(legend: &str) -> &str {
    // Select authored CEMT source, never parse a stored source document here.
    PAGE.split_once(&format!("legend=\"{legend}\""))
        .unwrap()
        .1
        .split_once("<template type=\"text/cem-ml\">")
        .unwrap()
        .1
        .split_once("</template>")
        .unwrap()
        .0
}

fn scalar(value: AtomValue) -> Item {
    Item::Atomic(value)
}

fn data(fields: impl IntoIterator<Item = (&'static str, AtomValue)>) -> TemplateData {
    // Only the host's control envelope and scalar slices are records.
    let slices = Item::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), vec![scalar(value)]))
            .collect(),
    );
    TemplateData::default().with_binding(
        "datadom",
        ItemStream::once(Item::Record([("slices".into(), vec![slices])].into())),
    )
}

fn counts(values: &[i64]) -> TemplateData {
    data(
        FRUITS
            .into_iter()
            .zip(values.iter().copied().map(AtomValue::Integer)),
    )
}

fn total(html: &str) -> &str {
    html.rsplit_once("<dd>")
        .unwrap()
        .1
        .split_once("</dd>")
        .unwrap()
        .0
        .trim()
}

#[test]
fn fruit_watcher_is_quiet_until_all_scalar_slices_are_bound() {
    for values in [&[][..], &[1][..], &[1, 12][..], &[1, 12, 0][..]] {
        let result = render_template(template(FRUIT), &counts(values));
        assert!(
            result.diagnostics.is_empty(),
            "{values:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(total(&result.rendered), "", "no partial or invented total");
    }
}

#[test]
fn fruit_watcher_preserves_complete_arithmetic_including_zero() {
    for (values, expected) in [([1, 12, 0, 0], "13"), ([2, 13, 1, 1], "17"), ([0; 4], "0")] {
        let result = render_template(template(FRUIT), &counts(&values));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(total(&result.rendered), expected);
    }
}

#[test]
fn invalid_bound_counts_and_unguarded_missing_arithmetic_remain_errors() {
    for invalid in [AtomValue::Null, AtomValue::String("ABC".into())] {
        let result = render_template(
            template(FRUIT),
            &data([
                ("lemons", invalid),
                ("cherries", AtomValue::Integer(12)),
                ("apples", AtomValue::Integer(0)),
                ("bananas", AtomValue::Integer(0)),
            ]),
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.ql.type_error"),
            "{:?}",
            result.diagnostics
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.render.eval_failed"));
        assert_eq!(total(&result.rendered), "");
    }
    let result = render_template(
        "{output | {$datadom.slices.lemons + datadom.slices.cherries}}",
        &counts(&[]),
    );
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.type_error"));
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.render.eval_failed"));
}

fn json_data(source: Option<&str>) -> TemplateData {
    let mut data = data([
        (
            "raw",
            source
                .map(|s| AtomValue::String(s.into()))
                .unwrap_or(AtomValue::Null),
        ),
        ("json", AtomValue::Null),
    ]);
    if let Some(source) = source {
        let limits = CemValueArtifactLimits::default();
        let bytes = native_values::import_document(
            source.as_bytes(),
            "application/json",
            "storage:diagnostics",
            &limits,
        )
        .unwrap();
        assert_eq!(
            native_values::export_json(&bytes, 0, None, &limits)
                .unwrap()
                .as_deref(),
            Some(source)
        );
        data.bind_native_slice("json", decode_values(&bytes, &limits).unwrap())
            .unwrap();
        // Drop the portable bytes before rendering: native values own their tree.
    }
    data
}

fn element_texts<'a>(html: &'a str, tag: &str) -> Vec<&'a str> {
    html.split(&format!("<{tag}>"))
        .skip(1)
        .map(|part| part.split_once(&format!("</{tag}>")).unwrap().0.trim())
        .collect()
}

fn roots(plan: &RenderPlan) -> Vec<(&str, &cem_ml::source_map::SourceMapStack)> {
    plan.nodes
        .iter()
        .filter_map(|node| match node {
            RenderPlanNode::Element {
                tag, source_map, ..
            } if tag != "local-storage" => Some((tag.as_str(), source_map)),
            _ => None,
        })
        .collect()
}

#[test]
fn json_shape_changes_add_roots_without_changing_the_existing_roots_provenance() {
    let compiled = compile_template(template(JSON), &CompileTemplateOptions::default());
    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    let pending = render_compiled_template(&compiled, &json_data(None));
    assert!(pending.diagnostics.is_empty(), "{:?}", pending.diagnostics);
    assert_eq!(
        element_texts(&render_plan_to_html(&pending), "output"),
        ["null", "null"]
    );
    let original = roots(&pending);
    assert_eq!(original.iter().map(|r| r.0).collect::<Vec<_>>(), ["p", "p"]);
    assert!(original.iter().all(|r| !r.1.frames.is_empty()));
    for (source, extra_root, expected) in [
        (r#"{"a":1,"b":"B"}"#, Some("ul"), "a: 1|b: B"),
        ("[1,2,3]", Some("ol"), "1|2|3"),
        (r#""ABC""#, None, "ABC"),
        ("12.345", None, "12.345"),
        ("false", None, "false"),
        ("0", None, "0"),
        ("null", None, "null"),
        (
            r#"{"fruit":"cherry","count":0}"#,
            Some("ul"),
            "fruit: cherry|count: 0",
        ),
    ] {
        let plan = render_compiled_template(&compiled, &json_data(Some(source)));
        assert!(
            plan.diagnostics.is_empty(),
            "{source}: {:?}",
            plan.diagnostics
        );
        let actual = roots(&plan);
        assert_eq!(
            &actual[..2],
            original.as_slice(),
            "stable native roots for {source}"
        );
        assert_eq!(actual.get(2).map(|r| r.0), extra_root);
        let html = render_plan_to_html(&plan);
        if extra_root.is_some() {
            assert_eq!(element_texts(&html, "li").join("|"), expected, "{source}");
        } else {
            assert_eq!(
                element_texts(&html, "output").last().copied(),
                Some(expected),
                "{source}"
            );
        }
    }
}

#[test]
fn invalid_json_is_rejected_by_import_and_the_unbound_view_is_quiet() {
    assert!(native_values::import_document(
        b"ABC",
        "application/json",
        "storage:diagnostics",
        &CemValueArtifactLimits::default()
    )
    .is_err());
    let result = render_template(
        template(JSON),
        &data([
            ("raw", AtomValue::String("ABC".into())),
            ("json", AtomValue::Null),
        ]),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(element_texts(&result.rendered, "output"), ["ABC", "null"]);
    assert!(!result.rendered.contains("<ul>"));
    assert!(!result.rendered.contains("<ol>"));
}
