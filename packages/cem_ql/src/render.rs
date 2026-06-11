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
use crate::eval::{effective_boolean, AtomValue, Item, ItemStream, QueryContextScope};
use crate::ir::CompiledQuery;

/// Binding name under which the `/datadom` data document is exposed to expressions.
const DATA_DOCUMENT_BINDING: &str = "datadom";
/// Loop-position binding name. The legacy HTML+XSLT bridge rewrites XPath `position()` to
/// `$position`; `cem:for-each` binds it to the 1-based iteration index.
const POSITION_BINDING: &str = "position";

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
    /// `cem:if` — emits its children only when `test` is truthy.
    If {
        test: Option<CompiledTemplateExpression>,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
    /// `cem:choose` — emits the children of the first branch whose `test` is truthy
    /// (a branch with `test: None` is `cem:otherwise`); at most one branch contributes.
    Choose {
        branches: Vec<ChooseBranch>,
        source_map: SourceMapStack,
    },
    /// `cem:for-each` — evaluates `select` to a sequence and renders `children` once per item,
    /// binding the current item to `$as` (default `item`). Flattens like the conditionals (no
    /// wrapper element).
    ForEach {
        select: Option<CompiledTemplateExpression>,
        as_name: String,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
}

#[derive(Debug, Clone)]
pub struct ChooseBranch {
    pub test: Option<CompiledTemplateExpression>,
    pub children: Vec<TemplateNode>,
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

    let mut declared_bindings: BTreeMap<String, ItemStream> = options
        .host_bindings
        .iter()
        .map(|name| (name.clone(), ItemStream::empty()))
        .collect();
    // The `/datadom` data document is always available to expressions for functional
    // selection (e.g. `datadom.attributes.label`), so declare it at compile time.
    declared_bindings.insert(DATA_DOCUMENT_BINDING.to_owned(), ItemStream::empty());
    // `{attribute @name=X}` / `{slice @name=X}` declarations introduce `$X` bindings, so
    // declare them too — the render engine owns declaration metadata, so the host runtime
    // no longer needs to scan the template to make `{$X}` compile.
    for name in scan_declaration_names(&tokens) {
        declared_bindings.entry(name).or_insert_with(ItemStream::empty);
    }
    let compile_context = CompileContext {
        policy_bindings: declared_bindings,
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
    let mut policy_bindings = data.bindings.clone();
    let datadom = data
        .bindings
        .get(DATA_DOCUMENT_BINDING)
        .cloned()
        .unwrap_or_else(|| build_data_document(&data.bindings));
    policy_bindings.insert(DATA_DOCUMENT_BINDING.to_owned(), datadom);
    seed_declaration_defaults(&artifact.nodes, &mut policy_bindings);
    let mut renderer = PlanRenderer {
        evaluation_context: EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
            policy_bindings,
        },
        diagnostics: artifact.diagnostics.clone(),
    };
    let mut nodes = Vec::new();
    for node in &artifact.nodes {
        if is_top_level_declaration(node) {
            continue;
        }
        renderer.render_into(node, &mut nodes);
    }
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

/// Build the `/datadom` data document exposed to cem-ql expressions for functional
/// data selection. Host bindings (the attributes/slices the runtime supplies) become
/// `datadom.attributes.<name>`, the functional-parity equivalent of the legacy
/// `/datadom/attributes` XPath model — navigated with cem-ql record/pipeline access
/// (`record_field`) rather than an XPath engine.
fn build_data_document(bindings: &BTreeMap<String, ItemStream>) -> ItemStream {
    let attributes: BTreeMap<String, Vec<Item>> = bindings
        .iter()
        .filter(|(name, _)| name.as_str() != DATA_DOCUMENT_BINDING)
        .map(|(name, stream)| (name.clone(), stream.items.clone()))
        .collect();
    let mut datadom = BTreeMap::new();
    datadom.insert("attributes".to_owned(), vec![Item::Record(attributes)]);
    ItemStream::once(Item::Record(datadom))
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
            if matches!(
                self.tokens[self.index].kind,
                SchemaTokenKind::NodeEnd { .. }
            ) {
                self.index += 1;
                continue;
            }
            if let Some(node) = self.compile_node() {
                nodes.push(node);
            }
        }
        nodes
    }

    /// Parse the node at the cursor (advancing it), or skip a stray token (returns `None`).
    fn compile_node(&mut self) -> Option<TemplateNode> {
        match &self.tokens[self.index].kind {
            SchemaTokenKind::NodeStart { name } if name == "$" => {
                Some(TemplateNode::Expression(self.compile_expression_node()))
            }
            SchemaTokenKind::NodeStart { name } if is_if_name(name) => Some(self.compile_if()),
            SchemaTokenKind::NodeStart { name } if is_choose_name(name) => {
                Some(self.compile_choose())
            }
            SchemaTokenKind::NodeStart { name } if is_for_each_name(name) => {
                Some(self.compile_for_each())
            }
            SchemaTokenKind::NodeStart { .. } => Some(self.compile_element()),
            SchemaTokenKind::Text(text) | SchemaTokenKind::Trivia(text) => {
                let text = text.clone();
                let token = self.tokens[self.index].clone();
                self.index += 1;
                Some(TemplateNode::Text {
                    text,
                    source_map: frame_for(&token),
                })
            }
            // Triple-backtick rich content is verbatim text: its body is emitted as-is with
            // braces preserved, so generators can produce output that itself contains literal
            // `{`/`}` (e.g. CSS rule blocks `:root { … }`) without colliding with cem-ml's
            // structural braces. No interpolation happens inside — pair it with sibling
            // `{cem:for-each …}`/`{$…}` nodes for the dynamic parts.
            SchemaTokenKind::RichContent { data } => {
                let text = data.clone();
                let token = self.tokens[self.index].clone();
                self.index += 1;
                Some(TemplateNode::Text {
                    text,
                    source_map: frame_for(&token),
                })
            }
            SchemaTokenKind::Comment(text) => {
                let text = text.clone();
                let token = self.tokens[self.index].clone();
                self.index += 1;
                Some(TemplateNode::Comment {
                    text,
                    source_map: frame_for(&token),
                })
            }
            _ => {
                self.index += 1;
                None
            }
        }
    }

    /// Parse children until the `NodeEnd` matching `tag` (or an unnamed close `}`).
    fn parse_children(&mut self, tag: &str) -> Vec<TemplateNode> {
        let mut children = Vec::new();
        while self.index < self.tokens.len() {
            if let SchemaTokenKind::NodeEnd { name: end } = &self.tokens[self.index].kind {
                let closes = end.as_deref().map(|end| end == tag).unwrap_or(true);
                self.index += 1;
                if closes {
                    break;
                }
                continue;
            }
            if let Some(node) = self.compile_node() {
                children.push(node);
            }
        }
        children
    }

    fn parse_attributes(&mut self) -> Vec<TemplateAttribute> {
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
        attributes
    }

    fn compile_element(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let SchemaTokenKind::NodeStart { name } = &start.kind else {
            unreachable!("compile_element is called only at NodeStart");
        };
        let tag = name.clone();
        self.index += 1;
        let attributes = self.parse_attributes();
        let children = self.parse_children(&tag);
        TemplateNode::Element {
            tag,
            attributes,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_if(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let parsed_test = self.parse_test_attribute();
        let test = self.require_test_attribute(parsed_test, &start, &tag);
        let children = self.parse_children(&tag);
        TemplateNode::If {
            test,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_choose(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        self.skip_attributes();
        let mut branches = Vec::new();
        let mut has_otherwise = false;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeEnd { name: end }
                    if end.as_deref().map(|end| end == tag).unwrap_or(true) =>
                {
                    self.index += 1;
                    break;
                }
                SchemaTokenKind::NodeStart { name } if is_when_name(name) => {
                    branches.push(self.compile_branch(true));
                }
                SchemaTokenKind::NodeStart { name } if is_otherwise_name(name) => {
                    let otherwise = self.tokens[self.index].clone();
                    if has_otherwise {
                        self.diagnostics.push(render_diagnostic(
                            "cem.ql.render.choose_multiple_otherwise",
                            "`cem:choose` must not contain more than one `cem:otherwise` branch"
                                .to_owned(),
                            otherwise.byte_range.start,
                            frame_for(&otherwise),
                        ));
                    }
                    has_otherwise = true;
                    branches.push(self.compile_branch(false));
                }
                SchemaTokenKind::NodeStart { .. } => {
                    let token = self.tokens[self.index].clone();
                    let name = node_start_name(&token);
                    self.diagnostics.push(render_diagnostic(
                        "cem.ql.render.choose_invalid_child",
                        format!(
                            "`cem:choose` direct children must be `cem:when` or `cem:otherwise`; found `{name}`"
                        ),
                        token.byte_range.start,
                        frame_for(&token),
                    ));
                    let _ = self.compile_element();
                }
                _ => self.index += 1,
            }
        }
        TemplateNode::Choose {
            branches,
            source_map: frame_for(&start),
        }
    }

    fn compile_branch(&mut self, is_when: bool) -> ChooseBranch {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let test = if is_when {
            let parsed_test = self.parse_test_attribute();
            self.require_test_attribute(parsed_test, &start, &tag)
        } else {
            self.skip_otherwise_attributes();
            None
        };
        let children = self.parse_children(&tag);
        ChooseBranch { test, children }
    }

    fn compile_for_each(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let (select, as_name) = self.parse_for_each_attributes(&start);
        let loop_name = as_name.unwrap_or_else(|| "item".to_owned());
        // Declare the loop variable so descendant `{$<name>}` expressions compile; restore the
        // prior declaration state after the block so the binding does not leak out of scope.
        let pre_existing = self.compile_context.policy_bindings.contains_key(&loop_name);
        self.compile_context
            .policy_bindings
            .entry(loop_name.clone())
            .or_insert_with(ItemStream::empty);
        // Also declare `position` (XSLT `position()` parity) so descendant `{$position}` compiles.
        let position_pre_existing = self
            .compile_context
            .policy_bindings
            .contains_key(POSITION_BINDING);
        self.compile_context
            .policy_bindings
            .entry(POSITION_BINDING.to_owned())
            .or_insert_with(ItemStream::empty);
        let children = self.parse_children(&tag);
        if !pre_existing {
            self.compile_context.policy_bindings.remove(&loop_name);
        }
        if !position_pre_existing {
            self.compile_context.policy_bindings.remove(POSITION_BINDING);
        }
        TemplateNode::ForEach {
            select,
            as_name: loop_name,
            children,
            source_map: frame_for(&start),
        }
    }

    /// Parse `cem:for-each` attributes: `@select` (the sequence expression, required) and `@as`
    /// (the loop variable name, default `item`; a leading `$` is tolerated). Other attributes
    /// are ignored.
    fn parse_for_each_attributes(
        &mut self,
        start: &SchemaToken,
    ) -> (Option<CompiledTemplateExpression>, Option<String>) {
        let mut select = None;
        let mut as_name = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let attr = name.clone();
                    let raw = value.clone().unwrap_or_default();
                    let token = self.tokens[self.index].clone();
                    self.index += 1;
                    match attr.as_str() {
                        "select" => select = Some(self.compile_expression(&raw, &token)),
                        "as" => {
                            let trimmed = raw.trim().trim_start_matches('$').to_owned();
                            if !trimmed.is_empty() {
                                as_name = Some(trimmed);
                            }
                        }
                        _ => {}
                    }
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        if select.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.for_each_missing_select",
                "`cem:for-each` requires a `@select` expression".to_owned(),
                start.byte_range.start,
                frame_for(start),
            ));
        }
        (select, as_name)
    }

    /// Compile the `@test` whole-expression attribute of a conditional, ignoring others.
    fn parse_test_attribute(&mut self) -> Option<CompiledTemplateExpression> {
        let mut test = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let is_test = name == "test";
                    let raw = value.clone().unwrap_or_default();
                    let token = self.tokens[self.index].clone();
                    self.index += 1;
                    if is_test {
                        test = Some(self.compile_expression(&raw, &token));
                    }
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        test
    }

    fn require_test_attribute(
        &mut self,
        test: Option<CompiledTemplateExpression>,
        token: &SchemaToken,
        conditional_name: &str,
    ) -> Option<CompiledTemplateExpression> {
        if test.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.conditional_test_missing",
                format!("`{conditional_name}` requires a `@test` attribute"),
                token.byte_range.start,
                frame_for(token),
            ));
        }
        test
    }

    fn skip_otherwise_attributes(&mut self) {
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, .. } => {
                    let token = self.tokens[self.index].clone();
                    if name == "test" {
                        self.diagnostics.push(render_diagnostic(
                            "cem.ql.render.otherwise_test_not_allowed",
                            "`cem:otherwise` must not declare a `@test` attribute".to_owned(),
                            token.byte_range.start,
                            frame_for(&token),
                        ));
                    }
                    self.index += 1;
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
    }

    fn skip_attributes(&mut self) {
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { .. } | SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
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
    /// Render a template node, appending zero or more plan nodes to `out`. Conditionals
    /// (`cem:if`/`cem:choose`) contribute the children of the selected branch (or none),
    /// so they flatten into the surrounding sequence rather than emitting a wrapper.
    fn render_into(&mut self, node: &TemplateNode, out: &mut Vec<RenderPlanNode>) {
        match node {
            TemplateNode::Element {
                tag,
                attributes,
                children,
                source_map,
            } => {
                let attributes = attributes
                    .iter()
                    .filter_map(|attribute| self.render_attribute(attribute))
                    .collect();
                let mut child_nodes = Vec::new();
                for child in children {
                    self.render_into(child, &mut child_nodes);
                }
                out.push(RenderPlanNode::Element {
                    tag: tag.clone(),
                    attributes,
                    children: child_nodes,
                    source_map: source_map.clone(),
                });
            }
            TemplateNode::Text { text, source_map } => out.push(RenderPlanNode::Text {
                text: text.clone(),
                source_map: source_map.clone(),
            }),
            TemplateNode::Comment { text, source_map } => out.push(RenderPlanNode::Comment {
                text: text.clone(),
                source_map: source_map.clone(),
            }),
            TemplateNode::Expression(expression) => out.push(RenderPlanNode::Text {
                text: self.evaluate_to_string(expression),
                source_map: expression.source_map.clone(),
            }),
            TemplateNode::If { test, children, .. } => {
                if self.test_is_truthy(test.as_ref()) {
                    for child in children {
                        self.render_into(child, out);
                    }
                }
            }
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    let matched = match &branch.test {
                        None => true,
                        Some(test) => self.test_is_truthy(Some(test)),
                    };
                    if matched {
                        for child in &branch.children {
                            self.render_into(child, out);
                        }
                        break;
                    }
                }
            }
            TemplateNode::ForEach {
                select,
                as_name,
                children,
                ..
            } => {
                let items = self.evaluate_select(select.as_ref());
                let previous = self.evaluation_context.policy_bindings.get(as_name).cloned();
                // XSLT `position()` parity: bind a 1-based index for the current iteration. Saved
                // and restored alongside the loop variable so nested loops see their own position.
                let previous_position =
                    self.evaluation_context.policy_bindings.get(POSITION_BINDING).cloned();
                for (offset, item) in items.into_iter().enumerate() {
                    self.evaluation_context
                        .policy_bindings
                        .insert(as_name.clone(), ItemStream::once(item));
                    self.evaluation_context.policy_bindings.insert(
                        POSITION_BINDING.to_owned(),
                        ItemStream::once(Item::Atomic(AtomValue::Integer((offset + 1) as i64))),
                    );
                    for child in children {
                        self.render_into(child, out);
                    }
                }
                // Restore the prior bindings so the loop variables do not leak past the block.
                match previous {
                    Some(prev) => {
                        self.evaluation_context
                            .policy_bindings
                            .insert(as_name.clone(), prev);
                    }
                    None => {
                        self.evaluation_context.policy_bindings.remove(as_name);
                    }
                }
                match previous_position {
                    Some(prev) => {
                        self.evaluation_context
                            .policy_bindings
                            .insert(POSITION_BINDING.to_owned(), prev);
                    }
                    None => {
                        self.evaluation_context.policy_bindings.remove(POSITION_BINDING);
                    }
                }
            }
        }
    }

    /// Evaluate a `cem:for-each` `@select` expression to the sequence of items to iterate.
    ///
    /// A selected `Item::Array` is flattened one level into its members, so iterating a
    /// data-document collection (e.g. `$datadom.slices.geometry` — the token rows the host
    /// bridge shapes from a `<table>`, delivered through the JSON boundary as a single array
    /// item) yields one iteration per row, matching legacy XSLT `for-each` node-set iteration.
    /// A bare sequence already iterates per item, so only array items are expanded.
    fn evaluate_select(&mut self, select: Option<&CompiledTemplateExpression>) -> Vec<Item> {
        let Some(select) = select else {
            return Vec::new();
        };
        let Some(query) = &select.query else {
            return Vec::new();
        };
        let stream = evaluate(query, &self.evaluation_context);
        self.diagnostics.extend(stream.diagnostics.clone());
        if let Some(error) = stream.error {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.for_each_failed",
                format!("`cem:for-each` select `{}` failed: {error:?}", select.source),
                select.byte_offset,
                select.source_map.clone(),
            ));
            return Vec::new();
        }
        stream
            .items
            .into_iter()
            .flat_map(|item| match item {
                Item::Array(members) => members,
                other => vec![other],
            })
            .collect()
    }

    /// Evaluate a conditional `@test` expression to a cem-ql effective-boolean.
    fn test_is_truthy(&mut self, test: Option<&CompiledTemplateExpression>) -> bool {
        let Some(test) = test else {
            return false;
        };
        let Some(query) = &test.query else {
            return false;
        };
        let stream = evaluate(query, &self.evaluation_context);
        self.diagnostics.extend(stream.diagnostics.clone());
        if let Some(error) = stream.error {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.test_failed",
                format!("conditional test `{}` failed: {error:?}", test.source),
                test.byte_offset,
                test.source_map.clone(),
            ));
            return false;
        }
        effective_boolean(&stream.items)
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

/// Top-level `<attribute>` / `<slice>` declarations configure the produced element
/// (declared attributes, slice state) rather than producing visible output, so they are
/// dropped from the render plan — matching the cem-elements projection boundary.
fn is_top_level_declaration(node: &TemplateNode) -> bool {
    matches!(node, TemplateNode::Element { tag, .. } if tag == "attribute" || tag == "slice")
}

/// Seed binding values from top-level `{attribute @name=X | default}` / `{slice @name=X | default}`
/// declarations: the declaration's text content is the default for `$X` when the host data
/// omits it (host-provided values win). Applying defaults in the render engine means the
/// browser runtime no longer needs to scan declarations to know them.
fn seed_declaration_defaults(nodes: &[TemplateNode], bindings: &mut BTreeMap<String, ItemStream>) {
    for node in nodes {
        let TemplateNode::Element {
            tag,
            attributes,
            children,
            ..
        } = node
        else {
            continue;
        };
        if tag != "attribute" && tag != "slice" {
            continue;
        }
        let Some(name) = declaration_name(attributes) else {
            continue;
        };
        if bindings.contains_key(&name) {
            continue; // a host-provided value overrides the declared default
        }
        let default = declaration_default_text(children);
        if !default.is_empty() {
            bindings.insert(name, ItemStream::once(Item::Atomic(AtomValue::String(default))));
        }
    }
}

/// Collect the `@name` of every `{attribute …}` / `{slice …}` declaration token, so their
/// `$name` bindings can be declared at compile time (otherwise embedded `{$name}` would fail
/// to compile with `unknown_variable`).
fn scan_declaration_names(tokens: &[SchemaToken]) -> Vec<String> {
    let mut names = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let SchemaTokenKind::NodeStart { name } = &tokens[index].kind else {
            index += 1;
            continue;
        };
        if name != "attribute" && name != "slice" {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        while cursor < tokens.len() {
            match &tokens[cursor].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    if name == "name" {
                        if let Some(value) = value {
                            names.push(value.clone());
                        }
                    }
                    cursor += 1;
                }
                SchemaTokenKind::Trivia(_) => cursor += 1,
                _ => break,
            }
        }
        index = cursor;
    }
    names
}

fn declaration_name(attributes: &[TemplateAttribute]) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == "name")
        .and_then(|attribute| match &attribute.value {
            Some(TemplateAttributeValue::Literal(value)) => Some(value.clone()),
            _ => None,
        })
}

fn declaration_default_text(children: &[TemplateNode]) -> String {
    let mut text = String::new();
    for child in children {
        if let TemplateNode::Text { text: chunk, .. } = child {
            text.push_str(chunk);
        }
    }
    text.trim().to_owned()
}

fn node_start_name(token: &SchemaToken) -> String {
    match &token.kind {
        SchemaTokenKind::NodeStart { name } => name.clone(),
        _ => String::new(),
    }
}

/// Local name of a (possibly `cem:`-prefixed) conditional element, so both the canonical
/// `cem:if`/`cem:choose`/... and the legacy bare `if`/`choose`/... spellings are accepted.
fn conditional_local_name(name: &str) -> &str {
    name.strip_prefix("cem:").unwrap_or(name)
}

fn is_if_name(name: &str) -> bool {
    conditional_local_name(name) == "if"
}

fn is_choose_name(name: &str) -> bool {
    conditional_local_name(name) == "choose"
}

fn is_when_name(name: &str) -> bool {
    conditional_local_name(name) == "when"
}

fn is_otherwise_name(name: &str) -> bool {
    conditional_local_name(name) == "otherwise"
}

fn is_for_each_name(name: &str) -> bool {
    conditional_local_name(name) == "for-each"
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
