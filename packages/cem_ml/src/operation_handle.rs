//! Awaitable operation ownership, bounded event subscriptions, and host control.
//!
//! Engine results and retained values remain native typed values. Only bounded
//! event metadata and opaque retained-handle identities cross a host boundary.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
#[cfg(feature = "debug-control")]
use std::sync::Weak;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::capability::{
    capability_manifest, CapabilityError, CapabilityManifest, CapabilityRequest,
    OperationHostLimits, DEFAULT_MAX_LIVE_SUBSCRIPTIONS, DEFAULT_STACK_FRAME_PAGE_SIZE,
    DEFAULT_SUBSCRIPTION_CAPACITY, DEFAULT_VARIABLE_PAGE_SIZE, MAX_ARTIFACT_REFERENCES,
    MAX_DEBUG_BREAKPOINTS, MAX_DEBUG_VALUE_PREVIEW_BYTES, MAX_IDENTITY_BYTES,
    MAX_INLINE_EVENT_PAYLOAD_BYTES, MAX_RECOVERED_CONTROL_FAILURES, MAX_RETAINED_HANDLES,
    MAX_STACK_FRAME_PAGE_SIZE, MAX_SUBSCRIPTION_CAPACITY, MAX_SUSPENDED_SNAPSHOT_BYTES,
    MAX_TERMINAL_DIAGNOSTICS, MAX_VARIABLE_PAGE_SIZE,
};
#[cfg(feature = "debug-control")]
use crate::debug_control::{
    BreakpointId, DebugConditionEvaluator, DebugControlError, DebugPage, DebugRuntimeEvent,
    DebugRuntimeObserver, DebugSafePointKind, ExecutableSafePoint, LogicalFrameSnapshot, PauseSpec,
    PauseTriggerHandle, SnapshotReferenceId, StepRequest, StopToken, SuspendedSnapshot,
    VariableFilter, VariableScopeSnapshot, VariableSnapshot,
};
use crate::diagnostics::Diagnostic;
#[cfg(feature = "debug-control")]
use crate::operation_control::TaskId;
use crate::operation_control::{
    ControlCause, ControlError, ControlFailure, ControlFailureSettlement, ControlRequestOutcome,
    ControlTerminalClass, ExecutionScopeId, ExecutionScopeState, ExecutionScopeTree,
    OperationControl, OperationId, MAX_CONTROL_REASON_BYTES, MAX_SOURCE_URI_BYTES,
};

pub const OPERATION_PROTOCOL_VERSION: u16 = 1;

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

opaque_id!(EventSubscriptionId);
opaque_id!(RetainedHandleId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationHostMessageKind {
    Initialize,
    Run,
    Progress,
    Event,
    Result,
    Control,
}

/// Versioned common envelope for initialize/run/progress/event/result/control
/// projections. Initialize is the only message without an operation identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationHostEnvelope<T> {
    pub protocol_version: u16,
    pub kind: OperationHostMessageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<OperationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    pub payload: T,
}

impl<T> OperationHostEnvelope<T> {
    pub fn initialize(payload: T) -> Self {
        Self {
            protocol_version: OPERATION_PROTOCOL_VERSION,
            kind: OperationHostMessageKind::Initialize,
            operation_id: None,
            sequence: None,
            payload,
        }
    }

    pub fn operation(
        kind: OperationHostMessageKind,
        operation_id: OperationId,
        sequence: Option<u64>,
        payload: T,
    ) -> Result<Self, OperationHandleError> {
        if kind == OperationHostMessageKind::Initialize {
            return Err(OperationHandleError::InvalidEnvelope);
        }
        Ok(Self {
            protocol_version: OPERATION_PROTOCOL_VERSION,
            kind,
            operation_id: Some(operation_id),
            sequence,
            payload,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationInitializeRequest {
    pub protocol_version: u16,
    pub capability: CapabilityRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationInitializeResponse {
    pub protocol_version: u16,
    pub capability: CapabilityManifest,
}

pub fn initialize_operation_host(
    request: OperationInitializeRequest,
) -> Result<OperationInitializeResponse, OperationHandleError> {
    initialize_operation_host_with_limits(request, OperationHostLimits::default())
}

pub fn initialize_operation_host_with_limits(
    request: OperationInitializeRequest,
    limits: OperationHostLimits,
) -> Result<OperationInitializeResponse, OperationHandleError> {
    if request.protocol_version != OPERATION_PROTOCOL_VERSION {
        return Err(OperationHandleError::ProtocolVersion {
            requested: request.protocol_version,
            supported: OPERATION_PROTOCOL_VERSION,
        });
    }
    validate_limits(limits)?;
    let mut capability =
        capability_manifest(request.capability).map_err(OperationHandleError::Capability)?;
    capability.operation_limits = limits;
    Ok(OperationInitializeResponse {
        protocol_version: OPERATION_PROTOCOL_VERSION,
        capability,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationEventKind {
    Accepted,
    ScopeCreated,
    ScopeState,
    TaskState,
    Progress,
    Diagnostic,
    Observability,
    BreakpointResolved,
    PauseRequested,
    Stopped,
    Continued,
    ControlFailure,
    SubscriptionGap,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub protocol_version: u16,
    pub operation_id: OperationId,
    pub sequence: u64,
    pub kind: OperationEventKind,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionGap {
    pub first_missing: u64,
    pub last_missing: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSubscriptionOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
    #[serde(default)]
    pub filters: BTreeSet<OperationEventKind>,
}

impl Default for EventSubscriptionOptions {
    fn default() -> Self {
        Self {
            from_sequence: Some(1),
            capacity: None,
            filters: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSubscriptionPoll {
    Event,
    Pending,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationSourceSelector {
    pub source_uri: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ExecutionScopeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_selector: Option<OperationSourceSelector>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlAckDisposition {
    Accepted,
    AlreadyRequested,
}

impl From<ControlRequestOutcome> for ControlAckDisposition {
    fn from(outcome: ControlRequestOutcome) -> Self {
        match outcome {
            ControlRequestOutcome::Accepted => Self::Accepted,
            ControlRequestOutcome::AlreadyRequested => Self::AlreadyRequested,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlAck {
    pub operation_id: OperationId,
    pub selected_scope: ExecutionScopeId,
    pub disposition: ControlAckDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RetainedHandleKind {
    NativeValue,
    Artifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetainedHandleMetadata {
    pub operation_id: OperationId,
    pub handle_id: RetainedHandleId,
    pub kind: RetainedHandleKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardedArtifact {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundedList<T> {
    pub items: Vec<T>,
    pub original_count: u32,
}

impl<T> BoundedList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            original_count: items.len().try_into().unwrap_or(u32::MAX),
            items,
        }
    }

    pub fn was_truncated(&self) -> bool {
        self.original_count as usize > self.items.len()
    }

    fn enforce(mut self, maximum: u32) -> Self {
        self.original_count = self
            .original_count
            .max(self.items.len().try_into().unwrap_or(u32::MAX));
        self.items.truncate(maximum as usize);
        self
    }
}

impl<T> Default for BoundedList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            original_count: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDisposition {
    pub retained: BoundedList<RetainedHandleMetadata>,
    pub discarded: BoundedList<DiscardedArtifact>,
}

impl ArtifactDisposition {
    pub fn new(retained: Vec<RetainedHandleMetadata>, discarded: Vec<DiscardedArtifact>) -> Self {
        Self {
            retained: BoundedList::new(retained),
            discarded: BoundedList::new(discarded),
        }
    }

    fn enforce(self, maximum: u32) -> Self {
        Self {
            retained: self.retained.enforce(maximum),
            discarded: self.discarded.enforce(maximum),
        }
    }
}

#[derive(Debug)]
pub enum OperationOutcome<R> {
    Succeeded {
        result: R,
        recovered_control_failures: BoundedList<ControlFailureSettlement>,
        artifacts: ArtifactDisposition,
    },
    Failed {
        cause: ControlFailure,
        diagnostics: BoundedList<Diagnostic>,
        artifacts: ArtifactDisposition,
    },
    Cancelled {
        reason: Option<String>,
        diagnostics: BoundedList<Diagnostic>,
        artifacts: ArtifactDisposition,
    },
    Fatal {
        cause: ControlFailure,
        diagnostics: BoundedList<Diagnostic>,
        restartable: bool,
        artifacts: ArtifactDisposition,
    },
}

impl<R> OperationOutcome<R> {
    pub fn succeeded(
        result: R,
        recovered_control_failures: Vec<ControlFailureSettlement>,
        artifacts: ArtifactDisposition,
    ) -> Self {
        Self::Succeeded {
            result,
            recovered_control_failures: BoundedList::new(recovered_control_failures),
            artifacts,
        }
    }

    pub fn failed(
        cause: ControlFailure,
        diagnostics: Vec<Diagnostic>,
        artifacts: ArtifactDisposition,
    ) -> Self {
        Self::Failed {
            cause,
            diagnostics: BoundedList::new(diagnostics),
            artifacts,
        }
    }

    pub fn cancelled(
        reason: Option<String>,
        diagnostics: Vec<Diagnostic>,
        artifacts: ArtifactDisposition,
    ) -> Self {
        Self::Cancelled {
            reason,
            diagnostics: BoundedList::new(diagnostics),
            artifacts,
        }
    }

    pub fn fatal(
        cause: ControlFailure,
        diagnostics: Vec<Diagnostic>,
        restartable: bool,
        artifacts: ArtifactDisposition,
    ) -> Self {
        Self::Fatal {
            cause,
            diagnostics: BoundedList::new(diagnostics),
            restartable,
            artifacts,
        }
    }

    pub fn from_control_failure(
        failure: ControlFailure,
        diagnostics: Vec<Diagnostic>,
        artifacts: ArtifactDisposition,
    ) -> Self {
        match failure.terminal_class() {
            ControlTerminalClass::Cancelled => Self::cancelled(
                failure.cause.cancellation_reason().map(str::to_owned),
                diagnostics,
                artifacts,
            ),
            ControlTerminalClass::Failed => Self::failed(failure, diagnostics, artifacts),
            ControlTerminalClass::Fatal => {
                let restartable = failure.cause.restartable();
                Self::fatal(failure, diagnostics, restartable, artifacts)
            }
        }
    }

    fn enforce(self, limits: OperationHostLimits) -> Self {
        match self {
            Self::Succeeded {
                result,
                recovered_control_failures,
                artifacts,
            } => Self::Succeeded {
                result,
                recovered_control_failures: recovered_control_failures
                    .enforce(limits.max_recovered_control_failures),
                artifacts: artifacts.enforce(limits.max_artifact_references),
            },
            Self::Failed {
                cause,
                diagnostics,
                artifacts,
            } => Self::Failed {
                cause,
                diagnostics: diagnostics.enforce(limits.max_terminal_diagnostics),
                artifacts: artifacts.enforce(limits.max_artifact_references),
            },
            Self::Cancelled {
                reason,
                diagnostics,
                artifacts,
            } => Self::Cancelled {
                reason,
                diagnostics: diagnostics.enforce(limits.max_terminal_diagnostics),
                artifacts: artifacts.enforce(limits.max_artifact_references),
            },
            Self::Fatal {
                cause,
                diagnostics,
                restartable,
                artifacts,
            } => Self::Fatal {
                cause,
                diagnostics: diagnostics.enforce(limits.max_terminal_diagnostics),
                restartable,
                artifacts: artifacts.enforce(limits.max_artifact_references),
            },
        }
    }

    pub fn status(&self) -> OperationTerminalStatus {
        match self {
            Self::Succeeded { .. } => OperationTerminalStatus::Succeeded,
            Self::Failed { .. } => OperationTerminalStatus::Failed,
            Self::Cancelled { .. } => OperationTerminalStatus::Cancelled,
            Self::Fatal { .. } => OperationTerminalStatus::Fatal,
        }
    }

    fn summary(&self) -> OperationTerminalSummary {
        match self {
            Self::Succeeded {
                recovered_control_failures,
                artifacts,
                ..
            } => OperationTerminalSummary {
                status: self.status(),
                cause_code: None,
                diagnostic_count: 0,
                recovered_control_failure_count: recovered_control_failures.original_count,
                retained_artifact_count: artifacts.retained.original_count,
                discarded_artifact_count: artifacts.discarded.original_count,
                restartable: None,
            },
            Self::Failed {
                cause,
                diagnostics,
                artifacts,
            } => OperationTerminalSummary {
                status: self.status(),
                cause_code: Some(cause.code().to_owned()),
                diagnostic_count: diagnostics.original_count,
                recovered_control_failure_count: 0,
                retained_artifact_count: artifacts.retained.original_count,
                discarded_artifact_count: artifacts.discarded.original_count,
                restartable: None,
            },
            Self::Cancelled {
                diagnostics,
                artifacts,
                ..
            } => OperationTerminalSummary {
                status: self.status(),
                cause_code: None,
                diagnostic_count: diagnostics.original_count,
                recovered_control_failure_count: 0,
                retained_artifact_count: artifacts.retained.original_count,
                discarded_artifact_count: artifacts.discarded.original_count,
                restartable: None,
            },
            Self::Fatal {
                cause,
                diagnostics,
                restartable,
                artifacts,
            } => OperationTerminalSummary {
                status: self.status(),
                cause_code: Some(cause.code().to_owned()),
                diagnostic_count: diagnostics.original_count,
                recovered_control_failure_count: 0,
                retained_artifact_count: artifacts.retained.original_count,
                discarded_artifact_count: artifacts.discarded.original_count,
                restartable: Some(*restartable),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationTerminalStatus {
    Succeeded,
    Failed,
    Cancelled,
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationTerminalSummary {
    pub status: OperationTerminalStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause_code: Option<String>,
    pub diagnostic_count: u32,
    pub recovered_control_failure_count: u32,
    pub retained_artifact_count: u32,
    pub discarded_artifact_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restartable: Option<bool>,
}

#[derive(Debug)]
pub enum TerminalClaim<R> {
    Published(Arc<OperationOutcome<R>>),
    AlreadyPublished(Arc<OperationOutcome<R>>),
}

impl<R> TerminalClaim<R> {
    pub fn published(&self) -> bool {
        matches!(self, Self::Published(_))
    }

    pub fn outcome(&self) -> &Arc<OperationOutcome<R>> {
        match self {
            Self::Published(outcome) | Self::AlreadyPublished(outcome) => outcome,
        }
    }
}

#[derive(Debug)]
pub enum OperationHandleError {
    ProtocolVersion {
        requested: u16,
        supported: u16,
    },
    Capability(CapabilityError),
    InvalidHostLimit {
        field: &'static str,
        requested: u64,
        minimum: u64,
        maximum: u64,
    },
    InvalidEnvelope,
    Disposed,
    TerminalPublished,
    SubscriptionLimit {
        maximum: u16,
    },
    InvalidSubscriptionCapacity {
        requested: u32,
        maximum: u32,
    },
    EventPayloadTooLarge {
        bytes: usize,
        maximum: u32,
    },
    EventSequenceExhausted,
    SubscriptionClosed(EventSubscriptionId),
    AmbiguousCancelTarget,
    InvalidSourceSelector,
    SourceSelectorNotFound,
    SourceSelectorAmbiguous {
        matches: usize,
    },
    Control(ControlError),
    InvalidRetainedLabel,
    RetainedHandleLimit {
        maximum: u32,
    },
    UnknownRetainedHandle(RetainedHandleId),
    ForeignRetainedHandle(OperationId),
    RetainedHandleKindMismatch,
    RetainedHandleTypeMismatch,
    ForeignTerminalCause(OperationId),
    TerminalControlConflict(ControlFailure),
    InvalidTerminalReason,
    Serialization(serde_json::Error),
}

impl OperationHandleError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ProtocolVersion { .. } => "cem.operation.protocol_version",
            Self::Capability(error) => error.code,
            Self::InvalidHostLimit { .. } => "cem.operation.host_limit_invalid",
            Self::InvalidEnvelope => "cem.operation.envelope_invalid",
            Self::Disposed => "cem.operation.disposed",
            Self::TerminalPublished => "cem.operation.terminal_published",
            Self::SubscriptionLimit { .. } => "cem.operation.subscription_limit",
            Self::InvalidSubscriptionCapacity { .. } => {
                "cem.operation.subscription_capacity_invalid"
            }
            Self::EventPayloadTooLarge { .. } => "cem.operation.event_payload_too_large",
            Self::EventSequenceExhausted => "cem.operation.event_sequence_exhausted",
            Self::SubscriptionClosed(_) => "cem.operation.subscription_closed",
            Self::AmbiguousCancelTarget => "cem.operation.cancel_target_ambiguous",
            Self::InvalidSourceSelector => "cem.operation.source_selector_invalid",
            Self::SourceSelectorNotFound => "cem.operation.source_selector_not_found",
            Self::SourceSelectorAmbiguous { .. } => "cem.operation.source_selector_ambiguous",
            Self::Control(error) => error.code(),
            Self::InvalidRetainedLabel => "cem.operation.retained_label_invalid",
            Self::RetainedHandleLimit { .. } => "cem.operation.retained_handle_limit",
            Self::UnknownRetainedHandle(_) => "cem.operation.retained_handle_unknown",
            Self::ForeignRetainedHandle(_) => "cem.operation.retained_handle_foreign",
            Self::RetainedHandleKindMismatch => "cem.operation.retained_handle_kind_mismatch",
            Self::RetainedHandleTypeMismatch => "cem.operation.retained_handle_type_mismatch",
            Self::ForeignTerminalCause(_) => "cem.operation.terminal_cause_foreign",
            Self::TerminalControlConflict(_) => "cem.operation.terminal_control_conflict",
            Self::InvalidTerminalReason => "cem.operation.terminal_reason_invalid",
            Self::Serialization(_) => "cem.operation.serialization_failed",
        }
    }
}

impl fmt::Display for OperationHandleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtocolVersion {
                requested,
                supported,
            } => write!(
                formatter,
                "operation protocol version {requested} is unsupported; expected {supported}"
            ),
            Self::Capability(error) => error.fmt(formatter),
            Self::InvalidHostLimit {
                field,
                requested,
                minimum,
                maximum,
            } => write!(
                formatter,
                "operation host limit {field}={requested} is outside {minimum}..={maximum}"
            ),
            Self::InvalidEnvelope => write!(formatter, "initialize cannot carry an operation id"),
            Self::Disposed => write!(formatter, "operation handle is disposed"),
            Self::TerminalPublished => {
                write!(formatter, "operation already published a terminal result")
            }
            Self::SubscriptionLimit { maximum } => {
                write!(
                    formatter,
                    "operation allows at most {maximum} live subscriptions"
                )
            }
            Self::InvalidSubscriptionCapacity { requested, maximum } => write!(
                formatter,
                "subscription capacity {requested} is outside 1..={maximum}"
            ),
            Self::EventPayloadTooLarge { bytes, maximum } => write!(
                formatter,
                "event payload is {bytes} bytes, exceeding the {maximum}-byte limit"
            ),
            Self::EventSequenceExhausted => write!(formatter, "operation event sequence exhausted"),
            Self::SubscriptionClosed(id) => write!(formatter, "event subscription {id} is closed"),
            Self::AmbiguousCancelTarget => write!(
                formatter,
                "cancel request cannot combine execution-scope and source-selector targets"
            ),
            Self::InvalidSourceSelector => write!(formatter, "source selector is invalid"),
            Self::SourceSelectorNotFound => {
                write!(formatter, "source selector matched no live scope")
            }
            Self::SourceSelectorAmbiguous { matches } => {
                write!(formatter, "source selector matched {matches} live scopes")
            }
            Self::Control(error) => error.fmt(formatter),
            Self::InvalidRetainedLabel => write!(formatter, "retained handle label is invalid"),
            Self::RetainedHandleLimit { maximum } => {
                write!(
                    formatter,
                    "operation allows at most {maximum} retained handles"
                )
            }
            Self::UnknownRetainedHandle(id) => write!(formatter, "unknown retained handle {id}"),
            Self::ForeignRetainedHandle(operation) => {
                write!(
                    formatter,
                    "retained handle belongs to operation {operation}"
                )
            }
            Self::RetainedHandleKindMismatch => write!(formatter, "retained handle kind mismatch"),
            Self::RetainedHandleTypeMismatch => write!(formatter, "retained handle type mismatch"),
            Self::ForeignTerminalCause(operation) => {
                write!(formatter, "terminal cause belongs to operation {operation}")
            }
            Self::TerminalControlConflict(failure) => write!(
                formatter,
                "successful terminal result conflicts with active root control failure: {failure}"
            ),
            Self::InvalidTerminalReason => {
                write!(formatter, "terminal cancellation reason is invalid")
            }
            Self::Serialization(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for OperationHandleError {}

impl From<ControlError> for OperationHandleError {
    fn from(error: ControlError) -> Self {
        Self::Control(error)
    }
}

struct RetainedEntry {
    metadata: RetainedHandleMetadata,
    value: Arc<dyn Any + Send + Sync>,
}

struct SubscriptionCursor {
    next_sequence: u64,
    capacity: u32,
    filters: BTreeSet<OperationEventKind>,
    waker: Option<Waker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CriticalEventKey {
    Breakpoint(u64),
    Stopped,
    Continued,
    Terminal,
}

struct OperationState<R> {
    disposed: bool,
    next_sequence: u64,
    events: VecDeque<Arc<OperationEvent>>,
    critical_events: BTreeMap<CriticalEventKey, Arc<OperationEvent>>,
    next_subscription_id: u64,
    subscriptions: BTreeMap<EventSubscriptionId, SubscriptionCursor>,
    terminal: Option<Arc<OperationOutcome<R>>>,
    result_wakers: Vec<Waker>,
    next_retained_handle_id: u64,
    retained: BTreeMap<RetainedHandleId, RetainedEntry>,
}

impl<R> Default for OperationState<R> {
    fn default() -> Self {
        Self {
            disposed: false,
            next_sequence: 1,
            events: VecDeque::new(),
            critical_events: BTreeMap::new(),
            next_subscription_id: 1,
            subscriptions: BTreeMap::new(),
            terminal: None,
            result_wakers: Vec::new(),
            next_retained_handle_id: 1,
            retained: BTreeMap::new(),
        }
    }
}

struct OperationCore<R> {
    operation_id: OperationId,
    control: OperationControl,
    limits: OperationHostLimits,
    state: Mutex<OperationState<R>>,
    changed: Condvar,
}

#[cfg(feature = "debug-control")]
struct OperationDebugObserver<R> {
    core: Weak<OperationCore<R>>,
}

#[cfg(feature = "debug-control")]
impl<R: Send + Sync + 'static> DebugRuntimeObserver for OperationDebugObserver<R> {
    fn event(&self, event: DebugRuntimeEvent) {
        let Some(core) = self.core.upgrade() else {
            return;
        };
        let (kind, payload) = match event {
            DebugRuntimeEvent::BreakpointResolved(payload) => (
                OperationEventKind::BreakpointResolved,
                serde_json::to_value(payload),
            ),
            DebugRuntimeEvent::PauseRequested(payload) => (
                OperationEventKind::PauseRequested,
                serde_json::to_value(payload),
            ),
            DebugRuntimeEvent::Stopped(payload) => {
                (OperationEventKind::Stopped, serde_json::to_value(payload))
            }
            DebugRuntimeEvent::Continued(payload) => {
                (OperationEventKind::Continued, serde_json::to_value(payload))
            }
        };
        let Ok(payload) = payload else {
            return;
        };
        if validate_payload(&payload, core.limits).is_err() {
            return;
        }
        let mut state = core.state.lock().expect("poisoned operation-handle mutex");
        if state.disposed || state.terminal.is_some() {
            return;
        }
        let Ok((_, wakers)) =
            append_event(&mut state, core.operation_id, core.limits, kind, payload)
        else {
            return;
        };
        drop(state);
        core.changed.notify_all();
        wake_all(wakers);
    }

    fn breakpoint_removed(&self, breakpoint: BreakpointId) {
        let Some(core) = self.core.upgrade() else {
            return;
        };
        core.state
            .lock()
            .expect("poisoned operation-handle mutex")
            .critical_events
            .remove(&CriticalEventKey::Breakpoint(breakpoint.get()));
    }
}

/// Cloneable read/control side of one operation. Completion ownership is held
/// separately by [`OperationTerminalPublisher`].
pub struct OperationHandle<R> {
    core: Arc<OperationCore<R>>,
}

impl<R> Clone for OperationHandle<R> {
    fn clone(&self) -> Self {
        Self {
            core: Arc::clone(&self.core),
        }
    }
}

impl<R> fmt::Debug for OperationHandle<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationHandle")
            .field("operation_id", &self.core.operation_id)
            .field("limits", &self.core.limits)
            .finish_non_exhaustive()
    }
}

/// Cloneable terminal claim token. The shared core accepts exactly one claim;
/// losing completion/cancellation/failure racers observe the retained winner.
pub struct OperationTerminalPublisher<R> {
    core: Arc<OperationCore<R>>,
}

impl<R> Clone for OperationTerminalPublisher<R> {
    fn clone(&self) -> Self {
        Self {
            core: Arc::clone(&self.core),
        }
    }
}

impl<R> fmt::Debug for OperationTerminalPublisher<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationTerminalPublisher")
            .field("operation_id", &self.core.operation_id)
            .finish_non_exhaustive()
    }
}

impl<R: Send + Sync + 'static> OperationHandle<R> {
    pub fn new(
        control: OperationControl,
        limits: OperationHostLimits,
    ) -> Result<(Self, OperationTerminalPublisher<R>), OperationHandleError> {
        validate_limits(limits)?;
        let operation_id = control.operation_id();
        let core = Arc::new(OperationCore {
            operation_id,
            control,
            limits,
            state: Mutex::new(OperationState::default()),
            changed: Condvar::new(),
        });
        #[cfg(feature = "debug-control")]
        core.control
            .attach_debug_observer(Arc::new(OperationDebugObserver::<R> {
                core: Arc::downgrade(&core),
            }));
        let handle = Self {
            core: Arc::clone(&core),
        };
        handle.publish_event(
            OperationEventKind::Accepted,
            &serde_json::json!({ "protocolVersion": OPERATION_PROTOCOL_VERSION }),
        )?;
        Ok((handle, OperationTerminalPublisher { core }))
    }

    pub fn with_defaults(
        control: OperationControl,
    ) -> Result<(Self, OperationTerminalPublisher<R>), OperationHandleError> {
        Self::new(control, OperationHostLimits::default())
    }

    pub fn operation_id(&self) -> OperationId {
        self.core.operation_id
    }

    pub fn limits(&self) -> OperationHostLimits {
        self.core.limits
    }

    #[cfg(feature = "debug-control")]
    pub fn activate_debug_control(
        &self,
        condition_evaluator: Option<Arc<dyn DebugConditionEvaluator>>,
    ) -> Result<(), DebugControlError> {
        self.core
            .control
            .activate_debug_control(self.core.limits, condition_evaluator)
    }

    #[cfg(feature = "debug-control")]
    pub fn debug_control_active(&self) -> bool {
        self.core.control.debug_control_active()
    }

    #[cfg(feature = "debug-control")]
    pub fn register_debug_safe_point(
        &self,
        scope: ExecutionScopeId,
        kind: DebugSafePointKind,
        phase: impl Into<String>,
        location: Option<crate::operation_control::SourceLocation>,
    ) -> Result<ExecutableSafePoint, DebugControlError> {
        self.core
            .control
            .register_debug_safe_point(scope, kind, phase, location)
    }

    #[cfg(feature = "debug-control")]
    pub fn debug_executable_locations(
        &self,
        source_uri: &str,
        start_line: Option<u32>,
        end_line: Option<u32>,
    ) -> Result<Vec<ExecutableSafePoint>, DebugControlError> {
        self.core
            .control
            .debug_executable_locations(source_uri, start_line, end_line)
    }

    #[cfg(feature = "debug-control")]
    pub fn remove_pause_trigger(&self, breakpoint: BreakpointId) -> Result<(), DebugControlError> {
        self.core.control.remove_pause_trigger(breakpoint)
    }

    #[cfg(feature = "debug-control")]
    pub fn pause(&self, spec: PauseSpec) -> Result<PauseTriggerHandle, DebugControlError> {
        self.core.control.install_pause_trigger(spec)
    }

    #[cfg(feature = "debug-control")]
    pub fn resume(
        &self,
        stop: StopToken,
    ) -> Result<crate::debug_control::ContinuedEvent, DebugControlError> {
        self.core.control.resume_debug(stop)
    }

    #[cfg(feature = "debug-control")]
    pub fn step(
        &self,
        request: StepRequest,
    ) -> Result<crate::debug_control::ContinuedEvent, DebugControlError> {
        self.core.control.step_debug(request)
    }

    #[cfg(feature = "debug-control")]
    pub fn suspended_snapshot(
        &self,
        stop: StopToken,
    ) -> Result<Arc<SuspendedSnapshot>, DebugControlError> {
        self.core.control.suspended_snapshot(stop)
    }

    #[cfg(feature = "debug-control")]
    pub fn debug_stack_trace(
        &self,
        stop: StopToken,
        task: TaskId,
        start: u32,
        count: Option<u32>,
    ) -> Result<DebugPage<LogicalFrameSnapshot>, DebugControlError> {
        self.core
            .control
            .debug_stack_trace(stop, task, start, count)
    }

    #[cfg(feature = "debug-control")]
    pub fn debug_frame_scopes(
        &self,
        stop: StopToken,
        frame: SnapshotReferenceId,
    ) -> Result<Vec<VariableScopeSnapshot>, DebugControlError> {
        self.core.control.debug_frame_scopes(stop, frame)
    }

    #[cfg(feature = "debug-control")]
    pub fn debug_variables(
        &self,
        stop: StopToken,
        reference: SnapshotReferenceId,
        filter: VariableFilter,
        start: u32,
        count: Option<u32>,
    ) -> Result<DebugPage<VariableSnapshot>, DebugControlError> {
        self.core
            .control
            .debug_variables(stop, reference, filter, start, count)
    }

    #[cfg(feature = "debug-control")]
    pub fn debug_native_value<T: Any + Send + Sync>(
        &self,
        stop: StopToken,
        reference: SnapshotReferenceId,
    ) -> Result<Arc<T>, DebugControlError> {
        self.core.control.debug_native_value(stop, reference)
    }

    #[cfg(feature = "debug-control")]
    pub fn debug_native_projection(
        &self,
        stop: StopToken,
        reference: SnapshotReferenceId,
    ) -> Result<Value, DebugControlError> {
        self.core.control.debug_native_projection(stop, reference)
    }

    pub fn execution_scope_tree(&self) -> ExecutionScopeTree {
        self.core.control.scope_tree()
    }

    pub fn terminal_summary(&self) -> Option<OperationTerminalSummary> {
        self.core
            .state
            .lock()
            .expect("poisoned operation-handle mutex")
            .terminal
            .as_ref()
            .map(|outcome| outcome.summary())
    }

    pub fn result(&self) -> OperationResultFuture<R> {
        OperationResultFuture {
            core: Arc::clone(&self.core),
        }
    }

    pub fn blocking_result_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<Arc<OperationOutcome<R>>>, OperationHandleError> {
        let deadline = Instant::now() + timeout;
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        loop {
            if let Some(outcome) = &state.terminal {
                return Ok(Some(Arc::clone(outcome)));
            }
            if state.disposed {
                return Err(OperationHandleError::Disposed);
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let (next, result) = self
                .core
                .changed
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .expect("poisoned operation-handle mutex");
            state = next;
            if result.timed_out() && state.terminal.is_none() {
                return Ok(None);
            }
        }
    }

    pub fn publish_event<T: Serialize>(
        &self,
        kind: OperationEventKind,
        payload: &T,
    ) -> Result<Arc<OperationEvent>, OperationHandleError> {
        if matches!(
            kind,
            OperationEventKind::SubscriptionGap | OperationEventKind::Terminal
        ) {
            return Err(OperationHandleError::InvalidEnvelope);
        }
        let payload = serde_json::to_value(payload).map_err(OperationHandleError::Serialization)?;
        validate_payload(&payload, self.core.limits)?;
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        if state.disposed {
            return Err(OperationHandleError::Disposed);
        }
        if state.terminal.is_some() {
            return Err(OperationHandleError::TerminalPublished);
        }
        let (event, wakers) = append_event(
            &mut state,
            self.core.operation_id,
            self.core.limits,
            kind,
            payload,
        )?;
        drop(state);
        self.core.changed.notify_all();
        wake_all(wakers);
        Ok(event)
    }

    pub fn subscribe(
        &self,
        options: EventSubscriptionOptions,
    ) -> Result<OperationEventSubscription<R>, OperationHandleError> {
        let capacity = options
            .capacity
            .unwrap_or(self.core.limits.default_subscription_capacity);
        if capacity == 0 || capacity > self.core.limits.max_subscription_capacity {
            return Err(OperationHandleError::InvalidSubscriptionCapacity {
                requested: capacity,
                maximum: self.core.limits.max_subscription_capacity,
            });
        }
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        if state.disposed {
            return Err(OperationHandleError::Disposed);
        }
        if state.subscriptions.len() >= self.core.limits.max_live_subscriptions as usize {
            return Err(OperationHandleError::SubscriptionLimit {
                maximum: self.core.limits.max_live_subscriptions,
            });
        }
        let id = EventSubscriptionId::from_raw(state.next_subscription_id);
        state.next_subscription_id = state
            .next_subscription_id
            .checked_add(1)
            .ok_or(OperationHandleError::EventSequenceExhausted)?;
        state.subscriptions.insert(
            id,
            SubscriptionCursor {
                next_sequence: options.from_sequence.unwrap_or(1).max(1),
                capacity,
                filters: options.filters,
                waker: None,
            },
        );
        Ok(OperationEventSubscription {
            id,
            core: Arc::clone(&self.core),
            closed: false,
        })
    }

    pub fn cancel(&self, request: CancelRequest) -> Result<ControlAck, OperationHandleError> {
        if request.scope.is_some() && request.source_selector.is_some() {
            return Err(OperationHandleError::AmbiguousCancelTarget);
        }
        let selected_scope = if let Some(scope) = request.scope {
            scope
        } else if let Some(selector) = &request.source_selector {
            resolve_source_selector(&self.core.control, selector)?
        } else {
            self.core.control.root_scope()
        };
        let reason = request.reason;
        let failure = ControlFailure {
            operation_id: self.core.operation_id,
            affected_scope: selected_scope,
            cause: ControlCause::HostCancellation {
                reason: reason.clone(),
            },
            source_map: None,
        };
        let failure_payload =
            serde_json::to_value(&failure).map_err(OperationHandleError::Serialization)?;
        validate_payload(&failure_payload, self.core.limits)?;
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        if state.disposed {
            return Err(OperationHandleError::Disposed);
        }
        if state.terminal.is_some() {
            return Err(OperationHandleError::TerminalPublished);
        }
        let outcome = self
            .core
            .control
            .cancel_scope(selected_scope, reason, None)?;
        let mut wakers = Vec::new();
        if outcome == ControlRequestOutcome::Accepted {
            let (_, event_wakers) = append_event(
                &mut state,
                self.core.operation_id,
                self.core.limits,
                OperationEventKind::ControlFailure,
                failure_payload,
            )?;
            wakers = event_wakers;
            state.critical_events.remove(&CriticalEventKey::Stopped);
        }
        let acknowledgement = ControlAck {
            operation_id: self.core.operation_id,
            selected_scope,
            disposition: outcome.into(),
        };
        drop(state);
        self.core.changed.notify_all();
        wake_all(wakers);
        Ok(acknowledgement)
    }

    pub fn retain_value<T: Any + Send + Sync>(
        &self,
        label: impl Into<String>,
        value: T,
    ) -> Result<RetainedHandleMetadata, OperationHandleError> {
        self.retain(
            RetainedHandleKind::NativeValue,
            label.into(),
            Arc::new(value),
        )
    }

    pub fn retain_artifact<T: Any + Send + Sync>(
        &self,
        label: impl Into<String>,
        value: T,
    ) -> Result<RetainedHandleMetadata, OperationHandleError> {
        self.retain(RetainedHandleKind::Artifact, label.into(), Arc::new(value))
    }

    pub fn resolve_retained<T: Any + Send + Sync>(
        &self,
        metadata: &RetainedHandleMetadata,
        expected_kind: RetainedHandleKind,
    ) -> Result<Arc<T>, OperationHandleError> {
        if metadata.operation_id != self.core.operation_id {
            return Err(OperationHandleError::ForeignRetainedHandle(
                metadata.operation_id,
            ));
        }
        if metadata.kind != expected_kind {
            return Err(OperationHandleError::RetainedHandleKindMismatch);
        }
        let state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        if state.disposed {
            return Err(OperationHandleError::Disposed);
        }
        let entry = state.retained.get(&metadata.handle_id).ok_or(
            OperationHandleError::UnknownRetainedHandle(metadata.handle_id),
        )?;
        if entry.metadata.kind != expected_kind {
            return Err(OperationHandleError::RetainedHandleKindMismatch);
        }
        Arc::clone(&entry.value)
            .downcast::<T>()
            .map_err(|_| OperationHandleError::RetainedHandleTypeMismatch)
    }

    pub fn dispose(&self) {
        let (result_wakers, subscription_wakers) = {
            let mut state = self
                .core
                .state
                .lock()
                .expect("poisoned operation-handle mutex");
            if state.disposed {
                return;
            }
            state.disposed = true;
            state.events.clear();
            state.critical_events.clear();
            state.retained.clear();
            state.terminal = None;
            let result_wakers = std::mem::take(&mut state.result_wakers);
            let subscription_wakers = state
                .subscriptions
                .values_mut()
                .filter_map(|cursor| cursor.waker.take())
                .collect();
            state.subscriptions.clear();
            (result_wakers, subscription_wakers)
        };
        self.core.changed.notify_all();
        wake_all(result_wakers);
        wake_all(subscription_wakers);
        #[cfg(feature = "debug-control")]
        self.core.control.complete_debug_control();
    }

    fn retain(
        &self,
        kind: RetainedHandleKind,
        label: String,
        value: Arc<dyn Any + Send + Sync>,
    ) -> Result<RetainedHandleMetadata, OperationHandleError> {
        if label.is_empty()
            || label.len() > MAX_IDENTITY_BYTES
            || label.chars().any(char::is_control)
        {
            return Err(OperationHandleError::InvalidRetainedLabel);
        }
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        if state.disposed {
            return Err(OperationHandleError::Disposed);
        }
        if state.retained.len() >= self.core.limits.max_retained_handles as usize {
            return Err(OperationHandleError::RetainedHandleLimit {
                maximum: self.core.limits.max_retained_handles,
            });
        }
        let handle_id = RetainedHandleId::from_raw(state.next_retained_handle_id);
        state.next_retained_handle_id = state
            .next_retained_handle_id
            .checked_add(1)
            .ok_or(OperationHandleError::EventSequenceExhausted)?;
        let metadata = RetainedHandleMetadata {
            operation_id: self.core.operation_id,
            handle_id,
            kind,
            label,
        };
        state.retained.insert(
            handle_id,
            RetainedEntry {
                metadata: metadata.clone(),
                value,
            },
        );
        Ok(metadata)
    }
}

impl<R: Send + Sync + 'static> OperationTerminalPublisher<R> {
    pub fn settle(
        &self,
        outcome: OperationOutcome<R>,
    ) -> Result<TerminalClaim<R>, OperationHandleError> {
        validate_outcome(self.core.operation_id, &outcome)?;
        let outcome = Arc::new(outcome.enforce(self.core.limits));
        let summary = outcome.summary();
        let payload = serde_json::to_value(summary).map_err(OperationHandleError::Serialization)?;
        validate_payload(&payload, self.core.limits)?;
        let (claim, result_wakers, subscription_wakers) = {
            let mut state = self
                .core
                .state
                .lock()
                .expect("poisoned operation-handle mutex");
            if state.disposed {
                return Err(OperationHandleError::Disposed);
            }
            if let Some(winner) = &state.terminal {
                return Ok(TerminalClaim::AlreadyPublished(Arc::clone(winner)));
            }
            validate_retained_artifacts(&state, outcome.as_ref())?;
            if matches!(outcome.as_ref(), OperationOutcome::Succeeded { .. }) {
                if let Err(ControlError::Triggered(failure)) = self
                    .core
                    .control
                    .check_scope(self.core.control.root_scope())
                {
                    return Err(OperationHandleError::TerminalControlConflict(failure));
                }
            }
            let (_, subscription_wakers) = append_event(
                &mut state,
                self.core.operation_id,
                self.core.limits,
                OperationEventKind::Terminal,
                payload,
            )?;
            state.terminal = Some(Arc::clone(&outcome));
            let result_wakers = std::mem::take(&mut state.result_wakers);
            (
                TerminalClaim::Published(outcome),
                result_wakers,
                subscription_wakers,
            )
        };
        self.core.changed.notify_all();
        wake_all(result_wakers);
        wake_all(subscription_wakers);
        #[cfg(feature = "debug-control")]
        self.core.control.complete_debug_control();
        Ok(claim)
    }
}

pub struct OperationResultFuture<R> {
    core: Arc<OperationCore<R>>,
}

impl<R> fmt::Debug for OperationResultFuture<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationResultFuture")
            .field("operation_id", &self.core.operation_id)
            .finish_non_exhaustive()
    }
}

impl<R> Future for OperationResultFuture<R> {
    type Output = Result<Arc<OperationOutcome<R>>, OperationHandleError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        if let Some(outcome) = &state.terminal {
            return Poll::Ready(Ok(Arc::clone(outcome)));
        }
        if state.disposed {
            return Poll::Ready(Err(OperationHandleError::Disposed));
        }
        if !state
            .result_wakers
            .iter()
            .any(|waker| waker.will_wake(context.waker()))
        {
            state.result_wakers.push(context.waker().clone());
        }
        Poll::Pending
    }
}

pub struct OperationEventSubscription<R> {
    id: EventSubscriptionId,
    core: Arc<OperationCore<R>>,
    closed: bool,
}

impl<R> fmt::Debug for OperationEventSubscription<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationEventSubscription")
            .field("id", &self.id)
            .field("operation_id", &self.core.operation_id)
            .field("closed", &self.closed)
            .finish()
    }
}

impl<R> OperationEventSubscription<R> {
    pub fn id(&self) -> EventSubscriptionId {
        self.id
    }

    pub fn next_event(&mut self) -> OperationEventFuture<'_, R> {
        OperationEventFuture { subscription: self }
    }

    pub fn try_next(
        &mut self,
    ) -> Result<(EventSubscriptionPoll, Option<Arc<OperationEvent>>), OperationHandleError> {
        if self.closed {
            return Ok((EventSubscriptionPoll::Closed, None));
        }
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        let result = next_subscription_event(&mut state, self.core.operation_id, self.id)?;
        if result.0 == EventSubscriptionPoll::Closed {
            self.closed = true;
        }
        Ok(result)
    }

    pub fn blocking_next_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Arc<OperationEvent>>, OperationHandleError> {
        let deadline = Instant::now() + timeout;
        let mut state = self
            .core
            .state
            .lock()
            .expect("poisoned operation-handle mutex");
        loop {
            let (status, event) =
                next_subscription_event(&mut state, self.core.operation_id, self.id)?;
            match status {
                EventSubscriptionPoll::Event => return Ok(event),
                EventSubscriptionPoll::Closed => {
                    self.closed = true;
                    return Ok(None);
                }
                EventSubscriptionPoll::Pending => {}
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let (next, result) = self
                .core
                .changed
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .expect("poisoned operation-handle mutex");
            state = next;
            if result.timed_out() {
                return Ok(None);
            }
        }
    }

    pub fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        self.core
            .state
            .lock()
            .expect("poisoned operation-handle mutex")
            .subscriptions
            .remove(&self.id);
    }
}

impl<R> Drop for OperationEventSubscription<R> {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct OperationEventFuture<'a, R> {
    subscription: &'a mut OperationEventSubscription<R>,
}

impl<R> Future for OperationEventFuture<'_, R> {
    type Output = Result<Option<Arc<OperationEvent>>, OperationHandleError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.subscription.closed {
            return Poll::Ready(Ok(None));
        }
        let core = Arc::clone(&self.subscription.core);
        let id = self.subscription.id;
        let mut state = core.state.lock().expect("poisoned operation-handle mutex");
        match next_subscription_event(&mut state, core.operation_id, id) {
            Ok((EventSubscriptionPoll::Event, event)) => Poll::Ready(Ok(event)),
            Ok((EventSubscriptionPoll::Closed, _)) => {
                drop(state);
                self.subscription.closed = true;
                Poll::Ready(Ok(None))
            }
            Ok((EventSubscriptionPoll::Pending, _)) => {
                let cursor = state
                    .subscriptions
                    .get_mut(&id)
                    .expect("pending subscription remains registered");
                if cursor
                    .waker
                    .as_ref()
                    .is_none_or(|waker| !waker.will_wake(context.waker()))
                {
                    cursor.waker = Some(context.waker().clone());
                }
                Poll::Pending
            }
            Err(error) => Poll::Ready(Err(error)),
        }
    }
}

fn validate_limits(limits: OperationHostLimits) -> Result<(), OperationHandleError> {
    validate_limit(
        "maxLiveSubscriptions",
        u64::from(limits.max_live_subscriptions),
        u64::from(DEFAULT_MAX_LIVE_SUBSCRIPTIONS),
    )?;
    validate_limit(
        "maxSubscriptionCapacity",
        u64::from(limits.max_subscription_capacity),
        u64::from(MAX_SUBSCRIPTION_CAPACITY),
    )?;
    validate_limit(
        "defaultSubscriptionCapacity",
        u64::from(limits.default_subscription_capacity),
        u64::from(
            limits
                .max_subscription_capacity
                .min(DEFAULT_SUBSCRIPTION_CAPACITY),
        ),
    )?;
    validate_limit(
        "maxInlineEventPayloadBytes",
        u64::from(limits.max_inline_event_payload_bytes),
        u64::from(MAX_INLINE_EVENT_PAYLOAD_BYTES),
    )?;
    validate_limit(
        "maxTerminalDiagnostics",
        u64::from(limits.max_terminal_diagnostics),
        u64::from(MAX_TERMINAL_DIAGNOSTICS),
    )?;
    validate_limit(
        "maxRecoveredControlFailures",
        u64::from(limits.max_recovered_control_failures),
        u64::from(MAX_RECOVERED_CONTROL_FAILURES),
    )?;
    validate_limit(
        "maxArtifactReferences",
        u64::from(limits.max_artifact_references),
        u64::from(MAX_ARTIFACT_REFERENCES),
    )?;
    validate_limit(
        "maxRetainedHandles",
        u64::from(limits.max_retained_handles),
        u64::from(MAX_RETAINED_HANDLES),
    )?;
    validate_limit(
        "maxStackFramePageSize",
        u64::from(limits.max_stack_frame_page_size),
        u64::from(MAX_STACK_FRAME_PAGE_SIZE),
    )?;
    validate_limit(
        "defaultStackFramePageSize",
        u64::from(limits.default_stack_frame_page_size),
        u64::from(
            limits
                .max_stack_frame_page_size
                .min(DEFAULT_STACK_FRAME_PAGE_SIZE),
        ),
    )?;
    validate_limit(
        "maxVariablePageSize",
        u64::from(limits.max_variable_page_size),
        u64::from(MAX_VARIABLE_PAGE_SIZE),
    )?;
    validate_limit(
        "defaultVariablePageSize",
        u64::from(limits.default_variable_page_size),
        u64::from(
            limits
                .max_variable_page_size
                .min(DEFAULT_VARIABLE_PAGE_SIZE),
        ),
    )?;
    validate_limit(
        "maxDebugValuePreviewBytes",
        u64::from(limits.max_debug_value_preview_bytes),
        u64::from(MAX_DEBUG_VALUE_PREVIEW_BYTES),
    )?;
    validate_limit(
        "maxSuspendedSnapshotBytes",
        limits.max_suspended_snapshot_bytes,
        MAX_SUSPENDED_SNAPSHOT_BYTES,
    )?;
    validate_limit(
        "maxDebugBreakpoints",
        u64::from(limits.max_debug_breakpoints),
        u64::from(MAX_DEBUG_BREAKPOINTS),
    )?;
    Ok(())
}

fn validate_limit(
    field: &'static str,
    requested: u64,
    maximum: u64,
) -> Result<(), OperationHandleError> {
    const MINIMUM: u64 = 1;
    if !(MINIMUM..=maximum).contains(&requested) {
        return Err(OperationHandleError::InvalidHostLimit {
            field,
            requested,
            minimum: MINIMUM,
            maximum,
        });
    }
    Ok(())
}

fn validate_payload(
    payload: &Value,
    limits: OperationHostLimits,
) -> Result<(), OperationHandleError> {
    let bytes = serde_json::to_vec(payload)
        .map_err(OperationHandleError::Serialization)?
        .len();
    if bytes > limits.max_inline_event_payload_bytes as usize {
        return Err(OperationHandleError::EventPayloadTooLarge {
            bytes,
            maximum: limits.max_inline_event_payload_bytes,
        });
    }
    Ok(())
}

fn append_event<R>(
    state: &mut OperationState<R>,
    operation_id: OperationId,
    limits: OperationHostLimits,
    kind: OperationEventKind,
    payload: Value,
) -> Result<(Arc<OperationEvent>, Vec<Waker>), OperationHandleError> {
    let sequence = state.next_sequence;
    state.next_sequence = state
        .next_sequence
        .checked_add(1)
        .ok_or(OperationHandleError::EventSequenceExhausted)?;
    let event = Arc::new(OperationEvent {
        protocol_version: OPERATION_PROTOCOL_VERSION,
        operation_id,
        sequence,
        kind,
        payload,
    });
    if kind == OperationEventKind::Continued {
        state.critical_events.remove(&CriticalEventKey::Stopped);
    }
    if let Some(key) = critical_event_key(kind, &event.payload) {
        state.critical_events.insert(key, Arc::clone(&event));
    }
    state.events.push_back(Arc::clone(&event));
    while state.events.len() > limits.max_subscription_capacity as usize {
        state.events.pop_front();
    }
    let wakers = state
        .subscriptions
        .values_mut()
        .filter_map(|cursor| cursor.waker.take())
        .collect();
    Ok((event, wakers))
}

fn critical_event_key(kind: OperationEventKind, payload: &Value) -> Option<CriticalEventKey> {
    match kind {
        OperationEventKind::BreakpointResolved => payload
            .get("breakpointId")
            .and_then(Value::as_u64)
            .map(CriticalEventKey::Breakpoint),
        OperationEventKind::Stopped => Some(CriticalEventKey::Stopped),
        OperationEventKind::Continued => Some(CriticalEventKey::Continued),
        OperationEventKind::Terminal => Some(CriticalEventKey::Terminal),
        _ => None,
    }
}

fn next_subscription_event<R>(
    state: &mut OperationState<R>,
    operation_id: OperationId,
    subscription_id: EventSubscriptionId,
) -> Result<(EventSubscriptionPoll, Option<Arc<OperationEvent>>), OperationHandleError> {
    loop {
        let (cursor, capacity, filters) = match state.subscriptions.get(&subscription_id) {
            Some(subscription) => (
                subscription.next_sequence,
                subscription.capacity,
                subscription.filters.clone(),
            ),
            None if state.disposed => return Ok((EventSubscriptionPoll::Closed, None)),
            None => return Err(OperationHandleError::SubscriptionClosed(subscription_id)),
        };
        if state.disposed {
            state.subscriptions.remove(&subscription_id);
            return Ok((EventSubscriptionPoll::Closed, None));
        }

        let latest = state.next_sequence.saturating_sub(1);
        let allowed_regular = latest
            .saturating_sub(capacity.saturating_sub(1) as u64)
            .max(1);
        let regular = state
            .events
            .iter()
            .filter(|event| event.sequence >= cursor && event.sequence >= allowed_regular)
            .min_by_key(|event| event.sequence)
            .cloned();
        let critical = state
            .critical_events
            .values()
            .filter(|event| event.sequence >= cursor)
            .min_by_key(|event| event.sequence)
            .cloned();
        let candidate = match (regular, critical) {
            (Some(left), Some(right)) if left.sequence <= right.sequence => Some(left),
            (Some(_), Some(right)) => Some(right),
            (Some(event), None) | (None, Some(event)) => Some(event),
            (None, None) => None,
        };

        if let Some(event) = candidate {
            if event.sequence > cursor {
                let last_missing = event.sequence - 1;
                state
                    .subscriptions
                    .get_mut(&subscription_id)
                    .expect("subscription remains live")
                    .next_sequence = event.sequence;
                let payload = serde_json::to_value(SubscriptionGap {
                    first_missing: cursor,
                    last_missing,
                })
                .expect("subscription gap is serializable");
                return Ok((
                    EventSubscriptionPoll::Event,
                    Some(Arc::new(OperationEvent {
                        protocol_version: OPERATION_PROTOCOL_VERSION,
                        operation_id,
                        sequence: last_missing,
                        kind: OperationEventKind::SubscriptionGap,
                        payload,
                    })),
                ));
            }

            state
                .subscriptions
                .get_mut(&subscription_id)
                .expect("subscription remains live")
                .next_sequence = event.sequence.saturating_add(1);
            if filters.is_empty() || filters.contains(&event.kind) {
                return Ok((EventSubscriptionPoll::Event, Some(event)));
            }
            continue;
        }

        if cursor <= latest {
            state
                .subscriptions
                .get_mut(&subscription_id)
                .expect("subscription remains live")
                .next_sequence = latest.saturating_add(1);
            let payload = serde_json::to_value(SubscriptionGap {
                first_missing: cursor,
                last_missing: latest,
            })
            .expect("subscription gap is serializable");
            return Ok((
                EventSubscriptionPoll::Event,
                Some(Arc::new(OperationEvent {
                    protocol_version: OPERATION_PROTOCOL_VERSION,
                    operation_id,
                    sequence: latest,
                    kind: OperationEventKind::SubscriptionGap,
                    payload,
                })),
            ));
        }
        if state.terminal.is_some() {
            state.subscriptions.remove(&subscription_id);
            return Ok((EventSubscriptionPoll::Closed, None));
        }
        return Ok((EventSubscriptionPoll::Pending, None));
    }
}

fn resolve_source_selector(
    control: &OperationControl,
    selector: &OperationSourceSelector,
) -> Result<ExecutionScopeId, OperationHandleError> {
    if selector.source_uri.is_empty()
        || selector.source_uri.len() > MAX_SOURCE_URI_BYTES
        || selector.source_uri.chars().any(char::is_control)
        || selector.line == 0
        || selector.column == Some(0)
    {
        return Err(OperationHandleError::InvalidSourceSelector);
    }
    let tree = control.scope_tree();
    let matches: Vec<_> = tree
        .scopes()
        .filter(|scope| scope.state != ExecutionScopeState::Completed)
        .filter(|scope| {
            scope.source_location.as_ref().is_some_and(|location| {
                location.source_uri == selector.source_uri
                    && location.line == selector.line
                    && selector
                        .column
                        .is_none_or(|column| location.column == Some(column))
            })
        })
        .map(|scope| scope.id)
        .collect();
    match matches.as_slice() {
        [scope] => Ok(*scope),
        [] => Err(OperationHandleError::SourceSelectorNotFound),
        _ => Err(OperationHandleError::SourceSelectorAmbiguous {
            matches: matches.len(),
        }),
    }
}

fn validate_outcome<R>(
    operation_id: OperationId,
    outcome: &OperationOutcome<R>,
) -> Result<(), OperationHandleError> {
    let cause = match outcome {
        OperationOutcome::Failed { cause, .. } | OperationOutcome::Fatal { cause, .. } => {
            Some(cause)
        }
        OperationOutcome::Succeeded { .. } | OperationOutcome::Cancelled { .. } => None,
    };
    if let Some(cause) = cause {
        if cause.operation_id != operation_id {
            return Err(OperationHandleError::ForeignTerminalCause(
                cause.operation_id,
            ));
        }
    }
    if let OperationOutcome::Cancelled {
        reason: Some(reason),
        ..
    } = outcome
    {
        if reason.is_empty()
            || reason.len() > MAX_CONTROL_REASON_BYTES
            || reason.chars().any(char::is_control)
        {
            return Err(OperationHandleError::InvalidTerminalReason);
        }
    }
    let artifacts = match outcome {
        OperationOutcome::Succeeded { artifacts, .. }
        | OperationOutcome::Failed { artifacts, .. }
        | OperationOutcome::Cancelled { artifacts, .. }
        | OperationOutcome::Fatal { artifacts, .. } => artifacts,
    };
    for metadata in &artifacts.retained.items {
        if metadata.operation_id != operation_id {
            return Err(OperationHandleError::ForeignRetainedHandle(
                metadata.operation_id,
            ));
        }
        validate_retained_label(&metadata.label)?;
    }
    for artifact in &artifacts.discarded.items {
        validate_retained_label(&artifact.label)?;
        if artifact.reason.as_ref().is_some_and(|reason| {
            reason.is_empty()
                || reason.len() > MAX_CONTROL_REASON_BYTES
                || reason.chars().any(char::is_control)
        }) {
            return Err(OperationHandleError::InvalidTerminalReason);
        }
    }
    Ok(())
}

fn validate_retained_artifacts<R>(
    state: &OperationState<R>,
    outcome: &OperationOutcome<R>,
) -> Result<(), OperationHandleError> {
    let artifacts = match outcome {
        OperationOutcome::Succeeded { artifacts, .. }
        | OperationOutcome::Failed { artifacts, .. }
        | OperationOutcome::Cancelled { artifacts, .. }
        | OperationOutcome::Fatal { artifacts, .. } => artifacts,
    };
    for metadata in &artifacts.retained.items {
        let entry = state.retained.get(&metadata.handle_id).ok_or(
            OperationHandleError::UnknownRetainedHandle(metadata.handle_id),
        )?;
        if entry.metadata != *metadata {
            return Err(OperationHandleError::RetainedHandleKindMismatch);
        }
    }
    Ok(())
}

fn validate_retained_label(label: &str) -> Result<(), OperationHandleError> {
    if label.is_empty() || label.len() > MAX_IDENTITY_BYTES || label.chars().any(char::is_control) {
        return Err(OperationHandleError::InvalidRetainedLabel);
    }
    Ok(())
}

fn wake_all(wakers: Vec<Waker>) {
    for waker in wakers {
        waker.wake();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Barrier;
    use std::task::{Wake, Waker};
    use std::thread;

    use crate::capability::{
        CapabilityAvailability, ControlCapabilityKind, RuntimeKind, DEFAULT_MAX_LIVE_SUBSCRIPTIONS,
        DEFAULT_SUBSCRIPTION_CAPACITY, MAX_INLINE_EVENT_PAYLOAD_BYTES, MAX_SUBSCRIPTION_CAPACITY,
    };
    use crate::operation_control::{
        ExecutionScopeKind, ExecutionScopeRegistration, SourceLocation,
    };
    use crate::scheduler::{AbortSignal, ScopePolicy};

    struct FlagWake(AtomicBool);

    impl Wake for FlagWake {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    fn operation<R: Send + Sync + 'static>() -> (
        OperationControl,
        OperationHandle<R>,
        OperationTerminalPublisher<R>,
    ) {
        let control = OperationControl::new(AbortSignal::new());
        let (handle, publisher) = OperationHandle::with_defaults(control.clone()).unwrap();
        (control, handle, publisher)
    }

    #[test]
    fn initialization_versions_and_discovers_effective_limits_and_native_controls() {
        let response = initialize_operation_host(OperationInitializeRequest {
            protocol_version: OPERATION_PROTOCOL_VERSION,
            capability: CapabilityRequest {
                runtime: RuntimeKind::Native,
                target_identity: "x86_64-unknown-linux-gnu".to_owned(),
                abi_identity: "cem-ml-rust-v1".to_owned(),
                debug_control_active: false,
            },
        })
        .unwrap();
        assert_eq!(response.protocol_version, OPERATION_PROTOCOL_VERSION);
        assert_eq!(
            response.capability.operation_limits,
            OperationHostLimits {
                max_live_subscriptions: DEFAULT_MAX_LIVE_SUBSCRIPTIONS,
                default_subscription_capacity: DEFAULT_SUBSCRIPTION_CAPACITY,
                max_subscription_capacity: MAX_SUBSCRIPTION_CAPACITY,
                max_inline_event_payload_bytes: MAX_INLINE_EVENT_PAYLOAD_BYTES,
                ..OperationHostLimits::default()
            }
        );
        assert_eq!(
            response
                .capability
                .control(ControlCapabilityKind::OperationHandles)
                .availability,
            CapabilityAvailability::Available
        );
        assert!(matches!(
            initialize_operation_host(OperationInitializeRequest {
                protocol_version: OPERATION_PROTOCOL_VERSION + 1,
                capability: CapabilityRequest {
                    runtime: RuntimeKind::Native,
                    target_identity: "native".to_owned(),
                    abi_identity: "v1".to_owned(),
                    debug_control_active: false,
                },
            }),
            Err(OperationHandleError::ProtocolVersion { .. })
        ));

        let stricter_limits = OperationHostLimits {
            max_live_subscriptions: 4,
            default_subscription_capacity: 32,
            max_subscription_capacity: 128,
            max_inline_event_payload_bytes: 16 * 1_024,
            max_terminal_diagnostics: 64,
            max_recovered_control_failures: 32,
            max_artifact_references: 512,
            max_retained_handles: 256,
            ..OperationHostLimits::default()
        };
        let stricter = initialize_operation_host_with_limits(
            OperationInitializeRequest {
                protocol_version: OPERATION_PROTOCOL_VERSION,
                capability: CapabilityRequest {
                    runtime: RuntimeKind::Native,
                    target_identity: "native".to_owned(),
                    abi_identity: "v1".to_owned(),
                    debug_control_active: false,
                },
            },
            stricter_limits,
        )
        .unwrap();
        assert_eq!(stricter.capability.operation_limits, stricter_limits);

        let oversized_limits = OperationHostLimits {
            max_live_subscriptions: DEFAULT_MAX_LIVE_SUBSCRIPTIONS + 1,
            ..OperationHostLimits::default()
        };
        assert!(matches!(
            initialize_operation_host_with_limits(
                OperationInitializeRequest {
                    protocol_version: OPERATION_PROTOCOL_VERSION,
                    capability: CapabilityRequest {
                        runtime: RuntimeKind::Native,
                        target_identity: "native".to_owned(),
                        abi_identity: "v1".to_owned(),
                        debug_control_active: false,
                    },
                },
                oversized_limits,
            ),
            Err(OperationHandleError::InvalidHostLimit {
                field: "maxLiveSubscriptions",
                ..
            })
        ));
    }

    #[test]
    fn result_future_wakes_and_terminal_racers_retain_one_winner() {
        let (_, handle, publisher) = operation::<String>();
        let mut result = handle.result();
        let flag = Arc::new(FlagWake(AtomicBool::new(false)));
        let waker = Waker::from(Arc::clone(&flag));
        let mut context = Context::from_waker(&waker);
        assert!(Pin::new(&mut result).poll(&mut context).is_pending());

        let barrier = Arc::new(Barrier::new(3));
        let racers: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|value| {
                let publisher = publisher.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    publisher
                        .settle(OperationOutcome::succeeded(
                            value.to_owned(),
                            Vec::new(),
                            ArtifactDisposition::default(),
                        ))
                        .unwrap()
                })
            })
            .collect();
        barrier.wait();
        let claims: Vec<_> = racers
            .into_iter()
            .map(|racer| racer.join().unwrap())
            .collect();
        assert_eq!(claims.iter().filter(|claim| claim.published()).count(), 1);
        assert!(flag.0.load(Ordering::SeqCst));
        let Poll::Ready(Ok(winner)) = Pin::new(&mut result).poll(&mut context) else {
            panic!("terminal result should be ready")
        };
        let OperationOutcome::Succeeded { result, .. } = winner.as_ref() else {
            panic!("expected successful winner")
        };
        assert!(result == "first" || result == "second");
        assert!(claims
            .iter()
            .all(|claim| Arc::ptr_eq(claim.outcome(), &winner)));
    }

    #[test]
    fn accepted_cancellation_and_normal_completion_race_without_state_mutation_after_terminal() {
        for _ in 0..32 {
            let (control, handle, publisher) = operation::<u32>();
            let barrier = Arc::new(Barrier::new(3));
            let success_thread = {
                let publisher = publisher.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    publisher.settle(OperationOutcome::succeeded(
                        7,
                        Vec::new(),
                        ArtifactDisposition::default(),
                    ))
                })
            };
            let cancel_thread = {
                let control = control.clone();
                let handle = handle.clone();
                let publisher = publisher.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    match handle.cancel(CancelRequest {
                        reason: Some("race".to_owned()),
                        ..CancelRequest::default()
                    }) {
                        Ok(_) => {
                            let failure = match control.check_scope(control.root_scope()) {
                                Err(ControlError::Triggered(failure)) => failure,
                                other => panic!("accepted cancellation has no failure: {other:?}"),
                            };
                            Some(
                                publisher
                                    .settle(OperationOutcome::from_control_failure(
                                        failure,
                                        Vec::new(),
                                        ArtifactDisposition::default(),
                                    ))
                                    .unwrap(),
                            )
                        }
                        Err(OperationHandleError::TerminalPublished) => None,
                        other => panic!("unexpected cancellation race result: {other:?}"),
                    }
                })
            };
            barrier.wait();
            let success = success_thread.join().unwrap();
            let cancelled = cancel_thread.join().unwrap();
            let winner = handle
                .blocking_result_timeout(Duration::from_millis(20))
                .unwrap()
                .expect("race publishes one result");
            match winner.status() {
                OperationTerminalStatus::Succeeded => {
                    assert!(success.unwrap().published());
                    assert!(cancelled.is_none());
                    assert!(!control.is_cancelled());
                }
                OperationTerminalStatus::Cancelled => {
                    assert!(matches!(
                        success,
                        Err(OperationHandleError::TerminalControlConflict(_))
                            | Ok(TerminalClaim::AlreadyPublished(_))
                    ));
                    assert!(cancelled.unwrap().published());
                    assert!(control.is_cancelled());
                }
                status => panic!("unexpected race terminal status: {status:?}"),
            }
        }
    }

    #[test]
    fn cancel_resolves_root_scope_and_unique_source_without_mutating_invalid_requests() {
        let (control, handle, _) = operation::<()>();
        let root_policy = ScopePolicy::host_root();
        let make_registration = |label: &str, line| ExecutionScopeRegistration {
            kind: ExecutionScopeKind::Transform,
            label: label.to_owned(),
            source_location: Some(SourceLocation {
                source_uri: "file:///input.cem".to_owned(),
                line,
                column: Some(3),
                end_line: None,
                end_column: None,
                byte_range: None,
            }),
            semantic_identities: BTreeMap::new(),
            error_boundary: None,
            effective_policy: root_policy,
        };
        let selected = control
            .register_scope(control.root_scope(), make_registration("selected", 7))
            .unwrap();
        let untouched = control
            .register_scope(control.root_scope(), make_registration("untouched", 8))
            .unwrap();

        assert!(matches!(
            handle.cancel(CancelRequest {
                scope: Some(selected),
                source_selector: Some(OperationSourceSelector {
                    source_uri: "file:///input.cem".to_owned(),
                    line: 7,
                    column: None,
                }),
                reason: None,
            }),
            Err(OperationHandleError::AmbiguousCancelTarget)
        ));
        assert!(control.check_scope(selected).is_ok());

        let ack = handle
            .cancel(CancelRequest {
                source_selector: Some(OperationSourceSelector {
                    source_uri: "file:///input.cem".to_owned(),
                    line: 7,
                    column: Some(3),
                }),
                reason: Some("caller stopped sub-transform".to_owned()),
                ..CancelRequest::default()
            })
            .unwrap();
        assert_eq!(ack.selected_scope, selected);
        assert_eq!(ack.disposition, ControlAckDisposition::Accepted);
        assert!(matches!(
            control.check_scope(selected),
            Err(ControlError::Triggered(_))
        ));
        assert!(control.check_scope(untouched).is_ok());
        let direct = handle
            .cancel(CancelRequest {
                scope: Some(untouched),
                reason: Some("direct scope".to_owned()),
                ..CancelRequest::default()
            })
            .unwrap();
        assert_eq!(direct.selected_scope, untouched);
        assert!(matches!(
            control.check_scope(untouched),
            Err(ControlError::Triggered(_))
        ));

        let (_, root_handle, _) = operation::<()>();
        let root_ack = root_handle.cancel(CancelRequest::default()).unwrap();
        assert_eq!(root_ack.selected_scope, ExecutionScopeId::from_raw(0));
    }

    #[test]
    fn subscriptions_have_independent_filters_gaps_and_retained_critical_events() {
        let (_, handle, publisher) = operation::<()>();
        let mut tiny = handle
            .subscribe(EventSubscriptionOptions {
                capacity: Some(2),
                ..EventSubscriptionOptions::default()
            })
            .unwrap();
        let mut progress_only = handle
            .subscribe(EventSubscriptionOptions {
                filters: BTreeSet::from([OperationEventKind::Progress]),
                ..EventSubscriptionOptions::default()
            })
            .unwrap();
        for value in 0..5 {
            handle
                .publish_event(
                    OperationEventKind::Progress,
                    &serde_json::json!({ "value": value }),
                )
                .unwrap();
        }
        handle
            .publish_event(
                OperationEventKind::Stopped,
                &serde_json::json!({ "stop": 1 }),
            )
            .unwrap();
        for value in 5..10 {
            handle
                .publish_event(
                    OperationEventKind::Diagnostic,
                    &serde_json::json!({ "value": value }),
                )
                .unwrap();
        }
        handle
            .publish_event(
                OperationEventKind::Continued,
                &serde_json::json!({ "stop": 1 }),
            )
            .unwrap();
        publisher
            .settle(OperationOutcome::succeeded(
                (),
                Vec::new(),
                ArtifactDisposition::default(),
            ))
            .unwrap();

        let (_, gap) = tiny.try_next().unwrap();
        let gap = gap.unwrap();
        assert_eq!(gap.kind, OperationEventKind::SubscriptionGap);
        let decoded: SubscriptionGap = serde_json::from_value(gap.payload.clone()).unwrap();
        assert!(decoded.first_missing <= decoded.last_missing);

        let mut seen_progress = 0;
        while let Some(event) = progress_only
            .blocking_next_timeout(Duration::from_millis(10))
            .unwrap()
        {
            if event.kind == OperationEventKind::Progress {
                seen_progress += 1;
            }
        }
        assert_eq!(seen_progress, 5);

        let mut post_terminal = handle
            .subscribe(EventSubscriptionOptions::default())
            .unwrap();
        let mut kinds = Vec::new();
        while let Some(event) = post_terminal
            .blocking_next_timeout(Duration::from_millis(10))
            .unwrap()
        {
            kinds.push(event.kind);
        }
        assert!(kinds.contains(&OperationEventKind::Continued));
        assert!(kinds.contains(&OperationEventKind::Terminal));

        let control = OperationControl::new(AbortSignal::new());
        let limits = OperationHostLimits {
            default_subscription_capacity: 2,
            max_subscription_capacity: 2,
            ..OperationHostLimits::default()
        };
        let (critical_handle, critical_publisher) =
            OperationHandle::<()>::new(control, limits).unwrap();
        critical_handle
            .publish_event(
                OperationEventKind::Stopped,
                &serde_json::json!({ "stop": 2 }),
            )
            .unwrap();
        for value in 0..3 {
            critical_handle
                .publish_event(
                    OperationEventKind::Diagnostic,
                    &serde_json::json!({ "value": value }),
                )
                .unwrap();
        }
        let mut while_stopped = critical_handle
            .subscribe(EventSubscriptionOptions::default())
            .unwrap();
        let mut retained_current_stop = false;
        loop {
            match while_stopped.try_next().unwrap() {
                (EventSubscriptionPoll::Event, Some(event)) => {
                    retained_current_stop |= event.kind == OperationEventKind::Stopped;
                }
                (EventSubscriptionPoll::Pending, None) => break,
                other => panic!("unexpected current-stop subscription state: {other:?}"),
            }
        }
        assert!(retained_current_stop);

        critical_handle
            .publish_event(
                OperationEventKind::Continued,
                &serde_json::json!({ "stop": 2 }),
            )
            .unwrap();
        critical_publisher
            .settle(OperationOutcome::succeeded(
                (),
                Vec::new(),
                ArtifactDisposition::default(),
            ))
            .unwrap();
        let mut after_continue = critical_handle
            .subscribe(EventSubscriptionOptions::default())
            .unwrap();
        let mut retained = Vec::new();
        while let Some(event) = after_continue
            .blocking_next_timeout(Duration::from_millis(10))
            .unwrap()
        {
            retained.push(event.kind);
        }
        assert!(!retained.contains(&OperationEventKind::Stopped));
        assert!(retained.contains(&OperationEventKind::Continued));
        assert!(retained.contains(&OperationEventKind::Terminal));
    }

    #[test]
    fn retained_handles_preserve_native_type_kind_operation_and_disposal() {
        let (_, handle, _) = operation::<()>();
        let value = handle
            .retain_value("parsed-ast", vec![1_u32, 2, 3])
            .unwrap();
        let resolved = handle
            .resolve_retained::<Vec<u32>>(&value, RetainedHandleKind::NativeValue)
            .unwrap();
        assert_eq!(resolved.as_slice(), &[1, 2, 3]);
        assert!(matches!(
            handle.resolve_retained::<String>(&value, RetainedHandleKind::NativeValue),
            Err(OperationHandleError::RetainedHandleTypeMismatch)
        ));
        assert!(matches!(
            handle.resolve_retained::<Vec<u32>>(&value, RetainedHandleKind::Artifact),
            Err(OperationHandleError::RetainedHandleKindMismatch)
        ));

        let (_, foreign, _) = operation::<()>();
        assert!(matches!(
            foreign.resolve_retained::<Vec<u32>>(&value, RetainedHandleKind::NativeValue),
            Err(OperationHandleError::ForeignRetainedHandle(_))
        ));
        handle.dispose();
        assert!(matches!(
            handle.resolve_retained::<Vec<u32>>(&value, RetainedHandleKind::NativeValue),
            Err(OperationHandleError::Disposed)
        ));
    }

    #[test]
    fn payload_subscription_and_retained_handle_limits_fail_without_partial_publication() {
        let (_, handle, _) = operation::<()>();
        let oversized = "x".repeat(MAX_INLINE_EVENT_PAYLOAD_BYTES as usize + 1);
        assert!(matches!(
            handle.publish_event(OperationEventKind::Progress, &oversized),
            Err(OperationHandleError::EventPayloadTooLarge { .. })
        ));
        let subscriptions: Vec<_> = (0..DEFAULT_MAX_LIVE_SUBSCRIPTIONS)
            .map(|_| {
                handle
                    .subscribe(EventSubscriptionOptions::default())
                    .unwrap()
            })
            .collect();
        assert!(matches!(
            handle.subscribe(EventSubscriptionOptions::default()),
            Err(OperationHandleError::SubscriptionLimit { .. })
        ));
        drop(subscriptions);

        let control = OperationControl::new(AbortSignal::new());
        let limits = OperationHostLimits {
            max_retained_handles: 1,
            ..OperationHostLimits::default()
        };
        let (bounded, _) = OperationHandle::<()>::new(control, limits).unwrap();
        bounded.retain_value("one", 1_u8).unwrap();
        assert!(matches!(
            bounded.retain_artifact("two", 2_u8),
            Err(OperationHandleError::RetainedHandleLimit { maximum: 1 })
        ));

        let tiny_control = OperationControl::new(AbortSignal::new());
        let tiny_limits = OperationHostLimits {
            max_inline_event_payload_bytes: 32,
            ..OperationHostLimits::default()
        };
        let (tiny_payload, _) =
            OperationHandle::<()>::new(tiny_control.clone(), tiny_limits).unwrap();
        assert!(matches!(
            tiny_payload.cancel(CancelRequest::default()),
            Err(OperationHandleError::EventPayloadTooLarge { .. })
        ));
        assert!(tiny_control.check_scope(tiny_control.root_scope()).is_ok());
    }

    #[test]
    fn terminal_control_classification_and_metadata_are_bounded() {
        let (control, handle, publisher) = operation::<()>();
        handle
            .cancel(CancelRequest {
                reason: Some("timeout owned by host".to_owned()),
                ..CancelRequest::default()
            })
            .unwrap();
        let failure = match control.check_scope(control.root_scope()) {
            Err(ControlError::Triggered(failure)) => failure,
            other => panic!("expected cancellation failure, got {other:?}"),
        };
        let diagnostics = (0..300)
            .map(|index| Diagnostic {
                uri: None,
                line: None,
                column: None,
                byte_offset: None,
                code: "cem.test".to_owned(),
                severity: crate::diagnostics::Severity::Info,
                message: index.to_string(),
                node: None,
                details: None,
                source_map: None,
            })
            .collect();
        publisher
            .settle(OperationOutcome::from_control_failure(
                failure,
                diagnostics,
                ArtifactDisposition::default(),
            ))
            .unwrap();
        let outcome = handle
            .blocking_result_timeout(Duration::from_millis(10))
            .unwrap()
            .unwrap();
        let OperationOutcome::Cancelled {
            reason,
            diagnostics,
            artifacts,
        } = outcome.as_ref()
        else {
            panic!("host cancellation must classify as cancelled")
        };
        assert_eq!(reason.as_deref(), Some("timeout owned by host"));
        assert_eq!(diagnostics.items.len(), 256);
        assert_eq!(diagnostics.original_count, 300);
        assert!(diagnostics.was_truncated());
        assert_eq!(artifacts.retained.original_count, 0);
        assert_eq!(artifacts.discarded.original_count, 0);

        let control = OperationControl::new(AbortSignal::new());
        let limits = OperationHostLimits {
            max_artifact_references: 1,
            ..OperationHostLimits::default()
        };
        let (artifact_handle, artifact_publisher) =
            OperationHandle::<()>::new(control, limits).unwrap();
        let first = artifact_handle.retain_artifact("first", 1_u8).unwrap();
        let second = artifact_handle.retain_artifact("second", 2_u8).unwrap();
        artifact_publisher
            .settle(OperationOutcome::succeeded(
                (),
                Vec::new(),
                ArtifactDisposition::new(
                    vec![first, second],
                    vec![
                        DiscardedArtifact {
                            label: "third".to_owned(),
                            reason: None,
                        },
                        DiscardedArtifact {
                            label: "fourth".to_owned(),
                            reason: Some("not committed".to_owned()),
                        },
                    ],
                ),
            ))
            .unwrap();
        let bounded = artifact_handle
            .blocking_result_timeout(Duration::from_millis(10))
            .unwrap()
            .unwrap();
        let OperationOutcome::Succeeded { artifacts, .. } = bounded.as_ref() else {
            panic!("expected successful artifact outcome")
        };
        assert_eq!(artifacts.retained.items.len(), 1);
        assert_eq!(artifacts.retained.original_count, 2);
        assert_eq!(artifacts.discarded.items.len(), 1);
        assert_eq!(artifacts.discarded.original_count, 2);
    }
}
