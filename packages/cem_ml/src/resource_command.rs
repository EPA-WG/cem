//! Native resource-backed commands. The host supplies the admitted logical route, ordered
//! from content to app. An invocation ID identifies work only inside its issuing coordinator.
use crate::parser::{document::CemDocument, tree::RetainedCemTree, AstNodeId, CemAstNode};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

pub type Parameters = BTreeMap<String, ParameterValue>;
pub enum ParameterValue {
    Scalar(Value),
    Native(Arc<RetainedCemTree>),
}

#[derive(Debug, PartialEq, Eq)]
struct Field {
    kind: String,
    required: bool,
    repeated: bool,
    private: bool,
    choices: BTreeSet<String>,
    min_length: usize,
}
#[derive(Debug, PartialEq, Eq)]
struct Definition {
    revision: u64,
    fields: BTreeMap<String, Field>,
}

fn attr<'a>(doc: &'a CemDocument, node: &CemAstNode, name: &str) -> Option<&'a str> {
    let CemAstNode::Element { attributes, .. } = node else {
        return None;
    };
    attributes.iter().find_map(|id| match doc.get(*id)? {
        CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        } if expanded_name.local_name == name && expanded_name.namespace_uri.is_empty() => {
            value.as_deref()
        }
        _ => None,
    })
}

fn children<'a>(
    doc: &'a CemDocument,
    parent: &'a CemAstNode,
    name: &'a str,
) -> impl Iterator<Item = &'a CemAstNode> {
    let ids = match parent {
        CemAstNode::Element { children, .. } => children.as_slice(),
        _ => &[],
    };
    ids.iter()
        .filter_map(|id| doc.get(*id))
        .filter(move |node| element_named(node, name))
}

fn element_named(node: &CemAstNode, name: &str) -> bool {
    matches!(node, CemAstNode::Element { expanded_name, .. }
        if expanded_name.local_name == name && expanded_name.namespace_uri.is_empty())
}

fn attribute_flag(doc: &CemDocument, node: &CemAstNode, name: &str) -> Result<bool, &'static str> {
    match attr(doc, node, name) {
        Some("true") => Ok(true),
        None | Some("false") => Ok(false),
        _ => Err("invalid boolean"),
    }
}

#[derive(Default, Debug)]
pub struct CommandCatalog(BTreeMap<String, Arc<Definition>>);

impl CommandCatalog {
    pub fn from_manifest(doc: &CemDocument, manifest: AstNodeId) -> Result<Self, &'static str> {
        let manifest = doc.get(manifest).ok_or("missing manifest")?;
        if !element_named(manifest, "manifest") || attr(doc, manifest, "version") != Some("1") {
            return Err("unsupported manifest");
        }
        let mut commands = BTreeMap::new();
        for command in children(doc, manifest, "command") {
            let id = attr(doc, command, "id")
                .filter(|x| !x.is_empty())
                .ok_or("missing command ID")?;
            let revision = attr(doc, command, "revision")
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|v| *v > 0)
                .ok_or("invalid revision")?;
            let mut schemas = children(doc, command, "parameters");
            let mut fields = BTreeMap::new();
            if let Some(schema) = schemas.next() {
                if schemas.next().is_some() {
                    return Err("duplicate parameter schema");
                }
                for param in children(doc, schema, "parameter") {
                    let name = attr(doc, param, "name")
                        .filter(|x| !x.is_empty())
                        .ok_or("missing parameter name")?;
                    let kind = attr(doc, param, "type").ok_or("missing parameter type")?;
                    if !matches!(
                        kind,
                        "string" | "enum" | "boolean" | "number" | "null" | "native"
                    ) {
                        return Err("unsupported parameter type");
                    }
                    let mut choices = BTreeSet::new();
                    for choice in children(doc, param, "choice") {
                        if !choices.insert(
                            attr(doc, choice, "value")
                                .ok_or("missing enum value")?
                                .to_owned(),
                        ) {
                            return Err("duplicate choice");
                        }
                    }
                    if (kind == "enum") == choices.is_empty() {
                        return Err("invalid choices");
                    }
                    let repeated = attribute_flag(doc, param, "repeated")?;
                    if kind == "native" && repeated {
                        return Err("unsupported native cardinality");
                    }
                    let field = Field {
                        kind: kind.into(),
                        required: attribute_flag(doc, param, "required")?,
                        repeated,
                        private: kind == "native"
                            || attribute_flag(doc, param, "sensitive")?
                            || attr(doc, param, "delivery") == Some("endpoint-only"),
                        choices,
                        min_length: attr(doc, param, "min-length")
                            .unwrap_or("0")
                            .parse()
                            .map_err(|_| "invalid minimum length")?,
                    };
                    if fields.insert(name.into(), field).is_some() {
                        return Err("duplicate parameter");
                    }
                }
            }
            if commands
                .insert(id.into(), Arc::new(Definition { revision, fields }))
                .is_some()
            {
                return Err("duplicate command");
            }
        }
        Ok(Self(commands))
    }
}

impl Field {
    fn accepts(&self, value: &ParameterValue) -> bool {
        let ParameterValue::Scalar(value) = value else {
            return self.kind == "native";
        };
        let accepts_one = |v: &Value| match self.kind.as_str() {
            "string" => v
                .as_str()
                .is_some_and(|s| s.chars().count() >= self.min_length),
            "enum" => v.as_str().is_some_and(|s| self.choices.contains(s)),
            "number" => v.is_number(),
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            _ => false,
        };
        if self.repeated {
            value.as_array().is_some_and(|v| v.iter().all(accepts_one))
        } else {
            accepts_one(value)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InvocationId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Succeeded,
    Cancelled,
    Unavailable,
    Unhandled,
    Failed,
}
#[derive(Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub invocation: InvocationId,
    pub status: Status,
}

/// The host increments the epoch on revocation and changes registration on replacement.
/// Each entry describes this particular command at one owner, even when only a guard exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owner {
    pub identity: u64,
    pub registration: u64,
    pub eligible: Option<bool>,
    pub guarded: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub instance: u64,
    pub generation: u64,
    pub epoch: u64,
    pub context: u64,
    pub document_revision: u64,
    pub owners: Vec<Owner>,
}
impl Route {
    fn provider(&self) -> Result<&Owner, Status> {
        let mut seen = BTreeSet::new();
        if self.owners.iter().any(|owner| !seen.insert(owner.identity)) {
            return Err(Status::Unavailable);
        }
        let nearest = self
            .owners
            .iter()
            .find(|owner| owner.eligible.is_some())
            .ok_or(Status::Unhandled)?;
        if nearest.eligible == Some(true) {
            Ok(nearest)
        } else {
            Err(Status::Unavailable)
        }
    }
}
struct Pending {
    command: String,
    definition: Arc<Definition>,
    route: Route,
}

pub struct CommandCoordinator {
    catalog: CommandCatalog,
    pending: BTreeMap<InvocationId, Pending>,
}

impl CommandCoordinator {
    pub fn new(catalog: CommandCatalog) -> Self {
        Self {
            catalog,
            pending: BTreeMap::new(),
        }
    }
    /// Reject an incompatible edit without partially publishing the replacement.
    pub fn publish(&mut self, catalog: CommandCatalog) -> Result<Vec<CommandResult>, &'static str> {
        for (id, next) in &catalog.0 {
            if let Some(old) = self.catalog.0.get(id) {
                if next.revision < old.revision || (next.revision == old.revision && old != next) {
                    return Err("incompatible command revision");
                }
            }
        }
        let mut retired = Vec::new();
        self.pending.retain(|id, p| {
            let current = catalog.0.get(&p.command) == Some(&p.definition);
            if !current {
                retired.push(CommandResult {
                    invocation: *id,
                    status: Status::Unavailable,
                });
            }
            current
        });
        self.catalog = catalog;
        Ok(retired)
    }
    pub fn begin(&mut self, command: &str, route: Route) -> Result<InvocationId, CommandResult> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let invocation = InvocationId(
            NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .expect("invocation ID space exhausted"),
        );
        let refusal = |status| CommandResult { invocation, status };
        let definition = self
            .catalog
            .0
            .get(command)
            .ok_or_else(|| refusal(Status::Unavailable))?;
        route.provider().map_err(refusal)?;
        if self.pending.values().any(|p| {
            p.command == command
                && p.route.instance == route.instance
                && p.route.context == route.context
        }) {
            return Err(refusal(Status::Unavailable));
        }
        self.pending.insert(
            invocation,
            Pending {
                command: command.into(),
                definition: definition.clone(),
                route,
            },
        );
        Ok(invocation)
    }
    pub fn cancel(&mut self, invocation: InvocationId) -> Option<CommandResult> {
        self.pending.remove(&invocation).map(|_| CommandResult {
            invocation,
            status: Status::Cancelled,
        })
    }

    /// Consume the ticket before entering host code. Retries, foreign IDs and late cancels
    /// cannot claim or publish another result. Read the route again after synchronous guards.
    pub fn dispatch(
        &mut self,
        invocation: InvocationId,
        parameters: Parameters,
        current: impl Fn() -> Option<Route>,
        mut guard: impl FnMut(&Owner) -> bool,
        execute: impl FnOnce(&Owner, AdmittedParameters<'_>) -> Result<(), ()>,
    ) -> Option<CommandResult> {
        let pending = self.pending.remove(&invocation)?;
        let status = (|| {
            let valid = || {
                current().as_ref() == Some(&pending.route)
                    && self.catalog.0.get(&pending.command) == Some(&pending.definition)
            };
            if !valid() {
                return Status::Unavailable;
            }
            let fields = &pending.definition.fields;
            if fields
                .iter()
                .any(|(k, f)| f.required && !parameters.contains_key(k))
                || parameters
                    .iter()
                    .any(|(k, v)| !fields.get(k).is_some_and(|f| f.accepts(v)))
            {
                return Status::Failed;
            }
            for owner in pending.route.owners.iter().rev().filter(|o| o.guarded) {
                if !guard(owner) {
                    return Status::Cancelled;
                }
                if !valid() {
                    return Status::Unavailable;
                }
            }
            let Ok(endpoint) = pending.route.provider() else {
                return Status::Unavailable;
            };
            match execute(
                endpoint,
                AdmittedParameters {
                    fields,
                    values: &parameters,
                },
            ) {
                Ok(()) => Status::Succeeded,
                Err(()) => Status::Failed,
            }
        })();
        Some(CommandResult { invocation, status })
    }
}

/// Private and native fields are available only to the endpoint. Observers can receive
/// the scalar projection; it never owns a retained document or sensitive string.
pub struct AdmittedParameters<'a> {
    fields: &'a BTreeMap<String, Field>,
    values: &'a Parameters,
}
impl AdmittedParameters<'_> {
    pub fn endpoint_values(&self) -> &Parameters {
        self.values
    }
    pub fn public_scalars(&self) -> BTreeMap<&str, &Value> {
        self.values
            .iter()
            .filter_map(|(k, v)| match v {
                ParameterValue::Scalar(value) if !self.fields[k].private => {
                    Some((k.as_str(), value))
                }
                _ => None,
            })
            .collect()
    }
}
