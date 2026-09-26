//! One retained sheet, with definition collection before reference emission.
use super::{
    diagnostic, emit_css_grouping_rule, emit_css_keyframes,
    subtree::{compose, SubtreeOptions},
    CssEmissionDiagnostic, CssGroupingContext, CssKeyframesEmission, CssRuleBodyItem, CssRuleMode,
    CssRuleSubtreeEmission,
};
use crate::{
    css_resources::{attribute, named, CssResourcePlan},
    parser::AstNodeId,
};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct CssStylesheetEmission {
    pub body: CssRuleSubtreeEmission,
    pub animation_names: BTreeMap<String, String>,
}

/// Emit the supported subset of one stylesheet/style-block. Collects forward
/// keyframe references across admitted stylesheet-level conditional groups.
/// Duplicate definitions retain order and share their decoded symbol identity.
/// The caller supplies a stable suffix and adds managed wrappers. Imports are
/// diagnosed/omitted: this is not import-closure compilation or installation.
pub fn emit_css_stylesheet(
    plan: &CssResourcePlan,
    mode: CssRuleMode,
    suffix: &str,
) -> Result<CssStylesheetEmission, CssEmissionDiagnostic> {
    let tree = &plan.tree;
    let root = tree
        .node(0)
        .and_then(|n| (n.children.len() == 1).then(|| n.children[0]))
        .filter(|id| named(tree, *id, "stylesheet") || named(tree, *id, "style-block"))
        .ok_or_else(|| {
            diagnostic(
                tree,
                0,
                "cem.scoped_css.stylesheet_tree_invalid",
                "expected a retained stylesheet or style-block",
            )
        })?;
    if suffix.is_empty() {
        return Err(diagnostic(
            tree,
            root,
            "cem.scoped_css.keyframe_scope_invalid",
            "keyframe namespace suffix must not be empty",
        ));
    }
    let mut definitions = BTreeMap::new();
    for &id in &tree.node(root).unwrap().children {
        collect(plan, id, suffix, 0, &mut definitions)?;
    }
    let animation_names = definitions
        .values()
        .filter_map(|d| d.rule.as_ref())
        .map(|r| (r.name.clone(), r.scoped_name.clone()))
        .collect();
    let mut body = CssRuleSubtreeEmission::default();
    let options = SubtreeOptions {
        mode,
        names: Some(&animation_names),
        definitions: Some(&definitions),
    };
    for &id in &tree.node(root).unwrap().children {
        if named(tree, id, "comment") || named(tree, id, "charset") {
            continue;
        }
        if named(tree, id, "rule") {
            compose(
                plan,
                id,
                &options,
                CssGroupingContext::Stylesheet,
                0,
                &mut body,
            )?;
        } else {
            body.diagnostics.push(diagnostic(
                tree,
                id,
                if named(tree, id, "import") {
                    "cem.scoped_css.stylesheet_import_pending"
                } else {
                    "cem.scoped_css.stylesheet_construct_unsupported"
                },
                "construct requires compilation outside the supported single-sheet subset",
            ));
        }
    }
    Ok(CssStylesheetEmission {
        body,
        animation_names,
    })
}

fn collect(
    plan: &CssResourcePlan,
    id: AstNodeId,
    suffix: &str,
    depth: usize,
    definitions: &mut BTreeMap<AstNodeId, CssKeyframesEmission>,
) -> Result<(), CssEmissionDiagnostic> {
    let tree = &plan.tree;
    if depth >= 64 {
        return Err(diagnostic(
            tree,
            id,
            "cem.scoped_css.subtree_depth_exceeded",
            "CSS rule nesting exceeds 64 levels",
        ));
    }
    if !named(tree, id, "rule") || attribute(tree, id, "kind") != Some("at") {
        return Ok(());
    }
    let name = attribute(tree, id, "name").unwrap_or_default();
    if ["keyframes", "-webkit-keyframes"]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
    {
        definitions.insert(id, emit_css_keyframes(plan, id, suffix)?);
    } else if ["media", "supports", "container", "starting-style"]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
    {
        // Reuse grouping admission, including invalid-condition suppression.
        // Style descendants are deliberately not searched for definitions.
        if let Some(rule) = emit_css_grouping_rule(plan, id, CssGroupingContext::Stylesheet)?.rule {
            for item in rule.body {
                if let CssRuleBodyItem::Deferred(child) = item {
                    collect(plan, child.node_id, suffix, depth + 1, definitions)?;
                }
            }
        }
    }
    Ok(())
}
