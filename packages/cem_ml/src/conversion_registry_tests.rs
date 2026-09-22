//! Registry assembly must preserve package contracts and independent ownership.
use super::*;

// Reference the previous construction path: resolve each package independently
// through the public loader, preserving the separate descriptor/artifact passes.
fn independently_loaded_metadata() -> (
    Vec<ConversionDescriptor>,
    Vec<ConversionPackageArtifactDescriptor>,
) {
    let descriptors = builtin_converter_package_schema_uris()
        .iter()
        .flat_map(|uri| {
            conversion_descriptors_from_schema_package(&load_builtin_schema_package(uri).unwrap())
                .unwrap()
        })
        .collect();
    let artifacts = builtin_converter_package_schema_uris()
        .iter()
        .flat_map(|uri| {
            conversion_package_artifacts_from_schema_package(
                &load_builtin_schema_package(uri).unwrap(),
            )
            .unwrap()
        })
        .collect();
    (descriptors, artifacts)
}

#[test]
fn builtin_assembly_preserves_all_metadata_order_selection_and_typed_output() {
    let (descriptors, artifacts) = independently_loaded_metadata();
    assert_eq!(builtin_conversion_descriptors(), descriptors);
    assert_eq!(builtin_conversion_package_artifacts(), artifacts);
    let mut reference = ConversionRegistry::new();
    for descriptor in descriptors {
        reference.register(descriptor).unwrap();
    }
    for artifact in artifacts {
        reference.register_package_artifact(artifact);
    }
    let actual = ConversionRegistry::with_builtin_converters();
    assert!(actual.converters().eq(reference.converters()));
    assert!(actual.package_artifacts().eq(reference.package_artifacts()));
    let target =
        TransformTemplateEncodingTarget::new(CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, "cem-tree");
    for profile in [None, Some("tabular")] {
        let select = |registry: &ConversionRegistry| {
            registry
                .select_package_artifact_for_output_stage(
                    "cem-ml",
                    "formatter",
                    Some(CEM_TRANSFORM_CONTENT_TYPE),
                    Some(CEM_TRANSFORM_SCHEMA_URI),
                    &target,
                    None,
                    "cem.format-tree",
                    profile,
                    None,
                )
                .unwrap()
                .unwrap()
                .clone()
        };
        assert_eq!(select(&actual), select(&reference));
        let schemas = SchemaRegistry::with_builtin_schemas();
        let mut pipeline = direct_cem_output_pipeline();
        pipeline.cemt_options.formatter_profile = profile.map(str::to_owned);
        pipeline.cemt_insertion_context.formatter_profile = profile.map(str::to_owned);
        pipeline.writer_insertion_context.formatter_profile = profile.map(str::to_owned);
        for (format, source) in [
            ("xml", "<r empty=''><![CDATA[<raw>🍒]]><?keep inert?></r>"),
            ("json", r#"{"fruit":"<raw>🍒","empty":""}"#),
            ("yaml", "fruit: '<raw>🍒'\nempty: ''"),
            ("csv", "fruit,empty\n<raw>🍒,\n"),
        ] {
            let owner =
                crate::import::import_data(source, format, "cem", "memory:registry").unwrap();
            let stream = Arc::new(crate::projection::cem_tree_inspection(owner.clone()));
            let render = |conversions: &ConversionRegistry| {
                let environment = ConversionOutputPipelineEnvironment {
                    schema_registry: &schemas,
                    conversion_registry: conversions,
                    package_artifact_reader: None,
                    artifact_cache: None,
                };
                let result = execute_conversion_output_pipeline_from_cem_tree_with_environment(
                    &environment,
                    &pipeline,
                    stream.clone(),
                    Some(owner.node(0).unwrap().source.clone()),
                    vec![],
                    "registry-test",
                    None,
                    Some(owner.source_uri()),
                );
                assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
                assert!(Arc::ptr_eq(
                    result
                        .raw_cem_tree
                        .as_ref()
                        .unwrap()
                        .owner()
                        .source_owner()
                        .unwrap(),
                    &owner
                ));
                result.output.unwrap()
            };
            assert_eq!(render(&actual), render(&reference), "{format}/{profile:?}");
        }
    }
}

#[test]
fn builtin_assemblies_keep_mutations_local_and_duplicate_errors_intact() {
    let mut changed = ConversionRegistry::with_builtin_converters();
    let independent = ConversionRegistry::with_builtin_converters();
    let mut descriptor = changed.converters().next().unwrap().clone();
    assert!(matches!(
        changed.register(descriptor.clone()),
        Err(ConversionRegistryError::DuplicateConverterId { .. })
    ));
    descriptor.id = "test:local-converter".into();
    changed.register(descriptor).unwrap();
    let mut artifact = changed.package_artifacts().next().unwrap().clone();
    artifact.path = "test/local-formatter.cemt".into();
    changed.register_package_artifact(artifact);
    let fresh = ConversionRegistry::with_builtin_converters();
    for untouched in [&independent, &fresh] {
        assert!(untouched.converter("test:local-converter").is_none());
        assert!(untouched
            .package_artifacts()
            .all(|item| item.path != "test/local-formatter.cemt"));
    }
    assert!(fresh.converters().eq(independent.converters()));
    assert!(fresh
        .package_artifacts()
        .eq(independent.package_artifacts()));
}
