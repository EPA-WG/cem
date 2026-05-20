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

        // ts_dts.rs, rust_hdr.rs, uri_publish.rs land in follow-up PRs;
        // their options gates are wired here already so future drops
        // only need to add the trait-call block above.
        let _ = options.emit_dts;
        let _ = options.emit_rust;

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
    fn emit_all_produces_two_artifacts_with_defaults() {
        let schema = CompiledSchema::cem_core();
        let output = SchemaCompiler::emit_all(&schema, &CompilerOptions::default()).unwrap();
        assert_eq!(output.artifacts.len(), 2);
        assert!(output
            .artifacts
            .iter()
            .any(|a| a.kind == ArtifactKind::RelaxNgXml));
        assert!(output
            .artifacts
            .iter()
            .any(|a| a.kind == ArtifactKind::RelaxNgCompact));
        assert_eq!(output.manifest.hash_scheme, "cem-bin/1+blake3");
        assert_eq!(output.manifest.artifacts.len(), 2);
    }

    #[test]
    fn disabling_an_emitter_drops_its_artifact() {
        let schema = CompiledSchema::cem_core();
        let opts = CompilerOptions {
            emit_rng_compact: false,
            ..Default::default()
        };
        let output = SchemaCompiler::emit_all(&schema, &opts).unwrap();
        assert_eq!(output.artifacts.len(), 1);
        assert_eq!(output.artifacts[0].kind, ArtifactKind::RelaxNgXml);
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
}
