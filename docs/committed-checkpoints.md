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
its finally block unless it already uses the shared processing host/support
boundary, which now tracks native work automatically. `whenRenderSettled()` waits
for that view; checkpoint availability additionally requires all native clients
to settle. Physical coverage below supplies no replacement authority.


### Physical runtime generations and native clients

Managed publications carry optional `receipt.nativeRuntimes` coverage. The top-level
`runtimeId`/`generation` remain logical host-session labels. Physical identities
are privately issued for actual Worker objects and the initialized fallback
`WebAssembly.Memory`. A fresh domain has an immutable generation `1` and a fresh
opaque runtime ID; another native object cannot reuse it. A realm-wide weak
physical-owner fence revokes both ledgers if separate framework copies attempt
to claim the same Worker/memory, rather than accepting partial coverage. Pooled roots on the same
Worker share coverage, as do all fallback roots using the same WASM module graph,
across documents and worker pools. Worker failure revokes its generation
synchronously. Selecting fallback moves the exact client into the shared fallback
domain and requires a fresh owner publication; old publications cannot relabel
themselves as fallback. Recreating a worker issues a different physical identity. Native results and
retained session leases pin their actual execution generation, including the
acknowledgement boundary. Failed cleanup stays attached to the old heap; confirmed
worker destruction retires committed old models without invalidating the healthy
fallback generation.

Every shared processing host registers a native client, including hosts with no
CemElementRuntime/checkpoint opt-in. All live native consumers and retained
HTTP documents/sessions must be represented through exact private capsule
witnesses; claiming an existing instance ID is insufficient. Queued and running compile,
render, stylesheet, value, document, session and cancellation jobs hold admission
until completion, including preflights, superseded jobs and cleanup. A stylesheet
load waiting for host I/O continues blocking after its initial request returns. Disposal
keeps its client enrolled until outstanding work and native releases settle;
repeated disposal observes the same completion. Cleanup intent stays registered
until native release acknowledges it; releases queued before disposal are swept
by disposal when they could not dispatch. Actual uncertain cleanup revokes
coverage for that generation. Standalone asynchronous render,
query, conversion, source and URL helpers also hold transient native clients.
Direct retained templates, documents, sessions, XPath libraries and XSLT wrappers
remain blockers until released or adopted by their exact managed owner. Evicted
XSLT wrappers retain their client until disposal. No omitted client, resource or
unpublished consumer is treated as empty portable state.

External suggestions/datalist sessions join coverage through a private witness
attached to their complete original source capsule. A copied scalar record cannot
enroll the host or cover another session. Inert source intent and retained proofs
still require their normal current-state/native validation. Once a local session
has been prepared, inactive intent exports that session's exact capsule witness
rather than substituting unrelated XML intent. One island may account
for its renderer and one or more source clients, including clients on different
physical runtimes. Current owner inspection caches only the private source witnesses; ordinary
owner/fingerprint and witness-current checks still run throughout capture.
Binding release/revocation retires the corresponding consumer;
a separate work lease survives an in-flight frame until it settles. Unused source
copies and idle standalone hosts require disposal before a shared fallback domain
can be complete.

Coverage contains only runtime ID, generation, mode, mutation revision, client IDs
and associated owner IDs. Native sealing bounds all collections/counters, checks
unique runtime/client identities and complete owner coverage, and rejects foreign,
duplicate, stale or authority-bearing records. Every asynchronous export/seal
rechecks the private registry and exact coverage revision. A failed or stale seal
preserves last-good bytes; policy revocation erases them. Limits refuse the whole
domain while ordinary rendering continues. Logical-only transport receipts remain
compatible but provide no physical-generation coverage.

These registrations cover the framework's shared native entrypoints. Separately
instantiated native command runtimes are different heaps and require their own
adapters before replacement. Embedders bypassing the framework through generated
WASM exports are outside this closed client boundary and must not use its receipt
to authorize replacement. Verified replacement admission and semantic resume
remain separate work. Neither
a physical coverage receipt nor a native seal grants permission to restart,
restore native handles or replay actions.

### Pressure closure and bounded native draining

Shared runtime support samples the native `runtimeMemoryStatus()` telemetry DTO
before and after asynchronous native support work, after initializing the actual
WASM memory. Soft pressure, hard pressure or unavailable/malformed telemetry
closes ordinary admission for that physical generation. The worker sends a
versioned, bounded pressure notification to its surviving host; every pooled
root closes together. Shared fallback hosts close together through their actual
module domain. This is a monotonic refusal signal, with no replacement authority.
No authored input can configure the sampler or reopen admission. Lower telemetry
or changing the native budget does not reopen the same generation.

Fresh host requests fail with `cem.runtime.memory.closed` before registering
source/stylesheet owners or copying input into the host queues. Standalone
native helpers also refuse before invoking their operation; refusal between
client enrollment and work admission still retires the transient client. Direct
XPath-library acquisition/reuse holds the same work lease, including acquisition
that outlives cache disposal. Synchronous native source views and XSLT
construction also check closure. Admitted queued and
running jobs remain enrolled until they settle, including cancellation,
superseded work, preflights and cleanup. Already queued jobs can complete or
receive a native pressure refusal; they are never retried in fallback because
of pressure. Session/publication/document release, stylesheet delivery/failure
for retained loads, disposal and cancellation remain available. These paths
retain their ordinary native ownership, byte bounds and memory-budget checks;
cleanup permission cannot start a new load or restore authority. A pending host
stylesheet fetch remains in the drain between native requests, until its native
completion or explicit release is acknowledged.

Pressure starts a five-second drain deadline whenever work remains, even without
a waiting caller. Once work settles the deadline is cleared; later cleanup
invalidates previous drain evidence and receives a new bounded deadline.
Internal `nativeCheckpointDrain(client, { timeoutMs, signal })` waits without
buffering input or canceling the actual work. Its timeout is a positive safe
integer up to 60 seconds, defaulting to five seconds; the generation's automatic
five-second deadline still applies. At most 16 concurrent waiters are retained.
Canceling a waiter removes its timer/listener without reopening admission or
canceling peers. A timeout permanently fences that generation's checkpoint
coverage. Late replies still finish cleanup but cannot restore drain evidence.
Revocation, disposal and generation changes also fence old evidence. Browser
timers run when the event loop can service them; they cannot interrupt a blocked
synchronous WASM call or guarantee prevention of an allocation trap.

Drain results are private, frozen witnesses bound to the exact generation and
mutation revision. Structural/serialized copies have no authority. A result
means work has settled, not that every retained owner is portable. Internal
`nativeCheckpointClosedCoverage` separately verifies current original witnesses
for every involved physical domain and repeats complete client, resource,
source-capsule and owner requirements. Missing clients, unknown retained resources,
duplicate/copied proofs, new cleanup, host I/O and stale revisions refuse the
whole result. Owner publication/privacy/currentness and native sealing must
still pass the coordinator's independent checks. A complete closed generation
can seal a fresh checkpoint within native recovery headroom. A failed/timed-out
drain preserves last-good bytes without making them current or authorizing
rollback of newer edits.

This is shared `cem-elements` lifecycle infrastructure. Studio and declarative
components contain no recovery behavior or JavaScript workaround. Separate
command-service heaps have their own managed CLI adapters (below); generated-WASM
bypasses remain an explicit unmanaged boundary; neither a successful drain nor physical coverage grants
whole-application replacement permission. Coordinated replacement, semantic
resume and declarative recovery status remain subsequent work.

### Separate command heaps and the unmanaged boundary

`@epa-wg/cem-ml-cli` owns a separate admission ledger for each linked native
module singleton, including command authoring, browser/Node worker execution,
main-thread fallback and resumable-operation host calls. The exact generated
telemetry function identifies its module; CLI adapters referring to that same
function share their ledger in the realm. Each worker owns its own ledger.
No render-domain label, matching command ID or render checkpoint can cover one
of these heaps. All shipped CLI native calls pass through an explicit method
contract; another generated export cannot silently become a managed call.

The adapter samples bounded native telemetry before/after calls and permanently
refuses fresh work after soft/hard pressure or unavailable telemetry. It keeps
async discovery and native command callbacks enrolled through completion, and
command workers remain enrolled until their terminal result is posted. Commands
already waiting on host capabilities can settle or be canceled; pressure does
not terminate the worker, retry the command, or fall back to another heap.
Cancellation, disposal and the existing operation-settlement paths remain
available. Fresh execution, artifact reads and native graph-author functions
refuse with `cem.runtime.memory.closed`. Returned graph authors retain the guard
on every invocation. Retained artifact counts and resumable hosts block capture
until native disposal acknowledges them; a partial artifact disposal cannot
cover the remaining handles. No live command handle or callback is serialized.
Uncertain native outcomes/cleanup and bounded ledger overflow refuse capture.

Browser/Node command-service clients expose `checkpointNative(timeoutMs)` as an
explicit monotonic close-and-drain operation, defaulting to five seconds. Native
work automatically has a five-second pressure deadline; explicit waits accept
positive safe integers up to 60 seconds without extending that deadline. Host
acknowledgements are also bounded to five seconds. At most 16 waiters and 256
simultaneous managed native calls are retained. A timeout permanently fences the
adapter, including late replies. Local drain witnesses bind to the original
ledger/revision. A surviving command client issues a frozen, original host
witness only after worker acknowledgement and all its executions, actions and
host capabilities settle. `isNativeCheckpointCurrent(witness)` rejects copies,
foreign clients, later cleanup, transport failure and closed clients. It cannot
resurrect worker evidence by copying a receipt. A refused retained-state capture
leaves cleanup available; callers may dispose artifacts and request a new witness.

Every command witness explicitly states `scope: "managed-cli-only"` and
`replacementAllowed: false`. A render receipt cannot cover a command heap.
Raw `@epa-wg/cem-ml/wasm` calls, initializers and independently instantiated
modules remain outside managed admission. No witness authorizes replacement,
replay, disposal of another owner's resources or rollback of newer drafts.

The linked runtime now exports `commandNativeResourceInventory()`: a bounded,
versioned counter-only census of all 29 retained resource/cache/operation roots
in the command/reference profile. Rust checks the exact manifest, including
empty registries; missing, duplicate, unknown or unsafe counters refuse the
report. Live documents, templates, reference sources/values/validation/execution/
snapshots, capability sessions, XPath/XSLT handles, callbacks, resumable hosts,
render output, active commands and memory reservations block acknowledgement.
Rebuildable cache entries and terminal artifact request/disposal journals remain
visible as discardable counts. Live cache leases still block. No AST, source
content, callback, native ID or live authority crosses this control boundary.
Journal retirement/reuse policy still belongs to the replacement coordinator.

Native mutable registry access and exported operations advance a safe-integer
revision, including same-count changes behind retained references; exhaustion
refuses the census. Read-only census/status/identity calls do not advance it.
The async command bridge enrolls its activity and memory reservation synchronously
before its first poll, holding both across every host callback wait. Consequently
an empty CLI ledger cannot hide a raw pending command or retained resource.

Embedding participation is explicit. `client.claimNativeEmbedding()` returns
an original private lease tied to the actual Worker object. Pass **every live
original lease** to `await client.checkpointNative(5000, [lease, ...])` to request
an inventory-backed acknowledgement. Omitted, duplicate, copied, foreign or
released leases refuse before closure; ownership changes during capture fence
the reply. Up to 256 owners are supported, and release/worker termination revoke
evidence. The optional `nativeInventory` and `embeddingOwners` fields describe
that acknowledged snapshot; the default call retains its previous managed-only
behavior. Worker `isNativeCheckpointCurrent` checks parent ownership/activity,
not unobservable later raw worker calls. A replacement coordinator must request
and atomically revalidate a fresh native revision under closed raw admission.

Main-thread authoring exposes the corresponding
`claimNodeCommandNativeEmbedding` / `claimBrowserCommandNativeEmbedding`,
`checkpoint…`, `release…` and `is…InventoryCurrent` functions. Their shared weak
registry keys ownership by the actual `WebAssembly.Memory`, not a caller label.
Main-thread currentness re-reads the native census revision. A new memory,
released owner, managed activity or raw same-count mutation invalidates the
original proof; copied/foreign records never become authority. Original leases
are in-memory contracts and cannot be serialized as portable checkpoint state.

Coordinated closure is a separate explicit operation. All original embedding
leases are required by `await client.closeNativeAdmission([lease, ...], timeoutMs)`;
legacy `checkpointNative` calls remain inventory-only. The coordinator closes the
native gate before waiting for managed work, alongside monotonic host admission.
Every ordinary generated WASM export checks a native RAII entry permit before
parsing, callbacks or registry access. Permits enroll synchronously, including
unpolled commands, and survive async callbacks. The twenty-ninth inventory root
counts these native entries. Raw aliases cannot start new work after closure.
Existing work can settle, and explicit disposal/cancellation/resource settlement,
cache release and bounded recovery control calls remain available. Cleanup advances
native revisions even when it leaves resource counts unchanged. Closure never
restarts a worker, retries an operation or clears a saved project/draft.

`await client.revalidateNativeCheckpoint(original, [lease, ...], timeoutMs)`
requires the current original coordinated witness and complete current leases.
The host validates original membership before dispatch and again after reply;
its monotonic ownership revision proves that membership stayed unchanged across
the native validation instant. Rust first refuses open/busy admission, collects
the full original native inventory without JavaScript callbacks, then holds its
admission lock while checking closed/idle state, zero live blockers and the exact
original/current inventory revision. New ordinary entries are excluded; native
cleanup mutations make the original revision stale. The `nativeClosure` record
contains only schema version, closed status, inventory revision and false
replacement authority. Copied/foreign witnesses, changed/released owners, stale
or forged replies, pending callbacks, timeout revival and lost workers refuse
confirmation. Failure leaves admission closed and cleanup available. After a
cleanup mutation, obtain a new close witness rather than revalidating an old one.

Main-thread `closeNodeCommandNativeAdmission` /
`closeBrowserCommandNativeAdmission` and `revalidateNodeCommandNativeCheckpoint` /
`revalidateBrowserCommandNativeCheckpoint` bind the same checks to the actual
WASM memory. Main currentness also uses native validation for coordinated proofs.
Worker synchronous currentness still covers parent activity/ownership only;
the async revalidation endpoint checks otherwise invisible raw cleanup changes.
It returns a new original witness; previous parent witnesses are superseded.

This confirms joint native/ownership consistency at one validation instant.
A later native cleanup can invalidate that instant. Replacement remains disabled:
a future coordinator must consume fresh confirmation at the actual retirement
boundary, preserve journals/portable checkpoints and resume under new runtime
and semantic authority. Unregistered JavaScript callers, direct memory mutation,
new module initialization and independent instances cannot be discovered or
consented for through an owner flag or scalar receipt. An independently initialized
heap is a new domain, never a reopening of the old closed one. Native Rust engine
APIs outside these generated entrypoints are not implicitly enrolled. Studio only
receives shared runtime assets and continues to use its declarative consumer.


Every owner requires explicit whole-island privacy consent before serialization.
The exporter writes the canonical inert data-island template, including existing
native binary envelopes; it does not derive state from rendered output or decode
an AST in JavaScript. Omission/redaction cannot yield a recoverable partial owner.
`setPolicy(stamp, consent)` erases both old host slots and requires new owner
publication. Traversal caps at 16,384 nodes/depth 64; islands and the combined
UTF-8 transport must fit 1 MiB. At most 256 owners and 256 admissions are tracked.
Overflow blocks the domain instead of omitting owners or buffering input; ordinary
rendering still proceeds. A fresh host domain is required after overflow.

Control capture supports owned native inputs, textareas, selects and buttons,
including unbound edits, and the existing shared plaintext-editor capability.
`cem_ml::checkpoint_control_state::validate` and WASM
`validateCheckpointControls` admit the explicit `cem-checkpoint-controls/1` JSON
control DTO before publication: at most 256 controls, 1,024 options per select
and 256 KiB for both input and normalized output. Stable render identities,
unique IDs, effective input types, option/value consistency, UTF-16 selection
bounds, focus, checked/indeterminate, disabled/read-only and custom validity are
validated. This named scalar UI boundary is not an AST representation.
Property-only edits are checked against the published control fingerprint;
composition or a deferred editor value blocks capture until a fresh publication.

Controls belong in `cem-form:form-state`, alongside the existing forms record.
A hydration marker identifies the extension. Checkpoint exports use root version
`1.0.0-checkpoint.1`; ordinary live snapshots remain `0.2.0`. Older generic
hydration must reject the new major version rather than silently dropping edits.
Current generic adoption also refuses these islands with
`cem-element.checkpoint_resume_required`: explicit native/domain semantic resume
is still required. Capture does not enable HTML-copy restoration.

The native `cem-checkpoint-runtime/1` control envelope extends capture to shared
form-control, checkable-control, choice-select and editable-choice capabilities,
alongside plaintext-editor. It preserves private authored/default values, custom
validity, form disabling, consumed selection requests, selected/preview values,
active/range anchors, expanded state and typeahead clock state where applicable.
Generated IDs, normalized option caches and listeners are derived again from the
canonical payload. Pending programmatic values, composition, unaccounted provider
leases/claims and deferred commits block capture.

Stable non-form capture supports navigation-landmark, tree-view (roving value and
bounded typeahead clock), tab-panels (roving index/count), and composite-menu
(active render-node identity and typeahead). Settled submenu sessions additionally
retain validated parent, launcher and open-panel owner/revision/node endpoints,
with `reacquire` disposition. These endpoints join the complete dependency receipt;
DOM IDs, geometry observers, popup leases and task callbacks are never serialized.
Explicit submenu policies retain `anchor` (invoker/target), `boundary`
(viewport/target), entry and return focus (auto/none/target), inherited boundary
owner and `anchor-lost` (close/freeze). Targets use current acknowledged
owner/revision/render-node endpoints, including scoped and native typed
references. Logical menu-chain receipts belong to the owned composite node;
explicit policy receipts belong to the host, so a launcher and an authored
return destination remain distinct dependencies. The portable policy always
requires `geometry: recompute`. It carries no DOM IDs, rectangles, style claims,
observers or focus/geometry leases.

Open explicit submenus must have a settled placement from their current geometry
owner. Capture checks current anchor/boundary/panel/containing-block geometry,
viewport, direction, pending geometry work and owned style claims without
repositioning, focusing or opening anything. Missing/changed anchors, stale
placements and foreign ownership refuse the complete domain, including a frozen
menu still visible after anchor loss. Closed panels may retain structurally
eligible focus targets inside hidden output; current visibility and focus
eligibility must be reacquired when reopening. Missing, ambiguous, disabled,
inert or unowned targets remain unsupported. Endpoints in another runtime require
explicit membership in the same checkpoint coordinator domain. Resume must acquire
fresh references, layout and focus authority; this capture contract does not
implement automatic runtime replacement or restore old pixels.
Selected/expanded values and tab selection remain in canonical
attributes/slices. Normalized item caches and generated accessibility IDs are
derived from the retained payload, never serialized browser ASTs. Stable focus is
an owner-local host or render-node identity; nested owners record their own focus.
Native controls retain their exact edits and selection in the control domain.
Keyboard, focus and surface lifecycle events invalidate publication before export.

Settled popup capture requires an internal trigger, stable render-node identities
and no external geometry references. Both open and closed states retain scroll
position and fresh geometry/authority intent. Settled native dialog/popover
controllers retain open/closed phase, modality, return value, focus-within state,
close reason and invocation command/context key/input kind/geometry. Invocation
source, return destination and logical parent use trusted owner/revision/node
endpoints and join the complete receipt graph. Resume must consume default-open
initialization and restore visibility without redelivering opening/closing actions.
Completed menu-to-dialog handoffs use the dialog invocation’s source, logical
parent and return destination endpoints after the menu relay has committed.
Resume restores state without invoking the command or reusing the old relay.
Open surfaces can capture a materialized native invocation `context` as an opaque
CEMV artifact/content-hash/root-index capsule. Checkpoint-enabled shared surfaces
retain a private bounded byte witness when invocation preparation is admitted.
Capture compares both metadata and actual bytes; caller metadata replacement or
in-place corruption blocks the complete domain. A stale asynchronous seal retains
the last-good checkpoint. Closing, failed preparation and disposal release the
witness. Native code validates context capsules with the same typed/provenance,
privacy and aggregate byte guards as retained renderer capsules. This preserves
context values for fresh admission; it carries no native handle or execution grant.

Preparation, closing/exit work, pointer or activation-key gestures, queued activations,
active tooltip interest/timers, opaque/live native contexts, external geometry and
unsupported deferred-body projections still block capture. A gesture settles after
native activation dispatch; disposal removes its listeners and pending timer.
Deferred native dialog bodies retain a native-validated `body` contract: current
template render-node identity, `retain`/`dispose` mode, materialization state and
at most 256 unique root identities, with `rematerialize` disposition. Materialized
bodies also carry an immutable original `cem-render-projection/1` record.
`reuseOnOpen` preserves whether the original template is still the current node;
replacing that node keeps retained output until the next ordinary open replaces it. This is
an explicitly named renderer-output/binary-artifact checkpoint transport, not authored
application data, an evaluation AST or DOM-to-source import. Entries come from
private committed native renderer attribute/text/topology records at mounting;
DOM is checked against those witnesses, never serialized to reconstruct source.
The original instance, artifact, revision and scope-policy identity comes from
private native publication records. Targeted fragment child patches and replacements
update these records along with the committed projection.

The flat projection transport preserves element/attribute/text/comment output and
privately registered paired dynamic range markers. Native validation checks
format, origin, namespace/name bounds, unique IDs and attributes, balanced depth,
range pairing and exact ordered roots. It caps transport entries at 4,096, depth
at 64 and attributes per element at 128. Projection contents share the aggregate
256 KiB original-source/resource budget; the complete runtime record remains
bounded to 512 KiB. Retained native attribute capsules use an optional shared `nativeArtifacts`
table of opaque CEMV bytes/content hashes, with per-attribute artifact/root indices.
Native validation checks the existing CEMV format, inner integrity and outer
content hash, selected attribute name/root, typed value contracts, source
provenance, sensitive attributes and credential-bearing provenance URLs. It
rejects duplicate/unreferenced artifacts, malformed indices and private fields.
At most 64 artifacts, 4,096 native graph records/edges and depth 64 are admitted;
artifact bytes share the existing aggregate 256 KiB budget. Decoded graph storage
is bounded too. Shared renderer capsules are encoded once per projection.

Private renderer records retain one bounded immutable byte witness per published
buffer. Capture compares actual bytes and metadata, including retained-clone
witnesses; an unchanged projected DOM string or claimed hash is insufficient.
Original retained artifacts survive current template/source changes without
replacing edits or focus. Native import reconstructs typed/materialized source
values and provenance in a new owner; it never reconstructs them from strings or
a JavaScript AST. Original CEMB reference-input envelopes remain in the owner's
separate native source contract. Executable/live source references, opaque native
contexts and unsupported bindings remain fenced; this does not revive source
grants or replay expressions/actions. Password/file types and sensitive autofill
tokens are refused in original output too, including historical produced-host inputs. Returned records cannot mutate retained
provenance. Original output remains portable when the current template changes,
including insertion/removal or empty output, without discarding retained edits.

Original produced component hosts are independent ownership boundaries. Each
records its original projection `hostNode` and current owner/revision/host endpoint;
both parent and child join the complete receipt graph. The child adapter captures
its current model, controls, focus and any nested deferred body. Pending,
composing, unsupported, unacknowledged or missing children block the complete
domain. Parent body inspection does not copy or replace child-owned DOM. Counterfeit
hosts, foreign insertions/mutations, altered range markers, untracked output and
raw contenteditable/private controls remain refused. Raw body control values,
selection, disabled/readonly state and focus use the ordinary control contract.

Future semantic resume must materialize the retained original output before
hydrating the independent child snapshots and restoring controls/focus, requiring
fresh current scope/authority. Never-materialized, empty and disposed bodies have
explicit state. Closing a `dispose` body and reopening uses the latest template;
capture never disposes or resets edits. This transport does not enable automatic
runtime replacement or execute opening/closing actions.

`cem-grid` remains declarative layout using ordinary payload/control/focus capture,
without admitting an arbitrary stateful ARIA grid controller.

Ready, editable listbox suggestions retain original source provenance, provider/query and
prepared revisions, exact publication configuration, query/feedback/visibility
intent, row availability/eligibility, scroll state, active row and committed
selection proof intent. Rows use source-order ordinals, so equal values remain
distinct. Values and caret selection remain in the editor's native-control
record. A proof is inert and must be revalidated against a fresh native source
and fresh host authority; no old source object, publication, lease or token is
serialized. Pending queries, composition, armed pointer/key commits, ancestors
with unsupported surface lifetimes and stale placements block capture.

Settled inactive listbox states have a separate strict `phase: inactive`
contract for idle, failed and unavailable providers. It retains original source
intent, profile, provider/query revisions, editor query and feedback intent, with
zero row count and no rows or active-selection fields. An optional retained
`proof` has its own original source envelope, exact source-order `index`, editor
`value` and `disposition: revalidate`. Its source may differ from the inactive
query’s source intent. The value must match the editor query and the ordinal
must fit that source’s row count; native datalist profiles cannot carry proofs.
Native source validation shares the aggregate byte budget. Resume must recreate
the native session and revalidate the exact ordinal/value with fresh authority
before restoring required-selection validity; transport admission is not proof
of semantic membership. Local sources supply their original captured XML and
label selectors; external hosts
must explicitly supply original source intent. Missing source/proof provenance,
pending queries/readiness, composition or revoked host policy still block capture.
Source validation does not retry a query or replay an action. Native controls
with `customError` and an unobservable validation message (for example a
read-only input after proof revocation) retain the existing whole-domain refusal;
re-enabling the control exposes the message for exact capture.

Native datalist capture uses `phase: native-datalist`: original native source,
provider revision, owned datalist render-node identity, attached/unavailable claim
state and `presentation: reset-on-resume`. Input type, edits, validity and caret
remain in the provider/control record. The exact shared lease accounts for both
attachment and placement subscriptions. Native picker visibility/selection is
not observable, so a focused native-datalist editor refuses the complete domain;
only its settled, unfocused model is portable. Resume requires fresh list/target
ownership and never restores an old browser picker or grants row-selection proof.
Failed local native sources retain an inactive source contract after releasing
claims/subscriptions; capture does not retry unchanged invalid sources. Repair
creates fresh native admission. One weakly owned current original-source snapshot
is retained, without native connections or a history of sources.

Externally admitted source-backed declarative views (without an editor controller)
retain a strict `nativePublication` record: original source envelope, listbox data
profile, query/revision, publication configuration, source-order scalar rows and
committed render-node identities, with `reacquire` disposition. The shared runtime
reads its actual committed native binding and verifies each row's runtime identity
and ownership; authored attributes cannot manufacture provenance. Native validation
shares the original-source byte ceiling and the existing publication/row rules.
Missing capture opt-in, revoked bindings/policy, stale or forged row markers and
unknown profiles refuse the whole domain, including after asynchronous sealing.
This record is resource metadata, not a provider lease or widget controller.
A live frame consumed by a stateful listbox also retains this provenance alongside
its controller contract; source bodies share the same aggregate byte ceiling.
The existing inactive-withdrawal and native-datalist attachment contracts account
for their non-interactive frames.
Unattached native-datalist frames require the standalone attachment contract below; a bare publication remains nonportable.

External `suggestionsControllerInputs` use the same shared controller, provider
lease, placement grants and complete editor-provider receipt as local listboxes.
The editor may be another current owner in the domain, resolved through native
projection or the scoped `@name` convention; an arbitrary ID supplies no authority.
Hosts opt the original session into capture and supply its
`checkpointSourceIntent()` getter for idle/failed/unavailable states. This getter
copies source intent with zero rows and never publishes, retries a query, or
modifies the retained source. Ready and retained-proof state come from the exact
native publication/source identities. Pending work, foreign claims, source/query
revocation and providers outside the admitted owner domain remain fenced.

A form provider can admit exactly one accounted shared suggestions consumer.
The shared adapter checks its exclusive lease, both attribute claim objects,
subscriptions and all validity owners (including empty claims). Ready listboxes,
attached datalists and inactive/unavailable consumers require exactly their
attributes, datalist or empty claim state respectively. Capture retains
provider revision/consumer/validity state. A completed commit's exact revision/value
and one-shot duplicate-native-change suppression are portable; a held key or
mismatched suppression record still blocks capture. Unknown subscriptions, competing
leases/claims or out-of-domain consumers block the complete checkpoint. Ordinary
widget admission and behavior are unchanged. The package-private manual-listbox
delegate shares its owning suggestions controller’s lease and subscriptions;
a standalone delegate or direct host datalist lease without a complete registered
owner contract remains unaccounted and blocks capture. No public API accepts a
caller-supplied portable provider record in place of live ownership.

A standalone shared manual-listbox delegate can opt in with
`lifecycle.checkpoint: true`. Its exact private lease owner and tagged provider
subscription register with the shared form-provider accounting. The produced
host captures `manualListbox`: current panel/editor endpoints, provider revision,
visibility, `reset-on-resume` presentation and `reacquire` disposition. Both
endpoints join the complete current receipt graph. This delegate owns no row
selection/value or editor ARIA claims; those cannot be silently included. The
host's `current()`/`ready()` callbacks remain current admission policy. Revoked
opt-in, pending attempts, composition/gestures, foreign claims/listeners, explicit
geometry or ancestor lifetimes, unsupported profiles and disconnected endpoints
block capture. Disconnect removes accounting and releases the exact lease once.
No opt-in or an unregistered direct datalist lease still blocks capture. These
contracts record reset/reacquisition intent, not a platform popup restoration or
callback/lease serialization.

`connectCemStandaloneDatalist` is the shared host attachment for an independently
owned field and committed native-datalist frame. Its trusted lifecycle explicitly
opts into `checkpoint: true`. The `standaloneDatalist` resource record retains
original bounded XML/CEMB source, exact editor-host and owned target endpoints,
provider revision, attached/unavailable claim, and reset/reacquisition intent.
It has no platform picker visibility, row-selection proof, DOM-generated source,
callback, lease, generated list ID or native handle. Native validation checks the
source and endpoints with the shared aggregate source budget. The complete
receipt includes the exclusive editor-provider and target placement dependencies.

The adapter reads the runtime's actual committed native binding, verifies target
and option attributes/text/topology against private native renderer witnesses,
and accounts for provider and placement subscriptions on its exact private lease.
Unknown, mutated, disconnected or replaced targets, native attribute capsules on
the target/options, pending/revoked source policy,
opaque source inputs, foreign claims/listeners and focused/composing editors block
the complete domain. Disabled/readonly fields can retain an unavailable attachment
with no list claim. Edits remain in the ordinary field/control checkpoint. A source
or target replacement needs fresh native admission and attachment ownership;
old revocations cannot revive. Disconnect releases its exact lease and removes
accounting once. Capture never renders, retries source input or opens a picker.
These are portable capture contracts; semantic resume and automatic recovery remain
separate work.

`CemNativeCapabilitySession.prepare(..., signal, true)` explicitly opts a trusted
host source into capture; declarative local suggestions select this only when
checkpoint publication is enabled. Original bounded XML or explicit CEMB reload
bundles can retain nonempty preparation control frames and additional materialized
native attribute/slice bindings. The optional `preparation` record preserves the
exact `dataJson` bytes of the existing explicitly named native-session control-frame
JSON boundary, effective native value limits, and a `cem-native-bindings/1` table.
JavaScript checks bounded scalar/control serialization and transports opaque CEMV;
native code reimports control-frame JSON through the common document importer and
validates binary integrity/hash, typed graphs, provenance, root indices, attribute
names and optional slice wrappers. Raw slices retain their selected value without
inventing a wrapper. Artifact pools reject duplicate/unreferenced artifacts and
duplicate target/name bindings. Explicit `resolve=false` and execution's default
`resolve=true` are preserved. The `cem-native-source/1` envelope retains
source revision, row count, selection, label selectors, source/binding graph and
directed crossing **intent** with `reacquire` disposition. Crossings never grant
authority on resume. Native validation imports XML normally, verifies/reloads
CEMB source and lexical metadata, and normalizes original-byte SHA-256 digests.
Source bodies share the aggregate 256 KiB resource ceiling; source count caps at
64, binding/crossing and placement records at 256, and rows at 1,024. CEMB sources
retain optional `maxDepth`, `maxWork` and `unresolved` overrides independently for
each source. Omission inherits the native policy; an explicit default remains
explicit. Portable depth/work values must be positive integers no larger than
4,294,967,295 (WASM usize width). The native validator accepts only `mandatory`,
`warning`, `ignore` or `neutral` unresolved dispositions and rejects unknown
fields. XML import has no source-policy override API, so XML envelopes with these
fields remain unsupported. These values never widen a crossing grant: every
source, scope policy and crossing must be freshly admitted on resume.
Live/executable binding authority, nonportable control
frames and unaccounted source ownership remain blocking. Preparation JSON and CEMV
share the aggregate 256 KiB original-source budget, with at most 64 artifacts and
256 additional bindings. CEMV validation also respects the retained effective
limits, including encoded binary overhead and the combined additional-binding byte
payload; capture does not relax them.

Source invocation snapshots copy original bytes and metadata before native
processing, including before asynchronous native-session submission. Execution
and the private checkpoint receive independent copies of the same admitted input;
caller mutation during preparation cannot rewrite either source bytes or selectors. Later caller buffer or selector mutations cannot change the captured
frame's provenance. Passive reference inputs are retained even when a frame does
not use a placement. Adapter state can be acknowledged after its existing
DOM/CSS publication settles, without forcing a template rerender that would
invalidate its source/placement leases. Shared adapters use `readInstanceSnapshot`
for revision-neutral inspection; the existing advancing snapshot API is preserved.

Committed foreign placements are exported from the actual authority registry.
Capture verifies current grants, exact producer ownership/revision/node identity,
requester routes and marker coverage, and retains the complete invocation source
and binding graph, selected source index, selector and producer path. Prepared
transactions, revoked endpoints, forged markers and absent producers refuse the
whole domain. Endpoint resolution currently requires the owning shared runtime;
authored IDs and marker attributes cannot establish ownership.

The acknowledged receipt may carry at most 256 `relationships`: a unique host ID,
kind (`editor-provider` or `placement`), requester/provider endpoints
(`instanceId`, `dataRevision`, owner-local `nodeId`), property and the required
`reacquire` disposition. Native sealing verifies both endpoints against the
complete owner receipt set, rejects duplicate requester/property routes and
multiple suggestions consumers of one exclusive editor, and allowlists placement
properties. Unknown fields, including old tokens, sessions or grants, are refused.
These are scalar dependency contracts, not source ASTs or authority.

Trusted hosts register intent with `coordinator.relationship(descriptor, current,
authority)`, using current runtime/generation/policy authority. The coordinator
copies descriptors and checks endpoint revisions and host freshness before and
after every asynchronous export/seal. Removal of an owner blocks dependent intent;
policy changes revoke registrations until explicitly removed/replaced. Release
functions are idempotent. The native contract is checked even if the host callback
claims currentness. Registration does not bypass unsupported owner export fences.
Runtime owner registrations also derive dependencies directly from their shared
adapters. The coordinator checks complete endpoint publication and graph identity
before/after every asynchronous export and seal; a changed graph invalidates
current bytes while preserving the last good export. Explicit semantic resume
and independently granted source/provider/placement authority are still required.
Never synthesize contracts from authored IDs or `data-cem-placement-ref-*` markers.

`cem_ml::checkpoint_runtime_state::validate` / WASM `validateCheckpointRuntime`
validate this explicit scalar lifecycle boundary. Input and normalized output are
bounded to 512 KiB; resources and consumed writer entries each cap at 256, choice
options/values at 1,024, and original HTTP source bodies at an aggregate 256 KiB.
Unique identities, safe counters, terminal lifecycle state, URL credential rules,
read/resubscribe dispositions and writer suppression are checked. Original HTTP
bytes use a base64 envelope with content type, source URI and a native SHA-256
digest; mismatched digests or malformed bodies fail admission. These are original
source bytes for the normal native import boundary, never a JS document/AST.
The envelope is split between the existing form (capability) and resource domains;
a hydration marker identifies the runtime extension. Generic hydration continues
to refuse it until explicit native/domain semantic resume exists.

Completed HTTP GET/HEAD resources carry `reimport-and-read-again`: a loaded GET
must retain its original native-import source bytes. Repository queries and
storage status retain their complete scalar envelopes, revisions and cursors;
live readers carry `read-and-subscribe`, other readers `read-again`. Local storage
retains exact raw values and existing opaque native JSON value artifacts; pending
native reads/exports block capture. Live/ambient location captures its complete
URL state. Live provider changes invalidate before recapture; storage/location
freshness checks also catch changes ahead of event delivery. Repository values
that DOM-native scalar records cannot preserve (Map, Date, buffers, undefined,
hidden/symbol/accessor properties, cycles, nonfinite numbers or excessive depth/size) refuse the complete domain.

Consumed location trigger tokens and implicit writer indices, plus the resource
`writeDisposition: suppress-until-new-input`, are resume obligations. Reacquisition
must restore writer suppression **before** evaluating the declaration and must
obtain fresh host permissions/subscriptions; it cannot replay an initialization
write or a command. Ordinary rendering retains its existing resource behavior.
Credential URLs, credential request headers, unsupported request methods or
unresolved resource state refuse capture. Whole-island consent must also exclude
secrets in response bytes, parameters, payload and slices; no owner is redacted,
omitted or truncated to make recovery appear available.

Shared `mountCemView` registers trusted command roots. `operation(instanceId)`
issues a host-generated opaque ID before delivery and returns
`{operationId, acknowledge}`; callers no longer supply/reuse operation IDs.
`CemHostCommandRequest.checkpointOperationId` exposes only that scalar identity to
trusted handlers in opted-in domains. Shared storage/location writes use the same
journal. Success acknowledges; throws, rejection and stale/disposed completions
retain `unknown`. No arguments, command names, source values or credentials enter
the journal, and no entry is ever an instruction to replay.

The trusted host may call `reconcileOperation(id, outcome, authority)` after
independently establishing a final `acknowledged` or `not-applied` outcome. Authority must match
`{runtimeId, generation, policyStamp}`; an authored record or late handler callback
cannot establish an external outcome. `retireOperations(ids, authority)` accepts
only terminal records and stages their exclusion from the next checkpoint.
Unknown records cannot be retired. Orphaned records continue to block capture
until explicitly resolved and staged; then every surviving owner must publish
and drain as usual. Journal compaction occurs **only after** a successful native
all-owner seal commits outside WASM. Failure, cancellation or stale sealing keeps
both records and retirement intent; policy changes revoke intent and erase old
private host bytes. Old acknowledgement handles cannot mutate retired entries.

A monotonic host counter and per-coordinator nonce keep IDs non-reusable without
an unbounded tombstone set. Retirement supports bounded repeated operations; the
256-entry journal ceiling still disables recovery if an additional effect cannot
be recorded. Retire before overflow: compaction cannot retroactively account for
an unjournaled effect. Ordinary actions remain available when capture is refused.

Separate `CemElementRuntime` instances can resolve checkpoint endpoints when the
trusted host explicitly gives them the **same** publication coordinator. Each
runtime registers its exact host and a private current-publication resolver.
`coordinator.endpoint(element)` stops at the nearest registered owner; foreign,
unregistered custom-element and data-island boundaries refuse lookup. A rendered
target also requires the renderer's private node/instance witness, matching live
markers and ownership. Copying markers, moving a node from another owner or
reusing an instance ID supplies no authority. Removal/disposal releases the
registration, and duplicate host/owner registration blocks the domain.

This supports shared provider/delegate, placement, geometry and focus dependency
receipts across worker/fallback clients. The all-owner barrier still checks every
acknowledged revision, exact dependency graph, privacy admission and pending
publication before and after asynchronous native validation/sealing. It never
calls an owner's full fingerprint recursively during endpoint lookup. Missing
acknowledgements request a settled interaction retry after peer publication;
this neither restarts an in-flight render nor accepts an unpublished endpoint. A
policy change disables that retry until a new owner publication; old frame
acknowledgements cannot renew themselves across revocation. Policy
revocation erases prior bytes; failed/stale seals keep the last-good checkpoint.
No DOM node, callback, live native handle, provider lease or layout authority is
serialized. Different coordinators remain separate even when their scalar domain
names happen to match.

Automatic capture is not enabled in Studio. Remaining unaccounted widget/source
states, endpoints outside an explicitly shared coordinator, unmanaged native runtime/client adapters,
whole-heap replacement admission, generation replacement, fresh-owner semantic
resume, stale-message fencing and declarative recovery status remain follow-up
work. The complete Studio long-session verifier must remain passing before
replacement is enabled. Saved project revisions and unsaved editor state remain
independent.

Evidence: native `committed_checkpoint`/`runtime_memory` fixtures, shared
`committed-checkpoint.spec.ts`, and real module
`packages/cem_ql/tests/committed-checkpoint-wasm.mjs`. The latter seals under soft
pressure, preserves bytes on failed replacement, and re-admits transport in an
independently instantiated WASM module. It does not claim the previous module was
garbage-collected or that owners have already resumed.

Publication evidence: `checkpoint-publication.spec.ts` exercises all-owner and
admission barriers, input coalescing, async sealing races, privacy/policy erasure,
stale handles, bounded operation outcomes, asynchronous exporter membership races and overflow. `checkpoint-publication.stories.ts` uses real native
sealing with worker and fallback rendering, nested owners, direct edits, snapshot
revision changes, held/failed/repaired CSS, unsupported-owner refusal,
cross-document inert-template traversal limits, reconnect ordinals and continued
ordinary rendering. Existing stylesheet and data-island fixtures remain required.

Control evidence: native `checkpoint_control_state` and `data_island_validation`
fixtures cover bounded normalization, invalid shapes, Unicode selections and
resource/form-domain envelopes. Browser fixtures exercise complete native edits,
the production source editor, composition, property-only invalidation, command
outcomes without replay, fixed URL reads, storage effects and orphan refusal.
The real-WASM fixture validates controls under recovery headroom and proves
reservations release after errors. No Studio UI JavaScript is introduced.

Runtime-state evidence includes native capability/resource and form-domain
fixtures, fresh native source reimport, real-WASM validation under recovery
headroom, production field/checkbox/select/autocomplete browser capture, live
provider changes and explicit orphan retirement. Host journal fixtures cover
current-policy reconciliation, unknown refusal, atomic retirement, failed/stale/
cancelled seals, stale handles and more than 256 repeated retirements without
retaining old identities or replaying effects.

Submenu policy evidence includes native role/default/endpoint validation and
independent-module WASM import, plus production menu worker/fallback plays using
scoped and native `{#...}` references. Closed/open, inherited/viewport boundaries,
none/explicit focus, read-only capture, preserved edits/caret, moved or missing
anchors, resized boundaries, altered style claims, foreign geometry ownership,
revocation, disposal and stale async seals are covered. Invalid state preserves
the last-good host bytes without silently skipping the affected owner.

Native retained-capsule evidence covers native graph/provenance round trips,
shared roots, integrity/hash/binding/private-field rejection, sensitive and
credential source refusal, and aggregate budgets. Worker/fallback plays retain
old source-value capsules across current source changes, preserve edits/caret and
focus, reject metadata replacement and in-place byte corruption, preserve
last-good bytes during stale async seals and dispose/rematerialize normally.
The real-WASM probe reimports renderer-produced capsules into an independent
module and verifies typed integer/boolean values at the explicit JSON export boundary.

Native context/preparation evidence covers original control-frame bytes, effective
limits, shared attribute and wrapped/raw slice artifacts, native privacy/integrity
and aggregate bounds, and independent-WASM validation/import. Worker/fallback
plays verify immutable asynchronous submission copies, actual rendering from a
fresh native session, returned-record isolation, changed/corrupted context refusal,
last-good bytes through stale seals, preserved edits/caret/focus, revocation and
normal close/disposal. Assertions for unsupported owners remain intact.


### Generation retirement and checkpoint/journal handoff

The shared `CemCheckpointPublicationCoordinator.handoff()` binds host transfer to
current all-owner publications, policy, native coverage and the existing sealed
checkpoint store. The store returns owned byte copies only while its exact
successful capture remains current. Pending/replaced captures, input, policy,
ownership or disposal invalidate the handoff. Unknown operation outcomes refuse
handoff until explicit current-policy reconciliation; terminal outcomes remain
in the native-sealed bytes. Last-good exports survive a refusal and never grant
rollback authority. No new event journal or AST serializer is introduced.

A browser/Node command-service client's `retireNativeGeneration(originalProof,
allOriginalEmbeddingLeases, handoff, timeoutMs)` accepts this trusted host adapter.
The host must cover every affected owner and heap separately; passing a rendering
handoff does not cover a command heap. The command heap must have zero live
resources and callbacks. The client retains an independent opaque byte copy,
bounded to 1 MiB, before native preparation. Adapter callbacks are trusted host
lifecycle code, not authored expressions, serialized receipts or checkpoint IDs.

Native preparation validates a fresh complete census at the exact original
confirmed revision and freezes every mutating entrypoint, including cleanup.
Stale cleanup before preparation cancels retirement. The worker privately retains
an original non-cloneable Rust RAII permit; its bounded scalar confirmation is
not a release or replacement permit. Raw cleanup during preparation refuses
without changing the confirmed frozen revision. Only disposal of that original
permit aborts the barrier and reopens cleanup under permanently closed ordinary
admission. No serialized revision can reopen it.

After acknowledgement, final host handoff and original-owner checks and the
termination request share one synchronous host turn. Native mutation stays
frozen until the exact worker is gone. Successful retirement revokes old client
ownership and returns retained checkpoint/journal bytes. Changed publications,
owner joins/leaves, copied/foreign evidence, native changes, malformed replies,
timeouts and worker loss cannot authorize termination. Preparation and abort
waits each have a maximum five-second bound; uncertain or late completion cannot
revive a witness. Failed preparation leaves the worker available for cleanup;
failed abort fences the client and leaves its native barrier conservative.

Native artifact disposal tombstones disappear with that heap only after this
handoff. They are old-handle idempotence history, not portable external-effect
outcomes; old handles never transfer to a new generation. The existing host
journal travels inside the checkpoint. No command or external action is retried.
The returned metadata still says `replacementAllowed: false`. Physical retirement
of shared rendering workers/fallback memories, automatic replacement and semantic
resume remain future work. Raw initialization/direct memory writes, unregistered
embeddings and direct transport-control injection remain unsupported boundaries.
Studio and component UI stay declarative; this opt-in capability lives entirely
in shared runtime support and is not automatically enabled for Studio.

The prepared retirement permit remains visible as one live `native-entries`
root until abort or physical worker termination; a held barrier never produces
a misleading empty resource census.
