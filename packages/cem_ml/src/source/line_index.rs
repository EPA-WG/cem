//! `LineIndex` — streaming byte-offset → reporting-coordinate projection.
//!
//! Line/column are derived reporting coordinates per `cem-ml-stack-design-impl.md`
//! §2.1; byte offsets are the ground truth. The index records the absolute
//! byte offset of every line start as scalars flow past the decoder, so
//! line projection is O(log n) over the accumulated checkpoints. Browser
//! and editor hosts also need UTF-16 positions, so the index records the
//! UTF-16 offset at each line start while keeping byte offsets as the
//! durable source identity.

use crate::source::ByteRange;

/// Newline projection for a single `SourceId`.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Absolute byte offset of each line start. Always begins with 0.
    line_starts: Vec<u64>,
    /// Absolute UTF-16 offset of each line start, parallel to `line_starts`.
    line_start_utf16_offsets: Vec<u64>,
    scalars: Vec<ScalarCheckpoint>,
    total_byte_len: u64,
    total_utf16_len: u64,
}

#[derive(Debug, Clone, Copy)]
struct ScalarCheckpoint {
    byte_start: u64,
    byte_end: u64,
    utf16_offset: u64,
}

impl Default for LineIndex {
    fn default() -> Self {
        Self {
            line_starts: vec![0],
            line_start_utf16_offsets: vec![0],
            scalars: Vec::new(),
            total_byte_len: 0,
            total_utf16_len: 0,
        }
    }
}

impl LineIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_utf8(source: &str) -> Self {
        let mut index = Self::new();
        let mut offset = 0u64;
        for scalar in source.chars() {
            let len = scalar.len_utf8() as u32;
            index.record_scalar(scalar, ByteRange::new(offset, len));
            offset += len as u64;
        }
        index
    }

    pub fn from_bytes_lossy(source: &[u8]) -> Self {
        match std::str::from_utf8(source) {
            Ok(text) => Self::from_utf8(text),
            Err(_) => Self::from_bytes_identity(source),
        }
    }

    /// Record one decoded scalar at the given absolute byte offset. Pass the
    /// byte range emitted by the decoder so multi-byte UTF-8 characters land
    /// at the start byte of their sequence.
    pub fn record_scalar(&mut self, scalar: char, range: ByteRange) {
        self.scalars.push(ScalarCheckpoint {
            byte_start: range.start,
            byte_end: range.end(),
            utf16_offset: self.total_utf16_len,
        });
        self.total_byte_len = self.total_byte_len.max(range.end());
        let scalar_utf16_len = scalar.len_utf16() as u64;
        if scalar == '\n' {
            self.line_starts.push(range.end());
            self.line_start_utf16_offsets
                .push(self.total_utf16_len + scalar_utf16_len);
        }
        self.total_utf16_len += scalar_utf16_len;
    }

    /// Project an absolute byte offset to (1-based line, 1-based column in
    /// bytes from line start). Column is byte-based to keep the index
    /// independent of grapheme-cluster policy; reporters that want
    /// character-based columns rerun grapheme segmentation on the line's
    /// bytes.
    pub fn project(&self, byte_offset: u64) -> LineCol {
        let idx = self.line_index_for_byte_offset(byte_offset);
        let column_start = self.line_starts[idx];
        let line = idx as u32 + 1;
        let column = byte_offset.saturating_sub(column_start) as u32 + 1;
        LineCol { line, column }
    }

    /// Project an absolute byte offset to the host-facing coordinate set used
    /// by browser, devtools, WASM, and CLI JSON consumers. `line` and
    /// `column` are one-based; `column` / `utf16_column` are UTF-16 code-unit
    /// columns to match DOM `Range` and editor APIs. `utf16_offset` is
    /// zero-based from the beginning of the source.
    pub fn project_host(&self, byte_offset: u64) -> HostCoordinate {
        let line_index = self.line_index_for_byte_offset(byte_offset);
        let utf16_offset = self.utf16_offset_for_byte_offset(byte_offset);
        let line_utf16_start = self.line_start_utf16_offsets[line_index];
        let utf16_column = utf16_offset.saturating_sub(line_utf16_start) as u32 + 1;
        HostCoordinate {
            line: line_index as u32 + 1,
            column: utf16_column,
            utf16_offset,
            utf16_column,
        }
    }

    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    fn line_index_for_byte_offset(&self, byte_offset: u64) -> usize {
        match self.line_starts.binary_search(&byte_offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        }
    }

    fn utf16_offset_for_byte_offset(&self, byte_offset: u64) -> u64 {
        if byte_offset >= self.total_byte_len {
            return self.total_utf16_len + byte_offset.saturating_sub(self.total_byte_len);
        }

        let idx = self
            .scalars
            .partition_point(|scalar| scalar.byte_start <= byte_offset);
        if idx == 0 {
            return byte_offset;
        }
        let scalar = self.scalars[idx - 1];
        if byte_offset < scalar.byte_end {
            scalar.utf16_offset
        } else if let Some(next) = self.scalars.get(idx) {
            next.utf16_offset
        } else {
            self.total_utf16_len
        }
    }

    fn from_bytes_identity(source: &[u8]) -> Self {
        let mut index = Self::new();
        for (offset, byte) in source.iter().enumerate() {
            let offset = offset as u64;
            index.scalars.push(ScalarCheckpoint {
                byte_start: offset,
                byte_end: offset + 1,
                utf16_offset: offset,
            });
            if *byte == b'\n' {
                index.line_starts.push(offset + 1);
                index.line_start_utf16_offsets.push(offset + 1);
            }
        }
        index.total_byte_len = source.len() as u64;
        index.total_utf16_len = source.len() as u64;
        index
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostCoordinate {
    pub line: u32,
    pub column: u32,
    pub utf16_offset: u64,
    pub utf16_column: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(s: &str) -> LineIndex {
        let mut i = LineIndex::new();
        let mut off = 0u64;
        for c in s.chars() {
            let mut buf = [0u8; 4];
            let bytes = c.encode_utf8(&mut buf).len() as u32;
            i.record_scalar(c, ByteRange::new(off, bytes));
            off += bytes as u64;
        }
        i
    }

    #[test]
    fn first_line_offsets_project_one_indexed() {
        let i = idx("abc");
        assert_eq!(i.project(0), LineCol { line: 1, column: 1 });
        assert_eq!(i.project(2), LineCol { line: 1, column: 3 });
    }

    #[test]
    fn newline_starts_next_line_on_next_offset() {
        let i = idx("ab\ncd");
        // 'a'=0 'b'=1 '\n'=2 'c'=3 'd'=4
        assert_eq!(i.project(0), LineCol { line: 1, column: 1 });
        assert_eq!(i.project(1), LineCol { line: 1, column: 2 });
        assert_eq!(i.project(2), LineCol { line: 1, column: 3 }); // '\n' itself
        assert_eq!(i.project(3), LineCol { line: 2, column: 1 });
        assert_eq!(i.project(4), LineCol { line: 2, column: 2 });
    }

    #[test]
    fn multibyte_chars_advance_column_by_byte_width() {
        // 'é' is U+00E9, two bytes in UTF-8 (0xC3 0xA9).
        let i = idx("aéb");
        // 'a' = bytes [0..1), 'é' = bytes [1..3), 'b' = bytes [3..4)
        assert_eq!(i.project(1), LineCol { line: 1, column: 2 });
        assert_eq!(i.project(3), LineCol { line: 1, column: 4 });
    }

    #[test]
    fn many_lines_project_correctly() {
        let i = idx("a\nb\nc\nd");
        assert_eq!(i.project(0).line, 1);
        assert_eq!(i.project(2).line, 2);
        assert_eq!(i.project(4).line, 3);
        assert_eq!(i.project(6).line, 4);
        assert_eq!(i.line_count(), 4);
    }

    #[test]
    fn host_projection_uses_utf16_columns_across_crlf_and_surrogates() {
        let source = "{p | first\r\né😀 {bad}}\n";
        let i = LineIndex::from_utf8(source);
        let brace = source.find("{bad").expect("fixture has nested brace") as u64;
        assert_eq!(brace, 19);
        assert_eq!(i.project(brace), LineCol { line: 2, column: 8 });
        assert_eq!(
            i.project_host(brace),
            HostCoordinate {
                line: 2,
                column: 5,
                utf16_offset: 16,
                utf16_column: 5,
            }
        );
    }
}
