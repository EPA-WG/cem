//! Immutable, bounded template sources. Snapshot preflight never falls back to I/O.
use crate::{
    engine::{EngineContext, FormatIdentity, TemplateInput},
    real::NativeTemplateDependencyInspection,
    resolver::{ResolvedRead, ResolverDiagnostic},
    run_config::ScopeConfig,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct TemplateSnapshotSource {
    pub uri: String,
    pub revision: String,
    pub bytes: Vec<u8>,
    pub content_type: String,
}
#[derive(Clone, Copy)]
pub struct TemplateSnapshotLimits {
    pub max_sources: usize,
    pub max_bytes: usize,
    pub max_imports: usize,
}
impl Default for TemplateSnapshotLimits {
    fn default() -> Self {
        Self {
            max_sources: 256,
            max_bytes: 16 * 1024 * 1024,
            max_imports: 256,
        }
    }
}
pub struct TemplateSourceSnapshot {
    sources: BTreeMap<String, TemplateSnapshotSource>,
    max_imports: usize,
    max_bytes: usize,
}
impl TemplateSourceSnapshot {
    pub fn new(
        sources: Vec<TemplateSnapshotSource>,
        limits: TemplateSnapshotLimits,
    ) -> Result<Arc<Self>, String> {
        if sources.is_empty() || sources.len() > limits.max_sources {
            return Err("nonempty bounded template snapshot required".into());
        }
        let mut retained = BTreeMap::new();
        let mut bytes = 0usize;
        for mut source in sources {
            let uri = url::Url::parse(&source.uri)
                .map_err(|_| "absolute snapshot source URI required")?;
            if uri.fragment().is_some()
                || source.revision.trim().is_empty()
                || source.content_type.trim().is_empty()
            {
                return Err(
                    "snapshot sources require revisions, content types and fragment-free URIs"
                        .into(),
                );
            }
            source.uri = uri.to_string();
            bytes = bytes
                .saturating_add(source.bytes.len())
                .saturating_add(source.uri.len())
                .saturating_add(source.revision.len())
                .saturating_add(source.content_type.len());
            if bytes > limits.max_bytes {
                return Err("template snapshot byte budget exceeded".into());
            }
            if retained.insert(source.uri.clone(), source).is_some() {
                return Err("duplicate template snapshot source URI".into());
            }
        }
        Ok(Arc::new(Self {
            sources: retained,
            max_imports: limits.max_imports,
            max_bytes: limits.max_bytes,
        }))
    }
    pub fn source(&self, uri: &str) -> Option<&TemplateSnapshotSource> {
        self.sources.get(uri)
    }
    pub(crate) fn max_imports(&self) -> usize {
        self.max_imports
    }
    pub(crate) fn max_bytes(&self) -> usize {
        self.max_bytes
    }
    pub(crate) fn read(&self, uri: &str, base: &str) -> Result<ResolvedRead, ResolverDiagnostic> {
        let resolved = url::Url::parse(base)
            .and_then(|base| base.join(uri))
            .map_err(|e| ResolverDiagnostic::Io {
                uri: uri.into(),
                message: format!("snapshot URI resolution failed: {e}"),
            })?;
        let source = self
            .source(resolved.as_str())
            .ok_or_else(|| ResolverDiagnostic::Io {
                uri: resolved.to_string(),
                message: "resource absent from immutable template snapshot; fallback I/O forbidden"
                    .into(),
            })?;
        Ok(ResolvedRead {
            uri: source.uri.clone(),
            bytes: source.bytes.clone(),
            content_type: Some(source.content_type.clone()),
        })
    }
}

/// Only snapshot preflight can construct this owner. Public inspection DTOs are
/// read-only views and cannot replace the retained acquisition source.
pub struct SnapshotTemplatePreflight {
    snapshot: Arc<TemplateSourceSnapshot>,
    context: Arc<EngineContext>,
    root: Arc<TemplateInput>,
    inspection: Arc<NativeTemplateDependencyInspection>,
}
impl SnapshotTemplatePreflight {
    pub fn snapshot(&self) -> &Arc<TemplateSourceSnapshot> {
        &self.snapshot
    }
    pub fn context(&self) -> &Arc<EngineContext> {
        &self.context
    }
    pub fn root(&self) -> &Arc<TemplateInput> {
        &self.root
    }
    pub fn inspection(&self) -> &NativeTemplateDependencyInspection {
        &self.inspection
    }
    pub fn inspection_owner(&self) -> &Arc<NativeTemplateDependencyInspection> {
        &self.inspection
    }
    pub fn validate_current(
        &self,
        snapshot: &Arc<TemplateSourceSnapshot>,
        context: &Arc<EngineContext>,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(snapshot, &self.snapshot) || !Arc::ptr_eq(context, &self.context) {
            return Err("template snapshot or engine owner changed".into());
        }
        context
            .operation_control
            .check_scope(crate::operation_control::ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.to_string())
    }
}
/// The host supplies its current immutable project snapshot, including the root.
/// Imported bytes use ordinary native import policy and depth checks, with snapshot
/// lookup replacing every acquisition path (including file and registered schemes).
pub fn preflight_template_snapshot(
    snapshot: Arc<TemplateSourceSnapshot>,
    context: Arc<EngineContext>,
    root_uri: &str,
    max_import_depth: u32,
) -> Result<SnapshotTemplatePreflight, String> {
    context
        .operation_control
        .check_scope(crate::operation_control::ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.to_string())?;
    let source = snapshot
        .source(root_uri)
        .ok_or("root template absent from snapshot")?;
    let root = Arc::new(TemplateInput {
        uri: source.uri.clone(),
        bytes: source.bytes.clone(),
        identity: Some(FormatIdentity {
            content_type: Some(source.content_type.clone()),
            ..Default::default()
        }),
        root_scope: ScopeConfig::default(),
    });
    let inspection = crate::real::inspect_native_template_dependencies_with_snapshot(
        &context,
        &root,
        max_import_depth,
        Some(&snapshot),
    );
    if inspection.preflight.is_none()
        || inspection
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
    {
        return Err(format!(
            "snapshot template preflight failed: {}",
            inspection
                .diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let result = SnapshotTemplatePreflight {
        snapshot,
        context,
        root,
        inspection: Arc::new(inspection),
    };
    result.validate_current(&result.snapshot, &result.context)?;
    Ok(result)
}
