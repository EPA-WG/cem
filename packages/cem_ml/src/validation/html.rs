use crate::diagnostics::{Diagnostic, Severity};

#[derive(Debug, Clone, Copy)]
pub struct HtmlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_html_source_bytes(request: HtmlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let declared_charset = request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "charset"));
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            return vec![html_diagnostic(
                &request,
                "",
                u64::try_from(error.valid_up_to()).ok(),
                "cem.html.parse_error",
                Severity::Error,
                format!("HTML validator currently requires UTF-8-compatible input bytes: {error}"),
            )]
        }
    };

    validate_html_source(&request, source, declared_charset.as_deref())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HtmlNamespace {
    Html,
    Svg,
    MathMl,
}

#[derive(Clone, Debug)]
struct HtmlAttributeView {
    local_name: String,
    value: String,
}

#[derive(Clone, Debug)]
struct HtmlStartTag {
    local_name: String,
    attributes: Vec<HtmlAttributeView>,
    self_closing: bool,
}

#[derive(Clone, Debug)]
struct HtmlElementFrame {
    local_name: String,
    child_namespace: HtmlNamespace,
}

#[derive(Clone, Debug, Default)]
struct HtmlDocumentState {
    reported_invalid_nesting: bool,
    reported_script: bool,
    reported_external_resource: bool,
    reported_invalid_custom_element_name: bool,
    reported_foreign_content: bool,
    reported_encoding_conflict: bool,
}

fn validate_html_source(
    request: &HtmlSourceValidationRequest<'_>,
    source: &str,
    declared_charset: Option<&str>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut element_stack = Vec::new();
    let mut state = HtmlDocumentState::default();
    let mut offset = 0usize;

    while let Some(relative_lt) = source[offset..].find('<') {
        let lt = offset + relative_lt;
        let token = &source[lt..];

        if token.starts_with("<!--") {
            offset = match token.find("-->") {
                Some(relative_end) => lt + relative_end + 3,
                None => source.len(),
            };
            continue;
        }

        if token.starts_with("</") {
            let Some(gt) = html_find_tag_end(source, lt + 2) else {
                break;
            };
            let local_name = html_parse_end_tag_name(&source[lt + 2..gt]);
            html_close_element(
                request,
                source,
                Some(lt as u64),
                &local_name,
                &mut element_stack,
                &mut state,
                &mut diagnostics,
            );
            offset = gt + 1;
            continue;
        }

        if token.starts_with("<!") {
            offset = html_find_tag_end(source, lt + 2).map_or(source.len(), |gt| gt + 1);
            continue;
        }

        if token.starts_with("<?") {
            offset = html_find_tag_end(source, lt + 2).map_or(source.len(), |gt| gt + 1);
            continue;
        }

        let Some(gt) = html_find_tag_end(source, lt + 1) else {
            break;
        };
        let Some(tag) = html_parse_start_tag(&source[lt + 1..gt]) else {
            offset = gt + 1;
            continue;
        };

        let parent_child_namespace = element_stack
            .last()
            .map(|frame: &HtmlElementFrame| frame.child_namespace)
            .unwrap_or(HtmlNamespace::Html);
        let namespace = html_element_namespace(parent_child_namespace, &tag);
        let child_namespace = html_child_namespace(namespace, &tag);
        html_validate_start_tag(
            request,
            source,
            Some(lt as u64),
            &tag,
            namespace,
            declared_charset,
            &mut state,
            &mut diagnostics,
        );

        if !tag.self_closing && !html_is_void_element(&tag.local_name, namespace) {
            element_stack.push(HtmlElementFrame {
                local_name: tag.local_name.clone(),
                child_namespace,
            });
        }

        if html_is_raw_text_element(&tag.local_name, namespace) {
            let closing = format!("</{}", tag.local_name);
            let lower_remaining = source[gt + 1..].to_ascii_lowercase();
            if let Some(relative_closing) = lower_remaining.find(&closing) {
                offset = gt + 1 + relative_closing;
            } else {
                offset = source.len();
            }
        } else {
            offset = gt + 1;
        }
    }

    diagnostics
}

fn html_find_tag_end(source: &str, mut offset: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut quote = None;
    while offset < bytes.len() {
        let byte = bytes[offset];
        match (quote, byte) {
            (Some(q), b) if b == q => quote = None,
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return Some(offset),
            _ => {}
        }
        offset += 1;
    }
    None
}

fn html_parse_end_tag_name(raw: &str) -> String {
    raw.trim_start()
        .split(|ch: char| ch.is_whitespace() || ch == '>')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn html_parse_start_tag(raw: &str) -> Option<HtmlStartTag> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('/') {
        return None;
    }

    let name_end = trimmed
        .find(|ch: char| ch.is_whitespace() || ch == '/' || ch == '>')
        .unwrap_or(trimmed.len());
    let local_name = trimmed[..name_end].to_ascii_lowercase();
    if local_name.is_empty() {
        return None;
    }
    let raw_attributes = &trimmed[name_end..];
    Some(HtmlStartTag {
        local_name,
        attributes: html_parse_attributes(raw_attributes),
        self_closing: trimmed.trim_end().ends_with('/'),
    })
}

fn html_parse_attributes(raw: &str) -> Vec<HtmlAttributeView> {
    let mut attributes = Vec::new();
    let bytes = raw.as_bytes();
    let mut offset = 0usize;
    while offset < bytes.len() {
        while offset < bytes.len() && (bytes[offset].is_ascii_whitespace() || bytes[offset] == b'/')
        {
            offset += 1;
        }
        if offset >= bytes.len() {
            break;
        }

        let name_start = offset;
        while offset < bytes.len()
            && !bytes[offset].is_ascii_whitespace()
            && !matches!(bytes[offset], b'=' | b'/' | b'>')
        {
            offset += 1;
        }
        let local_name = raw[name_start..offset].trim().to_ascii_lowercase();
        if local_name.is_empty() {
            offset += 1;
            continue;
        }

        while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }

        let mut value = String::new();
        if offset < bytes.len() && bytes[offset] == b'=' {
            offset += 1;
            while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
                offset += 1;
            }
            if offset < bytes.len() && matches!(bytes[offset], b'"' | b'\'') {
                let quote = bytes[offset];
                offset += 1;
                let value_start = offset;
                while offset < bytes.len() && bytes[offset] != quote {
                    offset += 1;
                }
                value = raw[value_start..offset].to_owned();
                if offset < bytes.len() {
                    offset += 1;
                }
            } else {
                let value_start = offset;
                while offset < bytes.len()
                    && !bytes[offset].is_ascii_whitespace()
                    && !matches!(bytes[offset], b'/' | b'>')
                {
                    offset += 1;
                }
                value = raw[value_start..offset].to_owned();
            }
        }

        attributes.push(HtmlAttributeView { local_name, value });
    }
    attributes
}

fn html_element_namespace(
    parent_child_namespace: HtmlNamespace,
    tag: &HtmlStartTag,
) -> HtmlNamespace {
    match (parent_child_namespace, tag.local_name.as_str()) {
        (HtmlNamespace::Html, "svg") => HtmlNamespace::Svg,
        (HtmlNamespace::Html, "math") => HtmlNamespace::MathMl,
        (namespace, _) => namespace,
    }
}

fn html_child_namespace(namespace: HtmlNamespace, tag: &HtmlStartTag) -> HtmlNamespace {
    match (namespace, tag.local_name.as_str()) {
        (HtmlNamespace::Svg, "foreignobject") => HtmlNamespace::Html,
        (HtmlNamespace::MathMl, "annotation-xml") if html_annotation_is_html(&tag.attributes) => {
            HtmlNamespace::Html
        }
        _ => namespace,
    }
}

fn html_annotation_is_html(attributes: &[HtmlAttributeView]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.local_name == "encoding"
            && matches!(
                attribute.value.trim().to_ascii_lowercase().as_str(),
                "text/html" | "application/xhtml+xml"
            )
    })
}

fn html_validate_start_tag(
    request: &HtmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    tag: &HtmlStartTag,
    namespace: HtmlNamespace,
    declared_charset: Option<&str>,
    state: &mut HtmlDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if tag.local_name.contains(':') && !state.reported_foreign_content {
        state.reported_foreign_content = true;
        diagnostics.push(html_diagnostic(
            request,
            source,
            byte_offset,
            "cem.html.foreign_content_unregistered",
            Severity::Warning,
            format!(
                "HTML tag `{}` is outside the HTML/SVG/MathML parser-default namespaces",
                tag.local_name
            ),
        ));
    }

    if namespace == HtmlNamespace::Html && tag.local_name == "meta" {
        html_validate_meta_charset(
            request,
            source,
            byte_offset,
            tag,
            declared_charset,
            state,
            diagnostics,
        );
    }

    if namespace == HtmlNamespace::Html
        && tag.local_name == "script"
        && html_script_is_executable(tag)
        && !state.reported_script
    {
        state.reported_script = true;
        diagnostics.push(html_diagnostic(
            request,
            source,
            byte_offset,
            "cem.html.script_rejected",
            Severity::Error,
            "Executable HTML script is rejected unless an explicit host policy enables it"
                .to_owned(),
        ));
    }

    if html_start_tag_requires_resource_policy(tag, namespace) && !state.reported_external_resource
    {
        state.reported_external_resource = true;
        diagnostics.push(html_diagnostic(
            request,
            source,
            byte_offset,
            "cem.html.external_resource_rejected",
            Severity::Error,
            "HTML/SVG/MathML external resource access requires an explicit resolver policy"
                .to_owned(),
        ));
    }

    if namespace == HtmlNamespace::Html {
        html_validate_custom_element_name(request, source, byte_offset, tag, state, diagnostics);
    }
}

fn html_validate_meta_charset(
    request: &HtmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    tag: &HtmlStartTag,
    declared_charset: Option<&str>,
    state: &mut HtmlDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if state.reported_encoding_conflict {
        return;
    }
    let Some(declared_charset) = declared_charset else {
        return;
    };
    let Some(meta_charset) = html_meta_charset(tag) else {
        return;
    };
    if !html_normalized_charset(declared_charset).eq(&html_normalized_charset(&meta_charset)) {
        state.reported_encoding_conflict = true;
        diagnostics.push(html_diagnostic(
            request,
            source,
            byte_offset,
            "cem.html.encoding_conflict",
            Severity::Warning,
            format!("HTML MIME charset `{declared_charset}` conflicts with meta charset `{meta_charset}`"),
        ));
    }
}

fn html_meta_charset(tag: &HtmlStartTag) -> Option<String> {
    if let Some(value) = html_attribute_value(tag, "charset") {
        return Some(value.trim().to_owned());
    }
    let http_equiv = html_attribute_value(tag, "http-equiv")?;
    if !http_equiv.eq_ignore_ascii_case("content-type") {
        return None;
    }
    let content = html_attribute_value(tag, "content")?;
    content.split(';').skip(1).find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("charset") {
            Some(value.trim().trim_matches('"').to_owned())
        } else {
            None
        }
    })
}

fn html_normalized_charset(value: &str) -> String {
    value.trim().trim_matches('"').to_ascii_lowercase()
}

fn html_script_is_executable(tag: &HtmlStartTag) -> bool {
    let Some(script_type) = html_attribute_value(tag, "type") else {
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

fn html_start_tag_requires_resource_policy(tag: &HtmlStartTag, namespace: HtmlNamespace) -> bool {
    tag.attributes.iter().any(|attribute| match namespace {
        HtmlNamespace::Html => html_attribute_requires_resource_policy(tag, attribute),
        HtmlNamespace::Svg => {
            matches!(attribute.local_name.as_str(), "href" | "src")
                && html_uri_requires_policy(&attribute.value)
                || html_css_url_reference_requires_policy(&attribute.value)
        }
        HtmlNamespace::MathMl => {
            tag.local_name == "annotation"
                && attribute.local_name == "src"
                && html_uri_requires_policy(&attribute.value)
        }
    })
}

fn html_attribute_requires_resource_policy(
    tag: &HtmlStartTag,
    attribute: &HtmlAttributeView,
) -> bool {
    match attribute.local_name.as_str() {
        "src" | "poster" | "action" => html_uri_requires_policy(&attribute.value),
        "srcset" => html_srcset_requires_policy(&attribute.value),
        "href" if matches!(tag.local_name.as_str(), "link" | "base") => {
            html_uri_requires_policy(&attribute.value)
        }
        "style" => html_css_url_reference_requires_policy(&attribute.value),
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
        if html_uri_requires_policy(reference) {
            return true;
        }
        search_start = url_start + 4;
    }
    false
}

fn html_validate_custom_element_name(
    request: &HtmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    tag: &HtmlStartTag,
    state: &mut HtmlDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if state.reported_invalid_custom_element_name {
        return;
    }
    let invalid_tag_name =
        tag.local_name.contains('-') && !html_custom_element_name_is_valid(&tag.local_name);
    let invalid_is_attribute = html_attribute_value(tag, "is")
        .is_some_and(|value| !html_custom_element_name_is_valid(value));
    if invalid_tag_name || invalid_is_attribute {
        state.reported_invalid_custom_element_name = true;
        diagnostics.push(html_diagnostic(
            request,
            source,
            byte_offset,
            "cem.html.custom_element_name_invalid",
            Severity::Error,
            "HTML custom element names must contain a hyphen and use a source-stable lowercase name"
                .to_owned(),
        ));
    }
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

fn html_attribute_value<'a>(tag: &'a HtmlStartTag, local_name: &str) -> Option<&'a str> {
    tag.attributes
        .iter()
        .find(|attribute| attribute.local_name == local_name)
        .map(|attribute| attribute.value.as_str())
}

fn html_close_element(
    request: &HtmlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    local_name: &str,
    element_stack: &mut Vec<HtmlElementFrame>,
    state: &mut HtmlDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if local_name.is_empty() {
        return;
    }
    if let Some(position) = element_stack
        .iter()
        .rposition(|frame| frame.local_name == local_name)
    {
        element_stack.truncate(position);
        return;
    }
    if !state.reported_invalid_nesting {
        state.reported_invalid_nesting = true;
        diagnostics.push(html_diagnostic(
            request,
            source,
            byte_offset,
            "cem.html.invalid_nesting_recovered",
            Severity::Warning,
            format!("HTML parser recovered an unmatched closing tag `</{local_name}>`"),
        ));
    }
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

fn html_is_raw_text_element(local_name: &str, namespace: HtmlNamespace) -> bool {
    namespace == HtmlNamespace::Html
        && matches!(local_name, "script" | "style" | "textarea" | "title")
}

fn html_diagnostic(
    request: &HtmlSourceValidationRequest<'_>,
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
        validate_html_source_bytes(HtmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.html",
            content_type: Some("text/html"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn html_source_validator_accepts_basic_document() {
        let diagnostics = validate(
            r#"<!doctype html>
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
    fn html_source_validator_reports_script_rejected() {
        let diagnostics = validate(
            r#"<!doctype html><html><body><script>alert("blocked")</script></body></html>"#,
        );

        assert!(has_code(&diagnostics, "cem.html.script_rejected"));
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
    fn html_source_validator_reports_invalid_custom_element_name() {
        let diagnostics = validate(r#"<!doctype html><html><body><x->Broken</x-></body></html>"#);

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
}
