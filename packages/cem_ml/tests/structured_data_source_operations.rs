use cem_ml::{
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, FormatIdentity, InspectRequest,
        InspectView, ParseProjection, ParseRequest,
    },
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, CemAstNode},
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
fn source(bytes: &str, mime: &str, schema: &str) -> EngineInput {
    EngineInput {
        uri: "memory:original/source".into(),
        bytes: bytes.as_bytes().into(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(mime.into()),
            schema: Some(schema.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    }
}
fn scope() -> ScopeConfig {
    ScopeConfig {
        cemt_formatter_profile: Some("tabular".into()),
        output_color_type: Some("none".into()),
        ..Default::default()
    }
}
fn document(bytes: Vec<u8>) -> cem_ml::parser::document::CemDocument {
    let doc = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(7), bytes),
    )))
    .build();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    doc
}
fn attributes(doc: &cem_ml::parser::document::CemDocument, local_name: &str) -> Vec<String> {
    doc.nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Attribute {
                expanded_name,
                value: Some(value),
                ..
            } if expanded_name.local_name == local_name => Some(value.clone()),
            _ => None,
        })
        .collect()
}
#[test]
fn structured_source_parse_preserves_source_identity_and_literal_data() {
    for (text,mime,schema) in [
        ("{\r\n  \"literal\": \"{#never} {$never()} Café 🧬\", \"n\": 1.20e+2, \"none\": null\r\n}","application/json","https://cem.dev/ns/data/json/1"),
        ("literal: '{#never} {$never()} Café 🧬'\r\nn: 120\r\nnone: null\r\n","application/yaml","https://cem.dev/ns/data/yaml/1"),
        (r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"note":{"type":"string"}},"properties":{"note":{"$ref":"#/$defs/note"}},"description":"{#never} {$never()} Café 🧬"}"##,"application/schema+json","https://cem.dev/ns/data/json-schema/1"),
    ] {
        let response = RealCemMlEngine::new().parse(ParseRequest { input: source(text,mime,schema), projection: ParseProjection::Ast,
            fail_level: FailLevel::Validate, preserve_source_offsets: true, presentation_scope: Some(scope()), context: EngineContext::default() }).unwrap();
        assert!(!response.diagnostics.iter().any(|d| d.severity.is_hard_violation()), "{mime}: {:?}",response.diagnostics);
        let primary=response.primary_bytes.expect("typed source presentation");
        assert_eq!(primary.schema.as_deref(),Some("https://cem.dev/ns/projection/ast/1"));
        let doc=document(primary.bytes);
        assert!(attributes(&doc,"schema").contains(&schema.into()),"{mime}: {:?}",attributes(&doc,"schema"));
        assert!(attributes(&doc,"content-type").contains(&mime.into()));
        assert!(attributes(&doc,"source-id").contains(&"memory:original/source".into()));
        assert!(attributes(&doc,"value").iter().any(|value|value=="{#never} {$never()} Café 🧬"));
        assert!(doc.nodes.iter().all(|node| match node { CemAstNode::Attribute {value_nodes,..}=>value_nodes.is_empty(),_=>true }));
        for show in [InspectView::Ast,InspectView::Tree,InspectView::Events,InspectView::Diagnostics] {
            let response=RealCemMlEngine::new().inspect(InspectRequest { input: source(text,mime,schema),show,presentation_scope:Some(scope()),context:EngineContext::default() }).unwrap();
            let text=String::from_utf8(response.primary_bytes.unwrap().bytes).unwrap();
            assert!(!text.contains("cem.tokenizer.")&&!text.contains("cem.ql."),"{mime}: {text}");
            assert!(text.contains(schema)&&text.contains(mime),"{text}");
        }
    }
}
#[test]
fn malformed_data_keeps_original_native_diagnostics_and_has_no_partial_parse_tree() {
    for (text, mime, schema, prefix) in [
        (
            "{\r\n\"café\": [1, }",
            "application/json",
            "https://cem.dev/ns/data/json/1",
            "cem.json.",
        ),
        (
            "café: [1, 2\r\n",
            "application/yaml",
            "https://cem.dev/ns/data/yaml/1",
            "cem.yaml.",
        ),
        (
            "{\r\n\"$defs\": [}",
            "application/schema+json",
            "https://cem.dev/ns/data/json-schema/1",
            "cem.json_schema.",
        ),
    ] {
        let response = RealCemMlEngine::new()
            .parse(ParseRequest {
                input: source(text, mime, schema),
                projection: ParseProjection::Ast,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: true,
                presentation_scope: Some(scope()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation() && d.code.starts_with(prefix)),
            "{:?}",
            response.diagnostics
        );
        assert!(
            !response
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("cem.tokenizer.")),
            "{:?}",
            response.diagnostics
        );
        assert!(response
            .diagnostics
            .iter()
            .all(|d| d.uri.as_deref() == Some("memory:original/source")));
        assert!(response.primary_bytes.is_none());
        assert!(response.primary.is_null());
    }
}

#[test]
fn source_aliases_events_ranges_and_explicit_dom_exports_share_the_import_owner() {
    for (text, mime, schema, literal_start) in [
        (
            "{\r\n  \"x\": \"Café 🧬\"\r\n}",
            "text/json; charset=utf-8",
            "https://cem.dev/ns/data/json/1",
            10,
        ),
        (
            "x: 'Café 🧬'\r\n",
            "text/x-yaml; charset=utf-8",
            "https://cem.dev/ns/data/yaml/1",
            3,
        ),
        (
            r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"note":{"type":"string"}},"$ref":"#/$defs/note"}"##,
            "application/json; charset=utf-8",
            "https://cem.dev/ns/data/json-schema/1",
            0,
        ),
    ] {
        for projection in [
            ParseProjection::Ast,
            ParseProjection::Events,
            ParseProjection::DomJson,
        ] {
            let result = RealCemMlEngine::new()
                .parse(ParseRequest {
                    input: source(text, mime, schema),
                    projection,
                    fail_level: FailLevel::Validate,
                    preserve_source_offsets: true,
                    presentation_scope: Some(scope()),
                    context: EngineContext::default(),
                })
                .unwrap();
            assert!(
                result.diagnostics.is_empty(),
                "{mime}: {:?}",
                result.diagnostics
            );
            assert!(!result.primary.is_null());
            if projection == ParseProjection::DomJson {
                assert!(result.primary_bytes.is_none());
                continue;
            }
            let primary = result.primary_bytes.unwrap();
            let doc = document(primary.bytes);
            assert!(attributes(&doc, "content-type").contains(&mime.into()));
            assert!(attributes(&doc, "schema").contains(&schema.into()));
            if literal_start > 0 {
                assert!(
                    attributes(&doc, "byte-offset").contains(&literal_start.to_string()),
                    "{mime}: {:?}",
                    attributes(&doc, "byte-offset")
                );
            }
            if projection == ParseProjection::Events {
                let kinds = attributes(&doc, "kind");
                assert!(
                    kinds.contains(&"open".into())
                        && kinds.contains(&"value".into())
                        && attributes(&doc, "node-kind").contains(&"attribute".into())
                );
                assert_eq!(
                    kinds.iter().filter(|k| k.as_str() == "open").count(),
                    kinds.iter().filter(|k| k.as_str() == "close").count()
                );
            }
        }
    }
}
#[test]
fn unsupported_yaml_and_import_depth_refuse_without_fallback_or_partial_trees() {
    let deep = format!("{}0{}", "[".repeat(66), "]".repeat(66));
    for (text, mime, schema) in [
        (
            "first: &value 1\nsecond: *value\n",
            "application/yaml",
            "https://cem.dev/ns/data/yaml/1",
        ),
        (
            deep.as_str(),
            "application/json",
            "https://cem.dev/ns/data/json/1",
        ),
        (
            deep.as_str(),
            "application/schema+json",
            "https://cem.dev/ns/data/json-schema/1",
        ),
    ] {
        let result = RealCemMlEngine::new()
            .parse(ParseRequest {
                input: source(text, mime, schema),
                projection: ParseProjection::Ast,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: true,
                presentation_scope: Some(scope()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(result.primary.is_null() && result.primary_bytes.is_none());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{:?}",
            result.diagnostics
        );
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code.starts_with("cem.tokenizer.")));
        let input = source(text, mime, schema);
        assert!(RealCemMlEngine::new()
            .inspect(InspectRequest {
                input: input.clone(),
                show: InspectView::Ast,
                presentation_scope: Some(scope()),
                context: EngineContext::default()
            })
            .is_err());
        let diagnostics = RealCemMlEngine::new()
            .inspect(InspectRequest {
                input,
                show: InspectView::Diagnostics,
                presentation_scope: Some(scope()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(diagnostics.body["diagnostics"]
            .as_array()
            .is_some_and(|d| !d.is_empty()));
    }
}

#[test]
fn json_schema_import_retains_its_native_owner_and_original_number_lexeme() {
    use cem_ml::{
        lifecycle::LoadedInputAstStream,
        validation::json_schema::{
            json_schema_document_ast_from_source_bytes, JsonSchemaSourceValidationRequest,
        },
    };
    use std::sync::Arc;
    let text = r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"limit":{"minimum":1.20e+2}},"$ref":"#/$defs/limit"}"##;
    let (native, diagnostics) =
        json_schema_document_ast_from_source_bytes(JsonSchemaSourceValidationRequest {
            bytes: text.as_bytes(),
            source_uri: "memory:original/schema",
            content_type: Some("application/schema+json"),
        });
    assert!(diagnostics.is_empty());
    let tree = cem_ml::import::try_retain_lifecycle(Arc::new(
        LoadedInputAstStream::JsonSchemaDocument(native.unwrap()),
    ))
    .unwrap()
    .unwrap();
    let Some(LoadedInputAstStream::JsonSchemaDocument(native)) =
        tree.native_owner().unwrap().downcast_ref()
    else {
        panic!("native schema owner was lost")
    };
    assert_eq!(native.source.uri, "memory:original/schema");
    assert_eq!(native.source.byte_length, text.len());
    assert!(format!("{:?}", native.json).contains("1.20e+2"));
    assert!(tree
        .ast()
        .iter()
        .all(|node| !matches!(node, CemAstNode::Reference { .. })));
}

#[test]
fn mime_only_inputs_infer_native_source_schema_without_changing_projection_identity() {
    for (text, mime, schema) in [
        ("null", "application/json", "https://cem.dev/ns/data/json/1"),
        ("[]", "text/x-yaml", "https://cem.dev/ns/data/yaml/1"),
        (
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema"}"#,
            "application/schema+json",
            "https://cem.dev/ns/data/json-schema/1",
        ),
    ] {
        let mut input = source(text, mime, schema);
        input.identity.as_mut().unwrap().schema = None;
        let result = RealCemMlEngine::new()
            .parse(ParseRequest {
                input,
                projection: ParseProjection::Ast,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: true,
                presentation_scope: Some(scope()),
                context: EngineContext::default(),
            })
            .unwrap();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let primary = result.primary_bytes.unwrap();
        assert_eq!(
            primary.schema.as_deref(),
            Some("https://cem.dev/ns/projection/ast/1")
        );
        let doc = document(primary.bytes);
        assert!(attributes(&doc, "schema").contains(&schema.into()));
        assert!(attributes(&doc, "content-type").contains(&mime.into()));
    }
}

#[test]
fn parse_budget_diagnostics_keep_the_original_source_uri() {
    let mut input = source("null", "application/json", "https://cem.dev/ns/data/json/1");
    input
        .root_scope
        .budgets
        .insert("parseMs".into(), "0".into());
    let result = RealCemMlEngine::new()
        .parse(ParseRequest {
            input,
            projection: ParseProjection::Ast,
            fail_level: FailLevel::Validate,
            preserve_source_offsets: true,
            presentation_scope: Some(scope()),
            context: EngineContext::default(),
        })
        .unwrap();
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "cem.scope.budget_exceeded")
        .expect("zero time budget must report an overrun");
    assert_eq!(diagnostic.uri.as_deref(), Some("memory:original/source"));
    assert!(diagnostic.severity.is_hard_violation());
}

#[test]
fn imported_events_conform_to_the_registered_event_kind_vocabulary() {
    let contract = document(
        include_bytes!(
            "../schema-packages/cem-events-projection/v1/schema/cem-events-projection.cem"
        )
        .to_vec(),
    );
    let allowed = contract
        .iter()
        .find_map(|node| {
            let CemAstNode::Element {
                attributes: ids, ..
            } = node
            else {
                return None;
            };
            let fields: Vec<_> = ids
                .iter()
                .filter_map(|id| match contract.get(*id) {
                    Some(CemAstNode::Attribute {
                        expanded_name,
                        value: Some(value),
                        ..
                    }) => Some((expanded_name.local_name.as_str(), value.as_str())),
                    _ => None,
                })
                .collect();
            (fields
                .iter()
                .any(|(name, value)| *name == "name" && *value == "kind"))
            .then(|| {
                fields
                    .iter()
                    .find(|(name, _)| *name == "values")
                    .map(|(_, v)| v.to_string())
            })
            .flatten()
        })
        .expect("registered kind enum");
    let result = RealCemMlEngine::new()
        .parse(ParseRequest {
            input: source(
                "{\"text\":\"Café 🧬\"}",
                "application/json",
                "https://cem.dev/ns/data/json/1",
            ),
            projection: ParseProjection::Events,
            fail_level: FailLevel::Validate,
            preserve_source_offsets: true,
            presentation_scope: Some(scope()),
            context: EngineContext::default(),
        })
        .unwrap();
    let doc = document(result.primary_bytes.unwrap().bytes);
    for kind in attributes(&doc, "kind") {
        assert!(
            allowed.split_whitespace().any(|v| v == kind),
            "kind {kind} is outside the registered vocabulary: {allowed}"
        );
    }
}
