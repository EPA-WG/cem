use super::*;
use cem_ql::application_envelope::v4 as publication;
use cem_ql::selected_registry::SelectedBuiltin;
fn built() -> publication::BuiltApplicationEnvelope {
    let (parent, app) = application();
    let control = OperationControl::default();
    let declarations=["cem-cart","cem-checkout"].into_iter().map(|tag|{
        let body="{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=cart-choice}{cem-app}{p | {$ | str:upper(\"tea\")}}";
        let d=super::super::super::declaration(tag,body);
        mixed::build(d,app.clone(),receivers(&app),&control).unwrap()
    }).collect();
    publication::build(
        parent,
        declarations,
        BTreeSet::from(["cem-cart".into(), "cem-checkout".into()]),
        &control,
    )
    .unwrap()
}
#[test]
fn publication_recomputes_complete_registry_and_keeps_original_parent_owners() {
    let b = built();
    let control = OperationControl::default();
    let v = publication::verify(&b.bytes, &b.pin, &control).unwrap();
    assert!(b.bytes.starts_with(publication::MAGIC));
    assert_eq!(v.component_count(), 3);
    assert_eq!(v.mixed_count(), 2);
    assert_eq!(
        v.registry_demand()
            .registry()
            .entries()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([SelectedBuiltin::StringUpper])
    );
    let mixed = v.mixed("cem-cart").unwrap();
    assert!(Arc::ptr_eq(mixed.application(), v.parent()));
    assert!(Arc::ptr_eq(
        mixed.executable_graph("cart-choice").unwrap(),
        v.parent()
            .assembly_owner(&v.parent().roots()["cart-choice"].assembly)
            .unwrap()
    ));
    assert!(!v.native_link_closure_verified());
    assert!(!v.runtime_admission_verified());
    assert!(cem_ql::application_envelope::verify(
        &b.bytes,
        &cem_ql::application_envelope::ApplicationEnvelopePin {
            hash: b.pin.hash.clone(),
            manifest_hash: b.pin.manifest_hash.clone(),
            registry_hash: b.pin.registry_hash.clone()
        },
        &control
    )
    .is_err());
}
#[test]
fn publication_refuses_duplicate_unreachable_unknown_and_conflicting_declarations() {
    for case in 0..5 {
        let (parent, app) = application();
        let control = OperationControl::default();
        let mut a = receivers(&app);
        let mut b = receivers(&app);
        if case == 4 {
            let ReceiverContract::Scalar(value) = b.get_mut("cart-count").unwrap() else {
                unreachable!()
            };
            value.source_hash = hash(b"different scalar source");
        }
        let tag = if case == 0 { "cem-first" } else { "cem-second" };
        let body="{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=cart-choice}{cem-app}";
        let first = mixed::build(
            super::super::super::declaration("cem-first", body),
            app.clone(),
            std::mem::take(&mut a),
            &control,
        )
        .unwrap();
        let second = mixed::build(
            super::super::super::declaration(tag, body),
            app.clone(),
            b,
            &control,
        )
        .unwrap();
        let roots = match case {
            1 => BTreeSet::from(["cem-first".into()]),
            2 => BTreeSet::from([
                "cem-first".into(),
                "cem-second".into(),
                "cem-missing".into(),
            ]),
            3 => BTreeSet::new(),
            _ => BTreeSet::from(["cem-first".into(), "cem-second".into()]),
        };
        assert!(
            publication::build(parent, vec![first, second], roots, &control).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn publication_verifies_legacy_refusal_parent_roots_and_source_owners() {
    let b = built();
    let control = OperationControl::default();
    let pin = envelope::ApplicationEnvelopePin {
        hash: b.pin.hash.clone(),
        manifest_hash: b.pin.manifest_hash.clone(),
        registry_hash: b.pin.registry_hash.clone(),
    };
    assert!(envelope::verify(&b.bytes, &pin, &control).is_err());
    assert!(cem_ql::application_declaration::admit(&b.bytes, "{}", &hash(b"{}")).is_err());
    let v = publication::verify(&b.bytes, &b.pin, &control).unwrap();
    let order = v.component_order();
    let base = order.iter().position(|t| t == "cem-app").unwrap();
    assert!(base < order.iter().position(|t| t == "cem-cart").unwrap());
    assert_eq!(v.declaration("cem-app").unwrap().tag, "cem-app");
    assert!(v.artifact("cem-cart").is_some());
    assert_eq!(v.requirements().declarations.len(), 3);
}
#[test]
fn publication_registry_includes_original_portable_validator_query_demand() {
    let (parent, app) = application_query(true);
    let control = OperationControl::default();
    let d=mixed::build(super::super::super::declaration("cem-query-app","{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=cart-choice}{cem-app}{p | {$ | str:upper(\"tea\")}}"),app.clone(),receivers(&app),&control).unwrap();
    let b = publication::build(
        parent,
        vec![d],
        BTreeSet::from(["cem-query-app".into()]),
        &control,
    )
    .unwrap();
    let v = publication::verify(&b.bytes, &b.pin, &control).unwrap();
    assert_eq!(
        v.registry_demand().query_functions(),
        &BTreeSet::from([SelectedBuiltin::SequenceCount])
    );
    assert_eq!(
        v.registry_demand()
            .registry()
            .entries()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([SelectedBuiltin::SequenceCount, SelectedBuiltin::StringUpper])
    );
    assert!(v.registry_demand().pinned_json().unwrap().0.len() < 65536);
}
#[test]
fn publication_registry_includes_original_assembly_query_leaf_demand() {
    let control = OperationControl::default();
    let mut base = super::super::super::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let structural = base
        .add_contract(super::super::super::structural(), BTreeSet::new())
        .unwrap();
    base.add_component(
        super::super::super::declaration("cem-app", "{p | Hello}"),
        BTreeMap::new(),
    )
    .unwrap();
    let base = base
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::from([structural]),
            &control,
        )
        .unwrap();
    let query = super::super::super::query_declaration(
        "{accepted: seq:count(value) == 1, diagnostics: ()}",
    );
    let mut module=super::super::wrapped("file:///app/query-types.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=choice @kind=scalar @rule={#check}}}}",Some("choice"));
    let root = datatype(&module.types.capture, "choice");
    module.types.bindings.insert(
        super::super::field(&module.types.capture, "choice", "rule"),
        cem_ql::datatype_module_capsule::ModuleTarget::Query(query.pin.clone()),
    );
    let leaves = vec![cem_ql::datatype_dependency_capsule::DependencyArtifact::query(query)];
    let graph = assembly::export(
        "app",
        &BTreeMap::from([("app".into(), module)]),
        &leaves,
        &[],
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let mut builder = envelope::EnvelopeBuilder::new(base).unwrap();
    let id = builder.add_assembly(graph, leaves, vec![]).unwrap();
    let parent = builder
        .build(
            BTreeMap::from([(
                "cart-choice".into(),
                envelope::AssemblyRoot {
                    assembly: id,
                    datatype: root,
                },
            )]),
            &control,
        )
        .unwrap();
    let app = Arc::new(envelope::verify(&parent.bytes, &parent.pin, &control).unwrap());
    let d=mixed::build(super::super::super::declaration("cem-query-app","{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=cart-choice}{cem-app}"),app.clone(),receivers(&app),&control).unwrap();
    let b = publication::build(
        parent,
        vec![d],
        BTreeSet::from(["cem-query-app".into()]),
        &control,
    )
    .unwrap();
    let v = publication::verify(&b.bytes, &b.pin, &control).unwrap();
    assert_eq!(
        v.registry_demand().query_functions(),
        &BTreeSet::from([SelectedBuiltin::SequenceCount])
    );
    assert_eq!(
        v.registry_demand()
            .registry()
            .entries()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([SelectedBuiltin::SequenceCount])
    );
}
#[test]
fn publication_checks_original_source_claims_across_declarations_scalars_and_parent() {
    for collision in 0..3 {
        let (parent, app) = application();
        let control = OperationControl::default();
        let mut declarations = vec![];
        for tag in ["cem-first", "cem-second"] {
            let source=format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{{attribute @name=items @type=cart-lines @maxItems=2}}{{attribute @name=count @type=cart-count}}{{attribute @name=choice @type=cart-choice}}{{cem-app}}</template></cem-element>");
            let types = receivers(&app);
            let uri = match collision {
                0 => "file:///same-source.xhtml".into(),
                1 => {
                    let ReceiverContract::Scalar(s) = &types["cart-count"] else {
                        unreachable!()
                    };
                    s.source_uri.clone()
                }
                _ => app
                    .application()
                    .declaration("cem-app")
                    .unwrap()
                    .source_uri
                    .clone(),
            };
            let d = cem_ql::declaration_artifact::build_declaration_artifact(
                &source,
                &uri,
                cem_ql::template_artifact::TemplateArtifactSourceMapMode::Dev,
                None,
            )
            .unwrap();
            declarations.push(mixed::build(d, app.clone(), types, &control).unwrap());
        }
        assert!(
            publication::build(
                parent,
                declarations,
                BTreeSet::from(["cem-first".into(), "cem-second".into()]),
                &control
            )
            .is_err(),
            "collision {collision}"
        );
    }
}
