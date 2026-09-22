//! Opt-in diagnostic timings. No elapsed-time assertion is a correctness gate.
use cem_ml::{
    conversion::{
        conversion_descriptors_from_schema_package,
        conversion_package_artifacts_from_schema_package, direct_cem_output_pipeline,
        execute_conversion_output_pipeline_from_cem_tree_with_environment,
        ConversionOutputPipelineEnvironment, ConversionRegistry,
    },
    import::import_data,
    projection::cem_tree_inspection,
    schema::{
        package_loader::BuiltinSchemaPackage, package_sources::builtin_schema_package_source,
        registry as schemas, SchemaRegistry,
    },
};
use cem_ql::{
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{
        compile_template, compile_template_module_closure, render_compiled_template,
        render_plan_to_html, CompileTemplateOptions, TemplateData, TemplateModuleClosure,
        TemplateModuleSource,
    },
};
use std::{hint::black_box, sync::Arc, time::Instant};

const VIEW: &str = include_str!("../../cem-elements/demo/data-tree-view.cemt");
const REQUEST: &str = include_str!("../../cem-elements/demo/data-tree-request.cemt");
const SMALL: &str = "<orchard xmlns:f=\"urn:fruit\"><f:fruit color=\"\">🍒</f:fruit><f:fruit>pre<![CDATA[<raw>🍋]]><?keep inert?>post</f:fruit></orchard>";
const LOCAL: &str = include_str!("../../cem-elements/demo/tree-source.xml");

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: Vec<(&str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), vec![value]))
            .collect(),
    )
}
fn measure<T>(case: &str, stage: &str, mut operation: impl FnMut() -> T) -> T {
    let start = Instant::now();
    let mut value = black_box(operation());
    let first = start.elapsed().as_secs_f64() * 1000.0;
    let mut times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let next = black_box(operation());
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        value = next;
    }
    times.sort_by(f64::total_cmp);
    println!("{case}\t{stage}\tfirst_ms={first:.3}\twarm_median_ms={:.3}\twarm_min_ms={:.3}\twarm_max_ms={:.3}", times[2], times[0], times[4]);
    value
}
fn input(source: &str) -> TemplateData {
    TemplateData::default()
        .with_binding("format", ItemStream::once(text("xml")))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    record(vec![("nodes", record(vec![("text", text(source))]))]),
                ),
                (
                    "slices",
                    record(vec![
                        ("branch.1.1", text("edit-0")),
                        ("branch.1.2", text("edit-0")),
                    ]),
                ),
                ("eventPayloads", record(vec![])),
            ])),
        )
}

// Profiling-only candidate: construct one schema registry for the entire
// conversion registry, retaining the production package order and validation.
// Exact descriptor/artifact equality below guards this duplicated test list.
fn conversions_with_one_schema_registry() -> ConversionRegistry {
    let schemas = SchemaRegistry::with_builtin_schemas();
    let mut conversions = ConversionRegistry::new();
    for uri in [
        schemas::CEM_ML_SCHEMA_URI,
        schemas::HTML_SCHEMA_URI,
        schemas::XML_SCHEMA_URI,
        schemas::CEM_DOM_PROJECTION_SCHEMA_URI,
        schemas::CEM_AST_PROJECTION_SCHEMA_URI,
        schemas::CEM_EVENTS_PROJECTION_SCHEMA_URI,
        schemas::CEM_QL_SCHEMA_URI,
        schemas::JSON_VALUE_SCHEMA_URI,
        schemas::JSON_SCHEMA_SCHEMA_URI,
        schemas::CSV_SCHEMA_URI,
        schemas::YAML_SCHEMA_URI,
        schemas::MARKDOWN_SCHEMA_URI,
        schemas::CSS_SCHEMA_URI,
        schemas::RELAX_NG_SCHEMA_URI,
        schemas::XHTML_SCHEMA_URI,
        schemas::SVG_SCHEMA_URI,
        schemas::MATHML_SCHEMA_URI,
        schemas::XSLT_SCHEMA_URI,
    ] {
        let descriptor = schemas.schema(uri).unwrap();
        let source = builtin_schema_package_source(&descriptor.package_id).unwrap();
        let package = BuiltinSchemaPackage {
            descriptor: descriptor.clone(),
            manifest_source: source.manifest_source,
            schema_source: source.schema_source,
        };
        for descriptor in conversion_descriptors_from_schema_package(&package).unwrap() {
            conversions.register(descriptor).unwrap();
        }
        for artifact in conversion_package_artifacts_from_schema_package(&package).unwrap() {
            conversions.register_package_artifact(artifact);
        }
    }
    conversions
}

#[test]
#[ignore = "profiling fixture: run with --release --ignored --nocapture"]
fn profile_authored_tree_stages() {
    let full = measure("shared", "compile-view", || {
        compile_template(VIEW, &CompileTemplateOptions::default())
    });
    assert!(full.diagnostics.is_empty(), "{:?}", full.diagnostics);
    // Counterfactual diagnostic only: retain every authored branch operation while
    // omitting the one inspection expression. The authored file is never edited.
    let expression = "{$cemml:inspect(document)}";
    assert_eq!(VIEW.matches(expression).count(), 1);
    let omitted = VIEW.replace(expression, "inspection omitted for profile");
    let branches = compile_template(&omitted, &CompileTemplateOptions::default());
    let inspection = compile_template(
        "{pre | {$cemml:inspect(doc)}}",
        &CompileTemplateOptions {
            host_bindings: vec!["doc".into()],
            ..Default::default()
        },
    );
    let hash = |source: &str| {
        cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes()).header_value()
    };
    let request = measure("shared", "compile-request", || {
        compile_template_module_closure(
            REQUEST,
            &TemplateModuleClosure {
                root_uri: "memory:/data-tree-request.cemt".into(),
                root_content_hash: hash(REQUEST),
                modules: vec![TemplateModuleSource {
                    alias: "view".into(),
                    parent_uri: None,
                    uri: "memory:/data-tree-view.cemt".into(),
                    content_hash: hash(VIEW),
                    source: VIEW.into(),
                }],
                ..Default::default()
            },
            &CompileTemplateOptions::default(),
        )
    });
    assert!(request.diagnostics.is_empty(), "{:?}", request.diagnostics);
    for (case, source) in [("small", SMALL), ("local-request", LOCAL)] {
        let owner = measure(case, "cem-import", || {
            import_data(source, "xml", "cem", "memory:profile.xml").unwrap()
        });
        let stream = measure(case, "inspection-projection", || {
            Arc::new(cem_tree_inspection(owner.clone()))
        });
        let registry = measure(
            case,
            "schema-registry",
            SchemaRegistry::with_builtin_schemas,
        );
        let conversions = measure(
            case,
            "conversion-registry",
            ConversionRegistry::with_builtin_converters,
        );
        let cloned = measure(case, "clone-built-conversion-registry", || {
            conversions.clone()
        });
        let assembled = measure(case, "conversion-registry-one-schema-registry", || {
            conversions_with_one_schema_registry()
        });
        assert!(assembled.converters().eq(conversions.converters()));
        assert!(assembled
            .package_artifacts()
            .eq(conversions.package_artifacts()));
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &registry,
            conversion_registry: &conversions,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let mut pipeline = direct_cem_output_pipeline();
        pipeline.cemt_options.formatter_profile = Some("tabular".into());
        pipeline.cemt_insertion_context.formatter_profile = Some("tabular".into());
        pipeline.writer_insertion_context.formatter_profile = Some("tabular".into());
        let formatted = measure(case, "inspection-output-pipeline", || {
            execute_conversion_output_pipeline_from_cem_tree_with_environment(
                &environment,
                &pipeline,
                stream.clone(),
                owner.node(0).map(|node| node.source.clone()),
                vec![],
                "profile",
                None,
                Some(owner.source_uri()),
            )
        });
        assert!(
            formatted.diagnostics.is_empty(),
            "{:?}",
            formatted.diagnostics
        );
        assert!(formatted
            .output
            .as_ref()
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("{ast"));
        // Both candidate constructions preserve the typed pipeline's exact
        // output and successful diagnostics. Neither changes production code.
        for candidate in [&cloned, &assembled] {
            let candidate_environment = ConversionOutputPipelineEnvironment {
                conversion_registry: candidate,
                ..environment
            };
            let candidate_formatted =
                execute_conversion_output_pipeline_from_cem_tree_with_environment(
                    &candidate_environment,
                    &pipeline,
                    stream.clone(),
                    owner.node(0).map(|node| node.source.clone()),
                    vec![],
                    "profile",
                    None,
                    Some(owner.source_uri()),
                );
            assert!(
                candidate_formatted.diagnostics.is_empty(),
                "{:?}",
                candidate_formatted.diagnostics
            );
            assert_eq!(candidate_formatted.output, formatted.output);
        }
        let inspection_data = TemplateData::default()
            .with_binding("doc", ItemStream::once(imported_cem_tree(owner.clone())));
        let inspected = measure(case, "inspection-expression", || {
            render_compiled_template(&inspection, &inspection_data)
        });
        assert!(
            inspected.diagnostics.is_empty(),
            "{:?}",
            inspected.diagnostics
        );
        let data = input(source);
        let plan = measure(case, "authored-render", || {
            render_compiled_template(&full, &data)
        });
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        let branch_plan = measure(case, "render-with-inspection-omitted", || {
            render_compiled_template(&branches, &data)
        });
        assert!(
            branch_plan.diagnostics.is_empty(),
            "{:?}",
            branch_plan.diagnostics
        );
        let html = measure(case, "html-export", || render_plan_to_html(&plan));
        let branch_html = render_plan_to_html(&branch_plan);
        assert_eq!(
            html.split("<h3>Branches</h3>").nth(1),
            branch_html.split("<h3>Branches</h3>").nth(1)
        );
        assert!(html.contains("aria-label=\"Selected branches\">2</output>"));
        let request_data = TemplateData::default()
            .with_binding("sourceUrl", ItemStream::once(text("./tree-source.xml")))
            .with_binding(
                "datadom",
                ItemStream::once(record(vec![(
                    "slices",
                    record(vec![
                        ("sourceUrl", text("./tree-source.xml")),
                        (
                            "resource",
                            record(vec![
                                ("state", text("loaded")),
                                ("resourceRevision", Item::Atomic(AtomValue::Integer(1))),
                                ("data", imported_cem_tree(owner.clone())),
                            ]),
                        ),
                    ]),
                )])),
            );
        let requested = measure(case, "retained-request-render", || {
            render_compiled_template(&request, &request_data)
        });
        assert!(
            requested.diagnostics.is_empty(),
            "{:?}",
            requested.diagnostics
        );
        assert!(render_plan_to_html(&requested).contains("Request state:"));
    }
}
