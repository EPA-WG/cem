use super::*;
use cem_ml::schema::{
    datatype_contracts::{CompiledDatatypeContract, DatatypeCompilation, LexicalInput},
    document_model::shipped_datatypes::ShippedDatatype,
    reference_traversal::ReferenceTraversalLimits,
};
use cem_ql::{
    datatype_compilation::{
        compile_datatypes_with_runtime, DatatypeImplementation, DatatypeImplementations,
        ExecutableDatatype, TokenizerBinding,
    },
    datatype_conversion::{
        ConversionCall, ConversionExecution, ConversionInput, ConversionRepresentation,
        ConversionSignature, ConversionStop, ConversionValue, NativeDatatypeConverter,
    },
    datatype_enumeration::{
        ConstantBinding, EqualityCall, EqualityExecution, NativeScalarEquality,
    },
    datatype_preparation::{
        NativeLexicalPreparer, PreparationCall, PreparationExecution, PreparationInput,
        PreparationSignature, PreparationStop,
    },
    datatype_serialization::{
        ListSerializationCall, ListSerializationSignature, NativeListSerializer,
        SerializationExecution, SerializationStop,
    },
    datatype_validation::ValidationRuntime,
    native_registrations::{OperationMode, PreparationMode},
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Clone, Debug)]
struct Engine {
    owner: Arc<()>,
    calls: Arc<[AtomicUsize; 4]>,
    changed: bool,
    cancel: Option<OperationControl>,
    armed: Arc<AtomicBool>,
    invalid_output: bool,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            owner: Arc::new(()),
            calls: Arc::new(std::array::from_fn(|_| AtomicUsize::new(0))),
            changed: false,
            cancel: None,
            armed: Arc::new(AtomicBool::new(false)),
            invalid_output: false,
        }
    }
}
impl Engine {
    fn enter(&self, role: usize, datatype: &Item) {
        self.calls[role].fetch_add(1, Ordering::AcqRel);
        assert!(cem_ql::eval::retained_cem_node(datatype).is_some());
        if self.armed.load(Ordering::Acquire) {
            if let Some(control) = &self.cancel {
                control.cancel_root(None, None).unwrap();
            }
        }
    }
}
impl NativeLexicalPreparer for Engine {
    fn prepare(&self, call: PreparationCall<'_>) -> PreparationExecution {
        self.enter(0, call.datatype);
        if self.invalid_output {
            return PreparationExecution::Prepared {
                value: vec![Item::Atomic(AtomValue::Boolean(true))],
                diagnostics: vec![],
            };
        }
        match call.lexical.text.as_ref() {
            "reject" => PreparationExecution::Rejected(vec![]),
            "pending" => PreparationExecution::Pending(vec![]),
            "unavailable" => PreparationExecution::Unavailable(vec![]),
            "failed" => PreparationExecution::Failed(vec![]),
            _ => PreparationExecution::Prepared {
                value: vec![string(if self.changed {
                    "changed"
                } else {
                    &call.lexical.text
                })],
                diagnostics: vec![],
            },
        }
    }
}
impl NativeDatatypeConverter for Engine {
    fn convert(&self, call: ConversionCall<'_>) -> ConversionExecution {
        self.enter(1, call.datatype);
        if self.invalid_output {
            return ConversionExecution::Converted {
                value: vec![Item::Atomic(AtomValue::Boolean(true))],
                diagnostics: vec![],
            };
        }
        let ConversionValue::Lexical(input) = call.value else {
            panic!("explicit lexical converter")
        };
        match input.text.as_ref() {
            "reject" => ConversionExecution::Rejected {
                diagnostics: vec![],
            },
            "pending" => ConversionExecution::Pending(vec![]),
            "unavailable" => ConversionExecution::Unavailable(vec![]),
            "failed" => ConversionExecution::Failed(vec![]),
            _ => ConversionExecution::Converted {
                value: vec![string(&format!("converted:{}", input.text))],
                diagnostics: vec![],
            },
        }
    }
}
impl NativeScalarEquality for Engine {
    fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution {
        self.enter(2, call.datatype);
        EqualityExecution::Complete {
            equal: call.left == call.right,
            diagnostics: vec![],
        }
    }
}
impl NativeListSerializer for Engine {
    fn serialize(&self, call: ListSerializationCall<'_>) -> SerializationExecution {
        self.enter(3, call.datatype);
        let strings = call
            .value
            .iter()
            .map(|v| match v.atom() {
                Some(AtomValue::String(s)) => s,
                _ => panic!("typed string"),
            })
            .collect::<Vec<_>>();
        SerializationExecution::Serialized {
            text: strings.join("|"),
            diagnostics: vec![],
        }
    }
}
fn string(s: &str) -> Item {
    Item::Atomic(AtomValue::String(s.into()))
}
fn lexical(s: &str) -> LexicalInput {
    LexicalInput::new(Arc::from(s), Default::default())
}
fn input(s: &str) -> ValidationInput {
    ValidationInput {
        value: vec![string(s)],
        candidate: vec![],
        fallback: Default::default(),
    }
}
fn text() -> String {
    TEXT.replace("{type @name=derived @list-base=names}", "{type @name=derived @list-base=names} {type @name=leaf @base=sample} {type @name=choice @base=sample @values=\"red blue\"}")
}
fn shared_requirement(
    mode: OperationMode,
    name: &str,
    id: ImplementationIdentity,
    root: Option<&native::VerifiedNativeRequirement>,
) -> Arc<native::VerifiedNativeRequirement> {
    let text = text();
    let c = capture(&text);
    let built = native::export_operation(
        &c,
        &datatype(&c, name),
        mode,
        SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), text.as_bytes()),
        id,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let verified = native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    Arc::new(if let Some(root) = root {
        verified
            .share_source(root, &OperationControl::default())
            .unwrap()
    } else {
        verified
    })
}
fn register(
    host: &mut FreshStructuralHost,
    id: ImplementationIdentity,
    engine: Engine,
) -> Result<(), native::NativeLinkError> {
    match id.signature {
        NativeSignature::Preparation(_) => host.register_native_preparer(id, engine),
        NativeSignature::Conversion(_) => host.register_native_converter(id, engine),
        NativeSignature::Equality(_) => host.register_native_equality(id, engine),
        NativeSignature::ListSerializer(_) => host.register_native_list_serializer(id, engine),
        _ => unreachable!(),
    }
}
fn implementations(root: &native::VerifiedNativeRequirement) -> DatatypeImplementations {
    let mut implementations = DatatypeImplementations::default();
    for (name, kind) in [
        ("sample", DatatypeKind::Scalar),
        ("leaf", DatatypeKind::Scalar),
        ("choice", DatatypeKind::Scalar),
        ("names", DatatypeKind::List),
    ] {
        implementations
            .register(DatatypeImplementation {
                source: datatype(root.capture(), name),
                kind,
                representation: if kind == DatatypeKind::List {
                    ValueRepresentation::List(ScalarRepresentation::String)
                } else {
                    ValueRepresentation::Scalar(ScalarRepresentation::String)
                },
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            })
            .unwrap();
    }
    implementations
}
fn compile(
    root: &native::VerifiedNativeRequirement,
    names: &[&str],
    implementations: &DatatypeImplementations,
) -> DatatypeCompilation {
    let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
    host.register_scope(
        root.source().clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    for name in ["sample", "leaf", "choice", "names"] {
        let source = datatype(root.capture(), name);
        host.register_datatype_source(source.clone()).unwrap();
        host.bind_literal_datatype(source.scope(), name, source.declaration().clone())
            .unwrap();
    }
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    compile_datatypes_with_runtime(
        root.source().ast_owner().clone(),
        &names
            .iter()
            .map(|n| datatype(root.capture(), n))
            .collect::<Vec<_>>(),
        &mut host,
        implementations,
        &Default::default(),
        ReferenceTraversalLimits::schema_defaults().unwrap(),
        &runtime,
        Default::default(),
    )
}
fn compiled(c: &DatatypeCompilation, name: &str) -> ExecutableDatatype {
    assert!(c.is_ready(), "{:?}", c.issues);
    c.contracts.iter().filter_map(|c| c.as_any().downcast_ref::<ExecutableDatatype>()).find(|c| matches!(c.source().attribute("name").map(|a| a.node()), Some(CemAstNode::Attribute{value:Some(v),..}) if v==name)).unwrap().clone()
}

#[test]
fn compiled_native_operations_preserve_explicit_boundaries_and_completion_states() {
    let descriptors = descriptors();
    let root = shared_requirement(descriptors[0].0, "sample", descriptors[0].2.clone(), None);
    let mut host = host();
    let engine = Engine::default();
    for (_, _, id) in &descriptors {
        register(&mut host, id.clone(), engine.clone()).unwrap();
    }
    let scope = host
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let mut implementations = implementations(&root);
    let mut bindings = vec![];
    for (mode, name, id) in descriptors {
        let req = shared_requirement(mode, name, id, Some(&root));
        let linked = host.link_native_operation(req, scope).unwrap();
        let source = datatype(root.capture(), name);
        assert_eq!(
            linked.install(&datatype(&capture(TEXT), name), &mut implementations),
            Err(native::NativeLinkError::InvalidSource)
        );
        linked.install(&source, &mut implementations).unwrap();
        assert_eq!(
            linked.install(&source, &mut implementations),
            Err(native::NativeLinkError::Duplicate)
        );
        bindings.push(linked);
    }
    let compilation = compile(&root, &["sample", "names"], &implementations);
    let sample = compiled(&compilation, "sample");
    let names = compiled(&compilation, "names");
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    assert_eq!(
        sample
            .validate(&input("red"), &runtime, Default::default())
            .accepted,
        Some(true)
    );
    assert!(engine.calls.iter().all(|n| n.load(Ordering::Acquire) == 0));
    let prep = |text| PreparationInput {
        lexical: lexical(text),
        candidate: vec![],
        fallback: Default::default(),
    };
    let result = sample.prepare_lexical(&prep("red"), &runtime, Default::default());
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.value, Some(vec![string("red")]));
    let convert = |text| ConversionInput {
        value: ConversionValue::Lexical(lexical(text)),
        candidate: vec![],
        fallback: Default::default(),
    };
    let result = sample.convert(&convert("red"), &runtime, Default::default());
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.value, Some(vec![string("converted:red")]));
    for (word, accepted) in [
        ("reject", Some(false)),
        ("pending", None),
        ("unavailable", None),
        ("failed", None),
    ] {
        let p = sample.prepare_lexical(&prep(word), &runtime, Default::default());
        let c = sample.convert(&convert(word), &runtime, Default::default());
        assert_eq!(p.accepted, accepted);
        assert_eq!(c.accepted, accepted);
        match word {
            "pending" => {
                assert!(matches!(p.stopped, Some(PreparationStop::Pending)));
                assert!(matches!(c.stopped, Some(ConversionStop::Pending)));
            }
            "unavailable" => {
                assert!(matches!(p.stopped, Some(PreparationStop::Unavailable)));
                assert!(matches!(c.stopped, Some(ConversionStop::Unavailable)));
            }
            "failed" => {
                assert!(matches!(p.stopped, Some(PreparationStop::Failed)));
                assert!(matches!(c.stopped, Some(ConversionStop::Failed)));
            }
            _ => (),
        }
    }
    let mut list = input("red");
    list.value.extend([string("red"), string("blue")]);
    let result = names.serialize_list(&list, &runtime, Default::default());
    assert_eq!(result.accepted, Some(true));
    assert_eq!(result.text.as_deref(), Some("red|red|blue"));
    let result = names.serialize_list(
        &list,
        &runtime,
        cem_ql::datatype_serialization::SerializationLimits {
            max_output_bytes: 1,
            ..Default::default()
        },
    );
    assert!(matches!(
        result.stopped,
        Some(SerializationStop::Limit("output-bytes"))
    ));
    assert!(result.text.is_none());
    host.close();
    assert!(matches!(
        sample
            .prepare_lexical(&prep("red"), &runtime, Default::default())
            .stopped,
        Some(PreparationStop::Unavailable)
    ));
    assert!(matches!(
        sample
            .convert(&convert("red"), &runtime, Default::default())
            .stopped,
        Some(ConversionStop::Unavailable)
    ));
    let result = names.serialize_list(&list, &runtime, Default::default());
    assert!(matches!(
        result.stopped,
        Some(SerializationStop::Unavailable)
    ));
    assert!(result.text.is_none());
}

#[test]
fn native_equality_keeps_constant_ownership_and_becomes_unavailable_after_retirement() {
    let (mode, _, id) = descriptors().remove(2);
    let root = shared_requirement(mode, "sample", id.clone(), None);
    let mut host = host();
    let engine = Engine::default();
    register(&mut host, id, engine.clone()).unwrap();
    let scope = host
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let binding = host.link_native_operation(root.clone(), scope).unwrap();
    let mut implementations = implementations(&root);
    let source = datatype(root.capture(), "sample");
    binding.install(&source, &mut implementations).unwrap();
    implementations
        .select_constant_interpreter(
            source.clone(),
            ConstantBinding::Ready(
                cem_ql::datatype_shipped::constant_interpreter(source, ShippedDatatype::String)
                    .unwrap(),
            ),
        )
        .unwrap();
    let compilation = compile(&root, &["choice"], &implementations);
    let choice = compiled(&compilation, "choice");
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    assert_eq!(choice.enumerations()[0].constants().len(), 2);
    assert!(choice.enumerations()[0]
        .constants()
        .iter()
        .all(|constant| Arc::ptr_eq(constant.token.source.document(), root.source().ast_owner())));
    for (word, accepted) in [("red", true), ("blue", true), ("green", false)] {
        assert_eq!(
            choice
                .validate(&input(word), &runtime, Default::default())
                .accepted,
            Some(accepted)
        );
    }
    assert!(engine.calls[2].load(Ordering::Acquire) > 0);
    binding.retire();
    let before = engine.calls[2].load(Ordering::Acquire);
    assert_eq!(
        choice
            .validate(&input("red"), &runtime, Default::default())
            .accepted,
        None
    );
    assert_eq!(engine.calls[2].load(Ordering::Acquire), before);
}

#[test]
fn checked_native_preparation_replacement_preserves_base_admission_and_budget() {
    for changed in [false, true] {
        let (mode, _, base_id) = descriptors().remove(0);
        let root = shared_requirement(mode, "sample", base_id.clone(), None);
        let mut child_id = base_id.clone();
        child_id.id = "prepare-leaf".into();
        let req = shared_requirement(
            OperationMode::Preparation(PreparationMode::CheckedReplacement),
            "leaf",
            child_id.clone(),
            Some(&root),
        );
        let mut host = host();
        let engine = Engine::default();
        register(&mut host, base_id, engine.clone()).unwrap();
        register(
            &mut host,
            child_id.clone(),
            Engine {
                changed,
                ..engine.clone()
            },
        )
        .unwrap();
        let scope = host
            .register_scope(
                root.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let base = host.link_native_operation(root.clone(), scope).unwrap();
        let child = host.link_native_operation(req, scope).unwrap();
        let mut implementations = implementations(&root);
        base.install(&datatype(root.capture(), "sample"), &mut implementations)
            .unwrap();
        child
            .install(&datatype(root.capture(), "leaf"), &mut implementations)
            .unwrap();
        let compilation = compile(&root, &["leaf"], &implementations);
        let leaf = compiled(&compilation, "leaf");
        assert_eq!(leaf.preparation().unwrap().invocations(), 2);
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let prep = |s| PreparationInput {
            lexical: lexical(s),
            candidate: vec![],
            fallback: Default::default(),
        };
        let result = leaf.prepare_lexical(&prep("red"), &runtime, Default::default());
        if changed {
            assert!(matches!(
                result.stopped,
                Some(PreparationStop::IncompatibleReplacement)
            ));
            assert_eq!(result.accepted, None);
        } else {
            assert_eq!(result.accepted, Some(true));
        }
        let result = leaf.prepare_lexical(&prep("reject"), &runtime, Default::default());
        assert_eq!(result.accepted, Some(false));
        let before = engine.calls[0].load(Ordering::Acquire);
        let result = leaf.prepare_lexical(
            &prep("red"),
            &runtime,
            cem_ql::datatype_preparation::PreparationLimits {
                max_preparations: 1,
                ..Default::default()
            },
        );
        assert!(matches!(
            result.stopped,
            Some(PreparationStop::Limit("preparations"))
        ));
        assert_eq!(engine.calls[0].load(Ordering::Acquire), before);
        // A Ready replacement must still fail in the ordinary compiler.
        let mut invalid = self::implementations(&root);
        base.install(&datatype(root.capture(), "sample"), &mut invalid)
            .unwrap();
        let source = datatype(root.capture(), "leaf");
        let NativeSignature::Preparation(signature) = child_id.signature else {
            unreachable!()
        };
        invalid
            .select_preparation(
                source.clone(),
                cem_ql::datatype_preparation::PreparationBinding::Ready(
                    cem_ql::datatype_preparation::RegisteredLexicalPreparation::new(
                        source,
                        child_id.id,
                        signature,
                        Engine::default(),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        let invalid = compile(&root, &["leaf"], &invalid);
        assert!(!invalid.is_ready());
        assert!(invalid
            .issues
            .iter()
            .any(|i| i.code == "preparation-base-replacement-unsupported"));
    }
}

fn descriptors() -> Vec<(OperationMode, &'static str, ImplementationIdentity)> {
    vec![
        (
            OperationMode::Preparation(PreparationMode::Ready),
            "sample",
            ImplementationIdentity::preparation(
                "prepare",
                hash(b"native"),
                "prepare",
                PreparationSignature {
                    kind: DatatypeKind::Scalar,
                    output: ValueRepresentation::Scalar(ScalarRepresentation::String),
                    candidate: CandidateRequirement::Optional,
                },
            ),
        ),
        (
            OperationMode::Conversion,
            "sample",
            ImplementationIdentity::conversion(
                "convert",
                hash(b"native"),
                "convert",
                ConversionSignature {
                    kind: DatatypeKind::Scalar,
                    input: ConversionRepresentation::Lexical,
                    output: ValueRepresentation::Scalar(ScalarRepresentation::String),
                    candidate: CandidateRequirement::Optional,
                },
            ),
        ),
        (
            OperationMode::Equality,
            "sample",
            ImplementationIdentity::equality(
                "equal",
                hash(b"native"),
                "equal",
                ScalarRepresentation::String,
            ),
        ),
        (
            OperationMode::ListSerializer,
            "names",
            ImplementationIdentity::list_serializer(
                "serialize",
                hash(b"native"),
                "serialize",
                ListSerializationSignature {
                    item: ScalarRepresentation::String,
                    candidate: CandidateRequirement::Optional,
                },
            ),
        ),
    ]
}
fn requirement(
    mode: OperationMode,
    name: &str,
    implementation: ImplementationIdentity,
) -> Arc<native::VerifiedNativeRequirement> {
    let c = capture(TEXT);
    let built = native::export_operation(
        &c,
        &datatype(&c, name),
        mode,
        identity(),
        implementation.clone(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let verified = native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    assert_eq!(verified.implementation(), &implementation);
    Arc::new(verified)
}
#[test]
fn exact_operation_requirements_roundtrip_and_refuse_mode_kind_and_abi_changes() {
    for (mode, name, implementation) in descriptors() {
        let _ = requirement(mode, name, implementation.clone());
        let c = capture(TEXT);
        let mut bad = implementation.clone();
        bad.abi.push_str("-unknown");
        assert!(native::export_operation(
            &c,
            &datatype(&c, name),
            mode,
            identity(),
            bad,
            &hash(b"registry"),
            &OperationControl::default()
        )
        .is_err());
        let wrong = if mode == OperationMode::Equality {
            OperationMode::Conversion
        } else {
            OperationMode::Equality
        };
        assert!(native::export_operation(
            &c,
            &datatype(&c, name),
            wrong,
            identity(),
            implementation.clone(),
            &hash(b"registry"),
            &OperationControl::default()
        )
        .is_err());
        let foreign = capture(TEXT);
        assert!(native::export_operation(
            &foreign,
            &datatype(&c, name),
            mode,
            identity(),
            implementation,
            &hash(b"registry"),
            &OperationControl::default()
        )
        .is_err());
    }
}

fn assert_unavailable(role: usize, descriptor: &ExecutableDatatype) {
    let control = OperationControl::default();
    let runtime = ValidationRuntime {
        control: &control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    match role {
        0 => {
            let result = descriptor.prepare_lexical(
                &PreparationInput {
                    lexical: lexical("red"),
                    candidate: vec![],
                    fallback: Default::default(),
                },
                &runtime,
                Default::default(),
            );
            assert!(
                matches!(result.stopped, Some(PreparationStop::Unavailable)),
                "{result:?}"
            );
            assert_eq!(result.accepted, None);
            assert!(result.value.is_none());
        }
        1 => {
            let result = descriptor.convert(
                &ConversionInput {
                    value: ConversionValue::Lexical(lexical("red")),
                    candidate: vec![],
                    fallback: Default::default(),
                },
                &runtime,
                Default::default(),
            );
            assert!(
                matches!(result.stopped, Some(ConversionStop::Unavailable)),
                "{result:?}"
            );
            assert_eq!(result.accepted, None);
            assert!(result.value.is_none());
        }
        2 => {
            let result = descriptor.validate(&input("red"), &runtime, Default::default());
            assert!(matches!(
                result.stopped.map(|s| s.reason),
                Some(cem_ql::datatype_validation::ValidationStopReason::Unavailable(_))
            ));
            assert_eq!(result.accepted, None);
        }
        _ => {
            let result = descriptor.serialize_list(&input("red"), &runtime, Default::default());
            assert!(
                matches!(result.stopped, Some(SerializationStop::Unavailable)),
                "{result:?}"
            );
            assert_eq!(result.accepted, None);
            assert!(result.text.is_none());
        }
    }
}
#[test]
fn all_operation_proxies_expire_without_retaining_callbacks_or_publishing_late_results() {
    for (role, (mode, name, id)) in descriptors().into_iter().enumerate() {
        for case in 0..7 {
            let root = shared_requirement(mode, name, id.clone(), None);
            let control = OperationControl::default();
            let mut host = FreshStructuralHost::new(&hash(b"registry"), control.clone()).unwrap();
            let engine = Engine {
                cancel: Some(control.clone()),
                ..Engine::default()
            };
            let weak = Arc::downgrade(&engine.owner);
            let calls = engine.calls.clone();
            let armed = engine.armed.clone();
            register(&mut host, id.clone(), engine).unwrap();
            let scope = host
                .register_scope(
                    root.source().clone(),
                    Some(Default::default()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            let mut binding = Some(host.link_native_operation(root.clone(), scope).unwrap());
            let mut implementations = implementations(&root);
            let source = datatype(root.capture(), name);
            binding
                .as_ref()
                .unwrap()
                .install(&source, &mut implementations)
                .unwrap();
            if role == 2 {
                implementations
                    .select_constant_interpreter(
                        source.clone(),
                        ConstantBinding::Ready(
                            cem_ql::datatype_shipped::constant_interpreter(
                                source.clone(),
                                ShippedDatatype::String,
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap();
            }
            let target = if role == 2 { "choice" } else { name };
            let compilation = compile(&root, &[target], &implementations);
            let descriptor = compiled(&compilation, target);
            let before = calls[role].load(Ordering::Acquire);
            match case {
                0 => {
                    host.remove_native_implementation(&id.id).unwrap();
                    assert!(weak.upgrade().is_none());
                }
                1 => {
                    control.cancel_root(None, None).unwrap();
                }
                2 => {
                    drop(binding.take());
                }
                3 => {
                    host.close();
                }
                4 => {
                    host.register_native_validator(validator_id(), Validator)
                        .unwrap();
                }
                5 => {
                    assert!(host.set_context(scope, Some(Default::default())));
                }
                _ => armed.store(true, Ordering::Release),
            }
            assert_unavailable(role, &descriptor);
            assert_eq!(
                calls[role].load(Ordering::Acquire) - before,
                usize::from(case == 6),
                "role {role}, case {case}"
            );
            if let Some(binding) = &binding {
                assert_eq!(
                    binding.install(&source, &mut DatatypeImplementations::default()),
                    Err(native::NativeLinkError::Expired)
                );
            }
            drop(host);
            assert!(
                weak.upgrade().is_none(),
                "role {role}, case {case}: a compiled descriptor retained native code"
            );
        }
    }
}

#[test]
fn operation_linking_requires_exact_actual_metadata_and_registered_source_allocation() {
    for (mode, name, id) in descriptors() {
        let root = shared_requirement(mode, name, id.clone(), None);
        for change in 0..4 {
            let mut host = host();
            let mut actual = id.clone();
            match change {
                0 => (),
                1 => actual.module_hash = hash(b"other-native-module"),
                2 => actual.export.push_str("_other"),
                _ => match &mut actual.signature {
                    NativeSignature::Preparation(sig) => {
                        sig.candidate = CandidateRequirement::Required
                    }
                    NativeSignature::Conversion(sig) => {
                        sig.candidate = CandidateRequirement::Required
                    }
                    NativeSignature::Equality(rep) => *rep = ScalarRepresentation::Boolean,
                    NativeSignature::ListSerializer(sig) => {
                        sig.candidate = CandidateRequirement::Required
                    }
                    _ => unreachable!(),
                },
            }
            if change > 0 {
                register(&mut host, actual, Engine::default()).unwrap();
            }
            let scope = host
                .register_scope(
                    root.source().clone(),
                    Some(Default::default()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            assert!(
                matches!(
                    host.link_native_operation(root.clone(), scope),
                    Err(native::NativeLinkError::Unavailable)
                ) == (change == 0)
            );
            if change > 0 {
                assert!(matches!(
                    host.link_native_operation(root.clone(), scope),
                    Err(native::NativeLinkError::IdentityMismatch)
                ));
            }
        }
        let mut host = host();
        register(&mut host, id.clone(), Engine::default()).unwrap();
        let scope = host
            .register_scope(
                root.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let foreign = shared_requirement(mode, name, id.clone(), None);
        assert!(matches!(
            host.link_native_operation(foreign, scope),
            Err(native::NativeLinkError::InvalidSource)
        ));
        let binding = host.link_native_operation(root.clone(), scope).unwrap();
        assert_eq!(
            register(&mut host, id, Engine::default()),
            Err(native::NativeLinkError::Duplicate)
        );
        binding
            .install(
                &datatype(root.capture(), name),
                &mut DatatypeImplementations::default(),
            )
            .unwrap();
    }
}

#[test]
fn repinned_operation_records_refuse_invalid_modes_sources_signatures_and_versions() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Packet {
        abi: String,
        ml_version: String,
        ql_version: String,
        registry_hash: String,
        identity: SourceIdentity,
        implementation: ImplementationIdentity,
        selection: Selection,
        source: cem_ml::schema::native_source::NativeSourceSnapshot,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    enum Selection {
        Operation {
            scope: u32,
            declaration: u32,
            mode: OperationMode,
        },
    }
    for (mode, name, id) in descriptors() {
        let c = capture(TEXT);
        let built = native::export_operation(
            &c,
            &datatype(&c, name),
            mode,
            identity(),
            id,
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap();
        for change in 0..8 {
            let mut packet: Packet =
                rmp_serde::from_slice(&built.bytes[native::MAGIC.len()..]).unwrap();
            let Selection::Operation {
                scope,
                declaration,
                mode,
            } = &mut packet.selection;
            match change {
                0 => packet.abi.push_str("-future"),
                1 => packet.implementation.abi.push_str("-future"),
                2 => {
                    *mode = if *mode == OperationMode::Equality {
                        OperationMode::Conversion
                    } else {
                        OperationMode::Equality
                    };
                }
                3 => *declaration = u32::MAX,
                4 => *scope = *declaration,
                5 => packet.identity.uri.push_str("other"),
                6 => packet.implementation.id = " ".into(),
                _ => packet.implementation.signature = NativeSignature::TokenizerSpans,
            }
            let bytes = [native::MAGIC, &rmp_serde::to_vec_named(&packet).unwrap()].concat();
            let mut pin = built.pin.clone();
            pin.hash = hash(&bytes);
            assert!(
                native::verify(&bytes, &pin, &OperationControl::default()).is_err(),
                "change {change}"
            );
        }
    }
}

#[test]
fn native_operation_candidates_and_output_shapes_use_shared_admission_checks() {
    for (role, (mode, name, mut id)) in descriptors().into_iter().enumerate() {
        if role == 2 {
            continue;
        }
        match &mut id.signature {
            NativeSignature::Preparation(sig) => sig.candidate = CandidateRequirement::Required,
            NativeSignature::Conversion(sig) => sig.candidate = CandidateRequirement::Required,
            NativeSignature::ListSerializer(sig) => sig.candidate = CandidateRequirement::Required,
            _ => unreachable!(),
        }
        let root = shared_requirement(mode, name, id.clone(), None);
        let mut host = host();
        let engine = Engine {
            invalid_output: true,
            ..Engine::default()
        };
        register(&mut host, id, engine.clone()).unwrap();
        let scope = host
            .register_scope(
                root.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let binding = host.link_native_operation(root.clone(), scope).unwrap();
        let mut implementations = implementations(&root);
        binding
            .install(&datatype(root.capture(), name), &mut implementations)
            .unwrap();
        let descriptor = compiled(&compile(&root, &[name], &implementations), name);
        let control = OperationControl::default();
        let runtime = ValidationRuntime {
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        for with_candidate in [false, true] {
            let candidate = if with_candidate {
                vec![cem_ql::eval::RetainedCemNode::new(
                    root.source().clone(),
                    datatype(root.capture(), name).declaration().node_id(),
                )
                .unwrap()
                .query_item()]
            } else {
                vec![]
            };
            match role {
                0 => {
                    let result = descriptor.prepare_lexical(
                        &PreparationInput {
                            lexical: lexical("red"),
                            candidate,
                            fallback: Default::default(),
                        },
                        &runtime,
                        Default::default(),
                    );
                    assert!(
                        matches!(
                            (&result.stopped, with_candidate),
                            (Some(PreparationStop::MissingCandidate), false)
                                | (Some(PreparationStop::InvalidOutput), true)
                        ),
                        "{result:?}"
                    );
                    assert_eq!(result.accepted, None);
                }
                1 => {
                    let result = descriptor.convert(
                        &ConversionInput {
                            value: ConversionValue::Lexical(lexical("red")),
                            candidate,
                            fallback: Default::default(),
                        },
                        &runtime,
                        Default::default(),
                    );
                    assert!(
                        matches!(
                            (&result.stopped, with_candidate),
                            (Some(ConversionStop::MissingCandidate), false)
                                | (Some(ConversionStop::InvalidOutput(_)), true)
                        ),
                        "{result:?}"
                    );
                    assert_eq!(result.accepted, None);
                }
                _ => {
                    let mut input = input("red");
                    input.candidate = candidate;
                    let result = descriptor.serialize_list(&input, &runtime, Default::default());
                    if with_candidate {
                        assert_eq!(result.accepted, Some(true));
                    } else {
                        assert!(matches!(
                            result.stopped,
                            Some(SerializationStop::MissingCandidate)
                        ));
                    }
                }
            }
            assert_eq!(
                engine.calls[role].load(Ordering::Acquire),
                usize::from(with_candidate)
            );
        }
    }
}

#[test]
fn native_requirement_signatures_do_not_override_compiler_type_representations() {
    for (mode, name, mut id) in descriptors() {
        match &mut id.signature {
            NativeSignature::Preparation(sig) => {
                sig.output = ValueRepresentation::Scalar(ScalarRepresentation::Boolean)
            }
            NativeSignature::Conversion(sig) => {
                sig.output = ValueRepresentation::Scalar(ScalarRepresentation::Boolean)
            }
            NativeSignature::Equality(rep) => *rep = ScalarRepresentation::Boolean,
            NativeSignature::ListSerializer(sig) => sig.item = ScalarRepresentation::Boolean,
            _ => unreachable!(),
        }
        let root = shared_requirement(mode, name, id.clone(), None);
        let mut host = host();
        register(&mut host, id, Engine::default()).unwrap();
        let scope = host
            .register_scope(
                root.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let binding = host.link_native_operation(root.clone(), scope).unwrap();
        let mut implementations = implementations(&root);
        binding
            .install(&datatype(root.capture(), name), &mut implementations)
            .unwrap();
        let compilation = compile(&root, &[name], &implementations);
        assert!(!compilation.is_ready());
        assert!(
            compilation
                .issues
                .iter()
                .any(|i| i.code.ends_with("incompatible")),
            "{:?}",
            compilation.issues
        );
    }
}

#[test]
fn known_unsupported_preparation_modes_refuse_before_compilation() {
    let text = text();
    let c = capture(&text);
    let id = descriptors().remove(0).2;
    let identity = SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), text.as_bytes());
    for (name, mode) in [
        ("leaf", PreparationMode::Ready),
        ("sample", PreparationMode::CheckedReplacement),
        ("derived", PreparationMode::Ready),
    ] {
        assert!(
            native::export_operation(
                &c,
                &datatype(&c, name),
                OperationMode::Preparation(mode),
                identity.clone(),
                id.clone(),
                &hash(b"registry"),
                &OperationControl::default()
            )
            .is_err(),
            "{name}/{mode:?}"
        );
    }
}
