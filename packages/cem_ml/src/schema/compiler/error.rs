//! Emitter error type for the schema compiler output module.
//!
//! Reference: `docs/cem-ml-stack-design-impl.md` §3.4.2.1.

use std::fmt;

#[derive(Debug)]
pub enum EmitError {
    IoError(std::io::Error),
    /// Tier A rejects schemas that declare a structural constraint the
    /// DFA profile does not support (AC-S-7). Emitter sees this when the
    /// IR carries a constraint kind absent from
    /// `TierAValidationProfile::supported_constraints`.
    UnsupportedConstraint {
        kind: String,
        schema_uri: String,
    },
    /// Required IR field is missing — caller fed a `CompiledSchema` that
    /// is not fully populated (e.g. version_identity.uri empty).
    MissingIrField {
        field: &'static str,
    },
    /// `DeterministicWriter` reject path: CR byte, trailing whitespace,
    /// or missing final newline detected at release time.
    NonDeterministicWrite {
        reason: &'static str,
    },
    /// A schema URI could not be resolved against the published
    /// manifests — either it is outside the well-known
    /// `https://cem.dev/ns/` scheme, or no published embedded version
    /// satisfies its version-tail constraint (AC-S-5 / AC-V-10).
    UnresolvableUri {
        uri: String,
        reason: &'static str,
    },
    /// An artifact relative path would escape the publication root or
    /// otherwise violate the stable publication-tree path grammar.
    InvalidArtifactPath {
        path: String,
        reason: &'static str,
    },
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::IoError(e) => write!(f, "schema emitter I/O error: {e}"),
            EmitError::UnsupportedConstraint { kind, schema_uri } => write!(
                f,
                "schema emitter: unsupported Tier A constraint `{kind}` in schema `{schema_uri}`"
            ),
            EmitError::MissingIrField { field } => {
                write!(f, "schema emitter: required IR field `{field}` is missing")
            }
            EmitError::NonDeterministicWrite { reason } => {
                write!(
                    f,
                    "schema emitter: non-deterministic write rejected ({reason})"
                )
            }
            EmitError::UnresolvableUri { uri, reason } => write!(
                f,
                "schema emitter: cannot resolve schema URI `{uri}` ({reason})"
            ),
            EmitError::InvalidArtifactPath { path, reason } => write!(
                f,
                "schema emitter: invalid artifact path `{path}` ({reason})"
            ),
        }
    }
}

impl std::error::Error for EmitError {}

impl From<std::io::Error> for EmitError {
    fn from(value: std::io::Error) -> Self {
        EmitError::IoError(value)
    }
}
