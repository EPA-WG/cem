//! Native CEMAPPD3 installation uses the ordinary source/profile/operation fixture.
use super::*;
use cem_ql::{
    application_envelope::{self as legacy, v3 as envelope},
    structural_receiver::application::{self as legacy_install, v3 as install},
};
fn application(
    rule: bool,
    setup: impl FnOnce(&mut FreshStructuralHost),
    base: legacy::BuiltApplicationEnvelope,
) -> (Fixture, Arc<envelope::VerifiedApplicationEnvelope>) {
    let mut loaded = None;
    let f = fixture_custom(
        rule,
        |bytes, leaves, profiles| {
            let control = OperationControl::default();
            let graph =
                assembly::verify(&bytes.bytes, &bytes.pin, &leaves, &profiles, &control).unwrap();
            let app = graph.module("app").unwrap();
            let scalar = datatype(app.capture(), "scalar");
            let list = datatype(app.capture(), "names");
            let mut b = envelope::EnvelopeBuilder::new(base).unwrap();
            let id = b.add_assembly(bytes, leaves, profiles).unwrap();
            let b = b
                .build(
                    BTreeMap::from([
                        (
                            "scalar".into(),
                            envelope::AssemblyRoot {
                                assembly: id.clone(),
                                datatype: scalar,
                            },
                        ),
                        (
                            "list".into(),
                            envelope::AssemblyRoot {
                                assembly: id.clone(),
                                datatype: list,
                            },
                        ),
                    ]),
                    &control,
                )
                .unwrap();
            let envelope = Arc::new(envelope::verify(&b.bytes, &b.pin, &control).unwrap());
            let graph = envelope.assembly_owner(&id).unwrap().clone();
            loaded = Some(envelope);
            graph
        },
        setup,
    );
    (f, loaded.unwrap())
}
fn base() -> legacy::BuiltApplicationEnvelope {
    let declaration = cem_ql::declaration_artifact::build_declaration_artifact(
        "<cem-element tag=\"cem-app\"><template id=\"cem-app\" type=\"text/cem-ml\">{p | Hello}</template></cem-element>",
        "file:///app/main.xhtml", cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev, None,
    ).unwrap();
    let mut b = legacy::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    b.add_component(declaration, BTreeMap::new()).unwrap();
    b.build(
        BTreeSet::from(["cem-app".into()]),
        BTreeSet::new(),
        &OperationControl::default(),
    )
    .unwrap()
}
#[test]
fn application_installation_binds_exact_roots_and_delegates_native_operations() {
    let (f, envelope) = application(false, |_| {}, base());
    let compilation = Arc::new(f.c);
    let owners = BTreeMap::from([(f.g.pin().hash.clone(), compilation.clone())]);
    let before = f.state.calls.lock().unwrap().clone();
    let staged = install::stage(&f.h, envelope, BTreeMap::new(), BTreeMap::new(), &owners).unwrap();
    let mut slot = install::ApplicationSlot::default();
    slot.publish(&f.h, staged).unwrap();
    assert_eq!(
        *f.state.calls.lock().unwrap(),
        before,
        "staging/publication invoked a native callback"
    );
    let current = slot.current(&f.h).unwrap();
    assert_eq!(current.root_count(), 2);
    let scalar = current.root(&f.h, "scalar").unwrap();
    assert_eq!(
        scalar
            .prepare_lexical(&f.h, &lexical("hello"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(
        scalar
            .convert(&f.h, &conversion("hello"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    assert_eq!(
        scalar
            .validate(&f.h, &input("hello"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let list = current.root(&f.h, "list").unwrap();
    assert_eq!(
        list.serialize_list(&f.h, &list_input(&["one", "two"]), Default::default())
            .unwrap()
            .text
            .as_deref(),
        Some("one|two")
    );
    assert!(current.root(&f.h, "unpublished").is_err());
    assert!(compilation.is_live(&f.h));
}
fn candidate(
    host: &FreshStructuralHost,
    envelope: &Arc<envelope::VerifiedApplicationEnvelope>,
    owner: &Arc<FreshDependencyDatatypeCompilation>,
) -> Result<install::LinkedApplication, String> {
    let id = envelope.roots()["scalar"].assembly.clone();
    install::stage(
        host,
        envelope.clone(),
        BTreeMap::new(),
        BTreeMap::new(),
        &BTreeMap::from([(id, owner.clone())]),
    )
}
fn recompile(f: &mut Fixture) -> Arc<FreshDependencyDatatypeCompilation> {
    Arc::new(
        f.h.compile_datatype_assembly(
            f.g.clone(),
            f.root,
            &f.scopes,
            &[],
            &f.implementations,
            &f.rules,
            &f.bindings,
            limits(),
        )
        .unwrap(),
    )
}
#[test]
fn application_replacement_and_clear_revoke_only_their_own_installed_roots() {
    let (f, envelope) = application(false, |_| {}, base());
    let owner = Arc::new(f.c);
    let mut slot = install::ApplicationSlot::default();
    slot.publish(&f.h, candidate(&f.h, &envelope, &owner).unwrap())
        .unwrap();
    let scalar = slot
        .current(&f.h)
        .unwrap()
        .root(&f.h, "scalar")
        .unwrap()
        .clone();
    let list = slot
        .current(&f.h)
        .unwrap()
        .root(&f.h, "list")
        .unwrap()
        .clone();
    slot.publish(&f.h, candidate(&f.h, &envelope, &owner).unwrap())
        .unwrap();
    assert!(!scalar.is_available(&f.h));
    assert!(!list.is_available(&f.h));
    assert!(owner.is_live(&f.h));
    let fresh = slot
        .current(&f.h)
        .unwrap()
        .root(&f.h, "scalar")
        .unwrap()
        .clone();
    assert!(fresh.is_available(&f.h));
    slot.clear();
    assert!(!fresh.is_available(&f.h));
    assert!(owner.is_live(&f.h));
    assert_eq!(
        owner
            .validate(&f.h, f.scalar, &input("ok"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    let calls = f.state.calls.lock().unwrap().len();
    assert!(fresh
        .prepare_lexical(&f.h, &lexical("refused"), Default::default())
        .is_err());
    assert_eq!(calls, f.state.calls.lock().unwrap().len());
}
#[test]
fn application_refused_staging_and_publication_preserve_prior_bindings_and_reusable_owners() {
    let (mut f, envelope) = application(false, |_| {}, base());
    let stale = recompile(&mut f);
    let owner = Arc::new(f.c);
    let mut slot = install::ApplicationSlot::default();
    slot.publish(&f.h, candidate(&f.h, &envelope, &owner).unwrap())
        .unwrap();
    let prior = slot
        .current(&f.h)
        .unwrap()
        .root(&f.h, "scalar")
        .unwrap()
        .clone();
    assert!(install::stage(
        &f.h,
        envelope.clone(),
        BTreeMap::new(),
        BTreeMap::new(),
        &BTreeMap::new()
    )
    .is_err());
    let id = f.g.pin().hash.clone();
    let extra = BTreeMap::from([(id.clone(), owner.clone()), (hash(b"extra"), owner.clone())]);
    assert!(install::stage(
        &f.h,
        envelope.clone(),
        BTreeMap::new(),
        BTreeMap::new(),
        &extra
    )
    .is_err());
    let staged = candidate(&f.h, &envelope, &stale).unwrap();
    let refused = staged.root(&f.h, "list").unwrap().clone();
    stale.retire();
    assert!(slot.publish(&f.h, staged).is_err());
    assert!(!refused.is_available(&f.h));
    assert!(candidate(&f.h, &envelope, &stale).is_err());
    assert!(prior.is_available(&f.h));
    assert!(owner.is_live(&f.h));
    assert!(slot.current(&f.h).is_ok());
    // Abandoning a valid staged candidate revokes only its own wrappers.
    let abandoned = candidate(&f.h, &envelope, &owner).unwrap();
    let abandoned_root = abandoned.root(&f.h, "scalar").unwrap().clone();
    drop(abandoned);
    assert!(!abandoned_root.is_available(&f.h));
    assert!(prior.is_available(&f.h));
}
#[test]
fn application_pin_equality_cannot_substitute_independently_restored_or_foreign_compilations() {
    let (first, a) = application(false, |_| {}, base());
    let (second, b) = application(false, |_| {}, base());
    assert_eq!(a.pin().hash, b.pin().hash);
    let first_owner = Arc::new(first.c);
    let second_owner = Arc::new(second.c);
    let mut slot = install::ApplicationSlot::default();
    slot.publish(&second.h, candidate(&second.h, &b, &second_owner).unwrap())
        .unwrap();
    // The compilation is live on this host, but belongs to a different restored
    // graph arena with identical pins. Metadata cannot authorize substitution.
    assert!(second_owner.is_live(&second.h));
    assert!(candidate(&second.h, &a, &second_owner).is_err());
    assert!(candidate(&first.h, &b, &second_owner).is_err());
    assert!(candidate(&second.h, &a, &first_owner).is_err());
    assert!(slot.current(&second.h).is_ok());
}
#[test]
fn application_installations_do_not_keep_compilation_or_operation_owners_alive() {
    for dropped in ["compilation", "operations", "rules"] {
        let (f, envelope) = application(true, |_| {}, base());
        let owner = Arc::new(f.c);
        let mut slot = install::ApplicationSlot::default();
        slot.publish(&f.h, candidate(&f.h, &envelope, &owner).unwrap())
            .unwrap();
        let root = slot
            .current(&f.h)
            .unwrap()
            .root(&f.h, "scalar")
            .unwrap()
            .clone();
        if dropped == "compilation" {
            let weak = Arc::downgrade(&owner);
            drop(owner);
            assert!(weak.upgrade().is_none());
        } else {
            if dropped == "operations" {
                drop(f.bindings);
            } else {
                drop(f.rules);
            }
            assert!(!owner.is_live(&f.h));
        }
        assert!(!root.is_available(&f.h));
        assert!(slot.current(&f.h).is_err());
    }
    let (f, envelope) = application(false, |_| {}, base());
    let owner = Arc::new(f.c);
    let installed = candidate(&f.h, &envelope, &owner).unwrap();
    let root = installed.root(&f.h, "scalar").unwrap().clone();
    let callbacks = Arc::downgrade(&f.state);
    drop(f.state);
    drop(f.h);
    assert!(
        callbacks.upgrade().is_none(),
        "installation retained a host callback"
    );
    let foreign = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    assert!(!root.is_available(&foreign));
}
#[test]
fn application_publication_rechecks_host_epoch_closure_and_cancellation() {
    for case in 0..3 {
        let (mut f, envelope) = application(false, |_| {}, base());
        let owner = Arc::new(f.c);
        let staged = candidate(&f.h, &envelope, &owner).unwrap();
        let root = staged.root(&f.h, "scalar").unwrap().clone();
        match case {
            0 => {
                assert!(f.h.set_context(f.root, Some(Default::default())));
            }
            1 => f.h.close(),
            _ => {
                f.control.cancel_root(None, None).unwrap();
            }
        }
        let mut slot = install::ApplicationSlot::default();
        assert!(slot.publish(&f.h, staged).is_err());
        assert!(!root.is_available(&f.h));
        assert!(slot.current(&f.h).is_err());
    }
}
#[test]
fn application_clear_during_callbacks_withholds_late_root_reports_without_retiring_compilations() {
    for operation in ["prepare", "convert", "serialize"] {
        let (f, envelope) = application(false, |_| {}, base());
        let owner = Arc::new(f.c);
        let mut slot = install::ApplicationSlot::default();
        slot.publish(&f.h, candidate(&f.h, &envelope, &owner).unwrap())
            .unwrap();
        let scalar = slot
            .current(&f.h)
            .unwrap()
            .root(&f.h, "scalar")
            .unwrap()
            .clone();
        let list = slot
            .current(&f.h)
            .unwrap()
            .root(&f.h, "list")
            .unwrap()
            .clone();
        let slot = std::rc::Rc::new(std::cell::RefCell::new(slot));
        let captured = slot.clone();
        ACTION.with(|a| *a.borrow_mut() = Some(Box::new(move || captured.borrow_mut().clear())));
        f.state.mode.store(8, Ordering::SeqCst);
        let refused = match operation {
            "prepare" => scalar
                .prepare_lexical(&f.h, &lexical("x"), Default::default())
                .is_err(),
            "convert" => scalar
                .convert(&f.h, &conversion("x"), Default::default())
                .is_err(),
            _ => list
                .serialize_list(&f.h, &list_input(&["x"]), Default::default())
                .is_err(),
        };
        ACTION.with(|a| *a.borrow_mut() = None);
        f.state.mode.store(0, Ordering::SeqCst);
        assert!(refused, "late {operation} report escaped");
        assert!(!scalar.is_available(&f.h));
        assert!(!list.is_available(&f.h));
        assert!(owner.is_live(&f.h));
    }
}
#[test]
fn application_root_reentry_and_busy_publication_refuse_without_disturbing_current_installation() {
    let (f, envelope) = application(false, |_| {}, base());
    let owner = Arc::new(f.c);
    let mut slot = install::ApplicationSlot::default();
    slot.publish(&f.h, candidate(&f.h, &envelope, &owner).unwrap())
        .unwrap();
    let scalar = slot
        .current(&f.h)
        .unwrap()
        .root(&f.h, "scalar")
        .unwrap()
        .clone();
    let list = slot
        .current(&f.h)
        .unwrap()
        .root(&f.h, "list")
        .unwrap()
        .clone();
    let staged = candidate(&f.h, &envelope, &owner).unwrap();
    let staged_root = staged.root(&f.h, "scalar").unwrap().clone();
    let slot = std::rc::Rc::new(std::cell::RefCell::new(slot));
    let host = std::rc::Rc::new(f.h);
    let attempted = Arc::new(AtomicUsize::new(0));
    let captured = (
        scalar.clone(),
        list,
        host.clone(),
        slot.clone(),
        std::cell::RefCell::new(Some(staged)),
        attempted.clone(),
    );
    ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            let (scalar, list, host, slot, staged, attempted) = &captured;
            assert!(scalar
                .prepare_lexical(host, &lexical("nested"), Default::default())
                .is_err());
            assert!(scalar
                .convert(host, &conversion("nested"), Default::default())
                .is_err());
            assert!(scalar
                .validate(host, &input("nested"), Default::default())
                .is_err());
            assert!(list
                .serialize_list(host, &list_input(&["nested"]), Default::default())
                .is_err());
            if let Some(staged) = staged.borrow_mut().take() {
                assert!(slot.borrow_mut().publish(host, staged).is_err());
            }
            attempted.fetch_add(1, Ordering::SeqCst);
        }))
    });
    f.state.mode.store(7, Ordering::SeqCst);
    let report = scalar
        .prepare_lexical(&host, &lexical("outer"), Default::default())
        .unwrap();
    ACTION.with(|a| *a.borrow_mut() = None);
    f.state.mode.store(0, Ordering::SeqCst);
    assert_eq!(report.accepted, Some(true));
    assert!(attempted.load(Ordering::SeqCst) > 0);
    assert!(!staged_root.is_available(&host));
    assert!(slot.borrow().current(&host).is_ok());
    assert!(scalar.is_available(&host));
    assert!(owner.is_live(&host));
}
fn structural_base() -> (
    legacy::BuiltApplicationEnvelope,
    legacy_install::ComponentReceiverSource,
) {
    use cem_ml::schema::structural_capsule;
    let text = include_str!("../../../cem_ml/fixtures/structural-cart.cem");
    let imported = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "file:///app/cart.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let contract = structural_capsule::export(
        &imported.captured,
        SourceIdentity::new("file:///app/cart.cem", SourceId(1), text.as_bytes()),
        "https://app.test/schema/cart/1",
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let mut builder = legacy::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let id = builder
        .add_contract(
            legacy::ContractArtifact::structural(contract),
            BTreeSet::new(),
        )
        .unwrap();
    let declaration = cem_ql::declaration_artifact::build_declaration_artifact(
        "<cem-element tag=\"cem-app\"><template id=\"cem-app\" type=\"text/cem-ml\">{attribute @name=items @type=cart-lines @maxItems=2}{p | Hello}</template></cem-element>",
        "file:///app/main.xhtml", cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev, None,
    ).unwrap();
    builder
        .add_component(
            declaration,
            BTreeMap::from([(
                "cart-lines".into(),
                legacy::NodeReceiverRequirement {
                    contract: id,
                    schema_uri: "https://app.test/schema/cart/1".into(),
                },
            )]),
        )
        .unwrap();
    let base = builder
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::new(),
            &OperationControl::default(),
        )
        .unwrap();
    let text = "{schema @name=receiver @namespace=https://app.test/receiver | {elements | {element @name=view @required-attributes=items}} {attributes | {attribute @name=items @type=node @maxItems=2}}}";
    let tree = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "file:///app/receiver.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
    .tree;
    let id = tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "attribute" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    (
        base,
        legacy_install::ComponentReceiverSource {
            schema_uri: "https://app.test/receiver".into(),
            source: tree,
            attributes: BTreeMap::from([("items".into(), id)]),
        },
    )
}
#[test]
fn application_structural_models_and_datatype_roots_replace_together_without_mixed_receiver_permissions(
) {
    let (base, source) = structural_base();
    let retained = |text: &str, uri: &str| {
        cem_ml::import::import_bytes_with_lexical_scopes(
            text.as_bytes(),
            "text/cem-ml",
            uri,
            CompiledSchema::cem_core(),
        )
        .unwrap()
        .tree
    };
    let input = retained("{view @items={#data}}", "file:///app/input.cem");
    let data = retained(
        "{cart @title=Tea | {item @quantity=3}}",
        "file:///app/data.cem",
    );
    let id = data
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let value = RetainedCemNode::new(data.clone(), id).unwrap().query_item();
    let setup = source.source.clone();
    let (mut f, envelope) = application(
        false,
        |h| {
            h.register_scope(
                setup,
                None,
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
            let from = h
                .register_scope(
                    input.clone(),
                    Some(StandaloneExpressionContext::default().with_binding(
                        "data",
                        StandaloneExpressionBinding::any(ItemStream::once(value)),
                    )),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            let to = h
                .register_scope(data, None, ReferenceScopePolicy::schema_defaults().unwrap())
                .unwrap();
            assert!(h.allow_scope_crossing(from, to));
        },
        base,
    );
    let owner = Arc::new(f.c);
    let sources = || {
        BTreeMap::from([(
            "cem-app".into(),
            legacy_install::ComponentReceiverSource {
                schema_uri: source.schema_uri.clone(),
                source: source.source.clone(),
                attributes: source.attributes.clone(),
            },
        )])
    };
    let owners = BTreeMap::from([(f.g.pin().hash.clone(), owner.clone())]);
    let staged =
        install::stage(&f.h, envelope.clone(), sources(), BTreeMap::new(), &owners).unwrap();
    let mut slot = install::ApplicationSlot::default();
    slot.publish(&f.h, staged).unwrap();
    let old = slot.current(&f.h).unwrap();
    let root = old.root(&f.h, "scalar").unwrap().clone();
    let model = old
        .application(&f.h)
        .unwrap()
        .model(&f.h, "cem-app")
        .unwrap()
        .clone();
    assert_eq!(model.attribute_datatypes.len(), 1);
    assert!(old
        .application(&f.h)
        .unwrap()
        .envelope()
        .receiver("cem-app", "scalar")
        .is_none());
    assert!(f.h.validate_input(input.clone(), &model).unwrap().complete);
    assert!(root.is_available(&f.h));
    assert!(install::stage(
        &f.h,
        envelope.clone(),
        BTreeMap::new(),
        BTreeMap::new(),
        &owners
    )
    .is_err());
    assert!(root.is_available(&f.h));
    slot.publish(
        &f.h,
        install::stage(&f.h, envelope, sources(), BTreeMap::new(), &owners).unwrap(),
    )
    .unwrap();
    assert!(!root.is_available(&f.h));
    assert!(slot
        .current(&f.h)
        .unwrap()
        .application(&f.h)
        .unwrap()
        .model(&f.h, "cem-app")
        .is_ok());
    assert!(!f.h.validate_input(input.clone(), &model).unwrap().complete);
    let current = slot
        .current(&f.h)
        .unwrap()
        .application(&f.h)
        .unwrap()
        .model(&f.h, "cem-app")
        .unwrap()
        .clone();
    assert!(
        f.h.validate_input(input.clone(), &current)
            .unwrap()
            .complete
    );
    slot.clear();
    assert!(!f.h.validate_input(input, &current).unwrap().complete);
    assert!(owner.is_live(&f.h));
}
#[test]
fn application_retained_roots_observe_peer_ownership_across_every_assembly() {
    for loss in ["retire-owner", "drop-owner", "retire-installation"] {
        let control = OperationControl::default();
        let mut builder = envelope::EnvelopeBuilder::new(base()).unwrap();
        let mut roots = BTreeMap::new();
        for name in ["first", "second"] {
            let module = wrapped(&format!("file:///app/{name}.cem"), &format!("@ns s = https://cem.dev/ns/schema/1\n@default s\n{{schema @namespace=urn:{name} | {{types | {{type @name=value @kind=scalar}}}}}}"), Some("value"));
            let root = *module.types.roots.first().unwrap();
            let b = assembly::export(
                "app",
                &BTreeMap::from([("app".into(), module)]),
                &[],
                &[],
                &hash(b"registry"),
                &control,
            )
            .unwrap();
            let id = builder.add_assembly(b, vec![], vec![]).unwrap();
            roots.insert(
                name.into(),
                envelope::AssemblyRoot {
                    assembly: id,
                    datatype: root,
                },
            );
        }
        let b = builder.build(roots, &control).unwrap();
        let envelope = Arc::new(envelope::verify(&b.bytes, &b.pin, &control).unwrap());
        let mut host = FreshStructuralHost::new(&hash(b"registry"), control).unwrap();
        let mut scopes = BTreeMap::new();
        for id in envelope.assembly_ids() {
            scopes.insert(
                id.to_string(),
                host.register_datatype_assembly_scope(
                    envelope.assembly(id).unwrap(),
                    "app",
                    Some(Default::default()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap(),
            );
        }
        let mut owners = BTreeMap::new();
        for id in envelope.assembly_ids() {
            let graph = envelope.assembly_owner(id).unwrap();
            let source = graph
                .module("app")
                .unwrap()
                .sources()
                .values()
                .next()
                .unwrap();
            let implementation = DatatypeImplementation {
                source: source.clone(),
                kind: DatatypeKind::Scalar,
                representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
                accepted_bases: vec![],
                bounds: Default::default(),
                tokenizer: TokenizerBinding::Absent,
                validator: None,
            };
            let owner = host
                .compile_datatype_assembly(
                    graph.clone(),
                    scopes[id],
                    &BTreeMap::new(),
                    &[],
                    &[implementation],
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    limits(),
                )
                .unwrap();
            owners.insert(id.to_string(), Arc::new(owner));
        }
        let mut slot = install::ApplicationSlot::default();
        slot.publish(
            &host,
            install::stage(
                &host,
                envelope.clone(),
                BTreeMap::new(),
                BTreeMap::new(),
                &owners,
            )
            .unwrap(),
        )
        .unwrap();
        let first = slot
            .current(&host)
            .unwrap()
            .root(&host, "first")
            .unwrap()
            .clone();
        let second = slot
            .current(&host)
            .unwrap()
            .root(&host, "second")
            .unwrap()
            .clone();
        let id = &envelope.roots()["second"].assembly;
        match loss {
            "retire-owner" => owners[id].retire(),
            "drop-owner" => {
                let weak = Arc::downgrade(&owners[id]);
                owners.remove(id);
                assert!(weak.upgrade().is_none());
            }
            _ => second.retire(),
        }
        assert!(!second.is_available(&host));
        assert!(!first.is_available(&host));
        assert!(first
            .validate(&host, &input("refused"), Default::default())
            .is_err());
        assert!(slot.current(&host).is_err());
        assert!(owners[&envelope.roots()["first"].assembly].is_live(&host));
    }
}
