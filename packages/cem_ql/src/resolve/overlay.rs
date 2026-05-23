//! Standard-library overlay map and fingerprint.

use std::collections::BTreeMap;

use super::{BindingId, QNameKey};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleUri(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OverlayFingerprint(pub String);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StdlibOverlay {
    pub map: BTreeMap<OverlayKey, BindingId>,
    pub fingerprint: Option<OverlayFingerprint>,
}

pub type OverlayMap = StdlibOverlay;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OverlayKey {
    pub module_uri: ModuleUri,
    pub name: QNameKey,
}

impl StdlibOverlay {
    pub fn with_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.fingerprint = Some(OverlayFingerprint(fingerprint.into()));
        self
    }

    pub fn insert(
        &mut self,
        module_uri: impl Into<String>,
        name: QNameKey,
        binding_id: BindingId,
    ) -> Option<BindingId> {
        self.map.insert(
            OverlayKey {
                module_uri: ModuleUri(module_uri.into()),
                name,
            },
            binding_id,
        )
    }

    pub fn lookup(&self, name: &QNameKey) -> Option<BindingId> {
        self.map
            .iter()
            .find_map(|(key, binding)| (&key.name == name).then_some(*binding))
    }
}
