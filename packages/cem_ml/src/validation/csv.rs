use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::registry::{CSV_CONTENT_TYPE, CSV_SCHEMA_URI};
use crate::source::decode::Utf8Decoder;
use crate::source::{ByteRange, BytesSource, EncodingDecoder, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::generic_data::{
    GenericDataDocumentAst, GenericDataMappingEntryAst, GenericDataSourceAst,
    GenericDataSourceRangeAst, GenericDataStreamDocumentAst, GenericDataValueAst,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const CSV_PACKAGE_ID: &str = "csv";
pub const GENERIC_DATA_CSV_NESTED_VALUE_UNSUPPORTED_CODE: &str =
    "cem.lifecycle.generic_data_csv_nested_value_unsupported";

#[derive(Debug, Clone, Copy)]
pub struct CsvSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDocumentAst {
    pub source: CsvDocumentSource,
    pub encoding: String,
    pub encoding_report: CsvEncodingReportAst,
    pub delimiter: String,
    pub header: String,
    pub dialect: CsvDialectAst,
    pub parse_facts: Vec<CsvDocumentParseFact>,
    pub rows: Vec<CsvRecordAst>,
    pub line_ending: Option<String>,
}

impl CsvDocumentAst {
    pub fn to_cemt_subject(&self) -> Value {
        let mut table = serde_json::Map::new();
        table.insert("kind".to_owned(), json!("csv-table"));
        table.insert("source".to_owned(), self.source.to_cemt_subject());
        table.insert("encoding".to_owned(), json!(self.encoding));
        table.insert(
            "encodingReport".to_owned(),
            self.encoding_report.to_cemt_subject(),
        );
        table.insert("delimiter".to_owned(), json!(self.delimiter));
        table.insert("header".to_owned(), json!(self.header));
        table.insert("dialect".to_owned(), self.dialect.to_cemt_subject());
        table.insert(
            "parseFacts".to_owned(),
            Value::Array(
                self.parse_facts
                    .iter()
                    .map(CsvDocumentParseFact::to_cemt_subject)
                    .collect(),
            ),
        );
        table.insert(
            "rows".to_owned(),
            Value::Array(
                self.rows
                    .iter()
                    .map(CsvRecordAst::to_cemt_subject)
                    .collect(),
            ),
        );
        if let Some(line_ending) = self.line_ending.as_deref() {
            table.insert("lineEnding".to_owned(), json!(line_ending));
        }
        Value::Object(table)
    }

    pub fn to_generic_data_ast(&self) -> GenericDataDocumentAst {
        let root = if self.header == "present" && !self.rows.is_empty() {
            csv_records_to_generic_data_mapping_sequence(&self.rows)
        } else {
            csv_records_to_generic_data_row_sequence(&self.rows)
        };
        let source_range = root
            .as_ref()
            .map(|root| root.source_range().clone())
            .unwrap_or_else(GenericDataSourceRangeAst::generated);
        GenericDataDocumentAst {
            source: GenericDataSourceAst {
                uri: self.source.uri.clone(),
                content_type: self.source.content_type.clone(),
                media_type: self.source.media_type.clone(),
                parameters: self.source.parameters.clone(),
                byte_length: self.source.byte_length,
            },
            documents: vec![GenericDataStreamDocumentAst {
                index: 0,
                source_range,
                root,
            }],
            line_ending: self.line_ending.clone(),
        }
    }
}

pub fn generic_data_ast_to_csv_cemt_subject(
    ast: &GenericDataDocumentAst,
) -> (Value, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let roots = ast
        .documents
        .iter()
        .filter_map(|document| document.root.as_ref())
        .collect::<Vec<_>>();
    let rows = if roots.len() == 1 {
        generic_data_value_to_csv_rows(roots[0], &ast.source.uri, &mut diagnostics)
    } else {
        generic_data_values_to_csv_rows(&roots, &ast.source.uri, &mut diagnostics)
    };
    let header = if rows.header_present {
        "present"
    } else {
        "absent"
    };
    let mut table = serde_json::Map::new();
    table.insert("kind".to_owned(), json!("csv-table"));
    table.insert("source".to_owned(), ast.source.to_cemt_subject());
    table.insert("encoding".to_owned(), json!("utf-8"));
    table.insert(
        "encodingReport".to_owned(),
        json!({
            "normalizedCharset": "utf-8",
            "decoderStatus": "decoded",
        }),
    );
    table.insert("delimiter".to_owned(), json!(","));
    table.insert("header".to_owned(), json!(header));
    let mut dialect = serde_json::Map::new();
    dialect.insert("delimiter".to_owned(), json!(","));
    dialect.insert("quote".to_owned(), json!("\""));
    dialect.insert("escape".to_owned(), json!("double-quote"));
    dialect.insert("header".to_owned(), json!(header));
    if let Some(line_ending) = ast.line_ending.as_deref() {
        dialect.insert("lineEnding".to_owned(), json!(line_ending));
        table.insert("lineEnding".to_owned(), json!(line_ending));
    }
    table.insert("dialect".to_owned(), Value::Object(dialect));
    table.insert("parseFacts".to_owned(), Value::Array(Vec::new()));
    table.insert("rows".to_owned(), Value::Array(rows.rows));
    (Value::Object(table), diagnostics)
}

struct GenericDataCsvRows {
    header_present: bool,
    rows: Vec<Value>,
}

fn csv_records_to_generic_data_row_sequence(rows: &[CsvRecordAst]) -> Option<GenericDataValueAst> {
    let source_range = csv_records_source_range(rows);
    Some(GenericDataValueAst::Sequence {
        source_range,
        items: rows
            .iter()
            .map(|row| GenericDataValueAst::Sequence {
                source_range: csv_source_range_to_generic_data_range(row.range),
                items: row
                    .fields
                    .iter()
                    .map(csv_field_to_generic_data_string)
                    .collect(),
            })
            .collect(),
    })
}

fn csv_records_to_generic_data_mapping_sequence(
    rows: &[CsvRecordAst],
) -> Option<GenericDataValueAst> {
    let header = rows.first()?;
    let source_range = csv_records_source_range(rows);
    Some(GenericDataValueAst::Sequence {
        source_range,
        items: rows
            .iter()
            .skip(1)
            .map(|row| GenericDataValueAst::Mapping {
                source_range: csv_source_range_to_generic_data_range(row.range),
                entries: row
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let key = header.fields.get(index).map_or_else(
                            || GenericDataValueAst::String {
                                source_range: GenericDataSourceRangeAst::generated(),
                                value: format!("field{}", index + 1),
                                lexeme: None,
                                style: None,
                            },
                            csv_field_to_generic_data_string,
                        );
                        GenericDataMappingEntryAst {
                            index,
                            key,
                            value: csv_field_to_generic_data_string(field),
                            source_range: csv_source_range_to_generic_data_range(field.range),
                        }
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn csv_field_to_generic_data_string(field: &CsvFieldAst) -> GenericDataValueAst {
    GenericDataValueAst::String {
        source_range: csv_source_range_to_generic_data_range(field.range),
        value: field.value.clone(),
        lexeme: None,
        style: None,
    }
}

fn csv_records_source_range(rows: &[CsvRecordAst]) -> GenericDataSourceRangeAst {
    match (rows.first(), rows.last()) {
        (Some(first), Some(last)) => csv_source_range_to_generic_data_range(CsvSourceRange {
            start: first.range.start,
            end: last.range.end,
        }),
        _ => GenericDataSourceRangeAst::generated(),
    }
}

fn csv_source_range_to_generic_data_range(range: CsvSourceRange) -> GenericDataSourceRangeAst {
    GenericDataSourceRangeAst {
        byte_offset: range.start.byte_offset,
        byte_length: range.byte_length(),
        line: range.start.line,
        column: range.start.column,
        source_map: Some(range.source_map()),
    }
}

fn generic_data_value_to_csv_rows(
    value: &GenericDataValueAst,
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> GenericDataCsvRows {
    match value {
        GenericDataValueAst::Mapping { entries, .. } => {
            generic_data_mappings_to_csv_rows(&[entries], uri, diagnostics)
        }
        GenericDataValueAst::Sequence { items, .. } => {
            if items
                .iter()
                .all(|item| matches!(item, GenericDataValueAst::Mapping { .. }))
            {
                let mappings = items
                    .iter()
                    .filter_map(|item| match item {
                        GenericDataValueAst::Mapping { entries, .. } => Some(entries),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                return generic_data_mappings_to_csv_rows(&mappings, uri, diagnostics);
            }
            let has_mapping = items
                .iter()
                .any(|item| matches!(item, GenericDataValueAst::Mapping { .. }));
            if has_mapping {
                diagnostics.push(generic_data_csv_unsupported_diagnostic(
                    uri,
                    value.source_range(),
                    "mixed mapping and non-mapping generic data sequences cannot be projected to CSV without a tabular schema",
                ));
                return GenericDataCsvRows {
                    header_present: false,
                    rows: Vec::new(),
                };
            }
            GenericDataCsvRows {
                header_present: false,
                rows: items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        generic_data_value_to_csv_row(index, item, uri, diagnostics)
                    })
                    .collect(),
            }
        }
        _ => GenericDataCsvRows {
            header_present: false,
            rows: vec![generic_data_scalar_to_csv_row(0, value, uri, diagnostics)],
        },
    }
}

fn generic_data_values_to_csv_rows(
    values: &[&GenericDataValueAst],
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> GenericDataCsvRows {
    if values
        .iter()
        .all(|value| matches!(value, GenericDataValueAst::Mapping { .. }))
    {
        let mappings = values
            .iter()
            .filter_map(|value| match value {
                GenericDataValueAst::Mapping { entries, .. } => Some(entries),
                _ => None,
            })
            .collect::<Vec<_>>();
        return generic_data_mappings_to_csv_rows(&mappings, uri, diagnostics);
    }
    if values
        .iter()
        .any(|value| matches!(value, GenericDataValueAst::Mapping { .. }))
    {
        let generated_range = GenericDataSourceRangeAst::generated();
        diagnostics.push(generic_data_csv_unsupported_diagnostic(
            uri,
            values
                .first()
                .map(|value| value.source_range())
                .unwrap_or(&generated_range),
            "mixed mapping and non-mapping generic data documents cannot be projected to CSV without a tabular schema",
        ));
        return GenericDataCsvRows {
            header_present: false,
            rows: Vec::new(),
        };
    }
    GenericDataCsvRows {
        header_present: false,
        rows: values
            .iter()
            .enumerate()
            .map(|(index, value)| generic_data_value_to_csv_row(index, value, uri, diagnostics))
            .collect(),
    }
}

fn generic_data_mappings_to_csv_rows(
    mappings: &[&Vec<GenericDataMappingEntryAst>],
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> GenericDataCsvRows {
    let mut header_names = Vec::<(String, GenericDataSourceRangeAst)>::new();
    for entries in mappings {
        for entry in entries.iter() {
            let name = generic_data_value_to_csv_scalar_text(&entry.key, uri, diagnostics);
            if !header_names.iter().any(|(existing, _)| existing == &name) {
                header_names.push((name, entry.key.source_range().clone()));
            }
        }
    }

    let mut rows = Vec::new();
    rows.push(generic_data_header_row_to_csv_row(&header_names));
    rows.extend(mappings.iter().enumerate().map(|(row_offset, entries)| {
        generic_data_mapping_to_csv_row(row_offset + 1, &header_names, entries, uri, diagnostics)
    }));
    GenericDataCsvRows {
        header_present: true,
        rows,
    }
}

fn generic_data_header_row_to_csv_row(
    header_names: &[(String, GenericDataSourceRangeAst)],
) -> Value {
    let source_range = header_names
        .first()
        .map(|(_, range)| range.clone())
        .unwrap_or_else(GenericDataSourceRangeAst::generated);
    csv_cemt_row(
        0,
        &source_range,
        header_names
            .iter()
            .enumerate()
            .map(|(index, (name, range))| csv_cemt_field(index, name, range))
            .collect(),
    )
}

fn generic_data_mapping_to_csv_row(
    row_index: usize,
    header_names: &[(String, GenericDataSourceRangeAst)],
    entries: &[GenericDataMappingEntryAst],
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Value {
    let source_range = entries
        .first()
        .map(|entry| entry.source_range.clone())
        .unwrap_or_else(GenericDataSourceRangeAst::generated);
    let fields = header_names
        .iter()
        .enumerate()
        .map(|(field_index, (header, _))| {
            let value = entries
                .iter()
                .find(|entry| {
                    generic_data_value_to_csv_scalar_text(&entry.key, uri, diagnostics) == *header
                })
                .map(|entry| {
                    (
                        generic_data_value_to_csv_scalar_text(&entry.value, uri, diagnostics),
                        entry.value.source_range().clone(),
                    )
                })
                .unwrap_or_else(|| ("".to_owned(), GenericDataSourceRangeAst::generated()));
            csv_cemt_field(field_index, &value.0, &value.1)
        })
        .collect();
    csv_cemt_row(row_index, &source_range, fields)
}

fn generic_data_value_to_csv_row(
    row_index: usize,
    value: &GenericDataValueAst,
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Value {
    match value {
        GenericDataValueAst::Sequence {
            source_range,
            items,
        } => csv_cemt_row(
            row_index,
            source_range,
            items
                .iter()
                .enumerate()
                .map(|(field_index, item)| {
                    let value = generic_data_value_to_csv_scalar_text(item, uri, diagnostics);
                    csv_cemt_field(field_index, &value, item.source_range())
                })
                .collect(),
        ),
        _ => generic_data_scalar_to_csv_row(row_index, value, uri, diagnostics),
    }
}

fn generic_data_scalar_to_csv_row(
    row_index: usize,
    value: &GenericDataValueAst,
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Value {
    let field_value = generic_data_value_to_csv_scalar_text(value, uri, diagnostics);
    csv_cemt_row(
        row_index,
        value.source_range(),
        vec![csv_cemt_field(0, &field_value, value.source_range())],
    )
}

fn generic_data_value_to_csv_scalar_text(
    value: &GenericDataValueAst,
    uri: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    match value {
        GenericDataValueAst::String { value, .. } => value.clone(),
        GenericDataValueAst::Number { lexeme, .. } => lexeme.clone(),
        GenericDataValueAst::Boolean { value, .. } => value.to_string(),
        GenericDataValueAst::Null { .. } => String::new(),
        GenericDataValueAst::Alias { alias, .. } => alias.clone().unwrap_or_default(),
        GenericDataValueAst::Mapping { source_range, .. }
        | GenericDataValueAst::Sequence { source_range, .. } => {
            diagnostics.push(generic_data_csv_unsupported_diagnostic(
                uri,
                source_range,
                "nested generic data values cannot be projected to a CSV field without an explicit flattening schema",
            ));
            String::new()
        }
    }
}

fn csv_cemt_row(
    index: usize,
    source_range: &GenericDataSourceRangeAst,
    fields: Vec<Value>,
) -> Value {
    json!({
        "index": index,
        "fieldCount": fields.len(),
        "byteOffset": source_range.byte_offset,
        "byteLength": source_range.byte_length,
        "sourceRange": source_range.to_cemt_subject(),
        "sourceMap": source_range.source_map_subject(),
        "fields": fields,
    })
}

fn csv_cemt_field(index: usize, value: &str, source_range: &GenericDataSourceRangeAst) -> Value {
    json!({
        "index": index,
        "value": value,
        "quoted": false,
        "byteOffset": source_range.byte_offset,
        "byteLength": source_range.byte_length,
        "sourceRange": source_range.to_cemt_subject(),
        "sourceMap": source_range.source_map_subject(),
    })
}

fn generic_data_csv_unsupported_diagnostic(
    uri: &str,
    source_range: &GenericDataSourceRangeAst,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        code: GENERIC_DATA_CSV_NESTED_VALUE_UNSUPPORTED_CODE.to_owned(),
        severity: Severity::Fatal,
        message: message.into(),
        details: Some(json!({
            "sourceRange": source_range.to_cemt_subject(),
            "sourceMap": source_range.source_map_subject(),
            "target": {
                "contentType": CSV_CONTENT_TYPE,
                "schema": CSV_SCHEMA_URI,
            },
        })),
        ..Diagnostic::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl CsvDocumentSource {
    fn from_request(
        request: CsvSourceValidationRequest<'_>,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        Self {
            uri: request.source_uri.to_owned(),
            content_type: request.content_type.unwrap_or(CSV_CONTENT_TYPE).to_owned(),
            media_type: request
                .content_type
                .map(csv_content_type_essence)
                .unwrap_or(CSV_CONTENT_TYPE.to_owned()),
            parameters,
            byte_length: request.bytes.len(),
        }
    }

    fn to_cemt_subject(&self) -> serde_json::Value {
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
pub struct CsvEncodingReportAst {
    pub declared_charset: Option<String>,
    pub normalized_charset: String,
    pub decoder_status: String,
    pub invalid_byte_offset: Option<u64>,
}

impl CsvEncodingReportAst {
    fn from_report(report: &CsvParseReport) -> Self {
        Self {
            declared_charset: report
                .content_type
                .as_deref()
                .and_then(|content_type| content_type_parameter(content_type, "charset")),
            normalized_charset: csv_normalized_charset_for_report(report).to_owned(),
            decoder_status: csv_decoder_status_for_report(report).to_owned(),
            invalid_byte_offset: report.facts.iter().find_map(|fact| match fact.kind {
                CsvParseFactKind::UnsupportedEncoding
                | CsvParseFactKind::DeclaredUsAsciiNonAsciiByte => fact.byte_offset,
                _ => None,
            }),
        }
    }

    fn to_cemt_subject(&self) -> serde_json::Value {
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
        serde_json::Value::Object(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDialectAst {
    pub delimiter: String,
    pub quote: String,
    pub escape: String,
    pub header: String,
    pub line_ending: Option<String>,
}

impl CsvDialectAst {
    fn new(header: impl Into<String>, line_ending: Option<&str>) -> Self {
        Self {
            delimiter: ",".to_owned(),
            quote: "\"".to_owned(),
            escape: "double-quote".to_owned(),
            header: header.into(),
            line_ending: line_ending.map(str::to_owned),
        }
    }

    fn to_cemt_subject(&self) -> serde_json::Value {
        let mut value = serde_json::Map::new();
        value.insert("delimiter".to_owned(), json!(self.delimiter));
        value.insert("quote".to_owned(), json!(self.quote));
        value.insert("escape".to_owned(), json!(self.escape));
        value.insert("header".to_owned(), json!(self.header));
        if let Some(line_ending) = self.line_ending.as_deref() {
            value.insert("lineEnding".to_owned(), json!(line_ending));
        }
        serde_json::Value::Object(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDocumentParseFact {
    pub kind: CsvParseFactKind,
    pub contract: Option<String>,
    pub behavior: Option<String>,
    pub diagnostic_code: Option<String>,
    pub diagnostic_severity: Option<String>,
    pub recoverable: bool,
    pub fatal: bool,
    pub parameter: Option<String>,
    pub actual: Option<String>,
    pub expected: Vec<String>,
    pub row_index: Option<usize>,
    pub field_index: Option<usize>,
    pub expected_count: Option<usize>,
    pub actual_count: Option<usize>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub message: String,
}

impl CsvDocumentParseFact {
    fn from_parse_fact(fact: &CsvParseFact, contracts: &CsvSchemaContractCatalog) -> Self {
        let binding = contracts.binding_for_fact(fact.kind);
        let fatal = binding.is_some_and(|binding| binding.severity.is_hard_violation());
        Self {
            kind: fact.kind,
            contract: binding.map(|binding| binding.contract.clone()),
            behavior: binding.and_then(|binding| binding.behavior.clone()),
            diagnostic_code: binding.map(|binding| binding.diagnostic_code.clone()),
            diagnostic_severity: binding
                .map(|binding| csv_severity_name(binding.severity).to_owned()),
            recoverable: !fatal,
            fatal,
            parameter: fact.parameter.clone(),
            actual: fact.actual.clone(),
            expected: fact.expected.clone(),
            row_index: fact.row_index,
            field_index: fact.field_index,
            expected_count: fact.expected_count,
            actual_count: fact.actual_count,
            line: fact.line,
            column: fact.column,
            byte_offset: fact.byte_offset,
            message: csv_fact_message(fact),
        }
    }

    fn to_cemt_subject(&self) -> serde_json::Value {
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
            "rowIndex": self.row_index,
            "fieldIndex": self.field_index,
            "expectedCount": self.expected_count,
            "actualCount": self.actual_count,
            "line": self.line,
            "column": self.column,
            "byteOffset": self.byte_offset,
            "message": self.message,
            "sourceRange": {
                "byteOffset": self.byte_offset,
                "line": self.line,
                "column": self.column,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRecordAst {
    pub index: usize,
    pub range: CsvSourceRange,
    pub record_ending: Option<CsvSourceRange>,
    pub fields: Vec<CsvFieldAst>,
}

impl CsvRecordAst {
    fn to_cemt_subject(&self) -> serde_json::Value {
        json!({
            "index": self.index,
            "fieldCount": self.fields.len(),
            "byteOffset": self.range.start.byte_offset,
            "byteLength": self.range.byte_length(),
            "sourceRange": self.range.to_cemt_subject(),
            "sourceMap": self.range.source_map(),
            "recordEndingSourceRange": self.record_ending.map(CsvSourceRange::to_cemt_subject),
            "recordEndingSourceMap": self.record_ending.map(CsvSourceRange::source_map),
            "fields": self.fields.iter().map(CsvFieldAst::to_cemt_subject).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvFieldAst {
    pub index: usize,
    pub value: String,
    pub quoted: bool,
    pub range: CsvSourceRange,
    pub delimiter_before: Option<CsvSourceRange>,
}

impl CsvFieldAst {
    fn to_cemt_subject(&self) -> serde_json::Value {
        json!({
            "index": self.index,
            "value": self.value,
            "quoted": self.quoted,
            "byteOffset": self.range.start.byte_offset,
            "byteLength": self.range.byte_length(),
            "sourceRange": self.range.to_cemt_subject(),
            "sourceMap": self.range.source_map(),
            "delimiterBeforeSourceRange": self.delimiter_before.map(CsvSourceRange::to_cemt_subject),
            "delimiterBeforeSourceMap": self.delimiter_before.map(CsvSourceRange::source_map),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvSourceRange {
    pub start: CsvSourcePosition,
    pub end: CsvSourcePosition,
}

impl CsvSourceRange {
    pub fn byte_length(self) -> u64 {
        self.end.byte_offset.saturating_sub(self.start.byte_offset)
    }

    fn byte_range(self) -> ByteRange {
        ByteRange::new(
            self.start.byte_offset,
            u32::try_from(self.byte_length()).unwrap_or(u32::MAX),
        )
    }

    pub fn source_map(self) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(self.byte_range()),
                transform: TransformKind::ContentTypeTransform {
                    content_type: CSV_CONTENT_TYPE.to_owned(),
                },
            }],
        }
    }

    fn to_cemt_subject(self) -> serde_json::Value {
        json!({
            "byteOffset": self.start.byte_offset,
            "byteLength": self.byte_length(),
            "line": self.start.line,
            "column": self.start.column,
            "endLine": self.end.line,
            "endColumn": self.end.column,
        })
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CsvDecodedSource {
    text: String,
    byte_offset_base: u64,
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
    pub fact_bindings: BTreeMap<String, CsvDiagnosticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDiagnosticBinding {
    pub fact_kind: String,
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
        let model = compile_schema_document_model(CSV_SCHEMA_URI, schema_source);
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
                    CsvDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: CsvParseFactKind) -> Option<&CsvDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_csv_source_bytes(request: CsvSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let contracts = CsvSchemaContractCatalog::from_builtin();
    let report = extract_csv_parse_report(request);
    validate_csv_parse_report(&report, &contracts)
}

pub fn csv_table_value_from_source_bytes(
    request: CsvSourceValidationRequest<'_>,
) -> (Option<serde_json::Value>, Vec<Diagnostic>) {
    let (table, diagnostics) = csv_document_ast_from_source_bytes(request);
    (table.map(|table| table.to_cemt_subject()), diagnostics)
}

pub fn csv_document_ast_from_source_bytes(
    request: CsvSourceValidationRequest<'_>,
) -> (Option<CsvDocumentAst>, Vec<Diagnostic>) {
    let report = extract_csv_parse_report(request);
    let contracts = CsvSchemaContractCatalog::from_builtin();
    let diagnostics = validate_csv_parse_report(&report, &contracts);
    let source = csv_decode_source_text(request, None).ok();
    let line_ending = source
        .as_ref()
        .and_then(|source| csv_detect_line_ending_style(&source.text));
    let header = csv_header_disposition(request.content_type);
    let rows = source
        .as_ref()
        .filter(|_| !csv_parse_report_has_encoding_blocker(&report))
        .map(csv_project_rows)
        .unwrap_or_default();
    let table = CsvDocumentAst {
        source: CsvDocumentSource::from_request(
            request,
            content_type_parameters(request.content_type),
        ),
        encoding: csv_table_encoding(&report).to_owned(),
        encoding_report: CsvEncodingReportAst::from_report(&report),
        delimiter: ",".to_owned(),
        header: header.to_owned(),
        dialect: CsvDialectAst::new(header, line_ending),
        parse_facts: csv_parse_facts_for_document(&report, &contracts),
        rows,
        line_ending: line_ending.map(str::to_owned),
    };

    (Some(table), diagnostics)
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

    let source = match csv_decode_source_text(request, charset.as_deref()) {
        Ok(source) => source,
        Err(fact) => {
            report.facts.push(fact);
            return report;
        }
    };

    collect_csv_quote_policy_facts(&source, &mut report.facts);
    collect_csv_record_facts(&source, &mut report.facts);
    report
}

fn csv_decode_source_text(
    request: CsvSourceValidationRequest<'_>,
    charset: Option<&str>,
) -> Result<CsvDecodedSource, CsvParseFact> {
    let mut decoder = Utf8Decoder::new(BytesSource::new(SourceId(1), request.bytes.to_vec()));
    let mut text = String::new();
    let mut first_scalar_byte_offset = None;

    while let Some(chunk) = decoder.decode_next() {
        for (scalar, range) in chunk.scalars {
            first_scalar_byte_offset.get_or_insert(range.start);
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
        let mut fact = csv_fact(
            CsvParseFactKind::UnsupportedEncoding,
            Some("charset"),
            charset.or(Some("utf-8")),
            &["valid UTF-8"],
        );
        fact.byte_offset = diagnostic.byte_offset;
        fact.message = Some(diagnostic.message.clone());
        return Err(fact);
    }

    let byte_offset_base = first_scalar_byte_offset
        .or_else(|| decoder.bom().map(|bom| bom.byte_range.end()))
        .unwrap_or(0);
    Ok(CsvDecodedSource {
        text,
        byte_offset_base,
    })
}

pub fn validate_csv_parse_report(
    report: &CsvParseReport,
    contracts: &CsvSchemaContractCatalog,
) -> Vec<Diagnostic> {
    report
        .facts
        .iter()
        .filter_map(|fact| {
            let binding = contracts.binding_for_fact(fact.kind)?;
            Some(csv_fact_diagnostic(
                report,
                fact,
                binding,
                csv_fact_message(fact),
            ))
        })
        .collect()
}

fn collect_csv_record_facts(source: &CsvDecodedSource, facts: &mut Vec<CsvParseFact>) {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(source.text.as_bytes());
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
                        fact.byte_offset =
                            csv_position_byte_offset(record.position(), source.byte_offset_base);
                        facts.push(fact);
                    }
                } else {
                    expected_field_count = Some(field_count);
                }
            }
            Err(error) => {
                let mut fact = csv_fact(csv_parse_error_fact_kind(&error), None, None, &[]);
                fact.line = csv_position_line(error.position());
                fact.byte_offset =
                    csv_position_byte_offset(error.position(), source.byte_offset_base);
                fact.message = Some(format!("CSV parse error: {error}"));
                facts.push(fact);
                break;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CsvQuoteState {
    StartField,
    InUnquoted,
    InQuoted { open: CsvSourcePosition },
    AfterQuote,
}

fn collect_csv_quote_policy_facts(source: &CsvDecodedSource, facts: &mut Vec<CsvParseFact>) {
    let bytes = source.text.as_bytes();
    let mut state = CsvQuoteState::StartField;
    let mut byte = 0usize;
    let mut line = 1u32;
    let mut column = 1u32;

    while byte < bytes.len() {
        let current = CsvSourcePosition {
            line,
            column,
            byte_offset: source.byte_offset_base + byte as u64,
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

fn csv_detect_line_ending_style(source: &str) -> Option<&'static str> {
    let bytes = source.as_bytes();
    let mut byte = 0usize;
    let mut saw_crlf = false;
    let mut saw_lf = false;
    let mut saw_cr = false;
    let mut state = CsvQuoteState::StartField;
    while byte < bytes.len() {
        match state {
            CsvQuoteState::StartField => match bytes[byte] {
                b'"' => {
                    state = CsvQuoteState::InQuoted {
                        open: CsvSourcePosition {
                            line: 0,
                            column: 0,
                            byte_offset: byte as u64,
                        },
                    };
                    byte += 1;
                }
                b',' => {
                    byte += 1;
                }
                b'\r' | b'\n' => {
                    csv_note_record_line_ending(
                        bytes,
                        &mut byte,
                        &mut saw_crlf,
                        &mut saw_lf,
                        &mut saw_cr,
                    );
                    state = CsvQuoteState::StartField;
                }
                _ => {
                    state = CsvQuoteState::InUnquoted;
                    byte += 1;
                }
            },
            CsvQuoteState::InUnquoted => match bytes[byte] {
                b',' => {
                    state = CsvQuoteState::StartField;
                    byte += 1;
                }
                b'\r' | b'\n' => {
                    csv_note_record_line_ending(
                        bytes,
                        &mut byte,
                        &mut saw_crlf,
                        &mut saw_lf,
                        &mut saw_cr,
                    );
                    state = CsvQuoteState::StartField;
                }
                _ => {
                    byte += 1;
                }
            },
            CsvQuoteState::InQuoted { .. } => {
                if bytes[byte] == b'"' {
                    if bytes.get(byte + 1) == Some(&b'"') {
                        byte += 2;
                    } else {
                        state = CsvQuoteState::AfterQuote;
                        byte += 1;
                    }
                } else {
                    byte += 1;
                }
            }
            CsvQuoteState::AfterQuote => match bytes[byte] {
                b',' => {
                    state = CsvQuoteState::StartField;
                    byte += 1;
                }
                b'\r' | b'\n' => {
                    csv_note_record_line_ending(
                        bytes,
                        &mut byte,
                        &mut saw_crlf,
                        &mut saw_lf,
                        &mut saw_cr,
                    );
                    state = CsvQuoteState::StartField;
                }
                _ => {
                    state = CsvQuoteState::InUnquoted;
                    byte += 1;
                }
            },
        }
    }

    csv_line_ending_style_from_flags(saw_crlf, saw_lf, saw_cr)
}

fn csv_note_record_line_ending(
    bytes: &[u8],
    byte: &mut usize,
    saw_crlf: &mut bool,
    saw_lf: &mut bool,
    saw_cr: &mut bool,
) {
    match bytes.get(*byte).copied() {
        Some(b'\r') if bytes.get(*byte + 1) == Some(&b'\n') => {
            *saw_crlf = true;
            *byte += 2;
        }
        Some(b'\r') => {
            *saw_cr = true;
            *byte += 1;
        }
        Some(b'\n') => {
            *saw_lf = true;
            *byte += 1;
        }
        Some(_) => {
            *byte += 1;
        }
        None => {}
    }
}

fn csv_line_ending_style_from_flags(
    saw_crlf: bool,
    saw_lf: bool,
    saw_cr: bool,
) -> Option<&'static str> {
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

fn csv_project_rows(source: &CsvDecodedSource) -> Vec<CsvRecordAst> {
    csv_scan_projected_records(source)
}

fn csv_scan_projected_records(source: &CsvDecodedSource) -> Vec<CsvRecordAst> {
    let text = source.text.as_str();
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut byte = 0usize;
    let mut line = 1u32;
    let mut column = 1u32;
    let mut row_index = 0usize;
    let mut row_start = csv_current_position(byte, line, column, source.byte_offset_base);
    let mut delimiter_before = None;

    loop {
        let mut field = csv_scan_projected_field(source, &mut byte, &mut line, &mut column);
        field.delimiter_before = delimiter_before.take();
        fields.push(field);

        if byte < bytes.len() && bytes[byte] == b',' {
            let delimiter_start = csv_current_position(byte, line, column, source.byte_offset_base);
            advance_csv_projection_cursor(text, &mut byte, &mut line, &mut column);
            let delimiter_end = csv_current_position(byte, line, column, source.byte_offset_base);
            delimiter_before = Some(CsvSourceRange {
                start: delimiter_start,
                end: delimiter_end,
            });
            continue;
        }

        let mut row_end = csv_current_position(byte, line, column, source.byte_offset_base);
        let mut record_ending = None;
        if byte < bytes.len() && matches!(bytes[byte], b'\r' | b'\n') {
            let record_ending_start =
                csv_current_position(byte, line, column, source.byte_offset_base);
            advance_csv_projection_cursor(text, &mut byte, &mut line, &mut column);
            row_end = csv_current_position(byte, line, column, source.byte_offset_base);
            record_ending = Some(CsvSourceRange {
                start: record_ending_start,
                end: row_end,
            });
        }

        for (field_index, field) in fields.iter_mut().enumerate() {
            field.index = field_index;
        }

        records.push(CsvRecordAst {
            index: row_index,
            range: CsvSourceRange {
                start: row_start,
                end: row_end,
            },
            record_ending,
            fields,
        });

        if byte >= bytes.len() {
            break;
        }

        row_index += 1;
        row_start = csv_current_position(byte, line, column, source.byte_offset_base);
        fields = Vec::new();
        delimiter_before = None;
    }

    records
}

fn csv_scan_projected_field(
    source: &CsvDecodedSource,
    byte: &mut usize,
    line: &mut u32,
    column: &mut u32,
) -> CsvFieldAst {
    let text = source.text.as_str();
    let bytes = text.as_bytes();
    let start = csv_current_position(*byte, *line, *column, source.byte_offset_base);
    let mut value = String::new();
    let mut quoted = false;

    if *byte < bytes.len() && bytes[*byte] == b'"' {
        quoted = true;
        advance_csv_projection_cursor(text, byte, line, column);
        while *byte < bytes.len() {
            if bytes[*byte] == b'"' {
                if bytes.get(*byte + 1) == Some(&b'"') {
                    value.push('"');
                    advance_csv_projection_cursor(text, byte, line, column);
                    advance_csv_projection_cursor(text, byte, line, column);
                    continue;
                }
                advance_csv_projection_cursor(text, byte, line, column);
                break;
            }

            let Some(ch) = text.get(*byte..).and_then(|tail| tail.chars().next()) else {
                break;
            };
            value.push(ch);
            advance_csv_projection_cursor(text, byte, line, column);
        }

        while *byte < bytes.len() && !matches!(bytes[*byte], b',' | b'\r' | b'\n') {
            advance_csv_projection_cursor(text, byte, line, column);
        }
    } else {
        while *byte < bytes.len() && !matches!(bytes[*byte], b',' | b'\r' | b'\n') {
            let Some(ch) = text.get(*byte..).and_then(|tail| tail.chars().next()) else {
                break;
            };
            value.push(ch);
            advance_csv_projection_cursor(text, byte, line, column);
        }
    }

    CsvFieldAst {
        index: 0,
        value,
        quoted,
        range: CsvSourceRange {
            start,
            end: csv_current_position(*byte, *line, *column, source.byte_offset_base),
        },
        delimiter_before: None,
    }
}

fn advance_csv_projection_cursor(source: &str, byte: &mut usize, line: &mut u32, column: &mut u32) {
    let bytes = source.as_bytes();
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
            let width = source
                .get(*byte..)
                .and_then(|tail| tail.chars().next())
                .map(char::len_utf8)
                .unwrap_or(1);
            *byte += width;
            *column = column.saturating_add(1);
        }
        None => {}
    }
}

fn csv_current_position(
    byte: usize,
    line: u32,
    column: u32,
    byte_offset_base: u64,
) -> CsvSourcePosition {
    CsvSourcePosition {
        line,
        column,
        byte_offset: byte_offset_base + byte as u64,
    }
}

fn csv_parse_facts_for_document(
    report: &CsvParseReport,
    contracts: &CsvSchemaContractCatalog,
) -> Vec<CsvDocumentParseFact> {
    report
        .facts
        .iter()
        .map(|fact| CsvDocumentParseFact::from_parse_fact(fact, contracts))
        .collect()
}

fn csv_parse_report_has_encoding_blocker(report: &CsvParseReport) -> bool {
    report.facts.iter().any(|fact| {
        matches!(
            fact.kind,
            CsvParseFactKind::UnsupportedCharset
                | CsvParseFactKind::UnsupportedEncoding
                | CsvParseFactKind::DeclaredUsAsciiNonAsciiByte
        )
    })
}

fn csv_table_encoding(report: &CsvParseReport) -> &'static str {
    csv_normalized_charset_for_report(report)
}

fn csv_normalized_charset_for_report(report: &CsvParseReport) -> &'static str {
    let charset = report
        .content_type
        .as_deref()
        .and_then(|content_type| content_type_parameter(content_type, "charset"));
    match charset.as_deref().map(csv_normalized_parameter).as_deref() {
        Some("us-ascii" | "ascii") => "us-ascii",
        Some("utf-8" | "utf8") | None => "utf-8",
        Some(_) => "other",
    }
}

fn csv_decoder_status_for_report(report: &CsvParseReport) -> &'static str {
    if report
        .facts
        .iter()
        .any(|fact| fact.kind == CsvParseFactKind::UnsupportedCharset)
    {
        "unsupported"
    } else if report.facts.iter().any(|fact| {
        matches!(
            fact.kind,
            CsvParseFactKind::UnsupportedEncoding | CsvParseFactKind::DeclaredUsAsciiNonAsciiByte
        )
    }) {
        "invalid"
    } else {
        "decoded"
    }
}

fn csv_header_disposition(content_type: Option<&str>) -> &'static str {
    content_type
        .and_then(|content_type| content_type_parameter(content_type, "header"))
        .as_deref()
        .map(csv_normalized_parameter)
        .and_then(|value| match value.as_str() {
            "present" => Some("present"),
            "absent" => Some("absent"),
            _ => None,
        })
        .unwrap_or("unknown")
}

fn csv_content_type_essence(content_type: &str) -> String {
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

fn csv_severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Fatal => "fatal",
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

fn csv_position_byte_offset(
    position: Option<&csv::Position>,
    byte_offset_base: u64,
) -> Option<u64> {
    position.map(|position| byte_offset_base + position.byte())
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn csv_table_projection_records_source_line_ending_style() {
        for (name, bytes, expected) in [
            ("lf", b"id,name\n1,Ada\n".as_slice(), Some("lf")),
            ("crlf", b"id,name\r\n1,Ada\r\n".as_slice(), Some("crlf")),
            (
                "quoted embedded lf with crlf records",
                b"id,note\r\n1,\"line one\nline two\"\r\n".as_slice(),
                Some("crlf"),
            ),
            ("cr", b"id,name\r1,Ada\r".as_slice(), Some("cr")),
            (
                "mixed",
                b"id,name\r\n1,Ada\n2,Lin\r".as_slice(),
                Some("mixed"),
            ),
            ("none", b"id,name".as_slice(), None),
        ] {
            let (table, diagnostics) =
                csv_table_value_from_source_bytes(CsvSourceValidationRequest {
                    bytes,
                    source_uri: "memory://table.csv",
                    content_type: Some("text/csv"),
                });

            assert!(
                diagnostics.is_empty(),
                "{name} should not produce diagnostics: {diagnostics:#?}"
            );
            assert_eq!(
                table
                    .as_ref()
                    .and_then(|table| table.get("lineEnding"))
                    .and_then(serde_json::Value::as_str),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn csv_table_projection_skips_utf8_bom_content_and_preserves_offsets() {
        let bytes = b"\xEF\xBB\xBFid,name,active\n1,Ada,true\n";
        let (table, diagnostics) = csv_table_value_from_source_bytes(CsvSourceValidationRequest {
            bytes,
            source_uri: "memory://bom.csv",
            content_type: Some("text/csv"),
        });

        assert!(
            diagnostics.is_empty(),
            "UTF-8 BOM should not produce CSV diagnostics: {diagnostics:#?}"
        );
        let table = table.expect("valid BOM CSV projects table data");
        assert_eq!(table["source"]["byteLength"], bytes.len());

        let rows = table["rows"].as_array().expect("rows array");
        assert_eq!(rows[0]["byteOffset"], 3);
        assert_eq!(rows[0]["sourceRange"]["byteOffset"], 3);
        assert_eq!(rows[0]["sourceRange"]["line"], 1);
        assert_eq!(rows[0]["sourceRange"]["column"], 1);
        assert_eq!(rows[0]["fields"][0]["value"], "id");
        assert_eq!(rows[0]["fields"][0]["byteOffset"], 3);
        assert_eq!(rows[0]["fields"][0]["byteLength"], 2);
        assert_eq!(rows[0]["fields"][0]["sourceRange"]["byteOffset"], 3);
        assert_eq!(
            rows[0]["fields"][0]["sourceMap"]["frames"][0]["source_id"],
            1
        );
        assert_eq!(
            rows[0]["fields"][0]["sourceMap"]["frames"][0]["span"]["ranges"]["start"],
            3
        );
        assert_eq!(
            rows[0]["fields"][0]["sourceMap"]["frames"][0]["span"]["ranges"]["len"],
            2
        );
        assert_eq!(rows[0]["fields"][1]["value"], "name");
        assert_eq!(rows[0]["fields"][1]["byteOffset"], 6);
        assert_eq!(
            rows[0]["fields"][1]["delimiterBeforeSourceRange"]["byteOffset"],
            5
        );
        assert_eq!(
            rows[0]["fields"][1]["delimiterBeforeSourceMap"]["frames"][0]["span"]["ranges"]
                ["start"],
            5
        );
        assert_eq!(
            rows[0]["recordEndingSourceMap"]["frames"][0]["span"]["ranges"]["start"],
            17
        );
    }

    #[test]
    fn csv_table_projection_exposes_schema_facing_parser_data() {
        let bytes = b"id,note\r\n1,\"Ada, Lovelace\"\r\n2,Lin\r\n";
        let (table, diagnostics) = csv_table_value_from_source_bytes(CsvSourceValidationRequest {
            bytes,
            source_uri: "memory://table.csv",
            content_type: Some("text/csv; charset=utf-8; header=present"),
        });

        assert!(
            diagnostics.is_empty(),
            "valid projection should not produce diagnostics: {diagnostics:#?}"
        );
        let table = table.expect("valid CSV projects to a schema-facing table");
        assert_eq!(table["kind"], "csv-table");
        assert_eq!(table["encoding"], "utf-8");
        assert_eq!(table["delimiter"], ",");
        assert_eq!(table["header"], "present");
        assert_eq!(table["lineEnding"], "crlf");
        assert_eq!(table["source"]["uri"], "memory://table.csv");
        assert_eq!(
            table["source"]["contentType"],
            "text/csv; charset=utf-8; header=present"
        );
        assert_eq!(table["source"]["byteLength"], bytes.len());
        assert_eq!(table["encodingReport"]["declaredCharset"], "utf-8");
        assert_eq!(table["encodingReport"]["normalizedCharset"], "utf-8");
        assert_eq!(table["encodingReport"]["decoderStatus"], "decoded");
        assert_eq!(table["dialect"]["delimiter"], ",");
        assert_eq!(table["dialect"]["quote"], "\"");
        assert_eq!(table["dialect"]["escape"], "double-quote");
        assert_eq!(table["dialect"]["header"], "present");
        assert_eq!(table["dialect"]["lineEnding"], "crlf");
        assert_eq!(table["parseFacts"].as_array().unwrap().len(), 0);

        let rows = table["rows"].as_array().expect("rows array");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["index"], 0);
        assert_eq!(rows[0]["fieldCount"], 2);
        assert_eq!(rows[0]["byteOffset"], 0);
        assert_eq!(rows[0]["byteLength"], 9);
        assert_eq!(rows[0]["sourceRange"]["line"], 1);
        assert_eq!(rows[0]["sourceRange"]["endLine"], 2);

        assert_eq!(rows[0]["fields"][0]["value"], "id");
        assert_eq!(rows[0]["fields"][0]["quoted"], false);
        assert_eq!(rows[0]["fields"][0]["byteOffset"], 0);
        assert_eq!(rows[0]["fields"][0]["byteLength"], 2);
        assert_eq!(rows[0]["fields"][1]["value"], "note");
        assert_eq!(rows[0]["fields"][1]["quoted"], false);
        assert_eq!(rows[0]["fields"][1]["byteOffset"], 3);
        assert_eq!(rows[0]["fields"][1]["byteLength"], 4);

        assert_eq!(rows[1]["index"], 1);
        assert_eq!(rows[1]["byteOffset"], 9);
        assert_eq!(rows[1]["byteLength"], 19);
        assert_eq!(rows[1]["fields"][1]["value"], "Ada, Lovelace");
        assert_eq!(rows[1]["fields"][1]["quoted"], true);
        assert_eq!(rows[1]["fields"][1]["byteOffset"], 11);
        assert_eq!(rows[1]["fields"][1]["byteLength"], 15);
    }

    #[test]
    fn csv_table_projection_carries_recoverable_parse_facts() {
        let bytes = b"id,name\n1,Ada\n2\n";
        let (table, diagnostics) = csv_table_value_from_source_bytes(CsvSourceValidationRequest {
            bytes,
            source_uri: "memory://table.csv",
            content_type: Some("text/csv; header=maybe"),
        });

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == INVALID_HEADER_PARAMETER_DIAGNOSTIC),
            "invalid header warning should still be emitted: {diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == INCONSISTENT_FIELD_COUNT_DIAGNOSTIC),
            "ragged row warning should still be emitted: {diagnostics:#?}"
        );
        let table = table.expect("warning-only CSV still projects table data");
        assert_eq!(table["header"], "unknown");
        assert_eq!(table["dialect"]["header"], "unknown");

        let facts = table["parseFacts"].as_array().expect("parse facts array");
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0]["kind"], "invalid-header-parameter");
        assert_eq!(facts[0]["contract"], HEADER_PARAMETER_VALUES_CONTRACT);
        assert_eq!(
            facts[0]["diagnosticCode"],
            INVALID_HEADER_PARAMETER_DIAGNOSTIC
        );
        assert_eq!(facts[0]["diagnosticSeverity"], "warning");
        assert_eq!(facts[0]["recoverable"], true);
        assert_eq!(facts[0]["fatal"], false);
        assert_eq!(facts[0]["parameter"], "header");
        assert_eq!(facts[0]["actual"], "maybe");
        assert_eq!(facts[1]["kind"], "ragged-row");
        assert_eq!(facts[1]["contract"], FIELD_COUNT_POLICY_CONTRACT);
        assert_eq!(facts[1]["rowIndex"], 3);
        assert_eq!(facts[1]["expectedCount"], 2);
        assert_eq!(facts[1]["actualCount"], 1);
        assert_eq!(facts[1]["recoverable"], true);
    }

    #[test]
    fn csv_table_projection_carries_fatal_parse_facts() {
        let bytes = b"id,name\n1,\"Ada\n";
        let (table, diagnostics) = csv_table_value_from_source_bytes(CsvSourceValidationRequest {
            bytes,
            source_uri: "memory://table.csv",
            content_type: Some("text/csv"),
        });

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == UNCLOSED_QUOTE_DIAGNOSTIC),
            "unclosed quote error should still be emitted: {diagnostics:#?}"
        );
        let table = table.expect("fatal parser facts still project neutral table data");
        let facts = table["parseFacts"].as_array().expect("parse facts array");
        let unclosed_quote = facts
            .iter()
            .find(|fact| fact["kind"] == "unclosed-quote")
            .expect("unclosed quote fact");

        assert_eq!(unclosed_quote["diagnosticSeverity"], "error");
        assert_eq!(unclosed_quote["recoverable"], false);
        assert_eq!(unclosed_quote["fatal"], true);
        assert_eq!(unclosed_quote["byteOffset"], 10);
        assert_eq!(unclosed_quote["sourceRange"]["line"], 2);
        assert_eq!(unclosed_quote["sourceRange"]["column"], 3);
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
    fn csv_fact_diagnostic_bindings_are_schema_declared_by_fact_kind() {
        let contracts = CsvSchemaContractCatalog::from_builtin();
        let cases = [
            (
                CsvParseFactKind::ParseError,
                CSV_SOURCE_PARSER_CONTRACT,
                PARSE_ERROR_DIAGNOSTIC,
                Severity::Error,
            ),
            (
                CsvParseFactKind::UnsupportedCharset,
                CHARSET_PARAMETER_SUPPORTED_CONTRACT,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
                Severity::Error,
            ),
            (
                CsvParseFactKind::UnsupportedEncoding,
                UTF8_DECODE_CONTRACT,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
                Severity::Error,
            ),
            (
                CsvParseFactKind::DeclaredUsAsciiNonAsciiByte,
                US_ASCII_BYTE_COMPATIBILITY_CONTRACT,
                UNSUPPORTED_ENCODING_DIAGNOSTIC,
                Severity::Error,
            ),
            (
                CsvParseFactKind::InvalidHeaderParameter,
                HEADER_PARAMETER_VALUES_CONTRACT,
                INVALID_HEADER_PARAMETER_DIAGNOSTIC,
                Severity::Warning,
            ),
            (
                CsvParseFactKind::InvalidQuoteEscape,
                QUOTE_ESCAPE_POLICY_CONTRACT,
                INVALID_QUOTE_ESCAPE_DIAGNOSTIC,
                Severity::Error,
            ),
            (
                CsvParseFactKind::UnclosedQuote,
                QUOTE_CLOSURE_POLICY_CONTRACT,
                UNCLOSED_QUOTE_DIAGNOSTIC,
                Severity::Error,
            ),
            (
                CsvParseFactKind::RaggedRow,
                FIELD_COUNT_POLICY_CONTRACT,
                INCONSISTENT_FIELD_COUNT_DIAGNOSTIC,
                Severity::Warning,
            ),
        ];

        for (fact_kind, contract, diagnostic_code, severity) in cases {
            let binding = contracts
                .binding_for_fact(fact_kind)
                .unwrap_or_else(|| panic!("schema binding for {}", fact_kind.as_str()));
            assert_eq!(binding.fact_kind, fact_kind.as_str());
            assert_eq!(binding.contract, contract);
            assert_eq!(binding.behavior.as_deref(), Some(CSV_PARSE_REPORT_BEHAVIOR));
            assert_eq!(binding.diagnostic_code, diagnostic_code);
            assert_eq!(binding.severity, severity);
        }
    }

    #[test]
    fn csv_header_diagnostic_is_schema_declared() {
        let source = crate::schema::package_sources::builtin_schema_package_source(CSV_PACKAGE_ID)
            .expect("CSV package source")
            .schema_source
            .replace(
                r#"{constraint @kind="header-parameter-values" @target="table" @diagnostic="cem.csv.invalid_header_parameter" @behavior="csv-parse-report-fact" @fact-kind="invalid-header-parameter" @policy="header parameter values must be present or absent; other values are reported as metadata drift"}"#,
                r#"{constraint @kind="header-parameter-values" @target="table" @diagnostic="example.csv.header_parameter" @behavior="csv-parse-report-fact" @fact-kind="invalid-header-parameter" @policy="header parameter values must be present or absent; other values are reported as metadata drift"}"#,
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

    #[test]
    fn csv_fact_kind_binding_is_schema_owned() {
        let source = crate::schema::package_sources::builtin_schema_package_source(CSV_PACKAGE_ID)
            .expect("CSV package source")
            .schema_source
            .replace(
                r#"@fact-kind="invalid-header-parameter""#,
                r#"@fact-kind="schema-ignored-header-parameter""#,
            );
        let contracts = CsvSchemaContractCatalog::from_schema_source(&source);
        assert!(contracts
            .binding_for_fact(CsvParseFactKind::InvalidHeaderParameter)
            .is_none());

        let report = extract_csv_parse_report(CsvSourceValidationRequest {
            bytes: b"id,name\n1,Ada\n",
            source_uri: "memory://table.csv",
            content_type: Some("text/csv; header=maybe"),
        });
        assert!(report
            .facts
            .iter()
            .any(|fact| fact.kind == CsvParseFactKind::InvalidHeaderParameter));

        let diagnostics = validate_csv_parse_report(&report, &contracts);
        assert!(
            diagnostics.is_empty(),
            "unbound parse facts stay neutral instead of falling back to Rust-owned diagnostics: {diagnostics:#?}"
        );
    }
}
