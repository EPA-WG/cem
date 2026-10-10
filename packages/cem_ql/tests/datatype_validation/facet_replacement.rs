use super::*;
use cem_ml::schema::{
    datatype_contracts::{DatatypeCompilation, LexicalInput, RegisteredTokenizer},
    document_model::{attribute_facets::*, shipped_datatypes::ShippedDatatype as T},
};
use cem_ql::{
    attribute_datatypes::{bind_attribute_datatype, BoundAttributeDatatype},
    attribute_validation::*,
    datatype_facets::*,
    datatype_preparation::*,
    preparation_evidence::AttributePreparationInvocation,
    schema_references::CemQlSchemaDeclarationHost,
};

fn compile(
    host: &mut CemQlSchemaDeclarationHost,
    sources: &[DatatypeSource],
    registrations: &DatatypeImplementations,
) -> DatatypeCompilation {
    compile_datatypes(
        sources[0].declaration().document().clone(),
        sources,
        host,
        registrations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
}
fn profile(source: &DatatypeSource, family: FacetFamily) -> RegisteredFacetProfile {
    RegisteredFacetProfile::new(source.clone(), "same-name-is-not-authority", family).unwrap()
}
fn fixture(families: &[FacetFamily], fields: &str) -> BoundAttributeDatatype {
    let representation = families[0].representation();
    let (kind, name, item, base) = match representation {
        ValueRepresentation::Scalar(_) => (DatatypeKind::Scalar, "scalar", "", ""),
        ValueRepresentation::List(_) => (
            DatatypeKind::List,
            "list",
            "{type @name=item @kind=scalar}",
            "@base=item",
        ),
        ValueRepresentation::Nodes => (DatatypeKind::Node, "node", "", ""),
    };
    let edge = if kind == DatatypeKind::List {
        "list-base"
    } else {
        "base"
    };
    let mut declarations = format!("{item} {{type @name=t0 @kind={name} {base}}}");
    for index in 1..families.len() {
        declarations.push_str(&format!(" {{type @name=t{index} @{edge}=t{}}}", index - 1));
    }
    let target = families.len() - 1;
    let src = source(&format!("{{schema @name=test @namespace=urn:test | {{types | {declarations}}} {{attributes | {{attribute @name=value @type=t{target} {fields} }} }} }}"));
    let declaration = node(&src, "attribute");
    let (mut host, sources) = types_fixture_source(src);
    let mut registrations = DatatypeImplementations::default();
    let offset = usize::from(kind == DatatypeKind::List);
    if offset == 1 {
        registrations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                T::String.representation(),
            ))
            .unwrap();
        registrations
            .select_preparation(
                sources[0].clone(),
                PreparationBinding::Ready(
                    cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::String)
                        .unwrap(),
                ),
            )
            .unwrap();
    }
    let root = &sources[offset];
    let mut entry = implementation(root, kind, representation);
    if kind == DatatypeKind::List {
        entry.tokenizer = TokenizerBinding::Ready(RegisteredTokenizer::whitespace());
    }
    registrations.register(entry).unwrap();
    let prep = match representation {
        ValueRepresentation::Scalar(_) => {
            Some(cem_ql::datatype_shipped::lexical_preparation(root.clone(), T::String).unwrap())
        }
        ValueRepresentation::List(item) => {
            Some(RegisteredLexicalPreparation::list_items(root.clone(), "list", item).unwrap())
        }
        ValueRepresentation::Nodes => None,
    };
    if let Some(prep) = prep {
        registrations
            .select_preparation(root.clone(), PreparationBinding::Ready(prep))
            .unwrap();
    }
    for (index, family) in families.iter().enumerate() {
        let source = &sources[offset + index];
        let p = profile(source, *family);
        registrations
            .select_facets(
                source.clone(),
                if index == 0 {
                    FacetProfileBinding::Ready(p)
                } else {
                    FacetProfileBinding::CheckedReplacement(p)
                },
            )
            .unwrap();
    }
    let result = compile(&mut host, &sources, &registrations);
    assert!(result.is_ready(), "{:?}", result.issues);
    let selected = sources.last().unwrap();
    let slot = match declaration.node() {
        CemAstNode::Element { attributes, .. } => attributes.iter().filter_map(|id| SchemaDeclarationNode::new(declaration.document().clone(), *id)).find(|n| matches!(n.node(), CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "type")).unwrap(),
        _ => unreachable!(),
    };
    host.bind_literal_attribute_type(slot, selected.declaration().clone())
        .unwrap();
    bind_attribute_datatype(
        declaration,
        &result,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap()
}
fn context(bound: &BoundAttributeFacets) -> FacetContext<'_> {
    // Empty maps are promoted for the lifetime of this test context.
    static EMPTY: std::sync::LazyLock<BTreeMap<String, String>> =
        std::sync::LazyLock::new(BTreeMap::new);
    static DIAGNOSTICS: std::sync::LazyLock<
        BTreeMap<String, cem_ml::schema::document_model::DiagnosticBehavior>,
    > = std::sync::LazyLock::new(BTreeMap::new);
    FacetContext {
        element_name: "sample",
        source: bound.binding().declaration.node(),
        diagnostic_behaviors: &DIAGNOSTICS,
        attribute_values: &EMPTY,
    }
}
fn input(text: &str) -> PreparationInput {
    PreparationInput {
        lexical: LexicalInput::new(Arc::from(text), Default::default()),
        candidate: vec![],
        fallback: Default::default(),
    }
}
fn run(bound: &BoundAttributeFacets, text: &str) -> AttributeValidation {
    let control = OperationControl::default();
    bound.validate_lexical(
        &input(text),
        context(bound),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    )
}

#[test]
fn checked_facets_preserve_transitive_registration_identity_and_base_admission() {
    let bound = fixture(
        &[
            FacetFamily::Shipped(T::Uri),
            FacetFamily::Shipped(T::String),
            FacetFamily::Shipped(T::String),
        ],
        "",
    );
    let profiles = bound.datatype.facet_profiles();
    assert_eq!(profiles.len(), 3);
    assert_eq!(profiles[0].family(), FacetFamily::Shipped(T::Uri));
    assert_ne!(
        profiles[0].source().declaration().identity(),
        profiles[1].source().declaration().identity()
    );
    assert!(Arc::ptr_eq(
        profiles[0].source().declaration().document(),
        bound.declaration.document()
    ));
    assert_eq!(
        bound
            .datatype
            .facet_profile()
            .unwrap()
            .source()
            .declaration()
            .identity(),
        profiles[2].source().declaration().identity()
    );
    let facets = bound
        .compile_facets("urn:test", Default::default())
        .unwrap();
    assert_eq!(facets.profiles().len(), 3);
    assert_eq!(run(&facets, "https://example.test/a").accepted, Some(true));
    let invalid = run(&facets, "not-uri");
    assert_eq!(invalid.accepted, Some(false));
    assert_eq!(
        invalid.facets.unwrap().diagnostics.len(),
        1,
        "stop at first rejected profile"
    );
}

#[test]
fn checked_facets_preserve_specialized_fields_and_reject_incompatible_local_models() {
    for (ty, field, good, bad) in [
        (
            T::Uri,
            "@uriHosts=allowed.example",
            "https://allowed.example/a",
            "https://other.example/a",
        ),
        (T::Path, "@pathExtensions=cem", "./a.cem", "./a.txt"),
    ] {
        let bound = fixture(&[FacetFamily::Shipped(ty), FacetFamily::Shipped(ty)], field)
            .compile_facets("urn:test", Default::default())
            .unwrap();
        assert_eq!(run(&bound, good).accepted, Some(true));
        assert_eq!(run(&bound, bad).accepted, Some(false));
        let weaker = fixture(
            &[FacetFamily::Shipped(ty), FacetFamily::Shipped(T::String)],
            field,
        );
        assert!(matches!(
            weaker.compile_facets("urn:test", Default::default()),
            Err(AttributeFacetBindingError::Contract(
                FacetCompilationError::Invalid(_)
            ))
        ));
    }
    let narrower = fixture(
        &[
            FacetFamily::Shipped(T::String),
            FacetFamily::Shipped(T::Uri),
        ],
        "",
    )
    .compile_facets("urn:test", Default::default())
    .unwrap();
    assert_eq!(run(&narrower, "not-uri").accepted, Some(false));
    assert_eq!(run(&narrower, "https://example.test").accepted, Some(true));
}

#[test]
fn checked_facet_selection_requires_exact_source_available_base_and_representation() {
    for mode in [
        "root",
        "missing",
        "unavailable",
        "representation",
        "ordinary",
    ] {
        let (mut host, sources) =
            types_fixture("{type @name=base @kind=scalar} {type @name=child @base=base}");
        let mut registrations = DatatypeImplementations::default();
        registrations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                T::String.representation(),
            ))
            .unwrap();
        if mode != "root" && mode != "missing" {
            registrations
                .select_facets(
                    sources[0].clone(),
                    if mode == "unavailable" {
                        FacetProfileBinding::Unavailable
                    } else {
                        FacetProfileBinding::Ready(profile(
                            &sources[0],
                            FacetFamily::Shipped(T::String),
                        ))
                    },
                )
                .unwrap();
        }
        let target = if mode == "root" {
            &sources[0]
        } else {
            &sources[1]
        };
        let p = profile(
            target,
            FacetFamily::Shipped(if mode == "representation" {
                T::Integer
            } else {
                T::String
            }),
        );
        registrations
            .select_facets(
                target.clone(),
                if mode == "ordinary" {
                    FacetProfileBinding::Ready(p)
                } else {
                    FacetProfileBinding::CheckedReplacement(p)
                },
            )
            .unwrap();
        let result = compile(&mut host, &sources, &registrations);
        let expected = match mode {
            "root" => "facet-profile-replacement-requires-base",
            "missing" => "facet-profile-replacement-base-unavailable",
            "unavailable" => "datatype-facet-profile-unavailable",
            "representation" => "facet-profile-representation-incompatible",
            _ => "facet-profile-base-replacement-unsupported",
        };
        assert!(!result.is_ready());
        assert!(
            result.issues.iter().any(|i| i.code == expected),
            "{mode}: {:?}",
            result.issues
        );
        assert_eq!(
            DatatypeImplementations::default().select_facets(
                sources[1].clone(),
                FacetProfileBinding::CheckedReplacement(profile(
                    &sources[0],
                    FacetFamily::Shipped(T::String)
                ))
            ),
            Err("unrelated-facet-profile-source")
        );
    }
}

#[test]
fn checked_facet_chains_preflight_aggregate_model_and_input_limits_and_control() {
    let families = [FacetFamily::Shipped(T::String); 3];
    let bound = fixture(&families, "@minLength=2");
    let facets = bound
        .compile_facets("urn:test", Default::default())
        .unwrap();
    let bytes = cem_ml::schema::document_model::AttributeValueContract {
        model: bound.local_constraints().clone(),
        ..Default::default()
    }
    .accounted_bytes()
    .max(1);
    assert!(matches!(
        bound.compile_facets(
            "urn:test",
            FacetLimits {
                max_model_bytes: bytes * 3 - 1,
                ..Default::default()
            }
        ),
        Err(AttributeFacetBindingError::Contract(
            FacetCompilationError::Limit
        ))
    ));
    for limits in [
        FacetLimits {
            max_model_bytes: bytes * 3 - 1,
            ..Default::default()
        },
        FacetLimits {
            max_input_bytes: 5,
            ..Default::default()
        },
        FacetLimits {
            max_diagnostics: 0,
            ..Default::default()
        },
    ] {
        let text = if limits.max_diagnostics == 0 {
            "x"
        } else {
            "ok"
        };
        assert!(matches!(
            facets.contract().validate_with_check(
                FacetInput::Scalar(text),
                context(&facets),
                limits,
                &mut || Ok::<_, ()>(())
            ),
            Err(FacetExecutionError::Limit)
        ));
    }
    let mut checks = 0;
    assert!(matches!(
        facets.contract().validate_with_check(
            FacetInput::Scalar("ok"),
            context(&facets),
            Default::default(),
            &mut || {
                checks += 1;
                if checks == 5 {
                    Err("cancelled")
                } else {
                    Ok(())
                }
            }
        ),
        Err(FacetExecutionError::Interrupted("cancelled"))
    ));
    assert!(matches!(
        AttributeFacetContract::compile_profiles(
            "urn:test",
            bound.local_constraints(),
            &[],
            Default::default()
        ),
        Err(FacetCompilationError::Invalid(_))
    ));
    assert!(matches!(
        AttributeFacetContract::compile_profiles(
            "urn:test",
            bound.local_constraints(),
            &[families[0], FacetFamily::Nodes],
            Default::default()
        ),
        Err(FacetCompilationError::Invalid(_))
    ));
}

#[test]
fn checked_facet_replacement_handles_prepared_lists_and_native_nodes() {
    let lists = fixture(
        &[
            FacetFamily::List(ScalarRepresentation::String),
            FacetFamily::Shipped(T::NameList),
        ],
        "@itemCount=2",
    )
    .compile_facets("urn:test", Default::default())
    .unwrap();
    assert_eq!(run(&lists, "a b").accepted, Some(true));
    assert_eq!(run(&lists, "a").accepted, Some(false));
    // List profiles use only the prepared count; they never add an item tokenizer.
    assert_eq!(run(&lists, "? !").accepted, Some(true));
    let nodes = fixture(&[FacetFamily::Nodes; 2], "@minItems=0 @maxItems=1")
        .compile_facets("urn:test", Default::default())
        .unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for count in [0, 1, 2] {
        let report = nodes.validate_nodes(
            &ValidationInput {
                value: vec![native(&nodes.binding().declaration); count],
                candidate: vec![],
                fallback: Default::default(),
            },
            context(&nodes),
            &runtime,
            Default::default(),
        );
        assert_eq!(report.accepted, Some(count <= 1), "{report:?}");
    }
}

#[test]
fn checked_facet_profiles_are_revalidated_after_sealed_preparation() {
    let facets = fixture(
        &[
            FacetFamily::Shipped(T::Uri),
            FacetFamily::Shipped(T::String),
        ],
        "",
    )
    .compile_facets("urn:test", Default::default())
    .unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for (text, accepted) in [("https://example.test", true), ("not-uri", false)] {
        let invocation =
            AttributePreparationInvocation::new(&facets, input(text), &runtime, Default::default())
                .unwrap();
        let evidence = invocation.prepare().evidence.unwrap();
        for _ in 0..2 {
            let report =
                facets.validate_pretyped(&invocation, Some(&evidence), context(&facets), 1_048_576);
            assert_eq!(report.accepted, Some(accepted));
        }
    }
}
