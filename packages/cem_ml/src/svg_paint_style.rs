//! SVG 2 structural clone-style provenance candidates, not computed style or
//! browser compatibility evidence. Source IDs never represent materialized clones.
use crate::{
    parser::{tree::CemTreeNodeKind, AstNodeId},
    svg_paint_content::SvgPaintContentPlan,
    svg_paint_dependencies::PaintTarget,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintContentSelection {
    Own,
    Template(AstNodeId),
    Empty,
    Unresolved(PaintTarget),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintInstanceParent {
    /// The receiving paint server, not the source server or graphics consumer.
    Host,
    /// Another source node instantiated in this same host's candidate context.
    SourceInstance(AstNodeId),
}
pub struct PaintStyleInstance {
    pub source: AstNodeId,
    pub parent: PaintInstanceParent,
}
pub struct PaintStyleHost {
    pub host: AstNodeId,
    /// Lexical ancestors of the receiving server; nearest parent first.
    pub host_ancestors: Vec<AstNodeId>,
    pub selection: PaintContentSelection,
    /// Authored href attribute IDs, in selection order.
    pub template_attributes: Vec<AstNodeId>,
    /// Only prospective cloned descendants. Own content stays in the source plan.
    pub instances: Vec<PaintStyleInstance>,
}
pub struct SvgPaintStylePlan {
    content: Arc<SvgPaintContentPlan>,
    hosts: Vec<PaintStyleHost>,
}
impl SvgPaintStylePlan {
    pub fn content(&self) -> &Arc<SvgPaintContentPlan> {
        &self.content
    }
    pub fn hosts(&self) -> &[PaintStyleHost] {
        &self.hosts
    }
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-paint-clone-style-evaluation-required",
            "svg-paint-clone-browser-compatibility-required",
            "svg-paint-rendering-evidence-required",
            "svg-computed-winning-source-required",
            "svg-computed-resource-acquisition-required",
        ]
    }
    /// Applies the SVG 2 non-descriptive-child selection rule to already bounded
    /// local candidates. Does not execute CSS, copy computed values or create DOM.
    pub fn new(content: Arc<SvgPaintContentPlan>, budget: usize) -> Result<Self, String> {
        let mut work = Work(budget.min(1_000_000));
        let tree = &content.paint().inventory().document().tree;
        let mut servers = BTreeMap::new();
        let mut relevant = BTreeSet::new();
        for server in content.servers() {
            work.spend(1)?;
            if servers.len() >= 256 {
                return Err("paint style host bound exceeded".into());
            }
            servers.insert(server.owner, server);
            for child in &tree
                .node(server.owner)
                .ok_or("paint style host missing")?
                .children
            {
                work.spend(1)?;
                let child = tree.node(*child).ok_or("paint style child missing")?;
                let descriptive = child.name.as_ref().is_some_and(|n| {
                    n.namespace_uri == "http://www.w3.org/2000/svg"
                        && matches!(n.local_name.as_str(), "title" | "desc" | "metadata")
                });
                if child.kind == CemTreeNodeKind::Element && !descriptive {
                    relevant.insert(server.owner);
                }
            }
        }
        let mut links = BTreeMap::new();
        for root in content.paint().roots() {
            work.spend(1)?;
            for link in &root.links {
                work.spend(1)?;
                // A traversal cycle is relative to that root. Content selection
                // must test own children before following links from each host.
                let target = match link.target {
                    PaintTarget::Cycle(id) => PaintTarget::Local(id),
                    _ => link.target.clone(),
                };
                if let Some(previous) = links.insert(link.owner, (link.attribute, target.clone())) {
                    if previous != (link.attribute, target) {
                        return Err("inconsistent paint style template edge".into());
                    }
                }
            }
        }
        let mut hosts = Vec::new();
        let mut instance_count = 0;
        for server in content.servers() {
            work.spend(1)?;
            let mut current = server.owner;
            let mut seen = BTreeSet::new();
            let mut template_attributes = Vec::new();
            let selection = loop {
                work.spend(1)?;
                if !seen.insert(current) {
                    break PaintContentSelection::Unresolved(PaintTarget::Cycle(current));
                }
                if relevant.contains(&current) {
                    break if current == server.owner {
                        PaintContentSelection::Own
                    } else {
                        PaintContentSelection::Template(current)
                    };
                }
                let Some((attribute, target)) = links.get(&current) else {
                    break PaintContentSelection::Empty;
                };
                if template_attributes.len() >= 64 {
                    return Err("paint style template depth exceeded".into());
                }
                template_attributes.push(*attribute);
                match target {
                    PaintTarget::Local(next) => {
                        if !servers.contains_key(next) {
                            return Err("paint style source missing".into());
                        }
                        current = *next;
                    }
                    other => break PaintContentSelection::Unresolved(other.clone()),
                }
            };
            let mut instances = Vec::new();
            if let PaintContentSelection::Template(source) = selection {
                for node in &servers[&source].nodes {
                    work.spend(1)?;
                    if node.node == source {
                        continue;
                    }
                    instance_count += 1;
                    if instance_count > 4096 {
                        return Err("paint style instance bound exceeded".into());
                    }
                    let parent = tree
                        .node(node.node)
                        .and_then(|n| n.parent)
                        .ok_or("paint style instance parent missing")?;
                    instances.push(PaintStyleInstance {
                        source: node.node,
                        parent: if parent == source {
                            PaintInstanceParent::Host
                        } else {
                            PaintInstanceParent::SourceInstance(parent)
                        },
                    });
                }
            }
            let ancestry = &server
                .nodes
                .first()
                .ok_or("paint style host content missing")?
                .source_ancestors;
            work.spend(ancestry.len())?;
            hosts.push(PaintStyleHost {
                host: server.owner,
                host_ancestors: ancestry.clone(),
                selection,
                template_attributes,
                instances,
            });
        }
        Ok(Self { content, hosts })
    }
}
struct Work(usize);
impl Work {
    fn spend(&mut self, count: usize) -> Result<(), String> {
        self.0 = self
            .0
            .checked_sub(count)
            .ok_or("paint style work budget exceeded")?;
        Ok(())
    }
}
