//! Expression hooks have lexical activation and whole-sequence input.
use super::*;
use construction::ResultItem;

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod tests;

/// Output destination is independent of lexical scopes and direct hook returns.
#[derive(Debug, Clone, Default)]
pub(super) enum ExpressionTarget {
    #[default]
    Content,
    Attribute {
        name: String,
        contract: Option<std::sync::Arc<cem_ml::schema::document_model::AttributeValueContract>>,
    },
}

impl ExpressionTarget {
    fn contract(&self) -> Option<&cem_ml::schema::document_model::AttributeValueContract> {
        match self {
            Self::Attribute { contract, .. } => contract.as_deref(),
            Self::Content => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ExpressionHook {
    id: String,
    into: String,
    test: Option<CompiledTemplateExpression>,
    priority: i64,
    returns: Option<String>,
    body: Vec<TemplateNode>,
    bindings: BTreeMap<String, ItemStream>,
}

/// Captured caller behavior is runtime state, never serialized as document data.
#[derive(Debug, Clone, Default)]
pub struct ExpressionScope {
    pub(super) scopes: Vec<Vec<ExpressionHook>>,
    pub(super) active: Vec<String>,
    pub(super) focus: Option<Item>,
    pub(super) call_depth: usize,
    pub(super) target: ExpressionTarget,
}

pub(super) fn module_hook_nodes(nodes: &[TemplateNode]) -> Vec<TemplateNode> {
    let mut hooks = Vec::new();
    for node in nodes {
        if is_expression_hook(node) {
            hooks.push(node.clone());
        } else if let TemplateNode::Element {
            tag,
            attributes,
            children,
            ..
        } = node
        {
            if local_template_name(tag) == "module"
                && literal_template_attribute(attributes, "__cem-module-uri").is_none()
            {
                hooks.extend(module_hook_nodes(children));
            }
        }
    }
    hooks
}
pub(super) fn imported_module_defaults(node: &TemplateNode) -> Option<Vec<TemplateNode>> {
    let TemplateNode::Element {
        tag,
        attributes,
        children,
        ..
    } = node
    else {
        return None;
    };
    (local_template_name(tag) == "module"
        && literal_template_attribute(attributes, "__cem-module-uri").is_some())
    .then(|| module_hook_nodes(children))
}

pub(super) fn is_expression_hook(node: &TemplateNode) -> bool {
    matches!(node, TemplateNode::Element { tag, attributes, .. }
        if local_template_name(tag) == "template" && literal_template_attribute(attributes, "on").as_deref() == Some("expression"))
}

impl PlanRenderer<'_> {
    /// Constructed nodes own their content destination, including when built
    /// inside an attribute payload or an expression hook's return body.
    pub(super) fn render_content_nodes(
        &mut self,
        nodes: &[TemplateNode],
        out: &mut ResultBuffer,
        attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        let previous = std::mem::take(&mut self.expression_target);
        let capture = self.capture_depth.take();
        self.render_nodes_scoped(nodes, out, attributes);
        self.capture_depth = capture;
        self.expression_target = previous;
    }

    pub(super) fn insert_expression_values(
        &mut self,
        input: ItemStream,
        source: &SourceMapStack,
        out: &mut ResultBuffer,
    ) {
        match self.expression_target.clone() {
            ExpressionTarget::Content => {
                let values = self.apply_expression_hook(input, "content", None, source);
                self.insert_values(values, source, out);
            }
            ExpressionTarget::Attribute { name, .. } => {
                let values = self.apply_expression_hook(input, "attribute", Some(&name), source);
                // A reference preserves atomic types as well as native nodes,
                // even across a callee's RenderPlan boundary.
                out.push(RenderPlanNode::Reference {
                    reference: cem_ml::value::CemReference::new(values.items),
                    source_map: source.clone(),
                });
            }
        }
    }

    pub(super) fn register_module_hooks(&mut self, nodes: &[TemplateNode]) {
        for node in nodes {
            if let TemplateNode::Element {
                tag,
                attributes,
                children,
                ..
            } = node
            {
                if local_template_name(tag) == "module"
                    && literal_template_attribute(attributes, "__cem-module-uri").is_none()
                {
                    for hook in module_hook_nodes(children) {
                        self.register_hook(&hook);
                    }
                }
            }
        }
    }

    pub(super) fn render_template_body(
        &mut self,
        body: &TemplateBody,
        out: &mut ResultBuffer,
        attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        // Callee module defaults are farther away than every active caller scope.
        self.hook_scopes.push(Vec::new());
        for hook in &body.defaults {
            self.register_hook(hook);
        }
        let defaults = self.hook_scopes.pop().expect("default scope");
        self.hook_scopes.insert(0, defaults);
        self.render_nodes_scoped(&body.nodes, out, attributes);
        self.hook_scopes.remove(0);
    }

    pub(super) fn register_hook(&mut self, node: &TemplateNode) {
        let TemplateNode::Element {
            attributes,
            children,
            source_map,
            ..
        } = node
        else {
            return;
        };
        let into = literal_template_attribute(attributes, "into").unwrap_or_default();
        if !matches!(into.as_str(), "content" | "attribute") {
            self.recovery_depth += 1;
            self.template_failure(render_diagnostic(
                "cem.ql.render.hook_target",
                "Expression hooks require @into=content or @into=attribute".into(),
                source_map_start(source_map),
                source_map.clone(),
            ));
            self.recovery_depth -= 1;
            return;
        }
        let hook = ExpressionHook {
            id: literal_template_attribute(attributes, "__cem-hook-id")
                .unwrap_or_else(|| format!("{:?}", node)),
            into,
            test: attributes.iter().find_map(|a| match (&*a.name, &a.value) {
                ("match", Some(TemplateAttributeValue::Expression(value))) => Some(value.clone()),
                _ => None,
            }),
            priority: literal_template_attribute(attributes, "priority")
                .and_then(|p| p.parse().ok())
                .unwrap_or(0),
            returns: literal_template_attribute(attributes, "returns"),
            body: template_body_nodes(children),
            bindings: {
                #[cfg(test)]
                let _profile = crate::compile_profile::Span::new("copy/hook-capture");
                self.evaluation_context.policy_bindings.clone()
            },
        };
        self.hook_scopes.last_mut().expect("root scope").push(hook);
    }

    pub(super) fn apply_expression_hook(
        &mut self,
        input: ItemStream,
        into: &str,
        attribute: Option<&str>,
        source: &SourceMapStack,
    ) -> ItemStream {
        // With no eligible handler, preserve the owned stream and leave the
        // environment in place. Match predicates are observable and must still
        // run through normal dispatch whenever a handler is eligible.
        if !self.hook_scopes.iter().flatten().any(|hook| {
            hook.into == into && !self.active_hooks.contains(&hook.id)
        }) {
            return input;
        }
        let scopes = {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/hook-scopes");
            self.hook_scopes.clone()
        };
        let previous = {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/hook-caller");
            self.evaluation_context.policy_bindings.clone()
        };
        let previous_focus = self.evaluation_context.current_item.clone();
        let mut result = {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/hook-default-input");
            input.clone()
        };
        'scopes: for scope in scopes.iter().rev() {
            let mut candidates: Vec<_> = scope
                .iter()
                .enumerate()
                .filter(|(_, h)| h.into == into && !self.active_hooks.contains(&h.id))
                .collect();
            candidates.sort_by_key(|(order, hook)| std::cmp::Reverse((hook.priority, *order)));
            for (_, hook) in candidates {
                {
                    #[cfg(test)]
                    let _profile = crate::compile_profile::Span::new("scope/hook-install");
                    self.evaluation_context.policy_bindings = {
                        #[cfg(test)]
                        let _profile = crate::compile_profile::Span::new("copy/hook-bindings");
                        hook.bindings.clone()
                    };
                }
                self.evaluation_context
                    .policy_bindings
                    .insert("value".into(), input.clone());
                self.evaluation_context.policy_bindings.insert(
                    "context".into(),
                    ItemStream::once(Item::Record(BTreeMap::from([
                        ("into".into(), string_stream(into.into()).items),
                        (
                            "attribute".into(),
                            attribute
                                .map(|name| {
                                    vec![Item::Record(BTreeMap::from([
                                        ("name".into(), string_stream(name.into()).items),
                                        (
                                            "type".into(),
                                            self.expression_target
                                                .contract()
                                                .and_then(|c| c.model.value_type.clone())
                                                .map(string_stream)
                                                .unwrap_or_default()
                                                .items,
                                        ),
                                        (
                                            "content_type".into(),
                                            self.expression_target
                                                .contract()
                                                .and_then(|c| c.content_type.clone())
                                                .map(string_stream)
                                                .unwrap_or_default()
                                                .items,
                                        ),
                                        (
                                            "pattern".into(),
                                            self.expression_target
                                                .contract()
                                                .map(|c| ItemStream::from_items(c.models().filter_map(|m| m.pattern.clone())
                                                    .map(|pattern| Item::Atomic(AtomValue::String(pattern))).collect()))
                                                .unwrap_or_default()
                                                .items,
                                        ),
                                    ]))]
                                })
                                .unwrap_or_default(),
                        ),
                    ]))),
                );
                self.evaluation_context.current_item =
                    Some(crate::eval::values::reference(input.items.clone()));
                self.recovery_depth += 1;
                let matches = hook
                    .test
                    .as_ref()
                    .is_none_or(|test| self.test_is_truthy(Some(test)));
                self.recovery_depth -= 1;
                if self.failure.is_some() || self.control_failed {
                    result = ItemStream::empty();
                    break 'scopes;
                }
                if !matches {
                    continue;
                }
                if self.call_depth >= self.max_call_depth {
                    self.recovery_depth += 1;
                    self.template_failure(render_diagnostic(
                        "cem.ql.render.hook_depth",
                        "Expression hook call depth exceeded".into(),
                        source_map_start(source),
                        source.clone(),
                    ));
                    self.recovery_depth -= 1;
                    result = ItemStream::empty();
                    break 'scopes;
                }
                self.active_hooks.push(hook.id.clone());
                self.call_depth += 1;
                let previous_capture = self.capture_depth.replace(self.render_scope_depth + 1);
                let mut buffer = ResultBuffer::default();
                self.recovery_depth += 1;
                self.render_nodes_scoped(&hook.body, &mut buffer, &mut Vec::new());
                self.recovery_depth -= 1;
                self.capture_depth = previous_capture;
                self.call_depth -= 1;
                self.active_hooks.pop();
                result = self.hook_result_values(buffer, source);
                if let Some(returns) = &hook.returns {
                    let contract = self.value_contract(returns);
                    result = self.convert_values(result, &contract, source);
                }
                break 'scopes;
            }
        }
        {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("scope/hook-restore");
            self.evaluation_context.policy_bindings = previous;
        }
        self.evaluation_context.current_item = previous_focus;
        result
    }
    pub(super) fn hook_result_values(
        &mut self,
        buffer: ResultBuffer,
        source: &SourceMapStack,
    ) -> ItemStream {
        let mut result = ItemStream::empty();
        for item in buffer {
            match item {
                ResultItem::Value(item, _) => result.items.push(item),
                ResultItem::Node(RenderPlanNode::Text { text, .. }) => {
                    result.items.extend(string_stream(text).items)
                }
                ResultItem::Node(RenderPlanNode::Reference { reference, .. }) => {
                    result.items.extend(reference.values().iter().cloned())
                }
                ResultItem::Node(node) => result
                    .items
                    .extend(crate::eval::output::output_nodes(vec![node]).items),
                ResultItem::Document(nodes) => result
                    .items
                    .extend(crate::eval::output::output_nodes(nodes).items),
                _ => {
                    self.recovery_depth += 1;
                    self.template_failure(render_diagnostic(
                        "cem.ql.render.hook_result",
                        "Hook result requires native values".into(),
                        source_map_start(source),
                        source.clone(),
                    ));
                    self.recovery_depth -= 1;
                }
            }
        }
        result
    }
}
