use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{XHTML_CONTENT_TYPE, XHTML_NAMESPACE_URI, XHTML_SCHEMA_URI};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::xml::{
    xml_document_ast_from_source_bytes, XmlDocumentAst, XmlEventAst, XmlEventKind,
    XmlParseFactKind, XmlSourceValidationRequest,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const XHTML_PACKAGE_ID: &str = "xhtml";
const XHTML_FACT_BEHAVIOR: &str = "xhtml-report-fact";

#[derive(Debug, Clone, Copy)]
pub struct XhtmlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XhtmlDocumentAst {
    pub source: XhtmlDocumentSource,
    pub xml_document: XmlDocumentAst,
    pub facts: Vec<XhtmlFact>,
    pub line_ending: Option<String>,
}

impl XhtmlDocumentAst {
    #[cfg(test)]
    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": "xhtml-document",
            "contentType": XHTML_CONTENT_TYPE,
            "schema": XHTML_SCHEMA_URI,
            "category": "xhtml-document",
            "source": self.source.to_cemt_subject(),
            "resourceKind": self.xml_document.resource_kind,
            "encodingReport": {
                "mimeCharset": self.xml_document.encoding_report.mime_charset,
                "declarationEncoding": self.xml_document.encoding_report.declaration_encoding,
                "normalizedEncoding": self.xml_document.encoding_report.normalized_encoding,
                "decoderStatus": self.xml_document.encoding_report.decoder_status,
            },
            "parseFacts": self
                .facts
                .iter()
                .map(XhtmlFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "events": self
                .xml_document
                .events
                .iter()
                .map(xhtml_event_to_cemt_subject)
                .collect::<Vec<_>>(),
            "lineEnding": self.line_ending,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XhtmlDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl XhtmlDocumentSource {
    fn from_xml(document: &XmlDocumentAst) -> Self {
        Self {
            uri: document.source.uri.clone(),
            content_type: document.source.content_type.clone(),
            media_type: document.source.media_type.clone(),
            parameters: document.source.parameters.clone(),
            byte_length: document.source.byte_length,
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
pub struct XhtmlSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XhtmlSourceRange {
    pub start: XhtmlSourcePosition,
    pub byte_length: u64,
}

impl XhtmlSourceRange {
    fn from_event(event: &XmlEventAst) -> Self {
        Self {
            start: XhtmlSourcePosition {
                line: event.source_range.start.line,
                column: event.source_range.start.column,
                byte_offset: event.source_range.start.byte_offset,
            },
            byte_length: event.source_range.byte_length,
        }
    }

    fn from_xml_fact(
        line: Option<u32>,
        column: Option<u32>,
        byte_offset: Option<u64>,
        byte_length: Option<u64>,
    ) -> Option<Self> {
        Some(Self {
            start: XhtmlSourcePosition {
                line: line.unwrap_or(1),
                column: column.unwrap_or(1),
                byte_offset: byte_offset?,
            },
            byte_length: byte_length.unwrap_or(1),
        })
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

    fn source_map(self) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    self.start.byte_offset,
                    u32::try_from(self.byte_length).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: XHTML_CONTENT_TYPE.to_owned(),
                },
            }],
        }
    }
}

fn xhtml_source_range_diagnostic_value(range: XhtmlSourceRange) -> Value {
    json!({
        "byteOffset": range.start.byte_offset,
        "byteLength": range.byte_length,
        "line": range.start.line,
        "column": range.start.column,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum XhtmlFactKind {
    NotWellFormedXml,
    UnsupportedEncoding,
    EncodingConflict,
    UnboundNamespacePrefix,
    DuplicateAttribute,
    DtdRejected,
    ExternalEntityRejected,
    EntityExpansionLimit,
    SourceMapUnavailable,
    RootNotHtml,
    NamespaceMissing,
    HeadBodyOrder,
    ProfileDeprecated,
    RootObserved,
    NamespaceObserved,
    HeadObserved,
    BodyObserved,
    DoctypeObserved,
    ForeignContentObserved,
}

impl XhtmlFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotWellFormedXml => "not-well-formed-xml",
            Self::UnsupportedEncoding => "unsupported-encoding",
            Self::EncodingConflict => "encoding-conflict",
            Self::UnboundNamespacePrefix => "unbound-namespace-prefix",
            Self::DuplicateAttribute => "duplicate-attribute",
            Self::DtdRejected => "dtd-rejected",
            Self::ExternalEntityRejected => "external-entity-rejected",
            Self::EntityExpansionLimit => "entity-expansion-limit",
            Self::SourceMapUnavailable => "source-map-unavailable",
            Self::RootNotHtml => "root-not-html",
            Self::NamespaceMissing => "namespace-missing",
            Self::HeadBodyOrder => "head-body-order",
            Self::ProfileDeprecated => "profile-deprecated",
            Self::RootObserved => "root-observed",
            Self::NamespaceObserved => "namespace-observed",
            Self::HeadObserved => "head-observed",
            Self::BodyObserved => "body-observed",
            Self::DoctypeObserved => "doctype-observed",
            Self::ForeignContentObserved => "foreign-content-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XhtmlFact {
    pub kind: XhtmlFactKind,
    pub source_range: Option<XhtmlSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

impl XhtmlFact {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "sourceRange": self.source_range.map(XhtmlSourceRange::to_cemt_subject),
            "message": self.message,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XhtmlDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XhtmlSchemaContractCatalog {
    fact_bindings: BTreeMap<String, XhtmlDiagnosticBinding>,
}

impl XhtmlSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<XhtmlSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(XHTML_PACKAGE_ID)
                .expect("built-in XHTML schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(XHTML_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != XHTML_FACT_BEHAVIOR {
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
                    XhtmlDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: XhtmlFactKind) -> Option<&XhtmlDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_xhtml_source_bytes(request: XhtmlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let (_, diagnostics) = xhtml_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn xhtml_document_ast_from_source_bytes(
    request: XhtmlSourceValidationRequest<'_>,
) -> (Option<XhtmlDocumentAst>, Vec<Diagnostic>) {
    let (xml_document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: request.bytes,
        source_uri: request.source_uri,
        content_type: request.content_type.or(Some(XHTML_CONTENT_TYPE)),
    });
    let Some(xml_document) = xml_document else {
        return (None, Vec::new());
    };
    let source = XhtmlDocumentSource::from_xml(&xml_document);
    let facts = xhtml_facts(&xml_document, &source);
    let diagnostics = xhtml_diagnostics(
        request.source_uri,
        &source.media_type,
        &facts,
        XhtmlSchemaContractCatalog::from_builtin(),
    );
    let line_ending = xml_document.line_ending.clone();
    (
        Some(XhtmlDocumentAst {
            source,
            xml_document,
            facts,
            line_ending,
        }),
        diagnostics,
    )
}

fn xhtml_facts(document: &XmlDocumentAst, source: &XhtmlDocumentSource) -> Vec<XhtmlFact> {
    let mut facts = document
        .parse_facts
        .iter()
        .map(|fact| XhtmlFact {
            kind: match fact.kind {
                XmlParseFactKind::ParseError => XhtmlFactKind::NotWellFormedXml,
                XmlParseFactKind::UnsupportedEncoding => XhtmlFactKind::UnsupportedEncoding,
                XmlParseFactKind::EncodingConflict => XhtmlFactKind::EncodingConflict,
                XmlParseFactKind::UnboundNamespacePrefix => XhtmlFactKind::UnboundNamespacePrefix,
                XmlParseFactKind::DuplicateAttribute => XhtmlFactKind::DuplicateAttribute,
                XmlParseFactKind::DtdRejected => XhtmlFactKind::DtdRejected,
                XmlParseFactKind::ExternalEntityRejected => XhtmlFactKind::ExternalEntityRejected,
                XmlParseFactKind::EntityExpansionLimit => XhtmlFactKind::EntityExpansionLimit,
                XmlParseFactKind::SourceMapUnavailable => XhtmlFactKind::SourceMapUnavailable,
            },
            source_range: XhtmlSourceRange::from_xml_fact(
                fact.line,
                fact.column,
                fact.byte_offset,
                fact.byte_length,
            ),
            message: fact.message.clone(),
            value: Some(fact.kind.as_str().to_owned()),
        })
        .collect::<Vec<_>>();

    if let Some(profile) = source.parameters.get("profile") {
        facts.push(XhtmlFact {
            kind: XhtmlFactKind::ProfileDeprecated,
            source_range: None,
            message: "application/xhtml+xml profile parameter is deprecated".to_owned(),
            value: Some(profile.clone()),
        });
    }

    let mut root_seen = false;
    let mut root_is_xhtml = false;
    let mut head_seen = false;
    let mut body_seen = false;
    let mut order_reported = false;
    let mut foreign_namespaces = BTreeSet::new();

    for event in &document.events {
        let range = Some(XhtmlSourceRange::from_event(event));
        if event.source_range.byte_length == 0 {
            facts.push(XhtmlFact {
                kind: XhtmlFactKind::SourceMapUnavailable,
                source_range: range,
                message: "XHTML event does not expose a non-empty source range".to_owned(),
                value: Some(event.index.to_string()),
            });
        }
        if event.kind == XmlEventKind::Doctype {
            facts.push(XhtmlFact {
                kind: XhtmlFactKind::DoctypeObserved,
                source_range: range,
                message: "XHTML doctype declaration was parsed and preserved".to_owned(),
                value: event.value.clone(),
            });
        }
        if !matches!(
            event.kind,
            XmlEventKind::StartElement | XmlEventKind::EmptyElement
        ) {
            if root_is_xhtml
                && event.depth == 1
                && matches!(event.kind, XmlEventKind::Text | XmlEventKind::Cdata)
                && !event.whitespace_only
            {
                push_head_body_order_fact(&mut facts, range, &mut order_reported);
            }
            continue;
        }

        let local_name = event.local_name.as_deref().unwrap_or_default();
        let namespace_uri = event.namespace_uri.as_deref().unwrap_or_default();
        if !root_seen && event.depth == 0 {
            root_seen = true;
            facts.push(XhtmlFact {
                kind: XhtmlFactKind::RootObserved,
                source_range: range,
                message: format!("XHTML root element `{local_name}` was parsed"),
                value: event.qualified_name.clone(),
            });
            if local_name != "html" {
                facts.push(XhtmlFact {
                    kind: XhtmlFactKind::RootNotHtml,
                    source_range: range,
                    message: format!(
                        "XHTML root element must be `html`, found `{}`",
                        event.qualified_name.as_deref().unwrap_or(local_name)
                    ),
                    value: event.qualified_name.clone(),
                });
            } else if namespace_uri != XHTML_NAMESPACE_URI {
                facts.push(XhtmlFact {
                    kind: XhtmlFactKind::NamespaceMissing,
                    source_range: range,
                    message: format!(
                        "XHTML root `html` element must use the `{XHTML_NAMESPACE_URI}` namespace"
                    ),
                    value: Some(namespace_uri.to_owned()),
                });
            } else {
                root_is_xhtml = true;
                facts.push(XhtmlFact {
                    kind: XhtmlFactKind::NamespaceObserved,
                    source_range: range,
                    message: "XHTML document namespace was parsed".to_owned(),
                    value: Some(XHTML_NAMESPACE_URI.to_owned()),
                });
            }
        } else if root_is_xhtml && event.depth == 1 {
            match (namespace_uri, local_name) {
                (XHTML_NAMESPACE_URI, "head") if !head_seen && !body_seen => {
                    head_seen = true;
                    facts.push(XhtmlFact {
                        kind: XhtmlFactKind::HeadObserved,
                        source_range: range,
                        message: "XHTML head element was parsed".to_owned(),
                        value: event.qualified_name.clone(),
                    });
                }
                (XHTML_NAMESPACE_URI, "body") if head_seen && !body_seen => {
                    body_seen = true;
                    facts.push(XhtmlFact {
                        kind: XhtmlFactKind::BodyObserved,
                        source_range: range,
                        message: "XHTML body element was parsed".to_owned(),
                        value: event.qualified_name.clone(),
                    });
                }
                _ => push_head_body_order_fact(&mut facts, range, &mut order_reported),
            }
        }

        if !namespace_uri.is_empty()
            && namespace_uri != XHTML_NAMESPACE_URI
            && foreign_namespaces.insert(namespace_uri.to_owned())
        {
            facts.push(XhtmlFact {
                kind: XhtmlFactKind::ForeignContentObserved,
                source_range: range,
                message: format!("XHTML foreign-content namespace `{namespace_uri}` was parsed"),
                value: Some(namespace_uri.to_owned()),
            });
        }
    }

    if root_is_xhtml && (!head_seen || !body_seen) {
        push_head_body_order_fact(
            &mut facts,
            document.events.first().map(XhtmlSourceRange::from_event),
            &mut order_reported,
        );
    }
    facts
}

fn push_head_body_order_fact(
    facts: &mut Vec<XhtmlFact>,
    source_range: Option<XhtmlSourceRange>,
    reported: &mut bool,
) {
    if *reported {
        return;
    }
    *reported = true;
    facts.push(XhtmlFact {
        kind: XhtmlFactKind::HeadBodyOrder,
        source_range,
        message:
            "XHTML html element must contain exactly one head element followed by one body element"
                .to_owned(),
        value: None,
    });
}

fn xhtml_diagnostics(
    source_uri: &str,
    content_type: &str,
    facts: &[XhtmlFact],
    contracts: &XhtmlSchemaContractCatalog,
) -> Vec<Diagnostic> {
    facts
        .iter()
        .filter_map(|fact| {
            let binding = contracts.binding_for_fact(fact.kind)?;
            Some(Diagnostic {
                uri: Some(source_uri.to_owned()),
                line: fact.source_range.map(|range| range.start.line),
                column: fact.source_range.map(|range| range.start.column),
                byte_offset: fact.source_range.map(|range| range.start.byte_offset),
                code: binding.diagnostic_code.clone(),
                severity: binding.severity,
                message: fact.message.clone(),
                details: Some(json!({
                    "xhtml": {
                        "phase": "xml-parse-and-xhtml-semantics",
                        "factKind": fact.kind.as_str(),
                        "contract": binding.contract,
                        "behavior": binding.behavior,
                        "policy": binding.policy,
                        "contentType": content_type,
                        "value": fact.value,
                        "sourceRange": fact.source_range.map(xhtml_source_range_diagnostic_value),
                    }
                })),
                source_map: fact.source_range.map(XhtmlSourceRange::source_map),
                ..Diagnostic::default()
            })
        })
        .collect()
}

#[cfg(test)]
fn xhtml_event_to_cemt_subject(event: &XmlEventAst) -> Value {
    let range = XhtmlSourceRange::from_event(event);
    json!({
        "index": event.index,
        "kind": event.kind.as_str(),
        "depth": event.depth,
        "qualifiedName": event.qualified_name,
        "localName": event.local_name,
        "prefix": event.prefix,
        "namespaceUri": event.namespace_uri,
        "attributes": event.attributes.iter().map(|attribute| json!({
            "qualifiedName": attribute.qualified_name,
            "localName": attribute.local_name,
            "prefix": attribute.prefix,
            "namespaceUri": attribute.namespace_uri,
            "value": attribute.value,
        })).collect::<Vec<_>>(),
        "value": event.value,
        "lexeme": event.lexeme,
        "whitespaceOnly": event.whitespace_only,
        "sourceRange": range.to_cemt_subject(),
        "sourceMap": range.source_map(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> (XhtmlDocumentAst, Vec<Diagnostic>) {
        let (document, diagnostics) =
            xhtml_document_ast_from_source_bytes(XhtmlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.xhtml",
                content_type: Some(XHTML_CONTENT_TYPE),
            });
        (document.expect("typed XHTML document"), diagnostics)
    }

    fn validate(source: &str) -> Vec<Diagnostic> {
        parse(source).1
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn xhtml_ast_reuses_xml_events_with_xhtml_identity_and_source_maps() {
        let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:svg="http://www.w3.org/2000/svg">
  <head><title>Basic</title></head>
  <body><svg:svg viewBox="0 0 1 1"><svg:path/></svg:svg></body>
</html>
"#;
        let (document, diagnostics) = parse(source);

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(document.source.media_type, XHTML_CONTENT_TYPE);
        assert_eq!(document.source.content_type, XHTML_CONTENT_TYPE);
        assert!(document
            .xml_document
            .events
            .iter()
            .all(|event| event.source_range.byte_length > 0));
        assert!(document.xml_document.events.iter().any(|event| {
            event.kind == XmlEventKind::Declaration && event.lexeme.starts_with("<?xml")
        }));
        assert!(document.xml_document.events.iter().any(|event| {
            event.qualified_name.as_deref() == Some("svg:path")
                && event.namespace_uri.as_deref() == Some("http://www.w3.org/2000/svg")
        }));
        assert!(document
            .facts
            .iter()
            .any(|fact| fact.kind == XhtmlFactKind::ForeignContentObserved));

        let subject = document.to_cemt_subject();
        assert_eq!(subject["kind"], json!("xhtml-document"));
        assert_eq!(subject["schema"], json!(XHTML_SCHEMA_URI));
        assert_eq!(
            subject["events"][0]["sourceMap"]["frames"][0]["transform"]["content_type"],
            json!(XHTML_CONTENT_TYPE)
        );
    }

    #[test]
    fn xhtml_source_validator_accepts_basic_document() {
        let diagnostics = validate(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Basic</title></head>
  <body><p>Hello.</p></body>
</html>
"#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn xhtml_source_validator_reports_schema_bound_structure_facts() {
        for (source, code) in [
            ("<html><head/><body/></html>", "cem.xhtml.namespace_missing"),
            (
                r#"<section xmlns="http://www.w3.org/1999/xhtml"><head/><body/></section>"#,
                "cem.xhtml.root_not_html",
            ),
            (
                r#"<html xmlns="http://www.w3.org/1999/xhtml"><body/><head/></html>"#,
                "cem.xhtml.head_body_order",
            ),
        ] {
            let diagnostics = validate(source);
            assert!(has_code(&diagnostics, code), "{diagnostics:?}");
            assert!(diagnostics.iter().all(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("xhtml"))
                    .and_then(|details| details.get("behavior"))
                    == Some(&json!(XHTML_FACT_BEHAVIOR))
            }));
        }
    }

    #[test]
    fn xhtml_source_validator_reports_not_well_formed_xml() {
        let diagnostics = validate(
            r#"<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Broken</title></head>
  <body><p>Missing close</body>
</html>
"#,
        );

        assert!(has_code(&diagnostics, "cem.xhtml.not_well_formed_xml"));
    }

    #[test]
    fn xhtml_source_validator_reports_profile_deprecated() {
        let (document, diagnostics) =
            xhtml_document_ast_from_source_bytes(XhtmlSourceValidationRequest {
                bytes: br#"<html xmlns="http://www.w3.org/1999/xhtml"><head/><body/></html>"#,
                source_uri: "fixture.xhtml",
                content_type: Some(
                    "application/xhtml+xml; profile=https://example.test/profile; charset=UTF-8",
                ),
            });

        assert!(has_code(&diagnostics, "cem.xhtml.profile_deprecated"));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
        let document = document.expect("typed XHTML document");
        assert_eq!(
            document
                .source
                .parameters
                .get("profile")
                .map(String::as_str),
            Some("https://example.test/profile")
        );
    }

    #[test]
    fn xhtml_inherits_xml_doctype_and_entity_safety_policy() {
        let diagnostics = validate(
            r#"<!DOCTYPE html [<!ENTITY remote SYSTEM "https://example.test/entity">]>
<html xmlns="http://www.w3.org/1999/xhtml"><head/><body>&remote;</body></html>"#,
        );

        assert!(has_code(&diagnostics, "cem.xhtml.dtd_rejected"));
        assert!(has_code(&diagnostics, "cem.xhtml.external_entity_rejected"));
    }
}
