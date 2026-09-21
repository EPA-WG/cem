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

#[test]
fn constructed_and_portable_values_share_xpath_identity_and_text() {
    use cem_ql::{
        eval::{
            output::output_nodes,
            portable::{decode_values, encode_values},
        },
        render::*,
    };
    let artifact = compile_template(
        r#"{r @count='2' | {name | ivy{em | saur}}}"#,
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    let values = output_nodes(plan.nodes);
    let limits = cem_ml::value::artifact::CemValueArtifactLimits::default();
    let portable = decode_values(&encode_values(&values, &limits).unwrap(), &limits).unwrap();
    for values in [values, portable] {
        let mut context = context("");
        context
            .policy_bindings
            .insert("values".into(), values.clone());
        let text = run(r#"native:call("tree.text", values)"#, &context);
        assert_eq!(
            text.items[0].atom(),
            Some(AtomValue::String("ivysaur".into()))
        );
        let a = run(r#"native:call("tree.keep", values)"#, &context);
        let b = run(r#"native:call("tree.keep", values)"#, &context);
        assert_eq!(a.items[0].identity(), b.items[0].identity());
        let node = a.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert!(
            node.parent_node().is_none(),
            "a constructed root has no fabricated document parent"
        );
        assert_eq!(node.attribute_nodes()[0].string_value(), "2");
    }
}

#[test]
fn xpath_reference_occurrences_have_output_parents_and_targets_keep_source_parents() {
    use cem_ql::{
        eval::{
            output::output_nodes,
            portable::{decode_values, encode_values},
        },
        render::*,
    };
    let plan = render_compiled_template(
        &compile_template(
            r#"{cem:variable @name=n @select='data:read("<source xmlns:p=\"urn:names\"><p:name rank=\"2\">ivy<em>saur</em></p:name></source>", "xml").root.children.children'}{output | {attribute @name=label @type=node @value='{n}'}{$n}{$n}}"#,
            &CompileTemplateOptions::default(),
        ),
        &TemplateData::default(),
    );
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    let values = output_nodes(plan.nodes);
    let limits = cem_ml::value::artifact::CemValueArtifactLimits::default();
    let restored = decode_values(&encode_values(&values, &limits).unwrap(), &limits).unwrap();
    for values in [values, restored] {
        let mut context = context("");
        context.policy_bindings.insert("values".into(), values);
        let output = run(r#"native:call("tree.keep", values)"#, &context);
        let node = output.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        let children = node.child_nodes();
        assert_eq!(children.len(), 2);
        assert_ne!(children[0].identity(), children[1].identity());
        assert_eq!(children[0].namespace_uri(), "urn:names");
        assert_eq!(
            children[0].parent_node().unwrap().identity(),
            node.identity()
        );
        assert_eq!(node.attribute_nodes()[0].string_value(), "ivysaur");
        let targets = run("seq:first(values.children).targets", &context);
        context.policy_bindings.insert("target".into(), targets);
        let target = run(r#"native:call("tree.keep", target)"#, &context);
        let target = target.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert_eq!(target.parent_node().unwrap().local_name(), "source");
        assert_ne!(target.identity(), children[0].identity());
        assert_eq!(target.source_key(), children[0].source_key());
        assert_eq!(
            target.source_line_number(),
            children[0].source_line_number()
        );
    }
}

#[test]
fn cloned_documents_and_standalone_attributes_keep_their_xpath_kinds() {
    let context = context("<r rank=\"2\">value</r>");
    for (query, kind) in [
        (
            r#"native:call("tree.keep", dom:clone(data:read(source, "xml").root))"#,
            cem_ml::validation::xpath::XPathResultNodeKind::Document,
        ),
        (
            r#"native:call("tree.keep", dom:clone(data:read(source, "xml").root.children.attributes))"#,
            cem_ml::validation::xpath::XPathResultNodeKind::Attribute,
        ),
    ] {
        let result = run(query, &context);
        let node = result.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert_eq!(node.result_node_kind(), kind);
        assert!(node.parent_node().is_none());
    }
}

#[test]
fn xpath_accepts_portable_scalars_and_returned_nodes_remain_portable() {
    use cem_ql::eval::portable::{decode_values, encode_values};
    let limits = cem_ml::value::artifact::CemValueArtifactLimits::default();
    let mut context = context("<r><name>ivy</name></r>");
    let selected = run(
        r#"native:call("tree.pick", data:read(source, "xml").root)"#,
        &context,
    );
    let restored = decode_values(&encode_values(&selected, &limits).unwrap(), &limits).unwrap();
    context.policy_bindings.insert("values".into(), restored);
    assert_eq!(
        run(r#"native:call("tree.text", values)"#, &context).items[0].atom(),
        Some(AtomValue::String("ivy".into()))
    );
    let scalar = ItemStream::once(Item::Atomic(AtomValue::Integer(2)));
    for values in [
        scalar.clone(),
        decode_values(&encode_values(&scalar, &limits).unwrap(), &limits).unwrap(),
    ] {
        context.policy_bindings.insert("values".into(), values);
        assert_eq!(
            run(r#"native:call("tree.text", values)"#, &context).items[0].atom(),
            Some(AtomValue::String("2".into()))
        );
    }
}

#[test]
fn root_insertions_are_detached_occurrences_while_explicit_references_select_targets() {
    use cem_ql::{
        eval::{
            output::output_nodes,
            portable::{decode_values, encode_values},
        },
        render::*,
    };
    let plan = render_compiled_template(
        &compile_template(
            r#"{$data:read("<r><name>ivy</name></r>", "xml").root.children.children}"#,
            &CompileTemplateOptions::default(),
        ),
        &TemplateData::default(),
    );
    let values = output_nodes(plan.nodes);
    let limits = cem_ml::value::artifact::CemValueArtifactLimits::default();
    for values in [
        values.clone(),
        decode_values(&encode_values(&values, &limits).unwrap(), &limits).unwrap(),
    ] {
        let mut context = context("");
        context.policy_bindings.insert("values".into(), values);
        let result = run(r#"native:call("tree.keep", values)"#, &context);
        let node = result.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert!(node.parent_node().is_none());
        let targets = run(
            r#"native:call("tree.keep", dom:reference(values.targets))"#,
            &context,
        );
        let node = targets.items[0]
            .view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap();
        assert_eq!(node.parent_node().unwrap().local_name(), "r");
    }
}

#[test]
fn constructed_xpath_cache_does_not_reuse_a_broader_scope_grant() {
    use cem_ql::eval::{
        QueryContextScope, QueryItemView, QueryItemViewKind, QueryNodeAccessError,
        QueryNodeIterator,
    };
    #[derive(Debug)]
    struct ScopedText;
    impl QueryItemView for ScopedText {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "test.scoped-text"
        }
        fn identity(&self) -> String {
            "scoped-text".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Node
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            let value = match name {
                "kind" => "text",
                "value" => "restricted",
                _ => return None,
            };
            Some(vec![Item::Atomic(AtomValue::String(value.into()))])
        }
        fn parent(&self, scope: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
            if scope.0 == 0 {
                Ok(None)
            } else {
                Err(QueryNodeAccessError::ScopeViolation)
            }
        }
        fn children(
            &self,
            _: QueryContextScope,
        ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
            Ok(Box::new(std::iter::empty()))
        }
    }
    let values =
        cem_ql::eval::output::output_nodes(vec![cem_ql::render::RenderPlanNode::Reference {
            reference: cem_ml::value::CemReference::new(vec![Item::native(ScopedText)]),
            source_map: Default::default(),
        }]);
    let mut context = context("");
    context.policy_bindings.insert("values".into(), values);
    assert_eq!(
        run(r#"native:call("tree.text", values)"#, &context).items[0].atom(),
        Some(AtomValue::String("restricted".into()))
    );
    context.scope = QueryContextScope(1);
    let query = compile(
        r#"native:call("tree.text", values)"#,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let denied = evaluate(&query, &context);
    assert!(denied.error.is_some());
    assert!(denied.items.is_empty());
}

#[test]
fn xpath_index_cache_is_bounded_across_scopes_and_releases_memory_with_owners() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ql::api::evaluate_with_control;
    use cem_ql::render::*;
    let plan = render_compiled_template(&compile_template("{r | {name | ivy}}", &CompileTemplateOptions::default()), &TemplateData::default());
    let mut context = context("");
    context.policy_bindings.insert("values".into(), cem_ql::eval::output::output_nodes(plan.nodes));
    let query = compile(r#"native:call("tree.text", values)"#, &CompileContext { policy_bindings: context.policy_bindings.clone(), ..Default::default() }).unwrap();
    let control = OperationControl::default();
    let mut last_charge = 0;
    for scope in 0..20 {
        context.scope = cem_ql::eval::QueryContextScope(scope);
        let result = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
        assert!(result.error.is_none(), "{:?}", result.diagnostics);
        let charge = control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap();
        assert!(charge > 0);
        if scope > 0 { assert_eq!(charge, last_charge); }
        last_charge = charge;
    }
    drop(query);
    drop(context);
    assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}

#[test]
fn cached_xpath_index_respects_lowered_memory_and_control_errors_are_not_caught() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ql::api::evaluate_with_control;
    use cem_ql::render::*;
    let plan = render_compiled_template(&compile_template("{r | {name | ivy}}", &CompileTemplateOptions::default()), &TemplateData::default());
    let mut context = context("");
    context.policy_bindings.insert("values".into(), cem_ql::eval::output::output_nodes(plan.nodes));
    run(r#"native:call("tree.text", values)"#, &context);
    let query = compile(r#"try { native:call("tree.text", values) } catch (code, message) { "caught" }"#, &CompileContext { policy_bindings: context.policy_bindings.clone(), ..Default::default() }).unwrap();
    let control = OperationControl::with_root_policy(Default::default(), cem_ml::scheduler::ScopePolicy::host_root().with_memory_bytes(64)).unwrap();
    let result = evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
    assert!(result.error.is_some(), "{result:?}");
    assert!(result.items.is_empty());
}
