# @epa-wg/cem-elements

`CemCommittedCheckpointStore` holds bounded session-private checkpoint bytes
outside WASM, with receipt freshness, cancellation and atomic replacement.
Use the shared native `sealCommittedCheckpoint` export as its trusted sealer.
`CemCheckpointPublicationCoordinator.inspectRecovery()` reports immutable owner
adapter/acknowledgment metadata and pending admissions/unknown operations without
exporting source, probing owners or touching native state. It grants no recovery
authority. Explicit whole-Studio graph-preview recovery passes isolated worker and
fallback acceptance. Empty credential-control capture is admitted under the
reviewed policy below; entered credentials block recovery. Automatic recovery
and complete Studio live-view checkpoint capacity remain gated. See
[the Studio admission contract](../../docs/committed-checkpoints.md#complete-studio-admission-acceptance).

`CemCheckpointPublicationCoordinator` adds opt-in all-owner capture through
`CemElementRuntimeOptions.checkpointPublication`. Supply one trusted coordinator
for the entire host domain, a native sealer and explicit whole-island privacy
consent. `maxBytes` defaults to 1 MiB; a host can explicitly reserve 4 MiB before
pressure for a complete workspace, with unchanged per-owner and aggregate-group
bounds. `automatic: true` coalesces captures after successful DOM/CSS publication
and admission drain. Native control edits and the shared plaintext editor use a
bounded native-validated control envelope; composition, credential/file inputs
and unsupported capabilities block the whole domain. Trusted `mountCemView`
commands and shared storage/location effects record opaque outcomes before
execution, without replay. Shared form, checkbox, select and autocomplete
capabilities retain private control state. Settled HTTP, repository, storage and
live/ambient URL resources carry native-validated source/reacquisition metadata;
resume must suppress consumed writes and reacquire host permissions. Pending or
unaccounted leased/private state blocks the complete domain. Trusted hosts can explicitly
reconcile unknown outcomes and stage terminal journal retirement under matching
runtime/generation/policy authority; compaction waits for a successful all-owner
seal. After an admitted handoff, newly created owners in the same live declaration
scope also seal through its current processing host, even after every resumed
owner is replaced. Removed/disposed owners release their sealing callbacks;
initial owners retain the configured host sealer. A delayed trusted native
`select` notification for an already acknowledged complete fingerprint does not
create another checkpoint input epoch or render. Changed selections, synthetic
events and incomplete source/provider state still invalidate normally; event
listeners continue to receive the DOM event. Form providers remain `composing`
through their existing final-input completion task, keeping capture, handoff and
commits closed until the final value is published. Restart/rebind/disconnect
fences stale completion callbacks. IDs are host-issued and never reused; no operation is replayed.
Checkpoint-only form envelopes require explicit semantic resume and are refused
by ordinary hydration. Ordinary rendering continues.
This is host-session capture, not automatic WASM replacement or resume. See
[the checkpoint contract](../../docs/committed-checkpoints.md).

Stable tree/tab/menu navigation and settled popup/dialog state now have
native checkpoint contracts, including owner-local focus anchors. Trusted hosts
can register inert provider/placement dependency intent with
`coordinator.relationship(descriptor, current, authority)`; native sealing requires
complete current endpoint receipts and fresh reacquisition. Shared active listbox
suggestions/providers, committed placements and settled open popup/dialog state
now capture bounded native-validated provenance and interaction state. Selection
proof, source crossings and invocation endpoints are reacquisition intents;
unknown leases, pending work, gestures, retained bodies and unaccounted source
inputs remain blocking. These contracts do not enable runtime replacement or
semantic resume; ordinary hydration still rejects checkpoint extensions.

Settled idle/failed/unavailable listbox sources preserve original XML and query
intent through a separate inactive checkpoint contract, without stale row proofs.
Native datalist capture accounts for the exact provider/placement subscriptions
and preserves original source, input edits and attached/unavailable claim intent.
Its editor must be unfocused because browser picker state cannot be observed;
presentation resets and relationships are freshly admitted on future resume.
Failed local native sources can be captured without automatically retrying them.
Portable opt-in native-session inputs are copied before asynchronous submission so caller
mutations cannot change execution or captured source provenance.
CEMB source checkpoints retain explicit depth/work and unresolved-reference
policies, including explicit defaults, with native validation and fresh admission
on resume. XML import policy overrides remain unsupported.
Separate `CemElementRuntime` instances sharing one trusted checkpoint coordinator
can capture provider/delegate, placement and geometry/focus endpoints together.
Exact registered hosts and private renderer identities establish endpoint ownership;
foreign domains, unregistered owners and stale publications block capture. This
explicit domain membership does not discover physical WASM runtime generations
or enable replacement/resume.


`CemNativeRecoveryScope` explicitly enrolls rendering runtimes and child live
previews across physical heaps. Pass it through `CemElementRuntime` host options;
preview children inherit worker/checkpoint policy and retain copied source bytes
for native reimport. Scope preparation refuses incomplete, busy, unattested overlapping or
unsupported members and guards the complete native allocation. Enrollment does
not enable automatic recovery or replace semantic owner tickets. See
[explicit runtime enrollment](../../docs/committed-checkpoints.md#explicit-runtime-recovery-enrollment).

Loaded live/artifact media controllers retain their original frames and resource
maps during explicit handoff. Use `CemCheckpointPublicationCoordinator.combine()`
for original coordinators sharing one physical heap; complete native owner/source
coverage remains mandatory. Mixed command bindings supply original
`prepareResume()` tickets as the fourth argument to `resumeCemCheckpointOwners`.
All command/rendering/preview routes bind before provider renewal. Static
`document`, `text`, `media` and `message` modes use a separate native-validated
`retain-presentation` policy. Their original DOM remains in place, preserving
scroll, text selection, edited controls and browser media state. Static document
frames remain opaque (`sandbox=""`): recovery neither inspects nor reconstructs
the document. A trusted load revision and unchanged frame/presentation witness
are required; pending initial loads, later load revisions, changed policies/topology, unknown modes
and presentations exceeding 4096 nodes refuse. Live/artifact controller grants
are unchanged. Complete whole-Studio acceptance remains a separate prerequisite.

Restricted live-preview HTTP requests can checkpoint original resource-map
sources with explicit `sourcePolicy: "retained-resource-map"`. Native validation
admits credential-free absolute HTTP(S) and `studio://` map addresses for this
policy only; ordinary HTTP checkpoints keep their HTTP(S) restriction. Original
native document receipts, bounded source bytes/digests and complete native census
remain mandatory. Inactive preparation reimports those bytes and rechecks the
original map resolver and policy without fetching or replaying a request. Missing
resources, changed inputs/policies and incomplete source receipts still refuse.
These scalar policies carry intent and never grant controller or source authority.


Explicit shared owner adapters can now use `publication.prepareResume()` before
retirement and `resumeCemCheckpointOwners(admission, tickets)` after original fresh
allocation. Exact private checkpoint bytes, complete physical/owner coverage,
current host/privacy witnesses and terminal journals guard one atomic owner gate.
Failure cancels and discards the complete fresh group. Fresh fallback `release`
permits only native cleanup after fencing. This transaction leaves DOM untouched.
Built-in plain retained CEM-ML, plaintext-editor and standalone form/checkable/choice owners prepare fresh processing contexts and
approved HTTP sources. After complete admission, call each returned
`CemResumedElementOwner.resume()` to renew its publication endpoint and evaluate
on the fresh generation, preserving retained DOM, edits and installed styles.
Form adapters retain the original controls, reset defaults, validity, focus and
selection, including consumed selection/value requests and one-shot provider-commit
change suppression. They reject newer edits and do not manufacture input/change
events. Provider relationships require separately renewed authority.
Action-command/action-control, disclosure-panel and visible native-list owners
retain original controllers, controls/options, form ownership, disclosure IDs and
scroll. Routed targets, descriptors and submenus require complete owned endpoints;
changed routes/controllers/options and unknown native picker/interest state refuse.
No click, submit, command or navigation is replayed. New explicit actions work
through the fresh processing/publication binding.
Trees, tabs, navigation, composite menus, popups and settled native surfaces also
retain their browser state; geometry is rechecked without replaying open/focus
actions. Parent reconciliation preserves unchanged authored child inputs, including
live open state; changed or removed authored inputs still apply. Passive source/placement consumers call the original host input callback,
validate its opaque source bytes/policy on the fresh heap, and acquire new invocation
leases from current registry grants. Copies, changed sources and revoked endpoints
cannot authorize publication. The first unchanged frame retains its sealed data
revision. Paired suggestions, manual listboxes and native datalists bind every
participating processing/publication owner before renewing source publications and
provider relationship claims. Native row/commit proofs are rebuilt and checked
against sealed source capsules; original DOM, edits, defaults and consumed changes
stay with the browser provider. Local canonical sources renew through shared
adapters. External suggestions and standalone delegates require their original
trusted lifecycle's `resume(freshHost)` policy; absent or withdrawn policy refuses.
Standalone listbox presentation resets, and native datalists require an unfocused
editor and reset picker presentation. Inactive sources without an admitted readonly
proof and unsupported storage/location rendering still refuse. Plain text storage
owners now compile on the fresh processing host, retain original storage authority
and raw values, replace parked live subscriptions and suppress consumed writes
until new input. Page and example keys remain independent. JSON/XML/dynamic-type
storage and legacy location adapters remain gated. External popup triggers and
anchor/boundary/focus roles require complete owned endpoints, original controller
and control identities, current layout evidence and fresh source/placement grants
when native references are used. Viewport geometry recomputes without replaying
open/focus actions; changed policies, replaced targets and stale layout refuse.
The complete production theme menu uses these shared contracts.
Complete Studio preview/command-runtime coverage
and successful whole-workspace acceptance are still required before automatic
recovery. See [explicit owner resume transactions](../../docs/committed-checkpoints.md#explicit-owner-resume-transactions).


Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

`@epa-wg/cem-elements` provides the `<cem-element>` browser substrate for declarative light-DOM custom elements. It is
the Phase 3.1 runtime gate before the project moves into Edge/SSR support and later `@epa-wg/custom-element` adoption.

Hosts can import `readCommandOutput` from `@epa-wg/cem-elements/command-result`
without registering custom elements. It reads retained artifacts or committed
publications with explicit byte/chunk limits and verifies length and SHA-256.
Failed commands that publish nothing return `available: false`; published empty
files return `available: true`. Missing successful outputs and corrupt publications
remain errors. Declarative consumers can retain diagnostics and disable output
transfers when `available` is false.

`readCommandOutputs` admits the complete output manifest with an explicit
`maxArtifacts` count and aggregate `maxBytes` budget. It verifies every native
publication before returning the set, preserving URI, retained handle and
source-map identity. Truncated/ambiguous manifests and invalid digests fail as a
set. Retained handles take precedence over committed transport copies; a failed
command with no publications returns an empty set. Use
`fallbackToFirstOutput: false` with the single-output reader when a specific URI
must exist instead of using its compatibility primary-output fallback.

Native CEM-ML node/reference values in relationship attributes use the shared
[reference-to-ID consumer](../../docs/cem-element-reference-ids-design.md): preserve
an explicit target ID or generate one from persisted instance identity and output
placement. A target needs one local placement, or one foreign placement admitted
by the [host placement coordinator](../../docs/cem-element-granted-placements-design.md).
`command-target`, `interaction`, `trigger-for`, `parent-item`, `focus-target`,
`return-focus`, `anchor` and `boundary` retain typed endpoint markers for their
shared capabilities; literal local names remain scoped. Popup/menu providers
check entry focus inside the surface, recheck return eligibility on restoring
close reasons, and fit independent anchors/boundaries inside the visual viewport.
Missing geometry rejects opening; established surfaces close on loss unless
`anchor-lost="freeze"` preserves the previous geometry.

For foreign authored reference chains, embedding hosts supply
`elementReferenceInputs(instance, snapshot, signal)` in `CemElementRuntimeOptions`.
The exported `CemElementReferenceInputs` type separates source bundles/selections,
context readiness, policy overrides and directed grants from authored documents
and snapshot data. Sources explicitly use experimental debug CEMB reload bundles
with original lexical capture. Passive native selections populate declared slices;
ready sources receive that invocation's completed frame. The runtime obtains fresh
inputs on each render and prevents superseded/disposed work from publishing.
After replacing readiness/context/grants, hosts call
`runtime.refreshElementReferences(instance)` to invalidate pending publication and
request a fresh invocation without changing authored attributes.
The signal aborts on replacement/disconnect; ignored late callback results cannot
hold the publication queue or supply authority to a later render.
Incomplete updates preserve prior DOM relationships. Contexts and grants are not
saved in hydration snapshots. Worker protocol v17, fallback and retained Node SSR
use the same native adapter. Stable production source transport remains a
separate boundary; cross-instance placement requires an additional host grant.

`CemElementRuntimeOptions.placementCoordinator` opts into host-controlled
cross-producer relationships. A `CemElementPlacementCoordinator(document)` (or
one permitted shadow root) registers committed elements with original source
capsules, native selectors and current producer revisions. Its `grant` explicitly
names the requesting instance and relationship properties. Matching DOM IDs
create no grant. IDs are reserved by the target producer at publication.

For coordinated producers, `transaction` captures all participant hosts/revisions;
`stage` prepares their native terminal plans without foreign relationships;
`enlist` admits a parent stage before preparing its detached child host.
`registerPrepared` permits targets only inside that private transaction, and
`publishGroup` commits every forest before activating relationships. Changed
revisions, grants, roots or IDs prevent publication and restore prior forests.
Revocation/disconnect removes affected relationships immediately. A reconnect
needs fresh grants, even when its durable instance identity stays the same.

`CemSsrPlacementCoordinator` provides the corresponding retained export-plan
transaction without browser globals. Its `resumeHints` carries paths, reserved
IDs, revisions, scope stamps and the exact selected ARIA profile; it omits
admission tokens, grants and source captures. Hydration clears foreign routes
before behavior activation and obtains fresh live host admission. Serialized
hints cannot recreate authority. Native/worker metadata stays on the named control
boundary and never transports DOM elements or AST records.

Reference export defaults to pinned `wai-aria-1.2-rec-20230606`.
`CemElementRuntimeOptions.ariaReferenceProfile` opts into experimental
`wai-aria-1.3-wd-20260604`, allowing nonempty ordered native target sequences for
`aria-details` and `aria-errormessage`. The exported constants
`DEFAULT_ARIA_REFERENCE_PROFILE` and `EXPERIMENTAL_ARIA_REFERENCE_PROFILE` name
these exact identifiers. Native export, worker/fallback processing, revision and
cache identity, SSR plans and resume metadata retain the selection. SSR hosts
use `new CemSsrPlacementCoordinator({ ariaReferenceProfile })`; hydration under
a different profile rerenders. Unknown identifiers fail preparation. The
[profile matrix](../../docs/cem-element-aria-reference-profile.md) records automated
DOM evidence and the separate, unverified assistive-technology mappings.

The shared `native-surface` capability connects one direct `part="surface"` native
dialog or tooltip popover to the shared preparation/open/close lifecycle. It supports persistent
nonmodal dialogs, native modal entry, popover visibility, cancelable close
requests, context-key policies, typed focus/return/anchor/boundary relationships,
and invocation-captured pointer/selection geometry. Centered tasks need no anchor.
Existing owners can use `connectCemNativeSurface(owner, { host })`; its `open`
accepts a `CemSurfaceInvocation`, optionally captured with
`captureCemSurfaceInvocation(source, event, { selectionOwner })`. The context
stays a retained `NativeCemValue`; geometry never becomes a focus destination.
Tooltips preserve stable description tokens and input focus. Owners survive
rerender, while disconnect cleans sessions and resources. A dialog opened without
an explicit invoker retains the focused entry control as its return destination.
Removing a conditional dialog returns there only if the dialog still owned focus
and the destination remains available; `return-focus="none"` suppresses shared
restoration. Explicit ARIA names take precedence; a direct `part="heading"` supplies
a stable generated heading relationship when no explicit name is present. A direct `template[slot="body"]` in the native owner defaults
to retained materialization; `materialize="eager"` mounts at connection and
`materialize="dispose"` removes the body after close/focus/exit cleanup. Body
projection preserves native values and stable identity while separately owned
draft slices survive disposal. The declaration capability waits for nested
produced elements to settle; shared host adapters can supply `prepareBody` with
an abort signal. Pending preparation keeps launcher focus and busy state; late
completion cannot reopen a canceled request. `open()` reports admission, while
`cem-open` reports actual visibility. Default-open does not replay on reconnect
or accepted serialized resume. Shared menu-to-task relay captures validated nested
or independent menu ancestry before dismissal, keeps menus available through
preparation, and returns eligible task exits to the stable root launcher.
Hosts can request generated buttons with `trigger`/`close-label`, replace them
with `slot="trigger"`/`slot="close"` controls or providers, or use typed
`trigger-for` for an external launcher. A slotted trigger takes precedence over
the label; external and local triggers conflict. Generated launchers support
`trigger-aria-label` and presence-based `trigger-disabled`. Native button routes
are adopted once, actual visibility drives expanded state, and close controls
use the cancelable lifecycle. Generated controls and owned attributes survive
rerender and clean up on disconnect without overwriting unrelated attributes.
Canonical `cem-dialog`, `cem-dialog-shell` and `cem-tooltip` declarations now
consume this capability. Tooltips accept slotted native/editor triggers or shared
control providers, scoped/typed external launchers and native interest. Focus and
hover are independent; Escape cancels delayed presentation, touch remains native,
and disabling suppresses presentation while retaining the description. Native-surface
behavior identity is v6; popup/composite identities remain v4.
For the shared lifecycle,
see the [implemented adapter contract](../../docs/cem-interaction-design.md#implemented-shared-native-surface-adapter).

Shared geometry now gives each actual panel an exclusive transient lease. A
host can fit several surfaces without replacing another surface's observer.
Queued resize, mutation and viewport updates are coalesced and fenced on reset
or release. Cleanup restores only current geometry claims, preserving newer
authored style values, priorities and placement attributes; shorthand claims
retain their longhands. Runtime copies share ownership within the realm.
Native dialog/tooltip registrations retain their semantic kind and require
fresh registration after a kind change. Geometry supplies no source/placement
authority, visibility or focus policy. Suggestions' manual-listbox delegate and
native publication/bindings are implemented; public composition remains pending in
[the active checklist](../../docs/todo.md#autocomplete-and-suggestions-design-for-cem-inputs).

Declarations may publish an exact Semantic Version for SSR adoption:

```html
<cem-element tag="cem-card" version="1.2.3">
    <template type="text/cem-ml">...</template>
</cem-element>
```

The declaration owns the version. Each SSR data island records it as
`declarationVersion`; produced instances do not receive a public `version`
attribute or property. `runtime.declarationVersionFor(instance)` provides
collision-safe introspection. Hydration adopts caret-compatible versions. A
missing, malformed, or incompatible declaration version rerenders from an
understood island, while an unsupported island schema still fails closed.

An identical declaration can remount after its previous same-scope owners have
disconnected. It reuses the browser constructor, processing scope and managed
stylesheet nodes. Styles move to another live compatible declaration when their
owner disconnects, and detach when the last owner leaves. Concurrent new
same-scope duplicates and incompatible replacements still fail; remounts cannot
revive disposed processing scopes. See the
[registration lifecycle](../../docs/cem-element-design.md).

Native declaration and inert-payload CSS is enabled by default.
`new CemElementRuntime({ retainedStylesheets: false })` selects the legacy
compatibility path. The default browser reader checks HTTP status,
honors cancellation and buffers at most the native 16 MiB limit plus one byte
for native oversize diagnostics. It passes the actual final URL and content type
to native admission.

Hosts can override transport with `retainedStylesheets: { read }`.
`read(request, signal)` receives the native
resolved URL, request ID, content-type hint and integrity metadata. It returns
`{ bytes: ArrayBuffer, finalUrl, contentType }`; native import validates the
response and resolves nested imports and resource URLs. The reader should honor
the abort signal and reject failed responses.

Render requests are batched per instance into the next task. Event listeners
capture slice values and event metadata immediately, while `currentTarget` is
available. Bubbling listeners, nested events, microtask follow-ups, behavior
updates and observed host changes then share one render of the latest state.
Separate later tasks can request another render; initialization or form
reconciliation can also require a follow-up pass. `whenRenderSettled(instance)`
waits for queued and in-flight rendering. New requests invalidate older in-flight
results immediately. Disconnected instances skip queued work; reconnect renders
current state.

In this mode, `whenDeclarationSettled` waits for retained CEM-ML/DOM source
registration. `whenRenderSettled` also waits for the instance's private, shared and payload
stylesheet loads, including during hydration. Shared sources apply without an
instance of their own declaration. Disconnect, context changes and scope disposal
release consumer generations. The internal `data-cem-css-context` marker is
recomputed and excluded from snapshots. Stylesheet diagnostics expose their
`sourceUri`, `stylesheetUrl` and byte range. XSLT result styles retain their
existing branch-local behavior.

Payload styles are direct host children outside the render range. Native owners
use the persisted instance identity and consuming module context; emitted CSS is
installed without parsing it again. Source edits and context changes replace the
installation. Updates prepare declaration CSS, payload CSS and the DOM patch
inside one per-host queue. Failed loads preserve the live generation; invalid
patches trigger full-render recovery. Runtime state is adopted before retiring
old styles, and resource work settles outside the queue. Hydration waits for
imports while preserving compatible rendered DOM and animation names.

When a stylesheet source revision invalidates an in-flight preparation, a still-current
owner cancels both CSS candidates and waits for a stable revision before preparing
them again, preserving published DOM and edited controls. Real CSS errors and
supersession, disconnect or disposal retain their existing failure and cancellation guards.

Prepared registry replacements compare predecessor generation tokens without owning
old connections. Activation rebinds scope observers and live callbacks outside the
preparation closure, then clears transferred commit records. Cancellation drops
candidate records and load bookkeeping while preserving committed styles. A live
connection therefore does not retain completed preparations or a predecessor chain.
Forced Chromium collection fixtures cover both worker and fallback paths, including
ancestor scope disposal.

Freshly validated native output now retains unchanged stylesheet installations.
Instance receipts compare native owner/cache/context identity, occurrence and live
host/mode/policy; matching CSS and markers avoid DOM writes or reinsertion.
Declaration sheets also retain CSSOM when siblings are added/removed. Ordering and
altered text/metadata are repaired with necessary DOM operations only. Changed native
identity remounts instance CSS even when bytes match. Imports still load and validate
on every attempt; pending/failed/cancelled loads preserve committed styles. This
retains CSSOM and running animations without skipping rendering or adding an app
cache. See [the lifecycle contract](../../docs/cem-element-lifecycle-principle.md).

Fresh render output also reuses unchanged DOM. Ownership checks include the
instance, template, policy, output target and ARIA profile; retained prior-plan
handles must match their stored revision and engine metadata. Revision tuples are
encoded without delimiter ambiguity. Direct projections compare complete immutable
child inputs, including native bytes, before retaining live children. This preserves
contenteditable edits, native form state, focus and selection while changed bindings
still reconcile. Identical direct/patch publications avoid DOM mutations. Explicit
`select[value]` remains controlled on every commit; ordinary selects retain edits.
Prepared patch handles drop snapshots, DOM targets and callbacks on consumption or
cancellation. Fresh native evaluation and resource loading still run; no
source-only render cache or Studio JavaScript is introduced.

Root insertion/removal/reordering now reconciles the owned range atomically,
including explicit empty output. Matching root identities retain DOM and edits;
removed roots are recreated when they return. Bounds, data islands, styles and
explicitly preserved runtime nodes survive an empty render. Committed root receipts
still detect corruption and ownership changes still replace the scope. Available
text IDs preserve root text nodes; restored text without IDs reconciles the range.
Same-parent produced-element moves preserve their live resources through native and
fallback DOM moves, including focus/editable selection. Child evaluation and imports
remain fresh; matching CSSOM/control identity survives those publications. The root operation is isolated from targeted operations and
covered by restricted-preview/placement admission. Render engine `1.7.0` and worker
protocol `v24` fence this patch vocabulary; incompatible frame versions reject
before mutation. Ordinary data/provider/resource evaluation remains unchanged.

The asynchronous `executeNativeSsrInitialRenderFixture` evidence host now loads
native payload CSS and returns `instanceStylesheetHtml` for direct host children
outside the render range. Worker/fallback hydration preserves its DOM and animation
identity. The synchronous host and legacy streamed update host still reject
retained-CSS policies with `cem.edge_ssr.retained_css_unavailable`.

`serializeDeclarationStylesheets` formats native declaration/shared output for
placement under its declaration owner and returns the consuming context marker.
Browser ownership reuses matching server style nodes after native loading,
deduplicates them, and releases them with their last consumer. See the
[SSR placement contract](../../docs/scoped-css-module-maps.md#declaration-stylesheet-markup-for-ssr).
The async initial host accepts caller-selected `declarations` batches and returns
their `declarationStylesheets` sidecars after all imports and serialization finish.
Adapters supply extracted render source and place each sidecar under its owner.

`executeNativeEdgeRenderUpdateFixture` preserves unchanged styles or loads a
complete native replacement before committing state and exposing update frames.
Changed inputs require native bindings and a byte reader. Protocol v14 carries
the emitted batch and its retained state; failed loads, cancellation and stale
ETags cannot publish partial stylesheet state. Edge state schema 1.1.0 retains
`stylesheetState` through `currentStylesheets`.

`publishEdgeCssDomUpdate` is the browser publication entry point for an Edge
adapter that owns a host's render range and declaration owners. Buffer progress
frames until terminal success, then supply the response, current state reader,
previous ETag, lifecycle abort signal and all declaration owners. It verifies
the content address and revision and publishes emitted CSS without parsing it.
Its `adopt` callback installs the next state before old-generation cleanup.
On `recovery-required`, fetch an authoritative full-render transaction and call
with `recovery: true`; queued ordinary updates remain blocked until it succeeds.
Abort the lifecycle signal on teardown. Do not independently mutate a host with
both an Edge adapter and the local runtime: the adapter must own state adoption.
See the [native update contract](../../docs/scoped-css-module-maps.md#streamed-native-css-updates).

Run the default and compatibility browser lanes:

```sh
yarn nx run cem-elements:test
yarn nx run cem-elements:test:legacy-css
```

`test:retained-css` remains an alias configuration for explicit native runs.
The legacy lane changes the shared Storybook preview; independent runtime
fixtures select their own options. Two serialization fixtures explicitly pin
the legacy import-suppression and UID-keyframe output contracts.

CEM-ML attribute and slice defaults in a root `{module | ...}` prelude initialize
the instance just like declarations in an unwrapped template. Slice defaults
populate both their named bindings and `datadom.slices`. Boolean defaults stay
boolean, and host-provided values take precedence, including empty strings. Defaults inside body
content or uncalled named templates do not initialize the outer instance.

DCE templates can use `$instanceID` to read the current instance's identity.
New instances receive a `cem-instance-` UUID; updates and reconnects keep it,
and hydration reuses the serialized snapshot's `instanceId`. Treat it as opaque
and reserve the `instanceID` binding name for the runtime. Authors importing or
cloning serialized instances must preserve document-wide uniqueness themselves;
restoring the same snapshot twice does not allocate a fresh identity.

Native template references in relationship attributes, such as
`@commandfor={#target}` or `@command-target={#provider}`, can designate an element
inserted into the same render forest. Shared native projection preserves its ID
or generates one from the persisted instance identity and produced placement.
Forward targets work; repeated placements, missing targets and incomplete chains
return diagnostics without publishing a replacement plan. The action capability
accepts projected typed `command-target`/`interaction` endpoints alongside its
existing scoped `@local-name` conveniences. Plain native ID strings stay unchanged.
See the [consumer contract](../../docs/cem-element-reference-ids-design.md).

`CemQlRenderOptions.elementReferenceInstanceId` explicitly enables this consumer
for low-level retained CEM-ML/XSLT render calls. The shared cem-element processing
path supplies `identity.instanceId` automatically. Generic CEM-QL rendering keeps
its existing projection. Authored foreign-source chains require an explicit
native lifecycle host; the browser convenience mode does not manufacture their
contexts or crossing grants. Foreign placements additionally require the host
placement channel described below.

The `instanceID` binding creates no DOM attribute or CSS custom property. When needed, author
a complete URL-valued property and matching target explicitly:

````cem-ml
{style |```
    [part~="preview"] { filter: var(--sample-filter); }
    svg { position: absolute; width: 0; height: 0; }
```}
{div @part=preview
     @style='--sample-filter: url("#{$instanceID}-filter")' | Preview}
{cem:variable @name=svgNS @select='"http://www.w3.org/2000/svg"' }
{cem:element @name=svg @namespace="{$svgNS}" |
    {cem:attribute @name=aria-hidden @value=true}
    {cem:element @name=filter @namespace="{$svgNS}" |
        {cem:attribute @name=id @value="{$instanceID}-filter"}
        {cem:element @name=feGaussianBlur @namespace="{$svgNS}" |
            {cem:attribute @name=stdDeviation @value="2"}
}   }   }
````

The example's dynamic inline value carries instance data; the presentation rule
stays in the shared static stylesheet. CSS `var()` consumes the complete
`url(...)` value, not an ID to concatenate inside `url()`. Suffix IDs further
inside loops so each rendered target remains unique. See the
[scoped CSS demo](demo/scoped-css.html) for two instances with different filters.
Automatic contextual `url(#id)` substitution is deferred to the
[wishlist](../../docs/wishlist.md#cem-elements-runtime).

For focused standalone and source-loaded verification, run
`CEM_DEMO_PATH=/packages/cem-elements/demo/scoped-css.html yarn nx run cem-elements:verify-demo-fixtures`.
Omit `CEM_DEMO_PATH` to verify the complete gallery.

External declaration loading through `src="#id"`, `src="url"`, and `src="url#id"`, plus `<http-request url="...">`
resource loading, uses the [CEM-ML resource lifecycle](../../docs/cem-ml-resource-lifecycle.md) as the base contract and
the [`cem-element` external resource loading contract](../../docs/cem-element-src-loading-contract.md) as the CEM Elements
binding for resource role, acquisition policy, metadata, and expected content-type context.

Ordinary HTML navigation keeps the host document's base by default. Set
`link-base="source"` on a declaration to resolve its rendered `a[href]` and
`area[href]` against the declaration's final source URL; `link-base="document"`
explicitly selects the default. Dynamic href bindings use the same policy.
Empty and fragment-only links still target the current document. Inert templates,
consumer payload, nested declarations and other URL attributes keep their own
behavior. This policy does not fetch targets or use import maps. See the
[navigation contract](../../docs/cem-element-src-loading-contract.md#ordinary-navigation-links).

HTML projection preserves the contents of nested HTML `<template>` elements
for their eventual consumer. Outer bindings and payload slots do not evaluate
text, attributes or slots inside those inert contents. This keeps embedded
CEM-ML such as `{td | ${$product.price}}` intact when a whole demo or declaration
document is loaded through `src`. The template element's own attributes remain
part of the outer projection.

Changing a rendered input's `value` also refreshes its live value after editing,
including returning to the initial value or clearing/removing the binding.
Unchanged bindings preserve pending edits during unrelated renders. Updates keep
the input node, focus and selection (clamped to the new length); native input
sanitization and form reset defaults still apply. File inputs and controls whose
values already reflect their attributes retain native behavior.

Parent reconciliation updates a registered child's inert authored payload while
preserving that child's generated output. Full plans and child-list patches use
the same payload boundary as targeted attribute patches. Consecutive changes to
projected input labels, names and defaults retain native input identity; unchanged
value bindings preserve edits. The calendar input's `AuthoredPayloadUpdates`
story covers this boundary in worker and fallback execution.

Slice-event bindings authored in a parent's projected content keep that parent
as their binding owner through named and default slots. A projected native
control or custom-element action updates the parent's slices; controls authored
inside the receiving component keep its own state. Child and parent rerenders
preserve that distinction. This source ownership is runtime metadata, not a
public attribute that payload markup can use to claim another owner. Widget and
form ownership still follows the receiving component's existing boundaries.

Declarations may opt into `capability="checkable-control"` for one owned native
`input[type="checkbox"][part~="control"]`. The host's `indeterminate` presence
sets the native mixed-state property, including empty values and the text
`false`. Activation clears that property natively. Unchanged authored presence
does not reapply it on unrelated renders or reconnects; removing and readding
the flag reapplies it. Nested custom-element controls are excluded, and multiple
owners or a switch role do not receive mixed state. Fragment loaders inherit
the source's capability metadata. Native checked/defaultChecked, value, forms, radio grouping,
reset, validity, keys and events remain browser-owned; this capability does not
add a form-associated host or synthetic change events.

Declarations may opt into `capability="calendar-input"` for a bounded single-date
calendar around exactly one direct authored `input[slot="input"][type="text"]`
and an optional `button[slot="toggle"][type="button"]`. A generated
`[part~=root]` projects both and contains a direct native `dialog[part~=dialog]`.
Day buttons use `role="gridcell"`, `data-date`, and `data-active`; navigation and
confirmation buttons use `data-calendar-action="previous|next|cancel|apply"`.
The input remains the exact native value, form, validity, reset and event owner.
This capability does not add a form-associated host or a host value API.

The shared model validates Gregorian `YYYY-MM-DD` in years 0001–9999, host
`min`/`max`, and presence-only `disabled`, `required` and `invalid`. Localized
labels retain Gregorian UTC date parts; malformed locale tags use the runtime
fallback. Render slices expose IDs, heading, weekdays, bounds, action state and
six weeks of days. Closed calendars leave day rows unmaterialized; opening
reads the current native value and creates the 42 cells, including inert blanks
beyond the year limits. Fieldset disabling and readonly input suppress opening;
composition and modified editing keys stay with the native input.

The native modal dialog registers with shared surface ownership. A silent
`expanded` property controls it; Escape, Cancel, backdrop, reset and reconnect
discard drafts. Apply closes and restores input focus before one native
`input`/`change` pair. No custom lifecycle events are emitted. Generation checks
prevent deferred resets or commits from crossing disconnection. Fragment loaders
inherit the source's `calendar-input` capability. The [datepicker contract](../cem-components/docs/datepicker-contract.md)
and [shared worker/fallback plays](src/lib/calendar-input.stories.ts) specify the
owner, keyboard, focus, event and lifecycle boundaries.

Declarations may opt into `capability="editable-choice"` for a single editable
combobox whose submitted option identity differs from its visible label. The
host owns form association; its unnamed `input[part~=control][role=combobox]`
owns native editing and focus. A stable root contains the control and conditional
`[part~=popup][role=listbox]`; owned rows use `part="option"` and
`data-option-index`. Nested custom-element controls and rows are excluded.

The capability shares canonical/native option normalization with `choice-select`.
It consumes the current authored payload without filtering, preserves committed
value/display text when suggestions disappear, and retains active option identity
when suggestions reorder. The runtime observes the island's `cem-payload:payload`
section; sibling nodes outside that section are not new payload. Render slices
expose groups, label/listbox/active IDs, expanded state and display text. Input,
change, reset, restore, required-selection close and silent setters follow the
[editable-choice contract](../cem-components/docs/autocomplete-contract.md).
Composition and modified editing keys stay with the input. Option commits update
native text, model, validity and form submission before the single input/change
pair. Scroll reveal is confined to the popup.

This ownership differs from the `suggestions` attachment: editable choice owns
its editor, submitted identity and committed label, accepts authoritative source
replacement and does not connect a separate editor provider or query/filter the
native suggestions model. Shared [runtime plays](src/lib/editable-choice.stories.ts)
cover worker/fallback lifecycle, native forms and nested ownership.

## Source-owned capabilities

Declare reusable behavior on the template with
`{template @capability=form-control @type=text/cem-ml | ...}` in CEM-ML, or
`<template capability="form-control" type="text/cem-ml">` in HTML. A capability
on the enclosing `cem-element` declaration remains supported. The runtime reads
this metadata before registering the produced class; it is not an instance flag.

Imports using `src="./controls.xhtml#account-input"` inherit the selected
template's capability, falling back to its immediate owning declaration. The
importer's own attribute takes precedence, including an explicitly empty value
that suppresses inheritance. Unknown capabilities and conflicts with
host-supplied behavior retain their existing diagnostics. See the
[loading contract](../../docs/cem-element-src-loading-contract.md#source-owned-capabilities)
and the runnable [form demo](demo/form.html).

**Component caveat:** preserve intrinsic capability metadata when copying a
template. Omitting it can leave visible markup without the component's form,
keyboard or popup behavior. Use the component's documented capability rather
than assigning `form-control` to every input component. Explicit overrides can
disable the source's behavior; the tag name does not supply a default.

## Plaintext document editors

`capability="plaintext-editor"` admits one `part="control"` editable element
inside a direct `part="root"`. `cem-source-editor` supplies the production
CEM-ML declaration and styles. The capability owns text values, native
composition/caret retention, explicit source selection and disposable native syntax paint. It exposes a string
`.value` to declarative input bindings and host commands, reading exact text nodes and native block/BR
line breaks to retain authored CRLF and Enter/paste source. Programmatic source is assigned
as text, never HTML. Readonly and disabled declarations retain native semantics.

A behavior's `preserveRenderedChildren` hook protects that browser-owned editing
subtree while authored attributes remain reconciled. Unrelated rendering does
not replace it or clear native undo. Accepted external text replacements are
applied by the capability, deferred during composition. The canonical value
slice remains serializable; DOM nodes, browser Selection and undo state are
transient. This capability does not add a form-associated host or language
services. See the [source editor contract](../cem-components/docs/source-editor-contract.md).

Explicit checkpoint resume now admits this shared editor on fresh workers and
owned fallback heaps. Native-validated control text and UTF-16 selection must
match the retained browser owner, including focus, readonly/disabled state and
consumed selection requests. Composition, deferred values and unreported DOM
text refuse capture; later edits refuse stale preparation/activation. Resume
keeps the same editing subtree and undo history, emits no input and renews
publication/write suppression before rendering. It clears old paint requests
and regenerates ranges on the fresh generation. Paint ranges and native job
identities never enter the checkpoint; late retired replies cannot repaint.
This is an explicit shared owner adapter, not automatic Studio recovery.

With `content-type="text/html|text/cem-ml|text/css"`, the capability reuses the
playground's lossless native highlighter through bounded `highlight` processing
jobs (protocol v29). The named `sourceHighlightPaintV1` presentation DTO contains
only schema/status and UTF-16 role ranges, never a source AST or token stream.
CSS Custom Highlights paint existing text without rewriting the editable DOM.
The original native client/generation controls admission, cancellation and paint
revocation; pressure, disposal and superseded text/file identities clear paint.
Source is limited to 256 KiB UTF-8 and 16,384 ranges; jobs debounce for 80 ms and
time out after five seconds. Missing browser support or native/language support
leaves plain editable text. `highlight="off"` opts out, `source-id` fences file
switches, and the default `content-type` remains `text/plain`. Formatting remains
an explicit command. No Studio-specific editing behavior is required.

## Native string form controls

Declarations may opt into `capability="form-control"` for one string-valued
native input or textarea marked `part="control"`. The produced host owns
ElementInternals submission and validity. Keep the child unnamed with `form=""`;
bind its value to `datadom.slices.value`, disabled state to
`datadom.slices.formDisabled`, and input events to the value slice. The capability
retains the host's `value` attribute as the reset default, forwards native
validity, handles fieldset disabling and string state restoration, and exposes
the form/value/validity host API. It does not change generic input patching.
Trusted Enter in a single-line control delegates implicit submission to the
host form, preserving default submitter activation and event cancellation.
It is not a file, checkbox/radio, or multi-value capability. Template-fragment
loaders inherit this metadata from the selected source; they need not repeat it.

`value-request` matches the shared select convention: a changed nonempty token
silently reapplies the authored `value`, even when that default text is unchanged.
An omitted `value` supplies an empty-string default, so an explicit request also
clears a staged value when CEM-ML omits the empty attribute.
Use it when a retained control switches editing context. Repeated tokens and
unrelated renders preserve live edits; removing the attribute does not reset the
value. Reintroducing a token after removal is a new request. Native form reset
still uses the authored default. A hydrated live slice wins on initial attachment.
Consumed tokens are included in portable form state so a fresh runtime does not
replay the request. A pending token change invalidates capture, handoff and stale
provider commits. Tokens identify context or requests; never derive them from
credential values. Entered credentials retain their existing recovery refusal.

Text navigation uses the host attributes `selection-request` (a new nonempty
token for each request), `selection-start`, `selection-end` (nonnegative safe
integers in UTF-16 code units), optional `selection-direction` (`none`, `forward`
or `backward`), and the presence flag `selection-focus`. The shared capability
applies the range after rendering and focuses only when explicitly requested.
Native range clamping/order rules apply. Readonly text may be selected; disabled,
composing and non-text controls ignore navigation. Each change to a nonempty
token is consumed once even when invalid or unavailable, so unrelated renders, reconnects and
later availability never replay focus. Issue a fresh token to retry or revisit
the same range. No input/change event or value mutation is caused by selection.

The shared provider now admits exactly one owned control, offers synchronous
replacement transactions with cancellable synthetic beforeinput, and updates
native value, slices, submission and validity before commit notifications.
Exclusive editor leases and independent validity-contributor leases are transient
and expire on disconnect/editor replacement. A lease's `current` reports its
captured provider connection; `valid` additionally requires exclusive ownership.
This lets the suggestions capability reacquire expired connections while leaving
competing live claims suspended. Author custom validity and native
constraints retain precedence. Composition ownership and handled key presses
remain captured through their terminal key so deferred submission/native surface
routes cannot reinterpret them. Native history remains browser-owned; a runtime
replacement does not promise an undo entry. The full suggestions declaration,
source/render composition and acceptance matrix are still pending in
[the active checklist](../../docs/todo.md#autocomplete-and-suggestions-design-for-cem-inputs).

The provider also exposes a separate `lease.datalist` claim. A trusted host
stages scalar options, registers the native datalist, grants `list` from the
editor producer and `editor-for` from the datalist producer, then calls
`CemSuggestionsPlacementCoordinator.prepareDatalist`. Pass that opaque admission
to `lease.datalist.set({ revision, current, placements })`. It retains the original
input and writes only `list`; authored relationships (including empty or later
same-ID writes) remain authoritative. Grant loss, option/type changes, competing
leases, revision changes and disconnect clear the claim and expire its admission.
The producer owns option cleanup; source lifecycle transitions must refresh the
coordinator. The local declarative `profile="native-datalist"` path now uses this claim and a
separate retained `native-datalist-v1` worker/fallback adapter. Its immutable
`publishDatalist({})` publications share the native component-frame route,
retain attributed source diagnostics, reject query controls and expose no
row-selection proof. The default remains the custom listbox. See the
[native profile design](../../docs/cem-suggestions-native-profiles-design.md).

The package-private native capability-session transport retains executable sources
through CEMB and exports explicit presentation values through CEMV. Prepared
sessions retain original owners, lexical capture, grants and source revisions
across queries. Immutable publications capture query revisions separately. Label-template inputs stay native, including authored descendant
references; per-session namespace completion leaves source ASTs unchanged.
Release/cancellation/disposal fence late work, and worker loss requires fresh
source authority instead of restoring old handles in fallback. The source/session
channel is not a new public suggestions API or durable snapshot field. See the
[adopted transport boundary](../../docs/cem-suggestions-data-design.md#native-session-transport).

The versioned native suggestions adapter now admits canonical CEM options,
HTML data and HTML option/group roots in those sessions. It requires materialized
scalar source attributes, preserves original source/content identity and rejects
invalid revisions atomically. Unicode 17.0.0 filtering produces immutable native
row/group views; row/group label templates use those native frames. Explicit
scalar queries can export CEMV, while a whole live view is rejected because its
source/content relationships cannot be preserved there. Host-admitted outbound
bindings now route canonical component frames to the original publication owner,
with one live owner per frame and ordinary materialized native inputs. Native
`editor-for` resolves exact provider endpoints without granting authority.
The shared placement coordinator admits exact opaque native rows mapped by a
trusted host to committed editor/listbox/row endpoints. It requires property-specific
grants in both directions before reserving IDs or activating provider claims.
Endpoints may share the same `current` freshness predicate. Each synchronous
listbox admission evaluates a shared predicate once before its endpoints and
again after all endpoint reads; predicates must be independent of invocation
count. Every endpoint still receives its own identity, native-row and grant
checks. No predicate result survives the admission call, so subsequent reads,
mutations and source/runtime revocation require fresh validation.
The shared controller uses those admissions for keyboard/mouse activation,
cancellable commits, selection provenance and leased validity. Query filtering
preserves original source identity; source release and worker loss synchronously
revoke proofs, while publication release only expires its consumer rows.

Declarations can opt into `capability="suggestions"` through the runtime’s
`suggestionsControllerInputs` host hook. That hook must supply the exact editor,
a direct component-owned `part="surface"` listbox and native preparation with
current placement grants. Markup alone does not grant authority. The hook and
row handles are transient; resume requires fresh admissions. Native option shells
use `@suggestion-row={#row}` to return bounded consumer placement metadata, removed
from DOM/CEMV output. `runtime.renderedSuggestionsFor(instance)` supplies exact
committed shells and native row handles for host-issued placement grants. Copies,
old views, ambiguous mappings and later render attempts cannot reuse the authority.
For opt-in platform close requests, supply a `platformClose` coordinator from
`createCemPlatformCloseCoordinator(window, admit)` in that host hook. The host
must control native watcher creation and return revocable authority for the
original editor and explicit ancestor lifetimes. Call `consume(event)` before
another native surface/watcher uses the same interaction. The runtime cannot
infer independent browser grouping from activation or markup alone. Admission
currently covers deliberate, fresh Arrow Up/Down openings; focus-only opens and
query refreshes do not acquire watchers. Pending cancellation, revocation and
teardown release the reservation; Escape/IME continue through the existing editor
lease. This does not enable platform close in the ordinary local gallery or the
native-datalist profile. See the [adopted close contract](../../docs/cem-suggestions-popup-design.md#non-key-platform-close-requests-adopted-design)
and [acceptance boundary](../cem-components/docs/suggestions-acceptance.md#platform-close-integration-2026-10-08).

Hosts can instead opt into local composition with
`new CemElementRuntime({ localSuggestions: true })`. The shared adapter admits a
single local slotted editor managed by that runtime, captures the original inert
options template from the payload island at the native XML/CEM import boundary,
and issues exact editor/listbox/row placement grants. It preserves the source
session across queries and uses canonical component compilation inputs on that
same processing owner. An absent source is ready-empty. Explicit inputs and
ambiguous slots conflict; foreign endpoints/sources require their own host hooks
and grants. Use the standard inert payload envelope for editor-plus-template
composition. `runtime.setLocalSuggestionsEnabled(false)` revokes affected local
controllers, source authority and provider claims synchronously; enabling again
or reconnecting obtains fresh collapsed admissions. Owner loss cannot restore
local authority from markup. The host opt-in and live bindings are excluded from
snapshot state. Protocol v22 carries the bounded native XML source import.

The shared controller exposes immutable presentation feedback with state, query
revision, eligible count and focused-session qualification. The declarative
capability can update one direct plain-text `part=status` live region
(`role=status`, `aria-live=polite`, `aria-atomic=true`) and the listbox's busy
state. The declaration supplies `pending-message`, `failure-message`,
`empty-message`, `single-message` and `multiple-message`; `%count` inserts the
eligible count. A nonempty host `options-error` supplies failure text. The runtime
does not supply language strings or touch the field's help/error/busy state.
Preview leaves unchanged status text alone; dismissal, composition, unavailability,
revocation and disconnect clear qualifying feedback. Late preparation cannot
restore an announcement after dismissal.

Native row/group label rendering admits its complete output before returning a
presentation plan, including rich native source content. It accepts a passive
HTML/SVG subset and rejects controls, focus/role/ID overrides, executable
attributes, host writes and template-owned styles or module mappings. One
node/depth/byte budget covers the projected subtree; invalid elements and
attributes retain their source attribution. This output check does not evaluate
authored descendant references or change the original source tree. A rejected
label does not invalidate the retained source session.

The local adapter captures `slot="option"` and `slot="group-label"` inert
`text/cem-ml` templates with their original namespace contexts. Each runs with
only its explicit native `suggestion` or `group` parameter and the standard
library. Default and custom labels publish atomically as native `labelContent`;
one shared output budget and retained-session byte budget cover the publication.
Source/template edits invalidate its leases. Observer-driven row synchronization
waits for the committed parent frame and fences old connection callbacks;
pending/failure scalar readiness withdraws the prior view immediately while
retaining focused status feedback. The production `cem-suggestions`
XHTML declaration and linked playground/gallery use this first local profile.
The shared row adapter admits mouse/touch/pen sequences and preserves touch/pen
pointerdown for native gestures. It suppresses compatibility-mouse focus only on
an exact armed row and retains a completed touch press through normal implicit
capture release. Native cancel, drag, scroll, blur and revision loss invalidate it.
Chromium browser-input plays verify both field declarations and worker/fallback
execution; physical devices, operating-system IMEs, screen readers and the full
source-example audit remain in the [acceptance record](../../packages/cem-components/docs/suggestions-acceptance.md)
and active checklist.

Changing an option's `selected` presence or replacing selected option nodes
refreshes the owning select after the whole render transaction. Single selects
use the last explicit selection, or native empty-selection rules when no option
is selected; multiple selects use all explicit selections. Unchanged option
identities and selected attributes preserve user choices during unrelated renders.
Select identity, focus and form-reset defaults are retained, and refresh does not
dispatch input or change events.

Set an explicit native `select[value]` binding to control the current selection
on every owning render, including unchanged plans and superseded updates. It
applies after options render and takes precedence over their `selected` defaults.
It uses native `select.value` semantics: one matching option, or no selection
when none matches; an empty string matches an empty-valued option. Removing the
binding restores option defaults, then subsequent unchanged renders preserve
user choices. Form reset still uses option defaults; the next owning render
reapplies the explicit value. Parent renders do not refresh a nested component's
selects. The NPM URL demo uses `currentversion` for this control and keeps
`initialversion` for ordinary defaults.

The [demo teaching-point audit](docs/demo-teaching-points.md) records the lessons
preserved from the local prototype and the standalone/source-loaded checks.

After each DOM commit, the runtime recaptures its own forms and reapplies
custom validity. Inserting or removing a required control updates form data,
validation state and form slice mirrors without another input event. Nested
instances retain ownership of their forms. `runtime.whenRenderSettled(instance)`
includes these refresh renders. Circular form rules stop after eight refreshes
with `cem-element.form_state_unstable`; a later state change can render again.

`custom-validity` retains its legacy boolean/message expression contract:
`condition ?? "Explain the error"` clears the message when the condition is
true and supplies it when false. Fallbacks retain their selected value, so
`str:length(datadom.slices.text ?? "")` can safely appear inside a rule.
Missing recognized data paths are empty. `str:length` and legacy `string-length`
count Unicode codepoints. In this validity dialect, false, null/missing, empty
strings, zero and the string `"false"` select the fallback; ordinary CEM-QL
queries use null/empty-sequence coalescing and retain false and zero.

Choice declarations provide a focusable `.cem-select__control` or a wrapper
with focusable children. The shared choice capability uses that control, or
its first usable HTML descendant, as the native
[validation anchor](https://html.spec.whatwg.org/multipage/custom-elements.html#dom-elementinternals-setvalidity).
It skips disabled, hidden and inert elements, preserves authored tab stops
(including `tabindex=-1`) and refreshes the anchor after each DOM commit.
Updating the anchor does not move focus; native invalid submission or `reportValidity()`
can focus the current anchor without changing the selected value.

CEMT and XSLT render plans derive node IDs from native source provenance, the
parent ID and the occurrence at that source site. Conditional output at another
site leaves those IDs stable, allowing unaffected DOM nodes to retain browser
state. The [data-tree demo](demo/data-tree.html) exercises a collapsed disclosure
while its parent's selection label disappears. Repeated results from one source
site remain positional within their parent; item-keyed reordering is outside
this identity contract.
Direct whole-plan projection retains its existing recovery rule: a changed
root-ID set replaces the scope with a diagnostic. Distinct authored root
branches, such as the storage demo's object `ul` and array `ol`, have distinct
source-based IDs and therefore use that recovery path.

For event-driven browser navigation, `location-element` accepts an optional
`trigger` token. An empty token does not write; a new nonempty token is consumed
before navigation, once per writer position in an instance. Rerendering a live
reader or editing a pending target with the same token does not reapply that
command. Slice event payloads expose a per-slice `revision` that advances even
for repeated equal-value events; use it as the token. Keep writer positions
stable, or isolate independent conditional writers in separate instances.
Omitting `trigger` preserves continuous/authoritative writer behavior. See the
[URL writer demo](demo/set-url.html) and [two-way version picker](demo/npm-versions-demo.html).

URI-backed canonical CEM-ML declarations and inline canonical declarations use the same
retained worker/fallback processing host. A host `loadSrcDocument` hook may keep returning
a complete string, or return `{ body, resolvedUrl, resolverIdentity }`, where `body` is an
`AsyncIterable<Uint8Array>`. The stream form preserves module-map/resolver identity and
makes the imported URL the base for relative resource controls inside that declaration.

A CEM-ML template can explicitly reference a separate XPath function library:

```html
<template type="text/cem-ml" xpath-functions="./fruit-functions.cemt">
{attribute @name=text}
{output | {$native:call("demo.label", text)}}
</template>
```

The library is a CEMT module containing public named functions with typed
parameters and XPath bodies; see the [interactive examples](demo/xpath-functions.html)
and [library source](demo/xpath-functions.cemt). Its reference uses the declaration's
scope-aware module resolver, including module maps and host resolution hooks. Relative
URLs resolve from the declaration document. A host redirect's final URL is preserved
as the library source URI. The library is a separate dependency, never a render-data
field, and inline function declarations do not implicitly enable it.

Each logical scope loads an immutable source snapshot once per resolved URL/policy.
The processing host shares retained companions by URL, source hash, compiler versions
and resolver policy. Worker transport carries bounded source and identity metadata;
the worker or fallback validates and compiles once, then selects its retained companion
for subsequent renders. Binary template cache hits still require that separate library.
The last evicted template consumer releases its companion; failed compilation and scope
disposal also release handles. Failed source loads may retry on the next render. Use a
new versioned library URL or a fresh scope to load changed source bytes.

This browser route accepts explicitly declared scalars and imported CEM nodes.
For example, the compatibility XML reader can be authored as:

```cem-ml
{cem-data @name=document @select=datadom.slices.source @type=xml @projection=xpath}
{cem:for-each @as=item @select='native:call("demo.items", document.root)' |
    {p | {$item}}}
```

The reader's `error` field reports invalid XML; check it before calling a function
with `document.root`. XPath selects nodes from the CEM tree, preserving source
ownership, identity and source maps. Worker and fallback hosts retain up to 16
successful imports per compiled template, keyed by source, format and projection,
and release them when the template is disposed. Each input is limited to 32 KiB,
64 levels and 4096 source events or values. Changed input gets a distinct owner;
unchanged cached input is not reparsed. Ordinary CEM reader roots also supply
native nodes. Records and source strings are not inferred as nodes.

The [XPath XML table/tree demos](./demo/xpath-nodes.html) use the separate
`xpath-nodes.cemt` function library and explicit XML XPath reader view.
Names retain their namespace URIs, adjacent text/CDATA shares a text node, and
CEM-QL sorting preserves each native node for XPath parent/sibling navigation.
The table selects rows by unique authored `@id`. The sorting gallery below
uses standard XPath `fn:sort`; HTTP ownership uses retained CEM documents.

The [XPath sequence demos](./demo/xpath-sequences.html) use
`xpath-sequences.cemt` for rounded word windows, reversal, head/tail selection
and distinct word counts. A second example discovers XML columns and matches
native cells by kind, local name and namespace URI. Its XPath expression
explicitly preserves first-seen column order; it does not depend on
`distinct-values` result order. Live edits add new columns without new field
expressions. Missing cells display ∅, empty cells display `""`, and repeated
cells join with ` / `.

The [XPath sorting demos](./demo/xpath-sort.html) use `xpath-sort.cemt` for
stable text/numeric ordering and multiple row keys. Inline functions capture
scalar direction settings inside XPath and return retained source nodes;
selection and source-sibling navigation survive reordering. Missing/invalid-last
keys are explicitly authored. Only codepoint collation is supported, and compiled
XPath program format v3 requires recompilation of older v1/v2 programs.

The [XPath validation demos](./demo/xpath-validation.html) use
`xpath-validation.cemt` for form fields and a local IPv4 prefix-length rule.
Regex lexical checks combine with integer ranges and `some`/`every`;
regex tokenize/replace format accepted tags. The demos explain the supported
regex subset and explicit exclusions. They do not implement subnet matching
or network enforcement.

The [XPath aggregate demos](./demo/xpath-aggregates.html) use
`xpath-aggregates.cemt` for decimal-list statistics and a live XML basket.
Each newly added fruit element contributes to the sum, extrema and average.
Authored XPath validates amounts before explicitly casting to decimal;
invalid/missing amounts are errors, empty input sums to zero, and absent
extrema/averages display ∅. Terminating decimal averages are exact; repeating
averages follow the existing 18-significant-digit half-even division policy.
Nonnumeric extrema and duration aggregates remain outside this native slice.

The [XPath maps/arrays demos](./demo/xpath-maps-arrays.html) use
`xpath-maps-arrays.cemt` for an optional-entry IP-filter preview and an XML basket
selected by one-based array position. Native maps/arrays stay opaque between
named calls; only explicit XPath lookup selects their contents. Empty-valued
entries differ from absent keys, and square arrays keep empty member slots.
These demos use declared scalars and retained CEM trees imported from XML and
JSON. The third maps/arrays case imports the standard JSON-to-XML tree, retains
selected nodes in an XPath array, and distinguishes null, empty and absent
members. XML, JSON, YAML and CSV readers share one native XPath node view;
format-specific code belongs to CEM AST import. The accepted
[loader contract](../../docs/cem-data-loader-plan.md) connects HTTP XML/JSON
to that same tree through the CEM-ML library. See the
[import principle](../../docs/cem-data-import-principle.md).

Library source and URI are each limited to 32 KiB, libraries to 64 XPath functions,
and each scope to 64 loaded
libraries; the WASM companion registry also enforces its count/byte limits. Calls share a 16,777,216-unit XPath work counter across nested expressions and
limit intermediate/final text to 1 MiB of UTF-8 atomic lexical bytes per value
or sequence. Atomizing native XML also obeys these limits; retained document
owners are not serialized or counted as output text. Limit failures cannot be
caught as CEM-QL data errors. The [live counts](demo/dom-merge.html) and
[string comparisons](demo/functions/str.html) demonstrate XML whitespace,
Unicode codepoints, tokenization and joining through [a text library](demo/xpath-text.cemt).
Library imports and templates using the older non-retained resource-render path are not
supported. Generic template binaries and
ordinary JSON data cannot install functions. Native-owner bindings remain available
at the Rust API described in the [CEM-QL contract](../cem_ql/README.md#xpath-function-companions).

Strict XSLT 3.0 declarations use `src="./view.xslt"` and
`xslt-template="entrypoint"`, with direct `<xslt-param name="..." select="...">`
children containing independent CEM-QL scalar selectors. Unmapped parameters
keep their XSLT defaults. Named entrypoints may run without initial focus;
source parsing stays in CEM-ML import. Imported stylesheets and native bundles
share the existing resolver, worker/fallback and disposal lifecycle.
The environment sets `CemElementRuntime({ controlInputBytes })` (default 8 MiB).
An explicit declaration scope may lower this through
`createCemDeclarationScope({ document, parent, controlInputBytes })`; omitted
values inherit and increases above a parent or environment ceiling fail.
Anonymous tags distinguish declaration runtimes and policies; a fixed public
tag registered under another policy is an incompatible declaration.
The effective limit is retained with the native component and checked in UTF-8
bytes before control decoding, independently of document import limits.
See the [control-input policy](../../docs/xslt-runtime-lowering.md#browser-control-envelope-decision)
and [parameter contract](../../docs/xslt-runtime-lowering.md#browser-state-binding-decision).

Phase 3B shares lazily allocated worker slots across compatible logical roots without
changing their scheduling semantics. The current `cem-processing-host-v4` contract
carries explicit native XSLT compilation options and retained host/scope limits;
its `document` retention/release remains alongside `compile`, `renderDiff`, `cancel` and `dispose`. The default
pool uses browser hardware concurrency capped at eight workers, a 64-operation queue per
slot, FIFO ordering per root, and round-robin cross-root dispatch. New CEM compilation
owners pass through the [checked bounded native cache](../cem_ql/README.md#checked-template-compilation-cache)
and receive independent WASM handles; processing rejects superseded or disposed
async results before publication. CSS/XSLT compiled owners and render plans retain
their bounded content-addressed LRU behavior; an evicted previous plan safely
falls back to a full scope replacement. `processingPoolPolicy` can lower the worker and
queue limits, while `onProcessingTrace` observes sequence-only scheduling decisions.

`<http-request>` is lowered by CEM-QL to a clone-safe host-control descriptor before the
worker render plan is diffed. URL resolution, policy, response streaming, `AbortSignal`,
and stale-resource revisions remain main-thread host responsibilities. Template-visible
states use the portable lifecycle vocabulary: `scheduled`, `in-progress`, `loaded`, and
`failed` for the implemented transitions. CEM-ML imports bounded response bytes
into a retained CEM tree. During native rendering, `datadom.slices.<name>.data`
is a CEM document node accepted by CEM-QL and XPath functions; JavaScript
snapshots carry lifecycle metadata with `data: null`. Worker messages carry
bytes and explicit execution-local bindings, and fallback re-imports those bytes
through the same library. Owners are released on replacement, disconnect and
scope disposal. See the [loader contract](../../docs/cem-data-loader-plan.md)
and [HTTP examples](demo/http-request.html). Progressive CEM-ML AST streaming
remains a later phase.

`<repository-query>` and `<storage-status>` use the same transient-control boundary for
logical repository reads. Applications register host-owned ports with
`CemRepositoryRegistry` and inject `registry.readOnly()` through the runtime's
`repositoryRegistry` option. The facade exposes only `query`, `subscribe`, and `status`;
it deliberately has no `execute` capability.

```cem
{repository-query
    @slice=projects
    @repository=studio-projects
    @operation=list-projects
    @parameters="{$datadom.attributes.query-parameters}"
    @live=true
    @cursor=0}
{storage-status @slice=storage @repository=studio-projects @live=true @cursor=0}
```

`parameters` is optional JSON; the example reads it from an instance attribute such as
`query-parameters='{"includeTrash":false}'` so the JSON reaches the declaration as one
interpolated value. `live` subscribes from the optional non-negative durable change cursor,
and every query revision is runtime-owned. Both resources project
`scheduled`, `loaded`, or `failed` envelopes under `datadom.slices.<name>`, reject stale
completions, and release subscriptions and abortable queries when superseded, removed,
or disconnected. `storage-status` reports the port's existing quota/persistence state;
rendering cannot invoke a repository mutation or call `navigator.storage.persist()`.

## Production-Ready Trigger

The package is considered browser-substrate production-ready when this command passes:

```bash
yarn nx run cem-elements:verify
```

That aggregate gate runs:

- `cem_ml_cli:validate-fixtures`
- `cem_ml_cli:e2e`
- `cem_ml:bench`
- `cem-elements:verify-substrate`
- `cem-elements:verify-legacy-fixtures`
- `cem-elements:verify-material-fixtures`
- `cem-elements:verify-cemt-pipeline-story`
- `cem-elements:verify-package`
- `cem-elements:test:unit`
- `cem-elements:test`

The gate covers file-backed legacy fixtures in `tests/parity/legacy/`, material parity fixtures in
`tests/parity/material/`, substrate CEM fixtures in `../../examples/cem-elements/`, unit coverage, Storybook browser
parity stories, and a Playwright screenshot check for the CEMT formatter/coloring/writer pipeline story.

The Phase 2 engine legs read both parity manifests directly. They extract every
inline or external declaration template, lower legacy bodies through the shared
Rust converter, and validate all 40 source sides under the package-owned
`https://cem.dev/ns/template/cem-element/1` profile. The same inputs run through
CLI roundtrip e2e, while `cem_ml:bench` applies the AC-N-1 aggregate budget to
each source side.

## Fixture Locations

- `tests/parity/legacy/` — legacy `<custom-element>` behavior mapped to CEM-ML/browser substrate fixtures.
- `tests/parity/material/` — the eight material reference components: `action`, `autocomplete`, `badge`, `dropdown`,
  `icon`, `icon-link`, `input`, and `menu`.
- `docs/legacy-parity-inventory.md` — legacy behavior support matrix and bridge/adoption deferrals.
- `docs/material-parity-inventory.md` — material feature support matrix and production-gate caveats.

## Handoff Condition

Passing `yarn nx run cem-elements:verify` means the `<cem-element>` browser substrate is ready for the Phase 3.5
Edge/SSR follow-up. It does not mean the legacy `@epa-wg/custom-element` package has adopted this implementation.
That adoption remains a later Phase 3.6 handoff after Edge/SSR boundaries are in place.

## Known Deferrals

- Full legacy XPath and broad XSLT behavior remain bridge/adoption work. The supported migration path is CEM-ML plus
  CEM-QL over structured `datadom.*` records.
- Scoped template styles intentionally render as page-global light-DOM styles for this gate; selector containment is a
  separate bridge/adoption primitive.
- Host-owned resolution remains explicit for bare module specifiers and external resource policy hooks.

## Building

Run `yarn nx run cem-elements:build` to build the library.

The build vendors the exact `cem_ql:build:wasm` browser module, declarations,
and WASM binary under `dist/lib/internal/runtime-support/vendor/`; published
runtime modules never import a monorepo-relative `packages/cem_ql` path. Run
`yarn nx run cem-elements:verify-package` to compare those bytes and verify the
real npm archive and clean-consumer import.

Package verification hashes completed dependency outputs with `**/*` (including
transitive WASM comparison inputs), the pinned source notice, package metadata,
README/license files, and Node/npm versions. Output paths are workspace-relative;
`dist/**` misses `packages/cem-elements/dist` and can replay a stale success.
Keep this gate cached, but do not replace output hashing with a source-only glob
or hash generated files before their build dependency finishes.

For a cache regression check, run `verify-package` twice normally, add a temporary
unexported TypeScript module under `src`, run again, change that module's emitted
content and run again, then repeat unchanged. Added/changed outputs must give
new verification hashes and execute the check; unchanged output must reuse it.
Remove only the temporary module and rerun to restore the original build. Inspect
both the verifier's package count and its task hash/cache status in
`.nx/cache/run.json` (Nx 22.7); a replayed success message alone proves nothing.
Do not use `--skip-nx-cache` for this regression check. Run it without concurrent
builds or edits to this package.
The [2026-10-08 evidence](docs/package-cache-evidence-2026-10-08.json) records
219 → 222 → 222 → 219 packaged files, distinct hashes for the two emitted module
versions, and successful cache reuse for unchanged and restored outputs.

## Testing

Run `yarn nx run cem-elements:test` to execute the runtime stories through Storybook Test.

Run `yarn nx run cem-elements:verify-demo-coverage` to require the unit contracts,
Storybook Chromium suite, and independent standalone/source-document demo verifier
together. The [authored page-and-legend inventory](docs/demo-storybook-coverage.json)
names one inventory-owning story for every HTML document, including supporting
libraries. Browser parsing checks its exact sample legends; after the owning
story's `play`, the preview verifies the real `<cem-element src>` declaration and
produced legends without adding a readiness wait. Unit checks reject missing
pages, stale story exports, and non-asynchronous or excluded owners.

When adding or changing a demo, update that inventory and its owning story along
with the source-contract and independent gallery checks. Inventory ownership
proves source loading and sample presence; each sample's rendered outcome and
interactions still need explicit `play` assertions. A library without sample
cards uses an explicit document or fragment story, not an inventory exclusion.

Run `yarn nx run cem-elements:verify-cemt-pipeline-story` to build Storybook and visually verify the CEMT output
pipeline story's formatted CEM tree, colored CEM tree, and writer output stages.

Run `yarn nx run cem-elements:storybook` to open the interactive Storybook runtime fixture surface.

### Native JSON storage

`{local-storage @key=preferences @slice=preferences @type=json @live=true}`
imports stored bytes through CEM-ML. `datadom.slices.preferences` is a native
CEM document; select its children and use `dom:text` for text. The
[storage demo](demo/local-storage.html) shows generic-data object properties,
arrays, scalars, and a basket edited through native `slice-value` attributes.
CEMT constructs replacement nodes without modifying the imported tree.

Native slice event values travel as portable CEM artifacts across workers,
fallback and saved hydration. Explicit writes export compact JSON through
CEM-ML, preserving property order, duplicate keys and exact number text.
Unchanged reads preserve original source bytes. Invalid reads keep raw storage
and publish a diagnostic with no native tree; invalid writes leave storage
unchanged. Empty/null slice writes remove the key, while a native JSON-null
node writes the literal `null`. Scalar storage types retain their established
browser-input coercion. Native event values target slices; native component
attributes use CEMT attribute construction.

### Form-associated custom control event values

Declarative event bindings read a live string `value` from native buttons, inputs,
textareas, selects and custom controls whose constructor declares
`static formAssociated = true`. This applies to `$target.value`, value aliases,
the default slice value and serialized event targets. It lets a parent bind a
`cem-select` change directly to its own slice. Non-string custom values are not
serialized as event target values; ordinary elements retain attribute fallback
only for default values and aliases.

An event on an icon, label or other descendant inside a native button reads the
nearest button's live value. This applies to the same bindings and serialized
target value, while the serialized target tag remains the actual clicked node.
Delegated menu activation therefore behaves consistently when users click a
button's glyph, text or surface.

Asynchronous checked-state publication preserves native checkbox/radio edits
made after that render was requested. Authored `checked` defaults still update;
a newer native selection keeps its live state across the radio group sharing
its name, root and form. A later authored change remains authoritative. No
synthetic input/change event is emitted during reconciliation.

### Native time and range inputs

`capability="time-input"` preserves one authored text `input[slot=input]` and
an optional native `button[slot=toggle][type=button]`. The declaration supplies
`part=root`, `part=popup` with `role=listbox` and `popover=manual`, and options
from `datadom.slices.options` with stable IDs and `data-option-index`. Native
input value, labels, validation, reset and form ownership stay on that input.
Project the slots in `datadom.slices.ownerSlots` order to retain authored owner
order, including a toggle placed before the input.
Canonical HH:mm choices use host min/max/interval or direct cem-option payload.
The silent expanded property controls presentation; commits close the popup and
focus the input before its input/change event pair. Composition and modified
editing keys, native readonly and inherited fieldset-disabled state are honored.
Disconnect closes transient presentation and invalidates pending reset work.
Disabled availability reaches the exact native owners at mutation delivery.
A retained list with matching values and disabled states can update its active
and selected state and native visibility immediately; changed options wait for
publication. Calendar close restores the editor and publishes its native commit
pair before any asynchronous follow-up render.

`capability="range-input"` preserves one `input[type=range][slot=single]` or a
pair with slots start/end. The existing data-cem-slider-thumb attribute remains
a compatibility alias; conflicting aliases and extra/nested owners are invalid.
The declaration supplies part=root, part=inputs, and optional single-value,
start-value and end-value parts inside an aria-hidden visual. Bounds and step
are projected silently. Non-crossing clamps run in capture before native events
reach application listeners. Form reset resynchronizes positions and labels
without dispatching replacement events. Private --_cem-range-start-position,
--_cem-range-end-position and --_cem-range-tick-spacing properties carry value
percentages. Neither capability creates a second form-associated host. Fragment
imports inherit the source's `time-input` or `range-input` capability; retain that
metadata when copying a template.

Project the slots in `datadom.slices.thumbSlots` order inside part=inputs. The
list preserves reversed start/end payloads and the legacy default-slot pair;
`datadom.slices.mode` reports single, range or invalid from the current authored
payload. Parent mode changes retain surviving native input identity and value.

The [shared time/range fixtures](src/lib/time-range-input.stories.ts) run worker
and fallback ownership, reset, reconnect and malformed-payload checks.

### Navigation and content capabilities

`navigation-landmark`, `tab-panels`, `workflow-panels`, `tree-view` and
`native-list` are versioned declarative capabilities. They identify owned native
nodes through public parts and structural ownership, so declaration tags can be
renamed. None makes the host form-associated. The tabs/workflow capabilities
consume direct `cem-tab`/`cem-step` payloads and retain stable panel relationships;
`tree-view` consumes recursive `cem-tree-item` payload. Selection/expansion and
published event details keep their component contracts. `native-list` normalizes
direct `cem-list-option` payload into an options slice for a native select.

The worker/fallback fixtures in `navigation-content.stories.ts` cover renamed
hosts, nested panel owners, presence flags, reconnect, native owner retention,
modifier/composition keys, hidden-panel recovery and outside-focus preservation.
Mixed canonical/legacy component stories load built compatibility declarations
through the shared preview; build cem-components before running those stories.

### Disclosure and data capabilities

`disclosure-panel`, `sort-action` and `pagination` are versioned declarative
capabilities. They use structural ownership and public parts, support renamed
hosts, and do not make hosts form-associated. Fragment imports inherit the
source's capability; they need not repeat it on the importing declaration.

- `disclosure-panel`: direct `part=root` contains a `part=heading[role=heading]`
  and its native `button[part=header]`, plus a persistent `part=panel`. The
  capability provides stable `headingId`, `summaryId`, `panelId` and validated
  `headingLevel` slices. Native click toggles presence-only `expanded`; disabled
  suppresses user activation. The template owns visibility and exact ARIA links.
- `sort-action`: direct `part=root[role=columnheader]` contains its native
  `button[part=button]`. Slices provide normalized direction, indicator and action
  label. Native activation cycles none/ascending/descending, clears other
  admitted sort owners in the nearest native table or table/grid role, and emits
  one `cem-sort`. Nested tables are independent; ordinary columnheaders are not
  enrolled. Data ordering and result announcements remain application-owned.
- `pagination`: direct `nav[part=root]` contains an optional
  `label[part=page-size] > select`, and `part=range-actions` with native
  `button[part=action][data-page-action=first|previous|next|last]` controls.
  Normalized slices supply numeric state, options, labels and availability.
  User transitions emit one `cem-page`; boundary clicks are suppressed in
  capture phase while a surviving button retains focus. Page-size changes
  preserve the first visible item. Invalid positive fractions below one use
  the default size of 50, avoiding division by zero.

All author changes are silent. Presence includes empty and false-valued flags.
Listeners are released on disconnect and reconnect once; exact native owners
survive live updates. See [worker/fallback fixtures](src/lib/disclosure-data.stories.ts).

### Read-only progress preparation

`capability="progress-status"` prepares a retained declarative status graphic from
`value`/`max` without installing event listeners, mutating attributes or managing
focus. Absent value is indeterminate; a present empty/invalid value is zero.
A missing/non-finite/non-positive maximum becomes 100, with finite values clamped
to 0..max. Slices expose `mode`, `indeterminate`, `value`, `max` and a normalized
100-unit `dashArray` (`25 75` for unknown progress). Divide before multiplying
to avoid overflow for large finite ranges. Worker and fallback executions share
the same preparation. Native linear progress can instead retain browser normalization.


### Declarative host-service views

For a service view inside retained declarative panes, pass `element` with the
already-authored direct child of `root`. Its tag and runtime declaration must
match, and it must not already have a host-view boundary. Binding initializes
slices and command authority without replacing the child or its siblings. The
parent declaration continues to own its DOM lifecycle; disposing the binding
revokes commands and updates but leaves the authored element in place. A disposed
boundary remains isolated from ancestor commands and cannot be rebound. Default
mounts without `element` retain their existing replace/remove ownership contract.


`mountCemView(runtime, { root, tag, state, commands })` mounts a registered
XHTML/CEM-ML view. Host JavaScript supplies non-UI services and explicitly named
state slices; the declaration owns visible markup, status mapping, disabled
state and interaction bindings. `view.update(values)` updates slices and
`view.whenSettled()` waits for the view and its produced descendants, settling
ancestors before descendants so nested payload updates are included. It walks
unchanged subtrees without querying all descendants for every owner. If an owner
publishes DOM changes while settling, traversal restarts in current DOM order;
new or replaced owners are included. Disposal stops further waits, and the
temporary mutation observer is released on completion or failure. Native surface
preparation is separate: `cem-open` reports admitted visibility.

Declare `host-command="save"` on a control, with optional
`host-command-event="input|change|submit|cem-close|cem-tree-activate|cem-tree-toggle|cem-disclosure-toggle|cem-tab|cem-preview-navigate"`
(one event, default `click`). Only
commands supplied by this mounted host are callable. The shared runtime invokes
the service synchronously on the event stack, preserving browser user-gesture
requirements. The service receives a frozen `{ command, eventType, value,
isTrusted }` record, with the current string value of a native or form-associated
control, a validated tree/tab activation value, or null; it receives no DOM node.
Clicks on nested icons or labels read the enclosing native/form-associated
control's value, stopping at the bound command element. Values are never taken
from unrelated controls outside that boundary.
Submit bindings prevent native
submission for an authorized command. Declare submit commands on native forms;
the request additionally includes frozen `fields` mapping names to arrays of
successful string values, captured synchronously with native `FormData` and the
submitter. Repeated names retain order; disabled/unnamed controls follow native
submission rules. File values and form controls belonging outside this mounted
view (including nested views) reject the command and appear in `view.errors`;
no partial input is delivered. Services must implement
persistence, routing, workers or browser APIs, never DOM/UI behavior.

Surface dismissal uses `host-command-event="cem-close"` on the surface host.
Only that host's own close event invokes its command; a nested surface cannot
dismiss its ancestor task. Cancelled close requests do not emit a close command.
The request remains a scalar record without surface nodes or event detail.

Tree navigation uses `host-command="open" host-command-event="cem-tree-activate"`
on the tree host. Only an event targeted at that bound owner is eligible;
descendant treeitems and nested owners cannot invoke its command. A `CustomEvent`
must carry an own data property `detail.value` containing a non-empty string
without whitespace, matching the tree's identity contract. Missing, inherited,
accessor, array or non-string values reject delivery with a `TypeError` in
`view.errors`. No accessor is evaluated and no additional detail is transported.
The frozen request receives the exact string as `value`. Expansion, focus and
selection remain under their existing tree/application owners; the bridge never
selects or navigates on their behalf.

One composite owner can bind several supported events independently. For example,
`host-command="open" host-command-event="cem-tree-activate"
host-command-cem-tree-toggle="expand"` routes activation and disclosure to distinct
services. An event-specific `host-command-<event>` takes precedence for that event;
removing it revokes that binding. `cem-tree-toggle` follows the same whitespace-free
identity checks as activation. Opt-in `cem-disclosure-toggle` carries a non-empty
section identity; the bridge forwards only its validated scalar `value`, never
the detail object or an owner node. Programmatic disclosure publication stays
silent. All bindings share the same nested-owner, cancellation, disabled and
checkpoint acknowledgment guards.

Disabled controls, cancelled events and unavailable commands do not invoke a
service. Input capture rechecks mounted-view/declaration authority and the command
binding before invocation. `isTrusted` reports the delivered event's actual value:
tree activation is a custom event and stays false even after native pointer or
keyboard input. It is not a browser permission credential. File/install actions
continue to use their own native activation and synchronous service invocation.

Bindings belong to trusted application declarations, not arbitrary document
content. A nested mounted view is a separate command boundary. State updates
preserve native owner identity, edits and focus through ordinary runtime
reconciliation. Synchronous failures and rejected promises are available through
`view.errors`; reporting an error does not construct UI. `dispose()` revokes
commands, stops state updates and removes only that view. Applications project
user-visible failure state through their own service-state contract. This is a
host adapter, not a serialized source of authority or a component behavior hook.

Binary-bearing edge content addresses use `stable-json-fnv1a64-binary-v2`.
Their canonical representation includes each buffer's actual byte length and
FNV-1a byte digest; repeated references reuse the digest within that call. Copied
and shared buffers therefore have identical content identity, and later mutations
are rehashed. Supplied native `contentHash` labels never substitute for checking
bytes. Nonbinary values keep the existing `stable-json-fnv1a64-v1` addresses.
Older binary-bearing v1 addresses fail current content verification and require
fresh publication; they are not silently reinterpreted. These hashes are cache
identities, not cryptographic authentication or authorization evidence.

Tab-local service routing may bind `host-command-event="cem-tab"` on the tab
owner. Only that owner's own event is accepted. The bridge reads an own nonempty
string `value` and discards indices and other event detail; inherited properties,
accessors and nested owners cannot acquire command authority. Unlike tree token
identities, tab values may contain whitespace. Services can project admitted pane
state through declaration slices, including revealing a retained editor before a
source-range request; no tab DOM methods or app event listeners are needed.


### Script-free live previews

`mountCemLivePreview(frame, { source, sourceUri, resources, styles?, declarations?, onError? })`
returns `{ ready, checkpoint, dispose }`. `resources` contains explicit `{ uri, contentType, text }`
transport records, `styles` names supplied CSS resources, and optional `declarations`
contains `{ tag, uri }` entries. The `live-preview` declarative capability backs
`cem-media-preview mode="live"`; ordinary applications should use that component.

Each mount creates a fresh iframe document, custom-element registry and root
CEM declaration scope. The parent runtime owns compilation, native resource import,
light-DOM rendering and event bindings. Source is installed as inert template text.
No project-authored JavaScript runs. The frame allows same-origin access for the
parent but has no script, form-navigation, popup or download permission. A fixed
CSP denies network; only explicitly supplied native resource bodies can load.
`restrictedPreview` additionally rejects active/navigation output before each patch,
and disables ambient local-storage and location resources. This option alone is
not a sandbox: use the shared mount, which establishes the complete boundary.

Readiness waits for the current mount's bootstrap document, identified by a fresh
internal marker. Initial `about:blank` and prior-navigation load events cannot
start native owners or acknowledge a preview. Recovery membership stays pending
until that document loads. Cancellation removes the waiter; a late load cannot
revive a disposed mount. The marker correlates document loading and grants no
resource or recovery authority.

Replacement/disposal cancels pending mounting, detaches resize observation, removes
produced instances, disposes their scope and clears resource ownership. Form state
is temporary; theme CSS must be supplied explicitly. Realm-aware input bindings use
the control's owning window. The shared tests cover UTF-8 resources, denied output,
interaction, replacement, independent frames and storage/location isolation.

### Immutable artifact-tree previews

`registerCemArtifactTree([{ path, contentType, bytes }])` returns a realm-local
`{ resourceSet, dispose }` registration. Pass its small `resourceSet` string and an
`artifact-path` to declarative `cem-media-preview mode="artifact-tree"`. Exact
bytes are cloned once into a bounded complete set (128 files / 16 MiB); they never
need to cross the UI's data projection. Release the registration when its run is
replaced. Every mount and generated object URL associated with it is revoked.

The shared `live-preview` capability also owns this mode's lifecycle, while
`mountCemArtifactPreview(frame, { resourceSet, path, onNavigate?, onStatus? })` is
the underlying browser API. Its controller exposes `ready`, `dispose` and
`setLocation(path)` for query/fragment updates within the mounted file. Relative
HTML links, directory index links, stylesheets/imports, inline CSS and images use
only this immutable set. Native retained CSS import owns resource decoding and
rewriting; the browser host owns inert HTML admission, object URLs and navigation.
Missing assets and denied references report bounded status without ambient fetch.

#### Inline SVG and MathML artifacts

Inline SVG and Presentation MathML in HTML/XHTML artifacts retain their browser
namespaces under a separate passive preview admission policy. SVG admits `svg`,
`g`, `title`, `desc`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`,
`path`, `text` and `tspan`. MathML admits `math`, `mrow`, `mi`, `mo`, `mn`,
`mspace`, `mfrac`, `msup`, `msub`, `msubsup`, `msqrt`, `mroot`, `mover`, `munder`
and `munderover`. The selected artifact still declares HTML or XHTML; this does
not relabel a standalone SVG or MathML resource as HTML.

The bounded foreign-attribute lists retain accessibility names/descriptions,
matching `xmlns`, literal geometry/text/paint attributes and the supported math
layout attributes. Paint accepts named/system colors, hexadecimal and numeric
RGB/HSL forms, `none` and `currentColor`; URL paint servers, CSS variables and
escaped values are excluded. Foreign `style`, event handlers, namespaced URL
attributes and other resource/navigation attributes are removed. SVG
`foreignObject`, scripts, animation, images, `use`, links, MathML annotations,
Content MathML and unknown elements are removed with bounded status messages.
Math tokens and SVG titles/descriptions admit plain text only: an HTML or other
namespace child cannot use an integration point to acquire HTML asset admission.
Foreign namespace changes and mismatched namespace declarations are refused.

This DOM admission is independent of native SVG/MathML parsing and validation.
The browser renders admitted geometry and presentation math; the preview does
not implement either source grammar, complete vocabulary or expression semantics.
Exact artifact bytes and native provenance remain available even when part of
their content is outside the preview subset.

No CEM runtime or authored scripts run inside this HTML output frame. The parent
can manage its document through the same-origin sandbox permission; forms, frames,
script execution and network sources remain denied. Authored URLs are admitted
before publication and rewritten to the mount's object URLs. The containing
application's CSP must also admit `blob:` in `img-src` and `style-src-elem`; a
child frame cannot relax a stricter inherited response policy. This does not
require blob script or worker execution. Parent inline-style-attribute policy
continues to apply (Studio keeps those attributes denied). The full supported
asset/content policy and small literal-envelope examples are documented in the
[component preview contract](../cem-components/docs/passive-payload-contract.md#explicit-output-preview-modes).

Links emit `cem-preview-navigate` with an own scalar `detail.value` containing the
path and optional query/fragment. Bind `host-command-event="cem-preview-navigate"`
on the preview host to let a routing service choose the next artifact. The shared
host-view bridge accepts only that owner's event, reads no accessors or inherited
detail, and forwards no other payload. It does not change editor/project selection.


### Guarded retained rendering

Shared WASM/worker processing can reuse a pure native render body for unchanged,
fully owned JSON inputs. Rust establishes input equality from actual parsed data;
unchanged browser revision strings alone never qualify. Worker and fallback
requests pin their inputs before async dispatch and reject superseded or released
render owners. This is shared runtime functionality; components and Studio require
no JavaScript or authored cache hints.

Native reference/document/companion bindings, opaque computations and unsupported
native-valued output keep fresh evaluation. Every hit still projects native values
and relationships, publishes host attribute updates, lowers resource controls and
prepares DOM patches under current publication checks. Matching DOM retains user
edits, focus and selection through the existing patch path. Targeted input patches
also invalidate logical owner receipts across inert payload/template boundaries;
returning to an earlier body reconciles changed owned attributes. Events and commands
retain normal admission. The native cache stores bounded owned output bytes only,
clears on template disposal/runtime reset, and retains no DOM, native owners or
previous connection. Optional processing `renderReuse` counters are diagnostic;
they never authorize publication. See the [native contract](../cem_ql/README.md#shared-retained-wasm-processing).

### Credential controls during explicit checkpoint recovery

Entered passwords and password/one-time-code autocomplete controls block recovery
for the complete runtime group. Recovery never clears credentials or omits their
owners. After explicit clearing, the native-validated `credential-empty` control
kind preserves the empty control and other consented edits without exporting a
secret. Nonempty reset values/custom errors and file controls remain refused;
property-only changes still invalidate currentness. See the
[credential privacy policy](../../docs/committed-checkpoints.md#credential-privacy-policy-for-live-login-recovery).

### Advisory recovery observation

`runtime.observeRecoveryStatus()` (or an explicit `CemRecoveryStatusObserver`)
provides a frozen, bounded, metadata-only status stream for declarative consumers.
It does not inspect credentials, run native work, capture or authorize recovery.
Dispose with the view; suspend publications across checkpoint quiescence and
release only after fresh owners settle. See the
[shared recovery-status contract](../cem-components/docs/recovery-status-contract.md).

### Optional native schema source loading

`CemSchemaAssetPackLoader` is a shared non-UI byte service for the opt-in native
schema-asset profile. It uses `openCemHttpResource`, the HTTP transport also used
by declarative resources. Custom resolvers/openers use the existing
`CemResourceResolutionRequest` / `CemHttpRequest` contracts. Source interpretation
stays in native code; JSON is used only for named catalog, import receipt and
package-status control records.

```ts
import { CemSchemaAssetPackLoader } from '@epa-wg/cem-elements/schema-assets';

const loader = new CemSchemaAssetPackLoader({
    baseUrl: new URL('./schema-assets/', deploymentUrl).href,
    runtime: { native: optionalNativeRuntime, current: () => owner.isCurrent() },
    loadHttpResource: host.loadHttpResource,
});
const source = await loader.loadSource({
    packageId: 'csv', version: '1.0.0',
    path: 'schema-packages/csv/v1/formatters/compact.cemt',
}, signal);
// source.bytes is an independent original-byte copy; sourceUri is canonical.
```

Initialize the optional native heap first. The loader authenticates `index.json`
against that heap's compiled pin before reading its catalog DTO. Native admission
and import run on every source access, including cache hits. Missing, offline,
tampered, incorrectly typed or oversized responses refuse without embedded
fallback. Digest-named static sources may use `application/octet-stream` as
transport MIME; the authenticated native catalog owns their semantic MIME.
Default transport omits credentials, denies redirects and cross-origin resolution,
and enforces a 30-second deadline. Hosts can allow cross-origin resolution or
lower the deadline, entry bound and aggregate capacity explicitly.

The service permits at most eight simultaneous loads, a 4 MiB index, 16 MiB
source, 65,536 chunks per response and 32 MiB retained/pending byte capacity.
Caller-owned result copies and native parsing allocations are separate from this transport capacity; native
memory admission remains mandatory. Abort/dispose fences late completions and
closes response iterators without waiting for uncooperative return handlers.
`dispose()` drops retained bytes, native callbacks and the memory identity, and
releases its persistent native owner if activation was requested.

#### Persistent activation and declarative demand

Supply `activationHostConfiguration` explicitly to enable `activatePackages`.
This is trusted embedding setup using the existing native command-host
configuration contract, including exact package/origin/manifest replacement
grants. It must never be copied from an authored template, demand URL or project
resource. Omission keeps the service acquisition-only; `{}` supplies no grants.
For example, a host can grant replacement of the built-in CSV package with its
exact pinned `schema-packages/csv/v1/package.cem` manifest. Then
`await loader.activatePackages([{ packageId: 'csv', version: '1.0.0' }], signal)`
fetches all that package's original bodies and atomically activates the checked
native contract. Failed selections preserve the incumbent; repeated successful
demand is idempotent. Publication is serialized with at most eight queued demands.

Owned-pack admission validates manifest declarations and compiles the native
schema contract. It does not run development examples or eagerly compile every
formatter/colorizer; those examples are outside the emitted runtime pack. Full
package `validate`/`check` retains its resource checks, and execution still
requires the exact admitted artifact body with no embedded fallback. This is an
explicit validation phase, without filtering diagnostic results to obtain readiness.

Install `loadHttpResource: request => loader.loadResource(request)` on a shared
`CemElementRuntime` host. An ordinary declarative resource can demand that package:

```cem
{http-request @slice=csvPackage
    @url="./schema-assets/demand/csv/1.0.0"
    @content-type="application/json"}
```

The absolute URL must match the loader's configured `baseUrl` plus
`demand/<packageId>/<version>`. The endpoint accepts GET without query/fragment,
authenticates the package against the native-pinned index and returns the named
`cem-schema-package-status/1` JSON status with `state=ready`. Use the existing
resource's `loaded` state to gate dependent controls/configurations. Ordinary
URLs use the host transport. No new UI control or Studio-specific JavaScript is
needed; native validation, not the URL or the status record, grants execution.

The loader owns the sole persistent package context on its physical optional
WASM heap. Install it on the same owned runtime that will execute the demanded
commands or import resources; activation never silently retargets a worker or
fallback heap. Ordinary command-service contexts, resolver contexts and CSV
resource imports use that heap's published context. Import still produces the
retained typed CEM tree, with checked source diagnostics and existing limits.
Defaults retain their complete embedded catalog until explicitly opted in.
The `external-schema-assets` native profile omits original CSV, Markdown, SCSS,
XSLT and Relax NG bodies. Complete native identity metadata remains
available for discovery, but it cannot authorize execution: missing optional
sources refuse with `cem.schema_asset.missing` until the original heap has an
admitted package. Exact formatter/colorizer body requirements still apply.

For a host spanning several physical runtimes, use `CemSchemaAssetDemandGroup`.
Its `run(work, originalEndpoint)` enrolls each actual heap/worker once and waits
for retained demand before running a newly allocated client. Bind `loadResource`
as the declarative HTTP opener and route managed asynchronous native work
through the same group. At most 16 physical participants, 64 demands and 1024
queued jobs are retained; each loader still enforces its 32 MiB byte bound.
`cleanup(work)` preserves exact-owner cleanup after ordinary demand is fenced.
`assertReady()` lets synchronous adapters refuse pending package publication.
`status()` contains advisory state and counts, never native handles or grants.

For resource-driven services, the optional native
`schemaAssetDemandPlanV1(commandRequestJson)` resolves an already lowered
`CommandServiceRequestV1` through the native identity catalog and graph lowerer.
It returns only `{ kind: "cem-schema-asset-demand-plan/1", packages: [{ packageId,
version }] }`. No JavaScript extension map, source parsing, grant or native owner
is involved. Call `await group.prepare(plan, signal)` **outside** `group.run`,
then enter `run` for dependent work. The group snapshots at most 64 identities;
an empty plan fetches nothing, and committed demands are reused. Original-byte
acquisition and trusted native replacement grants remain authoritative.
Cancelling a command stops its waiter; publication already started belongs to
the group and finishes across the enrolled cohort before later work can run.
Real publication failures fence the group. Disposal cancels its lifetime.

`CemElementRuntime` accepts the trusted `processingNativeWork(host, work, cleanup)`
hook. Resolve `processingSchemaAssetBinding(host)` inside that hook's queued
operation, then use its original endpoint and participant; cleanup must avoid
opening/enrolling an endpoint after native admission closes. The hook and trusted
package configuration are immutable for an existing logical root and distinguish
worker pools. Live previews inherit both, including fresh processing hosts, while
their own HTTP resource restrictions remain in place. Independent groups cannot
silently share one worker's package owner.

Each endpoint supplies its recovery policy witness and original fresh-target
adapter (`cemSchemaAssetGenerationTarget` handles the built-in facades). Demand
cohorts require every original participant before cleanup. A changed membership,
entered-credential witness, copied participant, failed peer or incomplete binding
cannot publish a fresh cohort. Native byte preparation and host staging precede
one complete binding transaction; staged package owners are reused by fresh hosts.
Fresh replay imports each body once before activation, without a redundant parse
pass or a network fetch. The existing five-second group deadline remains intact.

`schema-package-owners` is a nondiscardable native inventory root. Raw admission
closure refuses new owners, imports and activation; exact-owner cleanup remains
available. A live owner blocks the existing empty-inventory retirement check.
In-flight commands retain their native activity and cloned context through host
callbacks. Neither status records nor source counts/bytes can authorize release
or generation publication. Bind `current()` to the host's generation and
admission/credential policy; a false fence refuses new demand.

`deploymentUrls()` returns the complete immutable offline delivery list, including
metadata sidecars. Those sidecars are inventories, not trust anchors.
`prepareFreshRuntime({ native, current }, signal)` admits the retained index and
reimports original bytes into a distinct initialized heap without network reads
or native-handle transfer. If packages were active, it compiles and activates
them again under the same explicit host grants. Call it after old work drains
and before dropping the old loader's bytes. The target's admission/currentness
and physical memory identity are checked at asynchronous/native boundaries;
old bytes can survive an old endpoint becoming stale. The returned loader does
not retire the old runtime, publish a generation, serialize credentials or grant
checkpoint authority. Both native owners remain inventory blockers until their
host explicitly releases them. Existing credential refusal and coordinated recovery gates still
apply; automatic recovery is unchanged.

`CemSchemaAssetPackRecoveryBinding` adds an original generation-resource adapter
and a stable `loadResource`/`loadSource` route. Register it on the same physical
participant as its loader. Its trusted `target` callback resolves the fresh
runtime's native binding and participant; both source and target identities must
match the original physical registration. `cemSchemaAssetRuntimeFromBinding`
adapts an owned host's restricted `nativeResourceBinding()` without exposing
arbitrary native exports.

Prepare the complete generation group and host resume tickets **before** closing
admission. The owned invocation host's `closeNativeAdmission(owners, timeout,
handoff)` checks its complete original embedding ownership and the current
checkpoint handoff before releasing package ownership. Only immutable replay
bytes remain outside WASM; old native/memory references are dropped. Native
closure still requires an empty resource inventory. Source changes, entered
credentials, unknown ownership or stale handoffs refuse this path.

`resumeNativeGenerationBindings(admission, tickets)` handles a complete group of
host bindings; mixed rendering owners use `resumeCemCheckpointOwners` with their
original rendering tickets. Package restoration joins that transaction before
any host route binds. It recompiles under the existing trusted grants, stages
all routes inactive, then binds the complete set. Failure or revocation closes
every fresh route and disposes its package owners. The legacy single invocation
`resume` refuses attached resources so it cannot omit their restoration.

Owned browser invocation and command-service worker transports are supported.
For a command worker, supply its original `client.packageResourceBinding()`
directly as the loader's `runtime`. The shared asynchronous endpoint checks the
actual Worker identity, carries bounded original index/source bytes, and keeps
all native owner IDs inside that worker. It accepts one loader per physical
worker; duplicate loaders refuse. `hostConfiguration` is snapshotted when the
worker starts and renewed by its original allocator. A loader's
`activationHostConfiguration` cannot override the worker's setup. Package
messages reject unknown operations/fields, shared mutable buffers, oversized
indices/sources/selections and aggregate messages above 32 MiB before posting.

Register `CemSchemaAssetPackRecoveryBinding` on the worker's original
`generationParticipant()`. Resolve each fresh target using its own
`packageResourceBinding()` and participant. The command worker's
`closeNativeAdmission(owners, timeout, handoff)` closes raw and managed admission,
awaits package cleanup under the original embedding/checkpoint witnesses, then
requires an empty native census. The original five-second checkpoint deadline
also covers cleanup; a failed, missing, stale or late acknowledgment cannot
retire the source. Restoration recompiles packages before the complete command
binding transaction publishes. Aborted or lost asynchronous activation outcomes
block replay until explicit disposal.

Rendering workers and explicitly owned fallback runtimes also support package
transport. For lower-level processing hosts, set trusted
`CemProcessingHostRuntimeOptions.packageHostConfiguration` to the native host
configuration JSON before constructing a worker. With `CemElementRuntime`, the
corresponding host option is `processingPackageHostConfiguration`; it is also
retained by `previewRuntimeOptions()`. The original string is snapshotted by the
worker transport and its fresh-generation allocator. Worker pools with different
configurations cannot share a physical worker. An existing logical root rejects
configuration changes. This does not activate a package.

For an owned fallback, select
`isolateCemFallbackGeneration({ packageHostConfiguration })` before initializing
any fallback native work. After `ready()`, the generation exposes
`packageResourceBinding()`. A legacy singleton refuses an owned binding instead
of borrowing another heap or inventing disposal authority. Both fresh rendering
worker and fallback facades expose their own original package binding.

`await processingSchemaAssetBinding(host)` returns `{ runtime, participant }` for
an original shared processing host. Supply these to `CemSchemaAssetPackLoader`
and `CemSchemaAssetPackRecoveryBinding` respectively. Only trusted shared hosts
are accepted; copied objects cannot select a heap. One loader owns that exact
Worker/Memory. An explicitly attached package route refuses implicit worker
failure fallback, since another heap would not own its package context.

Rendering retirement releases packages inside
`retireNativeRenderingGeneration` after checking the original closed drain,
complete client/source coverage, current handoff and, for owned fallback, original
complete embedding consent. Resource release and native retirement share the
existing five-second deadline and 4 MiB rendering handoff limit. The native
empty-inventory permit remains mandatory after engine cleanup. Ordinary package
calls participate in the original drain and invalidate stale coverage; cleanup
alone does not reopen admission or mutate the closed proof. Fresh package
restoration precedes complete rendering/host binding publication. A revoked
fresh gate blocks saved ordinary package aliases before physical disposal, while
exact cleanup remains available. Failure fences the source and preserves its
checkpoint instead of replaying effects.

Command and rendering transports vendor the same strict original-byte/control
validator, including backing-buffer bounds. Native owner IDs and grants never
enter its demand, status or checkpoint DTOs. Explicit Studio-wide demand wiring
across the complete command/render recovery group remains a separate next step. A package activated on one heap never supplies another heap's
context. Ordinary reads retain their native admission policy; the recovery
binding's separate `current` fence blocks recovery while credentials are entered.
Automatic whole-Studio recovery remains disabled, and neither source bytes nor
native grants/handles are added to checkpoint/status records. The loader retains
its existing 32 MiB bound and normalized immutable transport policy.

`yarn nx run cem-elements:verify:schema-assets --parallel=1` prepares the explicit
optional profile and runs real native ABI integration tests plus an isolated
production-service-worker offline fixture. Ordinary unit tests do not require
that optional build.
