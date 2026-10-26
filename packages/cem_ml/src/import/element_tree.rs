//! Literal element-tree import with original event identity as compatibility metadata.
//! Consumers navigate the retained CEM tree, never these parser events.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementSourceHandle {
    Html { event_index: usize },
    Xml { event_index: usize },
}

#[derive(Debug)]
pub struct RetainedElementImport {
    pub tree: Arc<RetainedCemTree>,
    pub elements: Vec<(AstNodeId, ElementSourceHandle, usize)>,
    /// Imported document name policy; event handles are only public identity metadata.
    pub attribute_names_ascii_case_insensitive: bool,
}

/// Preserve the original lifecycle owner and its byte coordinates. This source
/// view admits only HTML/XML families; it grants no loading or execution.
pub fn retain_element_lifecycle(
    native: Arc<LoadedInputAstStream>,
) -> Result<RetainedElementImport, String> {
    let attribute_names_ascii_case_insensitive =
        matches!(native.as_ref(), LoadedInputAstStream::HtmlDocument(_));
    let (ast, mut semantics, elements, uri, format, length) = match native.as_ref() {
        LoadedInputAstStream::HtmlDocument(doc) => {
            check_bytes(doc.source.byte_length)?;
            html::validate(doc, MAX_DOCUMENT_VALUES)?;
            let (ast, semantics, event_nodes) = html::project_tracked(doc)?;
            let elements = doc
                .events
                .iter()
                .enumerate()
                .filter(|(_, event)| event.kind == html_parser::HtmlEventKind::StartElement)
                .map(|(index, event)| {
                    Ok((
                        event_nodes[index].ok_or("Missing imported HTML element")?,
                        ElementSourceHandle::Html {
                            event_index: event.index,
                        },
                        event.depth,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            (
                ast,
                semantics,
                elements,
                doc.source.uri.clone(),
                doc.source.content_type.clone(),
                doc.source.byte_length,
            )
        }
        owner => {
            let doc = match owner {
                LoadedInputAstStream::XmlDocument(doc) => doc,
                LoadedInputAstStream::XhtmlDocument(doc) => &doc.xml_document,
                LoadedInputAstStream::SvgDocument(doc) => &doc.xml_document,
                LoadedInputAstStream::MathMlDocument(doc) => &doc.xml_document,
                LoadedInputAstStream::XsltStylesheet(doc) => &doc.xml_document,
                LoadedInputAstStream::RelaxNgDocument(doc) => doc
                    .xml_document
                    .as_ref()
                    .ok_or("Relax NG compact syntax does not expose an element-tree view")?,
                _ => {
                    return Err(
                        "Lifecycle input does not expose an HTML/XML-family element-tree view"
                            .into(),
                    )
                }
            };
            check_bytes(doc.source.byte_length)?;
            validate_data_ast(owner, MAX_DOCUMENT_VALUES).map_err(|e| e.to_string())?;
            // Also bounds XSLT and Relax NG wrappers while retaining their outer owner.
            validate_xml_limits(doc, MAX_DOCUMENT_VALUES).map_err(|e| e.to_string())?;
            let imported = import_xml_ast_with_element_policy(doc, &mut None, true)?;
            let elements = doc
                .events
                .iter()
                .enumerate()
                .filter(|(_, event)| {
                    matches!(
                        event.kind,
                        xml::XmlEventKind::StartElement | xml::XmlEventKind::EmptyElement
                    )
                })
                .map(|(index, event)| {
                    Ok((
                        imported.event_nodes[index].ok_or("Missing imported XML element")?,
                        ElementSourceHandle::Xml {
                            event_index: event.index,
                        },
                        event.depth,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            (
                imported.ast,
                imported.semantics,
                elements,
                doc.source.uri.clone(),
                doc.source.media_type.clone(),
                doc.source.byte_length,
            )
        }
    };
    semantics
        .sources
        .entry(0)
        .or_insert_with(|| SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(0, length as u32)),
                transform: TransformKind::ContentTypeTransform {
                    content_type: format,
                },
            }],
        });
    semantics.ranges.entry(0).or_insert(CemTreeRange {
        line: 1,
        column: 1,
        offset: 0,
        length: length as u64,
    });
    let tree = RetainedCemTree::new(ast, uri, "", semantics, Some(native))?;
    Ok(RetainedElementImport {
        tree,
        elements,
        attribute_names_ascii_case_insensitive,
    })
}

fn check_bytes(length: usize) -> Result<(), String> {
    if length > MAX_DOCUMENT_BYTES {
        Err("Source exceeds the 16 MiB document import limit.".into())
    } else {
        Ok(())
    }
}
