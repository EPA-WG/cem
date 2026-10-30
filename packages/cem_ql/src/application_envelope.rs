//! Bounded inert application dependency packages. Verification is neither
//! executable datatype admission nor a native-generation publication receipt.
use crate::{
    application_requirements::{ApplicationRequirements, SelectedRegistryPlan},
    declaration_artifact::{BuiltDeclarationArtifact, DeclarationArtifactMetadata},
    native_registrations::{
        self, BuiltNativeRequirement, NativeRequirementPin, VerifiedNativeRequirement,
    },
    query_validator_capsule::declarations::{
        self as query, BuiltQueryDeclaration, QueryDeclarationPin, VerifiedQueryDeclaration,
    },
    render::TemplateArtifact,
};
use cem_ml::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    schema::structural_capsule::{
        self, BuiltStructuralCapsule, CapsulePin, VerifiedStructuralCapsule,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod blob;
mod graph;
use blob::Blob;
pub const MAGIC: &[u8] = b"CEMAPPD2\n";
const ABI: &str = "cem-application-dependencies/2";
const INVALID: &str = "application dependency envelope refused";
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_COMPONENTS: usize = 32;
const MAX_CONTRACTS: usize = 64;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeReceiverRequirement {
    pub contract: String,
    pub schema_uri: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Component {
    metadata: String,
    template: String,
    dependencies: BTreeSet<String>,
    receivers: BTreeMap<String, NodeReceiverRequirement>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum ContractPin {
    Structural(CapsulePin),
    Query(QueryDeclarationPin),
    Native(NativeRequirementPin),
}
impl ContractPin {
    fn hash(&self) -> &str {
        match self {
            Self::Structural(p) => &p.hash,
            Self::Query(p) => &p.hash,
            Self::Native(p) => &p.hash,
        }
    }
    fn registry(&self) -> &str {
        match self {
            Self::Structural(p) => &p.registry_hash,
            Self::Query(p) => &p.program.registry_hash,
            Self::Native(p) => &p.registry_hash,
        }
    }
}
pub struct ContractArtifact {
    bytes: Vec<u8>,
    pin: ContractPin,
}
impl ContractArtifact {
    pub fn structural(value: BuiltStructuralCapsule) -> Self {
        Self {
            bytes: value.bytes,
            pin: ContractPin::Structural(value.pin),
        }
    }
    pub fn query(value: BuiltQueryDeclaration) -> Self {
        Self {
            bytes: value.bytes,
            pin: ContractPin::Query(value.pin),
        }
    }
    pub fn native(value: BuiltNativeRequirement) -> Self {
        Self {
            bytes: value.bytes,
            pin: ContractPin::Native(value.pin),
        }
    }
    pub fn id(&self) -> &str {
        self.pin.hash()
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    roots: BTreeSet<String>,
    contract_roots: BTreeSet<String>,
    components: BTreeMap<String, Component>,
    contracts: BTreeMap<String, ContractPin>,
    dependencies: BTreeMap<String, BTreeSet<String>>,
    requirements_hash: String,
    registry_plan_hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    manifest: Manifest,
    blobs: BTreeMap<String, Blob>,
}
#[derive(Debug, Clone)]
pub struct ApplicationEnvelopePin {
    pub hash: String,
    pub manifest_hash: String,
    pub registry_hash: String,
}
pub struct BuiltApplicationEnvelope {
    pub bytes: Vec<u8>,
    pub pin: ApplicationEnvelopePin,
}
pub struct EnvelopeBuilder {
    manifest: Manifest,
    blobs: BTreeMap<String, Blob>,
}
impl EnvelopeBuilder {
    pub fn new(registry_hash: &str) -> Result<Self, String> {
        if !valid_hash(registry_hash) {
            return Err(INVALID.into());
        }
        Ok(Self {
            manifest: Manifest {
                abi: ABI.into(),
                ml_version: cem_ml::VERSION.into(),
                ql_version: crate::VERSION.into(),
                registry_hash: registry_hash.into(),
                roots: BTreeSet::new(),
                contract_roots: BTreeSet::new(),
                components: BTreeMap::new(),
                contracts: BTreeMap::new(),
                dependencies: BTreeMap::new(),
                requirements_hash: String::new(),
                registry_plan_hash: String::new(),
            },
            blobs: BTreeMap::new(),
        })
    }
    fn blob(&mut self, bytes: Vec<u8>) -> Result<String, String> {
        if bytes.len() > MAX_BYTES {
            return Err(INVALID.into());
        }
        let id = hash(&bytes);
        if !self.blobs.contains_key(&id) {
            if self.blobs.len() >= 128
                || self
                    .blobs
                    .values()
                    .map(|b| b.0.len())
                    .sum::<usize>()
                    .saturating_add(bytes.len())
                    > MAX_BYTES
            {
                return Err(INVALID.into());
            }
            self.blobs.insert(id.clone(), Blob(bytes));
        }
        Ok(id)
    }
    pub fn add_contract(
        &mut self,
        artifact: ContractArtifact,
        dependencies: BTreeSet<String>,
    ) -> Result<String, String> {
        let id = artifact.id().to_owned();
        if artifact.bytes.len() > 1024 * 1024
            || dependencies.len() > MAX_CONTRACTS
            || dependencies.iter().any(|id| !valid_hash(id))
            || artifact.pin.registry() != self.manifest.registry_hash
            || hash(&artifact.bytes) != id
        {
            return Err(INVALID.into());
        }
        if let Some(previous) = self.manifest.contracts.get(&id) {
            if encode(previous)? != encode(&artifact.pin)?
                || self.manifest.dependencies.get(&id) != Some(&dependencies)
            {
                return Err(INVALID.into());
            }
            return Ok(id);
        }
        if self.manifest.contracts.len() >= MAX_CONTRACTS {
            return Err(INVALID.into());
        }
        self.blob(artifact.bytes)?;
        self.manifest.contracts.insert(id.clone(), artifact.pin);
        self.manifest.dependencies.insert(id.clone(), dependencies);
        Ok(id)
    }
    pub fn add_component(
        &mut self,
        built: BuiltDeclarationArtifact,
        receivers: BTreeMap<String, NodeReceiverRequirement>,
    ) -> Result<String, String> {
        if built.metadata_json.len() > 64 * 1024
            || built.template.bytes.len() > 1024 * 1024
            || receivers.len() > 64
            || self.manifest.components.len() >= MAX_COMPONENTS
        {
            return Err(INVALID.into());
        }
        let (metadata, artifact) = crate::application_declaration::load_template(
            &built.template.bytes,
            &built.metadata_json,
            &built.metadata_hash,
        )?;
        if self.manifest.components.len() >= MAX_COMPONENTS
            || self.manifest.components.contains_key(&metadata.tag)
            || built.metadata_json.len() > 64 * 1024
            || built.template.bytes.len() > 1024 * 1024
        {
            return Err(INVALID.into());
        }
        // Use the shared IR inventory before recursive dependency inspection.
        let inventory = crate::application_requirements::declaration_requirements(
            &artifact,
            &metadata,
            &built.metadata_hash,
        )?;
        if !inventory.unresolved.is_empty() {
            return Err(INVALID.into());
        }
        let mut dependencies = BTreeSet::new();
        crate::declaration_closure::references(&artifact.nodes, &mut dependencies)?;
        if dependencies.len() > MAX_COMPONENTS {
            return Err(INVALID.into());
        }
        // Validate the complete insertion before changing staging state.
        let additions = [
            (hash(&built.template.bytes), built.template.bytes.len()),
            (
                hash(built.metadata_json.as_bytes()),
                built.metadata_json.len(),
            ),
        ]
        .into_iter()
        .filter(|(id, _)| !self.blobs.contains_key(id))
        .collect::<BTreeMap<_, _>>();
        if self.blobs.len().saturating_add(additions.len()) > 128
            || self
                .blobs
                .values()
                .map(|b| b.0.len())
                .sum::<usize>()
                .saturating_add(additions.values().sum::<usize>())
                > MAX_BYTES
        {
            return Err(INVALID.into());
        }
        let template = self.blob(built.template.bytes)?;
        let metadata_id = self.blob(built.metadata_json.into_bytes())?;
        self.manifest.components.insert(
            metadata.tag.clone(),
            Component {
                metadata: metadata_id,
                template,
                dependencies,
                receivers,
            },
        );
        Ok(metadata.tag)
    }
    pub fn build(
        mut self,
        roots: BTreeSet<String>,
        contract_roots: BTreeSet<String>,
        control: &OperationControl,
    ) -> Result<BuiltApplicationEnvelope, String> {
        let work = Work::new(control);
        work.check()?;
        self.manifest.roots = roots;
        self.manifest.contract_roots = contract_roots;
        let (components, requirements, plan) =
            graph::declarations(&self.manifest, &self.blobs, &work)?;
        drop(components);
        self.manifest.requirements_hash = requirements.pinned_json()?.1;
        self.manifest.registry_plan_hash = plan.pinned_json()?.1;
        graph::closure(&self.manifest, &work)?;
        let envelope = Envelope {
            manifest: self.manifest,
            blobs: self.blobs,
        };
        let pin = ApplicationEnvelopePin {
            hash: String::new(),
            manifest_hash: hash(&encode(&envelope.manifest)?),
            registry_hash: envelope.manifest.registry_hash.clone(),
        };
        let bytes = [MAGIC, &encode(&envelope)?].concat();
        if bytes.len() > MAX_BYTES {
            return Err(INVALID.into());
        }
        let pin = ApplicationEnvelopePin {
            hash: hash(&bytes),
            ..pin
        };
        verify(&bytes, &pin, control)?;
        Ok(BuiltApplicationEnvelope { bytes, pin })
    }
}
struct VerifiedComponent {
    metadata: DeclarationArtifactMetadata,
    artifact: TemplateArtifact,
}
enum VerifiedContract {
    Structural(Arc<VerifiedStructuralCapsule>),
    Query(Arc<VerifiedQueryDeclaration>),
    Native(Arc<VerifiedNativeRequirement>),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyKey {
    Component(String),
    Contract(String),
}
pub struct VerifiedApplicationEnvelope {
    pin: ApplicationEnvelopePin,
    manifest: Manifest,
    components: BTreeMap<String, VerifiedComponent>,
    contracts: BTreeMap<String, VerifiedContract>,
    order: Vec<DependencyKey>,
    registry_plan: SelectedRegistryPlan,
}
impl VerifiedApplicationEnvelope {
    pub fn pin(&self) -> &ApplicationEnvelopePin {
        &self.pin
    }
    pub fn roots(&self) -> &BTreeSet<String> {
        &self.manifest.roots
    }
    pub fn component_count(&self) -> usize {
        self.components.len()
    }
    pub fn contract_count(&self) -> usize {
        self.contracts.len()
    }
    pub fn dependency_order(&self) -> &[DependencyKey] {
        &self.order
    }
    pub fn native_link_closure_verified(&self) -> bool {
        false
    }
    pub fn registry_plan(&self) -> &SelectedRegistryPlan {
        &self.registry_plan
    }
    pub fn declaration(&self, tag: &str) -> Option<&DeclarationArtifactMetadata> {
        Some(&self.components.get(tag)?.metadata)
    }
    pub fn artifact(&self, tag: &str) -> Option<&TemplateArtifact> {
        Some(&self.components.get(tag)?.artifact)
    }
    pub fn receiver(&self, tag: &str, name: &str) -> Option<&NodeReceiverRequirement> {
        self.manifest.components.get(tag)?.receivers.get(name)
    }
    pub fn structural(&self, id: &str) -> Option<&Arc<VerifiedStructuralCapsule>> {
        match self.contracts.get(id)? {
            VerifiedContract::Structural(v) => Some(v),
            _ => None,
        }
    }
    pub fn query(&self, id: &str) -> Option<&Arc<VerifiedQueryDeclaration>> {
        match self.contracts.get(id)? {
            VerifiedContract::Query(v) => Some(v),
            _ => None,
        }
    }
    pub fn native(&self, id: &str) -> Option<&Arc<VerifiedNativeRequirement>> {
        match self.contracts.get(id)? {
            VerifiedContract::Native(v) => Some(v),
            _ => None,
        }
    }
}
pub fn verify(
    bytes: &[u8],
    pin: &ApplicationEnvelopePin,
    control: &OperationControl,
) -> Result<VerifiedApplicationEnvelope, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(&pin.hash)
        || !valid_hash(&pin.manifest_hash)
        || !valid_hash(&pin.registry_hash)
        || hash(bytes) != pin.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 32_768, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let envelope = Envelope::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&envelope)? != body {
        return Err(INVALID.into());
    }
    let manifest = envelope.manifest;
    if hash(&encode(&manifest)?) != pin.manifest_hash
        || manifest.abi != ABI
        || manifest.ml_version != cem_ml::VERSION
        || manifest.ql_version != crate::VERSION
        || manifest.registry_hash != pin.registry_hash
    {
        return Err(INVALID.into());
    }
    graph::blobs(&manifest, &envelope.blobs, &work)?;
    let (components, requirements, registry_plan) =
        graph::declarations(&manifest, &envelope.blobs, &work)?;
    if requirements.pinned_json()?.1 != manifest.requirements_hash
        || registry_plan.pinned_json()?.1 != manifest.registry_plan_hash
    {
        return Err(INVALID.into());
    }
    let order = graph::closure(&manifest, &work)?;
    let mut contracts = BTreeMap::new();
    let mut sources: BTreeMap<String, Arc<VerifiedNativeRequirement>> = BTreeMap::new();
    let mut implementations = BTreeMap::new();
    for (id, expected) in &manifest.contracts {
        work.check()?;
        if expected.hash() != id || expected.registry() != pin.registry_hash {
            return Err(INVALID.into());
        }
        let bytes = &envelope.blobs.get(id).ok_or(INVALID)?.0;
        let verified = match expected {
            ContractPin::Structural(p) => VerifiedContract::Structural(Arc::new(
                structural_capsule::verify(bytes, p, control)?,
            )),
            ContractPin::Query(p) => {
                VerifiedContract::Query(Arc::new(query::verify(bytes, p, control)?))
            }
            ContractPin::Native(p) => {
                let value = native_registrations::verify(bytes, p, control)?;
                let identity = value.implementation();
                let encoded = encode(identity)?;
                if implementations
                    .insert(identity.id.clone(), encoded.clone())
                    .is_some_and(|previous| previous != encoded)
                {
                    return Err(INVALID.into());
                }
                let key = hash(&encode(&p.source)?);
                let value = if let Some(source) = sources.get(&key) {
                    Arc::new(value.share_source(source, control)?)
                } else {
                    Arc::new(value)
                };
                sources.entry(key).or_insert_with(|| value.clone());
                VerifiedContract::Native(value)
            }
        };
        contracts.insert(id.clone(), verified);
    }
    graph::receivers(&manifest, &contracts, &work)?;
    work.check()?;
    Ok(VerifiedApplicationEnvelope {
        pin: pin.clone(),
        manifest,
        components,
        contracts,
        order,
        registry_plan,
    })
}
#[cfg(test)]
mod tests;
