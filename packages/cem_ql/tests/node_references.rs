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
    for source in [r#"#"target""#, "#42", "#true"] {
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
