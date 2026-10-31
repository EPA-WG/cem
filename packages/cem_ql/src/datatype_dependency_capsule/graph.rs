use super::*;
use cem_ml::{
    parser::CemAstNode,
    schema::{
        datatype_registry::{
            DatatypeDependencyRole as Role, DatatypeDependencyValue as Value, DatatypeKind,
            DatatypeKindSource,
        },
        registry::CEM_SCHEMA_URI,
    },
};
fn name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
pub(crate) enum ResolvedTarget {
    Local(AstNodeId),
    Datatype(DatatypeSource),
    Rule,
}
pub(crate) fn resolve_leaf_target(
    target: &DependencyTarget,
    artifacts: &BTreeMap<String, VerifiedArtifact>,
) -> Result<ResolvedTarget, String> {
    Ok(match target {
        DependencyTarget::LocalDatatype(id) => ResolvedTarget::Local(*id),
        DependencyTarget::Datatype { pin, root } => {
            let Some(VerifiedArtifact::Datatype(actual)) = artifacts.get(&pin.hash) else {
                return Err(INVALID.into());
            };
            if encode(pin)? != encode(actual.pin())? || !actual.roots().contains(root) {
                return Err(INVALID.into());
            }
            ResolvedTarget::Datatype(actual.sources().get(root).ok_or(INVALID)?.clone())
        }
        DependencyTarget::Query(pin) => {
            let Some(VerifiedArtifact::Query(actual)) = artifacts.get(&pin.hash) else {
                return Err(INVALID.into());
            };
            if encode(pin)? != encode(actual.pin())? {
                return Err(INVALID.into());
            }
            ResolvedTarget::Rule
        }
        DependencyTarget::Native(pin) => {
            let Some(VerifiedArtifact::Native(actual)) = artifacts.get(&pin.hash) else {
                return Err(INVALID.into());
            };
            if encode(pin)? != encode(actual.pin())?
                || !matches!(&actual.requirement, native::Requirement::Validator(_))
            {
                return Err(INVALID.into());
            }
            ResolvedTarget::Rule
        }
    })
}
pub(crate) fn catalog(
    capture: &Arc<LexicallyScopedDocument>,
    work: &Work<'_>,
) -> Result<BTreeMap<AstNodeId, DatatypeSource>, String> {
    let mut schemas = vec![];
    for node in &capture.document().nodes {
        work.check()?;
        if let CemAstNode::Element { node_id, .. } = node {
            if capture
                .expanded_name(capture.document(), *node_id)
                .is_some_and(|n| n.namespace_uri == CEM_SCHEMA_URI && n.local_name == "schema")
            {
                schemas.push(*node_id);
            }
        }
    }
    let [scope] = schemas.as_slice() else {
        return Err(INVALID.into());
    };
    let mut catalog = BTreeMap::new();
    let mut names = BTreeSet::new();
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
        let source = native::datatype(capture, *scope, *node_id)?;
        let Some(CemAstNode::Attribute {
            value: Some(n),
            value_nodes,
            ..
        }) = source.attribute("name").map(|a| a.node())
        else {
            return Err(INVALID.into());
        };
        if !value_nodes.is_empty() || !name(n) || !names.insert(n.clone()) {
            return Err(INVALID.into());
        }
        catalog.insert(*node_id, source);
    }
    if catalog.len() > 64 {
        return Err(INVALID.into());
    }
    Ok(catalog)
}
pub(super) fn inspect(
    capture: &Arc<LexicallyScopedDocument>,
    roots: &BTreeSet<AstNodeId>,
    bindings: &BTreeMap<AstNodeId, DependencyTarget>,
    artifacts: &BTreeMap<String, VerifiedArtifact>,
    work: &Work<'_>,
) -> Result<
    (
        BTreeMap<AstNodeId, DatatypeSource>,
        Vec<DependencySite>,
        Vec<AstNodeId>,
    ),
    String,
> {
    let catalog = catalog(capture, work)?;
    inspect_catalog(
        roots,
        bindings,
        &catalog,
        |target| resolve_leaf_target(target, artifacts),
        work,
    )
}
pub(crate) fn inspect_catalog<T: Clone>(
    roots: &BTreeSet<AstNodeId>,
    bindings: &BTreeMap<AstNodeId, T>,
    catalog: &BTreeMap<AstNodeId, DatatypeSource>,
    resolve: impl FnMut(&T) -> Result<ResolvedTarget, String>,
    work: &Work<'_>,
) -> Result<
    (
        BTreeMap<AstNodeId, DatatypeSource>,
        Vec<DependencySite<T>>,
        Vec<AstNodeId>,
    ),
    String,
> {
    inspect_catalog_inner(roots, bindings, catalog, resolve, work, false)
}
pub(crate) fn inspect_catalog_with_constants<T: Clone>(
    roots: &BTreeSet<AstNodeId>,
    bindings: &BTreeMap<AstNodeId, T>,
    catalog: &BTreeMap<AstNodeId, DatatypeSource>,
    resolve: impl FnMut(&T) -> Result<ResolvedTarget, String>,
    work: &Work<'_>,
) -> Result<
    (
        BTreeMap<AstNodeId, DatatypeSource>,
        Vec<DependencySite<T>>,
        Vec<AstNodeId>,
    ),
    String,
> {
    inspect_catalog_inner(roots, bindings, catalog, resolve, work, true)
}
fn inspect_catalog_inner<T: Clone>(
    roots: &BTreeSet<AstNodeId>,
    bindings: &BTreeMap<AstNodeId, T>,
    catalog: &BTreeMap<AstNodeId, DatatypeSource>,
    mut resolve: impl FnMut(&T) -> Result<ResolvedTarget, String>,
    work: &Work<'_>,
    constants: bool,
) -> Result<
    (
        BTreeMap<AstNodeId, DatatypeSource>,
        Vec<DependencySite<T>>,
        Vec<AstNodeId>,
    ),
    String,
> {
    if roots.is_empty() || roots.len() > 32 {
        return Err(INVALID.into());
    }
    let mut pending = roots.clone();
    let mut sources = BTreeMap::new();
    let mut sites = vec![];
    let mut used = BTreeSet::new();
    let mut edges = BTreeMap::<u32, BTreeSet<u32>>::new();
    while let Some(id) = pending.pop_first() {
        work.check()?;
        if sources.contains_key(&id) {
            continue;
        }
        let source = catalog.get(&id).ok_or(INVALID)?;
        let plan = source.plan();
        if !plan.issues.is_empty()
            || (!constants && !plan.constant_slots.is_empty())
            || !matches!(
                plan.kind,
                DatatypeKindSource::Inherited
                    | DatatypeKindSource::Explicit(
                        DatatypeKind::Scalar | DatatypeKind::Lexical | DatatypeKind::List
                    )
            )
        {
            return Err(INVALID.into());
        }
        edges.entry(id).or_default();
        for dep in &plan.dependencies {
            work.check()?;
            let field = dep.attribute.node_id();
            let target = bindings.get(&field).ok_or(INVALID)?;
            if !used.insert(field) || sites.len() >= 128 {
                return Err(INVALID.into());
            }
            match resolve(target)? {
                ResolvedTarget::Local(selected) => {
                    if dep.role == Role::ValidationRule {
                        return Err(INVALID.into());
                    }
                    let actual = catalog.get(&selected).ok_or(INVALID)?;
                    if let Value::Literal(n) = &dep.value {
                        if !matches!(actual.attribute("name").map(|a|a.node()),Some(CemAstNode::Attribute{value:Some(actual),..}) if actual==n)
                        {
                            return Err(INVALID.into());
                        }
                    }
                    pending.insert(selected);
                    edges.entry(id).or_default().insert(selected);
                }
                ResolvedTarget::Datatype(actual) => {
                    if dep.role == Role::ValidationRule {
                        return Err(INVALID.into());
                    }
                    if let Value::Literal(n) = &dep.value {
                        let Some((prefix, local)) = n.split_once(':') else {
                            return Err(INVALID.into());
                        };
                        if !name(prefix)
                            || !name(local)
                            || !matches!(actual.attribute("name").map(|a|a.node()),Some(CemAstNode::Attribute{value:Some(n),..}) if n==local)
                        {
                            return Err(INVALID.into());
                        }
                    }
                }
                ResolvedTarget::Rule => {
                    if dep.role != Role::ValidationRule {
                        return Err(INVALID.into());
                    }
                }
            }
            sites.push(DependencySite {
                datatype: id,
                field,
                reference: match &dep.value {
                    Value::Native(n) => Some(n.node_id()),
                    _ => None,
                },
                role: match dep.role {
                    Role::InheritedBase => "inherited-base",
                    Role::InheritedList => "inherited-list",
                    Role::ListItem => "list-item",
                    Role::ValidationRule => "validation-rule",
                }
                .into(),
                target: target.clone(),
            });
        }
        sources.insert(id, source.clone());
    }
    if used.len() != bindings.len() {
        return Err(INVALID.into());
    }
    let mut remaining = sources.keys().copied().collect::<BTreeSet<_>>();
    let mut depths = BTreeMap::<u32, usize>::new();
    let mut order = vec![];
    while !remaining.is_empty() {
        work.check()?;
        let id = remaining
            .iter()
            .copied()
            .find(|id| edges[id].iter().all(|t| !remaining.contains(t)))
            .ok_or(INVALID)?;
        let depth = edges[&id].iter().map(|t| depths[t] + 1).max().unwrap_or(0);
        if depth > 32 {
            return Err(INVALID.into());
        }
        depths.insert(id, depth);
        remaining.remove(&id);
        order.push(id);
    }
    Ok((sources, sites, order))
}
