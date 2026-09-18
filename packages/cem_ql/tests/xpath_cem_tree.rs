//! XPATH-CEM-TREE-PARITY: imported native trees share the XPath node boundary.
use cem_ml::resolver::{ResolverPolicy, ResolverRegistry};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{AtomValue, Item, ItemStream},
    xpath::functions::{CemtXPathFunctions, XPathQueryItem},
};
use std::{collections::BTreeMap, sync::Arc};

fn context(source: &str) -> EvaluationContext {
    let library = CemtXPathFunctions::compile(
        r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module |
    {function @name=tree.keep @visibility=public @returns=any |
        {param @name=document @type=any @required=true}
        {body | {xpath @context=document @sequence-type="node()" |
            {expression | .}
    }   }   }
    {function @name=tree.pick @visibility=public @returns=any |
        {param @name=document @type=any @required=true}
        {body | {xpath @context=document @sequence-type="node()*" |
            {expression | ```
/*/*
```}
    }   }   }
    {function @name=tree.text @visibility=public @returns=string |
        {param @name=node @type=any @required=true}
        {body | {xpath @context=node @sequence-type="xs:string" |
            {expression | string(.)}
}   }   }   }"#,
        "memory:tree.cemt",
    )
    .unwrap();
    let bytes = library.to_companion_bytes().unwrap();
    let library = CemtXPathFunctions::from_companion_bytes(
        &bytes,
        &cem_ml::content_cache::ContentHash::from_blake3(&bytes),
        library.source_hash(),
    )
    .unwrap();
    let mut context = EvaluationContext {
        policy_bindings: BTreeMap::from([(
            "source".into(),
            ItemStream::once(Item::Atomic(AtomValue::String(source.into()))),
        )]),
        ..Default::default()
    };
    library
        .install(
            &mut context.native_functions,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
    context
}

fn run(query: &str, context: &EvaluationContext) -> ItemStream {
    let compiled = compile(
        query,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let result = evaluate(&compiled, context);
    assert!(result.error.is_none(), "{query}: {result:?}");
    result
}

#[test]
fn imported_xml_and_both_json_trees_bind_to_the_same_reloaded_xpath_function() {
    for (source, format, projection, expected) in [
        ("<r><item>3</item><empty/></r>", "xml", "cem", vec!["3", ""]),
        (r#"{"qty":3,"note":null}"#, "json", "cem", vec!["3", ""]),
        ("qty: 3\nnote: null\n", "yaml", "cem", vec!["3", ""]),
        (
            r#"{"qty":3,"note":null}"#,
            "json",
            "json-to-xml",
            vec!["3", ""],
        ),
        ("qty,note\n3,hello", "csv", "cem", vec!["3hello"]),
    ] {
        let context = context(source);
        let result = run(
            &format!(
                r#"native:call("tree.pick", data:read(source, "{format}", "{projection}").root)"#
            ),
            &context,
        );
        let actual: Vec<_> = result
            .items
            .iter()
            .map(|item| {
                assert!(item
                    .view()
                    .unwrap()
                    .downcast_ref::<XPathQueryItem>()
                    .is_some());
                assert!(!item.source_map().unwrap().frames.is_empty());
                match item.atom().unwrap() {
                    AtomValue::String(value) => value,
                    other => panic!("{other:?}"),
                }
            })
            .collect();
        assert_eq!(actual, expected, "{format}/{projection}");
    }
}

#[test]
fn xml_semantic_text_and_json_carriage_returns_survive_import() {
    for (source, format, projection, expected) in [
        (
            "<r>a\r\nb<![CDATA[c\rd]]>&#13;</r>",
            "xml",
            "cem",
            "a\nbc\nd\r",
        ),
        (r#""a\rb""#, "json", "cem", "a\rb"),
        (r#""a\rb""#, "json", "json-to-xml", "a\rb"),
    ] {
        let result = run(
            &format!(
                r#"native:call("tree.text", data:read(source, "{format}", "{projection}").root)"#
            ),
            &context(source),
        );
        assert_eq!(
            result.items[0].atom(),
            Some(AtomValue::String(expected.into()))
        );
    }
}

#[test]
fn all_imports_share_bounded_retention_without_aliasing_views_or_contexts() {
    let mut context = context(r#"{"qty":3}"#);
    let query = r#"native:call("tree.keep", data:read(source, "json").root)"#;
    let first = run(query, &context);
    let native = |item: &Item| {
        item.view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap()
            .clone()
    };
    let node = native(&first.items[0]);
    assert_eq!(node, native(&run(query, &context).items[0]));
    let projected = run(
        r#"native:call("tree.keep", data:read(source, "json", "json-to-xml").root)"#,
        &context,
    );
    assert_ne!(node, native(&projected.items[0]));
    assert_ne!(
        node,
        native(&run(query, &self::context(r#"{"qty":3}"#)).items[0])
    );
    let tree = Arc::downgrade(node.owner());
    let source_owner = Arc::downgrade(node.owner().native_owner().unwrap());
    drop(first);
    drop(node);
    drop(projected);
    assert!(tree.upgrade().is_some());
    for i in 0..16 {
        context.policy_bindings.insert(
            "source".into(),
            ItemStream::once(Item::Atomic(AtomValue::String(format!("qty: {i}\n")))),
        );
        run(
            r#"native:call("tree.keep", data:read(source, "yaml").root)"#,
            &context,
        );
    }
    assert!(tree.upgrade().is_none());
    assert!(source_owner.upgrade().is_none());
    let last = run(
        r#"native:call("tree.keep", data:read(source, "yaml").root)"#,
        &context,
    );
    let weak = Arc::downgrade(native(&last.items[0]).owner());
    context.data_readers.clear();
    drop(context);
    assert!(weak.upgrade().is_some());
    drop(last);
    assert!(weak.upgrade().is_none());
}

#[test]
fn source_cem_text_members_adapt_to_one_canonical_xpath_node() {
    let context = context("<r>a<![CDATA[b]]>c</r>");
    let source = run(
        r#"data:read(source, "xml").root.children.children"#,
        &context,
    );
    assert_eq!(source.items.len(), 3);
    let mut nodes = Vec::new();
    for item in source.items {
        let mut bound = context.clone();
        bound
            .policy_bindings
            .insert("candidate".into(), ItemStream::once(item));
        let result = run(r#"native:call("tree.keep", candidate)"#, &bound);
        let native = result.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert_eq!(native.string_value(), "abc");
        nodes.push(native.clone());
    }
    assert!(nodes.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn imported_json_demo_library_packs_retained_tree_nodes_and_distinguishes_null() {
    for (source, summary, note) in [
        (
            r#"{"apple":2,"pear":3,"note":null}"#,
            "3 members; numeric total 5",
            "Null value",
        ),
        (
            r#"{"cherry":4,"note":""}"#,
            "2 members; numeric total 4",
            "Empty string",
        ),
        (
            r#"{"plum":1}"#,
            "1 members; numeric total 1",
            "Absent member",
        ),
    ] {
        let mut context = context(source);
        CemtXPathFunctions::compile(
            include_str!("../../cem-elements/demo/xpath-maps-arrays.cemt"),
            "memory:imports-demo.cemt",
        )
        .unwrap()
        .install(
            &mut context.native_functions,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
        let packed = run(
            r#"native:call("import.pack", data:read(source, "json", "json-to-xml").root)"#,
            &context,
        );
        context.policy_bindings.insert("packed".into(), packed);
        for (query, expected) in [
            (r#"native:call("import.summary", packed)"#, summary),
            (r#"native:call("import.note", packed)"#, note),
        ] {
            assert_eq!(
                run(query, &context).items[0].atom(),
                Some(AtomValue::String(expected.into()))
            );
        }
    }
}
