//! XSLT-VIEW-XML-NODE-PROJECTION: import supplies XML values to the shared CEM view.
use cem_ml::diagnostics::Diagnostic;
use cem_ml::lifecycle::LoadedInputAstStream;
use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use cem_ml::resolver::{ResolverPolicy, ResolverRegistry};
use cem_ml::source_map::FrameSpan;
use cem_ml::validation::xml::{
    xml_document_ast_from_source_bytes, XmlEventKind, XmlSourceValidationRequest,
};
use cem_ml::validation::xpath::{
    xpath_expression_ast_from_source_bytes, CemXPathEvaluator, XPathAttachment,
    XPathDynamicContext, XPathEvaluationLimits, XPathEvaluationRequest, XPathEvaluatorAdapter,
    XPathInvocationHost, XPathNativeNode, XPathNativeNodeHandle, XPathResultArtifact,
    XPathResultItem, XPathSourceRequest,
};
use cem_ml::validation::xpath::{XPathInvocationAdapter, XsltXPathInvocationAdapter};
use cem_ml::validation::xslt::{
    xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest,
};
use std::sync::Arc;

fn owner(source: &str) -> Arc<LoadedInputAstStream> {
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: "memory:xpath-view.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    Arc::new(LoadedInputAstStream::XmlDocument(document.unwrap()))
}

fn run(
    owner: &Arc<LoadedInputAstStream>,
    source: &str,
    limit: Option<u64>,
    control: Option<&OperationControl>,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory:xpath-view.xpath",
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
        dynamic_context: XPathDynamicContext {
            context_item: Some(XPathResultItem::from_native_node(
                XPathNativeNode::xml_document(Arc::clone(owner)).unwrap(),
            )),
            ..Default::default()
        },
        static_context: Default::default(),
        expected_result: None,
        resolver_registry: &registry,
        resolver_policy: &policy,
        evaluation_limits: XPathEvaluationLimits {
            max_sequence_items: limit,
            ..Default::default()
        },
        safety_policy_stamp: "xpath-normalized-view-tests",
        module_resolution: None,
    };
    let evaluator = CemXPathEvaluator::default();
    match control {
        Some(control) => evaluator.evaluate_with_control(request, control, ROOT_EXECUTION_SCOPE_ID),
        None => evaluator.evaluate(request),
    }
}

fn eval(owner: &Arc<LoadedInputAstStream>, source: &str) -> Vec<XPathResultItem> {
    run(owner, source, None, None)
        .unwrap_or_else(|errors| panic!("{source}: {errors:?}"))
        .sequence
        .items
}

fn values(items: &[XPathResultItem]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            XPathResultItem::Node {
                native_node: Some(node),
                ..
            } => node.string_value(),
            XPathResultItem::Atomic { value, .. } => value.lexical_value.clone(),
            other => panic!("unexpected result {other:?}"),
        })
        .collect()
}

#[test]
fn text_runs_coalesce_with_decoded_entities_and_canonical_identity() {
    let source = "<r><![CDATA[]]>a<![CDATA[b]]>c&amp;&#x1F352;<![CDATA[]]></r>";
    let owner = owner(source);
    let LoadedInputAstStream::XmlDocument(original) = owner.as_ref() else {
        unreachable!()
    };
    let snapshot = original.clone();
    let items = eval(&owner, "/r/text()");
    assert_eq!(values(&items), ["abc&🍒"]);
    let node = items[0].native_node().unwrap();
    assert!(Arc::ptr_eq(node.source_owner().as_ref().unwrap(), &owner));
    let events = original
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                XmlEventKind::Text | XmlEventKind::Cdata | XmlEventKind::EntityReference
            )
        })
        .collect::<Vec<_>>();
    assert!(events.len() > 3, "the source AST must remain lexical");
    for event in &events {
        let member = XPathNativeNode::xml_event(Arc::clone(&owner), event.index).unwrap();
        assert_eq!(
            &member, node,
            "every event in the run denotes the same node"
        );
        assert_eq!(member.string_value(), "abc&🍒");
    }
    assert_eq!(
        node.handle(),
        XPathNativeNodeHandle::XmlEvent {
            event_index: events[0].index
        }
    );
    let other_owner = self::owner(source);
    assert_ne!(
        node,
        eval(&other_owner, "/r/text()")[0].native_node().unwrap()
    );
    assert_eq!(
        original, &snapshot,
        "normalizing the view must not mutate source events"
    );
}

#[test]
fn text_run_source_map_and_covering_range_retain_all_source_events() {
    let source = "<r> a<![CDATA[b\nc]]>&amp;d</r>";
    let owner = owner(source);
    let items = eval(&owner, "/r/text()");
    assert_eq!(items.len(), 1);
    let XPathResultItem::Node {
        source_map,
        source_range: Some(range),
        ..
    } = &items[0]
    else {
        panic!("native node with provenance required");
    };
    let LoadedInputAstStream::XmlDocument(document) = owner.as_ref() else {
        unreachable!()
    };
    let expected = document
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                XmlEventKind::Text | XmlEventKind::Cdata | XmlEventKind::EntityReference
            )
        })
        .map(
            |event| match event.source_range.source_map().frames[0].span {
                FrameSpan::Single(range) => range,
                _ => unreachable!(),
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(source_map.frames.len(), 1);
    assert_eq!(source_map.frames[0].span, FrameSpan::Multi(expected));
    assert_eq!(range.start.byte_offset, 3);
    assert_eq!(range.start.line, 1);
    assert_eq!(range.start.column, 4);
    assert_eq!(range.byte_length as usize, source.len() - 7);
}

#[test]
fn empty_runs_disappear_and_markup_separates_nonempty_runs() {
    let owner = owner("\n<?xml-stylesheet href='a'?>\n<r><![CDATA[]]><x/> <![CDATA[z]]><!--stop-->b<![CDATA[c]]><?keep d?>e</r>\n");
    assert_eq!(values(&eval(&owner, "/r/text()")), [" z", "bc", "e"]);
    assert_eq!(
        values(&eval(&owner, "/r/node()")),
        ["", " z", "stop", "bc", "d", "e"]
    );
    assert_eq!(values(&eval(&owner, "/text()")), Vec::<String>::new());
    assert_eq!(eval(&owner, "/node()").len(), 2);
    let LoadedInputAstStream::XmlDocument(document) = owner.as_ref() else {
        unreachable!()
    };
    for event in &document.events {
        if event.kind == XmlEventKind::Cdata && event.value.as_deref() == Some("") {
            assert!(XPathNativeNode::xml_event(Arc::clone(&owner), event.index).is_err());
        }
    }
}

#[test]
fn positions_axes_order_and_identity_use_logical_nodes() {
    let owner = owner("<r>a<![CDATA[b]]><x>c<![CDATA[d]]></x>e<![CDATA[f]]><y/>g</r>");
    for (query, expected) in [
        ("/r/text() ! position()", vec!["1", "2", "3"]),
        ("/r/text() ! last()", vec!["3", "3", "3"]),
        ("/r/x/preceding-sibling::text()[1]", vec!["ab"]),
        ("/r/x/following-sibling::text()[1]", vec!["ef"]),
        ("/r/x/following::text()", vec!["ef", "g"]),
        ("/r/y/preceding::text()", vec!["ab", "cd", "ef"]),
        ("/r/text()[1]/parent::* is /r", vec!["true"]),
        ("/r/text()[1]/ancestor::node()[last()] is /", vec!["true"]),
        ("/r/text()[1] << /r/x/text()", vec!["true"]),
        (
            "(/r/text()[3], /r/text()[1], /r/text()[1]) union ()",
            vec!["ab", "g"],
        ),
    ] {
        assert_eq!(values(&eval(&owner, query)), expected, "{query}");
    }
}

#[test]
fn string_and_typed_values_include_complete_descendant_runs() {
    let owner = owner("<r>a<![CDATA[b]]><x>c&amp;d</x>e&#x1F352;</r>");
    for query in ["string(/)", "string(/r)", "data(/r)"] {
        assert_eq!(values(&eval(&owner, query)), ["abc&de🍒"], "{query}");
    }
    let data = eval(&owner, "data(/r/text())");
    assert_eq!(values(&data), ["ab", "e🍒"]);
    assert!(data.iter().all(|item| matches!(item,
        XPathResultItem::Atomic { value, .. } if value.type_name == "xs:untypedAtomic"
    )));
}

#[test]
fn namespace_declarations_are_not_attributes_and_shadowed_names_stay_expanded() {
    let owner = owner(
        r#"<p:r xmlns:p="urn:outer" xmlns="urn:default" xml:lang="en" a="1"><p:x xmlns:p="urn:inner" p:a="2"/></p:r>"#,
    );
    let attributes = eval(&owner, "/Q{urn:outer}r/@*");
    assert_eq!(attributes.len(), 2);
    assert_eq!(values(&attributes), ["en", "1"]);
    let inner = eval(&owner, "/Q{urn:outer}r/Q{urn:inner}x/@Q{urn:inner}a");
    assert_eq!(values(&inner), ["2"]);
    assert!(
        matches!(&inner[0], XPathResultItem::Node { expanded_name: Some(name), .. } if name == "{urn:inner}a")
    );
    let LoadedInputAstStream::XmlDocument(document) = owner.as_ref() else {
        unreachable!()
    };
    for event in &document.events {
        for (index, attr) in event.attributes.iter().enumerate() {
            let result = XPathNativeNode::xml_attribute(Arc::clone(&owner), event.index, index);
            if attr.qualified_name == "xmlns" || attr.prefix.as_deref() == Some("xmlns") {
                assert!(
                    result.is_err(),
                    "xmlns must not be constructible as an attribute node"
                );
            } else {
                assert!(result.is_ok());
            }
        }
    }
}

#[test]
fn processing_instruction_targets_and_declarations_use_imported_semantics() {
    let owner = owner(r#"<?xml version="1.0"?><r><?keep first?><?other second?><?keep?></r>"#);
    assert_eq!(eval(&owner, "/node()").len(), 1);
    let items = eval(&owner, "/r/processing-instruction()");
    let names = items
        .iter()
        .map(|item| match item {
            XPathResultItem::Node { expanded_name, .. } => expanded_name.as_deref().unwrap(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["keep", "other", "keep"]);
    for query in [
        "/r/processing-instruction('keep')",
        "/r/processing-instruction(keep)",
        "/r/processing-instruction( (: target :) \"keep\" (: end :) )",
        "/r/processing-instruction(' \tkeep\r\n ')",
    ] {
        assert_eq!(values(&eval(&owner, query)), ["first", ""], "{query}");
    }
}

#[test]
fn xslt_owned_xpath_reuses_normalized_native_nodes() {
    let (stylesheet, diagnostics) = xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
        bytes: br#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><xsl:value-of select="/r/text()"/></xsl:template></xsl:stylesheet>"#,
        source_uri: "memory:normalized-view.xsl",
        content_type: Some("application/xslt+xml"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let stylesheet = stylesheet.unwrap();
    let expression = &stylesheet
        .xpath_expressions
        .iter()
        .find(|embedded| embedded.attribute_name == "select")
        .unwrap()
        .expression;
    let XPathAttachment::Host(host) = &expression.attachment else {
        panic!("XSLT-owned typed expression required")
    };
    let registry = ResolverRegistry::new();
    let policy = ResolverPolicy::new();
    // Reuse the compiled expression against changed runtime input, retaining
    // each native owner; no serialization or expression-string bridge.
    for source in ["<r>a<![CDATA[b]]>&amp;</r>", "<r>new<![CDATA[value]]></r>"] {
        let owner = owner(source);
        let result = XsltXPathInvocationAdapter::default()
            .invoke(XPathEvaluationRequest {
                invocation_host: XPathInvocationHost::Xslt,
                expression,
                dynamic_context: XPathDynamicContext {
                    context_item: Some(XPathResultItem::from_native_node(
                        XPathNativeNode::xml_document(Arc::clone(&owner)).unwrap(),
                    )),
                    ..Default::default()
                },
                static_context: host.static_context.clone(),
                expected_result: host.expected_result.clone(),
                resolver_registry: &registry,
                resolver_policy: &policy,
                evaluation_limits: XPathEvaluationLimits {
                    max_sequence_items: Some(1),
                    ..Default::default()
                },
                safety_policy_stamp: "xslt-normalized-view-test",
                module_resolution: None,
            })
            .unwrap();
        assert_eq!(result.sequence.items, eval(&owner, "/r/text()"));
        assert!(Arc::ptr_eq(
            result.sequence.items[0]
                .native_node()
                .unwrap()
                .source_owner()
                .as_ref()
                .unwrap(),
            &owner
        ));
    }
}

#[test]
fn literal_line_endings_normalize_but_character_references_do_not() {
    let owner = owner("<r>a\r\nb<![CDATA[c\rd]]>&#13;<?keep x\r\ny?><!--c\rd--></r>");
    assert_eq!(
        values(&eval(&owner, "/r/node()")),
        ["a\nbc\nd\r", "x\ny", "c\nd"]
    );
}

#[test]
fn normalized_sequences_obey_limits_and_cancellation() {
    let owner = owner(&format!("<r>{}<x/>b</r>", "<![CDATA[a]]>".repeat(256)));
    assert_eq!(
        eval(&owner, "/r/text()[1]")[0]
            .native_node()
            .unwrap()
            .string_value(),
        "a".repeat(256)
    );
    let errors = run(&owner, "/r/text()", Some(1), None).unwrap_err();
    assert!(errors
        .iter()
        .any(|d| d.code == "cem.xpath.sequence_item_limit_exceeded"));
    let control = OperationControl::default();
    let result = run(&owner, "/r/text()", Some(2), Some(&control)).unwrap();
    assert_eq!(result.sequence.items.len(), 2);
    control.cancel_root(None, None).unwrap();
    let errors = run(&owner, "/r/text()", None, Some(&control)).unwrap_err();
    assert!(errors.iter().any(|d| d.code == "cem.xpath.control_failure"));
}

#[test]
fn invalid_pi_target_literals_raise_type_errors_even_without_pi_candidates() {
    let owner = owner("<r/>");
    for query in [
        "/r/processing-instruction('a b')",
        "/r/processing-instruction('')",
        "/r/processing-instruction('p:keep')",
    ] {
        let errors = run(&owner, query, None, None).unwrap_err();
        assert!(
            errors.iter().any(
                |error| error.code == "cem.xpath.processing_instruction_target"
                    && error.message.contains("err:XPTY0004")
            ),
            "{query}: {errors:?}"
        );
    }
}
