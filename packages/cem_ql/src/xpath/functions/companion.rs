//! CEMT-owned binding manifests with opaque, independently identified XPath
//! programs. JSON below is explicitly control metadata, never a data AST or
//! expression program. XPath owns program encoding, decoding and host checks.
use super::*;
use cem_ml::{
    content_cache::HASH_SCHEME,
    source_map::{FrameSpan, SourceMapStack},
    transform_template::TransformTemplateXPathVariableBinding,
    validation::xpath::{XPathAttachment, XPathExpandedName},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const COMPANION_CONTENT_TYPE: &str = "application/vnd.cem.cemt-xpath-functions+cem-bin";
pub const COMPANION_VERSION: &str = "cemt-xpath-functions/1";
pub const MAX_COMPANION_BYTES: usize = 4 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 128 * 1024;
const MAGIC: &[u8; 9] = b"CEMXPFN1\n";

mod host;
pub use host::CemtXPathCompanions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for CompanionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for CompanionError {}
impl CompanionError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.xpath_companion_invalid",
            message: message.into(),
        }
    }
    pub(crate) fn limit() -> Self {
        Self {
            code: "cem.ql.xpath_companion_limit",
            message: "XPath function companion exceeds byte, item or handle limits".into(),
        }
    }
    fn mismatch(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.xpath_companion_identity_mismatch",
            message: message.into(),
        }
    }
}
type Result<T> = std::result::Result<T, CompanionError>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    content_type: String,
    version: String,
    cem_ml_version: String,
    cem_ql_version: String,
    source_hash: String,
    source_uri: String,
    functions: Vec<Function>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Function {
    name: String,
    parameters: Vec<Parameter>,
    returns: ParamType,
    nullable: bool,
    context: Option<String>,
    variables: Vec<Variable>,
    source_map: SourceMapStack,
    program_hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Variable {
    binding: String,
    namespace_uri: Option<String>,
    local_name: String,
}

impl CemtXPathFunctions {
    /// Export compiler-owned declarations plus opaque XPath program bytes, not
    /// executable callbacks, source text or runtime data.
    pub fn to_companion_bytes(&self) -> Result<Vec<u8>> {
        let manifest = Manifest {
            content_type: COMPANION_CONTENT_TYPE.into(),
            version: COMPANION_VERSION.into(),
            cem_ml_version: cem_ml::VERSION.into(),
            cem_ql_version: crate::VERSION.into(),
            source_hash: self.source_hash.header_value(),
            source_uri: self.source_uri.clone(),
            functions: self
                .functions
                .iter()
                .map(|function| Function {
                    name: function.name.clone(),
                    parameters: function.parameters.clone(),
                    returns: function.returns,
                    nullable: function.nullable,
                    context: function.invocation.context_binding.clone(),
                    variables: function
                        .invocation
                        .variable_bindings
                        .iter()
                        .map(|binding| Variable {
                            binding: binding.host_binding.clone(),
                            namespace_uri: binding.expanded_name.namespace_uri.clone(),
                            local_name: binding.expanded_name.local_name.clone(),
                        })
                        .collect(),
                    source_map: function.invocation.source_map.clone(),
                    program_hash: function.artifact.content_hash().header_value(),
                })
                .collect(),
        };
        // Bound serialization allocations before encoding the manifest.
        validate_manifest(&manifest, &self.source_hash)?;
        let metadata =
            serde_json::to_vec(&manifest).map_err(|e| CompanionError::invalid(e.to_string()))?;
        if metadata.len() > MAX_MANIFEST_BYTES {
            return Err(CompanionError::limit());
        }
        let total = MAGIC.len()
            + 4
            + metadata.len()
            + self
                .functions
                .iter()
                .map(|f| 4 + f.artifact.bytes().len())
                .sum::<usize>();
        if total > MAX_COMPANION_BYTES {
            return Err(CompanionError::limit());
        }
        let mut bytes = Vec::with_capacity(total);
        bytes.extend_from_slice(MAGIC);
        write_blob(&mut bytes, &metadata);
        for function in &self.functions {
            write_blob(&mut bytes, function.artifact.bytes());
        }
        Ok(bytes)
    }

    /// Expected hashes must come from a trusted compiler result or manifest.
    /// Hash integrity is not publisher authentication. Reload installs nothing.
    pub fn from_companion_bytes(
        bytes: &[u8],
        expected_content_hash: &ContentHash,
        expected_source_hash: &ContentHash,
    ) -> Result<Self> {
        if bytes.len() > MAX_COMPANION_BYTES {
            return Err(CompanionError::limit());
        }
        if ContentHash::from_blake3(bytes) != *expected_content_hash {
            return Err(CompanionError::mismatch("companion content hash mismatch"));
        }
        let mut reader = Reader { bytes, position: 0 };
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(CompanionError::invalid("unsupported companion magic"));
        }
        // A byte-bounded, non-recursive manifest shape; serde's JSON recursion
        // guard stays enabled. No untrusted preallocated collection lengths.
        let metadata = reader.blob(MAX_MANIFEST_BYTES)?;
        let manifest: Manifest = serde_json::from_slice(metadata)
            .map_err(|e| CompanionError::invalid(format!("invalid companion manifest: {e}")))?;
        validate_manifest(&manifest, expected_source_hash)?;
        let mut functions = Vec::new();
        for entry in manifest.functions {
            let program =
                reader.blob(cem_ml::validation::xpath::artifact::XPATH_ARTIFACT_MAX_BYTES)?;
            let artifact = XPathCompiledArtifact::from_bytes(
                program.to_vec(),
                &parse_hash(&entry.program_hash)?,
            )
            .map_err(|e| CompanionError::invalid(e.to_string()))?;
            let expression = artifact
                .reload(&XPathArtifactLoadContext {
                    expected_source_hash: expected_source_hash.clone(),
                    invocation_host: XPathInvocationHost::Cemt,
                })
                .map_err(|e| CompanionError::invalid(e.to_string()))?;
            let XPathAttachment::Host(host) = &expression.attachment else {
                return Err(CompanionError::invalid(
                    "companion requires CEMT-owned programs",
                ));
            };
            let id = format!("{}#xpath", entry.name);
            if expression.source.uri != manifest.source_uri
                || host.owner.source_uri != manifest.source_uri
                || host.owner.node_id.as_deref() != Some(id.as_str())
                || host.owner.content_type.as_deref() != Some(CEM_TRANSFORM_CONTENT_TYPE)
                || host.owner.schema_uri.as_deref() != Some(CEM_TRANSFORM_SCHEMA_URI)
            {
                return Err(CompanionError::mismatch(
                    "companion function and XPath host identity disagree",
                ));
            }
            let expected_result = host
                .expected_result
                .clone()
                .ok_or_else(|| CompanionError::invalid("XPath result contract is required"))?;
            let result_contract = ResultContract::parse(&expected_result.sequence_type)
                .ok_or_else(|| CompanionError::invalid("unsupported result contract"))?;
            if result_contract.item_type.starts_with("xs:")
                && host
                    .static_context
                    .namespaces
                    .get("xs")
                    .is_some_and(|uri| uri != "http://www.w3.org/2001/XMLSchema")
            {
                return Err(CompanionError::invalid(
                    "XPath result xs prefix must identify XML Schema types",
                ));
            }
            let variables: Vec<_> = entry
                .variables
                .into_iter()
                .map(|binding| TransformTemplateXPathVariableBinding {
                    host_binding: binding.binding,
                    expanded_name: XPathExpandedName::new(
                        binding.namespace_uri,
                        binding.local_name,
                    ),
                })
                .collect();
            let declared: BTreeMap<_, _> = variables
                .iter()
                .map(|v| {
                    let name = &v.expanded_name;
                    let key = name
                        .namespace_uri
                        .as_ref()
                        .map(|ns| format!("Q{{{ns}}}{}", name.local_name))
                        .unwrap_or_else(|| name.local_name.clone());
                    (key, "item()*".to_owned())
                })
                .collect();
            if declared.len() != variables.len()
                || declared != host.static_context.variable_bindings
            {
                return Err(CompanionError::mismatch(
                    "companion variables and XPath static context disagree",
                ));
            }
            if entry
                .source_map
                .frames
                .iter()
                .any(|frame| frame.source_id.0 != host.owner.source_id)
            {
                return Err(CompanionError::mismatch(
                    "companion source maps and XPath owner disagree",
                ));
            }
            let invocation = TransformTemplateXPathInvocation {
                id,
                owner_function: entry.name.clone(),
                context_binding: entry.context,
                variable_bindings: variables,
                expected_result,
                static_context: host.static_context.clone(),
                expression: Arc::new(expression),
                source_map: entry.source_map,
            };
            functions.push(CemtXPathFunction {
                name: entry.name,
                parameters: entry.parameters,
                returns: entry.returns,
                nullable: entry.nullable,
                result_contract,
                invocation,
                artifact,
            });
        }
        if reader.position != bytes.len() {
            return Err(CompanionError::invalid("trailing companion bytes"));
        }
        Ok(Self {
            functions,
            source_hash: expected_source_hash.clone(),
            source_uri: manifest.source_uri,
        })
    }
}

fn validate_manifest(manifest: &Manifest, expected_source_hash: &ContentHash) -> Result<()> {
    if manifest.content_type != COMPANION_CONTENT_TYPE
        || manifest.version != COMPANION_VERSION
        || manifest.cem_ml_version != cem_ml::VERSION
        || manifest.cem_ql_version != crate::VERSION
        || parse_hash(&manifest.source_hash)? != *expected_source_hash
    {
        return Err(CompanionError::mismatch(
            "companion type, version or source identity mismatch",
        ));
    }
    if manifest.source_uri.len() > MAX_SOURCE_BYTES || manifest.functions.len() > MAX_FUNCTIONS {
        return Err(CompanionError::limit());
    }
    let mut names = BTreeSet::new();
    for function in &manifest.functions {
        if function.name.trim().is_empty()
            || function.name.len() > MAX_SOURCE_BYTES
            || !names.insert(&function.name)
            || !supported_type(function.returns)
        {
            return Err(CompanionError::invalid(
                "invalid or duplicate companion function",
            ));
        }
        if function.parameters.len() >= 255 || function.variables.len() > 254 {
            return Err(CompanionError::limit());
        }
        let mut params = BTreeSet::new();
        for parameter in &function.parameters {
            if parameter.name.trim().is_empty()
                || parameter.name.len() > MAX_SOURCE_BYTES
                || !supported_type(parameter.kind)
                || !params.insert(&parameter.name)
            {
                return Err(CompanionError::invalid(
                    "invalid or duplicate companion parameter",
                ));
            }
        }
        for binding in function
            .variables
            .iter()
            .map(|v| &v.binding)
            .chain(function.context.iter())
        {
            if !params.contains(binding) {
                return Err(CompanionError::invalid("undeclared companion host binding"));
            }
        }
        for variable in &function.variables {
            if variable.local_name.is_empty()
                || variable.local_name.len() > MAX_SOURCE_BYTES
                || variable
                    .namespace_uri
                    .as_ref()
                    .is_some_and(|ns| ns.is_empty() || ns.len() > MAX_SOURCE_BYTES)
            {
                return Err(CompanionError::invalid("invalid companion variable name"));
            }
        }
        parse_hash(&function.program_hash)?;
        validate_source_map(&function.source_map)?;
    }
    Ok(())
}

fn validate_source_map(map: &SourceMapStack) -> Result<()> {
    if map.frames.is_empty() || map.frames.len() > 64 {
        return Err(CompanionError::invalid(
            "invalid companion source-map frame count",
        ));
    }
    for frame in &map.frames {
        let ranges = match &frame.span {
            FrameSpan::Single(range) => std::slice::from_ref(range),
            FrameSpan::Multi(ranges) => ranges.as_slice(),
        };
        if frame.source_id.0 == 0
            || ranges.is_empty()
            || ranges.len() > 64
            || ranges.iter().any(|r| {
                r.start
                    .checked_add(u64::from(r.len))
                    .is_none_or(|end| end > MAX_SOURCE_BYTES as u64)
            })
        {
            return Err(CompanionError::invalid(
                "invalid companion source-map range",
            ));
        }
    }
    Ok(())
}

pub(crate) fn parse_hash(text: &str) -> Result<ContentHash> {
    let (scheme, hex) = text
        .split_once(':')
        .ok_or_else(|| CompanionError::mismatch("invalid content hash"))?;
    if scheme != HASH_SCHEME
        || hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(CompanionError::mismatch("invalid content hash"));
    }
    Ok(ContentHash {
        scheme: scheme.into(),
        hex: hex.into(),
    })
}
fn write_blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value);
}
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| CompanionError::invalid("truncated companion"))?;
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }
    fn blob(&mut self, max: usize) -> Result<&'a [u8]> {
        let len = u32::from_le_bytes(self.take(4)?.try_into().expect("four-byte length")) as usize;
        if len > max {
            return Err(CompanionError::limit());
        }
        self.take(len)
    }
}
