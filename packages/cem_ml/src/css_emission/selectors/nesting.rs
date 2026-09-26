//! Parent-aware fragments over retained selector nodes. No CSS parsing or flattening.
use super::*;
use crate::css_emission::CssRuleMode;

type Weight = (u32, u32, u32);
type Key = Vec<Option<String>>;
#[derive(Default)]
pub(super) struct ParentContext {
    weight: Weight,
    keys: BTreeSet<Key>,
}

/// Emit a nested selector list using its actual retained ancestors and the same
/// admission policy at every level. Ancestor diagnostics are included. The result
/// must remain nested under those admitted parents; it is not standalone CSS.
pub fn emit_css_nested_selectors(
    tree: &RetainedCemTree,
    rule: AstNodeId,
    mode: CssRuleMode,
) -> Result<CssSelectorEmission, CssEmissionDiagnostic> {
    if attribute(tree, rule, "selector-context") != Some("nested") {
        return Err(invalid(tree, rule));
    }
    compile(tree, rule, mode == CssRuleMode::Instance, 0).map(|(result, _)| result)
}

fn compile(
    tree: &RetainedCemTree,
    rule: AstNodeId,
    instance: bool,
    depth: usize,
) -> Result<(CssSelectorEmission, ParentContext), CssEmissionDiagnostic> {
    if depth >= 64 {
        return Err(invalid(tree, rule));
    }
    let nested = attribute(tree, rule, "selector-context") == Some("nested");
    let (mut ancestors, parent) = if nested {
        compile(tree, parent_rule(tree, rule)?, instance, depth + 1)?
    } else {
        (CssSelectorEmission::default(), ParentContext::default())
    };
    if nested && ancestors.selectors.is_empty() {
        ancestors.diagnostics.push(diagnostic(
            tree,
            rule,
            "cem.scoped_css.nesting_parent_suppressed",
            "no admitted parent selector remains",
        ));
        return Ok((ancestors, ParentContext::default()));
    }
    let mut result = emit_with_parent(tree, rule, instance, nested.then_some(&parent))?;
    let mut context = ParentContext::default();
    for selector in &result.selectors {
        context.weight = context.weight.max(selector.authored_specificity);
        context
            .keys
            .extend(terminal_keys(tree, selector.node_id, &parent));
    }
    ancestors.diagnostics.append(&mut result.diagnostics);
    result.diagnostics = ancestors.diagnostics;
    Ok((result, context))
}

fn parent_rule(
    tree: &RetainedCemTree,
    rule: AstNodeId,
) -> Result<AstNodeId, CssEmissionDiagnostic> {
    let mut current = tree.node(rule).and_then(|n| n.parent);
    for _ in 0..128 {
        let id = current.ok_or_else(|| invalid(tree, rule))?;
        if named(tree, id, "rule") && attribute(tree, id, "kind") == Some("style") {
            return Ok(id);
        }
        // Do not cross a reference-resetting or unknown at-rule boundary.
        if !(named(tree, id, "rule") || named(tree, id, "at-rule"))
            || !attribute(tree, id, "name").is_some_and(|name| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "media" | "supports" | "container" | "layer" | "starting-style"
                )
            })
        {
            return Err(invalid(tree, id));
        }
        current = tree.node(id).and_then(|n| n.parent);
    }
    Err(invalid(tree, rule))
}

fn contains_nesting(tree: &RetainedCemTree, id: AstNodeId) -> bool {
    attribute(tree, id, "kind") == Some("nesting")
        || tree
            .node(id)
            .is_some_and(|n| n.children.iter().any(|c| contains_nesting(tree, *c)))
}

pub(super) fn implicit(tree: &RetainedCemTree, id: AstNodeId) -> bool {
    tree.node(id)
        .and_then(|n| n.children.first())
        .is_some_and(|c| named(tree, *c, "combinator"))
        || !contains_nesting(tree, id)
}

fn add(
    tree: &RetainedCemTree,
    id: AstNodeId,
    a: Weight,
    b: Weight,
) -> Result<Weight, CssEmissionDiagnostic> {
    let overflow = || {
        diagnostic(
            tree,
            id,
            "cem.scoped_css.specificity_overflow",
            "composed selector specificity exceeds native bounds",
        )
    };
    Ok((
        a.0.checked_add(b.0).ok_or_else(overflow)?,
        a.1.checked_add(b.1).ok_or_else(overflow)?,
        a.2.checked_add(b.2).ok_or_else(overflow)?,
    ))
}

pub(super) fn composed_weight(
    tree: &RetainedCemTree,
    id: AstNodeId,
    parent: &ParentContext,
) -> Result<Weight, CssEmissionDiagnostic> {
    let weight = weight(tree, id, parent.weight)?;
    if implicit(tree, id) {
        add(tree, id, weight, parent.weight)
    } else {
        Ok(weight)
    }
}

fn weight(
    tree: &RetainedCemTree,
    id: AstNodeId,
    parent: Weight,
) -> Result<Weight, CssEmissionDiagnostic> {
    let node = tree.node(id).ok_or_else(|| invalid(tree, id))?;
    if named(tree, id, "selector-list") {
        return node
            .children
            .iter()
            .try_fold((0, 0, 0), |w, c| Ok(w.max(weight(tree, *c, parent)?)));
    }
    if named(tree, id, "selector") || named(tree, id, "compound-selector") {
        return node.children.iter().try_fold((0, 0, 0), |w, c| {
            add(tree, id, w, weight(tree, *c, parent)?)
        });
    }
    if named(tree, id, "combinator") {
        return Ok((0, 0, 0));
    }
    if !named(tree, id, "simple-selector") {
        return Err(invalid(tree, id));
    }
    Ok(match attribute(tree, id, "kind") {
        Some("nesting") => parent,
        Some("type" | "pseudo-element") => (0, 0, 1),
        Some("universal") => (0, 0, 0),
        Some("class" | "attribute") => (0, 1, 0),
        Some("id") => (1, 0, 0),
        Some("pseudo-class") => match (attribute(tree, id, "name"), node.children.first()) {
            (Some("where"), _) => (0, 0, 0),
            (Some("is" | "not" | "has"), Some(c)) => weight(tree, *c, parent)?,
            (Some("host" | "global"), Some(c)) => {
                add(tree, id, (0, 1, 0), weight(tree, *c, parent)?)?
            }
            (_, None) => (0, 1, 0),
            _ => return Err(invalid(tree, id)),
        },
        _ => return Err(invalid(tree, id)),
    })
}

fn key(tree: &RetainedCemTree, id: AstNodeId) -> Key {
    ["kind", "namespace", "name", "operator", "value", "modifier"]
        .iter()
        .map(|field| attribute(tree, id, field).map(str::to_owned))
        .collect()
}

fn terminal_keys(
    tree: &RetainedCemTree,
    selector: AstNodeId,
    parent: &ParentContext,
) -> BTreeSet<Key> {
    let Some(compound) = tree.node(selector).and_then(|n| n.children.last()) else {
        return BTreeSet::new();
    };
    // & cannot represent pseudo-elements. Their parent-list specificity still
    // contributes, but their tokens cannot duplicate a matched child's subject.
    if tree.node(*compound).is_some_and(|n| {
        n.children
            .iter()
            .any(|id| attribute(tree, *id, "kind") == Some("pseudo-element"))
    }) {
        return BTreeSet::new();
    }
    tree.node(*compound)
        .into_iter()
        .flat_map(|n| &n.children)
        .flat_map(|id| simple_keys(tree, *id, parent))
        .collect()
}

fn simple_keys(tree: &RetainedCemTree, id: AstNodeId, parent: &ParentContext) -> BTreeSet<Key> {
    match attribute(tree, id, "kind") {
        Some("class" | "attribute") => BTreeSet::from([key(tree, id)]),
        Some("nesting") => parent.keys.clone(),
        Some("pseudo-class")
            if matches!(attribute(tree, id, "name"), Some("is" | "host" | "global")) =>
        {
            tree.node(id)
                .into_iter()
                .flat_map(|n| &n.children)
                .flat_map(|list| &tree.node(*list).unwrap().children)
                .flat_map(|selector| terminal_keys(tree, *selector, parent))
                .collect()
        }
        _ => BTreeSet::new(),
    }
}

// Count parent references intersected on the same subject, saturating at the
// rejection threshold. Alternatives in :is use their maximum, not their sum;
// :where carries no weight and :has refers to a different subject.
fn parent_intersections(tree: &RetainedCemTree, id: AstNodeId) -> u32 {
    match attribute(tree, id, "kind") {
        Some("nesting") => 1,
        Some("pseudo-class")
            if matches!(attribute(tree, id, "name"), Some("is" | "host" | "global")) =>
        {
            tree.node(id)
                .into_iter()
                .flat_map(|n| &n.children)
                .flat_map(|list| &tree.node(*list).unwrap().children)
                .filter_map(|selector| tree.node(*selector)?.children.last())
                .map(|compound| {
                    tree.node(*compound)
                        .unwrap()
                        .children
                        .iter()
                        .map(|simple| parent_intersections(tree, *simple))
                        .sum::<u32>()
                        .min(2)
                })
                .max()
                .unwrap_or(0)
        }
        _ => 0,
    }
}

pub(super) fn check(
    tree: &RetainedCemTree,
    id: AstNodeId,
    parent: &ParentContext,
    bearing: bool,
) -> Result<(), CssEmissionDiagnostic> {
    let node = tree.node(id).ok_or_else(|| invalid(tree, id))?;
    let bearing = bearing
        && !(attribute(tree, id, "kind") == Some("pseudo-class")
            && attribute(tree, id, "name") == Some("where"));
    if bearing && named(tree, id, "compound-selector") {
        let mut keys = BTreeSet::new();
        let mut nesting_count = 0;
        for &child in &node.children {
            nesting_count += parent_intersections(tree, child);
            let incoming = simple_keys(tree, child, parent);
            if (nesting_count > 1 && parent.weight != (0, 0, 0))
                || incoming.iter().any(|k| keys.contains(k))
            {
                return Err(diagnostic(
                    tree,
                    child,
                    "cem.scoped_css.manufactured_specificity_unsupported",
                    "compound repeats inherited specificity-bearing selectors",
                ));
            }
            keys.extend(incoming);
        }
    }
    for &child in &node.children {
        check(tree, child, parent, bearing)?;
    }
    Ok(())
}
