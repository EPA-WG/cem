//! Attach expression diagnostics to original source without interpreting syntax.
use super::*;
use cem_ml::{
    diagnostics::Diagnostic,
    source::{ByteRange, SourceId},
    source_map::{map_expression_range, FrameSpan, SourceMapFrame, TransformKind},
};

impl CemQlSchemaDeclarationHost {
    /// Structural validation already supplies original authored byte offsets and
    /// source frames. Resolve the owning tree by retained arena identity, never
    /// by a placement's effective scope or document-local source IDs.
    pub(super) fn authored_structural_diagnostic(
        &self,
        source: &SchemaDeclarationNode,
        mut diagnostic: Diagnostic,
    ) -> Diagnostic {
        if diagnostic.uri.is_some() {
            return diagnostic;
        }
        let Some(tree) = self.source_tree(source) else {
            return diagnostic;
        };
        // CEM tokenizer frames can cover the whole input. The authored AST
        // builder frame is the node/attribute location; retain the full stack.
        if let Some(frame) = diagnostic.source_map.as_ref().and_then(|stack| {
            stack
                .frames
                .iter()
                .rev()
                .find(|frame| matches!(frame.transform, TransformKind::CemAstBuilder))
        }) {
            diagnostic.byte_offset = match &frame.span {
                FrameSpan::Single(range) => Some(range.start),
                FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start),
            };
        }
        diagnostic.uri = Some(tree.source_uri().into());
        if let Some(coordinate) = diagnostic
            .byte_offset
            .and_then(|offset| tree.source_byte_coordinate(offset))
        {
            diagnostic.line = Some(coordinate.line);
            diagnostic.column = Some(coordinate.column);
        } else if let Some(range) = tree.source_node_range(source.node_id()) {
            // Imported owners can retain event coordinates without retaining a
            // source line index. Use only the actual matching authored location.
            if diagnostic.byte_offset == Some(range.offset) && range.line > 0 && range.column > 0 {
                diagnostic.line = Some(range.line);
                diagnostic.column = Some(range.column);
            }
        }
        diagnostic
    }
    pub(super) fn source_diagnostics(
        &self,
        source: &SchemaDeclarationNode,
        diagnostics: Vec<Diagnostic>,
    ) -> Vec<Diagnostic> {
        let Some(tree) = self
            .scopes
            .iter()
            .find(|scope| Arc::ptr_eq(scope.tree.ast_owner(), source.document()))
            .map(|scope| &scope.tree)
        else {
            return diagnostics;
        };
        let provenance = match source.node() {
            CemAstNode::Reference { source, .. }
            | CemAstNode::Element { source, .. }
            | CemAstNode::Attribute { source, .. } => source,
            _ => return diagnostics,
        };
        diagnostics
            .into_iter()
            .map(|mut diagnostic| {
                // A native capability can report diagnostics from another original
                // node/document. Preserve that attribution and its coordinates.
                if diagnostic.node.is_some()
                    || diagnostic
                        .uri
                        .as_deref()
                        .is_some_and(|uri| uri != tree.source_uri())
                {
                    return diagnostic;
                }
                if matches!(source.node(), CemAstNode::Attribute { .. }) {
                    if diagnostic.source_map.as_ref().is_some_and(|stack| {
                        !stack.frames.iter().any(|frame| {
                            frame.source_id == SourceId(0)
                                && matches!(
                                    frame.transform,
                                    TransformKind::Query | TransformKind::QueryStep
                                )
                        })
                    }) {
                        return diagnostic;
                    }
                    // CEM literal attributes retain their name's source map,
                    // not a decoded-value character map. Anchor to that real source
                    // handle; keep query frames without inventing value coordinates.
                    let query = diagnostic.source_map.take().unwrap_or_default();
                    let mut stack = provenance.clone();
                    stack.frames.extend(query.frames);
                    diagnostic.source_map = Some(stack);
                    diagnostic.node = Some(source.identity());
                    diagnostic.uri = Some(tree.source_uri().into());
                    diagnostic.byte_offset =
                        provenance.origin().and_then(|frame| match &frame.span {
                            FrameSpan::Single(range) => Some(range.start),
                            FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start),
                        });
                    let coordinate = diagnostic
                        .byte_offset
                        .and_then(|offset| tree.source_byte_coordinate(offset));
                    diagnostic.line = coordinate.map(|coordinate| coordinate.line);
                    diagnostic.column = coordinate.map(|coordinate| coordinate.column);
                    return diagnostic;
                }
                let local = diagnostic.source_map.as_ref().and_then(|stack| {
                    stack.frames.iter().rev().find(|frame| {
                        frame.source_id == SourceId(0)
                            && matches!(
                                frame.transform,
                                TransformKind::Query | TransformKind::QueryStep
                            )
                    })
                });
                let Some(local) = local else {
                    return diagnostic;
                };
                let FrameSpan::Single(query) = local.span else {
                    return diagnostic;
                };
                let mut mapped = map_expression_range(provenance, query);
                // Parsers can put a one-byte diagnostic at EOF. Its location is a
                // point at the last payload end, not a byte of the host delimiter.
                if mapped.is_empty() && query.len > 0 {
                    mapped = map_expression_range(provenance, ByteRange::new(query.start, 0));
                }
                let query_stack = diagnostic.source_map.take().unwrap();
                let mut stack = provenance.clone();
                for (source_id, range) in &mapped {
                    stack.push(SourceMapFrame {
                        source_id: *source_id,
                        span: FrameSpan::Single(*range),
                        transform: TransformKind::ExpressionEmbedding { expression: query },
                    });
                }
                stack.frames.extend(query_stack.frames);
                diagnostic.source_map = Some(stack);
                diagnostic.uri = Some(tree.source_uri().into());
                diagnostic.node = Some(source.identity());
                diagnostic.byte_offset = mapped.first().map(|(_, range)| range.start);
                let coordinate = diagnostic
                    .byte_offset
                    .and_then(|offset| tree.source_byte_coordinate(offset));
                diagnostic.line = coordinate.map(|coordinate| coordinate.line);
                diagnostic.column = coordinate.map(|coordinate| coordinate.column);
                diagnostic
            })
            .collect()
    }
}
