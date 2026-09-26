use cem_ml::{
    css_emission::{emit_css_nested_selectors, CssRuleMode},
    import::import_data,
    parser::tree::RetainedCemTree,
    schema::registry::CSS_SCHEMA_URI,
};
fn rules(tree: &RetainedCemTree) -> Vec<u32> {
    (0..tree.ast().nodes.len() as u32)
        .filter(|id| {
            let node = tree.node(*id).unwrap();
            node.name
                .as_ref()
                .is_some_and(|n| n.namespace_uri == CSS_SCHEMA_URI && n.local_name == "rule")
        })
        .collect()
}
fn emitted(css: &str, mode: CssRuleMode) -> cem_ml::css_emission::CssSelectorEmission {
    let tree = import_data(css, "text/css", "cem", "nested.css").unwrap();
    emit_css_nested_selectors(&tree, *rules(&tree).last().unwrap(), mode).unwrap()
}
#[test]
fn emits_native_nesting_with_composed_authored_specificity() {
    for (selector, text, weight) in [
        ("&.active", "&.active", (0, 2, 0)),
        (".label", "& .label", (0, 2, 0)),
        ("> .label", "& > .label", (0, 2, 0)),
        (".outer &", ".outer &", (0, 2, 0)),
        (":where(&).label", ":where(&).label", (0, 1, 0)),
        (":is(&, .other)", ":is(&, .other)", (0, 1, 0)),
        ("&::before", "&::before", (0, 1, 1)),
    ] {
        let css = format!(".card {{ {selector} {{ color:red }} }}");
        let result = emitted(&css, CssRuleMode::Declaration);
        assert!(
            result.diagnostics.is_empty(),
            "{selector}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.selectors.len(), 1);
        assert_eq!(result.selectors[0].text, text);
        assert_eq!(result.selectors[0].authored_specificity, weight);
        let range = result.selectors[0].range;
        assert_eq!(
            &css[range.offset as usize..(range.offset + range.length) as usize],
            selector
        );
    }
}
#[test]
fn parent_list_maximum_and_multilevel_context_enforce_ceiling() {
    for css in [
        ".card, .card.active { & .label {} }",
        ".card { .label { &.active {} } }",
        ".card { @media screen { @supports (display:grid) { &.active.extra {} } } }",
    ] {
        let result = emitted(css, CssRuleMode::Declaration);
        assert!(result.selectors.is_empty(), "{css}");
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.scoped_css.specificity_unsupported"));
    }
    let result = emitted(
        ".card.active { :where(&) { .label {} } }",
        CssRuleMode::Declaration,
    );
    assert_eq!(result.selectors[0].authored_specificity, (0, 1, 0));
}
#[test]
fn rejected_parent_branches_do_not_contribute_to_children() {
    let result = emitted(
        ".card, #blocked, .a.b.c { &.active {} }",
        CssRuleMode::Declaration,
    );
    assert_eq!(result.selectors.len(), 1);
    assert_eq!(result.selectors[0].authored_specificity, (0, 2, 0));
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scoped_css.id_selector_unsupported"));
    let result = emitted("#blocked { :where(&) {} }", CssRuleMode::Declaration);
    assert!(result.selectors.is_empty());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scoped_css.nesting_parent_suppressed"));
}
#[test]
fn inherited_compound_weight_cannot_be_duplicated() {
    for css in [
        ".card { &.card {} }",
        ".card { && {} }",
        "[part=label] { &[part='label'] {} }",
        ".card { &.active { &.active {} } }",
        ".outer .card { &.card {} }",
    ] {
        let result = emitted(css, CssRuleMode::Instance);
        assert!(result.selectors.is_empty(), "{css}");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.scoped_css.manufactured_specificity_unsupported"),
            "{css}"
        );
    }
    for css in [
        ".card { & .card {} }",
        ".outer .card { &.outer {} }",
        ".card { :where(&&).card {} }",
        ":where(.card) { && {} }",
    ] {
        assert_eq!(
            emitted(css, CssRuleMode::Instance).selectors.len(),
            1,
            "{css}"
        );
    }
}
#[test]
fn instance_prefix_and_host_normalization_are_applied_at_the_correct_level() {
    let result = emitted(".card { .label { &.active {} } }", CssRuleMode::Instance);
    assert_eq!(result.selectors[0].text, "&.active");
    assert_eq!(result.selectors[0].authored_specificity, (0, 3, 0));
    let result = emitted(":host { &.active {} }", CssRuleMode::Declaration);
    assert_eq!(result.selectors[0].authored_specificity, (0, 2, 0));
    let result = emitted(".card { :host(&) {} }", CssRuleMode::Declaration);
    assert_eq!(result.selectors[0].text, ":where(:scope)&");
    assert_eq!(result.selectors[0].authored_specificity, (0, 2, 0));
}
#[test]
fn nested_api_requires_a_nested_rule_and_preserves_unsupported_diagnostics() {
    let tree = import_data(".card {}", "text/css", "cem", "nested.css").unwrap();
    assert!(emit_css_nested_selectors(&tree, rules(&tree)[0], CssRuleMode::Declaration).is_err());
    let result = emitted(".card { &:nth-child(2n) {} }", CssRuleMode::Declaration);
    assert!(result.selectors.is_empty());
    assert_eq!(
        result.diagnostics[0].code,
        "cem.scoped_css.selector_unsupported"
    );
}

#[test]
fn bounded_specificity_and_case_insensitive_group_ancestry() {
    let result = emitted(
        ".card { @MEDIA screen { @SUPPORTS (display:grid) { &.active {} } } }",
        CssRuleMode::Declaration,
    );
    assert_eq!(result.selectors[0].authored_specificity, (0, 2, 0));
    let css = format!(
        ".card {{ {} color:red; {} }}",
        "& & {".repeat(33),
        "}".repeat(33)
    );
    let result = emitted(&css, CssRuleMode::Instance);
    assert!(result.selectors.is_empty());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scoped_css.specificity_overflow"));
}

#[test]
fn functional_intersections_do_not_hide_repeated_parent_weight() {
    for css in [
        ":host { :is(&):is(&) {} }",
        ".card { :is(&).card {} }",
        ":is(.card, .other) { &.card {} }",
    ] {
        let result = emitted(css, CssRuleMode::Instance);
        assert!(result.selectors.is_empty(), "{css}");
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.scoped_css.manufactured_specificity_unsupported"));
    }
}

#[test]
fn pseudo_element_parent_branches_contribute_weight_but_not_duplicate_subject_tokens() {
    let result = emitted(
        ".card, .label::before { &.label {} }",
        CssRuleMode::Declaration,
    );
    assert!(result.diagnostics.is_empty());
    assert_eq!(result.selectors[0].authored_specificity, (0, 2, 1));
    assert_eq!(result.selectors[0].text, "&.label");
}

#[test]
fn functional_global_alias_retains_parent_weight_and_subject_intersections() {
    let result = emitted(
        ":global(.active) { :where(&) .label {} }",
        CssRuleMode::Declaration,
    );
    assert_eq!(result.selectors[0].text, ":where(&) .label");
    assert_eq!(result.selectors[0].authored_specificity, (0, 1, 0));
    assert_eq!(result.diagnostics[0].code, "cem.scoped_css.global_alias");
    let result = emitted(".card { :global(&) {} }", CssRuleMode::Declaration);
    assert_eq!(result.selectors[0].text, ":where(:scope)&");
    assert_eq!(result.selectors[0].authored_specificity, (0, 2, 0));
    let result = emitted(":global(.card) { &.card {} }", CssRuleMode::Instance);
    assert!(result.selectors.is_empty());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.scoped_css.manufactured_specificity_unsupported"));
}
