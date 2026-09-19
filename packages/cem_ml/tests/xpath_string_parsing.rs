//! XSLT-DATA-XPATH: standard parsing keeps native owners and typed failures.
use cem_ml::{
    diagnostics::Diagnostic,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::{
        xpath_expression_ast_from_source_bytes, CemXPathEvaluator, XPathAttachment,
        XPathEvaluationLimits, XPathEvaluationRequest, XPathEvaluatorAdapter, XPathInvocationHost,
        XPathResultArtifact, XPathResultItem, XPathSourceRequest,
    },
};

fn run(source: &str) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    controlled(source, XPathEvaluationLimits::default(), None)
}
fn controlled(
    source: &str,
    limits: XPathEvaluationLimits,
    control: Option<&OperationControl>,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "https://example.test/view.xslt",
            content_type: Some("application/xpath"),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    let registry = ResolverRegistry::new();
    let policy = ResolverPolicy::new();
    let request = XPathEvaluationRequest {
        invocation_host: XPathInvocationHost::StandaloneTransform,
        expression: &expression,
        dynamic_context: Default::default(),
        static_context: Default::default(),
        expected_result: None,
        resolver_registry: &registry,
        resolver_policy: &policy,
        evaluation_limits: limits,
        safety_policy_stamp: "string-parsing-tests",
        module_resolution: None,
    };
    match control {
        Some(control) => CemXPathEvaluator::default().evaluate_with_control(
            request,
            control,
            ROOT_EXECUTION_SCOPE_ID,
        ),
        None => CemXPathEvaluator::default().evaluate(request),
    }
}
fn values(source: &str) -> Vec<String> {
    run(source)
        .unwrap_or_else(|e| panic!("{source}: {e:?}"))
        .sequence
        .items
        .iter()
        .map(|i| match i {
            XPathResultItem::Atomic { value, .. } => value.lexical_value.clone(),
            item => item.native_node().unwrap().string_value(),
        })
        .collect()
}

#[test]
fn standard_parsing_supports_paths_empty_inputs_and_lazy_nested_calls() {
    assert_eq!(
        values("(parse-xml(()), json-to-xml(()))"),
        Vec::<String>::new()
    );
    assert_eq!(
        values("parse-xml('<r>a<![CDATA[b]]>&amp;</r>')/r/text()"),
        ["ab&"]
    );
    assert_eq!(
        values("json-to-xml('{\"rows\":[null,\"\"]}')/*/*/* ! local-name()"),
        ["null", "string"]
    );
    assert_eq!(
        values("if (false()) then parse-xml('bad') else 'safe'"),
        ["safe"]
    );
    assert_eq!(
        values("(function($s) { parse-xml($s)/r/@id })('<r id=\"yes\"/>')"),
        ["yes"]
    );
    assert_eq!(values("('<r/>', '<x/>')[exists(parse-xml(.)/r)]"), ["<r/>"]);
}

#[test]
fn parsed_nodes_keep_native_identity_source_and_uri_properties() {
    let result = run("let $d := parse-xml('<r/>') return ($d, $d/r/.., $d/r)").unwrap();
    let nodes: Vec<_> = result
        .sequence
        .items
        .iter()
        .map(|i| i.native_node().unwrap())
        .collect();
    assert_eq!(nodes[0], nodes[1]);
    assert!(std::sync::Arc::ptr_eq(nodes[0].owner(), nodes[2].owner()));
    assert!(nodes[0].source_owner().is_some());
    assert!(!nodes[2].source_map().frames.is_empty());
    assert_eq!(
        values("base-uri(parse-xml('<r/>'))"),
        ["https://example.test/view.xslt"]
    );
    assert!(values("document-uri(parse-xml('<r/>'))").is_empty());
    assert_eq!(values("parse-xml('<r xml:base=\"../data/\"><x xml:base=\"items/\" a=\"1\"/><y/></r>')/r/(x, x/@a, y) ! base-uri(.)"), ["https://example.test/data/items/", "https://example.test/data/items/", "https://example.test/data/"]);
}

#[test]
fn parsing_observes_import_text_work_sequence_limits_and_cancellation() {
    let errors = run(&format!(
        "json-to-xml('{}')",
        " ".repeat(cem_ml::import::MAX_BYTES + 1)
    ))
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.import_limit_exceeded");
    assert!(errors[0].error_name().is_none());
    for (limits, query, code) in [
        (
            XPathEvaluationLimits {
                max_text_bytes: Some(3),
                ..Default::default()
            },
            "parse-xml('<r/>')",
            "cem.xpath.text_byte_limit_exceeded",
        ),
        (
            XPathEvaluationLimits {
                max_work_units: Some(3),
                ..Default::default()
            },
            "parse-xml('<r/>')",
            "cem.xpath.work_limit_exceeded",
        ),
        (
            XPathEvaluationLimits {
                max_sequence_items: Some(1),
                ..Default::default()
            },
            "parse-xml('<r><x/><x/></r>')/r/x",
            "cem.xpath.sequence_item_limit_exceeded",
        ),
    ] {
        assert_eq!(controlled(query, limits, None).unwrap_err()[0].code, code);
    }
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let errors = controlled("parse-xml('<r/>')", Default::default(), Some(&control)).unwrap_err();
    assert!(errors.iter().any(|e| e.code == "cem.xpath.control_failure"));
}

#[test]
fn json_options_preserve_duplicates_escaping_and_standard_error_identity() {
    assert_eq!(values("json-to-xml('{\"a\":1,\"a\":2}')/*/*"), ["1", "2"]);
    assert_eq!(
        values("json-to-xml('{\"a\":1,\"a\":2}', map {'duplicates':'use-first'})/*/*"),
        ["1"]
    );
    assert_eq!(
        values("json-to-xml('\"\\uDEAD\"', map {'escape':true()})/*"),
        ["\\udead"]
    );
    for (source, code) in [
        ("parse-xml('')", "FODC0006"),
        ("json-to-xml('')", "FOJS0001"),
        (
            "json-to-xml('{\"a\":1,\"a\":2}', map {'duplicates':'reject'})",
            "FOJS0003",
        ),
        ("json-to-xml('{}', map {'validate':true()})", "FOJS0004"),
        ("json-to-xml('{}', map {'duplicates':'bad'})", "FOJS0005"),
        ("parse-xml(1)", "XPTY0004"),
        ("xs:integer('bad')", "FORG0001"),
        ("xs:integer(xs:double('INF'))", "FOCA0002"),
        ("xs:integer(xs:anyURI('1'))", "XPTY0004"),
    ] {
        let errors = run(source).unwrap_err();
        let name = errors[0].error_name().unwrap();
        assert_eq!(name.namespace_uri, "http://www.w3.org/2005/xqt-errors");
        assert_eq!(name.local_name, code, "{source}: {errors:?}");
        assert!(errors[0].source_map.is_some());
    }
}
