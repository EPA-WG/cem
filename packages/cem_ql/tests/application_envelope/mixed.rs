use super::*;

#[path = "publication.rs"]
mod publication;
use ::cem_ql;
use cem_ql::application_envelope::mixed::{self as mixed, ReceiverContract};
#[allow(dead_code)]
#[path = "../../fixtures/component-application/named_receiver_types.rs"]
mod scalar;
fn application() -> (
    envelope::BuiltApplicationEnvelope,
    Arc<envelope::VerifiedApplicationEnvelope>,
) {
    application_query(false)
}
fn application_query(with_query:bool)->(envelope::BuiltApplicationEnvelope,Arc<envelope::VerifiedApplicationEnvelope>){
    let control = OperationControl::default();
    let mut b = super::super::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let id = b
        .add_contract(super::super::structural(), BTreeSet::new())
        .unwrap();
    b.add_component(
        super::super::declaration("cem-app", "{p | Hello}"),
        BTreeMap::new(),
    )
    .unwrap();
    let mut contract_roots=BTreeSet::from([id]);
    if with_query {contract_roots.insert(b.add_contract(super::super::query_body("{accepted: seq:count(value) == 1, diagnostics: ()}"),BTreeSet::new()).unwrap());}
    let b = b
        .build(
            BTreeSet::from(["cem-app".into()]),
            contract_roots,
            &control,
        )
        .unwrap();
    let mut builder = EnvelopeBuilder::new(b).unwrap();
    let (m, p) = fixture();
    let root = datatype(&m["app"].types.capture, "choice");
    let id = builder.add_assembly(graph(&m, &p), vec![], p).unwrap();
    let b = builder
        .build(
            BTreeMap::from([(
                "cart-choice".into(),
                AssemblyRoot {
                    assembly: id,
                    datatype: root,
                },
            )]),
            &control,
        )
        .unwrap();
    let verified = Arc::new(envelope::verify(&b.bytes, &b.pin, &control).unwrap());
    (b, verified)
}
fn receivers(app: &envelope::VerifiedApplicationEnvelope) -> BTreeMap<String, ReceiverContract> {
    let pin = app
        .application()
        .dependency_order()
        .iter()
        .find_map(|k| match k {
            cem_ql::application_envelope::DependencyKey::Contract(id) => {
                app.application().structural(id).map(|v| v.pin().clone())
            }
            _ => None,
        })
        .unwrap();
    BTreeMap::from([
        ("cart-lines".into(), ReceiverContract::Node(pin)),
        (
            "cart-count".into(),
            ReceiverContract::Scalar(scalar::types().remove(0)),
        ),
        (
            "cart-choice".into(),
            ReceiverContract::Executable {
                root: "cart-choice".into(),
                target: app.roots()["cart-choice"].clone(),
            },
        ),
    ])
}
fn declaration() -> cem_ql::declaration_artifact::BuiltDeclarationArtifact {
    super::super::declaration("cem-mixed", "{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=cart-choice}{p | Example}")
}
#[test]
fn mixed_receiver_contracts_preserve_node_scalar_and_executable_requirements_without_admission() {
    let (_, app) = application();
    let control = OperationControl::default();
    let b = mixed::build(declaration(), app.clone(), receivers(&app), &control).unwrap();
    let v = mixed::verify(&b.bytes, &b.pin, app.clone(), &control).unwrap();
    assert!(b.bytes.starts_with(mixed::MAGIC));
    assert_eq!(v.declaration().tag, "cem-mixed");
    assert_eq!(v.receiver_count(), 3);
    assert_eq!(v.occurrence_count(), 3);
    assert!(v.requirements().unresolved.is_empty());
    assert!(matches!(
        v.receiver("cart-lines"),
        Some(ReceiverContract::Node(_))
    ));
    let Some(ReceiverContract::Scalar(value)) = v.receiver("cart-count") else {
        panic!("closed scalar contract")
    };
    assert_eq!(value.value.model.min_inclusive.as_deref(), Some("2"));
    assert_eq!(value.value.model.max_inclusive.as_deref(), Some("5"));
    assert!(!value.value.model.source_map.frames.is_empty());
    assert!(Arc::ptr_eq(v.application(), &app));
    let root = &app.roots()["cart-choice"];
    assert!(Arc::ptr_eq(
        v.executable_graph("cart-choice").unwrap(),
        app.assembly_owner(&root.assembly).unwrap()
    ));
    assert!(!v.runtime_admission_verified());
    assert!(!v.executable_closure_verified());
    assert!(cem_ql::application_declaration::admit(&b.bytes, "{}", &hash(b"{}")).is_err());
    assert_eq!(
        b.bytes,
        mixed::build(declaration(), app.clone(), receivers(&app), &control)
            .unwrap()
            .bytes
    );
}
#[test]
fn mixed_receiver_aliases_native_pins_and_assembly_targets_are_exact() {
    let (_, app) = application();
    for case in 0..13 {
        let mut contracts = receivers(&app);
        match case {
            0 => {
                contracts.remove("cart-count");
            }
            1 => {
                contracts.insert("unused".into(), contracts["cart-lines"].clone());
            }
            2 => {
                let node = contracts.remove("cart-lines").unwrap();
                contracts.insert("node".into(), node);
            }
            3 => {
                let ReceiverContract::Scalar(v) = contracts.get_mut("cart-count").unwrap() else {
                    unreachable!()
                };
                v.name = "different".into();
            }
            4 => {
                let ReceiverContract::Scalar(v) = contracts.get_mut("cart-count").unwrap() else {
                    unreachable!()
                };
                v.value.model.native_type_pending = true;
            }
            5 => {
                let ReceiverContract::Scalar(v) = contracts.get_mut("cart-count").unwrap() else {
                    unreachable!()
                };
                v.value.model.value_type = Some("node".into());
            }
            6 => {
                let ReceiverContract::Node(pin) = contracts.get_mut("cart-lines").unwrap() else {
                    unreachable!()
                };
                pin.schema_uri = "urn:other".into();
            }
            7 => {
                let ReceiverContract::Node(pin) = contracts.get_mut("cart-lines").unwrap() else {
                    unreachable!()
                };
                pin.source.hash = hash(b"other");
            }
            8 => {
                let ReceiverContract::Node(pin) = contracts.get_mut("cart-lines").unwrap() else {
                    unreachable!()
                };
                pin.registry_hash = hash(b"other");
            }
            9 => {
                let ReceiverContract::Node(pin) = contracts.get_mut("cart-lines").unwrap() else {
                    unreachable!()
                };
                pin.hash = app.roots()["cart-choice"].assembly.clone();
            }
            10 => {
                let ReceiverContract::Executable { root, .. } =
                    contracts.get_mut("cart-choice").unwrap()
                else {
                    unreachable!()
                };
                *root = "unexported".into();
            }
            11 => {
                let ReceiverContract::Executable { target, .. } =
                    contracts.get_mut("cart-choice").unwrap()
                else {
                    unreachable!()
                };
                target.datatype = u32::MAX;
            }
            _ => {
                let ReceiverContract::Executable { target, .. } =
                    contracts.get_mut("cart-choice").unwrap()
                else {
                    unreachable!()
                };
                target.assembly = hash(b"other graph");
            }
        }
        assert!(
            mixed::build(
                declaration(),
                app.clone(),
                contracts,
                &OperationControl::default()
            )
            .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn mixed_literal_occurrences_keep_fields_and_refuse_nested_dynamic_duplicate_or_unsupported_policies(
) {
    let (_, app) = application();
    let closed = || {
        BTreeMap::from([(
            "cart-count".into(),
            ReceiverContract::Scalar(scalar::types().remove(0)),
        )])
    };
    for body in [
        "{div | {attribute @name=count @type=cart-count}}",
        "{schema:attribute @name=count @type=cart-count}",
        "{attribute @name=count @type=cart-count}{attribute @name=count @type=integer}",
        "{attribute @name=count @type=cart-count @type=cart-count}",
        "{attribute @name=count @type=cart-count @maxItems='{$limit}'}",
        "{attribute @name=count @type=cart-count @unknown=policy}",
        "{attribute @name=count @type=cart-count | 3}",
        "{attribute @name=count @type=missing}",
        "{attribute @name=count}",
    ] {
        let source = format!("<cem-element tag=\"cem-mixed\"><template id=\"cem-mixed\" type=\"text/cem-ml\">{body}</template></cem-element>");
        let result = cem_ql::declaration_artifact::build_declaration_artifact(
            &source,
            "file:///app/mixed.xhtml",
            cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev,
            None,
        )
        .and_then(|declaration| {
            mixed::build(
                declaration,
                app.clone(),
                closed(),
                &OperationControl::default(),
            )
        });
        assert!(result.is_err(), "{body}");
    }
    let b = mixed::build(super::super::declaration("cem-mixed","{attribute @name=count @type=cart-count @minItems=0 @maxItems=2}{attribute @name=plain @type=string}"),app.clone(),closed(),&OperationControl::default()).unwrap();
    let v = mixed::verify(&b.bytes, &b.pin, app, &OperationControl::default()).unwrap();
    assert_eq!(v.occurrence_count(), 2);
    assert_eq!(v.occurrences()["count"].fields["minItems"], "0");
    assert_eq!(v.occurrences()["count"].fields["maxItems"], "2");
    assert_eq!(v.occurrences()["plain"].fields["type"], "string");
}
#[test]
fn mixed_declarations_preserve_existing_scalar_facet_validation_and_diagnostics() {
    use cem_ql::{
        eval::{AtomValue, Item, ItemStream},
        render::{render_compiled_template, render_plan_to_html, TemplateData},
    };
    let (_, app) = application();
    let mut contracts = receivers(&app);
    let scalar = contracts.remove("cart-count").unwrap();
    let b = mixed::build(
        super::super::declaration(
            "cem-count",
            "{attribute @name=count @type=cart-count}{p | {$count}}",
        ),
        app.clone(),
        BTreeMap::from([("cart-count".into(), scalar)]),
        &OperationControl::default(),
    )
    .unwrap();
    let v = mixed::verify(&b.bytes, &b.pin, app, &OperationControl::default()).unwrap();
    let Some(ReceiverContract::Scalar(scalar)) = v.receiver("cart-count") else {
        unreachable!()
    };
    for (input, accepted) in [("003", true), ("6", false)] {
        // Explicit existing scalar-only renderer boundary; no node or executable
        // requirement is removed or coerced to obtain a partial mixed render.
        let mut data = TemplateData::default().with_binding(
            "count",
            ItemStream::once(Item::Atomic(AtomValue::String(input.into()))),
        );
        data.value_types
            .insert("cart-count".into(), scalar.value.clone());
        let plan = render_compiled_template(v.artifact(), &data);
        assert_eq!(
            !plan
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()),
            accepted
        );
        if accepted {
            assert!(render_plan_to_html(&plan).contains(">3<"));
        }
    }
}
#[test]
fn mixed_declarations_refuse_conflicting_scalar_source_assertions() {
    let (_, app) = application();
    for uri in ["file:///app/app.cem", "file:///apps/example/cart.cem"] {
        let mut contracts = receivers(&app);
        let ReceiverContract::Scalar(v) = contracts.get_mut("cart-count").unwrap() else {
            unreachable!()
        };
        v.source_uri = uri.into();
        assert!(mixed::build(
            declaration(),
            app.clone(),
            contracts,
            &OperationControl::default()
        )
        .is_err());
    }
    let mut types = scalar::types();
    types[1].source_hash = hash(b"conflicting scalar source");
    let declaration = super::super::declaration(
        "cem-mixed",
        "{attribute @name=count @type=cart-count}{attribute @name=label @type=cart-label}",
    );
    let contracts = types
        .into_iter()
        .map(|v| (v.name.clone(), ReceiverContract::Scalar(v)))
        .collect();
    assert!(mixed::build(declaration, app, contracts, &OperationControl::default()).is_err());
}
#[test]
fn mixed_contract_dependency_tags_cannot_shadow_or_silently_acquire_unknown_components() {
    let (_, app) = application();
    let contracts = || {
        BTreeMap::from([(
            "cart-count".into(),
            ReceiverContract::Scalar(scalar::types().remove(0)),
        )])
    };
    for body in [
        "{attribute @name=count @type=cart-count}{cem-missing}",
        "{attribute @name=count @type=cart-count}{img @src='asset.png'}",
        "{attribute @name=count @type=cart-count}{cem-element @src='other.xhtml'}",
    ] {
        assert!(mixed::build(
            super::super::declaration("cem-mixed", body),
            app.clone(),
            contracts(),
            &OperationControl::default()
        )
        .is_err());
    }
    assert!(mixed::build(
        super::super::declaration("cem-app", "{attribute @name=count @type=cart-count}"),
        app.clone(),
        contracts(),
        &OperationControl::default()
    )
    .is_err());
    let b = mixed::build(
        super::super::declaration(
            "cem-mixed",
            "{attribute @name=count @type=cart-count}{cem-app}",
        ),
        app.clone(),
        contracts(),
        &OperationControl::default(),
    )
    .unwrap();
    let v = mixed::verify(&b.bytes, &b.pin, app, &OperationControl::default()).unwrap();
    assert_eq!(
        v.component_dependencies(),
        &BTreeSet::from(["cem-app".into()])
    );
}
#[test]
fn mixed_alias_and_occurrence_counts_have_separate_bounded_inventories() {
    let (_, app) = application();
    for count in [64, 65] {
        let body = (0..count)
            .map(|i| format!("{{attribute @name=field{i} @type=cart-count}}"))
            .collect::<String>();
        let c = BTreeMap::from([(
            "cart-count".into(),
            ReceiverContract::Scalar(scalar::types().remove(0)),
        )]);
        assert_eq!(
            mixed::build(
                super::super::declaration("cem-mixed", &body),
                app.clone(),
                c,
                &OperationControl::default()
            )
            .is_err(),
            count > 64
        );
        let mut c = BTreeMap::new();
        let mut body = String::new();
        for i in 0..count {
            let mut t = scalar::types().remove(0);
            t.name = format!("count-{i}");
            body.push_str(&format!("{{attribute @name=field{i} @type={}}}", t.name));
            c.insert(t.name.clone(), ReceiverContract::Scalar(t));
        }
        assert_eq!(
            mixed::build(
                super::super::declaration("cem-mixed", &body),
                app.clone(),
                c,
                &OperationControl::default()
            )
            .is_err(),
            count > 64
        );
    }
}
#[test]
fn mixed_verification_keeps_fresh_dependency_owners_and_legacy_admission_closed() {
    let (parent, app) = application();
    let c = OperationControl::default();
    let b = mixed::build(declaration(), app.clone(), receivers(&app), &c).unwrap();
    let v = mixed::verify(&b.bytes, &b.pin, app.clone(), &c).unwrap();
    let fresh = Arc::new(envelope::verify(&parent.bytes, &parent.pin, &c).unwrap());
    let restored = mixed::verify(&b.bytes, &b.pin, fresh.clone(), &c).unwrap();
    assert!(Arc::ptr_eq(restored.application(), &fresh));
    assert!(!Arc::ptr_eq(
        v.executable_graph("cart-choice").unwrap(),
        restored.executable_graph("cart-choice").unwrap()
    ));
    let cancelled = OperationControl::default();
    cancelled.cancel_root(None, None).unwrap();
    assert!(mixed::verify(&b.bytes, &b.pin, app.clone(), &cancelled).is_err());
    assert!(mixed::build(declaration(), app.clone(), receivers(&app), &cancelled).is_err());
    let mut wrong = b.pin.clone();
    wrong.application_hash = hash(b"wrong parent");
    assert!(mixed::verify(&b.bytes, &wrong, app.clone(), &c).is_err());
    let pin = envelope::ApplicationEnvelopePin {
        hash: b.pin.hash.clone(),
        manifest_hash: b.pin.manifest_hash.clone(),
        registry_hash: b.pin.registry_hash.clone(),
    };
    assert!(envelope::verify(&b.bytes, &pin, &c).is_err());
    let pin = cem_ql::application_envelope::ApplicationEnvelopePin {
        hash: b.pin.hash.clone(),
        manifest_hash: b.pin.manifest_hash.clone(),
        registry_hash: b.pin.registry_hash.clone(),
    };
    assert!(cem_ql::application_envelope::verify(&b.bytes, &pin, &c).is_err());
    assert_eq!(v.occurrence_count(), 3);
}
#[test]
fn mixed_literal_cardinality_reuses_native_bounded_count_policy() {
    let (_, app) = application();
    for (fields, valid) in [
        ("@itemCount=0", true),
        ("@minItems=0 @maxItems=100000", true),
        ("@itemCount=2 @minItems=1 @maxItems=3", true),
        ("@minItems=garbage", false),
        ("@itemCount=-1", false),
        ("@maxItems=100001", false),
        ("@maxItems=184467440737095516160", false),
        ("@minItems=3 @maxItems=2", false),
        ("@itemCount=3 @maxItems=2", false),
        ("@itemCount=1 @minItems=2", false),
    ] {
        let d = super::super::declaration(
            "cem-mixed",
            &format!("{{attribute @name=count @type=cart-count {fields}}}"),
        );
        let c = BTreeMap::from([(
            "cart-count".into(),
            ReceiverContract::Scalar(scalar::types().remove(0)),
        )]);
        assert_eq!(
            mixed::build(d, app.clone(), c, &OperationControl::default()).is_ok(),
            valid,
            "{fields}"
        );
    }
}
