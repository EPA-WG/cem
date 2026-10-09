use super::*;
use cem_ql::datatype_conversion::*;
#[derive(Debug)]
struct Convert {
    calls: Arc<AtomicUsize>,
    result: ConversionExecution,
}
impl NativeDatatypeConverter for Convert {
    fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        self.result.clone()
    }
}
fn signature() -> ConversionSignature {
    ConversionSignature {
        kind: DatatypeKind::Scalar,
        input: ConversionRepresentation::Lexical,
        output: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
    }
}
fn converter(
    source: &DatatypeSource,
    id: &str,
    calls: Arc<AtomicUsize>,
    result: ConversionExecution,
) -> RegisteredDatatypeConverter {
    RegisteredDatatypeConverter::new(source.clone(), id, signature(), Convert { calls, result })
        .unwrap()
}
fn request() -> ConversionInput {
    ConversionInput {
        value: ConversionValue::Lexical(cem_ml::schema::datatype_contracts::LexicalInput::new(
            Arc::from(" original "),
            Default::default(),
        )),
        candidate: vec![],
        fallback: DiagnosticAttribution {
            uri: Some("input.cem".into()),
            ..Default::default()
        },
    }
}
fn run(descriptor: &ExecutableDatatype) -> DatatypeConversion {
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    descriptor.convert(&request(), &runtime, Default::default())
}
#[test]
fn inherits_one_converter_and_retains_the_original_implementation_owner() {
    let (mut host, sources) =
        types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            signature().output,
        ))
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    implementations
        .select_converter(
            sources[0].clone(),
            ConverterBinding::Ready(converter(
                &sources[0],
                "test",
                calls.clone(),
                ConversionExecution::Converted {
                    value: input().value,
                    diagnostics: vec![],
                },
            )),
        )
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[1].clone()],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let descriptor = compiled(&result, &sources[1]);
    assert_eq!(
        validate_descriptor(descriptor, input().value).accepted,
        Some(true)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let output = run(descriptor);
    assert_eq!(output.accepted, Some(true));
    assert_eq!(output.value.unwrap(), input().value);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        output.converter.unwrap().source.declaration().identity(),
        sources[0].declaration().identity()
    );
}

fn single(
    sig: ConversionSignature,
    execution: ConversionExecution,
    calls: Arc<AtomicUsize>,
) -> cem_ml::schema::datatype_contracts::DatatypeCompilation {
    let text = match sig.kind {
        DatatypeKind::List => {
            "{type @name=item @kind=scalar} {type @name=sample @kind=list @base=item}"
        }
        DatatypeKind::Node => "{type @name=sample @kind=node}",
        _ => "{type @name=sample @kind=scalar}",
    };
    let (mut host, sources) = types_fixture(text);
    let mut implementations = DatatypeImplementations::default();
    if sig.kind == DatatypeKind::List {
        let ValueRepresentation::List(p) = sig.output else {
            panic!()
        };
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                ValueRepresentation::Scalar(p),
            ))
            .unwrap();
    }
    let sample = sources.last().unwrap();
    implementations
        .register(implementation(sample, sig.kind, sig.output))
        .unwrap();
    implementations
        .select_converter(
            sample.clone(),
            ConverterBinding::Ready(
                RegisteredDatatypeConverter::new(
                    sample.clone(),
                    "convert",
                    sig,
                    Convert {
                        calls,
                        result: execution,
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile_datatypes(
        sample.declaration().document().clone(),
        &[sample.clone()],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    result
}
fn last(result: &cem_ml::schema::datatype_contracts::DatatypeCompilation) -> &ExecutableDatatype {
    compiled(result, &result.sources[0])
}
#[test]
fn replacement_runs_once_and_all_inherited_and_local_rules_check_its_output() {
    let (mut host, sources) =
        types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
    let mut implementations = DatatypeImplementations::default();
    let mut validations = DatatypeValidationRegistry::default();
    let mut converter_calls = vec![];
    let mut validation_calls = vec![];
    for (i, s) in sources.iter().enumerate() {
        let rule = source(&declaration(false));
        let count = Arc::new(AtomicUsize::new(0));
        validations
            .register_native(
                "urn:test:validate",
                contract(&rule, false),
                adapter(),
                None,
                Check {
                    calls: count.clone(),
                    accepted: i != 0,
                },
            )
            .unwrap();
        validation_calls.push(count);
        let mut contract = implementation(s, DatatypeKind::Scalar, signature().output);
        contract.validator = Some((rule.schema.clone(), node(&rule, "behavior")));
        implementations.register(contract).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        implementations
            .select_converter(
                s.clone(),
                ConverterBinding::Ready(converter(
                    s,
                    "explicit",
                    count.clone(),
                    ConversionExecution::Converted {
                        value: input().value,
                        diagnostics: vec![],
                    },
                )),
            )
            .unwrap();
        converter_calls.push(count);
    }
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[1].clone()],
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready());
    let output = run(compiled(&result, &sources[1]));
    assert_eq!(output.accepted, Some(false));
    assert_eq!(output.value.unwrap(), input().value);
    assert_eq!(converter_calls[0].load(Ordering::SeqCst), 0);
    assert_eq!(converter_calls[1].load(Ordering::SeqCst), 1);
    assert!(validation_calls
        .iter()
        .all(|n| n.load(Ordering::SeqCst) == 1));
    let checked = output.validation.unwrap();
    assert_eq!(
        checked.completed[0].datatype.identity(),
        sources[0].declaration().identity()
    );
    assert_eq!(
        checked.completed[1].datatype.identity(),
        sources[1].declaration().identity()
    );
}
#[test]
fn unavailable_selection_blocks_inheritance_but_optional_absence_allows_validation() {
    let (mut host, sources) =
        types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            signature().output,
        ))
        .unwrap();
    let compile = |host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
                   implementations: &DatatypeImplementations| {
        compile_datatypes(
            sources[0].declaration().document().clone(),
            &[sources[1].clone()],
            host,
            implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
    };
    let without = compile(&mut host, &implementations);
    assert!(without.is_ready());
    assert!(matches!(
        run(compiled(&without, &sources[1])).stopped,
        Some(ConversionStop::NoConverter)
    ));
    implementations
        .select_converter(
            sources[0].clone(),
            ConverterBinding::Ready(converter(
                &sources[0],
                "base",
                Arc::new(AtomicUsize::new(0)),
                ConversionExecution::Converted {
                    value: input().value,
                    diagnostics: vec![],
                },
            )),
        )
        .unwrap();
    implementations
        .select_converter(sources[1].clone(), ConverterBinding::Unavailable)
        .unwrap();
    let pending = compile(&mut host, &implementations);
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-converter-unavailable");
}
#[test]
fn rejected_pending_unavailable_failed_and_malformed_results_never_fall_back() {
    for execution in [
        ConversionExecution::Rejected {
            diagnostics: vec![],
        },
        ConversionExecution::Pending(vec![]),
        ConversionExecution::Unavailable(vec![]),
        ConversionExecution::Failed(vec![]),
        ConversionExecution::Converted {
            value: vec![],
            diagnostics: vec![],
        },
    ] {
        let rejected = matches!(execution, ConversionExecution::Rejected { .. });
        let (mut host, sources) =
            types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                signature().output,
            ))
            .unwrap();
        let base = Arc::new(AtomicUsize::new(0));
        let derived = Arc::new(AtomicUsize::new(0));
        implementations
            .select_converter(
                sources[0].clone(),
                ConverterBinding::Ready(converter(
                    &sources[0],
                    "base",
                    base.clone(),
                    ConversionExecution::Converted {
                        value: input().value,
                        diagnostics: vec![],
                    },
                )),
            )
            .unwrap();
        implementations
            .select_converter(
                sources[1].clone(),
                ConverterBinding::Ready(converter(
                    &sources[1],
                    "derived",
                    derived.clone(),
                    execution,
                )),
            )
            .unwrap();
        let result = compile_datatypes(
            sources[0].declaration().document().clone(),
            &[sources[1].clone()],
            &mut host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(result.is_ready());
        let output = run(compiled(&result, &sources[1]));
        assert_eq!(output.accepted, if rejected { Some(false) } else { None });
        assert!(output.value.is_none());
        assert!(output.validation.is_none());
        assert_eq!(base.load(Ordering::SeqCst), 0);
        assert_eq!(derived.load(Ordering::SeqCst), 1);
    }
}
#[test]
fn list_conversion_preserves_empty_order_and_duplicates_without_implicit_item_conversion() {
    let mut sig = signature();
    sig.kind = DatatypeKind::List;
    sig.output = ValueRepresentation::List(ScalarRepresentation::String);
    for value in [vec![], query("(\"a\", \"a\", \"b\")").items] {
        let result = single(
            sig,
            ConversionExecution::Converted {
                value: value.clone(),
                diagnostics: vec![],
            },
            Arc::new(AtomicUsize::new(0)),
        );
        let output = run(last(&result));
        assert_eq!(output.accepted, Some(true));
        assert_eq!(output.value, Some(value));
    }
    let result = single(
        sig,
        ConversionExecution::Converted {
            value: query("1").items,
            diagnostics: vec![],
        },
        Arc::new(AtomicUsize::new(0)),
    );
    assert!(matches!(
        run(last(&result)).stopped,
        Some(ConversionStop::InvalidOutput("representation"))
    ));
}
#[test]
fn native_output_preserves_the_exact_input_view_and_does_not_rebuild_access() {
    let src = source("{schema | {target | {#pending}}}");
    let target = node(&src, "target");
    let original = native(&target);
    let sig = ConversionSignature {
        kind: DatatypeKind::Node,
        input: ConversionRepresentation::Values(ValueRepresentation::Nodes),
        output: ValueRepresentation::Nodes,
        candidate: CandidateRequirement::Optional,
    };
    for (values, valid) in [
        (vec![original.clone(), original.clone()], true),
        (vec![native(&target)], false),
    ] {
        let result = single(
            sig,
            ConversionExecution::Converted {
                value: values,
                diagnostics: vec![],
            },
            Arc::new(AtomicUsize::new(0)),
        );
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let mut request = request();
        request.value = ConversionValue::Values(vec![original.clone()]);
        let output = last(&result).convert(&request, &runtime, Default::default());
        assert_eq!(output.accepted, if valid { Some(true) } else { None });
        if !valid {
            assert!(matches!(
                output.stopped,
                Some(ConversionStop::InvalidOutput("native-view-identity"))
            ));
        }
    }
    assert!(target
        .document()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { targets: None, .. })));
}
#[test]
fn registration_requires_exact_source_compatible_output_and_explicit_native_boundaries() {
    let (mut host, sources) = types_fixture("{type @name=x @kind=scalar}");
    let (_, other) = types_fixture("{type @name=x @kind=scalar}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            signature().output,
        ))
        .unwrap();
    let capability = converter(
        &other[0],
        "same-name",
        Arc::new(AtomicUsize::new(0)),
        ConversionExecution::Rejected {
            diagnostics: vec![],
        },
    );
    assert!(implementations
        .select_converter(sources[0].clone(), ConverterBinding::Ready(capability))
        .is_err());
    let mut sig = signature();
    sig.output = ValueRepresentation::Scalar(ScalarRepresentation::Boolean);
    implementations
        .select_converter(
            sources[0].clone(),
            ConverterBinding::Ready(
                RegisteredDatatypeConverter::new(
                    sources[0].clone(),
                    "wrong-output",
                    sig,
                    Convert {
                        calls: Arc::new(AtomicUsize::new(0)),
                        result: ConversionExecution::Rejected {
                            diagnostics: vec![],
                        },
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    assert!(implementations
        .select_converter(sources[0].clone(), ConverterBinding::Unavailable)
        .is_err());
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(!result.is_ready());
    assert_eq!(result.issues[0].code, "converter-output-incompatible");
    sig.input = ConversionRepresentation::Values(ValueRepresentation::Nodes);
    assert!(RegisteredDatatypeConverter::new(
        sources[0].clone(),
        "extract",
        sig,
        Convert {
            calls: Arc::new(AtomicUsize::new(0)),
            result: ConversionExecution::Rejected {
                diagnostics: vec![]
            }
        }
    )
    .is_err());
}
#[test]
fn converter_preflight_and_output_limits_share_control_and_preserve_diagnostic_attribution() {
    use cem_ql::datatype_validation::ValidationLimits;
    let calls = Arc::new(AtomicUsize::new(0));
    let mut sig = signature();
    sig.candidate = CandidateRequirement::Required;
    let diagnostic = cem_ml::diagnostics::Diagnostic {
        code: "test.notice".into(),
        message: "Converted".into(),
        severity: cem_ml::diagnostics::Severity::Error,
        ..Default::default()
    };
    let result = single(
        sig,
        ConversionExecution::Converted {
            value: input().value,
            diagnostics: vec![diagnostic],
        },
        calls.clone(),
    );
    let descriptor = last(&result);
    assert!(matches!(
        run(descriptor).stopped,
        Some(ConversionStop::MissingCandidate)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let mut request = request();
    request.candidate = vec![native(result.sources[0].declaration())];
    let too_short = descriptor.convert(
        &request,
        &runtime,
        ConversionLimits {
            max_lexical_bytes: 1,
            ..Default::default()
        },
    );
    assert!(matches!(
        too_short.stopped,
        Some(ConversionStop::Limit("lexical-bytes"))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let too_few = descriptor.convert(
        &request,
        &runtime,
        ConversionLimits {
            max_output_values: 0,
            ..Default::default()
        },
    );
    assert!(matches!(
        too_few.stopped,
        Some(ConversionStop::Limit("output-values"))
    ));
    let too_many = descriptor.convert(
        &request,
        &runtime,
        ConversionLimits {
            validation: ValidationLimits {
                max_diagnostics: 0,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(matches!(
        too_many.stopped,
        Some(ConversionStop::Limit("diagnostics"))
    ));
    let output = descriptor.convert(&request, &runtime, Default::default());
    assert_eq!(output.accepted, Some(true));
    assert_eq!(output.diagnostics[0].node, request.candidate[0].identity());
    assert_eq!(
        output.diagnostics[0].source_map,
        request.candidate[0].source_map()
    );
    let abort = AbortSignal::new();
    abort.abort();
    let control = OperationControl::new(abort);
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let before = calls.load(Ordering::SeqCst);
    assert!(matches!(
        descriptor
            .convert(&request, &runtime, Default::default())
            .stopped,
        Some(ConversionStop::Control(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), before);
}

#[test]
fn typed_inputs_are_checked_and_post_callback_cancellation_prevents_publication() {
    #[derive(Debug)]
    struct Cancel;
    impl NativeDatatypeConverter for Cancel {
        fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
            call.runtime.control.abort_signal().abort();
            ConversionExecution::Converted {
                value: input().value,
                diagnostics: vec![],
            }
        }
    }
    let mut sig = signature();
    sig.input = ConversionRepresentation::Values(signature().output);
    let calls = Arc::new(AtomicUsize::new(0));
    let result = single(
        sig,
        ConversionExecution::Converted {
            value: input().value,
            diagnostics: vec![],
        },
        calls.clone(),
    );
    assert!(matches!(
        run(last(&result)).stopped,
        Some(ConversionStop::InvalidInput("value"))
    ));
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let mut request = request();
    request.value = ConversionValue::Values(vec![]);
    assert!(matches!(
        last(&result)
            .convert(&request, &runtime, Default::default())
            .stopped,
        Some(ConversionStop::InvalidInput("value"))
    ));
    request.value = ConversionValue::Values(input().value);
    assert!(matches!(
        last(&result)
            .convert(
                &request,
                &runtime,
                ConversionLimits {
                    max_input_values: 0,
                    ..Default::default()
                }
            )
            .stopped,
        Some(ConversionStop::Limit("input-values"))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        last(&result)
            .convert(&request, &runtime, Default::default())
            .accepted,
        Some(true)
    );
    let (mut host, sources) = types_fixture("{type @name=x @kind=scalar}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            sig.output,
        ))
        .unwrap();
    implementations
        .select_converter(
            sources[0].clone(),
            ConverterBinding::Ready(
                RegisteredDatatypeConverter::new(sources[0].clone(), "cancel", sig, Cancel)
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
    let output = last(&result).convert(&request, &runtime, Default::default());
    assert!(matches!(output.stopped, Some(ConversionStop::Control(_))));
    assert!(output.value.is_none());
    assert!(output.validation.is_none());
}
#[test]
fn conversion_and_validation_share_diagnostic_allowance_and_keep_failed_output_inspectable() {
    let (mut host, sources) = types_fixture(
        "{type @name=item @kind=scalar} {type @name=names @kind=list @base=item @min-items=1}",
    );
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            signature().output,
        ))
        .unwrap();
    implementations
        .register(implementation(
            &sources[1],
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
        ))
        .unwrap();
    let mut sig = signature();
    sig.kind = DatatypeKind::List;
    sig.output = ValueRepresentation::List(ScalarRepresentation::String);
    let diagnostic = cem_ml::diagnostics::Diagnostic {
        code: "converted".into(),
        uri: Some("original.cem".into()),
        ..Default::default()
    };
    implementations
        .select_converter(
            sources[1].clone(),
            ConverterBinding::Ready(
                RegisteredDatatypeConverter::new(
                    sources[1].clone(),
                    "empty",
                    sig,
                    Convert {
                        calls: Arc::new(AtomicUsize::new(0)),
                        result: ConversionExecution::Converted {
                            value: vec![],
                            diagnostics: vec![diagnostic],
                        },
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let result = compile_datatypes(
        sources[0].declaration().document().clone(),
        &[sources[1].clone()],
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready());
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let output = last(&result).convert(
        &request(),
        &runtime,
        ConversionLimits {
            validation: ValidationLimits {
                max_diagnostics: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(output.accepted, None);
    assert_eq!(output.value, Some(vec![]));
    assert_eq!(output.diagnostics[0].uri.as_deref(), Some("original.cem"));
    assert!(output.validation.unwrap().stopped.is_some());
    let output = run(last(&result));
    assert_eq!(output.accepted, Some(false));
    assert_eq!(output.value, Some(vec![]));
    assert_eq!(output.validation.unwrap().cardinality.len(), 1);
}
