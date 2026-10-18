use cem_ml::{
    engine::*,
    real::RealCemMlEngine,
    run_config::ScopeConfig,
    schema::registry::{
        CEM_QL_CONTENT_TYPE, CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI,
        CEM_QL_SCHEMA_URI, HTML_CONTENT_TYPE, HTML_SCHEMA_URI,
    },
};
use cem_ml_transform_cem_ql::engine_context_with_cem_ql_template_adapter;

const URI: &str = "studio://login/queries/expression.cemql";
fn identity() -> FormatIdentity {
    FormatIdentity {
        content_type: Some(CEM_QL_EXPRESSION_CONTENT_TYPE.into()),
        schema: Some(CEM_QL_EXPRESSION_SCHEMA_URI.into()),
        ..Default::default()
    }
}
fn request(source: &[u8]) -> ConvertRequest {
    ConvertRequest {
        input: EngineInput {
            uri: URI.into(),
            bytes: source.to_vec(),
            identity: Some(identity()),
            from_format: None,
            root_scope: Default::default(),
        },
        target: None,
        to_format: LayerFormat::Cem,
        preserve_source_offsets: true,
        target_scope: ScopeConfig {
            cemt_formatter_profile: Some("tabular".into()),
            output_color_type: Some("none".into()),
            ..Default::default()
        },
        context: engine_context_with_cem_ql_template_adapter(),
        scheduler_scope_id: 0,
    }
}
fn convert(request: ConvertRequest) -> ConvertResponse {
    RealCemMlEngine::new().convert(request).unwrap()
}

#[test]
fn standalone_format_preserves_identity_comments_unicode_and_is_idempotent() {
    let source = "// café\r\nlet label = \"🍒\";\r\nunknownFunction(label)\r\n";
    for identity_field in ["both", "schema", "content-type"] {
        let mut input = request(source.as_bytes());
        if identity_field == "schema" {
            input.input.identity.as_mut().unwrap().content_type = None;
        }
        if identity_field == "content-type" {
            input.input.identity.as_mut().unwrap().schema = None;
        }
        input
            .target_scope
            .cemt_formatter_options
            .insert("lineEnding".into(), "lf".into());
        let response = convert(input);
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let output = response.primary_bytes.unwrap();
        assert_eq!(output.content_type, CEM_QL_EXPRESSION_CONTENT_TYPE);
        assert_eq!(output.schema.as_deref(), Some(CEM_QL_EXPRESSION_SCHEMA_URI));
        assert_eq!(
            String::from_utf8_lossy(&output.bytes),
            source.replace("\r\n", "\n")
        );
        let writer = response
            .conversion
            .unwrap()
            .output_pipeline
            .unwrap()
            .stages
            .pop()
            .unwrap();
        assert_eq!(writer.schema.as_deref(), Some(CEM_QL_EXPRESSION_SCHEMA_URI));
        let again = convert(request(&output.bytes)).primary_bytes.unwrap();
        assert_eq!(again.bytes, output.bytes);
        assert_eq!(again.hash, output.hash);
    }
}

#[test]
fn standalone_format_rejects_invalid_source_without_shifting_diagnostics() {
    for source in ["\"🍒\" +", "module \"urn:wrong\" 1", "1 2", ""] {
        let response = convert(request(source.as_bytes()));
        assert!(response.primary_bytes.is_none(), "{source}");
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{source}"
        );
        assert!(!response
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.module_uri_missing"));
        assert!(response
            .diagnostics
            .iter()
            .all(|d| d.uri.as_deref() == Some(URI)));
        if source == "\"🍒\" +" {
            let error = response
                .diagnostics
                .iter()
                .find(|d| d.code == "cem.ql.parse_error")
                .unwrap();
            assert_eq!(
                (error.byte_offset, error.line, error.column),
                (Some(8), Some(1), Some(7))
            );
            assert!(error.source_map.is_some());
        }
    }
    assert!(convert(request(b"\xff"))
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.invalid_utf8"));
    assert!(convert(request("\"🍒\"".as_bytes())).diagnostics.is_empty());
}

#[test]
fn standalone_format_rejects_identity_changes_and_honors_cancellation() {
    let module = FormatIdentity {
        content_type: Some(CEM_QL_CONTENT_TYPE.into()),
        schema: Some(CEM_QL_SCHEMA_URI.into()),
        ..Default::default()
    };
    let mut invalid = request(b"1");
    invalid.input.identity.as_mut().unwrap().schema = module.schema.clone();
    let response = convert(invalid);
    assert!(response.primary_bytes.is_none());
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.query_identity_unsupported"));
    let mut changed = request(b"1");
    changed.target = Some(module.clone());
    let response = convert(changed);
    assert!(response.primary_bytes.is_none());
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.format_identity_unsupported"));
    let mut reverse = request(b"module \"urn:module\" 1");
    reverse.input.identity = Some(module);
    reverse.target = Some(identity());
    assert!(convert(reverse).primary_bytes.is_none());
    let mut mixed_html = request(b"1");
    mixed_html.target = Some(FormatIdentity {
        schema: Some(HTML_SCHEMA_URI.into()),
        ..identity()
    });
    let response = convert(mixed_html);
    assert!(response.primary_bytes.is_none());
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.format_identity_unsupported"));
    let aborted = request(b"1");
    aborted.context.abort_signal().abort();
    assert!(RealCemMlEngine::new().convert(aborted).is_err());
}

#[test]
fn standalone_format_keeps_registered_module_aliases_separate() {
    for media in ["text/cem-ql; charset=utf-8", "cem-ql/1", "cem-ql/module"] {
        let mut input = request(b"module \"urn:alias\" 1");
        input.input.identity = Some(FormatIdentity {
            content_type: Some(media.into()),
            schema: Some(CEM_QL_SCHEMA_URI.into()),
            ..Default::default()
        });
        let response = convert(input);
        assert!(
            response.diagnostics.is_empty(),
            "{media}: {:?}",
            response.diagnostics
        );
        let output = response.primary_bytes.unwrap();
        assert_eq!(output.content_type, media);
        assert_eq!(output.schema.as_deref(), Some(CEM_QL_SCHEMA_URI));
    }
    let mut unknown_html = request(b"1");
    unknown_html.target = Some(FormatIdentity {
        content_type: Some("application/x-mismatched".into()),
        schema: Some(HTML_SCHEMA_URI.into()),
        ..Default::default()
    });
    let response = convert(unknown_html);
    assert!(response.primary_bytes.is_none());
    assert!(response
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.format_identity_unsupported"));
}

#[test]
fn standalone_format_supports_explicit_identity_and_html_presentation() {
    let source = "\"<🍒>\"";
    let mut explicit = request(source.as_bytes());
    explicit.target = Some(identity());
    assert!(convert(explicit).diagnostics.is_empty());
    let mut html = request(source.as_bytes());
    html.target = Some(FormatIdentity {
        content_type: Some(HTML_CONTENT_TYPE.into()),
        schema: Some(HTML_SCHEMA_URI.into()),
        ..Default::default()
    });
    html.to_format = LayerFormat::Html;
    html.target_scope.output_color_type = Some("html-css-vars".into());
    let response = convert(html);
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    let output = response.primary_bytes.unwrap();
    assert_eq!(output.content_type, HTML_CONTENT_TYPE);
    assert_eq!(output.schema.as_deref(), Some(HTML_SCHEMA_URI));
    let text = String::from_utf8(output.bytes).unwrap();
    assert!(text.contains("&lt;🍒&gt;"), "{text}");
    assert!(!text.contains("module "));
}
