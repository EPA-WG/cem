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
    fragments: Vec<NestedFragmentTarget>,
    pub(super) cancel: AbortSignal,
}
/// Literal target identity only. Does not assert that the referring CSS/SVG
/// operation supports this element, or that the browser consumed it.
pub struct NestedFragmentTarget {
    pub edge: usize,
    pub tree: Arc<RetainedCemTree>,
    pub target: AstNodeId,
}
impl NestedSnapshots {
    pub fn fragment_targets(&self) -> &[NestedFragmentTarget] {
        &self.fragments
    }
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
        let mut integrity = IntegrityChecks::default();
        let mut work = paint::Work(1_000_000);
        while next < edges.len() {
            active(&cancel)?;
            active(&record.cancel)?;
            let edge = &edges[next];
            next += 1;
            let mut url = absolute(&edge.resolution.resolved_url)?;
            url.set_fragment(None);
            let key = url.to_string();
            if roots.contains(key.as_str()) {
                let body = &record
                    .bindings
                    .iter()
                    .find(|b| b.body.url() == key)
                    .unwrap()
                    .body;
                integrity.check(&key, edge, body.bytes())?;
                check_mime(edge, "image/svg+xml")?;
                if edge.requires_css {
                    return Err("CSS import targets SVG root snapshot".into());
                }
                continue;
            }
            if let Some(body) = bodies.get(&key) {
                let body: &NestedSnapshotBody = body;
                integrity.check(&key, edge, body.bytes())?;
                check_mime(edge, body.content_type())?;
                if edge.requires_css && body.content_type() != "text/css" {
                    return Err("nested CSS import MIME mismatch".into());
                }
                continue;
            }
            let response = pending_bodies
                .remove(&key)
                .ok_or("missing nested snapshot body")?;
            integrity.check(&key, edge, &response.bytes)?;
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
        let mut fragments = Vec::new();
        let mut indexes = BTreeMap::new();
        for (index, edge) in edges.iter().enumerate() {
            active(&cancel)?;
            active(&record.cancel)?;
            let mut url = absolute(&edge.resolution.resolved_url)?;
            let Some(fragment) = url.fragment().map(str::to_owned) else {
                continue;
            };
            url.set_fragment(None);
            let key = url.to_string();
            let tree = if let Some(binding) = record.bindings.iter().find(|b| b.body.url() == key) {
                binding.body.tree().clone()
            } else {
                let body = bodies.get(&key).ok_or("nested fragment body missing")?;
                // Non-SVG fragments retain their original URL, with no fabricated
                // SVG target proof (e.g. font fragments or raster media fragments).
                if body.content_type() != "image/svg+xml" {
                    continue;
                }
                body.tree().ok_or("nested SVG tree missing")?.clone()
            };
            if !crate::svg_paint_dependencies::literal_fragment(&fragment) {
                return Err("unsupported nested SVG fragment".into());
            }
            if !indexes.contains_key(&key) {
                let mut ids: BTreeMap<String, Vec<AstNodeId>> = BTreeMap::new();
                let mut pending = vec![0];
                while let Some(id) = pending.pop() {
                    active(&cancel)?;
                    let node = tree.node(id).ok_or("nested SVG node missing")?;
                    work.spend(1 + node.children.len() + node.attributes.len())?;
                    for attr in &node.attributes {
                        let a = tree.node(*attr).ok_or("nested SVG attribute missing")?;
                        work.spend(1 + a.value.len())?;
                        if node.kind == CemTreeNodeKind::Element
                            && a.name
                                .as_ref()
                                .is_some_and(|n| n.namespace_uri.is_empty() && n.local_name == "id")
                        {
                            ids.entry(a.value.clone()).or_default().push(id);
                        }
                    }
                    pending.extend(node.children.iter().copied());
                }
                indexes.insert(key.clone(), ids);
            }
            let targets = indexes[&key]
                .get(&fragment)
                .ok_or("missing nested SVG fragment target")?;
            if targets.len() != 1 {
                return Err("ambiguous nested SVG fragment target".into());
            }
            fragments.push(NestedFragmentTarget {
                edge: index,
                tree,
                target: targets[0],
            });
        }
        active(&cancel)?;
        active(&record.cancel)?;
        Ok(Self {
            bodies,
            edges,
            fragments,
            cancel,
        })
    }
}
// Bound hashing even when many graph edges reuse a large body. Every distinct
// expectation must pass; a prior success never blesses another edge's metadata.
#[derive(Default)]
struct IntegrityChecks {
    verified: BTreeSet<(String, String)>,
    bytes: usize,
}
impl IntegrityChecks {
    fn check(&mut self, url: &str, edge: &NestedSnapshotEdge, bytes: &[u8]) -> Result<(), String> {
        if let Some(metadata) = &edge.resolution.integrity {
            if metadata.len() > 4096 {
                return Err("nested integrity metadata bound exceeded".into());
            }
            let key = (url.to_owned(), metadata.clone());
            if self.verified.contains(&key) {
                return Ok(());
            }
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .ok_or("integrity work overflow")?;
            if self.bytes > 64 * 1024 * 1024 {
                return Err("nested integrity work bound exceeded".into());
            }
            crate::resource_integrity::verify_resource_integrity(bytes, metadata)?;
            self.verified.insert(key);
        }
        Ok(())
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
