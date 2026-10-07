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

fn attribute_model(native: bool) -> cem_ml::schema::document_model::SchemaDocumentModel {
    cem_ml::schema::document_model::compile_schema_document_model(
        "consumer",
        &"{schema | {elements | {element @name=item @optional-attributes=urn:allowed:*}} {attributes | {attribute @name=target @type=TYPE}}}".replace("TYPE", if native { "node" } else { "integer" }),
    )
}
fn structural_report(
    input: &ScopedCemImport,
    roots: &[u32],
    model: &cem_ml::schema::document_model::SchemaDocumentModel,
    host: &mut CemQlSchemaDeclarationHost,
) -> cem_ml::schema::input_references::StructuralInputValidation<
    cem_ql::schema_references::CemQlSchemaReferenceNode,
> {
    cem_ml::schema::input_references::validate_structural_input_roots_references(
        input.tree.ast_owner().clone(),
        roots,
        model,
        host,
        policy().limits,
    )
    .unwrap()
}

#[test]
fn structural_literal_attribute_names_use_completion_and_keep_lexical_typing() {
    let input =
        import("{container @xmlns:p={#namespace} | {item @p:target=003} {item @p:target=invalid}}");
    let roots = elements(&input, "item");
    let model = attribute_model(false);
    let mut host = host(&input, roots[0]);
    let pending = structural_report(&input, &roots, &model, &mut host);
    assert!(
        !pending.complete && !pending.failed,
        "{:?}",
        pending.diagnostics
    );
    assert!(pending.diagnostics.is_empty());
    for uri in ["urn:allowed", "urn:foreign", "urn:allowed"] {
        let report = host
            .with_completed_namespace_names(completion(&input, &roots, uri), |host| {
                structural_report(&input, &roots, &model, host)
            })
            .unwrap();
        assert!(report.complete && report.failed, "{:?}", report.diagnostics);
        let code = if uri == "urn:allowed" {
            cem_ml::schema::document_model::INVALID_ATTRIBUTE_TYPE_CODE
        } else {
            cem_ml::schema::document_model::UNKNOWN_ATTRIBUTE_CODE
        };
        assert_eq!(
            report.diagnostics.len(),
            if uri == "urn:allowed" { 1 } else { 2 }
        );
        assert!(report.diagnostics.iter().all(|d| d.code == code
            && d.uri.as_deref() == Some("names.cem")
            && d.byte_offset.is_some()));
        assert!(report
            .nodes
            .iter()
            .all(|n| Arc::ptr_eq(n.source.document(), input.tree.ast_owner())));
    }
    assert!(!structural_report(&input, &roots, &model, &mut host).complete);
    assert!(input.tree.ast().nodes.iter().any(|n| matches!(n, CemAstNode::Attribute {expanded_name, value: Some(value), ..} if expanded_name.namespace_uri == "p" && value == "003")));
}

#[test]
fn completed_native_attribute_names_admit_original_nodes_and_defer_outside_forest() {
    let input = import("{container @xmlns:p={#namespace} | {item @p:target={#library}} {item @p:target={#library}}} {payload}");
    let roots = elements(&input, "item");
    let payload = elements(&input, "payload")[0];
    let model = attribute_model(true);
    let mut host = host(&input, payload);
    let pending = structural_report(&input, &roots, &model, &mut host);
    assert!(
        !pending.complete && !pending.failed,
        "{:?}",
        pending.diagnostics
    );
    assert!(pending.nodes.iter().all(|n| n.attribute_values.is_empty()));
    let names = completion(&input, &roots[..1], "urn:allowed");
    let report = host
        .with_completed_namespace_names(names, |host| {
            structural_report(&input, &roots, &model, host)
        })
        .unwrap();
    assert!(
        !report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert!(report.nodes[0].attribute_values[0].complete);
    assert!(report.nodes[1].attribute_values.is_empty());
    let access = &report.nodes[0].attribute_values[0].access;
    assert_eq!(access.node(access.roots()[0]).unwrap().node_id(), payload);
    assert!(Arc::ptr_eq(
        access.node(0).unwrap().document(),
        input.tree.ast_owner()
    ));
    let foreign = host
        .with_completed_namespace_names(completion(&input, &roots, "urn:foreign"), |host| {
            structural_report(&input, &roots, &model, host)
        })
        .unwrap();
    assert!(!foreign.complete && foreign.failed);
    assert!(foreign.nodes.iter().all(|n| n.attribute_values.is_empty()));
    assert!(foreign
        .diagnostics
        .iter()
        .all(|d| d.code == cem_ml::schema::document_model::UNKNOWN_ATTRIBUTE_CODE));
    assert!(input.tree.ast().nodes.iter().all(|n| !matches!(
        n,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn selected_structural_attribute_completion_preserves_grants_and_one_work_budget() {
    use cem_ml::value::reference_resolution::ReferenceResolutionIssueKind;
    let input = import("{#library}");
    let vendor =
        import("{container @xmlns:p={#namespace} | {item @p:target={#library}}} {payload}");
    let target = elements(&vendor, "item")[0];
    let payload = elements(&vendor, "payload")[0];
    let roots = [input.captured.occurrences().next().unwrap()];
    let model = attribute_model(true);
    let mut host = CemQlSchemaDeclarationHost::new();
    let request = host.register_scope(input.tree.clone(), Some(context(&vendor, target)), policy());
    let destination = host.register_scope(
        vendor.tree.clone(),
        Some(context(&vendor, payload)),
        policy(),
    );
    host.attach_captured_names(&input.captured).unwrap();
    host.attach_captured_names(&vendor.captured).unwrap();
    host.with_completed_namespace_names(completion(&vendor, &[target], "urn:allowed"), |host| {
        let denied = structural_report(&input, &roots, &model, host);
        assert!(
            !denied.complete
                && denied.references[0]
                    .resolution
                    .issues
                    .iter()
                    .any(|i| i.kind == ReferenceResolutionIssueKind::ScopeDenied)
        );
    })
    .unwrap();
    host.allow_scope_crossing(request, destination);
    let pending = structural_report(&input, &roots, &model, &mut host);
    assert!(
        !pending.complete && !pending.failed,
        "{:?}",
        pending.diagnostics
    );
    host.with_completed_namespace_names(completion(&vendor, &[target], "urn:allowed"), |host| {
        let ready = structural_report(&input, &roots, &model, host);
        assert!(ready.complete && !ready.failed, "{:?}", ready.diagnostics);
        assert_eq!(ready.nodes[0].source.node_id(), target);
        assert!(Arc::ptr_eq(
            ready.nodes[0].source.document(),
            vendor.tree.ast_owner()
        ));
        assert_eq!(
            ready.nodes[0].attribute_values[0]
                .access
                .node(0)
                .unwrap()
                .node_id(),
            payload
        );
        let bounded = cem_ml::schema::input_references::validate_structural_input_roots_references(
            input.tree.ast_owner().clone(),
            &roots,
            &model,
            host,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_work: 2,
                ..policy().limits
            },
        )
        .unwrap();
        assert!(!bounded.complete);
        assert!(bounded.references[0]
            .resolution
            .issues
            .iter()
            .any(|i| i.kind == ReferenceResolutionIssueKind::WorkLimit));
        assert_eq!(bounded.references[0].resolution.work_used, 2);
    })
    .unwrap();
}

#[test]
fn completed_structural_attributes_keep_child_consuming_models() {
    use cem_ml::schema::input_references::{
        validate_structural_input_regions_references, InputSchemaRegion,
    };
    let input = import(
        "{container @xmlns:p={#namespace} | {host | {item @p:target={#library}}}} {payload}",
    );
    let root = elements(&input, "host")[0];
    let item = elements(&input, "item")[0];
    let mut host = host(&input, elements(&input, "payload")[0]);
    let outer = cem_ml::schema::document_model::compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=item} {element @name=item}}}",
    );
    let child = attribute_model(true);
    host.with_completed_namespace_names(completion(&input, &[root], "urn:allowed"), |host| {
        let report = validate_structural_input_regions_references(
            input.tree.ast_owner().clone(),
            &[root],
            &outer,
            &[InputSchemaRegion {
                host: node(&input, root),
                model: Some(&child),
            }],
            host,
            policy().limits,
        )
        .unwrap();
        assert!(
            report.complete && !report.failed,
            "{:?}",
            report.diagnostics
        );
        let selected = report
            .nodes
            .iter()
            .find(|n| n.source.node_id() == item)
            .unwrap();
        assert!(selected.attribute_values[0].complete);
        let enclosing = structural_report(&input, &[root], &outer, host);
        assert!(!enclosing.complete && enclosing.failed);
    })
    .unwrap();
}

#[test]
fn pending_structural_element_names_defer_presence_checks_until_completion() {
    let input = import("{container @xmlns:p={#namespace} | {p:item @p:target=003}}");
    let roots = elements(&input, "item");
    let mut model = attribute_model(false);
    model
        .elements
        .get_mut("item")
        .unwrap()
        .required_attributes
        .insert("needed".into());
    let mut host = host(&input, roots[0]);
    let pending = structural_report(&input, &roots, &model, &mut host);
    assert!(!pending.complete && !pending.failed);
    assert!(pending.diagnostics.is_empty());
    let ready = host
        .with_completed_namespace_names(completion(&input, &roots, "urn:allowed"), |host| {
            structural_report(&input, &roots, &model, host)
        })
        .unwrap();
    assert!(ready.complete && ready.failed, "{:?}", ready.diagnostics);
    assert_eq!(ready.diagnostics.len(), 1);
    assert_eq!(
        ready.diagnostics[0].code,
        cem_ml::schema::document_model::MISSING_REQUIRED_ATTRIBUTE_CODE
    );
    assert!(host
        .consuming_expanded_name(&node(&input, roots[0]))
        .is_none());
    assert!(
        matches!(node(&input, roots[0]).node(), CemAstNode::Element {expanded_name, ..} if expanded_name.namespace_uri == "p")
    );
}

#[test]
fn runtime_child_binding_forwards_completed_attribute_names_and_restores_context() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::ReferenceResolutionHost,
    };
    let input = import("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=item @optional-attributes=urn:allowed:*}} {attributes | {attribute @name=target @type=node}}}\n{container @xmlns:p={#namespace} | {host @schema-select=library | {item @p:target={#library}}}} {payload}");
    let root = elements(&input, "host")[0];
    let item = elements(&input, "item")[0];
    let payload = elements(&input, "payload")[0];
    let mut host = host(&input, elements(&input, "schema")[0]);
    let occurrence = input
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == "#library" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let original_scope = host.scope(&host.source_reference(node(&input, occurrence)));
    let outer = cem_ml::schema::document_model::compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=item} {element @name=item}}}",
    );
    for uri in ["urn:allowed", "urn:foreign"] {
        host.with_completed_namespace_names(completion(&input, &[root], uri), |host| {
            let report = host
                .validate_input_runtime_host_regions(
                    "child",
                    input.tree.clone(),
                    &[root],
                    &outer,
                    policy().limits,
                    |_| Some(context(&input, payload)),
                )
                .unwrap();
            assert_eq!(report.validation.complete, uri == "urn:allowed");
            assert_eq!(report.validation.failed, uri == "urn:foreign");
            assert!(report.inputs[0].is_ready());
            let selected = report
                .validation
                .nodes
                .iter()
                .find(|n| n.source.node_id() == item)
                .unwrap();
            assert_eq!(
                selected.attribute_values.len(),
                usize::from(uri == "urn:allowed")
            );
            if uri == "urn:allowed" {
                assert_eq!(
                    selected.attribute_values[0]
                        .access
                        .node(0)
                        .unwrap()
                        .node_id(),
                    payload
                );
            }
            assert_eq!(
                host.scope(&host.source_reference(node(&input, occurrence))),
                original_scope
            );
        })
        .unwrap();
    }
    assert!(host.consuming_expanded_name(&node(&input, item)).is_some()); // fixed unqualified element
    assert!(input.tree.ast().nodes.iter().all(|n| !matches!(
        n,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}
