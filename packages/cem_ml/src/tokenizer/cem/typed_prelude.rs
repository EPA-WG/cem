//! Explicit 1.1 lexical admission. No query execution or CEM reparse.
use super::*;

/// Bounded host admission for fragments. Kept under its original preview API name
/// for compatibility; normal documents use their leading @doc constraint.
#[derive(Debug, Clone, Copy)]
pub struct TypedPreludePreview {
    /// Maximum constructor bytes, capped at 64 KiB.
    pub max_value_bytes: usize,
    /// Combined brace/comment depth, capped at 128.
    pub max_nesting: u32,
}
impl Default for TypedPreludePreview {
    fn default() -> Self {
        Self {
            max_value_bytes: 64 * 1024,
            max_nesting: 128,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypedPreludeRole {
    SchemaSelector,
    Namespace,
    DefaultNamespace,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypedPreludeKind {
    Reference,
    Expression,
}

/// Structural scanner output, including invalid native attempts. Spans are
/// absolute source bytes; an invalid slot must never enter a literal decoder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedPreludeValue {
    pub role: TypedPreludeRole,
    pub prefix: Option<String>,
    pub role_range: ByteRange,
    pub prefix_range: Option<ByteRange>,
    pub value_range: ByteRange,
    pub payload_range: ByteRange,
    pub kind: TypedPreludeKind,
    pub expression: String,
    pub error: Option<String>,
}

impl CemTokenizer {
    /// An explicit host profile admits fragments. An authored @doc constraint
    /// overrides it; broad/older constraints preserve the 1.0 lexical branch.
    pub fn from_source_with_typed_prelude_preview<S: ByteSource>(
        source: S,
        limits: TypedPreludePreview,
    ) -> Self {
        Self::from_source_inner(source, None, Some(limits))
    }

    pub fn from_source_with_control_and_typed_prelude_preview<S: ByteSource>(
        source: S,
        control: crate::operation_control::OperationControl,
        scope: crate::operation_control::ExecutionScopeId,
        limits: TypedPreludePreview,
    ) -> Self {
        Self::from_source_inner(
            source,
            Some(TokenizerControl::new(control, scope)),
            Some(limits),
        )
    }

    /// Explicit host profile for fragments without an authored document header.
    pub fn from_source_with_format_profile<S: ByteSource>(
        source: S,
        profile: crate::schema::ir::SemVer,
    ) -> Result<Self, String> {
        if profile.major != 1
            || profile.minor > 1
            || profile.patch != 0
            || profile.prerelease.is_some()
        {
            return Err("unsupported CEM-ML fragment profile".into());
        }
        Ok(Self::from_source_inner(
            source,
            None,
            (profile.minor == 1).then(TypedPreludePreview::default),
        ))
    }

    pub(super) fn admit_prelude_doc(&mut self, body: &str) {
        self.typed_preludes_enabled = crate::parser::format::requires_typed_prelude_preview(body)
            && crate::parser::format::resolve_doc_directive(body).is_ok();
        if self.typed_preludes_enabled && self.prelude_preview.is_none() {
            self.prelude_preview = Some(TypedPreludePreview::default());
        }
    }

    /// Look ahead in the original scalar stream to distinguish quoted literals
    /// from native attempts. Never parse a Directive.data string downstream.
    pub(super) fn scan_typed_directive(
        &mut self,
        start: usize,
        name: &str,
        in_block: bool,
    ) -> bool {
        if !self.typed_preludes_enabled {
            return false;
        }
        let body_start = self.cursor;
        let mut quote = None;
        let mut native = None;
        while let Some(c) = self.peek() {
            if matches!(c, '\n' | '\r') {
                break;
            }
            if let Some(delimiter) = quote {
                if c == '\\' {
                    self.cursor += 1;
                    if self.peek().is_some_and(|c| !matches!(c, '\n' | '\r')) {
                        self.cursor += 1;
                    }
                    continue;
                }
                if c == delimiter {
                    quote = None;
                }
            } else if matches!(c, '\'' | '"') {
                quote = Some(c);
            } else if c == '{' {
                native = Some(self.cursor);
                break;
            }
            self.cursor += 1;
        }
        let Some(value_start) = native else {
            self.cursor = body_start;
            return false;
        };
        let role = match name {
            "schema" => TypedPreludeRole::SchemaSelector,
            "ns" => TypedPreludeRole::Namespace,
            "default" => TypedPreludeRole::DefaultNamespace,
            _ => {
                self.emit(
                    SchemaTokenKind::Error {
                        code: "cem.prelude.unsupported_site".into(),
                    },
                    self.range_from(start, value_start + 1),
                );
                self.cursor = body_start;
                return false;
            }
        };
        let limits = self.prelude_preview.unwrap();
        let mut header_start = body_start;
        while header_start < value_start && matches!(self.scalars[header_start].0, ' ' | '\t') {
            header_start += 1;
        }
        let mut header_end = value_start;
        while header_end > header_start && matches!(self.scalars[header_end - 1].0, ' ' | '\t') {
            header_end -= 1;
        }
        // Do not allocate an unbounded malformed header.
        let header: String = self.scalars[header_start..header_end]
            .iter()
            .take(64 * 1024)
            .map(|(c, _)| *c)
            .collect();
        let mut error: Option<String> = None;
        let mut prefix = None;
        let mut prefix_range = None;
        let role_range;
        let valid_header = match role {
            TypedPreludeRole::SchemaSelector => {
                role_range = self.range_from(header_start, (header_start + 6).min(header_end));
                header
                    .strip_suffix('=')
                    .is_some_and(|s| s.trim_end() == "select")
            }
            TypedPreludeRole::Namespace => {
                let end = (header_start..header_end)
                    .find(|i| !is_name_continue(self.scalars[*i].0))
                    .unwrap_or(header_end);
                let name: String = self.scalars[header_start..end]
                    .iter()
                    .map(|(c, _)| *c)
                    .collect();
                role_range = self.range_from(start, start + 3);
                prefix_range = Some(self.range_from(header_start, end));
                prefix = Some(name.clone());
                !name.is_empty()
                    && name.chars().next().is_some_and(is_name_start)
                    && header.strip_prefix(&name).is_some_and(|s| s.trim() == "=")
            }
            TypedPreludeRole::DefaultNamespace => {
                role_range = self.range_from(start, start + 8);
                prefix = Some(String::new());
                header.is_empty()
            }
        };
        if !valid_header
            || !matches!(self.scalars[body_start].0, ' ' | '\t')
            || self.range_from(body_start, value_start).len as usize > 64 * 1024
        {
            error = Some("cem.prelude.invalid_header".into());
        }
        self.cursor = value_start + 1;
        let kind = if self.peek() == Some('#') {
            TypedPreludeKind::Reference
        } else {
            if self.peek() != Some('$') {
                error = Some("cem.prelude.constructor_required".into());
            } else {
                self.cursor += 1;
                self.skip_horiz_ws();
                if self.peek() == Some('|') {
                    self.cursor += 1;
                }
            }
            TypedPreludeKind::Expression
        };
        let payload_start = self.cursor;
        if let Err(code) = self.scan_query_brace_body_bounded(Some((value_start, limits))) {
            error = Some(code.into());
        }
        let payload_end = self.cursor;
        if self.peek() == Some('}') && error.as_deref() != Some("cem.prelude.limit") {
            self.cursor += 1;
        } else if error.is_none() {
            error = Some("cem.prelude.unterminated".into());
        }
        let value_end = self.cursor;
        while let Some(c) = self.peek() {
            if matches!(c, '\n' | '\r') {
                break;
            }
            if !matches!(c, ' ' | '\t') && error.is_none() {
                error = Some("cem.prelude.trailing_value".into());
            }
            if in_block && c == '}' {
                break;
            }
            self.cursor += 1;
        }
        let mut pstart = payload_start;
        let mut pend = payload_end;
        while pstart < pend && self.scalars[pstart].0.is_whitespace() {
            pstart += 1;
        }
        while pend > pstart && self.scalars[pend - 1].0.is_whitespace() {
            pend -= 1;
        }
        let expression: String = self.scalars[pstart..pend].iter().map(|(c, _)| *c).collect();
        if error.is_none()
            && (expression.is_empty()
                || (kind == TypedPreludeKind::Reference
                    && expression.trim_start_matches('#').trim().is_empty()))
        {
            error = Some("cem.prelude.empty_value".into());
        }
        if let Some(code) = &error {
            self.parser_fact(
                CemMlParserFactKind::TokenizerInvalidTypedPrelude,
                code.clone(),
                self.range_from(value_start, value_end),
            );
        }
        self.emit(
            SchemaTokenKind::TypedDirective {
                name: name.into(),
                value: TypedPreludeValue {
                    role,
                    prefix,
                    role_range,
                    prefix_range,
                    value_range: self.range_from(value_start, value_end),
                    payload_range: self.range_from(pstart, pend),
                    kind,
                    expression,
                    error,
                },
            },
            self.range_from(start, self.cursor),
        );
        true
    }
}
