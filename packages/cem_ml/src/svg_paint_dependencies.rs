//! Conservative local paint/template candidates over one retained HTML inventory.
//! This is not a used-paint computation, load receipt or execution seal.
use crate::{
    html_base::HtmlFallbackBase,
    html_execution_inventory::HtmlExecutionInventory,
    import::svg_presentation::RetainedSvgPresentationAttribute,
    parser::{tree::RetainedCemTree, AstNodeId},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
const SVG: &str = "http://www.w3.org/2000/svg";
const XLINK: &str = "http://www.w3.org/1999/xlink";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintTarget {
    Local(AstNodeId),
    Missing,
    Ambiguous,
    WrongKind(AstNodeId),
    Cycle(AstNodeId),
    UnsupportedFragment,
    /// Includes external URLs and absolute/relative aliases: no fetch or guess.
    UntraversedUrl,
    /// Fragment href cannot borrow the CSS local-URL rule when bases differ.
    BaseContextRequired,
}
pub struct PaintLink {
    pub owner: AstNodeId,
    pub attribute: AstNodeId,
    pub authored_uri: String,
    pub target: PaintTarget,
}
pub struct PaintRoot {
    pub source: RetainedSvgPresentationAttribute,
    pub css_reference: AstNodeId,
    pub authored_uri: String,
    pub target: PaintTarget,
    pub links: Vec<PaintLink>,
}
pub struct SvgPaintPlan {
    inventory: Arc<HtmlExecutionInventory>,
    roots: Vec<PaintRoot>,
}
impl SvgPaintPlan {
    pub fn inventory(&self) -> &Arc<HtmlExecutionInventory> {
        &self.inventory
    }
    pub fn roots(&self) -> &[PaintRoot] {
        &self.roots
    }
    pub fn blockers(&self) -> &'static [&'static str] {
        &[
            "svg-paint-rendering-evidence-required",
            "svg-paint-content-dependencies-required",
            "svg-computed-winning-source-required",
            "svg-computed-resource-acquisition-required",
        ]
    }
    /// Retains candidates only. Consumers must revalidate the inventory through
    /// its original HTML host before using this plan in a live read set.
    pub fn from_inventory(
        inventory: Arc<HtmlExecutionInventory>,
        budget: usize,
    ) -> Result<Self, String> {
        let tree = &inventory.document().tree;
        let mut work = Work(budget.min(1_000_000));
        let mut ids: BTreeMap<String, Vec<AstNodeId>> = BTreeMap::new();
        let mut pending = vec![0];
        while let Some(id) = pending.pop() {
            let node = tree.node(id).ok_or("invalid paint inventory node")?;
            work.spend(1)?;
            work.spend(node.value.len())?;
            for attribute in &node.attributes {
                let attr = tree.node(*attribute).ok_or("invalid paint attribute")?;
                work.spend(1)?;
                work.spend(attr.value.len())?;
                if attr
                    .name
                    .as_ref()
                    .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == "id")
                {
                    ids.entry(attr.value.clone()).or_default().push(id);
                }
            }
            if node.name.as_ref().is_some_and(|n| {
                n.namespace_uri == "http://www.w3.org/1999/xhtml" && n.local_name == "template"
            }) {
                continue;
            }
            pending.extend(node.children.iter().rev().copied());
        }
        // This bounded profile only follows href fragments with unequivocal
        // document/base agreement. Embedded documents and URL aliases stay open.
        let base = inventory.base();
        let href_local = match &base.native_resolution().context.fallback {
            HtmlFallbackBase::DocumentUrl(document) => {
                let mut document =
                    url::Url::parse(document).map_err(|_| "invalid paint document URL")?;
                let mut effective =
                    url::Url::parse(base.effective_url()).map_err(|_| "invalid paint base URL")?;
                document.set_fragment(None);
                effective.set_fragment(None);
                matches!(document.scheme(), "http" | "https") && document == effective
            }
            _ => false,
        };
        let mut roots = Vec::new();
        for presentation in inventory.svg_presentations() {
            let source = &presentation.source;
            let property = tree
                .node(source.attribute())
                .and_then(|n| n.name.as_ref())
                .ok_or("paint property missing")?;
            if !matches!(property.local_name.as_str(), "fill" | "stroke") {
                continue;
            }
            for reference in &presentation.resources.references {
                if roots.len() >= 256 {
                    return Err("paint root budget exceeded".into());
                }
                work.spend(1)?;
                let target = lookup(&reference.authored_specifier, None, tree, &ids);
                let mut links = Vec::new();
                let mut seen = BTreeSet::new();
                let mut next = match target {
                    PaintTarget::Local(id) => Some(id),
                    _ => None,
                };
                while let Some(owner) = next {
                    work.spend(1)?;
                    seen.insert(owner);
                    let Some(attribute) = href(tree, owner) else {
                        break;
                    };
                    if links.len() >= 64 {
                        return Err("paint template depth exceeded".into());
                    }
                    let authored_uri = tree.node(attribute).unwrap().value.clone();
                    let mut target = if authored_uri.starts_with('#') && !href_local {
                        PaintTarget::BaseContextRequired
                    } else {
                        lookup(&authored_uri, kind(tree, owner), tree, &ids)
                    };
                    if let PaintTarget::Local(id) = target {
                        if seen.contains(&id) {
                            target = PaintTarget::Cycle(id);
                        }
                    }
                    next = match target {
                        PaintTarget::Local(id) => Some(id),
                        _ => None,
                    };
                    links.push(PaintLink {
                        owner,
                        attribute,
                        authored_uri,
                        target,
                    });
                }
                roots.push(PaintRoot {
                    source: source.clone(),
                    css_reference: reference.node_id,
                    authored_uri: reference.authored_specifier.clone(),
                    target,
                    links,
                });
            }
        }
        Ok(Self { inventory, roots })
    }
}
struct Work(usize);
impl Work {
    fn spend(&mut self, count: usize) -> Result<(), String> {
        self.0 = self
            .0
            .checked_sub(count)
            .ok_or("paint traversal work budget exceeded")?;
        Ok(())
    }
}
pub(crate) fn kind(tree: &RetainedCemTree, id: AstNodeId) -> Option<&str> {
    let name = tree.node(id)?.name.as_ref()?;
    (name.namespace_uri == SVG)
        .then_some(name.local_name.as_str())
        .filter(|n| matches!(*n, "linearGradient" | "radialGradient" | "pattern"))
}
fn lookup(
    uri: &str,
    parent: Option<&str>,
    tree: &RetainedCemTree,
    ids: &BTreeMap<String, Vec<AstNodeId>>,
) -> PaintTarget {
    let Some(fragment) = uri.strip_prefix('#') else {
        return PaintTarget::UntraversedUrl;
    };
    // No percent-decoding, Unicode normalization or SVG view-fragment guessing.
    // This initial profile admits only literal ASCII identifiers.
    if !literal_fragment(fragment) {
        return PaintTarget::UnsupportedFragment;
    }
    let Some(nodes) = ids.get(fragment) else {
        return PaintTarget::Missing;
    };
    if nodes.len() != 1 {
        return PaintTarget::Ambiguous;
    }
    let target = nodes[0];
    let Some(target_kind) = kind(tree, target) else {
        return PaintTarget::WrongKind(target);
    };
    if parent.is_some_and(|p| (p == "pattern") != (target_kind == "pattern")) {
        return PaintTarget::WrongKind(target);
    }
    PaintTarget::Local(target)
}
pub(crate) fn href(tree: &RetainedCemTree, owner: AstNodeId) -> Option<AstNodeId> {
    let attributes = &tree.node(owner)?.attributes;
    for namespace in ["", XLINK] {
        if let Some(id) = attributes.iter().find(|id| {
            tree.node(**id)
                .and_then(|n| n.name.as_ref())
                .is_some_and(|n| n.local_name == "href" && n.namespace_uri == namespace)
        }) {
            return Some(*id);
        }
    }
    None
}

// Shared bounded identifier profile for local and acquired external paint.
pub(crate) fn literal_fragment(fragment: &str) -> bool {
    !fragment.is_empty()
        && fragment.len() <= 4096
        && fragment
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b':'))
}
