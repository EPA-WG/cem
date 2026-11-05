use cem_ml::{
    ast::{decode::DebugBinaryDecoder, encode::DebugBinaryEncoder},
    events::cem::CemEventNormalizer,
    parser::{
        builder::CemAstBuilder,
        document::CemDocument,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
#[path = "support/legacy_cemb.rs"]
mod legacy_cemb;
fn parse(source: &str) -> CemDocument {
    CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
    )))
    .build()
}
#[test]
fn attribute_references_have_native_ownership_and_literals_stay_literal() {
    let source = r#"{item @id={#nodes} @quoted="{#nodes}" @native={#nodes} @general={1 + 2}}"#;
    let document = parse(source);
    let CemAstNode::Element { attributes, .. } = document.get(1).unwrap() else {
        panic!()
    };
    for &id in &attributes[..3] {
        let CemAstNode::Attribute {
            expanded_name,
            value,
            value_nodes,
            ..
        } = document.get(id).unwrap()
        else {
            panic!()
        };
        if expanded_name.local_name == "quoted" {
            assert_eq!(value.as_deref(), Some("{#nodes}"));
            assert!(value_nodes.is_empty());
        } else {
            assert!(value.is_none(), "native expressions are not literal values");
            assert_eq!(value_nodes.len(), 1);
            let CemAstNode::Reference {
                expression,
                context,
                targets,
                source: map,
                ..
            } = document.get(value_nodes[0]).unwrap()
            else {
                panic!()
            };
            assert_eq!(expression, "#nodes");
            assert_eq!(*context, 1);
            assert!(targets.is_none());
            let cem_ml::source_map::FrameSpan::Single(range) = map.current().unwrap().span else {
                panic!()
            };
            assert_eq!(
                &source[range.start as usize..range.end() as usize],
                "#nodes"
            );
        }
    }
    assert!(document.id_table.is_empty());
    let CemAstNode::Attribute { value_nodes, .. } =
        document.get(*attributes.last().unwrap()).unwrap()
    else {
        panic!()
    };
    assert!(
        matches!(document.get(value_nodes[0]), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "$")
    );
}
#[test]
fn native_attribute_binary_and_retained_owner_preserve_value_edges() {
    let source = "{item @native={#nodes}}";
    let document = parse(source);
    let binary = DebugBinaryEncoder::new().encode(&document);
    let decoded = DebugBinaryDecoder::new().decode(&binary.bytes).unwrap();
    let tree = RetainedCemTree::new(
        decoded,
        "fixture.cem",
        source,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let CemAstNode::Element { attributes, .. } = tree.ast().get(1).unwrap() else {
        panic!()
    };
    let attr = attributes[0];
    let CemAstNode::Attribute { value_nodes, .. } = tree.ast().get(attr).unwrap() else {
        panic!()
    };
    assert_eq!(tree.source_parent(value_nodes[0]), Some(attr));
    assert!(tree.node(attr).unwrap().children.is_empty());
}

#[test]
fn native_attribute_inspection_and_formatting_preserve_expression_intent() {
    let source = r#"{item @native={#nodes} @quoted="{#nodes}" @general={1 + 2}}"#;
    let document = parse(source);
    let formatted = cem_ml::formatter::format(&document);
    assert!(formatted.contains("@native={#nodes}"), "{formatted}");
    assert!(formatted.contains("@quoted=\"{#nodes}\""), "{formatted}");
    assert!(formatted.contains("@general={1 + 2}"), "{formatted}");
    assert_eq!(cem_ml::formatter::format(&parse(&formatted)), formatted);
    let dom = cem_ml::projection::dom_json(&document);
    assert_eq!(
        dom["children"][0]["attributes"][0]["valueNodes"][0]["kind"],
        "reference"
    );
    let stream = cem_ml::projection::cem_tree_nodes(&document);
    assert_eq!(stream.as_nodes()[0].attributes()[0].value_nodes.len(), 1);
    let inspection = cem_ml::projection::cem_document_inspection(&document, "fixture.cem");
    let debug = format!("{:?}", inspection.as_nodes());
    assert!(debug.contains("value-node-ids"));
    assert!(debug.contains("unevaluated"));
}
#[test]
fn native_value_owning_edges_cannot_be_dangling_shared_or_cyclic() {
    for edge in [0, 1, 2, 100] {
        let mut document = parse("{item @native={#nodes}}");
        let CemAstNode::Attribute { value_nodes, .. } = &mut document.nodes[2] else {
            panic!()
        };
        value_nodes.push(edge);
        let binary = DebugBinaryEncoder::new().encode(&document);
        assert!(DebugBinaryDecoder::new().decode(&binary.bytes).is_err());
        assert!(RetainedCemTree::new(
            document,
            "fixture.cem",
            "",
            CemTreeSemantics::default(),
            None
        )
        .is_err());
    }
}
#[test]
fn version_three_literal_attributes_decode_with_empty_native_slots() {
    for version in [2, 3, 4] {
        let decoded = legacy_cemb::decode(version, "literal");
        assert!(
            matches!(decoded.get(2), Some(CemAstNode::Attribute { value: Some(value), value_nodes, .. }) if value == "authored" && value_nodes.is_empty())
        );
        assert_eq!(decoded.id_table.get("authored"), Some(&1));
    }
}

#[test]
fn legacy_literal_compatibility_preserves_integrity_truncation_and_count_refusals() {
    use cem_ml::ast::{decode::DecodeError, format::fnv1a64};
    let decoder = DebugBinaryDecoder::new();
    for version in [2, 3, 4] {
        let bytes = legacy_cemb::bytes(version, "literal");
        let mut corrupt = bytes.clone();
        corrupt[0] ^= 1;
        assert!(matches!(
            decoder.decode(&corrupt),
            Err(DecodeError::IntegrityMismatch { .. })
        ));
        // Re-seal a structurally truncated body: the hash must not hide missing fields.
        let mut truncated = bytes[..bytes.len() - 9].to_vec();
        let hash = fnv1a64(&truncated);
        truncated.extend_from_slice(&hash.to_le_bytes());
        assert!(matches!(
            decoder.decode(&truncated),
            Err(DecodeError::UnexpectedEof)
        ));
        assert!(matches!(
            decoder.decode_bounded(&bytes, 0),
            Err(DecodeError::CountLimitExceeded)
        ));
    }
}

#[test]
fn current_metadata_cannot_be_relabelled_as_a_legacy_body() {
    let bytes = DebugBinaryEncoder::new()
        .encode(&parse("{item @id=authored}"))
        .bytes;
    assert!(u16::from_le_bytes(bytes[4..6].try_into().unwrap()) >= 5);
    for version in [2u16, 3, 4] {
        let mut relabelled = bytes.clone();
        relabelled[4..6].copy_from_slice(&version.to_le_bytes());
        let end = relabelled.len() - 8;
        let hash = cem_ml::ast::format::fnv1a64(&relabelled[..end]);
        relabelled[end..].copy_from_slice(&hash.to_le_bytes());
        assert!(DebugBinaryDecoder::new().decode(&relabelled).is_err());
    }
}
