//! Actual-host selection inspection and private primitive datatype compilation.
//! No portable permission or receiver admission. Imported rules require explicit owners.
use super::*;
use crate::{
    datatype_dependency_capsule::VerifiedDatatypeDependencies,
    datatype_names::{
        DatatypeExport, DatatypeNameCatalog, DatatypeNameLimits, DatatypeSchemaSource,
    },
};
use cem_ml::schema::{
    attribute_datatypes::AttributeDatatypeContextLease,
    datatype_registry::{traverse_native_datatype_dependencies, DatatypeSource},
    machine::{LexicalScopeSnapshot, LexicallyScopedDocument},
    reference_policy::ReferenceScopePolicyOverrides,
};
mod compilation;
mod host;
mod rules;
pub use compilation::{DatatypeDependencyCompileError, FreshDependencyDatatypeCompilation};
pub use rules::{DatatypeDependencyRuleRegistration, FreshDatatypeRuleBinding};
const REFUSED: &str = "fresh datatype dependency selection refused";
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DatatypeDependencyImport {
    pub consumer: DeclarationScope,
    pub module: String,
    pub root: AstNodeId,
}
/// An inspection of a completed selection, not permission to compile or publish.
/// Context liveness does not freeze mutable native callback/provider state.
pub struct FreshDatatypeDependencySelection {
    capsule: Arc<VerifiedDatatypeDependencies>,
    context: AttributeDatatypeContextLease,
    active: AtomicBool,
    sites: usize,
    datatypes: usize,
}
impl FreshDatatypeDependencySelection {
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        host.check().map_err(|_| REFUSED)?;
        if !self.active.load(Ordering::Acquire)
            || !self.context.matches(&host.context)
            || self.capsule.pin().registry_hash != host.registry_hash
        {
            return Err(REFUSED.into());
        }
        Ok(())
    }
    pub fn is_live(&self, host: &FreshStructuralHost) -> bool {
        self.check(host).is_ok()
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release)
    }
    pub fn site_count(&self, host: &FreshStructuralHost) -> Result<usize, String> {
        self.check(host)?;
        Ok(self.sites)
    }
    pub fn datatype_count(&self, host: &FreshStructuralHost) -> Result<usize, String> {
        self.check(host)?;
        Ok(self.datatypes)
    }
}
fn view<'a>(
    capsule: &'a VerifiedDatatypeDependencies,
    id: Option<&str>,
) -> Result<
    (
        &'a Arc<RetainedCemTree>,
        &'a Arc<LexicallyScopedDocument>,
        SchemaDeclarationNode,
    ),
    String,
> {
    Ok(match id {
        None => (
            capsule.source(),
            capsule.capture(),
            capsule
                .sources()
                .values()
                .next()
                .ok_or(REFUSED)?
                .scope()
                .clone(),
        ),
        Some(id) => (
            capsule.dependency_source(id).ok_or(REFUSED)?,
            capsule.dependency_capture(id).ok_or(REFUSED)?,
            capsule.dependency_schema(id).ok_or(REFUSED)?,
        ),
    })
}
fn literal(
    capture: &LexicallyScopedDocument,
    node: &SchemaDeclarationNode,
    name: &str,
) -> Result<String, String> {
    let CemAstNode::Element { attributes, .. } = node.node() else {
        return Err(REFUSED.into());
    };
    let mut value = None;
    for id in attributes {
        if capture.namespace_binding(node.document(), *id).is_some() {
            continue;
        }
        let Some(expanded) = capture.expanded_name(node.document(), *id) else {
            return Err(REFUSED.into());
        };
        if expanded.local_name != name {
            continue;
        }
        if !matches!(
            expanded.namespace_uri.as_str(),
            "" | cem_ml::schema::registry::CEM_SCHEMA_URI
        ) || value.is_some()
        {
            return Err(REFUSED.into());
        }
        let Some(CemAstNode::Attribute {
            value: Some(v),
            value_nodes,
            ..
        }) = node.document().get(*id)
        else {
            return Err(REFUSED.into());
        };
        if !value_nodes.is_empty() || v.trim().is_empty() {
            return Err(REFUSED.into());
        }
        value = Some(v.trim().to_owned());
    }
    value.ok_or_else(|| REFUSED.into())
}
impl FreshStructuralHost {
    /// Every runtime context/readiness and policy is explicitly supplied. Source
    /// metadata is attached to its exact original arena, without a crossing grant.
    pub fn register_dependency_scope(
        &mut self,
        capsule: &VerifiedDatatypeDependencies,
        id: Option<&str>,
        context: Option<StandaloneExpressionContext>,
        policy: ReferenceScopePolicy,
    ) -> Result<DeclarationScope, String> {
        self.check().map_err(|_| REFUSED)?;
        if capsule.pin().registry_hash != self.registry_hash {
            return Err(REFUSED.into());
        }
        let (tree, capture, _) = view(capsule, id)?;
        let scope = self.register_scope(tree.clone(), context, policy)?;
        self.inner
            .attach_captured_names(capture)
            .map_err(|_| REFUSED)?;
        self.inner
            .attach_captured_namespaces(capture.clone())
            .map_err(|_| REFUSED)?;
        if !self.inner.has_datatype_source_capture(capture) {
            return Err(REFUSED.into());
        }
        self.check().map_err(|_| REFUSED)?;
        Ok(scope)
    }
    /// Prepare original expression occurrences explicitly; None contexts remain
    /// pending. Saved lexical metadata is passed unchanged to the consumer.
    pub fn prepare_dependency_lexical_scopes<F>(
        &mut self,
        capsule: &VerifiedDatatypeDependencies,
        id: Option<&str>,
        prepare: F,
    ) -> Result<Vec<(AstNodeId, DeclarationScope)>, String>
    where
        F: FnMut(
            &SchemaDeclarationNode,
            &LexicalScopeSnapshot,
            DeclarationScope,
        ) -> (
            Option<StandaloneExpressionContext>,
            ReferenceScopePolicyOverrides,
        ),
    {
        self.check().map_err(|_| REFUSED)?;
        if capsule.pin().registry_hash != self.registry_hash {
            return Err(REFUSED.into());
        }
        let (_, capture, _) = view(capsule, id)?;
        if !self.inner.has_datatype_source_capture(capture) {
            return Err(REFUSED.into());
        }
        let result = self
            .inner
            .attach_captured_lexical_scopes_with_policy_overrides(capture, prepare)
            .map_err(|_| REFUSED.to_owned());
        if result.is_ok() {
            self.context.advance();
        }
        self.check().map_err(|_| REFUSED)?;
        result
    }
    pub fn select_datatype_dependencies(
        &mut self,
        capsule: Arc<VerifiedDatatypeDependencies>,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        limits: ReferenceTraversalLimits,
    ) -> Result<FreshDatatypeDependencySelection, String> {
        let Environment {
            context,
            catalog,
            allowed,
            expected,
            captures,
        } = self.dependency_environment(&capsule, root, scopes, imports, limits)?;
        let mut adapter = host::Host {
            inner: &mut self.inner,
            catalog: &catalog,
            allowed: &allowed,
            expected: &expected,
            captures: &captures,
            lease: &context,
            closed: &self.closed,
            control: &self.control,
        };
        let mut remaining = limits;
        let mut selected = BTreeMap::new();
        let mut datatypes = BTreeSet::new();
        for root in capsule.roots() {
            adapter.check().map_err(|_| REFUSED)?;
            let source = capsule.sources().get(root).ok_or(REFUSED)?.clone();
            let walk = traverse_native_datatype_dependencies(source, &mut adapter, remaining)
                .map_err(|_| REFUSED)?;
            if !walk.is_complete() {
                return Err(REFUSED.into());
            }
            for node in &walk.walk.resolution.nodes {
                if let cem_ml::schema::datatype_registry::DatatypeTraversalNode::Value(value, _) =
                    node
                {
                    if !adapter.ready(value) {
                        return Err(REFUSED.into());
                    }
                }
            }
            remaining.max_work = remaining
                .max_work
                .checked_sub(walk.walk.resolution.work_used)
                .ok_or(REFUSED)?;
            for plan in &walk.plans {
                datatypes.insert(plan.source.declaration().identity());
            }
            for site in &walk.sites {
                let [target] = site.targets.as_slice() else {
                    return Err(REFUSED.into());
                };
                let key = site.attribute.identity();
                let target = target.identity();
                if expected.get(&key) != Some(&target)
                    || selected
                        .insert(key, target.clone())
                        .is_some_and(|previous| previous != target)
                {
                    return Err(REFUSED.into());
                }
            }
        }
        adapter.check().map_err(|_| REFUSED)?;
        if capsule.sites().iter().any(|s| {
            SchemaDeclarationNode::new(capsule.capture().document().clone(), s.field)
                .is_none_or(|n| !selected.contains_key(&n.identity()))
        }) {
            return Err(REFUSED.into());
        }
        let result = FreshDatatypeDependencySelection {
            capsule,
            context,
            active: AtomicBool::new(true),
            sites: selected.len(),
            datatypes: datatypes.len(),
        };
        result.check(self)?;
        Ok(result)
    }
}

struct Environment {
    context: AttributeDatatypeContextLease,
    catalog: DatatypeNameCatalog,
    allowed: BTreeMap<String, DatatypeSource>,
    expected: BTreeMap<String, String>,
    captures: Vec<Arc<LexicallyScopedDocument>>,
}
impl FreshStructuralHost {
    fn dependency_environment(
        &mut self,
        capsule: &VerifiedDatatypeDependencies,
        root: DeclarationScope,
        scopes: &BTreeMap<String, DeclarationScope>,
        imports: &[DatatypeDependencyImport],
        limits: ReferenceTraversalLimits,
    ) -> Result<Environment, String> {
        self.check().map_err(|_| REFUSED)?;
        if capsule.pin().registry_hash != self.registry_hash
            || limits.max_work == 0
            || limits.max_work > 100_000
            || limits.max_depth == 0
            || limits.max_depth > 64
            || imports.len() > 128
            || scopes.len() != capsule.dependency_ids().count()
        {
            return Err(REFUSED.into());
        }
        let context = self.context.lease();
        let mut inputs = vec![];
        let mut registered = BTreeMap::new();
        let mut captures = vec![];
        for id in std::iter::once(None).chain(capsule.dependency_ids().map(Some)) {
            let (tree, capture, schema) = view(capsule, id)?;
            let scope = match id {
                None => root,
                Some(id) => *scopes.get(id).ok_or(REFUSED)?,
            };
            if !self.inner.has_datatype_source_capture(capture)
                || self
                    .inner
                    .registered_validator_context(scope, tree)
                    .is_none()
                || self
                    .inner
                    .scope(&self.inner.source_reference(schema.clone()))
                    != Some(scope)
            {
                return Err(REFUSED.into());
            }
            registered.insert(scope, inputs.len());
            captures.push(capture.clone());
            inputs.push(DatatypeSchemaSource {
                schema,
                captured: capture.clone(),
                imports: vec![],
            });
        }
        let mut seen = BTreeSet::new();
        for request in imports {
            if !seen.insert(request.clone()) {
                return Err(REFUSED.into());
            }
            let index = *registered.get(&request.consumer).ok_or(REFUSED)?;
            let module = capsule.datatype_module(&request.module).ok_or(REFUSED)?;
            if !module.roots().contains(&request.root) {
                return Err(REFUSED.into());
            }
            let source = module.sources().get(&request.root).ok_or(REFUSED)?;
            let namespace = literal(module.capture(), source.scope(), "namespace")?;
            let name = literal(module.capture(), source.declaration(), "name")?;
            inputs[index].imports.push(DatatypeExport {
                namespace,
                name,
                source: source.clone(),
            });
        }
        let catalog = DatatypeNameCatalog::discover_with_limits(
            &inputs,
            &mut self.inner,
            DatatypeNameLimits {
                max_work: limits.max_work,
                ..Default::default()
            },
            limits,
        )
        .map_err(|_| REFUSED)?;
        let mut allowed = BTreeMap::new();
        let mut expected = BTreeMap::new();
        for source in capsule.sources().values().chain(
            capsule
                .datatype_modules()
                .flat_map(|m| m.sources().values()),
        ) {
            allowed.insert(source.declaration().identity(), source.clone());
        }
        for site in capsule.sites() {
            let field =
                SchemaDeclarationNode::new(capsule.capture().document().clone(), site.field)
                    .ok_or(REFUSED)?;
            expected.insert(
                field.identity(),
                capsule
                    .expected_target(&site.target)
                    .ok_or(REFUSED)?
                    .identity(),
            );
        }
        for module in capsule.datatype_modules() {
            for source in module.sources().values() {
                for dep in source.plan().dependencies {
                    let cem_ml::schema::datatype_registry::DatatypeDependencyValue::Literal(name) =
                        &dep.value
                    else {
                        return Err(REFUSED.into());
                    };
                    let target = module.source_named(name).ok_or(REFUSED)?;
                    expected.insert(dep.attribute.identity(), target.declaration().identity());
                }
            }
        }
        self.check().map_err(|_| REFUSED)?;
        if !context.matches(&self.context) {
            return Err(REFUSED.into());
        }
        Ok(Environment {
            context,
            catalog,
            allowed,
            expected,
            captures,
        })
    }
}
