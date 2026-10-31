use super::*;
use cem_ql::native_registrations::profiles::{self, OperationProfile, OperationRole as Role};

fn built(
    mode: OperationMode,
    name: &str,
    id: ImplementationIdentity,
) -> native::BuiltNativeRequirement {
    let text = text();
    let c = capture(&text);
    native::export_operation(
        &c,
        &datatype(&c, name),
        mode,
        SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), text.as_bytes()),
        id,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap()
}
fn profile(
    artifact: &native::BuiltNativeRequirement,
    role: Role,
    kind: DatatypeKind,
) -> OperationProfile {
    let requirement =
        native::verify(&artifact.bytes, &artifact.pin, &OperationControl::default()).unwrap();
    let source = requirement.datatype_source().unwrap();
    OperationProfile {
        source: artifact.pin.source.clone(),
        scope: source.scope().node_id(),
        declaration: source.declaration().node_id(),
        kind,
        representation: if kind == DatatypeKind::List {
            ValueRepresentation::List(ScalarRepresentation::String)
        } else {
            ValueRepresentation::Scalar(ScalarRepresentation::String)
        },
        dependencies: BTreeMap::from([(role, artifact.pin.clone())]),
    }
}
fn bundle() -> (
    BTreeMap<String, OperationProfile>,
    Vec<native::BuiltNativeRequirement>,
) {
    let mut artifacts = vec![];
    let mut profiles = BTreeMap::new();
    for (mode, name, id) in descriptors().into_iter().chain(additions()) {
        let role = match mode {
            OperationMode::Preparation(_) => Role::Preparation,
            OperationMode::Conversion => Role::Conversion,
            OperationMode::Equality => Role::Equality,
            OperationMode::ListSerializer => Role::ListSerializer,
            OperationMode::Constant => Role::Constant,
            OperationMode::FacetProfile(_) => Role::FacetProfile,
            OperationMode::ListItems(_) => Role::ListItems,
        };
        let a = built(mode, name, id);
        let kind = if name == "names" {
            DatatypeKind::List
        } else {
            DatatypeKind::Scalar
        };
        let p = profile(&a, role, kind);
        let entry = profiles.entry(name.into()).or_insert(p.clone());
        entry.dependencies.extend(p.dependencies);
        artifacts.push(a);
    }
    let text = text();
    let c = capture(&text);
    let a = native::export_tokenizer(
        &c,
        &datatype(&c, "names"),
        TokenizerMode::Ready,
        SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), text.as_bytes()),
        tokenizer_id(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    profiles
        .get_mut("names")
        .unwrap()
        .dependencies
        .insert(Role::Tokenizer, a.pin.clone());
    artifacts.push(a);
    (profiles, artifacts)
}
#[test]
fn dependency_profiles_roundtrip_all_roles_with_shared_original_owners_and_no_authority() {
    let (p, artifacts) = bundle();
    let control = OperationControl::default();
    let b = profiles::export(&p, &artifacts, &hash(b"registry"), &control).unwrap();
    assert!(b.bytes.starts_with(profiles::MAGIC));
    let v = profiles::verify(&b.bytes, &b.pin, &artifacts, &control).unwrap();
    let other = profiles::verify(&b.bytes, &b.pin, &artifacts, &control).unwrap();
    assert!(!v.executable_closure_verified());
    assert_eq!(v.profile_ids().count(), 2);
    let sample = v.profile("sample").unwrap();
    let names = v.profile("names").unwrap();
    assert!(Arc::ptr_eq(
        sample.source().declaration().document(),
        names.source().declaration().document()
    ));
    assert!(!Arc::ptr_eq(
        sample.source().declaration().document(),
        other
            .profile("sample")
            .unwrap()
            .source()
            .declaration()
            .document()
    ));
    for id in v.profile_ids() {
        let entry = v.profile(id).unwrap();
        for req in entry.requirements().values() {
            assert!(Arc::ptr_eq(
                req.capture().document(),
                entry.source().declaration().document()
            ));
            let mut h = host();
            let scope = h
                .register_scope(
                    req.source().clone(),
                    Some(Default::default()),
                    ReferenceScopePolicy::schema_defaults().unwrap(),
                )
                .unwrap();
            assert!(matches!(
                h.link_native_operation(req.clone(), scope),
                Err(native::NativeLinkError::Unavailable)
            ));
        }
    }
    assert_eq!(
        b.bytes,
        profiles::export(&p, &artifacts, &hash(b"registry"), &control)
            .unwrap()
            .bytes
    );
}
#[test]
fn dependency_profiles_keep_literal_enumeration_sources_and_reject_role_or_representation_conflicts(
) {
    let a = built(OperationMode::Constant, "choice", additions().remove(0).2);
    let p = profile(&a, Role::Constant, DatatypeKind::Scalar);
    let control = OperationControl::default();
    let inputs = BTreeMap::from([("choice".into(), p.clone())]);
    let artifacts = vec![a];
    let b = profiles::export(&inputs, &artifacts, &hash(b"registry"), &control).unwrap();
    let v = profiles::verify(&b.bytes, &b.pin, &artifacts, &control).unwrap();
    assert!(
        matches!(v.profile("choice").unwrap().source().attribute("values").unwrap().node(), CemAstNode::Attribute{value:Some(s),..} if s=="red blue")
    );
    for change in 0..6 {
        let mut wrong = p.clone();
        match change {
            0 => wrong.representation = ValueRepresentation::Scalar(ScalarRepresentation::Boolean),
            1 => wrong.kind = DatatypeKind::List,
            2 => {
                let pin = wrong.dependencies.remove(&Role::Constant).unwrap();
                wrong.dependencies.insert(Role::Equality, pin);
            }
            3 => wrong.declaration += 1,
            4 => wrong.scope += 1,
            _ => wrong.source.uri.push_str(".other"),
        }
        assert!(profiles::export(
            &BTreeMap::from([("choice".into(), wrong)]),
            &artifacts,
            &hash(b"registry"),
            &control
        )
        .is_err());
    }
}
#[test]
fn dependency_profiles_refuse_missing_extra_duplicate_registry_and_source_disagreement() {
    let (mut p, mut artifacts) = bundle();
    let control = OperationControl::default();
    let b = profiles::export(&p, &artifacts, &hash(b"registry"), &control).unwrap();
    assert!(profiles::verify(&b.bytes, &b.pin, &artifacts[..1], &control).is_err());
    let extra = built(OperationMode::Constant, "choice", additions().remove(0).2);
    artifacts.push(extra);
    assert!(profiles::verify(&b.bytes, &b.pin, &artifacts, &control).is_err());
    artifacts.pop();
    artifacts.push(native::BuiltNativeRequirement {
        bytes: artifacts[0].bytes.clone(),
        pin: artifacts[0].pin.clone(),
    });
    assert!(profiles::verify(&b.bytes, &b.pin, &artifacts, &control).is_err());
    artifacts.pop();
    let mut pin = b.pin.clone();
    pin.registry_hash = hash(b"other");
    assert!(profiles::verify(&b.bytes, &pin, &artifacts, &control).is_err());
    p.insert("duplicate".into(), p["sample"].clone());
    assert!(profiles::export(&p, &artifacts, &hash(b"registry"), &control).is_err());
    p.remove("duplicate");
    // Same logical URI, different original bytes, even with valid individual pins.
    let t = text().replace("@namespace=urn:example", "@namespace=urn:changed");
    let c = capture(&t);
    let a = native::export_operation(
        &c,
        &datatype(&c, "sample"),
        OperationMode::Constant,
        SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), t.as_bytes()),
        additions().remove(0).2,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    p.get_mut("sample")
        .unwrap()
        .dependencies
        .insert(Role::Constant, a.pin.clone());
    let index = artifacts
        .iter()
        .position(|a| {
            matches!(
                native::verify(&a.bytes, &a.pin, &control)
                    .unwrap()
                    .implementation()
                    .signature,
                NativeSignature::Constant(_)
            )
        })
        .unwrap();
    artifacts[index] = a;
    assert!(profiles::export(&p, &artifacts, &hash(b"registry"), &control).is_err());
}
#[test]
fn dependency_profiles_keep_source_backed_contracts_without_result_adapter_authority() {
    for (selected, id) in source_descriptors() {
        let t = source_text(selected);
        let c = capture(&t);
        let a = native::export_source_operation(
            &c,
            &datatype(&c, "sample"),
            &node(&c, "behavior"),
            selected,
            SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), t.as_bytes()),
            id,
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap();
        let role = match selected {
            SourceProfile::Conversion => Role::Conversion,
            SourceProfile::Equality => Role::Equality,
            SourceProfile::Constant => Role::Constant,
        };
        let p = BTreeMap::from([("source".into(), profile(&a, role, DatatypeKind::Scalar))]);
        let control = OperationControl::default();
        let artifacts = [a];
        let b = profiles::export(&p, &artifacts, &hash(b"registry"), &control).unwrap();
        let v = profiles::verify(&b.bytes, &b.pin, &artifacts, &control).unwrap();
        let r = v
            .profile("source")
            .unwrap()
            .requirements()
            .get(&role)
            .unwrap();
        assert!(matches!(
            r.implementation().signature,
            NativeSignature::SourceConversion(_)
                | NativeSignature::SourceEquality(_)
                | NativeSignature::SourceConstant(_)
        ));
        for occurrence in c.occurrences() {
            assert_eq!(
                rmp_serde::to_vec_named(&c.snapshot(c.document(), occurrence)).unwrap(),
                rmp_serde::to_vec_named(&r.capture().snapshot(r.capture().document(), occurrence))
                    .unwrap()
            );
        }
    }
}
#[test]
fn dependency_profiles_reject_repinned_malformed_versions_limits_and_cancellation() {
    let (p, mut a) = bundle();
    let control = OperationControl::default();
    let b = profiles::export(&p, &a, &hash(b"registry"), &control).unwrap();
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    struct Wire {
        abi: String,
        ml_version: String,
        ql_version: String,
        registry_hash: String,
        profiles: BTreeMap<String, OperationProfile>,
    }
    let body: Wire = rmp_serde::from_slice(&b.bytes[profiles::MAGIC.len()..]).unwrap();
    assert_eq!(
        rmp_serde::to_vec_named(&body).unwrap(),
        &b.bytes[profiles::MAGIC.len()..]
    );
    for field in [
        "abi",
        "ml_version",
        "ql_version",
        "registry_hash",
        "empty",
        "invalid-id",
        "wrong-role",
    ] {
        let mut changed = body.clone();
        match field {
            "abi" => changed.abi = "unsupported".into(),
            "ml_version" => changed.ml_version = "unsupported".into(),
            "ql_version" => changed.ql_version = "unsupported".into(),
            "registry_hash" => changed.registry_hash = hash(b"other"),
            "empty" => changed.profiles.clear(),
            "invalid-id" => {
                let entry = changed.profiles.remove("sample").unwrap();
                changed.profiles.insert("invalid/profile".into(), entry);
            }
            _ => {
                let p = changed.profiles.get_mut("sample").unwrap();
                let pin = p.dependencies.remove(&Role::Constant).unwrap();
                p.dependencies.insert(Role::Tokenizer, pin);
            }
        }
        let bytes = [profiles::MAGIC, &rmp_serde::to_vec_named(&changed).unwrap()].concat();
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(
            profiles::verify(&bytes, &pin, &a, &control).is_err(),
            "{field}"
        );
    }
    for bytes in [
        vec![0; 64 * 1024 + 1],
        [b.bytes.as_slice(), &[0]].concat(),
        [b"CEMOP2\n", &b.bytes[profiles::MAGIC.len()..]].concat(),
        [profiles::MAGIC, &vec![0x91; 80], &[0xc0]].concat(),
    ] {
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(profiles::verify(&bytes, &pin, &a, &control).is_err());
    }
    #[derive(serde::Serialize)]
    struct Extra {
        #[serde(flatten)]
        capsule: Wire,
        unknown: bool,
    }
    let unknown = [
        profiles::MAGIC,
        &rmp_serde::to_vec_named(&Extra {
            capsule: body,
            unknown: true,
        })
        .unwrap(),
    ]
    .concat();
    let mut unsupported_role = b.bytes.clone();
    let at = unsupported_role
        .windows(9)
        .position(|s| s == b"Tokenizer")
        .unwrap();
    unsupported_role[at..at + 9].copy_from_slice(b"Invalidxx");
    for bytes in [unknown, unsupported_role] {
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(profiles::verify(&bytes, &pin, &a, &control).is_err());
    }
    a[0].bytes.push(0);
    assert!(profiles::verify(&b.bytes, &b.pin, &a, &control).is_err());
    let stopped = OperationControl::default();
    stopped.cancel_root(None, None).unwrap();
    assert!(profiles::export(&p, &a, &hash(b"registry"), &stopped).is_err());
    assert!(profiles::verify(&b.bytes, &b.pin, &a, &stopped).is_err());
}

#[test]
fn dependency_profiles_use_explicit_live_registrations_and_release_retired_callback_owners() {
    let (p, artifacts) = bundle();
    let control = OperationControl::default();
    let engine = Engine::default();
    let weak = Arc::downgrade(&engine.owner);
    let b = profiles::export(&p, &artifacts, &hash(b"registry"), &control).unwrap();
    let v = profiles::verify(&b.bytes, &b.pin, &artifacts, &control).unwrap();
    let entry = v.profile("sample").unwrap();
    let root = entry.requirements().values().next().unwrap();
    let mut h = host();
    for r in entry.requirements().values() {
        register_additional(&mut h, r.implementation().clone(), engine.clone()).unwrap();
    }
    let scope = h
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let mut registrations = implementations(root);
    let mut guards = vec![];
    for r in entry.requirements().values() {
        let linked = h.link_native_operation(r.clone(), scope).unwrap();
        assert!(matches!(
            linked.install(&datatype(&capture(&text()), "sample"), &mut registrations),
            Err(native::NativeLinkError::InvalidSource)
        ));
        linked.install(entry.source(), &mut registrations).unwrap();
        guards.push(linked);
    }
    assert!(engine.calls.iter().all(|n| n.load(Ordering::Acquire) == 0));
    let compilation = compile(root, &["choice"], &registrations);
    let choice = compiled(&compilation, "choice");
    assert_eq!(choice.enumerations()[0].constants().len(), 2);
    assert!(choice.enumerations()[0]
        .constants()
        .iter()
        .all(|c| Arc::ptr_eq(c.token.source.document(), root.source().ast_owner())));
    assert_eq!(engine.calls[0].load(Ordering::Acquire), 2);
    for guard in guards {
        guard.retire();
    }
    assert!(!compile(root, &["choice"], &registrations).is_ready());
    drop(h);
    drop(engine);
    assert!(
        weak.upgrade().is_none(),
        "inert profiles and compiled proxies must not retain host code"
    );
}
#[test]
fn dependency_profiles_bound_collections_and_reject_repinned_nested_trailing_data() {
    let (p, mut a) = bundle();
    let control = OperationControl::default();
    for count in [0, 65] {
        let mut too_many = BTreeMap::new();
        for n in 0..count {
            too_many.insert(format!("p{n}"), p["sample"].clone());
        }
        assert!(profiles::export(&too_many, &a, &hash(b"registry"), &control).is_err());
    }
    let b = profiles::export(&p, &a, &hash(b"registry"), &control).unwrap();
    let excessive = (0..65)
        .map(|_| native::BuiltNativeRequirement {
            bytes: a[0].bytes.clone(),
            pin: a[0].pin.clone(),
        })
        .collect::<Vec<_>>();
    assert!(profiles::verify(&b.bytes, &b.pin, &excessive, &control).is_err());
    let total = (0..64)
        .map(|_| native::BuiltNativeRequirement {
            bytes: vec![0; 64 * 1024],
            pin: a[0].pin.clone(),
        })
        .collect::<Vec<_>>();
    assert!(profiles::verify(&b.bytes, &b.pin, &total, &control).is_err());
    a[0].bytes.push(0);
    a[0].pin.hash = hash(&a[0].bytes);
    let mut p = p;
    p.get_mut("sample")
        .unwrap()
        .dependencies
        .insert(Role::Preparation, a[0].pin.clone());
    assert!(profiles::export(&p, &a, &hash(b"registry"), &control).is_err());
}

#[test]
fn dependency_profiles_preserve_original_constant_children_and_refuse_reference_slots() {
    for body in ["{constant @value=red}", "{#colors}"] {
        let t = text().replace(
            "{type @name=sample @kind=scalar}",
            &format!("{{type @name=sample @kind=scalar | {body}}}"),
        );
        let c = capture(&t);
        let control = OperationControl::default();
        let a = native::export_operation(
            &c,
            &datatype(&c, "sample"),
            OperationMode::Constant,
            SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), t.as_bytes()),
            additions().remove(0).2,
            &hash(b"registry"),
            &control,
        );
        if body == "{#colors}" {
            assert!(a.is_err());
            continue;
        }
        let a = a.unwrap();
        let p = BTreeMap::from([(
            "constants".into(),
            profile(&a, Role::Constant, DatatypeKind::Scalar),
        )]);
        let artifacts = [a];
        let b = profiles::export(&p, &artifacts, &hash(b"registry"), &control).unwrap();
        let v = profiles::verify(&b.bytes, &b.pin, &artifacts, &control).unwrap();
        let source = v.profile("constants").unwrap().source();
        assert_eq!(source.plan().constant_slots.len(), 1);
        assert_eq!(
            source.plan().constant_slots[0].node_id(),
            datatype(&c, "sample").plan().constant_slots[0].node_id()
        );
        assert!(!v.executable_closure_verified());
    }
}
#[test]
fn dependency_profiles_refuse_different_snapshots_even_under_identical_declared_source_identity() {
    let (mut p, mut artifacts) = bundle();
    let control = OperationControl::default();
    let t = text().replace("@namespace=urn:example", "@namespace=urn:changed");
    let c = capture(&t);
    let a = native::export_operation(
        &c,
        &datatype(&c, "sample"),
        OperationMode::Constant,
        SourceIdentity::new(
            "file:///apps/cart/types.cem",
            SourceId(1),
            text().as_bytes(),
        ),
        additions().remove(0).2,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assert_eq!(
        rmp_serde::to_vec_named(&a.pin.source).unwrap(),
        rmp_serde::to_vec_named(&artifacts[0].pin.source).unwrap()
    );
    let old = p
        .get_mut("sample")
        .unwrap()
        .dependencies
        .insert(Role::Constant, a.pin.clone())
        .unwrap();
    let index = artifacts
        .iter()
        .position(|a| a.pin.hash == old.hash)
        .unwrap();
    artifacts[index] = a;
    assert!(profiles::export(&p, &artifacts, &hash(b"registry"), &control).is_err());
}
