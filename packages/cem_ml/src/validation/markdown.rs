use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::{content_type_essence, MARKDOWN_CONTENT_TYPE};

#[derive(Debug, Clone, Copy)]
pub struct MarkdownSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_markdown_source_bytes(
    request: MarkdownSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if request
        .content_type
        .is_some_and(markdown_content_type_missing_charset)
    {
        diagnostics.push(markdown_charset_missing_diagnostic(&request));
    }

    let variant = request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "variant"));
    if let Some(variant) = variant.as_deref() {
        if !markdown_variant_is_known(variant) {
            diagnostics.push(markdown_unknown_variant_diagnostic(&request, variant));
        }
    }

    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            diagnostics.push(markdown_unsupported_encoding_diagnostic(&request, &error));
            return diagnostics;
        }
    };

    let options = markdown_parser_options(variant.as_deref());
    let parser = pulldown_cmark::Parser::new_ext(source, options).into_offset_iter();
    for (event, range) in parser {
        if matches!(
            event,
            pulldown_cmark::Event::Html(_) | pulldown_cmark::Event::InlineHtml(_)
        ) {
            diagnostics.push(markdown_embedded_html_rejected_diagnostic(
                &request,
                source,
                range.start,
            ));
        }
    }

    diagnostics
}

fn markdown_content_type_missing_charset(content_type: &str) -> bool {
    content_type_essence(content_type) == MARKDOWN_CONTENT_TYPE
        && content_type_parameter(content_type, "charset").is_none()
}

fn markdown_variant_is_known(variant: &str) -> bool {
    matches!(
        variant.trim().to_ascii_lowercase().as_str(),
        "commonmark" | "gfm" | "github-flavored-markdown"
    )
}

fn markdown_parser_options(variant: Option<&str>) -> pulldown_cmark::Options {
    let mut options = pulldown_cmark::Options::empty();
    if variant.map(str::trim).is_some_and(|variant| {
        variant.eq_ignore_ascii_case("gfm")
            || variant.eq_ignore_ascii_case("github-flavored-markdown")
    }) {
        options.insert(pulldown_cmark::Options::ENABLE_GFM);
    }
    options
}

fn markdown_charset_missing_diagnostic(
    request: &MarkdownSourceValidationRequest<'_>,
) -> Diagnostic {
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        code: "cem.markdown.charset_missing".to_owned(),
        severity: Severity::Warning,
        message: "text/markdown content type should include an explicit charset parameter"
            .to_owned(),
        ..Diagnostic::default()
    }
}

fn markdown_unknown_variant_diagnostic(
    request: &MarkdownSourceValidationRequest<'_>,
    variant: &str,
) -> Diagnostic {
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        code: "cem.markdown.unknown_variant".to_owned(),
        severity: Severity::Warning,
        message: format!("unknown Markdown variant `{variant}`"),
        ..Diagnostic::default()
    }
}

fn markdown_unsupported_encoding_diagnostic(
    request: &MarkdownSourceValidationRequest<'_>,
    error: &std::str::Utf8Error,
) -> Diagnostic {
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        byte_offset: u64::try_from(error.valid_up_to()).ok(),
        code: "cem.markdown.unsupported_encoding".to_owned(),
        severity: Severity::Error,
        message: format!("Markdown source must be valid UTF-8: {error}"),
        ..Diagnostic::default()
    }
}

fn markdown_embedded_html_rejected_diagnostic(
    request: &MarkdownSourceValidationRequest<'_>,
    source: &str,
    byte_offset: usize,
) -> Diagnostic {
    let (line, column) = line_col(source, byte_offset);
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        line: Some(line),
        column: Some(column),
        byte_offset: Some(byte_offset as u64),
        code: "cem.markdown.embedded_html_rejected".to_owned(),
        severity: Severity::Error,
        message: "Markdown embedded HTML is rejected unless an explicit policy permits it"
            .to_owned(),
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
        validate_markdown_source_bytes(MarkdownSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.md",
            content_type: Some("text/markdown; charset=utf-8; variant=CommonMark"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn markdown_source_validator_accepts_commonmark() {
        let diagnostics = validate(
            "# Release Notes\n\nThis document has **strong** text and a list.\n\n- Added schema validation.\n",
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn markdown_source_validator_reports_charset_missing() {
        let diagnostics = validate_markdown_source_bytes(MarkdownSourceValidationRequest {
            bytes: b"# Title\n",
            source_uri: "fixture.md",
            content_type: Some("text/markdown"),
        });

        assert!(has_code(&diagnostics, "cem.markdown.charset_missing"));
    }

    #[test]
    fn markdown_source_validator_reports_unknown_variant() {
        let diagnostics = validate_markdown_source_bytes(MarkdownSourceValidationRequest {
            bytes: b"# Title\n",
            source_uri: "fixture.md",
            content_type: Some("text/markdown; charset=utf-8; variant=CustomWiki"),
        });

        assert!(has_code(&diagnostics, "cem.markdown.unknown_variant"));
    }

    #[test]
    fn markdown_source_validator_reports_embedded_html() {
        let diagnostics = validate("# Unsafe\n\n<script>alert('x')</script>\n");

        assert!(has_code(
            &diagnostics,
            "cem.markdown.embedded_html_rejected"
        ));
    }

    #[test]
    fn markdown_source_validator_reports_unsupported_encoding() {
        let diagnostics = validate_markdown_source_bytes(MarkdownSourceValidationRequest {
            bytes: b"# Bad\n\xff\n",
            source_uri: "fixture.md",
            content_type: Some("text/markdown; charset=utf-8"),
        });

        assert!(has_code(&diagnostics, "cem.markdown.unsupported_encoding"));
    }
}
