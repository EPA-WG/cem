use super::*;

const FORMAT: &str = "cem+test://packages/cem-ml/v1/formatters/cem-format-tree.cemt";
const FORMAT_HELPER: &str = "cem+test://packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt";
const COLOR: &str = "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree.cemt";
const COLOR_HELPER: &str = "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt";
const READS: [&str; 8] = [
    FORMAT_HELPER,
    COLOR_HELPER, // Format binding: discover package output functions.
    FORMAT,
    FORMAT_HELPER, // Execute formatter: load body, re-admit helper.
    FORMAT_HELPER,
    COLOR_HELPER, // Color binding: discover package output functions.
    COLOR,
    COLOR_HELPER, // Execute colorizer: load body, re-admit helper.
];

struct Fixture {
    schemas: SchemaRegistry,
    conversions: ConversionRegistry,
    sources: BTreeMap<&'static str, &'static str>,
}

impl Fixture {
    fn new() -> Self {
        let mut conversions = ConversionRegistry::new();
        let mut sources = BTreeMap::new();
        for (path, kind, function, function_profile, formatter_profile, color_profile) in [
            (
                FORMAT,
                "formatter",
                "cem.format-tree",
                None,
                Some("compact"),
                None,
            ),
            (
                FORMAT_HELPER,
                CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND,
                "cem.format-tree.apply-stage",
                Some("cem.format-tree"),
                Some("compact"),
                None,
            ),
            (
                COLOR,
                "colorizer",
                "cem.color-tree",
                Some("css-custom-properties"),
                None,
                Some("classes"),
            ),
            (
                COLOR_HELPER,
                CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND,
                "cem.color-tree.apply-stage",
                Some("css-custom-properties"),
                None,
                None,
            ),
        ] {
            conversions.register_package_artifact(ConversionPackageArtifactDescriptor {
                package_id: "cem-ml".into(),
                kind: kind.into(),
                path: path.into(),
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.into()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.into()),
                target_content_type: Some(CEM_ML_CONTENT_TYPE.into()),
                target_schema: Some(CEM_ML_SCHEMA_URI.into()),
                target_category: Some("cem-tree".into()),
                function_name: Some(function.into()),
                function_profile: function_profile.map(str::to_owned),
                formatter_profile: formatter_profile.map(str::to_owned),
                color_profile: color_profile.map(str::to_owned),
                generated: false,
            });
            let canonical = path.replace(
                "cem+test://packages/cem-ml/v1/",
                "schema-packages/cem-ml/v1/",
            );
            sources.insert(
                path,
                builtin_schema_package_artifact_source("cem-ml", &canonical)
                    .expect("canonical CEMT fixture")
                    .source,
            );
        }
        Self {
            schemas: SchemaRegistry::with_builtin_schemas(),
            conversions,
            sources,
        }
    }

    fn read(
        &self,
        artifact: &ConversionPackageArtifactDescriptor,
    ) -> ConversionPackageArtifactRead {
        ConversionPackageArtifactRead {
            uri: artifact.path.clone(),
            bytes: self.sources[artifact.path.as_str()].as_bytes().to_vec(),
            content_type: artifact.content_type.clone(),
        }
    }

    fn run(
        &self,
        reader: &ConversionPackageArtifactReader<'_>,
        cache: Option<&ConversionOutputPipelineArtifactCache>,
    ) -> ConversionOutputPipelineExecution {
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &self.schemas,
            conversion_registry: &self.conversions,
            package_artifact_reader: Some(reader),
            artifact_cache: cache,
        };
        execute_conversion_output_pipeline_with_environment(
            &environment,
            &direct_html_output_pipeline(),
            serde_json::json!({"kind":"element","name":"main","children":[{"kind":"text","value":"Ready"}]}),
            None,
            Vec::new(),
            "test-dom-to-html-cemt",
            Some("output"),
            Some("test.cem"),
        )
    }
}

fn assert_success(execution: &ConversionOutputPipelineExecution) {
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
    for (actual, adapter, function) in [
        (
            &execution.format_execution,
            CEM_TREE_FORMAT_CEMT_ADAPTER_ID,
            "cem.format-tree",
        ),
        (
            &execution.color_execution,
            CEM_TREE_COLOR_CEMT_ADAPTER_ID,
            "cem.color-tree",
        ),
    ] {
        assert_eq!(
            *actual,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: adapter.into(),
                function_name: function.into(),
                body_function_name: Some(function.into()),
                fallback_function_name: None,
            })
        );
    }
    let output = execution
        .output
        .as_ref()
        .and_then(Value::as_str)
        .expect("native writer text");
    assert!(output.contains("<main"));
    assert!(output.contains("cem-color-syntax-name"));
}

pub(super) fn assert_environment_reader_trace() {
    let fixture = Fixture::new();
    for explicit_cache in [false, true] {
        for strict in [false, true] {
            let cache = ConversionOutputPipelineArtifactCache::default();
            let reads = RefCell::new(Vec::new());
            let read = |artifact: &ConversionPackageArtifactDescriptor| {
                reads.borrow_mut().push(artifact.path.clone());
                Ok(fixture.read(artifact))
            };
            let authoritative = AuthoritativeConversionPackageArtifactReader::new(&read);
            let reader: &ConversionPackageArtifactReader<'_> =
                if strict { &authoritative } else { &read };
            // A warm explicit cache still re-admits all eight reads on each run.
            for _ in 0..2 {
                reads.borrow_mut().clear();
                assert_success(&fixture.run(reader, explicit_cache.then_some(&cache)));
                assert_eq!(*reads.borrow(), READS.map(str::to_owned));
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Change {
    RenameBody,
    Malformed,
    Refuse,
}

#[test]
fn conversion_output_pipeline_execution_rechecks_changed_or_revoked_helpers() {
    let fixture = Fixture::new();
    for explicit_cache in [false, true] {
        for strict in [false, true] {
            for (step, path, function) in [
                (4, FORMAT_HELPER, "cem.format-tree.apply-stage"),
                (8, COLOR_HELPER, "cem.color-tree.apply-stage"),
            ] {
                for change in [Change::RenameBody, Change::Malformed, Change::Refuse] {
                    let cache = ConversionOutputPipelineArtifactCache::default();
                    let reads = RefCell::new(Vec::new());
                    let armed = std::cell::Cell::new(true);
                    let read = |artifact: &ConversionPackageArtifactDescriptor| {
                        reads.borrow_mut().push(artifact.path.clone());
                        let mut source = fixture.read(artifact);
                        if armed.get() && reads.borrow().len() >= step && artifact.path == path {
                            match change {
                                Change::RenameBody => {
                                    source.bytes = String::from_utf8(source.bytes)
                                        .unwrap()
                                        .replace(function, "replaced.helper.apply-stage")
                                        .into_bytes()
                                }
                                Change::Malformed => {
                                    source.bytes = b"{this is not a valid output module".to_vec()
                                }
                                Change::Refuse => return Err("freshness admission revoked".into()),
                            }
                        }
                        Ok(source)
                    };
                    let authoritative = AuthoritativeConversionPackageArtifactReader::new(&read);
                    let reader: &ConversionPackageArtifactReader<'_> =
                        if strict { &authoritative } else { &read };
                    let execution = fixture.run(reader, explicit_cache.then_some(&cache));
                    let messages = execution
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.message.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    assert!(
                        execution
                            .diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.severity.is_hard_violation()),
                        "{strict}/{explicit_cache}/{change:?}: {messages}"
                    );
                    assert!(messages.contains(path), "{messages}");
                    match change {
                        Change::RenameBody => {
                            assert!(messages.contains(function), "{messages}");
                            assert!(messages.contains("did not declare"), "{messages}");
                        }
                        Change::Malformed => {
                            assert!(messages.contains("could not parse"), "{messages}")
                        }
                        Change::Refuse => assert!(
                            messages.contains("freshness admission revoked"),
                            "{messages}"
                        ),
                    }
                    assert!(execution.output.is_none());
                    assert!(execution.native_writer_output.is_none());
                    assert!(execution.writer_elapsed_ns.is_none());
                    assert!(execution.output_spans.is_empty());
                    assert_eq!(
                        *reads.borrow(),
                        READS[..step]
                            .iter()
                            .map(|path| path.to_string())
                            .collect::<Vec<_>>()
                    );
                    // Restoring admission must not retain a failed or stale body.
                    armed.set(false);
                    reads.borrow_mut().clear();
                    assert_success(&fixture.run(reader, explicit_cache.then_some(&cache)));
                    assert_eq!(*reads.borrow(), READS.map(str::to_owned));
                }
            }
        }
    }
}

#[test]
fn conversion_output_pipeline_requalifies_helper_uri_and_mime_before_cache_hits() {
    let fixture = Fixture::new();
    for (step, path) in [(4, FORMAT_HELPER), (8, COLOR_HELPER)] {
        let cache = ConversionOutputPipelineArtifactCache::default();
        let reads = RefCell::new(Vec::new());
        let uri = format!("{path}?revision=2");
        let mime = "application/cem; charset=utf-8";
        let read = |artifact: &ConversionPackageArtifactDescriptor| {
            reads.borrow_mut().push(artifact.path.clone());
            let mut source = fixture.read(artifact);
            if reads.borrow().len() >= step && artifact.path == path {
                source.uri = uri.clone();
                source.content_type = Some(mime.into());
            }
            Ok(source)
        };
        let reader = AuthoritativeConversionPackageArtifactReader::new(&read);
        assert_success(&fixture.run(&reader, Some(&cache)));
        assert_eq!(*reads.borrow(), READS.map(str::to_owned));
        let options = cache.module_options.borrow();
        let admitted = options
            .iter()
            .find(|(key, _)| key.descriptor.path == path && key.uri == uri)
            .expect("fresh helper owner");
        assert_eq!(admitted.0.content_type.as_deref(), Some(mime));
        assert_eq!(
            admitted.0.digest,
            crate::schema::asset_pack::schema_asset_sha256(fixture.sources[path].as_bytes())
        );
        assert!(admitted.1.is_ok());
        assert!(options
            .keys()
            .any(|key| key.descriptor.path == path && key.uri == path));
    }
}
