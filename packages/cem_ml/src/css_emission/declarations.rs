//! Direct declaration fragments. Resource URLs and nested rules still require
//! full managed compilation; these fragments are not installable stylesheets.
use super::{components, diagnostic, selectors::ident, CssEmissionDiagnostic};
use crate::{
    css_resources::{attribute, named},
    parser::{
        tree::{CemTreeRange, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
};

#[derive(Debug)]
pub struct CssEmittedDeclaration {
    pub node_id: AstNodeId,
    pub text: String,
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug, Default)]
pub struct CssRuleDeclarations {
    pub declarations: Vec<CssEmittedDeclaration>,
    pub diagnostics: Vec<CssEmissionDiagnostic>,
    /// Nested rules/at-rules/imports are not silently flattened or discarded.
    /// The complete compiler must traverse the original rule's child order;
    /// never concatenate declaration fragments across these deferred children.
    pub deferred_children: Vec<AstNodeId>,
}

/// Emit direct declarations for a retained style rule, for both declaration and
/// instance CSS. Suppress !important and recovered value tokens. Ordinary value
/// grammar remains browser-owned. URL tokens are still authored/unresolved; the
/// caller must rewrite resources and compile deferred children before use.
/// Raw declaration @value is metadata, never a serialization fallback.
pub fn emit_css_rule_declarations(
    tree: &RetainedCemTree,
    rule: AstNodeId,
) -> Result<CssRuleDeclarations, CssEmissionDiagnostic> {
    if !named(tree, rule, "rule") || attribute(tree, rule, "kind") != Some("style") {
        return Err(invalid(tree, rule));
    }
    let mut result = CssRuleDeclarations::default();
    for &id in &tree.node(rule).unwrap().children {
        if named(tree, id, "selector-list") || named(tree, id, "comment") {
            continue;
        }
        if ["rule", "at-rule", "import"]
            .iter()
            .any(|name| named(tree, id, name))
        {
            result.deferred_children.push(id);
            continue;
        }
        if !named(tree, id, "declaration") {
            return Err(invalid(tree, id));
        }
        match attribute(tree, id, "important") {
            Some("true") => {
                result.diagnostics.push(diagnostic(
                    tree,
                    id,
                    "cem.scoped_css.important_unsupported",
                    "managed CSS suppresses declarations marked !important",
                ));
                continue;
            }
            None | Some("false") => {}
            _ => return Err(invalid(tree, id)),
        }
        let name = attribute(tree, id, "name")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| invalid(tree, id))?;
        let node = tree.node(id).unwrap();
        if node.children.is_empty() && attribute(tree, id, "value") != Some("") {
            return Err(invalid(tree, id));
        }
        if let Some(bad) = recovered_value(tree, id) {
            result.diagnostics.push(diagnostic(
                tree,
                bad,
                "cem.scoped_css.declaration_value_unsupported",
                "managed CSS suppresses a recovered declaration value",
            ));
            continue;
        }
        let value = components(tree, id).map_err(|_| invalid(tree, id))?;
        if value.is_empty() && attribute(tree, id, "custom-property") != Some("true") {
            result.diagnostics.push(diagnostic(
                tree,
                id,
                "cem.scoped_css.declaration_value_unsupported",
                "empty ordinary declaration value",
            ));
            continue;
        }
        result.declarations.push(CssEmittedDeclaration {
            node_id: id,
            text: format!("{}:{value};", ident(name)),
            source: node.source.clone(),
            range: node.range,
        });
    }
    Ok(result)
}

fn recovered_value(tree: &RetainedCemTree, id: AstNodeId) -> Option<AstNodeId> {
    if named(tree, id, "component-value") && attribute(tree, id, "kind") == Some("unknown") {
        return Some(id);
    }
    tree.node(id)?
        .children
        .iter()
        .find_map(|child| recovered_value(tree, *child))
}
fn invalid(tree: &RetainedCemTree, id: AstNodeId) -> CssEmissionDiagnostic {
    diagnostic(
        tree,
        id,
        "cem.scoped_css.declaration_tree_invalid",
        "missing or inconsistent retained declaration structure",
    )
}
