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

Settled popup capture requires stable render-node identities and the original
controller, trigger and panel. External triggers require an owned control endpoint
in the complete receipt graph (`trigger-for`), resolved again through the original
interaction scope. Anchor, boundary, focus-target and return-focus roles record
explicit defaults (`invoker`, `viewport`, `auto`/`none`) or owned target endpoints.
`anchor-lost` is `close`/`freeze`; geometry intent is always `recompute`. Typed native
references additionally require the original source callback, fresh validated
source capsules and current placement grants. Serialized selectors/endpoints never
grant authority. Unowned targets, duplicate names, replaced controls/controllers,
changed policies and missing endpoint coverage refuse the complete handoff.

Both open and closed popups retain scroll and open intent. Open capture and
activation of external/explicit geometry require current shared geometry evidence,
including visual viewport, anchor/boundary rectangles, containing block and
geometry style ownership. Internal natural absolute panels keep their existing
containing-block scroll behavior; fresh admission still repositions them. Fresh
admission recomputes placement through the retained controller without reopening,
clicking, moving focus or restoring old pixels. Unprocessed geometry changes refuse;
ordinary geometry observers can settle new layout before a new checkpoint.
Private guards include external trigger attributes and original DOM identities,
so a matching render-node string on a replacement cannot authorize recovery.
Parent reconciliation compares the previous and fresh authored child inputs: an
unchanged input cannot reset a child's live open state or move focus during a
resource-owner render. Changed and removed authored inputs still apply normally.
Focus references use authored or freshly granted native identities; adding an
unowned ID to another renderer's native output does not make it durable. Settled native dialog/popover
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
Text `local-storage` declarations (omitted or literal `type=text`) now use the
shared processing host. Native compilation returns a scalar routing policy;
JavaScript does not parse CEM-ML attributes or receive its AST. Processing
protocol v29 transports their named resource controls, including empty explicit
values. Unsupported storage types and location controls cannot enter that path
through dynamic output either. JSON/XML/dynamic-type storage and legacy location
declarations retain their ordinary renderer and remain unsupported for resume.

Text-storage resume uses the original retained owner, storage object, key, type,
live policy and exact raw value as private authority. Fresh source compilation
and native island/runtime validation precede activation. Key/source changes,
replaced storage objects or I/O callbacks, withdrawn preview permissions and
changes ahead of storage-event delivery refuse the complete handoff. Original live listeners
are detached while the ticket is parked; fresh listeners bind only after every
owner has activated and completed renewal. Cancellation cannot reconnect a
retired generation. The first fresh render keeps the original edits, selection,
theme and independent page/example keys, with initialization and slice writes
suppressed until new accepted input. Subsequent selection, external storage
events and key rebinding use the shared resource lifecycle normally. No theme-
or Studio-specific recovery behavior is added. The complete production theme menu
uses the shared external-trigger/viewport popup policy above; every popup, menu,
icon and storage owner participates. Theme and example keys remain independent.
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
The returned metadata still says `replacementAllowed: false`. Shared rendering-worker retirement is described below. Physical retirement of
shared fallback memories, automatic replacement and semantic resume remain future work. Raw initialization/direct memory writes, unregistered
embeddings and direct transport-control injection remain unsupported boundaries.
Studio and component UI stay declarative; this opt-in capability lives entirely
in shared runtime support and is not automatically enabled for Studio.

The prepared retirement permit remains visible as one live `native-entries`
root until abort or physical worker termination; a held barrier never produces
a misleading empty resource census.


### Shared rendering workers and the fallback disposal boundary

The opt-in publication coordinator now provides a single-generation lifecycle:

```ts
const drain = await publication.closeRenderingGeneration();
await publication.capture();
const retired = await publication.retireRenderingGeneration(drain);
// retired.checkpoint owns portable bytes outside the terminated WASM worker.
// retired.replacementAllowed remains false.
```

Closure first stops host admission across the actual pooled worker and drains
queued work, native callbacks and stylesheet I/O. It then closes ordinary native
entrypoints in that exact linked memory. Capture must run again after closure;
an earlier checkpoint is not current authority. Every pooled client, published
owner and original retained-source witness must be covered by the same current
private drain. Copied receipts, omitted peers/sources and cleanup mutations
refuse preparation. Unknown external outcomes refuse checkpoint handoff.

Retirement retains an independent host byte copy (at most 1 MiB), then performs
terminal cleanup of the rendering engine's owned sessions, documents, templates,
XPath libraries and stylesheet leases. The existing complete native census and
original RAII permit are reused: unmanaged retained native roots still refuse
the zero-blocker freeze. The worker keeps the permit privately; only bounded
control metadata crosses the transport. The final current checkpoint/source/
owner check and exact worker termination share one host turn. Old native
handles lose authority synchronously. Existing light DOM, form edits, focus,
selection and host checkpoint/journal bytes stay available.

Unlike command-worker preparation, rendering preparation may already have
released owned resources when a native blocker or newer input is discovered.
A failed preparation therefore fences the old managed generation permanently;
it never tries to reconstruct those handles or select fallback. The host
checkpoint remains exportable, while its old receipt cannot resume anything.
Abort drops an original held permit when possible and leaves native cleanup
under closed admission. Preparation and abort each have a maximum five-second
wait. Timeout, worker loss, malformed confirmation or late completion cannot
revive retirement authority or replay actions. Normal explicit scope disposal
continues to tear down its worker and releases pool ownership.

Default legacy main-thread/fallback clients share one WASM memory with standalone native
helpers and possible raw aliases. The default legacy module loader has no exclusive
memory disposal or fresh-module ownership boundary. Physical fallback retirement
therefore rejects with `cem.runtime.retirement.unsupported` before changing
admission or checkpoint state. It does not claim that revoking a handle frees
that memory. Multi-generation publication domains similarly refuse a partial
transaction before closing either heap. Coordinated multi-heap replacement,
semantic resume remain explicit next work; this API does
not enable automatic recovery in Studio. All behavior stays in shared
`cem-elements` runtime support; components and Studio UI remain declarative.


### Opt-in owned fallback generation

Before the first shared native call in a realm, an embedding can select an
isolated main-thread fallback:

```ts
import { isolateCemFallbackGeneration } from '@epa-wg/cem-elements';

const fallback = isolateCemFallbackGeneration();
await fallback.ready();
const owner = fallback.claimOwnership();
// Use the normal shared CEM render/query/XPath helpers.
// Dispose every managed host, helper, library and retained resource first.
const disposed = await fallback.disposeEmpty([owner]);
// disposed.releasedOwnedReferences === true
// disposed.replacementAllowed === false
```

The generated factory gives each instance private wasm-bindgen bindings,
finalizers, views and WASM memory. Shared render/query/XPath entrypoints use the
selected instance; raw legacy bindings retain their independent memory. Selection
is permanent for this realm and must precede any shared native use. No existing
alias is retargeted, and a retired or failed instance cannot be initialized again.
The public controller exposes no raw memory, module or native permit.

Each embedding claims an original ownership lease. Empty disposal requires all
current original leases exactly once; copied, foreign, released, omitted or
repeated leases refuse admission. Consent reads bounded own data slots without
calling array methods, iterators or accessors; reentrant disposal and ownership
changes during property inspection also refuse admission. Ownership changes
during draining or native preparation cancel release. All managed clients and resources must already be
gone. Closure then drains with a positive timeout of at most five seconds and
uses the existing full native inventory and original zero-root freeze permit.
An unmanaged native root still refuses disposal. Refusal after closure leaves
ordinary admission closed while permitting cleanup and a fresh disposal attempt.

Successful disposal drops the factory's WASM exports, instance/module and cached
views, plus the host domain's memory and telemetry references. Saved call aliases
and initializers remain fenced. Late finalizers cannot call the discarded
instance. This reports release of owned references, not synchronous browser
collection or an RSS reduction; externally retained raw references are outside
that claim. Initialization failures are terminal for that selected instance.

Empty disposal requires every managed client/resource to be released first.
Active retirement uses the additional checkpoint/journal transaction below.
Replacement, resume, automatic recovery and Studio status remain subsequent
work. The default legacy fallback behavior is unchanged.


### Active owned fallback retirement and checkpoint handoff

An owned fallback can retire active processing hosts/scopes through the same
publication coordinator used for worker retirement. Complete embedding consent
must be issued before closing admission and passed unchanged to both calls:

```ts
// After isolateCemFallbackGeneration(), ready(), and ownership claims:
const consent = fallback.retirementConsent([owner]);
// Every participating host, published owner and retained source belongs to
// this coordinator; include every embedding lease in retirementConsent().
const drain = await publication.closeRenderingGeneration(5000, consent);
await publication.capture();
const retired = await publication.retireRenderingGeneration(drain, 5000, consent);
// retired.checkpoint is an independent host copy; replacementAllowed is false.
```

The opaque consent token binds the selected actual memory and current original
ownership set. Copies, foreign tokens, changed ownership and a different consent
for an original closed drain refuse retirement. Consent array validation keeps
the same bounded own-data-slot policy as empty disposal. No token can authorize
another heap. Default legacy fallback remains unsupported before changing
admission, checkpoint state or native owners; worker call signatures remain
compatible without a consent argument.

Closure drains admitted native work and stylesheet I/O before closing ordinary
native entrypoints. Capture must then acknowledge the current complete owner,
client and retained-source set. Unknown external-operation outcomes refuse
handoff; acknowledging terminal outcomes requires another capture. Retirement
retains independent bounded checkpoint/journal bytes before any terminal cleanup.
Every covered client must have the original shared processing-host cleanup
adapter; omitted peers, missing adapters, copied drains or stale sources refuse
preparation.

Fallback cleanup is synchronous. The domain permits only covered native resource
deletions without changing the original host coverage revision. Membership,
input, ownership, fresh work and unrelated ledger mutations keep their usual
invalidation. Processing engines release their owned templates, documents,
sessions, XPath libraries and native stylesheet leases; source/owner metadata
stays available solely for final freshness validation. Existing light-DOM
stylesheet installations and edited controls are retained.

After cleanup, the private factory uses the existing complete native census and
original zero-root freeze permit. Post-cleanup host freshness callbacks run only
while that native freeze is held, so raw cleanup cannot race the last check.
Original consent, drain coverage, sources and checkpoint/journal currentness are
checked again before owned references are dropped. The positive preparation
deadline is at most five seconds and is rechecked after synchronous cleanup and
final callbacks. It refuses late commit authority; it cannot preempt synchronous
JavaScript or native CPU work.

Success clears factory and domain memory/telemetry references, invalidates old
native handles and preserves independent checkpoint bytes, styles, edited values,
focus and selection. Callback installation uses a separate scope from native
initialization so endpoint closures do not retain its memory/module locals.
Release still reports owned references, not synchronous browser GC or an RSS
bound. Preparation failure fences the managed generation, preserves last-good
host bytes and edited DOM, and never selects fallback or replays actions. A heap
blocked by unmanaged roots can remain quarantined until realm destruction;
failure does not claim that its memory was released. Earlier refusal before
cleanup retains closed admission and ordinary cleanup ownership. No replacement,
semantic resume or automatic Studio recovery is enabled by this transaction.

### Coordinated fresh-generation allocation and admission

`CemNativeGenerationGroup` coordinates an explicit complete set of managed
physical generations. Enroll original participants before retirement; a
publication coordinator exposes its exact rendering generation, an owned
fallback exposes the same participant through `generationParticipant()`, and a
browser/Node command-service client exposes its dedicated command worker.
Including both aliases of one physical participant is a duplicate, not another
heap. Rendering publication still requires complete client/source coverage;
command/fallback retirement still requires every original embedding lease and
the current acknowledged checkpoint and terminal journal.

```ts
import { CemNativeGenerationGroup } from '@epa-wg/cem-elements';

// Enroll every physical heap in this explicit recovery group before retiring.
const group = new CemNativeGenerationGroup([
    publication.generationParticipant(),
    commands.generationParticipant(),
]);
// Use the existing close/capture/retire protocols, with complete original
// fallback consent or command embedding ownership as applicable.
const admission = await group.allocate([renderingRetired, commandRetired]);
for (const generation of admission.generations) {
    // New generation-specific runtime handle and independent host bytes.
    console.log(generation.previousRuntimeId, generation.runtimeId);
}
// Explicitly fence the whole admitted group and request each physical cleanup.
admission.revoke();
```

Only original successful retirement result objects are accepted. Copies,
serialized receipts, scalar native confirmations, empty-disposal reports,
omitted/duplicate peers and receipts already consumed by another admission
cannot authorize allocation. Each participant belongs to one original group.
Caller arrays are bounded own-data-slot snapshots; accessors, iterators and
reentrant inspection cannot substitute members or supersede a running group.
The group retains its own bounded checkpoint/journal copies, independent of
caller modifications to returned arrays. The default deadline is five seconds,
with a positive maximum of five seconds, at most 16 heaps and an aggregate
four-MiB checkpoint limit (one MiB per retirement).

Allocation creates distinct WASM factory instances or dedicated workers using
the original shared runtime's captured setup. Fresh initializers and private
headroom probes run while shared operation handles remain inaccessible. Every
candidate must report native admission headroom and retain its physical
identity/liveness. A single synchronous host gate then admits the complete set;
no callback or asynchronous gap opens one peer before another. Allocation failure,
expired initialization, failed headroom or stale identity fences all candidates.
A failed allocation group is terminal; it cannot reuse the old receipts for another attempt.
Cancellation aborts owned fallback fetch/startup and terminates new workers;
a late successful initializer discards its result without opening admission.
Final synchronous probes recheck the deadline, but cannot preempt synchronous
native/JavaScript CPU work. Revocation fences handles immediately and rejects in-flight rendering requests
before detaching their transport. Node worker termination may finish asynchronously. Reference release never guarantees
synchronous GC or an RSS bound.

The returned `runtime` is generation-specific: `CemFreshFallbackGeneration`
provides a trusted `invoke` boundary limited to the existing shared native
entrypoints; `CemFreshRenderingGeneration` provides the typed processing
transport; a command generation is a new browser/Node command-service client.
The previous runtime aliases, processing hosts, source handles and clients stay
retired. The selected global fallback is never reassigned. Vendored copies of
the coordinator share original capabilities within the same realm, allowing a
rendering/fallback generation and a separately linked command worker to join
one transaction. The coordinator retains callable controls and weak identity history; it drops
candidate objects and strong identity sets before admission/after revocation,
so retained host admission handles do not keep discarded fallback memory alive.
The registry's registration/retirement issuers remain internal
trusted-host APIs, not authored CEM data or a security sandbox for arbitrary JS.

This is explicit allocation/admission, not semantic restoration. It does not
rebind existing processing hosts or owners, import checkpoints, restore widget
state, reacquire resources, apply stale checkpoint bytes to newer edits, replay
external actions, or enable automatic Studio recovery. Those require the next
resume contracts and renewed original ownership on the fresh generations.
Unknown operation outcomes still refuse the original retirement handoff. Only
the declared complete group is coordinated; unknown heaps, arbitrary raw
initializers, legacy fallback and shared-memory/native aliases outside these
ownership contracts receive no replacement authority. No Studio/component UI
JavaScript or theme/storage changes are introduced.

## Complete Studio admission acceptance

Studio's shell installer accepts an original `CemElementRuntime` through
`installCemStudioShellComponents({ runtime })` before its first call. The default
installation is unchanged; subsequent runtime injection refuses. This is host
service configuration, not an app-owned renderer or recovery implementation.
Studio composition remains declarative.

`CemCheckpointPublicationCoordinator.inspectRecovery()` returns immutable,
detached named lifecycle metadata: registered owner IDs/tags, acknowledgment and
adapter presence, open admissions, capture/block/disposal flags and the number of
unknown operation outcomes. It invokes no owner-currentness probes, serializers,
privacy callbacks or native work. Adapter presence is **not** a currentness,
source-policy, complete-coverage or recovery proof. Only the original capture,
retirement and fresh-resume gates authorize handoff; the report cannot replace any
of them. No source bytes, commands, callback objects or native handles are exposed.

The actual Studio shell/workbench has a separate repeated-session refusal lane in
`recovery.browser.spec.js`, included in `test:workbench` and the release gate. In
isolated worker and owned-fallback realms it clones Login, edits Unicode source,
retains paint/focus/selection and the draft journal, refuses partial capture/resume,
and continues exact Save and native graph builds. Unknown journal outcomes remain
unknown until explicit acknowledgment. Each realm owns a temporary database;
existing user storage is not cleared.

The release audit also requires checkpoint provider-handoff, retained-form and
reference-resume helpers beside the CEM facade in both Studio and Site source and
destination maps. Required-asset checks and existing byte-identity/relative-import
checks prevent a source-only test pass from masking missing deployment modules.
The exact workspace dependency audit includes declarative Storybook fixture routes;
published runtime dependencies remain unchanged.

This audit does **not** pass successful whole-workspace recovery. Theme text
storage and its external-trigger/viewport popup now have shared adapters.
Media/live previews own additional runtime state.
Shared action/disclosure/list owners now have guarded adapters, including
complete command/descriptor/submenu endpoint coverage.
Command-service heaps also need explicit host-service rebinding and complete group
coverage. These owners must not be omitted, silently reconstructed or treated as
plain retained templates. The ordinary Studio release/navigation/live-view gates
and individual real-heap resume tests remain complementary evidence. A bounded
editing/build probe is not an OOM/RSS guarantee. Successful complete-group Studio
retirement/resume and declarative recovery status remain on the active checklist;
automatic Studio recovery stays disabled.

## Explicit owner resume transactions

Fresh allocation and semantic owner admission are separate gates.
`resumeCemCheckpointOwners` now coordinates explicit **shared semantic adapters**;
it does not infer an adapter from an island or restart Studio. Built-in plain
retained CEM-ML owners provide adapters for canonical slices, native form-control
properties and approved HTTP source bytes. The shared plaintext-editor adapter
also admits native-validated live text, UTF-16 selection and consumed selection
requests. Standalone form/checkable/select/editable-choice owners likewise retain
their original native controls and private reset/validity/selection state, including
multiple values and consumed requests. Preparation compares the native-validated
DTOs and exact control/form identities with live state; activation rechecks private
metadata, host markup, composition and accepted input sequence. It neither restores
old edits over current controls nor emits input/change. Exact one-shot change
suppression after a completed provider commit remains effective across retirement,
and a subsequent edit emits normally. Shared provider relationships additionally
require complete paired activation and fresh native source authority. Retained trees, tabs, navigation, menus,
popups and settled native surfaces now have shared adapters too. Browser modal
visibility, edits, roving state, focus and controller metadata remain live; geometry
is rechecked without invoking open/command/focus policy again.

Shared action-command/action-control, disclosure-panel and native-list owners also
have retained resume adapters. Native validation admits only bounded lifecycle
DTOs; original host-only witnesses retain controller, control, option and form
identities. Command targets, interaction descriptors and projected submenus require
complete current endpoint coverage. Changed targets, same-markup replacement nodes,
form routing or controller replacement refuse preparation and late activation.
Disclosure IDs, heading relationships and expanded intent stay live. Visible native
listboxes retain selected properties, original options and scroll; native dropdown
picker modes remain refused because their open state is unobservable. Actions
retain transient invocation metadata without serializing or invoking it, and do not
click, submit, open or navigate during resume. Unknown interest relationships and
unowned routes remain refused. New explicit user actions work after publication
renews on the fresh generation.

Passive reference/placement consumers reauthorize through the original host callback
and registry. Fresh native validation checks opaque source bytes/digests and policy
against the sealed intent. Fresh private placement tokens derive from current
original grants, never from saved IDs, and preserve dependency revocation. There is
no DOM reservation/publication while inactive; the first fresh frame reauthorizes
once more before publication. Copied snapshots, changed/withdrawn source policy,
missing authority, stale producer identities and revoked grants refuse. Generated
producer IDs survive merging only while privately reserved by that registry.
Snapshot inspection is read-only, and an unchanged first fresh frame retains its
sealed data revision under a one-use original active-context guard. This avoids
manufacturing source changes from heap replacement; ordinary subsequent renders
still allocate new revisions.

Paired active suggestions, standalone manual listboxes and native datalist
providers now have shared resume adapters. Every participating route binds before
provider/source renewal, independent of the order in which the host calls returned
owners. A bounded presentation pause is issued only after all original witnesses
pass. Consumer presentation pauses precede provider claim pauses; consumed tickets
transfer cleanup ownership to the fresh transaction. Late host edits, replaced
controls, competing browser claims, revoked source policy or mismatched native
capsules refuse the whole group without restoring old values or replaying commits.

Canonical local suggestions reimport their original inert source and renew native
publications. External `CemSuggestionsControllerOptions` and standalone
`CemManualListboxLifecycle`/`CemStandaloneDatalistLifecycle` must provide the original
trusted `resume(freshHost)` callback. It must recheck host source permission, prepare
fresh native sources/publications and return the current lifecycle/options; the
host's normal binding callback must expose that new publication. No callback means
explicit refusal, never inferred permission from saved bytes or DOM IDs. Native
validation compares fresh source/preparation policies and digests with the sealed
capsules. Active/committed proof indices, values, availability and rendered rows
must match freshly acquired native identities. Readonly retained proofs also require
fresh native validation. Browser controls, authored defaults, focus/selection and
one-shot consumed-change state remain live; no selection/value is recommitted.

Standalone listbox surfaces opt into `reset-on-resume`; native datalists require
unfocused capture and reset native picker presentation while reacquiring the owned
`list` relationship. The shared adapter waits for actual native presentation events.
Only toggle delivery from the exact admitted presentation surface is excluded from
new-input accounting during that bounded renewal; keys, focus, edits and commands
retain ordinary invalidation. Unknown/withdrawn source policies, inactive
sources without an admitted readonly proof and unsupported storage/location rendering still refuse with
`cem.checkpoint.resume.unsupported`. Generic HTML hydration refuses checkpoint
extensions. Full long-session acceptance and declarative recovery status remain
necessary before automatic Studio recovery.

A trusted shared owner can register `resume` alongside its existing publication
endpoint. `witness()` captures a host-only currentness check before retirement.
It must detect property-only edits, disconnects, changed declarations, source
policies and widget state without reading the subsequently retired heap.
`prepare(context)` receives the exact native-sealed opaque island, frozen owner
receipt, fresh runtime and its original previous/fresh runtime identities. It
must use native lifecycle/source validation and import, reacquire current host
permissions, and install consumed-writer suppression before evaluating anything.
It returns an owner object and original bound `current`/`discard` controls.

Every prepared owner must obey `context.active()` and `context.signal`. While
inactive, it must not publish, accept input, subscribe to live sources, invoke
commands or deliver external effects. Source imports and authority preparation
must be reversible. Widget state is intent: native datalists reset presentation,
providers and placements acquire fresh grants/proofs, and focus/geometry follow
the existing explicit policies. The transaction does not manufacture these
permissions or replay even a terminal `not-applied` operation. Unknown outcomes
refuse a ticket before fresh preparation starts. These obligations belong in
shared runtime adapters, never application-local UI JavaScript.

For an already enrolled, single rendering generation with complete adapters:

```ts
const group = new CemNativeGenerationGroup([publication.generationParticipant()]);
const drain = await publication.closeRenderingGeneration(5000, consent);
await publication.capture();
const ticket = publication.prepareResume(); // after final capture, before retirement
const retired = await publication.retireRenderingGeneration(drain, 5000, consent);
const admission = await group.allocate([retired]);
const resumed = await resumeCemCheckpointOwners(admission, [ticket]);
// Use the fresh gated owner objects in resumed.owners.
resumed.revoke();
```

An explicit complete multi-generation transaction requires one original ticket
for every original physical generation, with exact private retirement bytes.
There are at most 16 tickets, 256 owners overall, four MiB of checkpoint bytes
and a positive preparation deadline no greater than five seconds. Command heaps
without owner-resume tickets cannot be silently omitted from a mixed group.
Copied admissions/tickets, duplicate or missing peers, reused authority,
checkpoint edits, changed policy/input/publication and unsupported owners refuse.
Array snapshots read own data slots without invoking caller iterators, methods
or accessors. Original admission claims lock before running freshness probes to
prevent reentrant double claims. Caller mutation of public checkpoint exports
cannot change the private resume input.

The publication coordinator retains one pending ticket. New tickets supersede
old ones; host input, owner removal, policy changes and disposal erase its private
bytes and cancel any active preparation. Current privacy consent is checked
again. Native retirement invalidates ordinary capture but does not, by itself,
invalidate the separate host-only resume witness. Successful or failed attempts
release old adapter/witness references and erase private ticket bytes; last-good
checkpoint storage remains governed by its existing policy/disposal rules.

All preparation and fresh-native/owner probes run before one synchronous owner
gate opens, with no activation callbacks. Timeout, explicit revocation, stale
input or preparation failure closes the gate, aborts preparation, discards every
prepared owner and revokes the entire fresh group. Late completions receive
cleanup and cannot become active. Shared revocation listeners run after the
native work gate closes and before physical cleanup. Fresh fallback handles
provide `release(name, args)` for the explicit native disposal/release entrypoints
only, allowing source cleanup in this window; allocation, evaluation and other
entrypoints remain fenced. After physical disposal this cleanup path is retired
too. Adapters must release partial native allocations on their own failure.

The transaction does not replace rendered DOM. Focus, caret and newer edits
remain with their current owners; a failed attempt cannot roll back to saved
pixels. Chromium acceptance exercises both trusted-host preparation and built-in
plain retained CEM-ML owners on actual fresh workers and owned fallback heaps.
After admission, call each returned `CemResumedElementOwner.resume()`. The first
call checks the complete staged set and binds every processing/publication route;
only then does bounded, one-shot source/provider renewal run. Each owner evaluates
on the fresh context, reusing an already renewed native provider frame. Rebinding requires an original
active context issued by that same coordinator for that owner, its retired
predecessor and the exact fresh native generation. Copies, foreign owners, live
predecessors, duplicate binding and revoked contexts refuse. Event sequences and
terminal journals survive; activation does not manufacture input.

The binding routes subsequent rendering and HTTP refresh to the new host and
keeps installed styles and unchanged DOM. Native controls/runtime validation and
checkpoint sealing use a bounded read-only recovery channel on that same heap,
including after ordinary admission closes. These operations retain drain
barriers without changing managed publication revisions. Native admission and
inventory counters remain authoritative and are revalidated at retirement;
preparation/freeze refuses overlapping recovery work. Worker retirement control protocol v2 carries
these named validation/sealing requests. Processing protocol v27 additionally accepts the trusted canonical DOM adapter’s
XML serialization, so namespace-qualified island parts and HTML void payload
can enter the existing native XML importer without relaxing HTML validation.
Original sealed bytes remain the integrity authority; no source AST crosses JS.
The adapter compares those bytes directly with the original retained island;
HTML reparsing would normalize authored CRLF. Plaintext-editor activation keeps
the editing subtree, native undo/redo, focus and selection, refuses later edits,
and emits no input. Composition/deferred values and unreported DOM text refuse
capture. Old paint requests are cancelled and native role ranges regenerated on
the admitted fresh generation; paint/job identities are never durable state.
Unsupported resource adapters, complete preview/command semantic handoff and
declarative recovery status remain subsequent work; Studio is not automatically
resumed. Explicit runtime enrollment is described below.

## Explicit runtime recovery enrollment

`CemNativeRecoveryScope` is a shared host membership boundary exported by
cem-elements and the browser/Node command packages. Enroll every rendering
runtime, live-preview child, command worker and command-invocation heap in the
same scope. `inspect()` returns detached labels without invoking endpoints or
reading source/private checkpoint bytes. Labels and inspection results confer
no authority. Authored markup and preview resource envelopes cannot enroll a
runtime or supply lifecycle callbacks.

`prepare()` probes every original endpoint with a bounded cancellation signal.
Busy work, unknown write outcomes, unsupported heaps, copied participants,
overlapping physical generations and membership changes refuse the complete
preparation. The returned `createGenerationGroup()` is one-shot and binds a
private membership/currentness guard through fresh allocation. Preparation does
not close native admission, capture owners, retire heaps, issue semantic resume
tickets or replay external actions. Each original adapter still needs its exact
ownership census, native closure, committed checkpoint and terminal journal.
Independent coordinators sharing one fallback memory require combined owner
coverage; they cannot retire that memory independently.

`CemElementRuntime({ recoveryScope, checkpointPublication })` enrolls its
rendering domain. A runtime without publication consent remains an explicit
blocker. Shared live previews inherit the parent's worker policy and a separate
checkpoint coordinator with the original privacy/sealing policy. Their source
bodies and allowed resource maps stay copied outside WASM for native reimport;
retirement does not navigate/remount the frame. Child scope disposal removes
membership and disposes its inherited coordinator. Parent privacy changes erase
child checkpoints; parent disposal closes children, including nested previews. Native source callbacks and
existing owner adapters remain the source authority on fresh processing contexts.
The retained parent media-preview controller still requires its own coordinated
resume contract; child enrollment alone never establishes whole-workspace resume.

`createEnrolledBrowserCommandServiceClient(options, scope, id, hostWitness)`
reserves membership before asynchronous startup and returns the client plus its
original enrollment. Failure removes the reservation after worker termination.
`client.enrollRecovery(scope, id, hostWitness)` enrolls an already ready worker. The
original host witness must account for application ledgers and pending writes;
a boolean supplied by an authored request is not a witness. Active executions,
transport actions, host callbacks and uncertain write outcomes refuse. New work
or host-ledger changes invalidate prepared membership. Original native inventory
and artifact-disposal obligations remain enforced at retirement. Fresh workers
receive snapshotted original host services/configuration; old handles keep their
old identity and commands are never replayed. The host must explicitly remove
the retired enrollment and enroll the admitted fresh client after coordinated
binding. Recovery does not silently retarget existing client aliases.

Browser invocation construction, synchronous graph authoring and presentation
have an owned shared binding, `createBrowserCommandInvocationBinding()`. Studio
uses this binding instead of initializing legacy singleton aliases. The low-level
`@epa-wg/cem-ml/wasm/factory` export generates a lexical closure per physical heap:
bindings, memory views, finalizers and initialization state are independent, with
no eval, blob modules or per-generation URLs. Initialization validates native
headroom before creating a candidate. Candidate work stays gated until the entire
original native generation group is admitted.

Public enrollment inputs use the shared scope's structural enrollment method,
so independently emitted host/CLI declarations remain compatible. Runtime
participants, ownership, receipts and admissions still require exact original
capabilities from the shared realm registry.

The authoring runtime exposes only three pure native operations. Raw sources and
named invocation/graph/presentation DTOs cross the API; no JavaScript CEM AST or
IR is introduced. Discovery holds admission across resolver awaits, and synchronous
graph/presentation calls invalidate prepared host witnesses. Complete original
embedding ownership, managed drain, native census/closure and an original terminal
host handoff are required for retirement. The original native RAII permit freezes
mutation before the final host witness and physical disposal; failed preparation
releases the permit without reopening ordinary admission. Successful disposal
clears all binding/cache/module/instance references and fences old runtime aliases.
Participant identity uniqueness uses weak registries, and checkpoint mutation
callbacks keep no shared closure with the physical memory witness. Revoking owners
therefore releases that witness even when the original controller remains retained.

`binding.resume(originalAdmission)` explicitly consumes a complete single-generation
pure-authoring admission, checks its original previous runtime identity and adopts
the fresh original runtime. Stable binding methods remain synchronous where they
were synchronous before. Copies, foreign bindings, used admissions, mixed groups
and revoked candidates refuse. Revocation of an older retired admission cannot
close a subsequently admitted generation. Startup failure and binding closure
remove only their own enrollment; allocation/resume replay no resolver or command.
The pure heap retains no requests, native handles or external effects to restore;
its independent terminal host handoff is retained by the original group receipt.

Legacy singleton APIs retain their old identity and behavior.
`enrollBrowserCommandInvocationRecovery()` still records that unsupported legacy
heap, so a worker-only group cannot masquerade as complete recovery. Owned
single-generation resume is not combined Studio semantic resume: retained media
preview controllers, combined preview/rendering/command handoff including shared
fallback owner coverage, repeated whole-Studio acceptance and declarative status
remain required before automatic recovery.
