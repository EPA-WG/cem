//! CPU worker pool (AC-A-3, AC-A-4, AC-A-7, AC-O-2).
//!
//! Tier B `WorkerPool` is intentionally deterministic: tasks dispatch
//! sequentially from the bounded queue in FIFO order so the scheduler
//! trace is reproducible across runs. A parallel runtime is a Tier B+
//! refinement that re-uses the same trace+queue surface to preserve
//! event-sequence determinism (AC-A-3 second paragraph).

use crate::scheduler::abort::AbortSignal;
use crate::scheduler::policy::ScopePolicy;
use crate::scheduler::queue::{BoundedQueue, QueueError};
use crate::scheduler::trace::{SchedulerEventKind, SchedulerTrace};

#[derive(Debug)]
pub struct TaskHandle {
    pub task: String,
    pub scope: u32,
}

/// Per-scope CPU worker pool. Owns its bounded queue, a clone-shared
/// trace, and the policy's caps.
#[derive(Debug)]
pub struct WorkerPool {
    scope: u32,
    policy: ScopePolicy,
    queue: BoundedQueue,
    trace: SchedulerTrace,
}

impl WorkerPool {
    pub fn new(scope: u32, policy: ScopePolicy, trace: SchedulerTrace) -> Self {
        let queue = BoundedQueue::new(scope, policy.queue_size, policy.overflow, trace.clone());
        Self {
            scope,
            policy,
            queue,
            trace,
        }
    }

    pub fn scope(&self) -> u32 {
        self.scope
    }

    pub fn policy(&self) -> ScopePolicy {
        self.policy
    }

    pub fn queue(&self) -> &BoundedQueue {
        &self.queue
    }

    pub fn trace(&self) -> &SchedulerTrace {
        &self.trace
    }

    /// Submit a task for execution. Returns a handle the caller can
    /// hand to [`Self::run_next`] later. Honours the queue's overflow
    /// policy and the [`AbortSignal`].
    pub fn submit(
        &self,
        task: impl Into<String>,
        abort: &AbortSignal,
    ) -> Result<(), QueueError> {
        self.queue.enqueue(task, abort)
    }

    /// Run the next queued task by invoking `f`. Records `Dispatch`
    /// (via the queue), then `Finish` (or `Abort` if the signal fires
    /// before invocation). Returns the task name when one was run.
    pub fn run_next<F>(&self, abort: &AbortSignal, f: F) -> Option<TaskHandle>
    where
        F: FnOnce(&str),
    {
        let task = self.queue.dequeue()?;
        if abort.is_aborted() {
            self.trace
                .record(self.scope, SchedulerEventKind::Abort, task.clone());
            return None;
        }
        f(&task);
        self.trace
            .record(self.scope, SchedulerEventKind::Finish, task.clone());
        Some(TaskHandle {
            task,
            scope: self.scope,
        })
    }

    /// Drain every queued task in FIFO order.
    pub fn run_to_completion<F>(&self, abort: &AbortSignal, mut f: F) -> Vec<TaskHandle>
    where
        F: FnMut(&str),
    {
        let mut handles = Vec::new();
        while !self.queue.is_empty() && !abort.is_aborted() {
            if let Some(handle) = self.run_next(abort, &mut f) {
                handles.push(handle);
            } else {
                break;
            }
        }
        handles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::policy::{OverflowPolicy, ScopePolicy};
    use std::sync::{Arc, Mutex};

    fn pool(cap: u32) -> WorkerPool {
        let policy = ScopePolicy {
            cpu_workers: 1,
            queue_size: cap,
            io_streams: 4,
            memory_bytes: 1024,
            overflow: OverflowPolicy::Reject,
        };
        WorkerPool::new(0, policy, SchedulerTrace::new())
    }

    #[test]
    fn run_next_dispatches_in_fifo_order() {
        let pool = pool(8);
        let abort = AbortSignal::new();
        for t in ["a", "b", "c"] {
            pool.submit(t, &abort).unwrap();
        }
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let cloned = executed.clone();
        pool.run_to_completion(&abort, move |task| {
            cloned.lock().unwrap().push(task.to_owned());
        });
        assert_eq!(*executed.lock().unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn abort_between_dispatch_and_run_short_circuits() {
        let pool = pool(8);
        let abort = AbortSignal::new();
        pool.submit("a", &abort).unwrap();
        abort.abort();
        let handle = pool.run_next(&abort, |_| panic!("must not invoke after abort"));
        assert!(handle.is_none());
    }

    #[test]
    fn trace_records_enqueue_dispatch_finish_for_each_task() {
        let pool = pool(4);
        let abort = AbortSignal::new();
        pool.submit("only", &abort).unwrap();
        pool.run_next(&abort, |_| {});
        let kinds: Vec<SchedulerEventKind> = pool.trace().snapshot().into_iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SchedulerEventKind::Enqueue,
                SchedulerEventKind::Dispatch,
                SchedulerEventKind::Finish,
            ]
        );
    }

    #[test]
    fn two_runs_emit_identical_traces() {
        let abort = AbortSignal::new();
        let run = || {
            let p = pool(8);
            for t in ["a", "b", "c"] {
                p.submit(t, &abort).unwrap();
            }
            p.run_to_completion(&abort, |_| {});
            p.trace().snapshot()
        };
        assert_eq!(run(), run());
    }
}
