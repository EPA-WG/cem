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
    fixture_with_fields("")
}
fn fixture_with_fields(
    fields: &str,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    cem_ml::schema::datatype_contracts::DatatypeCompilation,
    Vec<SchemaDeclarationNode>,
) {
    let profile = source(&format!("@ns ext = \"urn:constraint-extension\"\n{{schema @name=test @namespace=urn:test | {{types | {{type @name=sample @kind=scalar}}}} {{attributes | {{attribute @name=literal @type=vendor:sample {fields}}} {{attribute @name=native @type={{#target}} {fields}}}}}}}"));
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

#[test]
fn attribute_binding_retains_original_local_constraints_without_activation() {
    let (mut host, compilation, declarations) = fixture_with_fields(
        r#"@values="red blue" @pattern="[a-z]+" @whiteSpace=collapse @minLength=2 @default=red"#,
    );
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
    for (index, decl) in declarations.iter().enumerate() {
        let result = bind_attribute_datatype(
            decl.clone(),
            &compilation,
            &mut host,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
        .unwrap();
        let bound = result.bound.unwrap();
        let local = bound.local_constraints();
        assert_eq!(local.pattern.as_deref(), Some("[a-z]+"));
        assert_eq!(local.white_space.as_deref(), Some("collapse"));
        assert_eq!(local.min_length.as_deref(), Some("2"));
        assert_eq!(local.default_value.as_deref(), Some("red"));
        assert_eq!(
            local.allowed_values,
            std::collections::BTreeSet::from(["red".into(), "blue".into()])
        );
        assert_eq!(local.native_type_pending, index == 1);
        let CemAstNode::Element { source, .. } = decl.node() else {
            panic!()
        };
        assert_eq!(&local.source_map, source);
        assert!(bound
            .constraint_fields()
            .iter()
            .all(|field| Arc::ptr_eq(field.document(), decl.document())));
        let field=bound.constraint_fields().iter().find(|field|matches!(field.node(),CemAstNode::Attribute {expanded_name,..} if expanded_name.local_name=="pattern")).unwrap();
        assert!(
            matches!(field.node(),CemAstNode::Attribute {value:Some(value),..} if value=="[a-z]+")
        );
    }
}
#[test]
fn attribute_binding_requires_ready_literal_constraint_fields() {
    for (fields, state, issue) in [
        (
            "@pattern={pattern}",
            ReferenceResolutionState::Pending,
            "attribute-constraint-value-pending",
        ),
        (
            "@name={name}",
            ReferenceResolutionState::Pending,
            "attribute-constraint-value-pending",
        ),
        (
            r#"@name="""#,
            ReferenceResolutionState::Invalid,
            "attribute-name-required",
        ),
        (
            "@ext:pattern=ignored",
            ReferenceResolutionState::Invalid,
            "attribute-constraint-field-namespace",
        ),
    ] {
        let (mut host, compilation, declarations) = fixture_with_fields(fields);
        let target = compilation.sources[0].declaration().clone();
        host.bind_literal_attribute_type(slot(&declarations[0]), target)
            .unwrap();
        let result = bind_attribute_datatype(
            declarations[0].clone(),
            &compilation,
            &mut host,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
        .unwrap();
        assert!(result.bound.is_none(), "{fields}: {result:?}");
        assert_eq!(result.state, state);
        assert_eq!(result.issue, Some(issue));
    }
    let (mut host, compilation, declarations) =
        fixture_with_fields("@pattern={pending} @pattern=ready");
    host.bind_literal_attribute_type(
        slot(&declarations[0]),
        compilation.sources[0].declaration().clone(),
    )
    .unwrap();
    let bound = bind_attribute_datatype(
        declarations[0].clone(),
        &compilation,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap();
    assert_eq!(bound.local_constraints().pattern.as_deref(), Some("ready"));
}
