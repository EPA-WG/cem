use super::*;
type Fields = BTreeMap<String, String>;
pub(super) fn bounds(sources: &BTreeMap<String, ComponentReceiverSource>) -> Result<(), String> {
    let mut trees = BTreeSet::new();
    let mut nodes = 0usize;
    let mut attributes = 0usize;
    for source in sources.values() {
        attributes += source.attributes.len();
        if trees.insert(Arc::as_ptr(&source.source) as usize) {
            let count = source.source.ast().nodes.len();
            if count > 16_384 {
                return Err(INVALID.into());
            }
            nodes += count;
        }
        if source.attributes.len() > 64
            || attributes > 256
            || nodes > 65_536
            || source.schema_uri.len() > 4096
        {
            return Err(INVALID.into());
        }
    }
    Ok(())
}
fn fields_allowed(fields: &Fields) -> bool {
    fields.keys().all(|name| {
        matches!(
            name.as_str(),
            "name" | "type" | "itemCount" | "minItems" | "maxItems"
        )
    })
}
pub(super) fn expected(
    envelope: &VerifiedApplicationEnvelope,
    tag: &str,
    control: &OperationControl,
) -> Result<BTreeMap<String, Fields>, String> {
    let artifact = envelope.artifact(tag).ok_or(INVALID)?;
    let mut pending = artifact
        .nodes
        .iter()
        .map(|node| (node, 0))
        .collect::<Vec<_>>();
    let mut result = BTreeMap::new();
    let mut count = 0;
    while let Some((node, depth)) = pending.pop() {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        count += 1;
        if count > 100_000 || depth > 64 {
            return Err(INVALID.into());
        }
        match node {
            TemplateNode::Element {
                tag: node_tag,
                attributes,
                children,
                ..
            } => {
                if node_tag.rsplit(':').next() == Some("attribute") {
                    let mut fields = Fields::new();
                    for attribute in attributes {
                        let Some(TemplateAttributeValue::Literal(value)) = &attribute.value else {
                            return Err(INVALID.into());
                        };
                        if fields
                            .insert(attribute.name.clone(), value.clone())
                            .is_some()
                        {
                            return Err(INVALID.into());
                        }
                    }
                    if let Some(ty) = fields.get("type") {
                        if envelope.receiver(tag, ty).is_some() {
                            if depth != 0
                                || node_tag != "attribute"
                                || !children.is_empty()
                                || !fields_allowed(&fields)
                            {
                                return Err(INVALID.into());
                            }
                            let name = fields
                                .get("name")
                                .filter(|s| !s.is_empty() && s.len() <= 256)
                                .ok_or(INVALID)?;
                            if result.insert(name.clone(), fields).is_some() || result.len() > 64 {
                                return Err(INVALID.into());
                            }
                        }
                    }
                }
                pending.extend(children.iter().map(|node| (node, depth + 1)));
            }
            TemplateNode::If { children, .. }
            | TemplateNode::ForEach { children, .. }
            | TemplateNode::Result { children, .. } => {
                pending.extend(children.iter().map(|node| (node, depth + 1)))
            }
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    pending.extend(branch.children.iter().map(|node| (node, depth + 1)))
                }
            }
            _ => {}
        }
    }
    Ok(result)
}
pub(super) fn matches(
    source: &Arc<RetainedCemTree>,
    occurrence: AstNodeId,
    expected: &Fields,
) -> Result<(), String> {
    let Some(CemAstNode::Element {
        expanded_name,
        attributes,
        children,
        ..
    }) = source.ast().get(occurrence)
    else {
        return Err(INVALID.into());
    };
    if expanded_name.local_name != "attribute"
        || !matches!(
            expanded_name.namespace_uri.as_str(),
            "" | cem_ml::schema::registry::CEM_SCHEMA_URI
        )
        || !children.is_empty()
    {
        return Err(INVALID.into());
    }
    let mut actual = Fields::new();
    for id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value: Some(value),
            value_nodes,
            ..
        }) = source.ast().get(*id)
        else {
            return Err(INVALID.into());
        };
        if !expanded_name.namespace_uri.is_empty()
            || !value_nodes.is_empty()
            || actual
                .insert(expanded_name.local_name.clone(), value.clone())
                .is_some()
        {
            return Err(INVALID.into());
        }
    }
    if !fields_allowed(&actual)
        || !matches!(
            actual.get("type").map(String::as_str),
            Some("node" | "schema:node")
        )
    {
        return Err(INVALID.into());
    }
    actual.insert("type".into(), expected["type"].clone());
    if actual != *expected {
        return Err(INVALID.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
