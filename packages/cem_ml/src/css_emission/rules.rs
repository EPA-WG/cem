//! Ordered rule fragments and managed scope boundaries. Deferred children are
//! explicit holes, not emitted CSS; callers must finish compilation before use.
use std::collections::{BTreeMap, BTreeSet};

use super::{
    emit_css_declaration_selectors, emit_css_instance_selectors,
    emit_css_rule_declarations_with_resources, selectors::ident, CssEmissionDiagnostic,
    CssEmittedDeclaration, CssEmittedSelector,
};
use crate::{
    css_resources::CssResourcePlan,
    parser::{tree::CemTreeRange, AstNodeId},
    source_map::SourceMapStack,
    transform_template::transform_template_encode_css_string,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssRuleMode {
    Declaration,
    Instance,
}

#[derive(Debug)]
pub struct CssDeferredRule {
    /// Node in the same retained resource plan, still requiring compilation.
    pub node_id: AstNodeId,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug)]
pub enum CssRuleBodyItem {
    Declaration(CssEmittedDeclaration),
    Deferred(CssDeferredRule),
}

#[derive(Debug)]
pub struct CssStyleRule {
    pub node_id: AstNodeId,
    pub selectors: Vec<CssEmittedSelector>,
    /// Authored order, including the exact positions of nested constructs.
    pub body: Vec<CssRuleBodyItem>,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug)]
pub struct CssStyleRuleEmission {
    /// None when no selector survives policy or no body content remains.
    pub rule: Option<CssStyleRule>,
    pub diagnostics: Vec<CssEmissionDiagnostic>,
}

/// Join selector and resolved declaration emission without flattening nested
/// rules across surrounding declarations. All node IDs belong to `plan.tree`.
/// Diagnostics retain the existing selector/declaration policies and source data.
/// This deliberately returns structured parts, not installable CSS: nested rule,
/// keyframe-reference and whole-stylesheet policies still require compilation.
pub fn emit_css_style_rule(
    plan: &CssResourcePlan,
    rule: AstNodeId,
    mode: CssRuleMode,
) -> Result<CssStyleRuleEmission, CssEmissionDiagnostic> {
    let selectors = match mode {
        CssRuleMode::Declaration => emit_css_declaration_selectors(&plan.tree, rule)?,
        CssRuleMode::Instance => emit_css_instance_selectors(&plan.tree, rule)?,
    };
    let declarations = emit_css_rule_declarations_with_resources(plan, rule)?;
    let mut diagnostics = selectors.diagnostics;
    diagnostics.extend(declarations.diagnostics);
    if selectors.selectors.is_empty() {
        return Ok(CssStyleRuleEmission {
            rule: None,
            diagnostics,
        });
    }
    let mut accepted: BTreeMap<_, _> = declarations
        .declarations
        .into_iter()
        .map(|declaration| (declaration.node_id, declaration))
        .collect();
    let deferred: BTreeSet<_> = declarations.deferred_children.into_iter().collect();
    // The two emitters above validated the rule and its direct child structure.
    let node = plan.tree.node(rule).unwrap();
    let mut body = Vec::new();
    for &child in &node.children {
        if let Some(declaration) = accepted.remove(&child) {
            body.push(CssRuleBodyItem::Declaration(declaration));
        } else if deferred.contains(&child) {
            let child_node = plan.tree.node(child).unwrap();
            body.push(CssRuleBodyItem::Deferred(CssDeferredRule {
                node_id: child,
                source: child_node.source.clone(),
                range: child_node.range,
            }));
        }
    }
    Ok(CssStyleRuleEmission {
        rule: (!body.is_empty()).then_some(CssStyleRule {
            node_id: rule,
            selectors: selectors.selectors,
            body,
            source: node.source.clone(),
            range: node.range,
        }),
        diagnostics,
    })
}

/// Semantic names from an already validated declaration/ownership context, not
/// raw selector strings. The host remains responsible for tag registration,
/// public scope-name validation, context assignment and stylesheet lifecycle.
#[derive(Debug)]
pub enum CssManagedScope {
    Private {
        tag: String,
        context: Option<String>,
    },
    Shared {
        name: String,
        context: Option<String>,
    },
    Instance,
}

#[derive(Debug)]
pub struct CssScopeWrapper {
    pub opening: String,
    pub closing: &'static str,
}

/// Generate only the managed wrapper. It does not authorize installation of
/// incomplete rule fragments or qualify hosts with context attributes itself.
/// Instance styles use their parent as an implicit root, without a UUID selector.
pub fn emit_css_scope_wrapper(
    scope: &CssManagedScope,
) -> Result<CssScopeWrapper, CssEmissionDiagnostic> {
    let context_suffix = |context: &Option<String>| -> Result<String, CssEmissionDiagnostic> {
        match context {
            None => Ok(String::new()),
            Some(value) if value.is_empty() => Err(invalid_scope("empty CSS context identity")),
            Some(value) => Ok(format!(
                "[data-cem-css-context={}]",
                transform_template_encode_css_string(value)
            )),
        }
    };
    let root = match scope {
        CssManagedScope::Private { tag, context } => {
            if tag.is_empty() {
                return Err(invalid_scope("empty produced tag"));
            }
            format!(
                " ({tag}{context})",
                tag = ident(tag),
                context = context_suffix(context)?
            )
        }
        CssManagedScope::Shared { name, context } => {
            if name.is_empty() {
                return Err(invalid_scope("empty shared scope name"));
            }
            format!(
                " ([scope={}]{context}:has(> template[data-cem-island=\"instance\"]))",
                transform_template_encode_css_string(name),
                context = context_suffix(context)?
            )
        }
        CssManagedScope::Instance => String::new(),
    };
    Ok(CssScopeWrapper {
        opening: format!("@scope{root} to (:scope :has(> template[data-cem-island=\"instance\"]) > *, [slot] > *) {{"),
        closing: "}",
    })
}

fn invalid_scope(message: &str) -> CssEmissionDiagnostic {
    CssEmissionDiagnostic {
        code: "cem.scoped_css.scope_invalid",
        message: message.into(),
        source: SourceMapStack::default(),
        range: CemTreeRange::default(),
    }
}
