use super::*;
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    schema::{
        datatype_contracts::CompiledDatatypeContract,
        document_model::shipped_datatypes::ShippedDatatype as T,
    },
};
use cem_ql::datatype_serialization::*;

#[derive(Debug)]
struct Serialize {
    calls: Arc<AtomicUsize>,
    execution: SerializationExecution,
    cancel: bool,
}
impl NativeListSerializer for Serialize {
    fn serialize(&self, call: ListSerializationCall<'_>) -> SerializationExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        if self.cancel {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        self.execution.clone()
    }
}
fn signature() -> ListSerializationSignature {
    ListSerializationSignature {
        item: ScalarRepresentation::String,
        candidate: CandidateRequirement::Optional,
    }
}
fn registered(
    source: &DatatypeSource,
    signature: ListSerializationSignature,
    calls: Arc<AtomicUsize>,
    execution: SerializationExecution,
    cancel: bool,
) -> RegisteredListSerializer {
    RegisteredListSerializer::new(
        source.clone(),
        "test:serialize",
        signature,
        Serialize {
            calls,
            execution,
            cancel,
        },
    )
    .unwrap()
}
fn descriptor(
    sig: ListSerializationSignature,
    execution: SerializationExecution,
    calls: Arc<AtomicUsize>,
    cancel: bool,
) -> ExecutableDatatype {
    let (mut host, sources) = types_fixture(
        "{type @name=item @kind=scalar} {type @name=names @kind=list @base=item @max-items=3}",
    );
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    implementations
        .register(implementation(
            &sources[1],
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
        ))
        .unwrap();
    implementations
        .select_list_serializer(
            sources[1].clone(),
            ListSerializerBinding::Ready(registered(&sources[1], sig, calls, execution, cancel)),
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
    compiled(&result, &sources[1]).clone()
}
fn serialized(text: &str) -> SerializationExecution {
    SerializationExecution::Serialized {
        text: text.into(),
        diagnostics: vec![],
    }
}
fn input(values: &[&str]) -> ValidationInput {
    ValidationInput {
        value: values
            .iter()
            .map(|s| Item::Atomic(AtomValue::String((*s).into())))
            .collect(),
        candidate: vec![],
        fallback: DiagnosticAttribution {
            uri: Some("original.cem".into()),
            ..Default::default()
        },
    }
}
fn run(
    d: &ExecutableDatatype,
    input: &ValidationInput,
    limits: SerializationLimits,
) -> DatatypeSerialization {
    let control = OperationControl::default();
    d.serialize_list(
        input,
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        limits,
    )
}

#[test]
fn shipped_serialization_preserves_order_duplicates_and_original_lexical_views() {
    for (ty, original, expected) in [
        (T::NameList, "\u{2003}β a β\t", "β a β"),
        (T::WildcardNameList, "\ta:* a  a:* ", "a:* a a:*"),
    ] {
        let d = super::shipped_lists::list_descriptor(ty, None);
        let conversion = super::shipped_conversion::convert(&d, original);
        assert_eq!(conversion.accepted, Some(true));
        let values = conversion.value.unwrap();
        let keys: Vec<_> = values.iter().map(Item::identity).collect();
        let sources: Vec<_> = values
            .iter()
            .map(|v| {
                let (lexical, span) = cem_ql::datatype_shipped::token_source(v).unwrap();
                (lexical.text.clone(), lexical.source.clone(), span)
            })
            .collect();
        let request = ValidationInput {
            value: values,
            ..input(&[])
        };
        let result = run(&d, &request, Default::default());
        assert_eq!(result.accepted, Some(true), "{result:?}");
        assert_eq!(result.text.as_deref(), Some(expected));
        assert_eq!(
            result.serializer.unwrap().source.declaration().identity(),
            d.source().declaration().identity()
        );
        for (index, value) in request.value.iter().enumerate() {
            assert_eq!(value.identity(), keys[index]);
            let (lexical, span) = cem_ql::datatype_shipped::token_source(value).unwrap();
            assert!(Arc::ptr_eq(&lexical.text, &sources[index].0));
            assert_eq!(lexical.text.as_ref(), original);
            assert_eq!(lexical.source, sources[index].1);
            assert_eq!(span, sources[index].2);
        }
        for invalid in [
            input(&[]),
            input(&["a", "b:c"]),
            input(&["a", "b", "c", "d"]),
            input(&[" a"]),
            input(&["a "]),
            input(&["a\u{2003}"]),
            input(&["a b"]),
        ] {
            let result = run(&d, &invalid, Default::default());
            assert_eq!(result.accepted, Some(false));
            assert!(result.text.is_none());
        }
    }
}

#[test]
fn empty_serialization_is_present_and_validation_never_invokes_the_serializer() {
    let calls = Arc::new(AtomicUsize::new(0));
    let d = descriptor(signature(), serialized(""), calls.clone(), false);
    assert_eq!(validate_descriptor(&d, vec![]).accepted, Some(true));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = run(&d, &input(&[]), Default::default());
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.text, Some(String::new()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let result = run(&d, &input(&["a", "b", "c", "d"]), Default::default());
    assert_eq!(result.accepted, Some(false));
    assert!(result.text.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut limits = SerializationLimits::default();
    limits.validation.max_input_values = 1;
    let result = run(&d, &input(&["a", "b"]), limits);
    assert_eq!(result.accepted, None);
    assert!(result.text.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn serialization_requires_native_candidate_and_typed_items_before_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut sig = signature();
    sig.candidate = CandidateRequirement::Required;
    let d = descriptor(sig, serialized("a"), calls.clone(), false);
    let mut request = input(&["a"]);
    assert!(matches!(
        run(&d, &request, Default::default()).stopped,
        Some(SerializationStop::MissingCandidate)
    ));
    request.candidate = vec![Item::Atomic(AtomValue::String("fake".into()))];
    assert!(run(&d, &request, Default::default()).text.is_none());
    request.candidate = vec![native(d.source().declaration()); 2];
    assert!(run(&d, &request, Default::default()).text.is_none());
    request.candidate.truncate(1);
    request.value = vec![native(d.source().declaration())];
    let result = run(&d, &request, Default::default());
    assert!(result.text.is_none());
    assert_eq!(result.accepted, None);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    request.value = input(&["a"]).value;
    assert_eq!(
        run(&d, &request, Default::default()).text.as_deref(),
        Some("a")
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn serialization_keeps_rejected_pending_and_failed_outcomes_distinct() {
    for (execution, accepted, code) in [
        (
            SerializationExecution::Rejected {
                diagnostics: vec![],
            },
            Some(false),
            "",
        ),
        (SerializationExecution::Pending(vec![]), None, "Pending"),
        (
            SerializationExecution::Unavailable(vec![]),
            None,
            "Unavailable",
        ),
        (SerializationExecution::Failed(vec![]), None, "Failed"),
        (SerializationExecution::Limit("work"), None, "Limit"),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let d = descriptor(signature(), execution, calls.clone(), false);
        let result = run(&d, &input(&["a"]), Default::default());
        assert_eq!(result.accepted, accepted);
        assert!(result.text.is_none());
        assert_eq!(result.validation.unwrap().accepted, Some(true));
        if !code.is_empty() {
            assert!(format!("{:?}", result.stopped).contains(code));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn serialization_bounds_output_and_preserves_diagnostic_attribution() {
    let mut diagnostic = Diagnostic {
        code: "test.serialize".into(),
        severity: Severity::Warning,
        message: "serialized".into(),
        ..Default::default()
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let execution = |d| SerializationExecution::Serialized {
        text: "β".into(),
        diagnostics: vec![d],
    };
    let d = descriptor(
        signature(),
        execution(diagnostic.clone()),
        calls.clone(),
        false,
    );
    let result = run(&d, &input(&["β"]), Default::default());
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.diagnostics[0].uri.as_deref(), Some("original.cem"));
    let result = run(
        &d,
        &input(&["β"]),
        SerializationLimits {
            max_output_bytes: 1,
            ..Default::default()
        },
    );
    assert!(matches!(
        result.stopped,
        Some(SerializationStop::Limit("output-bytes"))
    ));
    assert!(result.text.is_none());
    let mut limits = SerializationLimits::default();
    limits.validation.max_diagnostics = 0;
    assert!(matches!(
        run(&d, &input(&["β"]), limits).stopped,
        Some(SerializationStop::Limit("diagnostics"))
    ));
    diagnostic.uri = Some("registered.cem".into());
    let d = descriptor(signature(), execution(diagnostic), calls, false);
    assert_eq!(
        run(&d, &input(&["β"]), Default::default()).diagnostics[0]
            .uri
            .as_deref(),
        Some("registered.cem")
    );
}

#[test]
fn cancellation_before_or_during_serialization_cannot_publish_text() {
    let calls = Arc::new(AtomicUsize::new(0));
    let d = descriptor(signature(), serialized("a"), calls.clone(), true);
    let result = run(&d, &input(&["a"]), Default::default());
    assert!(matches!(
        result.stopped,
        Some(SerializationStop::Control(_))
    ));
    assert!(result.text.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let result = d.serialize_list(
        &input(&["a"]),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    );
    assert!(matches!(
        result.stopped,
        Some(SerializationStop::Control(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn serializer_selection_checks_original_owners_and_effective_representation() {
    let (mut host, sources) =
        types_fixture("{type @name=item @kind=scalar} {type @name=names @kind=list @base=item}");
    let (_, copied) =
        types_fixture("{type @name=item @kind=scalar} {type @name=names @kind=list @base=item}");
    let calls = Arc::new(AtomicUsize::new(0));
    let registration = |source: &DatatypeSource, sig| {
        registered(source, sig, calls.clone(), serialized("a"), false)
    };
    let mut base = DatatypeImplementations::default();
    base.register(implementation(
        &sources[0],
        DatatypeKind::Scalar,
        ValueRepresentation::Scalar(ScalarRepresentation::String),
    ))
    .unwrap();
    base.register(implementation(
        &sources[1],
        DatatypeKind::List,
        ValueRepresentation::List(ScalarRepresentation::String),
    ))
    .unwrap();
    assert_eq!(
        base.select_list_serializer(
            sources[1].clone(),
            ListSerializerBinding::Ready(registration(&copied[1], signature()))
        ),
        Err("unrelated-list-serializer-source")
    );
    let compile = |host: &mut cem_ql::schema_references::CemQlSchemaDeclarationHost,
                   implementations: &DatatypeImplementations| {
        compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            host,
            implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        )
    };
    let absent = compile(&mut host, &base);
    assert!(absent.is_ready());
    assert!(matches!(
        run(
            compiled(&absent, &sources[1]),
            &input(&["a"]),
            Default::default()
        )
        .stopped,
        Some(SerializationStop::NoSerializer)
    ));
    let mut unavailable = base.clone();
    unavailable
        .select_list_serializer(sources[1].clone(), ListSerializerBinding::Unavailable)
        .unwrap();
    let result = compile(&mut host, &unavailable);
    assert!(!result.is_ready());
    assert!(result
        .issues
        .iter()
        .any(|i| i.code == "datatype-list-serializer-unavailable"));
    let mut incompatible = base.clone();
    let mut sig = signature();
    sig.item = ScalarRepresentation::Integer;
    incompatible
        .select_list_serializer(
            sources[1].clone(),
            ListSerializerBinding::Ready(registration(&sources[1], sig)),
        )
        .unwrap();
    let result = compile(&mut host, &incompatible);
    assert!(!result.is_ready());
    assert!(result
        .issues
        .iter()
        .any(|i| i.code == "list-serializer-input-incompatible"));
    let mut scalar = base.clone();
    scalar
        .select_list_serializer(
            sources[0].clone(),
            ListSerializerBinding::Ready(registration(&sources[0], signature())),
        )
        .unwrap();
    let result = compile(&mut host, &scalar);
    assert!(!result.is_ready());
    assert!(result
        .issues
        .iter()
        .any(|i| i.code == "list-serializer-requires-list"));
    base.select_list_serializer(
        sources[1].clone(),
        ListSerializerBinding::Ready(registration(&sources[1], signature())),
    )
    .unwrap();
    assert_eq!(
        base.select_list_serializer(sources[1].clone(), ListSerializerBinding::Unavailable),
        Err("duplicate-list-serializer-selection")
    );
    let result = compile(&mut host, &base);
    assert!(result.is_ready());
    assert!(compiled(&result, &sources[0]).list_serializer().is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn validation_and_serialization_share_the_diagnostic_allowance() {
    let profile = source(&declaration(false).replace(
        "@name=value @type=schema:string @source=value @required=true @cardinality=one",
        "@name=value @type=schema:string @source=value @required=true @cardinality=zero-or-more",
    ));
    let behavior = node(&profile, "behavior");
    let (mut host, sources) =
        types_fixture("{type @name=item @kind=scalar} {type @name=names @kind=list @base=item}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    let mut list = implementation(
        &sources[1],
        DatatypeKind::List,
        ValueRepresentation::List(ScalarRepresentation::String),
    );
    list.validator = Some((profile.schema.clone(), behavior.clone()));
    implementations.register(list).unwrap();
    let mut sig = super::super::signature(false);
    sig.kind = DatatypeKind::List;
    sig.value = ValueRepresentation::List(ScalarRepresentation::String);
    let mut registry = DatatypeValidationRegistry::default();
    registry.register_native(
        "urn:test:validate", DatatypeBehaviorContract::compile(&profile, &behavior, sig).unwrap(), adapter(), None,
        Outcome(RuleExecution::Complete(query(r#"{ accepted: true, diagnostics: ({ code: "validated", severity: "warning", message: "validated" }) }"#))),
    ).unwrap();
    #[derive(Debug)]
    struct Remaining;
    impl NativeListSerializer for Remaining {
        fn serialize(&self, call: ListSerializationCall<'_>) -> SerializationExecution {
            assert_eq!(call.limits.validation.max_diagnostics, 1);
            SerializationExecution::Serialized {
                text: "a".into(),
                diagnostics: vec![Diagnostic::default(); 2],
            }
        }
    }
    implementations
        .select_list_serializer(
            sources[1].clone(),
            ListSerializerBinding::Ready(
                RegisteredListSerializer::new(
                    sources[1].clone(),
                    "remaining",
                    signature(),
                    Remaining,
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
        &registry,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    let mut limits = SerializationLimits::default();
    limits.validation.max_diagnostics = 2;
    let result = run(compiled(&result, &sources[1]), &input(&["a"]), limits);
    assert!(
        matches!(
            result.stopped,
            Some(SerializationStop::Limit("diagnostics"))
        ),
        "{result:?}"
    );
    assert_eq!(result.accepted, None);
    assert!(result.text.is_none());
    assert_eq!(result.validation.unwrap().completed.len(), 1);
}
