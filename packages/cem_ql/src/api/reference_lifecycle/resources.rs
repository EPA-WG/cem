//! Host-driven resource staging over the same native resumable coordinator.
use super::*;
use crate::schema_references::{
    validation_session::{CemQlInputValidationSession, SchemaValidationSessionInputs},
    SchemaHostRuntimeValidation, SchemaUriLoadedScope,
};
use cem_ml::{
    import::ScopedCemImport,
    operation_control::{MemoryPermit, OperationControl, ROOT_EXECUTION_SCOPE_ID},
    resolver::{ResolvedRead, ResolverPolicy},
    schema::{
        input_validation::resumable::{
            InputValidationExecution, InputValidationProgress, InputValidationResourceCompletion,
            InputValidationSession, OwnedInputValidationRequest,
        },
        namespace_references::{NamespaceLexicalSnapshot, NamespaceNameCompletion},
        scope_controls::SchemaHostControl,
        uri_loading::SchemaUriResource,
    },
};
use std::sync::Mutex;
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceResourceRequest {
    pub id: u64,
    pub uri: String,
    pub content_type_hint: Option<String>,
    pub public_part: Option<String>,
    pub source_map: SourceMapStack,
}
#[derive(Debug, serde::Serialize)]
#[serde(tag = "state", content = "result", rename_all = "camelCase")]
pub enum ReferenceResourceProgress {
    AwaitResources(Vec<ReferenceResourceRequest>),
    Finished(ReferenceConsumerReport),
}
#[derive(Debug, Clone)]
pub struct ReferenceLoadedSource {
    pub index: usize,
    pub source: RetainedReferenceSource,
}
#[derive(Debug)]
struct LoadedInputs {
    inputs: SourceInputs,
    scopes: Vec<(AstNodeId, DeclarationScope)>,
    names: Arc<NamespaceNameCompletion>,
    name_issues: Vec<(AstNodeId, String)>,
}
#[derive(Debug, Default)]
struct State {
    loaded: Vec<LoadedInputs>,
    namespace: Option<Arc<NamespaceLifecycleSnapshot>>,
    validation: Option<SchemaHostRuntimeValidation>,
}
#[derive(Debug)]
struct Inputs {
    config: ReferenceValidationSession,
    state: Arc<Mutex<State>>,
}
impl Inputs {
    fn context(&self, node: &SchemaDeclarationNode) -> Option<StandaloneExpressionContext> {
        self.config.context_for(node).or_else(|| {
            self.state
                .lock()
                .unwrap()
                .loaded
                .iter()
                .find(|s| {
                    Arc::ptr_eq(
                        s.inputs.source.ingress().source().ast_owner(),
                        node.document(),
                    )
                })
                .and_then(|s| s.inputs.context_for(node.node_id()))
        })
    }
}
impl SchemaValidationSessionInputs for Inputs {
    fn namespace_lifecycle_enabled(&self) -> bool {
        true
    }
    fn namespace_context(
        &mut self,
        node: &SchemaDeclarationNode,
        _: &NamespaceLexicalSnapshot,
        _: DeclarationScope,
        _: &Arc<NamespaceNameCompletion>,
    ) -> (
        Option<StandaloneExpressionContext>,
        ReferenceScopePolicyOverrides,
    ) {
        (self.context(node), Default::default())
    }
    fn namespace_inspected(&mut self, snapshot: &NamespaceLifecycleSnapshot) {
        // Snapshot contains native handles; no source arena or evaluated graph is copied.
        self.state.lock().unwrap().namespace = Some(Arc::new(NamespaceLifecycleSnapshot {
            completion: snapshot.completion.clone(),
            ready_roots: snapshot.ready_roots.clone(),
            incomplete_roots: snapshot.incomplete_roots.clone(),
            properties: snapshot.properties.clone(),
            pending_properties: snapshot.pending_properties.clone(),
            scopes: snapshot.scopes.clone(),
            work_used: snapshot.work_used,
            work_exhausted: snapshot.work_exhausted,
        }));
    }
    fn runtime_context(
        &mut self,
        request: SchemaHostRuntimeContextRequest<'_>,
    ) -> Option<StandaloneExpressionContext> {
        self.context(match request {
            SchemaHostRuntimeContextRequest::Body(r) => r.contract.host(),
            SchemaHostRuntimeContextRequest::Occurrence { source, .. } => source,
        })
    }
    fn loaded_context(&mut self, _: &SchemaUriResource) -> Option<StandaloneExpressionContext> {
        None
    }
    fn public_exports(
        &mut self,
        _: &ScopedCemImport,
        _: &str,
    ) -> Result<Vec<SchemaDeclarationNode>, String> {
        Err("No explicit public parts adapter was supplied".into())
    }
    fn prepare_loaded(
        &mut self,
        host: &mut CemQlSchemaDeclarationHost,
        _: &SchemaHostControl,
        loaded: &SchemaUriLoadedScope,
    ) -> Result<(), Vec<Diagnostic>> {
        // Destination preparation is a separate explicit host operation after
        // staging. The coordinator has already retained its captured names.
        let _ = (host, loaded);
        Ok(())
    }
    fn inspected(&mut self, report: &SchemaHostRuntimeValidation) {
        self.state.lock().unwrap().validation = Some(report.clone());
    }
}
#[derive(Debug)]
pub struct ReferenceResourceExecution {
    config: ReferenceValidationSession,
    revision: Arc<AtomicU64>,
    expected_revision: u64,
    coordinator: Option<CemQlInputValidationSession<Inputs>>,
    state: Arc<Mutex<State>>,
    control: OperationControl,
    pending: BTreeSet<u64>,
    requested: usize,
    max_work: usize,
    report: Option<ReferenceConsumerReport>,
    finished: bool,
    permits: Vec<MemoryPermit>,
}
impl ReferenceValidationSession {
    /// Freeze requesting inputs. A subsequent parent mutation invalidates this
    /// execution; disposing its parent or any transport handles does not.
    pub fn start_resources(
        &self,
        resolver_policy: ResolverPolicy,
    ) -> Result<ReferenceResourceExecution, String> {
        let (report, prepared) = self.prepare()?;
        let state = Arc::new(Mutex::new(State::default()));
        let control = OperationControl::default();
        let max_work = prepared
            .as_ref()
            .map(|(_, policy)| policy.limits.max_work)
            .unwrap_or(
                ReferenceScopePolicy::schema_defaults()
                    .unwrap()
                    .limits
                    .max_work,
            );
        let coordinator = if let Some((model, policy)) = prepared {
            let request = OwnedInputValidationRequest {
                source: self.sources[0].source.ingress().source().clone(),
                lexical_scopes: Some(self.sources[0].source.require_lexical().unwrap().clone()),
                model,
                root_scope: Default::default(),
                policy: policy.clone(),
                behavior_evaluator: None,
                execution: InputValidationExecution {
                    control: control.clone(),
                    scope: ROOT_EXECUTION_SCOPE_ID,
                    resolver_policy,
                    retained_input_bytes: 0,
                },
            };
            Some(CemQlInputValidationSession::new(
                request,
                self.host(&policy)?,
                Inputs {
                    config: self.clone(),
                    state: state.clone(),
                },
            ))
        } else {
            None
        };
        Ok(ReferenceResourceExecution {
            config: self.clone(),
            revision: self.revision.clone(),
            expected_revision: self.revision.load(Ordering::Relaxed),
            coordinator,
            state,
            control,
            pending: BTreeSet::new(),
            requested: 0,
            max_work,
            report: Some(report),
            finished: false,
            permits: vec![],
        })
    }
}
impl ReferenceResourceExecution {
    fn check(&self) -> Result<(), String> {
        if self.finished {
            return Err("Resource execution is finished".into());
        }
        if self.revision.load(Ordering::Relaxed) != self.expected_revision {
            return Err("Requesting contexts or authority changed; begin a new execution".into());
        }
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.to_string())
    }
    pub fn cancel(&mut self) {
        self.control.abort_signal().abort();
    }
    pub fn advance(&mut self) -> Result<ReferenceResourceProgress, String> {
        self.check()?;
        if !self.pending.is_empty() {
            return Err("Resource completion batch is still pending".into());
        }
        let Some(coordinator) = &mut self.coordinator else {
            self.finished = true;
            return Ok(ReferenceResourceProgress::Finished(
                self.report.take().unwrap(),
            ));
        };
        let progress = coordinator.advance(vec![]).map_err(|d| format!("{d:?}"))?;
        self.check()?;
        let coordinator = self.coordinator.as_ref().unwrap();
        match progress {
            InputValidationProgress::AwaitResources(requests) => {
                self.requested = self
                    .requested
                    .checked_add(requests.len())
                    .ok_or("Resource work exhausted")?;
                if self.requested > self.max_work {
                    self.cancel();
                    return Err("Resource work exhausted".into());
                }
                let requests = requests
                    .into_iter()
                    .map(|r| {
                        self.pending.insert(r.id);
                        ReferenceResourceRequest {
                            id: r.id,
                            uri: r.resource.request().uri.clone(),
                            content_type_hint: r.resource.request().content_type_hint.clone(),
                            public_part: r.resource.public_part().map(str::to_owned),
                            source_map: r.resource.source_map().clone(),
                        }
                    })
                    .collect();
                Ok(ReferenceResourceProgress::AwaitResources(requests))
            }
            InputValidationProgress::Finished(outcome) => {
                self.finished = true;
                let mut state = self.state.lock().unwrap();
                let mut report = self.report.take().unwrap();
                let namespace = state.namespace.clone();
                if let (Some(namespace), Some(validation)) = (&namespace, state.validation.take()) {
                    ReferenceValidationSession::namespace_dependencies(
                        &mut report,
                        namespace,
                        &self.config.sources[0].source,
                    );
                    let mut reporting = self.config.clone();
                    reporting
                        .sources
                        .extend(state.loaded.iter().map(|loaded| loaded.inputs.clone()));
                    report = reporting.validation_report(
                        report,
                        namespace,
                        coordinator.host(),
                        validation,
                    );
                }
                report.complete &= outcome.complete;
                report.diagnostics.extend(
                    outcome
                        .diagnostics
                        .into_iter()
                        .filter(|d| !report.diagnostics.contains(d))
                        .collect::<Vec<_>>(),
                );
                report.failed |= report
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation());
                Ok(ReferenceResourceProgress::Finished(report))
            }
        }
    }
    pub fn complete(
        &mut self,
        id: u64,
        response: Result<ResolvedRead, Diagnostic>,
    ) -> Result<Option<ReferenceLoadedSource>, String> {
        self.complete_with_exports(id, response, |_, _| {
            Err("No explicit public parts adapter was supplied".into())
        })
    }
    pub fn complete_with_exports<F>(
        &mut self,
        id: u64,
        response: Result<ResolvedRead, Diagnostic>,
        exports: F,
    ) -> Result<Option<ReferenceLoadedSource>, String>
    where
        F: FnOnce(&ScopedCemImport, &str) -> Result<Vec<SchemaDeclarationNode>, String>,
    {
        self.check()?;
        if !self.pending.contains(&id) {
            return Err("Unknown or repeated resource completion".into());
        }
        if response
            .as_ref()
            .is_ok_and(|r| r.bytes.len() > cem_ml::ast::reload::ReloadLimits::default().max_bytes)
        {
            return Err("Resource byte limit exceeded".into());
        }
        let bytes = response.as_ref().ok().map(|r| r.bytes.clone());
        let permit = if let Some(bytes) = &bytes {
            Some(
                self.control
                    .charge_memory(ROOT_EXECUTION_SCOPE_ID, bytes.len() as u64, None)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let coordinator = self.coordinator.as_mut().ok_or("No resource coordinator")?;
        let result = coordinator.stage_resource_with_exports(
            InputValidationResourceCompletion {
                id,
                result: response,
            },
            exports,
        );
        self.pending.remove(&id);
        self.check()?;
        let Some(loaded) = result.map_err(|d| format!("{d:?}"))? else {
            return Ok(None);
        };
        let source = RetainedReferenceSource::from_capture(
            loaded.resource.imported.captured.clone(),
            loaded.resource.imported.tree.source_uri(),
            &bytes.unwrap(),
            Default::default(),
        )
        .map_err(|e| {
            self.cancel();
            e.to_string()
        })?;
        let coordinator = self.coordinator.as_mut().unwrap();
        let mut state = self.state.lock().unwrap();
        let index = self.config.sources.len() + state.loaded.len();
        let scopes = coordinator
            .host_mut()
            .attach_captured_lexical_scopes_with_policy_overrides(
                &loaded.resource.imported.captured,
                |_, _, _| (None, Default::default()),
            )
            .map_err(|e| format!("{e:?}"))?;
        let mut ready_roots = vec![];
        let mut name_issues = vec![];
        for node in roots(&source) {
            match NamespaceNameCompletion::new(
                source.require_lexical().unwrap().clone(),
                &[node],
                Default::default(),
            ) {
                Ok(_) => ready_roots.push(node),
                Err(error) => name_issues.push((node, format!("{error:?}"))),
            }
        }
        let names = Arc::new(
            NamespaceNameCompletion::new(
                source.require_lexical().unwrap().clone(),
                &ready_roots,
                Default::default(),
            )
            .map_err(|e| format!("{e:?}"))?,
        );
        state.loaded.push(LoadedInputs {
            inputs: SourceInputs {
                source: source.clone(),
                context: None,
                occurrence_contexts: Default::default(),
                policy: None,
                limits: None,
            },
            scopes: std::iter::once((0, loaded.scope)).chain(scopes).collect(),
            names,
            name_issues,
        });
        if let Some(permit) = permit {
            self.permits.push(permit);
        }
        Ok(Some(ReferenceLoadedSource { index, source }))
    }
    pub fn set_loaded_context(
        &mut self,
        index: usize,
        context: Option<StandaloneExpressionContext>,
    ) -> Result<(), String> {
        self.check()?;
        if !self.pending.is_empty() {
            return Err("Complete the resource batch before preparing contexts".into());
        }
        let mut state = self.state.lock().unwrap();
        let loaded = state
            .loaded
            .get_mut(
                index
                    .checked_sub(self.config.sources.len())
                    .ok_or("Only loaded contexts may change")?,
            )
            .ok_or("Unknown loaded source")?;
        loaded.inputs.context = context;
        for (node, scope) in &loaded.scopes {
            self.coordinator
                .as_mut()
                .unwrap()
                .host_mut()
                .set_context(*scope, loaded.inputs.context_for(*node));
        }
        Ok(())
    }
    pub fn set_loaded_occurrence_value_context(
        &mut self,
        index: usize,
        value: &crate::eval::Item,
        context: Option<StandaloneExpressionContext>,
    ) -> Result<(), String> {
        let node = self.loaded_occurrence_node(index, value)?;
        self.set_loaded_occurrence_context(index, node, context)
    }
    pub fn clear_loaded_occurrence_value_context(
        &mut self,
        index: usize,
        value: &crate::eval::Item,
    ) -> Result<(), String> {
        let node = self.loaded_occurrence_node(index, value)?;
        self.clear_loaded_occurrence_context(index, node)
    }
    fn loaded_occurrence_node(
        &self,
        index: usize,
        value: &crate::eval::Item,
    ) -> Result<AstNodeId, String> {
        let node = crate::eval::retained_cem_node(value)
            .ok_or("An original native source occurrence is required")?;
        let state = self.state.lock().unwrap();
        let loaded = state
            .loaded
            .get(
                index
                    .checked_sub(self.config.sources.len())
                    .ok_or("Only loaded contexts may change")?,
            )
            .ok_or("Unknown loaded source")?;
        if !Arc::ptr_eq(
            loaded.inputs.source.ingress().source().ast_owner(),
            node.owner().ast_owner(),
        ) {
            return Err("Occurrence belongs to another source owner".into());
        }
        Ok(node.node_id())
    }
    pub fn set_loaded_occurrence_context(
        &mut self,
        index: usize,
        node: AstNodeId,
        context: Option<StandaloneExpressionContext>,
    ) -> Result<(), String> {
        self.update_loaded_occurrence_context(index, node, Some(context))
    }
    pub fn clear_loaded_occurrence_context(
        &mut self,
        index: usize,
        node: AstNodeId,
    ) -> Result<(), String> {
        self.update_loaded_occurrence_context(index, node, None)
    }
    fn update_loaded_occurrence_context(
        &mut self,
        index: usize,
        node: AstNodeId,
        context: Option<Option<StandaloneExpressionContext>>,
    ) -> Result<(), String> {
        self.check()?;
        if !self.pending.is_empty() {
            return Err("Complete the resource batch before preparing contexts".into());
        }
        let mut state = self.state.lock().unwrap();
        let loaded = state
            .loaded
            .get_mut(
                index
                    .checked_sub(self.config.sources.len())
                    .ok_or("Only loaded contexts may change")?,
            )
            .ok_or("Unknown loaded source")?;
        if loaded
            .inputs
            .source
            .ingress()
            .source()
            .ast()
            .get(node)
            .is_none()
        {
            return Err("Unknown original node handle".into());
        }
        if let Some(context) = context {
            loaded.inputs.occurrence_contexts.insert(node, context);
        } else {
            loaded.inputs.occurrence_contexts.remove(&node);
        }
        for (node, scope) in &loaded.scopes {
            self.coordinator
                .as_mut()
                .unwrap()
                .host_mut()
                .set_context(*scope, loaded.inputs.context_for(*node));
        }
        Ok(())
    }
    /// Query a saved completion without import, selection or expression evaluation.
    /// Index 0 is the latest input namespace snapshot; loaded indices retain the
    /// captured ready name forest from import. Pending imported names stay excluded.
    pub fn query_snapshot(
        &self,
        index: usize,
    ) -> Result<super::query_snapshot::ReferenceQuerySnapshot, String> {
        let state = self.state.lock().unwrap();
        if index == 0 {
            return super::query_snapshot::ReferenceQuerySnapshot::from_lifecycle(
                self.config.sources[0].source.clone(),
                state
                    .namespace
                    .as_ref()
                    .ok_or("Advance execution before requesting a saved completion")?,
            );
        }
        let loaded = state
            .loaded
            .get(
                index
                    .checked_sub(self.config.sources.len())
                    .ok_or("No saved completion for this source")?,
            )
            .ok_or("Unknown loaded source")?;
        let mut report = ReferenceConsumerReport::pending();
        report.complete = loaded.name_issues.is_empty();
        report.placements = loaded.names.roots().len();
        for (node, reason) in &loaded.name_issues {
            report.dependencies.push(ReferenceConsumerDependency {
                kind: ReferenceConsumerDependencyKind::NamespaceNotReady,
                reason: reason.clone(),
                source_uri: loaded.inputs.source.ingress().source().source_uri().into(),
                node_id: Some(*node),
                source_map: loaded
                    .inputs
                    .source
                    .ingress()
                    .source()
                    .ast()
                    .get(*node)
                    .map(|n| node_source_map(n).clone()),
            });
        }
        super::query_snapshot::ReferenceQuerySnapshot::from_completion(
            loaded.inputs.source.clone(),
            loaded.names.clone(),
            report,
        )
    }
    pub fn allow_crossing(&mut self, from: usize, to: usize) -> Result<(), String> {
        self.check()?;
        if !self.pending.is_empty() {
            return Err("Complete the resource batch before granting crossings".into());
        }
        let state = self.state.lock().unwrap();
        let host = self
            .coordinator
            .as_mut()
            .ok_or("No resource coordinator")?
            .host_mut();
        let scope = |index: usize| -> Result<DeclarationScope, String> {
            if let Some(source) = self.config.sources.get(index) {
                host.source_scope(
                    &SchemaDeclarationNode::new(
                        source.source.ingress().source().ast_owner().clone(),
                        0,
                    )
                    .unwrap(),
                )
                .ok_or("Unknown original scope".into())
            } else {
                state
                    .loaded
                    .get(index - self.config.sources.len())
                    .map(|s| s.scopes[0].1)
                    .ok_or("Unknown loaded scope".into())
            }
        };
        let (from, to) = (scope(from)?, scope(to)?);
        host.allow_scope_crossing(from, to);
        Ok(())
    }
}
