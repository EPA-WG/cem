//! Literal schema source vocabulary; arena checks are shared with portable
//! query declarations, keeping this format's original whitespace-only profile.
use super::*;
pub(super) fn nodes(document: &CemDocument) -> Result<Vec<SourceNode>, String> {
    crate::schema::native_source::nodes(document, false)
}
pub(super) fn restore(
    nodes: &[SourceNode],
    source: &SourceIdentity,
    control: &Work<'_>,
) -> Result<CemDocument, String> {
    crate::schema::native_source::restore(nodes, source, false, &mut || check(control))
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
