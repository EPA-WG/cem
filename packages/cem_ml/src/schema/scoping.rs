//! Schema scoping — Tier A `cem:schema` forms + host-node attribute forms.
//!
//! Mirrors the design in `docs/cem-ml-stack-design.md` §13.1 and AC-F-2.
//! The four user-facing constructs:
//!
//! 1. **`@schema src="..."` prelude** — top-of-file shorthand. Sets the
//!    active schema for the document scope.
//! 2. **Inline declaration**:
//!    `{cem:schema @cem:name="..." | body}` declares an inline schema
//!    addressable by `cem:name` in descendant scopes. Declaration does
//!    *not* switch the parent scope's active schema.
//! 3. **Mid-document switch (element form)**:
//!    `{cem:schema @src="..." | body}` or `{cem:schema @select="..." | body}`
//!    wraps `body` in a scope whose active schema is the loaded one.
//!    A self-closing form `{cem:schema @src="..."}` opens a
//!    sibling-position scope until the parent close.
//! 4. **Mid-document switch (attribute form)**:
//!    `{element @cem:schema-src="..." | body}` or
//!    `{element @cem:schema-select="..." | body}` makes the host
//!    element a scope; the loaded schema applies inside only.
//!
//! Source attributes `src` / `cem:schema-src` carry URI literals;
//! `select` / `cem:schema-select` carry cem-ql expressions. The two are
//! mutually exclusive on the same host (AC-F-2 §"Source attributes").
//!
//! Tier A status: this module *tracks* schema-scoping declarations and
//! emits diagnostics for invalid combinations. Actual schema loading +
//! cem-ql evaluation lands with the schema-resolution + cem-ql layers.

use crate::source::ByteRange;
use crate::source_map::SourceMapStack;
use std::collections::HashMap;

/// What kind of schema source is bound on a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaSource {
    /// Top-of-file default schema for the document scope. No `src` or
    /// `select` resolved yet.
    Default,
    /// URI-literal load: `src="..."` or `cem:schema-src="..."`.
    Uri(String),
    /// cem-ql selector: `select="..."` or `cem:schema-select="..."`.
    /// Tier A records the expression; evaluation lands with cem-ql.
    Select(String),
    /// References an inline declaration by `cem:name`.
    InlineRef(String),
}

#[derive(Debug, Clone)]
pub struct InlineSchemaDeclaration {
    pub name: String,
    pub body_byte_range: ByteRange,
    /// Content-addressed cache identity per AC-CC-1. Tier A uses an
    /// FNV-1a 64-bit hash placeholder; the production form is
    /// `inline:<sha256-of-body>`.
    pub cache_identity: String,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone)]
pub struct SchemaScopeFrame {
    pub scope_id: u32,
    pub active: SchemaSource,
    /// Inline-schema declarations introduced at this scope.
    pub declared_inlines: HashMap<String, InlineSchemaDeclaration>,
    /// Snapshot of inherited inlines visible from this scope (declared
    /// at ancestors). Resolution walks this map then falls back to the
    /// declared map.
    pub inherited_inlines: HashMap<String, InlineSchemaDeclaration>,
}

impl SchemaScopeFrame {
    pub fn new_root(scope_id: u32) -> Self {
        Self {
            scope_id,
            active: SchemaSource::Default,
            declared_inlines: HashMap::new(),
            inherited_inlines: HashMap::new(),
        }
    }

    pub fn child_of(parent: &SchemaScopeFrame, scope_id: u32) -> Self {
        let mut inherited = parent.inherited_inlines.clone();
        // Child sees parent's declared inlines too.
        for (k, v) in &parent.declared_inlines {
            inherited.insert(k.clone(), v.clone());
        }
        Self {
            scope_id,
            active: parent.active.clone(),
            declared_inlines: HashMap::new(),
            inherited_inlines: inherited,
        }
    }

    pub fn declare_inline(&mut self, decl: InlineSchemaDeclaration) {
        self.declared_inlines.insert(decl.name.clone(), decl);
    }

    pub fn set_active(&mut self, source: SchemaSource) {
        self.active = source;
    }

    /// Resolve a `cem:name` lookup. Innermost-wins per AC-F-V-2: the
    /// scope's own declarations shadow inherited ones.
    pub fn resolve_name(&self, name: &str) -> Option<&InlineSchemaDeclaration> {
        self.declared_inlines
            .get(name)
            .or_else(|| self.inherited_inlines.get(name))
    }
}

/// Scope-chain context for schema scoping. The schema machine maintains
/// one of these alongside its `NsContext` stack — they describe the same
/// scope chain but track different state.
#[derive(Debug, Default, Clone)]
pub struct SchemaScopeContext {
    frames: Vec<SchemaScopeFrame>,
}

impl SchemaScopeContext {
    pub fn new() -> Self {
        Self {
            frames: vec![SchemaScopeFrame::new_root(0)],
        }
    }

    pub fn push(&mut self, scope_id: u32) {
        let frame = match self.frames.last() {
            Some(parent) => SchemaScopeFrame::child_of(parent, scope_id),
            None => SchemaScopeFrame::new_root(scope_id),
        };
        self.frames.push(frame);
    }

    pub fn pop(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    pub fn current(&self) -> &SchemaScopeFrame {
        self.frames.last().expect("schema scope stack has root")
    }

    pub fn current_mut(&mut self) -> &mut SchemaScopeFrame {
        self.frames.last_mut().expect("schema scope stack has root")
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Resolve a name walking from innermost to outermost scope.
    /// Innermost wins per AC-F-V-2.
    pub fn resolve_name(&self, name: &str) -> Option<&InlineSchemaDeclaration> {
        for frame in self.frames.iter().rev() {
            if let Some(decl) = frame.declared_inlines.get(name) {
                return Some(decl);
            }
        }
        None
    }
}

/// Tier A cache identity for an inline schema body. The production
/// form (`inline:<sha256-of-body>`) requires a sha2 dependency; the
/// FNV-1a 64-bit placeholder here is collision-resistant enough for
/// Tier A fixtures and identifies bodies content-addressed within one
/// crate build.
pub fn inline_cache_identity(body_bytes: &[u8]) -> String {
    let h = crate::ast::format::fnv1a64(body_bytes);
    format!("inline:fnv1a64:{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(name: &str, range: ByteRange) -> InlineSchemaDeclaration {
        InlineSchemaDeclaration {
            name: name.to_owned(),
            body_byte_range: range,
            cache_identity: inline_cache_identity(b""),
            source_map: SourceMapStack::default(),
        }
    }

    #[test]
    fn root_scope_starts_with_default_source_and_no_inlines() {
        let ctx = SchemaScopeContext::new();
        assert_eq!(ctx.current().active, SchemaSource::Default);
        assert!(ctx.current().declared_inlines.is_empty());
        assert!(ctx.current().inherited_inlines.is_empty());
    }

    #[test]
    fn child_inherits_parent_active_source() {
        let mut ctx = SchemaScopeContext::new();
        ctx.current_mut()
            .set_active(SchemaSource::Uri("schema://outer".to_owned()));
        ctx.push(1);
        assert_eq!(
            ctx.current().active,
            SchemaSource::Uri("schema://outer".to_owned())
        );
    }

    #[test]
    fn child_inherits_parent_inlines() {
        let mut ctx = SchemaScopeContext::new();
        ctx.current_mut()
            .declare_inline(decl("badge", ByteRange::new(0, 4)));
        ctx.push(1);
        assert!(ctx.current().inherited_inlines.contains_key("badge"));
        assert!(ctx.current().declared_inlines.is_empty());
    }

    #[test]
    fn child_inline_shadows_parent_via_resolve_name() {
        let mut ctx = SchemaScopeContext::new();
        ctx.current_mut()
            .declare_inline(decl("X", ByteRange::new(0, 4)));
        ctx.push(1);
        ctx.current_mut()
            .declare_inline(decl("X", ByteRange::new(10, 4)));
        // Inner scope resolves to the inner X.
        let inner = ctx.resolve_name("X").unwrap();
        assert_eq!(inner.body_byte_range, ByteRange::new(10, 4));
        // Pop back to the outer scope; the outer X is what's now
        // resolvable.
        ctx.pop();
        let outer = ctx.resolve_name("X").unwrap();
        assert_eq!(outer.body_byte_range, ByteRange::new(0, 4));
    }

    #[test]
    fn active_source_change_inside_child_does_not_leak_to_parent() {
        let mut ctx = SchemaScopeContext::new();
        ctx.current_mut()
            .set_active(SchemaSource::Uri("schema://outer".to_owned()));
        ctx.push(1);
        ctx.current_mut()
            .set_active(SchemaSource::Uri("schema://inner".to_owned()));
        assert_eq!(
            ctx.current().active,
            SchemaSource::Uri("schema://inner".to_owned())
        );
        ctx.pop();
        assert_eq!(
            ctx.current().active,
            SchemaSource::Uri("schema://outer".to_owned())
        );
    }

    #[test]
    fn cache_identity_is_content_addressed() {
        let body_a = b"body bytes";
        let body_b = b"body bytes";
        let body_c = b"other bytes";
        assert_eq!(inline_cache_identity(body_a), inline_cache_identity(body_b));
        assert_ne!(inline_cache_identity(body_a), inline_cache_identity(body_c));
        assert!(inline_cache_identity(body_a).starts_with("inline:fnv1a64:"));
    }

    #[test]
    fn pop_below_root_is_a_no_op() {
        let mut ctx = SchemaScopeContext::new();
        ctx.pop();
        ctx.pop();
        assert_eq!(ctx.depth(), 1);
    }
}
