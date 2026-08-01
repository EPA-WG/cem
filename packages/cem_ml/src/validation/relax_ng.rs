use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{
    content_type_essence, RELAX_NG_COMPACT_CONTENT_TYPE, RELAX_NG_NAMESPACE_URI,
    RELAX_NG_SCHEMA_URI, RELAX_NG_XML_CONTENT_TYPE,
};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::xml::{
    xml_document_ast_from_source_bytes, XmlDocumentAst, XmlEventKind, XmlParseFactKind,
    XmlSourceValidationRequest,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const RELAX_NG_PACKAGE_ID: &str = "relax-ng";
const RELAX_NG_FACT_BEHAVIOR: &str = "relax-ng-report-fact";

#[derive(Debug, Clone, Copy)]
pub struct RelaxNgSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelaxNgSyntaxKind {
    Xml,
    Compact,
}

impl RelaxNgSyntaxKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Xml => "xml",
            Self::Compact => "compact",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Xml => RELAX_NG_XML_CONTENT_TYPE,
            Self::Compact => RELAX_NG_COMPACT_CONTENT_TYPE,
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Self::Xml => "relax-ng-xml-document",
            Self::Compact => "relax-ng-compact-document",
        }
    }

    pub fn formatter_function(self) -> &'static str {
        match self {
            Self::Xml => "relax-ng.format-xml-document",
            Self::Compact => "relax-ng.format-compact-document",
        }
    }

    pub fn colorizer_function(self) -> &'static str {
        match self {
            Self::Xml => "relax-ng.color-xml-document",
            Self::Compact => "relax-ng.color-compact-document",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaxNgDocumentAst {
    pub source: RelaxNgDocumentSource,
    pub syntax_kind: RelaxNgSyntaxKind,
    pub xml_document: Option<XmlDocumentAst>,
    pub compact_tokens: Vec<RelaxNgCompactTokenAst>,
    pub facts: Vec<RelaxNgFact>,
    pub line_ending: Option<String>,
}

impl RelaxNgDocumentAst {
    pub fn to_cemt_subject(&self) -> Value {
        let xml_events = self
            .xml_document
            .as_ref()
            .map(XmlDocumentAst::to_cemt_subject)
            .and_then(|subject| subject.get("events").cloned())
            .unwrap_or_else(|| Value::Array(Vec::new()));
        json!({
            "kind": "relax-ng-document",
            "contentType": self.syntax_kind.content_type(),
            "schema": RELAX_NG_SCHEMA_URI,
            "category": self.syntax_kind.category(),
            "syntaxKind": self.syntax_kind.as_str(),
            "source": self.source.to_cemt_subject(),
            "xmlEvents": xml_events,
            "compactTokens": self
                .compact_tokens
                .iter()
                .map(|token| token.to_cemt_subject(&self.source.media_type))
                .collect::<Vec<_>>(),
            "parseFacts": self
                .facts
                .iter()
                .map(RelaxNgFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "lineEnding": self.line_ending,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaxNgDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl RelaxNgDocumentSource {
    fn from_request(
        request: RelaxNgSourceValidationRequest<'_>,
        syntax_kind: RelaxNgSyntaxKind,
    ) -> Self {
        let content_type = request
            .content_type
            .unwrap_or_else(|| syntax_kind.content_type());
        Self {
            uri: request.source_uri.to_owned(),
            content_type: content_type.to_owned(),
            media_type: content_type_essence(content_type),
            parameters: content_type_parameters(request.content_type),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelaxNgSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelaxNgSourceRange {
    pub start: RelaxNgSourcePosition,
    pub byte_length: u64,
}

impl RelaxNgSourceRange {
    fn from_offsets(line_index: &LineIndex, start: usize, end: usize) -> Self {
        let coordinate = line_index.project(start as u64);
        Self {
            start: RelaxNgSourcePosition {
                line: coordinate.line,
                column: coordinate.column,
                byte_offset: start as u64,
            },
            byte_length: end.saturating_sub(start) as u64,
        }
    }

    fn from_optional_parts(
        line: Option<u32>,
        column: Option<u32>,
        byte_offset: Option<u64>,
        byte_length: Option<u64>,
    ) -> Option<Self> {
        Some(Self {
            start: RelaxNgSourcePosition {
                line: line.unwrap_or(1),
                column: column.unwrap_or(1),
                byte_offset: byte_offset?,
            },
            byte_length: byte_length.unwrap_or(1),
        })
    }

    fn to_cemt_subject(self) -> Value {
        json!({
            "byteOffset": self.start.byte_offset,
            "byteLength": self.byte_length,
            "line": self.start.line,
            "column": self.start.column,
        })
    }

    fn source_map(self, content_type: &str) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    self.start.byte_offset,
                    u32::try_from(self.byte_length).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: content_type.to_owned(),
                },
            }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelaxNgCompactTokenKind {
    Keyword,
    Identifier,
    String,
    Operator,
    Punctuation,
    Comment,
    Whitespace,
    Raw,
}

impl RelaxNgCompactTokenKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Identifier => "identifier",
            Self::String => "string",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::Comment => "comment",
            Self::Whitespace => "whitespace",
            Self::Raw => "raw",
        }
    }

    fn role(self) -> &'static str {
        match self {
            Self::Keyword => "syntax.keyword",
            Self::Identifier => "syntax.name",
            Self::String => "syntax.string",
            Self::Operator | Self::Punctuation => "syntax.punctuation",
            Self::Comment => "syntax.comment",
            Self::Whitespace => "syntax.whitespace",
            Self::Raw => "syntax.raw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaxNgCompactTokenAst {
    pub index: usize,
    pub kind: RelaxNgCompactTokenKind,
    pub lexeme: String,
    pub depth: usize,
    pub source_range: RelaxNgSourceRange,
}

impl RelaxNgCompactTokenAst {
    fn to_cemt_subject(&self, content_type: &str) -> Value {
        json!({
            "index": self.index,
            "kind": self.kind.as_str(),
            "lexeme": self.lexeme,
            "depth": self.depth,
            "role": self.kind.role(),
            "sourceRange": self.source_range.to_cemt_subject(),
            "sourceMap": self.source_range.source_map(content_type),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelaxNgFactKind {
    XmlParseError,
    CompactParseError,
    UnsupportedEncoding,
    EncodingConflict,
    InvalidNamespace,
    InvalidRoot,
    UnknownElement,
    MissingRequiredAttribute,
    MissingStart,
    IncludeRejected,
    ExternalReferenceRejected,
    SourceMapUnavailable,
    NamespaceObserved,
    StartObserved,
    PatternObserved,
}

impl RelaxNgFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::XmlParseError => "xml-parse-error",
            Self::CompactParseError => "compact-parse-error",
            Self::UnsupportedEncoding => "unsupported-encoding",
            Self::EncodingConflict => "encoding-conflict",
            Self::InvalidNamespace => "invalid-namespace",
            Self::InvalidRoot => "invalid-root",
            Self::UnknownElement => "unknown-element",
            Self::MissingRequiredAttribute => "missing-required-attribute",
            Self::MissingStart => "missing-start",
            Self::IncludeRejected => "include-rejected",
            Self::ExternalReferenceRejected => "external-reference-rejected",
            Self::SourceMapUnavailable => "source-map-unavailable",
            Self::NamespaceObserved => "namespace-observed",
            Self::StartObserved => "start-observed",
            Self::PatternObserved => "pattern-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaxNgFact {
    pub kind: RelaxNgFactKind,
    pub syntax_kind: RelaxNgSyntaxKind,
    pub source_range: Option<RelaxNgSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

impl RelaxNgFact {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "syntaxKind": self.syntax_kind.as_str(),
            "sourceRange": self.source_range.map(RelaxNgSourceRange::to_cemt_subject),
            "message": self.message,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaxNgParseReport {
    pub source_uri: String,
    pub content_type: String,
    pub syntax_kind: RelaxNgSyntaxKind,
    pub facts: Vec<RelaxNgFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RelaxNgDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaxNgSchemaContractCatalog {
    fact_bindings: BTreeMap<String, RelaxNgDiagnosticBinding>,
}

impl RelaxNgSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<RelaxNgSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(RELAX_NG_PACKAGE_ID)
                .expect("built-in RELAX NG schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(RELAX_NG_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != RELAX_NG_FACT_BEHAVIOR {
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
                    RelaxNgDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: RelaxNgFactKind) -> Option<&RelaxNgDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_relax_ng_source_bytes(
    request: RelaxNgSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let (_, diagnostics) = relax_ng_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn relax_ng_document_ast_from_source_bytes(
    request: RelaxNgSourceValidationRequest<'_>,
) -> (Option<RelaxNgDocumentAst>, Vec<Diagnostic>) {
    let syntax_kind = relax_ng_syntax_kind(request.content_type);
    let source = RelaxNgDocumentSource::from_request(request, syntax_kind);
    let line_ending = std::str::from_utf8(request.bytes)
        .ok()
        .and_then(detect_line_ending_style)
        .map(str::to_owned);
    let (xml_document, compact_tokens, facts) = match syntax_kind {
        RelaxNgSyntaxKind::Xml => {
            let (document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: request.bytes,
                source_uri: request.source_uri,
                content_type: request.content_type.or(Some(RELAX_NG_XML_CONTENT_TYPE)),
            });
            let facts = relax_ng_xml_facts(document.as_ref());
            (document, Vec::new(), facts)
        }
        RelaxNgSyntaxKind::Compact => {
            let (tokens, facts) = relax_ng_compact_report(request);
            (None, tokens, facts)
        }
    };
    let report = RelaxNgParseReport {
        source_uri: request.source_uri.to_owned(),
        content_type: source.media_type.clone(),
        syntax_kind,
        facts: facts.clone(),
    };
    let diagnostics =
        validate_relax_ng_parse_report(&report, RelaxNgSchemaContractCatalog::from_builtin());
    (
        Some(RelaxNgDocumentAst {
            source,
            syntax_kind,
            xml_document,
            compact_tokens,
            facts,
            line_ending,
        }),
        diagnostics,
    )
}

pub fn validate_relax_ng_parse_report(
    report: &RelaxNgParseReport,
    contracts: &RelaxNgSchemaContractCatalog,
) -> Vec<Diagnostic> {
    report
        .facts
        .iter()
        .filter_map(|fact| relax_ng_diagnostic_from_fact(report, fact, contracts))
        .collect()
}

fn relax_ng_diagnostic_from_fact(
    report: &RelaxNgParseReport,
    fact: &RelaxNgFact,
    contracts: &RelaxNgSchemaContractCatalog,
) -> Option<Diagnostic> {
    let binding = contracts.binding_for_fact(fact.kind)?;
    Some(Diagnostic {
        uri: Some(report.source_uri.clone()),
        line: fact.source_range.map(|range| range.start.line),
        column: fact.source_range.map(|range| range.start.column),
        byte_offset: fact.source_range.map(|range| range.start.byte_offset),
        code: binding.diagnostic_code.clone(),
        severity: binding.severity,
        message: fact.message.clone(),
        details: Some(json!({
            "relaxNg": {
                "phase": "parse-and-semantics",
                "factKind": fact.kind.as_str(),
                "syntaxKind": report.syntax_kind.as_str(),
                "contract": binding.contract,
                "behavior": binding.behavior,
                "policy": binding.policy,
                "contentType": report.content_type,
                "value": fact.value,
                "sourceRange": fact.source_range.map(RelaxNgSourceRange::to_cemt_subject),
            }
        })),
        source_map: fact
            .source_range
            .map(|range| range.source_map(&report.content_type)),
        ..Diagnostic::default()
    })
}

fn relax_ng_syntax_kind(content_type: Option<&str>) -> RelaxNgSyntaxKind {
    if content_type.map(content_type_essence).as_deref() == Some(RELAX_NG_COMPACT_CONTENT_TYPE) {
        RelaxNgSyntaxKind::Compact
    } else {
        RelaxNgSyntaxKind::Xml
    }
}

fn relax_ng_xml_facts(document: Option<&XmlDocumentAst>) -> Vec<RelaxNgFact> {
    let Some(document) = document else {
        return vec![RelaxNgFact {
            kind: RelaxNgFactKind::XmlParseError,
            syntax_kind: RelaxNgSyntaxKind::Xml,
            source_range: None,
            message: "RELAX NG XML syntax could not be parsed".to_owned(),
            value: None,
        }];
    };
    let mut facts = document
        .parse_facts
        .iter()
        .map(|fact| RelaxNgFact {
            kind: match fact.kind {
                XmlParseFactKind::UnsupportedEncoding => RelaxNgFactKind::UnsupportedEncoding,
                XmlParseFactKind::EncodingConflict => RelaxNgFactKind::EncodingConflict,
                XmlParseFactKind::SourceMapUnavailable => RelaxNgFactKind::SourceMapUnavailable,
                _ => RelaxNgFactKind::XmlParseError,
            },
            syntax_kind: RelaxNgSyntaxKind::Xml,
            source_range: RelaxNgSourceRange::from_optional_parts(
                fact.line,
                fact.column,
                fact.byte_offset,
                fact.byte_length,
            ),
            message: fact.message.clone(),
            value: Some(fact.kind.as_str().to_owned()),
        })
        .collect::<Vec<_>>();
    let mut root_seen = false;
    let mut root_is_grammar = false;
    let mut start_seen = false;

    for event in &document.events {
        if !matches!(
            event.kind,
            XmlEventKind::StartElement | XmlEventKind::EmptyElement
        ) {
            continue;
        }
        let local_name = event.local_name.as_deref().unwrap_or_default();
        let namespace_uri = event.namespace_uri.as_deref().unwrap_or_default();
        let range = Some(RelaxNgSourceRange {
            start: RelaxNgSourcePosition {
                line: event.source_range.start.line,
                column: event.source_range.start.column,
                byte_offset: event.source_range.start.byte_offset,
            },
            byte_length: event.source_range.byte_length,
        });

        if !root_seen {
            root_seen = true;
            root_is_grammar = namespace_uri == RELAX_NG_NAMESPACE_URI && local_name == "grammar";
            if namespace_uri != RELAX_NG_NAMESPACE_URI {
                facts.push(RelaxNgFact {
                    kind: RelaxNgFactKind::InvalidNamespace,
                    syntax_kind: RelaxNgSyntaxKind::Xml,
                    source_range: range,
                    message: format!(
                        "RELAX NG XML root must use namespace `{RELAX_NG_NAMESPACE_URI}`"
                    ),
                    value: Some(namespace_uri.to_owned()),
                });
            }
            if local_name != "grammar" {
                facts.push(RelaxNgFact {
                    kind: RelaxNgFactKind::InvalidRoot,
                    syntax_kind: RelaxNgSyntaxKind::Xml,
                    source_range: range,
                    message: "RELAX NG XML syntax must use `grammar` as the document element"
                        .to_owned(),
                    value: event.qualified_name.clone(),
                });
            }
        }

        if namespace_uri != RELAX_NG_NAMESPACE_URI {
            continue;
        }
        facts.push(RelaxNgFact {
            kind: RelaxNgFactKind::PatternObserved,
            syntax_kind: RelaxNgSyntaxKind::Xml,
            source_range: range,
            message: format!("RELAX NG XML pattern `{local_name}` was parsed"),
            value: Some(local_name.to_owned()),
        });
        if !relax_ng_known_xml_element(local_name) {
            facts.push(RelaxNgFact {
                kind: RelaxNgFactKind::UnknownElement,
                syntax_kind: RelaxNgSyntaxKind::Xml,
                source_range: range,
                message: format!(
                    "RELAX NG XML syntax element `{}` is not in the RELAX NG structure vocabulary",
                    event.qualified_name.as_deref().unwrap_or(local_name)
                ),
                value: event.qualified_name.clone(),
            });
            continue;
        }
        if local_name == "start" {
            start_seen = true;
            facts.push(RelaxNgFact {
                kind: RelaxNgFactKind::StartObserved,
                syntax_kind: RelaxNgSyntaxKind::Xml,
                source_range: range,
                message: "RELAX NG XML start pattern was parsed".to_owned(),
                value: None,
            });
        }
        if local_name == "include" {
            facts.push(rejected_reference_fact(
                RelaxNgSyntaxKind::Xml,
                RelaxNgFactKind::IncludeRejected,
                range,
                xml_attribute_value(event, "href"),
            ));
        } else if local_name == "externalRef" {
            facts.push(rejected_reference_fact(
                RelaxNgSyntaxKind::Xml,
                RelaxNgFactKind::ExternalReferenceRejected,
                range,
                xml_attribute_value(event, "href"),
            ));
        }
        if let Some(attribute) = required_xml_attribute(local_name) {
            if xml_attribute_value(event, attribute).is_none() {
                facts.push(RelaxNgFact {
                    kind: RelaxNgFactKind::MissingRequiredAttribute,
                    syntax_kind: RelaxNgSyntaxKind::Xml,
                    source_range: range,
                    message: format!(
                        "RELAX NG XML `{local_name}` requires attribute `{attribute}`"
                    ),
                    value: Some(attribute.to_owned()),
                });
            }
        }
    }

    if !root_seen {
        facts.push(RelaxNgFact {
            kind: RelaxNgFactKind::InvalidRoot,
            syntax_kind: RelaxNgSyntaxKind::Xml,
            source_range: None,
            message: "RELAX NG XML syntax must contain a grammar document element".to_owned(),
            value: None,
        });
    } else if root_is_grammar && !start_seen {
        facts.push(RelaxNgFact {
            kind: RelaxNgFactKind::MissingStart,
            syntax_kind: RelaxNgSyntaxKind::Xml,
            source_range: document.events.first().map(|event| RelaxNgSourceRange {
                start: RelaxNgSourcePosition {
                    line: event.source_range.start.line,
                    column: event.source_range.start.column,
                    byte_offset: event.source_range.start.byte_offset,
                },
                byte_length: event.source_range.byte_length,
            }),
            message: "RELAX NG grammar must declare a start pattern".to_owned(),
            value: None,
        });
    }
    if root_is_grammar {
        facts.push(RelaxNgFact {
            kind: RelaxNgFactKind::NamespaceObserved,
            syntax_kind: RelaxNgSyntaxKind::Xml,
            source_range: None,
            message: "RELAX NG structure namespace was parsed".to_owned(),
            value: Some(RELAX_NG_NAMESPACE_URI.to_owned()),
        });
    }
    facts
}

fn relax_ng_compact_report(
    request: RelaxNgSourceValidationRequest<'_>,
) -> (Vec<RelaxNgCompactTokenAst>, Vec<RelaxNgFact>) {
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            return (
                Vec::new(),
                vec![RelaxNgFact {
                    kind: RelaxNgFactKind::UnsupportedEncoding,
                    syntax_kind: RelaxNgSyntaxKind::Compact,
                    source_range: Some(RelaxNgSourceRange {
                        start: RelaxNgSourcePosition {
                            line: 1,
                            column: 1,
                            byte_offset: error.valid_up_to() as u64,
                        },
                        byte_length: error.error_len().unwrap_or(1) as u64,
                    }),
                    message: format!("RELAX NG compact source must be valid UTF-8: {error}"),
                    value: None,
                }],
            )
        }
    };
    let parameters = content_type_parameters(request.content_type);
    if let Some(charset) = parameters.get("charset") {
        let normalized = charset.trim().trim_matches('"').to_ascii_lowercase();
        if !matches!(normalized.as_str(), "utf-8" | "us-ascii")
            || (normalized == "us-ascii" && !source.is_ascii())
        {
            return (
                Vec::new(),
                vec![RelaxNgFact {
                    kind: RelaxNgFactKind::UnsupportedEncoding,
                    syntax_kind: RelaxNgSyntaxKind::Compact,
                    source_range: None,
                    message: format!(
                        "RELAX NG compact content-type charset `{charset}` is not supported by the UTF-8 source adapter"
                    ),
                    value: Some(charset.clone()),
                }],
            );
        }
    }

    let line_index = LineIndex::from_utf8(source);
    let (tokens, mut facts) = lex_relax_ng_compact(source, &line_index);
    let significant = tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.kind,
                RelaxNgCompactTokenKind::Whitespace | RelaxNgCompactTokenKind::Comment
            )
        })
        .collect::<Vec<_>>();
    let start = significant.windows(2).find(|pair| {
        pair[0].lexeme == "start"
            && pair[1].kind == RelaxNgCompactTokenKind::Operator
            && matches!(pair[1].lexeme.as_str(), "=" | "|=" | "&=")
    });
    if let Some(pair) = start {
        facts.push(RelaxNgFact {
            kind: RelaxNgFactKind::StartObserved,
            syntax_kind: RelaxNgSyntaxKind::Compact,
            source_range: Some(pair[0].source_range),
            message: "RELAX NG compact start definition was parsed".to_owned(),
            value: None,
        });
    } else {
        facts.push(RelaxNgFact {
            kind: RelaxNgFactKind::MissingStart,
            syntax_kind: RelaxNgSyntaxKind::Compact,
            source_range: significant.first().map(|token| token.source_range),
            message: "RELAX NG compact syntax must declare a start pattern".to_owned(),
            value: None,
        });
    }

    for (index, token) in significant.iter().enumerate() {
        if token.kind != RelaxNgCompactTokenKind::Keyword {
            continue;
        }
        if relax_ng_compact_pattern_keyword(&token.lexeme) {
            facts.push(RelaxNgFact {
                kind: RelaxNgFactKind::PatternObserved,
                syntax_kind: RelaxNgSyntaxKind::Compact,
                source_range: Some(token.source_range),
                message: format!("RELAX NG compact pattern `{}` was parsed", token.lexeme),
                value: Some(token.lexeme.clone()),
            });
        }
        if token.lexeme == "include" || token.lexeme == "external" {
            let href = significant
                .get(index + 1)
                .filter(|next| next.kind == RelaxNgCompactTokenKind::String)
                .map(|next| next.lexeme.trim_matches(['\'', '"']).to_owned());
            facts.push(rejected_reference_fact(
                RelaxNgSyntaxKind::Compact,
                if token.lexeme == "include" {
                    RelaxNgFactKind::IncludeRejected
                } else {
                    RelaxNgFactKind::ExternalReferenceRejected
                },
                Some(token.source_range),
                href,
            ));
        }
        if matches!(token.lexeme.as_str(), "namespace" | "default") {
            facts.push(RelaxNgFact {
                kind: RelaxNgFactKind::NamespaceObserved,
                syntax_kind: RelaxNgSyntaxKind::Compact,
                source_range: Some(token.source_range),
                message: "RELAX NG compact namespace declaration was parsed".to_owned(),
                value: Some(token.lexeme.clone()),
            });
        }
    }
    (tokens, facts)
}

fn lex_relax_ng_compact(
    source: &str,
    line_index: &LineIndex,
) -> (Vec<RelaxNgCompactTokenAst>, Vec<RelaxNgFact>) {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut facts = Vec::new();
    let mut offset = 0usize;
    let mut delimiters: Vec<(u8, usize)> = Vec::new();

    while offset < bytes.len() {
        let start = offset;
        let (kind, end) = match bytes[offset] {
            byte if byte.is_ascii_whitespace() => {
                offset += 1;
                while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
                    offset += 1;
                }
                (RelaxNgCompactTokenKind::Whitespace, offset)
            }
            b'#' => {
                offset += 1;
                while offset < bytes.len() && bytes[offset] != b'\n' {
                    offset += 1;
                }
                (RelaxNgCompactTokenKind::Comment, offset)
            }
            quote @ (b'\'' | b'"') => {
                offset += 1;
                let mut escaped = false;
                let mut closed = false;
                while offset < bytes.len() {
                    let byte = bytes[offset];
                    offset += 1;
                    if escaped {
                        escaped = false;
                    } else if byte == b'\\' {
                        escaped = true;
                    } else if byte == quote {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    facts.push(compact_fact(
                        line_index,
                        start,
                        source.len(),
                        RelaxNgFactKind::CompactParseError,
                        "RELAX NG compact syntax has an unclosed string literal".to_owned(),
                    ));
                }
                (RelaxNgCompactTokenKind::String, offset)
            }
            b'{' | b'(' | b'[' => {
                delimiters.push((bytes[offset], offset));
                offset += 1;
                (RelaxNgCompactTokenKind::Punctuation, offset)
            }
            close @ (b'}' | b')' | b']') => {
                let expected = matching_open_delimiter(close);
                if delimiters.last().map(|(open, _)| *open) == Some(expected) {
                    delimiters.pop();
                } else {
                    facts.push(compact_fact(
                        line_index,
                        start,
                        start + 1,
                        RelaxNgFactKind::CompactParseError,
                        format!(
                            "RELAX NG compact syntax has an unmatched `{}`",
                            close as char
                        ),
                    ));
                }
                offset += 1;
                (RelaxNgCompactTokenKind::Punctuation, offset)
            }
            b'|' | b'&' if bytes.get(offset + 1) == Some(&b'=') => {
                offset += 2;
                (RelaxNgCompactTokenKind::Operator, offset)
            }
            b'=' | b'|' | b'&' | b',' | b'?' | b'*' | b'+' | b'-' => {
                offset += 1;
                (RelaxNgCompactTokenKind::Operator, offset)
            }
            b';' => {
                offset += 1;
                (RelaxNgCompactTokenKind::Punctuation, offset)
            }
            byte if compact_identifier_byte(byte) => {
                offset += 1;
                while offset < bytes.len() && compact_identifier_byte(bytes[offset]) {
                    offset += 1;
                }
                let lexeme = &source[start..offset];
                (
                    if relax_ng_compact_keyword(lexeme) {
                        RelaxNgCompactTokenKind::Keyword
                    } else {
                        RelaxNgCompactTokenKind::Identifier
                    },
                    offset,
                )
            }
            _ => {
                let character_length = source[offset..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1);
                offset += character_length;
                (RelaxNgCompactTokenKind::Raw, offset)
            }
        };
        let depth = delimiters.len();
        tokens.push(RelaxNgCompactTokenAst {
            index: tokens.len(),
            kind,
            lexeme: source[start..end].to_owned(),
            depth,
            source_range: RelaxNgSourceRange::from_offsets(line_index, start, end),
        });
    }

    for (open, start) in delimiters {
        facts.push(compact_fact(
            line_index,
            start,
            start + 1,
            RelaxNgFactKind::CompactParseError,
            format!(
                "RELAX NG compact syntax has an unclosed `{}` delimiter",
                open as char
            ),
        ));
    }
    (tokens, facts)
}

fn compact_fact(
    line_index: &LineIndex,
    start: usize,
    end: usize,
    kind: RelaxNgFactKind,
    message: String,
) -> RelaxNgFact {
    RelaxNgFact {
        kind,
        syntax_kind: RelaxNgSyntaxKind::Compact,
        source_range: Some(RelaxNgSourceRange::from_offsets(line_index, start, end)),
        message,
        value: None,
    }
}

fn rejected_reference_fact(
    syntax_kind: RelaxNgSyntaxKind,
    kind: RelaxNgFactKind,
    source_range: Option<RelaxNgSourceRange>,
    href: Option<String>,
) -> RelaxNgFact {
    let construct = if kind == RelaxNgFactKind::IncludeRejected {
        "include"
    } else {
        "external reference"
    };
    RelaxNgFact {
        kind,
        syntax_kind,
        source_range,
        message: format!(
            "RELAX NG {construct}{} is rejected until resolver policy enables it",
            href.as_deref()
                .map(|href| format!(" `{href}`"))
                .unwrap_or_default()
        ),
        value: href,
    }
}

fn xml_attribute_value(event: &crate::validation::xml::XmlEventAst, name: &str) -> Option<String> {
    event
        .attributes
        .iter()
        .find(|attribute| attribute.qualified_name == name)
        .map(|attribute| attribute.value.clone())
}

fn required_xml_attribute(local_name: &str) -> Option<&'static str> {
    match local_name {
        "define" | "ref" | "parentRef" | "param" => Some("name"),
        "include" | "externalRef" => Some("href"),
        "data" => Some("type"),
        _ => None,
    }
}

fn relax_ng_known_xml_element(local_name: &str) -> bool {
    matches!(
        local_name,
        "grammar"
            | "start"
            | "define"
            | "element"
            | "attribute"
            | "choice"
            | "group"
            | "interleave"
            | "oneOrMore"
            | "zeroOrMore"
            | "optional"
            | "list"
            | "mixed"
            | "ref"
            | "parentRef"
            | "empty"
            | "text"
            | "value"
            | "data"
            | "param"
            | "notAllowed"
            | "externalRef"
            | "include"
            | "div"
            | "name"
            | "anyName"
            | "nsName"
            | "except"
    )
}

fn relax_ng_compact_keyword(value: &str) -> bool {
    matches!(
        value,
        "attribute"
            | "default"
            | "div"
            | "element"
            | "empty"
            | "external"
            | "grammar"
            | "include"
            | "inherit"
            | "list"
            | "mixed"
            | "namespace"
            | "notAllowed"
            | "parent"
            | "start"
            | "string"
            | "text"
            | "token"
    )
}

fn relax_ng_compact_pattern_keyword(value: &str) -> bool {
    matches!(
        value,
        "attribute"
            | "element"
            | "empty"
            | "external"
            | "grammar"
            | "list"
            | "mixed"
            | "notAllowed"
            | "parent"
            | "text"
    )
}

fn compact_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'.')
}

fn matching_open_delimiter(close: u8) -> u8 {
    match close {
        b'}' => b'{',
        b')' => b'(',
        b']' => b'[',
        _ => close,
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

fn detect_line_ending_style(source: &str) -> Option<&'static str> {
    let has_crlf = source.contains("\r\n");
    let has_lone_cr = source
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'\r' && pair[1] != b'\n')
        || source.as_bytes().last() == Some(&b'\r');
    let has_lf = source.as_bytes().iter().enumerate().any(|(index, byte)| {
        *byte == b'\n' && (index == 0 || source.as_bytes()[index - 1] != b'\r')
    });
    match (has_crlf, has_lf, has_lone_cr) {
        (false, false, false) => None,
        (true, false, false) => Some("crlf"),
        (false, true, false) => Some("lf"),
        _ => Some("mixed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn relax_ng_xml_ast_reuses_typed_xml_events_and_source_maps() {
        let (document, diagnostics) = relax_ng_document_ast_from_source_bytes(
            RelaxNgSourceValidationRequest {
                bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><empty/></start></grammar>"#,
                source_uri: "fixture.rng",
                content_type: Some(RELAX_NG_XML_CONTENT_TYPE),
            },
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("typed RELAX NG document");
        assert_eq!(document.syntax_kind, RelaxNgSyntaxKind::Xml);
        let xml = document.xml_document.expect("typed XML event stream");
        assert!(xml
            .events
            .iter()
            .all(|event| event.source_range.byte_length > 0));
        assert!(document.compact_tokens.is_empty());
    }

    #[test]
    fn relax_ng_compact_ast_preserves_tokens_and_source_ranges() {
        let source = b"default namespace = \"\"\n\nstart = element note { text }\n";
        let (document, diagnostics) =
            relax_ng_document_ast_from_source_bytes(RelaxNgSourceValidationRequest {
                bytes: source,
                source_uri: "fixture.rnc",
                content_type: Some(RELAX_NG_COMPACT_CONTENT_TYPE),
            });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("typed RELAX NG compact document");
        assert_eq!(document.syntax_kind, RelaxNgSyntaxKind::Compact);
        assert_eq!(
            document
                .compact_tokens
                .iter()
                .map(|token| token.lexeme.as_str())
                .collect::<String>(),
            std::str::from_utf8(source).unwrap()
        );
        assert!(document
            .compact_tokens
            .iter()
            .all(|token| token.source_range.byte_length > 0));
    }

    #[test]
    fn relax_ng_xml_validator_reports_schema_bound_facts() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><define name="x"><unknown/></define></grammar>"#,
            source_uri: "fixture.rng",
            content_type: Some(RELAX_NG_XML_CONTENT_TYPE),
        });

        assert!(has_code(&diagnostics, "cem.relax_ng.missing_start"));
        assert!(has_code(&diagnostics, "cem.relax_ng.unknown_element"));
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("relaxNg"))
                .and_then(|details| details.get("behavior"))
                == Some(&json!(RELAX_NG_FACT_BEHAVIOR))
        }));
    }

    #[test]
    fn relax_ng_xml_validator_preserves_foreign_annotations() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0" xmlns:a="urn:annotation"><start><element name="note"><a:documentation>Visible to tooling.</a:documentation><text/></element></start></grammar>"#,
            source_uri: "fixture.rng",
            content_type: Some(RELAX_NG_XML_CONTENT_TYPE),
        });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn relax_ng_resolver_policy_rejects_xml_and_compact_references() {
        for (source, content_type, code) in [
            (
                br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><externalRef href="https://example.test/schema.rng"/></start></grammar>"#.as_slice(),
                RELAX_NG_XML_CONTENT_TYPE,
                "cem.relax_ng.external_ref_rejected",
            ),
            (
                b"include \"https://example.test/schema.rnc\"\nstart = empty\n".as_slice(),
                RELAX_NG_COMPACT_CONTENT_TYPE,
                "cem.relax_ng.include_rejected",
            ),
        ] {
            let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
                bytes: source,
                source_uri: "fixture",
                content_type: Some(content_type),
            });
            assert!(has_code(&diagnostics, code), "{diagnostics:?}");
        }
    }

    #[test]
    fn relax_ng_compact_validator_reports_unbalanced_delimiters() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: b"start = element note { text\n",
            source_uri: "fixture.rnc",
            content_type: Some(RELAX_NG_COMPACT_CONTENT_TYPE),
        });

        assert!(has_code(&diagnostics, "cem.relax_ng.compact_parse_error"));
    }

    #[test]
    fn relax_ng_validator_reports_unsupported_encoding_for_both_syntaxes() {
        for content_type in [RELAX_NG_XML_CONTENT_TYPE, RELAX_NG_COMPACT_CONTENT_TYPE] {
            let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
                bytes: b"start = \xff",
                source_uri: "fixture",
                content_type: Some(content_type),
            });
            assert!(has_code(&diagnostics, "cem.relax_ng.unsupported_encoding"));
        }
    }
}
