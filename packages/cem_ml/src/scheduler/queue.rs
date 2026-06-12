//! Bounded CPU work queue (AC-A-5) and external-I/O queue (AC-A-6).
//!
//! Tier B is single-threaded and deterministic: both queues are
//! FIFO. The CPU queue honours [`crate::scheduler::policy::OverflowPolicy`]
//! when full; the I/O queue is a permit pool sized by the scope's
//! [`crate::scheduler::policy::ScopePolicy::io_streams`] cap and
//! does NOT consume CPU-pool capacity.

use crate::scheduler::abort::AbortSignal;
use crate::scheduler::policy::OverflowPolicy;
use crate::scheduler::trace::{SchedulerEventKind, SchedulerTrace};
use crate::source_map::SourceMapStack;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    Overflow {
        scope: u32,
        task: String,
        capacity: u32,
    },
    Cancelled {
        scope: u32,
        task: String,
        source_map: Option<SourceMapStack>,
    },
    IoExhausted {
        scope: u32,
        task: String,
        capacity: u32,
    },
}

impl QueueError {
    pub fn code(&self) -> &'static str {
        match self {
            QueueError::Overflow { .. } => "cem.scheduler.queue_full",
            QueueError::Cancelled { .. } => "cem.scheduler.aborted",
            QueueError::IoExhausted { .. } => "cem.a.io_exhausted",
        }
    }
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::Overflow {
                scope,
                task,
                capacity,
            } => write!(
                f,
                "scope {scope} cpu queue full (capacity {capacity}); rejected task `{task}`"
            ),
            QueueError::Cancelled { scope, task, .. } => {
                write!(f, "scope {scope} task `{task}` cancelled by AbortSignal")
            }
            QueueError::IoExhausted {
                scope,
                task,
                capacity,
            } => write!(
                f,
                "scope {scope} io queue exhausted (capacity {capacity}); rejected task `{task}`"
            ),
        }
    }
}

impl std::error::Error for QueueError {}

/// Bounded FIFO queue. The CPU worker pool drains tasks in
/// enqueue order; overflow surfaces through the configured policy.
#[derive(Debug)]
pub struct BoundedQueue {
    scope: u32,
    capacity: u32,
    overflow: OverflowPolicy,
    parent: Option<Arc<Mutex<BoundedQueueInner>>>,
    inner: Arc<Mutex<BoundedQueueInner>>,
    trace: SchedulerTrace,
}

#[derive(Debug)]
struct BoundedQueueInner {
    tasks: VecDeque<String>,
}

impl BoundedQueue {
    pub fn new(scope: u32, capacity: u32, overflow: OverflowPolicy, trace: SchedulerTrace) -> Self {
        Self {
            scope,
            capacity,
            overflow,
            parent: None,
            inner: Arc::new(Mutex::new(BoundedQueueInner {
                tasks: VecDeque::new(),
            })),
            trace,
        }
    }

    pub fn with_parent(mut self, parent: &BoundedQueue) -> Self {
        self.parent = Some(parent.inner.clone());
        self
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("poisoned cpu-queue mutex")
            .tasks
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Enqueue a task. Honours the configured [`OverflowPolicy`]:
    /// `Block` is approximated as immediate accept (Tier B is
    /// single-threaded so a block reduces to "wait until the caller
    /// drains the queue"); `Reject` surfaces [`QueueError::Overflow`];
    /// `SpillToParent` forwards to the parent queue, degrading to
    /// reject if no parent exists.
    pub fn enqueue(&self, task: impl Into<String>, abort: &AbortSignal) -> Result<(), QueueError> {
        let task: String = task.into();
        if abort.is_aborted() {
            self.trace
                .record(self.scope, SchedulerEventKind::Abort, task.clone());
            return Err(QueueError::Cancelled {
                scope: self.scope,
                task,
                source_map: abort.source_map(),
            });
        }
        let mut inner = self.inner.lock().expect("poisoned cpu-queue mutex");
        if (inner.tasks.len() as u32) >= self.capacity {
            drop(inner);
            return self.handle_overflow(task);
        }
        inner.tasks.push_back(task.clone());
        drop(inner);
        self.trace
            .record(self.scope, SchedulerEventKind::Enqueue, task);
        Ok(())
    }

    fn handle_overflow(&self, task: String) -> Result<(), QueueError> {
        self.trace
            .record(self.scope, SchedulerEventKind::Overflow, task.clone());
        match self.overflow {
            OverflowPolicy::Reject => Err(QueueError::Overflow {
                scope: self.scope,
                task,
                capacity: self.capacity,
            }),
            OverflowPolicy::SpillToParent => match &self.parent {
                Some(parent) => {
                    let mut parent_inner = parent.lock().expect("poisoned parent cpu-queue mutex");
                    parent_inner.tasks.push_back(task.clone());
                    self.trace
                        .record(self.scope, SchedulerEventKind::Enqueue, task);
                    Ok(())
                }
                None => Err(QueueError::Overflow {
                    scope: self.scope,
                    task,
                    capacity: self.capacity,
                }),
            },
            OverflowPolicy::Block => {
                // Tier B single-thread: degrade to reject so the
                // caller doesn't deadlock. Real multi-threaded hosts
                // override this site with a condvar wait.
                Err(QueueError::Overflow {
                    scope: self.scope,
                    task,
                    capacity: self.capacity,
                })
            }
        }
    }

    /// Pop the next task from the queue. Records a `Dispatch` event.
    pub fn dequeue(&self) -> Option<String> {
        let mut inner = self.inner.lock().expect("poisoned cpu-queue mutex");
        let task = inner.tasks.pop_front()?;
        drop(inner);
        self.trace
            .record(self.scope, SchedulerEventKind::Dispatch, task.clone());
        Some(task)
    }
}

/// External-I/O queue (AC-A-6). Permits are independent of the CPU
/// pool. The scheduler trace records `IoAcquire` / `IoRelease` so
/// AC-O-2 captures the I/O schedule alongside the CPU schedule.
#[derive(Debug)]
pub struct IoQueue {
    scope: u32,
    capacity: u32,
    inflight: Arc<Mutex<u32>>,
    trace: SchedulerTrace,
}

impl IoQueue {
    pub fn new(scope: u32, capacity: u32, trace: SchedulerTrace) -> Self {
        Self {
            scope,
            capacity,
            inflight: Arc::new(Mutex::new(0)),
            trace,
        }
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn inflight(&self) -> u32 {
        *self.inflight.lock().expect("poisoned io mutex")
    }

    /// Acquire a permit. Returns an [`IoPermit`] guard that releases
    /// the slot on drop; surfaces [`QueueError::IoExhausted`] when
    /// the cap is reached.
    pub fn acquire(
        &self,
        task: impl Into<String>,
        abort: &AbortSignal,
    ) -> Result<IoPermit, QueueError> {
        let task: String = task.into();
        if abort.is_aborted() {
            self.trace
                .record(self.scope, SchedulerEventKind::Abort, task.clone());
            return Err(QueueError::Cancelled {
                scope: self.scope,
                task,
                source_map: abort.source_map(),
            });
        }
        let mut inflight = self.inflight.lock().expect("poisoned io mutex");
        if *inflight >= self.capacity {
            return Err(QueueError::IoExhausted {
                scope: self.scope,
                task,
                capacity: self.capacity,
            });
        }
        *inflight += 1;
        drop(inflight);
        self.trace
            .record(self.scope, SchedulerEventKind::IoAcquire, task.clone());
        Ok(IoPermit {
            scope: self.scope,
            task,
            inflight: self.inflight.clone(),
            trace: self.trace.clone(),
        })
    }
}

#[derive(Debug)]
pub struct IoPermit {
    scope: u32,
    task: String,
    inflight: Arc<Mutex<u32>>,
    trace: SchedulerTrace,
}

impl Drop for IoPermit {
    fn drop(&mut self) {
        if let Ok(mut count) = self.inflight.lock() {
            if *count > 0 {
                *count -= 1;
            }
        }
        self.trace
            .record(self.scope, SchedulerEventKind::IoRelease, self.task.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_drains_in_enqueue_order() {
        let trace = SchedulerTrace::new();
        let q = BoundedQueue::new(0, 4, OverflowPolicy::Reject, trace);
        let abort = AbortSignal::new();
        q.enqueue("a", &abort).unwrap();
        q.enqueue("b", &abort).unwrap();
        q.enqueue("c", &abort).unwrap();
        assert_eq!(q.dequeue().as_deref(), Some("a"));
        assert_eq!(q.dequeue().as_deref(), Some("b"));
        assert_eq!(q.dequeue().as_deref(), Some("c"));
        assert_eq!(q.dequeue(), None);
    }

    #[test]
    fn reject_policy_returns_overflow() {
        let trace = SchedulerTrace::new();
        let q = BoundedQueue::new(0, 1, OverflowPolicy::Reject, trace);
        let abort = AbortSignal::new();
        q.enqueue("a", &abort).unwrap();
        let err = q.enqueue("b", &abort).unwrap_err();
        assert_eq!(err.code(), "cem.scheduler.queue_full");
    }

    #[test]
    fn spill_to_parent_forwards_when_local_full() {
        let trace = SchedulerTrace::new();
        let parent = BoundedQueue::new(0, 8, OverflowPolicy::Reject, trace.clone());
        let child = BoundedQueue::new(1, 1, OverflowPolicy::SpillToParent, trace.clone())
            .with_parent(&parent);
        let abort = AbortSignal::new();
        child.enqueue("a", &abort).unwrap();
        child.enqueue("b", &abort).unwrap();
        assert_eq!(child.len(), 1);
        assert_eq!(parent.len(), 1, "second task should spill upward");
    }

    #[test]
    fn spill_without_parent_degrades_to_reject() {
        let trace = SchedulerTrace::new();
        let q = BoundedQueue::new(0, 1, OverflowPolicy::SpillToParent, trace);
        let abort = AbortSignal::new();
        q.enqueue("a", &abort).unwrap();
        let err = q.enqueue("b", &abort).unwrap_err();
        assert!(matches!(err, QueueError::Overflow { .. }));
    }

    #[test]
    fn abort_short_circuits_enqueue() {
        let trace = SchedulerTrace::new();
        let q = BoundedQueue::new(0, 4, OverflowPolicy::Reject, trace);
        let abort = AbortSignal::new();
        abort.abort();
        let err = q.enqueue("a", &abort).unwrap_err();
        assert!(matches!(err, QueueError::Cancelled { .. }));
    }

    #[test]
    fn io_queue_releases_on_drop() {
        let trace = SchedulerTrace::new();
        let io = IoQueue::new(0, 2, trace);
        let abort = AbortSignal::new();
        let p1 = io.acquire("fetch-a", &abort).unwrap();
        let p2 = io.acquire("fetch-b", &abort).unwrap();
        assert_eq!(io.inflight(), 2);
        let err = io.acquire("fetch-c", &abort).unwrap_err();
        assert!(matches!(err, QueueError::IoExhausted { .. }));
        drop(p1);
        assert_eq!(io.inflight(), 1);
        drop(p2);
        assert_eq!(io.inflight(), 0);
    }

    #[test]
    fn io_queue_does_not_use_cpu_queue_capacity() {
        let trace = SchedulerTrace::new();
        let cpu = BoundedQueue::new(0, 1, OverflowPolicy::Reject, trace.clone());
        let io = IoQueue::new(0, 4, trace);
        let abort = AbortSignal::new();
        cpu.enqueue("cpu-1", &abort).unwrap();
        // CPU is full; I/O slots independent.
        let _io_a = io.acquire("fetch", &abort).unwrap();
        let _io_b = io.acquire("fetch", &abort).unwrap();
        assert_eq!(io.inflight(), 2);
        assert_eq!(cpu.len(), 1);
    }
}
