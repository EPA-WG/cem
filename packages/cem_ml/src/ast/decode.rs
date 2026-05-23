//! Decoder mirror of [`crate::ast::encode::DebugBinaryEncoder`]. Reads a
//! byte payload back into a `CemDocument` for round-trip testing.

use crate::ast::format::{fnv1a64, NodeKindTag, FLAGS_NONE, MAGIC, VERSION};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode, ExpandedName, NameSlot};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use std::collections::HashMap;

#[derive(Debug)]
pub enum DecodeError {
    BadMagic,
    BadVersion(u16),
    UnexpectedEof,
    UnknownKindTag(u8),
    UnknownTransformTag(u16),
    IntegrityMismatch { expected: u64, actual: u64 },
    InvalidUtf8,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::BadMagic => f.write_str("invalid magic"),
            DecodeError::BadVersion(v) => write!(f, "unsupported version: {v}"),
            DecodeError::UnexpectedEof => f.write_str("unexpected end of payload"),
            DecodeError::UnknownKindTag(t) => write!(f, "unknown node kind tag: {t}"),
            DecodeError::UnknownTransformTag(t) => write!(f, "unknown transform tag: {t}"),
            DecodeError::IntegrityMismatch { expected, actual } => write!(
                f,
                "integrity hash mismatch: expected {expected:016x}, got {actual:016x}"
            ),
            DecodeError::InvalidUtf8 => f.write_str("invalid UTF-8 in string dictionary"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[derive(Default)]
pub struct DebugBinaryDecoder;

impl DebugBinaryDecoder {
    pub fn new() -> Self {
        Self
    }

    pub fn decode(&self, bytes: &[u8]) -> Result<CemDocument, DecodeError> {
        // Verify integrity hash first.
        if bytes.len() < 8 {
            return Err(DecodeError::UnexpectedEof);
        }
        let hash_offset = bytes.len() - 8;
        let stored_hash = u64::from_le_bytes(bytes[hash_offset..].try_into().unwrap());
        let actual_hash = fnv1a64(&bytes[..hash_offset]);
        if stored_hash != actual_hash {
            return Err(DecodeError::IntegrityMismatch {
                expected: stored_hash,
                actual: actual_hash,
            });
        }

        let mut r = Reader::new(&bytes[..hash_offset]);
        let mut magic = [0u8; 4];
        r.read_into(&mut magic)?;
        if magic != MAGIC {
            return Err(DecodeError::BadMagic);
        }
        let version = r.read_u16()?;
        if version != VERSION {
            return Err(DecodeError::BadVersion(version));
        }
        let flags = r.read_u16()?;
        // forward-compat: unknown flags ignored in Tier A.
        let _ = (FLAGS_NONE, flags);

        let strings = read_strings(&mut r)?;
        let source_ids = read_source_ids(&mut r)?;
        let transforms = read_transforms(&mut r)?;
        let source_map_frames = read_source_map_frames(&mut r, &source_ids, &transforms, &strings)?;

        let nodes = read_nodes(&mut r, &strings, &source_map_frames)?;
        let (attr_map, child_map) = read_edges(&mut r)?;
        let nodes = link_edges(nodes, attr_map, child_map);

        let id_table = read_id_table(&mut r, &strings)?;
        let unresolved_slots = read_unresolved_slots(&mut r, &strings, &source_map_frames)?;

        // Chunk metadata is parsed for completeness even though Tier A
        // round-trip tests don't need it; surfacing parse errors here
        // is the validation we want.
        read_chunk_metadata(&mut r)?;

        Ok(CemDocument {
            nodes,
            id_table,
            unresolved_slots,
            diagnostics: Vec::new(),
            // Binary AST round-trip is Tier A scope-only; the
            // document-format identity is recorded at parse time
            // (AC-F-8) and not yet serialized through the binary form.
            // A Tier B follow-up can extend AC-CC-* / AC-F-8 to carry
            // it on the binary header.
            format_identity: None,
        })
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn ensure(&self, need: usize) -> Result<(), DecodeError> {
        if self.cursor + need > self.bytes.len() {
            Err(DecodeError::UnexpectedEof)
        } else {
            Ok(())
        }
    }

    fn read_into(&mut self, buf: &mut [u8]) -> Result<(), DecodeError> {
        self.ensure(buf.len())?;
        buf.copy_from_slice(&self.bytes[self.cursor..self.cursor + buf.len()]);
        self.cursor += buf.len();
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        self.ensure(1)?;
        let v = self.bytes[self.cursor];
        self.cursor += 1;
        Ok(v)
    }

    fn read_u16(&mut self) -> Result<u16, DecodeError> {
        let mut b = [0u8; 2];
        self.read_into(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    fn read_u32(&mut self) -> Result<u32, DecodeError> {
        let mut b = [0u8; 4];
        self.read_into(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn read_u64(&mut self) -> Result<u64, DecodeError> {
        let mut b = [0u8; 8];
        self.read_into(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, DecodeError> {
        self.ensure(n)?;
        let v = self.bytes[self.cursor..self.cursor + n].to_vec();
        self.cursor += n;
        Ok(v)
    }
}

fn read_strings(r: &mut Reader<'_>) -> Result<Vec<String>, DecodeError> {
    let count = r.read_u32()?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let len = r.read_u32()?;
        let bytes = r.read_bytes(len as usize)?;
        out.push(String::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8)?);
    }
    Ok(out)
}

fn read_source_ids(r: &mut Reader<'_>) -> Result<Vec<SourceId>, DecodeError> {
    let count = r.read_u32()?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        out.push(SourceId(r.read_u32()?));
    }
    Ok(out)
}

fn read_transforms(r: &mut Reader<'_>) -> Result<Vec<(u16, u32)>, DecodeError> {
    let count = r.read_u32()?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        out.push((r.read_u16()?, r.read_u32()?));
    }
    Ok(out)
}

fn read_source_map_frames(
    r: &mut Reader<'_>,
    source_ids: &[SourceId],
    transforms: &[(u16, u32)],
    strings: &[String],
) -> Result<Vec<SourceMapFrame>, DecodeError> {
    let count = r.read_u32()?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let source_id_dict = r.read_u32()?;
        let span_kind = r.read_u8()?;
        let range_count = r.read_u32()?;
        let mut ranges = Vec::with_capacity(range_count as usize);
        for _ in 0..range_count {
            let start = r.read_u64()?;
            let len = r.read_u32()?;
            ranges.push(ByteRange::new(start, len));
        }
        let transform_dict = r.read_u32()?;
        let (tag, payload_dict) = transforms[transform_dict as usize];
        let payload = if payload_dict == u32::MAX {
            None
        } else {
            Some(strings[payload_dict as usize].clone())
        };
        let transform = decode_transform(tag, payload)?;
        let span = match span_kind {
            0 => FrameSpan::Single(ranges[0]),
            _ => FrameSpan::Multi(ranges),
        };
        out.push(SourceMapFrame {
            source_id: source_ids[source_id_dict as usize],
            span,
            transform,
        });
    }
    Ok(out)
}

fn decode_transform(tag: u16, payload: Option<String>) -> Result<TransformKind, DecodeError> {
    Ok(match tag {
        0 => TransformKind::HtmlTokenizer,
        1 => TransformKind::XmlTokenizer,
        2 => TransformKind::CemTokenizer,
        3 => TransformKind::EventNormalizer,
        4 => TransformKind::SchemaValidation {
            schema_id: payload.and_then(|p| p.parse().ok()).unwrap_or(0),
        },
        5 => TransformKind::CemAstBuilder,
        6 => TransformKind::HandoffBoundary {
            child_content_type: payload.unwrap_or_default(),
        },
        7 => TransformKind::ContentTypeTransform {
            content_type: payload.unwrap_or_default(),
        },
        8 => TransformKind::InterpreterRender,
        9 => TransformKind::Query,
        10 => TransformKind::QueryStep,
        _ => return Err(DecodeError::UnknownTransformTag(tag)),
    })
}

fn read_nodes(
    r: &mut Reader<'_>,
    strings: &[String],
    frames: &[SourceMapFrame],
) -> Result<Vec<CemAstNode>, DecodeError> {
    let count = r.read_u32()?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let kind = NodeKindTag::from_u8(r.read_u8()?).ok_or(DecodeError::UnknownKindTag(0))?;
        let node_id = r.read_u32()?;
        let node = match kind {
            NodeKindTag::Document => CemAstNode::Document {
                node_id,
                root_children: Vec::new(),
                source: read_source_map(r, frames)?,
            },
            NodeKindTag::Element => {
                let namespace_uri = read_string(r, strings)?;
                let local_name = read_string(r, strings)?;
                let schema_id_raw = r.read_u32()?;
                let schema_id = if schema_id_raw == u32::MAX {
                    None
                } else {
                    Some(schema_id_raw)
                };
                let has_explicit_boundary = r.read_u8()? != 0;
                let source = read_source_map(r, frames)?;
                CemAstNode::Element {
                    node_id,
                    expanded_name: ExpandedName {
                        namespace_uri,
                        local_name,
                        schema_id,
                    },
                    attributes: Vec::new(),
                    children: Vec::new(),
                    has_explicit_boundary,
                    source,
                }
            }
            NodeKindTag::Attribute => {
                let namespace_uri = read_string(r, strings)?;
                let local_name = read_string(r, strings)?;
                let schema_id_raw = r.read_u32()?;
                let schema_id = if schema_id_raw == u32::MAX {
                    None
                } else {
                    Some(schema_id_raw)
                };
                let has_value = r.read_u8()? != 0;
                let value = if has_value {
                    Some(read_string(r, strings)?)
                } else {
                    None
                };
                let source = read_source_map(r, frames)?;
                CemAstNode::Attribute {
                    node_id,
                    expanded_name: ExpandedName {
                        namespace_uri,
                        local_name,
                        schema_id,
                    },
                    value,
                    source,
                }
            }
            NodeKindTag::Text => CemAstNode::Text {
                node_id,
                data: read_string(r, strings)?,
                source: read_source_map(r, frames)?,
            },
            NodeKindTag::Whitespace => CemAstNode::Whitespace {
                node_id,
                data: read_string(r, strings)?,
                source: read_source_map(r, frames)?,
            },
            NodeKindTag::Comment => CemAstNode::Comment {
                node_id,
                data: read_string(r, strings)?,
                source: read_source_map(r, frames)?,
            },
            NodeKindTag::Cdata => CemAstNode::Cdata {
                node_id,
                data: read_string(r, strings)?,
                source: read_source_map(r, frames)?,
            },
            NodeKindTag::RawText => CemAstNode::RawText {
                node_id,
                data: read_string(r, strings)?,
                source: read_source_map(r, frames)?,
            },
            NodeKindTag::ProcessingInstruction => {
                let target = read_string(r, strings)?;
                let data = read_string(r, strings)?;
                let source = read_source_map(r, frames)?;
                CemAstNode::ProcessingInstruction {
                    node_id,
                    target,
                    data,
                    source,
                }
            }
            NodeKindTag::Error => CemAstNode::Error {
                node_id,
                code: read_string(r, strings)?,
                source: read_source_map(r, frames)?,
            },
        };
        out.push(node);
    }
    Ok(out)
}

fn read_string(r: &mut Reader<'_>, strings: &[String]) -> Result<String, DecodeError> {
    let idx = r.read_u32()? as usize;
    Ok(strings[idx].clone())
}

fn read_source_map(
    r: &mut Reader<'_>,
    frames: &[SourceMapFrame],
) -> Result<SourceMapStack, DecodeError> {
    let count = r.read_u32()?;
    let mut stack = SourceMapStack::default();
    for _ in 0..count {
        let idx = r.read_u32()? as usize;
        stack.frames.push(frames[idx].clone());
    }
    Ok(stack)
}

type AttrMap = HashMap<AstNodeId, Vec<AstNodeId>>;
type ChildMap = HashMap<AstNodeId, Vec<AstNodeId>>;

fn read_edges(r: &mut Reader<'_>) -> Result<(AttrMap, ChildMap), DecodeError> {
    let count = r.read_u32()?;
    let mut attrs: AttrMap = HashMap::new();
    let mut children: ChildMap = HashMap::new();
    for _ in 0..count {
        let parent = r.read_u32()?;
        let attr_count = r.read_u32()?;
        let mut attr_ids = Vec::with_capacity(attr_count as usize);
        for _ in 0..attr_count {
            attr_ids.push(r.read_u32()?);
        }
        let child_count = r.read_u32()?;
        let mut child_ids = Vec::with_capacity(child_count as usize);
        for _ in 0..child_count {
            child_ids.push(r.read_u32()?);
        }
        attrs.insert(parent, attr_ids);
        children.insert(parent, child_ids);
    }
    Ok((attrs, children))
}

fn link_edges(
    mut nodes: Vec<CemAstNode>,
    attr_map: AttrMap,
    child_map: ChildMap,
) -> Vec<CemAstNode> {
    for node in nodes.iter_mut() {
        match node {
            CemAstNode::Document {
                node_id,
                root_children,
                ..
            } => {
                if let Some(c) = child_map.get(node_id) {
                    *root_children = c.clone();
                }
            }
            CemAstNode::Element {
                node_id,
                attributes,
                children,
                ..
            } => {
                if let Some(a) = attr_map.get(node_id) {
                    *attributes = a.clone();
                }
                if let Some(c) = child_map.get(node_id) {
                    *children = c.clone();
                }
            }
            _ => {}
        }
    }
    nodes
}

fn read_id_table(
    r: &mut Reader<'_>,
    strings: &[String],
) -> Result<HashMap<String, AstNodeId>, DecodeError> {
    let count = r.read_u32()?;
    let mut out = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let name = read_string(r, strings)?;
        let target = r.read_u32()?;
        out.insert(name, target);
    }
    Ok(out)
}

fn read_unresolved_slots(
    r: &mut Reader<'_>,
    strings: &[String],
    frames: &[SourceMapFrame],
) -> Result<Vec<NameSlot>, DecodeError> {
    let count = r.read_u32()?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let owner_scope = r.read_u32()?;
        let target_name = read_string(r, strings)?;
        let source = read_source_map(r, frames)?;
        out.push(NameSlot {
            owner_scope,
            target_name,
            resolved: None,
            source,
        });
    }
    Ok(out)
}

fn read_chunk_metadata(r: &mut Reader<'_>) -> Result<(), DecodeError> {
    let _root_id = r.read_u32()?;
    let has_parent = r.read_u8()?;
    if has_parent != 0 {
        let _ = r.read_u32()?;
    }
    let dict_count = r.read_u32()?;
    for _ in 0..dict_count {
        let _ = r.read_u32()?;
    }
    let _local_start = r.read_u32()?;
    let _local_count = r.read_u32()?;
    let smd = r.read_u32()?;
    debug_assert_eq!(smd, 0);
    let cl = r.read_u32()?;
    debug_assert_eq!(cl, 0);
    let er = r.read_u32()?;
    debug_assert_eq!(er, 0);
    let _hash = r.read_u64()?;
    Ok(())
}
