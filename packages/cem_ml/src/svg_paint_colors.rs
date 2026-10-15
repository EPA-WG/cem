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
use std::sync::Arc;
const SVG: &str = "http://www.w3.org/2000/svg";

mod cascade;
use cascade::Cascade;

#[derive(Clone)]
pub enum PaintDeclarationOrigin {
    Presentation {
        attribute: AstNodeId,
    },
    Inline {
        attribute: AstNodeId,
        css: Arc<RetainedCemTree>,
        declaration: AstNodeId,
    },
    Rule {
        owner: AstNodeId,
        css: Arc<RetainedCemTree>,
        rule: AstNodeId,
        declaration: AstNodeId,
        selectors: Vec<AstNodeId>,
        specificity: (u32, u32, u32),
        sheet_order: usize,
        rule_order: usize,
    },
}
#[derive(Clone)]
pub struct PaintDeclarationWinner {
    pub origin: PaintDeclarationOrigin,
    pub important: bool,
    /// Declaration order within its inline list or rule, excluding comments.
    pub order: usize,
}
impl PaintDeclarationWinner {
    /// Original document owner; CSS-local IDs are held only in the tagged origin.
    pub fn document_source(&self) -> AstNodeId {
        match &self.origin {
            PaintDeclarationOrigin::Presentation { attribute }
            | PaintDeclarationOrigin::Inline { attribute, .. } => *attribute,
            PaintDeclarationOrigin::Rule { owner, .. } => *owner,
        }
    }
    pub fn css(&self) -> Option<&Arc<RetainedCemTree>> {
        match &self.origin {
            PaintDeclarationOrigin::Presentation { .. } => None,
            PaintDeclarationOrigin::Inline { css, .. }
            | PaintDeclarationOrigin::Rule { css, .. } => Some(css),
        }
    }
    pub fn css_declaration(&self) -> Option<AstNodeId> {
        match &self.origin {
            PaintDeclarationOrigin::Presentation { .. } => None,
            PaintDeclarationOrigin::Inline { declaration, .. }
            | PaintDeclarationOrigin::Rule { declaration, .. } => Some(*declaration),
        }
    }
}
pub struct PaintColorDerivation {
    pub host: AstNodeId,
    pub source: AstNodeId,
    pub property: &'static str,
    pub paint_winner: PaintDeclarationWinner,
    /// Nearest first, including currentColor forwarding declarations.
    pub color_winners: Vec<PaintDeclarationWinner>,
    pub rgba: [u8; 4],
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
        let cascade = Cascade::new(&plan, &mut work)?;
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
                    "link"
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
            if n.name.as_ref().is_some_and(|n| n.local_name == "style")
                && !cascade.sources.contains(&id)
            {
                return Err("paint color unretained stylesheet unsupported".into());
            }
            for id in &n.attributes {
                work.spend(1)?;
                let a = tree.node(*id).ok_or("paint color attribute missing")?;
                work.spend(a.value.len())?;
                if a.name.as_ref().is_some_and(|n| {
                    n.local_name.starts_with("on")
                        || (n.local_name == "style" && !cascade.sources.contains(id))
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
            let (paint_expression, paint_winner) = cascade
                .winner(tree, child, property, &mut work)?
                .ok_or("paint color requires explicit paint declaration")?;
            let mut color_winners = Vec::new();
            let mut inheritance_nodes = Vec::new();
            let rgba = match paint_expression {
                Expression::Literal(value) => value,
                Expression::Opacity(_) => return Err("opacity is not a color".into()),
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
                                cascade.winner(tree, id, "color", &mut work)?
                            {
                                color_winners.push(selected);
                                match value {
                                    Expression::Literal(value) => break value,
                                    Expression::Opacity(_) => {
                                        return Err("opacity is not a color".into())
                                    }
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
                property,
                paint_winner,
                color_winners,
                rgba,
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
    Opacity(f32),
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

// The acquired profile uses lexical source styles in the pinned Chromium
// model. Keep declaration and currentColor winner coordinates in their original
// document/CSS trees; no invented clones or default paint declarations.
pub(crate) fn evaluate_acquired(
    plan: &crate::svg_resource_acquisition::paint::AcquiredPaintContent,
    budget: usize,
) -> (Result<Vec<PaintColorDerivation>, String>, usize) {
    let budget = budget.min(1_000_000);
    let mut work = Work(budget);
    let result = (|| {
        let tree = &plan.inventory.document.tree;
        let cascade =
            Cascade::from_sources_with_opacity(tree, &plan.inventory.stylesheets, &mut work, true)?;
        let Some(source) = plan.content_owner else {
            return Ok(Vec::new());
        };
        let kind = local(tree, plan.host)?;
        let mut colors = Vec::new();
        for child in elements(tree, source, &mut work)? {
            let child_kind = local(tree, child)?;
            if matches!(child_kind, "title" | "desc" | "metadata") {
                continue;
            }
            let property = match (kind, child_kind) {
                ("linearGradient" | "radialGradient", "stop") => "stop-color",
                ("pattern", "rect") => "fill",
                _ => return Err("acquired color child outside bounded profile".into()),
            };
            if !elements(tree, child, &mut work)?.is_empty() {
                return Err("acquired color nested children unsupported".into());
            }
            let (value, paint_winner) = cascade
                .winner(tree, child, property, &mut work)?
                .ok_or("acquired color requires explicit paint declaration")?;
            let mut color_winners = Vec::new();
            let mut inheritance_nodes = Vec::new();
            let rgba = match value {
                Expression::Literal(rgba) => rgba,
                Expression::Opacity(_) => return Err("opacity is not a color".into()),
                Expression::Current => {
                    let mut next = Some(child);
                    loop {
                        work.spend(1)?;
                        let id =
                            next.ok_or("acquired currentColor requires explicit source color")?;
                        if inheritance_nodes.len() >= 128 {
                            return Err("acquired color ancestry bound exceeded".into());
                        }
                        inheritance_nodes.push(id);
                        let node = tree.node(id).ok_or("acquired color ancestor missing")?;
                        if node.name.as_ref().is_some_and(|n| n.namespace_uri == SVG) {
                            if let Some((value, winner)) =
                                cascade.winner(tree, id, "color", &mut work)?
                            {
                                color_winners.push(winner);
                                if let Expression::Literal(rgba) = value {
                                    break rgba;
                                }
                            }
                        }
                        next = node.parent;
                    }
                }
            };
            colors.push(PaintColorDerivation {
                host: plan.host,
                source: child,
                property,
                paint_winner,
                color_winners,
                rgba,
                inheritance_nodes,
            });
        }
        Ok(colors)
    })();
    (result, budget - work.0)
}

fn property_expression(property: &str, value: &str) -> Result<Expression, String> {
    if !matches!(property, "opacity" | "stop-opacity") {
        return expression(value);
    }
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let value = match parser.next().map_err(|_| "empty opacity")? {
        Token::Number { value, .. } => *value,
        Token::Percentage { unit_value, .. } => *unit_value,
        _ => return Err("unsupported opacity value".into()),
    };
    if !value.is_finite() || !parser.is_exhausted() {
        return Err("invalid opacity value".into());
    }
    Ok(Expression::Opacity(value.clamp(0., 1.)))
}

pub struct PaintOpacityDerivation {
    pub source: AstNodeId,
    pub property: &'static str,
    pub value: f32,
    pub winner: PaintDeclarationWinner,
}
/// Explicit declarations at their lexical nodes. Opacity is not inherited;
/// ancestors represent distinct compositing layers, never multiplied here.
pub(crate) fn evaluate_acquired_opacities(
    plan: &crate::svg_resource_acquisition::paint::AcquiredPaintContent,
    budget: usize,
) -> (Result<Vec<PaintOpacityDerivation>, String>, usize) {
    let budget = budget.min(1_000_000);
    let mut work = Work(budget);
    let result = (|| {
        let tree = &plan.inventory.document.tree;
        let cascade =
            Cascade::from_sources_with_opacity(tree, &plan.inventory.stylesheets, &mut work, true)?;
        let mut nodes = std::collections::BTreeSet::new();
        for node in &plan.nodes {
            work.spend(1 + node.source_ancestors.len())?;
            nodes.insert(node.source);
            nodes.extend(node.source_ancestors.iter().copied());
        }
        let mut values = Vec::new();
        for source in nodes {
            work.spend(1)?;
            let Some(node) = tree.node(source) else {
                return Err("opacity source missing".into());
            };
            if node.kind != CemTreeNodeKind::Element {
                continue;
            }
            for property in ["opacity", "stop-opacity"] {
                if property == "stop-opacity" && local(tree, source)? != "stop" {
                    continue;
                }
                if let Some((value, winner)) = cascade.winner(tree, source, property, &mut work)? {
                    let Expression::Opacity(value) = value else {
                        return Err("invalid opacity expression".into());
                    };
                    values.push(PaintOpacityDerivation {
                        source,
                        property,
                        value,
                        winner,
                    });
                }
            }
        }
        Ok(values)
    })();
    (result, budget - work.0)
}
