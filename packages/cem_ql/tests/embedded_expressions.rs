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
        expression.provenance.artifact_role == EmbeddedArtifactRole::Formatter
            && expression
                .provenance
                .source_path
                .ends_with("packages/cem_ml/schema-packages/csv/v1/formatters/pretty.cemt")
            && expression.provenance.host.kind == EmbeddedHostKind::ExpressionNode
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::Colorizer
            && expression
                .provenance
                .source_path
                .ends_with("packages/cem_ml/schema-packages/csv/v1/colorizers/terminal.cemt")
            && expression.provenance.host.kind == EmbeddedHostKind::ExpressionNode
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::Converter
            && expression.provenance.source_path.ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            )
            && matches!(
                expression.provenance.host.kind,
                EmbeddedHostKind::SelectAttribute | EmbeddedHostKind::TestAttribute
            )
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::Validator
            && expression.provenance.source_path.ends_with(
                "packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem",
            )
            && matches!(
                expression.provenance.host.kind,
                EmbeddedHostKind::BehaviorSelectAttribute
                    | EmbeddedHostKind::BehaviorMatchAttribute
            )
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.host.kind == EmbeddedHostKind::AttributeValueTemplate
            && expression.provenance.source_path.ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            )
            && expression.normalized_source == "node.name"
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::TransformConfig
            && expression
                .provenance
                .source_path
                .ends_with("packages/custom-element/material/importmap.transform.cem")
            && expression.provenance.host.kind == EmbeddedHostKind::AttributeValueTemplate
            && expression.normalized_source == "file"
    }));
}

#[test]
fn repository_extractor_preserves_complete_source_provenance() {
    let expressions =
        extract_repository_embedded_expressions(workspace_root()).expect("repository expressions");
    for expression in expressions {
        let provenance = &expression.provenance;
        assert!(
            !provenance.source_path.as_os_str().is_empty(),
            "missing source path for {expression:#?}"
        );
        assert!(
            provenance.host.range.len > 0,
            "missing host byte range for {expression:#?}"
        );
        assert!(
            provenance.cem_ql_range.len > 0,
            "missing CEM-QL sub-span for {expression:#?}"
        );
        assert!(
            provenance.host.range.start <= provenance.cem_ql_range.start,
            "host range must contain CEM-QL span for {expression:#?}"
        );
        assert!(
            provenance.host.range.end() >= provenance.cem_ql_range.end(),
            "host range must contain CEM-QL span for {expression:#?}"
        );
        assert_ne!(
            provenance.artifact_role,
            EmbeddedArtifactRole::Unknown,
            "repository expression must have an artifact role for {expression:#?}"
        );
        if provenance
            .source_path
            .starts_with("packages/cem_ml/schema-packages")
        {
            assert!(
                provenance.schema_package.is_some(),
                "schema-package expression must identify owning package for {expression:#?}"
            );
        }
        match provenance.host.kind {
            EmbeddedHostKind::ExpressionNode => {
                assert!(
                    provenance.host.attribute_name.is_none(),
                    "expression node must not report an attribute name for {expression:#?}"
                );
            }
            _ => {
                assert!(
                    provenance.host.attribute_name.is_some(),
                    "attribute-hosted expression must report the attribute name for {expression:#?}"
                );
            }
        }
    }
}

#[test]
fn source_extractor_keeps_host_and_expression_ranges_distinct() {
    let source = r#"{item @title="prefix { $label } suffix" | {$ $body }}"#;
    let expressions = extract_embedded_expressions_from_source("examples/range.cem", source);
    assert_eq!(expressions.len(), 2);

    let avt = &expressions[0];
    assert_eq!(avt.source, "$label");
    assert_eq!(avt.normalized_source, "label");
    assert!(avt.provenance.host.range.start < avt.provenance.cem_ql_range.start);
    assert!(avt.provenance.host.range.end() > avt.provenance.cem_ql_range.end());

    let content = &expressions[1];
    assert_eq!(content.source, "$body");
    assert_eq!(content.normalized_source, "body");
    assert!(content.provenance.host.range.start < content.provenance.cem_ql_range.start);
}
