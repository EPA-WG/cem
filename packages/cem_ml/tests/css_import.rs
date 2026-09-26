use cem_ml::{
    import::{import_data, import_data_bytes, try_retain_lifecycle},
    lifecycle::LoadedInputAstStream,
    parser::tree::RetainedCemTree,
    schema::registry::CSS_SCHEMA_URI,
};
use std::sync::Arc;

fn elements(tree: &RetainedCemTree, name: &str) -> Vec<u32> {
    (0..tree.ast().nodes.len() as u32)
        .filter(|id| {
            tree.node(*id)
                .unwrap()
                .name
                .as_ref()
                .is_some_and(|n| n.namespace_uri == CSS_SCHEMA_URI && n.local_name == name)
        })
        .collect()
}
fn attr(tree: &RetainedCemTree, id: u32, name: &str) -> Option<String> {
    tree.node(id).unwrap().attributes.iter().find_map(|id| {
        let node = tree.node(*id).unwrap();
        (node.name.as_ref().unwrap().local_name == name).then(|| node.value.clone())
    })
}

#[test]
fn css_import_retains_semantic_structure_urls_and_native_source() {
    let source = "/*url(fake)*/\n@import url(\"theme\") layer(base) supports(display:grid) screen;\n.card { background: url(icon); content: \"url(fake)\"; --x: { a: url(other) }; color: red !important; }";
    let tree = import_data(source, "text/css", "cem", "https://example.test/main.css").unwrap();
    let root = elements(&tree, "stylesheet")[0];
    assert_eq!(attr(&tree, root, "encoding").as_deref(), Some("utf-8"));
    let import = elements(&tree, "import")[0];
    assert_eq!(attr(&tree, import, "href").as_deref(), Some("theme"));
    assert_eq!(attr(&tree, import, "layer").as_deref(), Some("base"));
    assert_eq!(
        attr(&tree, import, "supports").as_deref(),
        Some("display:grid")
    );
    assert_eq!(attr(&tree, import, "media").as_deref(), Some("screen"));
    assert_eq!(tree.node(import).unwrap().range.line, 2);
    assert_eq!(
        tree.node(import).unwrap().range.offset,
        source.find("@import").unwrap() as u64
    );
    assert_eq!(elements(&tree, "declaration").len(), 4);
    assert!(
        elements(&tree, "declaration")
            .iter()
            .any(|id| attr(&tree, *id, "important").as_deref() == Some("true"))
    );
    let urls: Vec<_> = elements(&tree, "component-value")
        .into_iter()
        .filter(|id| attr(&tree, *id, "kind").as_deref() == Some("url"))
        .map(|id| attr(&tree, id, "value").unwrap())
        .collect();
    assert_eq!(urls, ["icon", "other"]);
    let native = tree
        .native_owner()
        .unwrap()
        .downcast_ref::<LoadedInputAstStream>()
        .unwrap();
    let LoadedInputAstStream::CssDocument(doc) = native else {
        panic!("CSS owner lost")
    };
    assert_eq!(
        doc.events
            .iter()
            .map(|e| e.lexeme.as_str())
            .collect::<String>(),
        source
    );
    let retained = try_retain_lifecycle(Arc::new(native.clone()))
        .unwrap()
        .unwrap();
    assert_eq!(elements(&retained, "declaration").len(), 4);
}

#[test]
fn css_import_style_block_empty_escapes_nested_rules_and_recovery() {
    let source = "@import 'theme'; @media screen { /* nested */ :scope { background: url(\"a\\20 b.svg\"); } }";
    let tree = import_data_bytes(
        source.as_bytes(),
        "text/css; mode=scoped-style-block",
        "cem",
        "inline.css",
    )
    .unwrap();
    assert_eq!(elements(&tree, "style-block").len(), 1);
    assert_eq!(elements(&tree, "import").len(), 1);
    assert_eq!(elements(&tree, "rule").len(), 2);
    assert_eq!(elements(&tree, "comment").len(), 1);
    let function = elements(&tree, "function")[0];
    assert_eq!(attr(&tree, function, "name").as_deref(), Some("url"));
    assert!(
        elements(&tree, "component-value")
            .iter()
            .any(|id| attr(&tree, *id, "value").as_deref() == Some("a b.svg"))
    );
    assert_eq!(
        elements(
            &import_data("", "text/css", "cem", "empty.css").unwrap(),
            "stylesheet"
        )
        .len(),
        1
    );
    assert!(import_data_bytes(&[0xff], "text/css", "cem", "bad.css").is_err());
    // Parser recovery does not make a hard syntax diagnostic importable.
    assert!(import_data("a { color: red", "text/css", "cem", "recovery.css").is_err());
}

#[test]
fn css_import_bounds_and_resource_policy_facts_are_preserved() {
    use cem_ml::validation::css::{CssFactKind, validate_css_document_ast};
    let tree = import_data(
        "@import 'https://example.test/style.css';",
        "text/css",
        "cem",
        "policy.css",
    )
    .unwrap();
    let owner = tree
        .native_owner()
        .unwrap()
        .downcast_ref::<LoadedInputAstStream>()
        .unwrap();
    let LoadedInputAstStream::CssDocument(doc) = owner else {
        panic!("missing CSS owner")
    };
    assert!(
        doc.facts
            .iter()
            .any(|f| f.kind == CssFactKind::ImportRejected)
    );
    assert!(
        validate_css_document_ast(doc)
            .iter()
            .any(|d| d.code == "cem.css.import_rejected" && d.severity.is_hard_violation())
    );
    assert!(
        import_data(&"/**/".repeat(4097), "text/css", "cem", "large.css")
            .unwrap_err()
            .contains("4096-event")
    );
    let deep = format!("a {{ color: {}red{}; }}", "f(".repeat(66), ")".repeat(66));
    assert!(
        import_data(&deep, "text/css", "cem", "deep.css")
            .unwrap_err()
            .contains("64-level")
    );
}

#[test]
fn css_import_preserves_at_rule_components_and_declaration_list_mode() {
    let source = "@namespace svg url(https://example.test/svg); @unknown test { color: red; }";
    let tree = import_data(source, "text/css", "cem", "rules.css").unwrap();
    assert_eq!(elements(&tree, "at-rule").len(), 2);
    assert!(
        elements(&tree, "component-value")
            .iter()
            .any(|id| attr(&tree, *id, "value").as_deref() == Some("https://example.test/svg"))
    );
    assert_eq!(elements(&tree, "block").len(), 1);
    let tree = import_data(
        "color: red; --x: 'url(fake)';",
        "text/css; mode=style-attribute",
        "cem",
        "attr.css",
    )
    .unwrap();
    assert_eq!(elements(&tree, "style-attribute").len(), 1);
    assert_eq!(elements(&tree, "declaration").len(), 2);
    assert!(
        elements(&tree, "component-value")
            .iter()
            .all(|id| attr(&tree, *id, "kind").as_deref() != Some("url"))
    );
}

#[test]
fn css_import_layer_names_reject_invalid_grammar_at_the_import_boundary() {
    for name in [
        "",
        "/* empty */",
        "base.",
        ".base",
        "base..theme",
        "base theme",
        "base .theme",
        "base. theme",
        "base,theme",
        "123",
        "'base'",
        "foo()",
        "initial",
        "INHERIT",
        "unset",
        "revert",
        "revert-layer",
        "base.initial",
        r"\69 nitial",
        "base{color:red}",
    ] {
        let css = format!("@import 'theme.css' layer({name});");
        let error = import_data(&css, "text/css", "cem", "layer.css")
            .err()
            .unwrap_or_else(|| panic!("accepted invalid layer: {name}"));
        assert!(error.contains("layer"), "{name}: {error}");
    }
}

#[test]
fn css_import_layer_names_retain_decoded_segments_and_anonymous_clauses() {
    for (clause, expected) in [
        ("layer", vec![]),
        ("LaYeR(default)", vec!["default"]),
        ("layer( base.theme )", vec!["base", "theme"]),
        ("layer(base/**/./**/theme)", vec!["base", "theme"]),
        (r"layer(base\.theme)", vec!["base.theme"]),
        (r"l\61 yer(\62 ase.theme)", vec!["base", "theme"]),
        ("layer(主题)", vec!["主题"]),
    ] {
        let css = format!("@import 'theme.css' {clause} supports(display:grid) screen;");
        let tree = import_data(&css, "text/css", "cem", "layer.css").unwrap();
        let layer = elements(&tree, "import-layer");
        assert_eq!(layer.len(), 1, "{clause}");
        let layer = layer[0];
        assert_eq!(
            attr(&tree, layer, "anonymous").as_deref(),
            Some(if expected.is_empty() { "true" } else { "false" })
        );
        let segments: Vec<_> = tree
            .node(layer)
            .unwrap()
            .children
            .iter()
            .map(|id| attr(&tree, *id, "value").unwrap())
            .collect();
        assert_eq!(segments, expected, "{clause}");
        assert_eq!(
            tree.node(layer).unwrap().range.offset,
            css.find(clause).unwrap() as u64
        );
        let import = elements(&tree, "import")[0];
        assert_eq!(
            attr(&tree, import, "supports").as_deref(),
            Some("display:grid")
        );
        assert_eq!(attr(&tree, import, "media").as_deref(), Some("screen"));
    }
    let tree = import_data("@import 'theme.css';", "text/css", "cem", "layer.css").unwrap();
    assert!(elements(&tree, "import-layer").is_empty());
}

#[test]
fn css_import_supports_rejects_invalid_outer_grammar() {
    for condition in [
        "",
        "/* empty */",
        "display",
        "display grid",
        ":grid",
        "not",
        "not not (display:grid)",
        "(display:grid) and",
        "(display:grid) (color:red)",
        "(display:grid) and (color:red) or (color:blue)",
        "not (display:grid) and (color:red)",
        "(display:grid), (color:red)",
        "display:grid; color:red",
        "display:grid !bad",
        "display:grid !important extra",
        "background:url(bad url)",
    ] {
        let css = format!("@import 'theme.css' supports({condition});");
        let error = import_data(&css, "text/css", "cem", "supports.css")
            .err()
            .unwrap_or_else(|| panic!("accepted invalid supports: {condition}"));
        assert!(error.contains("supports"), "{condition}: {error}");
    }
}

#[test]
fn css_import_supports_preserves_form_tokens_and_future_compatible_queries() {
    for (condition, form) in [
        ("display:grid", "declaration"),
        ("display:", "declaration"),
        ("--custom: { nested: value; }", "declaration"),
        ("color: red ! IMPORTANT", "declaration"),
        (r"d\69 splay: grid", "declaration"),
        ("background: url(icon.svg)", "declaration"),
        ("(display:grid)", "condition"),
        ("not (display:grid)", "condition"),
        (r"n\6ft/**/(display:grid)", "condition"),
        ("(display:grid) AND (color:red)", "condition"),
        (
            "(display:grid) or ((color:red) and (color:blue))",
            "condition",
        ),
        ("selector(:has(> .item))", "condition"),
        ("future-feature()", "condition"),
        ("()", "condition"),
        ("(future syntax; ! foo)", "condition"),
        // General-enclosed keeps inner unknown syntax; it must not be simplified.
        (
            "((display:grid) and (color:red) or (color:blue))",
            "condition",
        ),
    ] {
        let css = format!("@import 'theme.css' layer(base) supports({condition}) screen;");
        let tree = import_data(&css, "text/css", "cem", "supports.css").unwrap();
        let nodes = elements(&tree, "import-supports");
        assert_eq!(nodes.len(), 1, "{condition}");
        let node = nodes[0];
        assert_eq!(attr(&tree, node, "condition-form").as_deref(), Some(form));
        let retained: String = tree
            .node(node)
            .unwrap()
            .children
            .iter()
            .map(|id| attr(&tree, *id, "token").unwrap())
            .collect();
        assert_eq!(retained, condition);
        let range = tree.node(node).unwrap().range;
        assert_eq!(
            &css[range.offset as usize..(range.offset + range.length) as usize],
            format!("supports({condition})")
        );
        let import = elements(&tree, "import")[0];
        assert_eq!(attr(&tree, import, "supports").as_deref(), Some(condition));
        assert_eq!(attr(&tree, import, "media").as_deref(), Some("screen"));
    }
}

#[test]
fn css_import_media_checks_outer_grammar_without_evaluating_features() {
    for (query, valid) in [
        ("screen", true),
        ("not print", true),
        ("ONLY screen", true),
        ("screen and (color)", true),
        ("screen and not (color)", true),
        ("(400px < width <= 1000px)", true),
        ("not (hover)", true),
        ("(color) or (hover)", true),
        ("screen and ((color) or (hover))", true),
        (r"s\63 reen a\6e d (color)", true),
        ("unknown-medium", true),
        ("not unknown-medium", true),
        ("future-feature(a, b)", true),
        ("(future syntax; !)", true),
        ("()", true),
        ("screen or (color)", false),
        ("screen and (color) or (hover)", false),
        ("(color) and (hover) or (grid)", false),
        ("not (color) and (hover)", false),
        ("only", false),
        ("not", false),
        ("and", false),
        ("or", false),
        ("layer", false),
        ("only (color)", false),
        ("not only screen", false),
        ("screen and", false),
        ("screen print", false),
        ("&bad", false),
        ("(background:url(bad url))", false),
    ] {
        let css = format!("@import 'theme.css' supports(display:grid) {query};");
        let tree = import_data(&css, "text/css", "cem", "media.css").unwrap();
        let nodes = elements(&tree, "media-query");
        assert_eq!(nodes.len(), 1, "{query}");
        assert_eq!(
            attr(&tree, nodes[0], "syntax-valid").as_deref(),
            Some(if valid { "true" } else { "false" }),
            "{query}"
        );
        let retained: String = tree
            .node(nodes[0])
            .unwrap()
            .children
            .iter()
            .map(|id| attr(&tree, *id, "token").unwrap())
            .collect();
        assert_eq!(retained, query);
        let range = tree.node(nodes[0]).unwrap().range;
        assert_eq!(
            &css[range.offset as usize..(range.offset + range.length) as usize],
            query
        );
    }
}

#[test]
fn css_import_media_recovers_each_list_entry_and_preserves_nested_commas() {
    let css = "@import 'theme.css' screen, &bad, future(a,b),, print,";
    let tree = import_data(css, "text/css", "cem", "media.css").unwrap();
    assert_eq!(elements(&tree, "import-media").len(), 1);
    let queries = elements(&tree, "media-query");
    let status: Vec<_> = queries
        .iter()
        .map(|id| attr(&tree, *id, "syntax-valid").unwrap())
        .collect();
    assert_eq!(status, ["true", "false", "true", "false", "true", "false"]);
    let last = tree.node(*queries.last().unwrap()).unwrap().range;
    assert_eq!(last.offset, css.len() as u64);
    assert_eq!(last.length, 0);
    let tree = import_data(
        "@import 'theme.css' /* no media */;",
        "text/css",
        "cem",
        "media.css",
    )
    .unwrap();
    assert!(elements(&tree, "import-media").is_empty());
    assert!(elements(&tree, "media-query").is_empty());
}

#[test]
fn css_import_retains_selector_structure_host_arguments_and_specificity() {
    let css = "/* lead */\n:host(.active) > button.primary[data-state=\"on\" i]:is(:hover, :focus-visible), :where(#ignored) .child {color:red}";
    let tree = import_data(css, "text/css", "cem", "selectors.css").unwrap();
    let rule = elements(&tree, "rule")[0];
    let list = tree.node(rule).unwrap().children[0];
    assert_eq!(
        attr(&tree, list, "analysis-status").as_deref(),
        Some("complete")
    );
    let selectors = &tree.node(list).unwrap().children;
    assert_eq!(selectors.len(), 2);
    assert_eq!(
        attr(&tree, selectors[0], "specificity").as_deref(),
        Some("0-5-1")
    );
    assert_eq!(
        attr(&tree, selectors[1], "specificity").as_deref(),
        Some("0-1-0")
    );
    let host = elements(&tree, "simple-selector")
        .into_iter()
        .find(|id| attr(&tree, *id, "name").as_deref() == Some("host"))
        .unwrap();
    assert_eq!(attr(&tree, host, "kind").as_deref(), Some("pseudo-class"));
    assert_eq!(tree.node(host).unwrap().children.len(), 1);
    let range = tree.node(host).unwrap().range;
    assert_eq!(range.line, 2);
    assert_eq!(
        &css[range.offset as usize..(range.offset + range.length) as usize],
        ":host(.active)"
    );
    assert!(elements(&tree, "combinator")
        .iter()
        .any(|id| attr(&tree, *id, "kind").as_deref() == Some("child")));
    let attribute = elements(&tree, "simple-selector")
        .into_iter()
        .find(|id| attr(&tree, *id, "kind").as_deref() == Some("attribute"))
        .unwrap();
    assert_eq!(
        attr(&tree, attribute, "name").as_deref(),
        Some("data-state")
    );
    assert_eq!(attr(&tree, attribute, "value").as_deref(), Some("on"));
    assert_eq!(attr(&tree, attribute, "modifier").as_deref(), Some("i"));
}

#[test]
fn css_import_selector_analysis_keeps_unsupported_forms_without_false_specificity() {
    for selector in [
        "::part(control)",
        ":nth-child(2n of .item)",
        "& .child",
        ":host(.a,.b)",
        ":host(.a > .b)",
    ] {
        let css = format!("{selector} {{color:red}}");
        let tree = import_data(&css, "text/css", "cem", "selectors.css").unwrap();
        let rule = elements(&tree, "rule")[0];
        assert_eq!(attr(&tree, rule, "selector").as_deref(), Some(selector));
        let list = tree.node(rule).unwrap().children[0];
        assert_eq!(
            attr(&tree, list, "analysis-status").as_deref(),
            Some("unsupported"),
            "{selector}"
        );
        assert!(tree.node(list).unwrap().children.is_empty());
    }
}

#[test]
fn css_import_selector_structure_preserves_duplicates_escapes_and_relative_has() {
    let css = r".a/**/.\61 [part=x][part=x]:has(> .child) { color:red }";
    let tree = import_data(css, "text/css", "cem", "selectors.css").unwrap();
    let classes: Vec<_> = elements(&tree, "simple-selector")
        .into_iter()
        .filter(|id| attr(&tree, *id, "kind").as_deref() == Some("class"))
        .map(|id| attr(&tree, id, "value").unwrap())
        .collect();
    assert_eq!(classes, ["a", "a", "child"]);
    let compounds = elements(&tree, "compound-selector");
    assert_eq!(compounds.len(), 2);
    assert_eq!(tree.node(compounds[0]).unwrap().children.len(), 5);
    let child = elements(&tree, "combinator")
        .into_iter()
        .find(|id| attr(&tree, *id, "kind").as_deref() == Some("child"))
        .unwrap();
    let range = tree.node(child).unwrap().range;
    assert_eq!(
        css[range.offset as usize..(range.offset + range.length) as usize].trim(),
        ">"
    );

    assert_eq!(
        elements(&tree, "simple-selector")
            .iter()
            .filter(|id| attr(&tree, **id, "kind").as_deref() == Some("attribute"))
            .count(),
        2
    );
    assert!(elements(&tree, "combinator")
        .iter()
        .any(|id| attr(&tree, *id, "kind").as_deref() == Some("child")));
}

#[test]
fn css_import_retains_terminal_pseudo_elements_with_type_specificity() {
    for (selector, name, weight) in [
        (".item::before", "before", "0-1-1"),
        ("button:AFTER", "after", "0-0-2"),
        (r"::\62 efore", "before", "0-0-1"),
        (":first-line", "first-line", "0-0-1"),
        (":first-letter", "first-letter", "0-0-1"),
        (":WHERE(.a.b.c)::MARKER", "marker", "0-0-1"),
    ] {
        let css = format!("\n{selector} {{color:red}}");
        let tree = import_data(&css, "text/css", "cem", "pseudo.css").unwrap();
        let rule = elements(&tree, "rule")[0];
        let list = tree.node(rule).unwrap().children[0];
        assert_eq!(
            attr(&tree, list, "analysis-status").as_deref(),
            Some("complete"),
            "{selector}"
        );
        let selected = tree.node(list).unwrap().children[0];
        assert_eq!(
            attr(&tree, selected, "specificity").as_deref(),
            Some(weight)
        );
        let pseudo = elements(&tree, "simple-selector")
            .into_iter()
            .find(|id| attr(&tree, *id, "kind").as_deref() == Some("pseudo-element"))
            .unwrap();
        assert_eq!(attr(&tree, pseudo, "name").as_deref(), Some(name));
        let range = tree.node(pseudo).unwrap().range;
        assert_eq!(range.line, 2);
        assert!(
            css[range.offset as usize..(range.offset + range.length) as usize].starts_with(':')
        );
        assert_eq!(range.offset + range.length, 1 + selector.len() as u64);
    }
}

#[test]
fn css_import_marks_unimplemented_pseudo_element_contexts_unsupported() {
    for selector in [
        "::part(control)",
        "::before::marker",
        "::before:hover",
        "::before > .child",
        ":is(::before, .ok)",
        ":where(::after)",
        ":not(::before)",
        ":has(::before)",
        ":host(::before)",
        "::before.active",
    ] {
        let tree =
            import_data(&format!("{selector} {{}}"), "text/css", "cem", "pseudo.css").unwrap();
        let rule = elements(&tree, "rule")[0];
        let list = tree.node(rule).unwrap().children[0];
        assert_eq!(
            attr(&tree, list, "analysis-status").as_deref(),
            Some("unsupported"),
            "{selector}"
        );
        assert!(tree.node(list).unwrap().children.is_empty());
    }
}
