//! Typed suspension boundary for native engine validation. Session code owns
//! context preparation, loader generations, import, grants and readiness retries.
//! The coordinator owns transport scheduling and operation resource accounting.
use super::{normalize, InputValidationOutcome};
use crate::{
    diagnostics::Diagnostic,
    operation_control::{ExecutionScopeId, MemoryPermit, OperationControl},
    parser::tree::RetainedCemTree,
    resolver::{ResolvedRead, ResolverPolicy},
    run_config::ScopeConfig,
    schema::{
        document_model::{SchemaBehaviorEvaluator, SchemaDocumentModel},
        machine::LexicallyScopedDocument,
        reference_policy::ReferenceScopePolicy,
        uri_loading::SchemaUriResourceRequest,
    },
};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

#[derive(Debug, Clone)]
pub struct InputValidationExecution {
    /// The actual engine document scope, shared across all load/resume rounds.
    pub control: OperationControl,
    pub scope: ExecutionScopeId,
    pub resolver_policy: ResolverPolicy,
    /// Original input byte retention; imported resource bytes are charged separately.
    pub retained_input_bytes: usize,
}
#[derive(Debug, Clone)]
pub struct OwnedInputValidationRequest {
    pub source: Arc<RetainedCemTree>,
    pub lexical_scopes: Option<Arc<LexicallyScopedDocument>>,
    pub model: Arc<SchemaDocumentModel>,
    pub root_scope: ScopeConfig,
    pub policy: ReferenceScopePolicy,
    pub behavior_evaluator: Option<Arc<dyn SchemaBehaviorEvaluator>>,
    pub execution: InputValidationExecution,
}
#[derive(Debug)]
pub struct InputValidationResourceRequest {
    /// Session-local correlation token, not an authored node/context ID. Use a
    /// fresh token for each acquisition, including retries of the same control.
    pub id: u64,
    pub resource: SchemaUriResourceRequest,
}
#[derive(Debug)]
pub struct InputValidationResourceCompletion {
    pub id: u64,
    /// Original loader provenance, including failures; importing stays in session code.
    pub result: Result<ResolvedRead, Diagnostic>,
}
#[derive(Debug)]
pub enum InputValidationProgress {
    Finished(InputValidationOutcome),
    /// Yield at least one new request. Only entered controls should be requested.
    AwaitResources(Vec<InputValidationResourceRequest>),
}
/// Every call runs on CPU. The first receives an empty completion vector; later
/// calls receive the complete preceding batch in its authored request order.
/// No callback runs while waiting for I/O. Keep native owners and current runtime
/// contexts in the session; never persist evaluated targets in the source tree.
pub trait InputValidationSession: Debug + Send {
    fn advance(
        &mut self,
        completions: Vec<InputValidationResourceCompletion>,
    ) -> Result<InputValidationProgress, Vec<Diagnostic>>;
}

#[derive(Debug, Default)]
pub(crate) struct InputValidationRun {
    pub complete: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub pending: Option<PendingInputValidation>,
}
impl InputValidationRun {
    pub fn finished(outcome: InputValidationOutcome) -> Self {
        Self {
            complete: outcome.complete,
            diagnostics: outcome.diagnostics,
            pending: None,
        }
    }
}
#[derive(Debug)]
pub(crate) struct PendingInputValidation {
    session: Box<dyn InputValidationSession>,
    /// Keep original owner/metadata and execution capabilities for the full session,
    /// even if a custom session only retains a subset of its request.
    pub request: OwnedInputValidationRequest,
    pub resources: Vec<InputValidationResourceRequest>,
    seen: BTreeSet<u64>,
    permits: Vec<MemoryPermit>,
    uri: String,
}
impl PendingInputValidation {
    pub fn start(
        session: Box<dyn InputValidationSession>,
        request: OwnedInputValidationRequest,
        uri: &str,
    ) -> InputValidationRun {
        let permit = match request.execution.control.charge_memory(
            request.execution.scope,
            request.execution.retained_input_bytes as u64,
            None,
        ) {
            Ok(permit) => permit,
            Err(error) => {
                return InputValidationRun::finished(normalize(
                    Err(vec![super::failure(uri, error.to_string())]),
                    uri,
                ))
            }
        };
        Self {
            session,
            request,
            resources: vec![],
            seen: BTreeSet::new(),
            permits: vec![permit],
            uri: uri.into(),
        }
        .resume(vec![], vec![])
    }
    pub fn resume(
        mut self,
        completions: Vec<InputValidationResourceCompletion>,
        permits: Vec<MemoryPermit>,
    ) -> InputValidationRun {
        self.permits.extend(permits);
        self.resources.clear();
        let progress = self.session.advance(completions);
        match progress {
            Ok(InputValidationProgress::Finished(outcome)) => {
                InputValidationRun::finished(normalize(Ok(outcome), &self.uri))
            }
            Err(diagnostics) => {
                InputValidationRun::finished(normalize(Err(diagnostics), &self.uri))
            }
            Ok(InputValidationProgress::AwaitResources(resources)) => {
                let mut batch = BTreeSet::new();
                let valid = !resources.is_empty()
                    && resources.iter().all(|resource| {
                        !self.seen.contains(&resource.id) && batch.insert(resource.id)
                    });
                // Admission is additionally bounded by the effective requesting
                // scope's work cap for the entire session, not reset on resume.
                // The consumer still owns its graph traversal/destination budgets.
                let within_limit = self.seen.len().saturating_add(batch.len())
                    <= self.request.policy.limits.max_work;
                if !valid || !within_limit {
                    let mut diagnostic = super::failure(
                        &self.uri,
                        if valid {
                            "Resource requests exceed the session work bound"
                        } else {
                            "A resource yield must contain fresh, unique request IDs and make progress"
                        },
                    );
                    diagnostic.code = if valid {
                        "cem.schema_validation.resource_limit"
                    } else {
                        "cem.schema_validation.invalid_resource_yield"
                    }
                    .into();
                    return InputValidationRun::finished(normalize(
                        Err(vec![diagnostic]),
                        &self.uri,
                    ));
                }
                self.seen.extend(batch);
                self.resources = resources;
                InputValidationRun {
                    complete: false,
                    diagnostics: vec![],
                    pending: Some(self),
                }
            }
        }
    }
}
