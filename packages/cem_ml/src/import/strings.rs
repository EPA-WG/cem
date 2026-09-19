//! Bounded, already-decoded string import for native query functions.
use super::*;
use crate::{diagnostics::Severity, parser::tree::CemDocumentMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFailureKind {
    Malformed,
    DuplicateKey,
    Limit,
    Unsupported,
    Internal,
}

#[derive(Debug)]
pub struct ImportFailure {
    pub kind: ImportFailureKind,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}
impl ImportFailure {
    pub(super) fn new(kind: ImportFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            diagnostics: vec![],
        }
    }
}
impl std::fmt::Display for ImportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ImportFailure {}

#[derive(Debug, Clone, Copy)]
pub enum ImportStringProfile {
    Xml,
    JsonXml(JsonXmlProjectionOptions),
    Csv,
    Yaml,
}
pub struct ImportStringRequest<'a> {
    pub source: &'a str,
    /// Provenance is distinct from the parsed document's base/document URI.
    pub source_uri: &'a str,
    pub base_uri: Option<&'a str>,
    pub profile: ImportStringProfile,
}

fn parsed<T>((document, diagnostics): (Option<T>, Vec<Diagnostic>)) -> Result<T, ImportFailure> {
    if let Some(first) = diagnostics.iter().find(|d| d.severity.is_hard_violation()) {
        let kind = if first.code == "cem.json.resource_limit" {
            ImportFailureKind::Limit
        } else {
            ImportFailureKind::Malformed
        };
        return Err(ImportFailure {
            kind,
            message: first.message.clone(),
            diagnostics,
        });
    }
    document
        .ok_or_else(|| ImportFailure::new(ImportFailureKind::Malformed, "No document was produced"))
}

pub fn import_string(
    request: ImportStringRequest<'_>,
) -> Result<Arc<RetainedCemTree>, ImportFailure> {
    use ImportFailureKind::*;
    if request.source.len() > MAX_BYTES {
        return Err(ImportFailure::new(
            Limit,
            "Source exceeds the 32 KiB import limit.",
        ));
    }
    let bytes = request.source.as_bytes();
    let source_uri = request.source_uri;
    let native = Arc::new(match request.profile {
        ImportStringProfile::Xml => {
            let (document, diagnostics) =
                xml::xml_document_ast_from_string(xml::XmlSourceValidationRequest {
                    bytes,
                    source_uri,
                    content_type: Some("application/xml"),
                });
            if document.as_ref().is_some_and(|d| {
                d.parse_facts
                    .iter()
                    .any(|f| f.kind == xml::XmlParseFactKind::DtdRejected)
            }) {
                return Err(ImportFailure {
                    kind: Unsupported,
                    message: "DTD processing is not supported.".into(),
                    diagnostics,
                });
            }
            let doc = parsed((document, diagnostics))?;
            validate_xml_string(&doc, request.source)?;
            if doc.events.len() > MAX_VALUES
                || doc.events.iter().any(|e| {
                    e.depth > MAX_DEPTH
                        || (e.depth == MAX_DEPTH
                            && matches!(
                                e.kind,
                                xml::XmlEventKind::StartElement | xml::XmlEventKind::EmptyElement
                            ))
                })
            {
                return Err(ImportFailure::new(
                    Limit,
                    "XML exceeds the 64-level / 4096-event import limit.",
                ));
            }
            if doc
                .events
                .iter()
                .any(|e| e.kind == xml::XmlEventKind::Doctype)
            {
                return Err(ImportFailure::new(
                    Unsupported,
                    "DTD and unresolved entity references are not supported.",
                ));
            }
            LoadedInputAstStream::XmlDocument(doc)
        }
        ImportStringProfile::JsonXml(options) => {
            LoadedInputAstStream::JsonDocument(parsed(json::json_document_ast_for_xml(
                json::JsonSourceValidationRequest {
                    bytes,
                    source_uri,
                    content_type: Some("application/json"),
                },
                options.max_depth.min(MAX_DEPTH),
                options.max_values.min(MAX_VALUES),
            ))?)
        }
        ImportStringProfile::Csv => LoadedInputAstStream::CsvDocument(parsed(
            csv::csv_document_ast_from_source_bytes(csv::CsvSourceValidationRequest {
                bytes,
                source_uri,
                content_type: Some("text/csv"),
            }),
        )?),
        ImportStringProfile::Yaml => LoadedInputAstStream::YamlDocument(parsed(
            yaml::yaml_document_ast_from_source_bytes(yaml::YamlSourceValidationRequest {
                bytes,
                source_uri,
                content_type: Some("application/yaml"),
            }),
        )?),
    });
    validate_data_ast(&native)?;
    let (ast, mut semantics) = match native.as_ref() {
        LoadedInputAstStream::XmlDocument(doc) => {
            let mut imported = import_xml_ast(doc).map_err(|e| ImportFailure::new(Internal, e))?;
            xml_base_uris(&mut imported, doc, request.base_uri);
            (imported.ast, imported.semantics)
        }
        LoadedInputAstStream::JsonDocument(doc) => {
            let ImportStringProfile::JsonXml(mut options) = request.profile else {
                unreachable!()
            };
            options.max_depth = options.max_depth.min(MAX_DEPTH);
            options.max_values = options.max_values.min(MAX_VALUES);
            let ast = json_xml::project_json_to_xml(doc, &options).map_err(|e| ImportFailure {
                kind: match e.code {
                    "cem.json.xml_projection.duplicate_key" => DuplicateKey,
                    "cem.json.xml_projection.limit" => Limit,
                    _ => Malformed,
                },
                message: e.message.clone(),
                diagnostics: vec![Diagnostic {
                    uri: Some(source_uri.into()),
                    code: e.code.into(),
                    message: e.message,
                    severity: Severity::Error,
                    source_map: Some(e.source),
                    ..Default::default()
                }],
            })?;
            (ast, CemTreeSemantics::default())
        }
        owner => {
            let data = match owner {
                LoadedInputAstStream::CsvDocument(doc) => doc.to_generic_data_ast(),
                LoadedInputAstStream::YamlDocument(doc) => doc.to_generic_data_ast(),
                _ => unreachable!(),
            };
            let mut builder = ImportBuilder::new();
            builder
                .generic_data(&data)
                .map_err(|e| ImportFailure::new(Limit, e))?;
            (builder.ast, builder.semantics)
        }
    };
    semantics.document_metadata = Some(CemDocumentMetadata {
        base_uri: request.base_uri.map(str::to_owned),
        document_uri: None,
    });
    RetainedCemTree::new(ast, source_uri, request.source, semantics, Some(native))
        .map_err(|e| ImportFailure::new(Internal, e))
}

pub(super) fn xml_base_uris(
    imported: &mut XmlCemImport,
    doc: &xml::XmlDocumentAst,
    base: Option<&str>,
) {
    let mut stack = vec![base.map(str::to_owned)];
    imported.semantics.base_uris.clear();
    for (index, event) in doc.events.iter().enumerate() {
        if event.kind == xml::XmlEventKind::EndElement {
            stack.pop();
            continue;
        }
        let mut base = stack.last().cloned().flatten();
        if let Some(attribute) = event.attributes.iter().find(|a| {
            a.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
                && a.local_name == "base"
        }) {
            if let Some(value) = &attribute.entity_decoded_value {
                let value = value.trim();
                if !value.is_empty() {
                    base = Some(
                        url::Url::parse(value)
                            .or_else(|_| {
                                url::Url::parse(base.as_deref().unwrap_or(""))
                                    .and_then(|b| b.join(value))
                            })
                            .map(|u| u.to_string())
                            .unwrap_or_else(|_| value.to_owned()),
                    );
                }
                if let (Some(base), Some(id)) = (&base, imported.event_nodes[index]) {
                    imported.semantics.base_uris.insert(id, base.clone());
                }
            }
        }
        if event.kind == xml::XmlEventKind::StartElement {
            stack.push(base);
        }
    }
}

fn xml_char(c: char) -> bool {
    matches!(c, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}
fn ncname(name: &str) -> bool {
    let start = |c| matches!(c, 'A'..='Z' | '_' | 'a'..='z' | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}' | '\u{f8}'..='\u{2ff}' | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}' | '\u{200c}'..='\u{200d}' | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}' | '\u{3001}'..='\u{d7ff}' | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}' | '\u{10000}'..='\u{effff}');
    let mut chars = name.chars();
    chars.next().is_some_and(start) && chars.all(|c| start(c) || matches!(c, '-' | '.' | '0'..='9' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}'))
}
fn qname(name: &str) -> bool {
    let parts: Vec<_> = name.split(':').collect();
    parts.len() <= 2 && parts.into_iter().all(ncname)
}
fn validate_xml_string(doc: &xml::XmlDocumentAst, source: &str) -> Result<(), ImportFailure> {
    let invalid = |event: Option<&xml::XmlEventAst>, message: &str| ImportFailure {
        kind: ImportFailureKind::Malformed,
        message: message.into(),
        diagnostics: vec![Diagnostic {
            uri: Some(doc.source.uri.clone()),
            code: "cem.xml.parse_error".into(),
            message: message.into(),
            severity: Severity::Error,
            source_map: event.map(|e| e.source_range.source_map()),
            ..Default::default()
        }],
    };
    if !source.chars().all(xml_char) {
        return Err(invalid(None, "XML contains a character outside XML 1.0"));
    }
    for event in &doc.events {
        use xml::XmlEventKind::*;
        if matches!(event.kind, StartElement | EmptyElement | EndElement)
            && !event.qualified_name.as_deref().is_some_and(qname)
        {
            return Err(invalid(Some(event), "Invalid XML qualified name"));
        }
        if event.kind == EntityReference
            && !event
                .value
                .as_deref()
                .and_then(xml::xml_decode_entity_reference)
                .is_some_and(xml_char)
        {
            return Err(invalid(
                Some(event),
                "Invalid or undeclared XML entity reference",
            ));
        }
        if event.kind == Declaration && event.index != 0 {
            return Err(invalid(Some(event), "XML declaration must be first"));
        }
        if event.kind == Text && event.lexeme.contains("]]>") {
            return Err(invalid(
                Some(event),
                "CDATA end delimiter is not allowed in XML text",
            ));
        }
        for attribute in &event.attributes {
            if !qname(&attribute.qualified_name)
                || attribute.value.contains('<')
                || !attribute
                    .entity_decoded_value
                    .as_ref()
                    .is_some_and(|v| v.chars().all(xml_char))
            {
                return Err(invalid(Some(event), "Invalid XML attribute name or value"));
            }
        }
    }
    Ok(())
}
