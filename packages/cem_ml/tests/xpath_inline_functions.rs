//! XPATH-SORT-FUNCTIONS: closures, typed calls and retained native owners.
use cem_ml::{
    diagnostics::Diagnostic,
    lifecycle::LoadedInputAstStream,
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::{
        xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
        xpath::*,
    },
};
use std::sync::Arc;

fn xml(source: &str) -> XPathResultItem {
    let (document, errors) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: "memory:functions.xml",
        content_type: Some("application/xml"),
    });
    assert!(errors.is_empty(), "{errors:?}");
    XPathResultItem::from_native_node(
        XPathNativeNode::xml_document(Arc::new(LoadedInputAstStream::XmlDocument(
            document.unwrap(),
        )))
        .unwrap(),
    )
}
fn eval(
    source: &str,
    item: Option<XPathResultItem>,
    limits: XPathEvaluationLimits,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:functions.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    evaluate(&expression, item, limits)
}
fn evaluate(
    expression: &XPathExpressionAst,
    item: Option<XPathResultItem>,
    limits: XPathEvaluationLimits,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    CemXPathEvaluator::default().evaluate(XPathEvaluationRequest {
        invocation_host: XPathInvocationHost::StandaloneTransform,
        expression,
        dynamic_context: XPathDynamicContext {
            context_item: item,
            ..Default::default()
        },
        static_context: XPathStaticContext::default(),
        expected_result: None,
        resolver_registry: &ResolverRegistry::new(),
        resolver_policy: &ResolverPolicy::new(),
        evaluation_limits: XPathEvaluationLimits {
            max_sequence_items: limits.max_sequence_items.or(Some(10000)),
            ..limits
        },
        safety_policy_stamp: "sequence-test",
        module_resolution: None,
    })
}
fn values(source: &str) -> Vec<(String, String)> {
    eval(source, None, Default::default())
        .unwrap_or_else(|e| panic!("{source}: {e:?}"))
        .sequence
        .items
        .into_iter()
        .map(|item| match item {
            XPathResultItem::Atomic { value, .. } => (value.type_name, value.lexical_value),
            other => panic!("{other:?}"),
        })
        .collect()
}
fn strings(source: &str) -> Vec<String> {
    values(source).into_iter().map(|(_, v)| v).collect()
}
#[test]
fn inline_functions_capture_lexical_values_and_shadow_parameters() {
    assert_eq!(strings("(function($x) { $x + 1 })(2)"), ["3"]);
    assert_eq!(strings("(function($Q{}x) { $x })(3)"), ["3"]);
    assert_eq!(
        strings("let $x := 7, $f := function($y) { $x + $y }, $x := 100 return $f(2)"),
        ["9"]
    );
    assert_eq!(
        strings("let $x := 7 return (function($x) { $x })(2)"),
        ["2"]
    );
    assert_eq!(
        strings("let $make := function($x) { function($y) { $x + $y } } return $make(5)(6)"),
        ["11"]
    );
    assert_eq!(strings("(function() { (1, 2) })()"), ["1", "2"]);
    assert!(strings("(function() {})()").is_empty());
    assert_eq!(strings("3 => (function($x) { $x * 2 })()"), ["6"]);
    assert_eq!(strings("(map {'f': function($x) {$x}})?f('yes')"), ["yes"]);
}
#[test]
fn typed_functions_apply_function_conversion_and_result_checks() {
    assert_eq!(
        values("(function($x as xs:double) as xs:double { $x })(2)")[0].0,
        "xs:double"
    );
    assert_eq!(
        strings("(function($x as xs:integer) as xs:integer { $x })(xs:untypedAtomic('12'))"),
        ["12"]
    );
    assert_eq!(
        values("(function() as xs:string { xs:anyURI('a') })()")[0].0,
        "xs:string"
    );
    assert_eq!(
        strings("(function($x as item()*) as item()* { $x })((1, 2))"),
        ["1", "2"]
    );
    for source in [
        "(function($x as xs:integer) {$x})('12')",
        "(function($x as xs:integer) {$x})(())",
        "(function() as xs:integer {'bad'})()",
        "(function($x) {$x})()",
        "(function($x) {$x})(1, 2)",
    ] {
        let errors = eval(source, None, Default::default()).unwrap_err();
        assert!(
            errors[0].message.contains("XPTY0004"),
            "{source}: {errors:?}"
        );
        assert!(errors[0].source_map.is_some());
    }
}
#[test]
fn inline_body_has_no_implicit_focus_and_duplicate_parameters_fail() {
    for source in [
        "(function() { . })()",
        "(function() { position() })()",
        "(function() { last() })()",
    ] {
        let errors = eval(source, Some(xml("<r/>")), Default::default()).unwrap_err();
        assert_eq!(
            errors[0].code, "cem.xpath.context_item_missing",
            "{source}: {errors:?}"
        );
    }
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: b"if (true()) then 1 else function($x, $x) {$x}",
            source_uri: "memory:duplicate.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    assert!(expression.syntax_ast.is_none());
    assert!(expression
        .facts
        .iter()
        .any(|fact| fact.message.contains("XQST0039")));
}
#[test]
fn duplicate_parameters_compare_expanded_names_including_the_empty_namespace() {
    let source = "function($x, $Q{}x) {$x}";
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:duplicate.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    assert!(expression.syntax_ast.is_none());
    assert!(expression
        .facts
        .iter()
        .any(|fact| fact.message.contains("XQST0039")));
}

#[test]
fn captured_nodes_keep_native_identity_and_source_maps() {
    let document = xml("<r><v>a</v><v>b</v></r>");
    let expected = eval("/r/v", Some(document.clone()), Default::default()).unwrap();
    let result = eval(
        "let $nodes := /r/v, $f := function() {$nodes} return $f()",
        Some(document),
        Default::default(),
    )
    .unwrap();
    assert_eq!(result.sequence, expected.sequence);
    for (actual, expected) in result.sequence.items.iter().zip(&expected.sequence.items) {
        assert!(Arc::ptr_eq(
            actual.native_node().unwrap().owner(),
            expected.native_node().unwrap().owner()
        ));
    }
}
#[test]
fn calls_and_captures_obey_work_text_and_recursion_limits() {
    let errors = eval(
        "(function($x) {$x + $x})(5)",
        None,
        XPathEvaluationLimits {
            max_work_units: Some(3),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.work_limit_exceeded");
    let errors = eval(
        "let $f := function($self) {$self($self)} return $f($f)",
        None,
        Default::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.function_depth_exceeded");
}

#[test]
fn portable_closure_syntax_rebinds_changed_owners_without_serializing_captures() {
    use artifact::{XPathArtifactLoadContext, XPathCompiledArtifact};
    use cem_ml::content_cache::ContentHash;
    let source =
        "let $nodes := /r/v return (function($suffix as xs:string) { ($nodes, $suffix) })('!')";
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:portable.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 8 },
    );
    let source_hash = ContentHash::from_blake3(source.as_bytes());
    let artifact = XPathCompiledArtifact::compile(&expression, source_hash.clone()).unwrap();
    let loaded =
        XPathCompiledArtifact::from_bytes(artifact.bytes().to_vec(), artifact.content_hash())
            .unwrap()
            .reload(&XPathArtifactLoadContext {
                expected_source_hash: source_hash,
                invocation_host: XPathInvocationHost::StandaloneTransform,
            })
            .unwrap();
    assert!(loaded.source_text.is_none());
    assert!(loaded.tokens.is_empty());
    for text in ["<r><v>a</v></r>", "<r><v>b</v><v>c</v></r>"] {
        let document = xml(text);
        assert_eq!(
            evaluate(&loaded, Some(document.clone()), Default::default())
                .unwrap()
                .sequence,
            evaluate(&expression, Some(document), Default::default())
                .unwrap()
                .sequence
        );
    }
}

#[test]
fn escaped_native_closure_keeps_its_owner_but_exported_metadata_is_not_executable() {
    let closure = eval(
        "let $nodes := /r/v return function() {$nodes}",
        Some(xml("<r><v>kept</v></r>")),
        Default::default(),
    )
    .unwrap()
    .sequence
    .items
    .remove(0);
    let result = eval("(.)()", Some(closure.clone()), Default::default()).unwrap();
    assert_eq!(
        result.sequence.items[0]
            .native_node()
            .unwrap()
            .string_value(),
        "kept"
    );
    // This explicitly tests the result export boundary; it is not an AST handoff.
    let exported = serde_json::to_vec(&closure).unwrap();
    let metadata = serde_json::from_slice(&exported).unwrap();
    let errors = eval("(.)()", Some(metadata), Default::default()).unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.native_function_missing");
    let mut definition = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: b"function() {1 idiv 0}",
            source_uri: "memory:definition.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 99 },
    );
    definition.source_text = None;
    let closure = evaluate(&definition, None, Default::default())
        .unwrap()
        .sequence
        .items
        .remove(0);
    let errors = eval("(.)()", Some(closure), Default::default()).unwrap_err();
    assert_eq!(errors[0].uri.as_deref(), Some("memory:definition.xpath"));
    assert_eq!(
        errors[0].source_map.as_ref().unwrap().frames[0].source_id.0,
        99
    );
    assert_eq!(errors[0].byte_offset, Some(19));
    let definition = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: b"function() as xs:integer {'bad'}",
            source_uri: "memory:result-type.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 100 },
    );
    let closure = evaluate(&definition, None, Default::default())
        .unwrap()
        .sequence
        .items
        .remove(0);
    let errors = eval("(.)()", Some(closure), Default::default()).unwrap_err();
    assert_eq!(errors[0].uri.as_deref(), Some("memory:result-type.xpath"));
    assert_eq!(errors[0].byte_offset, Some(14));
}

#[test]
fn nested_expression_frames_cannot_bypass_the_inline_recursion_guard() {
    let source = format!(
        "let $f := function($self) {{ {}$self($self){} }} return $f($f)",
        "(".repeat(20),
        ")".repeat(20)
    );
    let errors = eval(&source, None, Default::default()).unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.function_depth_exceeded");
}
