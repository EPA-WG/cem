//! XPATH-SORT-NATIVE: stable typed ordering over retained native items.
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
    CemXPathEvaluator::default().evaluate(XPathEvaluationRequest {
        invocation_host: XPathInvocationHost::StandaloneTransform,
        expression: &expression,
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
fn sort_orders_typed_keys_stably_and_preserves_the_input_items() {
    assert_eq!(strings("sort((3, 1, 2, 1))"), ["1", "1", "2", "3"]);
    assert_eq!(
        strings("sort(('10', '2', 'a', 'A'))"),
        ["10", "2", "A", "a"]
    );
    assert_eq!(
        strings("sort((true(), false(), false()))"),
        ["false", "false", "true"]
    );
    assert_eq!(
        strings("sort((1, -2, 5, 10, -10, 10, 8), (), function($x) {abs($x)})"),
        ["1", "-2", "5", "8", "10", "-10", "10"]
    );
    assert_eq!(
        strings("sort((3, 1, 2), (), function($x) {()})"),
        ["3", "1", "2"]
    );
    assert_eq!(
        strings("sort((xs:double('NaN'), 2, xs:float('NaN'), -1))"),
        ["NaN", "NaN", "-1", "2"]
    );
    assert_eq!(
        strings("sort((12345678901234567891,12345678901234567890))"),
        ["12345678901234567890", "12345678901234567891"]
    );
    assert!(strings("sort(())").is_empty());
    assert_eq!(
        strings("sort(('a','b'), (), map {'a': 2, 'b': 1})"),
        ["b", "a"]
    );
    assert_eq!(strings("sort((1,2), (), [2,1])"), ["2", "1"]);
    assert_eq!(
        strings("sort((xs:untypedAtomic('2'),xs:untypedAtomic('10')))"),
        ["10", "2"]
    );
}
#[test]
fn sort_uses_lexicographic_multiple_keys_and_authored_invalid_last_policy() {
    let source = "let $rows := ([1,2,'a'], [1,1,'b'], [1,1,'c'], [0,3,'d']) return sort($rows, (), function($r) {($r?1,$r?2)})?3";
    assert_eq!(strings(source), ["d", "b", "c", "a"]);
    assert_eq!(
        strings("sort(([1,2,'a'], [1,1,'b']), (), function($r) {($r?1, -$r?2)})?3"),
        ["a", "b"]
    );
    for (sign, expected) in [
        ("", vec!["2", "10", "bad", "empty"]),
        ("-", vec!["10", "2", "bad", "empty"]),
    ] {
        let expression = format!("sort(('bad', '10', '2', 'empty'), (), function($v) {{ let $ok := $v castable as xs:decimal return (not($ok), if ($ok) then {sign}xs:decimal($v) else 0) }})");
        assert_eq!(strings(&expression), expected);
    }
    assert_eq!(
        strings("sort(([1,2], [1], [])) ! string-join(.?* ! string(.), '-')"),
        ["", "1", "1-2"]
    );
}
#[test]
fn sorted_native_nodes_preserve_owners_identity_and_source_maps() {
    let document = xml("<r><v>10</v><v>2</v><v>2</v></r>");
    let original = eval("/r/v", Some(document.clone()), Default::default())
        .unwrap()
        .sequence
        .items;
    let sorted = eval(
        "sort(/r/v, (), function($v) {xs:integer($v)})",
        Some(document),
        Default::default(),
    )
    .unwrap()
    .sequence
    .items;
    for (actual, index) in sorted.iter().zip([1, 2, 0]) {
        assert_eq!(actual, &original[index]);
        assert!(Arc::ptr_eq(
            actual.native_node().unwrap().owner(),
            original[index].native_node().unwrap().owner()
        ));
    }
}
#[test]
fn sort_rejects_invalid_keys_collations_and_arity_even_for_empty_inputs() {
    for source in [
        "sort((1,'a'))",
        "sort((true(),1))",
        "sort((), (), 1)",
        "sort((), (), function() {1})",
        "sort(1, (), function($x) {map{}})",
        "sort(1, (1,2))",
    ] {
        let errors = eval(source, None, Default::default()).unwrap_err();
        assert!(
            errors[0].message.contains("XPTY0004") || errors[0].message.contains("FOTY0013"),
            "{source}: {errors:?}"
        );
        assert!(errors[0].source_map.is_some());
    }
    let errors = eval("sort((), 'urn:unknown')", None, Default::default()).unwrap_err();
    assert!(errors[0].message.contains("FOCH0002"), "{errors:?}");
    assert_eq!(
        strings("sort(('b', 'a'), 'http://www.w3.org/2005/xpath-functions/collation/codepoint')"),
        ["a", "b"]
    );
}
#[test]
fn key_work_and_intermediate_storage_share_evaluation_limits() {
    let errors = eval(
        "sort((3,2,1), (), function($x) {1 to 100})",
        None,
        XPathEvaluationLimits {
            max_sequence_items: Some(50),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(errors[0].code.contains("limit"), "{errors:?}");
    let errors = eval(
        "sort(1 to 100, (), function($x) {-$x})",
        None,
        XPathEvaluationLimits {
            max_work_units: Some(150),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.work_limit_exceeded");
}
