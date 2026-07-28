use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{
    content_type_essence, JSON_SCHEMA_CONTENT_TYPE, JSON_SCHEMA_SCHEMA_URI,
};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::json::{
    extract_json_parse_report, json_document_ast_from_parse_report, JsonDocumentAst, JsonParseFact,
    JsonParseFactKind, JsonParseReport, JsonSourceRange, JsonSourceValidationRequest, JsonValueAst,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const JSON_SCHEMA_PACKAGE_ID: &str = "json-schema";
const JSON_SCHEMA_PARSE_ERROR_CODE: &str = "cem.json_schema.parse_error";
const JSON_SCHEMA_UNSUPPORTED_DIALECT_CODE: &str = "cem.json_schema.unsupported_dialect";
const SUPPORTED_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
const SUPPORTED_DRAFT_2020_12_HASH: &str = "https://json-schema.org/draft/2020-12/schema#";

#[derive(Debug, Clone, Copy)]
pub struct JsonSchemaSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSchemaDocumentAst {
    pub source: JsonSchemaDocumentSource,
    pub json: JsonDocumentAst,
    pub parse_facts: Vec<JsonSchemaParseFact>,
    pub dialect_facts: Vec<JsonSchemaDialectFact>,
    pub dialect: String,
}

impl JsonSchemaDocumentAst {
    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": "json-schema-document",
            "contentType": JSON_SCHEMA_CONTENT_TYPE,
            "schema": JSON_SCHEMA_SCHEMA_URI,
            "source": self.source.to_cemt_subject(),
            "json": self.json.to_cemt_subject(),
            "parseFacts": self
                .parse_facts
                .iter()
                .map(JsonSchemaParseFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "dialectFacts": self
                .dialect_facts
                .iter()
                .map(JsonSchemaDialectFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "dialect": self.dialect,
        })
    }

    pub fn into_json_document_ast(self) -> JsonDocumentAst {
        self.json
    }

    pub fn to_json_value(&self) -> Option<Value> {
        self.json.to_json_value()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSchemaDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl JsonSchemaDocumentSource {
    fn from_request(request: JsonSchemaSourceValidationRequest<'_>) -> Self {
        let content_type = request.content_type.unwrap_or(JSON_SCHEMA_CONTENT_TYPE);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSchemaParseFact {
    pub kind: JsonParseFactKind,
    pub diagnostic_code: String,
    pub diagnostic_severity: String,
    pub fatal: bool,
    pub member_name: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub byte_length: Option<u64>,
    pub message: String,
}

impl JsonSchemaParseFact {
    fn from_json_parse_fact(fact: &JsonParseFact, catalog: &JsonSchemaDiagnosticCatalog) -> Self {
        let severity = catalog.severity(JSON_SCHEMA_PARSE_ERROR_CODE, Severity::Error);
        Self {
            kind: fact.kind,
            diagnostic_code: JSON_SCHEMA_PARSE_ERROR_CODE.to_owned(),
            diagnostic_severity: severity_name(severity).to_owned(),
            fatal: json_schema_parse_fact_is_fatal(fact.kind),
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
            "diagnosticCode": self.diagnostic_code,
            "diagnosticSeverity": self.diagnostic_severity,
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
pub struct JsonSchemaDialectFact {
    pub kind: JsonSchemaDialectFactKind,
    pub dialect: Option<String>,
    pub diagnostic_code: Option<String>,
    pub diagnostic_severity: Option<String>,
    pub fatal: bool,
    pub source_range: Option<JsonSourceRange>,
    pub message: String,
}

impl JsonSchemaDialectFact {
    fn diagnostic(
        kind: JsonSchemaDialectFactKind,
        dialect: Option<String>,
        source_range: Option<JsonSourceRange>,
        message: impl Into<String>,
        catalog: &JsonSchemaDiagnosticCatalog,
    ) -> Self {
        let severity = catalog.severity(JSON_SCHEMA_UNSUPPORTED_DIALECT_CODE, Severity::Error);
        Self {
            kind,
            dialect,
            diagnostic_code: Some(JSON_SCHEMA_UNSUPPORTED_DIALECT_CODE.to_owned()),
            diagnostic_severity: Some(severity_name(severity).to_owned()),
            fatal: severity.is_hard_violation(),
            source_range,
            message: message.into(),
        }
    }

    fn supported(dialect: String, source_range: Option<JsonSourceRange>) -> Self {
        Self {
            kind: JsonSchemaDialectFactKind::SupportedDialect,
            dialect: Some(dialect),
            diagnostic_code: None,
            diagnostic_severity: None,
            fatal: false,
            source_range,
            message: "JSON Schema dialect is supported".to_owned(),
        }
    }

    fn to_cemt_subject(&self) -> Value {
        let mut fact = serde_json::Map::new();
        fact.insert("kind".to_owned(), json!(self.kind.as_str()));
        fact.insert("dialect".to_owned(), json!(self.dialect));
        fact.insert("diagnosticCode".to_owned(), json!(self.diagnostic_code));
        fact.insert(
            "diagnosticSeverity".to_owned(),
            json!(self.diagnostic_severity),
        );
        fact.insert("fatal".to_owned(), json!(self.fatal));
        fact.insert("message".to_owned(), json!(self.message));
        if let Some(source_range) = self.source_range {
            fact.insert("sourceRange".to_owned(), source_range.to_cemt_subject());
            fact.insert(
                "sourceMap".to_owned(),
                json!(json_schema_source_map(source_range)),
            );
        }
        Value::Object(fact)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonSchemaDialectFactKind {
    RootNotObject,
    MissingDialect,
    UnsupportedDialect,
    SupportedDialect,
}

impl JsonSchemaDialectFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RootNotObject => "root-not-object",
            Self::MissingDialect => "missing-dialect",
            Self::UnsupportedDialect => "unsupported-dialect",
            Self::SupportedDialect => "supported-dialect",
        }
    }
}

pub fn validate_json_schema_source_bytes(
    request: JsonSchemaSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let (_, diagnostics) = json_schema_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn json_schema_document_ast_from_source_bytes(
    request: JsonSchemaSourceValidationRequest<'_>,
) -> (Option<JsonSchemaDocumentAst>, Vec<Diagnostic>) {
    let catalog = JsonSchemaDiagnosticCatalog::from_builtin();
    let json_request = JsonSourceValidationRequest {
        bytes: request.bytes,
        source_uri: request.source_uri,
        content_type: request.content_type.or(Some(JSON_SCHEMA_CONTENT_TYPE)),
    };
    let report = extract_json_parse_report(json_request);
    let parse_facts = report
        .facts
        .iter()
        .map(|fact| JsonSchemaParseFact::from_json_parse_fact(fact, catalog))
        .collect::<Vec<_>>();
    let mut diagnostics = validate_json_schema_parse_report(&report, catalog);
    let fatal_parse = parse_facts.iter().any(|fact| fact.fatal);
    let json_document = (!fatal_parse)
        .then(|| json_document_ast_from_parse_report(json_request, &report))
        .flatten();

    let mut dialect_facts = Vec::new();
    if let Some(json_document) = json_document.as_ref() {
        dialect_facts = collect_json_schema_dialect_facts(json_document, catalog);
        diagnostics.extend(
            dialect_facts
                .iter()
                .filter_map(|fact| json_schema_dialect_diagnostic(&report, fact, catalog)),
        );
    }

    let fatal = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation());
    let dialect = supported_json_schema_dialect(&dialect_facts);
    let document = (!fatal).then(|| JsonSchemaDocumentAst {
        source: JsonSchemaDocumentSource::from_request(request),
        json: json_document.expect("non-fatal JSON Schema validation has a JSON AST"),
        parse_facts,
        dialect_facts,
        dialect,
    });

    (document, diagnostics)
}

fn validate_json_schema_parse_report(
    report: &JsonParseReport,
    catalog: &JsonSchemaDiagnosticCatalog,
) -> Vec<Diagnostic> {
    report
        .facts
        .iter()
        .map(|fact| json_schema_parse_diagnostic(report, fact, catalog))
        .collect()
}

fn json_schema_parse_diagnostic(
    report: &JsonParseReport,
    fact: &JsonParseFact,
    catalog: &JsonSchemaDiagnosticCatalog,
) -> Diagnostic {
    let severity = catalog.severity(JSON_SCHEMA_PARSE_ERROR_CODE, Severity::Error);
    Diagnostic {
        uri: Some(report.source_uri.clone()),
        line: fact.line,
        column: fact.column,
        byte_offset: fact.byte_offset,
        code: JSON_SCHEMA_PARSE_ERROR_CODE.to_owned(),
        severity,
        message: format!("JSON Schema parse error: {}", fact.message),
        node: None,
        details: Some(json!({
            "jsonSchema": {
                "factKind": fact.kind.as_str(),
                "phase": "parse",
                "memberName": fact.member_name,
                "sourceRange": {
                    "byteOffset": fact.byte_offset,
                    "byteLength": fact.byte_length,
                    "line": fact.line,
                    "column": fact.column,
                },
            },
        })),
        source_map: source_map_for_parse_fact(fact),
    }
}

fn collect_json_schema_dialect_facts(
    document: &JsonDocumentAst,
    catalog: &JsonSchemaDiagnosticCatalog,
) -> Vec<JsonSchemaDialectFact> {
    let Some(root) = document.root.as_ref() else {
        return vec![JsonSchemaDialectFact::diagnostic(
            JsonSchemaDialectFactKind::RootNotObject,
            None,
            None,
            "JSON Schema document must be an object with a `$schema` dialect declaration",
            catalog,
        )];
    };

    let JsonValueAst::Object { range, members } = root else {
        return vec![JsonSchemaDialectFact::diagnostic(
            JsonSchemaDialectFactKind::RootNotObject,
            None,
            Some(root.range()),
            "JSON Schema document must be an object with a `$schema` dialect declaration",
            catalog,
        )];
    };

    let Some(schema_member) = members.iter().find(|member| member.name == "$schema") else {
        return vec![JsonSchemaDialectFact::diagnostic(
            JsonSchemaDialectFactKind::MissingDialect,
            None,
            Some(*range),
            "JSON Schema object is missing required `$schema` dialect declaration",
            catalog,
        )];
    };

    let JsonValueAst::String {
        value: dialect,
        range,
        ..
    } = &schema_member.value
    else {
        return vec![JsonSchemaDialectFact::diagnostic(
            JsonSchemaDialectFactKind::UnsupportedDialect,
            None,
            Some(schema_member.value.range()),
            "JSON Schema `$schema` dialect declaration must be a string",
            catalog,
        )];
    };

    match dialect.as_str() {
        SUPPORTED_DRAFT_2020_12 | SUPPORTED_DRAFT_2020_12_HASH => {
            vec![JsonSchemaDialectFact::supported(
                dialect.clone(),
                Some(*range),
            )]
        }
        other => vec![JsonSchemaDialectFact::diagnostic(
            JsonSchemaDialectFactKind::UnsupportedDialect,
            Some(other.to_owned()),
            Some(*range),
            format!("unsupported JSON Schema dialect `{other}`; expected Draft 2020-12"),
            catalog,
        )],
    }
}

fn json_schema_dialect_diagnostic(
    report: &JsonParseReport,
    fact: &JsonSchemaDialectFact,
    catalog: &JsonSchemaDiagnosticCatalog,
) -> Option<Diagnostic> {
    let code = fact.diagnostic_code.as_ref()?;
    let severity = catalog.severity(code, Severity::Error);
    Some(Diagnostic {
        uri: Some(report.source_uri.clone()),
        line: fact.source_range.map(|range| range.start.line),
        column: fact.source_range.map(|range| range.start.column),
        byte_offset: fact.source_range.map(|range| range.start.byte_offset),
        code: code.clone(),
        severity,
        message: fact.message.clone(),
        node: None,
        details: Some(json!({
            "jsonSchema": {
                "factKind": fact.kind.as_str(),
                "phase": "dialect",
                "dialect": fact.dialect,
                "sourceRange": fact.source_range.map(JsonSourceRange::to_cemt_subject),
            },
        })),
        source_map: fact.source_range.map(json_schema_source_map),
    })
}

fn supported_json_schema_dialect(facts: &[JsonSchemaDialectFact]) -> String {
    facts
        .iter()
        .find_map(|fact| {
            (fact.kind == JsonSchemaDialectFactKind::SupportedDialect)
                .then(|| fact.dialect.clone())
                .flatten()
        })
        .unwrap_or_else(|| SUPPORTED_DRAFT_2020_12.to_owned())
}

fn json_schema_parse_fact_is_fatal(kind: JsonParseFactKind) -> bool {
    matches!(
        kind,
        JsonParseFactKind::ParseError | JsonParseFactKind::UnsupportedEncoding
    )
}

fn source_map_for_parse_fact(fact: &JsonParseFact) -> Option<SourceMapStack> {
    fact.byte_offset.map(|offset| {
        json_schema_source_map(JsonSourceRange {
            start: crate::validation::json::JsonSourcePosition {
                line: fact.line.unwrap_or(1),
                column: fact.column.unwrap_or(1),
                byte_offset: offset,
            },
            byte_length: fact.byte_length.unwrap_or(0),
        })
    })
}

fn json_schema_source_map(range: JsonSourceRange) -> SourceMapStack {
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(1),
            span: FrameSpan::Single(ByteRange::new(
                range.start.byte_offset,
                u32::try_from(range.byte_length).unwrap_or(u32::MAX),
            )),
            transform: TransformKind::ContentTypeTransform {
                content_type: JSON_SCHEMA_CONTENT_TYPE.to_owned(),
            },
        }],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsonSchemaDiagnosticCatalog {
    severities: BTreeMap<String, Severity>,
}

impl JsonSchemaDiagnosticCatalog {
    fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<JsonSchemaDiagnosticCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(JSON_SCHEMA_PACKAGE_ID)
                .expect("built-in JSON Schema package source must be registered");
            let model = compile_schema_document_model(JSON_SCHEMA_SCHEMA_URI, source.schema_source);
            let severities = model
                .diagnostics
                .into_iter()
                .map(|(code, diagnostic)| (code, diagnostic.severity))
                .collect();
            JsonSchemaDiagnosticCatalog { severities }
        })
    }

    fn severity(&self, code: &str, fallback: Severity) -> Severity {
        self.severities.get(code).copied().unwrap_or(fallback)
    }
}

fn severity_name(severity: Severity) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_schema_ast_accepts_draft_2020_12_dialect() {
        let (document, diagnostics) = json_schema_document_ast_from_source_bytes(
            JsonSchemaSourceValidationRequest {
                bytes:
                    br#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object"}"#,
                source_uri: "fixture.schema.json",
                content_type: Some(JSON_SCHEMA_CONTENT_TYPE),
            },
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let document = document.expect("valid JSON Schema AST");
        assert_eq!(document.source.content_type, JSON_SCHEMA_CONTENT_TYPE);
        assert_eq!(document.dialect, SUPPORTED_DRAFT_2020_12);
        assert!(matches!(
            document.dialect_facts[0].kind,
            JsonSchemaDialectFactKind::SupportedDialect
        ));
        assert_eq!(
            document
                .to_json_value()
                .and_then(|value| { value["type"].as_str().map(str::to_owned) }),
            Some("object".to_owned())
        );
    }

    #[test]
    fn json_schema_ast_binds_json_parse_error_to_schema_diagnostic() {
        let (document, diagnostics) =
            json_schema_document_ast_from_source_bytes(JsonSchemaSourceValidationRequest {
                bytes: br#"{"$schema":"https://json-schema.org/draft/2020-12/schema",}"#,
                source_uri: "invalid.schema.json",
                content_type: Some(JSON_SCHEMA_CONTENT_TYPE),
            });

        assert!(document.is_none());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == JSON_SCHEMA_PARSE_ERROR_CODE
                && diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("jsonSchema"))
                    .and_then(|details| details.get("phase"))
                    == Some(&json!("parse"))
        }));
    }

    #[test]
    fn json_schema_ast_binds_unsupported_dialect_to_schema_diagnostic() {
        let (document, diagnostics) =
            json_schema_document_ast_from_source_bytes(JsonSchemaSourceValidationRequest {
                bytes: br#"{"$schema":"http://json-schema.org/draft-07/schema#","type":"object"}"#,
                source_uri: "draft7.schema.json",
                content_type: Some(JSON_SCHEMA_CONTENT_TYPE),
            });

        assert!(document.is_none());
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == JSON_SCHEMA_UNSUPPORTED_DIALECT_CODE)
            .expect("unsupported dialect diagnostic");
        assert_eq!(diagnostic.line, Some(1));
        assert!(diagnostic.source_map.is_some());
    }
}
