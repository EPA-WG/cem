//! XML-VIEW-3: authored UI over retained CEM nodes, never parser records.
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
    render::{render_template, TemplateData},
};

const VIEW: &str = include_str!("../../cem-elements/demo/data-tree-view.cemt");

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: Vec<(&str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), vec![value]))
            .collect(),
    )
}
fn input(source: &str, format: &str, revision: i64, selected: bool) -> TemplateData {
    let mut slices = vec![];
    if selected {
        slices.extend([
            ("branch.1.1", text("edit-0")),
            ("branch.1.2", text("edit-0")),
        ]);
    }
    TemplateData::default()
        .with_binding("format", ItemStream::once(text(format)))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    record(vec![("nodes", record(vec![("text", text(source))]))]),
                ),
                ("slices", record(slices)),
                (
                    "eventPayloads",
                    record(vec![(
                        "source",
                        record(vec![(
                            "revision",
                            Item::Atomic(AtomValue::Integer(revision)),
                        )]),
                    )]),
                ),
            ])),
        )
}
fn render(source: &str, format: &str, revision: i64, selected: bool) -> String {
    let result = render_template(VIEW, &input(source, format, revision, selected));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.rendered
}

#[test]
fn authored_tree_uses_shared_inspection_for_all_import_formats() {
    for (format, source) in [
        ("xml", "<r><fruit>🍒</fruit><empty/></r>"),
        ("json", r#"{"fruit":"🍒","empty":""}"#),
        ("yaml", "fruit: 🍒\nempty: ''"),
        ("csv", "fruit,empty\n🍒,\n"),
    ] {
        let html = render(source, format, 0, false);
        assert!(html.contains("CEM-ML document"), "{format}: {html}");
        assert!(
            html.contains("{ast") && html.contains("🍒"),
            "{format}: {html}"
        );
        assert!(
            html.contains("Branches selected: <output aria-label=\"Selected branches\">0</output>"),
            "{html}"
        );
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("<summary>"));
        assert!(!html.contains('\u{1b}'));
    }
}

#[test]
fn source_details_are_inert_and_keep_namespaces_empty_values_and_mixed_content() {
    let html = render("<?xml-stylesheet href='never-fetch.xsl'?><r xmlns:p='urn:fruit'><p:fruit empty=''>pre<![CDATA[<raw>🍒]]><?keep inert?>post</p:fruit><script>neverRun()</script></r>", "xml", 0, false);
    assert!(!html.contains("<script>") && !html.contains("<raw>"));
    assert!(html.contains("&lt;raw&gt;🍒"));
    assert!(html.contains("@kind=cdata") && html.contains("@target=xml-stylesheet"));
    let branches = html.split("<h3>Branches</h3>").nth(1).unwrap();
    assert!(branches.contains("urn:fruit") && branches.contains("@empty"));
    assert!(
        branches.contains("\"\"") && branches.contains("keep"),
        "{branches}"
    );
    assert!(!branches.contains("http://www.w3.org/2000/xmlns/"));
    assert!(!branches.contains("@p"));
}

#[test]
fn multiple_branch_selections_expire_on_every_source_revision() {
    let source = "<r><first>🍒</first><second>🍋</second></r>";
    let selected = render(source, "xml", 0, true);
    assert!(
        selected.contains("Branches selected: <output aria-label=\"Selected branches\">2</output>"),
        "{selected}"
    );
    assert_eq!(
        selected.matches(" checked").count(),
        2,
        "{}",
        selected.split("<h3>Branches</h3>").nth(1).unwrap()
    );
    for revision in [1, 2] {
        let reset = render(source, "xml", revision, true);
        assert!(
            reset
                .contains("Branches selected: <output aria-label=\"Selected branches\">0</output>"),
            "{reset}"
        );
        assert!(!reset.contains(" checked"));
    }
}

#[test]
fn malformed_source_has_no_stale_inspection_and_can_be_repaired() {
    let html = render("<r><fruit></r>", "xml", 1, true);
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(!html.contains("<pre") && !html.contains("type=\"checkbox\""));
    let repaired = render("<r><fruit>🍒</fruit></r>", "xml", 2, true);
    assert!(!repaired.contains("role=\"alert\""));
    assert!(
        repaired.contains("Branches selected: <output aria-label=\"Selected branches\">0</output>")
    );
}

#[test]
fn request_view_consumes_retained_documents_and_lifecycle_without_reimport() {
    use cem_ml::import::import_data;
    use cem_ql::{
        eval::imported_cem_tree,
        render::{
            compile_template_module_closure, render_compiled_template, render_plan_to_html,
            CompileTemplateOptions, TemplateModuleClosure, TemplateModuleSource,
        },
    };
    const REQUEST: &str = include_str!("../../cem-elements/demo/data-tree-request.cemt");
    let hash =
        |s: &str| cem_ml::content_cache::ContentHash::from_blake3(s.as_bytes()).header_value();
    let artifact = compile_template_module_closure(
        REQUEST,
        &TemplateModuleClosure {
            root_uri: "https://example.test/demo/data-tree-request.cemt".into(),
            root_content_hash: hash(REQUEST),
            modules: vec![TemplateModuleSource {
                alias: "view".into(),
                parent_uri: None,
                uri: "https://example.test/demo/data-tree-view.cemt".into(),
                content_hash: hash(VIEW),
                source: VIEW.into(),
            }],
            ..Default::default()
        },
        &CompileTemplateOptions::default(),
    );
    for (format, source) in [
        (
            "xml",
            include_str!("../../cem-elements/demo/tree-source.xml"),
        ),
        (
            "json",
            include_str!("../../cem-elements/demo/tree-source.json"),
        ),
    ] {
        let owner = import_data(source, format, "cem", "memory:local-source").unwrap();
        for state in ["loaded", "failed", "loading"] {
            let data = TemplateData::default()
                .with_binding(
                    "sourceUrl",
                    ItemStream::once(text(&format!("./tree-source.{format}"))),
                )
                .with_binding(
                    "datadom",
                    ItemStream::once(record(vec![(
                        "slices",
                        record(vec![
                            ("sourceUrl", text(&format!("./tree-source.{format}"))),
                            (
                                "resource",
                                record(vec![
                                    ("state", text(state)),
                                    ("resourceRevision", Item::Atomic(AtomValue::Integer(1))),
                                    ("data", imported_cem_tree(owner.clone())),
                                ]),
                            ),
                        ]),
                    )])),
                );
            let rendered = render_compiled_template(&artifact, &data);
            assert!(
                rendered.diagnostics.is_empty(),
                "{:?}",
                rendered.diagnostics
            );
            let html = render_plan_to_html(&rendered);
            assert!(
                html.contains(&format!(
                    "<option value=\"./tree-source.{format}\" selected>"
                )),
                "{html}"
            );
            assert_eq!(html.matches(" selected>").count(), 1, "{html}");
            if state == "loaded" {
                assert!(html.contains("value=\"request-1\""), "{html}");
            }
            assert!(
                html.contains(&format!(
                    "Request state: <output aria-label=\"Request state\">{state}</output>"
                )),
                "{html}"
            );
            assert_eq!(html.contains("{ast"), state == "loaded", "{html}");
            assert_eq!(html.contains("role=\"alert\""), state == "failed", "{html}");
        }
    }
}
