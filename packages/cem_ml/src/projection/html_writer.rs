//! DOM-to-writer projection over the retained semantic tree. This is a typed
//! projection, not an HTML serializer or another source parser.
use super::*;
use crate::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::tree::{CemTreeNodeKind, RetainedCemTree},
    validation::html::{HtmlDocumentAst, HtmlEventKind},
};
const HTML: &str = "http://www.w3.org/1999/xhtml";
const SVG: &str = "http://www.w3.org/2000/svg";
const MATH: &str = "http://www.w3.org/1998/Math/MathML";
const XMLNS: &str = "http://www.w3.org/2000/xmlns/";
fn name(
    n: &ExpandedName,
    aliases: &BTreeMap<String, String>,
    attribute: bool,
) -> Result<String, String> {
    if n.namespace_uri == XMLNS {
        return Ok(if n.local_name == "xmlns" {
            "xmlns".into()
        } else {
            format!("xmlns:{}", n.local_name)
        });
    }
    if n.namespace_uri.is_empty()
        || (!attribute && matches!(n.namespace_uri.as_str(), HTML | SVG | MATH))
    {
        return Ok(n.local_name.clone());
    }
    if n.namespace_uri == "http://www.w3.org/XML/1998/namespace" {
        return Ok(format!("xml:{}", n.local_name));
    }
    if n.namespace_uri == "http://www.w3.org/1999/xlink" {
        return Ok(format!("xlink:{}", n.local_name));
    }
    let prefix = aliases
        .iter()
        .find(|(p, u)| **u == n.namespace_uri && (!attribute || !p.is_empty()))
        .map(|(p, _)| p)
        .ok_or_else(|| {
            format!(
                "DOM writer has no lexical namespace binding for {}",
                n.namespace_uri
            )
        })?;
    Ok(if prefix.is_empty() {
        n.local_name.clone()
    } else {
        format!("{prefix}:{}", n.local_name)
    })
}
fn node(
    tree: &RetainedCemTree,
    id: AstNodeId,
    aliases: &BTreeMap<String, String>,
    depth: usize,
    c: &OperationControl,
    output_attributes: &BTreeMap<AstNodeId, CemTreeAstAttribute>,
) -> Result<CemTreeAstNode, String> {
    c.check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.code().to_owned())?;
    if depth > 128 {
        return Err("DOM writer projection exceeds its depth bound".into());
    }
    let n = tree
        .node(id)
        .ok_or("DOM writer references an absent node")?;
    let source = n.source.clone();
    Ok(match n.kind {
        CemTreeNodeKind::Element => {
            let mut aliases = aliases.clone();
            for aid in &n.attributes {
                let a = tree
                    .node(*aid)
                    .ok_or("DOM writer references an absent attribute")?;
                let an = a.name.as_ref().ok_or("DOM attribute has no name")?;
                if an.namespace_uri == XMLNS {
                    aliases.insert(
                        if an.local_name == "xmlns" {
                            "".into()
                        } else {
                            an.local_name.clone()
                        },
                        a.value.clone(),
                    );
                } else if an.namespace_uri.is_empty() && an.local_name.starts_with("xmlns:") {
                    aliases.insert(an.local_name[6..].into(), a.value.clone());
                }
            }
            let mut attributes: Vec<_> = n
                .attributes
                .iter()
                .map(|id| {
                    let a = tree
                        .node(*id)
                        .ok_or("DOM writer references an absent attribute")?;
                    Ok(CemTreeAstAttribute {
                        name: name(
                            a.name.as_ref().ok_or("DOM attribute has no name")?,
                            &aliases,
                            true,
                        )?,
                        value: Some(a.value.clone()),
                        value_nodes: vec![],
                        source: a.source.clone(),
                    })
                })
                .collect::<Result<_, String>>()?;
            if let Some(extra) = output_attributes.get(&id) {
                if attributes.iter().any(|a| a.name == extra.name) {
                    return Err("DOM writer output attribute duplicates a retained name".into());
                }
                attributes.push(extra.clone());
            }
            let expanded = n.name.as_ref().ok_or("DOM element has no name")?;
            let children = if expanded.namespace_uri == HTML && expanded.local_name == "style" {
                style_text(tree, &n.children, c)?
            } else {
                n.children
                    .iter()
                    .map(|id| node(tree, *id, &aliases, depth + 1, c, output_attributes))
                    .collect::<Result<_, _>>()?
            };
            CemTreeAstNode::Element {
                name: name(expanded, &aliases, false)?,
                attributes,
                children,
                source,
            }
        }
        CemTreeNodeKind::Document => CemTreeAstNode::Document {
            children: n
                .children
                .iter()
                .map(|id| node(tree, *id, aliases, depth + 1, c, output_attributes))
                .collect::<Result<_, _>>()?,
            source,
        },
        CemTreeNodeKind::Text => {
            if matches!(tree.ast().get(id), Some(CemAstNode::RawText { .. })) {
                CemTreeAstNode::RawText {
                    data: n.value.clone(),
                    source,
                }
            } else {
                CemTreeAstNode::Text {
                    value: n.value.clone(),
                    source,
                }
            }
        }
        CemTreeNodeKind::Comment => CemTreeAstNode::Comment {
            data: n.value.clone(),
            source,
        },
        _ => {
            return Err(
                "DOM writer requires materialized document, element, text or comment nodes".into(),
            )
        }
    })
}

fn style_text(
    tree: &RetainedCemTree,
    children: &[AstNodeId],
    c: &OperationControl,
) -> Result<Vec<CemTreeAstNode>, String> {
    let mut text = String::new();
    let mut projected = Vec::with_capacity(children.len());
    for id in children {
        c.check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())?;
        let child = tree
            .node(*id)
            .filter(|node| node.kind == CemTreeNodeKind::Text)
            .ok_or("HTML style requires literal text children")?;
        text.push_str(&child.value);
        projected.push(CemTreeAstNode::RawText {
            data: child.value.clone(),
            source: child.source.clone(),
        });
    }
    // HTML's raw text tokenizer does not decode character references. Check
    // the whole body because an end tag can span adjacent native text nodes.
    if text.contains('\0') || text.to_ascii_lowercase().contains("</style") {
        return Err("Unsafe HTML style raw text".into());
    }
    Ok(projected)
}

pub fn stream(
    tree: Arc<RetainedCemTree>,
    original: &HtmlDocumentAst,
    c: &OperationControl,
) -> Result<CemTreeAstStream, String> {
    stream_with_output_attributes(tree, original, c, &BTreeMap::new())
}

/// Typed export annotations do not change the retained owner or its native
/// identity. The exporter is responsible for admitting each annotated node.
pub fn stream_with_output_attributes(
    tree: Arc<RetainedCemTree>,
    original: &HtmlDocumentAst,
    c: &OperationControl,
    output_attributes: &BTreeMap<AstNodeId, CemTreeAstAttribute>,
) -> Result<CemTreeAstStream, String> {
    if output_attributes.len() > 64
        || output_attributes.keys().any(|id| {
            tree.node(*id)
                .is_none_or(|n| n.kind != CemTreeNodeKind::Element)
        })
    {
        return Err("DOM writer output attribute owner is invalid".into());
    }
    let root = tree.node(0).ok_or("DOM writer has no document root")?;
    if root.kind != CemTreeNodeKind::Document {
        return Err("DOM writer requires a document root".into());
    }
    let mut nodes = root
        .children
        .iter()
        .map(|id| node(&tree, *id, &BTreeMap::new(), 0, c, output_attributes))
        .collect::<Result<Vec<_>, _>>()?;
    for event in original
        .events
        .iter()
        .filter(|e| e.kind == HtmlEventKind::Doctype)
    {
        let at = root
            .children
            .iter()
            .position(|id| {
                tree.node(*id)
                    .is_some_and(|n| n.range.offset > event.source_range.start.byte_offset)
            })
            .unwrap_or(nodes.len());
        nodes.insert(
            at,
            CemTreeAstNode::RawText {
                data: event.lexeme.clone(),
                source: original
                    .source_map_for_range(event.source_range)
                    .unwrap_or_default(),
            },
        );
    }
    let mut stream = CemTreeAstStream::new(nodes);
    stream.source_owner = Some(tree);
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derived_html_style_text_is_literal_and_cannot_close_its_owner() {
        let capture = |parts: &[&str]| {
            let mut capture = crate::import::dom::DomCapture::new(
                "cem-dom://writer/style",
                "",
                "",
                Default::default(),
            )
            .unwrap();
            capture.push(0, "doc", "document", "", "", "").unwrap();
            let style = capture
                .push(0, "style", "element", HTML, "style", "")
                .unwrap();
            for (index, part) in parts.iter().enumerate() {
                capture
                    .push(style, &format!("text-{index}"), "text", "", "", part)
                    .unwrap();
            }
            let tree = capture.finish().unwrap();
            node(
                &tree,
                style,
                &BTreeMap::new(),
                0,
                &OperationControl::default(),
                &BTreeMap::new(),
            )
        };
        let CemTreeAstNode::Element { children, .. } = capture(&["a > b {content:'<&'}"]).unwrap()
        else {
            panic!("style")
        };
        assert!(
            matches!(&children[0], CemTreeAstNode::RawText { data, .. } if data == "a > b {content:'<&'}")
        );
        assert!(capture(&["a {content:'</st", "yle>'}"]).is_err());
    }
    #[test]
    fn dom_writer_projection_preserves_html_names_doctype_raw_text_and_source_owner() {
        use crate::{
            import::element_tree::retain_html_elements,
            parser::tree::CemDocumentMetadata,
            validation::html::{html_document_ast_from_source_bytes, HtmlSourceValidationRequest},
        };
        let bytes=br##"<!doctype html><html><head><title>DOM</title><script type="text/plain">if (a < b) { c = '&'; }</script></head><body><svg><use xlink:href="#shape"/></svg><textarea>&lt;literal&gt;</textarea></body></html>"##;
        let source = Arc::new(
            html_document_ast_from_source_bytes(HtmlSourceValidationRequest {
                bytes,
                source_uri: "file:///dom.html",
                content_type: Some("text/html"),
            })
            .0
            .unwrap(),
        );
        let input = retain_html_elements(source.clone(), CemDocumentMetadata::default()).unwrap();
        let c = OperationControl::default();
        let stream = stream(input.tree.clone(), &source, &c).unwrap();
        assert!(Arc::ptr_eq(stream.source_owner().unwrap(), &input.tree));
        assert!(
            matches!(&stream.as_nodes()[0],CemTreeAstNode::RawText{data,..} if data=="<!doctype html>")
        );
        let CemTreeAstNode::Element {
            name,
            children,
            source: root_map,
            ..
        } = &stream.as_nodes()[1]
        else {
            panic!("HTML root")
        };
        assert_eq!(name, "html");
        assert!(!root_map.frames.is_empty());
        let CemTreeAstNode::Element { children: head, .. } = &children[0] else {
            panic!("head")
        };
        let CemTreeAstNode::Element {
            children: script, ..
        } = &head[1]
        else {
            panic!("script")
        };
        assert!(matches!(&script[0],CemTreeAstNode::RawText{data,..} if data.contains("a < b")));
        let CemTreeAstNode::Element { children: body, .. } = &children[1] else {
            panic!("body")
        };
        let CemTreeAstNode::Element {
            name,
            children: svg,
            ..
        } = &body[0]
        else {
            panic!("svg")
        };
        assert_eq!(name, "svg");
        let CemTreeAstNode::Element { attributes, .. } = &svg[0] else {
            panic!("use")
        };
        assert_eq!(attributes[0].name, "xlink:href");
        c.abort_signal().abort();
        assert!(super::stream(input.tree, &source, &c).is_err());
    }
}
