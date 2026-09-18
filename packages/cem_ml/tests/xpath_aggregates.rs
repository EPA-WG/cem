//! XPATH-DEMO-AGGREGATES: numeric promotion, exact decimals and bounded work.
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
        source_uri: "memory:aggregates.xml",
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
            source_uri: "memory:aggregates.xpath",
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
        safety_policy_stamp: "aggregate-test",
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
fn empty_aggregates_and_explicit_sum_zero_follow_their_signatures() {
    assert_eq!(values("sum(())"), [("xs:integer".into(), "0".into())]);
    for source in ["avg(())", "min(())", "max(())", "sum((), ())"] {
        assert!(values(source).is_empty(), "{source}");
    }
    for source in ["sum((), 'empty')", "sum((), xs:untypedAtomic('empty'))"] {
        assert_eq!(strings(source), ["empty"]);
    }
    assert_eq!(strings("sum((1,2), 'unused')"), ["3"]);
    assert_eq!(values("sum((), xs:decimal('0'))")[0].0, "xs:decimal");
    assert_eq!(strings("min((1,2), 'urn:ignored-for-numerics')"), ["1"]);
}

#[test]
fn exact_aggregates_preserve_integers_decimals_and_average_precision() {
    assert_eq!(
        strings("(0.1 gt 0, 0 lt 0.1, 0.01 ge 0, -0.1 lt 0, 0 gt -0.1, 0 eq 0.00)"),
        vec!["true"; 6]
    );
    for (source, ty, value) in [
        (
            "sum((12345678901234567890, 1))",
            "xs:integer",
            "12345678901234567891",
        ),
        ("sum((0.1, 0.2))", "xs:decimal", "0.3"),
        ("avg((0.1, 0.2))", "xs:decimal", "0.15"),
        ("avg((1,2))", "xs:decimal", "1.5"),
        ("avg((0,0,1))", "xs:decimal", "0.333333333333333333"),
        ("min((1,2.5))", "xs:integer", "1"),
        ("max((1,2.5))", "xs:decimal", "2.5"),
        (
            "min((12345678901234567890,12345678901234567891))",
            "xs:integer",
            "12345678901234567890",
        ),
        ("sum((-2, 2, 3))", "xs:integer", "3"),
        ("min((0.1, 0))", "xs:integer", "0"),
        ("max((0, 0.1))", "xs:decimal", "0.1"),
        ("min((-0.1, 0))", "xs:decimal", "-0.1"),
        ("max((-0.1, 0))", "xs:integer", "0"),
    ] {
        assert_eq!(values(source), [(ty.into(), value.into())], "{source}");
        let result = eval(source, None, Default::default()).unwrap();
        let XPathResultItem::Atomic { source_map, .. } = &result.sequence.items[0] else {
            panic!()
        };
        assert!(!source_map.frames.is_empty());
    }
}

#[test]
fn all_values_are_promoted_before_reduction_and_nan_does_not_hide_bad_types() {
    for name in ["sum", "avg", "min", "max"] {
        assert_eq!(
            values(&format!("{name}((xs:float('1'), xs:double('2')))"))[0].0,
            "xs:double"
        );
        assert_eq!(
            values(&format!("{name}((1, xs:untypedAtomic('2')))"))[0].0,
            "xs:double"
        );
        assert_eq!(
            values(&format!("{name}((xs:float('NaN'), xs:double('1')))")),
            [("xs:double".into(), "NaN".into())]
        );
        assert!(eval(
            &format!("{name}((xs:double('NaN'), 'bad'))"),
            None,
            Default::default()
        )
        .is_err());
    }
    assert_eq!(
        strings("sum((xs:float('3e38'), xs:float('3e38'), xs:double('0'))) lt xs:double('INF')"),
        ["true"]
    );
    assert_eq!(
        strings("min((16777217, xs:float('16777216'), xs:double('16777218')))"),
        ["1.6777216E7"]
    );
    assert_eq!(
        strings("max((xs:double('-INF'), xs:double('INF')))"),
        ["INF"]
    );
    assert_eq!(strings("avg((xs:float('INF'), xs:float('-INF')))"), ["NaN"]);
    assert_eq!(strings("min((xs:float('-0'), xs:float('0')))"), ["-0"]);
}

#[test]
fn invalid_and_unsupported_types_keep_argument_errors_and_ranges() {
    for name in ["sum", "avg", "min", "max"] {
        let source = format!("{name}((1, true()))");
        let errors = eval(&source, None, Default::default()).unwrap_err();
        assert!(errors[0].message.contains("FORG0006"), "{errors:?}");
        assert_eq!(errors[0].byte_offset, Some((name.len() + 1) as u64));
        assert!(errors[0].source_map.is_some());
        let source = format!("{name}(xs:untypedAtomic('bad'))");
        let errors = eval(&source, None, Default::default()).unwrap_err();
        assert!(errors[0].message.contains("FORG0001"), "{errors:?}");
    }
    for source in ["sum('2')", "avg(true())"] {
        assert!(eval(source, None, Default::default()).unwrap_err()[0]
            .message
            .contains("FORG0006"));
    }
    for source in ["min(('a','b'))", "max((false(),true()))"] {
        assert_eq!(
            eval(source, None, Default::default()).unwrap_err()[0].code,
            "cem.xpath.aggregate_type_unsupported"
        );
    }
    for source in [
        "sum((), (1,2))",
        "sum(1, (1,2))",
        "min((), ())",
        "max(1, 2)",
    ] {
        assert!(
            eval(source, None, Default::default()).unwrap_err()[0]
                .message
                .contains("XPTY0004"),
            "{source}"
        );
    }
    for source in ["sum()", "avg(1,2)", "min()", "max(1,2,3)"] {
        assert!(eval(source, None, Default::default()).is_err());
    }
}

#[test]
fn native_xml_and_arrays_atomize_without_replacing_owners() {
    let document = xml("<r><v>0.1</v><v>0.2</v></r>");
    let nodes = eval("/r/v", Some(document.clone()), Default::default()).unwrap();
    let array = XPathResultItem::Array {
        members: vec![nodes.sequence],
        source_map: nodes.source_map,
    };
    let result = eval("sum(.)", Some(array), Default::default()).unwrap();
    assert_eq!(result.sequence.items[0].kind(), XPathResultItemKind::Atomic);
    let exact = eval(
        "sum(/r/v ! xs:decimal(.))",
        Some(document),
        Default::default(),
    )
    .unwrap();
    let XPathResultItem::Atomic { value, .. } = &exact.sequence.items[0] else {
        panic!()
    };
    assert_eq!(
        (&*value.type_name, &*value.lexical_value),
        ("xs:decimal", "0.3")
    );
    let errors = eval(
        "sum(/r/v)",
        Some(xml("<r><v>oops</v></r>")),
        Default::default(),
    )
    .unwrap_err();
    assert!(errors[0].message.contains("FORG0001"));
}

#[test]
fn aggregate_inputs_intermediates_output_and_arithmetic_obey_limits() {
    let reduction_limit = XPathEvaluationLimits {
        max_work_units: Some(1000),
        ..Default::default()
    };
    assert!(eval("1 to 50", None, reduction_limit.clone()).is_ok());
    assert_eq!(
        eval("avg(1 to 50)", None, reduction_limit).unwrap_err()[0].code,
        "cem.xpath.work_limit_exceeded"
    );
    for (source, limits, code) in [
        (
            "sum((1,2,3))",
            XPathEvaluationLimits {
                max_sequence_items: Some(2),
                ..Default::default()
            },
            "cem.xpath.sequence_item_limit_exceeded",
        ),
        (
            "sum((999,999,999,999,999,999,999,999,999,999,999))",
            XPathEvaluationLimits {
                max_work_units: Some(50),
                ..Default::default()
            },
            "cem.xpath.work_limit_exceeded",
        ),
        (
            "avg((1,0,0))",
            XPathEvaluationLimits {
                max_text_bytes: Some(10),
                ..Default::default()
            },
            "cem.xpath.text_byte_limit_exceeded",
        ),
        (
            "sum(('aaaa', 'bbbb'))",
            XPathEvaluationLimits {
                max_text_bytes: Some(7),
                ..Default::default()
            },
            "cem.xpath.text_byte_limit_exceeded",
        ),
    ] {
        let errors = eval(source, None, limits).unwrap_err();
        assert_eq!(errors[0].code, code, "{source}: {errors:?}");
    }
}
