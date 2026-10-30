use super::*;
use cem_ml::{
    parser::CemAstNode,
    schema::{
        datatype_registry::{
            DatatypeDependencyRole, DatatypeDependencyValue, DatatypeKind, DatatypeKindSource,
        },
        registry::CEM_SCHEMA_URI,
    },
};
fn local_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
pub(super) fn inspect(
    capture: &LexicallyScopedDocument,
    roots: &BTreeSet<AstNodeId>,
    work: &Work<'_>,
) -> Result<
    (
        BTreeMap<AstNodeId, TypeRecord>,
        BTreeMap<AstNodeId, DatatypeSource>,
        Vec<AstNodeId>,
    ),
    String,
> {
    if roots.is_empty() || roots.len() > 32 || capture.document().nodes.len() > 16_384 {
        return Err(INVALID.into());
    }
    let mut schemas = vec![];
    for node in &capture.document().nodes {
        work.check()?;
        if let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        {
            if expanded_name.local_name == "$" {
                return Err(INVALID.into());
            }
            if capture
                .expanded_name(capture.document(), *node_id)
                .is_some_and(|n| n.namespace_uri == CEM_SCHEMA_URI && n.local_name == "schema")
            {
                schemas.push(*node_id)
            }
        }
    }
    let [scope] = schemas.as_slice() else {
        return Err(INVALID.into());
    };
    let mut catalog = BTreeMap::new();
    for node in &capture.document().nodes {
        work.check()?;
        let CemAstNode::Element { node_id, .. } = node else {
            continue;
        };
        if !capture
            .expanded_name(capture.document(), *node_id)
            .is_some_and(|n| n.namespace_uri == CEM_SCHEMA_URI && n.local_name == "type")
        {
            continue;
        }
        let source = crate::native_registrations::datatype(capture, *scope, *node_id)?;
        let Some(CemAstNode::Attribute {
            value: Some(name),
            value_nodes,
            ..
        }) = source.attribute("name").map(|a| a.node())
        else {
            return Err(INVALID.into());
        };
        if !value_nodes.is_empty()
            || !local_name(name)
            || catalog.insert(name.clone(), source).is_some()
        {
            return Err(INVALID.into());
        }
    }
    if catalog.len() > 64 {
        return Err(INVALID.into());
    }
    let by_id = catalog
        .values()
        .map(|s| (s.declaration().node_id(), s.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut sources = BTreeMap::new();
    let mut types = BTreeMap::new();
    let mut pending = roots.clone();
    while let Some(id) = pending.pop_first() {
        work.check()?;
        if types.contains_key(&id) {
            continue;
        }
        let source = by_id.get(&id).ok_or(INVALID)?;
        let plan = source.plan();
        if !plan.issues.is_empty() || !plan.constant_slots.is_empty() {
            return Err(INVALID.into());
        }
        let kind = match plan.kind {
            DatatypeKindSource::Explicit(DatatypeKind::Scalar) => "scalar",
            DatatypeKindSource::Explicit(DatatypeKind::Lexical) => "lexical",
            DatatypeKindSource::Explicit(DatatypeKind::List) => "list",
            DatatypeKindSource::Inherited => "inherited",
            _ => return Err(INVALID.into()),
        };
        let mut dependencies = vec![];
        for dependency in &plan.dependencies {
            work.check()?;
            let DatatypeDependencyValue::Literal(name) = &dependency.value else {
                return Err(INVALID.into());
            };
            if !local_name(name) {
                return Err(INVALID.into());
            }
            let target = catalog.get(name).ok_or(INVALID)?.declaration().node_id();
            let role = match dependency.role {
                DatatypeDependencyRole::InheritedBase => "inherited-base",
                DatatypeDependencyRole::InheritedList => "inherited-list",
                DatatypeDependencyRole::ListItem => "list-item",
                _ => return Err(INVALID.into()),
            };
            dependencies.push(Dependency {
                field: dependency.attribute.node_id(),
                role: role.into(),
                target,
            });
            pending.insert(target);
        }
        let name = catalog
            .iter()
            .find(|(_, s)| s.declaration().node_id() == id)
            .ok_or(INVALID)?
            .0
            .clone();
        types.insert(
            id,
            TypeRecord {
                name,
                kind: kind.into(),
                dependencies,
            },
        );
        sources.insert(id, source.clone());
    }
    let mut remaining = types.keys().copied().collect::<BTreeSet<_>>();
    let mut depths = BTreeMap::new();
    let mut order = vec![];
    while !remaining.is_empty() {
        work.check()?;
        let id = remaining
            .iter()
            .copied()
            .find(|id| {
                types[id]
                    .dependencies
                    .iter()
                    .all(|d| !remaining.contains(&d.target))
            })
            .ok_or(INVALID)?;
        let depth = types[&id]
            .dependencies
            .iter()
            .map(|d| depths[&d.target] + 1usize)
            .max()
            .unwrap_or(0);
        if depth > 32 {
            return Err(INVALID.into());
        }
        depths.insert(id, depth);
        remaining.remove(&id);
        order.push(id);
    }
    Ok((types, sources, order))
}
