//! Bounded flat graph ordering, including actual source-owned cycle witnesses.
use super::*;
enum Failure<K> {
    Refused,
    Depth,
    Cycle(Vec<K>),
}
fn order<K: Ord + Clone>(
    edges: &BTreeMap<K, BTreeSet<K>>,
    work: &Work<'_>,
) -> Result<Vec<K>, Failure<K>> {
    let mut remaining = BTreeMap::new();
    let mut reverse = BTreeMap::<K, BTreeSet<K>>::new();
    for (node, targets) in edges {
        work.check().map_err(|_| Failure::Refused)?;
        remaining.insert(node.clone(), targets.len());
        for target in targets {
            work.check().map_err(|_| Failure::Refused)?;
            if !edges.contains_key(target) {
                return Err(Failure::Refused);
            }
            reverse
                .entry(target.clone())
                .or_default()
                .insert(node.clone());
        }
    }
    let mut ready = remaining
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(k, _)| k.clone())
        .collect::<BTreeSet<_>>();
    let mut depths = BTreeMap::new();
    let mut result = vec![];
    while let Some(node) = ready.pop_first() {
        work.check().map_err(|_| Failure::Refused)?;
        let depth = edges[&node]
            .iter()
            .map(|n| depths[n] + 1usize)
            .max()
            .unwrap_or(0);
        if depth > 32 {
            return Err(Failure::Depth);
        }
        depths.insert(node.clone(), depth);
        result.push(node.clone());
        if let Some(consumers) = reverse.get(&node) {
            for consumer in consumers {
                work.check().map_err(|_| Failure::Refused)?;
                let n = remaining.get_mut(consumer).unwrap();
                *n -= 1;
                if *n == 0 {
                    ready.insert(consumer.clone());
                }
            }
        }
    }
    if result.len() == edges.len() {
        return Ok(result);
    }
    let mut cursor = remaining.iter().find(|(_, n)| **n > 0).unwrap().0.clone();
    let mut visited = BTreeMap::new();
    let mut path = vec![];
    loop {
        work.check().map_err(|_| Failure::Refused)?;
        if let Some(start) = visited.insert(cursor.clone(), path.len()) {
            let mut cycle = path[start..].to_vec();
            cycle.push(cursor);
            return Err(Failure::Cycle(cycle));
        }
        path.push(cursor.clone());
        cursor = edges[&cursor]
            .iter()
            .find(|n| remaining[*n] > 0)
            .unwrap()
            .clone();
    }
}
pub(super) fn inspect(
    root: &str,
    modules: &BTreeMap<String, VerifiedDatatypeModule>,
    leaves: &BTreeMap<String, VerifiedArtifact>,
    work: &Work<'_>,
) -> Result<(Vec<String>, Vec<ModuleDatatypeId>), ModuleVerificationError> {
    let mut edges = BTreeMap::new();
    let mut witnesses = BTreeMap::new();
    for (id, m) in modules {
        let targets = edges.entry(id.clone()).or_insert_with(BTreeSet::new);
        for s in &m.sites {
            work.check()?;
            if let ModuleTarget::ModuleDatatype { module, .. } = &s.target {
                targets.insert(module.clone());
                witnesses
                    .entry((id.clone(), module.clone()))
                    .or_insert_with(|| {
                        SchemaDeclarationNode::new(m.capture.document().clone(), s.field).unwrap()
                    });
            }
        }
    }
    let module_order = match order(&edges, work) {
        Ok(o) => o,
        Err(Failure::Cycle(cycle)) => {
            return Err(ModuleVerificationError {
                code: "datatype-module-cycle",
                related: cycle
                    .windows(2)
                    .filter_map(|w| witnesses.get(&(w[0].clone(), w[1].clone())).cloned())
                    .collect(),
                cycle_modules: cycle,
            })
        }
        Err(Failure::Depth) => {
            return Err(ModuleVerificationError::code("datatype-module-graph-depth"))
        }
        Err(Failure::Refused) => {
            return Err(ModuleVerificationError::code(
                "datatype-module-graph-refused",
            ))
        }
    };
    let mut seen = BTreeSet::new();
    let mut pending = vec![root.to_owned()];
    while let Some(id) = pending.pop() {
        work.check()?;
        if seen.insert(id.clone()) {
            pending.extend(edges[&id].iter().cloned());
        }
    }
    if seen.len() != modules.len() {
        return Err(ModuleVerificationError::code("datatype-module-unreachable"));
    }
    let mut declarations = BTreeMap::<ModuleDatatypeId, BTreeSet<ModuleDatatypeId>>::new();
    for (id, m) in modules {
        for n in m.sources.keys() {
            work.check()?;
            declarations
                .entry(ModuleDatatypeId {
                    module: id.clone(),
                    datatype: *n,
                })
                .or_default();
        }
        for s in &m.sites {
            work.check()?;
            let target = match &s.target {
                ModuleTarget::LocalDatatype(n) => Some(ModuleDatatypeId {
                    module: id.clone(),
                    datatype: *n,
                }),
                ModuleTarget::ModuleDatatype { module, root } => Some(ModuleDatatypeId {
                    module: module.clone(),
                    datatype: *root,
                }),
                ModuleTarget::Datatype { pin, root } => Some(ModuleDatatypeId {
                    module: pin.hash.clone(),
                    datatype: *root,
                }),
                _ => None,
            };
            if let Some(target) = target {
                declarations
                    .entry(ModuleDatatypeId {
                        module: id.clone(),
                        datatype: s.datatype,
                    })
                    .or_default()
                    .insert(target);
            }
        }
    }
    for (id, leaf) in leaves {
        if let VerifiedArtifact::Datatype(m) = leaf {
            for n in m.sources().keys() {
                work.check()?;
                declarations
                    .entry(ModuleDatatypeId {
                        module: id.clone(),
                        datatype: *n,
                    })
                    .or_default();
            }
            for (source, _, target) in m.bindings() {
                work.check()?;
                declarations
                    .entry(ModuleDatatypeId {
                        module: id.clone(),
                        datatype: source.declaration().node_id(),
                    })
                    .or_default()
                    .insert(ModuleDatatypeId {
                        module: id.clone(),
                        datatype: target.declaration().node_id(),
                    });
            }
        }
    }
    let datatype_order = order(&declarations, work).map_err(|f| {
        ModuleVerificationError::code(match f {
            Failure::Depth => "datatype-module-graph-depth",
            Failure::Cycle(_) => "datatype-module-type-cycle",
            Failure::Refused => "datatype-module-graph-refused",
        })
    })?;
    Ok((module_order, datatype_order))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_or_cancelled_work_is_not_misreported_as_a_cycle() {
        for cancel in [false, true] {
            let control = OperationControl::default();
            let work = Work::new(&control);
            if cancel {
                control.cancel_root(Some("fixture".into()), None).unwrap();
            } else {
                for _ in 0..100_000 {
                    work.check().unwrap();
                }
            }
            let edges =
                BTreeMap::from([("a", BTreeSet::from(["b"])), ("b", BTreeSet::from(["a"]))]);
            assert!(matches!(order(&edges, &work), Err(Failure::Refused)));
        }
    }
    #[test]
    fn deterministic_order_rejects_unknown_targets_and_includes_a_closed_cycle_witness() {
        let c = OperationControl::default();
        let work = Work::new(&c);
        let edges = BTreeMap::from([("a", BTreeSet::from(["b"]))]);
        assert!(matches!(order(&edges, &work), Err(Failure::Refused)));
        let edges = BTreeMap::from([
            ("z", BTreeSet::from(["a"])),
            ("a", BTreeSet::from(["b"])),
            ("b", BTreeSet::from(["a"])),
        ]);
        let Err(Failure::Cycle(cycle)) = order(&edges, &work) else {
            panic!("cycle")
        };
        assert_eq!(cycle, vec!["a", "b", "a"]);
    }
}
