use cem_ml::{
    css_emission::{emit_css_import_conditions, CssImportConditionEmission},
    import::import_data,
    parser::tree::RetainedCemTree,
};
fn import_id(tree: &RetainedCemTree) -> u32 {
    (0..tree.ast().nodes.len() as u32)
        .find(|id| {
            tree.node(*id)
                .unwrap()
                .name
                .as_ref()
                .is_some_and(|n| n.local_name == "import")
        })
        .unwrap()
}
#[test]
fn import_emission_parenthesizes_declarations_and_preserves_conditions() {
    for (condition, expected) in [
        ("display:grid", "(display:grid)"),
        ("font-family:foo\\ ", "(font-family:foo\\ )"),
        ("not (display:grid)", "not (display:grid)"),
        ("selector(:has(> .item))", "selector(:has(> .item))"),
        ("(future syntax; !)", "(future syntax; !)"),
        ("(color:red) or (color:blue)", "(color:red) or (color:blue)"),
    ] {
        let css = format!("@import 'theme' supports({condition}) screen and (color), print;");
        let tree = import_data(&css, "text/css", "cem", "emission.css").unwrap();
        let CssImportConditionEmission::Emit {
            wrappers,
            diagnostics,
        } = emit_css_import_conditions(&tree, import_id(&tree)).unwrap()
        else {
            panic!("suppressed")
        };
        assert!(diagnostics.is_empty());
        assert_eq!(wrappers.len(), 2);
        assert_eq!(wrappers[0].opening, format!("@supports {expected} {{"));
        assert_eq!(wrappers[1].opening, "@media screen and (color), print {");
        let range = wrappers[0].range;
        assert_eq!(
            &css[range.offset as usize..(range.offset + range.length) as usize],
            format!("supports({condition})")
        );
        assert!(!wrappers.iter().any(|w| w.opening.contains("@import")));
    }
    let tree = import_data("@import 'theme';", "text/css", "cem", "emission.css").unwrap();
    let CssImportConditionEmission::Emit {
        wrappers,
        diagnostics,
    } = emit_css_import_conditions(&tree, import_id(&tree)).unwrap()
    else {
        panic!("suppressed")
    };
    assert!(wrappers.is_empty() && diagnostics.is_empty());
}
#[test]
fn import_emission_recovers_media_entries_with_source_diagnostics() {
    let css = "@import 'theme' screen, &bad, future(a,b),, print;";
    let tree = import_data(css, "text/css", "cem", "emission.css").unwrap();
    let CssImportConditionEmission::Emit {
        wrappers,
        diagnostics,
    } = emit_css_import_conditions(&tree, import_id(&tree)).unwrap()
    else {
        panic!("suppressed")
    };
    assert_eq!(wrappers.len(), 1);
    assert_eq!(
        wrappers[0].opening,
        "@media screen, not all, future(a,b), not all, print {"
    );
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics
        .iter()
        .all(|d| d.code == "cem.scoped_css.media_query_recovered"));
    let range = diagnostics[0].range;
    assert_eq!(
        css[range.offset as usize..(range.offset + range.length) as usize].trim(),
        "&bad"
    );
    assert_eq!(diagnostics[1].range.length, 0);
}
#[test]
fn import_emission_suppresses_named_and_anonymous_layers() {
    for layer in ["layer", "layer(base)", r"layer(base\.theme)"] {
        let css = format!("@import 'theme' {layer} supports(display:grid) screen;");
        let tree = import_data(&css, "text/css", "cem", "emission.css").unwrap();
        let CssImportConditionEmission::Suppress(diagnostic) =
            emit_css_import_conditions(&tree, import_id(&tree)).unwrap()
        else {
            panic!("layer emitted")
        };
        assert_eq!(diagnostic.code, "cem.scoped_css.layer_unsupported");
        assert_eq!(diagnostic.range.offset, css.find(layer).unwrap() as u64);
    }
}
#[test]
fn import_emission_rejects_missing_typed_conditions_and_wrong_nodes() {
    for attrs in ["supports='display:grid'", "media='screen'"] {
        let xml = format!("<import xmlns='https://cem.dev/ns/data/css/1' href='theme' {attrs}/>");
        let tree = import_data(&xml, "application/xml", "cem", "legacy.xml").unwrap();
        assert_eq!(
            emit_css_import_conditions(&tree, import_id(&tree))
                .unwrap_err()
                .code,
            "cem.scoped_css.condition_tree_invalid"
        );
    }
    let tree = import_data("a {}", "text/css", "cem", "emission.css").unwrap();
    for id in [0, u32::MAX] {
        assert_eq!(
            emit_css_import_conditions(&tree, id).unwrap_err().code,
            "cem.scoped_css.condition_tree_invalid"
        );
    }
}
