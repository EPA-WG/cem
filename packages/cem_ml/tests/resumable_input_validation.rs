#![cfg(not(target_arch = "wasm32"))]
use cem_ml::{
    diagnostics::Diagnostic,
    engine::{
        CemMlEngine, EngineContext, EngineInput, FailLevel, InputFormat, ValidateProjection,
        ValidateRequest,
    },
    real::RealCemMlEngine,
    resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
        ResolverDiagnostic, ResourceResolver,
    },
    schema::{
        document_model::compile_schema_document_model,
        input_validation::resumable::{
            InputValidationProgress, InputValidationResourceCompletion,
            InputValidationResourceRequest, InputValidationSession, OwnedInputValidationRequest,
        },
        input_validation::{InputValidationOutcome, InputValidationRequest, InputValidationStage},
        registry::CEM_ML_SCHEMA_URI,
        uri_loading::{SchemaUriResource, SchemaUriResourceRequest},
    },
    source_map::SourceMapStack,
};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy)]
enum Mode {
    Rounds,
    Empty,
    Duplicate,
    Reused,
    Limited,
    ReadFailure,
    Cancel,
    Memory,
    CumulativeMemory,
}
#[derive(Debug, Default)]
struct Seen {
    cpu: Vec<String>,
    io: Vec<String>,
    resumes: usize,
    control: Option<cem_ml::operation_control::OperationControl>,
}
#[derive(Debug)]
struct Stage {
    mode: Mode,
    seen: Arc<Mutex<Seen>>,
}
impl InputValidationStage for Stage {
    fn validate(
        &self,
        _: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        panic!("native opt-in session must replace the synchronous stage")
    }
    fn start_resumable(
        &self,
        request: OwnedInputValidationRequest,
    ) -> Option<Box<dyn InputValidationSession>> {
        self.seen.lock().unwrap().control = Some(request.execution.control.clone());
        Some(Box::new(Session {
            request,
            mode: self.mode,
            seen: self.seen.clone(),
            round: 0,
            resources: vec![],
        }))
    }
}
#[derive(Debug)]
struct Session {
    request: OwnedInputValidationRequest,
    mode: Mode,
    seen: Arc<Mutex<Seen>>,
    round: u64,
    resources: Vec<SchemaUriResource>,
}
impl InputValidationSession for Session {
    fn advance(
        &mut self,
        completions: Vec<InputValidationResourceCompletion>,
    ) -> Result<InputValidationProgress, Vec<Diagnostic>> {
        assert!(std::thread::current()
            .name()
            .unwrap()
            .starts_with("cem-ml-cpu-"));
        let mut seen = self.seen.lock().unwrap();
        seen.cpu.push(std::thread::current().name().unwrap().into());
        seen.resumes += 1;
        drop(seen);
        let captured = self.request.lexical_scopes.as_ref().unwrap();
        assert!(Arc::ptr_eq(
            captured.document(),
            self.request.source.ast_owner()
        ));
        if self.round > 0 {
            assert_eq!(completions.len(), 1);
            let completion = completions.into_iter().next().unwrap();
            assert_eq!(completion.id, self.round);
            let response = completion.result.map_err(|d| vec![d])?;
            let resource = resource_request(&self.request, self.round)
                .import_response(response, |_, _| unreachable!())
                .map_err(|d| vec![d])?;
            assert!(Arc::ptr_eq(
                resource.targets[0].document(),
                resource.imported.tree.ast_owner()
            ));
            self.resources.push(resource);
        } else {
            assert!(completions.is_empty());
        }
        if matches!(self.mode, Mode::Empty) {
            return Ok(InputValidationProgress::AwaitResources(vec![]));
        }
        if self.round == 2 {
            assert!(!Arc::ptr_eq(
                self.resources[0].targets[0].document(),
                self.resources[1].targets[0].document()
            ));
            return Ok(InputValidationProgress::Finished(InputValidationOutcome {
                complete: true,
                diagnostics: vec![],
            }));
        }
        self.round += 1;
        let requests = if matches!(self.mode, Mode::Reused) {
            vec![1]
        } else if matches!(self.mode, Mode::Duplicate) {
            vec![self.round, self.round]
        } else {
            vec![self.round]
        };
        Ok(InputValidationProgress::AwaitResources(
            requests
                .into_iter()
                .map(|id| InputValidationResourceRequest {
                    id,
                    resource: resource_request(&self.request, id),
                })
                .collect(),
        ))
    }
}
fn resource_request(request: &OwnedInputValidationRequest, id: u64) -> SchemaUriResourceRequest {
    SchemaUriResourceRequest::new(
        &format!("schema{id}.cem"),
        "https://vendor.test/input.cem",
        Some("text/cem-ml"),
        &request.execution.resolver_policy,
        SourceMapStack::default(),
    )
    .unwrap()
}
struct Reader {
    mode: Mode,
    seen: Arc<Mutex<Seen>>,
}
impl ResourceResolver for Reader {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        assert!(std::thread::current()
            .name()
            .unwrap()
            .starts_with("cem-ml-io-"));
        self.seen.lock().unwrap().io.push(request.uri.clone());
        if matches!(self.mode, Mode::Cancel) {
            self.seen
                .lock()
                .unwrap()
                .control
                .as_ref()
                .unwrap()
                .abort_signal()
                .abort();
        }
        if matches!(self.mode, Mode::ReadFailure) {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: request.direction,
            });
        }
        let text =
            "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child}}}";
        Ok(ResolvedRead {
            uri: request.uri.clone(),
            bytes: if matches!(self.mode, Mode::Memory) {
                vec![b' '; 4096]
            } else {
                text.as_bytes().to_vec()
            },
            content_type: Some("text/cem-ml".into()),
        })
    }
    fn write(&self, _: &ResolveRequest, _: &[u8]) -> Result<ResolvedWrite, ResolverDiagnostic> {
        unreachable!()
    }
}
fn run(
    mode: Mode,
) -> (
    cem_ml::engine::EngineResult<cem_ml::engine::ValidateResponse>,
    Arc<Mutex<Seen>>,
) {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let mut context = EngineContext::default();
    context.schema = Some(CEM_ML_SCHEMA_URI.into());
    context.content_type = Some("text/cem-ml".into());
    context.scheduler.max_parallel_documents = Some(1);
    let mut model = compile_schema_document_model(
        "urn:resumable",
        if matches!(mode, Mode::Limited) {
            "{schema | {elements | {element @name=section}} {constraints | {constraint @kind=reference-traversal-work @value=1}}}"
        } else {
            "{schema | {elements | {element @name=section}}}"
        },
    );
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    assert!(model.is_ready_for_validation());
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(Arc::new(Stage {
        mode,
        seen: seen.clone(),
    }));
    context.resolver_registry.register(
        "https",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        Reader {
            mode,
            seen: seen.clone(),
        },
    );
    let mut root_scope = cem_ml::run_config::ScopeConfig::default();
    root_scope.budgets.insert("cpu".into(), "1".into());
    root_scope.budgets.insert("io".into(), "1".into());
    if matches!(mode, Mode::Memory) {
        root_scope.budgets.insert("memory".into(), "1024".into());
    }
    if matches!(mode, Mode::CumulativeMemory) {
        root_scope.budgets.insert("memory".into(), "110".into());
    }
    let response = RealCemMlEngine.validate(ValidateRequest {
        inputs: vec![EngineInput {
            uri: "https://vendor.test/input.cem".into(),
            bytes: b"{section}".to_vec(),
            from_format: Some(InputFormat::Cem),
            identity: None,
            root_scope,
        }],
        projection: ValidateProjection::Cem,
        fail_level: FailLevel::Validate,
        context,
    });
    (response, seen)
}
#[test]
fn typed_reads_release_cpu_and_resume_same_retained_session_for_multiple_rounds() {
    let (response, seen) = run(Mode::Rounds);
    let response = response.unwrap();
    assert!(
        response.report.report_ast.validation.unwrap().complete,
        "{:?}",
        response.report.diagnostics
    );
    let seen = seen.lock().unwrap();
    assert_eq!(seen.resumes, 3);
    let control = seen.control.as_ref().unwrap();
    assert_eq!(control.memory_charged(control.root_scope()).unwrap(), 0);
    assert_eq!(
        seen.io,
        [
            "https://vendor.test/schema1.cem",
            "https://vendor.test/schema2.cem"
        ]
    );
    let events = response.report.report_ast.scheduler_trace.events;
    assert_eq!(
        events
            .iter()
            .filter(|e| e.task.contains("schema-resource")
                && e.kind == cem_ml::scheduler::SchedulerEventKind::Dispatch)
            .count(),
        2
    );
}
#[test]
fn invalid_empty_or_duplicate_yields_do_not_schedule_transport() {
    for mode in [Mode::Empty, Mode::Duplicate] {
        let (response, seen) = run(mode);
        let response = response.unwrap();
        assert!(!response.report.report_ast.validation.unwrap().complete);
        assert!(response
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.schema_validation.invalid_resource_yield"));
        assert!(seen.lock().unwrap().io.is_empty());
    }
}
#[test]
fn failed_read_retains_resource_provenance_and_keeps_validation_incomplete() {
    let (response, seen) = run(Mode::ReadFailure);
    let response = response.unwrap();
    assert!(!response.report.report_ast.validation.unwrap().complete);
    assert!(response.report.diagnostics.iter().any(|d| d.uri.as_deref()
        == Some("https://vendor.test/schema1.cem")
        && d.code.starts_with("cem.resolver.")));
    assert_eq!(seen.lock().unwrap().resumes, 2);
}
#[test]
fn cancellation_during_io_prevents_session_resume() {
    let (response, seen) = run(Mode::Cancel);
    assert!(matches!(
        response,
        Err(cem_ml::engine::EngineError::Cancelled { .. })
    ));
    assert_eq!(seen.lock().unwrap().resumes, 1);
}
#[test]
fn resource_payload_cannot_exceed_document_memory_budget() {
    let (response, seen) = run(Mode::Memory);
    assert!(response.is_err());
    assert_eq!(seen.lock().unwrap().resumes, 1);
}

#[test]
fn resource_identity_and_work_accounting_survive_resume() {
    for (mode, code) in [
        (Mode::Reused, "cem.schema_validation.invalid_resource_yield"),
        (Mode::Limited, "cem.schema_validation.resource_limit"),
    ] {
        let (response, seen) = run(mode);
        let response = response.unwrap();
        assert!(!response.report.report_ast.validation.unwrap().complete);
        assert!(response.report.diagnostics.iter().any(|d| d.code == code));
        assert_eq!(seen.lock().unwrap().io.len(), 1);
    }
}

#[derive(Debug, Default)]
struct Gate {
    state: Mutex<(bool, bool)>,
    changed: std::sync::Condvar,
}
#[derive(Debug)]
struct ProbeStage {
    gate: Arc<Gate>,
    first: Stage,
}
impl InputValidationStage for ProbeStage {
    fn start_resumable(
        &self,
        request: OwnedInputValidationRequest,
    ) -> Option<Box<dyn InputValidationSession>> {
        if request.source.source_uri().ends_with("second.cem") {
            None
        } else {
            self.first.start_resumable(request)
        }
    }
    fn validate(
        &self,
        _: InputValidationRequest<'_>,
    ) -> Result<InputValidationOutcome, Vec<Diagnostic>> {
        assert!(std::thread::current()
            .name()
            .unwrap()
            .starts_with("cem-ml-cpu-"));
        let (mut state, timeout) = self
            .gate
            .changed
            .wait_timeout_while(
                self.gate.state.lock().unwrap(),
                std::time::Duration::from_secs(5),
                |state| !state.0,
            )
            .unwrap();
        assert!(
            !timeout.timed_out(),
            "coordinator must dispatch I/O before waiting for all CPU documents"
        );
        state.1 = true;
        self.gate.changed.notify_all();
        Ok(InputValidationOutcome {
            complete: true,
            diagnostics: vec![],
        })
    }
}
struct ProbeReader {
    gate: Arc<Gate>,
    reader: Reader,
}
impl ResourceResolver for ProbeReader {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        {
            let mut state = self.gate.state.lock().unwrap();
            state.0 = true;
            self.gate.changed.notify_all();
        }
        let (state, timeout) = self
            .gate
            .changed
            .wait_timeout_while(
                self.gate.state.lock().unwrap(),
                std::time::Duration::from_secs(5),
                |state| !state.1,
            )
            .unwrap();
        assert!(
            !timeout.timed_out(),
            "another document must use the sole CPU worker while this read waits"
        );
        drop(state);
        self.reader.read(request)
    }
    fn write(&self, _: &ResolveRequest, _: &[u8]) -> Result<ResolvedWrite, ResolverDiagnostic> {
        unreachable!()
    }
}
#[test]
fn another_document_runs_on_the_only_cpu_worker_while_a_session_awaits_io() {
    let gate = Arc::new(Gate::default());
    let seen = Arc::new(Mutex::new(Seen::default()));
    let mut context = EngineContext::default();
    context.schema = Some(CEM_ML_SCHEMA_URI.into());
    context.content_type = Some("text/cem-ml".into());
    context.scheduler.max_parallel_documents = Some(1);
    let mut model = compile_schema_document_model(
        "urn:probe",
        "{schema | {elements | {element @name=section}}}",
    );
    model.schema_uri = CEM_ML_SCHEMA_URI.into();
    context.schema_document_models.register(model);
    context.input_validation_stage = Some(Arc::new(ProbeStage {
        gate: gate.clone(),
        first: Stage {
            mode: Mode::Rounds,
            seen: seen.clone(),
        },
    }));
    context.resolver_registry.register(
        "https",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        ProbeReader {
            gate,
            reader: Reader {
                mode: Mode::Rounds,
                seen,
            },
        },
    );
    let response = RealCemMlEngine
        .validate(ValidateRequest {
            inputs: ["first.cem", "second.cem"]
                .into_iter()
                .map(|name| EngineInput {
                    uri: format!("https://vendor.test/{name}"),
                    bytes: b"{section}".to_vec(),
                    from_format: Some(InputFormat::Cem),
                    identity: None,
                    root_scope: Default::default(),
                })
                .collect(),
            projection: ValidateProjection::Cem,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap();
    let completion = response.report.report_ast.validation.unwrap();
    assert!(completion.complete);
    assert_eq!(
        completion
            .inputs
            .iter()
            .map(|i| i.input.as_str())
            .collect::<Vec<_>>(),
        [
            "https://vendor.test/first.cem",
            "https://vendor.test/second.cem"
        ]
    );
}

#[test]
fn imported_resource_memory_remains_charged_across_rounds_and_releases_on_failure() {
    let (response, seen) = run(Mode::CumulativeMemory);
    assert!(response.is_err());
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.resumes, 2,
        "first payload fits; retained first owner prevents second payload from fitting"
    );
    let control = seen.control.as_ref().unwrap();
    assert_eq!(control.memory_charged(control.root_scope()).unwrap(), 0);
}
