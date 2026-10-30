//! Pinned native scalar receiver contracts for the opt-in component ABI.
//! Control metadata and native binary records; no schema AST/host callbacks.
#[cfg(not(target_arch = "wasm32"))]
use crate::declaration_artifact::BuiltDeclarationArtifact;
use crate::{declaration_artifact::DeclarationArtifactMetadata, render::TemplateArtifact};
use cem_ml::schema::document_model::AttributeValueContract;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const MAGIC: &[u8] = b"CEMAPPD1\n";
const CONTRACT: &str = "cem-application-declaration/1";
const INVALID: &str = "compiled application receiver contracts refused";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedReceiverType {
    pub name: String,
    pub schema_uri: String,
    pub source_uri: String,
    pub source_hash: String,
    pub value: AttributeValueContract,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TypePin {
    schema_uri: String,
    source_uri: String,
    source_hash: String,
    contract_hash: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Metadata {
    contract: String,
    declaration_json: String,
    declaration_hash: String,
    payload_hash: String,
    types: BTreeMap<String, TypePin>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    template: Vec<u8>,
    types: Vec<NamedReceiverType>,
}
pub struct BuiltApplicationDeclaration {
    pub bytes: Vec<u8>,
    pub metadata_json: String,
    pub metadata_hash: String,
}
pub struct AdmittedApplicationDeclaration {
    pub metadata: DeclarationArtifactMetadata,
    pub artifact: TemplateArtifact,
    pub types: BTreeMap<String, AttributeValueContract>,
}

const MAX_BYTES: usize = 1024 * 1024;
fn hash(bytes: &[u8]) -> String {
    cem_ml::content_cache::ContentHash::from_blake3(bytes).header_value()
}
fn valid_hash(value: &str) -> bool {
    value.strip_prefix("cem-bin/1+blake3:").is_some_and(|v| {
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn uri(value: &str) -> bool {
    value.len() <= 4096 && url::Url::parse(value).is_ok_and(|u| !u.cannot_be_a_base())
}
fn primitive(name: &str) -> bool {
    matches!(
        name.strip_prefix("schema:").unwrap_or(name),
        "string" | "integer" | "decimal" | "number" | "boolean"
    )
}
pub(crate) fn reserved(name: &str) -> bool {
    name.starts_with("schema:")
        || primitive(name)
        || cem_ml::schema::document_model::shipped_datatypes::ShippedDatatype::ALL
            .iter()
            .any(|kind| kind.name() == name)
        || matches!(
            name,
            "any" | "node" | "date" | "time" | "dateTime" | "datetime"
        )
}
fn valid_type(value: &NamedReceiverType) -> bool {
    if value.name.is_empty()
        || value.name.len() > 256
        || reserved(&value.name)
        || !value
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        || !uri(&value.schema_uri)
        || !uri(&value.source_uri)
        || !valid_hash(&value.source_hash)
        || value.value.content_type.is_some()
        || value.value.restrictions.len() > 16
        || value.value.accounted_bytes() > 64 * 1024
        || value.value.model.source_map.frames.is_empty()
        || !value
            .value
            .model
            .value_type
            .as_deref()
            .is_some_and(primitive)
    {
        return false;
    }
    value.value.models().all(|model| {
        if model.native_type_pending || model.value_type.as_deref().is_some_and(|t| !primitive(t)) {
            return false;
        }
        // Only the existing scalar facet representation is portable here.
        // Executable callbacks, parser/node contracts and other model policies
        // cannot silently become a string contract or acquire runtime grants.
        let mut rest = model.clone();
        rest.name.clear();
        rest.value_type = None;
        rest.source_map = Default::default();
        rest.allowed_values.clear();
        rest.min_inclusive = None;
        rest.max_inclusive = None;
        rest.min_exclusive = None;
        rest.max_exclusive = None;
        rest.min_length = None;
        rest.max_length = None;
        rest.length = None;
        rest.total_digits = None;
        rest.fraction_digits = None;
        rest.pattern = None;
        rest.white_space = None;
        rest == Default::default()
    })
}
fn pins(types: &[NamedReceiverType]) -> Result<BTreeMap<String, TypePin>, String> {
    if types.len() > 64 {
        return Err(INVALID.into());
    }
    let mut result = BTreeMap::new();
    let mut accounted = 0usize;
    for value in types {
        if !valid_type(value) {
            return Err(INVALID.into());
        }
        accounted = accounted.saturating_add(value.value.accounted_bytes());
        if accounted > MAX_BYTES {
            return Err(INVALID.into());
        }
        let pin = TypePin {
            schema_uri: value.schema_uri.clone(),
            source_uri: value.source_uri.clone(),
            source_hash: value.source_hash.clone(),
            contract_hash: hash(&rmp_serde::to_vec_named(value).map_err(|_| INVALID)?),
        };
        if result.insert(value.name.clone(), pin).is_some() {
            return Err(INVALID.into());
        }
    }
    Ok(result)
}
pub(crate) fn required(artifact: &TemplateArtifact) -> Result<std::collections::BTreeSet<String>, String> {
    use crate::render::{TemplateAttributeValue, TemplateNode};
    let mut pending = artifact
        .nodes
        .iter()
        .map(|node| (node, 0))
        .collect::<Vec<_>>();
    let mut result = std::collections::BTreeSet::new();
    let mut count = 0;
    while let Some((node, depth)) = pending.pop() {
        count += 1;
        if count > 100_000 || depth > 64 {
            return Err(INVALID.into());
        }
        match node {
            TemplateNode::Element {
                tag,
                attributes,
                children,
                ..
            } => {
                if tag.rsplit(':').next() == Some("attribute") {
                    let types = attributes
                        .iter()
                        .filter(|a| a.name == "type")
                        .collect::<Vec<_>>();
                    if types.len() > 1 {
                        return Err(INVALID.into());
                    }
                    if let Some(value) = types.first() {
                        let Some(TemplateAttributeValue::Literal(name)) = &value.value else {
                            return Err(INVALID.into());
                        };
                        if !primitive(name) {
                            result.insert(name.clone());
                        }
                    }
                }
                pending.extend(children.iter().map(|n| (n, depth + 1)));
            }
            TemplateNode::Result { children, .. }
            | TemplateNode::If { children, .. }
            | TemplateNode::ForEach { children, .. } => {
                pending.extend(children.iter().map(|n| (n, depth + 1)))
            }
            TemplateNode::Choose { branches, .. } => {
                for b in branches {
                    pending.extend(b.children.iter().map(|n| (n, depth + 1)));
                }
            }
            _ => {}
        }
    }
    Ok(result)
}
pub(crate) fn load_template(
    bytes: &[u8],
    json: &str,
    pin: &str,
) -> Result<(DeclarationArtifactMetadata, TemplateArtifact), String> {
    let metadata =
        crate::declaration_artifact::admit_declaration_metadata(json, pin).map_err(|_| INVALID)?;
    if !(bytes.starts_with(b"CEMTPLA2\n") || bytes.starts_with(b"CEMTPLA3\n")) {
        return Err(INVALID.into());
    }
    let compiled = crate::template_artifact::CompiledTemplateArtifact::from_bytes(bytes.to_vec())
        .map_err(|_| INVALID)?;
    if compiled.identity.artifact_version != metadata.template_format {
        return Err(INVALID.into());
    }
    let artifact = compiled
        .reload_pinned(&metadata.template)
        .map_err(|_| INVALID)?;
    Ok((metadata, artifact))
}
/// A trusted build supplies already compiled native scalar facet contracts and
/// original schema/source identities. This does not serialize executable types.
#[cfg(not(target_arch = "wasm32"))]
pub fn build(
    built: BuiltDeclarationArtifact,
    types: Vec<NamedReceiverType>,
) -> Result<BuiltApplicationDeclaration, String> {
    let (_, artifact) = load_template(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )?;
    let type_pins = pins(&types)?;
    if type_pins.is_empty() || required(&artifact)? != type_pins.keys().cloned().collect() {
        return Err(INVALID.into());
    }
    let payload = Payload {
        template: built.template.bytes,
        types,
    };
    let bytes = [
        MAGIC,
        &rmp_serde::to_vec_named(&payload).map_err(|_| INVALID)?,
    ]
    .concat();
    if bytes.len() > MAX_BYTES {
        return Err(INVALID.into());
    }
    let metadata = Metadata {
        contract: CONTRACT.into(),
        declaration_json: built.metadata_json,
        declaration_hash: built.metadata_hash,
        payload_hash: hash(&bytes),
        types: type_pins,
    };
    let metadata_json = serde_json::to_string(&metadata).map_err(|_| INVALID)?;
    let metadata_hash = hash(metadata_json.as_bytes());
    admit(&bytes, &metadata_json, &metadata_hash)?;
    Ok(BuiltApplicationDeclaration {
        bytes,
        metadata_json,
        metadata_hash,
    })
}
pub fn admit(
    bytes: &[u8],
    json: &str,
    pin: &str,
) -> Result<AdmittedApplicationDeclaration, String> {
    if bytes.len() > MAX_BYTES
        || json.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || !valid_hash(pin)
        || hash(json.as_bytes()) != pin
    {
        return Err(INVALID.into());
    }
    let metadata: Metadata = serde_json::from_str(json).map_err(|_| INVALID)?;
    if metadata.contract != CONTRACT
        || metadata.types.is_empty()
        || metadata.types.len() > 64
        || metadata.payload_hash != hash(bytes)
    {
        return Err(INVALID.into());
    }
    let body = &bytes[MAGIC.len()..];
    let mut decoder = rmp_serde::Deserializer::from_read_ref(body);
    decoder.set_max_depth(64);
    let payload = Payload::deserialize(&mut decoder).map_err(|_| INVALID)?;
    // This version is canonical native binary control data. Unknown nested
    // fields, trailing data or alternate shapes cannot disappear on reload.
    if rmp_serde::to_vec_named(&payload).map_err(|_| INVALID)? != body
        || pins(&payload.types)? != metadata.types
    {
        return Err(INVALID.into());
    }
    let (declaration, artifact) = load_template(
        &payload.template,
        &metadata.declaration_json,
        &metadata.declaration_hash,
    )?;
    if required(&artifact)? != metadata.types.keys().cloned().collect() {
        return Err(INVALID.into());
    }
    let types = payload
        .types
        .into_iter()
        .map(|t| (t.name, t.value))
        .collect();
    Ok(AdmittedApplicationDeclaration {
        metadata: declaration,
        artifact,
        types,
    })
}
#[cfg(test)]
#[path = "application_declaration_tests.rs"]
mod tests;
