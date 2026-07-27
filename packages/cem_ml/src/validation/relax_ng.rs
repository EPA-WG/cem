use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::{content_type_essence, RELAX_NG_COMPACT_CONTENT_TYPE};
use std::collections::BTreeMap;

const RELAX_NG_STRUCTURE_NAMESPACE: &str = "http://relaxng.org/ns/structure/1.0";

#[derive(Debug, Clone, Copy)]
pub struct RelaxNgSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_relax_ng_source_bytes(
    request: RelaxNgSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    if request.content_type.map(content_type_essence).as_deref()
        == Some(RELAX_NG_COMPACT_CONTENT_TYPE)
    {
        validate_relax_ng_compact_source(&request)
    } else {
        validate_relax_ng_xml_source(&request)
    }
}

fn validate_relax_ng_xml_source(request: &RelaxNgSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => return vec![relax_ng_unsupported_utf8_diagnostic(request, &error)],
    };

    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;
    let mut namespace_stack = vec![NamespaceFrame::default()];
    let mut element_stack: Vec<String> = Vec::new();
    let mut root_seen = false;
    let mut root_is_grammar = false;
    let mut start_seen = false;

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                let (frame, mut namespace_diagnostics) =
                    namespace_frame_for_start(request, source, &start, namespace_stack.last());
                diagnostics.append(&mut namespace_diagnostics);
                let element = relax_ng_xml_element(&reader, &start, &frame);
                inspect_relax_ng_xml_element(
                    request,
                    source,
                    &element,
                    &mut root_seen,
                    &mut root_is_grammar,
                    &mut start_seen,
                    &mut diagnostics,
                );
                element_stack.push(element.qualified_name);
                namespace_stack.push(frame);
            }
            Ok(quick_xml::events::Event::Empty(start)) => {
                let (frame, mut namespace_diagnostics) =
                    namespace_frame_for_start(request, source, &start, namespace_stack.last());
                diagnostics.append(&mut namespace_diagnostics);
                let element = relax_ng_xml_element(&reader, &start, &frame);
                inspect_relax_ng_xml_element(
                    request,
                    source,
                    &element,
                    &mut root_seen,
                    &mut root_is_grammar,
                    &mut start_seen,
                    &mut diagnostics,
                );
            }
            Ok(quick_xml::events::Event::End(end)) => {
                let found = qname_display(end.name().as_ref());
                match element_stack.pop() {
                    Some(expected) if expected == found => {
                        if namespace_stack.len() > 1 {
                            namespace_stack.pop();
                        }
                    }
                    Some(expected) => {
                        if namespace_stack.len() > 1 {
                            namespace_stack.pop();
                        }
                        diagnostics.push(relax_ng_diagnostic(
                            request,
                            source,
                            Some(reader.error_position()),
                            "cem.relax_ng.xml_parse_error",
                            format!(
                                "RELAX NG XML end tag `</{found}>` does not match `<{expected}>`"
                            ),
                        ));
                    }
                    None => diagnostics.push(relax_ng_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "cem.relax_ng.xml_parse_error",
                        format!("RELAX NG XML end tag `</{found}>` has no matching start tag"),
                    )),
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => diagnostics.push(relax_ng_diagnostic(
                request,
                source,
                Some(reader.error_position()),
                "cem.relax_ng.xml_parse_error",
                "RELAX NG XML syntax must not use DTD declarations".to_owned(),
            )),
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(relax_ng_diagnostic(
                    request,
                    source,
                    Some(reader.error_position()),
                    "cem.relax_ng.xml_parse_error",
                    format!("RELAX NG XML parse error: {error}"),
                ));
                break;
            }
        }
    }

    if !root_seen {
        diagnostics.push(relax_ng_diagnostic(
            request,
            source,
            Some(0),
            "cem.relax_ng.xml_parse_error",
            "RELAX NG XML syntax must contain a grammar document element".to_owned(),
        ));
    } else if root_is_grammar && !start_seen {
        diagnostics.push(relax_ng_diagnostic(
            request,
            source,
            Some(0),
            "cem.relax_ng.missing_start",
            "RELAX NG grammar must declare a start pattern".to_owned(),
        ));
    }

    diagnostics
}

fn validate_relax_ng_compact_source(
    request: &RelaxNgSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => return vec![relax_ng_unsupported_utf8_diagnostic(request, &error)],
    };

    let mut diagnostics = Vec::new();
    if let Some(offset) = first_unbalanced_compact_brace(source) {
        diagnostics.push(relax_ng_diagnostic(
            request,
            source,
            Some(offset as u64),
            "cem.relax_ng.compact_parse_error",
            "RELAX NG compact syntax has unbalanced braces".to_owned(),
        ));
    }
    if !compact_source_declares_start(source) {
        diagnostics.push(relax_ng_diagnostic(
            request,
            source,
            Some(0),
            "cem.relax_ng.missing_start",
            "RELAX NG compact syntax must declare a start pattern".to_owned(),
        ));
    }
    diagnostics
}

#[derive(Clone, Debug, Default)]
struct NamespaceFrame {
    default_namespace: String,
    prefixes: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct RelaxNgXmlElement {
    qualified_name: String,
    local_name: String,
    namespace_uri: String,
    byte_offset: Option<u64>,
}

fn namespace_frame_for_start(
    request: &RelaxNgSourceValidationRequest<'_>,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    parent: Option<&NamespaceFrame>,
) -> (NamespaceFrame, Vec<Diagnostic>) {
    let mut frame = parent.cloned().unwrap_or_default();
    let mut diagnostics = Vec::new();

    for attr in start.attributes().with_checks(false) {
        match attr {
            Ok(attr) => {
                let key = qname_display(attr.key.as_ref());
                let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                if key == "xmlns" {
                    frame.default_namespace = value;
                } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                    frame.prefixes.insert(prefix.to_owned(), value);
                }
            }
            Err(error) => diagnostics.push(relax_ng_diagnostic(
                request,
                source,
                None,
                "cem.relax_ng.xml_parse_error",
                format!("RELAX NG XML attribute parse error: {error}"),
            )),
        }
    }

    (frame, diagnostics)
}

fn relax_ng_xml_element(
    reader: &quick_xml::Reader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
    frame: &NamespaceFrame,
) -> RelaxNgXmlElement {
    let qualified_name = qname_display(start.name().as_ref());
    let (prefix, local_name) = qname_parts(&qualified_name);
    let namespace_uri = prefix
        .and_then(|prefix| frame.prefixes.get(prefix).cloned())
        .unwrap_or_else(|| frame.default_namespace.clone());
    RelaxNgXmlElement {
        qualified_name,
        local_name,
        namespace_uri,
        byte_offset: reader
            .buffer_position()
            .checked_sub(start.as_ref().len() as u64 + 2),
    }
}

fn inspect_relax_ng_xml_element(
    request: &RelaxNgSourceValidationRequest<'_>,
    source: &str,
    element: &RelaxNgXmlElement,
    root_seen: &mut bool,
    root_is_grammar: &mut bool,
    start_seen: &mut bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !*root_seen {
        *root_seen = true;
        *root_is_grammar = element.namespace_uri == RELAX_NG_STRUCTURE_NAMESPACE
            && element.local_name == "grammar";
        if !*root_is_grammar {
            diagnostics.push(relax_ng_unknown_element_diagnostic(
                request, source, element,
            ));
        }
    }

    if element.namespace_uri == RELAX_NG_STRUCTURE_NAMESPACE {
        if !relax_ng_known_xml_element(&element.local_name) {
            diagnostics.push(relax_ng_unknown_element_diagnostic(
                request, source, element,
            ));
        } else if element.local_name == "start" {
            *start_seen = true;
        } else if matches!(element.local_name.as_str(), "include" | "externalRef") {
            diagnostics.push(relax_ng_diagnostic(
                request,
                source,
                element.byte_offset,
                if element.local_name == "include" {
                    "cem.relax_ng.include_rejected"
                } else {
                    "cem.relax_ng.external_ref_rejected"
                },
                format!(
                    "RELAX NG `{}` is rejected until resolver policy enables it",
                    element.local_name
                ),
            ));
        }
    }
}

fn relax_ng_known_xml_element(local_name: &str) -> bool {
    matches!(
        local_name,
        "grammar"
            | "start"
            | "define"
            | "element"
            | "attribute"
            | "choice"
            | "group"
            | "interleave"
            | "oneOrMore"
            | "zeroOrMore"
            | "optional"
            | "list"
            | "mixed"
            | "ref"
            | "parentRef"
            | "empty"
            | "text"
            | "value"
            | "data"
            | "param"
            | "notAllowed"
            | "externalRef"
            | "include"
            | "div"
            | "name"
            | "anyName"
            | "nsName"
            | "except"
    )
}

fn first_unbalanced_compact_brace(source: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut first_open: Option<usize> = None;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    for (offset, ch) in source.char_indices() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '#' => in_comment = true,
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    first_open = Some(offset);
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                if depth == 0 {
                    return Some(offset);
                }
                depth -= 1;
                if depth == 0 {
                    first_open = None;
                }
            }
            _ => {}
        }
    }

    if in_string {
        source.len().checked_sub(1)
    } else if depth == 0 {
        None
    } else {
        first_open
    }
}

fn compact_source_declares_start(source: &str) -> bool {
    source
        .lines()
        .map(|line| line.split_once('#').map(|(code, _)| code).unwrap_or(line))
        .any(|line| line.trim_start().starts_with("start"))
}

fn relax_ng_unknown_element_diagnostic(
    request: &RelaxNgSourceValidationRequest<'_>,
    source: &str,
    element: &RelaxNgXmlElement,
) -> Diagnostic {
    relax_ng_diagnostic(
        request,
        source,
        element.byte_offset,
        "cem.relax_ng.unknown_element",
        format!(
            "RELAX NG XML syntax element `{}` is not in the RELAX NG structure vocabulary",
            element.qualified_name
        ),
    )
}

fn relax_ng_unsupported_utf8_diagnostic(
    request: &RelaxNgSourceValidationRequest<'_>,
    error: &std::str::Utf8Error,
) -> Diagnostic {
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        byte_offset: u64::try_from(error.valid_up_to()).ok(),
        code: "cem.relax_ng.unsupported_encoding".to_owned(),
        severity: Severity::Error,
        message: format!("RELAX NG source must be valid UTF-8: {error}"),
        ..Diagnostic::default()
    }
}

fn relax_ng_diagnostic(
    request: &RelaxNgSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    code: &'static str,
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
        severity: Severity::Error,
        message,
        ..Diagnostic::default()
    }
}

fn qname_display(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn qname_parts(qualified_name: &str) -> (Option<&str>, String) {
    if let Some((prefix, local)) = qualified_name.split_once(':') {
        (Some(prefix), local.to_owned())
    } else {
        (None, qualified_name.to_owned())
    }
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

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn relax_ng_xml_validator_accepts_basic_grammar() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><empty/></start></grammar>"#,
            source_uri: "fixture.rng",
            content_type: Some("application/relax-ng+xml"),
        });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn relax_ng_xml_validator_reports_missing_start() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><define name="x"><empty/></define></grammar>"#,
            source_uri: "fixture.rng",
            content_type: Some("application/relax-ng+xml"),
        });

        assert!(has_code(&diagnostics, "cem.relax_ng.missing_start"));
    }

    #[test]
    fn relax_ng_xml_validator_reports_unknown_element() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><unknown/></start></grammar>"#,
            source_uri: "fixture.rng",
            content_type: Some("application/relax-ng+xml"),
        });

        assert!(has_code(&diagnostics, "cem.relax_ng.unknown_element"));
    }

    #[test]
    fn relax_ng_xml_validator_preserves_foreign_annotations() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0" xmlns:a="urn:annotation"><start><element name="note"><a:documentation>Visible to tooling.</a:documentation><text/></element></start></grammar>"#,
            source_uri: "fixture.rng",
            content_type: Some("application/relax-ng+xml"),
        });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn relax_ng_xml_validator_rejects_external_ref() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><externalRef href="https://example.test/schema.rng"/></start></grammar>"#,
            source_uri: "fixture.rng",
            content_type: Some("application/relax-ng+xml"),
        });

        assert!(has_code(&diagnostics, "cem.relax_ng.external_ref_rejected"));
    }

    #[test]
    fn relax_ng_compact_validator_reports_unbalanced_braces() {
        let diagnostics = validate_relax_ng_source_bytes(RelaxNgSourceValidationRequest {
            bytes: b"start = element note { text\n",
            source_uri: "fixture.rnc",
            content_type: Some("application/relax-ng-compact-syntax"),
        });

        assert!(has_code(&diagnostics, "cem.relax_ng.compact_parse_error"));
    }
}
