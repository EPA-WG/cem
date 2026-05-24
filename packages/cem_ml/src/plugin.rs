//! Transformation-plugin runtime (AC-PL-1..AC-PL-20).
//!
//! Tier B feature surface. CEM is the sole producer of plugin Rust ASTs
//! ([`project_plugin_sandboxing`][memory]); plugins run in-process with
//! host privileges and isolation is enforced at load time by the CEM
//! Rust AST validator. This module ships the descriptor surface, the
//! per-scope chain and inheritance rules, the observe/mutate gates,
//! source-map stitching across stacked mutate plugins, budgets,
//! lifecycle management, and the descriptor-level capability validator
//! that the future Rust AST walker will feed evidence into.
//!
//! Layout:
//! - [`descriptor`] — plugin descriptor + capability + context types.
//! - [`chain`] — scope-local registry and outer→inner chain merging.
//! - [`runtime`] — invoke pipeline, observer/mutate gates, source-map
//!   stitching, budget/abort/lifecycle plumbing.
//! - [`errors`] — public `PluginError` taxonomy.
//!
//! [memory]: ../../../../../../home/suns/.claude/projects/-home-suns-aWork-cem/memory/project_plugin_sandboxing.md

pub mod chain;
pub mod descriptor;
pub mod errors;
pub mod runtime;

pub use chain::{PluginChain, PluginRegistry};
pub use descriptor::{
    AbortSignal, ContentType, PluginCapability, PluginContext, PluginDescriptor, PluginEvidence,
    PluginInput, PluginInvoke, PluginMode, PluginOutput, ScopeId,
};
pub use errors::PluginError;
pub use runtime::{
    stitched_source_map, stitched_source_map_for_scope, PluginBudget, PluginRunReport,
    PluginRuntime,
};
