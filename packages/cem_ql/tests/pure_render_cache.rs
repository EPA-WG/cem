use cem_ml::{
    content_cache::ContentCacheLimits,
    dependency_identity::{DependencyAdmission, DependencyVersion, RENDER_DEPENDENCY_INPUTS},
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
};
use cem_ql::{
    render::read_dependencies::RenderFreshness,
    render::{compile_template, render_plan_to_html, TemplateData},
    render_cache::{PureRenderCache, RenderAdmission},
};
fn admission(revision: u64) -> RenderAdmission {
    RenderAdmission {
        owner: DependencyAdmission {
            owner: "consumer".into(),
            runtime_generation: 1,
            revision,
        },
        freshness: RenderFreshness {
            environment: version(1),
            inputs: RENDER_DEPENDENCY_INPUTS
                .iter()
                .map(|axis| (*axis, version(1)))
                .collect(),
        },
    }
}
fn version(revision: u64) -> DependencyVersion {
    DependencyVersion::Revision {
        owner: "provider".into(),
        generation: 1,
        revision,
    }
}
#[test]
fn real_hits_skip_evaluation_but_run_admission_and_return_independent_output() {
    let mut cache = PureRenderCache::new(ContentCacheLimits {
        max_entries: 2,
        max_bytes: 1024 * 1024,
    })
    .unwrap();
    let artifact = compile_template("{p @title=hello | {$1 + 2}}", &Default::default());
    let state = admission(1);
    let ticket = cache.begin(state.owner.clone());
    let control = OperationControl::default();
    let mut admissions = 0;
    for hit in [false, true] {
        let mut result = cache
            .render(
                &artifact,
                &TemplateData::default(),
                &ticket,
                &control,
                ROOT_EXECUTION_SCOPE_ID,
                || {
                    admissions += 1;
                    Ok(state.clone())
                },
                || Some(state.clone()),
            )
            .unwrap();
        assert_eq!(result.hit, hit);
        assert_eq!(
            render_plan_to_html(&result.plan),
            "<p title=\"hello\">3</p>"
        );
        result.plan.nodes.clear();
    }
    assert_eq!(admissions, 2);
    assert_eq!(cache.evaluations(), 1);
}

fn run(
    cache: &mut PureRenderCache,
    artifact: &cem_ql::render::TemplateArtifact,
    data: &TemplateData,
    state: &RenderAdmission,
) -> cem_ql::render_cache::CheckedPureRender {
    let ticket = cache.begin(state.owner.clone());
    cache
        .render(
            artifact,
            data,
            &ticket,
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            || Ok(state.clone()),
            || Some(state.clone()),
        )
        .unwrap()
}

#[test]
fn every_provider_axis_scope_policy_and_consumer_generation_invalidates() {
    let artifact = compile_template("{p | {$1 + 2}}", &Default::default());
    let data = TemplateData::default();
    let original = admission(1);
    let mut cache = PureRenderCache::default();
    assert!(!run(&mut cache, &artifact, &data, &original).hit);
    for axis in RENDER_DEPENDENCY_INPUTS {
        let mut changed = original.clone();
        changed.freshness.inputs.insert(*axis, version(2));
        assert!(!run(&mut cache, &artifact, &data, &changed).hit, "{axis:?}");
        assert!(run(&mut cache, &artifact, &data, &changed).hit);
    }
    for change in 0..4 {
        let mut changed = original.clone();
        match change {
            0 => changed.owner.owner = "other".into(),
            1 => changed.owner.runtime_generation += 1,
            2 => changed.freshness.environment = version(2),
            _ => {
                changed.freshness.inputs.insert(
                    cem_ml::dependency_identity::DependencyInput::Providers,
                    DependencyVersion::Revision {
                        owner: "other-provider".into(),
                        generation: 2,
                        revision: 1,
                    },
                );
            }
        }
        assert!(!run(&mut cache, &artifact, &data, &changed).hit);
    }
    let mut next_revision = original.clone();
    next_revision.owner.revision = 2;
    assert!(run(&mut cache, &artifact, &data, &next_revision).hit);
    let ticket = cache.begin(original.owner.clone());
    let control = OperationControl::with_root_policy(
        Default::default(),
        cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(8),
    )
    .unwrap();
    assert!(
        !cache
            .render(
                &artifact,
                &data,
                &ticket,
                &control,
                ROOT_EXECUTION_SCOPE_ID,
                || Ok(original.clone()),
                || Some(original.clone())
            )
            .unwrap()
            .hit
    );
}

#[test]
fn branch_template_provenance_contracts_and_changed_inputs_do_not_reuse() {
    use cem_ql::eval::{AtomValue, Item, ItemStream};
    let mut cache = PureRenderCache::default();
    let mut state = admission(1);
    let source = "{cem:if @test=flag | {p @title=selected | yes}}";
    let options = cem_ql::render::CompileTemplateOptions {
        host_bindings: vec!["flag".into()],
        source_uri: Some("memory:first".into()),
        ..Default::default()
    };
    let artifact = compile_template(source, &options);
    let mut data = TemplateData::default().with_binding(
        "flag",
        ItemStream::once(Item::Atomic(AtomValue::Boolean(true))),
    );
    assert!(!run(&mut cache, &artifact, &data, &state).hit);
    assert!(run(&mut cache, &artifact, &data, &state).hit);
    data.bindings.insert(
        "flag".into(),
        ItemStream::once(Item::Atomic(AtomValue::Boolean(false))),
    );
    state.freshness.inputs.insert(
        cem_ml::dependency_identity::DependencyInput::Data,
        version(2),
    );
    let empty = run(&mut cache, &artifact, &data, &state);
    assert!(!empty.hit);
    assert!(empty.plan.nodes.is_empty());
    assert!(run(&mut cache, &artifact, &data, &state).hit);
    let mut other = options;
    other.source_uri = Some("memory:other".into());
    assert!(!run(&mut cache, &compile_template(source, &other), &data, &state).hit);
    data.attribute_contracts
        .insert("title".into(), Default::default());
    assert!(!run(&mut cache, &artifact, &data, &state).hit);
}

#[test]
fn stale_foreign_and_reset_tickets_reject_before_admission() {
    use std::cell::Cell;
    let mut cache = PureRenderCache::default();
    let artifact = compile_template("{p | static}", &Default::default());
    let old = admission(1);
    let new = admission(2);
    let ticket = cache.begin(old.owner.clone());
    let calls = Cell::new(0);
    let result = cache.render(
        &artifact,
        &TemplateData::default(),
        &ticket,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
        || {
            calls.set(calls.get() + 1);
            Ok(new.clone())
        },
        || Some(new.clone()),
    );
    assert!(result.is_err());
    assert_eq!(calls.get(), 0);
    let mut foreign = PureRenderCache::default();
    assert!(foreign
        .render(
            &artifact,
            &TemplateData::default(),
            &ticket,
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            || {
                calls.set(calls.get() + 1);
                Ok(old.clone())
            },
            || Some(old.clone())
        )
        .is_err());
    run(&mut cache, &artifact, &TemplateData::default(), &old);
    cache.reset().unwrap();
    assert!(cache
        .render(
            &artifact,
            &TemplateData::default(),
            &ticket,
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            || {
                calls.set(calls.get() + 1);
                Ok(old.clone())
            },
            || Some(old.clone())
        )
        .is_err());
    assert_eq!(calls.get(), 0);
    assert_eq!(cache.stats().resident_entries, 0);
    assert!(!run(&mut cache, &artifact, &TemplateData::default(), &old).hit);
}

#[test]
fn admission_side_effects_run_on_hits_and_changed_snapshots_are_not_published() {
    use std::cell::Cell;
    let artifact = compile_template("{p | static}", &Default::default());
    let state = admission(1);
    let mut cache = PureRenderCache::default();
    let ticket = cache.begin(state.owner.clone());
    let events = Cell::new(0);
    let commands = Cell::new(0);
    let resources = Cell::new(0);
    for hit in [false, true] {
        let result = cache
            .render(
                &artifact,
                &TemplateData::default(),
                &ticket,
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
                || {
                    events.set(events.get() + 1);
                    commands.set(commands.get() + 1);
                    resources.set(resources.get() + 1);
                    Ok(state.clone())
                },
                || Some(state.clone()),
            )
            .unwrap();
        assert_eq!(result.hit, hit);
    }
    assert_eq!((events.get(), commands.get(), resources.get()), (2, 2, 2));
    let blocked = cache.render(
        &artifact,
        &TemplateData::default(),
        &ticket,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
        || {
            Err(cem_ml::content_cache::CacheError {
                code: "fixture.resource_failed",
                message: "resource admission failed".into(),
            })
        },
        || Some(state.clone()),
    );
    assert!(matches!(blocked, Err(error) if error.code == "fixture.resource_failed"));
    assert_eq!(cache.evaluations(), 1);
    let mut newer = state.clone();
    newer.freshness.environment = version(2);
    assert!(cache
        .render(
            &artifact,
            &TemplateData::default(),
            &ticket,
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            || Ok(newer),
            || Some(state.clone())
        )
        .is_err());
    let reads = Cell::new(0);
    let result = cache.render(
        &artifact,
        &TemplateData::default(),
        &ticket,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
        || Ok(state.clone()),
        || {
            reads.set(reads.get() + 1);
            (reads.get() < 3).then(|| state.clone())
        },
    );
    assert!(result.is_err());
    assert_eq!(cache.evaluations(), 1);
}

#[test]
fn cancellation_before_and_during_admission_never_returns_cached_output() {
    use cem_ml::scheduler::AbortSignal;
    use std::cell::Cell;
    let artifact = compile_template("{p | static}", &Default::default());
    let state = admission(1);
    let mut cache = PureRenderCache::default();
    run(&mut cache, &artifact, &TemplateData::default(), &state);
    let ticket = cache.begin(state.owner.clone());
    let calls = Cell::new(0);
    let aborted = AbortSignal::new();
    aborted.abort();
    assert!(cache
        .render(
            &artifact,
            &TemplateData::default(),
            &ticket,
            &OperationControl::new(aborted),
            ROOT_EXECUTION_SCOPE_ID,
            || {
                calls.set(1);
                Ok(state.clone())
            },
            || Some(state.clone())
        )
        .is_err());
    assert_eq!(calls.get(), 0);
    let abort = AbortSignal::new();
    let control = OperationControl::new(abort.clone());
    assert!(cache
        .render(
            &artifact,
            &TemplateData::default(),
            &ticket,
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            || {
                abort.abort();
                Ok(state.clone())
            },
            || Some(state.clone())
        )
        .is_err());
    assert_eq!(cache.evaluations(), 1);
}

#[test]
fn byte_count_capacity_refusal_and_lru_eviction_preserve_success() {
    let state = admission(1);
    let data = TemplateData::default();
    let first = compile_template("{p | first}", &Default::default());
    let second = compile_template("{p | second}", &Default::default());
    let mut cache = PureRenderCache::new(ContentCacheLimits {
        max_entries: 1,
        max_bytes: 1024 * 1024,
    })
    .unwrap();
    run(&mut cache, &first, &data, &state);
    run(&mut cache, &second, &data, &state);
    assert_eq!(cache.stats().resident_entries, 1);
    assert_eq!(cache.stats().evictions, 1);
    assert!(!run(&mut cache, &first, &data, &state).hit);
    assert_eq!(cache.evaluations(), 3);
    let mut cache = PureRenderCache::new(ContentCacheLimits {
        max_entries: 1,
        max_bytes: 256,
    })
    .unwrap();
    for _ in 0..2 {
        let result = run(&mut cache, &first, &data, &state);
        assert!(!result.hit);
        assert_eq!(render_plan_to_html(&result.plan), "<p>first</p>");
    }
    assert_eq!(cache.stats().resident_entries, 0);
}

#[test]
fn opaque_and_failed_queries_never_populate_cache_or_skip_callbacks() {
    use cem_ql::{
        eval::{AtomValue, Item, ItemStream},
        native::{NativeQueryFunction, NativeQueryRequest},
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    #[derive(Debug)]
    struct Counter(Arc<AtomicUsize>);
    impl NativeQueryFunction for Counter {
        fn call(&self, _: NativeQueryRequest<'_>) -> ItemStream {
            self.0.fetch_add(1, Ordering::SeqCst);
            ItemStream::once(Item::Atomic(AtomValue::Integer(7)))
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let mut data = TemplateData::default();
    data.native_functions
        .register("counter", 0, Counter(calls.clone()))
        .unwrap();
    let artifact = compile_template("{p | {$native:call(\"counter\")}}", &Default::default());
    let mut cache = PureRenderCache::default();
    let state = admission(1);
    for _ in 0..2 {
        let result = run(&mut cache, &artifact, &data, &state);
        assert!(!result.hit);
        assert_eq!(render_plan_to_html(&result.plan), "<p>7</p>");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(cache.stats().resident_entries, 0);
    let failed = compile_template(
        "{try | {$1 / 0}{catch @as=e | repaired}}",
        &Default::default(),
    );
    for _ in 0..2 {
        assert!(!run(&mut cache, &failed, &data, &state).hit);
    }
    let mut opaque = state;
    opaque.freshness = RenderFreshness::default();
    for _ in 0..2 {
        assert!(
            !run(
                &mut cache,
                &compile_template("{p | static}", &Default::default()),
                &data,
                &opaque
            )
            .hit
        );
    }
    assert_eq!(cache.stats().resident_entries, 0);
}

#[test]
fn hits_preserve_atomic_types_host_updates_and_source_provenance() {
    let artifact = compile_template(
        "{attribute @name=label @select='\"selected\"'}{input @value=42}",
        &cem_ql::render::CompileTemplateOptions {
            source_uri: Some("memory:origin".into()),
            ..Default::default()
        },
    );
    let mut cache = PureRenderCache::default();
    let state = admission(1);
    let data = TemplateData::default();
    let first = run(&mut cache, &artifact, &data, &state);
    let hit = run(&mut cache, &artifact, &data, &state);
    assert!(hit.hit);
    assert_eq!(first.plan.nodes, hit.plan.nodes);
    assert_eq!(
        first.plan.host_attribute_updates,
        hit.plan.host_attribute_updates
    );
    let first_map = cem_ql::render::render_plan_to_html_with_source_map(&first.plan);
    let hit_map = cem_ql::render::render_plan_to_html_with_source_map(&hit.plan);
    assert_eq!(
        format!("{:?}", first_map.output_spans),
        format!("{:?}", hit_map.output_spans)
    );
}

#[test]
fn native_attribute_views_are_never_retained_or_reconstructed_as_owned_atoms() {
    use cem_ql::eval::{AtomValue, Item, ItemStream, QueryItemView, QueryItemViewKind};
    use std::sync::Arc;
    #[derive(Debug)]
    struct View(Arc<()>);
    impl QueryItemView for View {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "test.native-atomic"
        }
        fn identity(&self) -> String {
            format!("{:p}", Arc::as_ptr(&self.0))
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Atomic
        }
        fn atom(&self) -> Option<AtomValue> {
            Some(AtomValue::String("native".into()))
        }
    }
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let data =
        TemplateData::default().with_binding("input", ItemStream::once(Item::native(View(owner))));
    let artifact = compile_template(
        "{p @title=\"{$input}\" | static}",
        &cem_ql::render::CompileTemplateOptions {
            host_bindings: vec!["input".into()],
            ..Default::default()
        },
    );
    let mut cache = PureRenderCache::default();
    for _ in 0..2 {
        let result = run(&mut cache, &artifact, &data, &admission(1));
        assert!(!result.hit);
        assert_eq!(
            render_plan_to_html(&result.plan),
            "<p title=\"native\">static</p>"
        );
    }
    assert_eq!(cache.stats().resident_entries, 0);
    drop(data);
    assert!(weak.upgrade().is_none());
}

#[test]
fn decoding_memory_pressure_falls_back_without_poisoning_the_operation() {
    let artifact = compile_template("{p | static}", &Default::default());
    let data = TemplateData::default();
    let state = admission(1);
    let mut cache = PureRenderCache::default();
    run(&mut cache, &artifact, &data, &state);
    let ticket = cache.begin(state.owner.clone());
    let control = OperationControl::default();
    let ceiling = control
        .scope_tree()
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .unwrap()
        .effective_policy
        .memory_bytes;
    let held = control
        .charge_memory(ROOT_EXECUTION_SCOPE_ID, ceiling - 1, None)
        .unwrap();
    let result = cache
        .render(
            &artifact,
            &data,
            &ticket,
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            || Ok(state.clone()),
            || Some(state.clone()),
        )
        .unwrap();
    assert!(!result.hit);
    assert_eq!(render_plan_to_html(&result.plan), "<p>static</p>");
    assert!(control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_ok());
    drop(held);
    assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    assert!(run(&mut cache, &artifact, &data, &state).hit);
}
