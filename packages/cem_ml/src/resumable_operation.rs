//! Coordinator-owned resumable transform/query operation planning.
//!
//! The coordinator retains the continuation and deterministic commit order.
//! Physical workers receive only stateless, versioned work packets containing
//! owned JSON metadata plus bounded transfer descriptors.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::diagnostics::Diagnostic;
use crate::engine::{
    CemMlEngine, EngineContext, EngineInput, FormatIdentity, InputFormat, TemplateInput,
    TransformExecutionPolicy, TransformRequest, TransformRuntimePhase, TransformSchedulerScopeIds,
    TransformTemplateEntrypoint, TransformTemplateKind,
};
use crate::operation_control::{ExecutionScopeId, OperationId, TaskId};
use crate::operation_handle::{OperationTerminalStatus, OperationTerminalSummary};
use crate::query::{
    QueryExportFormat, QueryExportRequest, QueryOwnedBindings, QueryResultExporterRegistry,
    QueryRunError, QueryRunRequest, QuerySource,
};
use crate::real::RealCemMlEngine;
use crate::run_config::ScopeConfig;
use crate::validation::css_selector::register_css_selector_query_exporters;
use crate::validation::xpath::register_xpath_query_exporters;
use crate::worker_control::{
    OperationWorkDomain, OperationWorkPacket, OperationWorkResult, OperationWorkResultStatus,
    OperationWorkStage, TransferBufferDescriptor, WorkerAddress, WorkerCoordinator,
    WorkerCoordinatorError, WorkerEnvelope, WorkerSlotId, MAX_WORK_INLINE_PAYLOAD_BYTES,
    MAX_WORK_STAGE_LABEL_BYTES, WORK_PACKET_PROTOCOL_VERSION,
};
#[cfg(feature = "debug-control")]
use crate::worker_control::{
    WorkerStopDisposition, WorkerStopGeneration, WorkerStopRendezvousStatus,
};

pub const MAX_RESUMABLE_OPERATIONS: usize = 1_024;
pub const MAX_WORK_PACKETS_PER_POLL: u32 = 64;

const DATA_PREPARE_STAGE: &str = "data-prepare";
const TEMPLATE_PREPARE_STAGE: &str = "template-prepare";
const QUERY_PREPARE_STAGE: &str = "query-prepare";
const EXECUTE_STAGE: &str = "execute";
const FINALIZE_STAGE: &str = "finalize";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationSource {
    pub uri: String,
    pub bytes: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_format: Option<InputFormat>,
    pub identity: FormatIdentity,
    #[serde(default)]
    pub root_scope: ScopeConfig,
}

impl OperationSource {
    fn normalized_scope(&self) -> ScopeConfig {
        let mut scope = self.root_scope.clone();
        if scope.default_content_type.is_none() {
            scope.default_content_type = self.identity.content_type.clone();
        }
        if scope.schema.is_none() {
            scope.schema = self.identity.schema.clone();
        }
        scope
    }

    fn engine_input(&self) -> EngineInput {
        EngineInput {
            uri: self.uri.clone(),
            bytes: self.bytes.clone(),
            from_format: self.from_format,
            identity: Some(self.identity.clone()),
            root_scope: self.normalized_scope(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ResumableRunRequest {
    Transform {
        data: OperationSource,
        template: OperationSource,
        #[serde(default)]
        params: BTreeMap<String, Value>,
        #[serde(default)]
        template_entrypoint: TransformTemplateEntrypoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<FormatIdentity>,
        #[serde(default)]
        target_scope: ScopeConfig,
        #[serde(default)]
        preserve_source_offsets: bool,
    },
    Query {
        data: OperationSource,
        query: OperationSource,
    },
}

impl ResumableRunRequest {
    pub fn domain(&self) -> OperationWorkDomain {
        match self {
            Self::Transform { .. } => OperationWorkDomain::Transform,
            Self::Query { .. } => OperationWorkDomain::Query,
        }
    }

    fn initial_work(&self) -> Result<VecDeque<PlannedWork>, ResumableOperationError> {
        let (first_label, first_source, second_label, second_source) = match self {
            Self::Transform { data, template, .. } => {
                (DATA_PREPARE_STAGE, data, TEMPLATE_PREPARE_STAGE, template)
            }
            Self::Query { data, query } => (DATA_PREPARE_STAGE, data, QUERY_PREPARE_STAGE, query),
        };
        validate_source(first_source)?;
        validate_source(second_source)?;
        Ok(VecDeque::from([
            PlannedWork::new(
                self.domain(),
                1,
                first_label,
                serde_json::to_value(first_source).map_err(serialization_error)?,
            ),
            PlannedWork::new(
                self.domain(),
                2,
                second_label,
                serde_json::to_value(second_source).map_err(serialization_error)?,
            ),
        ]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResumableOperationState {
    Running,
    PauseRequested,
    Paused,
    Stepping,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableOperationTerminal {
    pub status: OperationTerminalStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ResumableOperationError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableOperationError {
    pub code: String,
    pub message: String,
}

impl ResumableOperationError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ResumableOperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ResumableOperationError {}

impl From<WorkerCoordinatorError> for ResumableOperationError {
    fn from(error: WorkerCoordinatorError) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableOperationStart {
    pub operation_id: OperationId,
    pub state: ResumableOperationState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableOperationPoll {
    pub operation_id: OperationId,
    pub state: ResumableOperationState,
    pub packets: Vec<OperationWorkPacket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<ResumableOperationTerminal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableOperationResultAcceptance {
    pub operation_id: OperationId,
    pub state: ResumableOperationState,
    pub staged: bool,
    pub committed_task_ids: Vec<TaskId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<ResumableOperationTerminal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableWorkerReplacement {
    pub previous: WorkerAddress,
    pub replacement: WorkerAddress,
    pub affected_operation_ids: Vec<OperationId>,
    pub retry_packets: Vec<OperationWorkPacket>,
}

#[derive(Debug, Clone)]
struct PlannedWork {
    stage: OperationWorkStage,
    payload: Value,
    transfers: Vec<TransferBufferDescriptor>,
}

impl PlannedWork {
    fn new(domain: OperationWorkDomain, ordinal: u32, label: &str, payload: Value) -> Self {
        Self {
            stage: OperationWorkStage {
                domain,
                ordinal,
                label: label.to_owned(),
            },
            payload,
            transfers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ResumableOperationDriver {
    request: ResumableRunRequest,
    state: ResumableOperationState,
    ready: VecDeque<PlannedWork>,
    prepared: BTreeSet<String>,
    execute_dispatched: bool,
    finalize_dispatched: bool,
    next_task_id: u64,
    next_worker: usize,
    step_task: Option<TaskId>,
    #[cfg(feature = "debug-control")]
    step_stop_generation: Option<WorkerStopGeneration>,
    terminal: Option<ResumableOperationTerminal>,
}

impl ResumableOperationDriver {
    fn new(request: ResumableRunRequest) -> Result<Self, ResumableOperationError> {
        let ready = request.initial_work()?;
        Ok(Self {
            request,
            state: ResumableOperationState::Running,
            ready,
            prepared: BTreeSet::new(),
            execute_dispatched: false,
            finalize_dispatched: false,
            next_task_id: 1,
            next_worker: 0,
            step_task: None,
            #[cfg(feature = "debug-control")]
            step_stop_generation: None,
            terminal: None,
        })
    }

    fn required_prepare_count(&self) -> usize {
        2
    }

    fn queue_execute(&mut self) -> Result<(), ResumableOperationError> {
        if self.execute_dispatched || self.prepared.len() != self.required_prepare_count() {
            return Ok(());
        }
        self.execute_dispatched = true;
        self.ready.push_back(PlannedWork::new(
            self.request.domain(),
            3,
            EXECUTE_STAGE,
            serde_json::to_value(&self.request).map_err(serialization_error)?,
        ));
        Ok(())
    }

    fn apply_committed(
        &mut self,
        result: &OperationWorkResult,
    ) -> Result<Option<ResumableOperationTerminal>, ResumableOperationError> {
        match result.status {
            OperationWorkResultStatus::Failed => {
                return Ok(Some(ResumableOperationTerminal {
                    status: OperationTerminalStatus::Failed,
                    result: None,
                    error: Some(error_from_payload(&result.payload)),
                    reason: None,
                }));
            }
            OperationWorkResultStatus::Cancelled => {
                return Ok(Some(ResumableOperationTerminal {
                    status: OperationTerminalStatus::Cancelled,
                    result: None,
                    error: None,
                    reason: result
                        .payload
                        .get("reason")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                }));
            }
            OperationWorkResultStatus::Succeeded => {}
        }

        match result.stage.label.as_str() {
            DATA_PREPARE_STAGE | TEMPLATE_PREPARE_STAGE | QUERY_PREPARE_STAGE => {
                self.prepared.insert(result.stage.label.clone());
                self.queue_execute()?;
            }
            EXECUTE_STAGE => {
                if self.finalize_dispatched {
                    return Err(ResumableOperationError::new(
                        "cem.operation.finalize_duplicate",
                        "operation execute result attempted to queue finalize twice",
                    ));
                }
                self.finalize_dispatched = true;
                self.ready.push_back(PlannedWork::new(
                    self.request.domain(),
                    4,
                    FINALIZE_STAGE,
                    result.payload.clone(),
                ));
            }
            FINALIZE_STAGE => {
                return Ok(Some(ResumableOperationTerminal {
                    status: OperationTerminalStatus::Succeeded,
                    result: Some(result.payload.clone()),
                    error: None,
                    reason: None,
                }));
            }
            label => {
                return Err(ResumableOperationError::new(
                    "cem.operation.stage_unknown",
                    format!("unknown committed work stage `{label}`"),
                ));
            }
        }
        Ok(None)
    }
}

/// Common coordinator/continuation owner. Hosts drive it through explicit
/// start, bounded poll, result acceptance, and control calls.
#[derive(Debug, Clone)]
pub struct ResumableOperationHost {
    coordinator: WorkerCoordinator,
    workers: Vec<WorkerAddress>,
    operations: BTreeMap<OperationId, ResumableOperationDriver>,
    next_operation_id: u64,
}

impl ResumableOperationHost {
    pub fn new(worker_count: u16) -> Result<Self, ResumableOperationError> {
        let mut coordinator = WorkerCoordinator::new(worker_count)?;
        let mut workers = Vec::with_capacity(usize::from(worker_count));
        for slot in 1..=worker_count {
            let worker = coordinator.worker(WorkerSlotId::from_raw(u64::from(slot)))?;
            coordinator.accept(&WorkerEnvelope::new(
                worker,
                1,
                crate::operation_handle::OperationHostEnvelope::initialize(json!({
                    "runtime": "resumable-operation-host"
                })),
                Vec::new(),
            ))?;
            coordinator.mark_ready(worker)?;
            workers.push(worker);
        }
        Ok(Self {
            coordinator,
            workers,
            operations: BTreeMap::new(),
            next_operation_id: 1,
        })
    }

    pub fn workers(&self) -> &[WorkerAddress] {
        &self.workers
    }

    pub fn start(
        &mut self,
        request: ResumableRunRequest,
    ) -> Result<ResumableOperationStart, ResumableOperationError> {
        if self.operations.len() >= MAX_RESUMABLE_OPERATIONS {
            return Err(ResumableOperationError::new(
                "cem.operation.limit",
                format!("live operation count exceeds {MAX_RESUMABLE_OPERATIONS}"),
            ));
        }
        let operation = OperationId::from_raw(self.next_operation_id);
        self.next_operation_id = self.next_operation_id.checked_add(1).ok_or_else(|| {
            ResumableOperationError::new(
                "cem.operation.identity_exhausted",
                "operation identity space is exhausted",
            )
        })?;
        let driver = ResumableOperationDriver::new(request)?;
        self.coordinator.register_operation(operation)?;
        for worker in &self.workers {
            self.coordinator.assign_worker(operation, *worker)?;
        }
        self.operations.insert(operation, driver);
        Ok(ResumableOperationStart {
            operation_id: operation,
            state: ResumableOperationState::Running,
        })
    }

    pub fn poll(
        &mut self,
        operation: OperationId,
        max_packets: u32,
    ) -> Result<ResumableOperationPoll, ResumableOperationError> {
        if max_packets == 0 || max_packets > MAX_WORK_PACKETS_PER_POLL {
            return Err(ResumableOperationError::new(
                "cem.operation.poll_budget",
                format!("maxPackets={max_packets} is outside 1..={MAX_WORK_PACKETS_PER_POLL}"),
            ));
        }
        let (coordinator, operations, workers) =
            (&mut self.coordinator, &mut self.operations, &self.workers);
        let driver = operations.get_mut(&operation).ok_or_else(|| {
            ResumableOperationError::new(
                "cem.operation.unknown",
                format!("unknown operation {operation}"),
            )
        })?;
        if driver.state == ResumableOperationState::Terminal {
            return Ok(ResumableOperationPoll {
                operation_id: operation,
                state: driver.state,
                packets: Vec::new(),
                terminal: driver.terminal.clone(),
            });
        }
        if matches!(
            driver.state,
            ResumableOperationState::PauseRequested | ResumableOperationState::Paused
        ) {
            return Ok(ResumableOperationPoll {
                operation_id: operation,
                state: driver.state,
                packets: Vec::new(),
                terminal: None,
            });
        }

        let limit = if driver.state == ResumableOperationState::Stepping {
            1
        } else {
            max_packets
        };
        let mut packets = Vec::new();
        while packets.len() < limit as usize {
            let Some(work) = driver.ready.pop_front() else {
                break;
            };
            let task = TaskId::from_raw(driver.next_task_id);
            driver.next_task_id = driver.next_task_id.checked_add(1).ok_or_else(|| {
                ResumableOperationError::new(
                    "cem.operation.task_identity_exhausted",
                    "operation task identity space is exhausted",
                )
            })?;
            let worker = workers[driver.next_worker % workers.len()];
            driver.next_worker = driver.next_worker.wrapping_add(1);
            let packet = coordinator.dispatch_work(
                operation,
                task,
                ExecutionScopeId::from_raw(0),
                worker,
                work.stage,
                work.payload,
                work.transfers,
            )?;
            if driver.state == ResumableOperationState::Stepping {
                driver.step_task = Some(task);
            }
            packets.push(packet);
        }
        Ok(ResumableOperationPoll {
            operation_id: operation,
            state: driver.state,
            packets,
            terminal: None,
        })
    }

    pub fn accept_result(
        &mut self,
        result: OperationWorkResult,
    ) -> Result<ResumableOperationResultAcceptance, ResumableOperationError> {
        let operation = result.operation_id;
        let acceptance = self.coordinator.accept_work_result(result)?;
        let driver = self.operations.get_mut(&operation).ok_or_else(|| {
            ResumableOperationError::new(
                "cem.operation.unknown",
                format!("unknown operation {operation}"),
            )
        })?;
        let mut terminal = None;
        let mut stepped_task_committed = false;
        for committed in &acceptance.committed {
            stepped_task_committed |= driver.step_task == Some(committed.task_id);
            if let Some(outcome) = driver.apply_committed(committed)? {
                terminal = Some(outcome);
                break;
            }
        }
        if let Some(outcome) = terminal.clone() {
            self.publish_terminal(operation, outcome)?;
        } else if stepped_task_committed {
            driver.step_task = None;
            #[cfg(feature = "debug-control")]
            if let Some(generation) = driver.step_stop_generation.take() {
                driver.state = ResumableOperationState::PauseRequested;
                self.coordinator.begin_stop(operation, generation)?;
            }
        }
        let driver = self
            .operations
            .get(&operation)
            .expect("accepted result operation remains registered");
        Ok(ResumableOperationResultAcceptance {
            operation_id: operation,
            state: driver.state,
            staged: acceptance.staged,
            committed_task_ids: acceptance
                .committed
                .into_iter()
                .map(|result| result.task_id)
                .collect(),
            terminal: driver.terminal.clone(),
        })
    }

    pub fn cancel(
        &mut self,
        operation: OperationId,
        reason: Option<String>,
    ) -> Result<ResumableOperationTerminal, ResumableOperationError> {
        if let Some(terminal) = self
            .operations
            .get(&operation)
            .and_then(|driver| driver.terminal.clone())
        {
            return Ok(terminal);
        }
        self.coordinator.cancel_operation_work(operation)?;
        let terminal = ResumableOperationTerminal {
            status: OperationTerminalStatus::Cancelled,
            result: None,
            error: None,
            reason,
        };
        self.publish_terminal(operation, terminal.clone())?;
        Ok(terminal)
    }

    #[cfg(feature = "debug-control")]
    pub fn pause(
        &mut self,
        operation: OperationId,
        generation: WorkerStopGeneration,
    ) -> Result<WorkerStopRendezvousStatus, ResumableOperationError> {
        let driver = self.driver_mut(operation)?;
        if driver.state != ResumableOperationState::Running {
            return Err(invalid_state(operation, driver.state, "pause"));
        }
        driver.state = ResumableOperationState::PauseRequested;
        Ok(self.coordinator.begin_stop(operation, generation)?)
    }

    #[cfg(feature = "debug-control")]
    pub fn acknowledge_stop(
        &mut self,
        operation: OperationId,
        generation: WorkerStopGeneration,
        worker: WorkerAddress,
        disposition: WorkerStopDisposition,
    ) -> Result<WorkerStopRendezvousStatus, ResumableOperationError> {
        let status =
            self.coordinator
                .acknowledge_stop(operation, generation, worker, disposition)?;
        if matches!(status, WorkerStopRendezvousStatus::Complete { .. }) {
            self.driver_mut(operation)?.state = ResumableOperationState::Paused;
        }
        Ok(status)
    }

    #[cfg(feature = "debug-control")]
    pub fn continue_operation(
        &mut self,
        operation: OperationId,
        generation: WorkerStopGeneration,
    ) -> Result<(), ResumableOperationError> {
        let state = self.driver(operation)?.state;
        if state != ResumableOperationState::Paused {
            return Err(invalid_state(operation, state, "continue"));
        }
        self.coordinator.clear_stop(operation, generation)?;
        self.driver_mut(operation)?.state = ResumableOperationState::Running;
        Ok(())
    }

    #[cfg(feature = "debug-control")]
    pub fn step(
        &mut self,
        operation: OperationId,
        current_generation: WorkerStopGeneration,
        next_generation: WorkerStopGeneration,
    ) -> Result<(), ResumableOperationError> {
        let driver = self.driver(operation)?;
        if driver.state != ResumableOperationState::Paused {
            return Err(invalid_state(operation, driver.state, "step"));
        }
        if driver.ready.is_empty() {
            return Err(ResumableOperationError::new(
                "cem.operation.step_unavailable",
                format!("operation {operation} has no ready work to step"),
            ));
        }
        self.coordinator.clear_stop(operation, current_generation)?;
        let driver = self.driver_mut(operation)?;
        driver.state = ResumableOperationState::Stepping;
        driver.step_stop_generation = Some(next_generation);
        Ok(())
    }

    pub fn terminal(
        &self,
        operation: OperationId,
    ) -> Result<Option<&ResumableOperationTerminal>, ResumableOperationError> {
        Ok(self.driver(operation)?.terminal.as_ref())
    }

    /// Advance a physical slot generation and immediately initialize its
    /// replacement. Non-terminal operations retain their logical tasks and
    /// receive retry packets with incremented attempts.
    pub fn replace_worker(
        &mut self,
        slot: WorkerSlotId,
    ) -> Result<ResumableWorkerReplacement, ResumableOperationError> {
        let replacement = self.coordinator.replace_worker(slot)?;
        let worker = replacement.replacement;
        self.coordinator.accept(&WorkerEnvelope::new(
            worker,
            1,
            crate::operation_handle::OperationHostEnvelope::initialize(json!({
                "runtime": "resumable-operation-host-replacement"
            })),
            Vec::new(),
        ))?;
        self.coordinator.mark_ready(worker)?;
        let index = usize::try_from(slot.get().saturating_sub(1)).map_err(|_| {
            ResumableOperationError::new(
                "cem.worker.slot_invalid",
                format!("worker slot {slot} cannot index the host worker table"),
            )
        })?;
        let host_worker = self.workers.get_mut(index).ok_or_else(|| {
            ResumableOperationError::new(
                "cem.worker.slot_unknown",
                format!("unknown worker slot {slot}"),
            )
        })?;
        *host_worker = worker;

        let affected_operation_ids = replacement
            .affected_operations
            .iter()
            .copied()
            .collect::<Vec<_>>();
        for operation in &affected_operation_ids {
            if self.coordinator.terminal(*operation)?.is_none() {
                self.coordinator.assign_worker(*operation, worker)?;
            }
        }
        let mut retry_packets = Vec::new();
        for invalidated in replacement.invalidated_work {
            if self
                .coordinator
                .terminal(invalidated.operation_id)?
                .is_none()
            {
                retry_packets.push(self.coordinator.retry_work(
                    invalidated.operation_id,
                    invalidated.task_id,
                    worker,
                )?);
            }
        }
        Ok(ResumableWorkerReplacement {
            previous: replacement.previous,
            replacement: worker,
            affected_operation_ids,
            retry_packets,
        })
    }

    fn publish_terminal(
        &mut self,
        operation: OperationId,
        terminal: ResumableOperationTerminal,
    ) -> Result<(), ResumableOperationError> {
        let summary = terminal_summary(&terminal);
        let claim = self.coordinator.claim_terminal(operation, summary)?;
        let driver = self.driver_mut(operation)?;
        if claim.published {
            driver.terminal = Some(terminal);
        }
        driver.state = ResumableOperationState::Terminal;
        Ok(())
    }

    fn driver(
        &self,
        operation: OperationId,
    ) -> Result<&ResumableOperationDriver, ResumableOperationError> {
        self.operations.get(&operation).ok_or_else(|| {
            ResumableOperationError::new(
                "cem.operation.unknown",
                format!("unknown operation {operation}"),
            )
        })
    }

    fn driver_mut(
        &mut self,
        operation: OperationId,
    ) -> Result<&mut ResumableOperationDriver, ResumableOperationError> {
        self.operations.get_mut(&operation).ok_or_else(|| {
            ResumableOperationError::new(
                "cem.operation.unknown",
                format!("unknown operation {operation}"),
            )
        })
    }
}

/// Execute one stateless packet in a helper runtime. Invalid envelope metadata
/// is returned as a host-fatal error; transform/query failures are encoded as a
/// typed failed result so deterministic commit order is preserved.
pub fn execute_operation_work(
    packet: OperationWorkPacket,
) -> Result<OperationWorkResult, ResumableOperationError> {
    validate_packet(&packet)?;
    let outcome = match packet.stage.label.as_str() {
        DATA_PREPARE_STAGE | TEMPLATE_PREPARE_STAGE | QUERY_PREPARE_STAGE => {
            execute_prepare(&packet.payload)
        }
        EXECUTE_STAGE => execute_request_payload(&packet.payload),
        FINALIZE_STAGE => Ok(packet.payload.clone()),
        label => Err(ResumableOperationError::new(
            "cem.operation.stage_unknown",
            format!("unknown work stage `{label}`"),
        )),
    };
    let (status, payload) = match outcome {
        Ok(payload) => (OperationWorkResultStatus::Succeeded, payload),
        Err(error) => (OperationWorkResultStatus::Failed, json!({ "error": error })),
    };
    Ok(OperationWorkResult {
        work_protocol_version: WORK_PACKET_PROTOCOL_VERSION,
        operation_id: packet.operation_id,
        task_id: packet.task_id,
        scope_id: packet.scope_id,
        worker: packet.worker,
        attempt: packet.attempt,
        commit_sequence: packet.commit_sequence,
        stage: packet.stage,
        status,
        payload,
        transfers: Vec::new(),
    })
}

pub fn execute_request(request: &ResumableRunRequest) -> Result<Value, ResumableOperationError> {
    match request {
        ResumableRunRequest::Transform {
            data,
            template,
            params,
            template_entrypoint,
            target,
            target_scope,
            preserve_source_offsets,
        } => execute_transform(
            data,
            template,
            params,
            template_entrypoint,
            target,
            target_scope,
            *preserve_source_offsets,
        ),
        ResumableRunRequest::Query { data, query } => execute_query(data, query),
    }
}

fn execute_prepare(payload: &Value) -> Result<Value, ResumableOperationError> {
    let source: OperationSource =
        serde_json::from_value(payload.clone()).map_err(deserialization_error)?;
    validate_source(&source)?;
    Ok(json!({
        "uri": source.uri,
        "byteLength": source.bytes.len(),
        "digest": blake3::hash(&source.bytes).to_hex().to_string(),
    }))
}

fn execute_request_payload(payload: &Value) -> Result<Value, ResumableOperationError> {
    let request: ResumableRunRequest =
        serde_json::from_value(payload.clone()).map_err(deserialization_error)?;
    execute_request(&request)
}

#[allow(clippy::too_many_arguments)]
fn execute_transform(
    data: &OperationSource,
    template: &OperationSource,
    params: &BTreeMap<String, Value>,
    template_entrypoint: &TransformTemplateEntrypoint,
    target: &Option<FormatIdentity>,
    requested_target_scope: &ScopeConfig,
    preserve_source_offsets: bool,
) -> Result<Value, ResumableOperationError> {
    validate_source(data)?;
    validate_source(template)?;
    let context = EngineContext::default();
    let template_kind = crate::engine::classify_transform_template_identity_with_registry(
        &template.identity,
        &context.template_adapter_registry,
    )
    .map_err(|error| ResumableOperationError::new(error.code, error.message))?;
    let runtime_phase = match template_kind {
        TransformTemplateKind::Xslt => TransformRuntimePhase::XsltParity,
        TransformTemplateKind::CemNative => TransformRuntimePhase::CemNativeModules,
        TransformTemplateKind::CemQlExpression => TransformRuntimePhase::CemQlExpression,
        TransformTemplateKind::XPath => TransformRuntimePhase::XPath,
    };
    let mut target_scope = requested_target_scope.clone();
    if target_scope.default_content_type.is_none() {
        target_scope.default_content_type = target
            .as_ref()
            .and_then(|identity| identity.content_type.clone());
    }
    if target_scope.schema.is_none() {
        target_scope.schema = target.as_ref().and_then(|identity| identity.schema.clone());
    }
    let response = RealCemMlEngine::new()
        .transform(TransformRequest {
            data: data.engine_input(),
            template: TemplateInput {
                uri: template.uri.clone(),
                bytes: template.bytes.clone(),
                identity: Some(template.identity.clone()),
                root_scope: template.normalized_scope(),
            },
            template_kind,
            template_entrypoint: template_entrypoint.clone(),
            params: params.clone(),
            preserve_source_offsets,
            context,
            target: target.clone(),
            target_scope,
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 1,
                template_load: 2,
                execution: 3,
                output: 4,
            },
            execution_policy: TransformExecutionPolicy {
                runtime_phase,
                ..TransformExecutionPolicy::default()
            },
        })
        .map_err(|error| {
            ResumableOperationError::new("cem.operation.transform_failed", error.to_string())
        })?;
    serde_json::to_value(response).map_err(serialization_error)
}

fn execute_query(
    data: &OperationSource,
    query: &OperationSource,
) -> Result<Value, ResumableOperationError> {
    validate_source(data)?;
    validate_source(query)?;
    let response = crate::query::run_query(QueryRunRequest {
        data: data.engine_input(),
        query: QuerySource {
            uri: query.uri.clone(),
            bytes: query.bytes.clone(),
            identity: query.identity.clone(),
        },
        context: EngineContext::default(),
        context_item: None,
        bindings: QueryOwnedBindings::new(),
        limits: None,
    })
    .map_err(query_error)?;
    let mut exporters = QueryResultExporterRegistry::new();
    register_css_selector_query_exporters(&mut exporters);
    register_xpath_query_exporters(&mut exporters);
    let output = exporters
        .export(
            QueryExportFormat::Json,
            QueryExportRequest {
                result: &response.result,
                no_color: true,
            },
        )
        .map_err(|message| ResumableOperationError::new("cem.operation.query_export", message))?;
    let result = serde_json::from_slice::<Value>(&output.bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&output.bytes).into_owned()));
    Ok(json!({
        "language": response.language.as_str(),
        "inputs": response.inputs,
        "contentType": output.content_type,
        "result": result,
        "diagnostics": response.diagnostics,
    }))
}

fn validate_source(source: &OperationSource) -> Result<(), ResumableOperationError> {
    if source.uri.is_empty() || source.uri.chars().any(char::is_control) {
        return Err(ResumableOperationError::new(
            "cem.operation.source_uri",
            "operation source URI must be non-empty and contain no control characters",
        ));
    }
    if source.identity.content_type.is_none() && source.identity.schema.is_none() {
        return Err(ResumableOperationError::new(
            "cem.operation.source_identity",
            format!(
                "operation source `{}` has no content type or schema",
                source.uri
            ),
        ));
    }
    Ok(())
}

fn validate_packet(packet: &OperationWorkPacket) -> Result<(), ResumableOperationError> {
    if packet.work_protocol_version != WORK_PACKET_PROTOCOL_VERSION {
        return Err(ResumableOperationError::new(
            "cem.worker.work_protocol_version",
            format!(
                "work protocol version {} is unsupported; expected {}",
                packet.work_protocol_version, WORK_PACKET_PROTOCOL_VERSION
            ),
        ));
    }
    if packet.operation_id.get() == 0
        || packet.task_id.get() == 0
        || packet.worker.slot.get() == 0
        || packet.worker.generation.get() == 0
        || packet.attempt == 0
        || packet.commit_sequence.get() == 0
    {
        return Err(ResumableOperationError::new(
            "cem.worker.work_identity_invalid",
            "work packet identities must be non-zero",
        ));
    }
    if packet.stage.label.is_empty()
        || packet.stage.label.len() > MAX_WORK_STAGE_LABEL_BYTES
        || packet.stage.label.chars().any(char::is_control)
    {
        return Err(ResumableOperationError::new(
            "cem.worker.work_stage_invalid",
            "work packet stage label is invalid",
        ));
    }
    let payload_bytes =
        serde_json::to_vec(&packet.payload).expect("serde_json::Value serialization cannot fail");
    if payload_bytes.len() > MAX_WORK_INLINE_PAYLOAD_BYTES {
        return Err(ResumableOperationError::new(
            "cem.worker.work_payload_too_large",
            format!(
                "work packet payload is {} bytes, exceeding {}",
                payload_bytes.len(),
                MAX_WORK_INLINE_PAYLOAD_BYTES
            ),
        ));
    }
    Ok(())
}

fn terminal_summary(terminal: &ResumableOperationTerminal) -> OperationTerminalSummary {
    OperationTerminalSummary {
        status: terminal.status,
        cause_code: terminal.error.as_ref().map(|error| error.code.clone()),
        diagnostic_count: terminal
            .result
            .as_ref()
            .and_then(|result| result.get("diagnostics"))
            .and_then(Value::as_array)
            .map_or(0, |diagnostics| {
                diagnostics.len().try_into().unwrap_or(u32::MAX)
            }),
        recovered_control_failure_count: 0,
        retained_artifact_count: 0,
        discarded_artifact_count: 0,
        restartable: (terminal.status == OperationTerminalStatus::Fatal).then_some(false),
    }
}

fn error_from_payload(payload: &Value) -> ResumableOperationError {
    payload
        .get("error")
        .cloned()
        .and_then(|error| serde_json::from_value(error).ok())
        .unwrap_or_else(|| {
            ResumableOperationError::new(
                "cem.operation.work_failed",
                "worker returned a failed result without a typed error",
            )
        })
}

fn query_error(error: QueryRunError) -> ResumableOperationError {
    match error {
        QueryRunError::Contract(error) => {
            ResumableOperationError::new("cem.operation.query_contract", error.to_string())
        }
        QueryRunError::Execution(failure) => ResumableOperationError::new(
            "cem.operation.query_failed",
            diagnostic_messages(&failure.diagnostics),
        ),
    }
}

fn diagnostic_messages(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn serialization_error(error: serde_json::Error) -> ResumableOperationError {
    ResumableOperationError::new("cem.operation.serialize", error.to_string())
}

fn deserialization_error(error: serde_json::Error) -> ResumableOperationError {
    ResumableOperationError::new("cem.operation.deserialize", error.to_string())
}

#[cfg(feature = "debug-control")]
fn invalid_state(
    operation: OperationId,
    state: ResumableOperationState,
    action: &str,
) -> ResumableOperationError {
    ResumableOperationError::new(
        "cem.operation.state",
        format!("operation {operation} cannot {action} while {state:?}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::{
        XML_CONTENT_TYPE, XML_SCHEMA_URI, XPATH_CONTENT_TYPE, XPATH_RESULT_CONTENT_TYPE,
        XPATH_SCHEMA_URI,
    };

    fn identity(content_type: &str, schema: &str) -> FormatIdentity {
        FormatIdentity {
            content_type: Some(content_type.to_owned()),
            schema: Some(schema.to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn source(
        uri: &str,
        bytes: &[u8],
        content_type: &str,
        schema: &str,
        from_format: Option<InputFormat>,
    ) -> OperationSource {
        OperationSource {
            uri: uri.to_owned(),
            bytes: bytes.to_vec(),
            from_format,
            identity: identity(content_type, schema),
            root_scope: ScopeConfig::default(),
        }
    }

    fn transform_request() -> ResumableRunRequest {
        let target = identity(XPATH_RESULT_CONTENT_TYPE, XPATH_SCHEMA_URI);
        ResumableRunRequest::Transform {
            data: source(
                "memory:catalog.xml",
                b"<catalog><book id=\"a\"/><book id=\"b\"/></catalog>",
                XML_CONTENT_TYPE,
                XML_SCHEMA_URI,
                Some(InputFormat::Xml),
            ),
            template: source(
                "memory:books.xpath",
                b"/catalog/book",
                XPATH_CONTENT_TYPE,
                XPATH_SCHEMA_URI,
                None,
            ),
            params: BTreeMap::new(),
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            target: Some(target.clone()),
            target_scope: ScopeConfig {
                default_content_type: target.content_type,
                schema: target.schema,
                ..ScopeConfig::default()
            },
            preserve_source_offsets: true,
        }
    }

    fn query_request() -> ResumableRunRequest {
        ResumableRunRequest::Query {
            data: source(
                "memory:catalog.xml",
                b"<catalog><book id=\"a\"/><book id=\"b\"/></catalog>",
                XML_CONTENT_TYPE,
                XML_SCHEMA_URI,
                Some(InputFormat::Xml),
            ),
            query: source(
                "memory:books.xpath",
                b"/catalog/book/@id/string()",
                XPATH_CONTENT_TYPE,
                XPATH_SCHEMA_URI,
                None,
            ),
        }
    }

    fn drive(
        request: ResumableRunRequest,
        max_packets: u32,
        reverse_batches: bool,
    ) -> ResumableOperationTerminal {
        let mut host = ResumableOperationHost::new(2).unwrap();
        let operation = host.start(request).unwrap().operation_id;
        for _ in 0..16 {
            let poll = host.poll(operation, max_packets).unwrap();
            if let Some(terminal) = poll.terminal {
                return terminal;
            }
            let mut packets = poll.packets;
            if reverse_batches {
                packets.reverse();
            }
            for packet in packets {
                let result = execute_operation_work(packet).unwrap();
                let accepted = host.accept_result(result).unwrap();
                if let Some(terminal) = accepted.terminal {
                    return terminal;
                }
            }
        }
        panic!("resumable fixture did not reach a terminal result")
    }

    #[test]
    fn transform_output_matches_direct_native_execution_across_poll_chunk_sizes() {
        let request = transform_request();
        let direct = execute_request(&request).unwrap();
        let one_at_a_time = drive(request.clone(), 1, false);
        let out_of_order_batch = drive(request, 2, true);
        assert_eq!(one_at_a_time.status, OperationTerminalStatus::Succeeded);
        assert_eq!(one_at_a_time.result, Some(direct.clone()));
        assert_eq!(out_of_order_batch.result, Some(direct));
    }

    #[test]
    fn query_output_matches_direct_native_execution_across_poll_chunk_sizes() {
        let request = query_request();
        let direct = execute_request(&request).unwrap();
        let one_at_a_time = drive(request.clone(), 1, false);
        let out_of_order_batch = drive(request, 2, true);
        assert_eq!(one_at_a_time.status, OperationTerminalStatus::Succeeded);
        assert_eq!(one_at_a_time.result, Some(direct.clone()));
        assert_eq!(out_of_order_batch.result, Some(direct));
    }

    #[test]
    fn cancellation_discards_inflight_work_and_retains_one_terminal() {
        let mut host = ResumableOperationHost::new(2).unwrap();
        let operation = host.start(transform_request()).unwrap().operation_id;
        let packet = host.poll(operation, 1).unwrap().packets.remove(0);
        let cancelled = host
            .cancel(operation, Some("fixture cancellation".to_owned()))
            .unwrap();
        assert_eq!(cancelled.status, OperationTerminalStatus::Cancelled);
        assert_eq!(cancelled.reason.as_deref(), Some("fixture cancellation"));
        assert_eq!(
            host.cancel(operation, Some("losing reason".to_owned()))
                .unwrap(),
            cancelled
        );
        assert_eq!(
            host.accept_result(execute_operation_work(packet).unwrap())
                .unwrap_err()
                .code,
            "cem.worker.operation_terminal"
        );
        let poll = host.poll(operation, 4).unwrap();
        assert_eq!(poll.state, ResumableOperationState::Terminal);
        assert_eq!(poll.terminal, Some(cancelled));
    }

    #[cfg(feature = "debug-control")]
    #[test]
    fn all_stop_pause_continue_and_step_gate_bounded_work_dispatch() {
        let mut host = ResumableOperationHost::new(2).unwrap();
        let workers = host.workers().to_vec();
        let operation = host.start(query_request()).unwrap().operation_id;
        let initial = host.poll(operation, 2).unwrap().packets;
        for packet in initial {
            host.accept_result(execute_operation_work(packet).unwrap())
                .unwrap();
        }

        let first_stop = WorkerStopGeneration::from_raw(1);
        assert!(matches!(
            host.pause(operation, first_stop).unwrap(),
            WorkerStopRendezvousStatus::Pending { .. }
        ));
        assert!(matches!(
            host.acknowledge_stop(
                operation,
                first_stop,
                workers[0],
                WorkerStopDisposition::Parked,
            )
            .unwrap(),
            WorkerStopRendezvousStatus::Pending { .. }
        ));
        assert!(matches!(
            host.acknowledge_stop(
                operation,
                first_stop,
                workers[1],
                WorkerStopDisposition::ExternalWait,
            )
            .unwrap(),
            WorkerStopRendezvousStatus::Complete { .. }
        ));
        assert!(host.poll(operation, 4).unwrap().packets.is_empty());

        let second_stop = WorkerStopGeneration::from_raw(2);
        host.step(operation, first_stop, second_stop).unwrap();
        let stepped = host.poll(operation, 4).unwrap();
        assert_eq!(stepped.state, ResumableOperationState::Stepping);
        assert_eq!(stepped.packets.len(), 1);
        let accepted = host
            .accept_result(execute_operation_work(stepped.packets[0].clone()).unwrap())
            .unwrap();
        assert_eq!(accepted.state, ResumableOperationState::PauseRequested);
        for worker in &workers {
            host.acknowledge_stop(
                operation,
                second_stop,
                *worker,
                WorkerStopDisposition::ExternalWait,
            )
            .unwrap();
        }
        assert_eq!(
            host.poll(operation, 4).unwrap().state,
            ResumableOperationState::Paused
        );

        host.continue_operation(operation, second_stop).unwrap();
        let final_packet = host.poll(operation, 4).unwrap().packets.remove(0);
        let terminal = host
            .accept_result(execute_operation_work(final_packet).unwrap())
            .unwrap()
            .terminal
            .unwrap();
        assert_eq!(terminal.status, OperationTerminalStatus::Succeeded);
    }
}
