use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{
        builder::CemAstBuilder,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::{
        reference_policy::ReferenceScopePolicy, reference_traversal::ReferenceTraversalLimits,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::ReferenceResolutionState,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{Item, ItemStream},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::sync::Arc;
fn tree(text: &str) -> Arc<RetainedCemTree> {
    let ast = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    assert!(ast.diagnostics.is_empty(), "{:?}", ast.diagnostics);
    RetainedCemTree::new(ast, "schema.cem", text, CemTreeSemantics::default(), None).unwrap()
}
fn declarations(tree: &Arc<RetainedCemTree>) -> Vec<Item> {
    tree.ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                expanded_name,
                children,
                ..
            } if expanded_name.local_name == "elements" => Some(children),
            _ => None,
        })
        .flatten()
        .filter(|id| {
            matches!(
                tree.ast().get(**id),
                Some(CemAstNode::Element { .. } | CemAstNode::Reference { .. })
            )
        })
        .map(|id| {
            cem_ql::eval::RetainedCemNode::new(tree.clone(), *id)
                .unwrap()
                .query_item()
        })
        .collect()
}
fn context(items: Vec<Item>) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::any(ItemStream::from_items(items)),
    )
}
fn policy() -> ReferenceScopePolicy {
    ReferenceScopePolicy::schema_defaults().unwrap()
}
fn limits() -> ReferenceTraversalLimits {
    policy().limits
}
#[test]
fn retained_declarations_are_compiled_without_copying_the_source_owner() {
    let source = tree("{schema | {elements | {#library}} }");
    let library = tree(
        r#"{schema | {uses | {use @schema="https://cem.dev/ns/schema/1" @as="origin"}} {elements | {element @name="shared" @base="origin:element"}} }"#,
    );
    let mut host = CemQlSchemaDeclarationHost::new();
    let requesting = host.register_scope(
        source.clone(),
        Some(context(declarations(&library))),
        policy(),
    );
    let destination = host.register_scope(library.clone(), None, policy());
    let denied = host
        .compile("schema:test", source.clone(), limits())
        .unwrap();
    assert!(!denied.is_ready_for_validation());
    assert!(denied.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .issues
        .iter()
        .any(|i| i.kind
            == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::ScopeDenied));
    host.allow_scope_crossing(requesting, destination);
    let model = host
        .compile("schema:test", source.clone(), limits())
        .unwrap();
    assert!(
        model.is_ready_for_validation(),
        "{:?}",
        model.compile_diagnostics
    );
    assert!(model.elements["shared"]
        .optional_attributes
        .contains("base"));
    let target = &model.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .nodes[0];
    assert!(Arc::ptr_eq(target.document(), library.ast_owner()));
    assert!(std::ptr::eq(
        target.node(),
        library.ast().get(target.node_id()).unwrap()
    ));
    drop(host);
    drop(library);
    assert!(matches!(target.node(), CemAstNode::Element { .. }));
    assert!(source
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn runtime_context_controls_pending_empty_invalid_and_independent_results() {
    let source = tree("{schema | {elements | {#library}} }");
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(source.clone(), None, policy());
    assert_eq!(
        host.compile("schema:test", source.clone(), limits())
            .unwrap()
            .declaration_references
            .state(),
        ReferenceResolutionState::Pending
    );
    host.set_context(scope, Some(context(vec![])));
    let empty = host
        .compile("schema:test", source.clone(), limits())
        .unwrap();
    assert!(empty.is_ready_for_validation() && empty.elements.is_empty());
    for name in ["first", "second"] {
        let library = tree(&format!(
            "{{schema | {{elements | {{element @name=\"{name}\"}} }} }}"
        ));
        let dest = host.register_scope(library.clone(), None, policy());
        host.allow_scope_crossing(scope, dest);
        host.set_context(scope, Some(context(declarations(&library))));
        let model = host
            .compile("schema:test", source.clone(), limits())
            .unwrap();
        assert!(model.elements.contains_key(name));
        assert_eq!(model.elements.len(), 1);
    }
    host.set_context(
        scope,
        Some(context(vec![Item::Atomic(
            cem_ql::eval::AtomValue::Integer(7.into()),
        )])),
    );
    let invalid = host
        .compile("schema:test", source.clone(), limits())
        .unwrap();
    assert_eq!(
        invalid.declaration_references.state(),
        ReferenceResolutionState::Invalid
    );
    assert!(invalid
        .compile_diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation()));
}
#[test]
fn nested_source_references_use_destination_context_and_limits() {
    let source = tree("{schema | {elements | {#library}} }");
    let middle = tree("{schema | {elements | {#library}} }");
    let final_tree = tree("{schema | {elements | {element @name=\"terminal\"}} }");
    let mut host = CemQlSchemaDeclarationHost::new();
    let a = host.register_scope(
        source.clone(),
        Some(context(declarations(&middle))),
        policy(),
    );
    let mut bounded = policy();
    bounded.limits.max_work = 1;
    let b = host.register_scope(
        middle.clone(),
        Some(context(declarations(&final_tree))),
        bounded,
    );
    let c = host.register_scope(final_tree, None, policy());
    host.allow_scope_crossing(a, b);
    host.allow_scope_crossing(b, c);
    let result = host
        .compile("schema:test", source.clone(), limits())
        .unwrap();
    assert!(!result.is_ready_for_validation());
    assert!(result.declaration_references.sites[0]
        .resolution
        .as_ref()
        .unwrap()
        .issues
        .iter()
        .any(|i| i.kind
            == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::WorkLimit));
}

#[test]
fn child_scope_contexts_and_host_specific_scope_handles_are_explicit() {
    let source = tree("{schema | {elements | {##library}} }");
    let library = tree("{schema | {elements | {element @name=\"shared\"}} }");
    let mut host = CemQlSchemaDeclarationHost::new();
    let parent = host.register_scope(source.clone(), Some(context(vec![])), policy());
    let child = host.register_scope(
        source.clone(),
        Some(context(declarations(&library))),
        policy(),
    );
    let destination = host.register_scope(library.clone(), None, policy());
    let reference = source
        .ast()
        .nodes
        .iter()
        .find_map(|n| {
            if let CemAstNode::Reference { node_id, .. } = n {
                Some(*node_id)
            } else {
                None
            }
        })
        .unwrap();
    assert!(host.assign_subtree_scope(&source, reference, child));
    assert!(host.allow_scope_crossing(child, destination));
    let model = host
        .compile("schema:test", source.clone(), limits())
        .unwrap();
    assert!(model.is_ready_for_validation() && model.elements.contains_key("shared"));
    assert_eq!(
        model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .work_used,
        3,
        "outer source, nested constructor and terminal declaration"
    );
    let mut unrelated_host = CemQlSchemaDeclarationHost::new();
    let foreign = unrelated_host.register_scope(source.clone(), None, policy());
    assert_ne!(parent, foreign);
    assert!(!host.allow_scope_crossing(foreign, destination));
    assert!(!host.assign_subtree_scope(&source, reference, foreign));
    assert!(!host.set_context(foreign, None));
    let unregistered = tree("{schema | {elements | {element @name=\"unregistered\"}} }");
    assert!(!host.assign_subtree_scope(&unregistered, 0, child));
}

#[test]
fn malformed_native_reference_source_cannot_be_treated_as_a_resolved_selection() {
    let text = "{schema | {elements | {#library}} }";
    let mut ast = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
    )))
    .build();
    // A native producer can supply a reference node without its outer # form.
    // Construct this malformed input before retaining it; do not copy an arena.
    for n in &mut ast.nodes {
        if let CemAstNode::Reference { expression, .. } = n {
            *expression = "library".into();
        }
    }
    let source = RetainedCemTree::new(
        ast,
        "malformed.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    let library = tree("{schema | {elements | {element @name=\"valid\"}} }");
    let mut host = CemQlSchemaDeclarationHost::new();
    let a = host.register_scope(
        source.clone(),
        Some(context(declarations(&library))),
        policy(),
    );
    let b = host.register_scope(library, None, policy());
    host.allow_scope_crossing(a, b);
    let model = host.compile("schema:test", source, limits()).unwrap();
    assert_eq!(
        model.declaration_references.state(),
        ReferenceResolutionState::Invalid
    );
    assert!(model.compile_diagnostics.iter().any(|d| d.code
        == cem_ml::schema::declaration_references::INVALID_REFERENCE_TARGET
        && d.severity.is_hard_violation()));
}

#[test]
fn native_attribute_chains_keep_target_owners_and_obey_destination_limits() {
    let source = tree("{schema | {attributes | {#library}} }");
    let middle = tree("{schema | {attributes | {#library}} }");
    let terminal = tree(
        "{schema | {attributes | {attribute @name=size @type=schema:integer @minInclusive=1}} }",
    );
    let items = |owner: &Arc<RetainedCemTree>| {
        owner
            .ast()
            .nodes
            .iter()
            .filter_map(|node| match node {
                CemAstNode::Reference { node_id, .. } => Some(*node_id),
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "attribute" => Some(*node_id),
                _ => None,
            })
            .map(|id| {
                cem_ql::eval::RetainedCemNode::new(owner.clone(), id)
                    .unwrap()
                    .query_item()
            })
            .collect()
    };
    for bounded in [false, true] {
        let mut host = CemQlSchemaDeclarationHost::new();
        let a = host.register_scope(source.clone(), Some(context(items(&middle))), policy());
        let mut destination = policy();
        if bounded {
            destination.limits.max_work = 1;
        }
        let b = host.register_scope(middle.clone(), Some(context(items(&terminal))), destination);
        let c = host.register_scope(terminal.clone(), None, policy());
        assert!(!host
            .compile("consumer", source.clone(), limits())
            .unwrap()
            .is_ready_for_validation());
        assert!(host.allow_scope_crossing(a, b) && host.allow_scope_crossing(b, c));
        let model = host.compile("consumer", source.clone(), limits()).unwrap();
        if bounded {
            assert!(!model.is_ready_for_validation());
            assert!(model.declaration_references.sites[0].resolution.as_ref().unwrap().issues.iter()
                .any(|i| i.kind == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::WorkLimit));
        } else {
            assert!(model.is_ready_for_validation());
            assert_eq!(model.attributes["size"].min_inclusive.as_deref(), Some("1"));
            let retained = &model.declaration_references.sites[0]
                .resolution
                .as_ref()
                .unwrap()
                .nodes[0];
            assert!(Arc::ptr_eq(retained.document(), terminal.ast_owner()));
            drop(host);
            assert!(matches!(retained.node(), CemAstNode::Element { .. }));
        }
    }
    assert!(source
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

#[test]
fn diagnostic_and_behavior_reuse_requires_grants_and_binds_assembled_dependencies() {
    use cem_ql::eval::RetainedCemNode;
    let source=tree("{schema | {diagnostics | {#diagnostics}} {attributes | {attribute @name=size @type=schema:integer @type-diagnostic=fixture.value}} {behaviors | {#behaviors}} }");
    let behavior=tree("{schema @namespace=library | {behaviors | {behavior @name=value-check @implementation=engine @execution=ast-validation @primitive=schema:scalar-type}} }");
    let diagnostic=tree("{schema | {diagnostics | {diagnostic @code=fixture.value @behavior=value-check @severity=warning}} }");
    let item = |owner: &Arc<RetainedCemTree>, kind: &str| {
        let id = owner
            .ast()
            .nodes
            .iter()
            .find_map(|n| match n {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == kind => Some(*node_id),
                _ => None,
            })
            .unwrap();
        RetainedCemNode::new(owner.clone(), id)
            .unwrap()
            .query_item()
    };
    let context = StandaloneExpressionContext::default()
        .with_binding(
            "diagnostics",
            StandaloneExpressionBinding::any(ItemStream::once(item(&diagnostic, "diagnostic"))),
        )
        .with_binding(
            "behaviors",
            StandaloneExpressionBinding::any(ItemStream::once(item(&behavior, "behavior"))),
        );
    let mut host = CemQlSchemaDeclarationHost::new();
    let a = host.register_scope(source.clone(), Some(context), policy());
    let b = host.register_scope(behavior.clone(), None, policy());
    let c = host.register_scope(diagnostic.clone(), None, policy());
    host.allow_scope_crossing(a, c);
    let incomplete = host.compile("consumer", source.clone(), limits()).unwrap();
    assert!(!incomplete.is_ready_for_validation());
    assert!(!incomplete
        .compile_diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::UNKNOWN_DIAGNOSTIC_BEHAVIOR_CODE));
    host.allow_scope_crossing(a, b);
    let model = host.compile("consumer", source.clone(), limits()).unwrap();
    assert!(model.is_ready_for_validation() && model.compile_diagnostics.is_empty());
    assert_eq!(
        model.diagnostic_behaviors["fixture.value"].severity,
        cem_ml::diagnostics::Severity::Warning
    );
    assert!(Arc::ptr_eq(
        model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .nodes[0]
            .document(),
        diagnostic.ast_owner()
    ));
    assert!(Arc::ptr_eq(
        model.declaration_references.sites[1]
            .resolution
            .as_ref()
            .unwrap()
            .nodes[0]
            .document(),
        behavior.ast_owner()
    ));
}

#[test]
fn constraint_reuse_honors_scope_grants_and_preserves_policy_budget() {
    let source = tree("{schema | {constraints | {#library}}}");
    let library =
        tree("{schema | {constraints | {constraint @kind=reference-traversal-work @value=1}}}");
    let id = library
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "constraint" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let item = cem_ql::eval::RetainedCemNode::new(library.clone(), id)
        .unwrap()
        .query_item();
    let mut host = CemQlSchemaDeclarationHost::new();
    let a = host.register_scope(source.clone(), Some(context(vec![item])), policy());
    let b = host.register_scope(library.clone(), None, policy());
    assert!(!host
        .compile("consumer", source.clone(), limits())
        .unwrap()
        .is_ready_for_validation());
    host.allow_scope_crossing(a, b);
    let model = host.compile("consumer", source, limits()).unwrap();
    assert!(model.is_ready_for_validation());
    assert_eq!(
        model.constraints["reference-traversal-work"]
            .value
            .as_deref(),
        Some("1")
    );
    assert!(Arc::ptr_eq(
        model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .nodes[0]
            .document(),
        library.ast_owner()
    ));
    assert_eq!(policy().for_scope(&model).unwrap().limits.max_work, 1);
}

#[test]
fn field_contract_reuse_checks_scope_grants_and_local_targets_after_assembly() {
    let source = tree("{schema | {elements | {element @name=box}} {field-contracts | {#library}}}");
    let library = tree("{schema | {field-contracts | {field-contract @name=shared @target=box @required-attributes=command}}}");
    let id = library
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "field-contract" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let item = cem_ql::eval::RetainedCemNode::new(library.clone(), id)
        .unwrap()
        .query_item();
    let mut host = CemQlSchemaDeclarationHost::new();
    let a = host.register_scope(source.clone(), Some(context(vec![item])), policy());
    let b = host.register_scope(library.clone(), None, policy());
    assert!(!host
        .compile("consumer", source.clone(), limits())
        .unwrap()
        .is_ready_for_validation());
    host.allow_scope_crossing(a, b);
    let model = host.compile("consumer", source, limits()).unwrap();
    assert!(model.is_ready_for_validation() && model.compile_diagnostics.is_empty());
    assert_eq!(model.elements["box"].field_contracts.len(), 1);
    assert!(Arc::ptr_eq(
        model.declaration_references.sites[0]
            .resolution
            .as_ref()
            .unwrap()
            .nodes[0]
            .document(),
        library.ast_owner()
    ));
}

#[test]
fn structural_input_validation_uses_explicit_context_grants_and_consuming_schema() {
    let source = tree("{box | {#library}}");
    let library = tree("{item}");
    let id = library
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "item" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let item = cem_ql::eval::RetainedCemNode::new(library.clone(), id)
        .unwrap()
        .query_item();
    let model = cem_ml::schema::document_model::compile_schema_document_model("consumer", "{schema | {elements | {element @name=box @children=item} {element @name=item @required-attributes=command}}}");
    let mut host = CemQlSchemaDeclarationHost::new();
    let a = host.register_scope(source.clone(), None, policy());
    let b = host.register_scope(library.clone(), None, policy());
    assert!(
        !host
            .validate_input(source.clone(), &model, limits())
            .unwrap()
            .complete
    );
    host.set_context(a, Some(context(vec![item])));
    assert!(
        !host
            .validate_input(source.clone(), &model, limits())
            .unwrap()
            .complete
    );
    host.allow_scope_crossing(a, b);
    let report = host
        .validate_input(source.clone(), &model, limits())
        .unwrap();
    assert!(report.complete && report.failed);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == cem_ml::schema::document_model::MISSING_REQUIRED_ATTRIBUTE_CODE));
    let selected = report.nodes.iter().find(|n| matches!(n.source.node(), CemAstNode::Element {expanded_name, ..} if expanded_name.local_name == "item")).unwrap();
    assert!(Arc::ptr_eq(selected.source.document(), library.ast_owner()));
    let mut independent = CemQlSchemaDeclarationHost::new();
    independent.register_scope(source.clone(), Some(context(vec![])), policy());
    let report = independent
        .validate_input(source.clone(), &model, limits())
        .unwrap();
    assert!(report.complete && !report.failed);
    assert!(source
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

#[test]
fn native_retained_behavior_handoff_obeys_grants_and_preserves_original_context() {
    use cem_ml::{
        diagnostics::Diagnostic,
        parser::document::CemDocument,
        schema::{
            document_model::{
                compile_schema_document_model, SchemaBehaviorEvaluator, SchemaDocumentModel,
            },
            input_references::{RetainedBehaviorValidation, RetainedValidationStructure},
        },
    };
    #[derive(Debug)]
    struct Behavior {
        owner: Arc<RetainedCemTree>,
        calls: std::sync::atomic::AtomicUsize,
    }
    impl SchemaBehaviorEvaluator for Behavior {
        fn validate_document(&self, _: &CemDocument, _: &SchemaDocumentModel) -> Vec<Diagnostic> {
            panic!("retained behavior must not use a synthetic document")
        }
        fn validate_retained_structure(
            &self,
            structure: RetainedValidationStructure<'_>,
            _: &SchemaDocumentModel,
        ) -> RetainedBehaviorValidation {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let parent = &structure.nodes[structure.roots[0]];
            let placements: Vec<_> = parent.children.iter().copied().filter(|id| matches!(structure.nodes[*id].source.node(), CemAstNode::Element {expanded_name,..} if expanded_name.local_name == "item")).collect();
            let child = &structure.nodes[placements[0]];
            assert!(Arc::ptr_eq(child.source.document(), self.owner.ast_owner()));
            assert!(Arc::ptr_eq(
                child.declaring_schema.as_ref().unwrap().document(),
                self.owner.ast_owner()
            ));
            assert_eq!(placements.len(), 1);
            RetainedBehaviorValidation {
                complete: true,
                diagnostics: vec![],
            }
        }
    }
    let source = tree("{box | {#library}}");
    let library = tree("{schema | {item}}");
    let item = library
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "item" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let target = cem_ql::eval::RetainedCemNode::new(library.clone(), item)
        .unwrap()
        .query_item();
    let mut host = CemQlSchemaDeclarationHost::new();
    let from = host.register_scope(source.clone(), Some(context(vec![target])), policy());
    let to = host.register_scope(library.clone(), None, policy());
    let evaluator = Behavior {
        owner: library,
        calls: Default::default(),
    };
    let model = compile_schema_document_model(
        "consumer",
        "{schema | {elements | {element @name=box @children=item} {element @name=item}}}",
    );
    let denied = host
        .validate_input_with_behavior_evaluator(source.clone(), &model, limits(), Some(&evaluator))
        .unwrap();
    assert!(!denied.complete);
    assert_eq!(evaluator.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(host.allow_scope_crossing(from, to));
    let report = host
        .validate_input_with_behavior_evaluator(source.clone(), &model, limits(), Some(&evaluator))
        .unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(evaluator.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(Arc::ptr_eq(&report.source, source.ast_owner()));
    assert!(source
        .ast()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}

#[test]
fn native_base_compilation_uses_lexical_context_and_one_granted_traversal() {
    let leaf = tree("{schema | {elements | {element @name=leaf @required-attributes=deep}}}");
    let middle = tree("{schema | {elements | {element @name=middle @base={#library}}}}");
    for collection in [false, true] {
        let source = tree(if collection {
            "{schema | {elements | {#library}}}"
        } else {
            "{schema | {elements | {element @name=child @base={#library}}}}"
        });
        let mut host = CemQlSchemaDeclarationHost::new();
        let requesting = host.register_scope(
            source.clone(),
            Some(context(declarations(&middle))),
            policy(),
        );
        let destination =
            host.register_scope(middle.clone(), Some(context(declarations(&leaf))), policy());
        let final_scope = host.register_scope(leaf.clone(), None, policy());
        assert!(!host
            .compile("consumer", source.clone(), limits())
            .unwrap()
            .is_ready_for_validation());
        host.allow_scope_crossing(requesting, destination);
        assert!(!host
            .compile("consumer", source.clone(), limits())
            .unwrap()
            .is_ready_for_validation());
        host.allow_scope_crossing(destination, final_scope);
        let incomplete = host
            .compile(
                "consumer",
                source.clone(),
                ReferenceTraversalLimits {
                    max_depth: 1,
                    max_work: 100,
                },
            )
            .unwrap();
        assert!(!incomplete.is_ready_for_validation());
        let model = host.compile("consumer", source, limits()).unwrap();
        assert!(
            model.is_ready_for_validation(),
            "{:?}",
            model.compile_diagnostics
        );
        assert!(model.elements[if collection { "middle" } else { "child" }]
            .required_attributes
            .contains("deep"));
        drop(host);
        assert!(model
            .declaration_references
            .sites
            .iter()
            .flat_map(|site| site.resolution.as_ref().unwrap().nodes.iter())
            .any(|node| Arc::ptr_eq(node.document(), leaf.ast_owner())));
    }
}

#[test]
fn reused_attribute_types_stay_pending_without_an_executable_datatype_consumer() {
    let source = tree("{schema | {attributes | {#library}}}");
    let library = tree("{schema | {attributes | {attribute @name=target @type={#datatype}}}}");
    let id = library
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "attribute" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    let requesting = host.register_scope(
        source.clone(),
        Some(context(vec![cem_ql::eval::RetainedCemNode::new(
            library.clone(),
            id,
        )
        .unwrap()
        .query_item()])),
        policy(),
    );
    let destination = host.register_scope(library.clone(), None, policy());
    host.allow_scope_crossing(requesting, destination);
    let model = host.compile("consumer", source, limits()).unwrap();
    assert!(!model.is_ready_for_validation());
    assert!(!model.declaration_references.failed());
    let pending = model
        .declaration_references
        .sites
        .iter()
        .find(|site| site.occurrence.expression.as_deref() == Some("#datatype"))
        .unwrap();
    assert!(pending.resolution.is_none());
    assert!(model.attributes.contains_key("target"));
    drop(host);
    assert!(model
        .declaration_references
        .sites
        .iter()
        .flat_map(|site| site.resolution.iter())
        .flat_map(|resolution| &resolution.nodes)
        .any(|target| Arc::ptr_eq(target.document(), library.ast_owner())));
}

fn reference_source(
    tree: &Arc<RetainedCemTree>,
) -> cem_ml::schema::declaration_references::SchemaDeclarationNode {
    tree.ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Reference { node_id, .. } => {
                cem_ml::schema::declaration_references::SchemaDeclarationNode::new(
                    tree.ast_owner().clone(),
                    *node_id,
                )
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn source_expression_artifacts_are_retained_until_context_replacement() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    let source =
        tree("{schema | {elements | {#library}} {element @name=first} {element @name=second}}");
    let targets: Vec<_> = source
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "element" => Some(
                cem_ql::eval::RetainedCemNode::new(source.clone(), *node_id)
                    .unwrap()
                    .query_item(),
            ),
            _ => None,
        })
        .collect();
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(source.clone(), None, policy());
    let original = reference_source(&source);
    let node = host.source_reference(original.clone());
    assert!(host.compiled_source_expression(&original).is_none());
    assert!(matches!(
        host.evaluate(&node),
        ReferenceLinkEvaluation::Pending(_)
    ));
    assert!(host.compiled_source_expression(&original).is_none());
    host.set_context(scope, Some(context(vec![targets[0].clone()])));
    let ReferenceLinkEvaluation::Resolved(first) = host.evaluate(&node) else {
        panic!()
    };
    let artifact = host.compiled_source_expression(&original).unwrap();
    let ReferenceLinkEvaluation::Resolved(repeated) = host.evaluate(&node) else {
        panic!()
    };
    assert!(Arc::ptr_eq(
        &artifact,
        &host.compiled_source_expression(&original).unwrap()
    ));
    assert_eq!(
        host.declaration_node(&first[0]).unwrap().identity(),
        host.declaration_node(&repeated[0]).unwrap().identity()
    );
    host.set_context(scope, Some(context(vec![targets[1].clone()])));
    assert!(host.compiled_source_expression(&original).is_none());
    let ReferenceLinkEvaluation::Resolved(second) = host.evaluate(&node) else {
        panic!()
    };
    assert_ne!(
        host.declaration_node(&first[0]).unwrap().identity(),
        host.declaration_node(&second[0]).unwrap().identity()
    );
    assert!(!Arc::ptr_eq(
        &artifact,
        &host.compiled_source_expression(&original).unwrap()
    ));
    host.set_context(scope, None);
    assert!(host.compiled_source_expression(&original).is_none());
    assert!(matches!(
        host.evaluate(&node),
        ReferenceLinkEvaluation::Pending(_)
    ));
    assert!(matches!(
        original.node(),
        CemAstNode::Reference { targets: None, .. }
    ));
    // The replacement's static type forbids node construction even when its
    // supplied sequence is empty. Reusing the old Any compilation would miss it.
    let scalar_context = StandaloneExpressionContext::default().with_binding(
        "library",
        StandaloneExpressionBinding::new(
            ItemStream::empty(),
            cem_ql::types::Type::atom(cem_ql::types::AtomType::String),
        ),
    );
    host.set_context(scope, Some(scalar_context));
    let ReferenceLinkEvaluation::Invalid(diagnostics) = host.evaluate(&node) else {
        panic!()
    };
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.node.as_deref() == Some(original.identity().as_str())));
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.uri.as_deref() == Some("schema.cem")));
    assert!(host.compiled_source_expression(&original).is_none());
}

#[test]
fn general_attribute_slots_retain_artifacts_and_reject_removed_bindings() {
    use cem_ml::{
        schema::declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        value::reference_resolution::ReferenceLinkEvaluation,
    };
    let source = tree("{item @target={library}} {target}");
    let expression = source
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "$" => {
                SchemaDeclarationNode::new(source.ast_owner().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    let item = cem_ql::eval::RetainedCemNode::new(source.clone(), expression.node_id())
        .unwrap()
        .query_item();
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(source, Some(context(vec![item])), policy());
    let node = host.source_reference(expression.clone());
    assert!(matches!(
        host.evaluate_input_expression(&node),
        ReferenceLinkEvaluation::Resolved(_)
    ));
    let artifact = host.compiled_source_expression(&expression).unwrap();
    assert!(matches!(
        host.evaluate_input_expression(&node),
        ReferenceLinkEvaluation::Resolved(_)
    ));
    assert!(Arc::ptr_eq(
        &artifact,
        &host.compiled_source_expression(&expression).unwrap()
    ));
    host.set_context(scope, Some(StandaloneExpressionContext::default()));
    assert!(matches!(
        host.evaluate_input_expression(&node),
        ReferenceLinkEvaluation::Invalid(_)
    ));
    assert!(host.compiled_source_expression(&expression).is_none());
}

#[test]
fn identical_reference_sources_in_child_scopes_keep_distinct_artifacts() {
    use cem_ml::{
        schema::declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        value::reference_resolution::ReferenceResolutionHost,
    };
    let source = tree("{left | {#library}} {right | {#library}}");
    let refs: Vec<_> = source
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Reference { node_id, .. } => {
                SchemaDeclarationNode::new(source.ast_owner().clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(source.clone(), Some(context(vec![])), policy());
    let child = host.register_scope(source.clone(), Some(context(vec![])), policy());
    assert!(host.assign_subtree_scope(&source, refs[1].node_id(), child));
    for original in &refs {
        let node = host.source_reference(original.clone());
        host.evaluate(&node);
    }
    let first = host.compiled_source_expression(&refs[0]).unwrap();
    let second = host.compiled_source_expression(&refs[1]).unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    host.set_context(child, None);
    assert!(Arc::ptr_eq(
        &first,
        &host.compiled_source_expression(&refs[0]).unwrap()
    ));
    assert!(host.compiled_source_expression(&refs[1]).is_none());
}

#[test]
fn xml_cdata_and_curly_reference_slots_compile_with_equivalent_contexts() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    let curly = tree("{section | {#library} {after}}");
    let xml = cem_ml::import::import_data_bytes(b"<section xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr><![CDATA[#library]]></r:expr><after/></section>", "application/xml", "cem", "source.xml").unwrap();
    for source in [curly, xml] {
        let original = reference_source(&source);
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(source, Some(context(vec![])), policy());
        let node = host.source_reference(original.clone());
        assert!(
            matches!(host.evaluate(&node), ReferenceLinkEvaluation::Resolved(targets) if targets.is_empty())
        );
        assert!(host.compiled_source_expression(&original).is_some());
        assert!(
            matches!(original.node(), CemAstNode::Reference { expression, targets: None, source, .. } if expression == "#library" && !source.frames.is_empty())
        );
    }
    let malformed = tree("{#library[}");
    let original = reference_source(&malformed);
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(malformed, Some(context(vec![])), policy());
    let node = host.source_reference(original.clone());
    assert!(matches!(
        host.evaluate(&node),
        ReferenceLinkEvaluation::Invalid(_)
    ));
    assert!(host.compiled_source_expression(&original).is_none());
}

#[test]
fn retained_compilation_never_caches_runtime_capability_results() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Pick(Arc<AtomicUsize>);
    impl NativeQueryFunction for Pick {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            let index = self.0.fetch_add(1, Ordering::SeqCst);
            ItemStream::once(request.arguments[0].items[index].clone())
        }
    }
    let source = tree(r#"{#native:call("fixture.pick", library)} {first} {second}"#);
    let targets = ["first", "second"].map(|name| {
        let id = source
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
            .unwrap();
        cem_ql::eval::RetainedCemNode::new(source.clone(), id)
            .unwrap()
            .query_item()
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runtime = context(targets.into());
    runtime
        .native_functions
        .register("fixture.pick", 1, Pick(calls.clone()))
        .unwrap();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(source.clone(), Some(runtime), policy());
    let original = reference_source(&source);
    let node = host.source_reference(original.clone());
    let first_result = host.evaluate(&node);
    let ReferenceLinkEvaluation::Resolved(first) = first_result else {
        panic!("{first_result:?}")
    };
    let compiled = host.compiled_source_expression(&original).unwrap();
    let ReferenceLinkEvaluation::Resolved(second) = host.evaluate(&node) else {
        panic!()
    };
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(Arc::ptr_eq(
        &compiled,
        &host.compiled_source_expression(&original).unwrap()
    ));
    assert_ne!(
        host.declaration_node(&first[0]).unwrap().identity(),
        host.declaration_node(&second[0]).unwrap().identity()
    );
    assert!(matches!(
        original.node(),
        CemAstNode::Reference { targets: None, .. }
    ));
}

#[test]
fn expression_diagnostics_link_to_original_cem_occurrences_and_byte_positions() {
    use cem_ml::{
        schema::declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    for text in [
        "{section |\n {#library[}}",
        "{section @target={  library[  }}",
        "{section |\n {#native:call(\"é\", library)[}}",
    ] {
        let source = tree(text);
        let original = source
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Reference { node_id, .. } => {
                    SchemaDeclarationNode::new(source.ast_owner().clone(), *node_id)
                }
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "$" => {
                    SchemaDeclarationNode::new(source.ast_owner().clone(), *node_id)
                }
                _ => None,
            })
            .unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(source, Some(context(vec![])), policy());
        let node = host.source_reference(original.clone());
        let result = if matches!(original.node(), CemAstNode::Reference { .. }) {
            host.evaluate(&node)
        } else {
            host.evaluate_input_expression(&node)
        };
        let ReferenceLinkEvaluation::Invalid(diagnostics) = result else {
            panic!("{result:?}")
        };
        assert!(!diagnostics.is_empty());
        for diagnostic in diagnostics {
            assert_eq!(
                diagnostic.node.as_deref(),
                Some(original.identity().as_str())
            );
            assert_eq!(diagnostic.uri.as_deref(), Some("schema.cem"));
            assert!(diagnostic.byte_offset.unwrap() >= text.find("library").unwrap() as u64);
            assert!(
                diagnostic.byte_offset.unwrap()
                    <= text.rfind(']').unwrap_or_else(|| text.rfind('[').unwrap()) as u64 + 1
            );
            let expected_line = 1 + text[..diagnostic.byte_offset.unwrap() as usize]
                .bytes()
                .filter(|b| *b == b'\n')
                .count() as u32;
            assert_eq!(diagnostic.line, Some(expected_line));
        }
    }
}

#[test]
fn xml_expression_diagnostics_map_decoded_entities_and_split_cdata() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    for (payload, token) in [
        ("  #library[&#64;]  ", "&#64;"),
        ("\r\n#library<![CDATA[[@]]]>", "@"),
        ("#library[<![CDATA[\r\n@]]]>", "@"),
    ] {
        let text = format!("<section xmlns:r='https://cem.dev/ns/cem-ml/1'><r:expr>{payload}</r:expr><after/></section>");
        let source = cem_ml::import::import_data_bytes(
            text.as_bytes(),
            "application/xml",
            "cem",
            "source.xml",
        )
        .unwrap();
        let original = reference_source(&source);
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(source, Some(context(vec![])), policy());
        let node = host.source_reference(original.clone());
        let result = host.evaluate(&node);
        let ReferenceLinkEvaluation::Invalid(diagnostics) = result else {
            panic!("{result:?}")
        };
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.byte_offset == Some(text.find(token).unwrap() as u64))
            .unwrap_or_else(|| panic!("{text}: {diagnostics:?}"));
        assert_eq!(diagnostic.uri.as_deref(), Some("source.xml"));
        assert_eq!(
            diagnostic.node.as_deref(),
            Some(original.identity().as_str())
        );
        assert!(diagnostic
            .source_map
            .as_ref()
            .unwrap()
            .frames
            .iter()
            .any(|frame| match &frame.span {
                cem_ml::source_map::FrameSpan::Single(range) =>
                    range.start == text.find(token).unwrap() as u64
                        && range.len == token.len() as u32,
                _ => false,
            }));
    }
}

#[test]
fn equal_expression_errors_remain_attributed_to_each_original_occurrence() {
    use cem_ml::{
        schema::declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    let text = "{left | {#library[@]}}\n{right | {#library[@]}}";
    let source = tree(text);
    let originals: Vec<_> = source
        .ast()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Reference { node_id, .. } => {
                SchemaDeclarationNode::new(source.ast_owner().clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(source, Some(context(vec![])), policy());
    for (original, offset) in originals
        .iter()
        .zip(text.match_indices('@').map(|(offset, _)| offset as u64))
    {
        let node = host.source_reference(original.clone());
        let ReferenceLinkEvaluation::Invalid(diagnostics) = host.evaluate(&node) else {
            panic!()
        };
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.byte_offset == Some(offset))
            .unwrap();
        assert_eq!(
            diagnostic.node.as_deref(),
            Some(original.identity().as_str())
        );
    }
}

#[test]
fn legacy_missing_mapping_and_source_text_do_not_fabricate_coordinates() {
    use cem_ml::{
        schema::declaration_references::SchemaDeclarationHost,
        source_map::TransformKind,
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    for keep_mapping in [false, true] {
        let text = "{section | {#library[@]}}";
        let mut ast = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
        )))
        .build();
        if !keep_mapping {
            for node in &mut ast.nodes {
                if let CemAstNode::Reference { source, .. } = node {
                    source.frames.retain(|frame| {
                        !matches!(frame.transform, TransformKind::ExpressionEmbedding { .. })
                    });
                }
            }
        }
        let source =
            RetainedCemTree::new(ast, "legacy.cem", "", CemTreeSemantics::default(), None).unwrap();
        let original = reference_source(&source);
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(source, Some(context(vec![])), policy());
        let node = host.source_reference(original.clone());
        let ReferenceLinkEvaluation::Invalid(diagnostics) = host.evaluate(&node) else {
            panic!()
        };
        for diagnostic in diagnostics {
            assert_eq!(diagnostic.uri.as_deref(), Some("legacy.cem"));
            assert_eq!(
                diagnostic.node.as_deref(),
                Some(original.identity().as_str())
            );
            assert_eq!(diagnostic.line, None);
            assert_eq!(diagnostic.column, None);
            if !keep_mapping {
                assert_eq!(diagnostic.byte_offset, None);
            }
        }
    }
}

#[test]
fn runtime_expression_failures_map_local_calls_and_preserve_foreign_diagnostics() {
    use cem_ml::{
        diagnostics::Diagnostic,
        schema::declaration_references::SchemaDeclarationHost,
        value::reference_resolution::{ReferenceLinkEvaluation, ReferenceResolutionHost},
    };
    use cem_ql::native::{NativeQueryFunction, NativeQueryRequest};
    #[derive(Debug)]
    struct Fail(bool);
    impl NativeQueryFunction for Fail {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            let mut result = request.raise("fixture.runtime_failure", "original runtime message");
            if self.0 {
                result.diagnostics[0].uri = Some("foreign.cem".into());
                result.diagnostics[0].node = Some("original-foreign-node".into());
                result.diagnostics[0].byte_offset = Some(700);
            }
            result
        }
    }
    for foreign in [false, true] {
        let text = r#"{section | {#native:call("fixture.fail", library)}}"#;
        let source = tree(text);
        let original = reference_source(&source);
        let mut runtime = context(vec![]);
        runtime
            .native_functions
            .register("fixture.fail", 1, Fail(foreign))
            .unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(source, Some(runtime), policy());
        let node = host.source_reference(original.clone());
        let ReferenceLinkEvaluation::Invalid(diagnostics) = host.evaluate(&node) else {
            panic!()
        };
        let diagnostic: &Diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "fixture.runtime_failure")
            .unwrap();
        assert_eq!(diagnostic.message, "original runtime message");
        if foreign {
            assert_eq!(diagnostic.uri.as_deref(), Some("foreign.cem"));
            assert_eq!(diagnostic.node.as_deref(), Some("original-foreign-node"));
            assert_eq!(diagnostic.byte_offset, Some(700));
        } else {
            assert_eq!(diagnostic.uri.as_deref(), Some("schema.cem"));
            assert_eq!(
                diagnostic.node.as_deref(),
                Some(original.identity().as_str())
            );
            assert_eq!(
                diagnostic.byte_offset,
                Some(text.find("native:call").unwrap() as u64)
            );
        }
        assert!(host.compiled_source_expression(&original).is_some());
    }
}
