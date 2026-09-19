//! XSLT-DATA-PREREQUISITES: characterize the remaining shared parsing boundary.
//! Missing-function/error assertions document a scope gate, not conformance.
//! Replace them with positive acceptance tests when that expansion is approved.
use cem_ml::{
    diagnostics::Diagnostic,
    import::{import_data, MAX_BYTES},
    parser::tree::RetainedCemTree,
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::{
        xpath_expression_ast_from_source_bytes, CemXPathEvaluator, XPathAttachment,
        XPathDynamicContext, XPathEvaluationRequest, XPathEvaluatorAdapter, XPathInvocationHost,
        XPathNativeNode, XPathResultArtifact, XPathResultItem, XPathSourceRequest,
    },
};
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_plan_to_html, TemplateData},
    xslt::{compiler::compile_xslt_bundle, XsltBundle},
};
use std::sync::Arc;

fn evaluate(
    source: &str,
    tree: Option<Arc<RetainedCemTree>>,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:data-prerequisites.xpath",
            content_type: Some("application/xpath"),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    CemXPathEvaluator::default().evaluate(XPathEvaluationRequest {
        invocation_host: XPathInvocationHost::StandaloneTransform,
        expression: &expression,
        dynamic_context: XPathDynamicContext {
            context_item: tree
                .map(|tree| XPathResultItem::from_native_node(XPathNativeNode::cem_document(tree))),
            ..Default::default()
        },
        static_context: Default::default(),
        expected_result: None,
        resolver_registry: &ResolverRegistry::new(),
        resolver_policy: &ResolverPolicy::new(),
        evaluation_limits: Default::default(),
        safety_policy_stamp: "xslt-data-prerequisites",
        module_resolution: None,
    })
}

#[test]
fn imported_xml_and_json_nodes_already_retain_owners_through_xpath() {
    for (source, format, projection, select, values) in [
        (
            "<r><row>a<![CDATA[b]]></row><row/></r>",
            "xml",
            "cem",
            "/*/*",
            ["ab", ""],
        ),
        (
            r#"{"rows":[null,""]}"#,
            "json",
            "json-to-xml",
            "/*/*/*",
            ["", ""],
        ),
    ] {
        let tree = import_data(source, format, projection, "memory:imported-data").unwrap();
        let result = evaluate(select, Some(Arc::clone(&tree))).unwrap();
        assert_eq!(result.sequence.items.len(), 2);
        let nodes: Vec<_> = result
            .sequence
            .items
            .iter()
            .map(|item| item.native_node().unwrap())
            .collect();
        assert_ne!(
            nodes[0], nodes[1],
            "empty values must remain distinct nodes"
        );
        for (node, value) in nodes.into_iter().zip(values) {
            assert_eq!(node.string_value(), value);
            assert!(Arc::ptr_eq(node.owner(), &tree));
            assert!(node.source_owner().is_some());
            assert!(!node.source_map().frames.is_empty());
        }
    }
}

#[test]
fn standard_string_parsers_are_currently_missing_inside_xpath_programs() {
    for call in [
        "parse-xml('<r/>')",
        "parse-xml(())",
        "json-to-xml('{\"rows\":[null]}')",
        "json-to-xml('{}', map {})",
        "json-to-xml(())",
        "(function($s) { parse-xml($s) })('<r/>')",
    ] {
        let errors = evaluate(call, None).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|d| d.code == "cem.xpath.evaluation_unsupported"),
            "{call}: {errors:?}"
        );
        assert!(errors.iter().all(|d| d.source_map.is_some()));
        // A compiler rewrite must not hoist parsing out of conditional branches.
        let skipped = evaluate(&format!("if (false()) then {call} else 'skipped'"), None).unwrap();
        assert!(matches!(
            skipped.sequence.items.as_slice(),
            [XPathResultItem::Atomic { value, .. }] if value.lexical_value == "skipped"
        ));
    }
}

#[test]
fn import_rejects_invalid_documents_but_erases_failure_categories() {
    for source in ["", "<a/><b/>", "<missing:r/>", "<r>"] {
        assert!(
            import_data(source, "xml", "cem", "memory:invalid.xml").is_err(),
            "{source:?} must not become an XML document"
        );
    }
    let large = format!("\"{}\"", "a".repeat(MAX_BYTES));
    // The public API returns only prose for malformed input, resource limits,
    // unsupported capabilities and unsupported projections. An XSLT catch must
    // not guess these categories from their message strings.
    let errors: Vec<String> = [
        ("[", "json", "json-to-xml"),
        (large.as_str(), "json", "json-to-xml"),
        ("<!DOCTYPE r><r/>", "xml", "cem"),
        ("{}", "json", "unknown"),
    ]
    .into_iter()
    .map(|(source, format, projection)| {
        import_data(source, format, projection, "memory:import-error").unwrap_err()
    })
    .collect();
    assert!(errors.iter().all(|message| !message.is_empty()));
    assert!(errors.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn xslt_failure_retains_location_but_has_no_structured_standard_error_name() {
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
 xmlns:xs="http://www.w3.org/2001/XMLSchema" version="3.0">
 <xsl:template match="/">
  <p>discard this output</p>
  <xsl:value-of select="xs:integer('invalid')"/>
 </xsl:template>
</xsl:stylesheet>"#;
    let compiled = compile_xslt_bundle(source, "memory:data-error.xslt").unwrap();
    let bundle = XsltBundle::from_bytes(
        &compiled.bytes,
        &compiled.content_hash,
        &compiled.source_hash,
    )
    .unwrap();
    let plan = bundle.render(&TemplateData::default().with_binding(
        "document",
        ItemStream::once(imported_cem_tree(
            import_data("<r/>", "xml", "cem", "memory:input.xml").unwrap(),
        )),
    ));
    assert_eq!(render_plan_to_html(&plan), "");
    let error = plan
        .diagnostics
        .iter()
        .find(|d| d.code == "cem.xpath.cast_invalid")
        .unwrap_or_else(|| panic!("{:?}", plan.diagnostics));
    assert!(error.details.is_none());
    assert_eq!(error.uri.as_deref(), Some("memory:data-error.xslt"));
    assert_eq!(error.line, Some(5));
    assert!(error.source_map.is_some());
}
