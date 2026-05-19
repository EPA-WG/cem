//! `ScopedRegistryTree` — inherited lookup + collision detection
//! (AC-R-2, AC-R-3).

use crate::registry::registry::{CollisionDiagnostic, TemplateRegistry};
use crate::registry::template_ref::TemplateRef;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegistryScopeId(pub u32);

#[derive(Debug)]
pub struct ScopedRegistryTree {
    nodes: HashMap<RegistryScopeId, Node>,
    root: RegistryScopeId,
}

#[derive(Debug)]
struct Node {
    parent: Option<RegistryScopeId>,
    registry: TemplateRegistry,
}

/// Outcome of [`ScopedRegistryTree::resolve`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupResult<'a> {
    pub scope: RegistryScopeId,
    pub template_ref: &'a TemplateRef,
}

impl ScopedRegistryTree {
    pub fn new(root: RegistryScopeId) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(
            root,
            Node {
                parent: None,
                registry: TemplateRegistry::new(),
            },
        );
        Self { nodes, root }
    }

    pub fn root(&self) -> RegistryScopeId {
        self.root
    }

    pub fn scope_exists(&self, scope: RegistryScopeId) -> bool {
        self.nodes.contains_key(&scope)
    }

    pub fn add_scope(&mut self, scope: RegistryScopeId, parent: RegistryScopeId) -> bool {
        if !self.nodes.contains_key(&parent) || self.nodes.contains_key(&scope) {
            return false;
        }
        self.nodes.insert(
            scope,
            Node {
                parent: Some(parent),
                registry: TemplateRegistry::new(),
            },
        );
        true
    }

    /// Install a template in `scope`. AC-R-3: when an ancestor already
    /// owns an entry for the same name, a [`CollisionDiagnostic`] is
    /// returned; the install still succeeds (shadowing is permitted
    /// per AC-R-2's lookup semantics — the diagnostic is the warning
    /// hook).
    pub fn install(
        &mut self,
        scope: RegistryScopeId,
        name: impl Into<String>,
        template_ref: TemplateRef,
    ) -> Option<CollisionDiagnostic> {
        let name = name.into();
        // Walk ancestors first to capture the prior owner (if any).
        let ancestor_hit = self.find_ancestor_owner(scope, &name);
        let Some(node) = self.nodes.get_mut(&scope) else {
            return None;
        };
        node.registry.insert(name.clone(), template_ref.clone());
        ancestor_hit.map(|(ancestor_scope, ancestor_ref)| CollisionDiagnostic {
            name,
            child_scope: scope.0,
            ancestor_scope: ancestor_scope.0,
            child_ref: template_ref,
            ancestor_ref,
        })
    }

    /// AC-R-2 inherited lookup: search `scope` then walk to the root.
    pub fn resolve(&self, scope: RegistryScopeId, name: &str) -> Option<LookupResult<'_>> {
        let mut cursor = Some(scope);
        while let Some(id) = cursor {
            let node = self.nodes.get(&id)?;
            if let Some(template_ref) = node.registry.get(name) {
                return Some(LookupResult {
                    scope: id,
                    template_ref,
                });
            }
            cursor = node.parent;
        }
        None
    }

    fn find_ancestor_owner(
        &self,
        scope: RegistryScopeId,
        name: &str,
    ) -> Option<(RegistryScopeId, TemplateRef)> {
        let mut cursor = self.nodes.get(&scope)?.parent;
        while let Some(id) = cursor {
            let node = self.nodes.get(&id)?;
            if let Some(template_ref) = node.registry.get(name) {
                return Some((id, template_ref.clone()));
            }
            cursor = node.parent;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::template_ref::{RegistryId, TemplateRef};

    fn dce(tag: &str) -> TemplateRef {
        TemplateRef::DceTagName {
            tag_name: tag.into(),
        }
    }

    fn registry_entry(name: &str) -> TemplateRef {
        TemplateRef::RegistryEntry {
            registry_id: RegistryId(1),
            name: name.into(),
        }
    }

    #[test]
    fn local_install_resolves_locally() {
        let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
        assert!(tree.install(RegistryScopeId(0), "x-card", dce("x-card")).is_none());
        let hit = tree.resolve(RegistryScopeId(0), "x-card").unwrap();
        assert_eq!(hit.scope, RegistryScopeId(0));
        assert_eq!(*hit.template_ref, dce("x-card"));
    }

    #[test]
    fn missing_entry_returns_none() {
        let tree = ScopedRegistryTree::new(RegistryScopeId(0));
        assert!(tree.resolve(RegistryScopeId(0), "missing").is_none());
    }

    #[test]
    fn ac_r_2_lookup_falls_back_to_parent_registry() {
        let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
        tree.install(RegistryScopeId(0), "x-card", dce("x-card"));
        assert!(tree.add_scope(RegistryScopeId(1), RegistryScopeId(0)));
        let hit = tree.resolve(RegistryScopeId(1), "x-card").unwrap();
        // Resolution must surface the ancestor that owns the entry.
        assert_eq!(hit.scope, RegistryScopeId(0));
    }

    #[test]
    fn lookup_walks_multi_level_ancestry() {
        let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
        tree.install(RegistryScopeId(0), "x-card", dce("x-card"));
        tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
        tree.add_scope(RegistryScopeId(2), RegistryScopeId(1));
        tree.add_scope(RegistryScopeId(3), RegistryScopeId(2));
        let hit = tree.resolve(RegistryScopeId(3), "x-card").unwrap();
        assert_eq!(hit.scope, RegistryScopeId(0));
    }

    #[test]
    fn ac_r_3_shadowing_install_emits_collision_diagnostic() {
        let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
        tree.install(RegistryScopeId(0), "x-card", dce("x-card"));
        tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
        let collision = tree
            .install(RegistryScopeId(1), "x-card", registry_entry("custom-x-card"))
            .expect("expected collision diagnostic");
        assert_eq!(collision.code(), CollisionDiagnostic::CODE);
        assert_eq!(collision.child_scope, 1);
        assert_eq!(collision.ancestor_scope, 0);
        // Subsequent lookup at the child scope resolves to the child entry (shadowing).
        let hit = tree.resolve(RegistryScopeId(1), "x-card").unwrap();
        assert_eq!(hit.scope, RegistryScopeId(1));
        // Lookup at the root still resolves to the root entry (siblings
        // unaffected — see AC-X-2 isolation spirit).
        let hit = tree.resolve(RegistryScopeId(0), "x-card").unwrap();
        assert_eq!(hit.scope, RegistryScopeId(0));
    }

    #[test]
    fn local_install_in_new_scope_without_ancestor_owner_emits_no_collision() {
        let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
        tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
        // Parent has no entry for `x-card`; install in child is fine.
        assert!(tree.install(RegistryScopeId(1), "x-card", dce("x-card")).is_none());
    }

    #[test]
    fn install_in_unknown_scope_is_a_noop() {
        let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
        assert!(tree
            .install(RegistryScopeId(99), "x-card", dce("x-card"))
            .is_none());
        assert!(tree.resolve(RegistryScopeId(99), "x-card").is_none());
    }
}
