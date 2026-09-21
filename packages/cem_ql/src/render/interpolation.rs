//! Text-context conversion. Native nodes remain nodes in queries, template
//! parameters and explicit result construction; only interpolation reads text.
use super::*;
use crate::eval::QueryItemViewKind;

impl PlanRenderer<'_> {
    pub(super) fn render_stream_text(
        &mut self,
        stream: &ItemStream,
        source: &SourceMapStack,
    ) -> String {
        let Some((control, scope)) = self.control.clone() else { unreachable!("renderer always owns control") };
        let result = crate::eval::value_control::ValueControl::new(&control, scope,
            self.evaluation_context.scope, cem_ml::value::artifact::CemValueArtifactLimits::default())
            .and_then(|mut budget| budget.text(&stream.items));
        match result {
            Ok(text) => {
                if !self.charge_result(text.len(), 0, source) { return String::new(); }
                match control.charge_memory(scope, text.len() as u64, Some(source.clone())) {
                    Ok(permit) => { self.text_memory.push(permit); text }
                    Err(error) => { self.accept_control_check(Err(error), source); String::new() }
                }
            }
            Err(error) => { self.accept_control_check(Err(error), source); String::new() }
        }
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
