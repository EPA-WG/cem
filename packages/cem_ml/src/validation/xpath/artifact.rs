//! XPath-owned executable artifact boundary. This encodes typed programs only:
//! no source parser, runtime data AST, result sequence, or callback participates
//! in reload. The source syntax model remains independent of serialization.
use super::*;
use crate::content_cache::{ContentHash, HASH_SCHEME};
pub use crate::schema::registry::XPATH_ARTIFACT_CONTENT_TYPE;

mod codec;

pub const XPATH_ARTIFACT_VERSION: &str = "cem-xpath-artifact/1";
pub const XPATH_PROGRAM_FORMAT: &str = "cem-xpath-program-v2";
/// Bounds apply before allocation/recursive decoding as well as compilation.
pub const XPATH_ARTIFACT_MAX_BYTES: usize = 2 * 1024 * 1024;
const MAGIC: &[u8] = b"CEMXPATH1\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathArtifactIdentity {
    pub content_type: String,
    pub schema_uri: String,
    pub artifact_version: String,
    pub program_format: String,
    pub grammar_version: String,
    pub compiler_version: String,
    /// Hash of the owning source bytes; for an embedded slot this is the host
    /// stylesheet, not a reconstructed/decoded substring of its attribute.
    pub source_hash: ContentHash,
}

#[derive(Debug, Clone)]
struct Program {
    source: XPathExpressionSource,
    attachment: XPathAttachment,
    root: XPathExpressionSequence,
}

#[derive(Debug, Clone)]
pub struct XPathCompiledArtifact {
    identity: XPathArtifactIdentity,
    content_hash: ContentHash,
    bytes: Vec<u8>,
    program: Program,
}

#[derive(Debug, Clone)]
pub struct XPathArtifactLoadContext {
    pub expected_source_hash: ContentHash,
    pub invocation_host: XPathInvocationHost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathArtifactError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for XPathArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for XPathArtifactError {}

impl XPathArtifactError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "cem.xpath.artifact_invalid",
            message: message.into(),
        }
    }
    fn limit() -> Self {
        Self {
            code: "cem.xpath.artifact_limit",
            message: "XPath compiled artifact exceeds byte, depth, or item limits".into(),
        }
    }
    fn mismatch(message: impl Into<String>) -> Self {
        Self {
            code: "cem.xpath.artifact_identity_mismatch",
            message: message.into(),
        }
    }
}

impl XPathCompiledArtifact {
    pub fn compile(
        expression: &XPathExpressionAst,
        source_hash: ContentHash,
    ) -> Result<Self, XPathArtifactError> {
        let diagnostics =
            validate_xpath_expression_ast(expression, XPathSchemaContractCatalog::from_builtin());
        if diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
            return Err(XPathArtifactError::invalid(
                "cannot compile an invalid XPath expression",
            ));
        }
        let syntax = expression.syntax_ast.as_ref().ok_or_else(|| {
            XPathArtifactError::invalid("XPath compiled program requires typed syntax")
        })?;
        let identity = XPathArtifactIdentity {
            content_type: XPATH_ARTIFACT_CONTENT_TYPE.into(),
            schema_uri: XPATH_SCHEMA_URI.into(),
            artifact_version: XPATH_ARTIFACT_VERSION.into(),
            program_format: XPATH_PROGRAM_FORMAT.into(),
            grammar_version: XPATH_GRAMMAR_VERSION.into(),
            compiler_version: crate::VERSION.into(),
            source_hash,
        };
        // Bound the original typed tree while encoding, before any recursive
        // cloning. Decode through the same validating boundary used by hosts.
        let mut writer = codec::Writer::new(MAGIC);
        writer.write(&identity)?;
        writer.write(&expression.source)?;
        writer.write(&expression.attachment)?;
        writer.write(&syntax.root)?;
        let bytes = writer.finish();
        let hash = ContentHash::from_blake3(&bytes);
        Self::from_bytes(bytes, &hash)
    }

    /// The expected content hash must come from the caller's trusted manifest
    /// or compiler output. A digest checks integrity, not publisher authenticity.
    pub fn from_bytes(
        bytes: Vec<u8>,
        expected_hash: &ContentHash,
    ) -> Result<Self, XPathArtifactError> {
        if bytes.len() > XPATH_ARTIFACT_MAX_BYTES {
            return Err(XPathArtifactError::limit());
        }
        let content_hash = ContentHash::from_blake3(&bytes);
        if &content_hash != expected_hash {
            return Err(XPathArtifactError::mismatch(
                "XPath artifact content hash mismatch",
            ));
        }
        let mut reader = codec::Reader::new(&bytes);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(XPathArtifactError::invalid(
                "not a supported XPath compiled artifact",
            ));
        }
        let identity: XPathArtifactIdentity = reader.read()?;
        validate_identity(&identity)?;
        let program = Program {
            source: reader.read()?,
            attachment: reader.read()?,
            root: reader.read()?,
        };
        reader.finish()?;
        // Do not let artifacts change evaluator conventions (e.g. invalid
        // decimal format characters) merely by avoiding source validation.
        if let Some(context) = program.attachment.static_context() {
            xpath_validate_decimal_formats(context).map_err(XPathArtifactError::invalid)?;
        }
        if program.source.media_type != XPATH_CONTENT_TYPE
            && program.source.media_type != "text/xpath"
        {
            return Err(XPathArtifactError::invalid(
                "compiled program is not XPath source",
            ));
        }
        Ok(Self {
            identity,
            content_hash,
            bytes,
            program,
        })
    }

    pub fn identity(&self) -> &XPathArtifactIdentity {
        &self.identity
    }
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn reload(
        &self,
        context: &XPathArtifactLoadContext,
    ) -> Result<XPathExpressionAst, XPathArtifactError> {
        if self.identity.source_hash != context.expected_source_hash {
            return Err(XPathArtifactError::mismatch(
                "XPath owning-source hash mismatch",
            ));
        }
        let host_matches = match (&self.program.attachment, context.invocation_host) {
            (
                XPathAttachment::Standalone { .. }
                | XPathAttachment::StandaloneStaticContext { .. },
                XPathInvocationHost::Query | XPathInvocationHost::StandaloneTransform,
            ) => true,
            (XPathAttachment::Host(host), invocation) => matches!(
                (host.owner.node_kind, invocation),
                (XPathHostNodeKind::XsltAttribute, XPathInvocationHost::Xslt)
                    | (
                        XPathHostNodeKind::CemQlExpressionSlot,
                        XPathInvocationHost::CemQl
                    )
                    | (
                        XPathHostNodeKind::CemtExpressionSlot,
                        XPathInvocationHost::Cemt
                    )
            ),
            _ => false,
        };
        if !host_matches {
            return Err(XPathArtifactError::mismatch(
                "XPath artifact invocation host mismatch",
            ));
        }
        Ok(XPathExpressionAst {
            source: self.program.source.clone(),
            source_text: None,
            tokens: Vec::new(),
            events: Vec::new(),
            syntax_ast: Some(XPathSyntaxAst::new(self.program.root.clone())),
            facts: Vec::new(),
            attachment: self.program.attachment.clone(),
            line_ending: None,
        })
    }
}

fn validate_identity(identity: &XPathArtifactIdentity) -> Result<(), XPathArtifactError> {
    if identity.content_type != XPATH_ARTIFACT_CONTENT_TYPE
        || identity.schema_uri != XPATH_SCHEMA_URI
        || identity.artifact_version != XPATH_ARTIFACT_VERSION
        || identity.program_format != XPATH_PROGRAM_FORMAT
        || identity.grammar_version != XPATH_GRAMMAR_VERSION
        || identity.compiler_version != crate::VERSION
    {
        return Err(XPathArtifactError::mismatch(
            "XPath artifact type, schema, or version mismatch",
        ));
    }
    if identity.source_hash.scheme != HASH_SCHEME
        || identity.source_hash.hex.len() != 64
        || !identity
            .source_hash
            .hex
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(XPathArtifactError::invalid("invalid XPath source hash"));
    }
    Ok(())
}
