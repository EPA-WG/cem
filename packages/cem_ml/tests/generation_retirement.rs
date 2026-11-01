use cem_ml::{
    operation_control::OperationControl,
    runtime_admission::{generation::NativeGeneration, NativeAdmission, NativeRootKind},
    runtime_inventory::{RegistryInventory, COMMAND_REGISTRIES},
};
use std::{sync::Arc, time::Duration};
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn generation() -> Arc<NativeGeneration> {
    NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap()
}
fn base() -> Vec<RegistryInventory> {
    COMMAND_REGISTRIES
        .iter()
        .map(|n| RegistryInventory::new(n, 0, 0))
        .collect()
}
#[test]
fn joined_census_counts_actual_owners_and_legacy_manifest_cannot_freeze_a_claimed_gate() {
    let _serial = SERIAL.lock().unwrap();
    let gate = Arc::new(NativeAdmission::default());
    let g = NativeGeneration::claim(gate.clone()).unwrap();
    let root = g.resource(NativeRootKind::Publication).unwrap();
    let mut incomplete = base();
    incomplete.pop();
    assert!(g.complete_inventory(incomplete).is_err());
    let census = g.complete_inventory(base()).unwrap();
    assert_eq!(census.schema_version, 2);
    assert_eq!(census.profile, "cem-ml-cem-ql-generation-v2");
    assert_eq!(census.registries.len(), COMMAND_REGISTRIES.len() + 6);
    assert_eq!(census.blocking_resources, 1);
    assert_eq!(
        census
            .registries
            .iter()
            .find(|r| r.name == "enrolled-publications")
            .unwrap()
            .live,
        1
    );
    let drain = g
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(drain.seal().is_err());
    drop(root);
    let old = cem_ml::runtime_inventory::RuntimeResourceInventory::collect(base()).unwrap();
    assert!(gate.prepare_retirement(&old, old.revision).is_err());
    let witness = drain.seal().unwrap();
    assert!(drain
        .prepare_retirement(&witness, &old, old.revision)
        .is_err());
    let mut live = base();
    live[0].live = 1;
    let blocked = g.complete_inventory(live).unwrap();
    assert!(drain
        .prepare_retirement(&witness, &blocked, blocked.revision)
        .is_err());
    let census = g.complete_inventory(base()).unwrap();
    let permit = drain
        .prepare_retirement(&witness, &census, census.revision)
        .unwrap();
    assert_eq!(permit.inventory_revision(), census.revision);
    assert!(permit.is_current());
    assert!(gate.admit_owned(true).is_err());
    assert!(g.inventory().unwrap().frozen);
    drop(permit);
    assert!(gate.admit_owned(true).is_ok());
    assert!(!g.is_open());
}
#[test]
fn joined_freeze_refuses_foreign_stale_cancelled_and_expired_original_drains() {
    let _serial = SERIAL.lock().unwrap();
    let gate = Arc::new(NativeAdmission::default());
    let a = NativeGeneration::claim(gate.clone()).unwrap();
    let b = generation();
    let control = OperationControl::default();
    let d = a
        .begin_drain(Duration::from_secs(5), control.clone())
        .unwrap();
    let other = b
        .begin_drain(Duration::from_secs(5), Default::default())
        .unwrap();
    let w = d.seal().unwrap();
    let c = b.complete_inventory(base()).unwrap();
    assert!(other.prepare_retirement(&w, &c, c.revision).is_err());
    let c = a.complete_inventory(base()).unwrap();
    let cleanup = cem_ml::runtime_admission::NativeAdmission::default();
    // A full-census mutation invalidates even an unchanged local enrollment.
    drop(cleanup.admit(true).unwrap());
    assert!(d.prepare_retirement(&w, &c, c.revision).is_err());
    // A fresh full census cannot renew an older local witness after cleanup.
    drop(gate.admit_owned(true).unwrap());
    let c = a.complete_inventory(base()).unwrap();
    assert!(d.prepare_retirement(&w, &c, c.revision).is_err());
    let w = d.seal().unwrap();
    control.cancel_root(None, None).unwrap();
    let c = a.complete_inventory(base()).unwrap();
    assert!(d.prepare_retirement(&w, &c, c.revision).is_err());
    drop(d);
    let d = a
        .begin_drain(Duration::from_millis(100), Default::default())
        .unwrap();
    let w = d.seal().unwrap();
    let c = a.complete_inventory(base()).unwrap();
    std::thread::sleep(Duration::from_millis(120));
    assert!(d.prepare_retirement(&w, &c, c.revision).is_err());
}
#[test]
fn joined_permit_remains_original_and_expiration_or_cancellation_does_not_reopen_cleanup() {
    let _serial = SERIAL.lock().unwrap();
    let g = generation();
    let failed = OperationControl::default();
    failed
        .fail_scope(
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
            cem_ml::operation_control::ControlCause::MemoryExceeded {
                requested: 1,
                charged: 2,
                limit: 1,
            },
            None,
        )
        .unwrap();
    assert!(g.begin_drain(Duration::from_secs(5), failed).is_err());
    assert!(
        g.is_open(),
        "prior failures must refuse before closure, never be cleared"
    );
    let control = OperationControl::default();
    let d = g
        .begin_drain(Duration::from_secs(5), control.clone())
        .unwrap();
    let w = d.seal().unwrap();
    let c = g.complete_inventory(base()).unwrap();
    let permit = d.prepare_retirement(&w, &c, c.revision).unwrap();
    drop(d); // The permit retains its actual original drain, not serialized metadata.
    assert!(permit.is_current());
    control.cancel_root(None, None).unwrap();
    assert!(!permit.is_current());
    assert!(g.inventory().unwrap().frozen);
    drop(permit);
    assert!(!g.inventory().unwrap().frozen);
    assert!(!g.is_open());
    let d = g
        .begin_drain(Duration::from_millis(100), Default::default())
        .unwrap();
    let w = d.seal().unwrap();
    let c = g.complete_inventory(base()).unwrap();
    let permit = d.prepare_retirement(&w, &c, c.revision).unwrap();
    std::thread::sleep(Duration::from_millis(120));
    assert!(!permit.is_current());
    assert!(g.inventory().unwrap().frozen);
    drop(permit);
    assert!(!g.inventory().unwrap().frozen);
}

#[test]
fn every_linked_entry_and_global_retirement_share_the_original_generation() {
    let _serial = SERIAL.lock().unwrap();
    let entry = cem_ml::runtime_admission::admit(false).unwrap();
    let g = cem_ml::runtime_admission::linked_generation().unwrap();
    assert!(Arc::ptr_eq(
        g,
        cem_ml::runtime_admission::linked_generation().unwrap()
    ));
    assert!(NativeGeneration::claim_current().is_err());
    assert_eq!(g.inventory().unwrap().active_entries, 1);
    let root = g.resource(NativeRootKind::Registry).unwrap();
    cem_ml::runtime_admission::close().unwrap();
    assert!(cem_ml::runtime_admission::admit(false).is_err());
    drop(entry);
    let c = g.complete_inventory(base()).unwrap();
    assert!(cem_ml::runtime_admission::prepare_retirement(&c, c.revision).is_err());
    drop(root);
    let c = g.complete_inventory(base()).unwrap();
    cem_ml::runtime_admission::validate(&c, c.revision).unwrap();
    let permit = cem_ml::runtime_admission::prepare_retirement(&c, c.revision).unwrap();
    assert!(permit.is_current());
    assert!(cem_ml::runtime_admission::admit(true).is_err());
    drop(permit);
    assert!(cem_ml::runtime_admission::admit(true).is_ok());
    assert!(cem_ml::runtime_admission::admit(false).is_err());
}
