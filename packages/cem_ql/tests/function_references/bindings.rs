use super::*;
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    scheduler::AbortSignal,
    schema::{
        datatype_registry::DatatypeKind,
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationImplementation, ValidationSignature,
            ValueRepresentation,
        },
        function_references::FunctionSelection,
        value_contracts::{ContractName, ValueContracts},
    },
};
use cem_ql::{
    api::EvaluationContext,
    datatype_results::{DatatypeResultAdapter, DatatypeResultError, DiagnosticAttribution},
    datatype_validation::{
        validate_rules, BoundDatatypeRule, DatatypeValidationRegistry, LegacyAcceptance,
        ValidationBatch, ValidationInput, ValidationRuntime, ValidationStopReason,
    },
    eval::AtomValue,
};

fn selected(
    body: &str,
    diagnostics: bool,
) -> (
    ValueContractSource,
    Arc<RetainedCemTree>,
    FunctionSelection,
    FunctionSelectionBudget,
) {
    let result = if diagnostics {
        "datatype-diagnostic @cardinality=zero-or-more"
    } else {
        "datatype-validation-result"
    };
    let returns = if diagnostics {
        "diagnostic-sequence"
    } else {
        "datatype-validation-result"
    };
    let (caller, tree, captured) = source(&format!("{{types | {{type @name=sample @kind=scalar}}}} {{behaviors | {{behavior @name=check @implementation=function @function={{#chosen}} @execution=datatype-validation | {{inputs | {{input-binding @name=value @type=string @source=value @required=true}} {{input-binding @name=datatype @type=node @source=datatype @required=true}} {{input-binding @name=candidate @type=node @source=candidate @required=false @cardinality=zero-or-one}}}} {{result @type={result}}}}}}}"), CEM_SCHEMA_URI);
    let (library, library_tree, library_captured) = source(&format!("{{behaviors | {{behavior @name=library | {{function @name=check @visibility=public @returns={returns} | {{param @name=value @type=string @required=true}} {{param @name=datatype @type=node @required=true}} {{param @name=candidate @type=node @required=false @cardinality=zero-or-one}} {{body | {{$ {body} }}}}}}}}}}"), "urn:library");
    let target = RetainedCemNode::new(library_tree.clone(), node(&library, "function").node_id())
        .unwrap()
        .query_item();
    let mut host = CemQlSchemaDeclarationHost::new();
    let from = host.register_scope(
        tree.clone(),
        Some(context(vec![target])),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let to = host.register_scope(
        library_tree,
        None,
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.attach_captured_names(&captured).unwrap();
    host.attach_captured_names(&library_captured).unwrap();
    assert!(host.allow_scope_crossing(from, to));
    let mut budget = budget();
    let catalog = FunctionCatalog::collect(&[caller.clone(), library], &mut budget).unwrap();
    let selection = catalog
        .select(&node(&caller, "behavior"), &mut host, &mut budget)
        .unwrap();
    assert!(selection.target().is_some(), "{selection:?}");
    (caller, tree, selection, budget)
}
fn signature(diagnostics: bool) -> ValidationSignature {
    ValidationSignature {
        kind: DatatypeKind::Scalar,
        value: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
        result: if diagnostics {
            ResultRepresentation::Diagnostics(ContractName::new(
                CEM_SCHEMA_URI,
                "datatype-diagnostic",
            ))
        } else {
            ResultRepresentation::Accepted(ContractName::new(
                CEM_SCHEMA_URI,
                "datatype-validation-result",
            ))
        },
    }
}
fn adapter() -> DatatypeResultAdapter {
    // The builtin source is already a complete schema document.
    let builtin = cem_ml::schema::package_sources::builtin_schema_package_source("schema").unwrap();
    let captured = Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(2),
                builtin.schema_source.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    );
    let doc = captured.document().clone();
    let id = doc
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
    let src = ValueContractSource::new(
        SchemaDeclarationNode::new(doc, id).unwrap(),
        CEM_SCHEMA_URI,
        BTreeMap::from([("schema".into(), CEM_SCHEMA_URI.into())]),
    )
    .with_captured_names(captured)
    .unwrap();
    DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[src], Default::default()).unwrap()),
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"),
    )
    .unwrap()
}
fn run(rule: BoundDatatypeRule, candidate: Vec<Item>) -> ValidationBatch {
    let control = OperationControl::new(AbortSignal::new());
    validate_rules(
        &[rule],
        &ValidationInput {
            value: vec![Item::Atomic(AtomValue::String("same".into()))],
            candidate,
            fallback: DiagnosticAttribution::default(),
        },
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: EvaluationContext::default(),
        },
        Default::default(),
    )
}
fn datatype(src: &ValueContractSource, tree: &Arc<RetainedCemTree>) -> Item {
    RetainedCemNode::new(tree.clone(), node(src, "type").node_id())
        .unwrap()
        .query_item()
}

#[test]
fn selected_query_requires_explicit_authority_and_keeps_fixed_optional_roles() {
    let (src, tree, selection, mut budget) = selected(
        r#"{accepted: value == "same" && seq:count(candidate) == 0, diagnostics: {code: "selected", severity: "warning", message: "original", source: datatype}}"#,
        false,
    );
    let contract =
        DatatypeBehaviorContract::compile_selected(&selection, signature(false), &mut budget)
            .unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    assert_eq!(
        registry
            .bind(
                &src.schema,
                &node(&src, "behavior"),
                datatype(&src, &tree),
                DatatypeKind::Scalar
            )
            .unwrap_err()
            .code,
        "validation-capability-unavailable"
    );
    registry.register_query(contract, adapter(), None).unwrap();
    let rule = registry
        .bind(
            &src.schema,
            &node(&src, "behavior"),
            datatype(&src, &tree),
            DatatypeKind::Scalar,
        )
        .unwrap();
    let result = run(rule.clone(), vec![]);
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(
        result.completed[0].result.diagnostics[0].node,
        datatype(&src, &tree).identity()
    );
    assert_eq!(
        run(rule.clone(), vec![datatype(&src, &tree)]).accepted,
        Some(false)
    );
    let repeated = run(rule, vec![datatype(&src, &tree), datatype(&src, &tree)]);
    assert!(matches!(
        repeated.stopped.unwrap().reason,
        ValidationStopReason::InvalidInput(_)
    ));
}

#[test]
fn selected_diagnostic_sequence_requires_an_explicit_acceptance_mapping() {
    let (src, tree, selection, mut budget) = selected(
        r#"{code: "reject", severity: "warning", message: value, source: candidate}"#,
        true,
    );
    let contract =
        DatatypeBehaviorContract::compile_selected(&selection, signature(true), &mut budget)
            .unwrap();
    let mut registry = DatatypeValidationRegistry::default();
    assert_eq!(
        registry
            .register_query(contract.clone(), adapter(), None)
            .unwrap_err()
            .code,
        "result-registration-mismatch"
    );
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
            datatype(&src, &tree),
            DatatypeKind::Scalar,
        )
        .unwrap();
    let result = run(rule, vec![datatype(&src, &tree)]);
    assert_eq!(result.accepted, Some(false), "{result:?}");
    assert_eq!(
        result.completed[0].result.diagnostics[0].node,
        datatype(&src, &tree).identity()
    );
}

#[test]
fn selected_query_failures_retain_original_function_body_source() {
    for body in ["missing_role", "1 / 0"] {
        let (src, tree, selection, mut budget) = selected(body, false);
        let contract =
            DatatypeBehaviorContract::compile_selected(&selection, signature(false), &mut budget)
                .unwrap();
        let ValidationImplementation::Query { body_source, .. } = contract.implementation() else {
            panic!()
        };
        let original = body_source.clone();
        let mut registry = DatatypeValidationRegistry::default();
        if body == "missing_role" {
            let failure = registry
                .register_query(contract, adapter(), None)
                .unwrap_err();
            assert_eq!(failure.code, "query-compilation-failed");
            assert_eq!(failure.source.unwrap().identity(), original.identity());
        } else {
            registry.register_query(contract, adapter(), None).unwrap();
            let rule = registry
                .bind(
                    &src.schema,
                    &node(&src, "behavior"),
                    datatype(&src, &tree),
                    DatatypeKind::Scalar,
                )
                .unwrap();
            let result = run(rule, vec![]);
            let ValidationStopReason::Result(DatatypeResultError::Execution(stream)) =
                result.stopped.unwrap().reason
            else {
                panic!("expected execution failure")
            };
            let CemAstNode::Element { source, .. } = original.node() else {
                panic!()
            };
            assert!(stream.diagnostics.iter().any(|d| d
                .source_map
                .as_ref()
                .is_some_and(|m| m.frames.starts_with(&source.frames))));
        }
    }
}
