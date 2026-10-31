//! Exact named receiver occurrences from the shared compiled native IR.
use super::*;
use crate::render::{TemplateAttributeValue, TemplateNode};
fn fields(node: &TemplateNode) -> Result<BTreeMap<String, String>, String> {
    let TemplateNode::Element {
        attributes,
        children,
        ..
    } = node
    else {
        return Err(INVALID.into());
    };
    if !children.is_empty() {
        return Err(INVALID.into());
    }
    let mut fields = BTreeMap::new();
    for attr in attributes {
        let Some(TemplateAttributeValue::Literal(value)) = &attr.value else {
            return Err(INVALID.into());
        };
        if !matches!(
            attr.name.as_str(),
            "name" | "type" | "itemCount" | "minItems" | "maxItems"
        ) || fields.insert(attr.name.clone(), value.clone()).is_some()
        {
            return Err(INVALID.into());
        }
    }
    let name = fields.get("name").ok_or(INVALID)?;
    if name.is_empty()
        || name.len() > 256
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        || !fields.contains_key("type")
    {
        return Err(INVALID.into());
    }
    let counts =
        ["itemCount", "minItems", "maxItems"].map(|key| fields.get(key).map(String::as_str));
    if counts.iter().any(Option::is_some) {
        cem_ml::schema::structural_capsule::validate_literal_cardinality(
            counts[0], counts[1], counts[2],
        )?;
    }
    Ok(fields)
}
pub(super) fn inspect(
    artifact: &TemplateArtifact,
    receivers: &BTreeMap<String, ReceiverContract>,
    work: &Work<'_>,
) -> Result<BTreeMap<String, ReceiverOccurrence>, String> {
    if application_declaration::required(artifact)? != receivers.keys().cloned().collect() {
        return Err(INVALID.into());
    }
    let mut pending = artifact.nodes.iter().map(|n| (n, 0)).collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    let mut result = BTreeMap::new();
    let mut used = BTreeSet::new();
    while let Some((node, depth)) = pending.pop() {
        work.check()?;
        if depth > 64 {
            return Err(INVALID.into());
        }
        match node {
            TemplateNode::Element { tag, children, .. } => {
                if tag.rsplit(':').next() == Some("attribute") {
                    if depth != 0 || tag != "attribute" {
                        return Err(INVALID.into());
                    }
                    let fields = fields(node)?;
                    let name = fields["name"].clone();
                    if !seen.insert(name.clone()) {
                        return Err(INVALID.into());
                    }
                    let ty = &fields["type"];
                    if receivers.contains_key(ty) {
                        used.insert(ty.clone());
                    }
                    result.insert(name, ReceiverOccurrence { fields });
                    if result.len() > MAX_RECEIVERS {
                        return Err(INVALID.into());
                    }
                }
                pending.extend(children.iter().map(|n| (n, depth + 1)));
            }
            TemplateNode::If { children, .. }
            | TemplateNode::ForEach { children, .. }
            | TemplateNode::Result { children, .. } => {
                pending.extend(children.iter().map(|n| (n, depth + 1)))
            }
            TemplateNode::Choose { branches, .. } => {
                for b in branches {
                    pending.extend(b.children.iter().map(|n| (n, depth + 1)));
                }
            }
            _ => {}
        }
    }
    if used != receivers.keys().cloned().collect() {
        return Err(INVALID.into());
    }
    Ok(result)
}
