use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{
    content_type_essence, HTML_CONTENT_TYPE, HTML_NAMESPACE_URI, HTML_SCHEMA_URI,
    MATHML_NAMESPACE_URI, SVG_NAMESPACE_URI,
};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, BytesSource, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::tokenizer::html::HtmlTokenizer;
use crate::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const HTML_PACKAGE_ID: &str = "html";
const HTML_FACT_BEHAVIOR: &str = "html-report-fact";

#[derive(Debug, Clone, Copy)]
pub struct HtmlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlDocumentAst {
    pub source: HtmlDocumentSource,
    pub mode: HtmlDocumentMode,
    pub encoding_report: HtmlEncodingReportAst,
    pub events: Vec<HtmlEventAst>,
    pub facts: Vec<HtmlFact>,
    pub line_ending: Option<String>,
    pub recovery_count: usize,
}

impl HtmlDocumentAst {
    #[cfg(test)]
    pub fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": "html-document",
            "contentType": self.source.media_type,
            "schema": HTML_SCHEMA_URI,
            "category": "html-document",
            "source": self.source.to_cemt_subject(),
            "documentMode": self.mode.as_str(),
            "encodingReport": self.encoding_report.to_cemt_subject(),
            "parseFacts": self
                .facts
                .iter()
                .map(HtmlFact::to_cemt_subject)
                .collect::<Vec<_>>(),
            "events": self
                .events
                .iter()
                .map(HtmlEventAst::to_cemt_subject)
                .collect::<Vec<_>>(),
            "lineEnding": self.line_ending,
            "recoveryCount": self.recovery_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlDocumentSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

impl HtmlDocumentSource {
    fn from_request(
        request: HtmlSourceValidationRequest<'_>,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        let content_type = request.content_type.unwrap_or(HTML_CONTENT_TYPE);
        Self {
            uri: request.source_uri.to_owned(),
            content_type: content_type.to_owned(),
            media_type: content_type_essence(content_type),
            parameters,
            byte_length: request.bytes.len(),
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
pub enum HtmlDocumentMode {
    Document,
    Fragment,
}

impl HtmlDocumentMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Fragment => "fragment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlEncodingReportAst {
    pub mime_charset: Option<String>,
    pub meta_charset: Option<String>,
    pub normalized_encoding: String,
    pub decoder_status: String,
}

impl HtmlEncodingReportAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "mimeCharset": self.mime_charset,
            "metaCharset": self.meta_charset,
            "normalizedEncoding": self.normalized_encoding,
            "decoderStatus": self.decoder_status,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlNamespace {
    Html,
    Svg,
    MathMl,
}

impl HtmlNamespace {
    pub fn uri(self) -> &'static str {
        match self {
            Self::Html => HTML_NAMESPACE_URI,
            Self::Svg => SVG_NAMESPACE_URI,
            Self::MathMl => MATHML_NAMESPACE_URI,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Svg => "svg",
            Self::MathMl => "mathml",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlEventAst {
    pub index: usize,
    pub kind: HtmlEventKind,
    pub depth: usize,
    pub lexical_name: Option<String>,
    pub local_name: Option<String>,
    pub namespace: HtmlNamespace,
    pub namespace_uri: String,
    pub attributes: Vec<HtmlAttributeAst>,
    pub value: Option<String>,
    pub lexeme: String,
    pub whitespace_only: bool,
    pub self_closing: bool,
    pub void_element: bool,
    pub recovered: bool,
    pub source_range: HtmlSourceRange,
}

impl HtmlEventAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "index": self.index,
            "kind": self.kind.as_str(),
            "depth": self.depth,
            "lexicalName": self.lexical_name,
            "localName": self.local_name,
            "namespace": self.namespace.as_str(),
            "namespaceUri": self.namespace_uri,
            "attributes": self
                .attributes
                .iter()
                .map(HtmlAttributeAst::to_cemt_subject)
                .collect::<Vec<_>>(),
            "value": self.value,
            "lexeme": self.lexeme,
            "whitespaceOnly": self.whitespace_only,
            "selfClosing": self.self_closing,
            "voidElement": self.void_element,
            "recovered": self.recovered,
            "sourceRange": self.source_range.to_cemt_subject(),
            "sourceMap": serde_json::to_value(self.source_range.source_map())
                .unwrap_or(Value::Null),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlEventKind {
    Doctype,
    StartElement,
    EndElement,
    Text,
    RawText,
    Rcdata,
    Comment,
}

impl HtmlEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Doctype => "doctype",
            Self::StartElement => "start-element",
            Self::EndElement => "end-element",
            Self::Text => "text",
            Self::RawText => "raw-text",
            Self::Rcdata => "rcdata",
            Self::Comment => "comment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlAttributeAst {
    pub lexical_name: String,
    pub local_name: String,
    pub value: Option<String>,
    pub lexeme: String,
    pub duplicate: bool,
    pub source_range: HtmlSourceRange,
}

impl HtmlAttributeAst {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "lexicalName": self.lexical_name,
            "localName": self.local_name,
            "value": self.value,
            "lexeme": self.lexeme,
            "duplicate": self.duplicate,
            "sourceRange": self.source_range.to_cemt_subject(),
            "sourceMap": serde_json::to_value(self.source_range.source_map())
                .unwrap_or(Value::Null),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HtmlSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HtmlSourceRange {
    pub start: HtmlSourcePosition,
    pub byte_length: u64,
}

impl HtmlSourceRange {
    fn from_offsets(line_index: &LineIndex, start: usize, end: usize) -> Self {
        let coordinate = line_index.project(start as u64);
        Self {
            start: HtmlSourcePosition {
                line: coordinate.line,
                column: coordinate.column,
                byte_offset: start as u64,
            },
            byte_length: end.saturating_sub(start) as u64,
        }
    }

    fn from_byte_range(line_index: &LineIndex, range: ByteRange) -> Self {
        Self::from_offsets(line_index, range.start as usize, range.end() as usize)
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
                    content_type: HTML_CONTENT_TYPE.to_owned(),
                },
            }],
        }
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
}

fn html_source_range_diagnostic_value(range: HtmlSourceRange) -> Value {
    json!({
        "byteOffset": range.start.byte_offset,
        "byteLength": range.byte_length,
        "line": range.start.line,
        "column": range.start.column,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HtmlFactKind {
    ParseError,
    UnsupportedEncoding,
    EncodingConflict,
    InvalidDoctype,
    QuirksMode,
    InvalidNestingRecovered,
    DuplicateAttribute,
    ScriptRejected,
    EventHandlerRejected,
    ExternalResourceRejected,
    CustomElementNameInvalid,
    ForeignContentUnregistered,
    SourceMapUnavailable,
    DocumentObserved,
    FragmentObserved,
    DoctypeObserved,
    EncodingObserved,
    RecoveryObserved,
    ForeignContentObserved,
    RawTextObserved,
    VoidElementObserved,
    CustomElementObserved,
}

impl HtmlFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseError => "parse-error",
            Self::UnsupportedEncoding => "unsupported-encoding",
            Self::EncodingConflict => "encoding-conflict",
            Self::InvalidDoctype => "invalid-doctype",
            Self::QuirksMode => "quirks-mode",
            Self::InvalidNestingRecovered => "invalid-nesting-recovered",
            Self::DuplicateAttribute => "duplicate-attribute",
            Self::ScriptRejected => "script-rejected",
            Self::EventHandlerRejected => "event-handler-rejected",
            Self::ExternalResourceRejected => "external-resource-rejected",
            Self::CustomElementNameInvalid => "custom-element-name-invalid",
            Self::ForeignContentUnregistered => "foreign-content-unregistered",
            Self::SourceMapUnavailable => "source-map-unavailable",
            Self::DocumentObserved => "document-observed",
            Self::FragmentObserved => "fragment-observed",
            Self::DoctypeObserved => "doctype-observed",
            Self::EncodingObserved => "encoding-observed",
            Self::RecoveryObserved => "recovery-observed",
            Self::ForeignContentObserved => "foreign-content-observed",
            Self::RawTextObserved => "raw-text-observed",
            Self::VoidElementObserved => "void-element-observed",
            Self::CustomElementObserved => "custom-element-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlFact {
    pub kind: HtmlFactKind,
    pub source_range: Option<HtmlSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

impl HtmlFact {
    #[cfg(test)]
    fn to_cemt_subject(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "sourceRange": self.source_range.map(HtmlSourceRange::to_cemt_subject),
            "message": self.message,
            "value": self.value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HtmlDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlSchemaContractCatalog {
    fact_bindings: BTreeMap<String, HtmlDiagnosticBinding>,
}

impl HtmlSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<HtmlSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(HTML_PACKAGE_ID)
                .expect("built-in HTML schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(HTML_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != HTML_FACT_BEHAVIOR {
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
                    HtmlDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: HtmlFactKind) -> Option<&HtmlDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

pub fn validate_html_source_bytes(request: HtmlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let (_, diagnostics) = html_document_ast_from_source_bytes(request);
    diagnostics
}

pub fn html_document_ast_from_source_bytes(
    request: HtmlSourceValidationRequest<'_>,
) -> (Option<HtmlDocumentAst>, Vec<Diagnostic>) {
    let parameters =
        parse_content_type_parameters(request.content_type.unwrap_or(HTML_CONTENT_TYPE));
    let source_info = HtmlDocumentSource::from_request(request, parameters.clone());
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            let fact = HtmlFact {
                kind: HtmlFactKind::UnsupportedEncoding,
                source_range: None,
                message: format!(
                    "HTML document bytes are not valid UTF-8 at byte {}",
                    error.valid_up_to()
                ),
                value: parameters.get("charset").cloned(),
            };
            let diagnostics = html_fact_diagnostics(
                request.source_uri,
                &source_info.media_type,
                &[fact],
                HtmlSchemaContractCatalog::from_builtin(),
            );
            return (None, diagnostics);
        }
    };

    let line_index = LineIndex::from_utf8(source);
    let mut tokenizer =
        HtmlTokenizer::from_document_source(BytesSource::new(SourceId(1), request.bytes.to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    let tokenizer_diagnostics = tokenizer.take_diagnostics();
    let mut facts = tokenizer_diagnostics
        .iter()
        .map(|diagnostic| HtmlFact {
            kind: HtmlFactKind::ParseError,
            source_range: diagnostic.byte_offset.map(|offset| {
                HtmlSourceRange::from_offsets(&line_index, offset as usize, offset as usize)
            }),
            message: diagnostic.message.clone(),
            value: Some(diagnostic.code.clone()),
        })
        .collect::<Vec<_>>();
    let mime_charset = parameters.get("charset").cloned();
    let build = build_html_events(
        source,
        &line_index,
        &tokens,
        mime_charset.as_deref(),
        &mut facts,
    );
    let mode = if build.document_observed {
        HtmlDocumentMode::Document
    } else {
        HtmlDocumentMode::Fragment
    };
    facts.push(HtmlFact {
        kind: match mode {
            HtmlDocumentMode::Document => HtmlFactKind::DocumentObserved,
            HtmlDocumentMode::Fragment => HtmlFactKind::FragmentObserved,
        },
        source_range: build.events.first().map(|event| event.source_range),
        message: format!("HTML {} mode selected", mode.as_str()),
        value: Some(mode.as_str().to_owned()),
    });
    if mode == HtmlDocumentMode::Document && !build.valid_doctype {
        push_fact_once(
            &mut facts,
            HtmlFactKind::QuirksMode,
            build.events.first().map(|event| event.source_range),
            "HTML document is interpreted in quirks mode without a valid HTML doctype",
            Some("quirks".to_owned()),
        );
    }

    let normalized_encoding = mime_charset
        .as_deref()
        .map(normalize_charset)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "utf-8".to_owned());
    facts.push(HtmlFact {
        kind: HtmlFactKind::EncodingObserved,
        source_range: build.meta_charset_range,
        message: format!("HTML decoder used `{normalized_encoding}` identity metadata"),
        value: Some(normalized_encoding.clone()),
    });
    let ast = HtmlDocumentAst {
        source: source_info,
        mode,
        encoding_report: HtmlEncodingReportAst {
            mime_charset,
            meta_charset: build.meta_charset,
            normalized_encoding,
            decoder_status: "decoded-utf8".to_owned(),
        },
        events: build.events,
        recovery_count: build.recovery_count,
        facts,
        line_ending: detect_line_ending(source),
    };
    let diagnostics = html_fact_diagnostics(
        request.source_uri,
        &ast.source.media_type,
        &ast.facts,
        HtmlSchemaContractCatalog::from_builtin(),
    );
    (Some(ast), diagnostics)
}

#[derive(Debug, Clone)]
struct HtmlElementFrame {
    local_name: String,
    namespace: HtmlNamespace,
    child_namespace: HtmlNamespace,
}

#[derive(Debug)]
struct HtmlEventBuild {
    events: Vec<HtmlEventAst>,
    document_observed: bool,
    valid_doctype: bool,
    meta_charset: Option<String>,
    meta_charset_range: Option<HtmlSourceRange>,
    recovery_count: usize,
}

fn build_html_events(
    source: &str,
    line_index: &LineIndex,
    tokens: &[SchemaToken],
    mime_charset: Option<&str>,
    facts: &mut Vec<HtmlFact>,
) -> HtmlEventBuild {
    let mut events = Vec::new();
    let mut stack = Vec::<HtmlElementFrame>::new();
    let mut document_observed = false;
    let mut valid_doctype = false;
    let mut meta_charset = None;
    let mut meta_charset_range = None;
    let mut recovery_count = 0usize;
    let mut index = 0usize;

    while index < tokens.len() {
        let token = &tokens[index];
        match &token.kind {
            SchemaTokenKind::ProcessingInstruction { target, data }
                if target.eq_ignore_ascii_case("doctype") =>
            {
                document_observed = true;
                let range = HtmlSourceRange::from_byte_range(line_index, token.byte_range);
                let lexeme = source_slice(source, token.byte_range).to_owned();
                let is_html = data.trim().eq_ignore_ascii_case("html");
                valid_doctype |= is_html;
                facts.push(HtmlFact {
                    kind: HtmlFactKind::DoctypeObserved,
                    source_range: Some(range),
                    message: format!("HTML doctype `{}` observed", data.trim()),
                    value: Some(data.trim().to_owned()),
                });
                if !is_html {
                    push_fact_once(
                        facts,
                        HtmlFactKind::InvalidDoctype,
                        Some(range),
                        "HTML doctype is not the no-quirks `html` doctype",
                        Some(data.trim().to_owned()),
                    );
                }
                events.push(HtmlEventAst {
                    index: events.len(),
                    kind: HtmlEventKind::Doctype,
                    depth: 0,
                    lexical_name: Some(target.clone()),
                    local_name: Some("html".to_owned()),
                    namespace: HtmlNamespace::Html,
                    namespace_uri: HTML_NAMESPACE_URI.to_owned(),
                    attributes: Vec::new(),
                    value: Some(data.clone()),
                    lexeme,
                    whitespace_only: false,
                    self_closing: false,
                    void_element: false,
                    recovered: false,
                    source_range: range,
                });
            }
            SchemaTokenKind::Comment(value) => {
                let range = HtmlSourceRange::from_byte_range(line_index, token.byte_range);
                events.push(HtmlEventAst {
                    index: events.len(),
                    kind: HtmlEventKind::Comment,
                    depth: stack.len(),
                    lexical_name: None,
                    local_name: None,
                    namespace: stack
                        .last()
                        .map(|frame| frame.child_namespace)
                        .unwrap_or(HtmlNamespace::Html),
                    namespace_uri: stack
                        .last()
                        .map(|frame| frame.child_namespace.uri())
                        .unwrap_or(HTML_NAMESPACE_URI)
                        .to_owned(),
                    attributes: Vec::new(),
                    value: Some(value.clone()),
                    lexeme: source_slice(source, token.byte_range).to_owned(),
                    whitespace_only: false,
                    self_closing: false,
                    void_element: false,
                    recovered: false,
                    source_range: range,
                });
            }
            SchemaTokenKind::NodeStart { name } => {
                let tag_start = token.byte_range.start as usize;
                let tag_end = html_find_tag_end(source, tag_start)
                    .map(|end| end + 1)
                    .unwrap_or_else(|| token.byte_range.end() as usize);
                let tag_range = HtmlSourceRange::from_offsets(line_index, tag_start, tag_end);
                let lexeme = source
                    .get(tag_start..tag_end)
                    .unwrap_or_else(|| source_slice(source, token.byte_range))
                    .to_owned();
                let lexical_name = html_start_tag_lexical_name(&lexeme);
                let parent_namespace = stack
                    .last()
                    .map(|frame| frame.child_namespace)
                    .unwrap_or(HtmlNamespace::Html);
                let namespace = html_element_namespace(parent_namespace, name);
                let mut next = index + 1;
                let mut attributes = Vec::new();
                let mut names = BTreeSet::new();
                while let Some(attribute_token) = tokens.get(next) {
                    let SchemaTokenKind::Attribute { name, value, .. } = &attribute_token.kind
                    else {
                        break;
                    };
                    let attribute_range =
                        HtmlSourceRange::from_byte_range(line_index, attribute_token.byte_range);
                    let attribute_lexeme =
                        source_slice(source, attribute_token.byte_range).to_owned();
                    let duplicate = !names.insert(name.clone());
                    if duplicate {
                        push_fact_once(
                            facts,
                            HtmlFactKind::DuplicateAttribute,
                            Some(attribute_range),
                            "HTML parser ignored a duplicate ASCII case-folded attribute name",
                            Some(name.clone()),
                        );
                    }
                    attributes.push(HtmlAttributeAst {
                        lexical_name: html_attribute_lexical_name(&attribute_lexeme),
                        local_name: name.clone(),
                        value: value.clone(),
                        lexeme: attribute_lexeme,
                        duplicate,
                        source_range: attribute_range,
                    });
                    next += 1;
                }
                let self_closing = html_start_tag_is_self_closing(&lexeme);
                let void_element = html_is_void_element(name, namespace);
                let child_namespace = html_child_namespace(namespace, name, &attributes);
                if namespace == HtmlNamespace::Html && name == "html" {
                    document_observed = true;
                }
                validate_html_start_element(
                    name,
                    &lexical_name,
                    namespace,
                    &attributes,
                    mime_charset,
                    tag_range,
                    facts,
                    &mut meta_charset,
                    &mut meta_charset_range,
                );
                if namespace != HtmlNamespace::Html {
                    facts.push(HtmlFact {
                        kind: HtmlFactKind::ForeignContentObserved,
                        source_range: Some(tag_range),
                        message: format!(
                            "HTML parser entered {} foreign content for `{name}`",
                            namespace.as_str()
                        ),
                        value: Some(namespace.uri().to_owned()),
                    });
                }
                if void_element {
                    facts.push(HtmlFact {
                        kind: HtmlFactKind::VoidElementObserved,
                        source_range: Some(tag_range),
                        message: format!("HTML void element `{name}` observed"),
                        value: Some(name.clone()),
                    });
                }
                events.push(HtmlEventAst {
                    index: events.len(),
                    kind: HtmlEventKind::StartElement,
                    depth: stack.len(),
                    lexical_name: Some(lexical_name),
                    local_name: Some(name.clone()),
                    namespace,
                    namespace_uri: namespace.uri().to_owned(),
                    attributes,
                    value: None,
                    lexeme,
                    whitespace_only: false,
                    self_closing,
                    void_element,
                    recovered: false,
                    source_range: tag_range,
                });
                if !self_closing && !void_element {
                    stack.push(HtmlElementFrame {
                        local_name: name.clone(),
                        namespace,
                        child_namespace,
                    });
                }
                index = next.saturating_sub(1);
            }
            SchemaTokenKind::NodeEnd { name: Some(name) } => {
                let lexeme = source_slice(source, token.byte_range).to_owned();
                if lexeme == ">" || lexeme == "/>" {
                    index += 1;
                    continue;
                }
                let range = HtmlSourceRange::from_byte_range(line_index, token.byte_range);
                let lexical_name = html_end_tag_lexical_name(&lexeme);
                let (depth, namespace, recovered) = if let Some(position) =
                    stack.iter().rposition(|frame| frame.local_name == *name)
                {
                    let frame = stack[position].clone();
                    let recovered = position + 1 != stack.len();
                    if recovered {
                        recovery_count += 1;
                        facts.push(HtmlFact {
                            kind: HtmlFactKind::RecoveryObserved,
                            source_range: Some(range),
                            message: format!(
                                "HTML parser implicitly closed {} element(s) before `</{name}>`",
                                stack.len().saturating_sub(position + 1)
                            ),
                            value: Some(name.clone()),
                        });
                    }
                    stack.truncate(position);
                    (position, frame.namespace, recovered)
                } else {
                    recovery_count += 1;
                    let namespace = stack
                        .last()
                        .map(|frame| frame.child_namespace)
                        .unwrap_or(HtmlNamespace::Html);
                    push_fact_once(
                        facts,
                        HtmlFactKind::InvalidNestingRecovered,
                        Some(range),
                        &format!("HTML parser recovered unmatched closing tag `</{name}>`"),
                        Some(name.clone()),
                    );
                    (stack.len(), namespace, true)
                };
                events.push(HtmlEventAst {
                    index: events.len(),
                    kind: HtmlEventKind::EndElement,
                    depth,
                    lexical_name: Some(lexical_name),
                    local_name: Some(name.clone()),
                    namespace,
                    namespace_uri: namespace.uri().to_owned(),
                    attributes: Vec::new(),
                    value: None,
                    lexeme,
                    whitespace_only: false,
                    self_closing: false,
                    void_element: false,
                    recovered,
                    source_range: range,
                });
            }
            SchemaTokenKind::Text(value) | SchemaTokenKind::Trivia(value) => {
                let range = HtmlSourceRange::from_byte_range(line_index, token.byte_range);
                let parent = stack.last();
                let kind = match parent.map(|frame| frame.local_name.as_str()) {
                    Some("script" | "style")
                        if parent.is_some_and(|frame| frame.namespace == HtmlNamespace::Html) =>
                    {
                        HtmlEventKind::RawText
                    }
                    Some("textarea" | "title")
                        if parent.is_some_and(|frame| frame.namespace == HtmlNamespace::Html) =>
                    {
                        HtmlEventKind::Rcdata
                    }
                    _ => HtmlEventKind::Text,
                };
                if matches!(kind, HtmlEventKind::RawText | HtmlEventKind::Rcdata) {
                    facts.push(HtmlFact {
                        kind: HtmlFactKind::RawTextObserved,
                        source_range: Some(range),
                        message: format!("HTML {} parser state preserved", kind.as_str()),
                        value: parent.map(|frame| frame.local_name.clone()),
                    });
                }
                let namespace = parent
                    .map(|frame| frame.child_namespace)
                    .unwrap_or(HtmlNamespace::Html);
                events.push(HtmlEventAst {
                    index: events.len(),
                    kind,
                    depth: stack.len(),
                    lexical_name: None,
                    local_name: None,
                    namespace,
                    namespace_uri: namespace.uri().to_owned(),
                    attributes: Vec::new(),
                    value: Some(value.clone()),
                    lexeme: source_slice(source, token.byte_range).to_owned(),
                    whitespace_only: value.trim().is_empty(),
                    self_closing: false,
                    void_element: false,
                    recovered: false,
                    source_range: range,
                });
            }
            SchemaTokenKind::Error { code } => {
                push_fact_once(
                    facts,
                    HtmlFactKind::ParseError,
                    Some(HtmlSourceRange::from_byte_range(
                        line_index,
                        token.byte_range,
                    )),
                    &format!("HTML tokenizer emitted error `{code}`"),
                    Some(code.clone()),
                );
            }
            SchemaTokenKind::Attribute { .. }
            | SchemaTokenKind::NodeEnd { name: None }
            | SchemaTokenKind::ExpressionNode(_)
            | SchemaTokenKind::AnonymousScopeStart
            | SchemaTokenKind::Directive { .. }
            | SchemaTokenKind::RichContent { .. }
            | SchemaTokenKind::ProcessingInstruction { .. } => {}
        }
        index += 1;
    }

    HtmlEventBuild {
        events,
        document_observed,
        valid_doctype,
        meta_charset,
        meta_charset_range,
        recovery_count,
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_html_start_element(
    local_name: &str,
    lexical_name: &str,
    namespace: HtmlNamespace,
    attributes: &[HtmlAttributeAst],
    mime_charset: Option<&str>,
    range: HtmlSourceRange,
    facts: &mut Vec<HtmlFact>,
    meta_charset: &mut Option<String>,
    meta_charset_range: &mut Option<HtmlSourceRange>,
) {
    if local_name.contains(':') {
        push_fact_once(
            facts,
            HtmlFactKind::ForeignContentUnregistered,
            Some(range),
            &format!(
                "HTML tag `{lexical_name}` is outside parser-default HTML, SVG, and MathML namespaces"
            ),
            Some(lexical_name.to_owned()),
        );
    }
    if namespace == HtmlNamespace::Html && local_name == "meta" {
        if let Some(value) = html_meta_charset(attributes) {
            if meta_charset.is_none() {
                *meta_charset = Some(value.clone());
                *meta_charset_range = Some(range);
            }
            if let Some(mime_charset) = mime_charset {
                if normalize_charset(mime_charset) != normalize_charset(&value) {
                    push_fact_once(
                        facts,
                        HtmlFactKind::EncodingConflict,
                        Some(range),
                        &format!(
                            "HTML MIME charset `{mime_charset}` conflicts with meta charset `{value}`"
                        ),
                        Some(format!("{mime_charset}|{value}")),
                    );
                }
            }
        }
    }
    if namespace == HtmlNamespace::Html
        && local_name == "script"
        && html_script_is_executable(attributes)
    {
        push_fact_once(
            facts,
            HtmlFactKind::ScriptRejected,
            Some(range),
            "Executable HTML script is rejected unless an explicit host capability enables it",
            Some(local_name.to_owned()),
        );
    }
    if let Some(attribute) = attributes
        .iter()
        .find(|attribute| attribute.local_name.len() > 2 && attribute.local_name.starts_with("on"))
    {
        push_fact_once(
            facts,
            HtmlFactKind::EventHandlerRejected,
            Some(attribute.source_range),
            "Inline HTML event handlers are rejected unless an explicit host capability enables them",
            Some(attribute.local_name.clone()),
        );
    }
    if html_start_tag_requires_resource_policy(local_name, namespace, attributes) {
        push_fact_once(
            facts,
            HtmlFactKind::ExternalResourceRejected,
            Some(range),
            "HTML, SVG, or MathML external resource access requires an explicit resolver policy",
            Some(local_name.to_owned()),
        );
    }
    if namespace == HtmlNamespace::Html {
        let custom_name = if local_name.contains('-') {
            Some((lexical_name, local_name))
        } else {
            html_attribute_value(attributes, "is").map(|value| (value, value))
        };
        if let Some((lexical, semantic)) = custom_name {
            facts.push(HtmlFact {
                kind: HtmlFactKind::CustomElementObserved,
                source_range: Some(range),
                message: format!("HTML custom element name `{lexical}` observed"),
                value: Some(lexical.to_owned()),
            });
            if lexical != semantic.to_ascii_lowercase()
                || !html_custom_element_name_is_valid(semantic)
            {
                push_fact_once(
                    facts,
                    HtmlFactKind::CustomElementNameInvalid,
                    Some(range),
                    "HTML custom element names must contain a hyphen and use a source-stable lowercase name",
                    Some(lexical.to_owned()),
                );
            }
        }
    }
}

fn html_fact_diagnostics(
    source_uri: &str,
    media_type: &str,
    facts: &[HtmlFact],
    catalog: &HtmlSchemaContractCatalog,
) -> Vec<Diagnostic> {
    facts
        .iter()
        .filter_map(|fact| {
            let binding = catalog.binding_for_fact(fact.kind)?;
            let (line, column, byte_offset) = fact
                .source_range
                .map(|range| {
                    (
                        Some(range.start.line),
                        Some(range.start.column),
                        Some(range.start.byte_offset),
                    )
                })
                .unwrap_or((None, None, None));
            Some(Diagnostic {
                uri: Some(source_uri.to_owned()),
                line,
                column,
                byte_offset,
                code: binding.diagnostic_code.clone(),
                severity: binding.severity,
                message: fact.message.clone(),
                details: Some(json!({
                    "schema": HTML_SCHEMA_URI,
                    "schemaPackage": HTML_PACKAGE_ID,
                    "schemaConstraint": binding.contract,
                    "schemaBehavior": binding.behavior,
                    "schemaPolicy": binding.policy,
                    "factKind": fact.kind.as_str(),
                    "factValue": fact.value,
                    "contentType": media_type,
                    "sourceRange": fact.source_range.map(html_source_range_diagnostic_value),
                })),
                source_map: fact.source_range.map(HtmlSourceRange::source_map),
                ..Diagnostic::default()
            })
        })
        .collect()
}

fn push_fact_once(
    facts: &mut Vec<HtmlFact>,
    kind: HtmlFactKind,
    source_range: Option<HtmlSourceRange>,
    message: &str,
    value: Option<String>,
) {
    if facts.iter().any(|fact| fact.kind == kind) {
        return;
    }
    facts.push(HtmlFact {
        kind,
        source_range,
        message: message.to_owned(),
        value,
    });
}

fn html_find_tag_end(source: &str, tag_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut offset = tag_start.saturating_add(1);
    let mut quote = None;
    while offset < bytes.len() {
        let byte = bytes[offset];
        match (quote, byte) {
            (Some(expected), actual) if actual == expected => quote = None,
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return Some(offset),
            _ => {}
        }
        offset += 1;
    }
    None
}

fn html_start_tag_lexical_name(lexeme: &str) -> String {
    lexeme
        .strip_prefix('<')
        .unwrap_or(lexeme)
        .trim_start()
        .split(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '/' | '>'))
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn html_end_tag_lexical_name(lexeme: &str) -> String {
    lexeme
        .strip_prefix("</")
        .unwrap_or(lexeme)
        .trim_start()
        .split(|ch: char| ch.is_ascii_whitespace() || ch == '>')
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn html_attribute_lexical_name(lexeme: &str) -> String {
    lexeme
        .trim_start()
        .split(|ch: char| ch.is_ascii_whitespace() || ch == '=')
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn html_start_tag_is_self_closing(lexeme: &str) -> bool {
    lexeme
        .strip_suffix('>')
        .unwrap_or(lexeme)
        .trim_end()
        .ends_with('/')
}

fn html_element_namespace(
    parent_child_namespace: HtmlNamespace,
    local_name: &str,
) -> HtmlNamespace {
    match (parent_child_namespace, local_name) {
        (HtmlNamespace::Html, "svg") => HtmlNamespace::Svg,
        (HtmlNamespace::Html, "math") => HtmlNamespace::MathMl,
        (namespace, _) => namespace,
    }
}

fn html_child_namespace(
    namespace: HtmlNamespace,
    local_name: &str,
    attributes: &[HtmlAttributeAst],
) -> HtmlNamespace {
    match (namespace, local_name) {
        (HtmlNamespace::Svg, "foreignobject") => HtmlNamespace::Html,
        (HtmlNamespace::MathMl, "annotation-xml") if html_annotation_is_html(attributes) => {
            HtmlNamespace::Html
        }
        _ => namespace,
    }
}

fn html_annotation_is_html(attributes: &[HtmlAttributeAst]) -> bool {
    html_attribute_value(attributes, "encoding").is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "text/html" | "application/xhtml+xml"
        )
    })
}

fn html_meta_charset(attributes: &[HtmlAttributeAst]) -> Option<String> {
    if let Some(value) = html_attribute_value(attributes, "charset") {
        return Some(value.trim().to_owned());
    }
    let http_equiv = html_attribute_value(attributes, "http-equiv")?;
    if !http_equiv.eq_ignore_ascii_case("content-type") {
        return None;
    }
    let content = html_attribute_value(attributes, "content")?;
    content.split(';').skip(1).find_map(|part| {
        let (key, value) = part.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("charset")
            .then(|| value.trim().trim_matches('"').trim_matches('\'').to_owned())
    })
}

fn normalize_charset(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
}

fn html_script_is_executable(attributes: &[HtmlAttributeAst]) -> bool {
    let Some(script_type) = html_attribute_value(attributes, "type") else {
        return true;
    };
    let script_type = script_type.trim().to_ascii_lowercase();
    !(script_type.is_empty()
        || matches!(
            script_type.as_str(),
            "application/json"
                | "application/ld+json"
                | "importmap"
                | "speculationrules"
                | "text/plain"
        ))
}

fn html_start_tag_requires_resource_policy(
    local_name: &str,
    namespace: HtmlNamespace,
    attributes: &[HtmlAttributeAst],
) -> bool {
    attributes.iter().any(|attribute| match namespace {
        HtmlNamespace::Html => html_attribute_requires_resource_policy(local_name, attribute),
        HtmlNamespace::Svg => {
            matches!(attribute.local_name.as_str(), "href" | "src")
                && html_uri_requires_policy(attribute.value.as_deref().unwrap_or_default())
                || html_css_url_reference_requires_policy(
                    attribute.value.as_deref().unwrap_or_default(),
                )
        }
        HtmlNamespace::MathMl => {
            local_name == "annotation"
                && attribute.local_name == "src"
                && html_uri_requires_policy(attribute.value.as_deref().unwrap_or_default())
        }
    })
}

fn html_attribute_requires_resource_policy(local_name: &str, attribute: &HtmlAttributeAst) -> bool {
    let value = attribute.value.as_deref().unwrap_or_default();
    match attribute.local_name.as_str() {
        "src" | "poster" | "action" => html_uri_requires_policy(value),
        "srcset" => html_srcset_requires_policy(value),
        "href" if matches!(local_name, "link" | "base") => html_uri_requires_policy(value),
        "style" => html_css_url_reference_requires_policy(value),
        _ => false,
    }
}

fn html_uri_requires_policy(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    !(trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.to_ascii_lowercase().starts_with("data:"))
}

fn html_srcset_requires_policy(value: &str) -> bool {
    value
        .split(',')
        .filter_map(|candidate| candidate.split_whitespace().next())
        .any(html_uri_requires_policy)
}

fn html_css_url_reference_requires_policy(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let mut search_start = 0usize;
    while let Some(relative_start) = lower[search_start..].find("url(") {
        let url_start = search_start + relative_start;
        let reference = value[url_start + 4..]
            .split(')')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if html_uri_requires_policy(reference) {
            return true;
        }
        search_start = url_start + 4;
    }
    false
}

fn html_custom_element_name_is_valid(name: &str) -> bool {
    let name = name.trim();
    name.contains('-')
        && !name.starts_with("xml")
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && name
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn html_attribute_value<'a>(
    attributes: &'a [HtmlAttributeAst],
    local_name: &str,
) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| attribute.local_name == local_name && !attribute.duplicate)
        .and_then(|attribute| attribute.value.as_deref())
}

fn html_is_void_element(local_name: &str, namespace: HtmlNamespace) -> bool {
    namespace == HtmlNamespace::Html
        && matches!(
            local_name,
            "area"
                | "base"
                | "br"
                | "col"
                | "embed"
                | "hr"
                | "img"
                | "input"
                | "link"
                | "meta"
                | "param"
                | "source"
                | "track"
                | "wbr"
        )
}

fn parse_content_type_parameters(content_type: &str) -> BTreeMap<String, String> {
    content_type
        .split(';')
        .skip(1)
        .filter_map(|part| {
            let (name, value) = part.split_once('=')?;
            let name = name.trim().to_ascii_lowercase();
            (!name.is_empty()).then(|| {
                (
                    name,
                    value.trim().trim_matches('"').trim_matches('\'').to_owned(),
                )
            })
        })
        .collect()
}

fn source_slice(source: &str, range: ByteRange) -> &str {
    source
        .get(range.start as usize..range.end() as usize)
        .unwrap_or_default()
}

fn detect_line_ending(source: &str) -> Option<String> {
    if source.contains("\r\n") {
        Some("crlf".to_owned())
    } else if source.contains('\n') {
        Some("lf".to_owned())
    } else if source.contains('\r') {
        Some("cr".to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(source: &str) -> Vec<Diagnostic> {
        validate_html_source_bytes(HtmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.html",
            content_type: Some(HTML_CONTENT_TYPE),
        })
    }

    fn parse(source: &str, content_type: &str) -> (HtmlDocumentAst, Vec<Diagnostic>) {
        let (document, diagnostics) =
            html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: "fixture.html",
                content_type: Some(content_type),
            });
        (document.expect("HTML document AST"), diagnostics)
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn html_source_validator_accepts_basic_document() {
        let diagnostics = validate(
            r#"<!doctype html>
<!-- preserved -->
<html lang="en">
  <head><meta charset="utf-8"><title>Document</title></head>
  <body><main><h1>Welcome</h1><p>Hello.</p></main></body>
</html>
"#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn html_source_validator_accepts_fragment() {
        let diagnostics = validate(r#"<article><h2>Card</h2><p>Recovered fragment</article>"#);

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn html_source_validator_reports_script_and_event_handler_rejected() {
        let diagnostics = validate(
            r#"<!doctype html><html><body onload="start()"><script>if (a < b) alert("blocked")</script></body></html>"#,
        );

        assert!(has_code(&diagnostics, "cem.html.script_rejected"));
        assert!(has_code(&diagnostics, "cem.html.event_handler_rejected"));
    }

    #[test]
    fn html_source_validator_reports_external_resource_rejected() {
        let diagnostics = validate(
            r#"<!doctype html><html><body><img src="images/logo.png" alt="Logo"></body></html>"#,
        );

        assert!(has_code(
            &diagnostics,
            "cem.html.external_resource_rejected"
        ));
    }

    #[test]
    fn html_source_validator_accepts_local_fragment_form_action() {
        let diagnostics = validate(
            r##"<!doctype html><html><body><form method="post" action="#session"><button type="submit">Sign in</button></form></body></html>"##,
        );

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn html_source_validator_reports_invalid_custom_element_name() {
        let diagnostics =
            validate(r#"<!doctype html><html><body><X-Card>Broken</X-Card></body></html>"#);

        assert!(has_code(
            &diagnostics,
            "cem.html.custom_element_name_invalid"
        ));
    }

    #[test]
    fn html_source_validator_reports_encoding_conflict_warning() {
        let diagnostics = validate_html_source_bytes(HtmlSourceValidationRequest {
            bytes: br#"<!doctype html><html><head><meta charset="utf-8"><title>Encoding</title></head><body></body></html>"#,
            source_uri: "fixture.html",
            content_type: Some("text/html; charset=windows-1252"),
        });

        assert!(has_code(&diagnostics, "cem.html.encoding_conflict"));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }

    #[test]
    fn html_ast_preserves_identity_lexemes_ranges_comments_and_duplicates() {
        let source =
            "<!DOCTYPE html>\r\n<!--note--><HTML DATA-X='one' data-x=two><BODY><BR></BODY></HTML>";
        let (document, diagnostics) = parse(source, "text/html; charset=utf-8; profile=editor");

        assert!(has_code(&diagnostics, "cem.html.duplicate_attribute"));
        assert_eq!(document.mode, HtmlDocumentMode::Document);
        assert_eq!(document.source.media_type, HTML_CONTENT_TYPE);
        assert_eq!(document.source.parameters["profile"], "editor");
        assert_eq!(document.line_ending.as_deref(), Some("crlf"));
        assert!(document
            .events
            .iter()
            .any(|event| event.kind == HtmlEventKind::Comment && event.lexeme == "<!--note-->"));
        let html = document
            .events
            .iter()
            .find(|event| {
                event.kind == HtmlEventKind::StartElement
                    && event.local_name.as_deref() == Some("html")
            })
            .expect("html start event");
        assert_eq!(html.lexical_name.as_deref(), Some("HTML"));
        assert_eq!(html.local_name.as_deref(), Some("html"));
        assert_eq!(html.attributes[0].lexical_name, "DATA-X");
        assert!(html.attributes[1].duplicate);
        assert_eq!(html.source_range.start.byte_offset, 28);
        assert!(!html.source_range.source_map().frames.is_empty());
        let subject = document.to_cemt_subject();
        assert_eq!(subject["documentMode"], "document");
        assert!(subject["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["lexicalName"] == "HTML" && event["localName"] == "html"));
    }

    #[test]
    fn html_ast_preserves_raw_rcdata_void_and_foreign_content_boundaries() {
        let source = r#"<div><script>if (a < b) { x = "<p>"; }</script><textarea>a < b</textarea><br><svg viewBox="0 0 1 1"><foreignObject><P>x</P></foreignObject></svg><math><annotation-xml encoding="text/html"><B>y</B></annotation-xml></math></div>"#;
        let (document, diagnostics) = parse(source, HTML_CONTENT_TYPE);

        assert!(has_code(&diagnostics, "cem.html.script_rejected"));
        assert!(document
            .events
            .iter()
            .any(|event| { event.kind == HtmlEventKind::RawText && event.lexeme.contains("<p>") }));
        assert!(document
            .events
            .iter()
            .any(|event| event.kind == HtmlEventKind::Rcdata && event.lexeme == "a < b"));
        assert!(document
            .events
            .iter()
            .any(|event| { event.local_name.as_deref() == Some("br") && event.void_element }));
        let svg = document
            .events
            .iter()
            .find(|event| event.local_name.as_deref() == Some("svg"))
            .expect("svg event");
        assert_eq!(svg.namespace, HtmlNamespace::Svg);
        let foreign_html = document
            .events
            .iter()
            .find(|event| event.lexical_name.as_deref() == Some("P"))
            .expect("foreignObject HTML integration event");
        assert_eq!(foreign_html.namespace, HtmlNamespace::Html);
    }

    #[test]
    fn html_ast_records_recovery_without_xml_well_formedness_rules() {
        let (document, diagnostics) =
            parse("<article><p>text</article></missing>", HTML_CONTENT_TYPE);

        assert!(has_code(&diagnostics, "cem.html.invalid_nesting_recovered"));
        assert_eq!(document.mode, HtmlDocumentMode::Fragment);
        assert_eq!(document.recovery_count, 2);
        assert!(document
            .facts
            .iter()
            .any(|fact| fact.kind == HtmlFactKind::RecoveryObserved));
    }

    #[test]
    fn html_schema_contract_binds_every_reportable_fact() {
        let catalog = HtmlSchemaContractCatalog::from_builtin();
        for kind in [
            HtmlFactKind::ParseError,
            HtmlFactKind::UnsupportedEncoding,
            HtmlFactKind::EncodingConflict,
            HtmlFactKind::InvalidDoctype,
            HtmlFactKind::QuirksMode,
            HtmlFactKind::InvalidNestingRecovered,
            HtmlFactKind::DuplicateAttribute,
            HtmlFactKind::ScriptRejected,
            HtmlFactKind::EventHandlerRejected,
            HtmlFactKind::ExternalResourceRejected,
            HtmlFactKind::CustomElementNameInvalid,
            HtmlFactKind::ForeignContentUnregistered,
            HtmlFactKind::SourceMapUnavailable,
        ] {
            assert!(
                catalog.binding_for_fact(kind).is_some(),
                "missing schema binding for {}",
                kind.as_str()
            );
        }
    }

    #[test]
    fn html_source_validator_reports_non_utf8_input_through_schema_contract() {
        let diagnostics = validate_html_source_bytes(HtmlSourceValidationRequest {
            bytes: &[b'<', b'p', b'>', 0xff, b'<', b'/', b'p', b'>'],
            source_uri: "fixture.html",
            content_type: Some(HTML_CONTENT_TYPE),
        });

        assert!(has_code(&diagnostics, "cem.html.unsupported_encoding"));
        assert_eq!(
            diagnostics[0].details.as_ref().unwrap()["schema"],
            HTML_SCHEMA_URI
        );
    }
}
