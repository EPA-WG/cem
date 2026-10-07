//! Opt-in CEM-QL session for entered schema URI dependencies. The embedding
//! runtime supplies contexts, public exports, local lexical handoff and grants;
//! the engine coordinator supplies transport and execution resource accounting.
use crate::{
    api::StandaloneExpressionContext,
    schema_references::{
        CemQlSchemaDeclarationHost, DeclarationScope, NamespaceLifecycleSnapshot,
        SchemaHostRuntimeContextRequest, SchemaHostRuntimeValidation, SchemaUriLoadTicket,
        SchemaUriLoadedScope,
    },
};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    import::ScopedCemImport,
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        input_validation::{
            resumable::{
                InputValidationProgress, InputValidationResourceCompletion,
                InputValidationResourceRequest, InputValidationSession,
                OwnedInputValidationRequest,
            },
            InputValidationOutcome,
        },
        namespace_references::{NamespaceLexicalSnapshot, NamespaceNameCompletion},
        reference_policy::{ReferenceScopePolicy, ReferenceScopePolicyOverrides},
        scope_controls::{SchemaHostControl, SchemaHostSource},
        uri_loading::SchemaUriResource,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    sync::Arc,
};

/// Runtime-owned lifecycle inputs. Missing contexts remain pending. Preparing a
/// loaded owner must explicitly attach any local lexical contexts and directed
/// grants before the next validation invocation. No default crossing is granted.
pub trait SchemaValidationSessionInputs: Debug + Send {
    /// Explicitly opt in; existing sessions retain their previous behavior.
    fn namespace_lifecycle_enabled(&self) -> bool {
        false
    }
    /// Runtime inputs for an original occurrence after its captured namespace
    /// dependencies are ready. None finishes that region incomplete; the caller
    /// can supply new inputs in another execution rather than awaiting host data.
    fn namespace_context(
        &mut self,
        _source: &SchemaDeclarationNode,
        _snapshot: &NamespaceLexicalSnapshot,
        _enclosing: DeclarationScope,
        _completion: &Arc<NamespaceNameCompletion>,
    ) -> (
        Option<StandaloneExpressionContext>,
        ReferenceScopePolicyOverrides,
    ) {
        (None, ReferenceScopePolicyOverrides::default())
    }
    /// Ready names remain original-owner execution views. The runtime may retain
    /// this completion for shared query ingress or subsequent context preparation.
    fn namespace_inspected(&mut self, _snapshot: &NamespaceLifecycleSnapshot) {}
    fn runtime_context(
        &mut self,
        request: SchemaHostRuntimeContextRequest<'_>,
    ) -> Option<StandaloneExpressionContext>;
    fn loaded_context(
        &mut self,
        resource: &SchemaUriResource,
    ) -> Option<StandaloneExpressionContext>;
    fn public_exports(
        &mut self,
        imported: &ScopedCemImport,
        part: &str,
    ) -> Result<Vec<SchemaDeclarationNode>, String>;
    /// Explicit MIME hint for transports without response metadata (for example
    /// local files). No filename or URI-suffix type inference is performed.
    fn resource_content_type_hint(&mut self, _control: &SchemaHostControl) -> Option<String> {
        None
    }
    /// Inherit the requesting policy unless the embedding runtime supplies an
    /// explicit destination policy. Selected schema overrides apply at consumption.
    fn loaded_policy(
        &mut self,
        _control: &SchemaHostControl,
        inherited: &ReferenceScopePolicy,
    ) -> ReferenceScopePolicy {
        inherited.clone()
    }
    fn prepare_loaded(
        &mut self,
        host: &mut CemQlSchemaDeclarationHost,
        control: &SchemaHostControl,
        loaded: &SchemaUriLoadedScope,
    ) -> Result<(), Vec<Diagnostic>>;
    /// Optional inspection of each invocation, including incomplete snapshots.
    fn inspected(&mut self, _report: &SchemaHostRuntimeValidation) {}
}
#[derive(Debug)]
pub struct CemQlInputValidationSession<I: SchemaValidationSessionInputs> {
    request: OwnedInputValidationRequest,
    host: CemQlSchemaDeclarationHost,
    inputs: I,
    tickets: BTreeMap<u64, (SchemaHostControl, SchemaUriLoadTicket, ReferenceScopePolicy)>,
    attempted: BTreeSet<(usize, u32, String)>,
    next_id: u64,
    diagnostics: Vec<Diagnostic>,
}
impl<I: SchemaValidationSessionInputs> CemQlInputValidationSession<I> {
    /// The host must already retain the original input owner, its source-position
    /// names and any original local occurrence scopes. Context construction and
    /// registration remain explicit runtime work, not parser/engine defaults.
    pub fn new(
        request: OwnedInputValidationRequest,
        host: CemQlSchemaDeclarationHost,
        inputs: I,
    ) -> Self {
        Self {
            request,
            host,
            inputs,
            tickets: BTreeMap::new(),
            attempted: BTreeSet::new(),
            next_id: 0,
            diagnostics: vec![],
        }
    }
    pub fn host(&self) -> &CemQlSchemaDeclarationHost {
        &self.host
    }
    /// Host-only preparation of staged destinations before resuming. Mutations
    /// never establish authority from resource bytes or transport metadata.
    pub fn host_mut(&mut self) -> &mut CemQlSchemaDeclarationHost {
        &mut self.host
    }
    /// Settle one correlated resource without invoking validation. The original
    /// imported arena is returned for explicit host context/grant preparation.
    pub fn stage_resource(
        &mut self,
        completion: InputValidationResourceCompletion,
    ) -> Result<Option<SchemaUriLoadedScope>, Vec<Diagnostic>> {
        self.check_active()?;
        let Some((control, ticket, inherited)) = self.tickets.remove(&completion.id) else {
            return Err(vec![
                self.error("Unknown or repeated schema resource completion")
            ]);
        };
        let policy = self.inputs.loaded_policy(&control, &inherited);
        let inputs = std::cell::RefCell::new(&mut self.inputs);
        match self.host.complete_schema_uri_resource(
            &ticket,
            completion.result,
            policy,
            |resource| inputs.borrow_mut().loaded_context(resource),
            |imported, part| inputs.borrow_mut().public_exports(imported, part),
        ) {
            Ok(loaded) => {
                self.inputs
                    .prepare_loaded(&mut self.host, &control, &loaded)?;
                Ok(Some(loaded))
            }
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                Ok(None)
            }
        }
    }
    fn check_active(&self) -> Result<(), Vec<Diagnostic>> {
        self.request
            .execution
            .control
            .check_scope(self.request.execution.scope)
            .map_err(|error| vec![self.error(error.to_string())])
    }
    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            code: "cem.schema_validation.uri_session_failed".into(),
            severity: Severity::Error,
            uri: Some(self.request.source.source_uri().into()),
            message: message.into(),
            ..Default::default()
        }
    }
}
impl<I: SchemaValidationSessionInputs> InputValidationSession for CemQlInputValidationSession<I> {
    fn advance(
        &mut self,
        completions: Vec<InputValidationResourceCompletion>,
    ) -> Result<InputValidationProgress, Vec<Diagnostic>> {
        self.check_active()?;
        if completions.len() != self.tickets.len() {
            return Err(vec![
                self.error("Incomplete schema resource completion batch")
            ]);
        }
        // Validate correlation before import/context callbacks for any result.
        let mut ids = BTreeSet::new();
        if completions
            .iter()
            .any(|c| !self.tickets.contains_key(&c.id) || !ids.insert(c.id))
        {
            return Err(vec![
                self.error("Unknown or duplicate schema resource completion")
            ]);
        }
        for completion in completions {
            self.stage_resource(completion)?;
        }
        self.check_active()?;
        let Some(CemAstNode::Document { root_children, .. }) = self.request.source.ast().get(0)
        else {
            return Err(vec![self.error("Input source is not a retained document")]);
        };
        let mut namespace_complete = true;
        let mut namespace_diagnostics = vec![];
        let report = if self.inputs.namespace_lifecycle_enabled() {
            let captured = self.request.lexical_scopes.clone().ok_or_else(|| {
                vec![self.error("Namespace lifecycle requires original lexical capture")]
            })?;
            let inputs = std::cell::RefCell::new(&mut self.inputs);
            let request = &self.request;
            let (snapshot, validation) = self
                .host
                .with_namespace_lifecycle(
                    captured,
                    root_children,
                    request.policy.limits,
                    |source, snapshot, enclosing, completion| {
                        inputs
                            .borrow_mut()
                            .namespace_context(source, snapshot, enclosing, completion)
                    },
                    |host, snapshot| {
                        inputs.borrow_mut().namespace_inspected(snapshot);
                        host.validate_input_runtime_host_regions_with_behavior_evaluator(
                            &request.model.schema_uri,
                            request.source.clone(),
                            &snapshot.ready_roots,
                            &request.model,
                            request.policy.limits,
                            |context| inputs.borrow_mut().runtime_context(context),
                            request.behavior_evaluator.as_deref(),
                        )
                    },
                )
                .map_err(|error| vec![self.error(format!("Namespace lifecycle: {error:?}"))])?;
            namespace_complete = snapshot.is_complete();
            for property in &snapshot.properties {
                if let Some(preparation) = &property.preparation {
                    namespace_diagnostics.extend(preparation.selection.diagnostics.iter().cloned());
                }
            }
            validation
        } else {
            let inputs = &mut self.inputs;
            self.host
                .validate_input_runtime_host_regions_with_behavior_evaluator(
                    &self.request.model.schema_uri,
                    self.request.source.clone(),
                    root_children,
                    &self.request.model,
                    self.request.policy.limits,
                    |request| inputs.runtime_context(request),
                    self.request.behavior_evaluator.as_deref(),
                )
        }
        .map_err(|error| vec![self.error(error.to_string())])?;
        self.check_active()?;
        self.inputs.inspected(&report);
        let mut requests = vec![];
        for entered in &report.inputs {
            self.check_active()?;
            let region = entered.region();
            // A published pending/invalid result (or missing context) is not an
            // invitation to restart its acquisition or borrow a previous model.
            if region.preparation.is_some() {
                continue;
            }
            let Some(control) = region.contract.control() else {
                continue;
            };
            let SchemaHostSource::Uri(uri) = &control.source else {
                continue;
            };
            let key = (
                Arc::as_ptr(control.host.document()) as usize,
                control.host.node_id(),
                uri.clone(),
            );
            if !self.attempted.insert(key) {
                continue;
            }
            let Some(tree) = self.host.source_tree(&control.host) else {
                self.diagnostics.push(
                    self.error("Entered schema control has no original retained source owner"),
                );
                continue;
            };
            let base_url = tree.source_uri().to_owned();
            let Some(inherited) = entered
                .enclosing_scope()
                .and_then(|scope| self.host.reference_scope_policy(scope))
                .cloned()
            else {
                self.diagnostics.push(
                    self.error("Entered schema control has no effective requesting scope policy"),
                );
                continue;
            };
            let hint = self.inputs.resource_content_type_hint(control);
            let ticket = match self.host.begin_schema_uri_resource(
                control,
                &base_url,
                hint.as_deref(),
                &self.request.execution.resolver_policy,
            ) {
                Ok(ticket) => ticket,
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    continue;
                }
            };
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or_else(|| vec![self.error("Schema request identity space exhausted")])?;
            requests.push(InputValidationResourceRequest {
                id: self.next_id,
                resource: ticket.request().clone(),
            });
            self.tickets
                .insert(self.next_id, (control.clone(), ticket, inherited));
        }
        if !requests.is_empty() {
            return Ok(InputValidationProgress::AwaitResources(requests));
        }
        let mut diagnostics = report.validation.diagnostics;
        for diagnostic in namespace_diagnostics {
            if !diagnostics.contains(&diagnostic) {
                diagnostics.push(diagnostic);
            }
        }
        for diagnostic in &self.diagnostics {
            if !diagnostics.contains(diagnostic) {
                diagnostics.push(diagnostic.clone());
            }
        }
        Ok(InputValidationProgress::Finished(InputValidationOutcome {
            complete: namespace_complete
                && report.validation.complete
                && self.diagnostics.is_empty(),
            diagnostics,
        }))
    }
}
