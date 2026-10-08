//! Final label admission is a bounded presentation boundary, never a reference evaluator.
use super::SuggestionsError;
use crate::{
    eval::QueryContextScope,
    render::{project_render_plan_with_control, RenderPlan, RenderPlanNode, TemplateArtifact},
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    scheduler::ScopePolicy,
    source_map::SourceMapStack,
    value::artifact::CemValueArtifactLimits,
};

fn invalid(message: impl Into<String>, source: &SourceMapStack) -> SuggestionsError {
    SuggestionsError {
        code: "cem.suggestions.label_invalid",
        message: message.into(),
        source: source.clone(),
    }
}

/// Labels cannot install component styles, host defaults or module mappings.
pub fn admit_label_template(template: &TemplateArtifact) -> Result<(), SuggestionsError> {
    if !template.stylesheets.is_empty() || template.module_map.is_some() {
        return Err(invalid(
            "Labels cannot own styles or module mappings",
            &Default::default(),
        ));
    }
    Ok(())
}

/// Expand native content only for presentation and inspect the entire output before publication.
pub fn admit_label_output(
    plan: &RenderPlan,
    limits: &CemValueArtifactLimits,
) -> Result<RenderPlan, SuggestionsError> {
    if !plan.host_attribute_updates.is_empty() {
        return Err(invalid(
            "Labels cannot write host attributes",
            &Default::default(),
        ));
    }
    let control = OperationControl::with_root_policy(Default::default(), ScopePolicy::host_root())
        .map_err(|e| invalid(e.to_string(), &Default::default()))?;
    let projected = project_render_plan_with_control(
        plan,
        QueryContextScope(0),
        limits,
        &control,
        ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|e| invalid(e.to_string(), &Default::default()))?;
    admit_projected_label_output(projected)
}

pub(crate) fn admit_projected_label_output(
    projected: RenderPlan,
) -> Result<RenderPlan, SuggestionsError> {
    let mut pending: Vec<_> = projected.nodes.iter().collect();
    while let Some(node) = pending.pop() {
        match node {
            RenderPlanNode::Text { .. }
            | RenderPlanNode::Cdata { .. }
            | RenderPlanNode::Comment { .. } => {}
            RenderPlanNode::Element {
                tag,
                namespace,
                attributes,
                children,
                source_map,
                ..
            } => {
                let name = tag.to_ascii_lowercase();
                let allowed = match namespace.as_deref() {
                    None | Some("http://www.w3.org/1999/xhtml") => matches!(
                        name.as_str(),
                        "span"
                            | "div"
                            | "p"
                            | "br"
                            | "wbr"
                            | "b"
                            | "strong"
                            | "em"
                            | "i"
                            | "u"
                            | "s"
                            | "small"
                            | "mark"
                            | "sub"
                            | "sup"
                            | "abbr"
                            | "cite"
                            | "code"
                            | "kbd"
                            | "samp"
                            | "var"
                            | "time"
                            | "q"
                            | "blockquote"
                            | "pre"
                            | "ul"
                            | "ol"
                            | "li"
                            | "dl"
                            | "dt"
                            | "dd"
                            | "hr"
                            | "img"
                            | "figure"
                            | "figcaption"
                            | "bdi"
                            | "bdo"
                            | "ruby"
                            | "rt"
                            | "rp"
                            | "a"
                    ),
                    Some("http://www.w3.org/2000/svg") => matches!(
                        name.as_str(),
                        "svg"
                            | "g"
                            | "path"
                            | "text"
                            | "tspan"
                            | "circle"
                            | "rect"
                            | "line"
                            | "polyline"
                            | "polygon"
                            | "ellipse"
                            | "title"
                            | "desc"
                            | "defs"
                            | "lineargradient"
                            | "radialgradient"
                            | "stop"
                    ),
                    _ => false,
                };
                if !allowed {
                    return Err(invalid(
                        format!("Unsupported or interactive label element `{tag}`"),
                        source_map,
                    ));
                }
                for attribute in attributes {
                    if attribute.namespace.as_deref() == Some("http://www.w3.org/2000/xmlns/") {
                        continue;
                    }
                    let name = attribute.name.to_ascii_lowercase();
                    if name.starts_with("on")
                        || matches!(
                            name.as_str(),
                            "id" | "role"
                                | "tabindex"
                                | "autofocus"
                                | "contenteditable"
                                | "form"
                                | "slot"
                                | "part"
                                | "style"
                                | "popover"
                                | "popovertarget"
                                | "is"
                                | "interaction-scope"
                                | "interaction-name"
                                | "slice"
                                | "slice-event"
                                | "slice-value"
                        )
                        || tag.eq_ignore_ascii_case("a")
                            && matches!(name.as_str(), "href" | "download")
                    {
                        return Err(invalid(format!("Label attribute `{name}` would take ownership of interaction or relationships"), &attribute.source_map));
                    }
                }
                pending.extend(children);
            }
            RenderPlanNode::Reference { source_map, .. }
            | RenderPlanNode::ProcessingInstruction { source_map, .. } => {
                return Err(invalid("Unsupported unresolved label output", source_map))
            }
        }
    }
    Ok(projected)
}
