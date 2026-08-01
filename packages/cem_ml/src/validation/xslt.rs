use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{
    content_type_essence, XSLT_CONTENT_TYPE, XSLT_NAMESPACE_URI, XSLT_SCHEMA_URI,
};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::xml::{
    xml_document_ast_from_source_bytes, XmlAttributeAst, XmlDocumentAst, XmlEventAst, XmlEventKind,
    XmlParseFactKind, XmlSourceRange, XmlSourceValidationRequest,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const XSLT_PACKAGE_ID: &str = "xslt";
const XSLT_FACT_BEHAVIOR: &str = "xslt-report-fact";
pub const XSLT_TEXT_CONTENT_TYPE: &str = "text/xsl";

#[derive(Debug, Clone, Copy)]
pub struct XsltSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsltStylesheetAst {
    pub xml_document: XmlDocumentAst,
    pub version: Option<String>,
    pub facts: Vec<XsltFact>,
    pub line_ending: Option<String>,
}

impl XsltStylesheetAst {
    pub fn to_cemt_subject(&self) -> Value {
        let source = &self.xml_document.source;
        let encoding = &self.xml_document.encoding_report;
        json!({
            "kind": "xslt-stylesheet",
            "contentType": source.media_type,
            "schema": XSLT_SCHEMA_URI,
            "category": "xslt-stylesheet",
            "version": self.version,
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
                .map(XsltFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "events": self
                .xml_document
                .events
                .iter()
                .map(|event| xslt_event_to_cemt_subject(event, &source.media_type))
                .collect::<Vec<_>>(),
            "lineEnding": self.line_ending,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum XsltFactKind {
    NotWellFormedXml,
    UnsupportedEncoding,
    EncodingConflict,
    UnboundNamespacePrefix,
    DuplicateAttribute,
    DtdRejected,
    ExternalEntityRejected,
    EntityExpansionLimit,
    SourceMapUnavailable,
    RootNotStylesheet,
    NamespaceMissing,
    VersionMissing,
    VersionMalformed,
    UnsupportedVersion,
    EntryPointMissing,
    ExternalUriRejected,
    UnsupportedConstruct,
    RootObserved,
    NamespaceObserved,
    VersionObserved,
    DeclarationObserved,
    TemplateObserved,
    LiteralResultObserved,
    XPathObserved,
    BrowserEnginePolicyObserved,
    DoctypeObserved,
}

impl XsltFactKind {
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
            Self::RootNotStylesheet => "root-not-stylesheet",
            Self::NamespaceMissing => "namespace-missing",
            Self::VersionMissing => "version-missing",
            Self::VersionMalformed => "version-malformed",
            Self::UnsupportedVersion => "unsupported-version",
            Self::EntryPointMissing => "entrypoint-missing",
            Self::ExternalUriRejected => "external-uri-rejected",
            Self::UnsupportedConstruct => "unsupported-construct",
            Self::RootObserved => "root-observed",
            Self::NamespaceObserved => "namespace-observed",
            Self::VersionObserved => "version-observed",
            Self::DeclarationObserved => "declaration-observed",
            Self::TemplateObserved => "template-observed",
            Self::LiteralResultObserved => "literal-result-observed",
            Self::XPathObserved => "xpath-observed",
            Self::BrowserEnginePolicyObserved => "browser-engine-policy-observed",
            Self::DoctypeObserved => "doctype-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsltFact {
    pub kind: XsltFactKind,
    pub source_range: Option<XmlSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

impl XsltFact {
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "sourceRange": self.source_range.map(xslt_source_range_to_value),
            "message": self.message,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XsltDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsltSchemaContractCatalog {
    fact_bindings: BTreeMap<String, XsltDiagnosticBinding>,
}

impl XsltSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<XsltSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(XSLT_PACKAGE_ID)
                .expect("built-in XSLT schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(XSLT_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != XSLT_FACT_BEHAVIOR {
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
                    XsltDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: XsltFactKind) -> Option<&XsltDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_xslt_source_bytes(request: XsltSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    if request
        .content_type
        .map(content_type_essence)
        .as_deref()
        .is_some_and(is_xslt_custom_element_source_content_type)
    {
        return validate_xslt_compat_source_bytes(request);
    }
    let (_, diagnostics) = xslt_stylesheet_ast_from_source_bytes(request);
    diagnostics
}

pub fn xslt_stylesheet_ast_from_source_bytes(
    request: XsltSourceValidationRequest<'_>,
) -> (Option<XsltStylesheetAst>, Vec<Diagnostic>) {
    let (xml_document, _) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: request.bytes,
        source_uri: request.source_uri,
        content_type: request.content_type.or(Some(XSLT_CONTENT_TYPE)),
    });
    let Some(xml_document) = xml_document else {
        return (None, Vec::new());
    };
    let (version, facts) = xslt_facts(&xml_document);
    let diagnostics = xslt_fact_diagnostics(
        request.source_uri,
        &xml_document.source.media_type,
        &facts,
        XsltSchemaContractCatalog::from_builtin(),
    );
    let line_ending = xml_document.line_ending.clone();
    (
        Some(XsltStylesheetAst {
            xml_document,
            version,
            facts,
            line_ending,
        }),
        diagnostics,
    )
}

fn xslt_facts(document: &XmlDocumentAst) -> (Option<String>, Vec<XsltFact>) {
    let mut facts = document
        .parse_facts
        .iter()
        .map(|fact| XsltFact {
            kind: match fact.kind {
                XmlParseFactKind::ParseError => XsltFactKind::NotWellFormedXml,
                XmlParseFactKind::UnsupportedEncoding => XsltFactKind::UnsupportedEncoding,
                XmlParseFactKind::EncodingConflict => XsltFactKind::EncodingConflict,
                XmlParseFactKind::UnboundNamespacePrefix => XsltFactKind::UnboundNamespacePrefix,
                XmlParseFactKind::DuplicateAttribute => XsltFactKind::DuplicateAttribute,
                XmlParseFactKind::DtdRejected => XsltFactKind::DtdRejected,
                XmlParseFactKind::ExternalEntityRejected => XsltFactKind::ExternalEntityRejected,
                XmlParseFactKind::EntityExpansionLimit => XsltFactKind::EntityExpansionLimit,
                XmlParseFactKind::SourceMapUnavailable => XsltFactKind::SourceMapUnavailable,
            },
            source_range: xslt_source_range_from_xml_fact(
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
    let mut root_is_stylesheet = false;
    let mut root_range = None;
    let mut version = None;
    let mut top_level_template_seen = false;
    let mut external_uri_reported = false;
    let mut unsupported_construct_reported = false;
    let mut extension_namespaces = Vec::new();

    for event in &document.events {
        let range = Some(event.source_range);
        if event.source_range.byte_length == 0 {
            facts.push(XsltFact {
                kind: XsltFactKind::SourceMapUnavailable,
                source_range: range,
                message: "XSLT event does not expose a non-empty source range".to_owned(),
                value: Some(event.index.to_string()),
            });
        }
        if event.kind == XmlEventKind::Doctype {
            facts.push(XsltFact {
                kind: XsltFactKind::DoctypeObserved,
                source_range: range,
                message: "XSLT doctype declaration was parsed and preserved".to_owned(),
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
            facts.push(XsltFact {
                kind: XsltFactKind::RootObserved,
                source_range: range,
                message: format!("XSLT root element `{local_name}` was parsed"),
                value: event.qualified_name.clone(),
            });
            if !matches!(local_name, "stylesheet" | "transform") {
                facts.push(XsltFact {
                    kind: XsltFactKind::RootNotStylesheet,
                    source_range: range,
                    message: format!(
                        "XSLT root element must be `stylesheet` or `transform`, found `{}`",
                        event.qualified_name.as_deref().unwrap_or(local_name)
                    ),
                    value: event.qualified_name.clone(),
                });
                continue;
            }
            if namespace_uri != XSLT_NAMESPACE_URI {
                facts.push(XsltFact {
                    kind: XsltFactKind::NamespaceMissing,
                    source_range: range,
                    message: format!("XSLT root must use the `{XSLT_NAMESPACE_URI}` namespace"),
                    value: Some(namespace_uri.to_owned()),
                });
                continue;
            }

            root_is_stylesheet = true;
            facts.push(XsltFact {
                kind: XsltFactKind::NamespaceObserved,
                source_range: range,
                message: "XSLT document namespace was parsed".to_owned(),
                value: Some(XSLT_NAMESPACE_URI.to_owned()),
            });
            facts.push(XsltFact {
                kind: XsltFactKind::BrowserEnginePolicyObserved,
                source_range: range,
                message: "XSLT source is retained for native lifecycle processing without browser XSLTProcessor delegation".to_owned(),
                value: Some("native-only".to_owned()),
            });
            extension_namespaces = xslt_extension_namespaces(event);
            match xslt_attribute(event, "version") {
                None => facts.push(XsltFact {
                    kind: XsltFactKind::VersionMissing,
                    source_range: range,
                    message: "XSLT stylesheet root must declare a version attribute".to_owned(),
                    value: None,
                }),
                Some(attribute) => {
                    version = Some(attribute.value.clone());
                    facts.push(XsltFact {
                        kind: XsltFactKind::VersionObserved,
                        source_range: range,
                        message: format!("XSLT version `{}` was parsed", attribute.value),
                        value: Some(attribute.value.clone()),
                    });
                    match crate::schema::xslt::parse_xslt_version(&attribute.value) {
                        None => facts.push(XsltFact {
                            kind: XsltFactKind::VersionMalformed,
                            source_range: range,
                            message: format!("XSLT version `{}` is malformed", attribute.value),
                            value: Some(attribute.value.clone()),
                        }),
                        Some(parsed) if parsed.major == 0 || parsed.major > 3 => {
                            facts.push(XsltFact {
                                kind: XsltFactKind::UnsupportedVersion,
                                source_range: range,
                                message: format!(
                                    "XSLT version `{}` is not supported by the schema package",
                                    attribute.value
                                ),
                                value: Some(attribute.value.clone()),
                            });
                        }
                        Some(_) => {}
                    }
                }
            }
            continue;
        }

        if !root_is_stylesheet {
            continue;
        }
        let is_xslt_element = namespace_uri == XSLT_NAMESPACE_URI;
        if is_xslt_element {
            if event.depth == 1 && local_name == "template" {
                top_level_template_seen = true;
                facts.push(XsltFact {
                    kind: XsltFactKind::TemplateObserved,
                    source_range: range,
                    message: "Top-level XSLT template entrypoint was parsed".to_owned(),
                    value: xslt_attribute(event, "name")
                        .or_else(|| xslt_attribute(event, "match"))
                        .map(|attribute| attribute.value.clone()),
                });
            } else if event.depth == 1 {
                facts.push(XsltFact {
                    kind: XsltFactKind::DeclarationObserved,
                    source_range: range,
                    message: format!("Top-level XSLT declaration `xsl:{local_name}` was parsed"),
                    value: Some(local_name.to_owned()),
                });
            }

            if !external_uri_reported && xslt_event_requires_external_uri_policy(event) {
                external_uri_reported = true;
                facts.push(XsltFact {
                    kind: XsltFactKind::ExternalUriRejected,
                    source_range: range,
                    message: "XSLT external URI access requires an explicit resolver policy"
                        .to_owned(),
                    value: Some(local_name.to_owned()),
                });
            }
            if !unsupported_construct_reported
                && matches!(local_name, "function" | "result-document")
            {
                unsupported_construct_reported = true;
                facts.push(XsltFact {
                    kind: XsltFactKind::UnsupportedConstruct,
                    source_range: range,
                    message: format!(
                        "XSLT construct `xsl:{local_name}` is outside the bounded executable profile"
                    ),
                    value: Some(local_name.to_owned()),
                });
            }
        } else {
            facts.push(XsltFact {
                kind: XsltFactKind::LiteralResultObserved,
                source_range: range,
                message: format!(
                    "XSLT literal result element `{}` was preserved",
                    event.qualified_name.as_deref().unwrap_or(local_name)
                ),
                value: event.namespace_uri.clone(),
            });
            if !unsupported_construct_reported
                && (extension_namespaces
                    .iter()
                    .any(|extension| extension == namespace_uri)
                    || local_name == "script"
                    || namespace_uri.contains("microsoft.com")
                    || namespace_uri.contains("exslt.org"))
            {
                unsupported_construct_reported = true;
                facts.push(XsltFact {
                    kind: XsltFactKind::UnsupportedConstruct,
                    source_range: range,
                    message: format!(
                        "XSLT extension instruction `{}` requires an explicit capability",
                        event.qualified_name.as_deref().unwrap_or(local_name)
                    ),
                    value: event.namespace_uri.clone(),
                });
            }
        }

        for attribute in event.attributes.iter().filter(|attribute| {
            xslt_is_xpath_attribute(&attribute.local_name) && !attribute.value.trim().is_empty()
        }) {
            facts.push(XsltFact {
                kind: XsltFactKind::XPathObserved,
                source_range: range,
                message: format!(
                    "XSLT XPath-bearing attribute `{}` was preserved lexically",
                    attribute.qualified_name
                ),
                value: Some(attribute.value.clone()),
            });
            if !external_uri_reported && xslt_expression_uses_external_document(&attribute.value) {
                external_uri_reported = true;
                facts.push(XsltFact {
                    kind: XsltFactKind::ExternalUriRejected,
                    source_range: range,
                    message: "XSLT document() access requires an explicit resolver policy"
                        .to_owned(),
                    value: Some(attribute.value.clone()),
                });
            }
            if !unsupported_construct_reported
                && xslt_expression_uses_extension_function(&attribute.value)
            {
                unsupported_construct_reported = true;
                facts.push(XsltFact {
                    kind: XsltFactKind::UnsupportedConstruct,
                    source_range: range,
                    message: "XSLT extension function requires an explicit capability".to_owned(),
                    value: Some(attribute.value.clone()),
                });
            }
        }
    }

    if root_is_stylesheet && !top_level_template_seen {
        facts.push(XsltFact {
            kind: XsltFactKind::EntryPointMissing,
            source_range: root_range,
            message: "XSLT stylesheet must declare at least one top-level xsl:template".to_owned(),
            value: None,
        });
    }

    (version, facts)
}

fn xslt_attribute<'a>(event: &'a XmlEventAst, local_name: &str) -> Option<&'a XmlAttributeAst> {
    event
        .attributes
        .iter()
        .find(|attribute| attribute.local_name == local_name)
}

fn xslt_extension_namespaces(root: &XmlEventAst) -> Vec<String> {
    let Some(prefixes) = xslt_attribute(root, "extension-element-prefixes") else {
        return Vec::new();
    };
    prefixes
        .value
        .split_ascii_whitespace()
        .filter_map(|prefix| {
            let qualified_name = if prefix == "#default" {
                "xmlns".to_owned()
            } else {
                format!("xmlns:{prefix}")
            };
            root.attributes
                .iter()
                .find(|attribute| attribute.qualified_name == qualified_name)
                .map(|attribute| attribute.value.clone())
        })
        .collect()
}

fn xslt_event_requires_external_uri_policy(event: &XmlEventAst) -> bool {
    matches!(
        event.local_name.as_deref(),
        Some("include" | "import" | "result-document")
    ) && xslt_attribute(event, "href")
        .is_some_and(|attribute| xslt_uri_requires_policy(&attribute.value))
}

fn xslt_is_xpath_attribute(local_name: &str) -> bool {
    matches!(
        local_name,
        "select"
            | "test"
            | "match"
            | "use"
            | "count"
            | "from"
            | "group-by"
            | "group-adjacent"
            | "group-starting-with"
            | "group-ending-with"
    )
}

fn xslt_expression_uses_extension_function(value: &str) -> bool {
    value.char_indices().any(|(colon_index, character)| {
        if character != ':' || value[..colon_index].ends_with("http") {
            return false;
        }
        value[colon_index + 1..].find('(').is_some_and(|open| {
            !value[colon_index + 1..colon_index + 1 + open].contains(char::is_whitespace)
        })
    })
}

fn xslt_fact_diagnostics(
    source_uri: &str,
    content_type: &str,
    facts: &[XsltFact],
    contracts: &XsltSchemaContractCatalog,
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
                    "xslt": {
                        "phase": "xml-parse-and-xslt-semantics",
                        "factKind": fact.kind.as_str(),
                        "contract": binding.contract,
                        "behavior": binding.behavior,
                        "policy": binding.policy,
                        "contentType": content_type,
                        "value": fact.value,
                        "sourceRange": fact.source_range.map(xslt_source_range_to_value),
                    }
                })),
                source_map: fact
                    .source_range
                    .map(|range| xslt_source_map(range, content_type)),
                ..Diagnostic::default()
            })
        })
        .collect()
}

fn xslt_source_range_from_xml_fact(
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

fn xslt_source_range_to_value(range: XmlSourceRange) -> Value {
    json!({
        "byteOffset": range.start.byte_offset,
        "byteLength": range.byte_length,
        "line": range.start.line,
        "column": range.start.column,
    })
}

fn xslt_source_map(range: XmlSourceRange, content_type: &str) -> SourceMapStack {
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

fn xslt_event_to_cemt_subject(event: &XmlEventAst, content_type: &str) -> Value {
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
        "sourceRange": xslt_source_range_to_value(event.source_range),
        "sourceMap": xslt_source_map(event.source_range, content_type),
    })
}

pub fn validate_xslt_compat_source_bytes(
    request: XsltSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let content_type = request.content_type.map(content_type_essence);
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            return vec![xslt_not_well_formed_diagnostic(
                &request,
                "",
                u64::try_from(error.valid_up_to()).ok(),
                format!("XSLT source must be valid UTF-8: {error}"),
            )];
        }
    };

    if content_type
        .as_deref()
        .is_some_and(is_xslt_custom_element_source_content_type)
        && !xslt_source_has_stylesheet_root(source)
    {
        validate_xslt_legacy_fragment_source(&request, source)
    } else {
        validate_xslt_source(&request, source)
    }
}

fn is_xslt_custom_element_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "custom-element-xslt"
            | "text/custom-element-xslt"
            | "application/custom-element-xslt"
            | "text/x-custom-element-xslt"
    )
}

fn xslt_source_has_stylesheet_root(source: &str) -> bool {
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;
    let namespace_stack = vec![xml_initial_namespaces()];

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start))
            | Ok(quick_xml::events::Event::Empty(start)) => {
                let namespaces = xslt_namespaces_for_detection(&start, &namespace_stack);
                let qualified_name = qname_display(start.name().as_ref());
                let (namespace_uri, local_name) =
                    xml_element_expanded_name(&qualified_name, &namespaces);
                return matches!(local_name.as_str(), "stylesheet" | "transform")
                    && namespace_uri == XSLT_NAMESPACE_URI;
            }
            Ok(quick_xml::events::Event::Eof) => return false,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
}

fn xslt_namespaces_for_detection(
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
) -> BTreeMap<String, String> {
    let mut namespaces = namespace_stack
        .last()
        .cloned()
        .unwrap_or_else(xml_initial_namespaces);
    for attribute in start.attributes().with_checks(false).flatten() {
        let name = qname_display(attribute.key.as_ref());
        let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
        if name == "xmlns" {
            namespaces.insert(String::new(), value);
        } else if let Some(prefix) = name.strip_prefix("xmlns:") {
            namespaces.insert(prefix.to_owned(), value);
        }
    }
    namespaces
}

fn validate_xslt_legacy_fragment_source(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if xslt_legacy_fragment_contains_unsupported_construct(source) {
        diagnostics.push(xslt_diagnostic(
            request,
            source,
            Some(0),
            "legacy_xslt.unsupported_construct",
            Severity::Warning,
            "Legacy custom-element XSLT fragment contains a construct outside the bounded compatibility profile"
                .to_owned(),
        ));
    }
    diagnostics
}

fn xslt_legacy_fragment_contains_unsupported_construct(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    [
        "<xsl:copy-of",
        "<xsl:result-document",
        "<xsl:function",
        "<xsl:import",
        "<xsl:include",
        "<msxsl:script",
        "document(",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[derive(Clone, Debug)]
struct XsltAttributeView {
    local_name: String,
    value: String,
}

#[derive(Clone, Debug)]
struct XsltElementFrame {
    local_name: String,
    namespace_uri: String,
    attributes: Vec<XsltAttributeView>,
}

#[derive(Clone, Debug, Default)]
struct XsltDocumentState {
    root_is_stylesheet: bool,
    saw_top_level_template: bool,
    reported_external_uri: bool,
    reported_unsupported_construct: bool,
}

fn validate_xslt_source(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;

    let mut element_stack: Vec<XsltElementFrame> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut state = XsltDocumentState::default();

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                let start_offset = xml_event_position(&reader, &start, false);
                let (frame, namespaces, mut event_diagnostics) =
                    xslt_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                xslt_validate_element(
                    request,
                    source,
                    start_offset,
                    &frame,
                    &element_stack,
                    &mut state,
                    &mut root_count,
                    &mut diagnostics,
                );
                element_stack.push(frame);
                namespace_stack.push(namespaces);
            }
            Ok(quick_xml::events::Event::Empty(start)) => {
                let start_offset = xml_event_position(&reader, &start, true);
                let (frame, _, mut event_diagnostics) =
                    xslt_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                xslt_validate_element(
                    request,
                    source,
                    start_offset,
                    &frame,
                    &element_stack,
                    &mut state,
                    &mut root_count,
                    &mut diagnostics,
                );
            }
            Ok(quick_xml::events::Event::End(_)) => {
                if element_stack.pop().is_some() && namespace_stack.len() > 1 {
                    namespace_stack.pop();
                }
            }
            Ok(quick_xml::events::Event::Text(text)) => {
                if element_stack.is_empty() && !xml_bytes_are_whitespace(text.as_ref()) {
                    diagnostics.push(xslt_not_well_formed_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "XSLT document cannot contain character data outside the document element"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => {
                if !state.reported_external_uri {
                    state.reported_external_uri = true;
                    diagnostics.push(xslt_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.xslt.external_uri_rejected",
                        Severity::Error,
                        "XSLT DOCTYPE declarations are rejected because they can reference external resources"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(xslt_xml_error_diagnostic(
                    request,
                    source,
                    Some(reader.error_position()),
                    &error,
                ));
                break;
            }
        }
    }

    if root_count == 0 {
        diagnostics.push(xslt_not_well_formed_diagnostic(
            request,
            source,
            Some(0),
            "XSLT document must contain a document element".to_owned(),
        ));
    } else if state.root_is_stylesheet && !state.saw_top_level_template {
        diagnostics.push(xslt_diagnostic(
            request,
            source,
            Some(0),
            "cem.xslt.entrypoint_missing",
            Severity::Error,
            "XSLT stylesheet must declare at least one top-level xsl:template".to_owned(),
        ));
    }

    diagnostics
}

fn xslt_start_frame(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
    byte_offset: Option<u64>,
) -> (XsltElementFrame, BTreeMap<String, String>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut raw_attributes = Vec::new();
    let mut namespaces = namespace_stack
        .last()
        .cloned()
        .unwrap_or_else(xml_initial_namespaces);

    for attribute in start.attributes().with_checks(false) {
        match attribute {
            Ok(attribute) => {
                let name = qname_display(attribute.key.as_ref());
                let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                if name == "xmlns" {
                    namespaces.insert(String::new(), value.clone());
                } else if let Some(prefix) = name.strip_prefix("xmlns:") {
                    namespaces.insert(prefix.to_owned(), value.clone());
                }
                raw_attributes.push((name, value));
            }
            Err(error) => diagnostics.push(xslt_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                format!("XSLT XML attribute parse error: {error}"),
            )),
        }
    }

    let qualified_name = qname_display(start.name().as_ref());
    if let Some(prefix) = xml_qname_prefix(&qualified_name) {
        if !xml_prefix_is_bound(&namespaces, prefix) {
            diagnostics.push(xslt_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                format!("XSLT namespace prefix `{prefix}` is not bound for `{qualified_name}`"),
            ));
        }
    }

    let (namespace_uri, local_name) = xml_element_expanded_name(&qualified_name, &namespaces);
    let attributes = raw_attributes
        .into_iter()
        .filter(|(qualified_name, _)| !xml_attribute_is_namespace_declaration(qualified_name))
        .map(|(qualified_name, value)| {
            if let Some(prefix) = xml_qname_prefix(&qualified_name) {
                if !xml_prefix_is_bound(&namespaces, prefix) {
                    diagnostics.push(xslt_not_well_formed_diagnostic(
                        request,
                        source,
                        byte_offset,
                        format!(
                            "XSLT namespace prefix `{prefix}` is not bound for attribute `{qualified_name}`"
                        ),
                    ));
                }
            }
            let (_, local_name) = xml_attribute_expanded_name(&qualified_name, &namespaces);
            XsltAttributeView { local_name, value }
        })
        .collect();

    (
        XsltElementFrame {
            local_name,
            namespace_uri,
            attributes,
        },
        namespaces,
        diagnostics,
    )
}

fn xslt_validate_element(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    frame: &XsltElementFrame,
    element_stack: &[XsltElementFrame],
    state: &mut XsltDocumentState,
    root_count: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if element_stack.is_empty() {
        *root_count += 1;
        if *root_count > 1 {
            diagnostics.push(xslt_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                "XSLT document must have exactly one document element".to_owned(),
            ));
            return;
        }
        if !matches!(frame.local_name.as_str(), "stylesheet" | "transform") {
            diagnostics.push(xslt_diagnostic(
                request,
                source,
                byte_offset,
                "cem.xslt.root_not_stylesheet",
                Severity::Error,
                format!(
                    "XSLT root element must be `stylesheet` or `transform`, found `{}`",
                    frame.local_name
                ),
            ));
            return;
        }
        if frame.namespace_uri != XSLT_NAMESPACE_URI {
            diagnostics.push(xslt_diagnostic(
                request,
                source,
                byte_offset,
                "cem.xslt.namespace_missing",
                Severity::Error,
                "XSLT root element must use the http://www.w3.org/1999/XSL/Transform namespace"
                    .to_owned(),
            ));
            return;
        }

        state.root_is_stylesheet = true;
        xslt_validate_root_version(request, source, byte_offset, frame, diagnostics);
        return;
    }

    if !state.root_is_stylesheet {
        return;
    }

    let is_xslt_element = frame.namespace_uri == XSLT_NAMESPACE_URI;
    if is_xslt_element && element_stack.len() == 1 && frame.local_name == "template" {
        state.saw_top_level_template = true;
    }

    if is_xslt_element {
        xslt_validate_external_uri_policy(request, source, byte_offset, frame, state, diagnostics);
        if matches!(frame.local_name.as_str(), "function" | "result-document")
            && !state.reported_unsupported_construct
        {
            state.reported_unsupported_construct = true;
            diagnostics.push(xslt_diagnostic(
                request,
                source,
                byte_offset,
                "legacy_xslt.unsupported_construct",
                Severity::Warning,
                format!(
                    "XSLT construct `xsl:{}` is outside the bounded legacy compatibility profile",
                    frame.local_name
                ),
            ));
        }
    } else if xslt_is_extension_construct(frame) && !state.reported_unsupported_construct {
        state.reported_unsupported_construct = true;
        diagnostics.push(xslt_diagnostic(
            request,
            source,
            byte_offset,
            "legacy_xslt.unsupported_construct",
            Severity::Warning,
            format!(
                "XSLT extension construct `{}` is outside the bounded legacy compatibility profile",
                frame.local_name
            ),
        ));
    }
}

fn xslt_validate_root_version(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    frame: &XsltElementFrame,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(version) = xslt_attribute_value(frame, "version") else {
        diagnostics.push(xslt_diagnostic(
            request,
            source,
            byte_offset,
            "cem.xslt.version_missing",
            Severity::Error,
            "XSLT stylesheet root must declare a version attribute".to_owned(),
        ));
        return;
    };

    let Some(parsed) = crate::schema::xslt::parse_xslt_version(version) else {
        diagnostics.push(xslt_diagnostic(
            request,
            source,
            byte_offset,
            "cem.xslt.version_malformed",
            Severity::Error,
            format!("XSLT version `{version}` is malformed"),
        ));
        return;
    };

    if parsed.major == 0 || parsed.major > 3 {
        diagnostics.push(xslt_diagnostic(
            request,
            source,
            byte_offset,
            "cem.xslt.unsupported_version",
            Severity::Error,
            format!("XSLT version `{version}` is not supported by the schema package"),
        ));
    }
}

fn xslt_validate_external_uri_policy(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    frame: &XsltElementFrame,
    state: &mut XsltDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if state.reported_external_uri {
        return;
    }

    let direct_href_requires_policy = matches!(
        frame.local_name.as_str(),
        "include" | "import" | "result-document"
    ) && xslt_attribute_value(frame, "href")
        .is_some_and(xslt_uri_requires_policy);
    let expression_document_requires_policy = frame.attributes.iter().any(|attribute| {
        matches!(attribute.local_name.as_str(), "select" | "test")
            && xslt_expression_uses_external_document(&attribute.value)
    });

    if direct_href_requires_policy || expression_document_requires_policy {
        state.reported_external_uri = true;
        diagnostics.push(xslt_diagnostic(
            request,
            source,
            byte_offset,
            "cem.xslt.external_uri_rejected",
            Severity::Error,
            "XSLT external URI access requires an explicit resolver policy".to_owned(),
        ));
    }
}

fn xslt_attribute_value<'a>(frame: &'a XsltElementFrame, local_name: &str) -> Option<&'a str> {
    frame
        .attributes
        .iter()
        .find(|attribute| attribute.local_name == local_name)
        .map(|attribute| attribute.value.as_str())
}

fn xslt_uri_requires_policy(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    !(trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.to_ascii_lowercase().starts_with("data:"))
}

fn xslt_expression_uses_external_document(value: &str) -> bool {
    value.to_ascii_lowercase().contains("document(")
}

fn xslt_is_extension_construct(frame: &XsltElementFrame) -> bool {
    if frame.namespace_uri.is_empty() || frame.namespace_uri == XSLT_NAMESPACE_URI {
        return false;
    }

    frame.local_name == "script"
        || frame.namespace_uri.contains("microsoft.com")
        || frame.namespace_uri.contains("exslt.org")
}

fn xml_element_expanded_name(
    qualified_name: &str,
    namespaces: &BTreeMap<String, String>,
) -> (String, String) {
    if let Some((prefix, local_name)) = qualified_name.split_once(':') {
        (
            namespaces.get(prefix).cloned().unwrap_or_default(),
            local_name.to_owned(),
        )
    } else {
        (
            namespaces.get("").cloned().unwrap_or_default(),
            qualified_name.to_owned(),
        )
    }
}

fn xml_attribute_expanded_name(
    qualified_name: &str,
    namespaces: &BTreeMap<String, String>,
) -> (String, String) {
    if let Some((prefix, local_name)) = qualified_name.split_once(':') {
        let namespace_uri = namespaces.get(prefix).cloned().unwrap_or_default();
        (namespace_uri, local_name.to_owned())
    } else {
        (String::new(), qualified_name.to_owned())
    }
}

fn xml_qname_prefix(qualified_name: &str) -> Option<&str> {
    qualified_name
        .split_once(':')
        .map(|(prefix, _)| prefix)
        .filter(|prefix| !prefix.is_empty() && *prefix != "xml")
}

fn xml_prefix_is_bound(namespaces: &BTreeMap<String, String>, prefix: &str) -> bool {
    namespaces
        .get(prefix)
        .is_some_and(|namespace| !namespace.trim().is_empty())
}

fn xml_attribute_is_namespace_declaration(qualified_name: &str) -> bool {
    qualified_name == "xmlns" || qualified_name.starts_with("xmlns:")
}

fn xslt_xml_error_diagnostic(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    error: &quick_xml::Error,
) -> Diagnostic {
    xslt_not_well_formed_diagnostic(
        request,
        source,
        byte_offset,
        format!("XSLT XML parse error: {error}"),
    )
}

fn xslt_not_well_formed_diagnostic(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    message: String,
) -> Diagnostic {
    xslt_diagnostic(
        request,
        source,
        byte_offset,
        "cem.xslt.not_well_formed_xml",
        Severity::Error,
        message,
    )
}

fn xslt_diagnostic(
    request: &XsltSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    code: &'static str,
    severity: Severity,
    message: String,
) -> Diagnostic {
    let (line, column) = byte_offset
        .and_then(|offset| usize::try_from(offset).ok())
        .map(|offset| line_col(source, offset))
        .map(|(line, column)| (Some(line), Some(column)))
        .unwrap_or((None, None));
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        line,
        column,
        byte_offset,
        code: code.to_owned(),
        severity,
        message,
        ..Diagnostic::default()
    }
}

fn xml_initial_namespaces() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "xml".to_owned(),
            "http://www.w3.org/XML/1998/namespace".to_owned(),
        ),
        (
            "xmlns".to_owned(),
            "http://www.w3.org/2000/xmlns/".to_owned(),
        ),
    ])
}

fn xml_event_position(
    reader: &quick_xml::Reader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
    empty: bool,
) -> Option<u64> {
    let markup_overhead = if empty { 3 } else { 2 };
    reader
        .buffer_position()
        .checked_sub(start.as_ref().len() as u64 + markup_overhead)
}

fn xml_bytes_are_whitespace(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .map(|value| value.chars().all(char::is_whitespace))
        .unwrap_or(false)
}

fn qname_display(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn line_col(source: &str, byte_offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut column = 1u32;
    let limit = byte_offset.min(source.len());
    for byte in source[..limit].bytes() {
        if byte == b'\n' {
            line = line.saturating_add(1);
            column = 1;
        } else {
            column = column.saturating_add(1);
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(source: &str, content_type: &str) -> Vec<Diagnostic> {
        validate_xslt_source_bytes(XsltSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.xsl",
            content_type: Some(content_type),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn xslt_source_validator_accepts_basic_stylesheet() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/"><main/></xsl:template>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn xslt_source_validator_accepts_custom_element_alias_stylesheet() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/"><article><xsl:if test="$ready"><button>Continue</button></xsl:if></article></xsl:template>
</xsl:stylesheet>
"#,
            "custom-element-xslt",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn xslt_source_validator_accepts_custom_element_fragment_alias() {
        let diagnostics = validate(
            r#"<article><button>Continue</button></article>"#,
            "custom-element-xslt",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn xslt_source_validator_reports_missing_namespace() {
        let diagnostics = validate(
            r#"<stylesheet version="1.0">
  <template match="/"><main/></template>
</stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "cem.xslt.namespace_missing"));
    }

    #[test]
    fn xslt_source_validator_reports_missing_version() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
  <xsl:template match="/"><main/></xsl:template>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "cem.xslt.version_missing"));
    }

    #[test]
    fn xslt_source_validator_reports_malformed_version() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0.0">
  <xsl:template match="/"><main/></xsl:template>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "cem.xslt.version_malformed"));
    }

    #[test]
    fn xslt_source_validator_reports_unsupported_version() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="4.0">
  <xsl:template match="/"><main/></xsl:template>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "cem.xslt.unsupported_version"));
    }

    #[test]
    fn xslt_source_validator_reports_external_uri_rejected() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:include href="shared/base.xsl"/>
  <xsl:template match="/"><main/></xsl:template>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "cem.xslt.external_uri_rejected"));
    }

    #[test]
    fn xslt_source_validator_reports_missing_entrypoint() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:output method="html"/>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "cem.xslt.entrypoint_missing"));
    }

    #[test]
    fn xslt_source_validator_reports_unsupported_construct_warning() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:msxsl="urn:schemas-microsoft-com:xslt" version="1.0">
  <xsl:template match="/">
    <msxsl:script language="JScript">function run(){return 1;}</msxsl:script>
  </xsl:template>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "legacy_xslt.unsupported_construct"));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }

    #[test]
    fn xslt_source_validator_reports_not_well_formed_xml() {
        let diagnostics = validate(
            r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/">
    <main>
  </xsl:template>
</xsl:stylesheet>
"#,
            "application/xslt+xml",
        );

        assert!(has_code(&diagnostics, "cem.xslt.not_well_formed_xml"));
    }

    #[test]
    fn xslt_ast_preserves_media_type_version_xpath_lexemes_and_source_maps() {
        let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:template match="/"><main><xsl:value-of select="catalog/title"/></main></xsl:template>
</xsl:stylesheet>
"#;
        let (stylesheet, diagnostics) =
            xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.xsl",
                content_type: Some("text/xsl; charset=UTF-8"),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let stylesheet = stylesheet.expect("typed XSLT stylesheet");
        assert_eq!(stylesheet.version.as_deref(), Some("2.0"));
        assert_eq!(
            stylesheet.xml_document.source.media_type,
            XSLT_TEXT_CONTENT_TYPE
        );
        assert_eq!(
            stylesheet
                .xml_document
                .source
                .parameters
                .get("charset")
                .map(String::as_str),
            Some("UTF-8")
        );
        assert!(stylesheet
            .facts
            .iter()
            .any(|fact| fact.kind == XsltFactKind::XPathObserved
                && fact.value.as_deref() == Some("catalog/title")));
        assert!(stylesheet
            .xml_document
            .events
            .iter()
            .all(|event| event.source_range.byte_length > 0));
        let subject = stylesheet.to_cemt_subject();
        assert_eq!(subject["kind"], json!("xslt-stylesheet"));
        assert_eq!(subject["version"], json!("2.0"));
        assert_eq!(
            subject["events"][0]["sourceMap"]["frames"][0]["transform"]["content_type"],
            json!(XSLT_TEXT_CONTENT_TYPE)
        );
    }

    #[test]
    fn xslt_source_validator_reports_schema_bound_uri_and_extension_facts() {
        for (source, code) in [
            (
                r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0"><xsl:template match="/"><xsl:value-of select="document('catalog.xml')/catalog/title"/></xsl:template></xsl:stylesheet>"#,
                "cem.xslt.external_uri_rejected",
            ),
            (
                r#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:exsl="http://exslt.org/common" version="1.0"><xsl:template match="/"><xsl:value-of select="exsl:node-set($items)"/></xsl:template></xsl:stylesheet>"#,
                "legacy_xslt.unsupported_construct",
            ),
        ] {
            let diagnostics = validate(source, XSLT_CONTENT_TYPE);
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .unwrap_or_else(|| panic!("missing {code}: {diagnostics:?}"));
            assert_eq!(
                diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("xslt"))
                    .and_then(|details| details.get("behavior")),
                Some(&json!(XSLT_FACT_BEHAVIOR))
            );
            assert!(diagnostic.source_map.is_some());
        }
    }

    #[test]
    fn xslt_inherits_xml_doctype_and_entity_safety_policy() {
        let diagnostics = validate(
            r#"<!DOCTYPE xsl:stylesheet [<!ENTITY remote SYSTEM "https://example.test/entity">]><xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0"><xsl:template match="/"><main>&remote;</main></xsl:template></xsl:stylesheet>"#,
            XSLT_CONTENT_TYPE,
        );
        assert!(has_code(&diagnostics, "cem.xslt.dtd_rejected"));
        assert!(has_code(&diagnostics, "cem.xslt.external_entity_rejected"));
    }
}
