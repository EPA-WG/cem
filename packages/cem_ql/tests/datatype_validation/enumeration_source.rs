use super::*;
use cem_ml::schema::datatype_enumeration::{ConstantBehaviorContract, EqualityBehaviorContract};
use cem_ql::datatype_compilation::compile_datatypes_with_runtime;
use cem_ql::datatype_enumeration::*;

fn profile(equality: bool, body: &str) -> String {
    profile_impl(equality, Some(body))
}
fn profile_impl(equality: bool, body: Option<&str>) -> String {
    let execution = if equality {
        "datatype-equality"
    } else {
        "datatype-constant"
    };
    let roles = if equality {
        vec![
            ("left", "string"),
            ("right", "string"),
            ("datatype", "node"),
        ]
    } else {
        vec![
            ("value", "string"),
            ("datatype", "node"),
            ("candidate", "node"),
        ]
    };
    let inputs = roles
        .iter()
        .map(|(name, ty)| {
            format!("{{input-binding @name={name} @type={ty} @source={name} @required=true}}")
        })
        .collect::<String>();
    let params = roles
        .iter()
        .map(|(name, ty)| format!("{{param @name={name} @type={ty} @required=true}}"))
        .collect::<String>();
    let (implementation, function) = match body {
        Some(body) => ("@implementation=function @function=run", format!("{{function @name=run @returns={execution}-result | {params} {{body | {{$ {body} }}}} }}")),
        None => ("@implementation=engine @primitive=urn:test:scalar", String::new()),
    };
    format!("@ns schema = \"https://cem.dev/ns/schema/1\"\n@default schema\n{{schema @name=example @version=1.0.0 @namespace=urn:test | {{behaviors | {{behavior @name=op {implementation} @execution={execution} | {{inputs | {inputs}}} {{result @type=schema:{execution}-result}} {function} }} }} }}")
}
fn contracts() -> Arc<ValueContracts> {
    Arc::new(
        ValueContracts::compile(
            &[source(
                cem_ml::schema::package_sources::builtin_schema_package_source("schema")
                    .unwrap()
                    .schema_source,
            )],
            Default::default(),
        )
        .unwrap(),
    )
}
fn equality(src: &DatatypeSource, body: &str) -> RegisteredScalarEquality {
    let text = source(&profile(true, body));
    let contract = EqualityBehaviorContract::compile(
        &text,
        &node(&text, "behavior"),
        ScalarRepresentation::String,
        ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
    )
    .unwrap();
    RegisteredScalarEquality::from_query(
        src.clone(),
        contract,
        DatatypeEqualityResultAdapter::new(
            contracts(),
            ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
            ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn interpreter(src: &DatatypeSource, body: &str) -> RegisteredConstantInterpreter {
    let text = source(&profile(false, body));
    let contract = ConstantBehaviorContract::compile(
        &text,
        &node(&text, "behavior"),
        ScalarRepresentation::String,
        ContractName::new(CEM_SCHEMA_URI, "datatype-constant-result"),
    )
    .unwrap();
    RegisteredConstantInterpreter::from_query(
        src.clone(),
        contract,
        DatatypeConstantResultAdapter::new(
            contracts(),
            ContractName::new(CEM_SCHEMA_URI, "datatype-constant-result"),
            ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn run(eq: &str, constant: &str) -> cem_ml::schema::datatype_contracts::DatatypeCompilation {
    run_registered(
        |s| equality(s, eq),
        |s| interpreter(s, constant),
        &OperationControl::default(),
        Default::default(),
    )
}
fn run_registered(
    eq: impl FnOnce(&DatatypeSource) -> RegisteredScalarEquality,
    constant: impl FnOnce(&DatatypeSource) -> RegisteredConstantInterpreter,
    control: &OperationControl,
    limits: ConstantPreparationLimits,
) -> cem_ml::schema::datatype_contracts::DatatypeCompilation {
    let (mut host, sources) = types_fixture("{type @name=sample @kind=scalar @values=\" 3  5 \"}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(ScalarRepresentation::String),
        ))
        .unwrap();
    implementations
        .select_equality(sources[0].clone(), EqualityBinding::Ready(eq(&sources[0])))
        .unwrap();
    implementations
        .select_constant_interpreter(
            sources[0].clone(),
            ConstantBinding::Ready(constant(&sources[0])),
        )
        .unwrap();
    let mut runtime = ValidationRuntime {
        control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    for name in ["value", "left", "right"] {
        runtime
            .query
            .policy_bindings
            .insert(name.into(), query("\"ambient\""));
    }
    compile_datatypes_with_runtime(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        limits,
    )
}
const EQ: &str = "{equal: left == right, diagnostics: ()}";
const CONSTANT: &str = "{status: \"prepared\", value: value, diagnostics: {code: \"token\", severity: \"info\", message: value, source: candidate}}";
#[test]
fn query_enumeration_retains_tokens_and_uses_explicit_equality() {
    let result = run(EQ, CONSTANT);
    assert!(result.is_ready(), "{:?}", result.issues);
    let descriptor = result.contracts[0]
        .as_any()
        .downcast_ref::<ExecutableDatatype>()
        .unwrap();
    assert_eq!(descriptor.enumerations()[0].constants()[0].token.span, 1..2);
    assert_eq!(
        descriptor.enumerations()[0].constants()[0].token.text(),
        "3"
    );
    assert!(result.diagnostics.iter().all(|d| d.node.is_some()));
    assert_eq!(
        validate_descriptor(
            descriptor,
            vec![Item::Atomic(AtomValue::String("3".into()))]
        )
        .accepted,
        Some(true)
    );
    assert_eq!(
        validate_descriptor(
            descriptor,
            vec![Item::Atomic(AtomValue::String("003".into()))]
        )
        .accepted,
        Some(false)
    );
}
#[test]
fn query_constant_malformed_results_never_activate_contract() {
    for body in [
        "()",
        "{status: \"prepared\", diagnostics: ()}",
        "{status: \"prepared\", value: (), diagnostics: ()}",
        "{status: \"prepared\", value: 3, diagnostics: ()}",
        "{status: \"rejected\", value: (), diagnostics: ()}",
        "{status: \"pending\", diagnostics: ()}",
        "{status: \"prepared\", value: value, typo: true, diagnostics: ()}",
        "1 / 0",
    ] {
        let result = run(EQ, body);
        assert!(!result.is_ready(), "{body}");
        assert!(!result.issues.is_empty());
    }
}

#[test]
fn equality_requires_boolean_and_keeps_diagnostics_independent() {
    for (body, expected) in [
        ("{equal: false, diagnostics: ()}", Some(false)),
        ("{equal: true, diagnostics: {code: \"note\", severity: \"error\", message: \"detail\", source: datatype}}", Some(true)),
        ("()", None), ("true", None), ("{diagnostics: ()}", None),
        ("{equal: (), diagnostics: ()}", None), ("{equal: \"true\", diagnostics: ()}", None),
        ("{equal: true, extra: true, diagnostics: ()}", None), ("1 / 0", None),
    ] {
        let result = run(body, CONSTANT);
        assert!(result.is_ready(), "{:?}", result.issues);
        let descriptor = result.contracts[0].as_any().downcast_ref::<ExecutableDatatype>().unwrap();
        let output = validate_descriptor(descriptor, query("\"3\"").items);
        assert_eq!(output.accepted, expected, "{body}: {output:?}");
        if expected.is_none() { assert!(output.stopped.is_some()); }
    }
}

#[test]
fn enumeration_profiles_enforce_roles_owner_and_closed_query_bindings() {
    use cem_ml::schema::document_model::{compile_schema_document_model, validate_document_model};
    let model = compile_schema_document_model(
        CEM_SCHEMA_URI,
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    for equality in [false, true] {
        for body in [None, Some(if equality { EQ } else { CONSTANT })] {
            let src = source(&profile_impl(equality, body));
            let diagnostics = validate_document_model(src.schema.document(), &model);
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
        }
        let valid = profile(equality, if equality { EQ } else { CONSTANT });
        for invalid in [
            valid.replace("@source=datatype", "@source=value"),
            valid.replace("@type=node", "@type=object"),
            valid.replace("@required=true", "@required=false"),
            valid.replace("@execution=datatype-", "@execution=unsupported-"),
            valid.replace("@returns=datatype-", "@returns=unsupported-"),
        ] {
            let src = source(&invalid);
            let rejected = if equality {
                EqualityBehaviorContract::compile(
                    &src,
                    &node(&src, "behavior"),
                    ScalarRepresentation::String,
                    ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
                )
                .is_err()
            } else {
                ConstantBehaviorContract::compile(
                    &src,
                    &node(&src, "behavior"),
                    ScalarRepresentation::String,
                    ContractName::new(CEM_SCHEMA_URI, "datatype-constant-result"),
                )
                .is_err()
            };
            assert!(rejected, "{invalid}");
        }
    }
    let a = source(&profile(true, EQ));
    let b = source(&profile(true, EQ));
    assert!(EqualityBehaviorContract::compile(
        &a,
        &node(&b, "behavior"),
        ScalarRepresentation::String,
        ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result")
    )
    .is_err());
    let (_, sources) = types_fixture("{type @name=sample @kind=scalar}");
    let a = source(&profile(true, "{equal: ambient, diagnostics: ()}"));
    let contract = EqualityBehaviorContract::compile(
        &a,
        &node(&a, "behavior"),
        ScalarRepresentation::String,
        ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
    )
    .unwrap();
    assert!(
        RegisteredScalarEquality::from_query(sources[0].clone(), contract, equality_adapter())
            .is_err()
    );
}
fn equality_adapter() -> DatatypeEqualityResultAdapter {
    DatatypeEqualityResultAdapter::new(
        contracts(),
        ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap()
}
fn constant_adapter() -> DatatypeConstantResultAdapter {
    DatatypeConstantResultAdapter::new(
        contracts(),
        ContractName::new(CEM_SCHEMA_URI, "datatype-constant-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap()
}
#[derive(Debug, Clone)]
struct Callback {
    outcome: RuleExecution,
    calls: Arc<AtomicUsize>,
    cancel: bool,
}
impl SourceConstantInterpreter for Callback {
    fn interpret(&self, call: ConstantCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let candidate = cem_ql::eval::retained_cem_node(call.candidate).unwrap();
        assert!(Arc::ptr_eq(
            candidate.owner().ast_owner(),
            call.token.source.document()
        ));
        assert_eq!(candidate.node_id(), call.token.source.node_id());
        assert!(matches!(
            (call.token.text(), call.token.span.clone()),
            ("3", std::ops::Range { start: 1, end: 2 })
                | ("5", std::ops::Range { start: 4, end: 5 })
        ));
        if self.cancel {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        self.outcome.clone()
    }
}
impl SourceScalarEquality for Callback {
    fn compare(&self, call: EqualityCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        if self.cancel {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        self.outcome.clone()
    }
}
fn native_constant(src: &DatatypeSource, callback: Callback) -> RegisteredConstantInterpreter {
    let text = source(&profile_impl(false, None));
    let contract = ConstantBehaviorContract::compile(
        &text,
        &node(&text, "behavior"),
        ScalarRepresentation::String,
        ContractName::new(CEM_SCHEMA_URI, "datatype-constant-result"),
    )
    .unwrap();
    assert!(RegisteredConstantInterpreter::from_source(
        src.clone(),
        "wrong",
        contract.clone(),
        constant_adapter(),
        callback.clone()
    )
    .is_err());
    RegisteredConstantInterpreter::from_source(
        src.clone(),
        "urn:test:scalar",
        contract,
        constant_adapter(),
        callback,
    )
    .unwrap()
}
fn native_equality(src: &DatatypeSource, callback: Callback) -> RegisteredScalarEquality {
    let text = source(&profile_impl(true, None));
    let contract = EqualityBehaviorContract::compile(
        &text,
        &node(&text, "behavior"),
        ScalarRepresentation::String,
        ContractName::new(CEM_SCHEMA_URI, "datatype-equality-result"),
    )
    .unwrap();
    assert!(RegisteredScalarEquality::from_source(
        src.clone(),
        "wrong",
        contract.clone(),
        equality_adapter(),
        callback.clone()
    )
    .is_err());
    RegisteredScalarEquality::from_source(
        src.clone(),
        "urn:test:scalar",
        contract,
        equality_adapter(),
        callback,
    )
    .unwrap()
}
#[test]
fn source_enumeration_preserves_incomplete_states_cancellation_and_reports() {
    for (execution, cancel, code) in [
        (
            RuleExecution::Pending(vec![]),
            false,
            "datatype-constant-pending",
        ),
        (
            RuleExecution::Unavailable(vec![]),
            false,
            "datatype-constant-unavailable",
        ),
        (
            RuleExecution::Complete(query("{status: \"rejected\", diagnostics: ()}")),
            false,
            "datatype-constant-rejected",
        ),
        (
            RuleExecution::Complete(query("()")),
            false,
            "datatype-constant-failed",
        ),
        (
            RuleExecution::Complete(query(
                "{status: \"prepared\", value: \"3\", diagnostics: ()}",
            )),
            true,
            "datatype-constant-control",
        ),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let callback = Callback {
            outcome: execution,
            calls: calls.clone(),
            cancel,
        };
        let result = run_registered(
            |s| equality(s, EQ),
            |s| native_constant(s, callback),
            &OperationControl::default(),
            Default::default(),
        );
        assert!(!result.is_ready());
        assert_eq!(result.issues[0].code, code);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    for (execution, cancel) in [
        (RuleExecution::Pending(vec![]), false),
        (RuleExecution::Unavailable(vec![]), false),
        (RuleExecution::Complete(query("()")), false),
        (
            RuleExecution::Complete(query("{equal: true, diagnostics: ()}")),
            true,
        ),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let callback = Callback {
            outcome: execution,
            calls: calls.clone(),
            cancel,
        };
        let result = run_registered(
            |s| native_equality(s, callback),
            |s| interpreter(s, CONSTANT),
            &OperationControl::default(),
            Default::default(),
        );
        assert!(result.is_ready());
        let output = validate_descriptor(
            result.contracts[0]
                .as_any()
                .downcast_ref::<ExecutableDatatype>()
                .unwrap(),
            query("\"3\"").items,
        );
        assert_eq!(output.accepted, None);
        assert!(output.stopped.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    let mut stream = query("{equal: \"bad\", diagnostics: ()}");
    stream.diagnostics.push(cem_ml::diagnostics::Diagnostic {
        code: "already.emitted".into(),
        ..Default::default()
    });
    let callback = Callback {
        outcome: RuleExecution::Complete(stream),
        calls: Arc::new(AtomicUsize::new(0)),
        cancel: false,
    };
    let result = run_registered(
        |s| native_equality(s, callback),
        |s| interpreter(s, CONSTANT),
        &OperationControl::default(),
        Default::default(),
    );
    let output = validate_descriptor(
        result.contracts[0]
            .as_any()
            .downcast_ref::<ExecutableDatatype>()
            .unwrap(),
        query("\"3\"").items,
    );
    assert!(output
        .enumeration_diagnostics
        .iter()
        .any(|d| d.code == "already.emitted"));
}
#[test]
fn enumeration_result_adapters_enforce_scalar_types_and_cumulative_budgets() {
    let adapter = constant_adapter();
    let result = adapter
        .consume(
            query("{status: \"prepared\", value: 3, diagnostics: ()}"),
            &Default::default(),
            ScalarRepresentation::Integer,
            Default::default(),
        )
        .unwrap();
    assert_eq!(
        result.value,
        Some(Item::Atomic(AtomValue::Integer(3.into())))
    );
    for body in [
        "{status: \"prepared\", value: (3, 4), diagnostics: ()}",
        "{status: \"prepared\", value: {n: 3}, diagnostics: ()}",
        "{status: \"prepared\", value: \"3\", diagnostics: ()}",
    ] {
        assert!(adapter
            .consume(
                query(body),
                &Default::default(),
                ScalarRepresentation::Integer,
                Default::default()
            )
            .is_err());
    }
    let mut limits = ConstantPreparationLimits::default();
    limits.validation.max_diagnostics = 1;
    let result = run_registered(
        |s| equality(s, EQ),
        |s| interpreter(s, CONSTANT),
        &OperationControl::default(),
        limits,
    );
    assert!(!result.is_ready());
    let result = run("{equal: false, diagnostics: ()}", CONSTANT);
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let output = result.contracts[0]
        .as_any()
        .downcast_ref::<ExecutableDatatype>()
        .unwrap()
        .validate(
            &ValidationInput {
                value: query("\"3\"").items,
                candidate: vec![],
                fallback: Default::default(),
            },
            &runtime,
            ValidationLimits {
                max_comparisons: 1,
                ..Default::default()
            },
        );
    assert_eq!(output.accepted, None);
    assert_eq!(output.comparisons, 1);
}

#[test]
fn enumeration_adapters_observe_native_records_once_and_count_emitted_reports() {
    use cem_ql::eval::{QueryItemView, QueryItemViewKind};
    #[derive(Debug)]
    struct Record {
        equality: bool,
        calls: Arc<AtomicUsize>,
    }
    impl QueryItemView for Record {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.enumeration"
        }
        fn identity(&self) -> String {
            "result".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
            let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
            let mut fields = vec![("diagnostics".into(), vec![])];
            if self.equality {
                fields.push((
                    "equal".into(),
                    vec![Item::Atomic(AtomValue::Boolean(first))],
                ));
            } else {
                fields.push((
                    "status".into(),
                    vec![Item::Atomic(AtomValue::String(
                        if first { "prepared" } else { "rejected" }.into(),
                    ))],
                ));
                fields.push(("value".into(), query("3").items));
            }
            Some(fields)
        }
    }
    for equality in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let stream = ItemStream::once(Item::native(Record {
            equality,
            calls: calls.clone(),
        }));
        if equality {
            assert!(
                equality_adapter()
                    .consume(stream, &Default::default(), Default::default())
                    .unwrap()
                    .equal
            );
        } else {
            assert!(constant_adapter()
                .consume(
                    stream,
                    &Default::default(),
                    ScalarRepresentation::Integer,
                    Default::default()
                )
                .unwrap()
                .value
                .is_some());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    let mut stream = query("{equal: true, diagnostics: {code: \"detail\", severity: \"warning\", message: \"detail\"}}");
    stream.diagnostics.push(Default::default());
    assert!(equality_adapter()
        .consume(
            stream,
            &Default::default(),
            ValidationLimits {
                max_diagnostics: 1,
                ..Default::default()
            }
        )
        .is_err());
    assert!(equality_adapter().consume(query("{equal: true, diagnostics: {code: \"detail\", severity: \"bad\", message: \"detail\"}}"), &Default::default(), Default::default()).is_err());
}

#[test]
fn source_profiles_prepare_once_per_token_and_validate_without_conversion() {
    let preparations = Arc::new(AtomicUsize::new(0));
    let comparisons = Arc::new(AtomicUsize::new(0));
    let result = run_registered(
        |s| {
            native_equality(
                s,
                Callback {
                    outcome: RuleExecution::Complete(query("{equal: true, diagnostics: ()}")),
                    calls: comparisons.clone(),
                    cancel: false,
                },
            )
        },
        |s| {
            native_constant(
                s,
                Callback {
                    outcome: RuleExecution::Complete(query(
                        "{status: \"prepared\", value: \"3\", diagnostics: ()}",
                    )),
                    calls: preparations.clone(),
                    cancel: false,
                },
            )
        },
        &OperationControl::default(),
        Default::default(),
    );
    assert!(result.is_ready(), "{:?}", result.issues);
    assert_eq!(preparations.load(Ordering::SeqCst), 2);
    assert_eq!(comparisons.load(Ordering::SeqCst), 0);
    let descriptor = result.contracts[0]
        .as_any()
        .downcast_ref::<ExecutableDatatype>()
        .unwrap();
    assert_eq!(
        validate_descriptor(descriptor, query("\"3\"").items).accepted,
        Some(true)
    );
    assert_eq!(comparisons.load(Ordering::SeqCst), 1);
    assert_eq!(preparations.load(Ordering::SeqCst), 2);
}
