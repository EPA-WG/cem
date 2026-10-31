//! Exact CEMOS1 pins. CEMOP1 and its closed CEMNR1 inputs remain separate.
use super::*;
use crate::native_registrations::dependency_sources::{
    BuiltOperationSource, OperationSourcePin, VerifiedOperationSource,
};
pub const MAGIC: &[u8] = b"CEMOP2\n";
const FORMAT: shared::Format = shared::Format {
    magic: MAGIC,
    abi: "cem.native-operation-profiles/2",
    independent_sources: true,
};
pub type OperationProfile = super::OperationProfileRecord<OperationSourcePin>;
pub type VerifiedOperationProfile = super::VerifiedOperationProfileView<VerifiedOperationSource>;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationProfilesPin {
    pub hash: String,
    pub registry_hash: String,
}
pub struct BuiltOperationProfiles {
    pub bytes: Vec<u8>,
    pub pin: OperationProfilesPin,
}
pub struct VerifiedOperationProfiles {
    pin: OperationProfilesPin,
    profiles: BTreeMap<String, VerifiedOperationProfile>,
}
impl VerifiedOperationProfiles {
    pub fn pin(&self) -> &OperationProfilesPin {
        &self.pin
    }
    pub fn profile_ids(&self) -> impl Iterator<Item = &str> {
        self.profiles.keys().map(String::as_str)
    }
    pub fn profile(&self, id: &str) -> Option<&VerifiedOperationProfile> {
        self.profiles.get(id)
    }
    pub fn selections_verified(&self) -> bool {
        false
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
}
pub fn export(
    profiles: &BTreeMap<String, OperationProfile>,
    artifacts: &[BuiltOperationSource],
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltOperationProfiles, String> {
    let bytes = shared::export(profiles, artifacts, registry_hash, control, FORMAT)?;
    let pin = OperationProfilesPin {
        hash: hash(&bytes),
        registry_hash: registry_hash.into(),
    };
    Ok(BuiltOperationProfiles { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &OperationProfilesPin,
    artifacts: &[BuiltOperationSource],
    control: &OperationControl,
) -> Result<VerifiedOperationProfiles, String> {
    let profiles = shared::verify(
        bytes,
        &pin.hash,
        &pin.registry_hash,
        artifacts,
        control,
        FORMAT,
    )?;
    Ok(VerifiedOperationProfiles {
        pin: pin.clone(),
        profiles,
    })
}

pub(crate) fn share_module_source(
    profile: &VerifiedOperationProfile,
    identity: &SourceIdentity,
    capture: &Arc<LexicallyScopedDocument>,
    source: &Arc<RetainedCemTree>,
    control: &OperationControl,
) -> Result<VerifiedOperationProfile, String> {
    let mut requirements = BTreeMap::new();
    for (role, original) in profile.requirements() {
        requirements.insert(
            *role,
            Arc::new(original.share_module_source(identity, capture, source, control)?),
        );
    }
    let datatype = requirements
        .values()
        .next()
        .ok_or(INVALID)?
        .datatype_source()
        .clone();
    Ok(super::VerifiedOperationProfileView {
        source: datatype,
        kind: profile.kind(),
        representation: profile.representation(),
        requirements,
    })
}
