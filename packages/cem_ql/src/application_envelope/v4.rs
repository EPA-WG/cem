//! Complete native application composition. Verification is inert; a native
//! publication still requires exact live host, registry and code owners.
use super::{blob::Blob, encode, hash, valid_hash, Work, MAX_BYTES};
use super::{mixed, v3};
use crate::{
    application_requirements::ApplicationRequirements,
    declaration_artifact::DeclarationArtifactMetadata, render::TemplateArtifact,
};
use cem_ml::operation_control::OperationControl;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod claims;
mod demand;
pub use demand::RegistryDemand;
pub const MAGIC: &[u8] = b"CEMAPPD4\n";
const ABI: &str = "cem-application-dependencies/4";
const INVALID: &str = "complete application envelope refused";
const MAX_MIXED: usize = 32;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    abi: String,
    ml_version: String,
    ql_version: String,
    registry_hash: String,
    parent: v3::ApplicationEnvelopePin,
    mixed: BTreeMap<String, mixed::MixedReceiverPin>,
    roots: BTreeSet<String>,
    requirements_hash: String,
    registry_demand_hash: String,
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
pub struct VerifiedApplicationEnvelope {
    pin: ApplicationEnvelopePin,
    parent: Arc<v3::VerifiedApplicationEnvelope>,
    mixed: BTreeMap<String, Arc<mixed::VerifiedMixedReceiverDeclaration>>,
    requirements: ApplicationRequirements,
    demand: RegistryDemand,
    order: Vec<String>,
}
impl VerifiedApplicationEnvelope {
    pub fn pin(&self) -> &ApplicationEnvelopePin {
        &self.pin
    }
    pub fn parent(&self) -> &Arc<v3::VerifiedApplicationEnvelope> {
        &self.parent
    }
    pub fn roots(&self) -> &BTreeSet<String> {
        &self.requirements.roots
    }
    pub fn mixed(&self, tag: &str) -> Option<&Arc<mixed::VerifiedMixedReceiverDeclaration>> {
        self.mixed.get(tag)
    }
    pub fn mixed_tags(&self) -> impl Iterator<Item = &str> {
        self.mixed.keys().map(String::as_str)
    }
    pub fn mixed_count(&self) -> usize {
        self.mixed.len()
    }
    pub fn component_count(&self) -> usize {
        self.requirements.declarations.len()
    }
    pub fn declaration(&self, tag: &str) -> Option<&DeclarationArtifactMetadata> {
        self.mixed
            .get(tag)
            .map(|m| m.declaration())
            .or_else(|| self.parent.application().declaration(tag))
    }
    pub fn artifact(&self, tag: &str) -> Option<&TemplateArtifact> {
        self.mixed
            .get(tag)
            .map(|m| m.artifact())
            .or_else(|| self.parent.application().artifact(tag))
    }
    pub fn requirements(&self) -> &ApplicationRequirements {
        &self.requirements
    }
    pub fn registry_demand(&self) -> &RegistryDemand {
        &self.demand
    }
    pub fn component_order(&self) -> &[String] {
        &self.order
    }
    pub fn native_link_closure_verified(&self) -> bool {
        false
    }
    pub fn runtime_admission_verified(&self) -> bool {
        false
    }
}
pub fn build(
    parent: v3::BuiltApplicationEnvelope,
    declarations: Vec<mixed::BuiltMixedReceiverDeclaration>,
    roots: BTreeSet<String>,
    control: &OperationControl,
) -> Result<BuiltApplicationEnvelope, String> {
    let work = Work::new(control);
    work.check()?;
    if declarations.is_empty() || declarations.len() > MAX_MIXED || parent.bytes.len() > MAX_BYTES {
        return Err(INVALID.into());
    }
    let owner = Arc::new(v3::verify(&parent.bytes, &parent.pin, control)?);
    let mut values = BTreeMap::new();
    let mut pins = BTreeMap::new();
    let mut blobs = BTreeMap::from([(parent.pin.hash.clone(), Blob(parent.bytes))]);
    for built in declarations {
        work.check()?;
        let value = Arc::new(mixed::verify(
            &built.bytes,
            &built.pin,
            owner.clone(),
            control,
        )?);
        let tag = value.declaration().tag.clone();
        if values.insert(tag.clone(), value).is_some() {
            return Err(INVALID.into());
        }
        pins.insert(tag, built.pin.clone());
        blobs.insert(built.pin.hash, Blob(built.bytes));
    }
    let (requirements, demand, _) = demand::compose(&owner, &values, roots.clone(), &work)?;
    let manifest = Manifest {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        registry_hash: parent.pin.registry_hash.clone(),
        parent: parent.pin,
        mixed: pins,
        roots,
        requirements_hash: requirements.pinned_json()?.1,
        registry_demand_hash: demand.pinned_json()?.1,
    };
    let manifest_hash = hash(&encode(&manifest)?);
    let bytes = [MAGIC, &encode(&Envelope { manifest, blobs })?].concat();
    let pin = ApplicationEnvelopePin {
        hash: hash(&bytes),
        manifest_hash,
        registry_hash: owner.pin().registry_hash.clone(),
    };
    verify(&bytes, &pin, control)?;
    Ok(BuiltApplicationEnvelope { bytes, pin })
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
        || [&pin.hash, &pin.manifest_hash, &pin.registry_hash]
            .iter()
            .any(|h| !valid_hash(h))
        || hash(bytes) != pin.hash
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    cem_ml::native_binary::preflight_messagepack(body, 32_768, 64, || work.check())?;
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let e = Envelope::deserialize(&mut decoder).map_err(|_| INVALID)?;
    if encode(&e)? != body {
        return Err(INVALID.into());
    }
    let m = e.manifest;
    if m.abi != ABI
        || m.ml_version != cem_ml::VERSION
        || m.ql_version != crate::VERSION
        || hash(&encode(&m)?) != pin.manifest_hash
        || m.registry_hash != pin.registry_hash
        || m.parent.registry_hash != m.registry_hash
        || m.mixed.is_empty()
        || m.mixed.len() > MAX_MIXED
        || e.blobs.len() > MAX_MIXED + 1
    {
        return Err(INVALID.into());
    }
    let required = std::iter::once(m.parent.hash.clone())
        .chain(m.mixed.values().map(|p| p.hash.clone()))
        .collect::<BTreeSet<_>>();
    if required != e.blobs.keys().cloned().collect() {
        return Err(INVALID.into());
    }
    for (id, blob) in &e.blobs {
        work.check()?;
        if !valid_hash(id) || hash(&blob.0) != *id {
            return Err(INVALID.into());
        }
    }
    let parent = Arc::new(v3::verify(&e.blobs[&m.parent.hash].0, &m.parent, control)?);
    let mut mixed = BTreeMap::new();
    for (tag, pin) in &m.mixed {
        work.check()?;
        let value = Arc::new(super::mixed::verify(
            &e.blobs[&pin.hash].0,
            pin,
            parent.clone(),
            control,
        )?);
        if value.declaration().tag != *tag {
            return Err(INVALID.into());
        }
        mixed.insert(tag.clone(), value);
    }
    let (requirements, demand, order) = demand::compose(&parent, &mixed, m.roots, &work)?;
    if requirements.pinned_json()?.1 != m.requirements_hash
        || demand.pinned_json()?.1 != m.registry_demand_hash
    {
        return Err(INVALID.into());
    }
    work.check()?;
    Ok(VerifiedApplicationEnvelope {
        pin: pin.clone(),
        parent,
        mixed,
        requirements,
        demand,
        order,
    })
}
#[cfg(test)]
mod tests;
