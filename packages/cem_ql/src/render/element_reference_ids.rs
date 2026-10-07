//! Consumer-owned element relationships at the final DOM export boundary.
//! Original native owners and reference values are never rewritten.
use super::*;
use crate::eval::{item_identity, value_control::ValueControl, QueryItemViewKind};
use cem_ml::{
    schema::{
        reference_policy::{ReferenceOccurrence, ReferenceScopePolicy, ReferenceUnresolvedPolicy},
        reference_traversal::ReferenceTraversalLimits,
    },
    value::{
        artifact::CemValueArtifactLimits,
        reference_resolution::{
            resolve_owned_reference_structure, ReferenceLinkEvaluation, ReferenceResolutionHost,
            ReferenceResolutionState,
        },
        CemReference,
    },
};

/// Pinned consumer contracts; a browser never selects a profile implicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AriaReferenceProfile {
    #[default]
    Recommendation12,
    Draft13,
}
impl AriaReferenceProfile {
    pub fn identity(self) -> &'static str {
        match self {
            Self::Recommendation12 => "wai-aria-1.2-rec-20230606",
            Self::Draft13 => "wai-aria-1.3-wd-20260604",
        }
    }
}
impl std::str::FromStr for AriaReferenceProfile {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "wai-aria-1.2-rec-20230606" => Ok(Self::Recommendation12),
            "wai-aria-1.3-wd-20260604" => Ok(Self::Draft13),
            _ => Err("Unknown ARIA reference export profile"),
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ElementReferenceExportOptions {
    pub aria_profile: AriaReferenceProfile,
}

/// Host-owned control metadata. The target is an original retained native node;
/// only the producer may reserve the supplied ID for its committed placement.
#[derive(Debug, Clone)]
pub struct ElementPlacementAdmission {
    pub token: String,
    pub target: Item,
    pub producer: String,
    pub path: Vec<usize>,
    pub revision: String,
    pub id: String,
}
#[derive(Debug, Clone)]
pub struct ElementPlacementGrant {
    pub requester: String,
    pub token: String,
    pub properties: Vec<String>,
}
#[derive(Debug, Clone, Default)]
pub struct ElementPlacementSnapshot {
    pub admissions: Vec<ElementPlacementAdmission>,
    pub grants: Vec<ElementPlacementGrant>,
    pub committed_revisions: BTreeMap<String, String>,
    pub prepared_transaction: Option<ElementPlacementTransaction>,
}
#[derive(Debug, Clone)]
pub struct ElementPlacementTransaction {
    pub token: String,
    pub participants: BTreeSet<String>,
    pub producer_revisions: BTreeMap<String, String>,
}
/// Publication dependencies contain control metadata, never serialized nodes.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementPlacementUse {
    pub token: String,
    pub producer: String,
    pub revision: String,
    pub id: String,
    pub path: Vec<usize>,
    pub attribute: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
}
#[derive(Debug, Clone)]
pub struct ElementReferenceProjection {
    pub aria_profile: AriaReferenceProfile,
    pub plan: RenderPlan,
    pub placements: Vec<ElementPlacementUse>,
}

struct AdmissionIndex<'a> {
    targets: BTreeMap<String, Vec<&'a ElementPlacementAdmission>>,
    grants: BTreeMap<String, GrantedRequesters<'a>>,
}
type GrantedRequesters<'a> =
    BTreeMap<&'a str, BTreeMap<&'a str, BTreeMap<&'a str, &'a ElementPlacementAdmission>>>;
fn valid_token(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}
impl<'a> AdmissionIndex<'a> {
    fn prepare(
        snapshot: &'a ElementPlacementSnapshot,
        instance: &str,
        budget: &mut ValueControl<'_>,
    ) -> Result<Self, ElementReferenceProjectionError> {
        let source = SourceMapStack::default();
        let mut targets = BTreeMap::<String, Vec<_>>::new();
        let mut tokens = BTreeSet::new();
        let mut by_token = BTreeMap::new();
        let mut ids = BTreeSet::new();
        let mut placements = BTreeSet::new();
        if let Some(transaction) = &snapshot.prepared_transaction {
            budget
                .charge(transaction.token.len(), 0)
                .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
            if !valid_token(&transaction.token) || !transaction.participants.contains(instance) {
                return Err(error(
                    "cem.element_reference.transaction_invalid",
                    "Prepared placement transaction does not include this requester",
                    &source,
                ));
            }
            for participant in &transaction.participants {
                budget
                    .charge(participant.len(), 0)
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
                if !valid_token(participant) {
                    return Err(error(
                        "cem.element_reference.transaction_invalid",
                        "Invalid transaction participant",
                        &source,
                    ));
                }
            }
            for (producer, revision) in &transaction.producer_revisions {
                budget
                    .charge(producer.len() + revision.len(), 0)
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
                if !transaction.participants.contains(producer) || !valid_token(revision) {
                    return Err(error(
                        "cem.element_reference.transaction_invalid",
                        "Prepared revision needs a participating producer",
                        &source,
                    ));
                }
            }
        }
        for (producer, revision) in &snapshot.committed_revisions {
            budget
                .charge(producer.len() + revision.len(), 0)
                .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
            if !valid_token(producer) || !valid_token(revision) {
                return Err(error(
                    "cem.element_reference.placement_invalid",
                    "Invalid committed producer revision",
                    &source,
                ));
            }
        }
        for admission in &snapshot.admissions {
            let source = admission.target.source_map().unwrap_or_default();
            budget
                .charge(
                    std::mem::size_of::<ElementPlacementAdmission>()
                        + admission.token.len()
                        + admission.producer.len()
                        + admission.revision.len()
                        + admission.id.len()
                        + admission
                            .path
                            .len()
                            .saturating_mul(std::mem::size_of::<usize>()),
                    admission.path.len(),
                )
                .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
            if kind(&admission.target).as_deref() != Some("element")
                || !valid_token(&admission.token)
                || !valid_token(&admission.producer)
                || admission.producer == instance
                || !valid_token(&admission.id)
                || admission.path.is_empty()
                || !tokens.insert(admission.token.as_str())
                || !ids.insert(admission.id.as_str())
                || !placements.insert((admission.producer.as_str(), admission.path.as_slice()))
            {
                return Err(error(
                    "cem.element_reference.placement_invalid",
                    "Invalid or duplicate native placement admission",
                    &source,
                ));
            }
            if snapshot.committed_revisions.get(&admission.producer) != Some(&admission.revision)
                && !snapshot.prepared_transaction.as_ref().is_some_and(|t| {
                    t.producer_revisions.get(&admission.producer) == Some(&admission.revision)
                })
            {
                let mut failure = error(
                    "cem.element_reference.placement_stale",
                    "Placement has no matching committed producer revision",
                    &source,
                );
                failure.incomplete = true;
                return Err(failure);
            }
            let key = target_key(&admission.target);
            budget
                .charge(key.len(), 0)
                .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
            targets.entry(key).or_default().push(admission);
            by_token.insert(admission.token.as_str(), admission);
        }
        let mut grants = BTreeMap::<String, GrantedRequesters<'a>>::new();
        for grant in &snapshot.grants {
            budget
                .charge(grant.requester.len() + grant.token.len(), 0)
                .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
            if !valid_token(&grant.requester)
                || !tokens.contains(grant.token.as_str())
                || grant.properties.is_empty()
            {
                return Err(error(
                    "cem.element_reference.placement_invalid",
                    "Invalid placement grant",
                    &source,
                ));
            }
            for property in &grant.properties {
                budget
                    .charge(property.len(), 0)
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
                if arity("", property).is_none() {
                    return Err(error(
                        "cem.element_reference.placement_invalid",
                        "Unknown granted relationship property",
                        &source,
                    ));
                }
                let admission = by_token[grant.token.as_str()];
                let key = target_key(&admission.target);
                budget
                    .charge(key.len() + std::mem::size_of::<ElementPlacementGrant>(), 0)
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
                grants
                    .entry(key)
                    .or_default()
                    .entry(&grant.requester)
                    .or_default()
                    .entry(property)
                    .or_default()
                    .insert(&grant.token, admission);
            }
        }
        Ok(Self { targets, grants })
    }
    fn select(
        &self,
        target: &Item,
        instance: &str,
        property: &str,
        source: &SourceMapStack,
    ) -> Result<&'a ElementPlacementAdmission, ElementReferenceProjectionError> {
        let key = target_key(target);
        if !self.targets.contains_key(&key) {
            return Err(error(
                "cem.element_reference.target_missing",
                "Target has no produced or admitted placement",
                source,
            ));
        }
        let allowed = self
            .grants
            .get(&key)
            .and_then(|requests| requests.get(instance))
            .and_then(|properties| properties.get(property));
        let Some(allowed) = allowed else {
            return Err(error(
                "cem.element_reference.placement_denied",
                "Relationship needs an explicit placement grant",
                source,
            ));
        };
        if allowed.len() != 1 {
            return Err(error(
                "cem.element_reference.placement_ambiguous",
                "Grant must select exactly one foreign placement",
                source,
            ));
        }
        Ok(*allowed.values().next().expect("nonempty granted admission"))
    }
}

#[derive(Debug, Clone)]
pub struct ElementReferenceProjectionError {
    pub diagnostics: Vec<Diagnostic>,
    pub incomplete: bool,
    code: String,
    message: String,
}
impl ElementReferenceProjectionError {
    pub fn code(&self) -> &str {
        &self.code
    }
}
impl std::fmt::Display for ElementReferenceProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.message)
    }
}
impl std::error::Error for ElementReferenceProjectionError {}
fn error(
    code: &str,
    message: impl Into<String>,
    source: &SourceMapStack,
) -> ElementReferenceProjectionError {
    let message = message.into();
    ElementReferenceProjectionError {
        incomplete: false,
        code: code.into(),
        message: message.clone(),
        diagnostics: vec![render_diagnostic(
            code,
            message,
            source_map_start(source),
            source.clone(),
        )],
    }
}

/// Already-materialized values belong to this admitted producer invocation.
/// Authored source references remain pending; an embedding lifecycle host can
/// evaluate them explicitly through the `with_host` entry point below.
struct MaterializedHost {
    policy: ReferenceScopePolicy,
}
impl ReferenceResolutionHost for MaterializedHost {
    type Node = Item;
    type Scope = ();
    fn scope(&self, _: &Item) {}
    fn scope_limits(&self, _: &()) -> ReferenceTraversalLimits {
        self.policy.limits
    }
    fn reference_occurrence(&self, item: &Item) -> Option<ReferenceOccurrence> {
        let view = item.view()?;
        if kind(item).as_deref() != Some("reference") {
            return None;
        }
        Some(ReferenceOccurrence {
            identity: view.identity(),
            node_id: None,
            expression: None,
            source_map: item.source_map().unwrap_or_default(),
        })
    }
    fn unresolved_policy(&self, _: &Item) -> &ReferenceUnresolvedPolicy {
        &self.policy.unresolved
    }
    fn permits_edge(&self, _: &Item, _: &Item) -> bool {
        true
    }
    fn evaluate(&mut self, item: &Item) -> ReferenceLinkEvaluation<Item> {
        if let Some(values) = crate::eval::values::reference_values(item) {
            return ReferenceLinkEvaluation::Resolved(values.to_vec());
        }
        let Some(view) = item.view() else {
            return ReferenceLinkEvaluation::Unresolved("not-a-native-reference".into());
        };
        if view
            .field("targets_available")
            .and_then(|v| v.first()?.atom())
            == Some(AtomValue::Boolean(true))
        {
            return ReferenceLinkEvaluation::Resolved(view.field("targets").unwrap_or_default());
        }
        ReferenceLinkEvaluation::Pending("source-reference-requires-lifecycle-host".into())
    }
}
fn target_key(item: &Item) -> String {
    if let Some(node) = crate::eval::retained_cem_node(item) {
        return item_identity(&node.query_item());
    }
    item_identity(item)
}
fn kind(item: &Item) -> Option<String> {
    if item.view()?.kind() != QueryItemViewKind::Node {
        return None;
    }
    match item.view()?.field("kind")?.first()?.atom()? {
        AtomValue::String(s) => Some(s),
        _ => None,
    }
}
fn arity(tag: &str, name: &str) -> Option<bool> {
    match name {
        "aria-controls" | "aria-labelledby" | "aria-describedby" | "aria-owns" | "headers" => {
            Some(true)
        }
        "for" => Some(tag == "output"),
        "commandfor"
        | "popovertarget"
        | "interestfor"
        | "form"
        | "list"
        | "aria-activedescendant"
        | "aria-details"
        | "aria-errormessage"
        | "command-target"
        | "interaction"
        | "trigger-for"
        | "parent-item"
        | "focus-target"
        | "return-focus"
        | "anchor"
        | "boundary" => Some(false),
        _ => None,
    }
}
fn is_interaction(name: &str) -> bool {
    matches!(
        name,
        "command-target"
            | "interaction"
            | "trigger-for"
            | "parent-item"
            | "focus-target"
            | "return-focus"
            | "anchor"
            | "boundary"
    )
}

pub fn project_element_reference_ids(
    plan: &RenderPlan,
    instance_id: &str,
    limits: &CemValueArtifactLimits,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<RenderPlan, ElementReferenceProjectionError> {
    project_element_reference_ids_with_options(
        plan,
        instance_id,
        limits,
        control,
        scope,
        Default::default(),
    )
}
pub fn project_element_reference_ids_with_options(
    plan: &RenderPlan,
    instance_id: &str,
    limits: &CemValueArtifactLimits,
    control: &OperationControl,
    scope: ExecutionScopeId,
    options: ElementReferenceExportOptions,
) -> Result<RenderPlan, ElementReferenceProjectionError> {
    project_element_reference_ids_with_host_and_placements_with_options(
        plan,
        instance_id,
        limits,
        control,
        scope,
        &mut MaterializedHost {
            policy: ReferenceScopePolicy::schema_defaults().expect("embedded policy"),
        },
        &ElementPlacementSnapshot::default(),
        options,
    )
    .map(|result| result.plan)
}

/// Lifecycle callers supply actual contexts, scope policies and directed grants
/// through the existing bounded resolution host. Terminal targets must still have
/// exactly one placement in this producer; no browser or document ID lookup occurs.
pub fn project_element_reference_ids_with_host<H: ReferenceResolutionHost<Node = Item>>(
    plan: &RenderPlan,
    instance_id: &str,
    limits: &CemValueArtifactLimits,
    control: &OperationControl,
    scope: ExecutionScopeId,
    host: &mut H,
) -> Result<RenderPlan, ElementReferenceProjectionError> {
    project_element_reference_ids_with_host_and_placements(
        plan,
        instance_id,
        limits,
        control,
        scope,
        host,
        &ElementPlacementSnapshot::default(),
    )
    .map(|result| result.plan)
}

/// Placement grants supplement source-scope grants. Native export checks one
/// immutable snapshot; the host must recheck its returned tokens before commit.
pub fn project_element_reference_ids_with_host_and_placements<
    H: ReferenceResolutionHost<Node = Item>,
>(
    plan: &RenderPlan,
    instance_id: &str,
    limits: &CemValueArtifactLimits,
    control: &OperationControl,
    scope: ExecutionScopeId,
    host: &mut H,
    snapshot: &ElementPlacementSnapshot,
) -> Result<ElementReferenceProjection, ElementReferenceProjectionError> {
    project_element_reference_ids_with_host_and_placements_with_options(
        plan,
        instance_id,
        limits,
        control,
        scope,
        host,
        snapshot,
        Default::default(),
    )
}
pub fn project_element_reference_ids_with_host_and_placements_with_options<
    H: ReferenceResolutionHost<Node = Item>,
>(
    plan: &RenderPlan,
    instance_id: &str,
    limits: &CemValueArtifactLimits,
    control: &OperationControl,
    scope: ExecutionScopeId,
    host: &mut H,
    snapshot: &ElementPlacementSnapshot,
    options: ElementReferenceExportOptions,
) -> Result<ElementReferenceProjection, ElementReferenceProjectionError> {
    let source = SourceMapStack::default();
    if instance_id.is_empty() || instance_id.chars().any(char::is_whitespace) {
        return Err(error(
            "cem.element_reference.instance",
            "A persisted producer instance identity is required",
            &source,
        ));
    }
    let mut budget = ValueControl::new(control, scope, QueryContextScope(0), *limits)
        .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
    let admissions = AdmissionIndex::prepare(snapshot, instance_id, &mut budget)?;
    let mut used_placements = Vec::new();
    let mut projection = Projection {
        placements: BTreeMap::new(),
        budget: &mut budget,
    };
    let mut result = RenderPlan {
        nodes: vec![],
        host_attribute_updates: plan.host_attribute_updates.clone(),
        diagnostics: plan.diagnostics.clone(),
    };
    projection.nodes(&plan.nodes, &mut result.nodes, &[], 0)?;
    let placements = projection.placements;
    let mut relationships = Vec::new();
    collect(&result.nodes, &[], &mut relationships, &mut budget)?;
    let mut ids = BTreeMap::<String, Vec<Vec<usize>>>::new();
    collect_ids(&result.nodes, &[], &mut ids);
    let foreign_ids: BTreeSet<_> = snapshot.admissions.iter().map(|a| a.id.as_str()).collect();
    if ids.keys().any(|id| foreign_ids.contains(id.as_str())) {
        return Err(error(
            "cem.element_reference.id_conflict",
            "Produced and admitted placements share an ID",
            &source,
        ));
    }
    let targets = resolve_slots(&relationships, host, &mut budget)?;
    for ((path, index, attribute), targets) in relationships.into_iter().zip(targets) {
        let tag = element_tag(&result.nodes, &path);
        let multiple = arity(tag, &attribute.name).expect("collected relationship")
            || (options.aria_profile == AriaReferenceProfile::Draft13
                && matches!(
                    attribute.name.as_str(),
                    "aria-details" | "aria-errormessage"
                ));
        let unique = attribute.name == "headers" || (tag == "output" && attribute.name == "for");
        if targets.is_empty() || (!multiple && targets.len() != 1) {
            return Err(error(
                "cem.element_reference.cardinality",
                "Relationship requires one target, or a nonempty sequence for an ID list",
                &attribute.source_map,
            ));
        }
        let mut values = Vec::new();
        let mut uses_foreign_placement = false;
        let mut seen = std::collections::BTreeSet::new();
        for target in targets {
            if kind(&target).as_deref() != Some("element") {
                return Err(error(
                    "cem.element_reference.target_kind",
                    "Relationship targets must be native elements",
                    &attribute.source_map,
                ));
            }
            let Some(paths) = placements.get(&target_key(&target)) else {
                let admission = admissions.select(
                    &target,
                    instance_id,
                    &attribute.name,
                    &attribute.source_map,
                )?;
                if ids.contains_key(&admission.id) {
                    return Err(error(
                        "cem.element_reference.id_conflict",
                        "Admitted ID conflicts with this producer's ID space",
                        &attribute.source_map,
                    ));
                }
                budget
                    .charge(
                        admission.id.len() + std::mem::size_of::<ElementPlacementUse>(),
                        path.len(),
                    )
                    .map_err(|e| {
                        error(
                            "cem.element_reference.limit",
                            e.to_string(),
                            &attribute.source_map,
                        )
                    })?;
                used_placements.push(ElementPlacementUse {
                    token: admission.token.clone(),
                    producer: admission.producer.clone(),
                    revision: admission.revision.clone(),
                    id: admission.id.clone(),
                    path: path.clone(),
                    attribute: attribute.name.clone(),
                    transaction: if snapshot.committed_revisions.get(&admission.producer)
                        == Some(&admission.revision)
                    {
                        None
                    } else {
                        snapshot
                            .prepared_transaction
                            .as_ref()
                            .map(|t| t.token.clone())
                    },
                });
                uses_foreign_placement = true;
                if !unique || seen.insert(admission.id.clone()) {
                    values.push(admission.id.clone());
                }
                continue;
            };
            let [target_path] = paths.as_slice() else {
                return Err(error(
                    "cem.element_reference.target_ambiguous",
                    "Target has multiple produced placements; select a distinct occurrence",
                    &attribute.source_map,
                ));
            };
            let target_attributes = attributes_mut(&mut result.nodes, target_path);
            let id = if let Some(id) = target_attributes
                .iter()
                .find(|a| a.name == "id" && a.namespace.is_none())
            {
                if id.value.is_empty()
                    || id.value.chars().any(char::is_whitespace)
                    || ids.get(&id.value).is_none_or(|p| p.len() != 1)
                {
                    return Err(error(
                        "cem.element_reference.id_conflict",
                        "Referenced authored ID must be nonempty and unambiguous",
                        &attribute.source_map,
                    ));
                }
                id.value.clone()
            } else {
                let id = format!(
                    "{instance_id}-ref-{}",
                    target_path
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join("-")
                );
                if ids.contains_key(&id) || foreign_ids.contains(id.as_str()) {
                    return Err(error(
                        "cem.element_reference.id_conflict",
                        "Generated ID conflicts with an authored ID",
                        &attribute.source_map,
                    ));
                }
                budget.charge(id.len(), target_path.len()).map_err(|e| {
                    error(
                        "cem.element_reference.limit",
                        e.to_string(),
                        &attribute.source_map,
                    )
                })?;
                target_attributes.push(lexical_attribute("id", &id, &attribute.source_map));
                ids.insert(id.clone(), vec![target_path.clone()]);
                id
            };
            budget.charge(id.len(), path.len()).map_err(|e| {
                error(
                    "cem.element_reference.limit",
                    e.to_string(),
                    &attribute.source_map,
                )
            })?;
            if !unique || seen.insert(id.clone()) {
                values.push(id);
            }
        }
        let value = values.join(" ");
        budget.charge(value.len(), path.len()).map_err(|e| {
            error(
                "cem.element_reference.limit",
                e.to_string(),
                &attribute.source_map,
            )
        })?;
        let attributes = attributes_mut(&mut result.nodes, &path);
        // ID extraction is a terminal consumer export. The original retained plan
        // still owns the native reference; output uses browser lexical IDs.
        attributes[index].value = value.clone();
        attributes[index].value_stream = string_stream(value);
        attributes[index].contract = None;
        if is_interaction(&attribute.name) {
            attributes.push(lexical_attribute(
                &format!("data-cem-node-ref-{}", attribute.name),
                "",
                &attribute.source_map,
            ));
        }
        if uses_foreign_placement {
            attributes.push(lexical_attribute(
                &format!("data-cem-placement-ref-{}", attribute.name),
                "",
                &attribute.source_map,
            ));
        }
    }
    control
        .check_scope(scope)
        .map_err(|e| error("cem.element_reference.limit", e.to_string(), &source))?;
    Ok(ElementReferenceProjection {
        aria_profile: options.aria_profile,
        plan: result,
        placements: used_placements,
    })
}
// One owning structure groups attribute slots without adding an authored
// reference constructor or resetting destination accounting between slots.
fn resolve_slots<H: ReferenceResolutionHost<Node = Item>>(
    slots: &[(Vec<usize>, usize, RenderPlanAttribute)],
    host: &mut H,
    budget: &mut ValueControl<'_>,
) -> Result<Vec<Vec<Item>>, ElementReferenceProjectionError> {
    if slots.is_empty() {
        return Ok(vec![]);
    }
    let source = &slots[0].2.source_map;
    if budget.remaining_work() == 0 || budget.limits.max_depth == 0 {
        return Err(error(
            "cem.element_reference.limit",
            "Relationship work budget is exhausted",
            source,
        ));
    }
    let mut carriers = Vec::new();
    let mut indices = BTreeMap::new();
    for (index, (_, _, attribute)) in slots.iter().enumerate() {
        budget
            .charge(std::mem::size_of::<RenderPlanAttribute>(), 0)
            .map_err(|e| {
                error(
                    "cem.element_reference.limit",
                    e.to_string(),
                    &attribute.source_map,
                )
            })?;
        let carrier = crate::eval::output::output_attribute(attribute.clone());
        indices.insert(item_identity(&carrier), index);
        carriers.push(carrier);
    }
    let root = crate::eval::output::output_nodes(vec![RenderPlanNode::Element {
        tag: "reference-slots".into(),
        namespace: None,
        qualified_name: None,
        attributes: vec![],
        children: vec![],
        source_map: source.clone(),
    }])
    .items
    .remove(0);
    let identity = item_identity(&root);
    let walk = resolve_owned_reference_structure(
        root,
        host,
        ReferenceTraversalLimits {
            max_depth: budget.limits.max_depth,
            max_work: budget.remaining_work(),
        },
        ReferenceOccurrence {
            identity: identity.clone(),
            node_id: None,
            expression: None,
            source_map: source.clone(),
        },
        |_, item| {
            let key = item_identity(item);
            if key == identity {
                Some(carriers.clone())
            } else {
                indices
                    .get(&key)
                    .map(|&index| slots[index].2.value_stream.items.clone())
            }
        },
    )
    .map_err(|e| error("cem.element_reference.resolution", e.to_string(), source))?;
    for _ in 0..walk.resolution.work_used {
        budget
            .charge(0, 0)
            .map_err(|e| error("cem.element_reference.limit", e.to_string(), source))?;
    }
    if walk.resolution.state != ReferenceResolutionState::Resolved || walk.resolution.failed {
        let mut failure = error(
            "cem.element_reference.pending",
            "Relationship resolution is incomplete; no IDs were published",
            source,
        );
        failure.incomplete = true;
        failure.diagnostics = walk.resolution.diagnostics;
        return Err(failure);
    }
    let root = walk.roots[0];
    Ok(walk.children[root]
        .iter()
        .map(|&slot| {
            walk.children[slot]
                .iter()
                .map(|&target| walk.resolution.nodes[target].clone())
                .collect()
        })
        .collect())
}

fn lexical_attribute(name: &str, value: &str, source: &SourceMapStack) -> RenderPlanAttribute {
    RenderPlanAttribute {
        name: name.into(),
        namespace: None,
        qualified_name: None,
        value: value.into(),
        value_stream: string_stream(value.into()),
        contract: None,
        source_map: source.clone(),
    }
}
struct Projection<'a, 'b> {
    placements: BTreeMap<String, Vec<Vec<usize>>>,
    budget: &'a mut ValueControl<'b>,
}
impl Projection<'_, '_> {
    fn nodes(
        &mut self,
        input: &[RenderPlanNode],
        out: &mut Vec<RenderPlanNode>,
        parent: &[usize],
        depth: usize,
    ) -> Result<(), ElementReferenceProjectionError> {
        for node in input {
            self.node(node, None, out, parent, depth)?;
        }
        Ok(())
    }
    fn node(
        &mut self,
        node: &RenderPlanNode,
        origin: Option<&Item>,
        out: &mut Vec<RenderPlanNode>,
        parent: &[usize],
        depth: usize,
    ) -> Result<(), ElementReferenceProjectionError> {
        self.budget
            .charge(std::mem::size_of::<RenderPlanNode>(), depth)
            .map_err(|e| {
                error(
                    "cem.element_reference.limit",
                    e.to_string(),
                    &SourceMapStack::default(),
                )
            })?;
        if let RenderPlanNode::Reference {
            reference,
            source_map,
        } = node
        {
            for item in reference.values() {
                if kind(item).as_deref() == Some("reference") {
                    let Some(values) = item
                        .view()
                        .filter(|v| {
                            v.field("targets_available").and_then(|v| v.first()?.atom())
                                == Some(AtomValue::Boolean(true))
                        })
                        .and_then(|v| v.field("targets"))
                    else {
                        return Err(error(
                            "cem.element_reference.pending",
                            "Body reference needs an explicit lifecycle completion",
                            source_map,
                        ));
                    };
                    self.node(
                        &RenderPlanNode::Reference {
                            reference: CemReference::new(values),
                            source_map: source_map.clone(),
                        },
                        None,
                        out,
                        parent,
                        depth + 1,
                    )?;
                    continue;
                }
                let expanded = references::expand_reference_scoped(
                    &CemReference::new(vec![item.clone()]),
                    self.budget,
                )
                .map_err(|e| error("cem.element_reference.limit", e.to_string(), source_map))?;
                for expanded in expanded {
                    self.node(&expanded, Some(item), out, parent, depth + 1)?;
                }
            }
            return Ok(());
        }
        let mut path = parent.to_vec();
        path.push(out.len());
        let mut copy = match node {
            RenderPlanNode::Element {
                tag,
                qualified_name,
                namespace,
                attributes,
                source_map,
                ..
            } => {
                self.budget
                    .charge(
                        tag.len()
                            + qualified_name.as_ref().map_or(0, String::len)
                            + namespace.as_ref().map_or(0, String::len),
                        depth,
                    )
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), source_map))?;
                for a in attributes {
                    self.budget
                        .charge(
                            a.name.len()
                                + a.value.len()
                                + std::mem::size_of::<RenderPlanAttribute>()
                                + a.value_stream
                                    .items
                                    .len()
                                    .saturating_mul(std::mem::size_of::<Item>()),
                            depth,
                        )
                        .map_err(|e| {
                            error("cem.element_reference.limit", e.to_string(), &a.source_map)
                        })?;
                }
                RenderPlanNode::Element {
                    tag: tag.clone(),
                    qualified_name: qualified_name.clone(),
                    namespace: namespace.clone(),
                    attributes: attributes.clone(),
                    children: vec![],
                    source_map: source_map.clone(),
                }
            }
            RenderPlanNode::Text { text, source_map }
            | RenderPlanNode::Comment { text, source_map }
            | RenderPlanNode::Cdata { text, source_map } => {
                self.budget
                    .charge(text.len(), depth)
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), source_map))?;
                node.clone()
            }
            RenderPlanNode::ProcessingInstruction {
                target,
                data,
                source_map,
            } => {
                self.budget
                    .charge(target.len() + data.len(), depth)
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), source_map))?;
                node.clone()
            }
            RenderPlanNode::Reference { .. } => unreachable!(),
        };
        if let RenderPlanNode::Element {
            tag,
            attributes,
            children,
            source_map,
            ..
        } = &mut copy
        {
            if attributes.iter().any(|a| {
                a.name.starts_with("data-cem-node-ref-")
                    || a.name.starts_with("data-cem-placement-ref-")
            }) {
                return Err(error(
                    "cem.element_reference.reserved",
                    "Relationship projection markers are runtime-owned",
                    source_map,
                ));
            }
            let origin = origin.filter(|n| kind(n).as_deref() == Some("element"));
            if let Some(origin) = origin {
                let key = target_key(origin);
                self.budget
                    .charge(
                        key.len() + path.len().saturating_mul(std::mem::size_of::<usize>()),
                        depth,
                    )
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), source_map))?;
                self.placements.entry(key).or_default().push(path.clone());
                // Imported native attributes retain their original value nodes.
                // This is an owning value-slot access, never target evaluation.
                if let Some(native_attributes) = origin.view().and_then(|v| v.field("attributes")) {
                    for native_attribute in native_attributes {
                        let field = |name| {
                            native_attribute
                                .view()
                                .and_then(|v| v.field(name))
                                .and_then(|v| v.first()?.atom())
                        };
                        if let (Some(AtomValue::String(name)), Some(values)) = (
                            field("name"),
                            native_attribute.view().and_then(|v| v.field("valueNodes")),
                        ) {
                            if arity(tag, &name).is_some() && !values.is_empty() {
                                self.budget
                                    .charge(
                                        values.len().saturating_mul(std::mem::size_of::<Item>()),
                                        depth,
                                    )
                                    .map_err(|e| {
                                        error(
                                            "cem.element_reference.limit",
                                            e.to_string(),
                                            source_map,
                                        )
                                    })?;
                                if let Some(attribute) = attributes
                                    .iter_mut()
                                    .find(|a| a.name == name && a.namespace.is_none())
                                {
                                    attribute.value_stream = ItemStream::from_items(values);
                                }
                            }
                        }
                    }
                }
            }
            // Native children preserve original identities. Literal template
            // children are traversed directly without cloning an entire subtree.
            if let Some(values) = origin
                .and_then(|n| n.view())
                .and_then(|v| v.field("children"))
            {
                for item in values {
                    let source_map = item.source_map().unwrap_or_default();
                    self.node(
                        &RenderPlanNode::Reference {
                            source_map,
                            reference: CemReference::new(vec![item]),
                        },
                        None,
                        children,
                        &path,
                        depth + 1,
                    )?;
                }
            } else if let RenderPlanNode::Element {
                children: input, ..
            } = node
            {
                self.nodes(input, children, &path, depth + 1)?;
            }
        }
        out.push(copy);
        Ok(())
    }
}
fn collect(
    nodes: &[RenderPlanNode],
    parent: &[usize],
    out: &mut Vec<(Vec<usize>, usize, RenderPlanAttribute)>,
    budget: &mut ValueControl<'_>,
) -> Result<(), ElementReferenceProjectionError> {
    for (i, node) in nodes.iter().enumerate() {
        let mut path = parent.to_vec();
        path.push(i);
        if let RenderPlanNode::Element {
            tag,
            attributes,
            children,
            source_map,
            ..
        } = node
        {
            for (index, attribute) in attributes.iter().enumerate() {
                budget
                    .charge(std::mem::size_of::<RenderPlanAttribute>(), path.len())
                    .map_err(|e| error("cem.element_reference.limit", e.to_string(), source_map))?;
                if attribute.namespace.is_none()
                    && arity(tag, &attribute.name).is_some()
                    && attribute
                        .value_stream
                        .items
                        .iter()
                        .any(|n| kind(n).is_some())
                {
                    out.push((path.clone(), index, attribute.clone()));
                }
            }
            collect(children, &path, out, budget)?;
        }
    }
    Ok(())
}
fn collect_ids(
    nodes: &[RenderPlanNode],
    parent: &[usize],
    ids: &mut BTreeMap<String, Vec<Vec<usize>>>,
) {
    for (i, node) in nodes.iter().enumerate() {
        let mut path = parent.to_vec();
        path.push(i);
        if let RenderPlanNode::Element {
            attributes,
            children,
            ..
        } = node
        {
            for attribute in attributes
                .iter()
                .filter(|a| a.name == "id" && a.namespace.is_none())
            {
                ids.entry(attribute.value.clone())
                    .or_default()
                    .push(path.clone());
            }
            collect_ids(children, &path, ids);
        }
    }
}
fn element_tag<'a>(nodes: &'a [RenderPlanNode], path: &[usize]) -> &'a str {
    let RenderPlanNode::Element { tag, children, .. } = &nodes[path[0]] else {
        unreachable!("produced element path")
    };
    if path.len() == 1 {
        tag
    } else {
        element_tag(children, &path[1..])
    }
}
fn attributes_mut<'a>(
    nodes: &'a mut [RenderPlanNode],
    path: &[usize],
) -> &'a mut Vec<RenderPlanAttribute> {
    let RenderPlanNode::Element {
        attributes,
        children,
        ..
    } = &mut nodes[path[0]]
    else {
        unreachable!("produced element path")
    };
    if path.len() == 1 {
        attributes
    } else {
        attributes_mut(children, &path[1..])
    }
}
