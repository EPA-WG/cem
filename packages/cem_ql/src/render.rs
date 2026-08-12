//! Data-bound CEM-ML template rendering.
//!
//! This C2 slice gives the runtime a compile-once/render-many boundary:
//! canonical CEM-ML is tokenized by `cem_ml`, embedded CEM-QL expressions are
//! compiled by this crate, and render turns a host data snapshot into a
//! serializable-style render plan. A convenience HTML renderer remains for
//! Rust tests and CLI-style callers.

use std::collections::BTreeMap;

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::interpreter::{OutputSpan, OutputTarget, TransformOutput};
use cem_ml::operation_control::{
    ExecutionScopeId, OperationControl, SafePointPoller,
};
use cem_ml::scheduler::ScopePolicy;
use cem_ml::source::{ByteRange, BytesSource, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};

use crate::api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext};
use crate::eval::{effective_boolean, AtomValue, Item, ItemStream, QueryContextScope};
use crate::ir::CompiledQuery;

/// Binding name under which the `/datadom` data document is exposed to expressions.
const DATA_DOCUMENT_BINDING: &str = "datadom";
/// Stable transform primary artifact binding.
const PRIMARY_INPUT_BINDING: &str = "input";
/// Loop-position binding name. The legacy HTML+XSLT bridge rewrites XPath `position()` to
/// `position`; `cem:for-each` binds it to the 1-based iteration index.
const POSITION_BINDING: &str = "position";
const HTML_VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];
/// Browser/runtime-support bounded native template calls. The full transform-template
/// adapter has configurable limits; this render boundary keeps a conservative fixed
/// default until host options are threaded through the WASM API.
const MAX_TEMPLATE_CALL_DEPTH: usize = 32;

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
    pub skip_cemt_function_bodies: bool,
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
    /// binding the current item to `as` (default `item`). Flattens like the conditionals (no
    /// wrapper element).
    ForEach {
        select: Option<CompiledTemplateExpression>,
        as_name: String,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
    /// `cem:project-payload` — materializes the runtime's serialized payload-node records
    /// as render-plan nodes. This is the authoritative bridge for rich declarative payload;
    /// it never carries live DOM identity, JavaScript properties, or event listeners.
    ProjectPayload {
        select: Option<CompiledTemplateExpression>,
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
        namespace: Option<String>,
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
    Cdata {
        text: String,
        source_map: SourceMapStack,
    },
    ProcessingInstruction {
        target: String,
        data: String,
        source_map: SourceMapStack,
    },
}

#[derive(Debug, Clone)]
pub struct RenderPlanAttribute {
    pub name: String,
    pub namespace: Option<String>,
    pub value: String,
    /// Typed CEM-QL value before HTML/string serialization.
    ///
    /// Render-plan consumers that produce markup should continue using `value`. Runtime
    /// adapters can use this sidecar when an attribute is a semantic binding, such as
    /// CEM-native `@with:*` call parameters, where preserving booleans, numbers, records,
    /// and arrays matters.
    pub value_stream: ItemStream,
    pub source_map: SourceMapStack,
}

impl PartialEq for RenderPlanAttribute {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.namespace == other.namespace
            && self.value == other.value
            && self.source_map == other.source_map
    }
}

impl Eq for RenderPlanAttribute {}

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
    declared_bindings.insert(PRIMARY_INPUT_BINDING.to_owned(), ItemStream::empty());
    // `{attribute @name=X}` / `{slice @name=X}` declarations introduce named bindings, so
    // declare them too. The render engine owns declaration metadata, so the host runtime
    // no longer needs to scan the template to make `{$ X}` compile.
    for name in scan_declaration_names(&tokens) {
        declared_bindings
            .entry(name)
            .or_insert_with(ItemStream::empty);
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
        element_stack: Vec::new(),
        skip_cemt_function_bodies: options.skip_cemt_function_bodies,
    };
    let nodes = compiler.compile_all();
    TemplateArtifact {
        nodes,
        diagnostics: compiler.diagnostics,
    }
}

pub fn render_compiled_template(artifact: &TemplateArtifact, data: &TemplateData) -> RenderPlan {
    render_compiled_template_internal(artifact, data, None)
}

pub fn render_compiled_template_with_control(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> RenderPlan {
    render_compiled_template_internal(artifact, data, Some((control, scope)))
}

fn render_compiled_template_internal(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    control: Option<(&OperationControl, ExecutionScopeId)>,
) -> RenderPlan {
    let mut policy_bindings = data.bindings.clone();
    let datadom = data_document_with_host_bindings(&data.bindings);
    policy_bindings.insert(DATA_DOCUMENT_BINDING.to_owned(), datadom);
    seed_declaration_defaults(&artifact.nodes, &mut policy_bindings);
    let templates = collect_named_templates(&artifact.nodes);
    let mut renderer = PlanRenderer {
        evaluation_context: EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            diagnostics: Vec::new(),
            policy_bindings,
            current_item: None,
        },
        diagnostics: artifact.diagnostics.clone(),
        templates,
        call_depth: 0,
        max_call_depth: MAX_TEMPLATE_CALL_DEPTH,
        safe_points: control.map(|(control, scope)| SafePointPoller::new(control.clone(), scope)),
        control: control.map(|(control, scope)| (control.clone(), scope)),
        control_failed: false,
    };
    let mut nodes = Vec::new();
    let boundary_source = artifact
        .nodes
        .first()
        .map(template_node_source_map)
        .cloned()
        .unwrap_or_default();
    if renderer.force_render(&boundary_source) {
        for node in root_render_nodes(&artifact.nodes) {
            let mut ignored_attributes = Vec::new();
            renderer.render_into(node, &mut nodes, &mut ignored_attributes);
            if renderer.control_failed {
                break;
            }
        }
        renderer.force_render(&boundary_source);
    }
    if renderer.control_failed {
        nodes.clear();
    }
    RenderPlan {
        nodes,
        diagnostics: renderer.diagnostics,
    }
}

pub fn render_template(source: &str, data: &TemplateData) -> RenderedTemplate {
    let options = CompileTemplateOptions {
        host_bindings: data.bindings.keys().cloned().collect(),
        ..CompileTemplateOptions::default()
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
fn data_document_with_host_bindings(bindings: &BTreeMap<String, ItemStream>) -> ItemStream {
    let synthesized = build_data_document(bindings);
    let Some(explicit) = bindings.get(DATA_DOCUMENT_BINDING) else {
        return synthesized;
    };
    merge_data_documents(explicit.clone(), synthesized)
}

fn build_data_document(bindings: &BTreeMap<String, ItemStream>) -> ItemStream {
    let attributes: BTreeMap<String, Vec<Item>> = bindings
        .iter()
        .filter(|(name, _)| name.as_str() != DATA_DOCUMENT_BINDING)
        .map(|(name, stream)| (name.clone(), stream.items.clone()))
        .collect();
    let mut datadom = BTreeMap::new();
    for (name, stream) in bindings
        .iter()
        .filter(|(name, _)| name.as_str() != DATA_DOCUMENT_BINDING)
    {
        datadom.insert(name.clone(), stream.items.clone());
    }
    datadom.insert("attributes".to_owned(), vec![Item::Record(attributes)]);
    ItemStream::once(Item::Record(datadom))
}

fn merge_data_documents(mut explicit: ItemStream, synthesized: ItemStream) -> ItemStream {
    let Some(Item::Record(synthesized_fields)) = synthesized.items.first() else {
        return explicit;
    };
    for item in &mut explicit.items {
        let Item::Record(explicit_fields) = item else {
            continue;
        };
        for (name, values) in synthesized_fields {
            explicit_fields
                .entry(name.clone())
                .or_insert_with(|| values.clone());
        }
    }
    explicit
}

pub fn render_plan_to_html(plan: &RenderPlan) -> String {
    let mut renderer = RenderPlanHtmlRenderer::default();
    renderer.render_plan(plan);
    renderer.out
}

pub fn render_plan_to_html_with_source_map(plan: &RenderPlan) -> TransformOutput {
    let mut renderer = RenderPlanHtmlRenderer::default();
    renderer.render_plan(plan);
    let rendered_len = renderer.out.len() as u32;
    TransformOutput {
        target: OutputTarget::LightDomCustomElements,
        rendered: renderer.out,
        diagnostics: plan.diagnostics.clone(),
        source_map: SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(0, rendered_len)),
                transform: TransformKind::InterpreterRender,
            }],
        },
        output_spans: renderer.spans,
    }
}

pub fn render_plan_to_html_with_control(
    plan: &RenderPlan,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<TransformOutput, cem_ml::operation_control::ControlError> {
    let mut renderer = RenderPlanHtmlRenderer::controlled(control, scope);
    renderer.force()?;
    renderer.render_plan(plan);
    renderer.force()?;
    if let Some(error) = renderer.control_error {
        return Err(error);
    }
    let rendered_len = renderer.out.len() as u32;
    Ok(TransformOutput {
        target: OutputTarget::LightDomCustomElements,
        rendered: renderer.out,
        diagnostics: plan.diagnostics.clone(),
        source_map: SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(0, rendered_len)),
                transform: TransformKind::InterpreterRender,
            }],
        },
        output_spans: renderer.spans,
    })
}

pub fn render_plan_to_xml_with_source_map(plan: &RenderPlan) -> TransformOutput {
    let mut renderer = RenderPlanXmlRenderer::default();
    renderer.render_plan(plan);
    let rendered_len = renderer.out.len() as u32;
    TransformOutput {
        target: OutputTarget::Xml,
        rendered: renderer.out,
        diagnostics: plan.diagnostics.clone(),
        source_map: SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(0, rendered_len)),
                transform: TransformKind::InterpreterRender,
            }],
        },
        output_spans: renderer.spans,
    }
}

pub fn render_plan_to_xml_with_control(
    plan: &RenderPlan,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<TransformOutput, cem_ml::operation_control::ControlError> {
    let mut renderer = RenderPlanXmlRenderer::controlled(control, scope);
    renderer.force()?;
    renderer.render_plan(plan);
    renderer.force()?;
    if let Some(error) = renderer.control_error {
        return Err(error);
    }
    let rendered_len = renderer.out.len() as u32;
    Ok(TransformOutput {
        target: OutputTarget::Xml,
        rendered: renderer.out,
        diagnostics: plan.diagnostics.clone(),
        source_map: SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(0, rendered_len)),
                transform: TransformKind::InterpreterRender,
            }],
        },
        output_spans: renderer.spans,
    })
}

#[derive(Default)]
struct RenderPlanHtmlRenderer {
    out: String,
    spans: Vec<OutputSpan>,
    safe_points: Option<SafePointPoller>,
    control_error: Option<cem_ml::operation_control::ControlError>,
}

impl RenderPlanHtmlRenderer {
    fn controlled(control: &OperationControl, scope: ExecutionScopeId) -> Self {
        Self {
            safe_points: Some(SafePointPoller::new(control.clone(), scope)),
            ..Self::default()
        }
    }

    fn poll(&mut self) -> bool {
        if self.control_error.is_some() {
            return false;
        }
        let Some(safe_points) = self.safe_points.as_mut() else {
            return true;
        };
        match safe_points.poll_one() {
            Ok(()) => true,
            Err(error) => {
                self.control_error = Some(error);
                false
            }
        }
    }

    fn force(&mut self) -> Result<(), cem_ml::operation_control::ControlError> {
        if let Some(error) = self.control_error.clone() {
            return Err(error);
        }
        match self.safe_points.as_mut().map(SafePointPoller::force).transpose() {
            Ok(_) => Ok(()),
            Err(error) => {
                self.control_error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn render_plan(&mut self, plan: &RenderPlan) {
        for node in &plan.nodes {
            if !self.poll() {
                break;
            }
            self.render_node(node);
        }
    }

    fn render_node(&mut self, node: &RenderPlanNode) {
        if !self.poll() {
            return;
        }
        match node {
            RenderPlanNode::Element {
                tag,
                namespace: _,
                attributes,
                children,
                source_map,
            } => {
                let open_start = self.out.len() as u64;
                self.out.push('<');
                self.out.push_str(tag);
                for attribute in attributes {
                    if !self.poll() {
                        return;
                    }
                    self.render_attribute(attribute);
                }
                if HTML_VOID_ELEMENTS.contains(&tag.as_str()) && children.is_empty() {
                    self.out.push('>');
                    self.record_span(open_start, source_map);
                    return;
                }
                self.out.push('>');
                self.record_span(open_start, source_map);
                for child in children {
                    if !self.poll() {
                        return;
                    }
                    self.render_node(child);
                }
                let close_start = self.out.len() as u64;
                self.out.push_str("</");
                self.out.push_str(tag);
                self.out.push('>');
                self.record_span(close_start, source_map);
            }
            RenderPlanNode::Text { text, source_map } => {
                let start = self.out.len() as u64;
                self.escape_text(text);
                self.record_span(start, source_map);
            }
            RenderPlanNode::Comment { text, source_map } => {
                let start = self.out.len() as u64;
                self.out.push_str("<!--");
                self.push_raw(text);
                self.out.push_str("-->");
                self.record_span(start, source_map);
            }
            RenderPlanNode::Cdata { text, source_map } => {
                let start = self.out.len() as u64;
                self.escape_text(text);
                self.record_span(start, source_map);
            }
            RenderPlanNode::ProcessingInstruction {
                target,
                data,
                source_map,
            } => {
                let start = self.out.len() as u64;
                self.out.push_str("<?");
                self.out.push_str(target);
                if !data.is_empty() {
                    self.out.push(' ');
                    self.push_raw(data);
                }
                self.out.push_str("?>");
                self.record_span(start, source_map);
            }
        }
    }

    fn render_attribute(&mut self, attribute: &RenderPlanAttribute) {
        let start = self.out.len() as u64;
        self.out.push(' ');
        if let Some(namespace) = attribute
            .namespace
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            self.out.push_str(namespace);
            self.out.push(':');
        }
        self.out.push_str(&attribute.name);
        if !attribute.value.is_empty() {
            self.out.push_str("=\"");
            self.escape_attr(&attribute.value);
            self.out.push('"');
        }
        self.record_span(start, &attribute.source_map);
    }

    fn push_raw(&mut self, value: &str) {
        for character in value.chars() {
            if !self.poll() {
                break;
            }
            self.out.push(character);
        }
    }

    fn escape_text(&mut self, value: &str) {
        escape_controlled(&mut self.out, value, false, &mut self.safe_points, &mut self.control_error);
    }

    fn escape_attr(&mut self, value: &str) {
        escape_controlled(&mut self.out, value, true, &mut self.safe_points, &mut self.control_error);
    }

    fn record_span(&mut self, start: u64, origin: &SourceMapStack) {
        let end = self.out.len() as u64;
        if end <= start {
            return;
        }
        let mut origin = origin.clone();
        origin.push(SourceMapFrame {
            source_id: origin
                .frames
                .last()
                .map(|frame| frame.source_id)
                .unwrap_or(SourceId(0)),
            span: FrameSpan::Single(ByteRange::new(start, (end - start) as u32)),
            transform: TransformKind::InterpreterRender,
        });
        self.spans.push(OutputSpan {
            output_range: ByteRange::new(start, (end - start) as u32),
            origin,
        });
    }
}

#[derive(Default)]
struct RenderPlanXmlRenderer {
    out: String,
    spans: Vec<OutputSpan>,
    safe_points: Option<SafePointPoller>,
    control_error: Option<cem_ml::operation_control::ControlError>,
}

impl RenderPlanXmlRenderer {
    fn controlled(control: &OperationControl, scope: ExecutionScopeId) -> Self {
        Self {
            safe_points: Some(SafePointPoller::new(control.clone(), scope)),
            ..Self::default()
        }
    }

    fn poll(&mut self) -> bool {
        if self.control_error.is_some() {
            return false;
        }
        let Some(safe_points) = self.safe_points.as_mut() else {
            return true;
        };
        match safe_points.poll_one() {
            Ok(()) => true,
            Err(error) => {
                self.control_error = Some(error);
                false
            }
        }
    }

    fn force(&mut self) -> Result<(), cem_ml::operation_control::ControlError> {
        if let Some(error) = self.control_error.clone() {
            return Err(error);
        }
        match self.safe_points.as_mut().map(SafePointPoller::force).transpose() {
            Ok(_) => Ok(()),
            Err(error) => {
                self.control_error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn render_plan(&mut self, plan: &RenderPlan) {
        for node in &plan.nodes {
            if !self.poll() {
                break;
            }
            self.render_node(node);
        }
    }

    fn render_node(&mut self, node: &RenderPlanNode) {
        if !self.poll() {
            return;
        }
        match node {
            RenderPlanNode::Element {
                tag,
                namespace: _,
                attributes,
                children,
                source_map,
            } => {
                let open_start = self.out.len() as u64;
                self.out.push('<');
                self.out.push_str(tag);
                for attribute in attributes {
                    if !self.poll() {
                        return;
                    }
                    self.render_attribute(attribute);
                }
                if children.is_empty() {
                    self.out.push_str("/>");
                    self.record_span(open_start, source_map);
                    return;
                }

                self.out.push('>');
                self.record_span(open_start, source_map);
                for child in children {
                    if !self.poll() {
                        return;
                    }
                    self.render_node(child);
                }
                let close_start = self.out.len() as u64;
                self.out.push_str("</");
                self.out.push_str(tag);
                self.out.push('>');
                self.record_span(close_start, source_map);
            }
            RenderPlanNode::Text { text, source_map } => {
                let start = self.out.len() as u64;
                self.escape_text(text);
                self.record_span(start, source_map);
            }
            RenderPlanNode::Comment { text, source_map } => {
                let start = self.out.len() as u64;
                self.out.push_str("<!--");
                self.push_raw(text);
                self.out.push_str("-->");
                self.record_span(start, source_map);
            }
            RenderPlanNode::Cdata { text, source_map } => {
                let start = self.out.len() as u64;
                self.out.push_str("<![CDATA[");
                self.push_raw(text);
                self.out.push_str("]]>");
                self.record_span(start, source_map);
            }
            RenderPlanNode::ProcessingInstruction {
                target,
                data,
                source_map,
            } => {
                let start = self.out.len() as u64;
                self.out.push_str("<?");
                self.out.push_str(target);
                if !data.is_empty() {
                    self.out.push(' ');
                    self.push_raw(data);
                }
                self.out.push_str("?>");
                self.record_span(start, source_map);
            }
        }
    }

    fn render_attribute(&mut self, attribute: &RenderPlanAttribute) {
        let start = self.out.len() as u64;
        self.out.push(' ');
        if let Some(namespace) = attribute
            .namespace
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            self.out.push_str(namespace);
            self.out.push(':');
        }
        self.out.push_str(&attribute.name);
        self.out.push_str("=\"");
        self.escape_attr(&attribute.value);
        self.out.push('"');
        self.record_span(start, &attribute.source_map);
    }

    fn push_raw(&mut self, value: &str) {
        for character in value.chars() {
            if !self.poll() {
                break;
            }
            self.out.push(character);
        }
    }

    fn escape_text(&mut self, value: &str) {
        escape_controlled(&mut self.out, value, false, &mut self.safe_points, &mut self.control_error);
    }

    fn escape_attr(&mut self, value: &str) {
        escape_controlled(&mut self.out, value, true, &mut self.safe_points, &mut self.control_error);
    }

    fn record_span(&mut self, start: u64, origin: &SourceMapStack) {
        let end = self.out.len() as u64;
        if end <= start {
            return;
        }
        let mut origin = origin.clone();
        origin.push(SourceMapFrame {
            source_id: origin
                .frames
                .last()
                .map(|frame| frame.source_id)
                .unwrap_or(SourceId(0)),
            span: FrameSpan::Single(ByteRange::new(start, (end - start) as u32)),
            transform: TransformKind::InterpreterRender,
        });
        self.spans.push(OutputSpan {
            output_range: ByteRange::new(start, (end - start) as u32),
            origin,
        });
    }
}

struct TemplateCompiler<'a> {
    tokens: &'a [SchemaToken],
    index: usize,
    compile_context: CompileContext,
    diagnostics: Vec<Diagnostic>,
    element_stack: Vec<String>,
    skip_cemt_function_bodies: bool,
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
            SchemaTokenKind::NodeStart { name } if is_project_payload_name(name) => {
                Some(self.compile_project_payload())
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
        let children = if self.should_skip_cemt_function_body(&tag) {
            self.skip_children(&tag);
            Vec::new()
        } else {
            self.element_stack.push(tag.clone());
            let children = self.parse_children(&tag);
            self.element_stack.pop();
            children
        };
        TemplateNode::Element {
            tag,
            attributes,
            children,
            source_map: frame_for(&start),
        }
    }

    fn should_skip_cemt_function_body(&self, tag: &str) -> bool {
        self.skip_cemt_function_bodies
            && local_template_name(tag) == "body"
            && self.element_stack.last().is_some_and(|parent| {
                is_cemt_runtime_function_declaration_name(local_template_name(parent))
            })
    }

    fn skip_children(&mut self, tag: &str) {
        let mut depth = 0usize;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeStart { .. } => {
                    depth += 1;
                    self.index += 1;
                }
                SchemaTokenKind::NodeEnd { name } => {
                    let closes_current = name.as_deref().map(|end| end == tag).unwrap_or(true);
                    self.index += 1;
                    if depth == 0 && closes_current {
                        break;
                    }
                    depth = depth.saturating_sub(1);
                }
                _ => self.index += 1,
            }
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
        // Declare the loop variable so descendant `{$ <name>}` expressions compile; restore the
        // prior declaration state after the block so the binding does not leak out of scope.
        let pre_existing = self
            .compile_context
            .policy_bindings
            .contains_key(&loop_name);
        self.compile_context
            .policy_bindings
            .entry(loop_name.clone())
            .or_insert_with(ItemStream::empty);
        // Also declare `position` (XSLT `position()` parity) so descendant `{$ position}` compiles.
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
            self.compile_context
                .policy_bindings
                .remove(POSITION_BINDING);
        }
        TemplateNode::ForEach {
            select,
            as_name: loop_name,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_project_payload(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let mut select = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let token = self.tokens[self.index].clone();
                    let raw = value.clone().unwrap_or_default();
                    self.index += 1;
                    if name == "select" {
                        select = Some(self.compile_expression(&raw, &token));
                    }
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        if select.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.project_payload_missing_select",
                "`cem:project-payload` requires a `@select` expression".to_owned(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        let children = self.parse_children(&tag);
        if !children.is_empty() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.project_payload_children_ignored",
                "`cem:project-payload` does not accept template children".to_owned(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        TemplateNode::ProjectPayload {
            select,
            source_map: frame_for(&start),
        }
    }

    /// Parse `cem:for-each` attributes: `@select` (the sequence expression, required) and `@as`
    /// (the loop variable name, default `item`; a legacy leading `$` is tolerated). Other
    /// attributes are ignored.
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
    templates: BTreeMap<String, Vec<TemplateNode>>,
    call_depth: usize,
    max_call_depth: usize,
    safe_points: Option<SafePointPoller>,
    control: Option<(OperationControl, ExecutionScopeId)>,
    control_failed: bool,
}

impl PlanRenderer {
    fn poll_render(&mut self, source_map: &SourceMapStack) -> bool {
        if self.control_failed {
            return false;
        }
        let result = self
            .safe_points
            .as_mut()
            .map(SafePointPoller::poll_one)
            .transpose();
        self.accept_control_check(result, source_map)
    }

    fn force_render(&mut self, source_map: &SourceMapStack) -> bool {
        if self.control_failed {
            return false;
        }
        let result = self
            .safe_points
            .as_mut()
            .map(SafePointPoller::force)
            .transpose();
        self.accept_control_check(result, source_map)
    }

    fn accept_control_check(
        &mut self,
        result: Result<Option<()>, cem_ml::operation_control::ControlError>,
        source_map: &SourceMapStack,
    ) -> bool {
        let Err(error) = result else {
            return true;
        };
        if !self.control_failed {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.control_failure",
                format!("{}: {error}", error.code()),
                source_map_start(source_map),
                source_map.clone(),
            ));
        }
        self.control_failed = true;
        false
    }

    /// Render a template node, appending zero or more plan nodes to `out`. Conditionals
    /// (`cem:if`/`cem:choose`) contribute the children of the selected branch (or none),
    /// so they flatten into the surrounding sequence rather than emitting a wrapper.
    fn render_into(
        &mut self,
        node: &TemplateNode,
        out: &mut Vec<RenderPlanNode>,
        parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        let source_map = template_node_source_map(node);
        if !self.poll_render(source_map) {
            return;
        }
        match node {
            TemplateNode::Element {
                tag,
                attributes,
                children,
                source_map,
            } => {
                if local_template_name(tag) == "call" {
                    self.render_call_into(attributes, source_map, out, parent_attributes);
                    return;
                }
                if local_template_name(tag) == "body" {
                    for child in children {
                        self.render_into(child, out, parent_attributes);
                    }
                    return;
                }
                if local_template_name(tag) == "param" || is_named_template_declaration(node) {
                    return;
                }
                if local_template_name(tag) == "attribute" {
                    if let Some(attribute) =
                        self.render_constructed_attribute(attributes, children, source_map)
                    {
                        parent_attributes.push(attribute);
                    }
                    return;
                }
                if local_template_name(tag) == "element" {
                    self.render_constructed_element(attributes, children, source_map, out);
                    return;
                }
                if local_template_name(tag) == "comment" {
                    out.push(RenderPlanNode::Comment {
                        text: self.render_constructor_text(attributes, children, "value"),
                        source_map: source_map.clone(),
                    });
                    return;
                }
                if local_template_name(tag) == "cdata" {
                    out.push(RenderPlanNode::Cdata {
                        text: self.render_constructor_text(attributes, children, "value"),
                        source_map: source_map.clone(),
                    });
                    return;
                }
                if local_template_name(tag) == "processing-instruction" {
                    let Some((target, target_source_map)) = self.render_constructor_name_or_alias(
                        attributes,
                        &["target", "name"],
                        source_map,
                        "processing-instruction",
                    ) else {
                        return;
                    };
                    out.push(RenderPlanNode::ProcessingInstruction {
                        target,
                        data: self.render_constructor_text(attributes, children, "value"),
                        source_map: target_source_map,
                    });
                    return;
                }
                let mut attributes = attributes
                    .iter()
                    .filter_map(|attribute| self.render_attribute(attribute))
                    .collect::<Vec<_>>();
                let mut child_nodes = Vec::new();
                for child in children {
                    self.render_into(child, &mut child_nodes, &mut attributes);
                }
                out.push(RenderPlanNode::Element {
                    tag: tag.clone(),
                    namespace: None,
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
                        self.render_into(child, out, parent_attributes);
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
                            self.render_into(child, out, parent_attributes);
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
                let previous = self
                    .evaluation_context
                    .policy_bindings
                    .get(as_name)
                    .cloned();
                // XSLT `position()` parity: bind a 1-based index for the current iteration. Saved
                // and restored alongside the loop variable so nested loops see their own position.
                let previous_position = self
                    .evaluation_context
                    .policy_bindings
                    .get(POSITION_BINDING)
                    .cloned();
                for (offset, item) in items.into_iter().enumerate() {
                    if !self.poll_render(source_map) {
                        break;
                    }
                    self.evaluation_context
                        .policy_bindings
                        .insert(as_name.clone(), ItemStream::once(item));
                    self.evaluation_context.policy_bindings.insert(
                        POSITION_BINDING.to_owned(),
                        ItemStream::once(Item::Atomic(AtomValue::Integer((offset + 1) as i64))),
                    );
                    for child in children {
                        self.render_into(child, out, parent_attributes);
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
                        self.evaluation_context
                            .policy_bindings
                            .remove(POSITION_BINDING);
                    }
                }
            }
            TemplateNode::ProjectPayload { select, source_map } => {
                for item in self.evaluate_select(select.as_ref()) {
                    if !self.poll_render(source_map) {
                        break;
                    }
                    match payload_item_to_render_node(&item, source_map) {
                        Some(node) => out.push(node),
                        None => self.diagnostics.push(render_diagnostic(
                            "cem.ql.render.project_payload_invalid_node",
                            "`cem:project-payload` selected a value that is not a serialized payload node"
                                .to_owned(),
                            source_map_start(source_map),
                            source_map.clone(),
                        )),
                    }
                }
            }
        }
    }

    fn render_call_into(
        &mut self,
        attributes: &[TemplateAttribute],
        source_map: &SourceMapStack,
        out: &mut Vec<RenderPlanNode>,
        parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        if !self.force_render(source_map) {
            return;
        }
        if self.call_depth >= self.max_call_depth {
            self.diagnostics.push(render_diagnostic(
                "cem.transform_template.recursion_limit",
                format!(
                    "native template call recursion limit exceeded at depth {}; max depth is {}",
                    self.call_depth, self.max_call_depth
                ),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return;
        }

        let rendered_attributes = attributes
            .iter()
            .filter_map(|attribute| self.render_attribute(attribute))
            .collect::<Vec<_>>();
        let Some(template_name) = rendered_attributes
            .iter()
            .find(|attribute| attribute.name == "template")
            .map(|attribute| attribute.value.clone())
            .filter(|value| !value.is_empty())
        else {
            self.diagnostics.push(render_diagnostic(
                "cem.transform_template.call_unknown",
                "native template call is missing a `template` target".to_owned(),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return;
        };
        let Some(template_nodes) = self.templates.get(&template_name).cloned() else {
            self.diagnostics.push(render_diagnostic(
                "cem.transform_template.call_unknown",
                format!("native template call target `{template_name}` was not compiled"),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return;
        };

        let mut previous = BTreeMap::new();
        for attribute in rendered_attributes
            .iter()
            .filter(|attribute| attribute.name.starts_with("with:"))
        {
            let name = attribute.name.trim_start_matches("with:").to_owned();
            previous.insert(
                name.clone(),
                self.evaluation_context
                    .policy_bindings
                    .insert(name, attribute.value_stream.clone()),
            );
        }

        self.call_depth += 1;
        for node in &template_nodes {
            self.render_into(node, out, parent_attributes);
        }
        self.call_depth -= 1;

        for (name, value) in previous {
            match value {
                Some(stream) => {
                    self.evaluation_context.policy_bindings.insert(name, stream);
                }
                None => {
                    self.evaluation_context.policy_bindings.remove(&name);
                }
            }
        }
    }

    /// Evaluate a `cem:for-each` `@select` expression to the sequence of items to iterate.
    ///
    /// A selected `Item::Array` is flattened one level into its members, so iterating a
    /// data-document collection (e.g. `datadom.slices.geometry` — the token rows the host
    /// bridge shapes from a `<table>`, delivered as a single array item) yields one iteration
    /// per row, matching legacy XSLT `for-each` node-set iteration.
    /// A bare sequence already iterates per item, so only array items are expanded.
    fn evaluate_select(&mut self, select: Option<&CompiledTemplateExpression>) -> Vec<Item> {
        let Some(select) = select else {
            return Vec::new();
        };
        let Some(query) = &select.query else {
            return Vec::new();
        };
        let stream = self.evaluate_query(query);
        self.diagnostics.extend(stream.diagnostics.clone());
        if let Some(error) = stream.error {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.for_each_failed",
                format!(
                    "`cem:for-each` select `{}` failed: {error:?}",
                    select.source
                ),
                select.byte_offset,
                select.source_map.clone(),
            ));
            return Vec::new();
        }
        stream
            .items
            .into_iter()
            .flat_map(|item| item.members().unwrap_or_else(|| vec![item]))
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
        let stream = self.evaluate_query(query);
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
        let (value, value_stream) = self.render_attribute_value(attribute);
        let preserves_empty_value = match &attribute.value {
            None => true,
            Some(TemplateAttributeValue::Literal(value)) => value.is_empty(),
            Some(TemplateAttributeValue::Template(_))
            | Some(TemplateAttributeValue::Expression(_)) => false,
        };
        if value.is_empty()
            && !preserves_empty_value
            && (!attribute.name.starts_with("with:") || value_stream.items.is_empty())
        {
            return None;
        }
        Some(RenderPlanAttribute {
            name: attribute.name.clone(),
            namespace: None,
            value,
            value_stream,
            source_map: attribute.source_map.clone(),
        })
    }

    fn render_attribute_value(&mut self, attribute: &TemplateAttribute) -> (String, ItemStream) {
        match &attribute.value {
            None => (String::new(), string_stream(String::new())),
            Some(TemplateAttributeValue::Literal(value)) => {
                (value.clone(), string_stream(value.clone()))
            }
            Some(TemplateAttributeValue::Template(parts)) => {
                let mut value = String::new();
                for part in parts {
                    if !self.poll_render(&attribute.source_map) {
                        break;
                    }
                    match part {
                        TemplateAttributePart::Literal(literal) => value.push_str(literal),
                        TemplateAttributePart::Expression(expression) => {
                            value.push_str(&self.evaluate_to_string(expression));
                        }
                    }
                }
                let value_stream = string_stream(value.clone());
                (value, value_stream)
            }
            Some(TemplateAttributeValue::Expression(expression)) => {
                let value_stream = self.evaluate_to_stream(expression);
                let value = stream_to_string(&value_stream);
                (value, value_stream)
            }
        }
    }

    fn render_constructed_element(
        &mut self,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        source_map: &SourceMapStack,
        out: &mut Vec<RenderPlanNode>,
    ) {
        let Some((tag, tag_source_map)) =
            self.render_constructor_name(attributes, "name", source_map, "element")
        else {
            return;
        };
        let namespace = self.render_constructor_optional_text(attributes, "namespace");
        let mut rendered_attributes = Vec::new();
        let mut rendered_children = Vec::new();
        for child in children {
            self.render_into(child, &mut rendered_children, &mut rendered_attributes);
        }
        sort_render_plan_attributes(&mut rendered_attributes);
        out.push(RenderPlanNode::Element {
            tag,
            namespace,
            attributes: rendered_attributes,
            children: rendered_children,
            source_map: tag_source_map,
        });
    }

    fn render_constructed_attribute(
        &mut self,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        source_map: &SourceMapStack,
    ) -> Option<RenderPlanAttribute> {
        let (name, name_source_map) =
            self.render_constructor_name(attributes, "name", source_map, "attribute")?;
        let namespace = self.render_constructor_optional_text(attributes, "namespace");
        let Some(value_attribute) = attributes
            .iter()
            .find(|attribute| attribute.name == "value")
        else {
            let mut ignored_attributes = Vec::new();
            let mut rendered_children = Vec::new();
            for child in children {
                self.render_into(child, &mut rendered_children, &mut ignored_attributes);
            }
            let value = render_plan_nodes_to_text(&rendered_children);
            return Some(RenderPlanAttribute {
                name,
                namespace,
                value: value.clone(),
                value_stream: string_stream(value),
                source_map: name_source_map,
            });
        };
        let (value, value_stream) = self.render_attribute_value(value_attribute);
        Some(RenderPlanAttribute {
            name,
            namespace,
            value,
            value_stream,
            source_map: value_attribute.source_map.clone(),
        })
    }

    fn render_constructor_text(
        &mut self,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        value_attribute_name: &str,
    ) -> String {
        if let Some(attribute) = attributes
            .iter()
            .find(|attribute| attribute.name == value_attribute_name)
        {
            let (value, _) = self.render_attribute_value(attribute);
            return value;
        }
        let mut ignored_attributes = Vec::new();
        let mut rendered_children = Vec::new();
        for child in children {
            self.render_into(child, &mut rendered_children, &mut ignored_attributes);
        }
        render_plan_nodes_to_text(&rendered_children)
    }

    fn render_constructor_optional_text(
        &mut self,
        attributes: &[TemplateAttribute],
        attribute_name: &str,
    ) -> Option<String> {
        attributes
            .iter()
            .find(|attribute| attribute.name == attribute_name)
            .map(|attribute| self.render_attribute_value(attribute).0)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }

    fn render_constructor_name_or_alias(
        &mut self,
        attributes: &[TemplateAttribute],
        attribute_names: &[&str],
        source_map: &SourceMapStack,
        construct: &str,
    ) -> Option<(String, SourceMapStack)> {
        for attribute_name in attribute_names {
            if attributes
                .iter()
                .any(|attribute| attribute.name == *attribute_name)
            {
                return self.render_constructor_name(
                    attributes,
                    attribute_name,
                    source_map,
                    construct,
                );
            }
        }
        self.diagnostics.push(render_diagnostic(
            "cem.ql.render.dynamic_name_missing",
            format!(
                "`{construct}` constructor requires one of `{}`",
                attribute_names
                    .iter()
                    .map(|name| format!("@{name}"))
                    .collect::<Vec<_>>()
                    .join("`, `")
            ),
            source_map_start(source_map),
            source_map.clone(),
        ));
        None
    }

    fn render_constructor_name(
        &mut self,
        attributes: &[TemplateAttribute],
        attribute_name: &str,
        source_map: &SourceMapStack,
        construct: &str,
    ) -> Option<(String, SourceMapStack)> {
        let Some(attribute) = attributes
            .iter()
            .find(|attribute| attribute.name == attribute_name)
        else {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.dynamic_name_missing",
                format!("`{construct}` constructor requires `@{attribute_name}`"),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return None;
        };
        let (value, _) = self.render_attribute_value(attribute);
        let Some(name) = normalize_constructed_name(&value) else {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.dynamic_name_invalid",
                format!("`{construct}` constructor name `{value}` is not a valid output name"),
                source_map_start(&attribute.source_map),
                attribute.source_map.clone(),
            ));
            return None;
        };
        Some((name, attribute.source_map.clone()))
    }

    fn evaluate_to_string(&mut self, expression: &CompiledTemplateExpression) -> String {
        stream_to_string(&self.evaluate_to_stream(expression))
    }

    fn evaluate_to_stream(&mut self, expression: &CompiledTemplateExpression) -> ItemStream {
        let Some(query) = &expression.query else {
            return ItemStream::empty();
        };
        let stream = self.evaluate_query(query);
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
            return ItemStream::empty();
        }
        stream
    }

    fn evaluate_query(&self, query: &CompiledQuery) -> ItemStream {
        match &self.control {
            Some((control, scope)) => {
                evaluate_with_control(query, &self.evaluation_context, control, *scope)
            }
            None => evaluate(query, &self.evaluation_context),
        }
    }
}

fn string_stream(value: String) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::String(value)))
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
    if let Some(atom) = item.atom() {
        return match atom {
            AtomValue::String(value) => value,
            AtomValue::Integer(value) => value.to_string(),
            AtomValue::Decimal(value) => value,
            AtomValue::Double(value) => value.to_string(),
            AtomValue::Boolean(value) => value.to_string(),
            AtomValue::AnyUri(value) => value,
            AtomValue::Null => String::new(),
        };
    }
    match item {
        Item::Node(value) => value.clone(),
        Item::Record(_)
        | Item::Array(_)
        | Item::Native(_)
        | Item::Lambda(_)
        | Item::Resource(_) => String::new(),
        Item::Atomic(_) => unreachable!("atomic items return above"),
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
    match node {
        TemplateNode::Element {
            tag, attributes, ..
        } => match local_template_name(tag) {
            "attribute" | "slice" | "param" => true,
            "template" => declaration_name(attributes).is_some(),
            _ => false,
        },
        _ => false,
    }
}

fn is_named_template_declaration(node: &TemplateNode) -> bool {
    matches!(
        node,
        TemplateNode::Element {
            tag,
            attributes,
            ..
        } if local_template_name(tag) == "template" && declaration_name(attributes).is_some()
    )
}

/// Seed binding values from top-level `{attribute @name=X | default}` / `{slice @name=X | default}`
/// declarations: the declaration's text content is the default for `X` when the host data
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
        // Always bind a declared attribute/slice so `{$ X}` / `!X` references resolve even when
        // the host left it unset (DCE parity: a declared attribute is always referenceable). An
        // empty declaration binds Null; a non-empty default binds its text.
        let default = declaration_default_text(children);
        let value = if default.is_empty() {
            Item::Atomic(AtomValue::Null)
        } else {
            Item::Atomic(AtomValue::String(default))
        };
        bindings.insert(name, ItemStream::once(value));
    }
}

/// Collect the `@name` of every `{attribute …}` / `{slice …}` declaration token, so their
/// `name` bindings can be declared at compile time (otherwise embedded `{$ name}` would fail
/// to compile with `unknown_variable`).
fn scan_declaration_names(tokens: &[SchemaToken]) -> Vec<String> {
    let mut names = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let SchemaTokenKind::NodeStart { name } = &tokens[index].kind else {
            index += 1;
            continue;
        };
        if !matches!(local_template_name(name), "attribute" | "slice" | "param") {
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

fn root_render_nodes(nodes: &[TemplateNode]) -> Vec<&TemplateNode> {
    let mut roots = Vec::new();
    for node in nodes {
        if let TemplateNode::Element { tag, children, .. } = node {
            if local_template_name(tag) == "module" {
                roots.extend(module_body_nodes(children));
                continue;
            }
        }
        if !is_top_level_declaration(node) {
            roots.push(node);
        }
    }
    roots
}

fn template_node_source_map(node: &TemplateNode) -> &SourceMapStack {
    match node {
        TemplateNode::Element { source_map, .. }
        | TemplateNode::Text { source_map, .. }
        | TemplateNode::Comment { source_map, .. }
        | TemplateNode::If { source_map, .. }
        | TemplateNode::Choose { source_map, .. }
        | TemplateNode::ForEach { source_map, .. }
        | TemplateNode::ProjectPayload { source_map, .. } => source_map,
        TemplateNode::Expression(expression) => &expression.source_map,
    }
}

fn module_body_nodes(nodes: &[TemplateNode]) -> Vec<&TemplateNode> {
    for node in nodes {
        let TemplateNode::Element { tag, children, .. } = node else {
            continue;
        };
        if local_template_name(tag) == "body" {
            return children.iter().collect();
        }
    }
    Vec::new()
}

fn collect_named_templates(nodes: &[TemplateNode]) -> BTreeMap<String, Vec<TemplateNode>> {
    let mut templates = BTreeMap::new();
    collect_named_templates_into(nodes, &mut templates);
    templates
}

fn collect_named_templates_into(
    nodes: &[TemplateNode],
    templates: &mut BTreeMap<String, Vec<TemplateNode>>,
) {
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
        if local_template_name(tag) == "template" {
            if let Some(name) = declaration_name(attributes) {
                templates.insert(name, template_body_nodes(children));
            }
        }
        collect_named_templates_into(children, templates);
    }
}

fn template_body_nodes(children: &[TemplateNode]) -> Vec<TemplateNode> {
    for child in children {
        let TemplateNode::Element {
            tag,
            children: body,
            ..
        } = child
        else {
            continue;
        };
        if local_template_name(tag) == "body" {
            return body.clone();
        }
    }
    children
        .iter()
        .filter(|child| !is_top_level_declaration(child))
        .cloned()
        .collect()
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

fn local_template_name(name: &str) -> &str {
    conditional_local_name(name)
}

fn is_cemt_runtime_function_declaration_name(name: &str) -> bool {
    matches!(
        name,
        "function" | "encoding-function" | "format-function" | "color-function"
    )
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

fn is_project_payload_name(name: &str) -> bool {
    conditional_local_name(name) == "project-payload"
}

fn payload_item_to_render_node(item: &Item, source_map: &SourceMapStack) -> Option<RenderPlanNode> {
    let Item::Record(record) = item else {
        return None;
    };
    match record_string(record, "kind")?.as_str() {
        "text" => Some(RenderPlanNode::Text {
            text: record_string(record, "text").unwrap_or_default(),
            source_map: source_map.clone(),
        }),
        "comment" => Some(RenderPlanNode::Comment {
            text: record_string(record, "text").unwrap_or_default(),
            source_map: source_map.clone(),
        }),
        "element" => {
            let tag = record_string(record, "tag")?;
            let namespace = record_string(record, "namespace").filter(|value| !value.is_empty());
            let attributes = record_record(record, "attributes")
                .map(|attributes| {
                    attributes
                        .iter()
                        .map(|(name, values)| RenderPlanAttribute {
                            name: name.clone(),
                            namespace: None,
                            value: values.iter().map(item_to_string).collect::<String>(),
                            value_stream: ItemStream::from_items(values.clone()),
                            source_map: source_map.clone(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let children = record_items(record, "children")
                .into_iter()
                .flat_map(|item| item.members().unwrap_or_else(|| vec![item]))
                .filter_map(|item| payload_item_to_render_node(&item, source_map))
                .collect::<Vec<_>>();
            Some(RenderPlanNode::Element {
                tag,
                namespace,
                attributes,
                children,
                source_map: source_map.clone(),
            })
        }
        _ => None,
    }
}

fn record_items(record: &BTreeMap<String, Vec<Item>>, name: &str) -> Vec<Item> {
    record.get(name).cloned().unwrap_or_default()
}

fn record_record<'a>(record: &'a BTreeMap<String, Vec<Item>>, name: &str) -> Option<&'a BTreeMap<String, Vec<Item>>> {
    record.get(name)?.first().and_then(|item| match item {
        Item::Record(value) => Some(value),
        _ => None,
    })
}

fn record_string(record: &BTreeMap<String, Vec<Item>>, name: &str) -> Option<String> {
    record.get(name)?.first().map(item_to_string)
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
        details: None,
        source_map: Some(source_map),
    }
}

fn source_map_start(source_map: &SourceMapStack) -> u64 {
    source_map
        .frames
        .last()
        .and_then(|frame| match frame.span {
            FrameSpan::Single(range) => Some(range.start),
            FrameSpan::Multi(_) => None,
        })
        .unwrap_or(0)
}

fn normalize_constructed_name(value: &str) -> Option<String> {
    let name = value.trim();
    if name
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '<' | '>' | '/' | '=' | '"' | '\''))
    {
        return None;
    }
    Some(name.to_owned())
}

fn render_plan_nodes_to_text(nodes: &[RenderPlanNode]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            RenderPlanNode::Element { children, .. } => {
                text.push_str(&render_plan_nodes_to_text(children));
            }
            RenderPlanNode::Text { text: chunk, .. } => text.push_str(chunk),
            RenderPlanNode::Cdata { text: chunk, .. } => text.push_str(chunk),
            RenderPlanNode::Comment { .. } => {}
            RenderPlanNode::ProcessingInstruction { .. } => {}
        }
    }
    text
}

fn sort_render_plan_attributes(attributes: &mut [RenderPlanAttribute]) {
    attributes.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn escape_controlled(
    out: &mut String,
    value: &str,
    attribute: bool,
    safe_points: &mut Option<SafePointPoller>,
    control_error: &mut Option<cem_ml::operation_control::ControlError>,
) {
    for character in value.chars() {
        if control_error.is_some() {
            break;
        }
        if let Some(error) = safe_points
            .as_mut()
            .and_then(|safe_points| safe_points.poll_one().err())
        {
            *control_error = Some(error);
            break;
        }
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            _ => out.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(start: u64, len: u32) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(start, len)),
                transform: TransformKind::CemTokenizer,
            }],
        }
    }

    fn sample_plan() -> RenderPlan {
        RenderPlan {
            nodes: vec![RenderPlanNode::Element {
                tag: "p".to_owned(),
                namespace: None,
                attributes: vec![RenderPlanAttribute {
                    name: "title".to_owned(),
                    namespace: None,
                    value: "A&B".to_owned(),
                    value_stream: ItemStream::empty(),
                    source_map: stack(3, 12),
                }],
                children: vec![RenderPlanNode::Text {
                    text: "Hi <all>".to_owned(),
                    source_map: stack(16, 8),
                }],
                source_map: stack(0, 28),
            }],
            diagnostics: Vec::new(),
        }
    }

    fn record(fields: impl IntoIterator<Item = (&'static str, Vec<Item>)>) -> Item {
        Item::Record(
            fields
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn source_map_render_preserves_html_output() {
        let plan = sample_plan();

        assert_eq!(
            render_plan_to_html(&plan),
            r#"<p title="A&amp;B">Hi &lt;all&gt;</p>"#
        );
        assert_eq!(
            render_plan_to_html_with_source_map(&plan).rendered,
            render_plan_to_html(&plan)
        );
    }

    #[test]
    fn project_payload_materializes_serialized_rich_nodes() {
        let text = record([
            ("kind", vec![Item::Atomic(AtomValue::String("text".to_owned()))]),
            ("key", vec![Item::Atomic(AtomValue::String("0/0/0".to_owned()))]),
            ("text", vec![Item::Atomic(AtomValue::String("Ada".to_owned()))]),
        ]);
        let strong = record([
            ("kind", vec![Item::Atomic(AtomValue::String("element".to_owned()))]),
            ("key", vec![Item::Atomic(AtomValue::String("0/0".to_owned()))]),
            ("tag", vec![Item::Atomic(AtomValue::String("strong".to_owned()))]),
            ("namespace", vec![Item::Atomic(AtomValue::Null)]),
            ("attributes", vec![Item::Record(BTreeMap::new())]),
            ("children", vec![Item::Array(vec![text])]),
        ]);
        let comment = record([
            ("kind", vec![Item::Atomic(AtomValue::String("comment".to_owned()))]),
            ("key", vec![Item::Atomic(AtomValue::String("0/1".to_owned()))]),
            ("text", vec![Item::Atomic(AtomValue::String("note".to_owned()))]),
        ]);
        let span = record([
            ("kind", vec![Item::Atomic(AtomValue::String("element".to_owned()))]),
            ("key", vec![Item::Atomic(AtomValue::String("0".to_owned()))]),
            ("tag", vec![Item::Atomic(AtomValue::String("span".to_owned()))]),
            ("namespace", vec![Item::Atomic(AtomValue::Null)]),
            (
                "attributes",
                vec![Item::Record(BTreeMap::from([(
                    "class".to_owned(),
                    vec![Item::Atomic(AtomValue::String("rich".to_owned()))],
                )]))],
            ),
            ("children", vec![Item::Array(vec![strong, comment])]),
        ]);
        let payload = record([("nodes", vec![Item::Array(vec![span])])]);
        let datadom = record([("payload", vec![payload])]);
        let data = TemplateData::default()
            .with_binding("datadom", ItemStream::once(datadom));

        let rendered = render_template(
            r#"{div | {cem:project-payload @select="datadom.payload.nodes" | }}"#,
            &data,
        );

        assert_eq!(rendered.rendered, r#"<div><span class="rich"><strong>Ada</strong><!--note--></span></div>"#);
        assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
    }

    #[test]
    fn project_payload_requires_select() {
        let rendered = render_template(
            "{cem:project-payload | }",
            &TemplateData::default(),
        );
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.project_payload_missing_select"));
    }

    #[test]
    fn source_map_render_serializes_xml_specific_nodes() {
        let plan = RenderPlan {
            nodes: vec![
                RenderPlanNode::ProcessingInstruction {
                    target: "xml-stylesheet".to_owned(),
                    data: "href=\"main.css\"".to_owned(),
                    source_map: stack(0, 8),
                },
                RenderPlanNode::Element {
                    tag: "root".to_owned(),
                    namespace: None,
                    attributes: vec![RenderPlanAttribute {
                        name: "id".to_owned(),
                        namespace: None,
                        value: "a&b".to_owned(),
                        value_stream: ItemStream::empty(),
                        source_map: stack(8, 8),
                    }],
                    children: vec![
                        RenderPlanNode::Element {
                            tag: "empty".to_owned(),
                            namespace: None,
                            attributes: Vec::new(),
                            children: Vec::new(),
                            source_map: stack(16, 8),
                        },
                        RenderPlanNode::Cdata {
                            text: "x < y".to_owned(),
                            source_map: stack(24, 8),
                        },
                    ],
                    source_map: stack(8, 24),
                },
            ],
            diagnostics: Vec::new(),
        };

        assert_eq!(
            render_plan_to_xml_with_source_map(&plan).rendered,
            r#"<?xml-stylesheet href="main.css"?><root id="a&amp;b"><empty/><![CDATA[x < y]]></root>"#
        );
    }

    #[test]
    fn controlled_output_chunks_preserve_success_and_discard_cancelled_output() {
        let plan = RenderPlan {
            nodes: vec![RenderPlanNode::Text {
                text: "<hello & goodbye>".repeat(16),
                source_map: stack(0, 8),
            }],
            diagnostics: Vec::new(),
        };
        let control = OperationControl::default();
        let controlled = render_plan_to_html_with_control(
            &plan,
            &control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        )
        .unwrap();
        assert_eq!(controlled.rendered, render_plan_to_html(&plan));

        control.cancel_root(None, None).unwrap();
        let short_plan = RenderPlan {
            nodes: vec![RenderPlanNode::Text {
                text: "x".to_owned(),
                source_map: stack(0, 1),
            }],
            diagnostics: Vec::new(),
        };
        assert!(render_plan_to_html_with_control(
            &short_plan,
            &control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        )
        .is_err());
    }

    #[test]
    fn controlled_template_render_discards_a_pre_cancelled_plan() {
        let artifact = compile_template("{p | hello}", &CompileTemplateOptions::default());
        let control = OperationControl::default();
        control.cancel_root(None, None).unwrap();

        let plan = render_compiled_template_with_control(
            &artifact,
            &TemplateData::default(),
            &control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        );
        assert!(plan.nodes.is_empty());
        assert!(plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.control_failure"));
    }

    #[test]
    fn source_map_render_records_output_boundary_and_spans() {
        let output = render_plan_to_html_with_source_map(&sample_plan());

        assert_eq!(output.source_map.frames.len(), 1);
        assert!(matches!(
            output.source_map.frames[0].transform,
            TransformKind::InterpreterRender
        ));
        assert!(matches!(
            output.source_map.frames[0].span,
            FrameSpan::Single(ByteRange { start: 0, len })
                if len as usize == output.rendered.len()
        ));
        assert!(output.output_spans.iter().all(|span| {
            span.origin
                .frames
                .last()
                .is_some_and(|frame| matches!(frame.transform, TransformKind::InterpreterRender))
        }));

        let text_start = output.rendered.find("Hi").expect("text should render") as u64;
        assert!(output
            .output_spans
            .iter()
            .any(|span| span.output_range.start == text_start));
    }

    #[test]
    fn same_module_template_call_renders_body_with_params() {
        let source = r#"{module |
            {template @name="label" |
                {param @name="node"}
                {body | {span @data-kind="{node.kind}" | {$ node.text}}}
            }
            {body |
                {call @template="label" @with:node="{datadom.payload.nodes}"}
            }
        }"#;
        let mut node = BTreeMap::new();
        node.insert(
            "kind".to_owned(),
            vec![Item::Atomic(AtomValue::String("text".to_owned()))],
        );
        node.insert(
            "text".to_owned(),
            vec![Item::Atomic(AtomValue::String("Leaf".to_owned()))],
        );
        let mut payload = BTreeMap::new();
        payload.insert("nodes".to_owned(), vec![Item::Record(node)]);
        let mut datadom = BTreeMap::new();
        datadom.insert("payload".to_owned(), vec![Item::Record(payload)]);
        let data = TemplateData::default()
            .with_binding("datadom", ItemStream::once(Item::Record(datadom)));

        let rendered = render_template(source, &data);

        assert_eq!(
            rendered.rendered.trim(),
            r#"<span data-kind="text">Leaf</span>"#
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn unnamed_template_element_still_renders_as_html() {
        let rendered = render_template("{template | {span | fallback}}", &TemplateData::default());

        assert_eq!(
            rendered.rendered.trim(),
            "<template><span>fallback</span></template>"
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn native_recursive_template_renders_data_island_tree() {
        let source = r#"{module |
            {template @name="node" |
                {param @name="node"}
                {body |
                    {cem:choose |
                        {cem:when @test='node.kind == "element"' |
                            {details @open=open |
                                {summary |
                                    {b | {$ node.tag}}
                                    {cem:if @test="node.attributes.data-root" | {code | data-root="{$ node.attributes.data-root}"}}
                                    {cem:if @test="node.attributes.data-level" | {code | data-level="{$ node.attributes.data-level}"}}
                                    {cem:if @test="node.attributes.name" | {code | name="{$ node.attributes.name}"}}
                                    {cem:if @test="node.attributes.code" | {code | code="{$ node.attributes.code}"}}
                                }
                                {cem:for-each @select="node.children" @as="child" |
                                    {call @template="node" @with:node="{child}"}
                                }
                            }
                        }
                        {cem:when @test='node.kind == "text"' |
                            {p | {$ node.text}}
                        }
                    }
                }
            }
            {body |
                {article |
                    {h2 | embedded-xsl data island tree}
                    {details @open=open |
                        {summary |
                            {b | datadom}
                            {code | title="{$ datadom.attributes.title}"}
                            {code | data-demo="{$ datadom.attributes.data-demo}"}
                        }
                        {cem:for-each @select="datadom.payload.nodes" @as="node" |
                            {call @template="node" @with:node="{node}"}
                        }
                    }
                }
            }
        }"#;
        let text = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("text".to_owned()))],
            ),
            (
                "text",
                vec![Item::Atomic(AtomValue::String(
                    "Leaf text from cem-elements data island".to_owned(),
                ))],
            ),
        ]);
        let leaf = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("leaf".to_owned()))],
            ),
            (
                "attributes",
                vec![record([(
                    "data-level",
                    vec![Item::Atomic(AtomValue::String("3".to_owned()))],
                )])],
            ),
            ("children", vec![Item::Array(vec![text])]),
        ]);
        let item = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("item".to_owned()))],
            ),
            (
                "attributes",
                vec![record([(
                    "code",
                    vec![Item::Atomic(AtomValue::String("a1".to_owned()))],
                )])],
            ),
            ("children", vec![Item::Array(vec![leaf])]),
        ]);
        let section = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("section".to_owned()))],
            ),
            (
                "attributes",
                vec![record([
                    (
                        "data-level",
                        vec![Item::Atomic(AtomValue::String("1".to_owned()))],
                    ),
                    (
                        "name",
                        vec![Item::Atomic(AtomValue::String("alpha".to_owned()))],
                    ),
                ])],
            ),
            ("children", vec![Item::Array(vec![item])]),
        ]);
        let catalog = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("catalog".to_owned()))],
            ),
            (
                "attributes",
                vec![record([(
                    "data-root",
                    vec![Item::Atomic(AtomValue::String("cem-elements".to_owned()))],
                )])],
            ),
            ("children", vec![Item::Array(vec![section])]),
        ]);
        let datadom = record([
            (
                "attributes",
                vec![record([
                    (
                        "title",
                        vec![Item::Atomic(AtomValue::String(
                            "Anonymous DCE data island".to_owned(),
                        ))],
                    ),
                    (
                        "data-demo",
                        vec![Item::Atomic(AtomValue::String("cem-elements".to_owned()))],
                    ),
                ])],
            ),
            (
                "payload",
                vec![record([("nodes", vec![Item::Array(vec![catalog])])])],
            ),
        ]);
        let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

        let rendered = render_template(source, &data);

        assert!(rendered.rendered.contains("embedded-xsl data island tree"));
        assert!(rendered
            .rendered
            .contains("title=\"Anonymous DCE data island\""));
        assert!(rendered.rendered.contains("data-root=\"cem-elements\""));
        assert!(rendered.rendered.contains("data-level=\"3\""));
        assert!(rendered
            .rendered
            .contains("Leaf text from cem-elements data island"));
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn recursive_template_calls_are_bounded() {
        let source = r#"{module |
            {template @name="loop" | {body | {span | Loop {call @template="loop"}}}}
            {body | {div | {call @template="loop"}}}
        }"#;

        let rendered = render_template(source, &TemplateData::default());

        assert!(rendered.rendered.starts_with("<div><span>Loop "));
        assert!(rendered.rendered.ends_with("</span></div>"));
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "cem.transform_template.recursion_limit" }));
    }
}
