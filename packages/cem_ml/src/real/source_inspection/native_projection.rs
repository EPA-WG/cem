//! Presentation reads existing native owners; it never tokenizes source again.
use super::*;
use crate::{
    projection::{CemTreeAstAttribute, CemTreeAstNode, CemTreeAstStream},
    source_map::{FrameSpan, SourceMapFrame, TransformKind},
    validation::{relax_ng::RelaxNgDocumentAst, xml::XmlDocumentAst},
};

pub(super) fn project(
    native: LoadedInputAstStream,
    projection: ParseProjection,
) -> Result<Option<CemTreeAstStream>, String> {
    if !matches!(projection, ParseProjection::Ast | ParseProjection::Events) {
        return Ok(None);
    }
    let ast = projection == ParseProjection::Ast;
    Ok(Some(match native {
        LoadedInputAstStream::CssDocument(css) if ast => {
            projection::css_ast_cem_presentation_stream(&css)
        }
        LoadedInputAstStream::CssDocument(css) => {
            projection::css_events_cem_presentation_stream(&css)
        }
        LoadedInputAstStream::XsltStylesheet(xslt) => xml(xslt.xml_document, XSLT_SCHEMA_URI, ast)?,
        LoadedInputAstStream::RelaxNgDocument(mut rng) => match rng.xml_document.take() {
            Some(document) => xml(document, RELAX_NG_SCHEMA_URI, ast)?,
            None => compact_rng(&rng, ast),
        },
        _ => return Ok(None),
    }))
}

fn xml(document: XmlDocumentAst, schema: &str, ast: bool) -> Result<CemTreeAstStream, String> {
    if ast {
        let content_type = document.source.media_type.clone();
        let owner =
            crate::import::retain_xml(Arc::new(LoadedInputAstStream::XmlDocument(document)))?;
        return Ok(projection::cem_tree_inspection_with_identity(
            owner.tree,
            schema,
            &content_type,
        ));
    }
    let events = document
        .events
        .iter()
        .map(|event| {
            let source = event
                .source_range
                .source_map()
                .with_source_uri(&document.source.uri);
            let mut attributes = event_attributes(
                event.index,
                event.kind.as_str(),
                &event.lexeme,
                &document.source.uri,
                event.source_range.start.byte_offset,
                event.source_range.byte_length,
                &source,
            );
            if let Some(name) = event.qualified_name.as_deref() {
                attributes.push(projection::projection_attribute("name", name, &source));
            }
            if let Some(namespace) = event.namespace_uri.as_deref() {
                attributes.push(projection::projection_attribute(
                    "namespace",
                    namespace,
                    &source,
                ));
            }
            attributes.push(projection::projection_attribute(
                "depth",
                event.depth.to_string(),
                &source,
            ));
            element("event", attributes, Vec::new(), source)
        })
        .collect();
    let source = document
        .events
        .first()
        .map(|event| {
            event
                .source_range
                .source_map()
                .with_source_uri(&document.source.uri)
        })
        .unwrap_or_default();
    Ok(wrap(
        "event-stream",
        schema,
        &document.source.media_type,
        &document.source.uri,
        events,
        source,
        false,
    ))
}

fn compact_rng(document: &RelaxNgDocumentAst, ast: bool) -> CemTreeAstStream {
    let map = |offset, length| {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    offset,
                    u32::try_from(length).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: document.source.media_type.clone(),
                },
            }],
        }
        .with_source_uri(&document.source.uri)
    };
    let source = map(0, document.source.byte_length as u64);
    let mut nodes = Vec::new();
    if ast {
        nodes.push(projection::projection_node(
            "relax-ng-document",
            "relax-ng-compact-document",
            None,
            0,
            None,
            None,
            &document.source.uri,
            Some((0, document.source.byte_length as u64)),
            source.clone(),
        ));
    }
    for token in &document.compact_tokens {
        let range = token.source_range;
        let source = map(range.start.byte_offset, range.byte_length);
        let node = if ast {
            let mut node = projection::projection_node(
                &format!("token-{}", token.index),
                token.kind.as_str(),
                Some("relax-ng-document"),
                token.index,
                None,
                Some(&token.lexeme),
                &document.source.uri,
                Some((range.start.byte_offset, range.byte_length)),
                source.clone(),
            );
            if let CemTreeAstNode::Element { attributes, .. } = &mut node {
                attributes.push(projection::projection_attribute(
                    "depth",
                    token.depth.to_string(),
                    &source,
                ));
                attributes.push(projection::projection_attribute(
                    "role",
                    token.kind.role(),
                    &source,
                ));
            }
            node
        } else {
            element(
                "event",
                event_attributes(
                    token.index,
                    token.kind.as_str(),
                    &token.lexeme,
                    &document.source.uri,
                    range.start.byte_offset,
                    range.byte_length,
                    &source,
                ),
                Vec::new(),
                source,
            )
        };
        nodes.push(node);
    }
    wrap(
        if ast { "ast" } else { "event-stream" },
        RELAX_NG_SCHEMA_URI,
        &document.source.media_type,
        &document.source.uri,
        nodes,
        source,
        ast,
    )
}

fn wrap(
    name: &str,
    schema: &str,
    content_type: &str,
    uri: &str,
    nodes: Vec<CemTreeAstNode>,
    source: SourceMapStack,
    ast: bool,
) -> CemTreeAstStream {
    let root = element(
        name,
        vec![
            projection::projection_attribute("schema", schema, &source),
            projection::projection_attribute("content-type", content_type, &source),
            projection::projection_attribute("source-id", uri, &source),
            projection::projection_attribute("hash-scheme", "blake3", &source),
        ],
        nodes,
        source.clone(),
    );
    projection::projection_vocabulary_document(
        if ast { "cemast" } else { "cemevents" },
        if ast {
            CEM_AST_PROJECTION_SCHEMA_URI
        } else {
            CEM_EVENTS_PROJECTION_SCHEMA_URI
        },
        root,
        source,
    )
}

fn event_attributes(
    sequence: usize,
    kind: &str,
    value: &str,
    uri: &str,
    offset: u64,
    length: u64,
    source: &SourceMapStack,
) -> Vec<CemTreeAstAttribute> {
    vec![
        projection::projection_attribute("sequence", sequence.to_string(), source),
        projection::projection_attribute("kind", kind, source),
        projection::projection_attribute("value", value, source),
        projection::projection_attribute("source-id", uri, source),
        projection::projection_attribute("byte-offset", offset.to_string(), source),
        projection::projection_attribute("byte-length", length.to_string(), source),
    ]
}

fn element(
    name: &str,
    attributes: Vec<CemTreeAstAttribute>,
    children: Vec<CemTreeAstNode>,
    source: SourceMapStack,
) -> CemTreeAstNode {
    CemTreeAstNode::Element {
        name: name.into(),
        attributes,
        children,
        source,
    }
}
