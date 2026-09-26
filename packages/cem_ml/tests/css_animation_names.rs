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
