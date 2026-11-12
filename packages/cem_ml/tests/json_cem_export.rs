use cem_ml::{
    engine::{
        CemMlEngine, ConvertRequest, EngineContext, EngineInput, FormatIdentity, LayerFormat,
    },
    parser::CemAstNode,
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    schema::{
        registry::{CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI},
        vocab::CompiledSchema,
    },
    source_map::{FrameSpan, TransformKind},
};

const URI: &str = "memory:original/source.json";
const IDENTITIES: [(&str, &str); 3] = [
    ("application/json", "https://cem.dev/ns/data/json/1"),
    (
        "application/schema+json",
        "https://cem.dev/ns/data/json-schema/1",
    ),
    ("application/json", "https://cem.dev/ns/data/json-schema/1"),
];

fn convert(
    source: &[u8],
    identity: (&str, &str),
    explicit_target: bool,
) -> cem_ml::engine::ConvertResponse {
    RealCemMlEngine::new()
        .convert(ConvertRequest {
            input: EngineInput {
                uri: URI.into(),
                bytes: source.to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some(identity.0.into()),
                    schema: Some(identity.1.into()),
                    ..Default::default()
                }),
                root_scope: Default::default(),
            },
            to_format: if explicit_target {
                LayerFormat::DomJson
            } else {
                LayerFormat::Cem
            },
            target: explicit_target.then(|| FormatIdentity {
                content_type: Some(CEM_ML_CONTENT_TYPE.into()),
                schema: Some(CEM_ML_SCHEMA_URI.into()),
                ..Default::default()
            }),
            preserve_source_offsets: false,
            context: EngineContext::default(),
            target_scope: ScopeConfig {
                cemt_formatter_profile: Some("tabular".into()),
                output_color_type: Some("none".into()),
                ..Default::default()
            },
            scheduler_scope_id: 0,
        })
        .unwrap()
}

fn parse(bytes: &[u8]) -> cem_ml::import::ScopedCemImport {
    cem_ml::import::import_bytes_with_lexical_scopes(
        bytes,
        CEM_ML_CONTENT_TYPE,
        "memory:converted.cem",
        CompiledSchema::cem_core(),
    )
    .expect("valid native CEM document")
}

fn values(document: &cem_ml::import::ScopedCemImport) -> Vec<String> {
    document
        .tree
        .ast()
        .iter()
        .filter_map(|node| {
            let CemAstNode::Element { node_id, .. } = node else {
                return None;
            };
            let name = document
                .captured
                .expanded_name(document.tree.ast_owner(), *node_id)?;
            if name.namespace_uri != "cem:generic-data"
                || !matches!(name.local_name.as_str(), "string" | "number" | "boolean")
            {
                return None;
            }
            Some(
                document
                    .tree
                    .node(*node_id)
                    .unwrap()
                    .children
                    .iter()
                    .map(|id| document.tree.node(*id).unwrap().value.as_str())
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn json_cem_export_preserves_literal_properties_types_order_and_original_ranges() {
    let source = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","{$never()}":"Café 🧬 {#never} {$never()} <tag> & data","":"  spaces  ","number":1.2300e+04,"enabled":true,"none":null,"items":["line one\nline two","001",false]}"#;
    for identity in IDENTITIES {
        for explicit_target in [false, true] {
            let response = convert(source.as_bytes(), identity, explicit_target);
            assert!(
                response.diagnostics.is_empty(),
                "{:?}",
                response.diagnostics
            );
            let bytes = response.primary_bytes.expect("native CEM output");
            assert_eq!(bytes.content_type, CEM_ML_CONTENT_TYPE);
            assert_eq!(bytes.schema.as_deref(), Some(CEM_ML_SCHEMA_URI));
            assert!(bytes.provenance.as_ref().expect("native lineage").output_spans().iter().any(|span|
                span.origin.frames.iter().any(|frame|
                    matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == URI)
                    && matches!(&frame.span, FrameSpan::Single(range) if range.len > 0 && range.start + u64::from(range.len) <= source.len() as u64))));
            let document = parse(&bytes.bytes);
            assert_eq!(
                values(&document),
                [
                    "https://json-schema.org/draft/2020-12/schema",
                    "Café 🧬 {#never} {$never()} <tag> & data",
                    "  spaces  ",
                    "1.2300e+04",
                    "true",
                    "line one\nline two",
                    "001",
                    "false"
                ]
            );
            let names: Vec<_> = document
                .tree
                .ast()
                .iter()
                .filter_map(|node| match node {
                    CemAstNode::Attribute {
                        expanded_name,
                        value,
                        value_nodes,
                        ..
                    } if expanded_name.local_name == "name" => {
                        assert!(value_nodes.is_empty());
                        value.as_deref()
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                names,
                [
                    "$schema",
                    "{$never()}",
                    "",
                    "number",
                    "enabled",
                    "none",
                    "items"
                ]
            );
            assert!(document
                .tree
                .ast()
                .iter()
                .all(|node| !matches!(node, CemAstNode::Reference { .. })));
        }
    }
}

#[test]
fn json_cem_export_preserves_scalar_roots_and_json_schema_keywords_as_data() {
    for (source, kind, value) in [
        ("null", "null", None),
        ("false", "boolean", Some("false")),
        ("42", "number", Some("42")),
        (
            "\"  literal {$never()}  \"",
            "string",
            Some("  literal {$never()}  "),
        ),
        ("[]", "array", None),
        ("{}", "object", None),
    ] {
        let response = convert(source.as_bytes(), IDENTITIES[0], false);
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let document = parse(&response.primary_bytes.expect("CEM scalar root").bytes);
        let root = document.tree.ast().iter().find_map(|node| {
            let CemAstNode::Element { node_id, .. } = node else {
                return None;
            };
            let name = document
                .captured
                .expanded_name(document.tree.ast_owner(), *node_id)?;
            (name.namespace_uri == "cem:generic-data").then(|| name.local_name.clone())
        });
        assert_eq!(root.as_deref(), Some(kind));
        assert_eq!(
            values(&document),
            value.into_iter().map(str::to_owned).collect::<Vec<_>>()
        );
    }
    let source = r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"note":{"type":"string","const":"{$never()}"}},"$ref":"#/$defs/note"}"##;
    for identity in &IDENTITIES[1..] {
        let response = convert(source.as_bytes(), *identity, false);
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let document = parse(&response.primary_bytes.expect("CEM schema data").bytes);
        assert_eq!(
            values(&document),
            [
                "https://json-schema.org/draft/2020-12/schema",
                "string",
                "{$never()}",
                "#/$defs/note"
            ]
        );
    }
}

#[test]
fn json_cem_export_refuses_malformed_and_non_utf8_sources_without_output() {
    for identity in IDENTITIES {
        for source in [
            b"{\"items\":[1,}".as_slice(),
            b"{} {}",
            b"{\"x\":NaN}",
            b"{\"x\":\"\xff\"}",
        ] {
            let response = convert(source, identity, false);
            assert!(response
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()));
            assert!(response.primary_bytes.is_none());
            assert!(response.primary.is_null());
            assert!(
                !response
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "cem.lifecycle.internal_ast_target_unsupported"),
                "{:?}",
                response.diagnostics
            );
        }
    }
    for identity in &IDENTITIES[1..] {
        for source in [b"{}".as_slice(), br#"{"$schema":"urn:unsupported"}"#] {
            let response = convert(source, *identity, false);
            assert!(response
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.json_schema.unsupported_dialect"));
            assert!(response.primary_bytes.is_none());
            assert!(response.primary.is_null());
        }
    }
}
