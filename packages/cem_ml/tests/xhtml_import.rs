//! XHTML retains its original lifecycle owner at the common CEM import boundary.
use cem_ml::{
    import::{import_data_bytes, retain_lifecycle, try_retain_lifecycle},
    lifecycle::LoadedInputAstStream,
    parser::{tree::RetainedCemTree, AstNodeId, CemAstNode},
    validation::xhtml::{xhtml_document_ast_from_source_bytes, XhtmlSourceValidationRequest},
};
use std::sync::Arc;

const MIME: &str = "application/xhtml+xml;charset=utf-8";
const URI: &str = "memory://documents/article.xhtml";
const SOURCE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<h:html xmlns:h="http://www.w3.org/1999/xhtml" xmlns:s="http://www.w3.org/2000/svg">
  <h:head><h:title>Café &amp; documents</h:title></h:head>
  <h:body><h:p id="review">A &amp; B<![CDATA[ < C]]></h:p><s:svg><s:title>Foreign title</s:title></s:svg></h:body>
</h:html>
"#;

fn lifecycle(source: &str) -> Arc<LoadedInputAstStream> {
    let (document, _) = xhtml_document_ast_from_source_bytes(XhtmlSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: URI,
        content_type: Some(MIME),
    });
    Arc::new(LoadedInputAstStream::XhtmlDocument(document.unwrap()))
}

fn element(tree: &RetainedCemTree, namespace: &str, name: &str) -> AstNodeId {
    tree.ast()
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.namespace_uri == namespace && expanded_name.local_name == name => {
                Some(*node_id)
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn xhtml_lifecycle_import_keeps_namespaces_decoded_values_ranges_and_original_owner() {
    let owner = lifecycle(SOURCE);
    let weak = Arc::downgrade(&owner);
    let tree = try_retain_lifecycle(owner.clone()).unwrap().unwrap();
    let kept = tree
        .native_owner()
        .unwrap()
        .clone()
        .downcast::<LoadedInputAstStream>()
        .unwrap();
    assert!(Arc::ptr_eq(&owner, &kept));
    let LoadedInputAstStream::XhtmlDocument(document) = kept.as_ref() else {
        panic!("XHTML must retain its own lifecycle identity");
    };
    assert_eq!(document.source.content_type, MIME);
    assert_eq!(tree.source_uri(), URI);
    let paragraph = element(&tree, "http://www.w3.org/1999/xhtml", "p");
    let range = tree.source_node_range(paragraph).unwrap();
    assert_eq!(range.offset, SOURCE.find("<h:p").unwrap() as u64);
    assert_eq!(range.line, 4);
    assert_eq!(tree.node_source_uri(paragraph), Some(URI));
    assert!(tree.ast().iter().any(|node| matches!(node,
        CemAstNode::Text { data, .. } if data == "A & B")));
    assert!(tree.ast().iter().any(|node| matches!(node,
        CemAstNode::Cdata { data, .. } if data == " < C")));
    element(&tree, "http://www.w3.org/2000/svg", "title");
    drop(owner);
    drop(kept);
    assert!(
        weak.upgrade().is_some(),
        "the retained tree owns its original source"
    );
    drop(tree);
    assert!(
        weak.upgrade().is_none(),
        "disposing the last tree releases its owner"
    );
}

#[test]
fn xhtml_byte_import_selects_xhtml_before_the_generic_xml_suffix() {
    for projection in ["cem", "xpath"] {
        let tree = import_data_bytes(SOURCE.as_bytes(), MIME, projection, URI).unwrap();
        let owner = tree
            .native_owner()
            .unwrap()
            .clone()
            .downcast::<LoadedInputAstStream>()
            .unwrap();
        assert!(matches!(
            owner.as_ref(),
            LoadedInputAstStream::XhtmlDocument(_)
        ));
        element(&tree, "http://www.w3.org/1999/xhtml", "p");
    }
    // XML-shaped bytes do not satisfy the distinct XHTML root/namespace contract.
    assert!(import_data_bytes(b"<catalog/>", MIME, "cem", URI).is_err());
    assert!(import_data_bytes(b"<catalog/>", "application/xml", "cem", URI).is_ok());
}

#[test]
fn xhtml_import_refuses_hard_source_facts_at_both_entrypoints() {
    for source in [
        SOURCE.replace("http://www.w3.org/1999/xhtml", "urn:wrong-namespace"),
        SOURCE.replace("</h:p>", "</h:section>"),
        SOURCE.replace("<h:head>", "<missing:head>"),
        SOURCE.replace(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
            "<!DOCTYPE h:html SYSTEM 'file:///not-admitted.dtd'>",
        ),
        SOURCE.replace("Café &amp; documents", "&external;"),
        SOURCE.replace(
            "<h:head><h:title>Café &amp; documents</h:title></h:head>",
            "",
        ),
    ] {
        assert!(
            retain_lifecycle(lifecycle(&source)).is_err(),
            "retained: {source}"
        );
        assert!(
            import_data_bytes(source.as_bytes(), MIME, "cem", URI).is_err(),
            "bytes: {source}"
        );
    }
}

#[test]
fn xhtml_import_preserves_document_event_and_depth_limits() {
    let large = format!(
        "<html xmlns=\"http://www.w3.org/1999/xhtml\"><head/><body>{}</body></html>",
        "<p/>".repeat(8192)
    );
    assert!(import_data_bytes(large.as_bytes(), MIME, "cem", URI).is_ok());
    let nested = format!(
        "<html xmlns=\"http://www.w3.org/1999/xhtml\"><head/><body>{}{}</body></html>",
        "<div>".repeat(65),
        "</div>".repeat(65)
    );
    for result in [
        retain_lifecycle(lifecycle(&nested)),
        import_data_bytes(nested.as_bytes(), MIME, "cem", URI),
    ] {
        assert!(result.unwrap_err().contains("64-level"));
    }
    let over_events = large.replace(&"<p/>".repeat(8192), &"<p/>".repeat(65_536));
    assert!(retain_lifecycle(lifecycle(&over_events))
        .unwrap_err()
        .contains("65536-event"));
}
