use super::*;
use cem_ml::schema::reference_policy::ReferenceScopePolicy;
use cem_ql::{
    query_validator_capsule::declarations as portable, structural_receiver::FreshStructuralHost,
};

fn built(text: &str) -> portable::BuiltQueryDeclaration {
    let capture = capture(text);
    let source = from_capture(capture.clone());
    portable::export(
        &capture,
        &contract(&source),
        identity(text),
        &registry_hash(),
        &OperationControl::default(),
    )
    .unwrap()
}
fn verified(text: &str) -> Arc<portable::VerifiedQueryDeclaration> {
    let built = built(text);
    Arc::new(portable::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap())
}
fn host() -> FreshStructuralHost {
    FreshStructuralHost::new(&registry_hash(), OperationControl::default()).unwrap()
}
fn register(
    host: &mut FreshStructuralHost,
    declaration: &portable::VerifiedQueryDeclaration,
) -> cem_ql::schema_references::DeclarationScope {
    host.register_scope(
        declaration.source().clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .unwrap()
}
fn datatype(declaration: &portable::VerifiedQueryDeclaration) -> u32 {
    declaration
        .source()
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "type" => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
fn input(value: &str) -> ValidationInput {
    ValidationInput {
        value: vec![Item::Atomic(AtomValue::String(value.into()))],
        candidate: vec![],
        fallback: Default::default(),
    }
}

#[test]
fn portable_declarations_restore_fresh_owners_and_native_diagnostic_provenance() {
    let text = declaration(
        r#"{accepted: str:length(value) > 2, diagnostics: {code: "query.note", severity: "warning", message: "original", source: datatype}}"#,
    );
    let built = built(&text);
    let first =
        Arc::new(portable::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap());
    let second =
        Arc::new(portable::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap());
    assert!(!Arc::ptr_eq(
        first.source().ast_owner(),
        second.source().ast_owner()
    ));
    assert_eq!(first.source().source_uri(), identity(&text).uri);
    assert_eq!(
        first.source().ast().nodes.len(),
        capture(&text).document().nodes.len()
    );
    let original = source(&text);
    let mut full = DatatypeValidationRegistry::default();
    full.register_query(contract(&original), adapter(), None)
        .unwrap();
    let mut host = host();
    let scope = register(&mut host, &first);
    let binding = host
        .link_query(
            first.clone(),
            scope,
            datatype(&first),
            &functions(),
            adapter(),
        )
        .unwrap();
    let invocation = host.query_invocation(scope).unwrap();
    for value in ["a", "same"] {
        let native = run(&original, &full, value);
        let restored = host
            .validate_query(&invocation, &binding, &input(value), Default::default())
            .unwrap();
        assert_eq!(restored.accepted, native.accepted);
        let a = &native.completed[0].result.diagnostics[0];
        let b = &restored.completed[0].result.diagnostics[0];
        assert_eq!(
            (
                &a.code,
                &a.uri,
                &a.source_map,
                a.byte_offset,
                a.line,
                a.column
            ),
            (
                &b.code,
                &b.uri,
                &b.source_map,
                b.byte_offset,
                b.line,
                b.column
            )
        );
        assert!(Arc::ptr_eq(
            restored.completed[0].datatype.document(),
            first.source().ast_owner()
        ));
    }
}

#[test]
fn query_bindings_expire_with_real_host_contexts_and_cannot_cross_hosts() {
    let declaration = verified(&declaration("{accepted: true, diagnostics: ()}"));
    let mut original = host();
    let scope = register(&mut original, &declaration);
    let binding = original
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    let invocation = original.query_invocation(scope).unwrap();
    let mut foreign = host();
    let foreign_scope = register(&mut foreign, &declaration);
    let foreign_invocation = foreign.query_invocation(foreign_scope).unwrap();
    assert!(foreign
        .validate_query(
            &foreign_invocation,
            &binding,
            &input("same"),
            Default::default()
        )
        .is_err());
    assert!(original
        .validate_query(
            &foreign_invocation,
            &binding,
            &input("same"),
            Default::default()
        )
        .is_err());
    assert!(original.set_context(scope, Some(Default::default())));
    let next = original.query_invocation(scope).unwrap();
    assert!(original
        .validate_query(&next, &binding, &input("same"), Default::default())
        .is_err());
    let fresh = original
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    assert!(original
        .validate_query(&invocation, &fresh, &input("same"), Default::default())
        .is_err());
    assert_eq!(
        original
            .validate_query(&next, &fresh, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    fresh.retire();
    assert!(original
        .validate_query(&next, &fresh, &input("same"), Default::default())
        .is_err());
    let other = register(&mut original, &declaration);
    let binding = original
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    let invocation = original.query_invocation(scope).unwrap();
    assert!(original.allow_scope_crossing(scope, other));
    assert!(original
        .validate_query(&invocation, &binding, &input("same"), Default::default())
        .is_err());
    let invocation = original.query_invocation(scope).unwrap();
    let binding = original
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    assert!(original.allow_scope_crossing(scope, other));
    assert_eq!(
        original
            .validate_query(&invocation, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    original.close();
    assert!(original.query_invocation(scope).is_err());
}

#[test]
fn unavailable_functions_pending_scopes_and_foreign_source_allocations_never_link() {
    let text = declaration("{accepted: str:length(value) > 0, diagnostics: ()}");
    let declaration = verified(&text);
    let other = verified(&text);
    let mut host = host();
    let scope = register(&mut host, &declaration);
    assert!(host
        .link_query(
            other.clone(),
            scope,
            datatype(&other),
            &functions(),
            adapter()
        )
        .is_err());
    assert!(host
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &SelectedFunctionRegistry::new([]).unwrap(),
            adapter()
        )
        .is_err());
    let binding = host
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    let call = host.query_invocation(scope).unwrap();
    assert_eq!(
        host.validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert!(host.set_context(scope, None));
    assert!(host.query_invocation(scope).is_err());
    assert!(host
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter()
        )
        .is_err());
}

#[test]
fn replacement_is_staged_and_drop_never_transfers_invocation_authority() {
    let text = declaration("{accepted: str:length(value) > 0, diagnostics: ()}");
    let declaration = verified(&text);
    let mut owner = host();
    let scope = register(&mut owner, &declaration);
    let binding = owner
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    let invocation = owner.query_invocation(scope).unwrap();
    assert!(owner
        .replace_query(
            &binding,
            declaration.clone(),
            scope,
            datatype(&declaration),
            &SelectedFunctionRegistry::new([]).unwrap(),
            adapter()
        )
        .is_err());
    assert_eq!(
        owner
            .validate_query(&invocation, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let replaced = owner
        .replace_query(
            &binding,
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    assert!(owner
        .validate_query(&invocation, &binding, &input("same"), Default::default())
        .is_err());
    assert_eq!(
        owner
            .validate_query(&invocation, &replaced, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    drop(owner);
    let fresh_source = verified(&text);
    let mut fresh = host();
    let scope = register(&mut fresh, &fresh_source);
    let call = fresh.query_invocation(scope).unwrap();
    assert!(fresh
        .validate_query(&call, &replaced, &input("same"), Default::default())
        .is_err());
    let binding = fresh
        .link_query(
            fresh_source.clone(),
            scope,
            datatype(&fresh_source),
            &functions(),
            adapter(),
        )
        .unwrap();
    assert!(fresh
        .validate_query(&invocation, &binding, &input("same"), Default::default())
        .is_err());
    assert_eq!(
        fresh
            .validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}

#[test]
fn independent_pins_refuse_mixed_programs_unsupported_envelopes_and_bad_wire() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Packet {
        abi: String,
        ml_version: String,
        ql_version: String,
        source: cem_ml::schema::native_source::NativeSourceSnapshot,
        program: Vec<u8>,
    }
    let text = declaration("{accepted: true, diagnostics: ()}");
    let original = built(&text);
    let decode =
        || rmp_serde::from_slice::<Packet>(&original.bytes[portable::MAGIC.len()..]).unwrap();
    let encode = |packet: &Packet| {
        let bytes = [portable::MAGIC, &rmp_serde::to_vec_named(packet).unwrap()].concat();
        let mut pin = original.pin.clone();
        pin.hash = cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value();
        (bytes, pin)
    };
    for mutation in 0..4 {
        let mut packet = decode();
        match mutation {
            0 => packet.abi.push_str("/other"),
            1 => packet.ml_version.push_str("/other"),
            2 => packet.ql_version.push_str("/other"),
            _ => packet.program.push(0),
        }
        let (bytes, pin) = encode(&packet);
        assert!(portable::verify(&bytes, &pin, &OperationControl::default()).is_err());
    }
    let other = built(&declaration("{accepted: false, diagnostics: ()}"));
    let mut packet = decode();
    packet.program = rmp_serde::from_slice::<Packet>(&other.bytes[portable::MAGIC.len()..])
        .unwrap()
        .program;
    let (bytes, mut pin) = encode(&packet);
    pin.program = other.pin.program;
    assert!(portable::verify(&bytes, &pin, &OperationControl::default()).is_err());
    for body in [vec![0xdd, 255, 255, 255, 255], vec![0x91; 70]] {
        let bytes = [portable::MAGIC, &body].concat();
        let mut pin = original.pin.clone();
        pin.hash = cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value();
        assert!(portable::verify(&bytes, &pin, &OperationControl::default()).is_err());
    }
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    signal.abort();
    assert!(portable::verify(&original.bytes, &original.pin, &control).is_err());
    let capture = capture(&text);
    let unrelated = source(&text);
    assert!(portable::export(
        &capture,
        &contract(&unrelated),
        identity(&text),
        &registry_hash(),
        &OperationControl::default()
    )
    .is_err());
    let dynamic = text.replace(
        "{type @name=sample @kind=scalar}",
        "{type @name=sample @kind=scalar} {#pending}",
    );
    let capture = super::capture(&dynamic);
    let source = from_capture(capture.clone());
    assert!(portable::export(
        &capture,
        &contract(&source),
        identity(&dynamic),
        &registry_hash(),
        &OperationControl::default()
    )
    .is_err());
}

#[test]
fn invocation_budgets_are_shared_and_file_source_uris_survive_restore() {
    let text = declaration("{accepted: str:length(value) > 0, diagnostics: ()}");
    let capture = capture(&text);
    let source = from_capture(capture.clone());
    let mut identity = identity(&text);
    identity.uri = "file:///apps/cart/schema/validator.cem".into();
    let built = portable::export(
        &capture,
        &contract(&source),
        identity.clone(),
        &registry_hash(),
        &OperationControl::default(),
    )
    .unwrap();
    let declaration =
        Arc::new(portable::verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap());
    assert_eq!(declaration.source().source_uri(), identity.uri);
    let mut host = host();
    let scope = host
        .register_scope(
            declaration.source().clone(),
            Some(cem_ql::api::StandaloneExpressionContext {
                scope_policy: cem_ml::scheduler::ScopePolicy::host_root().with_queue_size(1),
                ..Default::default()
            }),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let binding = host
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    let call = host.query_invocation(scope).unwrap();
    let mut exhausted = false;
    for _ in 0..100 {
        let report = host
            .validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap();
        if report.accepted == None {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted);
    assert_eq!(
        host.validate_query(&call, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        None
    );
    let fresh = host.query_invocation(scope).unwrap();
    assert_eq!(
        host.validate_query(&fresh, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}

#[test]
fn retained_invocations_release_unavailable_host_native_capabilities() {
    #[derive(Debug)]
    struct Capability {
        _owner: Arc<()>,
    }
    impl cem_ql::native::NativeQueryFunction for Capability {
        fn call(&self, _: cem_ql::native::NativeQueryRequest<'_>) -> cem_ql::eval::ItemStream {
            panic!("a pure validator must never invoke this host callback");
        }
    }
    let declaration = verified(&declaration("{accepted: true, diagnostics: ()}"));
    let marker = Arc::new(());
    let weak = Arc::downgrade(&marker);
    let mut context = cem_ql::api::StandaloneExpressionContext::default();
    context
        .native_functions
        .register(
            "unavailable",
            0,
            Capability {
                _owner: marker.clone(),
            },
        )
        .unwrap();
    drop(marker);
    let mut owner = host();
    let scope = owner
        .register_scope(
            declaration.source().clone(),
            Some(context),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let binding = owner
        .link_query(
            declaration.clone(),
            scope,
            datatype(&declaration),
            &functions(),
            adapter(),
        )
        .unwrap();
    let invocation = owner.query_invocation(scope).unwrap();
    assert_eq!(
        owner
            .validate_query(&invocation, &binding, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    drop(owner);
    assert!(
        weak.upgrade().is_none(),
        "the retained invocation kept an unavailable capability alive"
    );
    drop(invocation);
}
