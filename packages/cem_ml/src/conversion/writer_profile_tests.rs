//! Opt-in attribution and bounded counterfactuals; production is unchanged.
use super::*;
use crate::{import::import_data, parser::tree::RetainedCemTree, projection::cem_tree_inspection};
use std::{hint::black_box, time::Duration};

#[derive(Default)]
struct Stage {
    calls: usize,
    elapsed: Duration,
}
type Stages = BTreeMap<String, Stage>;
thread_local! {
    static ACTIVE: RefCell<Option<Stages>> = const { RefCell::new(None) };
}

pub(crate) struct Span(Option<(String, std::time::Instant)>);
impl Span {
    pub(crate) fn new(label: &str) -> Self {
        Self(ACTIVE.with(|active| {
            active
                .borrow()
                .as_ref()
                .map(|_| (label.to_owned(), std::time::Instant::now()))
        }))
    }
    pub(crate) fn detail(label: &str, detail: &str) -> Self {
        Self(ACTIVE.with(|active| {
            active
                .borrow()
                .as_ref()
                .map(|_| (format!("{label}/{detail}"), std::time::Instant::now()))
        }))
    }
    fn finish(&mut self) {
        if let Some((label, start)) = self.0.take() {
            let elapsed = start.elapsed();
            ACTIVE.with(|active| {
                if let Some(stages) = active.borrow_mut().as_mut() {
                    let stage = stages.entry(label).or_default();
                    stage.calls += 1;
                    stage.elapsed += elapsed;
                }
            });
        }
    }
    pub(super) fn next(&mut self, label: &str) {
        self.finish();
        *self = Self::new(label);
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        self.finish();
    }
}
fn measure<T>(run: impl FnOnce() -> T) -> (T, Stages) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|active| *active.borrow_mut() = None);
        }
    }
    ACTIVE.with(|active| assert!(active.replace(Some(Stages::new())).is_none()));
    let _reset = Reset;
    let value = run();
    let stages = ACTIVE.with(|active| active.borrow_mut().take().unwrap());
    (value, stages)
}

fn pipeline() -> ConversionOutputPipeline {
    let mut pipeline = direct_cem_output_pipeline();
    pipeline.cemt_options.formatter_profile = Some("tabular".into());
    pipeline.cemt_insertion_context.formatter_profile = Some("tabular".into());
    pipeline.writer_insertion_context.formatter_profile = Some("tabular".into());
    pipeline
}
fn write(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    owner: &Arc<RetainedCemTree>,
) -> ConversionOutputPipelineExecution {
    write_pipeline(environment, owner, &pipeline())
}
fn write_pipeline(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    owner: &Arc<RetainedCemTree>,
    pipeline: &ConversionOutputPipeline,
) -> ConversionOutputPipelineExecution {
    execute_conversion_output_pipeline_from_cem_tree_with_environment(
        environment,
        pipeline,
        Arc::new(cem_tree_inspection(owner.clone())),
        owner.node(0).map(|node| node.source.clone()),
        vec![],
        "cemml:inspect",
        None,
        Some(owner.source_uri()),
    )
}
fn assert_same(
    actual: &ConversionOutputPipelineExecution,
    expected: &ConversionOutputPipelineExecution,
    owner: &Arc<RetainedCemTree>,
) {
    assert!(actual.diagnostics.is_empty(), "{:?}", actual.diagnostics);
    assert_eq!(actual.output, expected.output);
    assert_eq!(actual.source_map, expected.source_map);
    assert_eq!(actual.output_spans, expected.output_spans);
    assert_eq!(actual.format_execution, expected.format_execution);
    assert_eq!(actual.color_execution, expected.color_execution);
    assert!(actual.colored_cemt_tree.is_none());
    assert!(actual.color_elapsed_ns.is_none());
    for artifact in [&actual.raw_cem_tree, &actual.formatted_cemt_tree] {
        assert!(Arc::ptr_eq(
            artifact.as_ref().unwrap().owner().source_owner().unwrap(),
            owner
        ));
    }
    // Compare the API's explicit debug projection, never use it as an AST handoff.
    assert_eq!(
        rmp_serde::to_vec_named(&actual.formatted_cem_tree).unwrap(),
        rmp_serde::to_vec_named(&expected.formatted_cem_tree).unwrap()
    );
}
fn fork_cache(
    cache: &ConversionOutputPipelineArtifactCache,
) -> ConversionOutputPipelineArtifactCache {
    let _profile = Span::new("candidate/fork-metadata");
    ConversionOutputPipelineArtifactCache {
        module_options: RefCell::new(cache.module_options.borrow().clone()),
    }
}

// Supply the counterfactual through the public package-reader contract. No
// production helper or evaluator behavior is replaced by the profiling hooks.
fn single_build_reader(
    artifact: &ConversionPackageArtifactDescriptor,
) -> Result<ConversionPackageArtifactRead, String> {
    let source = builtin_schema_package_artifact_source(&artifact.package_id, &artifact.path)
        .ok_or_else(|| "no built-in source".to_owned())?;
    let mut bytes = source.source.as_bytes().to_vec();
    if source.path == "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt" {
        const BEFORE: &str = r#"match(typeOf(call("cem.format-tree.build-node-list", { subject: $subject })), {
                array: call("cem.format-tree.format-inter-node-whitespace", {
                    subject: call("cem.format-tree.build-node-list", { subject: $subject }),
                    lineEnding: $lineEnding
                }),
                default: call("cem.format-tree.build-node-list", { subject: $subject })
            })"#;
        const AFTER: &str = r#"call("cem.format-tree.format-inter-node-whitespace", {
                subject: call("cem.format-tree.build-node-list", { subject: $subject }),
                lineEnding: $lineEnding
            })"#;
        assert_eq!(source.source.matches(BEFORE).count(), 1);
        bytes = source.source.replacen(BEFORE, AFTER, 1).into_bytes();
    }
    Ok(ConversionPackageArtifactRead {
        uri: source.path.to_owned(),
        bytes,
        content_type: artifact.content_type.clone(),
    })
}

#[test]
fn single_build_candidate_preserves_native_output_and_release() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let candidate = ConversionOutputPipelineEnvironment {
        package_artifact_reader: Some(&single_build_reader),
        ..environment
    };
    for (format, source) in [
        (
            "xml",
            "<?keep inert?><r empty=''><![CDATA[<raw>🍒]]><!--note--><a>two</a></r>",
        ),
        (
            "json",
            r#"{"fruit":"<raw>🍒","empty":"","items":[1,true,null]}"#,
        ),
        (
            "yaml",
            "fruit: '<raw>🍒'\nempty: ''\nitems: [1, true, null]",
        ),
        ("csv", "fruit,empty\n<raw>🍒,\npear,two\n"),
    ] {
        let owner = import_data(
            source,
            format,
            "cem",
            &format!("memory:single-build.{format}"),
        )
        .unwrap();
        for profile in ["compact", "pretty", "tabular"] {
            let mut pipeline = pipeline();
            pipeline.cemt_options.formatter_profile = Some(profile.into());
            pipeline.cemt_insertion_context.formatter_profile = Some(profile.into());
            pipeline.writer_insertion_context.formatter_profile = Some(profile.into());
            let (expected, before) = measure(|| write_pipeline(&environment, &owner, &pipeline));
            let (actual, after) = measure(|| write_pipeline(&candidate, &owner, &pipeline));
            assert_same(&actual, &expected, &owner);
            let label = "eval/call/cem.format-tree.build-node-list";
            assert_eq!(before[label].calls, 2);
            assert_eq!(after[label].calls, 1);
        }
        let weak = Arc::downgrade(&owner);
        drop(owner);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn single_build_candidate_preserves_recursion_errors() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let candidate = ConversionOutputPipelineEnvironment {
        package_artifact_reader: Some(&single_build_reader),
        ..environment
    };
    let mut rejected = 0;
    for depth in [0, 4, 8, 16] {
        // Inspection is a flat preorder vocabulary. Exercise formatter call
        // depth with an actual nested native output tree instead.
        let mut node = CemTreeAstNode::Text {
            value: "value".into(),
            source: Default::default(),
        };
        for _ in 0..depth {
            node = CemTreeAstNode::Element {
                name: "r".into(),
                attributes: vec![],
                children: vec![node],
                source: Default::default(),
            };
        }
        let tree = Arc::new(CemTreeAstStream::new(vec![node]));
        let run = |environment| {
            execute_conversion_output_pipeline_from_cem_tree_with_environment(
                environment,
                &pipeline(),
                tree.clone(),
                None,
                vec![],
                "writer-depth",
                None,
                Some("memory:depth"),
            )
        };
        let expected = run(&environment);
        let actual = run(&candidate);
        assert_eq!(actual.diagnostics, expected.diagnostics);
        assert_eq!(actual.output, expected.output);
        assert_eq!(actual.source_map, expected.source_map);
        assert_eq!(actual.output_spans, expected.output_spans);
        if !expected.diagnostics.is_empty() {
            rejected += 1;
            assert!(format!("{:?}", expected.diagnostics).contains("recursion limit"));
        }
    }
    assert!(rejected > 0);
}

#[test]
fn writer_cache_counterfactuals_preserve_native_output_and_release_documents() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let baseline = ConversionOutputPipelineArtifactCache::default();
    for (format, source) in [
        ("xml", "<?keep inert?><r empty=''><![CDATA[<raw>🍒]]></r>"),
        ("json", r#"{"fruit":"<raw>🍒","empty":""}"#),
        ("yaml", "fruit: '<raw>🍒'\nempty: ''"),
        ("csv", "fruit,empty\n<raw>🍒,\n"),
    ] {
        let owner = import_data(source, format, "cem", &format!("memory:writer.{format}")).unwrap();
        let expected = write(&environment, &owner);
        let warm_environment = ConversionOutputPipelineEnvironment {
            artifact_cache: Some(&baseline),
            ..environment
        };
        let actual = write(&warm_environment, &owner);
        assert_same(&actual, &expected, &owner);
        drop(actual);
        let keys = baseline.module_options.borrow().len();
        assert!(keys > 0);
        let fork = fork_cache(&baseline);
        let actual = write(
            &ConversionOutputPipelineEnvironment {
                artifact_cache: Some(&fork),
                ..environment
            },
            &owner,
        );
        assert_same(&actual, &expected, &owner);
        fork.module_options.borrow_mut().clear();
        assert_eq!(baseline.module_options.borrow().len(), keys);
        drop(actual);
        drop(expected);
        let weak = Arc::downgrade(&owner);
        drop(owner);
        assert!(weak.upgrade().is_none());
    }
}

fn profile(
    name: &str,
    mut run: impl FnMut() -> ConversionOutputPipelineExecution,
    expected: &ConversionOutputPipelineExecution,
    owner: &Arc<RetainedCemTree>,
) {
    let mut samples = Vec::new();
    for _ in 0..6 {
        let (result, mut stages) = measure(|| {
            let _span = Span::new("total");
            black_box(run())
        });
        assert_same(&result, expected, owner);
        for (label, nanos) in [
            ("reported/format", result.format_elapsed_ns),
            ("reported/writer", result.writer_elapsed_ns),
        ] {
            stages.insert(
                label.into(),
                Stage {
                    calls: 1,
                    elapsed: Duration::from_nanos(nanos.unwrap().try_into().unwrap()),
                },
            );
        }
        samples.push(stages);
    }
    for (label, first) in &samples[0] {
        let mut times: Vec<_> = samples[1..]
            .iter()
            .map(|sample| {
                sample
                    .get(label)
                    .map_or(0.0, |stage| stage.elapsed.as_secs_f64() * 1000.0)
            })
            .collect();
        times.sort_by(f64::total_cmp);
        println!("{name}\t{label}\tfirst_calls={}\twarm_calls={}\tfirst_ms={:.3}\tmedian_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}", first.calls, samples[1].get(label).map_or(0, |stage| stage.calls), first.elapsed.as_secs_f64()*1000.0, times[2], times[0], times[4]);
    }
    let mut times = Vec::new();
    for _ in 0..6 {
        let start = std::time::Instant::now();
        let result = black_box(run());
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        assert_same(&result, expected, owner);
    }
    times.remove(0);
    times.sort_by(f64::total_cmp);
    println!(
        "{name}\tunprofiled_median_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}",
        times[2], times[0], times[4]
    );
}

#[test]
#[ignore = "profiling fixture: --release --lib profile_inspection_writer -- --ignored --nocapture --test-threads=1"]
fn profile_inspection_writer() {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schemas,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let large = format!("<r>{}</r>", "<row id='1'>value</row>".repeat(32));
    for (name, source) in [
        (
            "authored",
            include_str!("../../../cem-elements/demo/tree-source.xml"),
        ),
        ("32-rows", large.as_str()),
    ] {
        let owner = import_data(source, "xml", "cem", "memory:writer-profile").unwrap();
        let expected = write(&environment, &owner);
        profile(
            &format!("{name}/per-call-cache"),
            || write(&environment, &owner),
            &expected,
            &owner,
        );
        let baseline = ConversionOutputPipelineArtifactCache::default();
        profile(
            &format!("{name}/retained-cache"),
            || {
                write(
                    &ConversionOutputPipelineEnvironment {
                        artifact_cache: Some(&baseline),
                        ..environment
                    },
                    &owner,
                )
            },
            &expected,
            &owner,
        );
        println!(
            "{name}\tbaseline_entries={}",
            baseline.module_options.borrow().len()
        );
        profile(
            &format!("{name}/forked-cache"),
            || {
                let local = fork_cache(&baseline);
                write(
                    &ConversionOutputPipelineEnvironment {
                        artifact_cache: Some(&local),
                        ..environment
                    },
                    &owner,
                )
            },
            &expected,
            &owner,
        );
        profile(
            &format!("{name}/single-build"),
            || {
                write(
                    &ConversionOutputPipelineEnvironment {
                        package_artifact_reader: Some(&single_build_reader),
                        ..environment
                    },
                    &owner,
                )
            },
            &expected,
            &owner,
        );
    }
}
