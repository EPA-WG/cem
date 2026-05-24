//! HTML parity tokenizer — Tier A profile.
//!
//! Custom WHATWG-state tokenizer covering the subset needed by the
//! canonical `examples/semantic/*.html` parity fixtures. Implementation
//! mirrors the CEM curly tokenizer (eager scalar buffer + state machine
//! walking the buffer) so every emitted `SchemaToken` carries an absolute
//! byte range and a source-map stack rooted in
//! `TransformKind::HtmlTokenizer`. The token shapes match what the CEM
//! tokenizer emits (`NodeStart` / `NodeEnd` / `Attribute` / `Text` /
//! `Trivia` / `Comment` / `ProcessingInstruction`), so the shared event
//! normalizer can lower both profiles into the same `NormalizedEvent`
//! stream.
//!
//! Covered states (subset of the WHATWG HTML tokenizer):
//!
//! - `Data` — text content until `<`.
//! - `TagOpen` / `EndTagOpen` / `TagName`.
//! - `BeforeAttributeName` / `AttributeName` / `AfterAttributeName`.
//! - `BeforeAttributeValue` / `AttributeValue(Double|Single|Unquoted)` /
//!   `AfterAttributeValueQuoted`.
//! - `SelfClosingStartTag`.
//! - `MarkupDeclarationOpen` → `Comment` (`<!-- -->`) or
//!   `DOCTYPE` (`<!DOCTYPE ...>`).
//!
//! Deferred to Phase 11 follow-up: RAWTEXT / RCDATA / ScriptData state
//! machines for `<style>`, `<script>`, `<textarea>`, `<title>` raw-text
//! bodies; CDATA sections; numeric / named character references
//! (decoded text passes through verbatim in Tier A).
//!
//! Void elements per the HTML5 spec emit `NodeStart` immediately
//! followed by a synthetic `NodeEnd` so the event stream balances
//! against the wrapping CEM-ML parity fixture.

use crate::diagnostics::{Diagnostic, Severity};
use crate::source::decode::{DecodeConfig, Utf8Decoder};
use crate::source::{ByteRange, ByteSource, EncodingDecoder, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer, TokenizerProfile};
use std::collections::VecDeque;

pub struct HtmlTokenizer {
    source_id: SourceId,
    scalars: Vec<(char, ByteRange)>,
    cursor: usize,
    pending: VecDeque<SchemaToken>,
    diagnostics: Vec<Diagnostic>,
    base_source_map: SourceMapStack,
    end_offset: u64,
}

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

fn is_void(name: &str) -> bool {
    VOID_ELEMENTS.contains(&name)
}

impl HtmlTokenizer {
    pub fn from_source<S: ByteSource>(source: S) -> Self {
        let mut decoder = Utf8Decoder::with_config(
            source,
            DecodeConfig {
                default_encoding: None,
                strict_xml_chars: false,
            },
        );
        let source_id = decoder.source_id();
        let mut scalars = Vec::new();
        while let Some(c) = decoder.decode_next() {
            scalars.extend(c.scalars);
        }
        let diagnostics = decoder.take_diagnostics();
        let end_offset = scalars.last().map(|(_, r)| r.end()).unwrap_or(0);
        let base_source_map = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id,
                span: FrameSpan::Single(ByteRange::new(0, end_offset as u32)),
                transform: TransformKind::HtmlTokenizer,
            }],
        };
        let mut t = Self {
            source_id,
            scalars,
            cursor: 0,
            pending: VecDeque::new(),
            diagnostics,
            base_source_map,
            end_offset,
        };
        t.scan_document();
        t
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    fn peek(&self) -> Option<char> {
        self.scalars.get(self.cursor).map(|(c, _)| *c)
    }
    fn peek_at(&self, n: usize) -> Option<char> {
        self.scalars.get(self.cursor + n).map(|(c, _)| *c)
    }
    fn advance(&mut self) {
        self.cursor += 1;
    }
    fn at_end(&self) -> bool {
        self.cursor >= self.scalars.len()
    }

    fn scan_document(&mut self) {
        while !self.at_end() {
            self.scan_data();
        }
    }

    /// Data state — collect text until `<` or EOF.
    fn scan_data(&mut self) {
        let start = self.cursor;
        let mut last_text_end = self.cursor;
        while let Some(c) = self.peek() {
            if c == '<' {
                break;
            }
            self.advance();
            last_text_end = self.cursor;
        }
        if last_text_end > start {
            let range = self.range_from(start, last_text_end);
            let data: String = self.scalars[start..last_text_end]
                .iter()
                .map(|(c, _)| *c)
                .collect();
            // Split whitespace-only runs into Trivia for parity with the
            // CEM tokenizer's handling.
            if data.trim().is_empty() {
                self.emit(SchemaTokenKind::Trivia(data), range);
            } else {
                self.emit(SchemaTokenKind::Text(data), range);
            }
        }
        if self.peek() == Some('<') {
            self.scan_tag_open();
        }
    }

    fn scan_tag_open(&mut self) {
        debug_assert_eq!(self.peek(), Some('<'));
        let open_start = self.cursor;
        self.advance(); // consume '<'
        match self.peek() {
            Some('/') => {
                self.advance();
                self.scan_end_tag(open_start);
            }
            Some('!') => {
                self.advance();
                self.scan_markup_declaration(open_start);
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.scan_start_tag(open_start);
            }
            _ => {
                // Treat lone `<` as text.
                let range = self.range_from(open_start, self.cursor);
                self.emit(SchemaTokenKind::Text("<".into()), range);
            }
        }
    }

    fn scan_start_tag(&mut self, open_start: usize) {
        let name_start = self.cursor;
        while let Some(c) = self.peek() {
            if is_tag_name_char(c) {
                self.advance();
            } else {
                break;
            }
        }
        let name: String = self.scalars[name_start..self.cursor]
            .iter()
            .map(|(c, _)| c.to_ascii_lowercase())
            .collect();
        let head_range = self.range_from(open_start, self.cursor);
        self.emit(parity_start_kind(&name), head_range);

        let mut self_closing = false;
        loop {
            self.skip_html_whitespace();
            match self.peek() {
                None => {
                    self.diag(
                        "cem.html.unterminated_tag",
                        Severity::Error,
                        "start tag did not close before EOF",
                        self.current_offset(),
                    );
                    return;
                }
                Some('>') => {
                    let close_start = self.cursor;
                    self.advance();
                    if self_closing || is_void(&name) {
                        let close_range = self.range_from(close_start, self.cursor);
                        self.emit(
                            SchemaTokenKind::NodeEnd {
                                name: parity_end_name(&name),
                            },
                            close_range,
                        );
                    }
                    return;
                }
                Some('/') => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self_closing = true;
                    }
                    continue;
                }
                Some(_) => self.scan_attribute(),
            }
        }
    }

    fn scan_attribute(&mut self) {
        let attr_start = self.cursor;
        let name_start = self.cursor;
        while let Some(c) = self.peek() {
            if c == '=' || c == '>' || c == '/' || c.is_whitespace() {
                break;
            }
            self.advance();
        }
        let name_end = self.cursor;
        let raw_name: String = self.scalars[name_start..name_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        // Normalize attribute names to lowercase for HTML.
        let name = raw_name.to_ascii_lowercase();
        let name_range = self.range_from(name_start, name_end);

        self.skip_html_whitespace();
        let mut value: Option<String> = None;
        let mut value_range: Option<ByteRange> = None;
        if self.peek() == Some('=') {
            self.advance();
            self.skip_html_whitespace();
            match self.peek() {
                Some('"') => {
                    let (v, r) = self.scan_quoted_attr_value('"');
                    value = Some(v);
                    value_range = Some(r);
                }
                Some('\'') => {
                    let (v, r) = self.scan_quoted_attr_value('\'');
                    value = Some(v);
                    value_range = Some(r);
                }
                _ => {
                    let (v, r) = self.scan_unquoted_attr_value();
                    value = Some(v);
                    value_range = Some(r);
                }
            }
        }
        let total_range = self.range_from(attr_start, self.cursor);
        self.emit(
            SchemaTokenKind::Attribute {
                name,
                value,
                name_range,
                value_range,
            },
            total_range,
        );
    }

    fn scan_quoted_attr_value(&mut self, quote: char) -> (String, ByteRange) {
        let start = self.cursor;
        self.advance(); // opening quote
        let body_start = self.cursor;
        while let Some(c) = self.peek() {
            if c == quote {
                break;
            }
            self.advance();
        }
        let body_end = self.cursor;
        let mut data: String = self.scalars[body_start..body_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        if self.peek() == Some(quote) {
            self.advance();
        } else {
            self.diag(
                "cem.html.unterminated_attribute_value",
                Severity::Error,
                "attribute value is missing its closing quote",
                self.current_offset(),
            );
            data.push('\u{FFFD}');
        }
        let range = self.range_from(start, self.cursor);
        (data, range)
    }

    fn scan_unquoted_attr_value(&mut self) -> (String, ByteRange) {
        let start = self.cursor;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == '>' || c == '/' {
                break;
            }
            self.advance();
        }
        let data: String = self.scalars[start..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let range = self.range_from(start, self.cursor);
        (data, range)
    }

    fn scan_end_tag(&mut self, open_start: usize) {
        let name_start = self.cursor;
        while let Some(c) = self.peek() {
            if is_tag_name_char(c) {
                self.advance();
            } else {
                break;
            }
        }
        let name: String = self.scalars[name_start..self.cursor]
            .iter()
            .map(|(c, _)| c.to_ascii_lowercase())
            .collect();
        // Skip to `>`.
        while let Some(c) = self.peek() {
            if c == '>' {
                break;
            }
            self.advance();
        }
        if self.peek() == Some('>') {
            self.advance();
        }
        let range = self.range_from(open_start, self.cursor);
        self.emit(
            SchemaTokenKind::NodeEnd {
                name: parity_end_name(&name),
            },
            range,
        );
    }

    fn scan_markup_declaration(&mut self, open_start: usize) {
        // Already consumed `<!`. Distinguish:
        //   `--` → comment
        //   ASCII-case-insensitive `DOCTYPE` → doctype
        if self.peek() == Some('-') && self.peek_at(1) == Some('-') {
            self.advance();
            self.advance();
            self.scan_comment(open_start);
            return;
        }
        if self.match_keyword_ascii_case_insensitive("DOCTYPE") {
            self.scan_doctype(open_start);
            return;
        }
        // Bogus declaration: consume to `>` and emit a Comment.
        let body_start = self.cursor;
        while let Some(c) = self.peek() {
            if c == '>' {
                break;
            }
            self.advance();
        }
        let body_end = self.cursor;
        if self.peek() == Some('>') {
            self.advance();
        }
        let data: String = self.scalars[body_start..body_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let range = self.range_from(open_start, self.cursor);
        self.emit(SchemaTokenKind::Comment(data), range);
    }

    fn scan_comment(&mut self, open_start: usize) {
        let body_start = self.cursor;
        loop {
            match (self.peek(), self.peek_at(1), self.peek_at(2)) {
                (Some('-'), Some('-'), Some('>')) => {
                    let body_end = self.cursor;
                    self.advance();
                    self.advance();
                    self.advance();
                    let data: String = self.scalars[body_start..body_end]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    let range = self.range_from(open_start, self.cursor);
                    self.emit(SchemaTokenKind::Comment(data), range);
                    return;
                }
                (None, _, _) => {
                    self.diag(
                        "cem.html.unterminated_comment",
                        Severity::Error,
                        "comment is missing its closing `-->`",
                        self.current_offset(),
                    );
                    let data: String = self.scalars[body_start..self.cursor]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    let range = self.range_from(open_start, self.cursor);
                    self.emit(SchemaTokenKind::Comment(data), range);
                    return;
                }
                _ => self.advance(),
            }
        }
    }

    fn scan_doctype(&mut self, open_start: usize) {
        // Already matched `DOCTYPE`. Skip whitespace then collect body to `>`.
        self.skip_html_whitespace();
        let body_start = self.cursor;
        while let Some(c) = self.peek() {
            if c == '>' {
                break;
            }
            self.advance();
        }
        let body_end = self.cursor;
        if self.peek() == Some('>') {
            self.advance();
        }
        let data: String = self.scalars[body_start..body_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let range = self.range_from(open_start, self.cursor);
        self.emit(
            SchemaTokenKind::ProcessingInstruction {
                target: "DOCTYPE".to_owned(),
                data: data.trim().to_owned(),
            },
            range,
        );
    }

    fn match_keyword_ascii_case_insensitive(&mut self, keyword: &str) -> bool {
        if self.cursor + keyword.len() > self.scalars.len() {
            return false;
        }
        for (i, expected) in keyword.chars().enumerate() {
            let actual = self.scalars[self.cursor + i].0;
            if !actual.eq_ignore_ascii_case(&expected) {
                return false;
            }
        }
        self.cursor += keyword.len();
        true
    }

    fn skip_html_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C') {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn current_offset(&self) -> u64 {
        self.scalars
            .get(self.cursor)
            .map(|(_, r)| r.start)
            .unwrap_or(self.end_offset)
    }

    fn range_from(&self, start: usize, end: usize) -> ByteRange {
        let start_off = self
            .scalars
            .get(start)
            .map(|(_, r)| r.start)
            .unwrap_or(self.end_offset);
        let end_off = if end == 0 {
            start_off
        } else {
            self.scalars
                .get(end - 1)
                .map(|(_, r)| r.end())
                .unwrap_or(self.end_offset)
        };
        ByteRange::new(start_off, (end_off - start_off) as u32)
    }

    fn emit(&mut self, kind: SchemaTokenKind, byte_range: ByteRange) {
        self.pending.push_back(SchemaToken {
            kind,
            byte_range,
            profile: TokenizerProfile::Html,
            source_map: self.base_source_map.clone(),
        });
    }

    fn diag(&mut self, code: &str, severity: Severity, message: &str, byte_offset: u64) {
        self.diagnostics.push(Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset: Some(byte_offset),
            code: code.to_owned(),
            severity,
            message: message.to_owned(),
            node: None,
            source_map: None,
        });
    }
}

fn is_tag_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':')
}

fn parity_start_kind(name: &str) -> SchemaTokenKind {
    match name {
        "cem:scope" => SchemaTokenKind::AnonymousScopeStart,
        "cem:expr" => SchemaTokenKind::NodeStart {
            name: "$".to_owned(),
        },
        _ => SchemaTokenKind::NodeStart {
            name: name.to_owned(),
        },
    }
}

fn parity_end_name(name: &str) -> Option<String> {
    match name {
        "cem:scope" => None,
        "cem:expr" => Some("$".to_owned()),
        _ => Some(name.to_owned()),
    }
}

impl SchemaTokenizer for HtmlTokenizer {
    fn profile(&self) -> TokenizerProfile {
        TokenizerProfile::Html
    }

    fn next_token(&mut self) -> Option<SchemaToken> {
        self.pending.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::BytesSource;

    fn tokenize(input: &str) -> (Vec<SchemaToken>, Vec<Diagnostic>) {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let mut t = HtmlTokenizer::from_source(src);
        let mut out = Vec::new();
        while let Some(tok) = t.next_token() {
            out.push(tok);
        }
        let diags = t.take_diagnostics();
        (out, diags)
    }

    fn non_trivia(tokens: &[SchemaToken]) -> Vec<&SchemaToken> {
        tokens
            .iter()
            .filter(|t| !matches!(t.kind, SchemaTokenKind::Trivia(_)))
            .collect()
    }

    #[test]
    fn simple_element_emits_open_text_close() {
        let (tokens, diags) = tokenize("<p>Hello</p>");
        assert!(diags.is_empty(), "{:?}", diags);
        let nt = non_trivia(&tokens);
        assert!(matches!(
            &nt[0].kind,
            SchemaTokenKind::NodeStart { name } if name == "p"
        ));
        assert!(matches!(
            &nt[1].kind,
            SchemaTokenKind::Text(t) if t == "Hello"
        ));
        assert!(matches!(
            &nt[2].kind,
            SchemaTokenKind::NodeEnd { name: Some(n) } if n == "p"
        ));
    }

    #[test]
    fn tag_names_are_lowercased() {
        let (tokens, _) = tokenize("<P><DIV></DIV></P>");
        let names: Vec<&str> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                SchemaTokenKind::NodeStart { name } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["p", "div"]);
    }

    #[test]
    fn double_quoted_attribute() {
        let (tokens, diags) = tokenize(r#"<a href="/dashboard">link</a>"#);
        assert!(diags.is_empty(), "{:?}", diags);
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { name, value, .. } = &attr.kind {
            assert_eq!(name, "href");
            assert_eq!(value.as_deref(), Some("/dashboard"));
        }
    }

    #[test]
    fn single_quoted_attribute() {
        let (tokens, diags) = tokenize(r#"<a href='/x'>x</a>"#);
        assert!(diags.is_empty(), "{:?}", diags);
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { value, .. } = &attr.kind {
            assert_eq!(value.as_deref(), Some("/x"));
        }
    }

    #[test]
    fn unquoted_attribute_value() {
        let (tokens, diags) = tokenize("<input type=email>");
        assert!(diags.is_empty(), "{:?}", diags);
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { name, value, .. } = &attr.kind {
            assert_eq!(name, "type");
            assert_eq!(value.as_deref(), Some("email"));
        }
    }

    #[test]
    fn boolean_attribute_has_no_value() {
        let (tokens, _) = tokenize("<input required>");
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { name, value, .. } = &attr.kind {
            assert_eq!(name, "required");
            assert!(value.is_none());
        }
    }

    #[test]
    fn void_element_emits_synthetic_end_tag() {
        let (tokens, _) = tokenize("<input>");
        let mut iter = tokens.iter();
        assert!(matches!(
            iter.next().unwrap().kind,
            SchemaTokenKind::NodeStart { .. }
        ));
        assert!(matches!(
            iter.next().unwrap().kind,
            SchemaTokenKind::NodeEnd { .. }
        ));
    }

    #[test]
    fn self_closing_explicit_slash() {
        let (tokens, _) = tokenize("<br/>");
        let kinds: Vec<&str> = tokens
            .iter()
            .map(|t| match &t.kind {
                SchemaTokenKind::NodeStart { .. } => "open",
                SchemaTokenKind::NodeEnd { .. } => "close",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["open", "close"]);
    }

    #[test]
    fn comments_are_preserved() {
        let (tokens, diags) = tokenize("<!--hi-->");
        assert!(diags.is_empty());
        let c = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Comment(_)))
            .unwrap();
        if let SchemaTokenKind::Comment(d) = &c.kind {
            assert_eq!(d, "hi");
        }
    }

    #[test]
    fn doctype_emits_processing_instruction() {
        let (tokens, diags) = tokenize("<!doctype html>");
        assert!(diags.is_empty());
        let pi = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::ProcessingInstruction { .. }))
            .unwrap();
        if let SchemaTokenKind::ProcessingInstruction { target, data } = &pi.kind {
            assert_eq!(target, "DOCTYPE");
            assert_eq!(data, "html");
        }
    }

    #[test]
    fn attribute_with_namespace_prefix_is_preserved() {
        let (tokens, _) = tokenize(r#"<button cem:action="primary">Save</button>"#);
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { name, value, .. } = &attr.kind {
            assert_eq!(name, "cem:action");
            assert_eq!(value.as_deref(), Some("primary"));
        }
    }

    #[test]
    fn byte_ranges_are_absolute_and_contiguous() {
        let input = "<p>Hi</p>";
        let (tokens, _) = tokenize(input);
        let first = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::NodeStart { .. }))
            .unwrap();
        assert_eq!(first.byte_range.start, 0);
        let last_close = tokens
            .iter()
            .rev()
            .find(|t| matches!(t.kind, SchemaTokenKind::NodeEnd { .. }))
            .unwrap();
        assert_eq!(last_close.byte_range.end(), input.len() as u64);
    }

    #[test]
    fn source_map_carries_html_tokenizer_frame() {
        let (tokens, _) = tokenize("<p>x</p>");
        let first = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::NodeStart { .. }))
            .unwrap();
        assert!(matches!(
            first.source_map.frames[0].transform,
            TransformKind::HtmlTokenizer
        ));
    }

    #[test]
    fn login_parity_fixture_tokenizes_clean() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/semantic/login.html");
        let input = std::fs::read_to_string(&path).unwrap();
        let (tokens, diags) = tokenize(&input);
        let hard: Vec<_> = diags
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .collect();
        assert!(hard.is_empty(), "{hard:?}");
        // Expect at least a DOCTYPE, an <html>, a <main>, a <form>,
        // a <button>.
        let names: Vec<&str> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                SchemaTokenKind::NodeStart { name } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        for expected in ["html", "main", "form", "button", "input"] {
            assert!(
                names.contains(&expected),
                "expected <{expected}> in token stream"
            );
        }
        let pis: Vec<&str> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                SchemaTokenKind::ProcessingInstruction { target, .. } => Some(target.as_str()),
                _ => None,
            })
            .collect();
        assert!(pis.contains(&"DOCTYPE"));
    }

    #[test]
    fn all_parity_fixtures_tokenize_without_hard_violations() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/semantic");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("html") {
                continue;
            }
            let input = std::fs::read_to_string(&path).unwrap();
            let (tokens, diags) = tokenize(&input);
            let hard: Vec<_> = diags
                .iter()
                .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
                .collect();
            assert!(
                hard.is_empty(),
                "fixture `{}` produced hard violations: {hard:?}",
                path.display()
            );
            assert!(!tokens.is_empty());
            checked += 1;
        }
        assert!(checked >= 5);
    }
}
