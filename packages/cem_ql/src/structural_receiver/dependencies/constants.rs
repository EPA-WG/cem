//! Bounded original constant assertions, shared by inspection and compilation.
use super::*;
pub(super) struct Plan {
    pub references: BTreeMap<String, Vec<String>>,
    pub terminals: BTreeMap<String, Vec<String>>,
}
pub(super) fn plan(
    input: InputRef<'_>,
    limit: usize,
    control: &OperationControl,
) -> Result<Option<Plan>, String> {
    let InputRef::Assembly(graph) = input else {
        return Ok(None);
    };
    let mut work = limit;
    let mut spend = || {
        work = work.checked_sub(1).ok_or(REFUSED)?;
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| REFUSED)?;
        Ok::<(), String>(())
    };
    let original = |id: &crate::datatype_module_capsule::v2::ModuleNodeId| {
        let m = graph.module(&id.module).ok_or(REFUSED)?;
        SchemaDeclarationNode::new(m.capture().document().clone(), id.node).ok_or(REFUSED)
    };
    let mut expected = BTreeMap::new();
    for (reference, targets) in graph.constant_bindings() {
        spend()?;
        let targets = targets
            .iter()
            .map(|t| {
                spend()?;
                Ok(original(t)?.identity())
            })
            .collect::<Result<Vec<_>, String>>()?;
        if expected
            .insert(original(reference)?.identity(), targets)
            .is_some()
        {
            return Err(REFUSED.into());
        }
    }
    let mut terminals = BTreeMap::<String, Vec<String>>::new();
    for id in graph.module_ids() {
        for site in graph.module(id).ok_or(REFUSED)?.constant_sites() {
            spend()?;
            let source = graph
                .module(id)
                .ok_or(REFUSED)?
                .sources()
                .get(&site.datatype)
                .ok_or(REFUSED)?;
            let entry = terminals
                .entry(source.declaration().identity())
                .or_default();
            for target in &site.targets {
                spend()?;
                entry.push(target.identity());
            }
        }
    }
    Ok(Some(Plan {
        references: expected,
        terminals,
    }))
}
pub(super) fn select(
    allowed: &BTreeMap<String, DatatypeSource>,
    reachable: &BTreeSet<String>,
    adapter: &mut host::Host<'_>,
    remaining: &mut ReferenceTraversalLimits,
) -> Result<(usize, usize), String> {
    if adapter.constants.is_none() {
        return Ok((0, 0));
    }
    let mut sites = 0usize;
    let mut targets = 0usize;
    for id in reachable {
        adapter.check().map_err(|_| REFUSED)?;
        let source = allowed.get(id).ok_or(REFUSED)?;
        let slots = source.plan().constant_slots;
        if slots.is_empty() {
            continue;
        }
        if remaining.max_work == 0 {
            return Err(REFUSED.into());
        }
        let CemAstNode::Element {
            node_id,
            source: frames,
            ..
        } = source.declaration().node()
        else {
            return Err(REFUSED.into());
        };
        let origin = ReferenceOccurrence {
            identity: format!("datatype-constants:{}", source.declaration().identity()),
            node_id: Some(*node_id),
            expression: None,
            source_map: frames.clone(),
        };
        let root = adapter.inner.source_reference(source.declaration().clone());
        let mut first = true;
        let walk =
            resolve_owned_reference_structure(root, adapter, *remaining, origin, |host, _| {
                if !std::mem::take(&mut first) {
                    return None;
                }
                Some(
                    slots
                        .iter()
                        .map(|slot| host.inner.source_reference(slot.clone()))
                        .collect(),
                )
            })
            .map_err(|_| REFUSED)?;
        remaining.max_work = remaining
            .max_work
            .checked_sub(walk.resolution.work_used)
            .ok_or(REFUSED)?;
        adapter.check().map_err(|_| REFUSED)?;
        if !walk.resolution.is_complete() || walk.resolution.failed {
            return Err(REFUSED.into());
        }
        for node in &walk.resolution.nodes {
            if !adapter.ready(node) {
                return Err(REFUSED.into());
            }
        }
        let [root] = walk.roots.as_slice() else {
            return Err(REFUSED.into());
        };
        let selected = walk.children[*root]
            .iter()
            .map(|i| {
                adapter
                    .inner
                    .declaration_node(&walk.resolution.nodes[*i])
                    .map(|n| n.identity())
                    .ok_or(REFUSED)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if adapter.constants.ok_or(REFUSED)?.terminals.get(id) != Some(&selected) {
            return Err(REFUSED.into());
        }
        sites += slots.len();
        targets += selected.len();
    }
    adapter.check().map_err(|_| REFUSED)?;
    Ok((sites, targets))
}
