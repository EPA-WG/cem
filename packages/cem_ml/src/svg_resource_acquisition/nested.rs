//! Bounded graph of explicitly acquired nested snapshots. This is not a fetch,
//! browser delivery receipt, pixel decode validation or complete runtime graph.
use super::*;
use crate::{
    css_resources::{resolve_css_resources, CssResourceKind, CssResourcePlan},
    module_resolution::{
        CemModuleUrlReferrer, CemModuleUrlResolution, CemModuleUrlResolutionCapability,
        CemModuleUrlResolutionPurpose,
    },
};

pub struct NestedSnapshotBody {
    response: SvgResourceResponse,
    digest: String,
    tree: Option<Arc<RetainedCemTree>>,
}
impl NestedSnapshotBody {
    pub fn bytes(&self) -> &[u8] {
        &self.response.bytes
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn revision(&self) -> &str {
        &self.response.revision
    }
    pub fn content_type(&self) -> &str {
        &self.response.content_type
    }
    pub fn tree(&self) -> Option<&Arc<RetainedCemTree>> {
        self.tree.as_ref()
    }
}
pub struct NestedSnapshotEdge {
    pub source: Arc<RetainedCemTree>,
    pub node: AstNodeId,
    pub resolution: CemModuleUrlResolution,
    pub requires_css: bool,
}
pub struct NestedSnapshots {
    bodies: BTreeMap<String, NestedSnapshotBody>,
    edges: Vec<NestedSnapshotEdge>,
    pub(super) cancel: AbortSignal,
}
impl NestedSnapshots {
    pub fn bodies(&self) -> &BTreeMap<String, NestedSnapshotBody> {
        &self.bodies
    }
    pub fn edges(&self) -> &[NestedSnapshotEdge] {
        &self.edges
    }

    pub(super) fn capture(
        record: &SvgResourceRecord,
        cap: &CemModuleUrlResolutionCapability,
        responses: Vec<SvgResourceResponse>,
        cancel: AbortSignal,
    ) -> Result<Self, String> {
        active(&cancel)?;
        if responses.len() > 256 {
            return Err("nested snapshot count bound exceeded".into());
        }
        let mut total = 0usize;
        let mut pending_bodies = BTreeMap::new();
        for mut r in responses {
            active(&cancel)?;
            for size in [
                r.bytes.len(),
                r.requested_url.len(),
                r.final_url.len(),
                r.content_type.len(),
                r.revision.len(),
            ] {
                total = total
                    .checked_add(size)
                    .ok_or("nested snapshot byte overflow")?;
                if total > MAX_BYTES {
                    return Err("nested snapshot byte bound exceeded".into());
                }
            }
            let requested = absolute(&r.requested_url)?;
            let final_url = absolute(&r.final_url)?;
            if requested != final_url
                || final_url.fragment().is_some()
                || requested.fragment().is_some()
                || r.bytes.is_empty()
                || r.revision.trim().is_empty()
                || r.revision.len() > 4096
            {
                return Err("bounded nonredirected nested snapshot required".into());
            }
            r.content_type = r.content_type.to_ascii_lowercase();
            if !matches!(
                r.content_type.as_str(),
                "text/css"
                    | "image/svg+xml"
                    | "image/png"
                    | "image/jpeg"
                    | "image/gif"
                    | "image/webp"
                    | "image/avif"
                    | "font/woff"
                    | "font/woff2"
            ) {
                return Err("unsupported nested snapshot MIME".into());
            }
            r.requested_url = requested.to_string();
            r.final_url = final_url.to_string();
            if pending_bodies.insert(r.final_url.clone(), r).is_some() {
                return Err("duplicate nested snapshot URL".into());
            }
        }
        let mut edges = Vec::new();
        let content = record
            .content
            .as_ref()
            .ok_or("acquired content plan required")?;
        for (index, plan) in content.iter().enumerate() {
            for d in &plan.dependencies {
                let resolution = d
                    .resolution
                    .clone()
                    .map_err(|_| "unresolved nested dependency")?;
                let (source, node) = d
                    .css
                    .clone()
                    .unwrap_or((record.bindings[index].body.tree().clone(), d.source_node));
                let requires_css = crate::css_resources::named(&source, node, "import");
                push(
                    &mut edges,
                    NestedSnapshotEdge {
                        source,
                        node,
                        resolution,
                        requires_css,
                    },
                )?;
            }
        }
        let roots: BTreeSet<_> = record.bindings.iter().map(|b| b.body.url()).collect();
        let mut bodies = BTreeMap::new();
        let mut next = 0;
        let mut work = paint::Work(1_000_000);
        while next < edges.len() {
            active(&cancel)?;
            active(&record.cancel)?;
            let edge = &edges[next];
            next += 1;
            if edge.resolution.integrity.is_some() {
                return Err("nested integrity verification required".into());
            }
            let mut url = absolute(&edge.resolution.resolved_url)?;
            url.set_fragment(None);
            let key = url.to_string();
            if roots.contains(key.as_str()) {
                check_mime(edge, "image/svg+xml")?;
                if edge.requires_css {
                    return Err("CSS import targets SVG root snapshot".into());
                }
                continue;
            }
            if let Some(body) = bodies.get(&key) {
                let body: &NestedSnapshotBody = body;
                check_mime(edge, body.content_type())?;
                if edge.requires_css && body.content_type() != "text/css" {
                    return Err("nested CSS import MIME mismatch".into());
                }
                continue;
            }
            let response = pending_bodies
                .remove(&key)
                .ok_or("missing nested snapshot body")?;
            check_mime(edge, &response.content_type)?;
            if edge.requires_css && response.content_type != "text/css" {
                return Err("nested CSS import MIME mismatch".into());
            }
            let digest = ContentHash::from_blake3(&response.bytes).header_value();
            let mut tree = None;
            if response.content_type == "text/css" {
                let css =
                    crate::import::import_data_bytes(&response.bytes, "text/css", "cem", &key)?;
                if css
                    .ast_owner()
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation())
                {
                    return Err("invalid nested CSS".into());
                }
                append_css(&mut edges, &resolve_css_resources(css.clone(), cap, &key)?)?;
                tree = Some(css);
            } else if response.content_type == "image/svg+xml" {
                let doc = crate::import::import_data_bytes(
                    &response.bytes,
                    "image/svg+xml",
                    "cem",
                    &key,
                )?;
                if doc
                    .ast_owner()
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation())
                {
                    return Err("invalid nested SVG".into());
                }
                let root = doc
                    .node(0)
                    .and_then(|n| {
                        n.children.iter().find(|id| {
                            doc.node(**id)
                                .is_some_and(|n| n.kind == CemTreeNodeKind::Element)
                        })
                    })
                    .and_then(|id| doc.node(*id))
                    .and_then(|n| n.name.as_ref());
                if !root.is_some_and(|n| {
                    n.namespace_uri == "http://www.w3.org/2000/svg" && n.local_name == "svg"
                }) {
                    return Err("nested SVG root required".into());
                }
                let body = SvgAcquiredBody {
                    response,
                    tree: doc.clone(),
                    digest: digest.clone(),
                };
                let inventory = paint::inventory(&body, cap, &mut work, &cancel)?;
                for sheet in &inventory.stylesheets {
                    append_css(&mut edges, sheet.resources.as_ref().map_err(|e| e.clone())?)?;
                }
                for p in &inventory.presentations {
                    append_css(&mut edges, &p.resources)?;
                }
                for r in &inventory.document.references {
                    if doc
                        .node(r.attribute)
                        .and_then(|n| n.name.as_ref())
                        .is_some_and(|n| n.local_name == "href")
                        && crate::svg_paint_dependencies::href(&doc, r.owner) != Some(r.attribute)
                    {
                        continue;
                    }
                    let resolution = cap
                        .resolve_with_referrer(
                            CemModuleUrlResolutionPurpose::Css,
                            &r.authored_uri,
                            CemModuleUrlReferrer::Url(key.clone()),
                            r.source.clone(),
                        )
                        .map_err(|_| "unresolved nested SVG dependency")?;
                    push(
                        &mut edges,
                        NestedSnapshotEdge {
                            source: doc.clone(),
                            node: r.attribute,
                            resolution,
                            requires_css: false,
                        },
                    )?;
                }
                bodies.insert(
                    key,
                    NestedSnapshotBody {
                        response: body.response,
                        digest,
                        tree: Some(doc),
                    },
                );
                continue;
            }
            bodies.insert(
                key,
                NestedSnapshotBody {
                    response,
                    digest,
                    tree,
                },
            );
        }
        if !pending_bodies.is_empty() {
            return Err("unused nested snapshot body".into());
        }
        active(&cancel)?;
        active(&record.cancel)?;
        Ok(Self {
            bodies,
            edges,
            cancel,
        })
    }
}
fn push(edges: &mut Vec<NestedSnapshotEdge>, edge: NestedSnapshotEdge) -> Result<(), String> {
    if edges.len() >= 4096 {
        return Err("nested dependency edge bound exceeded".into());
    }
    edges.push(edge);
    Ok(())
}
fn append_css(edges: &mut Vec<NestedSnapshotEdge>, plan: &CssResourcePlan) -> Result<(), String> {
    for r in &plan.references {
        push(
            edges,
            NestedSnapshotEdge {
                source: plan.tree.clone(),
                node: r.node_id,
                resolution: r
                    .resolution
                    .clone()
                    .map_err(|_| "unresolved nested CSS dependency")?,
                requires_css: matches!(r.kind, CssResourceKind::Import { .. }),
            },
        )?;
    }
    Ok(())
}

fn check_mime(edge: &NestedSnapshotEdge, mime: &str) -> Result<(), String> {
    if edge
        .resolution
        .content_type_hint
        .as_ref()
        .is_some_and(|hint| !hint.eq_ignore_ascii_case(mime))
    {
        return Err("nested snapshot policy MIME mismatch".into());
    }
    Ok(())
}
