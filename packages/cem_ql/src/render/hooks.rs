//! Expression hooks have lexical activation and whole-sequence input.
use super::*;
use construction::ResultItem;

#[derive(Clone)]
pub(super) struct ExpressionHook {
    id: usize,
    into: String,
    test: Option<CompiledTemplateExpression>,
    priority: i64,
    returns: Option<String>,
    body: Vec<TemplateNode>,
    bindings: BTreeMap<String, ItemStream>,
}

pub(super) fn is_expression_hook(node: &TemplateNode) -> bool {
    matches!(node, TemplateNode::Element { tag, attributes, .. }
        if local_template_name(tag) == "template" && literal_template_attribute(attributes, "on").as_deref() == Some("expression"))
}

impl PlanRenderer<'_> {
    pub(super) fn register_module_hooks(&mut self, nodes: &[TemplateNode]) {
        for node in nodes {
            if let TemplateNode::Element { tag, children, .. } = node {
                if local_template_name(tag) == "module" {
                    self.register_module_hooks(children);
                } else if is_expression_hook(node) {
                    self.register_hook(node);
                }
            }
        }
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
            id: node as *const TemplateNode as usize,
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
            bindings: self.evaluation_context.policy_bindings.clone(),
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
        let scopes = self.hook_scopes.clone();
        let previous = self.evaluation_context.policy_bindings.clone();
        let previous_focus = self.evaluation_context.current_item.clone();
        let mut result = input.clone();
        'scopes: for scope in scopes.iter().rev() {
            let mut candidates: Vec<_> = scope
                .iter()
                .enumerate()
                .filter(|(_, h)| h.into == into && !self.active_hooks.contains(&h.id))
                .collect();
            candidates.sort_by_key(|(order, hook)| std::cmp::Reverse((hook.priority, *order)));
            for (_, hook) in candidates {
                self.evaluation_context.policy_bindings = hook.bindings.clone();
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
                                            self.active_attribute_contract
                                                .as_ref()
                                                .and_then(|c| c.model.value_type.clone())
                                                .map(string_stream)
                                                .unwrap_or_default()
                                                .items,
                                        ),
                                        (
                                            "content_type".into(),
                                            self.active_attribute_contract
                                                .as_ref()
                                                .and_then(|c| c.content_type.clone())
                                                .map(string_stream)
                                                .unwrap_or_default()
                                                .items,
                                        ),
                                        (
                                            "pattern".into(),
                                            self.active_attribute_contract
                                                .as_ref()
                                                .and_then(|c| c.model.pattern.clone())
                                                .map(string_stream)
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
                if hook
                    .test
                    .as_ref()
                    .is_some_and(|test| !self.test_is_truthy(Some(test)))
                {
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
                self.active_hooks.push(hook.id);
                self.call_depth += 1;
                let previous_capture = self.capture_depth.replace(self.render_scope_depth + 1);
                let mut buffer = ResultBuffer::default();
                self.render_nodes_scoped(&hook.body, &mut buffer, &mut Vec::new());
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
        self.evaluation_context.policy_bindings = previous;
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
