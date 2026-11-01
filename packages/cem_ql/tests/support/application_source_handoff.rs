use super::*;
use cem_ql::application_source_transport::{self as transport, SourceInput};

pub(super) fn inputs(p: &PublicationFixture) -> Vec<SourceInput> {
    p.f.source_inputs
        .iter()
        .chain(&p.source_inputs)
        .map(|s| SourceInput {
            identity: s.identity.clone(),
            tree: s.tree.clone(),
            capture: s.capture.clone(),
        })
        .collect()
}

#[test]
fn source_handoff_reimports_original_mixed_consumers_with_fresh_explicit_owners() {
    let mut p = fixture_rule(true);
    let linked = p.stage().unwrap();
    let bytes = transport::capture(&p.f.h, &linked, &inputs(&p), &p.f.control).unwrap();
    if let Some(dir) = std::env::var_os("CEM_APPLICATION_HANDOFF_FIXTURE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("application.cemah"), &bytes.bytes).unwrap();
        std::fs::write(
            dir.join("application.pin.json"),
            serde_json::to_vec(&bytes.pin).unwrap(),
        )
        .unwrap();
    }
    let restored =
        transport::verify(&bytes.bytes, &bytes.pin, &OperationControl::default()).unwrap();
    assert_eq!(restored.bytes(), bytes.bytes);
    assert_eq!(restored.claims().len(), 5); // one base, two mixed, two shared scalar roles
    assert!(!restored.matches_publication(&p.f.h, &linked));
    let (base, mixed) = restored.receivers().unwrap();
    assert_eq!(base.len(), 1);
    assert_eq!(mixed.len(), 2);
    assert!(Arc::ptr_eq(
        &mixed["cem-pub-one"].scalars["cart-count"].source,
        &mixed["cem-pub-two"].scalars["cart-count"].source
    ));
    assert!(!Arc::ptr_eq(
        &mixed["cem-pub-one"].receiver.source,
        &p.f.source.source
    ));
    drop(linked);
    drop(p);

    // The embedding supplies fresh contexts, callbacks, scopes and directed
    // grants before it constructs the actual compilation and registry owners.
    let mut fresh = fixture_sources(true, None, Some(&restored));
    let linked = fresh.stage().unwrap();
    let mut slot = install_publication::ApplicationSlot::default();
    slot.publish(&mut fresh.f.h, linked).unwrap();
    let current = slot.current(&fresh.f.h).unwrap().clone();
    assert!(restored.matches_publication(&fresh.f.h, &current));
    let items = fresh.f.value("items");
    let count = fresh.f.value("count");
    for tag in ["cem-pub-one", "cem-pub-two"] {
        let receiver = current.receivers(&fresh.f.h, tag).unwrap();
        let result = receiver
            .validate_node(&mut fresh.f.h, "items", &items)
            .unwrap();
        assert!(result.complete && !result.failed);
        assert!(
            receiver
                .validate_scalar(&mut fresh.f.h, "count", &count)
                .unwrap()
                .accepted
        );
        for field in ["choice", "names"] {
            let input = fresh.f.value(field);
            let invocation = receiver
                .preparation_invocation(&mut fresh.f.h, field, input, Default::default())
                .unwrap();
            let prepared = invocation.prepare(&mut fresh.f.h).unwrap();
            assert!(prepared.stopped.is_none(), "{:?}", prepared.stopped);
            assert!(prepared.evidence.is_some());
            let original = fresh.f.value(field);
            assert_eq!(
                invocation
                    .validate_prepared(&mut fresh.f.h, &original, prepared.evidence.as_ref())
                    .unwrap()
                    .accepted,
                Some(true)
            );
        }
    }
    let recaptured =
        transport::capture(&fresh.f.h, &current, &inputs(&fresh), &fresh.f.control).unwrap();
    assert_eq!(recaptured.bytes, bytes.bytes);
    slot.clear();
    assert!(!restored.matches_publication(&fresh.f.h, &current));
    assert!(transport::capture(&fresh.f.h, &current, &inputs(&fresh), &fresh.f.control).is_err());
}

#[test]
fn source_handoff_refuses_foreign_captures_missing_sources_changed_pins_and_disposal() {
    let mut p = fixture();
    let linked = p.stage().unwrap();
    let original = inputs(&p);
    let bytes = transport::capture(&p.f.h, &linked, &original, &p.f.control).unwrap();
    assert!(transport::capture(
        &p.f.h,
        &linked,
        &original[..original.len() - 1],
        &p.f.control
    )
    .is_err());
    let mut foreign = inputs(&p);
    foreign[0].capture = imported(SCALAR_TEXT, "file:///app/scalars.cem").1;
    assert!(transport::capture(&p.f.h, &linked, &foreign, &p.f.control).is_err());
    let mut changed = bytes.pin.clone();
    changed.application.registry_hash = hash(b"foreign");
    assert!(transport::verify(&bytes.bytes, &changed, &p.f.control).is_err());
    changed = bytes.pin.clone();
    changed.hash = hash(b"foreign");
    assert!(transport::verify(&bytes.bytes, &changed, &p.f.control).is_err());
    for n in [
        0,
        transport::MAGIC.len(),
        transport::MAGIC.len() + 3,
        bytes.bytes.len() - 1,
    ] {
        let candidate = &bytes.bytes[..n];
        let mut pin = bytes.pin.clone();
        pin.hash = hash(candidate);
        assert!(transport::verify(candidate, &pin, &p.f.control).is_err());
    }
    let oversize = vec![0; transport::MAX_BYTES + 1];
    let mut pin = bytes.pin.clone();
    pin.hash = hash(&oversize);
    assert!(transport::verify(&oversize, &pin, &p.f.control).is_err());
    assert!(linked
        .with_registry(&mut p.f.h, |_, host| transport::capture(
            host,
            &linked,
            &original,
            &p.f.control
        ))
        .unwrap()
        .is_err());
    let cancelled = OperationControl::default();
    cancelled
        .cancel_root(Some("source handoff cancelled".into()), None)
        .unwrap();
    assert!(transport::capture(&p.f.h, &linked, &original, &cancelled).is_err());
    assert!(transport::verify(&bytes.bytes, &bytes.pin, &cancelled).is_err());
    p.registry.retire();
    assert!(transport::capture(&p.f.h, &linked, &original, &p.f.control).is_err());
}

#[test]
fn source_handoff_does_not_export_contexts_or_grant_missing_owners() {
    let mut p = fixture_rule(true);
    let linked = p.stage().unwrap();
    let bytes = transport::capture(&p.f.h, &linked, &inputs(&p), &p.f.control).unwrap();
    let restored =
        transport::verify(&bytes.bytes, &bytes.pin, &OperationControl::default()).unwrap();
    let mut fresh = fixture_sources(true, None, Some(&restored));
    let staged = fresh.stage().unwrap();
    let mut slot = install_publication::ApplicationSlot::default();
    slot.publish(&mut fresh.f.h, staged).unwrap();
    let current = slot.current(&fresh.f.h).unwrap().clone();
    // A passive packet cannot keep a compilation alive after its actual owner
    // retires, and an old host cannot capture a fresh arena with equal pins.
    assert!(transport::capture(&p.f.h, &current, &inputs(&fresh), &p.f.control).is_err());
    fresh.f.owner.retire();
    assert!(!restored.matches_publication(&fresh.f.h, &current));
    assert!(transport::capture(&fresh.f.h, &current, &inputs(&fresh), &fresh.f.control).is_err());
}

#[test]
fn source_handoff_rejects_noncanonical_incomplete_and_conflicting_source_metadata() {
    use cem_ml::schema::native_source::dependencies::NativeDependencySourceSnapshot;
    use transport::{Selectors, SourceClaim, SourceRole};
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Record {
        identity: SourceIdentity,
        snapshot: NativeDependencySourceSnapshot,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Metadata {
        abi: String,
        ml: String,
        ql: String,
        claims: Vec<SourceClaim>,
        sources: BTreeMap<String, Record>,
    }
    let mut p = fixture();
    let linked = p.stage().unwrap();
    let built = transport::capture(&p.f.h, &linked, &inputs(&p), &p.f.control).unwrap();
    let at = transport::MAGIC.len();
    let end = at + 4 + u32::from_le_bytes(built.bytes[at..at + 4].try_into().unwrap()) as usize;
    for case in 0..10 {
        let mut m: Metadata = rmp_serde::from_slice(&built.bytes[end..]).unwrap();
        match case {
            0 => m.abi.push_str("future"),
            1 => m.ml.push_str("future"),
            2 => m.ql.push_str("future"),
            3 => {
                m.claims.pop();
            }
            4 => m.claims.swap(0, 1),
            5 => {
                let Selectors::Named(fields) = &mut m.claims[0].selectors else {
                    panic!()
                };
                fields.clear();
            }
            6 => m.claims[0].role = SourceRole::Base("cem-unknown".into()),
            7 => {
                let key = m.sources.keys().next().unwrap().clone();
                m.sources.remove(&key);
            }
            8 => {
                let claim = m
                    .claims
                    .iter_mut()
                    .find(|s| matches!(s.role, SourceRole::Scalar { .. }))
                    .unwrap();
                claim.schema_uri.push_str("foreign");
            }
            _ => {
                let Selectors::Named(fields) = &mut m.claims[0].selectors else {
                    panic!()
                };
                *fields.values_mut().next().unwrap() = u32::MAX;
            }
        }
        let bytes = [
            &built.bytes[..end],
            rmp_serde::to_vec_named(&m).unwrap().as_slice(),
        ]
        .concat();
        let mut pin = built.pin.clone();
        pin.hash = hash(&bytes);
        assert!(
            transport::verify(&bytes, &pin, &p.f.control).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn source_handoff_enrolls_actual_generations_and_refuses_capture_after_drain() {
    use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission};
    let generation = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let mut p = fixture_runtime(false, Some(&generation));
    let linked = p.stage().unwrap();
    let built = transport::capture(&p.f.h, &linked, &inputs(&p), &p.f.control).unwrap();
    assert_eq!(generation.inventory().unwrap().active_entries, 0);
    let restored = transport::verify(&built.bytes, &built.pin, &Default::default()).unwrap();
    let drain = generation
        .begin_drain(std::time::Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(transport::capture(&p.f.h, &linked, &inputs(&p), &p.f.control).is_err());
    assert!(drain.seal().is_err());
    drop(linked);
    drop(p);
    assert!(drain.seal().unwrap().is_current(&generation));
    let next = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let mut fresh = fixture_sources(false, Some(&next), Some(&restored));
    let app = fresh.stage().unwrap();
    assert!(restored.matches_publication(&fresh.f.h, &app));
    drop(app);
    drop(fresh);
    assert!(next
        .begin_drain(std::time::Duration::from_secs(5), Default::default())
        .unwrap()
        .seal()
        .unwrap()
        .is_current(&next));
}
