//! Namespace context — Tier A scope-chain resolution.
//!
//! Mirrors the design in
//! `docs/cem-ml-stack-design-impl.md` §3.4.1. The CEM-ML surface uses
//! `@ns prefix = "uri"` and `@default "uri"` directives; the XML parity
//! surface uses `xmlns:prefix="uri"` and `xmlns="uri"` attributes. Both
//! lower into the same `NamespaceBinding` records here.
//!
//! Tier A guarantees per AC-P-10 and AC-P-V-1:
//!
//! - Repeated prefix bindings rebind from the source position of the new
//!   declaration; previously resolved nodes retain their original
//!   expanded name (no retroactive rebind).
//! - The blank/default binding (`""`) is rebindable the same way.
//! - Nested scopes inherit the active bindings of their parent until they
//!   shadow them with their own declarations.

use crate::source::ByteRange;
use crate::source_map::SourceMapStack;
use std::collections::HashMap;

pub type NamespaceBindingId = u32;
pub type NsContextId = u32;

#[derive(Debug, Clone)]
pub struct NamespaceBinding {
    pub binding_id: NamespaceBindingId,
    /// Prefix name; `""` is the default (blank) binding.
    pub name: String,
    pub namespace_uri: String,
    pub declared_at: ByteRange,
    pub effective_from: ByteRange,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedQName {
    pub prefix: Option<String>,
    pub local_name: String,
    /// `None` when the prefix is unbound. Tier A reports this as a lint
    /// via `cem.lint.unbound_prefix` already (see `validation/rules.rs`).
    pub namespace_uri: Option<String>,
    pub binding_id: Option<NamespaceBindingId>,
}

/// Scope-chain namespace context. The schema machine pushes a new
/// `NsContext` at every `OpenScope`, inheriting the parent's bindings,
/// and pops it on `CloseScope`.
#[derive(Debug, Clone, Default)]
pub struct NsContext {
    pub scope_id: u32,
    /// Local bindings declared at this scope, in declaration order.
    bindings: Vec<NamespaceBinding>,
    /// Effective binding lookup by prefix name. When the scope inherits
    /// from a parent, the parent's bindings are pre-populated here so
    /// resolution is O(1). Shadowing replaces the entry; pop on close
    /// restores the parent's entry from `inherited_overrides`.
    active: HashMap<String, NamespaceBinding>,
    /// Previous (parent) entries replaced by a local binding at this
    /// scope. Used to restore parent state when this scope closes.
    inherited_overrides: HashMap<String, Option<NamespaceBinding>>,
    next_binding_id: NamespaceBindingId,
}

impl NsContext {
    pub fn new(scope_id: u32) -> Self {
        Self {
            scope_id,
            bindings: Vec::new(),
            active: HashMap::new(),
            inherited_overrides: HashMap::new(),
            next_binding_id: 1,
        }
    }

    /// Build a child context inheriting every active binding from `parent`.
    pub fn child_of(parent: &NsContext, scope_id: u32) -> Self {
        Self {
            scope_id,
            bindings: Vec::new(),
            active: parent.active.clone(),
            inherited_overrides: HashMap::new(),
            next_binding_id: parent.next_binding_id,
        }
    }

    /// Register a binding declared at this scope. The binding takes
    /// effect from `effective_from` onwards; earlier resolutions at this
    /// scope are unaffected (caller responsibility — resolve before
    /// declaring if mid-scope).
    pub fn declare(
        &mut self,
        name: impl Into<String>,
        namespace_uri: impl Into<String>,
        declared_at: ByteRange,
        effective_from: ByteRange,
        source_map: SourceMapStack,
    ) -> NamespaceBindingId {
        let name = name.into();
        let binding_id = self.next_binding_id;
        self.next_binding_id += 1;
        let binding = NamespaceBinding {
            binding_id,
            name: name.clone(),
            namespace_uri: namespace_uri.into(),
            declared_at,
            effective_from,
            source_map,
        };
        // Stash the previous active entry once per scope so close can
        // restore the parent's binding cleanly. After the first override
        // at this scope, further re-declarations replace the local entry
        // without touching `inherited_overrides`.
        self.inherited_overrides
            .entry(name.clone())
            .or_insert_with(|| self.active.get(&name).cloned());
        self.active.insert(name.clone(), binding.clone());
        self.bindings.push(binding);
        binding_id
    }

    /// Resolve a lexical qname (`prefix:local` or `local`) against the
    /// current active bindings. `Option<&'_ str>` of `None` for the
    /// `prefix` argument resolves against the default binding.
    pub fn resolve(&self, lexical: &str) -> ResolvedQName {
        let (prefix, local) = match lexical.split_once(':') {
            Some((p, l)) => (Some(p.to_owned()), l.to_owned()),
            None => (None, lexical.to_owned()),
        };
        let lookup_key = prefix.as_deref().unwrap_or("");
        let active = self.active.get(lookup_key).cloned();
        ResolvedQName {
            prefix,
            local_name: local,
            namespace_uri: active.as_ref().map(|b| b.namespace_uri.clone()),
            binding_id: active.as_ref().map(|b| b.binding_id),
        }
    }

    /// Returns the active binding for a prefix name (`""` for default).
    pub fn binding(&self, prefix: &str) -> Option<&NamespaceBinding> {
        self.active.get(prefix)
    }

    pub fn local_bindings(&self) -> &[NamespaceBinding] {
        &self.bindings
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: u64, len: u32) -> ByteRange {
        ByteRange::new(start, len)
    }

    fn smap() -> SourceMapStack {
        SourceMapStack::default()
    }

    #[test]
    fn empty_context_resolves_unprefixed_to_unbound() {
        let ctx = NsContext::new(1);
        let r = ctx.resolve("button");
        assert_eq!(r.prefix, None);
        assert_eq!(r.local_name, "button");
        assert!(r.namespace_uri.is_none());
    }

    #[test]
    fn declared_prefix_resolves_to_uri() {
        let mut ctx = NsContext::new(1);
        ctx.declare(
            "cem",
            "https://cem.dev/ns/core/1",
            range(0, 4),
            range(4, 0),
            smap(),
        );
        let r = ctx.resolve("cem:screen");
        assert_eq!(r.prefix.as_deref(), Some("cem"));
        assert_eq!(r.local_name, "screen");
        assert_eq!(
            r.namespace_uri.as_deref(),
            Some("https://cem.dev/ns/core/1")
        );
        assert!(r.binding_id.is_some());
    }

    #[test]
    fn default_binding_resolves_unprefixed_names() {
        let mut ctx = NsContext::new(1);
        ctx.declare(
            "",
            "http://www.w3.org/1999/xhtml",
            range(0, 8),
            range(8, 0),
            smap(),
        );
        let r = ctx.resolve("button");
        assert_eq!(r.prefix, None);
        assert_eq!(
            r.namespace_uri.as_deref(),
            Some("http://www.w3.org/1999/xhtml")
        );
    }

    #[test]
    fn child_inherits_parent_bindings() {
        let mut parent = NsContext::new(1);
        parent.declare(
            "cem",
            "https://cem.dev/ns/core/1",
            range(0, 4),
            range(4, 0),
            smap(),
        );
        let child = NsContext::child_of(&parent, 2);
        let r = child.resolve("cem:action");
        assert_eq!(
            r.namespace_uri.as_deref(),
            Some("https://cem.dev/ns/core/1")
        );
    }

    #[test]
    fn child_shadows_parent_binding_locally() {
        let mut parent = NsContext::new(1);
        parent.declare("x", "uri:outer", range(0, 4), range(4, 0), smap());
        let mut child = NsContext::child_of(&parent, 2);
        child.declare("x", "uri:inner", range(10, 4), range(14, 0), smap());
        assert_eq!(
            child.resolve("x:y").namespace_uri.as_deref(),
            Some("uri:inner")
        );
        // Parent unaffected.
        assert_eq!(
            parent.resolve("x:y").namespace_uri.as_deref(),
            Some("uri:outer")
        );
    }

    #[test]
    fn rebinding_same_prefix_uses_latest_at_resolve_time() {
        let mut ctx = NsContext::new(1);
        ctx.declare("p", "uri:first", range(0, 1), range(1, 0), smap());
        assert_eq!(
            ctx.resolve("p:x").namespace_uri.as_deref(),
            Some("uri:first")
        );
        ctx.declare("p", "uri:second", range(10, 1), range(11, 0), smap());
        // After the rebind, resolution picks the new URI; earlier
        // resolved names (captured before rebind) stay on uri:first per
        // AC-P-10 — that's the caller's responsibility to snapshot.
        assert_eq!(
            ctx.resolve("p:x").namespace_uri.as_deref(),
            Some("uri:second")
        );
    }

    #[test]
    fn default_namespace_rebinding_round_trips_html_svg_html() {
        // Mirrors AC-P-V-1: HTML default → SVG default → HTML default
        // rebound in one document.
        let mut html = NsContext::new(1);
        html.declare(
            "",
            "http://www.w3.org/1999/xhtml",
            range(0, 0),
            range(0, 0),
            smap(),
        );
        assert_eq!(
            html.resolve("input").namespace_uri.as_deref(),
            Some("http://www.w3.org/1999/xhtml")
        );

        let mut svg = NsContext::child_of(&html, 2);
        svg.declare(
            "",
            "http://www.w3.org/2000/svg",
            range(20, 0),
            range(20, 0),
            smap(),
        );
        assert_eq!(
            svg.resolve("path").namespace_uri.as_deref(),
            Some("http://www.w3.org/2000/svg")
        );

        // A nested element that rebinds the default back to HTML.
        let mut back_to_html = NsContext::child_of(&svg, 3);
        back_to_html.declare(
            "",
            "http://www.w3.org/1999/xhtml",
            range(40, 0),
            range(40, 0),
            smap(),
        );
        assert_eq!(
            back_to_html.resolve("input").namespace_uri.as_deref(),
            Some("http://www.w3.org/1999/xhtml")
        );

        // svg scope unaffected by the inner rebind.
        assert_eq!(
            svg.resolve("path").namespace_uri.as_deref(),
            Some("http://www.w3.org/2000/svg")
        );
    }

    #[test]
    fn local_bindings_lists_only_this_scopes_declarations() {
        let mut parent = NsContext::new(1);
        parent.declare("cem", "uri:cem", range(0, 4), range(4, 0), smap());
        let mut child = NsContext::child_of(&parent, 2);
        child.declare("html", "uri:html", range(20, 5), range(25, 0), smap());
        let names: Vec<&str> = child
            .local_bindings()
            .iter()
            .map(|b| b.name.as_str())
            .collect();
        assert_eq!(names, vec!["html"]);
    }
}
