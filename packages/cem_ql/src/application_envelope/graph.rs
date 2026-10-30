use super::*;
pub(super) fn blobs(
    manifest: &Manifest,
    blobs: &BTreeMap<String, Blob>,
    work: &Work<'_>,
) -> Result<(), String> {
    if manifest.components.is_empty()
        || manifest.components.len() > MAX_COMPONENTS
        || manifest.contracts.len() > MAX_CONTRACTS
        || blobs.len() > 128
    {
        return Err(INVALID.into());
    }
    let required = manifest
        .components
        .values()
        .flat_map(|c| [c.template.clone(), c.metadata.clone()])
        .chain(manifest.contracts.keys().cloned())
        .collect::<BTreeSet<_>>();
    if required != blobs.keys().cloned().collect() {
        return Err(INVALID.into());
    }
    let mut total = 0usize;
    for (id, blob) in blobs {
        work.check()?;
        total = total.checked_add(blob.0.len()).ok_or(INVALID)?;
        if total > MAX_BYTES || !valid_hash(id) || hash(&blob.0) != *id {
            return Err(INVALID.into());
        }
    }
    Ok(())
}
pub(super) fn declarations(
    manifest: &Manifest,
    blobs: &BTreeMap<String, Blob>,
    work: &Work<'_>,
) -> Result<
    (
        BTreeMap<String, VerifiedComponent>,
        ApplicationRequirements,
        SelectedRegistryPlan,
    ),
    String,
> {
    if manifest.components.is_empty() || manifest.components.len() > MAX_COMPONENTS {
        return Err(INVALID.into());
    }
    let mut result = BTreeMap::new();
    let mut requirements = ApplicationRequirements::default();
    requirements.roots = manifest.roots.clone();
    for (tag, component) in &manifest.components {
        work.check()?;
        let template = &blobs.get(&component.template).ok_or(INVALID)?.0;
        let metadata = &blobs.get(&component.metadata).ok_or(INVALID)?.0;
        if template.len() > 1024 * 1024 || metadata.len() > 64 * 1024 {
            return Err(INVALID.into());
        }
        let json = std::str::from_utf8(metadata).map_err(|_| INVALID)?;
        let (metadata, artifact) =
            crate::application_declaration::load_template(template, json, &component.metadata)?;
        if metadata.tag != *tag {
            return Err(INVALID.into());
        }
        let inventory = crate::application_requirements::declaration_requirements(
            &artifact,
            &metadata,
            &component.metadata,
        )?;
        if !inventory.unresolved.is_empty() {
            return Err(INVALID.into());
        }
        let mut dependencies = BTreeSet::new();
        crate::declaration_closure::references(&artifact.nodes, &mut dependencies)?;
        if dependencies != component.dependencies
            || crate::application_declaration::required(&artifact)?
                != component.receivers.keys().cloned().collect()
            || component.receivers.len() > 64
        {
            return Err(INVALID.into());
        }
        if component.receivers.keys().any(|name| {
            name.is_empty()
                || name.len() > 256
                || crate::application_declaration::reserved(name)
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        }) {
            return Err(INVALID.into());
        }
        requirements
            .component_dependencies
            .insert(tag.clone(), dependencies);
        requirements.declarations.insert(tag.clone(), inventory);
        result.insert(tag.clone(), VerifiedComponent { metadata, artifact });
    }
    let plan = requirements.selected_registry_plan()?;
    Ok((result, requirements, plan))
}
pub(super) fn closure(manifest: &Manifest, work: &Work<'_>) -> Result<Vec<DependencyKey>, String> {
    if manifest.roots.is_empty()
        || manifest.roots.len() > MAX_COMPONENTS
        || manifest.contract_roots.len() > MAX_CONTRACTS
        || manifest
            .dependencies
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != manifest.contracts.keys().cloned().collect()
    {
        return Err(INVALID.into());
    }
    let mut graph = BTreeMap::new();
    for (tag, c) in &manifest.components {
        graph.insert(
            DependencyKey::Component(tag.clone()),
            c.dependencies
                .iter()
                .cloned()
                .map(DependencyKey::Component)
                .chain(
                    c.receivers
                        .values()
                        .map(|r| DependencyKey::Contract(r.contract.clone())),
                )
                .collect::<BTreeSet<_>>(),
        );
    }
    for (id, deps) in &manifest.dependencies {
        graph.insert(
            DependencyKey::Contract(id.clone()),
            deps.iter().cloned().map(DependencyKey::Contract).collect(),
        );
    }
    let edges = graph.values().map(BTreeSet::len).sum::<usize>();
    if edges > 1024 || graph.values().flatten().any(|dep| !graph.contains_key(dep)) {
        return Err(INVALID.into());
    }
    let mut pending = manifest
        .roots
        .iter()
        .cloned()
        .map(DependencyKey::Component)
        .chain(
            manifest
                .contract_roots
                .iter()
                .cloned()
                .map(DependencyKey::Contract),
        )
        .collect::<BTreeSet<_>>();
    let mut reachable = BTreeSet::new();
    while let Some(node) = pending.pop_first() {
        work.check()?;
        let deps = graph.get(&node).ok_or(INVALID)?;
        if reachable.insert(node) {
            pending.extend(deps.iter().cloned());
        }
    }
    if reachable != graph.keys().cloned().collect() {
        return Err(INVALID.into());
    }
    let mut order = vec![];
    let mut depths = BTreeMap::new();
    while !graph.is_empty() {
        work.check()?;
        let Some(node) = graph
            .iter()
            .find(|(_, deps)| deps.is_empty())
            .map(|(node, _)| node.clone())
        else {
            return Err(INVALID.into());
        };
        let depth = *depths.entry(node.clone()).or_insert(0usize);
        if depth > 32 {
            return Err(INVALID.into());
        }
        graph.remove(&node);
        for (parent, deps) in &mut graph {
            work.check()?;
            if deps.remove(&node) {
                let next = depths.entry(parent.clone()).or_insert(0);
                *next = (*next).max(depth + 1);
            }
        }
        order.push(node);
    }
    Ok(order)
}
pub(super) fn receivers(
    manifest: &Manifest,
    contracts: &BTreeMap<String, VerifiedContract>,
    work: &Work<'_>,
) -> Result<(), String> {
    for receiver in manifest
        .components
        .values()
        .flat_map(|c| c.receivers.values())
    {
        work.check()?;
        let Some(VerifiedContract::Structural(contract)) = contracts.get(&receiver.contract) else {
            return Err(INVALID.into());
        };
        if contract.pin().schema_uri != receiver.schema_uri {
            return Err(INVALID.into());
        }
    }
    Ok(())
}
