//! Query helpers over a built `CemDocument`.
//!
//! Tier A coverage per AC-Q-*: role / state lookup, validation message
//! traversal, label resolution via `id_table`, and source-map lookup that
//! traces any node back to its origin byte range.

use crate::diagnostics::Diagnostic;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame};

/// Return the element node id whose `id="..."` attribute matched `target`,
/// or `None` if no element registered that id.
pub fn find_by_id<'a>(doc: &'a CemDocument, target: &str) -> Option<&'a CemAstNode> {
    let id = doc.id_table.get(target)?;
    doc.get(*id)
}

/// Iterator over every element node in document order.
pub fn elements(doc: &CemDocument) -> impl Iterator<Item = &CemAstNode> {
    doc.iter()
        .filter(|n| matches!(n, CemAstNode::Element { .. }))
}

/// Element node ids whose lexical local name (after `:` if present)
/// matches `local`.
pub fn find_by_local_name<'a>(
    doc: &'a CemDocument,
    local: &'a str,
) -> impl Iterator<Item = &'a CemAstNode> {
    doc.iter().filter(move |n| match n {
        CemAstNode::Element { expanded_name, .. } => expanded_name.local_name == local,
        _ => false,
    })
}

/// Every attribute on an element whose name carries the given namespace
/// prefix (e.g. `"cem"`). Tier A AST stores the lexical prefix in
/// `expanded_name.namespace_uri` until full namespace expansion lands;
/// `prefix = ""` selects unprefixed attributes.
pub fn attributes_in_prefix<'a>(
    doc: &'a CemDocument,
    element: &'a CemAstNode,
    prefix: &'a str,
) -> impl Iterator<Item = &'a CemAstNode> {
    let attr_ids: &[AstNodeId] = match element {
        CemAstNode::Element { attributes, .. } => attributes,
        _ => &[],
    };
    attr_ids.iter().filter_map(move |id| {
        let node = doc.get(*id)?;
        if let CemAstNode::Attribute { expanded_name, .. } = node {
            if expanded_name.namespace_uri == prefix {
                return Some(node);
            }
        }
        None
    })
}

/// CEM annotations on an element: attributes in the `cem:` namespace
/// excluding the `cem:state` attribute.
pub fn cem_annotations<'a>(
    doc: &'a CemDocument,
    element: &'a CemAstNode,
) -> impl Iterator<Item = &'a CemAstNode> {
    attributes_in_prefix(doc, element, "cem").filter(|attr| match attr {
        CemAstNode::Attribute { expanded_name, .. } => expanded_name.local_name != "state",
        _ => false,
    })
}

/// Element node ids that carry the CEM annotation with the given local
/// name (e.g. `"screen"`, `"action"`).
pub fn elements_with_annotation<'a>(
    doc: &'a CemDocument,
    annotation_local: &'a str,
) -> impl Iterator<Item = &'a CemAstNode> {
    doc.iter().filter(move |node| {
        let CemAstNode::Element { attributes, .. } = node else {
            return false;
        };
        attributes.iter().any(|attr_id| match doc.get(*attr_id) {
            Some(CemAstNode::Attribute { expanded_name, .. }) => {
                expanded_name.namespace_uri == "cem" && expanded_name.local_name == annotation_local
            }
            _ => false,
        })
    })
}

/// Decoded state names attached to an element via `cem:state="..."`.
/// Returns an empty `Vec` if the element has no state attribute.
pub fn state_of(doc: &CemDocument, element: &CemAstNode) -> Vec<String> {
    let CemAstNode::Element { attributes, .. } = element else {
        return Vec::new();
    };
    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = doc.get(*attr_id)
        else {
            continue;
        };
        if expanded_name.namespace_uri == "cem" && expanded_name.local_name == "state" {
            return value
                .as_deref()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect();
        }
    }
    Vec::new()
}

/// Project a node back to its origin byte range, walking the source-map
/// stack origin-first.
pub fn origin_byte_range(node: &CemAstNode) -> Option<ByteRange> {
    let stack = match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    };
    stack.frames.first().and_then(|frame| match &frame.span {
        FrameSpan::Single(r) => Some(*r),
        FrameSpan::Multi(rs) => rs.first().copied(),
    })
}

/// Validation diagnostics on this document. Equivalent to `doc.diagnostics`,
/// kept as a function so consumers can compose with other queries.
pub fn validation_messages(doc: &CemDocument) -> &[Diagnostic] {
    &doc.diagnostics
}

/// Resolve a `for`/`aria-*` reference attribute's value through the
/// document `id_table`. Returns the resolved target node or `None` if the
/// reference is unresolved (which the AST builder already recorded as a
/// `cem.ast.unresolved_reference` diagnostic).
pub fn resolve_reference<'a>(
    doc: &'a CemDocument,
    attribute: &CemAstNode,
) -> Option<&'a CemAstNode> {
    let value = match attribute {
        CemAstNode::Attribute { value, .. } => value.as_deref()?,
        _ => return None,
    };
    find_by_id(doc, value)
}

/// Walk every source-map frame on a node from origin to current.
pub fn source_map_frames(node: &CemAstNode) -> &[SourceMapFrame] {
    let stack = match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    };
    &stack.frames
}
