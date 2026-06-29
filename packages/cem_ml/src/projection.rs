//! Projection artifacts and debug/interchange views for AST / events / DOM.
//!
//! The `*_binary_artifact` functions produce hash-addressed CEM binary chunks
//! for runtime/cache handoff. The JSON projection functions remain
//! consumer-friendly debug/interchange views for `cem-ml parse --format
//! dom-json|ast|events` and the same projections for `convert` / `inspect`.

use crate::engine::InputFormat;
use crate::events::{
    cem::CemEventNormalizer, EventNormalizer, NormalizedEvent, ScalarValue, SeparatorKind,
    Synthesis, TriviaKind,
};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode, ExpandedName};
use crate::schema::registry::{
    CEM_AST_PROJECTION_CONTENT_TYPE, CEM_AST_PROJECTION_SCHEMA_URI,
    CEM_DOM_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
    CEM_EVENTS_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_SCHEMA_URI,
};
use crate::source::{ByteRange, BytesSource, SourceId};
use crate::source_map::{FrameSpan, SourceMapStack};
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::html::HtmlTokenizer;
use crate::tokenizer::xml::XmlTokenizer;
use crate::tokenizer::SchemaTokenizer;
use serde_json::{json, Value};

const BINARY_FORMAT_VERSION: &str = "cem-projection-bin/1";
const BINARY_MAGIC: &[u8; 8] = b"CEMPROJ\0";
const BINARY_VERSION: u16 = 1;
const HASH_SCHEME: &str = crate::content_cache::HASH_SCHEME;
const ROOT_CHUNK_ID: &str = "root";

#[derive(Clone, Copy)]
enum BinaryProjectionKind {
    Dom,
    Ast,
    Events,
}

impl BinaryProjectionKind {
    fn tag(self) -> u8 {
        match self {
            Self::Dom => 1,
            Self::Ast => 2,
            Self::Events => 3,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Dom => "dom",
            Self::Ast => "ast",
            Self::Events => "events",
        }
    }

    fn schema(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_PROJECTION_SCHEMA_URI,
            Self::Ast => CEM_AST_PROJECTION_SCHEMA_URI,
            Self::Events => CEM_EVENTS_PROJECTION_SCHEMA_URI,
        }
    }

    fn content_type(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_PROJECTION_CONTENT_TYPE,
            Self::Ast => CEM_AST_PROJECTION_CONTENT_TYPE,
            Self::Events => CEM_EVENTS_PROJECTION_CONTENT_TYPE,
        }
    }
}

/// Project a built `CemDocument` to the first canonical CEM DOM binary artifact.
///
/// The current response is a JSON envelope because the CLI/lib response boundary
/// is still `serde_json::Value`; the chunk payload inside the envelope is the
/// hash-addressed binary artifact bytes.
pub fn dom_binary_artifact(doc: &CemDocument) -> Value {
    document_binary_artifact(doc, BinaryProjectionKind::Dom)
}

/// Project a built `CemDocument` to the first canonical CEM AST binary artifact.
pub fn ast_binary_artifact(doc: &CemDocument) -> Value {
    document_binary_artifact(doc, BinaryProjectionKind::Ast)
}

/// Project the input source to the first canonical CEM event-stream binary
/// artifact.
pub fn events_binary_artifact_as(input: &[u8], from_format: InputFormat) -> Value {
    let bytes = encode_events_binary(input, from_format);
    binary_artifact_envelope(BinaryProjectionKind::Events, bytes)
}

fn document_binary_artifact(doc: &CemDocument, kind: BinaryProjectionKind) -> Value {
    let bytes = encode_document_binary(doc, kind);
    binary_artifact_envelope(kind, bytes)
}

fn binary_artifact_envelope(kind: BinaryProjectionKind, bytes: Vec<u8>) -> Value {
    let hash_hex = blake3::hash(&bytes).to_hex().to_string();
    let hash = format!("{HASH_SCHEME}:{hash_hex}");
    json!({
        "kind": "cem-binary-projection",
        "projection": kind.name(),
        "schema": kind.schema(),
        "contentType": kind.content_type(),
        "formatVersion": BINARY_FORMAT_VERSION,
        "hashScheme": HASH_SCHEME,
        "hash": hash,
        "byteLength": bytes.len(),
        "chunks": [{
            "id": ROOT_CHUNK_ID,
            "sealed": true,
            "byteOffset": 0,
            "byteLength": bytes.len(),
            "hash": hash,
            "dataEncoding": "hex",
            "data": hex_encode(&bytes),
        }],
    })
}

fn encode_document_binary(doc: &CemDocument, kind: BinaryProjectionKind) -> Vec<u8> {
    let mut out = Vec::new();
    write_binary_header(&mut out, kind);
    write_u32(&mut out, doc.nodes.len() as u32);
    for node in doc.nodes.iter() {
        encode_ast_node(&mut out, node);
    }
    out
}

fn encode_events_binary(input: &[u8], from_format: InputFormat) -> Vec<u8> {
    let src = BytesSource::new(SourceId(1), input.to_vec());
    let mut events = match from_format {
        InputFormat::Cem => collect_normalized_events(CemTokenizer::from_source(src)),
        InputFormat::Html => collect_normalized_events(HtmlTokenizer::from_source(src)),
        InputFormat::Xml => collect_normalized_events(XmlTokenizer::from_source(src)),
    };
    let mut out = Vec::new();
    write_binary_header(&mut out, BinaryProjectionKind::Events);
    write_u32(&mut out, events.len() as u32);
    for (sequence, event) in events.drain(..).enumerate() {
        encode_event(&mut out, sequence as u64, &event);
    }
    out
}

fn write_binary_header(out: &mut Vec<u8>, kind: BinaryProjectionKind) {
    out.extend_from_slice(BINARY_MAGIC);
    write_u16(out, BINARY_VERSION);
    write_u8(out, kind.tag());
    write_str(out, kind.schema());
    write_str(out, kind.content_type());
}

fn encode_ast_node(out: &mut Vec<u8>, node: &CemAstNode) {
    match node {
        CemAstNode::Document {
            node_id,
            root_children,
            source,
        } => {
            write_u8(out, 1);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_id_list(out, root_children);
        }
        CemAstNode::Element {
            node_id,
            expanded_name,
            attributes,
            children,
            has_explicit_boundary,
            source,
        } => {
            write_u8(out, 2);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_expanded_name(out, expanded_name);
            write_bool(out, *has_explicit_boundary);
            write_id_list(out, attributes);
            write_id_list(out, children);
        }
        CemAstNode::Attribute {
            node_id,
            expanded_name,
            value,
            source,
        } => {
            write_u8(out, 3);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_expanded_name(out, expanded_name);
            write_optional_str(out, value.as_deref());
        }
        CemAstNode::Text {
            node_id,
            data,
            source,
        } => encode_text_node(out, 4, *node_id, data, source),
        CemAstNode::Whitespace {
            node_id,
            data,
            source,
        } => encode_text_node(out, 5, *node_id, data, source),
        CemAstNode::Comment {
            node_id,
            data,
            source,
        } => encode_text_node(out, 6, *node_id, data, source),
        CemAstNode::ProcessingInstruction {
            node_id,
            target,
            data,
            source,
        } => {
            write_u8(out, 7);
            write_u32(out, *node_id);
            write_source_range(out, stack_origin(source));
            write_str(out, target);
            write_str(out, data);
        }
        CemAstNode::Cdata {
            node_id,
            data,
            source,
        } => encode_text_node(out, 8, *node_id, data, source),
        CemAstNode::RawText {
            node_id,
            data,
            source,
        } => encode_text_node(out, 9, *node_id, data, source),
        CemAstNode::Error {
            node_id,
            code,
            source,
        } => encode_text_node(out, 10, *node_id, code, source),
    }
}

fn encode_text_node(
    out: &mut Vec<u8>,
    tag: u8,
    node_id: AstNodeId,
    data: &str,
    source: &SourceMapStack,
) {
    write_u8(out, tag);
    write_u32(out, node_id);
    write_source_range(out, stack_origin(source));
    write_str(out, data);
}

fn write_expanded_name(out: &mut Vec<u8>, name: &ExpandedName) {
    write_str(out, &name.namespace_uri);
    write_str(out, &name.local_name);
    match name.schema_id {
        Some(schema_id) => {
            write_bool(out, true);
            write_u32(out, schema_id);
        }
        None => write_bool(out, false),
    }
}

fn encode_event(out: &mut Vec<u8>, sequence: u64, event: &NormalizedEvent) {
    write_u64(out, sequence);
    match event {
        NormalizedEvent::OpenScope {
            name, byte_range, ..
        } => {
            write_u8(out, 1);
            write_source_range(out, Some(*byte_range));
            write_qname(out, name);
        }
        NormalizedEvent::CloseScope {
            name,
            byte_range,
            synthesis,
            ..
        } => {
            write_u8(out, 2);
            write_source_range(out, Some(*byte_range));
            write_qname(out, name);
            write_u8(out, synthesis_tag(synthesis));
        }
        NormalizedEvent::Name { name, byte_range } => {
            write_u8(out, 3);
            write_source_range(out, Some(*byte_range));
            write_qname(out, name);
        }
        NormalizedEvent::Value { value, byte_range } => {
            write_u8(out, 4);
            write_source_range(out, Some(*byte_range));
            write_scalar_value(out, value);
        }
        NormalizedEvent::Trivia {
            kind,
            data,
            byte_range,
        } => {
            write_u8(out, 5);
            write_source_range(out, Some(*byte_range));
            write_u8(
                out,
                match kind {
                    TriviaKind::Whitespace => 1,
                    TriviaKind::Comment => 2,
                },
            );
            write_str(out, data);
        }
        NormalizedEvent::ProcessingInstruction {
            target,
            data,
            byte_range,
        } => {
            write_u8(out, 6);
            write_source_range(out, Some(*byte_range));
            write_str(out, target);
            write_str(out, data);
        }
        NormalizedEvent::Separator { kind, byte_range } => {
            write_u8(out, 7);
            write_source_range(out, Some(*byte_range));
            write_u8(out, separator_tag(kind));
        }
        NormalizedEvent::ModeSwitch {
            content_type,
            handoff,
        } => {
            write_u8(out, 8);
            write_source_range(out, Some(handoff.source_span));
            write_str(out, content_type);
        }
        NormalizedEvent::Error {
            code,
            byte_range,
            severity,
        } => {
            write_u8(out, 9);
            write_source_range(out, Some(*byte_range));
            write_str(out, code);
            write_u8(out, severity_tag(*severity));
        }
    }
}

fn write_qname(out: &mut Vec<u8>, name: &crate::events::QName) {
    write_str(out, &name.lexical_name);
    write_optional_str(out, name.prefix.as_deref());
    write_str(out, &name.local_name);
    write_source_range(out, Some(name.source_range));
}

fn write_scalar_value(out: &mut Vec<u8>, value: &ScalarValue) {
    match value {
        ScalarValue::Text(value) => {
            write_u8(out, 1);
            write_str(out, value);
        }
        ScalarValue::Int(value) => {
            write_u8(out, 2);
            out.extend_from_slice(&value.to_be_bytes());
        }
        ScalarValue::Float(value) => {
            write_u8(out, 3);
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        ScalarValue::Bool(value) => {
            write_u8(out, 4);
            write_bool(out, *value);
        }
        ScalarValue::Null => write_u8(out, 5),
    }
}

fn separator_tag(kind: &SeparatorKind) -> u8 {
    match kind {
        SeparatorKind::ElementBoundary => 1,
        SeparatorKind::Comma => 2,
        SeparatorKind::Colon => 3,
        SeparatorKind::Delimiter => 4,
        SeparatorKind::Newline => 5,
    }
}

fn synthesis_tag(synthesis: &Synthesis) -> u8 {
    match synthesis {
        Synthesis::Real => 1,
        Synthesis::SelfClosing => 2,
        Synthesis::VoidElement => 3,
        Synthesis::ImpliedByStartTag => 4,
        Synthesis::ImpliedByAncestorClose => 5,
        Synthesis::ImpliedByEof => 6,
    }
}

fn severity_tag(severity: crate::diagnostics::Severity) -> u8 {
    match severity {
        crate::diagnostics::Severity::Info => 1,
        crate::diagnostics::Severity::Warning => 2,
        crate::diagnostics::Severity::Error => 3,
        crate::diagnostics::Severity::Fatal => 4,
    }
}

fn collect_normalized_events<T: SchemaTokenizer>(tok: T) -> Vec<NormalizedEvent> {
    let mut n = CemEventNormalizer::new(tok);
    let mut out = Vec::new();
    while let Some(ev) = n.next_event() {
        out.push(ev);
    }
    out
}

fn write_id_list(out: &mut Vec<u8>, ids: &[AstNodeId]) {
    write_u32(out, ids.len() as u32);
    for id in ids {
        write_u32(out, *id);
    }
}

fn write_source_range(out: &mut Vec<u8>, range: Option<ByteRange>) {
    match range {
        Some(range) => {
            write_bool(out, true);
            write_u64(out, range.start);
            write_u32(out, range.len);
        }
        None => write_bool(out, false),
    }
}

fn write_optional_str(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            write_bool(out, true);
            write_str(out, value);
        }
        None => write_bool(out, false),
    }
}

fn write_str(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    write_u8(out, u8::from(value));
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Project a built `CemDocument` to a DOM-JSON tree:
///
/// ```json
/// {
///   "kind": "document",
///   "children": [
///     {
///       "kind": "element",
///       "name": "main",
///       "namespace": "",
///       "attributes": [{ "name": "cem:screen", "value": "login", "namespace": "cem" }],
///       "children": [...],
///       "byteRange": { "start": 130, "len": 12 }
///     }
///   ]
/// }
/// ```
pub fn dom_json(doc: &CemDocument) -> Value {
    let root = doc.root().cloned();
    match root {
        Some(CemAstNode::Document { root_children, .. }) => json!({
            "kind": "document",
            "children": root_children.iter().filter_map(|id| project_node(doc, *id)).collect::<Vec<_>>(),
        }),
        _ => Value::Null,
    }
}

fn project_node(doc: &CemDocument, id: AstNodeId) -> Option<Value> {
    let node = doc.get(id)?;
    let value = match node {
        CemAstNode::Document { root_children, .. } => json!({
            "kind": "document",
            "children": root_children.iter().filter_map(|id| project_node(doc, *id)).collect::<Vec<_>>(),
        }),
        CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            source,
            ..
        } => {
            let attrs: Vec<Value> = attributes
                .iter()
                .filter_map(|aid| match doc.get(*aid)? {
                    CemAstNode::Attribute {
                        expanded_name,
                        value,
                        ..
                    } => Some(json!({
                        "name": expanded_name.local_name,
                        "namespace": expanded_name.namespace_uri,
                        "value": value,
                    })),
                    _ => None,
                })
                .collect();
            json!({
                "kind": "element",
                "name": expanded_name.local_name,
                "namespace": expanded_name.namespace_uri,
                "attributes": attrs,
                "children": children.iter().filter_map(|cid| project_node(doc, *cid)).collect::<Vec<_>>(),
                "byteRange": project_byte_range(source.frames.first().and_then(|f| match &f.span {
                    crate::source_map::FrameSpan::Single(r) => Some(*r),
                    crate::source_map::FrameSpan::Multi(rs) => rs.first().copied(),
                })),
            })
        }
        CemAstNode::Text { data, source, .. } => json!({
            "kind": "text",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Whitespace { data, source, .. } => json!({
            "kind": "whitespace",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Comment { data, source, .. } => json!({
            "kind": "comment",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::ProcessingInstruction {
            target,
            data,
            source,
            ..
        } => json!({
            "kind": "processing-instruction",
            "target": target,
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Cdata { data, source, .. } => json!({
            "kind": "cdata",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::RawText { data, source, .. } => json!({
            "kind": "raw-text",
            "data": data,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Error { code, source, .. } => json!({
            "kind": "error",
            "code": code,
            "byteRange": project_byte_range(stack_origin(source)),
        }),
        CemAstNode::Attribute { .. } => return None,
    };
    Some(value)
}

fn stack_origin(stack: &SourceMapStack) -> Option<ByteRange> {
    stack.frames.first().and_then(|f| match &f.span {
        FrameSpan::Single(r) => Some(*r),
        FrameSpan::Multi(rs) => rs.first().copied(),
    })
}

fn project_byte_range(range: Option<ByteRange>) -> Value {
    match range {
        Some(r) => json!({ "start": r.start, "len": r.len }),
        None => Value::Null,
    }
}

/// Project the parsed AST as a typed-tree JSON (alias for `dom_json` in
/// Tier A; future CEM-specific projections add the `annotations` /
/// `state` fields here).
pub fn ast_json(doc: &CemDocument) -> Value {
    dom_json(doc)
}

/// Project the input source as a flat list of normalized events:
///
/// ```json
/// [
///   { "kind": "open", "name": "main", "byteRange": {...} },
///   { "kind": "name", "name": "cem:screen" },
///   { "kind": "value", "value": "login" },
///   { "kind": "close", "name": "main" }
/// ]
/// ```
pub fn events_json(input: &[u8]) -> Value {
    events_json_as(input, InputFormat::Cem)
}

pub fn events_json_as(input: &[u8], from_format: InputFormat) -> Value {
    let src = BytesSource::new(SourceId(1), input.to_vec());
    match from_format {
        InputFormat::Cem => collect_events(CemTokenizer::from_source(src)),
        InputFormat::Html => collect_events(HtmlTokenizer::from_source(src)),
        InputFormat::Xml => collect_events(XmlTokenizer::from_source(src)),
    }
}

fn collect_events<T: SchemaTokenizer>(tok: T) -> Value {
    let mut n = CemEventNormalizer::new(tok);
    let mut out: Vec<Value> = Vec::new();
    while let Some(ev) = n.next_event() {
        out.push(event_to_json(&ev));
    }
    Value::Array(out)
}

fn event_to_json(ev: &NormalizedEvent) -> Value {
    match ev {
        NormalizedEvent::OpenScope {
            name, byte_range, ..
        } => json!({
            "kind": "open",
            "name": name.lexical_name,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::CloseScope {
            name, byte_range, ..
        } => json!({
            "kind": "close",
            "name": name.lexical_name,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::Name { name, byte_range } => json!({
            "kind": "name",
            "name": name.lexical_name,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::Value { value, byte_range } => {
            let v = match value {
                ScalarValue::Text(t) => Value::String(t.clone()),
                ScalarValue::Int(i) => json!(*i),
                ScalarValue::Float(f) => json!(*f),
                ScalarValue::Bool(b) => Value::Bool(*b),
                ScalarValue::Null => Value::Null,
            };
            json!({
                "kind": "value",
                "value": v,
                "byteRange": project_byte_range(Some(*byte_range)),
            })
        }
        NormalizedEvent::Trivia {
            kind,
            data,
            byte_range,
        } => json!({
            "kind": "trivia",
            "trivia": match kind { TriviaKind::Whitespace => "whitespace", TriviaKind::Comment => "comment" },
            "data": data,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::ProcessingInstruction {
            target,
            data,
            byte_range,
        } => json!({
            "kind": "processing-instruction",
            "target": target,
            "data": data,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::Separator { byte_range, .. } => json!({
            "kind": "separator",
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
        NormalizedEvent::ModeSwitch { content_type, .. } => json!({
            "kind": "mode-switch",
            "contentType": content_type,
        }),
        NormalizedEvent::Error {
            code, byte_range, ..
        } => json!({
            "kind": "error",
            "code": code,
            "byteRange": project_byte_range(Some(*byte_range)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::builder::CemAstBuilder;

    fn parse(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    #[test]
    fn dom_json_root_is_document_kind() {
        let doc = parse("{p Hi}");
        let v = dom_json(&doc);
        assert_eq!(v["kind"], "document");
        assert!(v["children"].is_array());
    }

    #[test]
    fn dom_json_element_has_name_and_attributes() {
        let doc = parse(r#"{button @cem:action=primary | Save}"#);
        let v = dom_json(&doc);
        let button = v["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "button")
            .unwrap();
        assert_eq!(button["kind"], "element");
        let attr = &button["attributes"][0];
        assert_eq!(attr["name"], "action");
        assert_eq!(attr["namespace"], "cem");
        assert_eq!(attr["value"], "primary");
    }

    #[test]
    fn events_json_open_close_round_trip() {
        let v = events_json(b"{p Hi}");
        let arr = v.as_array().unwrap();
        let opens: Vec<&Value> = arr.iter().filter(|e| e["kind"] == "open").collect();
        let closes: Vec<&Value> = arr.iter().filter(|e| e["kind"] == "close").collect();
        assert_eq!(opens.len(), closes.len());
        assert_eq!(opens[0]["name"], "p");
    }

    #[test]
    fn dom_binary_artifact_is_hash_addressed_sealed_chunk() {
        let doc = parse("{p Hi}");
        let v = dom_binary_artifact(&doc);
        let chunk = &v["chunks"][0];

        assert_eq!(v["kind"], "cem-binary-projection");
        assert_eq!(v["projection"], "dom");
        assert_eq!(v["contentType"], CEM_DOM_PROJECTION_CONTENT_TYPE);
        assert_eq!(v["hashScheme"], HASH_SCHEME);
        assert_eq!(chunk["sealed"], true);
        assert_eq!(chunk["byteOffset"], 0);
        assert_eq!(chunk["byteLength"], v["byteLength"]);
        assert_eq!(chunk["hash"], v["hash"]);
        assert!(v["hash"].as_str().unwrap().starts_with("cem-bin/1+blake3:"));
        assert!(chunk["data"]
            .as_str()
            .unwrap()
            .starts_with("43454d50524f4a00"));
    }

    #[test]
    fn ast_binary_artifact_is_deterministic_for_same_document() {
        let doc = parse("{button @cem:action=primary | Save}");
        let first = ast_binary_artifact(&doc);
        let second = ast_binary_artifact(&doc);

        assert_eq!(first["projection"], "ast");
        assert_eq!(first["contentType"], CEM_AST_PROJECTION_CONTENT_TYPE);
        assert_eq!(first["hash"], second["hash"]);
        assert_eq!(first["chunks"][0]["data"], second["chunks"][0]["data"]);
    }

    #[test]
    fn events_binary_artifact_encodes_event_stream() {
        let v = events_binary_artifact_as(b"{p Hi}", InputFormat::Cem);

        assert_eq!(v["projection"], "events");
        assert_eq!(v["contentType"], CEM_EVENTS_PROJECTION_CONTENT_TYPE);
        assert!(v["byteLength"].as_u64().unwrap() > BINARY_MAGIC.len() as u64);
        assert!(v["chunks"][0]["data"]
            .as_str()
            .unwrap()
            .starts_with("43454d50524f4a00"));
    }
}
