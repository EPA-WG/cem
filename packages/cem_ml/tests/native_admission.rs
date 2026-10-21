use cem_ml::runtime_admission::NativeAdmission;
use cem_ml::runtime_inventory::{
    inventory_revision, RegistryInventory, RuntimeResourceInventory, COMMAND_REGISTRIES,
};

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn inventory() -> RuntimeResourceInventory {
    RuntimeResourceInventory::collect(
        COMMAND_REGISTRIES
            .iter()
            .map(|n| RegistryInventory::new(n, 0, 0))
            .collect(),
    )
    .unwrap()
}

#[test]
fn closure_is_monotonic_and_cleanup_remains_admitted() {
    let _serial = SERIAL.lock().unwrap();
    let gate = NativeAdmission::default();
    assert!(gate.ensure_idle_closed().is_err());
    let work = gate.admit(false).unwrap();
    assert_eq!(gate.active_entries(), 1);
    assert!(gate.ensure_idle_closed().is_err());
    gate.close().unwrap();
    gate.close().unwrap();
    assert!(gate.admit(false).is_err());
    assert!(gate.validate(&inventory(), inventory_revision()).is_err());
    drop(work);
    let snapshot = inventory();
    assert!(gate.ensure_idle_closed().is_ok());
    assert!(gate.validate(&snapshot, snapshot.revision).is_ok());
    let cleanup = gate.admit(true).unwrap();
    assert!(gate.validate(&snapshot, snapshot.revision).is_err());
    drop(cleanup);
    assert!(gate.validate(&snapshot, snapshot.revision).is_err());
    let fresh = inventory();
    let proof = gate.validate(&fresh, fresh.revision).unwrap();
    assert!(!proof.replacement_allowed);
    assert!(proof.closed);
}

#[test]
fn only_complete_idle_current_closed_inventories_can_validate() {
    let _serial = SERIAL.lock().unwrap();
    let gate = NativeAdmission::default();
    let snapshot = inventory();
    assert!(gate.validate(&snapshot, snapshot.revision).is_err());
    gate.close().unwrap();
    let mut snapshot = inventory();
    assert!(gate.validate(&snapshot, snapshot.revision + 1).is_err());
    snapshot.blocking_resources = 1;
    assert!(gate.validate(&snapshot, snapshot.revision).is_err());
    snapshot.blocking_resources = 0;
    snapshot.complete = false;
    assert!(gate.validate(&snapshot, snapshot.revision).is_err());
    snapshot.complete = true;
    assert!(gate.validate(&snapshot, snapshot.revision).is_ok());
}

#[test]
fn entry_limit_and_every_generated_export_have_explicit_native_contracts() {
    let _serial = SERIAL.lock().unwrap();
    let gate = NativeAdmission::default();
    let entries: Vec<_> = (0..256).map(|_| gate.admit(false).unwrap()).collect();
    assert!(gate.admit(false).is_err());
    gate.close().unwrap();
    drop(entries);
    assert_eq!(gate.active_entries(), 0);
    assert!(gate.admit(true).is_ok());

    // Discover modules recursively: new exports must choose an entry guard or
    // explicitly extend the bounded observation/control exception contract.
    fn sources(root: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![
        root.join("src/api/wasm.rs"),
        root.join("../cem_ql/src/api/wasm.rs"),
    ];
    sources(&root.join("src/api/wasm"), &mut files);
    sources(&root.join("../cem_ql/src/api/wasm"), &mut files);
    let exceptions = [
        "version",
        "capabilityManifest",
        "nodeWorkerCapabilityManifest",
        "browserWorkerCapabilityManifest",
        "workerProtocolDescriptor",
        "runtimeMemoryStatus",
        "templateCompilationCacheStats",
        "cemQlVersion",
        "retainedRenderCacheStats",
        "commandNativeResourceInventory",
        "commandNativeMemoryIdentity",
        "closeCommandNativeAdmission",
        "validateCommandNativeClosure",
        "prepareCommandNativeRetirement",
        "confirmation",
    ];
    let exports = regex::Regex::new(r#"(?s)#\[(?:wasm_bindgen|wasm_bindgen::prelude::wasm_bindgen)\([^\]]*js_name\s*=\s*"([^"]+)"[^\]]*\)\]\s*(?:#\[[^\]]*\]\s*)*pub fn [^{]+\{"#).unwrap();
    let all = regex::Regex::new(r#"(?s)#\[(?:wasm_bindgen|wasm_bindgen::prelude::wasm_bindgen)(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*pub fn"#).unwrap();
    let mut total = 0;
    for file in files {
        let source = std::fs::read_to_string(&file).unwrap();
        let captures: Vec<_> = exports.captures_iter(&source).collect();
        assert_eq!(
            captures.len(),
            all.find_iter(&source).count(),
            "unnamed entrypoint: {}",
            file.display()
        );
        for capture in captures {
            total += 1;
            let name = &capture[1];
            if exceptions.contains(&name) {
                continue;
            }
            let end = capture.get(0).unwrap().end();
            assert!(
                source[end..]
                    .trim_start()
                    .starts_with("let _native = match"),
                "unguarded {name}"
            );
            assert!(
                source[end..]
                    .lines()
                    .take(2)
                    .collect::<String>()
                    .contains("runtime_admission::admit("),
                "unguarded {name}"
            );
        }
    }
    assert!(total > 130);
}

#[test]
fn retirement_freezes_cleanup_until_the_original_permit_is_dropped() {
    let _serial = SERIAL.lock().unwrap();
    let gate = NativeAdmission::default();
    let snapshot = inventory();
    assert!(gate
        .prepare_retirement(&snapshot, snapshot.revision)
        .is_err());
    gate.close().unwrap();
    let old = inventory();
    drop(gate.admit(true).unwrap());
    assert!(gate.prepare_retirement(&old, old.revision).is_err());
    // Failed preparation leaves cleanup available.
    drop(gate.admit(true).unwrap());
    let current = inventory();
    let permit = gate.prepare_retirement(&current, current.revision).unwrap();
    assert_eq!(permit.inventory_revision(), current.revision);
    assert!(permit.frozen_revision() > current.revision);
    assert_eq!(
        gate.active_entries(),
        1,
        "the retirement permit remains a native root"
    );
    assert!(gate.admit(true).is_err());
    assert!(gate.admit(false).is_err());
    assert!(gate.ensure_idle_closed().is_err());
    assert!(gate
        .prepare_retirement(&inventory(), inventory_revision())
        .is_err());
    gate.close().unwrap();
    assert_eq!(inventory_revision(), permit.frozen_revision());
    drop(permit);
    assert_eq!(gate.active_entries(), 0);
    assert!(gate.admit(false).is_err());
    drop(gate.admit(true).unwrap());
    assert!(gate.prepare_retirement(&current, current.revision).is_err());
    let current = inventory();
    let permit = gate.prepare_retirement(&current, current.revision).unwrap();
    drop(permit);
}

#[test]
fn retirement_refuses_live_entries_and_incomplete_resource_handoffs() {
    let _serial = SERIAL.lock().unwrap();
    let gate = NativeAdmission::default();
    gate.close().unwrap();
    let work = gate.admit(true).unwrap();
    assert!(gate
        .prepare_retirement(&inventory(), inventory_revision())
        .is_err());
    drop(work);
    for kind in 0..3 {
        let mut snapshot = inventory();
        match kind {
            0 => snapshot.complete = false,
            1 => snapshot.blocking_resources = 1,
            _ => snapshot.replacement_allowed = true,
        }
        assert!(gate
            .prepare_retirement(&snapshot, snapshot.revision)
            .is_err());
        drop(gate.admit(true).unwrap());
    }
}
