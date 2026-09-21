//! Text-context conversion. Native nodes remain nodes in queries, template
//! parameters and explicit result construction; only interpolation reads text.
use super::*;
use crate::eval::{QueryItemViewKind, QueryNodeAccessError};

impl PlanRenderer<'_> {
    pub(super) fn render_stream_text(
        &mut self,
        stream: &ItemStream,
        source: &SourceMapStack,
    ) -> String {
        if !self.poll_render(source) || !self.force_render(source) {
            return String::new();
        }
        let mut text = String::new();
        for item in &stream.items {
            if !self.poll_render(source) {
                return String::new();
            }
            let view = match item {
                Item::Native(view) if view.kind() == QueryItemViewKind::Node => view,
                _ => {
                    text.push_str(&item_to_string(item));
                    continue;
                }
            };
            // Never fall back to atom() here: host scope checks and incremental
            // extraction must precede allocating the node's entire string value.
            let mut fragments = match view.text_fragments(self.evaluation_context.scope) {
                Ok(fragments) => fragments,
                Err(error) => {
                    self.text_access_failure(error, source);
                    return String::new();
                }
            };
            loop {
                if !self.poll_render(source) {
                    return String::new();
                }
                let Some(fragment) = fragments.next() else {
                    break;
                };
                let fragment = match fragment {
                    Ok(fragment) => fragment,
                    Err(error) => {
                        self.text_access_failure(error, source);
                        return String::new();
                    }
                };
                if !self.charge_result(fragment.len(), 0, source) {
                    return String::new();
                }
                if !fragment.is_empty() {
                    if let Some((control, scope)) = &self.control {
                        match control.charge_memory(
                            *scope,
                            fragment.len() as u64,
                            Some(source.clone()),
                        ) {
                            Ok(permit) => self.text_memory.push(permit),
                            Err(error) => {
                                self.accept_control_check(Err(error), source);
                                return String::new();
                            }
                        }
                    }
                    text.push_str(fragment);
                }
            }
        }
        if self.force_render(source) {
            text
        } else {
            String::new()
        }
    }

    fn text_access_failure(&mut self, error: QueryNodeAccessError, source: &SourceMapStack) {
        let (code, message) = match error {
            QueryNodeAccessError::ScopeViolation => (
                "cem.ql.scope_violation",
                "Node text access would leave the active query scope",
            ),
            QueryNodeAccessError::Unsupported => (
                "cem.ql.unsupported",
                "Node view does not provide text-context conversion",
            ),
        };
        let diagnostic = render_diagnostic(
            code,
            message.into(),
            source_map_start(source),
            source.clone(),
        );
        self.failure = Some(TemplateFailure {
            error: EvalError::Unsupported("node text access failed"),
            diagnostic: diagnostic.clone(),
        });
        self.control_failed = true;
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn render_attribute_stream(&mut self, attribute: &TemplateAttribute) -> ItemStream {
        match &attribute.value {
            Some(TemplateAttributeValue::Expression(expression)) => {
                self.evaluate_to_stream(expression)
            }
            _ => self.render_attribute_value(attribute).1,
        }
    }

    pub(super) fn render_call_attribute(
        &mut self,
        attribute: &TemplateAttribute,
    ) -> Option<RenderPlanAttribute> {
        if attribute.name.starts_with("with:")
            && matches!(attribute.value, Some(TemplateAttributeValue::Expression(_)))
        {
            let value_stream = self.render_attribute_stream(attribute);
            let value = value_stream
                .items
                .iter()
                .filter(|item| {
                    !item
                        .view()
                        .is_some_and(|view| view.kind() == QueryItemViewKind::Node)
                })
                .map(item_to_string)
                .collect();
            return Some(RenderPlanAttribute {
                contract: None,
                name: attribute.name.clone(),
                namespace: None,
                qualified_name: None,
                value,
                value_stream,
                source_map: attribute.source_map.clone(),
            });
        }
        self.render_attribute(attribute)
    }
}
