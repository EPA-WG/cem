use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{scoping::SchemaSource, vocab::CompiledSchema},
    source_map::TransformKind,
};
fn parse(text: &str, mime: &str) -> Result<ScopedCemImport, String> {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        mime,
        "memory:syntax",
        CompiledSchema::cem_core(),
    )
}
#[test]
fn block_preludes_are_own_line_opening_controls_and_restore_at_exit() {
    let source = "@schema outer\n{root |\n /* comment */\n @schema inner\n @ns p = urn:inner\n {#a}\n @schema literal-after-body\n}\n{#b}";
    let imported = parse(source, "text/cem-ml").unwrap();
    let owner = imported.captured.document();
    let snapshots = imported
        .captured
        .occurrences()
        .map(|id| imported.captured.snapshot(owner, id).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(snapshots.len(), 2);
    assert_eq!(
        snapshots[0].schema.active,
        SchemaSource::Uri("inner".into())
    );
    assert_eq!(
        snapshots[1].schema.active,
        SchemaSource::Uri("outer".into())
    );
    assert_eq!(
        snapshots[0].namespaces.binding("p").unwrap().namespace_uri,
        "urn:inner"
    );
    assert!(snapshots[1].namespaces.binding("p").is_none());
    assert!(owner.nodes.iter().any(
        |n| matches!(n,CemAstNode::Text{data,..} if data.contains("@schema literal-after-body"))
    ));
}
#[test]
fn block_literal_compatibility_is_bounded() {
    for content in [
        "@schema same-line",
        "\n \\@schema escaped",
        "\n @unknown literal",
        "\n text\n @schema late",
        "\n <?note?>\n @schema late",
    ] {
        let imported = parse(&format!("{{root |{content}\n}}"), "text/cem-ml").unwrap();
        assert!(!imported.captured.document().nodes.iter().any(|n| matches!(n,CemAstNode::Element{expanded_name,..} if expanded_name.local_name.starts_with("@"))));
    }
}
#[test]
fn xml_explicit_slots_keep_native_owners_and_unlisted_braces_literal() {
    let source = "<item xmlns:c='https://cem.dev/ns/core/1' c:expression-attributes='target general' target='{#nodes}' general='{1 &lt; 2}' literal='{#nodes}' xmlns:p='urn:later'/>";
    let imported = parse(source, "application/xml").unwrap();
    let owner = imported.captured.document();
    let mut native = vec![];
    for node in &owner.nodes {
        if let CemAstNode::Attribute {
            expanded_name,
            value,
            value_nodes,
            ..
        } = node
        {
            match expanded_name.local_name.as_str() {
                "target" | "general" => {
                    assert!(value.is_none());
                    assert_eq!(value_nodes.len(), 1);
                    native.push(value_nodes[0]);
                }
                "literal" => {
                    assert_eq!(value.as_deref(), Some("{#nodes}"));
                    assert!(value_nodes.is_empty());
                }
                _ => {}
            }
        }
    }
    assert_eq!(native.len(), 2);
    for id in native {
        let scope = imported
            .captured
            .snapshot(owner, id)
            .expect("native slot captured after complete header");
        assert_eq!(
            scope.namespaces.binding("p").unwrap().namespace_uri,
            "urn:later"
        );
    }
    assert!(owner.nodes.iter().any(
        |n| matches!(n,CemAstNode::Reference{expression,targets:None,..} if expression=="#nodes")
    ));
    let expr = owner
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Text { data, source, .. } if data == "1 < 2" => Some(source),
            _ => None,
        })
        .unwrap();
    assert!(expr
        .frames
        .iter()
        .any(|f| matches!(f.transform, TransformKind::ExpressionEmbedding { .. })));
    assert!(expr.frames.iter().any(|f| match (&f.transform, &f.span) {
        (TransformKind::ExpressionEmbedding { .. }, cem_ml::source_map::FrameSpan::Single(r)) =>
            &source[r.start as usize..r.end() as usize] == "&lt;",
        _ => false,
    }));
}
#[test]
fn xml_marker_is_exact_expanded_control_and_element_local() {
    for source in ["<root xmlns:c='urn:foreign' c:expression-attributes='target' target='{#nodes}'/>","<root xmlns:c='https://cem.dev/ns/core/1' c:expression-attributes='target' target='{#nodes}'><child target='{#nodes}'/></root>"] {
        let imported = parse(source,"application/xml").unwrap();
        let refs = imported.captured.document().nodes.iter().filter(|n| matches!(n,CemAstNode::Reference{..})).count();
        assert_eq!(refs, usize::from(source.contains("https://cem.dev/ns/core/1")));
    }
}
#[test]
fn xml_invalid_explicit_slots_fail_at_import_without_partial_activation() {
    for attrs in [
        "c:expression-attributes='missing'",
        "c:expression-attributes='target target' target='{#nodes}'",
        "c:expression-attributes='target' target='literal'",
        "c:expression-attributes='xmlns:p' xmlns:p='{#nodes}'",
        "c:expression-attributes='xml:lang' xml:lang='{#nodes}'",
        "c:expression-attributes='c:expression-attributes'",
        "c:expression-attributes='target' target='{#a}{#b}'",
    ] {
        assert!(
            parse(
                &format!("<item xmlns:c='https://cem.dev/ns/core/1' {attrs}/>"),
                "application/xml"
            )
            .is_err(),
            "{attrs}"
        );
    }
}

#[test]
fn xml_slot_braces_inside_quotes_and_nested_comments_do_not_end_the_slot() {
    for slot in [
        "{#nodes (: } (: { :) :)}",
        "{'a}b'}",
        "{'a''}b'}",
        "{1 /* } */ + 2}",
        "{#nodes // }&#10;}",
    ] {
        let source = format!("<item xmlns:c='https://cem.dev/ns/core/1' c:expression-attributes='target' target=\"{slot}\"/>");
        let imported = parse(&source, "application/xml").unwrap();
        assert_eq!(imported.captured.occurrences().count(), 1, "{slot}");
    }
}
#[test]
fn nested_block_namespace_preludes_shadow_and_restore_original_capture() {
    let source = "@ns p = urn:outer\n{root |\n @ns p = urn:inner\n {nested |\n @ns p = urn:nested\n {#a}\n }\n {#b}\n}\n{#c}";
    let imported = parse(source, "text/cem-ml").unwrap();
    let owner = imported.captured.document();
    let names = imported
        .captured
        .occurrences()
        .map(|id| {
            imported
                .captured
                .snapshot(owner, id)
                .unwrap()
                .namespaces
                .binding("p")
                .unwrap()
                .namespace_uri
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["urn:nested", "urn:inner", "urn:outer"]);
}

#[test]
fn block_prelude_line_endings_keep_directive_payloads_and_body_separate() {
    for line_ending in ["\n", "\r\n", "\r"] {
        let source =
            "@schema outer\n{root |\n @schema inner\n {#a}\n}\n{#b}".replace('\n', line_ending);
        let imported = parse(&source, "text/cem-ml").unwrap();
        let owner = imported.captured.document();
        let scopes = imported
            .captured
            .occurrences()
            .map(|id| {
                imported
                    .captured
                    .snapshot(owner, id)
                    .unwrap()
                    .schema
                    .active
                    .clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            scopes,
            vec![
                SchemaSource::Uri("inner".into()),
                SchemaSource::Uri("outer".into())
            ]
        );
    }
}
