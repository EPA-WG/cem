use super::*;
use cem_ml::schema::document_model::{
    attribute_facets::*, shipped_datatypes::ShippedDatatype as T,
};
use cem_ql::datatype_facets::*;

#[test]
fn facet_profiles_inherit_exact_original_registration_without_name_inference() {
    let (mut host, sources) =
        types_fixture("{type @name=uri @kind=scalar} {type @name=alias @base=uri}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::Uri.representation(),
        ))
        .unwrap();
    let compile = |host: &mut _, implementations: &_| {
        compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            host,
            implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
    };
    let absent = compile(&mut host, &implementations);
    assert!(absent.is_ready());
    assert!(compiled(&absent, &sources[1]).facet_profile().is_none());
    let registered = RegisteredFacetProfile::new(
        sources[0].clone(),
        "explicit:uri",
        FacetFamily::Shipped(T::Uri),
    )
    .unwrap();
    assert_eq!(
        implementations.select_facets(
            sources[1].clone(),
            FacetProfileBinding::Ready(registered.clone())
        ),
        Err("unrelated-facet-profile-source")
    );
    let mut unavailable = implementations.clone();
    unavailable
        .select_facets(sources[0].clone(), FacetProfileBinding::Unavailable)
        .unwrap();
    let pending = compile(&mut host, &unavailable);
    assert!(!pending.is_ready());
    assert!(pending
        .issues
        .iter()
        .any(|i| i.code == "datatype-facet-profile-unavailable"));
    implementations
        .select_facets(
            sources[0].clone(),
            FacetProfileBinding::Ready(registered.clone()),
        )
        .unwrap();
    assert_eq!(
        implementations.select_facets(sources[0].clone(), FacetProfileBinding::Ready(registered)),
        Err("duplicate-facet-profile-selection")
    );
    let ready = compile(&mut host, &implementations);
    assert!(ready.is_ready(), "{:?}", ready.issues);
    assert_eq!(
        compiled(&ready, &sources[1])
            .facet_profile()
            .unwrap()
            .source()
            .declaration()
            .identity(),
        sources[0].declaration().identity()
    );
}
#[test]
fn facet_profile_checks_applicability_and_retains_original_constraint_metadata() {
    use cem_ml::schema::document_model::compile_attribute_model;
    use cem_ql::attribute_datatypes::bind_attribute_datatype;
    let profile = source(
        r#"{schema @name=test @namespace=urn:test | {types | {type @name=sample @kind=scalar}} {attributes | {attribute @name=target @type=sample @uriHosts=allowed.example}}}"#,
    );
    let declaration = node(&profile, "attribute");
    let (mut host, sources) = types_fixture_source(profile);
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::Uri.representation(),
        ))
        .unwrap();
    implementations
        .select_facets(
            sources[0].clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(
                    sources[0].clone(),
                    "uri",
                    FacetFamily::Shipped(T::Uri),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let compilation = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    let CemAstNode::Element { attributes, .. } = declaration.node() else {
        panic!()
    };
    let slot=attributes.iter().filter_map(|id|SchemaDeclarationNode::new(declaration.document().clone(),*id)).find(|n|matches!(n.node(),CemAstNode::Attribute {expanded_name,..} if expanded_name.local_name=="type")).unwrap();
    host.bind_literal_attribute_type(slot, sources[0].declaration().clone())
        .unwrap();
    let bound = bind_attribute_datatype(
        declaration.clone(),
        &compilation,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap();
    let contract = bound
        .compile_facets("urn:test", Default::default())
        .unwrap();
    assert_eq!(
        contract.binding().declaration.identity(),
        declaration.identity()
    );
    assert_eq!(
        bound.local_constraints().value_type.as_deref(),
        Some("sample")
    );
    let local = compile_attribute_model(declaration.document(), declaration.node_id()).unwrap();
    assert!(AttributeFacetContract::compile(
        "urn:test",
        &local,
        FacetFamily::Shipped(T::String),
        Default::default()
    )
    .is_err());
    assert!(AttributeFacetContract::compile(
        "urn:test",
        &local,
        FacetFamily::Nodes,
        Default::default()
    )
    .is_err());
}

fn local_contract(family: FacetFamily, fields: &str) -> AttributeFacetContract {
    let src = source(&format!(
        "{{schema | {{attributes | {{attribute @name=value @type=unrelated {fields}}}}}}}"
    ));
    let decl = node(&src, "attribute");
    let model =
        cem_ml::schema::document_model::compile_attribute_model(decl.document(), decl.node_id())
            .unwrap();
    AttributeFacetContract::compile("urn:original", &model, family, Default::default()).unwrap()
}
fn check_local(
    contract: &AttributeFacetContract,
    input: FacetInput<'_>,
) -> AttributeFacetValidation {
    let src = source("{schema | {sample @value=original}}");
    let sample = node(&src, "sample");
    let CemAstNode::Element { attributes, .. } = sample.node() else {
        panic!()
    };
    let attr = SchemaDeclarationNode::new(sample.document().clone(), attributes[0]).unwrap();
    let result = contract
        .validate_with_check(
            input,
            FacetContext {
                element_name: "sample",
                source: attr.node(),
                diagnostic_behaviors: &Default::default(),
                attribute_values: &Default::default(),
            },
            Default::default(),
            &mut || Ok::<_, ()>(()),
        )
        .unwrap();
    let CemAstNode::Attribute { source, .. } = attr.node() else {
        panic!()
    };
    assert!(result
        .diagnostics
        .iter()
        .all(|d| d.source_map.as_ref() == Some(source)));
    result
}
#[test]
fn facet_profiles_reuse_shipped_scalar_semantics_without_rewriting_input() {
    for (family, fields, valid, invalid) in [
        (
            T::Uri,
            "@uriHosts=allowed.example",
            "https://allowed.example/a",
            "https://other.example/a",
        ),
        (T::Path, "@pathExtensions=cem", "./a.cem", "./a.txt"),
        (
            T::String,
            r#"@pattern="[a-z]+" @minLength=2 @whiteSpace=collapse"#,
            "  red  ",
            "123",
        ),
        (T::Integer, "@minInclusive=2 @maxInclusive=4", "003", "5"),
        (T::Integer, r#"@values="003 004""#, "003", "3"),
    ] {
        let contract = local_contract(FacetFamily::Shipped(family), fields);
        assert!(
            check_local(&contract, FacetInput::Scalar(valid)).accepted,
            "{family:?}"
        );
        assert!(
            !check_local(&contract, FacetInput::Scalar(invalid)).accepted,
            "{family:?}"
        );
    }
    // URI facet checks must not silently accept values outside the URI domain.
    assert!(
        !check_local(
            &local_contract(FacetFamily::Shipped(T::Uri), "@uriHosts=allowed.example"),
            FacetInput::Scalar("not-uri")
        )
        .accepted
    );
}
#[test]
fn facet_profiles_use_prepared_sequence_counts_without_atomization_or_retokenization() {
    let list = local_contract(
        FacetFamily::List(ScalarRepresentation::Integer),
        "@itemCount=2",
    );
    assert!(
        check_local(
            &list,
            FacetInput::List {
                lexical: "003,004",
                count: 2
            }
        )
        .accepted
    );
    assert!(
        !check_local(
            &list,
            FacetInput::List {
                lexical: "003,004",
                count: 1
            }
        )
        .accepted
    );
    let node = local_contract(FacetFamily::Nodes, "@minItems=0 @maxItems=2");
    assert!(check_local(&node, FacetInput::Nodes { count: 0 }).accepted);
    assert!(!check_local(&node, FacetInput::Nodes { count: 3 }).accepted);
    let node = local_contract(FacetFamily::Nodes, "");
    assert!(check_local(&node, FacetInput::Nodes { count: 1 }).accepted);
    assert!(!check_local(&node, FacetInput::Nodes { count: 0 }).accepted);
    let profile = source(
        r#"{schema | {attributes | {attribute @name=value @type={#target} @values="one two"}}}"#,
    );
    let decl = node_for_attribute(&profile);
    let model =
        cem_ml::schema::document_model::compile_attribute_model(decl.document(), decl.node_id())
            .unwrap();
    assert!(model.native_type_pending);
    assert!(matches!(
        AttributeFacetContract::compile("test", &model, FacetFamily::Nodes, Default::default()),
        Err(FacetCompilationError::Invalid(_))
    ));
    assert!(
        model.native_type_pending,
        "original readiness never changes"
    );
}
fn node_for_attribute(src: &ValueContractSource) -> SchemaDeclarationNode {
    node(src, "attribute")
}
#[test]
fn facet_profiles_check_syntax_limits_and_control_before_reporting_acceptance() {
    let contract = local_contract(FacetFamily::Shipped(T::String), "@minLength=2");
    let src = source("{schema | {sample @value=x}}");
    let candidate = node(&src, "sample");
    let behaviours = BTreeMap::new();
    let attributes = BTreeMap::new();
    let context = || FacetContext {
        element_name: "sample",
        source: candidate.node(),
        diagnostic_behaviors: &behaviours,
        attribute_values: &attributes,
    };
    assert!(matches!(
        contract.validate_with_check(
            FacetInput::Scalar("x"),
            context(),
            Default::default(),
            &mut || Err("cancelled")
        ),
        Err(FacetExecutionError::Interrupted("cancelled"))
    ));
    for limits in [
        FacetLimits {
            max_input_bytes: 0,
            ..Default::default()
        },
        FacetLimits {
            max_diagnostics: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            contract.validate_with_check(FacetInput::Scalar("x"), context(), limits, &mut || Ok::<
                _,
                (),
            >(
                ()
            )),
            Err(FacetExecutionError::Limit)
        ));
    }
    assert!(matches!(
        contract.validate_with_check(
            FacetInput::Nodes { count: 1 },
            context(),
            Default::default(),
            &mut || Ok::<_, ()>(())
        ),
        Err(FacetExecutionError::InvalidInput)
    ));
    for fields in ["@pattern=\"[\"", "@minLength=-1", "@whiteSpace=unknown"] {
        let src = source(&format!(
            "{{schema | {{attributes | {{attribute @name=value @type=unrelated {fields}}}}}}}"
        ));
        let decl = node(&src, "attribute");
        let model = cem_ml::schema::document_model::compile_attribute_model(
            decl.document(),
            decl.node_id(),
        )
        .unwrap();
        assert!(matches!(
            AttributeFacetContract::compile(
                "test",
                &model,
                FacetFamily::Shipped(T::String),
                Default::default()
            ),
            Err(FacetCompilationError::Invalid(_))
        ));
        assert!(matches!(
            AttributeFacetContract::compile(
                "test",
                &model,
                FacetFamily::Shipped(T::String),
                FacetLimits {
                    max_model_bytes: 0,
                    ..Default::default()
                }
            ),
            Err(FacetCompilationError::Limit)
        ));
    }
}

#[test]
fn facet_profiles_reject_representation_changes_and_derived_replacement() {
    let (mut host, sources) =
        types_fixture("{type @name=base @kind=scalar} {type @name=child @base=base}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::String.representation(),
        ))
        .unwrap();
    let mut mismatch = implementations.clone();
    mismatch
        .select_facets(
            sources[0].clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(
                    sources[0].clone(),
                    "integer",
                    FacetFamily::Shipped(T::Integer),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let compile = |host: &mut _, implementations: &_| {
        compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            host,
            implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
    };
    let result = compile(&mut host, &mismatch);
    assert!(!result.is_ready());
    assert!(result
        .issues
        .iter()
        .any(|i| i.code == "facet-profile-representation-incompatible"));
    for source in &sources {
        implementations
            .select_facets(
                source.clone(),
                FacetProfileBinding::Ready(
                    RegisteredFacetProfile::new(
                        source.clone(),
                        "string",
                        FacetFamily::Shipped(T::String),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
    }
    let result = compile(&mut host, &implementations);
    assert!(!result.is_ready());
    assert!(result
        .issues
        .iter()
        .any(|i| i.code == "facet-profile-base-replacement-unsupported"));
}
