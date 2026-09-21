//! Opt-in, format-neutral complex-content construction. Pending values stay
//! distinct from text nodes until their enclosing element/document consumes them.
use super::*;
use crate::xpath::functions::XPathQueryItem;
use cem_ml::validation::xpath::{
    XPathEvaluationLimits, XPathNativeNode, XPathResultItem, XPathResultNodeKind,
};

const XML: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS: &str = "http://www.w3.org/2000/xmlns/";

#[derive(Default)]
pub(super) struct ResultBuffer(Vec<ResultItem>);

pub(super) enum ResultItem {
    Node(RenderPlanNode),
    Document(Vec<RenderPlanNode>),
    Attribute(RenderPlanAttribute),
    Value(Item, SourceMapStack),
}

impl From<RenderPlanNode> for ResultItem {
    fn from(value: RenderPlanNode) -> Self {
        Self::Node(value)
    }
}

impl ResultBuffer {
    pub(super) fn values(&mut self, stream: ItemStream, source: &SourceMapStack) {
        self.0.extend(stream.items.into_iter().map(|item| ResultItem::Value(item, source.clone())));
    }
    pub(super) fn push(&mut self, node: RenderPlanNode) {
        self.0.push(node.into());
    }
    pub(super) fn extend(&mut self, nodes: impl IntoIterator<Item = impl Into<ResultItem>>) {
        self.0.extend(nodes.into_iter().map(Into::into));
    }
    pub(super) fn clear(&mut self) {
        self.0.clear();
    }
}

impl IntoIterator for ResultBuffer {
    type Item = ResultItem;
    type IntoIter = std::vec::IntoIter<ResultItem>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Error names are supplied by the caller's language lowering; shared CEMT
/// construction itself reports stable CEM diagnostics.
struct Policy<'a> {
    attributes: &'a [TemplateAttribute],
    source: &'a SourceMapStack,
    origin: Option<String>,
}

impl PlanRenderer<'_> {
    pub(super) fn finish_result_buffer(
        &mut self,
        buffer: ResultBuffer,
        source: &SourceMapStack,
    ) -> Vec<RenderPlanNode> {
        let mut nodes = Vec::new();
        for item in buffer.0 {
            match item {
                ResultItem::Node(node) => nodes.push(node),
                ResultItem::Document(children) => nodes.extend(children),
                _ => {
                    self.result_failure(
                        "unconsumed",
                        "Native results require an enclosing result element or document",
                        &Policy {
                            attributes: &[],
                            source,
                            origin: None,
                        },
                    );
                    return Vec::new();
                }
            }
        }
        if self.failure.is_some() || self.control_failed {
            return Vec::new();
        }
        for node in &mut nodes {
            self.fixup_namespaces(
                node,
                &BTreeMap::new(),
                &Policy {
                    attributes: &[],
                    source,
                    origin: None,
                },
                0,
            );
        }
        if self.failure.is_some() || self.control_failed {
            Vec::new()
        } else {
            nodes
        }
    }

    pub(super) fn render_result(
        &mut self,
        instruction: ResultInstruction,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        source: &SourceMapStack,
        out: &mut ResultBuffer,
    ) {
        let origin = attributes
            .iter()
            .find(|a| a.name == "origin-uri")
            .map(|a| self.render_attribute_value(a).0);
        let policy = Policy {
            attributes,
            source,
            origin,
        };
        if let Some(message) = validate_instruction(instruction, attributes, children) {
            self.result_failure("instruction", message, &policy);
            return;
        }
        match instruction {
            ResultInstruction::Sequence => {
                let select = attributes.iter().find_map(|a| match (&*a.name, &a.value) {
                    ("select", Some(TemplateAttributeValue::Expression(value))) => Some(value),
                    _ => None,
                });
                let Some(select) = select else {
                    self.result_failure(
                        "select",
                        "result-sequence requires an expression in select",
                        &policy,
                    );
                    return;
                };
                // Evaluation failures are fatal even outside an explicit try.
                self.recovery_depth += 1;
                let stream = self.evaluate_to_stream(select);
                self.recovery_depth -= 1;
                for item in stream.items {
                    if !self.charge_result(0, 0, source) {
                        break;
                    }
                    out.0.push(ResultItem::Value(item, source.clone()));
                }
            }
            ResultInstruction::Attribute => {
                let Some((qualified_name, source_map)) =
                    self.render_constructor_name(attributes, "name", source, "result-attribute")
                else {
                    return;
                };
                if !cem_ml::validation::xpath::xpath_is_qname(&qualified_name) {
                    self.result_failure(
                        "name",
                        "Result attribute name must be a lexical QName",
                        &policy,
                    );
                    return;
                }
                let namespace = self.render_constructor_optional_text(attributes, "namespace");
                if namespace.as_deref() == Some(XMLNS) {
                    self.result_failure(
                        "namespace",
                        "Result attributes cannot declare namespaces",
                        &policy,
                    );
                    return;
                }
                if qualified_name == "xmlns" && namespace.is_none() {
                    self.result_failure(
                        "reserved_attribute",
                        "xmlns is reserved for namespace declarations",
                        &policy,
                    );
                    return;
                }
                let previous_target = std::mem::replace(
                    &mut self.expression_target,
                    hooks::ExpressionTarget::Attribute {
                        name: qualified_name.clone(),
                        contract: None,
                    },
                );
                let value = if let Some(value) = attributes.iter().find(|a| a.name == "value") {
                    let stream = self.output_attribute_stream(&TemplateAttribute {
                        name: qualified_name.clone(),
                        ..value.clone()
                    });
                    self.render_stream_text(&stream, &value.source_map)
                } else {
                    let mut buffer = ResultBuffer::default();
                    let mut ignored = Vec::new();
                    self.render_nodes_scoped(children, &mut buffer, &mut ignored);
                    let mut content = SimpleContent::default();
                    for item in buffer.0 {
                        if let ResultItem::Node(RenderPlanNode::Reference {
                            reference,
                            source_map,
                        }) = item {
                            // Attribute-scope expressions retain their native sequence.
                            // This scalar constructor consumes it using the usual
                            // expression text/node rules, preserving text adjacency.
                            let mut projected = ResultBuffer::default();
                            self.insert_values(
                                ItemStream::from_items(reference.values().to_vec()),
                                &source_map,
                                &mut projected,
                            );
                            for item in projected.0 {
                                self.simple_result(item, &mut content, &policy, 0);
                            }
                        } else {
                            self.simple_result(item, &mut content, &policy, 0);
                        }
                    }
                    content.parts.join(" ")
                };
                self.expression_target = previous_target;
                if !self.charge_result(value.len(), 0, source) {
                    return;
                }
                out.0.push(ResultItem::Attribute(RenderPlanAttribute {
                contract: None,
                    name: qualified_name.rsplit(':').next().unwrap_or("").into(),
                    qualified_name: Some(qualified_name),
                    namespace,
                    value: value.clone(),
                    value_stream: string_stream(value),
                    source_map,
                }));
            }
            ResultInstruction::Element | ResultInstruction::Document => {
                let name = if matches!(instruction, ResultInstruction::Element) {
                    let Some(name) =
                        self.render_constructor_name(attributes, "name", source, "result-element")
                    else {
                        return;
                    };
                    if !cem_ml::validation::xpath::xpath_is_qname(&name.0) {
                        self.result_failure(
                            "name",
                            "Result element name must be a lexical QName",
                            &policy,
                        );
                        return;
                    }
                    Some(name.0)
                } else {
                    None
                };
                let namespace = self.render_constructor_optional_text(attributes, "namespace");
                let mut buffer = ResultBuffer::default();
                let mut legacy_attributes = Vec::new();
                self.render_content_nodes(children, &mut buffer, &mut legacy_attributes);
                if !legacy_attributes.is_empty() {
                    self.result_failure(
                        "legacy_attribute",
                        "Use result-attribute inside native result constructors",
                        &policy,
                    );
                    return;
                }
                let (mut children, mut result_attributes) =
                    self.normalize_content(buffer, name.is_some(), &policy);
                if self.failure.is_some() || self.control_failed {
                    return;
                }
                if let Some(qualified_name) = name {
                    let tag = qualified_name.rsplit(':').next().unwrap_or("").into();
                    let mut node = RenderPlanNode::Element {
                        tag,
                        namespace,
                        qualified_name: Some(qualified_name),
                        attributes: std::mem::take(&mut result_attributes),
                        children: std::mem::take(&mut children),
                        source_map: source.clone(),
                    };
                    self.fixup_namespaces(&mut node, &BTreeMap::new(), &policy, 0);
                    out.push(node);
                } else {
                    out.0.push(ResultItem::Document(children));
                }
            }
        }
    }

    fn result_failure(&mut self, kind: &str, message: &str, policy: &Policy<'_>) {
        if self.failure.is_some() || self.control_failed {
            return;
        }
        let literal = |name: &str| {
            policy.attributes.iter().find_map(|a| match &a.value {
                Some(TemplateAttributeValue::Literal(value)) if a.name == name => {
                    Some(value.as_str())
                }
                _ => None,
            })
        };
        let mut diagnostic = render_diagnostic(
            &format!("cem.ql.result.{kind}"),
            message.into(),
            source_map_start(policy.source),
            policy.source.clone(),
        );
        if let (Some(namespace), Some(local)) =
            (literal("error-uri"), literal(&format!("error-{kind}")))
        {
            diagnostic = diagnostic.with_error_name(namespace, local);
        }
        diagnostic.uri = policy.origin.clone();
        if let Some(line) = literal("origin-line").and_then(|s| s.parse().ok()) {
            diagnostic.line = Some(line);
        }
        if let Some(column) = literal("origin-column").and_then(|s| s.parse().ok()) {
            diagnostic.column = Some(column);
        }
        if let Some(offset) = literal("origin-offset").and_then(|s| s.parse().ok()) {
            diagnostic.byte_offset = Some(offset);
            let length = literal("origin-length")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            diagnostic
                .source_map
                .as_mut()
                .expect("construction source")
                .frames
                .insert(
                    0,
                    SourceMapFrame {
                        source_id: SourceId(1),
                        span: FrameSpan::Single(ByteRange::new(offset, length)),
                        transform: TransformKind::ContentTypeTransform {
                            content_type: literal("origin-content-type")
                                .unwrap_or("text/cem-ml")
                                .into(),
                        },
                    },
                );
        }
        self.failure = Some(TemplateFailure {
            error: EvalError::TypeError("native result construction failed"),
            diagnostic: diagnostic.clone(),
        });
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn charge_result(
        &mut self,
        bytes: usize,
        depth: usize,
        source: &SourceMapStack,
    ) -> bool {
        if !self.poll_render(source) {
            return false;
        }
        self.result_work = self.result_work.saturating_add(1);
        self.result_bytes = self.result_bytes.saturating_add(bytes as u64);
        let limits = XPathEvaluationLimits::default();
        if depth > 128.min(self.evaluation_context.scope_policy.stack_depth as usize)
            || self.result_work > limits.max_work_units.unwrap_or(u64::MAX).min(100_000)
            || self.result_bytes > limits.max_text_bytes.unwrap_or(u64::MAX).min(self.evaluation_context.scope_policy.memory_bytes)
        {
            let diagnostic = render_diagnostic(
                "cem.ql.result.limit",
                "Native result construction exceeded its depth, work or text budget".into(),
                source_map_start(source),
                source.clone(),
            );
            self.failure = Some(TemplateFailure {
                error: EvalError::BudgetExceeded(crate::eval::BudgetAxis::ClosureSize),
                diagnostic: diagnostic.clone(),
            });
            self.control_failed = true;
            self.diagnostics.push(diagnostic);
            return false;
        }
        true
    }

    fn normalize_content(
        &mut self,
        buffer: ResultBuffer,
        element: bool,
        policy: &Policy<'_>,
    ) -> (Vec<RenderPlanNode>, Vec<RenderPlanAttribute>) {
        let mut output = Vec::new();
        let mut attributes = Vec::new();
        let mut atomic = false;
        for item in buffer.0 {
            self.consume_result(
                item,
                element,
                &mut output,
                &mut attributes,
                &mut atomic,
                policy,
                0,
            );
            if self.failure.is_some() || self.control_failed {
                break;
            }
        }
        (output, attributes)
    }

    #[allow(clippy::too_many_arguments)]
    fn consume_result(
        &mut self,
        item: ResultItem,
        element: bool,
        out: &mut Vec<RenderPlanNode>,
        attributes: &mut Vec<RenderPlanAttribute>,
        atomic: &mut bool,
        policy: &Policy<'_>,
        depth: usize,
    ) {
        if !self.charge_result(0, depth, policy.source) {
            return;
        }
        match item {
            ResultItem::Value(item, source) => {
                if let Some(value) = item.view().and_then(|v| v.downcast_ref::<XPathQueryItem>()) {
                    self.consume_xpath(
                        value.xpath_item(),
                        element,
                        out,
                        attributes,
                        atomic,
                        policy,
                        depth + 1,
                    );
                } else if let Some(node) = crate::eval::result_native_node(&item) {
                    match node {
                        Ok(node) => self.consume_native(
                            &node,
                            element,
                            out,
                            attributes,
                            atomic,
                            policy,
                            depth + 1,
                        ),
                        Err(_) => self.result_failure("item", "Invalid retained CEM node", policy),
                    }
                } else if let Item::Array(items) = item {
                    for item in items {
                        self.consume_result(
                            ResultItem::Value(item, source.clone()),
                            element,
                            out,
                            attributes,
                            atomic,
                            policy,
                            depth + 1,
                        );
                    }
                } else if matches!(item, Item::Atomic(_)) {
                    self.append_atomic(item_to_string(&item), source, out, atomic, depth);
                } else {
                    self.result_failure("item", "Result content requires nodes or atomic values; maps/functions/records cannot be copied", policy);
                }
            }
            ResultItem::Attribute(attribute) => {
                *atomic = false;
                if !element {
                    self.result_failure(
                        "document_attribute",
                        "A document cannot contain an attribute",
                        policy,
                    );
                } else if !out.is_empty() {
                    self.result_failure(
                        "attribute_order",
                        "An attribute cannot follow a child node",
                        policy,
                    );
                } else if let Some(old) = attributes
                    .iter_mut()
                    .find(|a| a.name == attribute.name && a.namespace == attribute.namespace)
                {
                    *old = attribute;
                } else {
                    attributes.push(attribute);
                }
            }
            ResultItem::Document(nodes) => {
                // Even an empty document separates adjacent atomic runs.
                *atomic = false;
                for node in nodes {
                    self.append_node(node, out, depth);
                }
            }
            ResultItem::Node(node) => {
                *atomic = false;
                self.append_node(node, out, depth);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn consume_xpath(
        &mut self,
        item: &XPathResultItem,
        element: bool,
        out: &mut Vec<RenderPlanNode>,
        attributes: &mut Vec<RenderPlanAttribute>,
        atomic: &mut bool,
        policy: &Policy<'_>,
        depth: usize,
    ) {
        if !self.charge_result(0, depth, policy.source) {
            return;
        }
        match item {
            XPathResultItem::Node { native_node: Some(node), .. } => self.consume_native(node, element, out, attributes, atomic, policy, depth + 1),
            XPathResultItem::Atomic { value, source_map } => {
                if !self.charge_result(value.lexical_value.len(), depth, source_map) { return; }
                match value.string_value() {
                    Ok(value) => self.append_atomic(value, source_map.clone(), out, atomic, depth),
                    Err(_) => {
                        self.result_failure("atomic", "Native atomic string conversion failed", policy);
                        if let Some(failure) = &mut self.failure {
                            failure.error = EvalError::Unsupported("native atomic conversion unavailable or over budget");
                        }
                        self.control_failed = true;
                    }
                }
            }
            XPathResultItem::Array { members, .. } => {
                for member in members { for item in &member.items { self.consume_xpath(item, element, out, attributes, atomic, policy, depth + 1); } }
            }
            _ => self.result_failure("item", "Result content requires retained nodes or atomic values; maps and functions cannot be copied", policy),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn consume_native(
        &mut self,
        node: &XPathNativeNode,
        element: bool,
        out: &mut Vec<RenderPlanNode>,
        attributes: &mut Vec<RenderPlanAttribute>,
        atomic: &mut bool,
        policy: &Policy<'_>,
        depth: usize,
    ) {
        if let Some(item) = self.copy_native(node, policy, depth) {
            self.consume_result(item, element, out, attributes, atomic, policy, depth);
        }
    }

    fn copy_native(
        &mut self,
        node: &XPathNativeNode,
        policy: &Policy<'_>,
        depth: usize,
    ) -> Option<ResultItem> {
        if !self.charge_result(
            node.semantic_value().len() + node.local_name().len() + node.namespace_uri().len(),
            depth,
            &node.source_map(),
        ) {
            return None;
        }
        let source_map = node.source_map();
        let namespace = (!node.namespace_uri().is_empty()).then(|| node.namespace_uri().to_owned());
        let name = node.local_name().to_owned();
        Some(match node.result_node_kind() {
            XPathResultNodeKind::Document | XPathResultNodeKind::Element => {
                let mut children = Vec::new();
                for child in node.child_nodes() {
                    if let Some(ResultItem::Node(child)) =
                        self.copy_native(&child, policy, depth + 1)
                    {
                        children.push(child);
                    }
                }
                if node.result_node_kind() == XPathResultNodeKind::Document {
                    ResultItem::Document(children)
                } else {
                    let mut attributes = Vec::new();
                    for attr in node.attribute_nodes() {
                        if let Some(ResultItem::Attribute(attr)) =
                            self.copy_native(&attr, policy, depth + 1)
                        {
                            attributes.push(attr);
                        }
                    }
                    ResultItem::Node(RenderPlanNode::Element {
                        tag: name.clone(),
                        qualified_name: Some(name),
                        namespace,
                        attributes,
                        children,
                        source_map,
                    })
                }
            }
            XPathResultNodeKind::Attribute => ResultItem::Attribute(RenderPlanAttribute {
                contract: cem_ml::value::xpath::CemValueXPathProjection::attribute_contract(&node).cloned().map(std::sync::Arc::new),
                name: name.clone(),
                qualified_name: Some(name),
                namespace,
                value: node.semantic_value().into(),
                value_stream: string_stream(node.semantic_value().into()),
                source_map,
            }),
            XPathResultNodeKind::Text => ResultItem::Node(RenderPlanNode::Text {
                text: node.semantic_value().into(),
                source_map,
            }),
            XPathResultNodeKind::Comment => ResultItem::Node(RenderPlanNode::Comment {
                text: node.semantic_value().into(),
                source_map,
            }),
            XPathResultNodeKind::ProcessingInstruction => {
                ResultItem::Node(RenderPlanNode::ProcessingInstruction {
                    target: name,
                    data: node.semantic_value().into(),
                    source_map,
                })
            }
            _ => {
                self.result_failure("item", "Unsupported semantic node kind", policy);
                return None;
            }
        })
    }

    fn append_atomic(
        &mut self,
        text: String,
        source_map: SourceMapStack,
        out: &mut Vec<RenderPlanNode>,
        atomic: &mut bool,
        depth: usize,
    ) {
        let text = if *atomic { format!(" {text}") } else { text };
        self.append_node(RenderPlanNode::Text { text, source_map }, out, depth);
        *atomic = true;
    }

    fn append_node(&mut self, node: RenderPlanNode, out: &mut Vec<RenderPlanNode>, depth: usize) {
        if let RenderPlanNode::Text { text, source_map } = node {
            if !self.charge_result(text.len(), depth, &source_map) || text.is_empty() {
                return;
            }
            if let Some(RenderPlanNode::Text {
                text: previous,
                source_map: previous_source,
            }) = out.last_mut()
            {
                previous.push_str(&text);
                for frame in source_map.frames {
                    if !previous_source.frames.contains(&frame) {
                        previous_source.frames.push(frame);
                    }
                }
            } else {
                out.push(RenderPlanNode::Text { text, source_map });
            }
        } else {
            out.push(node);
        }
    }

    fn simple_result(
        &mut self,
        item: ResultItem,
        content: &mut SimpleContent,
        policy: &Policy<'_>,
        depth: usize,
    ) {
        if !self.charge_result(0, depth, policy.source) {
            return;
        }
        match item {
            ResultItem::Value(item, _) => {
                if let Some(value) = item.view().and_then(|v| v.downcast_ref::<XPathQueryItem>()) {
                    self.simple_xpath(value.xpath_item(), content, policy, depth + 1);
                } else if let Some(Ok(node)) = crate::eval::result_native_node(&item) {
                    if let Some(item) = self.copy_native(&node, policy, depth + 1) {
                        self.simple_result(item, content, policy, depth + 1);
                    }
                } else if let Item::Array(items) = item {
                    content.text = false;
                    for item in items {
                        let mut member = SimpleContent::default();
                        self.simple_result(
                            ResultItem::Value(item, policy.source.clone()),
                            &mut member,
                            policy,
                            depth + 1,
                        );
                        for part in member.parts {
                            content.atomic(part);
                        }
                    }
                    content.text = false;
                } else if matches!(item, Item::Atomic(_)) {
                    content.atomic(item_to_string(&item));
                } else {
                    self.result_failure(
                        "item",
                        "Simple content cannot atomize a function, map or record",
                        policy,
                    );
                }
            }
            ResultItem::Node(
                RenderPlanNode::Text { text, .. } | RenderPlanNode::Cdata { text, .. },
            ) => content.text(text),
            ResultItem::Node(RenderPlanNode::Comment { text, .. }) => content.atomic(text),
            ResultItem::Node(RenderPlanNode::ProcessingInstruction { data, .. }) => {
                content.atomic(data)
            }
            ResultItem::Node(RenderPlanNode::Element { children, .. })
            | ResultItem::Document(children) => {
                content.atomic(render_plan_nodes_to_text(&children))
            }
            ResultItem::Node(RenderPlanNode::Reference { reference, .. }) => {
                content.atomic(render_plan_nodes_to_text(&expand_reference(&reference)))
            }
            ResultItem::Attribute(attribute) => content.atomic(attribute.value),
        }
    }

    fn simple_xpath(
        &mut self,
        item: &XPathResultItem,
        content: &mut SimpleContent,
        policy: &Policy<'_>,
        depth: usize,
    ) {
        if !self.charge_result(0, depth, policy.source) {
            return;
        }
        match item {
            XPathResultItem::Node {
                native_node: Some(node),
                ..
            } => {
                if let Some(item) = self.copy_native(node, policy, depth + 1) {
                    self.simple_result(item, content, policy, depth + 1);
                }
            }
            XPathResultItem::Atomic { value, .. } => match value.string_value() {
                Ok(value) => content.atomic(value),
                Err(_) => {
                    self.result_failure("atomic", "Native atomic string conversion failed", policy);
                    if let Some(failure) = &mut self.failure {
                        failure.error = EvalError::Unsupported(
                            "native atomic conversion unavailable or over budget",
                        );
                    }
                    self.control_failed = true;
                }
            },
            XPathResultItem::Array { members, .. } => {
                content.text = false;
                for member in members {
                    for item in &member.items {
                        // Atomization of array members occurs after text merging;
                        // members never merge with surrounding text nodes.
                        let mut member_content = SimpleContent::default();
                        self.simple_xpath(item, &mut member_content, policy, depth + 1);
                        for part in member_content.parts {
                            content.atomic(part);
                        }
                    }
                }
                content.text = false;
            }
            _ => self.result_failure(
                "item",
                "Simple content cannot atomize a function or map",
                policy,
            ),
        }
    }

    fn fixup_namespaces(
        &mut self,
        node: &mut RenderPlanNode,
        inherited: &BTreeMap<String, String>,
        policy: &Policy<'_>,
        depth: usize,
    ) {
        let RenderPlanNode::Element {
            tag,
            namespace,
            qualified_name: Some(name),
            attributes,
            children,
            source_map,
        } = node
        else {
            return;
        };
        if !self.charge_result(0, depth, source_map) {
            return;
        }
        let mut bindings = inherited.clone();
        bindings.insert("xml".into(), XML.into());
        // Rebuild declarations from expanded names, never from source syntax.
        attributes.retain(|a| a.namespace.as_deref() != Some(XMLNS));
        let mut declarations = Vec::new();
        let mut bind = |prefix: &str, uri: &str| {
            if bindings.get(prefix).map(String::as_str).unwrap_or("") != uri {
                bindings.insert(prefix.into(), uri.into());
                let qname = if prefix.is_empty() {
                    "xmlns".into()
                } else {
                    format!("xmlns:{prefix}")
                };
                declarations.push(RenderPlanAttribute {
                contract: None,
                    name: if prefix.is_empty() {
                        "xmlns".into()
                    } else {
                        prefix.into()
                    },
                    namespace: Some(XMLNS.into()),
                    qualified_name: Some(qname),
                    value: uri.into(),
                    value_stream: string_stream(uri.into()),
                    source_map: source_map.clone(),
                });
            }
        };
        let uri = namespace.as_deref().unwrap_or("");
        if uri == XML && !name.starts_with("xml:") {
            *name = format!("xml:{tag}");
        }
        let prefix = name.split_once(':').map_or("", |(p, _)| p).to_owned();
        if !valid_namespace(&prefix, uri) {
            self.result_failure("namespace", "Invalid element namespace binding", policy);
            return;
        }
        bind(&prefix, uri);
        let mut used = BTreeMap::from([(prefix, uri.to_owned())]);
        for attribute in attributes.iter_mut() {
            let Some(qname) = &mut attribute.qualified_name else {
                continue;
            };
            let uri = attribute.namespace.as_deref().unwrap_or("");
            let mut prefix = qname.split_once(':').map_or("", |(p, _)| p).to_owned();
            if uri == XML {
                prefix = "xml".into();
            } else if !uri.is_empty()
                && (prefix.is_empty()
                    || prefix == "xmlns"
                    || prefix == "xml"
                    || used.get(&prefix).is_some_and(|old| old != uri))
            {
                let mut index = 1;
                while used.contains_key(&format!("ns{index}")) {
                    index += 1;
                }
                prefix = format!("ns{index}");
            }
            if !valid_namespace(&prefix, uri) || (attribute.name == "xmlns" && uri.is_empty()) {
                self.result_failure("namespace", "Invalid attribute namespace binding", policy);
                return;
            }
            *qname = if prefix.is_empty() {
                attribute.name.clone()
            } else {
                format!("{prefix}:{}", attribute.name)
            };
            if !prefix.is_empty() {
                bind(&prefix, uri);
                used.insert(prefix, uri.into());
            }
        }
        declarations.append(attributes);
        *attributes = declarations;
        for child in children {
            self.fixup_namespaces(child, &bindings, policy, depth + 1);
        }
    }
}

fn valid_namespace(prefix: &str, uri: &str) -> bool {
    uri != XMLNS
        && prefix != "xmlns"
        && (prefix == "xml") == (uri == XML)
        && (prefix.is_empty() || !uri.is_empty())
}

pub(super) fn validate_instruction(
    instruction: ResultInstruction,
    attributes: &[TemplateAttribute],
    children: &[TemplateNode],
) -> Option<&'static str> {
    let allowed: &[&str] = match instruction {
        ResultInstruction::Sequence => &["select"],
        ResultInstruction::Element => &["name", "namespace"],
        ResultInstruction::Attribute => &["name", "namespace", "value"],
        ResultInstruction::Document => &[],
    };
    let mut names = BTreeSet::new();
    for attribute in attributes {
        if !names.insert(&attribute.name) {
            return Some("Duplicate result control attribute");
        }
        let policy = matches!(
            attribute.name.as_str(),
            "origin-uri"
                | "origin-line"
                | "origin-column"
                | "origin-offset"
                | "origin-length"
                | "origin-content-type"
        ) || attribute.name.starts_with("error-");
        if !policy && !allowed.contains(&attribute.name.as_str()) {
            return Some("Unsupported result control attribute");
        }
        if attribute.name.starts_with("error-")
            && !matches!(attribute.value, Some(TemplateAttributeValue::Literal(_)))
        {
            return Some("Result error names must be static");
        }
    }
    if matches!(instruction, ResultInstruction::Sequence) {
        if !children.is_empty() {
            return Some("result-sequence requires empty content");
        }
        if !attributes.iter().any(|a| {
            a.name == "select" && matches!(a.value, Some(TemplateAttributeValue::Expression(_)))
        }) {
            return Some("result-sequence requires select");
        }
    }
    if matches!(
        instruction,
        ResultInstruction::Element | ResultInstruction::Attribute
    ) && !attributes
        .iter()
        .any(|a| a.name == "name" && a.value.is_some())
    {
        return Some("Result constructor requires name");
    }
    None
}

#[derive(Default)]
struct SimpleContent {
    parts: Vec<String>,
    text: bool,
}
impl SimpleContent {
    fn text(&mut self, value: String) {
        if value.is_empty() {
            return;
        }
        if self.text {
            self.parts
                .last_mut()
                .expect("preceding text")
                .push_str(&value);
        } else {
            self.parts.push(value);
        }
        self.text = true;
    }
    fn atomic(&mut self, value: String) {
        self.parts.push(value);
        self.text = false;
    }
}
