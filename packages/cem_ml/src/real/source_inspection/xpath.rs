//! Inert source tooling over the lifecycle-owned XPath syntax and tokens.
use super::*;
use crate::projection::{CemTreeAstNode, CemTreeAstStream};
use crate::validation::xpath::{XPathExpressionAst, XPathSourceRange, XPathSyntaxEventKind};

pub(super) fn load(input: &EngineInput, context: &EngineContext) -> Option<SourceInspection> {
    let identity = effective_input_identity(input, context);
    let mime = identity.content_type.as_deref().map(content_type_essence);
    if identity.schema.as_deref().map(str::trim) != Some(XPATH_SCHEMA_URI)
        && !matches!(mime.as_deref(), Some(XPATH_CONTENT_TYPE | "text/xpath"))
    {
        return None;
    }
    let loaded = load_document_input(input, context);
    let mut diagnostics = loaded.diagnostics;
    let (ast, events, offsets) = match loaded.ast_stream {
        Some(LoadedInputAstStream::XPathExpression(expression)) => {
            let offsets = expression
                .tokens
                .iter()
                .map(|token| {
                    ByteRange::new(
                        token.source_range.start.byte_offset,
                        u32::try_from(token.source_range.byte_length).unwrap_or(u32::MAX),
                    )
                })
                .collect();
            if diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
                (None, None, offsets)
            } else {
                (
                    Some(project(&expression, true)),
                    Some(project(&expression, false)),
                    offsets,
                )
            }
        }
        _ => {
            if !diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
                diagnostics.push(Diagnostic {
                    uri: Some(input.uri.clone()),
                    code: "cem.source.identity_unsupported".into(),
                    severity: Severity::Error,
                    message: "XPath source tooling requires its registered source identity".into(),
                    ..Default::default()
                });
            }
            (None, None, Vec::new())
        }
    };
    Some(SourceInspection {
        identity,
        ast,
        events,
        offsets,
        diagnostics,
    })
}

fn project(expression: &XPathExpressionAst, ast: bool) -> CemTreeAstStream {
    let uri = &expression.source.uri;
    let map = |range: XPathSourceRange| {
        range
            .source_map(
                expression.attachment.source_id(),
                &expression.source.media_type,
            )
            .with_source_uri(uri)
    };
    let range = XPathSourceRange::new(1, 1, 0, expression.source.byte_length as u64);
    let source = map(range);
    let mut nodes = Vec::new();
    if ast {
        nodes.push(projection::projection_node(
            "xpath-expression",
            "xpath-expression",
            None,
            0,
            None,
            None,
            uri,
            Some((0, range.byte_length)),
            source.clone(),
        ));
        // Syntax events come from the parser's existing typed tree. Parent links
        // describe that tree; lexical tokens remain a separate ordered view.
        let mut parents = vec!["xpath-expression".to_owned()];
        if let Some(syntax) = &expression.syntax_ast {
            for event in &syntax.events {
                if event.kind == XPathSyntaxEventKind::EndNode {
                    parents.pop();
                    continue;
                }
                let id = format!("syntax-{}", event.index);
                nodes.push(projection::projection_node(
                    &id,
                    event.node_kind.as_str(),
                    parents.last().map(String::as_str),
                    event.index,
                    None,
                    None,
                    uri,
                    Some((
                        event.source_range.start.byte_offset,
                        event.source_range.byte_length,
                    )),
                    map(event.source_range),
                ));
                parents.push(id);
            }
        }
        for token in &expression.tokens {
            let source = map(token.source_range);
            let mut node = projection::projection_node(
                &format!("token-{}", token.index),
                token.kind.as_str(),
                Some("xpath-expression"),
                token.index,
                None,
                Some(&token.lexeme),
                uri,
                Some((
                    token.source_range.start.byte_offset,
                    token.source_range.byte_length,
                )),
                source.clone(),
            );
            if let CemTreeAstNode::Element { attributes, .. } = &mut node {
                attributes.push(projection::projection_attribute(
                    "role",
                    token.kind.role(),
                    &source,
                ));
                attributes.push(projection::projection_attribute(
                    "depth",
                    token.depth.to_string(),
                    &source,
                ));
            }
            nodes.push(node);
        }
    } else {
        for event in &expression.events {
            let value = event
                .token_index
                .and_then(|index| expression.tokens.get(index))
                .map(|token| token.lexeme.as_str())
                .unwrap_or("");
            let source = map(event.source_range);
            let mut attributes = native_projection::event_attributes(
                nodes.len(),
                event.kind.as_str(),
                value,
                uri,
                event.source_range.start.byte_offset,
                event.source_range.byte_length,
                &source,
            );
            attributes.push(projection::projection_attribute(
                "event-domain",
                "lexical",
                &source,
            ));
            attributes.push(projection::projection_attribute(
                "native-sequence",
                event.index.to_string(),
                &source,
            ));
            attributes.push(projection::projection_attribute(
                "depth",
                event.depth.to_string(),
                &source,
            ));
            if let Some(index) = event.token_index {
                attributes.push(projection::projection_attribute(
                    "token-index",
                    index.to_string(),
                    &source,
                ));
            }
            if let Some(token) = event
                .token_index
                .and_then(|index| expression.tokens.get(index))
            {
                attributes.push(projection::projection_attribute(
                    "token-kind",
                    token.kind.as_str(),
                    &source,
                ));
            }
            nodes.push(native_projection::element(
                "event",
                attributes,
                Vec::new(),
                source,
            ));
        }
        if let Some(syntax) = &expression.syntax_ast {
            for event in &syntax.events {
                let source = map(event.source_range);
                let mut attributes = native_projection::event_attributes(
                    nodes.len(),
                    event.kind.as_str(),
                    "",
                    uri,
                    event.source_range.start.byte_offset,
                    event.source_range.byte_length,
                    &source,
                );
                attributes.push(projection::projection_attribute(
                    "event-domain",
                    "syntax",
                    &source,
                ));
                attributes.push(projection::projection_attribute(
                    "native-sequence",
                    event.index.to_string(),
                    &source,
                ));
                attributes.push(projection::projection_attribute(
                    "node-kind",
                    event.node_kind.as_str(),
                    &source,
                ));
                attributes.push(projection::projection_attribute(
                    "depth",
                    event.depth.to_string(),
                    &source,
                ));
                nodes.push(native_projection::element(
                    "event",
                    attributes,
                    Vec::new(),
                    source,
                ));
            }
        }
    }
    native_projection::wrap(
        if ast { "ast" } else { "event-stream" },
        XPATH_SCHEMA_URI,
        &expression.source.media_type,
        uri,
        nodes,
        source,
        ast,
    )
}
