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
