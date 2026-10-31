//! Separately versioned, inert operation/constant dependency profiles. Native
//! registrations retain their own source, ABI and result-contract verification.
//! No callback, adapter, host scope or executable closure is reconstructed here.
use super::*;
use crate::datatype_dependency_capsule::{encode, hash, valid_hash, Work};
use std::collections::BTreeSet;

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
pub struct OperationProfile {
    pub source: SourceIdentity,
    pub scope: u32,
    pub declaration: u32,
    /// Intended effective shape. Inheritance is still resolved by the host.
    pub kind: DatatypeKind,
    pub representation: ValueRepresentation,
    pub dependencies: BTreeMap<OperationRole, NativeRequirementPin>,
}
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    profiles: BTreeMap<String, OperationProfile>,
}
pub struct VerifiedOperationProfile {
    source: DatatypeSource,
    kind: DatatypeKind,
    representation: ValueRepresentation,
    requirements: BTreeMap<OperationRole, Arc<VerifiedNativeRequirement>>,
}
impl VerifiedOperationProfile {
    pub fn source(&self) -> &DatatypeSource {
        &self.source
    }
    pub fn kind(&self) -> DatatypeKind {
        self.kind
    }
    pub fn representation(&self) -> ValueRepresentation {
        self.representation
    }
    pub fn requirements(&self) -> &BTreeMap<OperationRole, Arc<VerifiedNativeRequirement>> {
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
fn compatible(p: &OperationProfile, role: OperationRole, r: &VerifiedNativeRequirement) -> bool {
    use NativeSignature as S;
    use OperationRole as R;
    let Some(source) = r.datatype_source() else {
        return false;
    };
    if r.pin().source != p.source
        || source.scope().node_id() != p.scope
        || source.declaration().node_id() != p.declaration
        || !source.plan().issues.is_empty()
        || !match source.plan().kind {
            DatatypeKindSource::Explicit(kind) => kind == p.kind,
            DatatypeKindSource::Inherited => true,
            _ => false,
        }
    {
        return false;
    }
    let shape = match p.kind {
        DatatypeKind::List => matches!(p.representation, ValueRepresentation::List(_)),
        DatatypeKind::Node => p.representation == ValueRepresentation::Nodes,
        _ => matches!(p.representation, ValueRepresentation::Scalar(_)),
    };
    shape
        && match (role, &r.implementation().signature) {
            (R::Tokenizer, S::TokenizerSpans) => p.kind == DatatypeKind::List,
            (R::Preparation, S::Preparation(s)) => p.kind == s.kind && p.representation == s.output,
            (R::Conversion, S::Conversion(s) | S::SourceConversion(s)) => {
                p.kind == s.kind && p.representation == s.output
            }
            (R::Equality, S::Equality(s) | S::SourceEquality(s))
            | (R::Constant, S::Constant(s) | S::SourceConstant(s)) => {
                p.representation == ValueRepresentation::Scalar(*s)
            }
            (R::ListSerializer, S::ListSerializer(s)) => {
                p.representation == ValueRepresentation::List(s.item)
            }
            (R::ListItems, S::ListItems(s)) => p.representation == ValueRepresentation::List(*s),
            (R::FacetProfile, S::FacetProfile(f)) => p.representation == f.representation(),
            _ => false,
        }
}
pub fn export(
    profiles: &BTreeMap<String, OperationProfile>,
    artifacts: &[BuiltNativeRequirement],
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltOperationProfiles, String> {
    let work = Work::new(control);
    work.check()?;
    // Bound caller-owned collections before cloning or serialization.
    if profiles.is_empty()
        || profiles.len() > MAX_PROFILES
        || artifacts.is_empty()
        || artifacts.len() > MAX_ARTIFACTS
        || !valid_hash(registry_hash)
        || profiles.iter().any(|(id, p)| {
            id.len() > 128
                || p.source.uri.len() > 4096
                || !valid_hash(&p.source.hash)
                || p.dependencies.len() > 8
                || p.dependencies.values().any(|dep| {
                    dep.source.uri.len() > 4096
                        || !valid_hash(&dep.source.hash)
                        || !valid_hash(&dep.hash)
                        || !valid_hash(&dep.registry_hash)
                })
        })
    {
        return Err(INVALID.into());
    }
    let capsule = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        profiles: profiles.clone(),
    };
    let bytes = [MAGIC, &encode(&capsule)?].concat();
    let pin = OperationProfilesPin {
        hash: hash(&bytes),
        registry_hash: registry_hash.into(),
    };
    verify(&bytes, &pin, artifacts, control)?;
    Ok(BuiltOperationProfiles { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &OperationProfilesPin,
    artifacts: &[BuiltNativeRequirement],
    control: &OperationControl,
) -> Result<VerifiedOperationProfiles, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&pin.hash)
        || !valid_hash(&pin.registry_hash)
        || hash(bytes) != pin.hash
        || artifacts.is_empty()
        || artifacts.len() > MAX_ARTIFACTS
    {
        return Err(INVALID.into());
    }
    let total = artifacts.iter().try_fold(bytes.len(), |sum, a| {
        work.check()?;
        // Preserve each nested CEMNR1 bound before any leaf decoding.
        if a.bytes.len() > super::MAX_BYTES {
            return Err(INVALID.to_owned());
        }
        sum.checked_add(a.bytes.len())
            .filter(|n| *n <= MAX_TOTAL_BYTES)
            .ok_or(INVALID.to_owned())
    })?;
    if total > MAX_TOTAL_BYTES {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 100_000, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let c = Capsule::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&c)? != body
        || c.abi != ABI
        || c.ml_version != cem_ml::VERSION
        || c.ql_version != crate::VERSION
        || c.registry_hash != pin.registry_hash
        || c.profiles.is_empty()
        || c.profiles.len() > MAX_PROFILES
    {
        return Err(INVALID.into());
    }
    let mut required = BTreeMap::new();
    let mut subjects = BTreeSet::new();
    for (id, p) in &c.profiles {
        work.check()?;
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            || p.dependencies.is_empty()
            || p.dependencies.len() > 8
            || !subjects.insert((p.source.uri.clone(), p.scope, p.declaration))
        {
            return Err(INVALID.into());
        }
        for dep in p.dependencies.values() {
            work.check()?;
            if dep.registry_hash != c.registry_hash
                || dep.source != p.source
                || !valid_hash(&dep.hash)
                || required.insert(dep.hash.clone(), dep).is_some()
            {
                return Err(INVALID.into());
            }
        }
    }
    if required.len() != artifacts.len() {
        return Err(INVALID.into());
    }
    let mut leaves = BTreeMap::new();
    let mut sources: BTreeMap<String, Arc<VerifiedNativeRequirement>> = BTreeMap::new();
    for a in artifacts {
        work.check()?;
        let expected = required.get(&a.pin.hash).ok_or(INVALID)?;
        // Compare the complete pin, never just its hash/URI.
        if expected.hash != a.pin.hash
            || expected.source != a.pin.source
            || expected.registry_hash != a.pin.registry_hash
            || leaves.contains_key(&a.pin.hash)
        {
            return Err(INVALID.into());
        }
        let checked = super::verify(&a.bytes, &a.pin, control)?;
        // Explicit metadata occurrence mapping after complete identity + snapshot
        // agreement. Each independently verified profile set keeps fresh owners.
        let checked = if let Some(root) = sources.get(&a.pin.source.uri) {
            checked.share_source(root, control)?
        } else {
            checked
        };
        let checked = Arc::new(checked);
        sources
            .entry(a.pin.source.uri.clone())
            .or_insert_with(|| checked.clone());
        leaves.insert(a.pin.hash.clone(), checked);
        work.check()?;
    }
    let mut profiles = BTreeMap::new();
    for (id, p) in c.profiles {
        work.check()?;
        let mut requirements = BTreeMap::new();
        for (role, dep) in &p.dependencies {
            work.check()?;
            let r = leaves.get(&dep.hash).ok_or(INVALID)?;
            if !compatible(&p, *role, r) {
                return Err(INVALID.into());
            }
            requirements.insert(*role, r.clone());
        }
        let source = requirements
            .values()
            .next()
            .and_then(|r| r.datatype_source())
            .ok_or(INVALID)?
            .clone();
        profiles.insert(
            id,
            VerifiedOperationProfile {
                source,
                kind: p.kind,
                representation: p.representation,
                requirements,
            },
        );
    }
    work.check()?;
    Ok(VerifiedOperationProfiles {
        pin: pin.clone(),
        profiles,
    })
}
