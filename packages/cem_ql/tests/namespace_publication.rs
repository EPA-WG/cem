use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode, reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, NamespacePublicationError, NamespaceScopePreparationIssue,
    },
};
use std::sync::Arc;
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "properties.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn elements(input: &ScopedCemImport, local: &str) -> Vec<u32> {
    input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => Some(*node_id),
            _ => None,
        })
        .collect()
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
fn node(input: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(input.tree.ast_owner().clone(), id).unwrap()
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn context(input: &ScopedCemImport, ids: &[u32]) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::from_items(
            ids.iter()
                .map(|id| {
                    RetainedCemNode::new(input.tree.clone(), *id)
                        .unwrap()
                        .query_item()
                })
                .collect(),
        )),
    )
}

fn fixture() -> ScopedCemImport {
    import("@ns public = urn:first\n{host @xmlns:v={#library} | {v:item}} {#reuse}")
}
fn host(
    input: &ScopedCemImport,
) -> (
    CemQlSchemaDeclarationHost,
    cem_ql::schema_references::DeclarationScope,
) {
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(input.tree.clone(), Some(inputs(input)), policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    (host, scope)
}
fn inputs(input: &ScopedCemImport) -> StandaloneExpressionContext {
    context(input, &elements(input, "@ns")).with_binding(
        "reuse",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![RetainedCemNode::new(
            input.tree.clone(),
            property(input),
        )
        .unwrap()
        .query_item()])),
    )
}
fn select(
    host: &mut CemQlSchemaDeclarationHost,
    input: &ScopedCemImport,
) -> cem_ql::schema_references::NamespaceScopePreparation {
    host.prepare_namespace_scope(
        node(input, input.captured.occurrences().last().unwrap()),
        policy().limits,
    )
    .unwrap()
}
#[test]
fn published_binding_is_reusable_without_mutating_source_or_reexecuting_slots() {
    let input = fixture();
    let (mut host, _) = host(&input);
    assert_eq!(
        select(&mut host, &input).issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    let report = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    host.publish_namespace_property(&report).unwrap();
    host.publish_namespace_property(&report).unwrap();
    for _ in 0..2 {
        let ready = select(&mut host, &input);
        assert!(ready.is_ready(), "{:?}", ready.issue);
        let target = ready.target.unwrap();
        assert_eq!(target.selected.node_id(), property(&input));
        assert_eq!(target.namespace_uri(), "urn:first");
        assert_eq!(
            target.binding_declaration().node_id(),
            elements(&input, "@ns")[0]
        );
        assert!(Arc::ptr_eq(
            target.selected.document(),
            input.tree.ast_owner()
        ));
    }
    assert!(input
        .captured
        .namespace_binding(input.tree.ast_owner(), property(&input))
        .is_none());
    assert!(input.tree.ast().nodes.iter().all(|n| !matches!(
        n,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}
#[test]
fn publications_and_reports_cannot_cross_execution_or_changed_inputs() {
    let input = fixture();
    let (mut first, scope) = host(&input);
    let report = first
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    first.publish_namespace_property(&report).unwrap();
    let (mut second, _) = host(&input);
    assert_eq!(
        second.publish_namespace_property(&report),
        Err(NamespacePublicationError::DifferentSnapshot)
    );
    assert_eq!(
        select(&mut second, &input).issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    first.set_context(scope, Some(inputs(&input)));
    assert_eq!(
        select(&mut first, &input).issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    assert_eq!(
        first.publish_namespace_property(&report),
        Err(NamespacePublicationError::DifferentSnapshot)
    );
    let fresh = first
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    first.publish_namespace_property(&fresh).unwrap();
    assert!(select(&mut first, &input).is_ready());
    let new_scope = first
        .register_lexical_scope(scope, Some(inputs(&input)), policy())
        .unwrap();
    first.assign_subtree_scope(&input.tree, elements(&input, "host")[0], new_scope);
    assert_eq!(
        select(&mut first, &input).issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    assert_eq!(
        first.publish_namespace_property(&fresh),
        Err(NamespacePublicationError::DifferentSnapshot)
    );
}
#[test]
fn pending_and_edited_reports_cannot_publish_and_empty_uri_is_ready() {
    let input = fixture();
    let (mut host, scope) = host(&input);
    host.set_context(scope, None);
    let pending = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert_eq!(
        host.publish_namespace_property(&pending),
        Err(NamespacePublicationError::NotReady)
    );
    host.set_context(scope, Some(inputs(&input)));
    let mut report = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    report.preparation.as_mut().unwrap().target = None;
    assert_eq!(
        host.publish_namespace_property(&report),
        Err(NamespacePublicationError::NotReady)
    );
    let reset = import("@default \"\"\n{host @xmlns={#library} | {item}} {#reuse}");
    let mut reset_host = CemQlSchemaDeclarationHost::new();
    let reset_inputs = context(&reset, &elements(&reset, "@default")).with_binding(
        "reuse",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![RetainedCemNode::new(
            reset.tree.clone(),
            property(&reset),
        )
        .unwrap()
        .query_item()])),
    );
    reset_host.register_scope(reset.tree.clone(), Some(reset_inputs), policy());
    reset_host
        .attach_captured_namespaces(reset.captured.clone())
        .unwrap();
    let report = reset_host
        .prepare_namespace_property(node(&reset, property(&reset)), policy().limits)
        .unwrap();
    reset_host.publish_namespace_property(&report).unwrap();
    let ready = select(&mut reset_host, &reset);
    assert!(ready.is_ready());
    assert_eq!(ready.target.unwrap().namespace_uri(), "");
}
#[test]
fn publication_does_not_grant_crossings_or_restart_consuming_budget() {
    let input = fixture();
    let (mut host, scope) = host(&input);
    let caller = import("{#reuse}");
    let caller_scope = host.register_scope(caller.tree.clone(), Some(inputs(&input)), policy());
    let report = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    host.publish_namespace_property(&report).unwrap();
    let reference = node(&caller, caller.captured.occurrences().next().unwrap());
    assert!(!host
        .prepare_namespace_scope(reference.clone(), policy().limits)
        .unwrap()
        .is_ready());
    host.allow_scope_crossing(caller_scope, scope);
    assert!(host
        .prepare_namespace_scope(reference.clone(), policy().limits)
        .unwrap()
        .is_ready());
    let mut limits = policy().limits;
    limits.max_work = 1;
    assert!(!host
        .prepare_namespace_scope(reference, limits)
        .unwrap()
        .is_ready());
}

#[test]
fn chained_publications_preserve_transitive_binding_dependencies() {
    let input = import(
        "@ns public = urn:first\n{host @xmlns:v={#library} @xmlns:w={#reuse} | {w:item}} {#final}",
    );
    let properties: Vec<_> = input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Attribute {
                node_id,
                value_nodes,
                ..
            } if !value_nodes.is_empty() => Some(*node_id),
            _ => None,
        })
        .collect();
    let mut host = CemQlSchemaDeclarationHost::new();
    let inputs = inputs(&input).with_binding(
        "final",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![RetainedCemNode::new(
            input.tree.clone(),
            properties[1],
        )
        .unwrap()
        .query_item()])),
    );
    let scope = host.register_scope(input.tree.clone(), Some(inputs.clone()), policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let first = host
        .prepare_namespace_property(node(&input, properties[0]), policy().limits)
        .unwrap();
    host.publish_namespace_property(&first).unwrap();
    let second = host
        .prepare_namespace_property(node(&input, properties[1]), policy().limits)
        .unwrap();
    assert!(second.is_ready());
    host.publish_namespace_property(&second).unwrap();
    let ready = select(&mut host, &input);
    assert!(ready.is_ready());
    assert_eq!(
        ready.target.unwrap().binding_declaration().node_id(),
        elements(&input, "@ns")[0]
    );
    // Handoff of unrelated occurrences does not replace the inputs that produced
    // these bindings. Reassigning the first selector does invalidate both results.
    let new_scope = host
        .register_lexical_scope(scope, Some(inputs), policy())
        .unwrap();
    host.assign_subtree_scope(&input.tree, elements(&input, "item")[0], new_scope);
    assert!(select(&mut host, &input).is_ready());
    let value = first.property.as_ref().unwrap().value.node_id();
    host.assign_subtree_scope(&input.tree, value, new_scope);
    assert_eq!(
        select(&mut host, &input).issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    assert_eq!(
        host.publish_namespace_property(&second),
        Err(NamespacePublicationError::DifferentSnapshot)
    );
    let fresh = host
        .prepare_namespace_property(node(&input, properties[0]), policy().limits)
        .unwrap();
    host.publish_namespace_property(&fresh).unwrap();
    let fresh = host
        .prepare_namespace_property(node(&input, properties[1]), policy().limits)
        .unwrap();
    host.publish_namespace_property(&fresh).unwrap();
    assert!(select(&mut host, &input).is_ready());
}
#[test]
fn edited_ready_result_is_rejected_and_effective_scope_budget_still_applies() {
    let input = fixture();
    let (mut host, scope) = host(&input);
    let report = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    let mut edited = report.clone();
    edited.property.as_mut().unwrap().prefix = "changed".into();
    assert!(edited.is_ready());
    assert_eq!(
        host.publish_namespace_property(&edited),
        Err(NamespacePublicationError::ReportMismatch)
    );
    host.publish_namespace_property(&report).unwrap();
    let caller = import("{#reuse}");
    let mut strict = policy();
    strict.limits.max_work = 1;
    let caller_scope = host.register_scope(caller.tree.clone(), Some(inputs(&input)), strict);
    host.allow_scope_crossing(caller_scope, scope);
    let reference = node(&caller, caller.captured.occurrences().next().unwrap());
    assert!(!host
        .prepare_namespace_scope(reference, policy().limits)
        .unwrap()
        .is_ready());
}

#[test]
fn reused_declaration_honors_stricter_destination_limits() {
    let input = fixture();
    let (mut host, scope) = host(&input);
    let mut strict = policy();
    strict.limits.max_work = 1;
    let destination = host
        .register_lexical_scope(scope, Some(inputs(&input)), strict)
        .unwrap();
    host.assign_subtree_scope(&input.tree, property(&input), destination);
    let value = input.captured.occurrences().next().unwrap();
    host.assign_subtree_scope(&input.tree, value, scope);
    let report = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    host.publish_namespace_property(&report).unwrap();
    let caller = import("{#(reuse, reuse)}");
    let caller_scope = host.register_scope(caller.tree.clone(), Some(inputs(&input)), policy());
    host.allow_scope_crossing(caller_scope, scope);
    let reference = node(&caller, caller.captured.occurrences().next().unwrap());
    let result = host
        .prepare_namespace_scope(reference, policy().limits)
        .unwrap();
    assert!(!result.selection.is_complete());
    assert!(result.selection.issues.iter().any(|issue| issue.kind
        == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::WorkLimit));
}

#[test]
fn publication_keeps_foreign_binding_provider_in_its_original_owner() {
    let input = fixture();
    let vendor = import("@ns public = urn:vendor");
    let mut host = CemQlSchemaDeclarationHost::new();
    let inputs = context(&vendor, &elements(&vendor, "@ns")).with_binding(
        "reuse",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![RetainedCemNode::new(
            input.tree.clone(),
            property(&input),
        )
        .unwrap()
        .query_item()])),
    );
    let source_scope = host.register_scope(input.tree.clone(), Some(inputs), policy());
    let vendor_scope = host.register_scope(
        vendor.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    host.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    host.allow_scope_crossing(source_scope, vendor_scope);
    let report = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    host.publish_namespace_property(&report).unwrap();
    let ready = select(&mut host, &input);
    assert!(ready.is_ready());
    let target = ready.target.unwrap();
    assert!(Arc::ptr_eq(
        target.selected.document(),
        input.tree.ast_owner()
    ));
    assert!(Arc::ptr_eq(
        target.binding_declaration().document(),
        vendor.tree.ast_owner()
    ));
    assert_eq!(target.namespace_uri(), "urn:vendor");
}
