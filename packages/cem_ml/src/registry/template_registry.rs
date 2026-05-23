//! Per-scope `TemplateRegistry` (AC-R-1).

use crate::registry::template_ref::TemplateRef;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub name: String,
    pub template_ref: TemplateRef,
}

#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    entries: BTreeMap<String, TemplateRef>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        name: impl Into<String>,
        template_ref: TemplateRef,
    ) -> Option<TemplateRef> {
        self.entries.insert(name.into(), template_ref)
    }

    pub fn remove(&mut self, name: &str) -> Option<TemplateRef> {
        self.entries.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&TemplateRef> {
        self.entries.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &TemplateRef)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Diagnostic emitted by the registry tree when a scope shadows an
/// ancestor's entry (AC-R-3). Stable code:
/// `cem.registry.shadowed_entry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionDiagnostic {
    pub name: String,
    pub child_scope: u32,
    pub ancestor_scope: u32,
    pub child_ref: TemplateRef,
    pub ancestor_ref: TemplateRef,
}

impl CollisionDiagnostic {
    pub const CODE: &'static str = "cem.registry.shadowed_entry";
    pub fn code(&self) -> &'static str {
        Self::CODE
    }
}

impl std::fmt::Display for CollisionDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "scope {} shadows ancestor scope {} for template name `{}` ({:?} ↦ {:?})",
            self.child_scope, self.ancestor_scope, self.name, self.ancestor_ref, self.child_ref
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::template_ref::{RegistryId, TemplateRef};

    fn entry(name: &str) -> TemplateRef {
        TemplateRef::RegistryEntry {
            registry_id: RegistryId(1),
            name: name.into(),
        }
    }

    #[test]
    fn insert_replaces_returns_prior_value() {
        let mut r = TemplateRegistry::new();
        assert!(r.insert("x", entry("a")).is_none());
        let prior = r.insert("x", entry("b")).unwrap();
        assert_eq!(prior, entry("a"));
        assert_eq!(r.get("x"), Some(&entry("b")));
    }

    #[test]
    fn remove_returns_prior_value() {
        let mut r = TemplateRegistry::new();
        r.insert("x", entry("a"));
        let removed = r.remove("x").unwrap();
        assert_eq!(removed, entry("a"));
        assert!(!r.contains("x"));
    }

    #[test]
    fn names_returns_lexicographic_keys() {
        let mut r = TemplateRegistry::new();
        r.insert("z", entry("z"));
        r.insert("a", entry("a"));
        r.insert("m", entry("m"));
        assert_eq!(r.names().collect::<Vec<_>>(), vec!["a", "m", "z"]);
    }
}
