use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::registry::{JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const JSON_PACKAGE_ID: &str = "json";

#[derive(Debug, Clone, Copy)]
pub struct JsonSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDocumentAst {
    pub source: JsonDocumentSource,
    pub encoding: String,
    pub encoding_report: JsonEncodingReportAst,
    pub parse_facts: Vec<JsonDocumentParseFact>,
    pub root: Option<JsonValueAst>,
    pub line_ending: Option<String>,
}

impl JsonDocumentAst {
    pub fn to_cemt_subject(&self) -> Value {
        let mut document = serde_json::Map::new();
        document.insert("kind".to_owned(), json!("json-document"));
        document.insert("contentType".to_owned(), json!(JSON_CONTENT_TYPE));
        document.insert("schema".to_owned(), json!(JSON_VALUE_SCHEMA_URI));
        document.insert("source".to_owned(), self.source.to_cemt_subject());
        document.insert("encoding".to_owned(), json!(self.encoding));
        document.insert(
            "encodingReport".to_owned(),
            self.encoding_report.to_cemt_subject(),
        );
        document.insert(
            "parseFacts".to_owned(),
            Value::Array(
                self.parse_facts
                    .iter()
                    .map(JsonDocumentParseFact::to_cemt_subject)
                    .collect(),
            ),
        );
        document.insert(
            "root".to_owned(),
            self.root
                .as_ref()
                .map(JsonValueAst::to_cemt_subject)
                .unwrap_or(Value::Null),
        );
        if let Some(line_ending) = self.line_ending.as_deref() {
            document.insert("lineEnding".to_owned(), json!(line_ending));
        }
        Value::Object(document)
    }

    pub fn to_json_value(&self) -> Option<Value> {
        self.root.as_ref().map(JsonValueAst::to_json_value)
    }

    pub fn to_json_text(&self, formatter_profile: &str) -> String {
        let style = JsonWriterStyle {
            pretty: matches!(formatter_profile, "pretty" | "tabular"),
            indent_width: 2,
        };
        self.root
            .as_ref()
            .map(|root| root.to_json_text(style, 0))
            .unwrap_or_else(|| "null".to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl JsonDocumentSource {
    fn from_request(
        request: JsonSourceValidationRequest<'_>,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        Self {
            uri: request.source_uri.to_owned(),
            content_type: request.content_type.unwrap_or(JSON_CONTENT_TYPE).to_owned(),
            media_type: request
                .content_type
                .map(json_content_type_essence)
                .unwrap_or(JSON_CONTENT_TYPE.to_owned()),
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
pub struct JsonEncodingReportAst {
    pub declared_charset: Option<String>,
    pub normalized_charset: String,
    pub decoder_status: String,
    pub invalid_byte_offset: Option<u64>,
}

impl JsonEncodingReportAst {
    fn from_report(report: &JsonParseReport) -> Self {
        Self {
            declared_charset: report
                .content_type
                .as_deref()
                .and_then(|content_type| content_type_parameter(content_type, "charset")),
            normalized_charset: json_normalized_charset_for_report(report).to_owned(),
            decoder_status: json_decoder_status_for_report(report).to_owned(),
            invalid_byte_offset: report.facts.iter().find_map(|fact| match fact.kind {
                JsonParseFactKind::UnsupportedEncoding => fact.byte_offset,
                _ => None,
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
pub struct JsonDocumentParseFact {
    pub kind: JsonParseFactKind,
    pub contract: Option<String>,
    pub behavior: Option<String>,
    pub diagnostic_code: Option<String>,
    pub diagnostic_severity: Option<String>,
    pub recoverable: bool,
    pub fatal: bool,
    pub member_name: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub byte_length: Option<u64>,
    pub message: String,
}

impl JsonDocumentParseFact {
    fn from_parse_fact(fact: &JsonParseFact, contracts: &JsonSchemaContractCatalog) -> Self {
        let binding = contracts.binding_for_fact(fact.kind);
        let fatal = binding.is_some_and(|binding| binding.severity.is_hard_violation());
        Self {
            kind: fact.kind,
            contract: binding.map(|binding| binding.contract.clone()),
            behavior: binding.and_then(|binding| binding.behavior.clone()),
            diagnostic_code: binding.map(|binding| binding.diagnostic_code.clone()),
            diagnostic_severity: binding
                .map(|binding| json_severity_name(binding.severity).to_owned()),
            recoverable: !fatal,
            fatal,
            member_name: fact.member_name.clone(),
            line: fact.line,
            column: fact.column,
            byte_offset: fact.byte_offset,
            byte_length: fact.byte_length,
            message: fact.message.clone(),
        }
    }

    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "contract": self.contract,
            "behavior": self.behavior,
            "diagnosticCode": self.diagnostic_code,
            "diagnosticSeverity": self.diagnostic_severity,
            "recoverable": self.recoverable,
            "fatal": self.fatal,
            "memberName": self.member_name,
            "line": self.line,
            "column": self.column,
            "byteOffset": self.byte_offset,
            "byteLength": self.byte_length,
            "message": self.message,
            "sourceRange": {
                "byteOffset": self.byte_offset,
                "byteLength": self.byte_length,
                "line": self.line,
                "column": self.column,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonMemberAst {
    pub index: usize,
    pub name: String,
    pub name_range: JsonSourceRange,
    pub range: JsonSourceRange,
    pub value: JsonValueAst,
}

impl JsonMemberAst {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "name": self.name,
            "nameSourceRange": self.name_range.to_cemt_subject(),
            "nameSourceMap": self.name_range.source_map(),
            "sourceRange": self.range.to_cemt_subject(),
            "sourceMap": self.range.source_map(),
            "value": self.value.to_cemt_subject(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValueAst {
    Object {
        range: JsonSourceRange,
        members: Vec<JsonMemberAst>,
    },
    Array {
        range: JsonSourceRange,
        items: Vec<JsonValueAst>,
    },
    String {
        range: JsonSourceRange,
        value: String,
        lexeme: String,
    },
    Number {
        range: JsonSourceRange,
        lexeme: String,
        number_kind: JsonNumberKind,
    },
    Boolean {
        range: JsonSourceRange,
        value: bool,
    },
    Null {
        range: JsonSourceRange,
    },
}

impl JsonValueAst {
    pub fn range(&self) -> JsonSourceRange {
        match self {
            Self::Object { range, .. }
            | Self::Array { range, .. }
            | Self::String { range, .. }
            | Self::Number { range, .. }
            | Self::Boolean { range, .. }
            | Self::Null { range } => *range,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Object { .. } => "object",
            Self::Array { .. } => "array",
            Self::String { .. } => "string",
            Self::Number { .. } => "number",
            Self::Boolean { .. } => "boolean",
            Self::Null { .. } => "null",
        }
    }

    fn to_json_value(&self) -> Value {
        match self {
            Self::Object { members, .. } => {
                let mut object = serde_json::Map::new();
                for member in members {
                    object.insert(member.name.clone(), member.value.to_json_value());
                }
                Value::Object(object)
            }
            Self::Array { items, .. } => Value::Array(
                items
                    .iter()
                    .map(JsonValueAst::to_json_value)
                    .collect::<Vec<_>>(),
            ),
            Self::String { value, .. } => Value::String(value.clone()),
            Self::Number { lexeme, .. } => serde_json::from_str::<Value>(lexeme)
                .unwrap_or_else(|_| Value::String(lexeme.clone())),
            Self::Boolean { value, .. } => Value::Bool(*value),
            Self::Null { .. } => Value::Null,
        }
    }

    fn to_cemt_subject(&self) -> Value {
        let range = self.range();
        let mut value = serde_json::Map::new();
        value.insert("kind".to_owned(), json!(self.kind()));
        value.insert("sourceRange".to_owned(), range.to_cemt_subject());
        value.insert("sourceMap".to_owned(), json!(range.source_map()));
        match self {
            Self::Object { members, .. } => {
                value.insert(
                    "members".to_owned(),
                    Value::Array(members.iter().map(JsonMemberAst::to_cemt_subject).collect()),
                );
            }
            Self::Array { items, .. } => {
                value.insert(
                    "items".to_owned(),
                    Value::Array(items.iter().map(JsonValueAst::to_cemt_subject).collect()),
                );
            }
            Self::String {
                value: text,
                lexeme,
                ..
            } => {
                value.insert("value".to_owned(), json!(text));
                value.insert("lexeme".to_owned(), json!(lexeme));
            }
            Self::Number {
                lexeme,
                number_kind,
                ..
            } => {
                value.insert("lexeme".to_owned(), json!(lexeme));
                value.insert("numberKind".to_owned(), json!(number_kind.as_str()));
            }
            Self::Boolean { value: boolean, .. } => {
                value.insert("value".to_owned(), json!(boolean));
            }
            Self::Null { .. } => {}
        }
        Value::Object(value)
    }

    fn to_json_text(&self, style: JsonWriterStyle, depth: usize) -> String {
        match self {
            Self::Object { members, .. } => {
                if members.is_empty() {
                    return "{}".to_owned();
                }
                if !style.pretty {
                    let fields = members
                        .iter()
                        .map(|member| {
                            format!(
                                "{}:{}",
                                json_string_literal(&member.name),
                                member.value.to_json_text(style, depth + 1)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    return format!("{{{fields}}}");
                }
                let child_indent = style.indent(depth + 1);
                let closing_indent = style.indent(depth);
                let fields = members
                    .iter()
                    .map(|member| {
                        format!(
                            "{child_indent}{}: {}",
                            json_string_literal(&member.name),
                            member.value.to_json_text(style, depth + 1)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{{\n{fields}\n{closing_indent}}}")
            }
            Self::Array { items, .. } => {
                if items.is_empty() {
                    return "[]".to_owned();
                }
                if !style.pretty {
                    let values = items
                        .iter()
                        .map(|item| item.to_json_text(style, depth + 1))
                        .collect::<Vec<_>>()
                        .join(",");
                    return format!("[{values}]");
                }
                let child_indent = style.indent(depth + 1);
                let closing_indent = style.indent(depth);
                let values = items
                    .iter()
                    .map(|item| format!("{child_indent}{}", item.to_json_text(style, depth + 1)))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("[\n{values}\n{closing_indent}]")
            }
            Self::String { value, .. } => json_string_literal(value),
            Self::Number { lexeme, .. } => lexeme.clone(),
            Self::Boolean { value, .. } => value.to_string(),
            Self::Null { .. } => "null".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct JsonWriterStyle {
    pretty: bool,
    indent_width: usize,
}

impl JsonWriterStyle {
    fn indent(self, depth: usize) -> String {
        " ".repeat(self.indent_width.saturating_mul(depth))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonNumberKind {
    Integer,
    Decimal,
    Exponent,
}

impl JsonNumberKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Decimal => "decimal",
            Self::Exponent => "exponent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonSourceRange {
    pub start: JsonSourcePosition,
    pub byte_length: u64,
}

impl JsonSourceRange {
    fn from_offsets(line_index: &LineIndex, start: usize, end: usize) -> Self {
        let coordinate = line_index.project(start as u64);
        Self {
            start: JsonSourcePosition {
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
                    content_type: JSON_CONTENT_TYPE.to_owned(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonParseReport {
    pub source_uri: String,
    pub content_type: Option<String>,
    pub parameters: BTreeMap<String, String>,
    pub facts: Vec<JsonParseFact>,
    pub root: Option<JsonValueAst>,
    pub line_ending: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonParseFact {
    pub kind: JsonParseFactKind,
    pub member_name: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub byte_length: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JsonParseFactKind {
    ParseError,
    UnsupportedEncoding,
    DuplicateMemberName,
    SourceMapUnavailable,
}

impl JsonParseFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseError => "parse-error",
            Self::UnsupportedEncoding => "unsupported-encoding",
            Self::DuplicateMemberName => "duplicate-member-name",
            Self::SourceMapUnavailable => "source-map-unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSchemaContractCatalog {
    pub fact_bindings: BTreeMap<String, JsonDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

impl JsonSchemaContractCatalog {
    pub fn from_builtin() -> Self {
        let source = crate::schema::package_sources::builtin_schema_package_source(JSON_PACKAGE_ID)
            .expect("built-in JSON schema package source must be registered");
        Self::from_schema_source(source.schema_source)
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(JSON_VALUE_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                let fact_kind = constraint.fact_kind.as_deref()?.trim();
                if fact_kind.is_empty() {
                    return None;
                }
                let diagnostic_code = constraint.diagnostic.as_deref()?.trim();
                if diagnostic_code.is_empty() {
                    return None;
                }
                let diagnostic = model.diagnostics.get(diagnostic_code)?;
                Some((
                    fact_kind.to_owned(),
                    JsonDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: JsonParseFactKind) -> Option<&JsonDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_json_source_bytes(request: JsonSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let contracts = JsonSchemaContractCatalog::from_builtin();
    let report = extract_json_parse_report(request);
    validate_json_parse_report(&report, &contracts)
}

pub fn json_document_ast_from_source_bytes(
    request: JsonSourceValidationRequest<'_>,
) -> (Option<JsonDocumentAst>, Vec<Diagnostic>) {
    let report = extract_json_parse_report(request);
    let contracts = JsonSchemaContractCatalog::from_builtin();
    let diagnostics = validate_json_parse_report(&report, &contracts);
    let parse_facts = report
        .facts
        .iter()
        .map(|fact| JsonDocumentParseFact::from_parse_fact(fact, &contracts))
        .collect::<Vec<_>>();
    let fatal = parse_facts.iter().any(|fact| fact.fatal);
    let document = (!fatal).then(|| JsonDocumentAst {
        source: JsonDocumentSource::from_request(request, report.parameters.clone()),
        encoding: json_normalized_charset_for_report(&report).to_owned(),
        encoding_report: JsonEncodingReportAst::from_report(&report),
        parse_facts,
        root: report.root.clone(),
        line_ending: report.line_ending.clone(),
    });

    (document, diagnostics)
}

pub fn extract_json_parse_report(request: JsonSourceValidationRequest<'_>) -> JsonParseReport {
    let parameters = content_type_parameters(request.content_type);
    let mut report = JsonParseReport {
        source_uri: request.source_uri.to_owned(),
        content_type: request.content_type.map(str::to_owned),
        parameters,
        facts: Vec::new(),
        root: None,
        line_ending: None,
    };

    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            let offset = error.valid_up_to() as u64;
            report.facts.push(JsonParseFact {
                kind: JsonParseFactKind::UnsupportedEncoding,
                member_name: None,
                line: None,
                column: None,
                byte_offset: Some(offset),
                byte_length: Some(error.error_len().unwrap_or(1) as u64),
                message: format!("JSON source must be valid UTF-8: {error}"),
            });
            return report;
        }
    };

    report.line_ending = json_detect_line_ending_style(source).map(str::to_owned);
    let mut parser = JsonParser::new(source);
    match parser.parse_document() {
        Ok(root) => {
            report.root = Some(root);
            report.facts.extend(parser.facts);
        }
        Err(fact) => report.facts.push(fact),
    }
    report
}

pub fn validate_json_parse_report(
    report: &JsonParseReport,
    contracts: &JsonSchemaContractCatalog,
) -> Vec<Diagnostic> {
    report
        .facts
        .iter()
        .map(|fact| json_diagnostic_from_fact(report, fact, contracts))
        .collect()
}

fn json_diagnostic_from_fact(
    report: &JsonParseReport,
    fact: &JsonParseFact,
    contracts: &JsonSchemaContractCatalog,
) -> Diagnostic {
    let binding = contracts.binding_for_fact(fact.kind);
    let severity = binding
        .map(|binding| binding.severity)
        .unwrap_or_else(|| match fact.kind {
            JsonParseFactKind::DuplicateMemberName | JsonParseFactKind::SourceMapUnavailable => {
                Severity::Warning
            }
            JsonParseFactKind::ParseError | JsonParseFactKind::UnsupportedEncoding => {
                Severity::Error
            }
        });
    let code = binding
        .map(|binding| binding.diagnostic_code.clone())
        .unwrap_or_else(|| match fact.kind {
            JsonParseFactKind::ParseError => "cem.json.parse_error".to_owned(),
            JsonParseFactKind::UnsupportedEncoding => "cem.json.unsupported_encoding".to_owned(),
            JsonParseFactKind::DuplicateMemberName => "cem.json.duplicate_member_name".to_owned(),
            JsonParseFactKind::SourceMapUnavailable => "cem.json.source_map_unavailable".to_owned(),
        });
    let source_map = fact.byte_offset.map(|offset| {
        let len = fact.byte_length.unwrap_or(0);
        JsonSourceRange {
            start: JsonSourcePosition {
                line: fact.line.unwrap_or(1),
                column: fact.column.unwrap_or(1),
                byte_offset: offset,
            },
            byte_length: len,
        }
        .source_map()
    });
    Diagnostic {
        uri: Some(report.source_uri.clone()),
        line: fact.line,
        column: fact.column,
        byte_offset: fact.byte_offset,
        code,
        severity,
        message: fact.message.clone(),
        node: None,
        details: Some(json!({
            "json": {
                "factKind": fact.kind.as_str(),
                "contract": binding.map(|binding| binding.contract.clone()),
                "behavior": binding.and_then(|binding| binding.behavior.clone()),
                "policy": binding.and_then(|binding| binding.policy.clone()),
                "memberName": fact.member_name,
                "sourceRange": {
                    "byteOffset": fact.byte_offset,
                    "byteLength": fact.byte_length,
                    "line": fact.line,
                    "column": fact.column,
                },
            },
        })),
        source_map,
    }
}

struct JsonParser<'a> {
    source: &'a str,
    line_index: LineIndex,
    byte: usize,
    facts: Vec<JsonParseFact>,
}

impl<'a> JsonParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            line_index: LineIndex::from_utf8(source),
            byte: 0,
            facts: Vec::new(),
        }
    }

    fn parse_document(&mut self) -> Result<JsonValueAst, JsonParseFact> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.byte != self.source.len() {
            return Err(self.error_here("unexpected trailing content after JSON root value"));
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValueAst, JsonParseFact> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string_value(),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(b't') => self.parse_literal(
                "true",
                JsonValueAst::Boolean {
                    range: self.range_from_current(4),
                    value: true,
                },
            ),
            Some(b'f') => self.parse_literal(
                "false",
                JsonValueAst::Boolean {
                    range: self.range_from_current(5),
                    value: false,
                },
            ),
            Some(b'n') => self.parse_literal(
                "null",
                JsonValueAst::Null {
                    range: self.range_from_current(4),
                },
            ),
            Some(_) => Err(self.error_here("expected JSON value")),
            None => Err(self.error_here("unexpected end of JSON source")),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValueAst, JsonParseFact> {
        let start = self.byte;
        self.expect_byte(b'{', "expected `{`")?;
        self.skip_whitespace();
        let mut members = Vec::new();
        let mut names = BTreeSet::new();
        if self.consume_if(b'}') {
            return Ok(JsonValueAst::Object {
                range: self.range(start, self.byte),
                members,
            });
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error_here("expected JSON object member name"));
            }
            let key = self.parse_string_token()?;
            self.skip_whitespace();
            self.expect_byte(b':', "expected `:` after JSON object member name")?;
            let value = self.parse_value()?;
            let member_range =
                self.range(key.range.start.byte_offset as usize, value.range().end());
            if !names.insert(key.value.clone()) {
                self.facts.push(JsonParseFact {
                    kind: JsonParseFactKind::DuplicateMemberName,
                    member_name: Some(key.value.clone()),
                    line: Some(key.range.start.line),
                    column: Some(key.range.start.column),
                    byte_offset: Some(key.range.start.byte_offset),
                    byte_length: Some(key.range.byte_length),
                    message: format!("duplicate JSON object member name `{}`", key.value),
                });
            }
            members.push(JsonMemberAst {
                index: members.len(),
                name: key.value,
                name_range: key.range,
                range: member_range,
                value,
            });
            self.skip_whitespace();
            if self.consume_if(b'}') {
                break;
            }
            self.expect_byte(b',', "expected `,` or `}` after JSON object member")?;
        }

        Ok(JsonValueAst::Object {
            range: self.range(start, self.byte),
            members,
        })
    }

    fn parse_array(&mut self) -> Result<JsonValueAst, JsonParseFact> {
        let start = self.byte;
        self.expect_byte(b'[', "expected `[`")?;
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.consume_if(b']') {
            return Ok(JsonValueAst::Array {
                range: self.range(start, self.byte),
                items,
            });
        }

        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_if(b']') {
                break;
            }
            self.expect_byte(b',', "expected `,` or `]` after JSON array item")?;
        }

        Ok(JsonValueAst::Array {
            range: self.range(start, self.byte),
            items,
        })
    }

    fn parse_string_value(&mut self) -> Result<JsonValueAst, JsonParseFact> {
        let token = self.parse_string_token()?;
        Ok(JsonValueAst::String {
            range: token.range,
            value: token.value,
            lexeme: token.lexeme,
        })
    }

    fn parse_string_token(&mut self) -> Result<JsonStringToken, JsonParseFact> {
        let start = self.byte;
        self.expect_byte(b'"', "expected JSON string")?;
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.byte += 1;
                    let lexeme = &self.source[start..self.byte];
                    let value = serde_json::from_str::<String>(lexeme).map_err(|error| {
                        self.error_at(start, format!("JSON string parse error: {error}"))
                    })?;
                    return Ok(JsonStringToken {
                        value,
                        lexeme: lexeme.to_owned(),
                        range: self.range(start, self.byte),
                    });
                }
                b'\\' => {
                    self.byte += 1;
                    self.consume_escape(start)?;
                }
                0x00..=0x1f => {
                    return Err(self.error_here("unescaped control character in JSON string"));
                }
                _ => self.advance_char(),
            }
        }
        Err(self.error_at(start, "unterminated JSON string"))
    }

    fn consume_escape(&mut self, string_start: usize) -> Result<(), JsonParseFact> {
        match self.peek() {
            Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                self.byte += 1;
                Ok(())
            }
            Some(b'u') => {
                self.byte += 1;
                for _ in 0..4 {
                    match self.peek() {
                        Some(byte) if byte.is_ascii_hexdigit() => self.byte += 1,
                        _ => {
                            return Err(
                                self.error_at(string_start, "invalid JSON unicode escape sequence")
                            );
                        }
                    }
                }
                Ok(())
            }
            Some(_) => Err(self.error_at(string_start, "invalid JSON string escape sequence")),
            None => Err(self.error_at(string_start, "unterminated JSON string escape sequence")),
        }
    }

    fn parse_number(&mut self) -> Result<JsonValueAst, JsonParseFact> {
        let start = self.byte;
        self.consume_if(b'-');
        match self.peek() {
            Some(b'0') => self.byte += 1,
            Some(b'1'..=b'9') => {
                self.byte += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.byte += 1;
                }
            }
            _ => return Err(self.error_at(start, "invalid JSON number")),
        }

        let mut number_kind = JsonNumberKind::Integer;
        if self.consume_if(b'.') {
            number_kind = JsonNumberKind::Decimal;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error_at(start, "invalid JSON number fraction"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.byte += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            number_kind = JsonNumberKind::Exponent;
            self.byte += 1;
            let _ = self.consume_if(b'+') || self.consume_if(b'-');
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error_at(start, "invalid JSON number exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.byte += 1;
            }
        }

        let lexeme = self.source[start..self.byte].to_owned();
        serde_json::from_str::<Value>(&lexeme)
            .map_err(|error| self.error_at(start, format!("JSON number parse error: {error}")))?;
        Ok(JsonValueAst::Number {
            range: self.range(start, self.byte),
            lexeme,
            number_kind,
        })
    }

    fn parse_literal(
        &mut self,
        literal: &str,
        value: JsonValueAst,
    ) -> Result<JsonValueAst, JsonParseFact> {
        if self.source[self.byte..].starts_with(literal) {
            self.byte += literal.len();
            Ok(value)
        } else {
            Err(self.error_here(format!("expected JSON literal `{literal}`")))
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.byte += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8, message: &str) -> Result<(), JsonParseFact> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.byte += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.byte).copied()
    }

    fn advance_char(&mut self) {
        let Some(scalar) = self.source[self.byte..].chars().next() else {
            return;
        };
        self.byte += scalar.len_utf8();
    }

    fn range_from_current(&self, len: usize) -> JsonSourceRange {
        self.range(self.byte, self.byte.saturating_add(len))
    }

    fn range(&self, start: usize, end: usize) -> JsonSourceRange {
        JsonSourceRange::from_offsets(&self.line_index, start, end)
    }

    fn error_here(&self, message: impl Into<String>) -> JsonParseFact {
        self.error_at(self.byte, message)
    }

    fn error_at(&self, byte_offset: usize, message: impl Into<String>) -> JsonParseFact {
        let coordinate = self.line_index.project(byte_offset as u64);
        JsonParseFact {
            kind: JsonParseFactKind::ParseError,
            member_name: None,
            line: Some(coordinate.line),
            column: Some(coordinate.column),
            byte_offset: Some(byte_offset as u64),
            byte_length: Some(1),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsonStringToken {
    value: String,
    lexeme: String,
    range: JsonSourceRange,
}

impl JsonSourceRange {
    fn end(self) -> usize {
        self.start.byte_offset.saturating_add(self.byte_length) as usize
    }
}

fn json_normalized_charset_for_report(report: &JsonParseReport) -> &'static str {
    if report
        .facts
        .iter()
        .any(|fact| fact.kind == JsonParseFactKind::UnsupportedEncoding)
    {
        "unsupported"
    } else {
        "utf-8"
    }
}

fn json_decoder_status_for_report(report: &JsonParseReport) -> &'static str {
    if report
        .facts
        .iter()
        .any(|fact| fact.kind == JsonParseFactKind::UnsupportedEncoding)
    {
        "error"
    } else {
        "ok"
    }
}

fn json_severity_name(severity: Severity) -> &'static str {
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
            Some((name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        })
        .collect()
}

fn content_type_parameter(content_type: &str, name: &str) -> Option<String> {
    content_type_parameters(Some(content_type)).remove(&name.to_ascii_lowercase())
}

fn json_content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn json_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("JSON string serialization should not fail")
}

fn json_detect_line_ending_style(source: &str) -> Option<&'static str> {
    let has_crlf = source.contains("\r\n");
    let has_lone_cr = source
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'\r' && pair[1] != b'\n')
        || source.ends_with('\r');
    let has_lf = source.as_bytes().contains(&b'\n');
    match (has_crlf, has_lone_cr, has_lf) {
        (false, false, false) => None,
        (true, false, true) => Some("crlf"),
        (false, false, true) => Some("lf"),
        _ => Some("mixed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_document_ast_preserves_value_source_ranges() {
        let (document, diagnostics) =
            json_document_ast_from_source_bytes(JsonSourceValidationRequest {
                bytes: br#"{"name":"Ada","items":[1,true,null]}"#,
                source_uri: "memory:json",
                content_type: Some(JSON_CONTENT_TYPE),
            });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("JSON document AST");
        let root = document.root.as_ref().expect("root value");
        assert_eq!(root.kind(), "object");
        let JsonValueAst::Object { members, .. } = root else {
            panic!("root should be object");
        };
        assert_eq!(members[0].name, "name");
        assert_eq!(members[0].name_range.start.byte_offset, 1);
        assert_eq!(members[0].value.range().start.byte_offset, 8);
        assert_eq!(document.to_json_value().unwrap()["items"][1], true);
        let source_map = members[0].name_range.source_map();
        let FrameSpan::Single(range) = source_map.frames[0].span else {
            panic!("JSON source map should be single-span");
        };
        assert_eq!(range.start, 1);
    }

    #[test]
    fn json_document_ast_reports_duplicate_members_as_schema_owned_warning() {
        let (document, diagnostics) =
            json_document_ast_from_source_bytes(JsonSourceValidationRequest {
                bytes: br#"{"name":"Ada","name":"Lin"}"#,
                source_uri: "memory:json",
                content_type: Some(JSON_CONTENT_TYPE),
            });

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "cem.json.duplicate_member_name");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(
            diagnostics[0]
                .details
                .as_ref()
                .and_then(|details| details.pointer("/json/factKind"))
                .and_then(Value::as_str),
            Some("duplicate-member-name")
        );
        assert_eq!(
            document.expect("JSON document AST").to_json_text("compact"),
            r#"{"name":"Ada","name":"Lin"}"#
        );
    }
}
