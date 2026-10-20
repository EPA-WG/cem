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

This stage provides sealing, host storage and explicit invalidation APIs.
Automatic all-owner quiescence/publication wiring, live island export, bounded
pending-input handling, runtime replacement, fresh-owner resume, stale-message
fencing and declarative recovery status follow together. They are not enabled
by this store. The complete Studio long-session verifier must be repaired for
the current navigator shell and pass before replacement is enabled. Saved project
revisions and unsaved editor state must remain independent.

Evidence: native `committed_checkpoint`/`runtime_memory` fixtures, shared
`committed-checkpoint.spec.ts`, and real module
`packages/cem_ql/tests/committed-checkpoint-wasm.mjs`. The latter seals under soft
pressure, preserves bytes on failed replacement, and re-admits transport in an
independently instantiated WASM module. It does not claim the previous module was
garbage-collected or that owners have already resumed.
