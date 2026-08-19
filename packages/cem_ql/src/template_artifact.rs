//! Portable precompiled CEM-ML component-template artifacts.
//!
//! The envelope carries stable identity stamps plus binary serialized template
//! IR. Embedded expressions contain the already-lowered [`CompiledQuery`] IR,
//! so reload never invokes the CEM-ML tokenizer or CEM-QL parser.

use std::collections::BTreeSet;

use cem_ml::content_cache::ContentHash;
use serde::{Deserialize, Serialize};

use crate::render::{
    compile_template, ChooseBranch, CompileTemplateOptions, CompiledTemplateExpression,
    TemplateArtifact, TemplateAttribute, TemplateAttributePart, TemplateAttributeValue,
    TemplateNode,
};

pub const CEM_TEMPLATE_ARTIFACT_CONTENT_TYPE: &str =
    "application/vnd.cem.template-artifact+cem-bin";
pub const CEM_TEMPLATE_ARTIFACT_VERSION: &str = "cem-template-artifact/1";
pub const CEM_TEMPLATE_IR_FORMAT: &str = "cem-template-ir-v1";

const ARTIFACT_MAGIC: &[u8] = b"CEMTPLA1\n";
const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateArtifactSourceMapMode {
    Dev,
    Prod,
}

impl TemplateArtifactSourceMapMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Prod => "prod",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledTemplateArtifactIdentity {
    pub content_type: String,
    pub artifact_version: String,
    pub ir_format: String,
    pub cem_ml_version: String,
    pub cem_ql_version: String,
    pub source_hash: ContentHash,
    pub source_map_mode: TemplateArtifactSourceMapMode,
    pub host_bindings: Vec<String>,
    pub skip_cemt_function_bodies: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledTemplateArtifact {
    pub identity: CompiledTemplateArtifactIdentity,
    pub content_hash: ContentHash,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateArtifactLoadContext {
    pub expected_source_hash: Option<ContentHash>,
    pub host_bindings: Vec<String>,
    pub source_map_mode: TemplateArtifactSourceMapMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateArtifactLoadError {
    pub code: &'static str,
    pub message: String,
}

impl TemplateArtifactLoadError {
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.template_artifact_unsupported",
            message: message.into(),
        }
    }

    fn hash_mismatch(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.template_artifact_hash_mismatch",
            message: message.into(),
        }
    }

    fn policy_mismatch(message: impl Into<String>) -> Self {
        Self {
            code: "cem.cc.policy_mismatch",
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TemplateArtifactLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TemplateArtifactLoadError {}

#[derive(Debug, Serialize, Deserialize)]
struct TemplateArtifactPayload {
    nodes: Vec<TemplateNode>,
    diagnostics_json: Vec<u8>,
}

pub fn compile_template_artifact(
    source: &str,
    options: &CompileTemplateOptions,
    source_map_mode: TemplateArtifactSourceMapMode,
) -> CompiledTemplateArtifact {
    let host_bindings = canonical_host_bindings(&options.host_bindings);
    let canonical_options = CompileTemplateOptions {
        host_bindings: host_bindings.clone(),
        skip_cemt_function_bodies: options.skip_cemt_function_bodies,
    };
    let mut artifact = compile_template(source, &canonical_options);
    if source_map_mode == TemplateArtifactSourceMapMode::Prod {
        strip_template_source_maps(&mut artifact);
    }
    let identity = CompiledTemplateArtifactIdentity {
        content_type: CEM_TEMPLATE_ARTIFACT_CONTENT_TYPE.to_owned(),
        artifact_version: CEM_TEMPLATE_ARTIFACT_VERSION.to_owned(),
        ir_format: CEM_TEMPLATE_IR_FORMAT.to_owned(),
        cem_ml_version: cem_ml::VERSION.to_owned(),
        cem_ql_version: crate::VERSION.to_owned(),
        source_hash: ContentHash::from_blake3(source.as_bytes()),
        source_map_mode,
        host_bindings,
        skip_cemt_function_bodies: options.skip_cemt_function_bodies,
    };
    let payload = TemplateArtifactPayload {
        nodes: artifact.nodes,
        diagnostics_json: serde_json::to_vec(&artifact.diagnostics)
            .expect("CEM diagnostics serialize as JSON"),
    };
    let payload_bytes = rmp_serde::to_vec_named(&payload)
        .expect("template artifact IR serializes with the pinned binary codec");
    let bytes = serialize_artifact_bytes(&identity, &payload_bytes);
    let content_hash = ContentHash::from_blake3(&bytes);
    CompiledTemplateArtifact {
        identity,
        content_hash,
        bytes,
    }
}

impl CompiledTemplateArtifact {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, TemplateArtifactLoadError> {
        validate_size(bytes.len())?;
        let (identity, _) = deserialize_artifact_bytes(&bytes)?;
        let content_hash = ContentHash::from_blake3(&bytes);
        Ok(Self {
            identity,
            content_hash,
            bytes,
        })
    }

    pub fn reload(
        &self,
        context: &TemplateArtifactLoadContext,
    ) -> Result<TemplateArtifact, TemplateArtifactLoadError> {
        validate_size(self.bytes.len())?;
        let actual_hash = ContentHash::from_blake3(&self.bytes);
        if actual_hash != self.content_hash {
            return Err(TemplateArtifactLoadError::hash_mismatch(
                "component-template artifact hash mismatch",
            ));
        }
        let (identity, payload_bytes) = deserialize_artifact_bytes(&self.bytes)?;
        if identity != self.identity {
            return Err(TemplateArtifactLoadError::hash_mismatch(
                "component-template artifact identity does not match its envelope",
            ));
        }
        identity.validate(context)?;
        let payload: TemplateArtifactPayload =
            rmp_serde::from_slice(&payload_bytes).map_err(|error| {
                TemplateArtifactLoadError::unsupported(format!(
                    "component-template IR payload is invalid: {error}"
                ))
            })?;
        let diagnostics = serde_json::from_slice(&payload.diagnostics_json).map_err(|error| {
            TemplateArtifactLoadError::unsupported(format!(
                "component-template diagnostics payload is invalid: {error}"
            ))
        })?;
        Ok(TemplateArtifact {
            nodes: payload.nodes,
            diagnostics,
        })
    }
}

impl CompiledTemplateArtifactIdentity {
    fn validate(
        &self,
        context: &TemplateArtifactLoadContext,
    ) -> Result<(), TemplateArtifactLoadError> {
        if self.content_type != CEM_TEMPLATE_ARTIFACT_CONTENT_TYPE
            || self.artifact_version != CEM_TEMPLATE_ARTIFACT_VERSION
            || self.ir_format != CEM_TEMPLATE_IR_FORMAT
            || self.cem_ml_version != cem_ml::VERSION
            || self.cem_ql_version != crate::VERSION
        {
            return Err(TemplateArtifactLoadError::hash_mismatch(
                "component-template format or compiler version does not match this runtime",
            ));
        }
        if self.source_map_mode != context.source_map_mode
            || self.host_bindings != canonical_host_bindings(&context.host_bindings)
        {
            return Err(TemplateArtifactLoadError::policy_mismatch(
                "component-template binding or source-map policy does not match this runtime",
            ));
        }
        if let Some(expected_source_hash) = context.expected_source_hash.as_ref() {
            if &self.source_hash != expected_source_hash {
                return Err(TemplateArtifactLoadError::hash_mismatch(
                    "component-template source hash does not match active source bytes",
                ));
            }
        }
        Ok(())
    }
}

fn canonical_host_bindings(bindings: &[String]) -> Vec<String> {
    bindings
        .iter()
        .filter(|binding| !binding.is_empty())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn serialize_artifact_bytes(
    identity: &CompiledTemplateArtifactIdentity,
    payload: &[u8],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(ARTIFACT_MAGIC);
    write_string(&mut bytes, &identity.content_type);
    write_string(&mut bytes, &identity.artifact_version);
    write_string(&mut bytes, &identity.ir_format);
    write_string(&mut bytes, &identity.cem_ml_version);
    write_string(&mut bytes, &identity.cem_ql_version);
    write_string(&mut bytes, &identity.source_hash.scheme);
    write_string(&mut bytes, &identity.source_hash.hex);
    write_string(&mut bytes, identity.source_map_mode.as_str());
    write_strings(&mut bytes, &identity.host_bindings);
    bytes.push(u8::from(identity.skip_cemt_function_bodies));
    write_bytes(&mut bytes, payload);
    bytes
}

fn deserialize_artifact_bytes(
    bytes: &[u8],
) -> Result<(CompiledTemplateArtifactIdentity, Vec<u8>), TemplateArtifactLoadError> {
    let mut reader = ArtifactReader::new(bytes);
    reader.read_magic(ARTIFACT_MAGIC)?;
    let content_type = reader.read_string()?;
    let artifact_version = reader.read_string()?;
    let ir_format = reader.read_string()?;
    let cem_ml_version = reader.read_string()?;
    let cem_ql_version = reader.read_string()?;
    let source_hash = ContentHash {
        scheme: reader.read_string()?,
        hex: reader.read_string()?,
    };
    let source_map_mode = match reader.read_string()?.as_str() {
        "dev" => TemplateArtifactSourceMapMode::Dev,
        "prod" => TemplateArtifactSourceMapMode::Prod,
        _ => {
            return Err(TemplateArtifactLoadError::unsupported(
                "component-template source-map mode is unsupported",
            ))
        }
    };
    let host_bindings = reader.read_strings()?;
    let skip_cemt_function_bodies = match reader.read_byte()? {
        0 => false,
        1 => true,
        _ => {
            return Err(TemplateArtifactLoadError::unsupported(
                "component-template compiler option tag is invalid",
            ))
        }
    };
    let payload = reader.read_vec()?;
    if !reader.is_empty() {
        return Err(TemplateArtifactLoadError::unsupported(
            "component-template artifact envelope has trailing bytes",
        ));
    }
    Ok((
        CompiledTemplateArtifactIdentity {
            content_type,
            artifact_version,
            ir_format,
            cem_ml_version,
            cem_ql_version,
            source_hash,
            source_map_mode,
            host_bindings,
            skip_cemt_function_bodies,
        },
        payload,
    ))
}

fn validate_size(size: usize) -> Result<(), TemplateArtifactLoadError> {
    if size > MAX_ARTIFACT_BYTES {
        Err(TemplateArtifactLoadError::unsupported(
            "component-template artifact exceeds the 64 MiB runtime limit",
        ))
    } else {
        Ok(())
    }
}

fn write_strings(bytes: &mut Vec<u8>, values: &[String]) {
    write_u32(bytes, values.len());
    for value in values {
        write_string(bytes, value);
    }
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_bytes(bytes, value.as_bytes());
}

fn write_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    write_u32(bytes, value.len());
    bytes.extend_from_slice(value);
}

fn write_u32(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend_from_slice(&(value.min(u32::MAX as usize) as u32).to_le_bytes());
}

struct ArtifactReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> ArtifactReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn read_magic(&mut self, magic: &[u8]) -> Result<(), TemplateArtifactLoadError> {
        if self.read_bytes(magic.len())? == magic {
            Ok(())
        } else {
            Err(TemplateArtifactLoadError::unsupported(
                "component-template artifact magic does not match",
            ))
        }
    }

    fn read_byte(&mut self) -> Result<u8, TemplateArtifactLoadError> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_string(&mut self) -> Result<String, TemplateArtifactLoadError> {
        String::from_utf8(self.read_vec()?).map_err(|_| {
            TemplateArtifactLoadError::unsupported(
                "component-template artifact string is not valid UTF-8",
            )
        })
    }

    fn read_strings(&mut self) -> Result<Vec<String>, TemplateArtifactLoadError> {
        let count = self.read_u32()? as usize;
        if count > self.bytes.len() {
            return Err(TemplateArtifactLoadError::unsupported(
                "component-template host-binding count is invalid",
            ));
        }
        (0..count).map(|_| self.read_string()).collect()
    }

    fn read_vec(&mut self) -> Result<Vec<u8>, TemplateArtifactLoadError> {
        let len = self.read_u32()? as usize;
        Ok(self.read_bytes(len)?.to_vec())
    }

    fn read_u32(&mut self) -> Result<u32, TemplateArtifactLoadError> {
        let mut raw = [0u8; 4];
        raw.copy_from_slice(self.read_bytes(4)?);
        Ok(u32::from_le_bytes(raw))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], TemplateArtifactLoadError> {
        let end = self.cursor.checked_add(len).ok_or_else(|| {
            TemplateArtifactLoadError::unsupported("component-template artifact length overflow")
        })?;
        if end > self.bytes.len() {
            return Err(TemplateArtifactLoadError::unsupported(
                "component-template artifact ended unexpectedly",
            ));
        }
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }

    fn is_empty(&self) -> bool {
        self.cursor == self.bytes.len()
    }
}

fn strip_template_source_maps(artifact: &mut TemplateArtifact) {
    for diagnostic in &mut artifact.diagnostics {
        diagnostic.source_map = None;
    }
    strip_nodes(&mut artifact.nodes);
}

fn strip_nodes(nodes: &mut [TemplateNode]) {
    for node in nodes {
        match node {
            TemplateNode::Element {
                attributes,
                children,
                source_map,
                ..
            } => {
                source_map.frames.clear();
                for attribute in attributes {
                    strip_attribute(attribute);
                }
                strip_nodes(children);
            }
            TemplateNode::Text { source_map, .. } | TemplateNode::Comment { source_map, .. } => {
                source_map.frames.clear();
            }
            TemplateNode::ProjectPayload { select, source_map } => {
                source_map.frames.clear();
                if let Some(expression) = select {
                    strip_expression(expression);
                }
            }
            TemplateNode::Expression(expression) => strip_expression(expression),
            TemplateNode::If {
                test,
                children,
                source_map,
            }
            | TemplateNode::ForEach {
                select: test,
                children,
                source_map,
                ..
            } => {
                source_map.frames.clear();
                if let Some(expression) = test {
                    strip_expression(expression);
                }
                strip_nodes(children);
            }
            TemplateNode::Choose {
                branches,
                source_map,
            } => {
                source_map.frames.clear();
                for ChooseBranch { test, children } in branches {
                    if let Some(expression) = test {
                        strip_expression(expression);
                    }
                    strip_nodes(children);
                }
            }
        }
    }
}

fn strip_attribute(attribute: &mut TemplateAttribute) {
    attribute.source_map.frames.clear();
    match attribute.value.as_mut() {
        Some(TemplateAttributeValue::Expression(expression)) => strip_expression(expression),
        Some(TemplateAttributeValue::Template(parts)) => {
            for part in parts {
                if let TemplateAttributePart::Expression(expression) = part {
                    strip_expression(expression);
                }
            }
        }
        Some(TemplateAttributeValue::Literal(_)) | None => {}
    }
}

fn strip_expression(expression: &mut CompiledTemplateExpression) {
    expression.source_map.frames.clear();
    if let Some(query) = expression.query.as_mut() {
        for source_map in &mut query.tree.source_maps {
            source_map.frames.clear();
        }
    }
}
