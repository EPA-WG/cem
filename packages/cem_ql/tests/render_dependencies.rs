use cem_ml::dependency_identity::{DependencyVersion, RENDER_DEPENDENCY_INPUTS};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::read_dependencies::{
    render_compiled_template_with_dependencies, RenderFreshness,
};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
    TemplateData,
};

fn revision(value: u64) -> DependencyVersion {
    DependencyVersion::Revision {
        owner: "native-host".into(),
        generation: 1,
        revision: value,
    }
}
fn freshness() -> RenderFreshness {
    RenderFreshness {
        environment: revision(1),
        inputs: RENDER_DEPENDENCY_INPUTS
            .iter()
            .map(|axis| (*axis, revision(1)))
            .collect(),
    }
}

#[test]
fn aggregates_taken_branch_and_loop_reads_without_changing_the_plan() {
    let data = TemplateData::default()
        .with_binding(
            "flag",
            ItemStream::once(Item::Atomic(AtomValue::Boolean(true))),
        )
        .with_binding(
            "left",
            ItemStream::once(Item::Atomic(AtomValue::String("Ada".into()))),
        )
        .with_binding(
            "right",
            ItemStream::once(Item::Atomic(AtomValue::String("Lin".into()))),
        );
    let artifact = compile_template(
        "{cem:if @test=flag | {p | {$left}}}{cem:for-each @select=left @as=row | {span | {$row}}}",
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    let tracked = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
    assert!(
        tracked.output.plan.diagnostics.is_empty(),
        "{:?}",
        tracked.output.plan.diagnostics
    );
    assert_eq!(
        render_plan_to_html(&tracked.output.plan),
        render_plan_to_html(&render_compiled_template(&artifact, &data))
    );
    let receipt = tracked.dependencies.unwrap();
    assert_eq!(
        receipt.binding_names().collect::<Vec<_>>(),
        vec!["flag", "left", "row"]
    );
    assert!(receipt.matches(&artifact, &data, &freshness()));
}

fn compile(source: &str, data: &TemplateData) -> cem_ql::render::TemplateArtifact {
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            source_uri: Some("memory:render-example".into()),
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    artifact
}

#[test]
fn every_provider_axis_owner_generation_and_environment_participates() {
    let data = TemplateData::default();
    let artifact = compile("{p | static}", &data);
    let versions = freshness();
    let receipt = render_compiled_template_with_dependencies(&artifact, &data, &versions)
        .dependencies
        .unwrap();
    for axis in RENDER_DEPENDENCY_INPUTS {
        for version in [
            revision(2),
            DependencyVersion::Opaque,
            DependencyVersion::Revision {
                owner: "other-provider".into(),
                generation: 1,
                revision: 1,
            },
            DependencyVersion::Revision {
                owner: "native-host".into(),
                generation: 2,
                revision: 1,
            },
        ] {
            let mut changed = versions.clone();
            changed.inputs.insert(*axis, version);
            assert!(!receipt.matches(&artifact, &data, &changed), "{axis:?}");
        }
        let mut missing = versions.clone();
        missing.inputs.remove(axis);
        let result = render_compiled_template_with_dependencies(&artifact, &data, &missing);
        assert!(result.dependencies.is_err());
        assert_eq!(render_plan_to_html(&result.output.plan), "<p>static</p>");
    }
    let mut changed = versions.clone();
    changed.environment = revision(2);
    assert!(!receipt.matches(&artifact, &data, &changed));
    assert!(render_compiled_template_with_dependencies(
        &artifact,
        &data,
        &RenderFreshness::default()
    )
    .dependencies
    .is_err());
}

#[test]
fn declarations_datadom_variables_named_templates_and_provenance_are_aggregated() {
    let data = TemplateData::default().with_binding(
        "input",
        ItemStream::once(Item::Atomic(AtomValue::String("Ada".into()))),
    );
    let source = r#"{module |
        {attribute @name=label @select=input}
        {template @name=greeting | {param @name=name}{body | {strong | {$name}}}}
        {body |
            {cem:variable @name=selected @select=datadom.attributes.label}
            {call @template=greeting @with:name="{$selected}"}
        }}"#;
    let artifact = compile(source, &data);
    let tracked = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
    assert!(
        tracked.output.plan.diagnostics.is_empty(),
        "{:?}",
        tracked.output.plan.diagnostics
    );
    let ordinary = render_compiled_template(&artifact, &data);
    let actual = cem_ql::render::render_plan_to_html_with_source_map(&tracked.output.plan);
    let expected = cem_ql::render::render_plan_to_html_with_source_map(&ordinary);
    assert_eq!(actual.rendered, expected.rendered);
    assert_eq!(
        format!("{:?}", actual.output_spans),
        format!("{:?}", expected.output_spans)
    );
    assert_eq!(
        tracked.output.plan.host_attribute_updates,
        ordinary.host_attribute_updates
    );
    assert_eq!(
        tracked
            .dependencies
            .unwrap()
            .binding_names()
            .collect::<Vec<_>>(),
        vec!["datadom", "input", "name", "selected"]
    );
}

#[test]
fn repeated_loop_queries_are_deduplicated_and_shadowing_is_not_external_freshness() {
    let data = TemplateData::default();
    let artifact = compile("{cem:variable @name=row @select='\"outer\"'}{cem:for-each @select='(1, 2, 3)' @as=row | {i | {$row}}}{b | {$row}}", &data);
    let tracked = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
    assert!(
        tracked.output.plan.diagnostics.is_empty(),
        "{:?}",
        tracked.output.plan.diagnostics
    );
    assert_eq!(
        render_plan_to_html(&tracked.output.plan),
        "<i>1</i><i>2</i><i>3</i><b>outer</b>"
    );
    let receipt = tracked.dependencies.unwrap();
    assert_eq!(receipt.distinct_query_count(), 4);
    assert_eq!(receipt.evaluation_count(), 6);
    assert_eq!(receipt.binding_names().collect::<Vec<_>>(), vec!["row"]);
}

#[test]
fn template_provenance_contracts_and_effective_policy_invalidate() {
    use cem_ml::schema::document_model::AttributeValueContract;
    let data = TemplateData::default();
    let artifact = compile("{p | static}", &data);
    let receipt = render_compiled_template_with_dependencies(&artifact, &data, &freshness())
        .dependencies
        .unwrap();
    assert!(!receipt.matches(&compile("{p | changed}", &data), &data, &freshness()));
    let other_uri = compile_template(
        "{p | static}",
        &CompileTemplateOptions {
            source_uri: Some("memory:other-origin".into()),
            ..Default::default()
        },
    );
    assert!(!receipt.matches(&other_uri, &data, &freshness()));
    for target in 0..3 {
        let mut changed = data.clone();
        let contracts = match target {
            0 => &mut changed.value_types,
            1 => &mut changed.attribute_contracts,
            _ => &mut changed.input_attribute_contracts,
        };
        contracts.insert("label".into(), AttributeValueContract::default());
        assert!(!receipt.matches(&artifact, &changed, &freshness()));
    }
    let policy = cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(64);
    assert!(!receipt.matches_with_policy(&artifact, &data, &freshness(), policy));
}

#[test]
fn repaired_failed_and_cancelled_renders_never_issue_receipts() {
    use cem_ml::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        scheduler::AbortSignal,
    };
    use cem_ql::render::read_dependencies::render_compiled_template_with_dependencies_and_control;
    let data = TemplateData::default();
    for source in [
        "{p | {$1 / 0}}",
        "{try | {p | partial}{$1 / 0}{catch @as=e | {b | repaired}}}",
    ] {
        let artifact = compile(source, &data);
        let actual = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
        assert_eq!(
            render_plan_to_html(&actual.output.plan),
            render_plan_to_html(&render_compiled_template(&artifact, &data))
        );
        assert!(actual.dependencies.is_err());
    }
    let artifact = compile("{p | static}", &data);
    let abort = AbortSignal::new();
    abort.abort();
    let control = OperationControl::new(abort);
    let actual = render_compiled_template_with_dependencies_and_control(
        &artifact,
        &data,
        &freshness(),
        &control,
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(actual.dependencies.is_err());
    assert!(actual.output.plan.nodes.is_empty());
    let control = OperationControl::with_root_policy(
        AbortSignal::new(),
        cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(8),
    )
    .unwrap();
    let actual = render_compiled_template_with_dependencies_and_control(
        &artifact,
        &data,
        &freshness(),
        &control,
        ROOT_EXECUTION_SCOPE_ID,
    );
    let receipt = actual.dependencies.unwrap();
    assert!(receipt.matches_with_policy(
        &artifact,
        &data,
        &freshness(),
        control
            .scope_tree()
            .scope(ROOT_EXECUTION_SCOPE_ID)
            .unwrap()
            .effective_policy
    ));
}

#[test]
fn opaque_native_callbacks_execute_for_every_render_attempt() {
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    #[derive(Debug)]
    struct Counter(Arc<AtomicUsize>);
    impl NativeQueryFunction for Counter {
        fn call(&self, _: NativeQueryRequest<'_>) -> ItemStream {
            self.0.fetch_add(1, Ordering::SeqCst);
            ItemStream::once(Item::Atomic(AtomValue::String("fresh".into())))
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let mut data = TemplateData::default();
    data.native_functions
        .register("counter", 0, Counter(calls.clone()))
        .unwrap();
    let artifact = compile("{p | {$native:call(\"counter\")}}", &data);
    for _ in 0..2 {
        let result = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
        assert_eq!(render_plan_to_html(&result.output.plan), "<p>fresh</p>");
        assert!(result.dependencies.is_err());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn custom_template_handlers_execute_but_cannot_attest_purity_with_query_stamps() {
    use cem_ml::source_map::SourceMapStack;
    use cem_ql::render::read_dependencies::render_compiled_template_with_calls_and_dependencies;
    use cem_ql::render::{RenderPlanAttribute, TemplateCallHandler, TemplateCallResult};
    use std::cell::Cell;
    struct Handler(Cell<usize>);
    impl TemplateCallHandler for Handler {
        fn handles(&self, tag: &str) -> bool {
            tag == "custom"
        }
        fn call(
            &self,
            _: &[RenderPlanAttribute],
            _: &SourceMapStack,
            _: &TemplateData,
            _: bool,
        ) -> TemplateCallResult {
            self.0.set(self.0.get() + 1);
            TemplateCallResult {
                plan: render_compiled_template(
                    &compile_template("{b | called}", &Default::default()),
                    &TemplateData::default(),
                ),
                failure: None,
            }
        }
    }
    let handler = Handler(Cell::new(0));
    let data = TemplateData::default();
    let artifact = compile("{custom}", &data);
    for _ in 0..2 {
        let result = render_compiled_template_with_calls_and_dependencies(
            &artifact,
            &data,
            &freshness(),
            None,
            &handler,
            false,
        );
        assert_eq!(render_plan_to_html(&result.output.plan), "<b>called</b>");
        assert!(result.dependencies.is_err());
    }
    assert_eq!(handler.0.get(), 2);
}

#[test]
fn aggregate_metadata_bounds_do_not_change_rendering() {
    use cem_ql::render::read_dependencies::MAX_RENDER_QUERIES;
    let data = TemplateData::default();
    let artifact = compile("{p | static}", &data);
    let mut large = freshness();
    large.environment = DependencyVersion::Revision {
        owner: "x".repeat(1024 * 1024),
        generation: 1,
        revision: 1,
    };
    let result = render_compiled_template_with_dependencies(&artifact, &data, &large);
    assert!(result.dependencies.is_err());
    assert_eq!(render_plan_to_html(&result.output.plan), "<p>static</p>");
    let source = (0..=MAX_RENDER_QUERIES)
        .map(|i| format!("{{${i}}}"))
        .collect::<String>();
    let artifact = compile(&source, &data);
    let result = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
    assert!(matches!(
        result.dependencies,
        Err(cem_ql::render::read_dependencies::RenderTrackingError::Bounds)
    ));
    assert_eq!(
        render_plan_to_html(&result.output.plan),
        render_plan_to_html(&render_compiled_template(&artifact, &data))
    );
}

#[test]
fn local_match_template_focus_is_recorded_under_the_complete_provider_snapshot() {
    let data = TemplateData::default().with_binding(
        "input",
        ItemStream::once(Item::Atomic(AtomValue::String("Ada".into()))),
    );
    let artifact = compile("{module | {template @match=true | {p | {$node}:{$.}}}{body | {apply-templates @select=input}}}", &data);
    let result = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
    assert!(
        result.output.plan.diagnostics.is_empty(),
        "{:?}",
        result.output.plan.diagnostics
    );
    assert_eq!(render_plan_to_html(&result.output.plan), "<p>Ada:Ada</p>");
    let receipt = result.dependencies.unwrap();
    assert!(receipt.reads_current_item());
    assert_eq!(
        receipt.binding_names().collect::<Vec<_>>(),
        vec!["input", "node"]
    );
}

#[test]
fn render_receipts_do_not_retain_native_input_owners() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    use std::sync::Arc;
    #[derive(Debug)]
    struct View(Arc<()>);
    impl QueryItemView for View {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "test.pure-record"
        }
        fn identity(&self) -> String {
            format!("{:p}", Arc::as_ptr(&self.0))
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            (name == "name").then(|| vec![Item::Atomic(AtomValue::String("Ada".into()))])
        }
    }
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let data =
        TemplateData::default().with_binding("input", ItemStream::once(Item::native(View(owner))));
    let artifact = compile("{p | {$input.name}}", &data);
    let result = render_compiled_template_with_dependencies(&artifact, &data, &freshness());
    assert_eq!(render_plan_to_html(&result.output.plan), "<p>Ada</p>");
    let receipt = result.dependencies.unwrap();
    drop(result.output);
    drop(data);
    assert!(weak.upgrade().is_none());
    assert_eq!(receipt.binding_names().collect::<Vec<_>>(), vec!["input"]);
}

#[test]
fn existing_native_provider_snapshots_supply_policy_scope_resolution_and_closure_freshness() {
    use cem_ml::{
        content_cache::ContentHash,
        dependency_identity::{
            DependencyClosureState, DependencyLayer, DependencySnapshot, SourceDependency,
        },
    };
    let data = TemplateData::default();
    let artifact = compile("{p | static}", &data);
    let mut snapshot = DependencySnapshot {
        layer: DependencyLayer::RenderProjection,
        engine_version: format!("cem-ml/{};cem-ql/{}", cem_ml::VERSION, cem_ql::VERSION),
        format_version: "native-render-test/1".into(),
        closure_state: DependencyClosureState::Ready,
        root_uri: "memory:example".into(),
        sources: [(
            "memory:example".into(),
            SourceDependency {
                content_type: "application/cem".into(),
                schema: None,
                version: DependencyVersion::Content(ContentHash::from_blake3(b"{p | static}")),
            },
        )]
        .into(),
        imports: vec![],
        options: Default::default(),
        policy: revision(1),
        resolution: revision(1),
        scope: revision(1),
        inputs: freshness().inputs,
    };
    let contract = RenderFreshness::from_snapshot(&snapshot).unwrap();
    let receipt = render_compiled_template_with_dependencies(&artifact, &data, &contract)
        .dependencies
        .unwrap();
    assert!(receipt.matches(&artifact, &data, &contract));
    for axis in 0..4 {
        let mut changed = snapshot.clone();
        match axis {
            0 => changed.policy = revision(2),
            1 => changed.resolution = revision(2),
            2 => changed.scope = revision(2),
            _ => changed.sources.get_mut("memory:example").unwrap().version = revision(2),
        }
        assert!(!receipt.matches(
            &artifact,
            &data,
            &RenderFreshness::from_snapshot(&changed).unwrap()
        ));
    }
    for state in [
        DependencyClosureState::Pending,
        DependencyClosureState::Failed,
        DependencyClosureState::Released,
    ] {
        snapshot.closure_state = state;
        assert!(RenderFreshness::from_snapshot(&snapshot).is_err());
    }
    snapshot.closure_state = DependencyClosureState::Ready;
    snapshot.inputs.insert(
        cem_ml::dependency_identity::DependencyInput::Providers,
        DependencyVersion::Opaque,
    );
    assert!(RenderFreshness::from_snapshot(&snapshot).is_err());
    snapshot.inputs = freshness().inputs;
    snapshot.layer = DependencyLayer::TemplateCompilation;
    assert!(RenderFreshness::from_snapshot(&snapshot).is_err());
}
