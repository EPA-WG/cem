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
