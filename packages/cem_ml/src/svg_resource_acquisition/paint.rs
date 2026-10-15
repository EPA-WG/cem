//! Acquired paint content selection and source provenance. No live cascade or
//! resource consumption authority. All node IDs refer to the acquired body.
use super::*;
use crate::{
    document_stylesheet_dependencies::{inspect_document_stylesheets, DocumentStylesheetInventory},
    module_resolution::{
        CemModuleUrlReferrer, CemModuleUrlResolution, CemModuleUrlResolutionCapability,
        CemModuleUrlResolutionError, CemModuleUrlResolutionPurpose,
    },
    svg_paint_clone_profile::SvgPaintCloneProfile,
    svg_paint_style::PaintInstanceParent,
};
const SVG: &str = "http://www.w3.org/2000/svg";

pub struct PaintAttributeSource {
    pub name: &'static str,
    /// None leaves the initial/context-dependent value unresolved; no fabricated
    /// attribute or source range represents a default.
    pub source: Option<(AstNodeId, AstNodeId)>,
}
pub struct AcquiredPaintNode {
    pub source: AstNodeId,
    pub source_ancestors: Vec<AstNodeId>,
    /// SVG 2 structural parent only. Pinned Chromium paints these from source
    /// styles in the supported profile; this is not a computed-style parent.
    pub instance_parent: PaintInstanceParent,
}
pub struct AcquiredStyleSource {
    pub owner: AstNodeId,
    pub source_node: AstNodeId,
    pub css: Option<Arc<RetainedCemTree>>,
}
pub struct AcquiredDependency {
    pub owner: AstNodeId,
    pub source_node: AstNodeId,
    /// CSS coordinates remain paired with the original document source node.
    pub css: Option<(Arc<RetainedCemTree>, AstNodeId)>,
    pub authored_uri: String,
    pub base_url: String,
    pub resolution: Result<CemModuleUrlResolution, CemModuleUrlResolutionError>,
}
pub struct AcquiredPaintContent {
    pub host: AstNodeId,
    pub profile: SvgPaintCloneProfile,
    pub attributes: Vec<PaintAttributeSource>,
    pub svg2_content_owner: Option<AstNodeId>,
    pub content_owner: Option<AstNodeId>,
    pub nodes: Vec<AcquiredPaintNode>,
    pub style_sources: Vec<AcquiredStyleSource>,
    pub dependencies: Vec<AcquiredDependency>,
    pub inventory: Arc<DocumentStylesheetInventory>,
    /// Bounded source-style derivations or an explicit unsupported-profile reason.
    /// Even successful author colors are not a browser winning-style seal.
    pub colors: Result<Vec<crate::svg_paint_colors::PaintColorDerivation>, String>,
    pub blockers: Vec<String>,
}
pub(super) struct Work(pub usize);
impl Work {
    fn spend(&mut self, count: usize) -> Result<(), String> {
        self.0 = self
            .0
            .checked_sub(count)
            .ok_or("acquired paint work budget exceeded")?;
        Ok(())
    }
}

pub(super) fn inventory(
    body: &SvgAcquiredBody,
    cap: &CemModuleUrlResolutionCapability,
    work: &mut Work,
    cancel: &AbortSignal,
) -> Result<Arc<DocumentStylesheetInventory>, String> {
    let tree = body.tree();
    let mut pending = vec![0];
    // Reject active/foreign and base-changing contexts throughout the body;
    // silently excluding an unused subtree would lose possible style effects.
    while let Some(id) = pending.pop() {
        active(cancel)?;
        let n = tree.node(id).ok_or("acquired content node missing")?;
        work.spend(1 + n.value.len() + n.children.len() + n.attributes.len())?;
        if matches!(
            n.kind,
            CemTreeNodeKind::ProcessingInstruction | CemTreeNodeKind::Reference
        ) {
            return Err("active acquired SVG context unsupported".into());
        }
        if n.kind == CemTreeNodeKind::Element {
            let name = n.name.as_ref().ok_or("acquired element name missing")?;
            if name.namespace_uri != SVG
                || matches!(
                    name.local_name.as_str(),
                    "script"
                        | "animate"
                        | "animateMotion"
                        | "animateTransform"
                        | "set"
                        | "discard"
                        | "foreignObject"
                )
            {
                return Err("active or foreign acquired SVG context unsupported".into());
            }
        }
        for attr in &n.attributes {
            let attr = tree.node(*attr).ok_or("acquired attribute missing")?;
            work.spend(1 + attr.value.len())?;
            if attr.name.as_ref().is_some_and(|a| {
                (a.namespace_uri.is_empty() && a.local_name.starts_with("on"))
                    || (a.namespace_uri == "http://www.w3.org/XML/1998/namespace"
                        && a.local_name == "base")
            }) {
                return Err("active or XML base acquired context unsupported".into());
            }
        }
        pending.extend(n.children.iter().rev().copied());
    }
    let result = inspect_document_stylesheets(tree.clone(), cap, body.url(), work.0);
    // Failed or truncated inventory cannot masquerade as an empty dependency set.
    if result
        .blockers
        .iter()
        .chain(&result.document.blockers)
        .any(|b| {
            b.contains("budget")
                || b.contains("incomplete")
                || b.contains("invalid")
                || b.contains("complete-document")
                || b.contains("unresolved")
                || b.contains("diagnostics")
        })
    {
        return Err("complete acquired SVG stylesheet inventory required".into());
    }
    if result.stylesheets.len() > 256 {
        return Err("acquired stylesheet bound exceeded".into());
    }
    for sheet in &result.stylesheets {
        let css = &sheet
            .resources
            .as_ref()
            .map_err(|e| format!("acquired CSS: {e}"))?
            .tree;
        work.spend(css.ast().nodes.len())?;
        if css
            .ast_owner()
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err("invalid acquired CSS".into());
        }
    }
    Ok(Arc::new(result))
}

pub(super) fn build(
    binding: &SvgResourceBinding,
    host: AstNodeId,
    links: &[SvgPaintTemplateLink],
    inventory: Arc<DocumentStylesheetInventory>,
    profile: SvgPaintCloneProfile,
    cap: &CemModuleUrlResolutionCapability,
    work: &mut Work,
    cancel: &AbortSignal,
) -> Result<AcquiredPaintContent, String> {
    let tree = binding.body.tree();
    let kind = crate::svg_paint_dependencies::kind(tree, host).ok_or("paint host missing")?;
    let chain: Vec<_> = std::iter::once(host)
        .chain(links.iter().map(|l| l.target))
        .collect();
    let names: &[&'static str] = match kind {
        "linearGradient" => &[
            "x1",
            "y1",
            "x2",
            "y2",
            "gradientUnits",
            "gradientTransform",
            "spreadMethod",
        ],
        "radialGradient" => &[
            "cx",
            "cy",
            "r",
            "fx",
            "fy",
            "fr",
            "gradientUnits",
            "gradientTransform",
            "spreadMethod",
        ],
        "pattern" => &[
            "x",
            "y",
            "width",
            "height",
            "patternUnits",
            "patternContentUnits",
            "patternTransform",
            "viewBox",
            "preserveAspectRatio",
        ],
        _ => return Err("unsupported paint host".into()),
    };
    let mut attributes = Vec::new();
    for name in names {
        let mut source = None;
        for owner in &chain {
            let n = tree.node(*owner).ok_or("paint template missing")?;
            work.spend(1 + n.attributes.len())?;
            // Gradient-specific geometry is read only on that gradient kind.
            let common = matches!(
                *name,
                "gradientUnits" | "gradientTransform" | "spreadMethod"
            );
            if kind != "pattern"
                && !common
                && crate::svg_paint_dependencies::kind(tree, *owner) != Some(kind)
            {
                continue;
            }
            if let Some(attr) = n.attributes.iter().find(|a| {
                tree.node(**a)
                    .and_then(|v| v.name.as_ref())
                    .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == *name)
            }) {
                source = Some((*owner, *attr));
                break;
            }
        }
        attributes.push(PaintAttributeSource { name, source });
    }
    let mut svg2_content_owner = None;
    let mut content_owner = None;
    for owner in &chain {
        active(cancel)?;
        for child in &tree.node(*owner).ok_or("paint template missing")?.children {
            work.spend(1)?;
            let n = tree.node(*child).ok_or("paint child missing")?;
            if n.kind != CemTreeNodeKind::Element {
                continue;
            }
            let name = n.name.as_ref().ok_or("paint child name missing")?;
            let descriptive = matches!(name.local_name.as_str(), "title" | "desc" | "metadata");
            if !descriptive && svg2_content_owner.is_none() {
                svg2_content_owner = Some(*owner);
            }
            if (kind == "pattern" || name.local_name == "stop") && content_owner.is_none() {
                content_owner = Some(*owner);
            }
        }
    }
    let mut nodes = Vec::new();
    let mut relevant = BTreeSet::new();
    // Preserve both receiving and lexical source contexts; never borrow the
    // graphics consumer's ancestors or an HTML document base for this body.
    for owner in &chain {
        relevant.insert(*owner);
        relevant.extend(ancestry(tree, *owner, work)?);
    }
    if let Some(owner) = content_owner {
        let mut pending: Vec<_> = tree
            .node(owner)
            .unwrap()
            .children
            .iter()
            .rev()
            .copied()
            .collect();
        while let Some(id) = pending.pop() {
            active(cancel)?;
            work.spend(1)?;
            if nodes.len() >= 4096 {
                return Err("acquired paint content node bound exceeded".into());
            }
            let n = tree.node(id).ok_or("selected paint node missing")?;
            let source_ancestors = ancestry(tree, id, work)?;
            relevant.insert(id);
            relevant.extend(source_ancestors.iter().copied());
            nodes.push(AcquiredPaintNode {
                source: id,
                source_ancestors,
                instance_parent: if n.parent == Some(owner) {
                    PaintInstanceParent::Host
                } else {
                    PaintInstanceParent::SourceInstance(n.parent.ok_or("paint parent missing")?)
                },
            });
            work.spend(n.children.len())?;
            pending.extend(n.children.iter().rev().copied());
        }
    }
    let mut style_sources = Vec::new();
    let mut dependencies = Vec::new();
    for sheet in &inventory.stylesheets {
        work.spend(1)?;
        // All stylesheet rules may match selected nodes; inline declarations
        // remain attached to the actual owner and its source ancestry.
        if tree.node(sheet.source_node).unwrap().kind == CemTreeNodeKind::Attribute
            && !relevant.contains(&sheet.owner)
        {
            continue;
        }
        let resources = sheet.resources.as_ref().map_err(|e| e.clone())?;
        style_sources.push(AcquiredStyleSource {
            owner: sheet.owner,
            source_node: sheet.source_node,
            css: Some(resources.tree.clone()),
        });
        append_css(
            &mut dependencies,
            sheet.owner,
            sheet.source_node,
            resources,
            binding.body.url(),
            work,
        )?;
    }
    for owner in &relevant {
        for attr in &tree.node(*owner).ok_or("style owner missing")?.attributes {
            work.spend(1)?;
            if tree
                .node(*attr)
                .and_then(|n| n.name.as_ref())
                .is_some_and(|n| {
                    n.namespace_uri.is_empty()
                        && matches!(
                            n.local_name.as_str(),
                            "color"
                                | "fill"
                                | "stroke"
                                | "stop-color"
                                | "stop-opacity"
                                | "opacity"
                                | "display"
                                | "visibility"
                                | "filter"
                                | "clip-path"
                                | "mask"
                                | "transform"
                        )
                })
            {
                style_sources.push(AcquiredStyleSource {
                    owner: *owner,
                    source_node: *attr,
                    css: None,
                });
            }
        }
    }
    for p in &inventory.presentations {
        work.spend(1)?;
        if relevant.contains(&p.source.owner()) {
            append_css(
                &mut dependencies,
                p.source.owner(),
                p.source.attribute(),
                &p.resources,
                binding.body.url(),
                work,
            )?;
        }
    }
    for reference in &inventory.document.references {
        work.spend(1)?;
        if !relevant.contains(&reference.owner)
            || links.iter().any(|l| l.attribute == reference.attribute)
        {
            continue;
        }
        // Honor href precedence for resources as well as paint templates.
        if tree
            .node(reference.attribute)
            .and_then(|n| n.name.as_ref())
            .is_some_and(|n| n.local_name == "href")
            && crate::svg_paint_dependencies::href(tree, reference.owner)
                != Some(reference.attribute)
        {
            continue;
        }
        dependencies.push(AcquiredDependency {
            owner: reference.owner,
            source_node: reference.attribute,
            css: None,
            authored_uri: reference.authored_uri.clone(),
            base_url: binding.body.url().into(),
            resolution: cap.resolve_with_referrer(
                CemModuleUrlResolutionPurpose::Css,
                &reference.authored_uri,
                CemModuleUrlReferrer::Url(binding.body.url().into()),
                reference.source.clone(),
            ),
        });
    }
    if dependencies.len() > 4096 {
        return Err("acquired dependency bound exceeded".into());
    }
    let mut blockers = inventory.blockers.clone();
    blockers.extend(inventory.document.blockers.iter().cloned());
    blockers.extend(
        [
            "svg-acquired-style-evaluation-required",
            "svg-resource-transitive-dependencies-required",
            "svg-resource-browser-response-identity-required",
            "svg-resource-consumption-required",
        ]
        .map(str::to_owned),
    );
    if dependencies.iter().any(|d| d.resolution.is_err()) {
        blockers.push("svg-acquired-dependency-resolution-required".into());
    }
    blockers.sort();
    blockers.dedup();
    let mut result = AcquiredPaintContent {
        host,
        profile,
        attributes,
        svg2_content_owner,
        content_owner,
        nodes,
        style_sources,
        dependencies,
        inventory,
        colors: Err("not evaluated".into()),
        blockers,
    };
    let (colors, spent) = crate::svg_paint_colors::evaluate_acquired(&result, work.0);
    work.spend(spent)?;
    if colors
        .as_ref()
        .err()
        .is_some_and(|e| e.contains("budget") || e.contains("bound exceeded"))
    {
        return Err("acquired color work bound exceeded".into());
    }
    result.colors = colors;
    Ok(result)
}
fn ancestry(
    tree: &RetainedCemTree,
    id: AstNodeId,
    work: &mut Work,
) -> Result<Vec<AstNodeId>, String> {
    let mut result = Vec::new();
    let mut next = tree.node(id).ok_or("paint source missing")?.parent;
    while let Some(id) = next {
        work.spend(1)?;
        if result.len() >= 128 {
            return Err("acquired source ancestry bound exceeded".into());
        }
        result.push(id);
        next = tree.node(id).ok_or("paint ancestor missing")?.parent;
    }
    Ok(result)
}
fn append_css(
    out: &mut Vec<AcquiredDependency>,
    owner: AstNodeId,
    source_node: AstNodeId,
    resources: &crate::css_resources::CssResourcePlan,
    base: &str,
    work: &mut Work,
) -> Result<(), String> {
    for r in &resources.references {
        work.spend(1 + r.authored_specifier.len())?;
        if out.len() >= 4096 {
            return Err("acquired dependency bound exceeded".into());
        }
        out.push(AcquiredDependency {
            owner,
            source_node,
            css: Some((resources.tree.clone(), r.node_id)),
            authored_uri: r.authored_specifier.clone(),
            base_url: base.into(),
            resolution: r.resolution.clone(),
        });
    }
    Ok(())
}
