//! XML lexical interpretation ends here; consumers only see generic source maps.
use crate::{
    parser::{document::CemDocument, AstNodeId, CemAstNode},
    source::{ByteRange, SourceId},
    source_map::{FrameSpan, SourceMapFrame, TransformKind},
    validation::xml::{self, XmlEventAst, XmlEventKind},
};
use std::collections::BTreeMap;

pub(super) type ExpressionPayloads = BTreeMap<AstNodeId, (u64, Vec<SourceMapFrame>)>;

pub(super) fn record(
    payloads: &mut ExpressionPayloads,
    parent: AstNodeId,
    event: &XmlEventAst,
    document: &CemDocument,
) {
    if !matches!(document.get(parent), Some(CemAstNode::Element { expanded_name, .. }) if expanded_name.local_name == "expr" && expanded_name.namespace_uri == "https://cem.dev/ns/cem-ml/1")
    {
        return;
    }
    let start = event.source_range.start.byte_offset;
    let source_id = event
        .source_range
        .source_map()
        .origin()
        .map(|frame| frame.source_id)
        .unwrap_or(SourceId(1));
    let (offset, frames) = payloads.entry(parent).or_default();
    let mut segment = |original: ByteRange, length: u32| {
        frames.push(SourceMapFrame {
            source_id,
            span: FrameSpan::Single(original),
            transform: TransformKind::ExpressionEmbedding {
                expression: ByteRange::new(*offset, length),
            },
        });
        *offset += u64::from(length);
    };
    if event.kind == XmlEventKind::EntityReference {
        if let Some(value) = xml::xml_decode_entity_reference(event.value.as_deref().unwrap_or(""))
        {
            segment(
                ByteRange::new(start, event.source_range.byte_length as u32),
                value.len_utf8() as u32,
            );
        }
        return;
    }
    if !matches!(event.kind, XmlEventKind::Text | XmlEventKind::Cdata) {
        return;
    }
    let text = event.value.as_deref().unwrap_or(&event.lexeme);
    let start = start
        + if event.kind == XmlEventKind::Cdata {
            9
        } else {
            0
        };
    let mut cursor = 0;
    while cursor < text.len() {
        if let Some(relative) = text[cursor..].find('\r') {
            let boundary = cursor + relative;
            if boundary > cursor {
                segment(
                    ByteRange::new(start + cursor as u64, (boundary - cursor) as u32),
                    (boundary - cursor) as u32,
                );
            }
            let width = if text.as_bytes().get(boundary + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            segment(ByteRange::new(start + boundary as u64, width as u32), 1);
            cursor = boundary + width;
        } else {
            let length = (text.len() - cursor) as u32;
            segment(ByteRange::new(start + cursor as u64, length), length);
            break;
        }
    }
}
