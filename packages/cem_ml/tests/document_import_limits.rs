use cem_ml::{
    import::{
        import_data, import_data_bytes, import_string, retain_lifecycle, ImportStringProfile,
        ImportStringRequest,
    },
    lifecycle::LoadedInputAstStream,
    parser::CemAstNode,
    validation::json::{json_document_ast_from_source_bytes, JsonSourceValidationRequest},
};
use std::sync::Arc;

fn array(values: usize) -> String {
    format!("[{}]", vec!["0"; values].join(","))
}

#[test]
fn catalog_sized_document_keeps_every_value_and_source_in_both_ingress_paths() {
    let source = array(8192);
    let uri = "memory:catalog.json";
    let bytes = import_data_bytes(source.as_bytes(), "application/json", "cem", uri).unwrap();
    let (parsed, diagnostics) = json_document_ast_from_source_bytes(JsonSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: uri,
        content_type: Some("application/json"),
    });
    assert!(diagnostics.is_empty());
    let retained = retain_lifecycle(Arc::new(LoadedInputAstStream::JsonDocument(
        parsed.unwrap(),
    )))
    .unwrap();
    for tree in [bytes, retained] {
        assert_eq!(tree.source_uri(), uri);
        let values: Vec<_> = tree
            .ast()
            .iter()
            .filter_map(|node| match node {
                CemAstNode::Text { node_id, data, .. } if data == "0" => Some(*node_id),
                _ => None,
            })
            .collect();
        assert_eq!(values.len(), 8192);
        assert_eq!(
            tree.source_node_range(*values.last().unwrap())
                .unwrap()
                .offset,
            (source.len() - 2) as u64
        );
    }
    assert!(import_data(&source, "json", "cem", uri)
        .unwrap_err()
        .contains("4096-value"));
    assert!(import_string(ImportStringRequest {
        source: &source,
        source_uri: uri,
        base_uri: None,
        profile: ImportStringProfile::JsonXml(Default::default())
    })
    .is_err());
}

#[test]
fn document_value_budget_has_an_exact_boundary_for_both_json_projections() {
    for projection in ["cem", "json-to-xml"] {
        // The root array consumes one value.
        assert!(import_data_bytes(
            array(65535).as_bytes(),
            "application/json",
            projection,
            "memory:limit"
        )
        .is_ok());
        let error = import_data_bytes(
            array(65536).as_bytes(),
            "application/json",
            projection,
            "memory:limit",
        )
        .unwrap_err();
        assert!(error.contains("65536-value"), "{error}");
    }
}

#[test]
fn document_depth_and_bytes_remain_bounded() {
    let nested = format!("{}0{}", "[".repeat(65), "]".repeat(65));
    assert!(
        import_data_bytes(nested.as_bytes(), "application/json", "cem", "memory:depth").is_err()
    );
    let mut oversized = vec![b' '; cem_ml::import::MAX_DOCUMENT_BYTES + 1];
    *oversized.last_mut().unwrap() = b'0';
    assert!(
        import_data_bytes(&oversized, "application/json", "cem", "memory:bytes")
            .unwrap_err()
            .contains("16 MiB")
    );
    let (parsed, _) = json_document_ast_from_source_bytes(JsonSourceValidationRequest {
        bytes: &oversized,
        source_uri: "memory:bytes",
        content_type: Some("application/json"),
    });
    assert!(
        retain_lifecycle(Arc::new(LoadedInputAstStream::JsonDocument(
            parsed.unwrap()
        )))
        .unwrap_err()
        .contains("16 MiB")
    );
}

#[test]
fn other_document_formats_use_the_document_budget() {
    for (source, mime) in [
        (format!("<r>{}</r>", "<v/>".repeat(8192)), "application/xml"),
        (("- 0\n").repeat(8192), "application/yaml"),
        (("0\n").repeat(8192), "text/csv;header=absent"),
    ] {
        assert!(
            import_data_bytes(source.as_bytes(), mime, "cem", "memory:document").is_ok(),
            "{mime}"
        );
    }
}
