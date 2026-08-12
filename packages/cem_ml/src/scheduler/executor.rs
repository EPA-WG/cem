//! Operation-owned native physical executors and deterministic logical commit.
//!
//! CPU and external-I/O work have independent bounded queues and fixed worker
//! sets. A child scope consumes one logical permit from itself and every
//! ancestor; it never creates another OS-thread pool. Physical completion is
//! staged, while public commit follows stable task paths.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::operation_control::{
    ControlCause, ControlError, ControlFailure, ExecutionScopeId, OperationControl, TaskId,
};
use crate::scheduler::{OverflowPolicy, SchedulerEventKind, SchedulerTrace, ScopePolicy};

const CONTROL_POLL_INTERVAL: Duration = Duration::from_millis(10);
pub const MAX_TASK_PATH_SEGMENTS: usize = 64;
pub const MAX_TASK_LABEL_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskPath(Vec<u32>);

impl TaskPath {
    pub fn new(segments: impl Into<Vec<u32>>) -> Result<Self, ScheduleError> {
        let segments = segments.into();
        if segments.is_empty() || segments.len() > MAX_TASK_PATH_SEGMENTS {
            return Err(ScheduleError::InvalidTaskPath);
        }
        Ok(Self(segments))
    }

    pub fn root(index: u32) -> Self {
        Self(vec![index])
    }

    pub fn child(&self, index: u32) -> Result<Self, ScheduleError> {
        let mut segments = self.0.clone();
        segments.push(index);
        Self::new(segments)
    }

    pub fn segments(&self) -> &[u32] {
        &self.0
    }
}

impl fmt::Display for TaskPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, segment) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str(".")?;
            }
            segment.fmt(formatter)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTaskSpec {
    pub owner: ExecutionScopeId,
    pub stable_path: TaskPath,
    pub dependencies: Vec<TaskId>,
    pub label: String,
    pub replayable: bool,
    trace_scope: Option<u32>,
}

impl ScheduledTaskSpec {
    pub fn new(owner: ExecutionScopeId, stable_path: TaskPath, label: impl Into<String>) -> Self {
        Self {
            owner,
            stable_path,
            dependencies: Vec::new(),
            label: label.into(),
            replayable: false,
            trace_scope: None,
        }
    }

    pub fn with_dependencies(mut self, dependencies: impl Into<Vec<TaskId>>) -> Self {
        self.dependencies = dependencies.into();
        self
    }

    pub fn replayable(mut self, replayable: bool) -> Self {
        self.replayable = replayable;
        self
    }

    /// Use a stable public/report scope identity while retaining `owner` for
    /// operation-control accounting.
    pub fn with_trace_scope(mut self, trace_scope: u32) -> Self {
        self.trace_scope = Some(trace_scope);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    Control(ControlError),
    QueueCapacity(ControlFailure),
    WorkerFailure(ControlFailure),
    UnknownDependency(TaskId),
    DependencyFailed(TaskId),
    DuplicateTaskPath(TaskPath),
    InvalidTaskPath,
    InvalidTaskLabel,
    ExecutorThreadWouldBlock,
    ResultChannelClosed,
    Shutdown,
}

impl ScheduleError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Control(error) => error.code(),
            Self::QueueCapacity(failure) | Self::WorkerFailure(failure) => failure.code(),
            Self::UnknownDependency(_) => "cem.scheduler.dependency_unknown",
            Self::DependencyFailed(_) => "cem.scheduler.dependency_failed",
            Self::DuplicateTaskPath(_) => "cem.scheduler.task_path_duplicate",
            Self::InvalidTaskPath => "cem.scheduler.task_path_invalid",
            Self::InvalidTaskLabel => "cem.scheduler.task_label_invalid",
            Self::ExecutorThreadWouldBlock => "cem.scheduler.executor_would_block",
            Self::ResultChannelClosed => "cem.scheduler.result_channel_closed",
            Self::Shutdown => "cem.scheduler.shutdown",
        }
    }
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Control(error) => error.fmt(formatter),
            Self::QueueCapacity(failure) | Self::WorkerFailure(failure) => failure.fmt(formatter),
            Self::UnknownDependency(task) => write!(formatter, "unknown task dependency {task}"),
            Self::DependencyFailed(task) => write!(formatter, "task dependency {task} failed"),
            Self::DuplicateTaskPath(path) => write!(formatter, "duplicate task path {path}"),
            Self::InvalidTaskPath => formatter.write_str("task path is empty or too deep"),
            Self::InvalidTaskLabel => formatter.write_str(
                "task label is empty, contains control characters, or exceeds 256 bytes",
            ),
            Self::ExecutorThreadWouldBlock => formatter
                .write_str("bounded block admission cannot wait on a physical executor thread"),
            Self::ResultChannelClosed => formatter.write_str("task result channel closed"),
            Self::Shutdown => formatter.write_str("native scheduler is shutting down"),
        }
    }
}

impl std::error::Error for ScheduleError {}

impl From<ControlError> for ScheduleError {
    fn from(error: ControlError) -> Self {
        Self::Control(error)
    }
}

#[derive(Debug)]
pub struct StagedTaskResult<T> {
    pub id: TaskId,
    pub owner: ExecutionScopeId,
    pub stable_path: TaskPath,
    pub value: T,
}

#[derive(Debug)]
pub struct CommittedTask<T> {
    pub id: TaskId,
    pub owner: ExecutionScopeId,
    pub stable_path: TaskPath,
    pub value: T,
}

pub struct NativeTaskHandle<T> {
    id: TaskId,
    owner: ExecutionScopeId,
    trace_scope: u32,
    stable_path: TaskPath,
    label: String,
    receiver: Receiver<Result<T, ScheduleError>>,
    trace: SchedulerTrace,
}

impl<T> fmt::Debug for NativeTaskHandle<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeTaskHandle")
            .field("id", &self.id)
            .field("owner", &self.owner)
            .field("stable_path", &self.stable_path)
            .finish_non_exhaustive()
    }
}

impl<T> NativeTaskHandle<T> {
    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn stable_path(&self) -> &TaskPath {
        &self.stable_path
    }

    pub fn join(self) -> Result<StagedTaskResult<T>, ScheduleError> {
        let result = self
            .receiver
            .recv()
            .map_err(|_| ScheduleError::ResultChannelClosed)?;
        match result {
            Ok(value) => {
                self.trace.record(
                    self.trace_scope,
                    SchedulerEventKind::Dispatch,
                    self.label.clone(),
                );
                self.trace
                    .record(self.trace_scope, SchedulerEventKind::Finish, self.label);
                Ok(StagedTaskResult {
                    id: self.id,
                    owner: self.owner,
                    stable_path: self.stable_path,
                    value,
                })
            }
            Err(error) => {
                self.trace
                    .record(self.trace_scope, SchedulerEventKind::Abort, self.label);
                Err(error)
            }
        }
    }
}

pub fn commit_in_stable_order<T>(
    mut handles: Vec<NativeTaskHandle<T>>,
) -> Result<Vec<CommittedTask<T>>, ScheduleError> {
    handles.sort_by(|left, right| left.stable_path.cmp(&right.stable_path));
    for pair in handles.windows(2) {
        if pair[0].stable_path == pair[1].stable_path {
            return Err(ScheduleError::DuplicateTaskPath(
                pair[0].stable_path.clone(),
            ));
        }
    }
    handles
        .into_iter()
        .map(|handle| {
            let staged = handle.join()?;
            Ok(CommittedTask {
                id: staged.id,
                owner: staged.owner,
                stable_path: staged.stable_path,
                value: staged.value,
            })
        })
        .collect()
}

type JobFn =
    Box<dyn FnOnce(u64, Option<ScheduleError>) -> Result<(), ScheduleError> + Send + 'static>;

struct Job {
    id: TaskId,
    owner: ExecutionScopeId,
    queue_scope: ExecutionScopeId,
    ancestors: Vec<ExecutionScopeId>,
    dependencies: Vec<TaskId>,
    failed_dependency: Option<TaskId>,
    run: JobFn,
}

#[derive(Debug, Clone, Copy)]
struct RuntimeScope {
    parent: Option<ExecutionScopeId>,
    policy: ScopePolicy,
    queued_cpu: u32,
    queued_io: u32,
    running_cpu: u32,
    running_io: u32,
}

#[derive(Default)]
struct SchedulerState {
    stopping: bool,
    scopes: BTreeMap<ExecutionScopeId, RuntimeScope>,
    known_tasks: BTreeSet<TaskId>,
    known_paths: BTreeSet<TaskPath>,
    completed_tasks: BTreeSet<TaskId>,
    failed_tasks: BTreeMap<TaskId, ScheduleError>,
    cpu_queue: VecDeque<Job>,
    io_queue: VecDeque<Job>,
}

struct SharedScheduler {
    control: OperationControl,
    state: Mutex<SchedulerState>,
    work_ready: Condvar,
    capacity_ready: Condvar,
}

thread_local! {
    static IS_EXECUTOR_THREAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutorClass {
    Cpu,
    Io,
}

pub struct NativeScheduler {
    shared: Arc<SharedScheduler>,
    cpu_workers: Vec<JoinHandle<()>>,
    io_workers: Vec<JoinHandle<()>>,
    trace: SchedulerTrace,
}

impl fmt::Debug for NativeScheduler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeScheduler")
            .field("operation_id", &self.shared.control.operation_id())
            .field("cpu_workers", &self.cpu_workers.len())
            .field("io_workers", &self.io_workers.len())
            .finish()
    }
}

impl NativeScheduler {
    pub fn new(control: OperationControl, trace: SchedulerTrace) -> Result<Self, ScheduleError> {
        let root = control.root_scope();
        let root_policy = control
            .scope_tree()
            .scope(root)
            .ok_or(ControlError::UnknownScope(root))?
            .effective_policy;
        let mut state = SchedulerState::default();
        state.scopes.insert(
            root,
            RuntimeScope {
                parent: None,
                policy: root_policy,
                queued_cpu: 0,
                queued_io: 0,
                running_cpu: 0,
                running_io: 0,
            },
        );
        let shared = Arc::new(SharedScheduler {
            control,
            state: Mutex::new(state),
            work_ready: Condvar::new(),
            capacity_ready: Condvar::new(),
        });
        let cpu_workers = spawn_workers(&shared, ExecutorClass::Cpu, root_policy.cpu_workers);
        let io_workers = spawn_workers(&shared, ExecutorClass::Io, root_policy.io_streams);
        Ok(Self {
            shared,
            cpu_workers,
            io_workers,
            trace,
        })
    }

    pub fn control(&self) -> &OperationControl {
        &self.shared.control
    }

    pub fn cpu_worker_count(&self) -> usize {
        self.cpu_workers.len()
    }

    pub fn io_worker_count(&self) -> usize {
        self.io_workers.len()
    }

    pub fn register_scope(&self, scope: ExecutionScopeId) -> Result<(), ScheduleError> {
        let snapshot = self.shared.control.scope_tree();
        let node = snapshot
            .scope(scope)
            .ok_or(ControlError::UnknownScope(scope))?;
        let mut state = self.shared.state.lock().expect("poisoned scheduler mutex");
        if state.scopes.contains_key(&scope) {
            return Ok(());
        }
        if node
            .parent
            .is_some_and(|parent| !state.scopes.contains_key(&parent))
        {
            return Err(ControlError::UnknownScope(node.parent.expect("checked parent")).into());
        }
        state.scopes.insert(
            scope,
            RuntimeScope {
                parent: node.parent,
                policy: node.effective_policy,
                queued_cpu: 0,
                queued_io: 0,
                running_cpu: 0,
                running_io: 0,
            },
        );
        Ok(())
    }

    pub fn submit_cpu<T, F>(
        &self,
        spec: ScheduledTaskSpec,
        work: F,
    ) -> Result<NativeTaskHandle<T>, ScheduleError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.submit(ExecutorClass::Cpu, spec, work)
    }

    pub fn submit_io<T, F>(
        &self,
        spec: ScheduledTaskSpec,
        work: F,
    ) -> Result<NativeTaskHandle<T>, ScheduleError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.submit(ExecutorClass::Io, spec, work)
    }

    fn submit<T, F>(
        &self,
        class: ExecutorClass,
        spec: ScheduledTaskSpec,
        work: F,
    ) -> Result<NativeTaskHandle<T>, ScheduleError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.shared.control.check_scope(spec.owner)?;
        if spec.label.is_empty()
            || spec.label.len() > MAX_TASK_LABEL_BYTES
            || spec.label.chars().any(char::is_control)
        {
            return Err(ScheduleError::InvalidTaskLabel);
        }
        let mut state = self.shared.state.lock().expect("poisoned scheduler mutex");
        if state.stopping {
            return Err(ScheduleError::Shutdown);
        }
        if !state.scopes.contains_key(&spec.owner) {
            return Err(ControlError::UnknownScope(spec.owner).into());
        }
        if let Some(unknown) = spec
            .dependencies
            .iter()
            .find(|dependency| !state.known_tasks.contains(dependency))
        {
            return Err(ScheduleError::UnknownDependency(*unknown));
        }
        if !state.known_paths.insert(spec.stable_path.clone()) {
            return Err(ScheduleError::DuplicateTaskPath(spec.stable_path));
        }
        let ancestors = runtime_ancestors(&state, spec.owner)?;
        let queue_scope =
            match wait_for_queue_capacity(&self.shared, state, spec.owner, class, &spec.label) {
                Ok(scope) => scope,
                Err(error) => {
                    self.shared
                        .state
                        .lock()
                        .expect("poisoned scheduler mutex")
                        .known_paths
                        .remove(&spec.stable_path);
                    return Err(error);
                }
            };
        let task = match self.shared.control.register_task(spec.owner) {
            Ok(task) => task,
            Err(error) => {
                let mut state = self.shared.state.lock().expect("poisoned scheduler mutex");
                decrement_queued(&mut state, queue_scope, class);
                state.known_paths.remove(&spec.stable_path);
                self.shared.capacity_ready.notify_all();
                return Err(error.into());
            }
        };
        let (sender, receiver) = mpsc::channel();
        let operation_id = self.shared.control.operation_id();
        let owner = spec.owner;
        let trace_scope = spec.trace_scope.unwrap_or(owner.get() as u32);
        let replayable = spec.replayable;
        let run = Box::new(move |worker: u64, preflight: Option<ScheduleError>| {
            let result = match preflight {
                Some(error) => Err(error),
                None => catch_unwind(AssertUnwindSafe(work)).map_err(|_| {
                    ScheduleError::WorkerFailure(ControlFailure {
                        operation_id,
                        affected_scope: owner,
                        cause: ControlCause::WorkerFailure {
                            worker: Some(worker),
                            restartable: replayable,
                        },
                        source_map: None,
                    })
                }),
            };
            let status = result.as_ref().map(|_| ()).map_err(Clone::clone);
            let _ = sender.send(result);
            status
        });

        state = self.shared.state.lock().expect("poisoned scheduler mutex");
        if state.stopping {
            decrement_queued(&mut state, queue_scope, class);
            state.known_paths.remove(&spec.stable_path);
            return Err(ScheduleError::Shutdown);
        }
        state.known_tasks.insert(task);
        let job = Job {
            id: task,
            owner: spec.owner,
            queue_scope,
            ancestors,
            dependencies: spec.dependencies,
            failed_dependency: None,
            run,
        };
        match class {
            ExecutorClass::Cpu => state.cpu_queue.push_back(job),
            ExecutorClass::Io => state.io_queue.push_back(job),
        }
        drop(state);
        self.trace
            .record(trace_scope, SchedulerEventKind::Enqueue, spec.label.clone());
        self.shared.work_ready.notify_all();
        Ok(NativeTaskHandle {
            id: task,
            owner: spec.owner,
            trace_scope,
            stable_path: spec.stable_path,
            label: spec.label,
            receiver,
            trace: self.trace.clone(),
        })
    }
}

impl Drop for NativeScheduler {
    fn drop(&mut self) {
        {
            let mut state = self.shared.state.lock().expect("poisoned scheduler mutex");
            state.stopping = true;
        }
        self.shared.work_ready.notify_all();
        self.shared.capacity_ready.notify_all();
        for worker in self.cpu_workers.drain(..).chain(self.io_workers.drain(..)) {
            let _ = worker.join();
        }
    }
}

fn spawn_workers(
    shared: &Arc<SharedScheduler>,
    class: ExecutorClass,
    count: u32,
) -> Vec<JoinHandle<()>> {
    (0..count.max(1))
        .map(|index| {
            let shared = Arc::clone(shared);
            let worker_id = match class {
                ExecutorClass::Cpu => u64::from(index) + 1,
                ExecutorClass::Io => (1_u64 << 32) + u64::from(index) + 1,
            };
            thread::Builder::new()
                .name(format!(
                    "cem-ml-{}-{index}",
                    if class == ExecutorClass::Cpu {
                        "cpu"
                    } else {
                        "io"
                    }
                ))
                .spawn(move || worker_loop(shared, class, worker_id))
                .expect("native scheduler worker spawn")
        })
        .collect()
}

fn worker_loop(shared: Arc<SharedScheduler>, class: ExecutorClass, worker_id: u64) {
    IS_EXECUTOR_THREAD.with(|flag| flag.set(true));
    loop {
        let job = {
            let mut state = shared.state.lock().expect("poisoned scheduler mutex");
            loop {
                if state.stopping {
                    return;
                }
                if let Some(job) = take_eligible_job(&mut state, class) {
                    shared.capacity_ready.notify_all();
                    break job;
                }
                state = shared
                    .work_ready
                    .wait(state)
                    .expect("poisoned scheduler mutex");
            }
        };
        let preflight = shared
            .control
            .check_scope(job.owner)
            .err()
            .map(ScheduleError::Control)
            .or_else(|| job.failed_dependency.map(ScheduleError::DependencyFailed));
        let result = (job.run)(worker_id, preflight);
        let mut state = shared.state.lock().expect("poisoned scheduler mutex");
        release_running(&mut state, &job.ancestors, class);
        match result {
            Ok(()) => {
                state.completed_tasks.insert(job.id);
            }
            Err(error) => {
                state.failed_tasks.insert(job.id, error);
            }
        }
        drop(state);
        // A control task becomes complete only after its scheduler permits are
        // released, so scoped failure delivery cannot race cleanup against a
        // still-accounted worker slot.
        let _ = shared.control.complete_task(job.id);
        shared.work_ready.notify_all();
        shared.capacity_ready.notify_all();
    }
}

fn take_eligible_job(state: &mut SchedulerState, class: ExecutorClass) -> Option<Job> {
    let position = {
        let queue = match class {
            ExecutorClass::Cpu => &state.cpu_queue,
            ExecutorClass::Io => &state.io_queue,
        };
        queue.iter().position(|job| {
            job.dependencies.iter().all(|dependency| {
                state.completed_tasks.contains(dependency)
                    || state.failed_tasks.contains_key(dependency)
            }) && permits_available(state, &job.ancestors, class)
        })?
    };
    let mut job = match class {
        ExecutorClass::Cpu => state.cpu_queue.remove(position),
        ExecutorClass::Io => state.io_queue.remove(position),
    }
    .expect("selected queued job exists");
    job.failed_dependency = job
        .dependencies
        .iter()
        .find(|dependency| state.failed_tasks.contains_key(dependency))
        .copied();
    decrement_queued(state, job.queue_scope, class);
    acquire_running(state, &job.ancestors, class);
    Some(job)
}

fn runtime_ancestors(
    state: &SchedulerState,
    scope: ExecutionScopeId,
) -> Result<Vec<ExecutionScopeId>, ScheduleError> {
    let mut ancestors = Vec::new();
    let mut cursor = Some(scope);
    while let Some(id) = cursor {
        let node = state
            .scopes
            .get(&id)
            .ok_or(ControlError::UnknownScope(id))?;
        ancestors.push(id);
        cursor = node.parent;
    }
    Ok(ancestors)
}

fn permits_available(
    state: &SchedulerState,
    ancestors: &[ExecutionScopeId],
    class: ExecutorClass,
) -> bool {
    ancestors.iter().all(|scope| {
        let node = state.scopes.get(scope).expect("known scheduler scope");
        match class {
            ExecutorClass::Cpu => node.running_cpu < node.policy.cpu_workers,
            ExecutorClass::Io => node.running_io < node.policy.io_streams,
        }
    })
}

fn acquire_running(
    state: &mut SchedulerState,
    ancestors: &[ExecutionScopeId],
    class: ExecutorClass,
) {
    for scope in ancestors {
        let node = state.scopes.get_mut(scope).expect("known scheduler scope");
        match class {
            ExecutorClass::Cpu => node.running_cpu += 1,
            ExecutorClass::Io => node.running_io += 1,
        }
    }
}

fn release_running(
    state: &mut SchedulerState,
    ancestors: &[ExecutionScopeId],
    class: ExecutorClass,
) {
    for scope in ancestors {
        let node = state.scopes.get_mut(scope).expect("known scheduler scope");
        match class {
            ExecutorClass::Cpu => node.running_cpu = node.running_cpu.saturating_sub(1),
            ExecutorClass::Io => node.running_io = node.running_io.saturating_sub(1),
        }
    }
}

fn queued(node: &RuntimeScope, class: ExecutorClass) -> u32 {
    match class {
        ExecutorClass::Cpu => node.queued_cpu,
        ExecutorClass::Io => node.queued_io,
    }
}

fn increment_queued(state: &mut SchedulerState, scope: ExecutionScopeId, class: ExecutorClass) {
    let node = state.scopes.get_mut(&scope).expect("known scheduler scope");
    match class {
        ExecutorClass::Cpu => node.queued_cpu += 1,
        ExecutorClass::Io => node.queued_io += 1,
    }
}

fn decrement_queued(state: &mut SchedulerState, scope: ExecutionScopeId, class: ExecutorClass) {
    let node = state.scopes.get_mut(&scope).expect("known scheduler scope");
    match class {
        ExecutorClass::Cpu => node.queued_cpu = node.queued_cpu.saturating_sub(1),
        ExecutorClass::Io => node.queued_io = node.queued_io.saturating_sub(1),
    }
}

enum CapacityDecision {
    Available(ExecutionScopeId),
    Wait,
    Reject(u32),
}

fn queue_capacity_decision(
    state: &SchedulerState,
    owner: ExecutionScopeId,
    class: ExecutorClass,
) -> Result<CapacityDecision, ScheduleError> {
    let mut cursor = owner;
    loop {
        let node = state
            .scopes
            .get(&cursor)
            .ok_or(ControlError::UnknownScope(cursor))?;
        if queued(node, class) < node.policy.queue_size {
            return Ok(CapacityDecision::Available(cursor));
        }
        match node.policy.overflow {
            OverflowPolicy::Reject => return Ok(CapacityDecision::Reject(node.policy.queue_size)),
            OverflowPolicy::Block => return Ok(CapacityDecision::Wait),
            OverflowPolicy::SpillToParent => match node.parent {
                Some(parent) => cursor = parent,
                None => return Ok(CapacityDecision::Reject(node.policy.queue_size)),
            },
        }
    }
}

fn wait_for_queue_capacity<'a>(
    shared: &'a Arc<SharedScheduler>,
    mut state: std::sync::MutexGuard<'a, SchedulerState>,
    owner: ExecutionScopeId,
    class: ExecutorClass,
    _label: &str,
) -> Result<ExecutionScopeId, ScheduleError> {
    loop {
        match queue_capacity_decision(&state, owner, class)? {
            CapacityDecision::Available(scope) => {
                increment_queued(&mut state, scope, class);
                return Ok(scope);
            }
            CapacityDecision::Reject(capacity) => {
                return Err(ScheduleError::QueueCapacity(ControlFailure {
                    operation_id: shared.control.operation_id(),
                    affected_scope: owner,
                    cause: ControlCause::QueueCapacityExceeded { capacity },
                    source_map: None,
                }));
            }
            CapacityDecision::Wait => {
                if IS_EXECUTOR_THREAD.with(std::cell::Cell::get) {
                    return Err(ScheduleError::ExecutorThreadWouldBlock);
                }
                drop(state);
                shared.control.check_scope(owner)?;
                state = shared.state.lock().expect("poisoned scheduler mutex");
                let (next, _) = shared
                    .capacity_ready
                    .wait_timeout(state, CONTROL_POLL_INTERVAL)
                    .expect("poisoned scheduler mutex");
                state = next;
                if state.stopping {
                    return Err(ScheduleError::Shutdown);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation_control::{ExecutionScopeKind, ExecutionScopeRegistration};
    use crate::scheduler::AbortSignal;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    fn policy(cpu: u32, queue: u32, io: u32, overflow: OverflowPolicy) -> ScopePolicy {
        ScopePolicy::host_root()
            .with_cpu_workers(cpu)
            .with_queue_size(queue)
            .with_io_streams(io)
            .with_overflow(overflow)
    }

    fn scheduler(root: ScopePolicy) -> NativeScheduler {
        let control = OperationControl::with_root_policy(AbortSignal::new(), root).unwrap();
        NativeScheduler::new(control, SchedulerTrace::new()).unwrap()
    }

    fn update_max(maximum: &AtomicUsize, candidate: usize) {
        let mut current = maximum.load(Ordering::Acquire);
        while candidate > current {
            match maximum.compare_exchange_weak(
                current,
                candidate,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
    }

    #[test]
    fn native_workers_execute_simultaneously_up_to_root_cap() {
        let scheduler = scheduler(policy(3, 8, 2, OverflowPolicy::Reject));
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for index in 0..6 {
            let active = Arc::clone(&active);
            let maximum = Arc::clone(&maximum);
            handles.push(
                scheduler
                    .submit_cpu(
                        ScheduledTaskSpec::new(
                            scheduler.control().root_scope(),
                            TaskPath::root(index),
                            format!("cpu-{index}"),
                        ),
                        move || {
                            let now = active.fetch_add(1, Ordering::AcqRel) + 1;
                            update_max(&maximum, now);
                            thread::sleep(Duration::from_millis(25));
                            active.fetch_sub(1, Ordering::AcqRel);
                            index
                        },
                    )
                    .unwrap(),
            );
        }
        let committed = commit_in_stable_order(handles).unwrap();
        assert_eq!(maximum.load(Ordering::Acquire), 3);
        assert_eq!(
            committed
                .into_iter()
                .map(|result| result.value)
                .collect::<Vec<_>>(),
            (0..6).collect::<Vec<_>>()
        );
    }

    #[test]
    fn child_logical_permits_constrain_without_creating_another_pool() {
        let scheduler = scheduler(policy(4, 8, 2, OverflowPolicy::Reject));
        let child_policy = policy(1, 8, 2, OverflowPolicy::Reject);
        let child = scheduler
            .control()
            .register_scope(
                scheduler.control().root_scope(),
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Document,
                    "child",
                    child_policy,
                ),
            )
            .unwrap();
        scheduler.register_scope(child).unwrap();
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for index in 0..3 {
            let active = Arc::clone(&active);
            let maximum = Arc::clone(&maximum);
            handles.push(
                scheduler
                    .submit_cpu(
                        ScheduledTaskSpec::new(
                            child,
                            TaskPath::root(index),
                            format!("child-{index}"),
                        ),
                        move || {
                            let now = active.fetch_add(1, Ordering::AcqRel) + 1;
                            update_max(&maximum, now);
                            thread::sleep(Duration::from_millis(15));
                            active.fetch_sub(1, Ordering::AcqRel);
                        },
                    )
                    .unwrap(),
            );
        }
        commit_in_stable_order(handles).unwrap();
        assert_eq!(maximum.load(Ordering::Acquire), 1);
        assert_eq!(scheduler.cpu_worker_count(), 4);
    }

    #[test]
    fn physical_completion_is_committed_in_stable_path_order() {
        let scheduler = scheduler(policy(3, 8, 2, OverflowPolicy::Reject));
        let mut handles = Vec::new();
        for (path, delay) in [(2, 0), (0, 30), (1, 10)] {
            handles.push(
                scheduler
                    .submit_cpu(
                        ScheduledTaskSpec::new(
                            scheduler.control().root_scope(),
                            TaskPath::root(path),
                            format!("task-{path}"),
                        ),
                        move || {
                            thread::sleep(Duration::from_millis(delay));
                            format!("value-{path}")
                        },
                    )
                    .unwrap(),
            );
        }
        let committed = commit_in_stable_order(handles).unwrap();
        assert_eq!(
            committed
                .iter()
                .map(|result| result.stable_path.segments()[0])
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            committed
                .into_iter()
                .map(|result| result.value)
                .collect::<Vec<_>>(),
            vec!["value-0", "value-1", "value-2"]
        );
    }

    #[test]
    fn io_executor_is_independent_from_saturated_cpu_workers() {
        let scheduler = scheduler(policy(1, 8, 1, OverflowPolicy::Reject));
        let cpu = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(scheduler.control().root_scope(), TaskPath::root(0), "cpu"),
                || thread::sleep(Duration::from_millis(80)),
            )
            .unwrap();
        let started = Instant::now();
        let io = scheduler
            .submit_io(
                ScheduledTaskSpec::new(scheduler.control().root_scope(), TaskPath::root(1), "io"),
                || "io-complete",
            )
            .unwrap();
        assert_eq!(io.join().unwrap().value, "io-complete");
        assert!(started.elapsed() < Duration::from_millis(60));
        cpu.join().unwrap();
    }

    #[test]
    fn worker_panic_is_a_typed_failure_and_pool_remains_usable() {
        let scheduler = scheduler(policy(1, 8, 1, OverflowPolicy::Reject));
        let failed = scheduler
            .submit_cpu::<(), _>(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(0),
                    "panic",
                ),
                || panic!("fixture worker panic"),
            )
            .unwrap();
        let error = failed.join().unwrap_err();
        assert_eq!(error.code(), "worker-failure");
        let healthy = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(1),
                    "healthy",
                ),
                || 42,
            )
            .unwrap();
        assert_eq!(healthy.join().unwrap().value, 42);
    }

    #[test]
    fn reject_overflow_returns_typed_queue_failure() {
        let scheduler = scheduler(policy(1, 1, 1, OverflowPolicy::Reject));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let running = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(0),
                    "running",
                ),
                move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
            .unwrap();
        started_rx.recv().unwrap();
        let queued = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(1),
                    "queued",
                ),
                || (),
            )
            .unwrap();
        let error = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(2),
                    "rejected",
                ),
                || (),
            )
            .unwrap_err();
        assert_eq!(error.code(), "queue-capacity-exceeded");
        release_tx.send(()).unwrap();
        running.join().unwrap();
        queued.join().unwrap();
    }

    #[test]
    fn block_overflow_waits_for_capacity_on_the_submitter() {
        let scheduler = scheduler(policy(1, 1, 1, OverflowPolicy::Block));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let running = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(0),
                    "running",
                ),
                move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
            .unwrap();
        started_rx.recv().unwrap();
        let queued = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(1),
                    "queued",
                ),
                || (),
            )
            .unwrap();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(25));
            release_tx.send(()).unwrap();
        });
        let started = Instant::now();
        let admitted = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(2),
                    "blocked",
                ),
                || (),
            )
            .unwrap();
        assert!(started.elapsed() >= Duration::from_millis(15));
        running.join().unwrap();
        queued.join().unwrap();
        admitted.join().unwrap();
    }

    #[test]
    fn cancellation_wakes_a_blocked_queue_admission() {
        let scheduler = scheduler(policy(1, 1, 1, OverflowPolicy::Block));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let running = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(0),
                    "running",
                ),
                move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
            .unwrap();
        started_rx.recv().unwrap();
        let _queued = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(1),
                    "queued",
                ),
                || (),
            )
            .unwrap();
        let abort = scheduler.control().abort_signal().clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(25));
            abort.abort();
        });
        let error = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(2),
                    "blocked",
                ),
                || (),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ScheduleError::Control(ControlError::Triggered(_))
        ));
        release_tx.send(()).unwrap();
        running.join().unwrap();
    }

    #[test]
    fn spill_to_parent_uses_parent_queue_but_retains_child_permits() {
        let scheduler = scheduler(policy(2, 3, 1, OverflowPolicy::Reject));
        let child_policy = policy(1, 1, 1, OverflowPolicy::SpillToParent);
        let child = scheduler
            .control()
            .register_scope(
                scheduler.control().root_scope(),
                ExecutionScopeRegistration::inherited(
                    ExecutionScopeKind::Document,
                    "spill-child",
                    child_policy,
                ),
            )
            .unwrap();
        scheduler.register_scope(child).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let running = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(child, TaskPath::root(0), "running"),
                move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
            .unwrap();
        started_rx.recv().unwrap();
        let local = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(child, TaskPath::root(1), "local"),
                || 1,
            )
            .unwrap();
        let spilled = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(child, TaskPath::root(2), "spilled"),
                || 2,
            )
            .unwrap();
        release_tx.send(()).unwrap();
        running.join().unwrap();
        assert_eq!(local.join().unwrap().value, 1);
        assert_eq!(spilled.join().unwrap().value, 2);
    }

    #[test]
    fn deadline_wakes_a_blocked_queue_admission() {
        let root = policy(1, 1, 1, OverflowPolicy::Block).with_timeout_ms(Some(30));
        let scheduler = scheduler(root);
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let running = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(0),
                    "running",
                ),
                move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                },
            )
            .unwrap();
        started_rx.recv().unwrap();
        let _queued = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(1),
                    "queued",
                ),
                || (),
            )
            .unwrap();
        let error = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(2),
                    "deadline-blocked",
                ),
                || (),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ScheduleError::Control(ControlError::Triggered(ControlFailure {
                cause: ControlCause::TimeoutExceeded { .. },
                ..
            }))
        ));
        release_tx.send(()).unwrap();
        running.join().unwrap();
    }

    #[test]
    fn declared_dependency_completes_before_dependent_dispatch() {
        let scheduler = scheduler(policy(2, 4, 1, OverflowPolicy::Reject));
        let parent_finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let parent_flag = Arc::clone(&parent_finished);
        let parent = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(0),
                    "parent",
                ),
                move || {
                    thread::sleep(Duration::from_millis(25));
                    parent_flag.store(true, Ordering::Release);
                },
            )
            .unwrap();
        let dependency = parent.id();
        let child_flag = Arc::clone(&parent_finished);
        let child = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(1),
                    "child",
                )
                .with_dependencies(vec![dependency]),
                move || child_flag.load(Ordering::Acquire),
            )
            .unwrap();
        parent.join().unwrap();
        assert!(child.join().unwrap().value);
    }

    #[test]
    fn failed_dependency_prevents_dependent_work_from_running() {
        let scheduler = scheduler(policy(2, 4, 1, OverflowPolicy::Reject));
        let parent = scheduler
            .submit_cpu::<(), _>(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(0),
                    "failed-parent",
                ),
                || panic!("fixture dependency failure"),
            )
            .unwrap();
        let dependency = parent.id();
        let child_ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let child_flag = Arc::clone(&child_ran);
        let child = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scheduler.control().root_scope(),
                    TaskPath::root(1),
                    "dependent",
                )
                .with_dependencies(vec![dependency]),
                move || child_flag.store(true, Ordering::Release),
            )
            .unwrap();

        assert!(matches!(
            parent.join().unwrap_err(),
            ScheduleError::WorkerFailure(_)
        ));
        assert!(matches!(
            child.join().unwrap_err(),
            ScheduleError::DependencyFailed(failed) if failed == dependency
        ));
        assert!(!child_ran.load(Ordering::Acquire));
    }

    #[test]
    fn randomized_completion_keeps_values_and_trace_canonical() {
        let run = |delays: [u64; 4]| {
            let trace = SchedulerTrace::new();
            let control = OperationControl::with_root_policy(
                AbortSignal::new(),
                policy(4, 8, 1, OverflowPolicy::Reject),
            )
            .unwrap();
            let scheduler = NativeScheduler::new(control, trace.clone()).unwrap();
            let mut handles = Vec::new();
            for (index, delay) in delays.into_iter().enumerate() {
                handles.push(
                    scheduler
                        .submit_cpu(
                            ScheduledTaskSpec::new(
                                scheduler.control().root_scope(),
                                TaskPath::root(index as u32),
                                format!("task-{index}"),
                            ),
                            move || {
                                thread::sleep(Duration::from_millis(delay));
                                format!("canonical-{index}")
                            },
                        )
                        .unwrap(),
                );
            }
            let values = commit_in_stable_order(handles)
                .unwrap()
                .into_iter()
                .map(|result| result.value)
                .collect::<Vec<_>>();
            (values, trace.snapshot())
        };
        assert_eq!(run([35, 5, 20, 0]), run([0, 30, 5, 20]));
    }
}
