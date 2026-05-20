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

use std::collections::BTreeMap;

pub use emitter::{EmissionCursor, SchemaEmitter};
pub use error::EmitError;
pub use output::{
    ArtifactDescriptor, ArtifactKind, CompilerOutput, ContentHash, EmittedArtifact, EmitterTag,
    PublicationManifest, ValidatorTag,
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

        // uri_publish.rs lands in the next emitter PR.

        let manifest = PublicationManifest {
            schema_uri: schema.version_identity.uri.clone(),
            embedded_version: schema.version_identity.embedded_version.clone(),
            hash_scheme: ContentHash::SCHEME,
            artifacts: manifest_artifacts,
        };

        Ok(CompilerOutput {
            schema_id: schema.schema_id,
            schema_uri: schema.version_identity.uri.clone(),
            embedded_version: schema.version_identity.embedded_version.clone(),
            artifacts,
            manifest,
            diagnostics: Vec::new(),
        })
    }

    /// Write a previously-emitted output tree to disk. Stub for the
    /// URI publication PR; the rng emitter PR exercises emit_all only
    /// (in-memory). Returns `Ok(())` for a no-op write so callers can
    /// integrate progressively.
    pub fn write_to_disk(
        _output: &CompilerOutput,
        _root_dir: &std::path::Path,
    ) -> Result<(), EmitError> {
        // The `uri_publish.rs` emitter PR fills this in (AC-S-5,
        // manifest + sidecars + TempThenRename adapter per §3.4.2.5).
        Ok(())
    }
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
    fn emit_all_produces_three_artifacts_with_defaults() {
        let schema = CompiledSchema::cem_core();
        let output = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        assert_eq!(output.artifacts.len(), 3);
        for kind in [
            ArtifactKind::RelaxNgXml,
            ArtifactKind::RelaxNgCompact,
            ArtifactKind::TypeScriptDts,
        ] {
            assert!(
                output.artifacts.iter().any(|a| a.kind == kind),
                "default emit_all missing artifact: {kind:?}"
            );
        }
        assert_eq!(output.manifest.hash_scheme, "cem-bin/1+blake3");
        assert_eq!(output.manifest.artifacts.len(), 3);
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
        assert_eq!(output.artifacts.len(), 1);
        assert_eq!(output.artifacts[0].kind, ArtifactKind::RelaxNgXml);
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

        // Explicit off — only RNG outputs remain.
        let opts = CompilerOptions {
            emit_dts: false,
            ..Default::default()
        };
        let output = SchemaCompiler::emit_all(&schema, &opts).unwrap();
        assert_eq!(output.artifacts.len(), 2);
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
    fn emit_rust_true_produces_four_artifacts() {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            emit_rust: true,
            ..Default::default()
        };
        let output = SchemaCompiler::emit_all(&schema, &opts).unwrap();
        assert_eq!(output.artifacts.len(), 4);
        for kind in [
            ArtifactKind::RelaxNgXml,
            ArtifactKind::RelaxNgCompact,
            ArtifactKind::TypeScriptDts,
            ArtifactKind::RustHeader,
        ] {
            assert!(
                output.artifacts.iter().any(|a| a.kind == kind),
                "emit_all with emit_rust=true missing artifact: {kind:?}"
            );
        }
        let rust_desc = &output.manifest.artifacts[&ArtifactKind::RustHeader];
        assert_eq!(rust_desc.emitted_by.emitter_name, "rust_hdr");
        assert_eq!(rust_desc.relative_path, "core/1.0.0/cem-core.rs");
    }
}
