use super::*;
use cem_ql::selected_registry::SelectedFunctionRegistry;
use cem_ql::{
    application_envelope::v4 as package,
    structural_receiver::application::publication as install_publication,
};
thread_local! {static SELECTOR_ACTION:std::cell::RefCell<Option<Box<dyn Fn()>>>=const{std::cell::RefCell::new(None)};}
#[derive(Debug)]
struct SelectorHook(Pick);
impl cem_ql::native::NativeQueryFunction for SelectorHook {
    fn call(&self, request: cem_ql::native::NativeQueryRequest<'_>) -> ItemStream {
        SELECTOR_ACTION.with(|a| {
            if let Some(callback) = a.borrow().as_ref() {
                callback();
            }
        });
        cem_ql::native::NativeQueryFunction::call(&self.0, request)
    }
}
struct PublicationFixture {
    source_inputs: Vec<cem_ql::application_source_transport::SourceInput>,
    f: Setup,
    package: Arc<package::VerifiedApplicationEnvelope>,
    registry: Arc<install_publication::RegistryOwner>,
    raw: package::BuiltApplicationEnvelope,
    peer: mixed::MixedReceiverSource,
    peer_choice: Arc<AtomicUsize>,
    peer_marker: std::sync::Weak<()>,
}
fn fixture() -> PublicationFixture {
    fixture_rule(false)
}
fn fixture_rule(rule: bool) -> PublicationFixture {
    fixture_runtime(rule, None)
}
fn fixture_runtime(
    rule: bool,
    generation: Option<&Arc<cem_ml::runtime_admission::generation::NativeGeneration>>,
) -> PublicationFixture {
    fixture_sources(rule, generation, None)
}
fn fixture_sources(
    rule: bool,
    generation: Option<&Arc<cem_ml::runtime_admission::generation::NativeGeneration>>,
    restored: Option<&cem_ql::application_source_transport::VerifiedApplicationSources>,
) -> PublicationFixture {
    let mut package = None;
    let mut raw = None;
    let peer_choice = Arc::new(AtomicUsize::new(0));
    let marker = Arc::new(());
    let peer_marker = Arc::downgrade(&marker);
    let peer_text=RECEIVER_TEXT.replace("@type={#scalar}","@type={#native:call(\"fixture.pick\", scalar, list)}");
    let (peer_tree,peer_capture)=match restored {
        Some(s)=>{let s=s.source("file:///app/peer.cem").unwrap();(s.tree.clone(),s.capture.clone())},
        None=>imported(&peer_text,"file:///app/peer.cem"),
    };
    let source_inputs=vec![cem_ql::application_source_transport::SourceInput{identity:SourceIdentity::new("file:///app/peer.cem",SourceId(1),peer_text.as_bytes()),tree:peer_tree.clone(),capture:peer_capture.clone()}];
    let peer = mixed::MixedReceiverSource {
        schema_uri: RECEIVER_URI.into(),
        source: peer_tree.clone(),
        capture: peer_capture.clone(),
        attributes: attributes(&peer_tree),
    };
    let f = configured_application_with_sources(
        INPUT,
        RECEIVER_TEXT,
        true,
        true,
        rule,
        false,
        |built, named, control| {
            let parent = Arc::new(envelope::verify(&built.bytes, &built.pin, control).unwrap());
            let pin = parent
                .application()
                .dependency_order()
                .iter()
                .find_map(|k| match k {
                    legacy::DependencyKey::Contract(id) => {
                        parent.application().structural(id).map(|s| s.pin().clone())
                    }
                    _ => None,
                })
                .unwrap();
            let declarations=["cem-pub-one","cem-pub-two"].into_iter().map(|tag|{
   let text=format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{{attribute @name=items @type=cart-lines @maxItems=2}}{{attribute @name=count @type=cart-count}}{{attribute @name=choice @type=cart-scalar}}{{attribute @name=names @type=cart-list @maxItems=2}}{{cem-app}}{{p | {{$ | str:upper(\"tea\")}}}}</template></cem-element>");
   let d=cem_ql::declaration_artifact::build_declaration_artifact(&text,&format!("file:///app/{tag}.xhtml"),cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev,None).unwrap();
   records::build(d,parent.clone(),BTreeMap::from([("cart-lines".into(),records::ReceiverContract::Node(pin.clone())),("cart-count".into(),records::ReceiverContract::Scalar(named.clone())),("cart-scalar".into(),records::ReceiverContract::Executable{root:"scalar".into(),target:parent.roots()["scalar"].clone()}),("cart-list".into(),records::ReceiverContract::Executable{root:"list".into(),target:parent.roots()["list"].clone()})]),control).unwrap()
  }).collect();
            let b = package::build(
                built,
                declarations,
                BTreeSet::from(["cem-pub-one".into(), "cem-pub-two".into()]),
                control,
            )
            .unwrap();
            let p = restored.map(|s|s.application().clone()).unwrap_or_else(|| Arc::new(package::verify(&b.bytes, &b.pin, control).unwrap()));
            let parent = p.parent().clone();
            package = Some(p);
            raw = Some(b);
            parent
        },
        |h, g| {
            let mut ctx = StandaloneExpressionContext::default()
                .with_binding(
                    "scalar",
                    value(
                        g,
                        "app",
                        &[datatype(g.module("app").unwrap().capture(), "scalar")],
                    ),
                )
                .with_binding(
                    "list",
                    value(
                        g,
                        "app",
                        &[datatype(g.module("app").unwrap().capture(), "names")],
                    ),
                );
            ctx.native_functions
                .register(
                    "fixture.pick",
                    2,
                    SelectorHook(Pick {
                        choice: peer_choice.clone(),
                        stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                        calls: Arc::new(AtomicUsize::new(0)),
                        _marker: marker.clone(),
                    }),
                )
                .unwrap();
            let from = h
                .register_captured_scope(
                    peer_tree,
                    peer_capture.clone(),
                    Some(ctx.clone()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            h.prepare_captured_lexical_scopes(&peer_capture, ctx)
                .unwrap();
            let to = h
                .registered_source_scope(g.module("app").unwrap().source())
                .unwrap();
            h.allow_scope_crossing(from, to);
        },
        true,
        generation,
        restored,
    );
    let package = package.unwrap();
    let registry = Arc::new(
        install_publication::link_registry(&f.h, &package, package.registry_demand().registry())
            .unwrap(),
    );
    PublicationFixture {
        source_inputs,
        f,
        package,
        registry,
        raw: raw.unwrap(),
        peer,
        peer_choice,
        peer_marker,
    }
}
impl PublicationFixture {
    fn sources(&self) -> BTreeMap<String, install_publication::MixedSource> {
        let mut sources = BTreeMap::new();
        for tag in self.package.mixed_tags() {
            let receiver = if tag == "cem-pub-two" {
                &self.peer
            } else {
                &self.f.source
            };
            sources.insert(
                tag.into(),
                install_publication::MixedSource {
                    receiver: mixed::MixedReceiverSource {
                        schema_uri: receiver.schema_uri.clone(),
                        source: receiver.source.clone(),
                        capture: receiver.capture.clone(),
                        attributes: receiver.attributes.clone(),
                    },
                    scalars: BTreeMap::from([(
                        "cart-count".into(),
                        mixed::ScalarReceiverSource {
                            identity: self.f.scalar.identity.clone(),
                            source: self.f.scalar.source.clone(),
                            attributes: self.f.scalar.attributes.clone(),
                        },
                    )]),
                },
            );
        }
        sources
    }
    fn stage(&mut self) -> Result<install_publication::LinkedApplication, String> {
        let sources = self.sources();
        self.stage_sources(sources)
    }
    fn stage_sources(
        &mut self,
        sources: BTreeMap<String, install_publication::MixedSource>,
    ) -> Result<install_publication::LinkedApplication, String> {
        install_publication::stage(
            &mut self.f.h,
            self.package.clone(),
            &self.registry,
            self.f
                .base_source
                .as_ref()
                .map(|s| {
                    (
                        "cem-app".into(),
                        legacy_install::ComponentReceiverSource {
                            schema_uri: s.schema_uri.clone(),
                            source: s.source.clone(),
                            attributes: s.attributes.clone(),
                        },
                    )
                })
                .into_iter()
                .collect(),
            BTreeMap::new(),
            sources,
            &BTreeMap::from([(
                self.package.parent().roots()["scalar"].assembly.clone(),
                self.f.owner.clone(),
            )]),
        )
    }
}
#[test]
fn complete_publication_groups_original_receivers_roots_and_exact_registry_owner() {
    let mut p = fixture();
    assert!(install_publication::link_registry(
        &p.f.h,
        &p.package,
        SelectedFunctionRegistry::new([]).unwrap()
    )
    .is_err());
    let linked = p.stage().unwrap();
    let mut slot = install_publication::ApplicationSlot::default();
    slot.publish(&mut p.f.h, linked).unwrap();
    let current = slot.current(&p.f.h).unwrap();
    assert_eq!(current.envelope().component_count(), 3);
    let input = p.f.value("count");
    assert!(
        current
            .receivers(&p.f.h, "cem-pub-one")
            .unwrap()
            .validate_scalar(&mut p.f.h, "count", &input)
            .unwrap()
            .accepted
    );
    let input = p.f.value("choice");
    let invocation = current
        .receivers(&p.f.h, "cem-pub-two")
        .unwrap()
        .preparation_invocation(&mut p.f.h, "choice", input, Default::default())
        .unwrap();
    let prepared = invocation.prepare(&mut p.f.h).unwrap();
    let input = p.f.value("choice");
    assert_eq!(
        invocation
            .validate_prepared(&mut p.f.h, &input, prepared.evidence.as_ref())
            .unwrap()
            .accepted,
        Some(true)
    );
    slot.clear();
    assert!(invocation.prepare(&mut p.f.h).is_err());
    assert!(p.f.owner.is_live(&p.f.h));
}
#[test]
fn complete_publication_renews_peer_selectors_for_root_field_and_receipt_operations() {
    let mut p = fixture();
    let mut slot = install_publication::ApplicationSlot::default();
    let first = p.stage().unwrap();
    slot.publish(&mut p.f.h, first).unwrap();
    let current = slot.current(&p.f.h).unwrap().clone();
    let input = p.f.value("choice");
    let invocation = current
        .receivers(&p.f.h, "cem-pub-one")
        .unwrap()
        .preparation_invocation(&mut p.f.h, "choice", input, Default::default())
        .unwrap();
    let prepared = invocation.prepare(&mut p.f.h).unwrap();
    let staged = p.stage().unwrap();
    p.peer_choice.store(1, Ordering::SeqCst);
    assert!(slot.publish(&mut p.f.h, staged).is_err());
    let input = p.f.value("count");
    assert!(current
        .receivers(&p.f.h, "cem-pub-one")
        .unwrap()
        .validate_scalar(&mut p.f.h, "count", &input)
        .is_err());
    assert!(current
        .prepare_root(&mut p.f.h, "scalar", &lexical("x"), Default::default())
        .is_err());
    let input = p.f.value("choice");
    assert!(invocation
        .validate_prepared(&mut p.f.h, &input, prepared.evidence.as_ref())
        .is_err());
    p.peer_choice.store(0, Ordering::SeqCst);
    assert!(
        current
            .prepare_root(&mut p.f.h, "scalar", &lexical("x"), Default::default())
            .unwrap()
            .accepted
            == Some(true)
    );
    assert!(slot.current(&p.f.h).is_ok());
    let choice = p.peer_choice.clone();
    ACTION.with(|a| *a.borrow_mut() = Some(Box::new(move || choice.store(1, Ordering::SeqCst))));
    p.f.state.mode.store(7, Ordering::SeqCst);
    assert!(current
        .prepare_root(&mut p.f.h, "scalar", &lexical("late"), Default::default())
        .is_err());
    ACTION.with(|a| *a.borrow_mut() = None);
    p.f.state.mode.store(0, Ordering::SeqCst);
    p.peer_choice.store(0, Ordering::SeqCst);
    assert!(p.f.owner.is_live(&p.f.h));
}
#[test]
fn complete_publication_atomic_replacement_retires_retained_apps_and_receipts_only() {
    let mut p = fixture();
    let mut slot = install_publication::ApplicationSlot::default();
    let staged = p.stage().unwrap();
    slot.publish(&mut p.f.h, staged).unwrap();
    let prior = slot.current(&p.f.h).unwrap().clone();
    let input = p.f.value("names");
    let invocation = prior
        .receivers(&p.f.h, "cem-pub-one")
        .unwrap()
        .preparation_invocation(&mut p.f.h, "names", input, Default::default())
        .unwrap();
    let result = invocation.prepare(&mut p.f.h).unwrap();
    let mut missing = p.sources();
    missing.remove("cem-pub-two");
    assert!(p.stage_sources(missing).is_err());
    assert!(prior.is_available(&p.f.h));
    let abandoned = p.stage().unwrap();
    drop(abandoned);
    assert!(prior.is_available(&p.f.h));
    let next = p.stage().unwrap();
    slot.publish(&mut p.f.h, next).unwrap();
    assert!(!prior.is_available(&p.f.h));
    let input = p.f.value("names");
    assert!(invocation
        .validate_prepared(&mut p.f.h, &input, result.evidence.as_ref())
        .is_err());
    assert!(slot.current(&p.f.h).is_ok());
    assert!(p.f.owner.is_live(&p.f.h));
    assert!(p.f.installed.root(&p.f.h, "scalar").is_ok());
    let retained = slot.current(&p.f.h).unwrap().clone();
    drop(slot);
    assert!(!retained.is_available(&p.f.h));
    assert!(p.f.owner.is_live(&p.f.h));
}
#[test]
fn complete_publication_all_fields_and_retained_reports_observe_registry_code_and_host_loss() {
    for loss in 0..8 {
        let mut p = fixture_rule(true);
        let app = p.stage().unwrap();
        let input = p.f.value("choice");
        let invocation = app
            .receivers(&p.f.h, "cem-pub-two")
            .unwrap()
            .preparation_invocation(&mut p.f.h, "choice", input, Default::default())
            .unwrap();
        let prepared = invocation.prepare(&mut p.f.h).unwrap();
        match loss {
            0 => p.registry.retire(),
            1 => drop(p.registry),
            2 => p.f.owner.retire(),
            3 => p.f.bindings.clear(),
            4 => p.f.rules.clear(),
            5 => p.f.h.close(),
            6 => {
                p.f.control.cancel_root(None, None).unwrap();
            }
            _ => {
                let s = p.f.h.registered_source_scope(&p.peer.source).unwrap();
                assert!(p.f.h.set_context(s, Some(Default::default())));
            }
        }
        assert!(!app.is_available(&p.f.h), "loss {loss}");
        let input = p.f.value("choice");
        assert!(invocation
            .validate_prepared(&mut p.f.h, &input, prepared.evidence.as_ref())
            .is_err());
        assert!(app
            .prepare_root(
                &mut p.f.h,
                "scalar",
                &lexical("refused"),
                Default::default()
            )
            .is_err());
        let mut slot = install_publication::ApplicationSlot::default();
        assert!(slot.publish(&mut p.f.h, app).is_err());
    }
}
#[test]
fn complete_publication_rejects_fresh_metadata_wrong_registry_and_missing_code_owners() {
    let mut p = fixture();
    let first = p.stage().unwrap();
    let mut slot = install_publication::ApplicationSlot::default();
    slot.publish(&mut p.f.h, first).unwrap();
    let mut entries = p
        .package
        .registry_demand()
        .registry()
        .entries()
        .collect::<Vec<_>>();
    entries.push(cem_ql::selected_registry::SelectedBuiltin::StringLower);
    assert!(install_publication::link_registry(
        &p.f.h,
        &p.package,
        SelectedFunctionRegistry::new(entries).unwrap()
    )
    .is_err());
    let mut extra = p.sources();
    let duplicate = extra.remove("cem-pub-two").unwrap();
    extra.insert("cem-unknown".into(), duplicate);
    assert!(p.stage_sources(extra).is_err());
    let fresh = Arc::new(package::verify(&p.raw.bytes, &p.raw.pin, &Default::default()).unwrap());
    let old_package = std::mem::replace(&mut p.package, fresh);
    assert!(
        p.stage().is_err(),
        "old registry cannot authorize a new snapshot"
    );
    p.registry = Arc::new(
        install_publication::link_registry(
            &p.f.h,
            &p.package,
            p.package.registry_demand().registry(),
        )
        .unwrap(),
    );
    assert!(
        p.stage().is_err(),
        "equal pins cannot substitute restored compilation arenas"
    );
    p.package = old_package;
    assert!(
        slot.current(&p.f.h).is_err(),
        "dropping the caller's original registry expires the prior app"
    );
    assert!(p.f.owner.is_live(&p.f.h));
}
#[test]
fn complete_publication_refuses_busy_access_and_withholds_callback_time_retirement() {
    for operation in ["clear", "registry", "prepare", "convert", "serialize"] {
        let mut p = fixture();
        let mut slot = install_publication::ApplicationSlot::default();
        let staged = p.stage().unwrap();
        slot.publish(&mut p.f.h, staged).unwrap();
        let app = slot.current(&p.f.h).unwrap().clone();
        let slot = std::rc::Rc::new(std::cell::RefCell::new(slot));
        let live = slot.clone();
        assert_eq!(
            app.with_registry(&mut p.f.h, |registry, host| {
                assert_eq!(registry.entries().count(), 1);
                assert!(!app.is_available(host));
                assert!(app.receivers(host, "cem-pub-two").is_err());
                assert!(live.borrow().current(host).is_err());
                42
            })
            .unwrap(),
            42
        );
        let registry = p.registry.clone();
        let clear = slot.clone();
        ACTION.with(|a| {
            *a.borrow_mut() = Some(Box::new(move || {
                if operation == "registry" {
                    registry.retire()
                } else {
                    clear.borrow_mut().clear()
                }
            }))
        });
        p.f.state.mode.store(8, Ordering::SeqCst);
        let refused = match operation {
            "convert" => app
                .convert_root(
                    &mut p.f.h,
                    "scalar",
                    &conversion("late"),
                    Default::default(),
                )
                .is_err(),
            "serialize" => app
                .serialize_root(&mut p.f.h, "list", &list_input(&["x"]), Default::default())
                .is_err(),
            _ => app
                .prepare_root(&mut p.f.h, "scalar", &lexical("late"), Default::default())
                .is_err(),
        };
        ACTION.with(|a| *a.borrow_mut() = None);
        p.f.state.mode.store(0, Ordering::SeqCst);
        assert!(refused, "{operation}");
        assert!(!app.is_available(&p.f.h));
        assert!(p.f.owner.is_live(&p.f.h));
    }
}
#[test]
fn complete_publication_keeps_no_compilation_registry_or_host_callbacks_alive() {
    let mut p = fixture();
    let app = p.stage().unwrap();
    let input = p.f.value("choice");
    let invocation = app
        .receivers(&p.f.h, "cem-pub-one")
        .unwrap()
        .preparation_invocation(&mut p.f.h, "choice", input, Default::default())
        .unwrap();
    let owner = Arc::downgrade(&p.f.owner);
    drop(p.f.owner);
    assert!(owner.upgrade().is_none());
    assert!(!app.is_available(&p.f.h));
    assert!(invocation.prepare(&mut p.f.h).is_err());
    let mut p = fixture();
    let app = p.stage().unwrap();
    let registry = Arc::downgrade(&p.registry);
    drop(p.registry);
    assert!(registry.upgrade().is_none());
    assert!(!app.is_available(&p.f.h));
    let mut p = fixture();
    let app = p.stage().unwrap();
    let callbacks = Arc::downgrade(&p.f.state);
    drop(p.f.state);
    drop(p.f.h);
    assert!(callbacks.upgrade().is_none());
    assert!(p.peer_marker.upgrade().is_none());
    let foreign = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    assert!(!app.is_available(&foreign));
}
#[test]
fn complete_publication_does_not_publish_registry_callback_output_after_last_caller_owner_drops() {
    let mut p = fixture();
    let app = p.stage().unwrap();
    let owner = std::cell::RefCell::new(Some(p.registry));
    assert!(app
        .with_registry(&mut p.f.h, |_, _| {
            owner.borrow_mut().take();
            "late"
        })
        .is_err());
    assert!(!app.is_available(&p.f.h));
    assert!(p.f.owner.is_live(&p.f.h));
}

#[test]
fn complete_publication_validates_parent_structural_input_under_the_common_guard() {
    let mut p = fixture();
    let app = p.stage().unwrap();
    let input = p.f.input.clone();
    let report = app
        .validate_base_input(&mut p.f.h, "cem-app", input.clone())
        .unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    p.registry.retire();
    assert!(app
        .validate_base_input(&mut p.f.h, "cem-app", input)
        .is_err());
    assert!(p.f.owner.is_live(&p.f.h));
}

#[path = "application_generation.rs"]
mod generation;

#[path = "application_source_handoff.rs"]
mod source_handoff;
