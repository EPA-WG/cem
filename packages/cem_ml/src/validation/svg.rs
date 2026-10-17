use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{SVG_CONTENT_TYPE, SVG_NAMESPACE_URI, SVG_SCHEMA_URI};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::xml::{
    xml_document_ast_from_source_bytes, XmlAttributeAst, XmlDocumentAst, XmlEventAst, XmlEventKind,
    XmlParseFactKind, XmlSourceRange, XmlSourceValidationRequest,
};
#[cfg(test)]
use crate::validation::xml::{xml_event_markup_tokens, XmlMarkupTokenKind};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const SVG_PACKAGE_ID: &str = "svg";
const SVG_FACT_BEHAVIOR: &str = "svg-report-fact";

#[derive(Debug, Clone, Copy)]
pub struct SvgSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgDocumentAst {
    pub source: SvgDocumentSource,
    pub xml_document: XmlDocumentAst,
    pub facts: Vec<SvgFact>,
    pub line_ending: Option<String>,
}

impl SvgDocumentAst {
    #[cfg(test)]
    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": "svg-document",
            "contentType": SVG_CONTENT_TYPE,
            "schema": SVG_SCHEMA_URI,
            "category": "svg-document",
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
                .map(SvgFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "events": svg_events_to_cemt_subject(&self.xml_document.events),
            "lineEnding": self.line_ending,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl SvgDocumentSource {
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
pub struct SvgSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgSourceRange {
    pub start: SvgSourcePosition,
    pub byte_length: u64,
}

impl SvgSourceRange {
    fn from_event(event: &XmlEventAst) -> Self {
        Self::from_xml_range(event.source_range)
    }

    fn from_xml_range(range: XmlSourceRange) -> Self {
        Self {
            start: SvgSourcePosition {
                line: range.start.line,
                column: range.start.column,
                byte_offset: range.start.byte_offset,
            },
            byte_length: range.byte_length,
        }
    }

    fn from_xml_fact(
        line: Option<u32>,
        column: Option<u32>,
        byte_offset: Option<u64>,
        byte_length: Option<u64>,
    ) -> Option<Self> {
        Some(Self {
            start: SvgSourcePosition {
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
                    content_type: SVG_CONTENT_TYPE.to_owned(),
                },
            }],
        }
    }
}

fn svg_source_range_diagnostic_value(range: SvgSourceRange) -> Value {
    json!({
        "byteOffset": range.start.byte_offset,
        "byteLength": range.byte_length,
        "line": range.start.line,
        "column": range.start.column,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SvgFactKind {
    NotWellFormedXml,
    UnsupportedEncoding,
    EncodingConflict,
    UnboundNamespacePrefix,
    DuplicateAttribute,
    DtdRejected,
    ExternalEntityRejected,
    EntityExpansionLimit,
    SourceMapUnavailable,
    RootNotSvg,
    NamespaceMissing,
    ViewBoxInvalid,
    AccessibleNameMissing,
    ExternalResourceRejected,
    ScriptRejected,
    ForeignContentRejected,
    RootObserved,
    NamespaceObserved,
    ViewBoxObserved,
    TitleObserved,
    ReferenceObserved,
    DoctypeObserved,
    ForeignContentObserved,
}

impl SvgFactKind {
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
            Self::RootNotSvg => "root-not-svg",
            Self::NamespaceMissing => "namespace-missing",
            Self::ViewBoxInvalid => "view-box-invalid",
            Self::AccessibleNameMissing => "accessible-name-missing",
            Self::ExternalResourceRejected => "external-resource-rejected",
            Self::ScriptRejected => "script-rejected",
            Self::ForeignContentRejected => "foreign-content-rejected",
            Self::RootObserved => "root-observed",
            Self::NamespaceObserved => "namespace-observed",
            Self::ViewBoxObserved => "view-box-observed",
            Self::TitleObserved => "title-observed",
            Self::ReferenceObserved => "reference-observed",
            Self::DoctypeObserved => "doctype-observed",
            Self::ForeignContentObserved => "foreign-content-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgFact {
    pub kind: SvgFactKind,
    pub source_range: Option<SvgSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

impl SvgFact {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "sourceRange": self.source_range.map(SvgSourceRange::to_cemt_subject),
            "message": self.message,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SvgDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgSchemaContractCatalog {
    fact_bindings: BTreeMap<String, SvgDiagnosticBinding>,
}

impl SvgSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<SvgSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(SVG_PACKAGE_ID)
                .expect("built-in SVG schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(SVG_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != SVG_FACT_BEHAVIOR {
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
                    SvgDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: SvgFactKind) -> Option<&SvgDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_svg_source_bytes(request: SvgSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let (_, diagnostics) = svg_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn svg_document_ast_from_source_bytes(
    request: SvgSourceValidationRequest<'_>,
) -> (Option<SvgDocumentAst>, Vec<Diagnostic>) {
    let (xml_document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: request.bytes,
        source_uri: request.source_uri,
        content_type: request.content_type.or(Some(SVG_CONTENT_TYPE)),
    });
    let Some(xml_document) = xml_document else {
        return (None, Vec::new());
    };
    let source = SvgDocumentSource::from_xml(&xml_document);
    let facts = svg_facts(&xml_document);
    let diagnostics = svg_diagnostics(
        request.source_uri,
        &source.media_type,
        &facts,
        SvgSchemaContractCatalog::from_builtin(),
    );
    let line_ending = xml_document.line_ending.clone();
    (
        Some(SvgDocumentAst {
            source,
            xml_document,
            facts,
            line_ending,
        }),
        diagnostics,
    )
}

fn svg_facts(document: &XmlDocumentAst) -> Vec<SvgFact> {
    let mut facts = document
        .parse_facts
        .iter()
        .map(|fact| SvgFact {
            kind: match fact.kind {
                XmlParseFactKind::ParseError => SvgFactKind::NotWellFormedXml,
                XmlParseFactKind::UnsupportedEncoding => SvgFactKind::UnsupportedEncoding,
                XmlParseFactKind::EncodingConflict => SvgFactKind::EncodingConflict,
                XmlParseFactKind::UnboundNamespacePrefix => SvgFactKind::UnboundNamespacePrefix,
                XmlParseFactKind::DuplicateAttribute => SvgFactKind::DuplicateAttribute,
                XmlParseFactKind::DtdRejected => SvgFactKind::DtdRejected,
                XmlParseFactKind::ExternalEntityRejected => SvgFactKind::ExternalEntityRejected,
                XmlParseFactKind::EntityExpansionLimit => SvgFactKind::EntityExpansionLimit,
                XmlParseFactKind::SourceMapUnavailable => SvgFactKind::SourceMapUnavailable,
            },
            source_range: SvgSourceRange::from_xml_fact(
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
    let mut root_is_svg = false;
    let mut root_has_accessible_name = false;
    let mut root_accessibility_exempt = false;
    let mut external_resource_reported = false;
    let mut script_reported = false;
    let mut foreign_namespaces = BTreeSet::new();

    for event in &document.events {
        let range = Some(SvgSourceRange::from_event(event));
        if event.source_range.byte_length == 0 {
            facts.push(SvgFact {
                kind: SvgFactKind::SourceMapUnavailable,
                source_range: range,
                message: "SVG event does not expose a non-empty source range".to_owned(),
                value: Some(event.index.to_string()),
            });
        }
        if event.kind == XmlEventKind::Doctype {
            facts.push(SvgFact {
                kind: SvgFactKind::DoctypeObserved,
                source_range: range,
                message: "SVG doctype declaration was parsed and preserved".to_owned(),
                value: event.value.clone(),
            });
        }
        if !matches!(
            event.kind,
            XmlEventKind::StartElement | XmlEventKind::EmptyElement
        ) {
            continue;
        }

        let local_name = event.local_name.as_deref().unwrap_or_default();
        let namespace_uri = event.namespace_uri.as_deref().unwrap_or_default();
        if !root_seen && event.depth == 0 {
            root_seen = true;
            facts.push(SvgFact {
                kind: SvgFactKind::RootObserved,
                source_range: range,
                message: format!("SVG root element `{local_name}` was parsed"),
                value: event.qualified_name.clone(),
            });
            if local_name != "svg" {
                facts.push(SvgFact {
                    kind: SvgFactKind::RootNotSvg,
                    source_range: range,
                    message: format!(
                        "SVG root element must be `svg`, found `{}`",
                        event.qualified_name.as_deref().unwrap_or(local_name)
                    ),
                    value: event.qualified_name.clone(),
                });
            } else if namespace_uri != SVG_NAMESPACE_URI {
                facts.push(SvgFact {
                    kind: SvgFactKind::NamespaceMissing,
                    source_range: range,
                    message: format!(
                        "SVG root `svg` element must use the `{SVG_NAMESPACE_URI}` namespace"
                    ),
                    value: Some(namespace_uri.to_owned()),
                });
            } else {
                root_is_svg = true;
                root_has_accessible_name = svg_event_has_accessible_name_attribute(event);
                root_accessibility_exempt = svg_event_is_accessibility_exempt(event);
                facts.push(SvgFact {
                    kind: SvgFactKind::NamespaceObserved,
                    source_range: range,
                    message: "SVG document namespace was parsed".to_owned(),
                    value: Some(SVG_NAMESPACE_URI.to_owned()),
                });
                if let Some(view_box) = svg_attribute(event, "viewBox") {
                    facts.push(SvgFact {
                        kind: SvgFactKind::ViewBoxObserved,
                        source_range: range,
                        message: "SVG root viewBox was parsed".to_owned(),
                        value: Some(view_box.value.clone()),
                    });
                    if !svg_view_box_is_valid(&view_box.value) {
                        facts.push(SvgFact {
                            kind: SvgFactKind::ViewBoxInvalid,
                            source_range: range,
                            message: format!(
                                "SVG viewBox must contain four finite numbers with non-negative width and height, found `{}`",
                                view_box.value
                            ),
                            value: Some(view_box.value.clone()),
                        });
                    }
                }
            }
        } else if root_is_svg
            && event.depth == 1
            && namespace_uri == SVG_NAMESPACE_URI
            && matches!(local_name, "title" | "desc")
        {
            root_has_accessible_name = true;
            facts.push(SvgFact {
                kind: SvgFactKind::TitleObserved,
                source_range: range,
                message: format!("SVG root accessibility element `{local_name}` was parsed"),
                value: event.qualified_name.clone(),
            });
        }

        if namespace_uri == SVG_NAMESPACE_URI
            && (local_name == "script" || svg_event_has_script_handler(event))
            && !script_reported
        {
            script_reported = true;
            facts.push(SvgFact {
                kind: SvgFactKind::ScriptRejected,
                source_range: range,
                message: "SVG scripts and event-handler attributes are rejected unless an explicit execution policy is enabled".to_owned(),
                value: event.qualified_name.clone(),
            });
        }

        for attribute in &event.attributes {
            if svg_attribute_is_reference(attribute) {
                facts.push(SvgFact {
                    kind: SvgFactKind::ReferenceObserved,
                    source_range: range,
                    message: format!(
                        "SVG reference attribute `{}` was parsed",
                        attribute.qualified_name
                    ),
                    value: Some(attribute.value.clone()),
                });
            }
            if !external_resource_reported && svg_attribute_requires_resource_policy(attribute) {
                external_resource_reported = true;
                facts.push(SvgFact {
                    kind: SvgFactKind::ExternalResourceRejected,
                    source_range: range,
                    message: format!(
                        "SVG attribute `{}` references an external resource without an explicit resolver policy",
                        attribute.qualified_name
                    ),
                    value: Some(attribute.value.clone()),
                });
            }
        }

        if !namespace_uri.is_empty()
            && namespace_uri != SVG_NAMESPACE_URI
            && foreign_namespaces.insert(namespace_uri.to_owned())
        {
            facts.push(SvgFact {
                kind: SvgFactKind::ForeignContentObserved,
                source_range: range,
                message: format!("SVG foreign-content namespace `{namespace_uri}` was parsed"),
                value: Some(namespace_uri.to_owned()),
            });
            facts.push(SvgFact {
                kind: SvgFactKind::ForeignContentRejected,
                source_range: range,
                message: format!(
                    "SVG foreign-content namespace `{namespace_uri}` requires an explicit registered schema or converter policy"
                ),
                value: Some(namespace_uri.to_owned()),
            });
        }
    }

    if root_is_svg && !root_accessibility_exempt && !root_has_accessible_name {
        facts.push(SvgFact {
            kind: SvgFactKind::AccessibleNameMissing,
            source_range: document
                .events
                .iter()
                .find(|event| {
                    event.depth == 0
                        && matches!(
                            event.kind,
                            XmlEventKind::StartElement | XmlEventKind::EmptyElement
                        )
                })
                .map(SvgSourceRange::from_event),
            message: "Visible SVG root should provide title, desc, aria-label, or aria-labelledby"
                .to_owned(),
            value: None,
        });
    }
    facts
}

fn svg_attribute<'a>(event: &'a XmlEventAst, local_name: &str) -> Option<&'a XmlAttributeAst> {
    event
        .attributes
        .iter()
        .find(|attribute| attribute.local_name == local_name)
}

fn svg_event_has_accessible_name_attribute(event: &XmlEventAst) -> bool {
    event.attributes.iter().any(|attribute| {
        matches!(
            attribute.local_name.as_str(),
            "aria-label" | "aria-labelledby"
        ) && !attribute.value.trim().is_empty()
    })
}

fn svg_event_is_accessibility_exempt(event: &XmlEventAst) -> bool {
    event.attributes.iter().any(|attribute| {
        let value = attribute.value.trim().to_ascii_lowercase();
        (attribute.local_name == "aria-hidden" && value == "true")
            || (attribute.local_name == "role" && matches!(value.as_str(), "none" | "presentation"))
            || (attribute.local_name == "hidden" && attribute.namespace_uri.is_none())
    })
}

fn svg_event_has_script_handler(event: &XmlEventAst) -> bool {
    event.attributes.iter().any(|attribute| {
        attribute.namespace_uri.is_none()
            && attribute.local_name.len() > 2
            && attribute.local_name.to_ascii_lowercase().starts_with("on")
    })
}

fn svg_attribute_is_reference(attribute: &XmlAttributeAst) -> bool {
    matches!(attribute.local_name.as_str(), "href" | "src")
        || attribute.value.to_ascii_lowercase().contains("url(")
}

fn svg_attribute_requires_resource_policy(attribute: &XmlAttributeAst) -> bool {
    (matches!(attribute.local_name.as_str(), "href" | "src")
        && svg_direct_resource_reference_requires_policy(&attribute.value))
        || svg_css_url_reference_requires_policy(&attribute.value)
}

fn svg_direct_resource_reference_requires_policy(value: &str) -> bool {
    let unquoted = value.trim().trim_matches('"').trim_matches('\'');
    if unquoted.is_empty()
        || unquoted.starts_with('#')
        || unquoted.to_ascii_lowercase().starts_with("data:")
    {
        return false;
    }
    true
}

fn svg_css_url_reference_requires_policy(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut search_start = 0usize;
    while let Some(relative_url_start) = lower[search_start..].find("url(") {
        let url_start = search_start + relative_url_start;
        let after_url = &trimmed[url_start + 4..];
        let reference = after_url
            .split(')')
            .next()
            .unwrap_or(after_url)
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if !reference.starts_with('#') && !reference.to_ascii_lowercase().starts_with("data:") {
            return true;
        }
        search_start = url_start + 4;
    }
    false
}

fn svg_view_box_is_valid(value: &str) -> bool {
    let values = value
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(values) = values else {
        return false;
    };
    values.len() == 4
        && values.iter().all(|value| value.is_finite())
        && values[2] >= 0.0
        && values[3] >= 0.0
}

fn svg_diagnostics(
    source_uri: &str,
    content_type: &str,
    facts: &[SvgFact],
    contracts: &SvgSchemaContractCatalog,
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
                    "svg": {
                        "phase": "xml-parse-and-svg-semantics",
                        "factKind": fact.kind.as_str(),
                        "contract": binding.contract,
                        "behavior": binding.behavior,
                        "policy": binding.policy,
                        "contentType": content_type,
                        "value": fact.value,
                        "sourceRange": fact.source_range.map(svg_source_range_diagnostic_value),
                    }
                })),
                source_map: fact.source_range.map(SvgSourceRange::source_map),
                ..Diagnostic::default()
            })
        })
        .collect()
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Default)]
struct SvgEventLayout {
    layout_sensitive: bool,
    structural_whitespace: bool,
    line_break_before: bool,
}

#[cfg(test)]
#[derive(Debug)]
struct SvgSensitiveFrame {
    start: usize,
    sensitive: bool,
}

#[cfg(test)]
fn svg_events_to_cemt_subject(events: &[XmlEventAst]) -> Vec<Value> {
    let layout = svg_event_layout(events);
    events
        .iter()
        .zip(layout)
        .map(|(event, layout)| svg_event_to_cemt_subject(event, layout))
        .collect()
}

#[cfg(test)]
fn svg_event_layout(events: &[XmlEventAst]) -> Vec<SvgEventLayout> {
    let mut sensitive_scopes = vec![None; events.len()];
    let mut stack = Vec::<SvgSensitiveFrame>::new();
    let mut ranges = Vec::<(usize, usize, usize)>::new();
    let mut next_scope = 0usize;

    for (index, event) in events.iter().enumerate() {
        match event.kind {
            XmlEventKind::StartElement => {
                let inherited = stack.last().is_some_and(|frame| frame.sensitive);
                stack.push(SvgSensitiveFrame {
                    start: index,
                    sensitive: inherited || svg_element_requires_lexical_layout(event),
                });
            }
            XmlEventKind::EmptyElement => {
                if stack.last().is_some_and(|frame| frame.sensitive)
                    || svg_element_requires_lexical_layout(event)
                {
                    ranges.push((index, index, next_scope));
                    next_scope += 1;
                }
            }
            XmlEventKind::EndElement => {
                if let Some(frame) = stack.pop().filter(|frame| frame.sensitive) {
                    ranges.push((frame.start, index, next_scope));
                    next_scope += 1;
                }
            }
            XmlEventKind::Text => {
                if !event.whitespace_only {
                    if let Some(frame) = stack.last_mut() {
                        frame.sensitive = true;
                    } else {
                        ranges.push((index, index, next_scope));
                        next_scope += 1;
                    }
                }
            }
            XmlEventKind::Cdata | XmlEventKind::EntityReference => {
                if let Some(frame) = stack.last_mut() {
                    frame.sensitive = true;
                } else {
                    ranges.push((index, index, next_scope));
                    next_scope += 1;
                }
            }
            XmlEventKind::Declaration
            | XmlEventKind::Comment
            | XmlEventKind::ProcessingInstruction
            | XmlEventKind::Doctype => {}
        }
    }

    for (start, end, scope) in ranges {
        for event_scope in &mut sensitive_scopes[start..=end] {
            *event_scope = Some(scope);
        }
    }

    let mut previous_scope = None;
    let mut has_previous = false;
    events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let scope = sensitive_scopes[index];
            let structural_whitespace = matches!(event.kind, XmlEventKind::Text)
                && event.whitespace_only
                && scope.is_none();
            let line_break_before = !structural_whitespace
                && has_previous
                && !(scope.is_some() && scope == previous_scope);
            if !structural_whitespace {
                has_previous = true;
                previous_scope = scope;
            }
            SvgEventLayout {
                layout_sensitive: scope.is_some(),
                structural_whitespace,
                line_break_before,
            }
        })
        .collect()
}

#[cfg(test)]
fn svg_element_requires_lexical_layout(event: &XmlEventAst) -> bool {
    let local_name = event.local_name.as_deref().unwrap_or_default();
    matches!(
        local_name,
        "text" | "tspan" | "textPath" | "title" | "desc" | "style" | "script" | "foreignObject"
    ) || event
        .namespace_uri
        .as_deref()
        .is_some_and(|namespace| namespace != SVG_NAMESPACE_URI)
        || event.attributes.iter().any(|attribute| {
            attribute.qualified_name == "xml:space" && attribute.value == "preserve"
        })
}

#[cfg(test)]
fn svg_event_to_cemt_subject(event: &XmlEventAst, layout: SvgEventLayout) -> Value {
    let range = SvgSourceRange::from_event(event);
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
        "layoutSensitive": layout.layout_sensitive,
        "structuralWhitespace": layout.structural_whitespace,
        "lineBreakBefore": layout.line_break_before,
        "markupTokens": svg_markup_tokens(event),
        "sourceRange": range.to_cemt_subject(),
        "sourceMap": range.source_map(),
    })
}

#[cfg(test)]
fn svg_markup_tokens(event: &XmlEventAst) -> Vec<Value> {
    xml_event_markup_tokens(event)
        .into_iter()
        .map(|token| {
            let range = SvgSourceRange::from_xml_range(token.source_range);
            json!({
                "kind": token.kind.as_str(),
                "text": token.text,
                "role": svg_markup_token_role(token.kind),
                "sourceRange": range.to_cemt_subject(),
                "sourceMap": range.source_map(),
            })
        })
        .collect()
}

#[cfg(test)]
fn svg_markup_token_role(kind: XmlMarkupTokenKind) -> &'static str {
    match kind {
        XmlMarkupTokenKind::Delimiter | XmlMarkupTokenKind::Equals => "syntax.punctuation",
        XmlMarkupTokenKind::ElementName => "syntax.name",
        XmlMarkupTokenKind::AttributeName => "syntax.attribute",
        XmlMarkupTokenKind::AttributeValue => "syntax.string",
        XmlMarkupTokenKind::Whitespace | XmlMarkupTokenKind::Raw => "syntax.raw",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> (SvgDocumentAst, Vec<Diagnostic>) {
        let (document, diagnostics) =
            svg_document_ast_from_source_bytes(SvgSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.svg",
                content_type: Some(SVG_CONTENT_TYPE),
            });
        (document.expect("typed SVG document"), diagnostics)
    }

    fn validate(source: &str) -> Vec<Diagnostic> {
        parse(source).1
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn svg_ast_reuses_xml_events_with_svg_identity_xlink_and_source_maps() {
        let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24">
  <title>Download</title>
  <use xlink:href="#download"/>
</svg>
"##;
        let (document, diagnostics) = parse(source);

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(document.source.media_type, SVG_CONTENT_TYPE);
        assert!(document
            .xml_document
            .events
            .iter()
            .all(|event| event.source_range.byte_length > 0));
        assert!(document.xml_document.events.iter().any(|event| {
            event.attributes.iter().any(|attribute| {
                attribute.qualified_name == "xlink:href"
                    && attribute.namespace_uri.as_deref() == Some("http://www.w3.org/1999/xlink")
            })
        }));
        let subject = document.to_cemt_subject();
        assert_eq!(subject["kind"], json!("svg-document"));
        assert_eq!(subject["schema"], json!(SVG_SCHEMA_URI));
        assert_eq!(
            subject["events"][0]["sourceMap"]["frames"][0]["transform"]["content_type"],
            json!(SVG_CONTENT_TYPE)
        );
    }

    #[test]
    fn svg_cemt_subject_marks_safe_layout_boundaries_and_tokenizes_markup() {
        let source = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <text data-kind='label'>Keep <tspan> this </tspan></text>
  <script><![CDATA[const marker = "<g/>";]]></script>
  <foreignObject><html:div xmlns:html="http://www.w3.org/1999/xhtml">Keep <html:span> this </html:span></html:div></foreignObject>
</svg>
"##;
        let (document, diagnostics) = parse(source);

        assert!(has_code(&diagnostics, "cem.svg.script_rejected"));
        assert!(has_code(&diagnostics, "cem.svg.foreign_content_rejected"));
        let subject = document.to_cemt_subject();
        let events = subject["events"].as_array().expect("SVG CEMT events");
        let root = events
            .iter()
            .find(|event| event["kind"] == "start-element" && event["qualifiedName"] == "svg")
            .expect("SVG root event");
        assert_eq!(root["lineBreakBefore"], true);
        assert_eq!(root["layoutSensitive"], false);
        assert_eq!(
            root["markupTokens"]
                .as_array()
                .expect("root markup tokens")
                .iter()
                .map(|token| (
                    token["kind"].as_str().unwrap(),
                    token["text"].as_str().unwrap()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("delimiter", "<"),
                ("element-name", "svg"),
                ("whitespace", " "),
                ("attribute-name", "xmlns"),
                ("equals", "="),
                ("attribute-value", "\"http://www.w3.org/2000/svg\""),
                ("delimiter", ">"),
            ]
        );
        assert!(root["markupTokens"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|token| token["kind"] != "whitespace")
            .all(|token| token["sourceMap"] != Value::Null));

        let text = events
            .iter()
            .find(|event| event["kind"] == "start-element" && event["qualifiedName"] == "text")
            .expect("text event");
        assert_eq!(text["layoutSensitive"], true);
        assert_eq!(text["lineBreakBefore"], true);
        assert_eq!(
            text["markupTokens"]
                .as_array()
                .unwrap()
                .iter()
                .find(|token| token["kind"] == "attribute-value")
                .unwrap()["text"],
            "'label'"
        );
        assert!(events.iter().any(|event| {
            event["kind"] == "text"
                && event["lexeme"] == " this "
                && event["layoutSensitive"] == true
                && event["structuralWhitespace"] == false
        }));
        assert!(events.iter().any(|event| {
            event["kind"] == "cdata"
                && event["layoutSensitive"] == true
                && event["lineBreakBefore"] == false
        }));
        assert!(events.iter().any(|event| {
            event["qualifiedName"] == "html:div" && event["layoutSensitive"] == true
        }));
        assert!(events.iter().any(|event| {
            event["kind"] == "text"
                && event["whitespaceOnly"] == true
                && event["structuralWhitespace"] == true
        }));
    }

    #[test]
    fn svg_source_validator_accepts_basic_icon() {
        let diagnostics = validate(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img" viewBox="0 0 24 24">
  <title>Download</title>
  <path d="M12 3v12"/>
</svg>
"#,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn svg_source_validator_reports_schema_bound_policy_facts() {
        for (source, code) in [
            (
                r#"<svg role="img"><title>Missing namespace</title></svg>"#,
                "cem.svg.namespace_missing",
            ),
            (
                r#"<section xmlns="http://www.w3.org/2000/svg"><title>Wrong root</title></section>"#,
                "cem.svg.root_not_svg",
            ),
            (
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 -1 24"><title>Bad viewport</title></svg>"#,
                "cem.svg.view_box_invalid",
            ),
            (
                r#"<svg xmlns="http://www.w3.org/2000/svg"><title>Scripted</title><script>alert(1)</script></svg>"#,
                "cem.svg.script_rejected",
            ),
            (
                r#"<svg xmlns="http://www.w3.org/2000/svg"><title>External</title><image href="https://example.test/logo.png"/></svg>"#,
                "cem.svg.external_resource_rejected",
            ),
            (
                r#"<svg xmlns="http://www.w3.org/2000/svg"><title>Foreign</title><foreignObject><p xmlns="http://www.w3.org/1999/xhtml">Text</p></foreignObject></svg>"#,
                "cem.svg.foreign_content_rejected",
            ),
        ] {
            let diagnostics = validate(source);
            assert!(has_code(&diagnostics, code), "{diagnostics:?}");
            assert!(diagnostics.iter().all(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("svg"))
                    .and_then(|details| details.get("behavior"))
                    == Some(&json!(SVG_FACT_BEHAVIOR))
            }));
        }
    }

    #[test]
    fn svg_source_validator_reports_accessible_name_missing_warning() {
        let diagnostics = validate(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 3v18"/></svg>"#,
        );
        assert!(has_code(&diagnostics, "cem.svg.accessible_name_missing"));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }

    #[test]
    fn svg_source_validator_reports_not_well_formed_xml() {
        let diagnostics = validate(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img"><title>Broken</title><path></svg>"#,
        );
        assert!(has_code(&diagnostics, "cem.svg.not_well_formed_xml"));
    }

    #[test]
    fn svg_inherits_xml_doctype_and_entity_safety_policy() {
        let diagnostics = validate(
            r#"<!DOCTYPE svg [<!ENTITY remote SYSTEM "https://example.test/entity">]>
<svg xmlns="http://www.w3.org/2000/svg"><title>&remote;</title></svg>"#,
        );
        assert!(has_code(&diagnostics, "cem.svg.dtd_rejected"));
        assert!(has_code(&diagnostics, "cem.svg.external_entity_rejected"));
    }

    #[test]
    fn svg_preserves_mime_parameters_and_rejects_event_handlers() {
        let (document, diagnostics) = svg_document_ast_from_source_bytes(
            SvgSourceValidationRequest {
                bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" onload="run()"/>"#,
                source_uri: "fixture.svg",
                content_type: Some("image/svg+xml; charset=UTF-8"),
            },
        );
        assert!(has_code(&diagnostics, "cem.svg.script_rejected"));
        assert_eq!(
            document
                .expect("typed SVG document")
                .source
                .parameters
                .get("charset")
                .map(String::as_str),
            Some("UTF-8")
        );
    }
}
