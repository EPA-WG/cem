//! Original native resource leases. Read-only counters grant no retirement authority.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeRootKind {
    Host,
    Compilation,
    Registry,
    Publication,
    Preparation,
    Evidence,
}
impl NativeRootKind {
    pub const ALL: [Self; 6] = [
        Self::Host,
        Self::Compilation,
        Self::Registry,
        Self::Publication,
        Self::Preparation,
        Self::Evidence,
    ];
    pub fn registry_name(self) -> &'static str {
        match self {
            Self::Host => "enrolled-hosts",
            Self::Compilation => "enrolled-compilations",
            Self::Registry => "enrolled-registries",
            Self::Publication => "enrolled-publications",
            Self::Preparation => "enrolled-preparations",
            Self::Evidence => "enrolled-evidence",
        }
    }
}
const MAX_RESOURCES: usize = 256;
pub struct OwnedNativeEntry {
    gate: Arc<NativeAdmission>,
}
impl Drop for OwnedNativeEntry {
    fn drop(&mut self) {
        if let Ok(mut state) = self.gate.state.lock() {
            state.active -= 1;
            mutation(&mut state);
        }
    }
}
pub struct NativeResourceLease {
    gate: Arc<NativeAdmission>,
    id: u64,
}
impl Drop for NativeResourceLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.gate.state.lock() {
            if state.resources.remove(&self.id).is_none() {
                state.uncertain = true;
            }
            mutation(&mut state);
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrolledInventory {
    pub contract: &'static str,
    pub revision: u64,
    pub closed: bool,
    pub frozen: bool,
    pub active_entries: usize,
    pub live_roots: usize,
    pub roots: BTreeMap<NativeRootKind, usize>,
    pub replacement_allowed: bool,
}
impl NativeAdmission {
    pub fn admit_owned(self: &Arc<Self>, cleanup: bool) -> Result<OwnedNativeEntry, &'static str> {
        let mut state = self.state.lock().map_err(|_| UNCERTAIN)?;
        if state.uncertain {
            return Err(UNCERTAIN);
        }
        if state.frozen {
            return Err("cem.runtime.retirement.pending");
        }
        if state.closed && !cleanup {
            return Err(CLOSED);
        }
        if state.active >= MAX_ENTRIES {
            return Err("cem.runtime.admission.limit");
        }
        state.active += 1;
        mutation(&mut state);
        Ok(OwnedNativeEntry { gate: self.clone() })
    }
    pub(super) fn enroll(
        self: &Arc<Self>,
        kind: NativeRootKind,
    ) -> Result<NativeResourceLease, &'static str> {
        let mut state = self.state.lock().map_err(|_| UNCERTAIN)?;
        if state.uncertain {
            return Err(UNCERTAIN);
        }
        if state.closed || state.frozen || !state.generation_claimed {
            return Err(CLOSED);
        }
        if state.resources.len() >= MAX_RESOURCES {
            return Err("cem.runtime.generation.resource-limit");
        }
        let id = state
            .next_resource
            .checked_add(1)
            .filter(|v| *v <= (1 << 53) - 1)
            .ok_or(UNCERTAIN)?;
        state.next_resource = id;
        state.resources.insert(id, kind);
        mutation(&mut state);
        Ok(NativeResourceLease {
            gate: self.clone(),
            id,
        })
    }
    pub fn enrolled_inventory(&self) -> Result<EnrolledInventory, &'static str> {
        let state = self.state.lock().map_err(|_| UNCERTAIN)?;
        if state.uncertain {
            return Err(UNCERTAIN);
        }
        let mut roots = BTreeMap::new();
        for kind in state.resources.values() {
            *roots.entry(*kind).or_insert(0) += 1;
        }
        Ok(EnrolledInventory {
            contract: "cem-native-enrolled-generation/1",
            revision: state.revision,
            closed: state.closed,
            frozen: state.frozen,
            active_entries: state.active,
            live_roots: state.resources.len(),
            roots,
            replacement_allowed: false,
        })
    }
}
