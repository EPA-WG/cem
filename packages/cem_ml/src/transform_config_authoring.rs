//! Bounded graph authoring over the native CEM AST. Hosts never parse graph text.
use crate::{
    diagnostics::Severity,
    engine::FormatIdentity,
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode, ExpandedName},
    source::{BytesSource, SourceId},
    source_map::SourceMapStack,
    tokenizer::cem::CemTokenizer,
    transform_config::{
        parse_transform_graph_config, TransformGraphConfig, TransformGraphEdge,
        TransformGraphEdgeRole, TransformGraphNode, TransformGraphParseRequest,
        TRANSFORM_CONFIG_SCHEMA_ELEMENTS,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphAuthoringRequest {
    pub source: String,
    pub edit: Option<GraphAuthoringEdit>,
    pub create: Option<GraphAuthoringCreate>,
    pub edge: Option<GraphAuthoringEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphAuthoringEdit {
    pub node: String,
    pub attribute: String,
    /// None removes an optional attribute. Empty text is a literal value.
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphAuthoringCreate {
    /// None appends an import to the run root; otherwise use an authored parent ID.
    pub parent: Option<String>,
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphAuthoringEdge {
    pub node: String,
    pub role: TransformGraphEdgeRole,
    /// None restores parent input, removes one named binding, or chooses the run root.
    pub source: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphAuthoringEdgeField {
    pub id: &'static str,
    pub source: Option<String>,
    pub can_clear: bool,
    pub requires_name: bool,
    pub bindings: Vec<GraphAuthoringBinding>,
}

#[derive(Debug, Serialize)]
pub struct GraphAuthoringBinding {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct GraphAuthoringParent {
    pub id: String,
    pub kind: String,
    pub children: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct GraphAuthoringType {
    pub id: &'static str,
    pub fields: Vec<GraphAuthoringField>,
}

#[derive(Debug, Serialize)]
pub struct GraphAuthoringResponse {
    pub source: String,
    pub nodes: Vec<GraphAuthoringNode>,
    pub parents: Vec<GraphAuthoringParent>,
    #[serde(rename = "nodeTypes")]
    pub node_types: Vec<GraphAuthoringType>,
    pub graph: TransformGraphConfig,
}

#[derive(Debug, Serialize)]
pub struct GraphAuthoringNode {
    pub id: String,
    pub kind: String,
    pub fields: Vec<GraphAuthoringField>,
    pub named: bool,
    pub edges: Vec<GraphAuthoringEdgeField>,
}

#[derive(Debug, Serialize)]
pub struct GraphAuthoringField {
    pub id: String,
    pub value: Option<String>,
    pub required: bool,
    pub choices: Vec<&'static str>,
    #[serde(rename = "hasChoices")]
    pub has_choices: bool,
}

/// Describe, edit one scalar attribute, or append one named node to the native AST.
/// Existing IDs/topology, expressions, aliases and parameter maps remain intact.
pub fn author_transform_config(
    request: GraphAuthoringRequest,
) -> Result<GraphAuthoringResponse, String> {
    if [
        request.edit.is_some(),
        request.create.is_some(),
        request.edge.is_some(),
    ]
    .into_iter()
    .filter(|active| *active)
    .count()
        > 1
    {
        return Err("Choose one graph authoring operation per request".into());
    }
    let (mut document, mut response) = describe(&request.source)?;
    if let Some(edge) = request.edge {
        return edit_edge(document, response, edge);
    }
    if let Some(create) = request.create {
        return create_node(document, response, create);
    }
    let Some(edit) = request.edit else {
        return Ok(response);
    };
    let field = response
        .nodes
        .iter()
        .find(|node| node.id == edit.node)
        .and_then(|node| node.fields.iter().find(|field| field.id == edit.attribute))
        .ok_or("Choose an editable named graph node and attribute")?;
    if field.required && edit.value.is_none() {
        return Err("A required graph attribute cannot be reset".into());
    }
    if let Some(value) = &edit.value {
        if value.contains('"') && value.contains('\'') {
            return Err(
                "A graph literal containing both quote delimiters must be edited in source".into(),
            );
        }
        if !field.choices.is_empty() && !field.choices.contains(&value.as_str()) {
            return Err("Choose a native graph attribute value".into());
        }
    }
    if field.value == edit.value {
        return Ok(response);
    }
    let owner = *document
        .id_table
        .get(&edit.node)
        .ok_or("Graph node has no authored ID")?;
    let attributes = match &document.nodes[owner as usize] {
        CemAstNode::Element { attributes, .. } => attributes.clone(),
        _ => unreachable!(),
    };
    let existing = attributes.iter().copied().find(|id| matches!(&document.nodes[*id as usize], CemAstNode::Attribute { expanded_name, .. } if expanded_name.namespace_uri.is_empty() && expanded_name.local_name == edit.attribute));
    if let Some(value) = &edit.value {
        if let Some(id) = existing {
            if let CemAstNode::Attribute {
                value: current,
                value_nodes,
                ..
            } = &mut document.nodes[id as usize]
            {
                *current = Some(value.clone());
                value_nodes.clear();
            }
        } else {
            let id = document.nodes.len() as u32;
            document.nodes.push(CemAstNode::Attribute {
                node_id: id,
                expanded_name: ExpandedName {
                    namespace_uri: String::new(),
                    local_name: edit.attribute.clone(),
                    schema_id: None,
                },
                value: Some(value.clone()),
                value_nodes: vec![],
                source: SourceMapStack::default(),
            });
            if let CemAstNode::Element { attributes, .. } = &mut document.nodes[owner as usize] {
                attributes.push(id);
            }
        }
    } else if let CemAstNode::Element { attributes, .. } = &mut document.nodes[owner as usize] {
        attributes.retain(|id| Some(*id) != existing);
    }
    let source = crate::formatter::format(&document);
    let (_, candidate) = describe(&source)?;
    // A formatter must never silently reinterpret a literal or an unrelated node.
    let actual = candidate
        .nodes
        .iter()
        .find(|node| node.id == edit.node)
        .and_then(|node| node.fields.iter().find(|field| field.id == edit.attribute));
    if actual.map(|field| &field.value) != Some(&edit.value)
        || response
            .graph
            .nodes
            .iter()
            .filter(|node| node.id != edit.node)
            .any(|node| !candidate.graph.nodes.contains(node))
    {
        return Err("Graph serialization changed an authored value".into());
    }
    response = candidate;
    Ok(response)
}

fn edge_role(role: TransformGraphEdgeRole) -> &'static str {
    match role {
        TransformGraphEdgeRole::Parent => "parent",
        TransformGraphEdgeRole::Input => "input",
        TransformGraphEdgeRole::With => "with",
    }
}

fn parent_of(document: &CemDocument, node: u32) -> Option<u32> {
    document.nodes.iter().find_map(|candidate| match candidate {
        CemAstNode::Element {
            node_id, children, ..
        } if children.contains(&node) => Some(*node_id),
        _ => None,
    })
}

fn parent_identity(document: &CemDocument, owner: u32) -> Option<Option<String>> {
    let parent = parent_of(document, owner)?;
    if matches!(&document.nodes[parent as usize], CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "run")
    {
        return Some(None);
    }
    document
        .id_table
        .iter()
        .find(|(_, id)| **id == parent)
        .map(|(id, _)| Some(id.clone()))
}

fn describe_edges(
    document: &CemDocument,
    owner: u32,
    node: &TransformGraphNode,
) -> Vec<GraphAuthoringEdgeField> {
    let kind = if node.kind == crate::transform_config::TransformGraphNodeKind::ImportMapRewrite {
        "rewrite-importmap".into()
    } else {
        serde_json::to_value(node.kind)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    };
    let schema = TRANSFORM_CONFIG_SCHEMA_ELEMENTS
        .iter()
        .find(|schema| schema.local_name == kind)
        .unwrap();
    let mut fields = vec![];
    if let Some(parent) = parent_identity(document, owner) {
        fields.push(GraphAuthoringEdgeField {
            id: "parent",
            source: parent,
            can_clear: kind == "import",
            requires_name: false,
            bindings: vec![],
        });
    }
    if schema.optional_attributes.contains(&"input") {
        fields.push(GraphAuthoringEdgeField {
            id: "input",
            source: node.input_ref.clone(),
            can_clear: true,
            requires_name: false,
            bindings: vec![],
        });
    }
    if schema.optional_attributes.contains(&"with:*") {
        fields.push(GraphAuthoringEdgeField {
            id: "with",
            source: None,
            can_clear: true,
            requires_name: true,
            bindings: node
                .with
                .iter()
                .map(|(name, source)| GraphAuthoringBinding {
                    name: name.clone(),
                    source: source.clone(),
                })
                .collect(),
        });
    }
    fields
}

fn edge_keys(edges: &[TransformGraphEdge]) -> Vec<(String, String, &'static str)> {
    let mut keys: Vec<_> = edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone(), edge_role(edge.role)))
        .collect();
    keys.sort();
    keys
}

fn edit_edge(
    mut document: CemDocument,
    before: GraphAuthoringResponse,
    edge: GraphAuthoringEdge,
) -> Result<GraphAuthoringResponse, String> {
    let role = edge_role(edge.role);
    let node = before
        .nodes
        .iter()
        .find(|node| node.id == edge.node && node.named)
        .ok_or("Choose a named graph edge target")?;
    let field = node
        .edges
        .iter()
        .find(|field| field.id == role)
        .ok_or("This node does not support the selected edge role")?;
    if field.requires_name {
        let name = edge
            .name
            .as_deref()
            .ok_or("A secondary input needs a binding name")?;
        if name.is_empty()
            || name.len() > 128
            || !name.chars().enumerate().all(|(index, c)| {
                c.is_ascii_alphabetic()
                    || c == '_'
                    || (index > 0 && (c.is_ascii_digit() || c == '-' || c == '.'))
            })
        {
            return Err("Use a binding name of 1–128 ASCII name characters starting with a letter or underscore".into());
        }
    } else if edge.name.is_some() {
        return Err("Only secondary edges accept a binding name".into());
    }
    if edge.source.is_none() && !field.can_clear {
        return Err("This graph node cannot move to the run root".into());
    }
    if let Some(source) = &edge.source {
        if !before
            .nodes
            .iter()
            .any(|node| node.id == *source && node.named)
        {
            return Err("Choose a named graph source node".into());
        }
    }
    let owner = *document.id_table.get(&edge.node).unwrap();
    let current_parent = parent_identity(&document, owner)
        .ok_or("The lexical parent needs an authored ID before edge editing")?;
    let mut expected_nodes = before.graph.nodes.clone();
    let expected = expected_nodes
        .iter_mut()
        .find(|node| node.id == edge.node)
        .unwrap();
    let mut expected_edges = before.graph.edges.clone();
    if edge.role == TransformGraphEdgeRole::Parent {
        let parent = before
            .parents
            .iter()
            .find(|parent| {
                Some(&parent.id) == edge.source.as_ref()
                    || (edge.source.is_none() && parent.id.is_empty())
            })
            .filter(|parent| parent.children.contains(&node.kind.as_str()))
            .ok_or("Choose a schema-approved graph parent")?;
        if current_parent == edge.source {
            return Ok(before);
        }
        let destination = if parent.id.is_empty() {
            let Some(CemAstNode::Document { root_children, .. }) = document.root() else {
                unreachable!()
            };
            *root_children.iter().find(|id| matches!(&document.nodes[**id as usize], CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "run")).unwrap()
        } else {
            *document.id_table.get(&parent.id).unwrap()
        };
        let mut descendants = vec![owner];
        while let Some(id) = descendants.pop() {
            if id == destination {
                return Err("Graph parent change would create a cycle".into());
            }
            if let CemAstNode::Element { children, .. } = &document.nodes[id as usize] {
                descendants.extend(children);
            }
        }
        let old = parent_of(&document, owner).unwrap();
        if let CemAstNode::Element { children, .. } = &mut document.nodes[old as usize] {
            children.retain(|id| *id != owner);
        }
        if let CemAstNode::Element { children, .. } = &mut document.nodes[destination as usize] {
            children.push(owner);
        }
        if expected.input_ref.is_none() {
            expected_edges
                .retain(|old| !(old.to == edge.node && old.role == TransformGraphEdgeRole::Parent));
            if let Some(source) = &edge.source {
                expected_edges.push(TransformGraphEdge {
                    from: source.clone(),
                    to: edge.node.clone(),
                    role: TransformGraphEdgeRole::Parent,
                });
            }
        }
    } else {
        let (namespace, attribute) = if edge.role == TransformGraphEdgeRole::Input {
            ("", "input")
        } else {
            ("with", edge.name.as_deref().unwrap())
        };
        let attributes = match &document.nodes[owner as usize] {
            CemAstNode::Element { attributes, .. } => attributes.clone(),
            _ => unreachable!(),
        };
        let existing = attributes.iter().copied().find(|id| matches!(&document.nodes[*id as usize], CemAstNode::Attribute { expanded_name, .. } if expanded_name.namespace_uri == namespace && expanded_name.local_name == attribute));
        if existing.is_some_and(|id| matches!(&document.nodes[id as usize], CemAstNode::Attribute { value_nodes, .. } if !value_nodes.is_empty())) { return Err("Expression-valued edges must be edited in source".into()); }
        let previous = if edge.role == TransformGraphEdgeRole::Input {
            expected.input_ref.clone()
        } else {
            expected.with.get(attribute).cloned()
        };
        if previous == edge.source {
            return Ok(before);
        }
        if let Some(value) = &edge.source {
            if let Some(id) = existing {
                if let CemAstNode::Attribute {
                    value: current,
                    value_nodes,
                    ..
                } = &mut document.nodes[id as usize]
                {
                    *current = Some(value.clone());
                    value_nodes.clear();
                }
            } else {
                let node_id = document.nodes.len() as u32;
                document.nodes.push(CemAstNode::Attribute {
                    node_id,
                    expanded_name: ExpandedName {
                        namespace_uri: namespace.into(),
                        local_name: attribute.into(),
                        schema_id: None,
                    },
                    value: Some(value.clone()),
                    value_nodes: vec![],
                    source: SourceMapStack::default(),
                });
                if let CemAstNode::Element { attributes, .. } = &mut document.nodes[owner as usize]
                {
                    attributes.push(node_id);
                }
            }
        } else if let CemAstNode::Element { attributes, .. } = &mut document.nodes[owner as usize] {
            attributes.retain(|id| Some(*id) != existing);
        }
        if edge.role == TransformGraphEdgeRole::Input {
            expected.input_ref = edge.source.clone();
            expected_edges.retain(|old| {
                !(old.to == edge.node
                    && matches!(
                        old.role,
                        TransformGraphEdgeRole::Parent | TransformGraphEdgeRole::Input
                    ))
            });
            if let Some(source) = edge.source.as_ref().or(current_parent.as_ref()) {
                expected_edges.push(TransformGraphEdge {
                    from: source.clone(),
                    to: edge.node.clone(),
                    role: if edge.source.is_some() {
                        TransformGraphEdgeRole::Input
                    } else {
                        TransformGraphEdgeRole::Parent
                    },
                });
            }
        } else {
            if let Some(previous) = previous {
                if let Some(index) = expected_edges.iter().position(|old| {
                    old.to == edge.node
                        && old.from == previous
                        && old.role == TransformGraphEdgeRole::With
                }) {
                    expected_edges.remove(index);
                }
            }
            if let Some(source) = &edge.source {
                expected.with.insert(attribute.into(), source.clone());
                expected_edges.push(TransformGraphEdge {
                    from: source.clone(),
                    to: edge.node.clone(),
                    role: TransformGraphEdgeRole::With,
                });
            } else {
                expected.with.remove(attribute);
            }
        }
    }
    let (_, candidate) = describe(&crate::formatter::format(&document))?;
    if candidate.graph.nodes.len() != expected_nodes.len()
        || expected_nodes
            .iter()
            .any(|node| !candidate.graph.nodes.contains(node))
        || edge_keys(&candidate.graph.edges) != edge_keys(&expected_edges)
        || (edge.role == TransformGraphEdgeRole::Parent
            && candidate
                .nodes
                .iter()
                .find(|node| node.id == edge.node)
                .and_then(|node| node.edges.iter().find(|field| field.id == "parent"))
                .map(|field| &field.source)
                != Some(&edge.source))
    {
        return Err("Graph edge serialization changed an existing identity, value or unrelated edge; use authored IDs or edit in source".into());
    }
    Ok(candidate)
}

fn describe_field(
    name: &str,
    kind: &str,
    value: Option<String>,
    required: bool,
) -> GraphAuthoringField {
    let choices = match (name, kind) {
        ("opaque", _) => vec!["true", "false"],
        ("mode", "join") => vec!["collect", "group-by", "match-by", "zip"],
        ("mode", "rewrite-importmap") => vec!["replace-imports", "merge", "replace-script"],
        ("missing", _) => vec!["error", "ignore", "insert"],
        ("style-policy", _) => vec!["auto", "inline", "link", "omit"],
        _ => vec![],
    };
    GraphAuthoringField {
        id: name.into(),
        value,
        required,
        has_choices: !choices.is_empty(),
        choices,
    }
}

fn create_node(
    mut document: CemDocument,
    before: GraphAuthoringResponse,
    create: GraphAuthoringCreate,
) -> Result<GraphAuthoringResponse, String> {
    if create.id.is_empty() || create.id.trim() != create.id || create.id.len() > 256 {
        return Err(
            "A graph node needs an authored ID of 1–256 bytes without surrounding whitespace"
                .into(),
        );
    }
    if document.id_table.contains_key(&create.id)
        || before.graph.nodes.iter().any(|node| node.id == create.id)
    {
        return Err("Graph node ID is already in use".into());
    }
    let parent = before
        .parents
        .iter()
        .find(|parent| parent.id == create.parent.as_deref().unwrap_or(""))
        .filter(|parent| parent.children.contains(&create.kind.as_str()))
        .ok_or("Choose a schema-approved node kind and authored parent")?;
    let kind = before
        .node_types
        .iter()
        .find(|kind| kind.id == create.kind)
        .ok_or("Choose a native graph node kind")?;
    for (name, value) in &create.attributes {
        let field = kind
            .fields
            .iter()
            .find(|field| field.id == *name)
            .ok_or("Unexpected graph creation attribute")?;
        if !field.choices.is_empty() && !field.choices.contains(&value.as_str()) {
            return Err("Choose a native graph attribute value".into());
        }
    }
    for field in &kind.fields {
        if field.required && !create.attributes.contains_key(&field.id) {
            return Err(format!("New {} node requires @{}", kind.id, field.id));
        }
    }
    if std::iter::once(&create.id)
        .chain(create.attributes.values())
        .any(|value| value.contains('"') && value.contains('\''))
    {
        return Err(
            "A graph literal containing both quote delimiters must be edited in source".into(),
        );
    }
    let owner = if parent.id.is_empty() {
        let Some(CemAstNode::Document { root_children, .. }) = document.root() else {
            unreachable!()
        };
        *root_children.iter().find(|id| matches!(&document.nodes[**id as usize], CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "run")).unwrap()
    } else {
        *document.id_table.get(&parent.id).unwrap()
    };
    let namespace = match &document.nodes[owner as usize] {
        CemAstNode::Element { expanded_name, .. } => expanded_name.namespace_uri.clone(),
        _ => unreachable!(),
    };
    let mut attributes = vec![];
    for (name, value) in std::iter::once(("id", &create.id)).chain(
        create
            .attributes
            .iter()
            .map(|(name, value)| (name.as_str(), value)),
    ) {
        let node_id = document.nodes.len() as u32;
        document.nodes.push(CemAstNode::Attribute {
            node_id,
            expanded_name: ExpandedName {
                namespace_uri: String::new(),
                local_name: name.into(),
                schema_id: None,
            },
            value: Some(value.clone()),
            value_nodes: vec![],
            source: SourceMapStack::default(),
        });
        attributes.push(node_id);
    }
    let node_id = document.nodes.len() as u32;
    document.nodes.push(CemAstNode::Element {
        node_id,
        expanded_name: ExpandedName {
            namespace_uri: namespace,
            local_name: create.kind.clone(),
            schema_id: None,
        },
        attributes,
        children: vec![],
        has_explicit_boundary: true,
        source: SourceMapStack::default(),
    });
    if let CemAstNode::Element { children, .. } = &mut document.nodes[owner as usize] {
        children.push(node_id);
    }
    let (_, candidate) = describe(&crate::formatter::format(&document))?;
    let added = candidate
        .nodes
        .iter()
        .find(|node| node.id == create.id && node.kind == create.kind);
    if candidate.graph.nodes.len() != before.graph.nodes.len() + 1
        || added.is_none()
        || before
            .graph
            .nodes
            .iter()
            .any(|node| !candidate.graph.nodes.contains(node))
        || before
            .graph
            .edges
            .iter()
            .any(|edge| !candidate.graph.edges.contains(edge))
        || create.attributes.iter().any(|(name, value)| {
            !added
                .unwrap()
                .fields
                .iter()
                .any(|field| field.id == *name && field.value.as_ref() == Some(value))
        })
    {
        return Err("Graph creation changed an existing identity or authored value; add authored IDs to anonymous nodes or edit in source".into());
    }
    Ok(candidate)
}

fn describe(source: &str) -> Result<(CemDocument, GraphAuthoringResponse), String> {
    if source.len() > 1024 * 1024 {
        return Err("Graph source exceeds the 1 MiB authoring limit".into());
    }
    let parsed = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: source.as_bytes().to_vec(),
        identity: FormatIdentity::default(),
        base_uri: None,
    })
    .map_err(|error| error.to_string())?;
    if let Some(diagnostic) = parsed
        .diagnostics
        .iter()
        .find(|diagnostic| matches!(diagnostic.severity, Severity::Error | Severity::Fatal))
    {
        return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    if parsed.graph.nodes.len() > 100 {
        return Err("Graph exceeds the 100-node authoring limit".into());
    }
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
    )))
    .build();
    for node in &document.nodes {
        if let CemAstNode::Element { attributes, .. } = node {
            let mut seen = BTreeSet::new();
            for id in attributes {
                if let CemAstNode::Attribute { expanded_name, .. } = &document.nodes[*id as usize] {
                    if !seen.insert((&expanded_name.namespace_uri, &expanded_name.local_name)) {
                        return Err("Duplicate graph attributes are ambiguous".into());
                    }
                }
            }
        }
    }
    let nodes: Vec<GraphAuthoringNode> = parsed.graph.nodes.iter().map(|node| {
        let kind = serde_json::to_value(node.kind).unwrap().as_str().unwrap().to_owned();
        let kind = if kind == "import-map-rewrite" { "rewrite-importmap".into() } else { kind };
        let schema = TRANSFORM_CONFIG_SCHEMA_ELEMENTS.iter().find(|schema| schema.local_name == kind).unwrap();
        let attributes = document.id_table.get(&node.id).and_then(|id| match &document.nodes[*id as usize] { CemAstNode::Element { attributes, .. } => Some(attributes), _ => None });
        let fields = attributes.map(|attributes| schema.required_attributes.iter().chain(schema.optional_attributes).filter(|name| **name != "id" && !name.contains(':') && !name.chars().any(char::is_uppercase)).filter_map(|name| {
            let attribute = attributes.iter().find_map(|id| match &document.nodes[*id as usize] { CemAstNode::Attribute { expanded_name, value, value_nodes, .. } if expanded_name.namespace_uri.is_empty() && expanded_name.local_name == *name => Some((value, value_nodes)), _ => None });
            // Alias spellings and expression-valued attributes remain source-only.
            let alias = match *name { "content-type" => "contentType", "template-content-type" => "templateContentType", "template-schema" => "templateSchema", "source-map" => "sourceMap", "target-map" => "targetMap", "style-policy" => "stylePolicy", _ => "" };
            if attribute.is_some_and(|(_, values)| !values.is_empty()) || attributes.iter().any(|id| matches!(&document.nodes[*id as usize], CemAstNode::Attribute { expanded_name, .. } if !alias.is_empty() && expanded_name.local_name == alias)) { return None; }
            Some(describe_field(name, &kind, attribute.and_then(|(value, _)| value.clone()), schema.required_attributes.contains(name)))
        }).collect()).unwrap_or_default();
        GraphAuthoringNode { id: node.id.clone(), kind, fields, named: attributes.is_some(), edges: document.id_table.get(&node.id).map(|owner| describe_edges(&document, *owner, node)).unwrap_or_default() }
    }).collect();
    let node_types = TRANSFORM_CONFIG_SCHEMA_ELEMENTS
        .iter()
        .filter(|schema| !matches!(schema.local_name, "run" | "param"))
        .map(|schema| GraphAuthoringType {
            id: schema.local_name,
            fields: schema
                .required_attributes
                .iter()
                .chain(schema.optional_attributes)
                .filter(|name| {
                    **name != "id" && !name.contains(':') && !name.chars().any(char::is_uppercase)
                })
                .map(|name| {
                    describe_field(
                        name,
                        schema.local_name,
                        None,
                        schema.required_attributes.contains(name),
                    )
                })
                .collect(),
        })
        .collect();
    let mut parents = vec![GraphAuthoringParent {
        id: String::new(),
        kind: "run".into(),
        children: TRANSFORM_CONFIG_SCHEMA_ELEMENTS
            .iter()
            .find(|schema| schema.local_name == "run")
            .unwrap()
            .child_elements
            .to_vec(),
    }];
    for node in &nodes {
        if !document.id_table.contains_key(&node.id) {
            continue;
        }
        let schema = TRANSFORM_CONFIG_SCHEMA_ELEMENTS
            .iter()
            .find(|schema| schema.local_name == node.kind)
            .unwrap();
        let children: Vec<_> = schema
            .child_elements
            .iter()
            .copied()
            .filter(|name| *name != "param")
            .collect();
        if !children.is_empty() {
            parents.push(GraphAuthoringParent {
                id: node.id.clone(),
                kind: node.kind.clone(),
                children,
            });
        }
    }
    Ok((
        document,
        GraphAuthoringResponse {
            source: source.into(),
            nodes,
            parents,
            node_types,
            graph: parsed.graph,
        },
    ))
}

/// JSON is an explicit host protocol boundary; graph text always uses the native AST.
pub fn author_transform_config_json(request: &str) -> String {
    let result = serde_json::from_str::<GraphAuthoringRequest>(request)
        .map_err(|error| error.to_string())
        .and_then(author_transform_config);
    match result {
        Ok(value) => serde_json::to_string(&value).unwrap(),
        Err(message) => serde_json::json!({"error": message}).to_string(),
    }
}
