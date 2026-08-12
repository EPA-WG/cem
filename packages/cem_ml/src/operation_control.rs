//! Operation-local control identities, execution scopes, cancellation, and
//! hierarchical resource accounting.
//!
//! This is the common semantic layer used by native and WASM hosts. Live
//! control state remains outside serializable run requests.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::scheduler::{AbortSignal, ResourceCap, ScopePolicy};
use crate::source::ByteRange;
use crate::source_map::SourceMapStack;

pub const ROOT_EXECUTION_SCOPE_ID: ExecutionScopeId = ExecutionScopeId(0);
/// Maximum number of caller-defined work units allowed between cooperative
/// control checks. Callers must report work in naturally bounded units such as
/// tokens, evaluator nodes, loop iterations, or output chunks.
pub const DEFAULT_SAFE_POINT_WORK_INTERVAL: u32 = 64;
pub const MAX_CONTROL_REASON_BYTES: usize = 512;
pub const MAX_EXECUTION_SCOPE_LABEL_BYTES: usize = 256;
pub const MAX_SCOPE_IDENTITY_BYTES: usize = 128;
pub const MAX_SOURCE_URI_BYTES: usize = 2_048;
pub const MAX_BOUNDARY_OWNER_BYTES: usize = 128;
pub const MAX_RESULT_CONTRACT_BYTES: usize = 256;
pub const MAX_CLEANUP_LABEL_BYTES: usize = 128;

static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

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

opaque_id!(OperationId);
opaque_id!(ExecutionScopeId);
opaque_id!(TaskId);
opaque_id!(FailureDeliveryId);
opaque_id!(CleanupActionId);
opaque_id!(MemoryPermitId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionScopeKind {
    Operation,
    Document,
    Parse,
    Schema,
    Handoff,
    Query,
    Template,
    Transform,
    Resolver,
    Plugin,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionScopeState {
    Queued,
    Running,
    Waiting,
    Parked,
    Unwinding,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScopeIdentityKind {
    RunConfig,
    Scheduler,
    SchemaParser,
    Query,
    Registry,
    Plugin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlCauseKind {
    HostCancellation,
    Superseded,
    StackDepthExceeded,
    MemoryExceeded,
    TimeoutExceeded,
    QueueCapacityExceeded,
    WorkerFailure,
    InternalFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ControlBoundaryPolicy {
    Recover {
        accepted_causes: BTreeSet<ControlCauseKind>,
    },
    FailFast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlErrorBoundary {
    pub owner: String,
    pub result_contract: String,
    pub policy: ControlBoundaryPolicy,
}

impl ControlErrorBoundary {
    pub fn recover(
        owner: impl Into<String>,
        result_contract: impl Into<String>,
        accepted_causes: impl IntoIterator<Item = ControlCauseKind>,
    ) -> Self {
        Self {
            owner: owner.into(),
            result_contract: result_contract.into(),
            policy: ControlBoundaryPolicy::Recover {
                accepted_causes: accepted_causes.into_iter().collect(),
            },
        }
    }

    pub fn fail_fast(owner: impl Into<String>, result_contract: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            result_contract: result_contract.into(),
            policy: ControlBoundaryPolicy::FailFast,
        }
    }

    fn accepts(&self, cause: ControlCauseKind) -> bool {
        matches!(
            &self.policy,
            ControlBoundaryPolicy::Recover { accepted_causes }
                if accepted_causes.contains(&cause)
        )
    }
}

pub type ScopeIdentityMap = BTreeMap<ScopeIdentityKind, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    pub source_uri: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_range: Option<ByteRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionScope {
    pub id: ExecutionScopeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ExecutionScopeId>,
    pub kind: ExecutionScopeKind,
    pub label: String,
    pub state: ExecutionScopeState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
    #[serde(default)]
    pub semantic_identities: ScopeIdentityMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_boundary: Option<ControlErrorBoundary>,
    pub effective_policy: ScopePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTask {
    pub id: TaskId,
    pub owner: ExecutionScopeId,
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct ExecutionScopeRegistration {
    pub kind: ExecutionScopeKind,
    pub label: String,
    pub source_location: Option<SourceLocation>,
    pub semantic_identities: ScopeIdentityMap,
    pub error_boundary: Option<ControlErrorBoundary>,
    pub effective_policy: ScopePolicy,
}

impl ExecutionScopeRegistration {
    pub fn inherited(
        kind: ExecutionScopeKind,
        label: impl Into<String>,
        parent: ScopePolicy,
    ) -> Self {
        Self {
            kind,
            label: label.into(),
            source_location: None,
            semantic_identities: ScopeIdentityMap::new(),
            error_boundary: None,
            effective_policy: parent,
        }
    }

    pub fn with_error_boundary(mut self, boundary: ControlErrorBoundary) -> Self {
        self.error_boundary = Some(boundary);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionScopeTree {
    operation_id: OperationId,
    root: ExecutionScopeId,
    scopes: BTreeMap<ExecutionScopeId, ExecutionScope>,
    tasks: BTreeMap<TaskId, ExecutionTask>,
    #[serde(skip)]
    next_scope_id: u64,
    #[serde(skip)]
    next_task_id: u64,
}

impl ExecutionScopeTree {
    fn new(operation_id: OperationId, root_policy: ScopePolicy) -> Self {
        let root = ExecutionScope {
            id: ROOT_EXECUTION_SCOPE_ID,
            parent: None,
            kind: ExecutionScopeKind::Operation,
            label: "operation".to_owned(),
            state: ExecutionScopeState::Running,
            source_location: None,
            semantic_identities: ScopeIdentityMap::new(),
            error_boundary: None,
            effective_policy: root_policy,
        };
        Self {
            operation_id,
            root: ROOT_EXECUTION_SCOPE_ID,
            scopes: BTreeMap::from([(ROOT_EXECUTION_SCOPE_ID, root)]),
            tasks: BTreeMap::new(),
            next_scope_id: 1,
            next_task_id: 1,
        }
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub fn root(&self) -> ExecutionScopeId {
        self.root
    }

    pub fn scope(&self, id: ExecutionScopeId) -> Option<&ExecutionScope> {
        self.scopes.get(&id)
    }

    pub fn scopes(&self) -> impl ExactSizeIterator<Item = &ExecutionScope> {
        self.scopes.values()
    }

    pub fn task(&self, id: TaskId) -> Option<&ExecutionTask> {
        self.tasks.get(&id)
    }

    pub fn tasks(&self) -> impl ExactSizeIterator<Item = &ExecutionTask> {
        self.tasks.values()
    }

    pub fn ancestors(&self, scope: ExecutionScopeId) -> Vec<ExecutionScopeId> {
        let mut ancestors = Vec::new();
        let mut cursor = Some(scope);
        while let Some(id) = cursor {
            let Some(node) = self.scopes.get(&id) else {
                break;
            };
            ancestors.push(id);
            cursor = node.parent;
        }
        ancestors
    }

    pub fn descendants(&self, scope: ExecutionScopeId) -> Vec<ExecutionScopeId> {
        self.scopes
            .keys()
            .copied()
            .filter(|candidate| self.ancestors(*candidate).contains(&scope))
            .collect()
    }

    fn allocate_scope_id(&mut self) -> Result<ExecutionScopeId, ControlError> {
        let id = ExecutionScopeId(self.next_scope_id);
        self.next_scope_id = self
            .next_scope_id
            .checked_add(1)
            .ok_or(ControlError::IdentityExhausted("executionScopeId"))?;
        Ok(id)
    }

    fn allocate_task_id(&mut self) -> Result<TaskId, ControlError> {
        let id = TaskId(self.next_task_id);
        self.next_task_id = self
            .next_task_id
            .checked_add(1)
            .ok_or(ControlError::IdentityExhausted("taskId"))?;
        Ok(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ControlCause {
    HostCancellation {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Superseded {
        revision: u64,
    },
    StackDepthExceeded {
        observed: u32,
        limit: u32,
    },
    MemoryExceeded {
        requested: u64,
        charged: u64,
        limit: u64,
    },
    TimeoutExceeded {
        active_elapsed_ms: u64,
        limit_ms: u64,
    },
    QueueCapacityExceeded {
        capacity: u32,
    },
    WorkerFailure {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worker: Option<u64>,
        restartable: bool,
    },
    InternalFailure {
        diagnostic_code: String,
    },
}

impl ControlCause {
    pub fn kind(&self) -> ControlCauseKind {
        match self {
            Self::HostCancellation { .. } => ControlCauseKind::HostCancellation,
            Self::Superseded { .. } => ControlCauseKind::Superseded,
            Self::StackDepthExceeded { .. } => ControlCauseKind::StackDepthExceeded,
            Self::MemoryExceeded { .. } => ControlCauseKind::MemoryExceeded,
            Self::TimeoutExceeded { .. } => ControlCauseKind::TimeoutExceeded,
            Self::QueueCapacityExceeded { .. } => ControlCauseKind::QueueCapacityExceeded,
            Self::WorkerFailure { .. } => ControlCauseKind::WorkerFailure,
            Self::InternalFailure { .. } => ControlCauseKind::InternalFailure,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::HostCancellation { .. } => "host-cancellation",
            Self::Superseded { .. } => "superseded",
            Self::StackDepthExceeded { .. } => "stack-depth-exceeded",
            Self::MemoryExceeded { .. } => "memory-exceeded",
            Self::TimeoutExceeded { .. } => "timeout-exceeded",
            Self::QueueCapacityExceeded { .. } => "queue-capacity-exceeded",
            Self::WorkerFailure { .. } => "worker-failure",
            Self::InternalFailure { .. } => "internal-failure",
        }
    }

    pub fn terminal_class(&self) -> ControlTerminalClass {
        match self {
            Self::HostCancellation { .. } | Self::Superseded { .. } => {
                ControlTerminalClass::Cancelled
            }
            Self::WorkerFailure {
                restartable: false, ..
            }
            | Self::InternalFailure { .. } => ControlTerminalClass::Fatal,
            _ => ControlTerminalClass::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlTerminalClass {
    Cancelled,
    Failed,
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlFailure {
    pub operation_id: OperationId,
    pub affected_scope: ExecutionScopeId,
    pub cause: ControlCause,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
}

impl ControlFailure {
    pub fn code(&self) -> &'static str {
        self.cause.code()
    }

    pub fn terminal_class(&self) -> ControlTerminalClass {
        self.cause.terminal_class()
    }
}

impl fmt::Display for ControlFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "operation {} scope {} stopped by {}",
            self.operation_id,
            self.affected_scope,
            self.cause.code()
        )
    }
}

impl std::error::Error for ControlFailure {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeCleanupRecord {
    pub scope: ExecutionScopeId,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlUnwindSummary {
    pub completed_scopes: Vec<ExecutionScopeId>,
    pub completed_tasks: Vec<TaskId>,
    pub cleanup_actions: Vec<ScopeCleanupRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureDeliveryToken {
    pub operation_id: OperationId,
    pub delivery_id: FailureDeliveryId,
    pub affected_scope: ExecutionScopeId,
    pub boundary_scope: ExecutionScopeId,
    pub cause_kind: ControlCauseKind,
    pub boundary_owner: String,
    pub result_contract: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlFailureSettlementKind {
    Recovered,
    EscalatedToRoot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlFailureSettlement {
    pub failure: ControlFailure,
    pub kind: ControlFailureSettlementKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_scope: Option<ExecutionScopeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_contract: Option<String>,
    pub unwind: ControlUnwindSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureDeliveryOutcome {
    AwaitingCleanup {
        affected_scope: ExecutionScopeId,
        unfinished_tasks: Vec<TaskId>,
    },
    Deliver(FailureDeliveryToken),
    Settled(ControlFailureSettlement),
}

#[derive(Debug, PartialEq, Eq)]
pub enum TypedRecoveryError<E> {
    Validation(E),
    Control(ControlError),
}

impl<E> From<ControlError> for TypedRecoveryError<E> {
    fn from(error: ControlError) -> Self {
        Self::Control(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    UnknownScope(ExecutionScopeId),
    UnknownTask(TaskId),
    TaskScopeMismatch {
        task: TaskId,
        owner: ExecutionScopeId,
        requested_scope: ExecutionScopeId,
    },
    ScopeCompleted(ExecutionScopeId),
    ScopeUnwinding(ExecutionScopeId),
    InvalidScopeTransition {
        scope: ExecutionScopeId,
        from: ExecutionScopeState,
        to: ExecutionScopeState,
    },
    InvalidLabel,
    InvalidSourceLocation,
    InvalidSemanticIdentity(ScopeIdentityKind),
    InvalidBoundary,
    InvalidCleanupLabel,
    InvalidReason,
    InvalidPolicy(String),
    CapRelaxationDenied {
        scope: ExecutionScopeId,
        cap: ResourceCap,
        parent_value: u64,
        attempted_value: u64,
    },
    IdentityExhausted(&'static str),
    UnknownDelivery(FailureDeliveryId),
    ForeignDelivery(OperationId),
    DeliverySettled(FailureDeliveryId),
    FailureMismatch(ExecutionScopeId),
    Triggered(ControlFailure),
}

impl ControlError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownScope(_) => "cem.control.unknown_scope",
            Self::UnknownTask(_) => "cem.control.unknown_task",
            Self::TaskScopeMismatch { .. } => "cem.control.task_scope_mismatch",
            Self::ScopeCompleted(_) => "cem.control.scope_completed",
            Self::ScopeUnwinding(_) => "cem.control.scope_unwinding",
            Self::InvalidScopeTransition { .. } => "cem.control.scope_transition_invalid",
            Self::InvalidLabel => "cem.control.scope_label_invalid",
            Self::InvalidSourceLocation => "cem.control.source_location_invalid",
            Self::InvalidSemanticIdentity(_) => "cem.control.semantic_identity_invalid",
            Self::InvalidBoundary => "cem.control.error_boundary_invalid",
            Self::InvalidCleanupLabel => "cem.control.cleanup_label_invalid",
            Self::InvalidReason => "cem.control.reason_invalid",
            Self::InvalidPolicy(_) => "cem.control.policy_invalid",
            Self::CapRelaxationDenied { .. } => "cem.a.cap_relaxation_denied",
            Self::IdentityExhausted(_) => "cem.control.identity_exhausted",
            Self::UnknownDelivery(_) => "cem.control.delivery_unknown",
            Self::ForeignDelivery(_) => "cem.control.delivery_foreign_operation",
            Self::DeliverySettled(_) => "cem.control.delivery_settled",
            Self::FailureMismatch(_) => "cem.control.failure_mismatch",
            Self::Triggered(failure) => failure.code(),
        }
    }
}

impl fmt::Display for ControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownScope(scope) => write!(formatter, "unknown execution scope {scope}"),
            Self::UnknownTask(task) => write!(formatter, "unknown logical task {task}"),
            Self::TaskScopeMismatch {
                task,
                owner,
                requested_scope,
            } => write!(
                formatter,
                "task {task} owned by scope {owner} cannot enter unrelated scope {requested_scope}"
            ),
            Self::ScopeCompleted(scope) => write!(formatter, "execution scope {scope} completed"),
            Self::ScopeUnwinding(scope) => write!(formatter, "execution scope {scope} is unwinding"),
            Self::InvalidScopeTransition { scope, from, to } => write!(
                formatter,
                "execution scope {scope} cannot transition from {from:?} to {to:?}"
            ),
            Self::InvalidLabel => formatter.write_str("execution-scope label is empty or too long"),
            Self::InvalidSourceLocation => formatter.write_str("source location is invalid"),
            Self::InvalidSemanticIdentity(kind) => {
                write!(formatter, "semantic identity {kind:?} is empty or too long")
            }
            Self::InvalidBoundary => formatter.write_str(
                "error-boundary owner, result contract, or accepted cause set is invalid",
            ),
            Self::InvalidCleanupLabel => formatter.write_str(
                "cleanup label is empty, too long, or contains control characters",
            ),
            Self::InvalidReason => write!(
                formatter,
                "control reason contains control characters or exceeds {MAX_CONTROL_REASON_BYTES} bytes"
            ),
            Self::InvalidPolicy(message) => write!(formatter, "invalid control policy: {message}"),
            Self::CapRelaxationDenied {
                scope,
                cap,
                parent_value,
                attempted_value,
            } => write!(
                formatter,
                "scope {scope} attempted to raise cap {cap:?} from {parent_value} to {attempted_value}"
            ),
            Self::IdentityExhausted(identity) => write!(formatter, "{identity} space exhausted"),
            Self::UnknownDelivery(delivery) => {
                write!(formatter, "unknown failure delivery {delivery}")
            }
            Self::ForeignDelivery(operation) => {
                write!(formatter, "failure delivery belongs to operation {operation}")
            }
            Self::DeliverySettled(delivery) => {
                write!(formatter, "failure delivery {delivery} is already settled")
            }
            Self::FailureMismatch(scope) => {
                write!(formatter, "failure does not match the active cause for scope {scope}")
            }
            Self::Triggered(failure) => failure.fmt(formatter),
        }
    }
}

impl std::error::Error for ControlError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlRequestOutcome {
    Accepted,
    AlreadyRequested,
}

#[derive(Debug, Default)]
struct ScopeAccounting {
    memory_charged: u64,
    activated_at: Option<Instant>,
    excluded_time: Duration,
}

struct RegisteredCleanup {
    scope: ExecutionScopeId,
    label: String,
    action: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl fmt::Debug for RegisteredCleanup {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredCleanup")
            .field("scope", &self.scope)
            .field("label", &self.label)
            .field("pending", &self.action.is_some())
            .finish()
    }
}

#[derive(Debug, Clone)]
struct ActiveFailureDelivery {
    token: FailureDeliveryToken,
    failure: ControlFailure,
    unwind: ControlUnwindSummary,
}

#[derive(Debug, Clone)]
struct MemoryCharge {
    owner: ExecutionScopeId,
    ancestors: Vec<ExecutionScopeId>,
    bytes: u64,
}

#[derive(Debug)]
struct ControlInner {
    tree: ExecutionScopeTree,
    accounting: BTreeMap<ExecutionScopeId, ScopeAccounting>,
    stack_depths: BTreeMap<(TaskId, ExecutionScopeId), u32>,
    scope_causes: BTreeMap<ExecutionScopeId, (ControlCause, Option<SourceMapStack>)>,
    cleanup_actions: BTreeMap<CleanupActionId, RegisteredCleanup>,
    memory_permits: BTreeMap<MemoryPermitId, MemoryCharge>,
    active_deliveries: BTreeMap<FailureDeliveryId, ActiveFailureDelivery>,
    delivery_by_scope: BTreeMap<ExecutionScopeId, FailureDeliveryId>,
    settlements: BTreeMap<ExecutionScopeId, ControlFailureSettlement>,
    cleanup_in_progress: BTreeSet<ExecutionScopeId>,
    next_cleanup_action_id: u64,
    next_memory_permit_id: u64,
    next_delivery_id: u64,
}

/// Clone-shared operation control. The legacy [`AbortSignal`] is retained as
/// a facade over root host cancellation; scoped causes and resource accounting
/// are owned here.
#[derive(Debug, Clone)]
pub struct OperationControl {
    operation_id: OperationId,
    abort_signal: AbortSignal,
    inner: Arc<Mutex<ControlInner>>,
}

/// Reusable bounded-work cooperative control poller.
///
/// The poller owns only clone-shared control state and an operation-local work
/// counter. It does not mutate evaluator state, output, or source locations.
/// `force` is used at call/host boundaries; `poll` is used inside bounded loops.
#[derive(Debug, Clone)]
pub struct SafePointPoller {
    control: OperationControl,
    scope: ExecutionScopeId,
    work_interval: u32,
    work_since_check: u32,
}

impl SafePointPoller {
    pub fn new(control: OperationControl, scope: ExecutionScopeId) -> Self {
        Self::with_work_interval(control, scope, DEFAULT_SAFE_POINT_WORK_INTERVAL)
    }

    pub fn root(control: OperationControl) -> Self {
        Self::new(control, ROOT_EXECUTION_SCOPE_ID)
    }

    pub fn from_abort_signal(abort_signal: &AbortSignal) -> Self {
        Self::root(OperationControl::new(abort_signal.clone()))
    }

    pub fn with_work_interval(
        control: OperationControl,
        scope: ExecutionScopeId,
        work_interval: u32,
    ) -> Self {
        Self {
            control,
            scope,
            work_interval: work_interval.max(1),
            work_since_check: 0,
        }
    }

    pub fn scope(&self) -> ExecutionScopeId {
        self.scope
    }

    pub fn force(&mut self) -> Result<(), ControlError> {
        self.work_since_check = 0;
        self.control.check_scope(self.scope)
    }

    pub fn poll(&mut self, work_units: u32) -> Result<(), ControlError> {
        let total = self.work_since_check.saturating_add(work_units);
        if total < self.work_interval {
            self.work_since_check = total;
            return Ok(());
        }
        self.work_since_check = total % self.work_interval;
        self.control.check_scope(self.scope)
    }

    pub fn poll_one(&mut self) -> Result<(), ControlError> {
        self.poll(1)
    }
}

impl Default for OperationControl {
    fn default() -> Self {
        Self::new(AbortSignal::new())
    }
}

impl OperationControl {
    pub fn new(abort_signal: AbortSignal) -> Self {
        Self::with_policy(next_operation_id(), abort_signal, ScopePolicy::host_root())
            .expect("host root policy is valid")
    }

    pub fn with_root_policy(
        abort_signal: AbortSignal,
        root_policy: ScopePolicy,
    ) -> Result<Self, ControlError> {
        Self::with_policy(next_operation_id(), abort_signal, root_policy)
    }

    pub fn with_policy(
        operation_id: OperationId,
        abort_signal: AbortSignal,
        root_policy: ScopePolicy,
    ) -> Result<Self, ControlError> {
        validate_policy(root_policy)?;
        let tree = ExecutionScopeTree::new(operation_id, root_policy);
        let mut accounting = BTreeMap::new();
        accounting.insert(
            ROOT_EXECUTION_SCOPE_ID,
            ScopeAccounting {
                activated_at: Some(Instant::now()),
                ..ScopeAccounting::default()
            },
        );
        Ok(Self {
            operation_id,
            abort_signal,
            inner: Arc::new(Mutex::new(ControlInner {
                tree,
                accounting,
                stack_depths: BTreeMap::new(),
                scope_causes: BTreeMap::new(),
                cleanup_actions: BTreeMap::new(),
                memory_permits: BTreeMap::new(),
                active_deliveries: BTreeMap::new(),
                delivery_by_scope: BTreeMap::new(),
                settlements: BTreeMap::new(),
                cleanup_in_progress: BTreeSet::new(),
                next_cleanup_action_id: 1,
                next_memory_permit_id: 1,
                next_delivery_id: 1,
            })),
        })
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub fn root_scope(&self) -> ExecutionScopeId {
        ROOT_EXECUTION_SCOPE_ID
    }

    pub fn abort_signal(&self) -> &AbortSignal {
        &self.abort_signal
    }

    pub fn is_cancelled(&self) -> bool {
        self.abort_signal.is_aborted()
    }

    pub fn scope_tree(&self) -> ExecutionScopeTree {
        self.inner
            .lock()
            .expect("poisoned operation-control mutex")
            .tree
            .clone()
    }

    pub fn register_scope(
        &self,
        parent: ExecutionScopeId,
        registration: ExecutionScopeRegistration,
    ) -> Result<ExecutionScopeId, ControlError> {
        validate_registration(&registration)?;
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let parent_node = inner
            .tree
            .scope(parent)
            .ok_or(ControlError::UnknownScope(parent))?;
        if parent_node.state == ExecutionScopeState::Completed {
            return Err(ControlError::ScopeCompleted(parent));
        }
        if parent_node.state == ExecutionScopeState::Unwinding {
            let ancestors = inner.tree.ancestors(parent);
            if let Some(failure) = failure_for_ancestors(self.operation_id, &inner, &ancestors) {
                return Err(ControlError::Triggered(failure));
            }
            return Err(ControlError::ScopeUnwinding(parent));
        }
        let parent_policy = parent_node.effective_policy;
        let scope = ExecutionScopeId::from_raw(inner.tree.next_scope_id);
        check_constrain_only(scope, parent_policy, registration.effective_policy)?;
        let scope = inner.tree.allocate_scope_id()?;
        inner.tree.scopes.insert(
            scope,
            ExecutionScope {
                id: scope,
                parent: Some(parent),
                kind: registration.kind,
                label: registration.label,
                state: ExecutionScopeState::Queued,
                source_location: registration.source_location,
                semantic_identities: registration.semantic_identities,
                error_boundary: registration.error_boundary,
                effective_policy: registration.effective_policy,
            },
        );
        inner.accounting.insert(scope, ScopeAccounting::default());
        Ok(scope)
    }

    pub fn set_scope_state(
        &self,
        scope: ExecutionScopeId,
        state: ExecutionScopeState,
    ) -> Result<(), ControlError> {
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let node = inner
            .tree
            .scopes
            .get_mut(&scope)
            .ok_or(ControlError::UnknownScope(scope))?;
        if node.state == ExecutionScopeState::Completed {
            return Err(ControlError::ScopeCompleted(scope));
        }
        if !valid_scope_transition(node.state, state) {
            return Err(ControlError::InvalidScopeTransition {
                scope,
                from: node.state,
                to: state,
            });
        }
        node.state = state;
        if state == ExecutionScopeState::Running {
            let accounting = inner
                .accounting
                .get_mut(&scope)
                .expect("registered scope has accounting");
            accounting.activated_at.get_or_insert_with(Instant::now);
        }
        Ok(())
    }

    pub fn complete_scope(&self, scope: ExecutionScopeId) -> Result<(), ControlError> {
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let node = inner
            .tree
            .scopes
            .get_mut(&scope)
            .ok_or(ControlError::UnknownScope(scope))?;
        node.state = ExecutionScopeState::Completed;
        Ok(())
    }

    pub fn register_task(&self, owner: ExecutionScopeId) -> Result<TaskId, ControlError> {
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let scope = inner
            .tree
            .scope(owner)
            .ok_or(ControlError::UnknownScope(owner))?;
        if scope.state == ExecutionScopeState::Completed {
            return Err(ControlError::ScopeCompleted(owner));
        }
        if scope.state == ExecutionScopeState::Unwinding {
            let ancestors = inner.tree.ancestors(owner);
            if let Some(failure) = failure_for_ancestors(self.operation_id, &inner, &ancestors) {
                return Err(ControlError::Triggered(failure));
            }
            return Err(ControlError::ScopeUnwinding(owner));
        }
        let task = inner.tree.allocate_task_id()?;
        inner.tree.tasks.insert(
            task,
            ExecutionTask {
                id: task,
                owner,
                completed: false,
            },
        );
        Ok(task)
    }

    pub fn complete_task(&self, task: TaskId) -> Result<(), ControlError> {
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let task = inner
            .tree
            .tasks
            .get_mut(&task)
            .ok_or(ControlError::UnknownTask(task))?;
        task.completed = true;
        Ok(())
    }

    pub fn register_cleanup(
        &self,
        scope: ExecutionScopeId,
        label: impl Into<String>,
        action: impl FnOnce() + Send + 'static,
    ) -> Result<ScopeCleanupGuard, ControlError> {
        let label = label.into();
        if label.is_empty()
            || label.len() > MAX_CLEANUP_LABEL_BYTES
            || label.chars().any(char::is_control)
        {
            return Err(ControlError::InvalidCleanupLabel);
        }
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let node = inner
            .tree
            .scope(scope)
            .ok_or(ControlError::UnknownScope(scope))?;
        if node.state == ExecutionScopeState::Completed {
            return Err(ControlError::ScopeCompleted(scope));
        }
        let action_id = CleanupActionId::from_raw(inner.next_cleanup_action_id);
        inner.next_cleanup_action_id = inner
            .next_cleanup_action_id
            .checked_add(1)
            .ok_or(ControlError::IdentityExhausted("cleanupActionId"))?;
        inner.cleanup_actions.insert(
            action_id,
            RegisteredCleanup {
                scope,
                label,
                action: Some(Box::new(action)),
            },
        );
        Ok(ScopeCleanupGuard {
            inner: Arc::clone(&self.inner),
            action_id,
            released: false,
        })
    }

    pub fn cancel_root(
        &self,
        reason: Option<String>,
        source_map: Option<SourceMapStack>,
    ) -> Result<ControlRequestOutcome, ControlError> {
        validate_reason(reason.as_deref())?;
        let accepted = self
            .abort_signal
            .abort_with_metadata(reason.clone(), source_map.clone());
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        inner
            .scope_causes
            .entry(ROOT_EXECUTION_SCOPE_ID)
            .or_insert((ControlCause::HostCancellation { reason }, source_map));
        mark_subtree_unwinding(&mut inner.tree, ROOT_EXECUTION_SCOPE_ID);
        Ok(if accepted {
            ControlRequestOutcome::Accepted
        } else {
            ControlRequestOutcome::AlreadyRequested
        })
    }

    pub fn cancel_scope(
        &self,
        scope: ExecutionScopeId,
        reason: Option<String>,
        source_map: Option<SourceMapStack>,
    ) -> Result<ControlRequestOutcome, ControlError> {
        if scope == ROOT_EXECUTION_SCOPE_ID {
            return self.cancel_root(reason, source_map);
        }
        validate_reason(reason.as_deref())?;
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let state = inner
            .tree
            .scope(scope)
            .ok_or(ControlError::UnknownScope(scope))?
            .state;
        if state == ExecutionScopeState::Completed {
            return Err(ControlError::ScopeCompleted(scope));
        }
        if inner.scope_causes.contains_key(&scope) {
            return Ok(ControlRequestOutcome::AlreadyRequested);
        }
        inner.scope_causes.insert(
            scope,
            (ControlCause::HostCancellation { reason }, source_map),
        );
        mark_subtree_unwinding(&mut inner.tree, scope);
        Ok(ControlRequestOutcome::Accepted)
    }

    pub fn check_scope(&self, scope: ExecutionScopeId) -> Result<(), ControlError> {
        self.check_scope_at(scope, Instant::now())
    }

    pub fn fail_scope(
        &self,
        scope: ExecutionScopeId,
        cause: ControlCause,
        source_map: Option<SourceMapStack>,
    ) -> Result<ControlFailure, ControlError> {
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let node = inner
            .tree
            .scope(scope)
            .ok_or(ControlError::UnknownScope(scope))?;
        if node.state == ExecutionScopeState::Completed {
            return Err(ControlError::ScopeCompleted(scope));
        }
        if let Some((active_cause, active_source_map)) = inner.scope_causes.get(&scope) {
            if active_cause != &cause || active_source_map != &source_map {
                return Err(ControlError::FailureMismatch(scope));
            }
        } else {
            inner
                .scope_causes
                .insert(scope, (cause.clone(), source_map.clone()));
        }
        mark_subtree_unwinding(&mut inner.tree, scope);
        Ok(ControlFailure {
            operation_id: self.operation_id,
            affected_scope: scope,
            cause,
            source_map,
        })
    }

    pub fn prepare_failure_delivery(
        &self,
        failure: &ControlFailure,
    ) -> Result<FailureDeliveryOutcome, ControlError> {
        if failure.operation_id != self.operation_id {
            return Err(ControlError::ForeignDelivery(failure.operation_id));
        }

        let (actions, unwind) = {
            let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
            if let Some(settlement) = inner.settlements.get(&failure.affected_scope) {
                return Ok(FailureDeliveryOutcome::Settled(settlement.clone()));
            }
            if let Some(delivery_id) = inner.delivery_by_scope.get(&failure.affected_scope) {
                let delivery = inner
                    .active_deliveries
                    .get(delivery_id)
                    .expect("delivery-by-scope references a live delivery");
                return Ok(FailureDeliveryOutcome::Deliver(delivery.token.clone()));
            }
            let active = inner
                .scope_causes
                .get(&failure.affected_scope)
                .ok_or(ControlError::FailureMismatch(failure.affected_scope))?;
            if active.0 != failure.cause || active.1 != failure.source_map {
                return Err(ControlError::FailureMismatch(failure.affected_scope));
            }
            let subtree: BTreeSet<_> = inner
                .tree
                .descendants(failure.affected_scope)
                .into_iter()
                .collect();
            let unfinished_tasks: Vec<_> = inner
                .tree
                .tasks
                .values()
                .filter(|task| subtree.contains(&task.owner) && !task.completed)
                .map(|task| task.id)
                .collect();
            if !unfinished_tasks.is_empty()
                || inner.cleanup_in_progress.contains(&failure.affected_scope)
            {
                return Ok(FailureDeliveryOutcome::AwaitingCleanup {
                    affected_scope: failure.affected_scope,
                    unfinished_tasks,
                });
            }
            inner.cleanup_in_progress.insert(failure.affected_scope);

            let mut scopes: Vec<_> = subtree.iter().copied().collect();
            scopes.sort_by(|left, right| {
                inner
                    .tree
                    .ancestors(*right)
                    .len()
                    .cmp(&inner.tree.ancestors(*left).len())
                    .then_with(|| left.cmp(right))
            });

            let mut cleanup_ids: Vec<_> = inner
                .cleanup_actions
                .iter()
                .filter(|(_, action)| subtree.contains(&action.scope))
                .map(|(id, action)| (*id, action.scope, inner.tree.ancestors(action.scope).len()))
                .collect();
            cleanup_ids.sort_by(|left, right| {
                right
                    .2
                    .cmp(&left.2)
                    .then_with(|| left.1.cmp(&right.1))
                    .then_with(|| right.0.cmp(&left.0))
            });
            let mut actions = Vec::with_capacity(cleanup_ids.len());
            let mut cleanup_records = Vec::with_capacity(cleanup_ids.len());
            for (id, _, _) in cleanup_ids {
                if let Some(mut cleanup) = inner.cleanup_actions.remove(&id) {
                    cleanup_records.push(ScopeCleanupRecord {
                        scope: cleanup.scope,
                        label: cleanup.label.clone(),
                    });
                    if let Some(action) = cleanup.action.take() {
                        actions.push(action);
                    }
                }
            }

            let permit_ids: Vec<_> = inner
                .memory_permits
                .iter()
                .filter(|(_, charge)| subtree.contains(&charge.owner))
                .map(|(id, _)| *id)
                .collect();
            for permit_id in permit_ids {
                release_memory_permit(&mut inner, permit_id);
            }

            let subtree_tasks: BTreeSet<_> = inner
                .tree
                .tasks
                .values()
                .filter(|task| subtree.contains(&task.owner))
                .map(|task| task.id)
                .collect();
            inner
                .stack_depths
                .retain(|(task, _), _| !subtree_tasks.contains(task));
            for scope in &scopes {
                if let Some(node) = inner.tree.scopes.get_mut(scope) {
                    node.state = ExecutionScopeState::Completed;
                }
            }

            let unwind = ControlUnwindSummary {
                completed_scopes: scopes,
                completed_tasks: subtree_tasks.into_iter().collect(),
                cleanup_actions: cleanup_records,
            };
            (actions, unwind)
        };

        let mut cleanup_failed = false;
        for action in actions {
            cleanup_failed |= catch_unwind(AssertUnwindSafe(action)).is_err();
        }
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        inner.cleanup_in_progress.remove(&failure.affected_scope);
        if cleanup_failed {
            let cleanup_failure = ControlFailure {
                operation_id: self.operation_id,
                affected_scope: ROOT_EXECUTION_SCOPE_ID,
                cause: ControlCause::InternalFailure {
                    diagnostic_code: "cem.control.cleanup_failed".to_owned(),
                },
                source_map: failure.source_map.clone(),
            };
            inner.scope_causes.insert(
                ROOT_EXECUTION_SCOPE_ID,
                (
                    cleanup_failure.cause.clone(),
                    cleanup_failure.source_map.clone(),
                ),
            );
            mark_subtree_unwinding(&mut inner.tree, ROOT_EXECUTION_SCOPE_ID);
            let settlement = ControlFailureSettlement {
                failure: cleanup_failure,
                kind: ControlFailureSettlementKind::EscalatedToRoot,
                boundary_scope: None,
                result_contract: None,
                unwind,
            };
            inner
                .settlements
                .insert(failure.affected_scope, settlement.clone());
            inner
                .settlements
                .entry(ROOT_EXECUTION_SCOPE_ID)
                .or_insert_with(|| settlement.clone());
            return Ok(FailureDeliveryOutcome::Settled(settlement));
        }

        deliver_or_escalate_locked(self.operation_id, &mut inner, failure.clone(), unwind, None)
    }

    pub fn decline_failure_delivery(
        &self,
        token: &FailureDeliveryToken,
    ) -> Result<FailureDeliveryOutcome, ControlError> {
        self.validate_delivery_operation(token)?;
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let delivery = inner
            .active_deliveries
            .remove(&token.delivery_id)
            .ok_or_else(|| delivery_state_error(&inner, token))?;
        if delivery.token != *token {
            inner
                .active_deliveries
                .insert(delivery.token.delivery_id, delivery);
            return Err(ControlError::UnknownDelivery(token.delivery_id));
        }
        inner.delivery_by_scope.remove(&token.affected_scope);
        deliver_or_escalate_locked(
            self.operation_id,
            &mut inner,
            delivery.failure,
            delivery.unwind,
            Some(token.boundary_scope),
        )
    }

    pub fn validate_and_recover<T, E>(
        &self,
        token: &FailureDeliveryToken,
        replacement: &T,
        validate: impl FnOnce(&T, &str, &str) -> Result<(), E>,
    ) -> Result<ControlFailureSettlement, TypedRecoveryError<E>> {
        self.validate_delivery_operation(token)?;
        {
            let inner = self.inner.lock().expect("poisoned operation-control mutex");
            let delivery = inner
                .active_deliveries
                .get(&token.delivery_id)
                .ok_or_else(|| delivery_state_error(&inner, token))?;
            if delivery.token != *token {
                return Err(ControlError::UnknownDelivery(token.delivery_id).into());
            }
        }
        validate(
            replacement,
            token.boundary_owner.as_str(),
            token.result_contract.as_str(),
        )
        .map_err(TypedRecoveryError::Validation)?;

        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let delivery = inner
            .active_deliveries
            .remove(&token.delivery_id)
            .ok_or_else(|| delivery_state_error(&inner, token))?;
        if delivery.token != *token {
            inner
                .active_deliveries
                .insert(delivery.token.delivery_id, delivery);
            return Err(ControlError::UnknownDelivery(token.delivery_id).into());
        }
        inner.delivery_by_scope.remove(&token.affected_scope);
        let settlement = ControlFailureSettlement {
            failure: delivery.failure,
            kind: ControlFailureSettlementKind::Recovered,
            boundary_scope: Some(token.boundary_scope),
            result_contract: Some(token.result_contract.clone()),
            unwind: delivery.unwind,
        };
        inner.scope_causes.remove(&token.affected_scope);
        inner
            .settlements
            .insert(token.affected_scope, settlement.clone());
        Ok(settlement)
    }

    pub fn failure_settlement(
        &self,
        affected_scope: ExecutionScopeId,
    ) -> Option<ControlFailureSettlement> {
        self.inner
            .lock()
            .expect("poisoned operation-control mutex")
            .settlements
            .get(&affected_scope)
            .cloned()
    }

    fn validate_delivery_operation(
        &self,
        token: &FailureDeliveryToken,
    ) -> Result<(), ControlError> {
        if token.operation_id != self.operation_id {
            return Err(ControlError::ForeignDelivery(token.operation_id));
        }
        Ok(())
    }

    fn check_scope_at(&self, scope: ExecutionScopeId, now: Instant) -> Result<(), ControlError> {
        if self.abort_signal.is_aborted() {
            let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
            let cause = ControlCause::HostCancellation {
                reason: self.abort_signal.reason(),
            };
            let source_map = self.abort_signal.source_map();
            inner
                .scope_causes
                .entry(ROOT_EXECUTION_SCOPE_ID)
                .or_insert_with(|| (cause.clone(), source_map.clone()));
            mark_subtree_unwinding(&mut inner.tree, ROOT_EXECUTION_SCOPE_ID);
            return Err(ControlError::Triggered(ControlFailure {
                operation_id: self.operation_id,
                affected_scope: ROOT_EXECUTION_SCOPE_ID,
                cause,
                source_map,
            }));
        }
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let node = inner
            .tree
            .scope(scope)
            .ok_or(ControlError::UnknownScope(scope))?;
        if node.state == ExecutionScopeState::Completed {
            return Err(ControlError::ScopeCompleted(scope));
        }
        let ancestors = known_ancestors(&inner.tree, scope)?;
        if let Some(failure) = failure_for_ancestors(self.operation_id, &inner, &ancestors) {
            return Err(ControlError::Triggered(failure));
        }
        if let Some(failure) = deadline_failure(self.operation_id, &inner, &ancestors, now) {
            let selected = failure.affected_scope;
            inner.scope_causes.insert(
                selected,
                (failure.cause.clone(), failure.source_map.clone()),
            );
            mark_subtree_unwinding(&mut inner.tree, selected);
            return Err(ControlError::Triggered(failure));
        }
        Ok(())
    }

    pub fn enter_frame(
        &self,
        task: TaskId,
        scope: ExecutionScopeId,
        source_map: Option<SourceMapStack>,
    ) -> Result<LogicalStackGuard, ControlError> {
        self.check_scope(scope)?;
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let owner = inner
            .tree
            .task(task)
            .ok_or(ControlError::UnknownTask(task))?
            .owner;
        let ancestors = known_ancestors(&inner.tree, scope)?;
        if let Some(failure) = failure_for_ancestors(self.operation_id, &inner, &ancestors) {
            return Err(ControlError::Triggered(failure));
        }
        if !ancestors.contains(&owner) {
            return Err(ControlError::TaskScopeMismatch {
                task,
                owner,
                requested_scope: scope,
            });
        }

        for ancestor in &ancestors {
            let observed = inner
                .stack_depths
                .get(&(task, *ancestor))
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            let limit = inner
                .tree
                .scope(*ancestor)
                .expect("known ancestor")
                .effective_policy
                .stack_depth;
            if observed > limit {
                let failure = ControlFailure {
                    operation_id: self.operation_id,
                    affected_scope: *ancestor,
                    cause: ControlCause::StackDepthExceeded { observed, limit },
                    source_map,
                };
                inner.scope_causes.insert(
                    *ancestor,
                    (failure.cause.clone(), failure.source_map.clone()),
                );
                mark_subtree_unwinding(&mut inner.tree, *ancestor);
                return Err(ControlError::Triggered(failure));
            }
        }
        for ancestor in &ancestors {
            *inner.stack_depths.entry((task, *ancestor)).or_default() += 1;
        }
        Ok(LogicalStackGuard {
            inner: Arc::clone(&self.inner),
            task,
            ancestors,
            released: false,
        })
    }

    pub fn charge_memory(
        &self,
        scope: ExecutionScopeId,
        bytes: u64,
        source_map: Option<SourceMapStack>,
    ) -> Result<MemoryPermit, ControlError> {
        self.check_scope(scope)?;
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
        let ancestors = known_ancestors(&inner.tree, scope)?;
        if let Some(failure) = failure_for_ancestors(self.operation_id, &inner, &ancestors) {
            return Err(ControlError::Triggered(failure));
        }
        let permit_id = MemoryPermitId::from_raw(inner.next_memory_permit_id);
        let next_memory_permit_id = inner
            .next_memory_permit_id
            .checked_add(1)
            .ok_or(ControlError::IdentityExhausted("memoryPermitId"))?;
        for ancestor in &ancestors {
            let charged = inner
                .accounting
                .get(ancestor)
                .expect("known scope has accounting")
                .memory_charged;
            let limit = inner
                .tree
                .scope(*ancestor)
                .expect("known ancestor")
                .effective_policy
                .memory_bytes;
            if charged.checked_add(bytes).is_none_or(|total| total > limit) {
                let failure = ControlFailure {
                    operation_id: self.operation_id,
                    affected_scope: *ancestor,
                    cause: ControlCause::MemoryExceeded {
                        requested: bytes,
                        charged,
                        limit,
                    },
                    source_map,
                };
                inner.scope_causes.insert(
                    *ancestor,
                    (failure.cause.clone(), failure.source_map.clone()),
                );
                mark_subtree_unwinding(&mut inner.tree, *ancestor);
                return Err(ControlError::Triggered(failure));
            }
        }
        for ancestor in &ancestors {
            inner
                .accounting
                .get_mut(ancestor)
                .expect("known scope has accounting")
                .memory_charged += bytes;
        }
        inner.next_memory_permit_id = next_memory_permit_id;
        inner.memory_permits.insert(
            permit_id,
            MemoryCharge {
                owner: scope,
                ancestors: ancestors.clone(),
                bytes,
            },
        );
        Ok(MemoryPermit {
            inner: Arc::clone(&self.inner),
            permit_id,
            owner: scope,
            bytes,
            released: false,
        })
    }

    pub fn memory_charged(&self, scope: ExecutionScopeId) -> Result<u64, ControlError> {
        let inner = self.inner.lock().expect("poisoned operation-control mutex");
        inner
            .accounting
            .get(&scope)
            .map(|accounting| accounting.memory_charged)
            .ok_or(ControlError::UnknownScope(scope))
    }

    pub fn logical_stack_depth(
        &self,
        task: TaskId,
        scope: ExecutionScopeId,
    ) -> Result<u32, ControlError> {
        let inner = self.inner.lock().expect("poisoned operation-control mutex");
        if inner.tree.task(task).is_none() {
            return Err(ControlError::UnknownTask(task));
        }
        if inner.tree.scope(scope).is_none() {
            return Err(ControlError::UnknownScope(scope));
        }
        Ok(inner.stack_depths.get(&(task, scope)).copied().unwrap_or(0))
    }
}

#[derive(Debug)]
pub struct LogicalStackGuard {
    inner: Arc<Mutex<ControlInner>>,
    task: TaskId,
    ancestors: Vec<ExecutionScopeId>,
    released: bool,
}

impl LogicalStackGuard {
    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if self.released {
            return;
        }
        if let Ok(mut inner) = self.inner.lock() {
            for ancestor in &self.ancestors {
                let key = (self.task, *ancestor);
                if let Some(depth) = inner.stack_depths.get_mut(&key) {
                    *depth = depth.saturating_sub(1);
                    if *depth == 0 {
                        inner.stack_depths.remove(&key);
                    }
                }
            }
        }
        self.released = true;
    }
}

impl Drop for LogicalStackGuard {
    fn drop(&mut self) {
        self.release_inner();
    }
}

#[derive(Debug)]
pub struct MemoryPermit {
    inner: Arc<Mutex<ControlInner>>,
    permit_id: MemoryPermitId,
    owner: ExecutionScopeId,
    bytes: u64,
    released: bool,
}

impl MemoryPermit {
    pub fn owner(&self) -> ExecutionScopeId {
        self.owner
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if self.released {
            return;
        }
        if let Ok(mut inner) = self.inner.lock() {
            release_memory_permit(&mut inner, self.permit_id);
        }
        self.released = true;
    }
}

impl Drop for MemoryPermit {
    fn drop(&mut self) {
        self.release_inner();
    }
}

#[derive(Debug)]
pub struct ScopeCleanupGuard {
    inner: Arc<Mutex<ControlInner>>,
    action_id: CleanupActionId,
    released: bool,
}

impl ScopeCleanupGuard {
    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if self.released {
            return;
        }
        let action = self.inner.lock().ok().and_then(|mut inner| {
            inner
                .cleanup_actions
                .remove(&self.action_id)
                .and_then(|mut cleanup| cleanup.action.take())
        });
        if let Some(action) = action {
            let _ = catch_unwind(AssertUnwindSafe(action));
        }
        self.released = true;
    }
}

impl Drop for ScopeCleanupGuard {
    fn drop(&mut self) {
        self.release_inner();
    }
}

fn release_memory_permit(inner: &mut ControlInner, permit_id: MemoryPermitId) {
    let Some(charge) = inner.memory_permits.remove(&permit_id) else {
        return;
    };
    for ancestor in charge.ancestors {
        if let Some(accounting) = inner.accounting.get_mut(&ancestor) {
            accounting.memory_charged = accounting.memory_charged.saturating_sub(charge.bytes);
        }
    }
}

fn delivery_state_error(inner: &ControlInner, token: &FailureDeliveryToken) -> ControlError {
    if inner.settlements.contains_key(&token.affected_scope) {
        ControlError::DeliverySettled(token.delivery_id)
    } else {
        ControlError::UnknownDelivery(token.delivery_id)
    }
}

fn deliver_or_escalate_locked(
    operation_id: OperationId,
    inner: &mut ControlInner,
    failure: ControlFailure,
    unwind: ControlUnwindSummary,
    after_boundary: Option<ExecutionScopeId>,
) -> Result<FailureDeliveryOutcome, ControlError> {
    let mut cursor = after_boundary
        .and_then(|scope| inner.tree.scope(scope).and_then(|node| node.parent))
        .or_else(|| {
            (after_boundary.is_none())
                .then(|| {
                    inner
                        .tree
                        .scope(failure.affected_scope)
                        .and_then(|node| node.parent)
                })
                .flatten()
        });
    while let Some(scope) = cursor {
        let node = inner
            .tree
            .scope(scope)
            .expect("boundary cursor references a known scope");
        let parent = node.parent;
        let boundary = node.error_boundary.clone();
        if let Some(boundary) = boundary {
            if boundary.accepts(failure.cause.kind()) {
                let delivery_id = FailureDeliveryId::from_raw(inner.next_delivery_id);
                inner.next_delivery_id = inner
                    .next_delivery_id
                    .checked_add(1)
                    .ok_or(ControlError::IdentityExhausted("failureDeliveryId"))?;
                let token = FailureDeliveryToken {
                    operation_id,
                    delivery_id,
                    affected_scope: failure.affected_scope,
                    boundary_scope: scope,
                    cause_kind: failure.cause.kind(),
                    boundary_owner: boundary.owner,
                    result_contract: boundary.result_contract,
                };
                inner.active_deliveries.insert(
                    delivery_id,
                    ActiveFailureDelivery {
                        token: token.clone(),
                        failure,
                        unwind,
                    },
                );
                inner
                    .delivery_by_scope
                    .insert(token.affected_scope, delivery_id);
                return Ok(FailureDeliveryOutcome::Deliver(token));
            }
        }
        cursor = parent;
    }

    inner.scope_causes.insert(
        ROOT_EXECUTION_SCOPE_ID,
        (failure.cause.clone(), failure.source_map.clone()),
    );
    mark_subtree_unwinding(&mut inner.tree, ROOT_EXECUTION_SCOPE_ID);
    let settlement = ControlFailureSettlement {
        failure,
        kind: ControlFailureSettlementKind::EscalatedToRoot,
        boundary_scope: None,
        result_contract: None,
        unwind,
    };
    inner
        .settlements
        .insert(settlement.failure.affected_scope, settlement.clone());
    inner
        .settlements
        .entry(ROOT_EXECUTION_SCOPE_ID)
        .or_insert_with(|| settlement.clone());
    Ok(FailureDeliveryOutcome::Settled(settlement))
}

fn next_operation_id() -> OperationId {
    OperationId(NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed))
}

fn validate_registration(registration: &ExecutionScopeRegistration) -> Result<(), ControlError> {
    let label_len = registration.label.len();
    if label_len == 0
        || label_len > MAX_EXECUTION_SCOPE_LABEL_BYTES
        || registration.label.chars().any(char::is_control)
    {
        return Err(ControlError::InvalidLabel);
    }
    if let Some(location) = &registration.source_location {
        if location.source_uri.is_empty()
            || location.source_uri.len() > MAX_SOURCE_URI_BYTES
            || location.source_uri.chars().any(char::is_control)
            || location.line == 0
            || location.column == Some(0)
            || location.end_line == Some(0)
            || location.end_column == Some(0)
            || location
                .byte_range
                .is_some_and(|range| range.start.checked_add(u64::from(range.len)).is_none())
            || location
                .end_line
                .is_some_and(|end_line| end_line < location.line)
            || location.end_line == Some(location.line)
                && matches!(
                    (location.column, location.end_column),
                    (Some(column), Some(end_column)) if end_column < column
                )
        {
            return Err(ControlError::InvalidSourceLocation);
        }
    }
    for (kind, identity) in &registration.semantic_identities {
        if identity.is_empty()
            || identity.len() > MAX_SCOPE_IDENTITY_BYTES
            || identity.chars().any(char::is_control)
        {
            return Err(ControlError::InvalidSemanticIdentity(*kind));
        }
    }
    if registration
        .error_boundary
        .as_ref()
        .is_some_and(|boundary| {
            boundary.owner.is_empty()
                || boundary.owner.len() > MAX_BOUNDARY_OWNER_BYTES
                || boundary.owner.chars().any(char::is_control)
                || boundary.result_contract.is_empty()
                || boundary.result_contract.len() > MAX_RESULT_CONTRACT_BYTES
                || boundary.result_contract.chars().any(char::is_control)
                || matches!(
                    &boundary.policy,
                    ControlBoundaryPolicy::Recover { accepted_causes }
                        if accepted_causes.is_empty()
                )
        })
    {
        return Err(ControlError::InvalidBoundary);
    }
    validate_policy(registration.effective_policy)
}

fn validate_reason(reason: Option<&str>) -> Result<(), ControlError> {
    if reason.is_some_and(|reason| {
        reason.len() > MAX_CONTROL_REASON_BYTES || reason.chars().any(char::is_control)
    }) {
        return Err(ControlError::InvalidReason);
    }
    Ok(())
}

fn validate_policy(policy: ScopePolicy) -> Result<(), ControlError> {
    policy
        .validate()
        .map_err(|error| ControlError::InvalidPolicy(error.to_string()))
}

fn check_constrain_only(
    scope: ExecutionScopeId,
    parent: ScopePolicy,
    child: ScopePolicy,
) -> Result<(), ControlError> {
    fn deny(
        scope: ExecutionScopeId,
        cap: ResourceCap,
        parent_value: u64,
        attempted_value: u64,
    ) -> ControlError {
        ControlError::CapRelaxationDenied {
            scope,
            cap,
            parent_value,
            attempted_value,
        }
    }

    for (cap, parent_value, child_value) in [
        (
            ResourceCap::CpuWorkers,
            u64::from(parent.cpu_workers),
            u64::from(child.cpu_workers),
        ),
        (
            ResourceCap::QueueSize,
            u64::from(parent.queue_size),
            u64::from(child.queue_size),
        ),
        (
            ResourceCap::IoStreams,
            u64::from(parent.io_streams),
            u64::from(child.io_streams),
        ),
        (
            ResourceCap::MemoryBytes,
            parent.memory_bytes,
            child.memory_bytes,
        ),
        (
            ResourceCap::StackDepth,
            u64::from(parent.stack_depth),
            u64::from(child.stack_depth),
        ),
    ] {
        if child_value > parent_value {
            return Err(deny(scope, cap, parent_value, child_value));
        }
    }
    match (parent.timeout_ms, child.timeout_ms) {
        (Some(parent), Some(child)) if child > parent => {
            Err(deny(scope, ResourceCap::TimeoutMs, parent, child))
        }
        (Some(parent), None) => Err(deny(scope, ResourceCap::TimeoutMs, parent, u64::MAX)),
        _ => Ok(()),
    }
}

fn known_ancestors(
    tree: &ExecutionScopeTree,
    scope: ExecutionScopeId,
) -> Result<Vec<ExecutionScopeId>, ControlError> {
    if tree.scope(scope).is_none() {
        return Err(ControlError::UnknownScope(scope));
    }
    Ok(tree.ancestors(scope))
}

fn mark_subtree_unwinding(tree: &mut ExecutionScopeTree, scope: ExecutionScopeId) {
    for descendant in tree.descendants(scope) {
        if let Some(node) = tree.scopes.get_mut(&descendant) {
            if node.state != ExecutionScopeState::Completed {
                node.state = ExecutionScopeState::Unwinding;
            }
        }
    }
}

fn valid_scope_transition(from: ExecutionScopeState, to: ExecutionScopeState) -> bool {
    use ExecutionScopeState::{Completed, Parked, Queued, Running, Unwinding, Waiting};
    from == to
        || matches!(
            (from, to),
            (Queued, Running | Waiting | Parked | Unwinding | Completed)
                | (Running, Waiting | Parked | Unwinding | Completed)
                | (Waiting, Running | Parked | Unwinding | Completed)
                | (Parked, Running | Unwinding | Completed)
                | (Unwinding, Completed)
        )
}

fn failure_for_ancestors(
    operation_id: OperationId,
    inner: &ControlInner,
    ancestors: &[ExecutionScopeId],
) -> Option<ControlFailure> {
    ancestors.iter().find_map(|scope| {
        inner
            .scope_causes
            .get(scope)
            .map(|(cause, source_map)| ControlFailure {
                operation_id,
                affected_scope: *scope,
                cause: cause.clone(),
                source_map: source_map.clone(),
            })
    })
}

fn deadline_failure(
    operation_id: OperationId,
    inner: &ControlInner,
    ancestors: &[ExecutionScopeId],
    now: Instant,
) -> Option<ControlFailure> {
    let mut expired = Vec::new();
    for scope in ancestors {
        let Some(node) = inner.tree.scope(*scope) else {
            continue;
        };
        let limit_ms = if node.kind == ExecutionScopeKind::Plugin {
            node.effective_policy.effective_plugin_timeout_ms()
        } else {
            node.effective_policy.timeout_ms
        };
        let Some(limit_ms) = limit_ms else {
            continue;
        };
        let Some(accounting) = inner.accounting.get(scope) else {
            continue;
        };
        let Some(activated_at) = accounting.activated_at else {
            continue;
        };
        let elapsed = now
            .saturating_duration_since(activated_at)
            .saturating_sub(accounting.excluded_time);
        let limit = Duration::from_millis(limit_ms);
        if elapsed >= limit {
            expired.push((*scope, activated_at + limit, elapsed, limit_ms));
        }
    }
    expired.sort_by_key(|(_, deadline, _, _)| *deadline);
    expired
        .into_iter()
        .next()
        .map(|(scope, _, elapsed, limit_ms)| ControlFailure {
            operation_id,
            affected_scope: scope,
            cause: ControlCause::TimeoutExceeded {
                active_elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
                limit_ms,
            },
            source_map: None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(memory: u64, stack: u32, timeout_ms: Option<u64>) -> ScopePolicy {
        ScopePolicy::host_root()
            .with_memory_bytes(memory)
            .with_stack_depth(stack)
            .with_timeout_ms(timeout_ms)
    }

    fn child(
        control: &OperationControl,
        parent: ExecutionScopeId,
        label: &str,
        policy: ScopePolicy,
    ) -> ExecutionScopeId {
        control
            .register_scope(
                parent,
                ExecutionScopeRegistration::inherited(ExecutionScopeKind::Transform, label, policy),
            )
            .unwrap()
    }

    fn boundary_child(
        control: &OperationControl,
        parent: ExecutionScopeId,
        label: &str,
        policy: ScopePolicy,
        boundary: ControlErrorBoundary,
    ) -> ExecutionScopeId {
        control
            .register_scope(
                parent,
                ExecutionScopeRegistration::inherited(ExecutionScopeKind::Transform, label, policy)
                    .with_error_boundary(boundary),
            )
            .unwrap()
    }

    #[test]
    fn identities_and_semantic_scope_mappings_are_stable_in_snapshots() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(41),
            AbortSignal::new(),
            policy(1_024, 8, None),
        )
        .unwrap();
        let mut registration = ExecutionScopeRegistration::inherited(
            ExecutionScopeKind::Query,
            "query",
            policy(512, 4, None),
        );
        registration
            .semantic_identities
            .insert(ScopeIdentityKind::Scheduler, "7".to_owned());
        registration
            .semantic_identities
            .insert(ScopeIdentityKind::RunConfig, "scope:input:0".to_owned());
        let scope = control
            .register_scope(ROOT_EXECUTION_SCOPE_ID, registration)
            .unwrap();
        let task = control.register_task(scope).unwrap();

        let snapshot = control.scope_tree();
        assert_eq!(snapshot.operation_id(), OperationId::from_raw(41));
        assert_eq!(
            snapshot.scope(scope).unwrap().parent,
            Some(ROOT_EXECUTION_SCOPE_ID)
        );
        assert_eq!(
            snapshot.scope(scope).unwrap().semantic_identities[&ScopeIdentityKind::Scheduler],
            "7"
        );
        assert_eq!(snapshot.task(task).unwrap().owner, scope);
    }

    #[test]
    fn control_causes_have_stable_typed_wire_fields_and_terminal_classes() {
        let timeout = ControlCause::TimeoutExceeded {
            active_elapsed_ms: 11,
            limit_ms: 10,
        };
        let value = serde_json::to_value(&timeout).unwrap();
        assert_eq!(value["kind"], "timeout-exceeded");
        assert_eq!(value["activeElapsedMs"], 11);
        assert_eq!(value["limitMs"], 10);
        assert_eq!(timeout.terminal_class(), ControlTerminalClass::Failed);
        assert_eq!(
            ControlCause::HostCancellation { reason: None }.terminal_class(),
            ControlTerminalClass::Cancelled
        );
    }

    #[test]
    fn child_cannot_relax_stack_memory_or_timeout_limits() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(1),
            AbortSignal::new(),
            policy(1_024, 8, Some(50)),
        )
        .unwrap();
        for (candidate, cap) in [
            (policy(2_048, 8, Some(50)), ResourceCap::MemoryBytes),
            (policy(1_024, 9, Some(50)), ResourceCap::StackDepth),
            (policy(1_024, 8, Some(51)), ResourceCap::TimeoutMs),
            (policy(1_024, 8, None), ResourceCap::TimeoutMs),
        ] {
            let error = control
                .register_scope(
                    ROOT_EXECUTION_SCOPE_ID,
                    ExecutionScopeRegistration::inherited(
                        ExecutionScopeKind::Parse,
                        "child",
                        candidate,
                    ),
                )
                .unwrap_err();
            assert!(matches!(
                error,
                ControlError::CapRelaxationDenied { cap: actual, .. } if actual == cap
            ));
        }
    }

    #[test]
    fn abort_signal_is_a_root_cancellation_facade_and_first_reason_wins() {
        let signal = AbortSignal::new();
        let control = OperationControl::new(signal.clone());
        assert_eq!(
            control
                .cancel_root(Some("host shutdown".to_owned()), None)
                .unwrap(),
            ControlRequestOutcome::Accepted
        );
        assert!(signal.is_aborted());
        signal.abort();
        let ControlError::Triggered(failure) =
            control.check_scope(ROOT_EXECUTION_SCOPE_ID).unwrap_err()
        else {
            panic!("root cancellation must be typed");
        };
        assert_eq!(failure.terminal_class(), ControlTerminalClass::Cancelled);
        assert!(matches!(
            failure.cause,
            ControlCause::HostCancellation { reason: Some(reason) } if reason == "host shutdown"
        ));
    }

    #[test]
    fn safe_point_poller_observes_cancellation_at_the_fixed_work_boundary() {
        let signal = AbortSignal::new();
        let control = OperationControl::new(signal.clone());
        let mut poller = SafePointPoller::with_work_interval(
            control,
            ROOT_EXECUTION_SCOPE_ID,
            DEFAULT_SAFE_POINT_WORK_INTERVAL,
        );
        poller.force().unwrap();
        signal.abort();

        poller.poll(DEFAULT_SAFE_POINT_WORK_INTERVAL - 1).unwrap();
        let ControlError::Triggered(failure) = poller.poll_one().unwrap_err() else {
            panic!("the fixed work boundary must observe cancellation");
        };
        assert_eq!(failure.terminal_class(), ControlTerminalClass::Cancelled);
    }

    #[test]
    fn safe_point_poller_checks_scoped_causes_and_deadlines() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(43),
            AbortSignal::new(),
            policy(4_096, 8, None),
        )
        .unwrap();
        let selected = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "selected",
            policy(2_048, 8, None),
        );
        let mut selected_poller = SafePointPoller::with_work_interval(control.clone(), selected, 1);
        control.cancel_scope(selected, None, None).unwrap();
        assert!(matches!(
            selected_poller.poll_one(),
            Err(ControlError::Triggered(ControlFailure {
                affected_scope,
                cause: ControlCause::HostCancellation { .. },
                ..
            })) if affected_scope == selected
        ));

        let deadline_control = OperationControl::with_policy(
            OperationId::from_raw(44),
            AbortSignal::new(),
            policy(4_096, 8, Some(1)),
        )
        .unwrap();
        let mut deadline_poller =
            SafePointPoller::with_work_interval(deadline_control, ROOT_EXECUTION_SCOPE_ID, 1);
        std::thread::sleep(Duration::from_millis(2));
        assert!(matches!(
            deadline_poller.poll_one(),
            Err(ControlError::Triggered(ControlFailure {
                cause: ControlCause::TimeoutExceeded { .. },
                ..
            }))
        ));
    }

    #[test]
    fn scoped_cancellation_selects_descendants_without_touching_siblings() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(2),
            AbortSignal::new(),
            policy(4_096, 8, None),
        )
        .unwrap();
        let selected = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "selected",
            policy(2_048, 8, None),
        );
        let descendant = child(&control, selected, "descendant", policy(1_024, 8, None));
        let sibling = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "sibling",
            policy(2_048, 8, None),
        );

        control
            .cancel_scope(selected, Some("superseded branch".to_owned()), None)
            .unwrap();
        for scope in [selected, descendant] {
            let ControlError::Triggered(failure) = control.check_scope(scope).unwrap_err() else {
                panic!("selected subtree must observe cancellation");
            };
            assert_eq!(failure.affected_scope, selected);
        }
        control.check_scope(sibling).unwrap();
    }

    #[test]
    fn completed_or_unwinding_scopes_cannot_return_to_running() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(7),
            AbortSignal::new(),
            policy(4_096, 8, None),
        )
        .unwrap();
        let scope = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "scope",
            policy(2_048, 8, None),
        );
        control
            .set_scope_state(scope, ExecutionScopeState::Unwinding)
            .unwrap();
        assert!(matches!(
            control.register_task(scope),
            Err(ControlError::ScopeUnwinding(actual)) if actual == scope
        ));
        assert!(matches!(
            control.register_scope(
                scope,
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Transform,
                    "late-child",
                    policy(1_024, 8, None),
                ),
            ),
            Err(ControlError::ScopeUnwinding(actual)) if actual == scope
        ));
        assert!(matches!(
            control
                .set_scope_state(scope, ExecutionScopeState::Running)
                .unwrap_err(),
            ControlError::InvalidScopeTransition { .. }
        ));
        control.complete_scope(scope).unwrap();
        assert!(matches!(
            control
                .set_scope_state(scope, ExecutionScopeState::Running)
                .unwrap_err(),
            ControlError::ScopeCompleted(actual) if actual == scope
        ));
    }

    #[test]
    fn logical_stack_is_per_task_and_releases_on_unwind() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(3),
            AbortSignal::new(),
            policy(4_096, 2, None),
        )
        .unwrap();
        let task_a = control.register_task(ROOT_EXECUTION_SCOPE_ID).unwrap();
        let task_b = control.register_task(ROOT_EXECUTION_SCOPE_ID).unwrap();
        let guard_a = control
            .enter_frame(task_a, ROOT_EXECUTION_SCOPE_ID, None)
            .unwrap();
        let _guard_b = control
            .enter_frame(task_b, ROOT_EXECUTION_SCOPE_ID, None)
            .unwrap();
        let guard_a2 = control
            .enter_frame(task_a, ROOT_EXECUTION_SCOPE_ID, None)
            .unwrap();
        let ControlError::Triggered(failure) = control
            .enter_frame(task_a, ROOT_EXECUTION_SCOPE_ID, None)
            .unwrap_err()
        else {
            panic!("third logical frame must fail");
        };
        assert!(matches!(
            failure.cause,
            ControlCause::StackDepthExceeded {
                observed: 3,
                limit: 2
            }
        ));
        drop((guard_a2, guard_a));
    }

    #[test]
    fn memory_charge_is_atomic_across_ancestors_and_releases() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(4),
            AbortSignal::new(),
            policy(100, 8, None),
        )
        .unwrap();
        let child = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "child",
            policy(60, 8, None),
        );
        let permit = control.charge_memory(child, 50, None).unwrap();
        assert_eq!(control.memory_charged(child).unwrap(), 50);
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 50);
        let ControlError::Triggered(failure) = control.charge_memory(child, 11, None).unwrap_err()
        else {
            panic!("child memory cap must fail before charging");
        };
        assert_eq!(failure.affected_scope, child);
        assert_eq!(control.memory_charged(child).unwrap(), 50);
        drop(permit);
        assert_eq!(control.memory_charged(child).unwrap(), 0);
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }

    #[test]
    fn ancestor_memory_failure_rolls_back_the_entire_child_charge() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(6),
            AbortSignal::new(),
            policy(100, 8, None),
        )
        .unwrap();
        let selected = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "selected",
            policy(80, 8, None),
        );
        let sibling = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "sibling",
            policy(80, 8, None),
        );
        let sibling_permit = control.charge_memory(sibling, 50, None).unwrap();
        let ControlError::Triggered(failure) =
            control.charge_memory(selected, 51, None).unwrap_err()
        else {
            panic!("the root memory cap must select the root subtree");
        };
        assert_eq!(failure.affected_scope, ROOT_EXECUTION_SCOPE_ID);
        assert_eq!(control.memory_charged(selected).unwrap(), 0);
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 50);
        drop(sibling_permit);
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }

    #[test]
    fn earliest_expired_ancestor_deadline_selects_its_scope() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(5),
            AbortSignal::new(),
            policy(4_096, 8, Some(10)),
        )
        .unwrap();
        let child = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "child",
            policy(2_048, 8, Some(5)),
        );
        control
            .set_scope_state(child, ExecutionScopeState::Running)
            .unwrap();
        let now = {
            let inner = control.inner.lock().unwrap();
            inner.accounting[&child].activated_at.unwrap() + Duration::from_millis(6)
        };
        let ControlError::Triggered(failure) = control.check_scope_at(child, now).unwrap_err()
        else {
            panic!("child deadline must expire");
        };
        assert_eq!(failure.affected_scope, child);
        assert!(matches!(
            failure.cause,
            ControlCause::TimeoutExceeded { limit_ms: 5, .. }
        ));
    }

    #[test]
    fn plugin_budget_becomes_a_plugin_scope_deadline() {
        let root_policy = policy(4_096, 8, None);
        let control = OperationControl::with_policy(
            OperationId::from_raw(8),
            AbortSignal::new(),
            root_policy,
        )
        .unwrap();
        let mut plugin_policy = root_policy;
        plugin_policy.plugin_time_budget_ms = Some(3);
        let plugin = control
            .register_scope(
                ROOT_EXECUTION_SCOPE_ID,
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Plugin,
                    "plugin",
                    plugin_policy,
                ),
            )
            .unwrap();
        control
            .set_scope_state(plugin, ExecutionScopeState::Running)
            .unwrap();
        let now = {
            let inner = control.inner.lock().unwrap();
            inner.accounting[&plugin].activated_at.unwrap() + Duration::from_millis(4)
        };
        let ControlError::Triggered(failure) = control.check_scope_at(plugin, now).unwrap_err()
        else {
            panic!("plugin budget must register a deadline");
        };
        assert_eq!(failure.affected_scope, plugin);
        assert!(matches!(
            failure.cause,
            ControlCause::TimeoutExceeded { limit_ms: 3, .. }
        ));
    }

    #[test]
    fn scoped_unwind_waits_for_tasks_and_cleans_descendants_before_typed_recovery() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(50),
            AbortSignal::new(),
            policy(100, 8, None),
        )
        .unwrap();
        let boundary = boundary_child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "query-boundary",
            policy(90, 8, None),
            ControlErrorBoundary::recover(
                "cem-ql",
                "cem-ql:stream<integer>",
                [ControlCauseKind::HostCancellation],
            ),
        );
        let selected = child(&control, boundary, "selected", policy(60, 8, None));
        let descendant = child(&control, selected, "descendant", policy(40, 8, None));
        let sibling = child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "sibling",
            policy(80, 8, None),
        );
        for scope in [boundary, selected, descendant, sibling] {
            control
                .set_scope_state(scope, ExecutionScopeState::Running)
                .unwrap();
        }

        let selected_task = control.register_task(selected).unwrap();
        let descendant_task = control.register_task(descendant).unwrap();
        let selected_frame = control.enter_frame(selected_task, selected, None).unwrap();
        let descendant_frame = control
            .enter_frame(descendant_task, descendant, None)
            .unwrap();
        let selected_memory = control.charge_memory(selected, 10, None).unwrap();
        let descendant_memory = control.charge_memory(descendant, 11, None).unwrap();
        let sibling_memory = control.charge_memory(sibling, 7, None).unwrap();

        let cleanup_order = Arc::new(Mutex::new(Vec::new()));
        let mut cleanup_guards = Vec::new();
        for (scope, label) in [
            (selected, "selected-1"),
            (selected, "selected-2"),
            (descendant, "descendant-1"),
            (descendant, "descendant-2"),
        ] {
            let observed = Arc::clone(&cleanup_order);
            let recorded_label = label.to_owned();
            cleanup_guards.push(
                control
                    .register_cleanup(scope, label, move || {
                        observed.lock().unwrap().push(recorded_label);
                    })
                    .unwrap(),
            );
        }

        control.cancel_scope(selected, None, None).unwrap();
        assert!(matches!(
            control.register_task(selected),
            Err(ControlError::Triggered(ControlFailure { affected_scope, .. }))
                if affected_scope == selected
        ));
        assert!(matches!(
            control.register_scope(
                selected,
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Transform,
                    "late-child",
                    policy(30, 8, None),
                ),
            ),
            Err(ControlError::Triggered(ControlFailure { affected_scope, .. }))
                if affected_scope == selected
        ));
        let ControlError::Triggered(failure) = control.check_scope(descendant).unwrap_err() else {
            panic!("selected cancellation must reach its descendant");
        };
        assert!(matches!(
            control.prepare_failure_delivery(&failure).unwrap(),
            FailureDeliveryOutcome::AwaitingCleanup { unfinished_tasks, .. }
                if unfinished_tasks == vec![selected_task, descendant_task]
        ));

        control.complete_task(selected_task).unwrap();
        control.complete_task(descendant_task).unwrap();
        let FailureDeliveryOutcome::Deliver(token) =
            control.prepare_failure_delivery(&failure).unwrap()
        else {
            panic!("completed subtree must deliver to the nearest boundary");
        };
        assert_eq!(token.boundary_scope, boundary);
        assert_eq!(
            *cleanup_order.lock().unwrap(),
            ["descendant-2", "descendant-1", "selected-2", "selected-1"]
        );
        assert_eq!(control.memory_charged(selected).unwrap(), 0);
        assert_eq!(control.memory_charged(descendant).unwrap(), 0);
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 7);
        assert_eq!(
            control
                .logical_stack_depth(selected_task, selected)
                .unwrap(),
            0
        );
        assert_eq!(
            control
                .logical_stack_depth(descendant_task, descendant)
                .unwrap(),
            0
        );
        control.check_scope(sibling).unwrap();

        #[derive(Debug)]
        enum QueryReplacement {
            Text,
            Integer(i64),
        }
        let invalid = control.validate_and_recover(
            &token,
            &QueryReplacement::Text,
            |replacement, owner, contract| {
                (owner == "cem-ql"
                    && contract == "cem-ql:stream<integer>"
                    && matches!(replacement, QueryReplacement::Integer(_)))
                .then_some(())
                .ok_or("replacement is not an integer stream")
            },
        );
        assert_eq!(
            invalid,
            Err(TypedRecoveryError::Validation(
                "replacement is not an integer stream"
            ))
        );
        let replacement = QueryReplacement::Integer(42);
        let settlement = control
            .validate_and_recover(&token, &replacement, |replacement, owner, contract| {
                (owner == "cem-ql"
                    && contract == "cem-ql:stream<integer>"
                    && matches!(replacement, QueryReplacement::Integer(_)))
                .then_some(())
                .ok_or("replacement is not an integer stream")
            })
            .unwrap();
        assert_eq!(settlement.kind, ControlFailureSettlementKind::Recovered);
        assert!(matches!(replacement, QueryReplacement::Integer(42)));
        assert!(matches!(
            control.validate_and_recover(&token, &replacement, |_, _, _| Ok::<_, ()>(())),
            Err(TypedRecoveryError::Control(ControlError::DeliverySettled(id)))
                if id == token.delivery_id
        ));
        control.check_scope(sibling).unwrap();

        drop((selected_frame, descendant_frame));
        drop((selected_memory, descendant_memory));
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 7);
        drop(sibling_memory);
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
        drop(cleanup_guards);
        assert_eq!(cleanup_order.lock().unwrap().len(), 4);
    }

    #[test]
    fn boundary_filter_fail_fast_and_decline_bubble_without_touching_siblings() {
        let root_policy = policy(4_096, 8, None);
        let control = OperationControl::with_policy(
            OperationId::from_raw(51),
            AbortSignal::new(),
            root_policy,
        )
        .unwrap();
        let outer = boundary_child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "outer",
            root_policy,
            ControlErrorBoundary::recover(
                "transform",
                "transform:artifact",
                [ControlCauseKind::HostCancellation],
            ),
        );
        let fail_fast = boundary_child(
            &control,
            outer,
            "fail-fast",
            root_policy,
            ControlErrorBoundary::fail_fast("schema", "schema:node"),
        );
        let inner = boundary_child(
            &control,
            fail_fast,
            "inner",
            root_policy,
            ControlErrorBoundary::recover(
                "template",
                "template:fragment",
                [ControlCauseKind::HostCancellation],
            ),
        );
        let rejecting = boundary_child(
            &control,
            inner,
            "rejecting",
            root_policy,
            ControlErrorBoundary::recover(
                "query",
                "query:value",
                [ControlCauseKind::TimeoutExceeded],
            ),
        );
        let selected = child(&control, rejecting, "selected", root_policy);
        let sibling = child(&control, inner, "sibling", root_policy);
        control.cancel_scope(selected, None, None).unwrap();
        let ControlError::Triggered(failure) = control.check_scope(selected).unwrap_err() else {
            panic!("selected scope must observe cancellation");
        };
        let FailureDeliveryOutcome::Deliver(inner_token) =
            control.prepare_failure_delivery(&failure).unwrap()
        else {
            panic!("cause filtering must select the accepting inner boundary");
        };
        assert_eq!(inner_token.boundary_scope, inner);
        let FailureDeliveryOutcome::Deliver(outer_token) =
            control.decline_failure_delivery(&inner_token).unwrap()
        else {
            panic!("decline must skip fail-fast and reach the outer boundary");
        };
        assert_eq!(outer_token.boundary_scope, outer);
        control.check_scope(sibling).unwrap();
        let settlement = control
            .validate_and_recover(&outer_token, &"artifact", |_, owner, contract| {
                (owner == "transform" && contract == "transform:artifact")
                    .then_some(())
                    .ok_or("wrong transform contract")
            })
            .unwrap();
        assert_eq!(settlement.kind, ControlFailureSettlementKind::Recovered);
        control.check_scope(sibling).unwrap();
    }

    #[test]
    fn resource_failure_uses_the_same_typed_boundary_delivery() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(52),
            AbortSignal::new(),
            policy(100, 8, None),
        )
        .unwrap();
        let boundary = boundary_child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "resource-boundary",
            policy(100, 8, None),
            ControlErrorBoundary::recover(
                "template",
                "template:fragment",
                [ControlCauseKind::MemoryExceeded],
            ),
        );
        let selected = child(&control, boundary, "selected", policy(10, 8, None));
        let ControlError::Triggered(failure) =
            control.charge_memory(selected, 11, None).unwrap_err()
        else {
            panic!("child memory cap must produce a typed control failure");
        };
        let FailureDeliveryOutcome::Deliver(token) =
            control.prepare_failure_delivery(&failure).unwrap()
        else {
            panic!("memory failure must use the common boundary path");
        };
        assert_eq!(token.boundary_scope, boundary);
        let settlement = control
            .validate_and_recover(&token, &String::new(), |_, owner, contract| {
                (owner == "template" && contract == "template:fragment")
                    .then_some(())
                    .ok_or("wrong template contract")
            })
            .unwrap();
        assert_eq!(settlement.kind, ControlFailureSettlementKind::Recovered);
    }

    #[test]
    fn unhandled_scoped_failure_escalates_to_root_once() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(55),
            AbortSignal::new(),
            policy(4_096, 8, None),
        )
        .unwrap();
        let fail_fast = boundary_child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "fail-fast",
            policy(4_096, 8, None),
            ControlErrorBoundary::fail_fast("schema", "schema:node"),
        );
        let rejecting = boundary_child(
            &control,
            fail_fast,
            "rejecting",
            policy(4_096, 8, None),
            ControlErrorBoundary::recover(
                "query",
                "query:value",
                [ControlCauseKind::TimeoutExceeded],
            ),
        );
        let selected = child(&control, rejecting, "selected", policy(4_096, 8, None));
        control.cancel_scope(selected, None, None).unwrap();
        let ControlError::Triggered(failure) = control.check_scope(selected).unwrap_err() else {
            panic!("selected cancellation must be typed");
        };
        let FailureDeliveryOutcome::Settled(settlement) =
            control.prepare_failure_delivery(&failure).unwrap()
        else {
            panic!("unaccepted failure must settle at the root");
        };
        assert_eq!(
            settlement.kind,
            ControlFailureSettlementKind::EscalatedToRoot
        );
        assert_eq!(settlement.failure.affected_scope, selected);

        let ControlError::Triggered(root_failure) =
            control.check_scope(ROOT_EXECUTION_SCOPE_ID).unwrap_err()
        else {
            panic!("root must retain the escalated failure");
        };
        assert_eq!(
            control.prepare_failure_delivery(&root_failure).unwrap(),
            FailureDeliveryOutcome::Settled(settlement)
        );
    }

    #[test]
    fn root_cancellation_and_cleanup_failure_escalate_exactly_once() {
        let control = OperationControl::with_policy(
            OperationId::from_raw(53),
            AbortSignal::new(),
            policy(4_096, 8, None),
        )
        .unwrap();
        let root_boundary_child = boundary_child(
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            "child-boundary",
            policy(4_096, 8, None),
            ControlErrorBoundary::recover(
                "query",
                "query:value",
                [ControlCauseKind::HostCancellation],
            ),
        );
        control.cancel_root(Some("stop".to_owned()), None).unwrap();
        let ControlError::Triggered(root_failure) =
            control.check_scope(root_boundary_child).unwrap_err()
        else {
            panic!("root cancellation must be typed");
        };
        let FailureDeliveryOutcome::Settled(root_settlement) =
            control.prepare_failure_delivery(&root_failure).unwrap()
        else {
            panic!("root cancellation must not enter a child recovery boundary");
        };
        assert_eq!(
            root_settlement.kind,
            ControlFailureSettlementKind::EscalatedToRoot
        );
        assert_eq!(root_settlement.boundary_scope, None);
        assert_eq!(
            control.prepare_failure_delivery(&root_failure).unwrap(),
            FailureDeliveryOutcome::Settled(root_settlement)
        );

        let cleanup_control = OperationControl::with_policy(
            OperationId::from_raw(54),
            AbortSignal::new(),
            policy(4_096, 8, None),
        )
        .unwrap();
        let boundary = boundary_child(
            &cleanup_control,
            ROOT_EXECUTION_SCOPE_ID,
            "boundary",
            policy(4_096, 8, None),
            ControlErrorBoundary::recover(
                "query",
                "query:value",
                [ControlCauseKind::HostCancellation],
            ),
        );
        let selected = child(
            &cleanup_control,
            boundary,
            "selected",
            policy(4_096, 8, None),
        );
        let _cleanup = cleanup_control
            .register_cleanup(selected, "panic", || panic!("cleanup failed"))
            .unwrap();
        cleanup_control.cancel_scope(selected, None, None).unwrap();
        let ControlError::Triggered(failure) = cleanup_control.check_scope(selected).unwrap_err()
        else {
            panic!("selected cancellation must be typed");
        };
        let FailureDeliveryOutcome::Settled(settlement) =
            cleanup_control.prepare_failure_delivery(&failure).unwrap()
        else {
            panic!("cleanup panic must bypass recovery and escalate");
        };
        assert_eq!(
            settlement.kind,
            ControlFailureSettlementKind::EscalatedToRoot
        );
        assert!(matches!(
            settlement.failure.cause,
            ControlCause::InternalFailure { ref diagnostic_code }
                if diagnostic_code == "cem.control.cleanup_failed"
        ));
        assert_eq!(
            cleanup_control.prepare_failure_delivery(&failure).unwrap(),
            FailureDeliveryOutcome::Settled(settlement.clone())
        );
        let ControlError::Triggered(root_cleanup_failure) = cleanup_control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .unwrap_err()
        else {
            panic!("cleanup failure must remain observable at the root");
        };
        assert_eq!(
            cleanup_control
                .prepare_failure_delivery(&root_cleanup_failure)
                .unwrap(),
            FailureDeliveryOutcome::Settled(settlement)
        );
    }
}
