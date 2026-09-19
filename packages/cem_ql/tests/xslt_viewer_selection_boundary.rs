//! XSLT-VIEW-SELECTION-BOUNDARY: prerequisites for source-based row selection.
//! These probes characterize the current boundary, not a completed viewer.
use cem_ml::{
    import::import_data,
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::{
        xpath_expression_ast_from_source_bytes, CemXPathEvaluator, XPathAttachment,
        XPathEvaluationRequest, XPathEvaluatorAdapter, XPathInvocationHost, XPathResultItem,
        XPathSourceRequest,
    },
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{render_plan_to_html, TemplateData},
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
fn csv_string_import_and_cemt_reader_currently_disagree_on_headers() {
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
}

#[test]
fn stylesheet_source_metadata_access_is_not_currently_an_installed_capability() {
    // These names describe the proposed extension, not a public API today.
    for function in ["Q{urn:cem:source}node-key", "Q{urn:cem:source}line-number"] {
        let source = format!(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
<xsl:template match="/"><p>discard on failure</p><xsl:value-of select="{function}(/*)"/></xsl:template>
</xsl:stylesheet>"#
        );
        let compiled = compile_xslt_bundle(&source, "memory:viewer-selection.xslt").unwrap();
        let bundle = XsltBundle::from_bytes(
            &compiled.bytes,
            &compiled.content_hash,
            &compiled.source_hash,
        )
        .unwrap();
        let plan = bundle.render(&TemplateData::default().with_binding(
            "document",
            ItemStream::once(imported_cem_tree(
                import_data("<r/>", "xml", "cem", "memory:viewer-input").unwrap(),
            )),
        ));
        assert_eq!(render_plan_to_html(&plan), "");
        assert!(
            plan.diagnostics.iter().any(|d| {
                d.code == "cem.xpath.evaluation_unsupported"
                    && d.message.contains(function.rsplit('}').next().unwrap())
                    && d.source_map.is_some()
            }),
            "{function}: {:?}",
            plan.diagnostics
        );
    }
}
