//! Bounded author rules over shared-import retained CSS. No selector reparsing.
use super::*;
use crate::css_resources::{attribute as css_attr, named};
use std::collections::{BTreeMap, BTreeSet};
type Specificity = (u32, u32, u32);
type Inline = BTreeMap<(AstNodeId, String), (Expression, PaintDeclarationWinner)>;
struct Selector {
    id: AstNodeId,
    simples: Vec<AstNodeId>,
    specificity: Specificity,
}
struct Declaration {
    id: AstNodeId,
    name: String,
    value: Expression,
    important: bool,
    order: usize,
}
struct Rule {
    owner: AstNodeId,
    css: Arc<RetainedCemTree>,
    id: AstNodeId,
    selectors: Vec<Selector>,
    declarations: Vec<Declaration>,
    sheet_order: usize,
    order: usize,
}
pub(super) struct Cascade {
    inline: Inline,
    rules: Vec<Rule>,
    pub sources: BTreeSet<AstNodeId>,
}
impl Cascade {
    pub fn new(plan: &SvgPaintStylePlan, work: &mut Work) -> Result<Self, String> {
        let inv = plan.content().paint().inventory();
        Self::from_sources(&inv.document().tree, inv.stylesheets(), work)
    }
    pub(super) fn from_sources(
        doc: &Arc<RetainedCemTree>,
        sources: &[crate::document_stylesheet_dependencies::DocumentStylesheetOccurrence],
        work: &mut Work,
    ) -> Result<Self, String> {
        // Explicit document order, independent of allocation IDs and inventory order.
        let mut positions = BTreeMap::new();
        let mut pending = vec![0];
        while let Some(id) = pending.pop() {
            work.spend(1)?;
            positions.insert(id, positions.len());
            pending.extend(
                doc.node(id)
                    .ok_or("document node missing")?
                    .children
                    .iter()
                    .rev(),
            );
        }
        let mut sheets: Vec<_> = sources.iter().collect();
        sheets.sort_by_key(|s| positions.get(&s.owner).copied().unwrap_or(usize::MAX));
        let mut result = Self {
            inline: BTreeMap::new(),
            rules: Vec::new(),
            sources: BTreeSet::new(),
        };
        let (mut sheet_count, mut rule_count, mut branch_count, mut decl_count) = (0, 0, 0, 0);
        for sheet in sheets {
            work.spend(1)?;
            local(doc, sheet.owner)?;
            if !positions.contains_key(&sheet.owner) || !result.sources.insert(sheet.source_node) {
                return Err("duplicate or detached paint CSS source".into());
            }
            let source = doc
                .node(sheet.source_node)
                .ok_or("paint CSS source missing")?;
            let inline = source.kind == CemTreeNodeKind::Attribute;
            if inline {
                if source.parent != Some(sheet.owner)
                    || !source
                        .name
                        .as_ref()
                        .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == "style")
                {
                    return Err("paint inline owner mismatch".into());
                }
            } else {
                if source.kind != CemTreeNodeKind::Element
                    || sheet.owner != sheet.source_node
                    || local(doc, sheet.owner)? != "style"
                {
                    return Err("paint stylesheet owner mismatch".into());
                }
                for attr in ["title", "media"] {
                    if attribute(doc, sheet.owner, attr, work)?.is_some() {
                        return Err("paint titled or media stylesheet unsupported".into());
                    }
                }
                sheet_count += 1;
                if sheet_count > 256 {
                    return Err("paint stylesheet bound exceeded".into());
                }
            }
            let css = &sheet
                .resources
                .as_ref()
                .map_err(|error| format!("paint CSS import failed: {error}"))?
                .tree;
            let roots = &css.node(0).ok_or("paint CSS document missing")?.children;
            if roots.len() != 1
                || !named(
                    css,
                    roots[0],
                    if inline {
                        "style-attribute"
                    } else {
                        "stylesheet"
                    },
                )
            {
                return Err("paint CSS root unsupported".into());
            }
            let children = &css.node(roots[0]).unwrap().children;
            if inline {
                for d in declarations(css, children, &mut decl_count, work)? {
                    let key = (sheet.owner, d.name);
                    if result
                        .inline
                        .get(&key)
                        .is_none_or(|(_, old)| !old.important || d.important)
                    {
                        result.inline.insert(
                            key,
                            (
                                d.value,
                                PaintDeclarationWinner {
                                    origin: PaintDeclarationOrigin::Inline {
                                        attribute: sheet.source_node,
                                        css: css.clone(),
                                        declaration: d.id,
                                    },
                                    important: d.important,
                                    order: d.order,
                                },
                            ),
                        );
                    }
                }
            } else {
                let mut order = 0;
                for id in children {
                    work.spend(1)?;
                    if named(css, *id, "comment") {
                        continue;
                    }
                    if !named(css, *id, "rule")
                        || css_attr(css, *id, "kind") != Some("style")
                        || css_attr(css, *id, "selector-context") != Some("root")
                    {
                        return Err("paint stylesheet rule unsupported".into());
                    }
                    rule_count += 1;
                    if rule_count > 1024 {
                        return Err("paint rule bound exceeded".into());
                    }
                    let parts = &css.node(*id).unwrap().children;
                    let lists: Vec<_> = parts
                        .iter()
                        .copied()
                        .filter(|n| named(css, *n, "selector-list"))
                        .collect();
                    if lists.len() != 1 {
                        return Err("paint selector list missing".into());
                    }
                    let selectors = selectors(css, lists[0], &mut branch_count, work)?;
                    let body: Vec<_> = parts.iter().copied().filter(|n| *n != lists[0]).collect();
                    result.rules.push(Rule {
                        owner: sheet.owner,
                        css: css.clone(),
                        id: *id,
                        selectors,
                        declarations: declarations(css, &body, &mut decl_count, work)?,
                        sheet_order: sheet_count - 1,
                        order,
                    });
                    order += 1;
                }
            }
        }
        Ok(result)
    }
    pub fn winner(
        &self,
        tree: &RetainedCemTree,
        owner: AstNodeId,
        property: &str,
        work: &mut Work,
    ) -> Result<Option<(Expression, PaintDeclarationWinner)>, String> {
        work.spend(1)?;
        let mut selected = self.inline.get(&(owner, property.to_owned())).cloned();
        for rule in &self.rules {
            work.spend(1)?;
            let mut matched = Vec::new();
            let mut specificity = (0, 0, 0);
            for selector in &rule.selectors {
                if matches_selector(&rule.css, selector, tree, owner, work)? {
                    matched.push(selector.id);
                    specificity = specificity.max(selector.specificity);
                }
            }
            if matched.is_empty() {
                continue;
            }
            for d in &rule.declarations {
                work.spend(1)?;
                if d.name != property {
                    continue;
                }
                work.spend(matched.len())?;
                let winner = PaintDeclarationWinner {
                    origin: PaintDeclarationOrigin::Rule {
                        owner: rule.owner,
                        css: rule.css.clone(),
                        rule: rule.id,
                        declaration: d.id,
                        selectors: matched.clone(),
                        specificity,
                        sheet_order: rule.sheet_order,
                        rule_order: rule.order,
                    },
                    important: d.important,
                    order: d.order,
                };
                if selected
                    .as_ref()
                    .is_none_or(|(_, old)| priority(&winner) > priority(old))
                {
                    selected = Some((d.value.clone(), winner));
                }
            }
        }
        if selected.is_some() {
            return Ok(selected);
        }
        let Some(id) = attribute(tree, owner, property, work)? else {
            return Ok(None);
        };
        Ok(Some((
            expression(&tree.node(id).unwrap().value)?,
            PaintDeclarationWinner {
                origin: PaintDeclarationOrigin::Presentation { attribute: id },
                important: false,
                order: 0,
            },
        )))
    }
}
fn priority(w: &PaintDeclarationWinner) -> (bool, bool, Specificity, usize, usize, usize) {
    match &w.origin {
        PaintDeclarationOrigin::Rule {
            specificity,
            sheet_order,
            rule_order,
            ..
        } => (
            w.important,
            false,
            *specificity,
            *sheet_order,
            *rule_order,
            w.order,
        ),
        PaintDeclarationOrigin::Inline { .. } => (w.important, true, (0, 0, 0), 0, 0, w.order),
        PaintDeclarationOrigin::Presentation { .. } => (false, false, (0, 0, 0), 0, 0, 0),
    }
}
fn declarations(
    css: &RetainedCemTree,
    ids: &[AstNodeId],
    count: &mut usize,
    work: &mut Work,
) -> Result<Vec<Declaration>, String> {
    let mut out = Vec::new();
    for id in ids {
        work.spend(1)?;
        if named(css, *id, "comment") {
            continue;
        }
        if !named(css, *id, "declaration") {
            return Err("paint CSS declaration structure unsupported".into());
        }
        *count += 1;
        if *count > 4096 {
            return Err("paint CSS declaration bound exceeded".into());
        }
        let name = css_attr(css, *id, "name").ok_or("paint property missing")?;
        let value = css_attr(css, *id, "value").ok_or("paint value missing")?;
        work.spend(name.len() + value.len() + css.node(*id).unwrap().attributes.len())?;
        let name = name.to_ascii_lowercase();
        if !matches!(name.as_str(), "color" | "fill" | "stop-color") {
            return Err("paint CSS property unsupported".into());
        }
        out.push(Declaration {
            id: *id,
            name,
            value: expression(value)?,
            important: css_attr(css, *id, "important") == Some("true"),
            order: out.len(),
        });
    }
    Ok(out)
}
fn selectors(
    css: &RetainedCemTree,
    list: AstNodeId,
    count: &mut usize,
    work: &mut Work,
) -> Result<Vec<Selector>, String> {
    if css_attr(css, list, "analysis-status") != Some("complete") {
        return Err("paint selector analysis incomplete".into());
    }
    let mut out = Vec::new();
    for id in &css.node(list).unwrap().children {
        work.spend(1)?;
        *count += 1;
        if *count > 4096 {
            return Err("paint selector bound exceeded".into());
        }
        let n = css.node(*id).ok_or("paint selector missing")?;
        if !named(css, *id, "selector")
            || n.children.len() != 1
            || !named(css, n.children[0], "compound-selector")
        {
            return Err("paint compound selector required".into());
        }
        let simples = &css.node(n.children[0]).unwrap().children;
        if simples.is_empty() || simples.len() > 64 {
            return Err("paint simple selector bound exceeded".into());
        }
        let mut specificity = (0, 0, 0);
        for simple in simples {
            work.spend(1)?;
            let sn = css.node(*simple).ok_or("paint simple selector missing")?;
            if !named(css, *simple, "simple-selector") || !sn.children.is_empty() {
                return Err("paint simple selector unsupported".into());
            }
            for attr in &sn.attributes {
                work.spend(
                    1 + css
                        .node(*attr)
                        .ok_or("selector attribute missing")?
                        .value
                        .len(),
                )?;
            }
            match css_attr(css, *simple, "kind") {
                Some("type" | "universal") => {
                    if !matches!(css_attr(css, *simple, "namespace"), Some("*" | ""))
                        || css_attr(css, *simple, "name").is_none()
                    {
                        return Err("paint selector namespace unsupported".into());
                    }
                    if css_attr(css, *simple, "kind") == Some("type") {
                        specificity.2 += 1;
                    }
                }
                Some("id" | "class") => {
                    if css_attr(css, *simple, "value").is_none() {
                        return Err("paint selector value missing".into());
                    }
                    if css_attr(css, *simple, "kind") == Some("id") {
                        specificity.0 += 1;
                    } else {
                        specificity.1 += 1;
                    }
                }
                _ => return Err("paint simple selector unsupported".into()),
            }
        }
        let expected = format!("{}-{}-{}", specificity.0, specificity.1, specificity.2);
        if css_attr(css, *id, "specificity") != Some(expected.as_str())
            || css_attr(css, *id, "specificity-kind").is_some()
        {
            return Err("paint selector specificity mismatch".into());
        }
        out.push(Selector {
            id: *id,
            simples: simples.clone(),
            specificity,
        });
    }
    if out.is_empty() {
        return Err("paint selector list empty".into());
    }
    Ok(out)
}
fn matches_selector(
    css: &RetainedCemTree,
    s: &Selector,
    doc: &RetainedCemTree,
    owner: AstNodeId,
    work: &mut Work,
) -> Result<bool, String> {
    work.spend(1)?;
    let name = local(doc, owner)?;
    for id in &s.simples {
        work.spend(1)?;
        match css_attr(css, *id, "kind").unwrap() {
            "type" | "universal" => {
                work.spend(name.len())?;
                if css_attr(css, *id, "namespace") == Some("")
                    || (css_attr(css, *id, "kind") == Some("type")
                        && css_attr(css, *id, "name") != Some(name))
                {
                    return Ok(false);
                }
            }
            kind => {
                let key = if kind == "id" { "id" } else { "class" };
                let value = css_attr(css, *id, "value").unwrap();
                work.spend(value.len())?;
                let Some(attr) = attribute(doc, owner, key, work)? else {
                    return Ok(false);
                };
                let actual = &doc.node(attr).unwrap().value;
                // CSS ASCII whitespace excludes vertical tab and nonbreaking space.
                let matches = if kind == "id" {
                    actual == value
                } else {
                    actual
                        .split([' ', '\t', '\n', '\r', '\u{c}'])
                        .any(|v| v == value)
                };
                if !matches {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}
