# Resource-backed commands

**Status:** design proposal, 2026-11-23; no runtime or component API is added by
this document. Companion documents: [application.md](application.md) and
[localization.md](localization.md). Existing native invoker commands and
`host-command` bindings retain their current contracts.

## Command identity and model

A command is a semantic operation with a stable resource ID. Buttons, menu
items, shortcuts and command palettes are presentations of that operation.
They reference the ID instead of copying its label, help and default bindings.
Use namespaced IDs such as `editor.copy` and `file.save-as`. Renaming or
translating a label does not change the ID, parameter names, handler or saved
user shortcut preferences. IDs are application-resource keys, not DOM IDs or
URLs; package import aliases must be explicit.

Windows menu selections and accelerators can deliver the same command ID via
`WM_COMMAND`, but menu, string and accelerator resources are separate types.
CEM's integrated command record is a proposed abstraction over that idea.
[Windows accelerators](https://learn.microsoft.com/en-us/windows/win32/menurc/about-keyboard-accelerators),
[Windows resource types](https://learn.microsoft.com/en-us/windows/win32/intl/preparing-a-resource-configuration-file).

Separate command semantics from presentation, as GTK separates actions from
menu labels. Resolve a contextual handler using the content origin, similar to
AppKit's responder-chain target discovery. These are design precedents, not an
assertion that DOM bubbling implements either toolkit automatically.
[GTK actions](https://docs.gtk.org/gtk4/actions.html),
[AppKit menu validation](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MenuList/Articles/EnablingMenuItems.html).

| Layer | Fields and ownership |
| --- | --- |
| Semantic definition | Stable ID, schema version, operation kind, parameter schema, optional parameter-dialog resource reference, repeat/concurrency policy, result schema, declared effect/undo capabilities |
| Localized resource | Name variants, description, accessible strings, category/search aliases, mnemonic candidates, parameter/help/error messages; see localization document |
| Shortcut profile | Structured platform/layout/locale defaults, primary/alternative bindings and user overrides |
| Handler registration | Component/context provider, supported ID, current eligibility and an admitted execution endpoint |
| Contextual state | `enabled`, `visible`, `checked`/mixed or radio value, `busy`, disabled reason and state revision |
| Invocation | Unique invocation ID, command revision, origin, selected handler/context, supplied/resolved parameters, lifecycle and result |

An operation kind is `action`, `toggle`, or `radio`. Toggle/radio invocations
request an explicit desired value where applicable; the handler confirms actual
state. A button click does not optimistically toggle every bound control.
Enabled/checked/busy state is contextual and mutable; it is not translated data.
One command can be available in one editor and disabled in another.

## Authoring and native HTML compatibility

The proposed canonical resource binding is **`command-id`**. The spelling
`command="resource-id"` conflicts with an existing API: `cem-action` already forwards
`command` to its native button. HTML reserves built-in values and custom
keywords beginning with `--`. A resource ID such as `file.save-as` does not
establish native command semantics. See the
[current action contract](component-reference.md#action-command-invokers) and
[HTML command keywords](https://html.spec.whatwg.org/multipage/form-elements.html#attr-button-command).

```html
<!-- Proposed resource bindings; not currently executable examples. -->
<cem-action command-id="editor.copy"></cem-action>
<cem-menu-item command-id="file.save-as"></cem-menu-item>

<!-- Existing native invocation remains distinct. -->
<cem-action command="show-modal" commandfor="preferences-surface"
            label="Preferences"></cem-action>
```

Do not infer a route from whether an arbitrary `command` string happens to be
in the resource registry. That would change behavior when resources load or a
future HTML command is standardized. Do not introduce `command` as an alias in
this proposal. A future breaking revision could reconsider that spelling, but
would require explicit versioning and migration.

| Proposed input | Contract |
| --- | --- |
| `command-id` | One stable semantic resource key, required for resource invocation |
| `command-parameters` | Native typed parameter value binding; omitted means no supplied values, not an empty string |
| `command-dialog` | Optional typed reference to a local parameter-dialog provider, overriding the definition's dialog resource; lexical HTML ID only on an explicitly documented same-app compatibility path |
| `command-context` | Optional typed content-origin reference; otherwise focus/session-derived |
| `command-label` | Optional presentation variant such as `menu` or `compact`; does not replace identity |

Resource-bound controls use native `button type="button"`. Reject simultaneous
resource execution and `href`, submit/reset behavior, `commandfor`,
`command-target`, `interaction`, `popovertarget`, or native `command` invocation
on that control. A separate submenu opener may use a native popup relationship.
An ordinary resource command's parameter dialog is opened by the orchestrator;
the button must not also open it through another mechanism.

Bindings populate labels, description relationships, effective shortcut hints,
and eligible state from one resource generation. Preserve accessible-name
precedence: explicit naming/projected label wins over the resource fallback;
diagnose overrides that contradict the command meaning. An explicit disabled
control can restrict availability, but cannot enable a handler that is disabled.
Unknown or unavailable resources disable execution with a diagnostic; never
fall back to evaluating the ID as code or an arbitrary URL.

These inputs must be added to action/icon-button/menu-item declarations and
generic runtime metadata in a future change. `cem-menu-item` does not currently
implement this resource binding. Explicit native controls can later opt into
the same generic protocol through the substrate; no document-global click scan.

## Handler discovery and command state

Handler providers register supported IDs, a synchronous eligibility snapshot,
and an execution endpoint with the runtime. Their declarative declaration maps
the invocation into component event/native slice state. Registration is a
proposed generic capability, not arbitrary application-local JavaScript.

For a given command, start at the retained content origin and walk its
participating component ancestry to the nearest application boundary. The
nearest provider declaring that ID owns it. If it is disabled, stop: do not
fall back to a parent handler and accidentally act on a different document.
Providers that do not declare the ID are skipped. With no provider, the command
is unhandled. Two registrations for the same ID at one provider are an error;
the same ID on different context instances is expected.

The same resolver powers menu/toolbar enabled state, command-palette results,
shortcut preflight and execution. Querying state cannot trigger side effects,
open dialogs or dispatch the execution event. Slow state computation publishes
a pending/unavailable revision; keyboard dispatch never awaits it. Recheck state
and document revision at commit, because a menu can remain open while selection,
permission, target or busy state changes.

## Commands across application boundaries

The [virtual application contract](application.md) adds a manager and native
context to each app. Resource/context inheritance does not merge handlers or
permit unhandled commands to escape. Command identity is interpreted inside the
target app's effective resource/override generation; two instances can expose
the same ID with different local state and implementations.

| Route | Admission and execution owner |
| --- | --- |
| Within one app | Retained local content context and nearest handler up to that app boundary |
| Container chrome to embedded app | Explicit target instance and exported command endpoint; execute inside that child under its context/eligibility |
| Embedded app to container service | Explicit allowlisted service request with typed parameters; container owns admission/execution and cannot be replaced by a child handler |
| Application management | Container/page-host manager owns `app.activate`, next/previous/start/pause/close and state-management commands, targeting registered instance IDs |
| Sibling app | Container-mediated exported endpoint; no direct ancestry fallback or automatic sibling discovery |

For example, container File > Save can bind to an exported `file.save-as` endpoint
of the manager-selected child. Pin the selected instance/generation/context when
the invocation is admitted. If the active selection changes while its parameter
dialog is open, do not silently save the newly selected app. If no eligible
target/endpoint exists, report unavailable instead of trying every child or the
container's own document handler.

Conversely, a child may request an exported container service, such as opening a
host-managed resource, without receiving permission to mutate the parent's
resources, storage/navigation settings, module maps, locale/theme or app registry.
An `app.activate` request from a child is still a request to the manager, not an
assignment to `activeInstanceId`. The container may accept, reject or transform
the request according to its admitted API. A child cannot export a replacement
for the parent's management command merely by using the same ID.

A bridge declares source and destination app identities, exported endpoint and
schema, allowed direction/effect, and admitted context/placement relationships.
Validate both sides, current generations and application admission epochs. Allocate a new local invocation
ID at a crossing, retaining an origin/correlation link; keep a bounded hop/path
record and reject loops. Correlate one result back through the same bridge and
release private/native parameter references after completion. Do not forward
events, references or grants verbatim merely because the ID string matches.
Crossing a browser frame additionally uses its admitted host message transport.

Raw `cem-command` execution stops at its owning app boundary. Runtime declarative
observers in other apps must not capture it as their own island event; exported
safe notifications use an explicit bridge. Same-document capture listeners can
still observe light-DOM events, so this is CEM admission/ownership insulation,
not a confidentiality guarantee against arbitrary script. Sensitive values
remain outside public event detail under the protected parameter contract below.

Local routing remains focused-context based within the identified running app.
Cross-app focus-to-activation and keyboard ownership are **TBD**, as recorded in
the application proposal. Management can have an explicit active-child reference
without assuming every `focusin`, hover, load or restore selects it.

Paused/offline/offloaded apps do not process business commands, shortcuts or
event-to-slice captures, including commands arriving through an export bridge.
Return unavailable at the manager; never queue the command to run on restart.
Only activation/start/close reach the stopped instance's management control
plane. Activation/start can prepare a runnable instance, but a subsequent business
command is a fresh admitted request, not a replay of the request that woke it.
Enclosing stopped apps also block descendant business admission. Save/clone of
sealed state can be manager bookkeeping without invoking the stopped app.

## Focus-origin event routing

Use **`cem-command`**, not native `command`, for application execution requests.
The semantics differ: native `CommandEvent` carries `command` and `source` and
is delivered to the explicit target; the HTML invocation algorithm does not
make it a bubbling application responder-chain request. It has no standard
parameter or async-result protocol.
[HTML invocation algorithm](https://html.spec.whatwg.org/multipage/form-elements.html#the-button-element),
[CommandEvent interface](https://html.spec.whatwg.org/multipage/interaction.html#the-commandevent-interface).

The following algorithm is proposed:

1. Resolve the resource, origin, handler and eligibility before suppressing any
   input default. Create one invocation ID and pin semantic/schema revision.
2. Validate supplied parameters. Collect missing values as described below.
   There is no execution event for a request still waiting on parameters.
3. Revalidate the pinned target and handler. Create an immutable parameter
   snapshot for every invocation, including fully supplied/parameterless ones;
   scalar detail and nested arrays must not be mutable by observers. Dispatch
   `cem-command` at the
   original focused/content element, or its retained stable context host when
   the inner rendered control has been replaced. A disconnected context fails;
   do not retarget to the new focused editor.
4. The event bubbles with `bubbles: true`, `composed: true`, `cancelable: true`.
   Participating providers along the route can observe it. The selected handler
   claims the runtime invocation synchronously; no other provider may claim it.
   An application boundary prevents any cross-application execution.
5. Claiming records ownership but does not mean success. It does not require
   stopping propagation, so ancestor declarative observers can record the same
   request without executing it. Unselected/duplicate handlers cannot execute.
6. After dispatch returns, if `preventDefault()` was called anywhere on the
   permitted route, cancel without executing. Otherwise recheck app/ancestor
   running gates, application generation/admission epoch, target lifetime,
   document identity/revision, handler identity, eligibility, authorization and
   semantic revision before starting the one claimed endpoint.
   Dispatch listeners or a synchronous slice render may have changed them. No
   claim yields `unhandled`; `stopPropagation()` by itself is neither a claim nor
   success. Diagnostics expose interrupted routes.

This keeps focused-element bubbling and durable event reflection while avoiding
two handlers acting on the same request. Event listeners must not perform the
side effect during observation; the admitted runtime endpoint starts only
after the dispatch/claim/cancellation decision. Declarative async handlers are
fed once after admission, never once per observing ancestor.

Activation-sensitive endpoints must connect directly to an admitted host service
in the same originating event stack after dispatch and revalidation. They cannot
depend on asynchronous slice rendering, a worker or a promise completing. An
ordinary declarative handler may run asynchronously when no such activation is
needed; otherwise return `activation-unavailable` and provide a fresh explicit
gesture instead of silently retrying the gated API.

Capture listeners can veto a request, but generic observers must not rewrite its
ID, target or parameters. Runtime invocation ownership is held in an internal
registry; do not put `claim()` functions, promises or DOM elements in `detail`.
An arbitrary author-dispatched lookalike `cem-command` cannot create an admitted
invocation, replay its ID, or grant host permissions. Programmatic invocation
uses the same admitted request path and remains subject to host policy.

## Events and data-island reflection

Use camelCase detail fields and hyphenated event/attribute names, following
[component conventions](conventions.md#3-events).

| Event | Time and payload meaning |
| --- | --- |
| `cem-command` | Final validated execution request; cancelable, routed from content origin |
| `cem-command-start` | Accepted endpoint starts; noncancelable lifecycle notification |
| `cem-command-result` | Exactly one terminal outcome for an invocation, including cancellation before start or unhandled resolution |
| `cem-command-state` | Contextual eligibility revision changed; no execution implied |

Core detail fields are `invocationId`, `applicationInstanceId`,
`applicationGeneration`, `applicationEpoch`, `commandId`, `commandRevision`,
`contextId`, `source` (`keyboard`, `action`, `menu`, `palette`, `host`, `programmatic`),
`resourceGeneration`, and `parameters`. Results add `status`, `code` and
optional typed `value`; state notifications have a context/command/state revision
instead of an invocation ID. `status` is `succeeded`, `cancelled`, `failed`,
`unhandled`, or `unavailable`. Progress, if supported, is correlated by ID and
bounded; it never substitutes for a terminal result.

Cross-app requests add admitted source/destination and bridge correlation
metadata. Names/titles are diagnostic presentation only; instance IDs plus fresh
admission identify endpoints. Context overrides, pause, reparenting, load or clone
invalidate stale endpoint tokens without transferring their authority. Check
the admission epoch after dispatch and before any later async continuation or
result publication, including same-worker pause/resume.

The scalar envelope is an explicitly named API/control boundary and remains
JSON-serializable under current conventions. For zero parameters use an empty
parameter set, not a positional null. One scalar parameter still has a schema
name; multiple parameters use named fields, not an order-dependent tuple.
`parameters` contains JSON-safe scalar/repeated-scalar values in that profile.

Rich CEM values require a separately declared native parameter profile and a
typed event-to-native-slice channel. Such an event exposes an opaque
`parametersRef` in the scalar detail instead of cloning its tree into a JavaScript
record; the admitted binding resolves the retained artifact into the owning
island. This extension must be implemented explicitly before claiming support
for tree/file parameters. Generic `CustomEvent.detail` cloning is insufficient.
File handles, DOM nodes, functions and transient WASM handles are never serialized
as event parameters. Private host capabilities remain in the admitted adapter.

Sensitivity is checked **before event dispatch and island capture**. The scalar
detail profile is restricted to capture-safe fields. Sensitive scalars and native
values use an opaque invocation-scoped reference; only the selected admitted
endpoint can access the full immutable parameters through an ephemeral channel.
Command-specific event projection gives observers safe metadata/redaction markers
and never resolves private values through ordinary `slice-event` serialization.
The existing generic detail-clone path is not sufficient for this extension.
Redacting only when persisting history would leave secrets in the durable island.
Release private references at terminal outcome/disposal and never hydrate them.

Reflect requests/results into existing `cem-events:event-state` and declared
slices, preserving invocation IDs, source, revision and native owners. Each
instance sees only events admitted by its binding/ownership context. Do not
create a global mutable `activecommand` record or reconstruct parameters from
rendered DOM. Event observation and executable handler registration are distinct;
a `slice-event="cem-command"` observer alone does not claim execution.

### The `activecommand` idea

Prefer a native invocation slice as the internal trigger. If a component needs
a public diagnostic projection, spell it `active-command`, following existing
lower-kebab-case attributes. It is **derived state**, never an executable input.
Likewise, `active-command-parameters` should not carry serialized structured
data; query the invocation slice/native value instead.

Attribute assignment cannot distinguish two successive Copy requests with the
same ID, correlate async results or atomically update ID plus parameters.
An event/invocation revision can. Locale updates, rerender and hydration must
not trigger execution. Multiple concurrent invocations also cannot be described
by one meaningful active-command attribute; use the correlated collection.

## Parameters and modal forms

Command parameters are locale-neutral and schema-defined. Each has a stable
name, type, required/optional status, optional default, constraints, cardinality
and sensitivity classification. Labels and enum display names are localized;
names and enum values are not. Reject unknown fields by default. Preserve
missing versus empty versus null, repeated values, booleans and numeric types.
Do not infer parameter names from translated labels or cast using UI language
rules without a declared typed control conversion.

| Command | Parameter schema | Resolution |
| --- | --- | --- |
| `editor.copy` | No explicit parameters | Handler uses retained selection/context; native editing is preferred where appropriate |
| `view.zoom-to` | Required `percent`, finite number within declared range | Use supplied value or one-field parameter dialog |
| `file.save-as` | Required `contentType` (allowed MIME enum), required `filePath` (host-specific destination string) | Use supplied values or referenced form; web profile resolves a picker capability instead of treating path text as authority |

The Save As example's schema is suitable for a desktop host. A browser profile
must explicitly adapt the destination to a suggested filename plus a
picker-issued capability, or a download. The adapter must not pretend a web page
can freely write an arbitrary filesystem path. The command's source document is
retained context, not an extra user-facing parameter.

Use `command-dialog`/the command definition's `parameterDialog` reference to
identify a dialog provider, rather than inventing `activecommand-parameters`.
The schema and dialog reference are semantic resources; translations may change
their labels but cannot select executable providers. A dialog resource resolves
to one admitted local instance per invocation/context, with generated native
IDs where necessary. Never globally locate the first dialog with a matching
resource ID. Typed references follow the existing reference/grant rules;
literal DOM IDs are only an explicit same-application compatibility boundary.

The future dialog contract must declare one parameter form and explicit accepted
submitters. A form may be nested through slots, but its native form owner and
provider ownership must be unambiguous. An unrelated form inside task content
cannot resolve the invocation.

1. Validate supplied values and apply/validate declared defaults for omitted
   fields. Explicit values, including empty/null where permitted, take precedence
   over defaults. If all required values now exist and no explicit confirmation
   policy applies, execute without opening a dialog. Required confirmation is a
   separate policy from missing parameters.
2. If values are missing and a permitted dialog exists, create a draft seeded
   from supplied values/defaults. Explicit values take precedence over defaults;
   the user may edit them in this invocation. Without a resolver, return
   `unavailable` with `parameters-required`; do not guess values or run partially.
3. Open `cem-dialog mode="modal"` through its existing shared surface lifecycle.
   Reset session acceptance and stale `returnValue` before opening. Opening
   retains the content target and does not itself execute the command.
4. Only an accepted native form submission can resolve parameters. Apply native
   constraint validation, capture the submitting button, construct the successful
   controls entry list, then validate/coerce against the command schema.
   Repeated names stay repeated; unchecked/disabled/unnamed controls follow
   native form semantics. Never flatten entries with `Object.fromEntries()`.
5. Cancel/escape/dismiss/forced close are cancellation, not successful input.
   `dialog.returnValue` is a string identifying the close action, not the full
   form data. `method="dialog"` closes a surface but does not save a document.
6. On accepted valid input, freeze a fresh parameter value and revalidate the
   target, authorization, handler state and schema revision. Dispatch the final
   `cem-command` once under the same invocation ID. Ignore repeated submits and
   late callbacks belonging to another dialog session.

The coordinator must capture and validate input at the submit boundary before
native `method="dialog"` completion loses session state. Schema errors prevent
closure/execution and remain associated with the controls. Cancel controls use
an explicit non-submit close request, or declared cancel submitter with
`formnovalidate`, so invalid required fields cannot trap cancellation. Enter
submission uses the native default accepted submitter and respects IME
composition; never invoke `form.submit()` to bypass validation.

Keep submit available for validation feedback while the draft is invalid;
prevent duplicate acceptance after a valid submit. Focus the invalid field or
an appropriate linked error summary. Final-submit host calls that need transient
activation must use preloaded synchronous validation/admission. If a required
remote/worker check interrupts that path, require a fresh explicit user gesture
before the gated operation instead of assuming activation persists.

Parameter resolution ends when valid input is accepted. Execution may still
fail. Default policy closes the resolver after acceptance and reports a
correlated command result elsewhere. A task that remains open through Save and
shows service errors must explicitly declare that workflow, using the existing
guarded surface lifecycle. Closing a dialog is never an operation-success signal.
While a modal task is open, present its relevant status/errors within the
reachable dialog, not in an inert background live region.

Focus enters the named dialog appropriately and returns to the session's
eligible invoker/content destination after cancellation or completion. Preserve
newer deliberate focus and never focus removed/disabled targets. Reuse existing
[dialog behavior](dialog-contract.md), including stable native owner identity;
do not replace it with a custom focus trap or a new modal toggle path.

## Lifecycle, concurrency and effects

```text
requested -> resolving -> awaiting-parameters -> ready -> dispatched -> running
                 |                |              |          |           |
                 +----------------+--------------+----------+-----------+
                                      -> one terminal result
```

Parameterless/fully supplied requests skip `awaiting-parameters`. Define the
transition rules in the implementation; a locale publication never advances
this state machine. By default, a command is single-flight per command/context:
additional activation returns `unavailable`/`busy`, without creating another
dialog. Explicit repeatable navigation commands may allow repetition; mutating
commands must declare serialization or independent concurrency. Invocation IDs
prevent duplicate admission and correlate results; they do not promise exactly
once execution by remote services. External retries need an idempotency contract.

Abort before dispatch guarantees no execution. Cancellation while running is a
request to an endpoint that declares abort support; it must not report
`cancelled` if the side effect already committed successfully. Report uncertain
remote outcomes explicitly as a failure code, with safe recovery guidance; do
not automatically retry Save, Delete or payment-like operations. Command state
refreshes after outcomes, and only the handler confirms a toggle or checked state.

Undo/redo is an optional domain transaction capability. Resources can supply
localized undo labels; labels alone cannot make an arbitrary command reversible.
Permission checks, clipboard policy and user activation remain host-owned.
Sensitive parameters/results are excluded from persisted event history,
telemetry and SSR snapshots unless an explicit safe projection permits them.
Hydration restores presentation and observed history, never executes old events.

## Shortcut dispatch

Store a binding as structured modifier/key data, not the human-readable string
`Ctrl+Shift+S`. A binding declares `key` versus physical `code` matching,
modifier set, platform, applicable context, editing policy and repeat policy.
The platform profile resolves an abstract primary modifier to Control or Meta.
User overrides, including an explicit unbound value, take precedence over
locale/platform defaults. UI language is not keyboard layout.

Use the following dispatch rules:

- Let target/component keydown listeners run first. The application observes
  bubbling `keydown` and skips an already prevented event. Preserve the later
  native default action unless an eligible invocation is admitted; outer
  applications also skip nested application events even if those did not claim
  a shortcut.
- Apply app/ancestor lifecycle admission first. Stopped apps do not process even
  unclaimed shortcuts or reflect them into their islands. Manager activation,
  start and close controls remain in the surviving control plane. The eventual
  cross-app focus/active-selection policy determines the running keyboard owner;
  this shortcut algorithm does not invent that policy.
- Ignore IME composition, dead/process keys and AltGraph text entry. Normalize
  key/case/modifiers under the declared matcher; do not treat AltGraph as an
  accidental Control+Alt shortcut.
- Preserve native editing in input, textarea, contenteditable and native form
  controls. Ordinary Copy/Paste/select/edit commands stay native there unless an
  editor provider explicitly owns equivalent behavior. Do not use deprecated
  `execCommand` as a universal execution engine.
- Preflight the same context handler/eligibility snapshot used by menus. A
  disabled/unhandled command does not swallow the browser default. Consume the
  key synchronously only after an eligible invocation is admitted, including an
  admitted parameter task. `preventDefault()` cannot be delayed until async work.
- Enforce reserved browser/OS combinations as a host profile constraint. Some
  keys never reach the page. Offer visible invocation alternatives, user
  rebinding and diagnostics rather than claiming a global OS hotkey.
- At the same active scope, conflicting IDs on one binding disable the ambiguous
  binding and expose a diagnostic. More specific context bindings may shadow an
  app-wide binding only by a declared precedence rule. Never choose by DOM order.
- Character-only shortcuts default off globally. A future opt-in must offer
  disabling/remapping or focus-only activation; see
  [WCAG character key shortcuts](https://www.w3.org/WAI/WCAG22/Understanding/character-key-shortcuts.html).

The first version supports simultaneous combinations and alternatives.
Multi-stroke chords require a separate timeout, cancellation, conflict and
accessible-discovery contract; do not interpret spaces ambiguously as a sequence.

Generate visible shortcut hints and `aria-keyshortcuts` from the **effective
installed** binding map. ARIA advertises behavior; it does not implement it.
Remove unavailable bindings from that projection, distinguish simultaneous
combination syntax from alternatives, and provide visible help beyond `title`.
[WAI-ARIA key shortcuts](https://www.w3.org/TR/wai-aria-1.2/#aria-keyshortcuts).

## Verification required before adoption

Native tests must cover definition validation, schema changes, locale-neutral
parameters, scalar/native profiles, handler/state resolution, claim/cancel
ordering, duplicate IDs, repeated invocation IDs, stale contexts and outcome
correlation. Browser tests must exercise focus-origin bubbling, observer slices,
two nested editors/applications, native command coexistence, menu/task focus
relay, parameter forms, invalid/cancel submits, repeat and activation-sensitive
host calls. Test resource-bound actions and menu items as separate declarations.

Include selection changes while menus are open; removed targets while dialogs
wait; successful and failed Save; separate Copy invocations with identical
parameters; locale change during execution; sensitive-value redaction; no
execution on rerender/SSR/hydration; and accessible names, shortcuts, state,
focus, RTL/long labels, inherited themes and forced colors. These are future
acceptance cases, not fixtures or tests delivered by this design-only change.

Cross-app acceptance also requires exported-child Save targeting, denied upward
context writes, allowlisted parent services, sibling isolation, bridge-loop
refusal, same-ID/different-instance state, stopped-app refusal without replay,
and stale-generation rejection after pause/load/clone. MDI manager commands are
tested independently from business handlers and TBD focus-selection behavior.
