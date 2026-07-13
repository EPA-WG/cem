use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::XHTML_NAMESPACE_URI;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct XhtmlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_xhtml_source_bytes(request: XhtmlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "profile"))
        .is_some()
    {
        diagnostics.push(xhtml_diagnostic(
            &request,
            "",
            None,
            "cem.xhtml.profile_deprecated",
            Severity::Info,
            "application/xhtml+xml profile parameter is deprecated".to_owned(),
        ));
    }

    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            diagnostics.push(xhtml_not_well_formed_diagnostic(
                &request,
                "",
                u64::try_from(error.valid_up_to()).ok(),
                format!("XHTML source must be valid UTF-8: {error}"),
            ));
            return diagnostics;
        }
    };

    diagnostics.extend(validate_xhtml_source(&request, source));
    diagnostics
}

#[derive(Clone, Debug)]
struct XhtmlElementFrame {
    local_name: String,
    namespace_uri: String,
}

#[derive(Clone, Debug, Default)]
struct XhtmlRootState {
    root_is_html: bool,
    saw_head: bool,
    saw_body: bool,
    reported_order: bool,
}

fn validate_xhtml_source(
    request: &XhtmlSourceValidationRequest<'_>,
    source: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;

    let mut element_stack: Vec<XhtmlElementFrame> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut root_state = XhtmlRootState::default();

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                let start_offset = xml_event_position(&reader, &start, false);
                let (frame, namespaces, mut event_diagnostics) =
                    xhtml_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                xhtml_validate_element_position(
                    request,
                    source,
                    start_offset,
                    &frame,
                    &element_stack,
                    &mut root_state,
                    &mut root_count,
                    &mut diagnostics,
                );
                element_stack.push(frame);
                namespace_stack.push(namespaces);
            }
            Ok(quick_xml::events::Event::Empty(start)) => {
                let start_offset = xml_event_position(&reader, &start, true);
                let (frame, _, mut event_diagnostics) =
                    xhtml_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                xhtml_validate_element_position(
                    request,
                    source,
                    start_offset,
                    &frame,
                    &element_stack,
                    &mut root_state,
                    &mut root_count,
                    &mut diagnostics,
                );
            }
            Ok(quick_xml::events::Event::End(end)) => {
                let found = qname_display(end.name().as_ref());
                match element_stack.pop() {
                    Some(expected) if expected.local_name == found => {
                        if namespace_stack.len() > 1 {
                            namespace_stack.pop();
                        }
                    }
                    Some(expected) => {
                        if namespace_stack.len() > 1 {
                            namespace_stack.pop();
                        }
                        diagnostics.push(xhtml_not_well_formed_diagnostic(
                            request,
                            source,
                            Some(reader.error_position()),
                            format!(
                                "XHTML end tag `</{found}>` does not match `<{}>`",
                                expected.local_name
                            ),
                        ));
                    }
                    None => diagnostics.push(xhtml_not_well_formed_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        format!("XHTML end tag `</{found}>` has no matching start tag"),
                    )),
                }
            }
            Ok(quick_xml::events::Event::Text(text)) => {
                if element_stack.is_empty() && !xml_bytes_are_whitespace(text.as_ref()) {
                    diagnostics.push(xhtml_not_well_formed_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "XHTML document cannot contain character data outside the document element"
                            .to_owned(),
                    ));
                } else if element_stack.len() == 1 && !xml_bytes_are_whitespace(text.as_ref()) {
                    xhtml_report_head_body_order(
                        request,
                        source,
                        Some(reader.error_position()),
                        &mut root_state,
                        &mut diagnostics,
                        "XHTML html element may only contain head and body child elements",
                    );
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => {}
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(xhtml_not_well_formed_diagnostic(
                    request,
                    source,
                    Some(reader.error_position()),
                    format!("XHTML XML parse error: {error}"),
                ));
                break;
            }
        }
    }

    if root_count == 0 {
        diagnostics.push(xhtml_not_well_formed_diagnostic(
            request,
            source,
            Some(0),
            "XHTML document must contain a document element".to_owned(),
        ));
    }
    if root_state.root_is_html && (!root_state.saw_head || !root_state.saw_body) {
        xhtml_report_head_body_order(
            request,
            source,
            Some(0),
            &mut root_state,
            &mut diagnostics,
            "XHTML html element must contain a head element followed by a body element",
        );
    }

    diagnostics
}

fn xhtml_start_frame(
    request: &XhtmlSourceValidationRequest<'_>,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
    byte_offset: Option<u64>,
) -> (XhtmlElementFrame, BTreeMap<String, String>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
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
                    namespaces.insert(String::new(), value);
                } else if let Some(prefix) = name.strip_prefix("xmlns:") {
                    namespaces.insert(prefix.to_owned(), value);
                }
            }
            Err(error) => diagnostics.push(xhtml_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                format!("XHTML XML attribute parse error: {error}"),
            )),
        }
    }

    let qualified_name = qname_display(start.name().as_ref());
    let (namespace_uri, local_name) = xhtml_expanded_name(&qualified_name, &namespaces);
    (
        XhtmlElementFrame {
            local_name,
            namespace_uri,
        },
        namespaces,
        diagnostics,
    )
}

fn xhtml_validate_element_position(
    request: &XhtmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    frame: &XhtmlElementFrame,
    element_stack: &[XhtmlElementFrame],
    root_state: &mut XhtmlRootState,
    root_count: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if element_stack.is_empty() {
        *root_count += 1;
        if *root_count > 1 {
            diagnostics.push(xhtml_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                "XHTML document must have exactly one document element".to_owned(),
            ));
            return;
        }
        if frame.local_name != "html" {
            diagnostics.push(xhtml_diagnostic(
                request,
                source,
                byte_offset,
                "cem.xhtml.root_not_html",
                Severity::Error,
                format!(
                    "XHTML root element must be `html`, found `{}`",
                    frame.local_name
                ),
            ));
            return;
        }
        if frame.namespace_uri != XHTML_NAMESPACE_URI {
            diagnostics.push(xhtml_diagnostic(
                request,
                source,
                byte_offset,
                "cem.xhtml.namespace_missing",
                Severity::Error,
                "XHTML root `html` element must use the http://www.w3.org/1999/xhtml namespace"
                    .to_owned(),
            ));
            return;
        }
        root_state.root_is_html = true;
        return;
    }

    if !root_state.root_is_html || element_stack.len() != 1 {
        return;
    }

    if frame.namespace_uri != XHTML_NAMESPACE_URI {
        xhtml_report_head_body_order(
            request,
            source,
            byte_offset,
            root_state,
            diagnostics,
            "XHTML html element direct children must be XHTML head and body elements",
        );
        return;
    }

    match frame.local_name.as_str() {
        "head" if !root_state.saw_head && !root_state.saw_body => {
            root_state.saw_head = true;
        }
        "body" if root_state.saw_head && !root_state.saw_body => {
            root_state.saw_body = true;
        }
        _ => xhtml_report_head_body_order(
            request,
            source,
            byte_offset,
            root_state,
            diagnostics,
            "XHTML html element must contain exactly one head element followed by one body element",
        ),
    }
}

fn xhtml_report_head_body_order(
    request: &XhtmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    root_state: &mut XhtmlRootState,
    diagnostics: &mut Vec<Diagnostic>,
    message: &str,
) {
    if root_state.reported_order {
        return;
    }
    root_state.reported_order = true;
    diagnostics.push(xhtml_diagnostic(
        request,
        source,
        byte_offset,
        "cem.xhtml.head_body_order",
        Severity::Error,
        message.to_owned(),
    ));
}

fn xhtml_expanded_name(
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

fn xhtml_not_well_formed_diagnostic(
    request: &XhtmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    message: String,
) -> Diagnostic {
    xhtml_diagnostic(
        request,
        source,
        byte_offset,
        "cem.xhtml.not_well_formed_xml",
        Severity::Error,
        message,
    )
}

fn xhtml_diagnostic(
    request: &XhtmlSourceValidationRequest<'_>,
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
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "xml".to_owned(),
        "http://www.w3.org/XML/1998/namespace".to_owned(),
    );
    namespaces.insert(
        "xmlns".to_owned(),
        "http://www.w3.org/2000/xmlns/".to_owned(),
    );
    namespaces
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
        validate_xhtml_source_bytes(XhtmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.xhtml",
            content_type: Some("application/xhtml+xml"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
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
    fn xhtml_source_validator_reports_missing_namespace() {
        let diagnostics = validate(
            r#"<html>
  <head><title>Missing namespace</title></head>
  <body><p>Not XHTML.</p></body>
</html>
"#,
        );

        assert!(has_code(&diagnostics, "cem.xhtml.namespace_missing"));
    }

    #[test]
    fn xhtml_source_validator_reports_root_not_html() {
        let diagnostics = validate(
            r#"<section xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Wrong root</title></head>
  <body><p>Bad root.</p></body>
</section>
"#,
        );

        assert!(has_code(&diagnostics, "cem.xhtml.root_not_html"));
    }

    #[test]
    fn xhtml_source_validator_reports_head_body_order() {
        let diagnostics = validate(
            r#"<html xmlns="http://www.w3.org/1999/xhtml">
  <body><p>Body first.</p></body>
  <head><title>Late head</title></head>
</html>
"#,
        );

        assert!(has_code(&diagnostics, "cem.xhtml.head_body_order"));
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
        let diagnostics = validate_xhtml_source_bytes(XhtmlSourceValidationRequest {
            bytes: br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Profile</title></head><body><p>Profile.</p></body></html>"#,
            source_uri: "fixture.xhtml",
            content_type: Some("application/xhtml+xml; profile=https://example.test/profile"),
        });

        assert!(has_code(&diagnostics, "cem.xhtml.profile_deprecated"));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }
}
