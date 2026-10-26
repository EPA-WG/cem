use super::*;
use crate::{
    conversion::{
        execute_markdown_document_output_pipeline_with_environment,
        ConversionOutputPipelineEnvironment,
    },
    engine::{
        CemMlEngine, EngineContext, FailLevel, ParseProjection, ParseRequest,
        SchemaPackageReplacementGrant,
    },
    lifecycle::LifecycleRegistry,
    schema::{
        asset_pack::{
            schema_asset_sha256, SchemaAssetPack, SchemaAssetPackIndex, SchemaAssetPackLimits,
            SchemaAssetSourceDescriptor, SchemaAssetSourceKind, SCHEMA_ASSET_PACK_FORMAT,
        },
        asset_pack_session::SchemaAssetSession,
        document_model::{
            compile_schema_document_model, compile_schema_document_model_from_retained,
        },
        package_compilation::SchemaPackageSourceRegistry,
        registry::{SchemaPackageOrigin, SchemaRegistry},
    },
};

const SCHEMA: &str = include_str!("../../../../schema-packages/markdown/v1/schema/markdown.cem");

fn input(bytes: &[u8], content_type: &str) -> EngineInput {
    EngineInput {
        uri: "memory://owned/article.md".into(),
        bytes: bytes.to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some(content_type.into()),
            schema: Some(MARKDOWN_SCHEMA_URI.into()),
            ..Default::default()
        }),
        root_scope: Default::default(),
    }
}

#[test]
fn owned_markdown_contract_defaults_to_gfm_without_rewriting_declared_identity() {
    let owner = SchemaPackageSourceRegistry::default()
        .retain("memory://owned/markdown-schema.cem", SCHEMA)
        .unwrap();
    let model = compile_schema_document_model_from_retained(MARKDOWN_SCHEMA_URI, &owner);
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    let mut context = EngineContext::default();
    context.schema_document_models.register(model);
    context.schema_registry.set_package_origin(
        "markdown",
        SchemaPackageOrigin::Manifest("memory://owned/markdown-schema.cem".into()),
    );
    for (content_type, expected_variant, tables) in [
        ("text/markdown; charset=utf-8", "GFM", true),
        (
            "text/markdown; charset=utf-8; variant=CommonMark",
            "CommonMark",
            false,
        ),
    ] {
        let source = input(
            b"| Name | Status |\n| --- | --- |\n| Owned | Ready |\n",
            content_type,
        );
        let loaded = LifecycleRegistry::with_builtin_adapters().load(&source, &context);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        let Some(LoadedInputAstStream::MarkdownDocument(document)) = loaded.ast_stream else {
            panic!("the admitted Markdown contract must retain the native document");
        };
        assert_eq!(document.source.uri, source.uri);
        assert_eq!(document.source.content_type, content_type);
        assert_eq!(document.variant, expected_variant);
        assert_eq!(
            document
                .events
                .iter()
                .any(|event| event.tag.as_deref() == Some("table")),
            tables
        );
    }
}

#[test]
fn owned_markdown_contract_retains_source_owner_and_refuses_unbound_or_unsafe_models() {
    let retain = |source: &str| {
        SchemaPackageSourceRegistry::default()
            .retain("memory://owned/markdown-schema.cem", source)
            .unwrap()
    };
    let customized = SCHEMA.replace(
        "@code=\"cem.markdown.unknown_variant\" @severity=\"warning\"",
        "@code=\"cem.markdown.unknown_variant\" @severity=\"error\"",
    );
    let owner = retain(&customized);
    let model = compile_schema_document_model_from_retained(MARKDOWN_SCHEMA_URI, &owner);
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(model
        .parser_contract
        .as_ref()
        .unwrap()
        .matches_owner(owner.ast_owner()));
    assert!(!model.is_ready_for_structural_validation());
    let mut context = EngineContext::default();
    context.schema_document_models.register(model);
    context.schema_registry.set_package_origin(
        "markdown",
        SchemaPackageOrigin::Manifest("memory://owned/markdown-schema.cem".into()),
    );
    let source = input(
        b"# Native article\n",
        "text/markdown; charset=utf-8; variant=Private",
    );
    let loaded = LifecycleRegistry::with_builtin_adapters().load(&source, &context);
    assert!(matches!(
        loaded.ast_stream,
        Some(LoadedInputAstStream::MarkdownDocument(_))
    ));
    assert!(loaded
        .diagnostics
        .iter()
        .any(|d| d.code == MARKDOWN_UNKNOWN_VARIANT_CODE && d.severity == Severity::Error));
    let inspection = compile_schema_document_model(MARKDOWN_SCHEMA_URI, SCHEMA);
    assert!(!inspection.is_ready_for_validation());
    let mut pending = EngineContext::default();
    pending.schema_document_models.inspect_candidate(inspection);
    assert!(LifecycleRegistry::with_builtin_adapters()
        .load(&source, &pending)
        .ast_stream
        .is_none());
    for unsafe_source in [
        SCHEMA.replace(
            "@code=\"cem.markdown.unsupported_encoding\" @severity=\"error\"",
            "@code=\"cem.markdown.unsupported_encoding\" @severity=\"warning\"",
        ),
        SCHEMA.replace(
            "{diagnostic @code=\"cem.markdown.embedded_html_rejected\" @severity=\"error\"}",
            "",
        ),
    ] {
        let invalid = compile_schema_document_model_from_retained(
            MARKDOWN_SCHEMA_URI,
            &retain(&unsafe_source),
        );
        assert!(!invalid.is_ready_for_validation());
        assert!(invalid
            .compile_diagnostics
            .iter()
            .any(|d| d.code == INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE));
    }
}

#[test]
fn admitted_markdown_originals_drive_native_parse_validation_and_color_pipeline() {
    let originals = [
        (
            "package.cem",
            SchemaAssetSourceKind::Manifest,
            include_str!("../../../../schema-packages/markdown/v1/package.cem"),
        ),
        ("schema/markdown.cem", SchemaAssetSourceKind::Schema, SCHEMA),
        (
            "formatters/markdown-format-document.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!(
                "../../../../schema-packages/markdown/v1/formatters/markdown-format-document.cemt"
            ),
        ),
        (
            "colorizers/markdown-color-document.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!(
                "../../../../schema-packages/markdown/v1/colorizers/markdown-color-document.cemt"
            ),
        ),
    ];
    let index = SchemaAssetPackIndex {
        format_version: SCHEMA_ASSET_PACK_FORMAT.into(),
        packages: vec![SchemaRegistry::with_builtin_schemas()
            .schema(MARKDOWN_SCHEMA_URI)
            .unwrap()
            .clone()
            .into()],
        sources: originals
            .iter()
            .map(|(path, kind, text)| SchemaAssetSourceDescriptor {
                package_id: "markdown".into(),
                version: "1.0.0".into(),
                path: format!("schema-packages/markdown/v1/{path}"),
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
    let encoded = index.to_bytes().unwrap();
    let pack = SchemaAssetPack::from_index_bytes(
        &encoded,
        &schema_asset_sha256(&encoded),
        SchemaAssetPackLimits::default(),
    )
    .unwrap();
    let mut context = EngineContext::default();
    let source = input(
        b"# Native article\n\n**Ready**\n",
        "text/markdown; charset=utf-8; variant=CommonMark",
    );
    #[cfg(feature = "external-schema-assets")]
    {
        let (unbound, diagnostics) =
            markdown_document_ast_from_source_bytes(MarkdownSourceValidationRequest {
                bytes: &source.bytes,
                source_uri: &source.uri,
                content_type: source.identity.as_ref().unwrap().content_type.as_deref(),
            });
        assert!(unbound.is_none());
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_asset.missing"));
        let blocked = LifecycleRegistry::with_builtin_adapters().load(&source, &context);
        assert!(blocked.ast_stream.is_none());
        assert!(blocked
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_asset.missing"));
    }
    context
        .schema_package_replacement_grants
        .push(SchemaPackageReplacementGrant {
            package_id: "markdown".into(),
            expected_origin: SchemaPackageOrigin::Builtin,
            replacement_manifest_uri: "schema-packages/markdown/v1/package.cem".into(),
        });
    let mut session = SchemaAssetSession::new(pack, context);
    assert!(session.activate(&[("markdown", "1.0.0")]).is_err());
    for (path, _, text) in originals {
        session
            .admit(
                "markdown",
                "1.0.0",
                &format!("schema-packages/markdown/v1/{path}"),
                text.as_bytes().to_vec(),
            )
            .unwrap();
    }
    let activated = session.activate(&[("markdown", "1.0.0")]).unwrap();
    assert!(
        !activated.iter().any(|d| d.severity.is_hard_violation()),
        "{activated:?}"
    );
    let loaded = LifecycleRegistry::with_builtin_adapters().load(&source, session.context());
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let LoadedInputAstStream::MarkdownDocument(document) = loaded.ast_stream.unwrap() else {
        panic!("retained Markdown stream")
    };
    assert_eq!(document.source.uri, source.uri);
    assert!(document
        .events
        .iter()
        .all(|event| event.source_range.byte_length > 0));
    for projection in [ParseProjection::Ast, ParseProjection::Events] {
        let parsed = crate::real::RealCemMlEngine
            .parse(ParseRequest {
                input: source.clone(),
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
            .unwrap();
        assert!(
            !parsed
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            "{projection:?}: {:?}",
            parsed.diagnostics
        );
        let serialized = serde_json::to_string(&parsed.primary).unwrap();
        assert!(serialized.contains("Native article"), "{serialized}");
        let presentation = std::str::from_utf8(
            &parsed
                .primary_bytes
                .as_ref()
                .expect("source presentation")
                .bytes,
        )
        .unwrap();
        assert!(presentation.contains(&source.uri), "{presentation}");
        if projection == ParseProjection::Ast {
            assert!(serialized.contains("\"frames\":[{"), "{serialized}");
        } else {
            assert!(
                serialized.contains("\"byteRange\":{\"len\":"),
                "{serialized}"
            );
        }
    }
    let invalid = input(
        b"<script>private()</script>\n",
        "text/markdown; charset=utf-8",
    );
    let refused = LifecycleRegistry::with_builtin_adapters()
        .load_for_source_validation(&invalid, session.context());
    assert!(refused
        .diagnostics
        .iter()
        .any(|d| d.code == MARKDOWN_EMBEDDED_HTML_REJECTED_CODE && d.severity.is_hard_violation()));
    for projection in [ParseProjection::Ast, ParseProjection::Events] {
        let refused = crate::real::RealCemMlEngine
            .parse(ParseRequest {
                input: invalid.clone(),
                projection,
                fail_level: FailLevel::Validate,
                preserve_source_offsets: true,
                presentation_scope: None,
                context: session.context().clone(),
            })
            .unwrap();
        assert!(refused.diagnostics.iter().any(|d| {
            d.code == MARKDOWN_EMBEDDED_HTML_REJECTED_CODE && d.severity.is_hard_violation()
        }));
        assert!(
            !refused.diagnostics.iter().any(|d| {
                d.code.starts_with("cem.tokenizer.") || d.code.starts_with("cem.schema.")
            }),
            "{:?}",
            refused.diagnostics
        );
    }
    let reader = session.pack().conversion_reader();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &session.context().schema_registry,
        conversion_registry: &session.context().converter_registry,
        package_artifact_reader: Some(&reader),
        artifact_cache: None,
    };
    let execution = execute_markdown_document_output_pipeline_with_environment(
        &environment,
        document,
        &crate::run_config::ScopeConfig {
            cemt_formatter_profile: Some("tabular".into()),
            cemt_color_profile: Some("html".into()),
            ..Default::default()
        },
        Some(&source.uri),
    );
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
    let output = execution
        .output
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .unwrap();
    assert!(
        output.contains("<span") && output.contains("Native article"),
        "{output}"
    );
    assert!(Arc::ptr_eq(
        execution
            .formatted_materialized_cemt_tree
            .as_ref()
            .unwrap()
            .owner(),
        execution
            .colored_materialized_cemt_tree
            .as_ref()
            .unwrap()
            .owner()
    ));
}
