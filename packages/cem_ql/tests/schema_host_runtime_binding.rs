use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    schema_references::{
        CemQlSchemaDeclarationHost, SchemaHostRuntimeContextRequest, SchemaHostRuntimeInputIssue,
    },
};
use std::sync::Arc;

fn capture(text: &str) -> (LexicallyScopedDocument, Arc<RetainedCemTree>) {
    let events = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
        SourceId(1),
        text.as_bytes().to_vec(),
    )));
    let captured =
        CemSchemaMachine::new(CompiledSchema::cem_core(), events).build_with_lexical_scopes();
    assert!(captured.document().diagnostics.is_empty());
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "scope.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    (captured, tree)
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn source(tree: &Arc<RetainedCemTree>, id: u32) -> SchemaDeclarationNode {
    SchemaDeclarationNode::new(tree.ast_owner().clone(), id).unwrap()
}
fn elements(tree: &Arc<RetainedCemTree>, name: &str) -> Vec<u32> {
    tree.ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .collect()
}
fn context(tree: &Arc<RetainedCemTree>, targets: &[u32]) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::from_items(
            targets
                .iter()
                .map(|id| {
                    RetainedCemNode::new(tree.clone(), *id)
                        .unwrap()
                        .query_item()
                })
                .collect(),
        )),
    )
}

fn body_context(tree: &Arc<RetainedCemTree>, targets: &[u32]) -> StandaloneExpressionContext {
    context(tree, targets).with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::from_items(
            targets
                .iter()
                .map(|id| {
                    RetainedCemNode::new(tree.clone(), *id)
                        .unwrap()
                        .query_item()
                })
                .collect(),
        )),
    )
}

#[test]
fn automatic_runtime_binding_uses_explicit_body_inputs_and_restores_mappings() {
    use cem_ml::schema::declaration_references::SchemaDeclarationHost;
    use cem_ml::schema::document_model::compile_schema_document_model;
    use cem_ml::value::reference_resolution::ReferenceResolutionHost;
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=first} {element @name=second}}} {host @schema-select=library | {#items}} {first} {second}");
    let boundary = elements(&tree, "host")[0];
    let reference = captured.occurrences().last().unwrap();
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children='first second'}}}",
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(
        tree.clone(),
        Some(context(&tree, &elements(&tree, "schema"))),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    for name in ["first", "second"] {
        let mut requests = 0;
        let report = host
            .validate_input_runtime_host_regions(
                "child",
                tree.clone(),
                &[boundary],
                &outer,
                policy().limits,
                |request| match request {
                    SchemaHostRuntimeContextRequest::Body(region) => {
                        requests += 1;
                        assert_eq!(region.contract.host().node_id(), boundary);
                        Some(body_context(&tree, &elements(&tree, name)))
                    }
                    SchemaHostRuntimeContextRequest::Occurrence { .. } => panic!("no local frames"),
                },
            )
            .unwrap();
        assert!(
            report.validation.complete && !report.validation.failed,
            "{:?}",
            report.validation.diagnostics
        );
        assert_eq!(requests, 1);
        assert_eq!(report.inputs.len(), 1);
        assert!(report.inputs[0].is_ready());
        assert_eq!(report.scopes.len(), 1);
        assert!(report
            .validation
            .nodes
            .iter()
            .any(|node| node.source.node_id() == elements(&tree, name)[0]));
        let node = host.source_reference(source(&tree, reference));
        assert_eq!(host.scope(&node), Some(original));
        assert!(host
            .scope_policy_overrides(report.scopes[0].scope())
            .is_some());
        assert!(matches!(
            tree.ast().get(reference),
            Some(CemAstNode::Reference { targets: None, .. })
        ));
    }
    let pending = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &[boundary],
            &outer,
            policy().limits,
            |_| None,
        )
        .unwrap();
    assert!(!pending.validation.complete);
    assert_eq!(pending.validation.nodes.len(), 1);
    assert_eq!(
        pending.inputs[0].issue(),
        Some(&SchemaHostRuntimeInputIssue::ContextNotReady)
    );
    assert!(pending.scopes.is_empty());
    assert_eq!(
        host.scope(&host.source_reference(source(&tree, reference))),
        Some(original)
    );
}

#[test]
fn entered_local_occurrences_replay_declarations_and_leave_inactive_frames_alone() {
    use cem_ml::schema::declaration_references::SchemaDeclarationHost;
    use cem_ml::schema::{
        document_model::compile_schema_document_model,
        reference_policy::{
            ReferencePolicyOverrideOrigin, ReferenceScopePolicyOverrides, UnresolvedDisposition,
        },
    };
    use cem_ml::value::reference_resolution::ReferenceResolutionHost;
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=chosen}} {constraints | {constraint @kind=reference-traversal-depth @value=3} {constraint @kind=reference-traversal-work @value=17} {constraint @kind=reference-unresolved-disposition @value=warning}}} {host @schema-select=library | {#items}} {host @schema-select=library | {#items}} {chosen}");
    let references: Vec<_> = captured.occurrences().collect();
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=host @children=chosen}}}",
    );
    let local = compile_schema_document_model(
        "local",
        "{schema | {constraints | {constraint @kind=reference-traversal-depth @value=128}}}",
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, &elements(&tree, "schema"))),
        policy(),
    );
    let original = host
        .attach_captured_lexical_scopes_with_policy_overrides(&captured, |_, _, _| {
            (
                None,
                ReferenceScopePolicyOverrides::from_schema(&local).unwrap(),
            )
        })
        .unwrap();
    let mut requests = vec![];
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &[elements(&tree, "host")[0]],
            &outer,
            policy().limits,
            |request| match request {
                SchemaHostRuntimeContextRequest::Body(_) => {
                    Some(body_context(&tree, &elements(&tree, "chosen")))
                }
                SchemaHostRuntimeContextRequest::Occurrence {
                    source,
                    original_scope,
                    child,
                } => {
                    requests.push((source.node_id(), original_scope));
                    assert_eq!(child.inputs().policy().unwrap().limits.max_depth, 3);
                    Some(body_context(&tree, &elements(&tree, "chosen")))
                }
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(requests, vec![(references[0], original[0].1)]);
    assert_eq!(report.occurrences.len(), 1);
    let effective = report.occurrences[0].1;
    let bounds = host.scope_limits(&Some(effective));
    assert_eq!(bounds.max_depth, 128);
    assert_eq!(bounds.max_work, 17);
    assert!(matches!(
        host.scope_policy_overrides(effective)
            .unwrap()
            .depth()
            .unwrap()
            .origin(),
        ReferencePolicyOverrideOrigin::Schema(_)
    ));
    assert!(host
        .scope_policy_overrides(effective)
        .unwrap()
        .work()
        .is_none());
    let pending = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &[elements(&tree, "host")[0]],
            &outer,
            policy().limits,
            |request| match request {
                SchemaHostRuntimeContextRequest::Body(_) => {
                    Some(body_context(&tree, &elements(&tree, "chosen")))
                }
                SchemaHostRuntimeContextRequest::Occurrence { .. } => None,
            },
        )
        .unwrap();
    assert!(!pending.validation.complete && !pending.validation.failed);
    assert!(pending.validation.references[0]
        .resolution
        .issues
        .iter()
        .any(|issue| issue.reason == "schema-context-not-ready"));
    let refnode = host.source_reference(source(&tree, references[0]));
    assert_eq!(host.scope(&refnode), Some(original[0].1));
    assert_eq!(
        host.unresolved_policy(&refnode).disposition(),
        UnresolvedDisposition::Neutral
    );
}

#[test]
fn nested_and_sibling_bodies_restore_models_and_selector_inputs() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured,tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema @name=outer | {elements | {element @name=nested @children=inner} {element @name=after}}} {s:schema @name=inner | {elements | {element @name=inner}}} {host @schema-select=library | {nested @schema-select={#library} | {#items}} {#items}} {sibling | {#items}} {inner} {after} {outside}");
    let schemas = elements(&tree, "schema");
    let outer = compile_schema_document_model("base","{schema | {elements | {element @name=host @children='nested after'} {element @name=sibling @children=outside} {element @name=outside}}}");
    let root_context = body_context(&tree, &elements(&tree, "outside")).with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(tree.clone(), schemas[0])
                .unwrap()
                .query_item(),
        )),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(root_context), policy());
    host.attach_captured_names(&captured).unwrap();
    let mut order = vec![];
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &[elements(&tree, "host")[0], elements(&tree, "sibling")[0]],
            &outer,
            policy().limits,
            |request| match request {
                SchemaHostRuntimeContextRequest::Body(region) => {
                    let boundary = region.contract.host().node_id();
                    order.push(boundary);
                    if boundary == elements(&tree, "host")[0] {
                        Some(
                            body_context(&tree, &elements(&tree, "after")).with_binding(
                                "library",
                                StandaloneExpressionBinding::any(ItemStream::once(
                                    RetainedCemNode::new(tree.clone(), schemas[1])
                                        .unwrap()
                                        .query_item(),
                                )),
                            ),
                        )
                    } else {
                        Some(body_context(&tree, &elements(&tree, "inner")))
                    }
                }
                _ => panic!("no original local frames"),
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(
        order,
        vec![elements(&tree, "host")[0], elements(&tree, "nested")[0]]
    );
    assert_eq!(
        report.inputs[1]
            .region()
            .preparation
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .declaration
            .node_id(),
        schemas[1]
    );
    for name in ["inner", "after", "outside"] {
        assert!(report
            .validation
            .nodes
            .iter()
            .any(|node| node.source.node_id() == elements(&tree, name)[0]));
    }
}

#[test]
fn invalid_policy_blocks_the_body_with_original_constraint_attribution() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured,tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {constraints | {constraint @kind=reference-traversal-depth @value=0}}} {host @schema-select=library | {nested @schema-select=library | {#items}}}");
    let outer =
        compile_schema_document_model("base", "{schema | {elements | {element @name=host}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(&tree, &elements(&tree, "schema"))),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let mut calls = 0;
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| {
                calls += 1;
                Some(StandaloneExpressionContext::default())
            },
        )
        .unwrap();
    assert_eq!(calls, 0);
    assert_eq!(report.inputs.len(), 1);
    assert!(matches!(
        report.inputs[0].issue(),
        Some(SchemaHostRuntimeInputIssue::InvalidPolicy(_))
    ));
    assert!(!report.validation.complete && report.validation.failed);
    assert_eq!(report.validation.nodes.len(), 1);
    let diagnostic = report
        .validation
        .diagnostics
        .iter()
        .find(|d| d.code == "cem.schema_scope.invalid_reference_policy")
        .unwrap();
    let Some(SchemaHostRuntimeInputIssue::InvalidPolicy(error)) = report.inputs[0].issue() else {
        panic!()
    };
    assert_eq!(diagnostic.source_map.as_ref(), Some(&error.source_map));
}

#[test]
fn selected_hosts_keep_directed_grants_and_request_and_destination_work_caps() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    use cem_ml::value::reference_resolution::ReferenceResolutionIssueKind;
    let (input_capture, input) = capture("{box | {#library} {#library}}");
    let (library_capture, library) =
        capture("{host @schema-select=library | {group | {#items}}} {chosen}");
    let (schema_capture,schema) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=group @children=chosen} {element @name=chosen}} {constraints | {constraint @kind=reference-traversal-work @value=2}}}");
    let outer = compile_schema_document_model("base","{schema | {elements | {element @name=box @children=host} {element @name=host @children=group}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(
        input.clone(),
        Some(context(&library, &elements(&library, "host"))),
        policy(),
    );
    let destination = host.register_scope(
        library.clone(),
        Some(context(&schema, &elements(&schema, "schema"))),
        policy(),
    );
    let declaration = host.register_scope(
        schema.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    for capture in [&input_capture, &library_capture, &schema_capture] {
        host.attach_captured_names(capture).unwrap();
    }
    let denied = host
        .validate_input_runtime_host_regions(
            "child",
            input.clone(),
            &elements(&input, "box"),
            &outer,
            policy().limits,
            |_| panic!("denied hosts cannot request inputs"),
        )
        .unwrap();
    assert!(!denied.validation.complete);
    assert!(denied.inputs.is_empty());
    assert!(host.allow_scope_crossing(origin, destination));
    assert!(host.allow_scope_crossing(destination, declaration));
    let mut limits = policy().limits;
    limits.max_work = 1;
    let exhausted = host
        .validate_input_runtime_host_regions(
            "child",
            input.clone(),
            &elements(&input, "box"),
            &outer,
            limits,
            |_| panic!("exhausted targets cannot request inputs"),
        )
        .unwrap();
    assert!(exhausted.inputs.is_empty());
    let mut requests = 0;
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            input.clone(),
            &elements(&input, "box"),
            &outer,
            policy().limits,
            |_| {
                requests += 1;
                Some(body_context(&library, &elements(&library, "chosen")))
            },
        )
        .unwrap();
    assert_eq!(requests, 1);
    assert_eq!(report.scopes.len(), 1);
    assert!(report.inputs[0].is_ready());
    assert_eq!(report.validation.references.len(), 2);
    assert!(!report.validation.complete && !report.validation.failed);
    for reference in &report.validation.references {
        assert_eq!(reference.resolution.work_used, 5);
        assert!(reference.resolution.issues.iter().any(|issue| issue.kind
            == ReferenceResolutionIssueKind::WorkLimit
            && issue.reason == "scope-work-limit"));
        assert!(reference.resolution.nodes.iter().all(|node| {
            use cem_ml::schema::declaration_references::SchemaDeclarationHost;
            host.declaration_node(node).unwrap().node_id() != elements(&library, "chosen")[0]
        }));
    }
    assert!(Arc::ptr_eq(
        report.inputs[0].region().contract.host().document(),
        library.ast_owner()
    ));
}

#[test]
fn rejected_occurrence_handoff_and_caller_unwind_restore_original_assignments() {
    use cem_ml::schema::{
        declaration_references::SchemaDeclarationHost,
        document_model::compile_schema_document_model,
        reference_policy::ReferenceScopePolicyOverrides,
    };
    use cem_ml::value::reference_resolution::{ReferenceResolutionError, ReferenceResolutionHost};
    let (captured,tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=group @children=chosen} {element @name=chosen}}} {host @schema-select=library | {group | {#items}}} {chosen}");
    let reference = captured.occurrences().last().unwrap();
    let group = elements(&tree, "group")[0];
    let outer = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=host @children=group}}}",
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(
        tree.clone(),
        Some(context(&tree, &elements(&tree, "schema"))),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let independent = host.register_scope(
        tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    assert!(host.assign_subtree_scope(&tree, reference, independent));
    let error = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(body_context(&tree, &elements(&tree, "chosen"))),
        )
        .unwrap_err();
    assert_eq!(error, ReferenceResolutionError::InvalidScopeHandoff);
    assert_eq!(
        host.scope(&host.source_reference(source(&tree, group))),
        Some(original)
    );
    assert_eq!(
        host.scope(&host.source_reference(source(&tree, reference))),
        Some(independent)
    );
    let captured_scope = host
        .register_lexical_scope_with_policy_overrides(
            original,
            None,
            ReferenceScopePolicyOverrides::default(),
        )
        .unwrap();
    assert!(host.assign_subtree_scope(&tree, reference, captured_scope));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = host.validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |request| match request {
                SchemaHostRuntimeContextRequest::Body(_) => {
                    Some(body_context(&tree, &elements(&tree, "chosen")))
                }
                SchemaHostRuntimeContextRequest::Occurrence { .. } => {
                    panic!("caller lifecycle hook failed")
                }
            },
        );
    }));
    assert!(panic.is_err());
    assert_eq!(
        host.scope(&host.source_reference(source(&tree, group))),
        Some(original)
    );
    assert_eq!(
        host.scope(&host.source_reference(source(&tree, reference))),
        Some(captured_scope)
    );
    let retry = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| Some(body_context(&tree, &elements(&tree, "chosen"))),
        )
        .unwrap();
    assert!(
        retry.validation.complete && !retry.validation.failed,
        "{:?}",
        retry.validation.diagnostics
    );
}

#[test]
fn host_attribute_inputs_stay_enclosing_and_body_expressions_retain_target_descendants() {
    use cem_ml::schema::{
        document_model::compile_schema_document_model,
        reference_policy::ReferenceScopePolicyOverrides,
    };
    let (captured,tree) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=target}} {attributes | {attribute @name=target @type=schema:node}}} {host @own={#items} @schema-select=library | {child @target={items}}} {outside} {chosen | {#missing}}");
    let outer = compile_schema_document_model("base","{schema | {elements | {element @name=host @required-attributes=own @children=child}} {attributes | {attribute @name=own @type=schema:node}}}");
    let root_context = body_context(&tree, &elements(&tree, "outside")).with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(tree.clone(), elements(&tree, "schema")[0])
                .unwrap()
                .query_item(),
        )),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(root_context.clone()), policy());
    host.attach_captured_lexical_scopes_with_policy_overrides(&captured, |_, _, _| {
        (
            Some(root_context.clone()),
            ReferenceScopePolicyOverrides::default(),
        )
    })
    .unwrap();
    let mut occurrences = vec![];
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |request| {
                if let SchemaHostRuntimeContextRequest::Occurrence { source, .. } = request {
                    occurrences.push(source.node_id());
                }
                Some(body_context(&tree, &elements(&tree, "chosen")))
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    let hostnode = report
        .validation
        .nodes
        .iter()
        .find(|node| node.source.node_id() == elements(&tree, "host")[0])
        .unwrap();
    let hostvalue = &hostnode.attribute_values[0].access;
    assert_eq!(
        hostvalue.node(hostvalue.roots()[0]).unwrap().node_id(),
        elements(&tree, "outside")[0]
    );
    let childnode = report
        .validation
        .nodes
        .iter()
        .find(|node| node.source.node_id() == elements(&tree, "child")[0])
        .unwrap();
    let bodyvalue = &childnode.attribute_values[0].access;
    let root = bodyvalue.roots()[0];
    assert_eq!(
        bodyvalue.node(root).unwrap().node_id(),
        elements(&tree, "chosen")[0]
    );
    let descendant = bodyvalue
        .children(root)
        .unwrap()
        .iter()
        .filter_map(|index| bodyvalue.node(*index))
        .find(|node| matches!(node.node(), CemAstNode::Reference { .. }))
        .expect("authored target reference remains navigable");
    assert!(
        matches!(descendant.node(),CemAstNode::Reference {expression,targets:None,..} if expression=="#missing")
    );
    assert_eq!(occurrences.len(), 1);
    assert_ne!(occurrences[0], descendant.node_id());
}

#[test]
fn wrapping_body_controls_activate_nested_models_and_restore_following_contexts() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured, tree) = capture("@ns s = https://cem.dev/ns/schema/1\n@ns c = https://cem.dev/ns/core/1\n{s:schema | {elements | {element @name=schema @children=deep} {element @name=inner}}} {s:schema | {elements | {element @name=deep}}} {c:schema @select=library | {schema @select={#library} | {#items}} {#items}} {sibling | {#items}} {inner} {deep} {outside}");
    let schemas = elements(&tree, "schema");
    let outer = compile_schema_document_model("base", "{schema | {elements | {element @name=schema @children='schema inner' @optional-attributes=select} {element @name=sibling @children=outside} {element @name=outside}} {attributes | {attribute @name=select @type=integer}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(
        tree.clone(),
        Some(
            body_context(&tree, &[schemas[0]]).with_binding(
                "items",
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(tree.clone(), elements(&tree, "outside")[0])
                        .unwrap()
                        .query_item(),
                )),
            ),
        ),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    let mut requests = vec![];
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &[schemas[2], elements(&tree, "sibling")[0]],
            &outer,
            policy().limits,
            |request| {
                let SchemaHostRuntimeContextRequest::Body(region) = request else {
                    panic!("no local occurrence frames")
                };
                let boundary = region.contract.host().node_id();
                requests.push(boundary);
                let name = if boundary == schemas[2] {
                    "inner"
                } else {
                    "deep"
                };
                let chosen = if boundary == schemas[2] {
                    schemas[1]
                } else {
                    schemas[0]
                };
                Some(
                    context(&tree, &[chosen]).with_binding(
                        "items",
                        StandaloneExpressionBinding::any(ItemStream::once(
                            RetainedCemNode::new(tree.clone(), elements(&tree, name)[0])
                                .unwrap()
                                .query_item(),
                        )),
                    ),
                )
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(requests, vec![schemas[2], schemas[3]]);
    assert_eq!(report.inputs.len(), 2);
    assert_eq!(report.scopes.len(), 2);
    let names: Vec<_> = report
        .validation
        .nodes
        .iter()
        .filter_map(|node| match node.source.node() {
            CemAstNode::Element { expanded_name, .. } => Some(expanded_name.local_name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        vec!["schema", "schema", "deep", "inner", "sibling", "outside"]
    );
    assert!(report
        .validation
        .nodes
        .iter()
        .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
    assert!(report
        .validation
        .nodes
        .iter()
        .all(|node| node.attribute_values.is_empty()));
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
    use cem_ml::schema::declaration_references::SchemaDeclarationHost;
    use cem_ml::value::reference_resolution::ReferenceResolutionHost;
    for id in captured.occurrences() {
        assert_eq!(
            host.scope(&host.source_reference(source(&tree, id))),
            Some(original)
        );
    }
}

#[test]
fn empty_wrapping_and_pending_body_controls_do_not_borrow_enclosing_inputs() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let outer = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=schema @children=inner} {element @name=inner}}}",
    );
    for (control, body, ready, expected_requests) in [
        ("@select=library", "", true, 1),
        ("@select={#library}", "{inner}", false, 1),
        ("@src=./external.cem", "{#items}", true, 0),
        ("@select=library @src=./external.cem", "{#items}", true, 0),
        ("@select=missing", "{#items}", true, 0),
    ] {
        let text=format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {{elements | {{element @name=inner}}}}}} {{schema {control} | {body}}}");
        let (captured, tree) = capture(&text);
        let schemas = elements(&tree, "schema");
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(tree.clone(), Some(context(&tree, &[schemas[0]])), policy());
        host.attach_captured_names(&captured).unwrap();
        let mut requests = 0;
        let report = host
            .validate_input_runtime_host_regions(
                "child",
                tree.clone(),
                &[schemas[1]],
                &outer,
                policy().limits,
                |_| {
                    requests += 1;
                    ready.then(StandaloneExpressionContext::default)
                },
            )
            .unwrap();
        assert_eq!(requests, expected_requests, "{control}");
        let complete = body.is_empty();
        assert_eq!(
            report.validation.complete, complete,
            "{control}: {:?}",
            report.validation.diagnostics
        );
        assert_eq!(
            report
                .validation
                .nodes
                .iter()
                .filter(|node| matches!(node.source.node(), CemAstNode::Element { .. }))
                .count(),
            1
        );
        assert!(report.validation.references.is_empty());
        assert!(!report
            .validation
            .diagnostics
            .iter()
            .any(|d| d.code == cem_ml::schema::document_model::UNKNOWN_ATTRIBUTE_CODE));
        if complete {
            assert!(report.inputs[0].is_ready());
            assert_eq!(report.scopes.len(), 1);
        } else {
            assert!(!report.inputs[0].is_ready());
            assert!(report.scopes.is_empty());
        }
    }
}

#[test]
fn repeated_reference_selected_wrappers_share_preparation_and_keep_traversal_bounds() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured,tree)=capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=inner}}} {schema @select={#library} | {#items}} {host | {#wrappers}} {inner}");
    let schemas = elements(&tree, "schema");
    let outer=compile_schema_document_model("base","{schema | {elements | {element @name=host @children=schema} {element @name=schema @children=inner}}}");
    let ctx = context(&tree, &[schemas[0]])
        .with_binding(
            "wrappers",
            StandaloneExpressionBinding::any(ItemStream::from_items(vec![
                RetainedCemNode::new(
                    tree.clone(),
                    schemas[1]
                )
                .unwrap()
                .query_item();
                2
            ])),
        )
        .with_binding(
            "items",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(tree.clone(), elements(&tree, "inner")[0])
                    .unwrap()
                    .query_item(),
            )),
        );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(ctx.clone()), policy());
    host.attach_captured_names(&captured).unwrap();
    let mut requests = 0;
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            policy().limits,
            |_| {
                requests += 1;
                Some(ctx.clone())
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(requests, 1);
    assert_eq!(report.inputs.len(), 1);
    assert_eq!(
        report
            .validation
            .nodes
            .iter()
            .filter(|node| node.source.node_id() == schemas[1])
            .count(),
        2
    );
    assert_eq!(
        report
            .validation
            .nodes
            .iter()
            .filter(|node| node.source.node_id() == elements(&tree, "inner")[0])
            .count(),
        2
    );
    let mut requests = 0;
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &elements(&tree, "host"),
            &outer,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_depth: 128,
                max_work: 2,
            },
            |_| {
                requests += 1;
                Some(ctx.clone())
            },
        )
        .unwrap();
    assert!(!report.validation.complete);
    assert!(report
        .validation
        .references
        .iter()
        .all(|r| r.resolution.work_used <= 2));
    assert!(report.validation.references.iter().any(|r| r
        .resolution
        .issues
        .iter()
        .any(|issue| issue.kind
            == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::WorkLimit)));
    assert!(requests <= 1);
}

#[test]
fn imported_wrapping_controls_consume_native_children_with_original_source_form() {
    use cem_ml::{
        import::import_xml_ast_with_lexical_scopes,
        schema::document_model::compile_schema_document_model,
        validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
    };
    let text="<root xmlns:c='https://cem.dev/ns/core/1' xmlns:s='https://cem.dev/ns/schema/1' xmlns:r='https://cem.dev/ns/cem-ml/1'><s:schema><elements><element name='inner'/></elements></s:schema><c:schema select='library'><r:expr>#items</r:expr></c:schema><c:schema select='library'></c:schema><inner/></root>";
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: text.as_bytes(),
        source_uri: "wrapper.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty());
    let imported =
        import_xml_ast_with_lexical_scopes(&document.unwrap(), CompiledSchema::cem_core()).unwrap();
    let tree = RetainedCemTree::from_shared(
        imported.captured.document().clone(),
        "wrapper.xml",
        text,
        imported.semantics,
        None,
    )
    .unwrap();
    let schemas = elements(&tree, "schema");
    let outer = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=schema @children=inner}}}",
    );
    let ctx = context(&tree, &[schemas[0]]).with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(tree.clone(), elements(&tree, "inner")[0])
                .unwrap()
                .query_item(),
        )),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(ctx.clone()), policy());
    host.attach_captured_names(&imported.captured).unwrap();
    let mut requests = 0;
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &schemas[1..3],
            &outer,
            policy().limits,
            |_| {
                requests += 1;
                Some(ctx.clone())
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(requests, 2);
    assert_eq!(report.inputs.len(), 2);
    assert_eq!(report.validation.references.len(), 1);
    assert!(report
        .validation
        .nodes
        .iter()
        .any(|node| node.source.node_id() == elements(&tree, "inner")[0]));
    assert!(report
        .validation
        .nodes
        .iter()
        .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn following_switch_keeps_its_enclosing_contract_and_changes_only_later_siblings() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    use cem_ml::schema::scope_controls::SchemaScopeControlExtent;
    let (captured,tree)=capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=after @required-attributes=inner @children=item} {element @name=item}}} {host | {before @outer=yes} {schema @own=yes @select={#library}} {after @inner=yes | {#items}} {#items}} {outside @outer=yes} {item}");
    let schemas = elements(&tree, "schema");
    let outer=compile_schema_document_model("base","{schema | {elements | {element @name=host @children='before schema after item'} {element @name=before @required-attributes=outer} {element @name=schema @required-attributes=own} {element @name=after @required-attributes=outer} {element @name=outside @required-attributes=outer}}}");
    let ctx = context(&tree, &[schemas[0]]).with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(tree.clone(), elements(&tree, "item")[0])
                .unwrap()
                .query_item(),
        )),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(tree.clone(), Some(ctx.clone()), policy());
    host.attach_captured_names(&captured).unwrap();
    let mut requests = 0;
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &[elements(&tree, "host")[0], elements(&tree, "outside")[0]],
            &outer,
            policy().limits,
            |request| {
                let SchemaHostRuntimeContextRequest::Body(region) = request else {
                    panic!("no local occurrence frames")
                };
                assert_eq!(
                    region.contract.extent(),
                    SchemaScopeControlExtent::Following
                );
                requests += 1;
                Some(ctx.clone())
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(requests, 1);
    assert_eq!(report.scopes.len(), 1);
    assert_eq!(report.validation.references.len(), 2);
    assert_eq!(
        report
            .validation
            .nodes
            .iter()
            .filter(|node| node.source.node_id() == elements(&tree, "item")[0])
            .count(),
        2
    );
    assert!(report
        .validation
        .nodes
        .iter()
        .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
    use cem_ml::schema::declaration_references::SchemaDeclarationHost;
    use cem_ml::value::reference_resolution::ReferenceResolutionHost;
    for id in captured.occurrences() {
        assert_eq!(
            host.scope(&host.source_reference(source(&tree, id))),
            Some(original)
        );
    }
}

#[test]
fn unavailable_following_regions_preserve_earlier_and_outer_validation_without_fallback() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let outer=compile_schema_document_model("base","{schema | {elements | {element @name=host @children='before schema after'} {element @name=before @required-attributes=outer} {element @name=schema @required-attributes=own} {element @name=after @required-attributes=outer} {element @name=outside @required-attributes=outer}}}");
    for control in [
        "@select=library",
        "@select={#library}",
        "@src=./missing.cem",
        "@select=missing",
        "@select=library @src=invalid",
    ] {
        let text=format!("@ns s = https://cem.dev/ns/schema/1\n{{s:schema | {{elements | {{element @name=after @required-attributes=inner}}}}}} {{host | {{before @outer=yes}} {{schema @own=yes {control}}} {{after @inner=yes | {{#missing}}}} }} {{outside @outer=yes}}");
        let (captured, tree) = capture(&text);
        let schemas = elements(&tree, "schema");
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(tree.clone(), Some(context(&tree, &[schemas[0]])), policy());
        host.attach_captured_names(&captured).unwrap();
        let report = host
            .validate_input_runtime_host_regions(
                "child",
                tree.clone(),
                &[elements(&tree, "host")[0], elements(&tree, "outside")[0]],
                &outer,
                policy().limits,
                |_| None,
            )
            .unwrap();
        assert!(!report.validation.complete, "{control}");
        assert_eq!(report.inputs.len(), 1, "{control}");
        assert!(report.scopes.is_empty());
        assert!(report
            .validation
            .nodes
            .iter()
            .any(|node| node.source.node_id() == elements(&tree, "before")[0]));
        assert!(report
            .validation
            .nodes
            .iter()
            .any(|node| node.source.node_id() == elements(&tree, "outside")[0]));
        assert!(report.validation.references.is_empty());
        assert!(
            !report
                .validation
                .diagnostics
                .iter()
                .any(|d| d.message.contains("outer")
                    || d.code == cem_ml::schema::document_model::UNKNOWN_ATTRIBUTE_CODE),
            "{control}: {:?}",
            report.validation.diagnostics
        );
    }
}

#[test]
fn nested_following_switches_restore_outer_regions_and_prepare_once_in_selected_structure() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let (captured,tree)=capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=nested @children='schema deep'} {element @name=schema} {element @name=after @required-attributes=inner}}} {s:schema | {elements | {element @name=deep @required-attributes=deep}}} {host | {schema @select=library} {nested | {schema @select={#library}} {deep @deep=yes}} {after @inner=yes}} {outside @outer=yes} {container | {#hosts}}");
    let schemas = elements(&tree, "schema");
    let outer=compile_schema_document_model("base","{schema | {elements | {element @name=container @children=host} {element @name=host @children='schema nested after'} {element @name=schema} {element @name=outside @required-attributes=outer}}}");
    let mut ctx = context(&tree, &[schemas[0]]).with_binding(
        "hosts",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![
            RetainedCemNode::new(
                tree.clone(),
                elements(&tree, "host")[0]
            )
            .unwrap()
            .query_item();
            2
        ])),
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(tree.clone(), Some(ctx.clone()), policy());
    host.attach_captured_names(&captured).unwrap();
    let mut requests = vec![];
    let report = host
        .validate_input_runtime_host_regions(
            "child",
            tree.clone(),
            &[
                elements(&tree, "container")[0],
                elements(&tree, "outside")[0],
            ],
            &outer,
            policy().limits,
            |request| {
                let SchemaHostRuntimeContextRequest::Body(region) = request else {
                    panic!("no local frames")
                };
                let id = region.contract.host().node_id();
                requests.push(id);
                // The second selector reads the first region's inputs; its body has
                // explicit independent inputs even though it has no references.
                ctx = context(&tree, &[schemas[1]]);
                Some(ctx.clone())
            },
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(requests, vec![schemas[2], schemas[3]]);
    assert_eq!(report.inputs.len(), 2);
    assert_eq!(
        report
            .validation
            .nodes
            .iter()
            .filter(|node| node.source.node_id() == elements(&tree, "deep")[0])
            .count(),
        2
    );
    assert_eq!(
        report
            .validation
            .nodes
            .iter()
            .filter(|node| node.source.node_id() == elements(&tree, "after")[0])
            .count(),
        2
    );
}

#[test]
fn following_switches_share_authorization_and_do_not_reset_active_work_budgets() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    use cem_ml::value::reference_resolution::ReferenceResolutionIssueKind;
    let (input_capture, input) = capture("{container | {#hosts}}");
    let (vendor_capture,vendor)=capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @children=item} {element @name=item}}} {host | {schema @select=library} {child | {#items}}} {item}");
    let schemas = elements(&vendor, "schema");
    let item = elements(&vendor, "item")[0];
    let ctx = context(&vendor, &[schemas[0]]).with_binding(
        "items",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(vendor.clone(), item)
                .unwrap()
                .query_item(),
        )),
    );
    let input_ctx = StandaloneExpressionContext::default().with_binding(
        "hosts",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(vendor.clone(), elements(&vendor, "host")[0])
                .unwrap()
                .query_item(),
        )),
    );
    let outer=compile_schema_document_model("base","{schema | {elements | {element @name=container @children=host} {element @name=host @children='schema child'} {element @name=schema}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(input.clone(), Some(input_ctx), policy());
    let destination = host.register_scope(vendor.clone(), Some(ctx.clone()), policy());
    host.attach_captured_names(&input_capture).unwrap();
    host.attach_captured_names(&vendor_capture).unwrap();
    let mut requests = 0;
    let denied = host
        .validate_input_runtime_host_regions(
            "child",
            input.clone(),
            &elements(&input, "container"),
            &outer,
            policy().limits,
            |_| {
                requests += 1;
                Some(ctx.clone())
            },
        )
        .unwrap();
    assert!(!denied.validation.complete);
    assert_eq!(requests, 0);
    assert!(denied.inputs.is_empty());
    assert!(denied.validation.references.iter().any(|r| r
        .resolution
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied)));
    host.allow_scope_crossing(origin, destination);
    let ready = host
        .validate_input_runtime_host_regions(
            "child",
            input.clone(),
            &elements(&input, "container"),
            &outer,
            policy().limits,
            |_| Some(ctx.clone()),
        )
        .unwrap();
    assert!(
        ready.validation.complete && !ready.validation.failed,
        "{:?}",
        ready.validation.diagnostics
    );
    assert_eq!(ready.inputs.len(), 1);
    assert!(ready
        .validation
        .nodes
        .iter()
        .any(|node| node.source.node_id() == item
            && Arc::ptr_eq(node.source.document(), vendor.ast_owner())));
    let limited = host
        .validate_input_runtime_host_regions(
            "child",
            input.clone(),
            &elements(&input, "container"),
            &outer,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_depth: 128,
                max_work: 4,
            },
            |_| Some(ctx.clone()),
        )
        .unwrap();
    assert!(!limited.validation.complete);
    assert!(limited
        .validation
        .references
        .iter()
        .all(|r| r.resolution.work_used <= 4));
    assert!(limited.validation.references.iter().any(|r| r
        .resolution
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit)));
    assert!(vendor.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn imported_following_switches_retry_fresh_contexts_and_restore_original_assignments() {
    use cem_ml::{
        import::import_xml_ast_with_lexical_scopes,
        schema::{
            declaration_references::SchemaDeclarationHost,
            document_model::compile_schema_document_model,
            scope_controls::SchemaScopeControlExtent,
        },
        validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
        value::reference_resolution::ReferenceResolutionHost,
    };
    let text="<root xmlns:c='https://cem.dev/ns/core/1' xmlns:s='https://cem.dev/ns/schema/1' xmlns:r='https://cem.dev/ns/cem-ml/1'><s:schema><elements><element name='inner' children='first second'/><element name='first'/><element name='second'/></elements></s:schema><host><c:schema select='library'/><inner><r:expr>#items</r:expr></inner></host><outside own='yes'/><first/><second/></root>";
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: text.as_bytes(),
        source_uri: "following.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty());
    let imported =
        import_xml_ast_with_lexical_scopes(&document.unwrap(), CompiledSchema::cem_core()).unwrap();
    let tree = RetainedCemTree::from_shared(
        imported.captured.document().clone(),
        "following.xml",
        text,
        imported.semantics,
        None,
    )
    .unwrap();
    let schemas = elements(&tree, "schema");
    let outer=compile_schema_document_model("base","{schema | {elements | {element @name=host @children='schema inner'} {element @name=schema} {element @name=outside @required-attributes=own}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(tree.clone(), Some(context(&tree, &[schemas[0]])), policy());
    host.attach_captured_names(&imported.captured).unwrap();
    for selected in [Some("first"), None, Some("second")] {
        let mut requests = 0;
        let report = host
            .validate_input_runtime_host_regions(
                "child",
                tree.clone(),
                &[elements(&tree, "host")[0], elements(&tree, "outside")[0]],
                &outer,
                policy().limits,
                |request| {
                    let SchemaHostRuntimeContextRequest::Body(region) = request else {
                        panic!("no local occurrence frames")
                    };
                    assert_eq!(
                        region.contract.extent(),
                        SchemaScopeControlExtent::Following
                    );
                    requests += 1;
                    selected.map(|name| {
                        StandaloneExpressionContext::default().with_binding(
                            "items",
                            StandaloneExpressionBinding::any(ItemStream::once(
                                RetainedCemNode::new(tree.clone(), elements(&tree, name)[0])
                                    .unwrap()
                                    .query_item(),
                            )),
                        )
                    })
                },
            )
            .unwrap();
        assert_eq!(requests, 1);
        assert_eq!(report.validation.complete, selected.is_some());
        assert!(
            !report.validation.failed,
            "{:?}",
            report.validation.diagnostics
        );
        if let Some(name) = selected {
            assert_eq!(report.scopes.len(), 1);
            assert_eq!(report.validation.references.len(), 1);
            assert!(report
                .validation
                .nodes
                .iter()
                .any(|node| node.source.node_id() == elements(&tree, name)[0]));
            let other = if name == "first" { "second" } else { "first" };
            assert!(!report
                .validation
                .nodes
                .iter()
                .any(|node| node.source.node_id() == elements(&tree, other)[0]));
        } else {
            assert!(report.scopes.is_empty());
            assert!(report.validation.references.is_empty());
        }
        for id in imported.captured.occurrences() {
            assert_eq!(
                host.scope(&host.source_reference(source(&tree, id))),
                Some(original)
            );
        }
        assert!(tree.ast().nodes.iter().all(|node| !matches!(
            node,
            CemAstNode::Reference {
                targets: Some(_),
                ..
            }
        )));
    }
}
#[test]
fn entered_document_preludes_use_fresh_literal_inputs_and_restore_nested_regions() {
    use cem_ml::schema::declaration_references::SchemaDeclarationHost;
    use cem_ml::schema::document_model::compile_schema_document_model;
    use cem_ml::value::reference_resolution::ReferenceResolutionHost;
    let text = "@ns s = https://cem.dev/ns/schema/1\n@schema select=library\n{host @schema-select=library | {inner @nested=yes | {#items}}} {after @outer=yes} {first} {second} {s:schema | {elements | {element @name=host @children=inner} {element @name=after @required-attributes=outer}}} {s:schema | {elements | {element @name=inner @required-attributes=nested @children='first second'} {element @name=first} {element @name=second}}}";
    let (captured, tree) = capture(text);
    let directives = elements(&tree, "@schema");
    let schemas = elements(&tree, "schema");
    let outer = compile_schema_document_model("base", "{schema | {elements | {element @name=host} {element @name=after @required-attributes=wrong}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let original = host.register_scope(tree.clone(), Some(context(&tree, &[schemas[0]])), policy());
    host.attach_captured_names(&captured).unwrap();
    for target in [Some("first"), None, Some("second")] {
        let mut requests = vec![];
        let report = host
            .validate_input_runtime_host_regions(
                "child",
                tree.clone(),
                &[
                    directives[0],
                    elements(&tree, "host")[0],
                    elements(&tree, "after")[0],
                ],
                &outer,
                policy().limits,
                |request| {
                    let SchemaHostRuntimeContextRequest::Body(region) = request else {
                        panic!("no local frames")
                    };
                    requests.push(region.contract.host().node_id());
                    if region.contract.host().node_id() == directives[0] {
                        Some(context(&tree, &[schemas[1]]))
                    } else {
                        target.map(|name| body_context(&tree, &elements(&tree, name)))
                    }
                },
            )
            .unwrap();
        assert_eq!(requests, vec![directives[0], elements(&tree, "host")[0]]);
        assert_eq!(report.inputs.len(), 2);
        assert_eq!(
            report.validation.complete,
            target.is_some(),
            "{:?}",
            report.validation.diagnostics
        );
        assert!(
            !report.validation.failed,
            "{:?}",
            report.validation.diagnostics
        );
        assert_eq!(report.scopes.len(), if target.is_some() { 2 } else { 1 });
        assert_eq!(
            report.validation.references.len(),
            usize::from(target.is_some())
        );
        if let Some(target) = target {
            assert!(report
                .validation
                .nodes
                .iter()
                .any(|node| node.source.node_id() == elements(&tree, target)[0]));
        }
        assert!(report
            .validation
            .nodes
            .iter()
            .all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
        for id in captured.occurrences() {
            assert_eq!(
                host.scope(&host.source_reference(source(&tree, id))),
                Some(original)
            );
        }
    }
    assert!(tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn unavailable_prelude_controls_leave_following_region_incomplete_without_fallback() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let outer = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=after @required-attributes=wrong}}}",
    );
    for body in [
        "src=./pending.cem",
        "select=missing",
        "select=library src=other",
        "select=''",
        "",
    ] {
        let (captured, tree) = capture(&format!(
            "@schema {body}\n{{after @selected=yes | {{#missing}}}}"
        ));
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            tree.clone(),
            Some(StandaloneExpressionContext::default()),
            policy(),
        );
        host.attach_captured_names(&captured).unwrap();
        let mut requests = 0;
        let report = host
            .validate_input_runtime_host_regions(
                "child",
                tree.clone(),
                &[elements(&tree, "@schema")[0], elements(&tree, "after")[0]],
                &outer,
                policy().limits,
                |_| {
                    requests += 1;
                    None
                },
            )
            .unwrap();
        assert_eq!(requests, 0, "{body}");
        assert_eq!(report.inputs.len(), 1, "{body}");
        assert!(!report.validation.complete, "{body}");
        assert!(report.scopes.is_empty());
        assert!(report.validation.references.is_empty());
        assert!(
            !report
                .validation
                .diagnostics
                .iter()
                .any(|d| d.message.contains("wrong")
                    || d.code == cem_ml::schema::document_model::UNKNOWN_ATTRIBUTE_CODE),
            "{body}: {:?}",
            report.validation.diagnostics
        );
        assert!(report
            .validation
            .nodes
            .iter()
            .any(|node| node.source.node_id() == elements(&tree, "after")[0]));
    }
}

#[test]
fn uri_runtime_regions_retry_loader_snapshots_across_existing_control_forms() {
    use cem_ml::schema::{
        declaration_references::SchemaDeclarationHost,
        document_model::compile_schema_document_model,
    };
    use cem_ml::value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost};
    let (loaded, library) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child @required-attributes=selected @children=item} {element @name=item}}} {s:schema | {elements | {element @name=child @required-attributes=wrong @children=item} {element @name=item}}}");
    let schemas = elements(&library, "schema");
    let outer = compile_schema_document_model("base", "{schema | {elements | {element @name=host @children='schema child'} {element @name=schema @required-attributes=own @children=child} {element @name=child @required-attributes=enclosing} {element @name=outside @required-attributes=outer}}}");
    for (text, control_name, root_names) in [
        ("{host @schema-src=./schema.cem | {child @selected=yes | {#items}}} {outside @outer=yes} {item}", "host", vec!["host", "outside"]),
        ("{schema @src=./schema.cem @own=yes | {child @selected=yes | {#items}}} {outside @outer=yes} {item}", "schema", vec!["schema", "outside"]),
        ("{host | {schema @src=./schema.cem @own=yes} {child @selected=yes | {#items}}} {outside @outer=yes} {item}", "schema", vec!["host", "outside"]),
        ("@schema src=./schema.cem\n{child @selected=yes | {#items}} {item}", "@schema", vec!["@schema", "child"]),
    ] {
        let (captured, tree) = capture(text);
        let boundary = source(&tree, elements(&tree, control_name)[0]);
        let roots: Vec<_> = root_names.iter().map(|name| elements(&tree, name)[0]).collect();
        let ctx = body_context(&tree, &elements(&tree, "item"));
        let mut host = CemQlSchemaDeclarationHost::new();
        let original = host.register_scope(tree.clone(), Some(ctx.clone()), policy());
        let destination = host.register_scope(library.clone(), Some(StandaloneExpressionContext::default()), policy());
        host.attach_captured_names(&captured).unwrap();
        host.attach_captured_names(&loaded).unwrap();
        host.allow_scope_crossing(original, destination);
        // Another spelling cannot satisfy this original control's authored URI.
        host.set_schema_uri_load(&boundary, "./other.cem", ReferenceLinkEvaluation::Resolved(vec![source(&library, schemas[0])])).unwrap();
        for chosen in [Some(0), None, Some(1), Some(0)] {
            host.set_schema_uri_load(&boundary, "./schema.cem", match chosen {
                Some(index) => ReferenceLinkEvaluation::Resolved(vec![source(&library, schemas[index])]),
                None => ReferenceLinkEvaluation::Pending("reload".into()),
            }).unwrap();
            let mut requests = 0;
            let report = host.validate_input_runtime_host_regions("child", tree.clone(), &roots, &outer, policy().limits, |request| {
                let SchemaHostRuntimeContextRequest::Body(region) = request else { panic!("no local frames"); };
                assert_eq!(region.contract.host().node_id(), boundary.node_id());
                requests += 1;
                Some(ctx.clone())
            }).unwrap();
            assert_eq!(requests, usize::from(chosen.is_some()), "{control_name}");
            assert_eq!(report.validation.complete, chosen.is_some(), "{control_name}: {:?}", report.validation.diagnostics);
            assert_eq!(report.validation.failed, chosen == Some(1), "{control_name}: {:?}", report.validation.diagnostics);
            assert_eq!(report.scopes.len(), usize::from(chosen.is_some()));
            assert_eq!(report.validation.references.len(), usize::from(chosen.is_some()));
            assert!(!report.validation.diagnostics.iter().any(|d| d.message.contains("enclosing")));
            for id in captured.occurrences() {
                assert_eq!(host.scope(&host.source_reference(source(&tree, id))), Some(original));
            }
            assert!(report.validation.nodes.iter().all(|node| Arc::ptr_eq(node.source.document(), tree.ast_owner())));
        }
        assert!(tree.ast().nodes.iter().all(|node| !matches!(node, CemAstNode::Reference { targets: Some(_), .. })));
    }
}

#[test]
fn nested_uri_controls_keep_independent_loads_and_restore_document_governance() {
    use cem_ml::schema::{
        declaration_references::SchemaDeclarationHost,
        document_model::compile_schema_document_model,
    };
    use cem_ml::value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost};
    let (captured, tree) = capture("@schema src=external.cem\n{host @schema-src=external.cem | {child @selected=yes | {#items}}} {outside @outer=yes} {item}");
    let (loaded, library) = capture("@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=host @children=child} {element @name=outside @required-attributes=outer}}} {s:schema | {elements | {element @name=child @required-attributes=selected @children=item} {element @name=item}}} {s:schema | {attributes | {attribute @name=value @type=string @pattern='['}}}");
    let schemas = elements(&library, "schema");
    let directive = source(&tree, elements(&tree, "@schema")[0]);
    let inner = source(&tree, elements(&tree, "host")[0]);
    let roots = [
        directive.node_id(),
        inner.node_id(),
        elements(&tree, "outside")[0],
    ];
    let outer = compile_schema_document_model(
        "base",
        "{schema | {elements | {element @name=outside @required-attributes=wrong}}}",
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let ctx = body_context(&tree, &elements(&tree, "item"));
    let original = host.register_scope(tree.clone(), Some(ctx.clone()), policy());
    let destination = host.register_scope(
        library.clone(),
        Some(StandaloneExpressionContext::default()),
        policy(),
    );
    host.attach_captured_names(&captured).unwrap();
    host.attach_captured_names(&loaded).unwrap();
    host.allow_scope_crossing(original, destination);
    host.set_schema_uri_load(
        &directive,
        "external.cem",
        ReferenceLinkEvaluation::Resolved(vec![source(&library, schemas[0])]),
    )
    .unwrap();
    for chosen in [Some(1), None, Some(2), Some(1)] {
        host.set_schema_uri_load(
            &inner,
            "external.cem",
            chosen.map_or_else(
                || ReferenceLinkEvaluation::Pending("inner-reload".into()),
                |index| ReferenceLinkEvaluation::Resolved(vec![source(&library, schemas[index])]),
            ),
        )
        .unwrap();
        let mut requests = vec![];
        let report = host
            .validate_input_runtime_host_regions(
                "child",
                tree.clone(),
                &roots,
                &outer,
                policy().limits,
                |request| {
                    let SchemaHostRuntimeContextRequest::Body(region) = request else {
                        panic!("no local frames");
                    };
                    requests.push(region.contract.host().node_id());
                    Some(ctx.clone())
                },
            )
            .unwrap();
        assert_eq!(
            report.validation.complete,
            chosen == Some(1),
            "{:?}",
            report.validation.diagnostics
        );
        assert_eq!(
            report.validation.failed,
            chosen == Some(2),
            "{:?}",
            report.validation.diagnostics
        );
        assert_eq!(requests.len(), if chosen == Some(1) { 2 } else { 1 });
        assert_eq!(requests[0], directive.node_id());
        assert_eq!(report.scopes.len(), if chosen == Some(1) { 2 } else { 1 });
        assert_eq!(
            report.validation.references.len(),
            usize::from(chosen == Some(1))
        );
        assert!(report
            .validation
            .nodes
            .iter()
            .any(|node| node.source.node_id() == roots[2]));
        assert!(!report
            .validation
            .diagnostics
            .iter()
            .any(|d| d.message.contains("wrong")));
        for id in captured.occurrences() {
            assert_eq!(
                host.scope(&host.source_reference(source(&tree, id))),
                Some(original)
            );
        }
    }
}
