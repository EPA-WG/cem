//! Inert module/profile assembly. Intended constant targets and profile mappings
//! are not completed selections, executable installations or receiver authority.
use super::*;
use crate::native_registrations::{
    dependency_sources::BuiltOperationSource, profiles::v2 as profiles,
};
use cem_ml::{parser::CemAstNode, schema::registry::CEM_SCHEMA_URI};
mod constants;
mod profile_inputs;
pub const MAGIC: &[u8] = b"CEMDM2\n";
const ABI: &str = "cem.native-datatype-modules/2";
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleNodeId {
    pub module: String,
    pub node: AstNodeId,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSelection {
    pub pin: profiles::OperationProfilesPin,
    pub id: String,
}
pub struct ProfileArtifact {
    pub bytes: Vec<u8>,
    pub pin: profiles::OperationProfilesPin,
    pub sources: Vec<BuiltOperationSource>,
}
#[derive(Clone)]
pub struct ModuleSource {
    pub types: super::ModuleSource,
    pub imports: BTreeSet<String>,
    pub constant_exports: BTreeSet<AstNodeId>,
    pub constants: BTreeMap<AstNodeId, Vec<ModuleNodeId>>,
    pub profiles: BTreeMap<AstNodeId, ProfileSelection>,
}
impl ModuleSource {
    pub fn new(types: super::ModuleSource) -> Self {
        Self {
            types,
            imports: BTreeSet::new(),
            constant_exports: BTreeSet::new(),
            constants: BTreeMap::new(),
            profiles: BTreeMap::new(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleRecord {
    types: super::ModuleRecord,
    imports: BTreeSet<String>,
    constant_exports: BTreeSet<AstNodeId>,
    constants: BTreeMap<AstNodeId, Vec<ModuleNodeId>>,
    profiles: BTreeMap<AstNodeId, ProfileSelection>,
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
pub struct ConstantSite {
    pub datatype: AstNodeId,
    pub slot: SchemaDeclarationNode,
    /// Source-owned intended terminal declarations, in authored order with duplicates.
    /// Literal syntax/values and actual selector results still need the shared compiler.
    pub targets: Vec<SchemaDeclarationNode>,
}
pub struct VerifiedModule {
    types: VerifiedDatatypeModule,
    constant_exports: BTreeSet<AstNodeId>,
    profiles: BTreeMap<AstNodeId, profiles::VerifiedOperationProfile>,
    constant_sites: Vec<ConstantSite>,
}
impl VerifiedModule {
    pub fn source(&self) -> &Arc<RetainedCemTree> {
        self.types.source()
    }
    pub fn capture(&self) -> &Arc<LexicallyScopedDocument> {
        self.types.capture()
    }
    pub fn identity(&self) -> &SourceIdentity {
        self.types.identity()
    }
    pub fn roots(&self) -> &BTreeSet<AstNodeId> {
        self.types.roots()
    }
    pub fn sources(&self) -> &BTreeMap<AstNodeId, DatatypeSource> {
        self.types.sources()
    }
    pub fn sites(&self) -> &[DependencySite<ModuleTarget>] {
        self.types.sites()
    }
    pub fn profiles(&self) -> &BTreeMap<AstNodeId, profiles::VerifiedOperationProfile> {
        &self.profiles
    }
    pub fn constant_exports(&self) -> &BTreeSet<AstNodeId> {
        &self.constant_exports
    }
    pub fn constant_sites(&self) -> &[ConstantSite] {
        &self.constant_sites
    }
}
pub struct VerifiedDatatypeModules {
    pin: DatatypeModulesPin,
    modules: BTreeMap<String, VerifiedModule>,
    leaves: BTreeMap<String, VerifiedArtifact>,
    module_order: Vec<String>,
    datatype_order: Vec<ModuleDatatypeId>,
    constant_order: Vec<ModuleNodeId>,
    constant_bindings: BTreeMap<ModuleNodeId, Vec<ModuleNodeId>>,
}
impl VerifiedDatatypeModules {
    pub fn pin(&self) -> &DatatypeModulesPin {
        &self.pin
    }
    pub fn module(&self, id: &str) -> Option<&VerifiedModule> {
        self.modules.get(id)
    }
    pub fn module_ids(&self) -> impl Iterator<Item = &str> {
        self.modules.keys().map(String::as_str)
    }
    pub fn leaf_ids(&self) -> impl Iterator<Item = &str> {
        self.leaves.keys().map(String::as_str)
    }
    pub(crate) fn leaf(&self, id: &str) -> Option<&VerifiedArtifact> {
        self.leaves.get(id)
    }
    /// Exact source assertions, not completed selections or crossing grants.
    pub fn constant_bindings(&self) -> &BTreeMap<ModuleNodeId, Vec<ModuleNodeId>> {
        &self.constant_bindings
    }
    pub fn module_order(&self) -> &[String] {
        &self.module_order
    }
    pub fn datatype_order(&self) -> &[ModuleDatatypeId] {
        &self.datatype_order
    }
    pub fn constant_order(&self) -> &[ModuleNodeId] {
        &self.constant_order
    }
    pub fn selections_verified(&self) -> bool {
        false
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
}
fn refused(code: &'static str) -> ModuleVerificationError {
    ModuleVerificationError::code(code)
}
fn shape(m: &ModuleRecord) -> bool {
    m.types.bindings.len() <= 128
        && m.imports.len() <= MAX_MODULES
        && m.constant_exports.len() <= 128
        && m.constants.len() <= 128
        && m.profiles.len() <= 64
        && m.constants.values().all(|v| v.len() <= 128)
}
pub fn export(
    root: &str,
    modules: &BTreeMap<String, ModuleSource>,
    artifacts: &[DependencyArtifact],
    profile_artifacts: &[ProfileArtifact],
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
        return Err(refused("datatype-module-graph-refused"));
    }
    let mut records = BTreeMap::new();
    for (id, m) in modules {
        work.check()?;
        if !module_name(id)
            || m.types.bindings.len() > 128
            || m.constants.len() > 128
            || m.constant_exports.len() > 128
            || m.profiles.len() > 64
            || m.imports.len() > MAX_MODULES
            || m.constants.values().any(|v| v.len() > 128)
        {
            return Err(refused("datatype-module-graph-limit"));
        }
        let source = NativeDependencySourceSnapshot::capture(
            &m.types.capture,
            &m.types.identity,
            &mut || work.check(),
        )?;
        let record = ModuleRecord {
            types: super::ModuleRecord {
                identity: m.types.identity.clone(),
                roots: m.types.roots.clone(),
                bindings: m.types.bindings.clone(),
                source,
            },
            imports: m.imports.clone(),
            constant_exports: m.constant_exports.clone(),
            constants: m.constants.clone(),
            profiles: m.profiles.clone(),
        };
        if encode(&record)?.len() > MAX_MODULE_BYTES {
            return Err(refused("datatype-module-graph-limit"));
        }
        records.insert(id.clone(), record);
    }
    let c = Capsule {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: registry_hash.into(),
        root: root.into(),
        modules: records,
    };
    let bytes = [MAGIC, &encode(&c)?].concat();
    let pin = DatatypeModulesPin {
        hash: hash(&bytes),
        registry_hash: registry_hash.into(),
        root: root.into(),
    };
    verify(&bytes, &pin, artifacts, profile_artifacts, control)?;
    Ok(BuiltDatatypeModules { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &DatatypeModulesPin,
    artifacts: &[DependencyArtifact],
    profile_artifacts: &[ProfileArtifact],
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
        return Err(refused("datatype-module-graph-refused"));
    }
    let own_bytes = profile_inputs::account(bytes.len(), profile_artifacts, &work)?;
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 100_000, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let c =
        Capsule::deserialize(&mut decoder).map_err(|_| refused("datatype-module-graph-refused"))?;
    if encode(&c)? != body
        || c.abi != ABI
        || c.ml_version != cem_ml::VERSION
        || c.ql_version != crate::VERSION
        || c.registry_hash != pin.registry_hash
        || c.root != pin.root
        || !c.modules.contains_key(&c.root)
        || c.modules.is_empty()
        || c.modules.len() > MAX_MODULES
    {
        return Err(refused("datatype-module-graph-refused"));
    }
    let mut total = 0usize;
    let mut raw_targets = 0usize;
    let mut agreement = SourceAgreement::new();
    let mut modules = BTreeMap::new();
    let mut catalogs = BTreeMap::new();
    let mut required = BTreeSet::new();
    for (id, m) in &c.modules {
        work.check()?;
        total = total
            .checked_add(m.types.bindings.len() + m.constants.len())
            .ok_or("module field overflow")?;
        for targets in m.constants.values() {
            work.check()?;
            raw_targets = raw_targets
                .checked_add(targets.len())
                .ok_or("constant target overflow")?;
        }
        if !module_name(id)
            || !shape(m)
            || total > MAX_FIELDS
            || raw_targets > MAX_FIELDS
            || encode(m)?.len() > MAX_MODULE_BYTES
        {
            return Err(refused("datatype-module-graph-limit"));
        }
        let capture = m
            .types
            .source
            .restore(&m.types.identity, &mut || work.check())?;
        remember_source(&mut agreement, &capture, &m.types.identity, &work)?;
        let catalog = graph::catalog(&capture, &work)?;
        if m.types.roots.len() > 32
            || m.types.roots.iter().any(|r| !catalog.contains_key(r))
            || (m.types.roots.is_empty()
                && (m.constant_exports.is_empty() || !m.types.bindings.is_empty()))
            || (id == &c.root && m.types.roots.is_empty())
        {
            return Err(refused("datatype-module-export-refused"));
        }
        let source = RetainedCemTree::from_shared(
            capture.document().clone(),
            &m.types.identity.uri,
            "",
            Default::default(),
            None,
        )?;
        modules.insert(
            id.clone(),
            VerifiedDatatypeModule {
                identity: m.types.identity.clone(),
                capture,
                source,
                roots: m.types.roots.clone(),
                sources: BTreeMap::new(),
                sites: vec![],
            },
        );
        catalogs.insert(id.clone(), catalog);
        required.extend(
            m.types
                .bindings
                .values()
                .filter_map(ModuleTarget::leaf_id)
                .map(str::to_owned),
        );
    }
    let leaves = verify_leaf_artifacts(
        artifacts,
        &required,
        &pin.registry_hash,
        own_bytes,
        &mut agreement,
        &work,
        control,
    )?;
    let mut inspected = BTreeMap::new();
    for (id, m) in &c.modules {
        work.check()?;
        if m.types.roots.is_empty() {
            continue;
        }
        let result = graph::inspect_catalog_with_constants(
            &m.types.roots,
            &m.types.bindings,
            &catalogs[id],
            |target| match target {
                ModuleTarget::LocalDatatype(n) => Ok(graph::ResolvedTarget::Local(*n)),
                ModuleTarget::ModuleDatatype { module, root } => {
                    let dest = modules.get(module).ok_or("unknown module")?;
                    if !dest.roots.contains(root) {
                        return Err("unexported datatype root".into());
                    }
                    Ok(graph::ResolvedTarget::Datatype(
                        catalogs[module]
                            .get(root)
                            .ok_or("unknown datatype")?
                            .clone(),
                    ))
                }
                other => graph::resolve_leaf_target(&other.leaf().ok_or("leaf required")?, &leaves),
            },
            &work,
        )
        .map_err(|_| ModuleVerificationError {
            code: "datatype-module-source-refused",
            related: m
                .types
                .bindings
                .keys()
                .filter_map(|n| {
                    SchemaDeclarationNode::new(modules[id].capture.document().clone(), *n)
                })
                .collect(),
            cycle_modules: vec![],
        })?;
        inspected.insert(id.clone(), result);
    }
    for (id, (sources, sites, _)) in inspected {
        let m = modules.get_mut(&id).unwrap();
        m.sources = sources;
        m.sites = sites;
    }
    let (mut constant_sites, constant_order, extra_edges) =
        constants::inspect(&c.modules, &modules, &work)?;
    for (id, m) in &c.modules {
        let mut imports = BTreeSet::new();
        for site in &modules[id].sites {
            work.check()?;
            if let ModuleTarget::ModuleDatatype { module, .. } = &site.target {
                imports.insert(module.clone());
            }
        }
        for (from, to, _) in &extra_edges {
            work.check()?;
            if from == id {
                imports.insert(to.clone());
            }
        }
        if imports != m.imports {
            return Err(refused("datatype-module-import-refused"));
        }
    }
    let (module_order, datatype_order) =
        order::inspect_extended(&c.root, &modules, &leaves, &extra_edges, &work)?;
    let mut bound_profiles = profile_inputs::inspect(
        &c.modules,
        &modules,
        profile_artifacts,
        &pin.registry_hash,
        &mut agreement,
        &work,
        control,
    )?;
    let result = modules
        .into_iter()
        .map(|(id, types)| {
            let m = VerifiedModule {
                types,
                constant_exports: c.modules[&id].constant_exports.clone(),
                profiles: bound_profiles.remove(&id).unwrap_or_default(),
                constant_sites: constant_sites.remove(&id).unwrap_or_default(),
            };
            (id, m)
        })
        .collect();
    work.check()?;
    Ok(VerifiedDatatypeModules {
        pin: pin.clone(),
        modules: result,
        leaves,
        module_order,
        datatype_order,
        constant_order,
        constant_bindings: c
            .modules
            .iter()
            .flat_map(|(module, m)| {
                m.constants.iter().map(move |(node, targets)| {
                    (
                        ModuleNodeId {
                            module: module.clone(),
                            node: *node,
                        },
                        targets.clone(),
                    )
                })
            })
            .collect(),
    })
}
