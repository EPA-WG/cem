//! JSON AST → native CEM AST using the W3C JSON-to-XML node mapping.
//!
//! This is an explicit untyped projection, not an XML serialization boundary or
//! a complete implementation of the XPath fn:json-to-xml function. The caller
//! retains the JSON owner; nodes keep original JSON source frames. Standard
//! function argument/error handling remains the responsibility of its adapter.
use super::json::{JsonDocumentAst, JsonMemberAst, JsonSourceRange, JsonValueAst};
use crate::parser::{
    document::CemDocument, tree::{CemTreeRange, CemTreeSemantics},
    AstNodeId, CemAstNode, ExpandedName,
};
use crate::source_map::{SourceMapStack, TransformKind};
use std::collections::BTreeSet;
use std::fmt::{self, Write};

pub const JSON_XML_NAMESPACE: &str = "http://www.w3.org/2005/xpath-functions";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsonXmlDuplicates {
    /// The standard untyped JSON-to-XML default. The source AST keeps all keys.
    #[default]
    Retain,
    UseFirst,
    Reject,
}

#[derive(Debug, Clone, Copy)]
pub struct JsonXmlProjectionOptions {
    /// Escape controls/backslashes and mark escaped values, or use XML 1.0
    /// characters with U+FFFD as the default fallback for invalid characters.
    pub escape: bool,
    pub duplicates: JsonXmlDuplicates,
    /// Root value has depth zero. Traversal is iterative even for larger limits.
    pub max_depth: usize,
    /// Maximum emitted JSON values, excluding generated key/text nodes.
    pub max_values: usize,
}

impl Default for JsonXmlProjectionOptions {
    fn default() -> Self {
        Self {
            escape: false,
            duplicates: JsonXmlDuplicates::Retain,
            max_depth: 64,
            max_values: 4096,
        }
    }
}

#[derive(Debug)]
pub struct JsonXmlProjectionError {
    pub code: &'static str,
    pub message: String,
    pub source: SourceMapStack,
}

impl fmt::Display for JsonXmlProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for JsonXmlProjectionError {}

fn error(
    code: &'static str,
    message: impl Into<String>,
    source: SourceMapStack,
) -> JsonXmlProjectionError {
    JsonXmlProjectionError {
        code,
        message: message.into(),
        source,
    }
}

struct Pending<'a> {
    parent: AstNodeId,
    value: &'a JsonValueAst,
    key: Option<&'a JsonMemberAst>,
    depth: usize,
}

/// Build the standard map/array/keyed-value structure without a serializer,
/// reparser, loss of member order, or any presentation-specific policy.
///
/// Import selects the default byte-parser profile or the standard string
/// profile. Original string lexemes retain codepoints absent from Rust strings.
/// This projection provides no schema validation or custom fallback callback.
pub fn project_json_to_xml(
    json: &JsonDocumentAst,
    options: &JsonXmlProjectionOptions,
) -> Result<CemDocument, JsonXmlProjectionError> {
    project_json_to_xml_with_semantics(json, options).map(|(ast, _)| ast)
}

/// Import retains original parser locations, including byte-only and non-UTF-8
/// inputs. Consumers never reconstruct source lines from a serialized tree.
pub fn project_json_to_xml_with_semantics(
    json: &JsonDocumentAst,
    options: &JsonXmlProjectionOptions,
) -> Result<(CemDocument, CemTreeSemantics), JsonXmlProjectionError> {
    if let Some(fact) = json.parse_facts.iter().find(|f| f.fatal) {
        return Err(error(
            "cem.json.xml_projection.invalid_source",
            &fact.message,
            json.root
                .as_ref()
                .map(|r| r.range().source_map())
                .unwrap_or_default(),
        ));
    }
    let root = json.root.as_ref().ok_or_else(|| {
        error(
            "cem.json.xml_projection.invalid_source",
            "No complete JSON value was parsed.",
            SourceMapStack::default(),
        )
    })?;
    let mut doc = CemDocument::default();
    let mut semantics = CemTreeSemantics::default();
    semantics.ranges.insert(0, tree_range(root.range()));
    doc.nodes.push(CemAstNode::Document {
        node_id: 0,
        root_children: vec![],
        source: projected_source(root.range()),
    });
    let mut pending = vec![Pending {
        parent: 0,
        value: root,
        key: None,
        depth: 0,
    }];
    let mut visited = 0;
    while let Some(Pending {
        parent,
        value,
        key,
        depth,
    }) = pending.pop()
    {
        visited += 1;
        if depth > options.max_depth
            || visited > options.max_values
            || doc.nodes.len() > u32::MAX as usize - 5
        {
            return Err(error(
                "cem.json.xml_projection.limit",
                "JSON-to-XML projection exceeds its depth/value/node limit.",
                value.range().source_map(),
            ));
        }
        let (name, scalar) = match value {
            JsonValueAst::Object { .. } => ("map", None),
            JsonValueAst::Array { .. } => ("array", None),
            JsonValueAst::String { lexeme, .. } => (
                "string",
                Some(xml_lexeme(lexeme, options.escape, value.range())?),
            ),
            JsonValueAst::Number { lexeme, .. } => ("number", Some(lexeme.clone())),
            JsonValueAst::Boolean { value, .. } => ("boolean", Some(value.to_string())),
            JsonValueAst::Null { .. } => ("null", None),
        };
        let source = projected_source(value.range());
        let id = push(
            &mut doc,
            parent,
            CemAstNode::Element {
                node_id: 0, // push assigns the arena index.
                expanded_name: expanded(JSON_XML_NAMESPACE, name),
                attributes: vec![],
                children: vec![],
                has_explicit_boundary: true,
                source: source.clone(),
            },
        );
        semantics.ranges.insert(id, tree_range(value.range()));
        if let Some(member) = key {
            let key = xml_lexeme(&member.name_lexeme, options.escape, member.name_range)?;
            let escaped = options.escape && key.contains('\\');
            let key_id = attribute(
                &mut doc,
                id,
                "key",
                key,
                projected_source(member.name_range),
            );
            semantics.ranges.insert(key_id, tree_range(member.name_range));
            if escaped {
                let escaped_id = attribute(
                    &mut doc,
                    id,
                    "escaped-key",
                    "true".into(),
                    projected_source(member.name_range),
                );
                semantics.ranges.insert(escaped_id, tree_range(member.name_range));
            }
        }
        if let Some(scalar) = scalar {
            if name == "string" && options.escape && scalar.contains('\\') {
                let escaped_id = attribute(&mut doc, id, "escaped", "true".into(), source.clone());
                semantics.ranges.insert(escaped_id, tree_range(value.range()));
            }
            // XDM does not contain zero-length text nodes.
            if !scalar.is_empty() {
                let text_id = push(
                    &mut doc,
                    id,
                    CemAstNode::Text {
                        node_id: 0,
                        data: scalar,
                        source,
                    },
                );
                semantics.ranges.insert(text_id, tree_range(value.range()));
            }
        }
        match value {
            JsonValueAst::Object { members, .. } => {
                let mut keys = BTreeSet::new();
                let mut selected = Vec::new();
                for member in members {
                    let identity = if options.escape {
                        xml_lexeme(&member.name_lexeme, true, member.name_range)?
                            .chars()
                            .map(|c| c as u32)
                            .collect()
                    } else {
                        codepoints(&member.name_lexeme, member.name_range)?
                    };
                    if options.duplicates != JsonXmlDuplicates::Retain && !keys.insert(identity) {
                        if options.duplicates == JsonXmlDuplicates::Reject {
                            return Err(error(
                                "cem.json.xml_projection.duplicate_key",
                                format!("Duplicate JSON key {:?}.", member.name),
                                member.name_range.source_map(),
                            ));
                        }
                        continue;
                    }
                    selected.push(member);
                }
                pending.extend(selected.into_iter().rev().map(|member| Pending {
                    parent: id,
                    value: &member.value,
                    key: Some(member),
                    depth: depth + 1,
                }));
            }
            JsonValueAst::Array { items, .. } => {
                pending.extend(items.iter().rev().map(|value| Pending {
                    parent: id,
                    value,
                    key: None,
                    depth: depth + 1,
                }))
            }
            _ => {}
        }
    }
    Ok((doc, semantics))
}

fn tree_range(range: JsonSourceRange) -> CemTreeRange {
    CemTreeRange {
        line: range.start.line,
        column: range.start.column,
        offset: range.start.byte_offset,
        length: range.byte_length,
    }
}

fn projected_source(range: JsonSourceRange) -> SourceMapStack {
    let mut source = range.source_map();
    if let Some(origin) = source.origin().cloned() {
        let mut frame = origin;
        frame.transform = TransformKind::CemAstBuilder;
        source.frames.push(frame);
    }
    source
}

fn expanded(namespace: &str, name: &str) -> ExpandedName {
    ExpandedName {
        namespace_uri: namespace.into(),
        local_name: name.into(),
        schema_id: None,
    }
}

fn push(doc: &mut CemDocument, parent: AstNodeId, mut node: CemAstNode) -> AstNodeId {
    let id = doc.nodes.len() as AstNodeId;
    match &mut node {
        CemAstNode::Element { node_id, .. }
        | CemAstNode::Attribute { node_id, .. }
        | CemAstNode::Text { node_id, .. } => *node_id = id,
        _ => unreachable!("only JSON value elements, attributes and text are constructed"),
    }
    let attribute = matches!(node, CemAstNode::Attribute { .. });
    doc.nodes.push(node);
    match &mut doc.nodes[parent as usize] {
        CemAstNode::Document { root_children, .. } => root_children.push(id),
        CemAstNode::Element {
            attributes,
            children,
            ..
        } => {
            if attribute {
                attributes.push(id);
            } else {
                children.push(id);
            }
        }
        _ => unreachable!("a projected parent is a document or element"),
    }
    id
}

fn attribute(
    doc: &mut CemDocument,
    parent: AstNodeId,
    name: &str,
    value: String,
    source: SourceMapStack,
) -> AstNodeId {
    push(
        doc,
        parent,
        CemAstNode::Attribute {
            node_id: 0,
            expanded_name: expanded("", name),
            value: Some(value),
            source,
        },
    )
}

fn codepoints(lexeme: &str, range: JsonSourceRange) -> Result<Vec<u32>, JsonXmlProjectionError> {
    super::json::json_string_codepoints(lexeme).map_err(|message| {
        error(
            "cem.json.xml_projection.invalid_source",
            message,
            range.source_map(),
        )
    })
}

fn xml_lexeme(
    lexeme: &str,
    escape: bool,
    range: JsonSourceRange,
) -> Result<String, JsonXmlProjectionError> {
    let mut out = String::new();
    for cp in codepoints(lexeme, range)? {
        let Some(c) = char::from_u32(cp) else {
            if escape {
                write!(out, "\\u{cp:04x}").expect("string write");
            } else {
                out.push('\u{fffd}');
            }
            continue;
        };
        let valid = matches!(c, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}');
        if escape && (c <= '\u{1f}' || ('\u{7f}'..='\u{9f}').contains(&c) || !valid || c == '\\') {
            match c {
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                _ => {
                    write!(out, "\\u{:04x}", c as u32).expect("writing into a String");
                }
            }
        } else {
            out.push(if valid { c } else { '\u{fffd}' });
        }
    }
    Ok(out)
}
