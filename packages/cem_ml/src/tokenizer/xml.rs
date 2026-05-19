//! XML 1.0 parity tokenizer profile.
//!
//! Tier A covers the XML subset needed for parity fixtures: elements,
//! end tags, quoted attributes, self-closing tags, comments, CDATA,
//! processing instructions, and declarations. The tokenizer emits the
//! same profile-agnostic `SchemaToken` shape as the CEM and HTML
//! tokenizers so the shared event normalizer, schema machine, AST
//! builder, validation, and transform layers can run unchanged.

use crate::diagnostics::{Diagnostic, Severity};
use crate::source::decode::{DecodeConfig, Utf8Decoder};
use crate::source::{ByteRange, ByteSource, EncodingDecoder, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer, TokenizerProfile};
use std::collections::VecDeque;

pub struct XmlTokenizer {
    source_id: SourceId,
    scalars: Vec<(char, ByteRange)>,
    cursor: usize,
    pending: VecDeque<SchemaToken>,
    diagnostics: Vec<Diagnostic>,
    base_source_map: SourceMapStack,
    end_offset: u64,
}

impl XmlTokenizer {
    pub fn from_source<S: ByteSource>(source: S) -> Self {
        let mut decoder = Utf8Decoder::with_config(
            source,
            DecodeConfig {
                default_encoding: None,
                strict_xml_chars: true,
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
                transform: TransformKind::XmlTokenizer,
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

    fn scan_document(&mut self) {
        while !self.at_end() {
            self.scan_data();
        }
    }

    fn scan_data(&mut self) {
        let start = self.cursor;
        while let Some(c) = self.peek() {
            if c == '<' {
                break;
            }
            self.advance();
        }
        if self.cursor > start {
            let range = self.range_from(start, self.cursor);
            let data: String = self.scalars[start..self.cursor]
                .iter()
                .map(|(c, _)| *c)
                .collect();
            if data.trim().is_empty() {
                self.emit(SchemaTokenKind::Trivia(data), range);
            } else {
                self.emit(SchemaTokenKind::Text(data), range);
            }
        }
        if self.peek() == Some('<') {
            self.scan_markup();
        }
    }

    fn scan_markup(&mut self) {
        debug_assert_eq!(self.peek(), Some('<'));
        let open_start = self.cursor;
        self.advance();
        match self.peek() {
            Some('/') => {
                self.advance();
                self.scan_end_tag(open_start);
            }
            Some('?') => {
                self.advance();
                self.scan_processing_instruction(open_start);
            }
            Some('!') => {
                self.advance();
                self.scan_declaration(open_start);
            }
            Some(c) if is_xml_name_start(c) => self.scan_start_tag(open_start),
            _ => {
                self.diag(
                    "cem.xml.invalid_markup",
                    Severity::Error,
                    "expected an XML name, end tag, processing instruction, comment, CDATA, or declaration after `<`",
                    self.current_offset(),
                );
                let range = self.range_from(open_start, self.cursor);
                self.emit(
                    SchemaTokenKind::Error {
                        code: "cem.xml.invalid_markup".to_owned(),
                    },
                    range,
                );
            }
        }
    }

    fn scan_start_tag(&mut self, open_start: usize) {
        let name_start = self.cursor;
        self.consume_xml_name();
        let name: String = self.scalars[name_start..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let head_range = self.range_from(open_start, self.cursor);
        self.emit(
            SchemaTokenKind::NodeStart { name: name.clone() },
            head_range,
        );

        loop {
            self.skip_xml_whitespace();
            match self.peek() {
                None => {
                    self.diag(
                        "cem.xml.unterminated_tag",
                        Severity::Error,
                        "start tag did not close before EOF",
                        self.current_offset(),
                    );
                    return;
                }
                Some('>') => {
                    self.advance();
                    return;
                }
                Some('/') if self.peek_at(1) == Some('>') => {
                    let close_start = self.cursor;
                    self.advance();
                    self.advance();
                    let close_range = self.range_from(close_start, self.cursor);
                    self.emit(
                        SchemaTokenKind::NodeEnd {
                            name: Some(name.clone()),
                        },
                        close_range,
                    );
                    return;
                }
                Some(c) if is_xml_name_start(c) => self.scan_attribute(),
                Some(_) => {
                    self.diag(
                        "cem.xml.invalid_attribute",
                        Severity::Error,
                        "expected an XML attribute name or tag close",
                        self.current_offset(),
                    );
                    self.advance();
                }
            }
        }
    }

    fn scan_attribute(&mut self) {
        let attr_start = self.cursor;
        let name_start = self.cursor;
        self.consume_xml_name();
        let name_end = self.cursor;
        let name: String = self.scalars[name_start..name_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let name_range = self.range_from(name_start, name_end);

        self.skip_xml_whitespace();
        let mut value = None;
        let mut value_range = None;
        if self.peek() == Some('=') {
            self.advance();
            self.skip_xml_whitespace();
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
                    self.diag(
                        "cem.xml.unquoted_attribute_value",
                        Severity::Error,
                        "XML attribute values must be quoted",
                        self.current_offset(),
                    );
                }
            }
        } else {
            self.diag(
                "cem.xml.missing_attribute_value",
                Severity::Error,
                "XML attributes must have an explicit value",
                self.current_offset(),
            );
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
        self.advance();
        let body_start = self.cursor;
        while let Some(c) = self.peek() {
            if c == quote {
                break;
            }
            if c == '<' {
                self.diag(
                    "cem.xml.invalid_attribute_value",
                    Severity::Error,
                    "XML attribute values cannot contain raw `<`",
                    self.current_offset(),
                );
            }
            self.advance();
        }
        let body_end = self.cursor;
        let data: String = self.scalars[body_start..body_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        if self.peek() == Some(quote) {
            self.advance();
        } else {
            self.diag(
                "cem.xml.unterminated_attribute_value",
                Severity::Error,
                "attribute value is missing its closing quote",
                self.current_offset(),
            );
        }
        let range = self.range_from(start, self.cursor);
        (data, range)
    }

    fn scan_end_tag(&mut self, open_start: usize) {
        self.skip_xml_whitespace();
        let name_start = self.cursor;
        if !matches!(self.peek(), Some(c) if is_xml_name_start(c)) {
            self.diag(
                "cem.xml.invalid_end_tag",
                Severity::Error,
                "end tag is missing an XML name",
                self.current_offset(),
            );
        } else {
            self.consume_xml_name();
        }
        let name: String = self.scalars[name_start..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        self.skip_xml_whitespace();
        if self.peek() == Some('>') {
            self.advance();
        } else {
            self.diag(
                "cem.xml.unterminated_end_tag",
                Severity::Error,
                "end tag is missing `>`",
                self.current_offset(),
            );
            while let Some(c) = self.peek() {
                if c == '>' {
                    self.advance();
                    break;
                }
                self.advance();
            }
        }
        let range = self.range_from(open_start, self.cursor);
        self.emit(SchemaTokenKind::NodeEnd { name: Some(name) }, range);
    }

    fn scan_declaration(&mut self, open_start: usize) {
        if self.peek() == Some('-') && self.peek_at(1) == Some('-') {
            self.advance();
            self.advance();
            self.scan_comment(open_start);
            return;
        }
        if self.match_exact("[CDATA[") {
            self.scan_cdata(open_start);
            return;
        }
        if self.match_exact("DOCTYPE") {
            self.scan_doctype(open_start);
            return;
        }
        self.diag(
            "cem.xml.unsupported_declaration",
            Severity::Error,
            "unsupported XML declaration after `<!`",
            self.current_offset(),
        );
        while let Some(c) = self.peek() {
            self.advance();
            if c == '>' {
                break;
            }
        }
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
                        "cem.xml.unterminated_comment",
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

    fn scan_cdata(&mut self, open_start: usize) {
        let body_start = self.cursor;
        loop {
            match (self.peek(), self.peek_at(1), self.peek_at(2)) {
                (Some(']'), Some(']'), Some('>')) => {
                    let body_end = self.cursor;
                    self.advance();
                    self.advance();
                    self.advance();
                    let data: String = self.scalars[body_start..body_end]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    let range = self.range_from(open_start, self.cursor);
                    self.emit(SchemaTokenKind::RichContent { data }, range);
                    return;
                }
                (None, _, _) => {
                    self.diag(
                        "cem.xml.unterminated_cdata",
                        Severity::Error,
                        "CDATA section is missing its closing `]]>`",
                        self.current_offset(),
                    );
                    let data: String = self.scalars[body_start..self.cursor]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    let range = self.range_from(open_start, self.cursor);
                    self.emit(SchemaTokenKind::RichContent { data }, range);
                    return;
                }
                _ => self.advance(),
            }
        }
    }

    fn scan_doctype(&mut self, open_start: usize) {
        self.skip_xml_whitespace();
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
        } else {
            self.diag(
                "cem.xml.unterminated_doctype",
                Severity::Error,
                "DOCTYPE declaration is missing `>`",
                self.current_offset(),
            );
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

    fn scan_processing_instruction(&mut self, open_start: usize) {
        let target_start = self.cursor;
        if !matches!(self.peek(), Some(c) if is_xml_name_start(c)) {
            self.diag(
                "cem.xml.invalid_processing_instruction",
                Severity::Error,
                "processing instruction is missing a target name",
                self.current_offset(),
            );
        } else {
            self.consume_xml_name();
        }
        let target: String = self.scalars[target_start..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        self.skip_xml_whitespace();
        let data_start = self.cursor;
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some('?'), Some('>')) => {
                    let data_end = self.cursor;
                    self.advance();
                    self.advance();
                    let data: String = self.scalars[data_start..data_end]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    let range = self.range_from(open_start, self.cursor);
                    self.emit(
                        SchemaTokenKind::ProcessingInstruction {
                            target,
                            data: data.trim().to_owned(),
                        },
                        range,
                    );
                    return;
                }
                (None, _) => {
                    self.diag(
                        "cem.xml.unterminated_processing_instruction",
                        Severity::Error,
                        "processing instruction is missing its closing `?>`",
                        self.current_offset(),
                    );
                    let data: String = self.scalars[data_start..self.cursor]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    let range = self.range_from(open_start, self.cursor);
                    self.emit(
                        SchemaTokenKind::ProcessingInstruction {
                            target,
                            data: data.trim().to_owned(),
                        },
                        range,
                    );
                    return;
                }
                _ => self.advance(),
            }
        }
    }

    fn consume_xml_name(&mut self) {
        if !matches!(self.peek(), Some(c) if is_xml_name_start(c)) {
            return;
        }
        self.advance();
        while let Some(c) = self.peek() {
            if is_xml_name_char(c) {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn match_exact(&mut self, s: &str) -> bool {
        let mut chars = s.chars();
        let len = s.chars().count();
        if self.cursor + len > self.scalars.len() {
            return false;
        }
        for i in 0..len {
            let expected = chars.next().unwrap();
            if self.scalars[self.cursor + i].0 != expected {
                return false;
            }
        }
        self.cursor += len;
        true
    }

    fn skip_xml_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if matches!(c, ' ' | '\t' | '\n' | '\r') {
                self.advance();
            } else {
                break;
            }
        }
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
            profile: TokenizerProfile::Xml,
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

fn is_xml_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '_' | ':')
}

fn is_xml_name_char(c: char) -> bool {
    is_xml_name_start(c) || c.is_ascii_digit() || matches!(c, '-' | '.')
}

impl SchemaTokenizer for XmlTokenizer {
    fn profile(&self) -> TokenizerProfile {
        TokenizerProfile::Xml
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
        let mut t = XmlTokenizer::from_source(src);
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
        assert!(diags.is_empty(), "{diags:?}");
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
    fn tag_and_attribute_names_preserve_case_and_prefixes() {
        let (tokens, diags) = tokenize(r#"<Svg:Path cem:Action="Primary"/>"#);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(tokens.iter().any(|t| {
            matches!(&t.kind, SchemaTokenKind::NodeStart { name } if name == "Svg:Path")
        }));
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { name, value, .. } = &attr.kind {
            assert_eq!(name, "cem:Action");
            assert_eq!(value.as_deref(), Some("Primary"));
        }
    }

    #[test]
    fn self_closing_tag_emits_matching_close() {
        let (tokens, diags) = tokenize(r#"<input required="required"/>"#);
        assert!(diags.is_empty(), "{diags:?}");
        let kinds: Vec<&str> = tokens
            .iter()
            .map(|t| match &t.kind {
                SchemaTokenKind::NodeStart { .. } => "open",
                SchemaTokenKind::Attribute { .. } => "attr",
                SchemaTokenKind::NodeEnd { .. } => "close",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["open", "attr", "close"]);
    }

    #[test]
    fn comments_cdata_and_processing_instructions_are_preserved() {
        let input = r#"<?xml version="1.0"?><!--hi--><root><![CDATA[a < b]]></root>"#;
        let (tokens, diags) = tokenize(input);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(tokens.iter().any(|t| {
            matches!(&t.kind, SchemaTokenKind::ProcessingInstruction { target, data }
                if target == "xml" && data == "version=\"1.0\"")
        }));
        assert!(tokens
            .iter()
            .any(|t| matches!(&t.kind, SchemaTokenKind::Comment(data) if data == "hi")));
        assert!(tokens.iter().any(|t| {
            matches!(&t.kind, SchemaTokenKind::RichContent { data } if data == "a < b")
        }));
    }

    #[test]
    fn unquoted_attribute_value_is_a_hard_diagnostic() {
        let (_tokens, diags) = tokenize("<input required=required/>");
        assert!(diags
            .iter()
            .any(|d| d.code == "cem.xml.unquoted_attribute_value"));
    }

    #[test]
    fn byte_ranges_are_absolute_and_source_map_is_xml_rooted() {
        let input = "<p>Hi</p>";
        let (tokens, diags) = tokenize(input);
        assert!(diags.is_empty(), "{diags:?}");
        let first = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::NodeStart { .. }))
            .unwrap();
        assert_eq!(first.byte_range.start, 0);
        assert!(matches!(
            first.source_map.frames[0].transform,
            TransformKind::XmlTokenizer
        ));
        let last_close = tokens
            .iter()
            .rev()
            .find(|t| matches!(t.kind, SchemaTokenKind::NodeEnd { .. }))
            .unwrap();
        assert_eq!(last_close.byte_range.end(), input.len() as u64);
    }

    #[test]
    fn namespace_rebinding_xml_fixture_tokenizes_clean() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/cem-ml/namespace-rebinding/default-html-svg-html.xml");
        let input = std::fs::read_to_string(&path).unwrap();
        let (tokens, diags) = tokenize(&input);
        let hard: Vec<_> = diags
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .collect();
        assert!(hard.is_empty(), "{hard:?}");
        for expected in ["main", "svg", "path", "form", "input"] {
            assert!(
                tokens.iter().any(|t| {
                    matches!(&t.kind, SchemaTokenKind::NodeStart { name } if name == expected)
                }),
                "expected <{expected}> in token stream"
            );
        }
    }
}
