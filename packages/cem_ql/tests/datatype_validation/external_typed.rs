use super::*;
use cem_ml::schema::{
    datatype_contracts::DatatypeCompilation,
    document_model::{attribute_facets::*, shipped_datatypes::ShippedDatatype as T},
};
use cem_ql::{
    attribute_datatypes::bind_attribute_datatype, attribute_validation::*, datatype_facets::*,
    external_typed::*,
};

fn compile(
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
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
fn select(
    registrations: &mut DatatypeImplementations,
    source: &DatatypeSource,
    family: FacetFamily,
) {
    registrations
        .select_facets(
            source.clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(source.clone(), "explicit-typed", family).unwrap(),
            ),
        )
        .unwrap();
}
fn fixture(
    rep: ScalarRepresentation,
    list: bool,
    fields: &str,
) -> Result<BoundAttributeFacets, AttributeFacetBindingError> {
    fixture_with(rep, list, fields, "", |_, _, _, _| {})
}
fn fixture_with(
    rep: ScalarRepresentation,
    list: bool,
    fields: &str,
    datatype_fields: &str,
    configure: impl FnOnce(
        &[DatatypeSource],
        &mut DatatypeImplementations,
        &mut DatatypeValidationRegistry,
        &mut Option<(SchemaDeclarationNode, SchemaDeclarationNode)>,
    ),
) -> Result<BoundAttributeFacets, AttributeFacetBindingError> {
    let declarations = if list {
        "{type @name=item @kind=scalar} {type @name=sample @kind=list @base=item}"
    } else {
        "{type @name=sample @kind=scalar}"
    };
    let declarations =
        declarations.replace("@name=sample", &format!("@name=sample {datatype_fields}"));
    let src = source(&format!("{{schema @name=test @namespace=urn:test | {{types | {declarations}}} {{attributes | {{attribute @name=value @type=sample {fields} }} }} }}"));
    let declaration = node(&src, "attribute");
    let (mut host, sources) = types_fixture_source(src);
    let mut registrations = DatatypeImplementations::default();
    let mut validations = DatatypeValidationRegistry::default();
    let mut rule = None;
    configure(&sources, &mut registrations, &mut validations, &mut rule);
    for (index, source) in sources.iter().enumerate() {
        let family = if list && index == 1 {
            FacetFamily::TypedList(rep)
        } else {
            FacetFamily::TypedScalar(rep)
        };
        let mut registration = implementation(
            source,
            if list && index == 1 {
                DatatypeKind::List
            } else {
                DatatypeKind::Scalar
            },
            family.representation(),
        );
        if index == sources.len() - 1 {
            registration.validator = rule.clone();
        }
        registrations.register(registration).unwrap();
        select(&mut registrations, source, family);
    }
    let control = OperationControl::default();
    let result = cem_ql::datatype_compilation::compile_datatypes_with_runtime(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &registrations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime(&control),
        Default::default(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let slot = match declaration.node() {
        CemAstNode::Element { attributes, .. } => attributes.iter().filter_map(|id| SchemaDeclarationNode::new(declaration.document().clone(), *id)).find(|n| matches!(n.node(), CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "type")).unwrap(), _ => unreachable!(),
    };
    host.bind_literal_attribute_type(slot, sources.last().unwrap().declaration().clone())
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
    .compile_facets("urn:test", Default::default())
}
fn context(bound: &BoundAttributeFacets) -> FacetContext<'_> {
    static EMPTY: std::sync::LazyLock<BTreeMap<String, String>> =
        std::sync::LazyLock::new(BTreeMap::new);
    static DIAGS: std::sync::LazyLock<
        BTreeMap<String, cem_ml::schema::document_model::DiagnosticBehavior>,
    > = std::sync::LazyLock::new(BTreeMap::new);
    FacetContext {
        element_name: "sample",
        source: bound.binding().declaration.node(),
        diagnostic_behaviors: &DIAGS,
        attribute_values: &EMPTY,
    }
}
fn runtime(control: &OperationControl) -> ValidationRuntime<'_> {
    ValidationRuntime {
        control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    }
}
fn consume(
    bound: &BoundAttributeFacets,
    value: Option<&ExternalTypedValue>,
    runtime: &ValidationRuntime<'_>,
    limits: AttributeValidationLimits,
) -> AttributeValidation {
    bound.validate_external(
        ExternalTypedInput {
            value,
            candidate: vec![],
            fallback: Default::default(),
        },
        context(bound),
        runtime,
        limits,
    )
}
#[test]
fn external_typed_scalars_require_explicit_profiles_and_preserve_exact_values() {
    for (rep, atom) in [
        (
            ScalarRepresentation::String,
            AtomValue::String("  not-uri  ".into()),
        ),
        (ScalarRepresentation::Integer, AtomValue::Integer(3)),
        (ScalarRepresentation::Boolean, AtomValue::Boolean(true)),
        (
            ScalarRepresentation::Decimal,
            AtomValue::Decimal("003.00".into()),
        ),
        (ScalarRepresentation::Double, AtomValue::Double(-0.0)),
        (
            ScalarRepresentation::AnyUri,
            AtomValue::AnyUri("../relative".into()),
        ),
    ] {
        let bound = fixture(rep, false, "").unwrap();
        let producer = ExternalTypedProducer::new(&bound, "host:computed").unwrap();
        let control = OperationControl::default();
        let runtime = runtime(&control);
        let values = producer
            .produce(
                Some(vec![Item::Atomic(atom.clone())]),
                &runtime,
                Default::default(),
            )
            .unwrap();
        assert_eq!(values.values()[0].atom(), Some(atom));
        assert_eq!(
            consume(&bound, Some(&values), &runtime, Default::default()).accepted,
            Some(true)
        );
        assert!(consume(&bound, None, &runtime, Default::default())
            .accepted
            .is_none());
        assert!(producer
            .produce(None, &runtime, Default::default())
            .is_err());
        assert!(producer
            .produce(Some(vec![]), &runtime, Default::default())
            .is_err());
        assert!(ExternalTypedProducer::new(&bound, " ").is_err());
    }
    let lexical =
        super::attribute_consumer::fixture(FacetFamily::Shipped(T::String), "", false, true, false);
    assert!(ExternalTypedProducer::new(&lexical, "no-implicit-grant").is_err());
}
#[test]
fn typed_only_profiles_reject_every_lexical_field_and_literal_defaults() {
    for fields in [
        "@default=hello",
        "@values=a",
        "@pattern=a",
        "@whiteSpace=collapse",
        "@length=2",
        "@minLength=1",
        "@maxLength=3",
        "@minInclusive=1",
        "@maxInclusive=5",
        "@minExclusive=0",
        "@maxExclusive=9",
        "@totalDigits=2",
        "@fractionDigits=1",
        "@stringPrefixes=a",
        "@pathExtensions=cem",
        "@uriHosts=example.test",
        "@mediaTypes=text/plain",
        "@itemCount=1",
    ] {
        assert!(
            matches!(
                fixture(ScalarRepresentation::String, false, fields),
                Err(AttributeFacetBindingError::Contract(
                    FacetCompilationError::Invalid(_)
                ))
            ),
            "{fields}"
        );
    }
    // Fail closed for every model field, including future additions, rather than
    // treating absent pattern/length as evidence of lexical independence.
    let src = source("{schema | {attributes | {attribute @name=x}}}");
    let decl = node(&src, "attribute");
    let mut model =
        cem_ml::schema::document_model::compile_attribute_model(decl.document(), decl.node_id())
            .unwrap();
    model
        .uri_query_parameter_values
        .insert("q".into(), ["a".into()].into());
    assert!(AttributeFacetContract::compile(
        "urn:test",
        &model,
        FacetFamily::TypedList(ScalarRepresentation::String),
        Default::default()
    )
    .is_err());
}
#[test]
fn typed_only_datatype_graphs_reject_lexical_ancestors_items_and_preparers() {
    for mode in ["preparer", "tokenizer", "item", "base", "mixed"] {
        let list = mode == "tokenizer" || mode == "item";
        let (mut host, sources) = types_fixture(if list {
            "{type @name=base @kind=scalar} {type @name=child @kind=list @base=base}"
        } else {
            "{type @name=base @kind=scalar} {type @name=child @base=base}"
        });
        let mut regs = DatatypeImplementations::default();
        let base = implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::String.representation(),
        );
        regs.register(base).unwrap();
        if mode != "base" && mode != "item" {
            select(
                &mut regs,
                &sources[0],
                if mode == "mixed" {
                    FacetFamily::Shipped(T::String)
                } else {
                    FacetFamily::TypedScalar(ScalarRepresentation::String)
                },
            );
        }
        if mode == "preparer" {
            regs.select_preparation(
                sources[0].clone(),
                cem_ql::datatype_preparation::PreparationBinding::Ready(
                    cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::String)
                        .unwrap(),
                ),
            )
            .unwrap();
        }
        if list {
            let mut entry = implementation(
                &sources[1],
                DatatypeKind::List,
                ValueRepresentation::List(ScalarRepresentation::String),
            );
            if mode == "tokenizer" {
                entry.tokenizer = TokenizerBinding::Ready(
                    cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace(),
                );
            }
            regs.register(entry).unwrap();
            select(
                &mut regs,
                &sources[1],
                FacetFamily::TypedList(ScalarRepresentation::String),
            );
        } else if mode == "base" || mode == "mixed" {
            regs.select_facets(
                sources[1].clone(),
                FacetProfileBinding::CheckedReplacement(
                    RegisteredFacetProfile::new(
                        sources[1].clone(),
                        "typed-child",
                        FacetFamily::TypedScalar(ScalarRepresentation::String),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        }
        assert!(!compile(&mut host, &sources, &regs).is_ready(), "{mode}");
    }
    let (mut host, sources) = types_fixture("{type @name=sample @kind=lexical}");
    let mut regs = DatatypeImplementations::default();
    regs.register(implementation(
        &sources[0],
        DatatypeKind::Lexical,
        T::String.representation(),
    ))
    .unwrap();
    select(
        &mut regs,
        &sources[0],
        FacetFamily::TypedScalar(ScalarRepresentation::String),
    );
    assert!(!compile(&mut host, &sources, &regs).is_ready());
}
#[test]
fn external_lists_preserve_order_duplicates_and_distinguish_empty_from_absent() {
    let bound = fixture(
        ScalarRepresentation::Integer,
        true,
        "@minItems=0 @maxItems=2",
    )
    .unwrap();
    let producer = ExternalTypedProducer::new(&bound, "items").unwrap();
    let control = OperationControl::default();
    let runtime = runtime(&control);
    for count in [0, 2, 3] {
        let value = producer
            .produce(
                Some(vec![Item::Atomic(AtomValue::Integer(7)); count]),
                &runtime,
                Default::default(),
            )
            .unwrap();
        assert_eq!(value.values().len(), count);
        assert_eq!(
            consume(&bound, Some(&value), &runtime, Default::default()).accepted,
            Some(count <= 2)
        );
    }
    assert!(consume(&bound, None, &runtime, Default::default())
        .accepted
        .is_none());
    let scalar = fixture(ScalarRepresentation::Integer, false, "").unwrap();
    let value = producer
        .produce(
            Some(vec![Item::Atomic(AtomValue::Integer(7))]),
            &runtime,
            Default::default(),
        )
        .unwrap();
    assert!(consume(&scalar, Some(&value), &runtime, Default::default())
        .accepted
        .is_none());
}
#[derive(Debug)]
struct Impostor;
impl cem_ql::eval::QueryItemView for Impostor {
    fn identity(&self) -> String {
        "impostor".into()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.typed-atomic"
    }
    fn kind(&self) -> cem_ql::eval::QueryItemViewKind {
        cem_ql::eval::QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<AtomValue> {
        panic!("untrusted atomic views must not be evaluated")
    }
}
#[test]
fn external_values_reject_spoofed_views_malformed_values_limits_and_revocation() {
    let bound = fixture(ScalarRepresentation::Decimal, false, "").unwrap();
    let producer = ExternalTypedProducer::new(&bound, "external").unwrap();
    let control = OperationControl::default();
    let runtime = runtime(&control);
    for value in [
        Item::native(Impostor),
        native(&bound.binding().declaration),
        Item::Atomic(AtomValue::Decimal("not-a-number".into())),
        Item::Atomic(AtomValue::String("3".into())),
    ] {
        assert!(producer
            .produce(Some(vec![value]), &runtime, Default::default())
            .is_err());
    }
    let values = || Some(vec![Item::Atomic(AtomValue::Decimal("3.25".into()))]);
    let mut limits = cem_ql::datatype_preparation::PreparationLimits::default();
    limits.max_output_values = 0;
    assert!(producer.produce(values(), &runtime, limits).is_err());
    limits = Default::default();
    limits.max_lexical_bytes = 2;
    assert!(producer.produce(values(), &runtime, limits).is_err());
    let value = producer
        .produce(values(), &runtime, Default::default())
        .unwrap();
    let mut limits = AttributeValidationLimits::default();
    limits.preparation.validation.max_input_values = 0;
    assert!(consume(&bound, Some(&value), &runtime, limits)
        .accepted
        .is_none());
    let clone = producer.clone();
    producer.close();
    assert!(clone
        .produce(values(), &runtime, Default::default())
        .is_err());
    assert!(consume(&bound, Some(&value), &runtime, Default::default())
        .accepted
        .is_none());
    let fresh = ExternalTypedProducer::new(&bound, "external").unwrap();
    assert!(fresh
        .produce(values(), &runtime, Default::default())
        .is_ok());
    control.cancel_root(None, None).unwrap();
    assert!(fresh
        .produce(values(), &runtime, Default::default())
        .is_err());
}

#[derive(Debug)]
struct FreshRule {
    calls: Arc<AtomicUsize>,
    mode: Arc<AtomicUsize>,
    revoke: Arc<std::sync::Mutex<Option<ExternalTypedProducer>>>,
}
impl NativeDatatypeValidator for FreshRule {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(producer) = self.revoke.lock().unwrap().take() {
            producer.close();
        }
        let mode = self.mode.load(Ordering::SeqCst);
        if mode == 2 {
            return RuleExecution::Pending(vec![]);
        }
        if mode == 3 {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        RuleExecution::Complete(query(if mode == 1 {
            "{accepted: false, diagnostics: ()}"
        } else {
            "{accepted: true, diagnostics: ()}"
        }))
    }
}
#[test]
fn external_consumption_reruns_rules_requires_candidates_and_checks_revocation_after_callbacks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mode = Arc::new(AtomicUsize::new(0));
    let revoke = Arc::new(std::sync::Mutex::new(None));
    let bound = fixture_with(
        ScalarRepresentation::String,
        false,
        "",
        "",
        |_, _, validations, selected| {
            let src = source(&declaration(true));
            let rule = node(&src, "behavior");
            let contract = DatatypeBehaviorContract::compile(&src, &rule, signature(true)).unwrap();
            validations
                .register_native(
                    "urn:test:validate",
                    contract,
                    adapter(),
                    None,
                    FreshRule {
                        calls: calls.clone(),
                        mode: mode.clone(),
                        revoke: revoke.clone(),
                    },
                )
                .unwrap();
            *selected = Some((src.schema, rule));
        },
    )
    .unwrap();
    let producer = ExternalTypedProducer::new(&bound, "external").unwrap();
    let control = OperationControl::default();
    let runtime = runtime(&control);
    let value = producer
        .produce(
            Some(vec![Item::Atomic(AtomValue::String("x".into()))]),
            &runtime,
            Default::default(),
        )
        .unwrap();
    assert!(consume(&bound, Some(&value), &runtime, Default::default())
        .accepted
        .is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let validate = |limits| {
        bound.validate_external(
            ExternalTypedInput {
                value: Some(&value),
                candidate: vec![native(&bound.binding().declaration)],
                fallback: Default::default(),
            },
            context(&bound),
            &runtime,
            limits,
        )
    };
    let mut limits = AttributeValidationLimits::default();
    limits.preparation.validation.max_rules = 0;
    assert!(validate(limits).accepted.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for (state, expected) in [
        (0, Some(true)),
        (0, Some(true)),
        (1, Some(false)),
        (2, None),
    ] {
        mode.store(state, Ordering::SeqCst);
        assert_eq!(validate(Default::default()).accepted, expected);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    mode.store(0, Ordering::SeqCst);
    *revoke.lock().unwrap() = Some(producer.clone());
    assert!(matches!(
        validate(Default::default()).stopped,
        Some(AttributeValidationStop::ExternalTyped(
            ExternalTypedError::Expired
        ))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 5);
    assert!(validate(Default::default()).accepted.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 5);
}

#[derive(Debug)]
struct Interpret;
impl cem_ql::datatype_enumeration::NativeConstantInterpreter for Interpret {
    fn interpret(
        &self,
        call: cem_ql::datatype_enumeration::ConstantCall<'_>,
    ) -> cem_ql::datatype_enumeration::ConstantExecution {
        cem_ql::datatype_enumeration::ConstantExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String(call.token.text().into()))],
            diagnostics: vec![],
        }
    }
}
#[derive(Debug)]
struct Equal;
impl cem_ql::datatype_enumeration::NativeScalarEquality for Equal {
    fn compare(
        &self,
        call: cem_ql::datatype_enumeration::EqualityCall<'_>,
    ) -> cem_ql::datatype_enumeration::EqualityExecution {
        cem_ql::datatype_enumeration::EqualityExecution::Complete {
            equal: call.left == call.right,
            diagnostics: vec![],
        }
    }
}
#[test]
fn external_values_keep_typed_enumerations_and_their_shared_comparison_limits() {
    use cem_ql::datatype_enumeration::*;
    let bound = fixture_with(
        ScalarRepresentation::String,
        false,
        "",
        "@values=\"a b\"",
        |sources, regs, _, _| {
            regs.select_equality(
                sources[0].clone(),
                EqualityBinding::Ready(
                    RegisteredScalarEquality::new(
                        sources[0].clone(),
                        "eq",
                        ScalarRepresentation::String,
                        Equal,
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
            regs.select_constant_interpreter(
                sources[0].clone(),
                ConstantBinding::Ready(
                    RegisteredConstantInterpreter::new(
                        sources[0].clone(),
                        "constants",
                        ScalarRepresentation::String,
                        Interpret,
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        },
    )
    .unwrap();
    let producer = ExternalTypedProducer::new(&bound, "vocabulary").unwrap();
    let control = OperationControl::default();
    let runtime = runtime(&control);
    for (text, accepted) in [("a", true), ("b", true), ("c", false)] {
        let value = producer
            .produce(
                Some(vec![Item::Atomic(AtomValue::String(text.into()))]),
                &runtime,
                Default::default(),
            )
            .unwrap();
        assert_eq!(
            consume(&bound, Some(&value), &runtime, Default::default()).accepted,
            Some(accepted)
        );
        let mut limits = AttributeValidationLimits::default();
        limits.preparation.validation.max_comparisons = 0;
        assert!(consume(&bound, Some(&value), &runtime, limits)
            .accepted
            .is_none());
    }
}

#[test]
fn typed_only_whole_list_derivatives_keep_item_admission_bounds_and_original_profiles() {
    let (mut host, sources) = types_fixture("{type @name=item @kind=scalar} {type @name=base @kind=list @base=item @min-items=1 @max-items=3} {type @name=derived @list-base=base @max-items=2} {type @name=leaf @list-base=derived}");
    let mut regs = DatatypeImplementations::default();
    regs.register(implementation(
        &sources[0],
        DatatypeKind::Scalar,
        T::String.representation(),
    ))
    .unwrap();
    select(
        &mut regs,
        &sources[0],
        FacetFamily::TypedScalar(ScalarRepresentation::String),
    );
    regs.register(implementation(
        &sources[1],
        DatatypeKind::List,
        ValueRepresentation::List(ScalarRepresentation::String),
    ))
    .unwrap();
    select(
        &mut regs,
        &sources[1],
        FacetFamily::TypedList(ScalarRepresentation::String),
    );
    let profile = RegisteredFacetProfile::new(
        sources[2].clone(),
        "derived",
        FacetFamily::TypedList(ScalarRepresentation::String),
    )
    .unwrap();
    assert_eq!(
        DatatypeImplementations::default().select_facets(
            sources[3].clone(),
            FacetProfileBinding::CheckedReplacement(profile.clone())
        ),
        Err("unrelated-facet-profile-source")
    );
    regs.select_facets(
        sources[2].clone(),
        FacetProfileBinding::CheckedReplacement(profile),
    )
    .unwrap();
    let result = compile(&mut host, &sources, &regs);
    assert!(result.is_ready());
    let base = compiled(&result, &sources[1]);
    let leaf = compiled(&result, &sources[3]);
    assert!(leaf.admits_external_typed());
    assert!(Arc::ptr_eq(base.item().unwrap(), leaf.item().unwrap()));
    assert_eq!(leaf.facet_profiles().len(), 2);
    assert_eq!(
        leaf.facet_profiles()[0].source().declaration().identity(),
        sources[1].declaration().identity()
    );
    for count in 0..4 {
        assert_eq!(
            validate_descriptor(
                leaf,
                vec![Item::Atomic(AtomValue::String("x".into())); count]
            )
            .accepted,
            Some(count == 1 || count == 2)
        );
    }
}

#[test]
fn external_wide_integers_retain_immutable_owners_and_share_metadata_budget() {
    use cem_ml::{
        schema::datatype_contracts::LexicalInput,
        source::ByteRange,
        source_map::{FrameSpan, SourceMapFrame, TransformKind},
    };
    use cem_ql::datatype_preparation::{PreparationBinding, PreparationInput};
    let (mut host, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let mut regs = DatatypeImplementations::default();
    regs.register(implementation(
        &sources[0],
        DatatypeKind::Scalar,
        T::Integer.representation(),
    ))
    .unwrap();
    regs.select_preparation(
        sources[0].clone(),
        PreparationBinding::Ready(
            cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::Integer).unwrap(),
        ),
    )
    .unwrap();
    let result = compile(&mut host, &sources, &regs);
    assert!(result.is_ready());
    let control = OperationControl::default();
    let runtime = runtime(&control);
    let mut source = cem_ml::source_map::SourceMapStack::default();
    source.frames.push(SourceMapFrame {
        source_id: SourceId(99),
        span: FrameSpan::Single(ByteRange { start: 0, len: 30 }),
        transform: TransformKind::TemplateTransform {
            function: "x".repeat(600_000),
        },
    });
    let prepared = compiled(&result, &sources[0]).prepare_lexical(
        &PreparationInput {
            lexical: LexicalInput::new(Arc::from("123456789012345678901234567890"), source.clone()),
            candidate: vec![],
            fallback: Default::default(),
        },
        &runtime,
        Default::default(),
    );
    let item = prepared.value.unwrap().pop().unwrap();
    let bound = fixture(ScalarRepresentation::Integer, true, "").unwrap();
    let producer = ExternalTypedProducer::new(&bound, "wide").unwrap();
    let accepted = producer
        .produce(Some(vec![item.clone()]), &runtime, Default::default())
        .unwrap();
    assert_eq!(accepted.values()[0].identity(), item.identity());
    assert_eq!(accepted.values()[0].source_map(), Some(source));
    assert_eq!(
        consume(&bound, Some(&accepted), &runtime, Default::default()).accepted,
        Some(true)
    );
    assert!(matches!(
        producer.produce(Some(vec![item.clone(), item]), &runtime, Default::default()),
        Err(ExternalTypedError::Limit)
    ));
}
