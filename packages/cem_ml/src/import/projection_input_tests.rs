use super::*;
use crate::{
    engine::{EngineContext, EngineInput, FormatIdentity, InputFormat},
    lifecycle::LifecycleRegistry,
    validation::{
        cem_ast_projection as ast, cem_dom_projection as dom, cem_events_projection as events,
    },
};

fn source() -> &'static [u8] {
    "@doc cem-ml 1\n{p @title=Ready | Café & tea}\n".as_bytes()
}
fn artifacts() -> Vec<crate::projection::BinaryProjectionArtifact> {
    let doc =
        crate::parser::builder::CemAstBuilder::new(crate::events::cem::CemEventNormalizer::new(
            crate::tokenizer::cem::CemTokenizer::from_source(crate::source::BytesSource::new(
                SourceId(1),
                source().to_vec(),
            )),
        ))
        .build();
    vec![
        crate::projection::dom_binary_projection_artifact(&doc),
        crate::projection::ast_binary_projection_artifact(&doc),
        crate::projection::events_binary_projection_artifact_as(source(), InputFormat::Cem),
    ]
}
fn validate(bytes: &[u8], mime: &str, family: &str) -> Vec<Diagnostic> {
    match family {
        "dom" => dom::validate_cem_dom_projection_source_bytes(
            dom::CemDomProjectionSourceValidationRequest {
                bytes,
                content_type: Some(mime),
                source_uri: "studio://projection/input",
            },
        ),
        "ast" => ast::validate_cem_ast_projection_source_bytes(
            ast::CemAstProjectionSourceValidationRequest {
                bytes,
                content_type: Some(mime),
                source_uri: "studio://projection/input",
            },
        ),
        _ => events::validate_cem_events_projection_source_bytes(
            events::CemEventsProjectionSourceValidationRequest {
                bytes,
                content_type: Some(mime),
                source_uri: "studio://projection/input",
            },
        ),
    }
}
#[test]
fn projection_payload_validation_rejects_every_truncation_and_trailing_bytes() {
    for artifact in artifacts() {
        assert!(validate(
            &artifact.bytes,
            &artifact.content_type,
            &artifact.projection
        )
        .is_empty());
        for length in 0..artifact.bytes.len() {
            assert!(
                !validate(
                    &artifact.bytes[..length],
                    &artifact.content_type,
                    &artifact.projection
                )
                .is_empty(),
                "{} accepted truncation {length}",
                artifact.projection
            );
        }
        let mut extra = artifact.bytes.clone();
        extra.push(0);
        assert!(
            !validate(&extra, &artifact.content_type, &artifact.projection).is_empty(),
            "{} accepted trailing bytes",
            artifact.projection
        );
    }
}
#[test]
fn projection_envelopes_require_exact_hashes_payload_and_chunk_membership() {
    for artifact in artifacts() {
        let mime = artifact.content_type.replace("+cem-bin", "+json");
        let envelope = artifact.to_json_envelope();
        assert!(validate(
            &serde_json::to_vec(&envelope).unwrap(),
            &mime,
            &artifact.projection
        )
        .is_empty());
        for field in ["hash", "byteLength", "chunks"] {
            let mut invalid = envelope.clone();
            invalid[field] = match field {
                "hash" => serde_json::json!("forged"),
                "byteLength" => serde_json::json!(1),
                _ => serde_json::json!([]),
            };
            assert!(
                !validate(
                    &serde_json::to_vec(&invalid).unwrap(),
                    &mime,
                    &artifact.projection
                )
                .is_empty(),
                "{} accepted forged {field}",
                artifact.projection
            );
        }
        let mut invalid = envelope.clone();
        invalid["chunks"][0]["hash"] = serde_json::json!("forged");
        assert!(!validate(
            &serde_json::to_vec(&invalid).unwrap(),
            &mime,
            &artifact.projection
        )
        .is_empty());
    }
}
#[test]
fn projection_lifecycle_sources_enter_shared_import_without_cem_tokenizer_fallback() {
    for artifact in artifacts() {
        let input = EngineInput {
            uri: "studio://projection/input".into(),
            bytes: artifact.bytes.clone(),
            from_format: None,
            identity: Some(FormatIdentity {
                content_type: Some(artifact.content_type.clone()),
                schema: Some(artifact.schema.clone()),
                ..Default::default()
            }),
            root_scope: Default::default(),
        };
        let loaded =
            LifecycleRegistry::with_builtin_adapters().load(&input, &EngineContext::default());
        assert!(
            loaded.ast_stream.is_some(),
            "{} input adapter missing: {:?}",
            artifact.projection,
            loaded.diagnostics
        );
        let owner = retain_lifecycle(Arc::new(loaded.ast_stream.unwrap()))
            .expect("native projection import");
        assert_eq!(owner.source_uri(), input.uri);
        assert!(owner.native_owner().is_some());
        assert!(
            import_data_bytes(&artifact.bytes, &artifact.content_type, "cem", &input.uri).is_ok()
        );
    }
}

#[test]
fn projection_json_sources_keep_native_owner_and_actual_artifact_ranges() {
    for family in ["dom", "ast", "events"] {
        let mime = format!("application/vnd.cem.{family}+json");
        let input = match family {
            "dom" => include_bytes!(
                "../../../../examples/projects/native-projections/projections/dispatch.dom.json"
            )
            .as_slice(),
            "ast" => include_bytes!(
                "../../../../examples/projects/native-projections/projections/dispatch.ast.json"
            )
            .as_slice(),
            _ => include_bytes!(
                "../../../../examples/projects/native-projections/projections/dispatch.events.json"
            )
            .as_slice(),
        };
        for uri in [
            "studio://first/projection.json",
            "studio://second/projection.json",
        ] {
            let tree = import_data_bytes(input, &mime, "cem", uri).unwrap();
            assert_eq!(tree.source_uri(), uri);
            assert!(tree.native_owner().is_some());
            for node in tree.ast().iter() {
                let map = match node {
                    CemAstNode::Document { source, .. }
                    | CemAstNode::Element { source, .. }
                    | CemAstNode::Attribute { source, .. }
                    | CemAstNode::Reference { source, .. }
                    | CemAstNode::Text { source, .. }
                    | CemAstNode::Whitespace { source, .. }
                    | CemAstNode::Comment { source, .. }
                    | CemAstNode::Cdata { source, .. }
                    | CemAstNode::RawText { source, .. }
                    | CemAstNode::ProcessingInstruction { source, .. }
                    | CemAstNode::Error { source, .. } => source,
                };
                for frame in &map.frames {
                    assert!(
                        matches!(&frame.transform,TransformKind::SourceBoundary{uri:origin}if origin==uri)
                    );
                    let FrameSpan::Single(r) = frame.span else {
                        panic!("range")
                    };
                    assert!(r.end() <= input.len() as u64);
                    std::str::from_utf8(&input[r.start as usize..r.end() as usize]).unwrap();
                }
            }
        }
    }
}

#[test]
fn projection_native_parse_inspect_and_conversion_use_imported_owners() {
    use crate::engine::*;
    let engine = crate::real::RealCemMlEngine;
    for artifact in artifacts() {
        let variants = [
            (artifact.bytes.clone(), artifact.content_type.clone()),
            (
                serde_json::to_vec(&artifact.to_json_envelope()).unwrap(),
                artifact.content_type.replace("+cem-bin", "+json"),
            ),
        ];
        for (bytes, mime) in variants {
            let input = EngineInput {
                uri: "studio://projection/input".into(),
                bytes,
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some(mime),
                    schema: Some(artifact.schema.clone()),
                    ..Default::default()
                }),
                root_scope: Default::default(),
            };
            for projection in [ParseProjection::Ast, ParseProjection::Events] {
                let result = engine
                    .parse(ParseRequest {
                        input: input.clone(),
                        projection,
                        fail_level: FailLevel::Validate,
                        presentation_scope: Some(Default::default()),
                        preserve_source_offsets: true,
                        context: EngineContext::default(),
                    })
                    .unwrap();
                assert!(
                    !result
                        .diagnostics
                        .iter()
                        .any(|d| d.severity.is_hard_violation()),
                    "{:?}",
                    result.diagnostics
                );
                assert!(result.primary_bytes.is_some());
            }
            let result = engine
                .inspect(InspectRequest {
                    input: input.clone(),
                    show: InspectView::Ast,
                    presentation_scope: Some(Default::default()),
                    context: EngineContext::default(),
                })
                .unwrap();
            assert!(
                !result
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation()),
                "{:?}",
                result.diagnostics
            );
            assert!(result.primary_bytes.is_some());
            for target in [LayerFormat::Cem, LayerFormat::Xml, LayerFormat::Html] {
                let result = engine
                    .convert(ConvertRequest {
                        input: input.clone(),
                        to_format: target,
                        preserve_source_offsets: true,
                        context: EngineContext::default(),
                        target: None,
                        target_scope: Default::default(),
                        scheduler_scope_id: 0,
                    })
                    .unwrap();
                assert!(
                    !result
                        .diagnostics
                        .iter()
                        .any(|d| d.severity.is_hard_violation()),
                    "{} {target:?}: {:?}",
                    artifact.projection,
                    result.diagnostics
                );
                let primary = result.primary_bytes.expect("native output");
                let text = std::str::from_utf8(&primary.bytes).unwrap();
                assert!(
                    text.contains("Café"),
                    "{} {target:?}: {text}",
                    artifact.projection
                );
            }
        }
    }
}

#[test]
fn projection_events_preserve_passive_expression_and_prelude_values() {
    let input = br#"[{"kind":"value","value":"$secret","valueSyntax":"expression","byteRange":{"start":500,"len":7}},
      {"kind":"value","value":{"role":"Namespace","prefix":"demo","role_range":{"start":0,"len":1},"prefix_range":null,"value_range":{"start":1,"len":1},"payload_range":{"start":1,"len":1},"kind":"Expression","expression":"$namespace","error":null},"byteRange":{"start":0,"len":2}}]"#;
    let doc = projection::load(
        input,
        "studio://events.json",
        Some("application/vnd.cem.events+json"),
        projection::ProjectionKind::Events,
    )
    .unwrap();
    let attrs: Vec<_> = doc
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|n| {
            if let CemAstNode::Attribute {
                expanded_name,
                value,
                ..
            } = n
            {
                Some((
                    expanded_name.local_name.as_str(),
                    value.as_deref().unwrap_or(""),
                ))
            } else {
                None
            }
        })
        .collect();
    for pair in [
        ("value-kind", "expression"),
        ("value", "$secret"),
        ("reported-offset", "500"),
        ("value-kind", "typed-prelude"),
        ("prelude-role", "namespace"),
        ("prelude-kind", "expression"),
        ("prefix", "demo"),
        ("value", "$namespace"),
    ] {
        assert!(attrs.contains(&pair), "{pair:?}: {attrs:?}");
    }
    assert!(!doc
        .tree
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { .. })));
}
#[test]
fn projection_refuses_forged_chunk_topology_and_payload_structure() {
    for artifact in artifacts() {
        let kind = projection::ProjectionKind::identity(Some(&artifact.schema), None).unwrap();
        let mime = kind.json_mime();
        for (field, value) in [
            ("parentId", serde_json::json!("node-0")),
            ("rootId", serde_json::json!("forged")),
            ("childLinks", serde_json::json!([])),
        ] {
            let mut e = artifact.to_json_envelope();
            e["chunks"][0][field] = value;
            assert!(
                projection::load(
                    &serde_json::to_vec(&e).unwrap(),
                    "studio://input",
                    Some(mime),
                    kind
                )
                .is_err(),
                "{} forged {field}",
                artifact.projection
            );
        }
        let mut e = artifact.to_json_envelope();
        e["chunks"][1]["childLinks"] = serde_json::json!([{"chunkId":"root","rootId":format!("projection:{}",artifact.projection)}]);
        assert!(projection::load(
            &serde_json::to_vec(&e).unwrap(),
            "studio://input",
            Some(mime),
            kind
        )
        .is_err());
        // Change payload tags, count, record ID/sequence, reported range and UTF-8.
        let header = artifact.chunk_metadata[0].byte_length;
        let mut corruptions = vec![];
        for (offset, bytes) in [
            (8, vec![0, 99]),
            (10, vec![99]),
            (header - 4, u32::MAX.to_be_bytes().to_vec()),
            (header, vec![255]),
        ] {
            let mut b = artifact.bytes.clone();
            b[offset..offset + bytes.len()].copy_from_slice(&bytes);
            corruptions.push(b);
        }
        if kind != projection::ProjectionKind::Events {
            let mut b = artifact.bytes.clone();
            b[header + 1..header + 5].copy_from_slice(&99u32.to_be_bytes());
            corruptions.push(b);
            let mut b = artifact.bytes.clone();
            b[header + 5] = 2;
            corruptions.push(b);
            let mut b = artifact.bytes.clone();
            let child_count = header + 6 + if b[header + 5] == 1 { 12 } else { 0 };
            if u32::from_be_bytes(b[child_count..child_count + 4].try_into().unwrap()) > 0 {
                b[child_count + 4..child_count + 8].copy_from_slice(&0u32.to_be_bytes());
                corruptions.push(b);
            }
        }
        if let Some(offset) = artifact
            .bytes
            .windows(5)
            .position(|v| v == "Café".as_bytes())
        {
            let mut b = artifact.bytes.clone();
            b[offset + 3] = 255;
            corruptions.push(b);
        }
        for b in corruptions {
            assert!(
                projection::load(&b, "studio://input", Some(kind.binary_mime()), kind).is_err(),
                "{} malformed payload accepted",
                artifact.projection
            );
        }
    }
}
#[test]
fn projection_reference_handles_and_reported_source_maps_are_not_restored() {
    let input=br#"{"kind":"document","children":[{"kind":"element","name":"item","namespace":"urn:demo","schemaId":4294967295,"attributes":[],"children":[{"kind":"reference","expression":"$private","context":4294967295,"targets":[4294967295],"sourceMap":{"uri":"file:///secret","start":999999},"byteRange":{"start":999999,"len":3}}]}]}"#;
    for mime in [
        "application/vnd.cem.dom+json",
        "application/vnd.cem.ast+json",
    ] {
        let owner = import_data_bytes(input, mime, "cem", "studio://actual.json").unwrap();
        assert!(owner.ast().nodes.iter().any(|n| matches!(
            n,
            CemAstNode::Reference {
                context: 0,
                targets: None,
                ..
            }
        )));
        for n in &owner.ast().nodes {
            if let CemAstNode::Element { expanded_name, .. } = n {
                assert!(expanded_name.schema_id.is_none());
            }
        }
        assert_eq!(
            owner.source_node_range(2).unwrap().offset,
            input
                .windows(19)
                .position(|s| s == b"{\"kind\":\"reference\"")
                .unwrap() as u64
        );
        assert!(import_data(
            std::str::from_utf8(input).unwrap(),
            mime,
            "cem",
            "studio://string.json"
        )
        .unwrap()
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { .. })));
        let kind = projection::ProjectionKind::identity(None, Some(mime)).unwrap();
        assert!(projection::load(
            input,
            "studio://actual.json",
            Some(&format!("{mime};charset=iso-8859-1")),
            kind
        )
        .is_err());
    }
}

#[test]
fn projection_namespace_exports_remain_structural_without_schema_grants() {
    use crate::engine::*;
    let bytes=br#"{"kind":"document","children":[{"kind":"element","name":"item","namespace":"urn:example","attributes":[{"name":"code","namespace":"urn:attr","value":"Ready"}],"children":[{"kind":"text","data":"safe"}]}]}"#;
    let input = EngineInput {
        uri: "studio://names.json".into(),
        bytes: bytes.to_vec(),
        identity: Some(FormatIdentity {
            schema: Some("https://cem.dev/ns/projection/ast/1".into()),
            content_type: Some("application/vnd.cem.ast+json".into()),
            ..Default::default()
        }),
        from_format: None,
        root_scope: Default::default(),
    };
    let engine = crate::real::RealCemMlEngine;
    for target in [
        LayerFormat::Cem,
        LayerFormat::Xml,
        LayerFormat::Ast,
        LayerFormat::DomJson,
        LayerFormat::AstBin,
        LayerFormat::DomBin,
    ] {
        let output = engine
            .convert(ConvertRequest {
                input: input.clone(),
                to_format: target,
                preserve_source_offsets: true,
                context: EngineContext::default(),
                target: None,
                target_scope: Default::default(),
                scheduler_scope_id: 0,
            })
            .unwrap();
        assert!(
            !output
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{target:?}: {:?}",
            output.diagnostics
        );
        let bytes = output.primary_bytes.unwrap();
        let tree = if target == LayerFormat::Cem {
            let round_trip = engine
                .convert(ConvertRequest {
                    input: EngineInput {
                        uri: "studio://round-trip.cem".into(),
                        bytes: bytes.bytes,
                        identity: Some(FormatIdentity {
                            content_type: Some(bytes.content_type),
                            ..Default::default()
                        }),
                        from_format: None,
                        root_scope: Default::default(),
                    },
                    to_format: LayerFormat::AstBin,
                    preserve_source_offsets: true,
                    context: Default::default(),
                    target: None,
                    target_scope: Default::default(),
                    scheduler_scope_id: 0,
                })
                .unwrap();
            // Exporting a passive namespace never grants an application schema.
            assert!(round_trip
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.schema.unresolved_namespace"));
            assert!(round_trip
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.schema.unresolved_namespace"
                    && d.message.contains("urn:example")));
            // The application validator resolved the generated declaration;
            // raw debug AST export has its separate lexical-prefix contract.
            continue;
        } else if target == LayerFormat::Xml {
            let loaded = LifecycleRegistry::with_builtin_adapters().load(
                &EngineInput {
                    uri: "studio://round-trip".into(),
                    bytes: bytes.bytes,
                    identity: Some(FormatIdentity {
                        content_type: Some(bytes.content_type),
                        ..Default::default()
                    }),
                    from_format: None,
                    root_scope: Default::default(),
                },
                &EngineContext::default(),
            );
            assert!(
                !loaded
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation()),
                "{:?}",
                loaded.diagnostics
            );
            retain_lifecycle(Arc::new(loaded.ast_stream.unwrap())).unwrap()
        } else {
            import_data_bytes(
                &bytes.bytes,
                &bytes.content_type,
                "cem",
                "studio://round-trip",
            )
            .unwrap()
        };
        assert!(tree.ast().nodes.iter().any(|n|matches!(n,CemAstNode::Element{expanded_name,..}if expanded_name.namespace_uri=="urn:example" && expanded_name.local_name=="item")),"{target:?}: {:?}",tree.ast());
    }
}
#[test]
fn projection_source_identity_refusals_never_fall_back_to_cem_text() {
    use crate::engine::*;
    let engine = crate::real::RealCemMlEngine;
    for (mime, schema) in [
        (
            "application/vnd.cem.ast+json",
            "https://cem.dev/ns/projection/dom/1",
        ),
        ("application/xml", "https://cem.dev/ns/projection/events/1"),
    ] {
        let input = EngineInput {
            uri: "studio://refused.json".into(),
            bytes: b"{p | not a projection}".to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(mime.into()),
                schema: Some(schema.into()),
                ..Default::default()
            }),
            from_format: None,
            root_scope: Default::default(),
        };
        let parsed = engine
            .parse(ParseRequest {
                input: input.clone(),
                projection: ParseProjection::Ast,
                preserve_source_offsets: true,
                fail_level: FailLevel::Validate,
                presentation_scope: Some(Default::default()),
                context: Default::default(),
            })
            .unwrap();
        assert!(parsed
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        assert!(parsed.primary_bytes.is_none());
        let converted = engine
            .convert(ConvertRequest {
                input,
                to_format: LayerFormat::Html,
                preserve_source_offsets: true,
                context: Default::default(),
                target: None,
                target_scope: Default::default(),
                scheduler_scope_id: 0,
            })
            .unwrap();
        assert!(converted
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        assert!(converted.primary_bytes.is_none());
    }
}

#[test]
fn projection_reference_exports_keep_a_declared_passive_namespace() {
    use crate::engine::*;
    let bytes=br#"{"kind":"document","children":[{"kind":"element","name":"item","attributes":[],"children":[{"kind":"reference","expression":"$private","context":1,"targets":[99]}]}]}"#;
    let input = EngineInput {
        uri: "studio://reference.json".into(),
        bytes: bytes.to_vec(),
        identity: Some(FormatIdentity {
            content_type: Some("application/vnd.cem.ast+json".into()),
            ..Default::default()
        }),
        from_format: None,
        root_scope: Default::default(),
    };
    let engine = crate::real::RealCemMlEngine;
    let output = engine
        .convert(ConvertRequest {
            input,
            to_format: LayerFormat::Xml,
            preserve_source_offsets: true,
            context: Default::default(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        })
        .unwrap();
    let primary = output.primary_bytes.unwrap();
    assert!(
        !output
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        output.diagnostics
    );
    let tree = import_data_bytes(
        &primary.bytes,
        "application/xml",
        "cem",
        "studio://reference.xml",
    )
    .unwrap();
    assert!(tree.ast().nodes.iter().any(|n|matches!(n,CemAstNode::Element{expanded_name,..}if expanded_name.namespace_uri=="https://cem.dev/ns/cem-ml/1" && expanded_name.local_name=="expr")));
    assert!(tree
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n,CemAstNode::Text{data,..}if data=="$private")));
    assert!(!tree.ast().nodes.iter().any(|n| matches!(
        n,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn projection_json_limits_apply_during_native_parsing_without_relaxing_strings() {
    use crate::validation::json::*;
    for bytes in [b"[0,1,2,3,4]".as_slice(), b"[[[[[[0]]]]]]".as_slice()] {
        let (doc, diagnostics) = json_document_ast_from_source_bytes_bounded(
            JsonSourceValidationRequest {
                bytes,
                source_uri: "studio://bounded.json",
                content_type: Some("application/json"),
            },
            4,
            4,
        );
        assert!(doc.is_none());
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "cem.json.resource_limit"
                && d.uri.as_deref() == Some("studio://bounded.json")));
    }
    let bytes = br#"{"kind":"document","children":[{"kind":"text","data":"\ud800"}]}"#;
    assert!(projection::load(
        bytes,
        "studio://strict.json",
        Some("application/vnd.cem.ast+json"),
        projection::ProjectionKind::Ast
    )
    .is_err());
}
