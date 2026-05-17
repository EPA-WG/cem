//! Layer 7 — `BinaryAstEncoder` interface stub.
//!
//! Per AC-F-11, the encoder interface is Tier A, the body is Tier B. The
//! types live here so consumers can program against the boundary today.

use crate::parser::CemAstNode;
use crate::source_map::SourceMapFrame;

#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub root_id: u32,
    pub parent_anchor: Option<u32>,
    pub dictionary_ids: Vec<u32>,
    pub source_map_deltas: Vec<SourceMapFrame>,
    pub integrity_hash: [u8; 32],
}

#[derive(Debug, Default)]
pub struct BinaryAstPayload {
    pub bytes: Vec<u8>,
    pub chunks: Vec<ChunkMetadata>,
}

/// Boundary for the deferred Tier B binary AST encoder. Tier A
/// implementations may return an empty payload or a not-implemented
/// error variant; downstream code should program against this trait
/// rather than a concrete encoder.
#[doc(hidden)]
pub trait BinaryAstEncoder: Send {
    fn encode(&self, nodes: &[CemAstNode]) -> BinaryAstPayload;
}
