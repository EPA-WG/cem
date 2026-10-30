//! Literal source arena and occurrence verification.
use super::structural_capsule::SourceIdentity;
use crate::{
    parser::{document::CemDocument, AstNodeId, CemAstNode, ExpandedName},
    source::SourceId,
    source_map::SourceMapStack,
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
mod tests;
const INVALID: &str = "native source arena refused";
const MAX_RECORDS: usize = 16_384;
fn uri(value: &str) -> bool {
    value.len() <= 4096 && url::Url::parse(value).is_ok_and(|u| !u.cannot_be_a_base())
}
fn valid_hash(value: &str) -> bool {
    value.strip_prefix("cem-bin/1+blake3:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceNode {
    pub(crate) kind: String,
    pub(crate) name: Option<ExpandedName>,
    pub(crate) attributes: Vec<AstNodeId>,
    pub(crate) children: Vec<AstNodeId>,
    pub(crate) value: Option<String>,
    pub(crate) boundary: bool,
    pub(crate) source: SourceMapStack,
}

/// Passive native source records. Text is retained exactly, but no reference
/// nodes, live scope, runtime policy or executable registration is admitted.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSourceSnapshot {
    nodes: Vec<SourceNode>,
    lexical: super::machine::LexicalReloadMetadata,
}
impl NativeSourceSnapshot {
    pub fn capture(
        capture: &super::machine::LexicallyScopedDocument,
        identity: &SourceIdentity,
        poll: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<Self, String> {
        poll()?;
        let nodes =
            nodes(capture.document(), true).map_err(|_| "native source node capture refused")?;
        // Validate the original owning graph as well as the restored one.
        restore(&nodes, identity, true, poll).map_err(|_| "native source graph capture refused")?;
        let fingerprint =
            *blake3::hash(&rmp_serde::to_vec_named(&nodes).map_err(|_| INVALID)?).as_bytes();
        let result = Self {
            nodes,
            lexical: super::machine::LexicalReloadMetadata::export(capture, fingerprint),
        };
        result
            .restore(identity, poll)
            .map_err(|_| "native source lexical capture refused")?;
        Ok(result)
    }
    pub fn restore(
        &self,
        identity: &SourceIdentity,
        poll: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<std::sync::Arc<super::machine::LexicallyScopedDocument>, String> {
        poll()?;
        let mut document = restore(&self.nodes, identity, true, poll)?;
        document.format_identity = self.lexical.format_identity.clone();
        if self.lexical.version != 2
            || !self.lexical.typed_preludes.is_empty()
            || !self.lexical.diagnostics.is_empty()
            || !self.lexical.pending_namespace_declarations.is_empty()
            || !self.lexical.pending_namespace_names.is_empty()
            || self
                .lexical
                .pending_namespace_bindings
                .values()
                .any(|pending| !pending.is_empty())
        {
            return Err(INVALID.into());
        }
        // Lexical validation performs no source parsing or evaluation. Its
        // passive occurrences remain tied to the new arena, never old handles.
        let fingerprint =
            *blake3::hash(&rmp_serde::to_vec_named(&self.nodes).map_err(|_| INVALID)?).as_bytes();
        self.lexical
            .validate(&document, fingerprint)
            .map_err(|_| INVALID)?;
        for map in self.lexical.source_maps() {
            poll()?;
            if map.frames.len() > 64 {
                return Err(INVALID.into());
            }
            for frame in &map.frames {
                poll()?;
                if frame.source_id != identity.id
                    || matches!(&frame.transform, crate::source_map::TransformKind::SourceBoundary { uri } if uri != &identity.uri)
                    || matches!(&frame.transform, crate::source_map::TransformKind::ScssOrigin { module_uri, .. } if module_uri != &identity.uri)
                {
                    return Err(INVALID.into());
                }
                let ranges = match &frame.span {
                    crate::source_map::FrameSpan::Single(range) => std::slice::from_ref(range),
                    crate::source_map::FrameSpan::Multi(ranges) => ranges.as_slice(),
                };
                if ranges.len() > 64 {
                    return Err(INVALID.into());
                }
                for range in ranges {
                    poll()?;
                    if range
                        .start
                        .checked_add(range.len as u64)
                        .is_none_or(|end| end > identity.byte_length as u64)
                    {
                        return Err(INVALID.into());
                    }
                }
            }
        }
        poll()?;
        Ok(std::sync::Arc::new(
            self.lexical.restore(std::sync::Arc::new(document)),
        ))
    }
}

pub(crate) fn nodes(document: &CemDocument, text: bool) -> Result<Vec<SourceNode>, String> {
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
                    if text || data.trim().is_empty() =>
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
                CemAstNode::RawText { node_id, data, .. } if text => {
                    result.kind = "raw-text".into();
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
pub(crate) fn restore(
    nodes: &[SourceNode],
    source: &SourceIdentity,
    text: bool,
    poll: &mut dyn FnMut() -> Result<(), String>,
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
        poll()?;
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
            "text" | "raw-text" | "whitespace" | "comment"
                if index > 0
                    && node.name.is_none()
                    && node.attributes.is_empty()
                    && node.children.is_empty()
                    && node.boundary =>
            {
                let data = node.value.clone().ok_or(INVALID)?;
                match node.kind.as_str() {
                    "text" if text || data.trim().is_empty() => CemAstNode::Text {
                        node_id,
                        data,
                        source,
                    },
                    "whitespace" if data.trim().is_empty() => CemAstNode::Whitespace {
                        node_id,
                        data,
                        source,
                    },
                    "raw-text" if text => CemAstNode::RawText {
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
        poll()?;
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
