use std::path::Path;

use cem_ql::embedded::{
    checked_in_cem_sources, extract_embedded_expressions_from_source,
    extract_repository_embedded_expressions, EmbeddedArtifactRole, EmbeddedHostKind,
};

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("packages/cem_ql has workspace root two levels up")
}

#[test]
fn checked_in_scan_finds_cem_and_cemt_sources() {
    let sources = checked_in_cem_sources(workspace_root()).expect("checked-in CEM sources");
    assert!(sources.iter().any(
        |path| path.ends_with("packages/cem_ml/schema-packages/csv/v1/formatters/pretty.cemt")
    ));
    assert!(sources.iter().any(|path| path.ends_with(
        "packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem"
    )));
    assert!(sources.iter().all(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "cem" | "cemt"))
    }));
}

#[test]
fn repository_extractor_covers_required_embedding_classes() {
    let expressions =
        extract_repository_embedded_expressions(workspace_root()).expect("repository expressions");
    assert!(
        expressions.len() > 100,
        "expected repository-wide embedded expression coverage, got {}",
        expressions.len()
    );
    assert!(expressions.iter().any(|expression| {
        expression.artifact_role == EmbeddedArtifactRole::Formatter
            && expression
                .source_path
                .ends_with("packages/cem_ml/schema-packages/csv/v1/formatters/pretty.cemt")
            && expression.host_kind == EmbeddedHostKind::ExpressionNode
    }));
    assert!(expressions.iter().any(|expression| {
        expression.artifact_role == EmbeddedArtifactRole::Colorizer
            && expression
                .source_path
                .ends_with("packages/cem_ml/schema-packages/csv/v1/colorizers/terminal.cemt")
            && expression.host_kind == EmbeddedHostKind::ExpressionNode
    }));
    assert!(expressions.iter().any(|expression| {
        expression.artifact_role == EmbeddedArtifactRole::Converter
            && expression.source_path.ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            )
            && matches!(
                expression.host_kind,
                EmbeddedHostKind::SelectAttribute | EmbeddedHostKind::TestAttribute
            )
    }));
    assert!(expressions.iter().any(|expression| {
        expression.artifact_role == EmbeddedArtifactRole::Validation
            && expression.source_path.ends_with(
                "packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem",
            )
            && matches!(
                expression.host_kind,
                EmbeddedHostKind::BehaviorSelectAttribute
                    | EmbeddedHostKind::BehaviorMatchAttribute
            )
    }));
    assert!(expressions.iter().any(|expression| {
        expression.host_kind == EmbeddedHostKind::AttributeValueTemplate
            && expression.source_path.ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            )
            && expression.normalized_source == "node.name"
    }));
}

#[test]
fn source_extractor_keeps_host_and_expression_ranges_distinct() {
    let source = r#"{item @title="prefix { $label } suffix" | {$ $body }}"#;
    let expressions = extract_embedded_expressions_from_source("examples/range.cem", source);
    assert_eq!(expressions.len(), 2);

    let avt = &expressions[0];
    assert_eq!(avt.source, "$label");
    assert_eq!(avt.normalized_source, "label");
    assert!(avt.host_range.start < avt.expression_range.start);
    assert!(avt.host_range.end() > avt.expression_range.end());

    let content = &expressions[1];
    assert_eq!(content.source, "$body");
    assert_eq!(content.normalized_source, "body");
    assert!(content.host_range.start < content.expression_range.start);
}
