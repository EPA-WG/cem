//! XPATH-DEMO-NODES: access native XML names without atomization or projection.
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
        source_uri: "memory:nodes.xml",
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
            source_uri: "memory:nodes.xpath",
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
        evaluation_limits: limits,
        safety_policy_stamp: "nodes-test",
        module_resolution: None,
    })
}
fn value(source: &str, item: Option<XPathResultItem>) -> (String, String) {
    let result =
        eval(source, item, Default::default()).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    match &result.sequence.items[..] {
        [XPathResultItem::Atomic { value, .. }] => {
            (value.type_name.clone(), value.lexical_value.clone())
        }
        other => panic!("{other:?}"),
    }
}
#[test]
fn native_names_cover_namespaces_attributes_text_and_unnamed_nodes() {
    let document = xml("<?go hello?><r xmlns='urn:a' xmlns:p='urn:b' a='1' p:a='2' xml:lang='en'>a<![CDATA[b]]>&amp;<p:item/><item xmlns='urn:c'/><!--note--></r>");
    for (path, name, namespace) in [
        (".", "", ""),
        ("/*", "r", "urn:a"),
        ("/*/*[1]", "item", "urn:b"),
        ("/*/*[2]", "item", "urn:c"),
        ("/*/@a", "a", ""),
        ("/*/@Q{urn:b}a", "a", "urn:b"),
        (
            "/*/@xml:lang",
            "lang",
            "http://www.w3.org/XML/1998/namespace",
        ),
        ("/*/text()", "", ""),
        ("/*/comment()", "", ""),
        ("/processing-instruction()", "go", ""),
        ("()", "", ""),
    ] {
        for (function, ty, expected) in [
            ("local-name", "xs:string", name),
            ("namespace-uri", "xs:anyURI", namespace),
        ] {
            assert_eq!(
                value(&format!("{function}({path})"), Some(document.clone())),
                (ty.into(), expected.into())
            );
            if path != "()" {
                let node = eval(path, Some(document.clone()), Default::default())
                    .unwrap()
                    .sequence
                    .items
                    .remove(0);
                assert_eq!(
                    value(&format!("fn:{function}()"), Some(node)),
                    (ty.into(), expected.into())
                );
            }
        }
    }
    assert_eq!(value("string(/*/text())", Some(document.clone())).1, "ab&");
    assert_eq!(value("data(/*/@a)", Some(document)).0, "xs:untypedAtomic");
}
#[test]
fn optional_nodes_reject_atomic_values_and_multiple_nodes_at_argument_range() {
    for function in ["local-name", "namespace-uri"] {
        for arg in ["1", "'r'", "(/*, /*)"] {
            let errors = eval(
                &format!("{function}({arg})"),
                Some(xml("<r/>")),
                Default::default(),
            )
            .unwrap_err();
            assert!(errors[0].message.contains("XPTY0004"), "{errors:?}");
            assert_eq!(errors[0].byte_offset, Some((function.len() + 1) as u64));
            assert!(errors[0].source_map.is_some());
        }
        let errors = eval(&format!("{function}()"), None, Default::default()).unwrap_err();
        assert!(errors[0].message.contains("XPDY0002"), "{errors:?}");
        let atomic = eval("1", None, Default::default())
            .unwrap()
            .sequence
            .items
            .remove(0);
        let errors = eval(&format!("{function}()"), Some(atomic), Default::default()).unwrap_err();
        assert!(errors[0].message.contains("XPTY0004"), "{errors:?}");
        for item in [
            XPathResultItem::Map {
                entries: vec![],
                source_map: Default::default(),
            },
            XPathResultItem::Array {
                members: vec![],
                source_map: Default::default(),
            },
            XPathResultItem::Function {
                evaluator_id: "test".into(),
                function_id: "f".into(),
                name: None,
                arity: 0,
                signature: "function() as item()*".into(),
                source_map: Default::default(),
            },
        ] {
            let errors =
                eval(&format!("{function}(.)"), Some(item), Default::default()).unwrap_err();
            assert!(errors[0].message.contains("XPTY0004"), "{errors:?}");
        }
        let mut detached = xml("<r/>");
        if let XPathResultItem::Node { native_node, .. } = &mut detached {
            *native_node = None;
        }
        let errors = eval(
            &format!("{function}(.)"),
            Some(detached),
            Default::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "cem.xpath.native_node_missing");
        assert!(eval(&format!("{function}((), ())"), None, Default::default()).is_err());
    }
    assert!(eval("node-name(/*)", Some(xml("<r/>")), Default::default()).is_err());
}
#[test]
fn names_obey_text_and_work_limits_without_reading_descendant_text() {
    for source in ["local-name(/*)", "namespace-uri(/*)"] {
        let document = xml("<longname xmlns='urn:long'/> ");
        for (limits, code) in [
            (
                XPathEvaluationLimits {
                    max_text_bytes: Some(3),
                    ..Default::default()
                },
                "cem.xpath.text_byte_limit_exceeded",
            ),
            (
                XPathEvaluationLimits {
                    max_work_units: Some(1),
                    ..Default::default()
                },
                "cem.xpath.work_limit_exceeded",
            ),
        ] {
            let errors = eval(source, Some(document.clone()), limits).unwrap_err();
            assert_eq!(errors[0].code, code);
            assert_eq!(errors[0].uri.as_deref(), Some("memory:nodes.xpath"));
        }
    }
    assert!(eval(
        "local-name(/*)",
        Some(xml("<r>very long text</r>")),
        XPathEvaluationLimits {
            max_text_bytes: Some(1),
            ..Default::default()
        }
    )
    .is_ok());
}
#[test]
fn navigation_preserves_native_identity_and_owner_after_name_access() {
    let document = xml("<r><item/><item/></r>");
    let owner = document.native_node().unwrap().owner().clone();
    let result = eval(
        "/*/*[local-name() = 'item'][2]/preceding-sibling::*",
        Some(document.clone()),
        Default::default(),
    )
    .unwrap();
    let expected = eval("/*/*[1]", Some(document.clone()), Default::default()).unwrap();
    let node = result.sequence.items[0].native_node().unwrap();
    assert_eq!(node, expected.sequence.items[0].native_node().unwrap());
    assert!(Arc::ptr_eq(node.owner(), &owner));
    assert_eq!(
        value(
            "local-name(/*/*[1]/following-sibling::*/parent::*)",
            Some(document)
        )
        .1,
        "r"
    );
}
