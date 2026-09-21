//! Native attribute content and shared schema conversion/validation.
use super::*;
use cem_ml::schema::document_model::{
    convert_attribute_value, AttributeValueContract, TypedAttributeValue,
};

/// Final string projection. The authoritative sequence stays available to
/// native consumers; strings containing markup are never parsed as structure.
pub fn project_attribute_value(attribute: &RenderPlanAttribute) -> String {
    if let Some(content_type) = attribute
        .contract
        .as_ref()
        .and_then(|c| c.content_type.as_deref())
    {
        if matches!(content_type, "text/html" | "application/xml" | "text/xml") {
            let plan = RenderPlan {
                nodes: vec![RenderPlanNode::Reference {
                    reference: cem_ml::value::CemReference::new(
                        attribute.value_stream.items.clone(),
                    ),
                    source_map: attribute.source_map.clone(),
                }],
                host_attribute_updates: Vec::new(),
                diagnostics: Vec::new(),
            };
            return if content_type == "text/html" {
                render_plan_to_html(&plan)
            } else {
                render_plan_to_xml_with_source_map(&plan).rendered
            };
        }
    }
    fn append(items: &[Item], result: &mut String) {
        for item in items {
            if let Some(targets) = crate::eval::values::reference_values(item) {
                append(targets, result);
            } else if let Some(view) = item
                .view()
                .filter(|v| v.kind() == crate::eval::QueryItemViewKind::Node)
            {
                if let Ok(fragments) = view.text_fragments(QueryContextScope(0)) {
                    for fragment in fragments {
                        if let Ok(fragment) = fragment {
                            result.push_str(fragment);
                        }
                    }
                }
            } else {
                result.push_str(&item_to_string(item));
            }
        }
    }
    let mut result = String::new();
    append(&attribute.value_stream.items, &mut result);
    result
}

#[derive(Debug, Clone)]
struct TypedValue(TypedAttributeValue);
impl crate::eval::QueryItemView for TypedValue {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.typed-atomic"
    }
    fn identity(&self) -> String {
        format!("{}:{}", self.0.datatype, self.0.lexical)
    }
    fn kind(&self) -> crate::eval::QueryItemViewKind {
        crate::eval::QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<AtomValue> {
        Some(AtomValue::String(self.0.lexical.clone()))
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        match name {
            "datatype" => Some(string_stream(self.0.datatype.clone()).items),
            "value" => Some(string_stream(self.0.lexical.clone()).items),
            _ => None,
        }
    }
}

impl PlanRenderer<'_> {
    pub(super) fn output_attribute_stream(&mut self, attribute: &TemplateAttribute) -> ItemStream {
        match &attribute.value {
            None => string_stream(String::new()),
            Some(TemplateAttributeValue::Literal(value)) => string_stream(value.clone()),
            Some(TemplateAttributeValue::Expression(expression)) => {
                let value = self.evaluate_to_stream(expression);
                self.apply_expression_hook(
                    value,
                    "attribute",
                    Some(&attribute.name),
                    &attribute.source_map,
                )
            }
            Some(TemplateAttributeValue::Template(parts)) => {
                let mut result = ItemStream::empty();
                for part in parts {
                    if !self.poll_render(&attribute.source_map) {
                        break;
                    }
                    match part {
                        TemplateAttributePart::Literal(literal) => {
                            result.items.extend(string_stream(literal.clone()).items)
                        }
                        TemplateAttributePart::Expression(expression) => {
                            let value = self.evaluate_to_stream(expression);
                            result.append_stream(self.apply_expression_hook(
                                value,
                                "attribute",
                                Some(&attribute.name),
                                &attribute.source_map,
                            ));
                        }
                    }
                }
                result
            }
        }
    }

    pub(super) fn value_contract(&self, type_name: &str) -> AttributeValueContract {
        self.value_types
            .get(type_name)
            .cloned()
            .unwrap_or_else(|| AttributeValueContract {
                model: cem_ml::schema::document_model::AttributeModel {
                    value_type: Some(type_name.into()),
                    ..Default::default()
                },
                ..Default::default()
            })
    }

    pub(super) fn convert_values(
        &mut self,
        values: ItemStream,
        contract: &AttributeValueContract,
        source: &SourceMapStack,
    ) -> ItemStream {
        if matches!(
            contract.model.value_type.as_deref(),
            Some("any") | Some("node")
        ) {
            if contract.model.value_type.as_deref() == Some("node")
                && values.items.iter().any(|item| {
                    !item
                        .view()
                        .is_some_and(|v| v.kind() == crate::eval::QueryItemViewKind::Node)
                })
            {
                self.attribute_contract_failure("Node content requires native nodes", source);
                return ItemStream::empty();
            }
            return values;
        }
        let lexical = self.render_stream_text(&values, source);
        match convert_attribute_value(&lexical, contract, source) {
            Ok(value) => {
                let item = match value.datatype.as_str() {
                    "integer" => match value.lexical.parse::<i64>() {
                        Ok(value) => Item::Atomic(AtomValue::Integer(value)),
                        Err(_) => Item::native(TypedValue(value)),
                    },
                    "number" | "decimal" => Item::Atomic(AtomValue::Decimal(value.lexical)),
                    "boolean" => Item::Atomic(AtomValue::Boolean(value.lexical == "true")),
                    "string" => Item::Atomic(AtomValue::String(value.lexical)),
                    _ => Item::native(TypedValue(value)),
                };
                ItemStream::once(item)
            }
            Err(diagnostics) => {
                self.recovery_depth += 1;
                for diagnostic in diagnostics {
                    self.template_failure(diagnostic);
                }
                self.recovery_depth -= 1;
                ItemStream::empty()
            }
        }
    }

    fn attribute_contract_failure(&mut self, message: &str, source: &SourceMapStack) {
        self.recovery_depth += 1;
        self.template_failure(render_diagnostic(
            "cem.value.contract",
            message.into(),
            source_map_start(source),
            source.clone(),
        ));
        self.recovery_depth -= 1;
    }

    pub(super) fn constructed_attribute_value(
        &mut self,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        source: &SourceMapStack,
    ) -> Option<RenderPlanAttribute> {
        let (name, name_source_map) =
            self.render_constructor_name(attributes, "name", source, "attribute")?;
        let namespace = self.render_constructor_optional_text(attributes, "namespace");
        let mut contract = self
            .attribute_contracts
            .get(&name)
            .cloned()
            .unwrap_or_default();
        if let Some(type_name) = literal_template_attribute(attributes, "type") {
            contract = self.value_contract(&type_name);
        }
        contract.model.name = name.clone();
        contract.content_type =
            literal_template_attribute(attributes, "content-type").or(contract.content_type);
        if contract.content_type.as_deref().is_some_and(|ct| {
            !matches!(
                ct,
                "text/plain" | "text/html" | "application/xml" | "text/xml"
            )
        }) {
            self.attribute_contract_failure(
                "No native attribute projector is registered for this content type",
                source,
            );
            return None;
        }
        for attribute in attributes {
            let Some(value) = literal_template_attribute(attributes, &attribute.name) else {
                continue;
            };
            let field = match attribute.name.as_str() {
                "pattern" => &mut contract.model.pattern,
                "minInclusive" => &mut contract.model.min_inclusive,
                "maxInclusive" => &mut contract.model.max_inclusive,
                "minExclusive" => &mut contract.model.min_exclusive,
                "maxExclusive" => &mut contract.model.max_exclusive,
                "minLength" => &mut contract.model.min_length,
                "maxLength" => &mut contract.model.max_length,
                "length" => &mut contract.model.length,
                "totalDigits" => &mut contract.model.total_digits,
                "fractionDigits" => &mut contract.model.fraction_digits,
                "whiteSpace" => &mut contract.model.white_space,
                _ => continue,
            };
            *field = Some(value);
        }
        let previous_contract = self
            .active_attribute_contract
            .replace(std::sync::Arc::new(contract.clone()));
        let mut values = if let Some(value) = attributes.iter().find(|a| a.name == "value") {
            self.output_attribute_stream(&TemplateAttribute {
                name: name.clone(),
                ..value.clone()
            })
        } else {
            let mut buffer = ResultBuffer::default();
            let capture = self.capture_depth.replace(self.render_scope_depth + 1);
            self.render_nodes_scoped(children, &mut buffer, &mut Vec::new());
            self.capture_depth = capture;
            self.hook_result_values(buffer, source)
        };
        // Rich values retain their sequence; ordinary string attributes retain
        // it too unless a scalar type/constraint explicitly requests conversion.
        if contract.model.value_type.is_some() || contract.model.pattern.is_some() {
            values = self.convert_values(values, &contract, source);
        }
        // A local conversion never relaxes a destination supplied by the host.
        if let Some(destination) = self.attribute_contracts.get(&name).cloned() {
            values = self.convert_values(values, &destination, source);
        }
        self.active_attribute_contract = previous_contract;
        let value = if values.items.iter().any(|item| {
            item.view()
                .is_some_and(|v| v.kind() == crate::eval::QueryItemViewKind::Node)
        }) {
            String::new()
        } else {
            self.render_stream_text(&values, source)
        };
        Some(RenderPlanAttribute {
            name,
            namespace,
            qualified_name: None,
            value,
            value_stream: values,
            contract: Some(std::sync::Arc::new(contract)),
            source_map: name_source_map,
        })
    }
}
