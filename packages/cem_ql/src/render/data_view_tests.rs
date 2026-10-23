//! Production declarative data viewer; formats resolve at the native import boundary.
use super::*;

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn render(value: &str, format: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cem-components/src/components/cem-data-view/cem-data-view.xhtml");
    let declaration = std::fs::read_to_string(path).unwrap_or_default();
    let source = declaration
        .split_once("<template id=\"cem-data-view\" type=\"text/cem-ml\">")
        .expect("production data-view template")
        .1
        .split_once("</template>")
        .unwrap()
        .0;
    let artifact = compile_template(source, &CompileTemplateOptions::default());
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let attrs = Item::Record(std::collections::BTreeMap::from([
        ("value".into(), vec![text(value)]),
        ("format".into(), vec![text(format)]),
        ("label".into(), vec![text("Preview data")]),
    ]));
    let data = TemplateData::default()
        .with_binding("value", ItemStream::once(text(value)))
        .with_binding("format", ItemStream::once(text(format)))
        .with_binding("label", ItemStream::once(text("Preview data")))
        .with_binding(
            "datadom",
            ItemStream::once(Item::Record(std::collections::BTreeMap::from([(
                "attributes".into(),
                vec![attrs],
            )]))),
        );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    render_plan_to_html(&plan)
}

#[test]
fn shared_data_view_json_preserves_missing_null_empty_and_heterogeneous_rows() {
    let html = render(
        r#"[{"name":"","value":null},{"name":"Ada"},3,false,[],{}]"#,
        "json",
    );
    assert!(html.contains("<table"), "{html}");
    assert!(html.contains("scope=\"col\""));
    assert!(html.contains("null"), "{html}");
    assert!(
        html.contains("&quot;&quot;") || html.contains("\"\""),
        "{html}"
    );
    assert!(html.contains("∅ missing"), "{html}");
    assert!(html.contains("Ada") && html.contains("false"), "{html}");
    assert!(html.contains("[]") && html.contains("{}"), "{html}");
}

#[test]
fn shared_data_view_csv_uses_native_quoted_fields_and_literal_text() {
    let html = render(
        "name,note\nAda,\"first, second\"\nBob,\"line one\nline two\"",
        "csv",
    );
    assert!(html.contains("<table"), "{html}");
    assert!(html.contains("first, second"), "{html}");
    assert!(html.contains("line one\nline two"), "{html}");
    let safe = render(r#"[{"x":"<img src=x onerror=alert(1)>"}]"#, "json");
    assert!(!safe.contains("<img"), "{safe}");
    assert!(safe.contains("&lt;img"), "{safe}");
}

#[test]
fn shared_data_view_limits_rows_columns_cells_depth_and_source_before_import() {
    let rows = format!(
        "[{}]",
        (0..103)
            .map(|n| format!("{{\"n\":{n}}}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let html = render(&rows, "json");
    assert_eq!(html.matches("<tr").count(), 101, "{html}");
    assert!(html.contains("Row limit: 100"), "{html}");
    let columns = format!(
        "[{{{}}}]",
        (0..35)
            .map(|n| format!("\"c{n}\":{n}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let html = render(&columns, "json");
    assert_eq!(html.matches("scope=\"col\"").count(), 32, "{html}");
    assert!(html.contains("Column limit: 32"), "{html}");
    let html = render(&format!("[{{\"x\":\"{}\"}}]", "é".repeat(2050)), "json");
    assert!(html.contains("cell limit: 2048"), "{html}");
    assert!(!html.contains(&"é".repeat(2049)));
    let html = render(
        r#"{"a":{"b":{"c":{"d":{"hidden":"secret tail"}}}}}"#,
        "json",
    );
    assert!(html.contains("Depth limit: 4"), "{html}");
    assert!(!html.contains("secret tail"), "{html}");
    let html = render(&format!("\"{}\"", "🟢".repeat(65535)), "json");
    assert!(html.contains("Source limit: 65536"), "{html}");
    assert!(!html.contains("🟢"), "{html}");
}

#[test]
fn shared_data_view_rejects_malformed_or_unsupported_data_and_handles_empty_roots() {
    for (source, format) in [("[", "json"), ("<root/>", "xml")] {
        let html = render(source, format);
        assert!(html.contains("role=\"alert\""), "{html}");
        assert!(!html.contains("<table"), "{html}");
    }
    for (source, expected) in [
        ("[]", "[]"),
        ("{}", "{}"),
        ("null", "null"),
        ("\"\"", "\"\""),
    ] {
        let html = render(source, "json");
        assert!(
            html.contains(expected) || html.contains("&quot;&quot;"),
            "{html}"
        );
    }
}
