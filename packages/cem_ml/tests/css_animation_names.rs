use std::sync::Arc;

use cem_ml::{
    css_emission::emit_css_animation_names,
    css_resources::{resolve_css_resources, CssResourcePlan},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
};

fn plan(css: &str) -> CssResourcePlan {
    let tree = import_data(css, "text/css", "cem", "urn:css:owner").unwrap();
    let handle = CemResolutionContextHandle::new("fixture");
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: "fixture".into(),
            resolver_identity: "fixture".into(),
            resource_policy_stamp: "policy".into(),
            frames: vec![CemModuleUrlFrame::new(
                "page",
                "https://example.test/page.html",
            )],
        },
    );
    resolve_css_resources(
        tree,
        &CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle),
        "https://example.test/styles/main.css",
    )
    .unwrap()
}
fn first_rule(plan: &CssResourcePlan) -> u32 {
    let root = plan.tree.node(0).unwrap().children[0];
    plan.tree.node(root).unwrap().children[0]
}

fn declaration(plan: &CssResourcePlan) -> u32 {
    plan.tree
        .node(first_rule(plan))
        .unwrap()
        .children
        .iter()
        .copied()
        .find(|id| {
            plan.tree
                .node(*id)
                .unwrap()
                .name
                .as_ref()
                .is_some_and(|n| n.local_name == "declaration")
        })
        .unwrap()
}

#[test]
fn static_animation_names_rewrite_decoded_symbols_and_preserve_other_tokens() {
    let source = r#".card {animation-name: none, p\75 lse, "pulse", external /* keep */;}"#;
    let plan = plan(source);
    let names = std::collections::BTreeMap::from([("pulse".into(), "pulse-owner".into())]);
    let result = emit_css_animation_names(&plan.tree, declaration(&plan), &names).unwrap();
    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result.value.as_deref(),
        Some(r#"none, "pulse-owner", "pulse-owner", external /* keep */"#)
    );
    assert_eq!(result.names.len(), 4);
    assert!(result
        .names
        .iter()
        .all(|n| n.source.origin().is_some() && n.range.length > 0));
    let original_range = result.names[1].range;
    assert_eq!(
        &source[original_range.offset as usize
            ..(original_range.offset + original_range.length) as usize],
        r#"p\75 lse"#
    );
    let other = std::collections::BTreeMap::from([("pulse".into(), "other-owner".into())]);
    assert!(
        emit_css_animation_names(&plan.tree, declaration(&plan), &other)
            .unwrap()
            .value
            .unwrap()
            .contains("other-owner")
    );
}

#[test]
fn animation_name_keywords_and_string_names_have_distinct_identity() {
    let names = std::collections::BTreeMap::from([
        ("none".into(), "none-owner".into()),
        ("inherit".into(), "inherit-owner".into()),
    ]);
    for (value, expected) in [
        ("NONE, \"none\"", "NONE, \"none-owner\""),
        ("inherit", "inherit"),
        ("\"inherit\"", "\"inherit-owner\""),
    ] {
        let plan = plan(&format!(".card {{-WEBKIT-animation-name:{value};}}"));
        assert_eq!(
            emit_css_animation_names(&plan.tree, declaration(&plan), &names)
                .unwrap()
                .value
                .as_deref(),
            Some(expected)
        );
    }
    let plan = plan(".card {width:1px}");
    assert!(emit_css_animation_names(&plan.tree, declaration(&plan), &names).is_err());
}

#[test]
fn animation_name_fragment_diagnoses_dynamic_and_invalid_profiles() {
    let names = std::collections::BTreeMap::new();
    for value in [
        "var(--motion)",
        "pulse, var(--motion)",
        "attr(data-motion)",
        "env(motion)",
    ] {
        let plan = plan(&format!(".card {{animation-name:{value};}}"));
        let result = emit_css_animation_names(&plan.tree, declaration(&plan), &names).unwrap();
        assert!(result.value.is_none() && result.names.is_empty());
        assert_eq!(
            result.diagnostics[0].code,
            "cem.scoped_css.animation_name_dynamic_unsupported"
        );
    }
    for value in [
        "",
        "pulse,",
        ",pulse",
        "pulse other",
        "inherit, pulse",
        "default",
        "future()",
    ] {
        let plan = plan(&format!(".card {{animation-name:{value};}}"));
        let result = emit_css_animation_names(&plan.tree, declaration(&plan), &names).unwrap();
        assert!(result.value.is_none() && result.names.is_empty());
        assert_eq!(
            result.diagnostics[0].code, "cem.scoped_css.animation_name_unsupported",
            "{value}"
        );
    }
}

#[test]
fn animation_name_fragments_require_import_owned_slots() {
    let tree = import_data(
        "<declaration xmlns='https://cem.dev/ns/data/css/1' name='animation-name' value='pulse'/>",
        "application/xml",
        "cem",
        "legacy.xml",
    )
    .unwrap();
    let declaration = tree.node(0).unwrap().children[0];
    let error = emit_css_animation_names(&tree, declaration, &Default::default()).unwrap_err();
    assert_eq!(error.code, "cem.scoped_css.animation_name_tree_invalid");
}

#[test]
fn shorthand_classifies_name_after_non_name_keyword_slots() {
    let names = [
        "linear",
        "backwards",
        "infinite",
        "reverse",
        "paused",
        "pulse",
        "none",
    ]
    .map(|name| (name.to_owned(), format!("{name}-owner")))
    .into_iter()
    .collect();
    for (value, expected) in [
        ("1s linear linear", "1s linear \"linear-owner\""),
        ("3s none backwards", "3s none \"backwards-owner\""),
        ("1s infinite infinite", "1s infinite \"infinite-owner\""),
        ("1s reverse reverse", "1s reverse \"reverse-owner\""),
        ("1s paused paused", "1s paused \"paused-owner\""),
        ("1s none none", "1s none none"),
        ("1s linear", "1s linear"),
        (r"pulse 1\73", r#""pulse-owner" 1\73"#),
        ("pulse 1e2ms", "\"pulse-owner\" 1e2ms"),
        ("inherit", "inherit"),
        ("1s \"none\"", "1s \"none-owner\""),
        (
            "p\\75 lse 2s ease -10ms 2 alternate both paused /*keep*/, external 1s",
            "\"pulse-owner\" 2s ease -10ms 2 alternate both paused /*keep*/, external 1s",
        ),
        (
            "pulse 1s cubic-bezier(0, .2, 1, 1), pulse 2s steps(2, end)",
            "\"pulse-owner\" 1s cubic-bezier(0, .2, 1, 1), \"pulse-owner\" 2s steps(2, end)",
        ),
    ] {
        let plan = plan(&format!(".card {{animation:{value};}}"));
        let result = emit_css_animation_names(&plan.tree, declaration(&plan), &names).unwrap();
        assert!(
            result.diagnostics.is_empty(),
            "{value}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.value.as_deref(), Some(expected), "{value}");
        assert!(result
            .names
            .iter()
            .all(|n| n.range.length > 0 && n.source.origin().is_some()));
    }
}

#[test]
fn shorthand_rejects_unsupported_or_dynamic_groups_atomically() {
    for value in [
        "",
        "pulse,",
        "pulse other",
        "-1s pulse",
        "-1e-50s pulse",
        "1s 2s 3s pulse",
        "1s -2 pulse",
        "inherit 1s",
        "1s default",
        "1px pulse",
        "pulse 1s future()",
        "pulse 1s steps(0)",
        "pulse 1s cubic-bezier(2,0,1,1)",
        "pulse 1s, other 1s 2s 3s",
    ] {
        let plan = plan(&format!(".card {{animation:{value};}}"));
        let result =
            emit_css_animation_names(&plan.tree, declaration(&plan), &Default::default()).unwrap();
        assert!(
            result.value.is_none() && !result.diagnostics.is_empty(),
            "{value}"
        );
    }
    for value in [
        "pulse var(--duration) linear",
        "var(--animation)",
        "pulse 1s steps(var(--count))",
    ] {
        let plan = plan(&format!(".card {{animation:{value};}}"));
        let result =
            emit_css_animation_names(&plan.tree, declaration(&plan), &Default::default()).unwrap();
        assert!(result.value.is_none(), "{value}");
        assert_eq!(
            result.diagnostics[0].code,
            "cem.scoped_css.animation_name_dynamic_unsupported"
        );
    }
}
