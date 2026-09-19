//! XSLT-VIEW-PARITY: the authored viewer runs over shared imported CEM trees.
use cem_ml::{import::import_data, validation::xpath::XPathExpandedName};
use cem_ql::{
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        RenderPlan, RenderPlanNode, TemplateData,
    },
    xslt::{
        compiler::{compile_xslt_bundle_with_options, XsltCompileOptions},
        XsltBundle,
    },
};

const VIEW: &str = include_str!("../../cem-elements/demo/data-table-view.xslt");

fn compiled() -> cem_ql::xslt::compiler::CompiledXsltBundle {
    let options = XsltCompileOptions {
        entrypoint: Some(XPathExpandedName::unqualified("viewer")),
        parameters: [
            "source",
            "initial",
            "format",
            "column",
            "direction",
            "mode",
            "selected",
        ]
        .into_iter()
        .map(|name| (XPathExpandedName::unqualified(name), name.into()))
        .collect(),
        ..Default::default()
    };
    compile_xslt_bundle_with_options(VIEW, "memory:data-table-view.xslt", &options).unwrap()
}
fn bundle() -> XsltBundle {
    let compiled = compiled();
    XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap()
}

fn cemt(source: &str, format: &str, column: &str) -> RenderPlan {
    cemt_controls(source, format, column, "number", "ascending", "")
}
fn cemt_controls(
    source: &str,
    format: &str,
    column: &str,
    mode: &str,
    direction: &str,
    selected: &str,
) -> RenderPlan {
    let record = |fields: Vec<(&str, Item)>| {
        Item::Record(
            fields
                .into_iter()
                .map(|(k, v)| (k.into(), vec![v]))
                .collect(),
        )
    };
    let string = |value: &str| Item::Atomic(AtomValue::String(value.into()));
    let data = TemplateData::default()
        .with_binding("format", ItemStream::once(string(format)))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    record(vec![("nodes", record(vec![("text", string(source))]))]),
                ),
                (
                    "slices",
                    record(vec![
                        ("column", string(column)),
                        ("mode", string(mode)),
                        ("direction", string(direction)),
                        ("selected", string(selected)),
                    ]),
                ),
            ])),
        );
    let artifact = compile_template(
        include_str!("../../cem-elements/demo/data-table-view.cemt"),
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    plan
}

#[derive(Debug, PartialEq, serde::Serialize)]
enum VisibleNode {
    Text(String),
    Element(String, Vec<(String, String)>, Vec<VisibleNode>),
}

fn visible(nodes: &[RenderPlanNode]) -> Vec<VisibleNode> {
    fn flush(text: &mut String, out: &mut Vec<VisibleNode>) {
        if !text.trim().is_empty() {
            out.push(VisibleNode::Text(text.trim().into()));
        }
        text.clear();
    }
    let mut out = vec![];
    let mut pending = String::new();
    for node in nodes {
        match node {
            RenderPlanNode::Text { text, .. } => pending.push_str(text),
            RenderPlanNode::Element {
                tag,
                attributes,
                children,
                ..
            } => {
                flush(&mut pending, &mut out);
                if tag == "style" {
                    continue;
                }
                let selection = tag == "button"
                    && attributes
                        .iter()
                        .any(|a| a.name == "slice" && a.value == "selected");
                let mut attrs: Vec<_> = attributes
                    .iter()
                    .filter(|a| {
                        !(selection && a.name == "value")
                            && !(tag == "select" && a.name == "value" && a.value.is_empty())
                    })
                    .map(|a| (a.name.clone(), a.value.clone()))
                    .collect();
                attrs.sort();
                out.push(VisibleNode::Element(tag.clone(), attrs, visible(children)));
            }
            other => panic!("unexpected render node {other:?}"),
        }
    }
    flush(&mut pending, &mut out);
    out
}

fn difference(left: &[VisibleNode], right: &[VisibleNode]) -> String {
    if left.len() != right.len() {
        return format!(
            "node count {} != {}: {left:?} / {right:?}",
            left.len(),
            right.len()
        );
    }
    for (a, b) in left.iter().zip(right) {
        if a == b {
            continue;
        }
        return match (a, b) {
            (VisibleNode::Element(at, aa, ac), VisibleNode::Element(bt, ba, bc))
                if at == bt && aa == ba =>
            {
                format!("{at}/{}", difference(ac, bc))
            }
            _ => format!("{a:?} != {b:?}"),
        };
    }
    String::new()
}

#[test]
fn native_viewer_dom_matches_cemt_for_namespaces_nested_missing_and_empty_values() {
    let bundle = bundle();
    for (format, source, column) in [
        (
            "xml",
            r#"<r xmlns="urn:root" xmlns:p="urn:rows"><p:row xmlns:q="urn:field" id="1" q:xmlns="first">A</p:row><p:row xmlns:q="urn:field" id="2" q:xmlns="second">B</p:row><single xmlns:s="urn:detail" s:xmlns="detail"/></r>"#,
            "",
        ),
        (
            "xml",
            r#"<r><row n="10">A<a>x</a></row><row n="2" later="">B<a>y</a><a>z</a></row><single><!--note--><?pi value?></single></r>"#,
            "@n",
        ),
        (
            "json",
            r#"[{"note":"","tags":["red","sweet"]},{"qty":2},{"note":null}]"#,
            "",
        ),
        (
            "json",
            r#"{"empty":[],"items":[1,null,""],"nested":{"label":"A"}}"#,
            "",
        ),
        (
            "yaml",
            "empty: []\nitems:\n  - qty: 10\n    tags: [a, b]\n  - qty: 2\n    note: null",
            "qty",
        ),
        (
            "csv",
            "qty,note\n10,\"red, sweet\"\n2,\"say \"\"hello\"\"\"",
            "qty",
        ),
        ("json", "[]", ""),
        ("json", "{}", ""),
        ("xml", "<r/>", ""),
        ("csv", "label", ""),
    ] {
        let actual = visible(&render(&bundle, source, format, column).nodes);
        let expected = visible(&cemt(source, format, column).nodes);
        assert!(
            actual == expected,
            "{format}: {source}\n{}",
            difference(&actual, &expected)
        );
    }
}

fn render(bundle: &XsltBundle, source: &str, format: &str, column: &str) -> RenderPlan {
    render_controls(bundle, source, format, column, "number", "ascending", "")
}
fn render_controls(
    bundle: &XsltBundle,
    source: &str,
    format: &str,
    column: &str,
    mode: &str,
    direction: &str,
    selected: &str,
) -> RenderPlan {
    let mut data = TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data("<input/>", "xml", "cem", "memory:viewer-input").unwrap(),
        )),
    );
    for (name, value) in [
        ("source", source),
        ("initial", source),
        ("format", format),
        ("column", column),
        ("direction", direction),
        ("mode", mode),
        ("selected", selected),
    ] {
        data = data.with_binding(
            name,
            ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
        );
    }
    let plan = bundle.render(&data);
    assert!(
        plan.diagnostics.is_empty(),
        "{format}: {:?}",
        plan.diagnostics
    );
    plan
}

#[test]
fn viewer_sorting_matches_cemt_for_finite_numbers_missing_invalid_and_stable_ties() {
    let bundle = bundle();
    let source = r#"[{"n":"10","label":"A"},{"n":"2","label":"B"},{"n":"2","label":"C"},{"n":"1e2","label":"D"},{"n":"NaN","label":"E"},{"label":"F"},{"n":"INF","label":"G"},{"n":"","label":"H"},{"n":"oops","label":"I"}]"#;
    for column in ["", "n"] {
        for mode in ["text", "number"] {
            for direction in ["ascending", "descending"] {
                let actual = visible(
                    &render_controls(&bundle, source, "json", column, mode, direction, "").nodes,
                );
                let expected =
                    visible(&cemt_controls(source, "json", column, mode, direction, "").nodes);
                assert!(
                    actual == expected,
                    "{column}/{mode}/{direction}: {}",
                    difference(&actual, &expected)
                );
            }
        }
    }
}

fn selections(nodes: &[RenderPlanNode]) -> Vec<(String, String, String)> {
    let mut found = vec![];
    for node in nodes {
        if let RenderPlanNode::Element {
            attributes,
            children,
            ..
        } = node
        {
            let attr = |name: &str| {
                attributes
                    .iter()
                    .find(|a| a.name == name)
                    .map(|a| a.value.clone())
                    .unwrap_or_default()
            };
            if attr("slice") == "selected" {
                found.push((attr("value"), attr("aria-label"), attr("aria-pressed")));
            }
            found.extend(selections(children));
        }
    }
    found
}

#[test]
fn viewer_selection_survives_sorting_and_clears_after_source_edits() {
    let bundle = bundle();
    let mut cases = vec![];
    for (format, source, column) in [
        (
            "xml",
            "<r>\n<row n='2'>A</row>\n<row n='10'>B</row></r>",
            "@n",
        ),
        ("json", "[\n{\"n\":2},\n{\"n\":10}]", "n"),
        ("csv", "n\n2\n10", "n"),
        ("yaml", "- n: 2\n- n: 10", "n"),
    ] {
        let before = render(&bundle, source, format, column);
        let key = &selections(&before.nodes)[0].0;
        let sorted = render_controls(&bundle, source, format, column, "number", "descending", key);
        let rows = selections(&sorted.nodes);
        assert_eq!(rows[1].0, *key);
        assert_eq!(rows[1].2, "true");
        assert_eq!(rows[0].2, "false");
        let edited = render_controls(
            &bundle,
            &source.replace("10", "11"),
            format,
            column,
            "number",
            "descending",
            key,
        );
        assert!(selections(&edited.nodes)
            .iter()
            .all(|(id, _, pressed)| id != key && pressed == "false"));
        for (plan, source, direction, selected) in [
            (&before, source.to_owned(), "ascending", ""),
            (&sorted, source.to_owned(), "descending", key.as_str()),
            (
                &edited,
                source.replace("10", "11"),
                "descending",
                key.as_str(),
            ),
        ] {
            cases.push(serde_json::json!({
                "controls":{"source":source,"initial":source,"format":format,"column":column,
                    "mode":"number","direction":direction,"selected":selected},
                "nodes":visible(&plan.nodes),"selections":selections(&plan.nodes),
            }));
        }
    }
    if let Ok(directory) = std::env::var("CEM_XSLT_VIEW_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let compiled = compiled();
        let default =
            cem_ql::xslt::compiler::compile_xslt_bundle(VIEW, "memory:data-table-view.xslt")
                .unwrap();
        std::fs::write(directory.join("viewer.xslt"), VIEW).unwrap();
        std::fs::write(directory.join("viewer.bin"), compiled.bytes).unwrap();
        std::fs::write(directory.join("viewer-default.bin"), default.bytes).unwrap();
        // Explicit deployment, scalar control and render-output metadata only.
        std::fs::write(directory.join("viewer.json"),serde_json::to_vec(&serde_json::json!({
            "contentHash":compiled.content_hash.header_value(),"sourceHash":compiled.source_hash.header_value(),"cases":cases,
        })).unwrap()).unwrap();
    }
}

#[test]
fn viewer_reports_malformed_sources_and_remains_resettable() {
    let bundle = bundle();
    for (format, source) in [
        ("xml", "<r>"),
        ("json", "[oops]"),
        ("csv", "label\n\"unclosed"),
        ("yaml", "items: ["),
    ] {
        let html = render_plan_to_html(&render(&bundle, source, format, ""));
        assert!(html.contains("role=\"alert\""), "{format}: {html}");
        assert!(html.contains("aria-label=\"Reset source\""));
        assert_eq!(html.matches("<select ").count(),3,"{format}: {html}");
        assert!(!html.contains("<table"));
    }
}

#[test]
fn authored_xslt_view_imports_and_sorts_all_four_formats() {
    let bundle = bundle();
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
        let html = render_plan_to_html(&render(&bundle, source, format, column));
        let table = html
            .split("<tbody>")
            .nth(1)
            .unwrap_or_else(|| panic!("{format}: {html}"));
        assert!(
            table.find('🍋') < table.find('🍌') && table.find('🍌') < table.find('🍒'),
            "{format}: {table}"
        );
        assert!(table.contains("Select source line"));
    }
}
