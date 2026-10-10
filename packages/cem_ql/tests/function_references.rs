//! Function selection uses the production host's retained query and grant path.
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{tree::RetainedCemTree, CemAstNode},
    schema::{
        declaration_references::SchemaDeclarationNode,
        function_references::{FunctionCatalog, FunctionSelectionBudget},
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        reference_policy::ReferenceScopePolicy,
        reference_traversal::ReferenceTraversalLimits,
        registry::CEM_SCHEMA_URI,
        value_contracts::ValueContractSource,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    value::reference_resolution::{ReferenceResolutionIssueKind, ReferenceResolutionState},
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{Item, ItemStream, RetainedCemNode},
    schema_references::CemQlSchemaDeclarationHost,
};
use std::{collections::BTreeMap, sync::Arc};

#[path = "function_references/bindings.rs"]
mod bindings;

fn source(
    body: &str,
    uri: &str,
) -> (
    ValueContractSource,
    Arc<RetainedCemTree>,
    Arc<LexicallyScopedDocument>,
) {
    let text = format!("@ns schema = \"{CEM_SCHEMA_URI}\"\n@default schema\n{{schema | {body}}}");
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let doc = captured.document().clone();
    assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
    let id = doc
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let source = ValueContractSource::new(
        SchemaDeclarationNode::new(doc.clone(), id).unwrap(),
        uri,
        BTreeMap::new(),
    )
    .with_captured_names(captured.clone())
    .unwrap();
    let tree = RetainedCemTree::from_shared(doc, uri, &text, Default::default(), None).unwrap();
    (source, tree, captured)
}
fn node(source: &ValueContractSource, local: &str) -> SchemaDeclarationNode {
    source
        .schema
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => {
                SchemaDeclarationNode::new(source.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn context(items: Vec<Item>) -> StandaloneExpressionContext {
    StandaloneExpressionContext::default().with_binding(
        "chosen",
        StandaloneExpressionBinding::any(ItemStream::from_items(items)),
    )
}
fn budget() -> FunctionSelectionBudget {
    FunctionSelectionBudget::new(ReferenceTraversalLimits {
        max_depth: 32,
        max_work: 10_000,
    })
    .unwrap()
}

#[test]
fn query_selection_requires_grants_preserves_owners_and_rechecks_context() {
    let (caller, tree, captured) = source(
        "{behaviors | {behavior @name=caller @implementation=function @function={#chosen}}}",
        "caller.cem",
    );
    let (library, library_tree, library_captured) = source("{behaviors | {behavior @name=library | {function @name=f @returns=string @visibility=public}}}", "library.cem");
    let function = node(&library, "function");
    let original_identity = function.identity();
    let original_owner = Arc::downgrade(function.document());
    let item = RetainedCemNode::new(library_tree.clone(), function.node_id())
        .unwrap()
        .query_item();
    let mut host = CemQlSchemaDeclarationHost::new();
    let from = host.register_scope(
        tree.clone(),
        Some(context(vec![item.clone()])),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let to = host.register_scope(
        library_tree.clone(),
        None,
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.attach_captured_names(&captured).unwrap();
    host.attach_captured_names(&library_captured).unwrap();
    let mut budget = budget();
    let catalog =
        FunctionCatalog::collect(&[caller.clone(), library.clone()], &mut budget).unwrap();
    let behavior = node(&caller, "behavior");
    let denied = catalog.select(&behavior, &mut host, &mut budget).unwrap();
    assert!(denied.target().is_none());
    assert!(denied
        .resolution
        .issues
        .iter()
        .any(|i| i.kind == ReferenceResolutionIssueKind::ScopeDenied));
    assert!(host.allow_scope_crossing(from, to));
    let selected = catalog.select(&behavior, &mut host, &mut budget).unwrap();
    let target = selected.target().unwrap().clone();
    assert_eq!(target.function().identity(), function.identity());
    assert!(Arc::ptr_eq(
        target.function().document(),
        library_tree.ast_owner()
    ));
    assert!(host.set_context(from, Some(context(vec![]))));
    let empty = catalog.select(&behavior, &mut host, &mut budget).unwrap();
    assert!(empty.target().is_none());
    assert!(empty
        .resolution
        .diagnostics
        .iter()
        .any(|d| d.code.ends_with("function-cardinality")));
    assert!(empty
        .resolution
        .diagnostics
        .iter()
        .any(|d| d.uri.as_deref() == Some("caller.cem")));
    assert!(host.set_context(from, Some(context(vec![item.clone(), item]))));
    assert!(catalog
        .select(&behavior, &mut host, &mut budget)
        .unwrap()
        .target()
        .is_none());
    assert!(tree
        .ast()
        .nodes
        .iter()
        .filter(|n| matches!(n, CemAstNode::Reference { .. }))
        .all(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
    drop(host);
    drop(catalog);
    drop(library);
    drop(library_tree);
    drop(library_captured);
    drop(selected);
    drop(function);
    assert_eq!(target.function().identity(), original_identity);
    assert!(original_owner.upgrade().is_some());
    drop(target);
    assert!(original_owner.upgrade().is_none());
}

#[test]
fn missing_context_and_scalar_query_results_never_become_functions() {
    for expression in ["chosen", "42"] {
        let (caller, tree, captured) = source(&format!("{{behaviors | {{behavior @name=caller @implementation=function @function={{#{expression}}}}}}}"), "caller.cem");
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            tree,
            (expression == "42").then(StandaloneExpressionContext::default),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        );
        host.attach_captured_names(&captured).unwrap();
        let mut budget = budget();
        let catalog = FunctionCatalog::collect(&[caller.clone()], &mut budget).unwrap();
        let selected = catalog
            .select(&node(&caller, "behavior"), &mut host, &mut budget)
            .unwrap();
        assert!(selected.target().is_none());
        assert_ne!(
            selected.resolution.state,
            ReferenceResolutionState::Resolved
        );
        if expression == "42" {
            assert!(!selected.resolution.diagnostics.is_empty());
        }
    }
}

#[test]
fn assembled_behavior_collections_resolve_forward_names_without_changing_owners() {
    let (caller, tree, captured) = source(
        "{behaviors | {behavior @name=caller @implementation=function @function=f} {#chosen}}",
        "caller.cem",
    );
    let (library, foreign, names) = source("{behaviors | {behavior @name=library | {function @name=f @returns=string @visibility=public}}}", "library.cem");
    let behavior = node(&library, "behavior");
    let function = node(&library, "function");
    let mut host = CemQlSchemaDeclarationHost::new();
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let a = host.register_scope(
        tree,
        Some(context(vec![RetainedCemNode::new(
            foreign.clone(),
            behavior.node_id(),
        )
        .unwrap()
        .query_item()])),
        policy.clone(),
    );
    let b = host.register_scope(foreign, None, policy);
    host.attach_captured_names(&captured).unwrap();
    host.attach_captured_names(&names).unwrap();
    let sources = [caller.clone(), library.clone()];
    let denied = FunctionCatalog::assemble(&sources, &mut host, &mut budget()).unwrap();
    assert!(!denied.assembly_is_complete());
    assert_eq!(
        denied
            .select(&node(&caller, "behavior"), &mut host, &mut budget())
            .unwrap()
            .resolution
            .state,
        ReferenceResolutionState::Pending
    );
    host.allow_scope_crossing(a, b);
    let catalog = FunctionCatalog::assemble(&sources, &mut host, &mut budget()).unwrap();
    assert!(catalog.assembly_is_complete());
    let selected = catalog
        .select(&node(&caller, "behavior"), &mut host, &mut budget())
        .unwrap();
    let target = selected.target().unwrap();
    assert_eq!(target.function().identity(), function.identity());
    assert_eq!(target.source().schema.identity(), library.schema.identity());
    assert_eq!(target.behavior().identity(), behavior.identity());
}

#[test]
fn assembled_collection_rejects_wrong_targets_with_original_attribution() {
    let (source, tree, captured) = source(
        "{behaviors | {#chosen} {behavior @name=library | {function @name=f @visibility=public}}}",
        "wrong-kind.cem",
    );
    let target = node(&source, "function");
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(
        tree.clone(),
        Some(context(vec![RetainedCemNode::new(tree, target.node_id())
            .unwrap()
            .query_item()])),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.attach_captured_names(&captured).unwrap();
    let catalog = FunctionCatalog::assemble(&[source], &mut host, &mut budget()).unwrap();
    assert!(!catalog.assembly_is_complete());
    let resolution = &catalog.assembly_resolutions()[0];
    assert_eq!(resolution.state, ReferenceResolutionState::Invalid);
    assert_eq!(resolution.nodes[0].identity(), target.identity());
    assert!(resolution
        .diagnostics
        .iter()
        .any(|d| d.code.ends_with("function-collection-target")
            && d.uri.as_deref() == Some("wrong-kind.cem")
            && d.source_map.is_some()
            && d.details
                .as_ref()
                .and_then(|v| v.get("target"))
                .and_then(|v| v.as_str())
                == Some(target.identity().as_str())));
}
