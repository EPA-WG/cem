use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::registry::{YAML_CONTENT_TYPE, YAML_SCHEMA_URI};
use crate::source::decode::Utf8Decoder;
use crate::source::{ByteRange, BytesSource, EncodingDecoder, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::generic_data::{
    GenericDataDocumentAst, GenericDataMappingEntryAst, GenericDataNumberKind,
    GenericDataSourceAst, GenericDataSourceRangeAst, GenericDataStreamDocumentAst,
    GenericDataValueAst,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser, Tag};
use yaml_rust2::scanner::{Marker, Scanner, TScalarStyle, TokenType};

const YAML_PACKAGE_ID: &str = "yaml";

#[derive(Debug, Clone, Copy)]
pub struct YamlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlDocumentAst {
    pub source: YamlDocumentSource,
    pub encoding: String,
    pub encoding_report: YamlEncodingReportAst,
    pub parse_facts: Vec<YamlDocumentParseFact>,
    pub directives: Vec<YamlDirectiveAst>,
    pub documents: Vec<YamlStreamDocumentAst>,
    pub line_ending: Option<String>,
}

impl YamlDocumentAst {
    pub fn to_cemt_subject(&self) -> Value {
        let mut stream = serde_json::Map::new();
        stream.insert("kind".to_owned(), json!("yaml-stream"));
        stream.insert("contentType".to_owned(), json!(YAML_CONTENT_TYPE));
        stream.insert("schema".to_owned(), json!(YAML_SCHEMA_URI));
        stream.insert("source".to_owned(), self.source.to_cemt_subject());
        stream.insert("encoding".to_owned(), json!(self.encoding));
        stream.insert(
            "encodingReport".to_owned(),
            self.encoding_report.to_cemt_subject(),
        );
        stream.insert(
            "parseFacts".to_owned(),
            Value::Array(
                self.parse_facts
                    .iter()
                    .map(YamlDocumentParseFact::to_cemt_subject)
                    .collect(),
            ),
        );
        stream.insert(
            "directives".to_owned(),
            Value::Array(
                self.directives
                    .iter()
                    .map(YamlDirectiveAst::to_cemt_subject)
                    .collect(),
            ),
        );
        stream.insert(
            "documents".to_owned(),
            Value::Array(
                self.documents
                    .iter()
                    .map(YamlStreamDocumentAst::to_cemt_subject)
                    .collect(),
            ),
        );
        if let Some(line_ending) = self.line_ending.as_deref() {
            stream.insert("lineEnding".to_owned(), json!(line_ending));
        }
        Value::Object(stream)
    }

    pub fn to_generic_data_ast(&self) -> GenericDataDocumentAst {
        GenericDataDocumentAst {
            source: GenericDataSourceAst {
                uri: self.source.uri.clone(),
                content_type: self.source.content_type.clone(),
                media_type: self.source.media_type.clone(),
                parameters: self.source.parameters.clone(),
                byte_length: self.source.byte_length,
            },
            documents: self
                .documents
                .iter()
                .map(|document| GenericDataStreamDocumentAst {
                    index: document.index,
                    source_range: yaml_source_range_to_generic_data_range(document.range),
                    root: document
                        .root
                        .as_ref()
                        .map(yaml_node_ast_to_generic_data_value),
                })
                .collect(),
            line_ending: self.line_ending.clone(),
        }
    }
}

pub fn generic_data_ast_to_yaml_cemt_subject(ast: &GenericDataDocumentAst) -> Value {
    let mut stream = serde_json::Map::new();
    stream.insert("kind".to_owned(), json!("yaml-stream"));
    stream.insert("contentType".to_owned(), json!(YAML_CONTENT_TYPE));
    stream.insert("schema".to_owned(), json!(YAML_SCHEMA_URI));
    stream.insert("source".to_owned(), ast.source.to_cemt_subject());
    stream.insert("encoding".to_owned(), json!("utf-8"));
    stream.insert(
        "encodingReport".to_owned(),
        json!({
            "normalizedCharset": "utf-8",
            "decoderStatus": "decoded",
        }),
    );
    stream.insert("parseFacts".to_owned(), Value::Array(Vec::new()));
    stream.insert("directives".to_owned(), Value::Array(Vec::new()));
    stream.insert(
        "documents".to_owned(),
        Value::Array(
            ast.documents
                .iter()
                .map(generic_data_document_to_yaml_cemt_subject)
                .collect(),
        ),
    );
    if let Some(line_ending) = ast.line_ending.as_deref() {
        stream.insert("lineEnding".to_owned(), json!(line_ending));
    }
    Value::Object(stream)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl YamlDocumentSource {
    fn from_request(
        request: YamlSourceValidationRequest<'_>,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        Self {
            uri: request.source_uri.to_owned(),
            content_type: request.content_type.unwrap_or(YAML_CONTENT_TYPE).to_owned(),
            media_type: request
                .content_type
                .map(yaml_content_type_essence)
                .unwrap_or(YAML_CONTENT_TYPE.to_owned()),
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
pub struct YamlEncodingReportAst {
    pub declared_charset: Option<String>,
    pub normalized_charset: String,
    pub decoder_status: String,
    pub invalid_byte_offset: Option<u64>,
}

impl YamlEncodingReportAst {
    fn from_report(report: &YamlParseReport) -> Self {
        Self {
            declared_charset: report
                .content_type
                .as_deref()
                .and_then(|content_type| content_type_parameter(content_type, "charset")),
            normalized_charset: yaml_normalized_charset_for_report(report).to_owned(),
            decoder_status: yaml_decoder_status_for_report(report).to_owned(),
            invalid_byte_offset: report.facts.iter().find_map(|fact| match fact.kind {
                YamlParseFactKind::UnsupportedEncoding => fact.byte_offset,
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
pub struct YamlDocumentParseFact {
    pub kind: YamlParseFactKind,
    pub contract: Option<String>,
    pub behavior: Option<String>,
    pub diagnostic_code: Option<String>,
    pub diagnostic_severity: Option<String>,
    pub recoverable: bool,
    pub fatal: bool,
    pub parameter: Option<String>,
    pub actual: Option<String>,
    pub expected: Vec<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub byte_length: Option<u64>,
    pub message: String,
}

impl YamlDocumentParseFact {
    fn from_parse_fact(fact: &YamlParseFact, contracts: &YamlSchemaContractCatalog) -> Self {
        let binding = contracts.binding_for_fact(fact.kind);
        let fatal = binding.is_some_and(|binding| binding.severity.is_hard_violation());
        Self {
            kind: fact.kind,
            contract: binding.map(|binding| binding.contract.clone()),
            behavior: binding.and_then(|binding| binding.behavior.clone()),
            diagnostic_code: binding.map(|binding| binding.diagnostic_code.clone()),
            diagnostic_severity: binding
                .map(|binding| yaml_severity_name(binding.severity).to_owned()),
            recoverable: !fatal,
            fatal,
            parameter: fact.parameter.clone(),
            actual: fact.actual.clone(),
            expected: fact.expected.clone(),
            line: fact.line,
            column: fact.column,
            byte_offset: fact.byte_offset,
            byte_length: fact.byte_length,
            message: yaml_fact_message(fact),
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
            "parameter": self.parameter,
            "actual": self.actual,
            "expected": self.expected,
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
pub struct YamlDirectiveAst {
    pub index: usize,
    pub name: String,
    pub value: Option<String>,
    pub range: YamlSourceRange,
}

impl YamlDirectiveAst {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "name": self.name,
            "value": self.value,
            "byteOffset": self.range.start.byte_offset,
            "sourceRange": self.range.to_cemt_subject(),
            "sourceMap": self.range.source_map(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlStreamDocumentAst {
    pub index: usize,
    pub range: YamlSourceRange,
    pub root: Option<YamlNodeAst>,
}

impl YamlStreamDocumentAst {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "byteOffset": self.range.start.byte_offset,
            "sourceRange": self.range.to_cemt_subject(),
            "sourceMap": self.range.source_map(),
            "root": self.root.as_ref().map(YamlNodeAst::to_cemt_subject),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlNodeAst {
    pub kind: YamlNodeKind,
    pub range: YamlSourceRange,
    pub tag: Option<String>,
    pub anchor: Option<String>,
    pub anchor_id: Option<usize>,
    pub alias: Option<String>,
    pub value: Option<String>,
    pub style: Option<String>,
    pub implicit_kind: Option<String>,
    pub sequence: Vec<YamlNodeAst>,
    pub mapping: Vec<YamlPairAst>,
}

impl YamlNodeAst {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "tag": self.tag,
            "anchor": self.anchor,
            "anchorId": self.anchor_id,
            "alias": self.alias,
            "value": self.value,
            "style": self.style,
            "implicitKind": self.implicit_kind,
            "byteOffset": self.range.start.byte_offset,
            "sourceRange": self.range.to_cemt_subject(),
            "sourceMap": self.range.source_map(),
            "sequence": self.sequence.iter().map(YamlNodeAst::to_cemt_subject).collect::<Vec<_>>(),
            "mapping": self.mapping.iter().map(YamlPairAst::to_cemt_subject).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlPairAst {
    pub index: usize,
    pub key: YamlNodeAst,
    pub value: YamlNodeAst,
}

impl YamlPairAst {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "key": self.key.to_cemt_subject(),
            "value": self.value.to_cemt_subject(),
        })
    }
}

fn yaml_node_ast_to_generic_data_value(node: &YamlNodeAst) -> GenericDataValueAst {
    let source_range = yaml_source_range_to_generic_data_range(node.range);
    match node.kind {
        YamlNodeKind::Mapping => GenericDataValueAst::Mapping {
            source_range,
            entries: node
                .mapping
                .iter()
                .map(|pair| GenericDataMappingEntryAst {
                    index: pair.index,
                    key: yaml_node_ast_to_generic_data_value(&pair.key),
                    value: yaml_node_ast_to_generic_data_value(&pair.value),
                    source_range: yaml_source_range_to_generic_data_range(pair.key.range),
                })
                .collect(),
        },
        YamlNodeKind::Sequence => GenericDataValueAst::Sequence {
            source_range,
            items: node
                .sequence
                .iter()
                .map(yaml_node_ast_to_generic_data_value)
                .collect(),
        },
        YamlNodeKind::Scalar => yaml_scalar_node_to_generic_data_value(node, source_range),
        YamlNodeKind::Alias => GenericDataValueAst::Alias {
            source_range,
            alias: node.alias.clone(),
        },
    }
}

fn yaml_scalar_node_to_generic_data_value(
    node: &YamlNodeAst,
    source_range: GenericDataSourceRangeAst,
) -> GenericDataValueAst {
    let value = node.value.clone().unwrap_or_default();
    match node.implicit_kind.as_deref().unwrap_or("string") {
        "null" => GenericDataValueAst::Null { source_range },
        "boolean" => GenericDataValueAst::Boolean {
            source_range,
            value: matches!(value.trim(), "true" | "True" | "TRUE"),
        },
        "integer" => GenericDataValueAst::Number {
            source_range,
            lexeme: value,
            number_kind: GenericDataNumberKind::Integer,
        },
        "float" => GenericDataValueAst::Number {
            source_range,
            lexeme: value,
            number_kind: GenericDataNumberKind::Decimal,
        },
        _ => GenericDataValueAst::String {
            source_range,
            value,
            lexeme: None,
            style: node.style.clone(),
        },
    }
}

fn yaml_source_range_to_generic_data_range(range: YamlSourceRange) -> GenericDataSourceRangeAst {
    GenericDataSourceRangeAst {
        byte_offset: range.start.byte_offset,
        byte_length: range.byte_length,
        line: range.start.line,
        column: range.start.column,
        source_map: Some(range.source_map()),
    }
}

fn generic_data_document_to_yaml_cemt_subject(document: &GenericDataStreamDocumentAst) -> Value {
    json!({
        "index": document.index,
        "byteOffset": document.source_range.byte_offset,
        "sourceRange": document.source_range.to_cemt_subject(),
        "sourceMap": document.source_range.source_map_subject(),
        "root": document.root.as_ref().map(generic_data_value_to_yaml_cemt_node),
    })
}

fn generic_data_value_to_yaml_cemt_node(value: &GenericDataValueAst) -> Value {
    match value {
        GenericDataValueAst::Mapping {
            source_range,
            entries,
        } => json!({
            "kind": "mapping",
            "tag": Value::Null,
            "anchor": Value::Null,
            "anchorId": Value::Null,
            "alias": Value::Null,
            "value": Value::Null,
            "style": Value::Null,
            "implicitKind": Value::Null,
            "byteOffset": source_range.byte_offset,
            "sourceRange": source_range.to_cemt_subject(),
            "sourceMap": source_range.source_map_subject(),
            "sequence": [],
            "mapping": entries
                .iter()
                .map(generic_data_mapping_entry_to_yaml_pair)
                .collect::<Vec<_>>(),
        }),
        GenericDataValueAst::Sequence {
            source_range,
            items,
        } => json!({
            "kind": "sequence",
            "tag": Value::Null,
            "anchor": Value::Null,
            "anchorId": Value::Null,
            "alias": Value::Null,
            "value": Value::Null,
            "style": Value::Null,
            "implicitKind": Value::Null,
            "byteOffset": source_range.byte_offset,
            "sourceRange": source_range.to_cemt_subject(),
            "sourceMap": source_range.source_map_subject(),
            "sequence": items
                .iter()
                .map(generic_data_value_to_yaml_cemt_node)
                .collect::<Vec<_>>(),
            "mapping": [],
        }),
        GenericDataValueAst::String {
            source_range,
            value,
            style,
            ..
        } => generic_data_scalar_to_yaml_cemt_node(
            source_range,
            value,
            "string",
            style.as_deref().unwrap_or("plain"),
        ),
        GenericDataValueAst::Number {
            source_range,
            lexeme,
            number_kind,
        } => generic_data_scalar_to_yaml_cemt_node(
            source_range,
            lexeme,
            number_kind.as_yaml_implicit_kind(),
            "plain",
        ),
        GenericDataValueAst::Boolean {
            source_range,
            value,
        } => generic_data_scalar_to_yaml_cemt_node(
            source_range,
            if *value { "true" } else { "false" },
            "boolean",
            "plain",
        ),
        GenericDataValueAst::Null { source_range } => {
            generic_data_scalar_to_yaml_cemt_node(source_range, "", "null", "plain")
        }
        GenericDataValueAst::Alias {
            source_range,
            alias,
        } => json!({
            "kind": "alias",
            "tag": Value::Null,
            "anchor": Value::Null,
            "anchorId": Value::Null,
            "alias": alias,
            "value": Value::Null,
            "style": Value::Null,
            "implicitKind": Value::Null,
            "byteOffset": source_range.byte_offset,
            "sourceRange": source_range.to_cemt_subject(),
            "sourceMap": source_range.source_map_subject(),
            "sequence": [],
            "mapping": [],
        }),
    }
}

fn generic_data_mapping_entry_to_yaml_pair(entry: &GenericDataMappingEntryAst) -> Value {
    json!({
        "index": entry.index,
        "key": generic_data_value_to_yaml_cemt_node(&entry.key),
        "value": generic_data_value_to_yaml_cemt_node(&entry.value),
    })
}

fn generic_data_scalar_to_yaml_cemt_node(
    source_range: &GenericDataSourceRangeAst,
    value: &str,
    implicit_kind: &str,
    style: &str,
) -> Value {
    json!({
        "kind": "scalar",
        "tag": Value::Null,
        "anchor": Value::Null,
        "anchorId": Value::Null,
        "alias": Value::Null,
        "value": value,
        "style": style,
        "implicitKind": implicit_kind,
        "byteOffset": source_range.byte_offset,
        "sourceRange": source_range.to_cemt_subject(),
        "sourceMap": source_range.source_map_subject(),
        "sequence": [],
        "mapping": [],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlNodeKind {
    Mapping,
    Sequence,
    Scalar,
    Alias,
}

impl YamlNodeKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Mapping => "mapping",
            Self::Sequence => "sequence",
            Self::Scalar => "scalar",
            Self::Alias => "alias",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlSourceRange {
    pub start: YamlSourcePosition,
    pub byte_length: u64,
}

impl YamlSourceRange {
    fn generated() -> Self {
        Self {
            start: YamlSourcePosition {
                line: 1,
                column: 1,
                byte_offset: 0,
            },
            byte_length: 0,
        }
    }

    fn from_marker(marker: Marker, byte_length: u64) -> Self {
        Self {
            start: YamlSourcePosition {
                line: u32::try_from(marker.line()).unwrap_or(u32::MAX),
                column: u32::try_from(marker.col().saturating_add(1)).unwrap_or(u32::MAX),
                byte_offset: marker.index() as u64,
            },
            byte_length,
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
                    content_type: YAML_CONTENT_TYPE.to_owned(),
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
pub struct YamlParseReport {
    pub source_uri: String,
    pub content_type: Option<String>,
    pub byte_len: u64,
    pub facts: Vec<YamlParseFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlParseFact {
    pub kind: YamlParseFactKind,
    pub parameter: Option<String>,
    pub actual: Option<String>,
    pub expected: Vec<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub byte_length: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct YamlDecodedSource {
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum YamlParseFactKind {
    ParseError,
    UnsupportedEncoding,
    UnresolvedAlias,
    DuplicateAnchor,
    UnsafeTag,
    SourceMapUnavailable,
}

impl YamlParseFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseError => "parse-error",
            Self::UnsupportedEncoding => "unsupported-encoding",
            Self::UnresolvedAlias => "unresolved-alias",
            Self::DuplicateAnchor => "duplicate-anchor",
            Self::UnsafeTag => "unsafe-tag",
            Self::SourceMapUnavailable => "source-map-unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlSchemaContractCatalog {
    pub fact_bindings: BTreeMap<String, YamlDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

impl YamlSchemaContractCatalog {
    pub fn from_builtin() -> Self {
        let source = crate::schema::package_sources::builtin_schema_package_source(YAML_PACKAGE_ID)
            .expect("built-in YAML schema package source must be registered");
        Self::from_schema_source(source.schema_source)
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(YAML_SCHEMA_URI, schema_source);
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
                    YamlDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: YamlParseFactKind) -> Option<&YamlDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

#[derive(Debug, Clone)]
struct YamlMarkedEvent {
    event: Event,
    range: YamlSourceRange,
}

#[derive(Debug, Clone, Default)]
struct YamlParseCapture {
    events: Vec<YamlMarkedEvent>,
    facts: Vec<YamlParseFact>,
}

impl MarkedEventReceiver for YamlParseCapture {
    fn on_event(&mut self, event: Event, marker: Marker) {
        if let Some(tag) = yaml_event_tag(&event) {
            if !is_safe_yaml_tag(tag) {
                self.facts.push(yaml_marker_fact(
                    YamlParseFactKind::UnsafeTag,
                    marker,
                    Some("tag"),
                    Some(&yaml_tag_display(tag)),
                    &["YAML core safe tag"],
                    format!(
                        "YAML node uses unsupported explicit tag `{}`",
                        yaml_tag_display(tag)
                    ),
                ));
            }
        }

        self.events.push(YamlMarkedEvent {
            event,
            range: YamlSourceRange::from_marker(marker, 0),
        });
    }
}

#[derive(Debug, Clone, Default)]
struct YamlLexicalFacts {
    anchor_names_by_id: BTreeMap<usize, String>,
    duplicate_anchor_facts: Vec<YamlParseFact>,
    directives: Vec<YamlDirectiveAst>,
}

pub fn validate_yaml_source_bytes(request: YamlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let contracts = YamlSchemaContractCatalog::from_builtin();
    let report = extract_yaml_parse_report(request);
    validate_yaml_parse_report(&report, &contracts)
}

pub fn yaml_stream_value_from_source_bytes(
    request: YamlSourceValidationRequest<'_>,
) -> (Option<Value>, Vec<Diagnostic>) {
    let (stream, diagnostics) = yaml_document_ast_from_source_bytes(request);
    (stream.map(|stream| stream.to_cemt_subject()), diagnostics)
}

pub fn yaml_document_ast_from_source_bytes(
    request: YamlSourceValidationRequest<'_>,
) -> (Option<YamlDocumentAst>, Vec<Diagnostic>) {
    let report = extract_yaml_parse_report(request);
    let contracts = YamlSchemaContractCatalog::from_builtin();
    let diagnostics = validate_yaml_parse_report(&report, &contracts);
    let decoded = yaml_decode_source_text(request).ok();
    let lexical = decoded
        .as_ref()
        .map(|source| collect_yaml_lexical_facts(&source.text))
        .unwrap_or_default();
    let capture = decoded
        .as_ref()
        .filter(|_| !yaml_parse_report_has_encoding_blocker(&report))
        .and_then(|source| capture_yaml_events(&source.text).ok())
        .unwrap_or_default();
    let documents = yaml_project_documents(&capture.events, &lexical.anchor_names_by_id);
    let line_ending = decoded
        .as_ref()
        .and_then(|source| yaml_detect_line_ending_style(&source.text));
    let stream = YamlDocumentAst {
        source: YamlDocumentSource::from_request(
            request,
            content_type_parameters(request.content_type),
        ),
        encoding: yaml_stream_encoding(&report).to_owned(),
        encoding_report: YamlEncodingReportAst::from_report(&report),
        parse_facts: yaml_parse_facts_for_document(&report, &contracts),
        directives: lexical.directives,
        documents,
        line_ending: line_ending.map(str::to_owned),
    };

    (Some(stream), diagnostics)
}

pub fn extract_yaml_parse_report(request: YamlSourceValidationRequest<'_>) -> YamlParseReport {
    let mut report = YamlParseReport {
        source_uri: request.source_uri.to_owned(),
        content_type: request.content_type.map(str::to_owned),
        byte_len: request.bytes.len() as u64,
        facts: Vec::new(),
    };

    let source = match yaml_decode_source_text(request) {
        Ok(source) => source,
        Err(fact) => {
            report.facts.push(fact);
            return report;
        }
    };

    let lexical = collect_yaml_lexical_facts(&source.text);
    report.facts.extend(lexical.duplicate_anchor_facts);

    match capture_yaml_events(&source.text) {
        Ok(capture) => {
            report.facts.extend(capture.facts);
        }
        Err(error) => {
            report.facts.push(yaml_scan_error_fact(&error));
        }
    }

    report
}

fn yaml_decode_source_text(
    request: YamlSourceValidationRequest<'_>,
) -> Result<YamlDecodedSource, YamlParseFact> {
    let mut decoder = Utf8Decoder::new(BytesSource::new(SourceId(1), request.bytes.to_vec()));
    let mut text = String::new();

    while let Some(chunk) = decoder.decode_next() {
        for (scalar, _) in chunk.scalars {
            text.push(scalar);
        }
    }

    let diagnostics = decoder.take_diagnostics();
    if let Some(diagnostic) = diagnostics.iter().find(|diagnostic| {
        matches!(
            diagnostic.code.as_str(),
            "cem.byte.invalid_utf8" | "cem.byte.unsupported_encoding"
        )
    }) {
        let mut fact = yaml_fact(
            YamlParseFactKind::UnsupportedEncoding,
            Some("charset"),
            request
                .content_type
                .and_then(|content_type| content_type_parameter(content_type, "charset"))
                .as_deref()
                .or(Some("utf-8")),
            &["valid UTF-8"],
        );
        fact.byte_offset = diagnostic.byte_offset;
        fact.message = Some(diagnostic.message.clone());
        return Err(fact);
    }

    Ok(YamlDecodedSource { text })
}

pub fn validate_yaml_parse_report(
    report: &YamlParseReport,
    contracts: &YamlSchemaContractCatalog,
) -> Vec<Diagnostic> {
    report
        .facts
        .iter()
        .filter_map(|fact| {
            let binding = contracts.binding_for_fact(fact.kind)?;
            Some(yaml_fact_diagnostic(
                report,
                fact,
                binding,
                yaml_fact_message(fact),
            ))
        })
        .collect()
}

fn capture_yaml_events(source: &str) -> Result<YamlParseCapture, yaml_rust2::scanner::ScanError> {
    let mut capture = YamlParseCapture::default();
    Parser::new_from_str(source).load(&mut capture, true)?;
    Ok(capture)
}

fn collect_yaml_lexical_facts(source: &str) -> YamlLexicalFacts {
    let mut facts = YamlLexicalFacts::default();
    let mut seen_anchor_names = BTreeSet::new();
    let mut next_anchor_id = 1usize;
    let mut directive_index = 0usize;
    let mut scanner = Scanner::new(source.chars());

    for token in scanner.by_ref() {
        match token.1 {
            TokenType::Anchor(name) => {
                if !seen_anchor_names.insert(name.clone()) {
                    facts.duplicate_anchor_facts.push(yaml_marker_fact(
                        YamlParseFactKind::DuplicateAnchor,
                        token.0,
                        Some("anchor"),
                        Some(&name),
                        &["unique anchor name"],
                        format!("YAML anchor `{name}` duplicates an earlier anchor"),
                    ));
                }
                facts.anchor_names_by_id.insert(next_anchor_id, name);
                next_anchor_id += 1;
            }
            TokenType::VersionDirective(major, minor) => {
                facts.directives.push(YamlDirectiveAst {
                    index: directive_index,
                    name: "YAML".to_owned(),
                    value: Some(format!("{major}.{minor}")),
                    range: YamlSourceRange::from_marker(token.0, 0),
                });
                directive_index += 1;
            }
            TokenType::TagDirective(handle, prefix) => {
                facts.directives.push(YamlDirectiveAst {
                    index: directive_index,
                    name: "TAG".to_owned(),
                    value: Some(format!("{handle} {prefix}")),
                    range: YamlSourceRange::from_marker(token.0, 0),
                });
                directive_index += 1;
            }
            _ => {}
        }
    }

    facts
}

fn yaml_project_documents(
    events: &[YamlMarkedEvent],
    anchor_names_by_id: &BTreeMap<usize, String>,
) -> Vec<YamlStreamDocumentAst> {
    let mut documents = Vec::new();
    let mut cursor = 0usize;

    while cursor < events.len() {
        match events[cursor].event {
            Event::StreamStart | Event::StreamEnd => {
                cursor += 1;
            }
            Event::DocumentStart => {
                let document_index = documents.len();
                let range = events[cursor].range;
                cursor += 1;
                let root = if cursor < events.len()
                    && !matches!(
                        events[cursor].event,
                        Event::DocumentEnd | Event::StreamEnd | Event::DocumentStart
                    ) {
                    Some(yaml_project_node(events, &mut cursor, anchor_names_by_id))
                } else {
                    None
                };
                if cursor < events.len() && matches!(events[cursor].event, Event::DocumentEnd) {
                    cursor += 1;
                }
                documents.push(YamlStreamDocumentAst {
                    index: document_index,
                    range,
                    root,
                });
            }
            _ => {
                cursor += 1;
            }
        }
    }

    documents
}

fn yaml_project_node(
    events: &[YamlMarkedEvent],
    cursor: &mut usize,
    anchor_names_by_id: &BTreeMap<usize, String>,
) -> YamlNodeAst {
    let Some(marked) = events.get(*cursor).cloned() else {
        return yaml_empty_scalar_node(YamlSourceRange::generated());
    };
    *cursor += 1;

    match marked.event {
        Event::Scalar(value, style, anchor_id, tag) => YamlNodeAst {
            kind: YamlNodeKind::Scalar,
            range: marked.range,
            tag: tag.as_ref().map(yaml_tag_display),
            anchor: yaml_anchor_name(anchor_names_by_id, anchor_id),
            anchor_id: yaml_anchor_id(anchor_id),
            alias: None,
            implicit_kind: Some(yaml_scalar_implicit_kind(&value, style).to_owned()),
            value: Some(value),
            style: Some(yaml_scalar_style_name(style).to_owned()),
            sequence: Vec::new(),
            mapping: Vec::new(),
        },
        Event::Alias(anchor_id) => YamlNodeAst {
            kind: YamlNodeKind::Alias,
            range: marked.range,
            tag: None,
            anchor: None,
            anchor_id: yaml_anchor_id(anchor_id),
            alias: yaml_anchor_name(anchor_names_by_id, anchor_id)
                .or_else(|| yaml_anchor_id(anchor_id).map(|id| id.to_string())),
            value: None,
            style: None,
            implicit_kind: None,
            sequence: Vec::new(),
            mapping: Vec::new(),
        },
        Event::SequenceStart(anchor_id, tag) => {
            let mut sequence = Vec::new();
            while *cursor < events.len() {
                if matches!(events[*cursor].event, Event::SequenceEnd) {
                    *cursor += 1;
                    break;
                }
                sequence.push(yaml_project_node(events, cursor, anchor_names_by_id));
            }
            YamlNodeAst {
                kind: YamlNodeKind::Sequence,
                range: marked.range,
                tag: tag.as_ref().map(yaml_tag_display),
                anchor: yaml_anchor_name(anchor_names_by_id, anchor_id),
                anchor_id: yaml_anchor_id(anchor_id),
                alias: None,
                value: None,
                style: None,
                implicit_kind: None,
                sequence,
                mapping: Vec::new(),
            }
        }
        Event::MappingStart(anchor_id, tag) => {
            let mut mapping = Vec::new();
            while *cursor < events.len() {
                if matches!(events[*cursor].event, Event::MappingEnd) {
                    *cursor += 1;
                    break;
                }
                let key = yaml_project_node(events, cursor, anchor_names_by_id);
                let value = yaml_project_node(events, cursor, anchor_names_by_id);
                mapping.push(YamlPairAst {
                    index: mapping.len(),
                    key,
                    value,
                });
            }
            YamlNodeAst {
                kind: YamlNodeKind::Mapping,
                range: marked.range,
                tag: tag.as_ref().map(yaml_tag_display),
                anchor: yaml_anchor_name(anchor_names_by_id, anchor_id),
                anchor_id: yaml_anchor_id(anchor_id),
                alias: None,
                value: None,
                style: None,
                implicit_kind: None,
                sequence: Vec::new(),
                mapping,
            }
        }
        _ => yaml_empty_scalar_node(marked.range),
    }
}

fn yaml_empty_scalar_node(range: YamlSourceRange) -> YamlNodeAst {
    YamlNodeAst {
        kind: YamlNodeKind::Scalar,
        range,
        tag: None,
        anchor: None,
        anchor_id: None,
        alias: None,
        value: Some(String::new()),
        style: Some("plain".to_owned()),
        implicit_kind: Some("null".to_owned()),
        sequence: Vec::new(),
        mapping: Vec::new(),
    }
}

fn yaml_anchor_id(anchor_id: usize) -> Option<usize> {
    (anchor_id > 0).then_some(anchor_id)
}

fn yaml_anchor_name(
    anchor_names_by_id: &BTreeMap<usize, String>,
    anchor_id: usize,
) -> Option<String> {
    yaml_anchor_id(anchor_id).and_then(|id| anchor_names_by_id.get(&id).cloned())
}

fn yaml_event_tag(event: &Event) -> Option<&Tag> {
    match event {
        Event::Scalar(_, _, _, tag)
        | Event::SequenceStart(_, tag)
        | Event::MappingStart(_, tag) => tag.as_ref(),
        _ => None,
    }
}

fn is_safe_yaml_tag(tag: &Tag) -> bool {
    let handle = tag.handle.trim();
    let suffix = tag.suffix.trim();
    if handle.is_empty() && suffix.is_empty() {
        return true;
    }

    match handle {
        "!" => suffix.is_empty(),
        "!!" | "tag:yaml.org,2002:" => is_safe_yaml_core_tag_name(suffix),
        _ => false,
    }
}

fn is_safe_yaml_core_tag_name(name: &str) -> bool {
    matches!(
        name,
        "binary"
            | "bool"
            | "float"
            | "int"
            | "map"
            | "merge"
            | "null"
            | "omap"
            | "pairs"
            | "seq"
            | "set"
            | "str"
            | "timestamp"
            | "value"
            | "yaml"
    )
}

fn yaml_tag_display(tag: &Tag) -> String {
    format!("{}{}", tag.handle, tag.suffix)
}

fn yaml_scalar_style_name(style: TScalarStyle) -> &'static str {
    match style {
        TScalarStyle::Plain => "plain",
        TScalarStyle::SingleQuoted => "single-quoted",
        TScalarStyle::DoubleQuoted => "double-quoted",
        TScalarStyle::Literal => "literal",
        TScalarStyle::Folded => "folded",
    }
}

fn yaml_scalar_implicit_kind(value: &str, style: TScalarStyle) -> &'static str {
    if style != TScalarStyle::Plain {
        return "string";
    }
    let normalized = value.trim();
    if normalized.is_empty() || matches!(normalized, "~" | "null" | "Null" | "NULL") {
        "null"
    } else if matches!(
        normalized,
        "true" | "True" | "TRUE" | "false" | "False" | "FALSE"
    ) {
        "boolean"
    } else if normalized.parse::<i64>().is_ok() {
        "integer"
    } else if normalized.parse::<f64>().is_ok() {
        "float"
    } else {
        "string"
    }
}

fn yaml_parse_facts_for_document(
    report: &YamlParseReport,
    contracts: &YamlSchemaContractCatalog,
) -> Vec<YamlDocumentParseFact> {
    report
        .facts
        .iter()
        .map(|fact| YamlDocumentParseFact::from_parse_fact(fact, contracts))
        .collect()
}

fn yaml_parse_report_has_encoding_blocker(report: &YamlParseReport) -> bool {
    report
        .facts
        .iter()
        .any(|fact| fact.kind == YamlParseFactKind::UnsupportedEncoding)
}

fn yaml_stream_encoding(report: &YamlParseReport) -> &'static str {
    yaml_normalized_charset_for_report(report)
}

fn yaml_normalized_charset_for_report(report: &YamlParseReport) -> &'static str {
    let charset = report
        .content_type
        .as_deref()
        .and_then(|content_type| content_type_parameter(content_type, "charset"));
    match charset.as_deref().map(yaml_normalized_parameter).as_deref() {
        Some("utf-8" | "utf8") | None => "utf-8",
        Some(_) => "other",
    }
}

fn yaml_decoder_status_for_report(report: &YamlParseReport) -> &'static str {
    if report
        .facts
        .iter()
        .any(|fact| fact.kind == YamlParseFactKind::UnsupportedEncoding)
    {
        "invalid"
    } else {
        "decoded"
    }
}

fn yaml_fact(
    kind: YamlParseFactKind,
    parameter: Option<&str>,
    actual: Option<&str>,
    expected: &[&str],
) -> YamlParseFact {
    YamlParseFact {
        kind,
        parameter: parameter.map(str::to_owned),
        actual: actual.map(str::to_owned),
        expected: expected.iter().map(|value| (*value).to_owned()).collect(),
        line: None,
        column: None,
        byte_offset: None,
        byte_length: None,
        message: None,
    }
}

fn yaml_marker_fact(
    kind: YamlParseFactKind,
    marker: Marker,
    parameter: Option<&str>,
    actual: Option<&str>,
    expected: &[&str],
    message: String,
) -> YamlParseFact {
    let mut fact = yaml_fact(kind, parameter, actual, expected);
    fact.line = u32::try_from(marker.line()).ok();
    fact.column = u32::try_from(marker.col().saturating_add(1)).ok();
    fact.byte_offset = Some(marker.index() as u64);
    fact.byte_length = Some(0);
    fact.message = Some(message);
    fact
}

fn yaml_scan_error_fact(error: &yaml_rust2::scanner::ScanError) -> YamlParseFact {
    let marker = error.marker();
    let message = format!("YAML parse error: {error}");
    yaml_marker_fact(
        yaml_parse_error_fact_kind(error),
        *marker,
        None,
        None,
        &[],
        message,
    )
}

fn yaml_parse_error_fact_kind(error: &yaml_rust2::scanner::ScanError) -> YamlParseFactKind {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("unknown anchor") {
        YamlParseFactKind::UnresolvedAlias
    } else {
        YamlParseFactKind::ParseError
    }
}

fn yaml_fact_diagnostic(
    report: &YamlParseReport,
    fact: &YamlParseFact,
    binding: &YamlDiagnosticBinding,
    message: String,
) -> Diagnostic {
    Diagnostic {
        uri: Some(report.source_uri.clone()),
        line: fact.line,
        column: fact.column,
        byte_offset: fact.byte_offset,
        code: binding.diagnostic_code.clone(),
        severity: binding.severity,
        message,
        details: Some(json!({
            "contract": binding.contract,
            "behavior": binding.behavior,
            "factKind": fact.kind.as_str(),
            "mediaType": {
                "contentType": report.content_type.as_deref(),
                "parameter": fact.parameter.as_deref(),
            },
            "sourceRange": {
                "byteOffset": fact.byte_offset,
                "byteLength": fact.byte_length,
                "line": fact.line,
                "column": fact.column,
            },
            "expected": fact.expected,
            "actual": fact.actual.as_deref(),
            "byteLength": report.byte_len,
        })),
        ..Diagnostic::default()
    }
}

fn yaml_fact_message(fact: &YamlParseFact) -> String {
    if let Some(message) = fact.message.clone() {
        return message;
    }

    match fact.kind {
        YamlParseFactKind::ParseError => "YAML parse error".to_owned(),
        YamlParseFactKind::UnsupportedEncoding => "YAML source must be valid UTF-8".to_owned(),
        YamlParseFactKind::UnresolvedAlias => "YAML alias references an unknown anchor".to_owned(),
        YamlParseFactKind::DuplicateAnchor => format!(
            "YAML anchor `{}` duplicates an earlier anchor",
            fact.actual.as_deref().unwrap_or("")
        ),
        YamlParseFactKind::UnsafeTag => format!(
            "YAML node uses unsupported explicit tag `{}`",
            fact.actual.as_deref().unwrap_or("")
        ),
        YamlParseFactKind::SourceMapUnavailable => {
            "YAML parser did not expose a source map for this node".to_owned()
        }
    }
}

fn yaml_severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Fatal => "fatal",
    }
}

fn yaml_content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn content_type_parameters(content_type: Option<&str>) -> BTreeMap<String, String> {
    content_type
        .into_iter()
        .flat_map(|content_type| content_type.split(';').skip(1))
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            Some((
                key.trim().to_ascii_lowercase(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect()
}

fn content_type_parameter(content_type: &str, name: &str) -> Option<String> {
    let needle = name.trim().to_ascii_lowercase();
    content_type.split(';').skip(1).find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key.trim().eq_ignore_ascii_case(&needle) {
            Some(value.trim().trim_matches('"').to_owned())
        } else {
            None
        }
    })
}

fn yaml_normalized_parameter(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn yaml_detect_line_ending_style(source: &str) -> Option<&'static str> {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut saw_crlf = false;
    let mut saw_lf = false;
    let mut saw_cr = false;

    while index < bytes.len() {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                saw_crlf = true;
                index += 2;
            }
            b'\r' => {
                saw_cr = true;
                index += 1;
            }
            b'\n' => {
                saw_lf = true;
                index += 1;
            }
            _ => {
                index += 1;
            }
        }
    }

    match (
        usize::from(saw_crlf) + usize::from(saw_lf) + usize::from(saw_cr),
        saw_crlf,
        saw_lf,
        saw_cr,
    ) {
        (0, _, _, _) => None,
        (1, true, false, false) => Some("crlf"),
        (1, false, true, false) => Some("lf"),
        (1, false, false, true) => Some("cr"),
        _ => Some("mixed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARSE_ERROR_DIAGNOSTIC: &str = "cem.yaml.parse_error";
    const UNSAFE_TAG_DIAGNOSTIC: &str = "cem.yaml.unsafe_tag";
    const DUPLICATE_ANCHOR_DIAGNOSTIC: &str = "cem.yaml.duplicate_anchor";

    #[test]
    fn yaml_parse_report_facts_are_schema_bound() {
        let report = extract_yaml_parse_report(YamlSourceValidationRequest {
            bytes: b"unsafe: !python/object/apply:os.system [echo bad]\n",
            source_uri: "memory://unsafe.yaml",
            content_type: Some("application/yaml"),
        });
        let contracts = YamlSchemaContractCatalog::from_builtin();
        let diagnostics = validate_yaml_parse_report(&report, &contracts);

        assert!(report
            .facts
            .iter()
            .any(|fact| fact.kind == YamlParseFactKind::UnsafeTag));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == UNSAFE_TAG_DIAGNOSTIC));
        let fact = report
            .facts
            .iter()
            .find(|fact| fact.kind == YamlParseFactKind::UnsafeTag)
            .expect("unsafe tag fact");
        assert_eq!(fact.line, Some(1));
        assert!(fact.column.is_some());
        assert!(fact.byte_offset.is_some());
    }

    #[test]
    fn yaml_stream_projection_preserves_source_facing_facts() {
        let (stream, diagnostics) =
            yaml_stream_value_from_source_bytes(YamlSourceValidationRequest {
                bytes: b"defaults: &base\n  name: Ada\npeople:\n  - *base\n",
                source_uri: "memory://basic.yaml",
                content_type: Some("text/yaml; charset=utf-8"),
            });

        assert!(
            diagnostics.is_empty(),
            "valid YAML should not produce diagnostics: {diagnostics:#?}"
        );
        let stream = stream.expect("valid YAML projects stream data");
        assert_eq!(stream["kind"], "yaml-stream");
        assert_eq!(stream["source"]["contentType"], "text/yaml; charset=utf-8");
        assert_eq!(stream["encodingReport"]["normalizedCharset"], "utf-8");
        assert_eq!(stream["lineEnding"], "lf");
        let documents = stream["documents"].as_array().expect("documents");
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0]["root"]["kind"], "mapping");
        assert_eq!(
            documents[0]["root"]["mapping"][0]["key"]["value"],
            "defaults"
        );
        assert_eq!(
            documents[0]["root"]["mapping"][0]["value"]["anchor"],
            "base"
        );
        assert_eq!(
            documents[0]["root"]["mapping"][1]["value"]["sequence"][0]["kind"],
            "alias"
        );
        assert_eq!(
            documents[0]["root"]["mapping"][1]["value"]["sequence"][0]["alias"],
            "base"
        );
        assert_eq!(
            documents[0]["root"]["mapping"][0]["key"]["sourceMap"]["frames"][0]["source_id"],
            1
        );
    }

    #[test]
    fn yaml_duplicate_anchor_is_recoverable_schema_owned_fact() {
        let (stream, diagnostics) =
            yaml_stream_value_from_source_bytes(YamlSourceValidationRequest {
                bytes: b"first: &dup 1\nsecond: &dup 2\n",
                source_uri: "memory://duplicate.yaml",
                content_type: Some("application/yaml"),
            });

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DUPLICATE_ANCHOR_DIAGNOSTIC));
        let stream = stream.expect("warning-only YAML still projects stream data");
        let facts = stream["parseFacts"].as_array().expect("parse facts");
        assert!(facts.iter().any(|fact| {
            fact["kind"] == "duplicate-anchor"
                && fact["diagnosticSeverity"] == "warning"
                && fact["recoverable"] == true
                && fact["fatal"] == false
        }));
    }

    #[test]
    fn yaml_parse_error_is_schema_owned_fact() {
        let (_stream, diagnostics) =
            yaml_stream_value_from_source_bytes(YamlSourceValidationRequest {
                bytes: b"name: [unterminated\n",
                source_uri: "memory://invalid.yaml",
                content_type: Some("application/yaml"),
            });

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == PARSE_ERROR_DIAGNOSTIC));
    }
}
