//! XPATH-DEMO-MAPS-ARRAYS: native sequence identity, types and limits.
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
        source_uri: "memory:containers.xml",
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
            source_uri: "memory:containers.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    let hash = cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes());
    let artifact = artifact::XPathCompiledArtifact::compile(&expression, hash.clone()).unwrap();
    let expression = artifact
        .reload(&artifact::XPathArtifactLoadContext {
            expected_source_hash: hash,
            invocation_host: XPathInvocationHost::StandaloneTransform,
        })
        .unwrap();
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
        safety_policy_stamp: "container-test",
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
fn square_and_curly_arrays_preserve_member_sequences_and_nesting() {
    assert_eq!(strings("(array:size([]), array:size([()]), array:size([(1,2), (), [3]]), array:size(array { (1,2), (), [3] }))"), ["0", "1", "3", "3"]);
    assert_eq!(strings("[(1,2), (), [3]]?1"), ["1", "2"]);
    assert!(strings("[1,()]?2").is_empty());
    assert_eq!(strings("[[1,2],[3,4]]?2?1"), ["3"]);
    assert_eq!(strings("array:get([(1,2)], 1)"), ["1", "2"]);
    assert_eq!(strings("array:size(array {})"), ["0"]);
    assert_eq!(strings("array:size([[], []])"), ["2"]);
}
#[test]
fn maps_distinguish_absence_empty_values_and_atomic_key_types() {
    let map = "map { 'empty': (), 'text': '', 1: 'number', '1': 'string', true(): 'bool' }";
    assert_eq!(strings(&format!("let $m := {map} return (map:contains($m,'empty'), map:contains($m,'absent'), empty(map:get($m,'empty')), empty(map:get($m,'absent')), $m?text, $m?1, $m?('1'), $m(true()), count(map:keys($m)))")), ["true", "false", "true", "true", "", "number", "string", "bool", "5"]);
    assert_eq!(
        strings("map:contains(map { xs:untypedAtomic('1'): 'x' }, 1)"),
        ["false"]
    );
    assert_eq!(strings("map:get(map { xs:anyURI('a'): 2 }, 'a')"), ["2"]);
    assert!(strings("map:keys(map {})").is_empty());
}
#[test]
fn key_equality_is_exact_transitive_and_handles_nan_infinities_and_zero() {
    for (a, b, same) in [
        ("1", "1.0", true),
        ("0", "-0e0", true),
        ("xs:double('NaN')", "xs:float('NaN')", true),
        ("xs:double('INF')", "xs:float('INF')", true),
        ("xs:double('-INF')", "xs:double('INF')", false),
        ("0.1", "0.1e0", false),
        ("xs:float('0.1')", "0.1e0", false),
        ("0.5", "0.5e0", true),
        ("9007199254740993", "9007199254740992e0", false),
        ("xs:untypedAtomic('a')", "xs:anyURI('a')", true),
    ] {
        assert_eq!(
            strings(&format!("map:contains(map {{ {a}: 1 }}, {b})")),
            [same.to_string()]
        );
        let source = format!("map {{ {a}: 1, {b}: 2 }}");
        let result = eval(&source, None, Default::default());
        if same {
            assert!(
                result.unwrap_err()[0].message.contains("XQDY0137"),
                "{source}"
            );
        } else {
            assert!(result.is_ok(), "{source}: {result:?}");
        }
    }
}
#[test]
fn lookups_keep_key_syntax_outer_focus_and_member_order_after_reload() {
    assert_eq!(
        strings("(map {'name':'a'}, map {'name':'b'})?name"),
        ["a", "b"]
    );
    assert_eq!(strings("[1,2,3]?(3,1,3)"), ["3", "1", "3"]);
    assert_eq!(strings("[[1,2],(),[3,4]]?*?*"), ["1", "2", "3", "4"]);
    assert_eq!(strings("(map {'x': 1}, map {'x': 2})[?x = 2]?x"), ["2"]);
    assert_eq!(strings("(1,2) ! [10,20]?(.)"), ["10", "20"]);
    assert_eq!(strings("let $key := 'x' return map {'x': 7}?($key)"), ["7"]);
    assert!(strings("map {'x': 7}?()").is_empty());
    assert_eq!(strings("map {'x': 7}?x"), ["7"]);
    assert_eq!(strings("[7](1)"), ["7"]);
}
#[test]
fn invalid_operands_keys_and_one_based_indices_report_standard_errors() {
    for (source, code) in [
        ("map { (): 1 }", "XPTY0004"),
        ("map { (1,2): 1 }", "XPTY0004"),
        ("map { map {}: 1 }", "FOTY0013"),
        ("map:get([], 'x')", "XPTY0004"),
        ("map:contains(map{}, ())", "XPTY0004"),
        ("array:size(map{})", "XPTY0004"),
        ("array:get([1], 0)", "FOAY0001"),
        ("[1]?2", "FOAY0001"),
        ("[1]?(-1)", "FOAY0001"),
        ("array:get([1], 1.0)", "XPTY0004"),
        ("[1]?name", "XPTY0004"),
        ("3?x", "XPTY0004"),
        ("?x", "XPDY0002"),
        ("[1]()", "XPTY0004"),
        ("array:get([1], xs:untypedAtomic('bad'))", "FORG0001"),
        ("map { 'x': 1 }?(map {})", "FOTY0013"),
    ] {
        let errors = eval(source, None, Default::default()).unwrap_err();
        assert!(errors[0].message.contains(code), "{source}: {errors:?}");
        assert!(errors[0].source_map.is_some(), "{source}");
    }
    assert_eq!(strings("array:get([4], xs:untypedAtomic('1'))"), ["4"]);
}
#[test]
fn nested_containers_retain_xml_owner_and_source_frames() {
    let root = xml("<basket><apple>2</apple><pear>3</pear></basket>");
    let node = root.native_node().unwrap();
    let owner = node.owner().clone();
    let result = eval(
        "map { 'rows': [ /basket/apple, /basket/pear ] }?rows?2",
        Some(root),
        Default::default(),
    )
    .unwrap();
    let item = &result.sequence.items[0];
    assert!(Arc::ptr_eq(item.native_node().unwrap().owner(), &owner));
    assert_eq!(item.native_node().unwrap().string_value(), "3");
    let XPathResultItem::Node { source_map, .. } = item else {
        panic!()
    };
    assert!(!source_map.frames.is_empty());
}
#[test]
fn container_construction_lookup_and_empty_members_charge_shared_limits() {
    for (source, limits, code) in [
        (
            "[('ab','cd')]",
            XPathEvaluationLimits {
                max_text_bytes: Some(3),
                ..Default::default()
            },
            "text_byte_limit",
        ),
        (
            "map { 'abc': 'def' }",
            XPathEvaluationLimits {
                max_text_bytes: Some(5),
                ..Default::default()
            },
            "text_byte_limit",
        ),
        (
            "[1,2]?*",
            XPathEvaluationLimits {
                max_sequence_items: Some(1),
                ..Default::default()
            },
            "sequence_item_limit",
        ),
        (
            "let $a := array { for $i in 1 to 50 return [] } return ($a,$a,$a)",
            XPathEvaluationLimits {
                max_work_units: Some(50),
                ..Default::default()
            },
            "work_limit",
        ),
    ] {
        let errors = eval(source, None, limits).unwrap_err();
        assert!(errors[0].code.contains(code), "{source}: {errors:?}");
    }
}
