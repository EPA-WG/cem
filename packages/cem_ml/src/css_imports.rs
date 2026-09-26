//! Retained CSS import closure driven by the shared host loader.
//!
//! Requests carry resolved URLs and mapping metadata, never serialized ASTs.
//! Before delivery, the host must enforce transport, byte/MIME/integrity policy
//! and import the response through the shared CEM import boundary. This module
//! handles graph order, native ownership, final URL bases, limits and cancellation.
//! Ready means the import closure is complete, not that CSS is ready to install:
//! condition validation, resource rewriting and scoped compilation still follow.
use crate::{
    css_resources::{resolve_css_resources, CssResourceKind, CssResourcePlan},
    module_resolution::{
        CemModuleUrlReferrer, CemModuleUrlResolution, CemModuleUrlResolutionCapability,
        CemModuleUrlResolutionPurpose,
    },
    parser::{tree::RetainedCemTree, AstNodeId},
    scheduler::AbortSignal,
    source_map::SourceMapStack,
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssImportState {
    Pending,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct CssImportLimits {
    /// Root and repeated import occurrences count toward this bound.
    pub max_sheets: usize,
    /// The root is depth zero.
    pub max_depth: usize,
}
impl Default for CssImportLimits {
    fn default() -> Self {
        Self {
            max_sheets: 64,
            max_depth: 16,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CssImportFailure {
    pub code: String,
    pub message: String,
    pub source: SourceMapStack,
}

impl std::fmt::Display for CssImportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for CssImportFailure {}

#[derive(Debug, Clone)]
pub struct CssImportRequest {
    pub id: u64,
    pub parent_sheet: usize,
    pub import_node: AstNodeId,
    pub conditions: CssResourceKind,
    pub resolution: CemModuleUrlResolution,
    pub source: SourceMapStack,
}

#[derive(Debug)]
pub struct CssImportSheet {
    pub url: String,
    pub resources: CssResourcePlan,
    ancestry: Vec<String>,
    depth: usize,
}

#[derive(Debug)]
pub struct CssImportEdge {
    pub parent_sheet: usize,
    pub import_node: AstNodeId,
    pub child_sheet: usize,
    pub conditions: CssResourceKind,
}

pub struct CssImportClosure {
    capability: CemModuleUrlResolutionCapability,
    limits: CssImportLimits,
    abort: AbortSignal,
    sheets: Vec<CssImportSheet>,
    edges: Vec<CssImportEdge>,
    pending: Vec<(usize, usize)>,
    in_flight: Option<CssImportRequest>,
    next_id: u64,
    failure: Option<CssImportFailure>,
}

impl CssImportClosure {
    pub fn new(
        tree: Arc<RetainedCemTree>,
        url: &str,
        capability: CemModuleUrlResolutionCapability,
        limits: CssImportLimits,
        abort: AbortSignal,
    ) -> Result<Self, CssImportFailure> {
        if limits.max_sheets == 0 {
            return Err(failure(
                "cem.css.import_limit",
                "CSS closure requires room for its root",
            ));
        }
        if abort.is_aborted() {
            return Err(failure(
                "cem.css.import_cancelled",
                "CSS import closure was cancelled",
            ));
        }
        let url = canonical_url(url)?;
        let resources = resolve_css_resources(tree, &capability, &url)
            .map_err(|e| failure("cem.css.import_tree_invalid", e))?;
        let mut closure = Self {
            capability,
            limits,
            abort,
            sheets: vec![CssImportSheet {
                url: url.clone(),
                resources,
                ancestry: vec![url],
                depth: 0,
            }],
            edges: Vec::new(),
            pending: Vec::new(),
            in_flight: None,
            next_id: 1,
            failure: None,
        };
        closure.enqueue(0);
        Ok(closure)
    }

    pub fn state(&self) -> CssImportState {
        if self.failure.is_some() || self.abort.is_aborted() {
            CssImportState::Failed
        } else if self.pending.is_empty() && self.in_flight.is_none() {
            CssImportState::Ready
        } else {
            CssImportState::Pending
        }
    }
    /// Partial sheets remain inspectable after failure, but must not be compiled.
    pub fn sheets(&self) -> &[CssImportSheet] {
        &self.sheets
    }
    pub fn edges(&self) -> &[CssImportEdge] {
        &self.edges
    }
    pub fn failure(&self) -> Option<CssImportFailure> {
        self.failure.clone().or_else(|| {
            self.abort.is_aborted().then(|| {
                let mut error = failure(
                    "cem.css.import_cancelled",
                    "CSS import closure was cancelled",
                );
                error.source = self
                    .in_flight
                    .as_ref()
                    .map(|r| r.source.clone())
                    .unwrap_or_default();
                error
            })
        })
    }

    /// Depth-first, source-order traversal. Repeated imports remain distinct
    /// occurrences (including anonymous layers); hosts may reuse native trees.
    /// Only one request may be outstanding, so delivery cannot reorder imports.
    pub fn next_import(&mut self) -> Result<Option<CssImportRequest>, CssImportFailure> {
        self.check_active()?;
        if self.in_flight.is_some() {
            return Err(failure(
                "cem.css.import_delivery_pending",
                "complete the outstanding CSS import first",
            ));
        }
        let Some((parent, index)) = self.pending.pop() else {
            return Ok(None);
        };
        let sheet = &self.sheets[parent];
        let reference = &sheet.resources.references[index];
        let result = (|| {
            if self.sheets.len() >= self.limits.max_sheets || sheet.depth >= self.limits.max_depth {
                return Err(failure(
                    "cem.css.import_limit",
                    "CSS import closure exceeds its sheet/depth limit",
                ));
            }
            let resolution = reference
                .resolution
                .clone()
                .map_err(|e| failure("cem.css.import_resolution_failed", e.to_string()))?;
            if sheet
                .ancestry
                .contains(&canonical_url(&resolution.resolved_url)?)
            {
                return Err(failure(
                    "cem.css.import_cycle",
                    "CSS import refers to an ancestor stylesheet",
                ));
            }
            Ok(CssImportRequest {
                id: self.next_id,
                parent_sheet: parent,
                import_node: reference.node_id,
                conditions: reference.kind.clone(),
                resolution,
                source: reference.source.clone(),
            })
        })();
        match result {
            Ok(request) => {
                self.next_id += 1;
                self.in_flight = Some(request.clone());
                Ok(Some(request))
            }
            Err(mut error) => {
                error.source = reference.source.clone();
                self.stop(error)
            }
        }
    }

    /// Deliver only a loader-validated retained CSS tree. Redirects use their
    /// final URL as the new base and must still satisfy the resolution policy.
    pub fn complete_import(
        &mut self,
        id: u64,
        tree: Arc<RetainedCemTree>,
        final_url: &str,
    ) -> Result<(), CssImportFailure> {
        self.check_active()?;
        let request = self.request(id)?.clone();
        let result = (|| {
            let final_url = canonical_url(final_url)?;
            let checked = self
                .capability
                .resolve_with_referrer(
                    CemModuleUrlResolutionPurpose::Css,
                    &final_url,
                    CemModuleUrlReferrer::Url(request.resolution.resolved_url.clone()),
                    request.source.clone(),
                )
                .map_err(|e| failure("cem.css.import_redirect_denied", e.to_string()))?;
            if canonical_url(&checked.resolved_url)? != final_url {
                return Err(failure(
                    "cem.css.import_redirect_denied",
                    "final stylesheet URL is remapped by the active context",
                ));
            }
            let parent = &self.sheets[request.parent_sheet];
            if parent.ancestry.contains(&final_url) {
                return Err(failure(
                    "cem.css.import_cycle",
                    "CSS import redirects to an ancestor stylesheet",
                ));
            }
            let root = tree
                .node(0)
                .and_then(|document| (document.children.len() == 1).then(|| document.children[0]))
                .and_then(|id| tree.node(id))
                .and_then(|node| node.name.as_ref());
            if !root.is_some_and(|name| {
                name.namespace_uri == crate::schema::registry::CSS_SCHEMA_URI
                    && name.local_name == "stylesheet"
            }) {
                return Err(failure(
                    "cem.css.import_tree_invalid",
                    "an imported sheet requires the CSS stylesheet entry mode",
                ));
            }
            let resources = resolve_css_resources(tree, &self.capability, &final_url)
                .map_err(|e| failure("cem.css.import_tree_invalid", e))?;
            let mut ancestry = parent.ancestry.clone();
            ancestry.push(canonical_url(&request.resolution.resolved_url)?);
            if !ancestry.contains(&final_url) {
                ancestry.push(final_url.clone());
            }
            Ok(CssImportSheet {
                url: final_url,
                resources,
                ancestry,
                depth: parent.depth + 1,
            })
        })();
        match result {
            Ok(sheet) => {
                let child = self.sheets.len();
                self.sheets.push(sheet);
                self.edges.push(CssImportEdge {
                    parent_sheet: request.parent_sheet,
                    import_node: request.import_node,
                    child_sheet: child,
                    conditions: request.conditions,
                });
                self.in_flight = None;
                self.enqueue(child);
                Ok(())
            }
            Err(mut error) => {
                error.source = request.source;
                self.stop(error)
            }
        }
    }

    pub fn fail_import(
        &mut self,
        id: u64,
        code: &str,
        message: &str,
    ) -> Result<(), CssImportFailure> {
        self.check_active()?;
        let mut error = failure(code, message);
        error.source = self.request(id)?.source.clone();
        self.failure = Some(error);
        self.pending.clear();
        self.in_flight = None;
        Ok(())
    }

    fn request(&self, id: u64) -> Result<&CssImportRequest, CssImportFailure> {
        self.in_flight
            .as_ref()
            .filter(|r| r.id == id)
            .ok_or_else(|| {
                failure(
                    "cem.css.import_delivery_invalid",
                    "CSS import delivery does not match the outstanding request",
                )
            })
    }
    fn enqueue(&mut self, sheet: usize) {
        for (index, reference) in self.sheets[sheet]
            .resources
            .references
            .iter()
            .enumerate()
            .rev()
        {
            if matches!(reference.kind, CssResourceKind::Import { .. }) {
                self.pending.push((sheet, index));
            }
        }
    }
    fn check_active(&mut self) -> Result<(), CssImportFailure> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.abort.is_aborted() {
            let mut error = failure(
                "cem.css.import_cancelled",
                "CSS import closure was cancelled",
            );
            error.source = self
                .in_flight
                .as_ref()
                .map(|r| r.source.clone())
                .unwrap_or_default();
            return self.stop(error);
        }
        Ok(())
    }
    fn stop<T>(&mut self, error: CssImportFailure) -> Result<T, CssImportFailure> {
        self.failure = Some(error.clone());
        self.pending.clear();
        self.in_flight = None;
        Err(error)
    }
}

fn canonical_url(value: &str) -> Result<String, CssImportFailure> {
    let mut url =
        url::Url::parse(value).map_err(|e| failure("cem.css.import_url_invalid", e.to_string()))?;
    // A fragment does not select a different stylesheet response or break cycles.
    url.set_fragment(None);
    Ok(url.to_string())
}
fn failure(code: &str, message: impl Into<String>) -> CssImportFailure {
    CssImportFailure {
        code: code.into(),
        message: message.into(),
        source: SourceMapStack::default(),
    }
}
