use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::{content_type_essence, XSLT_NAMESPACE_URI};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct XsltSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_xslt_source_bytes(request: XsltSourceValidationRequest<'_>) -> Vec<Diagnostic> {
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
}
