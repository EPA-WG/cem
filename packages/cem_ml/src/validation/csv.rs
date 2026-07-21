use crate::diagnostics::{Diagnostic, Severity};
use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use serde_json::json;
use std::collections::BTreeMap;

const CSV_PACKAGE_ID: &str = "csv";
const CSV_PARSE_REPORT_BEHAVIOR: &str = "csv-parse-report-fact";

const CSV_SOURCE_PARSER_CONTRACT: &str = "csv-source-parser";
const CHARSET_PARAMETER_SUPPORTED_CONTRACT: &str = "charset-parameter-supported";
const UTF8_DECODE_CONTRACT: &str = "utf-8-decode";
const US_ASCII_BYTE_COMPATIBILITY_CONTRACT: &str = "us-ascii-byte-compatibility";
const HEADER_PARAMETER_VALUES_CONTRACT: &str = "header-parameter-values";
const FIELD_COUNT_POLICY_CONTRACT: &str = "field-count-policy";
const QUOTE_ESCAPE_POLICY_CONTRACT: &str = "quote-escape-policy";
const QUOTE_CLOSURE_POLICY_CONTRACT: &str = "quote-closure-policy";

const PARSE_ERROR_DIAGNOSTIC: &str = "cem.csv.parse_error";
const UNSUPPORTED_ENCODING_DIAGNOSTIC: &str = "cem.csv.unsupported_encoding";
const INCONSISTENT_FIELD_COUNT_DIAGNOSTIC: &str = "cem.csv.inconsistent_field_count";
const UNCLOSED_QUOTE_DIAGNOSTIC: &str = "cem.csv.unclosed_quote";
const INVALID_QUOTE_ESCAPE_DIAGNOSTIC: &str = "cem.csv.invalid_quote_escape";
const INVALID_HEADER_PARAMETER_DIAGNOSTIC: &str = "cem.csv.invalid_header_parameter";

#[derive(Debug, Clone, Copy)]
pub struct CsvSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvParseReport {
    pub source_uri: String,
    pub content_type: Option<String>,
    pub byte_len: u64,
    pub facts: Vec<CsvParseFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvParseFact {
    pub kind: CsvParseFactKind,
    pub parameter: Option<String>,
    pub actual: Option<String>,
    pub expected: Vec<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub row_index: Option<usize>,
    pub field_index: Option<usize>,
    pub expected_count: Option<usize>,
    pub actual_count: Option<usize>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvParseFactKind {
    ParseError,
    UnsupportedCharset,
    UnsupportedEncoding,
    DeclaredUsAsciiNonAsciiByte,
    InvalidHeaderParameter,
    InvalidQuoteEscape,
    UnclosedQuote,
    RaggedRow,
}

impl CsvParseFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CsvParseFactKind::ParseError => "parse-error",
            CsvParseFactKind::UnsupportedCharset => "unsupported-charset",
            CsvParseFactKind::UnsupportedEncoding => "unsupported-encoding",
            CsvParseFactKind::DeclaredUsAsciiNonAsciiByte => "declared-us-ascii-non-ascii-byte",
            CsvParseFactKind::InvalidHeaderParameter => "invalid-header-parameter",
            CsvParseFactKind::InvalidQuoteEscape => "invalid-quote-escape",
            CsvParseFactKind::UnclosedQuote => "unclosed-quote",
            CsvParseFactKind::RaggedRow => "ragged-row",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvSchemaContractCatalog {
    pub parse_error: CsvDiagnosticBinding,
    pub unsupported_charset: CsvDiagnosticBinding,
    pub unsupported_encoding: CsvDiagnosticBinding,
    pub us_ascii_byte_compatibility: CsvDiagnosticBinding,
    pub header_parameter_values: CsvDiagnosticBinding,
    pub field_count_policy: CsvDiagnosticBinding,
    pub quote_escape_policy: CsvDiagnosticBinding,
    pub quote_closure_policy: CsvDiagnosticBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDiagnosticBinding {
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

impl CsvSchemaContractCatalog {
    pub fn from_builtin() -> Self {
        let source = crate::schema::package_sources::builtin_schema_package_source(CSV_PACKAGE_ID)
            .expect("built-in CSV schema package source must be registered");
        Self::from_schema_source(source.schema_source)
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let document = parse_cem_document(schema_source);

        Self {
            parse_error: diagnostic_binding_for_constraint(
                &document,
                CSV_SOURCE_PARSER_CONTRACT,
                PARSE_ERROR_DIAGNOSTIC,
                Severity::Error,
            ),
            unsupported_charset: diagnostic_binding_for_constraint(
                &document,
                CHARSET_PARAMETER_SUPPORTED_CONTRACT,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
                Severity::Error,
            ),
            unsupported_encoding: diagnostic_binding_for_constraint(
                &document,
                UTF8_DECODE_CONTRACT,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
                Severity::Error,
            ),
            us_ascii_byte_compatibility: diagnostic_binding_for_constraint(
                &document,
                US_ASCII_BYTE_COMPATIBILITY_CONTRACT,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
                Severity::Error,
            ),
            header_parameter_values: diagnostic_binding_for_constraint(
                &document,
                HEADER_PARAMETER_VALUES_CONTRACT,
                INVALID_HEADER_PARAMETER_DIAGNOSTIC,
                Severity::Warning,
            ),
            field_count_policy: diagnostic_binding_for_constraint(
                &document,
                FIELD_COUNT_POLICY_CONTRACT,
                INCONSISTENT_FIELD_COUNT_DIAGNOSTIC,
                Severity::Warning,
            ),
            quote_escape_policy: diagnostic_binding_for_constraint(
                &document,
                QUOTE_ESCAPE_POLICY_CONTRACT,
                INVALID_QUOTE_ESCAPE_DIAGNOSTIC,
                Severity::Error,
            ),
            quote_closure_policy: diagnostic_binding_for_constraint(
                &document,
                QUOTE_CLOSURE_POLICY_CONTRACT,
                UNCLOSED_QUOTE_DIAGNOSTIC,
                Severity::Error,
            ),
        }
    }

    fn binding_for_fact(&self, kind: CsvParseFactKind) -> &CsvDiagnosticBinding {
        match kind {
            CsvParseFactKind::ParseError => &self.parse_error,
            CsvParseFactKind::UnsupportedCharset => &self.unsupported_charset,
            CsvParseFactKind::UnsupportedEncoding => &self.unsupported_encoding,
            CsvParseFactKind::DeclaredUsAsciiNonAsciiByte => &self.us_ascii_byte_compatibility,
            CsvParseFactKind::InvalidHeaderParameter => &self.header_parameter_values,
            CsvParseFactKind::InvalidQuoteEscape => &self.quote_escape_policy,
            CsvParseFactKind::UnclosedQuote => &self.quote_closure_policy,
            CsvParseFactKind::RaggedRow => &self.field_count_policy,
        }
    }
}

pub fn validate_csv_source_bytes(request: CsvSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let contracts = CsvSchemaContractCatalog::from_builtin();
    let report = extract_csv_parse_report(request);
    validate_csv_parse_report(&report, &contracts)
}

pub fn extract_csv_parse_report(request: CsvSourceValidationRequest<'_>) -> CsvParseReport {
    let mut report = CsvParseReport {
        source_uri: request.source_uri.to_owned(),
        content_type: request.content_type.map(str::to_owned),
        byte_len: request.bytes.len() as u64,
        facts: Vec::new(),
    };

    let charset = request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "charset"));
    if let Some(charset) = charset.as_deref() {
        if !csv_charset_is_supported(charset) {
            report.facts.push(csv_fact(
                CsvParseFactKind::UnsupportedCharset,
                Some("charset"),
                Some(charset),
                &["utf-8", "utf8", "us-ascii", "ascii"],
            ));
            return report;
        }
    }

    if let Some(header) = request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "header"))
    {
        if !csv_header_parameter_is_known(&header) {
            report.facts.push(csv_fact(
                CsvParseFactKind::InvalidHeaderParameter,
                Some("header"),
                Some(&header),
                &["present", "absent"],
            ));
        }
    }

    if charset
        .as_deref()
        .is_some_and(csv_charset_declares_us_ascii)
    {
        if let Some(byte_offset) = request.bytes.iter().position(|byte| !byte.is_ascii()) {
            let mut fact = csv_fact(
                CsvParseFactKind::DeclaredUsAsciiNonAsciiByte,
                Some("charset"),
                charset.as_deref(),
                &["us-ascii byte range"],
            );
            fact.byte_offset = Some(byte_offset as u64);
            report.facts.push(fact);
            return report;
        }
    }

    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            let mut fact = csv_fact(
                CsvParseFactKind::UnsupportedEncoding,
                Some("charset"),
                charset.as_deref().or(Some("utf-8")),
                &["valid UTF-8"],
            );
            fact.byte_offset = u64::try_from(error.valid_up_to()).ok();
            fact.message = Some(format!("CSV source must be valid UTF-8: {error}"));
            report.facts.push(fact);
            return report;
        }
    };

    collect_csv_quote_policy_facts(source, &mut report.facts);
    collect_csv_record_facts(source, &mut report.facts);
    report
}

pub fn validate_csv_parse_report(
    report: &CsvParseReport,
    contracts: &CsvSchemaContractCatalog,
) -> Vec<Diagnostic> {
    report
        .facts
        .iter()
        .map(|fact| {
            csv_fact_diagnostic(
                report,
                fact,
                contracts.binding_for_fact(fact.kind),
                csv_fact_message(fact),
            )
        })
        .collect()
}

fn collect_csv_record_facts(source: &str, facts: &mut Vec<CsvParseFact>) {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(source.as_bytes());
    let mut expected_field_count: Option<usize> = None;
    for (row_index, result) in reader.records().enumerate() {
        match result {
            Ok(record) => {
                let field_count = record.len();
                if let Some(expected) = expected_field_count {
                    if field_count != expected {
                        let mut fact = csv_fact(CsvParseFactKind::RaggedRow, None, None, &[]);
                        fact.actual = Some(field_count.to_string());
                        fact.expected = vec![expected.to_string()];
                        fact.row_index = Some(row_index + 1);
                        fact.expected_count = Some(expected);
                        fact.actual_count = Some(field_count);
                        fact.line = csv_position_line(record.position());
                        fact.byte_offset = csv_position_byte_offset(record.position());
                        facts.push(fact);
                    }
                } else {
                    expected_field_count = Some(field_count);
                }
            }
            Err(error) => {
                let mut fact = csv_fact(csv_parse_error_fact_kind(&error), None, None, &[]);
                fact.line = csv_position_line(error.position());
                fact.byte_offset = csv_position_byte_offset(error.position());
                fact.message = Some(format!("CSV parse error: {error}"));
                facts.push(fact);
                break;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CsvSourcePosition {
    line: u32,
    column: u32,
    byte_offset: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CsvQuoteState {
    StartField,
    InUnquoted,
    InQuoted { open: CsvSourcePosition },
    AfterQuote,
}

fn collect_csv_quote_policy_facts(source: &str, facts: &mut Vec<CsvParseFact>) {
    let bytes = source.as_bytes();
    let mut state = CsvQuoteState::StartField;
    let mut byte = 0usize;
    let mut line = 1u32;
    let mut column = 1u32;

    while byte < bytes.len() {
        let current = CsvSourcePosition {
            line,
            column,
            byte_offset: byte as u64,
        };
        match state {
            CsvQuoteState::StartField => match bytes[byte] {
                b'"' => {
                    state = CsvQuoteState::InQuoted { open: current };
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b',' => {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b'\r' | b'\n' => {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                    state = CsvQuoteState::StartField;
                }
                _ => {
                    state = CsvQuoteState::InUnquoted;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            },
            CsvQuoteState::InUnquoted => match bytes[byte] {
                b'"' => {
                    facts.push(csv_positioned_fact(
                        CsvParseFactKind::InvalidQuoteEscape,
                        current,
                        "CSV quote appears inside an unquoted field",
                    ));
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b',' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b'\r' | b'\n' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                _ => {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            },
            CsvQuoteState::InQuoted { .. } => {
                if bytes[byte] == b'"' {
                    if bytes.get(byte + 1) == Some(&b'"') {
                        byte += 2;
                        column = column.saturating_add(2);
                    } else {
                        state = CsvQuoteState::AfterQuote;
                        advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                    }
                } else {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            }
            CsvQuoteState::AfterQuote => match bytes[byte] {
                b',' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b'\r' | b'\n' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                _ => {
                    facts.push(csv_positioned_fact(
                        CsvParseFactKind::InvalidQuoteEscape,
                        current,
                        "CSV quoted field has non-delimiter content after the closing quote",
                    ));
                    state = CsvQuoteState::InUnquoted;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            },
        }
    }

    if let CsvQuoteState::InQuoted { open } = state {
        facts.push(csv_positioned_fact(
            CsvParseFactKind::UnclosedQuote,
            open,
            "CSV quoted field is missing a closing quote",
        ));
    }
}

fn advance_csv_cursor(bytes: &[u8], byte: &mut usize, line: &mut u32, column: &mut u32) {
    match bytes.get(*byte).copied() {
        Some(b'\r') => {
            if bytes.get(*byte + 1) == Some(&b'\n') {
                *byte += 2;
            } else {
                *byte += 1;
            }
            *line = line.saturating_add(1);
            *column = 1;
        }
        Some(b'\n') => {
            *byte += 1;
            *line = line.saturating_add(1);
            *column = 1;
        }
        Some(_) => {
            *byte += 1;
            *column = column.saturating_add(1);
        }
        None => {}
    }
}

fn csv_fact(
    kind: CsvParseFactKind,
    parameter: Option<&str>,
    actual: Option<&str>,
    expected: &[&str],
) -> CsvParseFact {
    CsvParseFact {
        kind,
        parameter: parameter.map(str::to_owned),
        actual: actual.map(str::to_owned),
        expected: expected.iter().map(|value| (*value).to_owned()).collect(),
        line: None,
        column: None,
        byte_offset: None,
        row_index: None,
        field_index: None,
        expected_count: None,
        actual_count: None,
        message: None,
    }
}

fn csv_positioned_fact(
    kind: CsvParseFactKind,
    position: CsvSourcePosition,
    message: &str,
) -> CsvParseFact {
    let mut fact = csv_fact(kind, None, None, &[]);
    fact.line = Some(position.line);
    fact.column = Some(position.column);
    fact.byte_offset = Some(position.byte_offset);
    fact.message = Some(message.to_owned());
    fact
}

fn csv_fact_diagnostic(
    report: &CsvParseReport,
    fact: &CsvParseFact,
    binding: &CsvDiagnosticBinding,
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
                "line": fact.line,
                "column": fact.column,
            },
            "rowIndex": fact.row_index,
            "fieldIndex": fact.field_index,
            "expected": fact.expected,
            "actual": fact.actual.as_deref(),
            "expectedCount": fact.expected_count,
            "actualCount": fact.actual_count,
            "byteLength": report.byte_len,
        })),
        ..Diagnostic::default()
    }
}

fn csv_fact_message(fact: &CsvParseFact) -> String {
    if let Some(message) = fact.message.clone() {
        return message;
    }

    match fact.kind {
        CsvParseFactKind::ParseError => "CSV parse error".to_owned(),
        CsvParseFactKind::UnsupportedCharset => format!(
            "CSV content-type charset `{}` is not supported",
            fact.actual.as_deref().unwrap_or("")
        ),
        CsvParseFactKind::UnsupportedEncoding => "CSV source must be valid UTF-8".to_owned(),
        CsvParseFactKind::DeclaredUsAsciiNonAsciiByte => {
            "CSV source contains non-ASCII bytes but declares us-ascii charset".to_owned()
        }
        CsvParseFactKind::InvalidHeaderParameter => format!(
            "CSV header parameter `{}` must be `present` or `absent`",
            fact.actual.as_deref().unwrap_or("")
        ),
        CsvParseFactKind::InvalidQuoteEscape => {
            "CSV quote is not escaped according to the CSV quote policy".to_owned()
        }
        CsvParseFactKind::UnclosedQuote => "CSV quoted field is missing a closing quote".to_owned(),
        CsvParseFactKind::RaggedRow => format!(
            "CSV row {} has {} fields; expected {} from the first row",
            fact.row_index.unwrap_or_default(),
            fact.actual_count.unwrap_or_default(),
            fact.expected_count.unwrap_or_default()
        ),
    }
}

fn diagnostic_binding_for_constraint(
    document: &CemDocument,
    contract: &str,
    fallback_diagnostic_code: &str,
    fallback_severity: Severity,
) -> CsvDiagnosticBinding {
    let constraint_attrs =
        first_element_attrs_by_attr(document, "constraint", "kind", contract).unwrap_or_default();
    let diagnostic_code = constraint_attrs
        .get("diagnostic")
        .cloned()
        .unwrap_or_else(|| fallback_diagnostic_code.to_owned());
    let diagnostic_attrs =
        first_element_attrs_by_attr(document, "diagnostic", "code", &diagnostic_code)
            .unwrap_or_default();
    let severity = diagnostic_attrs
        .get("severity")
        .and_then(|value| parse_severity(value))
        .unwrap_or(fallback_severity);

    CsvDiagnosticBinding {
        contract: contract.to_owned(),
        behavior: constraint_attrs
            .get("behavior")
            .cloned()
            .or_else(|| Some(CSV_PARSE_REPORT_BEHAVIOR.to_owned())),
        diagnostic_code,
        severity,
        policy: constraint_attrs.get("policy").cloned(),
    }
}

fn parse_severity(value: &str) -> Option<Severity> {
    match value.trim().to_ascii_lowercase().as_str() {
        "info" => Some(Severity::Info),
        "warning" => Some(Severity::Warning),
        "error" => Some(Severity::Error),
        "fatal" => Some(Severity::Fatal),
        _ => None,
    }
}

fn csv_parse_error_fact_kind(error: &csv::Error) -> CsvParseFactKind {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("utf-8") || message.contains("utf8") {
        CsvParseFactKind::UnsupportedEncoding
    } else if (message.contains("eof") || message.contains("end of file"))
        && message.contains("quote")
    {
        CsvParseFactKind::UnclosedQuote
    } else if message.contains("quote") {
        CsvParseFactKind::InvalidQuoteEscape
    } else {
        CsvParseFactKind::ParseError
    }
}

fn csv_position_line(position: Option<&csv::Position>) -> Option<u32> {
    position.and_then(|position| u32::try_from(position.line()).ok())
}

fn csv_position_byte_offset(position: Option<&csv::Position>) -> Option<u64> {
    position.map(csv::Position::byte)
}

fn csv_normalized_parameter(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn csv_charset_is_supported(charset: &str) -> bool {
    matches!(
        csv_normalized_parameter(charset).as_str(),
        "utf-8" | "utf8" | "us-ascii" | "ascii"
    )
}

fn csv_charset_declares_us_ascii(charset: &str) -> bool {
    matches!(
        csv_normalized_parameter(charset).as_str(),
        "us-ascii" | "ascii"
    )
}

fn csv_header_parameter_is_known(header: &str) -> bool {
    matches!(
        csv_normalized_parameter(header).as_str(),
        "present" | "absent"
    )
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

fn parse_cem_document(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

fn first_element_attrs_by_attr(
    document: &CemDocument,
    local_name: &str,
    attr_name: &str,
    attr_value: &str,
) -> Option<BTreeMap<String, String>> {
    document.iter().find_map(|node| {
        let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        else {
            return None;
        };
        if expanded_name.local_name != local_name {
            return None;
        }

        let attrs = collect_attrs(document, *node_id);
        attrs
            .get(attr_name)
            .is_some_and(|value| value == attr_value)
            .then_some(attrs)
    })
}

fn collect_attrs(document: &CemDocument, node_id: AstNodeId) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = document.get(node_id) else {
        return attrs;
    };

    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attr_id)
        else {
            continue;
        };
        attrs.insert(
            expanded_name.local_name.clone(),
            value.clone().unwrap_or_default(),
        );
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parse_report_facts_are_neutral() {
        let report = extract_csv_parse_report(CsvSourceValidationRequest {
            bytes: b"id,name\n1,Ada\n",
            source_uri: "memory://table.csv",
            content_type: Some("text/csv; header=maybe"),
        });

        assert_eq!(report.facts.len(), 1);
        let fact = &report.facts[0];
        assert_eq!(fact.kind, CsvParseFactKind::InvalidHeaderParameter);
        assert_eq!(fact.kind.as_str(), "invalid-header-parameter");
        assert_eq!(fact.actual.as_deref(), Some("maybe"));
    }

    #[test]
    fn csv_current_validation_facts_are_schema_mapped() {
        let cases = [
            (
                "unsupported charset",
                b"id,name\n1,Ada\n".as_slice(),
                Some("text/csv; charset=iso-8859-1"),
                CsvParseFactKind::UnsupportedCharset,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
            ),
            (
                "us-ascii mismatch",
                b"id,name\n1,Ad\xc3\xa9\n".as_slice(),
                Some("text/csv; charset=us-ascii"),
                CsvParseFactKind::DeclaredUsAsciiNonAsciiByte,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
            ),
            (
                "invalid utf8",
                b"id,name\n1,\xff\n".as_slice(),
                Some("text/csv"),
                CsvParseFactKind::UnsupportedEncoding,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
            ),
            (
                "invalid header",
                b"id,name\n1,Ada\n".as_slice(),
                Some("text/csv; header=maybe"),
                CsvParseFactKind::InvalidHeaderParameter,
                INVALID_HEADER_PARAMETER_DIAGNOSTIC,
            ),
            (
                "invalid quote escape",
                b"id,name\n1,A\"da\n".as_slice(),
                Some("text/csv"),
                CsvParseFactKind::InvalidQuoteEscape,
                INVALID_QUOTE_ESCAPE_DIAGNOSTIC,
            ),
            (
                "unclosed quote",
                b"id,name\n1,\"Ada\n".as_slice(),
                Some("text/csv"),
                CsvParseFactKind::UnclosedQuote,
                UNCLOSED_QUOTE_DIAGNOSTIC,
            ),
            (
                "ragged row",
                b"id,name,email\n1,Ada,ada@example.test\n2,Lin\n".as_slice(),
                Some("text/csv"),
                CsvParseFactKind::RaggedRow,
                INCONSISTENT_FIELD_COUNT_DIAGNOSTIC,
            ),
        ];

        for (name, bytes, content_type, fact_kind, diagnostic_code) in cases {
            let report = extract_csv_parse_report(CsvSourceValidationRequest {
                bytes,
                source_uri: "memory://table.csv",
                content_type,
            });
            assert!(
                report.facts.iter().any(|fact| fact.kind == fact_kind),
                "{name} should report fact {fact_kind:?}: {report:#?}"
            );

            let diagnostics =
                validate_csv_parse_report(&report, &CsvSchemaContractCatalog::from_builtin());
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == diagnostic_code),
                "{name} should emit {diagnostic_code}: {diagnostics:#?}"
            );
            assert!(
                diagnostics.iter().all(|diagnostic| diagnostic
                    .details
                    .as_ref()
                    .is_some_and(|details| details.get("factKind").is_some())),
                "{name} diagnostics should carry schema-owned fact details: {diagnostics:#?}"
            );
        }
    }

    #[test]
    fn csv_header_diagnostic_is_schema_declared() {
        let source = crate::schema::package_sources::builtin_schema_package_source(CSV_PACKAGE_ID)
            .expect("CSV package source")
            .schema_source
            .replace(
                r#"{constraint @kind="header-parameter-values" @target="table" @diagnostic="cem.csv.invalid_header_parameter" @behavior="csv-parse-report-fact" @policy="header parameter values must be present or absent; other values are reported as metadata drift"}"#,
                r#"{constraint @kind="header-parameter-values" @target="table" @diagnostic="example.csv.header_parameter" @behavior="csv-parse-report-fact" @policy="header parameter values must be present or absent; other values are reported as metadata drift"}"#,
            )
            .replace(
                r#"{diagnostic @code="cem.csv.invalid_header_parameter" @severity="warning"}"#,
                r#"{diagnostic @code="example.csv.header_parameter" @severity="error"}"#,
            );
        let contracts = CsvSchemaContractCatalog::from_schema_source(&source);
        let report = extract_csv_parse_report(CsvSourceValidationRequest {
            bytes: b"id,name\n1,Ada\n",
            source_uri: "memory://table.csv",
            content_type: Some("text/csv; header=maybe"),
        });
        let diagnostics = validate_csv_parse_report(&report, &contracts);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "example.csv.header_parameter");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(
            diagnostics[0].details.as_ref().unwrap()["contract"],
            HEADER_PARAMETER_VALUES_CONTRACT
        );
        assert_eq!(
            diagnostics[0].details.as_ref().unwrap()["factKind"],
            "invalid-header-parameter"
        );
    }
}
