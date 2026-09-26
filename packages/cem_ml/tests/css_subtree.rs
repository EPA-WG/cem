use std::sync::Arc;

use cem_ml::{
    css_emission::{emit_css_rule_subtree, CssRuleMode},
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

#[test]
fn subtree_preserves_native_nesting_group_context_and_declaration_runs() {
    let plan = plan(".card,.label::before { color:red; @media all { color:blue; &.active { color:orange; } color:green; } color:purple; }");
    let result = emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.css(), ".card, .label::before {color:red;@media all {color:blue;&.active {color:orange;}color:green;}color:purple;}");
    assert!(result
        .fragments
        .iter()
        .all(|f| f.source.origin().is_some() && f.range.length > 0));
}

#[test]
fn subtree_suppresses_invalid_branches_and_reports_ancestors_once() {
    let plan =
        plan("#bad,.card { &.active { color:red; } &.card {color:pink} .label { color:green; } }");
    let result = emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert_eq!(
        result.css(),
        ".card {&.active {color:red;}& .label {color:green;}}"
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "cem.scoped_css.id_selector_unsupported")
            .count(),
        1
    );
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scoped_css.manufactured_specificity_unsupported"));
    assert!(result
        .diagnostics
        .iter()
        .all(|d| d.source.origin().is_some()));
}

#[test]
fn subtree_carries_instance_prefix_through_groups_and_resolves_resources() {
    let plan = plan(
        "@media all { .card { @supports (display:grid) { .label {background:url(icon.svg)} } } }",
    );
    let result = emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Instance).unwrap();
    assert!(result.diagnostics.is_empty());
    assert_eq!(result.css(), "@media all {:scope .card {@supports (display:grid) {& .label {background:url(\"https://example.test/styles/icon.svg\");}}}}");
}

#[test]
fn subtree_omits_empty_groups_and_diagnoses_unsupported_constructs() {
    for css in [
        ".card {@supports display:grid {color:red}}",
        ".card {&.card {color:red}}",
        "@media all {#bad {color:red}}",
    ] {
        let plan = plan(css);
        let result =
            emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
        assert_eq!(result.css(), "", "{css}");
        assert!(!result.diagnostics.is_empty());
    }
    let plan = plan(".card {color:red; @layer local {color:blue} color:green;}");
    let result = emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert_eq!(result.css(), ".card {color:red;color:green;}");
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.subtree_construct_unsupported"
    );
}

#[test]
fn subtree_bounds_traversal_and_rejects_detached_nested_entrypoints() {
    let source = format!(
        "{} .card {{color:red}} {}",
        "@media all {".repeat(65),
        "}".repeat(65)
    );
    // The shared importer rejects excessive depth before a subtree can exist.
    assert!(import_data(&source, "text/css", "cem", "deep.css")
        .unwrap_err()
        .contains("64-level"));
    let plan = self::plan(".card {.label {color:red}}");
    let nested = plan
        .tree
        .node(first_rule(&plan))
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
                .is_some_and(|n| n.local_name == "rule")
        })
        .unwrap();
    assert!(emit_css_rule_subtree(&plan, nested, CssRuleMode::Declaration).is_err());
}

/// Explicit file boundary for the Chromium fixture; never a runtime AST handoff.
#[test]
fn browser_fixture_emits_scoped_native_nesting() {
    use cem_ml::css_emission::{emit_css_scope_wrapper, CssManagedScope};
    let cases = [
        ("global-alias", ":global(cem-fixture.active) {color:purple; :where(&) {background-color:orange}}", CssRuleMode::Declaration),
        ("global-instance", ":global(.active) {background-color:orange}", CssRuleMode::Instance),
        ("container", ":host {display:block; container:panel / inline-size; width:400px;} .card {color:black; @container panel (width > 300px) {color:orange; &.active {background-color:pink}}}", CssRuleMode::Declaration),
        ("container-style", ":host {--theme:dark;} .card {color:black; @container style(--theme: dark) {color:orange}}", CssRuleMode::Declaration),
        ("starting-style", ".card {opacity:1; transition:opacity 1s linear; @starting-style {opacity:0;}}", CssRuleMode::Declaration),
        ("parent-list", ".card,.strong.extra { & span {color:orange} } .card span {color:black}", CssRuleMode::Declaration),
        ("declarations", ".pseudo::before {content:\"marker\";color:red; @media all {color:blue;} color:green;}", CssRuleMode::Declaration),
        ("group-order", ".card {color:red; @media all {@supports (display:grid) {color:blue; &.active {background-color:orange;} color:green;}} color:purple;}", CssRuleMode::Declaration),
        ("rejected-parent", "#blocked,.card { &.active {color:red} } .card.active {color:green}", CssRuleMode::Declaration),
        ("instance", ".card {@media all {.label {color:orange}}}", CssRuleMode::Instance),
        ("host", ":host {color:purple; &.active {background-color:orange}}", CssRuleMode::Declaration),
        ("duplicates", ".card {color:green; &.card {color:red} && {color:red}}", CssRuleMode::Declaration),
        ("zero-weight", ".card.active {:where(&) {.label {color:orange}}}", CssRuleMode::Declaration),
    ];
    let output = std::env::var_os("CEM_CSS_SUBTREE_FIXTURE_DIR").map(std::path::PathBuf::from);
    if let Some(dir) = &output {
        std::fs::create_dir_all(dir).unwrap();
    }
    for (name, css, mode) in cases {
        let plan = plan(css);
        let scope = match mode {
            CssRuleMode::Declaration => CssManagedScope::Private {
                tag: "cem-fixture".into(),
                context: None,
            },
            CssRuleMode::Instance => CssManagedScope::Instance,
        };
        let wrapper = emit_css_scope_wrapper(&scope).unwrap();
        let mut text = wrapper.opening;
        let root = plan.tree.node(0).unwrap().children[0];
        for &rule in &plan.tree.node(root).unwrap().children {
            let result = emit_css_rule_subtree(&plan, rule, mode).unwrap();
            assert!(!result.fragments.is_empty(), "{name}");
            text.push_str(&result.css());
        }
        text.push_str(wrapper.closing);
        if let Some(dir) = &output {
            std::fs::write(dir.join(format!("{name}.css")), text).unwrap();
        }
    }
    // References precede their definitions in one retained sheet.
    for (file, source, authored) in [
        ("keyframes", "@keyframes pulse {from {opacity:0} to {opacity:1}}", "pulse"),
        ("keyframes-empty", "@keyframes pulse {}", "pulse"),
        ("keyframes-string", "@keyframes \"quoted name\" {from {opacity:0} to {opacity:1}}", "\"quoted name\""),
        ("keyframes-shorthand", "@keyframes linear {from {opacity:0} to {opacity:1}}", "1s linear linear paused"),
        ("keyframes-conditional", "@keyframes pulse {from {opacity:0} to {opacity:1}} @media all {@keyframes pulse {from {opacity:.2} to {opacity:.6}}}", "pulse"),
    ] {
        let shorthand = file == "keyframes-shorthand";
        let property = if shorthand { "animation" } else { "animation-name" };
        let timing = if shorthand { "" } else {
            ";animation-duration:1s;animation-timing-function:linear;animation-play-state:paused"
        };
        let sheet_plan = plan(&format!(".card {{{property}:{authored}{timing}}} {source}"));
        let sheet = cem_ml::css_emission::emit_css_stylesheet(&sheet_plan, CssRuleMode::Declaration, "fixture").unwrap();
        assert!(sheet.body.diagnostics.is_empty());
        let scope = emit_css_scope_wrapper(&CssManagedScope::Private {
            tag: "cem-fixture".into(),
            context: None,
        })
        .unwrap();
        let css = format!(
            "{}{}{}",
            scope.opening,
            sheet.body.css(),
            scope.closing
        );
        if let Some(dir) = &output {
            std::fs::write(dir.join(format!("{file}.css")), css).unwrap();
        }
    }
}

#[test]
fn starting_style_composition_preserves_nesting_and_rejects_invalid_groups() {
    let plan = plan(".card {@starting-style {opacity:0; &.active {color:red} opacity:0.5;} @starting-style invalid {opacity:0.9} opacity:1;}");
    let result = emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert_eq!(
        result.css(),
        ".card {@starting-style {opacity:0;&.active {color:red;}opacity:0.5;}opacity:1;}"
    );
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.starting_style_prelude_invalid"
    );
}

#[test]
fn container_composition_carries_parent_specificity_and_declaration_order() {
    let plan = plan(".card {@container panel (width > 300px) {color:red; &.active {color:orange} color:green;} color:purple;}");
    let result = emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert!(result.diagnostics.is_empty());
    assert_eq!(result.css(), ".card {@container panel (width > 300px) {color:red;&.active {color:orange;}color:green;}color:purple;}");
    let plan = self::plan(".card.active {@container (width > 300px) {&.label {color:red}}}");
    let result = emit_css_rule_subtree(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert!(result.css().is_empty());
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.specificity_unsupported"
    );
}

#[test]
fn subtree_rewrites_animation_symbols_across_nested_declaration_runs() {
    let plan = plan(
        r#"@media all {.card {animation:pulse 1s linear; background:url(icon.svg); @supports(display:grid) {animation-name:pulse, external; &.active {animation-name:"pulse"} animation-name:var(--motion);} animation:var(--motion) !important; --animation:pulse; animation:1s linear linear;}}"#,
    );
    let names = std::collections::BTreeMap::from([
        ("pulse".into(), "pulse-owner".into()),
        ("linear".into(), "linear-owner".into()),
    ]);
    let result = cem_ml::css_emission::emit_css_rule_subtree_with_symbols(
        &plan,
        first_rule(&plan),
        CssRuleMode::Instance,
        &names,
    )
    .unwrap();
    assert_eq!(result.css(), "@media all {:scope .card {animation:\"pulse-owner\" 1s linear;background:url(\"https://example.test/styles/icon.svg\");@supports (display:grid) {animation-name:\"pulse-owner\", external;&.active {animation-name:\"pulse-owner\";}}--animation:pulse;animation:1s linear \"linear-owner\";}}");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect::<Vec<_>>(),
        [
            "cem.scoped_css.important_unsupported",
            "cem.scoped_css.animation_name_dynamic_unsupported",
        ]
    );
    assert!(result
        .fragments
        .iter()
        .all(|f| f.source.origin().is_some() && f.range.length > 0));
    let declaration = result
        .fragments
        .iter()
        .find(|f| f.text.starts_with("animation:"))
        .unwrap();
    assert!(plan
        .tree
        .node(declaration.node_id)
        .unwrap()
        .name
        .as_ref()
        .is_some_and(|n| n.local_name == "declaration"));
    // The resource plan and retained names are reusable in another ownership context.
    let other = std::collections::BTreeMap::from([("pulse".into(), "pulse-other".into())]);
    let result = cem_ml::css_emission::emit_css_rule_subtree_with_symbols(
        &plan,
        first_rule(&plan),
        CssRuleMode::Declaration,
        &other,
    )
    .unwrap();
    assert!(result.css().contains("animation:\"pulse-other\" 1s linear"));
    assert!(result.css().contains("animation:1s linear linear;"));
}

#[test]
fn stylesheet_collects_forward_names_and_preserves_conditional_duplicate_definitions() {
    let plan = plan(
        r#".card {animation:pulse 1s linear; animation-name:pulse, external} @keyframes pulse {from {opacity:0} to {opacity:1}} @media all {@keyframes pulse {from {opacity:.2} to {opacity:.8}}} @keyframes "empty" {}"#,
    );
    let result =
        cem_ml::css_emission::emit_css_stylesheet(&plan, CssRuleMode::Declaration, "owner")
            .unwrap();
    assert!(
        result.body.diagnostics.is_empty(),
        "{:?}",
        result.body.diagnostics
    );
    assert_eq!(
        result.animation_names.get("pulse").map(String::as_str),
        Some("pulse-owner")
    );
    assert_eq!(
        result.animation_names.get("empty").map(String::as_str),
        Some("empty-owner")
    );
    assert_eq!(result.body.css(), ".card {animation:\"pulse-owner\" 1s linear;animation-name:\"pulse-owner\", external;}@keyframes pulse-owner {from {opacity:0;}to {opacity:1;}}@media all {@keyframes pulse-owner {from {opacity:.2;}to {opacity:.8;}}}@keyframes empty-owner {}");
    assert!(result
        .body
        .fragments
        .iter()
        .all(|f| f.source.origin().is_some() && f.range.length > 0));
    assert!(result
        .body
        .fragments
        .iter()
        .any(|f| f.text == "opacity:.8;"));
    assert!(
        cem_ml::css_emission::emit_css_stylesheet(&plan, CssRuleMode::Declaration, "").is_err()
    );
}

#[test]
fn stylesheet_does_not_collect_names_from_suppressed_branches() {
    let plan = plan("@import 'other.css'; .card {animation-name:hidden, bad, nested;} @layer private {@keyframes hidden {}} @supports invalid {@keyframes bad {}} .card {@keyframes nested {}} @keyframes none {} @font-face {font-family:private;src:url(font.woff)}");
    let result =
        cem_ml::css_emission::emit_css_stylesheet(&plan, CssRuleMode::Declaration, "owner")
            .unwrap();
    assert!(result.animation_names.is_empty());
    assert_eq!(
        result.body.css(),
        ".card {animation-name:hidden, bad, nested;}"
    );
    assert!(result
        .body
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scoped_css.keyframe_name_invalid"));
    assert!(result
        .body
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scoped_css.stylesheet_import_pending"));
    assert!(result
        .body
        .diagnostics
        .iter()
        .all(|d| d.source.origin().is_some()));
}
