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
fn binary_roundtrip_retains_source_and_non_owning_repeated_edges() {
    let mut doc = parse("{section | {#nodes} {target} {#other} {#empty}}");
    let ids: Vec<_> = doc
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Reference { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect();
    if let CemAstNode::Reference { targets, .. } = &mut doc.nodes[ids[0] as usize] {
        *targets = Some(vec![ids[1], ids[1], ids[0], 0]);
    }
    if let CemAstNode::Reference { targets, .. } = &mut doc.nodes[ids[2] as usize] {
        *targets = Some(vec![]);
    }
    let payload = DebugBinaryEncoder::new().encode(&doc);
    let restored = DebugBinaryDecoder::new().decode(&payload.bytes).unwrap();
    assert_eq!(restored.nodes.len(), doc.nodes.len());
    for id in ids {
        match (doc.get(id), restored.get(id)) {
            (
                Some(CemAstNode::Reference {
                    expression: a,
                    context: b,
                    targets: c,
                    source: d,
                    ..
                }),
                Some(CemAstNode::Reference {
                    expression: x,
                    context: y,
                    targets: z,
                    source: s,
                    ..
                }),
            ) => {
                assert_eq!((a, b, c, d), (x, y, z, s));
                assert!(!s.frames.is_empty());
            }
            _ => panic!("reference occurrence changed"),
        }
    }
    match (doc.get(1), restored.get(1)) {
        (
            Some(CemAstNode::Element { children: a, .. }),
            Some(CemAstNode::Element { children: b, .. }),
        ) => assert_eq!(a, b),
        _ => panic!("structural owner changed"),
    }
}

#[test]
fn binary_decoder_rejects_invalid_reference_context_and_target_handles() {
    use cem_ml::ast::decode::DecodeError;
    for invalid_context in [false, true] {
        let mut doc = parse("{#nodes}");
        let invalid = doc.nodes.len() as u32;
        if let CemAstNode::Reference {
            context, targets, ..
        } = &mut doc.nodes[1]
        {
            if invalid_context {
                *context = invalid;
            } else {
                *targets = Some(vec![invalid]);
            }
        }
        let payload = DebugBinaryEncoder::new().encode(&doc);
        assert!(matches!(DebugBinaryDecoder::new().decode(&payload.bytes),
            Err(DecodeError::InvalidReference(id)) if id == invalid));
    }
}

#[test]
fn version_two_reads_ordinary_nodes_but_rejects_reference_tags() {
    use cem_ml::ast::{
        decode::DecodeError,
        format::{fnv1a64, NodeKindTag},
    };
    for (source, has_reference) in [("{section | {target}}", false), ("{#nodes}", true)] {
        let doc = parse(source);
        // Ordinary-node layout is unchanged by v3. Relabel that layout as v2
        // and recompute integrity; reference tags must remain a v3 extension.
        let mut bytes = DebugBinaryEncoder::new().encode(&doc).bytes;
        bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
        let hash_start = bytes.len() - 8;
        let hash = fnv1a64(&bytes[..hash_start]);
        bytes[hash_start..].copy_from_slice(&hash.to_le_bytes());
        let result = DebugBinaryDecoder::new().decode(&bytes);
        if has_reference {
            assert!(matches!(result, Err(DecodeError::UnknownKindTag(tag))
                if tag == NodeKindTag::Reference as u8));
        } else {
            assert_eq!(result.unwrap().nodes.len(), doc.nodes.len());
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

#[test]
fn cyclic_graph_roundtrip_and_inspection_preserve_edges_without_source_export_expansion() {
    use cem_ml::{
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        projection::{cem_tree_inspection, CemTreeAstNode},
    };
    use std::sync::Arc;
    let text = "{section | {#first} {target} {#second} {#empty} {#pending}}";
    let mut document = parse(text);
    let references: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Reference { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect();
    let target = document
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "target" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let edges = [
        Some(vec![target, references[1], target, references[0]]),
        Some(vec![references[0]]),
        Some(vec![]),
        None,
    ];
    for (&id, edges) in references.iter().zip(&edges) {
        let CemAstNode::Reference { targets, .. } = &mut document.nodes[id as usize] else {
            unreachable!()
        };
        *targets = edges.clone();
    }
    let original_format = cem_ml::formatter::format(&document);
    let bytes = DebugBinaryEncoder::new().encode(&document).bytes;
    // The reference layout is shared by versions 3 and 4; v2 rejection and
    // ordinary-node compatibility are covered by the version fixture above.
    for version in [3u16, 4u16] {
        let mut payload = bytes.clone();
        payload[4..6].copy_from_slice(&version.to_le_bytes());
        let hash_offset = payload.len() - 8;
        let hash = cem_ml::ast::format::fnv1a64(&payload[..hash_offset]);
        payload[hash_offset..].copy_from_slice(&hash.to_le_bytes());
        let restored = DebugBinaryDecoder::new().decode(&payload).unwrap();
        assert_eq!(restored.nodes.len(), document.nodes.len());
        for &id in &references {
            match (document.get(id).unwrap(), restored.get(id).unwrap()) {
                (
                    CemAstNode::Reference {
                        expression: a,
                        context: b,
                        targets: c,
                        source: d,
                        ..
                    },
                    CemAstNode::Reference {
                        expression: e,
                        context: f,
                        targets: g,
                        source: h,
                        ..
                    },
                ) => assert_eq!((a, b, c, d), (e, f, g, h)),
                _ => panic!("reference identity lost"),
            }
        }
        assert_eq!(cem_ml::formatter::format(&restored), original_format);
        let tree = RetainedCemTree::new(
            restored,
            "graph.cem",
            text,
            CemTreeSemantics::default(),
            None,
        )
        .unwrap();
        for (&id, expected) in references.iter().zip(&edges) {
            assert!(tree.node(id).unwrap().children.is_empty());
            let CemAstNode::Reference { targets, .. } = tree.ast().get(id).unwrap() else {
                unreachable!()
            };
            assert_eq!(targets, expected);
        }
        let inspection = cem_tree_inspection(tree.clone());
        assert!(Arc::ptr_eq(inspection.source_owner().unwrap(), &tree));
        let rows = inspection
            .as_nodes()
            .iter()
            .find(|node| node.name() == Some("ast"))
            .unwrap()
            .children();
        assert_eq!(rows.len(), tree.ast().nodes.len());
        for (&id, expected) in references.iter().zip(&edges) {
            let attrs = rows
                .iter()
                .find_map(|row| match row {
                    CemTreeAstNode::Element { attributes, .. }
                        if attributes.iter().any(|a| {
                            a.name == "id"
                                && a.value.as_deref() == Some(format!("node-{id}").as_str())
                        }) =>
                    {
                        Some(attributes)
                    }
                    _ => None,
                })
                .unwrap();
            let value = |name: &str| {
                attrs
                    .iter()
                    .find(|a| a.name == name)
                    .and_then(|a| a.value.as_deref())
            };
            assert_eq!(value("kind"), Some("reference"));
            assert_eq!(
                value("evaluation-state"),
                Some(if expected.is_some() {
                    "resolved"
                } else {
                    "unevaluated"
                })
            );
            let ids = expected
                .as_ref()
                .map(|ids| ids.iter().map(u32::to_string).collect::<Vec<_>>().join(" "));
            assert_eq!(value("target-ids"), ids.as_deref());
        }
        let dom = cem_ml::projection::dom_json(tree.ast());
        let refs: Vec<_> = dom["children"][0]["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["kind"] == "reference")
            .collect();
        assert_eq!(refs.len(), references.len());
        assert_eq!(refs[0]["targets"], serde_json::json!(edges[0]));
        assert_eq!(refs[2]["targets"], serde_json::json!([]));
        assert!(refs[3]["targets"].is_null());
        assert!(refs.iter().all(|node| node.get("children").is_none()));
        // Normalized events inspect source syntax, not the saved AST graph.
        let events = cem_ml::projection::NormalizedEventStream::from_source(
            original_format.as_bytes(),
            cem_ml::engine::InputFormat::Cem,
        );
        let payloads: Vec<_> = events
            .as_events()
            .iter()
            .filter_map(|event| match event {
                cem_ml::events::NormalizedEvent::Value {
                    value: cem_ml::events::ScalarValue::Text(text),
                    ..
                } if text.starts_with('#') => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(payloads, ["#first", "#second", "#empty", "#pending"]);
        assert_eq!(events.as_events().iter().filter(|event| matches!(event,
            cem_ml::events::NormalizedEvent::OpenScope { name, .. } if name.lexical_name == "$"
        )).count(), references.len());
        // Text exports are source conventions: they discard optional target
        // metadata and reconstruct fresh occurrences, without following cycles.
        for surface in [
            cem_ml::formatter::format(tree.ast()),
            cem_ml::interpreter::xml::XmlInterpreter::new()
                .render(tree.ast())
                .rendered,
        ] {
            let exported = if surface.starts_with('<') {
                import_data_bytes(surface.as_bytes(), "application/xml", "cem", "export.xml")
                    .unwrap()
                    .ast_owner()
                    .clone()
            } else {
                Arc::new(parse(&surface))
            };
            let exported_refs: Vec<_> = exported
                .nodes
                .iter()
                .filter_map(|n| match n {
                    CemAstNode::Reference {
                        expression,
                        targets,
                        ..
                    } => {
                        assert!(targets.is_none());
                        Some(expression.as_str())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(exported_refs, ["#first", "#second", "#empty", "#pending"]);
            assert_eq!(exported.nodes.iter().filter(|n| matches!(n, CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "target")).count(), 1);
        }
    }
}
