//! Final projection keeps native attributes intact while deriving their lexical
//! output. One budget covers nested markup payloads and reused subtrees.
use super::*;
use crate::eval::value_control::ValueControl;
use cem_ml::operation_control::ControlError;
use cem_ml::value::artifact::CemValueArtifactLimits;

pub fn project_render_plan_with_control(
    plan: &RenderPlan,
    query_scope: QueryContextScope,
    limits: &CemValueArtifactLimits,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<RenderPlan, ControlError> {
    let mut budget = ValueControl::new(control, scope, query_scope, *limits)?;
    let nodes = nodes(&plan.nodes, &mut budget, 0)?;
    control.check_scope(scope)?;
    Ok(RenderPlan {
        nodes,
        host_attribute_updates: plan.host_attribute_updates.clone(),
        diagnostics: plan.diagnostics.clone(),
    })
}

pub fn project_attribute_value_with_control(
    attribute: &RenderPlanAttribute,
    query_scope: QueryContextScope,
    limits: &CemValueArtifactLimits,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<String, ControlError> {
    attribute_value(
        attribute,
        &mut ValueControl::new(control, scope, query_scope, *limits)?,
        0,
    )
}

fn attribute_value(
    attribute: &RenderPlanAttribute,
    budget: &mut ValueControl<'_>,
    depth: usize,
) -> Result<String, ControlError> {
    budget.charge(0, depth)?;
    let content_type = attribute
        .contract
        .as_ref()
        .and_then(|c| c.content_type.as_deref());
    if matches!(
        content_type,
        Some("text/html" | "application/xml" | "text/xml")
    ) {
        let projected = nodes(
            &[RenderPlanNode::Reference {
                reference: cem_ml::value::CemReference::new(attribute.value_stream.items.clone()),
                source_map: attribute.source_map.clone(),
            }],
            budget,
            depth + 1,
        )?;
        let plan = RenderPlan {
            nodes: projected,
            host_attribute_updates: vec![],
            diagnostics: vec![],
        };
        let rendered = if content_type == Some("text/html") {
            let mut renderer = RenderPlanHtmlRenderer::controlled(budget.control, budget.scope);
            renderer.render_plan(&plan);
            renderer.force()?;
            renderer.out
        } else {
            let mut renderer = RenderPlanXmlRenderer::controlled(budget.control, budget.scope);
            renderer.render_plan(&plan);
            renderer.force()?;
            renderer.out
        };
        budget.charge(rendered.len(), depth)?;
        Ok(rendered)
    } else {
        budget.text(&attribute.value_stream.items)
    }
}

fn nodes(
    input: &[RenderPlanNode],
    budget: &mut ValueControl<'_>,
    depth: usize,
) -> Result<Vec<RenderPlanNode>, ControlError> {
    let mut result = Vec::new();
    for node in input {
        budget.charge(std::mem::size_of::<RenderPlanNode>(), depth)?;
        if let RenderPlanNode::Reference { reference, .. } = node {
            let expanded = references::expand_reference_scoped(reference, budget)?;
            result.extend(nodes(&expanded, budget, depth + 1)?);
            continue;
        }
        let node = match node {
            RenderPlanNode::Element {
                tag,
                qualified_name,
                namespace,
                attributes,
                children,
                source_map,
            } => {
                budget.charge(
                    tag.len()
                        + qualified_name.as_ref().map_or(0, String::len)
                        + namespace.as_ref().map_or(0, String::len),
                    depth,
                )?;
                let mut projected_attributes = Vec::new();
                for attribute in attributes {
                    budget.charge(
                        attribute.name.len() + std::mem::size_of::<RenderPlanAttribute>(),
                        depth,
                    )?;
                    let value = attribute_value(attribute, budget, depth + 1)?;
                    projected_attributes.push(RenderPlanAttribute {
                        value,
                        ..attribute.clone()
                    });
                }
                RenderPlanNode::Element {
                    tag: tag.clone(),
                    qualified_name: qualified_name.clone(),
                    namespace: namespace.clone(),
                    attributes: projected_attributes,
                    children: nodes(children, budget, depth + 1)?,
                    source_map: source_map.clone(),
                }
            }
            RenderPlanNode::Text { text, .. }
            | RenderPlanNode::Cdata { text, .. }
            | RenderPlanNode::Comment { text, .. } => {
                budget.charge(text.len(), depth)?;
                node.clone()
            }
            RenderPlanNode::ProcessingInstruction { target, data, .. } => {
                budget.charge(target.len() + data.len(), depth)?;
                node.clone()
            }
            RenderPlanNode::Reference { .. } => unreachable!(),
        };
        result.push(node);
    }
    Ok(result)
}
