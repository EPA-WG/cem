use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::SVG_NAMESPACE_URI;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct SvgSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_svg_source_bytes(request: SvgSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let _ = request.content_type;
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            return vec![svg_not_well_formed_diagnostic(
                &request,
                "",
                u64::try_from(error.valid_up_to()).ok(),
                format!("SVG source must be valid UTF-8: {error}"),
            )];
        }
    };

    validate_svg_source(&request, source)
}

#[derive(Clone, Debug)]
struct SvgAttributeView {
    namespace_uri: String,
    local_name: String,
    value: String,
}

#[derive(Clone, Debug)]
struct SvgElementFrame {
    local_name: String,
    namespace_uri: String,
    attributes: Vec<SvgAttributeView>,
}

#[derive(Clone, Debug, Default)]
struct SvgDocumentState {
    root_is_svg: bool,
    root_has_accessible_name: bool,
    root_accessibility_exempt: bool,
    reported_external_resource: bool,
    reported_script: bool,
}

fn validate_svg_source(request: &SvgSourceValidationRequest<'_>, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;

    let mut element_stack: Vec<SvgElementFrame> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut state = SvgDocumentState::default();

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                let start_offset = xml_event_position(&reader, &start, false);
                let (frame, namespaces, mut event_diagnostics) =
                    svg_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                svg_validate_element(
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
                    svg_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                svg_validate_element(
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
                    diagnostics.push(svg_not_well_formed_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "SVG document cannot contain character data outside the document element"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => {
                if !state.reported_external_resource {
                    state.reported_external_resource = true;
                    diagnostics.push(svg_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.svg.external_resource_rejected",
                        Severity::Error,
                        "SVG DOCTYPE declarations are rejected because they can reference external resources"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(svg_xml_error_diagnostic(
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
        diagnostics.push(svg_not_well_formed_diagnostic(
            request,
            source,
            Some(0),
            "SVG document must contain a document element".to_owned(),
        ));
    } else if state.root_is_svg
        && !state.root_accessibility_exempt
        && !state.root_has_accessible_name
    {
        diagnostics.push(svg_diagnostic(
            request,
            source,
            Some(0),
            "cem.svg.accessible_name_missing",
            Severity::Warning,
            "Visible SVG root should provide title, desc, aria-label, or aria-labelledby"
                .to_owned(),
        ));
    }

    diagnostics
}

fn svg_start_frame(
    request: &SvgSourceValidationRequest<'_>,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
    byte_offset: Option<u64>,
) -> (SvgElementFrame, BTreeMap<String, String>, Vec<Diagnostic>) {
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
            Err(error) => diagnostics.push(svg_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                format!("SVG XML attribute parse error: {error}"),
            )),
        }
    }

    let qualified_name = qname_display(start.name().as_ref());
    if let Some(prefix) = xml_qname_prefix(&qualified_name) {
        if !xml_prefix_is_bound(&namespaces, prefix) {
            diagnostics.push(svg_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                format!("SVG namespace prefix `{prefix}` is not bound for `{qualified_name}`"),
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
                    diagnostics.push(svg_not_well_formed_diagnostic(
                        request,
                        source,
                        byte_offset,
                        format!(
                            "SVG namespace prefix `{prefix}` is not bound for attribute `{qualified_name}`"
                        ),
                    ));
                }
            }
            let (namespace_uri, local_name) =
                xml_attribute_expanded_name(&qualified_name, &namespaces);
            SvgAttributeView {
                namespace_uri,
                local_name,
                value,
            }
        })
        .collect();

    (
        SvgElementFrame {
            local_name,
            namespace_uri,
            attributes,
        },
        namespaces,
        diagnostics,
    )
}

fn svg_validate_element(
    request: &SvgSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    frame: &SvgElementFrame,
    element_stack: &[SvgElementFrame],
    state: &mut SvgDocumentState,
    root_count: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if element_stack.is_empty() {
        *root_count += 1;
        if *root_count > 1 {
            diagnostics.push(svg_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                "SVG document must have exactly one document element".to_owned(),
            ));
            return;
        }
        if frame.local_name != "svg" {
            diagnostics.push(svg_diagnostic(
                request,
                source,
                byte_offset,
                "cem.svg.root_not_svg",
                Severity::Error,
                format!(
                    "SVG root element must be `svg`, found `{}`",
                    frame.local_name
                ),
            ));
            return;
        }
        if frame.namespace_uri != SVG_NAMESPACE_URI {
            diagnostics.push(svg_diagnostic(
                request,
                source,
                byte_offset,
                "cem.svg.namespace_missing",
                Severity::Error,
                "SVG root `svg` element must use the http://www.w3.org/2000/svg namespace"
                    .to_owned(),
            ));
            return;
        }

        state.root_is_svg = true;
        state.root_has_accessible_name = svg_frame_has_accessible_name_attribute(frame);
        state.root_accessibility_exempt = svg_frame_is_accessibility_exempt(frame);
    } else if state.root_is_svg
        && element_stack.len() == 1
        && frame.namespace_uri == SVG_NAMESPACE_URI
        && matches!(frame.local_name.as_str(), "title" | "desc")
    {
        state.root_has_accessible_name = true;
    }

    if frame.namespace_uri == SVG_NAMESPACE_URI
        && frame.local_name == "script"
        && !state.reported_script
    {
        state.reported_script = true;
        diagnostics.push(svg_diagnostic(
            request,
            source,
            byte_offset,
            "cem.svg.script_rejected",
            Severity::Error,
            "SVG script elements are rejected unless an explicit execution policy is enabled"
                .to_owned(),
        ));
    }

    if !state.reported_external_resource {
        if let Some(attribute) = frame
            .attributes
            .iter()
            .find(|attribute| svg_attribute_requires_resource_policy(attribute))
        {
            state.reported_external_resource = true;
            diagnostics.push(svg_diagnostic(
                request,
                source,
                byte_offset,
                "cem.svg.external_resource_rejected",
                Severity::Error,
                format!(
                    "SVG attribute `{}` references an external resource without an explicit resolver policy",
                    attribute.local_name
                ),
            ));
        }
    }
}

fn svg_frame_has_accessible_name_attribute(frame: &SvgElementFrame) -> bool {
    frame.attributes.iter().any(|attribute| {
        matches!(
            attribute.local_name.as_str(),
            "aria-label" | "aria-labelledby"
        ) && !attribute.value.trim().is_empty()
    })
}

fn svg_frame_is_accessibility_exempt(frame: &SvgElementFrame) -> bool {
    frame.attributes.iter().any(|attribute| {
        let value = attribute.value.trim().to_ascii_lowercase();
        (attribute.local_name == "aria-hidden" && value == "true")
            || (attribute.local_name == "role" && matches!(value.as_str(), "none" | "presentation"))
            || (attribute.local_name == "hidden" && attribute.namespace_uri.is_empty())
    })
}

fn svg_attribute_requires_resource_policy(attribute: &SvgAttributeView) -> bool {
    let is_direct_resource_reference = matches!(attribute.local_name.as_str(), "href" | "src");
    (is_direct_resource_reference
        && svg_direct_resource_reference_requires_policy(&attribute.value))
        || svg_css_url_reference_requires_policy(&attribute.value)
}

fn svg_direct_resource_reference_requires_policy(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let unquoted = trimmed.trim_matches('"').trim_matches('\'');
    if unquoted.starts_with('#') || unquoted.to_ascii_lowercase().starts_with("data:") {
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
        if svg_url_function_reference_requires_policy(reference) {
            return true;
        }
        search_start = url_start + 4;
    }
    false
}

fn svg_url_function_reference_requires_policy(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.starts_with('#') {
        return false;
    }
    !trimmed.to_ascii_lowercase().starts_with("data:")
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

fn svg_xml_error_diagnostic(
    request: &SvgSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    error: &quick_xml::Error,
) -> Diagnostic {
    svg_not_well_formed_diagnostic(
        request,
        source,
        byte_offset,
        format!("SVG XML parse error: {error}"),
    )
}

fn svg_not_well_formed_diagnostic(
    request: &SvgSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    message: String,
) -> Diagnostic {
    svg_diagnostic(
        request,
        source,
        byte_offset,
        "cem.svg.not_well_formed_xml",
        Severity::Error,
        message,
    )
}

fn svg_diagnostic(
    request: &SvgSourceValidationRequest<'_>,
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

    fn validate(source: &str) -> Vec<Diagnostic> {
        validate_svg_source_bytes(SvgSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.svg",
            content_type: Some("image/svg+xml"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
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
    fn svg_source_validator_reports_missing_namespace() {
        let diagnostics = validate(
            r#"<svg role="img" viewBox="0 0 24 24">
  <title>Missing namespace</title>
</svg>
"#,
        );

        assert!(has_code(&diagnostics, "cem.svg.namespace_missing"));
    }

    #[test]
    fn svg_source_validator_reports_root_not_svg() {
        let diagnostics = validate(
            r#"<section xmlns="http://www.w3.org/2000/svg">
  <title>Wrong root</title>
</section>
"#,
        );

        assert!(has_code(&diagnostics, "cem.svg.root_not_svg"));
    }

    #[test]
    fn svg_source_validator_reports_script_rejected() {
        let diagnostics = validate(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img">
  <title>Scripted</title>
  <script>alert("blocked")</script>
</svg>
"#,
        );

        assert!(has_code(&diagnostics, "cem.svg.script_rejected"));
    }

    #[test]
    fn svg_source_validator_reports_external_resource_rejected() {
        let diagnostics = validate(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img">
  <title>External image</title>
  <image href="https://example.test/logo.png"/>
</svg>
"#,
        );

        assert!(has_code(&diagnostics, "cem.svg.external_resource_rejected"));
    }

    #[test]
    fn svg_source_validator_reports_accessible_name_missing_warning() {
        let diagnostics = validate(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <path d="M12 3v18"/>
</svg>
"#,
        );

        assert!(has_code(&diagnostics, "cem.svg.accessible_name_missing"));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }

    #[test]
    fn svg_source_validator_reports_not_well_formed_xml() {
        let diagnostics = validate(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img">
  <title>Broken</title>
  <path>
</svg>
"#,
        );

        assert!(has_code(&diagnostics, "cem.svg.not_well_formed_xml"));
    }
}
