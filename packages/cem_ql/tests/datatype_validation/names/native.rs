use super::*;
use cem_ml::value::reference_resolution::ReferenceResolutionHost;
use cem_ql::api::{StandaloneExpressionBinding, StandaloneExpressionContext};
fn member(input: &DatatypeSchemaSource, kind: &str) -> SchemaDeclarationNode {
    input
        .schema
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == kind => {
                SchemaDeclarationNode::new(input.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn bind(
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
    input: &DatatypeSchemaSource,
    nodes: &[SchemaDeclarationNode],
) {
    let scope = host
        .scope(&host.source_reference(input.schema.clone()))
        .unwrap();
    let items = nodes
        .iter()
        .map(|node| {
            RetainedCemNode::new(host.source_tree(node).unwrap().clone(), node.node_id())
                .unwrap()
                .query_item()
        })
        .collect();
    host.set_context(
        scope,
        Some(StandaloneExpressionContext::default().with_binding(
            "library",
            StandaloneExpressionBinding::any(ItemStream::from_items(items)),
        )),
    );
}
#[test]
fn native_discovery_retains_vendor_environment_and_explicit_exports() {
    let (mut host, mut consumer) = authored("{schema @namespace=urn:consumer | {types | {#library} {type @name=local @base=v:exported}} {uses | {use @as=v @schema=urn:vendor}}}");
    let (vendor_host, vendor) = authored("{schema @namespace=urn:vendor | {library @xmlns:own=urn:vendor | {type @name=exported @base=own:base}} {types | {type @name=base @kind=scalar}}}");
    let target = member(&vendor, "type");
    let from = host
        .scope(&host.source_reference(consumer.schema.clone()))
        .unwrap();
    let to = host.register_scope(
        vendor_host.source_tree(&vendor.schema).unwrap().clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    bind(&mut host, &consumer, &[target.clone()]);
    let error = DatatypeNameCatalog::discover(
        &[consumer.clone(), vendor.clone()],
        &mut host,
        Default::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "datatype-selection-scope-denied");
    assert!(!error.pending);
    host.allow_scope_crossing(from, to);
    let catalog = DatatypeNameCatalog::discover(
        &[consumer.clone(), vendor.clone()],
        &mut host,
        Default::default(),
    )
    .unwrap();
    let selected = catalog.source(&target).unwrap();
    assert_eq!(selected.scope().identity(), vendor.schema.identity());
    assert!(Arc::ptr_eq(
        selected.declaration().document(),
        vendor.schema.document()
    ));
    assert!(matches!(
        catalog.lookup(selected, "own:base"),
        DatatypeNameLookup::Target(_)
    ));
    let local = catalog
        .sources()
        .find(|s| s.scope().identity() == consumer.schema.identity())
        .unwrap();
    assert!(matches!(
        catalog.lookup(local, "v:exported"),
        DatatypeNameLookup::Unresolved(_)
    ));
    consumer.imports.push(DatatypeExport {
        namespace: "urn:vendor".into(),
        name: "exported".into(),
        source: selected.clone(),
    });
    let ready =
        DatatypeNameCatalog::discover(&[consumer.clone(), vendor], &mut host, Default::default())
            .unwrap();
    assert!(matches!(
        ready.lookup(local, "v:exported"),
        DatatypeNameLookup::Target(_)
    ));
    let missing =
        DatatypeNameCatalog::discover(&[consumer], &mut host, Default::default()).unwrap_err();
    assert_eq!(missing.code, "datatype-selected-schema-unavailable");
    assert!(missing.pending);
}
#[test]
fn native_uses_supply_aliases_and_repeated_types_reuse_original_handles() {
    let (mut host, input) = authored("{schema @namespace=urn:own | {uses | {#library}} {types | {type @name=derived @base=p:base} {type @name=base @kind=scalar}} {library | {use @as=p @schema=urn:own}}}");
    bind(&mut host, &input, &[member(&input, "use")]);
    let catalog = DatatypeNameCatalog::discover(&[input], &mut host, Default::default()).unwrap();
    let derived = catalog
        .sources()
        .find(|s| s.attribute("base").is_some())
        .unwrap();
    assert!(matches!(
        catalog.lookup(derived, "p:base"),
        DatatypeNameLookup::Target(_)
    ));
    let (mut host, input) = authored("{schema @namespace=urn:own | {types | {#library} {#library}} {library | {type @name=base @kind=scalar}}}");
    bind(&mut host, &input, &[member(&input, "type")]);
    let catalog = DatatypeNameCatalog::discover(&[input], &mut host, Default::default()).unwrap();
    assert_eq!(catalog.sources().count(), 1);
}
#[test]
fn native_slots_distinguish_pending_empty_invalid_and_cycles() {
    let (mut host, input) = authored("{schema @namespace=urn:own | {types | {#library}} {library | {use @as=p @schema=urn:own}}}");
    let scope = host
        .scope(&host.source_reference(input.schema.clone()))
        .unwrap();
    host.set_context(scope, None);
    assert!(
        DatatypeNameCatalog::discover(&[input.clone()], &mut host, Default::default())
            .unwrap_err()
            .pending
    );
    bind(&mut host, &input, &[]);
    assert_eq!(
        DatatypeNameCatalog::discover(&[input.clone()], &mut host, Default::default())
            .unwrap()
            .sources()
            .count(),
        0
    );
    bind(&mut host, &input, &[member(&input, "use")]);
    assert_eq!(
        DatatypeNameCatalog::discover(&[input.clone()], &mut host, Default::default())
            .unwrap_err()
            .code,
        "datatype-declaration-required"
    );
    let reference = input
        .schema
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Reference { node_id, .. } => {
                SchemaDeclarationNode::new(input.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    bind(&mut host, &input, &[reference.clone()]);
    let error =
        DatatypeNameCatalog::discover(&[input.clone()], &mut host, Default::default()).unwrap_err();
    assert_eq!(error.code, "datatype-selection-cycle");
    assert_eq!(error.source.identity(), reference.identity());
    assert!(matches!(
        reference.node(),
        CemAstNode::Reference { targets: None, .. }
    ));
}
#[test]
fn native_slots_share_request_and_destination_budgets() {
    let (mut host, input) = authored("{schema @namespace=urn:own | {types | {#library} {#library}} {library | {type @name=base @kind=scalar}}}");
    bind(&mut host, &input, &[member(&input, "type")]);
    let error = DatatypeNameCatalog::discover_with_limits(
        &[input.clone()],
        &mut host,
        Default::default(),
        ReferenceTraversalLimits {
            max_depth: 8,
            max_work: 5,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "datatype-selection-work-limit");
    assert!(error.pending);
    let (vendor_host, vendor) =
        authored("{schema @namespace=urn:vendor | {types | {type @name=base @kind=scalar}}}");
    let mut policy = ReferenceScopePolicy::schema_defaults().unwrap();
    policy.limits.max_work = 1;
    let to = host.register_scope(
        vendor_host.source_tree(&vendor.schema).unwrap().clone(),
        Some(Default::default()),
        policy,
    );
    let from = host
        .scope(&host.source_reference(input.schema.clone()))
        .unwrap();
    host.allow_scope_crossing(from, to);
    bind(&mut host, &input, &[member(&vendor, "type")]);
    let error =
        DatatypeNameCatalog::discover(&[input, vendor], &mut host, Default::default()).unwrap_err();
    assert_eq!(error.code, "datatype-selection-work-limit");
}

#[test]
fn native_selection_resolves_chains_but_preserves_selected_descendants() {
    let (mut host, input) = authored("{schema @namespace=urn:own | {types | {#library}} {library | {type @name=base @kind=scalar | {#unavailable}}}} ");
    let target = member(&input, "type");
    bind(&mut host, &input, &[target.clone()]);
    let catalog =
        DatatypeNameCatalog::discover(&[input.clone()], &mut host, Default::default()).unwrap();
    assert_eq!(
        catalog.source(&target).unwrap().declaration().identity(),
        target.identity()
    );
    assert!(input.schema.document().nodes.iter().any(|node|matches!(node,CemAstNode::Reference {expression,targets:None,..} if expression == "#unavailable")));

    // A reference target is expanded with its own registered lifecycle context.
    let middle = input
        .schema
        .document()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == "#unavailable" => {
                SchemaDeclarationNode::new(input.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap();
    let parent = host
        .scope(&host.source_reference(input.schema.clone()))
        .unwrap();
    let item = RetainedCemNode::new(host.source_tree(&target).unwrap().clone(), target.node_id())
        .unwrap()
        .query_item();
    let context = StandaloneExpressionContext::default().with_binding(
        "unavailable",
        StandaloneExpressionBinding::any(ItemStream::once(item)),
    );
    let scope = host
        .register_lexical_scope(
            parent,
            Some(context),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let tree = host.source_tree(&middle).unwrap().clone();
    assert!(host.assign_subtree_scope(&tree, middle.node_id(), scope));
    bind(&mut host, &input, &[middle]);
    assert!(DatatypeNameCatalog::discover(&[input.clone()], &mut host, Default::default()).is_ok());
    let error = DatatypeNameCatalog::discover_with_limits(
        &[input],
        &mut host,
        Default::default(),
        ReferenceTraversalLimits {
            max_depth: 1,
            max_work: 100,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "datatype-selection-depth-limit");
    assert!(error.pending);
}

#[test]
fn pending_sibling_does_not_hide_invalid_selection() {
    let (mut host, input) = authored("{schema @namespace=urn:own | {types | {#pending} {#1}}}");
    let pending = input
        .schema
        .document()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == "#pending" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let parent = host
        .scope(&host.source_reference(input.schema.clone()))
        .unwrap();
    let scope = host
        .register_lexical_scope(
            parent,
            None,
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let tree = host.source_tree(&input.schema).unwrap().clone();
    host.assign_subtree_scope(&tree, pending, scope);
    let error = DatatypeNameCatalog::discover(&[input], &mut host, Default::default()).unwrap_err();
    assert!(!error.pending);
    assert_eq!(error.code, "datatype-selection-invalid");
    assert!(!error.diagnostics.is_empty());
    assert!(
        matches!(error.source.node(),CemAstNode::Reference {expression,..} if expression == "#1")
    );
}
