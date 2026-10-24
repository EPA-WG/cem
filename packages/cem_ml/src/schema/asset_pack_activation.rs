//! Immutable, consumer-owned activation of admitted native package sources.

use super::{asset_pack::SchemaAssetPack, registry::SchemaPackageOrigin};
use crate::{
    diagnostics::{Diagnostic, Severity},
    engine::{EngineContext, EngineError, EngineInput, EngineResult},
    resolver::{ResolvedRead, ResolverDiagnostic},
};
use std::{collections::BTreeMap, sync::Arc};

/// Published only after the existing manifest authority and model-readiness
/// checks succeed. Context clones share this immutable source allocation.
/// No numeric/global handle, checkpoint authority or runtime retirement grant.
pub struct SchemaAssetActivation {
    pack: Arc<SchemaAssetPack>,
    packages: BTreeMap<String, String>,
}

impl std::fmt::Debug for SchemaAssetActivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaAssetActivation")
            .field("index_digest", &self.pack.index_digest())
            .field("packages", &self.packages)
            .field("source_count", &self.pack.admitted_sources())
            .field("source_bytes", &self.pack.admitted_bytes())
            .finish()
    }
}

impl SchemaAssetActivation {
    pub fn index_digest(&self) -> &str {
        self.pack.index_digest()
    }
    pub fn owned_source_count(&self) -> usize {
        self.pack.admitted_sources()
    }
    pub fn owned_source_bytes(&self) -> usize {
        self.pack.admitted_bytes()
    }
    pub fn owns_package(&self, package_id: &str) -> bool {
        self.packages.contains_key(package_id)
    }

    pub(crate) fn owns_path(&self, path: &str) -> bool {
        self.pack.index().packages.iter().any(|package| {
            self.owns_package(&package.package_id)
                && package
                    .directory()
                    .is_ok_and(|directory| path.starts_with(&directory))
        })
    }

    pub(crate) fn check_manifest(
        &self,
        package_id: &str,
        uri: &str,
        source: &str,
    ) -> Result<(), String> {
        let descriptor = self
            .pack
            .index()
            .sources
            .iter()
            .find(|s| {
                s.package_id == package_id
                    && s.kind == super::asset_pack::SchemaAssetSourceKind::Manifest
            })
            .ok_or("Activated manifest is absent from its pinned index")?;
        if descriptor.path != uri
            || self
                .read_source(uri)
                .map_err(|error| error.to_string())?
                .bytes
                != source.as_bytes()
        {
            return Err("A pinned package cannot be changed through a mutable manifest; use a fresh consumer context".into());
        }
        Ok(())
    }

    pub(crate) fn read_source(&self, path: &str) -> Result<ResolvedRead, ResolverDiagnostic> {
        let read = || {
            let descriptor = self
                .pack
                .index()
                .sources
                .iter()
                .find(|s| s.path == path)
                .ok_or_else(|| "Source is absent from the pinned package index".to_owned())?;
            if self.packages.get(&descriptor.package_id) != Some(&descriptor.version) {
                return Err("Source package is not active in this consumer".to_owned());
            }
            let bytes = self
                .pack
                .source_bytes(&descriptor.package_id, &descriptor.version, path)
                .map_err(|error| error.to_string())?;
            Ok(ResolvedRead {
                uri: path.into(),
                bytes: bytes.to_vec(),
                content_type: Some(descriptor.content_type.clone()),
            })
        };
        read().map_err(|message| ResolverDiagnostic::Io {
            uri: path.into(),
            message: format!("{message}; no embedded fallback for an activated package"),
        })
    }

    pub(crate) fn read_artifact(
        &self,
        artifact: &crate::conversion::ConversionPackageArtifactDescriptor,
    ) -> Result<crate::conversion::ConversionPackageArtifactRead, String> {
        let version = self
            .packages
            .get(&artifact.package_id)
            .ok_or("Package is not active")?;
        let descriptor = self
            .pack
            .source_descriptor(&artifact.package_id, version, &artifact.path)
            .map_err(|error| format!("{error}; no embedded fallback for an activated package"))?;
        if artifact.content_type.as_deref().is_some_and(|mime| {
            super::registry::content_type_essence(mime)
                != super::registry::content_type_essence(&descriptor.content_type)
        }) {
            return Err(
                "Artifact MIME differs from the pinned source; no embedded fallback".into(),
            );
        }
        let read = self
            .read_source(&artifact.path)
            .map_err(|error| error.to_string())?;
        Ok(crate::conversion::ConversionPackageArtifactRead {
            uri: read.uri,
            bytes: read.bytes,
            content_type: read.content_type,
        })
    }

    /// Imports from an owned source remain in the canonical package namespace.
    /// Transport locations never become source identities or escape routes.
    pub(crate) fn read_import(
        &self,
        base: &str,
        import: &str,
    ) -> Option<Result<ResolvedRead, ResolverDiagnostic>> {
        if !self.owns_path(base) && !self.owns_path(import) {
            return None;
        }
        let joined = if import.starts_with("schema-packages/") {
            import.to_owned()
        } else {
            format!(
                "{}{import}",
                base.rsplit_once('/')
                    .map(|(p, _)| format!("{p}/"))
                    .unwrap_or_default()
            )
        };
        let mut parts = Vec::new();
        for part in joined.split('/') {
            match part {
                "." | "" => (),
                ".." => {
                    parts.pop();
                }
                _ => parts.push(part),
            }
        }
        Some(self.read_source(&parts.join("/")))
    }
}

/// Stage an explicit set of exact package versions against an immutable pack.
/// Existing built-ins require the same scoped replacement grants as external
/// manifests. Missing sources, dependencies, pending models and unauthorized
/// replacements leave the entire incumbent context and its preparations alive.
pub fn activate_schema_asset_packages(
    context: &mut EngineContext,
    pack: Arc<SchemaAssetPack>,
    identities: &[(&str, &str)],
) -> EngineResult<Vec<Diagnostic>> {
    context.ensure_active()?;
    let failure = |message: String| EngineError::SchemaResolution(message);
    if identities.is_empty() || identities.len() > pack.index().packages.len() {
        return Err(failure(
            "Activation requires a bounded, nonempty package selection".into(),
        ));
    }
    let mut packages = context
        .schema_asset_activation
        .as_ref()
        .map(|binding| binding.packages.clone())
        .unwrap_or_default();
    if let Some(binding) = &context.schema_asset_activation {
        if binding.index_digest() != pack.index_digest() {
            return Err(failure(
                "A consumer cannot mix independently pinned asset indexes".into(),
            ));
        }
        // Republishing the same index can add sources, never silently lose a
        // source on which an already active package depends.
        for source in &binding.pack.index().sources {
            if binding.owns_package(&source.package_id)
                && binding
                    .pack
                    .source_bytes(&source.package_id, &source.version, &source.path)
                    .is_ok()
            {
                pack.source_bytes(&source.package_id, &source.version, &source.path)
                    .map_err(|error| failure(error.to_string()))?;
            }
        }
    }
    let mut selected = BTreeMap::new();
    for &(id, version) in identities {
        if selected.insert(id, version).is_some() {
            return Err(failure(format!("Duplicate activation of package `{id}`")));
        }
        // A selected manifest cannot direct template loading outside its pinned
        // package. Bodies may be acquired later, but every identity must exist.
        let owned = pack
            .owned_package(id, version)
            .map_err(|error| failure(error.to_string()))?;
        let check_source = |path: &str, mime: Option<&str>| -> EngineResult<()> {
            let source = pack
                .source_descriptor(id, version, path)
                .map_err(|error| failure(error.to_string()))?;
            if mime.is_some_and(|mime| {
                super::registry::content_type_essence(mime)
                    != super::registry::content_type_essence(&source.content_type)
            }) {
                return Err(failure(format!(
                    "Package `{id}` declares mismatched MIME for `{path}`"
                )));
            }
            Ok(())
        };
        for artifact in crate::conversion::conversion_package_artifacts_from_schema_package(&owned)
            .map_err(|error| failure(error.to_string()))?
        {
            check_source(&artifact.path, artifact.content_type.as_deref())?;
        }
        for converter in crate::conversion::conversion_descriptors_from_schema_package(&owned)
            .map_err(|error| failure(error.to_string()))?
        {
            if let Some(template) = converter.template {
                check_source(&template.path, Some(&template.content_type))?;
            }
        }
        packages.insert(id.into(), version.into());
    }
    for &(id, version) in identities {
        let package = pack
            .index()
            .packages
            .iter()
            .find(|p| p.package_id == id && p.version == version)
            .expect("validated package");
        for dependency in &package.uses {
            if context.schema_registry.schema(dependency).is_none()
                && !pack.index().packages.iter().any(|p| {
                    &p.schema_uri == dependency
                        && selected.get(p.package_id.as_str()) == Some(&p.version.as_str())
                })
            {
                return Err(failure(format!(
                    "Package `{id}` requires active schema `{dependency}`"
                )));
            }
        }
    }
    let binding = Arc::new(SchemaAssetActivation { pack, packages });
    let mut staged = context.clone();
    staged.schema_asset_activation = Some(binding.clone());
    let mut diagnostics = Vec::new();
    for &(id, version) in identities {
        context.ensure_active()?;
        let package = binding
            .pack
            .index()
            .packages
            .iter()
            .find(|p| p.package_id == id && p.version == version)
            .expect("validated package");
        let manifest = binding
            .pack
            .index()
            .sources
            .iter()
            .find(|s| {
                s.package_id == id && s.kind == super::asset_pack::SchemaAssetSourceKind::Manifest
            })
            .expect("validated manifest");
        let read = binding
            .read_source(&manifest.path)
            .map_err(|error| failure(error.to_string()))?;
        diagnostics.extend(crate::real::load_schema_package_manifest_staged(
            &mut staged,
            &EngineInput {
                uri: read.uri.clone(),
                bytes: read.bytes,
                from_format: None,
                identity: None,
                root_scope: Default::default(),
            },
            false,
        )?);
        if diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
            return Ok(diagnostics);
        }
        if staged
            .schema_document_models
            .get(&package.schema_uri)
            .is_none_or(|model| !model.is_ready_for_validation())
            || staged.schema_registry.package_origin(id)
                != Some(&SchemaPackageOrigin::Manifest(read.uri))
            || staged
                .schema_document_models
                .resolve_for_identity(Some(&package.schema_uri), None, None)
                .is_none()
        {
            diagnostics.push(Diagnostic {
                code: "cem.schema_package.activation_not_ready".into(),
                severity: Severity::Error,
                uri: Some(manifest.path.clone()),
                message: format!(
                    "Package `{id}` did not produce an active, ready native schema model"
                ),
                ..Default::default()
            });
            return Ok(diagnostics);
        }
    }
    context.ensure_active()?;
    context
        .schema_document_models
        .retire_preparations_replaced_by(&staged.schema_document_models);
    *context = staged;
    Ok(diagnostics)
}

#[cfg(test)]
mod tests;
