//! Versioned, inert mixed receiver declarations. Original structural capsules,
//! closed scalar facets and assembly-root demands remain distinct contract kinds.
use super::{blob::Blob, encode, hash, valid_hash, Work};
use crate::{
    application_declaration::{self, NamedReceiverType},
    datatype_module_capsule::v2::VerifiedDatatypeModules,
    declaration_artifact::{BuiltDeclarationArtifact, DeclarationArtifactMetadata},
    render::TemplateArtifact,
};
use cem_ml::{
    operation_control::OperationControl,
    schema::structural_capsule::{CapsulePin, VerifiedStructuralCapsule},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod contracts;
mod occurrences;
pub const MAGIC: &[u8] = b"CEMMRC1\n";
const ABI: &str = "cem.native-mixed-receivers/1";
const INVALID: &str = "mixed receiver declaration refused";
const MAX_BYTES: usize = 1024 * 1024;
const MAX_RECEIVERS: usize = 64;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ReceiverContract {
    Node(CapsulePin),
    Scalar(NamedReceiverType),
    /// Exact intended exported root, never a serialized executable descriptor.
    Executable {
        root: String,
        target: super::v3::AssemblyRoot,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    abi: String,
    ml_version: String,
    ql_version: String,
    application: super::v3::ApplicationEnvelopePin,
    metadata_hash: String,
    template_hash: String,
    requirements_hash: String,
    receivers: BTreeMap<String, ReceiverContract>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    manifest: Manifest,
    metadata: Blob,
    template: Blob,
}
#[derive(Debug, Clone)]
pub struct MixedReceiverPin {
    pub hash: String,
    pub manifest_hash: String,
    pub application_hash: String,
    pub registry_hash: String,
}
pub struct BuiltMixedReceiverDeclaration {
    pub bytes: Vec<u8>,
    pub pin: MixedReceiverPin,
}
/// Exact literal authored fields. Receiver linking must compare these fields
/// against original native occurrences; no model is fabricated by this record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiverOccurrence {
    pub fields: BTreeMap<String, String>,
}
pub struct VerifiedMixedReceiverDeclaration {
    pin: MixedReceiverPin,
    application: Arc<super::v3::VerifiedApplicationEnvelope>,
    declaration: DeclarationArtifactMetadata,
    artifact: TemplateArtifact,
    receivers: BTreeMap<String, ReceiverContract>,
    occurrences: BTreeMap<String, ReceiverOccurrence>,
    dependencies: BTreeSet<String>,
    requirements: crate::application_requirements::DeclarationRequirements,
}
impl VerifiedMixedReceiverDeclaration {
    pub fn pin(&self) -> &MixedReceiverPin {
        &self.pin
    }
    pub fn application(&self) -> &Arc<super::v3::VerifiedApplicationEnvelope> {
        &self.application
    }
    pub fn declaration(&self) -> &DeclarationArtifactMetadata {
        &self.declaration
    }
    pub fn artifact(&self) -> &TemplateArtifact {
        &self.artifact
    }
    pub fn receiver(&self, name: &str) -> Option<&ReceiverContract> {
        self.receivers.get(name)
    }
    pub fn receiver_count(&self) -> usize {
        self.receivers.len()
    }
    pub fn occurrences(&self) -> &BTreeMap<String, ReceiverOccurrence> {
        &self.occurrences
    }
    pub fn occurrence_count(&self) -> usize {
        self.occurrences.len()
    }
    /// Static component dependencies are requirements, not an appended registry.
    pub fn component_dependencies(&self) -> &BTreeSet<String> {
        &self.dependencies
    }
    /// Shared demand inventory; the parent registry is not extended by verification.
    pub fn requirements(&self) -> &crate::application_requirements::DeclarationRequirements {
        &self.requirements
    }
    pub fn node(&self, alias: &str) -> Option<&Arc<VerifiedStructuralCapsule>> {
        let ReceiverContract::Node(pin) = self.receivers.get(alias)? else {
            return None;
        };
        self.application.application().structural(&pin.hash)
    }
    pub fn executable_graph(&self, alias: &str) -> Option<&Arc<VerifiedDatatypeModules>> {
        let ReceiverContract::Executable { target, .. } = self.receivers.get(alias)? else {
            return None;
        };
        self.application.assembly_owner(&target.assembly)
    }
    pub fn runtime_admission_verified(&self) -> bool {
        false
    }
    pub fn executable_closure_verified(&self) -> bool {
        false
    }
}
pub fn build(
    declaration: BuiltDeclarationArtifact,
    application: Arc<super::v3::VerifiedApplicationEnvelope>,
    receivers: BTreeMap<String, ReceiverContract>,
    control: &OperationControl,
) -> Result<BuiltMixedReceiverDeclaration, String> {
    let work = Work::new(control);
    work.check()?;
    if declaration.metadata_json.len() > 64 * 1024 || declaration.template.bytes.len() > MAX_BYTES {
        return Err(INVALID.into());
    }
    contracts::shape(&receivers, &work)?;
    let (metadata, artifact) = application_declaration::load_template(
        &declaration.template.bytes,
        &declaration.metadata_json,
        &declaration.metadata_hash,
    )?;
    let inventory = crate::application_requirements::declaration_requirements(
        &artifact,
        &metadata,
        &declaration.metadata_hash,
    )?;
    let requirements_hash = hash(&encode(&inventory)?);
    let manifest = Manifest {
        abi: ABI.into(),
        ml_version: cem_ml::VERSION.into(),
        ql_version: crate::VERSION.into(),
        application: application.pin().clone(),
        metadata_hash: declaration.metadata_hash,
        template_hash: hash(&declaration.template.bytes),
        requirements_hash,
        receivers,
    };
    let pin = MixedReceiverPin {
        hash: String::new(),
        manifest_hash: hash(&encode(&manifest)?),
        application_hash: application.pin().hash.clone(),
        registry_hash: application.pin().registry_hash.clone(),
    };
    let bytes = [
        MAGIC,
        &encode(&Envelope {
            manifest,
            metadata: Blob(declaration.metadata_json.into_bytes()),
            template: Blob(declaration.template.bytes),
        })?,
    ]
    .concat();
    let pin = MixedReceiverPin {
        hash: hash(&bytes),
        ..pin
    };
    verify(&bytes, &pin, application, control)?;
    Ok(BuiltMixedReceiverDeclaration { bytes, pin })
}
pub fn verify(
    bytes: &[u8],
    pin: &MixedReceiverPin,
    application: Arc<super::v3::VerifiedApplicationEnvelope>,
    control: &OperationControl,
) -> Result<VerifiedMixedReceiverDeclaration, String> {
    let work = Work::new(control);
    work.check()?;
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || [
            &pin.hash,
            &pin.manifest_hash,
            &pin.application_hash,
            &pin.registry_hash,
        ]
        .iter()
        .any(|p| !valid_hash(p))
        || hash(bytes) != pin.hash
        || pin.application_hash != application.pin().hash
        || pin.registry_hash != application.pin().registry_hash
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
    if hash(&encode(&m)?) != pin.manifest_hash
        || m.abi != ABI
        || m.ml_version != cem_ml::VERSION
        || m.ql_version != crate::VERSION
        || encode(&m.application)? != encode(application.pin())?
        || e.metadata.0.len() > 64 * 1024
        || hash(&e.metadata.0) != m.metadata_hash
        || hash(&e.template.0) != m.template_hash
    {
        return Err(INVALID.into());
    }
    contracts::shape(&m.receivers, &work)?;
    let json = std::str::from_utf8(&e.metadata.0).map_err(|_| INVALID)?;
    let (declaration, artifact) =
        application_declaration::load_template(&e.template.0, json, &m.metadata_hash)?;
    // This declaration is a separate contract, never an implicit replacement of
    // an already installed CEMAPPD2/CEMAPPD3 component with the same public tag.
    if application
        .application()
        .declaration(&declaration.tag)
        .is_some()
    {
        return Err(INVALID.into());
    }
    let inventory = crate::application_requirements::declaration_requirements(
        &artifact,
        &declaration,
        &m.metadata_hash,
    )?;
    if !inventory.unresolved.is_empty() || hash(&encode(&inventory)?) != m.requirements_hash {
        return Err(INVALID.into());
    }
    let occurrences = occurrences::inspect(&artifact, &m.receivers, &work)?;
    contracts::check(&m.receivers, &application, &work)?;
    let mut dependencies = BTreeSet::new();
    crate::declaration_closure::references(&artifact.nodes, &mut dependencies)?;
    if dependencies
        .iter()
        .any(|tag| application.application().declaration(tag).is_none())
    {
        return Err(INVALID.into());
    }
    work.check()?;
    Ok(VerifiedMixedReceiverDeclaration {
        pin: pin.clone(),
        application,
        declaration,
        artifact,
        receivers: m.receivers,
        occurrences,
        dependencies,
        requirements: inventory,
    })
}
#[cfg(test)]
mod tests;
