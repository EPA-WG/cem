//! Feature-gated cooperative pause, stepping, and immutable inspection state.
//!
//! Debugger threads are logical [`TaskId`] values. Physical workers remain
//! presentation metadata and never contribute to result or artifact identity.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use web_time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::capability::{
    OperationHostLimits, MAX_DEBUG_BREAKPOINTS, MAX_DEBUG_VALUE_PREVIEW_BYTES,
    MAX_STACK_FRAME_PAGE_SIZE, MAX_SUSPENDED_SNAPSHOT_BYTES, MAX_VARIABLE_PAGE_SIZE,
};
use crate::operation_control::{
    ControlError, ExecutionScope, ExecutionScopeId, ExecutionScopeState, OperationId,
    SourceLocation, TaskId, MAX_CONTROL_REASON_BYTES, MAX_EXECUTION_SCOPE_LABEL_BYTES,
    MAX_SOURCE_URI_BYTES,
};
use crate::source::ByteRange;

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

opaque_id!(StopId);
opaque_id!(BreakpointId);
opaque_id!(SnapshotReferenceId);
opaque_id!(SafePointId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopToken {
    pub operation_id: OperationId,
    pub stop_id: StopId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugBoundedList<T> {
    pub items: Vec<T>,
    pub original_count: u64,
}

impl<T> DebugBoundedList<T> {
    fn from_items(items: Vec<T>, original_count: usize) -> Self {
        Self {
            items,
            original_count: u64::try_from(original_count).unwrap_or(u64::MAX),
        }
    }

    pub fn was_truncated(&self) -> bool {
        self.original_count > self.items.len() as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugPage<T> {
    pub items: Vec<T>,
    pub start: u32,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugSourceSelector {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ExecutionScopeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DebugSafePointKind {
    Visible,
    Hidden,
    ScopeEnter,
    ScopeExit,
}

impl DebugSafePointKind {
    fn visible(self) -> bool {
        self != Self::Hidden
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableSafePoint {
    pub id: SafePointId,
    pub scope: ExecutionScopeId,
    pub kind: DebugSafePointKind,
    pub phase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PauseTriggerKind {
    NextSafePoint,
    ScopeEnter,
    ScopeExit,
    SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseSpec {
    pub trigger: PauseTriggerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ExecutionScopeId>,
    /// Logical task preferred to trigger a manual all-stop. Other tasks may
    /// trigger only while this task is not actively running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_task: Option<TaskId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<DebugSourceSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
    #[serde(default)]
    pub persistent: bool,
}

impl PauseSpec {
    pub fn next_safe_point(scope: Option<ExecutionScopeId>) -> Self {
        Self {
            trigger: PauseTriggerKind::NextSafePoint,
            scope,
            preferred_task: None,
            source: None,
            condition: None,
            hit_condition: None,
            persistent: false,
        }
    }

    pub fn source(source: DebugSourceSelector) -> Self {
        Self {
            trigger: PauseTriggerKind::SourceLocation,
            scope: None,
            preferred_task: None,
            source: Some(source),
            condition: None,
            hit_condition: None,
            persistent: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepMode {
    Next,
    StepIn,
    StepOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepRequest {
    pub stop: StopToken,
    pub task: TaskId,
    pub mode: StepMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StopReason {
    Breakpoint,
    Pause,
    Step,
    ControlFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakpointResolution {
    pub breakpoint_id: BreakpointId,
    pub verified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<ExecutableSafePoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseRequestedEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breakpoint_id: Option<BreakpointId>,
    pub generation: u64,
    pub reason: StopReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggering_task: Option<TaskId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEvent {
    pub stop: StopToken,
    pub reason: StopReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breakpoint_id: Option<BreakpointId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggering_task: Option<TaskId>,
    pub all_threads_stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuedEvent {
    pub stop: StopToken,
    pub all_threads_continued: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stepping_task: Option<TaskId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugRuntimeEvent {
    BreakpointResolved(BreakpointResolution),
    PauseRequested(PauseRequestedEvent),
    Stopped(StoppedEvent),
    Continued(ContinuedEvent),
}

pub trait DebugRuntimeObserver: Send + Sync {
    fn event(&self, _event: DebugRuntimeEvent) {}
    fn wake_scheduler(&self) {}
    fn breakpoint_removed(&self, _breakpoint: BreakpointId) {}
}

pub trait DebugConditionEvaluator: Send + Sync {
    fn validate(&self, expression: &str) -> Result<(), String>;
    fn evaluate(&self, expression: &str, context: &DebugConditionContext) -> Result<bool, String>;
}

#[derive(Debug, Clone)]
pub struct DebugConditionContext {
    pub task: TaskId,
    pub scope: ExecutionScopeId,
    pub location: Option<SourceLocation>,
    pub frame_names: Vec<String>,
    pub frames: Vec<LogicalFrameCapture>,
}

#[derive(Clone)]
pub struct DebugValueCapture {
    pub identity: Option<u64>,
    pub type_name: String,
    pub preview: String,
    pub original_length: Option<u64>,
    pub named: Vec<DebugVariableCapture>,
    pub indexed: Vec<DebugValueCapture>,
    pub native_value: Option<Arc<dyn Any + Send + Sync>>,
    /// Bounded, read-only host projection captured at the same safe point as
    /// the native value. Debug adapters never invoke user code while stopped.
    pub native_projection: Option<Value>,
}

impl fmt::Debug for DebugValueCapture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DebugValueCapture")
            .field("identity", &self.identity)
            .field("type_name", &self.type_name)
            .field("preview", &self.preview)
            .field("original_length", &self.original_length)
            .field("named", &self.named)
            .field("indexed", &self.indexed)
            .field("native_value", &self.native_value.as_ref().map(|_| "typed"))
            .field("native_projection", &self.native_projection)
            .finish()
    }
}

impl DebugValueCapture {
    pub fn scalar(type_name: impl Into<String>, preview: impl Into<String>) -> Self {
        Self {
            identity: None,
            type_name: type_name.into(),
            preview: preview.into(),
            original_length: None,
            named: Vec::new(),
            indexed: Vec::new(),
            native_value: None,
            native_projection: None,
        }
    }

    pub fn native<T: Any + Send + Sync>(
        type_name: impl Into<String>,
        preview: impl Into<String>,
        value: T,
    ) -> Self {
        Self {
            native_value: Some(Arc::new(value)),
            ..Self::scalar(type_name, preview)
        }
    }

    pub fn projected_native<T: Any + Send + Sync, P: Serialize>(
        type_name: impl Into<String>,
        preview: impl Into<String>,
        value: T,
        projection: P,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            native_value: Some(Arc::new(value)),
            native_projection: Some(serde_json::to_value(projection)?),
            ..Self::scalar(type_name, preview)
        })
    }
}

#[derive(Debug, Clone)]
pub struct DebugVariableCapture {
    pub name: String,
    pub declaration: Option<SourceLocation>,
    pub value: DebugValueCapture,
}

#[derive(Debug, Clone)]
pub struct DebugVariableScopeCapture {
    pub name: String,
    pub expensive: bool,
    pub variables: Vec<DebugVariableCapture>,
}

#[derive(Debug, Clone)]
pub struct LogicalFrameCapture {
    pub name: String,
    pub phase: String,
    pub location: Option<SourceLocation>,
    pub execution_scope: ExecutionScopeId,
    pub variable_scopes: Vec<DebugVariableScopeCapture>,
}

#[derive(Debug, Clone)]
pub struct DebugSafePointCapture {
    pub kind: DebugSafePointKind,
    pub phase: String,
    pub location: Option<SourceLocation>,
    pub frames: Vec<LogicalFrameCapture>,
}

impl DebugSafePointCapture {
    pub fn visible(
        phase: impl Into<String>,
        location: Option<SourceLocation>,
        frames: Vec<LogicalFrameCapture>,
    ) -> Self {
        Self {
            kind: DebugSafePointKind::Visible,
            phase: phase.into(),
            location,
            frames,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DebugTaskState {
    Queued,
    Running,
    ExternalWait,
    Parked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSnapshot {
    pub task: TaskId,
    pub state: DebugTaskState,
    pub owner: ExecutionScopeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_worker: Option<u64>,
    pub captured_frame_count: u32,
    pub original_frame_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionScopeSnapshot {
    pub id: ExecutionScopeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ExecutionScopeId>,
    pub kind: String,
    pub label: String,
    pub state: ExecutionScopeState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalFrameSnapshot {
    pub reference: SnapshotReferenceId,
    pub task: TaskId,
    pub index: u32,
    pub name: String,
    pub phase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
    pub execution_scope: ExecutionScopeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableScopeSnapshot {
    pub name: String,
    pub expensive: bool,
    pub variables_reference: SnapshotReferenceId,
    pub named_variables: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugValueSummary {
    pub type_name: String,
    pub preview: String,
    pub original_length: u64,
    pub preview_truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables_reference: Option<SnapshotReferenceId>,
    pub named_variables: u64,
    pub indexed_variables: u64,
    pub native_value: bool,
    pub opaque: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableSnapshot {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declaration: Option<SourceLocation>,
    pub value: DebugValueSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VariableFilter {
    Named,
    Indexed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspendedSnapshot {
    pub stop: StopToken,
    pub reason: StopReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggering_task: Option<TaskId>,
    pub threads: DebugBoundedList<ThreadSnapshot>,
    pub execution_scopes: DebugBoundedList<ExecutionScopeSnapshot>,
    pub retained_bytes: u64,
    pub retained_byte_limit: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSafePointOutcome {
    Disabled,
    Running,
    Resumed,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugControlError {
    Inactive,
    Completed,
    InvalidPauseSpec,
    InvalidSourceSelector,
    InvalidCondition,
    ConditionEvaluatorUnavailable,
    ConditionRejected(String),
    BreakpointLimit {
        maximum: u32,
    },
    UnknownBreakpoint(BreakpointId),
    BreakpointRemoved(BreakpointId),
    LocationNotExecutable,
    LocationAmbiguous {
        candidates: Vec<ExecutableSafePoint>,
    },
    UnknownTask(TaskId),
    UnknownScope(ExecutionScopeId),
    TaskScopeMismatch,
    NotStopped,
    ForeignStop(OperationId),
    StaleStop(StopId),
    InvalidPageSize {
        requested: u32,
        maximum: u32,
    },
    UnknownSnapshotReference(SnapshotReferenceId),
    NativeValueTypeMismatch,
    IdentityExhausted(&'static str),
    Control(ControlError),
}

impl DebugControlError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Inactive => "cem.debug.inactive",
            Self::Completed => "cem.debug.operation_completed",
            Self::InvalidPauseSpec => "cem.debug.pause_spec_invalid",
            Self::InvalidSourceSelector => "cem.debug.source_selector_invalid",
            Self::InvalidCondition => "cem.debug.condition_invalid",
            Self::ConditionEvaluatorUnavailable => "cem.debug.condition_evaluator_unavailable",
            Self::ConditionRejected(_) => "cem.debug.condition_rejected",
            Self::BreakpointLimit { .. } => "cem.debug.breakpoint_limit",
            Self::UnknownBreakpoint(_) => "cem.debug.breakpoint_unknown",
            Self::BreakpointRemoved(_) => "cem.debug.breakpoint_removed",
            Self::LocationNotExecutable => "cem.debug.location_not_executable",
            Self::LocationAmbiguous { .. } => "cem.debug.location_ambiguous",
            Self::UnknownTask(_) => "cem.debug.task_unknown",
            Self::UnknownScope(_) => "cem.debug.scope_unknown",
            Self::TaskScopeMismatch => "cem.debug.task_scope_mismatch",
            Self::NotStopped => "cem.debug.not_stopped",
            Self::ForeignStop(_) => "cem.debug.stop_foreign_operation",
            Self::StaleStop(_) => "cem.debug.stop_stale",
            Self::InvalidPageSize { .. } => "cem.debug.page_size_invalid",
            Self::UnknownSnapshotReference(_) => "cem.debug.reference_unknown",
            Self::NativeValueTypeMismatch => "cem.debug.native_value_type_mismatch",
            Self::IdentityExhausted(_) => "cem.debug.identity_exhausted",
            Self::Control(error) => error.code(),
        }
    }

    pub(crate) fn from_control(error: ControlError) -> Self {
        Self::Control(error)
    }
}

impl fmt::Display for DebugControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inactive => formatter.write_str("debug control is not active"),
            Self::Completed => formatter.write_str("operation debug control is complete"),
            Self::InvalidPauseSpec => formatter.write_str("pause specification is invalid"),
            Self::InvalidSourceSelector => formatter.write_str("source selector is invalid"),
            Self::InvalidCondition => formatter.write_str("condition or hit condition is invalid"),
            Self::ConditionEvaluatorUnavailable => {
                formatter.write_str("no bounded CEM-QL condition evaluator is registered")
            }
            Self::ConditionRejected(message) => write!(formatter, "condition rejected: {message}"),
            Self::BreakpointLimit { maximum } => {
                write!(formatter, "operation allows at most {maximum} breakpoints")
            }
            Self::UnknownBreakpoint(id) => write!(formatter, "unknown breakpoint {id}"),
            Self::BreakpointRemoved(id) => write!(formatter, "breakpoint {id} was removed"),
            Self::LocationNotExecutable => formatter.write_str("location is not executable"),
            Self::LocationAmbiguous { candidates } => write!(
                formatter,
                "location matches {} executable safe points",
                candidates.len()
            ),
            Self::UnknownTask(task) => write!(formatter, "unknown logical task {task}"),
            Self::UnknownScope(scope) => write!(formatter, "unknown execution scope {scope}"),
            Self::TaskScopeMismatch => {
                formatter.write_str("safe point is outside the task owner subtree")
            }
            Self::NotStopped => formatter.write_str("operation has no immutable stopped snapshot"),
            Self::ForeignStop(operation) => {
                write!(formatter, "stop belongs to operation {operation}")
            }
            Self::StaleStop(stop) => write!(formatter, "stop {stop} is stale or already consumed"),
            Self::InvalidPageSize { requested, maximum } => {
                write!(formatter, "page size {requested} is outside 1..={maximum}")
            }
            Self::UnknownSnapshotReference(reference) => {
                write!(formatter, "unknown stopped-snapshot reference {reference}")
            }
            Self::NativeValueTypeMismatch => formatter.write_str("native value type mismatch"),
            Self::IdentityExhausted(identity) => write!(formatter, "{identity} space exhausted"),
            Self::Control(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DebugControlError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitCondition {
    Exact(u64),
    AtLeast(u64),
    Every(u64),
}

impl HitCondition {
    fn parse(value: Option<&str>) -> Result<Option<Self>, DebugControlError> {
        let Some(value) = value else {
            return Ok(None);
        };
        let (kind, digits) = if let Some(digits) = value.strip_prefix(">=") {
            (1, digits)
        } else if let Some(digits) = value.strip_prefix('%') {
            (2, digits)
        } else {
            (0, value)
        };
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(DebugControlError::InvalidCondition);
        }
        let number = digits
            .parse::<u64>()
            .ok()
            .filter(|number| *number > 0)
            .ok_or(DebugControlError::InvalidCondition)?;
        Ok(Some(match kind {
            1 => Self::AtLeast(number),
            2 => Self::Every(number),
            _ => Self::Exact(number),
        }))
    }

    fn matches(self, hit: u64) -> bool {
        match self {
            Self::Exact(expected) => hit == expected,
            Self::AtLeast(minimum) => hit >= minimum,
            Self::Every(interval) => hit.is_multiple_of(interval),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugLifecycle {
    Running,
    PauseRequested,
    Stopped,
    Stepping,
    Completed,
}

#[derive(Debug, Clone)]
struct BreakpointRecord {
    spec: PauseSpec,
    resolved: Option<ExecutableSafePoint>,
    target_scope: Option<ExecutionScopeId>,
    hit_condition: Option<HitCondition>,
    hit_count: u64,
    enabled: bool,
    removed: bool,
    last_stop: Option<StoppedEvent>,
    wakers: Vec<Waker>,
}

#[derive(Debug, Clone)]
struct TaskRuntime {
    owner: ExecutionScopeId,
    dependencies: BTreeSet<TaskId>,
    state: DebugTaskState,
    state_before_park: Option<DebugTaskState>,
    physical_worker: Option<u64>,
    frames: Vec<LogicalFrameCapture>,
    safe_point: Option<ExecutableSafePoint>,
    atomic_depth: u32,
}

#[derive(Debug, Clone)]
struct PendingPause {
    generation: u64,
    breakpoint_id: Option<BreakpointId>,
    reason: StopReason,
    triggering_task: Option<TaskId>,
}

#[derive(Debug, Clone)]
struct StepPlan {
    task: TaskId,
    mode: StepMode,
    origin_depth: usize,
    origin_location: Option<SourceLocation>,
    #[cfg(not(target_arch = "wasm32"))]
    allowed_tasks: BTreeSet<TaskId>,
}

#[derive(Clone)]
struct SnapshotValueRecord {
    named: DebugBoundedList<VariableSnapshot>,
    indexed: DebugBoundedList<VariableSnapshot>,
    native_value: Option<Arc<dyn Any + Send + Sync>>,
    native_projection: Option<Value>,
}

impl fmt::Debug for SnapshotValueRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SnapshotValueRecord")
            .field("named", &self.named)
            .field("indexed", &self.indexed)
            .field("native_value", &self.native_value.as_ref().map(|_| "typed"))
            .field("native_projection", &self.native_projection)
            .finish()
    }
}

#[derive(Debug, Clone)]
struct SnapshotFrameRecord {
    scopes: Vec<VariableScopeSnapshot>,
}

#[derive(Debug, Clone)]
struct StoppedSnapshotState {
    public: Arc<SuspendedSnapshot>,
    stopped_at: Instant,
    live_scopes: Vec<ExecutionScopeId>,
    frames_by_task: BTreeMap<TaskId, DebugBoundedList<LogicalFrameSnapshot>>,
    frame_records: BTreeMap<SnapshotReferenceId, SnapshotFrameRecord>,
    value_records: BTreeMap<SnapshotReferenceId, SnapshotValueRecord>,
}

#[derive(Debug)]
struct DebugInner {
    active: bool,
    lifecycle: DebugLifecycle,
    limits: OperationHostLimits,
    snapshot_byte_limit: u64,
    next_breakpoint_id: u64,
    next_safe_point_id: u64,
    next_stop_id: u64,
    breakpoints: BTreeMap<BreakpointId, BreakpointRecord>,
    safe_points: BTreeMap<SafePointId, ExecutableSafePoint>,
    scopes: BTreeMap<ExecutionScopeId, ExecutionScope>,
    tasks: BTreeMap<TaskId, TaskRuntime>,
    pending_pause: Option<PendingPause>,
    current_snapshot: Option<StoppedSnapshotState>,
    last_stop_id: Option<StopId>,
    step_plan: Option<StepPlan>,
    condition_evaluator: Option<Arc<dyn DebugConditionEvaluator>>,
}

impl fmt::Debug for dyn DebugConditionEvaluator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DebugConditionEvaluator")
    }
}

#[derive(Debug)]
struct DebugShared {
    operation_id: OperationId,
    inner: Mutex<DebugInner>,
    changed: Condvar,
    observers: Mutex<Vec<Arc<dyn DebugRuntimeObserver>>>,
}

impl fmt::Debug for dyn DebugRuntimeObserver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DebugRuntimeObserver")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OperationDebugControl {
    shared: Arc<DebugShared>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResumeEffect {
    pub event: ContinuedEvent,
    pub excluded_time: Duration,
    pub live_scopes: Vec<ExecutionScopeId>,
}

impl OperationDebugControl {
    pub(crate) fn new(operation_id: OperationId, root: ExecutionScope) -> Self {
        let root_memory = root.effective_policy.memory_bytes;
        Self {
            shared: Arc::new(DebugShared {
                operation_id,
                inner: Mutex::new(DebugInner {
                    active: false,
                    lifecycle: DebugLifecycle::Running,
                    limits: OperationHostLimits::default(),
                    snapshot_byte_limit: snapshot_byte_limit(
                        OperationHostLimits::default(),
                        root_memory,
                    ),
                    next_breakpoint_id: 1,
                    next_safe_point_id: 1,
                    next_stop_id: 1,
                    breakpoints: BTreeMap::new(),
                    safe_points: BTreeMap::new(),
                    scopes: BTreeMap::from([(root.id, root)]),
                    tasks: BTreeMap::new(),
                    pending_pause: None,
                    current_snapshot: None,
                    last_stop_id: None,
                    step_plan: None,
                    condition_evaluator: None,
                }),
                changed: Condvar::new(),
                observers: Mutex::new(Vec::new()),
            }),
        }
    }

    pub(crate) fn configure(&self, limits: OperationHostLimits, root_memory_bytes: u64) {
        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        inner.limits = limits;
        inner.snapshot_byte_limit = snapshot_byte_limit(limits, root_memory_bytes);
    }

    pub(crate) fn attach_observer(&self, observer: Arc<dyn DebugRuntimeObserver>) {
        self.shared
            .observers
            .lock()
            .expect("poisoned debug observer mutex")
            .push(observer);
    }

    pub(crate) fn activate(
        &self,
        evaluator: Option<Arc<dyn DebugConditionEvaluator>>,
    ) -> Result<(), DebugControlError> {
        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        if inner.lifecycle == DebugLifecycle::Completed {
            return Err(DebugControlError::Completed);
        }
        inner.active = true;
        inner.condition_evaluator = evaluator;
        Ok(())
    }

    pub(crate) fn is_active(&self) -> bool {
        self.shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex")
            .active
    }

    pub(crate) fn register_scope(&self, scope: ExecutionScope) {
        self.shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex")
            .scopes
            .insert(scope.id, scope);
    }

    pub(crate) fn update_scope_state(&self, scope: ExecutionScopeId, state: ExecutionScopeState) {
        if let Some(scope) = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex")
            .scopes
            .get_mut(&scope)
        {
            scope.state = state;
        }
    }

    pub(crate) fn register_task(
        &self,
        task: TaskId,
        owner: ExecutionScopeId,
        dependencies: impl IntoIterator<Item = TaskId>,
    ) {
        self.shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex")
            .tasks
            .insert(
                task,
                TaskRuntime {
                    owner,
                    dependencies: dependencies.into_iter().collect(),
                    state: DebugTaskState::Queued,
                    state_before_park: None,
                    physical_worker: None,
                    frames: Vec::new(),
                    safe_point: None,
                    atomic_depth: 0,
                },
            );
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn task_started(
        &self,
        task: TaskId,
        worker: Option<u64>,
    ) -> Result<(), DebugControlError> {
        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        let stepping_allowed = inner
            .step_plan
            .as_ref()
            .is_some_and(|plan| plan.allowed_tasks.contains(&task));
        let lifecycle = inner.lifecycle;
        let task = inner
            .tasks
            .get_mut(&task)
            .ok_or(DebugControlError::UnknownTask(task))?;
        task.physical_worker = worker;
        if lifecycle == DebugLifecycle::Running || stepping_allowed {
            task.state = DebugTaskState::Running;
            task.state_before_park = None;
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn task_dispatch_allowed(&self, task: TaskId) -> bool {
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        if !inner.active {
            return true;
        }
        match inner.lifecycle {
            DebugLifecycle::Running => true,
            DebugLifecycle::Stepping => inner
                .step_plan
                .as_ref()
                .is_some_and(|plan| plan.allowed_tasks.contains(&task)),
            DebugLifecycle::PauseRequested
            | DebugLifecycle::Stopped
            | DebugLifecycle::Completed => false,
        }
    }

    pub(crate) fn complete_task(&self, task: TaskId) {
        let event = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            inner.tasks.remove(&task);
            try_commit_snapshot(self.shared.operation_id, &mut inner)
        };
        if let Some(event) = event {
            self.publish(DebugRuntimeEvent::Stopped(event));
        }
        self.changed();
    }

    pub(crate) fn register_safe_point(
        &self,
        scope: ExecutionScopeId,
        kind: DebugSafePointKind,
        phase: String,
        location: Option<SourceLocation>,
    ) -> Result<ExecutableSafePoint, DebugControlError> {
        validate_phase(&phase)?;
        if let Some(location) = &location {
            validate_location(location)?;
        }
        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        ensure_active(&inner)?;
        if !inner.scopes.contains_key(&scope) {
            return Err(DebugControlError::UnknownScope(scope));
        }
        if let Some(existing) = inner.safe_points.values().find(|point| {
            point.scope == scope
                && point.kind == kind
                && point.phase == phase
                && point.location == location
        }) {
            return Ok(existing.clone());
        }
        let id = SafePointId::from_raw(inner.next_safe_point_id);
        inner.next_safe_point_id = inner
            .next_safe_point_id
            .checked_add(1)
            .ok_or(DebugControlError::IdentityExhausted("safePointId"))?;
        let point = ExecutableSafePoint {
            id,
            scope,
            kind,
            phase,
            location,
        };
        inner.safe_points.insert(id, point.clone());
        Ok(point)
    }

    pub(crate) fn install_pause(
        &self,
        spec: PauseSpec,
    ) -> Result<PauseTriggerHandle, DebugControlError> {
        validate_pause_spec(&spec)?;
        let (id, resolution) = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            ensure_active(&inner)?;
            if let Some(task) = spec.preferred_task {
                if !inner.tasks.contains_key(&task) {
                    return Err(DebugControlError::UnknownTask(task));
                }
            }
            if inner.breakpoints.len() >= inner.limits.max_debug_breakpoints as usize {
                return Err(DebugControlError::BreakpointLimit {
                    maximum: inner.limits.max_debug_breakpoints,
                });
            }
            if let Some(condition) = &spec.condition {
                let evaluator = inner
                    .condition_evaluator
                    .as_ref()
                    .ok_or(DebugControlError::ConditionEvaluatorUnavailable)?;
                evaluator
                    .validate(condition)
                    .map_err(bounded_condition_error)?;
            }
            let hit_condition = HitCondition::parse(spec.hit_condition.as_deref())?;
            let (resolved, target_scope) = resolve_pause_target(&inner, &spec)?;
            let id = BreakpointId::from_raw(inner.next_breakpoint_id);
            inner.next_breakpoint_id = inner
                .next_breakpoint_id
                .checked_add(1)
                .ok_or(DebugControlError::IdentityExhausted("breakpointId"))?;
            let resolution = BreakpointResolution {
                breakpoint_id: id,
                verified: true,
                location: resolved.clone(),
                error_code: None,
            };
            inner.breakpoints.insert(
                id,
                BreakpointRecord {
                    spec,
                    resolved,
                    target_scope,
                    hit_condition,
                    hit_count: 0,
                    enabled: true,
                    removed: false,
                    last_stop: None,
                    wakers: Vec::new(),
                },
            );
            (id, resolution)
        };
        self.publish(DebugRuntimeEvent::BreakpointResolved(resolution));
        Ok(PauseTriggerHandle {
            control: self.clone(),
            breakpoint_id: id,
            last_seen_stop: None,
            removed: false,
        })
    }

    pub(crate) fn remove_breakpoint(
        &self,
        breakpoint: BreakpointId,
    ) -> Result<(), DebugControlError> {
        let wakers = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            let record = inner
                .breakpoints
                .get_mut(&breakpoint)
                .ok_or(DebugControlError::UnknownBreakpoint(breakpoint))?;
            record.removed = true;
            record.enabled = false;
            std::mem::take(&mut record.wakers)
        };
        wake_all(wakers);
        for observer in self.observers() {
            observer.breakpoint_removed(breakpoint);
        }
        Ok(())
    }

    pub(crate) fn executable_locations(
        &self,
        source_uri: &str,
        start_line: Option<u32>,
        end_line: Option<u32>,
    ) -> Result<Vec<ExecutableSafePoint>, DebugControlError> {
        if source_uri.is_empty()
            || source_uri.len() > MAX_SOURCE_URI_BYTES
            || source_uri.chars().any(char::is_control)
            || matches!((start_line, end_line), (Some(start), Some(end)) if start > end)
        {
            return Err(DebugControlError::InvalidSourceSelector);
        }
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        ensure_active(&inner)?;
        let mut points = inner
            .safe_points
            .values()
            .filter(|point| point.kind.visible())
            .filter(|point| {
                point.location.as_ref().is_some_and(|location| {
                    location.source_uri == source_uri
                        && start_line.is_none_or(|start| location.line >= start)
                        && end_line.is_none_or(|end| location.line <= end)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        points.sort_by_key(|point| {
            point
                .location
                .as_ref()
                .map(|location| (location.line, location.column.unwrap_or(0), point.id))
        });
        Ok(points)
    }

    pub(crate) fn safe_point(
        &self,
        task_id: TaskId,
        capture: DebugSafePointCapture,
    ) -> Result<DebugSafePointOutcome, DebugControlError> {
        if !self.is_active() {
            return Ok(DebugSafePointOutcome::Disabled);
        }
        validate_capture(&capture)?;
        let (scope, point) = {
            let inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            ensure_active(&inner)?;
            let task = inner
                .tasks
                .get(&task_id)
                .ok_or(DebugControlError::UnknownTask(task_id))?;
            let scope = capture
                .frames
                .first()
                .map(|frame| frame.execution_scope)
                .unwrap_or(task.owner);
            if !scope_is_descendant(&inner.scopes, task.owner, scope) {
                return Err(DebugControlError::TaskScopeMismatch);
            }
            (scope, find_or_create_point(&inner, scope, &capture))
        };
        let point = match point {
            Some(point) => point,
            None => self.register_safe_point(
                scope,
                capture.kind,
                capture.phase.clone(),
                capture.location.clone(),
            )?,
        };

        let mut events = Vec::new();
        let should_wait = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            ensure_active(&inner)?;
            if inner.lifecycle == DebugLifecycle::Completed {
                return Ok(DebugSafePointOutcome::Interrupted);
            }
            let task_owner = inner
                .tasks
                .get(&task_id)
                .ok_or(DebugControlError::UnknownTask(task_id))?
                .owner;
            {
                let task = inner.tasks.get_mut(&task_id).expect("validated debug task");
                task.frames = capture.frames.clone();
                task.safe_point = Some(point.clone());
                task.state = DebugTaskState::Running;
            }

            let (breakpoint, invalid_resolutions) = if matches!(
                inner.lifecycle,
                DebugLifecycle::Running | DebugLifecycle::Stepping
            ) {
                matching_breakpoint(&mut inner, task_id, task_owner, &point, &capture)?
            } else {
                (None, Vec::new())
            };
            events.extend(
                invalid_resolutions
                    .into_iter()
                    .map(DebugRuntimeEvent::BreakpointResolved),
            );
            let step_hit = breakpoint.is_none()
                && inner.lifecycle == DebugLifecycle::Stepping
                && inner
                    .step_plan
                    .as_ref()
                    .is_some_and(|plan| step_matches(plan, task_id, &capture));
            if let Some(breakpoint_id) = breakpoint {
                let reason =
                    if inner.breakpoints.get(&breakpoint_id).is_some_and(|record| {
                        record.spec.trigger == PauseTriggerKind::NextSafePoint
                    }) {
                        StopReason::Pause
                    } else {
                        StopReason::Breakpoint
                    };
                let requested =
                    request_pause(&mut inner, Some(breakpoint_id), reason, Some(task_id))?;
                events.push(DebugRuntimeEvent::PauseRequested(requested));
            } else if step_hit {
                let requested = request_pause(&mut inner, None, StopReason::Step, Some(task_id))?;
                events.push(DebugRuntimeEvent::PauseRequested(requested));
            }

            if inner.lifecycle == DebugLifecycle::PauseRequested {
                park_task(&mut inner, task_id);
                if let Some(stopped) = try_commit_snapshot(self.shared.operation_id, &mut inner) {
                    events.push(DebugRuntimeEvent::Stopped(stopped));
                }
            }
            inner
                .tasks
                .get(&task_id)
                .is_some_and(|task| task.state == DebugTaskState::Parked)
        };

        for event in events {
            self.publish(event);
        }
        self.changed();
        if !should_wait {
            return Ok(DebugSafePointOutcome::Running);
        }

        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        loop {
            if inner.lifecycle == DebugLifecycle::Completed || !inner.active {
                return Ok(DebugSafePointOutcome::Interrupted);
            }
            let Some(task) = inner.tasks.get(&task_id) else {
                return Ok(DebugSafePointOutcome::Interrupted);
            };
            if task.state != DebugTaskState::Parked {
                return Ok(DebugSafePointOutcome::Resumed);
            }
            inner = self
                .shared
                .changed
                .wait(inner)
                .expect("poisoned debug-control mutex");
        }
    }

    pub(crate) fn task_external_wait(
        &self,
        task: TaskId,
        waiting: bool,
    ) -> Result<(), DebugControlError> {
        if !self.is_active() {
            return Ok(());
        }
        let event = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            let lifecycle = inner.lifecycle;
            let runtime = inner
                .tasks
                .get_mut(&task)
                .ok_or(DebugControlError::UnknownTask(task))?;
            if waiting {
                runtime.state = DebugTaskState::ExternalWait;
                runtime.state_before_park = None;
            } else if matches!(
                lifecycle,
                DebugLifecycle::PauseRequested | DebugLifecycle::Stopped
            ) {
                runtime.state_before_park = Some(DebugTaskState::Running);
                runtime.state = DebugTaskState::Parked;
            } else {
                runtime.state = DebugTaskState::Running;
                runtime.state_before_park = None;
            }
            try_commit_snapshot(self.shared.operation_id, &mut inner)
        };
        if let Some(event) = event {
            self.publish(DebugRuntimeEvent::Stopped(event));
        }
        self.changed();
        Ok(())
    }

    pub(crate) fn enter_atomic(&self, task: TaskId) -> Result<(), DebugControlError> {
        if !self.is_active() {
            return Ok(());
        }
        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        let runtime = inner
            .tasks
            .get_mut(&task)
            .ok_or(DebugControlError::UnknownTask(task))?;
        runtime.atomic_depth = runtime.atomic_depth.saturating_add(1);
        Ok(())
    }

    pub(crate) fn exit_atomic(&self, task: TaskId) -> Result<(), DebugControlError> {
        if !self.is_active() {
            return Ok(());
        }
        let (event, should_wait) = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            let runtime = inner
                .tasks
                .get_mut(&task)
                .ok_or(DebugControlError::UnknownTask(task))?;
            runtime.atomic_depth = runtime.atomic_depth.saturating_sub(1);
            if runtime.atomic_depth == 0 && inner.lifecycle == DebugLifecycle::PauseRequested {
                park_task(&mut inner, task);
            }
            let event = try_commit_snapshot(self.shared.operation_id, &mut inner);
            let should_wait = inner
                .tasks
                .get(&task)
                .is_some_and(|runtime| runtime.state == DebugTaskState::Parked);
            (event, should_wait)
        };
        if let Some(event) = event {
            self.publish(DebugRuntimeEvent::Stopped(event));
        }
        self.changed();
        if should_wait {
            self.wait_while_parked(task);
        }
        Ok(())
    }

    pub(crate) fn resume(&self, stop: StopToken) -> Result<ResumeEffect, DebugControlError> {
        let effect = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            validate_current_stop(self.shared.operation_id, &inner, stop)?;
            resume_all(&mut inner, stop)
        };
        self.publish(DebugRuntimeEvent::Continued(effect.event.clone()));
        self.changed();
        Ok(effect)
    }

    pub(crate) fn step(&self, request: StepRequest) -> Result<ResumeEffect, DebugControlError> {
        let effect = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            validate_current_stop(self.shared.operation_id, &inner, request.stop)?;
            let (origin_depth, origin_location) = {
                let snapshot = inner
                    .current_snapshot
                    .as_ref()
                    .ok_or(DebugControlError::NotStopped)?;
                let frames = snapshot
                    .frames_by_task
                    .get(&request.task)
                    .ok_or(DebugControlError::UnknownTask(request.task))?;
                let origin = frames
                    .items
                    .first()
                    .ok_or(DebugControlError::UnknownTask(request.task))?;
                (frames.original_count as usize, origin.location.clone())
            };
            let allowed_tasks = dependency_closure(&inner.tasks, request.task)?;
            let stopped = inner
                .current_snapshot
                .take()
                .expect("validated current snapshot");
            let excluded_time = stopped.stopped_at.elapsed();
            let live_scopes = stopped.live_scopes;
            for (task_id, runtime) in &mut inner.tasks {
                if allowed_tasks.contains(task_id) {
                    restore_task(runtime);
                }
            }
            inner.lifecycle = DebugLifecycle::Stepping;
            inner.pending_pause = None;
            inner.step_plan = Some(StepPlan {
                task: request.task,
                mode: request.mode,
                origin_depth,
                origin_location,
                #[cfg(not(target_arch = "wasm32"))]
                allowed_tasks,
            });
            ResumeEffect {
                event: ContinuedEvent {
                    stop: request.stop,
                    all_threads_continued: false,
                    stepping_task: Some(request.task),
                },
                excluded_time,
                live_scopes,
            }
        };
        self.publish(DebugRuntimeEvent::Continued(effect.event.clone()));
        self.changed();
        Ok(effect)
    }

    pub(crate) fn cancel_pause(&self) -> Option<(Duration, Vec<ExecutionScopeId>)> {
        let stopped = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            let stopped = inner.current_snapshot.take();
            inner.pending_pause = None;
            inner.step_plan = None;
            if inner.lifecycle != DebugLifecycle::Completed {
                inner.lifecycle = DebugLifecycle::Running;
            }
            for runtime in inner.tasks.values_mut() {
                restore_task(runtime);
            }
            stopped
        };
        self.changed();
        stopped.map(|stopped| (stopped.stopped_at.elapsed(), stopped.live_scopes))
    }

    pub(crate) fn complete(&self) -> Option<(Duration, Vec<ExecutionScopeId>)> {
        let stopped = {
            let mut inner = self
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            inner.active = false;
            inner.lifecycle = DebugLifecycle::Completed;
            inner.pending_pause = None;
            inner.step_plan = None;
            let stopped = inner.current_snapshot.take();
            for runtime in inner.tasks.values_mut() {
                restore_task(runtime);
            }
            for breakpoint in inner.breakpoints.values_mut() {
                wake_all(std::mem::take(&mut breakpoint.wakers));
            }
            stopped
        };
        self.changed();
        stopped.map(|stopped| (stopped.stopped_at.elapsed(), stopped.live_scopes))
    }

    pub(crate) fn snapshot(
        &self,
        stop: StopToken,
    ) -> Result<Arc<SuspendedSnapshot>, DebugControlError> {
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        validate_current_stop(self.shared.operation_id, &inner, stop)?;
        Ok(Arc::clone(
            &inner
                .current_snapshot
                .as_ref()
                .expect("validated snapshot")
                .public,
        ))
    }

    pub(crate) fn stack_trace(
        &self,
        stop: StopToken,
        task: TaskId,
        start: u32,
        count: Option<u32>,
    ) -> Result<DebugPage<LogicalFrameSnapshot>, DebugControlError> {
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        validate_current_stop(self.shared.operation_id, &inner, stop)?;
        let count = validate_page_size(
            count.unwrap_or(inner.limits.default_stack_frame_page_size),
            inner.limits.max_stack_frame_page_size,
        )?;
        let frames = inner
            .current_snapshot
            .as_ref()
            .expect("validated snapshot")
            .frames_by_task
            .get(&task)
            .ok_or(DebugControlError::UnknownTask(task))?;
        Ok(page(&frames.items, frames.original_count, start, count))
    }

    pub(crate) fn frame_scopes(
        &self,
        stop: StopToken,
        frame: SnapshotReferenceId,
    ) -> Result<Vec<VariableScopeSnapshot>, DebugControlError> {
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        validate_current_stop(self.shared.operation_id, &inner, stop)?;
        inner
            .current_snapshot
            .as_ref()
            .expect("validated snapshot")
            .frame_records
            .get(&frame)
            .map(|record| record.scopes.clone())
            .ok_or(DebugControlError::UnknownSnapshotReference(frame))
    }

    pub(crate) fn variables(
        &self,
        stop: StopToken,
        reference: SnapshotReferenceId,
        filter: VariableFilter,
        start: u32,
        count: Option<u32>,
    ) -> Result<DebugPage<VariableSnapshot>, DebugControlError> {
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        validate_current_stop(self.shared.operation_id, &inner, stop)?;
        let count = validate_page_size(
            count.unwrap_or(inner.limits.default_variable_page_size),
            inner.limits.max_variable_page_size,
        )?;
        let record = inner
            .current_snapshot
            .as_ref()
            .expect("validated snapshot")
            .value_records
            .get(&reference)
            .ok_or(DebugControlError::UnknownSnapshotReference(reference))?;
        let values = match filter {
            VariableFilter::Named => &record.named,
            VariableFilter::Indexed => &record.indexed,
        };
        Ok(page(&values.items, values.original_count, start, count))
    }

    pub(crate) fn native_value<T: Any + Send + Sync>(
        &self,
        stop: StopToken,
        reference: SnapshotReferenceId,
    ) -> Result<Arc<T>, DebugControlError> {
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        validate_current_stop(self.shared.operation_id, &inner, stop)?;
        let value = inner
            .current_snapshot
            .as_ref()
            .expect("validated snapshot")
            .value_records
            .get(&reference)
            .and_then(|record| record.native_value.as_ref())
            .ok_or(DebugControlError::UnknownSnapshotReference(reference))?;
        Arc::clone(value)
            .downcast::<T>()
            .map_err(|_| DebugControlError::NativeValueTypeMismatch)
    }

    pub(crate) fn native_projection(
        &self,
        stop: StopToken,
        reference: SnapshotReferenceId,
    ) -> Result<Value, DebugControlError> {
        let inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        validate_current_stop(self.shared.operation_id, &inner, stop)?;
        inner
            .current_snapshot
            .as_ref()
            .expect("validated snapshot")
            .value_records
            .get(&reference)
            .and_then(|record| record.native_projection.clone())
            .ok_or(DebugControlError::UnknownSnapshotReference(reference))
    }

    fn trigger_poll(
        &self,
        breakpoint: BreakpointId,
        last_seen: Option<StopId>,
        waker: Option<&Waker>,
    ) -> Result<TriggerPoll, DebugControlError> {
        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        let completed = inner.lifecycle == DebugLifecycle::Completed;
        let record = inner
            .breakpoints
            .get_mut(&breakpoint)
            .ok_or(DebugControlError::UnknownBreakpoint(breakpoint))?;
        if record.removed {
            return Err(DebugControlError::BreakpointRemoved(breakpoint));
        }
        if let Some(event) = &record.last_stop {
            if last_seen.is_none_or(|seen| event.stop.stop_id > seen) {
                return Ok(TriggerPoll::Event(event.clone()));
            }
        }
        if completed {
            return Ok(TriggerPoll::Closed);
        }
        if let Some(waker) = waker {
            if !record.wakers.iter().any(|stored| stored.will_wake(waker)) {
                record.wakers.push(waker.clone());
            }
        }
        Ok(TriggerPoll::Pending)
    }

    fn publish(&self, event: DebugRuntimeEvent) {
        for observer in self.observers() {
            observer.event(event.clone());
        }
    }

    fn changed(&self) {
        self.shared.changed.notify_all();
        for observer in self.observers() {
            observer.wake_scheduler();
        }
    }

    fn observers(&self) -> Vec<Arc<dyn DebugRuntimeObserver>> {
        self.shared
            .observers
            .lock()
            .expect("poisoned debug observer mutex")
            .clone()
    }

    fn wait_while_parked(&self, task: TaskId) {
        let mut inner = self
            .shared
            .inner
            .lock()
            .expect("poisoned debug-control mutex");
        while inner.lifecycle != DebugLifecycle::Completed
            && inner
                .tasks
                .get(&task)
                .is_some_and(|runtime| runtime.state == DebugTaskState::Parked)
        {
            inner = self
                .shared
                .changed
                .wait(inner)
                .expect("poisoned debug-control mutex");
        }
    }
}

enum TriggerPoll {
    Event(StoppedEvent),
    Pending,
    Closed,
}

#[derive(Debug, Clone)]
pub struct PauseTriggerHandle {
    control: OperationDebugControl,
    breakpoint_id: BreakpointId,
    last_seen_stop: Option<StopId>,
    removed: bool,
}

impl PauseTriggerHandle {
    pub fn breakpoint_id(&self) -> BreakpointId {
        self.breakpoint_id
    }

    pub fn next_stop(&mut self) -> PauseTriggerFuture<'_> {
        PauseTriggerFuture { handle: self }
    }

    pub fn try_next(&mut self) -> Result<Option<StoppedEvent>, DebugControlError> {
        if self.removed {
            return Err(DebugControlError::BreakpointRemoved(self.breakpoint_id));
        }
        match self
            .control
            .trigger_poll(self.breakpoint_id, self.last_seen_stop, None)?
        {
            TriggerPoll::Event(event) => {
                self.last_seen_stop = Some(event.stop.stop_id);
                Ok(Some(event))
            }
            TriggerPoll::Pending | TriggerPoll::Closed => Ok(None),
        }
    }

    pub fn blocking_next_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<StoppedEvent>, DebugControlError> {
        let deadline = Instant::now() + timeout;
        loop {
            match self
                .control
                .trigger_poll(self.breakpoint_id, self.last_seen_stop, None)?
            {
                TriggerPoll::Event(event) => {
                    self.last_seen_stop = Some(event.stop.stop_id);
                    return Ok(Some(event));
                }
                TriggerPoll::Closed => return Ok(None),
                TriggerPoll::Pending => {}
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let inner = self
                .control
                .shared
                .inner
                .lock()
                .expect("poisoned debug-control mutex");
            let completed = inner.lifecycle == DebugLifecycle::Completed;
            let record = inner
                .breakpoints
                .get(&self.breakpoint_id)
                .ok_or(DebugControlError::UnknownBreakpoint(self.breakpoint_id))?;
            if record.removed {
                return Err(DebugControlError::BreakpointRemoved(self.breakpoint_id));
            }
            if record.last_stop.as_ref().is_some_and(|event| {
                self.last_seen_stop
                    .is_none_or(|seen| event.stop.stop_id > seen)
            }) {
                drop(inner);
                continue;
            }
            if completed {
                return Ok(None);
            }
            let (_guard, result) = self
                .control
                .shared
                .changed
                .wait_timeout(inner, deadline.saturating_duration_since(now))
                .expect("poisoned debug-control mutex");
            if result.timed_out() {
                return Ok(None);
            }
        }
    }

    pub fn remove(&mut self) -> Result<(), DebugControlError> {
        if self.removed {
            return Ok(());
        }
        self.control.remove_breakpoint(self.breakpoint_id)?;
        self.removed = true;
        Ok(())
    }
}

pub struct PauseTriggerFuture<'a> {
    handle: &'a mut PauseTriggerHandle,
}

impl Future for PauseTriggerFuture<'_> {
    type Output = Result<Option<StoppedEvent>, DebugControlError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.handle.removed {
            return Poll::Ready(Err(DebugControlError::BreakpointRemoved(
                self.handle.breakpoint_id,
            )));
        }
        match self.handle.control.trigger_poll(
            self.handle.breakpoint_id,
            self.handle.last_seen_stop,
            Some(context.waker()),
        ) {
            Ok(TriggerPoll::Event(event)) => {
                self.handle.last_seen_stop = Some(event.stop.stop_id);
                Poll::Ready(Ok(Some(event)))
            }
            Ok(TriggerPoll::Pending) => Poll::Pending,
            Ok(TriggerPoll::Closed) => Poll::Ready(Ok(None)),
            Err(error) => Poll::Ready(Err(error)),
        }
    }
}

fn ensure_active(inner: &DebugInner) -> Result<(), DebugControlError> {
    if inner.lifecycle == DebugLifecycle::Completed {
        return Err(DebugControlError::Completed);
    }
    if !inner.active {
        return Err(DebugControlError::Inactive);
    }
    Ok(())
}

fn validate_phase(phase: &str) -> Result<(), DebugControlError> {
    if phase.is_empty()
        || phase.len() > MAX_EXECUTION_SCOPE_LABEL_BYTES
        || phase.chars().any(char::is_control)
    {
        return Err(DebugControlError::InvalidPauseSpec);
    }
    Ok(())
}

fn validate_location(location: &SourceLocation) -> Result<(), DebugControlError> {
    if location.source_uri.is_empty()
        || location.source_uri.len() > MAX_SOURCE_URI_BYTES
        || location.source_uri.chars().any(char::is_control)
        || location.line == 0
        || location.column == Some(0)
        || location.end_line == Some(0)
        || location.end_column == Some(0)
        || location.end_line.is_some_and(|end| end < location.line)
    {
        return Err(DebugControlError::InvalidSourceSelector);
    }
    Ok(())
}

fn validate_selector(selector: &DebugSourceSelector) -> Result<(), DebugControlError> {
    validate_location(&SourceLocation {
        source_uri: selector.source_uri.clone(),
        line: selector.line,
        column: selector.column,
        end_line: selector.end_line,
        end_column: selector.end_column,
        byte_range: selector.byte_range,
    })
}

fn validate_pause_spec(spec: &PauseSpec) -> Result<(), DebugControlError> {
    if spec.condition.as_ref().is_some_and(|value| {
        value.is_empty()
            || value.len() > MAX_CONTROL_REASON_BYTES
            || value.chars().any(char::is_control)
    }) || spec
        .hit_condition
        .as_ref()
        .is_some_and(|value| value.len() > 32 || value.chars().any(char::is_control))
    {
        return Err(DebugControlError::InvalidCondition);
    }
    if let Some(source) = &spec.source {
        validate_selector(source)?;
    }
    match spec.trigger {
        PauseTriggerKind::NextSafePoint => {
            if spec.source.is_some() || spec.persistent {
                return Err(DebugControlError::InvalidPauseSpec);
            }
        }
        PauseTriggerKind::ScopeEnter | PauseTriggerKind::ScopeExit => {
            if spec.scope.is_some() == spec.source.is_some() || spec.preferred_task.is_some() {
                return Err(DebugControlError::InvalidPauseSpec);
            }
        }
        PauseTriggerKind::SourceLocation => {
            if spec.source.is_none() || spec.scope.is_some() || spec.preferred_task.is_some() {
                return Err(DebugControlError::InvalidPauseSpec);
            }
        }
    }
    Ok(())
}

fn validate_capture(capture: &DebugSafePointCapture) -> Result<(), DebugControlError> {
    validate_phase(&capture.phase)?;
    if let Some(location) = &capture.location {
        validate_location(location)?;
    }
    for frame in &capture.frames {
        validate_phase(&frame.name)?;
        validate_phase(&frame.phase)?;
        if let Some(location) = &frame.location {
            validate_location(location)?;
        }
        for scope in &frame.variable_scopes {
            validate_phase(&scope.name)?;
            for variable in &scope.variables {
                validate_phase(&variable.name)?;
                if let Some(location) = &variable.declaration {
                    validate_location(location)?;
                }
            }
        }
    }
    Ok(())
}

fn resolve_pause_target(
    inner: &DebugInner,
    spec: &PauseSpec,
) -> Result<(Option<ExecutableSafePoint>, Option<ExecutionScopeId>), DebugControlError> {
    if let Some(scope) = spec.scope {
        if !inner.scopes.contains_key(&scope) {
            return Err(DebugControlError::UnknownScope(scope));
        }
        return Ok((None, Some(scope)));
    }
    let Some(selector) = &spec.source else {
        return Ok((None, None));
    };
    if let Some(scope) = selector.scope {
        if !inner.scopes.contains_key(&scope) {
            return Err(DebugControlError::UnknownScope(scope));
        }
    }
    let candidates: Vec<_> = inner
        .safe_points
        .values()
        .filter(|point| selector_matches(selector, point))
        .cloned()
        .collect();
    match candidates.as_slice() {
        [point] => Ok((Some(point.clone()), Some(point.scope))),
        [] => Err(DebugControlError::LocationNotExecutable),
        _ => Err(DebugControlError::LocationAmbiguous { candidates }),
    }
}

fn selector_matches(selector: &DebugSourceSelector, point: &ExecutableSafePoint) -> bool {
    if selector.scope.is_some_and(|scope| scope != point.scope) {
        return false;
    }
    let Some(location) = &point.location else {
        return false;
    };
    location.source_uri == selector.source_uri
        && location.line == selector.line
        && selector
            .column
            .is_none_or(|column| location.column == Some(column))
        && selector
            .end_line
            .is_none_or(|line| location.end_line == Some(line))
        && selector
            .end_column
            .is_none_or(|column| location.end_column == Some(column))
        && selector
            .byte_range
            .as_ref()
            .is_none_or(|range| location.byte_range.as_ref() == Some(range))
}

fn find_or_create_point(
    inner: &DebugInner,
    scope: ExecutionScopeId,
    capture: &DebugSafePointCapture,
) -> Option<ExecutableSafePoint> {
    inner
        .safe_points
        .values()
        .find(|point| {
            point.scope == scope
                && point.kind == capture.kind
                && point.phase == capture.phase
                && point.location == capture.location
        })
        .cloned()
}

fn matching_breakpoint(
    inner: &mut DebugInner,
    task: TaskId,
    task_owner: ExecutionScopeId,
    point: &ExecutableSafePoint,
    capture: &DebugSafePointCapture,
) -> Result<(Option<BreakpointId>, Vec<BreakpointResolution>), DebugControlError> {
    let mut invalid_resolutions = Vec::new();
    let ids: Vec<_> = inner.breakpoints.keys().copied().collect();
    for id in ids {
        let matches = {
            let record = inner.breakpoints.get(&id).expect("known breakpoint");
            record.enabled
                && !record.removed
                && breakpoint_matches(
                    record,
                    task,
                    task_owner,
                    point,
                    capture,
                    &inner.scopes,
                    &inner.tasks,
                )
        };
        if !matches {
            continue;
        }
        let condition = inner
            .breakpoints
            .get(&id)
            .and_then(|record| record.spec.condition.clone());
        if let Some(condition) = condition {
            let evaluator = inner
                .condition_evaluator
                .as_ref()
                .ok_or(DebugControlError::ConditionEvaluatorUnavailable)?;
            let context = DebugConditionContext {
                task,
                scope: point.scope,
                location: point.location.clone(),
                frame_names: capture
                    .frames
                    .iter()
                    .map(|frame| frame.name.clone())
                    .collect(),
                frames: capture.frames.clone(),
            };
            match evaluator.evaluate(&condition, &context) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(message) => {
                    let record = inner.breakpoints.get_mut(&id).expect("known breakpoint");
                    record.enabled = false;
                    invalid_resolutions.push(BreakpointResolution {
                        breakpoint_id: id,
                        verified: false,
                        location: record.resolved.clone(),
                        error_code: Some(bounded_condition_error(message).code().to_owned()),
                    });
                    continue;
                }
            }
        }
        let record = inner.breakpoints.get_mut(&id).expect("known breakpoint");
        record.hit_count = record.hit_count.saturating_add(1);
        if record
            .hit_condition
            .is_some_and(|condition| !condition.matches(record.hit_count))
        {
            continue;
        }
        if !record.spec.persistent {
            record.enabled = false;
        }
        return Ok((Some(id), invalid_resolutions));
    }
    Ok((None, invalid_resolutions))
}

fn breakpoint_matches(
    record: &BreakpointRecord,
    task: TaskId,
    task_owner: ExecutionScopeId,
    point: &ExecutableSafePoint,
    capture: &DebugSafePointCapture,
    scopes: &BTreeMap<ExecutionScopeId, ExecutionScope>,
    tasks: &BTreeMap<TaskId, TaskRuntime>,
) -> bool {
    if record.spec.preferred_task.is_some_and(|preferred| {
        preferred != task
            && tasks
                .get(&preferred)
                .is_some_and(|runtime| runtime.state == DebugTaskState::Running)
    }) {
        return false;
    }
    let scope_match = record
        .target_scope
        .is_none_or(|scope| scope_is_descendant(scopes, scope, point.scope));
    if !scope_match || !scope_is_descendant(scopes, task_owner, point.scope) {
        return false;
    }
    match record.spec.trigger {
        PauseTriggerKind::NextSafePoint => capture.kind.visible(),
        PauseTriggerKind::ScopeEnter => capture.kind == DebugSafePointKind::ScopeEnter,
        PauseTriggerKind::ScopeExit => capture.kind == DebugSafePointKind::ScopeExit,
        PauseTriggerKind::SourceLocation => record
            .resolved
            .as_ref()
            .is_some_and(|resolved| resolved.id == point.id),
    }
}

fn scope_is_descendant(
    scopes: &BTreeMap<ExecutionScopeId, ExecutionScope>,
    ancestor: ExecutionScopeId,
    candidate: ExecutionScopeId,
) -> bool {
    let mut cursor = Some(candidate);
    while let Some(scope) = cursor {
        if scope == ancestor {
            return true;
        }
        cursor = scopes.get(&scope).and_then(|node| node.parent);
    }
    false
}

fn bounded_condition_error(message: impl Into<String>) -> DebugControlError {
    let message = message.into();
    DebugControlError::ConditionRejected(truncate_utf8(&message, MAX_CONTROL_REASON_BYTES).0)
}

fn request_pause(
    inner: &mut DebugInner,
    breakpoint_id: Option<BreakpointId>,
    reason: StopReason,
    triggering_task: Option<TaskId>,
) -> Result<PauseRequestedEvent, DebugControlError> {
    inner.next_stop_id = inner
        .next_stop_id
        .checked_add(1)
        .ok_or(DebugControlError::IdentityExhausted("stopId"))?;
    let generation = inner.next_stop_id - 1;
    inner.lifecycle = DebugLifecycle::PauseRequested;
    inner.step_plan = None;
    inner.pending_pause = Some(PendingPause {
        generation,
        breakpoint_id,
        reason,
        triggering_task,
    });
    for runtime in inner.tasks.values_mut() {
        if runtime.state == DebugTaskState::Queued {
            park_runtime(runtime);
        }
    }
    Ok(PauseRequestedEvent {
        breakpoint_id,
        generation,
        reason,
        triggering_task,
    })
}

fn park_task(inner: &mut DebugInner, task: TaskId) {
    if let Some(runtime) = inner.tasks.get_mut(&task) {
        if runtime.atomic_depth == 0 && runtime.state != DebugTaskState::ExternalWait {
            park_runtime(runtime);
        }
    }
}

fn park_runtime(runtime: &mut TaskRuntime) {
    if runtime.state != DebugTaskState::Parked {
        runtime.state_before_park = Some(runtime.state);
        runtime.state = DebugTaskState::Parked;
    }
}

fn restore_task(runtime: &mut TaskRuntime) {
    if runtime.state == DebugTaskState::Parked {
        runtime.state = runtime
            .state_before_park
            .take()
            .unwrap_or(DebugTaskState::Running);
    }
}

fn try_commit_snapshot(operation_id: OperationId, inner: &mut DebugInner) -> Option<StoppedEvent> {
    if inner.lifecycle != DebugLifecycle::PauseRequested
        || inner.pending_pause.is_none()
        || inner.tasks.values().any(|task| {
            !matches!(
                task.state,
                DebugTaskState::Parked | DebugTaskState::ExternalWait
            ) || task.atomic_depth > 0
        })
    {
        return None;
    }
    let pending = inner.pending_pause.take().expect("checked pending pause");
    let stop = StopToken {
        operation_id,
        stop_id: StopId::from_raw(pending.generation),
    };
    let stopped = StoppedEvent {
        stop,
        reason: pending.reason,
        breakpoint_id: pending.breakpoint_id,
        triggering_task: pending.triggering_task,
        all_threads_stopped: true,
    };
    let snapshot = build_snapshot(inner, stopped.clone());
    inner.last_stop_id = Some(stop.stop_id);
    inner.current_snapshot = Some(snapshot);
    inner.lifecycle = DebugLifecycle::Stopped;
    if let Some(breakpoint) = pending.breakpoint_id {
        if let Some(record) = inner.breakpoints.get_mut(&breakpoint) {
            record.last_stop = Some(stopped.clone());
            wake_all(std::mem::take(&mut record.wakers));
        }
    }
    Some(stopped)
}

fn resume_all(inner: &mut DebugInner, stop: StopToken) -> ResumeEffect {
    let stopped = inner
        .current_snapshot
        .take()
        .expect("validated current snapshot");
    let excluded_time = stopped.stopped_at.elapsed();
    let live_scopes = stopped.live_scopes;
    for runtime in inner.tasks.values_mut() {
        restore_task(runtime);
    }
    inner.lifecycle = DebugLifecycle::Running;
    inner.pending_pause = None;
    inner.step_plan = None;
    ResumeEffect {
        event: ContinuedEvent {
            stop,
            all_threads_continued: true,
            stepping_task: None,
        },
        excluded_time,
        live_scopes,
    }
}

fn validate_current_stop(
    operation_id: OperationId,
    inner: &DebugInner,
    stop: StopToken,
) -> Result<(), DebugControlError> {
    if stop.operation_id != operation_id {
        return Err(DebugControlError::ForeignStop(stop.operation_id));
    }
    let Some(current) = &inner.current_snapshot else {
        return match inner.last_stop_id {
            Some(last) if stop.stop_id <= last => Err(DebugControlError::StaleStop(stop.stop_id)),
            _ => Err(DebugControlError::NotStopped),
        };
    };
    if current.public.stop != stop {
        return Err(DebugControlError::StaleStop(stop.stop_id));
    }
    Ok(())
}

fn dependency_closure(
    tasks: &BTreeMap<TaskId, TaskRuntime>,
    selected: TaskId,
) -> Result<BTreeSet<TaskId>, DebugControlError> {
    if !tasks.contains_key(&selected) {
        return Err(DebugControlError::UnknownTask(selected));
    }
    let mut closure = BTreeSet::new();
    let mut pending = vec![selected];
    while let Some(task) = pending.pop() {
        if !closure.insert(task) {
            continue;
        }
        let runtime = tasks
            .get(&task)
            .ok_or(DebugControlError::UnknownTask(task))?;
        pending.extend(runtime.dependencies.iter().copied());
    }
    Ok(closure)
}

fn step_matches(plan: &StepPlan, task: TaskId, capture: &DebugSafePointCapture) -> bool {
    if task != plan.task || !capture.kind.visible() {
        return false;
    }
    let depth = capture.frames.len();
    let changed = capture.location != plan.origin_location;
    match plan.mode {
        StepMode::Next => depth <= plan.origin_depth && changed,
        StepMode::StepIn => depth > plan.origin_depth || changed,
        StepMode::StepOut => depth < plan.origin_depth,
    }
}

struct SnapshotBuilder {
    next_reference: u64,
    remaining: u64,
    limit: u64,
    preview_limit: usize,
    frame_records: BTreeMap<SnapshotReferenceId, SnapshotFrameRecord>,
    value_records: BTreeMap<SnapshotReferenceId, SnapshotValueRecord>,
    identities: BTreeMap<u64, SnapshotReferenceId>,
}

impl SnapshotBuilder {
    fn new(limit: u64, preview_limit: u32) -> Self {
        Self {
            next_reference: 1,
            remaining: limit,
            limit,
            preview_limit: preview_limit as usize,
            frame_records: BTreeMap::new(),
            value_records: BTreeMap::new(),
            identities: BTreeMap::new(),
        }
    }

    fn retained_bytes(&self) -> u64 {
        self.limit.saturating_sub(self.remaining)
    }

    fn charge(&mut self, bytes: usize) -> bool {
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        if bytes > self.remaining {
            return false;
        }
        self.remaining -= bytes;
        true
    }

    fn reference(&mut self) -> Option<SnapshotReferenceId> {
        if !self.charge(std::mem::size_of::<SnapshotReferenceId>()) {
            return None;
        }
        let reference = SnapshotReferenceId::from_raw(self.next_reference);
        self.next_reference = self.next_reference.checked_add(1)?;
        Some(reference)
    }

    fn text(&mut self, value: &str, maximum: usize) -> String {
        let maximum = maximum.min(self.remaining as usize);
        let (value, _) = truncate_utf8(value, maximum);
        let _ = self.charge(value.len());
        value
    }

    fn value(&mut self, capture: &DebugValueCapture, depth: usize) -> DebugValueSummary {
        let original_length = capture
            .original_length
            .unwrap_or(capture.preview.len() as u64);
        let preview_max = self.preview_limit.min(self.remaining as usize);
        let (preview, preview_truncated) = truncate_utf8(&capture.preview, preview_max);
        let _ = self.charge(preview.len());
        let type_name = self.text(&capture.type_name, MAX_EXECUTION_SCOPE_LABEL_BYTES);

        if let Some(identity) = capture.identity {
            if let Some(reference) = self.identities.get(&identity).copied() {
                return DebugValueSummary {
                    type_name,
                    preview,
                    original_length,
                    preview_truncated,
                    variables_reference: Some(reference),
                    named_variables: capture.named.len() as u64,
                    indexed_variables: capture.indexed.len() as u64,
                    native_value: capture.native_value.is_some(),
                    opaque: false,
                };
            }
        }

        let has_reference = !capture.named.is_empty()
            || !capture.indexed.is_empty()
            || capture.native_value.is_some();
        let reference = has_reference.then(|| self.reference()).flatten();
        if let (Some(identity), Some(reference)) = (capture.identity, reference) {
            self.identities.insert(identity, reference);
        }
        let mut named = Vec::new();
        let mut indexed = Vec::new();
        if depth < 64 && reference.is_some() {
            for variable in &capture.named {
                if !self.charge(std::mem::size_of::<VariableSnapshot>()) {
                    break;
                }
                named.push(self.variable(variable, depth + 1));
            }
            for (index, value) in capture.indexed.iter().enumerate() {
                if !self.charge(std::mem::size_of::<VariableSnapshot>()) {
                    break;
                }
                indexed.push(VariableSnapshot {
                    name: format!("[{index}]"),
                    declaration: None,
                    value: self.value(value, depth + 1),
                });
            }
        }
        if let Some(reference) = reference {
            let native_projection = capture.native_projection.clone().filter(|projection| {
                serde_json::to_vec(projection)
                    .ok()
                    .is_some_and(|encoded| self.charge(encoded.len()))
            });
            self.value_records.insert(
                reference,
                SnapshotValueRecord {
                    named: DebugBoundedList::from_items(named, capture.named.len()),
                    indexed: DebugBoundedList::from_items(indexed, capture.indexed.len()),
                    native_value: capture.native_value.clone(),
                    native_projection,
                },
            );
        }
        DebugValueSummary {
            type_name,
            preview,
            original_length,
            preview_truncated,
            variables_reference: reference,
            named_variables: capture.named.len() as u64,
            indexed_variables: capture.indexed.len() as u64,
            native_value: capture.native_value.is_some(),
            opaque: reference.is_none() && has_reference,
        }
    }

    fn variable(&mut self, capture: &DebugVariableCapture, depth: usize) -> VariableSnapshot {
        VariableSnapshot {
            name: self.text(&capture.name, MAX_EXECUTION_SCOPE_LABEL_BYTES),
            declaration: capture.declaration.clone(),
            value: self.value(&capture.value, depth),
        }
    }

    fn variable_scope(
        &mut self,
        capture: &DebugVariableScopeCapture,
    ) -> Option<VariableScopeSnapshot> {
        let reference = self.reference()?;
        let mut variables = Vec::new();
        for variable in &capture.variables {
            if !self.charge(std::mem::size_of::<VariableSnapshot>()) {
                break;
            }
            variables.push(self.variable(variable, 0));
        }
        self.value_records.insert(
            reference,
            SnapshotValueRecord {
                named: DebugBoundedList::from_items(variables, capture.variables.len()),
                indexed: DebugBoundedList::from_items(Vec::new(), 0),
                native_value: None,
                native_projection: None,
            },
        );
        Some(VariableScopeSnapshot {
            name: self.text(&capture.name, MAX_EXECUTION_SCOPE_LABEL_BYTES),
            expensive: capture.expensive,
            variables_reference: reference,
            named_variables: capture.variables.len() as u64,
        })
    }

    fn frame(
        &mut self,
        task: TaskId,
        index: usize,
        capture: &LogicalFrameCapture,
    ) -> Option<LogicalFrameSnapshot> {
        if !self.charge(std::mem::size_of::<LogicalFrameSnapshot>()) {
            return None;
        }
        let reference = self.reference()?;
        let public = LogicalFrameSnapshot {
            reference,
            task,
            index: u32::try_from(index).unwrap_or(u32::MAX),
            name: self.text(&capture.name, MAX_EXECUTION_SCOPE_LABEL_BYTES),
            phase: self.text(&capture.phase, MAX_EXECUTION_SCOPE_LABEL_BYTES),
            location: capture.location.clone(),
            execution_scope: capture.execution_scope,
        };
        let mut scopes = Vec::new();
        for scope in &capture.variable_scopes {
            if self.remaining == 0 {
                break;
            }
            let Some(scope) = self.variable_scope(scope) else {
                break;
            };
            scopes.push(scope);
        }
        self.frame_records
            .insert(reference, SnapshotFrameRecord { scopes });
        Some(public)
    }
}

fn build_snapshot(inner: &DebugInner, stopped: StoppedEvent) -> StoppedSnapshotState {
    let mut builder = SnapshotBuilder::new(
        inner.snapshot_byte_limit,
        inner.limits.max_debug_value_preview_bytes,
    );
    let original_thread_count = inner.tasks.len();
    let mut threads = Vec::new();
    let mut frames_by_task = BTreeMap::new();
    for (task_id, runtime) in &inner.tasks {
        if !builder.charge(std::mem::size_of::<ThreadSnapshot>()) {
            break;
        }
        let mut frames = Vec::new();
        for (index, capture) in runtime.frames.iter().enumerate() {
            let Some(frame) = builder.frame(*task_id, index, capture) else {
                break;
            };
            frames.push(frame);
        }
        threads.push(ThreadSnapshot {
            task: *task_id,
            state: runtime.state,
            owner: runtime.owner,
            physical_worker: runtime.physical_worker,
            captured_frame_count: u32::try_from(frames.len()).unwrap_or(u32::MAX),
            original_frame_count: runtime.frames.len() as u64,
        });
        frames_by_task.insert(
            *task_id,
            DebugBoundedList::from_items(frames, runtime.frames.len()),
        );
    }

    let live_scopes: Vec<_> = inner
        .scopes
        .values()
        .filter(|scope| scope.state != ExecutionScopeState::Completed)
        .map(|scope| scope.id)
        .collect();
    let original_scope_count = live_scopes.len();
    let mut execution_scopes = Vec::new();
    for scope_id in &live_scopes {
        let scope = inner.scopes.get(scope_id).expect("known live scope");
        if !builder.charge(std::mem::size_of::<ExecutionScopeSnapshot>()) {
            break;
        }
        execution_scopes.push(ExecutionScopeSnapshot {
            id: scope.id,
            parent: scope.parent,
            kind: format!("{:?}", scope.kind).to_ascii_lowercase(),
            label: builder.text(&scope.label, MAX_EXECUTION_SCOPE_LABEL_BYTES),
            state: scope.state,
            source_location: scope.source_location.clone(),
        });
    }
    let retained_bytes = builder.retained_bytes();
    let public = Arc::new(SuspendedSnapshot {
        stop: stopped.stop,
        reason: stopped.reason,
        triggering_task: stopped.triggering_task,
        threads: DebugBoundedList::from_items(threads, original_thread_count),
        execution_scopes: DebugBoundedList::from_items(execution_scopes, original_scope_count),
        retained_bytes,
        retained_byte_limit: inner.snapshot_byte_limit,
    });
    StoppedSnapshotState {
        public,
        stopped_at: Instant::now(),
        live_scopes,
        frames_by_task,
        frame_records: builder.frame_records,
        value_records: builder.value_records,
    }
}

fn snapshot_byte_limit(limits: OperationHostLimits, root_memory_bytes: u64) -> u64 {
    limits
        .max_suspended_snapshot_bytes
        .min(MAX_SUSPENDED_SNAPSHOT_BYTES)
        .min(root_memory_bytes / 8)
}

fn validate_page_size(requested: u32, maximum: u32) -> Result<u32, DebugControlError> {
    if requested == 0 || requested > maximum {
        return Err(DebugControlError::InvalidPageSize { requested, maximum });
    }
    Ok(requested)
}

fn page<T: Clone>(items: &[T], total: u64, start: u32, count: u32) -> DebugPage<T> {
    let start_index = start as usize;
    let end = start_index.saturating_add(count as usize).min(items.len());
    let page = if start_index < items.len() {
        items[start_index..end].to_vec()
    } else {
        Vec::new()
    };
    DebugPage {
        items: page,
        start,
        total,
    }
}

fn truncate_utf8(value: &str, maximum: usize) -> (String, bool) {
    if value.len() <= maximum {
        return (value.to_owned(), false);
    }
    let mut end = maximum.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_owned(), true)
}

fn wake_all(wakers: Vec<Waker>) {
    for waker in wakers {
        waker.wake();
    }
}

const _: () = {
    assert!(MAX_STACK_FRAME_PAGE_SIZE > 0);
    assert!(MAX_VARIABLE_PAGE_SIZE > 0);
    assert!(MAX_DEBUG_VALUE_PREVIEW_BYTES > 0);
    assert!(MAX_DEBUG_BREAKPOINTS > 0);
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    use crate::capability::{
        capability_manifest, CapabilityAvailability, CapabilityRequest, ControlCapabilityKind,
        RuntimeKind,
    };
    use crate::operation_control::{ControlError, OperationControl};
    use crate::operation_handle::{
        CancelRequest, EventSubscriptionOptions, OperationEventKind, OperationEventSubscription,
        OperationHandle, OperationOutcome, OperationTerminalPublisher,
    };
    use crate::scheduler::{
        AbortSignal, NativeScheduler, ScheduledTaskSpec, SchedulerTrace, ScopePolicy, TaskPath,
    };

    struct TestConditionEvaluator {
        fail_runtime: AtomicBool,
    }

    impl TestConditionEvaluator {
        fn passing() -> Self {
            Self {
                fail_runtime: AtomicBool::new(false),
            }
        }
    }

    impl DebugConditionEvaluator for TestConditionEvaluator {
        fn validate(&self, expression: &str) -> Result<(), String> {
            (expression == "true")
                .then_some(())
                .ok_or_else(|| "only the fixture boolean is accepted".to_owned())
        }

        fn evaluate(
            &self,
            _expression: &str,
            _context: &DebugConditionContext,
        ) -> Result<bool, String> {
            if self.fail_runtime.load(Ordering::SeqCst) {
                Err("fixture condition type mismatch".to_owned())
            } else {
                Ok(true)
            }
        }
    }

    fn operation(
        limits: OperationHostLimits,
    ) -> (
        OperationControl,
        OperationHandle<()>,
        OperationTerminalPublisher<()>,
    ) {
        let control = OperationControl::new(AbortSignal::new());
        let (handle, publisher) = OperationHandle::new(control.clone(), limits).unwrap();
        handle.activate_debug_control(None).unwrap();
        (control, handle, publisher)
    }

    fn location(line: u32) -> SourceLocation {
        SourceLocation {
            source_uri: "file:///fixture.cemt".to_owned(),
            line,
            column: Some(3),
            end_line: None,
            end_column: None,
            byte_range: Some(ByteRange::new(u64::from(line) * 10, 4)),
        }
    }

    fn frame(scope: ExecutionScopeId, line: u32, name: &str) -> LogicalFrameCapture {
        LogicalFrameCapture {
            name: name.to_owned(),
            phase: "evaluate".to_owned(),
            location: Some(location(line)),
            execution_scope: scope,
            variable_scopes: Vec::new(),
        }
    }

    fn capture(scope: ExecutionScopeId, line: u32, depth: usize) -> DebugSafePointCapture {
        let mut frames = vec![frame(scope, line, &format!("frame-{depth}"))];
        for index in 1..depth {
            frames.push(frame(scope, line.saturating_sub(index as u32), "caller"));
        }
        DebugSafePointCapture::visible("evaluate", Some(location(line)), frames)
    }

    fn next_stopped(subscription: &mut OperationEventSubscription<()>) -> StoppedEvent {
        loop {
            let event = subscription
                .blocking_next_timeout(Duration::from_secs(2))
                .unwrap()
                .expect("debug operation should publish a stopped event");
            if event.kind == OperationEventKind::Stopped {
                return serde_json::from_value(event.payload.clone()).unwrap();
            }
        }
    }

    #[test]
    fn default_feature_capability_and_negotiated_debug_limits_are_discoverable() {
        assert!(cfg!(feature = "debug-control"));
        let manifest = capability_manifest(CapabilityRequest {
            runtime: RuntimeKind::Native,
            target_identity: "x86_64-unknown-linux-gnu".to_owned(),
            abi_identity: "cem-ml-rust-v1".to_owned(),
            debug_control_active: true,
        })
        .unwrap();
        assert!(manifest.debug_control.compiled);
        assert!(manifest.debug_control.active);
        for control in [
            ControlCapabilityKind::Pause,
            ControlCapabilityKind::SourceBreakpoints,
            ControlCapabilityKind::Stepping,
            ControlCapabilityKind::SuspendedInspection,
        ] {
            assert_eq!(
                manifest.control(control).availability,
                CapabilityAvailability::Available
            );
        }
        assert_eq!(manifest.operation_limits.default_stack_frame_page_size, 64);
        assert_eq!(manifest.operation_limits.max_stack_frame_page_size, 512);
        assert_eq!(manifest.operation_limits.default_variable_page_size, 100);
        assert_eq!(manifest.operation_limits.max_variable_page_size, 1_000);
        assert_eq!(
            manifest.operation_limits.max_debug_value_preview_bytes,
            4_096
        );
        assert_eq!(
            manifest.operation_limits.max_suspended_snapshot_bytes,
            16 * 1_024 * 1_024
        );
    }

    #[test]
    fn all_stop_waits_for_atomic_exit_and_classifies_queued_and_external_tasks() {
        let (control, handle, _) = operation(OperationHostLimits::default());
        let root = control.root_scope();
        let atomic_task = control.register_task(root).unwrap();
        let queued_task = control.register_task(root).unwrap();
        let external_task = control.register_task(root).unwrap();
        control
            .set_debug_external_wait(external_task, true)
            .unwrap();
        control.enter_debug_atomic_region(atomic_task).unwrap();
        let mut trigger = handle.pause(PauseSpec::next_safe_point(None)).unwrap();

        let worker_control = control.clone();
        let worker = thread::spawn(move || {
            assert_eq!(
                worker_control
                    .debug_safe_point(atomic_task, capture(root, 1, 1))
                    .unwrap(),
                DebugSafePointOutcome::Running
            );
            worker_control
                .exit_debug_atomic_region(atomic_task)
                .unwrap();
            worker_control.complete_task(atomic_task).unwrap();
        });

        let stopped = trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .expect("atomic exit completes rendezvous");
        let snapshot = handle.suspended_snapshot(stopped.stop).unwrap();
        assert_eq!(snapshot.threads.original_count, 3);
        assert_eq!(snapshot.threads.items.len(), 3);
        assert!(snapshot.threads.items.iter().any(|thread| {
            thread.task == queued_task && thread.state == DebugTaskState::Parked
        }));
        assert!(snapshot.threads.items.iter().any(|thread| {
            thread.task == external_task && thread.state == DebugTaskState::ExternalWait
        }));
        handle.resume(stopped.stop).unwrap();
        worker.join().unwrap();
        assert!(matches!(
            handle.resume(stopped.stop),
            Err(DebugControlError::StaleStop(_))
        ));

        let foreign = OperationControl::new(AbortSignal::new());
        assert!(matches!(
            foreign.resume_debug(stopped.stop),
            Err(DebugControlError::ForeignStop(_)) | Err(DebugControlError::Inactive)
        ));
        control.complete_task(queued_task).unwrap();
        control.complete_task(external_task).unwrap();
    }

    #[test]
    fn native_scheduler_parks_before_dispatch_and_reports_physical_worker_as_metadata() {
        let (control, handle, _) = operation(OperationHostLimits::default());
        let scheduler = NativeScheduler::new(control.clone(), SchedulerTrace::new()).unwrap();
        let mut trigger = handle.pause(PauseSpec::next_safe_point(None)).unwrap();
        let ran = Arc::new(AtomicBool::new(false));
        let ran_in_task = Arc::clone(&ran);
        let task = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(control.root_scope(), TaskPath::root(1), "debug-dispatch"),
                move || ran_in_task.store(true, Ordering::SeqCst),
            )
            .unwrap();
        let task_id = task.id();
        let stopped = trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert!(!ran.load(Ordering::SeqCst));
        let snapshot = handle.suspended_snapshot(stopped.stop).unwrap();
        let thread = snapshot
            .threads
            .items
            .iter()
            .find(|thread| thread.task == task_id)
            .unwrap();
        assert!(thread.physical_worker.is_some());
        handle.resume(stopped.stop).unwrap();
        task.join().unwrap();
        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn all_stop_does_not_publish_a_partial_snapshot_while_a_running_task_is_unparked() {
        let (control, handle, _) = operation(OperationHostLimits::default());
        let root = control.root_scope();
        let triggering = control.register_task(root).unwrap();
        let running = control.register_task(root).unwrap();
        control.set_debug_external_wait(running, true).unwrap();
        control.set_debug_external_wait(running, false).unwrap();
        let mut trigger = handle.pause(PauseSpec::next_safe_point(None)).unwrap();
        let triggering_control = control.clone();
        let first_worker = thread::spawn(move || {
            triggering_control
                .debug_safe_point(triggering, capture(root, 5, 1))
                .unwrap();
            triggering_control.complete_task(triggering).unwrap();
        });
        assert!(trigger
            .blocking_next_timeout(Duration::from_millis(50))
            .unwrap()
            .is_none());

        let running_control = control.clone();
        let second_worker = thread::spawn(move || {
            running_control
                .debug_safe_point(running, capture(root, 6, 1))
                .unwrap();
            running_control.complete_task(running).unwrap();
        });
        let stopped = trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert_eq!(
            handle
                .suspended_snapshot(stopped.stop)
                .unwrap()
                .threads
                .original_count,
            2
        );
        handle.resume(stopped.stop).unwrap();
        first_worker.join().unwrap();
        second_worker.join().unwrap();
    }

    #[test]
    fn source_and_scope_breakpoints_persist_with_conditions_and_hit_counts() {
        let control = OperationControl::new(AbortSignal::new());
        let (handle, _) = OperationHandle::<()>::with_defaults(control.clone()).unwrap();
        handle
            .activate_debug_control(Some(Arc::new(TestConditionEvaluator::passing())))
            .unwrap();
        let root = control.root_scope();
        handle
            .register_debug_safe_point(
                root,
                DebugSafePointKind::Visible,
                "evaluate",
                Some(location(7)),
            )
            .unwrap();
        let mut source_trigger = handle
            .pause(PauseSpec {
                condition: Some("true".to_owned()),
                hit_condition: Some("%2".to_owned()),
                ..PauseSpec::source(DebugSourceSelector {
                    source_uri: location(7).source_uri,
                    line: 7,
                    column: Some(3),
                    end_line: None,
                    end_column: None,
                    byte_range: None,
                    scope: Some(root),
                })
            })
            .unwrap();
        let task = control.register_task(root).unwrap();
        let worker_control = control.clone();
        let worker = thread::spawn(move || {
            for _ in 0..4 {
                worker_control
                    .debug_safe_point(task, capture(root, 7, 1))
                    .unwrap();
            }
            worker_control.complete_task(task).unwrap();
        });

        let first = source_trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert_eq!(first.reason, StopReason::Breakpoint);
        handle.resume(first.stop).unwrap();
        let second = source_trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert!(second.stop.stop_id > first.stop.stop_id);
        handle.resume(second.stop).unwrap();
        worker.join().unwrap();

        let scope_task = control.register_task(root).unwrap();
        let mut scope_trigger = handle
            .pause(PauseSpec {
                trigger: PauseTriggerKind::ScopeEnter,
                scope: Some(root),
                preferred_task: None,
                source: None,
                condition: None,
                hit_condition: None,
                persistent: false,
            })
            .unwrap();
        let scope_control = control.clone();
        let scope_worker = thread::spawn(move || {
            let mut enter = capture(root, 8, 1);
            enter.kind = DebugSafePointKind::ScopeEnter;
            scope_control.debug_safe_point(scope_task, enter).unwrap();
            scope_control.complete_task(scope_task).unwrap();
        });
        let scope_stop = scope_trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        handle.resume(scope_stop.stop).unwrap();
        scope_worker.join().unwrap();

        for phase in ["first", "second"] {
            handle
                .register_debug_safe_point(
                    root,
                    DebugSafePointKind::Visible,
                    phase,
                    Some(location(9)),
                )
                .unwrap();
        }
        assert!(matches!(
            handle.pause(PauseSpec::source(DebugSourceSelector {
                source_uri: location(9).source_uri,
                line: 9,
                column: Some(3),
                end_line: None,
                end_column: None,
                byte_range: None,
                scope: Some(root),
            })),
            Err(DebugControlError::LocationAmbiguous { candidates }) if candidates.len() == 2
        ));
        assert!(matches!(
            handle.pause(PauseSpec::source(DebugSourceSelector {
                source_uri: location(99).source_uri,
                line: 99,
                column: Some(3),
                end_line: None,
                end_column: None,
                byte_range: None,
                scope: Some(root),
            })),
            Err(DebugControlError::LocationNotExecutable)
        ));
    }

    #[test]
    fn stepping_runs_selected_dependency_closure_and_breakpoint_wins_next() {
        let (control, handle, _) = operation(OperationHostLimits::default());
        let root = control.root_scope();
        handle
            .register_debug_safe_point(
                root,
                DebugSafePointKind::Visible,
                "evaluate",
                Some(location(4)),
            )
            .unwrap();
        let mut line_four = handle
            .pause(PauseSpec::source(DebugSourceSelector {
                source_uri: location(4).source_uri,
                line: 4,
                column: Some(3),
                end_line: None,
                end_column: None,
                byte_range: None,
                scope: Some(root),
            }))
            .unwrap();
        let dependency = control.register_task(root).unwrap();
        let selected = control
            .register_task_with_dependencies(root, [dependency])
            .unwrap();
        let unrelated = control.register_task(root).unwrap();
        let mut manual = handle
            .pause(PauseSpec::next_safe_point(Some(root)))
            .unwrap();
        let mut events = handle
            .subscribe(EventSubscriptionOptions::default())
            .unwrap();
        let worker_control = control.clone();
        let worker = thread::spawn(move || {
            for (line, depth) in [(1, 1), (2, 2), (3, 1), (4, 1)] {
                worker_control
                    .debug_safe_point(selected, capture(root, line, depth))
                    .unwrap();
            }
            worker_control.complete_task(selected).unwrap();
        });

        let first = manual
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert_eq!(next_stopped(&mut events).stop, first.stop);
        let continued = handle
            .step(StepRequest {
                stop: first.stop,
                task: selected,
                mode: StepMode::StepIn,
            })
            .unwrap();
        assert!(!continued.all_threads_continued);
        let second = next_stopped(&mut events);
        assert_eq!(second.reason, StopReason::Step);
        let snapshot = handle.suspended_snapshot(second.stop).unwrap();
        assert!(snapshot
            .threads
            .items
            .iter()
            .any(|thread| { thread.task == unrelated && thread.state == DebugTaskState::Parked }));

        handle
            .step(StepRequest {
                stop: second.stop,
                task: selected,
                mode: StepMode::StepOut,
            })
            .unwrap();
        let third = next_stopped(&mut events);
        assert_eq!(third.reason, StopReason::Step);
        handle
            .step(StepRequest {
                stop: third.stop,
                task: selected,
                mode: StepMode::Next,
            })
            .unwrap();
        let fourth = line_four
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert_eq!(fourth.reason, StopReason::Breakpoint);
        handle.resume(fourth.stop).unwrap();
        worker.join().unwrap();
        control.complete_task(dependency).unwrap();
        control.complete_task(unrelated).unwrap();
    }

    #[test]
    fn stopped_snapshot_pages_variables_cycles_and_native_values_with_generation_lifetime() {
        let limits = OperationHostLimits {
            max_debug_value_preview_bytes: 8,
            max_suspended_snapshot_bytes: 128 * 1_024,
            ..OperationHostLimits::default()
        };
        let (control, handle, _) = operation(limits);
        let root = control.root_scope();
        let task = control.register_task(root).unwrap();
        let mut trigger = handle.pause(PauseSpec::next_safe_point(None)).unwrap();
        let worker_control = control.clone();
        let worker = thread::spawn(move || {
            let cycle_leaf = DebugValueCapture {
                identity: Some(1),
                ..DebugValueCapture::scalar("cem-node", "same node")
            };
            let value = DebugValueCapture {
                identity: Some(1),
                type_name: "cem-node".to_owned(),
                preview: "a preview longer than eight bytes".to_owned(),
                original_length: Some(40),
                named: vec![DebugVariableCapture {
                    name: "self".to_owned(),
                    declaration: Some(location(11)),
                    value: cycle_leaf,
                }],
                indexed: vec![DebugValueCapture::scalar("text", "child")],
                native_value: Some(Arc::new(vec![1_u8, 2, 3])),
                native_projection: Some(serde_json::json!([1, 2, 3])),
            };
            let mut frame = frame(root, 11, "template-call");
            frame.variable_scopes = vec![DebugVariableScopeCapture {
                name: "lexical".to_owned(),
                expensive: false,
                variables: vec![
                    DebugVariableCapture {
                        name: "node".to_owned(),
                        declaration: Some(location(11)),
                        value,
                    },
                    DebugVariableCapture {
                        name: "flag".to_owned(),
                        declaration: None,
                        value: DebugValueCapture::scalar("boolean", "true"),
                    },
                ],
            }];
            worker_control
                .debug_safe_point(
                    task,
                    DebugSafePointCapture::visible("evaluate", Some(location(11)), vec![frame]),
                )
                .unwrap();
            worker_control.complete_task(task).unwrap();
        });

        let stopped = trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        let snapshot = handle.suspended_snapshot(stopped.stop).unwrap();
        assert!(snapshot.retained_bytes <= snapshot.retained_byte_limit);
        let frames = handle
            .debug_stack_trace(stopped.stop, task, 0, Some(1))
            .unwrap();
        assert_eq!(frames.total, 1);
        let scopes = handle
            .debug_frame_scopes(stopped.stop, frames.items[0].reference)
            .unwrap();
        let variables = handle
            .debug_variables(
                stopped.stop,
                scopes[0].variables_reference,
                VariableFilter::Named,
                0,
                Some(1),
            )
            .unwrap();
        assert_eq!(variables.total, 2);
        let node = &variables.items[0].value;
        assert!(node.preview_truncated);
        assert!(node.preview.len() <= 8);
        let node_reference = node.variables_reference.unwrap();
        let children = handle
            .debug_variables(stopped.stop, node_reference, VariableFilter::Named, 0, None)
            .unwrap();
        assert_eq!(
            children.items[0].value.variables_reference,
            Some(node_reference)
        );
        assert_eq!(
            handle
                .debug_native_value::<Vec<u8>>(stopped.stop, node_reference)
                .unwrap()
                .as_slice(),
            &[1, 2, 3]
        );
        assert!(matches!(
            handle.debug_stack_trace(stopped.stop, task, 0, Some(513)),
            Err(DebugControlError::InvalidPageSize { .. })
        ));
        handle.resume(stopped.stop).unwrap();
        worker.join().unwrap();
        assert!(matches!(
            handle.debug_variables(stopped.stop, node_reference, VariableFilter::Named, 0, None,),
            Err(DebugControlError::StaleStop(_))
        ));
    }

    #[test]
    fn cancellation_and_terminal_completion_wake_parked_tasks_and_invalidate_snapshots() {
        let (control, handle, _) = operation(OperationHostLimits::default());
        let root = control.root_scope();
        let task = control.register_task(root).unwrap();
        let mut trigger = handle.pause(PauseSpec::next_safe_point(None)).unwrap();
        let worker_control = control.clone();
        let worker =
            thread::spawn(move || worker_control.debug_safe_point(task, capture(root, 12, 1)));
        let stopped = trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        handle
            .cancel(CancelRequest {
                reason: Some("fixture cancellation".to_owned()),
                ..CancelRequest::default()
            })
            .unwrap();
        assert!(matches!(
            worker.join().unwrap(),
            Err(DebugControlError::Control(ControlError::Triggered(_)))
        ));
        assert!(matches!(
            handle.suspended_snapshot(stopped.stop),
            Err(DebugControlError::StaleStop(_)) | Err(DebugControlError::NotStopped)
        ));

        let (terminal_control, terminal_handle, terminal_publisher) =
            operation(OperationHostLimits::default());
        let terminal_root = terminal_control.root_scope();
        let terminal_task = terminal_control.register_task(terminal_root).unwrap();
        let mut terminal_trigger = terminal_handle
            .pause(PauseSpec::next_safe_point(None))
            .unwrap();
        let parked_control = terminal_control.clone();
        let parked = thread::spawn(move || {
            parked_control
                .debug_safe_point(terminal_task, capture(terminal_root, 13, 1))
                .unwrap()
        });
        terminal_trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        terminal_publisher
            .settle(OperationOutcome::succeeded(
                (),
                Vec::new(),
                crate::operation_handle::ArtifactDisposition::default(),
            ))
            .unwrap();
        assert_eq!(parked.join().unwrap(), DebugSafePointOutcome::Interrupted);
    }

    #[test]
    fn completed_stop_time_is_excluded_from_active_deadlines() {
        // Leave enough active time for this worker to be scheduled even while
        // the large library test binary is running in parallel. The parked
        // interval still exceeds the entire configured active deadline.
        let policy = ScopePolicy::host_root().with_timeout_ms(Some(2_000));
        let control = OperationControl::with_root_policy(AbortSignal::new(), policy).unwrap();
        let (handle, _) = OperationHandle::<()>::with_defaults(control.clone()).unwrap();
        handle.activate_debug_control(None).unwrap();
        let root = control.root_scope();
        let task = control.register_task(root).unwrap();
        let mut trigger = handle.pause(PauseSpec::next_safe_point(None)).unwrap();
        let worker_control = control.clone();
        let worker = thread::spawn(move || {
            worker_control
                .debug_safe_point(task, capture(root, 14, 1))
                .unwrap();
            worker_control.complete_task(task).unwrap();
        });
        let stopped = trigger
            .blocking_next_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        thread::sleep(Duration::from_millis(2_100));
        handle.resume(stopped.stop).unwrap();
        worker.join().unwrap();
        control.check_scope(root).unwrap();
    }

    #[test]
    fn retained_breakpoint_resolutions_survive_ring_gaps_until_removal() {
        let limits = OperationHostLimits {
            default_subscription_capacity: 2,
            max_subscription_capacity: 4,
            ..OperationHostLimits::default()
        };
        let (control, handle, _) = operation(limits);
        let root = control.root_scope();
        for line in [20, 21] {
            handle
                .register_debug_safe_point(
                    root,
                    DebugSafePointKind::Visible,
                    "evaluate",
                    Some(location(line)),
                )
                .unwrap();
        }
        let mut first = handle
            .pause(PauseSpec::source(DebugSourceSelector {
                source_uri: location(20).source_uri,
                line: 20,
                column: Some(3),
                end_line: None,
                end_column: None,
                byte_range: None,
                scope: Some(root),
            }))
            .unwrap();
        let second = handle
            .pause(PauseSpec::source(DebugSourceSelector {
                source_uri: location(21).source_uri,
                line: 21,
                column: Some(3),
                end_line: None,
                end_column: None,
                byte_range: None,
                scope: Some(root),
            }))
            .unwrap();
        for index in 0..10 {
            handle
                .publish_event(OperationEventKind::Progress, &index)
                .unwrap();
        }
        let collect_resolutions = |subscription: &mut OperationEventSubscription<()>| {
            let mut resolutions = BTreeSet::new();
            loop {
                match subscription.try_next().unwrap() {
                    (crate::operation_handle::EventSubscriptionPoll::Event, Some(event)) => {
                        if event.kind == OperationEventKind::BreakpointResolved {
                            resolutions.insert(event.payload["breakpointId"].as_u64().unwrap());
                        }
                    }
                    (crate::operation_handle::EventSubscriptionPoll::Pending, None) => break,
                    other => panic!("unexpected subscription state: {other:?}"),
                }
            }
            resolutions
        };
        let mut retained = handle
            .subscribe(EventSubscriptionOptions::default())
            .unwrap();
        assert_eq!(collect_resolutions(&mut retained).len(), 2);
        let removed_id = first.breakpoint_id().get();
        first.remove().unwrap();
        let mut after_remove = handle
            .subscribe(EventSubscriptionOptions::default())
            .unwrap();
        let remaining = collect_resolutions(&mut after_remove);
        assert_eq!(remaining.len(), 1);
        assert!(!remaining.contains(&removed_id));
        assert!(remaining.contains(&second.breakpoint_id().get()));
    }
}
