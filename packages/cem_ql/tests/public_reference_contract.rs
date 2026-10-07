use cem_ml::{
    import::import_bytes_with_lexical_scopes,
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::vocab::CompiledSchema,
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{imported_cem_tree, AtomValue, ItemStream},
};

#[test]
fn source_target_availability_is_inert_and_distinct_from_empty() {
    for targets in [None, Some(vec![]), Some(vec![1, 1])] {
        let parsed = import_bytes_with_lexical_scopes(
            b"{#input}",
            "text/cem-ml",
            "source.cem",
            CompiledSchema::cem_core(),
        )
        .unwrap();
        let bytes = cem_ml::ast::DebugBinaryEncoder::new()
            .encode(parsed.tree.ast())
            .bytes;
        let mut ast = cem_ml::ast::DebugBinaryDecoder::new()
            .decode(&bytes)
            .unwrap();
        if let CemAstNode::Reference {
            targets: stored, ..
        } = &mut ast.nodes[1]
        {
            *stored = targets.clone();
        } else {
            panic!()
        }
        let tree = RetainedCemTree::new(
            ast,
            "source.cem",
            "{#input}",
            CemTreeSemantics::default(),
            None,
        )
        .unwrap();
        let item = imported_cem_tree(tree.clone())
            .view()
            .unwrap()
            .field("children")
            .unwrap()
            .remove(0);
        let mut context = EvaluationContext::default();
        context
            .policy_bindings
            .insert("r".into(), ItemStream::once(item.clone()));
        let compiler = CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        };
        for (query, expected) in [
            ("r.targets_available", targets.is_some()),
            ("r is reference", true),
            ("r is node", true),
        ] {
            let result = evaluate(&compile(query, &compiler).unwrap(), &context);
            assert!(result.error.is_none(), "{result:?}");
            assert_eq!(result.items[0].atom(), Some(AtomValue::Boolean(expected)));
        }
        let selected = evaluate(&compile("r.targets", &compiler).unwrap(), &context);
        assert_eq!(selected.items.len(), targets.as_ref().map_or(0, Vec::len));
        for target in selected.items {
            assert_eq!(target.identity(), item.identity());
        }
        assert_eq!(
            item.view().unwrap().field("expression").unwrap()[0].atom(),
            Some(AtomValue::String("#input".into()))
        );
    }
}
