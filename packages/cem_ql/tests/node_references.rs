use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{AtomValue, ItemStream},
};

fn eval(source: &str) -> ItemStream {
    let query = compile(source, &CompileContext::default()).unwrap();
    evaluate(&query, &EvaluationContext::default())
}

#[test]
fn reference_operator_preserves_order_empty_sequences_and_attributes() {
    for (source, count) in [
        (
            r#"let doc = data:read("<root a='1'><x/><y/></root>", "xml"); #doc.root.children.children"#,
            2,
        ),
        (
            r#"let doc = data:read("<root a='1'/>", "xml"); #doc.root.children.attributes"#,
            1,
        ),
        ("#()", 0),
    ] {
        let result = eval(source);
        assert!(result.error.is_none(), "{:?}", result.diagnostics);
        assert_eq!(result.items.len(), 1);
        let view = result.items[0].view().unwrap();
        assert_eq!(
            view.field("kind").unwrap()[0].atom(),
            Some(AtomValue::String("reference".into()))
        );
        assert_eq!(view.field("targets").unwrap().len(), count);
    }
}

#[test]
fn existing_reference_is_a_target_without_implicit_dereferencing() {
    let result = eval("##()");
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    let inner = result.items[0].view().unwrap().field("targets").unwrap();
    assert_eq!(inner.len(), 1);
    assert_eq!(inner[0].view().unwrap().field("targets").unwrap().len(), 0);
    assert_ne!(
        result.items[0].view().unwrap().identity(),
        inner[0].view().unwrap().identity()
    );
}

#[test]
fn scalar_operands_are_rejected_without_id_or_expression_lookup() {
    for source in [
        r#"#"target""#,
        r#"#"https://example.test/doc#part""#,
        r##"#"#nodes""##,
        "#42",
        "#1.5",
        "#true",
    ] {
        assert!(
            compile(source, &CompileContext::default()).is_err(),
            "{source}"
        );
    }
    let query = compile(
        r#"#data:read("<r/>", "xml").error"#,
        &CompileContext::default(),
    )
    .unwrap();
    let result = evaluate(&query, &EvaluationContext::default());
    assert!(result.error.is_some());
    assert!(result.items.is_empty());
}

#[test]
fn unary_reference_precedence_matches_the_schema_contract() {
    use cem_ql::parser::{Expression, SurfaceNode, UnaryOp};
    for source in ["#nodes.children", "#nodes.nth(0)", "#f()", "#nodes is node"] {
        let parsed = cem_ql::api::parse(source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        assert!(
            matches!(
                &parsed.module.nodes[0],
                SurfaceNode::Expression(Expression::UnaryOp {
                    op: UnaryOp::Reference,
                    ..
                })
            ),
            "{source}: {:?}",
            parsed.module.nodes
        );
    }
    let parsed = cem_ql::api::parse("#nodes + other");
    assert!(matches!(
        &parsed.module.nodes[0],
        SurfaceNode::Expression(Expression::BinaryOp { .. })
    ));
    let result = eval(
        r#"let doc = data:read("<root><x/><y/></root>", "xml"); (#doc.root.children.children)"#,
    );
    let targets = result.items[0].view().unwrap().field("targets").unwrap();
    assert_eq!(
        targets
            .iter()
            .map(|item| item.view().unwrap().field("name").unwrap()[0].atom())
            .collect::<Vec<_>>(),
        vec![
            Some(AtomValue::String("x".into())),
            Some(AtomValue::String("y".into()))
        ]
    );
}

#[test]
fn repeated_and_mixed_targets_preserve_identity_and_document_ownership() {
    use cem_ql::api::{
        evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext,
    };
    // Separate executions create separate owners; repeated identical reads
    // inside one query may share a memoized import result.
    let a = eval(r#"data:read("<r/>", "xml").root"#);
    let b = eval(r#"data:read("<r/>", "xml").root"#);
    let context = StandaloneExpressionContext::default()
        .with_binding("a", StandaloneExpressionBinding::any(a))
        .with_binding("b", StandaloneExpressionBinding::any(b));
    let result = evaluate_expression("#(a, b, a, #a)", &context)
        .unwrap()
        .result;
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    let outer = result.items[0].view().unwrap();
    let targets = outer.field("targets").unwrap();
    assert_eq!(targets.len(), 4);
    assert_eq!(
        targets[0].view().unwrap().identity(),
        targets[2].view().unwrap().identity()
    );
    assert_ne!(
        targets[0].view().unwrap().identity(),
        targets[1].view().unwrap().identity()
    );
    assert_eq!(
        targets[0].view().unwrap().field("id").unwrap(),
        targets[1].view().unwrap().field("id").unwrap()
    );
    let inner = targets[3].view().unwrap();
    assert_ne!(inner.identity(), outer.identity());
    assert_eq!(
        inner.field("targets").unwrap()[0]
            .view()
            .unwrap()
            .identity(),
        targets[0].view().unwrap().identity()
    );
    assert!(outer
        .children(cem_ql::eval::QueryContextScope(0))
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn dynamically_supplied_scalars_are_rejected_without_lookup() {
    use cem_ql::{
        api::{evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::Item,
    };
    let node = eval(r#"data:read("<r/>", "xml").root"#).items.remove(0);
    for value in [
        AtomValue::String("#target".into()),
        AtomValue::String("https://example.test/data#part".into()),
        AtomValue::String("#nodes".into()),
        AtomValue::Boolean(true),
        AtomValue::Integer(42.into()),
        AtomValue::Null,
    ] {
        for operands in [
            vec![Item::Atomic(value.clone())],
            vec![node.clone(), Item::Atomic(value.clone())],
        ] {
            let context = StandaloneExpressionContext::default().with_binding(
                "operand",
                StandaloneExpressionBinding::any(ItemStream::from_items(operands)),
            );
            let result = evaluate_expression("#operand", &context).unwrap().result;
            assert!(result.error.is_some());
            assert!(result.items.is_empty());
        }
    }
}

#[test]
fn importable_ast_node_kinds_are_targets_without_following_retained_references() {
    use cem_ml::{
        parser::{
            document::CemDocument,
            tree::{CemTreeSemantics, RetainedCemTree},
            CemAstNode as N, ExpandedName,
        },
        source_map::SourceMapStack,
    };
    use cem_ql::{
        api::{evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::imported_cem_tree,
    };
    let source = SourceMapStack::default();
    let name = ExpandedName {
        namespace_uri: String::new(),
        local_name: "r".into(),
        schema_id: None,
    };
    let mut doc = CemDocument::default();
    doc.nodes = vec![
        N::Document {
            node_id: 0,
            root_children: vec![1],
            source: source.clone(),
        },
        N::Element {
            node_id: 1,
            expanded_name: name.clone(),
            attributes: vec![2],
            children: (3..12).collect(),
            has_explicit_boundary: true,
            source: source.clone(),
        },
        N::Attribute {
            node_id: 2,
            expanded_name: name,
            value: Some("a".into()),
            value_nodes: vec![],
            source: source.clone(),
        },
        N::Text {
            node_id: 3,
            data: "text".into(),
            source: source.clone(),
        },
        N::Whitespace {
            node_id: 4,
            data: " ".into(),
            source: source.clone(),
        },
        N::Comment {
            node_id: 5,
            data: "comment".into(),
            source: source.clone(),
        },
        N::ProcessingInstruction {
            node_id: 6,
            target: "pi".into(),
            data: "data".into(),
            source: source.clone(),
        },
        N::Cdata {
            node_id: 7,
            data: "data".into(),
            source: source.clone(),
        },
        N::RawText {
            node_id: 8,
            data: "raw".into(),
            source: source.clone(),
        },
        N::Text {
            node_id: 9,
            data: "other".into(),
            source: source.clone(),
        },
        N::Reference {
            node_id: 10,
            expression: "#notAvailable".into(),
            context: 1,
            targets: None,
            source: source.clone(),
        },
        N::Reference {
            node_id: 11,
            expression: "#()".into(),
            context: 1,
            targets: Some(vec![]),
            source,
        },
    ];
    let tree =
        RetainedCemTree::new(doc, "fixture.cem", "", CemTreeSemantics::default(), None).unwrap();
    let document = imported_cem_tree(tree.clone());
    let element = document
        .view()
        .unwrap()
        .field("children")
        .unwrap()
        .remove(0);
    let mut nodes = vec![document, element.clone()];
    nodes.extend(element.view().unwrap().field("attributes").unwrap());
    nodes.extend(element.view().unwrap().field("children").unwrap());
    assert_eq!(nodes.len(), 12);
    for node in nodes {
        let context = StandaloneExpressionContext::default().with_binding(
            "operand",
            StandaloneExpressionBinding::any(ItemStream::once(node.clone())),
        );
        let result = evaluate_expression("#operand", &context).unwrap().result;
        assert!(result.error.is_none(), "{:?}", result.diagnostics);
        let targets = result.items[0].view().unwrap().field("targets").unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0].view().unwrap().identity(),
            node.view().unwrap().identity()
        );
    }
    let children = element.view().unwrap().field("children").unwrap();
    let pending = children[7].view().unwrap();
    let empty = children[8].view().unwrap();
    assert_eq!(
        pending.field("kind").unwrap()[0].atom(),
        Some(AtomValue::String("reference".into()))
    );
    assert_eq!(
        pending.field("expression").unwrap()[0].atom(),
        Some(AtomValue::String("#notAvailable".into()))
    );
    assert_eq!(
        pending.field("context").unwrap()[0]
            .view()
            .unwrap()
            .identity(),
        element.view().unwrap().identity()
    );
    assert!(pending.field("targets").is_none());
    assert!(empty.field("targets").unwrap().is_empty());
    assert!(pending.field("children").unwrap().is_empty());
    assert!(matches!(
        tree.ast().get(10),
        Some(N::Reference { targets: None, .. })
    ));
}

#[test]
fn explicitly_supplied_error_node_is_a_target() {
    use cem_ml::parser::CemAstNode;
    use cem_ql::{
        api::{evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{Item, QueryItemView, QueryItemViewKind},
    };
    use std::any::Any;
    #[derive(Debug)]
    struct ErrorNode(CemAstNode);
    impl QueryItemView for ErrorNode {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.typed-error-node"
        }
        fn identity(&self) -> String {
            match self.0 {
                CemAstNode::Error { node_id, .. } => format!("{:p}:{node_id}", self),
                _ => unreachable!(),
            }
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Node
        }
    }
    let node = Item::native(ErrorNode(CemAstNode::Error {
        node_id: 0,
        code: "fixture".into(),
        source: Default::default(),
    }));
    let context = StandaloneExpressionContext::default().with_binding(
        "operand",
        StandaloneExpressionBinding::any(ItemStream::once(node.clone())),
    );
    let result = evaluate_expression("#operand", &context).unwrap().result;
    assert!(result.error.is_none(), "{:?}", result.diagnostics);
    let targets = result.items[0].view().unwrap().field("targets").unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(
        targets[0].view().unwrap().identity(),
        node.view().unwrap().identity()
    );
}
