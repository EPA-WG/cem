//! Bounded author-color derivations. Never a full cascade, used-paint result or
//! execution seal. The original inventory and explicit compatibility profile stay
//! retained; browser confirmation and live validation are separate obligations.
use crate::{
    parser::{
        tree::{CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    svg_paint_clone_profile::SvgPaintCloneProfile,
    svg_paint_style::{PaintContentSelection, SvgPaintStylePlan},
};
use cssparser::{Parser, ParserInput, Token};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
const SVG: &str = "http://www.w3.org/2000/svg";

#[derive(Clone)]
pub struct PaintDeclarationWinner {
    /// Document presentation/style attribute, never a CSS-local node ID.
    pub attribute: AstNodeId,
    pub css: Option<Arc<RetainedCemTree>>,
    pub css_declaration: Option<AstNodeId>,
    pub important: bool,
    /// Zero-based declaration order within this style attribute, excluding comments.
    pub order: usize,
}
pub struct PaintColorDerivation {
    pub host: AstNodeId,
    pub source: AstNodeId,
    pub paint_declaration: AstNodeId,
    pub property: &'static str,
    pub paint_winner: PaintDeclarationWinner,
    pub color_winners: Vec<PaintDeclarationWinner>,
    pub rgba: [u8; 4],
    /// Nearest first, including currentColor forwarding declarations.
    pub color_declarations: Vec<AstNodeId>,
    pub inheritance_nodes: Vec<AstNodeId>,
}
pub struct BoundedPaintColors {
    plan: Arc<SvgPaintStylePlan>,
    profile: SvgPaintCloneProfile,
    colors: Vec<PaintColorDerivation>,
    suppressed: Vec<AstNodeId>,
}
impl BoundedPaintColors {
    pub fn plan(&self) -> &Arc<SvgPaintStylePlan> {
        &self.plan
    }
    pub fn profile(&self) -> SvgPaintCloneProfile {
        self.profile
    }
    pub fn colors(&self) -> &[PaintColorDerivation] {
        &self.colors
    }
    pub fn suppressed_hosts(&self) -> &[AstNodeId] {
        &self.suppressed
    }
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-paint-color-browser-parity-required",
            "svg-paint-clone-execution-evidence-required",
            "svg-computed-winning-source-required",
            "svg-paint-rendering-evidence-required",
            "svg-computed-resource-acquisition-required",
        ]
    }
    pub fn evaluate(
        plan: Arc<SvgPaintStylePlan>,
        profile: SvgPaintCloneProfile,
        budget: usize,
    ) -> Result<Self, String> {
        let tree = &plan.content().paint().inventory().document().tree;
        let mut work = Work(budget.min(1_000_000));
        let (inline, inline_attributes) = inline_declarations(&plan, &mut work)?;
        // A deliberately strict author-only profile: no hidden competing author
        // cascade, scripted mutation, animation or template insertion.
        let mut pending = vec![0];
        while let Some(id) = pending.pop() {
            work.spend(1)?;
            let n = tree.node(id).ok_or("paint color node missing")?;
            work.spend(n.value.len())?;
            if n.kind == CemTreeNodeKind::ProcessingInstruction {
                return Err("paint color processing instructions unsupported".into());
            }
            if n.name.as_ref().is_some_and(|n| {
                matches!(
                    n.local_name.as_str(),
                    "style"
                        | "link"
                        | "script"
                        | "template"
                        | "animate"
                        | "animateMotion"
                        | "animateTransform"
                        | "set"
                        | "discard"
                )
            }) {
                return Err("paint color cascade or dynamic content unsupported".into());
            }
            for id in &n.attributes {
                work.spend(1)?;
                let a = tree.node(*id).ok_or("paint color attribute missing")?;
                work.spend(a.value.len())?;
                if a.name.as_ref().is_some_and(|n| {
                    n.local_name.starts_with("on")
                        || (n.local_name == "style" && !inline_attributes.contains(id))
                }) {
                    return Err(
                        "paint color unretained inline style or event handler unsupported".into(),
                    );
                }
            }
            pending.extend(n.children.iter().rev().copied());
        }
        let mut colors = Vec::new();
        let mut suppressed = Vec::new();
        for host in plan.hosts() {
            work.spend(1)?;
            let source = match host.selection {
                PaintContentSelection::Own => host.host,
                PaintContentSelection::Template(id) if host.template_attributes.len() == 1 => id,
                _ => return Err("paint color selection outside flat local profile".into()),
            };
            let host_kind = local(tree, host.host)?;
            if local(tree, source)? != host_kind {
                return Err("paint color cross-kind template unsupported".into());
            }
            if source != host.host && host_kind == "pattern" {
                let own = elements(tree, host.host, &mut work)?;
                if !own.is_empty() {
                    if own.len() != 1 || local(tree, own[0])? != "title" {
                        return Err("paint color descriptive pattern outside profile".into());
                    }
                    suppressed.push(host.host);
                    continue;
                }
            }
            let children = elements(tree, source, &mut work)?;
            if children.len() != 1 {
                return Err("paint color requires one flat source child".into());
            }
            let child = children[0];
            let property = match (host_kind, local(tree, child)?) {
                ("linearGradient" | "radialGradient", "stop") => "stop-color",
                ("pattern", "rect") => "fill",
                _ => return Err("paint color source child unsupported".into()),
            };
            if !elements(tree, child, &mut work)?.is_empty() {
                return Err("paint color nested content unsupported".into());
            }
            let (paint_expression, paint_winner) =
                winner(tree, &inline, child, property, &mut work)?
                    .ok_or("paint color requires explicit paint declaration")?;
            let declaration = paint_winner.attribute;
            let mut color_declarations = Vec::new();
            let mut color_winners = Vec::new();
            let mut inheritance_nodes = Vec::new();
            let rgba = match paint_expression {
                Expression::Literal(value) => value,
                Expression::Current => {
                    // The pinned Chromium candidate model uses source ancestry
                    // for inherited content. Do not rewrite the SVG 2 plan.
                    let mut current = Some(child);
                    loop {
                        work.spend(1)?;
                        let id = current.ok_or("paint color requires explicit inherited color")?;
                        if inheritance_nodes.len() >= 128 {
                            return Err("paint color inheritance bound exceeded".into());
                        }
                        inheritance_nodes.push(id);
                        let n = tree.node(id).ok_or("paint color ancestor missing")?;
                        if n.name.as_ref().is_some_and(|n| n.namespace_uri == SVG) {
                            if let Some((value, selected)) =
                                winner(tree, &inline, id, "color", &mut work)?
                            {
                                color_declarations.push(selected.attribute);
                                color_winners.push(selected);
                                match value {
                                    Expression::Literal(value) => break value,
                                    Expression::Current => {}
                                }
                            }
                        }
                        current = n.parent;
                    }
                }
            };
            colors.push(PaintColorDerivation {
                host: host.host,
                source: child,
                paint_declaration: declaration,
                property,
                paint_winner,
                color_winners,
                rgba,
                color_declarations,
                inheritance_nodes,
            });
        }
        Ok(Self {
            plan,
            profile,
            colors,
            suppressed,
        })
    }
}
type InlineWinners = BTreeMap<(AstNodeId, String), (Expression, PaintDeclarationWinner)>;
fn inline_declarations(
    plan: &SvgPaintStylePlan,
    work: &mut Work,
) -> Result<(InlineWinners, BTreeSet<AstNodeId>), String> {
    use crate::css_resources::{attribute as css_attr, named};
    let inventory = plan.content().paint().inventory();
    let document = &inventory.document().tree;
    let mut winners: InlineWinners = BTreeMap::new();
    let mut attributes = BTreeSet::new();
    let mut count = 0;
    for sheet in inventory.stylesheets() {
        work.spend(1)?;
        local(document, sheet.owner)?; // only SVG inline declarations in this profile
        let source = document
            .node(sheet.source_node)
            .ok_or("inline source missing")?;
        if !source
            .name
            .as_ref()
            .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == "style")
            || source.kind != CemTreeNodeKind::Attribute
        {
            return Err("paint color stylesheet cascade unsupported".into());
        }
        if !attributes.insert(sheet.source_node) {
            return Err("duplicate inline source owner".into());
        }
        let css = &sheet
            .resources
            .as_ref()
            .map_err(|_| "inline CSS import failed")?
            .tree;
        let roots = &css.node(0).ok_or("inline CSS document missing")?.children;
        if roots.len() != 1 || !named(css, roots[0], "style-attribute") {
            return Err("paint color requires retained inline declaration list".into());
        }
        let mut order = 0;
        for id in &css.node(roots[0]).unwrap().children {
            work.spend(1)?;
            if named(css, *id, "comment") {
                continue;
            }
            if !named(css, *id, "declaration") {
                return Err("paint color inline rule unsupported".into());
            }
            count += 1;
            if count > 4096 {
                return Err("paint color inline declaration bound exceeded".into());
            }
            let name = css_attr(css, *id, "name")
                .ok_or("inline property missing")?
                .to_ascii_lowercase();
            let value = css_attr(css, *id, "value").ok_or("inline value missing")?;
            work.spend(name.len() + value.len() + css.node(*id).unwrap().attributes.len())?;
            if !matches!(name.as_str(), "color" | "fill" | "stop-color") {
                return Err("paint color inline property unsupported".into());
            }
            // Unsupported/invalid values fail the bounded profile even when
            // losing; never reinterpret them as absent browser declarations.
            let value = expression(value)?;
            let important = css_attr(css, *id, "important") == Some("true");
            let key = (sheet.owner, name);
            let replace = winners
                .get(&key)
                .is_none_or(|(_, previous)| !previous.important || important);
            if replace {
                winners.insert(
                    key,
                    (
                        value,
                        PaintDeclarationWinner {
                            attribute: sheet.source_node,
                            css: Some(css.clone()),
                            css_declaration: Some(*id),
                            important,
                            order,
                        },
                    ),
                );
            }
            order += 1;
        }
    }
    Ok((winners, attributes))
}
fn winner(
    tree: &RetainedCemTree,
    inline: &InlineWinners,
    owner: AstNodeId,
    property: &str,
    work: &mut Work,
) -> Result<Option<(Expression, PaintDeclarationWinner)>, String> {
    work.spend(1)?;
    if let Some(value) = inline.get(&(owner, property.to_owned())) {
        return Ok(Some(value.clone()));
    }
    let Some(attribute) = attribute(tree, owner, property, work)? else {
        return Ok(None);
    };
    Ok(Some((
        expression(&tree.node(attribute).unwrap().value)?,
        PaintDeclarationWinner {
            attribute,
            css: None,
            css_declaration: None,
            important: false,
            order: 0,
        },
    )))
}
fn local(tree: &RetainedCemTree, id: AstNodeId) -> Result<&str, String> {
    let name = tree
        .node(id)
        .and_then(|n| n.name.as_ref())
        .ok_or("paint color element missing")?;
    if name.namespace_uri != SVG {
        return Err("paint color requires SVG elements".into());
    }
    Ok(&name.local_name)
}
fn elements(
    tree: &RetainedCemTree,
    id: AstNodeId,
    work: &mut Work,
) -> Result<Vec<AstNodeId>, String> {
    let mut result = Vec::new();
    for child in &tree.node(id).ok_or("paint color owner missing")?.children {
        work.spend(1)?;
        if tree.node(*child).ok_or("paint color child missing")?.kind == CemTreeNodeKind::Element {
            result.push(*child);
        }
    }
    Ok(result)
}
fn attribute(
    tree: &RetainedCemTree,
    id: AstNodeId,
    name: &str,
    work: &mut Work,
) -> Result<Option<AstNodeId>, String> {
    for id in &tree.node(id).ok_or("paint color owner missing")?.attributes {
        work.spend(1)?;
        let a = tree.node(*id).ok_or("paint color attribute missing")?;
        work.spend(a.value.len())?;
        if a.name
            .as_ref()
            .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == name)
        {
            return Ok(Some(*id));
        }
    }
    Ok(None)
}
#[derive(Clone)]
enum Expression {
    Literal([u8; 4]),
    Current,
}
fn expression(value: &str) -> Result<Expression, String> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let token = parser.next().map_err(|_| "empty paint color")?.clone();
    if !parser.is_exhausted() {
        return Err("paint color requires one color token".into());
    }
    match token {
        Token::Ident(name) => match name.to_ascii_lowercase().as_str() {
            "currentcolor" => Ok(Expression::Current),
            "red" => Ok(Expression::Literal([255, 0, 0, 255])),
            "blue" => Ok(Expression::Literal([0, 0, 255, 255])),
            "green" => Ok(Expression::Literal([0, 128, 0, 255])),
            "black" => Ok(Expression::Literal([0, 0, 0, 255])),
            "white" => Ok(Expression::Literal([255, 255, 255, 255])),
            "transparent" => Ok(Expression::Literal([0, 0, 0, 0])),
            _ => Err("paint color keyword unsupported".into()),
        },
        Token::Hash(value) | Token::IDHash(value) => {
            if !matches!(value.len(), 3 | 6) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("paint color requires three or six hex digits".into());
            }
            let mut rgba = [0, 0, 0, 255];
            for (i, channel) in rgba[..3].iter_mut().enumerate() {
                *channel = if value.len() == 3 {
                    u8::from_str_radix(&value[i..i + 1], 16).unwrap() * 17
                } else {
                    u8::from_str_radix(&value[2 * i..2 * i + 2], 16).unwrap()
                };
            }
            Ok(Expression::Literal(rgba))
        }
        _ => Err("paint color expression unsupported".into()),
    }
}
struct Work(usize);
impl Work {
    fn spend(&mut self, n: usize) -> Result<(), String> {
        self.0 = self
            .0
            .checked_sub(n)
            .ok_or("paint color work budget exceeded")?;
        Ok(())
    }
}
