//! Shared native canonical-island value construction.
use crate::eval::Item;
pub(crate) const PREFIXES: &[&str] = &[
    "cem-island",
    "cem-hydration",
    "cem-attributes",
    "cem-dataset",
    "cem-payload",
    "cem-slices",
    "cem-resources",
    "cem-form",
    "cem-validation",
    "cem-events",
];
pub(crate) fn attribute(
    name: &str,
    value: &str,
    source: &cem_ml::source_map::SourceMapStack,
) -> crate::render::RenderPlanAttribute {
    crate::render::RenderPlanAttribute {
        name: name.into(),
        namespace: None,
        qualified_name: None,
        value: value.into(),
        value_stream: crate::eval::ItemStream::once(Item::Atomic(crate::eval::AtomValue::String(
            value.into(),
        ))),
        contract: None,
        source_map: source.clone(),
    }
}
pub(crate) fn element(
    name: &str,
    ns: &str,
    prefix: Option<&str>,
    attributes: Vec<crate::render::RenderPlanAttribute>,
    children: Vec<crate::render::RenderPlanNode>,
    source: &cem_ml::source_map::SourceMapStack,
) -> crate::render::RenderPlanNode {
    crate::render::RenderPlanNode::Element {
        tag: name.into(),
        namespace: Some(ns.into()),
        qualified_name: prefix.map(|prefix| format!("{prefix}:{name}")),
        attributes,
        children,
        source_map: source.clone(),
    }
}
pub(crate) fn add_child(
    parent: &mut crate::render::RenderPlanNode,
    child: crate::render::RenderPlanNode,
) {
    if let crate::render::RenderPlanNode::Element { children, .. } = parent {
        children.push(child)
    }
}
