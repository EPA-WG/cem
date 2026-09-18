//! Explicit XSLT deployment bundles. Member codecs remain independently owned;
//! runtime documents only enter as retained native CEM trees.
//! See `docs/xslt-bundle.md` for the versioned binding and retention contract.
use crate::{
    native::NativeFunctionRegistry,
    render::{render_compiled_template, RenderPlan, TemplateArtifact, TemplateData},
    template_artifact::{
        CompiledTemplateArtifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode,
    },
    xpath::functions::supported_type,
};
use cem_ml::{
    content_cache::{ContentHash, HASH_SCHEME},
    schema::registry::XSLT_SCHEMA_URI,
    transform_template::TransformTemplateModuleParamType as ParamType,
    validation::xpath::{
        artifact::{XPathArtifactLoadContext, XPathCompiledArtifact, XPATH_ARTIFACT_MAX_BYTES},
        XPathAttachment, XPathEvaluationLimits, XPathExpandedName, XPathExpressionAst,
        XPathInvocationHost,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub mod compiler;
mod invocation;
mod source_maps;
pub use cem_ml::schema::registry::XSLT_BUNDLE_CONTENT_TYPE as BUNDLE_CONTENT_TYPE;
pub const BUNDLE_VERSION: &str = "cem-xslt-bundle/1";
pub const MAX_BUNDLE_BYTES: usize = 8 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 128 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 4096;
const MAGIC: &[u8; 9] = b"CEMXSLT1\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for BundleError {}
impl BundleError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "cem.xslt.bundle_invalid",
            message: message.into(),
        }
    }
    fn mismatch(message: impl Into<String>) -> Self {
        Self {
            code: "cem.xslt.bundle_identity_mismatch",
            message: message.into(),
        }
    }
    fn limit() -> Self {
        Self {
            code: "cem.xslt.bundle_limit",
            message: "XSLT bundle exceeds byte, depth, item or handle limits".into(),
        }
    }
}
type Result<T> = std::result::Result<T, BundleError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleSource {
    pub uri: String,
    pub hash: String,
    pub byte_length: u64,
}
impl BundleSource {
    pub fn new(uri: impl Into<String>, bytes: &[u8]) -> Self {
        Self {
            uri: uri.into(),
            hash: ContentHash::from_blake3(bytes).header_value(),
            byte_length: bytes.len() as u64,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyKind {
    Import,
    Include,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StylesheetDependency {
    pub kind: DependencyKind,
    pub target: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StylesheetSource {
    pub source: BundleSource,
    pub dependencies: Vec<StylesheetDependency>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BundleFocus {
    Absent,
    Singleton,
    Sequence,
}
impl BundleFocus {
    fn arity(self) -> usize {
        match self {
            Self::Absent => 0,
            Self::Singleton => 1,
            Self::Sequence => 3,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleVariable {
    pub name: XPathExpandedName,
    pub kind: ParamType,
    pub nullable: bool,
}
#[derive(Debug, Clone)]
pub struct BundleProgram {
    pub stylesheet: usize,
    pub focus: BundleFocus,
    pub variables: Vec<BundleVariable>,
    pub artifact: XPathCompiledArtifact,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramBinding {
    stylesheet: usize,
    focus: BundleFocus,
    variables: Vec<BundleVariable>,
    hash: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    content_type: String,
    schema_uri: String,
    version: String,
    cem_ml_version: String,
    cem_ql_version: String,
    stylesheets: Vec<StylesheetSource>,
    generated_source: BundleSource,
    template_hash: String,
    source_map_mode: TemplateArtifactSourceMapMode,
    host_bindings: Vec<String>,
    programs: Vec<ProgramBinding>,
}

#[derive(Debug)]
pub struct XsltBundle {
    manifest: Manifest,
    template: TemplateArtifact,
    expressions: Vec<Arc<XPathExpressionAst>>,
    functions: NativeFunctionRegistry,
    data_readers: crate::eval::DataReaderCache,
}

impl XsltBundle {
    /// Package a compiler-linked generated template and its typed XPath members.
    /// This does not perform stylesheet lowering or resolve imports.
    pub fn compose(
        template: &CompiledTemplateArtifact,
        generated_source: BundleSource,
        stylesheets: &[StylesheetSource],
        programs: &[BundleProgram],
    ) -> Result<Vec<u8>> {
        if stylesheets.len() > 64 || programs.len() > 128 {
            return Err(BundleError::limit());
        }
        let manifest = Manifest {
            content_type: BUNDLE_CONTENT_TYPE.into(),
            schema_uri: XSLT_SCHEMA_URI.into(),
            version: BUNDLE_VERSION.into(),
            cem_ml_version: cem_ml::VERSION.into(),
            cem_ql_version: crate::VERSION.into(),
            stylesheets: stylesheets.to_vec(),
            generated_source,
            template_hash: template.content_hash.header_value(),
            source_map_mode: template.identity.source_map_mode,
            host_bindings: template.identity.host_bindings.clone(),
            programs: programs
                .iter()
                .map(|p| ProgramBinding {
                    stylesheet: p.stylesheet,
                    focus: p.focus,
                    variables: p.variables.clone(),
                    hash: p.artifact.content_hash().header_value(),
                })
                .collect(),
        };
        let root_hash = manifest
            .stylesheets
            .first()
            .ok_or_else(|| BundleError::invalid("root stylesheet is required"))?
            .source
            .hash
            .clone();
        validate_manifest(&manifest, &parse_hash(&root_hash)?)?;
        let metadata =
            serde_json::to_vec(&manifest).map_err(|e| BundleError::invalid(e.to_string()))?;
        if metadata.len() > MAX_MANIFEST_BYTES {
            return Err(BundleError::limit());
        }
        let mut bytes = MAGIC.to_vec();
        write_blob(&mut bytes, &metadata)?;
        write_blob(&mut bytes, &template.bytes)?;
        for program in programs {
            write_blob(&mut bytes, program.artifact.bytes())?;
        }
        // Compiler composition and hostile reload use the same validation path.
        Self::from_bytes(
            &bytes,
            &ContentHash::from_blake3(&bytes),
            &parse_hash(&root_hash)?,
        )?;
        Ok(bytes)
    }

    pub fn from_bytes(
        bytes: &[u8],
        expected_hash: &ContentHash,
        expected_root_source_hash: &ContentHash,
    ) -> Result<Self> {
        if bytes.len() > MAX_BUNDLE_BYTES {
            return Err(BundleError::limit());
        }
        if ContentHash::from_blake3(bytes) != *expected_hash {
            return Err(BundleError::mismatch("bundle content hash mismatch"));
        }
        let mut reader = Reader { bytes, position: 0 };
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(BundleError::invalid("unsupported bundle magic"));
        }
        let manifest: Manifest = serde_json::from_slice(reader.blob(MAX_MANIFEST_BYTES)?)
            .map_err(|e| BundleError::invalid(format!("invalid bundle manifest: {e}")))?;
        validate_manifest(&manifest, expected_root_source_hash)?;
        let template_bytes = reader.blob(MAX_BUNDLE_BYTES)?;
        if ContentHash::from_blake3(template_bytes) != parse_hash(&manifest.template_hash)? {
            return Err(BundleError::mismatch("CEMT member hash mismatch"));
        }
        let compiled = CompiledTemplateArtifact::from_bytes(template_bytes.to_vec())
            .map_err(|e| BundleError::invalid(e.to_string()))?;
        if compiled.identity.skip_cemt_function_bodies {
            return Err(BundleError::invalid(
                "bundle requires linked executable CEMT bodies",
            ));
        }
        let template = compiled
            .reload(&TemplateArtifactLoadContext {
                expected_source_hash: Some(parse_hash(&manifest.generated_source.hash)?),
                host_bindings: manifest.host_bindings.clone(),
                source_map_mode: manifest.source_map_mode,
            })
            .map_err(|e| BundleError::invalid(e.to_string()))?;
        if template
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(BundleError::invalid(
                "generated CEMT contains compile errors",
            ));
        }
        source_maps::validate_template(&template, &manifest.generated_source)?;
        let mut expressions = Vec::new();
        for binding in &manifest.programs {
            let program = XPathCompiledArtifact::from_bytes(
                reader.blob(XPATH_ARTIFACT_MAX_BYTES)?.to_vec(),
                &parse_hash(&binding.hash)?,
            )
            .map_err(|e| BundleError::invalid(e.to_string()))?;
            let owner = &manifest.stylesheets[binding.stylesheet].source;
            let expression = program
                .reload(&XPathArtifactLoadContext {
                    expected_source_hash: parse_hash(&owner.hash)?,
                    invocation_host: XPathInvocationHost::Xslt,
                })
                .map_err(|e| BundleError::invalid(e.to_string()))?;
            source_maps::validate_xpath(&expression, owner)?;
            let XPathAttachment::Host(host) = &expression.attachment else {
                unreachable!("validated XSLT owner")
            };
            let declared: BTreeSet<_> = host
                .static_context
                .variable_bindings
                .keys()
                .map(|name| {
                    if let Some(eqname) = name.strip_prefix("Q{") {
                        let (uri, local) = eqname.split_once('}').ok_or_else(|| {
                            BundleError::invalid("invalid static variable EQName")
                        })?;
                        Ok(XPathExpandedName::new(Some(uri), local))
                    } else if let Some((prefix, local)) = name.split_once(':') {
                        let uri = host.static_context.namespaces.get(prefix).ok_or_else(|| {
                            BundleError::invalid("unbound static variable prefix")
                        })?;
                        Ok(XPathExpandedName::new(Some(uri.clone()), local))
                    } else {
                        Ok(XPathExpandedName::unqualified(name))
                    }
                })
                .collect::<Result<_>>()?;
            if declared.len() != host.static_context.variable_bindings.len()
                || declared != binding.variables.iter().map(|v| v.name.clone()).collect()
            {
                return Err(BundleError::mismatch(
                    "bundle variables and XPath static context disagree",
                ));
            }
            expressions.push(Arc::new(expression));
        }
        if reader.position != bytes.len() {
            return Err(BundleError::invalid("trailing bundle bytes"));
        }
        let mut bundle = Self {
            manifest,
            template,
            expressions,
            functions: NativeFunctionRegistry::default(),
            data_readers: Default::default(),
        };
        bundle.functions = bundle.native_functions(XPathEvaluationLimits::default())?;
        Ok(bundle)
    }

    pub fn template(&self) -> &TemplateArtifact {
        &self.template
    }
    pub fn expressions(&self) -> &[Arc<XPathExpressionAst>] {
        &self.expressions
    }
    pub fn host_bindings(&self) -> &[String] {
        &self.manifest.host_bindings
    }
    pub fn stylesheets(&self) -> &[StylesheetSource] {
        &self.manifest.stylesheets
    }
    pub fn generated_source(&self) -> &BundleSource {
        &self.manifest.generated_source
    }
    /// Explicit native hosts may select stricter XPath limits for the same programs.
    pub fn native_functions(
        &self,
        limits: XPathEvaluationLimits,
    ) -> Result<NativeFunctionRegistry> {
        invocation::registry(&self.manifest.programs, &self.expressions, limits)
    }
    pub fn render(&self, input: &TemplateData) -> RenderPlan {
        let mut data = input.clone();
        data.native_functions = self.functions.clone();
        data.data_readers = self.data_readers.clone();
        render_compiled_template(&self.template, &data)
    }
    /// Run the same bundle under the transform host's existing operation scope.
    pub fn render_with_control(
        &self,
        input: &TemplateData,
        control: &cem_ml::operation_control::OperationControl,
        scope: cem_ml::operation_control::ExecutionScopeId,
    ) -> RenderPlan {
        let mut data = input.clone();
        data.native_functions = self.functions.clone();
        data.data_readers = self.data_readers.clone();
        crate::render::render_compiled_template_with_control(&self.template, &data, control, scope)
    }

}

fn identifier(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
        Err(BundleError::invalid("invalid bundle identifier"))
    } else {
        Ok(())
    }
}
fn validate_source(source: &BundleSource) -> Result<()> {
    identifier(&source.uri)?;
    parse_hash(&source.hash)?;
    if source.byte_length == 0 || source.byte_length > MAX_BUNDLE_BYTES as u64 {
        return Err(BundleError::limit());
    }
    Ok(())
}
fn validate_manifest(m: &Manifest, expected_root: &ContentHash) -> Result<()> {
    if m.content_type != BUNDLE_CONTENT_TYPE
        || m.schema_uri != XSLT_SCHEMA_URI
        || m.version != BUNDLE_VERSION
        || m.cem_ml_version != cem_ml::VERSION
        || m.cem_ql_version != crate::VERSION
    {
        return Err(BundleError::mismatch(
            "bundle type, schema or runtime version mismatch",
        ));
    }
    if m.stylesheets.is_empty()
        || m.stylesheets.len() > 64
        || m.programs.len() > 128
        || m.host_bindings.len() > 254
    {
        return Err(BundleError::limit());
    }
    if parse_hash(&m.stylesheets[0].source.hash)? != *expected_root {
        return Err(BundleError::mismatch(
            "root stylesheet source hash mismatch",
        ));
    }
    validate_source(&m.generated_source)?;
    parse_hash(&m.template_hash)?;
    let mut uris = BTreeSet::from([&m.generated_source.uri]);
    for stylesheet in &m.stylesheets {
        validate_source(&stylesheet.source)?;
        if !uris.insert(&stylesheet.source.uri) {
            return Err(BundleError::invalid("duplicate source URI"));
        }
        if stylesheet.dependencies.len() > 128 {
            return Err(BundleError::limit());
        }
        if stylesheet
            .dependencies
            .iter()
            .any(|edge| edge.target >= m.stylesheets.len())
        {
            return Err(BundleError::invalid("unresolved stylesheet dependency"));
        }
    }
    // Memoize heights to bound DAG traversal even when dependency paths multiply.
    fn visit(
        index: usize,
        sources: &[StylesheetSource],
        active: &mut BTreeSet<usize>,
        heights: &mut BTreeMap<usize, usize>,
    ) -> Result<usize> {
        if let Some(height) = heights.get(&index) {
            return Ok(*height);
        }
        if active.len() >= 32 {
            return Err(BundleError::limit());
        }
        if !active.insert(index) {
            return Err(BundleError::invalid("cyclic stylesheet dependency"));
        }
        let mut height = 1;
        for edge in &sources[index].dependencies {
            height = height.max(1 + visit(edge.target, sources, active, heights)?);
        }
        if height > 32 {
            return Err(BundleError::limit());
        }
        active.remove(&index);
        heights.insert(index, height);
        Ok(height)
    }
    let mut heights = BTreeMap::new();
    visit(0, &m.stylesheets, &mut BTreeSet::new(), &mut heights)?;
    if heights.len() != m.stylesheets.len() {
        return Err(BundleError::invalid("unreachable stylesheet in closure"));
    }
    let mut names = BTreeSet::new();
    for name in &m.host_bindings {
        identifier(name)?;
        if !names.insert(name) {
            return Err(BundleError::invalid("duplicate host binding"));
        }
    }
    for program in &m.programs {
        if program.stylesheet >= m.stylesheets.len() {
            return Err(BundleError::invalid("unknown XPath stylesheet owner"));
        }
        if program.focus.arity() + program.variables.len() >= 255 {
            return Err(BundleError::limit());
        }
        parse_hash(&program.hash)?;
        let mut names = BTreeSet::new();
        for variable in &program.variables {
            identifier(&variable.name.local_name)?;
            if let Some(uri) = &variable.name.namespace_uri {
                identifier(uri)?;
            }
            if !supported_type(variable.kind) || !names.insert(&variable.name) {
                return Err(BundleError::invalid(
                    "unsupported or duplicate XPath variable",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn parse_hash(text: &str) -> Result<ContentHash> {
    let (scheme, hex) = text
        .split_once(':')
        .ok_or_else(|| BundleError::mismatch("invalid content hash"))?;
    if scheme != HASH_SCHEME
        || hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(BundleError::mismatch("invalid content hash"));
    }
    Ok(ContentHash {
        scheme: scheme.into(),
        hex: hex.into(),
    })
}
fn write_blob(bytes: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    if value.len().saturating_add(4) > MAX_BUNDLE_BYTES.saturating_sub(bytes.len()) {
        return Err(BundleError::limit());
    }
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(())
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
            .ok_or_else(|| BundleError::invalid("truncated bundle"))?;
        let value = &self.bytes[self.position..end];
        self.position = end;
        Ok(value)
    }
    fn blob(&mut self, limit: usize) -> Result<&'a [u8]> {
        let length =
            u32::from_le_bytes(self.take(4)?.try_into().expect("four-byte length")) as usize;
        if length > limit {
            return Err(BundleError::limit());
        }
        self.take(length)
    }
}

#[derive(Debug, Default)]
pub struct XsltBundleHost {
    next_id: u32,
    bytes: usize,
    entries: BTreeMap<u32, (Arc<XsltBundle>, usize)>,
}
impl XsltBundleHost {
    pub fn import(
        &mut self,
        bytes: &[u8],
        hash: &ContentHash,
        root_source_hash: &ContentHash,
    ) -> Result<u32> {
        if self.entries.len() >= 16
            || bytes.len() > (32usize * 1024 * 1024).saturating_sub(self.bytes)
        {
            return Err(BundleError::limit());
        }
        let next = self.next_id.checked_add(1).ok_or_else(BundleError::limit)?;
        let bundle = XsltBundle::from_bytes(bytes, hash, root_source_hash)?;
        self.entries.insert(next, (Arc::new(bundle), bytes.len()));
        self.bytes += bytes.len();
        self.next_id = next;
        Ok(next)
    }
    pub fn get(&self, id: u32) -> Option<Arc<XsltBundle>> {
        self.entries.get(&id).map(|(bundle, _)| bundle.clone())
    }
    pub fn dispose(&mut self, id: u32) -> bool {
        if let Some((_, bytes)) = self.entries.remove(&id) {
            self.bytes -= bytes;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_identity_and_host_limits_do_not_depend_on_source_loading() {
        let registry = cem_ml::schema::registry::SchemaRegistry::with_builtin_schemas();
        assert_eq!(
            registry
                .resolve_content_type(BUNDLE_CONTENT_TYPE)
                .unwrap()
                .schema_uri,
            XSLT_SCHEMA_URI
        );
        let hash = ContentHash::from_blake3(b"invalid");
        for mut host in [
            XsltBundleHost {
                next_id: u32::MAX,
                ..Default::default()
            },
            XsltBundleHost {
                bytes: 32 * 1024 * 1024,
                ..Default::default()
            },
        ] {
            assert_eq!(
                host.import(b"invalid", &hash, &hash).unwrap_err().code,
                "cem.xslt.bundle_limit"
            );
            assert!(host.entries.is_empty());
        }
    }
}
