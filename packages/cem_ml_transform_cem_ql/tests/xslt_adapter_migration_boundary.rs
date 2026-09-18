//! XSLT-ADAPTER-MIGRATION-GATE: characterize public contracts before replacing
//! the legacy adapter. These are migration probes, not desired typed semantics.
use cem_ml::{
    engine::{
        FormatIdentity, TemplateInput, TransformExecutionPolicy, TransformRuntimePhase,
        TransformTemplateEntrypoint,
    },
    run_config::ScopeConfig,
    transform_template::{
        TransformTemplateAdapter, TransformTemplateCompileRequest, TransformTemplateParameterArena,
    },
};
use cem_ml_transform_cem_ql::XsltParityTransformTemplateAdapter;
use cem_ql::xslt::compiler::compile_xslt_bundle;

fn legacy_accepts(source: &str) {
    let template = TemplateInput {
        uri: "memory:adapter-migration.xslt".into(),
        bytes: source.as_bytes().to_vec(),
        identity: Some(FormatIdentity {
            content_type: Some("application/xslt+xml".into()),
            ..Default::default()
        }),
        root_scope: ScopeConfig::default(),
    };
    let response = XsltParityTransformTemplateAdapter
        .compile(TransformTemplateCompileRequest {
            template: &template,
            entrypoint: &TransformTemplateEntrypoint::implicit(),
            params: &TransformTemplateParameterArena::default(),
            data_bindings: &[],
            module_options: Default::default(),
            module_preflight: Default::default(),
            execution_policy: TransformExecutionPolicy {
                runtime_phase: TransformRuntimePhase::XsltParity,
                ..Default::default()
            },
        })
        .expect("legacy adapter accepts its existing authoring contract");
    assert!(
        response
            .diagnostics
            .iter()
            .all(|d| !d.severity.is_hard_violation()),
        "{:?}",
        response.diagnostics
    );
}

#[test]
fn version_one_is_accepted_by_legacy_but_rejected_by_the_typed_profile() {
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0"><xsl:template match="/"><p>legacy</p></xsl:template></xsl:stylesheet>"#;
    legacy_accepts(source);
    let diagnostics = compile_xslt_bundle(source, "memory:adapter-migration.xslt").unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "cem.xslt.compile_unsupported"
            && d.message.contains("version 3.0")
            && d.source_map.is_some()));
}

#[test]
fn missing_namespace_is_accepted_by_legacy_but_rejected_by_the_typed_profile() {
    let source = r#"<xsl:stylesheet version="3.0"><xsl:template match="/"><p>legacy namespace shortcut</p></xsl:template></xsl:stylesheet>"#;
    legacy_accepts(source);
    let diagnostics = compile_xslt_bundle(source, "memory:adapter-migration.xslt").unwrap_err();
    assert!(diagnostics.iter().any(|d| d.severity.is_hard_violation()
        && d.uri.as_deref() == Some("memory:adapter-migration.xslt")
        && d.source_map.is_some()));
}

#[test]
fn namespace_correct_version_three_remains_the_shared_positive_case() {
    let source = r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><p>typed</p></xsl:template></xsl:stylesheet>"#;
    legacy_accepts(source);
    compile_xslt_bundle(source, "memory:adapter-migration.xslt").unwrap();
}
