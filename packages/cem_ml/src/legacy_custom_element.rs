//! Legacy `@epa-wg/custom-element` HTML+XSLT compatibility contract and
//! bounded lowering path.
//!
//! This module records the CEM-owned compatibility surface and provides the
//! first engine-side lowering entry point for the legacy custom-element syntax.
//! The browser adapter still has a TypeScript DOM converter for parity with the
//! browser parser, but the executable contract now lives in the CEM-ML engine.
//!
//! Scope boundary:
//! - Tier 1/2 pull-style constructs lower to canonical CEM-ML + `cem_ql`.
//! - Tier 3 push-template / standalone stylesheet constructs remain an explicit
//!   handoff, not an accidental browser-only feature.

use crate::diagnostics::{Diagnostic, Severity};
use serde::Serialize;
use std::collections::HashMap;

/// Host-template language marker used by the package adapter for untyped
/// legacy declarations.
pub const TEMPLATE_LANG: &str = "custom-element-xslt";

/// Content types that opt the engine conversion path into legacy lowering.
pub const TEMPLATE_CONTENT_TYPES: &[&str] = &[
    TEMPLATE_LANG,
    "text/custom-element-xslt",
    "application/custom-element-xslt",
    "text/x-custom-element-xslt",
];

/// Diagnostic code emitted when a legacy XPath function has no CEM-QL mapping.
pub const UNSUPPORTED_FUNCTION_CODE: &str = "legacy_xslt.unsupported_function";

/// Diagnostic code emitted when a Tier 3 XSLT construct is encountered.
pub const UNSUPPORTED_CONSTRUCT_CODE: &str = "legacy_xslt.unsupported_construct";

/// Legacy elements that are lowered as control-flow / expression nodes.
pub const CONTROL_FLOW_ELEMENTS: &[&str] = &[
    "value-of",
    "text",
    "if",
    "choose",
    "when",
    "otherwise",
    "for-each",
    "variable",
    "slot",
];

/// Legacy declaration/resource helper elements preserved as CEM-ML declarations
/// or inert render helpers.
pub const DECLARATION_ELEMENTS: &[&str] = &["attribute", "slice", "data", "option", "module-url"];

/// XSLT stylesheet adapter constructs implemented by the bounded Phase 4
/// compatibility profile.
pub const STYLESHEET_COMPAT_ELEMENTS: &[&str] = &[
    "stylesheet",
    "template",
    "call-template",
    "with-param",
    "param",
    "apply-templates",
    "sort",
    "copy",
    "copy-of",
    "attribute",
    "element",
    "output",
];

/// Tier 3 XSLT constructs that are outside the current material/demo bridge.
pub const TIER3_HANDOFF_ELEMENTS: &[&str] = &["function", "script"];

/// XPath functions the bridge lowers to CEM-QL directly or by special rewrite.
pub const SUPPORTED_XPATH_FUNCTIONS: &[&str] = &[
    "contains",
    "starts-with",
    "ends-with",
    "normalize-space",
    "translate",
    "substring",
    "substring-before",
    "substring-after",
    "string-length",
    "count",
    "sum",
    "not",
    "concat",
    "position",
    "current",
];

const HTML_VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];
const MAX_TEMPLATE_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyConversionDiagnostic {
    pub code: String,
    pub message: String,
}

impl LegacyConversionDiagnostic {
    pub fn to_engine_diagnostic(&self, uri: Option<String>) -> Diagnostic {
        Diagnostic {
            uri,
            code: self.code.clone(),
            severity: Severity::Warning,
            message: self.message.clone(),
            ..Diagnostic::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyConversionResult {
    /// Canonical CEM-ML source text ready for the CEM-QL render boundary.
    pub source: String,
    pub diagnostics: Vec<LegacyConversionDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyElementDisposition {
    /// Lower as legacy control flow or expression output.
    ControlFlow,
    /// Preserve as a CEM declaration/resource helper.
    Declaration,
    /// Treat as ordinary output markup.
    OutputElement,
    /// Explicit Tier 3 handoff/deferred construct.
    Tier3Handoff,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyXsltCompatDisposition {
    /// Implemented by the bounded custom-element fragment bridge.
    FragmentBridge,
    /// Implemented by the XSLT 1.0 compatibility adapter profile.
    StylesheetCompat,
    /// Explicitly outside the current compatibility profile.
    Handoff,
}

/// Classify XSLT names for the Phase 4 compatibility adapter profile.
pub fn xslt_compat_disposition(local_name: &str) -> LegacyXsltCompatDisposition {
    match local_name {
        name if STYLESHEET_COMPAT_ELEMENTS.contains(&name) => {
            LegacyXsltCompatDisposition::StylesheetCompat
        }
        name if CONTROL_FLOW_ELEMENTS.contains(&name) || DECLARATION_ELEMENTS.contains(&name) => {
            LegacyXsltCompatDisposition::FragmentBridge
        }
        _ => LegacyXsltCompatDisposition::Handoff,
    }
}

/// Classify a local element name after any `xsl:` prefix has been stripped.
pub fn element_disposition(local_name: &str) -> LegacyElementDisposition {
    if CONTROL_FLOW_ELEMENTS.contains(&local_name) {
        LegacyElementDisposition::ControlFlow
    } else if DECLARATION_ELEMENTS.contains(&local_name) {
        LegacyElementDisposition::Declaration
    } else if TIER3_HANDOFF_ELEMENTS.contains(&local_name) {
        LegacyElementDisposition::Tier3Handoff
    } else {
        LegacyElementDisposition::OutputElement
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyFunctionDisposition {
    /// Function lowers to the given CEM-QL function name.
    CemQl(&'static str),
    /// Function is supported by special syntax rewrite rather than direct call.
    Special,
    /// Function is not in the bridge subset.
    Unsupported,
}

/// Classify a legacy XPath function by the CEM-QL lowering contract.
pub fn function_disposition(name: &str) -> LegacyFunctionDisposition {
    match name {
        "contains" => LegacyFunctionDisposition::CemQl("str:contains"),
        "starts-with" => LegacyFunctionDisposition::CemQl("str:starts_with"),
        "ends-with" => LegacyFunctionDisposition::CemQl("str:ends_with"),
        "normalize-space" => LegacyFunctionDisposition::CemQl("str:normalize_space"),
        "translate" => LegacyFunctionDisposition::CemQl("str:translate"),
        "substring" => LegacyFunctionDisposition::CemQl("str:substring"),
        "substring-before" => LegacyFunctionDisposition::CemQl("str:substring_before"),
        "substring-after" => LegacyFunctionDisposition::CemQl("str:substring_after"),
        "string-length" => LegacyFunctionDisposition::CemQl("str:length"),
        "count" => LegacyFunctionDisposition::CemQl("seq:count"),
        "sum" => LegacyFunctionDisposition::CemQl("seq:sum"),
        "not" | "concat" | "position" | "current" | "hasBoolAttribute" => {
            LegacyFunctionDisposition::Special
        }
        _ => LegacyFunctionDisposition::Unsupported,
    }
}

/// Return true when an engine request content type opts into legacy lowering.
pub fn is_legacy_custom_element_content_type(content_type: &str) -> bool {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    TEMPLATE_CONTENT_TYPES.contains(&media_type.as_str())
}

/// Lower a legacy custom-element template fragment to canonical CEM-ML source.
///
/// This is intentionally a bounded fragment converter, not a full browser DOM or
/// XSLT implementation. It covers the old custom-element pull-style dialect:
/// bare and `xsl:` control-flow elements, AVT interpolation, inline node-set
/// `for-each` unrolling, and the XPath function set declared above.
pub fn convert_template_source(source: &str) -> LegacyConversionResult {
    let mut diagnostics = Vec::new();
    let nodes = LegacyFragmentParser::new(source).parse();
    let ctx = EmitCtx {
        root_nodes: nodes.clone(),
        templates: collect_templates(&nodes),
        ..EmitCtx::default()
    };
    let source = emit_children(&nodes, &ctx, &mut diagnostics);
    LegacyConversionResult {
        source,
        diagnostics,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LegacyNode {
    Element(LegacyElement),
    Text(String),
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyElement {
    tag: String,
    attributes: Vec<LegacyAttribute>,
    children: Vec<LegacyNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyAttribute {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Default)]
struct TemplateRegistry {
    named: HashMap<String, LegacyElement>,
    root: Option<LegacyElement>,
    by_match: Vec<LegacyElement>,
}

struct LegacyFragmentParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> LegacyFragmentParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    fn parse(mut self) -> Vec<LegacyNode> {
        self.parse_nodes_until(None)
    }

    fn parse_nodes_until(&mut self, closing_local_name: Option<&str>) -> Vec<LegacyNode> {
        let mut nodes = Vec::new();
        while self.cursor < self.input.len() {
            if self.starts_with("<!--") {
                self.consume_comment();
                nodes.push(LegacyNode::Comment);
            } else if self.starts_with("</") {
                let closing = self.consume_closing_tag();
                if closing_local_name
                    .map(|expected| local_name(&closing).eq_ignore_ascii_case(expected))
                    .unwrap_or(true)
                {
                    break;
                }
            } else if self.starts_with("<") {
                if let Some(element) = self.consume_element() {
                    nodes.push(LegacyNode::Element(element));
                } else {
                    nodes.push(LegacyNode::Text(self.consume_text()));
                }
            } else {
                nodes.push(LegacyNode::Text(self.consume_text()));
            }
        }
        nodes
    }

    fn starts_with(&self, pattern: &str) -> bool {
        self.input[self.cursor..].starts_with(pattern)
    }

    fn consume_comment(&mut self) {
        if let Some(end) = self.input[self.cursor + 4..].find("-->") {
            self.cursor += 4 + end + 3;
        } else {
            self.cursor = self.input.len();
        }
    }

    fn consume_closing_tag(&mut self) -> String {
        self.cursor += 2;
        self.skip_ws();
        let name = self.consume_name();
        if let Some(end) = self.input[self.cursor..].find('>') {
            self.cursor += end + 1;
        } else {
            self.cursor = self.input.len();
        }
        name
    }

    fn consume_element(&mut self) -> Option<LegacyElement> {
        if !self.starts_with("<") {
            return None;
        }
        self.cursor += 1;
        self.skip_ws();
        let tag = self.consume_name();
        if tag.is_empty() {
            return None;
        }

        let mut attributes = Vec::new();
        let mut self_closing = false;
        loop {
            self.skip_ws();
            if self.cursor >= self.input.len() {
                break;
            }
            if self.starts_with("/>") {
                self.cursor += 2;
                self_closing = true;
                break;
            }
            if self.starts_with(">") {
                self.cursor += 1;
                break;
            }
            let name = self.consume_name();
            if name.is_empty() {
                self.cursor += 1;
                continue;
            }
            self.skip_ws();
            let value = if self.starts_with("=") {
                self.cursor += 1;
                self.skip_ws();
                self.consume_attribute_value()
            } else {
                String::new()
            };
            attributes.push(LegacyAttribute { name, value });
        }

        let name = local_name(&tag).to_owned();
        let html_void = !tag.contains(':') && HTML_VOID_ELEMENTS.contains(&name.as_str());
        let children = if self_closing || html_void {
            Vec::new()
        } else {
            self.parse_nodes_until(Some(&name))
        };
        Some(LegacyElement {
            tag,
            attributes,
            children,
        })
    }

    fn consume_text(&mut self) -> String {
        let end = self.input[self.cursor..]
            .find('<')
            .map(|offset| self.cursor + offset)
            .unwrap_or(self.input.len());
        let text = decode_html_entities(&self.input[self.cursor..end]);
        self.cursor = end;
        text
    }

    fn consume_name(&mut self) -> String {
        let start = self.cursor;
        while let Some(ch) = self.input[self.cursor..].chars().next() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
                self.cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        self.input[start..self.cursor].to_owned()
    }

    fn consume_attribute_value(&mut self) -> String {
        let Some(first) = self.input[self.cursor..].chars().next() else {
            return String::new();
        };
        if first == '"' || first == '\'' {
            self.cursor += first.len_utf8();
            let start = self.cursor;
            while let Some(ch) = self.input[self.cursor..].chars().next() {
                if ch == first {
                    let value = decode_html_entities(&self.input[start..self.cursor]);
                    self.cursor += ch.len_utf8();
                    return value;
                }
                self.cursor += ch.len_utf8();
            }
            return decode_html_entities(&self.input[start..]);
        }
        let start = self.cursor;
        while let Some(ch) = self.input[self.cursor..].chars().next() {
            if ch.is_whitespace() || matches!(ch, '>' | '/') {
                break;
            }
            self.cursor += ch.len_utf8();
        }
        decode_html_entities(&self.input[start..self.cursor])
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.input[self.cursor..].chars().next() {
            if ch.is_whitespace() {
                self.cursor += ch.len_utf8();
            } else {
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct EmitCtx {
    loop_var: Option<String>,
    node_sets: HashMap<String, Vec<ItemNode>>,
    current_sets: HashMap<String, Vec<CurrentItem>>,
    scalars: HashMap<String, String>,
    item: Option<CurrentItem>,
    root_nodes: Vec<LegacyNode>,
    template_depth: usize,
    templates: TemplateRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemNode {
    tag: String,
    text: String,
    attrs: HashMap<String, String>,
    children: Vec<LegacyNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentItem {
    kind: CurrentItemKind,
    tag: String,
    text: String,
    attrs: HashMap<String, String>,
    children: Vec<LegacyNode>,
    parent: Option<Box<CurrentItem>>,
    position: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentItemKind {
    Document,
    Element,
    Attribute,
    Text,
}

fn emit_children(
    nodes: &[LegacyNode],
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let scoped = with_variable_scope(nodes, ctx, diagnostics);
    nodes
        .iter()
        .map(|node| emit_node(node, &scoped, diagnostics))
        .collect()
}

fn collect_templates(nodes: &[LegacyNode]) -> TemplateRegistry {
    let mut registry = TemplateRegistry::default();
    collect_templates_from_nodes(nodes, &mut registry);
    registry
}

fn collect_templates_from_nodes(nodes: &[LegacyNode], registry: &mut TemplateRegistry) {
    for node in nodes {
        let LegacyNode::Element(element) = node else {
            continue;
        };
        let name = local_name(&element.tag);
        if name == "template" && is_xslt_element(&element.tag) {
            if let Some(template_name) = attr_value(element, "name") {
                registry
                    .named
                    .insert(template_name.to_owned(), element.clone());
            }
            if attr_value(element, "match") == Some("/") {
                registry.root = Some(element.clone());
            } else if attr_value(element, "match").is_some() {
                registry.by_match.push(element.clone());
            }
            continue;
        }
        collect_templates_from_nodes(&element.children, registry);
    }
}

fn with_variable_scope(
    nodes: &[LegacyNode],
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> EmitCtx {
    let mut scoped = ctx.clone();
    for node in nodes {
        let LegacyNode::Element(element) = node else {
            continue;
        };
        if local_name(&element.tag) != "variable" {
            continue;
        }
        let Some(name) = attr_value(element, "name") else {
            continue;
        };
        let members: Vec<ItemNode> = element
            .children
            .iter()
            .filter_map(|child| match child {
                LegacyNode::Element(member) => Some(to_item_node(member)),
                _ => None,
            })
            .collect();
        if let Some(select) = attr_value(element, "select") {
            let source = scoped.clone();
            bind_select_value(&mut scoped, name, select, &source, diagnostics);
        } else if !members.is_empty() {
            scoped.node_sets.insert(name.to_owned(), members);
        }
    }
    scoped
}

fn bind_select_value(
    scoped: &mut EmitCtx,
    name: &str,
    select: &str,
    source: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) {
    let select = select.trim();
    if select != "." {
        if let Some(value) = source
            .item
            .as_ref()
            .and_then(|item| resolve_item_literal(select, item))
            .or_else(|| evaluate_xpath_literal(select, source))
        {
            scoped.scalars.insert(name.to_owned(), value);
            return;
        }
        if is_quoted_xpath_literal(select) {
            scoped.scalars.insert(
                name.to_owned(),
                format!("\"{}\"", unquote_xpath_literal(select).replace('"', "\\\"")),
            );
            return;
        }
    }
    if let Some(members) = select_apply_members(select, source).filter(|members| {
        !members.is_empty()
            || select.starts_with("exsl:node-set(")
            || select.starts_with("exslt:node-set(")
            || select.starts_with("node-set(")
    }) {
        bind_apply_members(scoped, name, members);
        return;
    }
    if let Some(items) = select_variable_current_items(select, source) {
        bind_current_items(scoped, name, items);
        return;
    }
    let rewritten = source
        .item
        .as_ref()
        .and_then(|item| resolve_item_literal(select, item))
        .or_else(|| evaluate_xpath_literal(select, source))
        .unwrap_or_else(|| rewrite_expression(select, source, false, diagnostics));
    scoped.scalars.insert(name.to_owned(), rewritten);
}

fn bind_apply_members(scoped: &mut EmitCtx, name: &str, members: Vec<ApplyMember>) {
    let current_items: Vec<CurrentItem> = members
        .iter()
        .filter_map(current_item_from_apply_member)
        .collect();
    bind_current_items(scoped, name, current_items);
}

fn bind_current_items(scoped: &mut EmitCtx, name: &str, current_items: Vec<CurrentItem>) {
    let item_nodes: Vec<ItemNode> = current_items
        .iter()
        .filter(|item| item.kind == CurrentItemKind::Element)
        .map(item_node_from_current_item)
        .collect();
    if !item_nodes.is_empty() {
        scoped.node_sets.insert(name.to_owned(), item_nodes);
    }
    if !current_items.is_empty() {
        let scalar = current_items
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>()
            .join("");
        scoped.current_sets.insert(name.to_owned(), current_items);
        scoped.scalars.insert(name.to_owned(), scalar);
    }
}

fn to_item_node(element: &LegacyElement) -> ItemNode {
    let mut attrs = HashMap::new();
    for attr in &element.attributes {
        attrs.insert(attr.name.clone(), attr.value.clone());
    }
    ItemNode {
        tag: local_name(&element.tag).to_owned(),
        text: text_content(element),
        attrs,
        children: element.children.clone(),
    }
}

fn emit_node(
    node: &LegacyNode,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    match node {
        LegacyNode::Text(text) => interpolate(text, ctx, diagnostics),
        LegacyNode::Comment => String::new(),
        LegacyNode::Element(element) => emit_element(element, ctx, diagnostics),
    }
}

fn emit_element(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let name = local_name(&element.tag);
    match name {
        "value-of" => return emit_value_of(element, ctx, diagnostics),
        "text" => return emit_xsl_text(element),
        "if" => return emit_if(element, ctx, diagnostics),
        "choose" => return emit_choose(element, ctx, diagnostics),
        "when" | "otherwise" => {
            diagnostics.push(diag(
                "legacy_xslt.orphan_branch",
                format!("<{}> outside <choose> is ignored", element.tag),
            ));
            return String::new();
        }
        "for-each" => return emit_for_each(element, ctx, diagnostics),
        "stylesheet" => return emit_stylesheet(element, ctx, diagnostics),
        "template" if is_xslt_element(&element.tag) => {
            return emit_template(element, ctx, diagnostics)
        }
        "call-template" => return emit_call_template(element, ctx, diagnostics),
        "apply-templates" => return emit_apply_templates(element, ctx, diagnostics),
        "copy" if is_xslt_element(&element.tag) => return emit_xsl_copy(element, ctx, diagnostics),
        "copy-of" if is_xslt_element(&element.tag) => {
            return emit_xsl_copy_of(element, ctx, diagnostics)
        }
        "element" if is_xslt_element(&element.tag) => {
            return emit_xsl_element(element, ctx, diagnostics)
        }
        "attribute" if is_xslt_element(&element.tag) => {
            diagnostics.push(diag(
                "legacy_xslt.attribute_outside_element",
                "<xsl:attribute> outside an emitted element is ignored",
            ));
            return String::new();
        }
        "output" if is_xslt_element(&element.tag) => return String::new(),
        "param" | "with-param" => return String::new(),
        "variable" => return String::new(),
        "slot" => return emit_slot(element, ctx, diagnostics),
        _ => {}
    }

    if element_disposition(name) == LegacyElementDisposition::Tier3Handoff
        && (is_xslt_element(&element.tag) || name == "function")
    {
        diagnostics.push(diag(
            UNSUPPORTED_CONSTRUCT_CODE,
            format!(
                "<{}> (Tier 3 / non-transpilable) is not converted",
                element.tag
            ),
        ));
        return String::new();
    }

    let tag = if element.tag.starts_with("xhtml:") {
        name
    } else {
        element.tag.as_str()
    };
    emit_generic_element(element, ctx, diagnostics, tag)
}

fn emit_generic_element(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
    tag: &str,
) -> String {
    let mut attrs = element
        .attributes
        .iter()
        .map(|attr| emit_attribute(attr, ctx, diagnostics))
        .collect::<String>();
    attrs.push_str(&emit_xsl_instruction_attributes(
        &element.children,
        ctx,
        diagnostics,
    ));
    let body = if local_name(tag) == "style" {
        emit_rich_content(&text_content(element))
    } else {
        emit_children_excluding_instruction_attributes(&element.children, ctx, diagnostics)
    };
    if body.is_empty() {
        format!("{{{tag}{attrs}}}")
    } else {
        format!("{{{tag}{attrs} | {body}}}")
    }
}

fn emit_children_excluding_instruction_attributes(
    nodes: &[LegacyNode],
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    nodes
        .iter()
        .filter(|node| !is_xsl_instruction_attribute_node(node))
        .map(|node| emit_node(node, ctx, diagnostics))
        .collect()
}

fn is_xsl_instruction_attribute_node(node: &LegacyNode) -> bool {
    matches!(
        node,
        LegacyNode::Element(element)
            if is_xslt_element(&element.tag)
                && (local_name(&element.tag) == "attribute"
                    || (local_name(&element.tag) == "copy-of"
                        && attr_value(element, "select") == Some("@*")))
    )
}

fn emit_xsl_instruction_attributes(
    nodes: &[LegacyNode],
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let mut attrs = String::new();
    for node in nodes {
        let LegacyNode::Element(element) = node else {
            continue;
        };
        if !is_xslt_element(&element.tag) {
            continue;
        }
        match local_name(&element.tag) {
            "attribute" => attrs.push_str(&emit_xsl_attribute(element, ctx, diagnostics)),
            "copy-of" if attr_value(element, "select") == Some("@*") => {
                if let Some(current) = &ctx.item {
                    attrs.push_str(&attrs_from_current(current));
                }
            }
            _ => {}
        }
    }
    attrs
}

fn emit_xsl_attribute(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let Some(name) = attr_value(element, "name") else {
        diagnostics.push(diag(
            "legacy_xslt.attribute_missing_name",
            "<xsl:attribute> without @name is ignored",
        ));
        return String::new();
    };
    if name.contains('{') || name.contains('}') || !is_name(name) {
        diagnostics.push(diag(
            UNSUPPORTED_CONSTRUCT_CODE,
            format!("<xsl:attribute name=\"{name}\"> has a dynamic or invalid name"),
        ));
        return String::new();
    }
    let value = if let Some(select) = attr_value(element, "select") {
        if let Some(item) = &ctx.item {
            resolve_item_literal(select.trim(), item).unwrap_or_else(|| {
                format!("{{{}}}", rewrite_expression(select, ctx, true, diagnostics))
            })
        } else {
            format!("{{{}}}", rewrite_expression(select, ctx, true, diagnostics))
        }
    } else {
        emit_children(&element.children, ctx, diagnostics)
    };
    attr_assign(name, &value)
}

fn emit_xsl_element(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let Some(name) = attr_value(element, "name") else {
        diagnostics.push(diag(
            "legacy_xslt.element_missing_name",
            "<xsl:element> without @name is ignored",
        ));
        return String::new();
    };
    let Some(tag) = resolve_constructed_name(name, ctx, diagnostics) else {
        return String::new();
    };
    let attrs = emit_xsl_instruction_attributes(&element.children, ctx, diagnostics);
    let body = emit_children_excluding_instruction_attributes(&element.children, ctx, diagnostics);
    if body.is_empty() {
        format!("{{{tag}{attrs}}}")
    } else {
        format!("{{{tag}{attrs} | {body}}}")
    }
}

fn resolve_constructed_name(
    value: &str,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> Option<String> {
    let value = value.trim();
    if is_name(value) {
        return Some(local_name(value).to_owned());
    }
    let Some(inner) = value
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        diagnostics.push(diag(
            UNSUPPORTED_CONSTRUCT_CODE,
            format!("<xsl:element name=\"{value}\"> has a dynamic name outside the bounded subset"),
        ));
        return None;
    };
    let inner = inner.trim();
    let resolved = if let Some(name) = inner.strip_prefix('$') {
        ctx.scalars.get(name).cloned()
    } else {
        ctx.item
            .as_ref()
            .and_then(|item| resolve_item_literal(inner, item))
    };
    let Some(resolved) = resolved else {
        diagnostics.push(diag(
            UNSUPPORTED_CONSTRUCT_CODE,
            format!("<xsl:element name=\"{value}\"> could not be resolved statically"),
        ));
        return None;
    };
    let resolved = resolved.trim().trim_matches('"').trim_matches('\'');
    if !is_name(resolved) {
        diagnostics.push(diag(
            UNSUPPORTED_CONSTRUCT_CODE,
            format!("<xsl:element name=\"{value}\"> resolved to invalid name \"{resolved}\""),
        ));
        return None;
    }
    Some(local_name(resolved).to_owned())
}

fn emit_attribute(
    attr: &LegacyAttribute,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    if attr.name == "xmlns" || attr.name.starts_with("xmlns:") {
        return String::new();
    }
    attr_assign(&attr.name, &interpolate(&attr.value, ctx, diagnostics))
}

fn emit_value_of(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let Some(select) = attr_value(element, "select") else {
        diagnostics.push(diag(
            "legacy_xslt.value_of_missing_select",
            "<value-of> without @select is ignored",
        ));
        return String::new();
    };
    if let Some(item) = &ctx.item {
        if let Some(literal) = resolve_item_literal(select.trim(), item) {
            return escape_literal(&literal);
        }
    }
    if let Some(name) = select.trim().strip_prefix('$') {
        if let Some(value) = ctx.scalars.get(name) {
            return escape_literal(value);
        }
    }
    if let Some(value) = evaluate_xpath_literal(select.trim(), ctx) {
        return escape_literal(&value);
    }
    format!("{{{}}}", rewrite_expression(select, ctx, true, diagnostics))
}

fn emit_xsl_text(element: &LegacyElement) -> String {
    escape_literal(&text_content(element))
}

fn emit_if(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let Some(test) = attr_value(element, "test") else {
        diagnostics.push(diag(
            "legacy_xslt.if_missing_test",
            "<if> without @test is ignored",
        ));
        return String::new();
    };
    if let Some(value) = evaluate_xpath_bool(test, ctx) {
        if value {
            return emit_children(&element.children, ctx, diagnostics);
        }
        return String::new();
    }
    let body = emit_children(&element.children, ctx, diagnostics);
    format!(
        "{{cem:if{} | {body}}}",
        expr_attr("test", &rewrite_expression(test, ctx, false, diagnostics))
    )
}

fn emit_choose(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let mut branches = String::new();
    let mut only_static_false = true;
    for child in &element.children {
        let LegacyNode::Element(branch) = child else {
            continue;
        };
        match local_name(&branch.tag) {
            "when" => {
                let Some(test) = attr_value(branch, "test") else {
                    diagnostics.push(diag(
                        "legacy_xslt.when_missing_test",
                        "<when> without @test is ignored",
                    ));
                    continue;
                };
                if let Some(value) = evaluate_xpath_bool(test, ctx) {
                    if value && only_static_false && branches.is_empty() {
                        return emit_children(&branch.children, ctx, diagnostics);
                    }
                    if !value {
                        continue;
                    }
                }
                only_static_false = false;
                let body = emit_children(&branch.children, ctx, diagnostics);
                branches.push_str(&format!(
                    "{{cem:when{} | {body}}}",
                    expr_attr("test", &rewrite_expression(test, ctx, false, diagnostics))
                ));
            }
            "otherwise" => {
                if only_static_false && branches.is_empty() {
                    return emit_children(&branch.children, ctx, diagnostics);
                }
                let body = emit_children(&branch.children, ctx, diagnostics);
                branches.push_str(&format!("{{cem:otherwise | {body}}}"));
            }
            _ => {}
        }
    }
    format!("{{cem:choose | {branches}}}")
}

fn emit_for_each(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let Some(select) = attr_value(element, "select") else {
        diagnostics.push(diag(
            "legacy_xslt.for_each_missing_select",
            "<for-each> without @select is ignored",
        ));
        return String::new();
    };

    if let Some(node_set_ref) = match_node_set_select(select, ctx) {
        let members = ctx
            .node_sets
            .get(&node_set_ref.name)
            .cloned()
            .unwrap_or_default();
        let predicate = node_set_ref
            .predicate
            .as_deref()
            .map(|predicate| rewrite_predicate(predicate, ctx, diagnostics));
        return members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                let item_ctx = EmitCtx {
                    item: Some(current_item_from_item_node(member, index + 1)),
                    loop_var: None,
                    ..ctx.clone()
                };
                let body = emit_children(&element.children, &item_ctx, diagnostics);
                if let Some(predicate) = &predicate {
                    format!("{{cem:if{} | {body}}}", expr_attr("test", predicate))
                } else {
                    body
                }
            })
            .collect();
    }

    if let Some(mut members) = select_apply_members(select, ctx) {
        apply_sort_children(&mut members, element);
        return members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                let current = match member {
                    ApplyMember::Item(member) => current_item_from_item_node(member, index + 1),
                    ApplyMember::Current(member) => CurrentItem {
                        position: index + 1,
                        ..member.clone()
                    },
                };
                let item_ctx = EmitCtx {
                    item: Some(current),
                    loop_var: None,
                    ..ctx.clone()
                };
                emit_children(&element.children, &item_ctx, diagnostics)
            })
            .collect();
    }

    let loop_var = "item".to_owned();
    let child_ctx = EmitCtx {
        loop_var: Some(loop_var.clone()),
        item: None,
        ..ctx.clone()
    };
    let body = emit_children(&element.children, &child_ctx, diagnostics);
    format!(
        "{{cem:for-each{} @as=\"{loop_var}\" | {body}}}",
        expr_attr(
            "select",
            &rewrite_expression(select, ctx, false, diagnostics)
        )
    )
}

fn emit_stylesheet(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let scoped = with_variable_scope(&element.children, ctx, diagnostics);
    if let Some(root) = &ctx.templates.root {
        let root_ctx = EmitCtx {
            item: Some(CurrentItem {
                kind: CurrentItemKind::Document,
                tag: "#document".to_owned(),
                text: String::new(),
                attrs: HashMap::new(),
                children: source_document_nodes(&ctx.root_nodes, element),
                parent: None,
                position: 1,
            }),
            ..scoped
        };
        return emit_children(&root.children, &root_ctx, diagnostics);
    }
    element
        .children
        .iter()
        .filter_map(|child| match child {
            LegacyNode::Element(child) if local_name(&child.tag) == "template" => None,
            other => Some(emit_node(other, &scoped, diagnostics)),
        })
        .collect()
}

fn emit_template(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    if attr_value(element, "name").is_some() || attr_value(element, "match").is_some() {
        return String::new();
    }
    emit_children(&element.children, ctx, diagnostics)
}

fn emit_call_template(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    if ctx.template_depth >= MAX_TEMPLATE_DEPTH {
        diagnostics.push(diag(
            "legacy_xslt.template_recursion_limit",
            format!("template recursion exceeded the bounded limit of {MAX_TEMPLATE_DEPTH}"),
        ));
        return String::new();
    }
    let Some(name) = attr_value(element, "name") else {
        diagnostics.push(diag(
            "legacy_xslt.call_template_missing_name",
            "<call-template> without @name is ignored",
        ));
        return String::new();
    };
    let Some(template) = ctx.templates.named.get(name) else {
        diagnostics.push(diag(
            "legacy_xslt.call_template_missing_target",
            format!("<call-template name=\"{name}\"> target was not found"),
        ));
        return String::new();
    };
    let mut scoped = with_call_template_params(element, ctx, diagnostics);
    scoped = with_template_param_defaults(&template.children, &scoped, diagnostics);
    scoped.template_depth += 1;
    emit_children(&template.children, &scoped, diagnostics)
}

fn with_call_template_params(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> EmitCtx {
    let mut scoped = ctx.clone();
    for child in &element.children {
        let LegacyNode::Element(param) = child else {
            continue;
        };
        if local_name(&param.tag) != "with-param" {
            continue;
        }
        let Some(name) = attr_value(param, "name") else {
            continue;
        };
        if let Some(select) = attr_value(param, "select") {
            let source = scoped.clone();
            bind_select_value(&mut scoped, name, select, &source, diagnostics);
        } else {
            let value = emit_children(&param.children, ctx, diagnostics);
            scoped.scalars.insert(name.to_owned(), value);
        }
    }
    scoped
}

fn with_template_param_defaults(
    nodes: &[LegacyNode],
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> EmitCtx {
    let mut scoped = ctx.clone();
    for node in nodes {
        let LegacyNode::Element(param) = node else {
            continue;
        };
        if local_name(&param.tag) != "param" {
            continue;
        }
        let Some(name) = attr_value(param, "name") else {
            continue;
        };
        if scoped.scalars.contains_key(name)
            || scoped.node_sets.contains_key(name)
            || scoped.current_sets.contains_key(name)
        {
            continue;
        }
        if let Some(select) = attr_value(param, "select") {
            let source = scoped.clone();
            bind_select_value(&mut scoped, name, select, &source, diagnostics);
        } else if !param.children.is_empty() {
            let value = emit_children(&param.children, &scoped, diagnostics);
            scoped.scalars.insert(name.to_owned(), value);
        } else {
            scoped.scalars.insert(name.to_owned(), String::new());
        }
    }
    scoped
}

fn emit_apply_templates(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let select = attr_value(element, "select").unwrap_or("*");
    if let Some(mut members) = select_apply_members(select, ctx) {
        apply_sort_children(&mut members, element);
        return members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                emit_apply_template_member(member, index + 1, element, ctx, diagnostics)
            })
            .collect();
    }
    diagnostics.push(diag(
        UNSUPPORTED_CONSTRUCT_CODE,
        format!(
            "<{} select=\"{}\"> is outside the bounded apply-templates subset",
            element.tag, select
        ),
    ));
    String::new()
}

fn emit_xsl_copy(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let Some(current) = &ctx.item else {
        diagnostics.push(diag(
            "legacy_xslt.copy_without_current_item",
            "<xsl:copy> without a current node is ignored",
        ));
        return String::new();
    };
    match current.kind {
        CurrentItemKind::Document => emit_children(&current.children, ctx, diagnostics),
        CurrentItemKind::Attribute | CurrentItemKind::Text => escape_literal(&current.text),
        CurrentItemKind::Element => {
            let attrs = emit_xsl_instruction_attributes(&element.children, ctx, diagnostics);
            let body =
                emit_children_excluding_instruction_attributes(&element.children, ctx, diagnostics);
            if body.is_empty() {
                format!("{{{}{attrs}}}", current.tag)
            } else {
                format!("{{{}{attrs} | {body}}}", current.tag)
            }
        }
    }
}

fn emit_xsl_copy_of(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let Some(select) = attr_value(element, "select") else {
        diagnostics.push(diag(
            "legacy_xslt.copy_of_missing_select",
            "<xsl:copy-of> without @select is ignored",
        ));
        return String::new();
    };
    let select = select.trim();
    if let Some(name) = select.strip_prefix('$') {
        if let Some(nodes) = ctx.node_sets.get(name) {
            return nodes
                .iter()
                .map(|node| serialize_item_node_to_cem(node, ctx, diagnostics))
                .collect();
        }
        if let Some(value) = ctx.scalars.get(name) {
            return escape_literal(value);
        }
    }
    if select == "." {
        if let Some(current) = &ctx.item {
            return serialize_current_item_to_cem(current, ctx, diagnostics);
        }
    }
    if let Some(members) = select_current_members(select, ctx) {
        return members
            .iter()
            .map(|member| match member {
                ApplyMember::Item(item) => serialize_item_node_to_cem(item, ctx, diagnostics),
                ApplyMember::Current(item) => serialize_current_item_to_cem(item, ctx, diagnostics),
            })
            .collect();
    }
    diagnostics.push(diag(
        UNSUPPORTED_CONSTRUCT_CODE,
        format!("<xsl:copy-of select=\"{select}\"> is outside the bounded copy subset"),
    ));
    String::new()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ApplyMember {
    Item(ItemNode),
    Current(CurrentItem),
}

fn select_apply_members(select: &str, ctx: &EmitCtx) -> Option<Vec<ApplyMember>> {
    if let Some(members) = select_exsl_node_set_members(select, ctx) {
        return Some(members);
    }

    if let Some(node_set_ref) = match_node_set_select(select, ctx) {
        let members = ctx
            .node_sets
            .get(&node_set_ref.name)
            .cloned()
            .unwrap_or_default();
        return Some(members.into_iter().map(ApplyMember::Item).collect());
    }

    let mut out = Vec::new();
    for part in select.split('|') {
        out.extend(select_current_members(part.trim(), ctx)?);
    }
    Some(out)
}

fn emit_apply_template_member(
    member: &ApplyMember,
    position: usize,
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let scoped = with_call_template_params(element, ctx, diagnostics);
    emit_apply_template_member_with_mode(
        member,
        position,
        attr_value(element, "mode"),
        &scoped,
        diagnostics,
    )
}

fn find_matching_template<'a>(
    member: &CurrentItem,
    mode: Option<&str>,
    templates: &'a TemplateRegistry,
) -> Option<&'a LegacyElement> {
    templates
        .by_match
        .iter()
        .enumerate()
        .filter(|(_, template)| {
            if attr_value(template, "mode") != mode {
                return false;
            }
            let Some(pattern) = attr_value(template, "match") else {
                return false;
            };
            matches_item_pattern(pattern, member)
        })
        .max_by_key(|(index, template)| {
            let priority = attr_value(template, "priority")
                .and_then(|value| value.trim().parse::<f64>().ok())
                .unwrap_or_else(|| {
                    default_template_priority(attr_value(template, "match").unwrap_or(""))
                });
            ((priority * 1000.0) as i64, *index as i64)
        })
        .map(|(_, template)| template)
}

fn matches_item_pattern(pattern: &str, member: &CurrentItem) -> bool {
    pattern
        .split('|')
        .any(|part| matches_single_item_pattern(part.trim(), member))
}

fn matches_single_item_pattern(pattern: &str, member: &CurrentItem) -> bool {
    match member.kind {
        CurrentItemKind::Document => pattern == "/",
        CurrentItemKind::Attribute => pattern == "@*" || pattern == "node()" || pattern == ".",
        CurrentItemKind::Text => pattern == "text()" || pattern == "node()" || pattern == ".",
        CurrentItemKind::Element => {
            pattern == "*"
                || pattern == "node()"
                || pattern == "."
                || pattern == member.tag
                || member.attrs.get("name").map(String::as_str) == Some(pattern)
                || matches_simple_predicate_pattern(pattern, member)
        }
    }
}

fn matches_simple_predicate_pattern(pattern: &str, member: &CurrentItem) -> bool {
    if !pattern.starts_with("*[") || !pattern.ends_with(']') {
        return false;
    }
    let predicate = pattern[2..pattern.len() - 1].trim();
    if predicate == "*" {
        return member
            .children
            .iter()
            .any(|child| matches!(child, LegacyNode::Element(_)));
    }
    if predicate == "not(*)" {
        return !member
            .children
            .iter()
            .any(|child| matches!(child, LegacyNode::Element(_)));
    }
    let Some(rest) = predicate.strip_prefix('@') else {
        return false;
    };
    let Some((name, value)) = rest.split_once('=') else {
        return member.attrs.contains_key(rest);
    };
    let expected = value.trim().trim_matches('"').trim_matches('\'');
    member.attrs.get(name.trim()).map(String::as_str) == Some(expected)
}

fn default_template_priority(pattern: &str) -> f64 {
    let pattern = pattern.trim();
    if pattern == "*" || pattern == "@*" || pattern == "node()" || pattern == "text()" {
        -0.5
    } else if pattern.contains('|') {
        0.0
    } else {
        0.5
    }
}

fn current_item_from_item_node(member: &ItemNode, position: usize) -> CurrentItem {
    CurrentItem {
        kind: CurrentItemKind::Element,
        tag: member.tag.clone(),
        text: member.text.clone(),
        attrs: member.attrs.clone(),
        children: member.children.clone(),
        parent: None,
        position,
    }
}

fn source_document_nodes(nodes: &[LegacyNode], stylesheet: &LegacyElement) -> Vec<LegacyNode> {
    nodes
        .iter()
        .filter(|node| match node {
            LegacyNode::Element(element) => element != stylesheet,
            _ => true,
        })
        .cloned()
        .collect()
}

fn select_current_members(select: &str, ctx: &EmitCtx) -> Option<Vec<ApplyMember>> {
    let current = ctx.item.as_ref()?;
    match select {
        "." => Some(vec![ApplyMember::Current(current.clone())]),
        ".." => current
            .parent
            .as_ref()
            .map(|parent| vec![ApplyMember::Current((**parent).clone())]),
        "*" => Some(element_children_with_parent(&current.children, current)),
        "@*" => Some(attribute_children(current)),
        "text()" | "./text()" => Some(text_children_with_parent(&current.children, current)),
        _ if select.starts_with("//") || select.starts_with('/') => {
            select_absolute_members(select, ctx)
        }
        _ if select.starts_with('$') => select_variable_path_members(select, ctx),
        _ if select.starts_with("preceding-sibling::") => {
            select_preceding_sibling_members(select, current, ctx)
        }
        _ if select.starts_with("../") => select_parent_path_members(select, current, ctx),
        _ if select.ends_with("/*") => select_current_path_members(select, current, ctx),
        _ => select_child_path_members(select, current, ctx),
    }
}

fn emit_default_template_rule(
    current: &CurrentItem,
    mode: Option<&str>,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    match current.kind {
        CurrentItemKind::Attribute | CurrentItemKind::Text => current.text.clone(),
        CurrentItemKind::Document | CurrentItemKind::Element => {
            if ctx.template_depth >= MAX_TEMPLATE_DEPTH {
                diagnostics.push(diag(
                    "legacy_xslt.template_recursion_limit",
                    format!(
                        "template recursion exceeded the bounded limit of {MAX_TEMPLATE_DEPTH}"
                    ),
                ));
                return String::new();
            }
            let mut members = element_children_with_parent(&current.children, current);
            members.extend(text_children_with_parent(&current.children, current));
            let child_ctx = EmitCtx {
                template_depth: ctx.template_depth + 1,
                ..ctx.clone()
            };
            members
                .iter()
                .enumerate()
                .map(|(index, member)| {
                    emit_apply_template_member_with_mode(
                        member,
                        index + 1,
                        mode,
                        &child_ctx,
                        diagnostics,
                    )
                })
                .collect()
        }
    }
}

fn emit_apply_template_member_with_mode(
    member: &ApplyMember,
    position: usize,
    mode: Option<&str>,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let current = match member {
        ApplyMember::Item(member) => current_item_from_item_node(member, position),
        ApplyMember::Current(member) => CurrentItem {
            position,
            ..member.clone()
        },
    };
    let item_ctx = EmitCtx {
        item: Some(current.clone()),
        loop_var: None,
        ..ctx.clone()
    };
    if let Some(template) = find_matching_template(&current, mode, &ctx.templates) {
        if item_ctx.template_depth >= MAX_TEMPLATE_DEPTH {
            diagnostics.push(diag(
                "legacy_xslt.template_recursion_limit",
                format!("template recursion exceeded the bounded limit of {MAX_TEMPLATE_DEPTH}"),
            ));
            return String::new();
        }
        let template_ctx = EmitCtx {
            template_depth: item_ctx.template_depth + 1,
            ..item_ctx
        };
        let template_ctx =
            with_template_param_defaults(&template.children, &template_ctx, diagnostics);
        return emit_children(&template.children, &template_ctx, diagnostics);
    }
    emit_default_template_rule(&current, mode, &item_ctx, diagnostics)
}

fn select_current_path_members(
    select: &str,
    current: &CurrentItem,
    ctx: &EmitCtx,
) -> Option<Vec<ApplyMember>> {
    let base = select.strip_suffix("/*")?;
    let base = base.strip_prefix("./").unwrap_or(base);
    let children = element_children_with_parent(&current.children, current);
    if base.is_empty() || base == "." {
        return Some(children);
    }
    let mut out = Vec::new();
    for member in children {
        let ApplyMember::Current(item) = member else {
            continue;
        };
        if item.tag == base {
            out.extend(filter_members_by_step(
                element_children_with_parent(&item.children, &item),
                "*",
                ctx,
            ));
        }
    }
    Some(out)
}

fn select_parent_path_members(
    select: &str,
    current: &CurrentItem,
    ctx: &EmitCtx,
) -> Option<Vec<ApplyMember>> {
    let parent = current.parent.as_ref()?;
    let rest = select.strip_prefix("../")?;
    match rest {
        ".." => parent
            .parent
            .as_ref()
            .map(|grandparent| vec![ApplyMember::Current((**grandparent).clone())]),
        "*" => Some(element_children_with_parent(&parent.children, parent)),
        "@*" => Some(attribute_children(parent)),
        _ if rest.starts_with("@") => Some(attribute_named(parent, &rest[1..])),
        _ => Some(filter_members_by_step(
            element_children_with_parent(&parent.children, parent),
            rest,
            ctx,
        )),
    }
}

fn select_child_path_members(
    select: &str,
    current: &CurrentItem,
    ctx: &EmitCtx,
) -> Option<Vec<ApplyMember>> {
    let select = select.strip_prefix("./").unwrap_or(select);
    if let Some(attr_name) = select.strip_prefix('@') {
        return Some(attribute_named(current, attr_name));
    }
    let mut members = vec![ApplyMember::Current(current.clone())];
    for step in select.split('/').filter(|part| !part.is_empty()) {
        let mut next = Vec::new();
        for member in members {
            let ApplyMember::Current(item) = member else {
                continue;
            };
            if step == "text()" {
                next.extend(text_children_with_parent(&item.children, &item));
            } else if step == "@*" {
                next.extend(attribute_children(&item));
            } else if let Some(attr_name) = step.strip_prefix('@') {
                next.extend(attribute_named(&item, attr_name));
            } else {
                next.extend(filter_members_by_step(
                    element_children_with_parent(&item.children, &item),
                    step,
                    ctx,
                ));
            }
        }
        members = next;
    }
    Some(members)
}

fn select_variable_current_items(select: &str, ctx: &EmitCtx) -> Option<Vec<CurrentItem>> {
    let select = select.trim();
    if !looks_like_node_variable_select(select, ctx) {
        return None;
    }
    let members = select_current_members(select, ctx)?;
    let items: Vec<CurrentItem> = members
        .into_iter()
        .filter_map(|member| match member {
            ApplyMember::Current(item) => Some(item),
            ApplyMember::Item(item) => Some(current_item_from_item_node(&item, 1)),
        })
        .collect();
    if items.is_empty() && select != "." {
        None
    } else {
        Some(items)
    }
}

fn select_exsl_node_set_members(select: &str, ctx: &EmitCtx) -> Option<Vec<ApplyMember>> {
    let trimmed = select.trim();
    let inner = trimmed
        .strip_prefix("exsl:node-set(")
        .or_else(|| trimmed.strip_prefix("exslt:node-set("))
        .or_else(|| trimmed.strip_prefix("node-set("))?;
    let close = inner.find(')')?;
    let var = inner[..close].trim().strip_prefix('$')?;
    let mut members: Vec<ApplyMember> = if let Some(nodes) = ctx.current_sets.get(var) {
        nodes.iter().cloned().map(ApplyMember::Current).collect()
    } else if let Some(nodes) = ctx.node_sets.get(var) {
        nodes.iter().cloned().map(ApplyMember::Item).collect()
    } else {
        return None;
    };
    let rest = inner[close + 1..].trim();
    if rest.is_empty() {
        return Some(members);
    }
    let mut path = rest.strip_prefix('/')?;
    if let Some(attr_name) = path.strip_prefix("@") {
        return Some(select_attributes_from_members(members, attr_name, ctx));
    }
    let (step, attr_name) = if let Some((step, attr_name)) = path.rsplit_once("/@") {
        (step, Some(attr_name))
    } else {
        (path, None)
    };
    path = step;
    members = filter_any_members_by_step(members, path, ctx);
    if let Some(attr_name) = attr_name {
        return Some(select_attributes_from_members(members, attr_name, ctx));
    }
    Some(members)
}

fn select_attributes_from_members(
    members: Vec<ApplyMember>,
    attr_name: &str,
    _ctx: &EmitCtx,
) -> Vec<ApplyMember> {
    members
        .into_iter()
        .filter_map(|member| current_item_from_apply_member(&member))
        .flat_map(|item| {
            if attr_name == "*" {
                attribute_children(&item)
            } else {
                attribute_named(&item, attr_name)
            }
        })
        .collect()
}

fn current_item_from_apply_member(member: &ApplyMember) -> Option<CurrentItem> {
    match member {
        ApplyMember::Current(item) => Some(item.clone()),
        ApplyMember::Item(item) => Some(current_item_from_item_node(item, 1)),
    }
}

fn item_node_from_current_item(item: &CurrentItem) -> ItemNode {
    ItemNode {
        tag: item.tag.clone(),
        text: item.text.clone(),
        attrs: item.attrs.clone(),
        children: item.children.clone(),
    }
}

fn looks_like_node_variable_select(select: &str, ctx: &EmitCtx) -> bool {
    if select.contains('=')
        || select.contains("!=")
        || select.contains("&gt;")
        || select.contains("&lt;")
        || select.contains('>')
        || select.contains('<')
    {
        return false;
    }
    if matches!(select, "." | ".." | "*" | "@*") {
        return true;
    }
    if select.starts_with("./")
        || select.starts_with("../")
        || select.starts_with("//")
        || select.starts_with('/')
        || select.contains('|')
        || select.starts_with("preceding-sibling::")
    {
        return true;
    }
    select.strip_prefix('$').is_some_and(|rest| {
        let name = rest
            .split_once('/')
            .map(|(name, _)| name)
            .unwrap_or(rest)
            .trim();
        ctx.current_sets.contains_key(name) || ctx.node_sets.contains_key(name)
    })
}

fn select_variable_path_members(select: &str, ctx: &EmitCtx) -> Option<Vec<ApplyMember>> {
    let rest = select.trim().strip_prefix('$')?;
    let (name, path) = rest
        .split_once('/')
        .map(|(name, path)| (name.trim(), Some(path)))
        .unwrap_or((rest.trim(), None));
    if !is_name(name) {
        return None;
    }
    let mut members: Vec<ApplyMember> = if let Some(items) = ctx.current_sets.get(name) {
        items.iter().cloned().map(ApplyMember::Current).collect()
    } else if let Some(items) = ctx.node_sets.get(name) {
        items.iter().cloned().map(ApplyMember::Item).collect()
    } else {
        return None;
    };
    let Some(path) = path else {
        return Some(members);
    };
    for step in path.split('/').filter(|part| !part.is_empty()) {
        let mut next = Vec::new();
        for member in members {
            let current = match member {
                ApplyMember::Current(item) => item,
                ApplyMember::Item(item) => current_item_from_item_node(&item, 1),
            };
            next.extend(select_step_from_current(&current, step, ctx));
        }
        members = next;
    }
    Some(members)
}

fn select_step_from_current(current: &CurrentItem, step: &str, ctx: &EmitCtx) -> Vec<ApplyMember> {
    if step == "text()" {
        return text_children_with_parent(&current.children, current);
    }
    if step == "@*" || step.starts_with("@*[") {
        return filter_any_members_by_step(attribute_children(current), step, ctx);
    }
    if let Some(attr_name) = step.strip_prefix('@') {
        return attribute_named(current, attr_name);
    }
    filter_members_by_step(
        element_children_with_parent(&current.children, current),
        step,
        ctx,
    )
}

fn select_preceding_sibling_members(
    select: &str,
    current: &CurrentItem,
    ctx: &EmitCtx,
) -> Option<Vec<ApplyMember>> {
    let step = select.strip_prefix("preceding-sibling::")?;
    let parent = current.parent.as_ref()?;
    let mut out = Vec::new();
    for sibling in element_children_with_parent(&parent.children, parent) {
        let ApplyMember::Current(item) = sibling else {
            continue;
        };
        if same_current_node(&item, current) {
            break;
        }
        out.push(ApplyMember::Current(item));
    }
    Some(filter_members_by_step(out, step, ctx))
}

fn same_current_node(left: &CurrentItem, right: &CurrentItem) -> bool {
    left.kind == right.kind
        && left.tag == right.tag
        && left.text == right.text
        && left.attrs == right.attrs
        && left.children == right.children
}

fn filter_members_by_step(
    members: Vec<ApplyMember>,
    step: &str,
    ctx: &EmitCtx,
) -> Vec<ApplyMember> {
    let (step_without_index, index) = parse_step_index(step);
    let filtered: Vec<ApplyMember> = members
        .into_iter()
        .filter(|member| match member {
            ApplyMember::Current(item) if item.kind == CurrentItemKind::Element => {
                step_matches_current(step_without_index, item)
                    && step_predicate_matches_current(step_without_index, item, ctx)
            }
            _ => false,
        })
        .collect();
    if let Some(index) = index {
        filtered
            .get(index.saturating_sub(1))
            .cloned()
            .into_iter()
            .collect()
    } else {
        filtered
    }
}

fn filter_any_members_by_step(
    members: Vec<ApplyMember>,
    step: &str,
    ctx: &EmitCtx,
) -> Vec<ApplyMember> {
    let (step_without_index, index) = parse_step_index(step);
    let filtered: Vec<ApplyMember> = members
        .into_iter()
        .filter_map(|member| {
            let item = match &member {
                ApplyMember::Current(item) => item.clone(),
                ApplyMember::Item(item) => current_item_from_item_node(item, 1),
            };
            if step_matches_current(step_without_index, &item)
                && step_predicate_matches_current(step_without_index, &item, ctx)
            {
                Some(member)
            } else {
                None
            }
        })
        .collect();
    if let Some(index) = index {
        filtered
            .get(index.saturating_sub(1))
            .cloned()
            .into_iter()
            .collect()
    } else {
        filtered
    }
}

fn select_absolute_members(select: &str, ctx: &EmitCtx) -> Option<Vec<ApplyMember>> {
    let mut text = select.trim();
    let descendant = text.starts_with("//");
    text = text.trim_start_matches('/');
    let mut want_attrs: Option<&str> = None;
    let mut want_text = false;
    if let Some(base) = text.strip_suffix("/@*") {
        text = base;
        want_attrs = Some("*");
    } else if let Some(index) = text.rfind("/@") {
        want_attrs = Some(&text[index + 2..]);
        text = &text[..index];
    } else if let Some(base) = text.strip_suffix("/text()") {
        text = base;
        want_text = true;
    }
    let segments: Vec<&str> = text.split('/').filter(|part| !part.is_empty()).collect();
    if segments.is_empty() {
        return Some(Vec::new());
    }
    let document = CurrentItem {
        kind: CurrentItemKind::Document,
        tag: "#document".to_owned(),
        text: String::new(),
        attrs: HashMap::new(),
        children: ctx.root_nodes.clone(),
        parent: None,
        position: 1,
    };
    let mut items = if descendant {
        find_descendant_current_items(&document, segments[0], ctx)
    } else {
        find_child_current_items(&document, segments[0], ctx)
    };
    for segment in segments.iter().skip(1) {
        items = items
            .iter()
            .flat_map(|item| find_child_current_items(item, segment, ctx))
            .collect();
    }
    if let Some(attr_name) = want_attrs {
        return Some(
            items
                .iter()
                .flat_map(|item| {
                    if attr_name == "*" {
                        attribute_children(item)
                    } else {
                        attribute_named(item, attr_name)
                    }
                })
                .collect(),
        );
    }
    if want_text {
        return Some(
            items
                .iter()
                .flat_map(|item| text_children_with_parent(&item.children, item))
                .collect(),
        );
    }
    Some(items.into_iter().map(ApplyMember::Current).collect())
}

fn find_child_current_items(
    parent: &CurrentItem,
    segment: &str,
    ctx: &EmitCtx,
) -> Vec<CurrentItem> {
    filter_members_by_step(
        element_children_with_parent(&parent.children, parent),
        segment,
        ctx,
    )
    .into_iter()
    .filter_map(|member| match member {
        ApplyMember::Current(item) => Some(item),
        ApplyMember::Item(_) => None,
    })
    .collect()
}

fn find_descendant_current_items(
    parent: &CurrentItem,
    segment: &str,
    ctx: &EmitCtx,
) -> Vec<CurrentItem> {
    let mut out = Vec::new();
    for child in element_children_with_parent(&parent.children, parent) {
        let ApplyMember::Current(item) = child else {
            continue;
        };
        if step_matches_current(segment, &item)
            && step_predicate_matches_current(segment, &item, ctx)
        {
            out.push(item.clone());
        }
        out.extend(find_descendant_current_items(&item, segment, ctx));
    }
    out
}

fn parse_step_index(step: &str) -> (&str, Option<usize>) {
    let Some(open) = step.rfind('[') else {
        return (step, None);
    };
    if !step.ends_with(']') {
        return (step, None);
    }
    let inner = &step[open + 1..step.len() - 1];
    if let Ok(index) = inner.trim().parse::<usize>() {
        (&step[..open], Some(index))
    } else {
        (step, None)
    }
}

fn step_name(step: &str) -> &str {
    step.split_once('[')
        .map(|(name, _)| name)
        .unwrap_or(step)
        .trim()
}

fn step_matches_current(step: &str, current: &CurrentItem) -> bool {
    let step = step_name(step);
    step == "*"
        || step == "@*"
        || step.ends_with(":*")
        || current.tag == step
        || step
            .rsplit_once(':')
            .map(|(_, local)| current.tag == local)
            .unwrap_or(false)
}

fn step_predicate_matches_current(step: &str, current: &CurrentItem, ctx: &EmitCtx) -> bool {
    let Some(predicate) = step
        .split_once('[')
        .and_then(|(_, rest)| rest.strip_suffix(']'))
    else {
        return true;
    };
    predicate_matches_current(predicate.trim(), current, ctx)
}

fn predicate_matches_current(predicate: &str, current: &CurrentItem, ctx: &EmitCtx) -> bool {
    if predicate == "*" {
        return current
            .children
            .iter()
            .any(|child| matches!(child, LegacyNode::Element(_)));
    }
    if predicate == "not(*)" {
        return !current
            .children
            .iter()
            .any(|child| matches!(child, LegacyNode::Element(_)));
    }
    if let Some((left, right)) = predicate.split_once('=') {
        let Some(expected) = resolve_predicate_value(right.trim(), ctx) else {
            return false;
        };
        let left = left.trim();
        if matches!(left, "name()" | "local-name()" | "local-name(.)") {
            return current.tag == expected;
        }
        if matches!(
            left,
            "text()" | "." | "string()" | "normalize-space(text())"
        ) {
            let actual = if left == "normalize-space(text())" {
                current.text.trim().to_owned()
            } else {
                current.text.clone()
            };
            return actual == expected;
        }
        if let Some(attr_name) = left.strip_prefix('@') {
            return current.attrs.get(attr_name.trim()).map(String::as_str)
                == Some(expected.as_str());
        }
        return false;
    }
    let Some(rest) = predicate.strip_prefix('@') else {
        return true;
    };
    current.attrs.contains_key(rest)
}

fn resolve_predicate_value(value: &str, ctx: &EmitCtx) -> Option<String> {
    let value = value.trim();
    if let Some(name) = value.strip_prefix('$') {
        return ctx
            .scalars
            .get(name)
            .map(|value| unquote_xpath_literal(value));
    }
    if matches!(
        value,
        "name()" | "local-name()" | "local-name(.)" | "name(current())" | "local-name(current())"
    ) {
        return ctx.item.as_ref().map(|item| item.tag.clone());
    }
    Some(value.trim_matches('"').trim_matches('\'').to_owned())
}

fn unquote_xpath_literal(value: &str) -> String {
    value.trim().trim_matches('"').trim_matches('\'').to_owned()
}

fn element_children_with_parent(nodes: &[LegacyNode], parent: &CurrentItem) -> Vec<ApplyMember> {
    nodes
        .iter()
        .filter_map(|node| match node {
            LegacyNode::Element(element) if !is_xslt_element(&element.tag) => {
                let mut item = current_item_from_element(element, 1);
                item.parent = Some(Box::new(parent.clone()));
                Some(ApplyMember::Current(item))
            }
            _ => None,
        })
        .collect()
}

fn attribute_children(current: &CurrentItem) -> Vec<ApplyMember> {
    current
        .attrs
        .iter()
        .map(|(name, value)| {
            let mut item = current_item_from_attribute(name, value);
            item.parent = Some(Box::new(current.clone()));
            ApplyMember::Current(item)
        })
        .collect()
}

fn attribute_named(current: &CurrentItem, name: &str) -> Vec<ApplyMember> {
    current
        .attrs
        .get(name)
        .map(|value| {
            let mut item = current_item_from_attribute(name, value);
            item.parent = Some(Box::new(current.clone()));
            vec![ApplyMember::Current(item)]
        })
        .unwrap_or_default()
}

fn text_children_with_parent(nodes: &[LegacyNode], parent: &CurrentItem) -> Vec<ApplyMember> {
    nodes
        .iter()
        .filter_map(|node| match node {
            LegacyNode::Text(text) if !text.trim().is_empty() => {
                Some(ApplyMember::Current(CurrentItem {
                    kind: CurrentItemKind::Text,
                    tag: "#text".to_owned(),
                    text: text.clone(),
                    attrs: HashMap::new(),
                    children: Vec::new(),
                    parent: Some(Box::new(parent.clone())),
                    position: 1,
                }))
            }
            _ => None,
        })
        .collect()
}

fn serialize_current_item_to_cem(
    item: &CurrentItem,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    match item.kind {
        CurrentItemKind::Document => item
            .children
            .iter()
            .map(|child| serialize_legacy_node_to_cem(child, ctx, diagnostics))
            .collect(),
        CurrentItemKind::Attribute | CurrentItemKind::Text => escape_literal(&item.text),
        CurrentItemKind::Element => {
            let attrs = attrs_from_current(item);
            let body = item
                .children
                .iter()
                .map(|child| serialize_legacy_node_to_cem(child, ctx, diagnostics))
                .collect::<String>();
            if body.is_empty() {
                format!("{{{}{attrs}}}", item.tag)
            } else {
                format!("{{{}{attrs} | {body}}}", item.tag)
            }
        }
    }
}

fn serialize_item_node_to_cem(
    item: &ItemNode,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let attrs = attrs_from_map(&item.attrs);
    let body = if item.children.is_empty() {
        escape_literal(&item.text)
    } else {
        item.children
            .iter()
            .map(|child| serialize_legacy_node_to_cem(child, ctx, diagnostics))
            .collect()
    };
    if body.is_empty() {
        format!("{{{}{attrs}}}", item.tag)
    } else {
        format!("{{{}{attrs} | {body}}}", item.tag)
    }
}

fn serialize_legacy_node_to_cem(
    node: &LegacyNode,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    match node {
        LegacyNode::Text(text) => escape_literal(&interpolate(text, ctx, diagnostics)),
        LegacyNode::Comment => String::new(),
        LegacyNode::Element(element) => {
            let tag = if element.tag.starts_with("xhtml:") {
                local_name(&element.tag)
            } else {
                element.tag.as_str()
            };
            let mut attrs = element
                .attributes
                .iter()
                .map(|attr| emit_attribute(attr, ctx, diagnostics))
                .collect::<String>();
            attrs.push_str(&emit_xsl_instruction_attributes(
                &element.children,
                ctx,
                diagnostics,
            ));
            let body =
                emit_children_excluding_instruction_attributes(&element.children, ctx, diagnostics);
            if body.is_empty() {
                format!("{{{tag}{attrs}}}")
            } else {
                format!("{{{tag}{attrs} | {body}}}")
            }
        }
    }
}

fn attrs_from_current(item: &CurrentItem) -> String {
    attrs_from_map(&item.attrs)
}

fn attrs_from_map(attrs: &HashMap<String, String>) -> String {
    let mut pairs: Vec<(&String, &String)> = attrs.iter().collect();
    pairs.sort_by(|(left, _), (right, _)| left.cmp(right));
    pairs
        .into_iter()
        .map(|(name, value)| attr_assign(name, value))
        .collect()
}

fn current_item_from_element(element: &LegacyElement, position: usize) -> CurrentItem {
    let mut attrs = HashMap::new();
    for attr in &element.attributes {
        attrs.insert(attr.name.clone(), attr.value.clone());
    }
    CurrentItem {
        kind: CurrentItemKind::Element,
        tag: local_name(&element.tag).to_owned(),
        text: direct_text_content(element),
        attrs,
        children: element.children.clone(),
        parent: None,
        position,
    }
}

fn current_item_from_attribute(name: &str, value: &str) -> CurrentItem {
    CurrentItem {
        kind: CurrentItemKind::Attribute,
        tag: local_name(name).to_owned(),
        text: value.to_owned(),
        attrs: HashMap::new(),
        children: Vec::new(),
        parent: None,
        position: 1,
    }
}

fn apply_sort_children(members: &mut [ApplyMember], element: &LegacyElement) {
    let sorts: Vec<SortSpec> = element
        .children
        .iter()
        .filter_map(|child| match child {
            LegacyNode::Element(child) if local_name(&child.tag) == "sort" => Some(SortSpec {
                select: attr_value(child, "select").unwrap_or(".").to_owned(),
                descending: attr_value(child, "order") == Some("descending"),
                numeric: attr_value(child, "data-type") == Some("number"),
            }),
            _ => None,
        })
        .collect();
    if sorts.is_empty() {
        return;
    };
    members.sort_by(|a, b| {
        for sort in &sorts {
            let ordering = if sort.numeric {
                let left = sort_key_for_member(a, &sort.select)
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0);
                let right = sort_key_for_member(b, &sort.select)
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0);
                left.partial_cmp(&right)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                sort_key_for_member(a, &sort.select).cmp(&sort_key_for_member(b, &sort.select))
            };
            let ordering = if sort.descending {
                ordering.reverse()
            } else {
                ordering
            };
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
        std::cmp::Ordering::Equal
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SortSpec {
    select: String,
    descending: bool,
    numeric: bool,
}

fn sort_key_for_member(member: &ApplyMember, select: &str) -> String {
    let current = match member {
        ApplyMember::Item(item) => current_item_from_item_node(item, 1),
        ApplyMember::Current(item) => item.clone(),
    };
    let select = select.trim();
    if let Some(literal) = resolve_item_literal(select, &current) {
        return literal;
    }
    current
        .children
        .iter()
        .find_map(|child| match child {
            LegacyNode::Element(element) if local_name(&element.tag) == select => {
                Some(text_content(element))
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn emit_slot(
    element: &LegacyElement,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let attrs = attr_value(element, "name")
        .map(|name| attr_assign("name", name))
        .unwrap_or_default();
    let fallback = emit_children(&element.children, ctx, diagnostics);
    if fallback.is_empty() {
        format!("{{slot{attrs}}}")
    } else {
        format!("{{slot{attrs} | {fallback}}}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeSetRef {
    name: String,
    predicate: Option<String>,
}

fn match_node_set_select(select: &str, ctx: &EmitCtx) -> Option<NodeSetRef> {
    let trimmed = select.trim();
    if let Some(name) = trimmed.strip_prefix('$') {
        if is_name(name) && ctx.node_sets.contains_key(name) {
            return Some(NodeSetRef {
                name: name.to_owned(),
                predicate: None,
            });
        }
    }

    let inner = trimmed
        .strip_prefix("exsl:node-set(")
        .or_else(|| trimmed.strip_prefix("node-set("))?;
    let close = inner.find(')')?;
    let var = inner[..close].trim().strip_prefix('$')?;
    if !ctx.node_sets.contains_key(var) {
        return None;
    }
    let rest = inner[close + 1..].trim();
    let rest = rest.strip_prefix('/')?.trim_start();
    let rest = rest.strip_prefix('*')?.trim();
    let predicate = if rest.is_empty() {
        None
    } else if rest.starts_with('[') && rest.ends_with(']') {
        Some(rest[1..rest.len() - 1].to_owned())
    } else {
        return None;
    };
    Some(NodeSetRef {
        name: var.to_owned(),
        predicate,
    })
}

fn rewrite_predicate(
    predicate: &str,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    if let Some(name) = predicate.trim().strip_prefix('$') {
        if is_name(name) {
            if let Some(value) = ctx.scalars.get(name) {
                return value.clone();
            }
        }
    }
    rewrite_expression(predicate, ctx, false, diagnostics)
}

fn interpolate(
    text: &str,
    ctx: &EmitCtx,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(open) = text[cursor..].find('{') {
        let open = cursor + open;
        out.push_str(&text[cursor..open]);
        let Some(close_offset) = text[open + 1..].find('}') else {
            out.push_str(&text[open..]);
            return out;
        };
        let close = open + 1 + close_offset;
        let expression = text[open + 1..close].trim();
        if let Some(item) = &ctx.item {
            if let Some(literal) = resolve_item_literal(expression, item) {
                out.push_str(&literal);
                cursor = close + 1;
                continue;
            }
        }
        if let Some(literal) = evaluate_xpath_literal(expression, ctx) {
            out.push_str(&literal);
            cursor = close + 1;
            continue;
        }
        out.push('{');
        out.push_str(&rewrite_expression(expression, ctx, true, diagnostics));
        out.push('}');
        cursor = close + 1;
    }
    out.push_str(&text[cursor..]);
    out
}

fn resolve_item_literal(expression: &str, item: &CurrentItem) -> Option<String> {
    if expression == "." {
        return Some(item.text.clone());
    }
    if matches!(expression, "text()" | "./text()") {
        return Some(item.text.clone());
    }
    if matches!(
        expression,
        "name()" | "name(.)" | "local-name()" | "local-name(.)"
    ) {
        return Some(item.tag.clone());
    }
    if expression == "position()" {
        return Some(item.position.to_string());
    }
    if let Some(name) = expression.strip_prefix('@') {
        if is_name(name) {
            return Some(item.attrs.get(name).cloned().unwrap_or_default());
        }
    }
    None
}

fn evaluate_xpath_bool(expression: &str, ctx: &EmitCtx) -> Option<bool> {
    let expression = expression.trim();
    if expression == "../.." {
        return ctx
            .item
            .as_ref()
            .and_then(|item| item.parent.as_ref())
            .map(|parent| parent.kind != CurrentItemKind::Document);
    }
    if let Some(inner) = single_function_arg(expression, "not") {
        return evaluate_xpath_bool(inner, ctx).map(|value| !value);
    }
    if let Some((left, operator, right)) = split_top_level_comparison(expression) {
        let left = evaluate_xpath_operand(left, ctx)?;
        let right = evaluate_xpath_operand(right, ctx)?;
        return Some(compare_xpath_operands(&left, operator, &right));
    }
    if let Some(value) = evaluate_xpath_operand(expression, ctx) {
        return Some(xpath_truthy(&value));
    }
    select_members_for_eval(expression, ctx).map(|members| !members.is_empty())
}

fn evaluate_xpath_operand(expression: &str, ctx: &EmitCtx) -> Option<String> {
    let expression = expression.trim();
    if let Some(value) = evaluate_xpath_literal(expression, ctx) {
        return Some(value);
    }
    if let Some(inner) = single_function_arg(expression, "normalize-space") {
        return evaluate_xpath_operand(inner, ctx).map(|value| value.trim().to_owned());
    }
    if let Some(item) = &ctx.item {
        if let Some(value) = resolve_item_literal(expression, item) {
            return Some(value);
        }
    }
    if let Some(name) = expression.strip_prefix('$') {
        if is_name(name) {
            return ctx
                .scalars
                .get(name)
                .map(|value| unquote_xpath_literal(value));
        }
    }
    if let Some(name) = expression.strip_prefix('@') {
        if is_name(name) {
            return ctx
                .item
                .as_ref()
                .map(|item| item.attrs.get(name).cloned().unwrap_or_default());
        }
    }
    if is_quoted_xpath_literal(expression) {
        return Some(unquote_xpath_literal(expression));
    }
    if expression.parse::<f64>().is_ok() {
        return Some(expression.to_owned());
    }
    // Only treat a node-set as a known operand value when we actually found
    // static members. An empty result means the path had no match in the
    // static template AST — for runtime references such as `//show-a` that
    // correspond to datadom slices, this is "unknown at compile time", not
    // "empty string", so we must return None to avoid incorrect static folding.
    select_members_for_eval(expression, ctx)
        .filter(|members| !members.is_empty())
        .map(|members| {
            members
                .iter()
                .map(member_string_value)
                .collect::<Vec<_>>()
                .join("")
        })
}

fn is_quoted_xpath_literal(value: &str) -> bool {
    (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
}

fn xpath_truthy(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value != "0" && value != "false"
}

fn compare_xpath_operands(left: &str, operator: &str, right: &str) -> bool {
    if let (Ok(left_num), Ok(right_num)) = (left.trim().parse::<f64>(), right.trim().parse::<f64>())
    {
        return match operator {
            "=" => left_num == right_num,
            "!=" => left_num != right_num,
            ">" => left_num > right_num,
            "<" => left_num < right_num,
            ">=" => left_num >= right_num,
            "<=" => left_num <= right_num,
            _ => false,
        };
    }
    match operator {
        "=" => left == right,
        "!=" => left != right,
        ">" => left > right,
        "<" => left < right,
        ">=" => left >= right,
        "<=" => left <= right,
        _ => false,
    }
}

fn split_top_level_comparison(expression: &str) -> Option<(&str, &str, &str)> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let bytes = expression.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let ch = expression[index..].chars().next()?;
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '!' if depth == 0 && expression[index..].starts_with("!=") => {
                return Some((&expression[..index], "!=", &expression[index + 2..]));
            }
            '>' if depth == 0 && expression[index..].starts_with(">=") => {
                return Some((&expression[..index], ">=", &expression[index + 2..]));
            }
            '<' if depth == 0 && expression[index..].starts_with("<=") => {
                return Some((&expression[..index], "<=", &expression[index + 2..]));
            }
            '=' if depth == 0 => {
                return Some((&expression[..index], "=", &expression[index + 1..]))
            }
            '>' if depth == 0 => {
                return Some((&expression[..index], ">", &expression[index + 1..]))
            }
            '<' if depth == 0 => {
                return Some((&expression[..index], "<", &expression[index + 1..]))
            }
            _ => {}
        }
        index += ch.len_utf8();
    }
    None
}

fn evaluate_xpath_literal(expression: &str, ctx: &EmitCtx) -> Option<String> {
    let expression = expression.trim();
    if let Some((left, right)) = split_top_level_plus(expression) {
        let left = evaluate_xpath_literal(left, ctx)?;
        let left = left.trim().parse::<f64>().ok()?;
        let right = right.trim().parse::<f64>().ok()?;
        let total = left + right;
        return Some(if total.fract() == 0.0 {
            (total as i64).to_string()
        } else {
            total.to_string()
        });
    }
    if let Some(select) = single_function_arg(expression, "count") {
        return select_members_for_eval(select, ctx).map(|members| members.len().to_string());
    }
    if let Some(select) = single_function_arg(expression, "sum") {
        return select_members_for_eval(select, ctx).map(|members| {
            let total = members.iter().map(member_numeric_value).sum::<f64>();
            if total.fract() == 0.0 {
                (total as i64).to_string()
            } else {
                total.to_string()
            }
        });
    }
    None
}

fn split_top_level_plus(expression: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (index, ch) in expression.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '+' if depth == 0 => return Some((&expression[..index], &expression[index + 1..])),
            _ => {}
        }
    }
    None
}

fn single_function_arg<'a>(expression: &'a str, name: &str) -> Option<&'a str> {
    let inner = expression
        .strip_prefix(name)?
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')?;
    Some(inner.trim())
}

fn select_members_for_eval(select: &str, ctx: &EmitCtx) -> Option<Vec<ApplyMember>> {
    let select = select.trim();
    if let Some(name) = select.strip_prefix('$') {
        if let Some(nodes) = ctx.current_sets.get(name) {
            return Some(nodes.iter().cloned().map(ApplyMember::Current).collect());
        }
        if let Some(nodes) = ctx.node_sets.get(name) {
            return Some(nodes.iter().cloned().map(ApplyMember::Item).collect());
        }
    }
    let mut out = Vec::new();
    for part in select.split('|') {
        let part = part.trim();
        if part.starts_with("//") || part.starts_with('/') {
            out.extend(select_absolute_members(part, ctx)?);
        } else {
            out.extend(select_current_members(part, ctx)?);
        }
    }
    Some(out)
}

fn member_numeric_value(member: &ApplyMember) -> f64 {
    let value = match member {
        ApplyMember::Item(item) => item.text.as_str(),
        ApplyMember::Current(item) => item.text.as_str(),
    };
    value.trim().parse::<f64>().unwrap_or(0.0)
}

fn member_string_value(member: &ApplyMember) -> String {
    match member {
        ApplyMember::Item(item) => item.text.clone(),
        ApplyMember::Current(item) => item.text.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum XToken {
    String(String),
    Number(String),
    Name(String),
    Var(String),
    Punct(String),
}

fn rewrite_expression(
    expression: &str,
    ctx: &EmitCtx,
    interpolation: bool,
    diagnostics: &mut Vec<LegacyConversionDiagnostic>,
) -> String {
    let tokens = tokenize_xpath(expression);
    let mut rewriter = XPathRewriter {
        tokens,
        pos: 0,
        ctx,
        diagnostics,
    };
    let bare = rewriter.rewrite_all().trim().to_owned();
    if interpolation && is_simple_path(&bare) {
        format!("${bare}")
    } else {
        bare
    }
}

fn tokenize_xpath(input: &str) -> Vec<XToken> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            tokens.push(XToken::String(chars[start..i].iter().collect()));
            if i < chars.len() {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            tokens.push(XToken::Number(chars[start..i].iter().collect()));
            continue;
        }
        if c == '$' {
            i += 1;
            let start = i;
            while i < chars.len() && is_name_char(chars[i]) {
                i += 1;
            }
            tokens.push(XToken::Var(chars[start..i].iter().collect()));
            continue;
        }
        if c == '@' {
            i += 1;
            let start = i;
            while i < chars.len() && is_name_char(chars[i]) {
                i += 1;
            }
            tokens.push(XToken::Punct("@".to_owned()));
            tokens.push(XToken::Name(chars[start..i].iter().collect()));
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            tokens.push(XToken::Punct("//".to_owned()));
            i += 2;
            continue;
        }
        if c == '?' && chars.get(i + 1) == Some(&'?') {
            tokens.push(XToken::Punct("??".to_owned()));
            i += 2;
            continue;
        }
        if c == '!' && chars.get(i + 1) == Some(&'=') {
            tokens.push(XToken::Punct("!=".to_owned()));
            i += 2;
            continue;
        }
        if is_name_start(c) {
            let start = i;
            while i < chars.len() && is_name_char(chars[i]) {
                i += 1;
            }
            tokens.push(XToken::Name(chars[start..i].iter().collect()));
            continue;
        }
        tokens.push(XToken::Punct(c.to_string()));
        i += 1;
    }
    tokens
}

struct XPathRewriter<'a, 'd> {
    tokens: Vec<XToken>,
    pos: usize,
    ctx: &'a EmitCtx,
    diagnostics: &'d mut Vec<LegacyConversionDiagnostic>,
}

impl XPathRewriter<'_, '_> {
    fn rewrite_all(&mut self) -> String {
        let mut out = String::new();
        while self.pos < self.tokens.len() {
            out.push_str(&self.rewrite_token());
        }
        out
    }

    fn peek(&self) -> Option<&XToken> {
        self.tokens.get(self.pos)
    }

    fn rewrite_token(&mut self) -> String {
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        match token {
            XToken::String(value) => format!("\"{}\" ", value.replace('"', "\\\"")),
            XToken::Number(value) => format!("{value} "),
            XToken::Var(value) => self
                .ctx
                .scalars
                .get(&value)
                .map(|scalar| format!("{scalar} "))
                .unwrap_or_else(|| format!("{value} ")),
            XToken::Punct(value) => self.rewrite_punct(&value),
            XToken::Name(value) => self.rewrite_name(&value),
        }
    }

    fn rewrite_punct(&mut self, value: &str) -> String {
        if value == "//" {
            if let Some(XToken::Name(next)) = self.peek().cloned() {
                self.pos += 1;
                return format!("datadom.slices.{next} ");
            }
            return String::new();
        }
        if value == "@" {
            if let Some(XToken::Name(next)) = self.peek().cloned() {
                self.pos += 1;
                let base = self.ctx.loop_var.as_deref().unwrap_or("datadom.attributes");
                return format!("{base}.{next} ");
            }
            return String::new();
        }
        if value == "." {
            return self
                .ctx
                .loop_var
                .as_ref()
                .map(|loop_var| format!("{loop_var} "))
                .unwrap_or_else(|| ". ".to_owned());
        }
        format!("{value} ")
    }

    fn rewrite_name(&mut self, value: &str) -> String {
        if matches!(self.peek(), Some(XToken::Punct(punct)) if punct == "(") {
            return self.rewrite_call(value);
        }
        if matches!(value, "and" | "or" | "div" | "mod" | "true" | "false") {
            return format!("{value} ");
        }
        format!("{value} ")
    }

    fn rewrite_call(&mut self, name: &str) -> String {
        self.pos += 1; // consume '('
        let mut args = Vec::new();
        let mut depth = 0;
        let mut current = String::new();
        while self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            match token {
                XToken::Punct(ref punct) if punct == "(" => {
                    depth += 1;
                    current.push_str(&self.rewrite_token());
                }
                XToken::Punct(ref punct) if punct == ")" => {
                    if depth == 0 {
                        self.pos += 1;
                        break;
                    }
                    depth -= 1;
                    current.push_str(&self.rewrite_token());
                }
                XToken::Punct(ref punct) if punct == "," && depth == 0 => {
                    args.push(current.trim().to_owned());
                    current.clear();
                    self.pos += 1;
                }
                _ => current.push_str(&self.rewrite_token()),
            }
        }
        if !current.trim().is_empty() {
            args.push(current.trim().to_owned());
        }
        self.emit_call(name, &args)
    }

    fn emit_call(&mut self, name: &str, args: &[String]) -> String {
        match function_disposition(name) {
            LegacyFunctionDisposition::Special if name == "position" => "position ".to_owned(),
            LegacyFunctionDisposition::Special if name == "current" => ". ".to_owned(),
            LegacyFunctionDisposition::Special if name == "not" => {
                format!("not ({}) ", args.join(", "))
            }
            LegacyFunctionDisposition::Special if name == "concat" => {
                format!("str:concat(({})) ", args.join(", "))
            }
            LegacyFunctionDisposition::Special if name == "hasBoolAttribute" => {
                // DCE compile-time rewrite: hasBoolAttribute($attr) expands to the
                // idiomatic HTML boolean attribute test — true when the value is empty,
                // the attribute name itself, or "true", and not explicitly "false".
                // The arg is either a bare variable name (from $attr rewriting) or a
                // quoted string literal — strip surrounding CEM-QL string quotes when
                // present so the attr name can be used both as a binding and a literal.
                let raw = args.first().map(|s| s.trim()).unwrap_or("");
                let attr = if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
                    &raw[1..raw.len() - 1]
                } else {
                    raw
                };
                format!(
                    r#"not ({attr} = "false") and ({attr} = "" or {attr} = "{attr}" or {attr} = "true") "#
                )
            }
            LegacyFunctionDisposition::CemQl(mapped) => {
                format!("{mapped}({}) ", args.join(", "))
            }
            _ => {
                self.diagnostics.push(diag(
                    UNSUPPORTED_FUNCTION_CODE,
                    format!("XPath function {name}() has no cem-ql mapping; passed through"),
                ));
                format!("{name}({}) ", args.join(", "))
            }
        }
    }
}

fn attr_value<'a>(element: &'a LegacyElement, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attr| attr.name == name)
        .map(|attr| attr.value.as_str())
}

fn text_content(element: &LegacyElement) -> String {
    element
        .children
        .iter()
        .map(|child| match child {
            LegacyNode::Text(text) => text.clone(),
            LegacyNode::Element(child) => text_content(child),
            LegacyNode::Comment => String::new(),
        })
        .collect()
}

fn direct_text_content(element: &LegacyElement) -> String {
    element
        .children
        .iter()
        .filter_map(|child| match child {
            LegacyNode::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn expr_attr(name: &str, expression: &str) -> String {
    attr_assign(name, expression)
}

fn attr_assign(name: &str, value: &str) -> String {
    if !value.contains('"') {
        format!(" @{name}=\"{value}\"")
    } else if !value.contains('\'') {
        format!(" @{name}='{value}'")
    } else {
        format!(" @{name}='{}'", value.replace('\'', "&apos;"))
    }
}

fn escape_literal(text: &str) -> String {
    if text.chars().any(|ch| matches!(ch, '{' | '}' | '|' | '`')) {
        emit_rich_content(text)
    } else {
        text.to_owned()
    }
}

fn emit_rich_content(text: &str) -> String {
    format!("```{text}```")
}

fn local_name(tag: &str) -> &str {
    tag.rsplit_once(':').map(|(_, local)| local).unwrap_or(tag)
}

fn is_xslt_element(tag: &str) -> bool {
    tag.starts_with("xsl:")
}

fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_owned();
    }
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn is_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(ch) if is_name_start(ch)) && chars.all(is_name_char)
}

fn is_simple_path(expression: &str) -> bool {
    let mut chars = expression.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
}

fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-' | ':')
}

fn diag(code: impl Into<String>, message: impl Into<String>) -> LegacyConversionDiagnostic {
    LegacyConversionDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(input: &str) -> LegacyConversionResult {
        convert_template_source(input)
    }

    #[test]
    fn classifies_material_bridge_elements() {
        assert_eq!(
            element_disposition("if"),
            LegacyElementDisposition::ControlFlow
        );
        assert_eq!(
            element_disposition("choose"),
            LegacyElementDisposition::ControlFlow
        );
        assert_eq!(
            element_disposition("attribute"),
            LegacyElementDisposition::Declaration
        );
        assert_eq!(
            element_disposition("module-url"),
            LegacyElementDisposition::Declaration
        );
        assert_eq!(
            element_disposition("span"),
            LegacyElementDisposition::OutputElement
        );
    }

    #[test]
    fn tier3_push_model_constructs_are_handoff_only() {
        for name in ["function", "script"] {
            assert_eq!(
                element_disposition(name),
                LegacyElementDisposition::Tier3Handoff
            );
        }
    }

    #[test]
    fn classifies_stylesheet_compat_constructs_separately() {
        for name in [
            "template",
            "call-template",
            "apply-templates",
            "stylesheet",
            "sort",
            "copy",
            "copy-of",
            "attribute",
            "element",
            "output",
        ] {
            assert_eq!(
                xslt_compat_disposition(name),
                LegacyXsltCompatDisposition::StylesheetCompat
            );
        }
        assert_eq!(
            xslt_compat_disposition("if"),
            LegacyXsltCompatDisposition::FragmentBridge
        );
        assert_eq!(
            xslt_compat_disposition("script"),
            LegacyXsltCompatDisposition::Handoff
        );
    }

    #[test]
    fn maps_supported_xpath_functions_to_cem_ql_contract() {
        assert_eq!(
            function_disposition("contains"),
            LegacyFunctionDisposition::CemQl("str:contains")
        );
        assert_eq!(
            function_disposition("string-length"),
            LegacyFunctionDisposition::CemQl("str:length")
        );
        assert_eq!(
            function_disposition("count"),
            LegacyFunctionDisposition::CemQl("seq:count")
        );
        assert_eq!(
            function_disposition("concat"),
            LegacyFunctionDisposition::Special
        );
        assert_eq!(
            function_disposition("position"),
            LegacyFunctionDisposition::Special
        );
    }

    #[test]
    fn hasboolattribute_is_a_special_dce_rewrite() {
        assert_eq!(
            function_disposition("hasBoolAttribute"),
            LegacyFunctionDisposition::Special
        );
    }

    #[test]
    fn recognizes_legacy_content_types() {
        assert!(is_legacy_custom_element_content_type(TEMPLATE_LANG));
        assert!(is_legacy_custom_element_content_type(
            "text/custom-element-xslt; charset=utf-8"
        ));
        assert!(!is_legacy_custom_element_content_type("text/html"));
    }

    #[test]
    fn lowers_avt_attribute_and_text_interpolation() {
        let result = convert(r#"<a href="{$href}">Go {$label}</a>"#);
        assert_eq!(result.source, r#"{a @href="{$href}" | Go {$label}}"#);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lowers_choose_when_contains_to_cem_choose() {
        let result = convert(
            r#"<choose><when test="contains($icon,'/')"><img src="{$icon}"/></when><when test="$icon"><span>{$icon}</span></when></choose>"#,
        );
        assert_eq!(
            result.source,
            r#"{cem:choose | {cem:when @test='str:contains(icon, "/")' | {img @src="{$icon}"}}{cem:when @test="icon" | {span | {$icon}}}}"#
        );
    }

    #[test]
    fn lowers_xsl_value_of() {
        let result = convert(r#"<xsl:value-of select="$name"/>"#);
        assert_eq!(result.source, "{$name}");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lowers_for_each_with_context_item_attribute_and_position() {
        let result = convert(
            r#"<for-each select="$rows"><div style="color:{@hex}">{position()}. {.}</div></for-each>"#,
        );
        assert_eq!(
            result.source,
            r#"{cem:for-each @select="rows" @as="item" | {div @style="color:{$item.hex}" | {$position}. {$item}}}"#
        );
    }

    #[test]
    fn unrolls_inline_node_set_variable() {
        let result = convert(
            r#"<variable name="fruits"><item>Apple</item><item>Banana</item></variable><ul><for-each select="exsl:node-set($fruits)/*"><li>{.}</li></for-each></ul>"#,
        );
        assert_eq!(result.source, "{ul | {li | Apple}{li | Banana}}");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn unrolls_node_set_with_scalar_predicate() {
        let result = convert(
            r#"<variable name="show" select="//show-items = 'yes'"/><variable name="items"><item>First</item><item>Second</item></variable><for-each select="exsl:node-set($items)/*[$show]"><span>{.}</span></for-each>"#,
        );
        assert_eq!(
            result.source,
            r#"{cem:if @test='datadom.slices.show-items = "yes"' | {span | First}}{cem:if @test='datadom.slices.show-items = "yes"' | {span | Second}}"#
        );
    }

    #[test]
    fn binds_exsl_node_set_selection_variables_as_node_set_aliases() {
        let result = convert(
            r#"<xsl:variable name="table-data"><row><cell>A1</cell><cell>A2</cell></row><row><cell>B1</cell><cell>B2</cell></row></xsl:variable><variable name="rows" select="exsl:node-set($table-data)/*"/><table><for-each select="$rows"><tr><for-each select="*"><td>{.}</td></for-each></tr></for-each></table>"#,
        );
        assert_eq!(
            result.source,
            "{table | {tr | {td | A1}{td | A2}}{tr | {td | B1}{td | B2}}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn binds_exsl_node_set_predicate_attribute_selection_as_scalar() {
        let result = convert(
            r#"<xsl:variable name="methods"><a href="h1" title="./set-url.html?history=pushState">pushState</a><a href="h2" title="./set-url.html?history=replaceState">replaceState</a></xsl:variable><variable name="selected-method" select="'replaceState'"/><variable name="selected-url" select="exsl:node-set($methods)/*[text() = $selected-method]/@title"/><p><xsl:value-of select="$selected-url"/></p>"#,
        );
        assert_eq!(result.source, "{p | ./set-url.html?history=replaceState}");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn maps_concat_to_sequence_join() {
        let result = convert(r#"<span title="{concat($a, '-', $b)}"/>"#);
        assert_eq!(
            result.source,
            r#"{span @title='{str:concat((a, "-", b))}'}"#
        );
    }

    #[test]
    fn lowers_named_template_call_with_params() {
        let result = convert(
            r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:call-template name="badge"><xsl:with-param name="label" select="'New'"/></xsl:call-template></xsl:template><xsl:template name="badge"><span class="badge">{$label}</span></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(result.source, r#"{span @class="badge" | {"New"}}"#);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lowers_template_param_defaults_from_current_item() {
        let result = convert(
            r#"<doc><item>A</item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:apply-templates select="//item"/></xsl:template><xsl:template match="item"><xsl:param name="childName" select="name()"/><p><xsl:value-of select="$childName"/></p></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(result.source, "{doc | {item | A}}{p | item}");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn passes_node_set_with_param_to_named_template() {
        let result = convert(
            r#"<doc><item>A</item><item>B</item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:call-template name="render"><xsl:with-param name="data" select="*"/></xsl:call-template></xsl:template><xsl:template name="render"><xsl:param name="data"/><out><xsl:apply-templates select="$data"/></out></xsl:template><xsl:template match="item"><i><xsl:value-of select="."/></i></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item | A}{item | B}}{out | {i | A}{i | B}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lowers_apply_templates_over_inline_node_set_with_simple_match() {
        let result = convert(
            r#"<xsl:stylesheet version="1.0"><xsl:variable name="items"><item>One</item><item>Two</item></xsl:variable><xsl:template match="/"><ul><xsl:apply-templates select="exsl:node-set($items)/*"/></ul></xsl:template><xsl:template match="item"><li><xsl:value-of select="."/></li></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(result.source, "{ul | {li | One}{li | Two}}");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lowers_apply_templates_over_document_children_attributes_and_text() {
        let result = convert(
            r#"<doc><item id="a">Alpha<child>Beta</child></item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:apply-templates select="*"/></xsl:template><xsl:template match="doc"><section><xsl:apply-templates select="*"/></section></xsl:template><xsl:template match="item"><p><xsl:value-of select="name()"/><xsl:apply-templates select="@*"/><xsl:value-of select="./text()"/><xsl:apply-templates select="*"/></p></xsl:template><xsl:template match="@*"><i><xsl:value-of select="name()"/>=<xsl:value-of select="."/></i></xsl:template><xsl:template match="child"><b><xsl:value-of select="."/></b></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item @id=\"a\" | Alpha{child | Beta}}}{section | {p | item{i | id=a}Alpha{b | Beta}}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn unrolls_for_each_over_current_attribute_and_child_union() {
        let result = convert(
            r#"<doc><item id="a"><child>Beta</child></item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:apply-templates select="//item"/></xsl:template><xsl:template match="item"><xsl:for-each select="@*|*"><b><xsl:value-of select="name()"/>:<xsl:value-of select="."/></b></xsl:for-each></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item @id=\"a\" | {child | Beta}}}{b | id:a}{b | child:Beta}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn selects_namespaced_wildcards_indexed_children_specific_attributes_and_parent_paths() {
        let result = convert(
            r#"<doc><row href="x"><xhtml:td>One</xhtml:td><xhtml:td>Two</xhtml:td></row></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:apply-templates select="//row/@href"/><xsl:apply-templates select="//row/xhtml:td[2]"/><xsl:apply-templates select="//row/xhtml:*"/></out></xsl:template><xsl:template match="@*"><a><xsl:value-of select="name()"/>=<xsl:value-of select="."/></a></xsl:template><xsl:template match="td"><b><xsl:apply-templates select="../@href"/>:<xsl:value-of select="."/></b></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {row @href=\"x\" | {td | One}{td | Two}}}{out | {a | href=x}{b | {a | href=x}:Two}{b | {a | href=x}:One}{b | {a | href=x}:Two}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lowers_copy_copy_of_and_xsl_attribute_in_bounded_templates() {
        let result = convert(
            r#"<doc><item id="a">Alpha<child title="b">Beta</child></item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:apply-templates select="//item"/><xsl:copy-of select="//child"/></out></xsl:template><xsl:template match="item"><xsl:copy><xsl:copy-of select="@*"/><xsl:attribute name="data-name"><xsl:value-of select="name()"/></xsl:attribute><xsl:apply-templates select="*"/></xsl:copy></xsl:template><xsl:template match="child"><leaf><xsl:attribute name="title"><xsl:value-of select="@title"/></xsl:attribute><xsl:value-of select="."/></leaf></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item @id=\"a\" | Alpha{child @title=\"b\" | Beta}}}{out | {item @id=\"a\" @data-name=\"item\" | {leaf @title=\"b\" | Beta}}{child @title=\"b\" | Beta}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lowers_dynamic_xsl_element_name_from_current_item_variable() {
        let result = convert(
            r#"<doc><alpha/><beta/></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:apply-templates select="//doc/*"/></out></xsl:template><xsl:template match="*"><xsl:variable name="p" select="name()"/><xsl:element name="{$p}"><xsl:attribute name="xv"><xsl:value-of select="$p"/></xsl:attribute></xsl:element></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {alpha}{beta}}{out | {alpha @xv=\"alpha\"}{beta @xv=\"beta\"}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn selects_predicates_with_scalar_param_values() {
        let result = convert(
            r#"<datadom><payload><div slot="hero">Hero</div><div slot="other">Other</div></payload></datadom><xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:call-template name="slot"><xsl:with-param name="slotname" select="'hero'"/></xsl:call-template></xsl:template><xsl:template name="slot"><xsl:param name="slotname"/><xsl:copy-of select="//payload/*[@slot=$slotname]"/></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{datadom | {payload | {div @slot=\"hero\" | Hero}{div @slot=\"other\" | Other}}}{div @slot=\"hero\" | Hero}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn evaluates_count_and_sum_over_sample_style_absolute_attribute_sets() {
        let result = convert(
            r#"<doc><value a="2" b="3"/><value c="4"/></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:value-of select="count(//value/@*)"/>/<xsl:value-of select="sum(//value/@*)"/></out></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {value @a=\"2\" @b=\"3\"}{value @c=\"4\"}}{out | 3/9}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn selects_variable_rooted_current_item_paths() {
        let result = convert(
            r#"<doc><row id="r1"><cell>A</cell><cell>B</cell><other>C</other></row></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:apply-templates select="//row"/></out></xsl:template><xsl:template match="row"><xsl:variable name="rowNode" select="."/><xsl:variable name="key" select="'cell'"/><xsl:variable name="attrKey" select="'id'"/><first><xsl:apply-templates select="$rowNode/*[name()=$key][1]"/></first><attr><xsl:apply-templates select="$rowNode/@*[name()=$attrKey]"/></attr></xsl:template><xsl:template match="cell"><b><xsl:value-of select="."/></b></xsl:template><xsl:template match="@*"><i><xsl:value-of select="name()"/>=<xsl:value-of select="."/></i></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {row @id=\"r1\" | {cell | A}{cell | B}{other | C}}}{out | {first | {b | A}}{attr | {i | id=r1}}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn selects_preceding_siblings_with_current_name_predicate() {
        let result = convert(
            r#"<doc><item>A</item><other>B</other><item>C</item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:apply-templates select="//item"/></out></xsl:template><xsl:template match="item"><n><xsl:value-of select="count(preceding-sibling::*[name()=name(current())]) + 1"/></n></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item | A}{other | B}{item | C}}{out | {n | 1}{n | 2}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn folds_static_if_tests_for_current_text_attributes_and_ancestors() {
        let result = convert(
            r#"<doc><wrap><item name="hero"> Text </item><item>   </item></wrap></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:apply-templates select="//item"/></out></xsl:template><xsl:template match="item"><xsl:if test="../..">/</xsl:if><xsl:if test="@name">named</xsl:if><xsl:if test="normalize-space(text()) != ''"><p><xsl:value-of select="text()"/></p></xsl:if></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {wrap | {item @name=\"hero\" |  Text }{item |    }}}{out | /named{p |  Text }/}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn folds_static_choose_count_comparisons() {
        let result = convert(
            r#"<doc><item>A</item><item>B</item><solo>C</solo></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><out><xsl:apply-templates select="//item"/><xsl:apply-templates select="//solo"/></out></xsl:template><xsl:template match="*"><xsl:variable name="tagName" select="name()"/><xsl:choose><xsl:when test="count(../*[name()=$tagName]) != 1"><many><xsl:value-of select="name()"/></many></xsl:when><xsl:otherwise><one><xsl:value-of select="name()"/></one></xsl:otherwise></xsl:choose></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item | A}{item | B}{solo | C}}{out | {many | item}{many | item}{one | solo}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn applies_template_priority_and_basic_sort() {
        let result = convert(
            r#"<doc><item rank="2">B</item><item rank="1">A</item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><ol><xsl:apply-templates select="//item"><xsl:sort select="@rank" order="ascending"/></xsl:apply-templates></ol></xsl:template><xsl:template match="item" priority="-10"><li>wrong</li></xsl:template><xsl:template match="*" priority="20"><li><xsl:value-of select="@rank"/>:<xsl:value-of select="."/></li></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item @rank=\"2\" | B}{item @rank=\"1\" | A}}{ol | {li | 1:A}{li | 2:B}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn applies_default_template_rules_for_unmatched_elements() {
        let result = convert(
            r#"<doc><wrap><item>Alpha</item></wrap><item>Beta</item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:apply-templates select="*"/></xsl:template><xsl:template match="item"><b><xsl:value-of select="."/></b></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {wrap | {item | Alpha}}{item | Beta}}{b | Alpha}{b | Beta}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn sorts_with_multiple_keys_and_numeric_type() {
        let result = convert(
            r#"<doc><item group="b" rank="2">B2</item><item group="a" rank="10">A10</item><item group="a" rank="2">A2</item></doc><xsl:stylesheet version="1.0"><xsl:template match="/"><ol><xsl:apply-templates select="//item"><xsl:sort select="@group"/><xsl:sort select="@rank" data-type="number"/></xsl:apply-templates></ol></xsl:template><xsl:template match="item"><li><xsl:value-of select="@group"/>:<xsl:value-of select="@rank"/>:<xsl:value-of select="."/></li></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(
            result.source,
            "{doc | {item @group=\"b\" @rank=\"2\" | B2}{item @group=\"a\" @rank=\"10\" | A10}{item @group=\"a\" @rank=\"2\" | A2}}{ol | {li | a:2:A2}{li | a:10:A10}{li | b:2:B2}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn stops_recursive_template_calls_at_bounded_limit() {
        let result = convert(
            r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><xsl:call-template name="again"/></xsl:template><xsl:template name="again"><xsl:call-template name="again"/></xsl:template></xsl:stylesheet>"#,
        );
        assert_eq!(result.source, "");
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].code,
            "legacy_xslt.template_recursion_limit"
        );
    }

    #[test]
    fn reports_unsupported_tier3_constructs() {
        let result = convert(r#"<xsl:copy-of select="node()"/>"#);
        assert_eq!(result.source, "");
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, UNSUPPORTED_CONSTRUCT_CODE);
    }

    #[test]
    fn reports_apply_templates_outside_bounded_subset() {
        let result = convert(r#"<xsl:apply-templates select="node()"/>"#);
        assert_eq!(result.source, "");
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, UNSUPPORTED_CONSTRUCT_CODE);
    }

    #[test]
    fn lowers_hasboolattribute_to_idiomatic_html_boolean_test() {
        // Variable-ref form used by material input component.
        let result = convert(
            r#"<if test="hasBoolAttribute($disabled)"><attribute name="disabled">{$disabled}</attribute></if>"#,
        );
        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.source,
            "{cem:if @test='not (disabled = \"false\") and (disabled = \"\" or disabled = \"disabled\" or disabled = \"true\")' | {attribute @name=\"disabled\" | {$disabled}}}"
        );
        // String-literal form (strips surrounding CEM-QL quotes).
        let result2 = convert(r#"<if test="hasBoolAttribute('required')"><span>yes</span></if>"#);
        assert!(result2.diagnostics.is_empty());
        assert!(result2.source.contains(r#"required = "required""#));
    }

    #[test]
    fn decodes_html_entities_in_xpath_attributes() {
        let result = convert(r#"<if test="string-length($image) &lt; 3">short</if>"#);
        assert_eq!(
            result.source,
            r#"{cem:if @test="str:length(image) < 3" | short}"#
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn prefixed_void_html_elements_can_carry_legacy_children() {
        let result = convert(
            r#"<xhtml:input value="{$value}"><if test="$required"><attribute name="required">{$required}</attribute></if></xhtml:input>"#,
        );
        assert_eq!(
            result.source,
            r#"{input @value="{$value}" | {cem:if @test="required" | {attribute @name="required" | {$required}}}}"#
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn bare_html_template_is_output_and_xsl_template_is_registered() {
        let bare = convert("<template><span>Demo</span></template>");
        assert_eq!(bare.source, "{template | {span | Demo}}");
        assert!(bare.diagnostics.is_empty());

        let xsl = convert(r#"<xsl:template match="/"><span>Demo</span></xsl:template>"#);
        assert_eq!(xsl.source, "");
        assert!(xsl.diagnostics.is_empty());
    }

    #[test]
    fn material_manifest_primary_templates_convert_under_engine_subset() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest_path =
            root.join("packages/custom-element/test-fixtures/legacy-compat-manifest.json");
        let manifest_text = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|err| panic!("read {}: {err}", manifest_path.display()));
        let manifest: serde_json::Value =
            serde_json::from_str(&manifest_text).expect("legacy manifest JSON");
        assert_eq!(manifest["schemaVersion"], 1);

        for component in manifest["materialComponents"]
            .as_array()
            .expect("materialComponents[]")
        {
            let name = component["name"].as_str().expect("component name");
            let html_path = root.join(format!(
                "packages/custom-element/material/components/{name}.html"
            ));
            let html = std::fs::read_to_string(&html_path)
                .unwrap_or_else(|err| panic!("read {}: {err}", html_path.display()));
            let templates = extract_document_templates(&html);
            assert!(
                !templates.is_empty(),
                "{name}: no top-level templates found"
            );

            let allows = allowed_diagnostics(component);
            let mut allowed_counts = vec![0usize; allows.len()];
            let mut results = Vec::new();
            for template in &templates {
                let result = convert_template_source(&template.body);
                for diagnostic in &result.diagnostics {
                    let Some(index) = allows
                        .iter()
                        .position(|allow| diagnostic_matches(allow, diagnostic))
                    else {
                        panic!(
                            "{name}: unexpected diagnostic {} -- {}",
                            diagnostic.code, diagnostic.message
                        );
                    };
                    allowed_counts[index] += 1;
                }
                results.push((template, result));
            }

            for (index, allow) in allows.iter().enumerate() {
                if let Some(max_count) = allow.max_count {
                    assert!(
                        allowed_counts[index] <= max_count,
                        "{name}: {} emitted {} times; max {max_count}",
                        allow.code,
                        allowed_counts[index]
                    );
                }
            }

            for required in component["requiredTemplates"]
                .as_array()
                .expect("requiredTemplates[]")
            {
                let selector = required["selector"].as_str().expect("selector");
                let min_source_len = required["minSourceLength"].as_u64().unwrap_or(1) as usize;
                let (_, result) = find_required_template_result(&results, selector)
                    .unwrap_or_else(|| panic!("{name}: required selector `{selector}` not found"));
                assert!(
                    result.source.trim().len() >= min_source_len,
                    "{name}: `{selector}` produced {} chars; expected >= {min_source_len}",
                    result.source.trim().len()
                );
            }
        }
    }

    #[derive(Debug)]
    struct ExtractedTemplate {
        attrs: std::collections::HashMap<String, String>,
        parent_custom_element_tag: Option<String>,
        body: String,
    }

    #[derive(Debug)]
    struct AllowedDiagnostic {
        code: String,
        message_includes: Option<String>,
        max_count: Option<usize>,
    }

    fn extract_document_templates(html: &str) -> Vec<ExtractedTemplate> {
        let mut templates = Vec::new();
        let mut cursor = 0;
        while let Some(start_offset) = html[cursor..].find("<template") {
            let start = cursor + start_offset;
            let Some(open_end_offset) = html[start..].find('>') else {
                break;
            };
            let open_end = start + open_end_offset;
            let open_tag = &html[start..=open_end];
            let attrs = parse_tag_attrs(open_tag);

            let mut depth = 1usize;
            let mut search = open_end + 1;
            let mut close_start = None;
            let mut close_end = None;
            while depth > 0 {
                let next_open = html[search..]
                    .find("<template")
                    .map(|offset| search + offset);
                let next_close = html[search..]
                    .find("</template>")
                    .map(|offset| search + offset);
                match (next_open, next_close) {
                    (Some(open), Some(close)) if open < close => {
                        depth += 1;
                        search = open + "<template".len();
                    }
                    (_, Some(close)) => {
                        depth -= 1;
                        if depth == 0 {
                            close_start = Some(close);
                            close_end = Some(close + "</template>".len());
                            break;
                        }
                        search = close + "</template>".len();
                    }
                    _ => break,
                }
            }

            let Some(close_start) = close_start else {
                break;
            };
            let close_end = close_end.expect("close_end set with close_start");
            templates.push(ExtractedTemplate {
                attrs,
                parent_custom_element_tag: parent_custom_element_tag(html, start),
                body: html[open_end + 1..close_start].to_owned(),
            });
            cursor = close_end;
        }
        templates
    }

    fn parse_tag_attrs(tag: &str) -> std::collections::HashMap<String, String> {
        let mut attrs = std::collections::HashMap::new();
        let chars: Vec<char> = tag.chars().collect();
        let mut cursor = tag.find(char::is_whitespace).unwrap_or(tag.len());
        while cursor < chars.len() {
            while cursor < chars.len() && chars[cursor].is_whitespace() {
                cursor += 1;
            }
            if cursor >= chars.len() || matches!(chars[cursor], '>' | '/') {
                break;
            }
            let name_start = cursor;
            while cursor < chars.len()
                && !chars[cursor].is_whitespace()
                && !matches!(chars[cursor], '=' | '>' | '/')
            {
                cursor += 1;
            }
            let name: String = chars[name_start..cursor].iter().collect();
            while cursor < chars.len() && chars[cursor].is_whitespace() {
                cursor += 1;
            }
            let value = if cursor < chars.len() && chars[cursor] == '=' {
                cursor += 1;
                while cursor < chars.len() && chars[cursor].is_whitespace() {
                    cursor += 1;
                }
                if cursor < chars.len() && matches!(chars[cursor], '"' | '\'') {
                    let quote = chars[cursor];
                    cursor += 1;
                    let value_start = cursor;
                    while cursor < chars.len() && chars[cursor] != quote {
                        cursor += 1;
                    }
                    let value: String = chars[value_start..cursor].iter().collect();
                    if cursor < chars.len() {
                        cursor += 1;
                    }
                    decode_html_entities(&value)
                } else {
                    let value_start = cursor;
                    while cursor < chars.len()
                        && !chars[cursor].is_whitespace()
                        && !matches!(chars[cursor], '>' | '/')
                    {
                        cursor += 1;
                    }
                    decode_html_entities(&chars[value_start..cursor].iter().collect::<String>())
                }
            } else {
                String::new()
            };
            attrs.insert(name, value);
        }
        attrs
    }

    fn parent_custom_element_tag(html: &str, template_start: usize) -> Option<String> {
        let prefix = &html[..template_start];
        let open = prefix.rfind("<custom-element")?;
        let close = prefix.rfind("</custom-element>");
        if close.map(|close| close > open).unwrap_or(false) {
            return None;
        }
        let open_end = html[open..].find('>').map(|offset| open + offset)?;
        parse_tag_attrs(&html[open..=open_end]).remove("tag")
    }

    fn allowed_diagnostics(component: &serde_json::Value) -> Vec<AllowedDiagnostic> {
        component["allowedDiagnostics"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|allow| AllowedDiagnostic {
                code: allow["code"].as_str().unwrap_or("").to_owned(),
                message_includes: allow["messageIncludes"].as_str().map(str::to_owned),
                max_count: allow["maxCount"].as_u64().map(|value| value as usize),
            })
            .collect()
    }

    fn diagnostic_matches(
        allow: &AllowedDiagnostic,
        diagnostic: &LegacyConversionDiagnostic,
    ) -> bool {
        allow.code == diagnostic.code
            && allow
                .message_includes
                .as_ref()
                .map(|needle| diagnostic.message.contains(needle))
                .unwrap_or(true)
    }

    fn find_required_template_result<'a>(
        results: &'a [(&'a ExtractedTemplate, LegacyConversionResult)],
        selector: &str,
    ) -> Option<&'a (&'a ExtractedTemplate, LegacyConversionResult)> {
        if let Some(id) = selector.strip_prefix("template#") {
            return results
                .iter()
                .find(|(template, _)| template.attrs.get("id").map(String::as_str) == Some(id));
        }
        let tag_marker = "custom-element[tag=\"";
        if let Some(rest) = selector.strip_prefix(tag_marker) {
            let tag = rest.split('"').next()?;
            return results
                .iter()
                .find(|(template, _)| template.parent_custom_element_tag.as_deref() == Some(tag));
        }
        None
    }
}
