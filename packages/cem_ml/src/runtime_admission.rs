//! Monotonic native entrypoint closure. Observation and cleanup stay available.
use crate::runtime_inventory::{inventory_revision, note_mutation, RuntimeResourceInventory};
use serde::Serialize;
use std::sync::Mutex;

pub const CLOSED: &str = "cem.runtime.admission.closed";
const UNCERTAIN: &str = "cem.runtime.admission.uncertain";
const MAX_ENTRIES: usize = 256;

#[derive(Default)]
struct State {
    closed: bool,
    active: usize,
}
#[derive(Default)]
pub struct NativeAdmission {
    state: Mutex<State>,
}

pub struct NativeEntry<'a> {
    gate: &'a NativeAdmission,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeClosureValidation {
    pub schema_version: u8,
    pub closed: bool,
    pub inventory_revision: u64,
    pub replacement_allowed: bool,
}

impl NativeAdmission {
    pub fn admit(&self, cleanup: bool) -> Result<NativeEntry<'_>, &'static str> {
        let mut state = self.state.lock().map_err(|_| UNCERTAIN)?;
        if state.closed && !cleanup {
            return Err(CLOSED);
        }
        if state.active >= MAX_ENTRIES {
            return Err("cem.runtime.admission.limit");
        }
        state.active += 1;
        note_mutation();
        Ok(NativeEntry { gate: self })
    }
    pub fn close(&self) -> Result<(), &'static str> {
        let mut state = self.state.lock().map_err(|_| UNCERTAIN)?;
        if !state.closed {
            state.closed = true;
            note_mutation();
        }
        Ok(())
    }
    pub fn active_entries(&self) -> usize {
        self.state.lock().map_or(usize::MAX, |state| state.active)
    }
    pub fn ensure_idle_closed(&self) -> Result<(), &'static str> {
        let state = self.state.lock().map_err(|_| UNCERTAIN)?;
        if !state.closed {
            return Err("cem.runtime.admission.open");
        }
        if state.active != 0 {
            return Err("cem.runtime.admission.stale");
        }
        Ok(())
    }
    /// The native lock excludes new admitted entries while the original census
    /// and revision are checked. WASM registry collection performs no JS calls.
    pub fn validate(
        &self,
        inventory: &RuntimeResourceInventory,
        expected: u64,
    ) -> Result<NativeClosureValidation, &'static str> {
        let state = self.state.lock().map_err(|_| UNCERTAIN)?;
        if !state.closed {
            return Err("cem.runtime.admission.open");
        }
        if state.active != 0
            || !inventory.complete
            || inventory.blocking_resources != 0
            || inventory.replacement_allowed
            || inventory.revision != expected
            || inventory_revision() != expected
        {
            return Err("cem.runtime.admission.stale");
        }
        Ok(NativeClosureValidation {
            schema_version: 1,
            closed: true,
            inventory_revision: expected,
            replacement_allowed: false,
        })
    }
}
impl Drop for NativeEntry<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.gate.state.lock() {
            state.active -= 1;
            note_mutation();
        }
    }
}

static GATE: NativeAdmission = NativeAdmission {
    state: Mutex::new(State {
        closed: false,
        active: 0,
    }),
};
pub fn admit(cleanup: bool) -> Result<NativeEntry<'static>, &'static str> {
    GATE.admit(cleanup)
}
pub fn close() -> Result<(), &'static str> {
    GATE.close()
}
pub fn active_entries() -> usize {
    GATE.active_entries()
}
pub fn validate(
    inventory: &RuntimeResourceInventory,
    expected: u64,
) -> Result<NativeClosureValidation, &'static str> {
    GATE.validate(inventory, expected)
}

/// Explicit API/control refusal for string-returning generated entrypoints.
pub fn refusal_json(code: &str) -> String {
    serde_json::json!({ "error": { "code": code, "message": "Native admission is closed or unavailable" },
        "nodes": [], "diagnostics": [{ "code": code, "severity": "error", "message": "Native admission is closed or unavailable" }] }).to_string()
}

pub fn ensure_idle_closed() -> Result<(), &'static str> {
    GATE.ensure_idle_closed()
}
