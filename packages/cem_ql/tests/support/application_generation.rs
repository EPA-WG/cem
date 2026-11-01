use super::*;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission, NativeRootKind};
use std::time::Duration;
#[test]
fn application_generation_counts_original_hosts_code_registry_publications_and_receipts() {
    let gate = Arc::new(NativeAdmission::default());
    let generation = NativeGeneration::claim(gate.clone()).unwrap();
    let mut p = fixture_runtime(false, Some(&generation));
    let app = p.stage().unwrap();
    let input = p.f.value("choice");
    let invocation = app
        .receivers(&p.f.h, "cem-pub-one")
        .unwrap()
        .preparation_invocation(&mut p.f.h, "choice", input, Default::default())
        .unwrap();
    let result = invocation.prepare(&mut p.f.h).unwrap();
    let inventory = generation.inventory().unwrap();
    for kind in [
        NativeRootKind::Host,
        NativeRootKind::Compilation,
        NativeRootKind::Registry,
        NativeRootKind::Publication,
        NativeRootKind::Preparation,
        NativeRootKind::Evidence,
    ] {
        assert_eq!(inventory.roots[&kind], 1, "{kind:?}");
    }
    assert_eq!(inventory.active_entries, 0);
    assert!(!inventory.replacement_allowed);
    let drain = generation
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(!app.is_available(&p.f.h));
    assert!(invocation.prepare(&mut p.f.h).is_err());
    assert!(drain.seal().is_err());
    drop(app);
    drop(p);
    assert_eq!(drain.poll().unwrap().live_roots, 2);
    invocation.close();
    assert_eq!(drain.poll().unwrap().live_roots, 2);
    drop(invocation);
    assert_eq!(drain.poll().unwrap().live_roots, 1);
    drop(result);
    assert!(drain.seal().unwrap().is_current(&generation));
}
#[test]
fn application_generation_fences_late_callbacks_and_waits_for_real_native_entries() {
    let generation = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let mut p = fixture_runtime(false, Some(&generation));
    let app = p.stage().unwrap();
    let drain = std::rc::Rc::new(std::cell::RefCell::new(None));
    let pending = drain.clone();
    let owner = generation.clone();
    ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            let d = owner
                .begin_drain(Duration::from_secs(5), Default::default())
                .unwrap();
            assert!(d.poll().unwrap().active_entries > 0);
            assert!(d.seal().is_err());
            *pending.borrow_mut() = Some(d);
        }))
    });
    p.f.state.mode.store(8, Ordering::SeqCst);
    assert!(app
        .prepare_root(&mut p.f.h, "scalar", &lexical("late"), Default::default())
        .is_err());
    ACTION.with(|a| *a.borrow_mut() = None);
    assert_eq!(
        drain
            .borrow()
            .as_ref()
            .unwrap()
            .poll()
            .unwrap()
            .active_entries,
        0
    );
    drop(app);
    drop(p);
    assert!(drain
        .borrow()
        .as_ref()
        .unwrap()
        .seal()
        .unwrap()
        .is_current(&generation));
}
#[test]
fn application_generation_requires_fresh_original_sources_and_code_after_reimport() {
    let old = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let mut a = fixture_runtime(false, Some(&old));
    let app = a.stage().unwrap();
    let input = a.f.value("choice");
    let invocation = app
        .receivers(&a.f.h, "cem-pub-one")
        .unwrap()
        .preparation_invocation(&mut a.f.h, "choice", input, Default::default())
        .unwrap();
    let evidence = invocation.prepare(&mut a.f.h).unwrap();
    let new = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let mut b = fixture_runtime(false, Some(&new));
    assert_eq!(a.raw.bytes, b.raw.bytes);
    let fresh = b.stage().unwrap();
    let input = b.f.value("choice");
    assert!(invocation
        .validate_prepared(&mut b.f.h, &input, evidence.evidence.as_ref())
        .is_err());
    let compiled = b.f.owner.clone();
    b.f.owner = a.f.owner.clone();
    assert!(b.stage().is_err());
    b.f.owner = compiled;
    let drain = old
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(app
        .prepare_root(&mut a.f.h, "scalar", &lexical("old"), Default::default())
        .is_err());
    assert_eq!(
        fresh
            .prepare_root(&mut b.f.h, "scalar", &lexical("fresh"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
    drop(evidence);
    drop(invocation);
    drop(app);
    drop(a);
    let proof = drain.seal().unwrap();
    assert!(proof.is_current(&old));
    assert!(!proof.is_current(&new));
    assert!(new.is_open());
}
#[test]
fn application_generation_retired_handles_and_closed_hosts_remain_inventory_roots_until_drop() {
    let generation = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let mut p = fixture_runtime(false, Some(&generation));
    let app = p.stage().unwrap();
    let mut slot = install_publication::ApplicationSlot::default();
    slot.publish(&mut p.f.h, app).unwrap();
    let retained = slot.current(&p.f.h).unwrap().clone();
    slot.clear();
    assert!(!retained.is_available(&p.f.h));
    assert_eq!(
        generation.inventory().unwrap().roots[&NativeRootKind::Publication],
        1
    );
    p.registry.retire();
    p.f.owner.retire();
    p.f.h.close();
    assert_eq!(generation.inventory().unwrap().live_roots, 4);
    let drain = generation
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(drain.seal().is_err());
    drop(p);
    assert_eq!(drain.poll().unwrap().live_roots, 1);
    drop(retained);
    drop(slot);
    assert!(drain.seal().unwrap().is_current(&generation));
}
#[test]
fn application_generation_owner_loss_fences_all_enrolled_native_aliases() {
    let gate = Arc::new(NativeAdmission::default());
    let generation = NativeGeneration::claim(gate.clone()).unwrap();
    let mut p = fixture_runtime(false, Some(&generation));
    let app = p.stage().unwrap();
    drop(generation);
    assert!(!app.is_available(&p.f.h));
    assert!(p
        .f
        .owner
        .prepare_lexical(
            &p.f.h,
            p.package.parent().roots()["scalar"].datatype,
            &lexical("old"),
            Default::default()
        )
        .is_err());
    assert!(app
        .prepare_root(&mut p.f.h, "scalar", &lexical("old"), Default::default())
        .is_err());
    assert!(gate.enrolled_inventory().unwrap().live_roots > 0);
    drop(app);
    drop(p);
    assert_eq!(gate.enrolled_inventory().unwrap().live_roots, 0);
}
#[test]
fn application_generation_counts_type_resolution_and_discards_a_closing_staged_candidate() {
    let generation = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let mut p = fixture_runtime(false, Some(&generation));
    let drain = std::rc::Rc::new(std::cell::RefCell::new(None));
    let pending = drain.clone();
    let owner = generation.clone();
    SELECTOR_ACTION.with(|a| {
        *a.borrow_mut() = Some(Box::new(move || {
            if pending.borrow().is_none() {
                let d = owner
                    .begin_drain(Duration::from_secs(5), Default::default())
                    .unwrap();
                assert!(d.poll().unwrap().active_entries > 0);
                *pending.borrow_mut() = Some(d);
            }
        }))
    });
    let result = p.stage();
    SELECTOR_ACTION.with(|a| *a.borrow_mut() = None);
    assert!(result.is_err());
    assert_eq!(
        drain
            .borrow()
            .as_ref()
            .unwrap()
            .poll()
            .unwrap()
            .active_entries,
        0
    );
    drop(p);
    assert!(drain
        .borrow()
        .as_ref()
        .unwrap()
        .seal()
        .unwrap()
        .is_current(&generation));
}
