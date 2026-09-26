//! Project retained semantic slots back onto the lossless source event stream.
//! Classification remains at import; presentation consumes roles and ranges only.
use super::*;
use crate::parser::CemAstNode;

pub(crate) fn annotate_retained_css_roles(document: &mut CssDocumentAst) {
    if !document.events.iter().any(|e| {
        (e.token_kind == "at-keyword"
            && e.value.as_deref().is_some_and(|n| {
                n.eq_ignore_ascii_case("keyframes") || n.eq_ignore_ascii_case("-webkit-keyframes")
            }))
            || (e.semantic_kind == CssSemanticKindAst::Property
                && e.value.as_deref().is_some_and(|n| {
                    [
                        "animation",
                        "animation-name",
                        "-webkit-animation",
                        "-webkit-animation-name",
                    ]
                    .iter()
                    .any(|p| n.eq_ignore_ascii_case(p))
                }))
    }) {
        return;
    }
    // Reuse the shared projection over the existing event stream, never tokenize
    // the whole source again. Unsupported/recovered/over-limit documents keep broad roles.
    let Ok((ast, semantics)) = project(document) else {
        return;
    };
    let mut roles = Vec::new();
    for node in &ast.nodes {
        let CemAstNode::Element {
            node_id,
            expanded_name,
            attributes,
            ..
        } = node
        else {
            continue;
        };
        let attr = |name: &str| {
            attributes
                .iter()
                .find_map(|id| match &ast.nodes[*id as usize] {
                    CemAstNode::Attribute {
                        expanded_name,
                        value,
                        ..
                    } if expanded_name.local_name == name => value.as_deref(),
                    _ => None,
                })
        };
        let role = match expanded_name.local_name.as_str() {
            "keyframe-name" if attr("syntax-valid") == Some("true") => CssSemanticKindAst::Symbol,
            "animation-name-slot" if attr("kind") == Some("ident") => CssSemanticKindAst::Symbol,
            "animation-name-slot" if attr("kind") == Some("keyword") => CssSemanticKindAst::Keyword,
            "animation-value-slot" | "keyframe-selector" => CssSemanticKindAst::Keyword,
            _ => continue,
        };
        if let Some(range) = semantics.ranges.get(node_id) {
            roles.push((*range, role));
        }
    }
    for event in &mut document.events {
        // Preserve lexical string/number/comment/function roles, even within a
        // semantically classified slot; only identifiers need refinement.
        if event.token_kind != "ident" || event.recovered {
            continue;
        }
        let start = event.source_range.start.byte_offset;
        let end = start + event.source_range.byte_length;
        if let Some((_, role)) = roles
            .iter()
            .find(|(r, _)| start >= r.offset && end <= r.offset + r.length)
        {
            event.semantic_kind = *role;
        }
    }
}
