//! Scope-local plugin registry and outer→inner chain merging
//! (AC-PL-6, AC-PL-7, AC-PL-8, AC-PL-9, AC-PL-11).

use crate::plugin::descriptor::{ContentType, PluginDescriptor, PluginMode, ScopeId};
use crate::plugin::errors::PluginError;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Plugins installed on a single scope. Preserves install order so
/// chain merging is deterministic.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Arc<PluginDescriptor>>,
    /// Tracks plugin names that arrived from ancestors. A descendant
    /// scope MUST NOT remove these (AC-PL-8); the registry rejects
    /// such attempts with [`PluginError::Inheritance`].
    sealed_ancestor_names: BTreeSet<String>,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugin_names", &self.plugin_names())
            .field("sealed_ancestor_names", &self.sealed_ancestor_names)
            .finish()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a scope-local plugin. AC-PL-4 / AC-PL-20 contract
    /// checks are enforced at registration: mutate plugins MUST set
    /// `supports_source_map`, and the descriptor's evidence MUST be
    /// covered by `requires`.
    pub fn install(&mut self, descriptor: Arc<PluginDescriptor>) -> Result<(), PluginError> {
        validate_registration(&descriptor)?;
        self.plugins.push(descriptor);
        Ok(())
    }

    /// Remove a scope-local plugin by name (AC-PL-19). Returns
    /// `Ok(true)` if removed, `Ok(false)` if not present. Removal of
    /// ancestor-installed plugins is rejected per AC-PL-8.
    pub fn uninstall(&mut self, plugin_name: &str, scope: ScopeId) -> Result<bool, PluginError> {
        if self.sealed_ancestor_names.contains(plugin_name) {
            return Err(PluginError::Inheritance {
                plugin: plugin_name.to_owned(),
                scope: scope.0,
            });
        }
        let before = self.plugins.len();
        self.plugins
            .retain(|p| p.name != plugin_name || self.sealed_ancestor_names.contains(&p.name));
        Ok(self.plugins.len() < before)
    }

    /// Record that a plugin from an ancestor scope is present at this
    /// scope. Future `uninstall` attempts on the same name will be
    /// rejected with [`PluginError::Inheritance`].
    pub fn seal_from_ancestor(&mut self, plugin: Arc<PluginDescriptor>) {
        self.sealed_ancestor_names.insert(plugin.name.clone());
        self.plugins.push(plugin);
    }

    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.name.clone()).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<PluginDescriptor>> {
        self.plugins.iter()
    }
}

/// Merged plugin chain visible to one scope at apply time. Order is
/// outer-most ancestor → inner-most scope, with `observe` plugins
/// preceding `mutate` plugins within each tier (AC-PL-9). Within a
/// mode, lower `priority` runs first; ties preserve registration
/// order.
#[derive(Debug, Default)]
pub struct PluginChain {
    pub observe: Vec<Arc<PluginDescriptor>>,
    pub mutate: Vec<Arc<PluginDescriptor>>,
}

impl PluginChain {
    pub fn is_empty(&self) -> bool {
        self.observe.is_empty() && self.mutate.is_empty()
    }

    pub fn len(&self) -> usize {
        self.observe.len() + self.mutate.len()
    }

    /// Build the chain a scope sees at apply time. `ancestors_outer_first`
    /// lists registries in outer→inner order (root first); `local` is the
    /// scope's own registry. Plugins are filtered by `content_type`.
    pub fn merged(
        ancestors_outer_first: &[&PluginRegistry],
        local: &PluginRegistry,
        content_type: &ContentType,
    ) -> Self {
        let mut chain = PluginChain::default();
        let push = |target: &mut Vec<Arc<PluginDescriptor>>, plugin: &Arc<PluginDescriptor>| {
            target.push(plugin.clone());
        };
        for registry in ancestors_outer_first {
            for plugin in registry.iter() {
                if !plugin.matches_input(content_type) {
                    continue;
                }
                match plugin.mode {
                    PluginMode::Observe => push(&mut chain.observe, plugin),
                    PluginMode::Mutate => push(&mut chain.mutate, plugin),
                }
            }
        }
        for plugin in local.iter() {
            if !plugin.matches_input(content_type) {
                continue;
            }
            match plugin.mode {
                PluginMode::Observe => push(&mut chain.observe, plugin),
                PluginMode::Mutate => push(&mut chain.mutate, plugin),
            }
        }
        // AC-PL-9: within a mode honor declared priority; preserve
        // registration order on ties via stable sort.
        chain.observe.sort_by_key(|p| p.priority);
        chain.mutate.sort_by_key(|p| p.priority);
        chain
    }
}

fn validate_registration(descriptor: &PluginDescriptor) -> Result<(), PluginError> {
    // AC-PL-4.
    if descriptor.mode == PluginMode::Mutate && !descriptor.supports_source_map {
        return Err(PluginError::SourceMapRequired {
            plugin: descriptor.name.clone(),
        });
    }
    // AC-PL-20 / AC-PL-V-7: every capability the AST validator
    // identified must be declared in `requires`.
    let mut missing: Vec<String> = descriptor
        .evidence
        .needs
        .iter()
        .filter(|cap| !descriptor.requires.contains(cap))
        .map(|cap| format!("{cap:?}"))
        .collect();
    if !missing.is_empty() {
        missing.sort();
        missing.dedup();
        return Err(PluginError::Capability {
            plugin: descriptor.name.clone(),
            missing,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::descriptor::{
        PluginCapability, PluginContext, PluginEvidence, PluginInput, PluginInvoke, PluginOutput,
    };

    struct Noop;
    impl PluginInvoke for Noop {
        fn invoke(
            &self,
            input: &PluginInput,
            ctx: &mut PluginContext<'_>,
        ) -> Result<PluginOutput, PluginError> {
            Ok(PluginOutput::observed(input, ctx.inbound_source_map.clone()))
        }
    }

    fn observe_descriptor(name: &str, ct: &str, priority: i32) -> Arc<PluginDescriptor> {
        Arc::new(PluginDescriptor {
            name: name.into(),
            version: "0.1".into(),
            input_content_types: vec![ct.into()],
            output_content_type: ct.into(),
            mode: PluginMode::Observe,
            supports_source_map: false,
            priority,
            requires: BTreeSet::new(),
            evidence: PluginEvidence::empty(),
            invoke: Arc::new(Noop),
        })
    }

    fn mutate_descriptor(name: &str, ct: &str, priority: i32) -> Arc<PluginDescriptor> {
        Arc::new(PluginDescriptor {
            name: name.into(),
            version: "0.1".into(),
            input_content_types: vec![ct.into()],
            output_content_type: ct.into(),
            mode: PluginMode::Mutate,
            supports_source_map: true,
            priority,
            requires: BTreeSet::new(),
            evidence: PluginEvidence::empty(),
            invoke: Arc::new(Noop),
        })
    }

    #[test]
    fn mutate_without_source_map_is_rejected() {
        let bad = Arc::new(PluginDescriptor {
            name: "bad".into(),
            version: "0.1".into(),
            input_content_types: vec!["text/css".into()],
            output_content_type: "text/css".into(),
            mode: PluginMode::Mutate,
            supports_source_map: false,
            priority: 0,
            requires: BTreeSet::new(),
            evidence: PluginEvidence::empty(),
            invoke: Arc::new(Noop),
        });
        let mut reg = PluginRegistry::new();
        let err = reg.install(bad).unwrap_err();
        assert!(matches!(err, PluginError::SourceMapRequired { .. }));
    }

    #[test]
    fn evidence_exceeding_requires_is_rejected() {
        let bad = Arc::new(PluginDescriptor {
            name: "leaky".into(),
            version: "0.1".into(),
            input_content_types: vec!["text/css".into()],
            output_content_type: "text/css".into(),
            mode: PluginMode::Observe,
            supports_source_map: false,
            priority: 0,
            requires: BTreeSet::new(),
            evidence: PluginEvidence::from([PluginCapability::FilesystemRead]),
            invoke: Arc::new(Noop),
        });
        let mut reg = PluginRegistry::new();
        let err = reg.install(bad).unwrap_err();
        assert!(matches!(err, PluginError::Capability { .. }));
    }

    #[test]
    fn merge_orders_observe_before_mutate_and_outer_before_inner() {
        let mut root = PluginRegistry::new();
        root.install(observe_descriptor("a-outer-observe", "text/css", 0))
            .unwrap();
        root.install(mutate_descriptor("a-outer-mutate", "text/css", 0))
            .unwrap();

        let mut local = PluginRegistry::new();
        local
            .install(observe_descriptor("b-inner-observe", "text/css", 0))
            .unwrap();
        local
            .install(mutate_descriptor("b-inner-mutate", "text/css", 0))
            .unwrap();

        let chain = PluginChain::merged(&[&root], &local, &"text/css".into());
        let observe_names: Vec<_> = chain.observe.iter().map(|p| p.name.clone()).collect();
        let mutate_names: Vec<_> = chain.mutate.iter().map(|p| p.name.clone()).collect();
        assert_eq!(observe_names, vec!["a-outer-observe", "b-inner-observe"]);
        assert_eq!(mutate_names, vec!["a-outer-mutate", "b-inner-mutate"]);
    }

    #[test]
    fn priority_orders_within_mode() {
        let mut local = PluginRegistry::new();
        local
            .install(observe_descriptor("late", "text/css", 50))
            .unwrap();
        local
            .install(observe_descriptor("early", "text/css", 10))
            .unwrap();
        let chain = PluginChain::merged(&[], &local, &"text/css".into());
        let names: Vec<_> = chain.observe.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["early", "late"]);
    }

    #[test]
    fn descendant_cannot_uninstall_ancestor_plugin() {
        let mut local = PluginRegistry::new();
        let ancestor = observe_descriptor("ancestor", "text/css", 0);
        local.seal_from_ancestor(ancestor);
        let err = local.uninstall("ancestor", ScopeId(7)).unwrap_err();
        assert!(matches!(err, PluginError::Inheritance { scope: 7, .. }));
    }

    #[test]
    fn descendant_can_uninstall_its_own_plugin() {
        let mut local = PluginRegistry::new();
        local
            .install(observe_descriptor("local", "text/css", 0))
            .unwrap();
        assert!(local.uninstall("local", ScopeId(1)).unwrap());
    }

    #[test]
    fn content_type_filter_skips_non_matching_plugins() {
        let mut local = PluginRegistry::new();
        local
            .install(observe_descriptor("scss-only", "text/scss", 0))
            .unwrap();
        local
            .install(observe_descriptor("css-only", "text/css", 0))
            .unwrap();
        let chain = PluginChain::merged(&[], &local, &"text/css".into());
        let names: Vec<_> = chain.observe.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, vec!["css-only"]);
    }
}
