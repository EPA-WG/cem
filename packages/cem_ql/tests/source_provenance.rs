//! SOURCE-PROVENANCE-QUERY: the same retained source metadata in both languages.
use cem_ml::{
    import::import_data,
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::XPathNativeNode,
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    xpath::functions::{CemtXPathFunctions, XPathQueryItem},
};
use std::{collections::BTreeMap, sync::Arc};

const LIBRARY: &str = r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module |
    {function @name=source.metadata @visibility=public @returns=any |
        {param @name=node @type=any @required=true}
        {body | {xpath @context=node @sequence-type="item()*" |
            {expression |``` (Q{urn:cem:source}node-key(.), Q{urn:cem:source}line-number(.), .) ```}
    }   }   }
}"#;

fn context(row: Item) -> EvaluationContext {
    let mut context = EvaluationContext {
        policy_bindings: BTreeMap::from([("row".into(), ItemStream::once(row))]),
        ..Default::default()
    };
    CemtXPathFunctions::compile(LIBRARY, "memory:source.cemt")
        .unwrap()
        .install(
            &mut context.native_functions,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
    context
}
fn run(source: &str, context: &EvaluationContext) -> ItemStream {
    let query = compile(
        source,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    evaluate(&query, context)
}

#[test]
fn both_query_languages_read_the_same_native_metadata_and_keep_owners() {
    for (format, source) in [
        ("xml", "<r><row>A</row><row>B</row></r>"),
        ("json", "[{\"label\":\"A\"},{\"label\":\"B\"}]"),
        ("yaml", "- label: A\n- label: B"),
        ("csv", "label\nA\nB"),
    ] {
        let tree = import_data(source, format, "cem", "memory:rows").unwrap();
        let native =
            XPathNativeNode::cem_document(tree.clone()).child_nodes()[0].child_nodes()[0].clone();
        let root = imported_cem_tree(tree.clone());
        let outer = root.view().unwrap().field("children").unwrap().remove(0);
        let generic = outer.view().unwrap().field("children").unwrap().remove(0);
        for row in [generic, XPathQueryItem::from_node(native.clone())] {
            let context = context(row);
            let result = run(
                r#"(data:node_key(row), data:line_number(row), native:call("source.metadata", row))"#,
                &context,
            );
            assert!(result.error.is_none(), "{format}: {:?}", result.diagnostics);
            assert_eq!(result.items.len(), 5);
            assert_eq!(result.items[0].atom(), result.items[2].atom());
            assert_eq!(result.items[1].atom(), result.items[3].atom());
            assert_eq!(
                result.items[0].atom(),
                Some(AtomValue::String(native.source_key().unwrap()))
            );
            assert_eq!(
                result.items[1].atom(),
                Some(AtomValue::Integer(
                    native.source_line_number().unwrap().into()
                ))
            );
            let retained = result.items[4]
                .view()
                .unwrap()
                .downcast_ref::<XPathQueryItem>()
                .unwrap()
                .xpath_item()
                .native_node()
                .unwrap();
            assert!(Arc::ptr_eq(retained.owner(), &tree));
            assert_eq!(retained.source_map(), native.source_map());
        }
    }
}

#[test]
fn cemt_metadata_functions_require_optional_native_nodes() {
    let tree = import_data("<r/>", "xml", "cem", "memory:row").unwrap();
    let context = context(imported_cem_tree(tree));
    for name in ["node_key", "line_number"] {
        let empty = run(&format!("data:{name}(())"), &context);
        assert!(empty.error.is_none());
        assert!(empty.items.is_empty());
        for argument in ["1", "\"text\"", "{kind: \"element\"}", "(row, row)"] {
            let failure = run(&format!("data:{name}({argument})"), &context);
            assert!(failure.error.is_some(), "{name}({argument})");
            assert!(
                failure
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "cem.ql.type_error"),
                "{:?}",
                failure.diagnostics
            );
        }
    }
}
