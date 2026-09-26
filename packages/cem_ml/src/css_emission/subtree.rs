//! Compose the supported rule subset, keeping native nesting and declaration runs.
use std::collections::BTreeMap;

use super::{
    diagnostic, grouping::emit_grouping_rule_with_symbols, rules::emit_style_rule_with_symbols,
    CssEmissionDiagnostic, CssGroupingContext, CssRuleBodyItem, CssRuleMode,
};
use crate::{
    css_resources::{attribute, named, CssResourcePlan},
    parser::{tree::CemTreeRange, AstNodeId},
    source_map::SourceMapStack,
};

#[derive(Debug)]
pub struct CssSubtreeFragment {
    pub text: String,
    pub node_id: AstNodeId,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug, Default)]
pub struct CssRuleSubtreeEmission {
    pub fragments: Vec<CssSubtreeFragment>,
    pub diagnostics: Vec<CssEmissionDiagnostic>,
}
impl CssRuleSubtreeEmission {
    pub fn css(&self) -> String {
        self.fragments
            .iter()
            .map(|part| part.text.as_str())
            .collect()
    }
}

/// Compose one top-level style rule or supported grouping rule and its supported descendants.
/// Output preserves declaration runs verbatim in order, including pseudo-element
/// matches after nested rules. Every fragment maps to its retained source node.
/// Unsupported constructs are diagnosed and omitted, never passed through raw.
/// This is a rule subset, not the complete managed stylesheet compiler: imports,
/// other grouping rules and keyframe rewriting still need their own compilation.
/// Callers must inspect diagnostics; this API does not authorize installation.
pub fn emit_css_rule_subtree(
    plan: &CssResourcePlan,
    rule: AstNodeId,
    mode: CssRuleMode,
) -> Result<CssRuleSubtreeEmission, CssEmissionDiagnostic> {
    emit_subtree(plan, rule, mode, None)
}

/// Compose a supported subtree with animation-reference rewriting throughout
/// its nested declarations. The caller owns the complete symbol map; definition
/// collection, namespace ownership and full stylesheet compilation remain separate.
/// The resource plan is reusable with another map without changing its retained tree.
pub fn emit_css_rule_subtree_with_symbols(
    plan: &CssResourcePlan,
    rule: AstNodeId,
    mode: CssRuleMode,
    names: &BTreeMap<String, String>,
) -> Result<CssRuleSubtreeEmission, CssEmissionDiagnostic> {
    emit_subtree(plan, rule, mode, Some(names))
}

fn emit_subtree(
    plan: &CssResourcePlan,
    rule: AstNodeId,
    mode: CssRuleMode,
    names: Option<&BTreeMap<String, String>>,
) -> Result<CssRuleSubtreeEmission, CssEmissionDiagnostic> {
    let tree = &plan.tree;
    let parent = tree.node(rule).and_then(|n| n.parent);
    if !named(tree, rule, "rule")
        || !parent.is_some_and(|id| named(tree, id, "stylesheet") || named(tree, id, "style-block"))
    {
        return Err(diagnostic(
            tree,
            rule,
            "cem.scoped_css.subtree_root_invalid",
            "expected a top-level retained CSS rule",
        ));
    }
    let mut output = CssRuleSubtreeEmission::default();
    compose(
        plan,
        rule,
        mode,
        CssGroupingContext::Stylesheet,
        names,
        0,
        &mut output,
    )?;
    Ok(output)
}

fn compose(
    plan: &CssResourcePlan,
    id: AstNodeId,
    mode: CssRuleMode,
    context: CssGroupingContext,
    names: Option<&BTreeMap<String, String>>,
    depth: usize,
    output: &mut CssRuleSubtreeEmission,
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
    let (opening, body, child_context) =
        if named(tree, id, "rule") && attribute(tree, id, "kind") == Some("style") {
            let result = emit_style_rule_with_symbols(plan, id, mode, names)?;
            append_diagnostics(output, result.diagnostics);
            let Some(rule) = result.rule else {
                return Ok(());
            };
            (
                format!(
                    "{} {{",
                    rule.selectors
                        .iter()
                        .map(|s| s.text.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                rule.body,
                CssGroupingContext::StyleRule,
            )
        } else if named(tree, id, "rule")
            && attribute(tree, id, "kind") == Some("at")
            && attribute(tree, id, "name").is_some_and(|n| {
                n.eq_ignore_ascii_case("media")
                    || n.eq_ignore_ascii_case("supports")
                    || n.eq_ignore_ascii_case("starting-style")
                    || n.eq_ignore_ascii_case("container")
            })
        {
            let result = emit_grouping_rule_with_symbols(plan, id, context, names)?;
            append_diagnostics(output, result.diagnostics);
            let Some(rule) = result.rule else {
                return Ok(());
            };
            (rule.opening, rule.body, context)
        } else {
            append_diagnostics(
                output,
                vec![diagnostic(
                    tree,
                    id,
                    "cem.scoped_css.subtree_construct_unsupported",
                    "construct requires compilation outside the supported grouping subset",
                )],
            );
            return Ok(());
        };
    let start = output.fragments.len();
    push_fragment(plan, output, id, opening);
    for item in body {
        match item {
            CssRuleBodyItem::Declaration(d) => output.fragments.push(CssSubtreeFragment {
                text: d.text,
                node_id: d.node_id,
                source: d.source,
                range: d.range,
            }),
            CssRuleBodyItem::Deferred(d) => compose(
                plan,
                d.node_id,
                mode,
                child_context,
                names,
                depth + 1,
                output,
            )?,
        }
    }
    if output.fragments.len() == start + 1 {
        output.fragments.truncate(start);
    } else {
        push_fragment(plan, output, id, "}".into());
    }
    Ok(())
}
fn push_fragment(
    plan: &CssResourcePlan,
    output: &mut CssRuleSubtreeEmission,
    id: AstNodeId,
    text: String,
) {
    let node = plan.tree.node(id).unwrap();
    output.fragments.push(CssSubtreeFragment {
        text,
        node_id: id,
        source: node.source.clone(),
        range: node.range,
    });
}
fn append_diagnostics(
    output: &mut CssRuleSubtreeEmission,
    diagnostics: Vec<CssEmissionDiagnostic>,
) {
    for d in diagnostics {
        // Parent-aware selector emission revisits ancestors. Preserve the first
        // source occurrence while avoiding duplicate reports for each descendant.
        if !output.diagnostics.iter().any(|old| {
            old.code == d.code
                && old.message == d.message
                && old.source == d.source
                && old.range.offset == d.range.offset
                && old.range.length == d.range.length
        }) {
            output.diagnostics.push(d);
        }
    }
}
