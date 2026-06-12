//! Plugin descriptor surface (AC-PL-1, AC-PL-2, AC-PL-18, AC-PL-20).
//!
//! Tier B. The descriptor names the plugin, declares the content
//! types it consumes and emits, fixes its mode, and (for `mutate`
//! plugins) commits to producing a source map. The `requires`
//! capability set is part of the descriptor so the CEM Rust AST
//! validator can match it against the plugin's static evidence at
//! load time.

use crate::source_map::SourceMapStack;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use syn::visit::Visit;

/// Stable identifier of a CEM scope that hosts plugins. Plugins
/// installed on the scope (and inherited from ancestors) execute
/// inside this identity (AC-PL-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(pub u32);

/// Content type accepted (or emitted) by a plugin. Tier B keeps the
/// vocabulary as free-form MIME strings so SCSS, JSX, JSON, project-
/// specific DSLs all sit on the same axis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentType(pub String);

impl ContentType {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ContentType {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for ContentType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Plugin mode per AC-PL-3 / AC-PL-4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginMode {
    /// Non-invasive: plugin observes the input and emits diagnostics
    /// only. Runtime enforces `output == input` (AC-PL-3).
    Observe,
    /// Invasive: plugin transforms the input and emits a source map
    /// (AC-PL-4).
    Mutate,
}

/// Capability the plugin claims to need. The CEM Rust AST validator
/// inspects the plugin source at load time and rejects a plugin whose
/// evidence references capabilities outside this declared set
/// (AC-PL-20).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluginCapability {
    /// Filesystem read access (e.g. `std::fs::read`).
    FilesystemRead,
    /// Filesystem write access.
    FilesystemWrite,
    /// Network I/O (`std::net::*`, `reqwest`, …).
    Network,
    /// Subprocess spawn (`std::process::Command`).
    Process,
    /// `unsafe` blocks or `extern "C"` FFI.
    UnsafeRust,
    /// Use of an `extern crate` that is not in the host's allow-list.
    ExternalCrate(String),
    /// Catch-all for capabilities introduced after Tier B ships.
    Other(String),
}

/// Static evidence produced by the CEM Rust AST validator for a
/// candidate plugin. Each variant cites the surface the AST walker
/// matched (e.g. `std::fs::read`, `unsafe { … }`, an `extern crate`
/// outside the allow-list).
///
/// Until the AST walker lands, plugin registrars can synthesise
/// evidence by hand to test the capability gate. AC-PL-V-7 covers the
/// rejection contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginEvidence {
    pub needs: BTreeSet<PluginCapability>,
}

impl PluginEvidence {
    pub fn empty() -> Self {
        Self {
            needs: BTreeSet::new(),
        }
    }

    pub fn from(needs: impl IntoIterator<Item = PluginCapability>) -> Self {
        Self {
            needs: needs.into_iter().collect(),
        }
    }

    /// Build static capability evidence by parsing Rust source and
    /// walking the syntax tree before the plugin is compiled or
    /// invoked (AC-PL-20). CEM is expected to call this at load time
    /// for plugin Rust emitted by its own AST pipeline.
    pub fn from_rust_source(
        rust_source: &str,
        external_crate_allowlist: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, crate::plugin::errors::PluginError> {
        let syntax = syn::parse_file(rust_source).map_err(|err| {
            crate::plugin::errors::PluginError::CapabilityScan {
                message: err.to_string(),
            }
        })?;
        let allowlist: BTreeSet<String> = external_crate_allowlist
            .into_iter()
            .map(Into::into)
            .collect();
        let mut visitor = RustCapabilityVisitor {
            allowlist: &allowlist,
            needs: BTreeSet::new(),
        };
        visitor.visit_file(&syntax);
        Ok(Self {
            needs: visitor.needs,
        })
    }
}

struct RustCapabilityVisitor<'a> {
    allowlist: &'a BTreeSet<String>,
    needs: BTreeSet<PluginCapability>,
}

impl RustCapabilityVisitor<'_> {
    fn record_path(&mut self, path: &syn::Path) {
        let Some(first) = path.segments.first().map(|s| s.ident.to_string()) else {
            return;
        };
        let second = path.segments.iter().nth(1).map(|s| s.ident.to_string());
        let third = path.segments.iter().nth(2).map(|s| s.ident.to_string());
        match (first.as_str(), second.as_deref(), third.as_deref()) {
            ("std", Some("fs"), Some("write")) => {
                self.needs.insert(PluginCapability::FilesystemWrite);
            }
            ("std", Some("fs"), _) => {
                self.needs.insert(PluginCapability::FilesystemRead);
            }
            ("std", Some("net"), _) => {
                self.needs.insert(PluginCapability::Network);
            }
            ("std", Some("process"), _) => {
                self.needs.insert(PluginCapability::Process);
            }
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for RustCapabilityVisitor<'_> {
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        self.record_path(&node.path);
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_use_tree(&mut self, node: &'ast syn::UseTree) {
        fn walk_use_tree(
            tree: &syn::UseTree,
            prefix: &mut Vec<String>,
            needs: &mut BTreeSet<PluginCapability>,
        ) {
            match tree {
                syn::UseTree::Path(path) => {
                    prefix.push(path.ident.to_string());
                    walk_use_tree(&path.tree, prefix, needs);
                    prefix.pop();
                }
                syn::UseTree::Name(name) => {
                    let mut full = prefix.clone();
                    full.push(name.ident.to_string());
                    record_path_parts(&full, needs);
                }
                syn::UseTree::Rename(rename) => {
                    let mut full = prefix.clone();
                    full.push(rename.ident.to_string());
                    record_path_parts(&full, needs);
                }
                syn::UseTree::Glob(_) => record_path_parts(prefix, needs),
                syn::UseTree::Group(group) => {
                    for item in group.items.iter() {
                        walk_use_tree(item, prefix, needs);
                    }
                }
            }
        }

        fn record_path_parts(parts: &[String], needs: &mut BTreeSet<PluginCapability>) {
            match (
                parts.first().map(String::as_str),
                parts.get(1).map(String::as_str),
                parts.get(2).map(String::as_str),
            ) {
                (Some("std"), Some("fs"), Some("write")) => {
                    needs.insert(PluginCapability::FilesystemWrite);
                }
                (Some("std"), Some("fs"), _) => {
                    needs.insert(PluginCapability::FilesystemRead);
                }
                (Some("std"), Some("net"), _) => {
                    needs.insert(PluginCapability::Network);
                }
                (Some("std"), Some("process"), _) => {
                    needs.insert(PluginCapability::Process);
                }
                _ => {}
            }
        }

        let mut prefix = Vec::new();
        walk_use_tree(node, &mut prefix, &mut self.needs);
        syn::visit::visit_use_tree(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.needs.insert(PluginCapability::UnsafeRust);
        syn::visit::visit_expr_unsafe(self, node);
    }

    fn visit_item_foreign_mod(&mut self, node: &'ast syn::ItemForeignMod) {
        self.needs.insert(PluginCapability::UnsafeRust);
        syn::visit::visit_item_foreign_mod(self, node);
    }

    fn visit_item_extern_crate(&mut self, node: &'ast syn::ItemExternCrate) {
        let crate_name = node.ident.to_string();
        if !self.allowlist.contains(&crate_name) {
            self.needs
                .insert(PluginCapability::ExternalCrate(crate_name));
        }
        syn::visit::visit_item_extern_crate(self, node);
    }
}

/// Cooperative cancellation primitive (AC-PL-19, AC-A-7). Plugins poll
/// `is_aborted()` between work chunks; the runtime fires the abort by
/// calling `abort()`.
#[derive(Debug, Clone, Default)]
pub struct AbortSignal {
    flag: Arc<AtomicBool>,
}

impl AbortSignal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn abort(&self) {
        self.flag.store(true, Ordering::Release);
    }

    pub fn is_aborted(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

/// Per-invocation context handed to `PluginInvoke::invoke`. The
/// runtime owns the channel into the diagnostic/observability bus so a
/// plugin can record validation findings without owning a reference to
/// the engine.
pub struct PluginContext<'a> {
    pub scope: ScopeId,
    pub abort: AbortSignal,
    pub diagnostics: &'a mut Vec<crate::diagnostics::Diagnostic>,
    /// Source-map stack inherited from upstream layers. Mutate plugins
    /// append a [`crate::source_map::SourceMapFrame`] to their output;
    /// observe plugins do not touch it.
    pub inbound_source_map: SourceMapStack,
}

/// Input passed to a plugin. Tier B keeps the surface byte-oriented;
/// downstream stacks (CEM AST, CSS AST, HTML AST) are reconstructed by
/// the host *after* the plugin chain runs over the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInput {
    pub content_type: ContentType,
    pub bytes: Vec<u8>,
}

impl PluginInput {
    pub fn new(content_type: impl Into<ContentType>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            content_type: content_type.into(),
            bytes: bytes.into(),
        }
    }
}

/// Output from a plugin. Mutate plugins must populate `source_map`
/// (AC-PL-4); observe plugins return the input unchanged.
#[derive(Debug, Clone)]
pub struct PluginOutput {
    pub content_type: ContentType,
    pub bytes: Vec<u8>,
    pub source_map: SourceMapStack,
}

impl PluginOutput {
    pub fn observed(input: &PluginInput, inbound_source_map: SourceMapStack) -> Self {
        Self {
            content_type: input.content_type.clone(),
            bytes: input.bytes.clone(),
            source_map: inbound_source_map,
        }
    }
}

/// Callable surface of a plugin. Implemented by host code (built-in
/// transformers) and by user plugins. Plugins are registered through
/// [`PluginDescriptor`] objects, not by side-effecting imports
/// (AC-PL-18).
pub trait PluginInvoke: Send + Sync {
    fn invoke(
        &self,
        input: &PluginInput,
        ctx: &mut PluginContext<'_>,
    ) -> Result<PluginOutput, crate::plugin::errors::PluginError>;
}

/// Descriptor that registers a plugin. AC-PL-1.
pub struct PluginDescriptor {
    pub name: String,
    pub version: String,
    pub input_content_types: Vec<ContentType>,
    pub output_content_type: ContentType,
    pub mode: PluginMode,
    pub supports_source_map: bool,
    /// Numeric priority honored within a mode (AC-PL-9). Lower runs first.
    pub priority: i32,
    /// Capability set the plugin is allowed to use. The Rust AST
    /// validator rejects a registration whose evidence exceeds this set
    /// (AC-PL-20, AC-PL-V-7).
    pub requires: BTreeSet<PluginCapability>,
    /// Static evidence produced by the CEM Rust AST validator. Tier B
    /// callers may pass [`PluginEvidence::empty`] when the AST walker
    /// has not yet been wired through.
    pub evidence: PluginEvidence,
    pub invoke: Arc<dyn PluginInvoke>,
}

impl std::fmt::Debug for PluginDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginDescriptor")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("input_content_types", &self.input_content_types)
            .field("output_content_type", &self.output_content_type)
            .field("mode", &self.mode)
            .field("supports_source_map", &self.supports_source_map)
            .field("priority", &self.priority)
            .field("requires", &self.requires)
            .field("evidence", &self.evidence)
            .finish_non_exhaustive()
    }
}

impl PluginDescriptor {
    pub fn matches_input(&self, ct: &ContentType) -> bool {
        self.input_content_types.iter().any(|c| c == ct)
    }
}
