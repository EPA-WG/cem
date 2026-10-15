//! Bounded source-over sample evaluation. Coverage is an explicit assumption,
//! not a hit-test, browser paint receipt, filter result or delivery evidence.
use super::*;
use paint::AcquiredPaintContent;
pub struct PatternSample<'a> {
    pub source: &'a AcquiredPaintContent,
    pub premultiplied_rgba: [f64; 4],
    pub contributors: Vec<AstNodeId>,
}
impl AcquiredPaintContent {
    pub fn composite_pattern_sample(
        &self,
        coverage: &[AstNodeId],
    ) -> Result<PatternSample<'_>, String> {
        let tree = &self.inventory.document.tree;
        if crate::svg_paint_dependencies::kind(tree, self.host) != Some("pattern") {
            return Err("pattern content required for compositing".into());
        }
        if coverage.len() > 4096 {
            return Err("sample coverage bound exceeded".into());
        }
        let colors = self.colors.as_ref().map_err(Clone::clone)?;
        let opacities = self.opacities.as_ref().map_err(Clone::clone)?;
        let covered: BTreeSet<_> = coverage.iter().copied().collect();
        let paints: BTreeMap<_, _> = colors.iter().map(|p| (p.source, p)).collect();
        if covered.len() != coverage.len() || covered.iter().any(|id| !paints.contains_key(id)) {
            return Err("duplicate or unknown sample coverage".into());
        }
        let opacity: BTreeMap<_, _> = opacities
            .iter()
            .filter(|p| p.property == "opacity")
            .map(|p| (p.source, f64::from(p.value)))
            .collect();
        let mut contributors = Vec::new();
        let mut work = 8192usize;
        // Effects can arrive through the original source ancestry, including
        // the template server and ancestors outside the selected subtree. Do
        // not silently turn inherited paint/effect overrides into defaults.
        let mut relevant = BTreeSet::new();
        for node in &self.nodes {
            work = work
                .checked_sub(1)
                .ok_or("compositing work bound exceeded")?;
            if !paints.contains_key(&node.source) {
                continue;
            }
            for id in std::iter::once(&node.source).chain(&node.source_ancestors) {
                work = work
                    .checked_sub(1)
                    .ok_or("compositing work bound exceeded")?;
                relevant.insert(*id);
            }
        }
        for id in relevant {
            for attr in &tree
                .node(id)
                .ok_or("compositing ancestor missing")?
                .attributes
            {
                work = work
                    .checked_sub(1)
                    .ok_or("compositing work bound exceeded")?;
                let a = tree
                    .node(*attr)
                    .and_then(|a| a.name.as_ref())
                    .ok_or("compositing ancestor attribute missing")?;
                if a.namespace_uri.is_empty()
                    && (a.local_name.starts_with("stroke")
                        || a.local_name.starts_with("mask")
                        || a.local_name.starts_with("clip")
                        || a.local_name.starts_with("color-interpolation")
                        || matches!(
                            a.local_name.as_str(),
                            "fill-opacity"
                                | "fill-rule"
                                | "visibility"
                                | "display"
                                | "filter"
                                | "mix-blend-mode"
                                | "isolation"
                                | "paint-order"
                                | "vector-effect"
                                | "shape-rendering"
                                | "color-rendering"
                                | "enable-background"
                        ))
                {
                    return Err("unmodeled inherited pattern compositing effect".into());
                }
            }
        }
        let rgba = if let Some(owner) = self.content_owner {
            children(
                tree,
                owner,
                &covered,
                &paints,
                &opacity,
                &mut contributors,
                &mut work,
                0,
            )?
        } else {
            [0.; 4]
        };
        Ok(PatternSample {
            source: self,
            premultiplied_rgba: rgba,
            contributors,
        })
    }
}
fn over(source: [f64; 4], dest: [f64; 4]) -> [f64; 4] {
    std::array::from_fn(|i| source[i] + dest[i] * (1. - source[3]))
}
fn children(
    tree: &RetainedCemTree,
    owner: AstNodeId,
    covered: &BTreeSet<AstNodeId>,
    paints: &BTreeMap<AstNodeId, &crate::svg_paint_colors::PaintColorDerivation>,
    opacity: &BTreeMap<AstNodeId, f64>,
    contributors: &mut Vec<AstNodeId>,
    work: &mut usize,
    depth: usize,
) -> Result<[f64; 4], String> {
    if depth > 128 {
        return Err("compositing depth bound exceeded".into());
    }
    let mut out = [0.; 4];
    for id in &tree
        .node(owner)
        .ok_or("compositing owner missing")?
        .children
    {
        *work = work
            .checked_sub(1)
            .ok_or("compositing work bound exceeded")?;
        let node = tree.node(*id).ok_or("compositing node missing")?;
        if node.kind != CemTreeNodeKind::Element {
            continue;
        }
        let name = node.name.as_ref().ok_or("compositing name missing")?;
        if matches!(name.local_name.as_str(), "title" | "desc" | "metadata") {
            continue;
        }
        if name.namespace_uri != "http://www.w3.org/2000/svg" {
            return Err("compositing requires SVG content".into());
        }
        for attr in &node.attributes {
            *work = work
                .checked_sub(1)
                .ok_or("compositing work bound exceeded")?;
            let a = tree
                .node(*attr)
                .and_then(|n| n.name.as_ref())
                .ok_or("compositing attribute missing")?;
            if !a.namespace_uri.is_empty()
                || !matches!(
                    a.local_name.as_str(),
                    "id" | "class"
                        | "style"
                        | "fill"
                        | "color"
                        | "opacity"
                        | "x"
                        | "y"
                        | "width"
                        | "height"
                        | "rx"
                        | "ry"
                        | "transform"
                )
            {
                return Err("unmodeled pattern compositing effect".into());
            }
        }
        let mut layer = match name.local_name.as_str() {
            "g" => children(
                tree,
                *id,
                covered,
                paints,
                opacity,
                contributors,
                work,
                depth + 1,
            )?,
            "rect" => {
                let paint = paints.get(id).ok_or("rectangle color missing")?;
                if covered.contains(id) {
                    contributors.push(*id);
                    let a = f64::from(paint.rgba[3]) / 255.;
                    [
                        f64::from(paint.rgba[0]) / 255. * a,
                        f64::from(paint.rgba[1]) / 255. * a,
                        f64::from(paint.rgba[2]) / 255. * a,
                        a,
                    ]
                } else {
                    [0.; 4]
                }
            }
            _ => return Err("pattern compositing content outside bounded profile".into()),
        };
        let factor = opacity.get(id).copied().unwrap_or(1.);
        for v in &mut layer {
            *v *= factor;
        }
        out = over(layer, out);
    }
    Ok(out)
}
