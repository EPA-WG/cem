//! Counter-only native registry census. No document, callback or live handle is exported.
use serde::Serialize;
use std::cell::{Ref, RefCell, RefMut};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const MAX_SAFE: u64 = (1 << 53) - 1;
static REVISION: AtomicU64 = AtomicU64::new(1);
static EXHAUSTED: AtomicBool = AtomicBool::new(false);
static BRIDGES: AtomicU64 = AtomicU64::new(0);

pub const COMMAND_REGISTRIES: &[&str] = &[
    "capability-sessions",
    "cem-documents",
    "cemt-call-cache",
    "cemt-object-cache",
    "command-artifact-journal",
    "command-artifacts",
    "command-bridges",
    "command-operations",
    "memory-operations",
    "native-entries",
    "native-values",
    "parse-observers",
    "reference-executions",
    "reference-snapshots",
    "reference-sources",
    "reference-validations",
    "render-cache",
    "render-output",
    "resolver-read-hooks",
    "resolver-write-hooks",
    "resumable-hosts",
    "template-cache",
    "templates",
    "transform-observers",
    "validate-observers",
    "xpath-companions",
    "xpath-programs",
    "xslt-bundles",
    "xslt-components",
];

pub fn inventory_revision() -> u64 {
    REVISION.load(Ordering::SeqCst)
}
pub fn note_mutation() {
    if REVISION
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
            value.checked_add(1).filter(|next| *next <= MAX_SAFE)
        })
        .is_err()
    {
        EXHAUSTED.store(true, Ordering::SeqCst);
    }
}

/// Conservatively invalidate snapshots before every mutable registry access,
/// including in-place changes that preserve the number of retained roots.
pub struct NativeRegistry<T>(RefCell<T>);
impl<T> NativeRegistry<T> {
    pub const fn new(value: T) -> Self {
        Self(RefCell::new(value))
    }
    pub fn borrow(&self) -> Ref<'_, T> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        note_mutation();
        self.0.borrow_mut()
    }
}
impl<T: Default> Default for NativeRegistry<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// Starts synchronously at the exported async bridge, before its first poll.
/// The guard moves into the future and survives every external callback wait.
pub struct NativeActivity {
    _private: (),
}
impl NativeActivity {
    pub fn begin() -> Self {
        BRIDGES.fetch_add(1, Ordering::SeqCst);
        note_mutation();
        Self { _private: () }
    }
    pub fn active() -> u64 {
        BRIDGES.load(Ordering::SeqCst)
    }
}
impl Drop for NativeActivity {
    fn drop(&mut self) {
        BRIDGES.fetch_sub(1, Ordering::SeqCst);
        note_mutation();
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryInventory {
    pub name: String,
    pub live: u64,
    pub discardable: u64,
}
impl RegistryInventory {
    pub fn new(name: &str, live: u64, discardable: u64) -> Self {
        Self {
            name: name.into(),
            live,
            discardable,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeResourceInventory {
    pub schema_version: u8,
    pub profile: &'static str,
    pub revision: u64,
    pub complete: bool,
    pub blocking_resources: u64,
    pub registries: Vec<RegistryInventory>,
    pub replacement_allowed: bool,
}
impl RuntimeResourceInventory {
    pub fn collect(mut entries: Vec<RegistryInventory>) -> Result<Self, &'static str> {
        if EXHAUSTED.load(Ordering::SeqCst) || entries.len() != COMMAND_REGISTRIES.len() {
            return Err("cem.runtime.inventory.incomplete");
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let mut blocking = 0u64;
        for (entry, expected) in entries.iter().zip(COMMAND_REGISTRIES) {
            if entry.name != *expected
                || entry.live > MAX_SAFE
                || entry.discardable > MAX_SAFE
                || (entry.discardable != 0
                    && !matches!(
                        entry.name.as_str(),
                        "template-cache"
                            | "render-cache"
                            | "command-artifact-journal"
                            | "cemt-call-cache"
                            | "cemt-object-cache"
                    ))
            {
                return Err("cem.runtime.inventory.incomplete");
            }
            blocking = blocking
                .checked_add(entry.live)
                .filter(|n| *n <= MAX_SAFE)
                .ok_or("cem.runtime.inventory.limit")?;
        }
        Ok(Self {
            schema_version: 1,
            profile: "cem-ml-cem-ql-command-v1",
            revision: inventory_revision(),
            complete: true,
            blocking_resources: blocking,
            registries: entries,
            replacement_allowed: false,
        })
    }
}
