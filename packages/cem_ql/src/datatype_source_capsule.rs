//! Inert original datatype sources and their exact local literal dependency graph.
//! Verification creates neither executable types nor implementation authority.
use cem_ml::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::{tree::RetainedCemTree, AstNodeId},
    schema::{
        datatype_registry::DatatypeSource, machine::LexicallyScopedDocument,
        native_source::NativeSourceSnapshot, structural_capsule::SourceIdentity,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod graph;
pub const MAGIC: &[u8] = b"CEMDT1\n";
const ABI: &str = "cem.native-datatype-sources/1";
const INVALID: &str = "native datatype sources refused";
const MAX_BYTES: usize = 64 * 1024;
fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}
fn valid_hash(value: &str) -> bool {
    value.strip_prefix("cem-bin/1+blake3:").is_some_and(|s| {
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
pub struct DatatypeSourcesPin {
    pub hash: String,
    pub source: SourceIdentity,
    pub registry_hash: String,
}
pub struct BuiltDatatypeSources {
    pub bytes: Vec<u8>,
    pub pin: DatatypeSourcesPin,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency {
    field: AstNodeId,
    role: String,
    target: AstNodeId,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypeRecord {
    name: String,
    kind: String,
    dependencies: Vec<Dependency>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    identity: SourceIdentity,
    roots: BTreeSet<AstNodeId>,
    types: BTreeMap<AstNodeId, TypeRecord>,
    source: NativeSourceSnapshot,
}
pub struct VerifiedDatatypeSources {
    pin: DatatypeSourcesPin,
    roots: BTreeSet<AstNodeId>,
    types: BTreeMap<AstNodeId, TypeRecord>,
    source: Arc<RetainedCemTree>,
    capture: Arc<LexicallyScopedDocument>,
    sources: BTreeMap<AstNodeId, DatatypeSource>,
    order: Vec<AstNodeId>,
    snapshot_hash: String,
}
impl VerifiedDatatypeSources {
    pub fn pin(&self) -> &DatatypeSourcesPin {
        &self.pin
    }
    pub fn roots(&self) -> &BTreeSet<AstNodeId> {
        &self.roots
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
    pub fn source_named(&self, name: &str) -> Option<&DatatypeSource> {
        self.types
            .iter()
            .find_map(|(id, t)| (t.name == name).then(|| &self.sources[id]))
    }
    pub fn dependency_order(&self) -> &[AstNodeId] {
        &self.order
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
    /// Share only a completely identical verified arena, never just a URI/hash
    /// claim or matching original node IDs. This is still inert source data.
    pub fn share_native_source(
        &self,
        anchor: &crate::native_registrations::VerifiedNativeRequirement,
        control: &OperationControl,
    ) -> Result<Self, String> {
        let work = Work::new(control);
        work.check()?;
        if self.pin.source != anchor.pin().source
            || self.pin.registry_hash != anchor.pin().registry_hash
            || self.snapshot_hash != anchor.source_snapshot_hash()
        {
            return Err(INVALID.into());
        }
        let (types, sources, order) = graph::inspect(anchor.capture(), &self.roots, &work)?;
        if types != self.types {
            return Err(INVALID.into());
        }
        work.check()?;
        Ok(Self {
            pin: self.pin.clone(),
            roots: self.roots.clone(),
            types,
            source: anchor.source().clone(),
            capture: anchor.capture().clone(),
            sources,
            order,
            snapshot_hash: self.snapshot_hash.clone(),
        })
    }
    pub(crate) fn bindings(
        &self,
    ) -> impl Iterator<Item = (&DatatypeSource, &str, &DatatypeSource)> {
        self.types.iter().flat_map(move |(id, t)| {
            t.dependencies.iter().map(move |dep| {
                (
                    &self.sources[id],
                    self.types[&dep.target].name.as_str(),
                    &self.sources[&dep.target],
                )
            })
        })
    }
}
pub fn export(
    capture: &LexicallyScopedDocument,
    identity: SourceIdentity,
    registry_hash: &str,
    roots: BTreeSet<AstNodeId>,
    control: &OperationControl,
) -> Result<BuiltDatatypeSources, String> {
    let work = Work::new(control);
    work.check()?;
    if !valid_hash(registry_hash) {
        return Err(INVALID.into());
    }
    let (types, _, _) = graph::inspect(capture, &roots, &work)?;
    let source = NativeSourceSnapshot::capture(capture, &identity, &mut || work.check())?;
    let capsule = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        identity: identity.clone(),
        roots,
        types,
        source,
    };
    let bytes = [MAGIC, &encode(&capsule)?].concat();
    if bytes.len() > MAX_BYTES {
        return Err(INVALID.into());
    }
    let pin = DatatypeSourcesPin {
        hash: hash(&bytes),
        source: identity,
        registry_hash: registry_hash.into(),
    };
    verify(&bytes, &pin, control)?;
    Ok(BuiltDatatypeSources { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &DatatypeSourcesPin,
    control: &OperationControl,
) -> Result<VerifiedDatatypeSources, String> {
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
    {
        return Err(INVALID.into());
    }
    let capture = capsule.source.restore(&pin.source, &mut || work.check())?;
    let (types, sources, order) = graph::inspect(&capture, &capsule.roots, &work)?;
    if types != capsule.types {
        return Err(INVALID.into());
    }
    let source = RetainedCemTree::from_shared(
        capture.document().clone(),
        &pin.source.uri,
        "",
        Default::default(),
        None,
    )
    .map_err(|_| INVALID)?;
    let snapshot_hash = hash(&encode(&capsule.source)?);
    work.check()?;
    Ok(VerifiedDatatypeSources {
        pin: pin.clone(),
        roots: capsule.roots,
        types,
        source,
        capture,
        sources,
        order,
        snapshot_hash,
    })
}

#[cfg(test)]
mod tests;
