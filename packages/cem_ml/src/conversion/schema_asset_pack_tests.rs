use super::*;
use crate::{
    operation_control::OperationControl,
    schema::asset_pack::{
        builtin_schema_asset_pack_export, schema_asset_sha256, SchemaAssetPack,
        SchemaAssetPackLimits,
    },
    validation::csv::{csv_document_ast_from_source_bytes, CsvSourceValidationRequest},
};

const COMPACT: &str = "schema-packages/csv/v1/formatters/compact.cemt";
const TERMINAL: &str = "schema-packages/csv/v1/colorizers/terminal.cemt";

fn compact_artifact(registry: &ConversionRegistry) -> &ConversionPackageArtifactDescriptor {
    registry
        .package_artifacts()
        .find(|artifact| artifact.path == COMPACT)
        .expect("native CSV formatter descriptor")
}

#[test]
fn authoritative_missing_artifact_refuses_embedded_fallback_while_legacy_reader_retains_it() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let reads = RefCell::new(0);
    let missing = |_: &ConversionPackageArtifactDescriptor| {
        *reads.borrow_mut() += 1;
        Err("selected immutable source is unavailable".to_owned())
    };
    let legacy = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: Some(&missing),
        artifact_cache: None,
    };
    let artifact = compact_artifact(&conversions);
    let embedded = read_cem_tree_cemt_output_stage_artifact(&legacy, artifact).unwrap();
    assert_eq!(embedded.uri, COMPACT);
    assert!(!embedded.bytes.is_empty());
    let strict_reader = AuthoritativeConversionPackageArtifactReader::new(&missing);
    let strict = ConversionOutputPipelineEnvironment {
        package_artifact_reader: Some(&strict_reader),
        ..legacy
    };
    assert_eq!(
        read_cem_tree_cemt_output_stage_artifact(&strict, artifact).unwrap_err(),
        "selected immutable source is unavailable"
    );
    assert_eq!(*reads.borrow(), 2);
}

#[test]
fn authoritative_missing_artifact_cannot_reuse_cached_embedded_module_options() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let cache = ConversionOutputPipelineArtifactCache::default();
    let legacy = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: Some(&cache),
    };
    let artifact = compact_artifact(&conversions);
    parse_cem_tree_cemt_output_artifact_module_options(&legacy, artifact).unwrap();
    assert!(!cache.module_options.borrow().is_empty());
    let missing = |_: &ConversionPackageArtifactDescriptor| {
        Err("selected immutable source is unavailable".to_owned())
    };
    let strict_reader = AuthoritativeConversionPackageArtifactReader::new(&missing);
    let strict = ConversionOutputPipelineEnvironment {
        package_artifact_reader: Some(&strict_reader),
        ..legacy
    };
    let error = parse_cem_tree_cemt_output_artifact_module_options(&strict, artifact)
        .expect_err("cached embedded options cannot satisfy authoritative source admission");
    assert!(
        error.contains("selected immutable source is unavailable"),
        "{error}"
    );
    parse_cem_tree_cemt_output_artifact_module_options(&legacy, artifact).unwrap();
}

#[test]
fn admitted_csv_pack_runs_native_formatter_and_colorizer_with_original_source_provenance() {
    let export = builtin_schema_asset_pack_export().unwrap();
    let selected_paths = [
        "schema-packages/csv/v1/package.cem",
        "schema-packages/csv/v1/schema/csv.cem",
        COMPACT,
        TERMINAL,
    ];
    let selected_bytes: usize = export
        .sources
        .iter()
        .filter(|source| selected_paths.contains(&source.descriptor.path.as_str()))
        .map(|source| source.source.len())
        .sum();
    let index = export.index.to_bytes().unwrap();
    let mut pack = SchemaAssetPack::from_index_bytes(
        &index,
        &schema_asset_sha256(&index),
        SchemaAssetPackLimits {
            max_total_bytes: selected_bytes,
            ..SchemaAssetPackLimits::default()
        },
    )
    .unwrap();
    let control = OperationControl::default();
    for path in selected_paths {
        let source = export
            .sources
            .iter()
            .find(|source| source.descriptor.path == path)
            .unwrap();
        pack.admit_source(
            "csv",
            &source.descriptor.version,
            path,
            source.source.as_bytes().to_vec(),
            &source.descriptor.content_type,
            &control,
            control.root_scope(),
        )
        .unwrap();
    }
    assert_eq!(pack.admitted_sources(), selected_paths.len());
    assert_eq!(pack.admitted_bytes(), selected_bytes);
    let loaded = pack.load_package("csv", "1.0.0").unwrap();
    let schemas = pack.schema_registry().unwrap();
    let manifest = std::str::from_utf8(
        pack.source_bytes("csv", &loaded.descriptor.version, selected_paths[0])
            .unwrap(),
    )
    .unwrap();
    let mut conversions = ConversionRegistry::new();
    for artifact in conversion_package_artifacts_from_validated_schema_package_manifest(
        "csv",
        manifest,
        "schema-packages/csv/v1",
    )
    .unwrap()
    {
        conversions.register_package_artifact(artifact);
    }
    for path in [COMPACT, TERMINAL] {
        let imported = pack
            .import_source("csv", &loaded.descriptor.version, path)
            .unwrap();
        assert_eq!(imported.tree.source_uri(), path);
        assert!(Arc::ptr_eq(
            imported.tree.ast_owner(),
            imported.captured.as_ref().unwrap().document(),
        ));
    }
    let reads = RefCell::new(Vec::new());
    let read = |artifact: &ConversionPackageArtifactDescriptor| {
        let descriptor = pack
            .source_descriptor(
                &artifact.package_id,
                &loaded.descriptor.version,
                &artifact.path,
            )
            .map_err(|error| error.to_string())?;
        let provenance = pack
            .source_provenance(
                &artifact.package_id,
                &loaded.descriptor.version,
                &artifact.path,
            )
            .map_err(|error| error.to_string())?;
        reads.borrow_mut().push(provenance.canonical_uri.to_owned());
        Ok(ConversionPackageArtifactRead {
            uri: provenance.canonical_uri.to_owned(),
            bytes: pack
                .source_bytes(
                    &artifact.package_id,
                    &loaded.descriptor.version,
                    &artifact.path,
                )
                .map_err(|error| error.to_string())?
                .to_vec(),
            content_type: Some(descriptor.content_type.clone()),
        })
    };
    let strict_reader = AuthoritativeConversionPackageArtifactReader::new(&read);
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: Some(&strict_reader),
        artifact_cache: None,
    };
    let target =
        TransformTemplateEncodingTarget::new(CSV_CONTENT_TYPE, CSV_SCHEMA_URI, "csv-document");
    let stage = cem_tree_cemt_output_stage(
        &environment,
        CSV_FORMAT_CEMT_STAGE_SPEC,
        &target,
        Some("compact"),
        Some("csv.format-document"),
    )
    .unwrap();
    assert_eq!(stage.template_uri, COMPACT);
    let stage = cem_tree_cemt_output_stage(
        &environment,
        CSV_COLOR_CEMT_STAGE_SPEC,
        &target,
        Some("terminal"),
        Some("csv.color-document"),
    )
    .unwrap();
    assert_eq!(stage.template_uri, TERMINAL);
    let (document, diagnostics) = csv_document_ast_from_source_bytes(CsvSourceValidationRequest {
        bytes: b"id,name\n1,Ada\n",
        source_uri: "memory://schema-asset-pack/input.csv",
        content_type: Some("text/csv; header=present"),
    });
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let document = document.unwrap();
    let scope = ScopeConfig {
        cemt_formatter_profile: Some("compact".to_owned()),
        cemt_color_profile: Some("terminal".to_owned()),
        ..ScopeConfig::default()
    };
    let execution = execute_csv_document_output_pipeline_with_environment(
        &environment,
        document.clone(),
        &scope,
        Some("memory://schema-asset-pack/input.csv"),
    );
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
    let embedded_environment = ConversionOutputPipelineEnvironment {
        package_artifact_reader: None,
        ..environment
    };
    let expected = execute_csv_document_output_pipeline_with_environment(
        &embedded_environment,
        document.clone(),
        &scope,
        Some("memory://schema-asset-pack/input.csv"),
    );
    assert!(
        expected.diagnostics.is_empty(),
        "{:?}",
        expected.diagnostics
    );
    assert_eq!(execution.output, expected.output);
    assert!(execution
        .output
        .as_ref()
        .and_then(Value::as_str)
        .unwrap()
        .contains('\u{1b}'));
    assert!(execution
        .output
        .as_ref()
        .and_then(Value::as_str)
        .unwrap()
        .contains("Ada"));
    let formatted = execution.formatted_materialized_cemt_tree.as_ref().unwrap();
    let colored = execution.colored_materialized_cemt_tree.as_ref().unwrap();
    assert!(Arc::ptr_eq(formatted.owner(), colored.owner()));
    assert!(colored.color_overlay().is_some());
    assert!(reads
        .borrow()
        .iter()
        .all(|path| path == COMPACT || path == TERMINAL));
    assert!(reads.borrow().iter().any(|path| path == COMPACT));
    assert!(reads.borrow().iter().any(|path| path == TERMINAL));
    let missing_scope = ScopeConfig {
        cemt_formatter_profile: Some("pretty".to_owned()),
        ..ScopeConfig::default()
    };
    let refused = execute_csv_document_output_pipeline_with_environment(
        &environment,
        document,
        &missing_scope,
        Some("memory://schema-asset-pack/input.csv"),
    );
    assert!(!refused.diagnostics.is_empty());
    assert!(refused.output.is_none());
    assert!(refused
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("no embedded fallback")));
    assert_eq!(pack.admitted_sources(), selected_paths.len());
    assert_eq!(pack.admitted_bytes(), selected_bytes);
}
