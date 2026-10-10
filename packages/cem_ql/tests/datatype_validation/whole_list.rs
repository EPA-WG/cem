use super::*;
use cem_ml::schema::datatype_contracts::{CompiledDatatypeContract, DatatypeCompilation};
use cem_ml::schema::{
    datatype_contracts::{LexicalInput, RegisteredTokenizer},
    document_model::{attribute_facets::FacetFamily, shipped_datatypes::ShippedDatatype as T},
};
use cem_ml::{
    schema::declaration_references::SchemaDeclarationHost,
    value::reference_resolution::ReferenceResolutionHost,
};
use cem_ql::api::{StandaloneExpressionBinding, StandaloneExpressionContext};
use cem_ql::{
    datatype_conversion::ConverterBinding,
    datatype_facets::{FacetProfileBinding, RegisteredFacetProfile},
    datatype_preparation::{PreparationBinding, PreparationInput, RegisteredLexicalPreparation},
    datatype_serialization::ListSerializerBinding,
};

fn registrations(sources: &[DatatypeSource]) -> DatatypeImplementations {
    registrations_with(sources, |_, _| {})
}
fn registrations_with(
    sources: &[DatatypeSource],
    mut configure: impl FnMut(usize, &mut DatatypeImplementation),
) -> DatatypeImplementations {
    let mut result = DatatypeImplementations::default();
    let mut scalar = implementation(
        &sources[0],
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::String),
    );
    configure(0, &mut scalar);
    result.register(scalar).unwrap();
    result
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::String)
                    .unwrap(),
            ),
        )
        .unwrap();
    let mut list = implementation(
        &sources[1],
        DatatypeKind::List,
        ValueRepresentation::List(ScalarRepresentation::String),
    );
    list.tokenizer = TokenizerBinding::Ready(RegisteredTokenizer::whitespace());
    configure(1, &mut list);
    result.register(list).unwrap();
    result
        .select_preparation(
            sources[1].clone(),
            PreparationBinding::Ready(
                RegisteredLexicalPreparation::list_items(
                    sources[1].clone(),
                    "items",
                    ScalarRepresentation::String,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    result
        .select_facets(
            sources[1].clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(
                    sources[1].clone(),
                    "list-facets",
                    FacetFamily::List(ScalarRepresentation::String),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    result
        .select_list_serializer(
            sources[1].clone(),
            ListSerializerBinding::Ready(
                cem_ql::datatype_shipped::list_serializer(sources[1].clone(), T::NameList).unwrap(),
            ),
        )
        .unwrap();
    result
        .select_converter(
            sources[1].clone(),
            ConverterBinding::Ready(
                cem_ql::datatype_shipped::converter(sources[1].clone(), T::NameList).unwrap(),
            ),
        )
        .unwrap();
    result
}

const CHAIN: &str = "{type @name=item @kind=scalar} {type @name=names @kind=list @base=item} {type @name=derived @list-base=names}";

#[test]
fn whole_list_native_selection_requires_exactly_one_original_type() {
    for selection in ["empty", "multiple", "wrong-target"] {
        let (mut host, sources) =
            types_fixture(&CHAIN.replace("@list-base=names", "@list-base={#selected}"));
        let values = match selection {
            "empty" => vec![],
            "multiple" => vec![
                native(sources[1].declaration()),
                native(sources[0].declaration()),
            ],
            _ => vec![native(sources[0].scope())],
        };
        let scope = host
            .scope(&host.source_reference(sources[0].declaration().clone()))
            .unwrap();
        host.set_context(
            scope,
            Some(StandaloneExpressionContext::default().with_binding(
                "selected",
                StandaloneExpressionBinding::any(ItemStream::from_items(values)),
            )),
        );
        let result = compile(&mut host, &sources, &registrations(&sources));
        assert!(!result.is_ready(), "{selection}");
        assert!(!result
            .contracts
            .iter()
            .any(|d| d.source().declaration().identity() == sources[2].declaration().identity()));
    }
}

#[test]
fn whole_list_incomplete_dependencies_and_cycles_never_produce_a_ready_descriptor() {
    for fields in [
        "@list-base=missing",
        "@list-base=derived",
        "@list-base={#missing}",
    ] {
        let (mut host, sources) = types_fixture(&CHAIN.replace("@list-base=names", fields));
        let result = compile(&mut host, &sources, &registrations(&sources));
        assert!(!result.is_ready(), "{fields}");
        assert!(!result
            .contracts
            .iter()
            .any(|d| d.source().declaration().identity() == sources[2].declaration().identity()));
    }
    let (mut host, sources) = types_fixture(CHAIN);
    let implementations = registrations_with(&sources, |index, entry| {
        if index == 1 {
            entry.tokenizer = TokenizerBinding::Unavailable;
        }
    });
    let result = compile(&mut host, &sources, &implementations);
    assert!(result
        .issues
        .iter()
        .any(|i| i.code == "datatype-tokenizer-unavailable"));
    assert!(!result.is_ready());
    let mut limits = ReferenceTraversalLimits::schema_defaults().unwrap();
    limits.max_work = 1;
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[2].clone()],
        &mut host,
        &registrations(&sources),
        &Default::default(),
        limits,
    );
    assert!(!result.is_ready());
}

#[test]
fn whole_list_cannot_replace_lexical_admission_or_representation() {
    for (selection, expected) in [
        ("tokenizer", "tokenizer-base-replacement-unsupported"),
        ("preparer", "preparation-base-replacement-unsupported"),
        ("facets", "facet-profile-base-replacement-unsupported"),
        ("representation", "incompatible-datatype-base"),
        ("serializer", "datatype-list-serializer-unavailable"),
        ("converter", "datatype-converter-unavailable"),
    ] {
        let (mut host, sources) = types_fixture(CHAIN);
        let mut implementations = registrations(&sources);
        let derived = sources[2].clone();
        match selection {
            "tokenizer" | "representation" => {
                let mut entry = implementation(
                    &derived,
                    DatatypeKind::List,
                    ValueRepresentation::List(if selection == "representation" {
                        ScalarRepresentation::Integer
                    } else {
                        ScalarRepresentation::String
                    }),
                );
                if selection == "tokenizer" {
                    entry.tokenizer = TokenizerBinding::Ready(RegisteredTokenizer::whitespace());
                }
                implementations.register(entry).unwrap();
            }
            "preparer" => implementations
                .select_preparation(
                    derived.clone(),
                    PreparationBinding::Ready(
                        RegisteredLexicalPreparation::list_items(
                            derived,
                            "replacement",
                            ScalarRepresentation::String,
                        )
                        .unwrap(),
                    ),
                )
                .unwrap(),
            "facets" => implementations
                .select_facets(
                    derived.clone(),
                    FacetProfileBinding::Ready(
                        RegisteredFacetProfile::new(
                            derived,
                            "replacement",
                            FacetFamily::List(ScalarRepresentation::String),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap(),
            "serializer" => implementations
                .select_list_serializer(derived, ListSerializerBinding::Unavailable)
                .unwrap(),
            "converter" => implementations
                .select_converter(derived, ConverterBinding::Unavailable)
                .unwrap(),
            _ => unreachable!(),
        }
        let result = compile(&mut host, &sources, &implementations);
        assert!(!result.is_ready());
        assert!(
            result.issues.iter().any(|i| i.code == expected),
            "{selection}: {:?}",
            result.issues.iter().map(|i| i.code).collect::<Vec<_>>()
        );
    }
}

#[derive(Debug)]
struct CountRule {
    calls: Arc<AtomicUsize>,
    accepted: Arc<std::sync::atomic::AtomicBool>,
}
impl NativeDatatypeValidator for CountRule {
    fn validate(&self, _: ValidationCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        RuleExecution::Complete(query(&format!(
            "{{accepted: {}, diagnostics: ()}}",
            self.accepted.load(Ordering::SeqCst)
        )))
    }
}
fn counted_rule(
    list: bool,
    calls: &Arc<AtomicUsize>,
    accepted: &Arc<std::sync::atomic::AtomicBool>,
    registry: &mut DatatypeValidationRegistry,
) -> (SchemaDeclarationNode, SchemaDeclarationNode) {
    let text = if list {
        declaration(false).replace(
            "@source=value @required=true @cardinality=one",
            "@source=value @required=true @cardinality=zero-or-more",
        )
    } else {
        declaration(false)
    };
    let profile = source(&text);
    let rule = node(&profile, "behavior");
    let mut sig = signature(false);
    if list {
        sig.kind = DatatypeKind::List;
        sig.value = ValueRepresentation::List(ScalarRepresentation::String);
    }
    registry
        .register_native(
            "urn:test:validate",
            DatatypeBehaviorContract::compile(&profile, &rule, sig).unwrap(),
            adapter(),
            None,
            CountRule {
                calls: calls.clone(),
                accepted: accepted.clone(),
            },
        )
        .unwrap();
    (profile.schema, rule)
}

#[test]
fn whole_list_sealed_attribute_consumption_retains_facets_tokens_and_runs_all_rules_once() {
    use cem_ml::schema::document_model::attribute_facets::FacetContext;
    use cem_ql::{
        attribute_datatypes::bind_attribute_datatype,
        preparation_evidence::AttributePreparationInvocation,
    };
    let profile = source(&format!("{{schema @name=test @namespace=urn:test | {{types | {CHAIN} {{type @name=leaf @list-base=derived @max-items=3}}}} {{attributes | {{attribute @name=value @type=leaf @itemCount=2}}}}}}"));
    let decl = node(&profile, "attribute");
    let (mut host, sources) = types_fixture_source(profile);
    let calls = Arc::new(AtomicUsize::new(0));
    let accepted = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut validations = DatatypeValidationRegistry::default();
    let item_rule = counted_rule(false, &calls, &accepted, &mut validations);
    let list_rule = counted_rule(true, &calls, &accepted, &mut validations);
    let mut implementations = registrations_with(&sources, |index, entry| {
        entry.validator = Some(if index == 0 {
            item_rule.clone()
        } else {
            list_rule.clone()
        });
    });
    for derived in &sources[2..] {
        let mut entry = implementation(
            derived,
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
        );
        entry.validator = Some(list_rule.clone());
        implementations.register(entry).unwrap();
    }
    let compilation = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[3].clone()],
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(
        compilation.is_ready(),
        "{:?}",
        compilation
            .issues
            .iter()
            .map(|i| i.code)
            .collect::<Vec<_>>()
    );
    let slot = match decl.node() { CemAstNode::Element { attributes, .. } => attributes.iter().filter_map(|id| SchemaDeclarationNode::new(decl.document().clone(), *id)).find(|n| matches!(n.node(), CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "type")).unwrap(), _ => unreachable!() };
    host.bind_literal_attribute_type(slot, sources[3].declaration().clone())
        .unwrap();
    let bound = bind_attribute_datatype(
        decl.clone(),
        &compilation,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap()
    .compile_facets("urn:test", Default::default())
    .unwrap();
    assert_eq!(
        bound.profile().source().declaration().identity(),
        sources[1].declaration().identity()
    );
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for (text, expected, spans) in [
        (" β β\t", true, vec![1..3, 4..6]),
        (" β\t", false, vec![1..3]),
    ] {
        calls.store(0, Ordering::SeqCst);
        let invocation = AttributePreparationInvocation::new(
            &bound,
            PreparationInput {
                lexical: LexicalInput::new(Arc::from(text), Default::default()),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        )
        .unwrap();
        let result = invocation.prepare();
        let evidence = result.evidence.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(evidence.lexical().text.as_ref(), text);
        assert_eq!(evidence.token_spans(), spans);
        let consume = || {
            bound.validate_pretyped(
                &invocation,
                Some(&evidence),
                FacetContext {
                    element_name: "sample",
                    source: decl.node(),
                    diagnostic_behaviors: &Default::default(),
                    attribute_values: &Default::default(),
                },
                1_048_576,
            )
        };
        assert_eq!(consume().accepted, Some(expected));
        assert_eq!(calls.load(Ordering::SeqCst), 3 + spans.len());
        accepted.store(false, Ordering::SeqCst);
        assert_eq!(consume().accepted, Some(false));
        assert_eq!(calls.load(Ordering::SeqCst), 2 * (3 + spans.len()));
        accepted.store(true, Ordering::SeqCst);
    }
    calls.store(0, Ordering::SeqCst);
    let mut limits = ValidationLimits::default();
    limits.max_rules = 4;
    let result = compiled(&compilation, &sources[3]).validate(
        &ValidationInput {
            value: query("(\"a\", \"a\")").items,
            candidate: vec![],
            fallback: Default::default(),
        },
        &runtime,
        limits,
    );
    assert_eq!(result.accepted, None);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn whole_list_override_rebinds_only_authorized_edges_and_recompiles_descendants() {
    use cem_ml::schema::document_model::SchemaDocumentModel;
    use cem_ql::{datatype_names::*, datatype_overrides::*};
    for native_edge in [false, true] {
        for rebind in [false, true] {
            let (mut host, sources) = types_fixture(&format!(
                "{{type @name=item @kind=scalar}} {{type @name=names @kind=list @base=item}} \
                 {{type @name=replacement @kind=list @base=item @max-items=1}} \
                 {{type @name=derived @list-base={}}} {{type @name=leaf @list-base=derived}}",
                if native_edge { "{#original}" } else { "names" }
            ));
            let names = Arc::new(
                DatatypeNameCatalog::collect(
                    &[DatatypeNameScope {
                        scope: sources[0].scope().clone(),
                        namespace: Some("urn:test".into()),
                        imports: vec![],
                        declarations: sources
                            .iter()
                            .map(|source| DatatypeNameDeclaration {
                                source: source.clone(),
                                aliases: Default::default(),
                            })
                            .collect(),
                    }],
                    &host,
                    Default::default(),
                )
                .unwrap(),
            );
            host.install_datatype_names(names.clone()).unwrap();
            let scope = host
                .scope(&host.source_reference(sources[0].declaration().clone()))
                .unwrap();
            host.set_context(
                scope,
                Some(StandaloneExpressionContext::default().with_binding(
                    "original",
                    StandaloneExpressionBinding::any(ItemStream::once(native(
                        sources[1].declaration(),
                    ))),
                )),
            );
            let mut implementations = registrations(&sources);
            implementations
                .register(implementation(
                    &sources[2],
                    DatatypeKind::List,
                    ValueRepresentation::List(ScalarRepresentation::String),
                ))
                .unwrap();
            let original = compile(&mut host, &sources, &implementations);
            assert!(original.is_ready());
            let old = Arc::new(original);
            let mut registry = DatatypeOverrideRegistry::new(
                names,
                SchemaDocumentModel {
                    datatype_compilation: Some(old.clone()),
                    ..Default::default()
                },
            )
            .unwrap();
            let request = DatatypeOverrideRequest {
                scope: sources[1].scope().clone(),
                namespace: "urn:test".into(),
                name: "names".into(),
                expected: sources[1].clone(),
                replacement: sources[2].clone(),
                rebind: if rebind {
                    vec![sources[3].attribute("list-base").unwrap().clone()]
                } else {
                    vec![]
                },
            };
            let grant = registry.authorize(&request).unwrap();
            let control = OperationControl::default();
            let runtime = ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            };
            let prepared = registry
                .prepare(
                    &[request],
                    &[grant],
                    &host,
                    DatatypeOverrideOptions {
                        implementations: &implementations,
                        validations: &Default::default(),
                        runtime: &runtime,
                        limits: ReferenceTraversalLimits::schema_defaults().unwrap(),
                        preparation: Default::default(),
                    },
                )
                .unwrap();
            registry.activate(&mut host, prepared).unwrap();
            let new = registry.model().datatype_compilation.as_ref().unwrap();
            let selected = compiled(new, &sources[3])
                .base()
                .unwrap()
                .source()
                .declaration()
                .identity();
            assert_eq!(
                selected,
                sources[if rebind { 2 } else { 1 }].declaration().identity()
            );
            assert_eq!(
                validate_descriptor(compiled(new, &sources[4]), query("(\"a\", \"a\")").items)
                    .accepted,
                Some(!rebind)
            );
            assert_eq!(
                validate_descriptor(compiled(&old, &sources[4]), query("(\"a\", \"a\")").items)
                    .accepted,
                Some(true)
            );
            let edge = new
                .dependency_sites
                .iter()
                .find(|site| {
                    site.attribute.identity()
                        == sources[3].attribute("list-base").unwrap().identity()
                })
                .unwrap();
            assert_eq!(
                edge.role,
                cem_ml::schema::datatype_registry::DatatypeDependencyRole::InheritedList
            );
        }
    }
}

#[test]
fn whole_list_keeps_item_enumeration_restrictions_and_explicit_output_capabilities() {
    use cem_ql::datatype_enumeration::{ConstantBinding, EqualityBinding};
    let (mut host, sources) = types_fixture(&CHAIN.replace(
        "@name=item @kind=scalar",
        "@name=item @kind=scalar @values='a b'",
    ));
    let mut implementations = registrations(&sources);
    implementations
        .select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(
                cem_ql::datatype_shipped::constant_interpreter(sources[0].clone(), T::String)
                    .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_equality(
            sources[0].clone(),
            EqualityBinding::Ready(super::shipped_conversion::equality(&sources[0], T::String)),
        )
        .unwrap();
    // Output capabilities may be explicitly replaced without replacing lexical ingress.
    implementations
        .select_list_serializer(
            sources[2].clone(),
            ListSerializerBinding::Ready(
                cem_ql::datatype_shipped::list_serializer(sources[2].clone(), T::NameList).unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_converter(
            sources[2].clone(),
            ConverterBinding::Ready(
                cem_ql::datatype_shipped::converter(sources[2].clone(), T::NameList).unwrap(),
            ),
        )
        .unwrap();
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let compilation = cem_ql::datatype_compilation::compile_datatypes_with_runtime(
        sources[0].declaration().document().clone(),
        &[sources[2].clone()],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        Default::default(),
    );
    assert!(
        compilation.is_ready(),
        "{:?}",
        compilation
            .issues
            .iter()
            .map(|i| i.code)
            .collect::<Vec<_>>()
    );
    let list = compiled(&compilation, &sources[2]);
    assert_eq!(
        list.list_serializer()
            .unwrap()
            .identity()
            .source
            .declaration()
            .identity(),
        sources[2].declaration().identity()
    );
    assert_eq!(
        list.converter()
            .unwrap()
            .identity()
            .source
            .declaration()
            .identity(),
        sources[2].declaration().identity()
    );
    for (text, expected) in [("a a b", true), ("a c", false)] {
        let prepared = list.prepare_lexical(
            &PreparationInput {
                lexical: LexicalInput::new(Arc::from(text), Default::default()),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(prepared.accepted, Some(expected));
        let validation = prepared.validation.unwrap();
        assert_eq!(
            validation.enumerations.len(),
            text.split_whitespace().count()
        );
        let converted = list.convert(
            &cem_ql::datatype_conversion::ConversionInput {
                value: cem_ql::datatype_conversion::ConversionValue::Lexical(LexicalInput::new(
                    Arc::from(text),
                    Default::default(),
                )),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(converted.accepted, Some(expected));
    }
}
fn compile(
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
    sources: &[DatatypeSource],
    implementations: &DatatypeImplementations,
) -> DatatypeCompilation {
    compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources.last().unwrap().clone()],
        host,
        implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
}

#[test]
fn whole_list_inherits_item_bounds_and_source_bound_capabilities() {
    for base in ["names", "{#names}"] {
        let (mut host, sources) = types_fixture(&format!(
            "{{type @name=item @kind=scalar}} {{type @name=names @kind=list @base=item @min-items=1 @max-items=4}} \
             {{type @name=pair @kind=list @list-base={base} @min-items=2 @max-items=3}} \
             {{type @name=leaf @list-base=pair @min-items=0 @max-items=9}}"));
        let implementations = registrations(&sources);
        let scope = host
            .scope(&host.source_reference(sources[1].declaration().clone()))
            .unwrap();
        host.set_context(
            scope,
            Some(StandaloneExpressionContext::default().with_binding(
                "names",
                StandaloneExpressionBinding::any(ItemStream::once(native(
                    sources[1].declaration(),
                ))),
            )),
        );
        let result = compile(&mut host, &sources, &implementations);
        assert!(
            result.is_ready(),
            "{:?} {:?}",
            result.issues,
            result.reference_issues
        );
        let root = compiled(&result, &sources[1]);
        let leaf = compiled(&result, &sources[3]);
        assert_eq!(leaf.bounds(), ItemBounds::new(2, Some(3)).unwrap());
        assert!(Arc::ptr_eq(root.item().unwrap(), leaf.item().unwrap()));
        assert_eq!(
            leaf.base().unwrap().source().declaration().identity(),
            sources[2].declaration().identity()
        );
        assert_eq!(
            leaf.tokenizer().unwrap().identity(),
            root.tokenizer().unwrap().identity()
        );
        assert_eq!(
            leaf.preparation()
                .unwrap()
                .identity()
                .source
                .declaration()
                .identity(),
            sources[1].declaration().identity()
        );
        assert_eq!(
            leaf.list_serializer()
                .unwrap()
                .identity()
                .source
                .declaration()
                .identity(),
            sources[1].declaration().identity()
        );
        assert!(leaf.converter().is_some());
        assert!(leaf.facet_profile().is_some());
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let prepared = leaf.prepare_lexical(
            &PreparationInput {
                lexical: LexicalInput::new(Arc::from(" β a β\t"), Default::default()),
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            Default::default(),
        );
        assert_eq!(prepared.accepted, Some(true), "{prepared:?}");
        let request = ValidationInput {
            value: prepared.value.unwrap(),
            candidate: vec![],
            fallback: Default::default(),
        };
        assert_eq!(
            leaf.serialize_list(&request, &runtime, Default::default())
                .text
                .as_deref(),
            Some("β a β")
        );
        let rejection = validate_descriptor(leaf, query("(\"a\", \"b\", \"c\", \"d\")").items);
        assert_eq!(rejection.accepted, Some(false));
        assert_eq!(
            rejection.cardinality[0].source.identity(),
            sources[2].declaration().identity()
        );
    }
}

#[test]
fn whole_list_rejects_implicit_inheritance_nested_items_and_incompatible_bases() {
    for (fields, expected) in [
        ("@base=names", "list-base-required"),
        ("@kind=list @base=names", "incompatible-list-item"),
        ("@list-base=item", "incompatible-list-base"),
        (
            "@list-base=names @min-items=5",
            "empty-cardinality-intersection",
        ),
        ("@list-base=names {#extra}", "unsupported-datatype-child"),
    ] {
        let (mut host, sources) = types_fixture(&format!(
            "{{type @name=item @kind=scalar}} {{type @name=names @kind=list @base=item @max-items=4}} {{type @name=derived {fields}}}"));
        let mut implementations = registrations(&sources);
        if fields == "@kind=list @base=names" {
            implementations
                .register(implementation(
                    &sources[2],
                    DatatypeKind::List,
                    ValueRepresentation::List(ScalarRepresentation::String),
                ))
                .unwrap();
        }
        let result = compile(&mut host, &sources, &implementations);
        assert!(!result.is_ready());
        assert!(
            result.issues.iter().any(|i| i.code == expected),
            "{fields}: {:?}",
            result.issues
        );
    }
}
