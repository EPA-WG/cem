//! Separately versioned, inert application + assembly inventory. Exact pins and
//! original source agreement are not executable installation or receiver grants.
use super::{blob::Blob, encode, hash, valid_hash, Work, MAX_BYTES};
use crate::{
    datatype_dependency_capsule::{ArtifactPin, DependencyArtifact},
    datatype_module_capsule::v2 as assembly,
    native_registrations::{dependency_sources as operations, profiles::v2 as profiles},
};
use cem_ml::{operation_control::OperationControl, parser::AstNodeId};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod agreement;
mod inventory;
pub const MAGIC: &[u8] = b"CEMAPPD3\n";
const ABI: &str = "cem-application-dependencies/3";
const INVALID: &str = "assembly application envelope refused";
const MAX_ASSEMBLIES: usize = 8;
const MAX_PROFILES: usize = 32;
const MAX_LEAVES: usize = 64;
const MAX_ROOTS: usize = 64;
const MAX_BLOBS: usize = 192;

/// Explicit application root in the assembly's exported root module. It is an
/// intended selection only, independent of named component receiver contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyRoot {
    pub assembly: String,
    pub datatype: AstNodeId,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssemblyRecord {
    pin: assembly::DatatypeModulesPin,
    leaves: BTreeSet<String>,
    profiles: BTreeSet<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileRecord {
    pin: profiles::OperationProfilesPin,
    sources: BTreeSet<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    application: super::ApplicationEnvelopePin,
    roots: BTreeMap<String, AssemblyRoot>,
    assemblies: BTreeMap<String, AssemblyRecord>,
    leaves: BTreeMap<String, ArtifactPin>,
    profiles: BTreeMap<String, ProfileRecord>,
    operations: BTreeMap<String, operations::OperationSourcePin>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    manifest: Manifest,
    blobs: BTreeMap<String, Blob>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationEnvelopePin {
    pub hash: String,
    pub manifest_hash: String,
    pub registry_hash: String,
}
pub struct BuiltApplicationEnvelope {
    pub bytes: Vec<u8>,
    pub pin: ApplicationEnvelopePin,
}
#[derive(Clone)]
pub struct EnvelopeBuilder {
    manifest: Manifest,
    blobs: BTreeMap<String, Blob>,
}
impl EnvelopeBuilder {
    /// Wrap the existing native structural application without widening its
    /// receivers. Full verification is performed by `build`, with caller control.
    pub fn new(application: super::BuiltApplicationEnvelope) -> Result<Self, String> {
        let mut builder = Self {
            manifest: Manifest {
                abi: ABI.into(),
                ml_version: cem_ml::VERSION.into(),
                ql_version: crate::VERSION.into(),
                registry_hash: application.pin.registry_hash.clone(),
                application: application.pin,
                roots: BTreeMap::new(),
                assemblies: BTreeMap::new(),
                leaves: BTreeMap::new(),
                profiles: BTreeMap::new(),
                operations: BTreeMap::new(),
            },
            blobs: BTreeMap::new(),
        };
        builder.blob(
            &builder.manifest.application.hash.clone(),
            application.bytes,
        )?;
        Ok(builder)
    }
    fn blob(&mut self, id: &str, bytes: Vec<u8>) -> Result<(), String> {
        if !valid_hash(id) || hash(&bytes) != id || bytes.len() > MAX_BYTES {
            return Err(INVALID.into());
        }
        if let Some(old) = self.blobs.get(id) {
            if old.0 != bytes {
                return Err(INVALID.into());
            }
            return Ok(());
        }
        if self.blobs.len() >= MAX_BLOBS
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
        self.blobs.insert(id.into(), Blob(bytes));
        Ok(())
    }
    /// Transactional staging: a refused insertion preserves all prior bytes and
    /// inventories. Semantic verification and explicit roots belong to `build`.
    pub fn add_assembly(
        &mut self,
        built: assembly::BuiltDatatypeModules,
        leaves: Vec<DependencyArtifact>,
        profiles: Vec<assembly::ProfileArtifact>,
    ) -> Result<String, String> {
        if leaves.len() > MAX_LEAVES || profiles.len() > MAX_PROFILES {
            return Err(INVALID.into());
        }
        let mut candidate = self.clone();
        let id = candidate.insert_assembly(built, leaves, profiles)?;
        *self = candidate;
        Ok(id)
    }
    fn insert_assembly(
        &mut self,
        built: assembly::BuiltDatatypeModules,
        leaves: Vec<DependencyArtifact>,
        profiles: Vec<assembly::ProfileArtifact>,
    ) -> Result<String, String> {
        let mut record = AssemblyRecord {
            pin: built.pin,
            leaves: BTreeSet::new(),
            profiles: BTreeSet::new(),
        };
        self.blob(&record.pin.hash, built.bytes)?;
        for leaf in leaves {
            let id = leaf.id().to_owned();
            if !record.leaves.insert(id.clone()) {
                return Err(INVALID.into());
            }
            self.blob(&id, leaf.bytes)?;
            inventory::insert(&mut self.manifest.leaves, &id, leaf.pin)?;
        }
        for profile in profiles {
            let id = profile.pin.hash.clone();
            if !record.profiles.insert(id.clone()) || profile.sources.len() > MAX_LEAVES {
                return Err(INVALID.into());
            }
            self.blob(&id, profile.bytes)?;
            let mut p = ProfileRecord {
                pin: profile.pin,
                sources: BTreeSet::new(),
            };
            for source in profile.sources {
                let id = source.pin.hash.clone();
                if !p.sources.insert(id.clone()) {
                    return Err(INVALID.into());
                }
                self.blob(&id, source.bytes)?;
                inventory::insert(&mut self.manifest.operations, &id, source.pin)?;
            }
            inventory::insert(&mut self.manifest.profiles, &id, p)?;
        }
        let id = record.pin.hash.clone();
        inventory::insert(&mut self.manifest.assemblies, &id, record)?;
        inventory::shape(&self.manifest)?;
        Ok(id)
    }
    pub fn build(
        mut self,
        roots: BTreeMap<String, AssemblyRoot>,
        control: &OperationControl,
    ) -> Result<BuiltApplicationEnvelope, String> {
        Work::new(control).check()?;
        self.manifest.roots = roots;
        let pin = ApplicationEnvelopePin {
            hash: String::new(),
            manifest_hash: hash(&encode(&self.manifest)?),
            registry_hash: self.manifest.registry_hash.clone(),
        };
        let bytes = [
            MAGIC,
            &encode(&Envelope {
                manifest: self.manifest,
                blobs: self.blobs,
            })?,
        ]
        .concat();
        let pin = ApplicationEnvelopePin {
            hash: hash(&bytes),
            ..pin
        };
        verify(&bytes, &pin, control)?;
        Ok(BuiltApplicationEnvelope { bytes, pin })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyKey {
    Application(String),
    Leaf(String),
    OperationSource(String),
    Profile(String),
    Assembly(String),
}
pub struct VerifiedApplicationEnvelope {
    pin: ApplicationEnvelopePin,
    roots: BTreeMap<String, AssemblyRoot>,
    application: Arc<super::VerifiedApplicationEnvelope>,
    assemblies: BTreeMap<String, Arc<assembly::VerifiedDatatypeModules>>,
    order: Vec<DependencyKey>,
}
impl VerifiedApplicationEnvelope {
    pub fn pin(&self) -> &ApplicationEnvelopePin {
        &self.pin
    }
    pub fn roots(&self) -> &BTreeMap<String, AssemblyRoot> {
        &self.roots
    }
    pub fn application(&self) -> &super::VerifiedApplicationEnvelope {
        &self.application
    }
    /// Inert original owner for explicit actual-host staging, not a binding.
    pub fn application_owner(&self) -> &Arc<super::VerifiedApplicationEnvelope> {
        &self.application
    }
    pub fn assembly(&self, id: &str) -> Option<&assembly::VerifiedDatatypeModules> {
        self.assemblies.get(id).map(Arc::as_ref)
    }
    /// Keep the checked source arenas when preparing actual-host compilations.
    pub fn assembly_owner(&self, id: &str) -> Option<&Arc<assembly::VerifiedDatatypeModules>> {
        self.assemblies.get(id)
    }
    pub fn assembly_ids(&self) -> impl Iterator<Item = &str> {
        self.assemblies.keys().map(String::as_str)
    }
    pub fn assembly_count(&self) -> usize {
        self.assemblies.len()
    }
    pub fn dependency_order(&self) -> &[DependencyKey] {
        &self.order
    }
    pub fn selections_verified(&self) -> bool {
        false
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
    pub fn native_link_closure_verified(&self) -> bool {
        false
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
    let m = envelope.manifest;
    if hash(&encode(&m)?) != pin.manifest_hash
        || m.abi != ABI
        || m.ml_version != cem_ml::VERSION
        || m.ql_version != crate::VERSION
        || m.registry_hash != pin.registry_hash
    {
        return Err(INVALID.into());
    }
    inventory::check(&m, &envelope.blobs, &work)?;
    let application = super::verify(
        &envelope.blobs[&m.application.hash].0,
        &m.application,
        control,
    )?;
    let mut agreement = agreement::Agreement::new(control);
    agreement.application(&application)?;
    let mut assemblies = BTreeMap::new();
    let mut modules = 0usize;
    for (id, record) in &m.assemblies {
        work.check()?;
        let leaves = inventory::leaves(&m, &envelope.blobs, record);
        let profiles = inventory::profiles(&m, &envelope.blobs, record);
        let graph = assembly::verify(
            &envelope.blobs[id].0,
            &record.pin,
            &leaves,
            &profiles,
            control,
        )
        .map_err(|e| e.to_string())?;
        modules += graph.module_ids().count();
        if modules > 64 {
            return Err(INVALID.into());
        }
        agreement.assembly(&graph)?;
        assemblies.insert(id.clone(), Arc::new(graph));
    }
    for root in m.roots.values() {
        work.check()?;
        let graph = assemblies.get(&root.assembly).ok_or(INVALID)?;
        if !graph
            .module(&graph.pin().root)
            .ok_or(INVALID)?
            .roots()
            .contains(&root.datatype)
        {
            return Err(INVALID.into());
        }
    }
    work.check()?;
    let order = inventory::order(&m);
    Ok(VerifiedApplicationEnvelope {
        pin: pin.clone(),
        roots: m.roots,
        application: Arc::new(application),
        assemblies,
        order,
    })
}
#[cfg(test)]
mod tests;
