use cem_ml::{
    ast::{decode::DebugBinaryDecoder, encode::DebugBinaryEncoder},
    events::cem::CemEventNormalizer,
    import::import_data_bytes,
    parser::{builder::CemAstBuilder, CemAstNode},
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};

fn parse(source: &str) -> cem_ml::parser::document::CemDocument {
    let tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    CemAstBuilder::new(CemEventNormalizer::new(tokenizer)).build()
}

#[test]
fn load_retains_forward_reference_without_evaluating() {
    let doc = parse("{section | {#nodes} {target}}");
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    let references: Vec<_> = doc
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Reference {
                node_id,
                expression,
                context,
                targets,
                ..
            } => Some((*node_id, expression, *context, targets)),
            _ => None,
        })
        .collect();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].1, "#nodes");
    assert_eq!(references[0].2, 1);
    assert!(references[0].3.is_none());
    assert_eq!(
        cem_ml::formatter::format(&doc),
        "{section |\n    {#nodes}\n    {target}\n}\n"
    );
}

#[test]
fn xml_namespace_alias_and_curly_surface_retain_equivalent_references() {
    let xml = import_data_bytes(b"<section xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr>#nodes</r:expr><target/></section>", "application/xml", "cem", "test.xml").unwrap();
    let reference = xml
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Reference {
                expression,
                targets,
                ..
            } => Some((expression, targets)),
            _ => None,
        })
        .expect("typed reference through namespace alias");
    assert_eq!(reference.0, "#nodes");
    assert!(reference.1.is_none());
    assert_eq!(xml.node(0).unwrap().children.len(), 1);
}

#[test]
fn binary_roundtrip_preserves_identity_empty_resolution_and_cycles() {
    let mut doc = parse("{section | {#nodes} {#other}}");
    let ids: Vec<_> = doc
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Reference { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect();
    for (index, id) in ids.iter().enumerate() {
        if let CemAstNode::Reference { targets, .. } = &mut doc.nodes[*id as usize] {
            *targets = Some(if index == 0 {
                vec![*id, ids[1], 0]
            } else {
                vec![]
            });
        }
    }
    let payload = DebugBinaryEncoder::new().encode(&doc);
    let restored = DebugBinaryDecoder::new().decode(&payload.bytes).unwrap();
    for id in ids {
        match (doc.get(id), restored.get(id)) {
            (
                Some(CemAstNode::Reference {
                    expression: a,
                    targets: x,
                    context: c,
                    ..
                }),
                Some(CemAstNode::Reference {
                    expression: b,
                    targets: y,
                    context: d,
                    ..
                }),
            ) => {
                assert_eq!((a, x, c), (b, y, d));
            }
            _ => panic!("reference identity changed"),
        }
    }
}

#[test]
fn braces_in_query_strings_do_not_close_reference_islands() {
    let doc = parse("{#nodes['}']} {#nodes[(: } :) true]}");
    let expressions: Vec<_> = doc
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Reference { expression, .. } => Some(expression.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(expressions, vec!["#nodes['}']", "#nodes[(: } :) true]"]);
}

#[test]
fn xml_export_and_debug_projection_preserve_pending_state() {
    let doc = parse("{section | {#nodes}}");
    let output = cem_ml::interpreter::xml::XmlInterpreter::new().render(&doc);
    let tree = import_data_bytes(
        output.rendered.as_bytes(),
        "application/xml",
        "cem",
        "roundtrip.xml",
    )
    .unwrap();
    assert!(tree.ast().nodes.iter().any(|node| matches!(node, CemAstNode::Reference { expression, targets: None, .. } if expression == "#nodes")));
    let debug = cem_ml::projection::dom_json(&doc);
    assert_eq!(debug["children"][0]["children"][1]["kind"], "reference");
}

#[test]
fn xml_reference_preserves_query_payload_provenance_and_following_siblings() {
    use cem_ml::source_map::FrameSpan;
    for payload in ["#nodes", "<![CDATA[#nodes]]>"] {
        let input = format!("<section xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr>{payload}</r:expr><after/></section>");
        let tree =
            import_data_bytes(input.as_bytes(), "application/xml", "cem", "reference.xml").unwrap();
        let query_start = input.find("#nodes").unwrap() as u64;
        let query_end = query_start + "#nodes".len() as u64;
        let reference = tree
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Reference {
                    expression,
                    source,
                    targets,
                    ..
                } => Some((expression, source, targets)),
                _ => None,
            })
            .expect("retained reference");
        assert_eq!(reference.0, "#nodes");
        assert!(reference.2.is_none());
        assert!(
            reference.1.frames.iter().any(|frame| {
                let covers_query = |range: &cem_ml::source::ByteRange| {
                    range.start <= query_start && range.end() >= query_end
                };
                match &frame.span {
                    FrameSpan::Single(range) => covers_query(range),
                    FrameSpan::Multi(ranges) => ranges.iter().any(covers_query),
                }
            }),
            "reference lost query-payload source spans: {:?}",
            reference.1
        );
        let children = tree
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element {
                    expanded_name,
                    children,
                    ..
                } if expanded_name.local_name == "section" => Some(children),
                _ => None,
            })
            .unwrap();
        assert_eq!(children.len(), 2);
        assert!(matches!(
            tree.ast().get(children[0]),
            Some(CemAstNode::Reference { .. })
        ));
        assert!(
            matches!(tree.ast().get(children[1]), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "after")
        );
    }
}

#[test]
fn xml_foreign_expression_vocabulary_does_not_construct_a_reference() {
    let tree = import_data_bytes(
        b"<section xmlns:r='https://example.test/vendor'><r:expr>#nodes</r:expr></section>",
        "application/xml",
        "cem",
        "vendor.xml",
    )
    .unwrap();
    assert!(!tree
        .ast()
        .nodes
        .iter()
        .any(|node| matches!(node, CemAstNode::Reference { .. })));
    assert!(tree.ast().nodes.iter().any(|node| matches!(node, CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "expr" && expanded_name.namespace_uri == "https://example.test/vendor")));
}
