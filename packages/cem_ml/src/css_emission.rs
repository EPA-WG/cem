//! Native managed-CSS emission helpers for trees from the shared CSS importer.
//! This is not an installable stylesheet compiler or a validator for arbitrary
//! user-constructed token attributes. No CSS is reparsed downstream.
mod declarations;
pub use declarations::{
    emit_css_rule_declarations, emit_css_rule_declarations_with_resources, CssEmittedDeclaration,
    CssRuleDeclarations,
};
mod resources;
mod subtree;
pub use subtree::{emit_css_rule_subtree, CssRuleSubtreeEmission, CssSubtreeFragment};

mod grouping;
pub use grouping::{
    emit_css_grouping_rule, CssGroupingContext, CssGroupingRule, CssGroupingRuleEmission,
};

mod rules;
pub use rules::{
    emit_css_scope_wrapper, emit_css_style_rule, CssDeferredRule, CssManagedScope, CssRuleBodyItem,
    CssRuleMode, CssScopeWrapper, CssStyleRule, CssStyleRuleEmission,
};

mod selectors;
pub use selectors::{
    emit_css_declaration_selectors, emit_css_instance_selectors, emit_css_nested_selectors,
    CssEmittedSelector, CssSelectorEmission,
};

use crate::{
    parser::{
        tree::{CemTreeRange, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
};

#[derive(Debug)]
pub struct CssEmissionDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug)]
pub struct CssConditionWrapper {
    /// Complete opening at-rule, including its opening brace.
    pub opening: String,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug)]
pub enum CssImportConditionEmission {
    /// Outermost wrapper first. Close each wrapper with `}` after its emitted body.
    Emit {
        wrappers: Vec<CssConditionWrapper>,
        diagnostics: Vec<CssEmissionDiagnostic>,
    },
    /// The entire import occurrence, including its body, must be omitted.
    Suppress(CssEmissionDiagnostic),
}

/// Emit only the conditional openings around an imported managed-CSS body.
/// The caller must compile/scope that body, preserve import order and close each
/// wrapper. These fragments do not authorize fetching or browser installation.
/// Raw `supports`/`media` attributes are metadata, never a parsing fallback.
pub fn emit_css_import_conditions(
    tree: &RetainedCemTree,
    id: AstNodeId,
) -> Result<CssImportConditionEmission, CssEmissionDiagnostic> {
    use crate::css_resources::{attribute, named};
    if !named(tree, id, "import") {
        return Err(invalid(tree, id, "expected a retained CSS import node"));
    }
    let children = &tree.node(id).unwrap().children;
    let layer = children
        .iter()
        .copied()
        .find(|child| named(tree, *child, "import-layer"));
    if layer.is_some() || attribute(tree, id, "layer").is_some() {
        return Ok(CssImportConditionEmission::Suppress(diagnostic(
            tree,
            layer.unwrap_or(id),
            "cem.scoped_css.layer_unsupported",
            "managed CSS suppresses layered imports",
        )));
    }
    let mut supports = None;
    let mut media = None;
    for &child in children {
        let slot = if named(tree, child, "import-supports") {
            &mut supports
        } else if named(tree, child, "import-media") {
            &mut media
        } else {
            return Err(invalid(
                tree,
                child,
                "unexpected CSS import condition child",
            ));
        };
        if slot.replace(child).is_some() {
            return Err(invalid(tree, child, "duplicate CSS import condition child"));
        }
    }
    for (name, child) in [("supports", supports), ("media", media)] {
        if attribute(tree, id, name).is_some() != child.is_some() {
            return Err(invalid(
                tree,
                id,
                "CSS import condition metadata and typed children disagree",
            ));
        }
    }
    let mut wrappers = Vec::new();
    let mut diagnostics = Vec::new();
    if let Some(supports) = supports {
        let text = components(tree, supports)?;
        if text.is_empty() {
            return Err(invalid(tree, supports, "empty supports condition"));
        }
        let condition = match attribute(tree, supports, "condition-form") {
            Some("declaration") => format!("({text})"),
            Some("condition") => text,
            _ => {
                return Err(invalid(
                    tree,
                    supports,
                    "missing or invalid supports condition form",
                ))
            }
        };
        wrappers.push(wrapper(tree, supports, format!("@supports {condition} {{")));
    }
    if let Some(media) = media {
        let (text, media_diagnostics) = media_condition(tree, media)?;
        diagnostics.extend(media_diagnostics);
        wrappers.push(wrapper(tree, media, format!("@media {text} {{")));
    }
    Ok(CssImportConditionEmission::Emit {
        wrappers,
        diagnostics,
    })
}

fn media_condition(
    tree: &RetainedCemTree,
    media: AstNodeId,
) -> Result<(String, Vec<CssEmissionDiagnostic>), CssEmissionDiagnostic> {
    use crate::css_resources::{attribute, named};
    let mut diagnostics = Vec::new();
    let queries = &tree.node(media).unwrap().children;
    if queries.is_empty() {
        return Err(invalid(tree, media, "empty typed media list"));
    }
    let mut output = Vec::new();
    for &query in queries {
        if !named(tree, query, "media-query") {
            return Err(invalid(tree, query, "expected a typed media query"));
        }
        match attribute(tree, query, "syntax-valid") {
            Some("true") => {
                let text = components(tree, query)?;
                if text.is_empty() {
                    return Err(invalid(tree, query, "empty valid media query"));
                }
                output.push(text);
            }
            Some("false") => {
                output.push("not all".to_owned());
                diagnostics.push(diagnostic(
                    tree,
                    query,
                    "cem.scoped_css.media_query_recovered",
                    "invalid media query emitted as not all",
                ));
            }
            _ => {
                return Err(invalid(
                    tree,
                    query,
                    "missing or invalid media query syntax status",
                ))
            }
        }
    }
    Ok((output.join(", "), diagnostics))
}

fn components(tree: &RetainedCemTree, id: AstNodeId) -> Result<String, CssEmissionDiagnostic> {
    components_with(tree, id, |_, token| Ok(token.to_owned()))
}

fn components_with(
    tree: &RetainedCemTree,
    id: AstNodeId,
    mut emit: impl FnMut(AstNodeId, &str) -> Result<String, CssEmissionDiagnostic>,
) -> Result<String, CssEmissionDiagnostic> {
    use crate::css_resources::{attribute, named};
    let children = &tree.node(id).unwrap().children;
    let mut tokens = Vec::new();
    for &child in children {
        if !named(tree, child, "component-value") {
            return Err(invalid(
                tree,
                child,
                "expected a retained CSS component value",
            ));
        }
        let token = attribute(tree, child, "token")
            .ok_or_else(|| invalid(tree, child, "CSS component has no retained token"))?;
        tokens.push((
            attribute(tree, child, "kind") == Some("whitespace"),
            emit(child, token)?,
        ));
    }
    // Do not String::trim(): an identifier token may end with an escaped space.
    let start = tokens
        .iter()
        .position(|(space, _)| !space)
        .unwrap_or(tokens.len());
    let end = tokens
        .iter()
        .rposition(|(space, _)| !space)
        .map_or(start, |i| i + 1);
    Ok(tokens[start..end]
        .iter()
        .map(|(_, token)| token.as_str())
        .collect())
}

fn wrapper(tree: &RetainedCemTree, id: AstNodeId, opening: String) -> CssConditionWrapper {
    let node = tree.node(id).unwrap();
    CssConditionWrapper {
        opening,
        source: node.source.clone(),
        range: node.range,
    }
}
fn invalid(tree: &RetainedCemTree, id: AstNodeId, message: &str) -> CssEmissionDiagnostic {
    diagnostic(tree, id, "cem.scoped_css.condition_tree_invalid", message)
}
fn diagnostic(
    tree: &RetainedCemTree,
    id: AstNodeId,
    code: &'static str,
    message: &str,
) -> CssEmissionDiagnostic {
    CssEmissionDiagnostic {
        code,
        message: message.to_owned(),
        source: tree.node(id).map(|n| n.source.clone()).unwrap_or_default(),
        range: tree.node(id).map(|n| n.range).unwrap_or_default(),
    }
}
