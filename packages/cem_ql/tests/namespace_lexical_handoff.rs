use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        namespace_references::{
            admit_namespace_scope_target, NamespaceNameCompletion, NamespaceNameCompletionError,
            NamespaceScopeTarget,
        },
        reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
        vocab::CompiledSchema,
    },
    value::reference_resolution::{
        resolve_reference, ReferenceResolutionIssueKind, ReferenceResolutionState,
    },
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{AtomValue, Item, ItemStream, RetainedCemNode},
    namespace_names::NamespaceQueryTree,
    schema_references::{
        CemQlSchemaDeclarationHost, LexicalScopeHandoffError, NamespaceLexicalScopeHandoffError,
    },
};
use std::{collections::BTreeMap, sync::Arc};
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "handoff.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn element(source: &ScopedCemImport, name: &str) -> u32 {
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
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
fn source(source: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(source.tree.ast_owner().clone(), id).unwrap()
}
fn declaration(input: &ScopedCemImport) -> u32 {
    input
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                value_nodes,
                ..
            } if expanded_name.namespace_uri == "xmlns" && !value_nodes.is_empty() => {
                Some(*node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn target(uri: &str) -> NamespaceScopeTarget {
    let vendor = import(&format!("@ns public = {uri}\n{{public:item}}"));
    admit_namespace_scope_target(source(&vendor, element(&vendor, "@ns")), &vendor.captured)
        .unwrap()
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn completion(input: &ScopedCemImport, root: u32, uri: Option<&str>) -> NamespaceNameCompletion {
    NamespaceNameCompletion::new(
        input.captured.clone(),
        &[root],
        uri.map(|uri| BTreeMap::from([(declaration(input), target(uri))]))
            .unwrap_or_default(),
    )
    .unwrap()
}
fn context(input: &ScopedCemImport, target: u32) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(input.tree.clone(), target)
                .unwrap()
                .query_item(),
        )),
    )
}

#[test]
fn pending_unused_binding_preflights_all_occurrences_before_callbacks_and_retry() {
    let input = import("@ns v = urn:outer\n{host @xmlns:v={#library} | {#library}} {#outside}");
    let roots = element(&input, "host");
    let refs = input.captured.occurrences().collect::<Vec<_>>();
    let mut host = CemQlSchemaDeclarationHost::new();
    let parent = host.register_scope(input.tree.clone(), Some(context(&input, roots)), policy());
    let pending = completion(&input, roots, None);
    let result = host.attach_completed_namespace_lexical_scopes(&pending, |_, _, _| {
        panic!("pending dependency must reject before any preparation")
    });
    assert!(
        matches!(result, Err(NamespaceLexicalScopeHandoffError::Namespace(NamespaceNameCompletionError::Pending {node, declaration: dep})) if node == refs[1] && dep == declaration(&input))
    );
    let ready = completion(&input, roots, Some("urn:ready"));
    let mut seen = vec![];
    let attached = host
        .attach_completed_namespace_lexical_scopes(&ready, |node, snapshot, scope| {
            assert_eq!(scope, parent);
            seen.push((
                node.node_id(),
                snapshot.namespace_uri("v").unwrap().to_owned(),
            ));
            (None, ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
    assert_eq!(
        seen,
        vec![(refs[0], "urn:outer".into()), (refs[1], "urn:ready".into())]
    );
    assert_eq!(attached.len(), 2);
    assert_eq!(host.lexical_scope_parent(attached[1].1), Some(parent));
    assert!(host
        .compiled_source_expression(&source(&input, refs[1]))
        .is_none());
    let reference = host.source_reference(source(&input, refs[1]));
    assert_eq!(
        resolve_reference(reference, &mut host, policy().limits)
            .unwrap()
            .state,
        ReferenceResolutionState::Pending
    );
    // Only selected occurrences were attached; the outside occurrence is free.
    let outside =
        NamespaceNameCompletion::new(input.captured.clone(), &[refs[2]], BTreeMap::new()).unwrap();
    assert_eq!(
        host.attach_completed_namespace_lexical_scopes(&outside, |_, _, _| (
            None,
            ReferenceScopePolicyOverrides::default()
        ))
        .unwrap()
        .len(),
        1
    );
    assert!(
        matches!(host.attach_completed_namespace_lexical_scopes(&ready, |_, _, _| panic!("repeat")), Err(NamespaceLexicalScopeHandoffError::Lexical(LexicalScopeHandoffError::OccurrenceAlreadyAssigned(id))) if id == refs[0])
    );
}

#[test]
fn pending_declaration_selector_can_handoff_its_pre_declaration_context_alone() {
    let input = import("@ns v = urn:outer\n{host @xmlns:v={#library} | {#library}}");
    let reference = input.captured.occurrences().next().unwrap();
    let pending =
        NamespaceNameCompletion::new(input.captured.clone(), &[reference], BTreeMap::new())
            .unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), None, policy());
    let attached = host
        .attach_completed_namespace_lexical_scopes(&pending, |node, snapshot, _| {
            assert_eq!(node.node_id(), reference);
            assert_eq!(snapshot.namespace_uri("v"), Some("urn:outer"));
            assert!(snapshot.completed_bindings().is_empty());
            (
                Some(context(&input, element(&input, "host"))),
                ReferenceScopePolicyOverrides::default(),
            )
        })
        .unwrap();
    assert_eq!(attached.len(), 1);
}

#[test]
fn independent_execution_contexts_retain_completed_native_views_and_original_sources() {
    let input = import("{host @xmlns:v={#library} | {#library} {v:item}} {outside}");
    let root = element(&input, "host");
    let item = element(&input, "item");
    let refs = input.captured.occurrences().collect::<Vec<_>>();
    for uri in ["urn:one", "urn:two"] {
        let names = Arc::new(completion(&input, root, Some(uri)));
        let view = NamespaceQueryTree::new(input.tree.clone(), names.clone()).unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(input.tree.clone(), None, policy());
        host.attach_completed_namespace_lexical_scopes(&names, |node, snapshot, _| {
            if node.node_id() == refs[1] {
                assert_eq!(snapshot.namespace_uri("v"), Some(uri));
            }
            let context = StandaloneExpressionContext::default().with_binding(
                "library",
                StandaloneExpressionBinding::any(ItemStream::once(view.node(item).unwrap())),
            );
            (Some(context), ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
        let reference = host.source_reference(source(&input, refs[1]));
        let result = resolve_reference(reference, &mut host, policy().limits).unwrap();
        assert!(result.is_complete(), "{:?}", result.issues);
        let target = host.declaration_node(&result.nodes[0]).unwrap();
        assert_eq!(target.node_id(), item);
        assert!(Arc::ptr_eq(target.document(), input.tree.ast_owner()));
        assert_eq!(
            view.node(item)
                .unwrap()
                .view()
                .unwrap()
                .field("namespace")
                .unwrap(),
            vec![Item::Atomic(AtomValue::String(uri.into()))]
        );
        assert_eq!(
            view.node(item)
                .unwrap()
                .view()
                .unwrap()
                .field("source")
                .unwrap()[0]
                .view()
                .unwrap()
                .field("namespace")
                .unwrap(),
            vec![Item::Atomic(AtomValue::String("v".into()))]
        );
        assert!(view.node(element(&input, "outside")).is_none());
    }
    assert!(input
        .captured
        .snapshot(input.tree.ast_owner(), refs[1])
        .unwrap()
        .namespaces
        .binding("v")
        .is_none());
    assert!(input.tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn completed_handoff_preserves_relationship_boundaries_policy_provenance_and_denied_crossings() {
    let input = import("{host @xmlns:v={#library} | {#library}}");
    let library = import("{target}");
    let ready = completion(&input, element(&input, "host"), Some("urn:ready"));
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), None, policy());
    let boundary = host.register_scope(input.tree.clone(), None, policy());
    assert!(host.assign_subtree_scope(&input.tree, element(&input, "host"), boundary));
    let destination = host.register_scope(library.tree.clone(), None, policy());
    let overrides = ReferenceScopePolicyOverrides::explicit(policy());
    let attached = host
        .attach_completed_namespace_lexical_scopes(&ready, |_, _, parent| {
            assert_eq!(parent, boundary);
            (
                Some(context(&library, element(&library, "target"))),
                overrides.clone(),
            )
        })
        .unwrap();
    let stored = host.scope_policy_overrides(attached[1].1).unwrap();
    assert_eq!(
        stored.depth().unwrap().value(),
        overrides.depth().unwrap().value()
    );
    assert_eq!(
        stored.work().unwrap().value(),
        overrides.work().unwrap().value()
    );
    assert!(matches!(
        stored.depth().unwrap().origin(),
        cem_ml::schema::reference_policy::ReferencePolicyOverrideOrigin::Caller
    ));
    let reference = host.source_reference(source(&input, attached[1].0));
    assert!(
        resolve_reference(reference.clone(), &mut host, policy().limits)
            .unwrap()
            .issues
            .iter()
            .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied)
    );
    assert!(host.allow_scope_crossing(boundary, destination));
    assert!(resolve_reference(reference, &mut host, policy().limits)
        .unwrap()
        .is_complete());
}

#[test]
fn foreign_registered_tree_cannot_supply_completion_handoff_owner() {
    let input = import("{#library}");
    let foreign = import("{#library}");
    let ready = NamespaceNameCompletion::new(
        input.captured.clone(),
        &[input.captured.occurrences().next().unwrap()],
        BTreeMap::new(),
    )
    .unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(foreign.tree.clone(), None, policy());
    assert!(matches!(
        host.attach_completed_namespace_lexical_scopes(&ready, |_, _, _| panic!("foreign owner")),
        Err(NamespaceLexicalScopeHandoffError::Lexical(
            LexicalScopeHandoffError::UnregisteredOwner
        ))
    ));
}

#[test]
fn general_attribute_expression_keeps_intrinsic_name_and_explicit_evaluation_hook() {
    use cem_ml::value::reference_resolution::ReferenceLinkEvaluation;
    let input = import("{host @xmlns:v={#namespace} @target={library} | {v:item}}");
    let names = Arc::new(completion(
        &input,
        element(&input, "host"),
        Some("urn:ready"),
    ));
    let view = NamespaceQueryTree::new(input.tree.clone(), names.clone()).unwrap();
    let expression = *input
        .captured
        .occurrences()
        .collect::<Vec<_>>()
        .last()
        .unwrap();
    assert_eq!(
        view.node(expression)
            .unwrap()
            .view()
            .unwrap()
            .field("name")
            .unwrap(),
        vec![Item::Atomic(AtomValue::String("$".into()))]
    );
    assert_eq!(
        view.node(expression)
            .unwrap()
            .view()
            .unwrap()
            .field("namespace")
            .unwrap(),
        vec![Item::Atomic(AtomValue::String("".into()))]
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), None, policy());
    host.attach_completed_namespace_lexical_scopes(&names, |node, snapshot, _| {
        if node.node_id() == expression {
            assert_eq!(snapshot.namespace_uri("v"), Some("urn:ready"));
        }
        let context = StandaloneExpressionContext::default().with_binding(
            "library",
            StandaloneExpressionBinding::any(ItemStream::once(
                view.node(element(&input, "item")).unwrap(),
            )),
        );
        (Some(context), ReferenceScopePolicyOverrides::default())
    })
    .unwrap();
    let original = source(&input, expression);
    assert!(host.compiled_source_expression(&original).is_none());
    let expression_node = host.source_reference(original);
    let ReferenceLinkEvaluation::Resolved(nodes) = host.evaluate_input_expression(&expression_node)
    else {
        panic!("ready general expression should select native nodes");
    };
    assert_eq!(nodes.len(), 1);
    let selected = host.declaration_node(&nodes[0]).unwrap();
    assert_eq!(selected.node_id(), element(&input, "item"));
    assert!(Arc::ptr_eq(selected.document(), input.tree.ast_owner()));
}
