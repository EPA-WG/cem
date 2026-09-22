//! Shared expression binding selection must preserve the outer CEMT environment.
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
    template_artifact::{
        compile_template_artifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode,
    },
};
use std::collections::BTreeMap;

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: impl IntoIterator<Item = (&'static str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), vec![value]))
            .collect(),
    )
}

fn data() -> TemplateData {
    TemplateData::default()
        .with_binding(
            "island",
            ItemStream::once(record([("label", text("outer")), ("extra", text("kept"))])),
        )
        .with_binding(
            "datadom",
            ItemStream::once(record([("island", record([("label", text("nested"))]))])),
        )
        .with_binding(
            "unused",
            ItemStream::once(record([("large", text(&"unread".repeat(1000)))])),
        )
}

#[test]
fn template_dispatch_sees_outer_bindings_and_keeps_both_island_paths_after_reload() {
    let source = r#"
        {attribute @name=label @select='island.label ?? "default"'}
        {template @mode=label @match=true | {b | {$label}:{$dom:text()}:{$datadom.island.label}}}
        {$cemt:apply_templates(("one", "two"), "label")}
        {p | {$str:concat(record:entries(island).key, ",")}}
        {span | {$label}:{$island.label}}
    "#
    .lines()
    .map(str::trim)
    .collect::<String>();
    let options = CompileTemplateOptions {
        host_bindings: vec!["island".into(), "unused".into()],
        ..Default::default()
    };
    let original = compile_template(&source, &options);
    let portable = compile_template_artifact(&source, &options, TemplateArtifactSourceMapMode::Dev);
    let reloaded = portable
        .reload(&TemplateArtifactLoadContext {
            host_bindings: options.host_bindings.clone(),
            expected_source_hash: None,
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    for artifact in [&original, &reloaded] {
        let output = render_compiled_template(artifact, &data());
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        assert_eq!(render_plan_to_html(&output), "<b>outer:one:nested</b><b>outer:two:nested</b><p>extra,label</p><span>outer:outer</span>");
    }
}

#[test]
fn selected_expressions_preserve_scope_shadowing_and_recovery() {
    let source = r#"
        {variable @name=label @select=island.label}
        {div | {variable @name=label @select='"inner"'}{p | {$label}}}
        {try | {variable @name=label @select='"discarded"'}
            {$report:raise("sample.failure", "recover")}
            {catch @as=e | {b | {$label}:{$e.code}}}}
        {span | {$label}:{$datadom.island.label}}
    "#
    .lines()
    .map(str::trim)
    .collect::<String>();
    let artifact = compile_template(
        &source,
        &CompileTemplateOptions {
            host_bindings: vec!["island".into(), "unused".into()],
            ..Default::default()
        },
    );
    let input = data();
    let before: BTreeMap<_, _> = input.bindings.clone();
    let output = render_compiled_template(&artifact, &input);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(
        render_plan_to_html(&output),
        "<div><p>inner</p></div><b>outer:sample.failure</b><span>outer:nested</span>"
    );
    assert_eq!(
        input.bindings, before,
        "evaluation must not narrow or mutate the caller's data"
    );
}
