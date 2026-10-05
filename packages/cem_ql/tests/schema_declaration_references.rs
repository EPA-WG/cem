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
