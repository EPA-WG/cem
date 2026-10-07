use cem_ml::{
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{admit_namespace_scope_target, NamespaceNameCompletion},
        reference_policy::ReferenceScopePolicy,
        scope_references::SchemaScopeTargetError,
        vocab::CompiledSchema,
    },
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, LexicalScopeHandoffError, SchemaScopePreparationIssue,
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
fn node(input: &ScopedCemImport, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(input.tree.ast_owner().clone(), id).unwrap()
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
fn completion(input: &ScopedCemImport, roots: &[u32], uri: &str) -> Arc<NamespaceNameCompletion> {
    let vendor = import(&format!("@ns public = {uri}\n"));
    let target =
        admit_namespace_scope_target(node(&vendor, elements(&vendor, "@ns")[0]), &vendor.captured)
            .unwrap();
    Arc::new(
        NamespaceNameCompletion::new(
            input.captured.clone(),
            roots,
            BTreeMap::from([(property(input), target)]),
        )
        .unwrap(),
    )
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
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
fn host(input: &ScopedCemImport, target: u32) -> CemQlSchemaDeclarationHost {
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), Some(context(input, target)), policy());
    host.attach_captured_names(&input.captured).unwrap();
    host
}

#[test]
fn completed_schema_target_names_are_invocation_local_and_original_names_stay_fixed() {
    let input = import("@ns p = urn:fixed\n{p:fixed}\n{container @xmlns:p={#namespace} | {p:schema | {elements | {element @name=child}}} {p:schema}} {#library}");
    let target = elements(&input, "schema")[0];
    let outside = elements(&input, "schema")[1];
    let reference = input.captured.occurrences().last().unwrap();
    let mut host = host(&input, target);
    assert!(matches!(
        host.prepare_schema_scope("selected", node(&input, reference), policy().limits)
            .unwrap()
            .issue,
        Some(SchemaScopePreparationIssue::TargetAdmission(
            SchemaScopeTargetError::NameNotReady
        ))
    ));
    host.with_completed_namespace_names(
        completion(
            &input,
            &[elements(&input, "fixed")[0]],
            "https://cem.dev/ns/schema/1",
        ),
        |host| {
            let excluded = host
                .prepare_schema_scope("selected", node(&input, reference), policy().limits)
                .unwrap();
            assert_eq!(
                excluded.issue,
                Some(SchemaScopePreparationIssue::TargetAdmission(
                    SchemaScopeTargetError::NameNotReady
                ))
            );
        },
    )
    .unwrap();
    let names = completion(
        &input,
        &[target, elements(&input, "fixed")[0]],
        "https://cem.dev/ns/schema/1",
    );
    for uri in ["https://cem.dev/ns/schema/1", "urn:foreign"] {
        let current = completion(&input, &[target, elements(&input, "fixed")[0]], uri);
        let result = host
            .with_completed_namespace_names(current, |host| {
                assert!(host.captured_expanded_name(&node(&input, target)).is_none());
                assert!(host
                    .consuming_expanded_name(&node(&input, outside))
                    .is_none());
                assert_eq!(
                    host.consuming_expanded_name(&node(&input, elements(&input, "fixed")[0]))
                        .unwrap()
                        .namespace_uri,
                    "urn:fixed"
                );
                host.prepare_schema_scope("selected", node(&input, reference), policy().limits)
                    .unwrap()
            })
            .unwrap();
        if uri == "urn:foreign" {
            assert_eq!(
                result.issue,
                Some(SchemaScopePreparationIssue::TargetAdmission(
                    SchemaScopeTargetError::InvalidKindOrName
                ))
            );
        } else {
            assert!(result.is_ready(), "{:?}", result.issue);
            assert!(result.model.unwrap().element("child").is_some());
            assert!(Arc::ptr_eq(
                result.target.unwrap().declaration.document(),
                input.tree.ast_owner()
            ));
        }
        assert!(host
            .consuming_expanded_name(&node(&input, target))
            .is_none());
    }
    assert_eq!(
        names.expanded_name(target).unwrap().namespace_uri,
        "https://cem.dev/ns/schema/1"
    );
    assert!(input
        .captured
        .expanded_name(input.tree.ast_owner(), target)
        .is_none());
    assert!(
        matches!(node(&input, target).node(), CemAstNode::Element {expanded_name, ..} if expanded_name.namespace_uri == "p")
    );
}

#[test]
fn completed_core_control_aliases_discover_body_and_following_forms_and_restore() {
    use cem_ml::schema::scope_controls::SchemaScopeControlExtent;
    let input = import("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child}}}\n{container @xmlns:c={#namespace} | {host @c:schema-select={library} | {child}} {c:schema @select=library} {child}}");
    let target = elements(&input, "schema")[0];
    let switch = elements(&input, "schema")[1];
    let body = elements(&input, "host")[0];
    let mut host = host(&input, target);
    let roots = [body, switch];
    for uri in ["https://cem.dev/ns/core/1", "urn:foreign"] {
        host.with_completed_namespace_names(completion(&input, &roots, uri), |host| {
            let body = host
                .prepare_schema_host_region("selected", node(&input, body), policy().limits)
                .unwrap();
            assert_eq!(body.contract.has_override(), uri != "urn:foreign");
            if uri != "urn:foreign" {
                assert!(body.is_ready());
                assert_eq!(body.contract.extent(), SchemaScopeControlExtent::Body);
            }
            let switched = host
                .prepare_schema_host_control("selected", node(&input, switch), policy().limits)
                .unwrap();
            assert!(switched.is_none()); // direct host API is distinct from shared wrapper discovery
            let region = host
                .validate_input_runtime_host_regions(
                    "selected",
                    input.tree.clone(),
                    &[switch],
                    &Default::default(),
                    policy().limits,
                    |_| Some(context(&input, target)),
                )
                .unwrap();
            assert_eq!(region.inputs.len(), usize::from(uri != "urn:foreign"));
            if uri != "urn:foreign" {
                assert_eq!(
                    region.inputs[0].region().contract.extent(),
                    SchemaScopeControlExtent::Following
                );
                assert!(region.inputs[0].is_ready());
            }
        })
        .unwrap();
        assert!(host
            .consuming_expanded_name(&node(&input, switch))
            .is_none());
    }
}

#[test]
fn completion_owner_admission_and_nested_invocation_unwind_leave_no_name_leaks() {
    let input = import("{host @xmlns:p={#namespace} | {p:schema}} {#library}");
    let foreign = import("{host @xmlns:p={#namespace} | {p:schema}} {#library}");
    let target = elements(&input, "schema")[0];
    let mut host = CemQlSchemaDeclarationHost::new();
    let names = completion(&input, &[target], "https://cem.dev/ns/schema/1");
    assert!(matches!(
        host.with_completed_namespace_names(names.clone(), |_| panic!("unregistered")),
        Err(LexicalScopeHandoffError::UnregisteredOwner)
    ));
    host.register_scope(input.tree.clone(), Some(context(&input, target)), policy());
    assert!(matches!(
        host.with_completed_namespace_names(
            completion(&foreign, &[elements(&foreign, "schema")[0]], "urn:foreign"),
            |_| panic!("wrong owner")
        ),
        Err(LexicalScopeHandoffError::UnregisteredOwner)
    ));
    host.with_completed_namespace_names(names.clone(), |host| {
        assert_eq!(
            host.consuming_expanded_name(&node(&input, target))
                .unwrap()
                .namespace_uri,
            "https://cem.dev/ns/schema/1"
        );
        let result = host
            .with_completed_namespace_names(completion(&input, &[target], "urn:inner"), |host| {
                assert_eq!(
                    host.consuming_expanded_name(&node(&input, target))
                        .unwrap()
                        .namespace_uri,
                    "urn:inner"
                );
                Err::<(), _>("consumer error")
            })
            .unwrap();
        assert_eq!(result, Err("consumer error"));
        assert_eq!(
            host.consuming_expanded_name(&node(&input, target))
                .unwrap()
                .namespace_uri,
            "https://cem.dev/ns/schema/1"
        );
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            host.with_completed_namespace_names(completion(&input, &[target], "urn:unwind"), |_| {
                panic!("consumer unwind")
            })
        }));
        assert!(unwind.is_err());
        assert_eq!(
            host.consuming_expanded_name(&node(&input, target))
                .unwrap()
                .namespace_uri,
            "https://cem.dev/ns/schema/1"
        );
    })
    .unwrap();
    assert!(host
        .consuming_expanded_name(&node(&input, target))
        .is_none());
    assert!(host.captured_expanded_name(&node(&input, target)).is_none());
    assert!(input.tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn name_completion_does_not_supply_contexts_grants_or_reset_reference_bounds() {
    use cem_ml::value::reference_resolution::ReferenceResolutionIssueKind;
    let input = import("{host @xmlns:s={#namespace} | {s:schema}} {#library}");
    let target = elements(&input, "schema")[0];
    let reference = input.captured.occurrences().last().unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(input.tree.clone(), Some(context(&input, target)), policy());
    let destination = host.register_scope(input.tree.clone(), None, policy());
    assert!(host.assign_subtree_scope(&input.tree, target, destination));
    let names = completion(&input, &[target], "https://cem.dev/ns/schema/1");
    host.with_completed_namespace_names(names, |host| {
        let denied = host
            .prepare_schema_scope("selected", node(&input, reference), policy().limits)
            .unwrap();
        assert!(denied
            .selection
            .issues
            .iter()
            .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied));
        host.allow_scope_crossing(origin, destination);
        let pending = host
            .prepare_schema_scope("selected", node(&input, reference), policy().limits)
            .unwrap();
        assert_eq!(
            pending.issue,
            Some(SchemaScopePreparationIssue::TargetContextNotReady)
        );
        host.set_context(destination, Some(context(&input, target)));
        let mut limits = policy().limits;
        limits.max_work = 1;
        let bounded = host
            .prepare_schema_scope("selected", node(&input, reference), limits)
            .unwrap();
        assert!(bounded
            .selection
            .issues
            .iter()
            .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
        assert!(host
            .prepare_schema_scope("selected", node(&input, reference), policy().limits)
            .unwrap()
            .is_ready());
    })
    .unwrap();
}

#[test]
fn completed_named_wrappers_keep_exact_declarations_and_original_empty_body_form() {
    use cem_ml::schema::{machine::SchemaElementForm, scope_controls::SchemaScopeControlExtent};
    let input = import("@ns s = https://cem.dev/ns/schema/1\n{container @xmlns:c={#namespace} | {c:schema @c:name=chosen | {s:schema | {elements | {element @name=child}}}} {c:schema @select=library |}} {#library}");
    let schemas = elements(&input, "schema");
    let wrapper = schemas[0];
    let exact = schemas[1];
    let empty_body = schemas[2];
    let reference = input.captured.occurrences().last().unwrap();
    let mut host = host(&input, wrapper);
    let names = completion(&input, &[wrapper, empty_body], "https://cem.dev/ns/core/1");
    let raw_source = match node(&input, wrapper).node() {
        CemAstNode::Element { source, .. } => source.clone(),
        _ => unreachable!(),
    };
    host.with_completed_namespace_names(names, |host| {
        let prepared = host.prepare_schema_scope("selected", node(&input, reference), policy().limits).unwrap();
        assert!(prepared.is_ready(), "{:?}", prepared.issue);
        let target = prepared.target.unwrap();
        assert_eq!(target.selected.node_id(), wrapper);
        assert_eq!(target.declaration.node_id(), exact);
        assert!(matches!(target.selected.node(), CemAstNode::Element {source, ..} if source == &raw_source));
        assert_eq!(host.captured_schema_element_form(&node(&input, empty_body)), Some(SchemaElementForm::Wrapping));
        let report = host.validate_input_runtime_host_regions("selected", input.tree.clone(), &[empty_body], &Default::default(), policy().limits, |_| Some(context(&input, wrapper))).unwrap();
        assert_eq!(report.inputs.len(), 1);
        assert_eq!(report.inputs[0].region().contract.extent(), SchemaScopeControlExtent::Body);
        assert!(report.inputs[0].is_ready());
    }).unwrap();
    assert!(host
        .consuming_expanded_name(&node(&input, wrapper))
        .is_none());
}

#[test]
fn nested_foreign_owner_completions_preserve_each_registered_original_owner() {
    let input = import("{host @xmlns:p={#namespace} | {p:schema}} {#library}");
    let foreign = import("{host @xmlns:p={#namespace} | {p:schema}} {#library}");
    let target = elements(&input, "schema")[0];
    let other = elements(&foreign, "schema")[0];
    assert_eq!(target, other); // owner identity, not arena-local address, gates lookup
    let mut host = host(&input, target);
    host.register_scope(
        foreign.tree.clone(),
        Some(context(&foreign, other)),
        policy(),
    );
    host.with_completed_namespace_names(
        completion(&input, &[target], "https://cem.dev/ns/schema/1"),
        |host| {
            assert!(host
                .consuming_expanded_name(&node(&foreign, other))
                .is_none());
            host.with_completed_namespace_names(
                completion(&foreign, &[other], "urn:other"),
                |host| {
                    assert_eq!(
                        host.consuming_expanded_name(&node(&input, target))
                            .unwrap()
                            .namespace_uri,
                        "https://cem.dev/ns/schema/1"
                    );
                    assert_eq!(
                        host.consuming_expanded_name(&node(&foreign, other))
                            .unwrap()
                            .namespace_uri,
                        "urn:other"
                    );
                    assert!(host.captured_expanded_name(&node(&input, target)).is_none());
                    assert!(host
                        .captured_expanded_name(&node(&foreign, other))
                        .is_none());
                },
            )
            .unwrap();
            assert!(host
                .consuming_expanded_name(&node(&foreign, other))
                .is_none());
            assert!(host
                .consuming_expanded_name(&node(&input, target))
                .is_some());
        },
    )
    .unwrap();
    assert!(host
        .consuming_expanded_name(&node(&input, target))
        .is_none());
    assert!(host
        .consuming_expanded_name(&node(&foreign, other))
        .is_none());
}
