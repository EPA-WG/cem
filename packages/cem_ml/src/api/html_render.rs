use serde::{Deserialize, Serialize};

use crate::diagnostics::{project_diagnostics_for_source, Diagnostic, Severity};
use crate::interpreter::OutputSpan;

pub const CEM_ML_HTML_RENDER_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CemMlHtmlRenderRequestV1 {
    source: String,
    #[serde(default)]
    source_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CemMlHtmlRenderResponseV1 {
    schema_version: u32,
    status: &'static str,
    html: String,
    diagnostics: Vec<Diagnostic>,
    output_spans: Vec<OutputSpan>,
}

/// Parse CEM-ML and project its static tree to light-DOM HTML.
///
/// The JSON envelope keeps the WASM ABI extensible and gives browser callers
/// the same diagnostics and output/source correspondence needed by Studio.
/// An invalid source can retain recovered HTML in the response, but consumers
/// must not inject it unless `status` is `rendered`.
pub fn render_cem_ml_to_html_v1_json(request_json: &str) -> String {
    let request = match serde_json::from_str::<CemMlHtmlRenderRequestV1>(request_json) {
        Ok(request) => request,
        Err(error) => {
            return serialize_response(CemMlHtmlRenderResponseV1 {
                schema_version: CEM_ML_HTML_RENDER_SCHEMA_VERSION,
                status: "error",
                html: String::new(),
                diagnostics: vec![Diagnostic {
                    code: "cem.html_render.request_invalid".to_owned(),
                    severity: Severity::Error,
                    message: format!("CEM-ML HTML render request is invalid: {error}"),
                    ..Diagnostic::default()
                }],
                output_spans: Vec::new(),
            });
        }
    };

    let mut output = crate::interpreter::light_dom::render_html(&request.source);
    project_diagnostics_for_source(&mut output.diagnostics, request.source.as_bytes());
    if let Some(source_url) = request.source_url {
        for diagnostic in &mut output.diagnostics {
            diagnostic.uri.get_or_insert_with(|| source_url.clone());
        }
    }
    let status = if output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        "invalid"
    } else {
        "rendered"
    };

    serialize_response(CemMlHtmlRenderResponseV1 {
        schema_version: CEM_ML_HTML_RENDER_SCHEMA_VERSION,
        status,
        html: output.rendered,
        diagnostics: output.diagnostics,
        output_spans: output.output_spans,
    })
}

fn serialize_response(response: CemMlHtmlRenderResponseV1) -> String {
    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"schemaVersion":1,"status":"error","html":"","diagnostics":[{"uri":null,"line":null,"column":null,"byteOffset":null,"code":"cem.html_render.serialize_failed","severity":"fatal","message":"CEM-ML HTML render response serialization failed"}],"outputSpans":[]}"#.to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_cem_ml_to_structured_html_with_source_spans() {
        let response: serde_json::Value = serde_json::from_str(&render_cem_ml_to_html_v1_json(
            r#"{"source":"{article @id=welcome | {strong | Hello}}","sourceUrl":"memory:welcome.cem"}"#,
        ))
        .unwrap();

        assert_eq!(response["schemaVersion"], 1);
        assert_eq!(response["status"], "rendered");
        assert_eq!(
            response["html"],
            "<article id=\"welcome\"><strong>Hello</strong></article>"
        );
        assert!(response["diagnostics"].as_array().unwrap().is_empty());
        assert!(!response["outputSpans"].as_array().unwrap().is_empty());
    }

    #[test]
    fn escapes_cem_ml_text_for_safe_html_projection() {
        let response: serde_json::Value = serde_json::from_str(&render_cem_ml_to_html_v1_json(
            r#"{"source":"{p | one < two & three}"}"#,
        ))
        .unwrap();

        assert_eq!(response["status"], "rendered");
        assert_eq!(response["html"], "<p>one &lt; two &amp; three</p>");
    }

    #[test]
    fn rejects_malformed_request_envelopes_as_structured_diagnostics() {
        let response: serde_json::Value =
            serde_json::from_str(&render_cem_ml_to_html_v1_json("{}")).unwrap();

        assert_eq!(response["status"], "error");
        assert_eq!(response["html"], "");
        assert_eq!(
            response["diagnostics"][0]["code"],
            "cem.html_render.request_invalid"
        );
        assert_eq!(response["diagnostics"][0]["severity"], "error");
    }
}
