use cem_ml::{
    css_emission::{emit_css_declaration_selectors, emit_css_instance_selectors},
    import::import_data,
    parser::tree::RetainedCemTree,
};

fn elements(tree: &RetainedCemTree, name: &str) -> Vec<u32> {
    (0..tree.ast().nodes.len() as u32)
        .filter(|id| {
            tree.node(*id).unwrap().name.as_ref().is_some_and(|n| {
                n.namespace_uri == cem_ml::schema::registry::CSS_SCHEMA_URI && n.local_name == name
            })
        })
        .collect()
}
fn attr(tree: &RetainedCemTree, id: u32, name: &str) -> Option<String> {
    tree.node(id).unwrap().attributes.iter().find_map(|id| {
        let node = tree.node(*id).unwrap();
        (node.name.as_ref().unwrap().local_name == name).then(|| node.value.clone())
    })
}
fn list(tree: &RetainedCemTree, rule: u32) -> u32 {
    *tree
        .node(rule)
        .unwrap()
        .children
        .iter()
        .find(|id| tree.node(**id).unwrap().name.as_ref().unwrap().local_name == "selector-list")
        .unwrap()
}

#[test]
fn retains_explicit_implicit_and_functional_nesting_without_false_specificity() {
    for selector in [
        "&.active",
        "> .label",
        ".label",
        ".outer &",
        ":is(&, .other)",
        ":where(&) .label",
        "&::before",
        "&&",
    ] {
        let css = format!(".card {{ {selector} {{ color: red; }} }}");
        let tree = import_data(&css, "text/css", "cem", "nesting.css").unwrap();
        let rules = elements(&tree, "rule");
        assert_eq!(rules.len(), 2, "{selector}");
        assert_eq!(
            attr(&tree, rules[0], "selector-context").as_deref(),
            Some("root")
        );
        assert_eq!(
            attr(&tree, rules[1], "selector-context").as_deref(),
            Some("nested")
        );
        let nested = list(&tree, rules[1]);
        assert_eq!(
            attr(&tree, nested, "analysis-status").as_deref(),
            Some("complete"),
            "{selector}"
        );
        let selected = tree.node(nested).unwrap().children[0];
        assert_eq!(
            attr(&tree, selected, "specificity-kind").as_deref(),
            Some("parent-dependent")
        );
        assert_eq!(attr(&tree, selected, "specificity"), None);
        let range = tree.node(selected).unwrap().range;
        assert_eq!(
            &css[range.offset as usize..(range.offset + range.length) as usize],
            selector
        );
        for selected in elements(&tree, "selector") {
            let range = tree.node(selected).unwrap().range;
            let text = &css[range.offset as usize..(range.offset + range.length) as usize];
            if text.contains('&') {
                assert_eq!(attr(&tree, selected, "specificity"), None, "{text}");
                assert_eq!(
                    attr(&tree, selected, "specificity-kind").as_deref(),
                    Some("parent-dependent"),
                    "{text}"
                );
            } else if text == ".other" {
                assert_eq!(
                    attr(&tree, selected, "specificity").as_deref(),
                    Some("0-1-0")
                );
            }
        }
        let nesting: Vec<_> = elements(&tree, "simple-selector")
            .into_iter()
            .filter(|id| attr(&tree, *id, "kind").as_deref() == Some("nesting"))
            .collect();
        assert_eq!(nesting.len(), selector.matches('&').count());
        for id in nesting {
            let range = tree.node(id).unwrap().range;
            assert_eq!(
                &css[range.offset as usize..(range.offset + range.length) as usize],
                "&"
            );
        }
    }
}

#[test]
fn grouping_preserves_style_context_but_does_not_create_it() {
    for group in [
        "@media screen",
        "@supports (display:grid)",
        "@layer example",
        "@container (width > 1px)",
        "@starting-style",
    ] {
        let css = format!(
            ".card {{ {group} {{ @media print {{ > .child {{ &.active {{color:red}} }} }} }} }}"
        );
        let tree = import_data(&css, "text/css", "cem", "nesting.css").unwrap();
        let styles: Vec<_> = elements(&tree, "rule")
            .into_iter()
            .filter(|id| attr(&tree, *id, "kind").as_deref() == Some("style"))
            .collect();
        for &rule in &styles[1..] {
            assert_eq!(
                attr(&tree, rule, "selector-context").as_deref(),
                Some("nested"),
                "{group}"
            );
            assert_eq!(
                attr(&tree, list(&tree, rule), "analysis-status").as_deref(),
                Some("complete")
            );
        }
        let tree = import_data(
            &format!("{group} {{ .child {{ color:red }} }}"),
            "text/css",
            "cem",
            "nesting.css",
        )
        .unwrap();
        let child = elements(&tree, "rule")[1];
        assert_eq!(
            attr(&tree, child, "selector-context").as_deref(),
            Some("root")
        );
        assert_eq!(
            attr(
                &tree,
                tree.node(list(&tree, child)).unwrap().children[0],
                "specificity"
            )
            .as_deref(),
            Some("0-1-0")
        );
    }
}

#[test]
fn root_nesting_and_unsupported_nested_grammar_remain_unsupported() {
    for css in [
        "& .child {}",
        "> .child {}",
        ".card { &-suffix {} }",
        ".card { &::part(label) {} }",
        ".card { & :nth-child(2n) {} }",
    ] {
        let tree = import_data(css, "text/css", "cem", "nesting.css").unwrap();
        let rule = *elements(&tree, "rule").last().unwrap();
        assert_eq!(
            attr(&tree, list(&tree, rule), "analysis-status").as_deref(),
            Some("unsupported"),
            "{css}"
        );
    }
}

#[test]
fn context_free_emitters_suppress_nested_rules_even_without_ampersands() {
    for selector in ["&.active", ".child", "> .child", ":where(&)"] {
        let tree = import_data(
            &format!(".card {{ {selector} {{color:red}} }}"),
            "text/css",
            "cem",
            "nesting.css",
        )
        .unwrap();
        let nested = elements(&tree, "rule")[1];
        for result in [
            emit_css_declaration_selectors(&tree, nested),
            emit_css_instance_selectors(&tree, nested),
        ] {
            let result = result.unwrap();
            assert!(result.selectors.is_empty(), "{selector}");
            assert_eq!(
                result.diagnostics[0].code,
                "cem.scoped_css.nesting_context_required"
            );
            assert_eq!(
                result.diagnostics[0].range.offset,
                tree.node(nested).unwrap().range.offset
            );
            assert_eq!(
                result.diagnostics[0].range.length,
                tree.node(nested).unwrap().range.length
            );
        }
    }
}

#[test]
fn standalone_query_profile_still_rejects_nesting() {
    use cem_ml::validation::css_selector::{
        css_selector_expression_ast_from_source_bytes, CssSelectorSourceRequest,
    };
    for selector in ["&", "&.active", ":is(&, .card)", ":where(&)", "> .child"] {
        let (_, diagnostics) =
            css_selector_expression_ast_from_source_bytes(CssSelectorSourceRequest {
                bytes: selector.as_bytes(),
                source_uri: "query.css-selector",
                content_type: Some(cem_ml::schema::registry::CSS_SELECTOR_CONTENT_TYPE),
                namespace_bindings: &Default::default(),
            });
        assert!(!diagnostics.is_empty(), "{selector}");
    }
}

#[test]
fn non_grouping_bodies_do_not_inherit_parent_selector_context() {
    let tree = import_data(
        ".card { @keyframes move { from {opacity:0} to {opacity:1} } }",
        "text/css",
        "cem",
        "nesting.css",
    )
    .unwrap();
    let styles: Vec<_> = elements(&tree, "rule")
        .into_iter()
        .filter(|id| attr(&tree, *id, "kind").as_deref() == Some("style"))
        .collect();
    assert_eq!(styles.len(), 3);
    for rule in styles {
        assert_eq!(
            attr(&tree, rule, "selector-context").as_deref(),
            Some("root")
        );
    }
}

#[test]
fn authored_scope_does_not_mislabel_scope_references_as_style_parent_references() {
    for css in [
        "@scope (.inside) { & .child {} }",
        ".card { @scope (.inside) { & .child {} } }",
        ".card { @scope (.inside) { @media screen { & .child {} } } }",
    ] {
        let tree = import_data(css, "text/css", "cem", "nesting.css").unwrap();
        let rule = *elements(&tree, "rule").last().unwrap();
        assert_eq!(
            attr(&tree, rule, "selector-context").as_deref(),
            Some("root")
        );
        assert_eq!(
            attr(&tree, list(&tree, rule), "analysis-status").as_deref(),
            Some("unsupported")
        );
    }
}
