//! Compiled CEM-QL artifact shell.

use cem_ml::content_cache::{CacheMode, ContentHash};

use crate::api::CompileContext;
use crate::ir::deserialize::IrDeserializer;
use crate::ir::serialize::IrSerializer;
use crate::ir::CompiledQuery;
use crate::parser::{Parser, SurfaceNode};
use crate::resolve::ImportKind;

pub const CEM_QL_ARTIFACT_CONTENT_TYPE: &str = "application/vnd.cem.query-artifact+cem-bin";
pub const CEM_QL_ARTIFACT_VERSION: &str = "cem-ql-artifact/1";
pub const CEM_QL_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifact {
    pub format: QueryArtifactFormat,
    pub identity: CompiledArtifactIdentity,
    pub content_hash: ContentHash,
    pub bytes: Vec<u8>,
}

impl CompiledArtifact {
    pub fn from_query(query: &CompiledQuery) -> Self {
        Self::from_query_with_context(query, &CompileContext::default())
    }

    pub fn from_query_with_context(query: &CompiledQuery, context: &CompileContext) -> Self {
        let ir_bytes = IrSerializer::serialize(query);
        let identity = CompiledArtifactIdentity::from_query(query, context);
        let bytes = serialize_artifact_bytes(&identity, &ir_bytes);
        let content_hash = ContentHash::from_blake3(&bytes);
        Self {
            format: QueryArtifactFormat::CemQlIrV1,
            identity,
            content_hash,
            bytes,
        }
    }

    pub fn reload(&self) -> Result<CompiledQuery, ArtifactLoadError> {
        self.reload_with_context(&CompileContext::default())
    }

    pub fn reload_with_context(
        &self,
        context: &CompileContext,
    ) -> Result<CompiledQuery, ArtifactLoadError> {
        if self.format != QueryArtifactFormat::CemQlIrV1 {
            return Err(ArtifactLoadError::unsupported(
                "unsupported CEM-QL artifact format",
            ));
        }
        let actual = ContentHash::from_blake3(&self.bytes);
        if actual != self.content_hash {
            return Err(ArtifactLoadError::artifact_hash_mismatch(
                "compiled artifact hash mismatch",
            ));
        }
        let (identity, ir_bytes) = deserialize_artifact_bytes(&self.bytes)?;
        if identity != self.identity {
            return Err(ArtifactLoadError::artifact_hash_mismatch(
                "compiled artifact identity does not match artifact header",
            ));
        }
        identity.validate_against_context(context)?;
        IrDeserializer::deserialize(&ir_bytes).map_err(ArtifactLoadError::unsupported)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryArtifactFormat {
    CemQlIrV1,
}

impl QueryArtifactFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            QueryArtifactFormat::CemQlIrV1 => "cem-ql-ir-v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifactIdentity {
    pub content_type: String,
    pub artifact_version: String,
    pub ir_format: String,
    pub schema_uri: String,
    pub schema_version: String,
    pub compiler_version: String,
    pub source_hash: ContentHash,
    pub source_uri: Option<String>,
    pub module_uri: Option<String>,
    pub cache_mode: String,
    pub source_map_mode: String,
    pub import_policy_stamp: String,
    pub import_closure: String,
    pub stdlib_overlay_fingerprint: String,
    pub type_profile: String,
}

impl CompiledArtifactIdentity {
    fn from_query(query: &CompiledQuery, context: &CompileContext) -> Self {
        Self {
            content_type: CEM_QL_ARTIFACT_CONTENT_TYPE.to_owned(),
            artifact_version: CEM_QL_ARTIFACT_VERSION.to_owned(),
            ir_format: QueryArtifactFormat::CemQlIrV1.as_str().to_owned(),
            schema_uri: cem_ml::schema::registry::CEM_QL_SCHEMA_URI.to_owned(),
            schema_version: CEM_QL_SCHEMA_VERSION.to_owned(),
            compiler_version: crate::VERSION.to_owned(),
            source_hash: ContentHash::from_blake3(query.source.as_bytes()),
            source_uri: context.source_uri.clone(),
            module_uri: module_uri_from_source(&query.source),
            cache_mode: cache_mode_stamp(context.cache_mode).to_owned(),
            source_map_mode: source_map_mode(context).to_owned(),
            import_policy_stamp: context.import_policy.cache_stamp(),
            import_closure: import_closure_stamp(&query.source, context),
            stdlib_overlay_fingerprint: stdlib_overlay_fingerprint(context),
            type_profile: context.type_config.cache_stamp(),
        }
    }

    fn validate_against_context(&self, context: &CompileContext) -> Result<(), ArtifactLoadError> {
        let expected_identity = CompiledArtifactIdentity {
            source_hash: self.source_hash.clone(),
            source_uri: self.source_uri.clone(),
            module_uri: self.module_uri.clone(),
            import_closure: self.import_closure.clone(),
            ..CompiledArtifactIdentity::from_context_only(context)
        };
        if self.content_type != expected_identity.content_type
            || self.artifact_version != expected_identity.artifact_version
            || self.ir_format != expected_identity.ir_format
            || self.schema_uri != expected_identity.schema_uri
            || self.schema_version != expected_identity.schema_version
            || self.compiler_version != expected_identity.compiler_version
        {
            return Err(ArtifactLoadError::artifact_hash_mismatch(
                "compiled artifact format/schema/compiler identity does not match this runtime",
            ));
        }
        if self.import_policy_stamp != expected_identity.import_policy_stamp
            || self.stdlib_overlay_fingerprint != expected_identity.stdlib_overlay_fingerprint
            || self.type_profile != expected_identity.type_profile
            || self.cache_mode != expected_identity.cache_mode
            || self.source_map_mode != expected_identity.source_map_mode
        {
            return Err(ArtifactLoadError::policy_mismatch(
                "compiled artifact policy/type/profile stamps do not match active context",
            ));
        }
        if let Some(expected_uri) = context.source_uri.as_ref() {
            if self.source_uri.as_deref() != Some(expected_uri.as_str()) {
                return Err(ArtifactLoadError::artifact_hash_mismatch(
                    "compiled artifact source URI does not match active source URI",
                ));
            }
        }
        if let Some(expected_hash) = context.expected_source_hash.as_ref() {
            if &self.source_hash != expected_hash {
                return Err(ArtifactLoadError::artifact_hash_mismatch(
                    "compiled artifact source hash does not match active source bytes",
                ));
            }
        }
        Ok(())
    }

    fn from_context_only(context: &CompileContext) -> Self {
        Self {
            content_type: CEM_QL_ARTIFACT_CONTENT_TYPE.to_owned(),
            artifact_version: CEM_QL_ARTIFACT_VERSION.to_owned(),
            ir_format: QueryArtifactFormat::CemQlIrV1.as_str().to_owned(),
            schema_uri: cem_ml::schema::registry::CEM_QL_SCHEMA_URI.to_owned(),
            schema_version: CEM_QL_SCHEMA_VERSION.to_owned(),
            compiler_version: crate::VERSION.to_owned(),
            source_hash: ContentHash::from_blake3(b""),
            source_uri: None,
            module_uri: None,
            cache_mode: cache_mode_stamp(context.cache_mode).to_owned(),
            source_map_mode: source_map_mode(context).to_owned(),
            import_policy_stamp: context.import_policy.cache_stamp(),
            import_closure: String::new(),
            stdlib_overlay_fingerprint: stdlib_overlay_fingerprint(context),
            type_profile: context.type_config.cache_stamp(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactLoadError {
    pub code: &'static str,
    pub message: String,
}

impl ArtifactLoadError {
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.unsupported",
            message: message.into(),
        }
    }

    fn artifact_hash_mismatch(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.artifact_hash_mismatch",
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

const ARTIFACT_MAGIC: &[u8] = b"CEMQLART1\n";

fn serialize_artifact_bytes(identity: &CompiledArtifactIdentity, ir_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(ARTIFACT_MAGIC);
    write_identity(&mut out, identity);
    write_bytes(&mut out, ir_bytes);
    out
}

fn deserialize_artifact_bytes(
    bytes: &[u8],
) -> Result<(CompiledArtifactIdentity, Vec<u8>), ArtifactLoadError> {
    let mut reader = ArtifactReader::new(bytes);
    reader.read_magic(ARTIFACT_MAGIC)?;
    let identity = read_identity(&mut reader)?;
    let ir_bytes = reader.read_vec()?;
    if !reader.is_empty() {
        return Err(ArtifactLoadError::unsupported(
            "compiled artifact envelope has trailing bytes",
        ));
    }
    Ok((identity, ir_bytes))
}

fn write_identity(out: &mut Vec<u8>, identity: &CompiledArtifactIdentity) {
    write_string(out, &identity.content_type);
    write_string(out, &identity.artifact_version);
    write_string(out, &identity.ir_format);
    write_string(out, &identity.schema_uri);
    write_string(out, &identity.schema_version);
    write_string(out, &identity.compiler_version);
    write_string(out, &identity.source_hash.scheme);
    write_string(out, &identity.source_hash.hex);
    write_optional_string(out, identity.source_uri.as_deref());
    write_optional_string(out, identity.module_uri.as_deref());
    write_string(out, &identity.cache_mode);
    write_string(out, &identity.source_map_mode);
    write_string(out, &identity.import_policy_stamp);
    write_string(out, &identity.import_closure);
    write_string(out, &identity.stdlib_overlay_fingerprint);
    write_string(out, &identity.type_profile);
}

fn read_identity(
    reader: &mut ArtifactReader<'_>,
) -> Result<CompiledArtifactIdentity, ArtifactLoadError> {
    Ok(CompiledArtifactIdentity {
        content_type: reader.read_string()?,
        artifact_version: reader.read_string()?,
        ir_format: reader.read_string()?,
        schema_uri: reader.read_string()?,
        schema_version: reader.read_string()?,
        compiler_version: reader.read_string()?,
        source_hash: ContentHash {
            scheme: reader.read_string()?,
            hex: reader.read_string()?,
        },
        source_uri: reader.read_optional_string()?,
        module_uri: reader.read_optional_string()?,
        cache_mode: reader.read_string()?,
        source_map_mode: reader.read_string()?,
        import_policy_stamp: reader.read_string()?,
        import_closure: reader.read_string()?,
        stdlib_overlay_fingerprint: reader.read_string()?,
        type_profile: reader.read_string()?,
    })
}

fn module_uri_from_source(source: &str) -> Option<String> {
    Parser::new(source)
        .parse_module()
        .module
        .nodes
        .into_iter()
        .find_map(|node| match node {
            SurfaceNode::Module(module) if !module.uri.trim().is_empty() => Some(module.uri),
            _ => None,
        })
}

fn import_closure_stamp(source: &str, context: &CompileContext) -> String {
    let parsed = Parser::new(source).parse_module();
    let mut imports = parsed
        .module
        .nodes
        .iter()
        .filter_map(|node| match node {
            SurfaceNode::Import(import) => {
                let alias = import.alias.as_deref().unwrap_or("");
                let status = match context.import_policy.resolve_import(import) {
                    Ok(resolution) => match resolution.kind {
                        ImportKind::PlatformStdlib => "platform-stdlib",
                        ImportKind::PluginRegistry => "plugin-registry",
                        ImportKind::External => "external",
                    }
                    .to_owned(),
                    Err(diagnostic) => format!("diagnostic:{}", diagnostic.code),
                };
                Some(format!(
                    "{}={}=>{}",
                    stamped_value(alias),
                    stamped_value(&import.uri),
                    status
                ))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    imports.sort();
    format!("imports/1;{}", imports.join(";"))
}

fn stdlib_overlay_fingerprint(context: &CompileContext) -> String {
    context
        .overlay
        .fingerprint
        .as_ref()
        .map(|fingerprint| fingerprint.0.clone())
        .unwrap_or_else(|| format!("cem:stdlib/all-known@{}", crate::VERSION))
}

fn source_map_mode(context: &CompileContext) -> &'static str {
    if context.source_map_base.current().is_some() {
        "dev"
    } else {
        "none"
    }
}

fn cache_mode_stamp(mode: CacheMode) -> &'static str {
    match mode {
        CacheMode::Dev => "dev",
        CacheMode::Prod => "prod",
    }
}

fn stamped_value(value: &str) -> String {
    format!("{}:{}", value.len(), value)
}

fn write_optional_string(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            out.push(1);
            write_string(out, value);
        }
        None => out.push(0),
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_bytes(out, value.as_bytes());
}

fn write_bytes(out: &mut Vec<u8>, value: &[u8]) {
    write_u32(out, value.len());
    out.extend_from_slice(value);
}

fn write_u32(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value.min(u32::MAX as usize) as u32).to_le_bytes());
}

struct ArtifactReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> ArtifactReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn read_magic(&mut self, magic: &[u8]) -> Result<(), ArtifactLoadError> {
        let actual = self.read_bytes(magic.len())?;
        if actual == magic {
            Ok(())
        } else {
            Err(ArtifactLoadError::unsupported(
                "compiled artifact envelope magic mismatch",
            ))
        }
    }

    fn read_optional_string(&mut self) -> Result<Option<String>, ArtifactLoadError> {
        let tag = self.read_bytes(1)?[0];
        match tag {
            0 => Ok(None),
            1 => self.read_string().map(Some),
            _ => Err(ArtifactLoadError::unsupported(
                "compiled artifact optional string tag is invalid",
            )),
        }
    }

    fn read_string(&mut self) -> Result<String, ArtifactLoadError> {
        let bytes = self.read_vec()?;
        String::from_utf8(bytes).map_err(|_| {
            ArtifactLoadError::unsupported("compiled artifact string is not valid UTF-8")
        })
    }

    fn read_vec(&mut self) -> Result<Vec<u8>, ArtifactLoadError> {
        let len = self.read_u32()? as usize;
        Ok(self.read_bytes(len)?.to_vec())
    }

    fn read_u32(&mut self) -> Result<u32, ArtifactLoadError> {
        let bytes = self.read_bytes(4)?;
        let mut raw = [0u8; 4];
        raw.copy_from_slice(bytes);
        Ok(u32::from_le_bytes(raw))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ArtifactLoadError> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or_else(|| ArtifactLoadError::unsupported("compiled artifact length overflow"))?;
        if end > self.bytes.len() {
            return Err(ArtifactLoadError::unsupported(
                "compiled artifact ended unexpectedly",
            ));
        }
        let out = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(out)
    }

    fn is_empty(&self) -> bool {
        self.cursor == self.bytes.len()
    }
}
