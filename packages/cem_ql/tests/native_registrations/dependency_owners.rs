//! CEMOS1 links actual host code without resolving the retained dependencies.
use super::*;
use native::NativeLinkError;
fn requirement(
    mode: OperationMode,
    name: &str,
    id: ImplementationIdentity,
) -> Arc<sources::VerifiedOperationSource> {
    let t = text()
        .replace(
            "{type @name=sample @kind=scalar}",
            "{type @name=sample @kind=scalar | {#colors}}",
        )
        .replace("@base=sample", "@base={#item}");
    let c = capture(&t);
    let built = sources::export_operation(
        &c,
        &datatype(&c, name),
        mode,
        identity_for(&t),
        id,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(checked(&built))
}
fn scope(
    h: &mut FreshStructuralHost,
    r: &sources::VerifiedOperationSource,
) -> cem_ql::schema_references::DeclarationScope {
    h.register_scope(
        r.source().clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .unwrap()
}
#[test]
fn dependency_operation_owners_link_all_direct_modes_without_evaluating_dependencies() {
    for (mode, name, id) in descriptors().into_iter().chain(additions()) {
        let r = requirement(mode, name, id.clone());
        let mut h = host();
        let engine = Engine::default();
        register_additional(&mut h, id, engine.clone()).unwrap();
        let s = scope(&mut h, &r);
        let owner = h.link_dependency_operation(r.clone(), s).unwrap();
        let original = r.datatype_source();
        let mut registry = DatatypeImplementations::default();
        assert!(matches!(
            owner.install(&datatype(&capture(&dependency_text()), name), &mut registry),
            Err(NativeLinkError::InvalidSource)
        ));
        owner.install(original, &mut registry).unwrap();
        assert!(matches!(
            owner.install(original, &mut registry),
            Err(NativeLinkError::Duplicate)
        ));
        assert!(engine.calls.iter().all(|n| n.load(Ordering::Acquire) == 0));
        assert!(!r.selections_verified());
        assert!(!r.executable_closure_verified());
        owner.retire();
        assert!(matches!(
            owner.install(original, &mut DatatypeImplementations::default()),
            Err(NativeLinkError::Expired)
        ));
    }
}

#[test]
fn dependency_operation_owners_require_exact_implementations_ready_scopes_and_registries() {
    let (mode, name, id) = additions().remove(0);
    let r = requirement(mode, name, id.clone());
    let mut h = host();
    let s = scope(&mut h, &r);
    assert!(matches!(
        h.link_dependency_operation(r.clone(), s),
        Err(NativeLinkError::Unavailable)
    ));
    let mut wrong = id.clone();
    wrong.module_hash = hash(b"wrong-module");
    register_additional(&mut h, wrong, Engine::default()).unwrap();
    assert!(matches!(
        h.link_dependency_operation(r.clone(), s),
        Err(NativeLinkError::IdentityMismatch)
    ));
    let mut h = host();
    register_additional(&mut h, id.clone(), Engine::default()).unwrap();
    let copy = requirement(mode, name, id.clone());
    let s = scope(&mut h, &copy);
    assert!(matches!(
        h.link_dependency_operation(r.clone(), s),
        Err(NativeLinkError::InvalidSource)
    ));
    let s = h
        .register_scope(
            r.source().clone(),
            None,
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    assert!(matches!(
        h.link_dependency_operation(r.clone(), s),
        Err(NativeLinkError::InvalidSource)
    ));
    let mut wrong_registry =
        FreshStructuralHost::new(&hash(b"other-registry"), Default::default()).unwrap();
    register_additional(&mut wrong_registry, id, Engine::default()).unwrap();
    let s = scope(&mut wrong_registry, &r);
    assert!(matches!(
        wrong_registry.link_dependency_operation(r, s),
        Err(NativeLinkError::IdentityMismatch)
    ));
}

#[test]
fn dependency_operation_owners_expire_without_retaining_host_callbacks() {
    for change in ["context", "remove", "stop", "close", "retire", "drop"] {
        let (mode, name, id) = additions().remove(0);
        let r = requirement(mode, name, id.clone());
        let control = OperationControl::default();
        let mut h = FreshStructuralHost::new(&hash(b"registry"), control.clone()).unwrap();
        let engine = Engine::default();
        let weak = Arc::downgrade(&engine.owner);
        register_additional(&mut h, id.clone(), engine.clone()).unwrap();
        let s = scope(&mut h, &r);
        let owner = h.link_dependency_operation(r.clone(), s).unwrap();
        assert!(owner.is_live_on(&h));
        assert!(!owner.is_live_on(&host()));
        let mut registry = DatatypeImplementations::default();
        owner.install(r.datatype_source(), &mut registry).unwrap();
        match change {
            "context" => assert!(h.set_context(s, Some(Default::default()))),
            "remove" => h.remove_native_implementation(&id.id).unwrap(),
            "stop" => {
                control.cancel_root(None, None).unwrap();
            }
            "close" => h.close(),
            "retire" => owner.retire(),
            "drop" => (),
            _ => unreachable!(),
        }
        if change != "drop" {
            assert!(!owner.is_live_on(&h));
            assert!(matches!(
                owner.install(r.datatype_source(), &mut DatatypeImplementations::default()),
                Err(NativeLinkError::Expired)
            ));
        }
        drop(h);
        drop(engine);
        assert!(
            weak.upgrade().is_none(),
            "metadata, owner and installed proxies must retain no callback"
        );
        assert!(matches!(
            owner.install(r.datatype_source(), &mut DatatypeImplementations::default()),
            Err(NativeLinkError::Expired)
        ));
    }
}

#[test]
fn dependency_source_operation_owners_reuse_checked_original_result_profiles() {
    for (profile, id) in source_descriptors() {
        let t = source_text(profile).replace(
            "{type @name=sample @kind=scalar}",
            "{type @name=sample @kind=scalar @base={#base} | {#colors}}",
        );
        let c = capture(&t);
        let b = sources::export_source_operation(
            &c,
            &datatype(&c, "sample"),
            &node(&c, "behavior"),
            profile,
            identity_for(&t),
            id.clone(),
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap();
        let r = Arc::new(checked(&b));
        let engine = Engine::default();
        let mut h = host();
        register_source(&mut h, id, engine.clone()).unwrap();
        let s = scope(&mut h, &r);
        assert!(matches!(
            h.link_dependency_operation(r.clone(), s),
            Err(NativeLinkError::InvalidSource)
        ));
        let wrong = if profile == SourceProfile::Conversion {
            SourceProfile::Equality
        } else {
            SourceProfile::Conversion
        };
        assert!(matches!(
            h.link_dependency_source_operation(r.clone(), s, adapter_for(wrong)),
            Err(NativeLinkError::InvalidSource)
        ));
        let owner = h
            .link_dependency_source_operation(r.clone(), s, adapter_for(profile))
            .unwrap();
        let mut registry = DatatypeImplementations::default();
        owner.install(r.datatype_source(), &mut registry).unwrap();
        assert!(engine.calls.iter().all(|n| n.load(Ordering::Acquire) == 0));
        assert!(owner.is_live_on(&h));
        owner.retire();
        assert!(!owner.is_live_on(&h));
        assert!(matches!(
            owner.install(r.datatype_source(), &mut DatatypeImplementations::default()),
            Err(NativeLinkError::Expired)
        ));
    }
}

fn tokenizer_requirement(checked_replacement: bool) -> Arc<sources::VerifiedOperationSource> {
    let t = text()
        .replace("@base=sample", "@base={#item}")
        .replace("@list-base=names", "@list-base={#list}");
    let c = capture(&t);
    let name = if checked_replacement {
        "derived"
    } else {
        "names"
    };
    let mode = if checked_replacement {
        TokenizerMode::CheckedReplacement
    } else {
        TokenizerMode::Ready
    };
    let b = sources::export_tokenizer(
        &c,
        &datatype(&c, name),
        mode,
        identity_for(&t),
        tokenizer_id(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(checked(&b))
}
fn list_implementation(r: &sources::VerifiedOperationSource) -> DatatypeImplementation {
    DatatypeImplementation {
        source: r.datatype_source().clone(),
        kind: DatatypeKind::List,
        representation: ValueRepresentation::List(ScalarRepresentation::String),
        accepted_bases: vec![],
        bounds: Default::default(),
        tokenizer: TokenizerBinding::Absent,
        validator: None,
    }
}
#[test]
fn dependency_tokenizer_owners_preserve_ready_and_checked_replacement_modes_and_lifetimes() {
    for replacement in [false, true] {
        let r = tokenizer_requirement(replacement);
        let mut h = host();
        h.register_native_tokenizer(tokenizer_id(), Whitespace)
            .unwrap();
        let s = scope(&mut h, &r);
        let owner = h.link_dependency_tokenizer(r.clone(), s).unwrap();
        assert!(owner.is_live_on(&h));
        assert!(!owner.is_live_on(&host()));
        assert!(matches!(
            h.link_dependency_operation(r.clone(), s),
            Err(NativeLinkError::InvalidSource)
        ));
        let mut implementation = list_implementation(&r);
        owner.install(&mut implementation).unwrap();
        let tokenizer = match &implementation.tokenizer {
            TokenizerBinding::CheckedReplacement(t) if replacement => t,
            TokenizerBinding::Ready(t) if !replacement => t,
            _ => panic!("mode changed"),
        };
        let control = OperationControl::default();
        assert!(tokenizer
            .tokenize(
                Some(LexicalInput::new(Arc::from("one two"), Default::default())),
                &control,
                ROOT_EXECUTION_SCOPE_ID,
                Default::default()
            )
            .is_ok());
        owner.retire();
        assert!(tokenizer
            .tokenize(
                Some(LexicalInput::new(Arc::from("one two"), Default::default())),
                &control,
                ROOT_EXECUTION_SCOPE_ID,
                Default::default()
            )
            .is_err());
        assert!(matches!(
            owner.install(&mut list_implementation(&r)),
            Err(NativeLinkError::Expired)
        ));
    }
}

#[test]
fn dependency_operation_owners_preserve_checked_replacement_preparation_facets_and_items() {
    let t = dependency_text();
    let c = capture(&t);
    for (mode, name, id) in [
        (
            OperationMode::Preparation(PreparationMode::CheckedReplacement),
            "sample",
            descriptors().remove(0).2,
        ),
        (
            OperationMode::FacetProfile(native::FacetMode::CheckedReplacement),
            "sample",
            additions().remove(1).2,
        ),
        (
            OperationMode::ListItems(PreparationMode::CheckedReplacement),
            "derived",
            additions().pop().unwrap().2,
        ),
    ] {
        let b = sources::export_operation(
            &c,
            &datatype(&c, name),
            mode,
            identity_for(&t),
            id.clone(),
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap();
        let r = Arc::new(checked(&b));
        let mut h = host();
        register_additional(&mut h, id, Engine::default()).unwrap();
        let s = scope(&mut h, &r);
        let owner = h.link_dependency_operation(r.clone(), s).unwrap();
        owner
            .install(r.datatype_source(), &mut DatatypeImplementations::default())
            .unwrap();
    }
}

#[derive(Debug)]
struct CancelTokens {
    control: OperationControl,
    armed: Arc<AtomicBool>,
    _owner: Arc<()>,
}
impl Tokenizer for CancelTokens {
    fn tokenize(
        &self,
        request: TokenizationRequest<'_>,
    ) -> Result<Vec<Range<usize>>, TokenizationError> {
        let result = Whitespace.tokenize(request);
        if self.armed.load(Ordering::Acquire) {
            self.control.cancel_root(None, None).unwrap();
        }
        result
    }
}
#[test]
fn dependency_tokenizer_owners_refuse_foreign_installation_and_late_success_without_retaining_code()
{
    let r = tokenizer_requirement(false);
    let control = OperationControl::default();
    let armed = Arc::new(AtomicBool::new(false));
    let marker = Arc::new(());
    let weak = Arc::downgrade(&marker);
    let mut h = FreshStructuralHost::new(&hash(b"registry"), control.clone()).unwrap();
    h.register_native_tokenizer(
        tokenizer_id(),
        CancelTokens {
            control: control.clone(),
            armed: armed.clone(),
            _owner: marker,
        },
    )
    .unwrap();
    let s = scope(&mut h, &r);
    let owner = h.link_dependency_tokenizer(r.clone(), s).unwrap();
    let mut copy = list_implementation(&tokenizer_requirement(false));
    assert!(matches!(
        owner.install(&mut copy),
        Err(NativeLinkError::InvalidSource)
    ));
    assert!(matches!(copy.tokenizer, TokenizerBinding::Absent));
    let mut implementation = list_implementation(&r);
    owner.install(&mut implementation).unwrap();
    let TokenizerBinding::Ready(tokens) = implementation.tokenizer else {
        panic!("ready tokenizer")
    };
    let lexical = LexicalInput::new(Arc::from("one two"), Default::default());
    let result = tokens
        .tokenize(
            Some(lexical.clone()),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            Default::default(),
        )
        .unwrap();
    assert_eq!(result.tokens, vec![0..3, 4..7]);
    armed.store(true, Ordering::Release);
    assert!(matches!(
        tokens.tokenize(
            Some(lexical),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            Default::default()
        ),
        Err(TokenizationError::Control(_))
    ));
    assert!(!owner.is_live_on(&h));
    drop(h);
    assert!(weak.upgrade().is_none());
}
