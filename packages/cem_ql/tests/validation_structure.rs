use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        declaration_references::SchemaDeclarationNode,
        input_references::{RetainedValidationStructure, StructuralValidationNode},
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    eval::{Item, QueryContextScope, QueryItemViewKind},
    validation_structure::{RetainedValidationQueryTree, ValidationPlacementNode},
};
use std::sync::Arc;
fn parse(source: &str) -> Arc<CemDocument> {
    let ast = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
    )))
    .build();
    assert!(ast.diagnostics.is_empty(), "{:?}", ast.diagnostics);
    Arc::new(ast)
}
fn element(owner: &Arc<CemDocument>, name: &str) -> SchemaDeclarationNode {
    let id = owner
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap();
    SchemaDeclarationNode::new(owner.clone(), id).unwrap()
}
fn snapshot() -> (Arc<CemDocument>, Vec<StructuralValidationNode>) {
    let source = parse("{left | {#items}}{right | {#items}}");
    let library = parse("{schema | {item @kind=page | value}}");
    let schema = element(&library, "schema");
    let item = element(&library, "item");
    let text = library
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Text { node_id, data, .. } if data.contains("value") => {
                SchemaDeclarationNode::new(library.clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    let node = |source, children| StructuralValidationNode {
        source,
        children,
        declaring_schema: None,
        children_complete: true,
        attribute_values: vec![],
    };
    let mut nodes = vec![
        node(element(&source, "left"), vec![2]),
        node(element(&source, "right"), vec![3]),
        node(item.clone(), vec![4]),
        node(item, vec![5]),
        node(text.clone(), vec![]),
        node(text, vec![]),
    ];
    nodes[2].declaring_schema = Some(schema.clone());
    nodes[3].declaring_schema = Some(schema);
    (source, nodes)
}
fn view<'a>(
    source: &'a Arc<CemDocument>,
    nodes: &'a [StructuralValidationNode],
) -> RetainedValidationStructure<'a> {
    RetainedValidationStructure {
        source,
        nodes,
        roots: &[0, 1],
        complete: true,
    }
}
fn placement(item: &Item) -> &ValidationPlacementNode {
    item.view()
        .unwrap()
        .downcast_ref::<ValidationPlacementNode>()
        .unwrap()
}
fn field(item: &Item, name: &str) -> Vec<Item> {
    item.view().unwrap().field(name).unwrap()
}
#[test]
fn repeated_placements_have_distinct_native_identity_and_consumed_ancestry() {
    let (source, nodes) = snapshot();
    let tree = RetainedValidationQueryTree::new(view(&source, &nodes)).unwrap();
    let roots = tree.roots();
    let first = field(&roots[0], "children").remove(0);
    let second = field(&roots[1], "children").remove(0);
    assert_eq!(first.view().unwrap().kind(), QueryItemViewKind::Node);
    assert_ne!(
        first.view().unwrap().identity(),
        second.view().unwrap().identity()
    );
    let a = placement(&first).source_node();
    let b = placement(&second).source_node();
    assert!(Arc::ptr_eq(a.document(), b.document()));
    assert_eq!(a.node_id(), b.node_id());
    assert!(Arc::ptr_eq(
        placement(&first).declaring_schema().unwrap().document(),
        a.document()
    ));
    let parent = first
        .view()
        .unwrap()
        .parent(QueryContextScope(0))
        .unwrap()
        .unwrap();
    assert_eq!(
        placement(&parent).source_node().node_id(),
        nodes[0].source.node_id()
    );
    let attrs = field(&first, "attributes");
    assert_eq!(attrs[0].view().unwrap().kind(), QueryItemViewKind::Node);
    assert!(Arc::ptr_eq(
        placement(&attrs[0]).source_node().document(),
        a.document()
    ));
    let attr_parent = attrs[0]
        .view()
        .unwrap()
        .parent(QueryContextScope(0))
        .unwrap()
        .unwrap();
    assert_eq!(
        attr_parent.view().unwrap().identity(),
        first.view().unwrap().identity()
    );
    let text: String = first
        .view()
        .unwrap()
        .text_fragments(QueryContextScope(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(text, "value");
    assert!(source
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
    drop(tree);
    drop(nodes);
    drop(source);
    assert_eq!(placement(&first).source_node().node_id(), a.node_id());
}
#[test]
fn overlapping_original_node_ids_do_not_merge_placements_or_owners() {
    let (source, mut nodes) = snapshot();
    let other = parse("{schema | {item @kind=page | other}}");
    let target = element(&other, "item");
    assert_eq!(target.node_id(), nodes[2].source.node_id());
    nodes[3].source = target;
    nodes[3].declaring_schema = Some(element(&other, "schema"));
    let tree = RetainedValidationQueryTree::new(view(&source, &nodes)).unwrap();
    let first = tree.node(2).unwrap();
    let second = tree.node(3).unwrap();
    assert_ne!(
        first.view().unwrap().identity(),
        second.view().unwrap().identity()
    );
    assert!(!Arc::ptr_eq(
        placement(&first).source_node().document(),
        placement(&second).source_node().document()
    ));
}
#[test]
fn incomplete_and_invalid_forests_are_rejected_before_query_access() {
    let (source, mut nodes) = snapshot();
    let mut pending = view(&source, &nodes);
    pending.complete = false;
    assert!(RetainedValidationQueryTree::new(pending).is_err());
    nodes[0].children_complete = false;
    assert!(RetainedValidationQueryTree::new(view(&source, &nodes)).is_err());
    nodes[0].children_complete = true;
    nodes[1].children = vec![2];
    assert!(RetainedValidationQueryTree::new(view(&source, &nodes)).is_err());
    nodes[1].children = vec![3];
    nodes[4].children = vec![0];
    assert!(RetainedValidationQueryTree::new(view(&source, &nodes)).is_err());
}

#[test]
fn standalone_queries_select_placements_by_consumed_parent_and_extract_native_text() {
    use cem_ql::{
        api::{evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{AtomValue, ItemStream},
    };
    let (source, nodes) = snapshot();
    let tree = RetainedValidationQueryTree::new(view(&source, &nodes)).unwrap();
    let items = vec![tree.node(2).unwrap(), tree.node(3).unwrap()];
    let context = StandaloneExpressionContext::default().with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::from_items(items)),
    );
    let selected = evaluate_expression(
        r#"seq:where(items, fn(candidate) => candidate.parent.name == "left")"#,
        &context,
    )
    .unwrap()
    .result;
    assert!(selected.error.is_none(), "{:?}", selected.diagnostics);
    assert_eq!(selected.items.len(), 1);
    assert_eq!(placement(&selected.items[0]).placement(), 2);
    let text = evaluate_expression("dom:text(items)", &context)
        .unwrap()
        .result;
    assert!(text.error.is_none(), "{:?}", text.diagnostics);
    assert_eq!(
        text.items[0].atom(),
        Some(AtomValue::String("valuevalue".into()))
    );
    let independent = RetainedValidationQueryTree::new(view(&source, &nodes)).unwrap();
    assert_ne!(
        tree.node(2).unwrap().view().unwrap().identity(),
        independent.node(2).unwrap().view().unwrap().identity()
    );
    assert!(Arc::ptr_eq(
        placement(&tree.node(2).unwrap()).source_node().document(),
        placement(&independent.node(2).unwrap())
            .source_node()
            .document()
    ));
}

#[test]
fn empty_completed_selection_is_queryable_but_pending_roots_are_not() {
    let source = parse("{#items}");
    let complete = RetainedValidationStructure {
        source: &source,
        nodes: &[],
        roots: &[],
        complete: true,
    };
    let tree = RetainedValidationQueryTree::new(complete).unwrap();
    assert!(tree.roots().is_empty());
    assert!(tree.node(0).is_none());
    let pending = RetainedValidationStructure {
        complete: false,
        ..complete
    };
    assert!(RetainedValidationQueryTree::new(pending).is_err());
}

#[test]
fn real_host_selection_readiness_and_scope_grants_control_placement_query_access() {
    use cem_ml::{
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        schema::{
            document_model::compile_schema_document_model, reference_policy::ReferenceScopePolicy,
        },
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let retained = |text: &str| {
        RetainedCemTree::new(
            Arc::try_unwrap(parse(text)).unwrap(),
            "fixture.cem",
            text,
            CemTreeSemantics::default(),
            None,
        )
        .unwrap()
    };
    let source = retained("{#library}");
    let library = retained("{schema | {item}}");
    let target = element(library.ast_owner(), "item");
    let item = RetainedCemNode::new(library.clone(), target.node_id())
        .unwrap()
        .query_item();
    let context = |items| {
        StandaloneExpressionContext::default().with_binding(
            "library",
            StandaloneExpressionBinding::any(ItemStream::from_items(items)),
        )
    };
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let from = host.register_scope(source.clone(), Some(context(vec![item])), policy.clone());
    let to = host.register_scope(library.clone(), None, policy.clone());
    let model =
        compile_schema_document_model("consumer", "{schema | {elements | {element @name=item}}}");
    let denied = host
        .validate_input(source.clone(), &model, policy.limits)
        .unwrap();
    assert!(!denied.complete);
    assert!(RetainedValidationQueryTree::new(denied.structure()).is_err());
    assert!(host.allow_scope_crossing(from, to));
    let selected = host
        .validate_input(source.clone(), &model, policy.limits)
        .unwrap();
    let tree = RetainedValidationQueryTree::new(selected.structure()).unwrap();
    assert_eq!(tree.roots().len(), 1);
    assert!(Arc::ptr_eq(tree.source(), source.ast_owner()));
    assert!(Arc::ptr_eq(
        placement(&tree.roots()[0]).source_node().document(),
        library.ast_owner()
    ));
    assert!(host.set_context(from, Some(context(vec![]))));
    let empty = host
        .validate_input(source.clone(), &model, policy.limits)
        .unwrap();
    assert!(empty.complete);
    assert!(RetainedValidationQueryTree::new(empty.structure())
        .unwrap()
        .roots()
        .is_empty());
    assert!(host.set_context(from, None));
    let pending = host
        .validate_input(source.clone(), &model, policy.limits)
        .unwrap();
    assert!(!pending.complete && pending.nodes.is_empty());
    assert!(RetainedValidationQueryTree::new(pending.structure()).is_err());
    assert!(source
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

struct AttributeHost {
    targets: Vec<SchemaDeclarationNode>,
    calls: usize,
    policy: cem_ml::schema::reference_policy::ReferenceScopePolicy,
    denied: Option<String>,
}
impl cem_ml::value::reference_resolution::ReferenceResolutionHost for AttributeHost {
    type Node = SchemaDeclarationNode;
    type Scope = usize;
    fn scope(&self, node: &Self::Node) -> usize {
        Arc::as_ptr(node.document()) as usize
    }
    fn scope_limits(
        &self,
        _: &usize,
    ) -> cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
        self.policy.limits
    }
    fn reference_occurrence(
        &self,
        node: &Self::Node,
    ) -> Option<cem_ml::schema::reference_policy::ReferenceOccurrence> {
        match node.node() {
            CemAstNode::Reference {
                expression, source, ..
            } => Some(cem_ml::schema::reference_policy::ReferenceOccurrence {
                identity: node.identity(),
                node_id: Some(node.node_id()),
                expression: Some(expression.clone()),
                source_map: source.clone(),
            }),
            _ => None,
        }
    }
    fn unresolved_policy(
        &self,
        _: &Self::Node,
    ) -> &cem_ml::schema::reference_policy::ReferenceUnresolvedPolicy {
        &self.policy.unresolved
    }
    fn permits_edge(&self, _: &Self::Node, target: &Self::Node) -> bool {
        self.denied.as_deref() != Some(target.identity().as_str())
    }
    fn evaluate(
        &mut self,
        reference: &Self::Node,
    ) -> cem_ml::value::reference_resolution::ReferenceLinkEvaluation<Self::Node> {
        self.calls += 1;
        assert!(
            matches!(reference.node(),CemAstNode::Reference{expression,..} if expression=="#target"),
            "descendant references must stay authored"
        );
        cem_ml::value::reference_resolution::ReferenceLinkEvaluation::Resolved(self.targets.clone())
    }
}
#[test]
fn consumed_attribute_values_are_native_and_keep_queries_inside_selected_targets() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    use cem_ql::attribute_values::NativeAttributeQueryNode;
    let source = parse("{box @target={#target}}");
    let mut library = parse("{outside | {target @label=value | {child} {#nested}}}");
    let outside = element(&library, "outside").node_id();
    let nested = library
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Reference { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    if let CemAstNode::Reference {
        context, targets, ..
    } = &mut Arc::get_mut(&mut library).unwrap().nodes[nested as usize]
    {
        *context = outside;
        *targets = Some(vec![outside]);
    }
    let model=cem_ml::schema::document_model::compile_schema_document_model("consumer","{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node @itemCount=2}}}");
    let mut host = AttributeHost {
        targets: vec![element(&library, "target"); 2],
        calls: 0,
        policy: cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults().unwrap(),
        denied: None,
    };
    let limits = host.policy.limits;
    let report =
        validate_structural_input_references(source.clone(), &model, &mut host, limits).unwrap();
    assert!(report.complete);
    assert!(!report.failed);
    assert_eq!(host.calls, 1);
    let tree = RetainedValidationQueryTree::new(report.structure()).unwrap();
    let root = tree.roots().remove(0);
    let attr = root.view().unwrap().field("attributes").unwrap().remove(0);
    let values = attr.view().unwrap().field("value").unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[0].view().unwrap().identity(),
        values[1].view().unwrap().identity()
    );
    for value in &values {
        let view = value.view().unwrap();
        assert_eq!(view.kind(), QueryItemViewKind::Node);
        let native = view.downcast_ref::<NativeAttributeQueryNode>().unwrap();
        assert!(Arc::ptr_eq(native.source_node().document(), &library));
        assert!(view.parent(QueryContextScope(0)).unwrap().is_none());
        let children = view.field("children").unwrap();
        assert_eq!(children.len(), 4);
        let reference = children
            .iter()
            .find(|node| {
                node.view()
                    .unwrap()
                    .downcast_ref::<NativeAttributeQueryNode>()
                    .is_some_and(|node| {
                        matches!(node.source_node().node(), CemAstNode::Reference { .. })
                    })
            })
            .unwrap();
        assert!(reference.view().unwrap().field("context").is_none());
        assert!(reference.view().unwrap().field("targets").is_none());
        assert!(reference
            .view()
            .unwrap()
            .parent(QueryContextScope(0))
            .unwrap()
            .is_some());
    }
    assert!(
        matches!(attr.view().unwrap().downcast_ref::<ValidationPlacementNode>().unwrap().source_node().node(),CemAstNode::Attribute{value_nodes,..} if !value_nodes.is_empty())
    );
}
#[test]
fn incomplete_attribute_subtree_access_never_becomes_a_query_tree() {
    use cem_ml::schema::input_references::validate_structural_input_references;
    let source = parse("{box @target={#target}}");
    let library = parse("{target | {child}}");
    let model=cem_ml::schema::document_model::compile_schema_document_model("consumer","{schema | {elements | {element @name=box @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node}}}");
    for denied in [false, true] {
        let mut host = AttributeHost {
            targets: vec![element(&library, "target")],
            calls: 0,
            policy: cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults()
                .unwrap(),
            denied: denied.then(|| element(&library, "child").identity()),
        };
        let limits = if denied {
            host.policy.limits
        } else {
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_depth: 128,
                max_work: 2,
            }
        };
        let report =
            validate_structural_input_references(source.clone(), &model, &mut host, limits)
                .unwrap();
        assert!(!report.complete);
        assert!(RetainedValidationQueryTree::new(report.structure()).is_err());
        let access = report.nodes[0].attribute_values[0].access.clone();
        assert!(cem_ql::attribute_values::NativeAttributeQueryTree::new(access).is_err());
    }
}
impl cem_ml::schema::input_references::InputReferenceHost for AttributeHost {
    fn source_node(&self, source: SchemaDeclarationNode) -> Self::Node {
        source
    }
    fn retained_node(&self, node: &Self::Node) -> Option<SchemaDeclarationNode> {
        Some(node.clone())
    }
}
