use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        namespace_references::NamespaceNameCompletionError,
        reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
        vocab::CompiledSchema,
    },
    value::reference_resolution::{resolve_reference, ReferenceResolutionState},
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, LexicalScopeHandoffError, NamespaceLexicalScopeHandoffError,
        NamespacePropertyActivationError,
    },
};
use std::sync::Arc;
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "activation.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn node(input: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(input.tree.ast_owner().clone(), id).unwrap()
}
fn element(input: &ScopedCemImport, local: &str) -> u32 {
    input
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
fn property(input: &ScopedCemImport) -> u32 {
    input
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
        .unwrap()
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn context(input: &ScopedCemImport) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(input.tree.clone(), element(input, "@ns"))
                .unwrap()
                .query_item(),
        )),
    )
}
fn host(input: &ScopedCemImport, ready: bool) -> CemQlSchemaDeclarationHost {
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), ready.then(|| context(input)), policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    host
}

#[test]
fn ready_results_complete_selected_names_and_install_only_selected_contexts() {
    let input = import("@ns public = urn:ready\n{host @xmlns:v={#library} | {v:item | {#library}}} {outside | {#library}}");
    let mut host = host(&input, true);
    let prepared = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    let mut seen = vec![];
    let activation = host
        .activate_namespace_properties(
            input.captured.clone(),
            &[element(&input, "item")],
            &[prepared],
            |source, snapshot, _, completion| {
                seen.push(source.node_id());
                assert_eq!(snapshot.namespace_uri("v"), Some("urn:ready"));
                assert!(Arc::ptr_eq(completion.captured(), &input.captured));
                (None, ReferenceScopePolicyOverrides::default())
            },
        )
        .unwrap();
    assert_eq!(activation.scopes.len(), 1);
    assert_eq!(seen, vec![activation.scopes[0].0]);
    assert_eq!(
        activation
            .completion
            .expanded_name(element(&input, "item"))
            .unwrap()
            .namespace_uri,
        "urn:ready"
    );
    let resolved = resolve_reference(
        host.source_reference(node(&input, seen[0])),
        &mut host,
        policy().limits,
    )
    .unwrap();
    assert_eq!(resolved.state, ReferenceResolutionState::Pending);
    assert_eq!(activation.completion.targets().len(), 1);
    assert!(!activation.completion.contains(element(&input, "outside")));
    assert!(input
        .captured
        .namespace_binding(input.tree.ast_owner(), property(&input))
        .is_none());
}

#[test]
fn missing_and_incomplete_results_reject_before_callbacks_and_can_retry() {
    let input =
        import("@ns public = urn:ready\n{host @xmlns:v={#library} | {v:item | {#library}}}");
    let root = element(&input, "item");
    let mut host = CemQlSchemaDeclarationHost::new();
    let parent = host.register_scope(input.tree.clone(), None, policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let pending = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    let failure = host.activate_namespace_properties(
        input.captured.clone(),
        &[root],
        &[pending],
        |_, _, _, _| panic!("pending callback"),
    );
    assert!(
        matches!(failure, Err(NamespacePropertyActivationError::PropertyNotReady(id)) if id == property(&input))
    );
    let missing =
        host.activate_namespace_properties(input.captured.clone(), &[root], &[], |_, _, _, _| {
            panic!("missing callback")
        });
    assert!(
        matches!(missing, Err(NamespacePropertyActivationError::Namespace(NamespaceNameCompletionError::Pending {declaration, ..})) if declaration == property(&input))
    );
    // A failed activation has not attached occurrence scopes.
    assert!(host.set_context(parent, Some(context(&input))));
    let ready = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    let result = host
        .activate_namespace_properties(input.captured.clone(), &[root], &[ready], |_, _, _, _| {
            (
                Some(context(&input)),
                ReferenceScopePolicyOverrides::default(),
            )
        })
        .unwrap();
    assert_eq!(result.scopes.len(), 1);
}

#[test]
fn duplicate_foreign_and_inconsistent_property_results_reject_before_mutation() {
    let input =
        import("@ns public = urn:ready\n{host @xmlns:v={#library} | {v:item | {#library}}}");
    let foreign =
        import("@ns public = urn:foreign\n{host @xmlns:v={#library} | {v:item | {#library}}}");
    let mut host = host(&input, true);
    let ready = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    let roots = [element(&input, "item")];
    assert!(
        matches!(host.activate_namespace_properties(input.captured.clone(), &roots, &[ready.clone(), ready.clone()], |_, _, _, _| panic!("duplicate")), Err(NamespacePropertyActivationError::DuplicateDeclaration(id)) if id == property(&input))
    );
    let mut changed = ready.clone();
    changed.property.as_mut().unwrap().prefix = "wrong".into();
    assert!(
        matches!(host.activate_namespace_properties(input.captured.clone(), &roots, &[changed], |_, _, _, _| panic!("mismatch")), Err(NamespacePropertyActivationError::PropertyMismatch(id)) if id == property(&input))
    );
    assert!(matches!(
        host.activate_namespace_properties(
            foreign.captured.clone(),
            &[],
            &[ready.clone()],
            |_, _, _, _| panic!("foreign")
        ),
        Err(NamespacePropertyActivationError::OwnerMismatch)
    ));
    let activated = host
        .activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[ready.clone()],
            |_, _, _, _| {
                (
                    Some(context(&input)),
                    ReferenceScopePolicyOverrides::default(),
                )
            },
        )
        .unwrap();
    assert!(
        matches!(host.activate_namespace_properties(input.captured.clone(), &roots, &[ready], |_, _, _, _| panic!("repeat")), Err(NamespacePropertyActivationError::Handoff(NamespaceLexicalScopeHandoffError::Lexical(LexicalScopeHandoffError::OccurrenceAlreadyAssigned(id)))) if id == activated.scopes[0].0)
    );
}

#[test]
fn unused_pending_prefix_blocks_context_activation_even_without_dependent_qnames() {
    let input = import("@ns public = urn:ready\n{host @xmlns:v={#library} | {plain | {#library}}}");
    let mut host = host(&input, true);
    let failure = host.activate_namespace_properties(
        input.captured.clone(),
        &[element(&input, "plain")],
        &[],
        |_, _, _, _| panic!("unused prefix is pending"),
    );
    assert!(
        matches!(failure, Err(NamespacePropertyActivationError::Handoff(NamespaceLexicalScopeHandoffError::Namespace(NamespaceNameCompletionError::Pending {declaration, ..}))) if declaration == property(&input))
    );
    let ready = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(host
        .activate_namespace_properties(
            input.captured.clone(),
            &[element(&input, "plain")],
            &[ready],
            |_, _, _, _| (
                Some(context(&input)),
                ReferenceScopePolicyOverrides::default()
            )
        )
        .is_ok());
}
