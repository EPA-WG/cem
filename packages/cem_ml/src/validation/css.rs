use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{content_type_essence, CSS_CONTENT_TYPE, CSS_SCHEMA_URI};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cssparser::{Parser, ParserInput, Token};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const CSS_PACKAGE_ID: &str = "css";
const CSS_FACT_BEHAVIOR: &str = "css-report-fact";

#[derive(Debug, Clone, Copy)]
pub struct CssSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssDocumentAst {
    pub source: CssDocumentSource,
    pub entry_mode: CssEntryMode,
    pub encoding_report: CssEncodingReportAst,
    pub events: Vec<CssEventAst>,
    pub facts: Vec<CssFact>,
    pub line_ending: Option<String>,
    pub recovery_count: usize,
}

impl CssDocumentAst {
    #[cfg(test)]
    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": "css-document",
            "contentType": self.source.media_type,
            "schema": CSS_SCHEMA_URI,
            "category": "css-document",
            "source": self.source.to_cemt_subject(),
            "entryMode": self.entry_mode.as_str(),
            "encodingReport": self.encoding_report.to_cemt_subject(),
            "parseFacts": self.facts.iter().map(CssFact::to_cemt_subject).collect::<Vec<_>>(),
            "events": self.events.iter().map(CssEventAst::to_cemt_subject).collect::<Vec<_>>(),
            "lineEnding": self.line_ending,
            "recoveryCount": self.recovery_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl CssDocumentSource {
    fn from_request(
        request: CssSourceValidationRequest<'_>,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        let content_type = request.content_type.unwrap_or(CSS_CONTENT_TYPE);
        Self {
            uri: request.source_uri.to_owned(),
            content_type: content_type.to_owned(),
            media_type: content_type_essence(content_type),
            parameters,
            byte_length: request.bytes.len(),
        }
    }

    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "uri": self.uri,
            "contentType": self.content_type,
            "mediaType": self.media_type,
            "parameters": self.parameters,
            "byteLength": self.byte_length,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssEntryMode {
    Stylesheet,
    DeclarationList,
    ScopedStyleBlock,
}

impl CssEntryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stylesheet => "stylesheet",
            Self::DeclarationList => "declaration-list",
            Self::ScopedStyleBlock => "scoped-style-block",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssEncodingReportAst {
    pub mime_charset: Option<String>,
    pub stylesheet_charset: Option<String>,
    pub bom: Option<String>,
    pub normalized_encoding: String,
    pub decoder_status: String,
}

impl CssEncodingReportAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "mimeCharset": self.mime_charset,
            "stylesheetCharset": self.stylesheet_charset,
            "bom": self.bom,
            "normalizedEncoding": self.normalized_encoding,
            "decoderStatus": self.decoder_status,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssEventAst {
    pub index: usize,
    pub depth: usize,
    pub kind: String,
    pub token_kind: String,
    pub value: Option<String>,
    pub lexeme: String,
    pub recovered: bool,
    pub source_range: CssSourceRange,
    pub source_map: SourceMapStack,
}

impl CssEventAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "depth": self.depth,
            "kind": self.kind,
            "tokenKind": self.token_kind,
            "value": self.value,
            "lexeme": self.lexeme,
            "recovered": self.recovered,
            "sourceRange": self.source_range.to_cemt_subject(),
            "sourceMap": serde_json::to_value(&self.source_map).unwrap_or(Value::Null),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssSourceRange {
    pub start: CssSourcePosition,
    pub byte_length: u64,
}

impl CssSourceRange {
    fn from_offsets(line_index: &LineIndex, start: usize, end: usize) -> Self {
        let coordinate = line_index.project(start as u64);
        Self {
            start: CssSourcePosition {
                line: coordinate.line,
                column: coordinate.column,
                byte_offset: start as u64,
            },
            byte_length: end.saturating_sub(start) as u64,
        }
    }

    fn byte_range(self) -> ByteRange {
        ByteRange::new(
            self.start.byte_offset,
            u32::try_from(self.byte_length).unwrap_or(u32::MAX),
        )
    }

    pub fn source_map(self) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(self.byte_range()),
                transform: TransformKind::ContentTypeTransform {
                    content_type: CSS_CONTENT_TYPE.to_owned(),
                },
            }],
        }
    }

    #[cfg(test)]
    fn to_cemt_subject(self) -> Value {
        json!({
            "byteOffset": self.start.byte_offset,
            "byteLength": self.byte_length,
            "line": self.start.line,
            "column": self.start.column,
        })
    }
}

fn css_source_range_diagnostic_value(range: CssSourceRange) -> Value {
    json!({
        "byteOffset": range.start.byte_offset,
        "byteLength": range.byte_length,
        "line": range.start.line,
        "column": range.start.column,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CssFactKind {
    ParseError,
    InvalidToken,
    BadString,
    BadUrl,
    EncodingConflict,
    InvalidSelector,
    InvalidDeclaration,
    UnknownAtRule,
    ImportRejected,
    UrlRejected,
    SourceMapUnavailable,
    StylesheetObserved,
    DeclarationListObserved,
    ScopedStyleBlockObserved,
    EncodingObserved,
    CommentObserved,
    CustomPropertyObserved,
    VendorSyntaxObserved,
    RecoveryObserved,
}

impl CssFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseError => "parse-error",
            Self::InvalidToken => "invalid-token",
            Self::BadString => "bad-string",
            Self::BadUrl => "bad-url",
            Self::EncodingConflict => "encoding-conflict",
            Self::InvalidSelector => "invalid-selector",
            Self::InvalidDeclaration => "invalid-declaration",
            Self::UnknownAtRule => "unknown-at-rule",
            Self::ImportRejected => "import-rejected",
            Self::UrlRejected => "url-rejected",
            Self::SourceMapUnavailable => "source-map-unavailable",
            Self::StylesheetObserved => "stylesheet-observed",
            Self::DeclarationListObserved => "declaration-list-observed",
            Self::ScopedStyleBlockObserved => "scoped-style-block-observed",
            Self::EncodingObserved => "encoding-observed",
            Self::CommentObserved => "comment-observed",
            Self::CustomPropertyObserved => "custom-property-observed",
            Self::VendorSyntaxObserved => "vendor-syntax-observed",
            Self::RecoveryObserved => "recovery-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssFact {
    pub kind: CssFactKind,
    pub source_range: Option<CssSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

impl CssFact {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "sourceRange": self.source_range.map(CssSourceRange::to_cemt_subject),
            "message": self.message,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CssDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSchemaContractCatalog {
    fact_bindings: BTreeMap<String, CssDiagnosticBinding>,
}

impl CssSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<CssSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(CSS_PACKAGE_ID)
                .expect("built-in CSS schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(CSS_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != CSS_FACT_BEHAVIOR {
                    return None;
                }
                let fact_kind = constraint.fact_kind.as_deref()?.trim();
                let diagnostic_code = constraint.diagnostic.as_deref()?.trim();
                if fact_kind.is_empty() || diagnostic_code.is_empty() {
                    return None;
                }
                let diagnostic = model.diagnostics.get(diagnostic_code)?;
                Some((
                    fact_kind.to_owned(),
                    CssDiagnosticBinding {
                        contract: constraint.kind.clone(),
                        behavior: constraint.behavior.clone(),
                        diagnostic_code: diagnostic.code.clone(),
                        severity: diagnostic.severity,
                        policy: constraint.policy.clone(),
                    },
                ))
            })
            .collect();
        Self { fact_bindings }
    }

    fn binding_for_fact(&self, kind: CssFactKind) -> Option<&CssDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_css_source_bytes(request: CssSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let (_, diagnostics) = css_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn validate_css_document_ast(document: &CssDocumentAst) -> Vec<Diagnostic> {
    let mut facts = document.facts.clone();
    for event in &document.events {
        if event.source_map.frames.is_empty() {
            push_fact_once(
                &mut facts,
                CssFactKind::SourceMapUnavailable,
                Some(event.source_range),
                "CSS lifecycle event is missing its source-map origin",
                None,
            );
        }
        if event.token_kind == "raw" {
            collect_generated_component_value_facts(&event.lexeme, event.source_range, &mut facts);
        }
    }

    let mut diagnostics = css_fact_diagnostics(
        &document.source.uri,
        &document.source.media_type,
        &facts,
        CssSchemaContractCatalog::from_builtin(),
    );
    for diagnostic in &mut diagnostics {
        let Some(byte_offset) = diagnostic.byte_offset else {
            continue;
        };
        if let Some(event) = document.events.iter().find(|event| {
            let start = event.source_range.start.byte_offset;
            let end = start.saturating_add(event.source_range.byte_length.max(1));
            (start..end).contains(&byte_offset)
        }) {
            diagnostic.source_map = Some(event.source_map.clone());
        }
    }
    diagnostics
}

pub fn css_document_ast_from_source_bytes(
    request: CssSourceValidationRequest<'_>,
) -> (Option<CssDocumentAst>, Vec<Diagnostic>) {
    let parameters =
        parse_content_type_parameters(request.content_type.unwrap_or(CSS_CONTENT_TYPE));
    let source_info = CssDocumentSource::from_request(request, parameters.clone());
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            let fact = CssFact {
                kind: CssFactKind::ParseError,
                source_range: None,
                message: format!(
                    "CSS document bytes are not valid UTF-8 at byte {}: {error}",
                    error.valid_up_to()
                ),
                value: parameters.get("charset").cloned(),
            };
            return (
                None,
                css_fact_diagnostics(
                    request.source_uri,
                    &source_info.media_type,
                    &[fact],
                    CssSchemaContractCatalog::from_builtin(),
                ),
            );
        }
    };

    let line_index = LineIndex::from_utf8(source);
    let mut parser_input = ParserInput::new(source);
    let mut parser = Parser::new(&mut parser_input);
    let mut events = Vec::new();
    let mut facts = Vec::new();
    collect_cssparser_events(&mut parser, source, &line_index, 0, &mut events, &mut facts);
    normalize_lossless_events(source, &line_index, &mut events);

    let declared_charset = parameters.get("charset").cloned();
    let entry_mode = infer_entry_mode(&parameters, source, &events);
    let analysis_facts =
        collect_css_policy_facts(source, &line_index, declared_charset.as_deref(), entry_mode);
    for fact in analysis_facts {
        push_fact_once(
            &mut facts,
            fact.kind,
            fact.source_range,
            &fact.message,
            fact.value,
        );
    }

    facts.push(CssFact {
        kind: match entry_mode {
            CssEntryMode::Stylesheet => CssFactKind::StylesheetObserved,
            CssEntryMode::DeclarationList => CssFactKind::DeclarationListObserved,
            CssEntryMode::ScopedStyleBlock => CssFactKind::ScopedStyleBlockObserved,
        },
        source_range: events.first().map(|event| event.source_range),
        message: format!("CSS {} entry mode selected", entry_mode.as_str()),
        value: Some(entry_mode.as_str().to_owned()),
    });
    if events.iter().any(|event| event.token_kind == "comment") {
        facts.push(CssFact {
            kind: CssFactKind::CommentObserved,
            source_range: events
                .iter()
                .find(|event| event.token_kind == "comment")
                .map(|event| event.source_range),
            message: "CSS comments are preserved in the lossless event stream".to_owned(),
            value: None,
        });
    }
    if source.contains("--") {
        facts.push(CssFact {
            kind: CssFactKind::CustomPropertyObserved,
            source_range: None,
            message: "CSS custom-property syntax is preserved without value normalization"
                .to_owned(),
            value: None,
        });
    }
    if source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ':' | ';' | '{' | '}'))
        .any(|part| part.starts_with('-') && !part.starts_with("--"))
    {
        facts.push(CssFact {
            kind: CssFactKind::VendorSyntaxObserved,
            source_range: None,
            message: "CSS vendor-prefixed syntax is preserved verbatim".to_owned(),
            value: None,
        });
    }

    let stylesheet_charset = css_leading_charset(source).map(|(charset, _)| charset);
    let normalized_encoding = declared_charset
        .as_deref()
        .map(css_normalized_charset)
        .filter(|value| !value.is_empty())
        .or_else(|| stylesheet_charset.as_deref().map(css_normalized_charset))
        .unwrap_or_else(|| "utf-8".to_owned());
    facts.push(CssFact {
        kind: CssFactKind::EncodingObserved,
        source_range: None,
        message: format!("CSS decoder used `{normalized_encoding}` identity metadata"),
        value: Some(normalized_encoding.clone()),
    });

    let recovery_count = facts
        .iter()
        .filter(|fact| {
            matches!(
                fact.kind,
                CssFactKind::ParseError
                    | CssFactKind::InvalidToken
                    | CssFactKind::BadString
                    | CssFactKind::BadUrl
                    | CssFactKind::InvalidSelector
                    | CssFactKind::InvalidDeclaration
            )
        })
        .count();
    if recovery_count > 0 {
        facts.push(CssFact {
            kind: CssFactKind::RecoveryObserved,
            source_range: None,
            message: format!("CSS parser recovery recorded {recovery_count} fact(s)"),
            value: Some(recovery_count.to_string()),
        });
    }

    let ast = CssDocumentAst {
        source: source_info,
        entry_mode,
        encoding_report: CssEncodingReportAst {
            mime_charset: declared_charset,
            stylesheet_charset,
            bom: source.strip_prefix('\u{feff}').map(|_| "utf-8".to_owned()),
            normalized_encoding,
            decoder_status: "decoded-utf8".to_owned(),
        },
        events,
        facts,
        line_ending: detect_line_ending(source),
        recovery_count,
    };
    let diagnostics = validate_css_document_ast(&ast);
    (Some(ast), diagnostics)
}

#[derive(Debug)]
struct CssTokenMetadata {
    token_kind: &'static str,
    value: Option<String>,
    recovered: bool,
    closing: Option<(&'static str, &'static str)>,
}

fn collect_cssparser_events<'i, 't>(
    parser: &mut Parser<'i, 't>,
    source: &'i str,
    line_index: &LineIndex,
    depth: usize,
    events: &mut Vec<CssEventAst>,
    facts: &mut Vec<CssFact>,
) {
    loop {
        let start = parser.position().byte_index();
        let token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => break,
        };
        let end = parser.position().byte_index();
        let metadata = css_token_metadata(&token);
        let source_range = CssSourceRange::from_offsets(line_index, start, end);
        events.push(CssEventAst {
            index: events.len(),
            depth,
            kind: if metadata.closing.is_some() {
                "block-open".to_owned()
            } else if metadata.token_kind == "whitespace" || metadata.token_kind == "comment" {
                "trivia".to_owned()
            } else {
                "token".to_owned()
            },
            token_kind: metadata.token_kind.to_owned(),
            value: metadata.value.clone(),
            lexeme: source.get(start..end).unwrap_or_default().to_owned(),
            recovered: metadata.recovered,
            source_range,
            source_map: source_range.source_map(),
        });

        match token {
            Token::BadString(_) => push_fact_once(
                facts,
                CssFactKind::BadString,
                Some(source_range),
                "CSS string token was recovered before a matching quote",
                metadata.value.clone(),
            ),
            Token::BadUrl(_) => push_fact_once(
                facts,
                CssFactKind::BadUrl,
                Some(source_range),
                "CSS url() token was recovered before a valid closing delimiter",
                metadata.value.clone(),
            ),
            Token::CloseParenthesis | Token::CloseSquareBracket | Token::CloseCurlyBracket => {
                push_fact_once(
                    facts,
                    CssFactKind::InvalidToken,
                    Some(source_range),
                    "CSS closing delimiter does not match an open component-value block",
                    Some(
                        events
                            .last()
                            .map_or_else(String::new, |event| event.lexeme.clone()),
                    ),
                );
            }
            _ => {}
        }

        if let Some((closing_kind, closing_lexeme)) = metadata.closing {
            let mut nested_end = end;
            let nested_result = parser.parse_nested_block(|nested| {
                collect_cssparser_events(
                    nested,
                    source,
                    line_index,
                    depth.saturating_add(1),
                    events,
                    facts,
                );
                nested_end = nested.position().byte_index();
                Ok::<(), cssparser::ParseError<'i, ()>>(())
            });
            let outer_end = parser.position().byte_index();
            if outer_end > nested_end
                && source
                    .get(nested_end..outer_end)
                    .is_some_and(|slice| slice.ends_with(closing_lexeme))
            {
                let closing_start = outer_end.saturating_sub(closing_lexeme.len());
                let source_range =
                    CssSourceRange::from_offsets(line_index, closing_start, outer_end);
                events.push(CssEventAst {
                    index: events.len(),
                    depth,
                    kind: "block-close".to_owned(),
                    token_kind: closing_kind.to_owned(),
                    value: None,
                    lexeme: source
                        .get(closing_start..outer_end)
                        .unwrap_or(closing_lexeme)
                        .to_owned(),
                    recovered: nested_result.is_err(),
                    source_range,
                    source_map: source_range.source_map(),
                });
            }
        }
    }
}

pub(crate) fn css_syntax_lossless_events(source: &str) -> (Vec<CssEventAst>, Vec<CssFact>) {
    let line_index = LineIndex::from_utf8(source);
    let mut parser_input = ParserInput::new(source);
    let mut parser = Parser::new(&mut parser_input);
    let mut events = Vec::new();
    let mut facts = Vec::new();
    collect_cssparser_events(&mut parser, source, &line_index, 0, &mut events, &mut facts);
    normalize_lossless_events(source, &line_index, &mut events);
    (events, facts)
}

fn css_token_metadata(token: &Token<'_>) -> CssTokenMetadata {
    let (token_kind, value, recovered, closing) = match token {
        Token::Ident(value) => ("ident", Some(value.to_string()), false, None),
        Token::AtKeyword(value) => ("at-keyword", Some(value.to_string()), false, None),
        Token::Hash(value) => ("hash", Some(value.to_string()), false, None),
        Token::IDHash(value) => ("id-hash", Some(value.to_string()), false, None),
        Token::QuotedString(value) => ("string", Some(value.to_string()), false, None),
        Token::UnquotedUrl(value) => ("url", Some(value.to_string()), false, None),
        Token::Delim(value) => ("delimiter", Some(value.to_string()), false, None),
        Token::Number { value, .. } => ("number", Some(value.to_string()), false, None),
        Token::Percentage { unit_value, .. } => {
            ("percentage", Some(unit_value.to_string()), false, None)
        }
        Token::Dimension { value, unit, .. } => {
            ("dimension", Some(format!("{value}{unit}")), false, None)
        }
        Token::WhiteSpace(value) => ("whitespace", Some((*value).to_owned()), false, None),
        Token::Comment(value) => ("comment", Some((*value).to_owned()), false, None),
        Token::Colon => ("colon", None, false, None),
        Token::Semicolon => ("semicolon", None, false, None),
        Token::Comma => ("comma", None, false, None),
        Token::IncludeMatch => ("include-match", None, false, None),
        Token::DashMatch => ("dash-match", None, false, None),
        Token::PrefixMatch => ("prefix-match", None, false, None),
        Token::SuffixMatch => ("suffix-match", None, false, None),
        Token::SubstringMatch => ("substring-match", None, false, None),
        Token::CDO => ("cdo", None, false, None),
        Token::CDC => ("cdc", None, false, None),
        Token::Function(value) => (
            "function-open",
            Some(value.to_string()),
            false,
            Some(("parenthesis-close", ")")),
        ),
        Token::ParenthesisBlock => (
            "parenthesis-open",
            None,
            false,
            Some(("parenthesis-close", ")")),
        ),
        Token::SquareBracketBlock => ("square-open", None, false, Some(("square-close", "]"))),
        Token::CurlyBracketBlock => ("curly-open", None, false, Some(("curly-close", "}"))),
        Token::BadUrl(value) => ("bad-url", Some(value.to_string()), true, None),
        Token::BadString(value) => ("bad-string", Some(value.to_string()), true, None),
        Token::CloseParenthesis => ("unmatched-parenthesis-close", None, true, None),
        Token::CloseSquareBracket => ("unmatched-square-close", None, true, None),
        Token::CloseCurlyBracket => ("unmatched-curly-close", None, true, None),
    };
    CssTokenMetadata {
        token_kind,
        value,
        recovered,
        closing,
    }
}

fn normalize_lossless_events(source: &str, line_index: &LineIndex, events: &mut Vec<CssEventAst>) {
    events.sort_by_key(|event| event.source_range.start.byte_offset);
    let parsed_events = std::mem::take(events);
    let mut normalized = Vec::with_capacity(parsed_events.len());
    let mut cursor = 0usize;
    for mut event in parsed_events {
        let start = event.source_range.start.byte_offset as usize;
        let end = start.saturating_add(event.source_range.byte_length as usize);
        if start > cursor {
            let source_range = CssSourceRange::from_offsets(line_index, cursor, start);
            normalized.push(CssEventAst {
                index: normalized.len(),
                depth: event.depth,
                kind: "trivia".to_owned(),
                token_kind: "presentation-gap".to_owned(),
                value: None,
                lexeme: source.get(cursor..start).unwrap_or_default().to_owned(),
                recovered: false,
                source_range,
                source_map: source_range.source_map(),
            });
        }
        if start >= cursor {
            event.index = normalized.len();
            normalized.push(event);
            cursor = end;
        }
    }
    if cursor < source.len() {
        let source_range = CssSourceRange::from_offsets(line_index, cursor, source.len());
        normalized.push(CssEventAst {
            index: normalized.len(),
            depth: 0,
            kind: "trivia".to_owned(),
            token_kind: "presentation-gap".to_owned(),
            value: None,
            lexeme: source[cursor..].to_owned(),
            recovered: false,
            source_range,
            source_map: source_range.source_map(),
        });
    }
    debug_assert_eq!(
        normalized
            .iter()
            .map(|event| event.lexeme.as_str())
            .collect::<String>(),
        source
    );
    *events = normalized;
}

fn infer_entry_mode(
    parameters: &BTreeMap<String, String>,
    source: &str,
    events: &[CssEventAst],
) -> CssEntryMode {
    let explicit_mode = parameters
        .get("mode")
        .or_else(|| parameters.get("entry"))
        .map(|value| value.trim().to_ascii_lowercase());
    match explicit_mode.as_deref() {
        Some("style-attribute" | "declaration-list") => return CssEntryMode::DeclarationList,
        Some("scoped" | "style-block" | "scoped-style-block") => {
            return CssEntryMode::ScopedStyleBlock
        }
        _ => {}
    }
    if parameters.contains_key("scope")
        || source.to_ascii_lowercase().contains(":host")
        || source.to_ascii_lowercase().contains("@scope")
    {
        return CssEntryMode::ScopedStyleBlock;
    }
    if !events
        .iter()
        .any(|event| event.depth == 0 && event.token_kind == "curly-open")
        && source.contains(':')
    {
        return CssEntryMode::DeclarationList;
    }
    CssEntryMode::Stylesheet
}

fn css_fact_diagnostics(
    source_uri: &str,
    media_type: &str,
    facts: &[CssFact],
    catalog: &CssSchemaContractCatalog,
) -> Vec<Diagnostic> {
    facts
        .iter()
        .filter_map(|fact| {
            let binding = catalog.binding_for_fact(fact.kind)?;
            let (line, column, byte_offset) = fact
                .source_range
                .map(|range| {
                    (
                        Some(range.start.line),
                        Some(range.start.column),
                        Some(range.start.byte_offset),
                    )
                })
                .unwrap_or((None, None, None));
            Some(Diagnostic {
                uri: Some(source_uri.to_owned()),
                line,
                column,
                byte_offset,
                code: binding.diagnostic_code.clone(),
                severity: binding.severity,
                message: fact.message.clone(),
                details: Some(json!({
                    "schema": CSS_SCHEMA_URI,
                    "schemaPackage": CSS_PACKAGE_ID,
                    "schemaConstraint": binding.contract,
                    "schemaBehavior": binding.behavior,
                    "schemaPolicy": binding.policy,
                    "factKind": fact.kind.as_str(),
                    "factValue": fact.value,
                    "contentType": media_type,
                    "sourceRange": fact.source_range.map(css_source_range_diagnostic_value),
                })),
                source_map: fact.source_range.map(CssSourceRange::source_map),
                ..Diagnostic::default()
            })
        })
        .collect()
}

fn push_fact_once(
    facts: &mut Vec<CssFact>,
    kind: CssFactKind,
    source_range: Option<CssSourceRange>,
    message: &str,
    value: Option<String>,
) {
    if facts.iter().any(|fact| fact.kind == kind) {
        return;
    }
    facts.push(CssFact {
        kind,
        source_range,
        message: message.to_owned(),
        value,
    });
}

fn parse_content_type_parameters(content_type: &str) -> BTreeMap<String, String> {
    content_type
        .split(';')
        .skip(1)
        .filter_map(|part| {
            let (name, value) = part.split_once('=')?;
            let name = name.trim().to_ascii_lowercase();
            if name.is_empty() {
                return None;
            }
            Some((name, value.trim().trim_matches('"').to_owned()))
        })
        .collect()
}

fn detect_line_ending(source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => return Some("crlf".to_owned()),
            b'\r' => return Some("cr".to_owned()),
            b'\n' => return Some("lf".to_owned()),
            _ => {}
        }
    }
    None
}

#[derive(Clone, Debug, Default)]
struct CssDocumentState {
    reported_bad_string: bool,
    reported_bad_url: bool,
    reported_encoding_conflict: bool,
    reported_import: bool,
    reported_invalid_declaration: bool,
    reported_invalid_selector: bool,
    reported_invalid_token: bool,
    reported_unknown_at_rule: bool,
    reported_url: bool,
}

fn collect_css_policy_facts(
    source: &str,
    line_index: &LineIndex,
    declared_charset: Option<&str>,
    entry_mode: CssEntryMode,
) -> Vec<CssFact> {
    let mut facts = Vec::new();
    let mut state = CssDocumentState::default();

    if let Some((charset, offset)) = css_leading_charset(source) {
        if let Some(declared_charset) = declared_charset {
            if css_normalized_charset(declared_charset) != css_normalized_charset(&charset) {
                state.reported_encoding_conflict = true;
                facts.push(css_analysis_fact(
                    line_index,
                    offset,
                    CssFactKind::EncodingConflict,
                    format!(
                        "CSS MIME charset `{}` conflicts with @charset `{}`",
                        declared_charset, charset
                    ),
                ));
            }
        }
    }

    let sanitized = css_sanitize_source(source, line_index, &mut state, &mut facts);
    css_validate_delimiters(line_index, &sanitized, &mut state, &mut facts);
    css_validate_at_rules_and_urls(source, line_index, &sanitized, &mut state, &mut facts);
    css_validate_rule_shapes(line_index, &sanitized, &mut state, &mut facts);
    if entry_mode == CssEntryMode::DeclarationList {
        css_validate_declaration_block(
            line_index,
            &sanitized,
            0,
            sanitized.len(),
            &mut state,
            &mut facts,
        );
    }

    facts
}

fn css_leading_charset(source: &str) -> Option<(String, usize)> {
    let mut offset = source
        .strip_prefix('\u{feff}')
        .map_or(0, |_| '\u{feff}'.len_utf8());
    offset = css_skip_whitespace_and_comments(source, offset);
    let rest = source[offset..].trim_start();
    let skipped = source[offset..].len() - rest.len();
    offset += skipped;
    if !rest.to_ascii_lowercase().starts_with("@charset") {
        return None;
    }
    let after_keyword = &source[offset + "@charset".len()..];
    let after_ws = after_keyword.trim_start();
    let value_offset = offset + "@charset".len() + (after_keyword.len() - after_ws.len());
    let quote = after_ws.as_bytes().first().copied()?;
    if !matches!(quote, b'"' | b'\'') {
        return None;
    }
    let value_start = value_offset + 1;
    let value_rest = &source[value_start..];
    let value_end = value_rest.find(char::from(quote))?;
    Some((value_rest[..value_end].to_owned(), value_offset))
}

fn css_skip_whitespace_and_comments(source: &str, mut offset: usize) -> usize {
    while offset < source.len() {
        let rest = &source[offset..];
        if let Some(ch) = rest.chars().next() {
            if ch.is_whitespace() {
                offset += ch.len_utf8();
                continue;
            }
        }
        if rest.starts_with("/*") {
            if let Some(end) = rest.find("*/") {
                offset += end + 2;
                continue;
            }
        }
        break;
    }
    offset
}

fn css_normalized_charset(value: &str) -> String {
    value.trim().trim_matches('"').to_ascii_lowercase()
}

fn css_sanitize_source(
    source: &str,
    line_index: &LineIndex,
    state: &mut CssDocumentState,
    facts: &mut Vec<CssFact>,
) -> String {
    let mut sanitized = String::with_capacity(source.len());
    let mut chars = source.char_indices().peekable();

    while let Some((offset, ch)) = chars.next() {
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            sanitized.push(' ');
            let (_, _) = chars.next().expect("peeked comment opener");
            sanitized.push(' ');
            let mut closed = false;
            while let Some((_, comment_ch)) = chars.next() {
                sanitized.push(if comment_ch == '\n' { '\n' } else { ' ' });
                if comment_ch == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                    let (_, slash) = chars.next().expect("peeked comment closer");
                    sanitized.push(if slash == '\n' { '\n' } else { ' ' });
                    closed = true;
                    break;
                }
            }
            if !closed && !state.reported_invalid_token {
                state.reported_invalid_token = true;
                facts.push(css_analysis_fact(
                    line_index,
                    offset,
                    CssFactKind::InvalidToken,
                    "CSS comment is missing a closing */".to_owned(),
                ));
            }
            continue;
        }

        if matches!(ch, '"' | '\'') {
            sanitized.push(' ');
            let quote = ch;
            let mut escaped = false;
            let mut closed = false;
            while let Some((_, string_ch)) = chars.next() {
                sanitized.push(if string_ch == '\n' { '\n' } else { ' ' });
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == quote {
                    closed = true;
                    break;
                } else if string_ch == '\n' {
                    break;
                }
            }
            if !closed && !state.reported_bad_string {
                state.reported_bad_string = true;
                facts.push(css_analysis_fact(
                    line_index,
                    offset,
                    CssFactKind::BadString,
                    "CSS string token was recovered before a matching quote".to_owned(),
                ));
            }
            continue;
        }

        sanitized.push(ch);
    }

    sanitized
}

fn css_validate_delimiters(
    line_index: &LineIndex,
    sanitized: &str,
    state: &mut CssDocumentState,
    facts: &mut Vec<CssFact>,
) {
    let mut stack = Vec::new();
    for (offset, ch) in sanitized.char_indices() {
        match ch {
            '{' | '[' | '(' => stack.push((ch, offset)),
            '}' | ']' | ')' => {
                let expected = match ch {
                    '}' => '{',
                    ']' => '[',
                    ')' => '(',
                    _ => unreachable!(),
                };
                match stack.pop() {
                    Some((open, _)) if open == expected => {}
                    _ if !state.reported_invalid_token => {
                        state.reported_invalid_token = true;
                        facts.push(css_analysis_fact(
                            line_index,
                            offset,
                            CssFactKind::InvalidToken,
                            format!(
                                "CSS closing delimiter `{ch}` does not match an open delimiter"
                            ),
                        ));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    if let Some((open, offset)) = stack.first().copied() {
        if !state.reported_invalid_token {
            state.reported_invalid_token = true;
            facts.push(css_analysis_fact(
                line_index,
                offset,
                CssFactKind::InvalidToken,
                format!("CSS delimiter `{open}` is not closed"),
            ));
        }
    }
}

fn css_validate_at_rules_and_urls(
    source: &str,
    line_index: &LineIndex,
    sanitized: &str,
    state: &mut CssDocumentState,
    facts: &mut Vec<CssFact>,
) {
    let lower = sanitized.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes[offset] == b'@' {
            let name_start = offset + 1;
            let mut name_end = name_start;
            while name_end < bytes.len()
                && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'-')
            {
                name_end += 1;
            }
            let name = &lower[name_start..name_end];
            if name == "import" && !state.reported_import {
                state.reported_import = true;
                facts.push(css_analysis_fact(
                    line_index,
                    offset,
                    CssFactKind::ImportRejected,
                    "CSS @import access requires an explicit resolver policy".to_owned(),
                ));
            } else if !name.is_empty()
                && !css_at_rule_is_known(name)
                && !state.reported_unknown_at_rule
            {
                state.reported_unknown_at_rule = true;
                facts.push(css_analysis_fact(
                    line_index,
                    offset,
                    CssFactKind::UnknownAtRule,
                    format!("CSS at-rule `@{name}` is preserved as an unknown at-rule"),
                ));
            }
            offset = name_end;
        } else {
            offset += 1;
        }
    }

    let mut search_start = 0usize;
    while let Some(relative_url_start) = lower[search_start..].find("url(") {
        let url_start = search_start + relative_url_start;
        match css_url_argument(source, url_start) {
            Some(reference) if css_url_requires_policy(&reference) && !state.reported_url => {
                state.reported_url = true;
                facts.push(css_analysis_fact(
                    line_index,
                    url_start,
                    CssFactKind::UrlRejected,
                    "CSS url() reference requires an explicit resolver or sanitizer policy"
                        .to_owned(),
                ));
            }
            None if !state.reported_bad_url => {
                state.reported_bad_url = true;
                facts.push(css_analysis_fact(
                    line_index,
                    url_start,
                    CssFactKind::BadUrl,
                    "CSS url() token was recovered without a closing parenthesis".to_owned(),
                ));
            }
            _ => {}
        }
        search_start = url_start + 4;
    }
}

fn css_at_rule_is_known(name: &str) -> bool {
    matches!(
        name,
        "charset"
            | "container"
            | "font-face"
            | "font-feature-values"
            | "import"
            | "keyframes"
            | "layer"
            | "media"
            | "namespace"
            | "page"
            | "property"
            | "scope"
            | "supports"
    )
}

fn css_url_argument(source: &str, url_start: usize) -> Option<String> {
    let after_open = url_start + 4;
    let bytes = source.as_bytes();
    let mut offset = after_open;
    let mut quote = None;
    while offset < bytes.len() {
        let byte = bytes[offset];
        match (quote, byte) {
            (Some(q), b) if b == q => quote = None,
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b')') => {
                return Some(
                    source[after_open..offset]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_owned(),
                )
            }
            _ => {}
        }
        offset += 1;
    }
    None
}

fn css_url_requires_policy(reference: &str) -> bool {
    let trimmed = reference.trim();
    !(trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.to_ascii_lowercase().starts_with("data:"))
}

fn collect_generated_component_value_facts(
    value: &str,
    source_range: CssSourceRange,
    facts: &mut Vec<CssFact>,
) {
    let lower = value.to_ascii_lowercase();
    let mut search_start = 0usize;
    while let Some(relative_url_start) = lower[search_start..].find("url(") {
        let url_start = search_start + relative_url_start;
        match css_url_argument(value, url_start) {
            Some(reference) if css_url_requires_policy(&reference) => push_fact_once(
                facts,
                CssFactKind::UrlRejected,
                Some(source_range),
                "CSS url() reference requires an explicit resolver or sanitizer policy",
                None,
            ),
            None => push_fact_once(
                facts,
                CssFactKind::BadUrl,
                Some(source_range),
                "CSS url() token was recovered without a closing parenthesis",
                None,
            ),
            _ => {}
        }
        search_start = url_start + 4;
    }
}

fn css_validate_rule_shapes(
    line_index: &LineIndex,
    sanitized: &str,
    state: &mut CssDocumentState,
    facts: &mut Vec<CssFact>,
) {
    let mut stack = Vec::new();
    let mut rule_start = 0usize;
    for (offset, ch) in sanitized.char_indices() {
        match ch {
            '{' => {
                if stack.is_empty() {
                    let prelude = sanitized[rule_start..offset].trim();
                    if prelude.is_empty() && !state.reported_invalid_selector {
                        state.reported_invalid_selector = true;
                        facts.push(css_analysis_fact(
                            line_index,
                            offset,
                            CssFactKind::InvalidSelector,
                            "CSS rule has an empty selector or prelude".to_owned(),
                        ));
                    }
                }
                stack.push(offset);
            }
            '}' => {
                if let Some(open_offset) = stack.pop() {
                    if stack.is_empty() {
                        css_validate_declaration_block(
                            line_index,
                            sanitized,
                            open_offset + 1,
                            offset,
                            state,
                            facts,
                        );
                        rule_start = offset + 1;
                    }
                }
            }
            ';' if stack.is_empty() => {
                rule_start = offset + 1;
            }
            _ => {}
        }
    }
}

fn css_validate_declaration_block(
    line_index: &LineIndex,
    sanitized: &str,
    start: usize,
    end: usize,
    state: &mut CssDocumentState,
    facts: &mut Vec<CssFact>,
) {
    if state.reported_invalid_declaration {
        return;
    }
    let mut declaration_start = start;
    for relative_semicolon in sanitized[start..end]
        .match_indices(';')
        .map(|(index, _)| index)
    {
        let declaration_end = start + relative_semicolon;
        let declaration = sanitized[declaration_start..declaration_end].trim();
        if css_declaration_is_malformed(declaration) {
            state.reported_invalid_declaration = true;
            facts.push(css_analysis_fact(
                line_index,
                declaration_start,
                CssFactKind::InvalidDeclaration,
                "CSS declaration was recovered without a property/value colon".to_owned(),
            ));
            return;
        }
        declaration_start = declaration_end + 1;
    }
    let declaration = sanitized[declaration_start..end].trim();
    if css_declaration_is_malformed(declaration) {
        state.reported_invalid_declaration = true;
        facts.push(css_analysis_fact(
            line_index,
            declaration_start,
            CssFactKind::InvalidDeclaration,
            "CSS declaration was recovered without a property/value colon".to_owned(),
        ));
    }
}

fn css_declaration_is_malformed(declaration: &str) -> bool {
    !declaration.is_empty()
        && !declaration.contains(':')
        && !declaration.contains('{')
        && !declaration.contains('}')
        && !declaration.trim_start().starts_with('@')
}

fn css_analysis_fact(
    line_index: &LineIndex,
    byte_offset: usize,
    kind: CssFactKind,
    message: String,
) -> CssFact {
    CssFact {
        kind,
        source_range: Some(CssSourceRange::from_offsets(
            line_index,
            byte_offset,
            byte_offset,
        )),
        message,
        value: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(source: &str) -> Vec<Diagnostic> {
        validate_css_source_bytes(CssSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.css",
            content_type: Some("text/css"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    fn parse(source: &str, content_type: &str) -> (CssDocumentAst, Vec<Diagnostic>) {
        let (document, diagnostics) =
            css_document_ast_from_source_bytes(CssSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.css",
                content_type: Some(content_type),
            });
        (document.expect("typed CSS document"), diagnostics)
    }

    #[test]
    fn css_document_ast_preserves_lossless_nested_component_events() {
        let source = "/* lead */\n.card:is(.active, [data-x=\"a,b\"]) { --gap: calc(1rem + 2px); }";
        let (document, diagnostics) = parse(source, "text/css; charset=utf-8");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(document.entry_mode, CssEntryMode::Stylesheet);
        assert_eq!(document.source.parameters["charset"], "utf-8");
        assert_eq!(document.line_ending.as_deref(), Some("lf"));
        assert_eq!(
            document
                .events
                .iter()
                .map(|event| event.lexeme.as_str())
                .collect::<String>(),
            source
        );
        assert!(document.events.iter().any(|event| {
            event.token_kind == "function-open" && event.value.as_deref() == Some("calc")
        }));
        assert!(document
            .events
            .iter()
            .all(|event| { event.source_range.source_map().frames[0].source_id == SourceId(1) }));
    }

    #[test]
    fn css_document_ast_selects_declaration_list_and_scoped_modes() {
        let (declarations, declaration_diagnostics) = parse(
            "color: currentColor; --gap: 0.5rem;",
            "text/css; mode=style-attribute",
        );
        assert!(declaration_diagnostics.is_empty());
        assert_eq!(declarations.entry_mode, CssEntryMode::DeclarationList);

        let (scoped, scoped_diagnostics) = parse(
            ":host { display: block; }",
            "text/css; mode=scoped-style-block; scope=component-card",
        );
        assert!(scoped_diagnostics.is_empty());
        assert_eq!(scoped.entry_mode, CssEntryMode::ScopedStyleBlock);
        assert!(scoped
            .facts
            .iter()
            .any(|fact| fact.kind == CssFactKind::ScopedStyleBlockObserved));
    }

    #[test]
    fn css_schema_contract_catalog_owns_diagnostic_metadata() {
        let (_, diagnostics) = parse("@import \"theme.css\";", "text/css");
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "cem.css.import_rejected")
            .expect("schema-bound import diagnostic");

        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("schemaBehavior"))
                .and_then(Value::as_str),
            Some(CSS_FACT_BEHAVIOR)
        );
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("factKind"))
                .and_then(Value::as_str),
            Some("import-rejected")
        );
        assert!(diagnostic.source_map.is_some());
    }

    #[test]
    fn css_declaration_list_recovery_is_mode_aware() {
        let (document, diagnostics) = parse(
            "color currentColor; --gap: calc(1rem + 2px);",
            "text/css; mode=style-attribute",
        );

        assert_eq!(document.entry_mode, CssEntryMode::DeclarationList);
        assert!(has_code(&diagnostics, "cem.css.invalid_declaration"));
    }

    #[test]
    fn css_external_font_urls_remain_lexical_and_require_policy() {
        let source =
            "@font-face { font-family: Demo; src: url(https://cdn.example.test/demo.woff2); }";
        let (document, diagnostics) = parse(source, "text/css");

        assert!(has_code(&diagnostics, "cem.css.url_rejected"));
        assert_eq!(
            document
                .events
                .iter()
                .map(|event| event.lexeme.as_str())
                .collect::<String>(),
            source
        );
    }

    #[test]
    fn css_source_validator_accepts_basic_stylesheet() {
        let diagnostics = validate(
            r#"@charset "utf-8";
:root { --space-2: 0.5rem; }
.card { padding: var(--space-2); color: currentColor; }
"#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn css_source_validator_accepts_style_attribute_fragment() {
        let diagnostics = validate("color: currentColor; margin-inline: 0; --card-gap: 0.75rem;");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn css_source_validator_reports_import_rejected() {
        let diagnostics =
            validate("@import \"shared/theme.css\";\n.card { color: currentColor; }\n");

        assert!(has_code(&diagnostics, "cem.css.import_rejected"));
    }

    #[test]
    fn css_source_validator_reports_url_rejected() {
        let diagnostics = validate(".hero { background-image: url(\"images/hero.png\"); }");

        assert!(has_code(&diagnostics, "cem.css.url_rejected"));
    }

    #[test]
    fn css_source_validator_reports_invalid_token() {
        let diagnostics = validate(".card { color: red;");

        assert!(has_code(&diagnostics, "cem.css.invalid_token"));
    }

    #[test]
    fn css_source_validator_reports_invalid_declaration() {
        let diagnostics = validate(".card { color currentColor; padding: 1rem; }");

        assert!(has_code(&diagnostics, "cem.css.invalid_declaration"));
    }

    #[test]
    fn css_source_validator_reports_encoding_conflict() {
        let diagnostics = validate_css_source_bytes(CssSourceValidationRequest {
            bytes: br#"@charset "utf-8";
.card { color: currentColor; }
"#,
            source_uri: "fixture.css",
            content_type: Some("text/css; charset=iso-8859-1"),
        });

        assert!(has_code(&diagnostics, "cem.css.encoding_conflict"));
    }
}
