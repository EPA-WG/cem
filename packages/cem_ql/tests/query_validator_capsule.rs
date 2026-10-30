use cem_ml::{
    events::cem::CemEventNormalizer,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{tree::RetainedCemTree, CemAstNode},
    schema::{
        datatype_registry::DatatypeKind,
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationSignature, ValueRepresentation,
        },
        declaration_references::SchemaDeclarationNode,
        machine::CemSchemaMachine,
        registry::CEM_SCHEMA_URI,
        structural_capsule::SourceIdentity,
        value_contracts::{ContractName, ValueContractSource, ValueContracts},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::selected_registry::{SelectedBuiltin, SelectedFunctionRegistry};
use cem_ql::{
    api::EvaluationContext,
    datatype_results::{DatatypeResultAdapter, DiagnosticAttribution},
    datatype_validation::{
        validate_rules, DatatypeValidationRegistry, ValidationInput, ValidationLimits,
        ValidationRuntime,
    },
    eval::{AtomValue, Item, RetainedCemNode},
    query_validator_capsule::{self as capsule, BuiltValidatorQuery},
};
use std::{collections::BTreeMap, sync::Arc};

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
    let schema = captured
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
        SchemaDeclarationNode::new(captured.document().clone(), schema).unwrap(),
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
fn declaration(body: &str) -> String {
    format!(
        r#"@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{{schema @name=example @namespace="urn:example" @version="1.0.0" |
 {{types | {{type @name=sample @kind=scalar}}}}
 {{behaviors | {{behavior @name=check @implementation=function @function=check-body @execution=datatype-validation |
  {{inputs |
   {{input-binding @name=value @type=schema:string @source=value @required=true @cardinality=one}}
   {{input-binding @name=datatype @type=schema:node @source=datatype @required=true @cardinality=one}}
   {{input-binding @name=candidate @type=schema:node @source=candidate @required=false @cardinality=zero-or-one}}
  }}
  {{result @type=schema:datatype-validation-result}}
  {{function @name=check-body @returns=datatype-validation-result |
   {{param @name=value @type=string @required=true @cardinality=one}}
   {{param @name=datatype @type=node @required=true @cardinality=one}}
   {{param @name=candidate @type=node @required=false @cardinality=zero-or-one}}
   {{body | {{$ {body} }}}}
  }}
 }}}}
}}"#
    )
}
fn contract(source: &ValueContractSource) -> DatatypeBehaviorContract {
    DatatypeBehaviorContract::compile(source, &node(source, "behavior"), contract_signature())
        .unwrap()
}
fn contract_signature() -> ValidationSignature {
    ValidationSignature {
        kind: DatatypeKind::Scalar,
        value: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
        result: ResultRepresentation::Accepted(ContractName::new(
            CEM_SCHEMA_URI,
            "datatype-validation-result",
        )),
    }
}
fn adapter() -> DatatypeResultAdapter {
    let schema = source(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[schema], Default::default()).unwrap()),
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap()
}
fn functions() -> SelectedFunctionRegistry {
    SelectedFunctionRegistry::new([
        SelectedBuiltin::StringLength,
        SelectedBuiltin::SequenceCount,
    ])
    .unwrap()
}
fn registry_hash() -> String {
    format!("cem-bin/1+blake3:{}", "a".repeat(64))
}
fn identity(text: &str) -> SourceIdentity {
    SourceIdentity::new(
        "https://example.test/schema.cem",
        SourceId(1),
        text.as_bytes(),
    )
}
fn build(text: &str) -> BuiltValidatorQuery {
    capsule::export(
        &contract(&source(text)),
        identity(text),
        &registry_hash(),
        &OperationControl::default(),
    )
    .unwrap()
}
fn run(
    source: &ValueContractSource,
    registry: &DatatypeValidationRegistry,
    value: &str,
) -> cem_ql::datatype_validation::ValidationBatch {
    let rule = bind(source, registry, DatatypeKind::Scalar);
    validate_rules(
        &[rule],
        &ValidationInput {
            value: vec![Item::Atomic(AtomValue::String(value.into()))],
            candidate: vec![],
            fallback: DiagnosticAttribution::default(),
        },
        &ValidationRuntime {
            control: &OperationControl::default(),
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: EvaluationContext::default(),
        },
        ValidationLimits::default(),
    )
}
fn native(source: &ValueContractSource, name: &str) -> Item {
    let target = node(source, name);
    let tree = RetainedCemTree::from_shared(
        target.document().clone(),
        "https://example.test/schema.cem",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    RetainedCemNode::new(tree, target.node_id())
        .unwrap()
        .query_item()
}
fn bind(
    source: &ValueContractSource,
    registry: &DatatypeValidationRegistry,
    kind: DatatypeKind,
) -> cem_ql::datatype_validation::BoundDatatypeRule {
    let datatype = node(source, "type");
    let tree = RetainedCemTree::from_shared(
        datatype.document().clone(),
        "https://example.test/schema.cem",
        "",
        Default::default(),
        None,
    )
    .unwrap();
    let item = RetainedCemNode::new(tree, datatype.node_id())
        .unwrap()
        .query_item();
    registry
        .bind(&source.schema, &node(source, "behavior"), item, kind)
        .unwrap()
}

#[test]
fn typed_query_roundtrip_matches_independent_native_validation_and_original_diagnostics() {
    let text = declaration(
        r#"{accepted: str:length(value) > 2, diagnostics: {code: "check.note", severity: "warning", message: "original", source: datatype}}"#,
    );
    let built = build(&text);
    let verified =
        Arc::new(capsule::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap());
    assert_eq!(verified.function_count(), 1);
    let fresh = source(&text);
    let mut reloaded = DatatypeValidationRegistry::default();
    reloaded
        .register_verified_query(
            contract(&fresh),
            &identity(&text),
            &registry_hash(),
            &functions(),
            verified,
            adapter(),
        )
        .unwrap();
    let original = source(&text);
    let mut full = DatatypeValidationRegistry::default();
    full.register_query(contract(&original), adapter(), None)
        .unwrap();
    for (value, accepted) in [("a", false), ("same", true)] {
        let left = run(&original, &full, value);
        let right = run(&fresh, &reloaded, value);
        assert_eq!(left.accepted, Some(accepted));
        assert_eq!(right.accepted, left.accepted);
        let a = &left.completed[0].result.diagnostics[0];
        let b = &right.completed[0].result.diagnostics[0];
        assert_eq!(
            (
                &a.code,
                &a.uri,
                &a.source_map,
                &a.byte_offset,
                a.line,
                a.column
            ),
            (
                &b.code,
                &b.uri,
                &b.source_map,
                &b.byte_offset,
                b.line,
                b.column
            )
        );
        let Item::Record(result) = &right.completed[0].result.original else {
            panic!("result record");
        };
        let Item::Record(diagnostic) = &result["diagnostics"][0] else {
            panic!("diagnostic record");
        };
        let source_item = &diagnostic["source"][0];
        let retained = cem_ql::eval::retained_cem_node(source_item).unwrap();
        assert!(Arc::ptr_eq(
            retained.owner().ast_owner(),
            fresh.schema.document()
        ));
        assert_eq!(retained.node_id(), node(&fresh, "type").node_id());
        assert_eq!(b.node, source_item.identity());
    }
}

#[test]
fn source_registry_and_result_mismatches_publish_no_registration() {
    let text = declaration("{accepted: true, diagnostics: ()}");
    let built = build(&text);
    let verified =
        Arc::new(capsule::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap());
    let fresh = source(&text);
    let mut registry = DatatypeValidationRegistry::default();
    let mut foreign = identity(&text);
    foreign.uri.push_str(".other");
    assert!(registry
        .register_verified_query(
            contract(&fresh),
            &foreign,
            &registry_hash(),
            &functions(),
            verified.clone(),
            adapter()
        )
        .is_err());
    assert!(registry
        .register_verified_query(
            contract(&fresh),
            &identity(&text),
            &format!("cem-bin/1+blake3:{}", "b".repeat(64)),
            &functions(),
            verified.clone(),
            adapter()
        )
        .is_err());
    let changed = source(&text.replace("accepted: true", "accepted: false"));
    assert!(registry
        .register_verified_query(
            contract(&changed),
            &identity(&text),
            &registry_hash(),
            &functions(),
            verified.clone(),
            adapter()
        )
        .is_err());
    let renamed = source(&text.replace("@name=check ", "@name=other "));
    assert!(registry
        .register_verified_query(
            contract(&renamed),
            &identity(&text),
            &registry_hash(),
            &functions(),
            verified.clone(),
            adapter()
        )
        .is_err());
    registry
        .register_verified_query(
            contract(&fresh),
            &identity(&text),
            &registry_hash(),
            &functions(),
            verified.clone(),
            adapter(),
        )
        .unwrap();
    assert!(registry
        .register_verified_query(
            contract(&fresh),
            &identity(&text),
            &registry_hash(),
            &functions(),
            verified,
            adapter()
        )
        .is_err());
    assert_eq!(run(&fresh, &registry, "same").accepted, Some(true));
}

#[test]
fn unsupported_effects_and_dynamic_calls_cannot_become_query_capsules() {
    for body in [
        r#"native:call("hidden", value)"#,
        r#"report:emit("code", "message", "error")"#,
        r#"data:read(value, "xml")"#,
        r#"modules:module_url(value)"#,
        r#"seq:map((1,2), fn(v) => v)"#,
        r#"{accepted: value.target == (), diagnostics: ()}"#,
        r#"try { value } catch (code, message) { true }"#,
    ] {
        let text = declaration(body);
        assert!(
            capsule::export(
                &contract(&source(&text)),
                identity(&text),
                &registry_hash(),
                &OperationControl::default()
            )
            .is_err(),
            "{body}"
        );
    }
    let text = declaration("{accepted: true, diagnostics: ()}").replace(
        "@returns=datatype-validation-result |",
        "@returns=datatype-validation-result @deterministic=false |",
    );
    assert!(capsule::export(
        &contract(&source(&text)),
        identity(&text),
        &registry_hash(),
        &OperationControl::default()
    )
    .is_err());
}

#[test]
fn matching_source_and_registry_hash_cannot_supply_a_missing_host_function() {
    let text = declaration("{accepted: str:length(value) > 0, diagnostics: ()}");
    let built = build(&text);
    let verified =
        Arc::new(capsule::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap());
    let fresh = source(&text);
    let mut registry = DatatypeValidationRegistry::default();
    for available in [
        SelectedFunctionRegistry::new([]).unwrap(),
        SelectedFunctionRegistry::new([SelectedBuiltin::StringUpper]).unwrap(),
    ] {
        assert!(registry
            .register_verified_query(
                contract(&fresh),
                &identity(&text),
                &registry_hash(),
                &available,
                verified.clone(),
                adapter()
            )
            .is_err());
    }
    registry
        .register_verified_query(
            contract(&fresh),
            &identity(&text),
            &registry_hash(),
            &functions(),
            verified,
            adapter(),
        )
        .unwrap();
    assert_eq!(run(&fresh, &registry, "same").accepted, Some(true));
}

#[test]
fn reloaded_results_still_pass_the_shared_result_contract() {
    for body in [
        "{accepted: 1, diagnostics: ()}",
        "{accepted: true, diagnostics: 1}",
        "()",
    ] {
        let text = declaration(body);
        let built = build(&text);
        let verified = Arc::new(
            capsule::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap(),
        );
        let fresh = source(&text);
        let mut registry = DatatypeValidationRegistry::default();
        registry
            .register_verified_query(
                contract(&fresh),
                &identity(&text),
                &registry_hash(),
                &functions(),
                verified,
                adapter(),
            )
            .unwrap();
        let result = run(&fresh, &registry, "same");
        assert_eq!(result.accepted, None, "{body}");
        assert!(result.stopped.is_some());
    }
}

#[test]
fn verified_rules_share_invocation_work_and_preserve_cancellation() {
    use cem_ql::{
        datatype_results::DatatypeResultError,
        datatype_validation::ValidationStopReason,
        eval::{BudgetAxis, EvalError},
    };
    let text = declaration("{accepted: str:length(value) > 0, diagnostics: ()}");
    let built = build(&text);
    let verified =
        Arc::new(capsule::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap());
    let fresh = source(&text);
    let mut registry = DatatypeValidationRegistry::default();
    registry
        .register_verified_query(
            contract(&fresh),
            &identity(&text),
            &registry_hash(),
            &functions(),
            verified,
            adapter(),
        )
        .unwrap();
    let rule = bind(&fresh, &registry, DatatypeKind::Scalar);
    let input = ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("same".into()))],
        candidate: vec![],
        fallback: Default::default(),
    };
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext {
            scope_policy: cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(16),
            ..Default::default()
        },
    };
    assert_eq!(
        validate_rules(&[rule.clone()], &input, &runtime, Default::default()).accepted,
        Some(true)
    );
    let result = validate_rules(
        &vec![rule.clone(); 256],
        &input,
        &runtime,
        Default::default(),
    );
    assert_eq!(result.accepted, None);
    assert!(!result.completed.is_empty());
    let ValidationStopReason::Result(DatatypeResultError::Execution(stream)) =
        result.stopped.unwrap().reason
    else {
        panic!("shared work exhaustion");
    };
    assert_eq!(
        stream.error,
        Some(EvalError::BudgetExceeded(BudgetAxis::ValidatorWork))
    );
    assert!(stream
        .diagnostics
        .iter()
        .any(|d| d.source_map.as_ref().is_some_and(|m| m.frames.len() > 1)));
    let shared = runtime.with_query_budget();
    assert_eq!(
        validate_rules(
            &vec![rule.clone(); 256],
            &input,
            &shared,
            Default::default()
        )
        .accepted,
        None
    );
    assert_eq!(
        validate_rules(&[rule.clone()], &input, &shared, Default::default()).accepted,
        None
    );
    assert_eq!(
        validate_rules(&[rule.clone()], &input, &runtime, Default::default()).accepted,
        Some(true)
    );
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    signal.abort();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: EvaluationContext::default(),
    };
    assert_eq!(
        validate_rules(&[rule], &input, &runtime, Default::default()).accepted,
        None
    );
}

#[test]
fn node_and_list_signatures_keep_native_inputs_and_candidate_requirements() {
    for node_values in [false, true] {
        let kind = if node_values {
            DatatypeKind::Node
        } else {
            DatatypeKind::List
        };
        let value = if node_values {
            ValueRepresentation::Nodes
        } else {
            ValueRepresentation::List(ScalarRepresentation::String)
        };
        let ty = if node_values { "node" } else { "string" };
        let text=declaration("{accepted: seq:count(value) == 2, diagnostics: ()}")
            .replace("@kind=scalar",if node_values {"@kind=node"} else {"@kind=list"})
            .replace("@name=value @type=schema:string @source=value @required=true @cardinality=one", &format!("@name=value @type=schema:{ty} @source=value @required=true @cardinality=zero-or-more"))
            .replace("@name=value @type=string @required=true @cardinality=one", &format!("@name=value @type={ty} @required=true @cardinality=zero-or-more"))
            .replace("@required=false @cardinality=zero-or-one", "@required=true @cardinality=one");
        let compile = |src: &ValueContractSource| {
            let mut signature = contract_signature();
            signature.kind = kind;
            signature.value = value;
            signature.candidate = CandidateRequirement::Required;
            DatatypeBehaviorContract::compile(src, &node(src, "behavior"), signature).unwrap()
        };
        let built = capsule::export(
            &compile(&source(&text)),
            identity(&text),
            &registry_hash(),
            &OperationControl::default(),
        )
        .unwrap();
        let verified = Arc::new(
            capsule::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap(),
        );
        let fresh = source(&text);
        let mut registry = DatatypeValidationRegistry::default();
        registry
            .register_verified_query(
                compile(&fresh),
                &identity(&text),
                &registry_hash(),
                &functions(),
                verified,
                adapter(),
            )
            .unwrap();
        let rule = bind(&fresh, &registry, kind);
        let values = if node_values {
            vec![native(&fresh, "type"); 2]
        } else {
            vec![Item::Atomic(AtomValue::String("same".into())); 2]
        };
        let mut input = ValidationInput {
            value: values,
            candidate: vec![],
            fallback: Default::default(),
        };
        let runtime = ValidationRuntime {
            control: &OperationControl::default(),
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: EvaluationContext::default(),
        };
        assert_eq!(
            validate_rules(&[rule.clone()], &input, &runtime, Default::default()).accepted,
            None
        );
        input.candidate = vec![native(&fresh, "type")];
        assert_eq!(
            validate_rules(&[rule.clone()], &input, &runtime, Default::default()).accepted,
            Some(true)
        );
        input.value.pop();
        assert_eq!(
            validate_rules(&[rule.clone()], &input, &runtime, Default::default()).accepted,
            Some(false)
        );
        input.value = vec![Item::Atomic(AtomValue::Integer(2))];
        assert_eq!(
            validate_rules(&[rule], &input, &runtime, Default::default()).accepted,
            None
        );
    }
}
