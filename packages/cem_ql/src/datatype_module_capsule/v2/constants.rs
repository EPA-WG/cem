//! Intended constant-reference topology only. This inventories node categories;
//! the shared compiler alone admits literal fields/values and actual selections.
use super::*;
fn source_node(
    modules: &BTreeMap<String, VerifiedDatatypeModule>,
    id: &ModuleNodeId,
) -> Result<SchemaDeclarationNode, ModuleVerificationError> {
    let module = modules.get(&id.module).ok_or("unknown constant module")?;
    SchemaDeclarationNode::new(module.capture.document().clone(), id.node)
        .ok_or_else(|| refused("datatype-module-constant-target-refused"))
}
fn is_reference(
    modules: &BTreeMap<String, VerifiedDatatypeModule>,
    id: &ModuleNodeId,
) -> Result<bool, ModuleVerificationError> {
    let source = source_node(modules, id)?;
    match source.node() {
        CemAstNode::Reference { targets: None, .. } => Ok(true),
        CemAstNode::Element { .. } => {
            let capture = &modules[&id.module].capture;
            let name = capture
                .expanded_name(source.document(), id.node)
                .ok_or("constant name unavailable")?;
            if name.local_name == "constant"
                && (name.namespace_uri.is_empty() || name.namespace_uri == CEM_SCHEMA_URI)
            {
                Ok(false)
            } else {
                Err(refused("datatype-module-constant-target-refused"))
            }
        }
        _ => Err(refused("datatype-module-constant-target-refused")),
    }
}
pub(super) fn inspect(
    records: &BTreeMap<String, ModuleRecord>,
    modules: &BTreeMap<String, VerifiedDatatypeModule>,
    work: &Work<'_>,
) -> Result<
    (
        BTreeMap<String, Vec<ConstantSite>>,
        Vec<ModuleNodeId>,
        Vec<(String, String, SchemaDeclarationNode)>,
    ),
    ModuleVerificationError,
> {
    let mut pending = BTreeSet::new();
    let mut edges = BTreeMap::<ModuleNodeId, BTreeSet<ModuleNodeId>>::new();
    let mut sites = BTreeMap::<String, Vec<(AstNodeId, ModuleNodeId)>>::new();
    let mut slot_count = 0;
    for (id, m) in records {
        for node in &m.constant_exports {
            work.check()?;
            let target = ModuleNodeId {
                module: id.clone(),
                node: *node,
            };
            if is_reference(modules, &target)? {
                pending.insert(target.clone());
            }
            edges.entry(target).or_default();
        }
        for (datatype, source) in &modules[id].sources {
            for slot in source.plan().constant_slots {
                work.check()?;
                slot_count += 1;
                if slot_count > MAX_FIELDS {
                    return Err(refused("datatype-module-constant-limit"));
                }
                let target = ModuleNodeId {
                    module: id.clone(),
                    node: slot.node_id(),
                };
                if is_reference(modules, &target)? {
                    pending.insert(target.clone());
                }
                edges.entry(target.clone()).or_default();
                sites
                    .entry(id.clone())
                    .or_default()
                    .push((*datatype, target));
            }
        }
    }
    let mut used = BTreeSet::new();
    let mut extra = vec![];
    while let Some(reference) = pending.pop_first() {
        work.check()?;
        if !used.insert(reference.clone()) {
            continue;
        }
        if used.len() > MAX_FIELDS {
            return Err(refused("datatype-module-constant-limit"));
        }
        let m = &records[&reference.module];
        let targets = m
            .constants
            .get(&reference.node)
            .ok_or("constant reference binding required")?;
        let witness = source_node(modules, &reference)?;
        for target in targets {
            work.check()?;
            let destination = records
                .get(&target.module)
                .ok_or("unknown constant module")?;
            if target.module != reference.module {
                if !destination.constant_exports.contains(&target.node) {
                    return Err(refused("datatype-module-constant-export-refused"));
                }
                extra.push((
                    reference.module.clone(),
                    target.module.clone(),
                    witness.clone(),
                ));
            }
            if is_reference(modules, target)? {
                pending.insert(target.clone());
            }
            edges.entry(target.clone()).or_default();
            edges
                .entry(reference.clone())
                .or_default()
                .insert(target.clone());
        }
    }
    for (id, m) in records {
        for node in m.constants.keys() {
            work.check()?;
            if !used.contains(&ModuleNodeId {
                module: id.clone(),
                node: *node,
            }) {
                return Err(refused("datatype-module-constant-unused"));
            }
        }
    }
    let order = order::order(&edges, work).map_err(|failure| match failure {
        order::Failure::Cycle(cycle) => ModuleVerificationError {
            code: "datatype-module-constant-cycle",
            related: cycle
                .iter()
                .filter_map(|id| source_node(modules, id).ok())
                .collect(),
            cycle_modules: cycle.into_iter().map(|id| id.module).collect(),
        },
        order::Failure::Depth => refused("datatype-module-constant-depth"),
        order::Failure::Refused => refused("datatype-module-graph-refused"),
    })?;
    let mut flattened = BTreeMap::<ModuleNodeId, Vec<ModuleNodeId>>::new();
    let mut cached_count = 0usize;
    for id in &order {
        work.check()?;
        let result = if is_reference(modules, id)? {
            let mut result = vec![];
            for target in &records[&id.module].constants[&id.node] {
                work.check()?;
                let children = &flattened[target];
                if result.len() + children.len() > 128 {
                    return Err(refused("datatype-module-constant-limit"));
                }
                for child in children {
                    work.check()?;
                    result.push(child.clone());
                }
            }
            result
        } else {
            vec![id.clone()]
        };
        cached_count = cached_count
            .checked_add(result.len())
            .ok_or("constant cache overflow")?;
        if cached_count > MAX_FIELDS {
            return Err(refused("datatype-module-constant-limit"));
        }
        flattened.insert(id.clone(), result);
    }
    let mut output = BTreeMap::new();
    let mut expanded_count = 0usize;
    for (id, slots) in sites {
        let mut result = vec![];
        for (datatype, slot) in slots {
            work.check()?;
            let targets = &flattened[&slot];
            expanded_count = expanded_count
                .checked_add(targets.len())
                .ok_or("constant expansion overflow")?;
            if expanded_count > MAX_FIELDS {
                return Err(refused("datatype-module-constant-limit"));
            }
            let targets = targets
                .iter()
                .map(|target| {
                    work.check()?;
                    source_node(modules, target)
                })
                .collect::<Result<Vec<_>, ModuleVerificationError>>()?;
            result.push(ConstantSite {
                datatype,
                slot: source_node(modules, &slot)?,
                targets,
            });
        }
        output.insert(id, result);
    }
    Ok((output, order, extra))
}
