//! Literal source arena and occurrence verification.
use super::*;

pub(super) fn nodes(document: &CemDocument) -> Result<Vec<SourceNode>, String> {
    if !document.diagnostics.is_empty()
        || !document.id_table.is_empty()
        || !document.unresolved_slots.is_empty()
        || !document.typed_preludes.is_empty()
        || document.nodes.len() > MAX_RECORDS
    {
        return Err(INVALID.into());
    }
    document
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let mut result = SourceNode {
                kind: String::new(),
                name: None,
                attributes: vec![],
                children: vec![],
                value: None,
                boundary: true,
                source: crate::schema::document_model::source_stack_for_node(node).clone(),
            };
            let id = match node {
                CemAstNode::Document {
                    node_id,
                    root_children,
                    ..
                } => {
                    result.kind = "document".into();
                    result.children = root_children.clone();
                    *node_id
                }
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    attributes,
                    children,
                    has_explicit_boundary,
                    ..
                } => {
                    result.kind = "element".into();
                    result.name = Some(expanded_name.clone());
                    result.attributes = attributes.clone();
                    result.children = children.clone();
                    result.boundary = *has_explicit_boundary;
                    *node_id
                }
                CemAstNode::Attribute {
                    node_id,
                    expanded_name,
                    value,
                    value_nodes,
                    ..
                } if value_nodes.is_empty() => {
                    result.kind = "attribute".into();
                    result.name = Some(expanded_name.clone());
                    result.value = value.clone();
                    *node_id
                }
                CemAstNode::Text { node_id, data, .. }
                | CemAstNode::Whitespace { node_id, data, .. }
                    if data.trim().is_empty() =>
                {
                    result.kind = if matches!(node, CemAstNode::Text { .. }) {
                        "text"
                    } else {
                        "whitespace"
                    }
                    .into();
                    result.value = Some(data.clone());
                    *node_id
                }
                CemAstNode::Comment { node_id, data, .. } => {
                    result.kind = "comment".into();
                    result.value = Some(data.clone());
                    *node_id
                }
                _ => return Err(INVALID.into()),
            };
            if id as usize != index {
                return Err(INVALID.into());
            }
            Ok(result)
        })
        .collect()
}
pub(super) fn restore(
    nodes: &[SourceNode],
    source: &SourceIdentity,
    control: &Work<'_>,
) -> Result<CemDocument, String> {
    if nodes.is_empty()
        || nodes.len() > MAX_RECORDS
        || !uri(&source.uri)
        || !valid_hash(&source.hash)
        || source.byte_length > 1024 * 1024
    {
        return Err(INVALID.into());
    }
    let mut document = CemDocument::default();
    let mut parents = vec![0u8; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        check(control)?;
        if node.source.frames.len() > 64 || (index > 0 && node.source.frames.is_empty()) {
            return Err(INVALID.into());
        }
        for frame in &node.source.frames {
            // The shared builder has no enclosing element for document trivia;
            // its one builder frame uses the reserved synthetic ID 0 while
            // retaining original input coordinates. Never admit it for fields.
            let margin = matches!(node.kind.as_str(), "text" | "whitespace" | "comment")
                && nodes[0].children.contains(&(index as u32))
                && node.source.frames.len() == 1
                && frame.source_id == SourceId(0)
                && frame.transform == crate::source_map::TransformKind::CemAstBuilder;
            if frame.source_id != source.id && !margin {
                return Err(INVALID.into());
            }
            // Transformation trace entries are passive provenance, never host
            // implementations. Original resource boundaries must stay exact.
            if matches!(&frame.transform, crate::source_map::TransformKind::SourceBoundary { uri } if uri != &source.uri)
                || matches!(&frame.transform, crate::source_map::TransformKind::ScssOrigin { module_uri, .. } if module_uri != &source.uri)
            {
                return Err(INVALID.into());
            }
            let ranges = match &frame.span {
                crate::source_map::FrameSpan::Single(range) => std::slice::from_ref(range),
                crate::source_map::FrameSpan::Multi(ranges) => ranges.as_slice(),
            };
            if ranges.len() > 64
                || ranges.iter().any(|r| {
                    r.start
                        .checked_add(r.len as u64)
                        .is_none_or(|e| e > source.byte_length as u64)
                })
            {
                return Err(INVALID.into());
            }
        }
        for (&child, is_attribute) in node
            .attributes
            .iter()
            .map(|id| (id, true))
            .chain(node.children.iter().map(|id| (id, false)))
        {
            let child = child as usize;
            if child == 0
                || child >= nodes.len()
                || parents[child] != 0
                || (nodes[child].kind == "attribute") != is_attribute
            {
                return Err(INVALID.into());
            }
            parents[child] = 1;
        }
        let node_id = index as u32;
        let source = node.source.clone();
        let ast = match node.kind.as_str() {
            "document"
                if index == 0
                    && node.name.is_none()
                    && node.value.is_none()
                    && node.attributes.is_empty()
                    && node.boundary =>
            {
                CemAstNode::Document {
                    node_id,
                    root_children: node.children.clone(),
                    source,
                }
            }
            "element" if index > 0 && node.value.is_none() => CemAstNode::Element {
                node_id,
                expanded_name: node.name.clone().ok_or(INVALID)?,
                attributes: node.attributes.clone(),
                children: node.children.clone(),
                has_explicit_boundary: node.boundary,
                source,
            },
            "attribute"
                if index > 0
                    && node.attributes.is_empty()
                    && node.children.is_empty()
                    && node.boundary =>
            {
                CemAstNode::Attribute {
                    node_id,
                    expanded_name: node.name.clone().ok_or(INVALID)?,
                    value: node.value.clone(),
                    value_nodes: vec![],
                    source,
                }
            }
            "text" | "whitespace" | "comment"
                if index > 0
                    && node.name.is_none()
                    && node.attributes.is_empty()
                    && node.children.is_empty()
                    && node.boundary =>
            {
                let data = node.value.clone().ok_or(INVALID)?;
                match node.kind.as_str() {
                    "text" if data.trim().is_empty() => CemAstNode::Text {
                        node_id,
                        data,
                        source,
                    },
                    "whitespace" if data.trim().is_empty() => CemAstNode::Whitespace {
                        node_id,
                        data,
                        source,
                    },
                    "comment" => CemAstNode::Comment {
                        node_id,
                        data,
                        source,
                    },
                    _ => return Err(INVALID.into()),
                }
            }
            _ => return Err(INVALID.into()),
        };
        document.nodes.push(ast);
    }
    if parents.iter().skip(1).any(|n| *n != 1) {
        return Err(INVALID.into());
    }
    let mut pending = vec![(0usize, 0usize)];
    let mut visits = 0;
    while let Some((id, depth)) = pending.pop() {
        check(control)?;
        visits += 1;
        if depth > 64 || visits > nodes.len() {
            return Err(INVALID.into());
        }
        pending.extend(
            nodes[id]
                .children
                .iter()
                .chain(&nodes[id].attributes)
                .map(|id| (*id as usize, depth + 1)),
        );
    }
    if visits != nodes.len() {
        return Err(INVALID.into());
    }
    Ok(document)
}

/// Inspect source vocabulary only. No schema compiler or parser runs here.
pub(super) fn declarations(
    document: &CemDocument,
    schema_uri: &str,
    control: &Work<'_>,
) -> Result<BTreeMap<AstNodeId, String>, String> {
    declarations_with_node_counts(document, schema_uri, control, false)
}
pub(super) fn receiver_declarations(
    document: &CemDocument,
    schema_uri: &str,
    control: &Work<'_>,
) -> Result<BTreeMap<AstNodeId, String>, String> {
    declarations_with_node_counts(document, schema_uri, control, true)
}
fn declarations_with_node_counts(
    document: &CemDocument,
    schema_uri: &str,
    control: &Work<'_>,
    node_counts: bool,
) -> Result<BTreeMap<AstNodeId, String>, String> {
    let mut roles = BTreeMap::new();
    let mut pending = vec![(0u32, "root")];
    let mut schema_count = 0;
    let mut collections = BTreeSet::new();
    let mut names = BTreeSet::new();
    while let Some((id, parent)) = pending.pop() {
        check(control)?;
        let Some(node) = document.get(id) else {
            return Err(INVALID.into());
        };
        match node {
            CemAstNode::Document { root_children, .. } if id == 0 => {
                pending.extend(root_children.iter().rev().map(|id| (*id, "root")))
            }
            CemAstNode::Element {
                expanded_name,
                attributes,
                children,
                ..
            } => {
                if !matches!(
                    expanded_name.namespace_uri.as_str(),
                    "" | crate::schema::registry::CEM_SCHEMA_URI
                ) {
                    return Err(INVALID.into());
                }
                let role = expanded_name.local_name.as_str();
                let allowed: &[&str] = match (parent, role) {
                    ("root", "schema") => {
                        schema_count += 1;
                        &["name", "namespace", "version"]
                    }
                    ("schema", "elements" | "attributes" | "field-contracts") => {
                        if !collections.insert(role.to_owned()) {
                            return Err(INVALID.into());
                        }
                        &[]
                    }
                    ("elements", "element") => &[
                        "name",
                        "required-attributes",
                        "optional-attributes",
                        "children",
                    ],
                    ("attributes", "attribute") if node_counts => &[
                        "name",
                        "type",
                        "values",
                        "minInclusive",
                        "maxInclusive",
                        "minExclusive",
                        "maxExclusive",
                        "minLength",
                        "maxLength",
                        "length",
                        "totalDigits",
                        "fractionDigits",
                        "pattern",
                        "whiteSpace",
                        "itemCount",
                        "minItems",
                        "maxItems",
                    ],
                    ("attributes", "attribute") => &[
                        "name",
                        "type",
                        "values",
                        "minInclusive",
                        "maxInclusive",
                        "minExclusive",
                        "maxExclusive",
                        "minLength",
                        "maxLength",
                        "length",
                        "totalDigits",
                        "fractionDigits",
                        "pattern",
                        "whiteSpace",
                    ],
                    ("field-contracts", "field-contract") => &[
                        "name",
                        "target",
                        "exact-total-children",
                        "min-total-children",
                        "max-total-children",
                    ],
                    _ => return Err(INVALID.into()),
                };
                let mut fields = BTreeMap::new();
                for attribute in attributes {
                    check(control)?;
                    let Some(CemAstNode::Attribute {
                        expanded_name,
                        value,
                        value_nodes,
                        ..
                    }) = document.get(*attribute)
                    else {
                        return Err(INVALID.into());
                    };
                    if !expanded_name.namespace_uri.is_empty()
                        || !allowed.contains(&expanded_name.local_name.as_str())
                        || !value_nodes.is_empty()
                        || fields
                            .insert(
                                expanded_name.local_name.as_str(),
                                value.as_deref().unwrap_or(""),
                            )
                            .is_some()
                    {
                        return Err(INVALID.into());
                    }
                }
                if role == "schema" {
                    if fields.get("namespace").copied() != Some(schema_uri)
                        || !fields.get("name").is_some_and(|v| name(v))
                    {
                        return Err(INVALID.into());
                    }
                }
                if matches!(role, "element" | "attribute" | "field-contract") {
                    let key = fields.get("name").filter(|v| name(v)).ok_or(INVALID)?;
                    if !names.insert((role.to_owned(), key.to_string())) {
                        return Err(INVALID.into());
                    }
                    roles.insert(id, role.into());
                }
                let parent = match role {
                    "schema" => "schema",
                    "elements" => "elements",
                    "attributes" => "attributes",
                    "field-contracts" => "field-contracts",
                    _ => "leaf",
                };
                pending.extend(children.iter().rev().map(|id| (*id, parent)));
            }
            CemAstNode::Text { data, .. } | CemAstNode::Whitespace { data, .. }
                if data.trim().is_empty() => {}
            CemAstNode::Comment { .. } => {}
            _ => return Err(INVALID.into()),
        }
    }
    if schema_count != 1 {
        return Err(INVALID.into());
    }
    Ok(roles)
}
pub(super) fn field(document: &CemDocument, id: u32, key: &str) -> Option<String> {
    let CemAstNode::Element { attributes, .. } = document.get(id)? else {
        return None;
    };
    attributes.iter().find_map(|id| match document.get(*id)? {
        CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        } if expanded_name.local_name == key => value.clone(),
        _ => None,
    })
}
