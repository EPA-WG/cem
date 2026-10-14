//! Authored content and lexical style provenance for local paint candidates.
//! No template-content selection, cloned instance tree or computed inheritance.
use crate::{
    parser::{tree::RetainedCemTree, AstNodeId},
    svg_paint_dependencies::{PaintTarget, SvgPaintPlan},
};
use std::{collections::BTreeSet, sync::Arc};
const SVG: &str = "http://www.w3.org/2000/svg";

pub struct PaintContentNode {
    pub node: AstNodeId,
    /// Nearest parent first, through the retained document root. These are source
    /// ancestors, never substituted with the referencing graphics element.
    pub source_ancestors: Vec<AstNodeId>,
}
pub struct PaintServerContent {
    pub owner: AstNodeId,
    /// Source order, including the server itself, text and descriptive content.
    pub nodes: Vec<PaintContentNode>,
    pub direct_stops: Vec<AstNodeId>,
    /// Attribute owners from the existing native direct-reference inventory.
    /// CSS URLs, indirect uses and runtime loads still require separate coverage.
    pub reference_attributes: Vec<AstNodeId>,
}
pub struct SvgPaintContentPlan {
    paint: Arc<SvgPaintPlan>,
    servers: Vec<PaintServerContent>,
}
impl SvgPaintContentPlan {
    pub fn paint(&self) -> &Arc<SvgPaintPlan> {
        &self.paint
    }
    pub fn servers(&self) -> &[PaintServerContent] {
        &self.servers
    }
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-paint-content-evaluation-required",
            "svg-paint-clone-style-provenance-required",
            "svg-paint-rendering-evidence-required",
            "svg-computed-winning-source-required",
            "svg-computed-resource-acquisition-required",
        ]
    }
    /// Bounded, immutable candidate analysis. Live use requires the original
    /// inventory's host validation; this plan supplies no independent authority.
    pub fn new(paint: Arc<SvgPaintPlan>, budget: usize) -> Result<Self, String> {
        let mut work = Work(budget.min(1_000_000));
        let mut owners = BTreeSet::new();
        for target in paint
            .roots()
            .iter()
            .flat_map(|r| std::iter::once(&r.target).chain(r.links.iter().map(|l| &l.target)))
        {
            work.spend(1)?;
            if let PaintTarget::Local(id) = target {
                owners.insert(*id);
            }
            if owners.len() > 256 {
                return Err("paint content server bound exceeded".into());
            }
        }
        let tree = &paint.inventory().document().tree;
        let mut servers = Vec::new();
        let mut count = 0;
        for owner in owners {
            let gradient = ["linearGradient", "radialGradient"]
                .iter()
                .any(|name| named(tree, owner, SVG, name));
            let mut nodes = Vec::new();
            let mut direct_stops = Vec::new();
            let mut members = BTreeSet::new();
            let mut pending = vec![owner];
            while let Some(id) = pending.pop() {
                count += 1;
                if count > 4096 {
                    return Err("paint content node bound exceeded".into());
                }
                work.spend(1)?;
                let node = tree.node(id).ok_or("paint content node missing")?;
                work.spend(node.value.len())?;
                for attribute in &node.attributes {
                    work.spend(1)?;
                    work.spend(
                        tree.node(*attribute)
                            .ok_or("paint content attribute missing")?
                            .value
                            .len(),
                    )?;
                }
                let mut source_ancestors = Vec::new();
                let mut parent = node.parent;
                while let Some(ancestor) = parent {
                    if source_ancestors.len() >= 128 {
                        return Err("paint content ancestry bound exceeded".into());
                    }
                    work.spend(1)?;
                    source_ancestors.push(ancestor);
                    parent = tree
                        .node(ancestor)
                        .ok_or("paint content ancestor missing")?
                        .parent;
                }
                if gradient && node.parent == Some(owner) && named(tree, id, SVG, "stop") {
                    direct_stops.push(id);
                }
                nodes.push(PaintContentNode {
                    node: id,
                    source_ancestors,
                });
                members.insert(id);
                if !named(tree, id, "http://www.w3.org/1999/xhtml", "template") {
                    pending.extend(node.children.iter().rev().copied());
                }
            }
            let mut reference_attributes = Vec::new();
            for reference in &paint.inventory().document().references {
                work.spend(1)?;
                if members.contains(&reference.owner) {
                    reference_attributes.push(reference.attribute);
                }
            }
            servers.push(PaintServerContent {
                owner,
                nodes,
                direct_stops,
                reference_attributes,
            });
        }
        Ok(Self { paint, servers })
    }
}
fn named(tree: &RetainedCemTree, id: AstNodeId, namespace: &str, local: &str) -> bool {
    tree.node(id)
        .and_then(|n| n.name.as_ref())
        .is_some_and(|n| n.namespace_uri == namespace && n.local_name == local)
}
struct Work(usize);
impl Work {
    fn spend(&mut self, count: usize) -> Result<(), String> {
        self.0 = self
            .0
            .checked_sub(count)
            .ok_or("paint content work budget exceeded")?;
        Ok(())
    }
}
