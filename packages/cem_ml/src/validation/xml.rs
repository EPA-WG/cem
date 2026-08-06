use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{content_type_essence, XML_CONTENT_TYPE, XML_SCHEMA_URI};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use serde_json::json;
#[cfg(test)]
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const XML_PACKAGE_ID: &str = "xml";

#[derive(Debug, Clone, Copy)]
pub struct XmlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDocumentAst {
    pub source: XmlDocumentSource,
    pub resource_kind: String,
    pub encoding_report: XmlEncodingReportAst,
    pub parse_facts: Vec<XmlParseFact>,
    pub events: Vec<XmlEventAst>,
    pub line_ending: Option<String>,
}

impl XmlDocumentAst {
    #[cfg(test)]
    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": "xml-document",
            "contentType": XML_CONTENT_TYPE,
            "schema": XML_SCHEMA_URI,
            "source": self.source.to_cemt_subject(),
            "resourceKind": self.resource_kind,
            "encodingReport": self.encoding_report.to_cemt_subject(),
            "parseFacts": self
                .parse_facts
                .iter()
                .map(XmlParseFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "events": self
                .events
                .iter()
                .map(XmlEventAst::to_cemt_subject)
                .collect::<Vec<_>>(),
            "lineEnding": self.line_ending,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl XmlDocumentSource {
    fn from_request(
        request: XmlSourceValidationRequest<'_>,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        let content_type = request.content_type.unwrap_or(XML_CONTENT_TYPE);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlEncodingReportAst {
    pub mime_charset: Option<String>,
    pub declaration_encoding: Option<String>,
    pub normalized_encoding: String,
    pub decoder_status: String,
}

impl XmlEncodingReportAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "mimeCharset": self.mime_charset,
            "declarationEncoding": self.declaration_encoding,
            "normalizedEncoding": self.normalized_encoding,
            "decoderStatus": self.decoder_status,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlEventAst {
    pub index: usize,
    pub kind: XmlEventKind,
    pub depth: usize,
    pub qualified_name: Option<String>,
    pub local_name: Option<String>,
    pub prefix: Option<String>,
    pub namespace_uri: Option<String>,
    pub attributes: Vec<XmlAttributeAst>,
    pub value: Option<String>,
    pub lexeme: String,
    pub whitespace_only: bool,
    pub source_range: XmlSourceRange,
}

impl XmlEventAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "kind": self.kind.as_str(),
            "depth": self.depth,
            "qualifiedName": self.qualified_name,
            "localName": self.local_name,
            "prefix": self.prefix,
            "namespaceUri": self.namespace_uri,
            "attributes": self
                .attributes
                .iter()
                .map(XmlAttributeAst::to_cemt_subject)
                .collect::<Vec<_>>(),
            "value": self.value,
            "lexeme": self.lexeme,
            "whitespaceOnly": self.whitespace_only,
            "sourceRange": self.source_range.to_cemt_subject(),
            "sourceMap": serde_json::to_value(self.source_range.source_map())
                .unwrap_or(Value::Null),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlEventKind {
    Declaration,
    StartElement,
    EmptyElement,
    EndElement,
    Text,
    Cdata,
    Comment,
    ProcessingInstruction,
    Doctype,
    EntityReference,
}

impl XmlEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Declaration => "declaration",
            Self::StartElement => "start-element",
            Self::EmptyElement => "empty-element",
            Self::EndElement => "end-element",
            Self::Text => "text",
            Self::Cdata => "cdata",
            Self::Comment => "comment",
            Self::ProcessingInstruction => "processing-instruction",
            Self::Doctype => "doctype",
            Self::EntityReference => "entity-reference",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XmlMarkupTokenKind {
    Delimiter,
    ElementName,
    Whitespace,
    AttributeName,
    Equals,
    AttributeValue,
    Raw,
}

impl XmlMarkupTokenKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Delimiter => "delimiter",
            Self::ElementName => "element-name",
            Self::Whitespace => "whitespace",
            Self::AttributeName => "attribute-name",
            Self::Equals => "equals",
            Self::AttributeValue => "attribute-value",
            Self::Raw => "raw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlMarkupTokenAst {
    pub(crate) kind: XmlMarkupTokenKind,
    pub(crate) text: String,
    pub(crate) source_range: XmlSourceRange,
}

pub(crate) fn xml_event_markup_tokens(event: &XmlEventAst) -> Vec<XmlMarkupTokenAst> {
    if !matches!(
        event.kind,
        XmlEventKind::StartElement | XmlEventKind::EmptyElement | XmlEventKind::EndElement
    ) {
        return Vec::new();
    }

    let lexeme = event.lexeme.as_str();
    let bytes = lexeme.as_bytes();
    let mut tokens = Vec::new();
    let mut offset = 0usize;
    if bytes.starts_with(b"</") {
        xml_push_markup_token(event, &mut tokens, XmlMarkupTokenKind::Delimiter, 0, 2);
        offset = 2;
    } else if bytes.starts_with(b"<") {
        xml_push_markup_token(event, &mut tokens, XmlMarkupTokenKind::Delimiter, 0, 1);
        offset = 1;
    }

    let name_start = offset;
    while offset < bytes.len()
        && !bytes[offset].is_ascii_whitespace()
        && !matches!(bytes[offset], b'/' | b'>')
    {
        offset += 1;
    }
    if offset > name_start {
        xml_push_markup_token(
            event,
            &mut tokens,
            XmlMarkupTokenKind::ElementName,
            name_start,
            offset,
        );
    }

    while offset < bytes.len() {
        if bytes[offset].is_ascii_whitespace() {
            let start = offset;
            while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
                offset += 1;
            }
            xml_push_markup_token(
                event,
                &mut tokens,
                XmlMarkupTokenKind::Whitespace,
                start,
                offset,
            );
            continue;
        }
        if bytes[offset..].starts_with(b"/>") {
            xml_push_markup_token(
                event,
                &mut tokens,
                XmlMarkupTokenKind::Delimiter,
                offset,
                offset + 2,
            );
            offset += 2;
            continue;
        }
        if bytes[offset] == b'>' {
            xml_push_markup_token(
                event,
                &mut tokens,
                XmlMarkupTokenKind::Delimiter,
                offset,
                offset + 1,
            );
            offset += 1;
            continue;
        }
        if bytes[offset] == b'=' {
            xml_push_markup_token(
                event,
                &mut tokens,
                XmlMarkupTokenKind::Equals,
                offset,
                offset + 1,
            );
            offset += 1;
            continue;
        }
        if matches!(bytes[offset], b'\'' | b'\"') {
            let quote = bytes[offset];
            let start = offset;
            offset += 1;
            while offset < bytes.len() && bytes[offset] != quote {
                offset += 1;
            }
            if offset < bytes.len() {
                offset += 1;
            }
            xml_push_markup_token(
                event,
                &mut tokens,
                XmlMarkupTokenKind::AttributeValue,
                start,
                offset,
            );
            continue;
        }

        let start = offset;
        while offset < bytes.len()
            && !bytes[offset].is_ascii_whitespace()
            && !matches!(bytes[offset], b'=' | b'/' | b'>')
        {
            offset += 1;
        }
        if offset == start {
            offset += 1;
            xml_push_markup_token(event, &mut tokens, XmlMarkupTokenKind::Raw, start, offset);
        } else {
            xml_push_markup_token(
                event,
                &mut tokens,
                XmlMarkupTokenKind::AttributeName,
                start,
                offset,
            );
        }
    }

    tokens
}

fn xml_push_markup_token(
    event: &XmlEventAst,
    tokens: &mut Vec<XmlMarkupTokenAst>,
    kind: XmlMarkupTokenKind,
    start: usize,
    end: usize,
) {
    let Some(text) = event.lexeme.get(start..end) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    tokens.push(XmlMarkupTokenAst {
        kind,
        text: text.to_owned(),
        source_range: xml_lexeme_range(event, start, end),
    });
}

fn xml_lexeme_range(event: &XmlEventAst, start: usize, end: usize) -> XmlSourceRange {
    xml_source_range_within(event.source_range, &event.lexeme, start, end)
}

fn xml_source_range_within(
    outer_range: XmlSourceRange,
    source: &str,
    start: usize,
    end: usize,
) -> XmlSourceRange {
    let prefix = &source[..start];
    let mut line = outer_range.start.line;
    let mut column = outer_range.start.column;
    let mut chars = prefix.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                line = line.saturating_add(1);
                column = 1;
            }
            '\n' => {
                line = line.saturating_add(1);
                column = 1;
            }
            _ => column = column.saturating_add(1),
        }
    }
    XmlSourceRange {
        start: XmlSourcePosition {
            line,
            column,
            byte_offset: outer_range.start.byte_offset + start as u64,
        },
        byte_length: (end - start) as u64,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAttributeAst {
    pub qualified_name: String,
    pub local_name: String,
    pub prefix: Option<String>,
    pub namespace_uri: Option<String>,
    /// Exact source lexeme between the attribute's quotes.
    pub value: String,
    /// Absolute source range for `value`, excluding the quotes.
    pub value_source_range: Option<XmlSourceRange>,
    /// Built-in and numeric XML references decoded to Unicode scalars.
    /// This is absent when the lexical value contains an unresolved or invalid
    /// reference; no external entity resolver is consulted.
    pub entity_decoded_value: Option<String>,
    /// Boundary-aware projection from `entity_decoded_value` to the original
    /// lexical XML attribute value. This is absent whenever decoding fails.
    pub entity_decoded_source_map: Option<XmlAttributeValueSourceMap>,
}

/// Projects scalar-aligned decoded XML attribute ranges to original source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAttributeValueSourceMap {
    decoded_byte_length: u64,
    source_range: XmlSourceRange,
    boundaries: Vec<XmlAttributeValueSourceBoundary>,
    spans: Vec<XmlAttributeValueSourceSpan>,
}

impl XmlAttributeValueSourceMap {
    pub fn decoded_byte_length(&self) -> u64 {
        self.decoded_byte_length
    }

    pub fn source_range(&self) -> XmlSourceRange {
        self.source_range
    }

    pub fn boundaries(&self) -> &[XmlAttributeValueSourceBoundary] {
        &self.boundaries
    }

    pub fn spans(&self) -> &[XmlAttributeValueSourceSpan] {
        &self.spans
    }

    /// Projects a decoded UTF-8 scalar boundary to its original XML position.
    /// Interior scalar bytes and positions outside the decoded value fail closed.
    pub fn project_boundary(&self, decoded_byte_offset: u64) -> Option<XmlSourcePosition> {
        if decoded_byte_offset > self.decoded_byte_length {
            return None;
        }
        let index = self
            .boundaries
            .binary_search_by_key(&decoded_byte_offset, |boundary| {
                boundary.decoded_byte_offset
            })
            .ok()?;
        Some(self.boundaries[index].source_position)
    }

    /// Projects a scalar-aligned decoded byte range to the smallest contiguous
    /// original XML source range containing the represented lexical material.
    pub fn project_range(&self, decoded_range: ByteRange) -> Option<XmlSourceRange> {
        let decoded_end = decoded_range
            .start
            .checked_add(u64::from(decoded_range.len))?;
        if decoded_end > self.decoded_byte_length {
            return None;
        }
        let start = self.project_boundary(decoded_range.start)?;
        let end = self.project_boundary(decoded_end)?;
        Some(XmlSourceRange {
            start,
            byte_length: end.byte_offset.checked_sub(start.byte_offset)?,
        })
    }
}

/// One scalar boundary shared by adjacent decoded/source spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlAttributeValueSourceBoundary {
    pub decoded_byte_offset: u64,
    pub source_position: XmlSourcePosition,
}

/// Maps one decoded UTF-8 scalar back to its exact lexical XML source span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlAttributeValueSourceSpan {
    pub decoded_byte_range: ByteRange,
    pub source_range: XmlSourceRange,
}

impl XmlAttributeAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "qualifiedName": self.qualified_name,
            "localName": self.local_name,
            "prefix": self.prefix,
            "namespaceUri": self.namespace_uri,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlSourceRange {
    pub start: XmlSourcePosition,
    pub byte_length: u64,
}

impl XmlSourceRange {
    fn from_offsets(line_index: &LineIndex, start: usize, end: usize) -> Self {
        let coordinate = line_index.project(start as u64);
        Self {
            start: XmlSourcePosition {
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
                    content_type: XML_CONTENT_TYPE.to_owned(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlParseFact {
    pub kind: XmlParseFactKind,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub byte_length: Option<u64>,
    pub message: String,
}

impl XmlParseFact {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "line": self.line,
            "column": self.column,
            "byteOffset": self.byte_offset,
            "byteLength": self.byte_length,
            "message": self.message,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum XmlParseFactKind {
    ParseError,
    UnsupportedEncoding,
    EncodingConflict,
    UnboundNamespacePrefix,
    DuplicateAttribute,
    DtdRejected,
    ExternalEntityRejected,
    EntityExpansionLimit,
    SourceMapUnavailable,
}

impl XmlParseFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseError => "parse-error",
            Self::UnsupportedEncoding => "unsupported-encoding",
            Self::EncodingConflict => "encoding-conflict",
            Self::UnboundNamespacePrefix => "unbound-namespace-prefix",
            Self::DuplicateAttribute => "duplicate-attribute",
            Self::DtdRejected => "dtd-rejected",
            Self::ExternalEntityRejected => "external-entity-rejected",
            Self::EntityExpansionLimit => "entity-expansion-limit",
            Self::SourceMapUnavailable => "source-map-unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlParseReport {
    pub source_uri: String,
    pub content_type: Option<String>,
    pub parameters: BTreeMap<String, String>,
    pub resource_kind: String,
    pub declaration_encoding: Option<String>,
    pub facts: Vec<XmlParseFact>,
    pub events: Vec<XmlEventAst>,
    pub line_ending: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlSchemaContractCatalog {
    pub fact_bindings: BTreeMap<String, XmlDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

impl XmlSchemaContractCatalog {
    pub fn from_builtin() -> Self {
        let source = builtin_schema_package_source(XML_PACKAGE_ID)
            .expect("built-in XML schema package source must be registered");
        Self::from_schema_source(source.schema_source)
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(XML_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != "xml-parse-report-fact" {
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
                    XmlDiagnosticBinding {
                        fact_kind: fact_kind.to_owned(),
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

    fn binding_for_fact(&self, kind: XmlParseFactKind) -> Option<&XmlDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_xml_source_bytes(request: XmlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let report = extract_xml_parse_report(request);
    validate_xml_parse_report(&report, &XmlSchemaContractCatalog::from_builtin())
}

pub fn xml_document_ast_from_source_bytes(
    request: XmlSourceValidationRequest<'_>,
) -> (Option<XmlDocumentAst>, Vec<Diagnostic>) {
    let report = extract_xml_parse_report(request);
    let contracts = XmlSchemaContractCatalog::from_builtin();
    let diagnostics = validate_xml_parse_report(&report, &contracts);
    let document = XmlDocumentAst {
        source: XmlDocumentSource::from_request(request, report.parameters.clone()),
        resource_kind: report.resource_kind.clone(),
        encoding_report: XmlEncodingReportAst {
            mime_charset: report.parameters.get("charset").cloned(),
            declaration_encoding: report.declaration_encoding.clone(),
            normalized_encoding: report
                .declaration_encoding
                .as_deref()
                .or_else(|| report.parameters.get("charset").map(String::as_str))
                .map(xml_normalized_encoding)
                .unwrap_or_else(|| "utf-8".to_owned()),
            decoder_status: if report
                .facts
                .iter()
                .any(|fact| fact.kind == XmlParseFactKind::UnsupportedEncoding)
            {
                "error".to_owned()
            } else {
                "decoded".to_owned()
            },
        },
        parse_facts: report.facts.clone(),
        events: report.events.clone(),
        line_ending: report.line_ending.clone(),
    };
    (Some(document), diagnostics)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum XmlSourceKind {
    Document,
    ExternalParsedEntity,
    Dtd,
}

impl XmlSourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::ExternalParsedEntity => "external-parsed-entity",
            Self::Dtd => "dtd",
        }
    }
}

fn xml_source_kind(request: &XmlSourceValidationRequest<'_>) -> XmlSourceKind {
    match request.content_type.map(content_type_essence).as_deref() {
        Some("application/xml-dtd") => XmlSourceKind::Dtd,
        Some("application/xml-external-parsed-entity")
        | Some("text/xml-external-parsed-entity") => XmlSourceKind::ExternalParsedEntity,
        _ => XmlSourceKind::Document,
    }
}

pub fn extract_xml_parse_report(request: XmlSourceValidationRequest<'_>) -> XmlParseReport {
    let kind = xml_source_kind(&request);
    let parameters = content_type_parameters(request.content_type);
    let mut report = XmlParseReport {
        source_uri: request.source_uri.to_owned(),
        content_type: request.content_type.map(str::to_owned),
        parameters,
        resource_kind: kind.as_str().to_owned(),
        declaration_encoding: None,
        facts: Vec::new(),
        events: Vec::new(),
        line_ending: None,
    };
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            report.facts.push(XmlParseFact {
                kind: XmlParseFactKind::UnsupportedEncoding,
                line: None,
                column: None,
                byte_offset: Some(error.valid_up_to() as u64),
                byte_length: Some(error.error_len().unwrap_or(1) as u64),
                message: format!("XML source must be valid UTF-8: {error}"),
            });
            return report;
        }
    };
    report.line_ending = xml_detect_line_ending_style(source).map(str::to_owned);
    let line_index = LineIndex::from_utf8(source);
    let mime_charset = report.parameters.get("charset").map(String::as_str);
    if let Some(charset) = mime_charset {
        if !xml_encoding_is_supported(charset) {
            report.facts.push(xml_fact(
                source,
                None,
                XmlParseFactKind::UnsupportedEncoding,
                format!("XML content-type charset `{charset}` is not supported"),
            ));
            return report;
        }
        if xml_normalized_encoding(charset) == "us-ascii" && !source.is_ascii() {
            report.facts.push(xml_fact(
                source,
                xml_first_non_ascii_offset(source),
                XmlParseFactKind::UnsupportedEncoding,
                "XML content-type charset `us-ascii` cannot represent non-ASCII source text"
                    .to_owned(),
            ));
            return report;
        }
    }
    if kind == XmlSourceKind::Dtd {
        if !source.trim().is_empty() {
            report.facts.push(xml_fact(
                source,
                Some(0),
                XmlParseFactKind::DtdRejected,
                "XML DTD resources are rejected until an explicit DTD policy enables them"
                    .to_owned(),
            ));
        }
        return report;
    }

    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;
    let mut element_stack: Vec<String> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut reported_multiple_roots = false;

    loop {
        let event_start = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                if element_stack.is_empty() {
                    root_count += 1;
                    if kind == XmlSourceKind::Document && root_count > 1 && !reported_multiple_roots
                    {
                        report.facts.push(xml_fact(
                            source,
                            xml_event_position(&reader, &start, false),
                            XmlParseFactKind::ParseError,
                            "XML document must have exactly one document element".to_owned(),
                        ));
                        reported_multiple_roots = true;
                    }
                }
                let (next_namespaces, attributes, mut facts) = project_xml_start_event(
                    source,
                    &start,
                    &namespace_stack,
                    Some(event_start as u64),
                );
                report.facts.append(&mut facts);
                let qualified_name = xml_qname_display(start.name().as_ref());
                report.events.push(xml_element_event(
                    report.events.len(),
                    XmlEventKind::StartElement,
                    element_stack.len(),
                    &qualified_name,
                    &next_namespaces,
                    attributes,
                    xml_source_lexeme(source, event_start, event_end),
                    XmlSourceRange::from_offsets(&line_index, event_start, event_end),
                ));
                element_stack.push(qualified_name);
                namespace_stack.push(next_namespaces);
            }
            Ok(quick_xml::events::Event::Empty(start)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                if element_stack.is_empty() {
                    root_count += 1;
                    if kind == XmlSourceKind::Document && root_count > 1 && !reported_multiple_roots
                    {
                        report.facts.push(xml_fact(
                            source,
                            xml_event_position(&reader, &start, true),
                            XmlParseFactKind::ParseError,
                            "XML document must have exactly one document element".to_owned(),
                        ));
                        reported_multiple_roots = true;
                    }
                }
                let (next_namespaces, attributes, mut facts) = project_xml_start_event(
                    source,
                    &start,
                    &namespace_stack,
                    Some(event_start as u64),
                );
                report.facts.append(&mut facts);
                let qualified_name = xml_qname_display(start.name().as_ref());
                report.events.push(xml_element_event(
                    report.events.len(),
                    XmlEventKind::EmptyElement,
                    element_stack.len(),
                    &qualified_name,
                    &next_namespaces,
                    attributes,
                    xml_source_lexeme(source, event_start, event_end),
                    XmlSourceRange::from_offsets(&line_index, event_start, event_end),
                ));
            }
            Ok(quick_xml::events::Event::End(end)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                let found = xml_qname_display(end.name().as_ref());
                let depth = element_stack.len().saturating_sub(1);
                let namespaces = namespace_stack
                    .last()
                    .cloned()
                    .unwrap_or_else(xml_initial_namespaces);
                report.events.push(xml_element_event(
                    report.events.len(),
                    XmlEventKind::EndElement,
                    depth,
                    &found,
                    &namespaces,
                    Vec::new(),
                    xml_source_lexeme(source, event_start, event_end),
                    XmlSourceRange::from_offsets(&line_index, event_start, event_end),
                ));
                match element_stack.pop() {
                    Some(expected) if expected == found => {
                        if namespace_stack.len() > 1 {
                            namespace_stack.pop();
                        }
                    }
                    Some(expected) => report.facts.push(xml_fact(
                        source,
                        Some(reader.error_position()),
                        XmlParseFactKind::ParseError,
                        format!("XML end tag `</{found}>` does not match `<{expected}>`"),
                    )),
                    None => report.facts.push(xml_fact(
                        source,
                        Some(reader.error_position()),
                        XmlParseFactKind::ParseError,
                        format!("XML end tag `</{found}>` has no matching start tag"),
                    )),
                }
            }
            Ok(quick_xml::events::Event::Decl(decl)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                report.events.push(xml_value_event(
                    report.events.len(),
                    XmlEventKind::Declaration,
                    element_stack.len(),
                    None,
                    xml_source_lexeme(source, event_start, event_end),
                    XmlSourceRange::from_offsets(&line_index, event_start, event_end),
                ));
                if let Err(error) = decl.version() {
                    report.facts.push(xml_reader_error_fact(
                        source,
                        Some(reader.error_position()),
                        &error,
                    ));
                }
                if let Some(encoding) = decl.encoding() {
                    match encoding {
                        Ok(encoding) => {
                            let encoding = String::from_utf8_lossy(encoding.as_ref());
                            report.declaration_encoding = Some(encoding.to_string());
                            if !xml_encoding_is_supported(&encoding) {
                                report.facts.push(xml_fact(
                                    source,
                                    Some(reader.error_position()),
                                    XmlParseFactKind::UnsupportedEncoding,
                                    format!(
                                        "XML declaration encoding `{encoding}` is not supported"
                                    ),
                                ));
                            } else if xml_normalized_encoding(&encoding) == "us-ascii"
                                && !source.is_ascii()
                            {
                                report.facts.push(xml_fact(
                                    source,
                                    xml_first_non_ascii_offset(source),
                                    XmlParseFactKind::UnsupportedEncoding,
                                    "XML declaration encoding `US-ASCII` cannot represent non-ASCII source text"
                                        .to_owned(),
                                ));
                            } else if let Some(charset) = mime_charset {
                                let declared = xml_normalized_encoding(&encoding);
                                let charset = xml_normalized_encoding(charset);
                                if declared != charset
                                    && !(declared == "utf-8" && charset == "us-ascii")
                                    && !(declared == "us-ascii" && charset == "utf-8")
                                {
                                    report.facts.push(xml_fact(
                                        source,
                                        Some(reader.error_position()),
                                        XmlParseFactKind::EncodingConflict,
                                        format!(
                                            "XML declaration encoding `{encoding}` conflicts with content-type charset `{charset}`"
                                        ),
                                    ));
                                }
                            }
                        }
                        Err(error) => report.facts.push(xml_attribute_error_fact(
                            source,
                            &error,
                            Some(reader.error_position()),
                        )),
                    }
                }
            }
            Ok(quick_xml::events::Event::DocType(value)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                let range = XmlSourceRange::from_offsets(&line_index, event_start, event_end);
                report.events.push(xml_value_event(
                    report.events.len(),
                    XmlEventKind::Doctype,
                    element_stack.len(),
                    Some(String::from_utf8_lossy(value.as_ref()).into_owned()),
                    xml_source_lexeme(source, event_start, event_end),
                    range,
                ));
                report.facts.push(xml_fact(
                    source,
                    Some(range.start.byte_offset),
                    XmlParseFactKind::DtdRejected,
                    "XML DTD declarations are rejected until an explicit DTD policy enables them"
                        .to_owned(),
                ));
            }
            Ok(quick_xml::events::Event::GeneralRef(reference)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                let range = XmlSourceRange::from_offsets(&line_index, event_start, event_end);
                let name = String::from_utf8_lossy(reference.as_ref()).into_owned();
                report.events.push(xml_value_event(
                    report.events.len(),
                    XmlEventKind::EntityReference,
                    element_stack.len(),
                    Some(name.clone()),
                    xml_source_lexeme(source, event_start, event_end),
                    range,
                ));
                if !xml_entity_reference_is_builtin(reference.as_ref()) {
                    report.facts.push(xml_fact(
                        source,
                        Some(range.start.byte_offset),
                        XmlParseFactKind::ExternalEntityRejected,
                        format!("XML entity reference `&{name};` is rejected"),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Text(text)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                let range = XmlSourceRange::from_offsets(&line_index, event_start, event_end);
                let value = String::from_utf8_lossy(text.as_ref()).into_owned();
                report.events.push(xml_value_event(
                    report.events.len(),
                    XmlEventKind::Text,
                    element_stack.len(),
                    Some(value),
                    xml_source_lexeme(source, event_start, event_end),
                    range,
                ));
                if kind == XmlSourceKind::Document
                    && element_stack.is_empty()
                    && !xml_bytes_are_whitespace(text.as_ref())
                {
                    report.facts.push(xml_fact(
                        source,
                        Some(range.start.byte_offset),
                        XmlParseFactKind::ParseError,
                        "XML document cannot contain character data outside the document element"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::CData(value)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                let range = XmlSourceRange::from_offsets(&line_index, event_start, event_end);
                report.events.push(xml_value_event(
                    report.events.len(),
                    XmlEventKind::Cdata,
                    element_stack.len(),
                    Some(String::from_utf8_lossy(value.as_ref()).into_owned()),
                    xml_source_lexeme(source, event_start, event_end),
                    range,
                ));
                if kind == XmlSourceKind::Document && element_stack.is_empty() {
                    report.facts.push(xml_fact(
                        source,
                        Some(range.start.byte_offset),
                        XmlParseFactKind::ParseError,
                        "XML document cannot contain CDATA outside the document element".to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Comment(value)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                report.events.push(xml_value_event(
                    report.events.len(),
                    XmlEventKind::Comment,
                    element_stack.len(),
                    Some(String::from_utf8_lossy(value.as_ref()).into_owned()),
                    xml_source_lexeme(source, event_start, event_end),
                    XmlSourceRange::from_offsets(&line_index, event_start, event_end),
                ));
            }
            Ok(quick_xml::events::Event::PI(value)) => {
                let event_end = usize::try_from(reader.buffer_position()).unwrap_or(source.len());
                report.events.push(xml_value_event(
                    report.events.len(),
                    XmlEventKind::ProcessingInstruction,
                    element_stack.len(),
                    Some(String::from_utf8_lossy(value.as_ref()).into_owned()),
                    xml_source_lexeme(source, event_start, event_end),
                    XmlSourceRange::from_offsets(&line_index, event_start, event_end),
                ));
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(error) => {
                report.facts.push(xml_reader_error_fact(
                    source,
                    Some(reader.error_position()),
                    &error,
                ));
                break;
            }
        }
    }

    if kind == XmlSourceKind::Document && root_count == 0 {
        report.facts.push(xml_fact(
            source,
            Some(0),
            XmlParseFactKind::ParseError,
            "XML document must contain a document element".to_owned(),
        ));
    }
    if let Some(unclosed) = element_stack.last() {
        report.facts.push(xml_fact(
            source,
            Some(reader.buffer_position()),
            XmlParseFactKind::ParseError,
            format!("XML start tag `<{unclosed}>` is missing a matching end tag"),
        ));
    }
    report
}

pub fn validate_xml_parse_report(
    report: &XmlParseReport,
    contracts: &XmlSchemaContractCatalog,
) -> Vec<Diagnostic> {
    report
        .facts
        .iter()
        .map(|fact| xml_diagnostic_from_fact(report, fact, contracts))
        .collect()
}

fn xml_diagnostic_from_fact(
    report: &XmlParseReport,
    fact: &XmlParseFact,
    contracts: &XmlSchemaContractCatalog,
) -> Diagnostic {
    let binding = contracts.binding_for_fact(fact.kind);
    let severity = binding
        .map(|binding| binding.severity)
        .unwrap_or_else(|| xml_fact_fallback_severity(fact.kind));
    let code = binding
        .map(|binding| binding.diagnostic_code.clone())
        .unwrap_or_else(|| format!("cem.xml.unbound_fact.{}", fact.kind.as_str()));
    Diagnostic {
        uri: Some(report.source_uri.clone()),
        line: fact.line,
        column: fact.column,
        byte_offset: fact.byte_offset,
        code,
        severity,
        message: fact.message.clone(),
        details: Some(json!({
            "xml": {
                "phase": "parse",
                "factKind": fact.kind.as_str(),
                "contract": binding.map(|binding| binding.contract.as_str()),
                "behavior": binding.and_then(|binding| binding.behavior.as_deref()),
                "policy": binding.and_then(|binding| binding.policy.as_deref()),
                "contentType": report.content_type,
                "resourceKind": report.resource_kind,
                "byteLength": fact.byte_length,
            }
        })),
        ..Diagnostic::default()
    }
}

fn xml_fact_fallback_severity(kind: XmlParseFactKind) -> Severity {
    match kind {
        XmlParseFactKind::EncodingConflict => Severity::Warning,
        XmlParseFactKind::SourceMapUnavailable => Severity::Info,
        _ => Severity::Error,
    }
}

fn project_xml_start_event(
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
    byte_offset: Option<u64>,
) -> (
    BTreeMap<String, String>,
    Vec<XmlAttributeAst>,
    Vec<XmlParseFact>,
) {
    let mut facts = Vec::new();
    let mut attributes = Vec::new();
    let mut next_namespaces = namespace_stack
        .last()
        .cloned()
        .unwrap_or_else(xml_initial_namespaces);

    for attribute in start.attributes().with_checks(false) {
        match attribute {
            Ok(attribute) => {
                let qualified_name = xml_qname_display(attribute.key.as_ref());
                let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                if qualified_name == "xmlns" {
                    next_namespaces.insert(String::new(), value.clone());
                } else if let Some(prefix) = qualified_name.strip_prefix("xmlns:") {
                    next_namespaces.insert(prefix.to_owned(), value.clone());
                }
                facts.extend(xml_entity_reference_facts(
                    source,
                    value.as_bytes(),
                    byte_offset,
                ));
                attributes.push((qualified_name, value));
            }
            Err(error) => facts.push(xml_attribute_error_fact(source, &error, byte_offset)),
        }
    }

    let element_name = xml_qname_display(start.name().as_ref());
    if let Some(prefix) = xml_qname_prefix(&element_name) {
        if !xml_prefix_is_bound(&next_namespaces, prefix) {
            facts.push(xml_unbound_namespace_prefix_fact(
                source,
                byte_offset,
                prefix,
                &element_name,
            ));
        }
    }

    let mut expanded_attributes = BTreeSet::new();
    let attributes = attributes
        .into_iter()
        .map(|(qualified_name, value)| {
            let namespace_declaration = xml_attribute_is_namespace_declaration(&qualified_name);
            let (namespace_uri, local_name) =
                xml_attribute_expanded_name(&qualified_name, &next_namespaces);
            let prefix = qualified_name
                .split_once(':')
                .map(|(prefix, _)| prefix.to_owned());
            if let Some(prefix) = xml_qname_prefix(&qualified_name) {
                if !xml_prefix_is_bound(&next_namespaces, prefix) {
                    facts.push(xml_unbound_namespace_prefix_fact(
                        source,
                        byte_offset,
                        prefix,
                        &qualified_name,
                    ));
                }
            }
            if !namespace_declaration
                && !expanded_attributes.insert((namespace_uri.clone(), local_name.clone()))
            {
                facts.push(xml_fact(
                    source,
                    byte_offset,
                    XmlParseFactKind::DuplicateAttribute,
                    format!(
                        "XML element `<{element_name}>` has a duplicate attribute `{qualified_name}`"
                    ),
                ));
            }
            XmlAttributeAst {
                qualified_name,
                local_name,
                prefix,
                namespace_uri: (!namespace_uri.is_empty()).then_some(namespace_uri),
                value,
                value_source_range: None,
                entity_decoded_value: None,
                entity_decoded_source_map: None,
            }
        })
        .collect::<Vec<_>>();

    (next_namespaces, attributes, facts)
}

fn xml_element_event(
    index: usize,
    kind: XmlEventKind,
    depth: usize,
    qualified_name: &str,
    namespaces: &BTreeMap<String, String>,
    attributes: Vec<XmlAttributeAst>,
    lexeme: String,
    source_range: XmlSourceRange,
) -> XmlEventAst {
    let (prefix, local_name) = xml_qname_parts(qualified_name);
    let namespace_uri = namespaces
        .get(prefix.as_deref().unwrap_or_default())
        .filter(|value| !value.is_empty())
        .cloned();
    let mut event = XmlEventAst {
        index,
        kind,
        depth,
        qualified_name: Some(qualified_name.to_owned()),
        local_name: Some(local_name),
        prefix,
        namespace_uri,
        attributes,
        value: None,
        whitespace_only: false,
        lexeme,
        source_range,
    };
    xml_attach_attribute_value_sources(&mut event);
    event
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XmlLexicalAttributeValue {
    qualified_name: String,
    value: String,
    source_range: XmlSourceRange,
}

fn xml_attach_attribute_value_sources(event: &mut XmlEventAst) {
    let lexical_values = xml_lexical_attribute_values(event);
    for (attribute, lexical) in event.attributes.iter_mut().zip(lexical_values) {
        if attribute.qualified_name != lexical.qualified_name || attribute.value != lexical.value {
            continue;
        }
        attribute.value_source_range = Some(lexical.source_range);
        if let Some((decoded, source_map)) =
            xml_entity_decode_attribute_value(&lexical.value, lexical.source_range)
        {
            attribute.entity_decoded_value = Some(decoded);
            attribute.entity_decoded_source_map = Some(source_map);
        }
    }
}

fn xml_lexical_attribute_values(event: &XmlEventAst) -> Vec<XmlLexicalAttributeValue> {
    let mut values = Vec::new();
    let mut qualified_name = None;
    for token in xml_event_markup_tokens(event) {
        match token.kind {
            XmlMarkupTokenKind::AttributeName => qualified_name = Some(token.text),
            XmlMarkupTokenKind::AttributeValue => {
                let Some(name) = qualified_name.take() else {
                    continue;
                };
                let bytes = token.text.as_bytes();
                if bytes.len() < 2
                    || !matches!(bytes[0], b'\'' | b'"')
                    || bytes.last() != Some(&bytes[0])
                {
                    continue;
                }
                values.push(XmlLexicalAttributeValue {
                    qualified_name: name,
                    value: token.text[1..token.text.len() - 1].to_owned(),
                    source_range: XmlSourceRange {
                        start: XmlSourcePosition {
                            line: token.source_range.start.line,
                            column: token.source_range.start.column.saturating_add(1),
                            byte_offset: token.source_range.start.byte_offset.saturating_add(1),
                        },
                        byte_length: token.source_range.byte_length.saturating_sub(2),
                    },
                });
            }
            _ => {}
        }
    }
    values
}

fn xml_entity_decode_attribute_value(
    lexical_value: &str,
    value_source_range: XmlSourceRange,
) -> Option<(String, XmlAttributeValueSourceMap)> {
    let mut decoded = String::new();
    let mut spans = Vec::new();
    let mut boundaries = vec![XmlAttributeValueSourceBoundary {
        decoded_byte_offset: 0,
        source_position: value_source_range.start,
    }];
    let mut source_offset = 0usize;

    while source_offset < lexical_value.len() {
        let (scalar, source_end) = if lexical_value.as_bytes()[source_offset] == b'&' {
            let reference_end = lexical_value[source_offset + 1..].find(';')? + source_offset + 2;
            let reference = &lexical_value[source_offset + 1..reference_end - 1];
            (
                xml_decode_attribute_entity_reference(reference)?,
                reference_end,
            )
        } else {
            let scalar = lexical_value[source_offset..].chars().next()?;
            (scalar, source_offset + scalar.len_utf8())
        };

        let decoded_start = decoded.len();
        decoded.push(scalar);
        spans.push(XmlAttributeValueSourceSpan {
            decoded_byte_range: ByteRange::new(
                decoded_start as u64,
                u32::try_from(scalar.len_utf8()).ok()?,
            ),
            source_range: xml_source_range_within(
                value_source_range,
                lexical_value,
                source_offset,
                source_end,
            ),
        });
        boundaries.push(XmlAttributeValueSourceBoundary {
            decoded_byte_offset: decoded.len() as u64,
            source_position: xml_source_range_within(
                value_source_range,
                lexical_value,
                source_end,
                source_end,
            )
            .start,
        });
        source_offset = source_end;
    }

    Some((
        decoded,
        XmlAttributeValueSourceMap {
            decoded_byte_length: boundaries
                .last()
                .map_or(0, |boundary| boundary.decoded_byte_offset),
            source_range: value_source_range,
            boundaries,
            spans,
        },
    ))
}

fn xml_decode_attribute_entity_reference(reference: &str) -> Option<char> {
    match reference {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "apos" => Some('\''),
        "quot" => Some('"'),
        _ => {
            let code_point = if let Some(hex) = reference.strip_prefix("#x") {
                u32::from_str_radix(hex, 16).ok()?
            } else if let Some(decimal) = reference.strip_prefix('#') {
                decimal.parse::<u32>().ok()?
            } else {
                return None;
            };
            xml_code_point_is_valid(code_point).then(|| char::from_u32(code_point))?
        }
    }
}

fn xml_code_point_is_valid(code_point: u32) -> bool {
    matches!(
        code_point,
        0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
    )
}

fn xml_value_event(
    index: usize,
    kind: XmlEventKind,
    depth: usize,
    value: Option<String>,
    lexeme: String,
    source_range: XmlSourceRange,
) -> XmlEventAst {
    let whitespace_only = value
        .as_deref()
        .is_some_and(|value| value.chars().all(char::is_whitespace));
    XmlEventAst {
        index,
        kind,
        depth,
        qualified_name: None,
        local_name: None,
        prefix: None,
        namespace_uri: None,
        attributes: Vec::new(),
        value,
        lexeme,
        whitespace_only,
        source_range,
    }
}

fn xml_qname_parts(qualified_name: &str) -> (Option<String>, String) {
    match qualified_name.split_once(':') {
        Some((prefix, local_name)) => (Some(prefix.to_owned()), local_name.to_owned()),
        None => (None, qualified_name.to_owned()),
    }
}

fn xml_source_lexeme(source: &str, start: usize, end: usize) -> String {
    source
        .get(start.min(source.len())..end.min(source.len()))
        .unwrap_or_default()
        .to_owned()
}

fn xml_initial_namespaces() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "xml".to_owned(),
            "http://www.w3.org/XML/1998/namespace".to_owned(),
        ),
        (
            "xmlns".to_owned(),
            "http://www.w3.org/2000/xmlns/".to_owned(),
        ),
    ])
}

fn xml_attribute_expanded_name(
    qualified_name: &str,
    namespaces: &BTreeMap<String, String>,
) -> (String, String) {
    if qualified_name == "xmlns" {
        return (
            "http://www.w3.org/2000/xmlns/".to_owned(),
            "xmlns".to_owned(),
        );
    }
    if let Some((prefix, local_name)) = qualified_name.split_once(':') {
        let namespace_uri = namespaces.get(prefix).cloned().unwrap_or_default();
        (namespace_uri, local_name.to_owned())
    } else {
        (String::new(), qualified_name.to_owned())
    }
}

fn xml_qname_prefix(qualified_name: &str) -> Option<&str> {
    qualified_name
        .split_once(':')
        .map(|(prefix, _)| prefix)
        .filter(|prefix| !prefix.is_empty() && *prefix != "xml")
}

fn xml_prefix_is_bound(namespaces: &BTreeMap<String, String>, prefix: &str) -> bool {
    namespaces
        .get(prefix)
        .is_some_and(|namespace| !namespace.trim().is_empty())
}

fn xml_attribute_is_namespace_declaration(qualified_name: &str) -> bool {
    qualified_name == "xmlns" || qualified_name.starts_with("xmlns:")
}

fn xml_qname_display(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}

fn xml_entity_reference_is_builtin(name: &[u8]) -> bool {
    name.starts_with(b"#") || matches!(name, b"amp" | b"lt" | b"gt" | b"apos" | b"quot")
}

fn xml_entity_reference_facts(
    source: &str,
    value: &[u8],
    byte_offset: Option<u64>,
) -> Vec<XmlParseFact> {
    let mut facts = Vec::new();
    let mut remaining = value;
    while let Some(start) = remaining.iter().position(|byte| *byte == b'&') {
        let after_amp = &remaining[start + 1..];
        let Some(end) = after_amp.iter().position(|byte| *byte == b';') else {
            break;
        };
        let reference = &after_amp[..end];
        if !xml_entity_reference_is_builtin(reference) {
            facts.push(xml_fact(
                source,
                byte_offset,
                XmlParseFactKind::ExternalEntityRejected,
                format!(
                    "XML entity reference `&{};` is rejected",
                    String::from_utf8_lossy(reference)
                ),
            ));
        }
        remaining = &after_amp[end + 1..];
    }
    facts
}

fn xml_bytes_are_whitespace(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .map(|value| value.chars().all(char::is_whitespace))
        .unwrap_or(false)
}

fn xml_first_non_ascii_offset(source: &str) -> Option<u64> {
    source
        .find(|character: char| !character.is_ascii())
        .and_then(|offset| u64::try_from(offset).ok())
}

fn xml_encoding_is_supported(encoding: &str) -> bool {
    matches!(
        xml_normalized_encoding(encoding).as_str(),
        "utf-8" | "us-ascii"
    )
}

fn xml_normalized_encoding(encoding: &str) -> String {
    encoding.trim().trim_matches('"').to_ascii_lowercase()
}

fn xml_event_position(
    reader: &quick_xml::Reader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
    empty: bool,
) -> Option<u64> {
    let markup_overhead = if empty { 3 } else { 2 };
    reader
        .buffer_position()
        .checked_sub(start.as_ref().len() as u64 + markup_overhead)
}

fn xml_reader_error_fact(
    source: &str,
    byte_offset: Option<u64>,
    error: &quick_xml::Error,
) -> XmlParseFact {
    let kind = match error {
        quick_xml::Error::Encoding(_) => XmlParseFactKind::UnsupportedEncoding,
        quick_xml::Error::InvalidAttr(quick_xml::events::attributes::AttrError::Duplicated(
            _,
            _,
        )) => XmlParseFactKind::DuplicateAttribute,
        quick_xml::Error::Namespace(_) => XmlParseFactKind::UnboundNamespacePrefix,
        _ => XmlParseFactKind::ParseError,
    };
    xml_fact(
        source,
        byte_offset,
        kind,
        format!("XML parse error: {error}"),
    )
}

fn xml_attribute_error_fact(
    source: &str,
    error: &quick_xml::events::attributes::AttrError,
    byte_offset: Option<u64>,
) -> XmlParseFact {
    let kind = match error {
        quick_xml::events::attributes::AttrError::Duplicated(_, _) => {
            XmlParseFactKind::DuplicateAttribute
        }
        _ => XmlParseFactKind::ParseError,
    };
    xml_fact(
        source,
        byte_offset,
        kind,
        format!("XML attribute parse error: {error}"),
    )
}

fn xml_unbound_namespace_prefix_fact(
    source: &str,
    byte_offset: Option<u64>,
    prefix: &str,
    qualified_name: &str,
) -> XmlParseFact {
    xml_fact(
        source,
        byte_offset,
        XmlParseFactKind::UnboundNamespacePrefix,
        format!("XML namespace prefix `{prefix}` is not bound for `{qualified_name}`"),
    )
}

fn xml_fact(
    source: &str,
    byte_offset: Option<u64>,
    kind: XmlParseFactKind,
    message: String,
) -> XmlParseFact {
    let (line, column) = byte_offset
        .and_then(|offset| usize::try_from(offset).ok())
        .map(|offset| line_col(source, offset))
        .map(|(line, column)| (Some(line), Some(column)))
        .unwrap_or((None, None));
    XmlParseFact {
        kind,
        line,
        column,
        byte_offset,
        byte_length: byte_offset.map(|_| 1),
        message,
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
            let (key, value) = part.split_once('=')?;
            Some((
                key.trim().to_ascii_lowercase(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect()
}

fn xml_detect_line_ending_style(source: &str) -> Option<&'static str> {
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

fn line_col(source: &str, byte_offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut column = 1u32;
    let limit = byte_offset.min(source.len());
    for byte in source[..limit].bytes() {
        if byte == b'\n' {
            line = line.saturating_add(1);
            column = 1;
        } else {
            column = column.saturating_add(1);
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(source: &str) -> Vec<Diagnostic> {
        validate_xml_source_bytes(XmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.xml",
            content_type: Some("application/xml"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn xml_source_validator_accepts_namespaced_document() {
        let request = XmlSourceValidationRequest {
            bytes: br#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns:meta="https://example.test/meta" meta:version="1">
  <item id="a1">Alpha</item>
</catalog>
"#,
            source_uri: "fixture.xml",
            content_type: Some("text/xml; charset=utf-8"),
        };
        let (document, diagnostics) = xml_document_ast_from_source_bytes(request);

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("typed XML document");
        assert_eq!(document.resource_kind, "document");
        assert_eq!(document.encoding_report.normalized_encoding, "utf-8");
        assert!(document.events.iter().any(|event| {
            event.kind == XmlEventKind::StartElement
                && event.qualified_name.as_deref() == Some("catalog")
                && event.attributes.iter().any(|attribute| {
                    attribute.qualified_name == "meta:version"
                        && attribute.namespace_uri.as_deref() == Some("https://example.test/meta")
                })
        }));
        assert!(document
            .events
            .iter()
            .all(|event| event.source_range.byte_length > 0));
        assert!(document.to_cemt_subject()["events"]
            .as_array()
            .is_some_and(|events| !events.is_empty()));
    }

    #[test]
    fn xml_attribute_ast_retains_entity_decoded_value_and_exact_source_spans() {
        let source = "<root\n  select=\"A &lt; B &#x1F4B0; &#8364; &amp; 💡\"\n/>\n";
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.xml",
                content_type: Some(XML_CONTENT_TYPE),
            });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("typed XML document");
        let attribute = document.events[0]
            .attributes
            .iter()
            .find(|attribute| attribute.qualified_name == "select")
            .expect("select attribute");
        assert_eq!(attribute.value, "A &lt; B &#x1F4B0; &#8364; &amp; 💡");
        assert_eq!(
            attribute.entity_decoded_value.as_deref(),
            Some("A < B 💰 € & 💡")
        );

        let value_range = attribute.value_source_range.expect("value source range");
        let value_start = value_range.start.byte_offset as usize;
        let value_end = value_start + value_range.byte_length as usize;
        assert_eq!(&source[value_start..value_end], attribute.value);
        assert_eq!(value_range.start.line, 2);

        let decoded = attribute
            .entity_decoded_value
            .as_deref()
            .expect("entity-decoded value");
        let mut decoded_cursor = 0u64;
        let mut source_cursor = value_range.start.byte_offset;
        let source_map = attribute
            .entity_decoded_source_map
            .as_ref()
            .expect("decoded attribute source map");
        assert_eq!(source_map.decoded_byte_length(), decoded.len() as u64);
        assert_eq!(source_map.source_range(), value_range);
        assert_eq!(source_map.boundaries().len(), source_map.spans().len() + 1);
        for span in source_map.spans() {
            assert_eq!(span.decoded_byte_range.start, decoded_cursor);
            assert!(span.source_range.start.byte_offset >= source_cursor);
            decoded_cursor = span.decoded_byte_range.end();
            source_cursor = span.source_range.start.byte_offset + span.source_range.byte_length;
        }
        assert_eq!(decoded_cursor, decoded.len() as u64);
        assert_eq!(source_cursor, value_end as u64);

        for (decoded_scalar, source_lexeme) in [
            ('<', "&lt;"),
            ('💰', "&#x1F4B0;"),
            ('€', "&#8364;"),
            ('&', "&amp;"),
            ('💡', "💡"),
        ] {
            let decoded_offset = decoded.find(decoded_scalar).expect("decoded scalar") as u64;
            let span = source_map
                .spans()
                .iter()
                .find(|span| span.decoded_byte_range.start == decoded_offset)
                .expect("decoded scalar source span");
            let start = span.source_range.start.byte_offset as usize;
            let end = start + span.source_range.byte_length as usize;
            assert_eq!(&source[start..end], source_lexeme);
        }
    }

    #[test]
    fn xml_attribute_value_source_map_projects_exact_scalar_boundaries() {
        let source = "<root\n  select=\"price &lt; 10 and &#x1F4B0;\"\n/>\n";
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.xml",
                content_type: Some(XML_CONTENT_TYPE),
            });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("typed XML document");
        let attribute = &document.events[0].attributes[0];
        let decoded = attribute
            .entity_decoded_value
            .as_deref()
            .expect("entity-decoded value");
        let source_map = attribute
            .entity_decoded_source_map
            .as_ref()
            .expect("typed decoded attribute source map");

        let project = |decoded_lexeme: &str| {
            let start = decoded.find(decoded_lexeme).expect("decoded lexeme") as u64;
            source_map
                .project_range(ByteRange::new(
                    start,
                    u32::try_from(decoded_lexeme.len()).expect("test range length"),
                ))
                .expect("projected XML source range")
        };
        let source_slice = |range: XmlSourceRange| {
            let start = range.start.byte_offset as usize;
            &source[start..start + range.byte_length as usize]
        };

        assert_eq!(source_slice(project("price")), "price");
        assert_eq!(source_slice(project("< 10")), "&lt; 10");
        assert_eq!(source_slice(project("💰")), "&#x1F4B0;");
        assert_eq!(
            source_map.project_range(ByteRange::new(
                0,
                u32::try_from(decoded.len()).expect("decoded test length"),
            )),
            attribute.value_source_range
        );

        let entity_decoded_start = decoded.find('<').expect("decoded entity") as u64;
        let entity_source_start = source.find("&lt;").expect("entity source") as u64;
        assert_eq!(
            source_map
                .project_range(ByteRange::new(entity_decoded_start, 0))
                .expect("position before entity")
                .start
                .byte_offset,
            entity_source_start
        );
        assert_eq!(
            source_map
                .project_range(ByteRange::new(entity_decoded_start + 1, 0))
                .expect("position after entity")
                .start
                .byte_offset,
            entity_source_start + "&lt;".len() as u64
        );

        let emoji_start = decoded.find('💰').expect("decoded emoji") as u64;
        assert_eq!(
            source_map.project_range(ByteRange::new(emoji_start + 1, 0)),
            None,
            "an interior UTF-8 byte is not a scalar boundary"
        );
        assert_eq!(
            source_map.project_range(ByteRange::new(decoded.len() as u64 + 1, 0)),
            None,
            "an out-of-bounds position must fail closed"
        );
    }

    #[test]
    fn xml_empty_attribute_value_source_map_projects_its_only_boundary() {
        let source = r#"<root value=""/>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.xml",
                content_type: Some(XML_CONTENT_TYPE),
            });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("typed XML document");
        let attribute = &document.events[0].attributes[0];
        let source_map = attribute
            .entity_decoded_source_map
            .as_ref()
            .expect("empty decoded attribute source map");
        assert_eq!(
            source_map.project_range(ByteRange::new(0, 0)),
            attribute.value_source_range
        );
    }

    #[test]
    fn xml_attribute_ast_keeps_unresolved_entities_lexical_without_a_decoded_map() {
        let source = r#"<root value="known &amp; unresolved &example;"/>"#;
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.xml",
                content_type: Some(XML_CONTENT_TYPE),
            });

        assert!(has_code(&diagnostics, "cem.xml.external_entity_rejected"));
        let document = document.expect("typed XML document with rejected entity fact");
        let attribute = &document.events[0].attributes[0];
        assert_eq!(attribute.value, "known &amp; unresolved &example;");
        assert!(attribute.value_source_range.is_some());
        assert_eq!(attribute.entity_decoded_value, None);
        assert!(attribute.entity_decoded_source_map.is_none());
    }

    #[test]
    fn xml_attribute_mapping_source_has_no_serialized_or_format_specific_bridge() {
        let source = include_str!("xml.rs");
        let mapping = source
            .split("fn xml_attach_attribute_value_sources")
            .nth(1)
            .and_then(|source| source.split("fn xml_value_event").next())
            .expect("generic XML attribute mapping implementation region");
        for forbidden in [
            "serde_json",
            "to_value",
            "from_value",
            "serialize",
            "deserialize",
            "replacement_tree",
            "xslt",
        ] {
            assert!(
                !mapping.contains(forbidden),
                "XML attribute mapping must not contain `{forbidden}`"
            );
        }
        let source_map_contract = source
            .split("pub struct XmlAttributeValueSourceMap")
            .nth(1)
            .and_then(|source| source.split("impl XmlAttributeAst").next())
            .expect("typed XML attribute source-map contract region");
        for forbidden in ["serde", "Serialize", "Deserialize", "serde_json"] {
            assert!(
                !source_map_contract.contains(forbidden),
                "XML attribute source-map contract must not contain `{forbidden}`"
            );
        }
    }

    #[test]
    fn xml_parse_facts_resolve_diagnostics_from_schema_bindings() {
        let catalog = XmlSchemaContractCatalog::from_builtin();
        for (kind, code, severity) in [
            (
                XmlParseFactKind::ParseError,
                "cem.xml.parse_error",
                Severity::Error,
            ),
            (
                XmlParseFactKind::EncodingConflict,
                "cem.xml.encoding_conflict",
                Severity::Warning,
            ),
            (
                XmlParseFactKind::SourceMapUnavailable,
                "cem.xml.source_map_unavailable",
                Severity::Info,
            ),
        ] {
            let binding = catalog
                .binding_for_fact(kind)
                .unwrap_or_else(|| panic!("{} binding", kind.as_str()));
            assert_eq!(binding.diagnostic_code, code);
            assert_eq!(binding.severity, severity);
            assert_eq!(binding.behavior.as_deref(), Some("xml-parse-report-fact"));
        }
    }

    #[test]
    fn xml_source_validator_reports_mismatched_tag() {
        let diagnostics = validate("<root><item></root>\n");

        assert!(has_code(&diagnostics, "cem.xml.parse_error"));
    }

    #[test]
    fn xml_source_validator_reports_unbound_namespace_prefix() {
        let diagnostics = validate("<root><meta:item/></root>\n");

        assert!(has_code(&diagnostics, "cem.xml.unbound_namespace_prefix"));
    }

    #[test]
    fn xml_source_validator_reports_dtd_rejected() {
        let diagnostics = validate(
            r#"<!DOCTYPE root SYSTEM "file:///etc/passwd">
<root/>
"#,
        );

        assert!(has_code(&diagnostics, "cem.xml.dtd_rejected"));
    }

    #[test]
    fn xml_source_validator_rejects_non_ascii_text_for_ascii_charset() {
        let diagnostics = validate_xml_source_bytes(XmlSourceValidationRequest {
            bytes: "<root>caf\u{00e9}</root>".as_bytes(),
            source_uri: "fixture.xml",
            content_type: Some("application/xml; charset=us-ascii"),
        });

        assert!(has_code(&diagnostics, "cem.xml.unsupported_encoding"));
    }
}
