use super::*;
use cem_ml::schema::datatype_conversion::{
    ConversionBehaviorContract, ConversionRepresentation, ConversionSignature,
};
use cem_ql::datatype_conversion::{
    ConversionInput, ConversionStop, ConversionValue, ConverterBinding,
    DatatypeConversionResultAdapter, RegisteredDatatypeConverter,
};
fn sig() -> ConversionSignature {
    ConversionSignature {
        kind: DatatypeKind::Scalar,
        input: ConversionRepresentation::Lexical,
        output: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
    }
}
fn text(body: &str) -> String {
    query_declaration(body).replace("datatype-validation", "datatype-conversion")
}
fn result_adapter() -> DatatypeConversionResultAdapter {
    let schema = source(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    DatatypeConversionResultAdapter::new(
        Arc::new(ValueContracts::compile(&[schema], Default::default()).unwrap()),
        ContractName::new(CEM_SCHEMA_URI, "datatype-conversion-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap()
}
fn contract(text: &str, signature: ConversionSignature) -> ConversionBehaviorContract {
    let source = source(text);
    ConversionBehaviorContract::compile(
        &source,
        &node(&source, "behavior"),
        signature,
        ContractName::new(CEM_SCHEMA_URI, "datatype-conversion-result"),
    )
    .unwrap()
}
fn descriptor(body: &str) -> Arc<ExecutableDatatype> {
    query_descriptor(body, sig())
}
fn query_descriptor(body: &str, signature: ConversionSignature) -> Arc<ExecutableDatatype> {
    let mut declaration = text(body);
    if signature.kind == DatatypeKind::Node {
        declaration=declaration.replace("@name=value @type=schema:string @source=value @required=true @cardinality=one", "@name=value @type=schema:node @source=value @required=true @cardinality=zero-or-more")
            .replace("@name=value @type=string @required=true @cardinality=one", "@name=value @type=node @required=true @cardinality=zero-or-more");
    }
    compiled_with(signature, |source| {
        RegisteredDatatypeConverter::from_query(
            source.clone(),
            contract(&declaration, signature),
            result_adapter(),
        )
        .unwrap()
    })
}
fn compiled_with(
    signature: ConversionSignature,
    register: impl FnOnce(&DatatypeSource) -> RegisteredDatatypeConverter,
) -> Arc<ExecutableDatatype> {
    let kind = if signature.kind == DatatypeKind::Node {
        "node"
    } else {
        "scalar"
    };
    let (mut host, sources) = types_fixture(&format!("{{type @name=sample @kind={kind}}}"));
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            signature.kind,
            signature.output,
        ))
        .unwrap();
    implementations
        .select_converter(
            sources[0].clone(),
            ConverterBinding::Ready(register(&sources[0])),
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
    Arc::new(compiled(&result, &sources[0]).clone())
}
fn request() -> ConversionInput {
    ConversionInput {
        value: ConversionValue::Lexical(cem_ml::schema::datatype_contracts::LexicalInput::new(
            Arc::from("original"),
            Default::default(),
        )),
        candidate: vec![],
        fallback: DiagnosticAttribution {
            uri: Some("input.cem".into()),
            ..Default::default()
        },
    }
}
#[test]
fn selected_conversion_checks_results_and_is_never_used_by_validation() {
    for (body, converts) in [
        (r#"{status: "converted", value: value, diagnostics: {code: "selected", severity: "info", message: "converted", source: datatype}}"#, true),
        (r#"{status: "converted", value: 1, diagnostics: ()}"#, false),
        ("1 / 0", false),
    ] {
        let original = source(&text(body).replace("@function=check-body", "@function={#chosen}"));
        let (selected, mut budget) = selected_function(&original);
        let contract = ConversionBehaviorContract::compile_selected(&selected, sig(), ContractName::new(CEM_SCHEMA_URI, "datatype-conversion-result"), &mut budget).unwrap();
        let descriptor = compiled_with(sig(), |source| RegisteredDatatypeConverter::from_query(source.clone(), contract, result_adapter()).unwrap());
        let control = OperationControl::default();
        let runtime = ValidationRuntime { control: &control, scope: ROOT_EXECUTION_SCOPE_ID, query: Default::default() };
        assert_eq!(validate_descriptor(&descriptor, vec![Item::Atomic(AtomValue::String("original".into()))]).accepted, Some(true));
        let result = descriptor.convert(&request(), &runtime, Default::default());
        assert_eq!(result.accepted, converts.then_some(true), "{result:?}");
        if converts {
            assert_eq!(result.value.unwrap(), vec![Item::Atomic(AtomValue::String("original".into()))]);
            assert!(result.diagnostics[0].node.is_some());
        }
    }
}
#[test]
fn query_conversion_retains_fixed_roles_and_original_diagnostics() {
    let descriptor = descriptor(
        r#"{status: "converted", value: value, diagnostics: {code:"note",severity:"warning",message:"converted",source: datatype}}"#,
    );
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let mut runtime = runtime;
    runtime.query.policy_bindings.insert(
        "value".into(),
        ItemStream::once(Item::Atomic(AtomValue::String("ambient".into()))),
    );
    let result = descriptor.convert(&request(), &runtime, Default::default());
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(
        result.value.unwrap(),
        vec![Item::Atomic(AtomValue::String("original".into()))]
    );
    assert!(result.diagnostics[0].node.is_some());
    for body in [
        r#"{status:"converted",diagnostics:()}"#,
        r#"{status:"rejected",value:(),diagnostics:()}"#,
        r#"{status:"pending",diagnostics:()}"#,
        r#"{status:"converted",value:1,diagnostics:()}"#,
        "()",
        "1 / 0",
    ] {
        let result = descriptor_for(body).convert(&request(), &runtime, Default::default());
        assert!(result.accepted.is_none(), "{body}: {result:?}");
        assert!(result.stopped.is_some());
    }
    let rejected = descriptor_for(r#"{status:"rejected",diagnostics:()}"#).convert(
        &request(),
        &runtime,
        Default::default(),
    );
    assert_eq!(rejected.accepted, Some(false));
    assert!(rejected.value.is_none());
}
fn descriptor_for(body: &str) -> Arc<ExecutableDatatype> {
    descriptor(body)
}
#[test]
fn conversion_profiles_require_exact_source_signatures_and_result_contracts() {
    for bad in [
        text("()").replace("@returns=datatype-conversion-result", "@returns=object"),
        text("()").replace(
            "@execution=datatype-conversion",
            "@execution=datatype-validation",
        ),
        text("()").replace("@name=candidate @type=node", "@name=candidate @type=object"),
    ] {
        let source = source(&bad);
        assert!(ConversionBehaviorContract::compile(
            &source,
            &node(&source, "behavior"),
            sig(),
            ContractName::new(CEM_SCHEMA_URI, "datatype-conversion-result")
        )
        .is_err());
    }
    let (_, sources) = types_fixture("{type @name=sample @kind=scalar}");
    assert!(RegisteredDatatypeConverter::from_query(
        sources[0].clone(),
        contract(&text("secret"), sig()),
        result_adapter()
    )
    .is_err());
    let descriptor = descriptor_for(r#"{status:"converted",value: value,diagnostics:()}"#);
    let control = OperationControl::new(AbortSignal::new());
    control.cancel_root(None, None).unwrap();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    assert!(matches!(
        descriptor
            .convert(&request(), &runtime, Default::default())
            .stopped,
        Some(ConversionStop::Control(_))
    ));
}

#[test]
fn node_query_conversion_preserves_empty_output_and_input_view_boundaries() {
    let signature = ConversionSignature {
        kind: DatatypeKind::Node,
        input: ConversionRepresentation::Values(ValueRepresentation::Nodes),
        output: ValueRepresentation::Nodes,
        candidate: CandidateRequirement::Optional,
    };
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let owner = source(&declaration(false));
    let node = native(&node(&owner, "type"));
    let request = ConversionInput {
        value: ConversionValue::Values(vec![node.clone()]),
        candidate: vec![],
        fallback: Default::default(),
    };
    let identity = query_descriptor(
        r#"{status:"converted",value: value,diagnostics:()}"#,
        signature,
    )
    .convert(&request, &runtime, Default::default());
    assert_eq!(identity.accepted, Some(true), "{identity:?}");
    assert_eq!(identity.value.unwrap()[0].identity(), node.identity());
    let empty = query_descriptor(r#"{status:"converted",value:(),diagnostics:()}"#, signature)
        .convert(&request, &runtime, Default::default());
    assert_eq!(empty.accepted, Some(true));
    assert!(empty.value.unwrap().is_empty());
    let wrong = query_descriptor(
        r#"{status:"converted",value: datatype,diagnostics:()}"#,
        signature,
    )
    .convert(&request, &runtime, Default::default());
    assert!(matches!(
        wrong.stopped,
        Some(ConversionStop::InvalidOutput("native-view-identity"))
    ));
    let mut limits = cem_ql::datatype_conversion::ConversionLimits::default();
    limits.max_output_values = 0;
    let bounded = query_descriptor(
        r#"{status:"converted",value: value,diagnostics:()}"#,
        signature,
    )
    .convert(&request, &runtime, limits);
    assert_eq!(bounded.accepted, None);
}

#[derive(Debug, Clone)]
struct SourceCallback {
    calls: Arc<AtomicUsize>,
    outcome: RuleExecution,
    cancel: bool,
}
impl cem_ql::datatype_conversion::SourceDatatypeConverter for SourceCallback {
    fn convert(&self, call: cem_ql::datatype_conversion::ConversionCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        assert!(matches!(call.value,ConversionValue::Lexical(v) if &*v.text == "original"));
        if self.cancel {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        self.outcome.clone()
    }
}
#[test]
fn source_conversion_is_explicit_and_preserves_incomplete_states_and_reports() {
    let declaration = declaration(false).replace("datatype-validation", "datatype-conversion");
    let contract = contract(&declaration, sig());
    let (_, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let calls = Arc::new(AtomicUsize::new(0));
    let implementation = SourceCallback {
        calls: calls.clone(),
        outcome: RuleExecution::Pending(vec![]),
        cancel: false,
    };
    assert!(RegisteredDatatypeConverter::from_source(
        sources[0].clone(),
        "different",
        contract.clone(),
        result_adapter(),
        implementation.clone()
    )
    .is_err());
    let mut malformed = query(r#"{status:"converted",value:1,diagnostics:()}"#);
    malformed.diagnostics.push(cem_ml::diagnostics::Diagnostic {
        code: "already.emitted".into(),
        message: "keep me".into(),
        severity: cem_ml::diagnostics::Severity::Warning,
        ..Default::default()
    });
    for (outcome, cancel, expected) in [
        (RuleExecution::Pending(vec![]), false, 0),
        (RuleExecution::Unavailable(vec![]), false, 1),
        (
            RuleExecution::Complete(query(
                r#"{status:"converted",value:"ignored",diagnostics:()}"#,
            )),
            true,
            2,
        ),
        (RuleExecution::Complete(malformed), false, 3),
        (
            RuleExecution::Complete(query(
                r#"{status:"converted",value:"converted",diagnostics:{code:"note",severity:"warning",message:"fallback"}}"#,
            )),
            false,
            4,
        ),
    ] {
        let implementation = SourceCallback {
            outcome,
            cancel,
            ..implementation.clone()
        };
        let descriptor = compiled_with(sig(), |source| {
            RegisteredDatatypeConverter::from_source(
                source.clone(),
                "urn:test:validate",
                contract.clone(),
                result_adapter(),
                implementation,
            )
            .unwrap()
        });
        let before = calls.load(Ordering::SeqCst);
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let mut input = request();
        let lexical_source = native(&node(&source(&declaration), "type"))
            .source_map()
            .unwrap();
        if let ConversionValue::Lexical(value) = &mut input.value {
            value.source = lexical_source.clone();
        }
        let result = descriptor.convert(&input, &runtime, Default::default());
        assert_eq!(calls.load(Ordering::SeqCst), before + 1);
        match expected {
            0 => assert!(matches!(result.stopped, Some(ConversionStop::Pending))),
            1 => assert!(matches!(result.stopped, Some(ConversionStop::Unavailable))),
            2 => assert!(matches!(result.stopped, Some(ConversionStop::Control(_)))),
            3 => {
                assert_eq!(result.accepted, None);
                assert_eq!(result.diagnostics[0].code, "already.emitted");
            }
            _ => {
                assert_eq!(result.accepted, Some(true));
                assert_eq!(result.diagnostics[0].uri.as_deref(), Some("input.cem"));
                assert_eq!(result.diagnostics[0].source_map, Some(lexical_source));
                assert!(result.validation.is_some());
            }
        }
    }
}

#[test]
fn conversion_adapter_reads_native_records_once_and_checks_closed_output() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    #[derive(Debug)]
    struct Record(Arc<AtomicUsize>);
    impl QueryItemView for Record {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.conversion"
        }
        fn identity(&self) -> String {
            "fixture".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
            let first = self.0.fetch_add(1, Ordering::SeqCst) == 0;
            Some(vec![
                (
                    "status".into(),
                    vec![Item::Atomic(AtomValue::String(
                        if first { "converted" } else { "rejected" }.into(),
                    ))],
                ),
                ("value".into(), vec![]),
                ("diagnostics".into(), vec![]),
            ])
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = result_adapter();
    let result = adapter
        .consume(
            ItemStream::once(Item::native(Record(calls.clone()))),
            &Default::default(),
            ValueRepresentation::List(ScalarRepresentation::Integer),
            Default::default(),
        )
        .unwrap();
    assert!(result.value.unwrap().is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for body in [
        r#"{status:"converted",value:(),diagnostics:(),typo:1}"#,
        r#"{status:"converted",value:{x:1},diagnostics:()}"#,
        r#"{status:"converted",value:(),diagnostics:{code:"x",severity:"typo",message:"bad"}}"#,
    ] {
        assert!(adapter
            .consume(
                query(body),
                &Default::default(),
                ValueRepresentation::Nodes,
                Default::default()
            )
            .is_err());
    }
}

#[test]
fn conversion_profile_is_admitted_and_numeric_values_remain_typed() {
    use cem_ml::schema::document_model::{compile_schema_document_model, validate_document_model};
    let model = compile_schema_document_model(
        CEM_SCHEMA_URI,
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    for text in [
        text(r#"{status:"converted",value: value,diagnostics:()}"#),
        declaration(false).replace("datatype-validation", "datatype-conversion"),
    ] {
        let source = source(&text);
        let diagnostics = validate_document_model(source.schema.document(), &model);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }
    let signature = ConversionSignature {
        output: ValueRepresentation::Scalar(ScalarRepresentation::Integer),
        ..sig()
    };
    let descriptor = query_descriptor(r#"{status:"converted",value:42,diagnostics:()}"#, signature);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let result = descriptor.convert(&request(), &runtime, Default::default());
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(
        result.value.unwrap(),
        vec![Item::Atomic(AtomValue::Integer(42.into()))]
    );
    let adapter = result_adapter();
    assert!(adapter
        .consume(
            query(r#"{status:"converted",value:(1,2),diagnostics:()}"#),
            &Default::default(),
            ValueRepresentation::List(ScalarRepresentation::Integer),
            Default::default()
        )
        .is_ok());
}
