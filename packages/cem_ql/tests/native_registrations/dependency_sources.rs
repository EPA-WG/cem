use super::*;
use cem_ml::schema::datatype_registry::{DatatypeDependencyRole, DatatypeDependencyValue};
use cem_ql::native_registrations::dependency_sources as sources;

fn dependency_text() -> String {
    text().replace("{type @name=sample @kind=scalar}",
        "{type @name=sample @kind=scalar @base={#base} @rule={#rule} | {#colors} {constant @value=red}}")
}
fn identity_for(t: &str) -> SourceIdentity {
    SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), t.as_bytes())
}
fn constant(t: &str) -> sources::BuiltOperationSource {
    let c = capture(t);
    sources::export_operation(
        &c,
        &datatype(&c, "sample"),
        OperationMode::Constant,
        identity_for(t),
        additions().remove(0).2,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap()
}
fn checked(b: &sources::BuiltOperationSource) -> sources::VerifiedOperationSource {
    sources::verify(&b.bytes, &b.pin, &OperationControl::default()).unwrap()
}
#[test]
fn dependency_sources_keep_exact_fields_constant_slots_and_reference_contexts_without_selection() {
    let t = dependency_text();
    let c = capture(&t);
    let b = constant(&t);
    let v = checked(&b);
    assert!(b.bytes.starts_with(sources::MAGIC));
    assert!(!v.selections_verified());
    assert!(!v.executable_closure_verified());
    assert_eq!(v.dependencies().len(), 2);
    assert_eq!(
        v.dependencies()[0].role,
        DatatypeDependencyRole::InheritedBase
    );
    assert_eq!(
        v.dependencies()[1].role,
        DatatypeDependencyRole::ValidationRule
    );
    assert!(v
        .dependencies()
        .iter()
        .all(|d| matches!(d.value, DatatypeDependencyValue::Native(_))));
    assert_eq!(v.constant_slots().len(), 2);
    assert!(
        matches!(v.constant_slots()[0].node(),CemAstNode::Reference{expression,..} if expression=="#colors")
    );
    assert_eq!(v.references().len(), 3);
    for reference in v.references() {
        assert!(Arc::ptr_eq(reference.document(), v.capture().document()));
        assert!(
            matches!(reference.node(),CemAstNode::Reference{context,targets:None,..} if matches!(v.capture().document().get(*context),Some(CemAstNode::Element{..})|Some(CemAstNode::Attribute{..})))
        );
        assert_eq!(
            format!("{:?}", c.document().get(reference.node_id())),
            format!("{:?}", Some(reference.node()))
        );
    }
    for occurrence in c.occurrences() {
        assert_eq!(
            rmp_serde::to_vec_named(&c.snapshot(c.document(), occurrence)).unwrap(),
            rmp_serde::to_vec_named(&v.capture().snapshot(v.capture().document(), occurrence))
                .unwrap()
        );
    }
    assert_eq!(b.bytes, constant(&t).bytes);
    let other = checked(&b);
    assert!(!Arc::ptr_eq(
        v.source().ast_owner(),
        other.source().ast_owner()
    ));
}
#[test]
fn dependency_sources_preserve_list_items_and_whole_list_inheritance() {
    for (name, t) in [
        ("names", text().replace("@base=sample", "@base={#item}")),
        (
            "derived",
            text().replace("@list-base=names", "@list-base={#list}"),
        ),
    ] {
        let c = capture(&t);
        let control = OperationControl::default();
        let id = descriptors().pop().unwrap().2;
        let b = sources::export_operation(
            &c,
            &datatype(&c, name),
            OperationMode::ListSerializer,
            identity_for(&t),
            id,
            &hash(b"registry"),
            &control,
        )
        .unwrap();
        let v = checked(&b);
        assert_eq!(v.dependencies().len(), 1);
        assert_eq!(
            v.dependencies()[0].role,
            if name == "names" {
                DatatypeDependencyRole::ListItem
            } else {
                DatatypeDependencyRole::InheritedList
            }
        );
    }
}
#[test]
fn dependency_sources_reuse_every_direct_operation_and_tokenizer_signature_check() {
    for (mode, name, id) in descriptors().into_iter().chain(additions()) {
        let t = text().replace(
            "{type @name=sample @kind=scalar}",
            "{type @name=sample @kind=scalar | {#colors}}",
        );
        let c = capture(&t);
        let control = OperationControl::default();
        let b = sources::export_operation(
            &c,
            &datatype(&c, name),
            mode,
            identity_for(&t),
            id.clone(),
            &hash(b"registry"),
            &control,
        )
        .unwrap();
        assert_eq!(checked(&b).implementation(), &id);
        let wrong = ImplementationIdentity::equality(
            "wrong",
            hash(b"native"),
            "equal",
            ScalarRepresentation::String,
        );
        if mode != OperationMode::Equality {
            assert!(sources::export_operation(
                &c,
                &datatype(&c, name),
                mode,
                identity_for(&t),
                wrong,
                &hash(b"registry"),
                &control
            )
            .is_err());
        }
    }
    let t = text().replace("@base=sample", "@base={#item}");
    let c = capture(&t);
    let control = OperationControl::default();
    let b = sources::export_tokenizer(
        &c,
        &datatype(&c, "names"),
        TokenizerMode::Ready,
        identity_for(&t),
        tokenizer_id(),
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assert_eq!(
        checked(&b).dependencies()[0].role,
        DatatypeDependencyRole::ListItem
    );
    assert!(sources::export_tokenizer(
        &c,
        &datatype(&c, "names"),
        TokenizerMode::CheckedReplacement,
        identity_for(&t),
        tokenizer_id(),
        &hash(b"registry"),
        &control
    )
    .is_err());
}
#[test]
fn dependency_sources_keep_original_source_backed_result_contracts_and_foreign_input_refusals() {
    for (profile, id) in source_descriptors() {
        let t = source_text(profile).replace(
            "{type @name=sample @kind=scalar}",
            "{type @name=sample @kind=scalar | {#colors}}",
        );
        let c = capture(&t);
        let copy = capture(&t);
        let control = OperationControl::default();
        let b = sources::export_source_operation(
            &c,
            &datatype(&c, "sample"),
            &node(&c, "behavior"),
            profile,
            identity_for(&t),
            id.clone(),
            &hash(b"registry"),
            &control,
        )
        .unwrap();
        assert_eq!(checked(&b).implementation(), &id);
        assert_eq!(checked(&b).constant_slots().len(), 1);
        assert!(sources::export_source_operation(
            &c,
            &datatype(&c, "sample"),
            &node(&copy, "behavior"),
            profile,
            identity_for(&t),
            id.clone(),
            &hash(b"registry"),
            &control
        )
        .is_err());
        assert!(sources::export_source_operation(
            &c,
            &datatype(&copy, "sample"),
            &node(&c, "behavior"),
            profile,
            identity_for(&t),
            id,
            &hash(b"registry"),
            &control
        )
        .is_err());
    }
}
#[test]
fn dependency_sources_require_explicit_whole_snapshot_agreement_for_owner_sharing() {
    let t = dependency_text();
    let b = constant(&t);
    let a = checked(&b);
    let copy = checked(&b);
    let control = OperationControl::default();
    assert!(!Arc::ptr_eq(
        a.source().ast_owner(),
        copy.source().ast_owner()
    ));
    let shared = a.share_source(&copy, &control).unwrap();
    assert!(Arc::ptr_eq(
        shared.source().ast_owner(),
        copy.source().ast_owner()
    ));
    assert!(shared
        .constant_slots()
        .iter()
        .all(|slot| Arc::ptr_eq(slot.document(), copy.source().ast_owner())));
    let t2 = t.replace("@namespace=urn:example", "@namespace=urn:changed");
    let c = capture(&t2);
    let other = sources::export_operation(
        &c,
        &datatype(&c, "sample"),
        OperationMode::Constant,
        identity_for(&t),
        additions().remove(0).2,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assert!(a.share_source(&checked(&other), &control).is_err());
    let mut wrong_pin = b.pin.clone();
    wrong_pin.registry_hash = hash(b"other");
    assert!(sources::verify(&b.bytes, &wrong_pin, &control).is_err());
}
#[test]
fn dependency_sources_cannot_enter_closed_native_or_existing_profile_readers() {
    let t = dependency_text();
    let b = constant(&t);
    let control = OperationControl::default();
    let pin = native::NativeRequirementPin {
        hash: b.pin.hash.clone(),
        source: b.pin.source.clone(),
        registry_hash: b.pin.registry_hash.clone(),
    };
    assert!(native::verify(&b.bytes, &pin, &control).is_err());
    use cem_ql::native_registrations::profiles::{self, OperationProfile, OperationRole};
    let source = checked(&b);
    let profiles = BTreeMap::from([(
        "constants".into(),
        OperationProfile {
            source: b.pin.source.clone(),
            scope: source.datatype_source().scope().node_id(),
            declaration: source.datatype_source().declaration().node_id(),
            kind: DatatypeKind::Scalar,
            representation: ValueRepresentation::Scalar(ScalarRepresentation::String),
            dependencies: BTreeMap::from([(OperationRole::Constant, pin.clone())]),
        },
    )]);
    let artifact = native::BuiltNativeRequirement {
        bytes: b.bytes.clone(),
        pin,
    };
    assert!(profiles::export(&profiles, &[artifact], &hash(b"registry"), &control).is_err());

    let c = capture(&t);
    assert!(native::export_operation(
        &c,
        &datatype(&c, "sample"),
        OperationMode::Constant,
        identity_for(&t),
        additions().remove(0).2,
        &hash(b"registry"),
        &control
    )
    .is_err());
    let closed_text = text();
    let closed = capture(&closed_text);
    let old = native::export_operation(
        &closed,
        &datatype(&closed, "sample"),
        OperationMode::Constant,
        identity_for(&closed_text),
        additions().remove(0).2,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assert!(sources::verify(&old.bytes, &b.pin, &control).is_err());
    let foreign = capture(&t);
    assert!(sources::export_operation(
        &c,
        &datatype(&foreign, "sample"),
        OperationMode::Constant,
        identity_for(&t),
        additions().remove(0).2,
        &hash(b"registry"),
        &control
    )
    .is_err());
}

#[test]
fn dependency_sources_reject_repinned_versions_fields_modes_contexts_and_trailing_records() {
    use cem_ml::schema::native_source::dependencies::NativeDependencySourceSnapshot;
    #[derive(serde::Serialize, serde::Deserialize)]
    enum Selection {
        Operation {
            scope: u32,
            declaration: u32,
            mode: OperationMode,
        },
        Validator {
            scope: u32,
            behavior: u32,
        },
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wire {
        abi: String,
        ml_version: String,
        ql_version: String,
        registry_hash: String,
        identity: SourceIdentity,
        implementation: ImplementationIdentity,
        selection: Selection,
        source: NativeDependencySourceSnapshot,
    }
    let b = constant(&dependency_text());
    let control = OperationControl::default();
    let raw = &b.bytes[sources::MAGIC.len()..];
    let body: Wire = rmp_serde::from_slice(raw).unwrap();
    assert_eq!(rmp_serde::to_vec_named(&body).unwrap(), raw);
    for case in 0..10 {
        let mut w: Wire = rmp_serde::from_slice(raw).unwrap();
        match case {
            0 => w.abi = "unsupported".into(),
            1 => w.ml_version = "unsupported".into(),
            2 => w.ql_version = "unsupported".into(),
            3 => w.registry_hash = hash(b"other"),
            4 => w.identity.uri.push_str(".other"),
            5 => w.implementation.abi = "unsupported".into(),
            6 => {
                w.selection = Selection::Validator {
                    scope: 1,
                    behavior: 1,
                }
            }
            7 => {
                let Selection::Operation { scope, .. } = &mut w.selection else {
                    unreachable!()
                };
                *scope = 0;
            }
            8 => {
                let Selection::Operation { declaration, .. } = &mut w.selection else {
                    unreachable!()
                };
                *declaration = u32::MAX;
            }
            _ => {
                let Selection::Operation { mode, .. } = &mut w.selection else {
                    unreachable!()
                };
                *mode = OperationMode::Equality;
            }
        }
        let bytes = [sources::MAGIC, &rmp_serde::to_vec_named(&w).unwrap()].concat();
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(sources::verify(&bytes, &pin, &control).is_err(), "{case}");
    }
    #[derive(serde::Serialize)]
    struct Extra {
        #[serde(flatten)]
        capsule: Wire,
        unknown: bool,
    }
    let extra = [
        sources::MAGIC,
        &rmp_serde::to_vec_named(&Extra {
            capsule: body,
            unknown: true,
        })
        .unwrap(),
    ]
    .concat();
    let mut context = b.bytes.clone();
    let key = b"\xa8contexts";
    let at = context.windows(key.len()).position(|s| s == key).unwrap() + key.len();
    // The first map key/value are small original node IDs in this bounded fixture.
    assert_eq!(context[at], 0x83);
    assert!(context[at + 1] < 0x80 && context[at + 2] < 0x80);
    context[at + 2] = 0;
    let mut version = b.bytes.clone();
    let key = b"\xa7version";
    let at = version.windows(key.len()).position(|s| s == key).unwrap() + key.len();
    assert_eq!(version[at], 1);
    version[at] = 2;
    let mut targets = b.bytes.clone();
    let key = b"\x87\xa4kind\xa9reference";
    let at = targets.windows(key.len()).position(|s| s == key).unwrap();
    targets[at] = 0x88;
    targets.splice(at + 1..at + 1, b"\xa7targets\x90".iter().copied());
    for bytes in [
        extra,
        context,
        version,
        targets,
        [b.bytes.as_slice(), &[0]].concat(),
        [b"CEMOS2\n", raw].concat(),
        [sources::MAGIC, &vec![0x91; 80], &[0xc0]].concat(),
        vec![0; 64 * 1024 + 1],
    ] {
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(sources::verify(&bytes, &pin, &control).is_err());
    }
}
#[test]
fn dependency_sources_bound_inventories_and_never_return_after_cancellation() {
    let t = dependency_text();
    let b = constant(&t);
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    assert!(sources::verify(&b.bytes, &b.pin, &control).is_err());
    let v = checked(&b);
    assert!(v.share_source(&checked(&b), &control).is_err());
    let c = capture(&t);
    assert!(sources::export_operation(
        &c,
        &datatype(&c, "sample"),
        OperationMode::Constant,
        identity_for(&t),
        additions().remove(0).2,
        &hash(b"registry"),
        &control
    )
    .is_err());
    for count in [129, 1000] {
        let t = text().replace(
            "{type @name=sample @kind=scalar}",
            &format!(
                "{{type @name=sample @kind=scalar | {}}}",
                "{#colors}".repeat(count)
            ),
        );
        let c = capture(&t);
        assert!(sources::export_operation(
            &c,
            &datatype(&c, "sample"),
            OperationMode::Constant,
            identity_for(&t),
            additions().remove(0).2,
            &hash(b"registry"),
            &OperationControl::default()
        )
        .is_err());
    }
    let mut pin = b.pin.clone();
    pin.hash = hash(b"wrong");
    assert!(sources::verify(&b.bytes, &pin, &OperationControl::default()).is_err());
}

#[test]
fn dependency_sources_preserve_checked_replacement_modes_on_original_native_fields() {
    let t = dependency_text();
    let c = capture(&t);
    let control = OperationControl::default();
    for (mode, id) in [
        (
            OperationMode::Preparation(PreparationMode::CheckedReplacement),
            descriptors().remove(0).2,
        ),
        (
            OperationMode::FacetProfile(FacetMode::CheckedReplacement),
            additions().remove(1).2,
        ),
    ] {
        let b = sources::export_operation(
            &c,
            &datatype(&c, "sample"),
            mode,
            identity_for(&t),
            id.clone(),
            &hash(b"registry"),
            &control,
        )
        .unwrap();
        assert_eq!(checked(&b).dependencies().len(), 2);
        let wrong = match mode {
            OperationMode::Preparation(_) => OperationMode::Preparation(PreparationMode::Ready),
            _ => OperationMode::FacetProfile(FacetMode::Ready),
        };
        assert!(sources::export_operation(
            &c,
            &datatype(&c, "sample"),
            wrong,
            identity_for(&t),
            id,
            &hash(b"registry"),
            &control
        )
        .is_err());
    }
    let t = text().replace("@list-base=names", "@list-base={#list}");
    let c = capture(&t);
    let b = sources::export_tokenizer(
        &c,
        &datatype(&c, "derived"),
        TokenizerMode::CheckedReplacement,
        identity_for(&t),
        tokenizer_id(),
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assert_eq!(
        checked(&b).dependencies()[0].role,
        DatatypeDependencyRole::InheritedList
    );
    let id = additions().pop().unwrap().2;
    let b = sources::export_operation(
        &c,
        &datatype(&c, "derived"),
        OperationMode::ListItems(PreparationMode::CheckedReplacement),
        identity_for(&t),
        id,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assert_eq!(
        checked(&b).dependencies()[0].role,
        DatatypeDependencyRole::InheritedList
    );
}

#[path = "dependency_owners.rs"]
mod owners;
