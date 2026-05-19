//! Layer 1 — `ByteSource` and `EncodingDecoder`.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design.md` §5. Tier A implements
//! synchronous chunked sources for in-memory bytes, strings, and files; the
//! `AsyncByteSource` wrapper around these adapters lands in Phase 11 of
//! `cem-ml-cli-plan.md` once the executor choice is finalized.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

pub mod decode;
pub mod line_index;

/// Recommended Tier A adapter chunk size (`MAX_SOURCE_CHUNK_BYTES` in the
/// design doc).
pub const MAX_SOURCE_CHUNK_BYTES: usize = 64 * 1024;

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

impl ByteRange {
    pub fn new(start: u64, len: u32) -> Self {
        Self { start, len }
    }
    pub fn end(self) -> u64 {
        self.start + self.len as u64
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BomInfo {
    pub encoding: Encoding,
    pub byte_range: ByteRange,
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

/// One raw transport chunk emitted by a `ByteSource`. `byte_range.start` is
/// the absolute offset of `bytes[0]` inside the source stream.
#[derive(Debug, Clone)]
pub struct ByteChunk {
    pub source_id: SourceId,
    pub bytes: Vec<u8>,
    pub byte_range: ByteRange,
}

/// Synchronous chunk-pull byte source boundary.
///
/// The Tier A streaming wrapper (`async fn next_chunk_async(...)`) lands in
/// Phase 11; everything in this module can be wrapped in `async fn` adapters
/// without changing the trait shape.
pub trait ByteSource: Send {
    fn source_id(&self) -> SourceId;
    fn next_chunk(&mut self) -> io::Result<Option<ByteChunk>>;
}

/// In-memory byte buffer adapter. Yields chunks bounded by `chunk_size`
/// (default `MAX_SOURCE_CHUNK_BYTES`).
pub struct BytesSource {
    source_id: SourceId,
    bytes: Vec<u8>,
    cursor: usize,
    chunk_size: usize,
}

impl BytesSource {
    pub fn new(source_id: SourceId, bytes: Vec<u8>) -> Self {
        Self {
            source_id,
            bytes,
            cursor: 0,
            chunk_size: MAX_SOURCE_CHUNK_BYTES,
        }
    }

    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size.max(1);
        self
    }
}

impl ByteSource for BytesSource {
    fn source_id(&self) -> SourceId {
        self.source_id
    }

    fn next_chunk(&mut self) -> io::Result<Option<ByteChunk>> {
        if self.cursor >= self.bytes.len() {
            return Ok(None);
        }
        let start = self.cursor;
        let end = (start + self.chunk_size).min(self.bytes.len());
        let slice = self.bytes[start..end].to_vec();
        self.cursor = end;
        Ok(Some(ByteChunk {
            source_id: self.source_id,
            bytes: slice,
            byte_range: ByteRange::new(start as u64, (end - start) as u32),
        }))
    }
}

/// In-memory string adapter. The string is treated as UTF-8 bytes for the
/// underlying source stream; encoding selection still runs in the decoder so
/// BOM-prefixed strings behave identically to byte input.
pub struct StringSource {
    inner: BytesSource,
}

impl StringSource {
    pub fn new(source_id: SourceId, content: String) -> Self {
        Self {
            inner: BytesSource::new(source_id, content.into_bytes()),
        }
    }

    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.inner = self.inner.with_chunk_size(chunk_size);
        self
    }
}

impl ByteSource for StringSource {
    fn source_id(&self) -> SourceId {
        self.inner.source_id()
    }
    fn next_chunk(&mut self) -> io::Result<Option<ByteChunk>> {
        self.inner.next_chunk()
    }
}

/// File adapter using buffered std I/O. Tier A is synchronous; async file
/// I/O (tokio/async-std) is a Phase 11 wrapper above this trait.
pub struct FileSource {
    source_id: SourceId,
    reader: BufReader<File>,
    cursor: u64,
    chunk_size: usize,
}

impl FileSource {
    pub fn open(source_id: SourceId, path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            source_id,
            reader: BufReader::with_capacity(MAX_SOURCE_CHUNK_BYTES, file),
            cursor: 0,
            chunk_size: MAX_SOURCE_CHUNK_BYTES,
        })
    }

    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size.max(1);
        self
    }
}

impl ByteSource for FileSource {
    fn source_id(&self) -> SourceId {
        self.source_id
    }

    fn next_chunk(&mut self) -> io::Result<Option<ByteChunk>> {
        let mut buf = vec![0u8; self.chunk_size];
        let mut filled = 0;
        while filled < buf.len() {
            match self.reader.read(&mut buf[filled..])? {
                0 => break,
                n => filled += n,
            }
        }
        if filled == 0 {
            return Ok(None);
        }
        buf.truncate(filled);
        let start = self.cursor;
        self.cursor += filled as u64;
        Ok(Some(ByteChunk {
            source_id: self.source_id,
            bytes: buf,
            byte_range: ByteRange::new(start, filled as u32),
        }))
    }
}

/// Encoding decoder boundary. Tier A turns chunks from a `ByteSource` into
/// `DecodedChunk` records preserving absolute byte offsets.
pub trait EncodingDecoder: Send {
    fn decode_next(&mut self) -> Option<DecodedChunk>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain<S: ByteSource>(mut s: S) -> Vec<ByteChunk> {
        let mut chunks = Vec::new();
        while let Some(c) = s.next_chunk().unwrap() {
            chunks.push(c);
        }
        chunks
    }

    #[test]
    fn bytes_source_yields_one_chunk_when_under_chunk_size() {
        let chunks = drain(BytesSource::new(SourceId(1), b"hello".to_vec()));
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].bytes, b"hello");
        assert_eq!(chunks[0].byte_range, ByteRange::new(0, 5));
        assert_eq!(chunks[0].source_id, SourceId(1));
    }

    #[test]
    fn bytes_source_splits_into_configured_chunk_size() {
        let chunks = drain(BytesSource::new(SourceId(2), b"abcdefgh".to_vec()).with_chunk_size(3));
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].bytes, b"abc");
        assert_eq!(chunks[0].byte_range, ByteRange::new(0, 3));
        assert_eq!(chunks[1].byte_range, ByteRange::new(3, 3));
        assert_eq!(chunks[2].bytes, b"gh");
        assert_eq!(chunks[2].byte_range, ByteRange::new(6, 2));
    }

    #[test]
    fn string_source_preserves_utf8_bytes() {
        let chunks = drain(StringSource::new(SourceId(3), "héllo".to_owned()));
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].bytes, "héllo".as_bytes());
    }

    #[test]
    fn empty_bytes_source_yields_no_chunks() {
        let chunks = drain(BytesSource::new(SourceId(4), Vec::new()));
        assert!(chunks.is_empty());
    }

    #[test]
    fn file_source_reads_back_full_content() {
        let dir = std::env::temp_dir().join("cem-ml-source-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hello.cem");
        std::fs::write(&path, b"file source test\n").unwrap();
        let src = FileSource::open(SourceId(5), &path)
            .unwrap()
            .with_chunk_size(4);
        let chunks = drain(src);
        let merged: Vec<u8> = chunks
            .iter()
            .flat_map(|c| c.bytes.iter().copied())
            .collect();
        assert_eq!(merged, b"file source test\n");
        let total: u64 = chunks.iter().map(|c| c.byte_range.len as u64).sum();
        assert_eq!(total, merged.len() as u64);
        // Ranges are contiguous.
        for w in chunks.windows(2) {
            assert_eq!(w[0].byte_range.end(), w[1].byte_range.start);
        }
    }
}
