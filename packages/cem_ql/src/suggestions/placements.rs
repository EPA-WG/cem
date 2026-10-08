//! Consumer-only construction annotation. No source identity is serialized into DOM.
use super::*;
use crate::render::{RenderPlan, RenderPlanNode};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SuggestionPlacement {
    pub path: Vec<usize>,
    pub handle: String,
}

/// The ordinary native projection bounds expansion first. The annotation must name
/// an exact row of the invocation's immutable view; strings and prior views fail.
pub fn project_suggestion_placements(
    plan: &RenderPlan,
    view: Option<&SuggestionsView>,
    limits: &CemValueArtifactLimits,
) -> Result<(RenderPlan, Vec<SuggestionPlacement>), SuggestionsError> {
    let mut plan = crate::render::project_render_plan_with_control(
        plan,
        QueryContextScope(0),
        limits,
        &cem_ml::operation_control::OperationControl::default(),
        cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|e| error("cem.value.projection", e.to_string(), None))?;
    fn walk(
        nodes: &mut [RenderPlanNode],
        parent: &[usize],
        view: Option<&SuggestionsView>,
        metadata: &mut Vec<SuggestionPlacement>,
        seen: &mut BTreeSet<String>,
        limits: &CemValueArtifactLimits,
        metadata_bytes: &mut usize,
    ) -> Result<(), SuggestionsError> {
        for (index, node) in nodes.iter_mut().enumerate() {
            let RenderPlanNode::Element {
                attributes,
                children,
                source_map,
                ..
            } = node
            else {
                continue;
            };
            let mut path = parent.to_vec();
            path.push(index);
            let annotations: Vec<_> = attributes
                .iter()
                .filter(|a| a.name == "suggestion-row")
                .collect();
            if !annotations.is_empty() {
                let invalid = || SuggestionsError {
                    code: "cem.suggestions.placement",
                    message:
                        "A unique option shell requires an exact current native suggestion row"
                            .into(),
                    source: source_map.clone(),
                };
                let roles: Vec<_> = attributes
                    .iter()
                    .filter(|a| a.name == "role" && a.namespace.is_none())
                    .collect();
                if annotations.len() != 1
                    || annotations[0].namespace.is_some()
                    || roles.len() != 1
                    || roles[0].value != "option"
                {
                    return Err(invalid());
                }
                let [item] = annotations[0].value_stream.items.as_slice() else {
                    return Err(invalid());
                };
                let mut item = item;
                let mut depth = path.len();
                while let Some(targets) = crate::eval::values::reference_values(item) {
                    depth += 1;
                    if depth > limits.max_depth {
                        return Err(invalid());
                    }
                    let [target] = targets else {
                        return Err(invalid());
                    };
                    item = target;
                }
                let native = item
                    .view()
                    .and_then(|v| v.downcast_ref::<SuggestionNode>())
                    .ok_or_else(invalid)?;
                let view = view.ok_or_else(invalid)?;
                if native.attribute.is_some() || !Arc::ptr_eq(&native.view, &view.0) {
                    return Err(invalid());
                }
                let Node::Row(row) = native.node else {
                    return Err(invalid());
                };
                let handle = view.0.plan.rows[row].handle.clone();
                if !seen.insert(handle.clone()) {
                    return Err(invalid());
                }
                let placement = SuggestionPlacement {
                    path: path.clone(),
                    handle,
                };
                let bytes = serde_json::to_vec(&placement).map_err(|_| invalid())?.len()
                    + if metadata.is_empty() { 2 } else { 1 };
                *metadata_bytes = metadata_bytes.checked_add(bytes).ok_or_else(invalid)?;
                if metadata.len() >= limits.max_values || *metadata_bytes > limits.max_bytes {
                    return Err(SuggestionsError {
                        code: "cem.suggestions.limit",
                        message: "Native placement metadata exceeds bounds".into(),
                        source: source_map.clone(),
                    });
                }
                metadata.push(placement);
                attributes.retain(|a| a.name != "suggestion-row");
            }
            walk(
                children,
                &path,
                view,
                metadata,
                seen,
                limits,
                metadata_bytes,
            )?;
        }
        Ok(())
    }
    let mut metadata = Vec::new();
    walk(
        &mut plan.nodes,
        &[],
        view,
        &mut metadata,
        &mut BTreeSet::new(),
        limits,
        &mut 0,
    )?;
    Ok((plan, metadata))
}

impl SuggestionsView {
    pub fn from_root(item: &Item) -> Option<Self> {
        let node = item.view()?.downcast_ref::<SuggestionNode>()?;
        (matches!(node.node, Node::Root) && node.attribute.is_none())
            .then(|| Self(node.view.clone()))
    }
}
