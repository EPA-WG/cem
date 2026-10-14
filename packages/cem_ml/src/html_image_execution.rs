//! Selected responsive-image observations and separately acquired opaque bytes.
//! A reacquisition is NOT proof of the browser's historical response identity.
use crate::{
    html_stylesheet_links::attribute,
    import::srcset::import_srcset_attribute,
    parser::{tree::RetainedCemTree, AstNodeId},
    scheduler::AbortSignal,
};
use std::{collections::BTreeMap, sync::Arc};
pub struct ImageTarget {
    pub owner: AstNodeId,
    pub urls: Vec<String>,
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
                    for candidate in &srcset.parsed().candidates {
                        if urls.len() >= 256 {
                            return Err("image candidate count exceeded".into());
                        }
                        urls.push(
                            base.join(&candidate.url)
                                .map_err(|_| "invalid image candidate URL")?
                                .to_string(),
                        );
                    }
                }
            }
            if urls.len() > 256 {
                return Err("image candidate count exceeded".into());
            }
            if let Some((_, src)) = attribute(&tree, id, "src") {
                if !src.is_empty() {
                    urls.push(
                        base.join(src)
                            .map_err(|_| "invalid image fallback URL")?
                            .to_string(),
                    );
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
            targets.push(ImageTarget { owner: id, urls });
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
