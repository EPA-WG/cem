//! Invocation-owned source lifecycle admission for native element-ID consumers.
use super::{
    reference_transport::RetainedReferenceSource, StandaloneExpressionBinding,
    StandaloneExpressionContext,
};
use crate::{
    render::{
        project_element_reference_ids_with_host_and_placements, ElementPlacementAdmission,
        ElementPlacementGrant, ElementPlacementSnapshot, ElementPlacementTransaction,
        ElementReferenceProjection, ElementReferenceProjectionError, RenderPlan, TemplateData,
    },
    schema_references::{CemQlSchemaDeclarationHost, DeclarationScope},
};
use cem_ml::{
    operation_control::{ExecutionScopeId, OperationControl},
    schema::reference_policy::ReferenceScopePolicy,
    value::artifact::CemValueArtifactLimits,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub struct ElementReferenceSource {
    pub source: RetainedReferenceSource,
    /// Explicitly supply this invocation's template inputs; false means pending.
    pub context: bool,
    pub policy: ReferenceScopePolicy,
}
#[derive(Clone)]
pub struct ElementReferenceBinding {
    pub source: usize,
    pub name: String,
    /// Passive native query selection, never reference evaluation or ID lookup.
    pub select: String,
}
pub struct ElementReferenceExecution {
    host: CemQlSchemaDeclarationHost,
    requesting: DeclarationScope,
    placements: ElementPlacementSnapshot,
}
#[derive(Clone)]
pub struct ElementPlacementSelection {
    pub source: usize,
    pub select: String,
    pub token: String,
    pub producer: String,
    pub path: Vec<usize>,
    pub revision: String,
    pub id: String,
}
#[derive(Clone, Default)]
pub struct ElementPlacementInputs {
    pub admissions: Vec<ElementPlacementSelection>,
    pub grants: Vec<ElementPlacementGrant>,
    pub committed_revisions: BTreeMap<String, String>,
    pub prepared_transaction: Option<ElementPlacementTransaction>,
}
fn context(data: &TemplateData) -> StandaloneExpressionContext {
    StandaloneExpressionContext {
        bindings: data
            .bindings
            .iter()
            .map(|(name, values)| {
                (
                    name.clone(),
                    StandaloneExpressionBinding::any(values.clone()),
                )
            })
            .collect(),
        native_functions: data.native_functions.clone(),
        ..Default::default()
    }
}
impl ElementReferenceExecution {
    /// Build a fresh host over original retained owners and captures. Query
    /// bindings are staged atomically and contexts never enter source metadata.
    pub fn prepare(
        sources: Vec<ElementReferenceSource>,
        requesting: usize,
        bindings: &[ElementReferenceBinding],
        grants: &[(usize, usize)],
        data: &mut TemplateData,
    ) -> Result<Self, String> {
        Self::prepare_with_placements(
            sources,
            requesting,
            bindings,
            grants,
            ElementPlacementInputs::default(),
            data,
        )
    }
    pub fn prepare_with_placements(
        sources: Vec<ElementReferenceSource>,
        requesting: usize,
        bindings: &[ElementReferenceBinding],
        grants: &[(usize, usize)],
        placements: ElementPlacementInputs,
        data: &mut TemplateData,
    ) -> Result<Self, String> {
        let source = sources
            .get(requesting)
            .ok_or("Unknown requesting reference source")?;
        let ceiling = source.policy.limits.max_work;
        if sources
            .len()
            .saturating_add(bindings.len())
            .saturating_add(grants.len())
            .saturating_add(placements.admissions.len())
            .saturating_add(placements.grants.len())
            .saturating_add(placements.committed_revisions.len())
            > ceiling
        {
            return Err("Element reference metadata work limit exceeded".into());
        }
        let mut owners = BTreeSet::new();
        for source in &sources {
            source
                .source
                .require_lexical()
                .map_err(|e| format!("Missing source capture: {e:?}"))?;
            if source.policy.limits.max_work == 0 || source.policy.limits.max_depth == 0 {
                return Err("Invalid reference source bounds".into());
            }
            let identity =
                std::sync::Arc::as_ptr(source.source.ingress().source().ast_owner()) as usize;
            if !owners.insert(identity) {
                return Err("Duplicate reference source owner".into());
            }
        }
        let mut names = BTreeSet::new();
        for binding in bindings {
            if sources.get(binding.source).is_none()
                || binding.name.is_empty()
                || matches!(
                    binding.name.as_str(),
                    "datadom" | "input" | "context" | "attributes" | "slices" | "instanceID"
                )
                || !names.insert(&binding.name)
            {
                return Err("Invalid or duplicate reference source binding".into());
            }
        }
        for &(from, to) in grants {
            if sources.get(from).is_none() || sources.get(to).is_none() {
                return Err("Unknown reference crossing source".into());
            }
        }
        let selection_context = context(data);
        let mut staged = data.clone();
        for binding in bindings {
            let values = sources[binding.source]
                .source
                .evaluate(&binding.select, &selection_context)
                .map_err(|e| format!("{}: {}", e.code, e.message))?
                .result;
            if values.error.is_some() || values.items.len() > ceiling {
                return Err("Reference source selection failed or exceeded bounds".into());
            }
            staged.bind_native_slice(&binding.name, values)?;
        }
        let frame = context(&staged);
        let mut snapshot = ElementPlacementSnapshot {
            admissions: Vec::new(),
            grants: placements.grants,
            committed_revisions: placements.committed_revisions,
            prepared_transaction: placements.prepared_transaction,
        };
        for admission in placements.admissions {
            let source = sources
                .get(admission.source)
                .ok_or("Unknown placement source")?;
            let selected = source
                .source
                .evaluate(&admission.select, &selection_context)
                .map_err(|e| format!("{}: {}", e.code, e.message))?
                .result;
            if selected.error.is_some()
                || selected.items.len() != 1
                || crate::eval::retained_cem_node(&selected.items[0]).is_none()
            {
                return Err("Placement admission must select exactly one original node".into());
            }
            snapshot.admissions.push(ElementPlacementAdmission {
                target: selected.items[0].clone(),
                token: admission.token,
                producer: admission.producer,
                path: admission.path,
                revision: admission.revision,
                id: admission.id,
            });
        }
        let mut host = CemQlSchemaDeclarationHost::new();
        let mut scopes = Vec::new();
        for source in &sources {
            let context = source.context.then(|| frame.clone());
            let scope = host.register_scope(
                source.source.ingress().source().clone(),
                context.clone(),
                source.policy.clone(),
            );
            scopes.push(scope);
            let capture = source.source.require_lexical().expect("validated capture");
            host.attach_captured_namespaces(capture.clone())
                .map_err(|e| format!("{e:?}"))?;
            host.attach_captured_lexical_scopes_with_policy_overrides(capture, |_, _, _| {
                (context.clone(), Default::default())
            })
            .map_err(|e| format!("{e:?}"))?;
        }
        for &(from, to) in grants {
            host.allow_scope_crossing(scopes[from], scopes[to]);
        }
        *data = staged;
        Ok(Self {
            host,
            requesting: scopes[requesting],
            placements: snapshot,
        })
    }
    pub fn project(
        self,
        plan: &RenderPlan,
        instance: &str,
        limits: &CemValueArtifactLimits,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<RenderPlan, ElementReferenceProjectionError> {
        self.project_with_placements(plan, instance, limits, control, scope)
            .map(|result| result.plan)
    }
    pub fn project_with_placements(
        mut self,
        plan: &RenderPlan,
        instance: &str,
        limits: &CemValueArtifactLimits,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<ElementReferenceProjection, ElementReferenceProjectionError> {
        project_element_reference_ids_with_host_and_placements(
            plan,
            instance,
            limits,
            control,
            scope,
            &mut self
                .host
                .element_reference_host(self.requesting)
                .expect("admitted requester"),
            &self.placements,
        )
    }
}
