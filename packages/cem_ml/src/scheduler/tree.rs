//! Scope policy inheritance with constrain-only semantics
//! (AC-A-4 second paragraph, AC-P-5).

use crate::scheduler::policy::{ResourceCap, ScopePolicy};
use std::collections::HashMap;

/// Stable identity of a scope inside the policy tree. Mirrors
/// [`crate::plugin::descriptor::ScopeId`] but kept local so the
/// scheduler can be used without depending on the plugin module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PolicyScopeId(pub u32);

/// Tree of scope policies. Inheritance is *constrain-only*: a child
/// scope MAY lower any cap below its parent's value, MAY keep parity,
/// but MUST NOT raise above the parent's bound. Attempts to relax
/// surface as [`ScopePolicyTreeError::CapRelaxationDenied`] with the
/// stable diagnostic code `cem.a.cap_relaxation_denied`.
#[derive(Debug)]
pub struct ScopePolicyTree {
    nodes: HashMap<PolicyScopeId, Node>,
    root: PolicyScopeId,
}

#[derive(Debug, Clone)]
struct Node {
    parent: Option<PolicyScopeId>,
    policy: ScopePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopePolicyTreeError {
    /// A child scope tried to raise a cap above the parent bound
    /// (AC-A-4). Diagnostic code: `cem.a.cap_relaxation_denied`.
    CapRelaxationDenied {
        scope: u32,
        cap: ResourceCap,
        parent_value: u64,
        attempted_value: u64,
    },
    /// Parent scope is not registered in the tree.
    UnknownParent { scope: u32, parent: u32 },
    /// Scope already exists.
    DuplicateScope(u32),
}

impl std::fmt::Display for ScopePolicyTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopePolicyTreeError::CapRelaxationDenied {
                scope,
                cap,
                parent_value,
                attempted_value,
            } => write!(
                f,
                "scope {scope} attempted to raise cap {cap:?} from parent bound {parent_value} to {attempted_value} (cem.a.cap_relaxation_denied)"
            ),
            ScopePolicyTreeError::UnknownParent { scope, parent } => {
                write!(f, "scope {scope} references unknown parent {parent}")
            }
            ScopePolicyTreeError::DuplicateScope(s) => write!(f, "scope {s} already registered"),
        }
    }
}

impl std::error::Error for ScopePolicyTreeError {}

impl ScopePolicyTreeError {
    pub fn code(&self) -> &'static str {
        match self {
            ScopePolicyTreeError::CapRelaxationDenied { .. } => "cem.a.cap_relaxation_denied",
            ScopePolicyTreeError::UnknownParent { .. } => "cem.a.unknown_parent_scope",
            ScopePolicyTreeError::DuplicateScope(_) => "cem.a.duplicate_scope",
        }
    }
}

impl ScopePolicyTree {
    /// Install the root scope's policy.
    pub fn new(root: PolicyScopeId, policy: ScopePolicy) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(
            root,
            Node {
                parent: None,
                policy,
            },
        );
        Self { nodes, root }
    }

    pub fn root(&self) -> PolicyScopeId {
        self.root
    }

    pub fn policy(&self, scope: PolicyScopeId) -> Option<&ScopePolicy> {
        self.nodes.get(&scope).map(|n| &n.policy)
    }

    /// Install a child scope policy. Returns
    /// [`ScopePolicyTreeError::CapRelaxationDenied`] for any cap that
    /// exceeds the parent's value.
    pub fn install(
        &mut self,
        scope: PolicyScopeId,
        parent: PolicyScopeId,
        policy: ScopePolicy,
    ) -> Result<(), ScopePolicyTreeError> {
        if self.nodes.contains_key(&scope) {
            return Err(ScopePolicyTreeError::DuplicateScope(scope.0));
        }
        let parent_policy = self
            .nodes
            .get(&parent)
            .map(|n| n.policy)
            .ok_or(ScopePolicyTreeError::UnknownParent {
                scope: scope.0,
                parent: parent.0,
            })?;
        check_constrain_only(scope.0, &parent_policy, &policy)?;
        self.nodes.insert(
            scope,
            Node {
                parent: Some(parent),
                policy,
            },
        );
        Ok(())
    }

    /// Walk from `scope` to the root, yielding `(scope_id, policy)`.
    pub fn ancestors(&self, scope: PolicyScopeId) -> Vec<(PolicyScopeId, ScopePolicy)> {
        let mut out = Vec::new();
        let mut cursor = Some(scope);
        while let Some(id) = cursor {
            if let Some(node) = self.nodes.get(&id) {
                out.push((id, node.policy));
                cursor = node.parent;
            } else {
                break;
            }
        }
        out
    }

    /// Effective policy seen at `scope`: identical to the registered
    /// policy because installs already enforced constrain-only.
    pub fn effective(&self, scope: PolicyScopeId) -> Option<ScopePolicy> {
        self.nodes.get(&scope).map(|n| n.policy)
    }
}

fn check_constrain_only(
    scope: u32,
    parent: &ScopePolicy,
    child: &ScopePolicy,
) -> Result<(), ScopePolicyTreeError> {
    fn deny(
        scope: u32,
        cap: ResourceCap,
        parent_value: u64,
        attempted_value: u64,
    ) -> ScopePolicyTreeError {
        ScopePolicyTreeError::CapRelaxationDenied {
            scope,
            cap,
            parent_value,
            attempted_value,
        }
    }
    if child.cpu_workers > parent.cpu_workers {
        return Err(deny(
            scope,
            ResourceCap::CpuWorkers,
            parent.cpu_workers as u64,
            child.cpu_workers as u64,
        ));
    }
    if child.queue_size > parent.queue_size {
        return Err(deny(
            scope,
            ResourceCap::QueueSize,
            parent.queue_size as u64,
            child.queue_size as u64,
        ));
    }
    if child.io_streams > parent.io_streams {
        return Err(deny(
            scope,
            ResourceCap::IoStreams,
            parent.io_streams as u64,
            child.io_streams as u64,
        ));
    }
    if child.memory_bytes > parent.memory_bytes {
        return Err(deny(
            scope,
            ResourceCap::MemoryBytes,
            parent.memory_bytes,
            child.memory_bytes,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::policy::OverflowPolicy;

    fn policy(cpu: u32, queue: u32, io: u32, mem: u64) -> ScopePolicy {
        ScopePolicy {
            cpu_workers: cpu,
            queue_size: queue,
            io_streams: io,
            memory_bytes: mem,
            overflow: OverflowPolicy::Reject,
        }
    }

    #[test]
    fn child_lowering_caps_is_accepted() {
        let mut tree = ScopePolicyTree::new(PolicyScopeId(0), policy(4, 32, 16, 1024));
        tree.install(PolicyScopeId(1), PolicyScopeId(0), policy(2, 16, 8, 512))
            .unwrap();
    }

    #[test]
    fn child_matching_parent_caps_is_accepted() {
        let mut tree = ScopePolicyTree::new(PolicyScopeId(0), policy(4, 32, 16, 1024));
        tree.install(PolicyScopeId(1), PolicyScopeId(0), policy(4, 32, 16, 1024))
            .unwrap();
    }

    #[test]
    fn child_raising_cpu_workers_is_denied() {
        let mut tree = ScopePolicyTree::new(PolicyScopeId(0), policy(4, 32, 16, 1024));
        let err = tree
            .install(PolicyScopeId(1), PolicyScopeId(0), policy(8, 32, 16, 1024))
            .unwrap_err();
        assert_eq!(err.code(), "cem.a.cap_relaxation_denied");
        assert!(matches!(
            err,
            ScopePolicyTreeError::CapRelaxationDenied { cap: ResourceCap::CpuWorkers, .. }
        ));
    }

    #[test]
    fn each_cap_is_independently_checked() {
        let mut tree = ScopePolicyTree::new(PolicyScopeId(0), policy(4, 32, 16, 1024));
        for (name, p) in [
            ("queue", policy(4, 33, 16, 1024)),
            ("io", policy(4, 32, 17, 1024)),
            ("memory", policy(4, 32, 16, 2048)),
        ] {
            let err = tree
                .install(PolicyScopeId(1), PolicyScopeId(0), p)
                .unwrap_err();
            assert!(matches!(err, ScopePolicyTreeError::CapRelaxationDenied { .. }), "cap {name}");
        }
    }

    #[test]
    fn ancestors_walk_to_root() {
        let mut tree = ScopePolicyTree::new(PolicyScopeId(0), policy(8, 64, 16, 4096));
        tree.install(PolicyScopeId(1), PolicyScopeId(0), policy(4, 32, 8, 2048))
            .unwrap();
        tree.install(PolicyScopeId(2), PolicyScopeId(1), policy(2, 16, 4, 1024))
            .unwrap();
        let ids: Vec<u32> = tree
            .ancestors(PolicyScopeId(2))
            .iter()
            .map(|(s, _)| s.0)
            .collect();
        assert_eq!(ids, vec![2, 1, 0]);
    }
}
