//! Operation-local control identities, execution scopes, cancellation, and
//! hierarchical resource accounting.
//!
//! This is the common semantic layer used by native and WASM hosts. Live
//! control state remains outside serializable run requests.

use std::collections::BTreeMap;
use std::fmt;
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
            effective_policy: parent,
        }
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
    InvalidScopeTransition {
        scope: ExecutionScopeId,
        from: ExecutionScopeState,
        to: ExecutionScopeState,
    },
    InvalidLabel,
    InvalidSourceLocation,
    InvalidSemanticIdentity(ScopeIdentityKind),
    InvalidReason,
    InvalidPolicy(String),
    CapRelaxationDenied {
        scope: ExecutionScopeId,
        cap: ResourceCap,
        parent_value: u64,
        attempted_value: u64,
    },
    IdentityExhausted(&'static str),
    Triggered(ControlFailure),
}

impl ControlError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownScope(_) => "cem.control.unknown_scope",
            Self::UnknownTask(_) => "cem.control.unknown_task",
            Self::TaskScopeMismatch { .. } => "cem.control.task_scope_mismatch",
            Self::ScopeCompleted(_) => "cem.control.scope_completed",
            Self::InvalidScopeTransition { .. } => "cem.control.scope_transition_invalid",
            Self::InvalidLabel => "cem.control.scope_label_invalid",
            Self::InvalidSourceLocation => "cem.control.source_location_invalid",
            Self::InvalidSemanticIdentity(_) => "cem.control.semantic_identity_invalid",
            Self::InvalidReason => "cem.control.reason_invalid",
            Self::InvalidPolicy(_) => "cem.control.policy_invalid",
            Self::CapRelaxationDenied { .. } => "cem.a.cap_relaxation_denied",
            Self::IdentityExhausted(_) => "cem.control.identity_exhausted",
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
            Self::InvalidScopeTransition { scope, from, to } => write!(
                formatter,
                "execution scope {scope} cannot transition from {from:?} to {to:?}"
            ),
            Self::InvalidLabel => formatter.write_str("execution-scope label is empty or too long"),
            Self::InvalidSourceLocation => formatter.write_str("source location is invalid"),
            Self::InvalidSemanticIdentity(kind) => {
                write!(formatter, "semantic identity {kind:?} is empty or too long")
            }
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

#[derive(Debug)]
struct ControlInner {
    tree: ExecutionScopeTree,
    accounting: BTreeMap<ExecutionScopeId, ScopeAccounting>,
    stack_depths: BTreeMap<(TaskId, ExecutionScopeId), u32>,
    scope_causes: BTreeMap<ExecutionScopeId, (ControlCause, Option<SourceMapStack>)>,
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
        let parent_policy = inner
            .tree
            .scope(parent)
            .ok_or(ControlError::UnknownScope(parent))?
            .effective_policy;
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
        inner.scope_causes.remove(&scope);
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

    pub fn cancel_root(
        &self,
        reason: Option<String>,
        source_map: Option<SourceMapStack>,
    ) -> Result<ControlRequestOutcome, ControlError> {
        validate_reason(reason.as_deref())?;
        Ok(
            if self.abort_signal.abort_with_metadata(reason, source_map) {
                ControlRequestOutcome::Accepted
            } else {
                ControlRequestOutcome::AlreadyRequested
            },
        )
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

    fn check_scope_at(&self, scope: ExecutionScopeId, now: Instant) -> Result<(), ControlError> {
        if self.abort_signal.is_aborted() {
            return Err(ControlError::Triggered(ControlFailure {
                operation_id: self.operation_id,
                affected_scope: ROOT_EXECUTION_SCOPE_ID,
                cause: ControlCause::HostCancellation {
                    reason: self.abort_signal.reason(),
                },
                source_map: self.abort_signal.source_map(),
            }));
        }
        let mut inner = self.inner.lock().expect("poisoned operation-control mutex");
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
        Ok(MemoryPermit {
            inner: Arc::clone(&self.inner),
            owner: scope,
            ancestors,
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
    owner: ExecutionScopeId,
    ancestors: Vec<ExecutionScopeId>,
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
            for ancestor in &self.ancestors {
                if let Some(accounting) = inner.accounting.get_mut(ancestor) {
                    accounting.memory_charged =
                        accounting.memory_charged.saturating_sub(self.bytes);
                }
            }
        }
        self.released = true;
    }
}

impl Drop for MemoryPermit {
    fn drop(&mut self) {
        self.release_inner();
    }
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

        poller
            .poll(DEFAULT_SAFE_POINT_WORK_INTERVAL - 1)
            .unwrap();
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
        let mut selected_poller =
            SafePointPoller::with_work_interval(control.clone(), selected, 1);
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
}
