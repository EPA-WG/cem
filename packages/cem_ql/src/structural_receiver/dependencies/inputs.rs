//! Borrow original verified inputs across distinct wire formats. This is not a
//! conversion into CEMDP1, a retained selection proof or an executable wire object.
use super::*;
use crate::{
    datatype_dependency_capsule::{DependencyTarget, VerifiedArtifact},
    datatype_module_capsule::{ModuleTarget, VerifiedDatatypeModules},
    native_registrations::{Requirement, VerifiedNativeRequirement},
    query_validator_capsule::declarations::VerifiedQueryDeclaration,
};
pub(super) enum Inputs {
    Dependencies(Arc<VerifiedDatatypeDependencies>),
    Modules(Arc<VerifiedDatatypeModules>),
}
#[derive(Clone, Copy)]
pub(super) enum InputRef<'a> {
    Dependencies(&'a VerifiedDatatypeDependencies),
    Modules(&'a VerifiedDatatypeModules),
}
impl Inputs {
    pub fn borrow(&self) -> InputRef<'_> {
        match self {
            Self::Dependencies(a) => InputRef::Dependencies(a),
            Self::Modules(a) => InputRef::Modules(a),
        }
    }
}
pub(super) struct View<'a> {
    pub tree: &'a Arc<RetainedCemTree>,
    pub capture: &'a Arc<LexicallyScopedDocument>,
    pub schema: SchemaDeclarationNode,
    pub datatypes: Option<&'a BTreeMap<AstNodeId, DatatypeSource>>,
    pub exports: Option<&'a BTreeSet<AstNodeId>>,
}
impl<'a> View<'a> {
    fn datatype(
        tree: &'a Arc<RetainedCemTree>,
        capture: &'a Arc<LexicallyScopedDocument>,
        sources: &'a BTreeMap<AstNodeId, DatatypeSource>,
        roots: &'a BTreeSet<AstNodeId>,
    ) -> Result<Self, String> {
        Ok(Self {
            tree,
            capture,
            schema: sources.values().next().ok_or(REFUSED)?.scope().clone(),
            datatypes: Some(sources),
            exports: Some(roots),
        })
    }
    fn leaf(a: &'a VerifiedArtifact) -> Result<Self, String> {
        let schema = match a {
            VerifiedArtifact::Datatype(a) => {
                a.sources().values().next().ok_or(REFUSED)?.scope().clone()
            }
            VerifiedArtifact::Query(a) => a.contract.owner().clone(),
            VerifiedArtifact::Native(a) => {
                let Requirement::Validator(c) = &a.requirement else {
                    return Err(REFUSED.into());
                };
                c.owner().clone()
            }
        };
        let (datatypes, exports) = match a {
            VerifiedArtifact::Datatype(a) => (Some(a.sources()), Some(a.roots())),
            _ => (None, None),
        };
        Ok(Self {
            tree: a.tree(),
            capture: a.capture(),
            schema,
            datatypes,
            exports,
        })
    }
}
impl<'a> InputRef<'a> {
    pub fn registry(self) -> &'a str {
        match self {
            Self::Dependencies(a) => &a.pin().registry_hash,
            Self::Modules(a) => &a.pin().registry_hash,
        }
    }
    pub fn ids(self) -> Vec<&'a str> {
        match self {
            Self::Dependencies(a) => a.dependency_ids().collect(),
            Self::Modules(a) => a
                .module_ids()
                .filter(|id| *id != a.pin().root)
                .chain(a.leaf_ids())
                .collect(),
        }
    }
    pub fn view(self, id: Option<&str>) -> Result<View<'a>, String> {
        match self {
            Self::Dependencies(a) => match id {
                None => View::datatype(a.source(), a.capture(), a.sources(), a.roots()),
                Some(id) => Ok(View {
                    tree: a.dependency_source(id).ok_or(REFUSED)?,
                    capture: a.dependency_capture(id).ok_or(REFUSED)?,
                    schema: a.dependency_schema(id).ok_or(REFUSED)?,
                    datatypes: a.dependency_datatypes(id),
                    exports: a.datatype_module(id).map(|m| m.roots()),
                }),
            },
            Self::Modules(a) => {
                let id = id.unwrap_or(&a.pin().root);
                if let Some(m) = a.module(id) {
                    View::datatype(m.source(), m.capture(), m.sources(), m.roots())
                } else {
                    View::leaf(a.leaf(id).ok_or(REFUSED)?)
                }
            }
        }
    }
    pub fn query(self, id: &str) -> Option<&'a VerifiedQueryDeclaration> {
        match self {
            Self::Dependencies(a) => a.query_module(id),
            Self::Modules(a) => match a.leaf(id)? {
                VerifiedArtifact::Query(q) => Some(q),
                _ => None,
            },
        }
    }
    pub fn native(self, id: &str) -> Option<&'a VerifiedNativeRequirement> {
        match self {
            Self::Dependencies(a) => a.native_module(id),
            Self::Modules(a) => match a.leaf(id)? {
                VerifiedArtifact::Native(n) => Some(n),
                _ => None,
            },
        }
    }
    pub fn rule_behavior(self, id: &str) -> Option<SchemaDeclarationNode> {
        if let Some(q) = self.query(id) {
            return Some(q.contract.behavior().clone());
        }
        let Requirement::Validator(c) = &self.native(id)?.requirement else {
            return None;
        };
        Some(c.behavior().clone())
    }
    pub fn root_has_rules(self) -> bool {
        match self {
            Self::Dependencies(a) => a.sites().iter().any(|s| s.role == "validation-rule"),
            Self::Modules(a) => a
                .module(&a.pin().root)
                .is_some_and(|m| m.sites().iter().any(|s| s.role == "validation-rule")),
        }
    }
    fn leaf_target(self, id: &str, root: AstNodeId) -> Result<SchemaDeclarationNode, String> {
        self.view(Some(id))?
            .datatypes
            .ok_or(REFUSED)?
            .get(&root)
            .map(|s| s.declaration().clone())
            .ok_or_else(|| REFUSED.into())
    }
    /// Immutable assertions identify required fields/owners, not permission to
    /// execute them. The actual resolver must reselect each field on every attempt.
    pub fn plan(self, limit: usize, control: &OperationControl) -> Result<Plan, String> {
        let mut remaining = limit;
        let mut spend = || {
            remaining = remaining.checked_sub(1).ok_or(REFUSED)?;
            control
                .check_scope(ROOT_EXECUTION_SCOPE_ID)
                .map_err(|_| REFUSED)?;
            Ok::<(), String>(())
        };
        let mut allowed = BTreeMap::new();
        let mut edges = vec![];
        for id in std::iter::once(None).chain(self.ids().into_iter().map(Some)) {
            spend()?;
            if let Some(types) = self.view(id)?.datatypes {
                for s in types.values() {
                    spend()?;
                    allowed.insert(s.declaration().identity(), s.clone());
                }
            }
        }
        match self {
            Self::Dependencies(a) => {
                for s in a.sites() {
                    spend()?;
                    let field = SchemaDeclarationNode::new(a.capture().document().clone(), s.field)
                        .ok_or(REFUSED)?;
                    let node = a
                        .sources()
                        .get(&s.datatype)
                        .ok_or(REFUSED)?
                        .declaration()
                        .identity();
                    let rule = match &s.target {
                        DependencyTarget::Query(p) => Some(p.hash.clone()),
                        DependencyTarget::Native(p) => Some(p.hash.clone()),
                        _ => None,
                    };
                    edges.push(Edge {
                        node,
                        field,
                        target: a.expected_target(&s.target).ok_or(REFUSED)?,
                        rule,
                    });
                }
                for m in a.datatype_modules() {
                    legacy_edges(m, &mut edges, &mut spend)?;
                }
            }
            Self::Modules(a) => {
                for id in a.module_ids() {
                    let m = a.module(id).ok_or(REFUSED)?;
                    for s in m.sites() {
                        spend()?;
                        let field =
                            SchemaDeclarationNode::new(m.capture().document().clone(), s.field)
                                .ok_or(REFUSED)?;
                        let (target, rule) = match &s.target {
                            ModuleTarget::LocalDatatype(id) => (
                                m.sources().get(id).ok_or(REFUSED)?.declaration().clone(),
                                None,
                            ),
                            ModuleTarget::ModuleDatatype { module, root } => (
                                a.module(module)
                                    .ok_or(REFUSED)?
                                    .sources()
                                    .get(root)
                                    .ok_or(REFUSED)?
                                    .declaration()
                                    .clone(),
                                None,
                            ),
                            ModuleTarget::Datatype { pin, root } => {
                                (self.leaf_target(&pin.hash, *root)?, None)
                            }
                            ModuleTarget::Query(p) => (
                                self.rule_behavior(&p.hash).ok_or(REFUSED)?,
                                Some(p.hash.clone()),
                            ),
                            ModuleTarget::Native(p) => (
                                self.rule_behavior(&p.hash).ok_or(REFUSED)?,
                                Some(p.hash.clone()),
                            ),
                        };
                        edges.push(Edge {
                            node: m
                                .sources()
                                .get(&s.datatype)
                                .ok_or(REFUSED)?
                                .declaration()
                                .identity(),
                            field,
                            target,
                            rule,
                        });
                    }
                }
                for id in a.leaf_ids() {
                    if let Some(VerifiedArtifact::Datatype(m)) = a.leaf(id) {
                        legacy_edges(m, &mut edges, &mut spend)?;
                    }
                }
            }
        }
        let expected = edges
            .iter()
            .map(|s| (s.field.identity(), s.target.identity()))
            .collect::<BTreeMap<_, _>>();
        if expected.len() != edges.len() {
            return Err(REFUSED.into());
        }
        let mut index = BTreeMap::<String, Vec<&Edge>>::new();
        for s in &edges {
            spend()?;
            index.entry(s.node.clone()).or_default().push(s);
        }
        let root = self.view(None)?;
        let types = root.datatypes.ok_or(REFUSED)?;
        let mut pending = root
            .exports
            .ok_or(REFUSED)?
            .iter()
            .map(|id| types[id].declaration().identity())
            .collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        let mut required = BTreeSet::new();
        let mut rules = BTreeSet::new();
        while let Some(node) = pending.pop() {
            spend()?;
            if !seen.insert(node.clone()) {
                continue;
            }
            if !allowed.contains_key(&node) {
                return Err(REFUSED.into());
            }
            if let Some(sites) = index.get(&node) {
                for s in sites {
                    spend()?;
                    required.insert(s.field.identity());
                    if let Some(rule) = &s.rule {
                        rules.insert(rule.clone());
                    } else {
                        pending.push(s.target.identity());
                    }
                }
            }
        }
        Ok(Plan {
            allowed,
            expected,
            required,
            rules,
        })
    }
}
struct Edge {
    node: String,
    field: SchemaDeclarationNode,
    target: SchemaDeclarationNode,
    rule: Option<String>,
}
fn legacy_edges(
    m: &crate::datatype_source_capsule::VerifiedDatatypeSources,
    edges: &mut Vec<Edge>,
    spend: &mut impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    for source in m.sources().values() {
        spend()?;
        for dep in source.plan().dependencies {
            spend()?;
            let cem_ml::schema::datatype_registry::DatatypeDependencyValue::Literal(name) =
                &dep.value
            else {
                return Err(REFUSED.into());
            };
            let target = m.source_named(name).ok_or(REFUSED)?;
            edges.push(Edge {
                node: source.declaration().identity(),
                field: dep.attribute,
                target: target.declaration().clone(),
                rule: None,
            });
        }
    }
    Ok(())
}
pub(super) struct Plan {
    pub allowed: BTreeMap<String, DatatypeSource>,
    pub expected: BTreeMap<String, String>,
    pub required: BTreeSet<String>,
    pub rules: BTreeSet<String>,
}
