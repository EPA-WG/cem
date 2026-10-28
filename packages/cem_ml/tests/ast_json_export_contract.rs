//! Exported semantic AST JSON is a source-validation boundary, not executable replay.
use cem_ml::{
    engine::*,
    real::RealCemMlEngine,
    schema::registry::{CEM_AST_JSON_PROJECTION_CONTENT_TYPE, CEM_AST_PROJECTION_SCHEMA_URI},
};
use serde_json::{json, Value};
fn input(bytes: &[u8]) -> EngineInput {
    EngineInput {
        uri: "studio://exports/original.cem".into(),
        bytes: bytes.into(),
        from_format: Some(InputFormat::Cem),
        identity: None,
        root_scope: Default::default(),
    }
}
fn export(bytes: &[u8]) -> Value {
    let response = RealCemMlEngine::new()
        .convert(ConvertRequest {
            input: input(bytes),
            to_format: LayerFormat::Ast,
            preserve_source_offsets: true,
            context: Default::default(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        })
        .unwrap();
    assert!(
        !response
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        response.diagnostics
    );
    response.primary
}
fn validate(value: &Value) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut source = input(&serde_json::to_vec(value).unwrap());
    source.uri = "studio://exports/output.ast.json".into();
    source.from_format = None;
    source.identity = Some(FormatIdentity {
        content_type: Some(CEM_AST_JSON_PROJECTION_CONTENT_TYPE.into()),
        schema: Some(CEM_AST_PROJECTION_SCHEMA_URI.into()),
        ..Default::default()
    });
    RealCemMlEngine::new()
        .validate(ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: Default::default(),
        })
        .unwrap()
        .report
        .diagnostics
}
#[test]
fn actual_ast_json_exports_validate_without_application_reshaping() {
    for source in [
        "{p | Café}",
        "{p @title=hello | text}",
        include_str!("../../../examples/projects/native-projections/data/dispatch.cem"),
    ] {
        let value = export(source.as_bytes());
        let diagnostics = validate(&value);
        assert!(diagnostics.is_empty(), "{value}: {diagnostics:?}");
        assert_eq!(value["kind"], "document");
        assert!(value["sourceMap"]["frames"].is_array());
        let element = value["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["kind"] == "element")
            .unwrap();
        assert!(element["namespace"].is_string());
        assert!(element["byteRange"]["start"].is_u64());
        assert!(element["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["kind"] == "text")
            .all(|n| n["data"].is_string() && n.get("value").is_none()));
    }
}
#[test]
fn exported_ast_rejects_old_array_and_malformed_document_envelopes() {
    let value = export(b"{p | Hi}");
    assert!(
        validate(&value).is_empty(),
        "actual export must first be valid"
    );
    for broken in [
        json!([value.clone()]),
        json!({"kind":"document","children":{}}),
        json!({"kind":"document","children":[{"kind":"text","value":"Hi"}]}),
        json!({"kind":"document","children":[{"kind":"reference","expression":"x","context":-1,"targets":[]}]}),
        json!({"kind":"document","children":[{"kind":"text","data":"Hi","byteRange":{"start":0,"len":-1}}]}),
    ] {
        assert!(
            validate(&broken)
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{broken}"
        );
    }
}

#[test]
fn semantic_ast_json_preserves_namespace_value_nodes_and_nonowning_graph_edges() {
    use cem_ml::{
        parser::{document::CemDocument, CemAstNode as Node, ExpandedName},
        source::{ByteRange, SourceId},
        source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
    };
    let source = SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(7),
            span: FrameSpan::Single(ByteRange::new(4, 9)),
            transform: TransformKind::SourceBoundary {
                uri: "studio://original/é.cem".into(),
            },
        }],
    };
    let name = |local: &str| ExpandedName {
        local_name: local.into(),
        namespace_uri: "urn:test".into(),
        schema_id: None,
    };
    let doc = CemDocument {
        nodes: vec![
            Node::Document {
                node_id: 0,
                root_children: vec![1],
                source: source.clone(),
            },
            Node::Element {
                node_id: 1,
                expanded_name: name("view"),
                attributes: vec![2],
                children: vec![3, 4],
                has_explicit_boundary: true,
                source: source.clone(),
            },
            Node::Attribute {
                node_id: 2,
                expanded_name: name("title"),
                value: None,
                value_nodes: vec![4, 3],
                source: source.clone(),
            },
            Node::Reference {
                node_id: 3,
                expression: "$original".into(),
                context: 1,
                targets: Some(vec![1, 1, 3]),
                source: source.clone(),
            },
            Node::Text {
                node_id: 4,
                data: "Café".into(),
                source: source.clone(),
            },
        ],
        ..Default::default()
    };
    let value = cem_ml::projection::ast_json(&doc);
    assert!(validate(&value).is_empty());
    let node = &value["children"][0];
    assert_eq!(node["namespace"], "urn:test");
    assert_eq!(node["attributes"][0]["namespace"], "urn:test");
    assert!(node["attributes"][0]["value"].is_null());
    assert_eq!(node["attributes"][0]["valueNodes"][0]["data"], "Café");
    let reference = &node["children"][0];
    assert_eq!(reference["kind"], "reference");
    assert_eq!(reference["context"], 1);
    assert_eq!(reference["targets"], json!([1, 1, 3]));
    assert_eq!(reference["expression"], "$original");
    assert_eq!(reference["byteRange"], json!({"start":4,"len":9}));
    assert_eq!(
        reference["sourceMap"],
        serde_json::to_value(&source).unwrap()
    );
    // Non-owning targets can repeat or cycle; exporting never follows them.
    assert_eq!(node["children"].as_array().unwrap().len(), 2);
    let subject = cem_ml::projection::ast_stream(&doc, None).into_cemt_subject();
    assert!(
        subject.is_array(),
        "CEMT presentation remains its own contract"
    );
}
