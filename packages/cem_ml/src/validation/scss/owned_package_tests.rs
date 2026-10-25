use super::*;
use crate::{
    engine::{
        CemMlEngine, ConvertRequest, EngineContext, EngineInput, FailLevel, FormatIdentity,
        LayerFormat, ParseProjection, ParseRequest, SchemaPackageReplacementGrant,
    },
    lifecycle::{LifecycleRegistry, LoadedInputAstStream},
    schema::{
        asset_pack::{
            schema_asset_sha256, SchemaAssetPack, SchemaAssetPackIndex, SchemaAssetPackLimits,
            SchemaAssetSourceDescriptor, SchemaAssetSourceKind, SCHEMA_ASSET_PACK_FORMAT,
        },
        asset_pack_session::SchemaAssetSession,
        document_model::{
            compile_schema_document_model, compile_schema_document_model_from_retained,
            INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE,
        },
        package_compilation::SchemaPackageSourceRegistry,
        registry::{SchemaPackageOrigin, SchemaRegistry},
    },
};

fn session() -> SchemaAssetSession {
    let originals = [
        (
            "package.cem",
            SchemaAssetSourceKind::Manifest,
            include_str!("../../../schema-packages/scss/v1/package.cem"),
        ),
        (
            "schema/scss.cem",
            SchemaAssetSourceKind::Schema,
            include_str!("../../../schema-packages/scss/v1/schema/scss.cem"),
        ),
        (
            "formatters/compact.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../schema-packages/scss/v1/formatters/compact.cemt"),
        ),
        (
            "formatters/pretty.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../schema-packages/scss/v1/formatters/pretty.cemt"),
        ),
        (
            "formatters/tabular.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../schema-packages/scss/v1/formatters/tabular.cemt"),
        ),
        (
            "formatters/scss-format-source.cemt",
            SchemaAssetSourceKind::Artifact,
            include_str!("../../../schema-packages/scss/v1/formatters/scss-format-source.cemt"),
        ),
        (
            "colorizers/terminal.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../schema-packages/scss/v1/colorizers/terminal.cemt"),
        ),
        (
            "colorizers/html.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../schema-packages/scss/v1/colorizers/html.cemt"),
        ),
        (
            "colorizers/md.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../schema-packages/scss/v1/colorizers/md.cemt"),
        ),
        (
            "colorizers/scss-color-source.cemt",
            SchemaAssetSourceKind::Artifact,
            include_str!("../../../schema-packages/scss/v1/colorizers/scss-color-source.cemt"),
        ),
    ];
    let index = SchemaAssetPackIndex {
        format_version: SCHEMA_ASSET_PACK_FORMAT.into(),
        packages: vec![SchemaRegistry::with_builtin_schemas()
            .schema(SCSS_SCHEMA_URI)
            .unwrap()
            .clone()
            .into()],
        sources: originals
            .iter()
            .map(|(path, kind, text)| SchemaAssetSourceDescriptor {
                package_id: "scss".into(),
                version: "1.0.0".into(),
                path: format!("schema-packages/scss/v1/{path}"),
                kind: *kind,
                content_type: match kind {
                    SchemaAssetSourceKind::Manifest => "application/vnd.cem.schema-package+cem",
                    SchemaAssetSourceKind::Schema => "application/vnd.cem.schema+cem",
                    _ => "application/vnd.cem.transform+cem",
                }
                .into(),
                byte_length: text.len(),
                digest: schema_asset_sha256(text.as_bytes()),
            })
            .collect(),
    };
    let bytes = index.to_bytes().unwrap();
    let pack = SchemaAssetPack::from_index_bytes(
        &bytes,
        &schema_asset_sha256(&bytes),
        SchemaAssetPackLimits::default(),
    )
    .unwrap();
    let mut context = EngineContext::default();
    context
        .schema_package_replacement_grants
        .push(SchemaPackageReplacementGrant {
            package_id: "scss".into(),
            expected_origin: SchemaPackageOrigin::Builtin,
            replacement_manifest_uri: "schema-packages/scss/v1/package.cem".into(),
        });
    let mut session = SchemaAssetSession::new(pack, context);
    assert!(session.activate(&[("scss", "1.0.0")]).is_err());
    for (path, _, text) in originals {
        session
            .admit(
                "scss",
                "1.0.0",
                &format!("schema-packages/scss/v1/{path}"),
                text.as_bytes().to_vec(),
            )
            .unwrap();
    }
    session
}

#[test]
fn admitted_scss_originals_preserve_passive_validation_and_typed_css_output() {
    let mut session = session();
    let diagnostics = session.activate(&[("scss", "1.0.0")]).unwrap();
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:#?}"
    );
    let input = EngineInput {
        uri: "memory://owned/styles/main.scss".into(),
        bytes: b"$space: 2px; .component { padding: $space; }".to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(SCSS_CONTENT_TYPE.into()),
            schema: Some(SCSS_SCHEMA_URI.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    };
    let validated = LifecycleRegistry::with_builtin_adapters()
        .load_for_source_validation(&input, session.context());
    assert!(
        validated.ast_stream.is_none(),
        "passive source validation must not expand CSS"
    );
    assert!(
        validated.diagnostics.is_empty(),
        "{:?}",
        validated.diagnostics
    );
    let loaded = LifecycleRegistry::with_builtin_adapters().load(&input, session.context());
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let Some(LoadedInputAstStream::CssDocument(document)) = loaded.ast_stream else {
        panic!("native typed CSS handoff")
    };
    assert_eq!(document.source.uri, input.uri);

    let colored = crate::conversion::execute_css_document_output_pipeline_with_environment(
        &crate::conversion::ConversionOutputPipelineEnvironment {
            schema_registry: &session.context().schema_registry,
            conversion_registry: &session.context().converter_registry,
            package_artifact_reader: Some(session.context()),
            artifact_cache: None,
        },
        document,
        &crate::run_config::ScopeConfig {
            cemt_formatter_profile: Some("tabular".into()),
            cemt_color_profile: Some("html".into()),
            ..Default::default()
        },
        Some(&input.uri),
    );
    assert!(colored.diagnostics.is_empty(), "{:?}", colored.diagnostics);
    let presentation = colored
        .output
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .unwrap();
    assert!(
        presentation.contains("<span")
            && presentation.contains("component")
            && presentation.contains("padding"),
        "{presentation}"
    );

    let mut module = input.clone();
    module.bytes = b"@use \"unavailable\"; .component { color: red; }".to_vec();
    let passive = LifecycleRegistry::with_builtin_adapters()
        .load_for_source_validation(&module, session.context());
    assert!(
        passive.diagnostics.is_empty(),
        "passive validation performed module resolution: {:?}",
        passive.diagnostics
    );
    let denied = LifecycleRegistry::with_builtin_adapters().load(&module, session.context());
    assert!(denied.ast_stream.is_none());
    assert!(
        denied.diagnostics.iter().any(|d| matches!(
            d.code.as_str(),
            "cem.scss.resolver_denied" | "cem.scss.module_error"
        )),
        "{:?}",
        denied.diagnostics
    );

    let response = crate::real::RealCemMlEngine
        .convert(ConvertRequest {
            input,
            to_format: LayerFormat::Css,
            preserve_source_offsets: true,
            context: session.context().clone(),
            target: Some(FormatIdentity {
                content_type: Some(CSS_CONTENT_TYPE.into()),
                schema: Some(CSS_SCHEMA_URI.into()),
                ..Default::default()
            }),
            target_scope: crate::run_config::ScopeConfig {
                cemt_formatter_profile: Some("tabular".into()),
                output_color_type: Some("none".into()),
                ..Default::default()
            },
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
    let text = response
        .primary_bytes
        .as_ref()
        .map(|bytes| std::str::from_utf8(&bytes.bytes).unwrap())
        .or_else(|| {
            response
                .primary
                .get("content")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap();
    assert!(
        text.contains(".component") && text.contains("padding: 2px") && !text.contains("<span"),
        "{text}"
    );
}

#[test]
fn scss_owned_fact_contract_keeps_original_owner_and_refuses_unsafe_or_pending_models() {
    use std::sync::Arc;
    const SCHEMA: &str = include_str!("../../../schema-packages/scss/v1/schema/scss.cem");
    let retain = |text: &str| {
        SchemaPackageSourceRegistry::default()
            .retain("memory://owned/scss-schema.cem", text)
            .unwrap()
    };
    let customized = SCHEMA
        .replace("cem.scss.import_deprecated", "example.scss.owned_import")
        .replace(
            "@code=\"example.scss.owned_import\" @severity=\"warning\"",
            "@code=\"example.scss.owned_import\" @severity=\"error\"",
        );
    let owner = retain(&customized);
    let model = compile_schema_document_model_from_retained(SCSS_SCHEMA_URI, &owner);
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(Arc::ptr_eq(
        model
            .parser_contract
            .as_ref()
            .unwrap()
            .source_owner()
            .unwrap(),
        &owner
    ));
    let mut context = EngineContext::default();
    context.schema_registry.set_package_origin(
        "scss",
        SchemaPackageOrigin::Manifest("memory://owned/scss-schema.cem".into()),
    );
    context.schema_document_models.register(model);
    let input = EngineInput {
        uri: "memory://owned/styles/source.scss".into(),
        bytes: b"@import \"unavailable\"; .component { color: red; }".to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(SCSS_CONTENT_TYPE.into()),
            schema: Some(SCSS_SCHEMA_URI.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    };
    let loaded = LifecycleRegistry::with_builtin_adapters().load(&input, &context);
    assert!(
        loaded.ast_stream.is_none(),
        "schema-owned hard source facts must block expansion"
    );
    assert!(loaded
        .diagnostics
        .iter()
        .any(|d| d.code == "example.scss.owned_import" && d.severity.is_hard_violation()));
    assert!(
        !loaded.diagnostics.iter().any(|d| matches!(
            d.code.as_str(),
            "cem.scss.module_error" | "cem.scss.resolver_denied"
        )),
        "schema-owned source refusal must happen before module loading"
    );

    let inspection = compile_schema_document_model(SCSS_SCHEMA_URI, SCHEMA);
    assert!(!inspection.is_ready_for_validation());
    let mut pending = EngineContext::default();
    pending.schema_document_models.inspect_candidate(inspection);
    assert!(LifecycleRegistry::with_builtin_adapters()
        .load(&input, &pending)
        .ast_stream
        .is_none());
    let original = SCHEMA
        .lines()
        .find(|line| line.contains("@kind=\"scss-parser-validity\""))
        .unwrap();
    for invalid in [
        SCHEMA.replace(
            "@fact-kind=\"parse-error\"",
            "@fact-kind=\"unsupported-native-fact\"",
        ),
        SCHEMA.replace(original, ""),
        SCHEMA.replace(
            original,
            &format!(
                "{original}\n{}",
                original.replace("scss-parser-validity", "duplicate-native-fact")
            ),
        ),
        SCHEMA.replace(
            "@code=\"cem.scss.resolver_denied\" @severity=\"error\"",
            "@code=\"cem.scss.resolver_denied\" @severity=\"warning\"",
        ),
    ] {
        let model = compile_schema_document_model_from_retained(SCSS_SCHEMA_URI, &retain(&invalid));
        assert!(!model.is_ready_for_validation());
        assert!(
            model
                .compile_diagnostics
                .iter()
                .any(|d| d.code == INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE),
            "{:?}",
            model.compile_diagnostics
        );
    }
}

#[test]
fn scss_owned_fact_contract_does_not_rebind_public_diagnostic_codes() {
    let source = include_str!("../../../schema-packages/scss/v1/schema/scss.cem")
        .replace("cem.scss.module_error", "example.scss.module_failure")
        .replace("cem.scss.import_deprecated", "cem.scss.module_error");
    let owner = SchemaPackageSourceRegistry::default()
        .retain("memory://owned/collision-schema.cem", &source)
        .unwrap();
    let model = compile_schema_document_model_from_retained(SCSS_SCHEMA_URI, &owner);
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    let mut context = EngineContext::default();
    context.schema_registry.set_package_origin(
        "scss",
        SchemaPackageOrigin::Manifest("memory://owned/collision-schema.cem".into()),
    );
    context.schema_document_models.register(model);
    let input = EngineInput {
        uri: "memory://owned/collision.scss".into(),
        bytes: b"@import \"reset.css\"; .component { color: red; }".to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(SCSS_CONTENT_TYPE.into()),
            schema: Some(SCSS_SCHEMA_URI.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    };
    let loaded = LifecycleRegistry::with_builtin_adapters().load(&input, &context);
    assert!(matches!(
        loaded.ast_stream,
        Some(LoadedInputAstStream::CssDocument(_))
    ));
    assert_eq!(loaded.diagnostics.len(), 1);
    let diagnostic = &loaded.diagnostics[0];
    assert_eq!(diagnostic.code, "cem.scss.module_error");
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert_eq!(
        diagnostic.details.as_ref().unwrap()["nativeFact"]["factKind"],
        "import-deprecated"
    );
}

#[test]
fn admitted_scss_engine_parse_uses_typed_css_without_reparsing_source_as_cem() {
    let mut session = session();
    let diagnostics = session.activate(&[("scss", "1.0.0")]).unwrap();
    assert!(
        !diagnostics.iter().any(|d| d.severity.is_hard_violation()),
        "{diagnostics:?}"
    );
    let source = EngineInput {
        uri: "memory://owned/parse/basic.scss".into(),
        bytes: include_bytes!("../../../schema-packages/scss/v1/examples/basic.scss").to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(SCSS_ALIAS_CONTENT_TYPE.into()),
            schema: Some(SCSS_SCHEMA_URI.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    };
    let parse = |input, projection| {
        crate::real::RealCemMlEngine
            .parse(ParseRequest {
                input,
                projection,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: true,
                presentation_scope: Some(crate::run_config::ScopeConfig {
                    cemt_formatter_profile: Some("tabular".into()),
                    output_color_type: Some("none".into()),
                    ..Default::default()
                }),
                context: session.context().clone(),
            })
            .unwrap()
    };
    for projection in [ParseProjection::Ast, ParseProjection::Events] {
        let response = parse(source.clone(), projection);
        assert!(
            !response
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{projection:?}: {:?}",
            response.diagnostics
        );
        let bytes = response
            .primary_bytes
            .as_ref()
            .expect("named native source presentation");
        let text = std::str::from_utf8(&bytes.bytes).unwrap();
        assert!(
            text.contains("card__title") && text.contains(&source.uri),
            "{text}"
        );
        let projected = serde_json::to_string(&response.primary).unwrap();
        assert!(
            projected.contains("ScssOrigin") && projected.contains(&source.uri),
            "retained SCSS source frames must reach the explicit AST/events response"
        );

        let malformed = EngineInput {
            bytes: b".card { color: ".to_vec(),
            ..source.clone()
        };
        let invalid = parse(malformed, projection);
        assert!(
            invalid
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.scss.parse_error" && d.severity.is_hard_violation()),
            "{:?}",
            invalid.diagnostics
        );
        assert!(
            !invalid
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("cem.tokenizer.")),
            "malformed SCSS must not acquire unrelated CEM diagnostics"
        );
    }
    for projection in [ParseProjection::DomJson, ParseProjection::Json] {
        let response = parse(source.clone(), projection);
        assert!(response.primary.is_null());
        assert!(response
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.source.dom_projection_unsupported"
                && d.severity.is_hard_violation()));
    }
}
