//! XPATH-DEMO-SEQUENCES: native sequence identity, types and limits.
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
        source_uri: "memory:sequences.xml",
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
            source_uri: "memory:sequences.xpath",
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
fn head_tail_reverse_preserve_empty_items_types_and_duplicates() {
    for source in ["head(())", "tail(())", "reverse(())", "tail('x')"] {
        assert!(values(source).is_empty(), "{source}");
    }
    assert_eq!(
        values("head(('', 1, true()))"),
        [("xs:string".into(), "".into())]
    );
    assert_eq!(
        values("tail(('x', 1, true()))"),
        [
            ("xs:integer".into(), "1".into()),
            ("xs:boolean".into(), "true".into())
        ]
    );
    assert_eq!(strings("reverse(('a', 'b', 'a', ''))"), ["", "a", "b", "a"]);
}
#[test]
fn subsequence_follows_one_based_rounded_double_bounds() {
    for (bounds, expected) in [
        ("2", vec!["b", "c", "d"]),
        ("2, 2", vec!["b", "c"]),
        ("1.5, 2.5", vec!["b", "c", "d"]),
        ("0, 3", vec!["a", "b"]),
        ("-0.5, 2", vec!["a"]),
        ("-2, 4", vec!["a"]),
        ("1, -1", vec![]),
        ("1, 0", vec![]),
        ("1e300", vec![]),
        ("xs:double('NaN')", vec![]),
        ("xs:double('INF')", vec![]),
        ("xs:double('-INF')", vec!["a", "b", "c", "d"]),
        ("xs:double('-INF'), xs:double('INF')", vec![]),
        ("1, xs:double('NaN')", vec![]),
        ("2, xs:double('INF')", vec!["b", "c", "d"]),
        ("0.49999999999999994e0, 1", vec![]),
        ("xs:untypedAtomic('2'), xs:float('2')", vec!["b", "c"]),
    ] {
        assert_eq!(
            strings(&format!("subsequence(('a','b','c','d'), {bounds})")),
            expected,
            "{bounds}"
        );
    }
    assert!(values("subsequence((), 2, 4)").is_empty());
}
#[test]
fn distinct_values_uses_atomic_equality_without_conflating_incomparable_types() {
    assert!(values("distinct-values(())").is_empty());
    assert_eq!(
        strings("distinct-values(('a', 'a', 'A', xs:anyURI('a'), xs:untypedAtomic('a')))"),
        ["a", "A"]
    );
    assert_eq!(
        values("distinct-values((1, xs:decimal('1.00'), '1', true(), false()))").len(),
        4
    );
    assert_eq!(
        strings("distinct-values((12345678901234567890,12345678901234567891))"),
        ["12345678901234567890", "12345678901234567891"]
    );
    assert_eq!(
        strings("distinct-values((xs:float('NaN'),xs:double('NaN'),0e0,-0e0,0))"),
        ["NaN", "0e0"]
    );
    assert_eq!(strings("distinct-values(('é','é','é'), 'http://www.w3.org/2005/xpath-functions/collation/codepoint')"), ["é", "é"]);
    assert_eq!(
        values("distinct-values((xs:untypedAtomic('a'), 'a'))")[0].0,
        "xs:untypedAtomic"
    );
    // The standard permits different representatives under non-transitive promotion.
    // Verify both constraints for every permutation, rather than requiring one order.
    let numbers = [
        "xs:float('1')",
        "xs:decimal('1.0000000000100000000001')",
        "xs:double('1.00000000001')",
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let sequence = order.map(|i| numbers[i]).join(",");
        let source = format!("let $input := ({sequence}), $result := distinct-values($input) return (every $v in $input satisfies $v = $result) and (every $i in 1 to count($result) satisfies not($result[$i] = subsequence($result, $i + 1)))");
        assert_eq!(strings(&source), ["true"], "{source}");
    }
}
#[test]
fn bad_bounds_collations_and_unsupported_atomic_types_fail_at_the_argument() {
    for argument in ["'2'", "true()", "xs:anyURI('2')", "()", "(1,2)"] {
        let source = format!("subsequence(('a','b'), {argument})");
        let errors = eval(&source, None, Default::default()).unwrap_err();
        assert!(errors[0].message.contains("XPTY0004"), "{errors:?}");
        assert_eq!(
            errors[0].byte_offset,
            Some(source.find(argument).unwrap() as u64)
        );
        assert!(errors[0].source_map.is_some());
    }
    let errors = eval(
        "subsequence((), xs:untypedAtomic('bad'))",
        None,
        Default::default(),
    )
    .unwrap_err();
    assert!(errors[0].message.contains("FORG0001"), "{errors:?}");
    let errors = eval(
        "distinct-values((), 'urn:unknown')",
        None,
        Default::default(),
    )
    .unwrap_err();
    assert!(errors[0].message.contains("FOCH0002"), "{errors:?}");
    assert_eq!(errors[0].byte_offset, Some(20));
    for source in [
        "head()",
        "tail((), ())",
        "reverse(1, 2)",
        "subsequence(())",
        "distinct-values((), 1)",
    ] {
        assert!(eval(source, None, Default::default()).is_err(), "{source}");
    }
    let unsupported = XPathResultItem::Atomic {
        value: XPathAtomicValue {
            type_name: "xs:date".into(),
            lexical_value: "2026-09-17".into(),
            namespace_uri: None,
            local_name: None,
        },
        source_map: Default::default(),
    };
    assert!(eval("distinct-values(.)", Some(unsupported), Default::default()).is_err());
}
#[test]
fn native_sequences_retain_node_identity_maps_arrays_functions_and_source_maps() {
    let document = xml("<r><v>a</v><v>b</v><v>a</v></r>");
    let original = eval("/r/v", Some(document.clone()), Default::default())
        .unwrap()
        .sequence
        .items;
    let reversed = eval("reverse(/r/v)", Some(document.clone()), Default::default())
        .unwrap()
        .sequence
        .items;
    for (a, b) in original.iter().zip(reversed.iter().rev()) {
        assert_eq!(a.native_node(), b.native_node());
        assert!(Arc::ptr_eq(
            a.native_node().unwrap().owner(),
            b.native_node().unwrap().owner()
        ));
        if let (
            XPathResultItem::Node { source_map: a, .. },
            XPathResultItem::Node { source_map: b, .. },
        ) = (a, b)
        {
            assert_eq!(a, b);
        }
    }
    let result = eval(
        "head(tail(reverse(/r/v)))",
        Some(document.clone()),
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        result.sequence.items[0].native_node(),
        original[1].native_node()
    );
    let result = eval("distinct-values(/r/v)", Some(document), Default::default()).unwrap();
    assert_eq!(result.sequence.items.len(), 2);
    let origin = eval("1", None, Default::default()).unwrap().source_map;
    let array = XPathResultItem::Array {
        members: vec![XPathResultSequence {
            sequence_type: "node()*".into(),
            items: original.clone(),
        }],
        source_map: origin.clone(),
    };
    let map = XPathResultItem::Map {
        entries: vec![],
        source_map: origin.clone(),
    };
    let function = XPathResultItem::Function {
        evaluator_id: "cem.xpath.native".into(),
        function_id: "f".into(),
        name: None,
        arity: 0,
        signature: "function() as item()*".into(),
        native_function: None,
        source_map: origin.clone(),
    };
    for item in [array.clone(), map, function] {
        for expression in [
            "head((., 1))",
            "tail((1, .))",
            "reverse(.)",
            "subsequence((1, .), 2, 1)",
        ] {
            let result = eval(expression, Some(item.clone()), Default::default()).unwrap();
            assert_eq!(result.sequence.items.len(), 1);
            assert_eq!(result.sequence.items[0], item);
        }
    }
    assert_eq!(
        eval("distinct-values(.)", Some(array), Default::default())
            .unwrap()
            .sequence
            .items
            .len(),
        2
    );
}
#[test]
fn sequence_intermediates_comparisons_and_atomization_share_limits() {
    for (source, limits, code) in [
        (
            "head(('aaa', 'bbb'))",
            XPathEvaluationLimits {
                max_text_bytes: Some(5),
                ..Default::default()
            },
            "cem.xpath.text_byte_limit_exceeded",
        ),
        (
            "reverse((1,2,3))",
            XPathEvaluationLimits {
                max_sequence_items: Some(2),
                ..Default::default()
            },
            "cem.xpath.sequence_item_limit_exceeded",
        ),
        (
            "distinct-values(1 to 50)",
            XPathEvaluationLimits {
                max_sequence_items: Some(100),
                max_work_units: Some(1000),
                ..Default::default()
            },
            "cem.xpath.work_limit_exceeded",
        ),
    ] {
        let errors = eval(source, None, limits).unwrap_err();
        assert_eq!(errors[0].code, code, "{errors:?}");
        assert!(errors[0].source_map.is_some());
    }
    let errors = eval(
        "distinct-values(/r/v)",
        Some(xml("<r><v>abcdef</v></r>")),
        XPathEvaluationLimits {
            max_text_bytes: Some(5),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "cem.xpath.text_byte_limit_exceeded");
}
