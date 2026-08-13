//! Host-neutral worker transport identities and coordinator-owned routing.
//!
//! Workers carry one runtime instance and exchange typed operation-host
//! envelopes with their coordinator. The coordinator owns every stable route;
//! replacing a physical worker advances its generation and invalidates all
//! routes into the previous instance before that slot can be reused.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

#[cfg(feature = "debug-control")]
use crate::debug_control::SnapshotReferenceId;
use crate::operation_control::{ExecutionScopeId, OperationId, TaskId};
use crate::operation_handle::{
    EventSubscriptionId, OperationHostEnvelope, OperationHostMessageKind, OperationTerminalSummary,
    RetainedHandleId, OPERATION_PROTOCOL_VERSION,
};

pub const WORKER_PROTOCOL_VERSION: u16 = 1;
pub const MAX_COORDINATED_WORKERS: u16 = 256;
pub const MAX_TRANSFER_BUFFERS_PER_MESSAGE: u16 = 64;
pub const MAX_TRANSFER_BYTES_PER_MESSAGE: u64 = 64 * 1_024 * 1_024;

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            pub const fn from_raw(value: u64) -> Self {
                Self(value)
            }

            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

opaque_id!(WorkerSlotId);
opaque_id!(WorkerGeneration);
opaque_id!(TransferBufferId);
#[cfg(feature = "debug-control")]
opaque_id!(WorkerStopGeneration);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerAddress {
    pub slot: WorkerSlotId,
    pub generation: WorkerGeneration,
}

impl WorkerAddress {
    pub const fn new(slot: WorkerSlotId, generation: WorkerGeneration) -> Self {
        Self { slot, generation }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkerLifecycleState {
    Initializing,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferBufferDescriptor {
    pub id: TransferBufferId,
    pub byte_length: u64,
}

/// Message-passing projection used by both Node worker threads and browser
/// dedicated workers. Actual buffers travel out of band in the host transfer
/// list and are referenced only by bounded descriptors here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerEnvelope<T> {
    pub worker_protocol_version: u16,
    pub worker: WorkerAddress,
    pub sequence: u64,
    pub operation: OperationHostEnvelope<T>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transfers: Vec<TransferBufferDescriptor>,
}

impl<T> WorkerEnvelope<T> {
    pub fn new(
        worker: WorkerAddress,
        sequence: u64,
        operation: OperationHostEnvelope<T>,
        transfers: Vec<TransferBufferDescriptor>,
    ) -> Self {
        Self {
            worker_protocol_version: WORKER_PROTOCOL_VERSION,
            worker,
            sequence,
            operation,
            transfers,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerCoordinatorLimits {
    pub max_workers: u16,
    pub max_transfer_buffers_per_message: u16,
    pub max_transfer_bytes_per_message: u64,
}

impl Default for WorkerCoordinatorLimits {
    fn default() -> Self {
        Self {
            max_workers: MAX_COORDINATED_WORKERS,
            max_transfer_buffers_per_message: MAX_TRANSFER_BUFFERS_PER_MESSAGE,
            max_transfer_bytes_per_message: MAX_TRANSFER_BYTES_PER_MESSAGE,
        }
    }
}

#[cfg(feature = "debug-control")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkerStopDisposition {
    Parked,
    ExternalWait,
}

#[cfg(feature = "debug-control")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerStopRendezvousStatus {
    Pending {
        awaiting: BTreeSet<WorkerAddress>,
    },
    Complete {
        parked: BTreeSet<WorkerAddress>,
        external_wait: BTreeSet<WorkerAddress>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerReplacement {
    pub previous: WorkerAddress,
    pub replacement: WorkerAddress,
    pub affected_operations: BTreeSet<OperationId>,
    pub invalidated_scope_routes: Vec<(OperationId, ExecutionScopeId)>,
    pub invalidated_task_routes: Vec<(OperationId, TaskId)>,
    pub invalidated_subscriptions: Vec<(OperationId, EventSubscriptionId)>,
    pub invalidated_retained_handles: Vec<(OperationId, RetainedHandleId)>,
    #[cfg(feature = "debug-control")]
    pub invalidated_snapshot_references: Vec<(OperationId, SnapshotReferenceId)>,
    #[cfg(feature = "debug-control")]
    pub invalidated_stop_rendezvous: BTreeSet<OperationId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTerminalClaim {
    pub published: bool,
    pub terminal: OperationTerminalSummary,
}

#[derive(Debug, Clone)]
struct WorkerSlot {
    generation: WorkerGeneration,
    lifecycle: WorkerLifecycleState,
    initialize_accepted: bool,
    next_sequence: u64,
}

impl WorkerSlot {
    fn address(&self, slot: WorkerSlotId) -> WorkerAddress {
        WorkerAddress::new(slot, self.generation)
    }
}

#[cfg(feature = "debug-control")]
#[derive(Debug, Clone)]
struct StopRendezvous {
    generation: WorkerStopGeneration,
    expected: BTreeSet<WorkerAddress>,
    classified: BTreeMap<WorkerAddress, WorkerStopDisposition>,
}

#[derive(Debug, Clone, Default)]
struct OperationRoutes {
    workers: BTreeSet<WorkerAddress>,
    scopes: BTreeMap<ExecutionScopeId, BTreeSet<WorkerAddress>>,
    tasks: BTreeMap<TaskId, WorkerAddress>,
    subscriptions: BTreeMap<EventSubscriptionId, WorkerAddress>,
    retained_handles: BTreeMap<RetainedHandleId, WorkerAddress>,
    #[cfg(feature = "debug-control")]
    snapshot_references: BTreeMap<SnapshotReferenceId, WorkerAddress>,
    #[cfg(feature = "debug-control")]
    stop: Option<StopRendezvous>,
    terminal: Option<OperationTerminalSummary>,
}

/// Deterministic coordinator state shared by Node and browser worker hosts.
/// It stores routing metadata only; engine values and buffers remain owned by
/// their runtime/host boundary.
#[derive(Debug, Clone)]
pub struct WorkerCoordinator {
    limits: WorkerCoordinatorLimits,
    slots: BTreeMap<WorkerSlotId, WorkerSlot>,
    operations: BTreeMap<OperationId, OperationRoutes>,
}

impl WorkerCoordinator {
    pub fn new(worker_count: u16) -> Result<Self, WorkerCoordinatorError> {
        Self::with_limits(worker_count, WorkerCoordinatorLimits::default())
    }

    pub fn with_limits(
        worker_count: u16,
        limits: WorkerCoordinatorLimits,
    ) -> Result<Self, WorkerCoordinatorError> {
        validate_limits(limits)?;
        if worker_count == 0 || worker_count > limits.max_workers {
            return Err(WorkerCoordinatorError::WorkerCount {
                requested: worker_count,
                maximum: limits.max_workers,
            });
        }
        let slots = (1..=worker_count)
            .map(|slot| {
                (
                    WorkerSlotId::from_raw(u64::from(slot)),
                    WorkerSlot {
                        generation: WorkerGeneration::from_raw(1),
                        lifecycle: WorkerLifecycleState::Initializing,
                        initialize_accepted: false,
                        next_sequence: 1,
                    },
                )
            })
            .collect();
        Ok(Self {
            limits,
            slots,
            operations: BTreeMap::new(),
        })
    }

    pub fn limits(&self) -> WorkerCoordinatorLimits {
        self.limits
    }

    pub fn worker(&self, slot: WorkerSlotId) -> Result<WorkerAddress, WorkerCoordinatorError> {
        let state = self
            .slots
            .get(&slot)
            .ok_or(WorkerCoordinatorError::UnknownWorkerSlot(slot))?;
        Ok(state.address(slot))
    }

    pub fn worker_state(
        &self,
        worker: WorkerAddress,
    ) -> Result<WorkerLifecycleState, WorkerCoordinatorError> {
        Ok(self.current_worker(worker)?.lifecycle)
    }

    pub fn mark_ready(&mut self, worker: WorkerAddress) -> Result<(), WorkerCoordinatorError> {
        let state = self.current_worker_mut(worker)?;
        if !state.initialize_accepted {
            return Err(WorkerCoordinatorError::WorkerInitializationMissing(worker));
        }
        state.lifecycle = WorkerLifecycleState::Ready;
        Ok(())
    }

    pub fn register_operation(
        &mut self,
        operation: OperationId,
    ) -> Result<(), WorkerCoordinatorError> {
        if operation.get() == 0 {
            return Err(WorkerCoordinatorError::InvalidOperation(operation));
        }
        if self.operations.contains_key(&operation) {
            return Err(WorkerCoordinatorError::OperationAlreadyRegistered(
                operation,
            ));
        }
        self.operations
            .insert(operation, OperationRoutes::default());
        Ok(())
    }

    pub fn assign_worker(
        &mut self,
        operation: OperationId,
        worker: WorkerAddress,
    ) -> Result<(), WorkerCoordinatorError> {
        self.ensure_ready(worker)?;
        let routes = self.operation_mut(operation)?;
        ensure_not_terminal(operation, routes)?;
        routes.workers.insert(worker);
        Ok(())
    }

    pub fn assign_scope(
        &mut self,
        operation: OperationId,
        scope: ExecutionScopeId,
        worker: WorkerAddress,
    ) -> Result<(), WorkerCoordinatorError> {
        validate_route_identity("scope", scope.get())?;
        self.ensure_assigned(operation, worker)?;
        self.operation_mut(operation)?
            .scopes
            .entry(scope)
            .or_default()
            .insert(worker);
        Ok(())
    }

    pub fn route_task(
        &mut self,
        operation: OperationId,
        task: TaskId,
        worker: WorkerAddress,
    ) -> Result<(), WorkerCoordinatorError> {
        validate_route_identity("task", task.get())?;
        self.ensure_assigned(operation, worker)?;
        self.operation_mut(operation)?.tasks.insert(task, worker);
        Ok(())
    }

    pub fn route_subscription(
        &mut self,
        operation: OperationId,
        subscription: EventSubscriptionId,
        worker: WorkerAddress,
    ) -> Result<(), WorkerCoordinatorError> {
        validate_route_identity("subscription", subscription.get())?;
        self.ensure_assigned(operation, worker)?;
        self.operation_mut(operation)?
            .subscriptions
            .insert(subscription, worker);
        Ok(())
    }

    pub fn route_retained_handle(
        &mut self,
        operation: OperationId,
        handle: RetainedHandleId,
        worker: WorkerAddress,
    ) -> Result<(), WorkerCoordinatorError> {
        validate_route_identity("retained-handle", handle.get())?;
        self.ensure_assigned(operation, worker)?;
        self.operation_mut(operation)?
            .retained_handles
            .insert(handle, worker);
        Ok(())
    }

    #[cfg(feature = "debug-control")]
    pub fn route_snapshot_reference(
        &mut self,
        operation: OperationId,
        reference: SnapshotReferenceId,
        worker: WorkerAddress,
    ) -> Result<(), WorkerCoordinatorError> {
        validate_route_identity("snapshot-reference", reference.get())?;
        self.ensure_assigned(operation, worker)?;
        self.operation_mut(operation)?
            .snapshot_references
            .insert(reference, worker);
        Ok(())
    }

    pub fn task_worker(
        &self,
        operation: OperationId,
        task: TaskId,
    ) -> Result<Option<WorkerAddress>, WorkerCoordinatorError> {
        Ok(self.operation(operation)?.tasks.get(&task).copied())
    }

    pub fn operation_workers(
        &self,
        operation: OperationId,
    ) -> Result<&BTreeSet<WorkerAddress>, WorkerCoordinatorError> {
        Ok(&self.operation(operation)?.workers)
    }

    pub fn scope_workers(
        &self,
        operation: OperationId,
        scope: ExecutionScopeId,
    ) -> Result<Option<&BTreeSet<WorkerAddress>>, WorkerCoordinatorError> {
        Ok(self.operation(operation)?.scopes.get(&scope))
    }

    pub fn subscription_worker(
        &self,
        operation: OperationId,
        subscription: EventSubscriptionId,
    ) -> Result<Option<WorkerAddress>, WorkerCoordinatorError> {
        Ok(self
            .operation(operation)?
            .subscriptions
            .get(&subscription)
            .copied())
    }

    pub fn retained_handle_worker(
        &self,
        operation: OperationId,
        handle: RetainedHandleId,
    ) -> Result<Option<WorkerAddress>, WorkerCoordinatorError> {
        Ok(self
            .operation(operation)?
            .retained_handles
            .get(&handle)
            .copied())
    }

    #[cfg(feature = "debug-control")]
    pub fn snapshot_reference_worker(
        &self,
        operation: OperationId,
        reference: SnapshotReferenceId,
    ) -> Result<Option<WorkerAddress>, WorkerCoordinatorError> {
        Ok(self
            .operation(operation)?
            .snapshot_references
            .get(&reference)
            .copied())
    }

    /// Validate a message before its payload is observed. Failed validation is
    /// non-mutating, including sequence failures and stale generations.
    pub fn accept<T>(
        &mut self,
        envelope: &WorkerEnvelope<T>,
    ) -> Result<(), WorkerCoordinatorError> {
        if envelope.worker_protocol_version != WORKER_PROTOCOL_VERSION {
            return Err(WorkerCoordinatorError::ProtocolVersion {
                requested: envelope.worker_protocol_version,
                supported: WORKER_PROTOCOL_VERSION,
            });
        }
        if envelope.operation.protocol_version != OPERATION_PROTOCOL_VERSION {
            return Err(WorkerCoordinatorError::OperationProtocolVersion {
                requested: envelope.operation.protocol_version,
                supported: OPERATION_PROTOCOL_VERSION,
            });
        }
        validate_operation_envelope(&envelope.operation)?;
        self.validate_transfers(&envelope.transfers)?;
        let worker_state = self.current_worker(envelope.worker)?;
        let lifecycle = worker_state.lifecycle;
        match lifecycle {
            WorkerLifecycleState::Ready
                if envelope.operation.kind == OperationHostMessageKind::Initialize =>
            {
                return Err(WorkerCoordinatorError::WorkerAlreadyReady(envelope.worker));
            }
            WorkerLifecycleState::Initializing
                if envelope.operation.kind != OperationHostMessageKind::Initialize =>
            {
                return Err(WorkerCoordinatorError::WorkerNotReady(envelope.worker));
            }
            WorkerLifecycleState::Initializing if worker_state.initialize_accepted => {
                return Err(WorkerCoordinatorError::WorkerInitializeAlreadyAccepted(
                    envelope.worker,
                ));
            }
            WorkerLifecycleState::Initializing | WorkerLifecycleState::Ready => {}
        }
        if let Some(operation) = envelope.operation.operation_id {
            self.ensure_assigned(operation, envelope.worker)?;
        }
        let state = self.current_worker_mut(envelope.worker)?;
        if envelope.sequence == 0 || envelope.sequence != state.next_sequence {
            return Err(WorkerCoordinatorError::MessageSequence {
                worker: envelope.worker,
                requested: envelope.sequence,
                expected: state.next_sequence,
            });
        }
        state.next_sequence = state.next_sequence.checked_add(1).ok_or(
            WorkerCoordinatorError::MessageSequenceExhausted(envelope.worker),
        )?;
        if envelope.operation.kind == OperationHostMessageKind::Initialize {
            state.initialize_accepted = true;
        }
        Ok(())
    }

    pub fn claim_terminal(
        &mut self,
        operation: OperationId,
        terminal: OperationTerminalSummary,
    ) -> Result<WorkerTerminalClaim, WorkerCoordinatorError> {
        let routes = self.operation_mut(operation)?;
        if let Some(existing) = &routes.terminal {
            return Ok(WorkerTerminalClaim {
                published: false,
                terminal: existing.clone(),
            });
        }
        routes.terminal = Some(terminal.clone());
        #[cfg(feature = "debug-control")]
        {
            routes.stop = None;
        }
        Ok(WorkerTerminalClaim {
            published: true,
            terminal,
        })
    }

    pub fn terminal(
        &self,
        operation: OperationId,
    ) -> Result<Option<&OperationTerminalSummary>, WorkerCoordinatorError> {
        Ok(self.operation(operation)?.terminal.as_ref())
    }

    #[cfg(feature = "debug-control")]
    pub fn begin_stop(
        &mut self,
        operation: OperationId,
        generation: WorkerStopGeneration,
    ) -> Result<WorkerStopRendezvousStatus, WorkerCoordinatorError> {
        if generation.get() == 0 {
            return Err(WorkerCoordinatorError::InvalidStopGeneration(generation));
        }
        let routes = self.operation_mut(operation)?;
        ensure_not_terminal(operation, routes)?;
        if routes.workers.is_empty() {
            return Err(WorkerCoordinatorError::OperationHasNoWorkers(operation));
        }
        if let Some(stop) = &routes.stop {
            return Err(WorkerCoordinatorError::StopAlreadyActive {
                operation,
                generation: stop.generation,
            });
        }
        routes.stop = Some(StopRendezvous {
            generation,
            expected: routes.workers.clone(),
            classified: BTreeMap::new(),
        });
        self.stop_status(operation, generation)
    }

    #[cfg(feature = "debug-control")]
    pub fn acknowledge_stop(
        &mut self,
        operation: OperationId,
        generation: WorkerStopGeneration,
        worker: WorkerAddress,
        disposition: WorkerStopDisposition,
    ) -> Result<WorkerStopRendezvousStatus, WorkerCoordinatorError> {
        self.ensure_assigned(operation, worker)?;
        let stop = self
            .operation_mut(operation)?
            .stop
            .as_mut()
            .ok_or(WorkerCoordinatorError::StopNotActive(operation))?;
        if stop.generation != generation {
            return Err(WorkerCoordinatorError::StaleStopGeneration {
                operation,
                requested: generation,
                current: stop.generation,
            });
        }
        if !stop.expected.contains(&worker) {
            return Err(WorkerCoordinatorError::WorkerNotInStop { operation, worker });
        }
        if let Some(existing) = stop.classified.get(&worker) {
            if *existing != disposition {
                return Err(WorkerCoordinatorError::StopClassificationConflict {
                    operation,
                    worker,
                });
            }
        } else {
            stop.classified.insert(worker, disposition);
        }
        self.stop_status(operation, generation)
    }

    #[cfg(feature = "debug-control")]
    pub fn clear_stop(
        &mut self,
        operation: OperationId,
        generation: WorkerStopGeneration,
    ) -> Result<(), WorkerCoordinatorError> {
        match self.operation(operation)?.stop.as_ref() {
            Some(stop) if stop.generation == generation => {}
            Some(stop) => {
                return Err(WorkerCoordinatorError::StaleStopGeneration {
                    operation,
                    requested: generation,
                    current: stop.generation,
                });
            }
            None => return Err(WorkerCoordinatorError::StopNotActive(operation)),
        }
        self.operation_mut(operation)?.stop = None;
        Ok(())
    }

    pub fn replace_worker(
        &mut self,
        slot: WorkerSlotId,
    ) -> Result<WorkerReplacement, WorkerCoordinatorError> {
        let previous = self.worker(slot)?;
        let replacement_generation = previous
            .generation
            .get()
            .checked_add(1)
            .map(WorkerGeneration::from_raw)
            .ok_or(WorkerCoordinatorError::WorkerGenerationExhausted(slot))?;

        let mut replacement = WorkerReplacement {
            previous,
            replacement: WorkerAddress::new(slot, replacement_generation),
            affected_operations: BTreeSet::new(),
            invalidated_scope_routes: Vec::new(),
            invalidated_task_routes: Vec::new(),
            invalidated_subscriptions: Vec::new(),
            invalidated_retained_handles: Vec::new(),
            #[cfg(feature = "debug-control")]
            invalidated_snapshot_references: Vec::new(),
            #[cfg(feature = "debug-control")]
            invalidated_stop_rendezvous: BTreeSet::new(),
        };

        for (operation, routes) in &mut self.operations {
            if routes.workers.remove(&previous) {
                replacement.affected_operations.insert(*operation);
            }
            routes.scopes.retain(|scope, workers| {
                if workers.remove(&previous) {
                    replacement
                        .invalidated_scope_routes
                        .push((*operation, *scope));
                }
                !workers.is_empty()
            });
            routes.tasks.retain(|task, worker| {
                if *worker == previous {
                    replacement
                        .invalidated_task_routes
                        .push((*operation, *task));
                    false
                } else {
                    true
                }
            });
            routes.subscriptions.retain(|subscription, worker| {
                if *worker == previous {
                    replacement
                        .invalidated_subscriptions
                        .push((*operation, *subscription));
                    false
                } else {
                    true
                }
            });
            routes.retained_handles.retain(|handle, worker| {
                if *worker == previous {
                    replacement
                        .invalidated_retained_handles
                        .push((*operation, *handle));
                    false
                } else {
                    true
                }
            });
            #[cfg(feature = "debug-control")]
            routes.snapshot_references.retain(|reference, worker| {
                if *worker == previous {
                    replacement
                        .invalidated_snapshot_references
                        .push((*operation, *reference));
                    false
                } else {
                    true
                }
            });
            #[cfg(feature = "debug-control")]
            if routes
                .stop
                .as_ref()
                .is_some_and(|stop| stop.expected.contains(&previous))
            {
                routes.stop = None;
                replacement.invalidated_stop_rendezvous.insert(*operation);
            }
        }

        let state = self
            .slots
            .get_mut(&slot)
            .expect("validated worker slot remains registered");
        state.generation = replacement_generation;
        state.lifecycle = WorkerLifecycleState::Initializing;
        state.initialize_accepted = false;
        state.next_sequence = 1;
        Ok(replacement)
    }

    #[cfg(feature = "debug-control")]
    fn stop_status(
        &self,
        operation: OperationId,
        generation: WorkerStopGeneration,
    ) -> Result<WorkerStopRendezvousStatus, WorkerCoordinatorError> {
        let stop = self
            .operation(operation)?
            .stop
            .as_ref()
            .ok_or(WorkerCoordinatorError::StopNotActive(operation))?;
        if stop.generation != generation {
            return Err(WorkerCoordinatorError::StaleStopGeneration {
                operation,
                requested: generation,
                current: stop.generation,
            });
        }
        let awaiting = stop
            .expected
            .difference(&stop.classified.keys().copied().collect())
            .copied()
            .collect::<BTreeSet<_>>();
        if !awaiting.is_empty() {
            return Ok(WorkerStopRendezvousStatus::Pending { awaiting });
        }
        let parked = stop
            .classified
            .iter()
            .filter_map(|(worker, disposition)| {
                (*disposition == WorkerStopDisposition::Parked).then_some(*worker)
            })
            .collect();
        let external_wait = stop
            .classified
            .iter()
            .filter_map(|(worker, disposition)| {
                (*disposition == WorkerStopDisposition::ExternalWait).then_some(*worker)
            })
            .collect();
        Ok(WorkerStopRendezvousStatus::Complete {
            parked,
            external_wait,
        })
    }

    fn validate_transfers(
        &self,
        transfers: &[TransferBufferDescriptor],
    ) -> Result<(), WorkerCoordinatorError> {
        if transfers.len() > usize::from(self.limits.max_transfer_buffers_per_message) {
            return Err(WorkerCoordinatorError::TransferCount {
                requested: transfers.len(),
                maximum: self.limits.max_transfer_buffers_per_message,
            });
        }
        let mut ids = BTreeSet::new();
        let mut total = 0u64;
        for transfer in transfers {
            if transfer.id.get() == 0 || !ids.insert(transfer.id) {
                return Err(WorkerCoordinatorError::InvalidTransferId(transfer.id));
            }
            if transfer.byte_length == 0 {
                return Err(WorkerCoordinatorError::EmptyTransfer(transfer.id));
            }
            total = total.checked_add(transfer.byte_length).ok_or(
                WorkerCoordinatorError::TransferBytes {
                    requested: u64::MAX,
                    maximum: self.limits.max_transfer_bytes_per_message,
                },
            )?;
        }
        if total > self.limits.max_transfer_bytes_per_message {
            return Err(WorkerCoordinatorError::TransferBytes {
                requested: total,
                maximum: self.limits.max_transfer_bytes_per_message,
            });
        }
        Ok(())
    }

    fn ensure_ready(&self, worker: WorkerAddress) -> Result<(), WorkerCoordinatorError> {
        let state = self.current_worker(worker)?;
        if state.lifecycle != WorkerLifecycleState::Ready {
            return Err(WorkerCoordinatorError::WorkerNotReady(worker));
        }
        Ok(())
    }

    fn ensure_assigned(
        &self,
        operation: OperationId,
        worker: WorkerAddress,
    ) -> Result<(), WorkerCoordinatorError> {
        self.current_worker(worker)?;
        let routes = self.operation(operation)?;
        ensure_not_terminal(operation, routes)?;
        if !routes.workers.contains(&worker) {
            return Err(WorkerCoordinatorError::WorkerNotAssigned { operation, worker });
        }
        Ok(())
    }

    fn current_worker(&self, worker: WorkerAddress) -> Result<&WorkerSlot, WorkerCoordinatorError> {
        let state = self
            .slots
            .get(&worker.slot)
            .ok_or(WorkerCoordinatorError::UnknownWorkerSlot(worker.slot))?;
        if worker.slot.get() == 0 || worker.generation.get() == 0 {
            return Err(WorkerCoordinatorError::InvalidWorkerAddress(worker));
        }
        if state.generation != worker.generation {
            return Err(WorkerCoordinatorError::StaleWorker {
                requested: worker,
                current: state.address(worker.slot),
            });
        }
        Ok(state)
    }

    fn current_worker_mut(
        &mut self,
        worker: WorkerAddress,
    ) -> Result<&mut WorkerSlot, WorkerCoordinatorError> {
        let state = self
            .slots
            .get_mut(&worker.slot)
            .ok_or(WorkerCoordinatorError::UnknownWorkerSlot(worker.slot))?;
        if worker.slot.get() == 0 || worker.generation.get() == 0 {
            return Err(WorkerCoordinatorError::InvalidWorkerAddress(worker));
        }
        if state.generation != worker.generation {
            return Err(WorkerCoordinatorError::StaleWorker {
                requested: worker,
                current: state.address(worker.slot),
            });
        }
        Ok(state)
    }

    fn operation(
        &self,
        operation: OperationId,
    ) -> Result<&OperationRoutes, WorkerCoordinatorError> {
        self.operations
            .get(&operation)
            .ok_or(WorkerCoordinatorError::UnknownOperation(operation))
    }

    fn operation_mut(
        &mut self,
        operation: OperationId,
    ) -> Result<&mut OperationRoutes, WorkerCoordinatorError> {
        self.operations
            .get_mut(&operation)
            .ok_or(WorkerCoordinatorError::UnknownOperation(operation))
    }
}

fn validate_limits(limits: WorkerCoordinatorLimits) -> Result<(), WorkerCoordinatorError> {
    if limits.max_workers == 0 || limits.max_workers > MAX_COORDINATED_WORKERS {
        return Err(WorkerCoordinatorError::InvalidLimit {
            field: "maxWorkers",
            requested: u64::from(limits.max_workers),
            maximum: u64::from(MAX_COORDINATED_WORKERS),
        });
    }
    if limits.max_transfer_buffers_per_message == 0
        || limits.max_transfer_buffers_per_message > MAX_TRANSFER_BUFFERS_PER_MESSAGE
    {
        return Err(WorkerCoordinatorError::InvalidLimit {
            field: "maxTransferBuffersPerMessage",
            requested: u64::from(limits.max_transfer_buffers_per_message),
            maximum: u64::from(MAX_TRANSFER_BUFFERS_PER_MESSAGE),
        });
    }
    if limits.max_transfer_bytes_per_message == 0
        || limits.max_transfer_bytes_per_message > MAX_TRANSFER_BYTES_PER_MESSAGE
    {
        return Err(WorkerCoordinatorError::InvalidLimit {
            field: "maxTransferBytesPerMessage",
            requested: limits.max_transfer_bytes_per_message,
            maximum: MAX_TRANSFER_BYTES_PER_MESSAGE,
        });
    }
    Ok(())
}

fn validate_operation_envelope<T>(
    envelope: &OperationHostEnvelope<T>,
) -> Result<(), WorkerCoordinatorError> {
    match (envelope.kind, envelope.operation_id) {
        (OperationHostMessageKind::Initialize, None) if envelope.sequence.is_none() => Ok(()),
        (OperationHostMessageKind::Initialize, Some(_)) | (_, None) => {
            Err(WorkerCoordinatorError::InvalidOperationEnvelope)
        }
        (_, Some(operation)) if operation.get() == 0 => {
            Err(WorkerCoordinatorError::InvalidOperation(operation))
        }
        (_, Some(_)) => Ok(()),
    }
}

fn ensure_not_terminal(
    operation: OperationId,
    routes: &OperationRoutes,
) -> Result<(), WorkerCoordinatorError> {
    if routes.terminal.is_some() {
        return Err(WorkerCoordinatorError::OperationTerminal(operation));
    }
    Ok(())
}

fn validate_route_identity(kind: &'static str, value: u64) -> Result<(), WorkerCoordinatorError> {
    if value == 0 {
        return Err(WorkerCoordinatorError::InvalidRouteIdentity { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerCoordinatorError {
    ProtocolVersion {
        requested: u16,
        supported: u16,
    },
    OperationProtocolVersion {
        requested: u16,
        supported: u16,
    },
    InvalidLimit {
        field: &'static str,
        requested: u64,
        maximum: u64,
    },
    WorkerCount {
        requested: u16,
        maximum: u16,
    },
    UnknownWorkerSlot(WorkerSlotId),
    InvalidWorkerAddress(WorkerAddress),
    StaleWorker {
        requested: WorkerAddress,
        current: WorkerAddress,
    },
    WorkerNotReady(WorkerAddress),
    WorkerInitializationMissing(WorkerAddress),
    WorkerInitializeAlreadyAccepted(WorkerAddress),
    WorkerAlreadyReady(WorkerAddress),
    WorkerGenerationExhausted(WorkerSlotId),
    MessageSequence {
        worker: WorkerAddress,
        requested: u64,
        expected: u64,
    },
    MessageSequenceExhausted(WorkerAddress),
    InvalidOperationEnvelope,
    InvalidOperation(OperationId),
    OperationAlreadyRegistered(OperationId),
    UnknownOperation(OperationId),
    OperationTerminal(OperationId),
    OperationHasNoWorkers(OperationId),
    WorkerNotAssigned {
        operation: OperationId,
        worker: WorkerAddress,
    },
    InvalidRouteIdentity {
        kind: &'static str,
    },
    TransferCount {
        requested: usize,
        maximum: u16,
    },
    InvalidTransferId(TransferBufferId),
    EmptyTransfer(TransferBufferId),
    TransferBytes {
        requested: u64,
        maximum: u64,
    },
    #[cfg(feature = "debug-control")]
    InvalidStopGeneration(WorkerStopGeneration),
    #[cfg(feature = "debug-control")]
    StopAlreadyActive {
        operation: OperationId,
        generation: WorkerStopGeneration,
    },
    #[cfg(feature = "debug-control")]
    StopNotActive(OperationId),
    #[cfg(feature = "debug-control")]
    StaleStopGeneration {
        operation: OperationId,
        requested: WorkerStopGeneration,
        current: WorkerStopGeneration,
    },
    #[cfg(feature = "debug-control")]
    WorkerNotInStop {
        operation: OperationId,
        worker: WorkerAddress,
    },
    #[cfg(feature = "debug-control")]
    StopClassificationConflict {
        operation: OperationId,
        worker: WorkerAddress,
    },
}

impl WorkerCoordinatorError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ProtocolVersion { .. } => "cem.worker.protocol_version",
            Self::OperationProtocolVersion { .. } => "cem.worker.operation_protocol_version",
            Self::InvalidLimit { .. } => "cem.worker.limit_invalid",
            Self::WorkerCount { .. } => "cem.worker.count_invalid",
            Self::UnknownWorkerSlot(_) => "cem.worker.slot_unknown",
            Self::InvalidWorkerAddress(_) => "cem.worker.address_invalid",
            Self::StaleWorker { .. } => "cem.worker.generation_stale",
            Self::WorkerNotReady(_) => "cem.worker.not_ready",
            Self::WorkerInitializationMissing(_) => "cem.worker.initialize_missing",
            Self::WorkerInitializeAlreadyAccepted(_) => "cem.worker.initialize_duplicate",
            Self::WorkerAlreadyReady(_) => "cem.worker.already_ready",
            Self::WorkerGenerationExhausted(_) => "cem.worker.generation_exhausted",
            Self::MessageSequence { .. } => "cem.worker.sequence_invalid",
            Self::MessageSequenceExhausted(_) => "cem.worker.sequence_exhausted",
            Self::InvalidOperationEnvelope => "cem.worker.operation_envelope_invalid",
            Self::InvalidOperation(_) => "cem.worker.operation_invalid",
            Self::OperationAlreadyRegistered(_) => "cem.worker.operation_registered",
            Self::UnknownOperation(_) => "cem.worker.operation_unknown",
            Self::OperationTerminal(_) => "cem.worker.operation_terminal",
            Self::OperationHasNoWorkers(_) => "cem.worker.operation_workers_empty",
            Self::WorkerNotAssigned { .. } => "cem.worker.operation_route_unknown",
            Self::InvalidRouteIdentity { .. } => "cem.worker.route_identity_invalid",
            Self::TransferCount { .. } => "cem.worker.transfer_count",
            Self::InvalidTransferId(_) => "cem.worker.transfer_id_invalid",
            Self::EmptyTransfer(_) => "cem.worker.transfer_empty",
            Self::TransferBytes { .. } => "cem.worker.transfer_bytes",
            #[cfg(feature = "debug-control")]
            Self::InvalidStopGeneration(_) => "cem.worker.stop_generation_invalid",
            #[cfg(feature = "debug-control")]
            Self::StopAlreadyActive { .. } => "cem.worker.stop_active",
            #[cfg(feature = "debug-control")]
            Self::StopNotActive(_) => "cem.worker.stop_missing",
            #[cfg(feature = "debug-control")]
            Self::StaleStopGeneration { .. } => "cem.worker.stop_generation_stale",
            #[cfg(feature = "debug-control")]
            Self::WorkerNotInStop { .. } => "cem.worker.stop_worker_unknown",
            #[cfg(feature = "debug-control")]
            Self::StopClassificationConflict { .. } => "cem.worker.stop_classification_conflict",
        }
    }
}

impl fmt::Display for WorkerCoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtocolVersion {
                requested,
                supported,
            } => write!(
                formatter,
                "worker protocol version {requested} is unsupported; expected {supported}"
            ),
            Self::OperationProtocolVersion {
                requested,
                supported,
            } => write!(
                formatter,
                "operation protocol version {requested} is unsupported; expected {supported}"
            ),
            Self::InvalidLimit {
                field,
                requested,
                maximum,
            } => write!(
                formatter,
                "worker limit {field}={requested} is outside 1..={maximum}"
            ),
            Self::WorkerCount { requested, maximum } => write!(
                formatter,
                "worker count {requested} is outside 1..={maximum}"
            ),
            Self::UnknownWorkerSlot(slot) => write!(formatter, "unknown worker slot {slot}"),
            Self::InvalidWorkerAddress(worker) => write!(
                formatter,
                "worker address {}:{} is invalid",
                worker.slot, worker.generation
            ),
            Self::StaleWorker { requested, current } => write!(
                formatter,
                "worker {}:{} is stale; current generation is {}:{}",
                requested.slot, requested.generation, current.slot, current.generation
            ),
            Self::WorkerNotReady(worker) => write!(
                formatter,
                "worker {}:{} is not initialized",
                worker.slot, worker.generation
            ),
            Self::WorkerInitializationMissing(worker) => write!(
                formatter,
                "worker {}:{} cannot become ready before initialize is accepted",
                worker.slot, worker.generation
            ),
            Self::WorkerInitializeAlreadyAccepted(worker) => write!(
                formatter,
                "worker {}:{} already accepted initialize",
                worker.slot, worker.generation
            ),
            Self::WorkerAlreadyReady(worker) => write!(
                formatter,
                "worker {}:{} is already initialized",
                worker.slot, worker.generation
            ),
            Self::WorkerGenerationExhausted(slot) => {
                write!(formatter, "worker slot {slot} exhausted its generation")
            }
            Self::MessageSequence {
                worker,
                requested,
                expected,
            } => write!(
                formatter,
                "worker {}:{} message sequence {requested} does not match {expected}",
                worker.slot, worker.generation
            ),
            Self::MessageSequenceExhausted(worker) => write!(
                formatter,
                "worker {}:{} exhausted its message sequence",
                worker.slot, worker.generation
            ),
            Self::InvalidOperationEnvelope => write!(
                formatter,
                "worker operation envelope has invalid initialize/operation ownership"
            ),
            Self::InvalidOperation(operation) => {
                write!(formatter, "operation {operation} is invalid")
            }
            Self::OperationAlreadyRegistered(operation) => {
                write!(formatter, "operation {operation} is already registered")
            }
            Self::UnknownOperation(operation) => {
                write!(formatter, "unknown operation {operation}")
            }
            Self::OperationTerminal(operation) => {
                write!(formatter, "operation {operation} is already terminal")
            }
            Self::OperationHasNoWorkers(operation) => {
                write!(formatter, "operation {operation} has no assigned workers")
            }
            Self::WorkerNotAssigned { operation, worker } => write!(
                formatter,
                "worker {}:{} is not assigned to operation {operation}",
                worker.slot, worker.generation
            ),
            Self::InvalidRouteIdentity { kind } => {
                write!(formatter, "worker {kind} route identity must be non-zero")
            }
            Self::TransferCount { requested, maximum } => write!(
                formatter,
                "worker message has {requested} transfers, exceeding {maximum}"
            ),
            Self::InvalidTransferId(id) => {
                write!(
                    formatter,
                    "worker transfer id {id} is invalid or duplicated"
                )
            }
            Self::EmptyTransfer(id) => write!(formatter, "worker transfer {id} is empty"),
            Self::TransferBytes { requested, maximum } => write!(
                formatter,
                "worker message transfers {requested} bytes, exceeding {maximum}"
            ),
            #[cfg(feature = "debug-control")]
            Self::InvalidStopGeneration(generation) => {
                write!(formatter, "worker stop generation {generation} is invalid")
            }
            #[cfg(feature = "debug-control")]
            Self::StopAlreadyActive {
                operation,
                generation,
            } => write!(
                formatter,
                "operation {operation} already has worker stop generation {generation}"
            ),
            #[cfg(feature = "debug-control")]
            Self::StopNotActive(operation) => {
                write!(formatter, "operation {operation} has no active worker stop")
            }
            #[cfg(feature = "debug-control")]
            Self::StaleStopGeneration {
                operation,
                requested,
                current,
            } => write!(
                formatter,
                "operation {operation} worker stop {requested} is stale; current is {current}"
            ),
            #[cfg(feature = "debug-control")]
            Self::WorkerNotInStop { operation, worker } => write!(
                formatter,
                "worker {}:{} does not participate in operation {operation} stop",
                worker.slot, worker.generation
            ),
            #[cfg(feature = "debug-control")]
            Self::StopClassificationConflict { operation, worker } => write!(
                formatter,
                "worker {}:{} has conflicting classifications for operation {operation}",
                worker.slot, worker.generation
            ),
        }
    }
}

impl std::error::Error for WorkerCoordinatorError {}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::operation_handle::OperationTerminalStatus;

    fn operation(value: u64) -> OperationId {
        OperationId::from_raw(value)
    }

    fn succeeded() -> OperationTerminalSummary {
        OperationTerminalSummary {
            status: OperationTerminalStatus::Succeeded,
            cause_code: None,
            diagnostic_count: 0,
            recovered_control_failure_count: 0,
            retained_artifact_count: 0,
            discarded_artifact_count: 0,
            restartable: None,
        }
    }

    fn initialized_worker(coordinator: &mut WorkerCoordinator, slot: u64) -> WorkerAddress {
        let worker = coordinator.worker(WorkerSlotId::from_raw(slot)).unwrap();
        let initialized = WorkerEnvelope::new(
            worker,
            1,
            OperationHostEnvelope::initialize(json!({ "runtime": "fixture" })),
            Vec::new(),
        );
        coordinator.accept(&initialized).unwrap();
        coordinator.mark_ready(worker).unwrap();
        worker
    }

    #[test]
    fn envelope_versions_sequences_and_transfer_bounds_fail_without_state_mutation() {
        let limits = WorkerCoordinatorLimits {
            max_workers: 2,
            max_transfer_buffers_per_message: 2,
            max_transfer_bytes_per_message: 8,
        };
        let mut coordinator = WorkerCoordinator::with_limits(1, limits).unwrap();
        let worker = coordinator.worker(WorkerSlotId::from_raw(1)).unwrap();
        assert_eq!(
            coordinator.mark_ready(worker).unwrap_err().code(),
            "cem.worker.initialize_missing"
        );
        let initialized = WorkerEnvelope::new(
            worker,
            1,
            OperationHostEnvelope::initialize(json!({ "runtime": "fixture" })),
            Vec::new(),
        );
        coordinator.accept(&initialized).unwrap();
        let duplicate_initialize = WorkerEnvelope::new(
            worker,
            2,
            OperationHostEnvelope::initialize(json!({ "runtime": "duplicate" })),
            Vec::new(),
        );
        assert_eq!(
            coordinator
                .accept(&duplicate_initialize)
                .unwrap_err()
                .code(),
            "cem.worker.initialize_duplicate"
        );
        coordinator.mark_ready(worker).unwrap();
        let operation = operation(11);
        coordinator.register_operation(operation).unwrap();
        coordinator.assign_worker(operation, worker).unwrap();

        assert_eq!(
            coordinator
                .accept(&duplicate_initialize)
                .unwrap_err()
                .code(),
            "cem.worker.already_ready"
        );

        let message = OperationHostEnvelope::operation(
            OperationHostMessageKind::Event,
            operation,
            Some(1),
            json!({ "event": "progress" }),
        )
        .unwrap();
        let duplicate = TransferBufferDescriptor {
            id: TransferBufferId::from_raw(1),
            byte_length: 4,
        };
        let invalid = WorkerEnvelope::new(worker, 2, message.clone(), vec![duplicate, duplicate]);
        assert_eq!(
            coordinator.accept(&invalid).unwrap_err().code(),
            "cem.worker.transfer_id_invalid"
        );

        let oversized = WorkerEnvelope::new(
            worker,
            2,
            message.clone(),
            vec![TransferBufferDescriptor {
                id: TransferBufferId::from_raw(2),
                byte_length: 9,
            }],
        );
        assert_eq!(
            coordinator.accept(&oversized).unwrap_err().code(),
            "cem.worker.transfer_bytes"
        );

        let accepted = WorkerEnvelope::new(
            worker,
            2,
            message,
            vec![TransferBufferDescriptor {
                id: TransferBufferId::from_raw(3),
                byte_length: 8,
            }],
        );
        coordinator.accept(&accepted).unwrap();
        assert_eq!(
            coordinator.accept(&accepted).unwrap_err(),
            WorkerCoordinatorError::MessageSequence {
                worker,
                requested: 2,
                expected: 3,
            }
        );

        let mut wrong_version = WorkerEnvelope::new(
            worker,
            3,
            OperationHostEnvelope::operation(
                OperationHostMessageKind::Progress,
                operation,
                Some(2),
                json!({}),
            )
            .unwrap(),
            Vec::new(),
        );
        wrong_version.worker_protocol_version += 1;
        assert_eq!(
            coordinator.accept(&wrong_version).unwrap_err().code(),
            "cem.worker.protocol_version"
        );
        wrong_version.worker_protocol_version = WORKER_PROTOCOL_VERSION;
        coordinator.accept(&wrong_version).unwrap();
    }

    #[test]
    fn coordinator_routes_stable_identities_and_replacement_invalidates_old_generation() {
        let mut coordinator = WorkerCoordinator::new(2).unwrap();
        let first = initialized_worker(&mut coordinator, 1);
        let second = initialized_worker(&mut coordinator, 2);
        let operation = operation(21);
        coordinator.register_operation(operation).unwrap();
        coordinator.assign_worker(operation, first).unwrap();
        coordinator.assign_worker(operation, second).unwrap();

        let scope = ExecutionScopeId::from_raw(31);
        let task = TaskId::from_raw(41);
        let subscription = EventSubscriptionId::from_raw(51);
        let retained = RetainedHandleId::from_raw(61);
        coordinator.assign_scope(operation, scope, first).unwrap();
        coordinator.assign_scope(operation, scope, second).unwrap();
        assert_eq!(
            coordinator
                .route_task(operation, TaskId::from_raw(0), first)
                .unwrap_err()
                .code(),
            "cem.worker.route_identity_invalid"
        );
        coordinator.route_task(operation, task, first).unwrap();
        coordinator
            .route_subscription(operation, subscription, first)
            .unwrap();
        coordinator
            .route_retained_handle(operation, retained, first)
            .unwrap();
        #[cfg(feature = "debug-control")]
        {
            coordinator
                .route_snapshot_reference(operation, SnapshotReferenceId::from_raw(71), first)
                .unwrap();
            assert_eq!(
                coordinator
                    .snapshot_reference_worker(operation, SnapshotReferenceId::from_raw(71))
                    .unwrap(),
                Some(first)
            );
        }
        assert_eq!(
            coordinator.operation_workers(operation).unwrap(),
            &BTreeSet::from([first, second])
        );
        assert_eq!(
            coordinator.scope_workers(operation, scope).unwrap(),
            Some(&BTreeSet::from([first, second]))
        );
        assert_eq!(
            coordinator
                .subscription_worker(operation, subscription)
                .unwrap(),
            Some(first)
        );
        assert_eq!(
            coordinator
                .retained_handle_worker(operation, retained)
                .unwrap(),
            Some(first)
        );

        let replacement = coordinator.replace_worker(first.slot).unwrap();
        assert_eq!(replacement.previous, first);
        assert_eq!(replacement.replacement.generation.get(), 2);
        assert_eq!(replacement.affected_operations, BTreeSet::from([operation]));
        assert_eq!(
            replacement.invalidated_scope_routes,
            vec![(operation, scope)]
        );
        assert_eq!(replacement.invalidated_task_routes, vec![(operation, task)]);
        assert_eq!(
            replacement.invalidated_subscriptions,
            vec![(operation, subscription)]
        );
        assert_eq!(
            replacement.invalidated_retained_handles,
            vec![(operation, retained)]
        );
        assert_eq!(coordinator.task_worker(operation, task).unwrap(), None);

        let late = WorkerEnvelope::new(
            first,
            2,
            OperationHostEnvelope::operation(
                OperationHostMessageKind::Event,
                operation,
                Some(1),
                json!({}),
            )
            .unwrap(),
            Vec::new(),
        );
        assert_eq!(
            coordinator.accept(&late).unwrap_err().code(),
            "cem.worker.generation_stale"
        );

        let replacement_worker = replacement.replacement;
        let initialized = WorkerEnvelope::new(
            replacement_worker,
            1,
            OperationHostEnvelope::initialize(json!({ "runtime": "replacement" })),
            Vec::new(),
        );
        coordinator.accept(&initialized).unwrap();
        coordinator.mark_ready(replacement_worker).unwrap();
        coordinator
            .assign_worker(operation, replacement_worker)
            .unwrap();
    }

    #[cfg(feature = "debug-control")]
    #[test]
    fn all_stop_completes_only_after_every_worker_is_parked_or_in_external_wait() {
        let mut coordinator = WorkerCoordinator::new(2).unwrap();
        let first = initialized_worker(&mut coordinator, 1);
        let second = initialized_worker(&mut coordinator, 2);
        let operation = operation(81);
        coordinator.register_operation(operation).unwrap();
        coordinator.assign_worker(operation, first).unwrap();
        coordinator.assign_worker(operation, second).unwrap();
        let generation = WorkerStopGeneration::from_raw(1);

        assert_eq!(
            coordinator.begin_stop(operation, generation).unwrap(),
            WorkerStopRendezvousStatus::Pending {
                awaiting: BTreeSet::from([first, second]),
            }
        );
        assert_eq!(
            coordinator
                .acknowledge_stop(operation, generation, first, WorkerStopDisposition::Parked,)
                .unwrap(),
            WorkerStopRendezvousStatus::Pending {
                awaiting: BTreeSet::from([second]),
            }
        );
        assert_eq!(
            coordinator
                .acknowledge_stop(
                    operation,
                    generation,
                    second,
                    WorkerStopDisposition::ExternalWait,
                )
                .unwrap(),
            WorkerStopRendezvousStatus::Complete {
                parked: BTreeSet::from([first]),
                external_wait: BTreeSet::from([second]),
            }
        );
        assert_eq!(
            coordinator
                .acknowledge_stop(
                    operation,
                    generation,
                    first,
                    WorkerStopDisposition::ExternalWait,
                )
                .unwrap_err()
                .code(),
            "cem.worker.stop_classification_conflict"
        );

        coordinator.clear_stop(operation, generation).unwrap();
        coordinator
            .begin_stop(operation, WorkerStopGeneration::from_raw(2))
            .unwrap();
        let replacement = coordinator.replace_worker(first.slot).unwrap();
        assert_eq!(
            replacement.invalidated_stop_rendezvous,
            BTreeSet::from([operation])
        );
        assert_eq!(
            coordinator
                .acknowledge_stop(
                    operation,
                    WorkerStopGeneration::from_raw(2),
                    second,
                    WorkerStopDisposition::Parked,
                )
                .unwrap_err()
                .code(),
            "cem.worker.stop_missing"
        );
    }

    #[test]
    fn one_terminal_claim_wins_and_prevents_new_routes() {
        let mut coordinator = WorkerCoordinator::new(1).unwrap();
        let worker = initialized_worker(&mut coordinator, 1);
        let operation = operation(91);
        coordinator.register_operation(operation).unwrap();
        coordinator.assign_worker(operation, worker).unwrap();

        let first = coordinator.claim_terminal(operation, succeeded()).unwrap();
        assert!(first.published);
        let mut conflicting = succeeded();
        conflicting.status = OperationTerminalStatus::Fatal;
        conflicting.cause_code = Some("cem.worker.fixture".to_owned());
        conflicting.restartable = Some(false);
        let second = coordinator.claim_terminal(operation, conflicting).unwrap();
        assert!(!second.published);
        assert_eq!(second.terminal, first.terminal);
        assert_eq!(
            coordinator.assign_worker(operation, worker).unwrap_err(),
            WorkerCoordinatorError::OperationTerminal(operation)
        );
    }

    #[test]
    fn serialized_envelope_exposes_stable_generation_and_transfer_metadata() {
        let worker = WorkerAddress::new(WorkerSlotId::from_raw(3), WorkerGeneration::from_raw(7));
        let envelope = WorkerEnvelope::new(
            worker,
            9,
            OperationHostEnvelope::initialize(json!({ "host": "browser" })),
            vec![TransferBufferDescriptor {
                id: TransferBufferId::from_raw(11),
                byte_length: 13,
            }],
        );
        assert_eq!(
            serde_json::to_value(envelope).unwrap(),
            json!({
                "workerProtocolVersion": 1,
                "worker": { "slot": 3, "generation": 7 },
                "sequence": 9,
                "operation": {
                    "protocolVersion": 1,
                    "kind": "initialize",
                    "payload": { "host": "browser" }
                },
                "transfers": [{ "id": 11, "byteLength": 13 }]
            })
        );
    }
}
