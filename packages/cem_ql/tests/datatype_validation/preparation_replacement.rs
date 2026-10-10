use super::*;
use cem_ml::schema::{
    datatype_contracts::LexicalInput, document_model::shipped_datatypes::ShippedDatatype as T,
};
use cem_ql::datatype_preparation::*;

#[derive(Debug)]
struct Prepare {
    calls: Arc<AtomicUsize>,
    value: AtomValue,
}
impl NativeLexicalPreparer for Prepare {
    fn prepare(&self, _: PreparationCall<'_>) -> PreparationExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        PreparationExecution::Prepared {
            value: vec![Item::Atomic(self.value.clone())],
            diagnostics: vec![],
        }
    }
}
fn request(text: &str) -> PreparationInput {
    PreparationInput {
        lexical: LexicalInput::new(Arc::from(text), Default::default()),
        candidate: vec![],
        fallback: Default::default(),
    }
}
fn run(d: &ExecutableDatatype, text: &str, limits: PreparationLimits) -> DatatypePreparation {
    let control = OperationControl::default();
    d.prepare_lexical(
        &request(text),
        &ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        limits,
    )
}
fn replacement(
    source: &DatatypeSource,
    calls: &Arc<AtomicUsize>,
    value: AtomValue,
    required: bool,
) -> RegisteredLexicalPreparation {
    RegisteredLexicalPreparation::new(
        source.clone(),
        "replacement",
        PreparationSignature {
            kind: DatatypeKind::Scalar,
            output: T::Boolean.representation(),
            candidate: if required {
                CandidateRequirement::Required
            } else {
                CandidateRequirement::Optional
            },
        },
        Prepare {
            calls: calls.clone(),
            value,
        },
    )
    .unwrap()
}
fn fixture(value: AtomValue) -> (ExecutableDatatype, Arc<AtomicUsize>) {
    let (mut host, sources) = types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base} {type @name=leaf @base=derived}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::Boolean.representation(),
        ))
        .unwrap();
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::Boolean)
                    .unwrap(),
            ),
        )
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    implementations
        .select_preparation(
            sources[1].clone(),
            PreparationBinding::CheckedReplacement(replacement(&sources[1], &calls, value, false)),
        )
        .unwrap();
    let compilation = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(
        compilation.is_ready(),
        "{:?}",
        compilation
            .issues
            .iter()
            .map(|i| i.code)
            .collect::<Vec<_>>()
    );
    (compiled(&compilation, &sources[2]).clone(), calls)
}
#[test]
fn checked_preparer_replacement_preserves_base_admission_and_value() {
    let (d, calls) = fixture(AtomValue::Boolean(true));
    let good = run(&d, " true ", Default::default());
    assert_eq!(good.accepted, Some(true));
    assert_eq!(good.preparations, 2);
    assert_eq!(good.input.lexical.text.as_ref(), " true ");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let widening = run(&d, "1", Default::default());
    assert_eq!(widening.accepted, Some(false));
    assert!(widening.value.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let changed = run(&d, "false", Default::default());
    assert_eq!(changed.accepted, None);
    assert!(changed.value.is_none());
    assert!(matches!(
        changed.stopped,
        Some(PreparationStop::IncompatibleReplacement)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
#[test]
fn checked_preparer_replacement_preflights_all_callbacks() {
    let (d, calls) = fixture(AtomValue::Boolean(true));
    let mut limits = PreparationLimits::default();
    limits.max_preparations = 1;
    let result = run(&d, "true", limits);
    assert!(matches!(
        result.stopped,
        Some(PreparationStop::Limit("preparations"))
    ));
    assert_eq!(result.preparations, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[derive(Debug, Clone)]
struct Action {
    calls: Arc<AtomicUsize>,
    execution: PreparationExecution,
    cancel: bool,
}
impl NativeLexicalPreparer for Action {
    fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(call.original.text.as_ref(), "true");
        assert_eq!(call.lexical.text.as_ref(), "true");
        if self.cancel {
            call.runtime.control.cancel_root(None, None).unwrap();
        }
        self.execution.clone()
    }
}
fn ready(value: Item) -> PreparationExecution {
    PreparationExecution::Prepared {
        value: vec![value],
        diagnostics: vec![],
    }
}
fn action_chain(actions: &[Action], required: bool) -> ExecutableDatatype {
    action_chain_with_representation(actions, required, T::Boolean.representation())
}
fn action_chain_with_representation(
    actions: &[Action],
    required: bool,
    representation: ValueRepresentation,
) -> ExecutableDatatype {
    let (mut host, sources) = types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base} {type @name=leaf @base=derived}");
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            representation,
        ))
        .unwrap();
    for (index, action) in actions.iter().enumerate() {
        let registered = RegisteredLexicalPreparation::new(
            sources[index].clone(),
            format!("preparer-{index}"),
            PreparationSignature {
                kind: DatatypeKind::Scalar,
                output: representation,
                candidate: if index == 0 && required {
                    CandidateRequirement::Required
                } else {
                    CandidateRequirement::Optional
                },
            },
            action.clone(),
        )
        .unwrap();
        implementations
            .select_preparation(
                sources[index].clone(),
                if index == 0 {
                    PreparationBinding::Ready(registered)
                } else {
                    PreparationBinding::CheckedReplacement(registered)
                },
            )
            .unwrap();
    }
    let compilation = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(compilation.is_ready());
    compiled(&compilation, &sources[2]).clone()
}
fn action(execution: PreparationExecution) -> Action {
    Action {
        calls: Arc::new(AtomicUsize::new(0)),
        execution,
        cancel: false,
    }
}
#[test]
fn checked_preparers_keep_transitive_guards_and_required_candidates() {
    let actions = [
        action(ready(Item::Atomic(AtomValue::Boolean(true)))),
        action(ready(Item::Atomic(AtomValue::Boolean(true)))),
        action(ready(Item::Atomic(AtomValue::Boolean(true)))),
    ];
    let d = action_chain(&actions, false);
    assert_eq!(d.preparation().unwrap().invocations(), 3);
    assert_eq!(run(&d, "true", Default::default()).accepted, Some(true));
    for a in &actions {
        assert_eq!(a.calls.load(Ordering::SeqCst), 1);
    }
    let mut limits = PreparationLimits::default();
    limits.validation.max_input_values = 2;
    assert!(matches!(
        run(&d, "true", limits).stopped,
        Some(PreparationStop::Limit("input-values"))
    ));
    for a in &actions {
        assert_eq!(a.calls.load(Ordering::SeqCst), 1);
    }
    let required = action_chain(&actions, true);
    assert!(matches!(
        run(&required, "true", Default::default()).stopped,
        Some(PreparationStop::MissingCandidate)
    ));
    for a in &actions {
        assert_eq!(a.calls.load(Ordering::SeqCst), 1);
    }
}
#[test]
fn checked_preparers_stop_after_any_incomplete_or_rejected_ancestor() {
    for execution in [
        PreparationExecution::Rejected(vec![]),
        PreparationExecution::Pending(vec![]),
        PreparationExecution::Unavailable(vec![]),
        PreparationExecution::Failed(vec![]),
        PreparationExecution::Limit("fixture"),
    ] {
        let actions = [
            action(ready(Item::Atomic(AtomValue::Boolean(true)))),
            action(execution.clone()),
            action(ready(Item::Atomic(AtomValue::Boolean(true)))),
        ];
        let result = run(&action_chain(&actions, false), "true", Default::default());
        assert!(result.value.is_none());
        assert_eq!(
            result.accepted,
            if matches!(execution, PreparationExecution::Rejected(_)) {
                Some(false)
            } else {
                None
            }
        );
        assert_eq!(actions[0].calls.load(Ordering::SeqCst), 1);
        assert_eq!(actions[1].calls.load(Ordering::SeqCst), 1);
        assert_eq!(actions[2].calls.load(Ordering::SeqCst), 0);
    }
    let mut first = action(ready(Item::Atomic(AtomValue::Boolean(true))));
    first.cancel = true;
    let second = action(ready(Item::Atomic(AtomValue::Boolean(true))));
    let result = run(
        &action_chain(&[first, second.clone()], false),
        "true",
        Default::default(),
    );
    assert!(matches!(result.stopped, Some(PreparationStop::Control(_))));
    assert_eq!(second.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn checked_preparer_registration_requires_exact_source_and_available_base() {
    for (which, base_ready, expected) in [
        (0, false, "preparation-replacement-requires-base"),
        (1, false, "preparation-replacement-base-unavailable"),
        (1, true, "datatype-preparation-unavailable"),
    ] {
        let (mut host, sources) =
            types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                T::Boolean.representation(),
            ))
            .unwrap();
        if base_ready {
            implementations
                .select_preparation(sources[0].clone(), PreparationBinding::Unavailable)
                .unwrap();
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let selected = replacement(&sources[which], &calls, AtomValue::Boolean(true), false);
        assert_eq!(
            DatatypeImplementations::default()
                .select_preparation(
                    sources[1 - which].clone(),
                    PreparationBinding::CheckedReplacement(selected.clone())
                )
                .unwrap_err(),
            "unrelated-preparation-source"
        );
        implementations
            .select_preparation(
                sources[which].clone(),
                PreparationBinding::CheckedReplacement(selected),
            )
            .unwrap();
        let compilation = compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            &mut host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(!compilation.is_ready());
        assert!(
            compilation.issues.iter().any(|i| i.code == expected),
            "{:?}",
            compilation
                .issues
                .iter()
                .map(|i| i.code)
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn checked_preparer_diagnostics_use_one_cumulative_budget() {
    let diagnostic = cem_ml::diagnostics::Diagnostic {
        code: "fixture".into(),
        message: "checked".into(),
        severity: cem_ml::diagnostics::Severity::Warning,
        ..Default::default()
    };
    let first = action(PreparationExecution::Prepared {
        value: vec![Item::Atomic(AtomValue::Boolean(true))],
        diagnostics: vec![diagnostic.clone()],
    });
    let second = first.clone();
    let third = action(ready(Item::Atomic(AtomValue::Boolean(true))));
    let d = action_chain(&[first, second, third.clone()], false);
    let mut limits = PreparationLimits::default();
    limits.validation.max_diagnostics = 1;
    let report = run(&d, "true", limits);
    assert!(matches!(
        report.stopped,
        Some(PreparationStop::Limit("diagnostics"))
    ));
    assert_eq!(report.diagnostics.len(), 1);
    assert!(report.diagnostics[0].source_map.is_some());
    assert_eq!(third.calls.load(Ordering::SeqCst), 0);
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
        Some(AtomValue::Boolean(true))
    }
    fn field(&self, _: &str) -> Option<Vec<Item>> {
        None
    }
}
#[test]
fn checked_preparers_require_immutable_values_including_ancestor_outputs() {
    for impostor_base in [true, false] {
        let normal = ready(Item::Atomic(AtomValue::Boolean(true)));
        let impostor = ready(Item::native(Impostor));
        let base = action(if impostor_base {
            impostor.clone()
        } else {
            normal.clone()
        });
        let child = action(if impostor_base { normal } else { impostor });
        let report = run(
            &action_chain(&[base, child.clone()], false),
            "true",
            Default::default(),
        );
        assert!(matches!(
            report.stopped,
            Some(PreparationStop::IncompatibleReplacement)
        ));
        assert!(report.value.is_none());
        assert_eq!(
            child.calls.load(Ordering::SeqCst),
            usize::from(!impostor_base)
        );
    }
}
#[derive(Debug)]
struct NarrowTokenizer {
    calls: Arc<AtomicUsize>,
    merge: bool,
}
impl cem_ml::schema::datatype_contracts::Tokenizer for NarrowTokenizer {
    fn tokenize(
        &self,
        request: cem_ml::schema::datatype_contracts::TokenizationRequest<'_>,
    ) -> Result<Vec<std::ops::Range<usize>>, cem_ml::schema::datatype_contracts::TokenizationError>
    {
        use cem_ml::schema::datatype_contracts::{RegisteredTokenizer, TokenizationError};
        self.calls.fetch_add(1, Ordering::SeqCst);
        if request.input.text.contains('b') {
            return Err(TokenizationError::Rejected);
        }
        if self.merge {
            return Ok(vec![0..request.input.text.len()]);
        }
        Ok(RegisteredTokenizer::whitespace()
            .tokenize(
                Some(request.input.clone()),
                request.control,
                request.scope,
                request.limits,
            )?
            .tokens)
    }
}
fn list_fixture(
    merge: bool,
) -> (
    cem_ql::datatype_facets::BoundAttributeFacets,
    Arc<AtomicUsize>,
) {
    use cem_ml::schema::{
        datatype_contracts::RegisteredTokenizer, document_model::attribute_facets::FacetFamily,
    };
    use cem_ql::{attribute_datatypes::bind_attribute_datatype, datatype_facets::*};
    let profile = source("{schema @name=test @namespace=urn:test | {types | {type @name=item @kind=scalar} {type @name=items @kind=list @base=item} {type @name=derived @list-base=items}} {attributes | {attribute @name=value @type=derived @itemCount=2}}}");
    let declaration = node(&profile, "attribute");
    let (mut host, sources) = types_fixture_source(profile);
    let mut implementations = DatatypeImplementations::default();
    implementations
        .register(implementation(
            &sources[0],
            DatatypeKind::Scalar,
            T::String.representation(),
        ))
        .unwrap();
    implementations
        .select_preparation(
            sources[0].clone(),
            PreparationBinding::Ready(
                cem_ql::datatype_shipped::lexical_preparation(sources[0].clone(), T::String)
                    .unwrap(),
            ),
        )
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    for index in [1, 2] {
        let mut entry = implementation(
            &sources[index],
            DatatypeKind::List,
            ValueRepresentation::List(ScalarRepresentation::String),
        );
        entry.tokenizer = if index == 1 {
            TokenizerBinding::Ready(RegisteredTokenizer::whitespace())
        } else {
            TokenizerBinding::CheckedReplacement(
                RegisteredTokenizer::new(
                    "replacement",
                    NarrowTokenizer {
                        calls: calls.clone(),
                        merge,
                    },
                )
                .unwrap(),
            )
        };
        implementations.register(entry).unwrap();
        let preparer = RegisteredLexicalPreparation::list_items(
            sources[index].clone(),
            "list",
            ScalarRepresentation::String,
        )
        .unwrap();
        implementations
            .select_preparation(
                sources[index].clone(),
                if index == 1 {
                    PreparationBinding::Ready(preparer)
                } else {
                    PreparationBinding::CheckedReplacement(preparer)
                },
            )
            .unwrap();
    }
    implementations
        .select_facets(
            sources[1].clone(),
            FacetProfileBinding::Ready(
                RegisteredFacetProfile::new(
                    sources[1].clone(),
                    "facets",
                    FacetFamily::List(ScalarRepresentation::String),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    implementations
        .select_converter(
            sources[1].clone(),
            cem_ql::datatype_conversion::ConverterBinding::Ready(
                cem_ql::datatype_shipped::converter(sources[1].clone(), T::NameList).unwrap(),
            ),
        )
        .unwrap();
    let compilation = compile_datatypes(
        sources[0].declaration().document().clone(),
        &sources,
        &mut host,
        &implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    );
    assert!(
        compilation.is_ready(),
        "{:?}",
        compilation
            .issues
            .iter()
            .map(|i| i.code)
            .collect::<Vec<_>>()
    );
    let slot = match declaration.node() { CemAstNode::Element { attributes, .. } => attributes.iter().filter_map(|id| SchemaDeclarationNode::new(declaration.document().clone(), *id)).find(|n| matches!(n.node(), CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "type")).unwrap(), _=>unreachable!() };
    host.bind_literal_attribute_type(slot, sources[2].declaration().clone())
        .unwrap();
    let bound = bind_attribute_datatype(
        declaration,
        &compilation,
        &mut host,
        ReferenceTraversalLimits::schema_defaults().unwrap(),
    )
    .unwrap()
    .bound
    .unwrap()
    .compile_facets("urn:test", Default::default())
    .unwrap();
    (bound, calls)
}
#[test]
fn checked_list_replacement_retains_spans_and_sealed_consumption_spends_guard_budget_once() {
    use cem_ml::schema::document_model::attribute_facets::FacetContext;
    use cem_ql::preparation_evidence::AttributePreparationInvocation;
    let (bound, calls) = list_fixture(false);
    let d = &bound.binding().datatype;
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let mut limits = PreparationLimits::default();
    limits.max_preparations = 5; // two list bindings, two tokenizer checks, two items (one shared orchestration unit).
    let invocation =
        AttributePreparationInvocation::new(&bound, request(" α α "), &runtime, limits).unwrap();
    let prepared = invocation.prepare();
    assert_eq!(prepared.report.preparations, 5);
    let evidence = prepared.evidence.unwrap();
    assert_eq!(evidence.token_spans(), &[1..3, 4..6]);
    assert_eq!(invocation.remaining_limits().max_preparations, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for _ in 0..2 {
        let report = bound.validate_pretyped(
            &invocation,
            Some(&evidence),
            FacetContext {
                element_name: "sample",
                source: bound.binding().declaration.node(),
                diagnostic_behaviors: &Default::default(),
                attribute_values: &Default::default(),
            },
            1_048_576,
        );
        assert_eq!(report.accepted, Some(true));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    assert!(invocation.prepare().evidence.is_none());
    assert_eq!(run(d, "a b", Default::default()).accepted, Some(false));
    let convert = d.convert(
        &cem_ql::datatype_conversion::ConversionInput {
            value: cem_ql::datatype_conversion::ConversionValue::Lexical(request("a b").lexical),
            candidate: vec![],
            fallback: Default::default(),
        },
        &runtime,
        Default::default(),
    );
    assert_eq!(convert.accepted, Some(false));
    let (bad, _) = list_fixture(true);
    let report = run(&bad.binding().datatype, "a a", Default::default());
    assert!(matches!(
        report.stopped,
        Some(PreparationStop::IncompatibleReplacement)
    ));
    assert!(report.value.is_none());
}

#[test]
fn checked_tokenizer_selection_requires_a_whole_list_with_an_available_tokenizer() {
    use cem_ml::schema::datatype_contracts::RegisteredTokenizer;
    for (index, base, expected) in [
        (
            1,
            TokenizerBinding::Absent,
            "tokenizer-replacement-requires-base",
        ),
        (
            2,
            TokenizerBinding::Absent,
            "tokenizer-replacement-base-unavailable",
        ),
        (
            2,
            TokenizerBinding::Unavailable,
            "datatype-tokenizer-unavailable",
        ),
    ] {
        let (mut host, sources) = types_fixture("{type @name=item @kind=scalar} {type @name=base @kind=list @base=item} {type @name=derived @list-base=base}");
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                T::String.representation(),
            ))
            .unwrap();
        for i in [1, 2] {
            let mut entry = implementation(
                &sources[i],
                DatatypeKind::List,
                ValueRepresentation::List(ScalarRepresentation::String),
            );
            entry.tokenizer = if i == index {
                TokenizerBinding::CheckedReplacement(RegisteredTokenizer::whitespace())
            } else {
                base.clone()
            };
            implementations.register(entry).unwrap();
        }
        let compilation = compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            &mut host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(!compilation.is_ready());
        assert!(compilation.issues.iter().any(|i| i.code == expected));
    }
}

#[test]
fn checked_preparation_uses_exact_representations_and_bounds_comparison_storage() {
    for (representation, base, child, accepted) in [
        (
            ScalarRepresentation::Decimal,
            AtomValue::Decimal("1.0".into()),
            AtomValue::Decimal("1.00".into()),
            None,
        ),
        (
            ScalarRepresentation::Double,
            AtomValue::Double(-0.0),
            AtomValue::Double(0.0),
            None,
        ),
        (
            ScalarRepresentation::Double,
            AtomValue::Double(f64::NAN),
            AtomValue::Double(f64::NAN),
            Some(true),
        ),
    ] {
        let d = action_chain_with_representation(
            &[
                action(ready(Item::Atomic(base))),
                action(ready(Item::Atomic(child))),
            ],
            false,
            ValueRepresentation::Scalar(representation),
        );
        assert_eq!(run(&d, "true", Default::default()).accepted, accepted);
    }
    let child = action(ready(Item::Atomic(AtomValue::String("too long".into()))));
    let d = action_chain_with_representation(
        &[child.clone(), child.clone()],
        false,
        T::String.representation(),
    );
    let mut limits = PreparationLimits::default();
    limits.max_lexical_bytes = 4;
    let report = run(&d, "true", limits);
    assert!(matches!(
        report.stopped,
        Some(PreparationStop::Limit("replacement-value-bytes"))
    ));
    assert_eq!(child.calls.load(Ordering::SeqCst), 1);
    for (ty, text) in [
        (T::Integer, "922337203685477580812345"),
        (T::Number, "1.2300"),
    ] {
        let (mut host, sources) =
            types_fixture("{type @name=base @kind=scalar} {type @name=derived @base=base}");
        let mut implementations = DatatypeImplementations::default();
        implementations
            .register(implementation(
                &sources[0],
                DatatypeKind::Scalar,
                ty.representation(),
            ))
            .unwrap();
        for i in [0, 1] {
            let selected =
                cem_ql::datatype_shipped::lexical_preparation(sources[i].clone(), ty).unwrap();
            implementations
                .select_preparation(
                    sources[i].clone(),
                    if i == 0 {
                        PreparationBinding::Ready(selected)
                    } else {
                        PreparationBinding::CheckedReplacement(selected)
                    },
                )
                .unwrap();
        }
        let compilation = compile_datatypes(
            sources[0].declaration().document().clone(),
            &sources,
            &mut host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(compilation.is_ready());
        let report = run(
            compiled(&compilation, &sources[1]),
            text,
            Default::default(),
        );
        assert_eq!(report.accepted, Some(true));
        assert_eq!(report.input.lexical.text.as_ref(), text);
        assert_eq!(report.preparations, 2);
    }
}
