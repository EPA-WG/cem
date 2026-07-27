use crate::diagnostics::{Diagnostic, Severity};

#[derive(Debug, Clone, Copy)]
pub struct CssSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

pub fn validate_css_source_bytes(request: CssSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let declared_charset = request
        .content_type
        .and_then(|content_type| content_type_parameter(content_type, "charset"));
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            return vec![css_diagnostic(
                &request,
                "",
                u64::try_from(error.valid_up_to()).ok(),
                "cem.css.parse_error",
                Severity::Error,
                format!("CSS validator currently requires UTF-8-compatible input bytes: {error}"),
            )];
        }
    };

    validate_css_source(&request, source, declared_charset.as_deref())
}

#[derive(Clone, Debug, Default)]
struct CssDocumentState {
    reported_bad_string: bool,
    reported_bad_url: bool,
    reported_encoding_conflict: bool,
    reported_import: bool,
    reported_invalid_declaration: bool,
    reported_invalid_selector: bool,
    reported_invalid_token: bool,
    reported_unknown_at_rule: bool,
    reported_url: bool,
}

fn validate_css_source(
    request: &CssSourceValidationRequest<'_>,
    source: &str,
    declared_charset: Option<&str>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut state = CssDocumentState::default();

    if let Some((charset, offset)) = css_leading_charset(source) {
        if let Some(declared_charset) = declared_charset {
            if css_normalized_charset(declared_charset) != css_normalized_charset(&charset) {
                state.reported_encoding_conflict = true;
                diagnostics.push(css_diagnostic(
                    request,
                    source,
                    Some(offset as u64),
                    "cem.css.encoding_conflict",
                    Severity::Warning,
                    format!(
                        "CSS MIME charset `{}` conflicts with @charset `{}`",
                        declared_charset, charset
                    ),
                ));
            }
        }
    }

    let sanitized = css_sanitize_source(request, source, &mut state, &mut diagnostics);
    css_validate_delimiters(request, source, &sanitized, &mut state, &mut diagnostics);
    css_validate_at_rules_and_urls(request, source, &sanitized, &mut state, &mut diagnostics);
    css_validate_rule_shapes(request, source, &sanitized, &mut state, &mut diagnostics);

    diagnostics
}

fn css_leading_charset(source: &str) -> Option<(String, usize)> {
    let mut offset = source
        .strip_prefix('\u{feff}')
        .map_or(0, |_| '\u{feff}'.len_utf8());
    offset = css_skip_whitespace_and_comments(source, offset);
    let rest = source[offset..].trim_start();
    let skipped = source[offset..].len() - rest.len();
    offset += skipped;
    if !rest.to_ascii_lowercase().starts_with("@charset") {
        return None;
    }
    let after_keyword = &source[offset + "@charset".len()..];
    let after_ws = after_keyword.trim_start();
    let value_offset = offset + "@charset".len() + (after_keyword.len() - after_ws.len());
    let quote = after_ws.as_bytes().first().copied()?;
    if !matches!(quote, b'"' | b'\'') {
        return None;
    }
    let value_start = value_offset + 1;
    let value_rest = &source[value_start..];
    let value_end = value_rest.find(char::from(quote))?;
    Some((value_rest[..value_end].to_owned(), value_offset))
}

fn css_skip_whitespace_and_comments(source: &str, mut offset: usize) -> usize {
    while offset < source.len() {
        let rest = &source[offset..];
        if let Some(ch) = rest.chars().next() {
            if ch.is_whitespace() {
                offset += ch.len_utf8();
                continue;
            }
        }
        if rest.starts_with("/*") {
            if let Some(end) = rest.find("*/") {
                offset += end + 2;
                continue;
            }
        }
        break;
    }
    offset
}

fn css_normalized_charset(value: &str) -> String {
    value.trim().trim_matches('"').to_ascii_lowercase()
}

fn css_sanitize_source(
    request: &CssSourceValidationRequest<'_>,
    source: &str,
    state: &mut CssDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let mut sanitized = String::with_capacity(source.len());
    let mut chars = source.char_indices().peekable();

    while let Some((offset, ch)) = chars.next() {
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            sanitized.push(' ');
            let (_, _) = chars.next().expect("peeked comment opener");
            sanitized.push(' ');
            let mut closed = false;
            while let Some((_, comment_ch)) = chars.next() {
                sanitized.push(if comment_ch == '\n' { '\n' } else { ' ' });
                if comment_ch == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                    let (_, slash) = chars.next().expect("peeked comment closer");
                    sanitized.push(if slash == '\n' { '\n' } else { ' ' });
                    closed = true;
                    break;
                }
            }
            if !closed && !state.reported_invalid_token {
                state.reported_invalid_token = true;
                diagnostics.push(css_diagnostic(
                    request,
                    source,
                    Some(offset as u64),
                    "cem.css.invalid_token",
                    Severity::Error,
                    "CSS comment is missing a closing */".to_owned(),
                ));
            }
            continue;
        }

        if matches!(ch, '"' | '\'') {
            sanitized.push(' ');
            let quote = ch;
            let mut escaped = false;
            let mut closed = false;
            while let Some((_, string_ch)) = chars.next() {
                sanitized.push(if string_ch == '\n' { '\n' } else { ' ' });
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == quote {
                    closed = true;
                    break;
                } else if string_ch == '\n' {
                    break;
                }
            }
            if !closed && !state.reported_bad_string {
                state.reported_bad_string = true;
                diagnostics.push(css_diagnostic(
                    request,
                    source,
                    Some(offset as u64),
                    "cem.css.bad_string",
                    Severity::Warning,
                    "CSS string token was recovered before a matching quote".to_owned(),
                ));
            }
            continue;
        }

        sanitized.push(ch);
    }

    sanitized
}

fn css_validate_delimiters(
    request: &CssSourceValidationRequest<'_>,
    source: &str,
    sanitized: &str,
    state: &mut CssDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut stack = Vec::new();
    for (offset, ch) in sanitized.char_indices() {
        match ch {
            '{' | '[' | '(' => stack.push((ch, offset)),
            '}' | ']' | ')' => {
                let expected = match ch {
                    '}' => '{',
                    ']' => '[',
                    ')' => '(',
                    _ => unreachable!(),
                };
                match stack.pop() {
                    Some((open, _)) if open == expected => {}
                    _ if !state.reported_invalid_token => {
                        state.reported_invalid_token = true;
                        diagnostics.push(css_diagnostic(
                            request,
                            source,
                            Some(offset as u64),
                            "cem.css.invalid_token",
                            Severity::Error,
                            format!(
                                "CSS closing delimiter `{ch}` does not match an open delimiter"
                            ),
                        ));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    if let Some((open, offset)) = stack.first().copied() {
        if !state.reported_invalid_token {
            state.reported_invalid_token = true;
            diagnostics.push(css_diagnostic(
                request,
                source,
                Some(offset as u64),
                "cem.css.invalid_token",
                Severity::Error,
                format!("CSS delimiter `{open}` is not closed"),
            ));
        }
    }
}

fn css_validate_at_rules_and_urls(
    request: &CssSourceValidationRequest<'_>,
    source: &str,
    sanitized: &str,
    state: &mut CssDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let lower = sanitized.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes[offset] == b'@' {
            let name_start = offset + 1;
            let mut name_end = name_start;
            while name_end < bytes.len()
                && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'-')
            {
                name_end += 1;
            }
            let name = &lower[name_start..name_end];
            if name == "import" && !state.reported_import {
                state.reported_import = true;
                diagnostics.push(css_diagnostic(
                    request,
                    source,
                    Some(offset as u64),
                    "cem.css.import_rejected",
                    Severity::Error,
                    "CSS @import access requires an explicit resolver policy".to_owned(),
                ));
            } else if !name.is_empty()
                && !css_at_rule_is_known(name)
                && !state.reported_unknown_at_rule
            {
                state.reported_unknown_at_rule = true;
                diagnostics.push(css_diagnostic(
                    request,
                    source,
                    Some(offset as u64),
                    "cem.css.unknown_at_rule",
                    Severity::Info,
                    format!("CSS at-rule `@{name}` is preserved as an unknown at-rule"),
                ));
            }
            offset = name_end;
        } else {
            offset += 1;
        }
    }

    let mut search_start = 0usize;
    while let Some(relative_url_start) = lower[search_start..].find("url(") {
        let url_start = search_start + relative_url_start;
        match css_url_argument(source, url_start) {
            Some(reference) if css_url_requires_policy(&reference) && !state.reported_url => {
                state.reported_url = true;
                diagnostics.push(css_diagnostic(
                    request,
                    source,
                    Some(url_start as u64),
                    "cem.css.url_rejected",
                    Severity::Error,
                    "CSS url() reference requires an explicit resolver or sanitizer policy"
                        .to_owned(),
                ));
            }
            None if !state.reported_bad_url => {
                state.reported_bad_url = true;
                diagnostics.push(css_diagnostic(
                    request,
                    source,
                    Some(url_start as u64),
                    "cem.css.bad_url",
                    Severity::Warning,
                    "CSS url() token was recovered without a closing parenthesis".to_owned(),
                ));
            }
            _ => {}
        }
        search_start = url_start + 4;
    }
}

fn css_at_rule_is_known(name: &str) -> bool {
    matches!(
        name,
        "charset"
            | "container"
            | "font-face"
            | "font-feature-values"
            | "import"
            | "keyframes"
            | "layer"
            | "media"
            | "namespace"
            | "page"
            | "property"
            | "scope"
            | "supports"
    )
}

fn css_url_argument(source: &str, url_start: usize) -> Option<String> {
    let after_open = url_start + 4;
    let bytes = source.as_bytes();
    let mut offset = after_open;
    let mut quote = None;
    while offset < bytes.len() {
        let byte = bytes[offset];
        match (quote, byte) {
            (Some(q), b) if b == q => quote = None,
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b')') => {
                return Some(
                    source[after_open..offset]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_owned(),
                )
            }
            _ => {}
        }
        offset += 1;
    }
    None
}

fn css_url_requires_policy(reference: &str) -> bool {
    let trimmed = reference.trim();
    !(trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.to_ascii_lowercase().starts_with("data:"))
}

fn css_validate_rule_shapes(
    request: &CssSourceValidationRequest<'_>,
    source: &str,
    sanitized: &str,
    state: &mut CssDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut stack = Vec::new();
    let mut rule_start = 0usize;
    for (offset, ch) in sanitized.char_indices() {
        match ch {
            '{' => {
                if stack.is_empty() {
                    let prelude = sanitized[rule_start..offset].trim();
                    if prelude.is_empty() && !state.reported_invalid_selector {
                        state.reported_invalid_selector = true;
                        diagnostics.push(css_diagnostic(
                            request,
                            source,
                            Some(offset as u64),
                            "cem.css.invalid_selector",
                            Severity::Warning,
                            "CSS rule has an empty selector or prelude".to_owned(),
                        ));
                    }
                }
                stack.push(offset);
            }
            '}' => {
                if let Some(open_offset) = stack.pop() {
                    if stack.is_empty() {
                        css_validate_declaration_block(
                            request,
                            source,
                            sanitized,
                            open_offset + 1,
                            offset,
                            state,
                            diagnostics,
                        );
                        rule_start = offset + 1;
                    }
                }
            }
            ';' if stack.is_empty() => {
                rule_start = offset + 1;
            }
            _ => {}
        }
    }
}

fn css_validate_declaration_block(
    request: &CssSourceValidationRequest<'_>,
    source: &str,
    sanitized: &str,
    start: usize,
    end: usize,
    state: &mut CssDocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if state.reported_invalid_declaration {
        return;
    }
    let mut declaration_start = start;
    for relative_semicolon in sanitized[start..end]
        .match_indices(';')
        .map(|(index, _)| index)
    {
        let declaration_end = start + relative_semicolon;
        let declaration = sanitized[declaration_start..declaration_end].trim();
        if css_declaration_is_malformed(declaration) {
            state.reported_invalid_declaration = true;
            diagnostics.push(css_diagnostic(
                request,
                source,
                Some(declaration_start as u64),
                "cem.css.invalid_declaration",
                Severity::Warning,
                "CSS declaration was recovered without a property/value colon".to_owned(),
            ));
            return;
        }
        declaration_start = declaration_end + 1;
    }
    let declaration = sanitized[declaration_start..end].trim();
    if css_declaration_is_malformed(declaration) {
        state.reported_invalid_declaration = true;
        diagnostics.push(css_diagnostic(
            request,
            source,
            Some(declaration_start as u64),
            "cem.css.invalid_declaration",
            Severity::Warning,
            "CSS declaration was recovered without a property/value colon".to_owned(),
        ));
    }
}

fn css_declaration_is_malformed(declaration: &str) -> bool {
    !declaration.is_empty()
        && !declaration.contains(':')
        && !declaration.contains('{')
        && !declaration.contains('}')
        && !declaration.trim_start().starts_with('@')
}

fn css_diagnostic(
    request: &CssSourceValidationRequest<'_>,
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
        validate_css_source_bytes(CssSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: "fixture.css",
            content_type: Some("text/css"),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn css_source_validator_accepts_basic_stylesheet() {
        let diagnostics = validate(
            r#"@charset "utf-8";
:root { --space-2: 0.5rem; }
.card { padding: var(--space-2); color: currentColor; }
"#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn css_source_validator_accepts_style_attribute_fragment() {
        let diagnostics = validate("color: currentColor; margin-inline: 0; --card-gap: 0.75rem;");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn css_source_validator_reports_import_rejected() {
        let diagnostics =
            validate("@import \"shared/theme.css\";\n.card { color: currentColor; }\n");

        assert!(has_code(&diagnostics, "cem.css.import_rejected"));
    }

    #[test]
    fn css_source_validator_reports_url_rejected() {
        let diagnostics = validate(".hero { background-image: url(\"images/hero.png\"); }");

        assert!(has_code(&diagnostics, "cem.css.url_rejected"));
    }

    #[test]
    fn css_source_validator_reports_invalid_token() {
        let diagnostics = validate(".card { color: red;");

        assert!(has_code(&diagnostics, "cem.css.invalid_token"));
    }

    #[test]
    fn css_source_validator_reports_invalid_declaration() {
        let diagnostics = validate(".card { color currentColor; padding: 1rem; }");

        assert!(has_code(&diagnostics, "cem.css.invalid_declaration"));
    }

    #[test]
    fn css_source_validator_reports_encoding_conflict() {
        let diagnostics = validate_css_source_bytes(CssSourceValidationRequest {
            bytes: br#"@charset "utf-8";
.card { color: currentColor; }
"#,
            source_uri: "fixture.css",
            content_type: Some("text/css; charset=iso-8859-1"),
        });

        assert!(has_code(&diagnostics, "cem.css.encoding_conflict"));
    }
}
