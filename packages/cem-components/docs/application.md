# Virtual applications and orchestration

**Status:** design proposal, 2026-11-23. This document proposes
`cem-application`; it does not introduce a shipped component, capability, or
binding syntax. Read it with [command.md](command.md) and
[localization.md](localization.md). Existing accepted interaction and lifecycle
contracts remain authoritative until an explicit implementation revision adopts
these additions.

**Owner revision — virtual applications:** `cem-application` is an embeddable
microapplication boundary, with a native CEM context, downward container
authority, application management and portable-state goals. This revision
supersedes the earlier description as only a command/localization region.
Focus-to-activation policy remains **TBD**; the contracts below do not select it.

## Purpose

`cem-application` represents one virtual application. The top-level instance
matches the page; embedded instances represent microapplications that can inherit
the container context or run with their own context, storage and logical URL.
The container can override an embedded app's context; an embedded app cannot
override its container or siblings. Inheritance is a choice of inputs, never
upward write authority.

Each instance also coordinates resource-backed commands and localization. A
menu item, toolbar action, command palette entry, keyboard
shortcut, or native-shell menu invokes the same stable command ID. The
application resolves a contextual handler, collects missing arguments when
necessary, and records a correlated outcome. A locale change replaces resource
presentations without changing command identity or executing commands again.

Application management follows MDI-style separation between a container manager
and managed children; it does not require draggable desktop windows. The virtual
application remains distinct from the existing compiled
`cem-application-declaration/1` artifact. It reuses native CEM context, resource
and checkpoint semantics instead of inventing a parallel application engine.

## Insulation and browser boundaries

The intended iframe analogy is independent application context and lifecycle.
Within admitted declarative CEM execution, an insulated app has private service
bindings and state and cannot modify its parent's context. Light-DOM composition
does not create a new browser document, JavaScript global, origin, custom-element
registry or security sandbox. Arbitrary same-document JavaScript and unmanaged
global CSS remain outside that virtual insulation guarantee.

Untrusted executable apps require an actual sandboxed browsing context/origin or
an equivalent host isolation adapter. That adapter can expose the same CEM app
management protocol. A real iframe's browser permissions, focus and accessibility
boundaries must still be respected; it is not implemented by a wrapper attribute.
[HTML iframe model](https://html.spec.whatwg.org/multipage/iframe-embed-object.html#the-iframe-element).

Even same-origin iframes do not inherently provide private local storage.
"Own storage" below means an application-scoped broker/provider, not a claim
that a custom-element subtree changes browser storage ownership.
[HTML Web Storage](https://html.spec.whatwg.org/multipage/webstorage.html).

## Native context and downward authority

An application's effective context is a native CEM context frame, represented at
the CEM AST/artifact boundary. CLI, native, WASM, browser and SSR must resolve
equivalent inputs identically. Application instance identity associates a frame
with its owner; it never replaces the native scope identity or authored source
bindings. See [context propagation](../../../docs/cem-ml-resource-lifecycle.md#context-tree-propagation)
and [native namespace scopes](../../../docs/cem-ml-stack-design.md).

| Context family | Inherited or app-owned inputs | Container authority |
| --- | --- | --- |
| Language/schema | Namespaces, default namespace, schema/content-type bindings, registered plugins and version pins | Supply child bindings/overrides within host registration and policy bounds |
| Resolution | Module/resource maps, source and resolver bases, accepted types, integrity and cache policy | Override permitted mappings or block them; no child can relax ancestor restrictions |
| Resources | Command definitions, message catalogs, assets, shortcut profiles and other typed values | Replace the entire resource set or specific declared values/fields |
| Environment | Requested languages, theme variations, formatting and accessibility preferences | Supply child settings/overrides; child settings affect only that child/subtree |
| Services | Storage, logical location/history, clipboard/network/host endpoints | Grant private, inherited or explicitly shared providers; revoke/rebind under a new context revision |
| Runtime | Budgets, lifecycle admission, scheduling priority, checkpoint/export policy and diagnostic boundaries | Manage descendants within enclosing host limits |

Two context composition modes are proposed:

- **`inherit`**, the default for embedded apps: use the container's admitted
  context inputs and resource mappings, augmented by app-local values where the
  relevant native lookup contract allows. Local state remains instance-owned.
  Inherited storage/URL providers may be shared, so writes can have explicitly
  shared effects; sharing must be visible in the effective context.
- **`insulated`**: start from the app's declared context/defaults and private
  storage/location providers; do not fall through to ambient parent resource or
  service values. Standard browser/environment preferences remain available
  through an admitted read-only provider. Ancestor policy ceilings and explicit
  container overrides still apply. Insulation cannot escape host authority.

Allow selected families to be explicitly inherited, private or supplied by the
container in either mode. A binding to a shared store is a deliberate exception
to private storage, not accidental fallback. A child can propose a different
local module map/namespace set for its own frame; it cannot rewrite any ancestor
frame. The parent can approve, replace or restrict that proposal.

Container overrides are a separately owned input to the child frame. Their
provenance records the container and targeted child instance. A child cannot
forge that input or relabel its own values as container policy. Ancestors can
constrain or override a descendant's effective inputs; an intermediate app
cannot cancel an enclosing mandatory restriction. Parent ownership does not
automatically issue cross-scope node or DOM-placement grants: keep
[directed source/placement admission](../../../docs/cem-element-granted-placements-design.md).

### Resolution and resource overrides

Use existing native lookup rules, not a universal new merge algorithm. Non-CSS
module resolution already lets outer mappings override inner defaults. CSS
resolution intentionally selects the closest applicable mapping, while retaining
all ancestor blocks and resource restrictions. App insulation controls which
default frames/providers are supplied; it does not reverse these resolver rules.
To enforce a container-selected CSS resource, construct the admitted child map
accordingly or reject incompatible configuration; do not patch a browser import
map or silently change native precedence.
[Scoped map contracts](../../../docs/scoped-css-module-maps.md#existing-boundaries).

Namespace changes remain lexical and downward: they affect subsequent uses in
their scope, not expanded names already resolved or bindings in a parent. A
container namespace override seeds/validates the child's context before
evaluation. A required binding conflicts with a disallowed child redeclaration
by producing a diagnostic, not by retroactively reinterpreting retained nodes.

Application resources follow this same context pattern. Container-owned
overrides may replace all resources, one resource, or schema-declared fields,
for example only a command's name and help. Missing fields use the admitted
underlying resource; explicit empty/removal values follow that field's schema.
Whole-message/plural units and executable/schema-bearing values are atomic unless
their schema explicitly defines a safe field overlay. Revalidate the effective
resource before publishing it. Semantic overrides use admitted context authority;
a translator bundle still cannot replace a handler or schema.

Resolve semantic definitions/overrides before localization. A presentation-only
override leaves the command's semantic revision unchanged; replacing its schema
or endpoint creates a new semantic revision and fences pending invocations.
Keep source and override provenance, context/grant identity and all relevant
revisions in cache/publication keys. Child-local edits never mutate a shared
parent tree. Use native retained values and references, not a deep-merged
JavaScript catalog or a second format-specific evaluator.

Per-app CEM module maps are native resolver state. Browser JavaScript import maps
operate at the document/global boundary with their own merge constraints; they
are not a mechanism for installing arbitrary subtree environments.
[HTML import maps](https://html.spec.whatwg.org/multipage/webappapis.html#import-maps).

### Storage and URL providers

The top-level app normally binds its location provider to page navigation and
its storage provider to a page/application partition. An insulated child gets
an independent logical URL/history and stable storage partition. An inherited
child can share a provider or receive a scoped projection supplied by the parent.
Make those relationships explicit rather than sharing through DOM ancestry.

Keep source-document URI, resource resolver base and logical app route URL
distinct. Child navigation changes its own logical location; only the container
decides whether/how that location projects into the page URL/history, including
Back/Forward. Shared navigation requests still pass the container's coordinator;
children do not directly write `window.location` or global history.
[HTML location/history](https://html.spec.whatwg.org/multipage/nav-history-apis.html).

Every CEM-managed storage/location reader and writer must resolve through the
effective provider, including subscriptions, forms, resource loaders and host
services. Missing or denied private providers fail explicitly, without falling
back to ambient origin storage/page location. Storage partitions are keyed by
stable provider/instance identity, never a mutable instance name or title.
Cookies, origin-level IndexedDB/network credentials and arbitrary third-party
JavaScript do not become partitioned just by naming an app. Their isolation
requires an appropriate host adapter or a stricter execution boundary.

These generic provider bindings are new work. Current document-global
storage/location behavior cannot be described as already insulated.

## Ownership

| Owner | Responsibility |
| --- | --- |
| `cem-application` declaration in `cem-components` | Declarative composition, resource/context bindings and application status projection; no production JavaScript |
| Generic capabilities in `cem-elements` | Native-context binding, application manager, service providers, registry subscription, command routing, invocation correlation, shortcut matching, parameter-form coordination and teardown |
| Native CEM-ML/import/resource layers | Typed resource trees, resource references, schema validation, locale selection inputs, provenance and lifecycle revisions |
| Command handler component | Context-specific eligibility, execution and result; stable domain target identity |
| Native `dialog` through `cem-dialog` | Modality, form controls, validation, focus and surface lifecycle |
| Host service adapter | Clipboard, file picker, persistence, native shell notifications and other admitted non-UI services |
| Application data | Document contents, selection, dirty state, authorization and undo history |
| Container application manager | Child registry, admitted downward overrides, lifecycle gates, active-child reference, scheduling and save/load/clone/offload transactions |

Follow the [declarative UI principle](../../../docs/declarative-ui-principle.md):
missing generic capabilities must be added to `cem-elements` before component
UI work. The runtime must use generic provider contracts, without branching on
`cem-application` or editor tag names. App-owned scripts may implement non-UI
services; they must not manufacture visible UI, attach local keyboard handlers,
or implement a competing dialog controller.

## Proposed application surface

These names are design inputs, not current attributes. Structured inputs are
native CEM values/references, never JSON text hidden in an attribute.

| Input | Meaning |
| --- | --- |
| Application definition reference | Stable component/application resource identity and declared native context/defaults; not an instance name |
| `context-mode` | Proposed `inherit` or `insulated`; per-family provider bindings can refine it |
| Native context bindings | Typed module maps, namespace/schema frames, services and policy; use native context vocabulary rather than new string-encoded maps |
| Container-owned context overrides | Admitted parent input targeting this instance; replace all or selected context/resource values; not child-writable policy |
| `instance-name` | Optional manager-unique stable readable name; generated on first admission if omitted |
| `resources` | Typed reference to an admitted application resource set containing semantic command definitions and locale manifests |
| `locale` | `auto` by default, or an explicit BCP 47 UI language tag; not an operating-system locale setter |
| `fallback-locale` | Application default language, required to exist in the resource manifest |
| `format-locale` | Optional separate date/number formatting preference; defaults to the resolved UI locale |
| `shortcut-profile` | Admitted platform profile; host-provided when available, otherwise an explicit web profile |
| Theme preference | Automatic environment matching or explicit app setting through existing CEM theme vocabulary; no new token family |
| Storage/location bindings | Typed inherited, private or container-supplied providers; logical URL is not necessarily the address bar |
| `command-context` | Optional typed reference to the initial content context; otherwise learned from focus |
| Default slot | Application contents, retaining native landmarks and component ownership |

Derived state includes requested languages, effective UI locale, format locale,
direction, context/resource/shortcut generations, lifecycle state, scheduling
class, instance identity, manager-selected active-child reference, content
context and bounded invocation summaries. Publish it in the application's durable data
island through ordinary native resource/slice/event channels. It is queryable
state, not a second JavaScript document model or a new island topology.

`cem-application` does **not** automatically set `role="application"`. Keep
normal document navigation and native landmarks. An application name can label
an appropriate region without forcing assistive technologies into a special
keyboard mode.

## Application boundary and lifecycle

The nearest application owns an invocation and its contextual state. Nested apps
can inherit resource/context inputs, but remain distinct execution and management
owners. Raw key/command execution stops at the app boundary; no unhandled child
command automatically executes in the container. Explicit export/service bridges
are described in [command.md](command.md#commands-across-application-boundaries).
Resource inheritance alone grants neither execution nor upward context writes.

1. On first admission, allocate stable instance identity/name and register with
   the parent manager (or page host for the root). Construct the effective native
   context/providers under existing scopes/grants. Validate command IDs and
   default locale/theme coverage before starting the app.
2. Materialize the typed definitions and a coherent presentation generation.
   Until ready, resource-bound controls are unavailable and no shortcut is
   intercepted. Normal native links, controls and browser editing still work.
3. Register generic handler/context providers and subscribe to their state.
   Multiple instances of an editor may handle the same command ID.
4. Register one coordinated dispatch boundary through the shared runtime.
   Management, business-command and lifecycle admission are separate. Key
   dispatch for a running app follows the eventual focus/activation policy;
   do not add a listener per menu item or infer activation from connection.
5. On context, resource, permission or host-profile changes, invalidate only the
   affected eligibility/presentation snapshots. Unrelated renders do not reset
   focus or drafts.
6. On disconnection, release listeners, provider registrations, native value
   owners and observers; abort pending preparation and dialogs. Fence late
   results from publishing to a replacement application instance. An already
   committed host side effect cannot be rolled back merely by disconnecting.

Resource loading uses the
[common lifecycle](../../../docs/cem-ml-resource-lifecycle.md), including failed
and superseded revisions. No second loading state machine is introduced for
locale bundles. The
[durable data-island contract](../../../docs/cem-element-lifecycle-principle.md)
continues to own rendering, resume and native value retention.

## Focus and command context

Maintain separate **DOM focus**, **content command context**, **invoker**, and
**return-focus destination**. They are frequently different elements.

**Cross-app focus and activation selection are TBD.** The following rules apply
within an already identified running app. They do not establish that focusing,
hovering, restoring or rendering an app automatically activates it.

When an editor owns focus, it becomes that app's content command context. Before focus
enters command chrome, such as a toolbar, menu or command palette, capture that
context in the interaction session. Opening a parameter dialog retains the
same target and invocation ID. Focusing the dialog must not replace the pending
Save As target with the dialog's form or an unrelated editor.

A context-menu session captures the context under the pointer or keyboard
invocation, which may differ from the last focused editor. That explicit context
has priority for that session. An explicitly supplied `command-context` uses the
existing typed reference/placement validation; it is not a CSS selector. Add it
to the reference consumer only as part of the future implementation.

For an ordinary shortcut, use current content focus. For command chrome, use its
captured content origin. For an application-scoped command with no content
origin, the application itself is the route origin. A focused native text field
within a parameter dialog gets normal editing commands; only the already
pending invocation retains its earlier target. Background application commands
are blocked during a modal task unless explicitly classified as modal-safe.

Provider identity and domain context are retained, rather than a translated
label, list index, or arbitrary descendant of the generated DOM. Validate that
the target remains connected, admitted and bound to the same document before
execution. Target loss cancels the request; it never silently saves whichever
document later receives focus. Cross-document/iframe routing requires an
explicit host bridge; DOM bubbling cannot cross a browsing context.

## Application manager and MDI conventions

Each container manages its direct children through a registry containing stable
instance IDs, names/titles, definition/context revisions, lifecycle state,
capabilities, dirty/checkpoint status and an optional `activeInstanceId`.
The page host manages the root app. A child cannot enumerate or manage unrelated
apps merely because their DOM nodes are visible. An ancestor delegates management
through admitted endpoints rather than treating a descendant ID as authority.

Borrow the MDI distinction between a frame manager and an active child, not the
assumption that each app is a native window. A manager has at most one selected
active direct child; nested managers can form an active ancestry path. Selection
does not imply exclusive visibility or exclusive execution: other running apps
remain lower priority. The manager/chrome remains able to start or close an app
whose own processing is stopped.
[Windows MDI](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-the-multiple-document-interface).

| Proposed management command | Meaning |
| --- | --- |
| `app.activate` | Select an admitted child by instance ID; start/load it first when necessary, then commit activation only on success |
| `app.next`, `app.previous` | Select the next/previous eligible direct child in the manager's declared stable order; skip closed/unavailable children and do nothing if none exists |
| `app.start` | Admit/start or resume a selected instance without implicitly replaying commands or claiming keyboard focus |
| `app.pause` | Request a coherent stop of business processing; managed transition, not immediate listener removal |
| `app.close` | Close the target instance with declared dirty-state/side-effect policy; close is not deletion of its saved state/storage |
| `app.save`, `app.load`, `app.clone`, `app.offload` | Explicit state-management transactions described below; not aliases for document Save As |

Management targets use stable instance IDs; names are an optional validated
manager-local lookup convenience. `app.next`/`app.previous` act on the manager's
registry order, not DOM query order, translated titles or a changing render list.
Whether that order is tab order, MRU or another explicit model remains an
interaction decision. No default OS hotkeys are claimed by these command names.
The manager localizes names/help through ordinary command resources.

**Still TBD:** focus-to-activation rules, keyboard ownership when focus and
selection differ, initial active-child choice, activation during modal tasks,
close fallback/MRU policy, and return-focus behavior between apps. Explicit
activation is an admitted manager operation; it is not proof that a focus policy
has been designed. Active state is a selection fact, not a command grant or a
DOM accessibility role.

## Lifecycle, priority and stopped-event admission

Separate execution state, manager-selected active state, visibility and network
connectivity. In this proposal, lifecycle **`offline`** means deliberately taken
out of application event processing. It does not mean that browser networking
is unavailable. Keep independent connectivity status; losing the network must
not suspend a usable local/cached app or be inferred solely from
`navigator.onLine`. [HTML browser state](https://html.spec.whatwg.org/dev/system-state.html#browser-state).

| State | App business processing | Priority and allowed management |
| --- | --- | --- |
| Starting/restoring | Gated until context, resources and state are admitted | Manager handles start/activation/close; failed preparation preserves the prior usable state |
| Running, selected active | Normal admitted events/commands | Prioritize interactive work; keep enclosing managers and required dependency work responsive |
| Running, other | Normal admitted work within budget | Low/background priority; no global shortcut/active-child entitlement |
| Pausing/stopping | Close admission, quiesce in-flight work | Manager/control plane only until the stop fence succeeds |
| Paused | No business input, timers, callbacks, commands or renders | Activation/start/close enter through the manager, not child event handlers |
| Offline | Same no-business-processing gate; deliberately unavailable to ordinary dispatch | Activation/start/close only through the manager |
| Offloaded | No live executable owners; portable state remains with the host/store | Activation/start loads under fresh admission; close can act without loading |
| Closed | Not a live managed execution target | Saved records can be loaded as an explicit management operation |

The parent manager is outside the stopped child's admission domain. Its
activation/start/close controls remain reachable outside the inert/unmounted
child content. Save/export of an already sealed snapshot can be host-side
bookkeeping, but must not secretly deliver a Save event into a stopped app.
Other management operations first transition the app through an allowed state
or return an explicit unavailable result.

Effective business admission also requires every enclosing app to be running.
Stopping a container gates its whole subtree; a child cannot start itself through
an ancestor stop. Keep child desired states separately so the manager can
reconcile them on restart without claiming they ran while stopped. The surviving
host control plane owns activation/start/close for a stopped subtree, including
when its immediate manager's business UI is paused. Stopped instances are never
effective active execution targets; manager selection changes must be coordinated
with lifecycle admission, without deciding the TBD focus/fallback policy.

Stopping is a transaction, not just `inert` or hidden markup:

1. Preflight modal/dirty/unknown-effect prerequisites while the app is still
   running and its owned prompt can accept input/submit. A required app-owned
   prompt cannot be deferred until after its event processing is disabled.
   Manager-owned prompts are a separate surface. Do not report the app stopped
   during this preflight; later input can invalidate its decisions.
2. Recheck document/context/owner revisions and the accepted decisions, then
   atomically close new business admission, advance the app's admission epoch,
   and invalidate preparation/publication tokens. If state changed during
   preflight, reprepare or refuse instead of discarding newer work.
3. Park/cancel/drain admitted native operations at coherent boundaries using
   [operation control](../../../docs/cem-ml-operation-control-design.md).
   Cancellation/unwind takes precedence over pause. Do not report `paused`
   while unaccounted app work is still executing.
4. Detach or suspend subscriptions, timers, animations, fetch/worker result
   delivery, storage/navigation notifications, shortcut routing and event-slice
   capture. Account for any external effect that cannot be interrupted; wait,
   refuse the transition or retain an explicit unknown outcome.
5. Publish the stopped state and, if retained visibly, suppress interaction and
   close/reconcile native top-layer surfaces. Native `inert` can supplement UI
   suppression but cannot suspend callbacks or serve as the execution gate.
   [HTML inert subtrees](https://html.spec.whatwg.org/multipage/interaction.html#inert-subtrees).

The gate covers every CEM-controlled entry point, including handlers that would
otherwise capture into a data island. A raw DOM event may still traverse the
shared document; that does not admit it into a stopped app. The host can still
cancel work, release resources, record side-effect journal outcomes, discard
stale callbacks and keep latest dependency/environment revisions. This control
bookkeeping never invokes app business logic or projects a new child render.

Do not buffer/replay missed clicks, shortcuts, commands, timer ticks or resource
callbacks. On activation/start, re-admit the current context and latest
locale/theme/provider revisions, reconcile external state and refresh
subscriptions before opening the business gate. Passive invalidations coalesce
as a latest revision/dirty marker in the manager, not an unbounded event queue.
Use a monotonic **application admission epoch**, independent of runtime generation,
advanced on stop, restart, context/provider rebind and disposal. Pause/resume can
retain a worker, so runtime generation alone cannot fence pre-stop callbacks.
Every callback, command, bridge and publication token captures instance ID,
runtime generation and admission epoch; check them plus app/ancestor running
state immediately before execution/publication. Retained parked work needs
explicit re-admission under the current epoch; never revive an old callback token.

Priority is cooperative. Active interactive work is scheduled ahead of other
running app work; bounded quotas and yielding prevent starvation. Native result
bytes/order must not depend on scheduling priority. Browser task scheduling is
an optional host optimization; it cannot preempt an already running callback or
control every event source. Feature-detect scheduling APIs, use the runtime's
bounded cooperative queue on unsupported hosts, and do not claim equivalent
browser priority control from a timer fallback.
[Prioritized Task Scheduling](https://wicg.github.io/scheduling-apis/).

Visibility may lower optional rendering urgency but never defines activation,
pause or offline state. `content-visibility`/`IntersectionObserver` can optimize
work without changing event authority. Browser page freeze/discard is a separate
outer lifecycle: keep checkpoints current before it happens; a discarded page
cannot be relied on to run app save/close callbacks.
[Page Lifecycle](https://developer.chrome.com/docs/web-platform/page-lifecycle-api).

## Instance identity, save, load, clone and offload

Allocate an opaque immutable **instance ID** and an associated readable
**instance name** immediately at first admitted creation, before child resources,
events or storage bindings can be published. If no name is supplied, generate it
from the application definition/component name plus a manager-unique suffix,
for example `notes-7k3p`. Persist that allocated name; do not regenerate it on
every render, activation or resume.

| Identity | Lifecycle |
| --- | --- |
| Application definition/resource ID | Stable identity of the component/application code and native context defaults |
| Instance ID | Stable logical application instance; management, private storage and save/load identity |
| Instance name | Readable manager-local identity/lookup alias, generated by default or explicitly supplied; may be renamed atomically |
| Localized title | User-facing label, independent of lookup/storage identity |
| Runtime generation and native context/scope IDs | Fresh admitted execution/context identity; not a substitute for instance ID and not portable authority |
| Admission epoch | Monotonic app event/work-admission revision, including stop/restart on the same runtime; never restored as executable authority |
| Saved artifact ID/revision | Identity/integrity of a particular portable state snapshot, independent of instance display name |

Names are validated and unique among registered/reserved instances in the
manager's scope. Explicit name collisions fail; generated names retry allocation.
Rename changes metadata/aliases, not stable consumers, storage partitions or
resource IDs. Moving an instance between managers requires explicit admission
and collision handling; changing its DOM parent is not a context/ownership
transfer protocol.

Portable application state is a new native CEM schema/codec contract, separate
from a page screenshot, rendered HTML export or a JavaScript object dump. It
records definition/version and resource revisions, logical instance identity/name,
admitted context intent, canonical app-owned data/islands and permitted drafts,
logical location/history, storage snapshot or provider reference under an explicit
export policy, child-instance records and versioned migration requirements.
An inherited/shared provider is a dependency/reference unless a separately
authorized export permits its contents; saving a child cannot capture its parent
or siblings incidentally. Child inclusion is explicit and bounded.

| Operation | Identity and state rule |
| --- | --- |
| Save | Produce a versioned complete portable snapshot under explicit consent; keep the live instance unless offloading is separately requested |
| Load/resume | Reconstitute the saved logical instance/name only after ownership, collision, schema/version and context admission; reject duplicate live IDs or explicitly replace the named existing instance transactionally |
| Clone | Create a new instance ID, generated name and private storage/location state seeded from exportable data; preserve source-instance provenance only as data |
| Offload | Quiesce, seal and safely hand state to a surviving host/store, then release live DOM/runtime/resource owners |
| Close | Resolve dirty-state/effect policy, unregister and dispose; retain or delete saved state/storage only under an explicit separate policy |

Default cloning copies app-owned mutable state into fresh private providers;
shared services/references require explicit new grants. A clone receives a fresh
runtime generation, context admission, controls, subscriptions and operation
namespace. It never copies live promises, browser activation, permissions,
source/placement grants, listeners or a pending command as executable work.
Unresolved side-effect outcomes block clone/resume until reconciled; terminal
journal entries can remain non-executable provenance. A migrated/cloned native
state requires a supported semantic re-instantiation path, not rewriting IDs in
a sealed checkpoint or reparsing rendered DOM.

Use [committed checkpoint principles](../../../docs/committed-checkpoints.md)
for coherent publication, owner coverage, privacy and operation journals. The
current committed-checkpoint store is private host memory, not automatic durable
storage, and some resume paths require original live owners/witnesses. It is not
already a general app unload/load/clone API. Define explicit artifact admission,
semantic migration and durable storage/export adapters before claiming those
operations. Prefer native CEM-ML/typed artifacts under the common import boundary;
JSON/CSV/binary transports remain named, validated boundaries.

Offload must first obtain an acknowledged, current, all-owner checkpoint for the
affected execution domain. Retire the old generation only after publication and
durable/surviving-host handoff succeeds. Failure preserves the live instance or
last acknowledged paused state and reports a result; never drop the only current
state to satisfy a memory target. Credentials/OTPs, unsupported file controls,
opaque live state and unknown effects retain their existing refusal rules.

Do not assume one app equals one WASM heap/worker. An app sharing a physical
runtime cannot independently free that runtime while other owners remain. Either
retain/refcount the shared runtime and release only supported app owners, or
coordinate checkpoint/retirement of the whole domain. Independent physical
offload requires a suitable execution-domain boundary; logical insulation alone
does not provide it. Resume reacquires resources/services/grants under the current
container policy, not saved historical authority. It never resumes side effects
or an old active-app/focus claim automatically.

## Invocation orchestration

```mermaid
sequenceDiagram
    participant I as Invoker or shortcut
    participant A as Application
    participant C as Focused context and ancestors
    participant D as Parameter dialog
    I->>A: Stable command ID and supplied parameters
    A->>C: Resolve handler and current state
    C-->>A: One eligible handler, or unavailable
    opt Required parameters are missing
        A->>D: Open modal form with retained invocation context
        D-->>A: Valid accepted parameters, or cancellation
    end
    A->>C: Revalidate and route cem-command
    C-->>A: Claim once and execute
    C-->>A: Correlated result
    A-->>I: Project outcome and refresh state
```

The full state machine, event contract, duplicate prevention and parameter rules
are in [command.md](command.md). The application orchestrates; it does not execute
every command in a document-global handler. A local editor can own Copy, its
document owner Save As, and the application Preferences.

Closing menus and opening a modal task must cooperate with the existing menu
task relay and [surface lifecycle](dialog-contract.md). Do not synthesize a
second click or native `command` to carry out the same operation. An accepted
parameter form produces one final execution request, not an opening request
followed by a second unrelated invocation.

## Locale changes as a transaction

`locale="auto"` and automatic UI theme selection follow the effective environment
provider: container preferences when inherited, otherwise browser/native-host
signals. Match only variations/locales actually available in the app; if none
matches, use the app's declared available default. Explicit app settings override
automatic selection for that subtree, subject to container-owned overrides and
ancestor restrictions. A child setting never changes the page/container setting.
Choose the highest-precedence applicable preference source first, then match its
choices. An unsupported explicit choice falls back to the app's declared default,
not a lower-precedence browser choice; an explicit exact-match requirement fails
instead. Language and theme use the same source-selection rule.

A native adapter can report OS/application language changes; the browser exposes its own
language preferences and `languagechange`. A web application cannot promise to
observe every OS setting change immediately. Explicit app preferences take
precedence within the container's permitted context. See
[HTML language preferences](https://html.spec.whatwg.org/multipage/system-state.html#language-preferences).

Theme matching uses existing CEM theme/token contracts for light/dark,
native/high-contrast and supported variations. Preserve browser forced colors
and other accessibility requirements; a private theme setting is not permission
to disable them. Actual arbitrary stylesheet insulation requires the stronger
host boundary discussed above, not a parallel local token system.
[CSS color adjustment](https://www.w3.org/TR/css-color-adjust-1/).

Prepare a new resource generation off the active projection, including strings,
direction, mnemonics, platform shortcuts and accessibility strings. Validate it,
then publish it as one application state revision. CEM render scheduling may
update several DOM owners, but input dispatch must never use bindings from a
different active generation than the presented controls. Use a brief dispatch
barrier until affected projections acknowledge the revision; do not claim the
browser performs an atomic multi-node DOM mutation.

Failed or superseded preparation leaves the previous coherent generation
active. The first load needs a valid packaged default before commands become
available. Open forms keep their edited values, selection, native owner identity
and pending invocation. Relocalize labels and validation messages without
reopening the task or clearing user input. Resolve a new mnemonic map; retain
the dialog's entry and return-focus relationships.

Presentation may change while an invocation waits. Its semantic command/schema
revision remains pinned. If that semantic revision changes, revalidate or
cancel the pending invocation instead of interpreting old input under a new
schema. Locale changes alone neither cancel execution nor replay a result.

## Host adapters and activation

Keep native menus and browser menus on the same command registry and state-query
contract. A host-origin request carries an admitted application/context identity
and a correlation ID; it must not broadcast into every application in a window.
Host menu updates subscribe to the published locale and eligibility revisions.

The existing `host-command` bridge is a narrower scalar service boundary, not
this proposed routing protocol. Extend it explicitly or add a separately named
adapter for typed parameters; preserve its nested-view and ownership checks.
DOM event `isTrusted` is not an authorization token. A runtime-generated
`cem-command` is synthetic even when a genuine user gesture caused it.

Preserve existing host checkpoint effect accounting in opted-in domains: begin
the admitted operation before a side effect, acknowledge its actual outcome,
and retain unknown outcomes on disconnect/failure. A command invocation ID
correlates UI requests; it does not replace the host operation journal or make
an unacknowledged effect safe to repeat.

Activation-gated operations must reach the permitted host API synchronously
from the originating click/key event, or from the final dialog submit gesture.
Do not await bundle loading, a worker round trip or eligibility computation and
assume transient activation survives. Preload eligibility and schemas; the
browser/host still decides permission. A free-form file path cannot grant write
access: web Save As normally asks a picker for a file capability or offers a
download fallback. Cancellation remains distinct from an execution error.

## Missing capabilities and implementation order

This proposal requires new shared capabilities; none are delivered here.

1. Bind application frames to the native context tree and existing resolver,
   namespace, import and grant contracts. Specify inherited/insulated provider
   composition and downward overrides; add generic storage/location adapters.
2. Define instance/manager identity, lifecycle admission and operation-control
   integration, including ancestor stops and active/background scheduling.
   Resolve the explicitly TBD focus/activation UX before claiming keyboard flow.
3. Define command/resource/override vocabularies and schema revisions; implement
   locale/theme matching, typed parameters and cross-format import validation.
4. Add handler registrations, eligibility queries, local one-claim routing,
   explicit cross-app export/service bridges and lifecycle-fenced host adapters.
5. Add parameter-form sessions using existing dialog/form capabilities; verify
   final-submit activation. Add shortcut profiles and coherent resource/theme
   publication, including native-menu projection and stopped-app deferral.
6. Define portable application-state admission/migration and manager transactions
   for save/load/clone/offload. Prove all-owner/privacy/effect accounting and
   shared-runtime retirement before physical offload.
7. Only then add the declarative `cem-application` component and resource binding
   to actions/menu items, with colocated stories and property/gallery coverage.

Do not add per-component scripts to bypass a missing stage. Existing native
invoker behavior must remain independently usable throughout migration.

## Acceptance and limits

Future implementation acceptance requires:

- Two editors handling the same ID route by content context, including toolbar,
  menu, pointer context menu, keyboard and native-shell invocation.
- An unhandled inner-application command never executes in the outer app;
  disposal/rebinding prevents late execution and publication.
- Top-level/page and embedded inherited/insulated apps resolve native contexts
  consistently in CLI/WASM/browser/SSR. Partial/all resource overrides propagate
  down only, retaining native CSS/non-CSS and namespace behavior.
- Private storage and logical navigation never fall back to ambient providers;
  shared provider writes and page URL projections require explicit ownership.
- Parent-to-child commands, child service requests and MDI management target the
  correct instance; resource inheritance and active selection confer no grants.
- Active/background workloads obey budgets without starving control operations;
  paused/offline apps and their descendants capture/process no business events.
  Restart uses latest revisions without replay; unsupported scheduling APIs do
  not weaken the gate.
- Generated names remain stable on save/load; clone allocates fresh instance and
  storage identity. Collisions, changed parent policy, unsupported migrations,
  entered credentials, unknown effects and shared-worker retirement fail safely.
- Cancel, invalid submit, rejected close and target removal produce no operation;
  accepted submit executes once, even under rapid activation.
- Locale switch during an open menu, edited form, pending command and slow bundle
  load preserves owner identity and rejects stale publication.
- Native Copy/Paste and form interaction remain available; shortcut/IME tests
  run on different keyboard layouts. Browser, native-shell and spoken AT
  verification are separate evidence.
- Inherited theme tokens, RTL, long labels, narrow viewports, keyboard focus and
  forced colors remain usable. Localization must not introduce component-local
  theme defaults or change the canonical token dimensions.
- SSR can render resolved resources; hydration subscribes to the same semantic
  revisions without replaying invocations. Persist locale and user keybindings;
  never auto-resume a command with side effects from saved event state.

This design leaves focus/active-app selection UX explicitly TBD. A strong
untrusted-app sandbox, OS adapter, portable app-state codec/migration, physical
offload, cross-process transport and durable undo engine require separate
implementation/evidence. None is implied by the light-DOM context boundary.
