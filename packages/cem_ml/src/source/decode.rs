//! `EncodingDecoder` Tier A body.
//!
//! Streaming UTF-8 decoder with BOM detection. Per
//! `cem-ml-stack-design-impl.md` §3.1:
//!
//! - Absolute byte offsets are preserved on every emitted scalar.
//! - The first bytes of each source initiation are inspected for a BOM. A
//!   supported BOM selects the encoding and is skipped from decoded scalars;
//!   the BOM byte range remains addressable via the returned `BomInfo`.
//! - Chunk boundaries are handled with a bounded carry-over (up to 3 bytes
//!   for an in-progress UTF-8 sequence).
//! - Invalid byte sequences emit `cem.byte.invalid_utf8` diagnostics; the
//!   decoder skips the offending byte and continues so a single malformed
//!   sequence does not abort the stream.
//! - XML 1.0 restricted characters emit `cem.byte.invalid_xml_char` warnings
//!   per the XML 1.0 `Char` production.

use crate::diagnostics::{Diagnostic, Severity};
use crate::source::{
    BomInfo, ByteChunk, ByteRange, ByteSource, DecodedChunk, Encoding, EncodingDecoder,
    EncodingSelection, SourceId,
};

/// Streaming UTF-8 decoder. UTF-16 and Latin-1 are recognized via BOM
/// (UTF-16LE/BE) or `default_encoding`, but their decoded scalar output is
/// stubbed in Tier A and emits `cem.byte.unsupported_encoding`.
pub struct Utf8Decoder<S: ByteSource> {
    source: S,
    config: DecodeConfig,
    initiated: bool,
    encoding: Encoding,
    selection: EncodingSelection,
    bom: Option<BomInfo>,
    /// Bytes carried over from the previous chunk because they were the start
    /// of an incomplete UTF-8 sequence. `carry_start` is their absolute
    /// source offset.
    carry: Vec<u8>,
    carry_start: u64,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default)]
pub struct DecodeConfig {
    /// Encoding hint from the caller / transport (e.g. `Content-Type`
    /// charset). Used only when no BOM is found.
    pub default_encoding: Option<Encoding>,
    /// Profile flag — when true, restricted XML 1.0 characters emit
    /// `cem.byte.invalid_xml_char` warnings.
    pub strict_xml_chars: bool,
}

impl<S: ByteSource> Utf8Decoder<S> {
    pub fn new(source: S) -> Self {
        Self::with_config(source, DecodeConfig::default())
    }

    pub fn with_config(source: S, config: DecodeConfig) -> Self {
        Self {
            source,
            config,
            initiated: false,
            encoding: Encoding::Utf8,
            selection: EncodingSelection::Utf8Fallback,
            bom: None,
            carry: Vec::new(),
            carry_start: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn source_id(&self) -> SourceId {
        self.source.source_id()
    }
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }
    pub fn selection(&self) -> EncodingSelection {
        self.selection
    }
    pub fn bom(&self) -> Option<BomInfo> {
        self.bom
    }

    /// Take accumulated diagnostics since the last call. The decoder retains
    /// no diagnostic history; callers fold them into the report tree per
    /// AC-P-3.
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    fn detect_bom(&mut self, chunk: &ByteChunk) -> usize {
        let bytes = &chunk.bytes;
        let start = chunk.byte_range.start;
        if bytes.len() >= 3 && bytes[0..3] == [0xEF, 0xBB, 0xBF] {
            self.encoding = Encoding::Utf8;
            self.selection = EncodingSelection::Bom;
            self.bom = Some(BomInfo {
                encoding: Encoding::Utf8,
                byte_range: ByteRange::new(start, 3),
            });
            return 3;
        }
        if bytes.len() >= 2 && bytes[0..2] == [0xFF, 0xFE] {
            self.encoding = Encoding::Utf16Le;
            self.selection = EncodingSelection::Bom;
            self.bom = Some(BomInfo {
                encoding: Encoding::Utf16Le,
                byte_range: ByteRange::new(start, 2),
            });
            self.push_diag_unsupported(start, 2, "UTF-16LE");
            return 2;
        }
        if bytes.len() >= 2 && bytes[0..2] == [0xFE, 0xFF] {
            self.encoding = Encoding::Utf16Be;
            self.selection = EncodingSelection::Bom;
            self.bom = Some(BomInfo {
                encoding: Encoding::Utf16Be,
                byte_range: ByteRange::new(start, 2),
            });
            self.push_diag_unsupported(start, 2, "UTF-16BE");
            return 2;
        }
        match self.config.default_encoding {
            Some(Encoding::Utf8) | None => {
                self.encoding = Encoding::Utf8;
                self.selection = if self.config.default_encoding.is_some() {
                    EncodingSelection::DefaultParameter
                } else {
                    EncodingSelection::Utf8Fallback
                };
            }
            Some(enc) => {
                self.encoding = enc;
                self.selection = EncodingSelection::DefaultParameter;
                self.push_diag_unsupported(start, 0, encoding_label(enc));
            }
        }
        0
    }

    fn push_diag_unsupported(&mut self, byte_offset: u64, len: u32, label: &str) {
        self.diagnostics.push(Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset: Some(byte_offset),
            code: "cem.byte.unsupported_encoding".to_owned(),
            severity: Severity::Error,
            message: format!(
                "encoding `{label}` is not decoded in Tier A; only UTF-8 is supported"
            ),
            node: None,
        });
        let _ = len; // reserved for future range-bearing diagnostic projection.
    }

    fn emit_invalid_utf8(&mut self, byte_offset: u64) {
        self.diagnostics.push(Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset: Some(byte_offset),
            code: "cem.byte.invalid_utf8".to_owned(),
            severity: Severity::Error,
            message: "invalid UTF-8 byte sequence".to_owned(),
            node: None,
        });
    }

    fn maybe_flag_xml_char(&mut self, scalar: char, range: ByteRange) {
        if !self.config.strict_xml_chars {
            return;
        }
        if !is_xml10_char(scalar) {
            self.diagnostics.push(Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: Some(range.start),
                code: "cem.byte.invalid_xml_char".to_owned(),
                severity: Severity::Warning,
                message: format!(
                    "U+{:04X} is outside the XML 1.0 Char production",
                    scalar as u32
                ),
                node: None,
            });
        }
    }

    fn decode_utf8_chunk(&mut self, chunk_bytes: &[u8], chunk_start: u64) -> Vec<(char, ByteRange)> {
        let mut scalars = Vec::with_capacity(chunk_bytes.len());
        let mut i = 0;
        while i < chunk_bytes.len() {
            let abs_offset = chunk_start + i as u64;
            let lead = chunk_bytes[i];
            let needed = utf8_lead_width(lead);
            if needed == 0 {
                self.emit_invalid_utf8(abs_offset);
                i += 1;
                continue;
            }
            if i + needed > chunk_bytes.len() {
                // Incomplete sequence at the chunk tail — carry over.
                self.carry.extend_from_slice(&chunk_bytes[i..]);
                self.carry_start = abs_offset;
                break;
            }
            let seq = &chunk_bytes[i..i + needed];
            match std::str::from_utf8(seq) {
                Ok(s) => {
                    let ch = s.chars().next().expect("non-empty seq");
                    let range = ByteRange::new(abs_offset, needed as u32);
                    self.maybe_flag_xml_char(ch, range);
                    scalars.push((ch, range));
                    i += needed;
                }
                Err(_) => {
                    self.emit_invalid_utf8(abs_offset);
                    i += 1;
                }
            }
        }
        scalars
    }
}

impl<S: ByteSource> EncodingDecoder for Utf8Decoder<S> {
    fn decode_next(&mut self) -> Option<DecodedChunk> {
        let chunk = match self.source.next_chunk() {
            Ok(Some(c)) => c,
            Ok(None) => {
                if !self.carry.is_empty() {
                    let start = self.carry_start;
                    let len = self.carry.len();
                    for off in 0..len as u64 {
                        self.emit_invalid_utf8(start + off);
                    }
                    self.carry.clear();
                }
                return None;
            }
            Err(e) => {
                self.diagnostics.push(Diagnostic {
                    uri: None,
                    line: None,
                    column: None,
                    byte_offset: None,
                    code: "cem.byte.io_error".to_owned(),
                    severity: Severity::Fatal,
                    message: e.to_string(),
                    node: None,
                });
                return None;
            }
        };

        let mut skip = 0;
        if !self.initiated {
            skip = self.detect_bom(&chunk);
            self.initiated = true;
        }

        if self.encoding != Encoding::Utf8 {
            // Already flagged via detect_bom or default-encoding path; stop
            // producing scalars so consumers don't run on garbage.
            return Some(DecodedChunk {
                source_id: chunk.source_id,
                byte_range: chunk.byte_range,
                encoding: self.encoding,
                scalars: Vec::new(),
            });
        }

        // Combine carry + body; the carry was retained from the prior call
        // because it was the start of an incomplete UTF-8 sequence.
        let body = &chunk.bytes[skip..];
        let body_start = chunk.byte_range.start + skip as u64;
        let combined: Vec<u8>;
        let combined_start: u64;
        if self.carry.is_empty() {
            combined = body.to_vec();
            combined_start = body_start;
        } else {
            // Stitch carry first.
            let mut buf = std::mem::take(&mut self.carry);
            let stitched_start = self.carry_start;
            buf.extend_from_slice(body);
            combined = buf;
            combined_start = stitched_start;
            self.carry_start = 0;
        }

        let scalars = self.decode_utf8_chunk(&combined, combined_start);

        Some(DecodedChunk {
            source_id: chunk.source_id,
            byte_range: chunk.byte_range,
            encoding: Encoding::Utf8,
            scalars,
        })
    }
}

fn utf8_lead_width(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => 0,
    }
}

/// XML 1.0 `Char` production: tab/LF/CR plus most of the BMP/non-BMP minus
/// the surrogate range and U+FFFE/U+FFFF.
fn is_xml10_char(c: char) -> bool {
    matches!(c as u32,
        0x09 | 0x0A | 0x0D |
        0x20..=0xD7FF |
        0xE000..=0xFFFD |
        0x10000..=0x10FFFF)
}

fn encoding_label(e: Encoding) -> &'static str {
    match e {
        Encoding::Utf8 => "UTF-8",
        Encoding::Utf16Le => "UTF-16LE",
        Encoding::Utf16Be => "UTF-16BE",
        Encoding::Latin1 => "Latin-1",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{BytesSource, SourceId};

    fn collect(decoder: &mut Utf8Decoder<BytesSource>) -> (Vec<(char, ByteRange)>, Vec<Diagnostic>) {
        let mut scalars = Vec::new();
        while let Some(c) = decoder.decode_next() {
            scalars.extend(c.scalars);
        }
        let diags = decoder.take_diagnostics();
        (scalars, diags)
    }

    #[test]
    fn ascii_input_decodes_with_correct_offsets() {
        let src = BytesSource::new(SourceId(1), b"abc".to_vec());
        let mut d = Utf8Decoder::new(src);
        let (scalars, diags) = collect(&mut d);
        assert!(diags.is_empty());
        assert_eq!(scalars.len(), 3);
        assert_eq!(scalars[0].0, 'a');
        assert_eq!(scalars[0].1, ByteRange::new(0, 1));
        assert_eq!(scalars[2].1, ByteRange::new(2, 1));
        assert_eq!(d.encoding(), Encoding::Utf8);
        assert_eq!(d.selection(), EncodingSelection::Utf8Fallback);
        assert!(d.bom().is_none());
    }

    #[test]
    fn utf8_bom_is_skipped_and_selects_utf8() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"ok");
        let src = BytesSource::new(SourceId(2), bytes);
        let mut d = Utf8Decoder::new(src);
        let (scalars, diags) = collect(&mut d);
        assert!(diags.is_empty());
        assert_eq!(d.selection(), EncodingSelection::Bom);
        assert_eq!(d.bom().unwrap().byte_range, ByteRange::new(0, 3));
        // Scalar offsets are absolute → start at 3 (after BOM).
        assert_eq!(scalars[0], ('o', ByteRange::new(3, 1)));
        assert_eq!(scalars[1], ('k', ByteRange::new(4, 1)));
    }

    #[test]
    fn utf16le_bom_is_detected_and_flagged_unsupported() {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend_from_slice(&[0x6F, 0x00, 0x6B, 0x00]); // "ok" in UTF-16LE
        let src = BytesSource::new(SourceId(3), bytes);
        let mut d = Utf8Decoder::new(src);
        let (scalars, diags) = collect(&mut d);
        assert_eq!(d.encoding(), Encoding::Utf16Le);
        assert!(scalars.is_empty(), "Tier A does not decode UTF-16 scalars");
        assert!(diags.iter().any(|d| d.code == "cem.byte.unsupported_encoding"));
    }

    #[test]
    fn multibyte_sequence_split_across_chunks() {
        // 'é' (U+00E9) = 0xC3 0xA9; chunk size 1 splits every byte.
        let src = BytesSource::new(SourceId(4), "aéb".as_bytes().to_vec()).with_chunk_size(1);
        let mut d = Utf8Decoder::new(src);
        let (scalars, diags) = collect(&mut d);
        assert!(diags.is_empty(), "split should not produce diagnostics");
        assert_eq!(scalars.len(), 3);
        assert_eq!(scalars[0], ('a', ByteRange::new(0, 1)));
        assert_eq!(scalars[1], ('é', ByteRange::new(1, 2)));
        assert_eq!(scalars[2], ('b', ByteRange::new(3, 1)));
    }

    #[test]
    fn orphan_continuation_byte_emits_diagnostic() {
        let src = BytesSource::new(SourceId(5), vec![b'a', 0x80, b'b']);
        let mut d = Utf8Decoder::new(src);
        let (scalars, diags) = collect(&mut d);
        assert_eq!(scalars.len(), 2);
        assert_eq!(scalars[0].0, 'a');
        assert_eq!(scalars[1].0, 'b');
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "cem.byte.invalid_utf8");
        assert_eq!(diags[0].byte_offset, Some(1));
    }

    #[test]
    fn truncated_sequence_at_eof_emits_diagnostic_per_byte() {
        // 0xC3 with no continuation = truncated 2-byte sequence at EOF.
        let src = BytesSource::new(SourceId(6), vec![b'a', 0xC3]);
        let mut d = Utf8Decoder::new(src);
        let (scalars, diags) = collect(&mut d);
        assert_eq!(scalars.len(), 1);
        assert_eq!(scalars[0].0, 'a');
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].byte_offset, Some(1));
        assert_eq!(diags[0].code, "cem.byte.invalid_utf8");
    }

    #[test]
    fn restricted_xml_char_flagged_when_strict_xml() {
        // U+0000 (NUL) is restricted in XML 1.0.
        let src = BytesSource::new(SourceId(7), vec![b'a', 0x00, b'b']);
        let mut d = Utf8Decoder::with_config(
            src,
            DecodeConfig {
                default_encoding: None,
                strict_xml_chars: true,
            },
        );
        let (scalars, diags) = collect(&mut d);
        assert_eq!(scalars.len(), 3);
        let xml = diags
            .iter()
            .find(|d| d.code == "cem.byte.invalid_xml_char")
            .expect("expected XML-char diag");
        assert_eq!(xml.byte_offset, Some(1));
        assert_eq!(xml.severity, Severity::Warning);
    }

    #[test]
    fn restricted_xml_char_not_flagged_when_relaxed() {
        let src = BytesSource::new(SourceId(8), vec![0x00]);
        let mut d = Utf8Decoder::new(src);
        let (_, diags) = collect(&mut d);
        assert!(diags.iter().all(|d| d.code != "cem.byte.invalid_xml_char"));
    }

    #[test]
    fn empty_input_emits_no_chunks() {
        let src = BytesSource::new(SourceId(9), Vec::new());
        let mut d = Utf8Decoder::new(src);
        assert!(d.decode_next().is_none());
        assert!(d.take_diagnostics().is_empty());
    }

    #[test]
    fn long_input_chunks_under_default_size_decodes_all_bytes() {
        let s = "hello world\n".repeat(8);
        let bytes = s.as_bytes().to_vec();
        let src = BytesSource::new(SourceId(10), bytes.clone()).with_chunk_size(7);
        let mut d = Utf8Decoder::new(src);
        let (scalars, diags) = collect(&mut d);
        assert!(diags.is_empty());
        assert_eq!(scalars.len(), s.chars().count());
        // Last scalar's end equals input length.
        let last = scalars.last().unwrap().1;
        assert_eq!(last.end(), bytes.len() as u64);
    }
}
