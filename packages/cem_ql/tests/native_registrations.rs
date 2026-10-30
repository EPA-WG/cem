use cem_ml::{
    events::cem::CemEventNormalizer,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::CemAstNode,
    schema::{
        datatype_contracts::{
            RegisteredTokenizer, TokenizationError, TokenizationRequest, Tokenizer,
        },
        datatype_registry::{DatatypeKind, DatatypeRegistry, DatatypeSource},
        datatype_validation::{
            CandidateRequirement, DatatypeBehaviorContract, ResultRepresentation,
            ScalarRepresentation, ValidationSignature, ValueRepresentation,
        },
        declaration_references::SchemaDeclarationNode,
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        reference_policy::ReferenceScopePolicy,
        structural_capsule::SourceIdentity,
        value_contracts::{ContractName, ValueContractSource, ValueContracts},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::{
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{
        NativeDatatypeValidator, RuleExecution, ValidationCall, ValidationInput,
    },
    eval::{AtomValue, Item, ItemStream},
    native_registrations::{
        self as native, ImplementationIdentity, NativeSignature, TokenizerMode,
    },
    structural_receiver::FreshStructuralHost,
};
use std::{collections::BTreeMap, ops::Range, sync::Arc};
const NS: &str = "https://cem.dev/ns/schema/1";
const TEXT: &str = r#"@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{schema @name=example @namespace=urn:example |
 {types | {type @name=sample @kind=scalar} {type @name=names @kind=list @base=sample} {type @name=derived @list-base=names}}
 {behaviors | {behavior @name=check @implementation=engine @primitive=urn:test:validate @execution=datatype-validation |
  {inputs |
   {input-binding @name=value @type=schema:string @source=value @required=true @cardinality=one}
   {input-binding @name=datatype @type=schema:node @source=datatype @required=true @cardinality=one}
   {input-binding @name=candidate @type=schema:node @source=candidate @required=false @cardinality=zero-or-one}}
  {result @type=schema:datatype-validation-result}}}}
"#;
fn capture(text: &str) -> Arc<LexicallyScopedDocument> {
    Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    )
}
fn node(capture: &LexicallyScopedDocument, name: &str) -> SchemaDeclarationNode {
    capture
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                expanded_name,
                node_id,
                ..
            } if expanded_name.local_name == name => {
                SchemaDeclarationNode::new(capture.document().clone(), *node_id)
            }
            _ => None,
        })
        .unwrap()
}
fn signature() -> ValidationSignature {
    ValidationSignature {
        kind: DatatypeKind::Scalar,
        value: ValueRepresentation::Scalar(ScalarRepresentation::String),
        candidate: CandidateRequirement::Optional,
        result: ResultRepresentation::Accepted(ContractName::new(NS, "datatype-validation-result")),
    }
}
fn contract(c: &Arc<LexicallyScopedDocument>) -> DatatypeBehaviorContract {
    let source = ValueContractSource::new(
        node(c, "schema"),
        NS,
        BTreeMap::from([("schema".into(), NS.into())]),
    )
    .with_captured_names(c.clone())
    .unwrap();
    DatatypeBehaviorContract::compile(&source, &node(c, "behavior"), signature()).unwrap()
}
fn datatype(c: &LexicallyScopedDocument, name: &str) -> DatatypeSource {
    let mut types = DatatypeRegistry::default();
    let scope = node(c, "schema");
    for n in &c.document().nodes {
        if let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = n
        {
            if expanded_name.local_name == "type" {
                types
                    .insert(
                        scope.clone(),
                        SchemaDeclarationNode::new(c.document().clone(), *node_id).unwrap(),
                    )
                    .unwrap();
            }
        }
    }
    types.source(&scope, name).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    cem_ml::content_cache::ContentHash::from_blake3(bytes).header_value()
}
fn identity() -> SourceIdentity {
    SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), TEXT.as_bytes())
}
fn validator_id() -> ImplementationIdentity {
    ImplementationIdentity::validator(
        "urn:test:validate",
        hash(b"module-a"),
        "validate",
        signature(),
    )
}
fn tokenizer_id() -> ImplementationIdentity {
    ImplementationIdentity::tokenizer("urn:test:tokens", hash(b"module-a"), "tokenize")
}
fn adapter() -> DatatypeResultAdapter {
    let c = capture(
        cem_ml::schema::package_sources::builtin_schema_package_source("schema")
            .unwrap()
            .schema_source,
    );
    let src = ValueContractSource::new(
        node(&c, "schema"),
        NS,
        BTreeMap::from([("schema".into(), NS.into())]),
    )
    .with_captured_names(c)
    .unwrap();
    DatatypeResultAdapter::new(
        Arc::new(ValueContracts::compile(&[src], Default::default()).unwrap()),
        ContractName::new(NS, "datatype-validation-result"),
        ContractName::new(NS, "datatype-diagnostic"),
    )
    .unwrap()
}
fn host() -> FreshStructuralHost {
    FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap()
}
#[derive(Debug)]
struct Validator;
impl NativeDatatypeValidator for Validator {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        RuleExecution::Complete(ItemStream::once(Item::Record(BTreeMap::from([
            (
                "accepted".into(),
                vec![Item::Atomic(AtomValue::Boolean(
                    call.value[0].atom() == Some(AtomValue::String("same".into())),
                ))],
            ),
            ("diagnostics".into(), vec![]),
        ]))))
    }
}
#[derive(Debug)]
struct Whitespace;
impl Tokenizer for Whitespace {
    fn tokenize(
        &self,
        request: TokenizationRequest<'_>,
    ) -> Result<Vec<Range<usize>>, TokenizationError> {
        RegisteredTokenizer::whitespace()
            .tokenize(
                Some(request.input.clone()),
                request.control,
                request.scope,
                request.limits,
            )
            .map(|t| t.tokens)
    }
}
fn validator() -> Arc<native::VerifiedNativeRequirement> {
    let c = capture(TEXT);
    let built = native::export_validator(
        &c,
        &contract(&c),
        identity(),
        validator_id(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap())
}
fn tokenizer(name: &str, mode: TokenizerMode) -> Arc<native::VerifiedNativeRequirement> {
    tokenizer_with(name, mode, tokenizer_id())
}
fn tokenizer_with(
    name: &str,
    mode: TokenizerMode,
    implementation: ImplementationIdentity,
) -> Arc<native::VerifiedNativeRequirement> {
    let c = capture(TEXT);
    let built = native::export_tokenizer(
        &c,
        &datatype(&c, name),
        mode,
        identity(),
        implementation,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap())
}
#[test]
fn exact_native_validator_requires_available_metadata_and_real_original_source() {
    let req = validator();
    let mut host = host();
    let scope = host
        .register_scope(
            req.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    assert!(matches!(
        host.link_native_validator(
            req.clone(),
            scope,
            datatype(req.capture(), "sample").declaration().node_id(),
            adapter()
        ),
        Err(native::NativeLinkError::Unavailable)
    ));
    host.register_native_validator(validator_id(), Validator)
        .unwrap();
    let binding = host
        .link_native_validator(
            req.clone(),
            scope,
            datatype(req.capture(), "sample").declaration().node_id(),
            adapter(),
        )
        .unwrap();
    let call = host.query_invocation(scope).unwrap();
    for (value, accepted) in [("same", true), ("different", false)] {
        let input = ValidationInput {
            value: vec![Item::Atomic(AtomValue::String(value.into()))],
            candidate: vec![],
            fallback: Default::default(),
        };
        assert_eq!(
            host.validate_query(&call, &binding, &input, Default::default())
                .unwrap()
                .accepted,
            Some(accepted)
        );
    }
}
#[test]
fn guarded_tokenizer_is_installed_only_on_its_original_list() {
    let req = tokenizer("names", TokenizerMode::Ready);
    let mut host = host();
    host.register_native_tokenizer(tokenizer_id(), Whitespace)
        .unwrap();
    let scope = host
        .register_scope(
            req.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let linked = host.link_native_tokenizer(req.clone(), scope).unwrap();
    let source = datatype(req.capture(), "names");
    let mut entry = cem_ql::datatype_compilation::DatatypeImplementation {
        source,
        kind: DatatypeKind::List,
        representation: ValueRepresentation::List(ScalarRepresentation::String),
        accepted_bases: vec![],
        bounds: Default::default(),
        tokenizer: cem_ql::datatype_compilation::TokenizerBinding::Absent,
        validator: None,
    };
    linked.install(&mut entry).unwrap();
    let cem_ql::datatype_compilation::TokenizerBinding::Ready(tokens) = entry.tokenizer else {
        panic!("ready tokenizer");
    };
    let result = tokens
        .tokenize(
            Some(cem_ml::schema::datatype_contracts::LexicalInput::new(
                Arc::from("  α  beta "),
                Default::default(),
            )),
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            Default::default(),
        )
        .unwrap();
    assert_eq!(result.tokens, vec![2..4, 6..10]);
    host.close();
    assert!(matches!(
        tokens.tokenize(
            Some(result.input),
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            Default::default()
        ),
        Err(TokenizationError::Unavailable)
    ));
}

#[test]
fn native_tokenizers_preserve_callback_availability_and_release_retired_owners() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Owned {
        _owner: Arc<()>,
        available: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
        expire_in_separator: bool,
    }
    impl Tokenizer for Owned {
        fn check_available(&self) -> Result<(), TokenizationError> {
            if self.available.load(Ordering::Acquire) {
                Ok(())
            } else {
                Err(TokenizationError::Unavailable)
            }
        }
        fn tokenize(
            &self,
            request: TokenizationRequest<'_>,
        ) -> Result<Vec<Range<usize>>, TokenizationError> {
            self.calls.fetch_add(1, Ordering::AcqRel);
            Whitespace.tokenize(request)
        }
        fn accepts_separator(&self, text: &str) -> bool {
            if self.expire_in_separator {
                self.available.store(false, Ordering::Release);
                false
            } else {
                text.chars().all(char::is_whitespace)
            }
        }
    }
    for case in 0..5 {
        let req = tokenizer("names", TokenizerMode::Ready);
        let control = OperationControl::default();
        let mut host = FreshStructuralHost::new(&hash(b"registry"), control.clone()).unwrap();
        let marker = Arc::new(());
        let weak = Arc::downgrade(&marker);
        let calls = Arc::new(AtomicUsize::new(0));
        host.register_native_tokenizer(
            tokenizer_id(),
            Owned {
                _owner: marker,
                available: Arc::new(AtomicBool::new(true)),
                calls: calls.clone(),
                expire_in_separator: case == 3,
            },
        )
        .unwrap();
        let scope = host
            .register_scope(
                req.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let linked = host.link_native_tokenizer(req.clone(), scope).unwrap();
        let mut implementation = cem_ql::datatype_compilation::DatatypeImplementation {
            source: datatype(req.capture(), "names"),
            kind: DatatypeKind::List,
            representation: ValueRepresentation::List(ScalarRepresentation::String),
            accepted_bases: vec![],
            bounds: Default::default(),
            tokenizer: cem_ql::datatype_compilation::TokenizerBinding::Absent,
            validator: None,
        };
        linked.install(&mut implementation).unwrap();
        let cem_ql::datatype_compilation::TokenizerBinding::Ready(tokens) =
            implementation.tokenizer
        else {
            panic!("ready")
        };
        match case {
            0 => {
                host.remove_native_implementation(&tokenizer_id().id)
                    .unwrap();
                assert!(weak.upgrade().is_none());
            }
            1 => {
                control.cancel_root(Some("fixture".into()), None).unwrap();
            }
            2 => drop(linked),
            3 => (),
            _ => {
                host.register_native_validator(validator_id(), Validator)
                    .unwrap();
            }
        }
        // The caller may still be live while the implementation's host is not.
        assert!(
            matches!(
                tokens.tokenize(
                    Some(cem_ml::schema::datatype_contracts::LexicalInput::new(
                        Arc::from(" a "),
                        Default::default()
                    )),
                    &OperationControl::default(),
                    ROOT_EXECUTION_SCOPE_ID,
                    Default::default()
                ),
                Err(TokenizationError::Unavailable)
            ),
            "case {case}"
        );
        assert_eq!(calls.load(Ordering::Acquire), usize::from(case == 3));
        drop(host);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn implementation_identity_signature_and_original_allocation_must_match() {
    let req = validator();
    for changed in 0..3 {
        let mut host = host();
        let mut actual = validator_id();
        match changed {
            0 => actual.module_hash = hash(b"other module"),
            1 => actual.export = "other_export".into(),
            _ => {
                let NativeSignature::Validator(sig) = &mut actual.signature else {
                    unreachable!()
                };
                sig.candidate = CandidateRequirement::Required;
            }
        }
        host.register_native_validator(actual, Validator).unwrap();
        let scope = host
            .register_scope(
                req.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        assert!(matches!(
            host.link_native_validator(
                req.clone(),
                scope,
                datatype(req.capture(), "sample").declaration().node_id(),
                adapter()
            ),
            Err(native::NativeLinkError::IdentityMismatch)
        ));
    }
    let mut host = host();
    host.register_native_validator(validator_id(), Validator)
        .unwrap();
    let scope = host
        .register_scope(
            req.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let other = validator();
    assert!(matches!(
        host.link_native_validator(
            other.clone(),
            scope,
            datatype(other.capture(), "sample").declaration().node_id(),
            adapter()
        ),
        Err(native::NativeLinkError::InvalidSource)
    ));
    let binding = host
        .link_native_validator(
            req.clone(),
            scope,
            datatype(req.capture(), "sample").declaration().node_id(),
            adapter(),
        )
        .unwrap();
    assert_eq!(
        host.register_native_validator(validator_id(), Validator),
        Err(native::NativeLinkError::Duplicate)
    );
    let call = host.query_invocation(scope).unwrap();
    let input = ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("same".into()))],
        candidate: vec![],
        fallback: Default::default(),
    };
    assert_eq!(
        host.validate_query(&call, &binding, &input, Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}

#[test]
fn native_binding_replacement_cancellation_unavailable_and_drop_preserve_lifecycle() {
    #[derive(Debug)]
    struct Owned(Arc<()>);
    impl NativeDatatypeValidator for Owned {
        fn validate(&self, _: ValidationCall<'_>) -> RuleExecution {
            let _ = &self.0;
            RuleExecution::Unavailable(vec![])
        }
    }
    let req = validator();
    let marker = Arc::new(());
    let weak = Arc::downgrade(&marker);
    let mut host = host();
    host.register_native_validator(validator_id(), Owned(marker))
        .unwrap();
    let scope = host
        .register_scope(
            req.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let id = datatype(req.capture(), "sample").declaration().node_id();
    let binding = host
        .link_native_validator(req.clone(), scope, id, adapter())
        .unwrap();
    let call = host.query_invocation(scope).unwrap();
    let input = ValidationInput {
        value: vec![Item::Atomic(AtomValue::String("same".into()))],
        candidate: vec![],
        fallback: Default::default(),
    };
    let report = host
        .validate_query(&call, &binding, &input, Default::default())
        .unwrap();
    assert_eq!(report.accepted, None);
    assert!(matches!(
        report.stopped.unwrap().reason,
        cem_ql::datatype_validation::ValidationStopReason::Unavailable(_)
    ));
    assert!(host
        .replace_native_validator(&binding, req.clone(), scope, u32::MAX, adapter())
        .is_err());
    assert!(host
        .validate_query(&call, &binding, &input, Default::default())
        .is_ok());
    let replaced = host
        .replace_native_validator(&binding, req.clone(), scope, id, adapter())
        .unwrap();
    assert!(host
        .validate_query(&call, &binding, &input, Default::default())
        .is_err());
    assert!(host
        .validate_query(&call, &replaced, &input, Default::default())
        .is_ok());
    drop(host);
    assert!(
        weak.upgrade().is_none(),
        "a retained binding kept a retired native callback alive"
    );
    let mut fresh = super_host();
    fresh
        .register_native_validator(validator_id(), Validator)
        .unwrap();
    let scope = fresh
        .register_scope(
            req.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let fresh_call = fresh.query_invocation(scope).unwrap();
    assert!(fresh
        .validate_query(&fresh_call, &replaced, &input, Default::default())
        .is_err());
    let fresh_binding = fresh
        .link_native_validator(req.clone(), scope, id, adapter())
        .unwrap();
    assert!(fresh
        .validate_query(&call, &fresh_binding, &input, Default::default())
        .is_err());
    assert!(fresh.set_context(scope, Some(Default::default())));
    assert!(fresh
        .validate_query(&fresh_call, &fresh_binding, &input, Default::default())
        .is_err());
}
fn super_host() -> FreshStructuralHost {
    host()
}

#[test]
fn native_requirement_wire_refuses_repinned_abi_signature_and_source_changes() {
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
        Validator {
            scope: u32,
            behavior: u32,
        },
        Tokenizer {
            scope: u32,
            declaration: u32,
            mode: TokenizerMode,
        },
    }
    let c = capture(TEXT);
    let built = native::export_validator(
        &c,
        &contract(&c),
        identity(),
        validator_id(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    for mutation in 0..7 {
        let mut packet: Packet =
            rmp_serde::from_slice(&built.bytes[native::MAGIC.len()..]).unwrap();
        match mutation {
            0 => packet.implementation.abi.push_str("/other"),
            1 => packet.implementation.id = "urn:other".into(),
            2 => packet.implementation.signature = NativeSignature::TokenizerSpans,
            3 => packet.identity.uri.push_str("other"),
            4 => {
                packet.selection = Selection::Validator {
                    scope: 0,
                    behavior: 0,
                }
            }
            5 => packet.ql_version.push_str("other"),
            _ => packet.abi.push_str("other"),
        }
        let bytes = [native::MAGIC, &rmp_serde::to_vec_named(&packet).unwrap()].concat();
        let mut pin = built.pin.clone();
        pin.hash = hash(&bytes);
        assert!(native::verify(&bytes, &pin, &OperationControl::default()).is_err());
    }
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    signal.abort();
    assert!(native::verify(&built.bytes, &built.pin, &control).is_err());
    let unrelated = capture(TEXT);
    assert!(native::export_validator(
        &unrelated,
        &contract(&c),
        identity(),
        validator_id(),
        &hash(b"registry"),
        &OperationControl::default()
    )
    .is_err());
}

#[test]
fn availability_loss_during_separator_checks_is_not_a_lexical_rejection() {
    use std::sync::atomic::{AtomicBool, Ordering};
    #[derive(Debug)]
    struct Closing(AtomicBool);
    impl Tokenizer for Closing {
        fn check_available(&self) -> Result<(), TokenizationError> {
            if self.0.load(Ordering::Acquire) {
                Err(TokenizationError::Unavailable)
            } else {
                Ok(())
            }
        }
        fn tokenize(
            &self,
            _: TokenizationRequest<'_>,
        ) -> Result<Vec<Range<usize>>, TokenizationError> {
            Ok(vec![2..3])
        }
        fn accepts_separator(&self, _: &str) -> bool {
            self.0.store(true, Ordering::Release);
            false
        }
    }
    let tokenizer =
        RegisteredTokenizer::new("separator-close", Closing(AtomicBool::new(false))).unwrap();
    assert!(matches!(
        tokenizer.tokenize(
            Some(cem_ml::schema::datatype_contracts::LexicalInput::new(
                Arc::from("x a"),
                Default::default()
            )),
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            Default::default()
        ),
        Err(TokenizationError::Unavailable)
    ));
}

#[test]
fn related_requirements_share_only_the_complete_verified_source() {
    let base = tokenizer("names", TokenizerMode::Ready);
    let derived = tokenizer("derived", TokenizerMode::CheckedReplacement);
    assert!(!Arc::ptr_eq(base.source(), derived.source()));
    let shared = derived
        .share_source(&base, &OperationControl::default())
        .unwrap();
    assert!(Arc::ptr_eq(shared.source(), base.source()));
    let mut pin = identity();
    pin.uri.push_str("other");
    let c = capture(TEXT);
    let built = native::export_tokenizer(
        &c,
        &datatype(&c, "derived"),
        TokenizerMode::CheckedReplacement,
        pin,
        tokenizer_id(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let other = native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    assert!(other
        .share_source(&base, &OperationControl::default())
        .is_err());
    // Even an identical external identity is insufficient for a changed arena.
    let changed = capture(&TEXT.replace("@name=sample", "@name=otherx"));
    let built = native::export_tokenizer(
        &changed,
        &datatype(&changed, "derived"),
        TokenizerMode::CheckedReplacement,
        identity(),
        tokenizer_id(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let other = native::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap();
    assert!(other
        .share_source(&base, &OperationControl::default())
        .is_err());
}

#[test]
fn compiler_preserves_checked_tokenizer_replacement_admission_and_spans() {
    use cem_ml::schema::{
        datatype_contracts::{LexicalInput, TokenizationLimits},
        document_model::shipped_datatypes::ShippedDatatype,
        reference_traversal::ReferenceTraversalLimits,
    };
    use cem_ql::datatype_compilation::{
        compile_datatypes, DatatypeImplementation, DatatypeImplementations, ExecutableDatatype,
        TokenizerBinding,
    };
    use cem_ql::datatype_preparation::{
        PreparationBinding, PreparationInput, PreparationStop, RegisteredLexicalPreparation,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug)]
    struct Counting {
        count: Arc<AtomicUsize>,
        whole: bool,
    }
    impl Tokenizer for Counting {
        fn tokenize(
            &self,
            request: TokenizationRequest<'_>,
        ) -> Result<Vec<Range<usize>>, TokenizationError> {
            self.count.fetch_add(1, Ordering::AcqRel);
            if self.whole {
                Ok(vec![0..request.input.text.len()])
            } else {
                Whitespace.tokenize(request)
            }
        }
    }
    for incompatible in [false, true] {
        let base = tokenizer("names", TokenizerMode::Ready);
        let replacement = ImplementationIdentity::tokenizer(
            "urn:test:replacement",
            hash(b"module-b"),
            "tokenize",
        );
        let child = Arc::new(
            tokenizer_with(
                "derived",
                TokenizerMode::CheckedReplacement,
                replacement.clone(),
            )
            .share_source(&base, &OperationControl::default())
            .unwrap(),
        );
        let base_calls = Arc::new(AtomicUsize::new(0));
        let replacement_calls = Arc::new(AtomicUsize::new(0));
        let mut native_host = host();
        native_host
            .register_native_tokenizer(
                tokenizer_id(),
                Counting {
                    count: base_calls.clone(),
                    whole: false,
                },
            )
            .unwrap();
        native_host
            .register_native_tokenizer(
                replacement,
                Counting {
                    count: replacement_calls.clone(),
                    whole: incompatible,
                },
            )
            .unwrap();
        let scope = native_host
            .register_scope(
                base.source().clone(),
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
        let base_binding = native_host
            .link_native_tokenizer(base.clone(), scope)
            .unwrap();
        let child_binding = native_host.link_native_tokenizer(child, scope).unwrap();
        let sources = [
            datatype(base.capture(), "sample"),
            datatype(base.capture(), "names"),
            datatype(base.capture(), "derived"),
        ];
        let mut compiler_host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
        compiler_host.register_scope(
            base.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        );
        for (source, name) in sources.iter().zip(["sample", "names", "derived"]) {
            compiler_host
                .register_datatype_source(source.clone())
                .unwrap();
            compiler_host
                .bind_literal_datatype(source.scope(), name, source.declaration().clone())
                .unwrap();
        }
        let mut implementations = DatatypeImplementations::default();
        for (index, source) in sources.iter().enumerate() {
            let mut entry = DatatypeImplementation {
                source: source.clone(),
                kind: if index == 0 {
                    DatatypeKind::Scalar
                } else {
                    DatatypeKind::List
                },
                representation: if index == 0 {
                    ValueRepresentation::Scalar(ScalarRepresentation::String)
                } else {
                    ValueRepresentation::List(ScalarRepresentation::String)
                },
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            };
            match index {
                1 => base_binding.install(&mut entry).unwrap(),
                2 => child_binding.install(&mut entry).unwrap(),
                _ => (),
            }
            implementations.register(entry).unwrap();
        }
        implementations
            .select_preparation(
                sources[0].clone(),
                PreparationBinding::Ready(
                    cem_ql::datatype_shipped::lexical_preparation(
                        sources[0].clone(),
                        ShippedDatatype::String,
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        implementations
            .select_preparation(
                sources[1].clone(),
                PreparationBinding::Ready(
                    RegisteredLexicalPreparation::list_items(
                        sources[1].clone(),
                        "items",
                        ScalarRepresentation::String,
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        let compiled = compile_datatypes(
            base.source().ast_owner().clone(),
            &sources,
            &mut compiler_host,
            &implementations,
            &Default::default(),
            ReferenceTraversalLimits::schema_defaults().unwrap(),
        );
        assert!(compiled.is_ready(), "{:?}", compiled.issues);
        let descriptor = compiled
            .contracts
            .iter()
            .find(|c| c.source().declaration().identity() == sources[2].declaration().identity())
            .unwrap()
            .as_any()
            .downcast_ref::<ExecutableDatatype>()
            .unwrap();
        assert_eq!(descriptor.tokenizer().unwrap().invocations(), 2);
        let lexical = LexicalInput::new(Arc::from("  α  beta "), Default::default());
        let runtime = cem_ql::datatype_validation::ValidationRuntime {
            control: &OperationControl::default(),
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let input = PreparationInput {
            lexical: lexical.clone(),
            candidate: vec![],
            fallback: Default::default(),
        };
        let result = descriptor.prepare_lexical(&input, &runtime, Default::default());
        assert_eq!(base_calls.load(Ordering::Acquire), 1);
        assert_eq!(replacement_calls.load(Ordering::Acquire), 1);
        if incompatible {
            assert!(matches!(
                result.stopped,
                Some(PreparationStop::IncompatibleReplacement)
            ));
            assert_eq!(result.accepted, None);
        } else {
            assert_eq!(result.accepted, Some(true), "{result:?}");
            assert_eq!(result.token_spans, vec![2..4, 6..10]);
        }
        assert!(matches!(
            descriptor.tokenizer().unwrap().tokenize(
                Some(lexical.clone()),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
                TokenizationLimits {
                    max_tokenizers: 1,
                    ..Default::default()
                }
            ),
            Err(TokenizationError::Limit)
        ));
        native_host.close();
        let expired = descriptor.prepare_lexical(&input, &runtime, Default::default());
        assert!(matches!(
            expired.stopped,
            Some(PreparationStop::Unavailable)
        ));
        assert_eq!(expired.accepted, None);
    }
}

#[path = "native_registrations/operations.rs"]
mod operations;
