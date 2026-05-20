//! Deterministic writer + helpers shared by every emitter.
//!
//! Reference: `cem-ml-stack-design-impl.md` §3.4.2.3 and
//! `cem-ml-stack-design.md` §13.2.4 (byte-stability rules).

use super::error::EmitError;
use super::output::ContentHash;

const INDENT_STEP: usize = 2;

/// Single accumulator used by every emitter. Writes are
/// indent-prefixed, LF-terminated, hashed-as-they-go. The hasher is
/// updated on every byte so `finalize()` returns both the byte buffer
/// and a `cem-bin/1+blake3` content hash without a second pass.
pub struct DeterministicWriter {
    sink: Vec<u8>,
    indent: u16,
    hasher: blake3::Hasher,
}

impl DeterministicWriter {
    pub fn new() -> Self {
        Self {
            sink: Vec::new(),
            indent: 0,
            hasher: blake3::Hasher::new(),
        }
    }

    /// Current indent level (0 = no leading spaces).
    pub fn indent_level(&self) -> u16 {
        self.indent
    }

    pub fn indent(&mut self) {
        self.indent = self.indent.saturating_add(1);
    }

    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    /// Append `text` followed by `\n`. Rejects `\r`, trailing
    /// whitespace, and embedded LF (a multi-line write would skip the
    /// per-line indent step, so callers MUST issue one `line()` call
    /// per line).
    pub fn line(&mut self, text: &str) -> Result<(), EmitError> {
        Self::validate_line(text)?;
        for _ in 0..(self.indent as usize * INDENT_STEP) {
            self.push_byte(b' ');
        }
        for byte in text.as_bytes() {
            self.push_byte(*byte);
        }
        self.push_byte(b'\n');
        Ok(())
    }

    /// Empty line (no indent, just `\n`).
    pub fn blank(&mut self) {
        self.push_byte(b'\n');
    }

    /// Finalize and return the produced bytes + content hash. The
    /// final newline guarantee (§13.2.4 rule 1) is asserted here.
    pub fn finalize(mut self) -> Result<(Vec<u8>, ContentHash), EmitError> {
        if !self.sink.ends_with(b"\n") {
            return Err(EmitError::NonDeterministicWrite {
                reason: "missing final newline",
            });
        }
        let hex = self.hasher.finalize().to_hex().to_string();
        Ok((
            std::mem::take(&mut self.sink),
            ContentHash {
                scheme: ContentHash::SCHEME,
                hex,
            },
        ))
    }

    fn validate_line(text: &str) -> Result<(), EmitError> {
        if text.as_bytes().contains(&b'\r') {
            return Err(EmitError::NonDeterministicWrite {
                reason: "CR byte in line()",
            });
        }
        if text.as_bytes().contains(&b'\n') {
            return Err(EmitError::NonDeterministicWrite {
                reason: "embedded LF in line(); callers must issue one line() per line",
            });
        }
        if text.ends_with(' ') || text.ends_with('\t') {
            return Err(EmitError::NonDeterministicWrite {
                reason: "trailing whitespace in line()",
            });
        }
        Ok(())
    }

    fn push_byte(&mut self, b: u8) {
        self.sink.push(b);
        self.hasher.update(&[b]);
    }
}

impl Default for DeterministicWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: escape a string for inclusion in an XML attribute
/// value or element text. RELAX NG content is plain XML, so the same
/// five-character escape applies.
pub fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// Escape a string literal for RELAX NG compact syntax. Compact RNG
/// string literals are double-quoted and use C-style `\` escapes.
/// CEM annotation values are always ASCII identifiers in cem-core/1,
/// so the minimal escape set (`\` and `"`) covers every emit case.
pub fn rnc_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_with_indent_writes_two_space_prefix() {
        let mut w = DeterministicWriter::new();
        w.indent();
        w.line("hello").unwrap();
        let (bytes, _) = w.finalize().unwrap();
        assert_eq!(bytes, b"  hello\n");
    }

    #[test]
    fn cr_byte_is_rejected() {
        let mut w = DeterministicWriter::new();
        let err = w.line("bad\rline").unwrap_err();
        assert!(matches!(err, EmitError::NonDeterministicWrite { .. }));
    }

    #[test]
    fn trailing_space_is_rejected() {
        let mut w = DeterministicWriter::new();
        let err = w.line("oops ").unwrap_err();
        assert!(matches!(err, EmitError::NonDeterministicWrite { .. }));
    }

    #[test]
    fn embedded_lf_is_rejected() {
        let mut w = DeterministicWriter::new();
        let err = w.line("a\nb").unwrap_err();
        assert!(matches!(err, EmitError::NonDeterministicWrite { .. }));
    }

    #[test]
    fn finalize_requires_trailing_newline() {
        let mut w = DeterministicWriter::new();
        // Manually push a byte without a trailing newline.
        w.push_byte(b'x');
        let err = w.finalize().unwrap_err();
        assert!(matches!(err, EmitError::NonDeterministicWrite { .. }));
    }

    #[test]
    fn finalize_returns_blake3_hash_of_bytes() {
        let mut w = DeterministicWriter::new();
        w.line("a").unwrap();
        let (bytes, hash) = w.finalize().unwrap();
        let expected = blake3::hash(&bytes).to_hex().to_string();
        assert_eq!(hash.hex, expected);
        assert_eq!(hash.scheme, "cem-bin/1+blake3");
    }

    #[test]
    fn xml_escape_handles_five_special_chars() {
        assert_eq!(xml_escape("a<b>&\"'"), "a&lt;b&gt;&amp;&quot;&apos;");
        assert_eq!(xml_escape("plain"), "plain");
    }

    #[test]
    fn rnc_escape_handles_backslash_and_quote() {
        assert_eq!(rnc_escape("a\\b\"c"), "a\\\\b\\\"c");
        assert_eq!(rnc_escape("primary"), "primary");
    }
}
