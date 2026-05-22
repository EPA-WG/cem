//! Schema compiler output module — projects a `CompiledSchema` into
//! byte-stable release artifacts (RELAX NG XML/compact mirrors today;
//! TypeScript `.d.ts`, Rust `.rs`, URI publication manifest later).
//!
//! Reference: `cem-ml-stack-design.md` §13.2 and
//! `cem-ml-stack-design-impl.md` §3.4.2. All design open questions
//! were resolved 2026-05-19; see §13.2.9 of the design doc.

pub mod byte_stability;
pub mod emitter;
pub mod error;
pub mod output;
pub mod rng_compact;
pub mod rng_xml;
pub mod rust_hdr;
pub mod ts_dts;
pub mod uri_publish;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub use emitter::{EmissionCursor, SchemaEmitter};
pub use error::EmitError;
pub use output::{
    ArtifactDescriptor, ArtifactKind, CompilerOutput, ContentHash, EmittedArtifact, EmitterTag,
    PublicationManifest, ValidatorTag,
};
pub use uri_publish::{
    match_rule_label, parse_schema_uri, resolve_uri, SchemaUriRef, UriResolution,
};

use crate::schema::ir::CompiledSchema;

const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct CompilerOptions {
    /// Tier A code, Tier B gate per OQ-SC-3 (resolved). Default
    /// `false`; the rust_hdr emitter is dormant in Tier A.
    pub emit_rust: bool,
    pub emit_dts: bool,
    pub emit_rng_xml: bool,
    pub emit_rng_compact: bool,
    /// AC-S-6 `Validated<T>` opt-in. Default `true`.
    pub include_validated_brand: bool,
    /// AUTO-GENERATED preamble per OQ-SC-8 (resolved). Default `true`;
    /// the header NEVER carries the content hash — it lives in the
    /// `.hash` sidecar only.
    pub embed_source_header: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            emit_rust: false,
            emit_dts: true,
            emit_rng_xml: true,
            emit_rng_compact: true,
            include_validated_brand: true,
            embed_source_header: true,
        }
    }
}

pub struct SchemaCompiler;

impl SchemaCompiler {
    /// Emit every enabled artifact and assemble the manifest.
    pub fn emit_all(
        schema: &CompiledSchema,
        options: &CompilerOptions,
    ) -> Result<CompilerOutput, EmitError> {
        emitter::reject_non_streamable_constraints(schema)?;

        let mut artifacts: Vec<EmittedArtifact> = Vec::new();
        let mut manifest_artifacts: BTreeMap<ArtifactKind, ArtifactDescriptor> = BTreeMap::new();

        if options.emit_rng_xml {
            let mut cursor = EmissionCursor::new(schema);
            let artifact = rng_xml::RngXmlEmitter.emit(schema, options, &mut cursor)?;
            register_manifest(
                &mut manifest_artifacts,
                &artifact,
                rng_xml::RngXmlEmitter::EMITTER_NAME,
            );
            artifacts.push(artifact);
        }

        if options.emit_rng_compact {
            let mut cursor = EmissionCursor::new(schema);
            let artifact = rng_compact::RngCompactEmitter.emit(schema, options, &mut cursor)?;
            register_manifest(
                &mut manifest_artifacts,
                &artifact,
                rng_compact::RngCompactEmitter::EMITTER_NAME,
            );
            artifacts.push(artifact);
        }

        if options.emit_dts {
            let mut cursor = EmissionCursor::new(schema);
            let artifact = ts_dts::TsDtsEmitter.emit(schema, options, &mut cursor)?;
            register_manifest(
                &mut manifest_artifacts,
                &artifact,
                ts_dts::TsDtsEmitter::EMITTER_NAME,
            );
            artifacts.push(artifact);
        }

        // Tier B gate — OQ-SC-3 (resolved). Default `false`; consumers
        // (or the `rust_hdr_compiles.rs` fixture) flip this on when
        // they want the .rs artifact + `cargo check` verification.
        if options.emit_rust {
            let mut cursor = EmissionCursor::new(schema);
            let artifact = rust_hdr::RustHdrEmitter.emit(schema, options, &mut cursor)?;
            register_manifest(
                &mut manifest_artifacts,
                &artifact,
                rust_hdr::RustHdrEmitter::EMITTER_NAME,
            );
            artifacts.push(artifact);
        }

        let manifest = PublicationManifest {
            schema_uri: schema.version_identity.uri.clone(),
            embedded_version: schema.version_identity.embedded_version.clone(),
            hash_scheme: ContentHash::SCHEME,
            artifacts: manifest_artifacts,
        };

        // The manifest records every content artifact's hash, so it is
        // built last and pushed as the final artifact; `write_to_disk`
        // relies on that ordering (AC-S-5; §13.2.6 step 2).
        let manifest_artifact = uri_publish::emit_manifest_artifact(schema, &manifest)?;
        artifacts.push(manifest_artifact);

        Ok(CompilerOutput {
            schema_id: schema.schema_id,
            schema_uri: schema.version_identity.uri.clone(),
            embedded_version: schema.version_identity.embedded_version.clone(),
            artifacts,
            manifest,
            diagnostics: Vec::new(),
        })
    }

    /// Write an emitted output tree to disk under `root_dir`
    /// (`packages/cem_ml/dist/lib/schema/`). Every content artifact and
    /// its `.hash` sidecar is written first; the manifest and its
    /// sidecar are written last, so a crash mid-publish leaves the
    /// previous manifest pointing at intact files (AC-S-5; §13.2.6
    /// step 2). Each file goes through a temp-then-rename so a partial
    /// write never leaves a truncated artifact (§3.4.2.5).
    ///
    /// `root_dir` is a build-output tree: `write_to_disk` overwrites
    /// freely. Treating a *published* version as immutable is release
    /// tooling's job (§13.2.6 step 5), not this writer's.
    pub fn write_to_disk(output: &CompilerOutput, root_dir: &Path) -> Result<(), EmitError> {
        for artifact in &output.artifacts {
            if artifact.kind != ArtifactKind::Manifest {
                write_artifact(root_dir, artifact)?;
            }
        }
        for artifact in &output.artifacts {
            if artifact.kind == ArtifactKind::Manifest {
                write_artifact(root_dir, artifact)?;
            }
        }
        Ok(())
    }
}

/// Write one artifact plus its `.hash` sidecar under `root_dir`.
fn write_artifact(root_dir: &Path, artifact: &EmittedArtifact) -> Result<(), EmitError> {
    let target = artifact_target_path(root_dir, &artifact.relative_path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    write_atomic(&target, &artifact.bytes)?;
    let sidecar = append_extension(&target, "hash");
    write_atomic(&sidecar, artifact.content_hash.to_sidecar_string().as_bytes())?;
    Ok(())
}

fn artifact_target_path(root_dir: &Path, relative_path: &str) -> Result<PathBuf, EmitError> {
    let relative = Path::new(relative_path);
    let mut normalized = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(segment) => normalized.push(segment),
            _ => {
                return Err(EmitError::InvalidArtifactPath {
                    path: relative_path.to_owned(),
                    reason: "artifact path must be a relative normal path",
                })
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(EmitError::InvalidArtifactPath {
            path: relative_path.to_owned(),
            reason: "artifact path cannot be empty",
        });
    }
    Ok(root_dir.join(normalized))
}

/// Write `bytes` to `target` through a sibling temp file + rename, so a
/// partial write never leaves a truncated file in place (§3.4.2.5).
fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), EmitError> {
    let tmp = append_extension(target, "tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, target)?;
    Ok(())
}

/// Append `.{suffix}` to a path without disturbing its existing
/// extension — `cem-core.rng` + `hash` → `cem-core.rng.hash`.
fn append_extension(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(suffix);
    PathBuf::from(name)
}

fn register_manifest(
    manifest: &mut BTreeMap<ArtifactKind, ArtifactDescriptor>,
    artifact: &EmittedArtifact,
    emitter_name: &'static str,
) {
    manifest.insert(
        artifact.kind,
        ArtifactDescriptor {
            relative_path: artifact.relative_path.clone(),
            content_hash: artifact.content_hash.clone(),
            byte_length: artifact.bytes.len() as u64,
            emitted_by: EmitterTag {
                crate_version: CRATE_VERSION,
                emitter_name,
            },
            validated_by: None,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_match_design_defaults() {
        let opts = CompilerOptions::default();
        assert!(!opts.emit_rust, "rust emitter default-off per OQ-SC-3");
        assert!(opts.emit_dts);
        assert!(opts.emit_rng_xml);
        assert!(opts.emit_rng_compact);
        assert!(opts.include_validated_brand);
        assert!(opts.embed_source_header);
    }

    #[test]
    fn emit_all_produces_three_content_artifacts_plus_manifest() {
        let schema = CompiledSchema::cem_core();
        let output = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        assert_eq!(output.artifacts.len(), 4);
        for kind in [
            ArtifactKind::RelaxNgXml,
            ArtifactKind::RelaxNgCompact,
            ArtifactKind::TypeScriptDts,
            ArtifactKind::Manifest,
        ] {
            assert!(
                output.artifacts.iter().any(|a| a.kind == kind),
                "default emit_all missing artifact: {kind:?}"
            );
        }
        // The manifest is the final artifact (written last on disk).
        assert_eq!(
            output.artifacts.last().map(|a| a.kind),
            Some(ArtifactKind::Manifest)
        );
        assert_eq!(output.manifest.hash_scheme, "cem-bin/1+blake3");
        // The manifest describes the three content artifacts and never
        // itself.
        assert_eq!(output.manifest.artifacts.len(), 3);
        assert!(!output
            .manifest
            .artifacts
            .contains_key(&ArtifactKind::Manifest));
    }

    #[test]
    fn disabling_an_emitter_drops_its_artifact() {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            emit_rng_compact: false,
            emit_dts: false,
            ..Default::default()
        };
        let output = SchemaCompiler::emit_all(&schema, &opts).unwrap();
        // One content artifact (rng_xml) plus the always-present manifest.
        assert_eq!(output.artifacts.len(), 2);
        assert_eq!(output.artifacts[0].kind, ArtifactKind::RelaxNgXml);
        assert_eq!(output.artifacts[1].kind, ArtifactKind::Manifest);
        assert_eq!(output.manifest.artifacts.len(), 1);
    }

    #[test]
    fn emit_dts_default_on_and_can_be_disabled_independently() {
        let schema = CompiledSchema::cem_core();
        // Default-on.
        let output = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        assert!(output
            .manifest
            .artifacts
            .contains_key(&ArtifactKind::TypeScriptDts));

        // Explicit off — the two RNG mirrors and the manifest remain.
        let opts = CompilerOptions {
            emit_dts: false,
            ..Default::default()
        };
        let output = SchemaCompiler::emit_all(&schema, &opts).unwrap();
        assert_eq!(output.artifacts.len(), 3);
        assert!(!output
            .artifacts
            .iter()
            .any(|a| a.kind == ArtifactKind::TypeScriptDts));
    }

    #[test]
    fn manifest_carries_byte_length_and_emitter_tag() {
        let schema = CompiledSchema::cem_core();
        let output = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        let rng_desc = &output.manifest.artifacts[&ArtifactKind::RelaxNgXml];
        assert!(rng_desc.byte_length > 0);
        assert_eq!(rng_desc.emitted_by.crate_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(rng_desc.emitted_by.emitter_name, "rng_xml");
        assert!(rng_desc.validated_by.is_none());
    }

    #[test]
    fn emit_rust_default_off_per_oq_sc_3() {
        // Tier A code, Tier B gate — OQ-SC-3 (resolved).
        let schema = CompiledSchema::cem_core();
        let output = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        assert!(
            !output
                .artifacts
                .iter()
                .any(|a| a.kind == ArtifactKind::RustHeader),
            "rust_hdr must be off by default; the Tier B gate flips it on"
        );
        assert!(!output
            .manifest
            .artifacts
            .contains_key(&ArtifactKind::RustHeader));
    }

    #[test]
    fn emit_rust_true_produces_four_content_artifacts_plus_manifest() {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            emit_rust: true,
            ..Default::default()
        };
        let output = SchemaCompiler::emit_all(&schema, &opts).unwrap();
        assert_eq!(output.artifacts.len(), 5);
        for kind in [
            ArtifactKind::RelaxNgXml,
            ArtifactKind::RelaxNgCompact,
            ArtifactKind::TypeScriptDts,
            ArtifactKind::RustHeader,
            ArtifactKind::Manifest,
        ] {
            assert!(
                output.artifacts.iter().any(|a| a.kind == kind),
                "emit_all with emit_rust=true missing artifact: {kind:?}"
            );
        }
        assert_eq!(
            output.artifacts.last().map(|a| a.kind),
            Some(ArtifactKind::Manifest)
        );
        let rust_desc = &output.manifest.artifacts[&ArtifactKind::RustHeader];
        assert_eq!(rust_desc.emitted_by.emitter_name, "rust_hdr");
        assert_eq!(rust_desc.relative_path, "core/1.0.0/cem-core.rs");
        assert_eq!(output.manifest.artifacts.len(), 4);
    }

    #[test]
    fn emit_all_rejects_non_streamable_constraints() {
        let mut schema = CompiledSchema::cem_core();
        schema
            .non_streamable_constraints
            .push(crate::schema::ir::NonStreamableConstraint {
                annotation: "screen",
                kind: crate::schema::ir::NonStreamableKind::FullDocumentBuffering,
                reason: "requires a whole-document pass",
            });

        let err = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap_err();
        assert!(
            matches!(err, EmitError::UnsupportedConstraint { .. }),
            "expected unsupported constraint error, got {err:?}"
        );
    }
}
