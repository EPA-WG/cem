use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{admit_namespace_scope_target, NamespaceNameCompletion},
        vocab::CompiledSchema,
    },
};
use cem_ql::{
    api::{evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{
        retained_cem_node, values::ReferenceView, AtomValue, Item, ItemStream, QueryContextScope,
    },
    namespace_names::NamespaceQueryTree,
};
use std::{collections::BTreeMap, sync::Arc};
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "source.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn element(source: &ScopedCemImport, local: &str) -> u32 {
    source
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
fn completed(source: &ScopedCemImport, uri: &str, root: u32) -> Arc<NamespaceNameCompletion> {
    let vendor = import(&format!("@ns public = {uri}\n{{public:item}}"));
    let target = admit_namespace_scope_target(
        SchemaDeclarationNode::new(vendor.tree.ast_owner().clone(), element(&vendor, "@ns"))
            .unwrap(),
        &vendor.captured,
    )
    .unwrap();
    let id = source
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Attribute {
                node_id,
                value_nodes,
                ..
            } if !value_nodes.is_empty() => Some(*node_id),
            _ => None,
        })
        .unwrap();
    Arc::new(
        NamespaceNameCompletion::new(
            source.captured.clone(),
            &[root],
            BTreeMap::from([(id, target)]),
        )
        .unwrap(),
    )
}
fn field(item: &Item, name: &str) -> Vec<Item> {
    item.view().unwrap().field(name).unwrap()
}
fn eval(source: &str, item: Item) -> Vec<Item> {
    let ctx = StandaloneExpressionContext::default().with_binding(
        "item",
        StandaloneExpressionBinding::any(ItemStream::once(item)),
    );
    let result = evaluate_expression(source, &ctx).unwrap().result;
    assert!(result.error.is_none(), "{:?}", result.error);
    result.items
}
#[test]
fn query_names_are_execution_local_and_source_handles_remain_original() {
    let source = import("{host @xmlns:v={#library} | {v:item @v:flag=yes | content {#related}} {v:pending}} {outside}");
    let root = element(&source, "item");
    let first =
        NamespaceQueryTree::new(source.tree.clone(), completed(&source, "urn:one", root)).unwrap();
    let second =
        NamespaceQueryTree::new(source.tree.clone(), completed(&source, "urn:two", root)).unwrap();
    let a = first.roots().remove(0);
    let b = second.roots().remove(0);
    assert_eq!(
        eval("item.namespace", a.clone()),
        vec![Item::Atomic(AtomValue::String("urn:one".into()))]
    );
    assert_eq!(
        eval("item.namespace", b.clone()),
        vec![Item::Atomic(AtomValue::String("urn:two".into()))]
    );
    assert_ne!(a.identity(), b.identity());
    assert_eq!(
        eval("item.attributes.namespace", a.clone()),
        vec![Item::Atomic(AtomValue::String("urn:one".into()))]
    );
    assert_eq!(
        field(
            &eval(
                "item.attribute({namespace: \"urn:one\", name: \"flag\"})",
                a.clone()
            )[0],
            "namespace"
        ),
        vec![Item::Atomic(AtomValue::String("urn:one".into()))]
    );
    assert!(eval(
        "item.attribute({namespace: \"urn:two\", name: \"flag\"})",
        a.clone()
    )
    .is_empty());
    let raw = field(&a, "source").remove(0);
    let metadata = "(data:node_key(item), data:line_number(item), data:base_uri(item), data:document_uri(item))";
    let original_metadata = eval(metadata, raw.clone());
    assert!(!original_metadata.is_empty());
    assert_eq!(eval(metadata, a.clone()), original_metadata);
    assert_eq!(
        field(&raw, "namespace"),
        vec![Item::Atomic(AtomValue::String("v".into()))]
    );
    for item in [&a, &b, &raw] {
        let original = retained_cem_node(item).unwrap();
        assert!(Arc::ptr_eq(
            original.owner().ast_owner(),
            source.tree.ast_owner()
        ));
        assert_eq!(original.node_id(), root);
        assert_eq!(item.source_map(), raw.source_map());
    }
    let reference = eval("#item", a.clone()).remove(0);
    let targets = reference
        .view()
        .unwrap()
        .downcast_ref::<ReferenceView>()
        .unwrap()
        .0
        .values();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].identity(), a.identity());
    assert!(Arc::ptr_eq(
        retained_cem_node(&targets[0]).unwrap().owner().ast_owner(),
        source.tree.ast_owner()
    ));
    assert!(source
        .captured
        .expanded_name(source.tree.ast_owner(), root)
        .is_none());
}
#[test]
fn selected_forest_axes_keep_completion_boundary_and_descendant_references() {
    let source = import(
        "{host @xmlns:v={#library} | {v:item @v:flag=yes | content {#related}} {v:pending}}",
    );
    let root = element(&source, "item");
    let tree =
        NamespaceQueryTree::new(source.tree.clone(), completed(&source, "urn:one", root)).unwrap();
    let item = tree.roots().remove(0);
    assert!(tree.node(element(&source, "pending")).is_none());
    assert!(item
        .view()
        .unwrap()
        .parent(QueryContextScope(0))
        .unwrap()
        .is_none());
    assert!(field(&item, "parent").is_empty());
    let attr = item
        .view()
        .unwrap()
        .attributes(QueryContextScope(0))
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    assert_eq!(
        attr.view()
            .unwrap()
            .parent(QueryContextScope(0))
            .unwrap()
            .unwrap()
            .identity(),
        item.identity()
    );
    let children = item
        .view()
        .unwrap()
        .children(QueryContextScope(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let reference = children
        .iter()
        .find(|item| {
            field(item, "kind") == vec![Item::Atomic(AtomValue::String("reference".into()))]
        })
        .unwrap();
    assert!(matches!(
        retained_cem_node(reference).unwrap().node(),
        CemAstNode::Reference { targets: None, .. }
    ));
    assert_eq!(field(reference, "context")[0].identity(), item.identity());
    // A reference-only selection cannot borrow its incomplete enclosing context.
    let reference_root = retained_cem_node(reference).unwrap().node_id();
    let reference_names = Arc::new(
        NamespaceNameCompletion::new(source.captured.clone(), &[reference_root], BTreeMap::new())
            .unwrap(),
    );
    let reference_view = NamespaceQueryTree::new(source.tree.clone(), reference_names)
        .unwrap()
        .roots()
        .remove(0);
    assert!(reference_view.view().unwrap().field("context").is_none());
    let text: String = item
        .view()
        .unwrap()
        .text_fragments(QueryContextScope(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(text.contains("content"));
}
#[test]
fn query_view_rejects_another_arena_even_with_equal_source_and_ids() {
    let source = import("{host @xmlns:v={#library} | {v:item}}");
    let other = import("{host @xmlns:v={#library} | {v:item}}");
    let names = completed(&source, "urn:one", element(&source, "item"));
    assert!(NamespaceQueryTree::new(other.tree, names).is_err());
}
