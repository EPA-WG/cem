//! Structural inspection of common CEM nodes. External syntax belongs to import.
use super::{
    ast_node_source_map, projection_attribute, projection_element, projection_node,
    projection_vocabulary_document, CemTreeAstStream,
};
use crate::{
    parser::{document::CemDocument, tree::RetainedCemTree, AstNodeId, CemAstNode},
    schema::registry::{CEM_AST_PROJECTION_SCHEMA_URI, CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI},
    source_map::{FrameSpan, SourceMapStack},
};
use std::sync::Arc;

/// Present source CEM nodes as inert AST-vocabulary records, retaining the input
/// owner through formatting/coloring/writing. This is not a new query document.
pub fn cem_tree_inspection(owner: Arc<RetainedCemTree>) -> CemTreeAstStream {
    cem_tree_inspection_with_identity(owner, CEM_ML_SCHEMA_URI, CEM_ML_CONTENT_TYPE)
}

/// Inspect an imported native owner under its source language identity.
pub(crate) fn cem_tree_inspection_with_identity(
    owner: Arc<RetainedCemTree>,
    schema: &str,
    content_type: &str,
) -> CemTreeAstStream {
    let source = owner
        .node(0)
        .expect("retained document root")
        .source
        .clone();
    let mut stream = inspection(owner.ast(), owner.source_uri(), &source, Some(&owner), schema, content_type);
    stream.source_owner = Some(owner);
    stream
}

/// Inspect an existing CEM parser document, including recoverable error nodes.
/// External document owners must enter through import and `cem_tree_inspection`.
pub fn cem_document_inspection(document: &CemDocument, source_uri: &str) -> CemTreeAstStream {
    let source = document
        .root()
        .map(ast_node_source_map)
        .cloned()
        .unwrap_or_default();
    inspection(document, source_uri, &source, None, CEM_ML_SCHEMA_URI, CEM_ML_CONTENT_TYPE)
}

fn inspection(
    document: &CemDocument,
    source_uri: &str,
    document_source: &SourceMapStack,
    owner: Option<&RetainedCemTree>,
    schema: &str,
    content_type: &str,
) -> CemTreeAstStream {
    let mut rows = Vec::new();
    // Iterative preorder preserves arena identity and sibling/attribute order
    // without making inspection recurse on the native call stack.
    let mut pending = vec![(0, None, 0)];
    while let Some((id, parent, order)) = pending.pop() {
        let Some(node) = document.get(id) else {
            continue;
        };
        let source = if id == 0 {
            document_source
        } else {
            ast_node_source_map(node)
        };
        use CemAstNode::*;
        let (kind, name, namespace, value, target) = match node {
            Reference { .. } => ("reference", None, None, None, None),
            Document { root_children, .. } => {
                push_children(&mut pending, id, root_children, 0);
                ("document", None, None, None, None)
            }
            Element {
                expanded_name,
                attributes,
                children,
                ..
            } => {
                push_children(&mut pending, id, children, attributes.len());
                push_children(&mut pending, id, attributes, 0);
                (
                    "element",
                    Some(expanded_name.local_name.as_str()),
                    Some(expanded_name.namespace_uri.as_str()),
                    None,
                    None,
                )
            }
            Attribute {
                expanded_name,
                value,
                value_nodes,
                ..
            } => {
                push_children(&mut pending, id, value_nodes, 0);
                (
                    "attribute",
                    Some(expanded_name.local_name.as_str()),
                    Some(expanded_name.namespace_uri.as_str()),
                    value.as_deref(),
                    None,
                )
            }
            Text { data, .. } => ("text", None, None, Some(data.as_str()), None),
            Whitespace { data, .. } => ("whitespace", None, None, Some(data.as_str()), None),
            Cdata { data, .. } => ("cdata", None, None, Some(data.as_str()), None),
            RawText { data, .. } => ("raw-text", None, None, Some(data.as_str()), None),
            Comment { data, .. } => ("comment", None, None, Some(data.as_str()), None),
            ProcessingInstruction { target, data, .. } => (
                "processing-instruction",
                None,
                None,
                Some(data.as_str()),
                Some(target.as_str()),
            ),
            Error { code, .. } => ("error", None, None, Some(code.as_str()), None),
        };
        let range = owner.and_then(|tree| tree.source_node_range(id));
        let byte_range = range
            .map(|r| (r.offset, r.length))
            .or_else(|| covering_range(source));
        let mut row = projection_node(
            &format!("node-{id}"),
            kind,
            parent.map(|p| format!("node-{p}")).as_deref(),
            order,
            name,
            value,
            source_uri,
            byte_range,
            source.clone(),
        );
        let super::CemTreeAstNode::Element { attributes, .. } = &mut row else {
            unreachable!()
        };
        for (key, value) in [("namespace", namespace), ("target", target)] {
            if let Some(value) = value {
                attributes.push(projection_attribute(key, value, source));
            }
        }
        if let CemAstNode::Attribute { value_nodes, .. } = node {
            if !value_nodes.is_empty() {
                attributes.push(projection_attribute("value-node-ids", value_nodes.iter().map(u32::to_string).collect::<Vec<_>>().join(" "), source));
            }
        }
        if let CemAstNode::Reference { expression, context, targets, .. } = node {
            if let Some(targets) = targets {
                attributes.push(projection_attribute("target-ids", targets.iter().map(u32::to_string).collect::<Vec<_>>().join(" "), source));
            }
            attributes.push(projection_attribute("expression", expression, source));
            attributes.push(projection_attribute("context-id", context.to_string(), source));
            attributes.push(projection_attribute("evaluation-state", if targets.is_some() { "resolved" } else { "unevaluated" }, source));
        }
        if let Some(range) = range {

            // Byte-only origins may not have a known line/column.
            if range.line > 0 {
                attributes.push(projection_attribute("line", range.line.to_string(), source));
                attributes.push(projection_attribute(
                    "column",
                    range.column.to_string(),
                    source,
                ));
            }
        }
        rows.push(row);
    }
    let root = projection_element(
        "ast",
        vec![
            projection_attribute("schema", schema, document_source),
            projection_attribute("content-type", content_type, document_source),
            projection_attribute("hash-scheme", "blake3", document_source),
            projection_attribute("source-id", source_uri, document_source),
        ],
        rows,
        document_source.clone(),
    );
    projection_vocabulary_document(
        "cemast",
        CEM_AST_PROJECTION_SCHEMA_URI,
        root,
        document_source.clone(),
    )
}

fn push_children(
    pending: &mut Vec<(AstNodeId, Option<AstNodeId>, usize)>,
    parent: AstNodeId,
    children: &[AstNodeId],
    offset: usize,
) {
    pending.extend(
        children
            .iter()
            .enumerate()
            .rev()
            .map(|(order, &id)| (id, Some(parent), order + offset)),
    );
}

fn covering_range(source: &SourceMapStack) -> Option<(u64, u64)> {
    let mut start = u64::MAX;
    let mut end = 0;
    let spans = match &source.origin()?.span {
        FrameSpan::Single(span) => std::slice::from_ref(span),
        FrameSpan::Multi(spans) => spans.as_slice(),
    };
    for span in spans {
        start = start.min(span.start);
        end = end.max(span.start.saturating_add(span.len as u64));
    }
    (start != u64::MAX).then(|| (start, end.saturating_sub(start)))
}

/// Normalized events of the imported common tree, not lexical source tokens.
/// The original native owner and source ranges remain retained throughout.
pub(crate) fn cem_tree_events_with_identity(
    owner: Arc<RetainedCemTree>,
    schema: &str,
    content_type: &str,
) -> CemTreeAstStream {
    use super::CEM_EVENTS_PROJECTION_SCHEMA_URI;
    let source = owner
        .node(0)
        .expect("retained document root")
        .source
        .clone();
    let mut pending = vec![(0, false)];
    let mut rows = Vec::new();
    while let Some((id, closing)) = pending.pop() {
        let Some(node) = owner.ast().get(id) else {
            continue;
        };
        let (kind, node_kind, name, value) = match node {
            CemAstNode::Document { root_children, .. } => {
                pending.extend(root_children.iter().rev().map(|&id| (id, false)));
                continue;
            }
            CemAstNode::Element {
                expanded_name,
                attributes,
                children,
                ..
            } => {
                if !closing {
                    pending.push((id, true));
                    pending.extend(children.iter().rev().map(|&id| (id, false)));
                    pending.extend(attributes.iter().rev().map(|&id| (id, false)));
                }
                (
                    if closing { "close" } else { "open" },
                    "element",
                    Some(expanded_name.local_name.as_str()),
                    None,
                )
            }
            CemAstNode::Attribute {
                expanded_name,
                value,
                ..
            } => (
                "value",
                "attribute",
                Some(expanded_name.local_name.as_str()),
                value.as_deref(),
            ),
            CemAstNode::Text { data, .. } => ("value", "text", None, Some(data.as_str())),
            CemAstNode::Whitespace { data, .. } => {
                ("trivia", "whitespace", None, Some(data.as_str()))
            }
            CemAstNode::Cdata { data, .. } => ("value", "cdata", None, Some(data.as_str())),
            CemAstNode::RawText { data, .. } => ("value", "raw-text", None, Some(data.as_str())),
            CemAstNode::Comment { data, .. } => ("trivia", "comment", None, Some(data.as_str())),
            CemAstNode::ProcessingInstruction { target, data, .. } => (
                "processing-instruction",
                "processing-instruction",
                Some(target.as_str()),
                Some(data.as_str()),
            ),
            CemAstNode::Error { code, .. } => ("error", "error", None, Some(code.as_str())),
            CemAstNode::Reference { expression, .. } => {
                ("value", "reference", None, Some(expression.as_str()))
            }
        };
        let origin = ast_node_source_map(node);
        let mut attributes = vec![
            projection_attribute("sequence", rows.len().to_string(), origin),
            projection_attribute("kind", kind, origin),
            projection_attribute("node-kind", node_kind, origin),
            projection_attribute("event-domain", "syntax", origin),
            projection_attribute("source-id", owner.source_uri(), origin),
        ];
        if let Some(range) = owner.source_node_range(id) {
            attributes.push(projection_attribute(
                "byte-offset",
                range.offset.to_string(),
                origin,
            ));
            attributes.push(projection_attribute(
                "byte-length",
                range.length.to_string(),
                origin,
            ));
        }
        if let Some(name) = name {
            attributes.push(projection_attribute("name", name, origin));
        }
        if let Some(value) = value {
            attributes.push(projection_attribute("value", value, origin));
        }
        rows.push(projection_element(
            "event",
            attributes,
            Vec::new(),
            origin.clone(),
        ));
    }
    let root = projection_element(
        "event-stream",
        vec![
            projection_attribute("schema", schema, &source),
            projection_attribute("content-type", content_type, &source),
            projection_attribute("hash-scheme", "blake3", &source),
            projection_attribute("source-id", owner.source_uri(), &source),
        ],
        rows,
        source.clone(),
    );
    let mut stream =
        projection_vocabulary_document("cemevents", CEM_EVENTS_PROJECTION_SCHEMA_URI, root, source);
    stream.source_owner = Some(owner);
    stream
}
