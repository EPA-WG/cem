//! Canonical CEM-ML curly tokenizer.
//!
//! Tier A implementation per `cem-ml-syntax.md` and
//! `cem-ml-stack-design-impl.md` §3.2. Eagerly decodes the full input into a
//! scalar buffer, then walks it producing `SchemaToken`s with absolute byte
//! ranges and origin-first source-map frames. A future streaming refinement
//! lands in Phase 11 once the schema machine + handoff stack are ready.
//!
//! Supported lexical surface:
//!
//! - Document-level directives: `@doc`, `@ns`, `@default`, `@schema`.
//! - Named nodes `{name @attr=value | content}` with explicit `|`.
//! - Relaxed content boundary (first non-attribute token starts content).
//! - Anonymous typed scopes `{@type=... | ...}`.
//! - Expression nodes `{$ ...}` and `{$ | ...}` (body emitted as a single
//!   `ExpressionNode` token; cem-ql lowering happens in a later layer).
//! - Line comments `// ...` and block comments `/* ... */`.
//! - Rich-content enclosures using triple backticks ```` ``` ```` (body
//!   emitted as a single `RichContent` token, brace contents preserved).
//! - Attribute values: bare identifier/number, `"..."`, `'...'`, and
//!   cem-ql AVT spans `{...}` (template-aware: emitted as the literal span;
//!   AVT parsing happens in a later layer).
//! - Bare `{...}` text interpolation in content is rejected with
//!   `cem.tokenizer.bare_brace_text` and skipped.

use crate::diagnostics::{Diagnostic, Severity};
use crate::source::decode::{DecodeConfig, Utf8Decoder};
use crate::source::{ByteRange, ByteSource, EncodingDecoder, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer, TokenizerProfile};
use std::collections::VecDeque;

pub struct CemTokenizer {
    source_id: SourceId,
    scalars: Vec<(char, ByteRange)>,
    cursor: usize,
    pending: VecDeque<SchemaToken>,
    diagnostics: Vec<Diagnostic>,
    /// Source-map stack pushed onto every token. Tier A: one frame per
    /// tokenizer instance identifying the CEM profile + originating source.
    base_source_map: SourceMapStack,
    end_offset: u64,
}

impl CemTokenizer {
    /// Build a tokenizer from a `ByteSource`, decoding all bytes eagerly.
    /// Decode diagnostics are surfaced via [`take_diagnostics`].
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
        let end_offset = scalars
            .last()
            .map(|(_, r)| r.end())
            .unwrap_or(0);
        let base_source_map = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id,
                span: FrameSpan::Single(ByteRange::new(0, end_offset as u32)),
                transform: TransformKind::CemTokenizer,
            }],
        };
        let mut tokenizer = Self {
            source_id,
            scalars,
            cursor: 0,
            pending: VecDeque::new(),
            diagnostics,
            base_source_map,
            end_offset,
        };
        tokenizer.scan_document();
        tokenizer
    }

    /// Drain accumulated diagnostics (decoder + tokenizer).
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

    fn current_offset(&self) -> u64 {
        self.scalars
            .get(self.cursor)
            .map(|(_, r)| r.start)
            .unwrap_or(self.end_offset)
    }

    fn advance(&mut self) -> Option<(char, ByteRange)> {
        let v = self.scalars.get(self.cursor).copied()?;
        self.cursor += 1;
        Some(v)
    }

    fn skip_trivia(&mut self) {
        loop {
            self.flush_whitespace_trivia();
            match (self.peek(), self.peek_at(1)) {
                (Some('/'), Some('/')) => self.consume_line_comment(),
                (Some('/'), Some('*')) => self.consume_block_comment(),
                _ => break,
            }
        }
    }

    fn flush_whitespace_trivia(&mut self) {
        let start = self.cursor;
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.cursor += 1;
            } else {
                break;
            }
        }
        if self.cursor > start {
            let range = self.range_from(start, self.cursor);
            let data: String = self.scalars[start..self.cursor]
                .iter()
                .map(|(c, _)| *c)
                .collect();
            self.emit(SchemaTokenKind::Trivia(data), range);
        }
    }

    fn skip_horiz_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.cursor += 1;
            } else {
                break;
            }
        }
    }

    fn consume_line_comment(&mut self) {
        let start = self.cursor;
        self.cursor += 2; // '//'
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.cursor += 1;
        }
        let range = self.range_from(start, self.cursor);
        let data: String = self.scalars[start + 2..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        self.emit(SchemaTokenKind::Comment(data), range);
    }

    fn consume_block_comment(&mut self) {
        let start = self.cursor;
        self.cursor += 2; // '/*'
        let body_start = self.cursor;
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some('*'), Some('/')) => {
                    let body_end = self.cursor;
                    self.cursor += 2;
                    let range = self.range_from(start, self.cursor);
                    let data: String = self.scalars[body_start..body_end]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    self.emit(SchemaTokenKind::Comment(data), range);
                    return;
                }
                (None, _) => {
                    let range = self.range_from(start, self.cursor);
                    self.diagnostic(
                        "cem.tokenizer.unterminated_block_comment",
                        Severity::Error,
                        "block comment is not terminated".into(),
                        range.start,
                    );
                    let data: String = self.scalars[body_start..self.cursor]
                        .iter()
                        .map(|(c, _)| *c)
                        .collect();
                    self.emit(SchemaTokenKind::Comment(data), range);
                    return;
                }
                _ => {
                    self.cursor += 1;
                }
            }
        }
    }

    fn scan_document(&mut self) {
        loop {
            self.skip_trivia();
            match self.peek() {
                None => break,
                Some('@') => self.scan_directive(),
                Some('{') => self.scan_node(),
                Some('`') if self.is_rich_open() => self.scan_rich_content(),
                Some(_) => self.scan_top_text(),
            }
        }
    }

    fn scan_top_text(&mut self) {
        // Fragments may carry text at top level. Collect until next structural
        // sigil.
        let start = self.cursor;
        while let Some(c) = self.peek() {
            if c == '{' || c == '@' || c == '`' {
                break;
            }
            self.cursor += 1;
        }
        if self.cursor > start {
            let range = self.range_from(start, self.cursor);
            let data: String = self.scalars[start..self.cursor]
                .iter()
                .map(|(c, _)| *c)
                .collect();
            self.emit(SchemaTokenKind::Text(data), range);
        } else {
            // Defensive: advance one to avoid an infinite loop on an
            // unexpected character at top level.
            self.advance();
        }
    }

    fn scan_directive(&mut self) {
        let start = self.cursor;
        // Consume '@'
        self.cursor += 1;
        let name_start = self.cursor;
        while let Some(c) = self.peek() {
            if is_name_continue(c) || c == '-' {
                self.cursor += 1;
            } else {
                break;
            }
        }
        let name: String = self.scalars[name_start..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        // Collect the rest of the directive body up to end-of-line or top-level
        // sigil. Directives terminate at newline in canonical form.
        let body_start = self.cursor;
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.cursor += 1;
        }
        let body: String = self.scalars[body_start..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let range = self.range_from(start, self.cursor);
        self.emit(
            SchemaTokenKind::Directive {
                name,
                data: body.trim().to_owned(),
            },
            range,
        );
    }

    fn scan_node(&mut self) {
        let open_start = self.cursor;
        self.cursor += 1; // consume '{'
        self.skip_horiz_ws();
        match self.peek() {
            Some('@') => {
                // Anonymous typed scope.
                let range = self.range_from(open_start, self.cursor);
                self.emit(SchemaTokenKind::AnonymousScopeStart, range);
                self.scan_attributes();
                self.scan_content_until_close();
            }
            Some('$') => {
                // Expression node.
                self.cursor += 1;
                let head_range = self.range_from(open_start, self.cursor);
                self.emit(
                    SchemaTokenKind::NodeStart {
                        name: "$".to_owned(),
                    },
                    head_range,
                );
                self.scan_expression_body();
            }
            Some(c) if is_name_start(c) => {
                let name_start = self.cursor;
                while let Some(ch) = self.peek() {
                    if is_qname_continue(ch) {
                        self.cursor += 1;
                    } else {
                        break;
                    }
                }
                let name: String = self.scalars[name_start..self.cursor]
                    .iter()
                    .map(|(c, _)| *c)
                    .collect();
                let head_range = self.range_from(open_start, self.cursor);
                self.emit(SchemaTokenKind::NodeStart { name }, head_range);
                self.scan_attributes();
                self.scan_content_until_close();
            }
            Some('}') => {
                // Empty `{}` — emit start/end pair on the same offset.
                let head_range = self.range_from(open_start, self.cursor);
                self.emit(
                    SchemaTokenKind::NodeStart {
                        name: String::new(),
                    },
                    head_range,
                );
                let close_start = self.cursor;
                self.cursor += 1;
                let close_range = self.range_from(close_start, self.cursor);
                self.emit(SchemaTokenKind::NodeEnd { name: None }, close_range);
            }
            Some(_) | None => {
                // Bare `{...}` text interpolation in content is rejected by
                // `scan_content_until_close`; at structural scan-node entry we
                // emit an error and skip to the next `}`.
                let range = self.range_from(open_start, self.cursor);
                self.diagnostic(
                    "cem.tokenizer.bare_brace_text",
                    Severity::Error,
                    "bare `{...}` text interpolation is not permitted".into(),
                    range.start,
                );
                self.emit(
                    SchemaTokenKind::Error {
                        code: "cem.tokenizer.bare_brace_text".to_owned(),
                    },
                    range,
                );
                self.skip_to_close();
            }
        }
    }

    fn scan_attributes(&mut self) {
        loop {
            self.skip_horiz_ws();
            if self.peek() == Some('\n') {
                self.advance();
                continue;
            }
            match self.peek() {
                Some('@') => self.scan_attribute(),
                _ => break,
            }
        }
    }

    fn scan_attribute(&mut self) {
        debug_assert_eq!(self.peek(), Some('@'));
        let attr_start = self.cursor;
        self.cursor += 1; // '@'
        let name_start = self.cursor;
        while let Some(c) = self.peek() {
            if is_qname_continue(c) {
                self.cursor += 1;
            } else {
                break;
            }
        }
        let name_end = self.cursor;
        let name: String = self.scalars[name_start..name_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let name_range = self.range_from(name_start, name_end);
        let mut value: Option<String> = None;
        let mut value_range: Option<ByteRange> = None;
        self.skip_horiz_ws();
        if self.peek() == Some('=') {
            self.cursor += 1;
            self.skip_horiz_ws();
            let (v, r) = self.scan_attribute_value();
            value = Some(v);
            value_range = Some(r);
        }
        let total_range = ByteRange::new(
            name_range.start.saturating_sub(1), // include leading '@'
            (self.scalars[attr_start..self.cursor]
                .iter()
                .map(|(_, r)| r.len as u64)
                .sum::<u64>()) as u32,
        );
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

    fn scan_attribute_value(&mut self) -> (String, ByteRange) {
        match self.peek() {
            Some('"') => self.scan_quoted_string('"'),
            Some('\'') => self.scan_quoted_string('\''),
            Some('{') => self.scan_avt_span(),
            _ => self.scan_bare_value(),
        }
    }

    fn scan_quoted_string(&mut self, quote: char) -> (String, ByteRange) {
        let start = self.cursor;
        self.cursor += 1; // opening quote
        let body_start = self.cursor;
        while let Some(c) = self.peek() {
            if c == quote {
                break;
            }
            self.cursor += 1;
        }
        let body_end = self.cursor;
        let mut data: String = self.scalars[body_start..body_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        if self.peek() == Some(quote) {
            self.cursor += 1;
        } else {
            self.diagnostic(
                "cem.tokenizer.unterminated_string",
                Severity::Error,
                "unterminated quoted attribute value".into(),
                self.current_offset(),
            );
            data.push('\u{FFFD}');
        }
        let range = self.range_from(start, self.cursor);
        (data, range)
    }

    fn scan_avt_span(&mut self) -> (String, ByteRange) {
        // Recognize the brace span and pass the body through verbatim. The
        // cem-ql layer parses the body later.
        let start = self.cursor;
        self.cursor += 1; // '{'
        let body_start = self.cursor;
        let mut depth = 1u32;
        while let Some(c) = self.peek() {
            match c {
                '{' => {
                    depth += 1;
                    self.cursor += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    self.cursor += 1;
                }
                _ => {
                    self.cursor += 1;
                }
            }
        }
        let body_end = self.cursor;
        let body: String = self.scalars[body_start..body_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        if self.peek() == Some('}') {
            self.cursor += 1;
        } else {
            self.diagnostic(
                "cem.tokenizer.unterminated_avt_span",
                Severity::Error,
                "unterminated `{...}` cem-ql span in attribute value".into(),
                self.current_offset(),
            );
        }
        let range = self.range_from(start, self.cursor);
        // Wrap with braces so consumers can see this is a span, not a literal.
        (format!("{{{body}}}"), range)
    }

    fn scan_bare_value(&mut self) -> (String, ByteRange) {
        let start = self.cursor;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == '}' || c == '|' {
                break;
            }
            self.cursor += 1;
        }
        let data: String = self.scalars[start..self.cursor]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let range = self.range_from(start, self.cursor);
        (data, range)
    }

    fn scan_content_until_close(&mut self) {
        // Optional explicit content boundary `|` (or Unicode `▷`).
        self.skip_horiz_ws();
        if matches!(self.peek(), Some('|') | Some('▷')) {
            let start = self.cursor;
            self.cursor += 1;
            let range = self.range_from(start, self.cursor);
            self.emit(SchemaTokenKind::Trivia("|".into()), range);
        }
        // Content loop until `}` or EOF.
        loop {
            // Eagerly flush whitespace/trivia between content tokens.
            self.flush_whitespace_trivia();
            match self.peek() {
                None => {
                    self.diagnostic(
                        "cem.tokenizer.unterminated_node",
                        Severity::Error,
                        "node is not closed with `}`".into(),
                        self.current_offset(),
                    );
                    return;
                }
                Some('}') => {
                    let close_start = self.cursor;
                    self.cursor += 1;
                    let range = self.range_from(close_start, self.cursor);
                    self.emit(SchemaTokenKind::NodeEnd { name: None }, range);
                    return;
                }
                Some('{') => {
                    // Inspect the next non-space character to decide.
                    let peek_idx = self.cursor + 1;
                    let mut probe = peek_idx;
                    while probe < self.scalars.len()
                        && (self.scalars[probe].0 == ' ' || self.scalars[probe].0 == '\t')
                    {
                        probe += 1;
                    }
                    let next = self.scalars.get(probe).map(|(c, _)| *c);
                    if matches!(next, Some('@') | Some('$')) || matches!(next, Some(c) if is_name_start(c)) {
                        self.scan_node();
                    } else if next == Some('}') {
                        // `{}` empty node.
                        self.scan_node();
                    } else {
                        // Bare `{...}` text interpolation is rejected.
                        let start = self.cursor;
                        self.cursor += 1;
                        let mut depth = 1u32;
                        while let Some(c) = self.peek() {
                            match c {
                                '{' => {
                                    depth += 1;
                                    self.cursor += 1;
                                }
                                '}' => {
                                    depth -= 1;
                                    self.cursor += 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {
                                    self.cursor += 1;
                                }
                            }
                        }
                        let range = self.range_from(start, self.cursor);
                        self.diagnostic(
                            "cem.tokenizer.bare_brace_text",
                            Severity::Error,
                            "bare `{...}` text interpolation in content is not permitted; \
                             use `{$ ...}` for an expression node"
                                .into(),
                            range.start,
                        );
                        self.emit(
                            SchemaTokenKind::Error {
                                code: "cem.tokenizer.bare_brace_text".to_owned(),
                            },
                            range,
                        );
                    }
                }
                Some('`') if self.is_rich_open() => {
                    self.scan_rich_content();
                }
                Some('/') if self.peek_at(1) == Some('/') => self.consume_line_comment(),
                Some('/') if self.peek_at(1) == Some('*') => self.consume_block_comment(),
                _ => self.scan_content_text(),
            }
        }
    }

    fn scan_content_text(&mut self) {
        let start = self.cursor;
        while let Some(c) = self.peek() {
            if c == '{' || c == '}' || c == '`' {
                break;
            }
            if c == '/' && (self.peek_at(1) == Some('/') || self.peek_at(1) == Some('*')) {
                break;
            }
            self.cursor += 1;
        }
        if self.cursor > start {
            let range = self.range_from(start, self.cursor);
            let data: String = self.scalars[start..self.cursor]
                .iter()
                .map(|(c, _)| *c)
                .collect();
            self.emit(SchemaTokenKind::Text(data), range);
        } else {
            // Safety net: avoid infinite loop on unexpected input.
            self.advance();
        }
    }

    fn scan_expression_body(&mut self) {
        // After `{$`, optionally skip `|`, then collect verbatim until matching `}`.
        self.skip_horiz_ws();
        if self.peek() == Some('|') {
            self.cursor += 1;
        }
        let start = self.cursor;
        let mut depth = 1u32;
        while let Some(c) = self.peek() {
            match c {
                '{' => {
                    depth += 1;
                    self.cursor += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    self.cursor += 1;
                }
                _ => {
                    self.cursor += 1;
                }
            }
        }
        let body_end = self.cursor;
        let body: String = self.scalars[start..body_end]
            .iter()
            .map(|(c, _)| *c)
            .collect();
        let body_range = self.range_from(start, body_end);
        self.emit(
            SchemaTokenKind::ExpressionNode(body.trim().to_owned()),
            body_range,
        );
        if self.peek() == Some('}') {
            let close_start = self.cursor;
            self.cursor += 1;
            let range = self.range_from(close_start, self.cursor);
            self.emit(
                SchemaTokenKind::NodeEnd {
                    name: Some("$".into()),
                },
                range,
            );
        } else {
            self.diagnostic(
                "cem.tokenizer.unterminated_expression",
                Severity::Error,
                "expression node `{$ ...}` is not closed with `}`".into(),
                self.current_offset(),
            );
        }
    }

    fn scan_rich_content(&mut self) {
        // Triple backtick fenced block. Body is opaque content.
        let start = self.cursor;
        self.cursor += 3;
        let body_start = self.cursor;
        loop {
            if self.is_rich_open() {
                let body_end = self.cursor;
                self.cursor += 3;
                let body: String = self.scalars[body_start..body_end]
                    .iter()
                    .map(|(c, _)| *c)
                    .collect();
                let range = self.range_from(start, self.cursor);
                self.emit(SchemaTokenKind::RichContent { data: body }, range);
                return;
            }
            if self.peek().is_none() {
                let body: String = self.scalars[body_start..self.cursor]
                    .iter()
                    .map(|(c, _)| *c)
                    .collect();
                let range = self.range_from(start, self.cursor);
                self.diagnostic(
                    "cem.tokenizer.unterminated_rich_content",
                    Severity::Error,
                    "rich-content enclosure is not terminated".into(),
                    range.start,
                );
                self.emit(SchemaTokenKind::RichContent { data: body }, range);
                return;
            }
            self.cursor += 1;
        }
    }

    fn is_rich_open(&self) -> bool {
        self.peek() == Some('`') && self.peek_at(1) == Some('`') && self.peek_at(2) == Some('`')
    }

    fn skip_to_close(&mut self) {
        let mut depth = 1u32;
        while let Some(c) = self.peek() {
            self.cursor += 1;
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
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
            profile: TokenizerProfile::Cem,
            source_map: self.base_source_map.clone(),
        });
    }

    fn diagnostic(&mut self, code: &str, severity: Severity, message: String, byte_offset: u64) {
        self.diagnostics.push(Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset: Some(byte_offset),
            code: code.to_owned(),
            severity,
            message,
            node: None,
            source_map: None,
        });
    }
}

impl SchemaTokenizer for CemTokenizer {
    fn profile(&self) -> TokenizerProfile {
        TokenizerProfile::Cem
    }

    fn next_token(&mut self) -> Option<SchemaToken> {
        self.pending.pop_front()
    }
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_name_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn is_qname_continue(c: char) -> bool {
    is_name_continue(c) || c == ':'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{BytesSource, SourceId};

    fn tokenize(input: &str) -> (Vec<SchemaToken>, Vec<Diagnostic>) {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let mut t = CemTokenizer::from_source(src);
        let mut out = Vec::new();
        while let Some(tok) = t.next_token() {
            out.push(tok);
        }
        let diags = t.take_diagnostics();
        (out, diags)
    }

    fn kinds(tokens: &[SchemaToken]) -> Vec<&'static str> {
        tokens
            .iter()
            .map(|t| match &t.kind {
                SchemaTokenKind::NodeStart { .. } => "NodeStart",
                SchemaTokenKind::NodeEnd { .. } => "NodeEnd",
                SchemaTokenKind::Attribute { .. } => "Attribute",
                SchemaTokenKind::Text(_) => "Text",
                SchemaTokenKind::Trivia(_) => "Trivia",
                SchemaTokenKind::Comment(_) => "Comment",
                SchemaTokenKind::ProcessingInstruction { .. } => "PI",
                SchemaTokenKind::ExpressionNode(_) => "Expr",
                SchemaTokenKind::AnonymousScopeStart => "AnonStart",
                SchemaTokenKind::Directive { .. } => "Directive",
                SchemaTokenKind::RichContent { .. } => "Rich",
                SchemaTokenKind::Error { .. } => "Error",
            })
            .collect()
    }

    fn non_trivia(tokens: &[SchemaToken]) -> Vec<&SchemaToken> {
        tokens
            .iter()
            .filter(|t| !matches!(t.kind, SchemaTokenKind::Trivia(_)))
            .collect()
    }

    #[test]
    fn simple_node_with_content_boundary() {
        let (tokens, diags) = tokenize("{p | Hello}");
        assert!(diags.is_empty(), "{:?}", diags);
        let nt = non_trivia(&tokens);
        assert_eq!(kinds(&nt.iter().cloned().cloned().collect::<Vec<_>>()), vec!["NodeStart", "Text", "NodeEnd"]);
        if let SchemaTokenKind::NodeStart { name } = &nt[0].kind {
            assert_eq!(name, "p");
        } else { panic!() }
        if let SchemaTokenKind::Text(t) = &nt[1].kind {
            assert_eq!(t.trim(), "Hello");
        } else { panic!() }
    }

    #[test]
    fn attribute_with_quoted_value_and_bare_value() {
        let (tokens, diags) = tokenize(r#"{field @name=email @label="Email"}"#);
        assert!(diags.is_empty(), "{:?}", diags);
        let attrs: Vec<&SchemaToken> = tokens
            .iter()
            .filter(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .collect();
        assert_eq!(attrs.len(), 2);
        if let SchemaTokenKind::Attribute { name, value, .. } = &attrs[0].kind {
            assert_eq!(name, "name");
            assert_eq!(value.as_deref(), Some("email"));
        }
        if let SchemaTokenKind::Attribute { name, value, .. } = &attrs[1].kind {
            assert_eq!(name, "label");
            assert_eq!(value.as_deref(), Some("Email"));
        }
    }

    #[test]
    fn boolean_attribute_has_no_value() {
        let (tokens, diags) = tokenize("{input @required}");
        assert!(diags.is_empty());
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
    fn nested_nodes_emit_nested_tokens() {
        let (tokens, diags) = tokenize("{a | {b | x}}");
        assert!(diags.is_empty(), "{:?}", diags);
        let nt: Vec<_> = non_trivia(&tokens).into_iter().cloned().collect();
        assert_eq!(
            kinds(&nt),
            vec!["NodeStart", "NodeStart", "Text", "NodeEnd", "NodeEnd"]
        );
    }

    #[test]
    fn relaxed_content_boundary_omitted() {
        let (tokens, diags) = tokenize("{p Hello}");
        assert!(diags.is_empty(), "{:?}", diags);
        let nt: Vec<_> = non_trivia(&tokens).into_iter().cloned().collect();
        assert_eq!(kinds(&nt), vec!["NodeStart", "Text", "NodeEnd"]);
    }

    #[test]
    fn expression_node_collects_body() {
        let (tokens, diags) = tokenize("{$ .name}");
        assert!(diags.is_empty(), "{:?}", diags);
        let exprs: Vec<&SchemaToken> = tokens
            .iter()
            .filter(|t| matches!(t.kind, SchemaTokenKind::ExpressionNode(_)))
            .collect();
        assert_eq!(exprs.len(), 1);
        if let SchemaTokenKind::ExpressionNode(body) = &exprs[0].kind {
            assert_eq!(body, ".name");
        }
    }

    #[test]
    fn expression_node_with_explicit_boundary() {
        let (tokens, diags) = tokenize("{$ | count(.items)}");
        assert!(diags.is_empty(), "{:?}", diags);
        let exprs: Vec<&SchemaToken> = tokens
            .iter()
            .filter(|t| matches!(t.kind, SchemaTokenKind::ExpressionNode(_)))
            .collect();
        assert_eq!(exprs.len(), 1);
        if let SchemaTokenKind::ExpressionNode(body) = &exprs[0].kind {
            assert_eq!(body, "count(.items)");
        }
    }

    #[test]
    fn anonymous_typed_scope() {
        let (tokens, diags) = tokenize(r#"{@type="text/html" | <p>hi</p>}"#);
        assert!(diags.is_empty(), "{:?}", diags);
        assert!(tokens.iter().any(|t| matches!(t.kind, SchemaTokenKind::AnonymousScopeStart)));
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { name, value, .. } = &attr.kind {
            assert_eq!(name, "type");
            assert_eq!(value.as_deref(), Some("text/html"));
        }
    }

    #[test]
    fn directives_are_emitted() {
        let (tokens, diags) = tokenize("@doc cem-ml 1\n@ns cem = \"https://cem.dev/ns/core/1\"\n");
        assert!(diags.is_empty(), "{:?}", diags);
        let dirs: Vec<&SchemaToken> = tokens
            .iter()
            .filter(|t| matches!(t.kind, SchemaTokenKind::Directive { .. }))
            .collect();
        assert_eq!(dirs.len(), 2);
        if let SchemaTokenKind::Directive { name, data } = &dirs[0].kind {
            assert_eq!(name, "doc");
            assert_eq!(data, "cem-ml 1");
        }
        if let SchemaTokenKind::Directive { name, data } = &dirs[1].kind {
            assert_eq!(name, "ns");
            assert!(data.starts_with("cem = "));
        }
    }

    #[test]
    fn line_and_block_comments_are_preserved() {
        let (tokens, diags) = tokenize("// hello\n/* block */ {p x}");
        assert!(diags.is_empty(), "{:?}", diags);
        let comments: Vec<&SchemaToken> = tokens
            .iter()
            .filter(|t| matches!(t.kind, SchemaTokenKind::Comment(_)))
            .collect();
        assert_eq!(comments.len(), 2);
    }

    #[test]
    fn rich_content_enclosure_preserved_verbatim() {
        let (tokens, diags) = tokenize("```<div>{x}</div>```");
        assert!(diags.is_empty(), "{:?}", diags);
        let rich = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::RichContent { .. }))
            .unwrap();
        if let SchemaTokenKind::RichContent { data } = &rich.kind {
            assert_eq!(data, "<div>{x}</div>");
        }
    }

    #[test]
    fn attribute_avt_span_recognized() {
        let (tokens, diags) = tokenize("{button @disabled={.busy} | Save}");
        assert!(diags.is_empty(), "{:?}", diags);
        let attrs: Vec<&SchemaToken> = tokens
            .iter()
            .filter(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .collect();
        assert_eq!(attrs.len(), 1);
        if let SchemaTokenKind::Attribute { name, value, .. } = &attrs[0].kind {
            assert_eq!(name, "disabled");
            assert_eq!(value.as_deref(), Some("{.busy}"));
        }
    }

    #[test]
    fn bare_brace_text_interpolation_is_rejected() {
        let (tokens, diags) = tokenize("{p Hello {.name}}");
        assert!(!diags.is_empty());
        assert_eq!(diags[0].code, "cem.tokenizer.bare_brace_text");
        // Error token emitted.
        assert!(tokens.iter().any(|t| matches!(&t.kind, SchemaTokenKind::Error { code } if code == "cem.tokenizer.bare_brace_text")));
    }

    #[test]
    fn byte_ranges_are_absolute_and_contiguous() {
        let input = "{a x}";
        let (tokens, _) = tokenize(input);
        // First NodeStart begins at offset 0.
        let first = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::NodeStart { .. }))
            .unwrap();
        assert_eq!(first.byte_range.start, 0);
        // NodeEnd's last byte hits the closing '}'.
        let last_close = tokens
            .iter()
            .rev()
            .find(|t| matches!(t.kind, SchemaTokenKind::NodeEnd { .. }))
            .unwrap();
        assert_eq!(last_close.byte_range.end(), input.len() as u64);
    }

    #[test]
    fn qname_with_prefix_in_attribute_name() {
        let (tokens, diags) = tokenize(r#"{main @cem:screen="login"}"#);
        assert!(diags.is_empty());
        let attr = tokens
            .iter()
            .find(|t| matches!(t.kind, SchemaTokenKind::Attribute { .. }))
            .unwrap();
        if let SchemaTokenKind::Attribute { name, .. } = &attr.kind {
            assert_eq!(name, "cem:screen");
        }
    }

    #[test]
    fn all_canonical_fixtures_tokenize_without_hard_violations() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
        let mut at_least_one = false;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("cem") {
                continue;
            }
            at_least_one = true;
            let input = std::fs::read_to_string(&path).unwrap();
            let (tokens, diags) = tokenize(&input);
            let hard: Vec<&Diagnostic> = diags
                .iter()
                .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
                .collect();
            assert!(
                hard.is_empty(),
                "fixture `{}` produced hard violations: {hard:?}",
                path.display()
            );
            assert!(!tokens.is_empty(), "fixture `{}` produced no tokens", path.display());
        }
        assert!(at_least_one, "no canonical .cem fixtures found");
    }

    #[test]
    fn login_fixture_tokenizes_clean() {
        let input = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/cem-ml/login.cem"),
        )
        .unwrap();
        let (tokens, diags) = tokenize(&input);
        assert!(
            diags.iter().all(|d| !matches!(
                d.severity,
                Severity::Error | Severity::Fatal
            )),
            "expected no Error/Fatal diags, got: {diags:?}"
        );
        // Smoke-test: at least one NodeStart `main`, one Directive `doc`.
        let names: Vec<&str> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                SchemaTokenKind::NodeStart { name } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert!(names.contains(&"main"));
        let dirs: Vec<&str> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                SchemaTokenKind::Directive { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert!(dirs.contains(&"doc"));
        assert!(dirs.contains(&"ns"));
        assert!(dirs.contains(&"default"));
    }
}
