//! Operation-owned native import; parser ownership and typed source maps remain intact.
use super::*;
use crate::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    value::artifact::CemValueArtifactLimits,
};
pub fn import_data_bytes_controlled(
    bytes: &[u8],
    content_type: &str,
    projection: &str,
    source_uri: &str,
    control: &OperationControl,
    limits: &CemValueArtifactLimits,
) -> Result<Arc<RetainedCemTree>, String> {
    let check = || {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_owned())
    };
    check()?;
    if bytes.len() > limits.max_bytes.min(MAX_DOCUMENT_BYTES) {
        return Err("Controlled import byte limit exceeded".into());
    }
    let permit = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            bytes.len().saturating_mul(128).saturating_add(1024) as u64,
            None,
        )
        .map_err(|e| e.code().to_owned())?;
    let mut tree = import_data_bytes(bytes, content_type, projection, source_uri)?;
    check()?;
    let policy = control
        .scope_tree()
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .ok_or("Missing import scope")?
        .effective_policy;
    if tree.ast().nodes.len() > limits.max_values {
        return Err("Controlled import value limit exceeded".into());
    }
    let mut pending = vec![(0, 0usize)];
    while let Some((id, depth)) = pending.pop() {
        check()?;
        if depth > limits.max_depth.min(64).min(policy.stack_depth as usize) {
            return Err("Controlled import depth limit exceeded".into());
        }
        let node = tree.node(id).ok_or("Missing imported node")?;
        pending.extend(
            node.children
                .iter()
                .chain(&node.attributes)
                .map(|id| (*id, depth + 1)),
        );
    }
    let arena = control
        .charge_memory(
            ROOT_EXECUTION_SCOPE_ID,
            tree.ast()
                .nodes
                .len()
                .saturating_mul(1024)
                .saturating_add(source_uri.len()) as u64,
            None,
        )
        .map_err(|e| e.code().to_owned())?;
    RetainedCemTree::retain_resource(&mut tree, Arc::new(permit))?;
    RetainedCemTree::retain_resource(&mut tree, Arc::new(arena))?;
    Ok(tree)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{AbortSignal, ScopePolicy};
    fn control() -> OperationControl {
        OperationControl::new(AbortSignal::new())
    }
    #[test]
    fn controlled_import_keeps_native_owner_maps_and_memory_through_last_tree_release() {
        let c = control();
        let tree = import_data_bytes_controlled(
            b"p {color:red}",
            "text/css",
            "cem",
            "https://example.test/source.css",
            &c,
            &Default::default(),
        )
        .unwrap();
        assert!(tree
            .native_owner()
            .unwrap()
            .downcast_ref::<LoadedInputAstStream>()
            .is_some());
        assert_eq!(tree.source_uri(), "https://example.test/source.css");
        assert!(!tree.node_source_map(1).unwrap().frames.is_empty());
        let clone = tree.clone();
        let charged = c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap();
        assert!(charged > 0);
        drop(tree);
        assert_eq!(charged, c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap());
        drop(clone);
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
    #[test]
    fn controlled_import_refuses_lowered_limits_and_cancellation_without_leaks() {
        let c = control();
        for limits in [
            CemValueArtifactLimits {
                max_bytes: 1,
                ..Default::default()
            },
            CemValueArtifactLimits {
                max_values: 1,
                ..Default::default()
            },
            CemValueArtifactLimits {
                max_depth: 1,
                ..Default::default()
            },
        ] {
            assert!(import_data_bytes_controlled(
                b"p {color:red}",
                "text/css",
                "cem",
                "https://example.test/source.css",
                &c,
                &limits
            )
            .is_err());
            assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
        }
        let low = OperationControl::with_root_policy(
            AbortSignal::new(),
            ScopePolicy::host_root().with_memory_bytes(32),
        )
        .unwrap();
        assert!(import_data_bytes_controlled(
            b"p {color:red}",
            "text/css",
            "cem",
            "https://example.test/source.css",
            &low,
            &Default::default()
        )
        .is_err());
        c.abort_signal().abort();
        assert!(import_data_bytes_controlled(
            b"p {}",
            "text/css",
            "cem",
            "https://example.test/source.css",
            &c,
            &Default::default()
        )
        .is_err());
    }
}
