//! Scoped custom-element / template registries (AC-R-1, AC-R-2, AC-R-3).
//!
//! Tier B feature surface. CEM does not police the browser
//! `customElements` registry; instead, DCE tag names and template
//! references are first-class entries in a CEM-owned registry. A
//! scope's registry is filled by template installs (locally) and by
//! ancestor inheritance; lookup falls back through the scope chain
//! until a match is found or the root is reached.
//!
//! Layout:
//! - [`template_ref`] — the `TemplateRef` enum and supporting types.
//! - [`template_registry`] — `TemplateRegistry` (per-scope name → entry table).
//! - [`tree`] — `ScopedRegistryTree` with inherited lookup and
//!   collision diagnostics.

pub mod template_ref;
pub mod template_registry;
pub mod tree;

pub use template_ref::{RegistryId, SchemaId, SourceId, TemplateRef};
pub use template_registry::{CollisionDiagnostic, RegistryEntry, TemplateRegistry};
pub use tree::{LookupResult, RegistryScopeId, ScopedRegistryTree};
