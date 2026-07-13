use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::{content_type_essence, MATHML_NAMESPACE_URI};
use std::collections::BTreeMap;

const MATHML_PRESENTATION_CONTENT_TYPE: &str = "application/mathml-presentation+xml";
const MATHML_CONTENT_CONTENT_TYPE: &str = "application/mathml-content+xml";

#[derive(Debug, Clone, Copy)]
pub struct MathMlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_mathml_source_bytes(request: MathMlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let profile = mathml_media_profile(&request, &mut diagnostics);
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            diagnostics.push(mathml_not_well_formed_diagnostic(
                &request,
                "",
                u64::try_from(error.valid_up_to()).ok(),
                format!("MathML source must be valid UTF-8: {error}"),
            ));
            return diagnostics;
        }
    };

    diagnostics.extend(validate_mathml_source(&request, source, profile));
    diagnostics
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MathMlMediaProfile {
    Generic,
    Presentation,
    Content,
}

#[derive(Clone, Debug)]
struct MathMlAttributeView {
    local_name: String,
    value: String,
}

#[derive(Clone, Debug)]
struct MathMlElementFrame {
    local_name: String,
    namespace_uri: String,
    attributes: Vec<MathMlAttributeView>,
}

#[derive(Clone, Debug, Default)]
struct MathMlDocumentState {
    root_is_math: bool,
    saw_presentation: bool,
    saw_content: bool,
    reported_external_annotation: bool,
}

fn mathml_media_profile(
    request: &MathMlSourceValidationRequest<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> MathMlMediaProfile {
    let essence = request.content_type.map(content_type_essence);
    let mut profile = match essence.as_deref() {
        Some(MATHML_PRESENTATION_CONTENT_TYPE) => MathMlMediaProfile::Presentation,
        Some(MATHML_CONTENT_CONTENT_TYPE) => MathMlMediaProfile::Content,
        _ => MathMlMediaProfile::Generic,
    };

    if let Some(parameter) = request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "profile"))
    {
        match parameter.trim().to_ascii_lowercase().as_str() {
            "generic" => profile = MathMlMediaProfile::Generic,
            "presentation" => profile = MathMlMediaProfile::Presentation,
            "content" => profile = MathMlMediaProfile::Content,
            _ => diagnostics.push(mathml_diagnostic(
                request,
                "",
                None,
                "cem.mathml.unsupported_profile",
                Severity::Warning,
                format!("MathML media profile `{parameter}` is not supported"),
            )),
        }
    }

    profile
}

fn validate_mathml_source(
    request: &MathMlSourceValidationRequest<'_>,
    source: &str,
    profile: MathMlMediaProfile,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;

    let mut element_stack: Vec<MathMlElementFrame> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut state = MathMlDocumentState::default();

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                let start_offset = xml_event_position(&reader, &start, false);
                let (frame, namespaces, mut event_diagnostics) =
                    mathml_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                mathml_validate_element(
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
                    mathml_start_frame(request, source, &start, &namespace_stack, start_offset);
                diagnostics.append(&mut event_diagnostics);
                mathml_validate_element(
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
                    diagnostics.push(mathml_not_well_formed_diagnostic(
                        request,
                        source,
                        Some(reader.error_position()),
                        "MathML document cannot contain character data outside the document element"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => {
                diagnostics.push(mathml_not_well_formed_diagnostic(
                    request,
                    source,
                    Some(reader.error_position()),
                    "MathML DOCTYPE declarations are rejected by default".to_owned(),
                ));
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(mathml_xml_error_diagnostic(
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
        diagnostics.push(mathml_not_well_formed_diagnostic(
            request,
            source,
            Some(0),
            "MathML document must contain a document element".to_owned(),
        ));
    } else if state.root_is_math {
        match profile {
            MathMlMediaProfile::Generic => {}
            MathMlMediaProfile::Presentation if !state.saw_presentation => {
                diagnostics.push(mathml_diagnostic(
                    request,
                    source,
                    Some(0),
                    "cem.mathml.malformed_expression",
                    Severity::Error,
                    "application/mathml-presentation+xml must contain presentation MathML"
                        .to_owned(),
                ));
            }
            MathMlMediaProfile::Content if !state.saw_content => {
                diagnostics.push(mathml_diagnostic(
                    request,
                    source,
                    Some(0),
                    "cem.mathml.malformed_expression",
                    Severity::Error,
                    "application/mathml-content+xml must contain content MathML".to_owned(),
                ));
            }
            _ => {}
        }
    }

    diagnostics
}

fn mathml_start_frame(
    request: &MathMlSourceValidationRequest<'_>,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
    byte_offset: Option<u64>,
) -> (
    MathMlElementFrame,
    BTreeMap<String, String>,
    Vec<Diagnostic>,
) {
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
            Err(error) => diagnostics.push(mathml_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                format!("MathML XML attribute parse error: {error}"),
            )),
        }
    }

    let qualified_name = qname_display(start.name().as_ref());
    if let Some(prefix) = xml_qname_prefix(&qualified_name) {
        if !xml_prefix_is_bound(&namespaces, prefix) {
            diagnostics.push(mathml_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                format!("MathML namespace prefix `{prefix}` is not bound for `{qualified_name}`"),
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
                    diagnostics.push(mathml_not_well_formed_diagnostic(
                        request,
                        source,
                        byte_offset,
                        format!(
                            "MathML namespace prefix `{prefix}` is not bound for attribute `{qualified_name}`"
                        ),
                    ));
                }
            }
            let (_, local_name) = xml_attribute_expanded_name(&qualified_name, &namespaces);
            MathMlAttributeView { local_name, value }
        })
        .collect();

    (
        MathMlElementFrame {
            local_name,
            namespace_uri,
            attributes,
        },
        namespaces,
        diagnostics,
    )
}

fn mathml_validate_element(
    request: &MathMlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    frame: &MathMlElementFrame,
    element_stack: &[MathMlElementFrame],
    state: &mut MathMlDocumentState,
    root_count: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if element_stack.is_empty() {
        *root_count += 1;
        if *root_count > 1 {
            diagnostics.push(mathml_not_well_formed_diagnostic(
                request,
                source,
                byte_offset,
                "MathML document must have exactly one document element".to_owned(),
            ));
            return;
        }
        if frame.local_name != "math" {
            diagnostics.push(mathml_diagnostic(
                request,
                source,
                byte_offset,
                "cem.mathml.root_not_math",
                Severity::Error,
                format!(
                    "MathML root element must be `math`, found `{}`",
                    frame.local_name
                ),
            ));
            return;
        }
        if frame.namespace_uri != MATHML_NAMESPACE_URI {
            diagnostics.push(mathml_diagnostic(
                request,
                source,
                byte_offset,
                "cem.mathml.namespace_missing",
                Severity::Error,
                "MathML root `math` element must use the http://www.w3.org/1998/Math/MathML namespace"
                    .to_owned(),
            ));
            return;
        }

        state.root_is_math = true;
        return;
    }

    if frame.namespace_uri != MATHML_NAMESPACE_URI {
        return;
    }

    if mathml_is_presentation_element(&frame.local_name) {
        state.saw_presentation = true;
    }
    if mathml_is_content_element(&frame.local_name) {
        state.saw_content = true;
    }
    if matches!(frame.local_name.as_str(), "annotation" | "annotation-xml")
        && !state.reported_external_annotation
    {
        if let Some(attribute) = frame.attributes.iter().find(|attribute| {
            attribute.local_name == "src" && mathml_src_requires_policy(&attribute.value)
        }) {
            state.reported_external_annotation = true;
            diagnostics.push(mathml_diagnostic(
                request,
                source,
                byte_offset,
                "cem.mathml.external_annotation_rejected",
                Severity::Warning,
                format!(
                    "MathML annotation src `{}` requires explicit loader policy",
                    attribute.value
                ),
            ));
        }
    }
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

fn mathml_src_requires_policy(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    !(trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.to_ascii_lowercase().starts_with("data:"))
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

fn mathml_xml_error_diagnostic(
    request: &MathMlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    error: &quick_xml::Error,
) -> Diagnostic {
    mathml_not_well_formed_diagnostic(
        request,
        source,
        byte_offset,
        format!("MathML XML parse error: {error}"),
    )
}

fn mathml_not_well_formed_diagnostic(
    request: &MathMlSourceValidationRequest<'_>,
    source: &str,
    byte_offset: Option<u64>,
    message: String,
) -> Diagnostic {
    mathml_diagnostic(
        request,
        source,
        byte_offset,
        "cem.mathml.not_well_formed_xml",
        Severity::Error,
        message,
    )
}

fn mathml_diagnostic(
    request: &MathMlSourceValidationRequest<'_>,
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

    fn validate(source: &str, content_type: &str) -> Vec<Diagnostic> {
        validate_mathml_source_bytes(MathMlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.mml",
            content_type: Some(content_type),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn mathml_source_validator_accepts_basic_presentation() {
        let diagnostics = validate(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="inline">
  <mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow>
</math>
"#,
            "application/mathml+xml",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn mathml_source_validator_accepts_content_alias() {
        let diagnostics = validate(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="block">
  <apply><plus/><ci>x</ci><cn>1</cn></apply>
</math>
"#,
            "application/mathml-content+xml",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn mathml_source_validator_reports_missing_namespace() {
        let diagnostics = validate(
            r#"<math display="inline">
  <mi>x</mi>
</math>
"#,
            "application/mathml+xml",
        );

        assert!(has_code(&diagnostics, "cem.mathml.namespace_missing"));
    }

    #[test]
    fn mathml_source_validator_reports_root_not_math() {
        let diagnostics = validate(
            r#"<mrow xmlns="http://www.w3.org/1998/Math/MathML">
  <mi>x</mi>
</mrow>
"#,
            "application/mathml+xml",
        );

        assert!(has_code(&diagnostics, "cem.mathml.root_not_math"));
    }

    #[test]
    fn mathml_source_validator_reports_content_profile_mismatch() {
        let diagnostics = validate(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML">
  <mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow>
</math>
"#,
            "application/mathml-content+xml",
        );

        assert!(has_code(&diagnostics, "cem.mathml.malformed_expression"));
    }

    #[test]
    fn mathml_source_validator_reports_external_annotation_warning() {
        let diagnostics = validate(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML" alttext="x squared">
  <semantics>
    <msup><mi>x</mi><mn>2</mn></msup>
    <annotation encoding="application/json" src="formula.json"/>
  </semantics>
</math>
"#,
            "application/mathml+xml",
        );

        assert!(has_code(
            &diagnostics,
            "cem.mathml.external_annotation_rejected"
        ));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }

    #[test]
    fn mathml_source_validator_reports_unsupported_profile_warning() {
        let diagnostics = validate(
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML">
  <mi>x</mi>
</math>
"#,
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
            r#"<math xmlns="http://www.w3.org/1998/Math/MathML">
  <mrow>
    <mi>x</mrow>
</math>
"#,
            "application/mathml+xml",
        );

        assert!(has_code(&diagnostics, "cem.mathml.not_well_formed_xml"));
    }
}
