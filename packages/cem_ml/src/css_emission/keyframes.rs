//! Scoped keyframe definitions from import-owned names and offset lists.
use super::{
    declarations::emit_keyframe_declarations, diagnostic, selectors::ident, CssEmissionDiagnostic,
    CssEmittedDeclaration,
};
use crate::{
    css_resources::{attribute, named, CssResourcePlan},
    parser::{
        tree::{CemTreeRange, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
};

#[derive(Debug)]
pub struct CssEmittedKeyframeOffset {
    pub node_id: AstNodeId,
    pub text: String,
    pub offset: f64,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}
#[derive(Debug)]
pub struct CssKeyframeBlock {
    pub node_id: AstNodeId,
    pub selectors: Vec<CssEmittedKeyframeOffset>,
    pub declarations: Vec<CssEmittedDeclaration>,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}
#[derive(Debug)]
pub struct CssKeyframesRule {
    pub node_id: AstNodeId,
    pub name: String,
    pub scoped_name: String,
    pub opening: String,
    pub frames: Vec<CssKeyframeBlock>,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}
impl CssKeyframesRule {
    pub fn css(&self) -> String {
        let mut css = self.opening.clone();
        for frame in &self.frames {
            css.push_str(
                &frame
                    .selectors
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            css.push_str(" {");
            for declaration in &frame.declarations {
                css.push_str(&declaration.text);
            }
            css.push('}');
        }
        css.push('}');
        css
    }
}
#[derive(Debug, Default)]
pub struct CssKeyframesEmission {
    pub rule: Option<CssKeyframesRule>,
    pub diagnostics: Vec<CssEmissionDiagnostic>,
}

/// Emit one definition using a caller-owned stable stylesheet/context suffix.
/// This does not choose ownership, rewrite animation references, assemble import
/// closures or authorize installation. Offsets bypass DOM selector policies.
pub fn emit_css_keyframes(
    plan: &CssResourcePlan,
    rule: AstNodeId,
    suffix: &str,
) -> Result<CssKeyframesEmission, CssEmissionDiagnostic> {
    let tree = &plan.tree;
    if suffix.is_empty() {
        return Err(diagnostic(
            tree,
            rule,
            "cem.scoped_css.keyframe_scope_invalid",
            "keyframe namespace suffix must not be empty",
        ));
    }
    if !named(tree, rule, "rule") || attribute(tree, rule, "kind") != Some("at") {
        return Err(invalid(tree, rule));
    }
    let kind = attribute(tree, rule, "name")
        .ok_or_else(|| invalid(tree, rule))?
        .to_ascii_lowercase();
    if !matches!(kind.as_str(), "keyframes" | "-webkit-keyframes") {
        return Err(invalid(tree, rule));
    }
    let node = tree.node(rule).unwrap();
    if attribute(tree, rule, "has-block") == Some("false") {
        return Ok(CssKeyframesEmission {
            rule: None,
            diagnostics: vec![diagnostic(
                tree,
                rule,
                "cem.scoped_css.keyframe_block_required",
                "keyframes require a block",
            )],
        });
    }
    if attribute(tree, rule, "has-block") != Some("true") {
        return Err(invalid(tree, rule));
    }
    if node.children.len() != 1 || !named(tree, node.children[0], "at-rule") {
        return Err(invalid(tree, rule));
    }
    let container = node.children[0];
    if attribute(tree, container, "name")
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some(&kind)
    {
        return Err(invalid(tree, container));
    }
    let children = &tree.node(container).unwrap().children;
    let names: Vec<_> = children
        .iter()
        .copied()
        .filter(|id| named(tree, *id, "keyframe-name"))
        .collect();
    if names.len() != 1 {
        return Err(invalid(tree, container));
    }
    let name_node = names[0];
    match attribute(tree, name_node, "syntax-valid") {
        Some("false") => {
            return Ok(CssKeyframesEmission {
                rule: None,
                diagnostics: vec![diagnostic(
                    tree,
                    name_node,
                    "cem.scoped_css.keyframe_name_invalid",
                    "invalid retained keyframe name",
                )],
            })
        }
        Some("true") => {}
        _ => return Err(invalid(tree, name_node)),
    }
    if !matches!(attribute(tree, name_node, "kind"), Some("ident" | "string")) {
        return Err(invalid(tree, name_node));
    }
    let name = attribute(tree, name_node, "value")
        .ok_or_else(|| invalid(tree, name_node))?
        .to_owned();
    let scoped_name = format!("{name}-{suffix}");
    let mut result = CssKeyframesEmission::default();
    let mut frames = Vec::new();
    for &id in children {
        if id == name_node || named(tree, id, "comment") {
            continue;
        }
        if !named(tree, id, "rule") || attribute(tree, id, "kind") != Some("keyframe") {
            result.diagnostics.push(unsupported_body(tree, id));
            continue;
        }
        let frame = tree.node(id).unwrap();
        let lists: Vec<_> = frame
            .children
            .iter()
            .copied()
            .filter(|child| named(tree, *child, "keyframe-selector-list"))
            .collect();
        if lists.len() != 1 {
            return Err(invalid(tree, id));
        }
        let list = lists[0];
        match attribute(tree, list, "syntax-valid") {
            Some("false") => {
                result.diagnostics.push(diagnostic(
                    tree,
                    list,
                    "cem.scoped_css.keyframe_offset_unsupported",
                    "invalid or unsupported keyframe offset list",
                ));
                continue;
            }
            Some("true") => {}
            _ => return Err(invalid(tree, list)),
        }
        let mut selectors = Vec::new();
        for &offset_id in &tree.node(list).unwrap().children {
            if named(tree, offset_id, "component-value") {
                continue;
            }
            if !named(tree, offset_id, "keyframe-selector")
                || !matches!(
                    attribute(tree, offset_id, "kind"),
                    Some("ident" | "percentage")
                )
            {
                return Err(invalid(tree, offset_id));
            }
            let offset: f64 = attribute(tree, offset_id, "value")
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| invalid(tree, offset_id))?;
            if !offset.is_finite() || !(0.0..=1.0).contains(&offset) {
                return Err(invalid(tree, offset_id));
            }
            let text = attribute(tree, offset_id, "token")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| invalid(tree, offset_id))?
                .to_owned();
            let offset_node = tree.node(offset_id).unwrap();
            selectors.push(CssEmittedKeyframeOffset {
                node_id: offset_id,
                text,
                offset,
                source: offset_node.source.clone(),
                range: offset_node.range,
            });
        }
        if selectors.is_empty() {
            return Err(invalid(tree, list));
        }
        let declarations = emit_keyframe_declarations(plan, id)?;
        result.diagnostics.extend(declarations.diagnostics);
        result.diagnostics.extend(
            declarations
                .deferred_children
                .into_iter()
                .map(|child| unsupported_body(tree, child)),
        );
        frames.push(CssKeyframeBlock {
            node_id: id,
            selectors,
            declarations: declarations.declarations,
            source: frame.source.clone(),
            range: frame.range,
        });
    }
    // Even an empty definition participates in animation matching/lifecycle.
    result.rule = Some(CssKeyframesRule {
        node_id: rule,
        opening: format!("@{kind} {} {{", ident(&scoped_name)),
        name,
        scoped_name,
        frames,
        source: node.source.clone(),
        range: node.range,
    });
    Ok(result)
}
fn invalid(tree: &RetainedCemTree, id: AstNodeId) -> CssEmissionDiagnostic {
    diagnostic(
        tree,
        id,
        "cem.scoped_css.keyframe_tree_invalid",
        "missing or inconsistent retained keyframe structure",
    )
}
fn unsupported_body(tree: &RetainedCemTree, id: AstNodeId) -> CssEmissionDiagnostic {
    diagnostic(
        tree,
        id,
        "cem.scoped_css.keyframe_body_unsupported",
        "keyframes admit only frame blocks with direct declarations",
    )
}
