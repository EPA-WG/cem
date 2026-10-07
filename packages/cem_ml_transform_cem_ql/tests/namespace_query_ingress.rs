use cem_ml::{
    engine::{EngineInput, FormatIdentity, InputFormat},
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    parser::CemAstNode,
    query::{
        run_query_with_source_owner, QueryRunError, QueryRunRequest, QueryRunResponse, QuerySource,
        QuerySourceOwner,
    },
    run_config::ScopeConfig,
    schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::{
            admit_namespace_scope_target, NamespaceNameCompletion, NamespaceNameCompletionError,
        },
        registry::CEM_QL_EXPRESSION_CONTENT_TYPE,
        vocab::CompiledSchema,
    },
};
use cem_ml_transform_cem_ql::{
    engine_context_with_cem_ql_template_adapter, CemQlNativeItemsOwner, CemQlQueryResultArtifact,
};
use cem_ql::eval::{retained_cem_node, AtomValue, Item, QueryContextScope};
use std::{collections::BTreeMap, sync::Arc};

fn import(text: &str) -> ScopedCemImport {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "memory:namespace-input.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
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
fn completed(input: &ScopedCemImport, roots: &[u32], uri: &str) -> Arc<NamespaceNameCompletion> {
    let vendor = import(&format!("@ns public = {uri}\n{{public:item}}"));
    let target = admit_namespace_scope_target(
        SchemaDeclarationNode::new(vendor.tree.ast_owner().clone(), element(&vendor, "@ns"))
            .unwrap(),
        &vendor.captured,
    )
    .unwrap();
    Arc::new(
        NamespaceNameCompletion::new(
            input.captured.clone(),
            roots,
            BTreeMap::from([(declaration(input), target)]),
        )
        .unwrap(),
    )
}
fn request(expression: &str) -> QueryRunRequest {
    QueryRunRequest {
        // Retained-owner execution must not try to parse these bytes.
        data: EngineInput {
            uri: "memory:namespace-input.cem".into(),
            bytes: vec![0xff],
            from_format: Some(InputFormat::Cem),
            identity: Some(FormatIdentity {
                content_type: Some("text/cem-ml".into()),
                ..Default::default()
            }),
            root_scope: ScopeConfig::default(),
        },
        query: QuerySource {
            uri: "memory:namespace-query.cemql".into(),
            bytes: expression.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some(CEM_QL_EXPRESSION_CONTENT_TYPE.into()),
                ..Default::default()
            },
        },
        context: engine_context_with_cem_ql_template_adapter(),
        context_item: None,
        bindings: Default::default(),
        limits: None,
    }
}
fn run(
    input: &ScopedCemImport,
    names: Arc<NamespaceNameCompletion>,
    expression: &str,
) -> QueryRunResponse {
    run_query_with_source_owner(
        request(expression),
        QuerySourceOwner::NamespaceCompleted {
            source: input.tree.clone(),
            completion: names,
        },
    )
    .unwrap()
}
fn items(response: &QueryRunResponse) -> &[Item] {
    &response
        .result
        .native_result
        .as_any()
        .downcast_ref::<CemQlQueryResultArtifact>()
        .unwrap()
        .stream()
        .items
}
fn strings(values: &[&str]) -> Vec<Item> {
    values
        .iter()
        .map(|value| Item::Atomic(AtomValue::String((*value).into())))
        .collect()
}

#[test]
fn independent_completion_ingress_exposes_names_and_original_authored_source() {
    let input = import("@ns v = urn:fixed\n{fixed @v:flag=yes}\n{host @xmlns:v={#library} | {v:item @v:flag=yes | {#related}}} {outside}");
    let root = element(&input, "item");
    let fixed = element(&input, "fixed");
    let mut identities = vec![];
    for uri in ["urn:one", "urn:two"] {
        let names = completed(&input, &[root, fixed], uri);
        let response = run(
            &input,
            names.clone(),
            "(input.namespace, input.attributes.namespace, input.source.namespace, input)",
        );
        let result = items(&response);
        assert_eq!(&result[..6], strings(&[uri, "", uri, "urn:fixed", "v", ""]));
        let owner = response
            .result
            .input_ast_owner
            .as_any()
            .downcast_ref::<CemQlNativeItemsOwner>()
            .unwrap();
        assert!(owner.lifecycle_owner().is_none());
        let QuerySourceOwner::NamespaceCompleted { source, completion } = owner.source_owner()
        else {
            panic!("completed owner")
        };
        assert!(Arc::ptr_eq(source, &input.tree));
        assert!(Arc::ptr_eq(completion, &names));
        assert_eq!(owner.stream().items.len(), 2);
        let projected = &result[6];
        // Retain both native views until their execution identities are compared.
        identities.push(projected.clone());
        let original = retained_cem_node(projected).unwrap();
        assert!(Arc::ptr_eq(original.owner(), &input.tree));
        assert_eq!(original.node_id(), root);
        let raw = projected.view().unwrap().field("source").unwrap().remove(0);
        assert_eq!(projected.source_map(), raw.source_map());
        assert!(!projected.source_map().unwrap().frames.is_empty());
    }
    assert_ne!(identities[0].identity(), identities[1].identity());
    assert!(input
        .captured
        .expanded_name(input.tree.ast_owner(), root)
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
fn reference_targets_and_axes_retain_selected_forest_and_authored_descendants() {
    let input = import("{host @xmlns:v={#library} | {v:item @v:flag=yes | content {#related}} {v:sibling}} {outside}");
    let root = element(&input, "item");
    let response = run(
        &input,
        completed(&input, &[root], "urn:ready"),
        "(input.parent, input, #input, input.children)",
    );
    let result = items(&response);
    let projected = &result[0];
    assert!(projected
        .view()
        .unwrap()
        .parent(QueryContextScope(0))
        .unwrap()
        .is_none());
    let targets = result[1].view().unwrap().field("targets").unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].identity(), projected.identity());
    assert!(Arc::ptr_eq(
        retained_cem_node(&targets[0]).unwrap().owner(),
        &input.tree
    ));
    let authored = result[2..]
        .iter()
        .find(|item| {
            matches!(
                retained_cem_node(item).unwrap().node(),
                CemAstNode::Reference { .. }
            )
        })
        .unwrap();
    assert!(
        matches!(retained_cem_node(authored).unwrap().node(), CemAstNode::Reference {expression, targets: None, ..} if expression == "#related")
    );
    assert!(authored.view().unwrap().field("targets").is_none());
    let response = run(&input, completed(&input, &[root], "urn:ready"), "(data:node_key(input), data:line_number(input), data:base_uri(input), data:document_uri(input))");
    let raw = run(&input, completed(&input, &[root], "urn:ready"), "(data:node_key(input.source), data:line_number(input.source), data:base_uri(input.source), data:document_uri(input.source))");
    assert_eq!(items(&response), items(&raw));
}

#[test]
fn mismatched_owner_rejects_before_query_compilation_and_pending_qnames_do_not_enter() {
    let input = import("{host @xmlns:v={#library} | {v:item}}");
    let foreign = import("{host @xmlns:v={#library} | {v:item}}");
    let root = element(&input, "item");
    assert!(
        matches!(NamespaceNameCompletion::new(input.captured.clone(), &[root], BTreeMap::new()), Err(NamespaceNameCompletionError::Pending {declaration: dep, ..}) if dep == declaration(&input))
    );
    let names = completed(&input, &[root], "urn:ready");
    let error = run_query_with_source_owner(
        request("("),
        QuerySourceOwner::NamespaceCompleted {
            source: foreign.tree,
            completion: names,
        },
    )
    .unwrap_err();
    let QueryRunError::Execution(failure) = error else {
        panic!("owner mismatch diagnostic")
    };
    assert_eq!(failure.diagnostics.len(), 1);
    assert_eq!(
        failure.diagnostics[0].code,
        "cem.ql.query_input_unsupported"
    );
    assert!(failure.diagnostics[0]
        .message
        .contains("another source owner"));
}

#[test]
fn empty_selected_forest_produces_empty_input_without_falling_back_to_whole_document() {
    let input = import("{host @xmlns:v={#library} | {v:item}}");
    let names = Arc::new(
        NamespaceNameCompletion::new(input.captured.clone(), &[], BTreeMap::new()).unwrap(),
    );
    let response = run(&input, names, "(input, #input)");
    assert_eq!(items(&response).len(), 1);
    assert!(items(&response)[0]
        .view()
        .unwrap()
        .field("targets")
        .unwrap()
        .is_empty());
    let owner = response
        .result
        .input_ast_owner
        .as_any()
        .downcast_ref::<CemQlNativeItemsOwner>()
        .unwrap();
    assert!(owner.stream().items.is_empty());
}

#[test]
fn retained_owner_runner_keeps_cancellation_budgets_and_language_admission() {
    use cem_ml::{
        query::QueryExecutionLimits,
        schema::registry::{CSS_SELECTOR_CONTENT_TYPE, XPATH_CONTENT_TYPE},
    };
    let input = import("{host @xmlns:v={#library} | {v:item}}");
    let names = completed(&input, &[element(&input, "item")], "urn:ready");
    let owner = || QuerySourceOwner::NamespaceCompleted {
        source: input.tree.clone(),
        completion: names.clone(),
    };
    let mut limited = request("(input, input)");
    limited.limits = Some(QueryExecutionLimits {
        max_result_items: Some(1),
        ..Default::default()
    });
    let error = run_query_with_source_owner(limited, owner()).unwrap_err();
    assert!(format!("{error:?}").contains("cem.ql.query_result_limit_exceeded"));
    let cancelled = request("(");
    cancelled.context.abort_signal().abort();
    let error = run_query_with_source_owner(cancelled, owner()).unwrap_err();
    assert!(format!("{error:?}").contains("cem.query.cancelled"));
    for (media, code) in [
        (
            CSS_SELECTOR_CONTENT_TYPE,
            "cem.css_selector.input_unsupported",
        ),
        (XPATH_CONTENT_TYPE, "cem.xpath.query_input_unsupported"),
    ] {
        let mut other_language = request("*");
        other_language.query.identity = FormatIdentity {
            content_type: Some(media.into()),
            ..Default::default()
        };
        let QueryRunError::Execution(failure) =
            run_query_with_source_owner(other_language, owner()).unwrap_err()
        else {
            panic!("language admission")
        };
        assert!(
            failure
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{:?}",
            failure.diagnostics
        );
    }
}

/// The normalized producer exercises existing block-prelude alias semantics;
/// inline CEM body parsing does not claim to recognize document directives.
fn lifecycle_input() -> ScopedCemImport {
    use cem_ml::{
        events::{cem::CemEventNormalizer, EventNormalizer, NormalizedEvent},
        parser::tree::{CemTreeSemantics, RetainedCemTree},
        schema::machine::CemSchemaMachine,
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    struct Events(std::vec::IntoIter<NormalizedEvent>);
    impl EventNormalizer for Events {
        fn next_event(&mut self) -> Option<NormalizedEvent> {
            self.0.next()
        }
    }
    fn events(text: &str) -> Vec<NormalizedEvent> {
        let mut source = CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
            SourceId(1),
            text.as_bytes().to_vec(),
        )));
        std::iter::from_fn(|| source.next_event()).collect()
    }
    let text = "@ns v = urn:fixed\n@ns public = urn:one\n@ns other = urn:two\n@default \"\"\n{fixed @v:flag=yes}\n{host @xmlns:v={library} | {first | {#related}} {inner @xmlns:v={#other} | {second | {#related}}} {third | {#related}} {plain @xmlns={#reset} | {reset | {#related}}}} {outside}";
    let mut stream = vec![];
    for event in events(text) {
        if matches!(&event, NormalizedEvent::OpenScope {name, ..} if name.lexical_name == "first" || name.lexical_name == "second")
        {
            stream.extend(events("@default v\n"));
        }
        stream.push(event);
    }
    let captured = Arc::new(
        CemSchemaMachine::new(CompiledSchema::cem_core(), Events(stream.into_iter()))
            .build_with_lexical_scopes(),
    );
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        "memory:namespace-input.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    ScopedCemImport { captured, tree }
}

#[test]
fn prepared_properties_activate_saved_dependencies_and_shared_query_ingress() {
    use cem_ml::schema::{
        namespace_references::decode_native_namespace_property,
        reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        namespace_names::NamespaceQueryTree,
        schema_references::CemQlSchemaDeclarationHost,
    };
    let input = lifecycle_input();
    let properties = input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Attribute {
                node_id,
                value_nodes,
                ..
            } if !value_nodes.is_empty() => Some(*node_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(properties.len(), 3);
    let declarations = input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "@ns" => Some(*node_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    let handle = |id| SchemaDeclarationNode::new(input.tree.ast_owner().clone(), id).unwrap();
    let native = properties
        .iter()
        .map(|id| decode_native_namespace_property(handle(*id), &input.captured).unwrap())
        .collect::<Vec<_>>();
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let roots = [
        element(&input, "first"),
        element(&input, "second"),
        element(&input, "third"),
        element(&input, "reset"),
        element(&input, "fixed"),
    ];
    let mut identities = vec![];
    for (outer, inner, expected) in [
        (
            declarations[1],
            declarations[2],
            ["urn:one", "urn:two", "urn:one", "", ""],
        ),
        (
            declarations[2],
            declarations[1],
            ["urn:two", "urn:one", "urn:two", "", ""],
        ),
    ] {
        let mut base = StandaloneExpressionContext::default();
        for (name, id) in [
            ("library", outer),
            ("other", inner),
            ("reset", element(&input, "@default")),
        ] {
            base = base.with_binding(
                name,
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(input.tree.clone(), id)
                        .unwrap()
                        .query_item(),
                )),
            );
        }
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(input.tree.clone(), Some(base.clone()), policy.clone());
        host.attach_captured_namespaces(input.captured.clone())
            .unwrap();
        // The outer property's general expression runs with original fixed v.
        let selector_names = Arc::new(
            NamespaceNameCompletion::new(
                input.captured.clone(),
                &[native[0].value.node_id()],
                BTreeMap::new(),
            )
            .unwrap(),
        );
        assert_eq!(
            selector_names
                .lexical_snapshot(&native[0].value)
                .unwrap()
                .namespace_uri("v"),
            Some("urn:fixed")
        );
        host.attach_completed_namespace_lexical_scopes(&selector_names, |_, _, _| {
            (Some(base.clone()), ReferenceScopePolicyOverrides::default())
        })
        .unwrap();
        let outer_result = host
            .prepare_namespace_property(handle(properties[0]), policy.limits)
            .unwrap();
        assert!(outer_result.is_ready());
        // Child/reset selectors wait for the saved outer binding. Their roots
        // exclude governed children and never borrow a later child binding.
        let selector_activation = host
            .activate_namespace_properties(
                input.captured.clone(),
                &[native[1].value.node_id(), native[2].value.node_id()],
                &[outer_result.clone()],
                |source, snapshot, _, _| {
                    assert!(
                        source.node_id() == native[1].value.node_id()
                            || source.node_id() == native[2].value.node_id()
                    );
                    assert_eq!(snapshot.namespace_uri("v"), Some(expected[0]));
                    (Some(base.clone()), ReferenceScopePolicyOverrides::default())
                },
            )
            .unwrap();
        assert_eq!(selector_activation.scopes.len(), 2);
        let inner_result = host
            .prepare_namespace_property(handle(properties[1]), policy.limits)
            .unwrap();
        let reset_result = host
            .prepare_namespace_property(handle(properties[2]), policy.limits)
            .unwrap();
        assert!(inner_result.is_ready() && reset_result.is_ready());
        let mut seen = vec![];
        let activation = host
            .activate_namespace_properties(
                input.captured.clone(),
                &roots,
                &[outer_result, inner_result, reset_result],
                |source, snapshot, _, names| {
                    let index = seen.len();
                    assert_eq!(snapshot.namespace_uri(""), Some(expected[index]));
                    seen.push(source.node_id());
                    let view = NamespaceQueryTree::new(input.tree.clone(), names.clone()).unwrap();
                    (
                        Some(StandaloneExpressionContext::default().with_binding(
                            "related",
                            StandaloneExpressionBinding::any(ItemStream::once(
                                view.node(roots[index]).unwrap(),
                            )),
                        )),
                        ReferenceScopePolicyOverrides::default(),
                    )
                },
            )
            .unwrap();
        assert_eq!(activation.scopes.len(), 4);
        let response = run(
            &input,
            activation.completion.clone(),
            "(input.namespace, input.source.namespace, input)",
        );
        assert_eq!(&items(&response)[..5], strings(&expected));
        // Compare live execution views. Dropped views may reuse their addresses.
        identities.push(items(&response)[10].clone());
        let owner = response
            .result
            .input_ast_owner
            .as_any()
            .downcast_ref::<CemQlNativeItemsOwner>()
            .unwrap();
        let QuerySourceOwner::NamespaceCompleted { source, completion } = owner.source_owner()
        else {
            panic!("completed ingress");
        };
        assert!(Arc::ptr_eq(source, &input.tree));
        assert!(Arc::ptr_eq(completion, &activation.completion));
        assert_eq!(
            activation
                .completion
                .expanded_name(element(&input, "fixed"))
                .unwrap()
                .namespace_uri,
            ""
        );
        assert_eq!(
            activation
                .completion
                .expanded_name(
                    input
                        .tree
                        .ast()
                        .nodes
                        .iter()
                        .find_map(|node| match node {
                            CemAstNode::Attribute {
                                node_id,
                                expanded_name,
                                ..
                            } if expanded_name.local_name == "flag" => Some(*node_id),
                            _ => None,
                        })
                        .unwrap()
                )
                .unwrap()
                .namespace_uri,
            "urn:fixed"
        );
        for source_id in seen {
            use cem_ml::{
                schema::declaration_references::SchemaDeclarationHost,
                value::reference_resolution::resolve_reference,
            };
            let result = resolve_reference(
                host.source_reference(handle(source_id)),
                &mut host,
                policy.limits,
            )
            .unwrap();
            assert!(result.is_complete(), "{:?}", result.issues);
            let source = host.declaration_node(&result.nodes[0]).unwrap();
            assert!(Arc::ptr_eq(source.document(), input.tree.ast_owner()));
        }
        assert!(items(&response)[10].source_map().is_some());
    }
    assert_ne!(identities[0].identity(), identities[1].identity());
    assert!(properties.iter().all(|id| input
        .captured
        .namespace_binding(input.tree.ast_owner(), *id)
        .is_none()));
    assert!(input.tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}

#[test]
fn property_activation_retries_keep_readiness_crossings_and_budgets_before_query_ingress() {
    use cem_ml::{
        schema::reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
        value::reference_resolution::{ReferenceResolutionIssueKind, ReferenceResolutionState},
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::{
            CemQlSchemaDeclarationHost, NamespacePropertyActivationError,
            NamespaceScopePreparationIssue,
        },
    };
    let input =
        import("@ns v = urn:fixed\n{host @xmlns:v={#library} | {v:item | {#related}}} {outside}");
    let vendor = import("@ns public = urn:ready\n{#library}");
    let original =
        SchemaDeclarationNode::new(input.tree.ast_owner().clone(), declaration(&input)).unwrap();
    let target = element(&vendor, "@ns");
    let link = vendor.captured.occurrences().next().unwrap();
    let binding = |tree: &ScopedCemImport, id| {
        StandaloneExpressionContext::default().with_binding(
            "library",
            StandaloneExpressionBinding::any(ItemStream::once(
                RetainedCemNode::new(tree.tree.clone(), id)
                    .unwrap()
                    .query_item(),
            )),
        )
    };
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let mut strict = policy.clone();
    strict.limits.max_work = 1;
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(input.tree.clone(), None, policy.clone());
    let destination = host.register_scope(vendor.tree.clone(), None, strict);
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    host.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    let roots = [element(&input, "item")];
    let pending = host
        .prepare_namespace_property(original.clone(), policy.limits)
        .unwrap();
    assert_eq!(
        pending.preparation.as_ref().unwrap().selection.state,
        ReferenceResolutionState::Pending
    );
    assert!(matches!(
        host.activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[pending],
            |_, _, _, _| panic!("pending source")
        ),
        Err(NamespacePropertyActivationError::PropertyNotReady(_))
    ));
    host.set_context(origin, Some(binding(&vendor, target)));
    let denied = host
        .prepare_namespace_property(original.clone(), policy.limits)
        .unwrap();
    let scope_issue = denied
        .preparation
        .as_ref()
        .unwrap()
        .selection
        .issues
        .iter()
        .find(|issue| issue.kind == ReferenceResolutionIssueKind::ScopeDenied)
        .unwrap();
    assert!(!scope_issue.occurrence.source_map.frames.is_empty());
    assert!(matches!(
        host.activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[denied],
            |_, _, _, _| panic!("denied crossing")
        ),
        Err(NamespacePropertyActivationError::PropertyNotReady(_))
    ));
    assert!(host.allow_scope_crossing(origin, destination));
    let pending_target = host
        .prepare_namespace_property(original.clone(), policy.limits)
        .unwrap();
    assert_eq!(
        pending_target.preparation.as_ref().unwrap().issue,
        Some(NamespaceScopePreparationIssue::TargetContextNotReady)
    );
    assert!(matches!(
        host.activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[pending_target],
            |_, _, _, _| panic!("pending target")
        ),
        Err(NamespacePropertyActivationError::PropertyNotReady(_))
    ));
    host.set_context(destination, Some(binding(&vendor, target)));
    let mut limited = policy.limits;
    limited.max_work = 1;
    let request_bounded = host
        .prepare_namespace_property(original.clone(), limited)
        .unwrap();
    assert!(request_bounded
        .preparation
        .as_ref()
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit));
    assert!(matches!(
        host.activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[request_bounded],
            |_, _, _, _| panic!("request work cap")
        ),
        Err(NamespacePropertyActivationError::PropertyNotReady(_))
    ));
    host.set_context(origin, Some(binding(&vendor, link)));
    let destination_bounded = host
        .prepare_namespace_property(original.clone(), policy.limits)
        .unwrap();
    assert!(destination_bounded
        .preparation
        .as_ref()
        .unwrap()
        .selection
        .issues
        .iter()
        .any(
            |issue| issue.kind == ReferenceResolutionIssueKind::WorkLimit
                && issue.reason == "scope-work-limit"
        ));
    assert!(matches!(
        host.activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[destination_bounded],
            |_, _, _, _| panic!("destination cap")
        ),
        Err(NamespacePropertyActivationError::PropertyNotReady(_))
    ));
    let relaxed = host
        .register_lexical_scope(destination, Some(binding(&vendor, target)), policy.clone())
        .unwrap();
    assert!(host.assign_subtree_scope(&vendor.tree, link, relaxed));
    assert!(host.assign_subtree_scope(&vendor.tree, target, relaxed));
    limited = policy.limits;
    limited.max_depth = 1;
    let depth_bounded = host
        .prepare_namespace_property(original.clone(), limited)
        .unwrap();
    assert!(depth_bounded
        .preparation
        .as_ref()
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|issue| issue.kind == ReferenceResolutionIssueKind::DepthLimit));
    assert!(matches!(
        host.activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[depth_bounded],
            |_, _, _, _| panic!("depth cap")
        ),
        Err(NamespacePropertyActivationError::PropertyNotReady(_))
    ));
    let ready = host
        .prepare_namespace_property(original, policy.limits)
        .unwrap();
    assert!(ready.is_ready());
    assert_eq!(ready.preparation.as_ref().unwrap().selection.work_used, 3);
    // Activation consumes exactly this result. It does not traverse the chain
    // again; current source inputs can be unavailable after explicit preparation.
    host.set_context(origin, None);
    let activation = host
        .activate_namespace_properties(
            input.captured.clone(),
            &roots,
            &[ready],
            |_, snapshot, _, _| {
                assert_eq!(snapshot.namespace_uri("v"), Some("urn:ready"));
                (None, ReferenceScopePolicyOverrides::default())
            },
        )
        .unwrap();
    assert_eq!(activation.scopes.len(), 1);
    let response = run(
        &input,
        activation.completion,
        "(input.namespace, input.source.namespace)",
    );
    assert_eq!(items(&response), strings(&["urn:ready", "v"]));
    assert!(input
        .captured
        .namespace_binding(input.tree.ast_owner(), declaration(&input))
        .is_none());
}

#[test]
fn coordinated_ingress_retains_independent_names_and_authored_reference_children() {
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{ItemStream, RetainedCemNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let input = import("@ns public = urn:first\n@ns public = urn:second\n{host @xmlns:v={#library} | {v:item @v:flag=yes | {#related}}}");
    let declarations: Vec<_> = input
        .tree
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "@ns" => Some(*node_id),
            _ => None,
        })
        .collect();
    let mut host = CemQlSchemaDeclarationHost::new();
    let policy = cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults().unwrap();
    host.register_scope(
        input.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy.clone(),
    );
    let mut retained = vec![];
    for (target, uri) in [
        (declarations[0], "urn:first"),
        (declarations[1], "urn:second"),
    ] {
        let context = StandaloneExpressionContext::default()
            .with_binding(
                "library",
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(input.tree.clone(), target)
                        .unwrap()
                        .query_item(),
                )),
            )
            .with_binding(
                "related",
                StandaloneExpressionBinding::any(ItemStream::empty()),
            );
        let (snapshot, response) = host.with_namespace_lifecycle(input.captured.clone(), &[element(&input, "item")], policy.limits,
            |_, _, _, _| (Some(context.clone()), Default::default()),
            |_, snapshot| {
                assert!(snapshot.is_complete());
                run(&input, snapshot.completion.clone(), "(input.namespace, input.attributes.namespace, input.children.kind, input.source.namespace)")
            }).unwrap();
        assert_eq!(
            items(&response),
            strings(&[uri, uri, "whitespace", "reference", "v"])
        );
        retained.push((snapshot, response));
    }
    assert_eq!(items(&retained[0].1)[0], strings(&["urn:first"])[0]);
    assert_eq!(items(&retained[1].1)[0], strings(&["urn:second"])[0]);
    assert!(input
        .captured
        .expanded_name(input.tree.ast_owner(), element(&input, "item"))
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
fn coordinated_pending_ingress_exposes_only_independent_ready_regions() {
    use cem_ql::{api::StandaloneExpressionContext, schema_references::CemQlSchemaDeclarationHost};
    let input = import("@ns public = urn:ready\n{host @xmlns:v={#library} | {v:item}} {outside}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let policy = cem_ml::schema::reference_policy::ReferenceScopePolicy::schema_defaults().unwrap();
    host.register_scope(
        input.tree.clone(),
        Some(StandaloneExpressionContext::default()),
        policy.clone(),
    );
    let (snapshot, response) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &[element(&input, "item"), element(&input, "outside")],
            policy.limits,
            |_, _, _, _| (None, Default::default()),
            |_, snapshot| run(&input, snapshot.completion.clone(), "input.name"),
        )
        .unwrap();
    assert!(!snapshot.is_complete());
    assert_eq!(items(&response), strings(&["outside"]));
    assert!(!snapshot.completion.contains(element(&input, "item")));
}
