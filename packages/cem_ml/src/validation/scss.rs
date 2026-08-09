//! CEM-owned SCSS syntax and expansion.
//!
//! Grass 0.13.4 is a behavioral and algorithmic reference for the staged
//! lexer/parser/evaluator shape: token-cursor parsing, balanced delimiter
//! handling, explicit lexical environments, definition registration, and
//! visitor-style expansion. This module is an independent implementation over
//! CEM types. It does not depend on Grass, reuse its AST or source code, invoke
//! its serializer, or accept serialized CSS as an intermediate representation.

use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::{
    content_type_essence, CSS_CONTENT_TYPE, CSS_SCHEMA_URI, SCSS_CONTENT_TYPE, SCSS_SCHEMA_URI,
};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::css::{
    CssDocumentAst, CssDocumentSource, CssEncodingReportAst, CssEntryMode, CssEventAst,
    CssSourcePosition, CssSourceRange,
};
use std::collections::BTreeMap;

pub use crate::source_map::ScssOriginKind;

const SCSS_ALIAS_CONTENT_TYPE: &str = "text/x-scss";

#[derive(Debug, Clone, Copy)]
pub struct ScssSourceRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssSourceAst {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub syntax: String,
    pub encoding: String,
    pub byte_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScssSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScssSourceRange {
    pub start: ScssSourcePosition,
    pub byte_length: u64,
}

impl ScssSourceRange {
    fn from_offsets(line_index: &LineIndex, start: usize, end: usize) -> Self {
        let coordinate = line_index.project(start as u64);
        Self {
            start: ScssSourcePosition {
                line: coordinate.line,
                column: coordinate.column,
                byte_offset: start as u64,
            },
            byte_length: end.saturating_sub(start) as u64,
        }
    }

    fn end(self) -> usize {
        (self.start.byte_offset + self.byte_length) as usize
    }

    fn byte_range(self) -> ByteRange {
        ByteRange::new(
            self.start.byte_offset,
            u32::try_from(self.byte_length).unwrap_or(u32::MAX),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScssTokenKind {
    Whitespace,
    Comment,
    AtKeyword,
    Variable,
    Identifier,
    Number,
    String,
    InterpolationStart,
    InterpolationEnd,
    CurlyOpen,
    CurlyClose,
    ParenOpen,
    ParenClose,
    BracketOpen,
    BracketClose,
    Colon,
    Semicolon,
    Comma,
    Delimiter,
}

impl ScssTokenKind {
    fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssTokenAst {
    pub index: usize,
    pub depth: usize,
    pub kind: ScssTokenKind,
    pub lexeme: String,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScssStatementKind {
    VariableDeclaration,
    Rule,
    Declaration,
    MixinDeclaration,
    Include,
    FunctionDeclaration,
    Return,
    If,
    Each,
    For,
    While,
    Use,
    Forward,
    Import,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssExpression {
    pub raw: String,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssParameter {
    pub name: String,
    pub default: Option<ScssExpression>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssVariableDeclaration {
    pub name: String,
    pub value: ScssExpression,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssRule {
    pub selector: ScssExpression,
    pub body: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssDeclaration {
    pub name: ScssExpression,
    pub value: ScssExpression,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssMixinDeclaration {
    pub name: String,
    pub parameters: Vec<ScssParameter>,
    pub body: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssInclude {
    pub name: String,
    pub arguments: Vec<ScssExpression>,
    pub content: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssFunctionDeclaration {
    pub name: String,
    pub parameters: Vec<ScssParameter>,
    pub body: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssConditional {
    pub condition: ScssExpression,
    pub body: Vec<ScssStatement>,
    pub else_body: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssEachLoop {
    pub binding: String,
    pub values: Vec<ScssExpression>,
    pub body: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssForLoop {
    pub binding: String,
    pub from: ScssExpression,
    pub to: ScssExpression,
    pub inclusive: bool,
    pub body: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssWhileLoop {
    pub condition: ScssExpression,
    pub body: Vec<ScssStatement>,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssModuleDirective {
    pub value: String,
    pub source_range: ScssSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScssStatement {
    Variable(ScssVariableDeclaration),
    Rule(ScssRule),
    Declaration(ScssDeclaration),
    Mixin(ScssMixinDeclaration),
    Include(ScssInclude),
    Function(ScssFunctionDeclaration),
    Return(ScssExpression, ScssSourceRange),
    If(ScssConditional),
    Each(ScssEachLoop),
    For(ScssForLoop),
    While(ScssWhileLoop),
    Use(ScssModuleDirective),
    Forward(ScssModuleDirective),
    Import(ScssModuleDirective),
    Unknown(ScssSourceRange),
}

impl ScssStatement {
    pub fn kind(&self) -> ScssStatementKind {
        match self {
            Self::Variable(_) => ScssStatementKind::VariableDeclaration,
            Self::Rule(_) => ScssStatementKind::Rule,
            Self::Declaration(_) => ScssStatementKind::Declaration,
            Self::Mixin(_) => ScssStatementKind::MixinDeclaration,
            Self::Include(_) => ScssStatementKind::Include,
            Self::Function(_) => ScssStatementKind::FunctionDeclaration,
            Self::Return(..) => ScssStatementKind::Return,
            Self::If(_) => ScssStatementKind::If,
            Self::Each(_) => ScssStatementKind::Each,
            Self::For(_) => ScssStatementKind::For,
            Self::While(_) => ScssStatementKind::While,
            Self::Use(_) => ScssStatementKind::Use,
            Self::Forward(_) => ScssStatementKind::Forward,
            Self::Import(_) => ScssStatementKind::Import,
            Self::Unknown(_) => ScssStatementKind::Unknown,
        }
    }

    pub fn source_range(&self) -> ScssSourceRange {
        match self {
            Self::Variable(value) => value.source_range,
            Self::Rule(value) => value.source_range,
            Self::Declaration(value) => value.source_range,
            Self::Mixin(value) => value.source_range,
            Self::Include(value) => value.source_range,
            Self::Function(value) => value.source_range,
            Self::Return(_, range) => *range,
            Self::If(value) => value.source_range,
            Self::Each(value) => value.source_range,
            Self::For(value) => value.source_range,
            Self::While(value) => value.source_range,
            Self::Use(value) | Self::Forward(value) | Self::Import(value) => value.source_range,
            Self::Unknown(range) => *range,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScssFactKind {
    UseObserved,
    ForwardObserved,
    DeprecatedImport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssFact {
    pub kind: ScssFactKind,
    pub source_range: ScssSourceRange,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScssStylesheetAst {
    pub source: ScssSourceAst,
    pub tokens: Vec<ScssTokenAst>,
    pub statements: Vec<ScssStatement>,
    pub facts: Vec<ScssFact>,
    pub line_ending: Option<String>,
}

pub fn parse_scss_source_bytes(
    request: ScssSourceRequest<'_>,
) -> (Option<ScssStylesheetAst>, Vec<Diagnostic>) {
    let content_type = request.content_type.unwrap_or(SCSS_CONTENT_TYPE);
    let media_type = content_type_essence(content_type);
    if !matches!(
        media_type.as_str(),
        SCSS_CONTENT_TYPE | SCSS_ALIAS_CONTENT_TYPE
    ) {
        return (
            None,
            vec![scss_diagnostic(
                request.source_uri,
                "cem.scss.identity_mismatch",
                Severity::Error,
                format!(
                    "SCSS source requires `{SCSS_CONTENT_TYPE}` or `{SCSS_ALIAS_CONTENT_TYPE}`"
                ),
                None,
            )],
        );
    }
    if request.source_uri.to_ascii_lowercase().ends_with(".sass") {
        return (
            None,
            vec![scss_diagnostic(
                request.source_uri,
                "cem.scss.parse_error",
                Severity::Error,
                "Indented Sass syntax is outside the SCSS v1 contract",
                None,
            )],
        );
    }
    if let Some(charset) = content_type_parameter(content_type, "charset") {
        if normalize_charset(&charset) != "utf-8" {
            return (
                None,
                vec![scss_diagnostic(
                    request.source_uri,
                    "cem.scss.unsupported_encoding",
                    Severity::Error,
                    format!("SCSS v1 only accepts UTF-8, not `{charset}`"),
                    None,
                )],
            );
        }
    }
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            return (
                None,
                vec![scss_diagnostic(
                    request.source_uri,
                    "cem.scss.unsupported_encoding",
                    Severity::Error,
                    format!(
                        "SCSS bytes are not valid UTF-8 at byte {}",
                        error.valid_up_to()
                    ),
                    None,
                )],
            )
        }
    };
    let line_index = LineIndex::from_utf8(source);
    let (tokens, mut diagnostics) = tokenize_scss(source, request.source_uri, &line_index);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return (None, diagnostics);
    }
    let mut parser = ScssParser::new(source, &tokens, &line_index, request.source_uri);
    let statements = parser.parse_stylesheet();
    diagnostics.extend(parser.diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return (None, diagnostics);
    }
    let facts = collect_scss_facts(&statements);
    for fact in &facts {
        if fact.kind == ScssFactKind::DeprecatedImport {
            diagnostics.push(scss_diagnostic(
                request.source_uri,
                "cem.scss.import_deprecated",
                Severity::Warning,
                "Sass @import is accepted for compatibility but deprecated; use @use or @forward",
                Some(fact.source_range),
            ));
        }
    }
    (
        Some(ScssStylesheetAst {
            source: ScssSourceAst {
                uri: request.source_uri.to_owned(),
                content_type: content_type.to_owned(),
                media_type,
                syntax: "scss".to_owned(),
                encoding: "utf-8".to_owned(),
                byte_length: request.bytes.len(),
            },
            tokens,
            statements,
            facts,
            line_ending: detect_line_ending(source),
        }),
        diagnostics,
    )
}

fn tokenize_scss(
    source: &str,
    source_uri: &str,
    line_index: &LineIndex,
) -> (Vec<ScssTokenAst>, Vec<Diagnostic>) {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Delimiter {
        Curly,
        Interpolation,
        Paren,
        Bracket,
    }

    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut delimiters = Vec::new();
    let mut offset = 0usize;
    while offset < source.len() {
        let start = offset;
        let rest = &source[offset..];
        let first = rest.chars().next().expect("source character");
        let kind = if first.is_whitespace() {
            offset += first.len_utf8();
            while offset < source.len()
                && source[offset..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                offset += source[offset..].chars().next().unwrap().len_utf8();
            }
            ScssTokenKind::Whitespace
        } else if rest.starts_with("//") {
            offset += 2;
            while offset < source.len() && !source[offset..].starts_with(['\n', '\r']) {
                offset += source[offset..].chars().next().unwrap().len_utf8();
            }
            ScssTokenKind::Comment
        } else if rest.starts_with("/*") {
            offset += 2;
            if let Some(end) = source[offset..].find("*/") {
                offset += end + 2;
            } else {
                offset = source.len();
                diagnostics.push(scss_diagnostic(
                    source_uri,
                    "cem.scss.parse_error",
                    Severity::Error,
                    "Unterminated SCSS block comment",
                    Some(ScssSourceRange::from_offsets(line_index, start, offset)),
                ));
            }
            ScssTokenKind::Comment
        } else if matches!(first, '\'' | '"') {
            let quote = first;
            offset += first.len_utf8();
            let mut escaped = false;
            let mut closed = false;
            while offset < source.len() {
                let current = source[offset..].chars().next().unwrap();
                offset += current.len_utf8();
                if escaped {
                    escaped = false;
                } else if current == '\\' {
                    escaped = true;
                } else if current == quote {
                    closed = true;
                    break;
                }
            }
            if !closed {
                diagnostics.push(scss_diagnostic(
                    source_uri,
                    "cem.scss.parse_error",
                    Severity::Error,
                    "Unterminated SCSS string",
                    Some(ScssSourceRange::from_offsets(line_index, start, offset)),
                ));
            }
            ScssTokenKind::String
        } else if rest.starts_with("#{") {
            offset += 2;
            delimiters.push(Delimiter::Interpolation);
            ScssTokenKind::InterpolationStart
        } else if first == '$' || first == '@' {
            offset += first.len_utf8();
            while offset < source.len() {
                let current = source[offset..].chars().next().unwrap();
                if !is_identifier_continue(current) {
                    break;
                }
                offset += current.len_utf8();
            }
            if first == '$' {
                ScssTokenKind::Variable
            } else {
                ScssTokenKind::AtKeyword
            }
        } else if first.is_ascii_digit()
            || (first == '.'
                && rest
                    .chars()
                    .nth(1)
                    .is_some_and(|value| value.is_ascii_digit()))
        {
            offset += first.len_utf8();
            while offset < source.len() {
                let current = source[offset..].chars().next().unwrap();
                if !(current.is_ascii_alphanumeric()
                    || matches!(current, '.' | '%' | '-' | '_' | '+'))
                {
                    break;
                }
                offset += current.len_utf8();
            }
            ScssTokenKind::Number
        } else if is_identifier_start(first) {
            offset += first.len_utf8();
            while offset < source.len() {
                let current = source[offset..].chars().next().unwrap();
                if !is_identifier_continue(current) {
                    break;
                }
                offset += current.len_utf8();
            }
            ScssTokenKind::Identifier
        } else {
            offset += first.len_utf8();
            match first {
                '{' => {
                    delimiters.push(Delimiter::Curly);
                    ScssTokenKind::CurlyOpen
                }
                '}' => match delimiters.pop() {
                    Some(Delimiter::Interpolation) => ScssTokenKind::InterpolationEnd,
                    Some(Delimiter::Curly) => ScssTokenKind::CurlyClose,
                    Some(other) => {
                        delimiters.push(other);
                        ScssTokenKind::CurlyClose
                    }
                    None => ScssTokenKind::CurlyClose,
                },
                '(' => {
                    delimiters.push(Delimiter::Paren);
                    ScssTokenKind::ParenOpen
                }
                ')' => {
                    if delimiters.last() == Some(&Delimiter::Paren) {
                        delimiters.pop();
                    }
                    ScssTokenKind::ParenClose
                }
                '[' => {
                    delimiters.push(Delimiter::Bracket);
                    ScssTokenKind::BracketOpen
                }
                ']' => {
                    if delimiters.last() == Some(&Delimiter::Bracket) {
                        delimiters.pop();
                    }
                    ScssTokenKind::BracketClose
                }
                ':' => ScssTokenKind::Colon,
                ';' => ScssTokenKind::Semicolon,
                ',' => ScssTokenKind::Comma,
                _ => ScssTokenKind::Delimiter,
            }
        };
        let depth = delimiters.len();
        tokens.push(ScssTokenAst {
            index: tokens.len(),
            depth,
            kind,
            lexeme: source[start..offset].to_owned(),
            source_range: ScssSourceRange::from_offsets(line_index, start, offset),
        });
    }
    if delimiters
        .iter()
        .any(|delimiter| matches!(delimiter, Delimiter::Curly | Delimiter::Interpolation))
    {
        diagnostics.push(scss_diagnostic(
            source_uri,
            "cem.scss.parse_error",
            Severity::Error,
            "Unclosed SCSS block or interpolation",
            tokens.last().map(|token| token.source_range),
        ));
    }
    (tokens, diagnostics)
}

fn is_identifier_start(value: char) -> bool {
    value.is_alphabetic() || matches!(value, '_' | '-')
}

fn is_identifier_continue(value: char) -> bool {
    is_identifier_start(value) || value.is_ascii_digit()
}

struct ScssParser<'a> {
    source: &'a str,
    tokens: &'a [ScssTokenAst],
    line_index: &'a LineIndex,
    source_uri: &'a str,
    position: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ScssParser<'a> {
    fn new(
        source: &'a str,
        tokens: &'a [ScssTokenAst],
        line_index: &'a LineIndex,
        source_uri: &'a str,
    ) -> Self {
        Self {
            source,
            tokens,
            line_index,
            source_uri,
            position: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_stylesheet(&mut self) -> Vec<ScssStatement> {
        self.parse_statements(false)
    }

    fn parse_statements(&mut self, stop_at_close: bool) -> Vec<ScssStatement> {
        let mut statements = Vec::new();
        loop {
            self.skip_trivia();
            let Some(token) = self.current() else {
                if stop_at_close {
                    self.error("Expected `}` before the end of SCSS input", None);
                }
                break;
            };
            if token.kind == ScssTokenKind::CurlyClose {
                if stop_at_close {
                    self.position += 1;
                    break;
                }
                self.error("Unmatched `}` in SCSS source", Some(token.source_range));
                self.position += 1;
                continue;
            }
            let statement = if token.kind == ScssTokenKind::Variable {
                self.parse_variable()
            } else if token.kind == ScssTokenKind::AtKeyword {
                match token.lexeme.to_ascii_lowercase().as_str() {
                    "@mixin" => self.parse_mixin(),
                    "@include" => self.parse_include(),
                    "@function" => self.parse_function(),
                    "@return" => self.parse_return(),
                    "@if" => self.parse_if(),
                    "@each" => self.parse_each(),
                    "@for" => self.parse_for(),
                    "@while" => self.parse_while(),
                    "@use" => self.parse_module_directive(ScssStatementKind::Use),
                    "@forward" => self.parse_module_directive(ScssStatementKind::Forward),
                    "@import" => self.parse_module_directive(ScssStatementKind::Import),
                    _ => self.parse_unknown(),
                }
            } else {
                self.parse_rule_or_declaration()
            };
            match statement {
                Some(statement) => statements.push(statement),
                None => self.recover_statement(),
            }
        }
        statements
    }

    fn parse_variable(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        let name = self.current()?.lexeme.trim_start_matches('$').to_owned();
        self.position += 1;
        self.skip_trivia();
        self.expect(ScssTokenKind::Colon, "Expected `:` after SCSS variable")?;
        let value_start = self.position;
        let end = self.scan_to_top_level(&[ScssTokenKind::Semicolon])?;
        let value = self.expression(value_start, end);
        self.position = end + 1;
        Some(ScssStatement::Variable(ScssVariableDeclaration {
            name,
            value,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_rule_or_declaration(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        let Some(boundary) = self.scan_to_top_level(&[
            ScssTokenKind::CurlyOpen,
            ScssTokenKind::Semicolon,
            ScssTokenKind::CurlyClose,
        ]) else {
            self.error(
                "Expected `{` after an SCSS selector or `;` after a declaration",
                Some(self.range_for_tokens(start, self.tokens.len())),
            );
            return None;
        };
        match self.tokens[boundary].kind {
            ScssTokenKind::CurlyOpen => {
                let selector = self.expression(start, boundary);
                self.position = boundary + 1;
                let body = self.parse_statements(true);
                Some(ScssStatement::Rule(ScssRule {
                    selector,
                    body,
                    source_range: self.range_for_tokens(start, self.position),
                }))
            }
            ScssTokenKind::Semicolon => {
                let colon = (start..boundary)
                    .find(|index| self.tokens[*index].kind == ScssTokenKind::Colon);
                let Some(colon) = colon else {
                    self.error(
                        "Expected `:` in SCSS declaration",
                        Some(self.range_for_tokens(start, boundary + 1)),
                    );
                    return None;
                };
                let name = self.expression(start, colon);
                let value = self.expression(colon + 1, boundary);
                self.position = boundary + 1;
                Some(ScssStatement::Declaration(ScssDeclaration {
                    name,
                    value,
                    source_range: self.range_for_tokens(start, self.position),
                }))
            }
            ScssTokenKind::CurlyClose => {
                self.error(
                    "Expected `;` after SCSS declaration",
                    Some(self.range_for_tokens(start, boundary)),
                );
                None
            }
            _ => None,
        }
    }

    fn parse_mixin(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        self.skip_trivia();
        let name = self.take_name("Expected mixin name")?;
        let parameters = self.parse_parameters()?;
        self.skip_trivia();
        self.expect(
            ScssTokenKind::CurlyOpen,
            "Expected `{` after mixin declaration",
        )?;
        let body = self.parse_statements(true);
        Some(ScssStatement::Mixin(ScssMixinDeclaration {
            name,
            parameters,
            body,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_include(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        self.skip_trivia();
        let name = self.take_qualified_name("Expected mixin name after @include")?;
        let arguments = self.parse_arguments().unwrap_or_default();
        self.skip_trivia();
        let content = if self.matches(ScssTokenKind::CurlyOpen) {
            self.position += 1;
            self.parse_statements(true)
        } else {
            self.expect(ScssTokenKind::Semicolon, "Expected `;` after @include")?;
            Vec::new()
        };
        Some(ScssStatement::Include(ScssInclude {
            name,
            arguments,
            content,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_function(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        self.skip_trivia();
        let name = self.take_name("Expected function name")?;
        let parameters = self.parse_parameters()?;
        self.skip_trivia();
        self.expect(
            ScssTokenKind::CurlyOpen,
            "Expected `{` after function declaration",
        )?;
        let body = self.parse_statements(true);
        Some(ScssStatement::Function(ScssFunctionDeclaration {
            name,
            parameters,
            body,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_return(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        let value_start = self.position;
        let end = self.scan_to_top_level(&[ScssTokenKind::Semicolon])?;
        let value = self.expression(value_start, end);
        self.position = end + 1;
        Some(ScssStatement::Return(
            value,
            self.range_for_tokens(start, self.position),
        ))
    }

    fn parse_if(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        let condition_start = self.position;
        let open = self.scan_to_top_level(&[ScssTokenKind::CurlyOpen])?;
        let condition = self.expression(condition_start, open);
        self.position = open + 1;
        let body = self.parse_statements(true);
        self.skip_trivia();
        let else_body = if self
            .current()
            .is_some_and(|token| token.kind == ScssTokenKind::AtKeyword && token.lexeme == "@else")
        {
            self.position += 1;
            self.skip_trivia();
            self.expect(ScssTokenKind::CurlyOpen, "Expected `{` after @else")?;
            self.parse_statements(true)
        } else {
            Vec::new()
        };
        Some(ScssStatement::If(ScssConditional {
            condition,
            body,
            else_body,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_each(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        self.skip_trivia();
        let binding = self
            .current()
            .filter(|token| token.kind == ScssTokenKind::Variable)?
            .lexeme
            .trim_start_matches('$')
            .to_owned();
        self.position += 1;
        self.skip_trivia();
        if self.current()?.lexeme != "in" {
            self.error(
                "Expected `in` in @each rule",
                self.current().map(|token| token.source_range),
            );
            return None;
        }
        self.position += 1;
        let values_start = self.position;
        let open = self.scan_to_top_level(&[ScssTokenKind::CurlyOpen])?;
        let values = self.expression_list(values_start, open);
        self.position = open + 1;
        let body = self.parse_statements(true);
        Some(ScssStatement::Each(ScssEachLoop {
            binding,
            values,
            body,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_for(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        self.skip_trivia();
        let binding = self
            .current()
            .filter(|token| token.kind == ScssTokenKind::Variable)?
            .lexeme
            .trim_start_matches('$')
            .to_owned();
        self.position += 1;
        self.skip_trivia();
        if self.current()?.lexeme != "from" {
            self.error(
                "Expected `from` in @for rule",
                self.current().map(|token| token.source_range),
            );
            return None;
        }
        self.position += 1;
        let from_start = self.position;
        let through = (self.position..self.tokens.len())
            .find(|index| matches!(self.tokens[*index].lexeme.as_str(), "through" | "to"))?;
        let from = self.expression(from_start, through);
        let inclusive = self.tokens[through].lexeme == "through";
        self.position = through + 1;
        let to_start = self.position;
        let open = self.scan_to_top_level(&[ScssTokenKind::CurlyOpen])?;
        let to = self.expression(to_start, open);
        self.position = open + 1;
        let body = self.parse_statements(true);
        Some(ScssStatement::For(ScssForLoop {
            binding,
            from,
            to,
            inclusive,
            body,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_while(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        let condition_start = self.position;
        let open = self.scan_to_top_level(&[ScssTokenKind::CurlyOpen])?;
        let condition = self.expression(condition_start, open);
        self.position = open + 1;
        let body = self.parse_statements(true);
        Some(ScssStatement::While(ScssWhileLoop {
            condition,
            body,
            source_range: self.range_for_tokens(start, self.position),
        }))
    }

    fn parse_module_directive(&mut self, kind: ScssStatementKind) -> Option<ScssStatement> {
        let start = self.position;
        self.position += 1;
        let value_start = self.position;
        let end = self.scan_to_top_level(&[ScssTokenKind::Semicolon])?;
        let value = self.raw_for_tokens(value_start, end).trim().to_owned();
        self.position = end + 1;
        let directive = ScssModuleDirective {
            value,
            source_range: self.range_for_tokens(start, self.position),
        };
        Some(match kind {
            ScssStatementKind::Use => ScssStatement::Use(directive),
            ScssStatementKind::Forward => ScssStatement::Forward(directive),
            ScssStatementKind::Import => ScssStatement::Import(directive),
            _ => unreachable!("module directive kind"),
        })
    }

    fn parse_unknown(&mut self) -> Option<ScssStatement> {
        let start = self.position;
        let boundary = self.scan_to_top_level(&[
            ScssTokenKind::CurlyOpen,
            ScssTokenKind::Semicolon,
            ScssTokenKind::CurlyClose,
        ])?;
        if self.tokens[boundary].kind == ScssTokenKind::CurlyOpen {
            self.position = boundary + 1;
            let _ = self.parse_statements(true);
        } else if self.tokens[boundary].kind == ScssTokenKind::Semicolon {
            self.position = boundary + 1;
        } else {
            self.position = boundary;
        }
        Some(ScssStatement::Unknown(
            self.range_for_tokens(start, self.position),
        ))
    }

    fn parse_parameters(&mut self) -> Option<Vec<ScssParameter>> {
        self.skip_trivia();
        self.expect(ScssTokenKind::ParenOpen, "Expected `(` before parameters")?;
        let start = self.position;
        let close = self.scan_to_top_level(&[ScssTokenKind::ParenClose])?;
        let chunks = self.comma_chunks(start, close);
        self.position = close + 1;
        let mut parameters = Vec::new();
        for (chunk_start, chunk_end) in chunks {
            let significant =
                (chunk_start..chunk_end).find(|index| !self.tokens[*index].kind.is_trivia());
            let Some(name_index) = significant else {
                continue;
            };
            if self.tokens[name_index].kind != ScssTokenKind::Variable {
                self.error(
                    "Expected SCSS parameter name",
                    Some(self.range_for_tokens(chunk_start, chunk_end)),
                );
                return None;
            }
            let colon = (name_index + 1..chunk_end)
                .find(|index| self.tokens[*index].kind == ScssTokenKind::Colon);
            parameters.push(ScssParameter {
                name: self.tokens[name_index]
                    .lexeme
                    .trim_start_matches('$')
                    .to_owned(),
                default: colon.map(|colon| self.expression(colon + 1, chunk_end)),
                source_range: self.range_for_tokens(chunk_start, chunk_end),
            });
        }
        Some(parameters)
    }

    fn parse_arguments(&mut self) -> Option<Vec<ScssExpression>> {
        self.skip_trivia();
        if !self.matches(ScssTokenKind::ParenOpen) {
            return Some(Vec::new());
        }
        self.position += 1;
        let start = self.position;
        let close = self.scan_to_top_level(&[ScssTokenKind::ParenClose])?;
        let arguments = self.expression_list(start, close);
        self.position = close + 1;
        Some(arguments)
    }

    fn expression_list(&self, start: usize, end: usize) -> Vec<ScssExpression> {
        self.comma_chunks(start, end)
            .into_iter()
            .filter(|(chunk_start, chunk_end)| {
                (*chunk_start..*chunk_end).any(|index| !self.tokens[index].kind.is_trivia())
            })
            .map(|(chunk_start, chunk_end)| self.expression(chunk_start, chunk_end))
            .collect()
    }

    fn comma_chunks(&self, start: usize, end: usize) -> Vec<(usize, usize)> {
        let mut chunks = Vec::new();
        let mut chunk_start = start;
        let mut nesting = 0usize;
        for index in start..end {
            match self.tokens[index].kind {
                ScssTokenKind::ParenOpen | ScssTokenKind::BracketOpen => nesting += 1,
                ScssTokenKind::ParenClose | ScssTokenKind::BracketClose => {
                    nesting = nesting.saturating_sub(1)
                }
                ScssTokenKind::Comma if nesting == 0 => {
                    chunks.push((chunk_start, index));
                    chunk_start = index + 1;
                }
                _ => {}
            }
        }
        chunks.push((chunk_start, end));
        chunks
    }

    fn expression(&self, start: usize, end: usize) -> ScssExpression {
        let (start, end) = self.trim_token_range(start, end);
        ScssExpression {
            raw: self.raw_for_tokens(start, end),
            source_range: self.range_for_tokens(start, end),
        }
    }

    fn scan_to_top_level(&self, stop: &[ScssTokenKind]) -> Option<usize> {
        let mut nesting = 0usize;
        for index in self.position..self.tokens.len() {
            let kind = self.tokens[index].kind;
            if nesting == 0 && stop.contains(&kind) {
                return Some(index);
            }
            match kind {
                ScssTokenKind::ParenOpen | ScssTokenKind::BracketOpen => nesting += 1,
                ScssTokenKind::ParenClose | ScssTokenKind::BracketClose => {
                    if nesting > 0 {
                        nesting -= 1;
                    } else if stop.contains(&kind) {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn take_name(&mut self, message: &str) -> Option<String> {
        let token = self.current()?;
        if token.kind != ScssTokenKind::Identifier {
            self.error(message, Some(token.source_range));
            return None;
        }
        let name = token.lexeme.clone();
        self.position += 1;
        Some(name)
    }

    fn take_qualified_name(&mut self, message: &str) -> Option<String> {
        let start = self.position;
        while let Some(token) = self.current() {
            if matches!(
                token.kind,
                ScssTokenKind::ParenOpen | ScssTokenKind::Semicolon
            ) || token.kind.is_trivia()
            {
                break;
            }
            self.position += 1;
        }
        if start == self.position {
            self.error(message, self.current().map(|token| token.source_range));
            None
        } else {
            Some(self.raw_for_tokens(start, self.position).trim().to_owned())
        }
    }

    fn expect(&mut self, kind: ScssTokenKind, message: &str) -> Option<()> {
        self.skip_trivia();
        if self.matches(kind) {
            self.position += 1;
            Some(())
        } else {
            self.error(message, self.current().map(|token| token.source_range));
            None
        }
    }

    fn matches(&self, kind: ScssTokenKind) -> bool {
        self.current().is_some_and(|token| token.kind == kind)
    }

    fn current(&self) -> Option<&ScssTokenAst> {
        self.tokens.get(self.position)
    }

    fn skip_trivia(&mut self) {
        while self.current().is_some_and(|token| token.kind.is_trivia()) {
            self.position += 1;
        }
    }

    fn trim_token_range(&self, mut start: usize, mut end: usize) -> (usize, usize) {
        while start < end && self.tokens[start].kind.is_trivia() {
            start += 1;
        }
        while end > start && self.tokens[end - 1].kind.is_trivia() {
            end -= 1;
        }
        (start, end)
    }

    fn raw_for_tokens(&self, start: usize, end: usize) -> String {
        if start >= end || start >= self.tokens.len() {
            return String::new();
        }
        let byte_start = self.tokens[start].source_range.start.byte_offset as usize;
        let byte_end = self.tokens[end - 1].source_range.end();
        self.source[byte_start..byte_end].to_owned()
    }

    fn range_for_tokens(&self, start: usize, end: usize) -> ScssSourceRange {
        if start >= end || start >= self.tokens.len() {
            let offset = self.tokens.get(start).map_or(self.source.len(), |token| {
                token.source_range.start.byte_offset as usize
            });
            return ScssSourceRange::from_offsets(self.line_index, offset, offset);
        }
        let byte_start = self.tokens[start].source_range.start.byte_offset as usize;
        let byte_end = self.tokens[end - 1].source_range.end();
        ScssSourceRange::from_offsets(self.line_index, byte_start, byte_end)
    }

    fn recover_statement(&mut self) {
        while let Some(kind) = self.current().map(|token| token.kind) {
            self.position += 1;
            if matches!(kind, ScssTokenKind::Semicolon | ScssTokenKind::CurlyClose) {
                break;
            }
        }
    }

    fn error(&mut self, message: &str, range: Option<ScssSourceRange>) {
        self.diagnostics.push(scss_diagnostic(
            self.source_uri,
            "cem.scss.parse_error",
            Severity::Error,
            message,
            range,
        ));
    }
}

fn collect_scss_facts(statements: &[ScssStatement]) -> Vec<ScssFact> {
    fn visit(statements: &[ScssStatement], facts: &mut Vec<ScssFact>) {
        for statement in statements {
            match statement {
                ScssStatement::Use(value) => facts.push(ScssFact {
                    kind: ScssFactKind::UseObserved,
                    source_range: value.source_range,
                    value: Some(value.value.clone()),
                }),
                ScssStatement::Forward(value) => facts.push(ScssFact {
                    kind: ScssFactKind::ForwardObserved,
                    source_range: value.source_range,
                    value: Some(value.value.clone()),
                }),
                ScssStatement::Import(value) => facts.push(ScssFact {
                    kind: ScssFactKind::DeprecatedImport,
                    source_range: value.source_range,
                    value: Some(value.value.clone()),
                }),
                ScssStatement::Rule(value) => visit(&value.body, facts),
                ScssStatement::Mixin(value) => visit(&value.body, facts),
                ScssStatement::Function(value) => visit(&value.body, facts),
                ScssStatement::If(value) => {
                    visit(&value.body, facts);
                    visit(&value.else_body, facts);
                }
                ScssStatement::Each(value) => visit(&value.body, facts),
                ScssStatement::For(value) => visit(&value.body, facts),
                ScssStatement::While(value) => visit(&value.body, facts),
                _ => {}
            }
        }
    }
    let mut facts = Vec::new();
    visit(statements, &mut facts);
    facts
}

#[derive(Debug, Clone, Copy)]
pub struct ScssEvaluationRequest<'a> {
    pub stylesheet: &'a ScssStylesheetAst,
}

#[derive(Debug, Clone)]
pub struct ScssEvaluationResult {
    pub document: Option<CssDocumentAst>,
    pub diagnostics: Vec<Diagnostic>,
    pub target_schema: &'static str,
}

#[derive(Debug, Clone)]
struct OriginSpec {
    kind: ScssOriginKind,
    name: Option<String>,
    range: ScssSourceRange,
}

#[derive(Debug, Clone)]
struct EvaluatedValue {
    text: String,
    origins: Vec<OriginSpec>,
}

#[derive(Debug, Clone)]
struct VariableBinding {
    value: String,
    origins: Vec<OriginSpec>,
}

#[derive(Debug, Clone, Default)]
struct EvaluationScope {
    variables: BTreeMap<String, VariableBinding>,
}

#[derive(Debug, Clone)]
struct GeneratedDeclaration {
    name: String,
    value: String,
    source_range: ScssSourceRange,
    origins: Vec<OriginSpec>,
}

#[derive(Debug, Clone)]
struct GeneratedRule {
    selector: String,
    source_range: ScssSourceRange,
    selector_origins: Vec<OriginSpec>,
    declarations: Vec<GeneratedDeclaration>,
}

struct ScssEvaluator<'a> {
    stylesheet: &'a ScssStylesheetAst,
    mixins: BTreeMap<String, ScssMixinDeclaration>,
    functions: BTreeMap<String, ScssFunctionDeclaration>,
    diagnostics: Vec<Diagnostic>,
}

pub fn evaluate_scss_to_css(request: ScssEvaluationRequest<'_>) -> ScssEvaluationResult {
    let mut evaluator = ScssEvaluator::new(request.stylesheet);
    let rules = evaluator.evaluate();
    let document = if evaluator
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        None
    } else {
        Some(evaluator.css_document(rules))
    };
    ScssEvaluationResult {
        document,
        diagnostics: evaluator.diagnostics,
        target_schema: CSS_SCHEMA_URI,
    }
}

impl<'a> ScssEvaluator<'a> {
    fn new(stylesheet: &'a ScssStylesheetAst) -> Self {
        let mut mixins = BTreeMap::new();
        let mut functions = BTreeMap::new();
        for statement in &stylesheet.statements {
            match statement {
                ScssStatement::Mixin(value) => {
                    mixins.insert(normalize_name(&value.name), value.clone());
                }
                ScssStatement::Function(value) => {
                    functions.insert(normalize_name(&value.name), value.clone());
                }
                _ => {}
            }
        }
        Self {
            stylesheet,
            mixins,
            functions,
            diagnostics: Vec::new(),
        }
    }

    fn evaluate(&mut self) -> Vec<GeneratedRule> {
        let mut scope = EvaluationScope::default();
        let mut rules = Vec::new();
        let statements = self.stylesheet.statements.clone();
        self.evaluate_top_level(&statements, &mut scope, &mut rules, &[]);
        rules
    }

    fn evaluate_top_level(
        &mut self,
        statements: &[ScssStatement],
        scope: &mut EvaluationScope,
        rules: &mut Vec<GeneratedRule>,
        frames: &[OriginSpec],
    ) {
        for statement in statements {
            match statement {
                ScssStatement::Variable(value) => self.bind_variable(value, scope, frames),
                ScssStatement::Rule(value) => {
                    rules.extend(self.evaluate_rule(value, scope, None, frames));
                }
                ScssStatement::If(value) => {
                    let condition = self.evaluate_expression(&value.condition, scope, frames);
                    let branch = if is_truthy(&condition.text) {
                        &value.body
                    } else {
                        &value.else_body
                    };
                    self.evaluate_top_level(branch, scope, rules, frames);
                }
                ScssStatement::Each(value) => {
                    for item in &value.values {
                        let evaluated = self.evaluate_expression(item, scope, frames);
                        let mut iteration = scope.clone();
                        iteration.variables.insert(
                            normalize_name(&value.binding),
                            VariableBinding {
                                value: evaluated.text,
                                origins: evaluated.origins,
                            },
                        );
                        self.evaluate_top_level(&value.body, &mut iteration, rules, frames);
                    }
                }
                ScssStatement::For(value) => {
                    let from = self.evaluate_expression(&value.from, scope, frames);
                    let to = self.evaluate_expression(&value.to, scope, frames);
                    if let (Some((from, unit)), Some((to, to_unit))) =
                        (parse_numeric(&from.text), parse_numeric(&to.text))
                    {
                        if unit == to_unit {
                            let end = if value.inclusive {
                                to as i64
                            } else {
                                to as i64 - 1
                            };
                            for number in from as i64..=end {
                                let mut iteration = scope.clone();
                                iteration.variables.insert(
                                    normalize_name(&value.binding),
                                    VariableBinding {
                                        value: format_number(number as f64, &unit),
                                        origins: vec![OriginSpec {
                                            kind: ScssOriginKind::Definition,
                                            name: Some(value.binding.clone()),
                                            range: value.source_range,
                                        }],
                                    },
                                );
                                self.evaluate_top_level(&value.body, &mut iteration, rules, frames);
                            }
                        }
                    }
                }
                ScssStatement::While(value) => {
                    let mut iterations = 0usize;
                    while is_truthy(
                        &self
                            .evaluate_expression(&value.condition, scope, frames)
                            .text,
                    ) && iterations < 1_024
                    {
                        self.evaluate_top_level(&value.body, scope, rules, frames);
                        iterations += 1;
                    }
                }
                _ => {}
            }
        }
    }

    fn evaluate_rule(
        &mut self,
        rule: &ScssRule,
        scope: &EvaluationScope,
        parent_selector: Option<&str>,
        frames: &[OriginSpec],
    ) -> Vec<GeneratedRule> {
        let selector_value = self.evaluate_interpolated(&rule.selector, scope, frames);
        let selector = combine_selectors(parent_selector, &selector_value.text);
        let mut local_scope = scope.clone();
        let mut declarations = Vec::new();
        let mut nested = Vec::new();
        self.evaluate_rule_body(
            &rule.body,
            &mut local_scope,
            &selector,
            &mut declarations,
            &mut nested,
            frames,
        );
        let mut rules = Vec::new();
        if !declarations.is_empty() {
            rules.push(GeneratedRule {
                selector,
                source_range: rule.selector.source_range,
                selector_origins: selector_value.origins,
                declarations,
            });
        }
        rules.extend(nested);
        rules
    }

    fn evaluate_rule_body(
        &mut self,
        statements: &[ScssStatement],
        scope: &mut EvaluationScope,
        selector: &str,
        declarations: &mut Vec<GeneratedDeclaration>,
        nested: &mut Vec<GeneratedRule>,
        frames: &[OriginSpec],
    ) {
        for statement in statements {
            match statement {
                ScssStatement::Variable(value) => self.bind_variable(value, scope, frames),
                ScssStatement::Declaration(value) => {
                    let name = self.evaluate_interpolated(&value.name, scope, frames);
                    let evaluated = self.evaluate_expression(&value.value, scope, frames);
                    let mut origins = frames.to_vec();
                    origins.extend(name.origins);
                    origins.extend(evaluated.origins);
                    declarations.push(GeneratedDeclaration {
                        name: name.text,
                        value: evaluated.text,
                        source_range: value.source_range,
                        origins,
                    });
                }
                ScssStatement::Rule(value) => {
                    nested.extend(self.evaluate_rule(value, scope, Some(selector), frames));
                }
                ScssStatement::Include(value) => {
                    self.evaluate_include(value, scope, selector, declarations, nested, frames);
                }
                ScssStatement::If(value) => {
                    let condition = self.evaluate_expression(&value.condition, scope, frames);
                    let branch = if is_truthy(&condition.text) {
                        &value.body
                    } else {
                        &value.else_body
                    };
                    self.evaluate_rule_body(branch, scope, selector, declarations, nested, frames);
                }
                ScssStatement::Each(value) => {
                    for item in &value.values {
                        let evaluated = self.evaluate_expression(item, scope, frames);
                        let mut iteration = scope.clone();
                        iteration.variables.insert(
                            normalize_name(&value.binding),
                            VariableBinding {
                                value: evaluated.text,
                                origins: evaluated.origins,
                            },
                        );
                        self.evaluate_rule_body(
                            &value.body,
                            &mut iteration,
                            selector,
                            declarations,
                            nested,
                            frames,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn evaluate_include(
        &mut self,
        include: &ScssInclude,
        scope: &EvaluationScope,
        selector: &str,
        declarations: &mut Vec<GeneratedDeclaration>,
        nested: &mut Vec<GeneratedRule>,
        frames: &[OriginSpec],
    ) {
        let name = normalize_name(&include.name);
        let Some(mixin) = self.mixins.get(&name).cloned() else {
            self.diagnostics.push(scss_diagnostic(
                &self.stylesheet.source.uri,
                "cem.scss.module_error",
                Severity::Error,
                format!("SCSS mixin `{}` is not defined", include.name),
                Some(include.source_range),
            ));
            return;
        };
        let mut call_scope = scope.clone();
        for (index, parameter) in mixin.parameters.iter().enumerate() {
            let evaluated = include
                .arguments
                .get(index)
                .map(|argument| self.evaluate_expression(argument, scope, frames))
                .or_else(|| {
                    parameter
                        .default
                        .as_ref()
                        .map(|default| self.evaluate_expression(default, scope, frames))
                });
            if let Some(evaluated) = evaluated {
                let mut origins = evaluated.origins;
                origins.push(OriginSpec {
                    kind: ScssOriginKind::Definition,
                    name: Some(parameter.name.clone()),
                    range: parameter.source_range,
                });
                call_scope.variables.insert(
                    normalize_name(&parameter.name),
                    VariableBinding {
                        value: evaluated.text,
                        origins,
                    },
                );
            }
        }
        let mut call_frames = frames.to_vec();
        call_frames.push(OriginSpec {
            kind: ScssOriginKind::Definition,
            name: Some(mixin.name.clone()),
            range: mixin.source_range,
        });
        call_frames.push(OriginSpec {
            kind: ScssOriginKind::CallSite,
            name: Some(include.name.clone()),
            range: include.source_range,
        });
        self.evaluate_rule_body(
            &mixin.body,
            &mut call_scope,
            selector,
            declarations,
            nested,
            &call_frames,
        );
        if !include.content.is_empty() {
            self.evaluate_rule_body(
                &include.content,
                &mut call_scope,
                selector,
                declarations,
                nested,
                &call_frames,
            );
        }
    }

    fn bind_variable(
        &mut self,
        declaration: &ScssVariableDeclaration,
        scope: &mut EvaluationScope,
        frames: &[OriginSpec],
    ) {
        let evaluated = self.evaluate_expression(&declaration.value, scope, frames);
        let mut origins = evaluated.origins;
        origins.push(OriginSpec {
            kind: ScssOriginKind::Definition,
            name: Some(declaration.name.clone()),
            range: declaration.source_range,
        });
        scope.variables.insert(
            normalize_name(&declaration.name),
            VariableBinding {
                value: evaluated.text,
                origins,
            },
        );
    }

    fn evaluate_interpolated(
        &mut self,
        expression: &ScssExpression,
        scope: &EvaluationScope,
        frames: &[OriginSpec],
    ) -> EvaluatedValue {
        self.evaluate_text(
            expression.raw.trim(),
            expression.source_range,
            scope,
            frames,
            true,
        )
    }

    fn evaluate_expression(
        &mut self,
        expression: &ScssExpression,
        scope: &EvaluationScope,
        frames: &[OriginSpec],
    ) -> EvaluatedValue {
        self.evaluate_text(
            expression.raw.trim(),
            expression.source_range,
            scope,
            frames,
            false,
        )
    }

    fn evaluate_text(
        &mut self,
        raw: &str,
        range: ScssSourceRange,
        scope: &EvaluationScope,
        frames: &[OriginSpec],
        selector_mode: bool,
    ) -> EvaluatedValue {
        let mut text = raw.to_owned();
        let mut origins = frames.to_vec();
        text = self.expand_custom_functions(&text, range, scope, frames, &mut origins);
        text = expand_interpolations(
            &text,
            range,
            |inner| self.evaluate_text(inner, range, scope, frames, false),
            &mut origins,
        );
        text = substitute_variables(&text, scope, &mut origins);
        if let Some(arithmetic) = evaluate_arithmetic(text.trim()) {
            text = arithmetic;
        }
        if selector_mode {
            text = text.trim().to_owned();
        }
        EvaluatedValue { text, origins }
    }

    fn expand_custom_functions(
        &mut self,
        raw: &str,
        range: ScssSourceRange,
        scope: &EvaluationScope,
        frames: &[OriginSpec],
        origins: &mut Vec<OriginSpec>,
    ) -> String {
        let mut output = String::new();
        let mut cursor = 0usize;
        while cursor < raw.len() {
            let Some((name_start, name_end, open, close)) = next_function_call(raw, cursor) else {
                output.push_str(&raw[cursor..]);
                break;
            };
            let name = &raw[name_start..name_end];
            let key = normalize_name(name);
            let Some(function) = self.functions.get(&key).cloned() else {
                output.push_str(&raw[cursor..=open]);
                cursor = open + 1;
                continue;
            };
            output.push_str(&raw[cursor..name_start]);
            let arguments = split_expression_text(&raw[open + 1..close]);
            let evaluated = self.evaluate_function(&function, &arguments, range, scope, frames);
            output.push_str(&evaluated.text);
            origins.extend(evaluated.origins);
            cursor = close + 1;
        }
        output
    }

    fn evaluate_function(
        &mut self,
        function: &ScssFunctionDeclaration,
        arguments: &[String],
        call_range: ScssSourceRange,
        scope: &EvaluationScope,
        frames: &[OriginSpec],
    ) -> EvaluatedValue {
        let mut function_scope = scope.clone();
        for (index, parameter) in function.parameters.iter().enumerate() {
            let raw = arguments
                .get(index)
                .cloned()
                .or_else(|| parameter.default.as_ref().map(|value| value.raw.clone()))
                .unwrap_or_default();
            let evaluated = self.evaluate_text(&raw, call_range, scope, frames, false);
            function_scope.variables.insert(
                normalize_name(&parameter.name),
                VariableBinding {
                    value: evaluated.text,
                    origins: evaluated.origins,
                },
            );
        }
        let mut function_frames = frames.to_vec();
        function_frames.push(OriginSpec {
            kind: ScssOriginKind::Definition,
            name: Some(function.name.clone()),
            range: function.source_range,
        });
        function_frames.push(OriginSpec {
            kind: ScssOriginKind::CallSite,
            name: Some(function.name.clone()),
            range: call_range,
        });
        for statement in &function.body {
            match statement {
                ScssStatement::Variable(value) => {
                    self.bind_variable(value, &mut function_scope, &function_frames)
                }
                ScssStatement::Return(value, _) => {
                    return self.evaluate_expression(value, &function_scope, &function_frames)
                }
                ScssStatement::If(value) => {
                    let condition = self.evaluate_expression(
                        &value.condition,
                        &function_scope,
                        &function_frames,
                    );
                    let branch = if is_truthy(&condition.text) {
                        &value.body
                    } else {
                        &value.else_body
                    };
                    for nested in branch {
                        if let ScssStatement::Return(value, _) = nested {
                            return self.evaluate_expression(
                                value,
                                &function_scope,
                                &function_frames,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        self.diagnostics.push(scss_diagnostic(
            &self.stylesheet.source.uri,
            "cem.scss.module_error",
            Severity::Error,
            format!(
                "SCSS function `{}` completed without @return",
                function.name
            ),
            Some(function.source_range),
        ));
        EvaluatedValue {
            text: String::new(),
            origins: function_frames,
        }
    }

    fn css_document(&self, rules: Vec<GeneratedRule>) -> CssDocumentAst {
        let mut events = Vec::new();
        for rule in rules {
            let selector_map = self.source_map(rule.source_range, &rule.selector_origins);
            push_css_event(
                &mut events,
                0,
                "token",
                "ident",
                &rule.selector,
                rule.source_range,
                selector_map.clone(),
            );
            push_css_event(
                &mut events,
                0,
                "trivia",
                "whitespace",
                " ",
                rule.source_range,
                selector_map.clone(),
            );
            push_css_event(
                &mut events,
                0,
                "block-open",
                "curly-open",
                "{",
                rule.source_range,
                selector_map.clone(),
            );
            push_css_event(
                &mut events,
                1,
                "trivia",
                "whitespace",
                "\n",
                rule.source_range,
                selector_map,
            );
            for declaration in rule.declarations {
                let source_map = self.source_map(declaration.source_range, &declaration.origins);
                push_css_event(
                    &mut events,
                    1,
                    "trivia",
                    "whitespace",
                    "  ",
                    declaration.source_range,
                    source_map.clone(),
                );
                push_css_event(
                    &mut events,
                    1,
                    "token",
                    "ident",
                    &declaration.name,
                    declaration.source_range,
                    source_map.clone(),
                );
                push_css_event(
                    &mut events,
                    1,
                    "token",
                    "colon",
                    ":",
                    declaration.source_range,
                    source_map.clone(),
                );
                push_css_event(
                    &mut events,
                    1,
                    "trivia",
                    "whitespace",
                    " ",
                    declaration.source_range,
                    source_map.clone(),
                );
                push_css_event(
                    &mut events,
                    1,
                    "token",
                    "raw",
                    &declaration.value,
                    declaration.source_range,
                    source_map.clone(),
                );
                push_css_event(
                    &mut events,
                    1,
                    "token",
                    "semicolon",
                    ";",
                    declaration.source_range,
                    source_map.clone(),
                );
                push_css_event(
                    &mut events,
                    1,
                    "trivia",
                    "whitespace",
                    "\n",
                    declaration.source_range,
                    source_map,
                );
            }
            let closing_map = self.source_map(rule.source_range, &[]);
            push_css_event(
                &mut events,
                0,
                "block-close",
                "curly-close",
                "}",
                rule.source_range,
                closing_map.clone(),
            );
            push_css_event(
                &mut events,
                0,
                "trivia",
                "whitespace",
                "\n",
                rule.source_range,
                closing_map,
            );
        }
        let byte_length = events.iter().map(|event| event.lexeme.len()).sum();
        CssDocumentAst {
            source: CssDocumentSource {
                uri: self.stylesheet.source.uri.clone(),
                content_type: CSS_CONTENT_TYPE.to_owned(),
                media_type: CSS_CONTENT_TYPE.to_owned(),
                parameters: BTreeMap::new(),
                byte_length,
            },
            entry_mode: CssEntryMode::Stylesheet,
            encoding_report: CssEncodingReportAst {
                mime_charset: Some("utf-8".to_owned()),
                stylesheet_charset: None,
                bom: None,
                normalized_encoding: "utf-8".to_owned(),
                decoder_status: "generated-utf8".to_owned(),
            },
            events,
            facts: Vec::new(),
            line_ending: self.stylesheet.line_ending.clone(),
            recovery_count: 0,
        }
    }

    fn source_map(&self, range: ScssSourceRange, origins: &[OriginSpec]) -> SourceMapStack {
        let mut frames = vec![
            scss_origin_frame(
                &self.stylesheet.source.uri,
                ScssOriginKind::Source,
                None,
                range,
            ),
            scss_origin_frame(
                &self.stylesheet.source.uri,
                ScssOriginKind::Module,
                None,
                range,
            ),
        ];
        for origin in origins {
            frames.push(scss_origin_frame(
                &self.stylesheet.source.uri,
                origin.kind,
                origin.name.clone(),
                origin.range,
            ));
        }
        frames.push(scss_origin_frame(
            &self.stylesheet.source.uri,
            ScssOriginKind::Expansion,
            None,
            range,
        ));
        SourceMapStack { frames }
    }
}

fn push_css_event(
    events: &mut Vec<CssEventAst>,
    depth: usize,
    kind: &str,
    token_kind: &str,
    lexeme: &str,
    _origin_range: ScssSourceRange,
    source_map: SourceMapStack,
) {
    let source_range = next_generated_css_range(events, lexeme);
    events.push(CssEventAst {
        index: events.len(),
        depth,
        kind: kind.to_owned(),
        token_kind: token_kind.to_owned(),
        value: None,
        lexeme: lexeme.to_owned(),
        recovered: false,
        source_range,
        source_map,
    });
}

fn next_generated_css_range(events: &[CssEventAst], lexeme: &str) -> CssSourceRange {
    let (byte_offset, line, column) = events.last().map_or((0, 1, 1), |previous| {
        let mut line = previous.source_range.start.line;
        let mut column = previous.source_range.start.column;
        for byte in previous.lexeme.bytes() {
            if byte == b'\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (
            previous.source_range.start.byte_offset + previous.lexeme.len() as u64,
            line,
            column,
        )
    });
    CssSourceRange {
        start: CssSourcePosition {
            line,
            column,
            byte_offset,
        },
        byte_length: lexeme.len() as u64,
    }
}

fn scss_origin_frame(
    module_uri: &str,
    origin_kind: ScssOriginKind,
    name: Option<String>,
    range: ScssSourceRange,
) -> SourceMapFrame {
    SourceMapFrame {
        source_id: SourceId(1),
        span: FrameSpan::Single(range.byte_range()),
        transform: TransformKind::ScssOrigin {
            origin_kind,
            module_uri: module_uri.to_owned(),
            name,
        },
    }
}

fn substitute_variables(
    input: &str,
    scope: &EvaluationScope,
    origins: &mut Vec<OriginSpec>,
) -> String {
    let mut output = String::new();
    let mut cursor = 0usize;
    while cursor < input.len() {
        let current = input[cursor..].chars().next().unwrap();
        if current != '$' {
            output.push(current);
            cursor += current.len_utf8();
            continue;
        }
        let name_start = cursor + 1;
        let mut name_end = name_start;
        while name_end < input.len() {
            let value = input[name_end..].chars().next().unwrap();
            if !is_identifier_continue(value) {
                break;
            }
            name_end += value.len_utf8();
        }
        let name = &input[name_start..name_end];
        if let Some(binding) = scope.variables.get(&normalize_name(name)) {
            output.push_str(&binding.value);
            origins.extend(binding.origins.clone());
        } else {
            output.push_str(&input[cursor..name_end]);
        }
        cursor = name_end;
    }
    output
}

fn expand_interpolations(
    input: &str,
    range: ScssSourceRange,
    mut evaluate: impl FnMut(&str) -> EvaluatedValue,
    origins: &mut Vec<OriginSpec>,
) -> String {
    let mut output = String::new();
    let mut cursor = 0usize;
    while let Some(relative) = input[cursor..].find("#{") {
        let start = cursor + relative;
        output.push_str(&input[cursor..start]);
        let Some(close) = matching_brace(input, start + 1) else {
            output.push_str(&input[start..]);
            return output;
        };
        let evaluated = evaluate(&input[start + 2..close]);
        output.push_str(unquote(evaluated.text.trim()));
        origins.extend(evaluated.origins);
        origins.push(OriginSpec {
            kind: ScssOriginKind::Interpolation,
            name: None,
            range,
        });
        cursor = close + 1;
    }
    output.push_str(&input[cursor..]);
    output
}

fn next_function_call(input: &str, start: usize) -> Option<(usize, usize, usize, usize)> {
    let mut cursor = start;
    while cursor < input.len() {
        let current = input[cursor..].chars().next()?;
        if is_identifier_start(current) {
            let name_start = cursor;
            cursor += current.len_utf8();
            while cursor < input.len() {
                let value = input[cursor..].chars().next()?;
                if !is_identifier_continue(value) {
                    break;
                }
                cursor += value.len_utf8();
            }
            let name_end = cursor;
            while cursor < input.len()
                && input[cursor..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                cursor += input[cursor..].chars().next()?.len_utf8();
            }
            if input[cursor..].starts_with('(') {
                let close = matching_parenthesis(input, cursor)?;
                return Some((name_start, name_end, cursor, close));
            }
        } else {
            cursor += current.len_utf8();
        }
    }
    None
}

fn matching_parenthesis(input: &str, open: usize) -> Option<usize> {
    matching_delimiter(input, open, '(', ')')
}

fn matching_brace(input: &str, open: usize) -> Option<usize> {
    matching_delimiter(input, open, '{', '}')
}

fn matching_delimiter(input: &str, open: usize, opener: char, closer: char) -> Option<usize> {
    let mut depth = 0usize;
    for (relative, value) in input[open..].char_indices() {
        if value == opener {
            depth += 1;
        } else if value == closer {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(open + relative);
            }
        }
    }
    None
}

fn split_expression_text(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, value) in input.char_indices() {
        match value {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                values.push(input[start..index].trim().to_owned());
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < input.len() || !input.trim().is_empty() {
        values.push(input[start..].trim().to_owned());
    }
    values
}

fn evaluate_arithmetic(input: &str) -> Option<String> {
    for operator in ['*', '/', '+', '-'] {
        let Some(index) = find_top_level_operator(input, operator) else {
            continue;
        };
        let (left, right) = (input[..index].trim(), input[index + 1..].trim());
        let (left_number, left_unit) = parse_numeric(left)?;
        let (right_number, right_unit) = parse_numeric(right)?;
        let (result, unit) = match operator {
            '*' if left_unit.is_empty() => (left_number * right_number, right_unit),
            '*' if right_unit.is_empty() => (left_number * right_number, left_unit),
            '/' if right_unit.is_empty() && right_number != 0.0 => {
                (left_number / right_number, left_unit)
            }
            '+' if left_unit == right_unit => (left_number + right_number, left_unit),
            '-' if left_unit == right_unit => (left_number - right_number, left_unit),
            _ => return None,
        };
        return Some(format_number(result, &unit));
    }
    None
}

fn find_top_level_operator(input: &str, operator: char) -> Option<usize> {
    let mut depth = 0usize;
    for (index, value) in input.char_indices() {
        match value {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if value == operator && depth == 0 && index > 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn parse_numeric(input: &str) -> Option<(f64, String)> {
    let input = input.trim();
    let mut end = 0usize;
    for (index, value) in input.char_indices() {
        if value.is_ascii_digit() || value == '.' || (index == 0 && matches!(value, '+' | '-')) {
            end = index + value.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    Some((input[..end].parse().ok()?, input[end..].to_owned()))
}

fn format_number(value: f64, unit: &str) -> String {
    if value.fract() == 0.0 {
        format!("{}{unit}", value as i64)
    } else {
        format!("{value}{unit}")
    }
}

fn combine_selectors(parent: Option<&str>, child: &str) -> String {
    match parent {
        Some(parent) if child.contains('&') => child.replace('&', parent),
        Some(parent) => format!("{} {}", parent.trim(), child.trim()),
        None => child.trim().to_owned(),
    }
}

fn normalize_name(value: &str) -> String {
    value.trim().replace('_', "-")
}

fn is_truthy(value: &str) -> bool {
    !matches!(value.trim(), "" | "false" | "null")
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn scss_diagnostic(
    uri: &str,
    code: &str,
    severity: Severity,
    message: impl Into<String>,
    range: Option<ScssSourceRange>,
) -> Diagnostic {
    let (line, column, byte_offset) = range
        .map(|range| {
            (
                Some(range.start.line),
                Some(range.start.column),
                Some(range.start.byte_offset),
            )
        })
        .unwrap_or((None, None, None));
    Diagnostic {
        uri: Some(uri.to_owned()),
        line,
        column,
        byte_offset,
        code: code.to_owned(),
        severity,
        message: message.into(),
        details: Some(serde_json::json!({
            "schema": SCSS_SCHEMA_URI,
            "schemaPackage": "scss",
            "sourceRange": range.map(|range| serde_json::json!({
                "byteOffset": range.start.byte_offset,
                "byteLength": range.byte_length,
                "line": range.start.line,
                "column": range.start.column,
            })),
        })),
        source_map: range.map(|range| SourceMapStack {
            frames: vec![scss_origin_frame(uri, ScssOriginKind::Source, None, range)],
        }),
        ..Diagnostic::default()
    }
}

fn content_type_parameter(content_type: &str, target: &str) -> Option<String> {
    content_type.split(';').skip(1).find_map(|parameter| {
        let (name, value) = parameter.split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case(target)
            .then(|| value.trim().trim_matches('"').to_owned())
    })
}

fn normalize_charset(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "utf8" | "utf-8" => "utf-8".to_owned(),
        other => other.to_owned(),
    }
}

fn detect_line_ending(source: &str) -> Option<String> {
    let has_crlf = source.contains("\r\n");
    let without_crlf = source.replace("\r\n", "");
    let has_lf = without_crlf.contains('\n');
    let has_cr = without_crlf.contains('\r');
    match (has_crlf, has_lf, has_cr) {
        (false, false, false) => None,
        (true, false, false) => Some("crlf".to_owned()),
        (false, true, false) => Some("lf".to_owned()),
        (false, false, true) => Some("cr".to_owned()),
        _ => Some("mixed".to_owned()),
    }
}
