use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{content_type_essence, MARKDOWN_CONTENT_TYPE, MARKDOWN_SCHEMA_URI};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const MARKDOWN_PACKAGE_ID: &str = "markdown";
const MARKDOWN_UNSUPPORTED_ENCODING_CODE: &str = "cem.markdown.unsupported_encoding";
const MARKDOWN_CHARSET_MISSING_CODE: &str = "cem.markdown.charset_missing";
const MARKDOWN_UNKNOWN_VARIANT_CODE: &str = "cem.markdown.unknown_variant";
const MARKDOWN_EMBEDDED_HTML_REJECTED_CODE: &str = "cem.markdown.embedded_html_rejected";

#[derive(Debug, Clone, Copy)]
pub struct MarkdownSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDocumentAst {
    pub source: MarkdownDocumentSource,
    pub encoding_report: MarkdownEncodingReportAst,
    pub encoding_facts: Vec<MarkdownEncodingFact>,
    pub variant: String,
    pub variant_facts: Vec<MarkdownVariantFact>,
    pub parse_facts: Vec<MarkdownParseFact>,
    pub events: Vec<MarkdownEventAst>,
    pub line_ending: Option<String>,
}

impl MarkdownDocumentAst {
    pub fn to_cemt_subject(&self) -> Value {
        let mut document = serde_json::Map::new();
        document.insert("kind".to_owned(), json!("markdown-document"));
        document.insert("contentType".to_owned(), json!(MARKDOWN_CONTENT_TYPE));
        document.insert("schema".to_owned(), json!(MARKDOWN_SCHEMA_URI));
        document.insert("source".to_owned(), self.source.to_cemt_subject());
        document.insert(
            "encodingReport".to_owned(),
            self.encoding_report.to_cemt_subject(),
        );
        document.insert(
            "encodingFacts".to_owned(),
            Value::Array(
                self.encoding_facts
                    .iter()
                    .map(MarkdownEncodingFact::to_cemt_subject)
                    .collect(),
            ),
        );
        document.insert("variant".to_owned(), json!(self.variant));
        document.insert(
            "variantFacts".to_owned(),
            Value::Array(
                self.variant_facts
                    .iter()
                    .map(MarkdownVariantFact::to_cemt_subject)
                    .collect(),
            ),
        );
        document.insert(
            "parseFacts".to_owned(),
            Value::Array(
                self.parse_facts
                    .iter()
                    .map(MarkdownParseFact::to_cemt_subject)
                    .collect(),
            ),
        );
        document.insert(
            "events".to_owned(),
            Value::Array(
                self.events
                    .iter()
                    .map(MarkdownEventAst::to_cemt_subject)
                    .collect(),
            ),
        );
        if let Some(line_ending) = self.line_ending.as_deref() {
            document.insert("lineEnding".to_owned(), json!(line_ending));
        }
        Value::Object(document)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl MarkdownDocumentSource {
    fn from_request(
        request: MarkdownSourceValidationRequest<'_>,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        let content_type = request.content_type.unwrap_or(MARKDOWN_CONTENT_TYPE);
        Self {
            uri: request.source_uri.to_owned(),
            content_type: content_type.to_owned(),
            media_type: content_type_essence(content_type),
            parameters,
            byte_length: request.bytes.len(),
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownEncodingReportAst {
    pub declared_charset: Option<String>,
    pub normalized_charset: String,
    pub decoder_status: String,
    pub invalid_byte_offset: Option<u64>,
}

impl MarkdownEncodingReportAst {
    fn from_request(
        request: MarkdownSourceValidationRequest<'_>,
        encoding_facts: &[MarkdownEncodingFact],
    ) -> Self {
        let unsupported = encoding_facts
            .iter()
            .find(|fact| fact.kind == MarkdownEncodingFactKind::UnsupportedEncoding);
        Self {
            declared_charset: request
                .content_type
                .and_then(|content_type| content_type_parameter(content_type, "charset")),
            normalized_charset: unsupported
                .map(|_| "unsupported".to_owned())
                .unwrap_or_else(|| "utf-8".to_owned()),
            decoder_status: unsupported
                .map(|_| "error".to_owned())
                .unwrap_or_else(|| "decoded".to_owned()),
            invalid_byte_offset: unsupported.and_then(|fact| {
                fact.source_range
                    .map(|source_range| source_range.start.byte_offset)
            }),
        }
    }

    fn to_cemt_subject(&self) -> Value {
        let mut value = serde_json::Map::new();
        if let Some(charset) = self.declared_charset.as_deref() {
            value.insert("declaredCharset".to_owned(), json!(charset));
        }
        value.insert(
            "normalizedCharset".to_owned(),
            json!(self.normalized_charset),
        );
        value.insert("decoderStatus".to_owned(), json!(self.decoder_status));
        if let Some(byte_offset) = self.invalid_byte_offset {
            value.insert("invalidByteOffset".to_owned(), json!(byte_offset));
        }
        Value::Object(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownEncodingFact {
    pub kind: MarkdownEncodingFactKind,
    pub diagnostic_code: Option<String>,
    pub diagnostic_severity: Option<String>,
    pub recoverable: bool,
    pub fatal: bool,
    pub parameter: Option<String>,
    pub actual: Option<String>,
    pub expected: Vec<String>,
    pub source_range: Option<MarkdownSourceRange>,
    pub message: String,
}

impl MarkdownEncodingFact {
    fn charset_missing(catalog: &MarkdownDiagnosticCatalog) -> Self {
        let severity = catalog.severity(MARKDOWN_CHARSET_MISSING_CODE, Severity::Warning);
        Self {
            kind: MarkdownEncodingFactKind::CharsetMissing,
            diagnostic_code: Some(MARKDOWN_CHARSET_MISSING_CODE.to_owned()),
            diagnostic_severity: Some(markdown_severity_name(severity).to_owned()),
            recoverable: !severity.is_hard_violation(),
            fatal: severity.is_hard_violation(),
            parameter: Some("charset".to_owned()),
            actual: None,
            expected: vec!["explicit charset parameter".to_owned()],
            source_range: None,
            message: "text/markdown content type should include an explicit charset parameter"
                .to_owned(),
        }
    }

    fn unsupported_encoding(
        request: MarkdownSourceValidationRequest<'_>,
        error: &std::str::Utf8Error,
        catalog: &MarkdownDiagnosticCatalog,
    ) -> Self {
        let severity = catalog.severity(MARKDOWN_UNSUPPORTED_ENCODING_CODE, Severity::Error);
        Self {
            kind: MarkdownEncodingFactKind::UnsupportedEncoding,
            diagnostic_code: Some(MARKDOWN_UNSUPPORTED_ENCODING_CODE.to_owned()),
            diagnostic_severity: Some(markdown_severity_name(severity).to_owned()),
            recoverable: !severity.is_hard_violation(),
            fatal: severity.is_hard_violation(),
            parameter: Some("charset".to_owned()),
            actual: request
                .content_type
                .and_then(|content_type| content_type_parameter(content_type, "charset"))
                .or(Some("utf-8".to_owned())),
            expected: vec!["valid UTF-8".to_owned()],
            source_range: Some(MarkdownSourceRange::from_bytes_lossy(
                request.bytes,
                error.valid_up_to(),
                1,
            )),
            message: format!("Markdown source must be valid UTF-8: {error}"),
        }
    }

    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "diagnosticCode": self.diagnostic_code,
            "diagnosticSeverity": self.diagnostic_severity,
            "recoverable": self.recoverable,
            "fatal": self.fatal,
            "parameter": self.parameter,
            "actual": self.actual,
            "expected": self.expected,
            "message": self.message,
            "sourceRange": self.source_range.map(MarkdownSourceRange::to_cemt_subject),
            "sourceMap": self.source_range.map(MarkdownSourceRange::source_map),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownEncodingFactKind {
    CharsetMissing,
    UnsupportedEncoding,
}

impl MarkdownEncodingFactKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::CharsetMissing => "charset-missing",
            Self::UnsupportedEncoding => "unsupported-encoding",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownVariantFact {
    pub kind: MarkdownVariantFactKind,
    pub variant: Option<String>,
    pub diagnostic_code: Option<String>,
    pub diagnostic_severity: Option<String>,
    pub recoverable: bool,
    pub fatal: bool,
    pub message: String,
}

impl MarkdownVariantFact {
    fn default_variant() -> Self {
        Self {
            kind: MarkdownVariantFactKind::DefaultVariant,
            variant: Some("CommonMark".to_owned()),
            diagnostic_code: None,
            diagnostic_severity: None,
            recoverable: true,
            fatal: false,
            message: "Markdown variant defaults to CommonMark".to_owned(),
        }
    }

    fn known_variant(variant: &str) -> Self {
        Self {
            kind: MarkdownVariantFactKind::KnownVariant,
            variant: Some(markdown_normalized_variant(Some(variant))),
            diagnostic_code: None,
            diagnostic_severity: None,
            recoverable: true,
            fatal: false,
            message: format!(
                "Markdown variant `{}` is supported",
                markdown_normalized_variant(Some(variant))
            ),
        }
    }

    fn unknown_variant(variant: &str, catalog: &MarkdownDiagnosticCatalog) -> Self {
        let severity = catalog.severity(MARKDOWN_UNKNOWN_VARIANT_CODE, Severity::Warning);
        Self {
            kind: MarkdownVariantFactKind::UnknownVariant,
            variant: Some(variant.to_owned()),
            diagnostic_code: Some(MARKDOWN_UNKNOWN_VARIANT_CODE.to_owned()),
            diagnostic_severity: Some(markdown_severity_name(severity).to_owned()),
            recoverable: !severity.is_hard_violation(),
            fatal: severity.is_hard_violation(),
            message: format!("unknown Markdown variant `{variant}`"),
        }
    }

    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "variant": self.variant,
            "diagnosticCode": self.diagnostic_code,
            "diagnosticSeverity": self.diagnostic_severity,
            "recoverable": self.recoverable,
            "fatal": self.fatal,
            "message": self.message,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownVariantFactKind {
    DefaultVariant,
    KnownVariant,
    UnknownVariant,
}

impl MarkdownVariantFactKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::DefaultVariant => "default-variant",
            Self::KnownVariant => "known-variant",
            Self::UnknownVariant => "unknown-variant",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownParseFact {
    pub kind: MarkdownParseFactKind,
    pub diagnostic_code: Option<String>,
    pub diagnostic_severity: Option<String>,
    pub recoverable: bool,
    pub fatal: bool,
    pub event_index: Option<usize>,
    pub event_kind: Option<String>,
    pub raw: Option<String>,
    pub source_range: Option<MarkdownSourceRange>,
    pub message: String,
}

impl MarkdownParseFact {
    fn embedded_html(
        event_index: usize,
        event_kind: &str,
        raw: Option<String>,
        source_range: MarkdownSourceRange,
        catalog: &MarkdownDiagnosticCatalog,
    ) -> Self {
        let severity = catalog.severity(MARKDOWN_EMBEDDED_HTML_REJECTED_CODE, Severity::Error);
        Self {
            kind: MarkdownParseFactKind::EmbeddedHtml,
            diagnostic_code: Some(MARKDOWN_EMBEDDED_HTML_REJECTED_CODE.to_owned()),
            diagnostic_severity: Some(markdown_severity_name(severity).to_owned()),
            recoverable: !severity.is_hard_violation(),
            fatal: severity.is_hard_violation(),
            event_index: Some(event_index),
            event_kind: Some(event_kind.to_owned()),
            raw,
            source_range: Some(source_range),
            message: "Markdown embedded HTML is rejected unless an explicit policy permits it"
                .to_owned(),
        }
    }

    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "diagnosticCode": self.diagnostic_code,
            "diagnosticSeverity": self.diagnostic_severity,
            "recoverable": self.recoverable,
            "fatal": self.fatal,
            "eventIndex": self.event_index,
            "eventKind": self.event_kind,
            "raw": self.raw,
            "message": self.message,
            "sourceRange": self.source_range.map(MarkdownSourceRange::to_cemt_subject),
            "sourceMap": self.source_range.map(MarkdownSourceRange::source_map),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownParseFactKind {
    EmbeddedHtml,
}

impl MarkdownParseFactKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::EmbeddedHtml => "embedded-html",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownEventAst {
    pub index: usize,
    pub kind: String,
    pub tag: Option<String>,
    pub text: Option<String>,
    pub destination: Option<String>,
    pub title: Option<String>,
    pub info: Option<String>,
    pub level: Option<u32>,
    pub checked: Option<bool>,
    pub ordered_start: Option<u64>,
    pub source_range: MarkdownSourceRange,
}

impl MarkdownEventAst {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "kind": self.kind,
            "tag": self.tag,
            "text": self.text,
            "destination": self.destination,
            "title": self.title,
            "info": self.info,
            "level": self.level,
            "checked": self.checked,
            "orderedStart": self.ordered_start,
            "byteOffset": self.source_range.start.byte_offset,
            "byteLength": self.source_range.byte_length,
            "sourceRange": self.source_range.to_cemt_subject(),
            "sourceMap": self.source_range.source_map(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownSourceRange {
    pub start: MarkdownSourcePosition,
    pub byte_length: u64,
}

impl MarkdownSourceRange {
    fn from_offsets(line_index: &LineIndex, start: usize, end: usize) -> Self {
        let coordinate = line_index.project(start as u64);
        Self {
            start: MarkdownSourcePosition {
                line: coordinate.line,
                column: coordinate.column,
                byte_offset: start as u64,
            },
            byte_length: end.saturating_sub(start) as u64,
        }
    }

    fn from_bytes_lossy(bytes: &[u8], start: usize, byte_length: usize) -> Self {
        let line_index = LineIndex::from_bytes_lossy(bytes);
        let coordinate = line_index.project(start as u64);
        Self {
            start: MarkdownSourcePosition {
                line: coordinate.line,
                column: coordinate.column,
                byte_offset: start as u64,
            },
            byte_length: byte_length as u64,
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
                    content_type: MARKDOWN_CONTENT_TYPE.to_owned(),
                },
            }],
        }
    }

    fn to_cemt_subject(self) -> Value {
        json!({
            "byteOffset": self.start.byte_offset,
            "byteLength": self.byte_length,
            "line": self.start.line,
            "column": self.start.column,
        })
    }
}

pub fn validate_markdown_source_bytes(
    request: MarkdownSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let (_, diagnostics) = markdown_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn markdown_document_ast_from_source_bytes(
    request: MarkdownSourceValidationRequest<'_>,
) -> (Option<MarkdownDocumentAst>, Vec<Diagnostic>) {
    let catalog = MarkdownDiagnosticCatalog::from_builtin();
    let parameters = content_type_parameters(request.content_type);
    let mut encoding_facts = collect_markdown_encoding_facts(request, catalog);
    let variant_facts = collect_markdown_variant_facts(parameters.get("variant"), catalog);
    let variant = markdown_normalized_variant(parameters.get("variant").map(String::as_str));

    let mut parse_facts = Vec::new();
    let decoded = match std::str::from_utf8(request.bytes) {
        Ok(source) => Some(source),
        Err(error) => {
            encoding_facts.push(MarkdownEncodingFact::unsupported_encoding(
                request, &error, catalog,
            ));
            None
        }
    };

    let events = decoded
        .map(|source| {
            collect_markdown_events_and_facts(
                source,
                markdown_parser_options(parameters.get("variant").map(String::as_str)),
                &mut parse_facts,
                catalog,
            )
        })
        .unwrap_or_default();
    let line_ending = decoded
        .and_then(markdown_detect_line_ending_style)
        .map(str::to_owned);

    let document = MarkdownDocumentAst {
        source: MarkdownDocumentSource::from_request(request, parameters),
        encoding_report: MarkdownEncodingReportAst::from_request(request, &encoding_facts),
        encoding_facts,
        variant,
        variant_facts,
        parse_facts,
        events,
        line_ending,
    };
    let diagnostics = markdown_document_ast_diagnostics(&document, request.source_uri, catalog);

    (Some(document), diagnostics)
}

fn collect_markdown_encoding_facts(
    request: MarkdownSourceValidationRequest<'_>,
    catalog: &MarkdownDiagnosticCatalog,
) -> Vec<MarkdownEncodingFact> {
    let mut facts = Vec::new();
    if request
        .content_type
        .is_some_and(markdown_content_type_missing_charset)
    {
        facts.push(MarkdownEncodingFact::charset_missing(catalog));
    }
    facts
}

fn collect_markdown_variant_facts(
    variant: Option<&String>,
    catalog: &MarkdownDiagnosticCatalog,
) -> Vec<MarkdownVariantFact> {
    match variant {
        Some(variant) if markdown_variant_is_known(variant) => {
            vec![MarkdownVariantFact::known_variant(variant)]
        }
        Some(variant) => vec![MarkdownVariantFact::unknown_variant(variant, catalog)],
        None => vec![MarkdownVariantFact::default_variant()],
    }
}

fn collect_markdown_events_and_facts(
    source: &str,
    options: pulldown_cmark::Options,
    parse_facts: &mut Vec<MarkdownParseFact>,
    catalog: &MarkdownDiagnosticCatalog,
) -> Vec<MarkdownEventAst> {
    let line_index = LineIndex::from_utf8(source);
    let parser = pulldown_cmark::Parser::new_ext(source, options).into_offset_iter();
    let mut events = Vec::new();

    for (index, (event, range)) in parser.enumerate() {
        let event_ast = markdown_event_ast(index, &event, range, &line_index);
        if matches!(
            event,
            pulldown_cmark::Event::Html(_) | pulldown_cmark::Event::InlineHtml(_)
        ) {
            parse_facts.push(MarkdownParseFact::embedded_html(
                index,
                &event_ast.kind,
                event_ast.text.clone(),
                event_ast.source_range,
                catalog,
            ));
        }
        events.push(event_ast);
    }

    events
}

fn markdown_document_ast_diagnostics(
    document: &MarkdownDocumentAst,
    source_uri: &str,
    catalog: &MarkdownDiagnosticCatalog,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(
        document
            .encoding_facts
            .iter()
            .filter_map(|fact| markdown_encoding_fact_diagnostic(source_uri, fact, catalog)),
    );
    diagnostics.extend(
        document
            .variant_facts
            .iter()
            .filter_map(|fact| markdown_variant_fact_diagnostic(source_uri, fact, catalog)),
    );
    diagnostics.extend(
        document
            .parse_facts
            .iter()
            .filter_map(|fact| markdown_parse_fact_diagnostic(source_uri, fact, catalog)),
    );
    diagnostics
}

fn markdown_encoding_fact_diagnostic(
    source_uri: &str,
    fact: &MarkdownEncodingFact,
    catalog: &MarkdownDiagnosticCatalog,
) -> Option<Diagnostic> {
    let code = fact.diagnostic_code.as_ref()?;
    let severity = catalog.severity(code, Severity::Error);
    Some(Diagnostic {
        uri: Some(source_uri.to_owned()),
        line: fact.source_range.map(|range| range.start.line),
        column: fact.source_range.map(|range| range.start.column),
        byte_offset: fact.source_range.map(|range| range.start.byte_offset),
        code: code.clone(),
        severity,
        message: fact.message.clone(),
        details: Some(json!({
            "markdown": {
                "phase": "encoding",
                "factKind": fact.kind.as_str(),
                "parameter": fact.parameter,
                "actual": fact.actual,
                "expected": fact.expected,
                "sourceRange": fact.source_range.map(MarkdownSourceRange::to_cemt_subject),
            },
        })),
        source_map: fact.source_range.map(MarkdownSourceRange::source_map),
        ..Diagnostic::default()
    })
}

fn markdown_variant_fact_diagnostic(
    source_uri: &str,
    fact: &MarkdownVariantFact,
    catalog: &MarkdownDiagnosticCatalog,
) -> Option<Diagnostic> {
    let code = fact.diagnostic_code.as_ref()?;
    let severity = catalog.severity(code, Severity::Warning);
    Some(Diagnostic {
        uri: Some(source_uri.to_owned()),
        code: code.clone(),
        severity,
        message: fact.message.clone(),
        details: Some(json!({
            "markdown": {
                "phase": "variant",
                "factKind": fact.kind.as_str(),
                "variant": fact.variant,
            },
        })),
        ..Diagnostic::default()
    })
}

fn markdown_parse_fact_diagnostic(
    source_uri: &str,
    fact: &MarkdownParseFact,
    catalog: &MarkdownDiagnosticCatalog,
) -> Option<Diagnostic> {
    let code = fact.diagnostic_code.as_ref()?;
    let severity = catalog.severity(code, Severity::Error);
    Some(Diagnostic {
        uri: Some(source_uri.to_owned()),
        line: fact.source_range.map(|range| range.start.line),
        column: fact.source_range.map(|range| range.start.column),
        byte_offset: fact.source_range.map(|range| range.start.byte_offset),
        code: code.clone(),
        severity,
        message: fact.message.clone(),
        details: Some(json!({
            "markdown": {
                "phase": "parse",
                "factKind": fact.kind.as_str(),
                "eventIndex": fact.event_index,
                "eventKind": fact.event_kind,
                "sourceRange": fact.source_range.map(MarkdownSourceRange::to_cemt_subject),
            },
        })),
        source_map: fact.source_range.map(MarkdownSourceRange::source_map),
        ..Diagnostic::default()
    })
}

fn markdown_event_ast(
    index: usize,
    event: &pulldown_cmark::Event<'_>,
    range: std::ops::Range<usize>,
    line_index: &LineIndex,
) -> MarkdownEventAst {
    MarkdownEventAst {
        index,
        kind: markdown_event_kind(event).to_owned(),
        tag: markdown_event_tag(event),
        text: markdown_event_text(event),
        destination: markdown_event_destination(event),
        title: markdown_event_title(event),
        info: markdown_event_info(event),
        level: markdown_event_level(event),
        checked: markdown_event_checked(event),
        ordered_start: markdown_event_ordered_start(event),
        source_range: MarkdownSourceRange::from_offsets(line_index, range.start, range.end),
    }
}

fn markdown_event_kind(event: &pulldown_cmark::Event<'_>) -> &'static str {
    match event {
        pulldown_cmark::Event::Start(_) => "start",
        pulldown_cmark::Event::End(_) => "end",
        pulldown_cmark::Event::Text(_) => "text",
        pulldown_cmark::Event::Code(_) => "code",
        pulldown_cmark::Event::InlineMath(_) => "inline-math",
        pulldown_cmark::Event::DisplayMath(_) => "display-math",
        pulldown_cmark::Event::Html(_) => "html",
        pulldown_cmark::Event::InlineHtml(_) => "inline-html",
        pulldown_cmark::Event::FootnoteReference(_) => "footnote-reference",
        pulldown_cmark::Event::SoftBreak => "soft-break",
        pulldown_cmark::Event::HardBreak => "hard-break",
        pulldown_cmark::Event::Rule => "thematic-break",
        pulldown_cmark::Event::TaskListMarker(_) => "task-list-marker",
    }
}

fn markdown_event_tag(event: &pulldown_cmark::Event<'_>) -> Option<String> {
    match event {
        pulldown_cmark::Event::Start(tag) => Some(markdown_start_tag_name(tag)),
        pulldown_cmark::Event::End(tag) => Some(markdown_end_tag_name(tag)),
        _ => None,
    }
}

fn markdown_event_text(event: &pulldown_cmark::Event<'_>) -> Option<String> {
    match event {
        pulldown_cmark::Event::Text(text)
        | pulldown_cmark::Event::Code(text)
        | pulldown_cmark::Event::InlineMath(text)
        | pulldown_cmark::Event::DisplayMath(text)
        | pulldown_cmark::Event::Html(text)
        | pulldown_cmark::Event::InlineHtml(text)
        | pulldown_cmark::Event::FootnoteReference(text) => Some(text.to_string()),
        _ => None,
    }
}

fn markdown_event_destination(event: &pulldown_cmark::Event<'_>) -> Option<String> {
    match event {
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. })
        | pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) => {
            Some(dest_url.to_string())
        }
        _ => None,
    }
}

fn markdown_event_title(event: &pulldown_cmark::Event<'_>) -> Option<String> {
    match event {
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { title, .. })
        | pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { title, .. }) => {
            (!title.is_empty()).then(|| title.to_string())
        }
        _ => None,
    }
}

fn markdown_event_info(event: &pulldown_cmark::Event<'_>) -> Option<String> {
    match event {
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(
            pulldown_cmark::CodeBlockKind::Indented,
        )) => Some("indented".to_owned()),
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(
            pulldown_cmark::CodeBlockKind::Fenced(info),
        )) => Some(info.to_string()),
        _ => None,
    }
}

fn markdown_event_level(event: &pulldown_cmark::Event<'_>) -> Option<u32> {
    match event {
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => {
            Some(markdown_heading_level(*level))
        }
        pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Heading(level)) => {
            Some(markdown_heading_level(*level))
        }
        _ => None,
    }
}

fn markdown_event_checked(event: &pulldown_cmark::Event<'_>) -> Option<bool> {
    match event {
        pulldown_cmark::Event::TaskListMarker(checked) => Some(*checked),
        _ => None,
    }
}

fn markdown_event_ordered_start(event: &pulldown_cmark::Event<'_>) -> Option<u64> {
    match event {
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::List(start)) => *start,
        _ => None,
    }
}

fn markdown_start_tag_name(tag: &pulldown_cmark::Tag<'_>) -> String {
    match tag {
        pulldown_cmark::Tag::Paragraph => "paragraph",
        pulldown_cmark::Tag::Heading { .. } => "heading",
        pulldown_cmark::Tag::BlockQuote(_) => "blockquote",
        pulldown_cmark::Tag::CodeBlock(_) => "code-block",
        pulldown_cmark::Tag::HtmlBlock => "html-block",
        pulldown_cmark::Tag::List(Some(_)) => "ordered-list",
        pulldown_cmark::Tag::List(None) => "list",
        pulldown_cmark::Tag::Item => "list-item",
        pulldown_cmark::Tag::FootnoteDefinition(_) => "footnote-definition",
        pulldown_cmark::Tag::DefinitionList => "definition-list",
        pulldown_cmark::Tag::DefinitionListTitle => "definition-list-title",
        pulldown_cmark::Tag::DefinitionListDefinition => "definition-list-definition",
        pulldown_cmark::Tag::Table(_) => "table",
        pulldown_cmark::Tag::TableHead => "table-head",
        pulldown_cmark::Tag::TableRow => "table-row",
        pulldown_cmark::Tag::TableCell => "table-cell",
        pulldown_cmark::Tag::Emphasis => "emphasis",
        pulldown_cmark::Tag::Strong => "strong",
        pulldown_cmark::Tag::Strikethrough => "strikethrough",
        pulldown_cmark::Tag::Superscript => "superscript",
        pulldown_cmark::Tag::Subscript => "subscript",
        pulldown_cmark::Tag::Link { .. } => "link",
        pulldown_cmark::Tag::Image { .. } => "image",
        pulldown_cmark::Tag::MetadataBlock(_) => "metadata-block",
    }
    .to_owned()
}

fn markdown_end_tag_name(tag: &pulldown_cmark::TagEnd) -> String {
    match tag {
        pulldown_cmark::TagEnd::Paragraph => "paragraph",
        pulldown_cmark::TagEnd::Heading(_) => "heading",
        pulldown_cmark::TagEnd::BlockQuote(_) => "blockquote",
        pulldown_cmark::TagEnd::CodeBlock => "code-block",
        pulldown_cmark::TagEnd::HtmlBlock => "html-block",
        pulldown_cmark::TagEnd::List(true) => "ordered-list",
        pulldown_cmark::TagEnd::List(false) => "list",
        pulldown_cmark::TagEnd::Item => "list-item",
        pulldown_cmark::TagEnd::FootnoteDefinition => "footnote-definition",
        pulldown_cmark::TagEnd::DefinitionList => "definition-list",
        pulldown_cmark::TagEnd::DefinitionListTitle => "definition-list-title",
        pulldown_cmark::TagEnd::DefinitionListDefinition => "definition-list-definition",
        pulldown_cmark::TagEnd::Table => "table",
        pulldown_cmark::TagEnd::TableHead => "table-head",
        pulldown_cmark::TagEnd::TableRow => "table-row",
        pulldown_cmark::TagEnd::TableCell => "table-cell",
        pulldown_cmark::TagEnd::Emphasis => "emphasis",
        pulldown_cmark::TagEnd::Strong => "strong",
        pulldown_cmark::TagEnd::Strikethrough => "strikethrough",
        pulldown_cmark::TagEnd::Superscript => "superscript",
        pulldown_cmark::TagEnd::Subscript => "subscript",
        pulldown_cmark::TagEnd::Link => "link",
        pulldown_cmark::TagEnd::Image => "image",
        pulldown_cmark::TagEnd::MetadataBlock(_) => "metadata-block",
    }
    .to_owned()
}

fn markdown_heading_level(level: pulldown_cmark::HeadingLevel) -> u32 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 1,
        pulldown_cmark::HeadingLevel::H2 => 2,
        pulldown_cmark::HeadingLevel::H3 => 3,
        pulldown_cmark::HeadingLevel::H4 => 4,
        pulldown_cmark::HeadingLevel::H5 => 5,
        pulldown_cmark::HeadingLevel::H6 => 6,
    }
}

fn markdown_content_type_missing_charset(content_type: &str) -> bool {
    content_type_essence(content_type) == MARKDOWN_CONTENT_TYPE
        && content_type_parameter(content_type, "charset").is_none()
}

fn markdown_variant_is_known(variant: &str) -> bool {
    matches!(
        variant.trim().to_ascii_lowercase().as_str(),
        "commonmark" | "gfm" | "github-flavored-markdown"
    )
}

fn markdown_normalized_variant(variant: Option<&str>) -> String {
    match variant.map(str::trim) {
        Some(variant) if variant.eq_ignore_ascii_case("gfm") => "GFM".to_owned(),
        Some(variant) if variant.eq_ignore_ascii_case("github-flavored-markdown") => {
            "GFM".to_owned()
        }
        Some(variant) if variant.eq_ignore_ascii_case("commonmark") => "CommonMark".to_owned(),
        Some(variant) => variant.to_owned(),
        None => "CommonMark".to_owned(),
    }
}

fn markdown_parser_options(variant: Option<&str>) -> pulldown_cmark::Options {
    let mut options = pulldown_cmark::Options::empty();
    if variant.map(str::trim).is_some_and(|variant| {
        variant.eq_ignore_ascii_case("gfm")
            || variant.eq_ignore_ascii_case("github-flavored-markdown")
    }) {
        options.insert(pulldown_cmark::Options::ENABLE_GFM);
        options.insert(pulldown_cmark::Options::ENABLE_TABLES);
        options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
        options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    }
    options
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownDiagnosticCatalog {
    severities: BTreeMap<String, Severity>,
}

impl MarkdownDiagnosticCatalog {
    fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<MarkdownDiagnosticCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(MARKDOWN_PACKAGE_ID)
                .expect("built-in Markdown package source must be registered");
            let model = compile_schema_document_model(MARKDOWN_SCHEMA_URI, source.schema_source);
            let severities = model
                .diagnostics
                .into_iter()
                .map(|(code, diagnostic)| (code, diagnostic.severity))
                .collect();
            MarkdownDiagnosticCatalog { severities }
        })
    }

    fn severity(&self, code: &str, fallback: Severity) -> Severity {
        self.severities.get(code).copied().unwrap_or(fallback)
    }
}

fn markdown_severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Fatal => "fatal",
    }
}

fn content_type_parameters(content_type: Option<&str>) -> BTreeMap<String, String> {
    let Some(content_type) = content_type else {
        return BTreeMap::new();
    };
    content_type
        .split(';')
        .skip(1)
        .filter_map(|part| {
            let (name, value) = part.split_once('=')?;
            Some((
                name.trim().to_ascii_lowercase(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect()
}

fn content_type_parameter(content_type: &str, name: &str) -> Option<String> {
    content_type_parameters(Some(content_type)).remove(&name.to_ascii_lowercase())
}

fn markdown_detect_line_ending_style(source: &str) -> Option<&'static str> {
    let has_crlf = source.contains("\r\n");
    let has_lone_cr = source
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'\r' && pair[1] != b'\n')
        || source.ends_with('\r');
    if has_crlf && has_lone_cr {
        Some("mixed")
    } else if has_crlf {
        Some("crlf")
    } else if has_lone_cr {
        Some("cr")
    } else if source.contains('\n') {
        Some("lf")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(source: &str) -> Vec<Diagnostic> {
        validate_markdown_source_bytes(MarkdownSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.md",
            content_type: Some("text/markdown; charset=utf-8; variant=CommonMark"),
        })
    }

    fn open_ast(source: &str) -> (MarkdownDocumentAst, Vec<Diagnostic>) {
        open_ast_with_content_type(source, "text/markdown; charset=utf-8; variant=CommonMark")
    }

    fn open_ast_with_content_type(
        source: &str,
        content_type: &str,
    ) -> (MarkdownDocumentAst, Vec<Diagnostic>) {
        let (document, diagnostics) =
            markdown_document_ast_from_source_bytes(MarkdownSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.md",
                content_type: Some(content_type),
            });
        (document.expect("Markdown document AST"), diagnostics)
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn markdown_source_validator_accepts_commonmark() {
        let diagnostics = validate(
            "# Release Notes\n\nThis document has **strong** text and a list.\n\n- Added schema validation.\n",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn markdown_source_validator_returns_typed_ast_stream_with_source_maps() {
        let (document, diagnostics) = open_ast(
            "# Release Notes\n\nThis document has **strong** text and [a link](https://example.test).\n",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(document.source.media_type, MARKDOWN_CONTENT_TYPE);
        assert_eq!(document.variant, "CommonMark");
        assert_eq!(document.encoding_report.decoder_status, "decoded");
        assert!(document
            .variant_facts
            .iter()
            .any(|fact| fact.kind == MarkdownVariantFactKind::KnownVariant));
        assert!(document
            .events
            .iter()
            .any(|event| event.kind == "start" && event.tag.as_deref() == Some("heading")));
        let text = document
            .events
            .iter()
            .find(|event| event.text.as_deref() == Some("Release Notes"))
            .expect("heading text event");
        assert_eq!(text.source_range.start.line, 1);
        assert!(!text.source_range.source_map().frames.is_empty());

        let subject = document.to_cemt_subject();
        assert_eq!(subject["kind"], "markdown-document");
        assert_eq!(
            subject["events"]
                .as_array()
                .and_then(|events| events.iter().find(|event| event["text"] == "Release Notes"))
                .and_then(|event| event["sourceMap"]["frames"].as_array())
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn markdown_gfm_variant_emits_table_and_task_list_ast_events() {
        let (document, diagnostics) = open_ast_with_content_type(
            "# Worklog\n\n| Task | Status |\n| --- | --- |\n| Schema validation | Done |\n\n- [x] Add parser-backed validation.\n- [ ] Connect converter profiles.\n",
            "text/markdown; charset=utf-8; variant=GFM",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(document.variant, "GFM");
        assert!(document
            .events
            .iter()
            .any(|event| event.kind == "start" && event.tag.as_deref() == Some("table")));
        assert!(document
            .events
            .iter()
            .any(|event| event.kind == "start" && event.tag.as_deref() == Some("table-row")));
        assert!(document
            .events
            .iter()
            .any(|event| event.kind == "task-list-marker" && event.checked == Some(true)));
        assert!(document
            .events
            .iter()
            .any(|event| event.kind == "task-list-marker" && event.checked == Some(false)));
    }

    #[test]
    fn markdown_source_validator_reports_charset_missing() {
        let (document, diagnostics) =
            markdown_document_ast_from_source_bytes(MarkdownSourceValidationRequest {
                bytes: b"# Title\n",
                source_uri: "fixture.md",
                content_type: Some("text/markdown"),
            });

        let document = document.expect("Markdown AST");
        assert!(document
            .encoding_facts
            .iter()
            .any(|fact| fact.kind == MarkdownEncodingFactKind::CharsetMissing));
        assert!(has_code(&diagnostics, MARKDOWN_CHARSET_MISSING_CODE));
    }

    #[test]
    fn markdown_source_validator_reports_unknown_variant() {
        let (document, diagnostics) =
            markdown_document_ast_from_source_bytes(MarkdownSourceValidationRequest {
                bytes: b"# Title\n",
                source_uri: "fixture.md",
                content_type: Some("text/markdown; charset=utf-8; variant=CustomWiki"),
            });

        let document = document.expect("Markdown AST");
        assert_eq!(document.variant, "CustomWiki");
        assert!(document
            .variant_facts
            .iter()
            .any(|fact| fact.kind == MarkdownVariantFactKind::UnknownVariant));
        assert!(has_code(&diagnostics, MARKDOWN_UNKNOWN_VARIANT_CODE));
    }

    #[test]
    fn markdown_source_validator_reports_embedded_html() {
        let (document, diagnostics) = open_ast("# Unsafe\n\n<script>alert('x')</script>\n");

        assert!(document
            .parse_facts
            .iter()
            .any(|fact| fact.kind == MarkdownParseFactKind::EmbeddedHtml
                && fact.source_range.is_some()));
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == MARKDOWN_EMBEDDED_HTML_REJECTED_CODE)
            .expect("embedded HTML diagnostic");
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("markdown"))
                .and_then(|details| details.get("phase")),
            Some(&json!("parse"))
        );
        assert!(diagnostic.source_map.is_some());
    }

    #[test]
    fn markdown_source_validator_reports_unsupported_encoding() {
        let (document, diagnostics) =
            markdown_document_ast_from_source_bytes(MarkdownSourceValidationRequest {
                bytes: b"# Bad\n\xff\n",
                source_uri: "fixture.md",
                content_type: Some("text/markdown; charset=utf-8"),
            });

        let document = document.expect("Markdown AST");
        assert_eq!(document.encoding_report.decoder_status, "error");
        assert!(document.events.is_empty());
        assert!(document.encoding_facts.iter().any(|fact| fact.kind
            == MarkdownEncodingFactKind::UnsupportedEncoding
            && fact.source_range.is_some()));
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == MARKDOWN_UNSUPPORTED_ENCODING_CODE)
            .expect("unsupported encoding diagnostic");
        assert!(diagnostic.source_map.is_some());
    }
}
