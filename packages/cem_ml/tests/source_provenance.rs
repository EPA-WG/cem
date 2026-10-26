//! SOURCE-PROVENANCE-IMPORT: stable source tokens over retained common trees.
use cem_ml::{
    import::{
        import_data, import_data_bytes, import_string, CsvHeader, CsvImportOptions,
        ImportStringProfile, ImportStringRequest, JsonXmlDuplicates, JsonXmlProjectionOptions,
    },
    parser::{
        document::CemDocument,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    validation::xpath::XPathNativeNode,
};

fn string(source: &str, profile: ImportStringProfile) -> XPathNativeNode {
    XPathNativeNode::cem_document(
        import_string(ImportStringRequest {
            source,
            source_uri: "memory:source",
            base_uri: None,
            profile,
        })
        .unwrap(),
    )
}

#[test]
fn imported_keys_are_stable_distinct_and_invalidated_by_content_or_source_changes() {
    for (format, source) in [
        ("xml", "<r><x>A</x><x>A</x></r>"),
        ("json", "[\"A\",\"A\"]"),
        ("yaml", "[A, A]"),
        ("csv", "v\nA\nA"),
    ] {
        let import = |text: &str, uri: &str| {
            XPathNativeNode::cem_document(import_data(text, format, "cem", uri).unwrap())
        };
        let first = import(source, "memory:source");
        let again = import(source, "memory:source");
        let bytes = XPathNativeNode::cem_document(
            import_data_bytes(source.as_bytes(), format, "cem", "memory:source").unwrap(),
        );
        assert_ne!(first, again, "XDM identity remains per-document");
        assert!(first.source_key().is_some());
        assert_eq!(first.source_key(), again.source_key(), "{format}");
        assert_eq!(first.source_key(), bytes.source_key(), "{format}");
        assert_ne!(
            first.source_key(),
            import(&source.replace('A', "B"), "memory:source").source_key()
        );
        assert_ne!(
            first.source_key(),
            import(source, "memory:other").source_key()
        );
        let rows = first.child_nodes()[0].child_nodes();
        assert_ne!(rows[0].source_key(), rows[1].source_key(), "{format}");
    }
}

#[test]
fn string_import_profiles_and_json_projection_options_cannot_alias() {
    let source = "[\n{\"x\":\"A\"},\n{\"x\":\"A\"}\n]";
    let options = JsonXmlProjectionOptions::default();
    let first = string(source, ImportStringProfile::JsonXml(options));
    let again = string(source, ImportStringProfile::JsonXml(options));
    assert_eq!(first.source_key(), again.source_key());
    for changed in [
        JsonXmlProjectionOptions {
            escape: true,
            ..options
        },
        JsonXmlProjectionOptions {
            duplicates: JsonXmlDuplicates::UseFirst,
            ..options
        },
    ] {
        assert_ne!(
            first.source_key(),
            string(source, ImportStringProfile::JsonXml(changed)).source_key()
        );
    }
    let generic =
        XPathNativeNode::cem_document(import_data(source, "json", "cem", "memory:source").unwrap());
    assert_ne!(first.source_key(), generic.source_key());
    for tree in [
        first,
        XPathNativeNode::cem_document(
            import_data_bytes(source.as_bytes(), "json", "json-to-xml", "memory:source").unwrap(),
        ),
    ] {
        let rows = tree.child_nodes()[0].child_nodes();
        assert_eq!(
            rows.iter()
                .map(XPathNativeNode::source_line_number)
                .collect::<Vec<_>>(),
            [Some(2), Some(3)]
        );
        assert!(rows.iter().all(|row| !row.source_map().frames.is_empty()));
    }
    let keyed = XPathNativeNode::cem_document(
        import_data_bytes(
            b"{\n\"key\":\n\"value\"}",
            "json",
            "json-to-xml",
            "memory:keyed",
        )
        .unwrap(),
    )
    .child_nodes()[0]
        .child_nodes()[0]
        .clone();
    assert_eq!(keyed.source_line_number(), Some(3));
    assert_eq!(keyed.attribute_nodes()[0].source_line_number(), Some(2));
    assert_eq!(keyed.child_nodes()[0].source_line_number(), Some(3));
}

#[test]
fn canonical_text_nodes_share_their_source_key_and_original_location() {
    let tree = string(
        "<r>\na<![CDATA[b]]>&amp;<x/>\n</r>",
        ImportStringProfile::Xml,
    );
    let root = tree.child_nodes()[0].clone();
    let text = root.child_nodes()[0].clone();
    assert_eq!(text.string_value(), "\nab&");
    assert_eq!(text.source_line_number(), Some(1));
    let owner = tree.owner();
    let aliases: Vec<_> = (0..owner.ast().nodes.len() as u32)
        .filter_map(|id| XPathNativeNode::cem_node(owner.clone(), id).ok())
        .filter(|node| *node == text)
        .collect();
    assert!(aliases.len() > 1);
    assert!(aliases
        .iter()
        .all(|node| node.source_key() == text.source_key()));
}

#[test]
fn synthetic_trees_do_not_fabricate_source_provenance() {
    let mut ast = CemDocument::default();
    ast.nodes.push(CemAstNode::Document {
        node_id: 0,
        root_children: vec![],
        source: Default::default(),
    });
    let tree = XPathNativeNode::cem_document(
        RetainedCemTree::new(
            ast,
            "memory:synthetic",
            "",
            CemTreeSemantics::default(),
            None,
        )
        .unwrap(),
    );
    assert_eq!(tree.source_key(), None);
    assert_eq!(tree.source_line_number(), None);
    // Byte offsets alone cannot recover line numbers when original source text
    // and import-provided locations are both unavailable.
    let imported = import_data("\n<r/>", "xml", "cem", "memory:source").unwrap();
    let unknown = XPathNativeNode::cem_document(
        RetainedCemTree::new(
            CemDocument {
                nodes: imported.ast().nodes.clone(),
                ..Default::default()
            },
            "memory:unknown",
            "",
            CemTreeSemantics::default(),
            None,
        )
        .unwrap(),
    )
    .child_nodes()
    .into_iter()
    .find(|node| node.local_name() == "r")
    .unwrap();
    assert!(unknown.source_map().origin().is_some());
    assert_eq!(unknown.source_key(), None);
    assert_eq!(unknown.source_line_number(), None);
}

#[test]
fn csv_header_option_matches_cemt_import_without_changing_the_default() {
    let source = "label,qty\n\"B, C\",10\nA,2";
    let absent = string(source, ImportStringProfile::Csv);
    let explicit_absent = string(
        source,
        ImportStringProfile::CsvWithOptions(CsvImportOptions {
            header: CsvHeader::Absent,
        }),
    );
    let present = string(
        source,
        ImportStringProfile::CsvWithOptions(CsvImportOptions {
            header: CsvHeader::Present,
        }),
    );
    assert_eq!(absent.source_key(), explicit_absent.source_key());
    assert_ne!(absent.source_key(), present.source_key());
    assert_eq!(absent.child_nodes()[0].child_nodes().len(), 3);
    let rows = present.child_nodes()[0].child_nodes();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].local_name(), "object");
    assert_eq!(rows[0].string_value(), "B, C10");
    assert_eq!(rows[0].source_line_number(), Some(2));
    let cemt =
        XPathNativeNode::cem_document(import_data(source, "csv", "cem", "memory:source").unwrap());
    assert_eq!(present.string_value(), cemt.string_value());
    assert_eq!(
        rows[0].child_nodes()[0].attribute_nodes()[0].string_value(),
        "label"
    );
    for (source, profile) in [
        ("", ImportStringProfile::Csv),
        ("[A, B]", ImportStringProfile::Yaml),
    ] {
        let document = string(source, profile);
        assert!(!document.source_map().frames.is_empty());
        assert_eq!(document.source_line_number(), Some(1));
    }
}

#[test]
fn source_and_semantic_maps_keep_distinct_ranges_for_coalesced_text() {
    let tree = import_data(
        "<r>é<![CDATA[b]]>&amp;</r>",
        "xml",
        "cem",
        "memory:original.xml",
    )
    .unwrap();
    let ids: Vec<_> = tree
        .ast()
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Text { node_id, .. } | CemAstNode::Cdata { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect();
    assert!(ids.len() > 1);
    let semantic = tree.node_source_map(ids[0]).unwrap();
    for id in &ids {
        assert_eq!(tree.node_source_map(*id).unwrap(), semantic);
        let source = tree.source_node_map(*id).unwrap();
        assert_eq!(source.frames.len(), 2);
        assert!(matches!(&source.frames[1].transform,
            cem_ml::source_map::TransformKind::SourceBoundary { uri } if uri == "memory:original.xml"));
        let origin = source.origin().unwrap();
        assert_eq!(source.frames[1].span, origin.span);
    }
    assert_ne!(tree.source_node_map(ids[0]), tree.source_node_map(ids[1]));
    assert!(tree.source_node_map(u32::MAX).is_none());
    assert!(tree.node_source_map(u32::MAX).is_none());
}

#[test]
fn retained_maps_keep_existing_origin_boundaries_and_do_not_invent_empty_maps() {
    let imported = import_data("<r/>", "xml", "cem", "memory:original.xml").unwrap();
    let mut ast = CemDocument {
        nodes: imported.ast().nodes.clone(),
        ..Default::default()
    };
    for node in &mut ast.nodes {
        if let CemAstNode::Element { source, .. } = node {
            *source = source.clone().with_source_uri("memory:original.xml");
        }
    }
    let tree = RetainedCemTree::new(
        ast,
        "memory:wrapper.xml",
        "",
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let id = tree.node(0).unwrap().children[0];
    for map in [
        tree.node_source_map(id).unwrap(),
        tree.source_node_map(id).unwrap(),
    ] {
        assert_eq!(map.frames.len(), 2);
        assert!(matches!(&map.frames[1].transform,
            cem_ml::source_map::TransformKind::SourceBoundary { uri } if uri == "memory:original.xml"));
    }
    let mut ast = CemDocument::default();
    ast.nodes.push(CemAstNode::Document {
        node_id: 0,
        root_children: vec![],
        source: Default::default(),
    });
    let synthetic = RetainedCemTree::new(
        ast,
        "memory:synthetic",
        "",
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    assert!(synthetic.node_source_map(0).unwrap().frames.is_empty());
    assert!(synthetic.source_node_map(0).unwrap().frames.is_empty());
}
