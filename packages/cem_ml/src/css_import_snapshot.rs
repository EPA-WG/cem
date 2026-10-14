//! Immutable acquired CSS responses and sealed, inspection-only import closure.
//! No resolver registry, filesystem or network fallback participates in capture.
use crate::{
    css_imports::{
        CssImportClosure, CssImportFailure, CssImportLimits, CssImportResponsePolicy,
        CssImportState,
    },
    module_resolution::CemModuleUrlResolutionCapability,
    parser::tree::RetainedCemTree,
    resolver::ResolvedRead,
    scheduler::AbortSignal,
    source_map::SourceMapStack,
};
use std::{collections::BTreeMap, sync::Arc};

/// Trusted host acquisition metadata. Revisions are opaque; the host must supply
/// its current immutable snapshot again at consumption, including after edits.
#[derive(Clone, Debug)]
pub struct CssSnapshotSource {
    pub requested_url: String,
    pub final_url: String,
    pub revision: String,
    pub bytes: Vec<u8>,
    pub content_type: String,
}
#[derive(Clone, Copy)]
pub struct CssSnapshotLimits {
    pub max_sources: usize,
    pub max_bytes: usize,
}
impl Default for CssSnapshotLimits {
    fn default() -> Self {
        Self {
            max_sources: 256,
            max_bytes: 16 * 1024 * 1024,
        }
    }
}
pub struct CssSourceSnapshot {
    sources: BTreeMap<String, Arc<CssSnapshotSource>>,
}
impl CssSourceSnapshot {
    pub fn new(
        sources: Vec<CssSnapshotSource>,
        limits: CssSnapshotLimits,
    ) -> Result<Arc<Self>, String> {
        if sources.len() > limits.max_sources {
            return Err("CSS snapshot source limit exceeded".into());
        }
        let mut retained = BTreeMap::new();
        let mut finals: BTreeMap<String, Arc<CssSnapshotSource>> = BTreeMap::new();
        let mut bytes = 0usize;
        for mut source in sources {
            for value in [&mut source.requested_url, &mut source.final_url] {
                let url =
                    url::Url::parse(value).map_err(|_| "absolute CSS snapshot URL required")?;
                if url.fragment().is_some() {
                    return Err("fragment-free CSS snapshot URLs required".into());
                }
                *value = url.to_string();
            }
            if source.revision.trim().is_empty() || source.content_type.trim().is_empty() {
                return Err("CSS snapshot revision and content type required".into());
            }
            for size in [
                source.bytes.len(),
                source.requested_url.len(),
                source.final_url.len(),
                source.revision.len(),
                source.content_type.len(),
            ] {
                bytes = bytes
                    .checked_add(size)
                    .ok_or("CSS snapshot byte count overflow")?;
            }
            if bytes > limits.max_bytes {
                return Err("CSS snapshot byte limit exceeded".into());
            }
            if let Some(previous) = finals.get(&source.final_url) {
                if previous.revision != source.revision
                    || previous.bytes != source.bytes
                    || previous.content_type != source.content_type
                {
                    return Err("conflicting CSS snapshot aliases for one final URL".into());
                }
            }
            let source = Arc::new(source);
            finals.insert(source.final_url.clone(), source.clone());
            if retained
                .insert(source.requested_url.clone(), source)
                .is_some()
            {
                return Err("duplicate CSS snapshot request URL".into());
            }
        }
        Ok(Arc::new(Self { sources: retained }))
    }
    pub fn source(&self, requested_url: &str) -> Option<&Arc<CssSnapshotSource>> {
        self.sources.get(requested_url)
    }
}

#[derive(Clone, Copy)]
pub struct CssSnapshotCaptureLimits {
    pub imports: CssImportLimits,
    pub responses: CssImportResponsePolicy,
    /// Counts native root nodes before any resolution. Imported responses retain
    /// the shared CSS importer's byte, event and depth bounds.
    pub max_root_nodes: usize,
}
impl Default for CssSnapshotCaptureLimits {
    fn default() -> Self {
        Self {
            imports: Default::default(),
            responses: Default::default(),
            max_root_nodes: 65_536,
        }
    }
}
pub struct CssSnapshotAcquisition {
    pub sheet: usize,
    pub source: Arc<CssSnapshotSource>,
}

/// Only successful snapshot capture constructs this record. The root is retained
/// from its caller's execution/source owner; imported revisions come from snapshot.
pub struct SnapshotCssImportClosure {
    closure: Arc<CssImportClosure>,
    snapshot: Arc<CssSourceSnapshot>,
    capability: CemModuleUrlResolutionCapability,
    acquisitions: Vec<CssSnapshotAcquisition>,
    blockers: Vec<String>,
}
impl SnapshotCssImportClosure {
    pub fn closure(&self) -> &CssImportClosure {
        &self.closure
    }
    /// Exact ready closure owner for native compilation; cloning this Arc does
    /// not reconstruct sources or waive snapshot/policy validation.
    pub fn closure_owner(&self) -> &Arc<CssImportClosure> {
        &self.closure
    }
    pub fn acquisitions(&self) -> &[CssSnapshotAcquisition] {
        &self.acquisitions
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
    pub fn validate_current(
        &self,
        root: &Arc<RetainedCemTree>,
        root_url: &str,
        capability: &CemModuleUrlResolutionCapability,
        snapshot: &Arc<CssSourceSnapshot>,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(snapshot, &self.snapshot)
            || !Arc::ptr_eq(root, &self.closure.sheets()[0].resources.tree)
            || canonical_url(root_url)? != self.closure.sheets()[0].url
            || !self.capability.same_owner(capability)
        {
            return Err("CSS snapshot, root, base or resolver owner changed".into());
        }
        // state() latches observed resolver drift, including an attempted A-B-A.
        if self.closure.state() != CssImportState::Ready
            || self.closure.stable_resolution_identity().is_none()
        {
            return Err(self
                .closure
                .failure()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "CSS snapshot closure is not current and ready".into()));
        }
        Ok(())
    }
}

pub fn capture_css_import_snapshot(
    root: Arc<RetainedCemTree>,
    root_url: &str,
    capability: &CemModuleUrlResolutionCapability,
    snapshot: Arc<CssSourceSnapshot>,
    limits: CssSnapshotCaptureLimits,
    abort: AbortSignal,
) -> Result<SnapshotCssImportClosure, CssImportFailure> {
    if capability
        .cache_identity()
        .is_none_or(|identity| identity.trim().is_empty())
    {
        return Err(failure(
            "cem.css.snapshot_fingerprint",
            "complete CSS resolver fingerprint required",
        ));
    }
    if root.ast().nodes.len() > limits.max_root_nodes {
        return Err(failure(
            "cem.css.snapshot_root_limit",
            "retained CSS root node limit exceeded",
        ));
    }
    if root
        .ast_owner()
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        return Err(failure(
            "cem.css.snapshot_root_invalid",
            "retained CSS root has import diagnostics",
        ));
    }
    let mut closure = CssImportClosure::new(
        root.clone(),
        root_url,
        capability.clone(),
        limits.imports,
        abort,
    )?;
    let mut acquisitions = Vec::new();
    while let Some(request) = closure.next_import()? {
        let key = canonical_url(&request.resolution.resolved_url)
            .map_err(|e| failure("cem.css.snapshot_url", e))?;
        let Some(source) = snapshot.source(&key) else {
            closure.fail_import(
                request.id,
                "cem.css.snapshot_missing",
                "CSS response absent from immutable snapshot; fallback I/O forbidden",
            )?;
            return Err(closure.failure().unwrap());
        };
        // The existing loader validates MIME, integrity, bytes, final URL policy,
        // cycles and placement before attaching the imported native tree.
        closure.complete_response(
            request.id,
            ResolvedRead {
                uri: source.final_url.clone(),
                bytes: source.bytes.clone(),
                content_type: Some(source.content_type.clone()),
            },
            &limits.responses,
        )?;
        acquisitions.push(CssSnapshotAcquisition {
            sheet: closure.sheets().len() - 1,
            source: source.clone(),
        });
    }
    let mut blockers = vec![
        "css-conditions-and-execution-history-required".into(),
        "css-runtime-dependency-closure-required".into(),
        "atomic-host-transaction-required".into(),
    ];
    if closure.sheets().iter().any(|sheet| {
        sheet
            .resources
            .references
            .iter()
            .any(|r| r.resolution.is_err())
    }) {
        blockers.push("css-resource-resolution-incomplete".into());
    }
    let result = SnapshotCssImportClosure {
        closure: Arc::new(closure),
        snapshot,
        capability: capability.clone(),
        acquisitions,
        blockers,
    };
    result
        .validate_current(&root, root_url, capability, &result.snapshot)
        .map_err(|e| failure("cem.css.snapshot_stale", e))?;
    Ok(result)
}
pub(crate) fn canonical_url(value: &str) -> Result<String, String> {
    let mut url = url::Url::parse(value).map_err(|e| e.to_string())?;
    url.set_fragment(None);
    Ok(url.to_string())
}
fn failure(code: &str, message: impl Into<String>) -> CssImportFailure {
    CssImportFailure {
        code: code.into(),
        message: message.into(),
        source: SourceMapStack::default(),
        location: None,
    }
}
