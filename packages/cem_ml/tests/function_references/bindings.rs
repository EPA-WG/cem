use super::*;
use cem_ml::schema::{
    datatype_conversion::{
        ConversionBehaviorContract, ConversionRepresentation, ConversionSignature,
    },
    datatype_enumeration::{ConstantBehaviorContract, EqualityBehaviorContract},
    datatype_registry::DatatypeKind,
    datatype_validation::{
        CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation, ScalarRepresentation,
        ValidationImplementation, ValidationSignature, ValueRepresentation,
    },
    document_model::compile_schema_document_model,
    value_contracts::ContractName,
};

fn profile(kind: &str, params: &str, result: &str) -> (ValueContractSource, ValueContractSource) {
    let equality = kind == "equality";
    let candidate_required = kind == "constant";
    let roles: Vec<(&str, &str, bool, &str)> = if equality {
        vec![
            ("left", "string", true, "one"),
            ("right", "string", true, "one"),
            ("datatype", "node", true, "one"),
        ]
    } else {
        vec![
            ("value", "string", true, "one"),
            ("datatype", "node", true, "one"),
            (
                "candidate",
                "node",
                candidate_required,
                if candidate_required {
                    "one"
                } else {
                    "zero-or-one"
                },
            ),
        ]
    };
    let inputs = roles
        .iter()
        .map(|(n, t, r, c)| {
            format!(
                "{{input-binding @name={n} @type={t} @source={n} @required={r} @cardinality={c}}}"
            )
        })
        .collect::<String>();
    let authored_params = roles
        .iter()
        .map(|(n, t, r, c)| {
            format!("{{param @name={n} @type=original:{t} @required={r} @cardinality={c}}}")
        })
        .collect::<String>();
    let params = if params.is_empty() {
        authored_params
    } else {
        params.into()
    };
    let result_field = if result == "diagnostic-sequence" {
        "datatype-diagnostic @cardinality=zero-or-more"
    } else {
        result
    };
    let mut caller = source(&format!("{{behaviors | {{behavior @name=caller @implementation=function @function={{#chosen}} @execution=datatype-{kind} | {{inputs | {inputs}}} {{result @type=contracts:{result_field}}}}}}}"), "urn:caller");
    caller
        .bindings
        .insert("contracts".into(), CEM_SCHEMA_URI.into());
    caller
        .bindings
        .insert("original".into(), "urn:wrong".into());
    let mut library = source(&format!("{{behaviors | {{behavior @name=library | {{function @name=f @returns={result} @visibility=public | {params} {{body | {{$ {{accepted: true, diagnostics: ()}} }}}}}}}}}}"), "urn:library");
    library
        .bindings
        .insert("original".into(), CEM_SCHEMA_URI.into());
    (caller, library)
}
fn select(
    caller: &ValueContractSource,
    library: &ValueContractSource,
    budget: &mut FunctionSelectionBudget,
) -> FunctionSelection {
    let catalog = FunctionCatalog::collect(&[caller.clone(), library.clone()], budget).unwrap();
    let mut host = Host {
        crossing: true,
        ..Host::default()
    };
    host.bind(&references(caller)[0], nodes(library, "function"));
    catalog
        .select(&nodes(caller, "behavior")[0], &mut host, budget)
        .unwrap()
}
fn signature() -> ValidationSignature {
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
#[test]
fn selected_validation_keeps_foreign_function_context_and_caller_registration_identity() {
    let (caller, library) = profile("validation", "", "datatype-validation-result");
    let mut budget = budget();
    let selected = select(&caller, &library, &mut budget);
    let contract =
        DatatypeBehaviorContract::compile_selected(&selected, signature(), &mut budget).unwrap();
    assert_eq!(contract.owner().identity(), caller.schema.identity());
    assert_eq!(
        contract.behavior().identity(),
        nodes(&caller, "behavior")[0].identity()
    );
    let ValidationImplementation::Query {
        function,
        source,
        body_source,
        ..
    } = contract.implementation()
    else {
        panic!()
    };
    assert_eq!(
        function.identity(),
        nodes(&library, "function")[0].identity()
    );
    assert_eq!(source.bindings["original"], CEM_SCHEMA_URI);
    assert!(Arc::ptr_eq(
        body_source.document(),
        library.schema.document()
    ));
}
#[test]
fn all_registered_profiles_check_selected_function_signatures() {
    for kind in ["conversion", "equality", "constant"] {
        let result = format!("datatype-{kind}-result");
        let (caller, library) = profile(kind, "", &result);
        let mut budget = budget();
        let selected = select(&caller, &library, &mut budget);
        let name = ContractName::new(CEM_SCHEMA_URI, &result);
        match kind {
            "conversion" => {
                ConversionBehaviorContract::compile_selected(
                    &selected,
                    ConversionSignature {
                        kind: DatatypeKind::Scalar,
                        input: ConversionRepresentation::Lexical,
                        output: ValueRepresentation::Scalar(ScalarRepresentation::String),
                        candidate: CandidateRequirement::Optional,
                    },
                    name,
                    &mut budget,
                )
                .unwrap();
            }
            "equality" => {
                EqualityBehaviorContract::compile_selected(
                    &selected,
                    ScalarRepresentation::String,
                    name,
                    &mut budget,
                )
                .unwrap();
            }
            _ => {
                ConstantBehaviorContract::compile_selected(
                    &selected,
                    ScalarRepresentation::String,
                    name,
                    &mut budget,
                )
                .unwrap();
            }
        }
    }
}
#[test]
fn selected_signature_rejects_renamed_roles_and_incompatible_return_contracts() {
    for params in [
        "{param @name=renamed @type=string @required=true}",
        "{param @name=value @type=string @required=true @nullable=true}",
        "{param @name=value @type=node @required=true}",
    ] {
        let (caller, library) = profile("validation", params, "datatype-validation-result");
        let mut budget = budget();
        let selected = select(&caller, &library, &mut budget);
        let error = DatatypeBehaviorContract::compile_selected(&selected, signature(), &mut budget)
            .unwrap_err();
        assert!(Arc::ptr_eq(
            error.source.unwrap().document(),
            library.schema.document()
        ));
    }
    let (caller, _) = profile("validation", "", "datatype-validation-result");
    let (_, library) = profile("validation", "", "object");
    let mut budget = budget();
    let selected = select(&caller, &library, &mut budget);
    assert_eq!(
        DatatypeBehaviorContract::compile_selected(&selected, signature(), &mut budget)
            .unwrap_err()
            .code,
        "function-result-mismatch"
    );
}
#[test]
fn selected_binding_cannot_be_forged_by_editing_pending_resolution_metadata() {
    let (caller, library) = profile("validation", "", "datatype-validation-result");
    let mut budget = budget();
    let catalog =
        FunctionCatalog::collect(&[caller.clone(), library.clone()], &mut budget).unwrap();
    let pending = source("{#pending}", "urn:pending");
    let mut host = Host {
        crossing: true,
        ..Host::default()
    };
    host.bind(
        &references(&caller)[0],
        vec![
            nodes(&library, "function")[0].clone(),
            references(&pending)[0].clone(),
        ],
    );
    let mut selected = catalog
        .select(&nodes(&caller, "behavior")[0], &mut host, &mut budget)
        .unwrap();
    assert_eq!(selected.resolution.state, ReferenceResolutionState::Pending);
    selected.resolution.state = ReferenceResolutionState::Resolved;
    selected.resolution.failed = false;
    assert_eq!(
        DatatypeBehaviorContract::compile_selected(&selected, signature(), &mut budget)
            .unwrap_err()
            .code,
        "function-selection-incomplete"
    );
}
#[test]
fn signature_binding_charges_the_same_finite_budget() {
    let (caller, library) = profile("validation", "", "datatype-validation-result");
    let mut selection_budget = budget();
    let selected = select(&caller, &library, &mut selection_budget);
    let mut tiny = FunctionSelectionBudget::new(ReferenceTraversalLimits {
        max_depth: 8,
        max_work: 1,
    })
    .unwrap();
    assert_eq!(
        DatatypeBehaviorContract::compile_selected(&selected, signature(), &mut tiny)
            .unwrap_err()
            .code,
        "function-work-limit"
    );
}
#[test]
fn source_only_native_function_slots_block_readiness_even_when_unused() {
    for slot in ["{#chosen}", "{$ chosen}"] {
        let text = format!("{{schema | {{behaviors | {{behavior @name=unused @implementation=function @execution=datatype-validation @function={slot}}}}}}}");
        let model = compile_schema_document_model("urn:test", &text);
        assert!(!model.is_ready_for_validation());
        assert!(!model.declaration_references.sites.is_empty());
    }
}

#[test]
fn diagnostic_only_signature_requires_the_registered_sequence_contract() {
    let (caller, library) = profile("validation", "", "diagnostic-sequence");
    let mut budget = budget();
    let selected = select(&caller, &library, &mut budget);
    let mut diagnostics = signature();
    diagnostics.result =
        ResultRepresentation::Diagnostics(ContractName::new(CEM_SCHEMA_URI, "datatype-diagnostic"));
    DatatypeBehaviorContract::compile_selected(&selected, diagnostics, &mut budget).unwrap();
    assert_eq!(
        DatatypeBehaviorContract::compile_selected(&selected, signature(), &mut budget)
            .unwrap_err()
            .code,
        "result-signature-mismatch"
    );
}

#[test]
fn reused_behavior_keeps_its_native_function_readiness_guard() {
    use cem_ml::schema::declaration_references::{
        compile_schema_with_declaration_references, SchemaDeclarationKind,
    };
    let caller = source("{behaviors | {#shared}}", "urn:caller");
    let library = source("{behaviors | {behavior @name=shared @implementation=function @execution=datatype-validation @function={#chosen}}}", "urn:library");
    let mut host = Host {
        crossing: true,
        ..Host::default()
    };
    host.bind(&references(&caller)[0], nodes(&library, "behavior"));
    let model = compile_schema_with_declaration_references(
        "urn:caller",
        caller.schema.document().clone(),
        &mut host,
        ReferenceTraversalLimits {
            max_depth: 64,
            max_work: 10_000,
        },
    )
    .unwrap();
    assert!(!model.is_ready_for_validation());
    assert!(model
        .declaration_references
        .sites
        .iter()
        .any(|site| site.kind == SchemaDeclarationKind::BehaviorFunction));
}
