use std::sync::Arc;

use cem_ml::{
    css_emission::emit_css_rule_declarations_with_resources,
    css_resources::{resolve_css_resources, CssResourcePlan},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlMapping,
        CemModuleUrlResolutionCapability, CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    parser::tree::RetainedCemTree,
};

fn capability(target: &str) -> CemModuleUrlResolutionCapability {
    let mut outer = CemModuleUrlFrame::new("page", "https://example.test/page.html");
    outer
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target("./outer.svg"));
    outer
        .specifiers
        .resources
        .insert("blocked".into(), CemModuleUrlMapping::blocked());
    outer
        .specifiers
        .resources
        .insert("#local".into(), CemModuleUrlMapping::blocked());
    let mut inner = CemModuleUrlFrame::new("component", "https://example.test/components/card.cem");
    inner
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target(target));
    let handle = CemResolutionContextHandle::new(target);
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: target.into(),
            resolver_identity: "fixture".into(),
            resource_policy_stamp: "policy".into(),
            frames: vec![outer, inner],
        },
    );
    CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle)
}

fn plan(css: &str) -> CssResourcePlan {
    let tree = import_data(css, "text/css", "cem", "urn:css:source-owner").unwrap();
    resolve_css_resources(
        tree,
        &capability("./inner.svg"),
        "https://example.test/assets/imported.css",
    )
    .unwrap()
}
fn rule(tree: &RetainedCemTree) -> u32 {
    (0..tree.ast().nodes.len() as u32)
        .find(|id| {
            tree.node(*id)
                .unwrap()
                .name
                .as_ref()
                .is_some_and(|n| n.local_name == "rule")
        })
        .unwrap()
}
fn emitted(plan: &CssResourcePlan) -> Vec<String> {
    let result = emit_css_rule_declarations_with_resources(plan, rule(&plan.tree)).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.declarations.into_iter().map(|d| d.text).collect()
}

#[test]
fn resource_emission_uses_closest_maps_bases_and_nested_typed_references() {
    let css = r#".item { background:image-set(url(icon) 1x, URL( "./a\20 b.svg" /* c */) 2x); --bundle:{ note:"é"; icon: url(icon); note:"url(fake)" }; content:"é url(fake)"; mask:url(../mask.svg#part); }"#;
    let plan = plan(css);
    assert_eq!(emitted(&plan), [
        "background:image-set(url(\"https://example.test/components/inner.svg\") 1x, url(\"https://example.test/assets/a%20b.svg\") 2x);",
        "--bundle:{ note:\"é\"; icon: url(\"https://example.test/components/inner.svg\"); note:\"url(fake)\" };",
        "content:\"é url(fake)\";",
        "mask:url(\"https://example.test/mask.svg#part\");",
    ]);
    let result = emit_css_rule_declarations_with_resources(&plan, rule(&plan.tree)).unwrap();
    let declaration = &result.declarations[0];
    assert_eq!(
        &css[declaration.range.offset as usize
            ..(declaration.range.offset + declaration.range.length) as usize],
        r#"background:image-set(url(icon) 1x, URL( "./a\20 b.svg" /* c */) 2x);"#
    );
    assert!(declaration.source.origin().is_some());
    let second = resolve_css_resources(
        plan.tree.clone(),
        &capability("./other.svg"),
        "https://example.test/other/main.css",
    )
    .unwrap();
    assert!(Arc::ptr_eq(&plan.tree, &second.tree));
    assert!(emitted(&second)[0].contains("components/other.svg"));
    assert!(emitted(&second)[0].contains("other/a%20b.svg"));
    assert!(emitted(&plan)[0].contains("components/inner.svg"));
}

#[test]
fn resource_emission_suppresses_failed_declarations_and_keeps_siblings() {
    let css = ".item { background:linear-gradient(red,blue),url(blocked); color:green; mask:url(\"\"); outline:0 !important; }";
    let plan = plan(css);
    let result = emit_css_rule_declarations_with_resources(&plan, rule(&plan.tree)).unwrap();
    assert_eq!(
        result
            .declarations
            .iter()
            .map(|d| d.text.as_str())
            .collect::<Vec<_>>(),
        ["color:green;"]
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect::<Vec<_>>(),
        [
            "cem.scoped_css.resource_resolution_failed",
            "cem.scoped_css.resource_resolution_failed",
            "cem.scoped_css.important_unsupported",
        ]
    );
    assert_eq!(
        result.diagnostics[0].range.offset as usize,
        css.find("url(blocked)").unwrap()
    );
    assert!(result.diagnostics[0].source.origin().is_some());
    assert!(result.diagnostics[0].message.contains("blocked"));
}

#[test]
fn resource_emission_does_not_rewrite_local_fragments_or_ordinary_strings() {
    let plan = plan(
        r##".item { filter:URL( "#local" /* keep */); clip-path:url(#clip); --ref:url(\23 escaped); content:"url(icon)"; --name:foo\ ; }"##,
    );
    assert_eq!(
        emitted(&plan),
        [
            r##"filter:URL( "#local" /* keep */);"##,
            "clip-path:url(#clip);",
            r"--ref:url(\23 escaped);",
            r#"content:"url(icon)";"#,
            r"--name:foo\ ;",
        ]
    );
}

#[test]
fn resource_emission_requires_a_complete_unambiguous_plan() {
    let mut plan = plan(".item { background:url(icon); }");
    plan.references.clear();
    assert_eq!(
        emit_css_rule_declarations_with_resources(&plan, rule(&plan.tree))
            .unwrap_err()
            .code,
        "cem.scoped_css.resource_plan_invalid"
    );
    let mut plan = self::plan(".item { background:url(icon); }");
    plan.references[0].authored_specifier = "other".into();
    assert_eq!(
        emit_css_rule_declarations_with_resources(&plan, rule(&plan.tree))
            .unwrap_err()
            .code,
        "cem.scoped_css.resource_plan_invalid"
    );
}

#[test]
fn resource_emission_quotes_resolved_values_without_changing_css_structure() {
    let mut plan = plan(".item { background:url(icon); }");
    plan.references[0].resolution.as_mut().unwrap().resolved_url =
        "https://example.test/a\"\\\n);color:red".into();
    assert_eq!(
        emitted(&plan),
        ["background:url(\"https://example.test/a\\22 \\5C \\A );color:red\");"]
    );
    let css = format!(".item {{{}}}", emitted(&plan)[0]);
    let imported = self::plan(&css);
    assert_eq!(imported.references.len(), 1);
    assert_eq!(
        imported.references[0].authored_specifier,
        plan.references[0].resolution.as_ref().unwrap().resolved_url
    );
    assert_eq!(
        cem_ml::css_emission::emit_css_rule_declarations(&imported.tree, rule(&imported.tree))
            .unwrap()
            .declarations
            .len(),
        1
    );
}

#[test]
fn resource_emission_rejects_duplicate_and_mismatched_ranges() {
    let mut plan = plan(".item { background:url(icon); }");
    let mut second = resolve_css_resources(
        plan.tree.clone(),
        &capability("./inner.svg"),
        "https://example.test/assets/imported.css",
    )
    .unwrap();
    plan.references.push(second.references.remove(0));
    assert_eq!(
        emit_css_rule_declarations_with_resources(&plan, rule(&plan.tree))
            .unwrap_err()
            .code,
        "cem.scoped_css.resource_plan_invalid"
    );
    plan.references.pop();
    plan.references[0].range.offset += 1;
    assert_eq!(
        emit_css_rule_declarations_with_resources(&plan, rule(&plan.tree))
            .unwrap_err()
            .code,
        "cem.scoped_css.resource_plan_invalid"
    );
}

#[test]
fn resource_emission_preserves_nested_rule_boundaries_and_suppression_policy() {
    let plan = plan(".item { background:url(icon); @media screen { .child {mask:url(icon)} } color:green; --bad:image-set(url(bad value) 1x); mask:url(blocked) !important; }");
    let result = emit_css_rule_declarations_with_resources(&plan, rule(&plan.tree)).unwrap();
    assert_eq!(
        result
            .declarations
            .iter()
            .map(|d| d.text.as_str())
            .collect::<Vec<_>>(),
        [
            "background:url(\"https://example.test/components/inner.svg\");",
            "color:green;",
        ]
    );
    assert_eq!(result.deferred_children.len(), 1);
    assert!(result.declarations[0].node_id < result.deferred_children[0]);
    assert!(result.deferred_children[0] < result.declarations[1].node_id);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect::<Vec<_>>(),
        [
            "cem.scoped_css.declaration_value_unsupported",
            "cem.scoped_css.important_unsupported",
        ]
    );
}
