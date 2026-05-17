//! Layer 4 — `SchemaMachine`.
//!
//! Public contract per AC-F-10 / `cem-ml-stack-design-impl.md` §3.4.
//! Tier A vocab + machine live in submodules.

pub mod machine;
pub mod vocab;

use crate::source::ByteRange;
use crate::source_map::SourceMapStack;

pub type ScopeId = u32;
pub type SchemaId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramePhase {
    Attribute,
    Header,
    Content,
    Closed,
}

#[derive(Debug, Clone)]
pub struct SchemaVersionIdentity {
    pub schema_id: SchemaId,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// One schema frame on the validation stack. Diagnostics emitted inside the
/// frame bubble to the nearest schema-declared boundary per AC-P-4.
#[derive(Debug, Clone)]
pub struct SchemaFrame {
    pub scope_id: ScopeId,
    pub schema_id: SchemaId,
    pub schema_version: SchemaVersionIdentity,
    pub language_id: String,
    pub phase: FramePhase,
    pub source_span: ByteRange,
    pub source_map_stack: SourceMapStack,
    pub expected_close: Option<String>,
}

pub trait SchemaMachine: Send {
    fn current(&self) -> Option<&SchemaFrame>;
    fn frames(&self) -> &[SchemaFrame];
}
