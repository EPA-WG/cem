//! Layer 1 — `ByteSource` and `EncodingDecoder`.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design.md` §5. Tier A exposes the
//! boundary types so downstream layers can be written against them; the streaming
//! body lands in Phase 11 of `cem-ml-cli-plan.md`.

use serde::{Deserialize, Serialize};

/// Stable opaque identity for a byte stream. Source-map frames reference it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId(pub u32);

/// Absolute byte range inside a `SourceId`. Ground-truth coordinate per
/// `cem-ml-stack-design-impl.md` §2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: u64,
    pub len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Latin1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EncodingSelection {
    Bom,
    DefaultParameter,
    Utf8Fallback,
}

/// One unit of decoded scalars plus the originating byte range. Layers above
/// Layer 1 never see raw transport bytes — they consume `DecodedChunk`.
#[derive(Debug, Clone)]
pub struct DecodedChunk {
    pub source_id: SourceId,
    pub byte_range: ByteRange,
    pub encoding: Encoding,
    pub scalars: Vec<(char, ByteRange)>,
}

/// Async byte source boundary. The trait body intentionally has no method that
/// is callable today; declaring the type fixes the public boundary used by
/// `tokenizer` and downstream layers.
///
/// The Tier A streaming body (`async fn next_chunk(&mut self) -> Option<...>`)
/// lands with the parser in Phase 11.
pub trait ByteSource: Send {
    fn source_id(&self) -> SourceId;
}

/// Encoding decoder boundary. Tier A turns chunks from a `ByteSource` into
/// `DecodedChunk` records preserving absolute byte offsets.
pub trait EncodingDecoder: Send {
    fn decode_next(&mut self) -> Option<DecodedChunk>;
}
