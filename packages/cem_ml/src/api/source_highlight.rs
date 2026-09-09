use serde::{Deserialize, Serialize};
use std::ops::Range;

use crate::diagnostics::{project_diagnostics_for_source, Diagnostic, Severity};
use crate::schema::registry::content_type_essence;
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};
use crate::validation::css::{
    css_document_ast_from_source_bytes, css_event_semantic_role, CssSourceValidationRequest,
};
use crate::validation::html::{
    html_document_ast_from_source_bytes, html_event_markup_tokens, HtmlDocumentAst, HtmlEventAst,
    HtmlEventKind, HtmlNamespace, HtmlSourceValidationRequest,
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

const MAX_CONTENT_SCOPE_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceScopeKind {
    Html,
    CemMl,
    Css,
}

#[derive(Debug, Default)]
struct ScopeHighlight {
    diagnostics: Vec<Diagnostic>,
    spans: Vec<SourceHighlightSpanV1>,
    content_scopes: Vec<ActiveContentTypeScopeAst>,
}

/// One lossless source token after all parent-owned content-type handoffs have
/// been applied.
///
/// This is the shared formatter/colorizer input used by host source views and
/// lifecycle formatters. `formatter` and `colorizer` identify the language
/// adapter which assigned `role`; the enclosing document does not reinterpret
/// a child language's tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSyntaxTokenAst {
    pub byte_offset: usize,
    pub byte_length: usize,
    pub line: u32,
    pub column: u32,
    pub text: String,
    pub kind: &'static str,
    pub role: &'static str,
    pub content_type: String,
    pub formatter: &'static str,
    pub colorizer: &'static str,
    pub scope_depth: usize,
}

impl SourceSyntaxTokenAst {
    pub fn end(&self) -> usize {
        self.byte_offset.saturating_add(self.byte_length)
    }
}

/// A flat AST stream whose tokens retain the active nested language scope.
#[derive(Debug, Default)]
pub struct SourceSyntaxAstStream {
    pub diagnostics: Vec<Diagnostic>,
    pub tokens: Vec<SourceSyntaxTokenAst>,
}

/// A child content-type region selected by its parent AST.
///
/// The range contains only bytes owned by the child parser. Parent syntax such
/// as an HTML close tag or CEM-ML rich-content fence remains outside it, so the
/// parent formatter resumes exactly at the declared return boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ContentTypeScopeAst {
    content_type: String,
    body_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveContentTypeScopeAst {
    kind: SourceScopeKind,
    content_type: String,
    body_range: Range<usize>,
    depth: usize,
}

/// Highlight authored source without changing its bytes.
///
/// Each AST-owned content-type scope selects its own formatter/colorizer roles.
/// HTML roles come from the same lossless lexical pieces exposed to
/// `html.format-document.tree`; CEM-ML structural roles come from the canonical
/// tokenizer, and nested scopes are handed to their declared language parser.
/// Punctuation inside structural tokens is split only for presentation.
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

    let highlighted = source_syntax_ast_stream(
        &request.source,
        &request.content_type,
        request.source_url.as_deref(),
    );
    let (status, mut diagnostics, spans) = match highlighted {
        Some(stream) => (
            "highlighted",
            stream.diagnostics,
            stream
                .tokens
                .into_iter()
                .map(|token| SourceHighlightSpanV1 {
                    byte_offset: token.byte_offset,
                    byte_length: token.byte_length,
                    role: token.role,
                })
                .collect(),
        ),
        None => ("unsupported", Vec::new(), Vec::new()),
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

/// Parse source into a generic, lossless token stream and recursively dispatch
/// every AST-owned embedded body to the formatter/colorizer registered for its
/// active content type.
pub fn source_syntax_ast_stream(
    source: &str,
    content_type: &str,
    source_url: Option<&str>,
) -> Option<SourceSyntaxAstStream> {
    let highlighted = highlight_source_scope(source, content_type, source_url, 0)?;
    let spans = normalize_spans(highlighted.spans);
    let line_index = crate::source::line_index::LineIndex::from_utf8(source);
    let mut tokens = Vec::with_capacity(spans.len());
    let mut cursor = 0usize;
    for span in spans {
        if span.byte_offset > cursor {
            push_syntax_token(
                source,
                &line_index,
                &highlighted.content_scopes,
                &mut tokens,
                cursor,
                span.byte_offset - cursor,
                "syntax.raw",
            );
        }
        push_syntax_token(
            source,
            &line_index,
            &highlighted.content_scopes,
            &mut tokens,
            span.byte_offset,
            span.byte_length,
            span.role,
        );
        cursor = span.end();
    }
    if cursor < source.len() {
        push_syntax_token(
            source,
            &line_index,
            &highlighted.content_scopes,
            &mut tokens,
            cursor,
            source.len() - cursor,
            "syntax.raw",
        );
    }
    Some(SourceSyntaxAstStream {
        diagnostics: highlighted.diagnostics,
        tokens,
    })
}

fn push_syntax_token(
    source: &str,
    line_index: &crate::source::line_index::LineIndex,
    scopes: &[ActiveContentTypeScopeAst],
    tokens: &mut Vec<SourceSyntaxTokenAst>,
    byte_offset: usize,
    byte_length: usize,
    role: &'static str,
) {
    if byte_length == 0 {
        return;
    }
    let end = byte_offset.saturating_add(byte_length);
    let Some(scope) = scopes
        .iter()
        .filter(|scope| byte_offset >= scope.body_range.start && end <= scope.body_range.end)
        .max_by_key(|scope| scope.depth)
    else {
        return;
    };
    if end > source.len() || !source.is_char_boundary(byte_offset) || !source.is_char_boundary(end)
    {
        return;
    }
    let position = line_index.project(byte_offset as u64);
    tokens.push(SourceSyntaxTokenAst {
        byte_offset,
        byte_length,
        line: position.line,
        column: position.column,
        text: source[byte_offset..end].to_owned(),
        kind: syntax_token_kind(role),
        role,
        content_type: scope.content_type.clone(),
        formatter: scope.kind.formatter_name(),
        colorizer: scope.kind.colorizer_name(),
        scope_depth: scope.depth,
    });
}

fn syntax_token_kind(role: &str) -> &'static str {
    match role {
        "syntax.punctuation" => "punctuation",
        "syntax.name" => "name",
        "syntax.attribute" => "attribute",
        "syntax.property" => "property",
        "syntax.value" => "value",
        "syntax.function" => "function",
        "syntax.keyword" => "keyword",
        "syntax.string" => "string",
        "syntax.number" => "number",
        "syntax.comment" => "comment",
        "syntax.text" => "text",
        "diagnostic.error" => "error",
        _ => "raw",
    }
}

fn highlight_source_scope(
    source: &str,
    content_type: &str,
    source_url: Option<&str>,
    depth: usize,
) -> Option<ScopeHighlight> {
    if depth >= MAX_CONTENT_SCOPE_DEPTH {
        return None;
    }
    let kind = source_scope_kind(content_type)?;
    let mut highlighted = match kind {
        SourceScopeKind::Html => highlight_html_source(source, source_url, depth),
        SourceScopeKind::CemMl => highlight_cem_ml_source(source, source_url, depth),
        SourceScopeKind::Css => highlight_css_source(source, content_type, source_url),
    };
    highlighted.content_scopes.push(ActiveContentTypeScopeAst {
        kind,
        content_type: content_type.to_owned(),
        body_range: 0..source.len(),
        depth,
    });
    Some(highlighted)
}

fn source_scope_kind(content_type: &str) -> Option<SourceScopeKind> {
    match content_type_essence(content_type).as_str() {
        "html" | "text/html" => Some(SourceScopeKind::Html),
        "cem" | "cemml" | "cem-ml" | "application/cem" | "text/cem-ml" | "text/cem" => {
            Some(SourceScopeKind::CemMl)
        }
        "css" | "text/css" => Some(SourceScopeKind::Css),
        _ => None,
    }
}

impl SourceScopeKind {
    fn formatter_name(self) -> &'static str {
        match self {
            Self::Html => "html.format-document",
            Self::CemMl => "cem.format-tree",
            Self::Css => "css.format-document",
        }
    }

    fn colorizer_name(self) -> &'static str {
        match self {
            Self::Html => "html.color-document",
            Self::CemMl => "cem.color-tree",
            Self::Css => "css.color-document",
        }
    }
}

fn highlight_html_source(source: &str, source_url: Option<&str>, depth: usize) -> ScopeHighlight {
    let (document, diagnostics) =
        html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri: source_url.unwrap_or("memory:cem-source-highlight.html"),
            content_type: Some("text/html"),
        });
    let Some(document) = document else {
        return ScopeHighlight {
            diagnostics,
            spans: Vec::new(),
            content_scopes: Vec::new(),
        };
    };
    let child_scopes = html_content_type_scopes(&document);
    let mut spans = Vec::new();
    for event in &document.events {
        let event_range = html_event_range(event);
        if child_scopes
            .iter()
            .any(|scope| range_contains(&scope.body_range, &event_range))
        {
            continue;
        }
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

    let mut highlighted = ScopeHighlight {
        diagnostics,
        spans,
        content_scopes: Vec::new(),
    };
    for scope in child_scopes {
        highlight_child_scope(source, &scope, source_url, depth, &mut highlighted);
    }
    highlighted
}

fn highlight_cem_ml_source(source: &str, source_url: Option<&str>, depth: usize) -> ScopeHighlight {
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    let diagnostics = tokenizer.take_diagnostics();
    let child_scopes = cem_content_type_scopes(&tokens);
    let mut spans = Vec::new();
    for token in &tokens {
        if let Some(scope) = child_scopes.iter().find(|scope| {
            let token_range = schema_token_range(token);
            scope.body_range.start >= token_range.start && scope.body_range.end <= token_range.end
        }) {
            highlight_cem_rich_content_boundary(source, token, scope, &mut spans);
            continue;
        }
        highlight_cem_token(source, token, &mut spans);
    }

    let mut highlighted = ScopeHighlight {
        diagnostics,
        spans,
        content_scopes: Vec::new(),
    };
    for scope in child_scopes {
        highlight_child_scope(source, &scope, source_url, depth, &mut highlighted);
    }
    highlighted
}

fn highlight_css_source(
    source: &str,
    content_type: &str,
    source_url: Option<&str>,
) -> ScopeHighlight {
    let (document, diagnostics) = css_document_ast_from_source_bytes(CssSourceValidationRequest {
        bytes: source.as_bytes(),
        source_uri: source_url.unwrap_or("memory:cem-source-highlight.css"),
        content_type: Some(content_type),
    });
    let mut spans = Vec::new();
    if let Some(document) = document {
        for event in &document.events {
            push_span(
                source,
                &mut spans,
                event.source_range.start.byte_offset as usize,
                event.source_range.byte_length as usize,
                css_event_semantic_role(event),
            );
        }
    }
    ScopeHighlight {
        diagnostics,
        spans,
        content_scopes: Vec::new(),
    }
}

fn html_content_type_scopes(document: &HtmlDocumentAst) -> Vec<ContentTypeScopeAst> {
    let mut scopes = Vec::new();
    for (index, event) in document.events.iter().enumerate() {
        let Some(content_type) = html_child_content_type(event) else {
            continue;
        };
        if source_scope_kind(&content_type).is_none() {
            continue;
        }
        let Some(local_name) = event.local_name.as_deref() else {
            continue;
        };
        let Some(close) = document.events[index + 1..].iter().find(|candidate| {
            candidate.kind == HtmlEventKind::EndElement
                && candidate.depth == event.depth
                && candidate.namespace == event.namespace
                && candidate.local_name.as_deref() == Some(local_name)
        }) else {
            continue;
        };
        let body_start = html_event_range(event).end;
        let body_end = html_event_range(close).start;
        if body_start <= body_end {
            scopes.push(ContentTypeScopeAst {
                content_type,
                body_range: body_start..body_end,
            });
        }
    }
    retain_outermost_scopes(scopes)
}

fn html_child_content_type(event: &HtmlEventAst) -> Option<String> {
    if event.kind != HtmlEventKind::StartElement
        || event.namespace != HtmlNamespace::Html
        || event.self_closing
        || event.void_element
    {
        return None;
    }
    match event.local_name.as_deref()? {
        "template" => event
            .attributes
            .iter()
            .find(|attribute| attribute.local_name == "type" && !attribute.duplicate)
            .and_then(|attribute| attribute.value.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        "style" => Some(
            event
                .attributes
                .iter()
                .find(|attribute| attribute.local_name == "type" && !attribute.duplicate)
                .and_then(|attribute| attribute.value.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("text/css")
                .to_owned(),
        ),
        _ => None,
    }
}

#[derive(Debug)]
struct CemContentFrameAst {
    node_name: Option<String>,
    child_content_type: Option<String>,
}

fn cem_content_type_scopes(tokens: &[SchemaToken]) -> Vec<ContentTypeScopeAst> {
    let mut frames: Vec<CemContentFrameAst> = Vec::new();
    let mut scopes = Vec::new();
    for token in tokens {
        match &token.kind {
            SchemaTokenKind::NodeStart { name } => frames.push(CemContentFrameAst {
                node_name: Some(name.clone()),
                child_content_type: cem_node_child_content_type(name).map(str::to_owned),
            }),
            SchemaTokenKind::AnonymousScopeStart => frames.push(CemContentFrameAst {
                node_name: None,
                child_content_type: None,
            }),
            SchemaTokenKind::Attribute { name, value, .. }
                if name == "type"
                    && frames.last().is_some_and(|frame| frame.node_name.is_none()) =>
            {
                if let (Some(frame), Some(value)) = (frames.last_mut(), value.as_deref()) {
                    frame.child_content_type = Some(value.trim().to_owned());
                }
            }
            SchemaTokenKind::RichContent { data } => {
                let Some(content_type) = frames
                    .last()
                    .and_then(|frame| frame.child_content_type.as_deref())
                else {
                    continue;
                };
                if source_scope_kind(content_type).is_none() {
                    continue;
                }
                let token_start = token.byte_range.start as usize;
                let token_end = token.byte_range.end() as usize;
                let body_start = token_start.saturating_add(3).min(token_end);
                let body_end = body_start.saturating_add(data.len()).min(token_end);
                scopes.push(ContentTypeScopeAst {
                    content_type: content_type.to_owned(),
                    body_range: body_start..body_end,
                });
            }
            SchemaTokenKind::NodeEnd { .. } => {
                frames.pop();
            }
            _ => {}
        }
    }
    scopes
}

fn cem_node_child_content_type(node_name: &str) -> Option<&'static str> {
    (node_name.rsplit(':').next() == Some("style")).then_some("text/css; mode=scoped-style-block")
}

fn highlight_cem_rich_content_boundary(
    source: &str,
    token: &SchemaToken,
    scope: &ContentTypeScopeAst,
    spans: &mut Vec<SourceHighlightSpanV1>,
) {
    let token_range = schema_token_range(token);
    push_span(
        source,
        spans,
        token_range.start,
        scope.body_range.start.saturating_sub(token_range.start),
        "syntax.punctuation",
    );
    push_span(
        source,
        spans,
        scope.body_range.end,
        token_range.end.saturating_sub(scope.body_range.end),
        "syntax.punctuation",
    );
}

fn highlight_child_scope(
    parent_source: &str,
    scope: &ContentTypeScopeAst,
    source_url: Option<&str>,
    parent_depth: usize,
    parent: &mut ScopeHighlight,
) {
    let Some(child_source) = parent_source.get(scope.body_range.clone()) else {
        return;
    };
    let Some(mut child) = highlight_source_scope(
        child_source,
        &scope.content_type,
        source_url,
        parent_depth + 1,
    ) else {
        push_span(
            parent_source,
            &mut parent.spans,
            scope.body_range.start,
            scope.body_range.len(),
            "syntax.string",
        );
        return;
    };
    rebase_spans(&mut child.spans, scope.body_range.start);
    rebase_content_scopes(&mut child.content_scopes, scope.body_range.start);
    rebase_diagnostics(&mut child.diagnostics, scope.body_range.start);
    parent.spans.extend(child.spans);
    parent.content_scopes.extend(child.content_scopes);
    parent.diagnostics.extend(child.diagnostics);
}

fn rebase_spans(spans: &mut [SourceHighlightSpanV1], byte_offset: usize) {
    for span in spans {
        span.byte_offset = span.byte_offset.saturating_add(byte_offset);
    }
}

fn rebase_content_scopes(scopes: &mut [ActiveContentTypeScopeAst], byte_offset: usize) {
    for scope in scopes {
        scope.body_range.start = scope.body_range.start.saturating_add(byte_offset);
        scope.body_range.end = scope.body_range.end.saturating_add(byte_offset);
    }
}

fn rebase_diagnostics(diagnostics: &mut [Diagnostic], byte_offset: usize) {
    let byte_offset = byte_offset as u64;
    for diagnostic in diagnostics {
        diagnostic.byte_offset = diagnostic
            .byte_offset
            .map(|offset| offset.saturating_add(byte_offset));
        diagnostic.line = None;
        diagnostic.column = None;
        diagnostic.uri = None;
        if let Some(source_map) = diagnostic.source_map.as_mut() {
            for frame in &mut source_map.frames {
                match &mut frame.span {
                    crate::source_map::FrameSpan::Single(range) => {
                        range.start = range.start.saturating_add(byte_offset);
                    }
                    crate::source_map::FrameSpan::Multi(ranges) => {
                        for range in ranges {
                            range.start = range.start.saturating_add(byte_offset);
                        }
                    }
                }
            }
        }
    }
}

fn retain_outermost_scopes(mut scopes: Vec<ContentTypeScopeAst>) -> Vec<ContentTypeScopeAst> {
    scopes.sort_by_key(|scope| (scope.body_range.start, usize::MAX - scope.body_range.end));
    let mut outermost: Vec<ContentTypeScopeAst> = Vec::new();
    for scope in scopes {
        if outermost
            .last()
            .is_some_and(|parent| range_contains(&parent.body_range, &scope.body_range))
        {
            continue;
        }
        outermost.push(scope);
    }
    outermost
}

fn html_event_range(event: &HtmlEventAst) -> Range<usize> {
    let start = event.source_range.start.byte_offset as usize;
    start..start.saturating_add(event.source_range.byte_length as usize)
}

fn schema_token_range(token: &SchemaToken) -> Range<usize> {
    token.byte_range.start as usize..token.byte_range.end() as usize
}

fn range_contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    inner.start >= outer.start && inner.end <= outer.end
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
        "syntax.property" => Some("dfn"),
        "syntax.value" => Some("data"),
        "syntax.function" => Some("kbd"),
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
    fn html_template_content_type_drives_nested_cem_ml_and_css_scope_coloring() {
        let source = concat!(
            "<cem-element><template>{section @class=plain | Plain}</template>",
            "<template type=\"text/cem-ml\">{style |```\n",
            ".card { color: red; width: 12px; }\n",
            "```}{section @class=card | Scoped}</template></cem-element>",
            "<p id=after>Done</p>"
        );
        let value = response(
            &serde_json::json!({ "source": source, "contentType": "text/html" }).to_string(),
        );

        assert_eq!(value["status"], "highlighted");
        assert_eq!(
            role_at(&value, source.find("{section @class=plain").unwrap() + 1),
            Some("syntax.text"),
            "an untyped HTML template remains in the surrounding HTML scope"
        );
        assert_eq!(
            role_at(&value, source.find("{style").unwrap() + 1),
            Some("syntax.name"),
            "the typed template body is formatted as CEM-ML"
        );
        assert_eq!(
            role_at(&value, source.find("color:").unwrap()),
            Some("syntax.property"),
            "the style rich-content body is formatted as scoped CSS"
        );
        assert_eq!(
            role_at(&value, source.find("12px").unwrap()),
            Some("syntax.number"),
            "CSS dimensions keep the CSS colorizer's number role"
        );
        assert_eq!(
            role_at(&value, source.rfind("section @class=card").unwrap()),
            Some("syntax.name"),
            "the parent CEM-ML scope resumes after CSS"
        );
        assert_eq!(
            role_at(&value, source.rfind("p id=after").unwrap()),
            Some("syntax.name"),
            "the parent HTML scope resumes after the typed template"
        );
        assert_eq!(semantic_html_text(value["html"].as_str().unwrap()), source);
    }

    #[test]
    fn css_source_roles_distinguish_properties_values_functions_and_custom_properties() {
        let source = ".card { --accent: #312e81; color: var(--accent); display: grid; }";
        let value = response(
            &serde_json::json!({ "source": source, "contentType": "text/css" }).to_string(),
        );

        assert_eq!(value["status"], "highlighted");
        assert_eq!(
            role_at(&value, source.find("card").unwrap()),
            Some("syntax.name")
        );
        assert_eq!(
            role_at(&value, source.find("--accent").unwrap()),
            Some("syntax.attribute")
        );
        assert_eq!(
            role_at(&value, source.find("#312e81").unwrap()),
            Some("syntax.value")
        );
        assert_eq!(
            role_at(&value, source.find("color:").unwrap()),
            Some("syntax.property")
        );
        assert_eq!(
            role_at(&value, source.find("var(").unwrap()),
            Some("syntax.function")
        );
        assert_eq!(
            role_at(&value, source.rfind("--accent").unwrap()),
            Some("syntax.attribute")
        );
        assert_eq!(
            role_at(&value, source.find("grid").unwrap()),
            Some("syntax.value")
        );

        let html = value["html"].as_str().unwrap();
        assert!(html.contains("<var>--accent</var>"));
        assert!(html.contains("<data>#312e81</data>"));
        assert!(html.contains("<dfn>color</dfn>"));
        assert!(html.contains("<kbd>var(</kbd>"));
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
        for tag in [
            "b", "var", "dfn", "data", "kbd", "strong", "i", "u", "small", "samp", "mark",
        ] {
            text = text.replace(&format!("<{tag}>"), "");
            text = text.replace(&format!("</{tag}>"), "");
        }
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&")
    }

    fn role_at(value: &serde_json::Value, byte_offset: usize) -> Option<&str> {
        value["spans"].as_array()?.iter().find_map(|span| {
            let start = span["byteOffset"].as_u64()? as usize;
            let end = start.saturating_add(span["byteLength"].as_u64()? as usize);
            (start..end)
                .contains(&byte_offset)
                .then(|| span["role"].as_str())
                .flatten()
        })
    }
}
