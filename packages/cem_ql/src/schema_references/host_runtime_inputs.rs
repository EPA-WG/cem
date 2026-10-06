//! Child runtime input preparation, before occurrence-scope installation.
//! Contexts are explicitly supplied; source bindings and scope budgets stay intact.
use super::{CemQlSchemaDeclarationHost, DeclarationScope, SchemaHostRegionPreparation};
use crate::api::StandaloneExpressionContext;
use cem_ml::schema::reference_policy::{
    ReferencePolicyError, ReferenceScopePolicy, ReferenceScopePolicyOverrides,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaHostRuntimeInputIssue {
    SchemaNotReady,
    UnregisteredHost,
    ForeignPreparation,
    ContextNotReady,
    InvalidPolicy(ReferencePolicyError),
    UnrelatedOccurrenceScope,
}

/// Immutable inputs for one selected child schema. Readiness means the inputs
/// are usable, not that a child scope has been installed or its body evaluated.
/// Repeat schema selection and input preparation when lifecycle inputs change.
#[derive(Debug, Clone)]
pub struct SchemaHostRuntimeInputs {
    region: SchemaHostRegionPreparation,
    enclosing_scope: Option<DeclarationScope>,
    context: Option<StandaloneExpressionContext>,
    policy: Option<ReferenceScopePolicy>,
    issue: Option<SchemaHostRuntimeInputIssue>,
}
impl SchemaHostRuntimeInputs {
    pub fn region(&self) -> &SchemaHostRegionPreparation {
        &self.region
    }
    pub fn enclosing_scope(&self) -> Option<DeclarationScope> {
        self.enclosing_scope
    }
    pub fn context(&self) -> Option<&StandaloneExpressionContext> {
        self.context.as_ref()
    }
    pub fn policy(&self) -> Option<&ReferenceScopePolicy> {
        self.policy.as_ref()
    }
    pub fn issue(&self) -> Option<&SchemaHostRuntimeInputIssue> {
        self.issue.as_ref()
    }
    pub fn is_ready(&self) -> bool {
        self.issue.is_none()
            && self.region.is_ready()
            && self.enclosing_scope.is_some()
            && self.context.is_some()
            && self.policy.is_some()
    }
}

impl CemQlSchemaDeclarationHost {
    /// Derive the selected child's policy from the original host's enclosing
    /// policy. Missing context remains pending even when an enclosing context is
    /// available. Invalid policy retains its original error without fallback.
    /// This accepts a snapshot selected by this host; it neither reruns selection
    /// nor allocates scopes, rebinds occurrences, grants crossings or evaluates
    /// body references. Occurrence-policy precedence is a separate handoff rule.
    pub fn prepare_schema_host_runtime_inputs(
        &self,
        region: SchemaHostRegionPreparation,
        context: Option<StandaloneExpressionContext>,
    ) -> SchemaHostRuntimeInputs {
        let enclosing_scope = self.source_scope(region.contract.host());
        let mut inputs = SchemaHostRuntimeInputs {
            region,
            enclosing_scope,
            context,
            policy: None,
            issue: None,
        };
        if !inputs.region.is_ready() {
            inputs.issue = Some(SchemaHostRuntimeInputIssue::SchemaNotReady);
            return inputs;
        }
        let Some(enclosing) = enclosing_scope.and_then(|scope| self.scope_record(scope)) else {
            inputs.issue = Some(SchemaHostRuntimeInputIssue::UnregisteredHost);
            return inputs;
        };
        let prepared = inputs.region.preparation.as_ref().unwrap();
        if prepared.selection.nodes.iter().any(|node| {
            node.scope
                .and_then(|scope| self.scope_record(scope))
                .is_none()
        }) {
            inputs.issue = Some(SchemaHostRuntimeInputIssue::ForeignPreparation);
            return inputs;
        }
        match enclosing.policy.for_scope(prepared.model.as_ref().unwrap()) {
            Ok(policy) => {
                inputs.policy = Some(policy);
                if inputs.context.is_none() {
                    inputs.issue = Some(SchemaHostRuntimeInputIssue::ContextNotReady);
                }
            }
            Err(error) => {
                inputs.issue = Some(SchemaHostRuntimeInputIssue::InvalidPolicy(error));
            }
        }
        inputs
    }
}

/// Registered child frame, still awaiting an explicit source/body assignment.
/// Original captured frames and their contexts are never rewritten.
#[derive(Debug, Clone)]
pub struct SchemaHostRuntimeScope {
    inputs: SchemaHostRuntimeInputs,
    scope: DeclarationScope,
}
impl SchemaHostRuntimeScope {
    pub fn inputs(&self) -> &SchemaHostRuntimeInputs {
        &self.inputs
    }
    pub fn scope(&self) -> DeclarationScope {
        self.scope
    }
}
impl CemQlSchemaDeclarationHost {
    /// Allocate an immutable lexical frame from ready explicit inputs. It shares
    /// the enclosing relationship boundary, not new crossing authority. Source
    /// assignment remains separate, so failure cannot rebind an occurrence.
    pub fn register_schema_host_runtime_scope(
        &mut self,
        inputs: SchemaHostRuntimeInputs,
    ) -> Result<SchemaHostRuntimeScope, SchemaHostRuntimeInputIssue> {
        if let Some(issue) = inputs.issue() {
            return Err(issue.clone());
        }
        if !inputs.is_ready() {
            return Err(SchemaHostRuntimeInputIssue::SchemaNotReady);
        }
        let parent = inputs.enclosing_scope.unwrap();
        if self.scope_record(parent).is_none() {
            return Err(SchemaHostRuntimeInputIssue::ForeignPreparation);
        }
        let local = ReferenceScopePolicyOverrides::from_schema(
            inputs
                .region
                .preparation
                .as_ref()
                .unwrap()
                .model
                .as_ref()
                .unwrap(),
        )
        .map_err(SchemaHostRuntimeInputIssue::InvalidPolicy)?;
        let scope = self
            .register_lexical_scope_with_policy_overrides(parent, inputs.context.clone(), local)
            .expect("checked parent scope belongs to this host");
        Ok(SchemaHostRuntimeScope { inputs, scope })
    }

    /// Replay declarations between the original enclosing and occurrence frames
    /// onto a selected child frame. Context is explicitly supplied, including None
    /// for pending inputs; an available child context is never substituted.
    /// Independent relationship roots require their own explicit handoff.
    /// This allocates a new frame without assigning source or editing old frames.
    pub fn register_schema_host_occurrence_scope(
        &mut self,
        child: &SchemaHostRuntimeScope,
        original: DeclarationScope,
        context: Option<StandaloneExpressionContext>,
    ) -> Result<DeclarationScope, SchemaHostRuntimeInputIssue> {
        if self.scope_record(child.scope).is_none() {
            return Err(SchemaHostRuntimeInputIssue::ForeignPreparation);
        }
        let enclosing = child.inputs.enclosing_scope.unwrap();
        let mut current = original;
        let mut path = vec![];
        while current != enclosing {
            let Some(record) = self.scope_record(current) else {
                return Err(SchemaHostRuntimeInputIssue::ForeignPreparation);
            };
            path.push(record.local_policy.clone());
            let Some(parent) = record.lexical_parent else {
                return Err(SchemaHostRuntimeInputIssue::UnrelatedOccurrenceScope);
            };
            current = parent;
        }
        let mut local = ReferenceScopePolicyOverrides::default();
        for overrides in path.into_iter().rev() {
            local = overrides.overlay(&local);
        }
        Ok(self
            .register_lexical_scope_with_policy_overrides(child.scope, context, local)
            .expect("checked child scope belongs to this host"))
    }
}
