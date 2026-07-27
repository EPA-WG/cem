//! AC-T-7 - CLI routes CEM-QL template embedding validation through the
//! `cem_ml_transform_cem_ql` bridge crate.

use cem_ml::engine::InputFormat;
use cem_ml_transform_cem_ql::{
    CemQlTemplateEmbeddingIdentity, CemQlTemplateEmbeddingValidationRequest,
};

#[test]
fn bridge_embedding_validator_reports_cem_ql_diagnostic_with_host_source_map() {
    let diagnostics = cem_ml_transform_cem_ql::validate_cem_ql_template_embedding_source_bytes(
        CemQlTemplateEmbeddingValidationRequest {
            bytes: b"{p | {$ 1 + }}",
            from_format: InputFormat::Cem,
            source_uri: Some("broken.cem"),
            identity: CemQlTemplateEmbeddingIdentity::default(),
        },
    );

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.starts_with("cem.ql."))
        .unwrap_or_else(|| panic!("expected CEM-QL diagnostic: {diagnostics:?}"));
    assert_eq!(diagnostic.uri.as_deref(), Some("broken.cem"));
    let stack = diagnostic
        .source_map
        .as_ref()
        .expect("embedding diagnostic carries source-map stack");
    assert!(
        stack.frames.iter().any(|frame| {
            matches!(
                frame.transform,
                cem_ml::source_map::TransformKind::CemTokenizer
            )
        }),
        "host CemTokenizer frame preserved"
    );
    assert!(
        stack.frames.iter().any(|frame| {
            matches!(
                frame.transform,
                cem_ml::source_map::TransformKind::TemplateEmbedding { .. }
            )
        }),
        "TemplateEmbedding frame pushed for the host to CEM-QL boundary"
    );
}

#[test]
fn bridge_embedding_validator_accepts_valid_host_surfaces() {
    for source in [
        "{p | {$ 1 + 2 * 3 }}",
        r#"{for-each @select="(1, 2, 3) | (3, 4)" | item}"#,
        r#"{button @label="hello {1 + 1}" | Save}"#,
    ] {
        let diagnostics = cem_ml_transform_cem_ql::validate_cem_ql_template_embedding_source_bytes(
            CemQlTemplateEmbeddingValidationRequest {
                bytes: source.as_bytes(),
                from_format: InputFormat::Cem,
                source_uri: Some("valid.cem"),
                identity: CemQlTemplateEmbeddingIdentity::default(),
            },
        );
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
    }
}

#[test]
fn html_input_does_not_invoke_cem_ql_template_pass() {
    let diagnostics = cem_ml_transform_cem_ql::validate_cem_ql_template_embedding_source_bytes(
        CemQlTemplateEmbeddingValidationRequest {
            bytes: b"<p>Hello {{ not-cem-ql }}</p>",
            from_format: InputFormat::Html,
            source_uri: Some("page.html"),
            identity: CemQlTemplateEmbeddingIdentity::default(),
        },
    );

    assert!(
        diagnostics.is_empty(),
        "HTML inputs produce no cem-ql template diagnostics; got {diagnostics:?}"
    );
}

#[test]
fn template_embedding_target_is_registered() {
    let project = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("project.json");
    let text = std::fs::read_to_string(&project)
        .unwrap_or_else(|err| panic!("read {}: {err}", project.display()));
    assert!(
        text.contains("\"test:template-embedding\""),
        "project.json must expose the AC-T-7 verification target"
    );
}
