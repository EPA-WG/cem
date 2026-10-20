use cem_ql::{
    render::{compile_template, render_plan_to_html},
    render_cache::retained::{RetainedJsonRenderCache, RetainedJsonRenderContext},
};
#[test]
fn retained_inputs_use_actual_values_and_native_owner_not_browser_hashes() {
    let artifact = compile_template(
        "{p | {$value}}",
        &cem_ql::render::CompileTemplateOptions {
            host_bindings: vec!["value".into()],
            ..Default::default()
        },
    );
    let context = RetainedJsonRenderContext {
        owner: "engine/instance".into(),
        scope_policy_stamp: "scope/1".into(),
    };
    let mut cache = RetainedJsonRenderCache::default();
    let first = cache
        .render(
            &artifact,
            "{\"value\":\"Ada\"}",
            &context,
            &Default::default(),
        )
        .unwrap();
    let hit = cache
        .render(
            &artifact,
            "{\"value\":\"Ada\"}",
            &context,
            &Default::default(),
        )
        .unwrap();
    assert!(!first.hit);
    assert!(hit.hit);
    let changed = cache
        .render(
            &artifact,
            "{\"value\":\"Lin\"}",
            &context,
            &Default::default(),
        )
        .unwrap();
    assert!(!changed.hit);
    assert_eq!(render_plan_to_html(&changed.plan), "<p>Lin</p>");
    assert_eq!(cache.evaluations(), 2);
    assert_eq!(cache.admissions(), 3);
}

#[test]
fn owner_scope_and_runtime_reset_do_not_adopt_another_admission() {
    let artifact = compile_template("{p | static}", &Default::default());
    let mut context = RetainedJsonRenderContext {
        owner: "engine/instance".into(),
        scope_policy_stamp: "scope/1".into(),
    };
    let mut cache = RetainedJsonRenderCache::default();
    assert!(
        !cache
            .render(&artifact, "{}", &context, &Default::default())
            .unwrap()
            .hit
    );
    context.owner = "another/instance".into();
    assert!(
        !cache
            .render(&artifact, "{}", &context, &Default::default())
            .unwrap()
            .hit
    );
    context.scope_policy_stamp = "scope/2".into();
    assert!(
        !cache
            .render(&artifact, "{}", &context, &Default::default())
            .unwrap()
            .hit
    );
    assert!(
        cache
            .render(&artifact, "{}", &context, &Default::default())
            .unwrap()
            .hit
    );
    let generation = cache.stats().runtime_generation;
    cache.reset().unwrap();
    assert_eq!(cache.stats().runtime_generation, generation + 1);
    assert_eq!(cache.stats().resident_entries, 0);
    assert!(
        !cache
            .render(&artifact, "{}", &context, &Default::default())
            .unwrap()
            .hit
    );
}
#[test]
fn json_resource_and_node_handles_remain_opaque_even_when_unused() {
    let artifact = compile_template("{p | static}", &Default::default());
    let context = RetainedJsonRenderContext {
        owner: "engine/instance".into(),
        scope_policy_stamp: "scope/1".into(),
    };
    let mut cache = RetainedJsonRenderCache::default();
    for json in [
        r#"{"input":{"$node":"node-owner"}}"#,
        r#"{"input":{"$resource":{"id":"resource","contentType":"application/xml"}}}"#,
    ] {
        for _ in 0..2 {
            assert!(
                !cache
                    .render(&artifact, json, &context, &Default::default())
                    .unwrap()
                    .hit
            );
        }
    }
    assert_eq!(cache.evaluations(), 4);
    assert_eq!(cache.admissions(), 4);
    assert_eq!(cache.stats().resident_entries, 0);
    assert!(cache
        .render(&artifact, "invalid json", &context, &Default::default())
        .is_err());
}
