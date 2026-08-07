use std::collections::BTreeMap;
use std::sync::Arc;

use cem_ml::lifecycle::LoadedInputAstStream;
use cem_ml::resolver::{ResolverPolicy, ResolverRegistry};
use cem_ml::source::ByteRange;
use cem_ml::source_map::{FrameSpan, SourceMapStack};
use cem_ml::validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest};
use cem_ml::validation::xpath::{
    xpath_expression_ast_from_source_bytes, CemQlXPathInvocationAdapter, XPathAtomicValue,
    XPathAttachment, XPathDynamicContext, XPathEvaluationLimits, XPathEvaluationPhase,
    XPathEvaluationRequest, XPathExpandedName, XPathExpectedResult, XPathHostNodeKind,
    XPathInvocationAdapter, XPathInvocationHost, XPathNativeNode, XPathResultItem,
    XPathResultSequence, XPathSourceRange, XPathSourceRequest, XPathStaticContext,
    XPATH_CONTENT_TYPE,
};
use cem_ql::api::{CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI};
use cem_ql::xpath::{
    cem_ql_xpath_expression_slot_from_source_bytes, invoke_cem_ql_xpath_expression_slot,
    CemQlXPathExpressionSlotRequest, CemQlXPathHostBindings, CemQlXPathRuntimeContext,
};

fn integer_binding(value: &str) -> XPathResultSequence {
    XPathResultSequence {
        sequence_type: "xs:integer".to_owned(),
        items: vec![XPathResultItem::Atomic {
            value: XPathAtomicValue {
                type_name: "xs:integer".to_owned(),
                lexical_value: value.to_owned(),
                namespace_uri: None,
                local_name: None,
            },
            source_map: SourceMapStack::default(),
        }],
    }
}

#[test]
fn schema_owned_cem_ql_xpath_slot_compiles_once_and_invokes_native_bindings() {
    let expression_source = "$vars:limit = /root/n, /root/n[$vars:index]";
    let expression_range = XPathSourceRange::new(5, 9, 96, expression_source.len() as u64);
    let static_context = XPathStaticContext {
        namespaces: BTreeMap::from([("vars".to_owned(), "urn:cem:variables".to_owned())]),
        variable_bindings: BTreeMap::from([
            ("vars:limit".to_owned(), "xs:integer".to_owned()),
            ("vars:index".to_owned(), "xs:integer".to_owned()),
        ]),
        ..XPathStaticContext::default()
    };
    let expected_result = XPathExpectedResult {
        sequence_type: "item()*".to_owned(),
        min_items: Some(2),
        max_items: Some(2),
    };
    let (slot, diagnostics) =
        cem_ql_xpath_expression_slot_from_source_bytes(CemQlXPathExpressionSlotRequest {
            bytes: expression_source.as_bytes(),
            source_uri: "memory://query.cem-ql",
            source_id: 73,
            slot_path: "module/functions/catalog/xpath",
            owner_range: XPathSourceRange::new(4, 1, 72, 96),
            expression_range,
            static_context: static_context.clone(),
            expected_result: Some(expected_result.clone()),
            evaluation_phase: XPathEvaluationPhase::Runtime,
            resolver_policy_stamp: Some("resolver:cem-ql-runtime"),
            safety_policy_stamp: Some("xpath-safety/1;cem-ql-expression-slot"),
        });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let slot = slot.expect("schema-owned CEM-QL XPath expression slot");
    assert_eq!(slot.slot_path, "module/functions/catalog/xpath");
    let XPathAttachment::Host(host) = &slot.expression.attachment else {
        panic!("CEM-QL XPath slot must own a host-attached AST")
    };
    assert_eq!(host.owner.node_kind, XPathHostNodeKind::CemQlExpressionSlot);
    assert_eq!(
        host.owner.content_type.as_deref(),
        Some(CEM_QL_EXPRESSION_CONTENT_TYPE)
    );
    assert_eq!(
        host.owner.schema_uri.as_deref(),
        Some(CEM_QL_EXPRESSION_SCHEMA_URI)
    );
    assert_eq!(host.expression_range, expression_range);
    assert_eq!(
        slot.expression
            .syntax_ast
            .as_ref()
            .expect("typed XPath AST")
            .root
            .source_range,
        expression_range
    );

    let xml_source = br#"<root><n>2</n><n>10</n></root>"#;
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: xml_source,
        source_uri: "memory://cem-ql-context.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let owner = Arc::new(LoadedInputAstStream::XmlDocument(
        document.expect("typed XML document"),
    ));
    let context_item = XPathResultItem::from_native_node(
        XPathNativeNode::xml_document(Arc::clone(&owner)).expect("native XML document node"),
    );
    let host_bindings = CemQlXPathHostBindings {
        context_item: Some(context_item),
        variable_bindings: BTreeMap::from([
            (
                XPathExpandedName::new(Some("urn:cem:variables"), "limit"),
                integer_binding("2"),
            ),
            (
                XPathExpandedName::new(Some("urn:cem:variables"), "index"),
                integer_binding("2"),
            ),
        ]),
    };
    let resolver_registry = ResolverRegistry::new();
    let resolver_policy = ResolverPolicy::new();
    let result = invoke_cem_ql_xpath_expression_slot(
        &slot,
        &host_bindings,
        XPathEvaluationLimits {
            max_sequence_items: Some(2),
        },
        CemQlXPathRuntimeContext {
            resolver_registry: &resolver_registry,
            resolver_policy: &resolver_policy,
        },
    )
    .expect("CEM-QL invokes the fused XPath AST over native bindings");

    let [XPathResultItem::Atomic { value, .. }, XPathResultItem::Node { native_node, .. }] =
        result.sequence.items.as_slice()
    else {
        panic!("expected boolean and native node result: {result:?}");
    };
    assert_eq!(value.type_name, "xs:boolean");
    assert_eq!(value.lexical_value, "true");
    assert_eq!(result.invocation_host, XPathInvocationHost::CemQl);
    assert_eq!(result.expression_uri, "memory://query.cem-ql");
    assert!(result.safety_policy_stamp.contains("xpath-items=2"));
    assert_eq!(
        result.source_map.frames[0].span,
        FrameSpan::Single(ByteRange::new(96, expression_source.len() as u32))
    );
    assert!(Arc::ptr_eq(
        native_node.as_ref().expect("native result node").owner(),
        &owner
    ));
}

#[test]
fn cem_ql_xpath_adapter_rejects_non_owned_asts_and_runtime_bridges() {
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: b"1",
            source_uri: "memory://standalone.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 1 },
    );
    let resolver_registry = ResolverRegistry::new();
    let resolver_policy = ResolverPolicy::new();
    let diagnostics = CemQlXPathInvocationAdapter
        .invoke(XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::CemQl,
            expression: &expression,
            dynamic_context: XPathDynamicContext::default(),
            static_context: XPathStaticContext::default(),
            expected_result: None,
            resolver_registry: &resolver_registry,
            resolver_policy: &resolver_policy,
            evaluation_limits: XPathEvaluationLimits::default(),
            safety_policy_stamp: "xpath-safety/1;cem-ql-expression-slot",
        })
        .expect_err("CEM-QL adapter rejects a standalone XPath AST");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "cem.xpath.invocation_host_mismatch");

    let source = include_str!("../src/xpath.rs");
    let runtime = source
        .split_once("pub fn invoke_cem_ql_xpath_expression_slot")
        .expect("CEM-QL XPath runtime source boundary")
        .1
        .split_once("fn cem_ql_xpath_slot_attachment")
        .expect("CEM-QL XPath runtime source boundary")
        .0;
    for forbidden in [
        "serde_json",
        "source_text",
        "xpath_expression_ast_from_source_bytes",
        "xml_document_ast_from_source_bytes",
        "ItemStream",
    ] {
        assert!(
            !runtime.contains(forbidden),
            "CEM-QL XPath runtime must not cross `{forbidden}`"
        );
    }
}
