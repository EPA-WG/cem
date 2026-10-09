use super::*;
use cem_ml::schema::datatype_contracts::LexicalInput;
use cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype as T;
use cem_ql::datatype_preparation::*;

fn request(text: &str) -> PreparationInput {
    PreparationInput {
        lexical: LexicalInput::new(Arc::from(text), Default::default()),
        candidate: vec![],
        fallback: DiagnosticAttribution {
            uri: Some("original.cem".into()),
            ..Default::default()
        },
    }
}
fn run(d: &ExecutableDatatype, text: &str) -> DatatypePreparation {
    let control = OperationControl::default();
    d.prepare_lexical(
        &request(text),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    )
}
#[test]
fn lexical_preparation_inherits_original_identity_without_converter_fallback() {
    let (mut host, sources) =
        types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::Integer.representation(),
        ))
        .unwrap();
    let compile = |host: &mut _, implementations: &_| {
        compile_datatypes(
            sources[0].declaration().document().clone(),
            &[sources[1].clone()],
            host,
            implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
    };
    let absent = compile(&mut host, &implementations);
    assert!(absent.is_ready());
    assert!(matches!(
        run(compiled(&absent, &sources[1]), "003").stopped,
        Some(PreparationStop::NoPreparation)
    ));
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::Integer)
                    .unwrap(),
            ),
        )
        .unwrap();
    let ready = compile(&mut host, &implementations);
    assert!(ready.is_ready(), "{:?}", ready.issues);
    let result = run(compiled(&ready, &sources[1]), "003");
    assert_eq!(result.accepted, Some(true));
    assert_eq!(&*result.input.lexical.text, "003");
    assert_eq!(result.value.unwrap()[0].atom(), Some(AtomValue::Integer(3)));
    assert_eq!(
        result.preparer.unwrap().source.declaration().identity(),
        sources[0].declaration().identity()
    );
    assert!(compiled(&ready, &sources[1]).converter().is_none());
}
#[test]
fn lexical_preparation_preserves_boolean_admission_and_wide_integers() {
    for (ty, cases) in [
        (
            T::Boolean,
            vec![
                ("", true),
                (" true ", true),
                ("false", true),
                ("1", false),
                ("0", false),
            ],
        ),
        (
            T::Integer,
            vec![
                ("+003", true),
                ("9223372036854775808", true),
                ("1.0", false),
            ],
        ),
    ] {
        let (mut host, sources) = types_fixture("{type @name=sample @kind=scalar}");
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                ty.representation(),
            ))
            .unwrap();
        implementations
            .select_preparation(
                sources[0].clone(),
                PreparationBinding::Ready(
                    cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), ty).unwrap(),
                ),
            )
            .unwrap();
        let result = compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            &mut host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(result.is_ready());
        for (text, accepted) in cases {
            let p = run(compiled(&result, &sources[0]), text);
            assert_eq!(p.accepted, Some(accepted), "{ty:?} {text}: {p:?}");
            assert_eq!(&*p.input.lexical.text, text);
        }
    }
}
#[test]
fn list_preparation_uses_original_item_capability_and_retains_token_spans() {
    let (mut host, sources) = types_fixture(
        "{type @name=item @kind=scalar} {type @name=list @kind=list @base=item @max-items=3}",
    );
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::Integer.representation(),
        ))
        .unwrap();
    let mut list = implementation(
        &sources[1],
        DatatypeKind::List,
        ValueRepresentation::List(ScalarRepresentation::Integer),
    );
    list.tokenizer = TokenizerBinding::Ready(
        cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace(),
    );
    implementations.register(list).unwrap();
    implementations
        .select_preparation(
            sources[1].clone(),
            PreparationBinding::Ready(
                RegisteredLexicalPreparation::list_items(
                    sources[1].clone(),
                    "test:list",
                    ScalarRepresentation::Integer,
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
    assert!(!compile(&mut host, &implementations).is_ready());
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::Integer)
                    .unwrap(),
            ),
        )
        .unwrap();
    let compiled_types = compile(&mut host, &implementations);
    assert!(compiled_types.is_ready(), "{:?}", compiled_types.issues);
    let d = compiled(&compiled_types, &sources[1]);
    let result = run(d, " 003\t4 003 ");
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.token_spans, vec![1..4, 5..6, 7..10]);
    assert_eq!(
        result
            .value
            .unwrap()
            .iter()
            .map(Item::atom)
            .collect::<Vec<_>>(),
        vec![
            Some(AtomValue::Integer(3)),
            Some(AtomValue::Integer(4)),
            Some(AtomValue::Integer(3))
        ]
    );
    assert_eq!(run(d, "").accepted, Some(true));
    assert_eq!(run(d, "1 2 3 4").accepted, Some(false));
    let rejected = run(d, "1 bad 3");
    assert_eq!(rejected.accepted, Some(false));
    assert!(rejected.value.is_none());
}

#[derive(Debug)]
struct MockPreparation {
    calls: Arc<AtomicUsize>,
    result: PreparationExecution,
    cancel: bool,
}
impl NativeLexicalPreparer for MockPreparation {
    fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        assert_eq!(call.original.text, call.lexical.text);
        if self.cancel {
            call.runtime.control.abort_signal().abort();
        }
        self.result.clone()
    }
}
#[derive(Debug)]
struct ForbiddenConverter;
impl cem_ql::datatype_conversion::NativeDatatypeConverter for ForbiddenConverter {
    fn convert(
        &self,
        _: cem_ql::datatype_conversion::ConversionCall<'_>,
    ) -> cem_ql::datatype_conversion::ConversionExecution {
        panic!("validation preparation must never invoke conversion")
    }
}
fn mocked(
    result: PreparationExecution,
    required: bool,
    cancel: bool,
) -> (ExecutableDatatype, Arc<AtomicUsize>) {
    use cem_ql::datatype_conversion::*;
    let (mut host, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let mut implementations = DatatypeImplementations::default();
    let output = ValueRepresentation::Scalar(ScalarRepresentation::String);
    implementations
        .register(implementation(&sources[0], DatatypeKind::Scalar, output))
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                RegisteredLexicalPreparation::new(
                    sources[0].clone(),
                    "test",
                    PreparationSignature {
                        kind: DatatypeKind::Scalar,
                        output,
                        candidate: if required {
                            CandidateRequirement::Required
                        } else {
                            CandidateRequirement::Optional
                        },
                    },
                    MockPreparation {
                        calls: calls.clone(),
                        result,
                        cancel,
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_converter(
            sources[0].clone(),
            ConverterBinding::Ready(
                RegisteredDatatypeConverter::new(
                    sources[0].clone(),
                    "never",
                    ConversionSignature {
                        kind: DatatypeKind::Scalar,
                        input: ConversionRepresentation::Lexical,
                        output,
                        candidate: CandidateRequirement::Optional,
                    },
                    ForbiddenConverter,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    (compiled(&result, &sources[0]).clone(), calls)
}
fn prepared() -> PreparationExecution {
    PreparationExecution::Prepared {
        value: vec![Item::Atomic(AtomValue::String("value".into()))],
        diagnostics: vec![],
    }
}
#[test]
fn lexical_preparation_checks_completion_representation_and_preserves_diagnostics() {
    for (execution, accepted) in [
        (prepared(), Some(true)),
        (PreparationExecution::Rejected(vec![]), Some(false)),
        (PreparationExecution::Pending(vec![]), None),
        (PreparationExecution::Unavailable(vec![]), None),
        (PreparationExecution::Failed(vec![]), None),
        (PreparationExecution::Limit("test"), None),
        (
            PreparationExecution::Prepared {
                value: vec![],
                diagnostics: vec![],
            },
            None,
        ),
        (
            PreparationExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::Boolean(true))],
                diagnostics: vec![],
            },
            None,
        ),
    ] {
        let (d, calls) = mocked(execution, false, false);
        assert_eq!(validate_descriptor(&d, input().value).accepted, Some(true));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "typed validation bypasses preparation"
        );
        let result = run(&d, "original");
        assert_eq!(result.accepted, accepted, "{result:?}");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        if accepted.is_none() {
            assert!(result.value.is_none());
            assert!(result.stopped.is_some());
        }
    }
    let diagnostic = cem_ml::diagnostics::Diagnostic {
        code: "example".into(),
        ..Default::default()
    };
    let (d, _) = mocked(
        PreparationExecution::Prepared {
            value: input().value,
            diagnostics: vec![diagnostic],
        },
        false,
        false,
    );
    let result = run(&d, "original");
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.diagnostics[0].uri.as_deref(), Some("original.cem"));
    assert_eq!(
        result.diagnostics[0].source_map,
        Some(result.input.lexical.source)
    );
}
#[test]
fn lexical_preparation_checks_candidate_control_and_shared_budgets() {
    let (d, calls) = mocked(prepared(), true, false);
    assert!(matches!(
        run(&d, "x").stopped,
        Some(PreparationStop::MissingCandidate)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let (d, calls) = mocked(prepared(), false, false);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let mut invalid = request("x");
    invalid.candidate = input().value;
    assert!(matches!(
        d.prepare_lexical(&invalid, &runtime, Default::default())
            .stopped,
        Some(PreparationStop::InvalidInput("candidate"))
    ));
    for limits in [
        PreparationLimits {
            max_lexical_bytes: 0,
            ..Default::default()
        },
        PreparationLimits {
            max_preparations: 0,
            ..Default::default()
        },
        PreparationLimits {
            max_output_values: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            d.prepare_lexical(&request("x"), &runtime, limits).stopped,
            Some(PreparationStop::Limit(_))
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = d.prepare_lexical(
        &request("x"),
        &runtime,
        PreparationLimits {
            validation: ValidationLimits {
                max_input_values: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(
        result.accepted, None,
        "preparation and validation share input visits"
    );
    assert!(result.validation.unwrap().stopped.is_some());
    control.abort_signal().abort();
    let before = calls.load(Ordering::SeqCst);
    assert!(matches!(
        d.prepare_lexical(&request("x"), &runtime, Default::default())
            .stopped,
        Some(PreparationStop::Control(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), before);
    let (d, _) = mocked(prepared(), false, true);
    let canceled = run(&d, "x");
    assert!(matches!(
        canceled.stopped,
        Some(PreparationStop::Control(_))
    ));
    assert!(canceled.value.is_none());
}
#[test]
fn lexical_preparation_selection_checks_owners_availability_and_inheritance() {
    let (mut host, sources) =
        types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::String.representation(),
        ))
        .unwrap();
    let registration =
        cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::String).unwrap();
    assert_eq!(
        implementations.select_preparation(
            sources[1].clone(),
            PreparationBinding::Ready(registration.clone())
        ),
        Err("unrelated-preparation-source")
    );
    let mut unavailable = implementations.clone();
    unavailable
        .select_preparation(sources[0].clone(), PreparationBinding::Unavailable)
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
    let pending = compile(&mut host, &unavailable);
    assert!(!pending.is_ready());
    assert!(pending
        .issues
        .iter()
        .any(|i| i.code == "datatype-preparation-unavailable"));
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(registration.clone()),
        )
        .unwrap();
    assert_eq!(
        implementations
            .select_preparation(sources[0].clone(), PreparationBinding::Ready(registration)),
        Err("duplicate-preparation-selection")
    );
    implementations
        .select_preparation(
            sources[1].clone(),
            PreparationBinding::Ready(
                cem_ql::datatype_shipped::lexical_preparation(sources[1].clone(), T::String)
                    .unwrap(),
            ),
        )
        .unwrap();
    let replaced = compile(&mut host, &implementations);
    assert!(!replaced.is_ready());
    assert!(replaced
        .issues
        .iter()
        .any(|i| i.code == "preparation-base-replacement-unsupported"));
}

#[test]
fn lexical_preparation_still_runs_rules_with_shared_diagnostic_budget() {
    let profile = source(&declaration(false));
    let (mut host, sources) = types_fixture_source(profile.clone());
    let mut implementations = DatatypeImplementations::default();
    let mut entry = implementation(
        &sources[0],
        DatatypeKind::Scalar,
        T::String.representation(),
    );
    entry.validator = Some((profile.schema.clone(), node(&profile, "behavior")));
    implementations.register(entry).unwrap();
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                RegisteredLexicalPreparation::new(
                    sources[0].clone(),
                    "test",
                    PreparationSignature {
                        kind: DatatypeKind::Scalar,
                        output: T::String.representation(),
                        candidate: CandidateRequirement::Optional,
                    },
                    MockPreparation {
                        calls: Arc::new(AtomicUsize::new(0)),
                        cancel: false,
                        result: PreparationExecution::Prepared {
                            value: input().value,
                            diagnostics: vec![cem_ml::diagnostics::Diagnostic {
                                code: "prepared".into(),
                                ..Default::default()
                            }],
                        },
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let mut validations = DatatypeValidationRegistry::default();
    validations.register_native("urn:test:validate",contract(&profile,false),adapter(),None,Outcome(RuleExecution::Complete(query("{ accepted: false, diagnostics: ({code: \"rule\", severity: \"warning\", message: \"rejected\"}) }")))).unwrap();
    let compiled_types = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(compiled_types.is_ready(), "{:?}", compiled_types.issues);
    let d = compiled(&compiled_types, &sources[0]);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for (budget, accepted) in [(0, None), (1, None), (2, Some(false))] {
        let result = d.prepare_lexical(
            &request("text"),
            &runtime,
            PreparationLimits {
                validation: ValidationLimits {
                    max_diagnostics: budget,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert_eq!(result.accepted, accepted, "budget {budget}: {result:?}");
        assert!(result.diagnostics.len() <= budget);
    }
}

#[test]
fn lexical_preparation_rejects_native_and_incompatible_signatures() {
    let (mut host, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let mock = || MockPreparation {
        calls: Arc::new(AtomicUsize::new(0)),
        result: prepared(),
        cancel: false,
    };
    for (kind, output) in [
        (DatatypeKind::Node, ValueRepresentation::Nodes),
        (
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
        ),
        (DatatypeKind::Scalar, ValueRepresentation::Nodes),
    ] {
        assert!(RegisteredLexicalPreparation::new(
            sources[0].clone(),
            "bad",
            PreparationSignature {
                kind,
                output,
                candidate: CandidateRequirement::Optional
            },
            mock()
        )
        .is_err());
    }
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::String.representation(),
        ))
        .unwrap();
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::Integer)
                    .unwrap(),
            ),
        )
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(!result.is_ready());
    assert_eq!(result.issues[0].code, "preparation-output-incompatible");
}
