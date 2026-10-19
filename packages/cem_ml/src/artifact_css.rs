//! Explicit CSS resource transport for immutable artifact previews. Import owns
//! token decoding/ranges; hosts supply admitted replacement URLs, never an AST.
use crate::{
    css_resources::{attribute, resource_reference, CssResourceKind},
    import::import_data,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct ArtifactCssResource {
    pub specifier: String,
    pub import: bool,
}
#[derive(Debug, Serialize)]
pub struct ArtifactCss {
    pub resources: Vec<ArtifactCssResource>,
    pub css: String,
}

pub fn remap_artifact_css(
    source: &str,
    inline: bool,
    replacements: Option<&BTreeMap<String, String>>,
) -> Result<ArtifactCss, String> {
    let tree = import_data(
        source,
        if inline {
            "text/css; mode=style-attribute"
        } else {
            "text/css; mode=stylesheet"
        },
        "cem",
        "urn:cem:artifact-css",
    )?;
    let mut pending = vec![0];
    let mut resources = Vec::new();
    let mut edits = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).ok_or("cem.preview.css_invalid_node")?;
        if attribute(&tree, id, "resource-analysis") == Some("unsupported") {
            return Err("cem.preview.css_resource_grammar_unsupported".into());
        }
        let Some((specifier, kind)) = resource_reference(&tree, id)? else {
            pending.extend(node.children.iter().rev().copied());
            continue;
        };
        resources.push(ArtifactCssResource {
            specifier: specifier.clone(),
            import: matches!(kind, CssResourceKind::Import { .. }),
        });
        if specifier.starts_with('#') && kind == CssResourceKind::Url {
            continue;
        }
        let Some(replacements) = replacements else {
            continue;
        };
        let target = replacements
            .get(&specifier)
            .ok_or_else(|| format!("cem.preview.css_missing_mapping: {specifier}"))?;
        let mut replacement = format!("url({})", css_string(target));
        if let CssResourceKind::Import {
            layer,
            supports,
            media,
        } = kind
        {
            replacement = format!("@import {replacement}");
            if let Some(layer) = layer {
                replacement.push_str(&if layer.is_empty() {
                    " layer".into()
                } else {
                    format!(" layer({layer})")
                });
            }
            if let Some(supports) = supports {
                replacement.push_str(&format!(" supports({supports})"));
            }
            if let Some(media) = media {
                replacement.push_str(&format!(" {media}"));
            }
            replacement.push(';');
        }
        edits.push((
            node.range.offset as usize,
            (node.range.offset + node.range.length) as usize,
            replacement,
        ));
    }
    edits.sort_by_key(|(start, _, _)| *start);
    let mut css = String::new();
    let mut cursor = 0;
    for (start, end, replacement) in edits {
        if start < cursor || source.get(start..end).is_none() {
            return Err("cem.preview.css_invalid_range".into());
        }
        css.push_str(&source[cursor..start]);
        css.push_str(&replacement);
        cursor = end;
    }
    css.push_str(&source[cursor..]);
    Ok(ArtifactCss { resources, css })
}
fn css_string(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        if ch == '\0' {
            result.push('\u{fffd}');
        } else if ch == '"' || ch == '\\' || ch.is_control() {
            result.push_str(&format!("\\{:X} ", ch as u32));
        } else {
            result.push(ch);
        }
    }
    result.push('"');
    result
}
