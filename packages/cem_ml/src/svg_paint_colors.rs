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

pub struct PaintColorDerivation {
    pub host: AstNodeId,
    pub source: AstNodeId,
    pub paint_declaration: AstNodeId,
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
                if a.name
                    .as_ref()
                    .is_some_and(|n| n.local_name == "style" || n.local_name.starts_with("on"))
                {
                    return Err("paint color inline cascade or event handler unsupported".into());
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
            let declaration = attribute(tree, child, property, &mut work)?
                .ok_or("paint color requires explicit paint declaration")?;
            let paint_expression = expression(&tree.node(declaration).unwrap().value)?;
            let mut color_declarations = Vec::new();
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
                            if let Some(a) = attribute(tree, id, "color", &mut work)? {
                                color_declarations.push(a);
                                match expression(&tree.node(a).unwrap().value)? {
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
