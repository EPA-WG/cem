//! Deterministic uncompressed encoder for `CemDocument`.
//!
//! Implementation rules:
//!
//! - Strings are interned by *first-seen* order; the deterministic walk
//!   over the node list guarantees stable indices.
//! - All multi-byte integers are little-endian.
//! - Section order is fixed: header → dictionaries → nodes → edges →
//!   id_table → unresolved_slots → chunk metadata → integrity hash.
//! - The integrity hash is a 64-bit FNV-1a computed over every byte
//!   preceding the hash itself.

use crate::ast::format::{
    fnv1a64, BinaryAstPayload, ChunkMetadata, NodeKindTag, FLAGS_NONE, MAGIC, VERSION,
};
use crate::parser::document::CemDocument;
use crate::parser::CemAstNode;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct DebugBinaryEncoder;

impl DebugBinaryEncoder {
    pub fn new() -> Self {
        Self
    }

    pub fn encode(&self, doc: &CemDocument) -> BinaryAstPayload {
        let mut dicts = Dictionaries::default();
        // Pre-walk to populate dictionaries deterministically.
        for node in &doc.nodes {
            seed_node(&mut dicts, node);
        }
        for target in doc.id_table.keys() {
            dicts.intern_string(target);
        }
        for slot in &doc.unresolved_slots {
            dicts.intern_string(&slot.target_name);
            seed_source_map(&mut dicts, &slot.source);
        }

        let mut out = Vec::<u8>::new();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&FLAGS_NONE.to_le_bytes());

        dicts.write(&mut out);
        write_nodes(&mut out, &doc.nodes, &dicts);
        write_edges(&mut out, &doc.nodes);
        write_id_table(&mut out, doc, &dicts);
        write_unresolved_slots(&mut out, doc, &dicts);

        let local_node_count = doc.nodes.len() as u32;
        // Write chunk metadata with a placeholder hash; the trailing hash
        // covers everything before it, so decode and encode agree.
        let placeholder_chunk = ChunkMetadata {
            root_id: 0,
            parent_anchor: None,
            dictionary_ids: vec![0],
            local_node_start: 0,
            local_node_count,
            source_map_deltas: Vec::new(),
            child_links: Vec::new(),
            external_references: Vec::new(),
            integrity_hash: 0,
        };
        write_chunk_metadata(&mut out, &placeholder_chunk);

        let integrity_hash = fnv1a64(&out);
        out.extend_from_slice(&integrity_hash.to_le_bytes());

        let chunk = ChunkMetadata {
            integrity_hash,
            ..placeholder_chunk
        };
        BinaryAstPayload {
            bytes: out,
            chunks: vec![chunk],
        }
    }
}

#[derive(Default)]
pub(super) struct Dictionaries {
    pub(super) strings: Vec<String>,
    string_index: BTreeMap<String, u32>,
    pub(super) source_ids: Vec<SourceId>,
    source_id_index: BTreeMap<u32, u32>,
    pub(super) transforms: Vec<EncodedTransform>,
    transform_index: BTreeMap<String, u32>,
    pub(super) source_map_frames: Vec<EncodedSourceMapFrame>,
    source_map_index: BTreeMap<String, u32>,
}

#[derive(Debug, Clone)]
pub(super) struct EncodedTransform {
    /// Discriminant tag (`TransformKind` variant index).
    pub tag: u16,
    /// Optional string payload (interned id), e.g. schema id text or
    /// content-type label. `u32::MAX` means absent.
    pub string_payload: u32,
}

#[derive(Debug, Clone)]
pub(super) struct EncodedSourceMapFrame {
    pub source_id_dict: u32,
    pub span_kind: u8,
    pub ranges: Vec<ByteRange>,
    pub transform_dict: u32,
}

impl Dictionaries {
    pub(super) fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(i) = self.string_index.get(s) {
            return *i;
        }
        let i = self.strings.len() as u32;
        self.strings.push(s.to_owned());
        self.string_index.insert(s.to_owned(), i);
        i
    }

    pub(super) fn intern_source_id(&mut self, sid: SourceId) -> u32 {
        if let Some(i) = self.source_id_index.get(&sid.0) {
            return *i;
        }
        let i = self.source_ids.len() as u32;
        self.source_ids.push(sid);
        self.source_id_index.insert(sid.0, i);
        i
    }

    pub(super) fn intern_transform(&mut self, t: &TransformKind) -> u32 {
        let (tag, payload) = encode_transform(t);
        let payload_id = match payload {
            Some(s) => self.intern_string(&s),
            None => u32::MAX,
        };
        let key = format!("{tag}:{payload_id}");
        if let Some(i) = self.transform_index.get(&key) {
            return *i;
        }
        let i = self.transforms.len() as u32;
        self.transforms.push(EncodedTransform {
            tag,
            string_payload: payload_id,
        });
        self.transform_index.insert(key, i);
        i
    }

    pub(super) fn intern_source_map_frame(&mut self, frame: &SourceMapFrame) -> u32 {
        let source_id_dict = self.intern_source_id(frame.source_id);
        let transform_dict = self.intern_transform(&frame.transform);
        let (span_kind, ranges) = match &frame.span {
            FrameSpan::Single(r) => (0u8, vec![*r]),
            FrameSpan::Multi(rs) => (1u8, rs.clone()),
        };
        let key = source_map_frame_key(frame);
        if let Some(i) = self.source_map_index.get(&key) {
            return *i;
        }
        let i = self.source_map_frames.len() as u32;
        self.source_map_frames.push(EncodedSourceMapFrame {
            source_id_dict,
            span_kind,
            ranges,
            transform_dict,
        });
        self.source_map_index.insert(key, i);
        i
    }

    pub(super) fn write(&self, out: &mut Vec<u8>) {
        // Strings.
        out.extend_from_slice(&(self.strings.len() as u32).to_le_bytes());
        for s in &self.strings {
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        // Source ids.
        out.extend_from_slice(&(self.source_ids.len() as u32).to_le_bytes());
        for sid in &self.source_ids {
            out.extend_from_slice(&sid.0.to_le_bytes());
        }
        // Transforms.
        out.extend_from_slice(&(self.transforms.len() as u32).to_le_bytes());
        for t in &self.transforms {
            out.extend_from_slice(&t.tag.to_le_bytes());
            out.extend_from_slice(&t.string_payload.to_le_bytes());
        }
        // Source-map frames.
        out.extend_from_slice(&(self.source_map_frames.len() as u32).to_le_bytes());
        for f in &self.source_map_frames {
            out.extend_from_slice(&f.source_id_dict.to_le_bytes());
            out.push(f.span_kind);
            out.extend_from_slice(&(f.ranges.len() as u32).to_le_bytes());
            for r in &f.ranges {
                out.extend_from_slice(&r.start.to_le_bytes());
                out.extend_from_slice(&r.len.to_le_bytes());
            }
            out.extend_from_slice(&f.transform_dict.to_le_bytes());
        }
    }
}

fn seed_node(d: &mut Dictionaries, node: &CemAstNode) {
    match node {
        CemAstNode::Document { source, .. } => seed_source_map(d, source),
        CemAstNode::Element {
            expanded_name,
            source,
            ..
        } => {
            d.intern_string(&expanded_name.namespace_uri);
            d.intern_string(&expanded_name.local_name);
            seed_source_map(d, source);
        }
        CemAstNode::Attribute {
            expanded_name,
            value,
            source,
            ..
        } => {
            d.intern_string(&expanded_name.namespace_uri);
            d.intern_string(&expanded_name.local_name);
            if let Some(v) = value {
                d.intern_string(v);
            }
            seed_source_map(d, source);
        }
        CemAstNode::Text { data, source, .. }
        | CemAstNode::Whitespace { data, source, .. }
        | CemAstNode::Comment { data, source, .. }
        | CemAstNode::Cdata { data, source, .. }
        | CemAstNode::RawText { data, source, .. } => {
            d.intern_string(data);
            seed_source_map(d, source);
        }
        CemAstNode::ProcessingInstruction {
            target,
            data,
            source,
            ..
        } => {
            d.intern_string(target);
            d.intern_string(data);
            seed_source_map(d, source);
        }
        CemAstNode::Error { code, source, .. } => {
            d.intern_string(code);
            seed_source_map(d, source);
        }
    }
}

fn seed_source_map(d: &mut Dictionaries, stack: &SourceMapStack) {
    for frame in &stack.frames {
        d.intern_source_map_frame(frame);
    }
}

fn encode_transform(t: &TransformKind) -> (u16, Option<String>) {
    match t {
        TransformKind::HtmlTokenizer => (0, None),
        TransformKind::XmlTokenizer => (1, None),
        TransformKind::CemTokenizer => (2, None),
        TransformKind::EventNormalizer => (3, None),
        TransformKind::SchemaValidation { schema_id } => (4, Some(schema_id.to_string())),
        TransformKind::CemAstBuilder => (5, None),
        TransformKind::HandoffBoundary { child_content_type } => {
            (6, Some(child_content_type.clone()))
        }
        TransformKind::ContentTypeTransform { content_type } => (7, Some(content_type.clone())),
        TransformKind::InterpreterRender => (8, None),
        TransformKind::Query => (9, None),
        TransformKind::QueryStep => (10, None),
    }
}

fn write_nodes(out: &mut Vec<u8>, nodes: &[CemAstNode], d: &Dictionaries) {
    out.extend_from_slice(&(nodes.len() as u32).to_le_bytes());
    for node in nodes {
        match node {
            CemAstNode::Document {
                node_id,
                root_children,
                source,
            } => {
                out.push(NodeKindTag::Document as u8);
                out.extend_from_slice(&node_id.to_le_bytes());
                write_source_map(out, d, source);
                let _ = root_children; // edges section carries children
            }
            CemAstNode::Element {
                node_id,
                expanded_name,
                has_explicit_boundary,
                source,
                ..
            } => {
                out.push(NodeKindTag::Element as u8);
                out.extend_from_slice(&node_id.to_le_bytes());
                write_string_ref(out, d, &expanded_name.namespace_uri);
                write_string_ref(out, d, &expanded_name.local_name);
                out.extend_from_slice(&expanded_name.schema_id.unwrap_or(u32::MAX).to_le_bytes());
                out.push(if *has_explicit_boundary { 1 } else { 0 });
                write_source_map(out, d, source);
            }
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                value,
                source,
            } => {
                out.push(NodeKindTag::Attribute as u8);
                out.extend_from_slice(&node_id.to_le_bytes());
                write_string_ref(out, d, &expanded_name.namespace_uri);
                write_string_ref(out, d, &expanded_name.local_name);
                out.extend_from_slice(&expanded_name.schema_id.unwrap_or(u32::MAX).to_le_bytes());
                match value {
                    Some(v) => {
                        out.push(1);
                        write_string_ref(out, d, v);
                    }
                    None => out.push(0),
                }
                write_source_map(out, d, source);
            }
            CemAstNode::Text {
                node_id,
                data,
                source,
            } => write_data_node(out, d, NodeKindTag::Text, *node_id, data, source),
            CemAstNode::Whitespace {
                node_id,
                data,
                source,
            } => write_data_node(out, d, NodeKindTag::Whitespace, *node_id, data, source),
            CemAstNode::Comment {
                node_id,
                data,
                source,
            } => write_data_node(out, d, NodeKindTag::Comment, *node_id, data, source),
            CemAstNode::Cdata {
                node_id,
                data,
                source,
            } => write_data_node(out, d, NodeKindTag::Cdata, *node_id, data, source),
            CemAstNode::RawText {
                node_id,
                data,
                source,
            } => write_data_node(out, d, NodeKindTag::RawText, *node_id, data, source),
            CemAstNode::ProcessingInstruction {
                node_id,
                target,
                data,
                source,
            } => {
                out.push(NodeKindTag::ProcessingInstruction as u8);
                out.extend_from_slice(&node_id.to_le_bytes());
                write_string_ref(out, d, target);
                write_string_ref(out, d, data);
                write_source_map(out, d, source);
            }
            CemAstNode::Error {
                node_id,
                code,
                source,
            } => {
                out.push(NodeKindTag::Error as u8);
                out.extend_from_slice(&node_id.to_le_bytes());
                write_string_ref(out, d, code);
                write_source_map(out, d, source);
            }
        }
    }
}

fn write_data_node(
    out: &mut Vec<u8>,
    d: &Dictionaries,
    tag: NodeKindTag,
    node_id: u32,
    data: &str,
    source: &SourceMapStack,
) {
    out.push(tag as u8);
    out.extend_from_slice(&node_id.to_le_bytes());
    write_string_ref(out, d, data);
    write_source_map(out, d, source);
}

fn write_string_ref(out: &mut Vec<u8>, d: &Dictionaries, s: &str) {
    let idx = d
        .string_index
        .get(s)
        .copied()
        .expect("seeded dictionary must contain every emitted string");
    out.extend_from_slice(&idx.to_le_bytes());
}

fn write_source_map(out: &mut Vec<u8>, d: &Dictionaries, stack: &SourceMapStack) {
    out.extend_from_slice(&(stack.frames.len() as u32).to_le_bytes());
    for frame in &stack.frames {
        let key = source_map_frame_key(frame);
        let idx = d
            .source_map_index
            .get(&key)
            .copied()
            .expect("seeded dictionary must contain every source-map frame");
        out.extend_from_slice(&idx.to_le_bytes());
    }
}

fn source_map_frame_key(frame: &SourceMapFrame) -> String {
    let (span_kind, ranges) = match &frame.span {
        FrameSpan::Single(r) => (0u8, vec![*r]),
        FrameSpan::Multi(rs) => (1u8, rs.clone()),
    };
    let (tag, payload) = encode_transform(&frame.transform);
    let payload_s = payload.unwrap_or_default();
    format!(
        "sid={} tag={} payload={} span={} ranges={}",
        frame.source_id.0,
        tag,
        payload_s,
        span_kind,
        ranges
            .iter()
            .map(|r| format!("{}+{}", r.start, r.len))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn write_edges(out: &mut Vec<u8>, nodes: &[CemAstNode]) {
    // Count nodes that produce edges.
    let edge_emitters: Vec<(u32, Vec<u32>, Vec<u32>)> = nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Document {
                node_id,
                root_children,
                ..
            } => Some((*node_id, root_children.clone(), Vec::new())),
            CemAstNode::Element {
                node_id,
                attributes,
                children,
                ..
            } => Some((*node_id, children.clone(), attributes.clone())),
            _ => None,
        })
        .collect();
    out.extend_from_slice(&(edge_emitters.len() as u32).to_le_bytes());
    for (parent, children, attrs) in edge_emitters {
        out.extend_from_slice(&parent.to_le_bytes());
        out.extend_from_slice(&(attrs.len() as u32).to_le_bytes());
        for a in attrs {
            out.extend_from_slice(&a.to_le_bytes());
        }
        out.extend_from_slice(&(children.len() as u32).to_le_bytes());
        for c in children {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
}

fn write_id_table(out: &mut Vec<u8>, doc: &CemDocument, d: &Dictionaries) {
    // Sort by string-dict index for deterministic order.
    let mut entries: Vec<(&String, u32)> = doc.id_table.iter().map(|(k, v)| (k, *v)).collect();
    entries.sort_by_key(|(k, _)| d.string_index.get(*k).copied().unwrap_or(u32::MAX));
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (name, target) in entries {
        write_string_ref(out, d, name);
        out.extend_from_slice(&target.to_le_bytes());
    }
}

fn write_unresolved_slots(out: &mut Vec<u8>, doc: &CemDocument, d: &Dictionaries) {
    out.extend_from_slice(&(doc.unresolved_slots.len() as u32).to_le_bytes());
    for slot in &doc.unresolved_slots {
        out.extend_from_slice(&slot.owner_scope.to_le_bytes());
        write_string_ref(out, d, &slot.target_name);
        write_source_map(out, d, &slot.source);
    }
}

fn write_chunk_metadata(out: &mut Vec<u8>, chunk: &ChunkMetadata) {
    out.extend_from_slice(&chunk.root_id.to_le_bytes());
    match chunk.parent_anchor {
        Some(anchor) => {
            out.push(1);
            out.extend_from_slice(&anchor.to_le_bytes());
        }
        None => out.push(0),
    }
    out.extend_from_slice(&(chunk.dictionary_ids.len() as u32).to_le_bytes());
    for id in &chunk.dictionary_ids {
        out.extend_from_slice(&id.to_le_bytes());
    }
    out.extend_from_slice(&chunk.local_node_start.to_le_bytes());
    out.extend_from_slice(&chunk.local_node_count.to_le_bytes());
    out.extend_from_slice(&(chunk.source_map_deltas.len() as u32).to_le_bytes());
    // Tier A: source_map_deltas is always empty (whole-document chunk).
    debug_assert!(chunk.source_map_deltas.is_empty());
    out.extend_from_slice(&(chunk.child_links.len() as u32).to_le_bytes());
    debug_assert!(chunk.child_links.is_empty());
    out.extend_from_slice(&(chunk.external_references.len() as u32).to_le_bytes());
    debug_assert!(chunk.external_references.is_empty());
    out.extend_from_slice(&chunk.integrity_hash.to_le_bytes());
}

impl super::BinaryAstEncoder for DebugBinaryEncoder {
    fn encode(&self, nodes: &[CemAstNode]) -> BinaryAstPayload {
        // Build a temporary document so the encoder body is shared.
        let doc = CemDocument {
            nodes: nodes.to_vec(),
            ..CemDocument::default()
        };
        self.encode(&doc)
    }
}
