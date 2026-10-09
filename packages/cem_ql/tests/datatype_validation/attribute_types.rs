use super::*;
use cem_ml::schema::datatype_contracts::CompiledDatatypeContract;
use cem_ml::{
    schema::declaration_references::SchemaDeclarationHost,
    value::reference_resolution::{ReferenceResolutionHost, ReferenceResolutionState},
};
use cem_ql::attribute_datatypes::bind_attribute_datatype;

fn fixture() -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    cem_ml::schema::datatype_contracts::DatatypeCompilation,
    Vec<SchemaDeclarationNode>,
) {
    let profile = source("{schema @name=test @namespace=urn:test | {types | {type @name=sample @kind=scalar}} {attributes | {attribute @name=literal @type=vendor:sample} {attribute @name=native @type={#target}}}}");
    let declarations = profile
        .schema
        .document()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "attribute" => {
                SchemaDeclarationNode::new(profile.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    let (mut host, sources) = types_fixture_source(profile);
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    let compilation = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(compilation.is_ready());
    (host, compilation, declarations)
}
fn slot(declaration: &SchemaDeclarationNode) -> SchemaDeclarationNode {
    let CemAstNode::Element { attributes, .. } = declaration.node() else {
        panic!()
    };
    attributes.iter().filter_map(|id| SchemaDeclarationNode::new(declaration.document().clone(),*id)).find(|n| matches!(n.node(),CemAstNode::Attribute {expanded_name,..} if expanded_name.local_name == "type")).unwrap()
}
#[test]
fn literal_and_native_attribute_slots_bind_same_original_contract() {
    let (mut host, compilation, declarations) = fixture();
    let limits = ReferenceTraversalLimits::schema_defaults().unwrap();
    let pending =
        bind_attribute_datatype(declarations[0].clone(), &compilation, &mut host, limits).unwrap();
    assert!(pending.bound.is_none());
    assert_eq!(pending.state, ReferenceResolutionState::Pending);
    let target = compilation.sources[0].declaration().clone();
    host.bind_literal_attribute_type(slot(&declarations[0]), target.clone())
        .unwrap();
    let scope = host
        .scope(&host.source_reference(declarations[1].clone()))
        .unwrap();
    assert!(host.set_context(
        scope,
        Some(
            cem_ql::api::StandaloneExpressionContext::default().with_binding(
                "target",
                cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(native(&target)))
            )
        )
    ));
    for declaration in declarations {
        let result =
            bind_attribute_datatype(declaration.clone(), &compilation, &mut host, limits).unwrap();
        assert_eq!(
            result.state,
            ReferenceResolutionState::Resolved,
            "{result:?}"
        );
        let bound = result.bound.unwrap();
        assert_eq!(bound.declaration.identity(), declaration.identity());
        assert_eq!(
            bound.datatype.source().declaration().identity(),
            target.identity()
        );
        assert!(result.work_used > 0);
    }
}
#[test]
fn attribute_type_binding_preserves_incomplete_compilation_and_singleton_policy() {
    let (mut host, mut compilation, declarations) = fixture();
    let limits = ReferenceTraversalLimits::schema_defaults().unwrap();
    let target = compilation.sources[0].declaration().clone();
    host.bind_literal_attribute_type(slot(&declarations[0]), target.clone())
        .unwrap();
    compilation.contracts.clear();
    let result =
        bind_attribute_datatype(declarations[0].clone(), &compilation, &mut host, limits).unwrap();
    assert!(result.bound.is_none());
    assert_eq!(result.issue, Some("datatype-compilation-incomplete"));
    let scope = host
        .scope(&host.source_reference(declarations[1].clone()))
        .unwrap();
    assert!(host.set_context(
        scope,
        Some(
            cem_ql::api::StandaloneExpressionContext::default().with_binding(
                "target",
                cem_ql::api::StandaloneExpressionBinding::any(ItemStream::from_items(vec![
                    native(&target),
                    native(&target)
                ]))
            )
        )
    ));
    let result =
        bind_attribute_datatype(declarations[1].clone(), &compilation, &mut host, limits).unwrap();
    assert_eq!(result.issue, Some("attribute-type-singleton-required"));
    assert_eq!(result.state, ReferenceResolutionState::Invalid);
}

#[test]
fn attribute_type_binding_requires_directed_crossing_and_exact_snapshot_owner() {
    let (mut host, old_compilation, declarations) = fixture();
    let (_, replacement, _) = fixture();
    let source = replacement.sources[0].clone();
    let target = source.declaration().clone();
    let tree = RetainedCemTree::from_shared(
        target.document().clone(),
        "replacement.cem",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    let destination = host.register_scope(
        tree,
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.register_datatype_source(source).unwrap();
    let requester = host
        .scope(&host.source_reference(declarations[0].clone()))
        .unwrap();
    host.bind_literal_attribute_type(slot(&declarations[0]), target.clone())
        .unwrap();
    assert!(host.set_context(
        requester,
        Some(
            cem_ql::api::StandaloneExpressionContext::default().with_binding(
                "target",
                cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(native(&target))),
            )
        )
    ));
    let limits = ReferenceTraversalLimits::schema_defaults().unwrap();
    for declaration in &declarations {
        let denied =
            bind_attribute_datatype(declaration.clone(), &replacement, &mut host, limits).unwrap();
        assert!(denied.bound.is_none(), "{denied:?}");
        assert_ne!(denied.state, ReferenceResolutionState::Resolved);
    }
    assert!(host.allow_scope_crossing(requester, destination));
    for declaration in &declarations {
        let stale =
            bind_attribute_datatype(declaration.clone(), &old_compilation, &mut host, limits)
                .unwrap();
        assert_eq!(stale.issue, Some("attribute-type-contract-unavailable"));
        assert!(stale.bound.is_none());
        let bound = bind_attribute_datatype(declaration.clone(), &replacement, &mut host, limits)
            .unwrap()
            .bound
            .unwrap();
        assert!(Arc::ptr_eq(
            bound.datatype.source().declaration().document(),
            target.document()
        ));
        assert!(Arc::ptr_eq(
            bound.declaration.document(),
            declarations[0].document()
        ));
    }
}

#[test]
fn attribute_type_binding_retains_pending_context_and_traversal_limits() {
    let (mut host, compilation, declarations) = fixture();
    let target = compilation.sources[0].declaration().clone();
    let scope = host
        .scope(&host.source_reference(declarations[0].clone()))
        .unwrap();
    host.bind_literal_attribute_type(slot(&declarations[0]), target.clone())
        .unwrap();
    assert!(host.set_context(scope, None));
    let limits = ReferenceTraversalLimits::schema_defaults().unwrap();
    // An explicitly completed lexical binding needs no expression inputs.
    assert!(
        bind_attribute_datatype(declarations[0].clone(), &compilation, &mut host, limits)
            .unwrap()
            .bound
            .is_some()
    );
    let pending =
        bind_attribute_datatype(declarations[1].clone(), &compilation, &mut host, limits).unwrap();
    assert!(pending.bound.is_none());
    assert_eq!(pending.state, ReferenceResolutionState::Pending);
    assert!(host.set_context(
        scope,
        Some(
            cem_ql::api::StandaloneExpressionContext::default().with_binding(
                "target",
                cem_ql::api::StandaloneExpressionBinding::any(ItemStream::once(native(&target))),
            )
        )
    ));
    for declaration in &declarations {
        let bounded = bind_attribute_datatype(
            declaration.clone(),
            &compilation,
            &mut host,
            ReferenceTraversalLimits {
                max_work: 1,
                ..limits
            },
        )
        .unwrap();
        assert!(bounded.bound.is_none(), "{bounded:?}");
        assert_ne!(bounded.state, ReferenceResolutionState::Resolved);
    }
}
