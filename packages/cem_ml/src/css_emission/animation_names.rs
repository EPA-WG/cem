//! Static animation name fragments over import-owned slots and a native symbol map.
use super::{components_with, diagnostic, CssEmissionDiagnostic, CssSubtreeFragment};
use crate::{
    css_resources::{attribute, named},
    parser::{tree::RetainedCemTree, AstNodeId},
    transform_template::transform_template_encode_css_string,
};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct CssAnimationNameEmission {
    pub value: Option<String>,
    pub names: Vec<CssSubtreeFragment>,
    pub diagnostics: Vec<CssEmissionDiagnostic>,
}

/// Emit a value fragment for animation-name/animation and prefixed forms. The map is
/// compiler-owned decoded original -> scoped name, never serialized runtime AST.
/// Unmapped names remain external. Keywords are not symbol references; quoted
/// strings with identical spelling are. Ordinary declaration policy still applies.
/// Dynamic names and unsupported grammar have no emitted value, not a raw fallback.
pub fn emit_css_animation_names(
    tree: &RetainedCemTree,
    declaration: AstNodeId,
    names: &BTreeMap<String, String>,
) -> Result<CssAnimationNameEmission, CssEmissionDiagnostic> {
    if !named(tree, declaration, "declaration")
        || !attribute(tree, declaration, "name").is_some_and(|n| {
            n.eq_ignore_ascii_case("animation-name")
                || n.eq_ignore_ascii_case("-webkit-animation-name")
                || n.eq_ignore_ascii_case("animation")
                || n.eq_ignore_ascii_case("-webkit-animation")
        })
    {
        return Err(invalid(tree, declaration));
    }
    let lists: Vec<_> = tree
        .node(declaration)
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|id| named(tree, *id, "animation-name-list"))
        .collect();
    if lists.len() != 1 {
        return Err(invalid(tree, declaration));
    }
    let list = lists[0];
    match attribute(tree, list, "analysis-status") {
        Some("complete") => {}
        Some("dynamic" | "unsupported") => {
            return Ok(CssAnimationNameEmission {
                diagnostics: vec![diagnostic(
                    tree,
                    list,
                    if attribute(tree, list, "analysis-status") == Some("dynamic") {
                        "cem.scoped_css.animation_name_dynamic_unsupported"
                    } else {
                        "cem.scoped_css.animation_name_unsupported"
                    },
                    "animation references require the supported static-name profile",
                )],
                ..Default::default()
            })
        }
        _ => return Err(invalid(tree, list)),
    }
    let mut slots = BTreeMap::new();
    for &slot in &tree.node(list).unwrap().children {
        let is_name = named(tree, slot, "animation-name-slot");
        let is_value = named(tree, slot, "animation-value-slot")
            && matches!(
                attribute(tree, declaration, "name")
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("animation" | "-webkit-animation")
            )
            && matches!(
                attribute(tree, slot, "kind"),
                Some("duration" | "delay" | "easing" | "iteration" | "direction" | "fill" | "play")
            );
        if !is_value
            && (!is_name
                || !matches!(
                    attribute(tree, slot, "kind"),
                    Some("ident" | "string" | "keyword")
                ))
        {
            return Err(invalid(tree, slot));
        }
        let node = tree.node(slot).unwrap();
        if slots
            .insert((node.range.offset, node.range.length), slot)
            .is_some()
        {
            return Err(invalid(tree, slot));
        }
    }
    if slots.is_empty() {
        return Err(invalid(tree, list));
    }
    let mut fragments = Vec::new();
    let value = components_with(tree, declaration, |component, token| {
        let node = tree.node(component).unwrap();
        let Some(slot) = slots.remove(&(node.range.offset, node.range.length)) else {
            return if matches!(
                attribute(tree, component, "kind"),
                Some("whitespace" | "comment" | "delimiter")
            ) {
                Ok(token.to_owned())
            } else {
                Err(invalid(tree, component))
            };
        };
        if attribute(tree, slot, "token") != Some(token) {
            return Err(invalid(tree, slot));
        }
        let original = attribute(tree, slot, "value").ok_or_else(|| invalid(tree, slot))?;
        let replacement = if named(tree, slot, "animation-name-slot")
            && attribute(tree, slot, "kind") != Some("keyword")
        {
            names
                .get(original)
                .map(|name| transform_template_encode_css_string(name))
        } else {
            None
        };
        let text = replacement.unwrap_or_else(|| token.to_owned());
        let slot_node = tree.node(slot).unwrap();
        if named(tree, slot, "animation-name-slot") {
            fragments.push(CssSubtreeFragment {
                node_id: slot,
                text: text.clone(),
                source: slot_node.source.clone(),
                range: slot_node.range,
            });
        }
        Ok(text)
    })?;
    if !slots.is_empty() {
        return Err(invalid(tree, list));
    }
    Ok(CssAnimationNameEmission {
        value: Some(value),
        names: fragments,
        diagnostics: Vec::new(),
    })
}
fn invalid(tree: &RetainedCemTree, id: AstNodeId) -> CssEmissionDiagnostic {
    diagnostic(
        tree,
        id,
        "cem.scoped_css.animation_name_tree_invalid",
        "missing or inconsistent retained animation-name slots",
    )
}
