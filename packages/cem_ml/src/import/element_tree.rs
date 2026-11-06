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

/// Import an already-owned HTML source into the common literal element tree.
/// Source acquisition/validation policy remains with the native importer.
pub fn retain_html_elements(
    owner: Arc<html_parser::HtmlDocumentAst>,
    metadata: crate::parser::tree::CemDocumentMetadata,
) -> Result<RetainedElementImport, String> {
    check_bytes(owner.source.byte_length)?;
    html::validate(&owner, MAX_DOCUMENT_VALUES)?;
    let (ast, mut semantics, event_nodes) = html::project_tracked(&owner)?;
    let elements = owner
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.kind == html_parser::HtmlEventKind::StartElement)
        .map(|(index, event)| {
            Ok((
                event_nodes[index].ok_or("Missing imported HTML element")?,
                ElementSourceHandle::Html { event_index: index },
                event.depth,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    semantics.source_fingerprint = Some(source_fingerprint(
        &owner.source.uri,
        owner.source_text.as_bytes(),
        &["html-element-owner/1"],
    ));
    semantics.document_metadata = Some(metadata);
    let tree = RetainedCemTree::new(
        ast,
        owner.source.uri.clone(),
        &owner.source_text,
        semantics,
        Some(owner.clone()),
    )?;
    Ok(RetainedElementImport {
        tree,
        elements,
        attribute_names_ascii_case_insensitive: true,
    })
}

pub struct InlineCemTemplateSource {
    owner: Arc<html_parser::HtmlDocumentAst>,
    text: String,
    segments: Vec<(ByteRange, html_parser::HtmlSourceRange, bool)>,
}
impl InlineCemTemplateSource {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn source_map_for_range(&self, range: ByteRange) -> Option<SourceMapStack> {
        if range.start > self.text.len() as u64
            || u64::from(range.len) > self.text.len() as u64 - range.start
        {
            return None;
        }
        let first = self
            .segments
            .partition_point(|(decoded, _, _)| decoded.end() <= range.start);
        let mut result = SourceMapStack::default();
        for (decoded, original, identical) in self.segments[first..]
            .iter()
            .take_while(|(decoded, _, _)| decoded.start < range.end())
        {
            let mut original = *original;
            if *identical {
                let start = decoded.start.max(range.start);
                let end = decoded.end().min(range.end());
                original.start.byte_offset += start - decoded.start;
                original.byte_length = end - start;
            }
            // Entity/line-ending changes map conservatively to the authored run.
            if let Some(map) = self.owner.source_map_for_range(original) {
                for frame in map.frames {
                    if !result.frames.contains(&frame) {
                        result.frames.push(frame);
                    }
                }
            }
        }
        (!result.frames.is_empty()).then_some(result)
    }
}

impl RetainedElementImport {
    /// Check the native decoder's materializable HTML text profile before
    /// deriving a whole-page AST. Earlier capture/import remains lossless.
    pub fn validate_html_text_projection(&self) -> Result<(), String> {
        use html_parser::HtmlEventKind;
        let owner = self
            .tree
            .native_owner()
            .and_then(|owner| owner.downcast_ref::<html_parser::HtmlDocumentAst>())
            .ok_or("Projection requires an original HTML owner")?;
        for event in &owner.events {
            if matches!(event.kind, HtmlEventKind::Text | HtmlEventKind::Rcdata) {
                require_projectable_references(&event.lexeme)?;
            }
            for attr in &event.attributes {
                if attr.lexeme.contains('\r') {
                    return Err("Attribute carriage-return normalization is unsupported".into());
                }
                require_projectable_references(&attr.lexeme)?;
            }
        }
        Ok(())
    }

    /// Explicit HTML-to-CEM source handoff; no serialization or HTML reparse.
    pub fn inline_cem_template_source(
        &self,
        node: AstNodeId,
    ) -> Result<InlineCemTemplateSource, String> {
        use html_parser::{HtmlEventKind, HtmlNamespace};
        let owner = self
            .tree
            .native_owner()
            .ok_or("Missing original HTML owner")?
            .clone()
            .downcast::<html_parser::HtmlDocumentAst>()
            .map_err(|_| "Handoff requires an original HTML owner")?;
        let start = self
            .elements
            .iter()
            .find_map(|(id, source, _)| match source {
                ElementSourceHandle::Html { event_index } if *id == node => Some(*event_index),
                _ => None,
            })
            .ok_or("Missing HTML template source handle")?;
        let template = owner
            .events
            .get(start)
            .ok_or("Invalid HTML source handle")?;
        if template.namespace != HtmlNamespace::Html
            || template.local_name.as_deref() != Some("template")
            || !template.attributes.iter().any(|attr| {
                !attr.duplicate
                    && attr.local_name == "type"
                    && attr.value.as_deref() == Some("text/cem-ml")
            })
        {
            return Err("Expected an explicit text/cem-ml template".into());
        }
        let mut text = String::new();
        let mut segments = Vec::new();
        let mut closed = false;
        for event in owner.events.iter().skip(start + 1) {
            if event.kind == HtmlEventKind::EndElement
                && event.depth == template.depth
                && event.local_name.as_deref() == Some("template")
            {
                closed = true;
                break;
            }
            match event.kind {
                HtmlEventKind::Text | HtmlEventKind::RawText | HtmlEventKind::Rcdata => {
                    // The native decoder's current profile supports XML named
                    // references and numeric references. Never compile a silent
                    // fallback spelling where browser text would be different.
                    if event.kind != HtmlEventKind::RawText
                        && quick_xml::escape::unescape(&event.lexeme).is_err()
                    {
                        return Err("Inline CEM source uses unsupported character references; escape literal ampersands".into());
                    }
                    let value = html::decoded_text(event);
                    if text.len().saturating_add(value.len()) > MAX_DOCUMENT_BYTES {
                        return Err("Inline template source limit".into());
                    }
                    let decoded = ByteRange::new(text.len() as u64, value.len() as u32);
                    segments.push((decoded, event.source_range, value == event.lexeme));
                    text.push_str(&value);
                }
                HtmlEventKind::Comment => {}
                _ => return Err("Inline CEM source cannot contain HTML child elements".into()),
            }
        }
        if !closed {
            return Err("Unclosed HTML source template".into());
        }
        Ok(InlineCemTemplateSource {
            owner,
            text,
            segments,
        })
    }
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

#[cfg(test)]
#[path = "ssr_element_tests.rs"]
mod ssr_element_tests;

// This belongs at the syntax import boundary. XML and HTML numeric character
// references differ for NUL and C1 controls; never silently materialize them.
fn require_projectable_references(text: &str) -> Result<(), String> {
    quick_xml::escape::unescape(text)
        .map_err(|_| "Unsupported HTML character reference".to_string())?;
    for tail in text.split("&#").skip(1) {
        let reference = tail.split(';').next().unwrap_or_default();
        let value = if let Some(hex) = reference
            .strip_prefix('x')
            .or_else(|| reference.strip_prefix('X'))
        {
            u32::from_str_radix(hex, 16)
        } else {
            reference.parse::<u32>()
        }
        .map_err(|_| "Invalid HTML numeric reference".to_string())?;
        if value == 0 || (0x80..=0x9f).contains(&value) || char::from_u32(value).is_none() {
            return Err("HTML numeric reference requires an unsupported replacement".into());
        }
    }
    Ok(())
}
