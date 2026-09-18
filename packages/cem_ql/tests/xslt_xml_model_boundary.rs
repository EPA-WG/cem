//! XSLT-VIEW-XML-NODE-GATE: the default CEM reader stays source-oriented.
//! XSLT-VIEW-XML-NODE-PROJECTION: XPath normalizes its own native node view
//! without altering that reader contract or the retained source AST.
use cem_ml::lifecycle::LoadedInputAstStream;
use cem_ml::resolver::{ResolverPolicy, ResolverRegistry};
use cem_ml::validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest};
use cem_ml::validation::xpath::{
    xpath_expression_ast_from_source_bytes, CemXPathEvaluator, XPathAttachment,
    XPathDynamicContext, XPathEvaluationRequest, XPathEvaluatorAdapter, XPathInvocationHost,
    XPathNativeNode, XPathResultItem, XPathSourceRequest,
};
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, QueryItemViewKind};
use std::sync::Arc;

fn read(source: &str) -> Item {
    // JSON is used only to quote a source-string literal, never to hand off an AST.
    let query = format!(
        "data:read({}, \"xml\")",
        serde_json::to_string(source).unwrap()
    );
    let compiled = compile(&query, &CompileContext::default()).expect("probe compiles");
    let result = evaluate(&compiled, &EvaluationContext::default());
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let report = &result.items[0];
    assert_eq!(text(report, "error"), "");
    field(report, "root").remove(0)
}

fn field(item: &Item, name: &str) -> Vec<Item> {
    item.view()
        .expect("native view")
        .field(name)
        .unwrap_or_default()
}

fn text(item: &Item, name: &str) -> String {
    match field(item, name).as_slice() {
        [Item::Atomic(AtomValue::String(value))] => value.clone(),
        other => panic!("expected {name} string, got {other:?}"),
    }
}

#[test]
fn cem_xml_text_cdata_boundaries_have_distinct_native_identities() {
    let document = read("<r>a<![CDATA[b]]>c</r>");
    let element = field(&document, "children").remove(0);
    let children = field(&element, "children");
    assert_eq!(
        children
            .iter()
            .map(|node| text(node, "kind"))
            .collect::<Vec<_>>(),
        ["text", "cdata", "text"]
    );
    assert_eq!(
        children
            .iter()
            .map(|node| text(node, "value"))
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    for node in &children {
        assert_eq!(node.view().unwrap().kind(), QueryItemViewKind::Node);
        assert!(node.source_map().is_some());
        assert!(node.identity().is_some());
    }
    assert_ne!(children[0].identity(), children[1].identity());
    assert_ne!(children[1].identity(), children[2].identity());
    // XPath /r/text() must instead return one native text node with value abc.
    // Concatenating values is possible, but does not construct that native node.
    assert_eq!(field(&element, "children"), children);
}

#[test]
fn cem_xml_preserves_empty_cdata_and_separate_whitespace_nodes() {
    let document = read("<r><![CDATA[]]><x/> <![CDATA[z]]></r>");
    let element = field(&document, "children").remove(0);
    let children = field(&element, "children");
    assert_eq!(
        children
            .iter()
            .map(|node| text(node, "kind"))
            .collect::<Vec<_>>(),
        ["cdata", "element", "whitespace", "cdata"]
    );
    assert_eq!(text(&children[0], "value"), "");
    assert_eq!(text(&children[2], "value"), " ");
    assert_eq!(text(&children[3], "value"), "z");
    assert!(children[0].source_map().is_some());
    // XDM omits the empty text node and coalesces the final whitespace + CDATA.
}

#[test]
fn cem_xml_keeps_declaration_and_xmlns_source_nodes_without_navigation_metadata() {
    let document = read(
        r#"<?xml version="1.0"?><?keep yes?><p:r xmlns:p="urn:p" xmlns:q="urn:q" q:a="1"><q:x/></p:r>"#,
    );
    let children = field(&document, "children");
    assert_eq!(
        children
            .iter()
            .map(|node| text(node, "kind"))
            .collect::<Vec<_>>(),
        [
            "processing-instruction",
            "processing-instruction",
            "element"
        ]
    );
    assert_eq!(text(&children[0], "target"), "xml");
    // Current import also falls back to "xml" for a real PI's target; its
    // retained data still contains the original lexical target and content.
    assert_eq!(text(&children[1], "target"), "xml");
    assert_eq!(text(&children[1], "value"), "keep yes");
    let element = &children[2];
    assert_eq!(text(element, "name"), "r");
    assert_eq!(text(element, "namespace"), "urn:p");
    let attributes = field(element, "attributes");
    assert_eq!(attributes.len(), 3);
    assert_eq!(
        text(&attributes[0], "namespace"),
        "http://www.w3.org/2000/xmlns/"
    );
    assert_eq!(
        text(&attributes[1], "namespace"),
        "http://www.w3.org/2000/xmlns/"
    );
    assert_eq!(text(&attributes[2], "namespace"), "urn:q");
    assert_eq!(text(&attributes[2], "name"), "a");
    for name in ["parent", "root", "prefix", "document-order"] {
        assert!(
            field(element, name).is_empty(),
            "unexpected {name} capability"
        );
    }
    // Filtering can exclude declarations/xmlns, but cannot restore a lost
    // lexical prefix or construct a coalesced native text-node identity.
}

#[test]
fn xpath_native_owner_exposes_normalized_text_without_changing_cem_reader() {
    let source = "<r>a<![CDATA[b]]>c</r>";
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: "memory:xdm-probe.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let owner = Arc::new(LoadedInputAstStream::XmlDocument(document.unwrap()));
    let root = XPathNativeNode::xml_document(Arc::clone(&owner)).unwrap();
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: b"/r/text()",
            source_uri: "memory:xdm-probe.xpath",
            content_type: Some("application/xpath"),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 7 },
    );
    let registry = ResolverRegistry::new();
    let policy = ResolverPolicy::new();
    let result = CemXPathEvaluator::default()
        .evaluate(XPathEvaluationRequest {
            invocation_host: XPathInvocationHost::StandaloneTransform,
            expression: &expression,
            dynamic_context: XPathDynamicContext {
                context_item: Some(XPathResultItem::from_native_node(root)),
                ..XPathDynamicContext::default()
            },
            static_context: Default::default(),
            expected_result: None,
            resolver_registry: &registry,
            resolver_policy: &policy,
            evaluation_limits: Default::default(),
            safety_policy_stamp: "xslt-xml-model-probe",
            module_resolution: None,
        })
        .expect("existing native XPath path executes");
    let nodes = result
        .sequence
        .items
        .iter()
        .map(|item| item.native_node().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        nodes.len(),
        1,
        "XPath coalesces text events over the retained native XML owner"
    );
    assert_eq!(
        nodes
            .iter()
            .map(|node| node.string_value())
            .collect::<Vec<_>>(),
        ["abc"]
    );
    for node in nodes {
        assert!(Arc::ptr_eq(node.source_owner().as_ref().unwrap(), &owner));
        assert!(!node.source_map().frames.is_empty());
    }
}
