use cem_ml::{
    events::cem::CemEventNormalizer,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    scheduler::AbortSignal,
    schema::{
        datatype_registry::DatatypeKind,
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationSignature, ValueRepresentation,
        },
        declaration_references::SchemaDeclarationNode,
        machine::CemSchemaMachine,
        registry::CEM_SCHEMA_URI,
        value_contracts::{ContractName, ValueContractSource, ValueContracts},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    datatype_results::{DatatypeResultAdapter, DiagnosticAttribution},
    datatype_validation::{
        validate_rules, DatatypeValidationRegistry, LegacyAcceptance, NativeDatatypeValidator,
        RuleExecution, ValidationCall, ValidationInput, ValidationLimits, ValidationRuntime,
    },
    eval::{AtomValue, Item, ItemStream, RetainedCemNode},
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

fn source(text: &str) -> ValueContractSource {
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    assert!(captured.document().diagnostics.is_empty());
    let root = captured
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    ValueContractSource::new(
        SchemaDeclarationNode::new(captured.document().clone(), root).unwrap(),
        CEM_SCHEMA_URI,
        BTreeMap::from([("schema".into(), CEM_SCHEMA_URI.into())]),
    )
    .with_captured_names(captured)
    .unwrap()
}
fn node(source: &ValueContractSource, name: &str) -> SchemaDeclarationNode {
    source
        .schema
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => {
                SchemaDeclarationNode::new(source.schema.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn native(node: &SchemaDeclarationNode) -> Item {
    let tree = RetainedCemTree::from_shared(
        node.document().clone(),
        "original.cem",
        "",
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    RetainedCemNode::new(tree, node.node_id())
        .unwrap()
        .query_item()
}
fn declaration(required: bool) -> String {
    format!(
        r#"@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{{schema @name=example @namespace="urn:example" @version="1.0.0" |
 {{types | {{type @name=sample @kind=scalar}}}}
 {{behaviors | {{behavior @name=check @implementation=engine @primitive="urn:test:validate" @execution=datatype-validation |
  {{inputs |
   {{input-binding @name=value @type=schema:string @source=value @required=true @cardinality=one}}
   {{input-binding @name=datatype @type=schema:node @source=datatype @required=true @cardinality=one}}
   {{input-binding @name=candidate @type=schema:node @source=candidate @required={required} @cardinality={card}}}
  }}
  {{result @type=schema:datatype-validation-result}}
 }}}}
}}"#,
        card = if required { "one" } else { "zero-or-one" }
    )
}
fn signature(required: bool) -> ValidationSignature {
    ValidationSignature {
        kind: DatatypeKind::Scalar,
        value: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: if required {
            CandidateRequirement::Required
        } else {
            CandidateRequirement::Optional
        },
        result: ResultRepresentation::Accepted(ContractName::new(
            CEM_SCHEMA_URI,
            "datatype-validation-result",
        )),
    }
}
fn contract(source: &ValueContractSource, required: bool) -> DatatypeBehaviorContract {
    DatatypeBehaviorContract::compile(source, &node(source, "behavior"), signature(required))
        .unwrap()
}
fn adapter() -> DatatypeResultAdapter {
    let source = source(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[source], Default::default()).unwrap()),
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap()
}
fn query(s: &str) -> ItemStream {
    evaluate(
        &compile(s, &CompileContext::default()).unwrap(),
        &EvaluationContext::default(),
    )
}
#[derive(Debug)]
struct Check {
    calls: Arc<AtomicUsize>,
    accepted: bool,
}
impl NativeDatatypeValidator for Check {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(cem_ql::eval::retained_cem_node(call.datatype).is_some());
        assert_eq!(
            call.value,
            &[Item::Atomic(AtomValue::String("same".into()))]
        );
        RuleExecution::Complete(query(if self.accepted {
            "{ accepted: true, diagnostics: () }"
        } else {
            "{ accepted: false, diagnostics: () }"
        }))
    }
}
#[test]
fn signatures_check_original_roles_types_cardinality_and_closed_source_forms() {
    let good = source(&declaration(false));
    contract(&good, false);
    for text in [
        declaration(false).replace("@name=value", "@name=renamed"),
        declaration(false).replace("@source=value", "@source=candidate"),
        declaration(false).replace("@type=schema:node", "@type=schema:object"),
        declaration(false).replace("@cardinality=zero-or-one", "@cardinality=one"),
        declaration(false).replace("@required=false", "@required=true"),
        declaration(false).replace("@cardinality=one", "@cardinality=zero-or-more"),
        declaration(false).replace(
            "@execution=datatype-validation",
            "@execution=ast-validation",
        ),
    ] {
        let bad = source(&text);
        assert!(
            DatatypeBehaviorContract::compile(&bad, &node(&bad, "behavior"), signature(false))
                .is_err(),
            "{text}"
        );
    }
}
#[test]
fn authority_is_exact_owner_and_behavior_and_optional_candidate_keeps_its_slot() {
    let original = source(&declaration(false));
    let unrelated = source(&declaration(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = DatatypeValidationRegistry::default();
    registry
        .register_native(
            "urn:test:validate",
            contract(&original, false),
            adapter(),
            None,
            Check {
                calls: calls.clone(),
                accepted: true,
            },
        )
        .unwrap();
    assert!(registry
        .bind(
            &unrelated.schema,
            &node(&unrelated, "behavior"),
            native(&node(&original, "type")),
            DatatypeKind::Scalar
        )
        .is_err());
    let rule = registry
        .bind(
            &original.schema,
            &node(&original, "behavior"),
            native(&node(&original, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap();
    let control = OperationControl::new(AbortSignal::new());
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext::default(),
    };
    let input = ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("same".into()))],
        candidate: vec![],
        fallback: DiagnosticAttribution::default(),
    };
    let result = validate_rules(&[rule], &input, &runtime, ValidationLimits::default());
    assert_eq!(result.accepted, Some(true));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

fn input() -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("same".into()))],
        candidate: vec![],
        fallback: DiagnosticAttribution {
            uri: Some("input.cem".into()),
            ..Default::default()
        },
    }
}
fn run(
    rules: &[cem_ql::datatype_validation::BoundDatatypeRule],
    input: &ValidationInput,
) -> cem_ql::datatype_validation::ValidationBatch {
    let control = OperationControl::new(AbortSignal::new());
    validate_rules(
        rules,
        input,
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: EvaluationContext::default(),
        },
        ValidationLimits::default(),
    )
}
fn registered(
    required: bool,
    accepted: bool,
    calls: Arc<AtomicUsize>,
) -> cem_ql::datatype_validation::BoundDatatypeRule {
    let source = source(&declaration(required));
    let mut registry = DatatypeValidationRegistry::default();
    registry
        .register_native(
            "urn:test:validate",
            contract(&source, required),
            adapter(),
            None,
            Check { calls, accepted },
        )
        .unwrap();
    registry
        .bind(
            &source.schema,
            &node(&source, "behavior"),
            native(&node(&source, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap()
}
#[test]
fn preflight_missing_or_invalid_candidates_never_invokes_any_rule() {
    use cem_ql::datatype_validation::ValidationStopReason;
    let calls = Arc::new(AtomicUsize::new(0));
    let optional = registered(false, true, calls.clone());
    let required = registered(true, true, calls.clone());
    let missing = run(&[optional.clone(), required.clone()], &input());
    assert!(matches!(
        missing.stopped.unwrap().reason,
        ValidationStopReason::MissingCandidate
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for candidates in [
        vec![Item::Node("invented".into())],
        vec![Item::Record(Default::default())],
        vec![required.datatype().clone(), required.datatype().clone()],
    ] {
        let mut bad = input();
        bad.candidate = candidates;
        assert_eq!(run(&[optional.clone()], &bad).accepted, None);
    }
    let mut supplied = input();
    supplied.candidate = vec![required.datatype().clone()];
    assert_eq!(run(&[required], &supplied).accepted, Some(true));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for values in [
        vec![],
        vec![Item::Atomic(AtomValue::Boolean(true))],
        vec![Item::Array(input().value)],
        vec![optional.datatype().clone()],
    ] {
        let mut bad = input();
        bad.value = values;
        assert_eq!(run(&[optional.clone()], &bad).accepted, None);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
#[test]
fn effective_restrictions_accumulate_rejection_and_keep_original_declarations() {
    let calls = Arc::new(AtomicUsize::new(0));
    let rejecting = registered(false, false, calls.clone());
    let accepting = registered(false, true, calls.clone());
    let original = rejecting.behavior().identity();
    let result = run(&[rejecting, accepting], &input());
    assert_eq!(result.accepted, Some(false));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.completed[0].behavior.identity(), original);
    assert!(!result.completed[0].result.accepted);
    assert!(result.completed[1].result.accepted);
    assert_eq!(
        result.completed[0].result.diagnostics[0].uri.as_deref(),
        Some("input.cem")
    );
}
fn query_declaration(body: &str) -> String {
    declaration(false)
        .replace(
            "@implementation=engine @primitive=\"urn:test:validate\"",
            "@implementation=function @function=check-body",
        )
        .replace(
            "  {result @type=schema:datatype-validation-result}",
            &format!(
                r#"  {{result @type=schema:datatype-validation-result}}
  {{function @name=check-body @returns=datatype-validation-result |
   {{param @name=value @type=string @required=true @cardinality=one}}
   {{param @name=datatype @type=node @required=true @cardinality=one}}
   {{param @name=candidate @type=node @required=false @cardinality=zero-or-one}}
   {{body | {{$ {body} }}}}
  }}"#
            ),
        )
}
fn query_rule(body: &str) -> cem_ql::datatype_validation::BoundDatatypeRule {
    let source = source(&query_declaration(body));
    let mut registry = DatatypeValidationRegistry::default();
    registry
        .register_query(contract(&source, false), adapter(), None)
        .unwrap();
    registry
        .bind(
            &source.schema,
            &node(&source, "behavior"),
            native(&node(&source, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap()
}
#[test]
fn authored_query_uses_fixed_native_bindings_and_checks_results() {
    let rule = query_rule(
        r#"{ accepted: value == "same", diagnostics: {code: "query.note", severity: "warning", message: "retained", source: datatype} }"#,
    );
    let original = rule.datatype().identity();
    let result = run(&[rule], &input());
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(result.completed[0].result.diagnostics[0].node, original);
    for body in ["{ accepted: 1, diagnostics: () }", "1 / 0", "()"] {
        let result = run(&[query_rule(body)], &input());
        assert_eq!(result.accepted, None);
        assert!(result.stopped.is_some());
    }
    for text in [
        query_declaration("()")
            .replace("@name=candidate @type=node", "@name=candidate @type=object"),
        query_declaration("()").replace("@returns=datatype-validation-result", "@returns=object"),
        query_declaration("()").replace("@name=candidate @type=node", "@name=renamed @type=node"),
    ] {
        let src = source(&text);
        assert!(
            DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), signature(false))
                .is_err()
        );
    }
}
#[derive(Debug, Clone)]
struct Outcome(RuleExecution);
impl NativeDatatypeValidator for Outcome {
    fn validate(&self, _: ValidationCall<'_>) -> RuleExecution {
        self.0.clone()
    }
}
fn outcome_rule(outcome: RuleExecution) -> cem_ql::datatype_validation::BoundDatatypeRule {
    let src = source(&declaration(false));
    let mut registry = DatatypeValidationRegistry::default();
    registry
        .register_native(
            "urn:test:validate",
            contract(&src, false),
            adapter(),
            None,
            Outcome(outcome),
        )
        .unwrap();
    registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            native(&node(&src, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap()
}
#[test]
fn incomplete_and_failed_execution_never_become_acceptance_or_erase_prior_results() {
    let diag = cem_ml::diagnostics::Diagnostic {
        code: "pending".into(),
        ..Default::default()
    };
    for outcome in [
        RuleExecution::Pending(vec![diag.clone()]),
        RuleExecution::Unavailable(vec![diag]),
        RuleExecution::Complete(query("1 / 0")),
        RuleExecution::Complete(query("{ accepted: true }")),
    ] {
        let result = run(
            &[
                registered(false, false, Arc::new(AtomicUsize::new(0))),
                outcome_rule(outcome),
            ],
            &input(),
        );
        assert_eq!(result.accepted, None);
        assert_eq!(result.completed.len(), 1);
        assert!(!result.completed[0].result.accepted);
        assert!(result.stopped.is_some());
    }
}
#[test]
fn diagnostic_compatibility_requires_registration_and_explicit_code_mapping() {
    use cem_ml::diagnostics::{Diagnostic, Severity};
    use cem_ql::datatype_results::native_diagnostic_value;
    let text = declaration(false).replace(
        "@type=schema:datatype-validation-result",
        "@type=schema:datatype-diagnostic @cardinality=zero-or-more",
    );
    let src = source(&text);
    let mut sig = signature(false);
    sig.result =
        ResultRepresentation::Diagnostics(ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"));
    let compiled = DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), sig).unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    assert!(registry
        .register_native(
            "urn:test:validate",
            compiled.clone(),
            adapter(),
            None,
            Outcome(RuleExecution::Complete(ItemStream::empty()))
        )
        .is_err());
    for severity in [Severity::Info, Severity::Error] {
        let mut registry = DatatypeValidationRegistry::default();
        let diagnostic = Diagnostic {
            code: "reject".into(),
            severity,
            message: "original".into(),
            uri: Some("legacy.cem".into()),
            ..Default::default()
        };
        registry
            .register_native(
                "urn:test:validate",
                compiled.clone(),
                adapter(),
                Some(LegacyAcceptance::RejectCodes(["reject".into()].into())),
                Outcome(RuleExecution::Complete(ItemStream::once(
                    native_diagnostic_value(diagnostic.clone()),
                ))),
            )
            .unwrap();
        let rule = registry
            .bind(
                &src.schema,
                &node(&src, "behavior"),
                native(&node(&src, "type")),
                DatatypeKind::Scalar,
            )
            .unwrap();
        let result = run(&[rule], &input());
        assert_eq!(result.accepted, Some(false));
        assert_eq!(result.completed[0].result.diagnostics, vec![diagnostic]);
    }
    for (stream, expected) in [
        (
            query(r#"{code: "notice", severity: "error", message: "accepted error"}"#),
            Some(true),
        ),
        (ItemStream::empty(), Some(true)),
        (query("{message:\"bad\"}"), None),
        (query("1 / 0"), None),
    ] {
        let mut registry = DatatypeValidationRegistry::default();
        registry
            .register_native(
                "urn:test:validate",
                compiled.clone(),
                adapter(),
                Some(LegacyAcceptance::RejectCodes(["reject".into()].into())),
                Outcome(RuleExecution::Complete(stream)),
            )
            .unwrap();
        let rule = registry
            .bind(
                &src.schema,
                &node(&src, "behavior"),
                native(&node(&src, "type")),
                DatatypeKind::Scalar,
            )
            .unwrap();
        assert_eq!(run(&[rule], &input()).accepted, expected);
    }
}
#[test]
fn registration_rejects_wrong_implementation_duplicates_and_kind_mismatch() {
    let src = source(&declaration(false));
    let mut registry = DatatypeValidationRegistry::default();
    let cb = || Check {
        calls: Arc::new(AtomicUsize::new(0)),
        accepted: true,
    };
    assert!(registry
        .register_native("wrong", contract(&src, false), adapter(), None, cb())
        .is_err());
    registry
        .register_native(
            "urn:test:validate",
            contract(&src, false),
            adapter(),
            None,
            cb(),
        )
        .unwrap();
    assert!(registry
        .register_native(
            "urn:test:validate",
            contract(&src, false),
            adapter(),
            None,
            cb()
        )
        .is_err());
    assert!(registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            native(&node(&src, "type")),
            DatatypeKind::Node
        )
        .is_err());
    assert!(registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            native(&node(&src, "behavior")),
            DatatypeKind::Scalar
        )
        .is_err());
    assert!(registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            Item::Record(Default::default()),
            DatatypeKind::Scalar
        )
        .is_err());
    let unrelated = source(&declaration(false));
    assert!(DatatypeBehaviorContract::compile(
        &src,
        &node(&unrelated, "behavior"),
        signature(false)
    )
    .is_err());
    let mut rebound = src.clone();
    rebound
        .bindings
        .insert("schema".into(), "urn:vendor".into());
    assert!(DatatypeBehaviorContract::compile(
        &rebound,
        &node(&rebound, "behavior"),
        signature(false)
    )
    .is_err());
}
#[test]
fn cancellation_and_batch_limits_stop_before_callbacks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let rules = [registered(false, true, calls.clone())];
    let abort = AbortSignal::new();
    let control = OperationControl::new(abort.clone());
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext::default(),
    };
    for limits in [
        ValidationLimits {
            max_rules: 0,
            ..Default::default()
        },
        ValidationLimits {
            max_input_values: 0,
            ..Default::default()
        },
    ] {
        assert_eq!(
            validate_rules(&rules, &input(), &runtime, limits).accepted,
            None
        );
    }
    abort.abort();
    assert_eq!(
        validate_rules(&rules, &input(), &runtime, Default::default()).accepted,
        None
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[derive(Debug)]
struct SequenceCheck {
    expected: Vec<Item>,
    calls: Arc<AtomicUsize>,
}
impl NativeDatatypeValidator for SequenceCheck {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        assert_eq!(call.value, self.expected);
        self.calls.fetch_add(1, Ordering::SeqCst);
        RuleExecution::Complete(query("{ accepted: true, diagnostics: () }"))
    }
}
#[test]
fn list_and_node_rules_receive_complete_ordered_sequences_including_empty() {
    let targets = query(r#"data:read("<r><a/><b/></r>", "xml").root"#).items;
    for (kind, representation, ty, values) in [
        (
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
            "schema:string",
            vec![
                Item::Atomic(AtomValue::String("b".into())),
                Item::Atomic(AtomValue::String("a".into())),
                Item::Atomic(AtomValue::String("b".into())),
            ],
        ),
        (
            DatatypeKind::Node,
            ValueRepresentation::Nodes,
            "schema:node",
            vec![targets[0].clone(), targets[0].clone()],
        ),
    ] {
        for value in [values, vec![]] {
            let text = declaration(false).replace(
                "@name=value @type=schema:string @source=value @required=true @cardinality=one",
                &format!(
                    "@name=value @type={ty} @source=value @required=true @cardinality=zero-or-more"
                ),
            );
            let src = source(&text);
            let mut sig = signature(false);
            sig.kind = kind;
            sig.value = representation;
            let contract =
                DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), sig).unwrap();
            let mut registry = DatatypeValidationRegistry::default();
            let calls = Arc::new(AtomicUsize::new(0));
            registry
                .register_native(
                    "urn:test:validate",
                    contract,
                    adapter(),
                    None,
                    SequenceCheck {
                        expected: value.clone(),
                        calls: calls.clone(),
                    },
                )
                .unwrap();
            let rule = registry
                .bind(
                    &src.schema,
                    &node(&src, "behavior"),
                    native(&node(&src, "type")),
                    kind,
                )
                .unwrap();
            let mut request = input();
            request.value = value;
            assert_eq!(run(&[rule], &request).accepted, Some(true));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }
}
#[test]
fn validation_profile_is_admitted_by_schema_metamodel() {
    use cem_ml::schema::document_model::{compile_schema_document_model, validate_document_model};
    let model = compile_schema_document_model(
        CEM_SCHEMA_URI,
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    for text in [
        declaration(false),
        query_declaration("{ accepted: true, diagnostics: () }"),
    ] {
        let source = source(&text);
        let diagnostics = validate_document_model(source.schema.document(), &model);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }
}

#[test]
fn retained_registration_outlives_sources_and_cancellation_is_checked_after_native_calls() {
    use cem_ql::datatype_validation::ValidationStopReason;
    #[derive(Debug)]
    struct Cancel(AbortSignal);
    impl NativeDatatypeValidator for Cancel {
        fn validate(&self, _: ValidationCall<'_>) -> RuleExecution {
            self.0.abort();
            RuleExecution::Complete(query("{ accepted: true, diagnostics: () }"))
        }
    }
    let src = source(&declaration(false));
    let weak = Arc::downgrade(src.schema.document());
    let abort = AbortSignal::new();
    let control = OperationControl::new(abort.clone());
    let mut registry = DatatypeValidationRegistry::default();
    registry
        .register_native(
            "urn:test:validate",
            contract(&src, false),
            adapter(),
            None,
            Cancel(abort.clone()),
        )
        .unwrap();
    let rule = registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            native(&node(&src, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap();
    drop(registry);
    drop(src);
    assert!(weak.upgrade().is_some());
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext::default(),
    };
    let result = validate_rules(&[rule], &input(), &runtime, Default::default());
    assert_eq!(result.accepted, None);
    assert!(matches!(
        result.stopped.as_ref().unwrap().reason,
        ValidationStopReason::Control(_)
    ));
    drop(result);
    assert!(weak.upgrade().is_none());
    // An empty restriction list cannot bypass a cancelled lifecycle stage.
    assert_eq!(
        validate_rules(&[], &input(), &runtime, Default::default()).accepted,
        None
    );
}
#[test]
fn diagnostic_sequence_query_mapping_ignores_side_channel_reports_and_retains_candidate() {
    let text=query_declaration(r#"(report:emit("side", "emitted", "error"), { code: "notice", severity: "error", message: "candidate", source: candidate })"#)
        .replace("@type=schema:datatype-validation-result","@type=schema:datatype-diagnostic @cardinality=zero-or-more")
        .replace("@returns=datatype-validation-result","@returns=diagnostic-sequence");
    let src = source(&text);
    let mut sig = signature(false);
    sig.result =
        ResultRepresentation::Diagnostics(ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"));
    let contract = DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), sig).unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    registry
        .register_query(
            contract,
            adapter(),
            Some(LegacyAcceptance::RejectCodes(["reject".into()].into())),
        )
        .unwrap();
    let rule = registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            native(&node(&src, "type")),
            DatatypeKind::Scalar,
        )
        .unwrap();
    let mut request = input();
    request.candidate = query(r#"data:read("<candidate/>", "xml").root"#).items;
    let result = run(&[rule], &request);
    assert_eq!(result.accepted, Some(true));
    assert_eq!(
        result.completed[0].result.diagnostics[0].node,
        request.candidate[0].identity()
    );
    assert!(result.completed[0]
        .result
        .execution_diagnostics
        .iter()
        .any(|d| d.code == "side"));
}
#[test]
fn scalar_representations_are_explicit_and_diagnostics_budget_is_cumulative() {
    for (ty, repr, value) in [
        (
            "boolean",
            ScalarRepresentation::Boolean,
            AtomValue::Boolean(true),
        ),
        (
            "integer",
            ScalarRepresentation::Integer,
            AtomValue::Integer(3),
        ),
        (
            "decimal",
            ScalarRepresentation::Decimal,
            AtomValue::Decimal("3.0".into()),
        ),
        (
            "double",
            ScalarRepresentation::Double,
            AtomValue::Double(3.0),
        ),
        (
            "uri",
            ScalarRepresentation::AnyUri,
            AtomValue::AnyUri("urn:x".into()),
        ),
    ] {
        let src = source(
            &declaration(false).replace("@type=schema:string", &format!("@type=schema:{ty}")),
        );
        let mut sig = signature(false);
        sig.value = ValueRepresentation::Scalar(repr);
        let compiled =
            DatatypeBehaviorContract::compile(&src, &node(&src, "behavior"), sig).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let values = vec![Item::Atomic(value)];
        let mut registry = DatatypeValidationRegistry::default();
        registry
            .register_native(
                "urn:test:validate",
                compiled,
                adapter(),
                None,
                SequenceCheck {
                    expected: values.clone(),
                    calls: calls.clone(),
                },
            )
            .unwrap();
        let rule = registry
            .bind(
                &src.schema,
                &node(&src, "behavior"),
                native(&node(&src, "type")),
                DatatypeKind::Scalar,
            )
            .unwrap();
        assert_eq!(run(&[rule.clone()], &input()).accepted, None);
        let mut request = input();
        request.value = values;
        assert_eq!(run(&[rule], &request).accepted, Some(true));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let rules = [
        registered(false, false, calls.clone()),
        registered(false, false, calls.clone()),
        registered(false, true, calls.clone()),
    ];
    let control = OperationControl::new(AbortSignal::new());
    let result = validate_rules(
        &rules,
        &input(),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: EvaluationContext::default(),
        },
        ValidationLimits {
            max_diagnostics: 1,
            ..Default::default()
        },
    );
    assert_eq!(result.accepted, None);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.completed.len(), 2);
}

#[test]
fn original_candidate_supplies_fallback_attribution_for_source_less_diagnostics() {
    let rule = registered(false, false, Arc::new(AtomicUsize::new(0)));
    let mut request = input();
    request.candidate = query(r#"data:read("<candidate/>", "xml").root"#).items;
    let result = run(&[rule], &request);
    assert_eq!(result.accepted, Some(false));
    let diagnostic = &result.completed[0].result.diagnostics[0];
    assert_eq!(diagnostic.node, request.candidate[0].identity());
    assert_eq!(diagnostic.source_map, request.candidate[0].source_map());
    assert_ne!(diagnostic.uri.as_deref(), Some("input.cem"));
}

#[path = "datatype_validation/compilation.rs"]
mod compilation;
