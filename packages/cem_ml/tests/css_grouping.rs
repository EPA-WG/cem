use cem_ml::{
    css_emission::{emit_css_grouping_rule, CssGroupingContext, CssRuleBodyItem},
    css_resources::{resolve_css_resources, CssResourcePlan},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
};
use std::sync::Arc;
fn plan(css: &str) -> CssResourcePlan {
    let tree = import_data(css, "text/css", "cem", "groups.css").unwrap();
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
fn group(plan: &CssResourcePlan) -> u32 {
    (0..plan.tree.ast().nodes.len() as u32)
        .find(|id| {
            let node = plan.tree.node(*id).unwrap();
            node.name.as_ref().is_some_and(|n| n.local_name == "rule")
                && node.children.iter().any(|id| {
                    plan.tree
                        .node(*id)
                        .unwrap()
                        .name
                        .as_ref()
                        .is_some_and(|n| n.local_name == "at-rule")
                })
        })
        .unwrap()
}
#[test]
fn grouping_media_retains_empty_and_recovered_queries() {
    for (condition, expected, recovered) in [
        ("screen and (color), print", "screen and (color), print", 0),
        ("screen, &bad,, print", "screen, not all, not all, print", 2),
        ("", "all", 0),
        ("/* empty */", "all", 0),
        ("future(a,b)", "future(a,b)", 0),
    ] {
        let source = format!("@media {condition} {{ .item {{color:red}} }}");
        let plan = plan(&source);
        let result =
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
        assert_eq!(result.diagnostics.len(), recovered);
        let rule = result.rule.unwrap();
        assert_eq!(rule.opening, format!("@media {expected} {{"));
        assert_eq!(rule.body.len(), 1);
        assert!(matches!(rule.body[0], CssRuleBodyItem::Deferred(_)));
        assert_eq!(rule.range.length as usize, source.len());
        assert!(rule.source.origin().is_some());
    }
}
#[test]
fn grouping_supports_preserves_conditions_and_suppresses_invalid_forms() {
    for condition in [
        "(display:grid)",
        "not (display:grid)",
        "selector(:has(> .item))",
        "(color:red) or (color:blue)",
        "(future syntax; !)",
    ] {
        let plan = plan(&format!("@supports {condition} {{ .item {{color:red}} }}"));
        let result =
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.rule.unwrap().opening,
            format!("@supports {condition} {{")
        );
    }
    for condition in [
        "",
        "display:grid",
        "(a) and",
        "not not (a)",
        "(a) and (b) or (c)",
    ] {
        let plan = plan(&format!("@supports {condition} {{ .item {{color:red}} }}"));
        let result =
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
        assert!(result.rule.is_none());
        assert_eq!(
            result.diagnostics[0].code,
            "cem.scoped_css.supports_condition_invalid"
        );
    }
}
#[test]
fn grouping_nested_declarations_keep_order_and_resolved_urls() {
    let plan = plan(".item { @media screen {color:red; @supports (display:grid) {color:blue} background:url(icon.svg); color:green !important;} }");
    let result =
        emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::StyleRule).unwrap();
    let body = result.rule.unwrap().body;
    assert_eq!(body.len(), 3);
    assert!(matches!(&body[0], CssRuleBodyItem::Declaration(d) if d.text == "color:red;"));
    assert!(matches!(&body[1], CssRuleBodyItem::Deferred(_)));
    assert!(
        matches!(&body[2], CssRuleBodyItem::Declaration(d) if d.text == "background:url(\"https://example.test/styles/icon.svg\");")
    );
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.important_unsupported"
    );
    let result =
        emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
    assert_eq!(result.rule.unwrap().body.len(), 1);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "cem.scoped_css.group_declaration_unsupported")
            .count(),
        2
    );
}
#[test]
fn grouping_rejects_missing_structure_and_non_block_rules() {
    let plan = plan("@media screen;");
    assert!(
        emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet)
            .unwrap()
            .rule
            .is_none()
    );
    assert!(emit_css_grouping_rule(&plan, 0, CssGroupingContext::Stylesheet).is_err());
    let tree = import_data("<rule xmlns='https://cem.dev/ns/data/css/1' kind='at' name='media' has-block='true'><at-rule name='media' prelude='screen'/></rule>", "application/xml", "cem", "missing.xml").unwrap();
    let rule = tree.node(0).unwrap().children[0];
    let plan = CssResourcePlan {
        tree,
        references: vec![],
    };
    assert_eq!(
        emit_css_grouping_rule(&plan, rule, CssGroupingContext::Stylesheet)
            .unwrap_err()
            .code,
        "cem.scoped_css.group_tree_invalid"
    );
}

#[test]
fn grouping_keeps_recovered_query_source_and_nested_context() {
    let source = ".item { @MEDIA screen,&bad {color:red; @SUPPORTS (display:grid) {color:blue}} }";
    let plan = plan(source);
    let result =
        emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::StyleRule).unwrap();
    let diagnostic = &result.diagnostics[0];
    assert_eq!(diagnostic.code, "cem.scoped_css.media_query_recovered");
    assert_eq!(
        &source[diagnostic.range.offset as usize
            ..(diagnostic.range.offset + diagnostic.range.length) as usize],
        "&bad "
    );
    assert!(diagnostic.source.origin().is_some());
    let rule = result.rule.unwrap();
    assert_eq!(rule.opening, "@media screen, not all {");
    let CssRuleBodyItem::Deferred(child) = &rule.body[1] else {
        panic!("missing nested group")
    };
    let nested =
        emit_css_grouping_rule(&plan, child.node_id, CssGroupingContext::StyleRule).unwrap();
    assert!(nested.diagnostics.is_empty());
    let nested = nested.rule.unwrap();
    assert_eq!(nested.opening, "@supports (display:grid) {");
    assert!(matches!(&nested.body[0], CssRuleBodyItem::Declaration(d) if d.text == "color:blue;"));
}

#[test]
fn grouping_omits_empty_bodies_and_does_not_accept_other_at_rules() {
    for source in [
        "@media screen {}",
        "@supports (display:grid) {}",
        "@media {color:red}",
        "@container panel {}",
        "@container panel {color:red}",
    ] {
        let plan = plan(source);
        assert!(
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet)
                .unwrap()
                .rule
                .is_none()
        );
    }
    let plan = plan("@layer local { .item {color:red} }");
    assert_eq!(
        emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet)
            .unwrap_err()
            .code,
        "cem.scoped_css.group_tree_invalid"
    );
}

#[test]
fn starting_style_validates_empty_prelude_and_preserves_group_context() {
    for prelude in ["", " /* retained comment */ "] {
        let source = format!(".card {{ @STARTING-STYLE {prelude} {{opacity:0; &.active {{color:red}} opacity:0.5;}} }}");
        let plan = plan(&source);
        let result =
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::StyleRule).unwrap();
        assert!(result.diagnostics.is_empty());
        let rule = result.rule.unwrap();
        assert_eq!(rule.opening, "@starting-style {");
        assert_eq!(rule.body.len(), 3);
        assert!(matches!(&rule.body[0], CssRuleBodyItem::Declaration(d) if d.text == "opacity:0;"));
        assert!(matches!(&rule.body[1], CssRuleBodyItem::Deferred(_)));
        assert!(
            matches!(&rule.body[2], CssRuleBodyItem::Declaration(d) if d.text == "opacity:0.5;")
        );
        assert!(rule.source.origin().is_some());
    }
    let plan = plan("@starting-style {opacity:0; .card {opacity:0}} ");
    let result =
        emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
    assert_eq!(result.rule.unwrap().body.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.group_declaration_unsupported"
    );
}

#[test]
fn starting_style_rejects_nonempty_preludes_and_missing_blocks() {
    for prelude in ["screen", "(width:1px)", "future()", "/* comment */ invalid"] {
        let plan = plan(&format!(
            "@starting-style {prelude} {{.card {{opacity:0}}}}"
        ));
        let result =
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
        assert!(result.rule.is_none());
        assert_eq!(
            result.diagnostics[0].code,
            "cem.scoped_css.starting_style_prelude_invalid"
        );
        assert!(result.diagnostics[0].source.origin().is_some());
    }
    let plan = plan("@starting-style;");
    let result =
        emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
    assert!(result.rule.is_none());
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.group_block_required"
    );
}

#[test]
fn container_groups_preserve_named_boolean_and_future_queries() {
    for condition in [
        "(width > 300px)",
        "panel (width > 300px)",
        "panel",
        "only",
        "not (width > 300px)",
        "panel not (width > 300px)",
        "(width > 300px) and (height > 100px)",
        "(width > 300px) or style(--theme: dark)",
        "panel (width > 300px), style(--theme: dark)",
        "future(a,b)",
        "scroll-state(stuck: top)",
        "p\\61 nel /* name */ (width > 300px)",
    ] {
        let plan = plan(&format!("@container {condition} {{ .card {{color:red}} }}"));
        let result =
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
        assert!(result.diagnostics.is_empty(), "{condition}");
        let rule = result.rule.unwrap();
        assert_eq!(rule.opening, format!("@container {condition} {{"));
        assert_eq!(rule.body.len(), 1);
        assert!(rule.source.origin().is_some());
    }
}

#[test]
fn container_groups_reject_invalid_lists_names_and_boolean_structure() {
    for condition in [
        "",
        "/* empty */",
        ",",
        "panel,",
        ",panel",
        "panel,,other",
        "panel, none",
        "none",
        "inherit",
        "initial (width)",
        "default",
        "revert-layer",
        "n\\6f ne",
        "and (width)",
        "or (width)",
        "not",
        "not panel",
        "panel other",
        "panel and (width)",
        "(width) (height)",
        "(width) and",
        "(width) and (height) or (orientation)",
        "screen and (width)",
        "[width]",
        "(width) invalid",
    ] {
        let plan = plan(&format!("@container {condition} {{ .card {{color:red}} }}"));
        let result =
            emit_css_grouping_rule(&plan, group(&plan), CssGroupingContext::Stylesheet).unwrap();
        assert!(result.rule.is_none(), "{condition}");
        assert_eq!(
            result.diagnostics[0].code, "cem.scoped_css.container_condition_invalid",
            "{condition}"
        );
    }
}
