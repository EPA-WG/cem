//! Separately versioned, inert operation/constant dependency profiles. Native
//! registrations retain their own source, ABI and result-contract verification.
//! No callback, adapter, host scope or executable closure is reconstructed here.
use super::*;
use crate::datatype_dependency_capsule::{encode, hash, valid_hash, Work};
mod shared;
pub mod v2;

pub const MAGIC: &[u8] = b"CEMOP1\n";
const ABI: &str = "cem.native-operation-profiles/1";
const MAX_PROFILES: usize = 64;
const MAX_ARTIFACTS: usize = 64;
const MAX_BYTES: usize = 64 * 1024;
const MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024;
const INVALID: &str = "native operation dependency profiles refused";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OperationRole {
    Tokenizer,
    Preparation,
    Conversion,
    Equality,
    ListSerializer,
    Constant,
    FacetProfile,
    ListItems,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationProfileRecord<P> {
    pub source: SourceIdentity,
    pub scope: u32,
    pub declaration: u32,
    /// Intended effective shape. Inheritance is still resolved by the host.
    pub kind: DatatypeKind,
    pub representation: ValueRepresentation,
    pub dependencies: BTreeMap<OperationRole, P>,
}
pub type OperationProfile = OperationProfileRecord<NativeRequirementPin>;
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
pub struct VerifiedOperationProfileView<R> {
    source: DatatypeSource,
    kind: DatatypeKind,
    representation: ValueRepresentation,
    requirements: BTreeMap<OperationRole, Arc<R>>,
}
pub type VerifiedOperationProfile = VerifiedOperationProfileView<VerifiedNativeRequirement>;
impl<R> VerifiedOperationProfileView<R> {
    pub fn source(&self) -> &DatatypeSource {
        &self.source
    }
    pub fn kind(&self) -> DatatypeKind {
        self.kind
    }
    pub fn representation(&self) -> ValueRepresentation {
        self.representation
    }
    pub fn requirements(&self) -> &BTreeMap<OperationRole, Arc<R>> {
        &self.requirements
    }
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
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
}
const FORMAT: shared::Format = shared::Format {
    magic: MAGIC,
    abi: ABI,
    independent_sources: false,
};
pub fn export(
    profiles: &BTreeMap<String, OperationProfile>,
    artifacts: &[BuiltNativeRequirement],
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
    artifacts: &[BuiltNativeRequirement],
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
