use super::*;
use cem_ml::schema::{
    datatype_contracts::{LexicalInput, RegisteredTokenizer},
    document_model::{attribute_facets::FacetFamily, shipped_datatypes::ShippedDatatype as T},
};
use cem_ql::{
    attribute_datatypes::bind_attribute_datatype,
    datatype_facets::{BoundAttributeFacets, FacetProfileBinding, RegisteredFacetProfile},
    datatype_preparation::*,
    preparation_evidence::*,
};

fn fixture(ty: T, list: bool) -> BoundAttributeFacets {
    build(ty, list, |s| {
        cem_ql::datatype_shipped::lexical_preparation(s.clone(), ty).unwrap()
    })
}
fn build(
    ty: T,
    list: bool,
    preparer: impl FnOnce(&DatatypeSource) -> RegisteredLexicalPreparation,
) -> BoundAttributeFacets {
    let declarations = if list {
        "{type @name=item @kind=scalar} {type @name=sample @kind=list @base=item}"
    } else {
        "{type @name=sample @kind=scalar}"
    };
    let profile = source(&format!("{{schema @name=test @namespace=urn:test | {{types | {declarations}}} {{attributes | {{attribute @name=value @type=sample}}}}}}"));
    let declaration = node(&profile, "attribute");
    let (mut host, sources) = types_fixture_source(profile);
    let mut implementations = DatatypeImplementations::default();
    let sample = sources.last().unwrap();
    let item = &sources[0];
    implementations
        .register(implementation(
            item,
            DatatypeKind::Scalar,
            ty.representation(),
        ))
        .unwrap();
    implementations
        .select_preparation(item.clone(), PreparationBinding::Ready(preparer(item)))
        .unwrap();
    let family = if list {
        let ValueRepresentation::Scalar(p) = ty.representation() else {
            panic!()
        };
        let mut entry = implementation(sample, DatatypeKind::List, ValueRepresentation::List(p));
        entry.tokenizer = TokenizerBinding::Ready(RegisteredTokenizer::whitespace());
        implementations.register(entry).unwrap();
        implementations
            .select_preparation(
                sample.clone(),
                PreparationBinding::Ready(
                    RegisteredLexicalPreparation::list_items(sample.clone(), "list", p).unwrap(),
                ),
            )
            .unwrap();
        FacetFamily::List(p)
    } else {
        FacetFamily::Shipped(ty)
    };
    implementations
        .select_facets(
            sample.clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(sample.clone(), "facets", family).unwrap(),
            ),
        )
        .unwrap();
    let compilation = compile_datatypes(
        sample.declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(compilation.is_ready(), "{:?}", compilation.issues);
    let slot = declaration
        .document()
        .get(declaration.node_id())
        .and_then(|n| match n {
            CemAstNode::Element { attributes, .. } => {
                attributes
                    .iter()
                    .find_map(|id| match declaration.document().get(*id) {
                        Some(CemAstNode::Attribute { expanded_name, .. })
                            if expanded_name.local_name == "type" =>
                        {
                            SchemaDeclarationNode::new(declaration.document().clone(), *id)
                        }
                        _ => None,
                    })
            }
            _ => None,
        })
        .unwrap();
    host.bind_literal_attribute_type(slot, sample.declaration().clone())
        .unwrap();
    bind_attribute_datatype(
        declaration,
        &compilation,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap()
    .compile_facets("urn:test", Default::default())
    .unwrap()
}
fn input(text: &str) -> PreparationInput {
    PreparationInput {
        lexical: LexicalInput::new(Arc::from(text), Default::default()),
        candidate: vec![],
        fallback: Default::default(),
    }
}
fn invocation(
    bound: &BoundAttributeFacets,
    input: PreparationInput,
    control: &OperationControl,
    limits: PreparationLimits,
) -> AttributePreparationInvocation {
    AttributePreparationInvocation::new(
        bound,
        input,
        &ValidationRuntime {
            control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        limits,
    )
    .unwrap()
}

#[test]
fn evidence_keeps_original_spelling_and_does_not_issue_an_acceptance_verdict() {
    for (ty, text, atom) in [
        (T::Integer, "003", AtomValue::Integer(3)),
        (T::Boolean, "", AtomValue::Boolean(true)),
        (T::Boolean, " false ", AtomValue::Boolean(false)),
    ] {
        let bound = fixture(ty, false);
        let control = OperationControl::default();
        let request = input(text);
        let lexical = request.lexical.text.clone();
        let live = invocation(&bound, request, &control, Default::default());
        let result = live.prepare();
        assert!(result.stopped.is_none(), "{result:?}");
        assert!(result.report.validation.is_none());
        assert_eq!(result.report.accepted, None);
        let evidence = result.evidence.unwrap();
        assert_eq!(evidence.values()[0].atom(), Some(atom));
        assert!(Arc::ptr_eq(&evidence.lexical().text, &lexical));
        assert_eq!(&*evidence.lexical().text, text);
        assert!(evidence.verify(&live).is_ok());
        assert!(evidence.matches_binding(&bound));
        assert!(!evidence.matches_binding(&fixture(ty, false)));
        assert!(evidence.candidate().is_empty());
    }
    let bound = fixture(T::Integer, false);
    let control = OperationControl::default();
    let live = invocation(
        &bound,
        input("9223372036854775808"),
        &control,
        Default::default(),
    );
    let evidence = live.prepare().evidence.unwrap();
    assert!(evidence.values()[0].view().is_some());
    assert_eq!(&*evidence.lexical().text, "9223372036854775808");
    let token_descriptor = super::shipped_lists::list_descriptor(T::NameList, None);
    let token = super::shipped_conversion::convert(&token_descriptor, "β")
        .value
        .unwrap()
        .remove(0);
    let bound = callback_bound(
        PreparationExecution::Prepared {
            value: vec![token.clone()],
            diagnostics: vec![],
        },
        false,
        Arc::new(AtomicUsize::new(0)),
    );
    let live = invocation(&bound, input("β"), &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    assert_eq!(evidence.values()[0].identity(), token.identity());
    let (original, span) = cem_ql::datatype_shipped::token_source(&token).unwrap();
    let (retained, retained_span) =
        cem_ql::datatype_shipped::token_source(&evidence.values()[0]).unwrap();
    assert!(std::ptr::eq(original, retained));
    assert_eq!(span, retained_span);
}

#[test]
fn list_evidence_retains_order_duplicates_unicode_spans_and_empty_input() {
    let bound = fixture(T::String, true);
    let control = OperationControl::default();
    let live = invocation(
        &bound,
        input("\u{2003}β a β\t"),
        &control,
        Default::default(),
    );
    let mut result = live.prepare();
    let evidence = result.evidence.unwrap();
    assert_eq!(
        evidence.values().iter().map(Item::atom).collect::<Vec<_>>(),
        vec![
            Some(AtomValue::String("β".into())),
            Some(AtomValue::String("a".into())),
            Some(AtomValue::String("β".into()))
        ]
    );
    assert_eq!(evidence.token_spans(), &[3..5, 6..7, 8..10]);
    result.report.input.lexical.text = Arc::from("forged");
    result.report.value.as_mut().unwrap()[0] = Item::Atomic(AtomValue::String("forged".into()));
    result.report.token_spans.clear();
    assert_eq!(&*evidence.lexical().text, "\u{2003}β a β\t");
    assert_eq!(evidence.token_spans().len(), 3);
    assert_eq!(
        evidence.values()[0].atom(),
        Some(AtomValue::String("β".into()))
    );
    let empty = invocation(&bound, input(" \t"), &control, Default::default());
    let evidence = empty.prepare().evidence.unwrap();
    assert!(evidence.values().is_empty());
    assert!(evidence.token_spans().is_empty());
    assert_eq!(&*evidence.lexical().text, " \t");
}

#[test]
fn lexical_rejection_partial_preparation_and_limits_cannot_issue_evidence() {
    let control = OperationControl::default();
    let boolean = fixture(T::Boolean, false);
    for text in ["1", "0"] {
        let live = invocation(&boolean, input(text), &control, Default::default());
        let result = live.prepare();
        assert!(result.evidence.is_none());
        assert_eq!(result.report.accepted, Some(false));
    }
    let list = fixture(T::Integer, true);
    let live = invocation(&list, input("003 invalid"), &control, Default::default());
    let result = live.prepare();
    assert!(result.evidence.is_none());
    assert!(result.report.value.is_none());
    let mut limits = PreparationLimits::default();
    limits.max_preparations = 1;
    let live = invocation(&list, input("003 004"), &control, limits);
    assert!(live.prepare().evidence.is_none());
}

#[test]
fn clones_share_budget_and_evidence_expires_on_close_drop_or_cancellation() {
    let bound = fixture(T::Integer, false);
    let control = OperationControl::default();
    let mut limits = PreparationLimits::default();
    limits.max_preparations = 1;
    let live = invocation(&bound, input("003"), &control, limits);
    let clone = live.clone();
    let evidence = live.prepare().evidence.unwrap();
    assert!(clone.prepare().evidence.is_none());
    assert_eq!(clone.remaining_limits().max_preparations, 0);
    let foreign = invocation(&bound, input("003"), &control, Default::default());
    assert!(evidence.verify(&foreign).is_err());
    clone.close();
    assert!(evidence.verify(&live).is_err());
    assert!(live.prepare().evidence.is_none());
    let live = invocation(&bound, input("003"), &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    drop(live);
    assert!(!evidence.is_live());
    let live = invocation(&bound, input("003"), &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    control.cancel_root(None, None).unwrap();
    assert!(evidence.verify(&live).is_err());
    assert!(live.prepare().evidence.is_none());
}

#[derive(Debug)]
struct Callback {
    execution: PreparationExecution,
    calls: Arc<AtomicUsize>,
}
impl NativeLexicalPreparer for Callback {
    fn prepare(&self, _: PreparationCall<'_>) -> PreparationExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.execution.clone()
    }
}
fn callback_bound(
    execution: PreparationExecution,
    required: bool,
    calls: Arc<AtomicUsize>,
) -> BoundAttributeFacets {
    build(T::String, false, |s| {
        RegisteredLexicalPreparation::new(
            s.clone(),
            "callback",
            PreparationSignature {
                kind: DatatypeKind::Scalar,
                output: T::String.representation(),
                candidate: if required {
                    CandidateRequirement::Required
                } else {
                    CandidateRequirement::Optional
                },
            },
            Callback { execution, calls },
        )
        .unwrap()
    })
}

#[test]
fn incomplete_or_malformed_callbacks_and_untracked_native_views_are_not_sealed() {
    #[derive(Debug)]
    struct Mutable;
    impl cem_ql::eval::QueryItemView for Mutable {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "cem.typed-atomic"
        }
        fn identity(&self) -> String {
            "imitated".into()
        }
        fn kind(&self) -> cem_ql::eval::QueryItemViewKind {
            cem_ql::eval::QueryItemViewKind::Atomic
        }
        fn atom(&self) -> Option<AtomValue> {
            Some(AtomValue::String("value".into()))
        }
    }
    for execution in [
        PreparationExecution::Pending(vec![]),
        PreparationExecution::Unavailable(vec![]),
        PreparationExecution::Failed(vec![]),
        PreparationExecution::Limit("work"),
        PreparationExecution::Prepared {
            value: vec![],
            diagnostics: vec![],
        },
        PreparationExecution::Prepared {
            value: vec![Item::native(Mutable)],
            diagnostics: vec![],
        },
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let bound = callback_bound(execution, false, calls.clone());
        let control = OperationControl::default();
        let live = invocation(&bound, input("value"), &control, Default::default());
        let result = live.prepare();
        assert!(result.evidence.is_none(), "{result:?}");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn source_candidate_must_be_original_literal_attribute_with_matching_text_and_map() {
    let calls = Arc::new(AtomicUsize::new(0));
    let bound = callback_bound(
        PreparationExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String("sample".into()))],
            diagnostics: vec![],
        },
        true,
        calls.clone(),
    );
    let control = OperationControl::default();
    let absent = invocation(&bound, input("sample"), &control, Default::default());
    assert!(absent.prepare().evidence.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let candidate = native(&bound.binding().slot);
    let mut request = input("sample");
    request.lexical.source = candidate.source_map().unwrap();
    request.candidate = vec![candidate.clone()];
    let live = invocation(&bound, request.clone(), &control, Default::default());
    let evidence = live.prepare().evidence.unwrap();
    assert_eq!(evidence.candidate()[0].identity(), candidate.identity());
    assert_eq!(evidence.lexical().source, candidate.source_map().unwrap());
    let mut wrong_map = request.clone();
    assert!(!wrong_map.lexical.source.frames.is_empty());
    wrong_map.lexical.source.frames.clear();
    assert!(matches!(
        AttributePreparationInvocation::new(
            &bound,
            wrong_map,
            &ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default(),
            },
            Default::default()
        ),
        Err(EvidenceStop::SourceMismatch)
    ));
    request.lexical.text = Arc::from("forged");
    assert!(AttributePreparationInvocation::new(
        &bound,
        request,
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default()
        },
        Default::default()
    )
    .is_err());
    let mut public = live.prepare().report;
    public.input.candidate.clear();
    assert_eq!(evidence.candidate().len(), 1);
}

#[test]
fn evidence_is_issued_before_a_rejecting_datatype_rule_without_running_that_rule() {
    let bound = super::attribute_consumer::fixture(
        FacetFamily::Shipped(T::Integer),
        "",
        true,
        false,
        false,
    );
    let control = OperationControl::default();
    let live = invocation(&bound, input("003"), &control, Default::default());
    let result = live.prepare();
    assert!(result.evidence.is_some());
    assert!(result.report.validation.is_none());
    assert_eq!(result.report.accepted, None);
    let validation = validate_descriptor(
        &bound.binding().datatype,
        result.evidence.unwrap().values().to_vec(),
    );
    assert_eq!(validation.accepted, Some(false));
}

#[test]
fn callback_cancellation_and_oversized_prepared_text_cannot_publish_evidence() {
    #[derive(Debug)]
    struct Cancel;
    impl NativeLexicalPreparer for Cancel {
        fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution {
            call.runtime.control.cancel_root(None, None).unwrap();
            PreparationExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::String("value".into()))],
                diagnostics: vec![],
            }
        }
    }
    let bound = build(T::String, false, |s| {
        RegisteredLexicalPreparation::new(
            s.clone(),
            "cancel",
            PreparationSignature {
                kind: DatatypeKind::Scalar,
                output: T::String.representation(),
                candidate: CandidateRequirement::Optional,
            },
            Cancel,
        )
        .unwrap()
    });
    let control = OperationControl::default();
    let live = invocation(&bound, input("value"), &control, Default::default());
    assert!(live.prepare().evidence.is_none());
    let control = OperationControl::default();
    let bound = callback_bound(
        PreparationExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String("too large".into()))],
            diagnostics: vec![],
        },
        false,
        Arc::new(AtomicUsize::new(0)),
    );
    let mut limits = PreparationLimits::default();
    limits.max_lexical_bytes = 1;
    let live = invocation(&bound, input("a"), &control, limits);
    let result = live.prepare();
    assert!(matches!(
        result.stopped,
        Some(EvidenceStop::Limit("value-bytes"))
    ));
    assert!(result.evidence.is_none());
}

#[test]
fn evidence_metadata_is_bounded_before_retention() {
    let control = OperationControl::default();
    let bound = fixture(T::String, false);
    let mut request = input("a");
    request.fallback.uri = Some("x".repeat(MAX_EVIDENCE_METADATA_BYTES + 1));
    assert!(matches!(
        AttributePreparationInvocation::new(
            &bound,
            request,
            &ValidationRuntime {
                control: &control,
                scope: ROOT_EXECUTION_SCOPE_ID,
                query: Default::default()
            },
            Default::default()
        ),
        Err(EvidenceStop::Limit("metadata-bytes"))
    ));
    let diagnostic = cem_ml::diagnostics::Diagnostic {
        details: Some(serde_json::json!({"text": "x".repeat(MAX_EVIDENCE_METADATA_BYTES + 1)})),
        ..Default::default()
    };
    let bound = callback_bound(
        PreparationExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String("a".into()))],
            diagnostics: vec![diagnostic],
        },
        false,
        Arc::new(AtomicUsize::new(0)),
    );
    let live = invocation(&bound, input("a"), &control, Default::default());
    let result = live.prepare();
    assert!(result.evidence.is_none());
    assert!(matches!(
        result.stopped,
        Some(EvidenceStop::Limit("metadata-bytes"))
    ));
}

#[test]
fn reentrant_issuance_cannot_obtain_another_allowance_and_unwind_spends_the_reservation() {
    #[derive(Debug)]
    struct Reenter(Arc<std::sync::Mutex<Option<AttributePreparationInvocation>>>);
    impl NativeLexicalPreparer for Reenter {
        fn prepare(&self, _: PreparationCall<'_>) -> PreparationExecution {
            let live = self.0.lock().unwrap().as_ref().unwrap().clone();
            assert!(matches!(live.prepare().stopped, Some(EvidenceStop::Busy)));
            PreparationExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::String("a".into()))],
                diagnostics: vec![],
            }
        }
    }
    let slot = Arc::new(std::sync::Mutex::new(None));
    let bound = build(T::String, false, |s| {
        RegisteredLexicalPreparation::new(
            s.clone(),
            "reenter",
            PreparationSignature {
                kind: DatatypeKind::Scalar,
                output: T::String.representation(),
                candidate: CandidateRequirement::Optional,
            },
            Reenter(slot.clone()),
        )
        .unwrap()
    });
    let control = OperationControl::default();
    let mut limits = PreparationLimits::default();
    limits.max_preparations = 2;
    let live = invocation(&bound, input("a"), &control, limits);
    *slot.lock().unwrap() = Some(live.clone());
    assert!(live.prepare().evidence.is_some());
    slot.lock().unwrap().take();
    assert_eq!(live.remaining_limits().max_preparations, 1);
    #[derive(Debug)]
    struct Panic;
    impl NativeLexicalPreparer for Panic {
        fn prepare(&self, _: PreparationCall<'_>) -> PreparationExecution {
            panic!("fixture callback panic");
        }
    }
    let bound = build(T::String, false, |s| {
        RegisteredLexicalPreparation::new(
            s.clone(),
            "panic",
            PreparationSignature {
                kind: DatatypeKind::Scalar,
                output: T::String.representation(),
                candidate: CandidateRequirement::Optional,
            },
            Panic,
        )
        .unwrap()
    });
    let live = invocation(&bound, input("a"), &control, limits);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| live.prepare())).is_err());
    assert_eq!(live.remaining_limits().max_preparations, 0);
    assert_eq!(live.remaining_limits().validation.max_diagnostics, 0);
    assert!(live.prepare().evidence.is_none());
}
