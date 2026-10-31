//! Inert intended dependencies of original datatype fields. A checked target pin
//! is neither a resolved native selection nor a grant to execute that target.
use crate::{
    datatype_source_capsule::{
        self as datatype, BuiltDatatypeSources, DatatypeSourcesPin, VerifiedDatatypeSources,
    },
    native_registrations::{
        self as native, BuiltNativeRequirement, NativeRequirementPin, VerifiedNativeRequirement,
    },
    query_validator_capsule::declarations::{
        self as query, BuiltQueryDeclaration, QueryDeclarationPin, VerifiedQueryDeclaration,
    },
};
use cem_ml::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{tree::RetainedCemTree, AstNodeId},
    schema::{
        datatype_registry::DatatypeSource, machine::LexicallyScopedDocument,
        native_source::dependencies::NativeDependencySourceSnapshot,
        structural_capsule::SourceIdentity,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod graph;
mod selection_inputs;
pub const MAGIC: &[u8] = b"CEMDP1\n";
const ABI: &str = "cem.native-datatype-dependencies/1";
const INVALID: &str = "native datatype dependencies refused";
const MAX_BYTES: usize = 64 * 1024;
fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}
fn valid_hash(s: &str) -> bool {
    s.strip_prefix("cem-bin/1+blake3:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn encode(value: &impl Serialize) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(value).map_err(|_| INVALID.into())
}
struct Work<'a> {
    control: &'a OperationControl,
    left: Cell<usize>,
}
impl<'a> Work<'a> {
    fn new(control: &'a OperationControl) -> Self {
        Self {
            control,
            left: Cell::new(100_000),
        }
    }
    fn check(&self) -> Result<(), String> {
        self.left
            .set(self.left.get().checked_sub(1).ok_or(INVALID)?);
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID.into())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatatypeDependenciesPin {
    pub hash: String,
    pub source: SourceIdentity,
    pub registry_hash: String,
}
pub struct BuiltDatatypeDependencies {
    pub bytes: Vec<u8>,
    pub pin: DatatypeDependenciesPin,
}
/// Source-attributed intended selections. The actual host must independently
/// resolve every field and compare the original target before compilation.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum DependencyTarget {
    LocalDatatype(AstNodeId),
    Datatype {
        pin: DatatypeSourcesPin,
        root: AstNodeId,
    },
    Query(QueryDeclarationPin),
    Native(NativeRequirementPin),
}
impl DependencyTarget {
    fn external(&self) -> Option<&str> {
        match self {
            Self::LocalDatatype(_) => None,
            Self::Datatype { pin, .. } => Some(&pin.hash),
            Self::Query(pin) => Some(&pin.hash),
            Self::Native(pin) => Some(&pin.hash),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum ArtifactPin {
    Datatype(DatatypeSourcesPin),
    Query(QueryDeclarationPin),
    Native(NativeRequirementPin),
}
impl ArtifactPin {
    fn hash(&self) -> &str {
        match self {
            Self::Datatype(p) => &p.hash,
            Self::Query(p) => &p.hash,
            Self::Native(p) => &p.hash,
        }
    }
    fn registry(&self) -> &str {
        match self {
            Self::Datatype(p) => &p.registry_hash,
            Self::Query(p) => &p.program.registry_hash,
            Self::Native(p) => &p.registry_hash,
        }
    }
}
pub struct DependencyArtifact {
    bytes: Vec<u8>,
    pin: ArtifactPin,
}
impl DependencyArtifact {
    pub fn datatype(value: BuiltDatatypeSources) -> Self {
        Self {
            bytes: value.bytes,
            pin: ArtifactPin::Datatype(value.pin),
        }
    }
    pub fn query(value: BuiltQueryDeclaration) -> Self {
        Self {
            bytes: value.bytes,
            pin: ArtifactPin::Query(value.pin),
        }
    }
    pub fn native(value: BuiltNativeRequirement) -> Self {
        Self {
            bytes: value.bytes,
            pin: ArtifactPin::Native(value.pin),
        }
    }
    pub fn id(&self) -> &str {
        self.pin.hash()
    }
}
enum VerifiedArtifact {
    Datatype(VerifiedDatatypeSources),
    Query(VerifiedQueryDeclaration),
    Native(VerifiedNativeRequirement),
}
impl VerifiedArtifact {
    fn source(&self) -> &SourceIdentity {
        match self {
            Self::Datatype(a) => &a.pin().source,
            Self::Query(a) => &a.pin().program.source,
            Self::Native(a) => &a.pin().source,
        }
    }
    fn capture(&self) -> &Arc<LexicallyScopedDocument> {
        match self {
            Self::Datatype(a) => a.capture(),
            Self::Query(a) => a.capture(),
            Self::Native(a) => a.capture(),
        }
    }
}
/// Original source occurrences, not a runtime-ready descriptor or permission.
pub struct DependencySite {
    pub datatype: AstNodeId,
    pub field: AstNodeId,
    pub reference: Option<AstNodeId>,
    pub role: String,
    pub target: DependencyTarget,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    identity: SourceIdentity,
    registry_hash: String,
    roots: BTreeSet<AstNodeId>,
    bindings: BTreeMap<AstNodeId, DependencyTarget>,
    source: NativeDependencySourceSnapshot,
}
pub struct VerifiedDatatypeDependencies {
    pin: DatatypeDependenciesPin,
    roots: BTreeSet<AstNodeId>,
    capture: Arc<LexicallyScopedDocument>,
    source: Arc<RetainedCemTree>,
    sources: BTreeMap<AstNodeId, DatatypeSource>,
    sites: Vec<DependencySite>,
    order: Vec<AstNodeId>,
    _dependencies: BTreeMap<String, VerifiedArtifact>,
}
impl VerifiedDatatypeDependencies {
    pub fn pin(&self) -> &DatatypeDependenciesPin {
        &self.pin
    }
    pub fn capture(&self) -> &Arc<LexicallyScopedDocument> {
        &self.capture
    }
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        &self.source
    }
    pub fn sources(&self) -> &BTreeMap<AstNodeId, DatatypeSource> {
        &self.sources
    }
    pub fn sites(&self) -> &[DependencySite] {
        &self.sites
    }
    pub fn dependency_order(&self) -> &[AstNodeId] {
        &self.order
    }
    pub fn selections_verified(&self) -> bool {
        false
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
}
pub fn export(
    capture: &LexicallyScopedDocument,
    identity: SourceIdentity,
    registry_hash: &str,
    roots: BTreeSet<AstNodeId>,
    bindings: BTreeMap<AstNodeId, DependencyTarget>,
    artifacts: &[DependencyArtifact],
    control: &OperationControl,
) -> Result<BuiltDatatypeDependencies, String> {
    let work = Work::new(control);
    work.check()?;
    if !valid_hash(registry_hash) || bindings.len() > 128 {
        return Err(INVALID.into());
    }
    let source = NativeDependencySourceSnapshot::capture(capture, &identity, &mut || work.check())?;
    let capsule = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        identity: identity.clone(),
        registry_hash: registry_hash.into(),
        roots,
        bindings,
        source,
    };
    let bytes = [MAGIC, &encode(&capsule)?].concat();
    let pin = DatatypeDependenciesPin {
        hash: hash(&bytes),
        source: identity,
        registry_hash: registry_hash.into(),
    };
    verify(&bytes, &pin, artifacts, control)?;
    Ok(BuiltDatatypeDependencies { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &DatatypeDependenciesPin,
    artifacts: &[DependencyArtifact],
    control: &OperationControl,
) -> Result<VerifiedDatatypeDependencies, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&pin.hash)
        || !valid_hash(&pin.registry_hash)
        || hash(bytes) != pin.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 16_384, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let capsule = Capsule::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&capsule)? != body
        || capsule.abi != ABI
        || capsule.ml_version != cem_ml::VERSION
        || capsule.ql_version != crate::VERSION
        || capsule.identity != pin.source
        || capsule.registry_hash != pin.registry_hash
        || capsule.bindings.len() > 128
    {
        return Err(INVALID.into());
    }
    let capture = capsule.source.restore(&pin.source, &mut || work.check())?;
    let required = capsule
        .bindings
        .values()
        .filter_map(DependencyTarget::external)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if artifacts.len() > 64
        || artifacts.len() != required.len()
        || artifacts
            .iter()
            .map(|a| a.bytes.len())
            .try_fold(bytes.len(), |sum, n| sum.checked_add(n))
            .is_none_or(|n| n > 4 * 1024 * 1024)
    {
        return Err(INVALID.into());
    }
    let mut dependencies = BTreeMap::new();
    let mut snapshots = BTreeMap::from([(
        pin.source.uri.clone(),
        (pin.source.clone(), hash(&encode(&capsule.source)?)),
    )]);
    for artifact in artifacts {
        work.check()?;
        let id = artifact.id();
        if !required.contains(id)
            || dependencies.contains_key(id)
            || artifact.pin.registry() != pin.registry_hash
        {
            return Err(INVALID.into());
        }
        let verified = match &artifact.pin {
            ArtifactPin::Datatype(p) => {
                VerifiedArtifact::Datatype(datatype::verify(&artifact.bytes, p, control)?)
            }
            ArtifactPin::Query(p) => {
                VerifiedArtifact::Query(query::verify(&artifact.bytes, p, control)?)
            }
            ArtifactPin::Native(p) => {
                VerifiedArtifact::Native(native::verify(&artifact.bytes, p, control)?)
            }
        };
        let snapshot = NativeDependencySourceSnapshot::capture(
            verified.capture(),
            verified.source(),
            &mut || work.check(),
        )?;
        let entry = (verified.source().clone(), hash(&encode(&snapshot)?));
        if snapshots
            .get(&verified.source().uri)
            .is_some_and(|previous| previous != &entry)
        {
            return Err(INVALID.into());
        }
        snapshots.insert(verified.source().uri.clone(), entry);
        dependencies.insert(id.to_owned(), verified);
    }
    let (sources, sites, order) = graph::inspect(
        &capture,
        &capsule.roots,
        &capsule.bindings,
        &dependencies,
        &work,
    )?;
    let source = RetainedCemTree::from_shared(
        capture.document().clone(),
        &pin.source.uri,
        "",
        Default::default(),
        None,
    )
    .map_err(|_| INVALID)?;
    work.check()?;
    Ok(VerifiedDatatypeDependencies {
        pin: pin.clone(),
        roots: capsule.roots,
        capture,
        source,
        sources,
        sites,
        order,
        _dependencies: dependencies,
    })
}
#[cfg(test)]
mod tests;
