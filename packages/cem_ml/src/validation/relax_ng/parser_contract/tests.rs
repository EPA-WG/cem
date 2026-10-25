use super::*;
use crate::{
    engine::{CemMlEngine, EngineContext},
    lifecycle::LifecycleRegistry,
    schema::{
        document_model::{
            compile_schema_document_model, compile_schema_document_model_from_retained,
        },
        package_compilation::SchemaPackageSourceRegistry,
        registry::SchemaPackageOrigin,
    },
};

fn source() -> &'static str {
    include_str!("../../../../schema-packages/relax-ng/v1/schema/relax-ng.cem")
}

fn retained(source: &str) -> Arc<crate::parser::tree::RetainedCemTree> {
    SchemaPackageSourceRegistry::default()
        .retain("memory://owned/relax-ng-schema.cem", source)
        .unwrap()
}

// This fixture publishes an explicitly retained schema directly. Real asset
// admission, original-body authentication and grants are verified separately.
fn test_origin(context: &mut EngineContext) {
    context.schema_registry.set_package_origin(
        "relax-ng",
        SchemaPackageOrigin::Manifest("memory://owned/relax-ng-schema.cem".into()),
    );
}

// Named Parse response projection only: verify that native provenance reaches
// this explicit API boundary, without using JSON as a parser handoff.
fn has_native_source_frames(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object
                .get("sourceMap")
                .and_then(|map| map.get("frames"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|frames| !frames.is_empty())
                || object.values().any(has_native_source_frames)
        }
        serde_json::Value::Array(values) => values.iter().any(has_native_source_frames),
        _ => false,
    }
}

fn projected_attribute_values(
    value: &serde_json::Value,
    node: &str,
    attribute: &str,
) -> Vec<String> {
    let mut found = Vec::new();
    fn visit(value: &serde_json::Value, node: &str, attribute: &str, found: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(object) => {
                if object
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|name| name.rsplit(':').next() == Some(node))
                {
                    if let Some(attributes) = object
                        .get("attributes")
                        .and_then(serde_json::Value::as_array)
                    {
                        for field in attributes {
                            if field.get("name").and_then(serde_json::Value::as_str)
                                == Some(attribute)
                            {
                                if let Some(value) =
                                    field.get("value").and_then(serde_json::Value::as_str)
                                {
                                    found.push(value.into());
                                }
                            }
                        }
                    }
                }
                for value in object.values() {
                    visit(value, node, attribute, found);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    visit(value, node, attribute, found);
                }
            }
            _ => (),
        }
    }
    visit(value, node, attribute, &mut found);
    found
}

fn input() -> EngineInput {
    EngineInput {
        uri: "memory://source/input.rng".into(),
        bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><define name="note"><empty/></define></grammar>"#.to_vec(),
        from_format: None,
        identity: Some(FormatIdentity { content_type: Some(RELAX_NG_XML_CONTENT_TYPE.into()), schema: Some(RELAX_NG_SCHEMA_URI.into()), ..Default::default() }),
        root_scope: Default::default(),
    }
}

#[test]
fn relax_ng_parser_contract_retains_original_owner_and_refuses_inspection_only_models() {
    let owner = retained(source());
    let model = compile_schema_document_model_from_retained(RELAX_NG_SCHEMA_URI, &owner);
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    let contract = model.parser_contract.as_ref().unwrap();
    assert!(Arc::ptr_eq(contract.source_owner().unwrap(), &owner));
    assert!(contract.matches_owner(owner.ast_owner()));
    assert!(!contract.matches_owner(retained(source()).ast_owner()));

    let inspection = compile_schema_document_model(RELAX_NG_SCHEMA_URI, source());
    assert!(!inspection.is_ready_for_validation());
    assert!(inspection
        .parser_contract
        .as_ref()
        .unwrap()
        .source_owner()
        .is_none());
    let mut pending = EngineContext::default();
    pending.schema_document_models.inspect_candidate(inspection);
    test_origin(&mut pending);
    let blocked = LifecycleRegistry::with_builtin_adapters().load(&input(), &pending);
    assert!(blocked.ast_stream.is_none());
    // The external profile first refuses missing admitted package originals;
    // the complete profile reaches the unbound-model guard directly.
    let refusal = if cfg!(feature = "external-schema-assets") {
        "cem.schema_asset.missing"
    } else {
        crate::schema::document_model::MODEL_NOT_READY_CODE
    };
    assert!(
        blocked.diagnostics.iter().any(|d| d.code == refusal),
        "{:?}",
        blocked.diagnostics
    );
}

#[test]
fn relax_ng_parser_contract_refuses_missing_unknown_duplicate_and_unsafe_fact_bindings() {
    let binding = source()
        .lines()
        .find(|line| line.contains("@kind=\"relax-ng-xml-parse-report\""))
        .unwrap();
    for invalid in [
        source().replace(
            "@fact-kind=\"xml-parse-error\"",
            "@fact-kind=\"unregistered-fact\"",
        ),
        source().replace(binding, ""),
        source().replace(
            binding,
            &format!(
                "{binding}\n{}",
                binding.replace("relax-ng-xml-parse-report", "duplicate-native-fact")
            ),
        ),
        source().replace(
            "@code=\"cem.relax_ng.external_ref_rejected\" @severity=\"error\"",
            "@code=\"cem.relax_ng.external_ref_rejected\" @severity=\"warning\"",
        ),
    ] {
        let model =
            compile_schema_document_model_from_retained(RELAX_NG_SCHEMA_URI, &retained(&invalid));
        assert!(!model.is_ready_for_validation(), "{invalid}");
        assert!(
            model
                .compile_diagnostics
                .iter()
                .any(|d| d.code == INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE),
            "{:?}",
            model.compile_diagnostics
        );
        let mut context = EngineContext::default();
        context.schema_document_models.inspect_candidate(model);
        test_origin(&mut context);
        let blocked = LifecycleRegistry::with_builtin_adapters().load(&input(), &context);
        assert!(
            blocked.ast_stream.is_none(),
            "invalid model fell back to an embedded parser"
        );
    }
}

#[test]
fn relax_ng_parser_contract_uses_owned_diagnostics_and_preserves_native_source() {
    let customized = source().replace("cem.relax_ng.missing_start", "example.relax_ng.owned_fact");
    let owner = retained(&customized);
    let model = compile_schema_document_model_from_retained(RELAX_NG_SCHEMA_URI, &owner);
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    let mut context = EngineContext::default();
    context.schema_document_models.register(model);
    test_origin(&mut context);
    let input = input();
    let loaded = LifecycleRegistry::with_builtin_adapters().load(&input, &context);
    assert_eq!(loaded.adapter_id, Some("relax-ng"));
    let diagnostic = loaded
        .diagnostics
        .iter()
        .find(|d| d.code == "example.relax_ng.owned_fact")
        .expect("retained schema diagnostic");
    assert_eq!(diagnostic.uri.as_deref(), Some(input.uri.as_str()));
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(matches!(
        loaded.ast_stream,
        Some(LoadedInputAstStream::RelaxNgDocument(_))
    ));
    assert_eq!(loaded.bytes, input.bytes);
}

#[cfg(feature = "external-schema-assets")]
#[test]
fn relax_ng_contextless_parser_refuses_missing_original_contract_without_panicking() {
    assert!(builtin_schema_package_source(RELAX_NG_PACKAGE_ID).is_none());
    let input = input();
    let (document, diagnostics) =
        relax_ng_document_ast_from_source_bytes(RelaxNgSourceValidationRequest {
            bytes: &input.bytes,
            source_uri: &input.uri,
            content_type: input.identity.as_ref().unwrap().content_type.as_deref(),
        });
    assert!(document.is_none());
    assert!(diagnostics
        .iter()
        .any(|d| d.code == "cem.schema_asset.missing" && d.severity.is_hard_violation()));
    assert_eq!(diagnostics[0].uri.as_deref(), Some(input.uri.as_str()));
}

#[test]
fn admitted_relax_ng_originals_drive_native_parse_validation_and_color_pipeline() {
    use crate::{
        conversion::{
            execute_relax_ng_document_output_pipeline_with_environment,
            ConversionOutputPipelineEnvironment,
        },
        engine::SchemaPackageReplacementGrant,
        schema::{
            asset_pack::{
                schema_asset_sha256, SchemaAssetPack, SchemaAssetPackIndex, SchemaAssetPackLimits,
                SchemaAssetSourceDescriptor, SchemaAssetSourceKind, SCHEMA_ASSET_PACK_FORMAT,
            },
            asset_pack_session::SchemaAssetSession,
            registry::SchemaRegistry,
        },
    };
    // Real original manifest, schema and every declared output artifact. None
    // can borrow an embedded optional body or an unverified helper reader.
    let originals = [
        (
            "package.cem",
            SchemaAssetSourceKind::Manifest,
            include_str!("../../../../schema-packages/relax-ng/v1/package.cem"),
        ),
        (
            "schema/relax-ng.cem",
            SchemaAssetSourceKind::Schema,
            include_str!("../../../../schema-packages/relax-ng/v1/schema/relax-ng.cem"),
        ),
        (
            "formatters/compact-compact.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../../schema-packages/relax-ng/v1/formatters/compact-compact.cemt"),
        ),
        (
            "formatters/compact-pretty.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../../schema-packages/relax-ng/v1/formatters/compact-pretty.cemt"),
        ),
        (
            "formatters/compact-tabular.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../../schema-packages/relax-ng/v1/formatters/compact-tabular.cemt"),
        ),
        (
            "formatters/relax-ng-format-schema.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!(
                "../../../../schema-packages/relax-ng/v1/formatters/relax-ng-format-schema.cemt"
            ),
        ),
        (
            "formatters/xml-compact.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../../schema-packages/relax-ng/v1/formatters/xml-compact.cemt"),
        ),
        (
            "formatters/xml-pretty.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../../schema-packages/relax-ng/v1/formatters/xml-pretty.cemt"),
        ),
        (
            "formatters/xml-tabular.cemt",
            SchemaAssetSourceKind::Formatter,
            include_str!("../../../../schema-packages/relax-ng/v1/formatters/xml-tabular.cemt"),
        ),
        (
            "colorizers/compact-html.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../../schema-packages/relax-ng/v1/colorizers/compact-html.cemt"),
        ),
        (
            "colorizers/compact-md.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../../schema-packages/relax-ng/v1/colorizers/compact-md.cemt"),
        ),
        (
            "colorizers/compact-terminal.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!(
                "../../../../schema-packages/relax-ng/v1/colorizers/compact-terminal.cemt"
            ),
        ),
        (
            "colorizers/relax-ng-color-schema.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!(
                "../../../../schema-packages/relax-ng/v1/colorizers/relax-ng-color-schema.cemt"
            ),
        ),
        (
            "colorizers/xml-html.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../../schema-packages/relax-ng/v1/colorizers/xml-html.cemt"),
        ),
        (
            "colorizers/xml-md.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../../schema-packages/relax-ng/v1/colorizers/xml-md.cemt"),
        ),
        (
            "colorizers/xml-terminal.cemt",
            SchemaAssetSourceKind::Colorizer,
            include_str!("../../../../schema-packages/relax-ng/v1/colorizers/xml-terminal.cemt"),
        ),
    ];
    let index = SchemaAssetPackIndex {
        format_version: SCHEMA_ASSET_PACK_FORMAT.into(),
        packages: vec![SchemaRegistry::with_builtin_schemas()
            .schema(RELAX_NG_SCHEMA_URI)
            .unwrap()
            .clone()
            .into()],
        sources: originals
            .iter()
            .map(|(path, kind, text)| SchemaAssetSourceDescriptor {
                package_id: "relax-ng".into(),
                version: "1.0.0".into(),
                path: format!("schema-packages/relax-ng/v1/{path}"),
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
    context
        .schema_package_replacement_grants
        .push(SchemaPackageReplacementGrant {
            package_id: "relax-ng".into(),
            expected_origin: SchemaPackageOrigin::Builtin,
            replacement_manifest_uri: "schema-packages/relax-ng/v1/package.cem".into(),
        });
    let mut session = SchemaAssetSession::new(pack, context);
    assert!(session.activate(&[("relax-ng", "1.0.0")]).is_err());
    for (path, _, text) in originals {
        session
            .admit(
                "relax-ng",
                "1.0.0",
                &format!("schema-packages/relax-ng/v1/{path}"),
                text.as_bytes().to_vec(),
            )
            .unwrap();
    }
    let activated = session.activate(&[("relax-ng", "1.0.0")]).unwrap();
    assert!(
        !activated.iter().any(|d| d.severity.is_hard_violation()),
        "{activated:?}"
    );
    let model = session
        .context()
        .schema_document_models
        .resolve_for_identity(Some(RELAX_NG_SCHEMA_URI), None, None)
        .expect("active original schema model");
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(model
        .parser_contract
        .as_ref()
        .unwrap()
        .source_owner()
        .is_some());
    let invalid = LifecycleRegistry::with_builtin_adapters()
        .load_for_source_validation(&input(), session.context());
    assert!(
        invalid
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.relax_ng.missing_start" && d.severity.is_hard_violation()),
        "{:?}",
        invalid.diagnostics
    );
    for (content_type, bytes) in [
        (
            "application/relax-ng+xml",
            include_str!("../../../../schema-packages/relax-ng/v1/examples/basic-schema.rng"),
        ),
        (
            "application/relax-ng-compact-syntax",
            include_str!("../../../../schema-packages/relax-ng/v1/examples/basic-schema.rnc"),
        ),
    ] {
        let mut source = input();
        source.bytes = bytes.as_bytes().to_vec();
        source.identity.as_mut().unwrap().content_type = Some(content_type.into());
        // Exercise the real Parse command boundary under hard validation,
        // beyond lifecycle loading and formatter execution in isolation.
        for projection in [
            crate::engine::ParseProjection::Ast,
            crate::engine::ParseProjection::Events,
        ] {
            let parse = |source| {
                crate::real::RealCemMlEngine
                    .parse(crate::engine::ParseRequest {
                        input: source,
                        projection,
                        fail_level: crate::engine::FailLevel::Validate,
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
            let response = parse(source.clone());
            assert!(
                !response
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation()),
                "{content_type} {projection:?}: {:?}",
                response.diagnostics
            );
            assert!(
                has_native_source_frames(&response.primary),
                "native source frames were lost from {projection:?}"
            );
            let root = if projection == crate::engine::ParseProjection::Ast {
                "ast"
            } else {
                "event-stream"
            };
            assert_eq!(
                projected_attribute_values(&response.primary, root, "schema"),
                [RELAX_NG_SCHEMA_URI.to_owned()]
            );
            if projection == crate::engine::ParseProjection::Events {
                assert_eq!(projected_attribute_values(&response.primary, "event", "value").concat().as_bytes(), source.bytes.as_slice(),
                    "native events must retain every original XML/compact source byte in lexer order");
            }
            let projected = serde_json::to_string(&response.primary).unwrap();
            assert!(
                projected.contains(&source.uri),
                "native Parse response lost its original source URI"
            );
            let presentation = response
                .primary_bytes
                .as_ref()
                .expect("native CEM-ML Parse presentation");
            assert!(std::str::from_utf8(&presentation.bytes)
                .unwrap()
                .contains(&source.uri));
            let mut malformed = source.clone();
            let expected = if content_type == RELAX_NG_COMPACT_CONTENT_TYPE {
                malformed.bytes = b"start = element note { text".to_vec();
                "cem.relax_ng.compact_parse_error"
            } else {
                malformed.bytes = input().bytes;
                "cem.relax_ng.missing_start"
            };
            let refused = parse(malformed);
            assert!(
                refused
                    .diagnostics
                    .iter()
                    .any(|d| d.code == expected && d.severity.is_hard_violation()),
                "{:?}",
                refused.diagnostics
            );
            assert!(
                !refused
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "cem.schema.unresolved_namespace"
                        || d.code.contains("unknown_html_element")
                        || d.code.starts_with("cem.tokenizer.")),
                "native source acquired unrelated CEM diagnostics: {:?}",
                refused.diagnostics
            );
        }
        let loaded = LifecycleRegistry::with_builtin_adapters().load(&source, session.context());
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        let LoadedInputAstStream::RelaxNgDocument(document) = loaded.ast_stream.unwrap() else {
            panic!("retained relax-ng stream")
        };
        assert_eq!(document.source.uri, source.uri);
        let reader = session.pack().conversion_reader();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &session.context().schema_registry,
            conversion_registry: &session.context().converter_registry,
            package_artifact_reader: Some(&reader),
            artifact_cache: None,
        };
        let execution = execute_relax_ng_document_output_pipeline_with_environment(
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
        assert!(output.contains("<span"), "{output}");
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
}
