//! JSON projections of the AST / events / DOM for CLI output formats.
//!
//! Distinct from the canonical-AST serde derives — these are stable,
//! consumer-friendly shapes for `cem-ml parse --format dom-json|ast|events`
//! and the same projections for `convert` / `inspect` views.

use crate::engine::InputFormat;
use crate::events::{
    cem::CemEventNormalizer, EventNormalizer, NormalizedEvent, ScalarValue, TriviaKind,
};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::{ByteRange, BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::html::HtmlTokenizer;
use crate::tokenizer::xml::XmlTokenizer;
use crate::tokenizer::SchemaTokenizer;
use serde_json::{json, Value};

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

fn stack_origin(stack: &crate::source_map::SourceMapStack) -> Option<ByteRange> {
    stack.frames.first().and_then(|f| match &f.span {
        crate::source_map::FrameSpan::Single(r) => Some(*r),
        crate::source_map::FrameSpan::Multi(rs) => rs.first().copied(),
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
}
