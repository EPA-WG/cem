//! Data-bound CEM-ML template rendering.
//!
//! This C2 slice gives the runtime a compile-once/render-many boundary:
//! canonical CEM-ML is tokenized by `cem_ml`, embedded CEM-QL expressions are
//! compiled by this crate, and render turns a host data snapshot into a
//! serializable-style render plan. A convenience HTML renderer remains for
//! Rust tests and CLI-style callers.

use std::collections::BTreeMap;

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::scheduler::ScopePolicy;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};

use crate::api::{compile, evaluate, CompileContext, EvaluationContext};
use crate::eval::{AtomValue, Item, ItemStream, QueryContextScope};
use crate::ir::CompiledQuery;

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

#[derive(Debug, Clone, Default)]
pub struct CompileTemplateOptions {
    pub host_bindings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TemplateArtifact {
    pub nodes: Vec<TemplateNode>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub enum TemplateNode {
    Element {
        tag: String,
        attributes: Vec<TemplateAttribute>,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
    Text {
        text: String,
        source_map: SourceMapStack,
    },
    Comment {
        text: String,
        source_map: SourceMapStack,
    },
    Expression(CompiledTemplateExpression),
}

#[derive(Debug, Clone)]
pub struct TemplateAttribute {
    pub name: String,
    pub value: Option<TemplateAttributeValue>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone)]
pub enum TemplateAttributeValue {
    Literal(String),
    Template(Vec<TemplateAttributePart>),
    Expression(CompiledTemplateExpression),
}

#[derive(Debug, Clone)]
pub enum TemplateAttributePart {
    Literal(String),
    Expression(CompiledTemplateExpression),
}

#[derive(Debug, Clone)]
pub struct CompiledTemplateExpression {
    pub source: String,
    pub query: Option<CompiledQuery>,
    pub source_map: SourceMapStack,
    pub byte_offset: u64,
}

#[derive(Debug, Clone)]
pub struct RenderPlan {
    pub nodes: Vec<RenderPlanNode>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderPlanNode {
    Element {
        tag: String,
        attributes: Vec<RenderPlanAttribute>,
        children: Vec<RenderPlanNode>,
        source_map: SourceMapStack,
    },
    Text {
        text: String,
        source_map: SourceMapStack,
    },
    Comment {
        text: String,
        source_map: SourceMapStack,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPlanAttribute {
    pub name: String,
    pub value: String,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    pub rendered: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn compile_template(source: &str, options: &CompileTemplateOptions) -> TemplateArtifact {
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }

    let compile_context = CompileContext {
        policy_bindings: options
            .host_bindings
            .iter()
            .map(|name| (name.clone(), ItemStream::empty()))
            .collect(),
        ..CompileContext::default()
    };
    let mut compiler = TemplateCompiler {
        tokens: &tokens,
        index: 0,
        compile_context,
        diagnostics: tokenizer.take_diagnostics(),
    };
    let nodes = compiler.compile_all();
    TemplateArtifact {
        nodes,
        diagnostics: compiler.diagnostics,
    }
}

pub fn render_compiled_template(artifact: &TemplateArtifact, data: &TemplateData) -> RenderPlan {
    let mut renderer = PlanRenderer {
        evaluation_context: EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
            policy_bindings: data.bindings.clone(),
        },
        diagnostics: artifact.diagnostics.clone(),
    };
    let nodes = artifact
        .nodes
        .iter()
        .filter_map(|node| renderer.render_node(node))
        .collect();
    RenderPlan {
        nodes,
        diagnostics: renderer.diagnostics,
    }
}

pub fn render_template(source: &str, data: &TemplateData) -> RenderedTemplate {
    let options = CompileTemplateOptions {
        host_bindings: data.bindings.keys().cloned().collect(),
    };
    let artifact = compile_template(source, &options);
    let plan = render_compiled_template(&artifact, data);
    RenderedTemplate {
        rendered: render_plan_to_html(&plan),
        diagnostics: plan.diagnostics,
    }
}

pub fn render_plan_to_html(plan: &RenderPlan) -> String {
    let mut out = String::new();
    for node in &plan.nodes {
        render_plan_node_to_html(node, &mut out);
    }
    out
}

struct TemplateCompiler<'a> {
    tokens: &'a [SchemaToken],
    index: usize,
    compile_context: CompileContext,
    diagnostics: Vec<Diagnostic>,
}

impl TemplateCompiler<'_> {
    fn compile_all(&mut self) -> Vec<TemplateNode> {
        let mut nodes = Vec::new();
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeStart { name } if name == "$" => {
                    nodes.push(TemplateNode::Expression(self.compile_expression_node()));
                }
                SchemaTokenKind::NodeStart { .. } => nodes.push(self.compile_element()),
                SchemaTokenKind::Text(text) | SchemaTokenKind::Trivia(text) => {
                    let token = self.tokens[self.index].clone();
                    nodes.push(TemplateNode::Text {
                        text: text.clone(),
                        source_map: frame_for(&token),
                    });
                    self.index += 1;
                }
                SchemaTokenKind::Comment(text) => {
                    let token = self.tokens[self.index].clone();
                    nodes.push(TemplateNode::Comment {
                        text: text.clone(),
                        source_map: frame_for(&token),
                    });
                    self.index += 1;
                }
                _ => self.index += 1,
            }
        }
        nodes
    }

    fn compile_element(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let SchemaTokenKind::NodeStart { name } = &start.kind else {
            unreachable!("compile_element is called only at NodeStart");
        };
        let tag = name.clone();
        self.index += 1;

        let mut attributes = Vec::new();
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let token = self.tokens[self.index].clone();
                    attributes.push(TemplateAttribute {
                        name: name.clone(),
                        value: value
                            .as_ref()
                            .map(|value| self.compile_attribute_value(value, &token)),
                        source_map: frame_for(&token),
                    });
                    self.index += 1;
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }

        let mut children = Vec::new();
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeEnd { name: end }
                    if end.as_deref().map(|end| end == tag).unwrap_or(true) =>
                {
                    self.index += 1;
                    break;
                }
                SchemaTokenKind::NodeStart { name } if name == "$" => {
                    children.push(TemplateNode::Expression(self.compile_expression_node()));
                }
                SchemaTokenKind::NodeStart { .. } => children.push(self.compile_element()),
                SchemaTokenKind::Text(text) | SchemaTokenKind::Trivia(text) => {
                    let token = self.tokens[self.index].clone();
                    children.push(TemplateNode::Text {
                        text: text.clone(),
                        source_map: frame_for(&token),
                    });
                    self.index += 1;
                }
                SchemaTokenKind::Comment(text) => {
                    let token = self.tokens[self.index].clone();
                    children.push(TemplateNode::Comment {
                        text: text.clone(),
                        source_map: frame_for(&token),
                    });
                    self.index += 1;
                }
                _ => self.index += 1,
            }
        }

        TemplateNode::Element {
            tag,
            attributes,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_expression_node(&mut self) -> CompiledTemplateExpression {
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
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => self.index += 1,
            }
        }

        self.compile_expression(&source, &host)
    }

    fn compile_attribute_value(
        &mut self,
        value: &str,
        host: &SchemaToken,
    ) -> TemplateAttributeValue {
        if let Some(source) = whole_avt_expression(value) {
            return TemplateAttributeValue::Expression(self.compile_expression(source, host));
        }

        let parts = split_avt(value)
            .into_iter()
            .map(|part| match part {
                RawAttributePart::Literal(value) => TemplateAttributePart::Literal(value),
                RawAttributePart::Expression(source) => {
                    TemplateAttributePart::Expression(self.compile_expression(&source, host))
                }
            })
            .collect::<Vec<_>>();
        if parts.len() == 1 {
            if let Some(TemplateAttributePart::Literal(value)) = parts.first() {
                return TemplateAttributeValue::Literal(value.clone());
            }
        }
        TemplateAttributeValue::Template(parts)
    }

    fn compile_expression(
        &mut self,
        source: &str,
        host: &SchemaToken,
    ) -> CompiledTemplateExpression {
        let source = normalize_host_expression(source).to_owned();
        let query = match compile(&source, &self.compile_context) {
            Ok(query) => Some(query),
            Err(error) => {
                self.diagnostics.push(render_diagnostic(
                    "cem.ql.render.compile_failed",
                    format!("template expression `{source}` failed to compile: {error}"),
                    host.byte_range.start,
                    host.source_map.clone(),
                ));
                None
            }
        };
        CompiledTemplateExpression {
            source,
            query,
            source_map: frame_for(host),
            byte_offset: host.byte_range.start,
        }
    }
}

struct PlanRenderer {
    evaluation_context: EvaluationContext,
    diagnostics: Vec<Diagnostic>,
}

impl PlanRenderer {
    fn render_node(&mut self, node: &TemplateNode) -> Option<RenderPlanNode> {
        match node {
            TemplateNode::Element {
                tag,
                attributes,
                children,
                source_map,
            } => Some(RenderPlanNode::Element {
                tag: tag.clone(),
                attributes: attributes
                    .iter()
                    .filter_map(|attribute| self.render_attribute(attribute))
                    .collect(),
                children: children
                    .iter()
                    .filter_map(|child| self.render_node(child))
                    .collect(),
                source_map: source_map.clone(),
            }),
            TemplateNode::Text { text, source_map } => Some(RenderPlanNode::Text {
                text: text.clone(),
                source_map: source_map.clone(),
            }),
            TemplateNode::Comment { text, source_map } => Some(RenderPlanNode::Comment {
                text: text.clone(),
                source_map: source_map.clone(),
            }),
            TemplateNode::Expression(expression) => Some(RenderPlanNode::Text {
                text: self.evaluate_to_string(expression),
                source_map: expression.source_map.clone(),
            }),
        }
    }

    fn render_attribute(&mut self, attribute: &TemplateAttribute) -> Option<RenderPlanAttribute> {
        let value = match &attribute.value {
            None => String::new(),
            Some(TemplateAttributeValue::Literal(value)) => value.clone(),
            Some(TemplateAttributeValue::Template(parts)) => {
                let mut value = String::new();
                for part in parts {
                    match part {
                        TemplateAttributePart::Literal(literal) => value.push_str(literal),
                        TemplateAttributePart::Expression(expression) => {
                            value.push_str(&self.evaluate_to_string(expression));
                        }
                    }
                }
                value
            }
            Some(TemplateAttributeValue::Expression(expression)) => {
                let value = self.evaluate_to_string(expression);
                if value.is_empty() {
                    return None;
                }
                value
            }
        };
        Some(RenderPlanAttribute {
            name: attribute.name.clone(),
            value,
            source_map: attribute.source_map.clone(),
        })
    }

    fn evaluate_to_string(&mut self, expression: &CompiledTemplateExpression) -> String {
        let Some(query) = &expression.query else {
            return String::new();
        };
        let stream = evaluate(query, &self.evaluation_context);
        self.diagnostics.extend(stream.diagnostics.clone());
        if let Some(error) = stream.error {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.eval_failed",
                format!(
                    "template expression `{}` failed: {error:?}",
                    expression.source
                ),
                expression.byte_offset,
                expression.source_map.clone(),
            ));
            return String::new();
        }
        stream_to_string(&stream)
    }
}

enum RawAttributePart {
    Literal(String),
    Expression(String),
}

fn split_avt(value: &str) -> Vec<RawAttributePart> {
    let mut out = Vec::new();
    let mut chars = value.char_indices().peekable();
    let mut literal_start = 0;
    while let Some((offset, c)) = chars.next() {
        if c != '{' {
            continue;
        }
        if matches!(chars.peek(), Some((_, '{'))) {
            let (_, next) = chars.next().expect("peeked char exists");
            debug_assert_eq!(next, '{');
            if literal_start < offset {
                out.push(RawAttributePart::Literal(
                    value[literal_start..offset].to_owned(),
                ));
            }
            out.push(RawAttributePart::Literal("{".to_owned()));
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
            if literal_start < offset {
                out.push(RawAttributePart::Literal(
                    value[literal_start..offset].to_owned(),
                ));
            }
            out.push(RawAttributePart::Expression(
                value[body_start..end].trim().to_owned(),
            ));
            literal_start = end + 1;
        }
    }
    if literal_start < value.len() {
        out.push(RawAttributePart::Literal(value[literal_start..].to_owned()));
    }
    if out.is_empty() {
        out.push(RawAttributePart::Literal(value.to_owned()));
    }
    out
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

/// Build a per-node source-map stack from a token's real absolute `byte_range`.
///
/// The CEM tokenizer stamps every token's `source_map` with the whole-document
/// base frame, so cloning it loses per-node offsets. The accurate location lives
/// on `token.byte_range`; this rebuilds a single-frame stack from it so render
/// plans (and the WASM `byteOffset`) carry author-byte-exact per-node frames.
fn frame_for(token: &SchemaToken) -> SourceMapStack {
    let source_id = token
        .source_map
        .origin()
        .map(|frame| frame.source_id)
        .unwrap_or(SourceId(1));
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id,
            span: FrameSpan::Single(token.byte_range),
            transform: TransformKind::CemTokenizer,
        }],
    }
}

fn render_diagnostic(
    code: &str,
    message: String,
    byte_offset: u64,
    source_map: SourceMapStack,
) -> Diagnostic {
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: Some(byte_offset),
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        node: None,
        source_map: Some(source_map),
    }
}

fn render_plan_node_to_html(node: &RenderPlanNode, out: &mut String) {
    match node {
        RenderPlanNode::Element {
            tag,
            attributes,
            children,
            ..
        } => {
            out.push('<');
            out.push_str(tag);
            for attribute in attributes {
                out.push(' ');
                out.push_str(&attribute.name);
                if !attribute.value.is_empty() {
                    out.push_str("=\"");
                    escape_attr_into(out, &attribute.value);
                    out.push('"');
                }
            }
            out.push('>');
            for child in children {
                render_plan_node_to_html(child, out);
            }
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        RenderPlanNode::Text { text, .. } => escape_text_into(out, text),
        RenderPlanNode::Comment { text, .. } => {
            out.push_str("<!--");
            out.push_str(text);
            out.push_str("-->");
        }
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
