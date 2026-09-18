//! CEM-LOADER-NATIVE: byte ingress and native owner lifetime.
use cem_ml::import::{documents::CemDocuments, import_data_bytes};
use cem_ml::validation::xpath::XPathNativeNode;
use std::sync::Arc;

#[test]
fn loader_imports_bytes_and_retains_selected_nodes_after_release() {
    let mut documents = CemDocuments::default();
    for (bytes, content_type) in [
        (
            b"{\"qty\":3}".as_slice(),
            "application/problem+json; charset=utf-8",
        ),
        (b"<qty>3</qty>".as_slice(), "text/xml; charset=utf-8"),
        (b"qty: 3".as_slice(), "application/yaml"),
        (b"qty\n3".as_slice(), "text/csv"),
        (b"3".as_slice(), "text/plain; charset=utf-8"),
    ] {
        let id = documents
            .retain(bytes, content_type, "https://example.test/data")
            .unwrap();
        let tree = documents.get(id).unwrap();
        assert!(Arc::ptr_eq(&tree, &documents.get(id).unwrap()));
        let node = XPathNativeNode::cem_document(tree);
        assert_eq!(node.string_value(), "3");
        assert!(documents.dispose(id));
        assert!(documents.get(id).is_none());
        assert_eq!(node.string_value(), "3");
        assert!(!documents.dispose(id));
    }
}

#[test]
fn failed_imports_do_not_publish_handles_and_ids_are_not_reused() {
    let mut documents = CemDocuments::default();
    assert!(documents
        .retain(b"{", "application/json", "bad.json")
        .is_err());
    assert!(documents
        .retain(b"<r>", "application/xml", "bad.xml")
        .is_err());
    assert!(documents
        .retain(b"ok", "application/octet-stream", "unknown")
        .is_err());
    assert!(documents
        .retain(&[0xff], "application/json", "bad.json")
        .is_err());
    let first = documents.retain(b"{}", "application/json", "one").unwrap();
    documents.dispose(first);
    let second = documents.retain(b"{}", "application/json", "two").unwrap();
    assert!(second > first);
    assert!(documents.get(first).is_none());
}

#[test]
fn retained_document_count_and_byte_ingress_are_bounded() {
    let mut documents = CemDocuments::default();
    for _ in 0..64 {
        documents
            .retain(b"{}", "application/json", "memory:data")
            .unwrap();
    }
    assert!(documents
        .retain(b"{}", "application/json", "memory:excess")
        .is_err());
    assert!(documents.dispose(1));
    assert!(documents
        .retain(b"{}", "application/json", "memory:again")
        .is_ok());
    assert!(import_data_bytes(
        &vec![b' '; 16 * 1024 * 1024 + 1],
        "application/json",
        "cem",
        "large"
    )
    .is_err());
}
