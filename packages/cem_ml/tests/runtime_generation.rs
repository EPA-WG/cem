use cem_ml::operation_control::OperationControl;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission, NativeRootKind};
use std::{sync::Arc, time::Duration};
#[test]
fn native_generation_closes_actual_admission_and_counts_retained_roots_and_work() {
    let gate = Arc::new(NativeAdmission::default());
    let generation = NativeGeneration::claim(gate.clone()).unwrap();
    assert!(NativeGeneration::claim(gate.clone()).is_err());
    let resource = generation.resource(NativeRootKind::Publication).unwrap();
    let work = generation.admit().unwrap();
    let drain = generation
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(gate.admit(false).is_err());
    assert!(generation.resource(NativeRootKind::Host).is_err());
    assert!(drain.seal().is_err());
    let inventory = drain.poll().unwrap();
    assert_eq!(inventory.active_entries, 1);
    assert_eq!(inventory.live_roots, 1);
    drop(work);
    assert_eq!(drain.poll().unwrap().active_entries, 0);
    drop(resource);
    let witness = drain.seal().unwrap();
    assert!(witness.is_current(&generation));
    let cleanup = gate.admit(true).unwrap();
    drop(cleanup);
    assert!(!witness.is_current(&generation));
    assert!(drain.seal().unwrap().is_current(&generation));
}
#[test]
fn native_generation_rejects_foreign_cancelled_expired_and_dropped_drain_authority() {
    let a = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let b = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    let control = OperationControl::default();
    let drain = a
        .begin_drain(Duration::from_secs(5), control.clone())
        .unwrap();
    let proof = drain.seal().unwrap();
    assert!(!proof.is_current(&b));
    control.cancel_root(None, None).unwrap();
    assert!(!proof.is_current(&a));
    drop(drain);
    let drain = a
        .begin_drain(Duration::from_millis(100), Default::default())
        .unwrap();
    let proof = drain.seal().unwrap();
    std::thread::sleep(Duration::from_millis(120));
    assert!(drain.poll().is_err());
    assert!(!proof.is_current(&a));
    drop(drain);
    let drain = a
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    let proof = drain.seal().unwrap();
    drop(drain);
    assert!(!proof.is_current(&a));
    assert!(a.admit().is_err());
}
#[test]
fn native_generation_bounds_real_roots_and_requires_original_drain_lifetime() {
    let generation = NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap();
    assert!(generation
        .begin_drain(Duration::ZERO, Default::default())
        .is_err());
    assert!(generation
        .begin_drain(Duration::from_secs(6), Default::default())
        .is_err());
    assert!(generation.is_open());
    let roots = (0..256)
        .map(|_| generation.resource(NativeRootKind::Evidence).unwrap())
        .collect::<Vec<_>>();
    assert!(generation.resource(NativeRootKind::Host).is_err());
    assert_eq!(generation.inventory().unwrap().live_roots, 256);
    let drain = generation
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(generation
        .begin_drain(Duration::from_secs(5), Default::default())
        .is_err());
    drop(roots);
    let proof = drain.seal().unwrap();
    drop(drain);
    assert!(!proof.is_current(&generation));
    let fresh = generation
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(fresh.seal().unwrap().is_current(&generation));
    assert!(!proof.is_current(&generation));
}
#[test]
fn native_generation_live_enrollment_blocks_even_a_forged_empty_legacy_census() {
    use cem_ml::runtime_inventory::{
        RegistryInventory, RuntimeResourceInventory, COMMAND_REGISTRIES,
    };
    let gate = Arc::new(NativeAdmission::default());
    let generation = NativeGeneration::claim(gate.clone()).unwrap();
    let root = generation.resource(NativeRootKind::Host).unwrap();
    let _drain = generation
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    // Deliberately forged caller DTO: it must not authorize a native freeze.
    let forged = RuntimeResourceInventory::collect(
        COMMAND_REGISTRIES
            .iter()
            .map(|name| RegistryInventory::new(name, 0, 0))
            .collect(),
    )
    .unwrap();
    assert!(gate.validate(&forged, forged.revision).is_err());
    assert!(gate.prepare_retirement(&forged, forged.revision).is_err());
    assert_eq!(generation.inventory().unwrap().live_roots, 1);
    drop(root);
}
#[test]
fn native_generation_owner_drop_closes_original_gate_but_leaves_cleanup_available() {
    let gate = Arc::new(NativeAdmission::default());
    let generation = NativeGeneration::claim(gate.clone()).unwrap();
    let root = generation.resource(NativeRootKind::Publication).unwrap();
    drop(generation);
    assert!(gate.admit_owned(false).is_err());
    assert!(gate.admit_owned(true).is_ok());
    assert!(NativeGeneration::claim(gate.clone()).is_err());
    assert_eq!(gate.enrolled_inventory().unwrap().live_roots, 1);
    drop(root);
    assert_eq!(gate.enrolled_inventory().unwrap().live_roots, 0);
}
