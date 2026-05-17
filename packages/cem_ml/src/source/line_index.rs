//! `LineIndex` — streaming byte-offset → (line, column) projection.
//!
//! Line/column are derived reporting coordinates per `cem-ml-stack-design-impl.md`
//! §2.1; byte offsets are the ground truth. The index records the absolute
//! byte offset of every newline as scalars flow past the decoder, so
//! projection is O(log n) over the accumulated checkpoints.

use crate::source::ByteRange;

/// Newline projection for a single `SourceId`.
#[derive(Debug, Default, Clone)]
pub struct LineIndex {
    /// Absolute byte offset of each `\n` observed in the stream, in order.
    line_starts: Vec<u64>,
}

impl LineIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one decoded scalar at the given absolute byte offset. Pass the
    /// byte range emitted by the decoder so multi-byte UTF-8 characters land
    /// at the start byte of their sequence.
    pub fn record_scalar(&mut self, scalar: char, range: ByteRange) {
        if scalar == '\n' {
            self.line_starts.push(range.start);
        }
    }

    /// Project an absolute byte offset to (1-based line, 1-based column in
    /// bytes from line start). Column is byte-based to keep the index
    /// independent of grapheme-cluster policy; reporters that want
    /// character-based columns rerun grapheme segmentation on the line's
    /// bytes.
    pub fn project(&self, byte_offset: u64) -> LineCol {
        // Find the largest recorded newline offset strictly less than
        // byte_offset; that newline ended the previous line.
        let prev_newline = match self.line_starts.binary_search(&byte_offset) {
            Ok(_) => {
                // byte_offset == a newline position → it's the '\n' character
                // itself. Treat it as still on the previous line.
                let idx = self
                    .line_starts
                    .iter()
                    .position(|&n| n == byte_offset)
                    .expect("binary_search Ok implies presence");
                idx.checked_sub(1).map(|i| self.line_starts[i])
            }
            Err(idx) => idx.checked_sub(1).map(|i| self.line_starts[i]),
        };
        let line = match prev_newline {
            None => 1,
            Some(nl) => {
                // count of newlines <= nl, plus one for the line after
                let count = self
                    .line_starts
                    .iter()
                    .position(|&n| n > nl)
                    .unwrap_or(self.line_starts.len());
                (count as u32) + 1
            }
        };
        let column_start = prev_newline.map(|nl| nl + 1).unwrap_or(0);
        let column = (byte_offset.saturating_sub(column_start) as u32) + 1;
        LineCol { line, column }
    }

    pub fn line_count(&self) -> u32 {
        (self.line_starts.len() as u32) + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub column: u32,
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
}
