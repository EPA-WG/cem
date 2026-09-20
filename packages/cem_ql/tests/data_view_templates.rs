//! Presentation assertions for authored CEMT, not a Rust table implementation.
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};

const VIEW: &str = include_str!("../../cem-elements/demo/data-table-view.cemt");

#[test]
fn authored_view_keeps_processing_instruction_target_and_data() {
    let result = render_template(VIEW, &data("<r><!--note--><?keep inert?></r>", "xml", ""));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let output = result.rendered.split("</textarea>").nth(1).unwrap();
    assert!(
        output.contains("<span class=\"value\">keep inert</span>"),
        "{output}"
    );
}

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: Vec<(&str, Vec<Item>)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
fn data(source: &str, format: &str, column: &str) -> TemplateData {
    TemplateData::default()
        .with_binding("format", ItemStream::once(string(format)))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    vec![record(vec![(
                        "nodes",
                        vec![record(vec![("text", vec![string(source)])])],
                    )])],
                ),
                (
                    "slices",
                    vec![record(vec![
                        ("column", vec![string(column)]),
                        ("mode", vec![string("number")]),
                    ])],
                ),
            ])),
        )
}

#[test]
fn authored_view_transforms_all_four_formats_and_sorts_original_nodes() {
    for (format, source, column) in [
        (
            "xml",
            "<r><row qty=\"10\">🍒</row><row qty=\"2\">🍋</row><row qty=\"3\">🍌</row></r>",
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
        let result = render_template(VIEW, &data(source, format, column));
        assert!(
            result.diagnostics.is_empty(),
            "{format}: {:?}",
            result.diagnostics
        );
        let table = result
            .rendered
            .split("<tbody>")
            .nth(1)
            .unwrap_or_else(|| panic!("CEMT generated no table: {}", result.rendered));
        assert!(
            table.find('🍋') < table.find('🍌') && table.find('🍌') < table.find('🍒'),
            "{format}: {table}"
        );
        assert!(table.contains("Select source line"));
        assert!(!result.rendered.contains("<cem-data"));
    }
}

#[test]
fn authored_view_preserves_empty_missing_null_and_nested_collections() {
    let result = render_template(
        VIEW,
        &data(
            r#"[{"note":"","tags":["red","sweet"]},{"qty":2},{"note":null}]"#,
            "json",
            "",
        ),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.rendered.matches("<table ").count(), 2);
    assert!(result.rendered.contains("∅"));
    assert!(result.rendered.contains("&quot;&quot;"));
    assert!(result.rendered.contains("null"));
    assert!(result.rendered.contains("sweet"));
    let invalid = render_template(VIEW, &data("[oops]", "json", ""));
    assert!(invalid.diagnostics.is_empty(), "{:?}", invalid.diagnostics);
    assert!(invalid.rendered.contains("role=\"alert\""));
    assert!(!invalid.rendered.contains("<table "));
}

#[test]
fn authored_xml_grouping_uses_expanded_names_and_unions_later_columns() {
    let result = render_template(
        VIEW,
        &data(
            r#"<r xmlns:a="urn:one" xmlns:b="urn:one" xmlns:c="urn:two"><a:row id="1">first</a:row><b:row id="2" later="">second</b:row><c:row id="3">separate</c:row></r>"#,
            "xml",
            "",
        ),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.rendered.matches("<table ").count(), 1);
    assert!(result.rendered.contains("@later</th>"));
    assert!(result.rendered.contains("∅"));
    assert!(result.rendered.contains("&quot;&quot;"));
    assert!(result.rendered.contains("separate"));
}

#[test]
fn xml_view_groups_within_parents_and_keeps_first_seen_columns_and_mixed_content() {
    let source = r#"<r xmlns:a="urn:one" xmlns:b="urn:one" xmlns:c="urn:two"><left><a:row same="" id="one">pre<![CDATA[🍒]]><same>child</same>post<nest><v>red</v><v>green</v></nest></a:row><b:row id="two" later=""><c:same>other</c:same></b:row><c:row>different</c:row><c:row>namespace</c:row></left><right><a:row>ivysaur</a:row><b:row>venusaur</b:row></right></r>"#;
    let result = render_template(VIEW, &data(source, "xml", ""));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    // Inspect the explicit HTML output, never turn it into a runtime data tree.
    let output = result.rendered.split("</textarea>").nth(1).unwrap();
    let headings: Vec<Vec<_>> = output
        .split("<thead>")
        .skip(1)
        .map(|table| {
            table
                .split("</thead>")
                .next()
                .unwrap()
                .split("<th scope=\"col\">")
                .skip(1)
                .map(|cell| cell.split("</th>").next().unwrap())
                .collect()
        })
        .collect();
    assert_eq!(
        headings,
        [
            vec![
                "✓",
                "@same",
                "@id",
                "#text",
                "same",
                "nest",
                "@later",
                "urn:two|same"
            ],
            vec!["✓", "#text"], // nested v siblings
            vec!["✓", "#text"], // same local row name, different namespace
            vec!["✓", "#text"], // same expanded row name, different parent
        ]
    );
    assert!(output.contains("pre🍒post"), "{output}");
    assert!(
        output.contains("&quot;&quot;"),
        "empty attribute stays present"
    );
    assert!(output.contains("∅"), "missing columns stay absent");
    for value in [
        "child",
        "other",
        "red",
        "green",
        "different",
        "namespace",
        "ivysaur",
        "venusaur",
    ] {
        assert!(
            output.contains(&format!("<span class=\"value\">{value}</span>")),
            "{value}: {output}"
        );
    }
}

#[test]
fn viewer_excludes_namespace_declarations_but_keeps_real_namespaced_attributes() {
    let source = r#"<r xmlns="urn:root" xmlns:p="urn:rows" code="root"><p:row xmlns:q="urn:field" id="1" q:xmlns="first">A</p:row><p:row xmlns:q="urn:field" id="2" q:xmlns="second">B</p:row><single xmlns:s="urn:detail" s:xmlns="detail"/></r>"#;
    let result = render_template(VIEW, &data(source, "xml", ""));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let rendered = result.rendered.split("</textarea>").nth(1).unwrap();
    assert!(
        !rendered.contains("http://www.w3.org/2000/xmlns/"),
        "{rendered}"
    );
    assert!(!rendered.contains("@p:"), "{rendered}");
    assert!(!rendered.contains("@q:"), "{rendered}");
    assert!(!rendered.contains("@s:"), "{rendered}");
    assert!(!rendered.contains("@xmlns: urn:root"), "{rendered}");
    assert!(rendered.contains("@urn:field|xmlns</th>"), "{rendered}");
    assert!(rendered.contains("@xmlns: detail"), "{rendered}");
    assert!(rendered.contains("@code: root"), "{rendered}");
    assert_eq!(rendered.matches("<table ").count(), 1);
    assert!(rendered.contains("first") && rendered.contains("second"));
}

#[test]
fn imported_aspects_replace_only_selected_presentations_and_can_be_disabled() {
    use cem_ql::render::{
        compile_template_module_closure, render_compiled_template, render_plan_to_html,
        CompileTemplateOptions, TemplateModuleClosure, TemplateModuleSource,
    };
    let extension = include_str!("../../cem-elements/demo/data-table-aspects.cemt");
    let hash =
        |s: &str| cem_ml::content_cache::ContentHash::from_blake3(s.as_bytes()).header_value();
    let artifact = compile_template_module_closure(
        extension,
        &TemplateModuleClosure {
            root_uri: "https://example.test/data-table-aspects.cemt".into(),
            root_content_hash: hash(extension),
            modules: vec![TemplateModuleSource {
                alias: "base".into(),
                parent_uri: None,
                uri: "https://example.test/data-table-view.cemt".into(),
                content_hash: hash(VIEW),
                source: VIEW.into(),
            }],
            ..TemplateModuleClosure::default()
        },
        &CompileTemplateOptions::default(),
    );
    let source = r#"{"visits":[{"address":"192.0.2.1","hits":10},{"address":"192.0.2.2","hits":2}],"notes":[{"message":"🍒"},{"message":"🍋"}],"filter":{"kind":"ip-filter","address":"192.0.2.0/24","action":"allow"}}"#;
    for enabled in [true, false] {
        let mut input = data(source, "json", "");
        input = input.with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    vec![record(vec![(
                        "nodes",
                        vec![record(vec![("text", vec![string(source)])])],
                    )])],
                ),
                (
                    "slices",
                    vec![record(vec![(
                        "aspects",
                        vec![Item::Atomic(AtomValue::Boolean(enabled))],
                    )])],
                ),
            ])),
        );
        let plan = render_compiled_template(&artifact, &input);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        let html = render_plan_to_html(&plan);
        assert!(html.contains("<table aria-label=\"visits\""), "{html}");
        assert_eq!(html.contains("<table aria-label=\"notes\""), !enabled);
        assert_eq!(html.contains("aria-label=\"IP filter\""), enabled, "{html}");
        if enabled {
            assert!(html.contains("🌳 notes · array"));
            assert!(html.contains("value=\"192.0.2.0/24\""));
            assert!(html.contains("allow: 192.0.2.0/24"));
        }
    }
}
