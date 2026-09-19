//! XSLT-VIEW-SELECTION-BOUNDARY: prerequisites for source-based row selection.
//! Native provenance is approved; the full viewer remains a separate fixture.
use cem_ml::{
    import::import_data,
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::{
        xpath_expression_ast_from_source_bytes, CemXPathEvaluator, XPathAttachment,
        XPathEvaluationRequest, XPathEvaluatorAdapter, XPathInvocationHost, XPathNativeNode,
        XPathResultItem, XPathSourceRequest,
    },
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{render_plan_to_html, render_template, RenderPlanNode, TemplateData},
    xslt::{compiler::compile_xslt_bundle, XsltBundle},
};
use std::{collections::BTreeMap, sync::Arc};

const CASES: [(&str, &str, &str, [u32; 2]); 4] = [
    (
        "xml",
        "<r>\n<row>B</row>\n<row>A</row>\n</r>",
        "parse-xml",
        [2, 3],
    ),
    (
        "json",
        "[\n{\"label\":\"B\"},\n{\"label\":\"A\"}\n]",
        "json-to-xml",
        [2, 3],
    ),
    ("csv", "label\nB\nA", "Q{urn:cem:import}parse-csv", [2, 3]),
    (
        "yaml",
        "- label: B\n- label: A",
        "Q{urn:cem:import}parse-yaml",
        [1, 2],
    ),
];

fn xpath(source: &str) -> Vec<XPathResultItem> {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:viewer-selection.xpath",
            content_type: Some("application/xpath"),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    CemXPathEvaluator::default()
        .evaluate(XPathEvaluationRequest {
            expression: &expression,
            invocation_host: XPathInvocationHost::StandaloneTransform,
            dynamic_context: Default::default(),
            static_context: Default::default(),
            expected_result: None,
            resolver_registry: &ResolverRegistry::new(),
            resolver_policy: &ResolverPolicy::new(),
            evaluation_limits: Default::default(),
            safety_policy_stamp: "viewer-selection-boundary",
            module_resolution: None,
        })
        .unwrap()
        .sequence
        .items
}

const SELECTION: &str = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
 xmlns:source="urn:cem:source" xmlns:import="urn:cem:import" xmlns:err="http://www.w3.org/2005/xqt-errors" version="3.0">
<xsl:template match="/">
 <xsl:variable name="format" select="string(/*/@format)"/>
 <xsl:variable name="direction" select="string(/*/@direction)"/>
 <xsl:variable name="selected" select="string(/*/@selected)"/>
 <xsl:try>
  <xsl:variable name="parsed" select="if ($format = 'xml') then parse-xml(string(/*)) else if ($format = 'json') then json-to-xml(string(/*)) else if ($format = 'csv') then import:parse-csv(string(/*), map {'header':string(/*/@header)}) else import:parse-yaml(string(/*))"/>
  <xsl:for-each select="$parsed/*/*"><xsl:sort select="string(.)" order="{$direction}"/>
   <button value="{source:node-key(.)}" aria-label="Select source line {source:line-number(.)}" aria-pressed="{source:node-key(.) = $selected}"><xsl:value-of select="."/></button>
  </xsl:for-each>
  <xsl:catch errors="import:invalid-options"><p role="alert">Invalid CSV options</p></xsl:catch>
 </xsl:try>
</xsl:template></xsl:stylesheet>"#;

fn selection_input(
    format: &str,
    source: &str,
    direction: &str,
    selected: &str,
    header: &str,
) -> String {
    let source = source
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r#"<input format="{format}" direction="{direction}" selected="{selected}" header="{header}">{source}</input>"#
    )
}

fn selection_rows(bundle: &XsltBundle, input: &str) -> Vec<(String, String, String, String)> {
    let plan = bundle.render(&TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data(input, "xml", "cem", "memory:selection-input").unwrap(),
        )),
    ));
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    plan.nodes
        .iter()
        .map(|node| {
            let RenderPlanNode::Element {
                attributes,
                children,
                ..
            } = node
            else {
                panic!("expected button");
            };
            let attr = |name: &str| {
                attributes
                    .iter()
                    .find(|attr| attr.name == name)
                    .unwrap()
                    .value
                    .clone()
            };
            let label = children
                .iter()
                .map(|child| match child {
                    RenderPlanNode::Text { text, .. } => text.as_str(),
                    _ => panic!("expected label"),
                })
                .collect::<String>();
            (
                attr("value"),
                attr("aria-label"),
                attr("aria-pressed"),
                label,
            )
        })
        .collect()
}

#[test]
fn parsed_row_selection_survives_rerenders_sort_changes_and_bundle_reload() {
    let compiled = compile_xslt_bundle(SELECTION, "memory:selection.xslt").unwrap();
    let bundle = XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap();
    let mut exported = vec![];
    for (format, source, _, lines) in CASES {
        let initial = selection_input(format, source, "ascending", "", "present");
        let first = selection_rows(&bundle, &initial);
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].3, "A");
        assert_eq!(first[1].3, "B");
        assert_eq!(first[0].1, format!("Select source line {}", lines[1]));
        assert_eq!(first[1].1, format!("Select source line {}", lines[0]));
        assert_ne!(first[0].0, first[1].0);
        let selected = selection_input(format, source, "descending", &first[0].0, "present");
        let second = selection_rows(&bundle, &selected);
        assert_eq!(second[1].0, first[0].0);
        assert_eq!(second[1].2, "true");
        assert_eq!(second[0].2, "false");
        assert_eq!(second[0].3, "B");
        let changed = selection_input(
            format,
            &source.replace('B', "C"),
            "ascending",
            &first[0].0,
            "present",
        );
        let third = selection_rows(&bundle, &changed);
        assert!(third
            .iter()
            .all(|row| row.2 == "false" && row.0 != first[0].0));
        // Explicit expected render-output metadata for native/WASM comparison.
        exported.extend([
            serde_json::json!({"input": initial, "rows": first}),
            serde_json::json!({"input": selected, "rows": second}),
            serde_json::json!({"input": changed, "rows": third}),
        ]);
    }
    let invalid = selection_input("csv", "label\nA", "ascending", "", "invalid");
    let plan = bundle.render(&TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data(&invalid, "xml", "cem", "memory:selection-input").unwrap(),
        )),
    ));
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan),
        "<p role=\"alert\">Invalid CSV options</p>"
    );
    if let Ok(directory) = std::env::var("CEM_SOURCE_PROVENANCE_FIXTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::write(directory.join("selection.xslt"), SELECTION).unwrap();
        std::fs::write(directory.join("selection.bin"), compiled.bytes).unwrap();
        std::fs::write(directory.join("selection.json"), serde_json::to_vec(&serde_json::json!({
            "sourceHash":compiled.source_hash.header_value(), "contentHash":compiled.content_hash.header_value(),
            "cases":exported, "invalid":invalid,
        })).unwrap()).unwrap();
    }
}

fn field(item: &Item, name: &str) -> String {
    match item.view().unwrap().field(name).unwrap().as_slice() {
        [Item::Atomic(AtomValue::String(value))] => value.clone(),
        other => panic!("expected {name} string: {other:?}"),
    }
}

fn cemt_rows(source: &str, format: &str) -> Vec<Item> {
    // Only scalar host controls; the external document is imported by data:read.
    let bindings = BTreeMap::from([
        (
            "source".into(),
            ItemStream::once(Item::Atomic(AtomValue::String(source.into()))),
        ),
        (
            "format".into(),
            ItemStream::once(Item::Atomic(AtomValue::String(format.into()))),
        ),
    ]);
    let query = compile(
        r#"seq:where(data:read(source, format).root.children.children, fn(n) => n.kind == "element")"#,
        &CompileContext { policy_bindings: bindings.clone(), ..Default::default() },
    ).unwrap();
    let result = evaluate(
        &query,
        &EvaluationContext {
            policy_bindings: bindings,
            ..Default::default()
        },
    );
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    result.items
}

#[test]
fn existing_cemt_selection_keys_and_source_lines_survive_fresh_reads() {
    for (format, source, _, lines) in CASES {
        let first = cemt_rows(source, format);
        let reread = cemt_rows(source, format);
        assert_eq!(first.len(), 2, "{format}");
        assert_eq!(reread.len(), 2, "{format}");
        assert_ne!(field(&first[0], "id"), field(&first[1], "id"));
        for ((before, after), line) in first.iter().zip(&reread).zip(lines) {
            assert_eq!(field(before, "id"), field(after, "id"), "{format}");
            assert_eq!(field(before, "line"), line.to_string(), "{format}");
            assert!(!before.source_map().unwrap().frames.is_empty());
        }
        let changed = cemt_rows(&source.replace('B', "C"), format);
        assert_ne!(field(&first[0], "id"), field(&changed[0], "id"), "{format}");
    }
}

#[test]
fn cemt_namespace_declarations_are_not_attributes_in_the_xpath_data_model() {
    let source = "<r><p:row xmlns:p='urn:rows' id='1'/><p:row xmlns:p='urn:rows' id='2'/></r>";
    let rows = cemt_rows(source, "xml");
    for row in &rows {
        let attributes = row.view().unwrap().field("attributes").unwrap();
        assert_eq!(attributes.len(), 2);
        assert_eq!(
            field(&attributes[0], "namespace"),
            "http://www.w3.org/2000/xmlns/"
        );
        assert_eq!(field(&attributes[0], "name"), "p");
        assert_eq!(field(&attributes[1], "name"), "id");
    }
    let attributes = xpath(&format!(
        "parse-xml('{}')/*/*/@*",
        source.replace('\'', "''")
    ));
    assert_eq!(attributes.len(), 2);
    assert!(attributes
        .iter()
        .all(|item| item.native_node().unwrap().local_name() == "id"));
    // Existing host-control shape only; the XML source itself stays text until
    // cem-data imports it. No document-object substitute is built by this test.
    let record =
        |name: &str, value: Item| Item::Record(BTreeMap::from([(name.into(), vec![value])]));
    let payload = record(
        "payload",
        record(
            "nodes",
            record("text", Item::Atomic(AtomValue::String(source.into()))),
        ),
    );
    let rendered = render_template(
        include_str!("../../cem-elements/demo/data-table-view.cemt"),
        &TemplateData::default()
            .with_binding("datadom", ItemStream::once(payload))
            .with_binding(
                "format",
                ItemStream::once(Item::Atomic(AtomValue::String("xml".into()))),
            ),
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert!(!rendered
        .rendered
        .contains("<th scope=\"col\">@http://www.w3.org/2000/xmlns/|p</th>"));
    assert!(rendered.rendered.contains("<th scope=\"col\">@id</th>"));
    // The approved presentation excludes declarations; generic data and XPath
    // retain their respective source-oriented and semantic attribute contracts.
}

#[test]
fn standard_imports_retain_original_lines_and_nodes_during_sorting() {
    for (format, source, function, lines) in CASES {
        // Isolate provenance for the two actual data rows. The CSV header
        // mismatch is asserted separately below; this is not a viewer workaround.
        let rows = if format == "csv" {
            "/*/*[position() gt 1]"
        } else {
            "/*/*"
        };
        let select = format!(
            "let $rows := {function}('{}'){rows} return ($rows, sort($rows, (), function($row) {{ string($row) }}))",
            source.replace('\'', "''")
        );
        let first = xpath(&select);
        let reread = xpath(&select);
        assert_eq!(first.len(), 4, "{format}");
        assert_eq!(reread.len(), 4, "{format}");
        for (index, expected) in [(2, 1), (3, 0)] {
            let original = first[expected].native_node().unwrap();
            let sorted = first[index].native_node().unwrap();
            assert_eq!(original, sorted, "{format}");
            assert!(Arc::ptr_eq(original.owner(), sorted.owner()));
            assert!(original.source_owner().is_some());
            assert_eq!(original.source_map(), sorted.source_map());
            let XPathResultItem::Node {
                source_range: Some(range),
                ..
            } = &first[index]
            else {
                panic!("source range missing for {format}");
            };
            assert_eq!(range.start.line, lines[expected], "{format}");
        }
        assert_eq!(first[2].native_node().unwrap().string_value(), "A");
        assert_eq!(first[3].native_node().unwrap().string_value(), "B");
        // Separate parses create separate XDM documents. A pointer-based node
        // identity cannot serve as a persisted UI selection key across renders.
        assert_ne!(first[0].native_node(), reread[0].native_node(), "{format}");
        assert_ne!(
            first[0].native_node().unwrap().identity(),
            reread[0].native_node().unwrap().identity(),
            "{format}"
        );
    }
}

#[test]
fn csv_one_argument_profile_stays_compatible_and_explicit_headers_match_cemt() {
    let source = "label\nB\nA";
    let cemt = cemt_rows(source, "csv");
    assert_eq!(cemt.len(), 2);
    assert_eq!(field(&cemt[0], "name"), "object");
    let parsed = xpath("Q{urn:cem:import}parse-csv('label\nB\nA')/*/*");
    assert_eq!(parsed.len(), 3);
    assert_eq!(
        parsed
            .iter()
            .map(|item| item.native_node().unwrap().string_value())
            .collect::<Vec<_>>(),
        ["label", "B", "A"]
    );
    assert_eq!(parsed[0].native_node().unwrap().local_name(), "array");
    let present = xpath("Q{urn:cem:import}parse-csv('label\nB\nA', map {'header':'present'})/*/*");
    assert_eq!(present.len(), cemt.len());
    assert_eq!(present[0].native_node().unwrap().local_name(), "object");
    assert_eq!(present[0].native_node().unwrap().string_value(), "B");
}

#[test]
fn stylesheet_source_metadata_access_is_native_and_survives_bundle_reload() {
    for function in ["Q{urn:cem:source}node-key", "Q{urn:cem:source}line-number"] {
        let source = format!(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
<xsl:template match="/"><xsl:value-of select="{function}(/*)"/></xsl:template>
</xsl:stylesheet>"#
        );
        let compiled = compile_xslt_bundle(&source, "memory:viewer-selection.xslt").unwrap();
        let bundle = XsltBundle::from_bytes(
            &compiled.bytes,
            &compiled.content_hash,
            &compiled.source_hash,
        )
        .unwrap();
        let tree = import_data("<r/>", "xml", "cem", "memory:viewer-input").unwrap();
        let root = XPathNativeNode::cem_document(tree.clone()).child_nodes()[0].clone();
        let plan = bundle.render(
            &TemplateData::default()
                .with_binding("document", ItemStream::once(imported_cem_tree(tree))),
        );
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(
            render_plan_to_html(&plan),
            if function.ends_with("node-key") {
                root.source_key().unwrap()
            } else {
                "1".into()
            }
        );
    }
}
