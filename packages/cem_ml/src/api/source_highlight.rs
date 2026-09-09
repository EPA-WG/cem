use serde::{Deserialize, Serialize};

use crate::diagnostics::{project_diagnostics_for_source, Diagnostic, Severity};
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};
use crate::validation::html::{
    html_document_ast_from_source_bytes, html_event_markup_tokens, HtmlEventKind,
    HtmlSourceValidationRequest,
};

pub const SOURCE_HIGHLIGHT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceHighlightRequestV1 {
    source: String,
    content_type: String,
    #[serde(default)]
    source_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceHighlightResponseV1 {
    schema_version: u32,
    status: &'static str,
    html: String,
    diagnostics: Vec<Diagnostic>,
    spans: Vec<SourceHighlightSpanV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SourceHighlightSpanV1 {
    byte_offset: usize,
    byte_length: usize,
    role: &'static str,
}

impl SourceHighlightSpanV1 {
    fn end(&self) -> usize {
        self.byte_offset.saturating_add(self.byte_length)
    }
}

/// Highlight authored source without changing its bytes.
///
/// HTML roles come from the same lossless lexical pieces exposed to
/// `html.format-document.tree`. CEM-ML roles come from the canonical tokenizer;
/// punctuation inside structural tokens is split only for presentation.
pub fn highlight_source_to_html_v1_json(request_json: &str) -> String {
    let request = match serde_json::from_str::<SourceHighlightRequestV1>(request_json) {
        Ok(request) => request,
        Err(error) => {
            return serialize_response(SourceHighlightResponseV1 {
                schema_version: SOURCE_HIGHLIGHT_SCHEMA_VERSION,
                status: "error",
                html: String::new(),
                diagnostics: vec![Diagnostic {
                    code: "cem.source_highlight.request_invalid".to_owned(),
                    severity: Severity::Error,
                    message: format!("source-highlight request is invalid: {error}"),
                    ..Diagnostic::default()
                }],
                spans: Vec::new(),
            });
        }
    };

    let content_type = request
        .content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let (status, mut diagnostics, spans) = match content_type.as_str() {
        "html" | "text/html" => {
            highlight_html_source(&request.source, request.source_url.as_deref())
        }
        "cem" | "cemml" | "cem-ml" | "application/cem" => highlight_cem_ml_source(&request.source),
        _ => ("unsupported", Vec::new(), Vec::new()),
    };
    project_diagnostics_for_source(&mut diagnostics, request.source.as_bytes());
    if let Some(source_url) = request.source_url {
        for diagnostic in &mut diagnostics {
            diagnostic.uri.get_or_insert_with(|| source_url.clone());
        }
    }
    let html = render_semantic_html(&request.source, &spans);
    serialize_response(SourceHighlightResponseV1 {
        schema_version: SOURCE_HIGHLIGHT_SCHEMA_VERSION,
        status,
        html,
        diagnostics,
        spans,
    })
}

fn highlight_html_source(
    source: &str,
    source_url: Option<&str>,
) -> (&'static str, Vec<Diagnostic>, Vec<SourceHighlightSpanV1>) {
    let (document, diagnostics) =
        html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: source_url.unwrap_or("memory:cem-source-highlight.html"),
            content_type: Some("text/html"),
        });
    let Some(document) = document else {
        return ("invalid", diagnostics, Vec::new());
    };
    let mut spans = Vec::new();
    for event in &document.events {
        let markup_tokens = html_event_markup_tokens(event);
        if !markup_tokens.is_empty() {
            for token in markup_tokens {
                push_span(
                    source,
                    &mut spans,
                    token.source_range.start.byte_offset as usize,
                    token.source_range.byte_length as usize,
                    token.kind.semantic_role(),
                );
            }
            continue;
        }
        let role = match event.kind {
            HtmlEventKind::Comment => "syntax.comment",
            HtmlEventKind::Text => "syntax.text",
            HtmlEventKind::RawText | HtmlEventKind::Rcdata => "syntax.string",
            HtmlEventKind::Doctype => "syntax.keyword",
            HtmlEventKind::StartElement | HtmlEventKind::EndElement => "syntax.raw",
        };
        push_span(
            source,
            &mut spans,
            event.source_range.start.byte_offset as usize,
            event.source_range.byte_length as usize,
            role,
        );
    }
    ("highlighted", diagnostics, normalize_spans(spans))
}

fn highlight_cem_ml_source(
    source: &str,
) -> (&'static str, Vec<Diagnostic>, Vec<SourceHighlightSpanV1>) {
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    let diagnostics = tokenizer.take_diagnostics();
    let mut spans = Vec::new();
    for token in &tokens {
        highlight_cem_token(source, token, &mut spans);
    }
    ("highlighted", diagnostics, normalize_spans(spans))
}

fn highlight_cem_token(source: &str, token: &SchemaToken, spans: &mut Vec<SourceHighlightSpanV1>) {
    let start = token.byte_range.start as usize;
    let end = token.byte_range.end() as usize;
    match &token.kind {
        SchemaTokenKind::NodeStart { .. } => {
            push_span(
                source,
                spans,
                start,
                usize::from(start < end),
                "syntax.punctuation",
            );
            let mut cursor = start.saturating_add(1);
            while cursor < end && source.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            push_span(
                source,
                spans,
                cursor,
                end.saturating_sub(cursor),
                "syntax.name",
            );
        }
        SchemaTokenKind::NodeEnd { .. } | SchemaTokenKind::AnonymousScopeStart => {
            highlight_cem_punctuation_and_raw(source, start, end, spans);
        }
        SchemaTokenKind::Attribute {
            name_range,
            value_range,
            ..
        } => {
            let name_start = name_range.start as usize;
            let name_end = name_range.end() as usize;
            highlight_cem_punctuation_and_raw(source, start, name_start, spans);
            push_span(
                source,
                spans,
                name_start,
                name_end.saturating_sub(name_start),
                "syntax.attribute",
            );
            if let Some(value_range) = value_range {
                let value_start = value_range.start as usize;
                let value_end = value_range.end() as usize;
                highlight_cem_punctuation_and_raw(source, name_end, value_start, spans);
                highlight_cem_value(source, value_start, value_end, spans);
                highlight_cem_punctuation_and_raw(source, value_end, end, spans);
            } else {
                highlight_cem_punctuation_and_raw(source, name_end, end, spans);
            }
        }
        SchemaTokenKind::Text(_) | SchemaTokenKind::ExpressionNode(_) => {
            push_span(
                source,
                spans,
                start,
                end.saturating_sub(start),
                "syntax.string",
            );
        }
        SchemaTokenKind::Trivia(_) => {
            highlight_cem_punctuation_and_raw(source, start, end, spans);
        }
        SchemaTokenKind::Comment(_) => {
            push_span(
                source,
                spans,
                start,
                end.saturating_sub(start),
                "syntax.comment",
            );
        }
        SchemaTokenKind::ProcessingInstruction { .. } => {
            highlight_wrapped_value(source, start, end, "<?", "?>", "syntax.keyword", spans);
        }
        SchemaTokenKind::Directive { name, .. } => {
            push_span(source, spans, start, 1, "syntax.punctuation");
            let name_start = start.saturating_add(1);
            let name_end = name_start.saturating_add(name.len());
            push_span(
                source,
                spans,
                name_start,
                name_end.saturating_sub(name_start),
                "syntax.keyword",
            );
            highlight_cem_literal_tail(source, name_end, end, spans);
        }
        SchemaTokenKind::RichContent { .. } => {
            highlight_wrapped_value(source, start, end, "```", "```", "syntax.string", spans);
        }
        SchemaTokenKind::Error { .. } => {
            push_span(
                source,
                spans,
                start,
                end.saturating_sub(start),
                "diagnostic.error",
            );
        }
    }
}

fn highlight_cem_value(
    source: &str,
    start: usize,
    end: usize,
    spans: &mut Vec<SourceHighlightSpanV1>,
) {
    let Some(value) = source.get(start..end) else {
        return;
    };
    let first = value.as_bytes().first().copied();
    let last = value.as_bytes().last().copied();
    if value.len() >= 2 && first == last && matches!(first, Some(b'\'' | b'"')) {
        push_span(source, spans, start, 1, "syntax.punctuation");
        push_span(
            source,
            spans,
            start + 1,
            value.len().saturating_sub(2),
            "syntax.string",
        );
        push_span(source, spans, end - 1, 1, "syntax.punctuation");
    } else if value.len() >= 2 && value.starts_with('{') && value.ends_with('}') {
        push_span(source, spans, start, 1, "syntax.punctuation");
        push_span(
            source,
            spans,
            start + 1,
            value.len().saturating_sub(2),
            "syntax.string",
        );
        push_span(source, spans, end - 1, 1, "syntax.punctuation");
    } else {
        push_span(
            source,
            spans,
            start,
            end.saturating_sub(start),
            "syntax.string",
        );
    }
}

fn highlight_wrapped_value(
    source: &str,
    start: usize,
    end: usize,
    open: &str,
    close: &str,
    body_role: &'static str,
    spans: &mut Vec<SourceHighlightSpanV1>,
) {
    let Some(value) = source.get(start..end) else {
        return;
    };
    let has_open = value.starts_with(open);
    let has_close = value.len() >= open.len() + close.len() && value.ends_with(close);
    let body_start = start + if has_open { open.len() } else { 0 };
    let body_end = end.saturating_sub(if has_close { close.len() } else { 0 });
    if has_open {
        push_span(source, spans, start, open.len(), "syntax.punctuation");
    }
    push_span(
        source,
        spans,
        body_start,
        body_end.saturating_sub(body_start),
        body_role,
    );
    if has_close {
        push_span(source, spans, body_end, close.len(), "syntax.punctuation");
    }
}

fn highlight_cem_literal_tail(
    source: &str,
    start: usize,
    end: usize,
    spans: &mut Vec<SourceHighlightSpanV1>,
) {
    let mut cursor = start;
    let bytes = source.as_bytes();
    while cursor < end {
        if bytes[cursor].is_ascii_whitespace() {
            let token_start = cursor;
            while cursor < end && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            push_span(
                source,
                spans,
                token_start,
                cursor - token_start,
                "syntax.raw",
            );
        } else if matches!(bytes[cursor], b'=' | b'\'' | b'"') {
            push_span(source, spans, cursor, 1, "syntax.punctuation");
            cursor += 1;
        } else {
            let token_start = cursor;
            while cursor < end
                && !bytes[cursor].is_ascii_whitespace()
                && !matches!(bytes[cursor], b'=' | b'\'' | b'"')
            {
                cursor += 1;
            }
            push_span(
                source,
                spans,
                token_start,
                cursor - token_start,
                "syntax.string",
            );
        }
    }
}

fn highlight_cem_punctuation_and_raw(
    source: &str,
    start: usize,
    end: usize,
    spans: &mut Vec<SourceHighlightSpanV1>,
) {
    let mut cursor = start;
    while cursor < end {
        let Some(ch) = source[cursor..end].chars().next() else {
            break;
        };
        let role = if matches!(ch, '{' | '}' | '|' | '▷' | '@' | '=' | '\'' | '"') {
            "syntax.punctuation"
        } else {
            "syntax.raw"
        };
        let token_start = cursor;
        cursor += ch.len_utf8();
        while cursor < end {
            let Some(next) = source[cursor..end].chars().next() else {
                break;
            };
            let next_role = if matches!(next, '{' | '}' | '|' | '▷' | '@' | '=' | '\'' | '"') {
                "syntax.punctuation"
            } else {
                "syntax.raw"
            };
            if next_role != role {
                break;
            }
            cursor += next.len_utf8();
        }
        push_span(source, spans, token_start, cursor - token_start, role);
    }
}

fn push_span(
    source: &str,
    spans: &mut Vec<SourceHighlightSpanV1>,
    byte_offset: usize,
    byte_length: usize,
    role: &'static str,
) {
    let end = byte_offset.saturating_add(byte_length);
    if byte_length == 0
        || end > source.len()
        || !source.is_char_boundary(byte_offset)
        || !source.is_char_boundary(end)
    {
        return;
    }
    spans.push(SourceHighlightSpanV1 {
        byte_offset,
        byte_length,
        role,
    });
}

fn normalize_spans(mut spans: Vec<SourceHighlightSpanV1>) -> Vec<SourceHighlightSpanV1> {
    spans.sort_by_key(|span| (span.byte_offset, span.byte_length));
    let mut normalized: Vec<SourceHighlightSpanV1> = Vec::with_capacity(spans.len());
    for mut span in spans {
        if let Some(previous) = normalized.last() {
            if span.byte_offset < previous.end() {
                let overlap = previous.end() - span.byte_offset;
                if overlap >= span.byte_length {
                    continue;
                }
                span.byte_offset += overlap;
                span.byte_length -= overlap;
            }
        }
        if let Some(previous) = normalized.last_mut() {
            if previous.end() == span.byte_offset && previous.role == span.role {
                previous.byte_length += span.byte_length;
                continue;
            }
        }
        normalized.push(span);
    }
    normalized
}

fn render_semantic_html(source: &str, spans: &[SourceHighlightSpanV1]) -> String {
    let mut html = String::with_capacity(source.len() + spans.len() * 8);
    let mut cursor = 0usize;
    for span in spans {
        if span.byte_offset > cursor {
            escape_html_into(&source[cursor..span.byte_offset], &mut html);
        }
        let end = span.end();
        if let Some(tag) = semantic_html_tag(span.role) {
            html.push('<');
            html.push_str(tag);
            html.push('>');
            escape_html_into(&source[span.byte_offset..end], &mut html);
            html.push_str("</");
            html.push_str(tag);
            html.push('>');
        } else {
            escape_html_into(&source[span.byte_offset..end], &mut html);
        }
        cursor = end;
    }
    if cursor < source.len() {
        escape_html_into(&source[cursor..], &mut html);
    }
    html
}

fn semantic_html_tag(role: &str) -> Option<&'static str> {
    match role {
        "syntax.name" => Some("b"),
        "syntax.attribute" => Some("var"),
        "syntax.keyword" => Some("strong"),
        "syntax.string" => Some("i"),
        "syntax.number" => Some("u"),
        "syntax.comment" => Some("small"),
        "syntax.text" => Some("samp"),
        "diagnostic.error" => Some("mark"),
        "syntax.punctuation" | "syntax.raw" => None,
        _ => None,
    }
}

fn escape_html_into(value: &str, output: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
}

fn serialize_response(response: SourceHighlightResponseV1) -> String {
    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"schemaVersion":1,"status":"error","html":"","diagnostics":[{"uri":null,"line":null,"column":null,"byteOffset":null,"code":"cem.source_highlight.serialize_failed","severity":"fatal","message":"source-highlight response serialization failed"}],"spans":[]}"#.to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(request: &str) -> serde_json::Value {
        serde_json::from_str(&highlight_source_to_html_v1_json(request)).unwrap()
    }

    #[test]
    fn html_source_roles_split_markup_primitives_and_preserve_visible_source() {
        let source = r#"<!-- card --><article data-kind='hero' hidden>Ready</article>"#;
        let value = response(
            &serde_json::json!({ "source": source, "contentType": "text/html" }).to_string(),
        );

        assert_eq!(value["status"], "highlighted");
        let html = value["html"].as_str().unwrap();
        assert!(html.contains("<small>&lt;!-- card --&gt;</small>"));
        assert!(html.contains("&lt;<b>article</b>"));
        assert!(html.contains("<var>data-kind</var>=&#39;<i>hero</i>&#39;"));
        assert!(html.contains("<samp>Ready</samp>"));
        assert!(!html.contains("class="));
        assert!(!html.contains("data-role="));
        assert_eq!(semantic_html_text(html), source);
    }

    #[test]
    fn cem_ml_source_roles_split_braces_attributes_quotes_and_values() {
        let source = r#"{article @data-kind="hero" | {strong | Ready}}"#;
        let value = response(
            &serde_json::json!({ "source": source, "contentType": "application/cem" }).to_string(),
        );

        assert_eq!(value["status"], "highlighted");
        let html = value["html"].as_str().unwrap();
        assert!(html.contains("{<b>article</b>"));
        assert!(html.contains("@<var>data-kind</var>=&quot;<i>hero</i>&quot;"));
        assert!(!html.contains("class="));
        assert!(!html.contains("data-role="));
        assert_eq!(semantic_html_text(html), source);
    }

    #[test]
    fn cem_ml_tokenizer_errors_have_an_explicit_diagnostic_role() {
        let source = "{article | Invalid interpolation: {42}}";
        let value = response(
            &serde_json::json!({ "source": source, "contentType": "application/cem" }).to_string(),
        );

        assert_eq!(value["status"], "highlighted");
        assert_eq!(
            value["diagnostics"][0]["code"],
            "cem.tokenizer.bare_brace_text"
        );
        assert!(value["html"]
            .as_str()
            .unwrap()
            .contains("<mark>{42}</mark>"));
    }

    #[test]
    fn unsupported_source_type_returns_safe_uncolored_html() {
        let value = response(r#"{"source":"a < b","contentType":"text/plain"}"#);
        assert_eq!(value["status"], "unsupported");
        assert_eq!(value["html"], "a &lt; b");
        assert!(value["spans"].as_array().unwrap().is_empty());
    }

    #[test]
    fn malformed_request_is_a_structured_error() {
        let value = response("{}");
        assert_eq!(value["status"], "error");
        assert_eq!(
            value["diagnostics"][0]["code"],
            "cem.source_highlight.request_invalid"
        );
    }

    fn semantic_html_text(html: &str) -> String {
        let mut text = html.to_owned();
        for tag in ["b", "var", "strong", "i", "u", "small", "samp", "mark"] {
            text = text.replace(&format!("<{tag}>"), "");
            text = text.replace(&format!("</{tag}>"), "");
        }
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&")
    }
}
