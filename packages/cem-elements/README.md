# @epa-wg/cem-elements

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

`@epa-wg/cem-elements` provides the `<cem-element>` browser substrate for declarative light-DOM custom elements. It is
the Phase 3.1 runtime gate before the project moves into Edge/SSR support and later `@epa-wg/custom-element` adoption.

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
dialog or tooltip popover to an eager semantic lifecycle. It supports persistent
nonmodal dialogs, native modal entry, popover visibility, cancelable close
requests, context-key policies, typed focus/return/anchor/boundary relationships,
and invocation-captured pointer/selection geometry. Centered tasks need no anchor.
Existing owners can use `connectCemNativeSurface(owner, { host })`; its `open`
accepts a `CemSurfaceInvocation`, optionally captured with
`captureCemSurfaceInvocation(source, event, { selectionOwner })`. The context
stays a retained `NativeCemValue`; geometry never becomes a focus destination.
Tooltips preserve stable description tokens and input focus. Owners survive
rerender, while disconnect cleans sessions and resources. Authors supply names
on native owners. Component migration and lazy body preparation remain separate;
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
loaders must repeat the capability on their loading declaration.

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
Source/template edits invalidate its leases. The production `cem-suggestions`
XHTML declaration and linked playground/gallery use this first local profile.
Full touch/pen, device IME, screen-reader and source-example parity verification
remain in the active checklist.

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
slot, FIFO ordering per root, and round-robin cross-root dispatch. Compiled artifacts and
render plans use bounded content-addressed LRU retention; an evicted previous plan safely
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

Declarative event bindings read a live string `value` from native inputs,
textareas, selects and custom controls whose constructor declares
`static formAssociated = true`. This applies to `$target.value`, value aliases,
the default slice value and serialized event targets. It lets a parent bind a
`cem-select` change directly to its own slice. Non-string custom values are not
serialized as event target values; ordinary elements retain attribute fallback
only for default values and aliases.
