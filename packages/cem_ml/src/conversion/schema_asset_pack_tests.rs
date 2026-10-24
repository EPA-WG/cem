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
    let loaded = pack.owned_package("csv", "1.0.0").unwrap();
    let schemas = pack.schema_registry().unwrap();
    let mut conversions = ConversionRegistry::new();
    for artifact in conversion_package_artifacts_from_schema_package(&loaded).unwrap() {
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
    let admitted_reader = pack.conversion_reader();
    let read = |artifact: &ConversionPackageArtifactDescriptor| {
        let source = admitted_reader.read(artifact)?;
        reads.borrow_mut().push(source.uri.clone());
        Ok(source)
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

#[test]
fn output_artifact_cache_revalidates_source_bytes_for_legacy_and_authoritative_readers() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let artifact = compact_artifact(&conversions);
    let cache = ConversionOutputPipelineArtifactCache::default();
    let original = builtin_schema_package_artifact_source("csv", COMPACT)
        .unwrap()
        .source;
    let bytes = RefCell::new(original.as_bytes().to_vec());
    let read = |_: &ConversionPackageArtifactDescriptor| {
        Ok(ConversionPackageArtifactRead {
            uri: COMPACT.to_owned(),
            bytes: bytes.borrow().clone(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
        })
    };
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: Some(&read),
        artifact_cache: Some(&cache),
    };
    parse_cem_tree_cemt_output_artifact_module_options(&environment, artifact).unwrap();
    let first_count = cache.module_options.borrow().len();
    *bytes.borrow_mut() = b"{this is not a valid output module".to_vec();
    assert!(
        parse_cem_tree_cemt_output_artifact_module_options(&environment, artifact).is_err(),
        "a descriptor cannot reuse a different body's parsed options"
    );
    *bytes.borrow_mut() = original.as_bytes().to_vec();
    let strict_reader = AuthoritativeConversionPackageArtifactReader::new(&read);
    let strict = ConversionOutputPipelineEnvironment {
        package_artifact_reader: Some(&strict_reader),
        ..environment
    };
    parse_cem_tree_cemt_output_artifact_module_options(&strict, artifact).unwrap();
    assert!(cache.module_options.borrow().len() >= first_count);
    // Identical immutable bytes may reuse admitted cached options even for a
    // strict reader; admission still runs on every lookup.
    let previous = cache.module_options.borrow().len();
    parse_cem_tree_cemt_output_artifact_module_options(&strict, artifact).unwrap();
    assert_eq!(cache.module_options.borrow().len(), previous);
}

#[test]
fn strict_source_cache_qualifies_uri_mime_descriptor_and_revalidates_refusal() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let artifact = compact_artifact(&conversions);
    let cache = ConversionOutputPipelineArtifactCache::default();
    let selected = RefCell::new(Some(ConversionPackageArtifactRead {
        uri: COMPACT.into(),
        bytes: builtin_schema_package_artifact_source("csv", COMPACT)
            .unwrap()
            .source
            .as_bytes()
            .to_vec(),
        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.into()),
    }));
    let read = |_: &ConversionPackageArtifactDescriptor| {
        selected
            .borrow()
            .clone()
            .ok_or_else(|| "admission revoked".to_owned())
    };
    let strict = AuthoritativeConversionPackageArtifactReader::new(&read);
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: Some(&strict),
        artifact_cache: Some(&cache),
    };
    for _ in 0..2 {
        parse_cem_tree_cemt_output_artifact_module_options(&environment, artifact).unwrap();
    }
    assert_eq!(
        cache.module_options.borrow().len(),
        1,
        "admitted strict reads may cache parsed options"
    );
    selected.borrow_mut().as_mut().unwrap().uri = "memory://another-source.cemt".into();
    parse_cem_tree_cemt_output_artifact_module_options(&environment, artifact).unwrap();
    assert_eq!(cache.module_options.borrow().len(), 2);
    selected.borrow_mut().as_mut().unwrap().content_type =
        Some("application/cem; charset=utf-8".into());
    parse_cem_tree_cemt_output_artifact_module_options(&environment, artifact).unwrap();
    assert_eq!(cache.module_options.borrow().len(), 3);
    let mut descriptor = artifact.clone();
    descriptor.generated = !descriptor.generated;
    parse_cem_tree_cemt_output_artifact_module_options(&environment, &descriptor).unwrap();
    assert_eq!(cache.module_options.borrow().len(), 4);
    *selected.borrow_mut() = None;
    assert!(
        parse_cem_tree_cemt_output_artifact_module_options(&environment, artifact)
            .unwrap_err()
            .contains("admission revoked")
    );
}

#[test]
fn output_artifact_source_cache_stays_bounded_across_admitted_source_revisions() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let artifact = compact_artifact(&conversions);
    let cache = ConversionOutputPipelineArtifactCache::default();
    let uri = RefCell::new(String::new());
    let read = |_: &ConversionPackageArtifactDescriptor| {
        Ok(ConversionPackageArtifactRead {
            uri: uri.borrow().clone(),
            bytes: b"{module @version=\"1.0.0\"}".to_vec(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.into()),
        })
    };
    let strict = AuthoritativeConversionPackageArtifactReader::new(&read);
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: Some(&strict),
        artifact_cache: Some(&cache),
    };
    for index in 0..260 {
        *uri.borrow_mut() = format!("memory://source-{index}.cemt");
        parse_cem_tree_cemt_output_artifact_module_options(&environment, artifact).unwrap();
    }
    assert_eq!(cache.module_options.borrow().len(), 256);
}

#[test]
fn owned_packed_package_outlives_store_and_missing_package_never_loads_embedded_sources() {
    let export = builtin_schema_asset_pack_export().unwrap();
    let index = export.index.to_bytes().unwrap();
    let mut pack = SchemaAssetPack::from_index_bytes(
        &index,
        &schema_asset_sha256(&index),
        SchemaAssetPackLimits::default(),
    )
    .unwrap();
    assert!(pack
        .owned_package("csv", "1.0.0")
        .unwrap_err()
        .code
        .contains("missing"));
    let control = OperationControl::default();
    let mut packages = Vec::new();
    for package_id in ["csv", "cem-dom-projection"] {
        let descriptor = export
            .index
            .packages
            .iter()
            .find(|package| package.package_id == package_id)
            .unwrap();
        for source in export.sources.iter().filter(|source| {
            source.descriptor.package_id == package_id
                && (source.descriptor.path.ends_with("/package.cem")
                    || source.descriptor.path == descriptor.source)
        }) {
            pack.admit_source(
                package_id,
                "1.0.0",
                &source.descriptor.path,
                source.source.as_bytes().to_vec(),
                &source.descriptor.content_type,
                &control,
                control.root_scope(),
            )
            .unwrap();
        }
        packages.push(pack.owned_package(package_id, "1.0.0").unwrap());
    }
    let clones = packages.clone();
    assert!(std::ptr::eq::<str>(
        packages[0].schema_source.as_ref(),
        clones[0].schema_source.as_ref()
    ));
    drop(pack);
    drop(export);
    drop(packages);
    assert!(
        !conversion_package_artifacts_from_schema_package(&clones[0])
            .unwrap()
            .is_empty()
    );
    assert!(!conversion_descriptors_from_schema_package(&clones[1])
        .unwrap()
        .is_empty());
}
