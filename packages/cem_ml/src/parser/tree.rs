//! Retained, format-independent semantic view of a typed CEM document.
//! Import supplies decoded values; this layer never interprets source syntax.
use super::{document::CemDocument, AstNodeId, CemAstNode, ExpandedName};
use crate::source_map::{FrameSpan, SourceMapStack};
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemTreeNodeKind {
    Document,
    Element,
    Attribute,
    Text,
    Comment,
    ProcessingInstruction,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CemTreeRange {
    pub line: u32,
    pub column: u32,
    pub offset: u64,
    pub length: u64,
}

/// Import-only overrides preserve the public source-oriented AST fields.
#[derive(Debug, Default)]
pub struct CemTreeSemantics {
    /// Import-owned, versioned source/profile fingerprint. Synthetic trees omit it.
    pub source_fingerprint: Option<[u8; 32]>,
    pub document_metadata: Option<CemDocumentMetadata>,
    pub base_uris: BTreeMap<AstNodeId, String>,
    pub sources: BTreeMap<AstNodeId, SourceMapStack>,
    pub values: BTreeMap<AstNodeId, String>,
    pub names: BTreeMap<AstNodeId, ExpandedName>,
    pub omitted: BTreeSet<AstNodeId>,
    pub ranges: BTreeMap<AstNodeId, CemTreeRange>,
}

#[derive(Debug, Default)]
pub struct CemDocumentMetadata {
    pub base_uri: Option<String>,
    pub document_uri: Option<String>,
}

#[derive(Debug)]
pub struct CemTreeNode {
    pub base_uri: Option<String>,
    pub kind: CemTreeNodeKind,
    pub name: Option<ExpandedName>,
    pub value: String,
    pub parent: Option<AstNodeId>,
    pub children: Vec<AstNodeId>,
    pub attributes: Vec<AstNodeId>,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
    pub order: usize,
}

pub struct RetainedCemTree {
    ast: CemDocument,
    source_uri: String,
    metadata: CemDocumentMetadata,
    native_owner: Option<Arc<dyn Any + Send + Sync>>,
    nodes: Vec<CemTreeNode>,
    canonical: Vec<Option<AstNodeId>>,
    source_fingerprint: Option<[u8; 32]>,
    source_lines_known: Vec<bool>,
    source_ranges: Vec<CemTreeRange>,
    // Import-decoded source values before semantic text coalescing. Source-node
    // interpolation must neither decode syntax nor read a neighbour's text.
    source_values: BTreeMap<AstNodeId, String>,
}

/// Incremental string-value traversal. Every visited node yields one fragment
/// (possibly empty), so callers can bound work and poll cancellation even for
/// subtrees that contain no text. The stack grows with depth, not sibling count.
pub struct CemTreeTextFragments<'a> {
    tree: &'a RetainedCemTree,
    root: Option<AstNodeId>,
    pending: Vec<std::slice::Iter<'a, AstNodeId>>,
    source: bool,
}

impl<'a> Iterator for CemTreeTextFragments<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let is_root = self.root.is_some();
        let id = if let Some(id) = self.root.take() {
            id
        } else {
            loop {
                if let Some(id) = self.pending.last_mut()?.next() {
                    break *id;
                }
                self.pending.pop();
            }
        };
        if self.source {
            use CemAstNode::*;
            let value = match self.tree.ast.get(id).expect("validated source node") {
                Document { root_children, .. } => {
                    self.pending.push(root_children.iter());
                    return Some("");
                }
                Element { children, .. } => {
                    self.pending.push(children.iter());
                    return Some("");
                }
                Text { data, .. } | Whitespace { data, .. } | Cdata { data, .. }
                | RawText { data, .. } => data.as_str(),
                Attribute { value, .. } if is_root => value.as_deref().unwrap_or(""),
                Comment { data, .. } | ProcessingInstruction { data, .. } if is_root => data,
                _ => return Some(""),
            };
            Some(self.tree.source_values.get(&id).map_or(value, String::as_str))
        } else {
            let node = &self.tree.nodes[id as usize];
            if matches!(node.kind, CemTreeNodeKind::Document | CemTreeNodeKind::Element) {
                self.pending.push(node.children.iter());
                Some("")
            } else if is_root || node.kind == CemTreeNodeKind::Text {
                Some(&node.value)
            } else {
                Some("")
            }
        }
    }
}

impl std::fmt::Debug for RetainedCemTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetainedCemTree")
            .field("source_uri", &self.source_uri)
            .field("nodes", &self.nodes.len())
            .finish_non_exhaustive()
    }
}

impl RetainedCemTree {
    pub fn new(
        ast: CemDocument,
        source_uri: impl Into<String>,
        source_text: &str,
        semantics: CemTreeSemantics,
        native_owner: Option<Arc<dyn Any + Send + Sync>>,
    ) -> Result<Arc<Self>, String> {
        if !matches!(ast.root(), Some(CemAstNode::Document { node_id: 0, .. })) {
            return Err("A retained CEM tree requires a document root at node 0.".into());
        }
        let line_index = crate::source::line_index::LineIndex::from_utf8(source_text);
        let mut nodes = Vec::with_capacity(ast.nodes.len());
        let mut source_lines_known = Vec::with_capacity(ast.nodes.len());
        for (id, node) in ast.nodes.iter().enumerate() {
            use CemAstNode::*;
            let (node_id, kind, name, value, children, attributes, source) = match node {
                Document {
                    node_id,
                    root_children,
                    source,
                } => (
                    *node_id,
                    CemTreeNodeKind::Document,
                    None,
                    "",
                    root_children.clone(),
                    vec![],
                    source,
                ),
                Element {
                    node_id,
                    expanded_name,
                    children,
                    attributes,
                    source,
                    ..
                } => (
                    *node_id,
                    CemTreeNodeKind::Element,
                    Some(expanded_name.clone()),
                    "",
                    children.clone(),
                    attributes.clone(),
                    source,
                ),
                Attribute {
                    node_id,
                    expanded_name,
                    value,
                    source,
                } => (
                    *node_id,
                    CemTreeNodeKind::Attribute,
                    Some(expanded_name.clone()),
                    value.as_deref().unwrap_or(""),
                    vec![],
                    vec![],
                    source,
                ),
                Text {
                    node_id,
                    data,
                    source,
                }
                | Whitespace {
                    node_id,
                    data,
                    source,
                }
                | Cdata {
                    node_id,
                    data,
                    source,
                }
                | RawText {
                    node_id,
                    data,
                    source,
                } => (
                    *node_id,
                    CemTreeNodeKind::Text,
                    None,
                    data.as_str(),
                    vec![],
                    vec![],
                    source,
                ),
                Comment {
                    node_id,
                    data,
                    source,
                } => (
                    *node_id,
                    CemTreeNodeKind::Comment,
                    None,
                    data.as_str(),
                    vec![],
                    vec![],
                    source,
                ),
                ProcessingInstruction {
                    node_id,
                    target,
                    data,
                    source,
                } => (
                    *node_id,
                    CemTreeNodeKind::ProcessingInstruction,
                    Some(ExpandedName {
                        namespace_uri: String::new(),
                        local_name: target.clone(),
                        schema_id: None,
                    }),
                    data.as_str(),
                    vec![],
                    vec![],
                    source,
                ),
                Error { .. } => {
                    return Err("A CEM error node cannot enter the semantic tree.".into())
                }
            };
            if node_id as usize != id {
                return Err("CEM node IDs must match arena positions.".into());
            }
            let range = semantics
                .ranges
                .get(&node_id)
                .copied()
                .unwrap_or_else(|| source_range(source, &line_index));
            source_lines_known.push(
                semantics.ranges.contains_key(&node_id) || !source_text.is_empty(),
            );
            nodes.push(CemTreeNode {
                base_uri: semantics.base_uris.get(&node_id).cloned(),
                kind,
                name: semantics.names.get(&node_id).cloned().or(name),
                value: semantics
                    .values
                    .get(&node_id)
                    .cloned()
                    .unwrap_or_else(|| value.into()),
                parent: None,
                children,
                attributes,
                source: semantics
                    .sources
                    .get(&node_id)
                    .cloned()
                    .unwrap_or_else(|| source.clone()),
                range,
                order: 0,
            });
        }
        // Validate the entire source graph, including omitted nodes, before normalization.
        let mut seen = vec![false; nodes.len()];
        let mut pending = vec![(0, None, false)];
        let mut order = 0;
        while let Some((id, parent, attribute)) = pending.pop() {
            let Some(node) = nodes.get_mut(id as usize) else {
                return Err("Dangling CEM node reference.".into());
            };
            if seen[id as usize] {
                return Err("CEM trees cannot contain cycles or shared child nodes.".into());
            }
            if (node.kind == CemTreeNodeKind::Attribute) != attribute
                || (id != 0 && node.kind == CemTreeNodeKind::Document)
            {
                return Err("Invalid CEM child/attribute node kind.".into());
            }
            seen[id as usize] = true;
            node.parent = parent;
            node.order = order;
            order += 1;
            pending.extend(
                node.children
                    .iter()
                    .rev()
                    .map(|&child| (child, Some(id), false)),
            );
            pending.extend(
                node.attributes
                    .iter()
                    .rev()
                    .map(|&attr| (attr, Some(id), true)),
            );
        }
        if seen.iter().any(|seen| !seen) {
            return Err("Unreachable nodes in CEM tree.".into());
        }
        if semantics.omitted.contains(&0) {
            return Err("The CEM document root cannot be omitted.".into());
        }
        // Inspection addresses source arena nodes, including nodes that the
        // semantic view will coalesce or omit. Keep their original ranges.
        let source_ranges = nodes.iter().map(|node| node.range).collect();
        let mut canonical: Vec<_> = (0..nodes.len())
            .map(|id| (!semantics.omitted.contains(&(id as u32))).then_some(id as u32))
            .collect();
        let mut omitted: Vec<_> = semantics.omitted.iter().copied().collect();
        while let Some(id) = omitted.pop() {
            let Some(node) = nodes.get(id as usize) else {
                return Err("Omitted CEM node does not exist.".into());
            };
            canonical[id as usize] = None;
            omitted.extend(node.children.iter().chain(&node.attributes).copied());
        }
        for parent in 0..nodes.len() {
            if canonical[parent].is_none()
                || !matches!(
                    nodes[parent].kind,
                    CemTreeNodeKind::Document | CemTreeNodeKind::Element
                )
            {
                continue;
            }
            let mut children: Vec<AstNodeId> = Vec::new();
            for child in std::mem::take(&mut nodes[parent].children) {
                if canonical[child as usize].is_none() {
                    continue;
                }
                if let Some(&previous) = children.last().filter(|&&id| {
                    nodes[id as usize].kind == CemTreeNodeKind::Text
                        && nodes[child as usize].kind == CemTreeNodeKind::Text
                }) {
                    let value = nodes[child as usize].value.clone();
                    let source = nodes[child as usize].source.clone();
                    let end = nodes[child as usize]
                        .range
                        .offset
                        .saturating_add(nodes[child as usize].range.length);
                    let prior = &mut nodes[previous as usize];
                    prior.value.push_str(&value);
                    merge_source(&mut prior.source, &source);
                    prior.range.length = end
                        .saturating_sub(prior.range.offset)
                        .max(prior.range.length);
                    canonical[child as usize] = Some(previous);
                } else {
                    children.push(child);
                }
            }
            children.retain(|&id| {
                nodes[id as usize].kind != CemTreeNodeKind::Text
                    || !nodes[id as usize].value.is_empty()
            });
            nodes[parent].children = children;
            nodes[parent]
                .attributes
                .retain(|&id| canonical[id as usize].is_some());
        }
        for canonical_id in &mut canonical {
            if canonical_id.is_some_and(|id| {
                nodes[id as usize].kind == CemTreeNodeKind::Text
                    && nodes[id as usize].value.is_empty()
            }) {
                *canonical_id = None;
            }
        }
        let source_uri = source_uri.into();
        let metadata = semantics
            .document_metadata
            .unwrap_or_else(|| CemDocumentMetadata {
                base_uri: Some(source_uri.clone()),
                document_uri: Some(source_uri.clone()),
            });
        Ok(Arc::new(Self {
            ast,
            source_uri,
            metadata,
            native_owner,
            nodes,
            canonical,
            source_fingerprint: semantics.source_fingerprint,
            source_lines_known,
            source_ranges,
            source_values: semantics.values,
        }))
    }

    pub fn ast(&self) -> &CemDocument {
        &self.ast
    }
    /// Parent in the original source arena, including nodes omitted or
    /// coalesced by the semantic view. Does not canonicalize the source ID.
    pub fn source_parent(&self, id: AstNodeId) -> Option<AstNodeId> {
        self.nodes.get(id as usize)?.parent
    }
    /// Decoded string value of an original source node, preserving individual
    /// text/CDATA boundaries and excluding descendant comments and attributes.
    pub fn source_text_fragments(&self, id: AstNodeId) -> Option<CemTreeTextFragments<'_>> {
        self.ast.get(id)?;
        Some(CemTreeTextFragments {
            tree: self,
            root: Some(id),
            pending: Vec::new(),
            source: true,
        })
    }
    /// String value of the normalized semantic node, without materializing its
    /// descendants or concatenating their text ahead of the caller's limits.
    pub fn text_fragments(&self, id: AstNodeId) -> Option<CemTreeTextFragments<'_>> {
        Some(CemTreeTextFragments {
            tree: self,
            root: Some(self.canonical_id(id)?),
            pending: Vec::new(),
            source: false,
        })
    }
    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }
    /// Original range without XPath canonicalization or text coalescing.
    /// Unknown line/column coordinates are zero; byte coordinates remain intact.
    pub fn source_node_range(&self, id: AstNodeId) -> Option<CemTreeRange> {
        let mut range = *self.source_ranges.get(id as usize)?;
        if !self.source_lines_known[id as usize] {
            range.line = 0;
            range.column = 0;
        }
        Some(range)
    }
    /// Opaque source-selection token; independent of per-document XDM identity.
    pub fn source_key(&self, id: AstNodeId) -> Option<String> {
        let id = self.canonical_id(id)?;
        let hash = blake3::Hash::from_bytes(self.source_fingerprint?);
        Some(format!("cem-source:1:{}:{id}", hash.to_hex()))
    }
    pub fn source_line_number(&self, id: AstNodeId) -> Option<u32> {
        let id = self.canonical_id(id)?;
        if !self.source_lines_known[id as usize] {
            return None;
        }
        let node = self.node(id)?;
        (node.source.origin().is_some() && node.range.line > 0).then_some(node.range.line)
    }
    pub fn base_uri(&self) -> Option<&str> {
        self.metadata.base_uri.as_deref()
    }
    pub fn document_uri(&self) -> Option<&str> {
        self.metadata.document_uri.as_deref()
    }
    pub fn node_base_uri(&self, mut id: AstNodeId) -> Option<&str> {
        loop {
            let node = self.node(id)?;
            if let Some(uri) = node.base_uri.as_deref() {
                return Some(uri);
            }
            match node.parent {
                Some(parent) => id = parent,
                None => return self.base_uri(),
            }
        }
    }
    pub fn native_owner(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.native_owner.as_ref()
    }
    pub fn canonical_id(&self, id: AstNodeId) -> Option<AstNodeId> {
        self.canonical.get(id as usize).copied().flatten()
    }
    pub fn node(&self, id: AstNodeId) -> Option<&CemTreeNode> {
        self.nodes.get(self.canonical_id(id)? as usize)
    }
}

fn source_range(
    source: &SourceMapStack,
    lines: &crate::source::line_index::LineIndex,
) -> CemTreeRange {
    let Some(frame) = source.origin() else {
        return CemTreeRange {
            line: 1,
            column: 1,
            ..Default::default()
        };
    };
    let spans = match &frame.span {
        FrameSpan::Single(span) => vec![span],
        FrameSpan::Multi(spans) => spans.iter().collect(),
    };
    let offset = spans.iter().map(|s| s.start).min().unwrap_or(0);
    let end = spans
        .iter()
        .map(|s| s.start.saturating_add(s.len as u64))
        .max()
        .unwrap_or(offset);
    let location = lines.project(offset);
    CemTreeRange {
        line: location.line,
        column: location.column,
        offset,
        length: end.saturating_sub(offset),
    }
}

pub(crate) fn merge_source(target: &mut SourceMapStack, source: &SourceMapStack) {
    if target.frames.len() == 1
        && source.frames.len() == 1
        && target.frames[0].source_id == source.frames[0].source_id
        && target.frames[0].transform == source.frames[0].transform
    {
        let spans = |span: &FrameSpan| match span {
            FrameSpan::Single(s) => vec![*s],
            FrameSpan::Multi(s) => s.clone(),
        };
        let mut merged = spans(&target.frames[0].span);
        merged.extend(spans(&source.frames[0].span));
        target.frames[0].span = FrameSpan::Multi(merged);
    } else {
        target.frames.extend(source.frames.clone());
    }
}
