//! Engine scheduling surface (AC-A-4..AC-A-7, AC-O-2).
//!
//! Tier B. The scheduler owns four cross-cutting concerns:
//!
//! - **Per-scope resource caps** ([`policy`]): CPU worker count,
//!   bounded queue size, external-I/O stream count, accounted-memory cap,
//!   logical stack depth, active-time timeout, and the overflow policy applied
//!   when a queue is full.
//! - **Scope inheritance** ([`tree`]): policies are organised in a
//!   tree; children inherit parent caps and MAY constrain them
//!   further only — any attempt to raise a cap above the parent's
//!   bound is rejected with `cem.a.cap_relaxation_denied` (AC-A-4
//!   second paragraph).
//! - **Pools and queues** ([`pool`], [`queue`]): bounded compatibility
//!   queues preserve the original trace surface. Native hosts additionally use
//!   [`executor`] for operation-owned physical CPU and I/O executors,
//!   hierarchical logical permits, and deterministic staged-result commit.
//! - **Cancellation** ([`abort`]): a cooperatively shared
//!   [`abort::AbortSignal`] short-circuits pending work and surfaces
//!   the cancellation through the trace (AC-A-7).
//!
//! Observability of the schedule is part of the contract: the
//! [`trace::SchedulerTrace`] records every enqueue/start/finish/abort
//! with a monotonic sequence number so a postmortem reader can replay
//! the schedule deterministically (AC-O-2).

pub mod abort;
#[cfg(not(target_arch = "wasm32"))]
pub mod executor;
pub mod policy;
pub mod pool;
pub mod queue;
pub mod trace;
pub mod tree;

pub use abort::AbortSignal;
#[cfg(not(target_arch = "wasm32"))]
pub use executor::{
    CommittedTask, NativeScheduler, NativeTaskHandle, ScheduleError, ScheduledTaskSpec,
    StagedTaskResult, TaskPath,
};
pub use policy::{OverflowPolicy, ResourceCap, ScopePolicy, ScopePolicyError};
pub use pool::{TaskHandle, WorkerPool};
pub use queue::{BoundedQueue, IoQueue, QueueError};
pub use trace::{SchedulerEvent, SchedulerEventKind, SchedulerTrace};
pub use tree::{ScopePolicyTree, ScopePolicyTreeError};
