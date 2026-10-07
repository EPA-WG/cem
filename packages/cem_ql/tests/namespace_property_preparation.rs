use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{
            NamespaceNameCompletion, NamespaceScopeTargetError, NativeNamespacePropertyError,
        },
        reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
    value::reference_resolution::{ReferenceResolutionIssueKind, ReferenceResolutionState},
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, NamespacePropertyPreparationIssue,
        NamespaceScopePreparationIssue,
    },
};
use std::{collections::BTreeMap, sync::Arc};
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

#[test]
fn property_consumption_uses_original_pre_declaration_context_and_independent_results() {
    for expression in ["#library", "library", "(library, #())"] {
        let input = import(&format!("@ns v = urn:outer\n@ns public = urn:first\n{{host @xmlns:v={{{expression}}} | {{v:item}}}}\n@ns public = urn:later\n"));
        let targets = elements(&input, "@ns");
        let value = input.captured.occurrences().next().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            input.tree.clone(),
            Some(StandaloneExpressionContext::default()),
            policy(),
        );
        host.attach_captured_namespaces(input.captured.clone())
            .unwrap();
        let scopes = host
            .attach_captured_lexical_scopes(&input.captured, |source, snapshot, _| {
                assert_eq!(source.node_id(), value);
                assert_eq!(
                    snapshot.namespaces.binding("v").unwrap().namespace_uri,
                    "urn:outer"
                );
                (Some(context(&input, &[targets[1]])), policy())
            })
            .unwrap();
        assert!(host
            .compiled_source_expression(&node(&input, value))
            .is_none());
        let prepared = host
            .prepare_namespace_property(node(&input, property(&input)), policy().limits)
            .unwrap();
        assert!(
            prepared.is_ready(),
            "property: {:?}, admission: {:?}",
            prepared.issue,
            prepared
                .preparation
                .as_ref()
                .and_then(|result| result.issue)
        );
        let original = prepared.property.as_ref().unwrap();
        assert_eq!(original.prefix, "v");
        assert_eq!(original.value.node_id(), value);
        assert!(Arc::ptr_eq(
            original.declaration.document(),
            input.tree.ast_owner()
        ));
        let target = prepared
            .preparation
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap();
        assert_eq!(target.namespace_uri(), "urn:first");
        assert_eq!(target.binding().name, "public");
        let completed = NamespaceNameCompletion::new(
            input.captured.clone(),
            &[elements(&input, "item")[0]],
            BTreeMap::from([(property(&input), target.clone())]),
        )
        .unwrap();
        assert!(host.set_context(scopes[0].1, Some(context(&input, &[targets[2]]))));
        let later = host
            .prepare_namespace_property(node(&input, property(&input)), policy().limits)
            .unwrap();
        assert!(later.is_ready());
        assert_eq!(
            later.preparation.unwrap().target.unwrap().namespace_uri(),
            "urn:later"
        );
        assert_eq!(
            completed
                .expanded_name(elements(&input, "item")[0])
                .unwrap()
                .namespace_uri,
            "urn:first"
        );
        assert!(input
            .captured
            .namespace_binding(input.tree.ast_owner(), property(&input))
            .is_none());
        assert!(input.tree.ast().nodes.iter().all(|node| !matches!(
            node,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
    }
}

#[test]
fn absent_metadata_context_and_invalid_property_shape_do_not_evaluate_or_fallback() {
    let input = import("@ns v = urn:outer\n@ns public = urn:ready\n{host @xmlns:v={#library} @target={#library} | {v:item}}");
    let value = input.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let missing = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert_eq!(
        missing.issue,
        Some(NamespacePropertyPreparationIssue::MetadataNotReady)
    );
    assert!(missing.preparation.is_none());
    let scope = host.register_scope(input.tree.clone(), None, policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let pending = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(
        pending.preparation.unwrap().selection.state,
        ReferenceResolutionState::Pending
    );
    assert!(host
        .compiled_source_expression(&node(&input, value))
        .is_none());
    host.set_context(scope, Some(context(&input, &[elements(&input, "@ns")[1]])));
    assert!(host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap()
        .is_ready());
    let ordinary = input
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "target" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let rejected = host
        .prepare_namespace_property(node(&input, ordinary), policy().limits)
        .unwrap();
    assert_eq!(
        rejected.issue,
        Some(NamespacePropertyPreparationIssue::Property(
            NativeNamespacePropertyError::NotNativeNamespaceDeclaration
        ))
    );
    assert!(rejected.preparation.is_none());
    let ordinary_value = input.captured.occurrences().last().unwrap();
    assert!(host
        .compiled_source_expression(&node(&input, ordinary_value))
        .is_none());
}

#[test]
fn general_expression_scalar_errors_keep_original_diagnostics_and_do_not_supply_a_uri() {
    let input = import("{host @xmlns:v={\"urn:fake\"} | {v:item}}");
    let value = input.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        input.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let result = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(!result.is_ready());
    let selection = result.preparation.unwrap().selection;
    assert_eq!(selection.state, ReferenceResolutionState::Invalid);
    assert!(selection.failed);
    let diagnostic = selection
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == cem_ml::schema::attribute_references::INVALID_NATIVE_TARGET
        })
        .unwrap();
    assert_eq!(diagnostic.uri.as_deref(), Some("properties.cem"));
    assert_eq!(
        diagnostic.node.as_deref(),
        Some(node(&input, value).identity().as_str())
    );
    assert!(!diagnostic.source_map.as_ref().unwrap().frames.is_empty());
}

#[test]
fn singleton_admission_rejects_empty_repeated_and_non_namespace_targets() {
    let input = import("@ns public = urn:ready\n{schema @namespace=urn:fake} {data} {host @xmlns:v={library} | {v:item}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(input.tree.clone(), None, policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let target = elements(&input, "@ns")[0];
    for ids in [vec![], vec![target, target]] {
        host.set_context(scope, Some(context(&input, &ids)));
        let result = host
            .prepare_namespace_property(node(&input, property(&input)), policy().limits)
            .unwrap();
        assert!(!result.is_ready());
        assert_eq!(
            result.preparation.unwrap().issue,
            Some(NamespaceScopePreparationIssue::TargetCount(ids.len()))
        );
    }
    for local in ["schema", "data"] {
        host.set_context(scope, Some(context(&input, &elements(&input, local))));
        let result = host
            .prepare_namespace_property(node(&input, property(&input)), policy().limits)
            .unwrap();
        assert!(!result.is_ready());
        assert_eq!(
            result.preparation.unwrap().issue,
            Some(NamespaceScopePreparationIssue::TargetAdmission(
                NamespaceScopeTargetError::NotNamespaceDeclaration
            ))
        );
    }
}

#[test]
fn crossings_contexts_and_request_destination_limits_remain_required() {
    let input = import("{host @xmlns:v={#library} | {v:item}}");
    let vendor = import("@ns public = urn:vendor\n{#library}");
    let target = elements(&vendor, "@ns")[0];
    let link = vendor.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        input.tree.clone(),
        Some(context(&vendor, &[target])),
        policy(),
    );
    let mut destination_policy = policy();
    destination_policy.limits.max_work = 1;
    let destination = host.register_scope(vendor.tree.clone(), None, destination_policy);
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    host.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    let denied = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(!denied.is_ready());
    assert!(denied
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(
            |issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied
                && !issue.occurrence.source_map.frames.is_empty()
        ));
    assert!(host.allow_scope_crossing(origin, destination));
    let pending = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert_eq!(
        pending.preparation.unwrap().issue,
        Some(NamespaceScopePreparationIssue::TargetContextNotReady)
    );
    host.set_context(destination, Some(context(&vendor, &[target])));
    let ready = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(ready.is_ready());
    assert_eq!(ready.preparation.unwrap().selection.work_used, 2);
    let mut limits = policy().limits;
    limits.max_work = 1;
    let bounded = host
        .prepare_namespace_property(node(&input, property(&input)), limits)
        .unwrap();
    assert!(bounded
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
    host.set_context(origin, Some(context(&vendor, &[link])));
    let bounded = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(bounded
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(
            |issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit
                && issue.reason == "scope-work-limit"
        ));
    limits = policy().limits;
    limits.max_depth = 1;
    let bounded = host
        .prepare_namespace_property(node(&input, property(&input)), limits)
        .unwrap();
    assert!(bounded
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::DepthLimit));
}

#[test]
fn pending_namespace_targets_and_general_root_cycles_stay_incomplete() {
    let input = import("{host @xmlns:v={library} | {v:item}}");
    let value = input.captured.occurrences().next().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(
        input.tree.clone(),
        Some(context(&input, &[property(&input)])),
        policy(),
    );
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let pending = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(
        pending.preparation.unwrap().issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    host.set_context(scope, Some(context(&input, &[value])));
    let cycle = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(!cycle.is_ready());
    assert!(cycle
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::Cycle
            && issue.occurrence.node_id == Some(value)));
}

#[test]
fn native_default_property_can_consume_a_ready_empty_namespace_reset() {
    let input = import("@default \"\"\n{host @xmlns={#library} | {item}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        input.tree.clone(),
        Some(context(&input, &[elements(&input, "@default")[0]])),
        policy(),
    );
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let result = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(result.is_ready());
    assert_eq!(result.property.unwrap().prefix, "");
    let target = result.preparation.unwrap().target.unwrap();
    assert_eq!(target.namespace_uri(), "");
    let names = NamespaceNameCompletion::new(
        input.captured.clone(),
        &[elements(&input, "item")[0]],
        BTreeMap::from([(property(&input), target)]),
    )
    .unwrap();
    assert_eq!(
        names
            .expanded_name(elements(&input, "item")[0])
            .unwrap()
            .namespace_uri,
        ""
    );
}

#[test]
fn selected_general_expression_targets_are_not_implicitly_executed() {
    let input = import("{host @xmlns:v={library} @value={missing()} | {v:item}}");
    let selected = input.captured.occurrences().last().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        input.tree.clone(),
        Some(context(&input, &[selected])),
        policy(),
    );
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    let prepared = host
        .prepare_namespace_property(node(&input, property(&input)), policy().limits)
        .unwrap();
    assert!(!prepared.is_ready());
    let result = prepared.preparation.unwrap();
    assert!(result.selection.is_complete());
    assert_eq!(result.selection.work_used, 2);
    assert_eq!(
        result.issue,
        Some(NamespaceScopePreparationIssue::TargetAdmission(
            NamespaceScopeTargetError::NotNamespaceDeclaration
        ))
    );
    assert!(host
        .compiled_source_expression(&node(&input, selected))
        .is_none());
}
