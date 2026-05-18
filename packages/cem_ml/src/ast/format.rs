//! Wire-format constants and shared types for the debug binary AST.

use crate::source_map::SourceMapFrame;

pub const MAGIC: [u8; 4] = *b"CEMB";
pub const VERSION: u16 = 1;
pub const FLAGS_NONE: u16 = 0;

/// Kind tag for every variant of `CemAstNode`. Stable across the lifetime
/// of `VERSION`. Adding a variant requires bumping `VERSION`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKindTag {
    Document = 0,
    Element = 1,
    Attribute = 2,
    Text = 3,
    Whitespace = 4,
    Comment = 5,
    ProcessingInstruction = 6,
    Cdata = 7,
    RawText = 8,
    Error = 9,
}

impl NodeKindTag {
    pub fn from_u8(b: u8) -> Option<Self> {
        Some(match b {
            0 => NodeKindTag::Document,
            1 => NodeKindTag::Element,
            2 => NodeKindTag::Attribute,
            3 => NodeKindTag::Text,
            4 => NodeKindTag::Whitespace,
            5 => NodeKindTag::Comment,
            6 => NodeKindTag::ProcessingInstruction,
            7 => NodeKindTag::Cdata,
            8 => NodeKindTag::RawText,
            9 => NodeKindTag::Error,
            _ => return None,
        })
    }
}

/// Subtree chunk metadata. Tier A emits one chunk per document (the
/// whole-document chunk); a Phase 11 streaming encoder splits the document
/// into multiple chunks that share dictionaries.
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub root_id: u32,
    pub parent_anchor: Option<u32>,
    /// Indices into the dictionaries section. Tier A always emits a
    /// single-chunk payload so `dictionary_ids` is `[0]`.
    pub dictionary_ids: Vec<u32>,
    /// Local node range covered by this chunk: `[start_node_id .. start_node_id + node_count]`.
    pub local_node_start: u32,
    pub local_node_count: u32,
    /// Source-map frame deltas for nodes in this chunk that introduce
    /// new transforms (used for incremental streaming; Tier A leaves it empty).
    pub source_map_deltas: Vec<SourceMapFrame>,
    /// Element ids in this chunk that own children outside the chunk
    /// (Tier A is whole-document so this is always empty).
    pub child_links: Vec<ChildLink>,
    /// External references the chunk depends on (e.g. shared schema ids).
    /// Tier A leaves this empty.
    pub external_references: Vec<u32>,
    /// FNV-1a hash over the payload bytes covered by this chunk.
    pub integrity_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildLink {
    pub parent: u32,
    pub external_chunk: u32,
    pub external_node: u32,
}

#[derive(Debug, Default, Clone)]
pub struct BinaryAstPayload {
    pub bytes: Vec<u8>,
    pub chunks: Vec<ChunkMetadata>,
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}
