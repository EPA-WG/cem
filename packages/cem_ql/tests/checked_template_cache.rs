use cem_ml::{
    content_cache::{ContentCacheLimits, ContentHash},
    dependency_identity::DependencyAdmission,
};
use cem_ql::{
    render::{
        render_compiled_template, CompileTemplateOptions, TemplateData, TemplateModuleClosure,
        TemplateModuleSource,
    },
    template_artifact::TemplateArtifactSourceMapMode as Mode,
    template_cache::TemplateCompilationCache,
};
fn admission() -> DependencyAdmission {
    DependencyAdmission {
        owner: "fixture".into(),
        runtime_generation: 1,
        revision: 1,
    }
}
fn closure(source: &str) -> TemplateModuleClosure {
    TemplateModuleClosure {
        root_uri: "studio://sample/root.cemt".into(),
        root_content_hash: ContentHash::from_blake3(source.as_bytes()).header_value(),
        resolver_policy_stamp: "resolver/1".into(),
        ..Default::default()
    }
}
#[test]
fn real_hits_skip_compilation_and_reload_independent_artifacts() {
    let mut cache = TemplateCompilationCache::default();
    let a = admission();
    let ticket = cache.begin(a.clone());
    let source = "{p | Hello}";
    let c = closure(source);
    let options = CompileTemplateOptions::default();
    let first = cache
        .compile(source, Some(&c), &options, Mode::Dev, &ticket, Some(&a))
        .unwrap();
    let mut hit = cache
        .compile(source, Some(&c), &options, Mode::Dev, &ticket, Some(&a))
        .unwrap();
    assert!(!first.hit);
    assert!(hit.hit);
    assert_eq!(cache.compilations(), 1);
    assert_eq!(
        serde_json::to_value(&first.artifact.nodes).unwrap(),
        serde_json::to_value(&hit.artifact.nodes).unwrap()
    );
    assert_eq!(
        render_compiled_template(&first.artifact, &TemplateData::default()).nodes,
        render_compiled_template(&hit.artifact, &TemplateData::default()).nodes
    );
    hit.artifact.nodes.clear();
    assert!(!first.artifact.nodes.is_empty());
}
#[test]
fn tampered_bytes_contract_and_versions_never_hit_previously_valid_entry() {
    let source = "{p | Hello}";
    let options = CompileTemplateOptions::default();
    let a = admission();
    let mut cache = TemplateCompilationCache::default();
    let ticket = cache.begin(a.clone());
    let c = closure(source);
    cache
        .compile(source, Some(&c), &options, Mode::Dev, &ticket, Some(&a))
        .unwrap();
    for case in 0..4 {
        let mut invalid = c.clone();
        match case {
            0 => invalid.root_content_hash = ContentHash::from_blake3(b"wrong").header_value(),
            1 => invalid.parameter_contract = vec!["wrong".into()],
            2 => invalid.cem_ql_version = "wrong".into(),
            _ => invalid.cem_ml_version = "wrong".into(),
        };
        let result = cache
            .compile(
                source,
                Some(&invalid),
                &options,
                Mode::Dev,
                &ticket,
                Some(&a),
            )
            .unwrap();
        assert!(!result.hit);
        assert!(result
            .artifact
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()));
    }
    assert_eq!(cache.compilations(), 5);
}
#[test]
fn imports_rehash_actual_bytes_before_lookup() {
    let source =
        "{module | {import @as=docs @src=docs.cemt} {body | {call @from=docs @template=label}}}";
    let partial = "{module | {template @name=label @visibility=public | {body | {p | Hello}}}}";
    let mut c = closure(source);
    c.modules.push(TemplateModuleSource {
        alias: "docs".into(),
        parent_uri: None,
        uri: "studio://sample/docs.cemt".into(),
        source: partial.into(),
        content_hash: ContentHash::from_blake3(partial.as_bytes()).header_value(),
    });
    let a = admission();
    let mut cache = TemplateCompilationCache::default();
    let ticket = cache.begin(a.clone());
    let o = CompileTemplateOptions::default();
    assert!(
        !cache
            .compile(source, Some(&c), &o, Mode::Dev, &ticket, Some(&a))
            .unwrap()
            .hit
    );
    assert!(
        cache
            .compile(source, Some(&c), &o, Mode::Dev, &ticket, Some(&a))
            .unwrap()
            .hit
    );
    c.modules[0].source = partial.replace("Hello", "Edited");
    let result = cache
        .compile(source, Some(&c), &o, Mode::Dev, &ticket, Some(&a))
        .unwrap();
    assert!(!result.hit);
    assert!(result
        .artifact
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation()));
    c.modules[0].content_hash =
        ContentHash::from_blake3(c.modules[0].source.as_bytes()).header_value();
    assert!(
        !cache
            .compile(source, Some(&c), &o, Mode::Dev, &ticket, Some(&a))
            .unwrap()
            .hit
    );
}
#[test]
fn freshness_uri_options_and_mode_invalidate_and_opaque_context_bypasses() {
    let source = "{p | Hello}";
    let a = admission();
    let mut cache = TemplateCompilationCache::default();
    let ticket = cache.begin(a.clone());
    let base = closure(source);
    let o = CompileTemplateOptions::default();
    cache
        .compile(source, Some(&base), &o, Mode::Dev, &ticket, Some(&a))
        .unwrap();
    for case in 0..5 {
        let mut c = base.clone();
        let mut options = o.clone();
        let mut mode = Mode::Dev;
        match case {
            0 => c.resolver_policy_stamp = "resolver/2".into(),
            1 => c.root_uri = "studio://other/root.cemt".into(),
            2 => options.host_bindings = vec!["extra".into()],
            3 => mode = Mode::Prod,
            _ => options.skip_cemt_function_bodies = true,
        };
        assert!(
            !cache
                .compile(source, Some(&c), &options, mode, &ticket, Some(&a))
                .unwrap()
                .hit
        );
        assert!(
            cache
                .compile(source, Some(&c), &options, mode, &ticket, Some(&a))
                .unwrap()
                .hit
        );
    }
    let mut opaque = base;
    opaque.resolver_policy_stamp.clear();
    for _ in 0..2 {
        assert!(
            !cache
                .compile(source, Some(&opaque), &o, Mode::Dev, &ticket, Some(&a))
                .unwrap()
                .hit
        );
    }
}
#[test]
fn cancelled_superseded_and_replaced_requests_do_no_work_or_publication() {
    let source = "{p | Hello}";
    let a = admission();
    let mut cache = TemplateCompilationCache::default();
    let ticket = cache.begin(a.clone());
    let o = CompileTemplateOptions::default();
    assert!(cache
        .compile(source, None, &o, Mode::Dev, &ticket, None)
        .is_err());
    let mut newer = a.clone();
    newer.revision += 1;
    assert!(cache
        .compile(source, None, &o, Mode::Dev, &ticket, Some(&newer))
        .is_err());
    cache.reset().unwrap();
    assert!(cache
        .compile(source, None, &o, Mode::Dev, &ticket, Some(&a))
        .is_err());
    let mut other = TemplateCompilationCache::default();
    assert!(other
        .compile(source, None, &o, Mode::Dev, &ticket, Some(&a))
        .is_err());
    assert_eq!(cache.compilations(), 0);
    assert_eq!(other.compilations(), 0);
}
#[test]
fn standalone_modes_preserve_uncached_plans_and_source_frames_under_eviction() {
    let source = "{p | Hello}";
    let a = admission();
    let mut cache = TemplateCompilationCache::new(ContentCacheLimits {
        max_entries: 1,
        max_bytes: 1024 * 1024,
    })
    .unwrap();
    let ticket = cache.begin(a.clone());
    let o = CompileTemplateOptions {
        source_uri: Some("studio://source.cemt".into()),
        ..Default::default()
    };
    for mode in [Mode::Dev, Mode::Prod] {
        let first = cache
            .compile(source, None, &o, mode, &ticket, Some(&a))
            .unwrap();
        let hit = cache
            .compile(source, None, &o, mode, &ticket, Some(&a))
            .unwrap();
        assert!(hit.hit);
        assert_eq!(
            serde_json::to_value(&first.artifact.nodes).unwrap(),
            serde_json::to_value(&hit.artifact.nodes).unwrap()
        );
        assert_eq!(first.artifact.diagnostics, hit.artifact.diagnostics);
        let p = render_compiled_template(&first.artifact, &TemplateData::default());
        let q = render_compiled_template(&hit.artifact, &TemplateData::default());
        assert_eq!(p.nodes, q.nodes);
    }
    assert_eq!(cache.stats().resident_entries, 1);
    assert_eq!(cache.stats().evictions, 1);
    assert_eq!(cache.stats().live_leases, 0);
}

#[test]
fn cache_capacity_refusal_preserves_successful_compilation_without_retaining_payloads() {
    let mut cache = TemplateCompilationCache::new(ContentCacheLimits {
        max_entries: 1,
        max_bytes: 1,
    })
    .unwrap();
    let a = admission();
    let ticket = cache.begin(a.clone());
    for _ in 0..2 {
        let result = cache
            .compile(
                "{p | Content}",
                None,
                &CompileTemplateOptions::default(),
                Mode::Dev,
                &ticket,
                Some(&a),
            )
            .unwrap();
        assert!(!result.hit);
        assert!(result.artifact.diagnostics.is_empty());
        assert!(!result.artifact.nodes.is_empty());
    }
    assert_eq!(cache.compilations(), 2);
    assert_eq!(cache.stats().retained_bytes, 0);
    assert_eq!(cache.stats().resident_entries, 0);
}
