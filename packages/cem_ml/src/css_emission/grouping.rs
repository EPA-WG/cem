//! Conditional grouping fragments. Child rules keep their inherited context and
//! original positions; recursive selector/keyframe compilation remains explicit.
use super::{
    components, declarations::emit_group_declarations, diagnostic, media_condition,
    rules::ordered_body, CssEmissionDiagnostic, CssRuleBodyItem,
};
use crate::{
    css_resources::{attribute, named, CssResourcePlan},
    parser::{
        tree::{CemTreeRange, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssGroupingContext {
    Stylesheet,
    /// A containing style rule supplies the implicit selector for declarations.
    StyleRule,
}

#[derive(Debug)]
pub struct CssGroupingRule {
    pub node_id: AstNodeId,
    pub opening: String,
    pub body: Vec<CssRuleBodyItem>,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug)]
pub struct CssGroupingRuleEmission {
    pub rule: Option<CssGroupingRule>,
    pub diagnostics: Vec<CssEmissionDiagnostic>,
}

/// Emit retained @media/@supports/@starting-style groups. Does not flatten groups or invent
/// selectors for their declarations. The caller must propagate style-rule context
/// through nested groups, compile deferred children and close each emitted block.
/// No raw prelude parsing/fallback or browser feature evaluation is performed.
pub fn emit_css_grouping_rule(
    plan: &CssResourcePlan,
    rule: AstNodeId,
    context: CssGroupingContext,
) -> Result<CssGroupingRuleEmission, CssEmissionDiagnostic> {
    let tree = &plan.tree;
    if !named(tree, rule, "rule") || attribute(tree, rule, "kind") != Some("at") {
        return Err(invalid(tree, rule));
    }
    let name = attribute(tree, rule, "name").ok_or_else(|| invalid(tree, rule))?;
    let local = if name.eq_ignore_ascii_case("media") {
        "group-media"
    } else if name.eq_ignore_ascii_case("supports") {
        "group-supports"
    } else if name.eq_ignore_ascii_case("starting-style") {
        "group-starting-style"
    } else {
        return Err(invalid(tree, rule));
    };
    match attribute(tree, rule, "has-block") {
        Some("false") => {
            return Ok(suppressed(diagnostic(
                tree,
                rule,
                "cem.scoped_css.group_block_required",
                "conditional grouping rule requires a block",
            )))
        }
        Some("true") => {}
        _ => return Err(invalid(tree, rule)),
    }
    let node = tree.node(rule).unwrap();
    if node.children.len() != 1 || !named(tree, node.children[0], "at-rule") {
        return Err(invalid(tree, rule));
    }
    let container = node.children[0];
    if attribute(tree, container, "name") != Some(name) {
        return Err(invalid(tree, container));
    }
    let conditions: Vec<_> = tree
        .node(container)
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|child| {
            named(tree, *child, "group-media")
                || named(tree, *child, "group-supports")
                || named(tree, *child, "group-starting-style")
        })
        .collect();
    if conditions.len() != 1 || !named(tree, conditions[0], local) {
        return Err(invalid(tree, container));
    }
    let condition = conditions[0];
    let (text, mut diagnostics) = if local == "group-media" {
        if attribute(tree, condition, "empty") == Some("true") {
            if !tree.node(condition).unwrap().children.is_empty() {
                return Err(invalid(tree, condition));
            }
            ("all".to_owned(), Vec::new())
        } else {
            media_condition(tree, condition)?
        }
    } else if local == "group-starting-style" {
        match attribute(tree, condition, "syntax-valid") {
            Some("false") => {
                return Ok(suppressed(diagnostic(
                    tree,
                    condition,
                    "cem.scoped_css.starting_style_prelude_invalid",
                    "@starting-style requires an empty prelude",
                )))
            }
            Some("true") => {}
            _ => return Err(invalid(tree, condition)),
        }
        (String::new(), Vec::new())
    } else {
        match attribute(tree, condition, "syntax-valid") {
            Some("false") => {
                return Ok(suppressed(diagnostic(
                    tree,
                    condition,
                    "cem.scoped_css.supports_condition_invalid",
                    "invalid retained @supports condition",
                )))
            }
            Some("true") => {}
            _ => return Err(invalid(tree, condition)),
        }
        let text = components(tree, condition)?;
        if text.is_empty() {
            return Err(invalid(tree, condition));
        }
        (text, Vec::new())
    };
    let mut declarations = emit_group_declarations(plan, container)?;
    diagnostics.append(&mut declarations.diagnostics);
    if context == CssGroupingContext::Stylesheet {
        for declaration in declarations.declarations.drain(..) {
            diagnostics.push(diagnostic(
                tree,
                declaration.node_id,
                "cem.scoped_css.group_declaration_unsupported",
                "group declaration has no containing style rule",
            ));
        }
    }
    let body = ordered_body(
        tree,
        container,
        declarations.declarations,
        declarations.deferred_children,
    );
    Ok(CssGroupingRuleEmission {
        rule: (!body.is_empty()).then_some(CssGroupingRule {
            node_id: rule,
            opening: if local == "group-starting-style" {
                "@starting-style {".into()
            } else {
                format!("@{} {text} {{", name.to_ascii_lowercase())
            },
            body,
            source: node.source.clone(),
            range: node.range,
        }),
        diagnostics,
    })
}
fn suppressed(diagnostic: CssEmissionDiagnostic) -> CssGroupingRuleEmission {
    CssGroupingRuleEmission {
        rule: None,
        diagnostics: vec![diagnostic],
    }
}
fn invalid(tree: &RetainedCemTree, id: AstNodeId) -> CssEmissionDiagnostic {
    diagnostic(
        tree,
        id,
        "cem.scoped_css.group_tree_invalid",
        "missing or inconsistent retained grouping structure",
    )
}
