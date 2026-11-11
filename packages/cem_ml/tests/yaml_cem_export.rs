use cem_ml::{
    engine::{
        CemMlEngine, ConvertRequest, EngineContext, EngineInput, FormatIdentity, LayerFormat,
    },
    parser::CemAstNode,
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    schema::{
        registry::{CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, YAML_CONTENT_TYPE},
        vocab::CompiledSchema,
    },
    source_map::{FrameSpan, TransformKind},
};

const URI: &str = "memory:original/settings.yml";
fn convert(source: &[u8], explicit_target: bool) -> cem_ml::engine::ConvertResponse {
    RealCemMlEngine::new()
        .convert(ConvertRequest {
            input: EngineInput {
                uri: URI.into(),
                bytes: source.to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some(YAML_CONTENT_TYPE.into()),
                    ..Default::default()
                }),
                root_scope: Default::default(),
            },
            to_format: if explicit_target {
                LayerFormat::DomJson
            } else {
                LayerFormat::Cem
            },
            preserve_source_offsets: false,
            context: EngineContext::default(),
            target: explicit_target.then(|| FormatIdentity {
                content_type: Some(CEM_ML_CONTENT_TYPE.into()),
                schema: Some(CEM_ML_SCHEMA_URI.into()),
                ..Default::default()
            }),
            target_scope: ScopeConfig {
                cemt_formatter_profile: Some("tabular".into()),
                output_color_type: Some("none".into()),
                ..Default::default()
            },
            scheduler_scope_id: 0,
        })
        .unwrap()
}
fn parse(bytes: Vec<u8>) -> cem_ml::import::ScopedCemImport {
    cem_ml::import::import_bytes_with_lexical_scopes(
        &bytes,
        CEM_ML_CONTENT_TYPE,
        "memory:converted.cem",
        CompiledSchema::cem_core(),
    )
    .expect("valid native CEM document")
}
fn texts(imported: &cem_ml::import::ScopedCemImport) -> Vec<String> {
    imported
        .tree
        .ast()
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element { node_id, .. } => {
                let name = imported
                    .captured
                    .expanded_name(imported.tree.ast_owner(), *node_id)?;
                if name.namespace_uri != "cem:generic-data"
                    || !matches!(name.local_name.as_str(), "string" | "boolean" | "number")
                {
                    return None;
                }
                let node = imported.tree.node(*node_id).unwrap();
                Some(
                    node.children
                        .iter()
                        .map(|id| imported.tree.node(*id).unwrap().value.as_str())
                        .collect(),
                )
            }
            _ => None,
        })
        .collect()
}
#[test]
fn yaml_cem_export_preserves_types_order_literals_and_native_provenance() {
    let source = "service:\n  name: 'Café 🧬 {#never} {$never()} <tag> & data'\n  enabled: true\n  retries: 3\n  empty: ''\n  none: null\n  tags: [api, public]\n";
    for explicit_target in [false, true] {
        let response = convert(source.as_bytes(), explicit_target);
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let bytes = response.primary_bytes.expect("native CEM output");
        assert_eq!(bytes.content_type, CEM_ML_CONTENT_TYPE);
        assert_eq!(bytes.schema.as_deref(), Some(CEM_ML_SCHEMA_URI));
        let spans = bytes
            .provenance
            .as_ref()
            .expect("native writer lineage")
            .output_spans();
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|span| span.origin.frames.iter().any(|frame| {
            matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == URI)
                && matches!(&frame.span, FrameSpan::Single(range) if range.len > 0 && range.start + u64::from(range.len) <= source.len() as u64)
        })));
        let document = parse(bytes.bytes);
        let elements: Vec<_> = document
            .tree
            .ast()
            .iter()
            .filter_map(|node| match node {
                CemAstNode::Element { node_id, .. } => {
                    let name = document
                        .captured
                        .expanded_name(document.tree.ast_owner(), *node_id)?;
                    (name.namespace_uri == "cem:generic-data").then_some(name.local_name.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            elements,
            [
                "object", "property", "object", "property", "string", "property", "boolean",
                "property", "number", "property", "string", "property", "null", "property",
                "array", "string", "string"
            ]
        );
        assert_eq!(
            texts(&document),
            [
                "Café 🧬 {#never} {$never()} <tag> & data",
                "true",
                "3",
                "",
                "api",
                "public"
            ]
        );
        assert!(document
            .tree
            .ast()
            .iter()
            .all(|node| !matches!(node, CemAstNode::Reference { .. })));
        assert!(document.tree.ast().iter().all(|node| match node {
            CemAstNode::Attribute { value_nodes, .. } => value_nodes.is_empty(),
            _ => true,
        }));
    }
}
#[test]
fn yaml_cem_export_preserves_multiple_documents_and_scalar_roots() {
    let response = convert(b"---\n- one\n- false\n---\n42\n", false);
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    let document = parse(response.primary_bytes.expect("CEM stream").bytes);
    assert_eq!(texts(&document), ["one", "false", "42"]);
}
#[test]
fn yaml_cem_export_refuses_invalid_unsafe_and_non_utf8_sources_atomically() {
    for source in [
        b"items: [\n".as_slice(),
        b"value: !unsafe literal\n",
        b"original: &anchor value\ncopy: *anchor\n",
        b"name: \xff\n",
    ] {
        let response = convert(source, false);
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{source:?}"
        );
        assert!(response.primary_bytes.is_none(), "{source:?}");
        assert!(response.primary.is_null(), "{source:?}");
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

#[test]
fn yaml_cem_export_preserves_string_whitespace_and_multiline_values() {
    let response = convert("\u{feff}padding: '  spaces  '\nblock: |-\n  line one\n  line two\nnumberText: '001'\nquoted: '\"quoted\"'\n".as_bytes(), false);
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    let imported = parse(response.primary_bytes.expect("CEM literal values").bytes);
    assert_eq!(
        texts(&imported),
        ["  spaces  ", "line one\nline two", "001", "\"quoted\""]
    );
}
