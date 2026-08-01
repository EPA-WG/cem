use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{MATHML_CONTENT_TYPE, MATHML_NAMESPACE_URI, MATHML_SCHEMA_URI};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::xml::{
    xml_document_ast_from_source_bytes, XmlAttributeAst, XmlDocumentAst, XmlEventAst, XmlEventKind,
    XmlParseFactKind, XmlSourceRange, XmlSourceValidationRequest,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const MATHML_PACKAGE_ID: &str = "mathml";
const MATHML_FACT_BEHAVIOR: &str = "mathml-report-fact";
pub const MATHML_PRESENTATION_CONTENT_TYPE: &str = "application/mathml-presentation+xml";
pub const MATHML_CONTENT_CONTENT_TYPE: &str = "application/mathml-content+xml";

#[derive(Debug, Clone, Copy)]
pub struct MathMlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathMlMediaProfile {
    Generic,
    Presentation,
    Content,
}

impl MathMlMediaProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Presentation => "presentation",
            Self::Content => "content",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathMlDocumentAst {
    pub xml_document: XmlDocumentAst,
    pub media_profile: MathMlMediaProfile,
    pub facts: Vec<MathMlFact>,
    pub line_ending: Option<String>,
}

impl MathMlDocumentAst {
    pub fn to_cemt_subject(&self) -> Value {
        let source = &self.xml_document.source;
        let encoding = &self.xml_document.encoding_report;
        json!({
            "kind": "mathml-document",
            "contentType": source.media_type,
            "schema": MATHML_SCHEMA_URI,
            "category": "mathml-document",
            "mediaProfile": self.media_profile.as_str(),
            "source": {
                "uri": source.uri,
                "contentType": source.content_type,
                "mediaType": source.media_type,
                "parameters": source.parameters,
                "byteLength": source.byte_length,
            },
            "resourceKind": self.xml_document.resource_kind,
            "encodingReport": {
                "mimeCharset": encoding.mime_charset,
                "declarationEncoding": encoding.declaration_encoding,
                "normalizedEncoding": encoding.normalized_encoding,
                "decoderStatus": encoding.decoder_status,
            },
            "parseFacts": self
                .facts
                .iter()
                .map(|fact| fact.to_cemt_subject())
                .collect::<Vec<_>>(),
            "events": self
                .xml_document
                .events
                .iter()
                .map(|event| mathml_event_to_cemt_subject(event, &source.media_type))
                .collect::<Vec<_>>(),
            "lineEnding": self.line_ending,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MathMlFactKind {
    NotWellFormedXml,
    UnsupportedEncoding,
    EncodingConflict,
    UnboundNamespacePrefix,
    DuplicateAttribute,
    DtdRejected,
    ExternalEntityRejected,
    EntityExpansionLimit,
    SourceMapUnavailable,
    RootNotMath,
    NamespaceMissing,
    UnsupportedProfile,
    MalformedExpression,
    ExternalAnnotationRejected,
    ForeignContentRejected,
    RootObserved,
    NamespaceObserved,
    ProfileObserved,
    PresentationObserved,
    ContentObserved,
    SemanticsObserved,
    AnnotationObserved,
    AccessibilityTextObserved,
    DoctypeObserved,
    ForeignContentObserved,
}

impl MathMlFactKind {
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
            Self::RootNotMath => "root-not-math",
            Self::NamespaceMissing => "namespace-missing",
            Self::UnsupportedProfile => "unsupported-profile",
            Self::MalformedExpression => "malformed-expression",
            Self::ExternalAnnotationRejected => "external-annotation-rejected",
            Self::ForeignContentRejected => "foreign-content-rejected",
            Self::RootObserved => "root-observed",
            Self::NamespaceObserved => "namespace-observed",
            Self::ProfileObserved => "profile-observed",
            Self::PresentationObserved => "presentation-observed",
            Self::ContentObserved => "content-observed",
            Self::SemanticsObserved => "semantics-observed",
            Self::AnnotationObserved => "annotation-observed",
            Self::AccessibilityTextObserved => "accessibility-text-observed",
            Self::DoctypeObserved => "doctype-observed",
            Self::ForeignContentObserved => "foreign-content-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathMlFact {
    pub kind: MathMlFactKind,
    pub source_range: Option<XmlSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

impl MathMlFact {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "sourceRange": self.source_range.map(mathml_source_range_to_value),
            "message": self.message,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MathMlDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathMlSchemaContractCatalog {
    fact_bindings: BTreeMap<String, MathMlDiagnosticBinding>,
}

impl MathMlSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<MathMlSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(MATHML_PACKAGE_ID)
                .expect("built-in MathML schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(MATHML_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != MATHML_FACT_BEHAVIOR {
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
                    MathMlDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: MathMlFactKind) -> Option<&MathMlDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_mathml_source_bytes(request: MathMlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let (_, diagnostics) = mathml_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn mathml_document_ast_from_source_bytes(
    request: MathMlSourceValidationRequest<'_>,
) -> (Option<MathMlDocumentAst>, Vec<Diagnostic>) {
    let (xml_document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: request.bytes,
        source_uri: request.source_uri,
        content_type: request.content_type.or(Some(MATHML_CONTENT_TYPE)),
    });
    let Some(xml_document) = xml_document else {
        return (None, Vec::new());
    };
    let (media_profile, unsupported_profile) = mathml_media_profile(&xml_document);
    let facts = mathml_facts(&xml_document, media_profile, unsupported_profile);
    let diagnostics = mathml_diagnostics(
        request.source_uri,
        &xml_document.source.media_type,
        &facts,
        MathMlSchemaContractCatalog::from_builtin(),
    );
    let line_ending = xml_document.line_ending.clone();
    (
        Some(MathMlDocumentAst {
            xml_document,
            media_profile,
            facts,
            line_ending,
        }),
        diagnostics,
    )
}

fn mathml_media_profile(document: &XmlDocumentAst) -> (MathMlMediaProfile, Option<String>) {
    let mut profile = match document.source.media_type.as_str() {
        MATHML_PRESENTATION_CONTENT_TYPE => MathMlMediaProfile::Presentation,
        MATHML_CONTENT_CONTENT_TYPE => MathMlMediaProfile::Content,
        _ => MathMlMediaProfile::Generic,
    };
    let Some(parameter) = document.source.parameters.get("profile") else {
        return (profile, None);
    };
    match parameter.trim().to_ascii_lowercase().as_str() {
        "generic" => profile = MathMlMediaProfile::Generic,
        "presentation" => profile = MathMlMediaProfile::Presentation,
        "content" => profile = MathMlMediaProfile::Content,
        _ => return (profile, Some(parameter.clone())),
    }
    (profile, None)
}

fn mathml_facts(
    document: &XmlDocumentAst,
    profile: MathMlMediaProfile,
    unsupported_profile: Option<String>,
) -> Vec<MathMlFact> {
    let mut facts = document
        .parse_facts
        .iter()
        .map(|fact| MathMlFact {
            kind: match fact.kind {
                XmlParseFactKind::ParseError => MathMlFactKind::NotWellFormedXml,
                XmlParseFactKind::UnsupportedEncoding => MathMlFactKind::UnsupportedEncoding,
                XmlParseFactKind::EncodingConflict => MathMlFactKind::EncodingConflict,
                XmlParseFactKind::UnboundNamespacePrefix => MathMlFactKind::UnboundNamespacePrefix,
                XmlParseFactKind::DuplicateAttribute => MathMlFactKind::DuplicateAttribute,
                XmlParseFactKind::DtdRejected => MathMlFactKind::DtdRejected,
                XmlParseFactKind::ExternalEntityRejected => MathMlFactKind::ExternalEntityRejected,
                XmlParseFactKind::EntityExpansionLimit => MathMlFactKind::EntityExpansionLimit,
                XmlParseFactKind::SourceMapUnavailable => MathMlFactKind::SourceMapUnavailable,
            },
            source_range: mathml_source_range_from_xml_fact(
                fact.line,
                fact.column,
                fact.byte_offset,
                fact.byte_length,
            ),
            message: fact.message.clone(),
            value: Some(fact.kind.as_str().to_owned()),
        })
        .collect::<Vec<_>>();

    facts.push(MathMlFact {
        kind: MathMlFactKind::ProfileObserved,
        source_range: None,
        message: format!("MathML `{}` media profile was selected", profile.as_str()),
        value: Some(profile.as_str().to_owned()),
    });
    if let Some(parameter) = unsupported_profile {
        facts.push(MathMlFact {
            kind: MathMlFactKind::UnsupportedProfile,
            source_range: None,
            message: format!("MathML media profile `{parameter}` is not supported"),
            value: Some(parameter),
        });
    }

    let mut root_seen = false;
    let mut root_is_math = false;
    let mut root_range = None;
    let mut saw_presentation = false;
    let mut saw_content = false;
    let mut external_annotation_reported = false;
    let mut foreign_namespaces = BTreeSet::new();

    for event in &document.events {
        let range = Some(event.source_range);
        if event.source_range.byte_length == 0 {
            facts.push(MathMlFact {
                kind: MathMlFactKind::SourceMapUnavailable,
                source_range: range,
                message: "MathML event does not expose a non-empty source range".to_owned(),
                value: Some(event.index.to_string()),
            });
        }
        if event.kind == XmlEventKind::Doctype {
            facts.push(MathMlFact {
                kind: MathMlFactKind::DoctypeObserved,
                source_range: range,
                message: "MathML doctype declaration was parsed and preserved".to_owned(),
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
            root_range = range;
            facts.push(MathMlFact {
                kind: MathMlFactKind::RootObserved,
                source_range: range,
                message: format!("MathML root element `{local_name}` was parsed"),
                value: event.qualified_name.clone(),
            });
            if local_name != "math" {
                facts.push(MathMlFact {
                    kind: MathMlFactKind::RootNotMath,
                    source_range: range,
                    message: format!(
                        "MathML root element must be `math`, found `{}`",
                        event.qualified_name.as_deref().unwrap_or(local_name)
                    ),
                    value: event.qualified_name.clone(),
                });
            } else if namespace_uri != MATHML_NAMESPACE_URI {
                facts.push(MathMlFact {
                    kind: MathMlFactKind::NamespaceMissing,
                    source_range: range,
                    message: format!(
                        "MathML root `math` element must use the `{MATHML_NAMESPACE_URI}` namespace"
                    ),
                    value: Some(namespace_uri.to_owned()),
                });
            } else {
                root_is_math = true;
                facts.push(MathMlFact {
                    kind: MathMlFactKind::NamespaceObserved,
                    source_range: range,
                    message: "MathML document namespace was parsed".to_owned(),
                    value: Some(MATHML_NAMESPACE_URI.to_owned()),
                });
                if mathml_attribute(event, "alttext")
                    .is_some_and(|attribute| !attribute.value.trim().is_empty())
                {
                    facts.push(MathMlFact {
                        kind: MathMlFactKind::AccessibilityTextObserved,
                        source_range: range,
                        message: "MathML root alttext accessibility text was preserved".to_owned(),
                        value: mathml_attribute(event, "alttext")
                            .map(|attribute| attribute.value.clone()),
                    });
                }
            }
            continue;
        }

        if !root_is_math {
            continue;
        }
        if !namespace_uri.is_empty() && namespace_uri != MATHML_NAMESPACE_URI {
            if foreign_namespaces.insert(namespace_uri.to_owned()) {
                facts.push(MathMlFact {
                    kind: MathMlFactKind::ForeignContentObserved,
                    source_range: range,
                    message: format!(
                        "MathML foreign-content namespace `{namespace_uri}` was parsed"
                    ),
                    value: Some(namespace_uri.to_owned()),
                });
                facts.push(MathMlFact {
                    kind: MathMlFactKind::ForeignContentRejected,
                    source_range: range,
                    message: format!(
                        "MathML foreign-content namespace `{namespace_uri}` requires an explicit registered schema or converter policy"
                    ),
                    value: Some(namespace_uri.to_owned()),
                });
            }
            continue;
        }
        if namespace_uri != MATHML_NAMESPACE_URI {
            continue;
        }

        if mathml_is_presentation_element(local_name) {
            if !saw_presentation {
                facts.push(MathMlFact {
                    kind: MathMlFactKind::PresentationObserved,
                    source_range: range,
                    message: "Presentation MathML expression content was parsed".to_owned(),
                    value: Some(local_name.to_owned()),
                });
            }
            saw_presentation = true;
        }
        if mathml_is_content_element(local_name) {
            if !saw_content {
                facts.push(MathMlFact {
                    kind: MathMlFactKind::ContentObserved,
                    source_range: range,
                    message: "Content MathML expression content was parsed".to_owned(),
                    value: Some(local_name.to_owned()),
                });
            }
            saw_content = true;
        }
        if local_name == "semantics" {
            facts.push(MathMlFact {
                kind: MathMlFactKind::SemanticsObserved,
                source_range: range,
                message: "MathML semantics boundary was parsed".to_owned(),
                value: event.qualified_name.clone(),
            });
        }
        if matches!(local_name, "annotation" | "annotation-xml") {
            facts.push(MathMlFact {
                kind: MathMlFactKind::AnnotationObserved,
                source_range: range,
                message: format!("MathML `{local_name}` annotation boundary was parsed"),
                value: mathml_attribute(event, "encoding").map(|attribute| attribute.value.clone()),
            });
            facts.push(MathMlFact {
                kind: MathMlFactKind::AccessibilityTextObserved,
                source_range: range,
                message: "MathML semantic annotation was preserved as accessibility material"
                    .to_owned(),
                value: mathml_attribute(event, "encoding").map(|attribute| attribute.value.clone()),
            });
        }
        if !external_annotation_reported {
            if let Some(attribute) = event.attributes.iter().find(|attribute| {
                matches!(attribute.local_name.as_str(), "src" | "definitionURL")
                    && mathml_uri_requires_policy(&attribute.value)
            }) {
                external_annotation_reported = true;
                facts.push(MathMlFact {
                    kind: MathMlFactKind::ExternalAnnotationRejected,
                    source_range: range,
                    message: format!(
                        "MathML attribute `{}` URI `{}` requires explicit resolver policy",
                        attribute.qualified_name, attribute.value
                    ),
                    value: Some(attribute.value.clone()),
                });
            }
        }
    }

    if root_is_math {
        let mismatch = match profile {
            MathMlMediaProfile::Generic => None,
            MathMlMediaProfile::Presentation if !saw_presentation => {
                Some("application/mathml-presentation+xml must contain presentation MathML")
            }
            MathMlMediaProfile::Content if !saw_content => {
                Some("application/mathml-content+xml must contain content MathML")
            }
            _ => None,
        };
        if let Some(message) = mismatch {
            facts.push(MathMlFact {
                kind: MathMlFactKind::MalformedExpression,
                source_range: root_range,
                message: message.to_owned(),
                value: Some(profile.as_str().to_owned()),
            });
        }
    }

    facts
}

fn mathml_attribute<'a>(event: &'a XmlEventAst, local_name: &str) -> Option<&'a XmlAttributeAst> {
    event
        .attributes
        .iter()
        .find(|attribute| attribute.local_name == local_name)
}

fn mathml_is_presentation_element(local_name: &str) -> bool {
    matches!(
        local_name,
        "mi" | "mn"
            | "mo"
            | "mtext"
            | "mspace"
            | "ms"
            | "mrow"
            | "mfrac"
            | "msqrt"
            | "mroot"
            | "mstyle"
            | "merror"
            | "mpadded"
            | "mphantom"
            | "mfenced"
            | "menclose"
            | "msub"
            | "msup"
            | "msubsup"
            | "munder"
            | "mover"
            | "munderover"
            | "mmultiscripts"
            | "mtable"
            | "mtr"
            | "mlabeledtr"
            | "mtd"
            | "maction"
    )
}

fn mathml_is_content_element(local_name: &str) -> bool {
    matches!(
        local_name,
        "apply"
            | "bind"
            | "ci"
            | "cn"
            | "csymbol"
            | "lambda"
            | "piecewise"
            | "piece"
            | "otherwise"
            | "interval"
            | "list"
            | "set"
            | "vector"
            | "matrix"
            | "matrixrow"
            | "declare"
    )
}

fn mathml_uri_requires_policy(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    !(trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.to_ascii_lowercase().starts_with("data:"))
}

fn mathml_diagnostics(
    source_uri: &str,
    content_type: &str,
    facts: &[MathMlFact],
    contracts: &MathMlSchemaContractCatalog,
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
                    "mathml": {
                        "phase": "xml-parse-and-mathml-semantics",
                        "factKind": fact.kind.as_str(),
                        "contract": binding.contract,
                        "behavior": binding.behavior,
                        "policy": binding.policy,
                        "contentType": content_type,
                        "value": fact.value,
                        "sourceRange": fact.source_range.map(mathml_source_range_to_value),
                    }
                })),
                source_map: fact
                    .source_range
                    .map(|range| mathml_source_map(range, content_type)),
                ..Diagnostic::default()
            })
        })
        .collect()
}

fn mathml_source_range_from_xml_fact(
    line: Option<u32>,
    column: Option<u32>,
    byte_offset: Option<u64>,
    byte_length: Option<u64>,
) -> Option<XmlSourceRange> {
    Some(XmlSourceRange {
        start: crate::validation::xml::XmlSourcePosition {
            line: line.unwrap_or(1),
            column: column.unwrap_or(1),
            byte_offset: byte_offset?,
        },
        byte_length: byte_length.unwrap_or(1),
    })
}

fn mathml_source_range_to_value(range: XmlSourceRange) -> Value {
    json!({
        "byteOffset": range.start.byte_offset,
        "byteLength": range.byte_length,
        "line": range.start.line,
        "column": range.start.column,
    })
}

fn mathml_source_map(range: XmlSourceRange, content_type: &str) -> SourceMapStack {
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(1),
            span: FrameSpan::Single(ByteRange::new(
                range.start.byte_offset,
                u32::try_from(range.byte_length).unwrap_or(u32::MAX),
            )),
            transform: TransformKind::ContentTypeTransform {
                content_type: content_type.to_owned(),
            },
        }],
    }
}

fn mathml_event_to_cemt_subject(event: &XmlEventAst, content_type: &str) -> Value {
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
        "sourceRange": mathml_source_range_to_value(event.source_range),
        "sourceMap": mathml_source_map(event.source_range, content_type),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str, content_type: &str) -> (MathMlDocumentAst, Vec<Diagnostic>) {
        let (document, diagnostics) =
            mathml_document_ast_from_source_bytes(MathMlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.mml",
                content_type: Some(content_type),
            });
        (document.expect("typed MathML document"), diagnostics)
    }

    fn validate(source: &str, content_type: &str) -> Vec<Diagnostic> {
        parse(source, content_type).1
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn mathml_ast_reuses_xml_events_with_profile_identity_and_source_maps() {
        let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<math xmlns="http://www.w3.org/1998/Math/MathML"><apply><plus/><ci>x</ci></apply></math>
"#;
        let (document, diagnostics) =
            parse(source, "application/mathml-content+xml; charset=UTF-8");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(document.media_profile, MathMlMediaProfile::Content);
        assert_eq!(
            document.xml_document.source.media_type,
            MATHML_CONTENT_CONTENT_TYPE
        );
        assert_eq!(
            document
                .xml_document
                .source
                .parameters
                .get("charset")
                .map(String::as_str),
            Some("UTF-8")
        );
        assert!(document
            .xml_document
            .events
            .iter()
            .all(|event| event.source_range.byte_length > 0));
        let subject = document.to_cemt_subject();
        assert_eq!(subject["kind"], json!("mathml-document"));
        assert_eq!(subject["mediaProfile"], json!("content"));
        assert_eq!(
            subject["events"][0]["sourceMap"]["frames"][0]["transform"]["content_type"],
            json!(MATHML_CONTENT_CONTENT_TYPE)
        );
    }

    #[test]
    fn mathml_source_validator_accepts_presentation_and_content_profiles() {
        for (source, content_type) in [
            (
                r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow></math>"#,
                MATHML_PRESENTATION_CONTENT_TYPE,
            ),
            (
                r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><apply><plus/><ci>x</ci><cn>1</cn></apply></math>"#,
                MATHML_CONTENT_CONTENT_TYPE,
            ),
        ] {
            let diagnostics = validate(source, content_type);
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
        }
    }

    #[test]
    fn mathml_source_validator_reports_schema_bound_policy_facts() {
        for (source, content_type, code) in [
            (
                r#"<math><mi>x</mi></math>"#,
                MATHML_CONTENT_TYPE,
                "cem.mathml.namespace_missing",
            ),
            (
                r#"<mrow xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></mrow>"#,
                MATHML_CONTENT_TYPE,
                "cem.mathml.root_not_math",
            ),
            (
                r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mrow><mi>x</mi></mrow></math>"#,
                MATHML_CONTENT_CONTENT_TYPE,
                "cem.mathml.malformed_expression",
            ),
            (
                r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><semantics><mi>x</mi><annotation src="formula.json"/></semantics></math>"#,
                MATHML_CONTENT_TYPE,
                "cem.mathml.external_annotation_rejected",
            ),
            (
                r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><annotation-xml><span xmlns="http://www.w3.org/1999/xhtml">x</span></annotation-xml></math>"#,
                MATHML_CONTENT_TYPE,
                "cem.mathml.foreign_content_rejected",
            ),
        ] {
            let diagnostics = validate(source, content_type);
            assert!(has_code(&diagnostics, code), "{diagnostics:?}");
            assert!(diagnostics.iter().all(|diagnostic| {
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("mathml"))
                    .and_then(|details| details.get("behavior"))
                    == Some(&json!(MATHML_FACT_BEHAVIOR))
            }));
        }
    }

    #[test]
    fn mathml_preserves_semantics_annotation_and_accessibility_facts() {
        let (document, diagnostics) = parse(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML" alttext="x squared"><semantics><msup><mi>x</mi><mn>2</mn></msup><annotation encoding="application/json">{"name":"x squared"}</annotation></semantics></math>"#,
            MATHML_CONTENT_TYPE,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        for kind in [
            MathMlFactKind::SemanticsObserved,
            MathMlFactKind::AnnotationObserved,
            MathMlFactKind::AccessibilityTextObserved,
        ] {
            assert!(document.facts.iter().any(|fact| fact.kind == kind));
        }
    }

    #[test]
    fn mathml_source_validator_reports_unsupported_profile_warning() {
        let diagnostics = validate(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>"#,
            "application/mathml+xml; profile=custom",
        );
        assert!(has_code(&diagnostics, "cem.mathml.unsupported_profile"));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }

    #[test]
    fn mathml_source_validator_reports_not_well_formed_xml() {
        let diagnostics = validate(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mrow><mi>x</mrow></math>"#,
            MATHML_CONTENT_TYPE,
        );
        assert!(has_code(&diagnostics, "cem.mathml.not_well_formed_xml"));
    }

    #[test]
    fn mathml_inherits_xml_doctype_and_entity_safety_policy() {
        let diagnostics = validate(
            r#"<!DOCTYPE math [<!ENTITY remote SYSTEM "https://example.test/entity">]><math xmlns="http://www.w3.org/1998/Math/MathML"><mi>&remote;</mi></math>"#,
            MATHML_CONTENT_TYPE,
        );
        assert!(has_code(&diagnostics, "cem.mathml.dtd_rejected"));
        assert!(has_code(
            &diagnostics,
            "cem.mathml.external_entity_rejected"
        ));
    }
}
