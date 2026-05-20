//! Compiler output shapes — `CompilerOutput`, `EmittedArtifact`,
//! `ArtifactKind`, `PublicationManifest`, `ContentHash`, `EmitterTag`.
//!
//! Reference: `cem-ml-stack-design-impl.md` §3.4.2.2.

use std::collections::BTreeMap;

use crate::diagnostics::Diagnostic;
use crate::schema::ir::SemVer;
use crate::schema::SchemaId;
use crate::source_map::SourceMapStack;

/// `cem-bin/1+blake3` content hash. Sidecar bytes are the canonical
/// representation: `{scheme}:{hex}\n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHash {
    pub scheme: &'static str,
    pub hex: String,
}

impl ContentHash {
    pub const SCHEME: &'static str = "cem-bin/1+blake3";

    /// Compute the blake3 hash over `bytes` and wrap it in the CEM
    /// content-hash envelope.
    pub fn from_blake3(bytes: &[u8]) -> Self {
        Self {
            scheme: Self::SCHEME,
            hex: blake3::hash(bytes).to_hex().to_string(),
        }
    }

    /// Canonical sidecar string: `{scheme}:{hex}\n` (LF, no trailing
    /// whitespace, single final newline per §13.2.4 rule 1).
    pub fn to_sidecar_string(&self) -> String {
        format!("{}:{}\n", self.scheme, self.hex)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactKind {
    RelaxNgXml,
    RelaxNgCompact,
    TypeScriptDts,
    RustHeader,
    Manifest,
}

impl ArtifactKind {
    pub fn extension(self) -> &'static str {
        match self {
            ArtifactKind::RelaxNgXml => "rng",
            ArtifactKind::RelaxNgCompact => "rnc",
            ArtifactKind::TypeScriptDts => "d.ts",
            ArtifactKind::RustHeader => "rs",
            ArtifactKind::Manifest => "json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmittedArtifact {
    pub kind: ArtifactKind,
    /// Path under `dist/lib/schema/`, e.g. `core/1.0.0/cem-core.rng`.
    pub relative_path: String,
    /// UTF-8, LF-only, single final newline; see §13.2.4.
    pub bytes: Vec<u8>,
    pub content_hash: ContentHash,
    /// Frame chain back to the CEM-native source. Empty for now;
    /// populated when the markdown-driven compiler lands.
    pub source_map: SourceMapStack,
}

/// Crate version + emitter name. Not part of the hash input — kept on
/// the descriptor so reproductions can confirm which emitter wrote the
/// artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitterTag {
    pub crate_version: &'static str,
    pub emitter_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorTag {
    pub name: &'static str,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArtifactDescriptor {
    pub relative_path: String,
    pub content_hash: ContentHash,
    pub byte_length: u64,
    pub emitted_by: EmitterTag,
    /// Recorded only after the verification fixture passes (Tier A
    /// emitters leave this `None`; URI-publish work populates it once
    /// the xmllint / Trang fixtures are wired into release tooling).
    pub validated_by: Option<ValidatorTag>,
}

#[derive(Debug, Clone)]
pub struct PublicationManifest {
    pub schema_uri: String,
    pub embedded_version: SemVer,
    /// Always `cem-bin/1+blake3`. Static string keeps the hash scheme
    /// out of the per-artifact hash input.
    pub hash_scheme: &'static str,
    /// Stable key order via `BTreeMap` traversal (§13.2.4 rule 4).
    pub artifacts: BTreeMap<ArtifactKind, ArtifactDescriptor>,
}

#[derive(Debug, Clone)]
pub struct CompilerOutput {
    pub schema_id: SchemaId,
    pub schema_uri: String,
    pub embedded_version: SemVer,
    pub artifacts: Vec<EmittedArtifact>,
    pub manifest: PublicationManifest,
    /// Compile-time diagnostics raised during emission. Empty when the
    /// emit path was clean.
    pub diagnostics: Vec<Diagnostic>,
}
