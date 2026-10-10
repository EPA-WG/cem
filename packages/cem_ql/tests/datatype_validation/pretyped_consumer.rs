use super::*;
use cem_ml::schema::{
    datatype_contracts::{LexicalInput, RegisteredTokenizer},
    document_model::{attribute_facets::*, shipped_datatypes::ShippedDatatype as T},
};
use cem_ql::{
    attribute_datatypes::bind_attribute_datatype, attribute_validation::*, datatype_enumeration::*,
    datatype_facets::*, datatype_preparation::*, preparation_evidence::*,
};
use std::sync::atomic::AtomicBool;

#[derive(Debug)]
struct Prepare {
    calls: Arc<AtomicUsize>,
    integer: bool,
    malformed: Arc<AtomicBool>,
}
impl NativeLexicalPreparer for Prepare {
    fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let atom = if self.malformed.load(Ordering::SeqCst) {
            AtomValue::Boolean(false)
        } else if self.integer {
            AtomValue::Integer(call.lexical.text.trim().parse().unwrap())
        } else {
            AtomValue::String(call.lexical.text.to_string())
        };
        PreparationExecution::Prepared {
            value: vec![Item::Atomic(atom)],
            diagnostics: vec![],
        }
    }
}
#[derive(Debug)]
struct Rule {
    calls: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    accepted: bool,
    warning: bool,
    action: Arc<std::sync::Mutex<RuleAction>>,
}
#[derive(Debug, Default, Clone)]
struct RuleAction {
    retained: Option<(AttributePreparationInvocation, SealedPreparationEvidence)>,
    panic: bool,
    close: bool,
    pending: bool,
}
impl NativeDatatypeValidator for Rule {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let action = self.action.lock().unwrap().clone();
        if action.pending && call.value.len() == 1 {
            return RuleExecution::Pending(vec![cem_ml::diagnostics::Diagnostic {
                code: "pending".into(),
                message: "rule pending".into(),
                severity: cem_ml::diagnostics::Severity::Warning,
                ..Default::default()
            }]);
        }
        if action.panic {
            panic!("fixture validation panic");
        }
        if let Some((live, evidence)) = action.retained {
            let bound = evidence.binding();
            let result = bound.validate_pretyped(
                &live,
                Some(&evidence),
                FacetContext {
                    element_name: "sample",
                    source: bound.binding().declaration.node(),
                    diagnostic_behaviors: &Default::default(),
                    attribute_values: &Default::default(),
                },
                FacetLimits::default().max_model_bytes,
            );
            assert!(matches!(
                result.stopped,
                Some(AttributeValidationStop::Evidence(EvidenceStop::Busy))
            ));
            assert!(matches!(live.prepare().stopped, Some(EvidenceStop::Busy)));
            if action.close {
                live.close();
            }
        }
        if self.cancel.load(Ordering::SeqCst) {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        let diagnostics = if self.warning {
            "({code: \"warning\", severity: \"warning\", message: \"checked\"})"
        } else {
            "()"
        };
        RuleExecution::Complete(query(&format!(
            "{{accepted: {}, diagnostics: {diagnostics}}}",
            self.accepted
        )))
    }
}
#[derive(Debug)]
struct Equality;
impl NativeScalarEquality for Equality {
    fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution {
        EqualityExecution::Complete {
            equal: call.left == call.right,
            diagnostics: vec![],
        }
    }
}
struct Fixture {
    bound: BoundAttributeFacets,
    preparations: Arc<AtomicUsize>,
    rules: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    malformed: Arc<AtomicBool>,
    action: Arc<std::sync::Mutex<RuleAction>>,
}
fn fixture(
    family: FacetFamily,
    fields: &str,
    type_fields: &str,
    required: bool,
    accepted: bool,
    warning: bool,
) -> Fixture {
    let list = matches!(family, FacetFamily::List(_));
    let inherited = type_fields.contains("@base=ancestor");
    let integer = matches!(
        family.representation(),
        ValueRepresentation::Scalar(ScalarRepresentation::Integer)
            | ValueRepresentation::List(ScalarRepresentation::Integer)
    );
    let kind = if list {
        DatatypeKind::List
    } else {
        DatatypeKind::Scalar
    };
    let kind_name = if list { "list" } else { "scalar" };
    let item = if list {
        "{type @name=item @kind=scalar}"
    } else if inherited {
        "{type @name=ancestor @kind=scalar @values=\"3 5\"}"
    } else {
        ""
    };
    let base = if list { "@base=item" } else { "" };
    let profile = source(&format!("{{schema @name=test @namespace=urn:test | {{types | {item} {{type @name=sample @kind={kind_name} {base} {type_fields}}}}} {{attributes | {{attribute @name=value @type=sample {fields}}}}}}}"));
    let decl = node(&profile, "attribute");
    let (mut host, sources) = types_fixture_source(profile);
    let selected = sources.last().unwrap();
    let preparations = Arc::new(AtomicUsize::new(0));
    let rules = Arc::new(AtomicUsize::new(0));
    let cancel = Arc::new(AtomicBool::new(false));
    let malformed = Arc::new(AtomicBool::new(false));
    let action = Arc::new(std::sync::Mutex::new(RuleAction::default()));
    let mut text = declaration(required);
    if integer {
        text = text.replace("@type=schema:string", "@type=schema:integer");
    }
    if list {
        text = text.replace(
            "@source=value @required=true @cardinality=one",
            "@source=value @required=true @cardinality=zero-or-more",
        );
    }
    let rule_source = source(&text);
    let rule = node(&rule_source, "behavior");
    let mut sig = signature(required);
    sig.kind = kind;
    sig.value = family.representation();
    let contract = DatatypeBehaviorContract::compile(&rule_source, &rule, sig).unwrap();
    let mut validations = DatatypeValidationRegistry::default();
    validations
        .register_native(
            "urn:test:validate",
            contract,
            adapter(),
            None,
            Rule {
                calls: rules.clone(),
                cancel: cancel.clone(),
                accepted,
                warning,
                action: action.clone(),
            },
        )
        .unwrap();
    let mut implementations = DatatypeImplementations::default();
    let mut registration = implementation(selected, kind, family.representation());
    registration.validator = Some((rule_source.schema.clone(), rule.clone()));
    let scalar = if list || inherited {
        &sources[0]
    } else {
        selected
    };
    let primitive = if integer {
        ScalarRepresentation::Integer
    } else {
        ScalarRepresentation::String
    };
    if list || inherited {
        let mut base_registration = implementation(
            scalar,
            DatatypeKind::Scalar,
            ValueRepresentation::Scalar(primitive),
        );
        if inherited {
            base_registration.validator = Some((rule_source.schema, rule));
        }
        if list {
            let mut item_text = declaration(false);
            if integer {
                item_text = item_text.replace("@type=schema:string", "@type=schema:integer");
            }
            let item_source = source(&item_text);
            let item_rule = node(&item_source, "behavior");
            let mut item_sig = signature(false);
            item_sig.value = ValueRepresentation::Scalar(primitive);
            let item_contract =
                DatatypeBehaviorContract::compile(&item_source, &item_rule, item_sig).unwrap();
            validations
                .register_native(
                    "urn:test:validate",
                    item_contract,
                    adapter(),
                    None,
                    Rule {
                        calls: rules.clone(),
                        cancel: cancel.clone(),
                        accepted,
                        warning,
                        action: action.clone(),
                    },
                )
                .unwrap();
            base_registration.validator = Some((item_source.schema, item_rule));
        }
        implementations.register(base_registration).unwrap();
    }
    if list {
        registration.tokenizer = TokenizerBinding::Ready(RegisteredTokenizer::whitespace());
    }
    implementations.register(registration).unwrap();
    implementations
        .select_preparation(
            scalar.clone(),
            PreparationBinding::Ready(
                RegisteredLexicalPreparation::new(
                    scalar.clone(),
                    "counted",
                    PreparationSignature {
                        kind: DatatypeKind::Scalar,
                        output: ValueRepresentation::Scalar(primitive),
                        candidate: CandidateRequirement::Optional,
                    },
                    Prepare {
                        calls: preparations.clone(),
                        integer,
                        malformed: malformed.clone(),
                    },
                )
                .unwrap(),
            ),
        )
        .unwrap();
    if list {
        implementations
            .select_preparation(
                selected.clone(),
                PreparationBinding::Ready(
                    RegisteredLexicalPreparation::list_items(selected.clone(), "list", primitive)
                        .unwrap(),
                ),
            )
            .unwrap();
    }
    if !type_fields.is_empty() && integer {
        for selected in if inherited {
            sources.as_slice()
        } else {
            std::slice::from_ref(selected)
        } {
            implementations
                .select_equality(
                    selected.clone(),
                    EqualityBinding::Ready(
                        RegisteredScalarEquality::new(
                            selected.clone(),
                            "equality",
                            primitive,
                            Equality,
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
            implementations
                .select_constant_interpreter(
                    selected.clone(),
                    ConstantBinding::Ready(
                        cem_ql::datatype_shipped::constant_interpreter(
                            selected.clone(),
                            T::Integer,
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
        }
    }
    let facet_source = if inherited { scalar } else { selected };
    implementations
        .select_facets(
            facet_source.clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(facet_source.clone(), "facets", family).unwrap(),
            ),
        )
        .unwrap();
    let compilation_control = OperationControl::default();
    let compilation = cem_ql::datatype_compilation::compile_datatypes_with_runtime(
        selected.declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &validations,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &ValidationRuntime {
            control: &compilation_control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    );
    assert!(compilation.is_ready(), "{:?}", compilation.issues);
    let slot = match decl.node() { CemAstNode::Element { attributes, .. } => attributes.iter().filter_map(|id|SchemaDeclarationNode::new(decl.document().clone(),*id)).find(|n|matches!(n.node(),CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name=="type")).unwrap(), _=>panic!() };
    host.bind_literal_attribute_type(slot, selected.declaration().clone())
        .unwrap();
    let bound = bind_attribute_datatype(
        decl,
        &compilation,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap()
    .compile_facets("urn:test", Default::default())
    .unwrap();
    rules.store(0, Ordering::SeqCst);
    Fixture {
        bound,
        preparations,
        rules,
        cancel,
        malformed,
        action,
    }
}
fn invocation(
    f: &Fixture,
    text: &str,
    control: &OperationControl,
    limits: PreparationLimits,
) -> AttributePreparationInvocation {
    AttributePreparationInvocation::new(
        &f.bound,
        PreparationInput {
            lexical: LexicalInput::new(Arc::from(text), Default::default()),
            candidate: vec![],
            fallback: Default::default(),
        },
        &ValidationRuntime {
            control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        limits,
    )
    .unwrap()
}
fn consume(
    f: &Fixture,
    live: &AttributePreparationInvocation,
    evidence: Option<&SealedPreparationEvidence>,
) -> AttributeValidation {
    f.bound.validate_pretyped(
        live,
        evidence,
        FacetContext {
            element_name: "sample",
            source: f.bound.binding().declaration.node(),
            diagnostic_behaviors: &Default::default(),
            attribute_values: &Default::default(),
        },
        FacetLimits::default().max_model_bytes,
    )
}
#[test]
fn pretyped_consumption_keeps_lexical_values_distinct_from_typed_enumeration() {
    for (text, values, expected) in [
        ("003", "003", true),
        ("003", "3", false),
        ("004", "004", false),
    ] {
        let f = fixture(
            FacetFamily::Shipped(T::Integer),
            &format!("@values={values}"),
            "@values=3",
            false,
            true,
            false,
        );
        let control = OperationControl::default();
        let live = invocation(&f, text, &control, Default::default());
        let evidence = live.prepare().evidence.unwrap();
        let calls = f.preparations.load(Ordering::SeqCst);
        let result = consume(&f, &live, Some(&evidence));
        assert_eq!(result.accepted, Some(expected), "{result:?}");
        let AttributeDatatypePhase::Pretyped {
            evidence: retained,
            validation,
        } = result.datatype.unwrap()
        else {
            panic!()
        };
        assert_eq!(&*retained.lexical().text, text);
        assert_eq!(
            retained.values()[0].atom(),
            Some(AtomValue::Integer(text.parse().unwrap()))
        );
        assert_eq!(validation.accepted, Some(text != "004"));
        assert_eq!(f.preparations.load(Ordering::SeqCst), calls);
        assert_eq!(f.rules.load(Ordering::SeqCst), 1);
        assert!(
            live.remaining_limits().validation.max_comparisons
                < ValidationLimits::default().max_comparisons
        );
    }
}
#[test]
fn pretyped_strings_and_lists_use_original_text_and_duplicate_spans() {
    for text in ["β", "a"] {
        let f = fixture(
            FacetFamily::Shipped(T::String),
            "@pattern=β",
            "",
            false,
            true,
            false,
        );
        let control = OperationControl::default();
        let live = invocation(&f, text, &control, Default::default());
        let evidence = live.prepare().evidence.unwrap();
        assert_eq!(
            consume(&f, &live, Some(&evidence)).accepted,
            Some(text == "β")
        );
        assert_eq!(f.preparations.load(Ordering::SeqCst), 1);
    }
    let f = fixture(
        FacetFamily::List(ScalarRepresentation::String),
        "@minItems=3 @maxItems=3",
        "",
        false,
        true,
        false,
    );
    let control = OperationControl::default();
    let live = invocation(&f, "\u{2003}β a β\t", &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    assert_eq!(evidence.token_spans(), &[3..5, 6..7, 8..10]);
    let result = consume(&f, &live, Some(&evidence));
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(f.preparations.load(Ordering::SeqCst), 3);
    assert_eq!(evidence.values().len(), 3);
    assert_eq!(evidence.values()[0], evidence.values()[2]);
    assert!(f.bound.binding().datatype.converter().is_none());
    assert!(f.bound.binding().datatype.list_serializer().is_none());
}
#[test]
fn absent_evidence_is_incomplete_and_present_empty_list_is_validated() {
    let f = fixture(
        FacetFamily::List(ScalarRepresentation::Integer),
        "@minItems=0 @maxItems=0",
        "",
        false,
        true,
        false,
    );
    let control = OperationControl::default();
    let live = invocation(&f, " \t", &control, Default::default());
    assert_eq!(consume(&f, &live, None).accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 0);
    let evidence = live.prepare().evidence.unwrap();
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, Some(true));
    assert_eq!(f.rules.load(Ordering::SeqCst), 1);
    assert_eq!(f.preparations.load(Ordering::SeqCst), 0);
}
#[test]
fn foreign_bindings_invocations_and_closed_evidence_never_run_rules() {
    let f = fixture(FacetFamily::Shipped(T::Integer), "", "", false, true, false);
    let other = fixture(FacetFamily::Shipped(T::Integer), "", "", false, true, false);
    let control = OperationControl::default();
    let live = invocation(&f, "003", &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    let foreign = invocation(&f, "003", &control, Default::default());
    for result in [
        consume(&other, &live, Some(&evidence)),
        consume(&f, &foreign, Some(&evidence)),
    ] {
        assert_eq!(result.accepted, None);
        assert!(result.stopped.is_some());
    }
    live.close();
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 0);
    assert_eq!(other.rules.load(Ordering::SeqCst), 0);
}
#[test]
fn source_less_prepared_values_cannot_manufacture_a_required_rule_candidate() {
    let f = fixture(FacetFamily::Shipped(T::Integer), "", "", true, true, false);
    let control = OperationControl::default();
    let live = invocation(&f, "003", &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    let result = consume(&f, &live, Some(&evidence));
    assert_eq!(result.accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 0);
    let AttributeDatatypePhase::Pretyped { validation, .. } = result.datatype.unwrap() else {
        panic!()
    };
    assert!(matches!(
        validation.stopped.unwrap().reason,
        cem_ql::datatype_validation::ValidationStopReason::MissingCandidate
    ));
}
#[test]
fn validation_repeats_rules_without_preparation_and_spends_shared_allowances() {
    let f = fixture(
        FacetFamily::Shipped(T::Integer),
        "",
        "",
        false,
        false,
        false,
    );
    let control = OperationControl::default();
    let mut limits = PreparationLimits::default();
    limits.validation.max_rules = 2;
    let live = invocation(&f, "003", &control, limits);
    let evidence = live.prepare().evidence.unwrap();
    for _ in 0..2 {
        assert_eq!(
            consume(&f, &live.clone(), Some(&evidence)).accepted,
            Some(false)
        );
    }
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 2);
    assert_eq!(f.preparations.load(Ordering::SeqCst), 1);
    assert_eq!(live.remaining_limits().validation.max_rules, 0);
    let f = fixture(
        FacetFamily::Shipped(T::Integer),
        "",
        "@values=3",
        false,
        true,
        false,
    );
    let mut limits = PreparationLimits::default();
    limits.validation.max_comparisons = 1;
    let live = invocation(&f, "003", &control, limits);
    let evidence = live.prepare().evidence.unwrap();
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, Some(true));
    assert_eq!(live.remaining_limits().validation.max_comparisons, 0);
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, None);
}
#[test]
fn datatype_and_facets_share_diagnostics_and_cancellation_never_accepts_partial_work() {
    let pending = fixture(FacetFamily::Shipped(T::Integer), "", "", false, true, false);
    let control = OperationControl::default();
    let mut limits = PreparationLimits::default();
    limits.validation.max_diagnostics = 2;
    let live = invocation(&pending, "003", &control, limits);
    let evidence = live.prepare().evidence.unwrap();
    pending.action.lock().unwrap().pending = true;
    for remaining in [1, 0] {
        assert_eq!(consume(&pending, &live, Some(&evidence)).accepted, None);
        assert_eq!(
            live.remaining_limits().validation.max_diagnostics,
            remaining
        );
    }

    for (budget, expected) in [(0, None), (1, None), (2, Some(false))] {
        let f = fixture(
            FacetFamily::Shipped(T::Integer),
            "@maxInclusive=4",
            "",
            false,
            true,
            true,
        );
        let control = OperationControl::default();
        let mut limits = PreparationLimits::default();
        limits.validation.max_diagnostics = budget;
        let live = invocation(&f, "005", &control, limits);
        let evidence = live.prepare().evidence.unwrap();
        assert_eq!(consume(&f, &live, Some(&evidence)).accepted, expected);
        assert_eq!(live.remaining_limits().validation.max_diagnostics, 0);
    }
    let f = fixture(FacetFamily::Shipped(T::Integer), "", "", false, true, false);
    let control = OperationControl::default();
    let live = invocation(&f, "003", &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    f.cancel.store(true, Ordering::SeqCst);
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, None);
    assert!(!evidence.is_live());
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 1);
}

#[test]
fn inherited_and_local_rules_and_enumerations_all_run_against_prepared_values() {
    for (text, expected) in [("003", true), ("005", false), ("006", false)] {
        let f = fixture(
            FacetFamily::Shipped(T::Integer),
            "@maxInclusive=4",
            "@base=ancestor @values=3",
            false,
            true,
            false,
        );
        let control = OperationControl::default();
        let live = invocation(&f, text, &control, Default::default());
        let evidence = live.prepare().evidence.unwrap();
        let result = consume(&f, &live, Some(&evidence));
        assert_eq!(result.accepted, Some(expected), "{result:?}");
        let AttributeDatatypePhase::Pretyped { validation, .. } = result.datatype.unwrap() else {
            panic!()
        };
        assert_eq!(validation.completed.len(), 2);
        assert_eq!(validation.enumerations.len(), 2);
        assert_eq!(f.rules.load(Ordering::SeqCst), 2);
        assert_eq!(f.preparations.load(Ordering::SeqCst), 1);
        assert_eq!(result.facets.unwrap().accepted, text == "003");
    }
}
#[test]
fn malformed_preparation_and_modified_public_reports_cannot_supply_missing_evidence() {
    let f = fixture(FacetFamily::Shipped(T::Integer), "", "", false, true, false);
    let control = OperationControl::default();
    let live = invocation(&f, "003", &control, Default::default());
    f.malformed.store(true, Ordering::SeqCst);
    let mut result = live.prepare();
    assert!(result.evidence.is_none());
    result.report.value = Some(vec![Item::Atomic(AtomValue::Integer(3))]);
    result.report.accepted = Some(true);
    assert_eq!(consume(&f, &live, result.evidence.as_ref()).accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 0);
}
#[test]
fn inspection_and_validation_spend_one_input_budget_and_empty_receipts_do_not_reset_it() {
    let f = fixture(
        FacetFamily::List(ScalarRepresentation::Integer),
        "@minItems=0 @maxItems=0",
        "",
        false,
        true,
        false,
    );
    let control = OperationControl::default();
    let mut limits = PreparationLimits::default();
    limits.validation.max_input_values = 2;
    let live = invocation(&f, "", &control, limits);
    let evidence = live.prepare().evidence.unwrap();
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, Some(true));
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, Some(true));
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 2);
}

#[test]
fn required_original_candidate_is_retained_without_new_navigation_authority() {
    let f = fixture(
        FacetFamily::Shipped(T::String),
        "@pattern=sample",
        "",
        true,
        true,
        false,
    );
    let control = OperationControl::default();
    let candidate = native(&f.bound.binding().slot);
    let source = candidate.source_map().unwrap();
    let live = AttributePreparationInvocation::new(
        &f.bound,
        PreparationInput {
            lexical: LexicalInput::new(Arc::from("sample"), source.clone()),
            candidate: vec![candidate.clone()],
            fallback: Default::default(),
        },
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        Default::default(),
    )
    .unwrap();
    let evidence = live.prepare().evidence.unwrap();
    let result = consume(&f, &live, Some(&evidence));
    assert_eq!(result.accepted, Some(true), "{result:?}");
    assert_eq!(evidence.candidate()[0].identity(), candidate.identity());
    assert_eq!(evidence.candidate()[0].source_map(), Some(source));
    assert_eq!(f.rules.load(Ordering::SeqCst), 1);
}
#[test]
fn reentrant_consumption_cannot_reset_budget_and_callback_close_or_panic_cannot_accept() {
    for close in [false, true] {
        let f = fixture(FacetFamily::Shipped(T::Integer), "", "", false, true, false);
        let control = OperationControl::default();
        let live = invocation(&f, "003", &control, Default::default());
        let evidence = live.prepare().evidence.unwrap();
        *f.action.lock().unwrap() = RuleAction {
            retained: Some((live.clone(), evidence.clone())),
            close,
            panic: false,
            pending: false,
        };
        let report = consume(&f, &live, Some(&evidence));
        assert_eq!(report.accepted, if close { None } else { Some(true) });
        assert_eq!(f.rules.load(Ordering::SeqCst), 1);
        assert_eq!(f.preparations.load(Ordering::SeqCst), 1);
        *f.action.lock().unwrap() = RuleAction::default();
    }
    let f = fixture(FacetFamily::Shipped(T::Integer), "", "", false, true, false);
    let control = OperationControl::default();
    let live = invocation(&f, "003", &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    f.action.lock().unwrap().panic = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| consume(
            &f,
            &live,
            Some(&evidence)
        )))
        .is_err()
    );
    assert_eq!(live.remaining_limits().max_preparations, 0);
    assert_eq!(live.remaining_limits().validation.max_rules, 0);
    assert_eq!(live.remaining_limits().validation.max_comparisons, 0);
    assert_eq!(live.remaining_limits().validation.max_input_values, 0);
    assert_eq!(consume(&f, &live, Some(&evidence)).accepted, None);
    assert_eq!(f.rules.load(Ordering::SeqCst), 1);
}

#[test]
fn duplicate_item_failures_keep_occurrence_ranges_and_original_decoded_spans() {
    let f = fixture(
        FacetFamily::List(ScalarRepresentation::String),
        "",
        "",
        false,
        false,
        true,
    );
    let control = OperationControl::default();
    let live = invocation(&f, "\u{2003}β a β\t", &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    let report = consume(&f, &live, Some(&evidence));
    assert_eq!(report.accepted, Some(false));
    let AttributeDatatypePhase::Pretyped {
        evidence,
        validation,
    } = report.datatype.unwrap()
    else {
        panic!()
    };
    assert_eq!(validation.item_occurrences.len(), 3);
    for (index, span) in [3..5, 6..7, 8..10].into_iter().enumerate() {
        let occurrence = &validation.item_occurrences[index];
        assert_eq!(occurrence.index, index);
        assert_eq!(evidence.token_spans()[occurrence.index], span);
        assert_eq!(occurrence.rules, index + 1..index + 2);
        let rule = &validation.completed[occurrence.rules.start];
        assert!(!rule.result.accepted);
        assert_eq!(rule.result.diagnostics.len(), 1);
    }
    assert_eq!(f.preparations.load(Ordering::SeqCst), 3);
    assert_eq!(f.rules.load(Ordering::SeqCst), 4);
    f.action.lock().unwrap().pending = true;
    let stopped = consume(&f, &live, Some(&evidence));
    assert_eq!(stopped.accepted, None);
    let AttributeDatatypePhase::Pretyped {
        evidence,
        validation,
    } = stopped.datatype.unwrap()
    else {
        panic!()
    };
    assert_eq!(validation.stopped_item, Some(0));
    assert_eq!(
        evidence.token_spans()[validation.stopped_item.unwrap()],
        3..5
    );
    assert_eq!(validation.item_occurrences[0].rules, 1..1);
}
