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
        CemQlSchemaDeclarationHost, NamespaceLifecycleIssue, NamespaceScopePreparationIssue,
    },
};
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

fn root(input: &ScopedCemImport, name: &str) -> u32 {
    elements(input, name)[0]
}
#[test]
fn coordinator_stages_original_dependencies_and_restores_execution_state() {
    let input = import(
        "@ns public = urn:first\n{host @xmlns:v={#library} | {v:item | {#reuse}}} {outside}",
    );
    let (mut host, _) = host(&input);
    let mut seen = vec![];
    let (snapshot, names) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "host"), root(&input, "outside")],
            policy().limits,
            |source, lexical, _, _| {
                seen.push((
                    source.node_id(),
                    lexical.namespace_uri("v").map(str::to_owned),
                ));
                (
                    Some(inputs(&input)),
                    cem_ml::schema::reference_policy::ReferenceScopePolicyOverrides::default(),
                )
            },
            |host, snapshot| {
                assert!(snapshot.is_complete());
                assert!(select(host, &input).is_ready());
                let item = node(&input, root(&input, "item"));
                assert_eq!(
                    host.consuming_expanded_name(&item).unwrap().namespace_uri,
                    "urn:first"
                );
                snapshot
                    .completion
                    .expanded_name(item.node_id())
                    .unwrap()
                    .namespace_uri
                    .clone()
            },
        )
        .unwrap();
    assert_eq!(names, "urn:first");
    assert!(snapshot.is_complete());
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].1, None);
    assert_eq!(seen[1].1.as_deref(), Some("urn:first"));
    assert!(host
        .consuming_expanded_name(&node(&input, root(&input, "item")))
        .is_none());
    assert_eq!(
        select(&mut host, &input).issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
    assert!(input
        .captured
        .namespace_binding(input.tree.ast_owner(), property(&input))
        .is_none());
}
#[test]
fn unavailable_inputs_leave_independent_regions_ready_and_retry_with_fresh_inputs() {
    let input = import("@ns public = urn:first\n@ns public = urn:second\n{host @xmlns:v={#library} | {v:item | {#reuse}}} {outside}");
    let (mut host, _) = host(&input);
    let roots = [root(&input, "host"), root(&input, "outside")];
    let (pending, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &roots,
            policy().limits,
            |_, _, _, _| (None, Default::default()),
            |_, snapshot| {
                assert!(!snapshot.is_complete());
                assert_eq!(snapshot.ready_roots, vec![roots[1]]);
                assert!(!snapshot.completion.contains(root(&input, "item")));
            },
        )
        .unwrap();
    assert!(!pending.is_complete());
    for (target, uri) in [
        (elements(&input, "@ns")[0], "urn:first"),
        (elements(&input, "@ns")[1], "urn:second"),
    ] {
        let runtime = context(&input, &[target]).with_binding(
            "reuse",
            StandaloneExpressionBinding::any(ItemStream::from_items(vec![RetainedCemNode::new(
                input.tree.clone(),
                property(&input),
            )
            .unwrap()
            .query_item()])),
        );
        let (ready, ()) = host
            .with_namespace_lifecycle(
                input.captured.clone(),
                &roots,
                policy().limits,
                |_, _, _, _| (Some(runtime.clone()), Default::default()),
                |_, snapshot| {
                    assert!(snapshot.is_complete());
                },
            )
            .unwrap();
        assert_eq!(
            ready
                .completion
                .expanded_name(root(&input, "item"))
                .unwrap()
                .namespace_uri,
            uri
        );
    }
    assert!(!pending.completion.contains(root(&input, "item")));
}
#[test]
fn failed_consumer_restores_names_scopes_and_publications_on_unwind() {
    let input = import("@ns public = urn:first\n{host @xmlns:v={#library} | {v:item | {#reuse}}}");
    let (mut host, _) = host(&input);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        host.with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "host")],
            policy().limits,
            |_, _, _, _| (Some(inputs(&input)), Default::default()),
            |host, snapshot| {
                assert!(snapshot.is_complete());
                assert!(select(host, &input).is_ready());
                panic!("fixture consumer");
            },
        )
        .unwrap();
    }));
    assert!(panic.is_err());
    assert!(host
        .consuming_expanded_name(&node(&input, root(&input, "item")))
        .is_none());
    assert_eq!(
        select(&mut host, &input).issue,
        Some(NamespaceScopePreparationIssue::TargetBindingNotReady)
    );
}
#[test]
fn work_limit_rejects_before_callbacks_and_explicit_pending_scope_cannot_borrow() {
    let input = import("@ns public = urn:first\n{host @xmlns:v={#library} | {v:item}} {outside}");
    let (mut host, scope) = host(&input);
    let mut limits = policy().limits;
    limits.max_work = 1;
    let result = host.with_namespace_lifecycle(
        input.captured.clone(),
        &[root(&input, "host")],
        limits,
        |_, _, _, _| panic!("bounded before callback"),
        |_, _| (),
    );
    assert!(matches!(
        result,
        Err(cem_ql::schema_references::NamespaceLifecycleError::WorkLimit)
    ));
    let pending = host.register_lexical_scope(scope, None, policy()).unwrap();
    let value = input.captured.occurrences().next().unwrap();
    host.assign_subtree_scope(&input.tree, value, pending);
    let (report, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "host")],
            policy().limits,
            |_, _, _, _| panic!("explicit original occurrence inputs stay authoritative"),
            |_, snapshot| {
                assert!(!snapshot.is_complete());
            },
        )
        .unwrap();
    assert!(!report.is_complete());
    assert!(report
        .incomplete_roots
        .iter()
        .any(|(_, issue)| matches!(issue, NamespaceLifecycleIssue::Namespace(_))));
}

#[test]
fn unused_unavailable_namespace_properties_keep_their_own_region_incomplete() {
    let input = import("@ns public = urn:first\n{host @xmlns:v={#library}} {outside}");
    let (mut host, _) = host(&input);
    let (report, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "host"), root(&input, "outside")],
            policy().limits,
            |_, _, _, _| (None, Default::default()),
            |_, snapshot| {
                assert!(!snapshot.is_complete());
                assert_eq!(snapshot.ready_roots, vec![root(&input, "outside")]);
                assert_eq!(snapshot.pending_properties, vec![property(&input)]);
            },
        )
        .unwrap();
    assert!(matches!(
        report.incomplete_roots[0].1,
        NamespaceLifecycleIssue::PropertyNotReady(_)
    ));
}

#[test]
fn coordinated_schema_controls_activate_only_completed_core_names() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    for (uri, expected) in [("https://cem.dev/ns/core/1", 1), ("urn:foreign", 0)] {
        let input = import(&format!("@ns public = {uri}\n@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {{elements | {{element @name=child}}}}}}\n{{host @xmlns:c={{#namespace}} @c:schema-select={{library}} | {{child}}}}"));
        let schema = root(&input, "schema");
        let namespace = root(&input, "@ns");
        let runtime = context(&input, &[schema]).with_binding(
            "namespace",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(input.tree.clone(), namespace)
                    .unwrap()
                    .query_item(),
            )),
        );
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(input.tree.clone(), Some(runtime.clone()), policy());
        let model = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @children=child @optional-attributes=urn:foreign:*} {element @name=child}} {attributes | {attribute @name=schema-select @type=node}}}");
        let (snapshot, report) = host
            .with_namespace_lifecycle(
                input.captured.clone(),
                &[root(&input, "host")],
                policy().limits,
                |_, _, _, _| (Some(runtime.clone()), Default::default()),
                |host, snapshot| {
                    host.validate_input_runtime_host_regions(
                        "outer",
                        input.tree.clone(),
                        &snapshot.ready_roots,
                        &model,
                        policy().limits,
                        |_| Some(runtime.clone()),
                    )
                    .unwrap()
                },
            )
            .unwrap();
        assert!(snapshot.is_complete());
        assert_eq!(
            report.inputs.len(),
            expected,
            "{uri}: {:?}",
            report.validation.diagnostics
        );
        assert!(
            report.validation.complete,
            "{uri}: {:?}",
            report.validation.diagnostics
        );
    }
}

#[test]
fn authorized_pending_declaration_dependencies_outside_query_forest_are_staged_once_ready() {
    let input = import(
        "@ns public = urn:first\n{other @xmlns:v={#library}} {host @xmlns:w={#reuse} | {w:item}}",
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
    let (mut host, _) = host(&input);
    let runtime = inputs(&input);
    let mut seen = vec![];
    let (snapshot, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "item")],
            policy().limits,
            |source, _, _, _| {
                seen.push(source.node_id());
                (Some(runtime.clone()), Default::default())
            },
            |_, snapshot| {
                assert!(snapshot.is_complete());
            },
        )
        .unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(snapshot.properties.len(), 2);
    assert_eq!(
        snapshot
            .completion
            .expanded_name(root(&input, "item"))
            .unwrap()
            .namespace_uri,
        "urn:first"
    );
    assert!(!snapshot.completion.contains(root(&input, "other")));
    assert!(!snapshot.completion.contains(properties[0]));
    // The failed first selection and its retry both consumed the same request's
    // work; only the final per-property selection report is retained.
    let initial_work = snapshot.work_used;
    let mut limits = policy().limits;
    limits.max_work = initial_work - 1;
    let (limited, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "item")],
            limits,
            |_, _, _, _| (Some(runtime.clone()), Default::default()),
            |_, snapshot| {
                assert!(!snapshot.is_complete());
            },
        )
        .unwrap();
    assert!(limited.work_used <= limits.max_work);
}
#[test]
fn foreign_namespace_providers_require_explicit_grants_on_each_invocation() {
    let input = import("{host @xmlns:v={#library} | {v:item}} {outside}");
    let vendor = import("@ns public = urn:vendor");
    let runtime = context(&vendor, &[root(&vendor, "@ns")]);
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(input.tree.clone(), Some(runtime.clone()), policy());
    let destination = host.register_scope(vendor.tree.clone(), Some(Default::default()), policy());
    host.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    let roots = [root(&input, "item"), root(&input, "outside")];
    let (denied, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &roots,
            policy().limits,
            |_, _, _, _| (Some(runtime.clone()), Default::default()),
            |_, _| (),
        )
        .unwrap();
    assert!(!denied.is_complete());
    assert_eq!(denied.ready_roots, vec![roots[1]]);
    host.allow_scope_crossing(origin, destination);
    let (ready, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &roots,
            policy().limits,
            |_, _, _, _| (Some(runtime.clone()), Default::default()),
            |_, _| (),
        )
        .unwrap();
    assert!(ready.is_complete());
    assert_eq!(
        ready
            .completion
            .expanded_name(roots[0])
            .unwrap()
            .namespace_uri,
        "urn:vendor"
    );
    assert!(std::sync::Arc::ptr_eq(
        ready.properties[0]
            .preparation
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .binding_declaration()
            .document(),
        vendor.tree.ast_owner()
    ));
}

#[test]
fn consumed_namespace_controls_do_not_exempt_ordinary_native_data_attributes() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let input = import("@ns public = urn:first\n{host @xmlns:v={#library} @c={#library}}");
    let (mut host, _) = host(&input);
    let model = compile_schema_document_model("outer", "{schema | {elements | {element @name=host @optional-attributes=c}} {attributes | {attribute @name=c @type=string}}}");
    let (snapshot, report) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "host")],
            policy().limits,
            |_, _, _, _| (Some(inputs(&input)), Default::default()),
            |host, snapshot| {
                host.validate_input_roots(
                    input.tree.clone(),
                    &snapshot.ready_roots,
                    &model,
                    policy().limits,
                )
                .unwrap()
            },
        )
        .unwrap();
    assert!(snapshot.is_complete());
    assert!(report.failed);
    assert!(report.diagnostics.iter().any(|d| d.code
        == cem_ml::schema::document_model::INVALID_ATTRIBUTE_TYPE_CODE
        && d.message.contains("attribute `c`")));
    assert!(report
        .diagnostics
        .iter()
        .all(|d| !d.message.contains("attribute `v`")));
}

#[test]
fn selector_local_policy_is_preserved_under_coordinator_request_limits() {
    let input = import("@ns public = urn:first\n{host @xmlns:v={#library} | {v:item}}");
    let (mut host, _) = host(&input);
    let mut strict = policy();
    strict.limits.max_work = 1;
    let (snapshot, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "host")],
            policy().limits,
            |_, _, _, _| {
                (
                    Some(inputs(&input)),
                    cem_ml::schema::reference_policy::ReferenceScopePolicyOverrides::explicit(
                        strict.clone(),
                    ),
                )
            },
            |host, snapshot| {
                assert!(!snapshot.is_complete());
                assert_eq!(
                    host.reference_scope_policy(snapshot.scopes[0].1)
                        .unwrap()
                        .limits
                        .max_work,
                    1
                );
            },
        )
        .unwrap();
    assert!(snapshot.properties[0]
        .preparation
        .as_ref()
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind
            == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::WorkLimit));
}

#[test]
fn explicit_inputs_cannot_bypass_pending_pre_declaration_namespace_dependencies() {
    let input =
        import("@ns public = urn:first\n{host @xmlns:v={#library} @xmlns:w={#library} | {w:item}}");
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
            } if !value_nodes.is_empty() => Some((*node_id, value_nodes[0])),
            _ => None,
        })
        .collect();
    let (mut host, scope) = host(&input);
    let explicit = host
        .register_lexical_scope(scope, Some(inputs(&input)), policy())
        .unwrap();
    host.assign_subtree_scope(&input.tree, properties[1].1, explicit);
    let (snapshot, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[root(&input, "item")],
            policy().limits,
            |source, _, _, _| {
                assert_eq!(source.node_id(), properties[0].1);
                (None, Default::default())
            },
            |_, snapshot| {
                assert!(snapshot.ready_roots.is_empty());
            },
        )
        .unwrap();
    assert!(!snapshot.is_complete());
    assert_eq!(
        snapshot.pending_properties,
        vec![properties[0].0, properties[1].0]
    );
}
