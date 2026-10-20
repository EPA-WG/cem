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
Explicit menu geometry/focus policies still require their own adapters.
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
Preparation, closing/exit work, pointer or activation-key gestures, queued activations,
active tooltip interest/timers, native context handles, external geometry and
unsupported deferred-body projections still block capture. A gesture settles after
native activation dispatch; disposal removes its listeners and pending timer.
Deferred native dialog bodies retain a native-validated `body` contract: current
template render-node identity, `retain`/`dispose` mode, materialization state and
at most 256 unique root identities, with `rematerialize` disposition. Materialized
bodies also carry an immutable original `cem-render-projection/1` record.
`reuseOnOpen` preserves whether the original template is still the current node;
replacing that node keeps retained output until the next ordinary open replaces it. This is
an explicitly named scalar renderer-output checkpoint transport, not authored
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
bounded to 512 KiB. Opaque native attribute capsules cannot enter this scalar
transport and still refuse capture. Password/file types and sensitive autofill
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
checkpoint publication is enabled. Only original bounded XML or explicit CEMB
reload bundles with empty scalar input and no additional native slice/attribute
bindings are currently portable. The `cem-native-source/1` envelope retains
source revision, row count, selection, label selectors, source/binding graph and
directed crossing **intent** with `reacquire` disposition. Crossings never grant
authority on resume. Native validation imports XML normally, verifies/reloads
CEMB source and lexical metadata, and normalizes original-byte SHA-256 digests.
Source bodies share the aggregate 256 KiB resource ceiling; source count caps at
64, binding/crossing and placement records at 256, and rows at 1,024. Custom
source traversal policies, nonempty input data, additional native bindings and
unaccounted source ownership remain blocking.

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

Automatic capture is not enabled in Studio. Remaining unaccounted widget/source
states and cross-runtime endpoints, physical runtime-domain discovery,
memory-pressure admission drain, generation replacement, fresh-owner semantic
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
