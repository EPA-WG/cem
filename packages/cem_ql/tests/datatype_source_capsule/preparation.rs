use super::*;
use cem_ml::schema::datatype_contracts::LexicalInput;
use cem_ml::schema::datatype_validation::CandidateRequirement;
use cem_ql::datatype_preparation::{
    NativeLexicalPreparer, PreparationBinding, PreparationCall, PreparationExecution,
    PreparationInput, PreparationLimits, PreparationSignature, RegisteredLexicalPreparation,
};
use cem_ql::preparation_evidence::EvidenceStop;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
#[derive(Clone, Default)]
struct Engine {
    calls: Arc<AtomicUsize>,
    mode: Arc<AtomicUsize>,
    action: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
    tokenizer_closed: Arc<std::sync::atomic::AtomicBool>,
    inherited_tokenizer: bool,
    tokenizer_action: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
}
impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Engine(..)")
    }
}
impl NativeLexicalPreparer for Engine {
    fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution {
        self.calls.fetch_add(1, Ordering::AcqRel);
        let action = self.action.lock().unwrap().clone();
        if let Some(action) = action {
            action();
        }
        match self.mode.load(Ordering::Acquire) {
            1 => return PreparationExecution::Rejected(vec![]),
            2 => return PreparationExecution::Pending(vec![]),
            3 => return PreparationExecution::Unavailable(vec![]),
            4 => return PreparationExecution::Failed(vec![]),
            5 => {
                return PreparationExecution::Prepared {
                    value: vec![Item::native(Impostor)],
                    diagnostics: vec![],
                }
            }
            _ => {}
        }
        PreparationExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String(
                call.lexical.text.to_uppercase(),
            ))],
            diagnostics: vec![],
        }
    }
}
fn fixture() -> (
    FreshStructuralHost,
    cem_ql::structural_receiver::FreshDatatypeCompilation,
    Engine,
) {
    fixture_with(Engine::default(), OperationControl::default())
}
fn fixture_with(
    engine: Engine,
    control: OperationControl,
) -> (
    FreshStructuralHost,
    cem_ql::structural_receiver::FreshDatatypeCompilation,
    Engine,
) {
    let capture = capture(TEXT);
    let b = capsule::export(
        &capture,
        SourceIdentity::new("file:///app/types.cem", SourceId(1), TEXT.as_bytes()),
        &hash(b"registry"),
        BTreeSet::from([node(&capture, "word"), node(&capture, "more")]),
        &OperationControl::default(),
    )
    .unwrap();
    let sources =
        Arc::new(capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap());
    let mut host = FreshStructuralHost::new(&hash(b"registry"), control).unwrap();
    let scope = host
        .register_scope(
            sources.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let mut implementations = DatatypeImplementations::default();
    for (name, kind) in [
        ("word", DatatypeKind::Scalar),
        ("words", DatatypeKind::List),
    ] {
        implementations
            .register(DatatypeImplementation {
                source: sources.source_named(name).unwrap().clone(),
                kind,
                representation: if kind == DatatypeKind::List {
                    ValueRepresentation::List(ScalarRepresentation::String)
                } else {
                    ValueRepresentation::Scalar(ScalarRepresentation::String)
                },
                accepted_bases: vec![],
                bounds: ItemBounds::default(),
                tokenizer: if kind == DatatypeKind::List {
                    let base = cem_ml::schema::datatype_contracts::RegisteredTokenizer::new(
                        "guarded-whitespace",
                        GuardedWhitespace(
                            engine.tokenizer_closed.clone(),
                            engine.tokenizer_action.clone(),
                        ),
                    )
                    .unwrap();
                    TokenizerBinding::Ready(if engine.inherited_tokenizer {
                        cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace()
                            .checked_replacement(&base)
                    } else {
                        base
                    })
                } else {
                    TokenizerBinding::Absent
                },
                validator: None,
            })
            .unwrap();
    }
    let list = sources.source_named("words").unwrap().clone();
    implementations
        .select_preparation(
            list.clone(),
            PreparationBinding::Ready(
                RegisteredLexicalPreparation::list_items(
                    list,
                    "list-items",
                    ScalarRepresentation::String,
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let source = sources.source_named("word").unwrap().clone();
    let preparer = RegisteredLexicalPreparation::new(
        source.clone(),
        "prepare",
        PreparationSignature {
            kind: DatatypeKind::Scalar,
            output: ValueRepresentation::Scalar(ScalarRepresentation::String),
            candidate: CandidateRequirement::Optional,
        },
        engine.clone(),
    )
    .unwrap();
    implementations
        .select_preparation(source, PreparationBinding::Ready(preparer))
        .unwrap();
    let compiled = host
        .compile_datatype_sources(sources, scope, &implementations, &Default::default())
        .unwrap();
    (host, compiled, engine)
}
fn input(text: &str) -> PreparationInput {
    PreparationInput {
        lexical: LexicalInput::new(Arc::from(text), Default::default()),
        candidate: vec![],
        fallback: Default::default(),
    }
}
#[test]
fn private_root_receipts_prepare_once_and_validate_without_replaying_preparation() {
    let (host, compiled, engine) = fixture();
    let invocation = compiled
        .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
        .unwrap();
    let result = invocation.prepare(&host).unwrap();
    let evidence = result.evidence.unwrap();
    assert_eq!(
        evidence.values()[0].atom(),
        Some(AtomValue::String("TEA".into()))
    );
    assert_eq!(evidence.lexical().text.as_ref(), "Tea");
    assert_eq!(result.report.accepted, None);
    assert_eq!(
        invocation
            .validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(
        invocation
            .validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(engine.calls.load(Ordering::Acquire), 1);
}

#[derive(Debug)]
struct Impostor;
impl cem_ql::eval::QueryItemView for Impostor {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.typed-atomic"
    }
    fn identity(&self) -> String {
        "fake".into()
    }
    fn kind(&self) -> cem_ql::eval::QueryItemViewKind {
        cem_ql::eval::QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<AtomValue> {
        Some(AtomValue::String("TEA".into()))
    }
}
#[test]
fn list_receipts_retain_decoded_spans_and_present_empty_values() {
    let (host, compiled, engine) = fixture();
    let invocation = compiled
        .preparation_invocation(
            &host,
            "more",
            input("Tea  coffee"),
            PreparationLimits::default(),
        )
        .unwrap();
    let prepared = invocation.prepare(&host).unwrap();
    let evidence = prepared.evidence.unwrap();
    assert_eq!(evidence.values().len(), 2);
    assert_eq!(evidence.token_spans(), &[0..3, 5..11]);
    assert_eq!(evidence.lexical().text.as_ref(), "Tea  coffee");
    assert_eq!(
        invocation
            .validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(engine.calls.load(Ordering::Acquire), 2);
    let empty = compiled
        .preparation_invocation(&host, "more", input(""), PreparationLimits::default())
        .unwrap();
    let evidence = empty.prepare(&host).unwrap().evidence.unwrap();
    assert!(evidence.values().is_empty());
    assert_eq!(
        empty
            .validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(false)
    );
    assert!(matches!(
        empty.validate_prepared(&host, None),
        Err(EvidenceStop::MissingEvidence)
    ));
}
#[test]
fn frozen_invocations_refuse_foreign_receipts_roots_and_hosts_without_spending_budget() {
    let (host, compiled, _) = fixture();
    let first = compiled
        .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
        .unwrap();
    let second = compiled
        .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
        .unwrap();
    let list = compiled
        .preparation_invocation(&host, "more", input("Tea"), PreparationLimits::default())
        .unwrap();
    let evidence = first.prepare(&host).unwrap().evidence.unwrap();
    let before = second.remaining_limits().validation.max_input_values;
    assert!(matches!(
        second.validate_prepared(&host, Some(&evidence)),
        Err(EvidenceStop::ForeignInvocation)
    ));
    assert_eq!(
        second.remaining_limits().validation.max_input_values,
        before
    );
    assert!(matches!(
        list.validate_prepared(&host, Some(&evidence)),
        Err(EvidenceStop::ForeignInvocation)
    ));
    let (foreign, other, _) = fixture();
    assert!(first.prepare(&foreign).is_err());
    assert!(other
        .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
        .is_err());
    assert!(compiled
        .preparation_invocation(&host, "short", input("Tea"), PreparationLimits::default())
        .is_err());
    assert!(evidence.is_live());
    assert_eq!(
        first
            .validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
}
#[test]
fn dropping_retiring_or_changing_owners_invalidates_retained_evidence() {
    for case in 0..5 {
        let (mut host, compiled, _) = fixture();
        let invocation = compiled
            .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
            .unwrap();
        let evidence = invocation.prepare(&host).unwrap().evidence.unwrap();
        match case {
            0 => compiled.retire(),
            1 => drop(compiled),
            2 => host.close(),
            3 => {
                host.register_scope(
                    compiled.sources().source().clone(),
                    None,
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            }
            _ => invocation.close(),
        }
        assert!(!evidence.is_live(), "case {case}");
        assert!(invocation
            .validate_prepared(&host, Some(&evidence))
            .is_err());
    }
    let (host, compiled, _) = fixture();
    let invocation = compiled
        .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
        .unwrap();
    let evidence = invocation.prepare(&host).unwrap().evidence.unwrap();
    drop(invocation);
    assert!(!evidence.is_live());
}
#[test]
fn preparation_and_consumption_share_cumulative_work_even_for_repeated_evidence() {
    let (host, compiled, engine) = fixture();
    let limits = PreparationLimits {
        max_preparations: 1,
        validation: ValidationLimits {
            max_input_values: 7,
            ..Default::default()
        },
        ..Default::default()
    };
    let invocation = compiled
        .preparation_invocation(&host, "word", input("Tea"), limits)
        .unwrap();
    let evidence = invocation.prepare(&host).unwrap().evidence.unwrap();
    assert_eq!(invocation.remaining_limits().max_preparations, 0);
    for _ in 0..2 {
        assert_eq!(
            invocation
                .validate_prepared(&host, Some(&evidence))
                .unwrap()
                .accepted,
            Some(true)
        );
    }
    assert!(matches!(
        invocation.validate_prepared(&host, Some(&evidence)),
        Err(EvidenceStop::Limit(_))
    ));
    let repeated = invocation.prepare(&host).unwrap();
    assert!(repeated.evidence.is_none());
    assert!(matches!(
        repeated.stopped,
        Some(EvidenceStop::Preparation(
            cem_ql::datatype_preparation::PreparationStop::Limit(_)
        ))
    ));
    assert_eq!(engine.calls.load(Ordering::Acquire), 1);
}
#[test]
fn inspection_reports_cannot_become_evidence_after_rejection_stops_or_untracked_views() {
    for mode in 1..=5 {
        let (host, compiled, engine) = fixture();
        engine.mode.store(mode, Ordering::Release);
        let invocation = compiled
            .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
            .unwrap();
        let result = invocation.prepare(&host).unwrap();
        assert!(result.evidence.is_none(), "mode {mode}");
        if mode == 1 {
            assert_eq!(result.report.accepted, Some(false));
            assert!(result.stopped.is_none());
        } else if mode == 5 {
            assert!(matches!(result.stopped, Some(EvidenceStop::UntrackedView)));
        } else {
            assert!(matches!(result.stopped, Some(EvidenceStop::Preparation(_))));
        }
    }
}
#[test]
fn original_attribute_candidate_and_input_metadata_are_checked_before_preparation() {
    let (host, compiled, engine) = fixture();
    let source = compiled
        .sources()
        .source_named("word")
        .unwrap()
        .attribute("name")
        .unwrap();
    let candidate =
        cem_ql::eval::RetainedCemNode::new(compiled.sources().source().clone(), source.node_id())
            .unwrap()
            .query_item();
    let lexical = match source.node() {
        CemAstNode::Attribute { source, .. } => source.clone(),
        _ => unreachable!(),
    };
    let mut original = input("word");
    original.lexical.source = lexical;
    original.candidate = vec![candidate];
    let mut changed = original.clone();
    changed.lexical.text = Arc::from("wrong");
    assert!(matches!(
        compiled.preparation_invocation(&host, "word", changed, PreparationLimits::default()),
        Err(EvidenceStop::SourceMismatch)
    ));
    let invocation = compiled
        .preparation_invocation(&host, "word", original, PreparationLimits::default())
        .unwrap();
    let evidence = invocation.prepare(&host).unwrap().evidence.unwrap();
    assert!(matches!(
        evidence.origin(),
        cem_ql::preparation_evidence::PreparationOrigin::Attribute(_)
    ));
    let mut huge = input("Tea");
    huge.fallback.uri =
        Some("x".repeat(cem_ql::preparation_evidence::MAX_EVIDENCE_METADATA_BYTES + 1));
    assert!(matches!(
        compiled.preparation_invocation(&host, "word", huge, PreparationLimits::default()),
        Err(EvidenceStop::Limit("metadata-bytes"))
    ));
    assert_eq!(engine.calls.load(Ordering::Acquire), 1);
}
#[test]
fn reentrant_calls_cannot_obtain_another_allowance_and_unwind_spends_the_reservation() {
    let (host, compiled, engine) = fixture();
    let host = Arc::new(host);
    let invocation = Arc::new(
        compiled
            .preparation_invocation(
                &host,
                "word",
                input("Tea"),
                PreparationLimits {
                    max_preparations: 2,
                    ..Default::default()
                },
            )
            .unwrap(),
    );
    let weak = Arc::downgrade(&invocation);
    let callback_host = host.clone();
    *engine.action.lock().unwrap() = Some(Arc::new(move || {
        assert!(matches!(
            weak.upgrade()
                .unwrap()
                .prepare(&callback_host)
                .unwrap()
                .stopped,
            Some(EvidenceStop::Busy)
        ));
    }));
    assert!(invocation.prepare(&host).unwrap().evidence.is_some());
    assert_eq!(invocation.remaining_limits().max_preparations, 1);
    *engine.action.lock().unwrap() = Some(Arc::new(|| panic!("fixture panic")));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| invocation.prepare(&host)))
            .is_err()
    );
    engine.action.lock().unwrap().take();
    assert_eq!(invocation.remaining_limits().max_preparations, 0);
    assert!(invocation.prepare(&host).unwrap().evidence.is_none());
}
#[test]
fn callback_time_cancellation_or_compilation_retirement_cannot_issue_evidence() {
    for retired in [false, true] {
        let control = OperationControl::default();
        let (host, compiled, engine) = fixture_with(Engine::default(), control.clone());
        let compiled = Arc::new(compiled);
        let invocation = compiled
            .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
            .unwrap();
        let owner = Arc::downgrade(&compiled);
        *engine.action.lock().unwrap() = Some(Arc::new(move || {
            if retired {
                owner.upgrade().unwrap().retire()
            } else {
                control.cancel_root(None, None).unwrap();
            }
        }));
        assert!(invocation.prepare(&host).is_err());
        engine.action.lock().unwrap().take();
    }
}

struct GuardedWhitespace(
    Arc<std::sync::atomic::AtomicBool>,
    Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
);
impl std::fmt::Debug for GuardedWhitespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GuardedWhitespace(..)")
    }
}
impl cem_ml::schema::datatype_contracts::Tokenizer for GuardedWhitespace {
    fn check_available(&self) -> Result<(), cem_ml::schema::datatype_contracts::TokenizationError> {
        let action = self.1.lock().unwrap().clone();
        if let Some(action) = action {
            action();
        }
        if self.0.load(Ordering::Acquire) {
            Err(cem_ml::schema::datatype_contracts::TokenizationError::Unavailable)
        } else {
            Ok(())
        }
    }
    fn tokenize(
        &self,
        request: cem_ml::schema::datatype_contracts::TokenizationRequest<'_>,
    ) -> Result<Vec<std::ops::Range<usize>>, cem_ml::schema::datatype_contracts::TokenizationError>
    {
        let values = cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace()
            .tokenize(
                Some(request.input.clone()),
                request.control,
                request.scope,
                request.limits,
            )?;
        Ok(values.tokens)
    }
}
#[test]
fn retained_list_evidence_expires_when_the_selected_tokenizer_retires() {
    for inherited_tokenizer in [false, true] {
        let (host, compiled, engine) = fixture_with(
            Engine {
                inherited_tokenizer,
                ..Default::default()
            },
            OperationControl::default(),
        );

        let invocation = compiled
            .preparation_invocation(&host, "more", input("Tea"), PreparationLimits::default())
            .unwrap();
        let evidence = invocation.prepare(&host).unwrap().evidence.unwrap();
        assert!(evidence.is_live());
        engine.tokenizer_closed.store(true, Ordering::Release);
        assert!(!evidence.is_live());
        assert!(invocation
            .validate_prepared(&host, Some(&evidence))
            .is_err());
    }
}

#[derive(Clone, Default)]
struct Rule {
    calls: Arc<AtomicUsize>,
    reject: Arc<std::sync::atomic::AtomicBool>,
    pending: Arc<std::sync::atomic::AtomicBool>,
    action: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
}
impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Rule(..)")
    }
}
impl cem_ql::datatype_validation::NativeDatatypeValidator for Rule {
    fn validate(
        &self,
        _: cem_ql::datatype_validation::ValidationCall<'_>,
    ) -> cem_ql::datatype_validation::RuleExecution {
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(action) = self.action.lock().unwrap().clone() {
            action()
        }
        if self.pending.load(Ordering::Acquire) {
            return cem_ql::datatype_validation::RuleExecution::Pending(vec![]);
        }
        cem_ql::datatype_validation::RuleExecution::Complete(cem_ql::eval::ItemStream::once(
            Item::Record(std::collections::BTreeMap::from([
                (
                    "accepted".into(),
                    vec![Item::Atomic(AtomValue::Boolean(
                        !self.reject.load(Ordering::Acquire),
                    ))],
                ),
                ("diagnostics".into(), vec![]),
            ])),
        ))
    }
}
fn declaration_node(
    c: &cem_ml::schema::machine::LexicallyScopedDocument,
    name: &str,
) -> cem_ml::schema::declaration_references::SchemaDeclarationNode {
    let id = c
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == name => Some(*node_id),
            _ => None,
        })
        .unwrap();
    cem_ml::schema::declaration_references::SchemaDeclarationNode::new(c.document().clone(), id)
        .unwrap()
}
fn rule_fixture() -> (
    FreshStructuralHost,
    cem_ql::structural_receiver::FreshDatatypeCompilation,
    Engine,
    Rule,
) {
    use cem_ml::schema::{
        datatype_validation::{
            DatatypeBehaviorContract, ResultRepresentation, ValidationSignature,
        },
        value_contracts::{ContractName, ValueContractSource, ValueContracts},
    };
    use std::collections::BTreeMap;
    let ns = "https://cem.dev/ns/schema/1";
    let text=format!("{} }} {{behaviors | {{behavior @name=check @implementation=engine @primitive=urn:test:validate @execution=datatype-validation | {{inputs | {{input-binding @name=value @type=schema:string @source=value @required=true @cardinality=one}}{{input-binding @name=datatype @type=schema:node @source=datatype @required=true @cardinality=one}}{{input-binding @name=candidate @type=schema:node @source=candidate @required=false @cardinality=zero-or-one}}}} {{result @type=schema:datatype-validation-result}}}}}} }}",TEXT.trim_end().strip_suffix("}}").unwrap());
    let c = capture(&text);
    let b = capsule::export(
        &c,
        SourceIdentity::new("file:///app/types.cem", SourceId(1), text.as_bytes()),
        &hash(b"registry"),
        BTreeSet::from([node(&c, "word")]),
        &OperationControl::default(),
    )
    .unwrap();
    let sources =
        Arc::new(capsule::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap());
    let owner = declaration_node(sources.capture(), "schema");
    let behavior = declaration_node(sources.capture(), "behavior");
    let signature = ValidationSignature {
        kind: DatatypeKind::Scalar,
        value: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
        result: ResultRepresentation::Accepted(ContractName::new(ns, "datatype-validation-result")),
    };
    let contract_source = ValueContractSource::new(
        owner.clone(),
        ns,
        BTreeMap::from([("schema".into(), ns.into())]),
    )
    .with_captured_names(sources.capture().clone())
    .unwrap();
    let contract =
        DatatypeBehaviorContract::compile(&contract_source, &behavior, signature).unwrap();
    let schema = capture(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    let src = ValueContractSource::new(
        declaration_node(&schema, "schema"),
        ns,
        BTreeMap::from([("schema".into(), ns.into())]),
    )
    .with_captured_names(schema)
    .unwrap();
    let adapter = cem_ql::datatype_results::DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[src], Default::default()).unwrap()),
        ContractName::new(ns, "datatype-validation-result"),
        ContractName::new(ns, "datatype-diagnostic"),
    )
    .unwrap();
    let rule = Rule::default();
    let mut validations = cem_ql::datatype_validation::DatatypeValidationRegistry::default();
    validations
        .register_native("urn:test:validate", contract, adapter, None, rule.clone())
        .unwrap();
    let mut implementations = DatatypeImplementations::default();
    let source = sources.source_named("word").unwrap().clone();
    implementations
        .register(DatatypeImplementation {
            source: source.clone(),
            kind: DatatypeKind::Scalar,
            representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
            accepted_bases: vec![],
            bounds: ItemBounds::default(),
            tokenizer: TokenizerBinding::Absent,
            validator: Some((owner, behavior)),
        })
        .unwrap();
    let engine = Engine::default();
    implementations
        .select_preparation(
            source.clone(),
            PreparationBinding::Ready(
                RegisteredLexicalPreparation::new(
                    source,
                    "prepare",
                    PreparationSignature {
                        kind: DatatypeKind::Scalar,
                        output: ValueRepresentation::Scalar(ScalarRepresentation::String),
                        candidate: CandidateRequirement::Optional,
                    },
                    engine.clone(),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let scope = host
        .register_scope(
            sources.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let compiled = host
        .compile_datatype_sources(sources, scope, &implementations, &validations)
        .unwrap();
    (host, compiled, engine, rule)
}
#[test]
fn each_consumption_runs_current_rules_and_never_reuses_a_validation_verdict() {
    let (host, compiled, engine, rule) = rule_fixture();
    let live = compiled
        .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
        .unwrap();
    let evidence = live.prepare(&host).unwrap().evidence.unwrap();
    assert_eq!(rule.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        live.validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
    rule.reject.store(true, Ordering::Release);
    assert_eq!(
        live.validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(false)
    );
    rule.pending.store(true, Ordering::Release);
    assert_eq!(
        live.validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        None
    );
    assert_eq!(rule.calls.load(Ordering::Acquire), 3);
    assert_eq!(engine.calls.load(Ordering::Acquire), 1);
}
#[test]
fn validation_reentrancy_and_rule_budget_share_the_original_invocation() {
    let (host, compiled, engine, rule) = rule_fixture();
    let host = Arc::new(host);
    let live = Arc::new(
        compiled
            .preparation_invocation(
                &host,
                "word",
                input("Tea"),
                PreparationLimits {
                    validation: ValidationLimits {
                        max_rules: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap(),
    );
    let evidence = live.prepare(&host).unwrap().evidence.unwrap();
    let weak = Arc::downgrade(&live);
    let callback_host = host.clone();
    let original = evidence.clone();
    *rule.action.lock().unwrap() = Some(Arc::new(move || {
        let live = weak.upgrade().unwrap();
        assert!(matches!(
            live.validate_prepared(&callback_host, Some(&original)),
            Err(EvidenceStop::Busy)
        ));
        assert!(matches!(
            live.prepare(&callback_host).unwrap().stopped,
            Some(EvidenceStop::Busy)
        ));
    }));
    assert_eq!(
        live.validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
    rule.action.lock().unwrap().take();
    assert_eq!(live.remaining_limits().validation.max_rules, 0);
    assert_eq!(
        live.validate_prepared(&host, Some(&evidence))
            .unwrap()
            .accepted,
        None
    );
    assert_eq!(rule.calls.load(Ordering::Acquire), 1);
    assert_eq!(engine.calls.load(Ordering::Acquire), 1);
}
#[test]
fn callback_retirement_cannot_publish_a_prepared_validation_result() {
    let (host, compiled, _, rule) = rule_fixture();
    let compiled = Arc::new(compiled);
    let live = compiled
        .preparation_invocation(&host, "word", input("Tea"), PreparationLimits::default())
        .unwrap();
    let evidence = live.prepare(&host).unwrap().evidence.unwrap();
    let owner = Arc::downgrade(&compiled);
    *rule.action.lock().unwrap() = Some(Arc::new(move || owner.upgrade().unwrap().retire()));
    assert!(live.validate_prepared(&host, Some(&evidence)).is_err());
    rule.action.lock().unwrap().take();
    assert!(!evidence.is_live());
}
#[test]
fn typed_only_compilations_do_not_gain_implicit_lexical_preparation() {
    let sources = verified();
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let scope = host
        .register_scope(
            sources.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let compiled = host
        .compile_datatype_sources(
            sources.clone(),
            scope,
            &implementations(&sources),
            &Default::default(),
        )
        .unwrap();
    let live = compiled
        .preparation_invocation(&host, "more", input("Tea"), PreparationLimits::default())
        .unwrap();
    let result = live.prepare(&host).unwrap();
    assert!(result.evidence.is_none());
    assert!(matches!(
        result.stopped,
        Some(EvidenceStop::Preparation(
            cem_ql::datatype_preparation::PreparationStop::NoPreparation
        ))
    ));
}

#[test]
fn availability_callbacks_cannot_prepare_reentrantly_before_budget_reservation() {
    let (host, compiled, engine) = fixture();
    let host = Arc::new(host);
    let live = Arc::new(
        compiled
            .preparation_invocation(&host, "more", input("Tea"), PreparationLimits::default())
            .unwrap(),
    );
    let weak = Arc::downgrade(&live);
    let callback_host = host.clone();
    let first = Arc::new(std::sync::atomic::AtomicBool::new(true));
    *engine.tokenizer_action.lock().unwrap() = Some(Arc::new(move || {
        if first.swap(false, Ordering::AcqRel) {
            assert!(matches!(
                weak.upgrade()
                    .unwrap()
                    .prepare(&callback_host)
                    .unwrap()
                    .stopped,
                Some(EvidenceStop::Busy)
            ));
        }
    }));
    assert!(live.prepare(&host).unwrap().evidence.is_some());
    engine.tokenizer_action.lock().unwrap().take();
    assert_eq!(engine.calls.load(Ordering::Acquire), 1);
}
