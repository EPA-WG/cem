//! AI-facing context projections over canonical CEM AST data.
//!
//! This module deliberately projects from [`CemDocument`] instead of replacing
//! canonical AST/DOM/event projections. Records carry canonical `cem-ast://`
//! refs and source maps so hosts can expand back to authoritative data.

use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source_map::SourceMapStack;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextProjectionKind {
    #[default]
    ContextPack,
    EntityGraph,
    SemanticTokens,
    ContextFragment,
    EmbeddingRecord,
}

impl AiContextProjectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContextPack => "ai-context-pack",
            Self::EntityGraph => "ai-entity-graph",
            Self::SemanticTokens => "ai-semantic-tokens",
            Self::ContextFragment => "ai-context-fragment",
            Self::EmbeddingRecord => "ai-embedding-record",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextProjectionRequest {
    #[serde(default)]
    pub kind: AiContextProjectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<AstNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextProjection {
    pub kind: AiContextProjectionKind,
    pub metadata: AiContextProjectionMetadata,
    pub records: Vec<AiContextRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextProjectionMetadata {
    pub canonical_projection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_ref: Option<String>,
    pub record_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextRecordKind {
    Document,
    Element,
    Attribute,
    Text,
    Whitespace,
    Comment,
    ProcessingInstruction,
    Cdata,
    RawText,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextBoundary {
    Data,
    Instruction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextRecord {
    pub id: String,
    pub canonical_ref: String,
    pub node_id: AstNodeId,
    pub kind: AiContextRecordKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facets: Vec<String>,
    pub boundary: AiContextBoundary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
}

pub fn project_cem_document_for_ai(
    document: &CemDocument,
    request: AiContextProjectionRequest,
) -> AiContextProjection {
    let root = request.root.unwrap_or(0);
    let parents = ai_context_parent_map(document);
    let mut node_ids = Vec::new();
    let mut seen = BTreeSet::new();
    collect_ai_context_node_ids(document, root, &mut seen, &mut node_ids);
    let included_ids = node_ids
        .iter()
        .copied()
        .filter(|node_id| {
            document
                .get(*node_id)
                .is_some_and(|node| ai_context_includes_node(request.kind, node))
        })
        .collect::<BTreeSet<_>>();

    let mut records = Vec::new();
    for node_id in node_ids {
        let Some(node) = document.get(node_id) else {
            continue;
        };
        if !included_ids.contains(&node_id) {
            continue;
        }
        records.push(ai_context_record_for_node(
            document,
            request.kind,
            node,
            parents.get(&node_id).copied(),
            &included_ids,
        ));
    }

    AiContextProjection {
        kind: request.kind,
        metadata: AiContextProjectionMetadata {
            canonical_projection: "cem-ast".to_owned(),
            root_ref: document.get(root).map(|_| ai_context_canonical_ref(root)),
            record_count: records.len(),
        },
        records,
    }
}

fn ai_context_record_for_node(
    document: &CemDocument,
    projection: AiContextProjectionKind,
    node: &CemAstNode,
    parent: Option<AstNodeId>,
    included_ids: &BTreeSet<AstNodeId>,
) -> AiContextRecord {
    let node_id = ai_context_node_id(node);
    AiContextRecord {
        id: format!("{}:{}", projection.as_str(), node_id),
        canonical_ref: ai_context_canonical_ref(node_id),
        node_id,
        kind: ai_context_record_kind(node),
        label: ai_context_node_label(node),
        text: ai_context_node_text(node),
        parent_ref: parent
            .filter(|parent_id| included_ids.contains(parent_id))
            .map(ai_context_canonical_ref),
        child_refs: ai_context_child_ids(node)
            .into_iter()
            .filter(|child_id| included_ids.contains(child_id))
            .map(ai_context_canonical_ref)
            .collect(),
        attributes: ai_context_element_attributes(document, node),
        facets: ai_context_facets(projection, node),
        boundary: ai_context_boundary(node),
        source_map: ai_context_source_map(node).cloned(),
    }
}

fn collect_ai_context_node_ids(
    document: &CemDocument,
    node_id: AstNodeId,
    seen: &mut BTreeSet<AstNodeId>,
    output: &mut Vec<AstNodeId>,
) {
    let Some(node) = document.get(node_id) else {
        return;
    };
    if !seen.insert(node_id) {
        return;
    }
    output.push(node_id);
    for child_id in ai_context_child_ids(node) {
        collect_ai_context_node_ids(document, child_id, seen, output);
    }
}

fn ai_context_parent_map(document: &CemDocument) -> BTreeMap<AstNodeId, AstNodeId> {
    let mut parents = BTreeMap::new();
    for node in document.iter() {
        let parent = ai_context_node_id(node);
        for child in ai_context_child_ids(node) {
            parents.insert(child, parent);
        }
    }
    parents
}

fn ai_context_includes_node(projection: AiContextProjectionKind, node: &CemAstNode) -> bool {
    match projection {
        AiContextProjectionKind::ContextPack | AiContextProjectionKind::ContextFragment => true,
        AiContextProjectionKind::EntityGraph => matches!(
            node,
            CemAstNode::Document { .. } | CemAstNode::Element { .. } | CemAstNode::Attribute { .. }
        ),
        AiContextProjectionKind::SemanticTokens => !matches!(
            node,
            CemAstNode::Document { .. } | CemAstNode::Whitespace { .. }
        ),
        AiContextProjectionKind::EmbeddingRecord => ai_context_node_text(node)
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty()),
    }
}

fn ai_context_facets(projection: AiContextProjectionKind, node: &CemAstNode) -> Vec<String> {
    let mut facets = vec![projection.as_str().to_owned()];
    match node {
        CemAstNode::Element { .. } => facets.push("node".to_owned()),
        CemAstNode::Attribute { .. } => facets.push("attribute".to_owned()),
        CemAstNode::Text { .. } | CemAstNode::Cdata { .. } | CemAstNode::RawText { .. } => {
            facets.push("text".to_owned())
        }
        CemAstNode::Error { .. } => facets.push("diagnostic".to_owned()),
        _ => {}
    }
    facets
}

fn ai_context_boundary(node: &CemAstNode) -> AiContextBoundary {
    match node {
        CemAstNode::ProcessingInstruction { .. } => AiContextBoundary::Instruction,
        _ => AiContextBoundary::Data,
    }
}

fn ai_context_canonical_ref(node_id: AstNodeId) -> String {
    format!("cem-ast://node/{node_id}")
}

fn ai_context_node_id(node: &CemAstNode) -> AstNodeId {
    match node {
        CemAstNode::Document { node_id, .. }
        | CemAstNode::Element { node_id, .. }
        | CemAstNode::Attribute { node_id, .. }
        | CemAstNode::Text { node_id, .. }
        | CemAstNode::Whitespace { node_id, .. }
        | CemAstNode::Comment { node_id, .. }
        | CemAstNode::ProcessingInstruction { node_id, .. }
        | CemAstNode::Cdata { node_id, .. }
        | CemAstNode::RawText { node_id, .. }
        | CemAstNode::Error { node_id, .. } => *node_id,
    }
}

fn ai_context_record_kind(node: &CemAstNode) -> AiContextRecordKind {
    match node {
        CemAstNode::Document { .. } => AiContextRecordKind::Document,
        CemAstNode::Element { .. } => AiContextRecordKind::Element,
        CemAstNode::Attribute { .. } => AiContextRecordKind::Attribute,
        CemAstNode::Text { .. } => AiContextRecordKind::Text,
        CemAstNode::Whitespace { .. } => AiContextRecordKind::Whitespace,
        CemAstNode::Comment { .. } => AiContextRecordKind::Comment,
        CemAstNode::ProcessingInstruction { .. } => AiContextRecordKind::ProcessingInstruction,
        CemAstNode::Cdata { .. } => AiContextRecordKind::Cdata,
        CemAstNode::RawText { .. } => AiContextRecordKind::RawText,
        CemAstNode::Error { .. } => AiContextRecordKind::Error,
    }
}

fn ai_context_node_label(node: &CemAstNode) -> String {
    match node {
        CemAstNode::Document { .. } => "document".to_owned(),
        CemAstNode::Element { expanded_name, .. } => expanded_name.local_name.clone(),
        CemAstNode::Attribute { expanded_name, .. } => format!("@{}", expanded_name.local_name),
        CemAstNode::Text { .. } => "#text".to_owned(),
        CemAstNode::Whitespace { .. } => "#whitespace".to_owned(),
        CemAstNode::Comment { .. } => "#comment".to_owned(),
        CemAstNode::ProcessingInstruction { target, .. } => format!("?{target}"),
        CemAstNode::Cdata { .. } => "#cdata".to_owned(),
        CemAstNode::RawText { .. } => "#raw-text".to_owned(),
        CemAstNode::Error { code, .. } => format!("!{code}"),
    }
}

fn ai_context_node_text(node: &CemAstNode) -> Option<String> {
    match node {
        CemAstNode::Attribute { value, .. } => value.clone(),
        CemAstNode::Text { data, .. }
        | CemAstNode::Whitespace { data, .. }
        | CemAstNode::Comment { data, .. }
        | CemAstNode::Cdata { data, .. }
        | CemAstNode::RawText { data, .. } => Some(data.clone()),
        CemAstNode::ProcessingInstruction { data, .. } => Some(data.clone()),
        CemAstNode::Error { code, .. } => Some(code.clone()),
        CemAstNode::Document { .. } | CemAstNode::Element { .. } => None,
    }
}

fn ai_context_child_ids(node: &CemAstNode) -> Vec<AstNodeId> {
    match node {
        CemAstNode::Document { root_children, .. } => root_children.clone(),
        CemAstNode::Element {
            attributes,
            children,
            ..
        } => attributes.iter().chain(children.iter()).copied().collect(),
        _ => Vec::new(),
    }
}

fn ai_context_element_attributes(
    document: &CemDocument,
    node: &CemAstNode,
) -> BTreeMap<String, String> {
    let CemAstNode::Element { attributes, .. } = node else {
        return BTreeMap::new();
    };
    let mut output = BTreeMap::new();
    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attr_id)
        else {
            continue;
        };
        output.insert(
            expanded_name.local_name.clone(),
            value.clone().unwrap_or_default(),
        );
    }
    output
}

fn ai_context_source_map(node: &CemAstNode) -> Option<&SourceMapStack> {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => Some(source),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    fn parse(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    #[test]
    fn context_pack_projects_ast_records_with_canonical_refs() {
        let document = parse("{button @id=save @cem:action=primary | Save}");

        let projection = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextPack,
                root: None,
            },
        );

        assert_eq!(projection.kind, AiContextProjectionKind::ContextPack);
        assert_eq!(projection.metadata.canonical_projection, "cem-ast");
        assert_eq!(
            projection.metadata.root_ref.as_deref(),
            Some("cem-ast://node/0")
        );
        assert_eq!(projection.metadata.record_count, projection.records.len());
        let button = projection
            .records
            .iter()
            .find(|record| record.kind == AiContextRecordKind::Element)
            .expect("element record");
        assert_eq!(button.label, "button");
        assert_eq!(button.parent_ref.as_deref(), Some("cem-ast://node/0"));
        assert_eq!(
            button.attributes.get("id").map(String::as_str),
            Some("save")
        );
        assert!(button.source_map.is_some());
        assert!(button.facets.contains(&"ai-context-pack".to_owned()));

        let id_attr = projection
            .records
            .iter()
            .find(|record| record.label == "@id")
            .expect("id attribute record");
        assert_eq!(id_attr.text.as_deref(), Some("save"));
        assert_eq!(id_attr.boundary, AiContextBoundary::Data);

        assert!(projection.records.iter().any(|record| {
            record.kind == AiContextRecordKind::Text && record.text.as_deref() == Some("Save")
        }));
    }

    #[test]
    fn projection_kinds_filter_records_for_ai_tasks() {
        let document = parse("{button @id=save | Save}");
        let element_id = document
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .expect("element node id");

        let fragment = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextFragment,
                root: Some(element_id),
            },
        );
        assert_eq!(
            fragment.metadata.root_ref.as_deref(),
            Some(format!("cem-ast://node/{element_id}").as_str())
        );
        assert!(!fragment
            .records
            .iter()
            .any(|record| record.kind == AiContextRecordKind::Document));
        assert!(fragment
            .records
            .iter()
            .find(|record| record.node_id == element_id)
            .is_some_and(|record| record.parent_ref.is_none()));

        let semantic = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::SemanticTokens,
                root: None,
            },
        );
        assert!(semantic.records.iter().all(|record| {
            record.kind != AiContextRecordKind::Document
                && record.kind != AiContextRecordKind::Whitespace
        }));
        assert!(semantic
            .records
            .iter()
            .all(|record| record.facets.contains(&"ai-semantic-tokens".to_owned())));

        let entity_graph = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::EntityGraph,
                root: None,
            },
        );
        assert!(entity_graph.records.iter().all(|record| matches!(
            record.kind,
            AiContextRecordKind::Document
                | AiContextRecordKind::Element
                | AiContextRecordKind::Attribute
        )));
        let entity_refs = entity_graph
            .records
            .iter()
            .map(|record| record.canonical_ref.as_str())
            .collect::<BTreeSet<_>>();
        assert!(entity_graph.records.iter().all(|record| record
            .parent_ref
            .as_deref()
            .is_none_or(|parent_ref| entity_refs.contains(parent_ref))));
        assert!(entity_graph.records.iter().all(|record| record
            .child_refs
            .iter()
            .all(|child_ref| entity_refs.contains(child_ref.as_str()))));

        let embedding = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::EmbeddingRecord,
                root: None,
            },
        );
        assert!(embedding
            .records
            .iter()
            .any(|record| record.text.as_deref() == Some("Save")));
        assert!(embedding.records.iter().all(|record| {
            record
                .text
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty())
        }));
    }
}
