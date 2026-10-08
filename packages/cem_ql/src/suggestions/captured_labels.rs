//! Local inert XML templates retain their source owner and namespace context.
//! Their only runtime input is the explicit native row/group parameter.
use super::{labels::*, SuggestionsError, SuggestionsView};
use crate::{
    eval::{retained_cem_node, AtomValue, Item, ItemStream, QueryContextScope, RetainedCemNode},
    render::{
        compile_template, render_compiled_template, CompileTemplateOptions, RenderPlan,
        RenderPlanNode, ResultInstruction, TemplateArtifact, TemplateAttribute,
        TemplateAttributeValue, TemplateData, TemplateNode,
    },
};
use cem_ml::{
    parser::CemAstNode, source_map::SourceMapStack, value::artifact::CemValueArtifactLimits,
};
use std::collections::BTreeMap;

pub struct CapturedLabel {
    source: RetainedCemNode,
    artifact: TemplateArtifact,
    parameter: &'static str,
}
fn failure(message: impl Into<String>, source: &SourceMapStack) -> SuggestionsError {
    SuggestionsError {
        code: "cem.suggestions.label_invalid",
        message: message.into(),
        source: source.clone(),
    }
}
fn field_text(item: &Item, name: &str) -> Option<String> {
    match item.view()?.field(name)?.first()?.atom()? {
        AtomValue::String(s) => Some(s),
        _ => None,
    }
}
impl CapturedLabel {
    pub fn capture(
        item: &Item,
        parameter: &'static str,
        limits: &CemValueArtifactLimits,
    ) -> Result<Self, SuggestionsError> {
        let origin = item.source_map().unwrap_or_default();
        let source = retained_cem_node(item)
            .ok_or_else(|| failure("A label requires an original native source", &origin))?;
        let CemAstNode::Element {
            expanded_name,
            children,
            ..
        } = source.node()
        else {
            return Err(failure("A label requires an inert XML template", &origin));
        };
        if expanded_name.namespace_uri != "http://www.w3.org/1999/xhtml" {
            return Err(failure(
                "Local labels require an XHTML text/cem-ml template",
                &origin,
            ));
        }
        let mut namespaces =
            BTreeMap::from([("xml".into(), "http://www.w3.org/XML/1998/namespace".into())]);
        let mut ancestor = Some(item.clone());
        let mut work = 0;
        let mut depth = 0;
        let mut template_type = None;
        while let Some(current) = ancestor {
            depth += 1;
            if depth > limits.max_depth {
                return Err(failure("Label capture depth limit exceeded", &origin));
            }
            let view = current
                .view()
                .ok_or_else(|| failure("Missing label source view", &origin))?;
            for attribute in view
                .attributes(QueryContextScope(0))
                .map_err(|_| failure("Label attributes unavailable", &origin))?
            {
                work += 1;
                if work > limits.max_values {
                    return Err(failure("Label capture work limit exceeded", &origin));
                }
                let attribute =
                    attribute.map_err(|_| failure("Label attribute unavailable", &origin))?;
                let name = field_text(&attribute, "name").unwrap_or_default();
                let value = field_text(&attribute, "value").unwrap_or_default();
                if depth == 1 && name == "type" {
                    template_type = Some(value.clone());
                }
                if field_text(&attribute, "namespace").as_deref()
                    == Some("http://www.w3.org/2000/xmlns/")
                {
                    namespaces
                        .entry(if name == "xmlns" {
                            "".into()
                        } else {
                            name.trim_start_matches("xmlns:").into()
                        })
                        .or_insert(value);
                }
            }
            ancestor = view
                .parent(QueryContextScope(0))
                .map_err(|_| failure("Label parent unavailable", &origin))?;
        }
        if template_type.as_deref() != Some("text/cem-ml") {
            return Err(failure("Label type must be text/cem-ml", &origin));
        }
        let mut text = String::new();
        for child in children {
            work += 1;
            if work > limits.max_values {
                return Err(failure("Label capture work limit exceeded", &origin));
            }
            if !matches!(
                source.owner().ast().get(*child),
                Some(
                    CemAstNode::Text { .. }
                        | CemAstNode::Whitespace { .. }
                        | CemAstNode::Cdata { .. }
                )
            ) {
                return Err(failure(
                    "Local CEM-ML templates require text content",
                    &origin,
                ));
            }
            for fragment in source
                .owner()
                .source_text_fragments(*child)
                .into_iter()
                .flatten()
            {
                if text.len().saturating_add(fragment.len()) > limits.max_bytes {
                    return Err(failure("Label capture byte limit exceeded", &origin));
                }
                text.push_str(&fragment);
            }
        }
        // Bound the native token stream before recursive template compilation.
        use cem_ml::tokenizer::{SchemaTokenKind, SchemaTokenizer};
        let mut tokenizer = cem_ml::tokenizer::cem::CemTokenizer::from_source(
            cem_ml::source::BytesSource::new(cem_ml::source::SourceId(1), text.as_bytes().to_vec()),
        );
        let mut depth = 0usize;
        while let Some(token) = tokenizer.next_token() {
            work += 1;
            match token.kind {
                SchemaTokenKind::NodeStart { .. } => depth += 1,
                SchemaTokenKind::NodeEnd { .. } => depth = depth.saturating_sub(1),
                _ => {}
            }
            if work > limits.max_values || depth > limits.max_depth {
                return Err(failure(
                    "Label compilation work/depth limit exceeded",
                    &origin,
                ));
            }
        }
        let mut artifact = compile_template(
            &text,
            &CompileTemplateOptions {
                host_bindings: vec![parameter.into()],
                ..Default::default()
            },
        );
        admit_label_template(&artifact)?;
        if let Some(d) = artifact
            .diagnostics
            .iter()
            .find(|d| d.severity.is_hard_violation())
        {
            return Err(failure(&d.message, &origin));
        }
        bind_names(&mut artifact.nodes, &namespaces, &origin)?;
        Ok(Self {
            source,
            artifact,
            parameter,
        })
    }
    fn render(&self, input: Item) -> Result<RenderPlan, SuggestionsError> {
        let mut data = TemplateData::default();
        data.bindings
            .insert(self.parameter.into(), ItemStream::once(input));
        let plan = render_compiled_template(&self.artifact, &data);
        if let Some(d) = plan
            .diagnostics
            .iter()
            .find(|d| d.severity.is_hard_violation())
        {
            return Err(failure(
                &d.message,
                &self.source.query_item().source_map().unwrap_or_default(),
            ));
        }
        Ok(plan)
    }
}
fn literal(name: &str, value: String, source: &SourceMapStack) -> TemplateAttribute {
    TemplateAttribute {
        name: name.into(),
        value: Some(TemplateAttributeValue::Literal(value)),
        source_map: source.clone(),
    }
}
// Captured XML text may contain entities. Use the original template span
// conservatively instead of claiming decoded CEM-ML offsets are XML offsets.
fn remap_origin(node: &mut TemplateNode, source: &SourceMapStack) {
    use crate::render::{CompiledTemplateExpression, TemplateAttributePart};
    fn expression(value: &mut Option<CompiledTemplateExpression>, source: &SourceMapStack) {
        if let Some(value) = value {
            value.source_map = source.clone();
        }
    }
    match node {
        TemplateNode::Expression(value) => value.source_map = source.clone(),
        TemplateNode::Element {
            attributes,
            source_map,
            ..
        }
        | TemplateNode::Result {
            attributes,
            source_map,
            ..
        } => {
            *source_map = source.clone();
            for attr in attributes {
                attr.source_map = source.clone();
                match &mut attr.value {
                    Some(TemplateAttributeValue::Expression(value)) => {
                        value.source_map = source.clone()
                    }
                    Some(TemplateAttributeValue::Template(parts)) => {
                        for part in parts {
                            if let TemplateAttributePart::Expression(value) = part {
                                value.source_map = source.clone();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        TemplateNode::If {
            test, source_map, ..
        } => {
            *source_map = source.clone();
            expression(test, source);
        }
        TemplateNode::ForEach {
            select, source_map, ..
        }
        | TemplateNode::ProjectPayload { select, source_map }
        | TemplateNode::Variable {
            select, source_map, ..
        } => {
            *source_map = source.clone();
            expression(select, source);
        }
        TemplateNode::Choose {
            branches,
            source_map,
        } => {
            *source_map = source.clone();
            for branch in branches {
                expression(&mut branch.test, source);
            }
        }
        TemplateNode::Text { source_map, .. } | TemplateNode::Comment { source_map, .. } => {
            *source_map = source.clone()
        }
    }
}

// Ordinary result names inherit the source's namespace bindings. Control nodes
// already have dedicated compiler variants or native template semantics.
fn bind_names(
    nodes: &mut [TemplateNode],
    namespaces: &BTreeMap<String, String>,
    source: &SourceMapStack,
) -> Result<(), SuggestionsError> {
    for node in nodes {
        remap_origin(node, source);
        match node {
            TemplateNode::Element {
                tag,
                attributes,
                children,
                source_map,
            } => {
                bind_names(children, namespaces, source)?;
                *source_map = source.clone();
                if matches!(
                    tag.as_str(),
                    "module" | "body" | "template" | "param" | "call" | "attribute" | "slice"
                ) {
                    continue;
                }
                let prefix = tag.split_once(':').map_or("", |(prefix, _)| prefix);
                let namespace = namespaces.get(prefix);
                if !prefix.is_empty() && namespace.is_none() {
                    return Err(failure(
                        format!("Unbound label namespace `{prefix}`"),
                        source,
                    ));
                }
                let mut attrs = vec![literal("name", tag.clone(), source)];
                if let Some(uri) = namespace {
                    attrs.push(literal("namespace", uri.clone(), source));
                }
                let mut body = Vec::new();
                for attribute in attributes.drain(..) {
                    let mut controls = vec![literal("name", attribute.name.clone(), source)];
                    if let Some((prefix, _)) = attribute.name.split_once(':') {
                        let uri = namespaces.get(prefix).ok_or_else(|| {
                            failure(
                                format!("Unbound label attribute namespace `{prefix}`"),
                                source,
                            )
                        })?;
                        controls.push(literal("namespace", uri.clone(), source));
                    }
                    controls.push(TemplateAttribute {
                        name: "value".into(),
                        value: attribute.value,
                        source_map: source.clone(),
                    });
                    body.push(TemplateNode::Result {
                        instruction: ResultInstruction::Attribute,
                        attributes: controls,
                        children: vec![],
                        source_map: source.clone(),
                    });
                }
                body.append(children);
                *node = TemplateNode::Result {
                    instruction: ResultInstruction::Element,
                    attributes: attrs,
                    children: body,
                    source_map: source.clone(),
                };
            }
            TemplateNode::Result { children, .. }
            | TemplateNode::If { children, .. }
            | TemplateNode::ForEach { children, .. } => bind_names(children, namespaces, source)?,
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    bind_names(&mut branch.children, namespaces, source)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn prepare_labels(
    view: SuggestionsView,
    option: Option<&CapturedLabel>,
    group: Option<&CapturedLabel>,
    limits: &CemValueArtifactLimits,
) -> Result<(SuggestionsView, usize), SuggestionsError> {
    let control = cem_ml::operation_control::OperationControl::with_root_policy(
        Default::default(),
        cem_ml::scheduler::ScopePolicy::host_root(),
    )
    .map_err(|e| failure(e.to_string(), &Default::default()))?;
    let mut budget = crate::eval::value_control::ValueControl::new(
        &control,
        cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        QueryContextScope(0),
        *limits,
    )
    .map_err(|e| failure(e.to_string(), &Default::default()))?;
    let mut rows = Vec::new();
    let mut groups = Vec::new();
    for (count, template, is_group) in [
        (view.0.plan.rows.len(), option, false),
        (view.0.plan.groups.len(), group, true),
    ] {
        for index in 0..count {
            let input = if is_group {
                view.group(index)?
            } else {
                view.row(index)?
            };
            let plan = if let Some(template) = template {
                template.render(input)?
            } else {
                let label = if is_group {
                    &view.0.plan.groups[index].label
                } else {
                    &view.0.plan.rows[index].label
                };
                let source = input.source_map().unwrap_or_default();
                let explicit = !is_group
                    && view.0.plan.rows[index]
                        .source
                        .view()
                        .and_then(|v| v.field("attributes"))
                        .unwrap_or_default()
                        .iter()
                        .any(|a| {
                            field_text(a, "name").as_deref() == Some("label")
                                && field_text(a, "value").is_some_and(|v| !v.is_empty())
                        });
                let nodes = if is_group || explicit {
                    vec![RenderPlanNode::Text {
                        text: label.clone(),
                        source_map: source,
                    }]
                } else {
                    vec![RenderPlanNode::Reference {
                        reference: cem_ml::value::CemReference::new(
                            view.0.plan.rows[index].content.clone(),
                        ),
                        source_map: source,
                    }]
                };
                RenderPlan {
                    nodes,
                    host_attribute_updates: vec![],
                    diagnostics: vec![],
                }
            };
            if !plan.host_attribute_updates.is_empty() {
                return Err(failure(
                    "Labels cannot write host attributes",
                    &Default::default(),
                ));
            }
            let projected = crate::render::project_render_plan_with_budget(&plan, &mut budget)
                .map_err(|e| {
                    failure(e.to_string(), &input_source(view.clone(), is_group, index))
                })?;
            let admitted = admit_projected_label_output(projected)?;
            let values = crate::eval::output::output_nodes(admitted.nodes);
            if is_group {
                groups.push(values);
            } else {
                rows.push(values);
            }
        }
    }
    Ok((view.with_labels(rows, groups), budget.charged_bytes()))
}
fn input_source(view: SuggestionsView, group: bool, index: usize) -> SourceMapStack {
    (if group {
        view.group(index)
    } else {
        view.row(index)
    })
    .ok()
    .and_then(|v| v.source_map())
    .unwrap_or_default()
}
