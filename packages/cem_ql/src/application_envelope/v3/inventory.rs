//! Exact flat inventories; no executable edges inferred from a registry hash.
use super::*;
pub(super) fn insert<T: Serialize>(
    map: &mut BTreeMap<String, T>,
    id: &str,
    value: T,
) -> Result<(), String> {
    if let Some(old) = map.get(id) {
        if encode(old)? != encode(&value)? {
            return Err(INVALID.into());
        }
    } else {
        map.insert(id.into(), value);
    }
    Ok(())
}
pub(super) fn shape(m: &Manifest) -> Result<(), String> {
    if !valid_hash(&m.registry_hash)
        || m.application.registry_hash != m.registry_hash
        || m.assemblies.len() > MAX_ASSEMBLIES
        || m.leaves.len() > MAX_LEAVES
        || m.profiles.len() > MAX_PROFILES
        || m.operations.len() > MAX_LEAVES
        || m.roots.len() > MAX_ROOTS
        || m.assemblies
            .values()
            .any(|a| a.leaves.len() > MAX_LEAVES || a.profiles.len() > MAX_PROFILES)
        || m.profiles.values().any(|p| p.sources.len() > MAX_LEAVES)
        || m.assemblies
            .values()
            .map(|a| a.leaves.len() + a.profiles.len())
            .sum::<usize>()
            > 256
        || m.profiles.values().map(|p| p.sources.len()).sum::<usize>() > 256
    {
        return Err(INVALID.into());
    }
    Ok(())
}
pub(super) fn check(
    m: &Manifest,
    blobs: &BTreeMap<String, Blob>,
    work: &Work<'_>,
) -> Result<(), String> {
    shape(m)?;
    if m.assemblies.is_empty()
        || m.roots.is_empty()
        || blobs.len() > MAX_BLOBS
        || m.roots.keys().any(|id| {
            id.is_empty()
                || id.len() > 64
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        })
        || m.roots
            .values()
            .map(|r| r.assembly.clone())
            .collect::<BTreeSet<_>>()
            != m.assemblies.keys().cloned().collect()
    {
        return Err(INVALID.into());
    }
    let required = std::iter::once(m.application.hash.clone())
        .chain(m.assemblies.keys().cloned())
        .chain(m.leaves.keys().cloned())
        .chain(m.profiles.keys().cloned())
        .chain(m.operations.keys().cloned())
        .collect::<BTreeSet<_>>();
    if required != blobs.keys().cloned().collect()
        || m.assemblies
            .values()
            .flat_map(|a| a.leaves.iter().cloned())
            .collect::<BTreeSet<_>>()
            != m.leaves.keys().cloned().collect()
        || m.assemblies
            .values()
            .flat_map(|a| a.profiles.iter().cloned())
            .collect::<BTreeSet<_>>()
            != m.profiles.keys().cloned().collect()
        || m.profiles
            .values()
            .flat_map(|p| p.sources.iter().cloned())
            .collect::<BTreeSet<_>>()
            != m.operations.keys().cloned().collect()
    {
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
    for (id, hash, registry) in m
        .assemblies
        .iter()
        .map(|(id, a)| (id, &a.pin.hash, &a.pin.registry_hash))
        .chain(
            m.profiles
                .iter()
                .map(|(id, p)| (id, &p.pin.hash, &p.pin.registry_hash)),
        )
        .chain(
            m.operations
                .iter()
                .map(|(id, p)| (id, &p.hash, &p.registry_hash)),
        )
    {
        work.check()?;
        if id != hash || registry != &m.registry_hash {
            return Err(INVALID.into());
        }
    }
    for (id, pin) in &m.leaves {
        work.check()?;
        if id != pin.hash() || pin.registry() != m.registry_hash {
            return Err(INVALID.into());
        }
    }
    Ok(())
}
pub(super) fn leaves(
    m: &Manifest,
    blobs: &BTreeMap<String, Blob>,
    record: &AssemblyRecord,
) -> Vec<DependencyArtifact> {
    record
        .leaves
        .iter()
        .map(|id| DependencyArtifact {
            bytes: blobs[id].0.clone(),
            pin: m.leaves[id].clone(),
        })
        .collect()
}
pub(super) fn profiles(
    m: &Manifest,
    blobs: &BTreeMap<String, Blob>,
    record: &AssemblyRecord,
) -> Vec<assembly::ProfileArtifact> {
    record
        .profiles
        .iter()
        .map(|id| {
            let p = &m.profiles[id];
            assembly::ProfileArtifact {
                bytes: blobs[id].0.clone(),
                pin: p.pin.clone(),
                sources: p
                    .sources
                    .iter()
                    .map(|id| operations::BuiltOperationSource {
                        bytes: blobs[id].0.clone(),
                        pin: m.operations[id].clone(),
                    })
                    .collect(),
            }
        })
        .collect()
}
pub(super) fn order(m: &Manifest) -> Vec<DependencyKey> {
    std::iter::once(DependencyKey::Application(m.application.hash.clone()))
        .chain(m.leaves.keys().cloned().map(DependencyKey::Leaf))
        .chain(
            m.operations
                .keys()
                .cloned()
                .map(DependencyKey::OperationSource),
        )
        .chain(m.profiles.keys().cloned().map(DependencyKey::Profile))
        .chain(m.assemblies.keys().cloned().map(DependencyKey::Assembly))
        .collect()
}
