use super::*;
use cem_ql::datatype_compilation::compile_datatypes_with_runtime;
use cem_ql::datatype_enumeration::*;
use cem_ql::datatype_validation::ValidationStopReason;

#[derive(Debug)]
struct Interpret;
impl NativeConstantInterpreter for Interpret {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        ConstantExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String(call.token.text().into()))],
            diagnostics: vec![],
        }
    }
}
#[derive(Debug)]
struct Equal {
    numeric: bool,
}
impl NativeScalarEquality for Equal {
    fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution {
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        let equal = if self.numeric {
            let (Item::Atomic(AtomValue::String(a)), Item::Atomic(AtomValue::String(b))) =
                (call.left, call.right)
            else {
                panic!()
            };
            a.parse::<u64>().unwrap() == b.parse::<u64>().unwrap()
        } else {
            call.left == call.right
        };
        EqualityExecution::Complete {
            equal,
            diagnostics: vec![],
        }
    }
}
fn capabilities(
    implementations: &mut DatatypeImplementations,
    source: &DatatypeSource,
    numeric: bool,
) {
    implementations
        .select_equality(
            source.clone(),
            EqualityBinding::Ready(
                RegisteredScalarEquality::new(
                    source.clone(),
                    "eq",
                    ScalarRepresentation::String,
                    Equal { numeric },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_constant_interpreter(
            source.clone(),
            ConstantBinding::Ready(
                RegisteredConstantInterpreter::new(
                    source.clone(),
                    "constant",
                    ScalarRepresentation::String,
                    Interpret,
                )
                .unwrap(),
            ),
        )
        .unwrap();
}
fn compile(
    host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
    roots: &[DatatypeSource],
    implementations: &DatatypeImplementations,
) -> cem_ml::schema::datatype_contracts::DatatypeCompilation {
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    compile_datatypes_with_runtime(
        roots[0].declaration().document().clone(),
        roots,
        host,
        implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        Default::default(),
    )
}
#[test]
fn enumeration_uses_registered_equality_and_retains_token_sources() {
    for numeric in [false, true] {
        let (mut host, sources) =
            types_fixture("{type @name=sample @kind=scalar @values=\" 3  5 \"}");
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                ValueRepresentation::Scalar(ScalarRepresentation::String),
            ))
            .unwrap();
        capabilities(&mut implementations, &sources[0], numeric);
        let result = compile(&mut host, &sources, &implementations);
        assert!(result.is_ready(), "{:?}", result.issues);
        let descriptor = compiled(&result, &sources[0]);
        let vocabulary = &descriptor.enumerations()[0];
        assert_eq!(vocabulary.constants()[0].token.text(), "3");
        assert_eq!(vocabulary.constants()[0].token.span, 1..2);
        assert_eq!(
            vocabulary.constants()[0].token.source.identity(),
            sources[0].attribute("values").unwrap().identity()
        );
        assert_eq!(
            validate_descriptor(
                descriptor,
                vec![Item::Atomic(AtomValue::String("003".into()))]
            )
            .accepted,
            Some(numeric)
        );
    }
}
#[test]
fn enumeration_preparation_requires_explicit_lifecycle_runtime() {
    let (mut host, sources) = types_fixture("{type @name=sample @kind=scalar @values=3}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    capabilities(&mut implementations, &sources[0], true);
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(!result.is_ready());
    assert_eq!(
        result.issues[0].code,
        "datatype-constant-context-unavailable"
    );
}
#[test]
fn enumeration_derived_equality_cannot_reinterpret_base_vocabulary() {
    let (mut host, sources) = types_fixture(
        "{type @name=base @kind=scalar @values=3} {type @name=derived @base=base @values=3}",
    );
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    capabilities(&mut implementations, &sources[0], false);
    capabilities(&mut implementations, &sources[1], true);
    let result = compile(&mut host, &[sources[1].clone()], &implementations);
    assert!(result.is_ready(), "{:?}", result.issues);
    let output = validate_descriptor(
        compiled(&result, &sources[1]),
        vec![Item::Atomic(AtomValue::String("003".into()))],
    );
    assert_eq!(output.accepted, Some(false));
    assert_eq!(output.enumerations.len(), 2);
    assert!(!output.enumerations[0].accepted);
    assert!(output.enumerations[1].accepted);
}

#[derive(Debug)]
struct ConstantResult(ConstantExecution);
impl NativeConstantInterpreter for ConstantResult {
    fn interpret(&self, _: ConstantCall<'_>) -> ConstantExecution {
        self.0.clone()
    }
}
fn setup(
    text: &str,
) -> (
    cem_ql::schema_references::CemQlSchemaDeclarationHost,
    Vec<DatatypeSource>,
    DatatypeImplementations,
) {
    let (host, sources) = types_fixture(text);
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    (host, sources, implementations)
}
#[test]
fn enumeration_preparation_checks_tag_representation_and_contract() {
    use cem_ml::schema::datatype_contracts::DatatypeIssueState;
    let cases = [
        (
            ConstantExecution::Prepared {
                value: vec![],
                diagnostics: vec![],
            },
            "datatype-constant-representation",
            DatatypeIssueState::Invalid,
        ),
        (
            ConstantExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::Boolean(true))],
                diagnostics: vec![],
            },
            "datatype-constant-representation",
            DatatypeIssueState::Invalid,
        ),
        (
            ConstantExecution::Rejected(vec![]),
            "datatype-constant-rejected",
            DatatypeIssueState::Invalid,
        ),
        (
            ConstantExecution::Pending(vec![]),
            "datatype-constant-pending",
            DatatypeIssueState::Pending,
        ),
        (
            ConstantExecution::Unavailable(vec![]),
            "datatype-constant-unavailable",
            DatatypeIssueState::Pending,
        ),
        (
            ConstantExecution::Failed(vec![]),
            "datatype-constant-failed",
            DatatypeIssueState::Pending,
        ),
    ];
    for (execution, code, state) in cases {
        let (mut host, sources, mut implementations) =
            setup("{type @name=sample @kind=scalar @values=3}");
        implementations
            .select_equality(
                sources[0].clone(),
                EqualityBinding::Ready(
                    RegisteredScalarEquality::new(
                        sources[0].clone(),
                        "eq",
                        ScalarRepresentation::String,
                        Equal { numeric: false },
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        implementations
            .select_constant_interpreter(
                sources[0].clone(),
                ConstantBinding::Ready(
                    RegisteredConstantInterpreter::new(
                        sources[0].clone(),
                        "constant",
                        ScalarRepresentation::String,
                        ConstantResult(execution),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        let result = compile(&mut host, &sources, &implementations);
        assert!(!result.is_ready());
        assert_eq!(result.issues[0].code, code);
        assert_eq!(result.issues[0].state, state);
        assert_eq!(
            result.issues[0].source.identity(),
            sources[0].attribute("values").unwrap().identity()
        );
    }
    let (mut host, sources, mut implementations) =
        setup("{type @name=base @kind=scalar @values=3} {type @name=derived @base=base @values=4}");
    capabilities(&mut implementations, &sources[0], false);
    let result = compile(&mut host, &[sources[1].clone()], &implementations);
    assert!(!result.is_ready());
    assert_eq!(result.issues[0].code, "datatype-constant-contract-rejected");
}
#[test]
fn enumeration_rejects_empty_vocabularies_and_nonliteral_tokens() {
    for values in ["\"   \"", "{value}"] {
        let (mut host, sources, mut implementations) = setup(&format!(
            "{{type @name=sample @kind=scalar @values={values}}}"
        ));
        capabilities(&mut implementations, &sources[0], false);
        let result = compile(&mut host, &sources, &implementations);
        assert!(!result.is_ready());
        assert_eq!(
            result.issues[0].state,
            cem_ml::schema::datatype_contracts::DatatypeIssueState::Invalid
        );
    }
}
#[test]
fn enumeration_registration_requires_exact_source_and_preserves_unavailable_override() {
    let (mut host,sources,mut implementations)=setup("{type @name=base @kind=scalar} {type @name=derived @base=base} {type @name=leaf @base=derived @values=3}");
    let (_, foreign, _) = setup("{type @name=base @kind=scalar}");
    let r = RegisteredScalarEquality::new(
        foreign[0].clone(),
        "eq",
        ScalarRepresentation::String,
        Equal { numeric: false },
    )
    .unwrap();
    assert_eq!(
        implementations
            .select_equality(sources[0].clone(), EqualityBinding::Ready(r))
            .unwrap_err(),
        "unrelated-scalar-capability-source"
    );
    capabilities(&mut implementations, &sources[0], false);
    implementations
        .select_equality(sources[1].clone(), EqualityBinding::Unavailable)
        .unwrap();
    assert!(implementations
        .select_equality(sources[1].clone(), EqualityBinding::Unavailable)
        .is_err());
    let result = compile(&mut host, &[sources[2].clone()], &implementations);
    assert!(!result.is_ready());
    assert_eq!(result.issues[0].code, "datatype-enumeration-unavailable");
}
#[test]
fn enumeration_preparation_budgets_are_shared_across_roots() {
    for case in 0..3 {
        let (mut host, sources, mut implementations) = setup(
            "{type @name=base @kind=scalar @values=3} {type @name=derived @base=base @values=3}",
        );
        capabilities(&mut implementations, &sources[0], false);
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let mut limits = ConstantPreparationLimits::default();
        match case {
            0 => limits.max_constants = 1,
            1 => limits.max_lexical_bytes = 1,
            _ => limits.validation.max_input_values = 2,
        }
        let result = compile_datatypes_with_runtime(
            sources[0].declaration().document().clone(),
            &sources,
            &mut host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
            &runtime,
            limits,
        );
        assert!(!result.is_ready());
        assert_eq!(
            result.issues[0].code,
            [
                "datatype-constant-count-limit",
                "datatype-constant-byte-limit",
                "datatype-constant-value-limit"
            ][case]
        );
    }
}
#[test]
fn enumeration_comparisons_share_one_budget_across_list_items_and_restrictions() {
    let (mut host,sources,mut implementations)=setup("{type @name=base @kind=scalar @values=\"3 5\"} {type @name=derived @base=base @values=3} {type @name=list @kind=list @base=derived}");
    capabilities(&mut implementations, &sources[0], false);
    implementations
        .register(implementation(
            &sources[2],
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
        ))
        .unwrap();
    let result = compile(&mut host, &[sources[2].clone()], &implementations);
    assert!(result.is_ready(), "{:?}", result.issues);
    let descriptor = compiled(&result, &sources[2]);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let output = descriptor.validate(
        &ValidationInput {
            value: vec![Item::Atomic(AtomValue::String("3".into())); 2],
            ..input()
        },
        &runtime,
        ValidationLimits {
            max_comparisons: 3,
            ..Default::default()
        },
    );
    assert_eq!(output.accepted, None);
    assert_eq!(output.comparisons, 3);
    assert_eq!(output.enumerations.len(), 3);
    assert!(matches!(
        output.stopped.unwrap().reason,
        ValidationStopReason::Limit("comparisons")
    ));
}
#[derive(Debug)]
struct AbortConstant;
impl NativeConstantInterpreter for AbortConstant {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
        call.runtime.control.abort_signal().abort();
        Interpret.interpret(call)
    }
}
#[test]
fn enumeration_cancellation_after_callback_prevents_readiness() {
    let (mut host, sources, mut implementations) =
        setup("{type @name=sample @kind=scalar @values=3}");
    implementations
        .select_equality(
            sources[0].clone(),
            EqualityBinding::Ready(
                RegisteredScalarEquality::new(
                    sources[0].clone(),
                    "eq",
                    ScalarRepresentation::String,
                    Equal { numeric: false },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(
                RegisteredConstantInterpreter::new(
                    sources[0].clone(),
                    "constant",
                    ScalarRepresentation::String,
                    AbortConstant,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile(&mut host, &sources, &implementations);
    assert!(!result.is_ready());
    assert_eq!(result.issues[0].code, "datatype-constant-control");
}
#[test]
fn enumeration_never_calls_converter_during_preparation_or_validation() {
    use cem_ql::datatype_conversion::*;
    #[derive(Debug)]
    struct Never;
    impl NativeDatatypeConverter for Never {
        fn convert(&self, _: ConversionCall<'_>) -> ConversionExecution {
            panic!("implicit converter")
        }
    }
    let (mut host, sources, mut implementations) =
        setup("{type @name=sample @kind=scalar @values=3}");
    capabilities(&mut implementations, &sources[0], true);
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
                        output: ValueRepresentation::Scalar(ScalarRepresentation::String),
                        candidate: CandidateRequirement::Optional,
                    },
                    Never,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile(&mut host, &sources, &implementations);
    assert!(result.is_ready());
    assert_eq!(
        validate_descriptor(
            compiled(&result, &sources[0]),
            vec![Item::Atomic(AtomValue::String("003".into()))]
        )
        .accepted,
        Some(true)
    );
}

#[derive(Debug)]
struct EqualityResult {
    result: EqualityExecution,
    abort: bool,
}
impl NativeScalarEquality for EqualityResult {
    fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution {
        if self.abort {
            call.runtime.control.abort_signal().abort();
        }
        self.result.clone()
    }
}
fn notice() -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        code: "test.equality".into(),
        message: "comparison detail".into(),
        severity: cem_ml::diagnostics::Severity::Error,
        ..Default::default()
    }
}
#[test]
fn enumeration_comparison_outcomes_diagnostics_and_cancellation_are_distinct() {
    for (execution, expected) in [
        (
            EqualityExecution::Complete {
                equal: true,
                diagnostics: vec![notice()],
            },
            Some(true),
        ),
        (
            EqualityExecution::Complete {
                equal: false,
                diagnostics: vec![],
            },
            Some(false),
        ),
        (EqualityExecution::Pending(vec![notice()]), None),
        (EqualityExecution::Unavailable(vec![notice()]), None),
        (EqualityExecution::Failed(vec![notice()]), None),
    ] {
        let (mut host, sources, mut implementations) =
            setup("{type @name=sample @kind=scalar @values=3}");
        implementations
            .select_equality(
                sources[0].clone(),
                EqualityBinding::Ready(
                    RegisteredScalarEquality::new(
                        sources[0].clone(),
                        "eq",
                        ScalarRepresentation::String,
                        EqualityResult {
                            result: execution,
                            abort: false,
                        },
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        implementations
            .select_constant_interpreter(
                sources[0].clone(),
                ConstantBinding::Ready(
                    RegisteredConstantInterpreter::new(
                        sources[0].clone(),
                        "constant",
                        ScalarRepresentation::String,
                        Interpret,
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        let result = compile(&mut host, &sources, &implementations);
        assert!(result.is_ready());
        let output = validate_descriptor(compiled(&result, &sources[0]), input().value);
        assert_eq!(output.accepted, expected);
        for d in &output.enumeration_diagnostics {
            assert_eq!(d.uri.as_deref(), Some("input.cem"));
        }
        if expected.is_none() {
            assert!(output.stopped.is_some());
        }
    }
    let (mut host, sources, mut implementations) =
        setup("{type @name=sample @kind=scalar @values=3}");
    implementations
        .select_equality(
            sources[0].clone(),
            EqualityBinding::Ready(
                RegisteredScalarEquality::new(
                    sources[0].clone(),
                    "eq",
                    ScalarRepresentation::String,
                    EqualityResult {
                        result: EqualityExecution::Complete {
                            equal: true,
                            diagnostics: vec![],
                        },
                        abort: true,
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(
                RegisteredConstantInterpreter::new(
                    sources[0].clone(),
                    "constant",
                    ScalarRepresentation::String,
                    Interpret,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile(&mut host, &sources, &implementations);
    let output = validate_descriptor(compiled(&result, &sources[0]), input().value);
    assert_eq!(output.accepted, None);
    assert!(matches!(
        output.stopped.unwrap().reason,
        ValidationStopReason::Control(_)
    ));
}
#[test]
fn enumeration_diagnostic_budget_is_shared_and_preparation_is_attributed() {
    let (mut host, sources, mut implementations) = setup(
        "{type @name=base @kind=scalar @values=\"3 5\"} {type @name=derived @base=base @values=3}",
    );
    implementations
        .select_equality(
            sources[0].clone(),
            EqualityBinding::Ready(
                RegisteredScalarEquality::new(
                    sources[0].clone(),
                    "eq",
                    ScalarRepresentation::String,
                    EqualityResult {
                        result: EqualityExecution::Complete {
                            equal: true,
                            diagnostics: vec![notice()],
                        },
                        abort: false,
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(
                RegisteredConstantInterpreter::new(
                    sources[0].clone(),
                    "constant",
                    ScalarRepresentation::String,
                    ConstantResult(ConstantExecution::Prepared {
                        value: vec![Item::Atomic(AtomValue::String("3".into()))],
                        diagnostics: vec![notice()],
                    }),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile(&mut host, &[sources[1].clone()], &implementations);
    assert!(result.is_ready());
    assert_eq!(result.diagnostics.len(), 4);
    assert!(result
        .diagnostics
        .iter()
        .all(|d| d.node.is_some() && d.source_map.is_some()));
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let output = compiled(&result, &sources[1]).validate(
        &input(),
        &runtime,
        ValidationLimits {
            max_diagnostics: 1,
            ..Default::default()
        },
    );
    assert_eq!(output.accepted, None);
    assert_eq!(output.enumerations.len(), 1);
    assert!(matches!(
        output.stopped.unwrap().reason,
        ValidationStopReason::Limit("diagnostics")
    ));
    let pending = compile_datatypes_with_runtime(
        sources[0].declaration().document().clone(),
        &[sources[1].clone()],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        ConstantPreparationLimits {
            validation: ValidationLimits {
                max_diagnostics: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-constant-diagnostic-limit");
}
#[test]
fn enumeration_integer_constants_use_typed_equality_without_rewriting_tokens() {
    #[derive(Debug)]
    struct Integer;
    impl NativeConstantInterpreter for Integer {
        fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution {
            ConstantExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::Integer(
                    call.token.text().parse().unwrap(),
                ))],
                diagnostics: vec![],
            }
        }
    }
    let (mut host, sources) = types_fixture("{type @name=sample @kind=scalar @values=003}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::Integer),
        ))
        .unwrap();
    implementations
        .select_equality(
            sources[0].clone(),
            EqualityBinding::Ready(
                RegisteredScalarEquality::new(
                    sources[0].clone(),
                    "integer-eq",
                    ScalarRepresentation::Integer,
                    Equal { numeric: false },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(
                RegisteredConstantInterpreter::new(
                    sources[0].clone(),
                    "integer-constant",
                    ScalarRepresentation::Integer,
                    Integer,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile(&mut host, &sources, &implementations);
    assert!(result.is_ready());
    let descriptor = compiled(&result, &sources[0]);
    assert_eq!(
        descriptor.enumerations()[0].constants()[0].token.text(),
        "003"
    );
    assert_eq!(
        validate_descriptor(descriptor, vec![Item::Atomic(AtomValue::Integer(3.into()))]).accepted,
        Some(true)
    );
    assert_eq!(
        validate_descriptor(descriptor, vec![Item::Atomic(AtomValue::Integer(4.into()))]).accepted,
        Some(false)
    );
}
#[test]
fn enumeration_constants_must_pass_local_rules_with_original_source_candidate() {
    #[derive(Debug)]
    struct Candidate {
        expected: SchemaDeclarationNode,
        accepted: Option<bool>,
    }
    impl NativeDatatypeValidator for Candidate {
        fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
            assert_eq!(call.candidate.len(), 1);
            let candidate = cem_ql::eval::retained_cem_node(&call.candidate[0]).unwrap();
            assert_eq!(candidate.node_id(), self.expected.node_id());
            assert!(Arc::ptr_eq(
                candidate.owner().ast_owner(),
                self.expected.document()
            ));
            let mut stream = query(match self.accepted {
                Some(true) => "{ accepted: true, diagnostics: () }",
                Some(false) => "{ accepted: false, diagnostics: () }",
                None => "1 / 0",
            });
            if self.accepted.is_none() {
                stream.diagnostics.push(notice());
            }
            RuleExecution::Complete(stream)
        }
    }
    for accepted in [Some(false), Some(true), None] {
        let text = declaration(true).replace("@name=sample", "@name=sample @values=same");
        let schema = source(&text);
        let mut registry = DatatypeRegistry::default();
        registry
            .insert(schema.schema.clone(), node(&schema, "type"))
            .unwrap();
        let source = registry.source(&schema.schema, "sample").unwrap();
        let tree = RetainedCemTree::from_shared(
            source.declaration().document().clone(),
            "types.cem",
            "",
            Default::default(),
            None,
        )
        .unwrap();
        let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
        host.register_scope(
            tree,
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        );
        host.register_datatype_source(source.clone()).unwrap();
        let mut implementations = DatatypeImplementations::default();
        let mut implementation = implementation(
            &source,
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        );
        implementation.validator = Some((schema.schema.clone(), node(&schema, "behavior")));
        implementations.register(implementation).unwrap();
        let mut validations = DatatypeValidationRegistry::default();
        validations
            .register_native(
                "urn:test:validate",
                contract(&schema, true),
                adapter(),
                None,
                Candidate {
                    expected: source.attribute("values").unwrap().clone(),
                    accepted,
                },
            )
            .unwrap();
        capabilities(&mut implementations, &source, false);
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let result = compile_datatypes_with_runtime(
            source.declaration().document().clone(),
            &[source],
            &mut host,
            &implementations,
            &validations,
            ReferenceTraversalLimits::schema_defaults().unwrap(),
            &runtime,
            Default::default(),
        );
        assert_eq!(
            result.is_ready(),
            accepted == Some(true),
            "{:?}",
            result.issues
        );
        if accepted.is_none() {
            assert_eq!(
                result.issues[0].code,
                "datatype-constant-validation-incomplete"
            );
            assert!(result
                .diagnostics
                .iter()
                .any(|d| d.code == "test.equality" && d.node.is_some()));
        }
        if accepted == Some(false) {
            assert_eq!(result.issues[0].code, "datatype-constant-contract-rejected");
        }
    }
}
