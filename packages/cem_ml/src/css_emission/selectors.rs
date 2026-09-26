//! Managed selector fragments; no source-text parsing or installation.
use super::{diagnostic, CssEmissionDiagnostic};
use crate::{
    css_resources::{attribute, named},
    parser::{
        tree::{CemTreeRange, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
    transform_template::transform_template_encode_css_string,
};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct CssEmittedSelector {
    pub text: String,
    /// Authored weight, before host rewriting. Used for the library ceiling.
    pub authored_specificity: (u32, u32, u32),
    pub source: SourceMapStack,
    pub range: CemTreeRange,
}

#[derive(Debug, Default)]
pub struct CssSelectorEmission {
    /// Accepted top-level selectors in authored order. An empty list omits the rule.
    pub selectors: Vec<CssEmittedSelector>,
    pub diagnostics: Vec<CssEmissionDiagnostic>,
}

/// Emit selector fragments for declaration/private or named shared CSS from a
/// shared-import rule. These still require managed @scope wrapping and validated
/// declarations. This API does not implement instance CSS or accept raw fallback.
pub fn emit_css_declaration_selectors(
    tree: &RetainedCemTree,
    rule: AstNodeId,
) -> Result<CssSelectorEmission, CssEmissionDiagnostic> {
    emit_selectors(tree, rule, false)
}

/// Emit instance selector fragments for a managed implicit @scope. Host aliases
/// become :scope; other top-level selectors receive a :scope descendant prefix.
/// ID/duplicate policy still applies, but the declaration specificity ceiling
/// does not. The caller must validate declarations and apply the scope limits.
pub fn emit_css_instance_selectors(
    tree: &RetainedCemTree,
    rule: AstNodeId,
) -> Result<CssSelectorEmission, CssEmissionDiagnostic> {
    emit_selectors(tree, rule, true)
}

fn emit_selectors(
    tree: &RetainedCemTree,
    rule: AstNodeId,
    instance: bool,
) -> Result<CssSelectorEmission, CssEmissionDiagnostic> {
    if !named(tree, rule, "rule") || attribute(tree, rule, "kind") != Some("style") {
        return Err(invalid(tree, rule));
    }
    if attribute(tree, rule, "selector-context") == Some("nested") {
        return Ok(CssSelectorEmission {
            selectors: Vec::new(),
            diagnostics: vec![diagnostic(
                tree,
                rule,
                "cem.scoped_css.nesting_context_required",
                "nested selectors require an enclosing style-rule context",
            )],
        });
    }
    let lists: Vec<_> = tree
        .node(rule)
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|id| named(tree, *id, "selector-list"))
        .collect();
    if lists.len() != 1 {
        return Err(invalid(tree, rule));
    }
    let list = lists[0];
    let mut result = CssSelectorEmission::default();
    match attribute(tree, list, "analysis-status") {
        Some("unsupported") => {
            result.diagnostics.push(diagnostic(
                tree,
                list,
                "cem.scoped_css.selector_unsupported",
                "selector structure is outside the retained profile",
            ));
            return Ok(result);
        }
        Some("complete") => {}
        _ => return Err(invalid(tree, list)),
    }
    if tree.node(list).unwrap().children.is_empty() {
        return Err(invalid(tree, list));
    }
    for &id in &tree.node(list).unwrap().children {
        if !named(tree, id, "selector") {
            return Err(invalid(tree, id));
        }
        let weight = specificity(tree, id)?;
        if let Err(d) = policy(tree, id, true) {
            result.diagnostics.push(d);
            continue;
        }
        if !instance && weight > (0, 2, 1) {
            result.diagnostics.push(diagnostic(
                tree,
                id,
                "cem.scoped_css.specificity_unsupported",
                "authored declaration/shared selector exceeds 0-2-1",
            ));
            continue;
        }
        let mut aliases = Vec::new();
        match emit(tree, id, instance, &mut aliases) {
            Ok(mut text) => {
                if instance && !starts_with_scope(tree, id) {
                    text = format!(":scope {text}");
                }
                let node = tree.node(id).unwrap();
                result.selectors.push(CssEmittedSelector {
                    text,
                    authored_specificity: weight,
                    source: node.source.clone(),
                    range: node.range,
                });
                result.diagnostics.extend(aliases);
            }
            Err(d) => result.diagnostics.push(d),
        }
    }
    Ok(result)
}

// Mirror the existing instance contract using retained structure, not a regex:
// only an initial explicit scope/host alias avoids the top-level prefix. A host
// nested inside :is/:where/:not does not change that decision.
fn starts_with_scope(tree: &RetainedCemTree, selector: AstNodeId) -> bool {
    let Some(&compound) = tree.node(selector).and_then(|n| n.children.first()) else {
        return false;
    };
    let Some(&simple) = tree.node(compound).and_then(|n| n.children.first()) else {
        return false;
    };
    named(tree, compound, "compound-selector")
        && named(tree, simple, "simple-selector")
        && attribute(tree, simple, "kind") == Some("pseudo-class")
        && matches!(
            attribute(tree, simple, "name"),
            Some("host" | "scope" | "root" | "global")
        )
}

fn specificity(
    tree: &RetainedCemTree,
    id: AstNodeId,
) -> Result<(u32, u32, u32), CssEmissionDiagnostic> {
    let parts: Vec<_> = attribute(tree, id, "specificity")
        .unwrap_or("")
        .split('-')
        .collect();
    if parts.len() != 3 {
        return Err(invalid(tree, id));
    }
    let number = |s: &str| s.parse::<u32>().map_err(|_| invalid(tree, id));
    Ok((number(parts[0])?, number(parts[1])?, number(parts[2])?))
}

fn policy(
    tree: &RetainedCemTree,
    id: AstNodeId,
    bearing: bool,
) -> Result<(), CssEmissionDiagnostic> {
    let node = tree.node(id).ok_or_else(|| invalid(tree, id))?;
    if named(tree, id, "simple-selector") && attribute(tree, id, "kind") == Some("id") {
        return Err(diagnostic(
            tree,
            id,
            "cem.scoped_css.id_selector_unsupported",
            "managed selectors cannot contain IDs",
        ));
    }
    let bearing = bearing
        && !(named(tree, id, "simple-selector")
            && attribute(tree, id, "kind") == Some("pseudo-class")
            && attribute(tree, id, "name") == Some("where"));
    let mut tokens = BTreeSet::new();
    for &child in &node.children {
        if bearing
            && named(tree, id, "compound-selector")
            && matches!(attribute(tree, child, "kind"), Some("class" | "attribute"))
        {
            // Decoded semantic identity catches equivalent escape/quote spellings.
            let key: Vec<_> = ["kind", "namespace", "name", "operator", "value", "modifier"]
                .iter()
                .map(|name| attribute(tree, child, name))
                .collect();
            if !tokens.insert(key) {
                return Err(diagnostic(
                    tree,
                    child,
                    "cem.scoped_css.manufactured_specificity_unsupported",
                    "compound repeats a specificity-bearing class or attribute",
                ));
            }
        }
        policy(tree, child, bearing)?;
    }
    Ok(())
}

fn emit(
    tree: &RetainedCemTree,
    id: AstNodeId,
    instance: bool,
    diagnostics: &mut Vec<CssEmissionDiagnostic>,
) -> Result<String, CssEmissionDiagnostic> {
    let node = tree.node(id).ok_or_else(|| invalid(tree, id))?;
    if named(tree, id, "selector-list") {
        if attribute(tree, id, "analysis-status") != Some("complete") {
            return Err(invalid(tree, id));
        }
        return node
            .children
            .iter()
            .map(|child| emit(tree, *child, instance, diagnostics))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| items.join(", "));
    }
    if named(tree, id, "selector") || named(tree, id, "compound-selector") {
        return node
            .children
            .iter()
            .map(|child| emit(tree, *child, instance, diagnostics))
            .collect();
    }
    if named(tree, id, "combinator") {
        return Ok(match attribute(tree, id, "kind") {
            Some("descendant") => " ",
            Some("child") => " > ",
            Some("next-sibling") => " + ",
            Some("subsequent-sibling") => " ~ ",
            _ => return Err(invalid(tree, id)),
        }
        .to_owned());
    }
    if !named(tree, id, "simple-selector") {
        return Err(invalid(tree, id));
    }
    let field = |name| attribute(tree, id, name).ok_or_else(|| invalid(tree, id));
    match field("kind")? {
        "class" => Ok(format!(".{}", ident(field("value")?))),
        "type" | "universal" => {
            let prefix = match field("namespace")? {
                "*" => "*|",
                "" => "|",
                _ => return Err(invalid(tree, id)),
            };
            let name = if field("kind")? == "universal" {
                "*".to_owned()
            } else {
                ident(field("name")?)
            };
            Ok(format!("{prefix}{name}"))
        }
        "attribute" => {
            let prefix = match field("namespace")? {
                "*" => "*|",
                "" => "",
                _ => return Err(invalid(tree, id)),
            };
            let mut text = format!("[{prefix}{}", ident(field("name")?));
            if let Some(op) = attribute(tree, id, "operator") {
                if !matches!(op, "=" | "~=" | "|=" | "^=" | "$=" | "*=") {
                    return Err(invalid(tree, id));
                }
                text.push_str(op);
                text.push_str(&transform_template_encode_css_string(field("value")?));
                if let Some(modifier) = attribute(tree, id, "modifier") {
                    if !matches!(modifier, "i" | "s") {
                        return Err(invalid(tree, id));
                    }
                    text.push(' ');
                    text.push_str(modifier);
                }
            }
            text.push(']');
            Ok(text)
        }
        "pseudo-element" => Ok(format!("::{}", ident(field("name")?))),
        "pseudo-class" => {
            let name = field("name")?;
            if matches!(name, "host" | "root" | "global") {
                if name != "host" {
                    diagnostics.push(diagnostic(
                        tree,
                        id,
                        "cem.scoped_css.global_alias",
                        "global selector contained as a host alias",
                    ));
                }
                let mut output = if instance { ":scope" } else { ":where(:scope)" }.to_owned();
                if let Some(&list) = node.children.first() {
                    let arg = emit(tree, list, instance, diagnostics)?;
                    // A type/universal argument cannot follow a pseudo-class in
                    // a compound. :is preserves its weight and intersection.
                    let has_type = tree.node(list).unwrap().children.iter().any(|s| {
                        tree.node(*s).unwrap().children.iter().any(|c| {
                            tree.node(*c).unwrap().children.iter().any(|v| {
                                matches!(attribute(tree, *v, "kind"), Some("type" | "universal"))
                            })
                        })
                    });
                    if has_type {
                        output.push_str(&format!(":is({arg})"));
                    } else {
                        output.push_str(&arg);
                    }
                }
                Ok(output)
            } else if let Some(&list) = node.children.first() {
                Ok(format!(
                    ":{}({})",
                    ident(name),
                    emit(tree, list, instance, diagnostics)?
                ))
            } else {
                Ok(format!(":{}", ident(name)))
            }
        }
        _ => Err(invalid(tree, id)),
    }
}

// CSS identifier serialization, including escaped leading digits and -digit.
pub(super) fn ident(value: &str) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        let escape = ch.is_control()
            || !(ch.is_alphanumeric() || ch == '_' || ch == '-' || !ch.is_ascii())
            || (ch.is_ascii_digit() && (index == 0 || (index == 1 && value.starts_with('-'))))
            || (ch == '-' && value == "-");
        if escape {
            output.push_str(&format!("\\{:X} ", ch as u32));
        } else {
            output.push(ch);
        }
    }
    output
}
fn invalid(tree: &RetainedCemTree, id: AstNodeId) -> CssEmissionDiagnostic {
    diagnostic(
        tree,
        id,
        "cem.scoped_css.selector_tree_invalid",
        "missing or inconsistent retained selector structure",
    )
}
