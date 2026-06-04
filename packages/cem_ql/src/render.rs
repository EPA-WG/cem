//! Data-bound CEM-ML template rendering.
//!
//! This is the first C2 slice: canonical CEM-ML is tokenized by `cem_ml`,
//! embedded CEM-QL expressions are evaluated by this crate, and the result is
//! emitted as light-DOM markup. The browser/WASM boundary will wrap this with a
//! compiled artifact and serializable render plan in later slices.

use std::collections::BTreeMap;

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::scheduler::ScopePolicy;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};

use crate::api::{compile, evaluate, CompileContext, EvaluationContext};
use crate::eval::{AtomValue, Item, ItemStream, QueryContextScope};

#[derive(Debug, Clone, Default)]
pub struct TemplateData {
    pub bindings: BTreeMap<String, ItemStream>,
}

impl TemplateData {
    pub fn with_binding(mut self, name: impl Into<String>, value: ItemStream) -> Self {
        self.bindings.insert(name.into(), value);
        self
    }
}

#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    pub rendered: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn render_template(source: &str, data: &TemplateData) -> RenderedTemplate {
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }

    let mut renderer = Renderer {
        tokens: &tokens,
        index: 0,
        compile_context: CompileContext {
            policy_bindings: data.bindings.clone(),
            ..CompileContext::default()
        },
        evaluation_context: EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
            policy_bindings: data.bindings.clone(),
        },
        rendered: String::new(),
        diagnostics: tokenizer.take_diagnostics(),
    };
    renderer.render_all();
    RenderedTemplate {
        rendered: renderer.rendered,
        diagnostics: renderer.diagnostics,
    }
}

struct Renderer<'a> {
    tokens: &'a [SchemaToken],
    index: usize,
    compile_context: CompileContext,
    evaluation_context: EvaluationContext,
    rendered: String,
    diagnostics: Vec<Diagnostic>,
}

impl Renderer<'_> {
    fn render_all(&mut self) {
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeStart { name } if name == "$" => self.render_expression_node(),
                SchemaTokenKind::NodeStart { .. } => self.render_element(),
                SchemaTokenKind::Text(text) | SchemaTokenKind::Trivia(text) => {
                    escape_text_into(&mut self.rendered, text);
                    self.index += 1;
                }
                SchemaTokenKind::ExpressionNode(source) => {
                    let rendered = self.evaluate_to_string(source, self.tokens[self.index].clone());
                    escape_text_into(&mut self.rendered, &rendered);
                    self.index += 1;
                }
                SchemaTokenKind::Comment(text) => {
                    self.rendered.push_str("<!--");
                    self.rendered.push_str(text);
                    self.rendered.push_str("-->");
                    self.index += 1;
                }
                SchemaTokenKind::NodeEnd { .. } => {
                    self.index += 1;
                }
                SchemaTokenKind::Attribute { .. }
                | SchemaTokenKind::AnonymousScopeStart
                | SchemaTokenKind::Directive { .. }
                | SchemaTokenKind::RichContent { .. }
                | SchemaTokenKind::ProcessingInstruction { .. }
                | SchemaTokenKind::Error { .. } => {
                    self.index += 1;
                }
            }
        }
    }

    fn render_element(&mut self) {
        let start = self.tokens[self.index].clone();
        let SchemaTokenKind::NodeStart { name } = &start.kind else {
            return;
        };
        let name = name.clone();
        self.index += 1;

        self.rendered.push('<');
        self.rendered.push_str(&name);

        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let token = self.tokens[self.index].clone();
                    if let Some(value) = value {
                        if let Some(value) = self.evaluate_attribute_value(name, value, token) {
                            self.rendered.push(' ');
                            self.rendered.push_str(name);
                            self.rendered.push_str("=\"");
                            escape_attr_into(&mut self.rendered, &value);
                            self.rendered.push('"');
                        }
                    } else {
                        self.rendered.push(' ');
                        self.rendered.push_str(name);
                    }
                    self.index += 1;
                }
                SchemaTokenKind::Trivia(_) => {
                    self.index += 1;
                }
                _ => break,
            }
        }

        self.rendered.push('>');

        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeEnd { name: end }
                    if end.as_deref().map(|end| end == name).unwrap_or(true) =>
                {
                    self.index += 1;
                    break;
                }
                SchemaTokenKind::NodeStart { name } if name == "$" => self.render_expression_node(),
                SchemaTokenKind::NodeStart { .. } => self.render_element(),
                SchemaTokenKind::Text(text) | SchemaTokenKind::Trivia(text) => {
                    escape_text_into(&mut self.rendered, text);
                    self.index += 1;
                }
                SchemaTokenKind::ExpressionNode(source) => {
                    let rendered = self.evaluate_to_string(source, self.tokens[self.index].clone());
                    escape_text_into(&mut self.rendered, &rendered);
                    self.index += 1;
                }
                SchemaTokenKind::Comment(text) => {
                    self.rendered.push_str("<!--");
                    self.rendered.push_str(text);
                    self.rendered.push_str("-->");
                    self.index += 1;
                }
                _ => {
                    self.index += 1;
                }
            }
        }

        self.rendered.push_str("</");
        self.rendered.push_str(&name);
        self.rendered.push('>');
    }

    fn render_expression_node(&mut self) {
        let host = self.tokens[self.index].clone();
        self.index += 1;
        let mut source = String::new();

        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::ExpressionNode(body) => {
                    source.push_str(body);
                    self.index += 1;
                }
                SchemaTokenKind::NodeEnd { name } if name.as_deref() == Some("$") => {
                    self.index += 1;
                    break;
                }
                SchemaTokenKind::Trivia(_) => {
                    self.index += 1;
                }
                _ => {
                    self.index += 1;
                }
            }
        }

        let rendered = self.evaluate_to_string(&source, host);
        escape_text_into(&mut self.rendered, &rendered);
    }

    fn evaluate_attribute_value(
        &mut self,
        _name: &str,
        value: &str,
        host: SchemaToken,
    ) -> Option<String> {
        if let Some(source) = whole_avt_expression(value) {
            let value = self.evaluate_to_string(source, host);
            if value.is_empty() {
                return None;
            }
            return Some(value);
        }

        Some(interpolate_avt(value, |source| {
            self.evaluate_to_string(source, host.clone())
        }))
    }

    fn evaluate_to_string(&mut self, source: &str, host: SchemaToken) -> String {
        let source = normalize_host_expression(source);
        match compile(source, &self.compile_context) {
            Ok(query) => {
                let stream = evaluate(&query, &self.evaluation_context);
                self.diagnostics.extend(stream.diagnostics.clone());
                if let Some(error) = stream.error {
                    self.diagnostics.push(render_diagnostic(
                        "cem.ql.render.eval_failed",
                        format!("template expression `{source}` failed: {error:?}"),
                        &host,
                    ));
                    return String::new();
                }
                stream_to_string(&stream)
            }
            Err(error) => {
                self.diagnostics.push(render_diagnostic(
                    "cem.ql.render.compile_failed",
                    format!("template expression `{source}` failed to compile: {error}"),
                    &host,
                ));
                String::new()
            }
        }
    }
}

fn stream_to_string(stream: &ItemStream) -> String {
    stream
        .items
        .iter()
        .map(item_to_string)
        .collect::<Vec<_>>()
        .join("")
}

fn item_to_string(item: &Item) -> String {
    match item {
        Item::Atomic(AtomValue::String(value)) => value.clone(),
        Item::Atomic(AtomValue::Integer(value)) => value.to_string(),
        Item::Atomic(AtomValue::Decimal(value)) => value.clone(),
        Item::Atomic(AtomValue::Double(value)) => value.to_string(),
        Item::Atomic(AtomValue::Boolean(value)) => value.to_string(),
        Item::Atomic(AtomValue::AnyUri(value)) => value.clone(),
        Item::Atomic(AtomValue::Null) => String::new(),
        Item::Node(value) => value.clone(),
        Item::Record(_) | Item::Array(_) | Item::Lambda(_) | Item::Resource(_) => String::new(),
    }
}

fn normalize_host_expression(source: &str) -> &str {
    let trimmed = source.trim();
    if let Some(rest) = trimmed.strip_prefix('$') {
        let is_simple_binding = !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
        if is_simple_binding {
            return rest;
        }
    }
    trimmed
}

fn whole_avt_expression(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        Some(trimmed[1..trimmed.len() - 1].trim())
    } else {
        None
    }
}

fn interpolate_avt(value: &str, mut eval: impl FnMut(&str) -> String) -> String {
    let mut out = String::new();
    let mut chars = value.char_indices().peekable();
    let mut literal_start = 0;
    while let Some((offset, c)) = chars.next() {
        if c != '{' {
            continue;
        }
        if matches!(chars.peek(), Some((_, '{'))) {
            let (_, next) = chars.next().expect("peeked char exists");
            debug_assert_eq!(next, '{');
            out.push_str(&value[literal_start..offset]);
            out.push('{');
            literal_start = offset + 2;
            continue;
        }

        let mut depth = 1u32;
        let body_start = offset + 1;
        let mut body_end = None;
        while let Some((inner_offset, inner)) = chars.next() {
            match inner {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = Some(inner_offset);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end) = body_end {
            out.push_str(&value[literal_start..offset]);
            out.push_str(&eval(value[body_start..end].trim()));
            literal_start = end + 1;
        }
    }
    out.push_str(&value[literal_start..]);
    out
}

fn render_diagnostic(code: &str, message: String, host: &SchemaToken) -> Diagnostic {
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: Some(host.byte_range.start),
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        node: None,
        source_map: Some(host.source_map.clone()),
    }
}

fn escape_text_into(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn escape_attr_into(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}
