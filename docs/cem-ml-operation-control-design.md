# CEM-ML Operation Control, Scheduling, and Debugger Design

**Status:** Canonical Phase 2.5 design, accepted 2026-08-11.

This document defines the common CEM-ML contract for worker-pool execution,
cooperative cancellation, resource-limit enforcement, pause/resume, debugger
inspection, and host control. It extends the asynchronous and scoped resource
requirements in [`cem-ml-ac.md`](cem-ml-ac.md), the normalized scope policy in
[`cem-ml-phase2-run-config-contract.md`](cem-ml-phase2-run-config-contract.md),
and the versioned host envelope in
[`cem-ml-deployment-contract.md`](cem-ml-deployment-contract.md).

The common Rust engine owns these semantics. Native CLI, Node, browser-worker,
WASM, Studio, and Debug Adapter Protocol (DAP) integrations are host adapters;
they must not implement independent cancellation, scheduling, or debugger
state machines.

## 1. Outcome and invariants

CEM-ML operations execute as trees of bounded scopes and tasks. A host receives
one awaitable operation handle, can observe a bounded repeatable event stream,
can cancel the whole operation or one execution-scope subtree, and can request
an all-stop debugger snapshot. Resource limits use the same cooperative unwind
machinery as cancellation but remain typed policy failures.

The following rules are invariant:

1. Exactly one terminal outcome is published for every accepted operation.
2. An explicit root cancellation is terminal and cannot be caught by a child
   scope.
3. A scoped cancellation unwinds only the selected execution scope and its
   descendants before entering the normal error-boundary mechanism.
4. Stack-depth, memory, and timeout exhaustion return a typed failed outcome,
   not a host-cancelled outcome.
5. Cancellation and resource enforcement are present in every supported build.
   Only debugger-specific pause, stepping, snapshots, inspection, and transports
   may be compiled out.
6. Pause is cooperative. A stopped event is published only after an all-stop
   rendezvous has produced a coherent suspended state.
7. Cancellation takes precedence over pause, including while tasks are parked.
8. Parallel scheduling must not change result bytes, diagnostics, source maps,
   artifact identities, or canonical report ordering.
9. No host may expose raw Rust pointers, native references, WASM memory views,
   or unbounded retained values through operation or debugger APIs.

## 2. Verified starting point

The implementation at acceptance provides a useful foundation but not the
complete contract in this document:

- `scheduler::WorkerPool` owns a bounded FIFO queue but invokes work
  sequentially. `ScopePolicy.cpu_workers` is currently policy metadata rather
  than a real parallel pool size.
- `scheduler::AbortSignal` is one clone-shared operation-wide atomic boolean
  with an optional source map. It has no target scope, cause, deadline, pause
  state, or terminal-outcome ownership.
- scheduler policy already describes CPU workers, queue size, independent I/O
  streams, memory bytes, and plugin time, with constrain-only inheritance.
  Memory is explicitly informational today.
- normalized run config recognizes operation time budgets, but most current
  checks compare elapsed time after a phase. They diagnose overuse without
  interrupting the running work.
- parser/schema, scheduler, run-config, query, registry, and plugin scope IDs
  are distinct types or number spaces. None is a universal runtime-control ID.
- source maps and error-boundary bubbling already provide the provenance and
  recovery model needed for scoped failure, but there is no pause, breakpoint,
  stack/scope inspection, or DAP implementation.

Migration must preserve the working cancellation checks while replacing these
partial mechanisms incrementally.

## 3. Runtime identity model

### 3.1 Stable operation-local identities

The runtime introduces opaque IDs with no pointer or worker-index semantics:

```text
OperationId       stable within one initialized host session
ExecutionScopeId  stable for the lifetime of one operation
TaskId            stable logical task identity within one operation
WorkerId          ephemeral physical executor identity
StopId            one suspended-state generation
BreakpointId      one installed pause trigger
SubscriptionId    one event-stream consumer
```

IDs are bounded integers or bounded opaque strings at host boundaries. A
request using an unknown, stale, or foreign ID fails without changing execution
state. `WorkerId` is telemetry only and must never contribute to output,
source-map, UID, cache, or artifact identity.

### 3.2 Execution-scope tree

Every operation owns one `ExecutionScopeTree`. Its root represents the whole
operation. Parser roots, embedded handoffs, schema contexts, query evaluation,
template calls, transform stages, resolver calls, plugins, and output staging
register child execution scopes as work becomes live.

```text
ExecutionScope {
  id: ExecutionScopeId
  parent?: ExecutionScopeId
  kind: operation | document | parse | schema | handoff | query |
        template | transform | resolver | plugin | output
  label: bounded string
  state: queued | running | waiting | parked | unwinding | completed
  sourceLocation?: SourceLocation
  semanticIdentities: ScopeIdentityMap
  effectivePolicy: EffectiveControlPolicy
}
```

`ScopeIdentityMap` records mappings to existing run-config, scheduler,
schema/parser, query, registry, and plugin scope identities. Those semantic IDs
remain owned by their subsystems. An `ExecutionScopeId` is the only ID accepted
as a direct runtime-control target.

The tree is operation-local and append-mostly. Completed nodes retain bounded
metadata until the operation result and retained event subscribers no longer
need them; heavy values and frames are released as soon as their scope closes.

### 3.3 Source selectors

A source line is a location, not a scope ID. A source selector contains:

```text
SourceSelector {
  sourceUri: canonical logical URI
  line: positive one-based line
  column?: positive one-based Unicode scalar column
  endLine?: positive one-based line
  endColumn?: positive one-based Unicode scalar column
  byteRange?: { start, end }
  scope?: ExecutionScopeId
}
```

Rust-native APIs may use byte ranges internally. Host and DAP projections carry
line/column coordinates and preserve the canonical URI. The DAP adapter converts
columns at its boundary to DAP's UTF-16-code-unit convention.

The runtime resolves a selector against registered safe-point locations and
live execution scopes. A unique match returns its executable location. No match
returns `location-not-executable`. Multiple matches return `location-ambiguous`
with bounded candidate scope/location records; callers may retry with a scope
filter. The runtime never guesses a scope from a bare line number.

## 4. Scheduler and worker pools

### 4.1 Logical scheduler and physical executors

The common engine owns a deterministic logical scheduler. A host supplies a
physical executor implementation:

| Host                     | Physical executor                                                             |
| ------------------------ | ----------------------------------------------------------------------------- |
| Native Rust / native CLI | Fixed bounded set of OS worker threads                                        |
| Node                     | Bounded Node worker-thread pool, one initialized runtime per worker           |
| Browser                  | Bounded dedicated-worker pool, one WASM instance per worker                   |
| Browser fallback         | One dedicated worker, then main-thread WASM only when workers are unavailable |

Shared-memory WASM threads are an optional optimization. Correctness, control,
and inspection must work with message-passing workers and may not require
`SharedArrayBuffer` or cross-origin isolation.

The root effective `cpuWorkers` value sizes the operation's physical capacity,
bounded by host policy. A child does not allocate a new set of OS threads. Each
scope receives logical concurrency permits capped by its effective
`cpuWorkers`; a task must hold permits for its scope and all ancestors before it
can dispatch. This implements per-scope limits without multiplying physical
threads for every nested scope.

`SchedulerConfig.threadPool`, when present, selects a host-registered executor
identity; it never names an engine-created global singleton. An unknown identity
fails preflight. `maxParallelDocuments` adds a document-class permit bound and
cannot raise the root `cpuWorkers` cap. Native and Node root defaults are
`min(available_parallelism, 8)`, floored at one; browser defaults are
`min(navigator.hardwareConcurrency, 8)`, floored at one.

### 4.2 Tasks and ownership

Deferrable work becomes a `ScheduledTask` with a stable path assigned by its
owner before dispatch:

```text
ScheduledTask {
  id: TaskId
  owner: ExecutionScopeId
  stablePath: TaskPath
  class: cpu | io-continuation | control
  dependencies: TaskId[]
  commitKey: DeterministicCommitKey
}
```

Tasks may move between physical workers. Debugger threads therefore map to
logical `TaskId` values, not OS thread IDs. Worker association is exposed as
additional telemetry.

CPU work uses the CPU queue. External filesystem, URL, and host resolver work
uses the independent bounded I/O permit queue and does not retain a CPU permit
while awaiting an external result. The continuation reacquires CPU permits
before re-entering engine code.

### 4.3 Queue and overflow policy

Queue size and overflow policy remain scope-policy fields with constrain-only
inheritance. Their operational meanings are:

- `reject`: fail the submitted subtree with `queue-capacity-exceeded`;
- `block`: suspend the submitting task cooperatively until capacity or a
  control failure becomes available; it must not block an OS worker thread;
- `spill-to-parent`: enqueue against the parent's queue and permits while
  retaining the child scope as semantic owner. At the root it degrades to
  `reject`.

Queued work observes cancellation, timeout, and pause before dispatch. Queue
wait contributes to the owning scope's active-time timeout after that scope has
been activated.

### 4.4 Determinism and commit barrier

Parallel tasks may complete physically in any order. They publish only staged
task results. An ordered join/commit barrier consumes those results by stable
task path and declared dependency order.

Canonical diagnostics, source maps, artifacts, report events, and output bytes
are emitted at that barrier. Logical enqueue, dispatch eligibility, join, and
commit sequences are deterministic. Actual worker start/completion order and
durations may be recorded in a separate debug telemetry channel explicitly
marked non-canonical; consumers must not compare that telemetry for byte-for-byte
reproducibility.

Worker panic, loss, or protocol corruption yields a typed runtime failure. A
host may retry only a task declared pure and replayable and only with the same
stable path, inputs, policy, and resource charges. Otherwise the affected scope
unwinds; a root worker failure is terminal `fatal` when engine invariants can no
longer be trusted.

## 5. Unified operation control

### 5.1 State machine

The common control core replaces the boolean-only model:

```text
OperationState = running | pause-requested | stopped | resuming |
                 cancelling | completed

ScopeControlState = active | pause-requested | parked |
                    cancelling | unwinding | completed
```

Only these operation transitions are valid:

```text
running ---------> pause-requested -> stopped -> resuming -> running
   |                     |               |          |
   +---------------------+---------------+----------+-> cancelling
   +--------------------------------------------------> completed
cancelling -------------------------------------------> completed
```

Terminal completion is guarded by one compare-and-set owner. Concurrent cancel,
failure, worker-loss, and normal-completion races produce one terminal outcome;
later terminal attempts become trace events only.

### 5.2 Control cause

Every unwind carries a typed cause:

```text
ControlCause =
  HostCancellation { reason? }
  Superseded { revision }
  StackDepthExceeded { observed, limit }
  MemoryExceeded { requested, charged, limit }
  TimeoutExceeded { activeElapsed, limit }
  QueueCapacityExceeded { capacity }
  WorkerFailure { worker?, restartable }
  InternalFailure { diagnosticCode }
```

`HostCancellation` and `Superseded` project to a `cancelled` terminal status
when they reach the operation root. Resource and queue causes project to
`failed`. `WorkerFailure` is `failed` when safe scope-local unwind is possible
and `fatal` when runtime integrity is unknown.

### 5.3 Root cancellation

`cancel()` without a target selects the root execution scope. The request is
idempotent. The first accepted request:

1. records the bounded reason and initiating source location, if any;
2. wakes paused tasks and blocked queue/I/O waiters;
3. prevents new task and resolver dispatch;
4. marks all scopes and tasks for cooperative unwind;
5. discards unpublished staged results;
6. waits for cleanup and atomic-region exit within the host hard-cancel grace
   period; and
7. publishes exactly one cancelled result.

No error boundary may recover a root cancellation. A hard worker/process
termination is a host fallback after the grace period, not normal cancellation
semantics. `hardCancelGraceMs` is negotiated during initialization, defaults to
2,000 ms, and is bounded to 10..30,000 ms. The effective value is disclosed in
capabilities and is not a serializable document/run-config property.

### 5.4 Scoped cancellation

`cancel(scope)` selects one live execution scope and all descendants. It does
not directly cancel ancestors or unrelated siblings.

After descendant tasks stop and owned resources are released, the runtime emits
a typed `ScopedCancellation` at the cancelled scope's nearest error boundary.
Cancellation cannot be hidden. A boundary may recover only through an explicit
cancellation handler that returns a type-compatible replacement result. There
is no implicit empty string, empty sequence, null artifact, or partially built
tree fallback.

If no handler accepts the cancellation, or the effective policy is fail-fast,
the failure bubbles to the parent boundary. Reaching the root converts it to
the operation's cancelled terminal outcome. A recovered scoped cancellation is
retained in diagnostics and control events even when the overall operation
succeeds.

Repeated cancellation of the same subtree is idempotent. Cancelling an already
completed scope returns `scope-completed`; cancelling a scope from another
operation returns `foreign-scope`; neither affects the operation.

#### 5.4.1 Runtime error-boundary contract

An execution scope may register one optional runtime error-boundary descriptor.
The descriptor has a bounded subsystem owner, a stable subsystem-owned result-
contract identifier, and either `recover` with an explicit accepted set of
control-cause kinds or `fail-fast`. This metadata selects delivery; it does not
serialize, erase, or interpret a replacement value. The operation root is an
implicit non-recoverable boundary.

Failure delivery starts only after every task owned by the selected subtree has
completed and the control core has run owned cleanup descendant-first and LIFO
within each scope. Stack frames, memory charges, permits, staged values, and
registered cleanup actions must therefore be released before a handler runs.
The nearest surviving `recover` boundary that accepts the cause receives a
single-use delivery token. A `fail-fast` boundary or one that does not accept
the cause is skipped while bubbling toward the root.

The owning subsystem executes its typed handler and validates the replacement
against the descriptor's result contract before asking the control core to
record recovery. The common core never owns a universal recovery-value format.
An incompatible or declined replacement leaves the failure pending so it can
bubble to the next boundary. Each token can be recovered or declined once;
each selected subtree records exactly one final recovered or root-escalated
settlement. Recovery completes the failed subtree rather than retrying it, and
unaffected siblings remain eligible to run.

Cleanup failure replaces the original recoverable delivery with a fatal
`internal-failure` rooted at `cem.control.cleanup_failed`; a partially cleaned
subtree is never exposed to a recovery handler.

### 5.5 Safe points and atomic regions

All long-running common code must reach cooperative safe points at bounded work
intervals. Required locations include:

- tokenizer chunks, parser events, schema transitions, and embedded handoffs;
- scheduler enqueue/dequeue/join and task dependency boundaries;
- resolver request dispatch and response acceptance;
- query IR nodes, loops, ranges, path steps, predicates, function calls, and
  materialization budgets;
- template calls, expression evaluation, render nodes, output chunks, and
  transform stages;
- plugin boundaries and plugin-supplied cooperative callbacks;
- report, artifact, and output staging boundaries.

A safe point checks root and ancestor control state, timeout state, and pause
generation. Cancellation checks remain compiled in all builds. Debugger frame
capture is performed only when the `debug-control` feature is built and runtime
debugging is active.

The first-release common poller uses a fixed maximum interval of 64 bounded work
units. A work unit is the smallest already-bounded operation owned by the path,
such as one token/event, IR node or loop item, path step, render node/character,
or staged artifact. Paths may check more frequently. Operation entry, host-call
dispatch, host-result acceptance, and final result acceptance are forced safe
points and do not wait for the quota. Consequently, a control failure can leave
an internal prefix under construction, but that prefix is discarded before it
crosses a result, resolver, plugin, transform, artifact, or output boundary.

Host calls and publication may declare a short atomic region. Pause waits for
an atomic region to exit. Cancellation prevents its result from being accepted
but cannot undo an irreversible external side effect already started. Resolver
and output contracts must therefore use cancellation-aware host APIs where
available and staged transactional output for multi-destination publication.
Atomic regions may not contain unbounded evaluator loops.

## 6. Resource enforcement

### 6.1 Policy inheritance

CPU workers, queue size, I/O streams, memory, stack depth, and timeout are typed
fields in `EffectiveControlPolicy`. Resource ceilings inherit downward and may
only be constrained. Error-handling policy may be locally permissive only where
an ancestor explicitly permits it; it cannot relax an ancestor resource cap.

The authored `ScopeConfig.budgets` surface gains canonical `stackDepth` and
`timeoutMs` keys while accepting normalized aliases `stack`, `maxStackDepth`,
`timeout`, and `scopeTimeoutMs`. Existing operation keys such as `parseMs`,
`validateMs`, `convertMs`, and `pluginMs` create child-scope deadlines; their
effective limit is the minimum of the operation-specific value and inherited
`timeoutMs`. The root logical stack default is 256 frames. Existing specialized
limits such as template or SCSS recursion caps remain stricter child limits.
There is no implicit general timeout; a host or scope must declare one. Memory
continues to use the effective root/scope policy defaults.

An attempted cap increase fails during plan/scope creation with
`cem.a.cap_relaxation_denied`. A zero or invalid cap fails normalization before
work begins. The effective child limit for every resource is therefore no
greater than each ancestor limit.

### 6.2 Logical stack depth

`stackDepth` counts engine logical frames, not native machine stack bytes.
Parser recursion, embedded handoffs, query calls, template calls, transform
calls, plugin re-entry, and recursive value traversal must enter through a
bounded frame guard. The guard charges the scope and ancestors before invoking
deeper work and releases on unwind.

Crossing the limit fails before the next recursive call with
`stack-depth-exceeded`, preserving the active call/source-map stack. Engine code
must eliminate or guard uncontrolled native recursion. Stack overflow inside
uncooperative third-party or host code is a worker failure, not a recoverable
limit event.

### 6.3 Memory accounting

Accountable engine allocations use scope-owned arenas, buffers, collections,
artifact stores, and retained-handle stores that acquire `MemoryPermit` values.
Charging is atomic across the owner scope and all ancestors. If any effective
cap would be exceeded, the entire charge rolls back and the limiting scope
receives `memory-exceeded` before allocation or growth.

Permits release on buffer shrink, handle disposal, scope unwind, or result
ownership transfer. Shared immutable data is charged once to its owning scope;
borrowers retain a reference charge rather than duplicating its byte size.
Cross-worker transfers charge the receiver before the sender releases ownership
so the limit cannot be bypassed during transit.

The first implementation must enumerate all accounted stores and report an
`accountedBytes`/`unaccountedHostBytes` capability statement. It may not claim a
hard process-wide heap guarantee until the allocator and host runtimes can
enforce one. OS/WASM out-of-memory remains a fatal worker failure.

### 6.4 Active-time deadlines

A scope timeout starts when the execution scope becomes active and includes CPU
execution, queue waits after activation, child work, and awaited resolver or
plugin time. Time deliberately spent in a completed debugger stop is subtracted
from every live scope's active elapsed time.

The scheduler registers a monotonic deadline for each timed scope. The
effective deadline is the earliest of its own deadline and all ancestors.
Deadline expiry marks that scope subtree with `timeout-exceeded`, wakes queued
or waiting tasks, and rejects later results. Hosts should propagate remaining
deadlines to cancellable I/O. A synchronous host call that cannot be interrupted
is checked immediately on return and remains subject to the hard worker timeout
fallback.

Pause rendezvous time before the stopped state is reached still counts as
active time. Once all tasks are parked and the stopped event is committed, the
timeout clock freezes until resume begins. A timeout or cancellation that wins
before the rendezvous completes cancels the pending pause and no stale stopped
event is published.

### 6.5 Resource failure and error boundaries

A resource limit selects the scope whose effective cap was crossed. For
example, a child allocation that crosses the parent's remaining memory cap
fails the parent subtree, while a child-local stack limit fails only that child
subtree. The runtime first unwinds the selected subtree and releases its
charges, then delivers a typed `ResourceFailure` to the nearest surviving error
boundary.

An explicit resource-failure handler may return a type-compatible fallback
after unwind. It may not resume or retry the expired scope, ignore an ancestor
cap, or allocate a replacement that exceeds the boundary's remaining budget.
Without an accepting handler, the failure bubbles to the next boundary.
Reaching the operation root produces `failed` with the original typed cause.
Recovered failures remain visible in diagnostics and terminal metadata.

## 7. Pause, breakpoints, and stepping

### 7.1 Pause triggers

Pause is debugger control, separate from terminal cancellation. A host installs
a `PauseSpec`:

```text
PauseSpec {
  trigger: nextSafePoint | scopeEnter | scopeExit | sourceLocation |
           nextStep | stepIn | stepOut
  scope?: ExecutionScopeId
  source?: SourceSelector
  condition?: bounded side-effect-free expression
  hitCondition?: bounded hit-count expression
  persistent: boolean
}
```

`nextSafePoint` with no scope pauses on the next visible safe point anywhere in
the operation. With a scope, it triggers only in that scope subtree.
`scopeEnter`/`scopeExit` require a scope or a source selector that resolves
uniquely. Source breakpoints resolve to executable safe points as described in
section 3.3. Conditions execute under a separate small evaluation budget and
cannot perform I/O, mutate state, or suppress cancellation/resource checks.

Conditions are side-effect-free CEM-QL boolean expressions evaluated against
the candidate frame's lexical bindings plus read-only `$frame`, `$scope`, and
`$task` records. A condition type error marks the breakpoint invalid instead of
stopping execution. `hitCondition` accepts a positive decimal `N` (exactly the
Nth hit), `>=N`, or `%N` (every Nth hit); other forms are rejected during
breakpoint resolution. Hit counters are operation-local and reset only when the
breakpoint is replaced or removed.

Manual pause triggers are one-shot. Source, scope, and conditional breakpoints
may be persistent and can therefore produce multiple stopped events over one
operation.

### 7.2 All-stop rendezvous

When any trigger hits, the control core increments the pause generation and
requests all-stop:

1. the triggering task records its frame and parks;
2. running tasks park at their next safe point;
3. queued tasks are marked parked without dispatch;
4. I/O waits retain bounded request metadata; returned responses are held and
   not admitted into engine state;
5. tasks in atomic regions park immediately after leaving the region;
6. the scheduler verifies that every live task is parked, completed, or safely
   represented as an external wait; and
7. the runtime creates one immutable `SuspendedSnapshot` and emits `stopped`
   with `allThreadsStopped: true`.

The snapshot receives a fresh `StopId`. Object and variable references are
valid only for that stopped generation. An inspection request before step 7
returns `not-stopped` rather than a changing partial view.

### 7.3 Continue and stop ownership

Each stopped event carries a `StopToken { operationId, stopId }`. Continue
accepts that token or the pause-trigger handle whose current pending event owns
that token. A stale, foreign, already-consumed, or trigger-with-no-current-stop
argument fails without resuming anything.

Continue invalidates snapshot object references, releases held I/O responses,
emits one continued event, and resumes all tasks. Rust names this method
`resume` because `continue` is a language keyword; JavaScript and wire adapters
expose `continue`.

### 7.4 Stepping

`nextStep`, `stepIn`, and `stepOut` operate from a valid stop and select one
logical task/frame. Unrelated tasks remain parked. The scheduler may run the
selected task and only the dependency closure required to advance it. Hidden
dependency work remains subject to breakpoints, cancellation, and limits.

- `nextStep` stops at the next visible safe point in the selected frame;
- `stepIn` stops at the first visible child frame, or behaves as `nextStep` if
  no child is entered;
- `stepOut` runs until the selected frame exits and stops in its caller.

When the step completes, the runtime again creates an all-stop snapshot. If a
different persistent breakpoint, cancellation, limit, or fatal failure wins
first, that cause owns the next event. Reverse execution and frame restart are
not supported by this design.

## 8. Operation and event APIs

### 8.1 Common API shape

Conceptually, every host exposes:

```text
OperationHandle<Result> : Awaitable<Result> {
  operationId: OperationId
  result(): Promise<Result>
  subscribe(options?): EventSubscription
  cancel(request?: CancelRequest): Promise<ControlAck>
  pause(spec: PauseSpec): Promise<PauseTriggerHandle>
  continue(stop: StopToken | PauseTriggerHandle): Promise<ControlAck>
  step(request: StepRequest): Promise<ControlAck>
  dispose(): Promise<void>
}

PauseTriggerHandle : PromiseLike<PausedEvent>, AsyncIterable<PausedEvent> {
  breakpointId: BreakpointId
  next(): Promise<PausedEvent>
  remove(): Promise<void>
}
```

JavaScript handles implement `PromiseLike<Result>` so `await handle` is
equivalent to `await handle.result()`. `PauseTriggerHandle.next()` may be called
again after continue when the trigger is persistent. Awaiting a pause-trigger
handle is shorthand for its next not-yet-observed hit; each caller retains an
independent cursor. Rust uses a result future, control handle, and streams over
the same shared operation core rather than requiring a self-referential
`Future` object.

`CancelRequest` contains an optional execution scope, bounded reason, and
optional source selector. A source selector must resolve uniquely before the
request is acknowledged. Execution-scope and source-selector targeting are
mutually exclusive; supplying both rejects the request without changing control
state. Omitting all targeting fields selects the root.

### 8.2 Events and subscriptions

Every event has operation ID, monotonic sequence, kind, and bounded payload.
Important event families are:

```text
accepted | scope-created | scope-state | task-state | progress |
diagnostic | observability | breakpoint-resolved | pause-requested |
stopped | continued | control-failure | subscription-gap | terminal
```

`subscribe({ fromSequence, capacity, filters })` creates an independent bounded
cursor. A slow subscriber receives `subscription-gap { firstMissing,
lastMissing }` and continues from the oldest retained event. It never causes
engine work to block.

Unless a host negotiates stricter limits during initialization, the common
defaults are 16 live subscriptions per operation, 256 queued events per
subscription, a requested-capacity ceiling of 4,096 events, and 64 KiB per
inline event payload. Terminal diagnostics and recovered-control-failure lists
default to 256 entries each, artifact-reference lists and retained lazy handles
are capped at 4,096 entries each, and every truncation is reported by an
original count alongside the retained prefix. Inspection defaults are 64 stack
frames per page (512 maximum), 100 variable children per page (1,000 maximum),
and 4 KiB per string or byte preview. A stopped snapshot may retain at most the
smaller of 16 MiB or one eighth of the operation's effective root memory cap;
exceeding that limit produces truncated/opaque entries rather than failing the
paused operation.
All effective limits are returned by initialization and capability discovery.

The runtime retains an accepted breakpoint resolution until that breakpoint is
removed, retains the current stopped event until continue or cancellation, and
retains its matching continued event plus the terminal event until the
operation handle is disposed. These critical records live outside subscriber
rings and therefore cannot become gaps. Large artifacts, value children,
traces, and source-map collections are returned as paged handles, not embedded
without bounds in events.

Subscriptions created after terminal completion may read the retained terminal
summary and available tail but are not promised unbounded replay from sequence
zero. Disposal releases subscriptions, breakpoints, snapshots, and retained
handles; it does not erase already committed output.

### 8.3 Terminal outcomes

```text
OperationOutcome<Result> =
  succeeded { result, recoveredControlFailures[], artifacts }
  failed { cause, diagnostics, artifacts }
  cancelled { reason?, diagnostics, artifacts }
  fatal { cause, diagnostics, restartable, artifacts }
```

All statuses retain bounded diagnostics and an explicit retained/discarded
artifact statement. `artifacts` contains bounded `retained` and `discarded`
reference lists plus their original counts, even when one list is empty. A
cancelled or failed operation cannot expose an
uncommitted primary result as if it succeeded. Scoped cancellation that is
explicitly recovered may appear in `recoveredControlFailures` of a successful
result. Native results and retained values stay typed and engine-owned; wire
events contain only bounded metadata and opaque handles, with paging performed
by the owning host adapter.

## 9. Suspended-state inspection

`SuspendedSnapshot` is immutable and bounded:

```text
SuspendedSnapshot {
  stop: StopToken
  reason: breakpoint | pause | step | control-failure
  triggeringTask?: TaskId
  threads: ThreadSnapshot[]
  executionScopes: ExecutionScopeSnapshot[]
  retainedBytes
}
```

A debugger thread represents a logical scheduled task. Each thread exposes its
current physical worker as optional presentation metadata, state, owning
execution scope, and paged logical frames. Frames include function/operation
name, phase, source-mapped location, execution scope, and handles for lexical,
dynamic, input, output, scheduler, and diagnostics scopes where applicable.

Variables and native semantic values are read-only in the first version. They
provide bounded summaries, types, child counts, named/indexed paging, optional
declaration/source locations, and stop-local reference handles. String/byte
previews are truncated with explicit original lengths. Cycles are represented
by handles; traversal depth and total retained snapshot bytes are capped.

Inspection never invokes user code, resolver I/O, plugin callbacks, getters, or
mutating query evaluation. Unsupported values return a typed opaque summary.
All stop-local handles become invalid immediately on continue, cancellation, or
terminal completion.

## 10. DAP projection

DAP is the canonical editor debugger projection. The adapter follows the
standard stopped-state discovery flow `threads -> stackTrace -> scopes ->
variables`, maps source breakpoints through the safe-point resolver, and uses
stop-local variable-reference lifetimes. See the
[official DAP overview](https://microsoft.github.io/debug-adapter-protocol/overview.html).

Standard mappings are:

| DAP surface                   | CEM-ML behavior                                                                                   |
| ----------------------------- | ------------------------------------------------------------------------------------------------- |
| `initialize`                  | Advertise only compiled and active debugger capabilities                                          |
| `launch` / `attach`           | Create or bind one operation/debug session                                                        |
| `setBreakpoints`              | Replace persistent source pause triggers for one URI                                              |
| `breakpointLocations`         | Return executable CEM safe-point locations                                                        |
| `threads`                     | Return live logical tasks, not OS/WASM workers                                                    |
| `stackTrace`                  | Return paged logical engine frames for one task                                                   |
| `scopes` / `variables`        | Return bounded read-only snapshot values                                                          |
| `pause`                       | Request all-stop; supplied thread is the preferred trigger task                                   |
| `continue`                    | Resume the supplied stopped generation and all tasks                                              |
| `next` / `stepIn` / `stepOut` | Use the step semantics in section 7.4                                                             |
| `stopped`                     | Always set `allThreadsStopped: true`                                                              |
| `continued`                   | Set `allThreadsContinued: true` for continue and false while stepping one task/dependency closure |
| `terminate`                   | Request root operation cancellation                                                               |
| DAP `cancel`                  | Cancel only a pending DAP protocol request, never the transformation                              |
| `disconnect`                  | Apply the launch/attach ownership rule below                                                      |

The adapter does not advertise mutation, reverse execution, restart-frame,
write-memory, or expression side effects. Side-effect-free evaluate/watch may
be added only after it reuses the bounded condition evaluator.

Versioned custom requests use the `cem/` namespace only for gaps in DAP:

- `cem/operation` returns operation identity and terminal/control state;
- `cem/executionScopes` returns the execution-scope tree and semantic mappings;
- `cem/cancel` performs root or scoped transformation cancellation;
- `cem/nativeValue` retrieves a bounded CEM AST/event/item/artifact projection;
- `cem/workerTopology` returns logical pools and physical worker telemetry.

DAP `terminateThreads` is not used for scoped cancellation because a DAP thread
is a logical task, while CEM cancellation targets an execution-scope subtree.

Disconnect is deterministic. A session that launched its operation requests
root cancellation by default; `terminateDebuggee: false` removes debugger
triggers, resumes a stopped operation, and leaves it running. A session attached
to a pre-existing operation resumes and detaches by default; it cancels only
when `terminateDebuggee: true`. Unexpected transport loss applies the same
default after the negotiated hard-cancel grace period so an operation cannot
remain parked forever.

## 11. CLI and host transports

### 11.1 Native CLI

Ordinary commands retain signal cancellation and resource enforcement without
activating debugger snapshots. `SIGINT` and `SIGTERM` request root
cancellation; observed host cancellation keeps exit status 130. Typed resource
failures follow the existing non-cancellation failure policy.

The debugger entrypoint is explicit:

```text
cem-ml debug --stdio
cem-ml debug --listen 127.0.0.1:0
```

The DAP `launch` request carries the existing CEM-ML command and arguments.
With `--stdio`, stdout is reserved exclusively for DAP framing; command output
is sent through DAP output events or explicit destinations. The TCP form is
single-session and loopback-only in the first version; a non-loopback bind is
rejected. Its selected endpoint is reported on stderr.

### 11.2 Node and browser workers

The versioned initialize/run/progress/event/result envelope gains operation
handles and control messages keyed by `OperationId`. Browser and Node clients
project the same awaitable handle and subscription semantics.

Each worker hosts one runtime instance. The pool coordinator owns operation,
scope, task, stop, subscription, and retained-handle routing. Worker messages
carry stable IDs and transferable buffers, never pointers. A stopped snapshot
is committed only after every participating worker acknowledges the pause
generation or is classified as an external wait.

Worker termination is the hard fallback for uncooperative cancellation,
timeout, OOM, or panic. The coordinator rejects late messages from a terminated
worker by worker generation and guarantees one operation terminal outcome.

## 12. Build and capability model

Cancellation, safe-point cancellation polling, logical stack guards, memory
accounting, timeouts, and scheduler limits are core safety behavior and have no
off switch.

The common crates add a default-enabled Cargo feature:

```toml
[features]
default = ["debug-control"]
debug-control = []
```

Every dependent CEM Rust crate disables dependency defaults explicitly and
forwards its own `debug-control` feature to `cem-ml/debug-control`. The CLI and
WASM deployment projects select the feature at their top-level build target, so
a transitive dependency cannot accidentally re-enable debugger code. The
existing `fake-engine` test feature remains orthogonal.

Debugger-only pause state, breakpoint registries, suspended snapshots, frame
and variable capture, DAP adapter code, and debugger transports are located in
feature-gated modules. Shared cancellation safe points call small feature-gated
debug hooks rather than duplicating control branches throughout evaluators.

When `debug-control` is compiled but inactive, ordinary execution performs no
frame/value capture and only a cheap disabled hook check at existing safe
points. `cem-ml debug` or an explicit host debug-session request activates it.
There is no global CLI flag that silently changes ordinary command transport.

A stripped build uses `--no-default-features`. It omits the CLI `debug`
subcommand and debugger host bindings entirely, while operation cancellation,
resource failures, progress/diagnostic events, and terminal results remain.
Code compiled against stripped Rust features cannot call debugger-only APIs.

The capability manifest is extended with independently reportable entries for:

- root cancellation and scoped cancellation;
- stack, memory, timeout, queue, CPU, and I/O enforcement;
- operation handles and bounded subscriptions;
- pause, source breakpoints, stepping, and suspended inspection;
- DAP and `cem/` custom request versions;
- executor topology and effective maximum worker count;
- `debug-control` compiled and active states;
- accounted-memory coverage and hard-cancel availability.

Hosts must report unavailable functionality rather than installing no-op
controls. Adding these entries increments `CAPABILITY_CONTRACT_VERSION` from 1
to 2. The host protocol's first implemented operation-control envelope includes
them from its initial major version; no pre-existing worker envelope requires a
wire migration. Capability output is covered in both default and stripped
builds.

Default and stripped artifacts use different ABI/capability identities and
cache keys. Release packaging must not substitute one profile for the other.
The default artifact remains the supported general distribution; stripped
artifacts are opt-in trusted-input/performance variants.

## 13. Compatibility and migration

Migration proceeds without breaking the current cancellation foundation:

1. Add opaque operation/scope/task identities and the execution-scope tree while
   mapping existing scheduler and semantic scope IDs.
2. Introduce the unified control core. Keep `AbortSignal` as a compatibility
   facade over root host cancellation for one deprecation cycle;
   `EngineContext::with_abort_signal` installs that facade into the new control.
3. Replace phase-end-only time checks with registered deadlines and deep safe
   points. Add logical stack and accounted-memory guards.
4. Replace sequential `WorkerPool` execution with the common logical scheduler
   and native physical executor, preserving the existing queue/trace API where
   compatible.
5. Add deterministic staging/commit before enabling parallel output-producing
   work.
6. Add operation handles and host wire messages, then worker-pool hosts.
7. Add feature-gated pause/snapshot APIs and the DAP adapter.
8. Deprecate direct `AbortSignal` construction only after every public host can
   create and control an operation handle.

Serializable run requests remain free of live control objects. Host-owned
`OperationControl` stays in `EngineContext`/operation handles, while normalized
run plans carry only policy values and stable identities.

## 14. Required verification

### 14.1 Scheduling and determinism

- Prove actual simultaneous native task execution up to root and child caps.
- Prove queue reject, cooperative block, and spill-to-parent behavior.
- Prove I/O waits do not retain CPU permits and respect I/O caps.
- Run randomized worker delays and assert identical result bytes, diagnostics,
  source maps, artifact IDs, canonical report events, and commit order.
- Prove worker loss, replay eligibility, stale generation rejection, and one
  terminal result.

### 14.2 Cancellation and resources

- Cover pre-start, queued, running, I/O-waiting, atomic-region, output-staging,
  paused, and completion-race root cancellation.
- Cover scoped cancellation with unaffected siblings, explicit boundary
  recovery, type-compatible replacement, unhandled escalation, and fail-fast
  escalation.
- Cover logical stack failure before native recursion, descendant and ancestor
  memory caps with charge rollback/release, and scope/ancestor timeouts.
- Prove debugger-stopped time is excluded while rendezvous and I/O wait time are
  included in active timeout.
- Prove host cancellation returns cancelled while stack/memory/timeout/queue
  causes return failed.

### 14.3 Pause and inspection

- Cover manual next-safe-point pause, scoped pause, scope enter/exit, source
  line/column/range resolution, ambiguity, persistent triggers, and conditions.
- Prove all-stop waits for every task or classified external wait, never exposes
  a partial snapshot, and cancellation wins during rendezvous or while stopped.
- Cover repeated stop/continue cycles, stale/foreign stop tokens, next/step-in/
  step-out, dependency-closure execution, and a breakpoint winning during step.
- Verify bounded thread, stack, scope, variable, native-value, paging,
  truncation, cycle, and stop-local handle behavior.
- Prove slow event consumers receive gaps without losing the current stop or
  terminal event.

### 14.4 Host and build parity

- Run the control matrix against native Rust, native CLI, Node workers, browser
  workers, single-worker fallback, and main-thread WASM fallback where claimed.
- Fixture DAP initialize, breakpoint resolution, stopped/continued, threads,
  stacks, scopes, variables, stepping, root terminate, and `cem/cancel` scoped
  cancellation.
- Verify ordinary CLI output is unchanged and debug stdio contains only DAP
  framing.
- Build and test default and `--no-default-features` variants; assert debugger
  code/symbol surfaces and capabilities are absent from the stripped build while
  cancellation and every resource enforcement test remains green.

## 15. Implementation gates

The implementation sequence is deliberately gated:

1. **Control identities and accounting:** execution-scope tree, unified causes,
   typed policy, deadlines, stack guards, and memory permits.
2. **Native scheduling:** real worker pool, cooperative queue waits, independent
   I/O permits, deterministic staging and commit.
3. **Deep cooperation:** bounded safe points in XPath, query, template/render,
   transform, resolver, plugin, parser, and output paths.
4. **Scoped failure:** subtree unwind, cleanup, boundary delivery, explicit
   recovery, and escalation.
5. **Host handle:** awaitable result, bounded subscriptions, control methods,
   versioned messages, and exact terminal ownership.
6. **Debugger core:** pause generations, all-stop snapshots, breakpoints,
   stepping, inspection, and feature gating.
7. **DAP and CLI:** standard mappings, `cem/` extensions, stdio/listen
   transports, and ordinary-command isolation.
8. **Worker parity:** Node/browser pools, WASM operation handles, hard fallback,
   and cross-host fixtures.
9. **Stripped profile:** no-debug builds, capability proof, binary/performance
   comparison, and release packaging.

No later gate may claim completion before its native Rust contract fixtures are
green. Browser demos and DAP clients verify host wiring after the common engine
semantics are established.
