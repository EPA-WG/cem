# Portable committed checkpoints

`cem_ml::committed_checkpoint` defines `cem-committed-checkpoint/1`.
`sealCommittedCheckpoint(candidateJson, receiptJson)` exposes it in each linked
WASM runtime. The independently held receipt belongs to the trusted host, after
acknowledged atomic DOM/CSS publication for **every owner sharing that runtime**.
Authored data, cached render output and candidate metadata cannot supply this
authority.

The receipt records runtime identity, generation, committed sequence, policy
stamp, every owner's instance/declaration/revision identity and accepted event
sequence, plus external operation IDs and outcomes. Uncertain external outcomes
are `unknown`; both unknown and acknowledged operations are records for
reconciliation, never commands to replay. Candidates must match the complete
receipt exactly. Missing/duplicate/mixed owners, unknown fields/versions, invalid
identities and non-portable counters fail closed.

Each owner's `island` is policy-filtered serialized DOM-native data-island markup,
including portable payload, slices, form/validation state and resource semantics.
It is inert text in this transport, not a second state authority. Resume must
pass existing [lifecycle admission](cem-element-lifecycle-principle.md#reconnection-serialized-html-and-hydration)
and applicable domain schemas. Sealing validates transport and receipt equality;
it does **not** certify island semantics or grant capabilities. Derived caches
are rebuilt. Never export live native handles, pointers, WASM memory, JS AST
substitutes, credentials, listeners or other live host objects. If an owner cannot
export complete portable state under its privacy policy, recovery is unavailable;
do not omit the owner. Redaction/omission is explicit and cannot recover state
from rendered DOM.

Both JSON inputs and resulting checkpoint are each bounded to 1 MiB; owner and
operation counts are each capped at 256. Generations/committed sequences are
positive JavaScript-safe integers; accepted event sequences may be zero. Native
sealing is synchronous bounded transport work, without resource callbacks,
rendering, actions or replay. Its exclusive recovery reservation estimates eight
times combined input bytes plus 64 KiB. Ordinary reservations must drain first;
the reservation must fit within withheld headroom and remaining hard budget.
It blocks ordinary admission while held and releases on return/error/drop.
This does not disable operation-control cancellation or render memory checks.
ABI copies precede admission; headroom is a forecast, not an OOM guarantee.

`CemCommittedCheckpointStore` in shared `cem-elements` owns bytes outside WASM.
Construct one per affected runtime before pressure: it reserves two fixed 1 MiB
host slots. `acknowledge(receipt)` copies the trusted publication receipt.
If replacing an entire worker, keep this store in the surviving host, outside
the worker being terminated.
`capture(candidate, nativeSealer, signal)` seals against that receipt, checks
cancellation/freshness after return, then atomically replaces stored bytes.
The sealer is a trusted adapter to the native export, not an authored callback.
It encodes directly into the alternate slot. Errors, oversized UTF-8 output,
overlap, cancellation and intervening invalidation/publication preserve the last
good bytes. Returned copies cannot mutate the store. No WASM view, DOM owner,
capability object or sealer callback is retained after capture finishes.

Hosts must call `invalidate()` immediately on pending input, owner-set or policy
changes, even before another commit. `current()` is available only when saved
state matches the current acknowledged receipt. `lastCommitted()` can export
older work but cannot authorize rollback of newer edits. Monotonic generation
and sequence checks fence old receipts; a newer generation may start at sequence
1. `dispose()` clears both slots and receipt metadata on session teardown.
Returned copies remain the caller's responsibility. Storage is session-private
host memory; this API never implicitly writes local storage, IndexedDB or disk.
Dispose the store on privacy/security boundary changes: invalidation deliberately
retains older private bytes for work recovery.

`CemCheckpointPublicationCoordinator` adds an opt-in host publication barrier.
Construct it before mounting owners, with a host `runtimeId`, positive generation,
policy stamp, trusted native `seal` adapter and `allowOwner` consent function.
Pass the **same coordinator** as `checkpointPublication` to every participating
`CemElementRuntime` in that host domain, including nested scopes. Its active owner
registry is lifecycle-owned; disconnected/disposed owners remove their export
hooks. A new owner blocks capture until it has successfully published.

Render tokens invalidate before queued work. Admission leases remain outstanding
until their preparation/render/resource promises settle, including superseded
work. Only successful DOM/CSS publication followed by stable form state
acknowledges a revision; a failed/recovery-required CSS or patch transaction does
not. Host attributes and canonical island mutation records are checked before
capture/currentness, including mutations preceding observer delivery. Snapshot
revision changes invalidate rather than silently relabel the prior publication.
Capture listeners invalidate on beforeinput/input/change/click/submit/reset in
the capture phase; they coalesce a fresh ordinary render without delaying,
cancelling or replaying the event. Accepted event ordinals are committed only
with the subsequent published owner revision. Weak instance counters preserve
these ordinals across disconnect/reconnect without retaining disconnected owner
objects or input payloads. Custom host registrations must supply their trusted
prior ordinal as register's optional fourth argument; fresh instances use zero.

`automatic: true` coalesces capture attempts after publication and admission drain;
the default is explicit `await coordinator.capture(signal)`. There is at most one
active seal and one scheduled successor, not a queue of input payloads. New input,
publication, membership or policy changes fence an in-flight seal before host
bytes replace the last-good slot. `coordinator.current()` probes every owner;
`lastCommitted()` exports older work without authorizing rollback. Additional
host work outside the instance render path must hold `coordinator.admit()` until
its finally block. The host must account for every client of any runtime it later
intends to replace: this registry does not discover physical WASM heaps, worker
pool lanes or unrelated clients, and supplies no replacement authority.

Every owner requires explicit whole-island privacy consent before serialization.
The exporter writes the canonical inert data-island template, including existing
native binary envelopes; it does not derive state from rendered output or decode
an AST in JavaScript. Omission/redaction cannot yield a recoverable partial owner.
`setPolicy(stamp, consent)` erases both old host slots and requires new owner
publication. Traversal caps at 16,384 nodes/depth 64; islands and the combined
UTF-8 transport must fit 1 MiB. At most 256 owners and 256 admissions are tracked.
Overflow blocks the domain instead of omitting owners or buffering input; ordinary
rendering still proceeds. A fresh host domain is required after overflow.

Portability is deliberately conservative: plain declarative owners and directly
bound scalar value/checked slices must agree with their live controls. Unsupported
or unbound control edits, password/file controls, behavior capabilities, foreign
placements, live HTTP/repository/storage/location resources and host-command
owners make the **entire domain unavailable**. Such state needs complete portable
control/resource/capability and external-operation outcome admission before it
can participate. No owner is skipped, password/file controls are refused, and no action
is retried. The trusted consent policy must also refuse credentials in authored
payload or slices. The current capture receipt has an empty operation journal because
those action owners are refused.

Automatic capture is not enabled in Studio. Physical runtime-domain discovery,
resource/capability/control portability, operation journaling, memory-pressure
admission drain, generation replacement, fresh-owner semantic resume, stale-message
fencing and declarative recovery status remain follow-up work. The complete Studio
long-session verifier now covers the current navigator shell and must remain
passing before replacement is enabled. Saved project revisions and unsaved editor
state remain independent.

Evidence: native `committed_checkpoint`/`runtime_memory` fixtures, shared
`committed-checkpoint.spec.ts`, and real module
`packages/cem_ql/tests/committed-checkpoint-wasm.mjs`. The latter seals under soft
pressure, preserves bytes on failed replacement, and re-admits transport in an
independently instantiated WASM module. It does not claim the previous module was
garbage-collected or that owners have already resumed.

Publication evidence: `checkpoint-publication.spec.ts` exercises all-owner and
admission barriers, input coalescing, async sealing races, privacy/policy erasure,
stale handles and overflow. `checkpoint-publication.stories.ts` uses real native
sealing with worker and fallback rendering, nested owners, direct edits, snapshot
revision changes, held/failed/repaired CSS, unsupported-owner refusal,
cross-document inert-template traversal limits, reconnect ordinals and continued
ordinary rendering. Existing stylesheet and data-island fixtures remain required.
