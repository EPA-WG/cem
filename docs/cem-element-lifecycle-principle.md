# A DCE Instance Has One Durable Data Island and Derived Rendered Output

**Status: normative.** This is the basic lifecycle rule for produced custom
element instances managed by `cem-elements`. It takes precedence over lifecycle
examples and design text that reuse an authored payload envelope as the data
island or treat rendered children as payload after an island exists.

## The rule

A produced DCE instance owns one stable, direct, inert WHATWG template marked
`data-cem-island="instance"`. The template contains exactly one
`cem-island:context-root`, currently at lifecycle version `0.2.0`. Its qualified
DOM is lowered into the canonical namespace-aware CEM AST instance state.
Rendered light DOM is a derived projection outside that template; it is never
the data source once the island exists.

The three instance regions have distinct ownership:

```text
produced DCE instance
├── template[data-cem-island="instance"]  canonical inert state
│   └── cem-island:context-root            CEM AST context/error boundary
│       ├── cem-hydration:data
│       ├── cem-attributes:attributes      canonical host attributes
│       ├── cem-dataset:dataset            validated attributes projection
│       ├── cem-payload:payload            explicit content/schema handoff
│       ├── cem-slices:slices
│       ├── cem-resources:resources        portable semantic runtime state
│       ├── cem-form:form-state
│       ├── cem-validation:validation-state
│       └── cem-events:event-state
└── rendered range                         derived or provisional output
```

The island marker is a runtime and serialized-lifecycle identity. It is not an
authoring hook or an authored CSS selector. The prefixed names and `xmlns:*`
attributes are semantic namespace input in an HTML document: outside XHTML,
SVG, and MathML, the browser does not create custom DOM namespaces. The
`cem-element` HTML adapter therefore resolves lexical prefixes and namespace
declaration attributes while lowering the subtree into CEM AST. XML/XHTML input
uses its native namespace information and must produce the same expanded names.

The island schema owns topology and the context-root error boundary. Qualified
domain namespaces own each part's meaning, while payload, resource result,
slice, and event subtrees preserve the namespace and schema of the AST that
produced them. A source-language schema such as JSON is never selected merely
because state is record-shaped; foreign languages enter only through an
explicit content-type/schema handoff.

## Native slice values

`DataIslandSnapshot.nativeSlices` carries portable native CEM value artifacts,
separately from the scalar/control `slices` record. A native slice has a null
placeholder in that control record; only the native binding channel supplies
its query value. Native attributes use the same binary graph format. Neither
channel serializes a document into JavaScript records or reconstructs nodes
from rendered DOM text.

Saved hydration and explicit edge export encode native slices in the named
`cem-native-slices-v1` binary envelope. Event bindings may retain a native
`slice-value` attribute wrapper; the receiver validates its name and unwraps
its authoritative native value sequence. References, types and source metadata
remain in the artifact. Environment limits apply and CEM scopes may lower them.

For `local-storage @type=json`, CEM-ML imports source bytes to a native document.
An edit supplies a replacement native value and CEM-ML exports JSON. Reading
never rewrites unchanged source bytes. Failed reads clear the native binding
and preserve raw storage; failed writes preserve storage and restore the last
accepted binding. A host null/empty sequence removes the key; a generic-data
JSON-null node writes `null`. Live changes, key rebinding, disconnect and direct
or ancestor scope disposal invalidate older asynchronous work. Reconnect reads current storage.
Portable artifacts belong to durable slice state; temporary WASM decode handles
are released after each render, and storage I/O retains no WASM handles.

Plain text `local-storage` uses shared worker/fallback processing. Its retained
checkpoint adapter prepares fresh native compilation while storage subscriptions
are parked, checks original storage authority and raw bytes through activation,
and renews listeners only after the complete owner group binds. Initialization
and consumed slice writes stay suppressed until new input. Existing DOM, edited
controls and independent page/example storage keys are preserved. JSON/XML and
dynamic storage types, and legacy location bindings, keep ordinary rendering
but require separate reviewed adapters before fresh-runtime resume.

Payload snapshot slot and attribute indexes preserve encounter order and own their
arrays per snapshot. Building a group appends each entry once; it must not copy
the growing group for every node. Authored names such as `constructor` and
`__proto__` are ordinary own keys, including after structured-clone transport.

Retained stylesheet replacement owns only the current generation after publication.
A preparation compares an opaque predecessor generation and the registry's source
revision; it does not own the preceding connection. The registry preserves that
connection until successful activation. Activation creates live callbacks and
scope observers outside the preparation's closure, then retires the predecessor.
The live connection must not retain its preparation object or transferred commit
array. Activation and cancellation clear transferred records; cancellation also
clears staged load bookkeeping and removed scope listeners while preserving the
committed generation. Caller-held commit groups still require their normal guards.

Retired abort signals can retain exception stacks and render snapshots; disconnecting
listeners alone does not release a reachable ownership chain. Forced Chromium
collection fixtures verify both retired generations and completed/cancelled preparation
records in worker and fallback modes. The active generation remains owned until
normal replacement, disconnection or scope disposal.

Stylesheet installation reuse follows fresh native validation, including import
reads, redirect/response policy and effective resolution context. Native CSS
identities cover the complete source/import closure, base URLs and managed scope.
Declaration owners retain matching derived style nodes; instance nodes retain one
current weakly keyed receipt for native owner/cache/context identity, occurrence,
processing host/mode and policy. No source-only or browser AST cache authorizes reuse.
Native load ownership still transfers through the current publication transaction.

Matching installed CSS and metadata stay in place, preserving CSSOM and animations.
Only necessary ordering/attachment changes run. Managed instance styles form a
suffix outside the render range; declaration sheets keep their relative native
occurrence order. Native identity changes remount instance installations even when
CSS bytes happen to match. Altered text/metadata are repaired from admitted native
output. Added/removed siblings do not reinstall unchanged sheets. Pending, failed
or cancelled candidates preserve committed styles. Disposing an owning generation
deletes its installation receipts; no predecessor history is added.

DOM reuse also follows fresh evaluation, not source text or a data-revision string
alone. Native data, attributes, payload, slices, resources, form/validation, events,
refresh and provider state continue through ordinary processing. A prior-plan handle
must match its retained artifact, policy, engine, source-map mode and exact revision.
Revision tuples use structured encoding so delimiter-containing IDs cannot alias.
DOM ownership includes produced tag, instance, template artifact, scope policy,
output target and effective ARIA reference profile. Changed ownership replaces the
scope and does not transfer focus or edited state to another owner.

Direct projection retains the current immutable child-input snapshot per element.
Complete freshly evaluated child output, including native capsule bytes and source
metadata, must match before leaving those live children untouched. The snapshots
share one input tree and are weakly owned by DOM nodes; no predecessor chain or
application cache is added. Changed output reconciles normally, including explicit
input/textarea values and editable content. Unchanged children preserve edited
contenteditable text, native control state, focus and browser selection. Patch mode
continues to use the retained output diff. An identical direct or patch publication
performs no DOM mutations in the covered form fixture.

Uncontrolled selects retain edits. Explicit `select[value]` continues to own live
selection on every admitted commit, including empty patches; this existing controlled
binding is intentional. Cancellation/stale admission cannot publish a candidate.
Consumed or cancelled prepared patches release their frame snapshots, targets,
range and option callbacks even while the consumed handle remains reachable.

Root topology changes use a single atomic `reconcileRoots` operation carrying the
fresh evaluated root list, including an explicit empty list. It cannot mix with
targeted operations, and duplicate root identities are rejected. Matching element
IDs, tags and namespaces retain live nodes; available text/comment IDs also match
at the range boundary. Restored text without in-memory IDs reconciles through the
range instead of emitting unresolvable text targets. Insertions/removals/reordering
retain compatible controls and their edited state. Empty output removes owned
roots while keeping bounds, the data island, styles and explicitly preserved nodes.
A later nonempty render creates new nodes for roots that were actually removed.

Root integrity compares live identities with the last committed root list, rather
than treating a deliberate change in the next list as corruption. Damaged roots
still trigger scope recovery; ownership changes still replace the scope. Root
transactions retain normal revision, cancellation and prepared-range guards.
Restricted live previews validate every serialized root and descendant, and
placement markers continue to require fresh native admission. The render engine
is `1.7.0`, with processing-host protocol `v28`; incompatible explicit frame engine
versions are rejected before mutation.

Same-parent moves use `moveBefore` when supported and ordinary insertion otherwise.
A synchronous weak move flag lets produced CEM descendants retain live connections,
resource consumers and stylesheet ownership through either internal reorder path.
The flag holds no state/history and clears in `finally`. Both stylesheet document
observers drain earlier genuine removals before the operation and exclude only the
exact verified move records afterward. Weak listener subscriptions retain no prior
consumer or installation history. Real removal and moves to
another parent still follow the ordinary context/disconnect/reconnect lifecycle.
Focus and input selection survive reorders; connected editable selection endpoints
are restored on the fallback path. Child evaluation, provenance updates and import
freshness continue normally; an authored child-input comparison cannot authorize
skipping them. Matching stylesheet installations and live controls survive the
fresh publication. Host snapshots and mutation invalidation retain their existing
contract, including changed and reverted attributes and native capsule handoffs.

This does not skip imports or provider/event/command evaluation. Selective native
read tracking, pure evaluation memoization and whole-process OOM recovery are separate
work; DOM identity does not authorize any of them.

## First connection and payload capture

When no direct marked island exists, the instance is in first-connection mode.
The runtime extracts author payload from exactly one of these forms:

1. ordinary instance child nodes, which may act as visible progressive-enhancement
   fallback before upgrade; or
2. one unmarked direct `<template>` with no other meaningful sibling, whose
   `content` is an explicit inert payload envelope. Per-instance CSS requires this
   form so it cannot affect the page before upgrade.

An unmarked direct `<template type="cem-ml">` or
`<template type="text/cem-ml">` is instead a CEM-ML source payload boundary.
The shorthand is accepted at author ingress and canonical serialization uses
`type="text/cem-ml"`. The template source remains in the payload part; its
validated namespace-aware AST is derived and must not be serialized as a second
competing authority.

An unmarked template mixed with meaningful siblings, or multiple direct unmarked
templates, is ambiguous and fails closed. A literal template intended as payload
must be nested inside the explicit payload envelope.

The runtime creates a new marked island and context root, moves the extracted
nodes into `cem-payload:payload`, records its explicit content type and schema,
and initializes every domain part in deterministic order. It does not relabel an
ordinary payload envelope as the island. Moving retains source-node identity
during capture; it does not make those nodes visible.

The declaration and the complete island state then produce a new rendered
projection. Source payload nodes remain inert in the island while materialized
render nodes occupy the instance's owned render range.

Parent reconciliation retains the authored explicit-template boundary after
capture. Its patch identity addresses the captured payload container; the
ordinary envelope stays inert and is never reinserted into live child output.
Updates inside the envelope reconcile the captured source nodes, leaving the
child's rendered native owners and edited values under their existing ownership.
The same rule applies to worker patches, full reconciliation and resumed HTML
payloads.

## Declaration version and SSR compatibility

A declaration may publish its template contract as an exact Semantic Version:

```html
<cem-element tag="cem-card" version="1.2.3">
    <template type="text/cem-ml">...</template>
</cem-element>
```

The declaration attribute is the authoring authority. The runtime carries the
same value as `declarationVersion` in every produced data-island snapshot, but
does not reflect it to a public `version` attribute or property on produced
instances: `version` remains available as component-owned input. Runtime code
can inspect the effective value through `declarationVersionFor(instance)`.

Declaration versions use caret-style compatibility bands. Stable `1.x`
versions are mutually compatible, `0.2.x` is compatible only within `0.2`, and
`0.0.3` is compatible only with `0.0.3`. Build metadata does not affect
compatibility. A prerelease is compatible only with the same core version and
prerelease identifiers.

The attribute is optional during migration. A missing, malformed, or
incompatible declaration version never authorizes adoption. A malformed author
declaration is rejected. When the serialized island schema and payload are
understood, a missing, malformed, or incompatible SSR declaration version causes
a diagnosed browser render from island state. It does not cause rendered output
to be recaptured as payload. An unsupported island schema remains frozen because
the browser cannot safely interpret its state.

## Reconnection, serialized HTML, and hydration

The presence of one direct marked island selects resume mode before the runtime
examines any other child. In resume mode:

- lexical namespace bindings are resolved and the complete island is admitted
  through its context and domain schemas before any work begins;
- island payload and portable semantic runtime state are reused;
- no sibling node is captured or moved into payload;
- a valid identity-matched rendered range is adopted without an initial rerender;
- provisional content, including a server- or loader-supplied `loading...` view,
  is replaced when the instance renders on load; and
- invalid, incomplete, unknown, duplicated, out-of-order, or unsafe serialized
  regions diagnose and fail closed without turning rendered output into input
  data. Existing SSR output remains frozen static content; no render, event,
  resource, or capability work starts. A missing, malformed, or incompatible
  declaration version is the explicit render-on-load exception when the island
  itself is understood.

This rule applies both to a new element parsed from SSR HTML and to an existing
in-memory instance reconnecting to a document.

## Hydration data stays in the DOM model

Browser hydration data is HTML/XML DOM data carried by the marked island. It is
available to the first client render and remains part of the island for later
updates. Browser hydration must not require switching from the HTML/XML parser to
a JSON parser or treating a sibling JSON script as a second state authority. The
context-root and hydration-data versions must agree; the current pre-1.0 contract
is `0.2.0`.

Hydration has two valid outcomes:

1. **Render on load.** The island and any hydration-provided data are used as
   render input. Provisional or loading output is replaced by the committed
   projection. This includes the forward-compatibility fallback for a missing,
   malformed, or incompatible SSR declaration version when the island is
   understood.
2. **Adopt retained output.** When island version, compatible declaration
   version, instance identity, declaration artifact, data revision, scope
   policy, source fidelity, and render boundaries agree, the existing rendered
   range is retained and client invalidation takes over without an initial
   rerender.

DOM-native hydration data and rendered output must agree on revision identity.
On mismatch, retained output is not trusted. The declaration-version fallback
renders only from an understood island; it never reconstructs payload from stale
output. Other invalid or unsafe identity conditions continue to fail closed.

Structured-clone `DataIslandSnapshot` records remain valid processing-boundary
transport for workers, Edge, or other non-DOM hosts. They are derived from the
island and are not a parallel browser hydration authority. Policy-controlled SSR
serialization may omit or redact sensitive/transient sections; omitted data is not
recoverable from rendered DOM.

[Portable committed checkpoints](committed-checkpoints.md) wrap policy-filtered
island markup with trusted all-owner publication receipts. Their host-owned
bytes remain outside replaceable WASM memory. A transport seal does not admit
island semantics; restoration still follows the resume rules above. Older
checkpoints cannot authorize rollback of pending edits or replay external actions.

The opt-in checkpoint control extension stores native-validated scalar control
state in `cem-form:form-state`, not in the dataset or reconstructed rendered
markup. Its checkpoint-only root/hydration version is
`1.0.0-checkpoint.1`, with a hydration marker identifying the extension; ordinary
live snapshots retain their existing version. Generic hydration must refuse this
extension until explicit semantic resume admits controls and resource/capability
state. Runtime checkpoint metadata additionally retains supported private form/
choice capability state and resource reimport/read/resubscribe policies in their
existing domains. Consumed writer state must be admitted and restored before
any declaration evaluation; reacquisition needs fresh host authority and cannot
replay writes or external actions. Explicit journal retirement commits only with
a successful complete checkpoint. Unsupported or private state blocks the complete runtime domain rather
than producing a partially recoverable owner.

## State authority and portable capabilities

`cem-attributes:attributes` is authoritative after serialized load. Attribute
nodes retain exact DOM strings, including the empty string. The live host is
reconciled from those nodes after admission. `cem-dataset:dataset` is a dedicated
transformation/indexing view, not an independent authority; it must exactly equal
the DOMStringMap projection of canonical `data-*` attributes or adoption fails.

The resource part contains the full portable semantic runtime state: resource
identity and kind, lifecycle status, revision/cursor, policy stamps, diagnostics,
source provenance, safe request/response metadata, and typed result AST. Live
host capability objects—abort controllers, listeners, subscriptions, streams,
DOM nodes, storage API handles, and credentials—are not AST data. A browser or
server host recreates permitted capabilities from semantic state under its
current resolver, privacy, and security policies.

## Mutation and ownership consequences

- Payload-section mutations invalidate rendering while the original nodes remain
  inert.
- Host attribute, derived dataset, slice, form, validation, event, resource, and
  hydration changes update the namespace-aware island through one revisioned
  runtime transaction before a processing snapshot is derived.
- Runtime-authored island synchronization must not recursively trigger a second
  render transaction.
- Render commits may replace only the owned render range and must preserve the
  island node.
- Nested initialized DCE instances retain their own islands and render ownership.
- Serialization must preserve the island before the rendered range so reparsing
  selects resume mode before payload discovery.

## Nested custom-element ownership

A registered custom element owns the output it creates. A CEM parent owns the
attributes and light-DOM inputs it authored for that element. This applies to
full render-plan commits and worker patch transactions:

- Update and remove authored attributes normally. Preserve attributes added by
  the custom element itself, including loading and readiness state.
- Preserve component-created child nodes. Reconcile only the parent's authored
  inputs, including their text, order, additions and removals. An unchanged
  input must not reset the component's rendered output.
- Do not rewrite render metadata or bind events/resources inside another
  component's output. A slice on the child host belongs to the parent; slices
  inside the child's own output belong to the child.
- CEM instances continue to own their durable islands. Native elements and
  unregistered tags retain ordinary CEM rendering behavior.

Components that consume/remove their authored inputs should expose subsequent
updates through attributes or their declared input API. Targeted patches cannot
update a node the receiving component has removed; existing missing-target
validation fails before committing that transaction.

Browser evidence is in `custom-element-ownership.stories.ts`; source and packaged
component playground checks also cover theme changes while the source viewer
and interactive action preview are live.


Checkpoint non-form interaction uses the shared native runtime control contract:
owner-local focus anchors, tree/menu roving state and typeahead, tabs, and
settled surfaces. Rebuild derived item caches/accessibility IDs from canonical
payload, and consume default-open initialization before any resume render.
Preparation/closing sessions and unaccounted widget/source authority remain
nonportable. Provider/placement receipt contracts describe current owner
and revision endpoints and require fresh host reacquisition; neither serialized
IDs nor a matching old token confer authority. Exact supported states and
publication fences are specified in [Committed checkpoints](committed-checkpoints.md).


Active listbox suggestion checkpoint state preserves source-order row/proof
identity, exact native publication/query state and editor/provider claims through
the shared adapters. Original XML/CEMB sources and lexical bindings pass native
import/reload validation. Committed placement source graphs and settled surface
invocation endpoints derive dependencies from actual host authority. These
records require fresh source/provider/placement admission and proof validation;
resume must not replay actions or default-open initialization. Gesture and
preparation/closing barriers, unaccounted claims/sources, retained bodies and
unknown endpoints continue refusing the complete domain. See the supported
capture limits in [Committed checkpoints](committed-checkpoints.md).


Inactive suggestion capture records original source/query intent separately from
ready row proofs. Native datalist contracts preserve provider edits and exact
claim/subscription ownership while refusing a focused, unobservable UA picker.
Failed local sources release transient claims and remain failed until ordinary
source/configuration repair; capture performs no retry. Native session capture
copies original inputs before asynchronous submission. All source interpretation
and lifecycle-envelope validation remain native. Exact supported states and
remaining fences are in [Committed checkpoints](committed-checkpoints.md).


Managed checkpoint coverage follows actual shared Worker/fallback generations and
all their native clients. An opted-out consumer or host-only job cannot disappear
from an opted-in owner's coverage. Source capsules may account for external
native sessions only through their private current witnesses. Disposal, binding
revocation and in-flight frame completion retire ownership at their actual
lifecycle boundaries. Coverage mutation fences asynchronous seals; it never
turns last-good bytes into authority to roll back edits or restart a runtime.
Native memory pressure closes ordinary admission for the affected physical
generation. Admitted work, host stylesheet I/O and cleanup retain their leases
until completion. Bounded drain deadlines, cancellation and privately issued
generation/revision witnesses fence late or copied evidence; timeout never
authorizes termination, partial coverage, replay or rollback. Native release and
bounded checkpoint control/sealing paths remain available under their existing
memory and ownership checks. Separate command heaps and unmanaged exports need
independent closed boundaries before coordinated replacement.
Physical coverage and its remaining admission/resume fences are specified in
[Committed checkpoints](committed-checkpoints.md#physical-runtime-generations-and-native-clients).


Fresh processing bindings and checkpoint source preparation
---------------------------------------------------------

`prepareCemCheckpointProcessing` is the shared native preparation building block
for trusted semantic owner adapters. It accepts an original resume context,
explicit named control DTOs extracted from that context's sealed island, complete
canonical compilation inputs, the current host event sequence and an explicit
source-policy callback. It validates controls/runtime state on the fresh heap,
imports the opaque island through the native document importer and
releases that temporary import. Approved external document bytes are reimported
under new logical handles; source trees never pass through JavaScript objects.
Preparation neither evaluates templates nor installs widgets, subscriptions or
external effects. Host/source access remains unavailable until every owner is
admitted by the original resume transaction.

Fresh worker and owned-fallback hosts use original privately registered runtime
facades, independent client/resource ledgers and generation-bound render/XPath
helpers. Copies cannot obtain a processing binding. Roots on one fresh heap share
helper caches and worker job sequencing. Existing hosts, aliases and the selected
global fallback never retarget; failure of a fresh worker refuses processing
instead of retrying on the retired fallback. Group revocation fences work before
source/engine cleanup and physical disposal.

The prepared owner's `consumeWrite` must precede journal admission and every
external write. It suppresses all writes until the host accepts an input newer
than the receipt, including implicit storage writes and newly exposed conditional
writers. Resource refreshes and renders do not advance that sequence. Previously
consumed location tokens remain consumed after new input; suppression does not
consume a new token prematurely. Invalid or regressing event sequences refuse.
Built-in plain retained CEM-ML owners use this helper. Their pre-retirement
witness covers the live canonical island, rendered integrity, control properties,
input sequence, declaration and source policy without reading a retired heap.
The existing DOM adapter compares the sealed bytes directly with its original
retained island, then serializes that same DOM with XMLSerializer
for native XML ingress, preserving namespace-qualified parts, void HTML payload
and text; it does not construct a JavaScript source AST. The original sealed
bytes remain the transaction authority. Preparation revalidates that identity, compiles complete authoring
inputs and reimports approved HTTP source bytes while inactive. Rendered markup
is an integrity witness, never a substitute for canonical data.

After complete owner admission, the trusted host calls each returned
`CemResumedElementOwner.resume()`. It binds publication to that original active
coordinator/owner context and its fresh native client, preserves event counters
and journals, then evaluates using the fresh processing host. Existing hosts and
aliases stay retired. Ordinary subsequent rendering and source refresh use the
new binding. Native control validation and sealing use a bounded generation-bound
read-only recovery path, which remains available after ordinary admission closes
without invalidating the closed-generation proof. In-flight recovery controls
remain drain/retirement barriers.

The shared plaintext-editor adapter admits native-validated text/UTF-16 selection,
focus, authored-value and consumed-selection metadata only while they still match
the retained browser owner. It preserves that editing subtree and undo history
instead of replaying source/selection over live state. Composition, deferred
values and unreported DOM text refuse capture, and later edits refuse stale
activation. Paint is discarded and regenerated on the fresh generation, outside
canonical data; delayed retired replies retain no painting authority. HTML
reparsing of sealed islands is avoided because it normalizes authored CRLF.

Standalone `form-control`, `checkable-control`, `choice-select` and
`editable-choice` owners also admit fresh processing. Native validation supplies
the named scalar control/capability DTOs. A shared browser adapter compares them
with retained controller metadata and the original control objects, form ownership
and effective disabled state. It preserves edited values, checked/mixed state,
choice previews and multiple selections, authored reset defaults, validity and
UTF-16 selection. Fresh evaluation synchronizes the existing form internals;
it does not reapply defaults, selection/value requests or dispatch input/change.
The exact revision/value/control-bound one-shot native change suppression survives
a committed provider edit; the next genuine edit clears it as usual. Accounted
shared provider relationships require the paired handoff described below.
New edits, changed defaults, control replacement and composition refuse stale
preparation or activation, including between admission and explicit owner activation.

Retained tree, tab-panel, navigation, composite-menu, popup and native-surface
owners use the same private-controller/control identity guard. Open modal surfaces
stay in the browser top layer, including edited contents and focus. Menu/popup/surface
geometry is recomputed after admission, without repeating open, command or focus
actions. Missing anchors and changed endpoint policies refuse activation.

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

Passive source consumers rerun their original trusted host input callback while
inactive. Source bytes, crossings, bindings and policies must validate natively on
the fresh heap and match the sealed source intent. Placement invocation leases
receive new private tokens from the original registry's current grants. They remain
dependent on the original grant, source bytes/policy, endpoint identity/revision
and DOM root; revocation fences the renewed route. Preparation neither reserves
IDs nor publishes relationships. The first fresh render reauthorizes again before
publication, detecting policy/alias changes after admission. Render merging preserves
generated producer IDs only through an actual current registry reservation.

Snapshot inspection, including behavior `snapshot()`, is read-only. It neither
allocates a data revision nor rewrites/drains mutations from sealed hydration data.
The first unchanged resume frame keeps the sealed canonical data revision, so
changing the physical runtime alone does not invalidate provider dependencies.
The one-use revision authority belongs to that original active owner context;
new input refuses stale first-frame authority and subsequent renders advance normally.

Paired suggestions, standalone manual listboxes and native datalists now stage
all owners before activation. Original host/control/claim-graph witnesses are
checked as a complete set, then every processing/publication route binds before
any source or provider relationship renews. A bounded browser presentation pause
retains the exact owned controls and claims through native authority loss; it
never makes an expired native row, source or invocation valid. Provider claim
writes and new openings refuse while paused. All pauses transfer from the consumed
ticket to the fresh transaction and release on completion, cancellation or failure.

Canonical local suggestions rebuild source sessions/publications on the fresh host.
The original trusted `resume(freshHost)` callback is mandatory for external sources
and standalone delegates; it must recheck current host policy and reacquire its
source/binding, rather than return an old native handle. Fresh native capsule
validation checks source bytes, policies, preparation and digests against the
sealed intent. Active and committed indices/values are revalidated against new
native row identities; no provider value is recommitted. The browser provider
retains edited controls, defaults, focus/selection and consumed-change suppression.
Standalone manual listboxes reset their native presentation. Datalists capture only
unfocused, settled editors and reacquire the owned list relationship; browser picker
selection/visibility is never restored. Adapter-driven toggle delivery from the
exact admitted surface is awaited and is not counted as new user input. Actual
edits, focus, keys and commands remain input and refuse stale recovery.

Unknown source policies, inactive sources without an admitted readonly proof and legacy
storage/location rendering still refuse complete-domain resume. Full owner/source
acceptance and declarative recovery status remain follow-up work. Studio does not
restart itself automatically.
