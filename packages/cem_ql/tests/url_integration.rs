//! Shared test-control matrix; JSON here is an explicit expected-result boundary.
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
    TemplateData,
};
use serde_json::{json, Value};

fn control_item(item: &Item) -> Value {
    match item {
        Item::Atomic(atom) => match atom {
            AtomValue::String(value) => json!({"kind":"atomic","type":"string","value":value}),
            AtomValue::AnyUri(value) => json!({"kind":"atomic","type":"any-uri","value":value}),
            AtomValue::Integer(value) => json!({"kind":"atomic","type":"integer","value":value}),
            AtomValue::Boolean(value) => json!({"kind":"atomic","type":"boolean","value":value}),
            _ => panic!("unexpected URL scalar: {atom:?}"),
        },
        Item::Record(fields) => json!({"kind":"record","fields":fields.iter()
            .map(|(key, items)| (key.clone(), items.iter().map(control_item).collect::<Vec<_>>()))
            .collect::<std::collections::BTreeMap<_,_>>()}),
        _ => panic!("unexpected URL item: {item:?}"),
    }
}

#[test]
fn native_url_matrix_matches_typed_results_and_ordered_diagnostics() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/url/query-matrix.json")).unwrap();
    for case in cases {
        let source = case["query"].as_str().unwrap();
        let query = compile(source, &CompileContext::default()).unwrap();
        let result = evaluate(&query, &EvaluationContext::default());
        assert_eq!(
            json!(result.items.iter().map(control_item).collect::<Vec<_>>()),
            case["items"],
            "{}",
            case["id"]
        );
        assert_eq!(
            json!(result
                .diagnostics
                .iter()
                .map(|d| &d.code)
                .collect::<Vec<_>>()),
            case["diagnosticCodes"],
            "{}",
            case["id"]
        );
        assert_eq!(
            result.error.is_some(),
            case["error"].as_bool().unwrap(),
            "{}",
            case["id"]
        );
        for diagnostic in result.diagnostics {
            assert!(diagnostic.byte_offset.is_some());
            assert!(diagnostic.source_map.unwrap().current().is_some());
        }
    }
}

#[test]
fn native_server_render_reuses_compiled_url_template_with_changed_bindings() {
    let artifact = compile_template(
        include_str!("../fixtures/url/render.cemt"),
        &CompileTemplateOptions {
            host_bindings: vec!["path".into(), "base".into(), "search".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for (path, search, expected) in [
        (
            "../b",
            "b=2&a=1&a=3",
            "<a href=\"https://h/b?b=2&amp;a=1&amp;a=3\">a=1&amp;a=3&amp;b=2</a>",
        ),
        (
            "next",
            "x=%3Ctag%3E&x=a+b",
            "<a href=\"https://h/a/next?x=%3Ctag%3E&amp;x=a+b\">x=%3Ctag%3E&amp;x=a+b</a>",
        ),
    ] {
        let mut data = TemplateData::default();
        for (key, value) in [
            ("path", path),
            ("base", "https://h/a/c"),
            ("search", search),
        ] {
            data = data.with_binding(
                key,
                ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
            );
        }
        let plan = render_compiled_template(&artifact, &data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(render_plan_to_html(&plan), expected);
    }
    let data = TemplateData::default()
        .with_binding(
            "path",
            ItemStream::once(Item::Atomic(AtomValue::String("bad".into()))),
        )
        .with_binding(
            "base",
            ItemStream::once(Item::Atomic(AtomValue::String("bad".into()))),
        )
        .with_binding(
            "search",
            ItemStream::once(Item::Atomic(AtomValue::String(String::new()))),
        );
    let plan = render_compiled_template(&artifact, &data);
    assert!(
        plan.diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.url_base_invalid"),
        "{:?}",
        plan.diagnostics
    );
}
