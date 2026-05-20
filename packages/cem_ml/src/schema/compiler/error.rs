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
    UnsupportedConstraint { kind: String, schema_uri: String },
    /// Required IR field is missing — caller fed a `CompiledSchema` that
    /// is not fully populated (e.g. version_identity.uri empty).
    MissingIrField { field: &'static str },
    /// `DeterministicWriter` reject path: CR byte, trailing whitespace,
    /// or missing final newline detected at release time.
    NonDeterministicWrite { reason: &'static str },
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
                write!(f, "schema emitter: non-deterministic write rejected ({reason})")
            }
        }
    }
}

impl std::error::Error for EmitError {}

impl From<std::io::Error> for EmitError {
    fn from(value: std::io::Error) -> Self {
        EmitError::IoError(value)
    }
}
