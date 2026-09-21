//! CEMT-COMPACT-TEMPLATES: direct and imported rendering share body validation.
use cem_ql::render::{
    compile_template_module_closure, render_compiled_template, render_plan_to_html,
    render_template, CompileTemplateOptions, TemplateData, TemplateModuleClosure,
    TemplateModuleSource,
};

fn imported(source: &str, base: &str) -> cem_ql::render::TemplateArtifact {
    let hash =
        |s: &str| cem_ml::content_cache::ContentHash::from_blake3(s.as_bytes()).header_value();
    compile_template_module_closure(
        source,
        &TemplateModuleClosure {
            root_uri: "memory:entry.cemt".into(),
            root_content_hash: hash(source),
            modules: vec![TemplateModuleSource {
                alias: "base".into(),
                parent_uri: None,
                uri: "memory:base.cemt".into(),
                content_hash: hash(base),
                source: base.into(),
            }],
            ..Default::default()
        },
        &CompileTemplateOptions::default(),
    )
}

#[test]
fn compact_imported_calls_matching_and_local_overrides_preserve_focus() {
    let base = r#"{module |
        {template @name=present @visibility=public | {param @name=subject}
            {apply-templates @select=subject @mode=cell}}
        {template @name=fallback @mode=cell @match=true | {span | {$dom:text()}}}
    }"#;
    let source = r#"{module | {import @as=base @src="./base.cemt"}
        {template @name=special @mode=cell @match='node == "ivy"' |
            {cem:variable @name=label @select=dom:text()}{b | {$label}}}
        {body | {call @from=base @template=present @with:subject='{("ivy", "saur")}'}}}"#;
    let artifact = imported(source, base);
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(""),
        "<b>ivy</b><span>saur</span>"
    );
}

#[test]
fn compact_imports_keep_private_entrypoints_private() {
    let artifact = imported("{module | {import @as=base @src='./base.cemt'} {body | {call @from=base @template=secret}}}", "{module | {template @name=secret | hidden}}");
    assert!(
        artifact
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.template.module_template_not_public"),
        "{:?}",
        artifact.diagnostics
    );
}

#[test]
fn mixed_or_duplicate_bodies_fail_before_output() {
    for body in [
        "{body | first}{b | second}",
        "text{body | wrapped}",
        "{body | first}{body | second}",
        "{body | wrapped}{$node}",
        "{body | wrapped}// output comment\n",
    ] {
        for wrapper in [false, true] {
            let source = format!("{{template @name=label | {body}}}{{call @template=label}}");
            let source = if wrapper {
                format!("{{module | {{template @name=label | {body}}}{{body | {{call @template=label}}}}}}")
            } else {
                source
            };
            let output = render_template(&source, &TemplateData::default());
            assert!(
                output
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "cem.transform_template.declaration_invalid"),
                "{source}: {:?}",
                output.diagnostics
            );
            assert!(output.rendered.is_empty());
        }
    }
}
