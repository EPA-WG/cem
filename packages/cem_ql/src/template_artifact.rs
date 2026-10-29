//! Portable precompiled CEM-ML component-template artifacts.
//!
//! The envelope carries stable identity stamps plus binary serialized template
//! IR. Embedded expressions contain the already-lowered [`CompiledQuery`] IR,
//! so reload never invokes the CEM-ML tokenizer or CEM-QL parser.
//! Legacy v1 carries CSS source and adopts it through the native importer.
//! Opt-in v2 instead admits pinned compiled inline CSS trees without reparsing.

#[cfg(test)]
mod application_profile_tests;
#[cfg(test)]
mod compiled_style_tests;

use std::collections::BTreeSet;

use cem_ml::content_cache::ContentHash;
use serde::{Deserialize, Serialize};

use crate::render::{
    compile_template, ChooseBranch, CompileTemplateOptions, CompiledTemplateExpression,
    TemplateArtifact, TemplateAttribute, TemplateAttributePart, TemplateAttributeValue,
    TemplateModuleMapArtifact, TemplateNode, TemplateStylesheetArtifact,
};

pub const CEM_TEMPLATE_ARTIFACT_CONTENT_TYPE: &str =
    "application/vnd.cem.template-artifact+cem-bin";
pub const CEM_TEMPLATE_ARTIFACT_VERSION: &str = "cem-template-artifact/1";
pub const CEM_TEMPLATE_IR_FORMAT: &str = "cem-template-ir-v1";

pub const CEM_COMPILED_STYLE_ARTIFACT_VERSION: &str = "cem-template-artifact/2";
const COMPILED_STYLE_IR_FORMAT: &str = "cem-template-ir-v2";
const COMPILED_ARTIFACT_MAGIC: &[u8] = b"CEMTPLA2\n";
pub const CEM_COMPILED_IMPORT_ARTIFACT_VERSION: &str = "cem-template-artifact/3";
const COMPILED_IMPORT_IR_FORMAT: &str = "cem-template-ir-v3";
const COMPILED_IMPORT_MAGIC: &[u8] = b"CEMTPLA3\n";
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

/// Trusted deployment inputs pinned outside the artifact being admitted.
/// These identities are requirements, not source grants or native owners.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PinnedTemplateArtifactContext {
    pub content_hash: String,
    pub source_hash: String,
    pub cem_ml_version: String,
    pub cem_ql_version: String,
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
    #[serde(default)]
    stylesheets: Vec<TemplateStylesheetArtifact>,
    #[serde(default)]
    module_map: Option<TemplateModuleMapArtifact>,
    diagnostics_json: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompiledStyle {
    scope: Option<String>,
    hash: String,
    bytes: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    imports: Option<cem_ml::compiled_css_imports::CompiledCssImportsArtifact>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompiledStylePayload {
    nodes: Vec<TemplateNode>,
    stylesheets: Vec<CompiledStyle>,
    module_map: Option<TemplateModuleMapArtifact>,
    diagnostics_json: Vec<u8>,
}

/// Opt-in native template with compiled inline CSS. Full/source builds keep v1.
pub fn compile_template_artifact_with_compiled_styles(source: &str, options: &CompileTemplateOptions,
    mode: TemplateArtifactSourceMapMode) -> Result<CompiledTemplateArtifact, String> {
    let mut artifact = compile_template(source, options);
    if mode == TemplateArtifactSourceMapMode::Prod { strip_template_source_maps(&mut artifact); }
    let mut stylesheets = Vec::new();
    for style in &artifact.stylesheets {
        let bytes = cem_ml::compiled_css::encode(style.css_tree(), style.css())?;
        stylesheets.push(CompiledStyle { scope: style.scope.clone(), hash: ContentHash::from_blake3(&bytes).header_value(), bytes, imports: None });
    }
    let mut compiled = encode_template_artifact(source, options, mode, &mut artifact);
    compiled.identity.artifact_version = CEM_COMPILED_STYLE_ARTIFACT_VERSION.into();
    compiled.identity.ir_format = COMPILED_STYLE_IR_FORMAT.into();
    let payload = CompiledStylePayload { nodes: artifact.nodes, stylesheets, module_map: artifact.module_map,
        diagnostics_json: serde_json::to_vec(&artifact.diagnostics).map_err(|e| e.to_string())? };
    let body = rmp_serde::to_vec_named(&payload).map_err(|e| e.to_string())?;
    compiled.bytes = serialize_artifact_bytes(&compiled.identity, &body);
    compiled.content_hash = ContentHash::from_blake3(&compiled.bytes);
    Ok(compiled)
}

/// Build-only upgrade of a compiled template with a closed, pinned import set.
/// The loader is explicit; runtime resolution/response policy remains authoritative.
pub fn compile_template_artifact_imports(
    mut compiled: CompiledTemplateArtifact,
    base_url: &str,
    capability: cem_ml::module_resolution::CemModuleUrlResolutionCapability,
    load: &mut dyn FnMut(
        &cem_ml::css_imports::CssImportRequest,
    ) -> Result<cem_ml::resolver::ResolvedRead, String>,
) -> Result<CompiledTemplateArtifact, String> {
    let artifact = compiled
        .reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(compiled.identity.source_hash.clone()),
            host_bindings: compiled.identity.host_bindings.clone(),
            source_map_mode: compiled.identity.source_map_mode,
        })
        .map_err(|e| e.to_string())?;
    let mut stylesheets = Vec::new();
    for style in &artifact.stylesheets {
        let imports = cem_ml::compiled_css_imports::compile(
            style.css_tree().clone(),
            base_url,
            capability.clone(),
            load,
        )?;
        let bytes = cem_ml::compiled_css::encode(style.css_tree(), style.css())?;
        stylesheets.push(CompiledStyle {
            scope: style.scope.clone(),
            hash: ContentHash::from_blake3(&bytes).header_value(),
            bytes,
            imports: Some(imports),
        });
    }
    compiled.identity.artifact_version = CEM_COMPILED_IMPORT_ARTIFACT_VERSION.into();
    compiled.identity.ir_format = COMPILED_IMPORT_IR_FORMAT.into();
    let payload = CompiledStylePayload {
        nodes: artifact.nodes,
        stylesheets,
        module_map: artifact.module_map,
        diagnostics_json: serde_json::to_vec(&artifact.diagnostics).map_err(|e| e.to_string())?,
    };
    compiled.bytes = serialize_artifact_bytes(
        &compiled.identity,
        &rmp_serde::to_vec_named(&payload).map_err(|e| e.to_string())?,
    );
    validate_size(compiled.bytes.len()).map_err(|e| e.to_string())?;
    compiled.content_hash = ContentHash::from_blake3(&compiled.bytes);
    Ok(compiled)
}


pub fn compile_template_artifact(
    source: &str,
    options: &CompileTemplateOptions,
    source_map_mode: TemplateArtifactSourceMapMode,
) -> CompiledTemplateArtifact {
    #[cfg(test)]
    let mut profile = crate::compile_profile::Span::new("template-artifact/compile");
    let host_bindings = canonical_host_bindings(&options.host_bindings);
    let canonical_options = CompileTemplateOptions {
        host_bindings: host_bindings.clone(),
        skip_cemt_function_bodies: options.skip_cemt_function_bodies,
        source_uri: options.source_uri.clone(),
    };
    let mut artifact = compile_template(source, &canonical_options);
    #[cfg(test)]
    profile.next("template-artifact/encode");
    encode_template_artifact(source, &canonical_options, source_map_mode, &mut artifact)
}

/// Encode an already compiled closure without parsing or lowering it again.
/// The caller owns dependency verification; this preserves the existing wire envelope.
pub(crate) fn encode_template_artifact(
    source: &str,
    options: &CompileTemplateOptions,
    source_map_mode: TemplateArtifactSourceMapMode,
    artifact: &mut TemplateArtifact,
) -> CompiledTemplateArtifact {
    let host_bindings = canonical_host_bindings(&options.host_bindings);
    if source_map_mode == TemplateArtifactSourceMapMode::Prod {
        strip_template_source_maps(artifact);
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
    #[derive(Serialize)]
    struct PayloadRef<'a> {
        nodes: &'a [TemplateNode],
        stylesheets: &'a [TemplateStylesheetArtifact],
        module_map: &'a Option<TemplateModuleMapArtifact>,
        diagnostics_json: Vec<u8>,
    }
    let payload = PayloadRef {
        nodes: &artifact.nodes,
        stylesheets: &artifact.stylesheets,
        module_map: &artifact.module_map,
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
    /// Admit a deployment artifact without requiring original source text.
    /// Every hash/version must match independently supplied build metadata;
    /// failure never requests or compiles a source fallback.
    pub fn reload_pinned(
        &self,
        context: &PinnedTemplateArtifactContext,
    ) -> Result<TemplateArtifact, TemplateArtifactLoadError> {
        if context.content_hash != self.content_hash.header_value()
            || context.source_hash != self.identity.source_hash.header_value()
            || context.cem_ml_version != cem_ml::VERSION
            || context.cem_ql_version != crate::VERSION
        {
            return Err(TemplateArtifactLoadError::hash_mismatch(
                "component-template artifact does not match pinned deployment hashes/versions",
            ));
        }
        let template = self.reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(self.identity.source_hash.clone()),
            host_bindings: context.host_bindings.clone(),
            source_map_mode: context.source_map_mode,
        })?;
        if template.diagnostics.iter().any(|diagnostic| diagnostic.severity.is_hard_violation()) {
            return Err(TemplateArtifactLoadError {
                code: "cem.ql.template_artifact_invalid_build",
                message: "pinned deployment template contains build errors".into(),
            });
        }
        Ok(template)
    }

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
        let payload: TemplateArtifactPayload = if identity.artifact_version != CEM_TEMPLATE_ARTIFACT_VERSION {
            // Share the v1 slice-reader monomorphizations for template/query IR.
            let mut decoder = rmp_serde::Deserializer::from_read_ref(payload_bytes.as_slice()); decoder.set_max_depth(64);
            let payload = CompiledStylePayload::deserialize(&mut decoder).map_err(|e|
                TemplateArtifactLoadError::unsupported(format!("compiled stylesheet payload is invalid: {e}")))?;
            // Probe one marker only; never walk an untrusted trailing object.
            match bool::deserialize(&mut decoder) {
                Err(rmp_serde::decode::Error::InvalidMarkerRead(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {},
                _ => return Err(TemplateArtifactLoadError::unsupported("compiled stylesheet payload has trailing bytes")),
            }
            let imports_required = identity.artifact_version == CEM_COMPILED_IMPORT_ARTIFACT_VERSION;
            let stylesheets = payload.stylesheets.into_iter().map(|style| {
                if style.imports.is_some() != imports_required {
                    return Err(TemplateArtifactLoadError::unsupported("compiled stylesheet closure/format mismatch"));
                }
                let mut admitted = TemplateStylesheetArtifact::from_compiled_css(&style.bytes, &style.hash, style.scope)
                    .map_err(TemplateArtifactLoadError::unsupported)?;
                admitted.compiled_imports = style.imports.map(cem_ml::compiled_css_imports::CompiledCssImports::admit)
                    .transpose().map_err(TemplateArtifactLoadError::unsupported)?.map(std::sync::Arc::new);
                Ok(admitted)
            }).collect::<Result<Vec<_>, TemplateArtifactLoadError>>()?;
            TemplateArtifactPayload { nodes: payload.nodes, stylesheets, module_map: payload.module_map, diagnostics_json: payload.diagnostics_json }
        } else {
            #[cfg(all(target_arch = "wasm32", feature = "component-runtime"))]
            { return Err(TemplateArtifactLoadError::unsupported("component runtime requires compiled CSS artifacts")); }
            #[cfg(not(all(target_arch = "wasm32", feature = "component-runtime")))]
            rmp_serde::from_slice(&payload_bytes).map_err(|error| TemplateArtifactLoadError::unsupported(format!(
                "component-template IR payload is invalid: {error}")))?
        };
        let diagnostics = serde_json::from_slice(&payload.diagnostics_json).map_err(|error| {
            TemplateArtifactLoadError::unsupported(format!(
                "component-template diagnostics payload is invalid: {error}"
            ))
        })?;
        Ok(TemplateArtifact {
            nodes: payload.nodes,
            stylesheets: payload.stylesheets,
            module_map: payload.module_map,
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
            || !((self.artifact_version == CEM_TEMPLATE_ARTIFACT_VERSION && self.ir_format == CEM_TEMPLATE_IR_FORMAT)
                || (self.artifact_version == CEM_COMPILED_STYLE_ARTIFACT_VERSION && self.ir_format == COMPILED_STYLE_IR_FORMAT)
                || (self.artifact_version == CEM_COMPILED_IMPORT_ARTIFACT_VERSION && self.ir_format == COMPILED_IMPORT_IR_FORMAT))
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
    bytes.extend_from_slice(if identity.artifact_version == CEM_COMPILED_IMPORT_ARTIFACT_VERSION { COMPILED_IMPORT_MAGIC } else if identity.artifact_version == CEM_COMPILED_STYLE_ARTIFACT_VERSION { COMPILED_ARTIFACT_MAGIC } else { ARTIFACT_MAGIC });
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
    let (magic, version) = if bytes.starts_with(COMPILED_IMPORT_MAGIC) {
        (COMPILED_IMPORT_MAGIC, CEM_COMPILED_IMPORT_ARTIFACT_VERSION)
    } else if bytes.starts_with(COMPILED_ARTIFACT_MAGIC) {
        (COMPILED_ARTIFACT_MAGIC, CEM_COMPILED_STYLE_ARTIFACT_VERSION)
    } else { (ARTIFACT_MAGIC, CEM_TEMPLATE_ARTIFACT_VERSION) };
    reader.read_magic(magic)?;
    let content_type = reader.read_string()?;
    let artifact_version = reader.read_string()?;
    let ir_format = reader.read_string()?;
    if artifact_version != version {
        return Err(TemplateArtifactLoadError::unsupported("template artifact magic/version mismatch"));
    }
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

pub(crate) fn strip_template_source_maps(artifact: &mut TemplateArtifact) {
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
            }
            | TemplateNode::Result {
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
            TemplateNode::Variable {
                select, source_map, ..
            } => {
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
