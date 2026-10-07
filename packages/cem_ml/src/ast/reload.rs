//! Explicit debug-AST/source/lexical export boundary. Runtime contexts and authority stay with the caller.
use super::{DebugBinaryDecoder, DebugBinaryEncoder, VERSION};
pub use crate::schema::machine::LexicalReloadMetadata;
use crate::{
    parser::{document::CemDocument, CemAstNode},
    schema::machine::LexicallyScopedDocument,
    source::SourceId,
    source_map::{FrameSpan, SourceMapStack},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

mod ingress;
pub use ingress::ReloadIngress;

pub const RELOAD_VERSION: u16 = 1;
pub const DEBUG_CEMB_CODEC: &str = "cem.debug.cemb";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReloadLimits {
    pub max_bytes: usize,
    /// Arena nodes and entries in each debug binary dictionary/edge collection.
    pub max_nodes: usize,
}
impl Default for ReloadLimits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_nodes: 100000,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadError {
    UnsupportedVersion,
    FingerprintMismatch,
    InvalidMetadata,
    InvalidSource,
    InvalidPayload(String),
    InvalidEnvelope(String),
    LimitExceeded,
}
impl std::fmt::Display for ReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CEM reference reload: {self:?}")
    }
}
impl std::error::Error for ReloadError {}
impl ReloadError {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "UnsupportedVersion",
            Self::FingerprintMismatch => "FingerprintMismatch",
            Self::InvalidMetadata => "InvalidMetadata",
            Self::InvalidSource => "InvalidSource",
            Self::InvalidPayload(_) => "InvalidPayload",
            Self::InvalidEnvelope(_) => "InvalidEnvelope",
            Self::LimitExceeded => "LimitExceeded",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadDependency {
    MissingLexicalMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReloadSource {
    pub source_id: SourceId,
    pub uri: String,
    pub fingerprint: [u8; 32],
    pub byte_length: usize,
    pub bytes: Option<Vec<u8>>,
}
impl ReloadSource {
    pub fn new(
        source_id: SourceId,
        uri: impl Into<String>,
        bytes: &[u8],
        include_bytes: bool,
    ) -> Self {
        Self {
            source_id,
            uri: uri.into(),
            fingerprint: fingerprint(bytes),
            byte_length: bytes.len(),
            bytes: include_bytes.then(|| bytes.to_vec()),
        }
    }
    /// Explicit verified host handoff; this performs no resource lookup.
    pub fn supply_bytes(&mut self, bytes: Vec<u8>) -> Result<(), ReloadError> {
        if bytes.len() != self.byte_length || fingerprint(&bytes) != self.fingerprint {
            return Err(ReloadError::FingerprintMismatch);
        }
        self.bytes = Some(bytes);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceReloadBundle {
    pub version: u16,
    pub codec: String,
    pub codec_version: u16,
    pub payload: Vec<u8>,
    pub payload_fingerprint: [u8; 32],
    pub sources: Vec<ReloadSource>,
    pub lexical: Option<LexicalReloadMetadata>,
}

#[derive(Debug)]
pub struct ReloadedReferenceDocument {
    pub document: Arc<CemDocument>,
    pub lexical: Option<Arc<LexicallyScopedDocument>>,
    pub sources: Vec<ReloadSource>,
}
impl ReloadedReferenceDocument {
    pub fn require_lexical(&self) -> Result<&Arc<LexicallyScopedDocument>, ReloadDependency> {
        self.lexical
            .as_ref()
            .ok_or(ReloadDependency::MissingLexicalMetadata)
    }
    pub fn source_text(&self, id: SourceId) -> Result<Option<&str>, ReloadError> {
        let source = self
            .sources
            .iter()
            .find(|s| s.source_id == id)
            .ok_or(ReloadError::InvalidSource)?;
        source
            .bytes
            .as_ref()
            .map(|bytes| std::str::from_utf8(bytes).map_err(|_| ReloadError::InvalidSource))
            .transpose()
    }
}

fn fingerprint(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

impl ReferenceReloadBundle {
    pub fn export(
        capture: &LexicallyScopedDocument,
        sources: Vec<ReloadSource>,
        limits: ReloadLimits,
    ) -> Result<Self, ReloadError> {
        if capture.document().nodes.len() > limits.max_nodes {
            return Err(ReloadError::LimitExceeded);
        }
        let payload = DebugBinaryEncoder::new().encode(capture.document()).bytes;
        let payload_fingerprint = fingerprint(&payload);
        let bundle = Self {
            version: RELOAD_VERSION,
            codec: DEBUG_CEMB_CODEC.into(),
            codec_version: VERSION,
            payload,
            payload_fingerprint,
            sources,
            lexical: Some(LexicalReloadMetadata::export(capture, payload_fingerprint)),
        };
        bundle.check_envelope(limits)?;
        bundle.encoded(limits)?;
        bundle.validate_document(capture.document(), limits)?;
        Ok(bundle)
    }

    fn encoded(&self, limits: ReloadLimits) -> Result<Vec<u8>, ReloadError> {
        struct LimitedBytes {
            bytes: Vec<u8>,
            max: usize,
            exceeded: bool,
        }
        impl std::io::Write for LimitedBytes {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if bytes.len() > self.max.saturating_sub(self.bytes.len()) {
                    self.exceeded = true;
                    return Err(std::io::Error::other("reload envelope byte limit exceeded"));
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut output = LimitedBytes {
            bytes: Vec::new(),
            max: limits.max_bytes,
            exceeded: false,
        };
        let encoded =
            self.serialize(&mut rmp_serde::Serializer::new(&mut output).with_struct_map());
        if output.exceeded {
            return Err(ReloadError::LimitExceeded);
        }
        encoded.map_err(|e| ReloadError::InvalidEnvelope(e.to_string()))?;
        Ok(output.bytes)
    }
    pub fn encode(&self, limits: ReloadLimits) -> Result<Vec<u8>, ReloadError> {
        // Validate export too: callers can edit the explicit envelope.
        self.reload(limits)?;
        self.encoded(limits)
    }
    pub fn decode(bytes: &[u8], limits: ReloadLimits) -> Result<Self, ReloadError> {
        Self::decode_with_document(bytes, limits).map(|(bundle, _)| bundle)
    }
    /// Retain the owner verified during decoding rather than decoding its arena again.
    pub fn decode_with_document(
        bytes: &[u8],
        limits: ReloadLimits,
    ) -> Result<(Self, ReloadedReferenceDocument), ReloadError> {
        let result = Self::read_envelope(bytes, limits)?;
        let document = result.reload(limits)?;
        Ok((result, document))
    }
    fn read_envelope(bytes: &[u8], limits: ReloadLimits) -> Result<Self, ReloadError> {
        if bytes.len() > limits.max_bytes {
            return Err(ReloadError::LimitExceeded);
        }
        let mut decoder = rmp_serde::Deserializer::new(std::io::Cursor::new(bytes));
        let result = Self::deserialize(&mut decoder)
            .map_err(|e| ReloadError::InvalidEnvelope(e.to_string()))?;
        if decoder.position() as usize != bytes.len() {
            return Err(ReloadError::InvalidEnvelope("trailing bytes".into()));
        }
        result.check_envelope(limits)?;
        Ok(result)
    }
    /// Attach passive inputs to an owner already verified with `previous`.
    /// Payload admission never decodes an arena, reparses source or changes a
    /// completed capture. Omitted inputs preserve previously verified inputs.
    pub fn attach_to(
        bytes: &[u8],
        previous: &Self,
        retained: &ReloadedReferenceDocument,
        limits: ReloadLimits,
    ) -> Result<(Self, ReloadedReferenceDocument), ReloadError> {
        let mut candidate = Self::read_envelope(bytes, limits)?;
        if candidate.payload_fingerprint != previous.payload_fingerprint
            || candidate.payload != previous.payload
        {
            return Err(ReloadError::FingerprintMismatch);
        }
        if candidate.codec != previous.codec || candidate.codec_version != previous.codec_version {
            return Err(ReloadError::UnsupportedVersion);
        }
        if candidate.sources.len() != retained.sources.len() {
            return Err(ReloadError::InvalidSource);
        }
        let existing_sources: BTreeMap<_, _> =
            retained.sources.iter().map(|s| (s.source_id.0, s)).collect();
        for source in &mut candidate.sources {
            let existing = existing_sources
                .get(&source.source_id.0)
                .ok_or(ReloadError::InvalidSource)?;
            if source.uri != existing.uri
                || source.fingerprint != existing.fingerprint
                || source.byte_length != existing.byte_length
            {
                return Err(ReloadError::InvalidSource);
            }
            if source.bytes.is_none() {
                source.bytes = existing.bytes.clone();
            }
        }
        if let (Some(old), Some(new)) = (&previous.lexical, &candidate.lexical) {
            let encode = |metadata: &LexicalReloadMetadata| {
                rmp_serde::to_vec_named(metadata)
                    .map_err(|e| ReloadError::InvalidEnvelope(e.to_string()))
            };
            if encode(old)? != encode(new)? {
                return Err(ReloadError::InvalidMetadata);
            }
        }
        if candidate.lexical.is_none() {
            candidate.lexical = previous.lexical.clone();
        }
        candidate.check_envelope(limits)?;
        candidate.encoded(limits)?;
        candidate.validate_document(&retained.document, limits)?;
        let lexical = match &retained.lexical {
            Some(capture) if Arc::ptr_eq(capture.document(), &retained.document) => {
                Some(capture.clone())
            }
            Some(_) => return Err(ReloadError::InvalidMetadata),
            None => candidate
                .lexical
                .as_ref()
                .map(|metadata| Arc::new(metadata.restore(retained.document.clone()))),
        };
        let document = ReloadedReferenceDocument {
            document: retained.document.clone(),
            lexical,
            sources: candidate.sources.clone(),
        };
        Ok((candidate, document))
    }
    fn check_envelope(&self, limits: ReloadLimits) -> Result<(), ReloadError> {
        if self.version != RELOAD_VERSION
            || self.codec != DEBUG_CEMB_CODEC
            || !(2..=VERSION).contains(&self.codec_version)
        {
            return Err(ReloadError::UnsupportedVersion);
        }
        if self.payload.len() > limits.max_bytes
            || self.sources.len() > limits.max_nodes
            || self
                .sources
                .iter()
                .try_fold(self.payload.len(), |count, source| {
                    count.checked_add(source.bytes.as_ref().map_or(0, Vec::len))
                })
                .is_none_or(|n| n > limits.max_bytes)
        {
            return Err(ReloadError::LimitExceeded);
        }
        if fingerprint(&self.payload) != self.payload_fingerprint {
            return Err(ReloadError::FingerprintMismatch);
        }
        if self
            .payload
            .get(4..6)
            .map(|v| u16::from_le_bytes(v.try_into().unwrap()))
            != Some(self.codec_version)
        {
            return Err(ReloadError::UnsupportedVersion);
        }
        let mut ids = BTreeMap::new();
        for source in &self.sources {
            if source.uri.is_empty() || ids.insert(source.source_id.0, ()).is_some() {
                return Err(ReloadError::InvalidSource);
            }
            if let Some(bytes) = &source.bytes {
                if bytes.len() != source.byte_length || fingerprint(bytes) != source.fingerprint {
                    return Err(ReloadError::FingerprintMismatch);
                }
            }
        }
        Ok(())
    }
    fn validate_document(
        &self,
        document: &CemDocument,
        limits: ReloadLimits,
    ) -> Result<(), ReloadError> {
        if document.nodes.len() > limits.max_nodes {
            return Err(ReloadError::LimitExceeded);
        }
        if let Some(lexical) = &self.lexical {
            lexical.validate(document, self.payload_fingerprint)?;
        }
        let sources: BTreeMap<_, _> = self.sources.iter().map(|s| (s.source_id.0, s)).collect();
        for map in document
            .nodes
            .iter()
            .map(node_source)
            .chain(self.lexical.iter().flat_map(|l| l.source_maps()))
        {
            for frame in &map.frames {
                let source = sources
                    .get(&frame.source_id.0)
                    .ok_or(ReloadError::InvalidSource)?;
                let spans = match &frame.span {
                    FrameSpan::Single(range) => std::slice::from_ref(range),
                    FrameSpan::Multi(ranges) => ranges.as_slice(),
                };
                for span in spans {
                    if span
                        .start
                        .checked_add(span.len as u64)
                        .is_none_or(|end| end > source.byte_length as u64)
                    {
                        return Err(ReloadError::InvalidSource);
                    }
                }
            }
        }
        Ok(())
    }
    pub fn reload(&self, limits: ReloadLimits) -> Result<ReloadedReferenceDocument, ReloadError> {
        self.check_envelope(limits)?;
        self.encoded(limits)?;
        let mut document = DebugBinaryDecoder::new()
            .decode_bounded(&self.payload, limits.max_nodes)
            .map_err(|e| match e {
                super::decode::DecodeError::CountLimitExceeded => ReloadError::LimitExceeded,
                _ => ReloadError::InvalidPayload(e.to_string()),
            })?;
        self.validate_document(&document, limits)?;
        if let Some(lexical) = &self.lexical {
            document.format_identity = lexical.format_identity.clone();
        }
        let document = Arc::new(document);
        crate::parser::tree::RetainedCemTree::from_shared(
            document.clone(),
            "",
            "",
            Default::default(),
            None,
        )
        .map_err(ReloadError::InvalidPayload)?;
        // Only passive metadata is copied; every capture handle refers to this exact allocation.
        let lexical = self
            .lexical
            .as_ref()
            .map(|l| Arc::new(l.restore(document.clone())));
        Ok(ReloadedReferenceDocument {
            document,
            lexical,
            sources: self.sources.clone(),
        })
    }
}

fn node_source(node: &CemAstNode) -> &SourceMapStack {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. }
        | CemAstNode::Reference { source, .. } => source,
    }
}
