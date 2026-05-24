//! Engine scheduler integration smoke
//! (AC-A-4..AC-A-7, AC-O-2).
//!
//! Exercises the scheduler as a whole: policy tree (constrain-only
//! inheritance), worker pool / bounded queue / io queue, abort
//! propagation, and a deterministic trace.

use cem_ml::scheduler::tree::{PolicyScopeId, ScopePolicyTreeError};
use cem_ml::scheduler::{
    AbortSignal, BoundedQueue, IoQueue, OverflowPolicy, SchedulerTrace, ScopePolicy,
    ScopePolicyTree, WorkerPool,
};
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

fn policy(cpu: u32, queue: u32, io: u32, mem: u64, overflow: OverflowPolicy) -> ScopePolicy {
    ScopePolicy {
        cpu_workers: cpu,
        queue_size: queue,
        io_streams: io,
        memory_bytes: mem,
        plugin_time_budget_ms: None,
        overflow,
    }
}

#[test]
fn child_cannot_raise_any_inherited_cap() {
    let root = policy(4, 16, 8, 1024, OverflowPolicy::Reject);
    let mut tree = ScopePolicyTree::new(PolicyScopeId(0), root);
    // Lower → ok.
    tree.install(
        PolicyScopeId(1),
        PolicyScopeId(0),
        policy(2, 8, 4, 512, OverflowPolicy::Reject),
    )
    .unwrap();
    // Match → ok.
    tree.install(
        PolicyScopeId(2),
        PolicyScopeId(0),
        policy(4, 16, 8, 1024, OverflowPolicy::Reject),
    )
    .unwrap();
    // Raise CPU → denied.
    let err = tree
        .install(
            PolicyScopeId(3),
            PolicyScopeId(0),
            policy(8, 16, 8, 1024, OverflowPolicy::Reject),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        ScopePolicyTreeError::CapRelaxationDenied { .. }
    ));
    assert_eq!(err.code(), "cem.a.cap_relaxation_denied");
}

#[test]
fn end_to_end_pool_run_emits_deterministic_trace() {
    let trace = SchedulerTrace::new();
    let pool = WorkerPool::new(
        7,
        policy(2, 8, 4, 1024, OverflowPolicy::Reject),
        trace.clone(),
    );
    let abort = AbortSignal::new();

    for t in ["tokenize", "normalize", "ast-build"] {
        pool.submit(t, &abort).unwrap();
    }
    pool.run_to_completion(&abort, |_| {});

    let kinds: Vec<_> = trace
        .snapshot()
        .into_iter()
        .map(|e| (e.scope, e.kind, e.task))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (7, cem_ml::scheduler::SchedulerEventKind::Enqueue, "tokenize".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Enqueue, "normalize".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Enqueue, "ast-build".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Dispatch, "tokenize".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Finish, "tokenize".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Dispatch, "normalize".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Finish, "normalize".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Dispatch, "ast-build".into()),
            (7, cem_ml::scheduler::SchedulerEventKind::Finish, "ast-build".into()),
        ]
    );
}

#[test]
fn io_queue_is_independent_from_cpu_pool() {
    let trace = SchedulerTrace::new();
    let cpu_queue = BoundedQueue::new(0, 1, OverflowPolicy::Reject, trace.clone());
    let io = IoQueue::new(0, 3, trace);
    let abort = AbortSignal::new();
    cpu_queue.enqueue("cpu", &abort).unwrap();
    let a = io.acquire("fetch-a", &abort).unwrap();
    let b = io.acquire("fetch-b", &abort).unwrap();
    let c = io.acquire("fetch-c", &abort).unwrap();
    let exhausted = io.acquire("fetch-d", &abort).unwrap_err();
    assert_eq!(exhausted.code(), "cem.a.io_exhausted");
    assert_eq!(io.inflight(), 3);
    drop((a, b, c));
    assert_eq!(io.inflight(), 0);
    assert_eq!(cpu_queue.len(), 1);
}

#[test]
fn bounded_cpu_queue_overflow_carries_scope_id() {
    let trace = SchedulerTrace::new();
    let queue = BoundedQueue::new(12, 1, OverflowPolicy::Reject, trace);
    let abort = AbortSignal::new();
    queue.enqueue("first", &abort).unwrap();
    let err = queue.enqueue("second", &abort).unwrap_err();
    assert_eq!(err.code(), "cem.scheduler.queue_full");
    match err {
        cem_ml::scheduler::QueueError::Overflow { scope, .. } => assert_eq!(scope, 12),
        other => panic!("expected overflow queue error, got {other:?}"),
    }
}

#[test]
fn abort_signal_cancels_pool_and_io_queue() {
    let trace = SchedulerTrace::new();
    let pool = WorkerPool::new(
        0,
        policy(1, 4, 2, 1024, OverflowPolicy::Reject),
        trace.clone(),
    );
    let io = IoQueue::new(0, 4, trace);
    let abort = AbortSignal::new();
    abort.abort();

    let cpu_err = pool.submit("late", &abort).unwrap_err();
    assert_eq!(cpu_err.code(), "cem.scheduler.aborted");
    let io_err = io.acquire("late-io", &abort).unwrap_err();
    assert_eq!(io_err.code(), "cem.scheduler.aborted");
}

#[test]
fn abort_signal_carries_cancel_site_source_map() {
    let trace = SchedulerTrace::new();
    let pool = WorkerPool::new(3, policy(1, 4, 2, 1024, OverflowPolicy::Reject), trace);
    let abort = AbortSignal::new();
    let mut stack = SourceMapStack::default();
    stack.push(SourceMapFrame {
        source_id: SourceId(9),
        span: FrameSpan::Single(ByteRange::new(20, 5)),
        transform: TransformKind::CemTokenizer,
    });
    abort.abort_with_source_map(stack.clone());

    let err = pool.submit("cancelled-task", &abort).unwrap_err();
    assert_eq!(err.code(), "cem.scheduler.aborted");
    match err {
        cem_ml::scheduler::QueueError::Cancelled { source_map, .. } => {
            assert_eq!(source_map, Some(stack));
        }
        other => panic!("expected cancelled queue error, got {other:?}"),
    }
}
