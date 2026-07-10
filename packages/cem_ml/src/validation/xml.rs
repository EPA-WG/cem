use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::content_type_essence;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub struct XmlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_xml_source_bytes(request: XmlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => return vec![xml_unsupported_utf8_diagnostic(&request, &error)],
    };

    let mime_charset = request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "charset"));
    if let Some(charset) = mime_charset.as_deref() {
        if !xml_encoding_is_supported(charset) {
            return vec![xml_unsupported_encoding_diagnostic(
                &request,
                source,
                None,
                &format!("XML content-type charset `{charset}` is not supported"),
            )];
        }
    }

    match xml_source_kind(&request) {
        XmlSourceKind::Dtd => {
            if source.trim().is_empty() {
                Vec::new()
            } else {
                vec![xml_diagnostic(
                    &request,
                    source,
                    None,
                    "cem.xml.dtd_rejected",
                    Severity::Error,
                    "XML DTD resources are rejected until an explicit DTD policy enables them"
                        .to_owned(),
                )]
            }
        }
        XmlSourceKind::Document | XmlSourceKind::ExternalParsedEntity => {
            validate_xml_source(&request, source, mime_charset.as_deref())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum XmlSourceKind {
    Document,
    ExternalParsedEntity,
    Dtd,
}

fn xml_source_kind(request: &XmlSourceValidationRequest<'_>) -> XmlSourceKind {
    match request.content_type.map(content_type_essence).as_deref() {
        Some("application/xml-dtd") => XmlSourceKind::Dtd,
        Some("application/xml-external-parsed-entity")
        | Some("text/xml-external-parsed-entity") => XmlSourceKind::ExternalParsedEntity,
        _ => XmlSourceKind::Document,
    }
}

fn validate_xml_source(
    request: &XmlSourceValidationRequest<'_>,
    source: &str,
    mime_charset: Option<&str>,
) -> Vec<Diagnostic> {
    let kind = xml_source_kind(request);
    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;

    let mut element_stack: Vec<String> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut reported_multiple_roots = false;

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                if element_stack.is_empty() {
                    root_count += 1;
                    if kind == XmlSourceKind::Document && root_count > 1 && !reported_multiple_roots
                    {
                        diagnostics.push(xml_diagnostic(
                            request,
                            source,
                            xml_event_position(&reader, &start, false),
                            "cem.xml.parse_error",
                            Severity::Error,
                            "XML document must have exactly one document element".to_owned(),
                        ));
                        reported_multiple_roots = true;
                    }
                }

                let (next_namespaces, mut start_diagnostics) =
                    validate_xml_start_event(request, source, &start, &namespace_stack);
                diagnostics.append(&mut start_diagnostics);
                element_stack.push(xml_qname_display(start.name().as_ref()));
                namespace_stack.push(next_namespaces);
            }
            Ok(quick_xml::events::Event::Empty(start)) => {
                if element_stack.is_empty() {
                    root_count += 1;
                    if kind == XmlSourceKind::Document && root_count > 1 && !reported_multiple_roots
                    {
                        diagnostics.push(xml_diagnostic(
                            request,
                            source,
                            xml_event_position(&reader, &start, true),
                            "cem.xml.parse_error",
                            Severity::Error,
                            "XML document must have exactly one document element".to_owned(),
                        ));
                        reported_multiple_roots = true;
                    }
                }

                let (_, mut start_diagnostics) =
                    validate_xml_start_event(request, source, &start, &namespace_stack);
                diagnostics.append(&mut start_diagnostics);
            }
            Ok(quick_xml::events::Event::End(end)) => {
                let found = xml_qname_display(end.name().as_ref());
                match element_stack.pop() {
                    Some(expected) if expected == found => {
                        if namespace_stack.len() > 1 {
                            namespace_stack.pop();
                        }
                    }
                    Some(expected) => diagnostics.push(xml_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        Severity::Error,
                        format!("XML end tag `</{found}>` does not match `<{expected}>`"),
                    )),
                    None => diagnostics.push(xml_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        Severity::Error,
                        format!("XML end tag `</{found}>` has no matching start tag"),
                    )),
                }
            }
            Ok(quick_xml::events::Event::Decl(decl)) => {
                if let Err(error) = decl.version() {
                    diagnostics.push(xml_reader_error_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        &error,
                    ));
                }
                if let Some(encoding) = decl.encoding() {
                    match encoding {
                        Ok(encoding) => {
                            let encoding = String::from_utf8_lossy(encoding.as_ref());
                            if !xml_encoding_is_supported(&encoding) {
                                diagnostics.push(xml_unsupported_encoding_diagnostic(
                                    request,
                                    source,
                                    Some(reader.error_position()),
                                    &format!(
                                        "XML declaration encoding `{encoding}` is not supported"
                                    ),
                                ));
                            } else if let Some(charset) = mime_charset {
                                let declared = xml_normalized_encoding(&encoding);
                                let charset = xml_normalized_encoding(charset);
                                if declared != charset
                                    && !(declared == "utf-8" && charset == "us-ascii")
                                    && !(declared == "us-ascii" && charset == "utf-8")
                                {
                                    diagnostics.push(xml_diagnostic(
                                        request,
                                        source,
                                        Some(reader.error_position()),
                                        "cem.xml.encoding_conflict",
                                        Severity::Warning,
                                        format!(
                                            "XML declaration encoding `{encoding}` conflicts with content-type charset `{charset}`"
                                        ),
                                    ));
                                }
                            }
                        }
                        Err(error) => diagnostics.push(xml_attribute_error_diagnostic(
                            request,
                            source,
                            &error,
                            Some(reader.error_position()),
                        )),
                    }
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => diagnostics.push(xml_diagnostic(
                request,
                source,
                Some(reader.error_position()),
                "cem.xml.dtd_rejected",
                Severity::Error,
                "XML DTD declarations are rejected until an explicit DTD policy enables them"
                    .to_owned(),
            )),
            Ok(quick_xml::events::Event::GeneralRef(reference)) => {
                if !xml_entity_reference_is_builtin(reference.as_ref()) {
                    diagnostics.push(xml_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.external_entity_rejected",
                        Severity::Error,
                        format!(
                            "XML entity reference `&{};` is rejected",
                            String::from_utf8_lossy(reference.as_ref())
                        ),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Text(text)) => {
                if kind == XmlSourceKind::Document
                    && element_stack.is_empty()
                    && !xml_bytes_are_whitespace(text.as_ref())
                {
                    diagnostics.push(xml_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        Severity::Error,
                        "XML document cannot contain character data outside the document element"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::CData(_)) if kind == XmlSourceKind::Document => {
                if element_stack.is_empty() {
                    diagnostics.push(xml_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        Severity::Error,
                        "XML document cannot contain CDATA outside the document element".to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(xml_reader_error_diagnostic(
                    request,
                    source,
                    Some(reader.error_position()),
                    &error,
                ));
                break;
            }
        }
    }

    if kind == XmlSourceKind::Document && root_count == 0 {
        diagnostics.push(xml_diagnostic(
            request,
            source,
            Some(0),
            "cem.xml.parse_error",
            Severity::Error,
            "XML document must contain a document element".to_owned(),
        ));
    }
    if let Some(unclosed) = element_stack.last() {
        diagnostics.push(xml_diagnostic(
            request,
            source,
            Some(reader.buffer_position()),
            "cem.xml.parse_error",
            Severity::Error,
            format!("XML start tag `<{unclosed}>` is missing a matching end tag"),
        ));
    }

    diagnostics
}

#[derive(Clone, Debug)]
struct XmlAttributeView {
    qualified_name: String,
}

fn validate_xml_start_event(
    request: &XmlSourceValidationRequest<'_>,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
) -> (BTreeMap<String, String>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut attributes = Vec::new();
    let mut next_namespaces = namespace_stack
        .last()
        .cloned()
        .unwrap_or_else(xml_initial_namespaces);

    for attribute in start.attributes().with_checks(false) {
        match attribute {
            Ok(attribute) => {
                let qualified_name = xml_qname_display(attribute.key.as_ref());
                let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                if qualified_name == "xmlns" {
                    next_namespaces.insert(String::new(), value.clone());
                } else if let Some(prefix) = qualified_name.strip_prefix("xmlns:") {
                    next_namespaces.insert(prefix.to_owned(), value.clone());
                }
                diagnostics.extend(xml_entity_reference_diagnostics(
                    request,
                    source,
                    value.as_bytes(),
                ));
                attributes.push(XmlAttributeView { qualified_name });
            }
            Err(error) => diagnostics.push(xml_attribute_error_diagnostic(
                request, source, &error, None,
            )),
        }
    }

    let element_name = xml_qname_display(start.name().as_ref());
    if let Some(prefix) = xml_qname_prefix(&element_name) {
        if !xml_prefix_is_bound(&next_namespaces, prefix) {
            diagnostics.push(xml_unbound_namespace_prefix_diagnostic(
                request,
                source,
                None,
                prefix,
                &element_name,
            ));
        }
    }

    let mut expanded_attributes = BTreeSet::new();
    for attribute in attributes {
        if xml_attribute_is_namespace_declaration(&attribute.qualified_name) {
            continue;
        }

        let (namespace_uri, local_name) =
            xml_attribute_expanded_name(&attribute.qualified_name, &next_namespaces);
        if let Some(prefix) = xml_qname_prefix(&attribute.qualified_name) {
            if !xml_prefix_is_bound(&next_namespaces, prefix) {
                diagnostics.push(xml_unbound_namespace_prefix_diagnostic(
                    request,
                    source,
                    None,
                    prefix,
                    &attribute.qualified_name,
                ));
            }
        }

        if !expanded_attributes.insert((namespace_uri.clone(), local_name.clone())) {
            diagnostics.push(xml_diagnostic(
                request,
                source,
                None,
                "cem.xml.duplicate_attribute",
                Severity::Error,
                format!(
                    "XML element `<{element_name}>` has a duplicate attribute `{}`",
                    attribute.qualified_name
                ),
            ));
        }
    }

    (next_namespaces, diagnostics)
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

fn xml_qname_display(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}

fn xml_entity_reference_is_builtin(name: &[u8]) -> bool {
    name.starts_with(b"#") || matches!(name, b"amp" | b"lt" | b"gt" | b"apos" | b"quot")
}

fn xml_entity_reference_diagnostics(
    request: &XmlSourceValidationRequest<'_>,
    source: &str,
    value: &[u8],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut remaining = value;
    while let Some(start) = remaining.iter().position(|byte| *byte == b'&') {
        let after_amp = &remaining[start + 1..];
        let Some(end) = after_amp.iter().position(|byte| *byte == b';') else {
            break;
        };
        let reference = &after_amp[..end];
        if !xml_entity_reference_is_builtin(reference) {
            diagnostics.push(xml_diagnostic(
                request,
                source,
                None,
                "cem.xml.external_entity_rejected",
                Severity::Error,
                format!(
                    "XML entity reference `&{};` is rejected",
                    String::from_utf8_lossy(reference)
                ),
            ));
        }
        remaining = &after_amp[end + 1..];
    }
    diagnostics
}

fn xml_bytes_are_whitespace(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .map(|value| value.chars().all(char::is_whitespace))
        .unwrap_or(false)
}

fn xml_encoding_is_supported(encoding: &str) -> bool {
    matches!(
        xml_normalized_encoding(encoding).as_str(),
        "utf-8" | "us-ascii"
    )
}

fn xml_normalized_encoding(encoding: &str) -> String {
    encoding.trim().trim_matches('"').to_ascii_lowercase()
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

fn xml_reader_error_diagnostic(
    request: &XmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    error: &quick_xml::Error,
) -> Diagnostic {
    let code = match error {
        quick_xml::Error::Encoding(_) => "cem.xml.unsupported_encoding",
        quick_xml::Error::InvalidAttr(quick_xml::events::attributes::AttrError::Duplicated(
            _,
            _,
        )) => "cem.xml.duplicate_attribute",
        quick_xml::Error::Namespace(_) => "cem.xml.unbound_namespace_prefix",
        _ => "cem.xml.parse_error",
    };
    xml_diagnostic(
        request,
        source,
        byte_offset,
        code,
        Severity::Error,
        format!("XML parse error: {error}"),
    )
}

fn xml_attribute_error_diagnostic(
    request: &XmlSourceValidationRequest<'_>,
    source: &str,
    error: &quick_xml::events::attributes::AttrError,
    base_offset: Option<u64>,
) -> Diagnostic {
    let code = match error {
        quick_xml::events::attributes::AttrError::Duplicated(_, _) => "cem.xml.duplicate_attribute",
        _ => "cem.xml.parse_error",
    };
    xml_diagnostic(
        request,
        source,
        base_offset,
        code,
        Severity::Error,
        format!("XML attribute parse error: {error}"),
    )
}

fn xml_unbound_namespace_prefix_diagnostic(
    request: &XmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    prefix: &str,
    qualified_name: &str,
) -> Diagnostic {
    xml_diagnostic(
        request,
        source,
        byte_offset,
        "cem.xml.unbound_namespace_prefix",
        Severity::Error,
        format!("XML namespace prefix `{prefix}` is not bound for `{qualified_name}`"),
    )
}

fn xml_unsupported_utf8_diagnostic(
    request: &XmlSourceValidationRequest<'_>,
    error: &std::str::Utf8Error,
) -> Diagnostic {
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        byte_offset: u64::try_from(error.valid_up_to()).ok(),
        code: "cem.xml.unsupported_encoding".to_owned(),
        severity: Severity::Error,
        message: format!("XML source must be valid UTF-8: {error}"),
        ..Diagnostic::default()
    }
}

fn xml_unsupported_encoding_diagnostic(
    request: &XmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    message: &str,
) -> Diagnostic {
    xml_diagnostic(
        request,
        source,
        byte_offset,
        "cem.xml.unsupported_encoding",
        Severity::Error,
        message.to_owned(),
    )
}

fn xml_diagnostic(
    request: &XmlSourceValidationRequest<'_>,
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

    fn validate(source: &str) -> Vec<Diagnostic> {
        validate_xml_source_bytes(XmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.xml",
            content_type: Some("application/xml"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn xml_source_validator_accepts_namespaced_document() {
        let diagnostics = validate_xml_source_bytes(XmlSourceValidationRequest {
            bytes: br#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns:meta="https://example.test/meta" meta:version="1">
  <item id="a1">Alpha</item>
</catalog>
"#,
            source_uri: "fixture.xml",
            content_type: Some("text/xml; charset=utf-8"),
        });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn xml_source_validator_reports_mismatched_tag() {
        let diagnostics = validate("<root><item></root>\n");

        assert!(has_code(&diagnostics, "cem.xml.parse_error"));
    }

    #[test]
    fn xml_source_validator_reports_unbound_namespace_prefix() {
        let diagnostics = validate("<root><meta:item/></root>\n");

        assert!(has_code(&diagnostics, "cem.xml.unbound_namespace_prefix"));
    }

    #[test]
    fn xml_source_validator_reports_dtd_rejected() {
        let diagnostics = validate(
            r#"<!DOCTYPE root SYSTEM "file:///etc/passwd">
<root/>
"#,
        );

        assert!(has_code(&diagnostics, "cem.xml.dtd_rejected"));
    }
}
