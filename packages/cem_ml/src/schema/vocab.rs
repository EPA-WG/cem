//! Compiled CEM Core schema vocabulary — re-exports the §3.4 IR types
//! from `schema/ir.rs` for back-compat with existing callers
//! (`crate::schema::vocab::CompiledSchema` resolves through here).
//!
//! Source of truth for the cem-core/1 schema content:
//! [`../../schema/cem-core.md`](../../schema/cem-core.md). The Rust
//! factory lives in [`super::ir::CompiledSchema::cem_core`].

pub use super::ir::{
    AnnotationDef, CompiledSchema, NonStreamableConstraint, NonStreamableKind,
    CEM_CORE_NAMESPACE, CEM_CORE_SCHEMA_ID,
};
