//! Flat, inert source-backed module graphs. Verification is not runtime selection,
//! code closure, implementation registration or a receiver publication receipt.
use crate::{
    datatype_dependency_capsule::{
        encode, graph, hash, remember_source, valid_hash, verify_leaf_artifacts,
        DependencyArtifact, DependencySite, DependencyTarget, SourceAgreement, VerifiedArtifact,
        Work,
    },
    datatype_source_capsule::DatatypeSourcesPin,
    native_registrations::NativeRequirementPin,
    query_validator_capsule::declarations::QueryDeclarationPin,
};
use cem_ml::{
    operation_control::OperationControl,
    parser::{tree::RetainedCemTree, AstNodeId},
    schema::{
        datatype_registry::DatatypeSource, declaration_references::SchemaDeclarationNode,
        machine::LexicallyScopedDocument,
        native_source::dependencies::NativeDependencySourceSnapshot,
        structural_capsule::SourceIdentity,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod order;
const ABI: &str = "cem.native-datatype-modules/1";
pub const MAGIC: &[u8] = b"CEMDM1\n";
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_MODULE_BYTES: usize = 64 * 1024;
const MAX_MODULES: usize = 32;
const MAX_FIELDS: usize = 512;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatatypeModulesPin {
    pub hash: String,
    pub registry_hash: String,
    pub root: String,
}
pub struct BuiltDatatypeModules {
    pub bytes: Vec<u8>,
    pub pin: DatatypeModulesPin,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ModuleTarget {
    LocalDatatype(AstNodeId),
    ModuleDatatype {
        module: String,
        root: AstNodeId,
    },
    Datatype {
        pin: DatatypeSourcesPin,
        root: AstNodeId,
    },
    Query(QueryDeclarationPin),
    Native(NativeRequirementPin),
}
impl ModuleTarget {
    fn leaf(&self) -> Option<DependencyTarget> {
        Some(match self {
            Self::Datatype { pin, root } => DependencyTarget::Datatype {
                pin: pin.clone(),
                root: *root,
            },
            Self::Query(pin) => DependencyTarget::Query(pin.clone()),
            Self::Native(pin) => DependencyTarget::Native(pin.clone()),
            _ => return None,
        })
    }
    fn leaf_id(&self) -> Option<&str> {
        match self {
            Self::Datatype { pin, .. } => Some(&pin.hash),
            Self::Query(p) => Some(&p.hash),
            Self::Native(p) => Some(&p.hash),
            _ => None,
        }
    }
}
#[derive(Clone)]
pub struct ModuleSource {
    pub identity: SourceIdentity,
    pub capture: Arc<LexicallyScopedDocument>,
    pub roots: BTreeSet<AstNodeId>,
    pub bindings: BTreeMap<AstNodeId, ModuleTarget>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleRecord {
    identity: SourceIdentity,
    roots: BTreeSet<AstNodeId>,
    bindings: BTreeMap<AstNodeId, ModuleTarget>,
    source: NativeDependencySourceSnapshot,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    root: String,
    modules: BTreeMap<String, ModuleRecord>,
}
#[derive(Debug)]
pub struct ModuleVerificationError {
    pub code: &'static str,
    /// Original fields on a cycle or a refused source; never executable contracts.
    pub related: Vec<SchemaDeclarationNode>,
    pub cycle_modules: Vec<String>,
}
impl ModuleVerificationError {
    fn code(code: &'static str) -> Self {
        Self {
            code,
            related: vec![],
            cycle_modules: vec![],
        }
    }
}
impl From<String> for ModuleVerificationError {
    fn from(_: String) -> Self {
        Self::code("datatype-module-graph-refused")
    }
}
impl From<&str> for ModuleVerificationError {
    fn from(_: &str) -> Self {
        Self::code("datatype-module-graph-refused")
    }
}
impl std::fmt::Display for ModuleVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}
impl std::error::Error for ModuleVerificationError {}
pub struct VerifiedDatatypeModule {
    identity: SourceIdentity,
    capture: Arc<LexicallyScopedDocument>,
    source: Arc<RetainedCemTree>,
    roots: BTreeSet<AstNodeId>,
    sources: BTreeMap<AstNodeId, DatatypeSource>,
    sites: Vec<DependencySite<ModuleTarget>>,
}
impl VerifiedDatatypeModule {
    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }
    pub fn capture(&self) -> &Arc<LexicallyScopedDocument> {
        &self.capture
    }
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        &self.source
    }
    pub fn roots(&self) -> &BTreeSet<AstNodeId> {
        &self.roots
    }
    pub fn sources(&self) -> &BTreeMap<AstNodeId, DatatypeSource> {
        &self.sources
    }
    pub fn sites(&self) -> &[DependencySite<ModuleTarget>] {
        &self.sites
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleDatatypeId {
    pub module: String,
    pub datatype: AstNodeId,
}
pub struct VerifiedDatatypeModules {
    pin: DatatypeModulesPin,
    modules: BTreeMap<String, VerifiedDatatypeModule>,
    leaves: BTreeMap<String, VerifiedArtifact>,
    module_order: Vec<String>,
    datatype_order: Vec<ModuleDatatypeId>,
}
impl VerifiedDatatypeModules {
    pub fn pin(&self) -> &DatatypeModulesPin {
        &self.pin
    }
    pub fn module(&self, id: &str) -> Option<&VerifiedDatatypeModule> {
        self.modules.get(id)
    }
    pub fn module_ids(&self) -> impl Iterator<Item = &str> {
        self.modules.keys().map(String::as_str)
    }
    pub fn leaf_ids(&self) -> impl Iterator<Item = &str> {
        self.leaves.keys().map(String::as_str)
    }
    pub fn leaf_source(&self, id: &str) -> Option<&Arc<RetainedCemTree>> {
        self.leaves.get(id).map(VerifiedArtifact::tree)
    }
    pub fn leaf_capture(&self, id: &str) -> Option<&Arc<LexicallyScopedDocument>> {
        self.leaves.get(id).map(VerifiedArtifact::capture)
    }
    pub fn leaf_datatypes(&self, id: &str) -> Option<&BTreeMap<AstNodeId, DatatypeSource>> {
        match self.leaves.get(id)? {
            VerifiedArtifact::Datatype(a) => Some(a.sources()),
            _ => None,
        }
    }
    pub(crate) fn leaf(&self, id: &str) -> Option<&VerifiedArtifact> {
        self.leaves.get(id)
    }
    pub fn module_order(&self) -> &[String] {
        &self.module_order
    }
    pub fn datatype_order(&self) -> &[ModuleDatatypeId] {
        &self.datatype_order
    }
    pub fn selections_verified(&self) -> bool {
        false
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
}
fn module_name(id: &str) -> bool {
    id.len() <= 128
        && id
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
pub fn export(
    root: &str,
    modules: &BTreeMap<String, ModuleSource>,
    artifacts: &[DependencyArtifact],
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltDatatypeModules, ModuleVerificationError> {
    let work = Work::new(control);
    work.check()?;
    if modules.is_empty()
        || modules.len() > MAX_MODULES
        || !modules.contains_key(root)
        || !valid_hash(registry_hash)
    {
        return Err(ModuleVerificationError::code(
            "datatype-module-graph-refused",
        ));
    }
    let mut records = BTreeMap::new();
    for (id, m) in modules {
        work.check()?;
        if !module_name(id) {
            return Err(ModuleVerificationError::code("datatype-module-id-refused"));
        }
        let source =
            NativeDependencySourceSnapshot::capture(&m.capture, &m.identity, &mut || work.check())?;
        let record = ModuleRecord {
            identity: m.identity.clone(),
            roots: m.roots.clone(),
            bindings: m.bindings.clone(),
            source,
        };
        if encode(&record)?.len() > MAX_MODULE_BYTES {
            return Err(ModuleVerificationError::code("datatype-module-graph-limit"));
        }
        records.insert(id.clone(), record);
    }
    let capsule = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        root: root.into(),
        modules: records,
    };
    let bytes = [MAGIC, &encode(&capsule)?].concat();
    let pin = DatatypeModulesPin {
        hash: hash(&bytes),
        root: root.into(),
        registry_hash: registry_hash.into(),
    };
    verify(&bytes, &pin, artifacts, control)?;
    Ok(BuiltDatatypeModules { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &DatatypeModulesPin,
    artifacts: &[DependencyArtifact],
    control: &OperationControl,
) -> Result<VerifiedDatatypeModules, ModuleVerificationError> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&pin.hash)
        || !valid_hash(&pin.registry_hash)
        || hash(bytes) != pin.hash
    {
        return Err(ModuleVerificationError::code(
            "datatype-module-graph-refused",
        ));
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 100_000, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let c = Capsule::deserialize(&mut decoder)
        .map_err(|_| ModuleVerificationError::code("datatype-module-graph-refused"))?;
    if encode(&c)? != body
        || c.abi != ABI
        || c.ml_version != cem_ml::VERSION
        || c.ql_version != crate::VERSION
        || c.registry_hash != pin.registry_hash
        || c.root != pin.root
        || !c.modules.contains_key(&c.root)
        || c.modules.is_empty()
        || c.modules.len() > MAX_MODULES
        || c.modules
            .values()
            .try_fold(0usize, |sum, m| sum.checked_add(m.bindings.len()))
            .is_none_or(|n| n > MAX_FIELDS)
    {
        return Err(ModuleVerificationError::code(
            "datatype-module-graph-refused",
        ));
    }
    let mut agreement = SourceAgreement::new();
    let mut modules = BTreeMap::new();
    let mut catalogs = BTreeMap::new();
    let mut required = BTreeSet::new();
    for (id, m) in &c.modules {
        work.check()?;
        if !module_name(id) || encode(m)?.len() > MAX_MODULE_BYTES {
            return Err(ModuleVerificationError::code("datatype-module-graph-limit"));
        }
        let capture = m.source.restore(&m.identity, &mut || work.check())?;
        remember_source(&mut agreement, &capture, &m.identity, &work)?;
        let catalog = graph::catalog(&capture, &work)?;
        if m.roots.is_empty()
            || m.roots.len() > 32
            || m.roots.iter().any(|r| !catalog.contains_key(r))
        {
            return Err(ModuleVerificationError::code(
                "datatype-module-export-refused",
            ));
        }
        let source = RetainedCemTree::from_shared(
            capture.document().clone(),
            &m.identity.uri,
            "",
            Default::default(),
            None,
        )?;
        modules.insert(
            id.clone(),
            VerifiedDatatypeModule {
                identity: m.identity.clone(),
                capture,
                source,
                roots: m.roots.clone(),
                sources: BTreeMap::new(),
                sites: vec![],
            },
        );
        catalogs.insert(id.clone(), catalog);
        required.extend(
            m.bindings
                .values()
                .filter_map(ModuleTarget::leaf_id)
                .map(str::to_owned),
        );
    }
    let leaves = verify_leaf_artifacts(
        artifacts,
        &required,
        &pin.registry_hash,
        bytes.len(),
        &mut agreement,
        &work,
        control,
    )?;
    let mut inspections = BTreeMap::new();
    for (id, m) in &c.modules {
        let result = graph::inspect_catalog(
            &m.roots,
            &m.bindings,
            &catalogs[id],
            |target| match target {
                ModuleTarget::LocalDatatype(n) => Ok(graph::ResolvedTarget::Local(*n)),
                ModuleTarget::ModuleDatatype { module, root } => {
                    let target = modules.get(module).ok_or("unknown datatype module")?;
                    if !target.roots.contains(root) {
                        return Err("unexported datatype module root".into());
                    }
                    Ok(graph::ResolvedTarget::Datatype(
                        catalogs[module]
                            .get(root)
                            .ok_or("unknown datatype export")?
                            .clone(),
                    ))
                }
                other => graph::resolve_leaf_target(
                    &other.leaf().ok_or("datatype leaf required")?,
                    &leaves,
                ),
            },
            &work,
        )
        .map_err(|_| ModuleVerificationError {
            code: "datatype-module-source-refused",
            related: m
                .bindings
                .keys()
                .filter_map(|n| {
                    SchemaDeclarationNode::new(modules[id].capture.document().clone(), *n)
                })
                .collect(),
            cycle_modules: vec![],
        })?;
        inspections.insert(id.clone(), result);
    }
    for (id, (sources, sites, _)) in inspections {
        let module = modules.get_mut(&id).unwrap();
        module.sources = sources;
        module.sites = sites;
    }
    let (module_order, datatype_order) = order::inspect(&c.root, &modules, &leaves, &work)?;
    work.check()?;
    Ok(VerifiedDatatypeModules {
        pin: pin.clone(),
        modules,
        leaves,
        module_order,
        datatype_order,
    })
}
#[cfg(test)]
mod tests;
