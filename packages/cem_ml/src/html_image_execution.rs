//! Selected responsive-image observations and separately acquired opaque bytes.
//! A reacquisition is NOT proof of the browser's historical response identity.
use crate::{
    html_stylesheet_links::attribute,
    import::srcset::{import_srcset_attribute, SrcsetCandidate},
    parser::{tree::RetainedCemTree, AstNodeId},
    scheduler::AbortSignal,
};
use std::{collections::BTreeMap, sync::Arc};
/// One authored source occurrence, retained before URL deduplication.
/// Candidate ranges address the decoded native attribute, not source-file edits.
pub struct ImageOccurrence {
    pub owner: AstNodeId,
    pub attribute: AstNodeId,
    pub ordinal: usize,
    pub url: String,
    /// None identifies the img src fallback.
    pub candidate: Option<SrcsetCandidate>,
}
pub struct ImageTarget {
    pub owner: AstNodeId,
    pub urls: Vec<String>,
    pub occurrences: Vec<ImageOccurrence>,
}
pub struct ImagePlan {
    tree: Arc<RetainedCemTree>,
    targets: Vec<ImageTarget>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageObservation {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub complete: bool,
}
pub struct ImageResponse {
    pub requested_url: String,
    pub final_url: String,
    pub content_type: String,
    pub revision: String,
    pub bytes: Vec<u8>,
}
pub struct ImageRecord {
    plan: Arc<ImagePlan>,
    responses: Vec<ImageResponse>,
    observations: Vec<ImageObservation>,
    environment: String,
    cancel: AbortSignal,
    revoked: bool,
}
fn named(tree: &RetainedCemTree, id: AstNodeId, name: &str) -> bool {
    tree.node(id)
        .and_then(|n| n.name.as_ref())
        .is_some_and(|n| n.namespace_uri == "http://www.w3.org/1999/xhtml" && n.local_name == name)
}
impl ImagePlan {
    /// Inspection only. The execution owner must separately validate full DOM,
    /// effective base and policy before consuming this plan or its observations.
    pub fn from_tree(
        tree: Arc<RetainedCemTree>,
        base: &str,
        nodes: usize,
        images: usize,
    ) -> Result<Arc<Self>, String> {
        if nodes == 0 || nodes > 65_536 || images == 0 || images > 256 || base.len() > 4096 {
            return Err("bounded image inventory required".into());
        }
        let base = url::Url::parse(base).map_err(|_| "absolute image base required")?;
        let mut pending = vec![0];
        let mut remaining = nodes;
        let mut targets = Vec::new();
        let mut url_bytes = 4usize * 1024 * 1024;
        while let Some(id) = pending.pop() {
            remaining = remaining
                .checked_sub(1)
                .ok_or("image node budget exceeded")?;
            let node = tree.node(id).ok_or("missing image inventory node")?;
            if named(&tree, id, "template") {
                continue;
            }
            pending.extend(node.children.iter().rev());
            if !named(&tree, id, "img") {
                continue;
            }
            let mut owners = Vec::new();
            if let Some(parent) = node.parent.filter(|p| named(&tree, *p, "picture")) {
                for sibling in &tree.node(parent).unwrap().children {
                    if *sibling == id {
                        break;
                    }
                    if named(&tree, *sibling, "source") {
                        owners.push(*sibling);
                    }
                }
            }
            owners.push(id);
            let mut urls = Vec::new();
            let mut occurrences = Vec::new();
            for owner in owners {
                if let Some((attr, _)) = attribute(&tree, owner, "srcset") {
                    let srcset = import_srcset_attribute(
                        tree.clone(),
                        owner,
                        attr,
                        crate::import::srcset::SrcsetLimits {
                            max_bytes: 65_536,
                            max_candidates: 256,
                        },
                    )?;
                    if !srcset.parsed().complete || !srcset.parsed().issues.is_empty() {
                        return Err("complete well-formed image candidate profile required".into());
                    }
                    for (ordinal, candidate) in srcset.parsed().candidates.iter().enumerate() {
                        if urls.len() >= 256 {
                            return Err("image candidate count exceeded".into());
                        }
                        let url = base
                            .join(&candidate.url)
                            .map_err(|_| "invalid image candidate URL")?
                            .to_string();
                        occurrences.push(ImageOccurrence {
                            owner,
                            attribute: attr,
                            ordinal,
                            url: url.clone(),
                            candidate: Some(candidate.clone()),
                        });
                        urls.push(url);
                    }
                }
            }
            if urls.len() > 256 {
                return Err("image candidate count exceeded".into());
            }
            if let Some((attr, src)) = attribute(&tree, id, "src") {
                if !src.is_empty() {
                    if occurrences.len() >= 256 {
                        return Err("image candidate count exceeded".into());
                    }
                    let url = base
                        .join(src)
                        .map_err(|_| "invalid image fallback URL")?
                        .to_string();
                    occurrences.push(ImageOccurrence {
                        owner: id,
                        attribute: attr,
                        ordinal: 0,
                        url: url.clone(),
                        candidate: None,
                    });
                    urls.push(url);
                }
            }
            for url in &urls {
                if url.len() > 4096 {
                    return Err("image URL byte limit exceeded".into());
                }
                url_bytes = url_bytes
                    .checked_sub(url.len())
                    .ok_or("image candidate byte limit exceeded")?;
            }
            urls.sort();
            urls.dedup();
            if urls.is_empty() {
                continue;
            }
            if urls.len() > 256 || targets.len() >= images {
                return Err("image candidate/count budget exceeded".into());
            }
            targets.push(ImageTarget {
                owner: id,
                urls,
                occurrences,
            });
        }
        Ok(Arc::new(Self { tree, targets }))
    }
    pub fn targets(&self) -> &[ImageTarget] {
        &self.targets
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn observe(
        &self,
        mut observe: impl FnMut(AstNodeId) -> Result<ImageObservation, String>,
    ) -> Result<Vec<ImageObservation>, String> {
        self.targets
            .iter()
            .map(|target| {
                let value = observe(target.owner)?;
                let url = url::Url::parse(&value.url)
                    .map_err(|_| "absolute selected image URL required")?;
                if !matches!(url.scheme(), "http" | "https")
                    || url.fragment().is_some()
                    || !target.urls.contains(&value.url)
                    || !value.complete
                    || value.width == 0
                    || value.height == 0
                {
                    return Err("loaded native image candidate required".into());
                }
                // currentSrc identifies a request URL, never the winning authored
                // occurrence. Do not guess from descriptor, dimensions or source order.
                if target
                    .occurrences
                    .iter()
                    .filter(|c| c.url == value.url)
                    .count()
                    != 1
                {
                    return Err("ambiguous selected image candidate occurrence".into());
                }
                Ok(value)
            })
            .collect()
    }
}
impl ImageRecord {
    pub fn capture(
        plan: Arc<ImagePlan>,
        responses: Vec<ImageResponse>,
        environment: &str,
        mut observe: impl FnMut(AstNodeId) -> Result<ImageObservation, String>,
        cancel: AbortSignal,
        max_bytes: usize,
    ) -> Result<Self, String> {
        if cancel.is_aborted()
            || environment.is_empty()
            || environment.len() > 4096
            || responses.len() > 256
            || max_bytes == 0
            || max_bytes > 4 * 1024 * 1024
        {
            return Err("bounded live image acquisition required".into());
        }
        let observations = plan.observe(&mut observe)?;
        let mut sources = BTreeMap::new();
        let mut remaining = max_bytes;
        for response in &responses {
            for url in [&response.requested_url, &response.final_url] {
                let parsed =
                    url::Url::parse(url).map_err(|_| "absolute image acquisition URL required")?;
                if !matches!(parsed.scheme(), "http" | "https")
                    || parsed.fragment().is_some()
                    || url.len() > 4096
                {
                    return Err("HTTP(S) image acquisition required".into());
                }
            }
            let mime = response
                .content_type
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if !matches!(
                mime.as_str(),
                "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/avif"
            ) || response.bytes.is_empty()
                || response.revision.is_empty()
                || response.revision.len() > 4096
                || response.content_type.len() > 256
            {
                return Err("bounded raster image response metadata required".into());
            }
            remaining = remaining
                .checked_sub(response.bytes.len())
                .ok_or("image acquisition byte budget exceeded")?;
            if sources.insert(&response.requested_url, response).is_some() {
                return Err("duplicate image acquisition".into());
            }
        }
        if observations.iter().any(|o| !sources.contains_key(&o.url))
            || sources
                .keys()
                .any(|url| !observations.iter().any(|o| &o.url == *url))
            || observations != plan.observe(observe)?
            || cancel.is_aborted()
        {
            return Err("complete stable selected image acquisition required".into());
        }
        Ok(Self {
            plan,
            responses,
            observations,
            environment: environment.into(),
            cancel,
            revoked: false,
        })
    }
    pub fn responses(&self) -> &[ImageResponse] {
        &self.responses
    }
    pub fn validate(
        &mut self,
        plan: &Arc<ImagePlan>,
        environment: &str,
        observe: impl FnMut(AstNodeId) -> Result<ImageObservation, String>,
    ) -> Result<(), String> {
        let result = (|| {
            if self.revoked
                || self.cancel.is_aborted()
                || !Arc::ptr_eq(plan, &self.plan)
                || environment != self.environment
                || plan.observe(observe)? != self.observations
                || self.cancel.is_aborted()
            {
                return Err("stale image selection/acquisition record".into());
            }
            Ok(())
        })();
        if result.is_err() {
            self.revoked = true;
        }
        result
    }
}

/// Byte ingress from a transport that withholds its response until acceptance.
/// This ledger is execution-local; neither an acknowledgement nor its digest is
/// a transferable seal, and it does not prove cache-bypassed image selection.
pub struct ImageDeliveryCapture {
    responses: BTreeMap<String, ImageResponse>,
    cancel: AbortSignal,
    bytes: usize,
    closed: bool,
}
impl ImageDeliveryCapture {
    pub fn new(cancel: AbortSignal) -> Self {
        Self {
            responses: BTreeMap::new(),
            cancel,
            bytes: 0,
            closed: false,
        }
    }
    pub fn accept(&mut self, mut response: ImageResponse) -> Result<(), String> {
        use sha2::{Digest, Sha256};
        let result = (|| {
            if self.closed || self.cancel.is_aborted() {
                return Err("closed image delivery capture".into());
            }
            let url = url::Url::parse(&response.requested_url)
                .map_err(|_| "absolute delivery URL required")?;
            let mime = response
                .content_type
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if !matches!(url.scheme(), "http" | "https")
                || url.fragment().is_some()
                || response.requested_url != response.final_url
                || response.requested_url.len() > 4096
                || response.content_type.len() > 256
                || response.bytes.is_empty()
                || response.bytes.len() > 4 * 1024 * 1024
                || !matches!(
                    mime.as_str(),
                    "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/avif"
                )
            {
                return Err("bounded direct raster delivery required".into());
            }
            if let Some(previous) = self.responses.get(&response.requested_url) {
                if previous.bytes != response.bytes
                    || previous.content_type != response.content_type
                {
                    return Err("image delivery body changed within execution".into());
                }
                return Ok(());
            }
            if self.responses.len() >= 256
                || self.bytes.saturating_add(response.bytes.len()) > 4 * 1024 * 1024
            {
                return Err("image delivery budget exceeded".into());
            }
            response.revision = format!("{:x}", Sha256::digest(&response.bytes));
            self.bytes += response.bytes.len();
            self.responses
                .insert(response.requested_url.clone(), response);
            Ok(())
        })();
        if result.is_err() {
            self.closed = true;
            self.responses.clear();
        }
        result
    }
    /// Close byte ingress before publication; select only responses demanded by
    /// the native plan, allowing earlier original-document loads to be unused.
    pub fn seal(&mut self, selected: &[String]) -> Result<Vec<ImageResponse>, String> {
        let valid = !self.closed
            && !self.cancel.is_aborted()
            && selected.len() <= 256
            && selected.iter().all(|url| self.responses.contains_key(url));
        self.closed = true;
        let responses = std::mem::take(&mut self.responses);
        if !valid {
            return Err("complete live image delivery receipts required".into());
        }
        Ok(responses
            .into_iter()
            .filter(|(url, _)| selected.contains(url))
            .map(|(_, response)| response)
            .collect())
    }
}

/// Explicit new-delivery URLs. The source tree and parsed occurrence provenance
/// remain native; these routes are transport controls, never historical seals.
pub struct ImageReplayRoute {
    pub url: String,
    pub source: String,
    pub owner: AstNodeId,
    pub attribute: AstNodeId,
    pub ordinal: usize,
}
pub struct ImageReplayPlan {
    plan: Arc<ImagePlan>,
    routes: Vec<ImageReplayRoute>,
    replacements: Vec<(AstNodeId, AstNodeId, String)>,
}
impl ImageReplayPlan {
    pub fn new(
        tree: Arc<RetainedCemTree>,
        base: &str,
        document: &str,
        nonce: &str,
    ) -> Result<Self, String> {
        if nonce.len() != 32
            || !nonce
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err("fresh bounded image replay nonce required".into());
        }
        let document =
            url::Url::parse(document).map_err(|_| "absolute replay document required")?;
        if !matches!(document.scheme(), "http" | "https") {
            return Err("HTTP replay document required".into());
        }
        let plan = ImagePlan::from_tree(tree, base, 10_000, 256)?;
        let mut routes = Vec::new();
        let mut edits =
            BTreeMap::<AstNodeId, (AstNodeId, Vec<(std::ops::Range<usize>, String)>)>::new();
        let mut seen = std::collections::BTreeSet::new();
        for target in plan.targets() {
            for occurrence in &target.occurrences {
                if !seen.insert((occurrence.attribute, occurrence.ordinal)) {
                    continue;
                }
                let source = url::Url::parse(&occurrence.url)
                    .map_err(|_| "absolute replay source required")?;
                if source.origin() != document.origin()
                    || source.fragment().is_some()
                    || !source.username().is_empty()
                    || source.password().is_some()
                    || source.path().starts_with("/__cem_image_delivery/")
                    || routes.len() >= 256
                {
                    return Err("bounded same-origin original replay sources required".into());
                }
                let alias = document
                    .join(&format!("/__cem_image_delivery/{nonce}/{}", routes.len()))
                    .map_err(|_| "invalid replay route")?
                    .to_string();
                let attribute = plan
                    .tree()
                    .node(occurrence.attribute)
                    .ok_or("replay source attribute missing")?;
                let range = occurrence
                    .candidate
                    .as_ref()
                    .map_or(0..attribute.value.len(), |c| c.url_range.clone());
                edits
                    .entry(occurrence.attribute)
                    .or_insert_with(|| (occurrence.owner, vec![]))
                    .1
                    .push((range, alias.clone()));
                routes.push(ImageReplayRoute {
                    url: alias,
                    source: source.to_string(),
                    owner: occurrence.owner,
                    attribute: occurrence.attribute,
                    ordinal: occurrence.ordinal,
                });
            }
        }
        let mut replacements = Vec::new();
        let mut budget = 4usize * 1024 * 1024;
        for (attribute, (owner, mut edits)) in edits {
            let mut value = plan.tree().node(attribute).unwrap().value.clone();
            edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
            for (range, replacement) in edits {
                value.replace_range(range, &replacement);
            }
            budget = budget
                .checked_sub(value.len())
                .ok_or("replay attribute byte bound exceeded")?;
            replacements.push((owner, attribute, value));
        }
        Ok(Self {
            plan,
            routes,
            replacements,
        })
    }
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        self.plan.tree()
    }
    pub fn routes(&self) -> &[ImageReplayRoute] {
        &self.routes
    }
    pub fn replacements(&self) -> &[(AstNodeId, AstNodeId, String)] {
        &self.replacements
    }
}
