use cem_ql::render::{render_template, TemplateData};

#[test]
fn match_rules_have_modes_explicit_priority_and_late_local_override() {
    let rendered = render_template(
        r#"
        {template @name=fallback @mode=summary @match=true @priority=-10 |
            {param @name=node}{body | {span | fallback}}}
        {template @name=older @mode=summary @match='node == "🍒"' @priority=5 |
            {param @name=node}{body | {b | older}}}
        {template @name=newer @mode=summary @match='node == "🍒"' @priority=5 |
            {param @name=node}{body | {strong | {$node}}}}
        {template @name=other @mode=detail @match=true @priority=100 |
            {param @name=node}{body | should not run}}
        {section | {apply-templates @select='("🍒", "🍋")' @mode=summary}}
    "#,
        &TemplateData::default(),
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(
        rendered.rendered.trim(),
        "<section><strong>🍒</strong><span>fallback</span></section>"
    );
}

#[test]
fn recursive_match_dispatch_is_bounded_and_unmatched_values_are_inert() {
    let recursive = render_template(
        r#"
        {template @name=again @match=true | {param @name=node}
            {body | {apply-templates @select=node}}}
        {apply-templates @select='"loop"'}
    "#,
        &TemplateData::default(),
    );
    assert!(recursive
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.transform_template.recursion_limit"));
    let unmatched = render_template(
        r#"{apply-templates @select='"<img src=x onerror=bad()>"'}"#,
        &TemplateData::default(),
    );
    assert!(
        unmatched.diagnostics.is_empty(),
        "{:?}",
        unmatched.diagnostics
    );
    assert_eq!(unmatched.rendered.trim(), "");
}

#[test]
fn imported_base_rules_remain_extensible_without_rewriting_the_base_template() {
    use cem_ql::render::{
        compile_template_module_closure, render_compiled_template, render_plan_to_html,
        CompileTemplateOptions, TemplateModuleClosure, TemplateModuleSource,
    };
    let base = r#"{module |
        {template @name=present @visibility=public | {param @name=subject}
            {body | {apply-templates @select=subject @mode=card}}}
        {template @name=default @mode=card @match=true | {param @name=node}
            {body | {span | base: {$node}}}}
    }"#;
    let extension = r#"{module |
        {import @as=base @src="./base.cemt"}
        {template @name=special @mode=card @match='node == "🍒"' | {param @name=node}
            {body | {strong | override: {$node}}}}
        {body | {call @from=base @template=present @with:subject='{("🍒", "🍋")}'}}
    }"#;
    let hash =
        |s: &str| cem_ml::content_cache::ContentHash::from_blake3(s.as_bytes()).header_value();
    let artifact = compile_template_module_closure(
        extension,
        &TemplateModuleClosure {
            root_uri: "https://example.test/extension.cemt".into(),
            root_content_hash: hash(extension),
            modules: vec![TemplateModuleSource {
                alias: "base".into(),
                parent_uri: None,
                uri: "https://example.test/base.cemt".into(),
                content_hash: hash(base),
                source: base.into(),
            }],
            ..TemplateModuleClosure::default()
        },
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan).trim(),
        "<strong>override: 🍒</strong><span>base: 🍋</span>"
    );
}

#[test]
fn invalid_match_metadata_is_diagnosed_and_empty_parameters_do_not_inherit() {
    for source in [
        "{template @match}",
        "{template @match=true @priority=banana}",
        "{template @match=true @priority=1.5}",
        "{template @match=true @mode='{node}'}",
        "{apply-templates}",
    ] {
        let result = render_template(source, &TemplateData::default());
        assert!(!result.diagnostics.is_empty(), "{source}");
    }
    let result = render_template(
        r#"
        {template @name=outer | {param @name=label}
            {body | {apply-templates @select='"subject"' @with:label='{()}'}}}
        {template @match=true | {param @name=label}{body | {span | {$label}}}}
        {call @template=outer @with:label=must-not-leak}
    "#,
        &TemplateData::default(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.rendered.trim(), "<span></span>");
}
