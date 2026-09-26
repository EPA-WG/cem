use std::sync::Arc;

use cem_ml::{
    css_emission::{
        emit_css_scope_wrapper, emit_css_style_rule, CssManagedScope, CssRuleBodyItem, CssRuleMode,
    },
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
fn style_rule_assembly_retains_order_sources_and_deferred_boundaries() {
    let css = ":host,.item { color:red; @media screen { color:blue; } background:url(icon.svg); & .child {color:pink} color:green; }";
    let plan = plan(css);
    let result = emit_css_style_rule(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let rule = result.rule.unwrap();
    assert_eq!(
        rule.selectors
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>(),
        [":where(:scope)", ".item"]
    );
    assert_eq!(rule.body.len(), 5);
    let mut order = Vec::new();
    for (index, item) in rule.body.iter().enumerate() {
        match item {
            CssRuleBodyItem::Declaration(d) => {
                order.push(d.node_id);
                assert_eq!(
                    d.text,
                    match index {
                        0 => "color:red;",
                        2 => "background:url(\"https://example.test/styles/icon.svg\");",
                        4 => "color:green;",
                        _ => panic!("declaration moved across deferred child"),
                    }
                );
                assert!(d.source.origin().is_some());
            }
            CssRuleBodyItem::Deferred(d) => {
                order.push(d.node_id);
                let text =
                    &css[d.range.offset as usize..(d.range.offset + d.range.length) as usize];
                assert_eq!(
                    text,
                    match index {
                        1 => "@media screen { color:blue; }",
                        3 => "& .child {color:pink}",
                        _ => panic!("deferred child moved"),
                    }
                );
                assert!(d.source.origin().is_some());
            }
        }
    }
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(rule.source.origin().is_some());
    assert_eq!(rule.range.length as usize, css.len());
}

#[test]
fn style_rule_assembly_combines_policy_diagnostics_and_preserves_valid_siblings() {
    let plan = plan("#bad,.item { color:red !important; background:url(\"\"); color:green; }");
    let result = emit_css_style_rule(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    let rule = result.rule.unwrap();
    assert_eq!(rule.selectors.len(), 1);
    assert_eq!(rule.selectors[0].text, ".item");
    assert_eq!(rule.body.len(), 1);
    assert!(matches!(&rule.body[0], CssRuleBodyItem::Declaration(d) if d.text == "color:green;"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect::<Vec<_>>(),
        [
            "cem.scoped_css.id_selector_unsupported",
            "cem.scoped_css.important_unsupported",
            "cem.scoped_css.resource_resolution_failed",
        ]
    );
}

#[test]
fn style_rule_assembly_handles_empty_suppressed_and_instance_rules() {
    for css in [
        "#bad {color:red}",
        ".item {}",
        ".item {color:red !important}",
    ] {
        let plan = plan(css);
        assert!(
            emit_css_style_rule(&plan, first_rule(&plan), CssRuleMode::Declaration)
                .unwrap()
                .rule
                .is_none()
        );
    }
    let plan = plan(":host,.one.two.three {color:red}");
    let declaration =
        emit_css_style_rule(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    assert_eq!(declaration.rule.unwrap().selectors.len(), 1);
    let instance = emit_css_style_rule(&plan, first_rule(&plan), CssRuleMode::Instance).unwrap();
    assert!(instance.diagnostics.is_empty());
    assert_eq!(
        instance
            .rule
            .unwrap()
            .selectors
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>(),
        [":scope", ":scope .one.two.three"]
    );
    assert!(emit_css_style_rule(&plan, 0, CssRuleMode::Declaration).is_err());
}

#[test]
fn scope_wrappers_preserve_private_shared_and_instance_boundaries() {
    let limit = " to (:scope :has(> template[data-cem-island=\"instance\"]) > *, [slot] > *) {";
    for (scope, root) in [
        (CssManagedScope::Private { tag: "cem-card".into(), context: None }, "@scope (cem-card)"),
        (CssManagedScope::Private { tag: "cem-card".into(), context: Some("ctx".into()) }, "@scope (cem-card[data-cem-css-context=\"ctx\"])"),
        (CssManagedScope::Shared { name: "controls".into(), context: None }, "@scope ([scope=\"controls\"]:has(> template[data-cem-island=\"instance\"]))"),
        (CssManagedScope::Shared { name: "controls".into(), context: Some("ctx".into()) }, "@scope ([scope=\"controls\"][data-cem-css-context=\"ctx\"]:has(> template[data-cem-island=\"instance\"]))"),
        (CssManagedScope::Instance, "@scope"),
    ] {
        let wrapper = emit_css_scope_wrapper(&scope).unwrap();
        assert_eq!(wrapper.opening, format!("{root}{limit}"));
        assert_eq!(wrapper.closing, "}");
        assert!(!wrapper.opening.contains("data-cem-render-scope"));
        assert!(!wrapper.opening.contains("data-cem-instance-scope"));
        import_data(&format!("{} .item {{color:red}} {}", wrapper.opening, wrapper.closing),
            "text/css", "cem", "scope.css").unwrap();
    }
}

#[test]
fn scope_wrapper_encodes_semantic_names_and_rejects_empty_inputs() {
    let wrapper = emit_css_scope_wrapper(&CssManagedScope::Private {
        tag: "cem-card,body".into(),
        context: Some("x\"] {color:red} /*".into()),
    })
    .unwrap();
    assert!(wrapper.opening.contains("cem-card\\2C body"));
    assert!(wrapper
        .opening
        .contains("[data-cem-css-context=\"x\\\"] {color:red} /*\"]"));
    for scope in [
        CssManagedScope::Private {
            tag: String::new(),
            context: None,
        },
        CssManagedScope::Shared {
            name: String::new(),
            context: None,
        },
        CssManagedScope::Shared {
            name: "controls".into(),
            context: Some(String::new()),
        },
    ] {
        assert_eq!(
            emit_css_scope_wrapper(&scope).unwrap_err().code,
            "cem.scoped_css.scope_invalid"
        );
    }
}

#[test]
fn style_rule_assembly_keeps_deferred_only_bodies_and_rejects_incomplete_resource_plans() {
    let plan = plan(".item { @media screen {color:red} }");
    let result = emit_css_style_rule(&plan, first_rule(&plan), CssRuleMode::Declaration).unwrap();
    let rule = result.rule.unwrap();
    assert_eq!(rule.body.len(), 1);
    assert!(matches!(rule.body[0], CssRuleBodyItem::Deferred(_)));

    let mut plan = self::plan(".item {background:url(icon.svg)}");
    plan.references.clear();
    assert_eq!(
        emit_css_style_rule(&plan, first_rule(&plan), CssRuleMode::Declaration)
            .unwrap_err()
            .code,
        "cem.scoped_css.resource_plan_invalid"
    );
}
