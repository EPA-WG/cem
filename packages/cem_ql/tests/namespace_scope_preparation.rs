//! Namespace selection is a consumer stage over original declarations.
use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{NamespaceNameCompletion, NamespaceScopeTargetError},
        reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
    value::reference_resolution::{ReferenceResolutionIssueKind, ReferenceResolutionState},
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{Item, ItemStream, QueryItemView, QueryItemViewKind, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, LexicalScopeHandoffError, NamespaceScopePreparationIssue,
    },
};
use std::{collections::BTreeMap, sync::Arc};
fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "names.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
}
fn ids(source: &ScopedCemImport, local: &str) -> Vec<u32> {
    source
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
fn node(source: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(source.tree.ast_owner().clone(), id).unwrap()
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn query_context(source: &ScopedCemImport, ids: &[u32]) -> StandaloneExpressionContext {
    let items = ids
        .iter()
        .map(|id| {
            RetainedCemNode::new(source.tree.clone(), *id)
                .unwrap()
                .query_item()
        })
        .collect();
    context(ItemStream::from_items(items))
}
fn context(items: ItemStream) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default()
        .with_binding("library", StandaloneExpressionBinding::any(items))
}
#[derive(Debug)]
struct HostNode;
impl QueryItemView for HostNode {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "fixture.host-node"
    }
    fn identity(&self) -> String {
        "host-node".into()
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
}
#[test]
fn selection_admits_original_namespace_declarations_and_completes_original_uses() {
    let source = import("@ns public = urn:first\n{host @xmlns:v={#library} | {v:item}}\n@ns public = urn:later\n{#library}");
    let targets = ids(&source, "@ns");
    let reference = source.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(
        source.tree.clone(),
        Some(query_context(&source, &[targets[0]])),
        policy(),
    );
    host.attach_captured_namespaces(source.captured.clone())
        .unwrap();
    host.attach_captured_namespaces(source.captured.clone())
        .unwrap();
    let result = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(result.is_ready(), "{:?}", result.issue);
    let target = result.target.unwrap();
    assert_eq!(target.selected.node_id(), targets[0]);
    assert_eq!(target.binding().name, "public");
    assert_eq!(target.namespace_uri(), "urn:first");
    assert!(Arc::ptr_eq(
        target.selected.document(),
        source.tree.ast_owner()
    ));
    let declaration = source
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Attribute {
                node_id,
                value_nodes,
                ..
            } if !value_nodes.is_empty() => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let item = ids(&source, "item")[0];
    let completion = NamespaceNameCompletion::new(
        source.captured.clone(),
        &[item],
        BTreeMap::from([(declaration, target)]),
    )
    .unwrap();
    assert_eq!(
        completion.expanded_name(item).unwrap().namespace_uri,
        "urn:first"
    );
    host.set_context(scope, Some(query_context(&source, &[targets[1]])));
    let other = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert_eq!(other.target.unwrap().namespace_uri(), "urn:later");
    assert_eq!(
        completion.expanded_name(item).unwrap().namespace_uri,
        "urn:first"
    );
    assert!(matches!(
        source.tree.ast().get(reference),
        Some(CemAstNode::Reference { targets: None, .. })
    ));
}
#[test]
fn missing_context_metadata_and_pending_declarations_remain_inspectable() {
    let source = import("@ns public = urn:first\n{host @xmlns:v={#library} | {v:item}} {#library}");
    let reference = source.captured.occurrences().last().unwrap();
    let target = ids(&source, "@ns")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    assert_eq!(
        host.attach_captured_namespaces(source.captured.clone()),
        Err(LexicalScopeHandoffError::UnregisteredOwner)
    );
    let scope = host.register_scope(source.tree.clone(), None, policy());
    let pending = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(pending.selection.state, ReferenceResolutionState::Pending);
    host.set_context(scope, Some(query_context(&source, &[target])));
    let pending = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert_eq!(
        pending.issue,
        Some(NamespaceScopePreparationIssue::TargetMetadataNotReady)
    );
    host.attach_captured_namespaces(source.captured.clone())
        .unwrap();
    let declaration = source
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Attribute {
                node_id,
                value_nodes,
                ..
            } if !value_nodes.is_empty() => Some(*node_id),
            _ => None,
        })
        .unwrap();
    host.set_context(scope, Some(query_context(&source, &[declaration])));
    let pending = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert_eq!(
        pending.issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    assert!(pending.selection.is_complete());
    assert_eq!(pending.selection.nodes.len(), 1);
    assert!(pending.target.is_none() && !pending.is_ready());
    let foreign =
        import("@ns public = urn:first\n{host @xmlns:v={#library} | {v:item}} {#library}");
    assert_eq!(
        host.attach_captured_namespaces(foreign.captured),
        Err(LexicalScopeHandoffError::UnregisteredOwner)
    );
}
#[test]
fn empty_default_is_ready_but_cardinality_and_other_kinds_are_rejected() {
    let source = import("@default \"\"\n{schema @namespace=urn:fake} {data} {#library}");
    let reference = source.captured.occurrences().next().unwrap();
    let reset = ids(&source, "@default")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(
        source.tree.clone(),
        Some(query_context(&source, &[reset])),
        policy(),
    );
    host.attach_captured_namespaces(source.captured.clone())
        .unwrap();
    let ready = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(ready.is_ready());
    assert_eq!(ready.target.unwrap().namespace_uri(), "");
    for chosen in [vec![], vec![reset, reset]] {
        host.set_context(scope, Some(query_context(&source, &chosen)));
        let result = host
            .prepare_namespace_scope(node(&source, reference), policy().limits)
            .unwrap();
        assert_eq!(
            result.issue,
            Some(NamespaceScopePreparationIssue::TargetCount(chosen.len()))
        );
        assert!(!result.is_ready() && result.target.is_none());
    }
    for local in ["schema", "data"] {
        host.set_context(scope, Some(query_context(&source, &ids(&source, local))));
        let result = host
            .prepare_namespace_scope(node(&source, reference), policy().limits)
            .unwrap();
        assert_eq!(
            result.issue,
            Some(NamespaceScopePreparationIssue::TargetAdmission(
                NamespaceScopeTargetError::NotNamespaceDeclaration
            ))
        );
        assert!(!result.is_ready() && result.target.is_none());
    }
}
#[test]
fn granted_target_context_and_destination_budget_are_required() {
    let source = import("{#library}");
    let vendor = import("@ns public = urn:vendor\n{#library}");
    let target = ids(&vendor, "@ns")[0];
    let reference = source.captured.occurrences().next().unwrap();
    let link = vendor.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        source.tree.clone(),
        Some(query_context(&vendor, &[target])),
        policy(),
    );
    let mut destination_policy = policy();
    destination_policy.limits.max_work = 1;
    let destination = host.register_scope(vendor.tree.clone(), None, destination_policy);
    host.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    let denied = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(!denied.is_ready());
    assert!(denied
        .selection
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::ScopeDenied
            && i.occurrence.source_map.origin().is_some()));
    assert!(host.allow_scope_crossing(origin, destination));
    let pending = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert_eq!(
        pending.issue,
        Some(NamespaceScopePreparationIssue::TargetContextNotReady)
    );
    assert!(pending.target.is_some() && !pending.is_ready());
    host.set_context(destination, Some(query_context(&vendor, &[target])));
    assert!(host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap()
        .is_ready());
    host.set_context(origin, Some(query_context(&vendor, &[link])));
    let limited = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(!limited.is_ready());
    assert!(limited
        .selection
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::WorkLimit));
    host.set_context(origin, Some(query_context(&vendor, &[target])));
    let mut limits = policy().limits;
    limits.max_work = 1;
    let limited = host
        .prepare_namespace_scope(node(&source, reference), limits)
        .unwrap();
    assert!(!limited.is_ready());
    assert!(limited
        .selection
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::WorkLimit));
    // Permission is directed: destination evaluation cannot use the source scope.
    let reverse_scope = host.register_scope(
        vendor.tree.clone(),
        Some(query_context(&source, &[reference])),
        policy(),
    );
    assert!(host.assign_subtree_scope(&vendor.tree, link, reverse_scope));
    assert!(host.allow_scope_crossing(origin, reverse_scope));
    host.set_context(origin, Some(query_context(&vendor, &[link])));
    host.set_context(reverse_scope, Some(query_context(&vendor, &[target])));
    let mut depth_limits = policy().limits;
    depth_limits.max_depth = 1;
    let depth_limited = host
        .prepare_namespace_scope(node(&source, reference), depth_limits)
        .unwrap();
    assert!(!depth_limited.is_ready());
    assert!(depth_limited
        .selection
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::DepthLimit));
    host.set_context(reverse_scope, Some(query_context(&source, &[reference])));
    host.set_context(origin, Some(query_context(&vendor, &[link])));
    let reverse = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(!reverse.is_ready());
    assert!(reverse
        .selection
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::ScopeDenied));
}
#[test]
fn unregistered_source_and_non_source_native_targets_do_not_admit() {
    let source = import("{#library}");
    let reference = source.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let unresolved = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert_eq!(
        unresolved.selection.state,
        ReferenceResolutionState::Unresolved
    );
    let scope = host.register_scope(
        source.tree.clone(),
        Some(context(ItemStream::once(Item::native(HostNode)))),
        policy(),
    );
    let invalid = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert_eq!(
        invalid.issue,
        Some(NamespaceScopePreparationIssue::TargetHasNoSourceHandle)
    );
    assert!(!invalid.is_ready());
    host.set_context(scope, Some(query_context(&source, &[reference])));
    let cycle = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(!cycle.is_ready());
    assert!(cycle
        .selection
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::Cycle));
}

#[test]
fn xml_namespace_attributes_use_decoded_binding_and_original_source_handle() {
    let source = import("{#library}");
    let vendor = import_bytes_with_lexical_scopes(
        b"<root xmlns:v='urn:a&amp;b'><v:item/></root>",
        "application/xml",
        "vendor.xml",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let declaration = vendor
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Attribute { node_id, .. }
                if vendor
                    .captured
                    .namespace_binding(vendor.tree.ast_owner(), *node_id)
                    .is_some() =>
            {
                Some(*node_id)
            }
            _ => None,
        })
        .unwrap();
    let reference = source.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        source.tree.clone(),
        Some(query_context(&vendor, &[declaration])),
        policy(),
    );
    let destination = host.register_scope(
        vendor.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    assert!(host.allow_scope_crossing(origin, destination));
    let ready = host
        .prepare_namespace_scope(node(&source, reference), policy().limits)
        .unwrap();
    assert!(ready.is_ready(), "{:?}", ready.issue);
    let target = ready.target.unwrap();
    assert_eq!(target.namespace_uri(), "urn:a&b");
    assert_eq!(target.selected.node_id(), declaration);
    assert!(Arc::ptr_eq(
        target.selected.document(),
        vendor.tree.ast_owner()
    ));
    assert!(matches!(
        target.selected.node(),
        CemAstNode::Attribute { .. }
    ));
    assert!(target.binding().source_map.origin().is_some());
    assert_eq!(ready.selection.work_used, 2);
}
