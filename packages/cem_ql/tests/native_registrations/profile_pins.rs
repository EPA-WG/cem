use super::*;
use cem_ql::native_registrations::{
    dependency_sources as sources,
    profiles::{self, v2, OperationRole as Role},
};

fn source_text_with_references() -> String {
    text()
        .replace(
            "{type @name=sample @kind=scalar}",
            "{type @name=sample @kind=scalar @rule={#rule} | {#colors} {constant @value=red}}",
        )
        .replace("@base=sample", "@base={#item}")
        .replace("@list-base=names", "@list-base={#list}")
}
fn identity(t: &str) -> SourceIdentity {
    SourceIdentity::new("file:///apps/cart/types.cem", SourceId(1), t.as_bytes())
}
fn artifact(
    t: &str,
    name: &str,
    mode: OperationMode,
    id: ImplementationIdentity,
) -> sources::BuiltOperationSource {
    let c = capture(t);
    sources::export_operation(
        &c,
        &datatype(&c, name),
        mode,
        identity(t),
        id,
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap()
}
fn profile(
    a: &sources::BuiltOperationSource,
    role: Role,
    kind: DatatypeKind,
) -> v2::OperationProfile {
    let r = sources::verify(&a.bytes, &a.pin, &OperationControl::default()).unwrap();
    let s = r.datatype_source();
    v2::OperationProfile {
        source: a.pin.source.clone(),
        scope: s.scope().node_id(),
        declaration: s.declaration().node_id(),
        kind,
        representation: if kind == DatatypeKind::List {
            ValueRepresentation::List(ScalarRepresentation::String)
        } else {
            ValueRepresentation::Scalar(ScalarRepresentation::String)
        },
        dependencies: BTreeMap::from([(role, a.pin.clone())]),
    }
}
fn bundle() -> (
    BTreeMap<String, v2::OperationProfile>,
    Vec<sources::BuiltOperationSource>,
) {
    let t = source_text_with_references();
    let mut p = BTreeMap::new();
    let mut a = vec![];
    for (mode, name, id) in descriptors().into_iter().chain(additions()) {
        let role = match mode {
            OperationMode::Preparation(_) => Role::Preparation,
            OperationMode::Conversion => Role::Conversion,
            OperationMode::Equality => Role::Equality,
            OperationMode::Constant => Role::Constant,
            OperationMode::FacetProfile(_) => Role::FacetProfile,
            OperationMode::ListItems(_) => Role::ListItems,
            OperationMode::ListSerializer => Role::ListSerializer,
        };
        let record = artifact(&t, name, mode, id);
        let entry = profile(
            &record,
            role,
            if name == "names" {
                DatatypeKind::List
            } else {
                DatatypeKind::Scalar
            },
        );
        p.entry(name.into())
            .or_insert_with(|| entry.clone())
            .dependencies
            .extend(entry.dependencies);
        a.push(record);
    }
    let c = capture(&t);
    let tokens = sources::export_tokenizer(
        &c,
        &datatype(&c, "names"),
        TokenizerMode::Ready,
        identity(&t),
        tokenizer_id(),
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    p.get_mut("names")
        .unwrap()
        .dependencies
        .insert(Role::Tokenizer, tokens.pin.clone());
    a.push(tokens);
    (p, a)
}
#[test]
fn versioned_profile_pins_keep_all_roles_original_sources_and_fresh_owners_without_selection() {
    let (p, a) = bundle();
    let control = OperationControl::default();
    let b = v2::export(&p, &a, &hash(b"registry"), &control).unwrap();
    assert!(b.bytes.starts_with(v2::MAGIC));
    let v = v2::verify(&b.bytes, &b.pin, &a, &control).unwrap();
    let copy = v2::verify(&b.bytes, &b.pin, &a, &control).unwrap();
    assert!(!v.executable_closure_verified());
    assert!(!v.selections_verified());
    assert_eq!(v.profile_ids().count(), 2);
    let sample = v.profile("sample").unwrap();
    let list = v.profile("names").unwrap();
    assert!(Arc::ptr_eq(
        sample.source().declaration().document(),
        list.source().declaration().document()
    ));
    assert!(!Arc::ptr_eq(
        sample.source().declaration().document(),
        copy.profile("sample")
            .unwrap()
            .source()
            .declaration()
            .document()
    ));
    for id in v.profile_ids() {
        let entry = v.profile(id).unwrap();
        for source in entry.requirements().values() {
            assert!(Arc::ptr_eq(
                source.capture().document(),
                entry.source().declaration().document()
            ));
            assert!(!source.selections_verified());
            assert!(source
                .references()
                .iter()
                .all(|r| matches!(r.node(), CemAstNode::Reference { targets: None, .. })));
        }
    }
    let constant = &sample.requirements()[&Role::Constant];
    assert_eq!(constant.constant_slots().len(), 2);
    assert!(
        matches!(constant.constant_slots()[0].node(),CemAstNode::Reference{expression,..} if expression=="#colors")
    );
    assert_eq!(
        b.bytes,
        v2::export(&p, &a, &hash(b"registry"), &control)
            .unwrap()
            .bytes
    );
    let mut reversed = a;
    reversed.reverse();
    assert_eq!(
        b.bytes,
        v2::export(&p, &reversed, &hash(b"registry"), &control)
            .unwrap()
            .bytes
    );
    let reordered = v2::verify(&b.bytes, &b.pin, &reversed, &control).unwrap();
    let sample = reordered.profile("sample").unwrap();
    assert!(sample.requirements().values().all(|r| Arc::ptr_eq(
        r.capture().document(),
        sample.source().declaration().document()
    )));
}
#[test]
fn versioned_profile_pins_reuse_source_result_and_checked_replacement_contracts() {
    for (selected, id) in source_descriptors() {
        let t = source_text(selected).replace(
            "{type @name=sample @kind=scalar}",
            "{type @name=sample @kind=scalar | {#colors}}",
        );
        let c = capture(&t);
        let control = OperationControl::default();
        let a = sources::export_source_operation(
            &c,
            &datatype(&c, "sample"),
            &node(&c, "behavior"),
            selected,
            identity(&t),
            id.clone(),
            &hash(b"registry"),
            &control,
        )
        .unwrap();
        let role = match selected {
            SourceProfile::Conversion => Role::Conversion,
            SourceProfile::Equality => Role::Equality,
            SourceProfile::Constant => Role::Constant,
        };
        let p = BTreeMap::from([("source".into(), profile(&a, role, DatatypeKind::Scalar))]);
        let a = [a];
        let b = v2::export(&p, &a, &hash(b"registry"), &control).unwrap();
        let v = v2::verify(&b.bytes, &b.pin, &a, &control).unwrap();
        let r = &v.profile("source").unwrap().requirements()[&role];
        assert_eq!(r.implementation(), &id);
        assert_eq!(r.constant_slots().len(), 1);
        for occurrence in c.occurrences() {
            assert_eq!(
                rmp_serde::to_vec_named(&c.snapshot(c.document(), occurrence)).unwrap(),
                rmp_serde::to_vec_named(&r.capture().snapshot(r.capture().document(), occurrence))
                    .unwrap()
            );
        }
    }
    let t = source_text_with_references()
        .replace("@kind=scalar @rule=", "@kind=scalar @base={#base} @rule=");
    for (mode, id, role) in [
        (
            OperationMode::Preparation(PreparationMode::CheckedReplacement),
            descriptors().remove(0).2,
            Role::Preparation,
        ),
        (
            OperationMode::FacetProfile(FacetMode::CheckedReplacement),
            additions().remove(1).2,
            Role::FacetProfile,
        ),
    ] {
        let a = artifact(&t, "sample", mode, id);
        let p = BTreeMap::from([(
            "replacement".into(),
            profile(&a, role, DatatypeKind::Scalar),
        )]);
        assert!(v2::export(&p, &[a], &hash(b"registry"), &OperationControl::default()).is_ok());
    }
}
#[test]
fn versioned_profile_pins_reject_mismatched_roles_shapes_occurrences_and_subjects() {
    let (p, a) = bundle();
    let control = OperationControl::default();
    for case in 0..8 {
        let mut p = p.clone();
        let sample = p.get_mut("sample").unwrap();
        match case {
            0 => sample.representation = ValueRepresentation::Scalar(ScalarRepresentation::Boolean),
            1 => sample.kind = DatatypeKind::List,
            2 => sample.declaration += 1,
            3 => sample.scope += 1,
            4 => {
                let pin = sample.dependencies.remove(&Role::Constant).unwrap();
                sample.dependencies.insert(Role::Tokenizer, pin);
            }
            5 => sample.source.uri.push_str(".other"),
            6 => {
                let copy = sample.clone();
                p.insert("duplicate".into(), copy);
            }
            _ => {
                sample
                    .dependencies
                    .get_mut(&Role::Constant)
                    .unwrap()
                    .registry_hash = hash(b"other")
            }
        }
        assert!(
            v2::export(&p, &a, &hash(b"registry"), &control).is_err(),
            "{case}"
        );
    }
}
#[test]
fn versioned_profile_pins_check_complete_sources_beyond_matching_declared_identity() {
    for changed_identity in [false, true] {
        let (mut p, mut a) = bundle();
        let control = OperationControl::default();
        let t = source_text_with_references();
        let changed = t.replace("@namespace=urn:example", "@namespace=urn:changed");
        let c = capture(&changed);
        let replacement = sources::export_operation(
            &c,
            &datatype(&c, "sample"),
            OperationMode::Constant,
            identity(if changed_identity { &changed } else { &t }),
            additions().remove(0).2,
            &hash(b"registry"),
            &control,
        )
        .unwrap();
        let old = p
            .get_mut("sample")
            .unwrap()
            .dependencies
            .insert(Role::Constant, replacement.pin.clone())
            .unwrap();
        let at = a.iter().position(|a| a.pin.hash == old.hash).unwrap();
        a[at] = replacement;
        assert!(v2::export(&p, &a, &hash(b"registry"), &control).is_err());
    }
    // Two internally consistent profiles still cannot disagree about one URI.
    let (mut p, mut a) = bundle();
    let control = OperationControl::default();
    let t =
        source_text_with_references().replace("@namespace=urn:example", "@namespace=urn:changed");
    let other = artifact(
        &t,
        "choice",
        OperationMode::Constant,
        additions().remove(0).2,
    );
    p.insert(
        "other".into(),
        profile(&other, Role::Constant, DatatypeKind::Scalar),
    );
    a.push(other);
    assert!(v2::export(&p, &a, &hash(b"registry"), &control).is_err());
}

#[test]
fn versioned_profile_pins_keep_old_readers_and_leaf_formats_separate() {
    let t = text();
    let c = capture(&t);
    let control = OperationControl::default();
    let id = additions().remove(0).2;
    let old = native::export_operation(
        &c,
        &datatype(&c, "sample"),
        OperationMode::Constant,
        identity(&t),
        id.clone(),
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let new = artifact(&t, "sample", OperationMode::Constant, id);
    let p2 = profile(&new, Role::Constant, DatatypeKind::Scalar);
    let p1 = profiles::OperationProfile {
        source: p2.source.clone(),
        scope: p2.scope,
        declaration: p2.declaration,
        kind: p2.kind,
        representation: p2.representation,
        dependencies: BTreeMap::from([(Role::Constant, old.pin.clone())]),
    };
    let b1 = profiles::export(
        &BTreeMap::from([("closed".into(), p1)]),
        std::slice::from_ref(&old),
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let b2 = v2::export(
        &BTreeMap::from([("closed".into(), p2.clone())]),
        std::slice::from_ref(&new),
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let old_pin = profiles::OperationProfilesPin {
        hash: b2.pin.hash.clone(),
        registry_hash: b2.pin.registry_hash.clone(),
    };
    assert!(profiles::verify(&b2.bytes, &old_pin, std::slice::from_ref(&old), &control).is_err());
    let new_pin = v2::OperationProfilesPin {
        hash: b1.pin.hash.clone(),
        registry_hash: b1.pin.registry_hash.clone(),
    };
    assert!(v2::verify(&b1.bytes, &new_pin, std::slice::from_ref(&new), &control).is_err());
    // Relabelling a native requirement's pin does not turn its bytes into CEMOS1.
    let wrong = sources::BuiltOperationSource {
        bytes: old.bytes,
        pin: sources::OperationSourcePin {
            hash: old.pin.hash,
            source: old.pin.source,
            registry_hash: old.pin.registry_hash,
        },
    };
    let mut p2 = p2;
    p2.dependencies.insert(Role::Constant, wrong.pin.clone());
    assert!(v2::export(
        &BTreeMap::from([("closed".into(), p2)]),
        &[wrong],
        &hash(b"registry"),
        &control
    )
    .is_err());
    // Changing only the outer magic is also insufficient, even with a fresh hash.
    let bytes = [v2::MAGIC, &b1.bytes[profiles::MAGIC.len()..]].concat();
    let pin = v2::OperationProfilesPin {
        hash: hash(&bytes),
        registry_hash: hash(b"registry"),
    };
    assert!(v2::verify(&bytes, &pin, &[new], &control).is_err());
}
#[test]
fn versioned_profile_pins_refuse_missing_extra_duplicate_and_cancelled_inputs_with_fixed_bounds() {
    let (p, mut a) = bundle();
    let control = OperationControl::default();
    let b = v2::export(&p, &a, &hash(b"registry"), &control).unwrap();
    assert!(v2::verify(&b.bytes, &b.pin, &a[..1], &control).is_err());
    a.push(artifact(
        &source_text_with_references(),
        "choice",
        OperationMode::Constant,
        additions().remove(0).2,
    ));
    assert!(v2::verify(&b.bytes, &b.pin, &a, &control).is_err());
    a.pop();
    a.push(sources::BuiltOperationSource {
        bytes: a[0].bytes.clone(),
        pin: a[0].pin.clone(),
    });
    assert!(v2::verify(&b.bytes, &b.pin, &a, &control).is_err());
    a.pop();
    let mut pin = b.pin.clone();
    pin.registry_hash = hash(b"other");
    assert!(v2::verify(&b.bytes, &pin, &a, &control).is_err());
    for n in [0, 65] {
        let too_many = (0..n)
            .map(|i| (format!("p{i}"), p["sample"].clone()))
            .collect();
        assert!(v2::export(&too_many, &a, &hash(b"registry"), &control).is_err());
    }
    for (count, size) in [(65, 1), (64, 64 * 1024), (1, 64 * 1024 + 1)] {
        let oversized = (0..count)
            .map(|_| sources::BuiltOperationSource {
                bytes: vec![0; size],
                pin: a[0].pin.clone(),
            })
            .collect::<Vec<_>>();
        assert!(v2::verify(&b.bytes, &b.pin, &oversized, &control).is_err());
    }
    let stopped = OperationControl::default();
    stopped.cancel_root(None, None).unwrap();
    assert!(v2::export(&p, &a, &hash(b"registry"), &stopped).is_err());
    assert!(v2::verify(&b.bytes, &b.pin, &a, &stopped).is_err());
}
#[test]
fn versioned_profile_pins_refuse_repinned_manifest_versions_unknown_fields_roles_and_encodings() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wire {
        abi: String,
        ml_version: String,
        ql_version: String,
        registry_hash: String,
        profiles: BTreeMap<String, v2::OperationProfile>,
    }
    let (p, a) = bundle();
    let control = OperationControl::default();
    let b = v2::export(&p, &a, &hash(b"registry"), &control).unwrap();
    let raw = &b.bytes[v2::MAGIC.len()..];
    let body: Wire = rmp_serde::from_slice(raw).unwrap();
    assert_eq!(rmp_serde::to_vec_named(&body).unwrap(), raw);
    for case in 0..9 {
        let mut w: Wire = rmp_serde::from_slice(raw).unwrap();
        match case {
            0 => w.abi = "unsupported".into(),
            1 => w.ml_version = "unsupported".into(),
            2 => w.ql_version = "unsupported".into(),
            3 => w.registry_hash = hash(b"other"),
            4 => w.profiles.clear(),
            5 => {
                let entry = w.profiles.remove("sample").unwrap();
                w.profiles.insert("invalid/id".into(), entry);
            }
            6 => w.profiles.get_mut("sample").unwrap().dependencies.clear(),
            7 => {
                w.profiles
                    .get_mut("sample")
                    .unwrap()
                    .dependencies
                    .get_mut(&Role::Constant)
                    .unwrap()
                    .hash = "invalid".into()
            }
            _ => w.profiles.get_mut("sample").unwrap().source.hash = hash(b"other-source"),
        }
        let bytes = [v2::MAGIC, &rmp_serde::to_vec_named(&w).unwrap()].concat();
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(v2::verify(&bytes, &pin, &a, &control).is_err(), "{case}");
    }
    #[derive(serde::Serialize)]
    struct Extra {
        #[serde(flatten)]
        capsule: Wire,
        unknown: bool,
    }
    let extra = [
        v2::MAGIC,
        &rmp_serde::to_vec_named(&Extra {
            capsule: body,
            unknown: true,
        })
        .unwrap(),
    ]
    .concat();
    let mut role = b.bytes.clone();
    let at = role.windows(9).position(|s| s == b"Tokenizer").unwrap();
    role[at..at + 9].copy_from_slice(b"Invalidxx");
    for bytes in [
        extra,
        role,
        [b.bytes.as_slice(), &[0]].concat(),
        [b"CEMOP3\n", raw].concat(),
        [v2::MAGIC, &vec![0x91; 80], &[0xc0]].concat(),
        vec![0; 64 * 1024 + 1],
    ] {
        let mut pin = b.pin.clone();
        pin.hash = hash(&bytes);
        assert!(v2::verify(&bytes, &pin, &a, &control).is_err());
    }
}
#[test]
fn versioned_profile_pins_preserve_nested_source_verification_after_repinning() {
    for case in 0..3 {
        let (mut p, mut a) = bundle();
        let control = OperationControl::default();
        let old = a[0].pin.hash.clone();
        match case {
            0 => a[0].bytes.push(0),
            1 => {
                let key = b"\xa7version";
                let at = a[0]
                    .bytes
                    .windows(key.len())
                    .position(|s| s == key)
                    .unwrap()
                    + key.len();
                a[0].bytes[at] = 2;
            }
            _ => {
                let key = b"\xa8contexts";
                let at = a[0]
                    .bytes
                    .windows(key.len())
                    .position(|s| s == key)
                    .unwrap()
                    + key.len();
                assert!(a[0].bytes[at + 1] < 0x80 && a[0].bytes[at + 2] < 0x80);
                a[0].bytes[at + 2] = 0;
            }
        }
        a[0].pin.hash = hash(&a[0].bytes);
        let pin = p
            .get_mut("sample")
            .unwrap()
            .dependencies
            .get_mut(&Role::Preparation)
            .unwrap();
        assert_eq!(pin.hash, old);
        *pin = a[0].pin.clone();
        assert!(
            v2::export(&p, &a, &hash(b"registry"), &control).is_err(),
            "{case}"
        );
    }
}

#[test]
fn versioned_profile_pins_link_only_explicit_actual_host_operation_owners() {
    let (p, a) = bundle();
    let control = OperationControl::default();
    let b = v2::export(&p, &a, &hash(b"registry"), &control).unwrap();
    let graph = v2::verify(&b.bytes, &b.pin, &a, &control).unwrap();
    let root = graph
        .profile("sample")
        .unwrap()
        .requirements()
        .values()
        .next()
        .unwrap();
    let engine = Engine::default();
    let mut h = host();
    for id in graph.profile_ids() {
        for (role, r) in graph.profile(id).unwrap().requirements() {
            if *role == Role::Tokenizer {
                h.register_native_tokenizer(r.implementation().clone(), Whitespace)
                    .unwrap();
            } else {
                register_additional(&mut h, r.implementation().clone(), engine.clone()).unwrap();
            }
        }
    }
    let scope = h
        .register_scope(
            root.source().clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    let mut owners = vec![];
    let mut registry = DatatypeImplementations::default();
    for id in graph.profile_ids() {
        let profile = graph.profile(id).unwrap();
        for (role, r) in profile.requirements() {
            if *role == Role::Tokenizer {
                let owner = h.link_dependency_tokenizer(r.clone(), scope).unwrap();
                assert!(owner.is_live_on(&h));
                // The ordinary operations below retain their own explicit owners.
                owner.retire();
            } else {
                let owner = h.link_dependency_operation(r.clone(), scope).unwrap();
                owner.install(profile.source(), &mut registry).unwrap();
                owners.push(owner);
            }
        }
    }
    assert_eq!(owners.len(), 7);
    assert!(engine.calls.iter().all(|n| n.load(Ordering::Acquire) == 0));
    assert!(!graph.selections_verified());
    assert!(!graph.executable_closure_verified());
}
