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
