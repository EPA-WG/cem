use super::*;
use cem_ml::{
    parser::tree::RetainedCemTree,
    schema::{document_model::AttributeValueContract, machine::LexicallyScopedDocument},
};
use cem_ql::{application_envelope::mixed as records, structural_receiver::application::mixed};
const RECEIVER_URI: &str = "https://app.test/receiver/1";
const SCALAR_URI: &str = "https://app.test/scalars/1";
const SCALAR_TEXT:&str="{schema @name=scalars @namespace=https://app.test/scalars/1 | {attributes | {attribute @name=cart-count @type=integer @minInclusive=2 @maxInclusive=5}}}";
const RECEIVER_TEXT:&str="{schema @name=receiver @namespace=https://app.test/receiver/1 | {elements | {element @name=view @optional-attributes=\"items count choice names\"}} {attributes | {attribute @name=items @type=node @maxItems=2} {attribute @name=count @type=integer @minInclusive=3 @maxInclusive=4} {attribute @name=choice @type={#scalar}} {attribute @name=names @type={#list} @maxItems=2}}}";
fn imported(text: &str, uri: &str) -> (Arc<RetainedCemTree>, Arc<LexicallyScopedDocument>) {
    let i = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        uri,
        CompiledSchema::cem_core(),
    )
    .unwrap();
    (i.tree, i.captured)
}
fn attributes(tree: &RetainedCemTree) -> BTreeMap<String, u32> {
    tree.ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "attribute" => {
                cem_ml::schema::document_model::compile_attribute_model(tree.ast(), *node_id)
                    .map(|a| (a.name, *node_id))
            }
            _ => None,
        })
        .collect()
}
fn data_attributes(tree: &RetainedCemTree) -> BTreeMap<String, u32> {
    tree.ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } => Some((expanded_name.local_name.clone(), *node_id)),
            _ => None,
        })
        .collect()
}
struct Setup {
    h: FreshStructuralHost,
    owner: Arc<FreshDependencyDatatypeCompilation>,
    bindings: BTreeMap<DatatypeAssemblyOperationKey, DatatypeAssemblyOperationBinding>,
    rules: BTreeMap<String, Arc<FreshDatatypeRuleBinding>>,
    state: Arc<State>,
    choice: Arc<AtomicUsize>,
    selector_calls: Arc<AtomicUsize>,
    control: OperationControl,
    installed: Arc<install::LinkedApplication>,
    contract: Arc<records::VerifiedMixedReceiverDeclaration>,
    source: mixed::MixedReceiverSource,
    scalar: mixed::ScalarReceiverSource,
    input: Arc<RetainedCemTree>,
    ids: BTreeMap<String, u32>,
}
impl Setup {
    fn value(&self, name: &str) -> mixed::ReceiverInput {
        mixed::ReceiverInput {
            source: self.input.clone(),
            attribute: self.ids[name],
        }
    }
    fn bind(&mut self) -> Result<mixed::LinkedMixedReceivers, String> {
        let roots = BTreeMap::from([
            (
                "scalar".into(),
                self.installed.root(&self.h, "scalar")?.clone(),
            ),
            ("list".into(), self.installed.root(&self.h, "list")?.clone()),
        ]);
        mixed::stage(
            &mut self.h,
            self.contract.clone(),
            mixed::MixedReceiverSource {
                schema_uri: self.source.schema_uri.clone(),
                source: self.source.source.clone(),
                capture: self.source.capture.clone(),
                attributes: self.source.attributes.clone(),
            },
            BTreeMap::from([(
                "cart-count".into(),
                mixed::ScalarReceiverSource {
                    identity: self.scalar.identity.clone(),
                    source: self.scalar.source.clone(),
                    attributes: self.scalar.attributes.clone(),
                },
            )]),
            roots,
        )
    }
}
fn setup(input_text: &str, grant: bool) -> Setup {
    configured(input_text, RECEIVER_TEXT, grant, grant, false)
}
fn configured(
    input_text: &str,
    receiver_text: &str,
    receiver_grant: bool,
    input_grant: bool,
    rule: bool,
) -> Setup {
    configured_profile(
        input_text,
        receiver_text,
        receiver_grant,
        input_grant,
        rule,
        false,
    )
}
fn configured_profile(
    input_text: &str,
    receiver_text: &str,
    receiver_grant: bool,
    input_grant: bool,
    rule: bool,
    typed: bool,
) -> Setup {
    let choice = Arc::new(AtomicUsize::new(0));
    let selector_calls = Arc::new(AtomicUsize::new(0));
    let control = OperationControl::default();
    let (scalar_tree, scalar_capture) = imported(SCALAR_TEXT, "file:///app/scalars.cem");
    let scalar_attrs = attributes(&scalar_tree);
    let model = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
        SCALAR_URI,
        &scalar_tree,
    );
    let named = cem_ql::application_declaration::NamedReceiverType {
        name: "cart-count".into(),
        schema_uri: SCALAR_URI.into(),
        source_uri: "file:///app/scalars.cem".into(),
        source_hash: hash(SCALAR_TEXT.as_bytes()),
        value: AttributeValueContract {
            model: model.attributes["cart-count"].clone(),
            ..Default::default()
        },
    };
    let (receiver, capture) = imported(receiver_text, "file:///app/receiver.cem");
    let attrs = attributes(&receiver);
    let (input, input_capture) = imported(input_text, "file:///app/input.cem");
    let ids = data_attributes(&input);
    let (data, data_capture) = imported(
        "{cart @title=Tea | {item @quantity=3}}",
        "file:///app/data.cem",
    );
    let data_id = node(&data_capture, "cart", 0);
    let mut b = legacy::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let cart_text = include_str!("../../../cem_ml/fixtures/structural-cart.cem");
    let (cart, cart_capture) = imported(cart_text, "file:///app/cart.cem");
    let _ = cart;
    let sc = cem_ml::schema::structural_capsule::export(
        &cart_capture,
        SourceIdentity::new("file:///app/cart.cem", SourceId(1), cart_text.as_bytes()),
        "https://app.test/schema/cart/1",
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let node_pin = sc.pin.clone();
    let id = b
        .add_contract(legacy::ContractArtifact::structural(sc), BTreeSet::new())
        .unwrap();
    let decl=cem_ql::declaration_artifact::build_declaration_artifact("<cem-element tag=\"cem-app\"><template id=\"cem-app\" type=\"text/cem-ml\">{p | Hello}</template></cem-element>","file:///app/base.xhtml",cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev,None).unwrap();
    b.add_component(decl, BTreeMap::new()).unwrap();
    let base = b
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::from([id]),
            &control,
        )
        .unwrap();
    let metadata = std::cell::RefCell::new(None);
    let mut parent = None;
    let f = fixture_custom_profile(
        rule,
        typed,
        |bytes, leaves, profiles| {
            let old =
                assembly::verify(&bytes.bytes, &bytes.pin, &leaves, &profiles, &control).unwrap();
            let root = old.module("app").unwrap();
            let scalar = datatype(root.capture(), "scalar");
            let list = datatype(root.capture(), "names");
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
            let p = Arc::new(envelope::verify(&b.bytes, &b.pin, &control).unwrap());
            let g = p.assembly_owner(&id).unwrap().clone();
            *metadata.borrow_mut() = Some(g.clone());
            parent = Some(p);
            g
        },
        |h| {
            let g = metadata.borrow();
            let g = g.as_ref().unwrap();
            h.register_captured_scope(
                scalar_tree.clone(),
                scalar_capture,
                Some(Default::default()),
                ReferenceScopePolicy::schema_defaults().unwrap(),
            )
            .unwrap();
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
                    Pick {
                        choice: choice.clone(),
                        stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                        calls: selector_calls.clone(),
                        _marker: Arc::new(()),
                    },
                )
                .unwrap();
            let from = h
                .register_captured_scope(
                    receiver.clone(),
                    capture.clone(),
                    Some(ctx.clone()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            h.prepare_captured_lexical_scopes(&capture, ctx).unwrap();
            let target = h
                .registered_source_scope(g.module("app").unwrap().source())
                .unwrap();
            if receiver_grant {
                h.allow_scope_crossing(from, target);
            }
            let ctx = StandaloneExpressionContext::default().with_binding(
                "data",
                StandaloneExpressionBinding::any(ItemStream::once(
                    RetainedCemNode::new(data.clone(), data_id)
                        .unwrap()
                        .query_item(),
                )),
            );
            let from = h
                .register_captured_scope(
                    input.clone(),
                    input_capture.clone(),
                    Some(ctx.clone()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            h.prepare_captured_lexical_scopes(&input_capture, ctx)
                .unwrap();
            let to = h
                .register_captured_scope(
                    data,
                    data_capture,
                    Some(Default::default()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            if input_grant {
                h.allow_scope_crossing(from, to);
            }
        },
    );
    let parent = parent.unwrap();
    let owner = Arc::new(f.c);
    let installed = Arc::new(candidate(&f.h, &parent, &owner).unwrap());
    let decl=cem_ql::declaration_artifact::build_declaration_artifact("<cem-element tag=\"cem-bound-mixed\"><template id=\"cem-bound-mixed\" type=\"text/cem-ml\">{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=cart-scalar}{attribute @name=names @type=cart-list @maxItems=2}{p | Mixed}</template></cem-element>","file:///app/mixed.xhtml",cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev,None).unwrap();
    let contracts = BTreeMap::from([
        (
            "cart-lines".into(),
            records::ReceiverContract::Node(node_pin),
        ),
        (
            "cart-count".into(),
            records::ReceiverContract::Scalar(named),
        ),
        (
            "cart-scalar".into(),
            records::ReceiverContract::Executable {
                root: "scalar".into(),
                target: parent.roots()["scalar"].clone(),
            },
        ),
        (
            "cart-list".into(),
            records::ReceiverContract::Executable {
                root: "list".into(),
                target: parent.roots()["list"].clone(),
            },
        ),
    ]);
    let c = records::build(decl, parent.clone(), contracts, &control).unwrap();
    let contract = Arc::new(records::verify(&c.bytes, &c.pin, parent.clone(), &control).unwrap());
    Setup {
        h: f.h,
        owner,
        bindings: f.bindings,
        rules: f.rules,
        state: f.state,
        choice,
        selector_calls,
        control: f.control,
        installed,
        contract,
        source: mixed::MixedReceiverSource {
            schema_uri: RECEIVER_URI.into(),
            source: receiver,
            capture,
            attributes: attrs,
        },
        scalar: mixed::ScalarReceiverSource {
            identity: SourceIdentity::new(
                "file:///app/scalars.cem",
                SourceId(1),
                SCALAR_TEXT.as_bytes(),
            ),
            source: scalar_tree,
            attributes: vec![scalar_attrs["cart-count"]],
        },
        input,
        ids,
    }
}
#[test]
fn mixed_binding_installs_original_receivers_and_live_parent_guards() {
    let mut f = setup(
        "{view @items={#data} @count=3 @choice=ok @names=\"a b\"}",
        true,
    );
    let bound = f.bind().unwrap();
    assert_eq!(bound.receiver_count(), 4);
    assert!(bound.is_available(&f.h));
    let count = f.value("count");
    assert!(
        bound
            .validate_scalar(&mut f.h, "count", &count)
            .unwrap()
            .accepted
    );
    let items = f.value("items");
    let node = bound.validate_node(&mut f.h, "items", &items).unwrap();
    assert!(node.complete && !node.failed, "{:?}", node.diagnostics);
    let choice = f.value("choice");
    let input = f.value("choice");
    let invocation = bound
        .preparation_invocation(&mut f.h, "choice", input, Default::default())
        .unwrap();
    let result = invocation.prepare(&mut f.h).unwrap();
    let evidence = result.evidence.as_ref().unwrap();
    assert_eq!(
        invocation
            .validate_prepared(&mut f.h, &choice, Some(evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
}

const INPUT: &str = "{view @items={#data} @count=3 @choice=ok @names=\"a b\"}";
#[test]
fn mixed_binding_intersects_scalar_facets_and_revalidates_typed_inspection() {
    for (n, accepted) in [
        ("2", false),
        ("3", true),
        ("4", true),
        ("5", false),
        ("bad", false),
    ] {
        let mut f = setup(&INPUT.replace("@count=3", &format!("@count={n}")), true);
        let bound = f.bind().unwrap();
        let input = f.value("count");
        let report = bound.validate_scalar(&mut f.h, "count", &input).unwrap();
        assert_eq!(report.accepted, accepted, "{n}");
        if let Some(mut value) = report.value {
            assert!(
                bound
                    .validate_scalar_typed(&mut f.h, "count", &input, &value)
                    .unwrap()
                    .accepted
            );
            value.lexical = "2".into();
            assert!(
                !bound
                    .validate_scalar_typed(&mut f.h, "count", &input, &value)
                    .unwrap()
                    .accepted
            );
        } else {
            assert!(!report.diagnostics.is_empty());
            assert!(report
                .diagnostics
                .iter()
                .any(|d| d.source_map.as_ref().is_some_and(|s| !s.frames.is_empty())));
        }
    }
}
#[test]
fn mixed_binding_refuses_missing_grants_wrong_selectors_and_unsupported_local_facets() {
    let mut f = configured(INPUT, RECEIVER_TEXT, false, true, false);
    assert!(f.bind().is_err());
    for text in [
        RECEIVER_TEXT.replace("@type={#scalar}", "@type={#list}"),
        RECEIVER_TEXT.replace("@type={#scalar}", "@type={#missing}"),
        RECEIVER_TEXT.replace("@type={#scalar}", "@type={#scalar} @pattern=x"),
        RECEIVER_TEXT.replace("@name=items @type=node", "@name=items @type=node @values=x"),
    ] {
        let mut f = configured(INPUT, &text, true, true, false);
        assert!(f.bind().is_err(), "{text}");
    }
    let mut f = configured(INPUT, RECEIVER_TEXT, true, false, false);
    let bound = f.bind().unwrap();
    let input = f.value("items");
    let report = bound.validate_node(&mut f.h, "items", &input).unwrap();
    assert!(!report.complete);
    assert!(report.targets.is_empty());
}
#[test]
fn mixed_binding_requires_exact_registered_sources_occurrences_and_scalar_projection() {
    for case in 0..7 {
        let mut f = setup(INPUT, true);
        match case {
            0 => {
                f.source.source = imported(RECEIVER_TEXT, "file:///app/receiver.cem").0;
            }
            1 => {
                f.source
                    .attributes
                    .insert("count".into(), f.source.attributes["items"]);
            }
            2 => {
                f.source.attributes.remove("choice");
            }
            3 => {
                f.scalar.source = imported(SCALAR_TEXT, "file:///app/scalars.cem").0;
            }
            4 => {
                f.scalar.identity.hash = hash(b"changed");
            }
            5 => {
                f.scalar.attributes.clear();
            }
            _ => {
                f.source.capture = imported(RECEIVER_TEXT, "file:///app/receiver.cem").1;
            }
        }
        assert!(f.bind().is_err(), "case {case}");
    }
    let mut f = setup(INPUT, true);
    let bound = f.bind().unwrap();
    let input = mixed::ReceiverInput {
        source: imported(INPUT, "file:///app/input.cem").0,
        attribute: f.ids["count"],
    };
    assert!(bound.validate_scalar(&mut f.h, "count", &input).is_err());
    let input = f.value("count");
    assert!(bound.validate_scalar(&mut f.h, "choice", &input).is_err());
    assert!(bound.validate_node(&mut f.h, "count", &input).is_err());
}
#[test]
fn mixed_binding_consumes_prepared_lists_without_retokenizing_and_enforces_local_counts() {
    for (text, accepted) in [("a b", true), ("a b c", false)] {
        let mut f = setup(&INPUT.replace("a b", text), true);
        let bound = f.bind().unwrap();
        let input = f.value("names");
        let original = f.value("names");
        let invocation = bound
            .preparation_invocation(&mut f.h, "names", input, Default::default())
            .unwrap();
        let prepared = invocation.prepare(&mut f.h).unwrap();
        assert!(prepared.stopped.is_none(), "{:?}", prepared.stopped);
        let evidence = prepared.evidence.as_ref().unwrap();
        assert_eq!(
            invocation.token_spans(&mut f.h, evidence).unwrap().len(),
            text.split_whitespace().count()
        );
        let before = f.state.calls.lock().unwrap().clone();
        let report = invocation
            .validate_prepared(&mut f.h, &original, Some(evidence))
            .unwrap();
        assert_eq!(report.accepted, Some(accepted));
        assert_eq!(
            *f.state.calls.lock().unwrap(),
            before,
            "preparation was replayed while consuming evidence"
        );
        if !accepted {
            assert!(!report.diagnostics.is_empty());
        }
    }
}
#[test]
fn mixed_binding_refuses_missing_foreign_copied_receipts_and_exhausted_shared_budgets() {
    let mut f = setup(INPUT, true);
    let bound = f.bind().unwrap();
    let input = f.value("choice");
    let other = f.value("choice");
    let a = bound
        .preparation_invocation(&mut f.h, "choice", input, Default::default())
        .unwrap();
    let b = bound
        .preparation_invocation(&mut f.h, "choice", other, Default::default())
        .unwrap();
    let prepared = a.prepare(&mut f.h).unwrap();
    let evidence = prepared.evidence.as_ref().unwrap();
    let original = f.value("choice");
    assert!(b
        .validate_prepared(&mut f.h, &original, Some(evidence))
        .is_err());
    assert!(b.token_spans(&mut f.h, evidence).is_err());
    assert!(a.validate_prepared(&mut f.h, &original, None).is_err());
    let copy = mixed::ReceiverInput {
        source: imported(INPUT, "file:///app/input.cem").0,
        attribute: original.attribute,
    };
    assert!(a
        .validate_prepared(&mut f.h, &copy, Some(evidence))
        .is_err());
    assert_eq!(
        a.validate_prepared(&mut f.h, &original, Some(evidence))
            .unwrap()
            .accepted,
        Some(true)
    );
    a.close();
    assert!(a.token_spans(&mut f.h, evidence).is_err());
    let mut limits = cem_ql::datatype_preparation::PreparationLimits::default();
    limits.max_preparations = 0;
    let input = f.value("choice");
    let invocation = bound
        .preparation_invocation(&mut f.h, "choice", input, limits)
        .unwrap();
    let report = invocation.prepare(&mut f.h).unwrap();
    assert!(report.evidence.is_none());
    assert!(report.stopped.is_some() || report.report.stopped.is_some());
    let input = f.value("choice");
    let invocation = bound
        .preparation_invocation(&mut f.h, "choice", input, Default::default())
        .unwrap();
    let before = invocation.remaining_limits();
    invocation.prepare(&mut f.h).unwrap();
    assert!(invocation.remaining_limits().max_preparations < before.max_preparations);
}
#[test]
fn mixed_binding_does_not_infer_executable_typed_ingress_from_missing_lexical_facets() {
    let mut f = setup(INPUT, true);
    let bound = f.bind().unwrap();
    let input = f.value("choice");
    let before = f.state.calls.lock().unwrap().clone();
    assert!(bound
        .validate_executable_typed(
            &mut f.h,
            "choice",
            &input,
            vec![Item::Atomic(AtomValue::String("ok".into()))],
            Default::default()
        )
        .is_err());
    assert_eq!(*f.state.calls.lock().unwrap(), before);
}
#[test]
fn mixed_binding_atomic_replacement_retires_receipts_but_not_reusable_owners() {
    let mut f = setup(INPUT, true);
    let first = f.bind().unwrap();
    let input = f.value("choice");
    let invocation = first
        .preparation_invocation(&mut f.h, "choice", input, Default::default())
        .unwrap();
    let prepared = invocation.prepare(&mut f.h).unwrap();
    let mut slot = mixed::MixedReceiverSlot::default();
    slot.publish(&mut f.h, first).unwrap();
    f.scalar.identity.hash = hash(b"wrong");
    assert!(f.bind().is_err());
    assert!(slot.current(&f.h).is_ok());
    f.scalar.identity.hash = hash(SCALAR_TEXT.as_bytes());
    let second = f.bind().unwrap();
    slot.publish(&mut f.h, second).unwrap();
    let input = f.value("choice");
    assert!(invocation
        .validate_prepared(&mut f.h, &input, prepared.evidence.as_ref())
        .is_err());
    assert!(slot.current(&f.h).is_ok());
    assert!(f.owner.is_live(&f.h));
    let current = slot.current(&f.h).unwrap();
    let input = f.value("names");
    let invocation = current
        .preparation_invocation(&mut f.h, "names", input, Default::default())
        .unwrap();
    slot.clear();
    assert!(invocation.prepare(&mut f.h).is_err());
    assert!(f.owner.is_live(&f.h));
    assert!(f.installed.root(&f.h, "scalar").is_ok());
}
#[test]
fn mixed_binding_rechecks_epochs_closure_cancellation_and_all_required_owners() {
    for loss in 0..7 {
        let mut f = configured(INPUT, RECEIVER_TEXT, true, true, true);
        let bound = f.bind().unwrap();
        let input = f.value("choice");
        let invocation = bound
            .preparation_invocation(&mut f.h, "choice", input, Default::default())
            .unwrap();
        let prepared = invocation.prepare(&mut f.h).unwrap();
        match loss {
            0 => {
                let scope = f.h.registered_source_scope(&f.input).unwrap();
                assert!(f.h.set_context(scope, Some(Default::default())));
            }
            1 => f.h.close(),
            2 => {
                f.control.cancel_root(None, None).unwrap();
            }
            3 => f.owner.retire(),
            4 => {
                f.bindings.clear();
            }
            5 => {
                f.rules.clear();
            }
            _ => {
                f.installed.root(&f.h, "list").unwrap().retire();
            }
        }
        assert!(!bound.is_available(&f.h), "loss {loss}");
        let input = f.value("choice");
        assert!(invocation
            .validate_prepared(&mut f.h, &input, prepared.evidence.as_ref())
            .is_err());
        let input = f.value("count");
        assert!(bound.validate_scalar(&mut f.h, "count", &input).is_err());
        let mut slot = mixed::MixedReceiverSlot::default();
        assert!(slot.publish(&mut f.h, bound).is_err());
    }
}
#[test]
fn mixed_binding_keeps_no_compilation_host_or_native_callback_owners_alive() {
    let mut f = setup(INPUT, true);
    let bound = f.bind().unwrap();
    let input = f.value("choice");
    let invocation = bound
        .preparation_invocation(&mut f.h, "choice", input, Default::default())
        .unwrap();
    let weak = Arc::downgrade(&f.owner);
    drop(f.owner);
    assert!(weak.upgrade().is_none());
    assert!(!bound.is_available(&f.h));
    assert!(invocation.prepare(&mut f.h).is_err());
    let mut f = setup(INPUT, true);
    let bound = f.bind().unwrap();
    let weak = Arc::downgrade(&f.state);
    let input = f.value("count");
    drop(f.state);
    drop(f.h);
    assert!(weak.upgrade().is_none());
    let mut foreign = FreshStructuralHost::new(&hash(b"registry"), Default::default()).unwrap();
    assert!(!bound.is_available(&foreign));
    assert!(bound
        .validate_scalar(&mut foreign, "count", &input)
        .is_err());
}
#[test]
fn mixed_binding_withholds_late_preparation_after_callback_clear_and_cancellation() {
    for cancel in [false, true] {
        let mut f = setup(INPUT, true);
        let bound = f.bind().unwrap();
        let input = f.value("choice");
        let invocation = bound
            .preparation_invocation(&mut f.h, "choice", input, Default::default())
            .unwrap();
        let mut slot = mixed::MixedReceiverSlot::default();
        slot.publish(&mut f.h, bound).unwrap();
        let slot = std::rc::Rc::new(std::cell::RefCell::new(slot));
        let captured = slot.clone();
        ACTION.with(|a| *a.borrow_mut() = Some(Box::new(move || captured.borrow_mut().clear())));
        f.state
            .mode
            .store(if cancel { 9 } else { 8 }, Ordering::SeqCst);
        assert!(invocation.prepare(&mut f.h).is_err());
        ACTION.with(|a| *a.borrow_mut() = None);
        f.state.mode.store(0, Ordering::SeqCst);
        if !cancel {
            assert!(f.owner.is_live(&f.h));
        }
    }
}
#[test]
fn mixed_binding_renews_mutable_original_type_selection_before_publish_consume_and_return() {
    let text = RECEIVER_TEXT.replace(
        "@type={#scalar}",
        "@type={#native:call(\"fixture.pick\", scalar, list)}",
    );
    let mut f = configured(INPUT, &text, true, true, false);
    let first = f.bind().unwrap();
    let mut slot = mixed::MixedReceiverSlot::default();
    slot.publish(&mut f.h, first).unwrap();
    let staged = f.bind().unwrap();
    let input = f.value("choice");
    let invocation = slot
        .current(&f.h)
        .unwrap()
        .preparation_invocation(&mut f.h, "choice", input, Default::default())
        .unwrap();
    let prepared = invocation.prepare(&mut f.h).unwrap();
    let before = f.selector_calls.load(Ordering::SeqCst);
    f.choice.store(1, Ordering::SeqCst);
    assert!(slot.publish(&mut f.h, staged).is_err());
    let input = f.value("choice");
    assert!(invocation
        .validate_prepared(&mut f.h, &input, prepared.evidence.as_ref())
        .is_err());
    let input = f.value("count");
    assert!(slot
        .current(&f.h)
        .unwrap()
        .validate_scalar(&mut f.h, "count", &input)
        .is_err());
    assert!(f.selector_calls.load(Ordering::SeqCst) > before);
    f.choice.store(0, Ordering::SeqCst);
    assert!(
        slot.current(&f.h)
            .unwrap()
            .validate_scalar(&mut f.h, "count", &input)
            .unwrap()
            .accepted
    );
    let choice = f.choice.clone();
    ACTION.with(|a| *a.borrow_mut() = Some(Box::new(move || choice.store(1, Ordering::SeqCst))));
    f.state.mode.store(7, Ordering::SeqCst);
    assert!(invocation.prepare(&mut f.h).is_err());
    ACTION.with(|a| *a.borrow_mut() = None);
    f.state.mode.store(0, Ordering::SeqCst);
    f.choice.store(0, Ordering::SeqCst);
    assert!(f.owner.is_live(&f.h));
}
#[test]
fn mixed_binding_accepts_explicit_typed_profiles_and_enforces_receiving_list_counts() {
    let mut f = configured_profile(INPUT, RECEIVER_TEXT, true, true, false, true);
    let bound = f.bind().unwrap();
    let input = f.value("choice");
    let report = bound
        .validate_executable_typed(
            &mut f.h,
            "choice",
            &input,
            vec![Item::Atomic(AtomValue::String("ok".into()))],
            Default::default(),
        )
        .unwrap();
    assert_eq!(report.accepted, Some(true));
    let input = f.value("names");
    let report = bound
        .validate_executable_typed(
            &mut f.h,
            "names",
            &input,
            vec![Item::Atomic(AtomValue::String("a".into())); 3],
            Default::default(),
        )
        .unwrap();
    assert_eq!(report.accepted, Some(false));
    assert!(!report.diagnostics.is_empty());
    let input = f.value("choice");
    let report = bound
        .validate_executable_typed(
            &mut f.h,
            "choice",
            &input,
            vec![Item::Atomic(AtomValue::Boolean(true))],
            Default::default(),
        )
        .unwrap();
    assert_ne!(report.accepted, Some(true));
    f.bindings.clear();
    assert!(!bound.is_available(&f.h));
}
#[test]
fn mixed_binding_primitive_fields_need_no_named_scalar_source_and_extra_inputs_refuse() {
    let mut f = setup(INPUT, true);
    let parent = f.contract.application().clone();
    let control = OperationControl::default();
    let decl=cem_ql::declaration_artifact::build_declaration_artifact("<cem-element tag=\"cem-primitive-mixed\"><template id=\"cem-primitive-mixed\" type=\"text/cem-ml\">{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=integer}{attribute @name=choice @type=cart-scalar}{attribute @name=names @type=cart-list @maxItems=2}{p | Mixed}</template></cem-element>","file:///app/primitive.xhtml",cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev,None).unwrap();
    let types = ["cart-lines", "cart-scalar", "cart-list"]
        .into_iter()
        .map(|n| (n.into(), f.contract.receiver(n).unwrap().clone()))
        .collect();
    let built = records::build(decl, parent.clone(), types, &control).unwrap();
    let contract = Arc::new(records::verify(&built.bytes, &built.pin, parent, &control).unwrap());
    let source = || mixed::MixedReceiverSource {
        schema_uri: f.source.schema_uri.clone(),
        source: f.source.source.clone(),
        capture: f.source.capture.clone(),
        attributes: f.source.attributes.clone(),
    };
    let roots = || {
        BTreeMap::from([
            (
                "scalar".into(),
                f.installed.root(&f.h, "scalar").unwrap().clone(),
            ),
            (
                "list".into(),
                f.installed.root(&f.h, "list").unwrap().clone(),
            ),
        ])
    };
    let bindings = roots();
    let original = source();
    let bound = mixed::stage(
        &mut f.h,
        contract.clone(),
        original,
        BTreeMap::new(),
        bindings,
    )
    .unwrap();
    let input = f.value("count");
    assert!(
        bound
            .validate_scalar(&mut f.h, "count", &input)
            .unwrap()
            .accepted
    );
    // Complete input key sets are checked before any native result can be used.
    let source = mixed::MixedReceiverSource {
        schema_uri: f.source.schema_uri.clone(),
        source: f.source.source.clone(),
        capture: f.source.capture.clone(),
        attributes: f.source.attributes.clone(),
    };
    assert!(mixed::stage(&mut f.h, contract, source, BTreeMap::new(), BTreeMap::new()).is_err());
    assert!(bound.is_available(&f.h));
}
#[test]
fn mixed_binding_charges_local_diagnostics_to_the_shared_preparation_allowance() {
    let mut f = setup(&INPUT.replace("a b", "a b c"), true);
    let bound = f.bind().unwrap();
    let input = f.value("names");
    let original = f.value("names");
    let mut limits = cem_ql::datatype_preparation::PreparationLimits::default();
    limits.validation.max_diagnostics = 0;
    let invocation = bound
        .preparation_invocation(&mut f.h, "names", input, limits)
        .unwrap();
    let report = invocation.prepare(&mut f.h).unwrap();
    let evidence = report.evidence.as_ref().unwrap();
    assert!(invocation
        .validate_prepared(&mut f.h, &original, Some(evidence))
        .is_err());
    assert_eq!(invocation.remaining_limits().validation.max_diagnostics, 0);
}
#[test]
fn mixed_binding_withholds_spans_after_tokenizer_readiness_retires_the_group() {
    let mut f = setup(INPUT, true);
    let bound = f.bind().unwrap();
    let input = f.value("names");
    let invocation = bound
        .preparation_invocation(&mut f.h, "names", input, Default::default())
        .unwrap();
    let report = invocation.prepare(&mut f.h).unwrap();
    let mut slot = mixed::MixedReceiverSlot::default();
    slot.publish(&mut f.h, bound).unwrap();
    let slot = std::rc::Rc::new(std::cell::RefCell::new(slot));
    let captured = slot.clone();
    ACTION.with(|a| *a.borrow_mut() = Some(Box::new(move || captured.borrow_mut().clear())));
    f.state.mode.store(13, Ordering::SeqCst);
    assert!(invocation
        .token_spans(&mut f.h, report.evidence.as_ref().unwrap())
        .is_err());
    ACTION.with(|a| *a.borrow_mut() = None);
    f.state.mode.store(0, Ordering::SeqCst);
    assert!(f.owner.is_live(&f.h));
}
