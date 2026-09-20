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
    let source = owner
        .node(0)
        .expect("retained document root")
        .source
        .clone();
    let mut stream = inspection(owner.ast(), owner.source_uri(), &source, Some(&owner));
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
    inspection(document, source_uri, &source, None)
}

fn inspection(
    document: &CemDocument,
    source_uri: &str,
    document_source: &SourceMapStack,
    owner: Option<&RetainedCemTree>,
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
                ..
            } => (
                "attribute",
                Some(expanded_name.local_name.as_str()),
                Some(expanded_name.namespace_uri.as_str()),
                value.as_deref(),
                None,
            ),
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
            projection_attribute("schema", CEM_ML_SCHEMA_URI, document_source),
            projection_attribute("content-type", CEM_ML_CONTENT_TYPE, document_source),
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
