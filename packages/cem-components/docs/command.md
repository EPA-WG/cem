# Resource-backed commands

**Status:** design proposal, 2026-11-23; no runtime or component API is added by
this document. Companion documents: [application.md](application.md) and
[localization.md](localization.md). Existing native invoker commands and
`host-command` bindings retain their current contracts.

**Insulation revision — 2026-11-24:** every application starts with a private
command/context environment. Inheritance, shared services and named-group
membership are explicit host-admitted relationships. Within that selected
environment, existing native CEM per-property scope inheritance still applies;
this proposal adds no generic `inherit` or `inherited` syntax.

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

For a given command, resolve the retained content origin to its registered
command context, then walk its logical component owners toward the owning app.
The nearest provider declaring the ID owns the decision. A disabled or pending
provider returns `unavailable` and ends the search.
Providers that do not declare the ID are skipped. With no provider, the command
is unhandled. Two registrations for the same ID at one provider are an error;
the same ID on different context instances is expected.

The same resolver powers menu/toolbar enabled state, command-palette results,
shortcut preflight and execution. Querying state cannot trigger side effects,
open dialogs or dispatch the execution event. Slow state computation publishes
a pending/unavailable revision; keyboard dispatch never awaits it. Recheck state
and document revision at commit, because a menu can remain open while selection,
permission, target or busy state changes.

A command may have its label in the server-rendered view while its handler or
required provider loads lazily. Until the executing app can admit the command,
keep the control unavailable and leave the shortcut unclaimed. Enable it when
its own dependencies are ready. A fresh invocation is then required. Other
ready commands continue under the [application startup contract](application.md#first-display-and-lazy-features).

## Commands across application boundaries

The [virtual application contract](application.md) adds a manager and native
context to each app. Insulation is the default: a child does not obtain parent
handlers, services, mutable state or execution-group membership from structural
AST/DOM ancestry. The host may admit particular resource/context families or
shared provider bindings. Resource inheritance never merges handlers or permits
unhandled commands to escape. Command identity is interpreted inside the target
app's effective resource/override generation; two instances can expose the same
ID with different local state and implementations.

A named scope identifies a native context/group binding, not a permission to
join it. Resolving a scope name, matching a vendor/package name, or sharing
immutable command definitions cannot confer endpoint access. Group membership,
mutable provider sharing and command export permissions are separate admitted
relationships. Native implicit property lookup applies inside the selected app
context; it does not make missing local handlers fall through to an external
container. A trusted shared execution group may share expressly admitted mutable
services; that does not merge instance-local command state, invocation IDs or
lifecycle gates.

Declarative applications invoke generic `cem-elements` capabilities; that
framework is the guarded browser API boundary. App authors need no extra wrapper
component, prefixed global names or frame-specific command implementation to
obtain isolation. The same logical command, context, parameter and result API
applies to embedded light-DOM and iframe hosts. Local runtime bindings and
admitted bridge transports enforce the same command authority.
An actual cross-origin/sandboxed frame adds browser enforcement for hybrid apps
that include executable code outside the CEM-only contract; it is not a default
prerequisite for a declarative CEM microapplication.

| Route | Admission and execution owner |
| --- | --- |
| Within one app | Retained local content context and nearest handler up to that app boundary |
| Container chrome to embedded app | Explicit target instance and exported command endpoint; execute inside that child under its context/eligibility |
| Embedded app to container service | Explicit allowlisted service request with typed parameters; container owns admission/execution and cannot be replaced by a child handler |
| Application management | Container/page-host manager owns `app.activate`, next/previous/start/pause/close and state-management commands, targeting registered instance IDs |
| Sibling app | Container-mediated exported endpoint; no direct ancestry fallback or automatic sibling discovery |

For example, container File > Save can bind to an exported `file.save-as` endpoint
of the manager-selected child. Pin the selected instance and context when
opening that menu and revalidate them at invocation. Selection changes while
the menu or parameter form is open preserve that target. Losing the target or
its exported endpoint makes the action unavailable.

Conversely, a child may request an exported container service, such as opening a
host-managed resource, without receiving permission to mutate the parent's
resources, storage/navigation settings, module maps, locale/theme or app registry.
An `app.activate` request from a child is still a request to the manager, not an
assignment to `activeInstanceId`. The container may accept, reject or transform
the request according to its admitted API. A child cannot export a replacement
for the parent's management command merely by using the same ID.

A bridge declares source and destination app identities, exported endpoint and
schema, allowed direction/effect, and admitted context/placement relationships.
Validate both sides, current generations and application admission epochs.
Allocate a new local invocation ID at a crossing, retaining an origin/correlation
link; keep a bounded hop/path record and reject loops. Correlate one result back
through the same bridge and release private/native parameter references after
completion. Do not forward
events, references or grants verbatim merely because the ID string matches.
Crossing a browser frame additionally uses its admitted host message transport.
Authenticate the current endpoint and bound schema/capabilities, not a payload's
claimed app, scope or vendor name. Browser transport restrictions remain in
force; inability to preserve a required activation capability produces an
explicit unavailable result, never a direct ambient API fallback.

Raw `cem-command` execution stops at its owning app boundary. Runtime declarative
observers in other apps must not capture it as their own island event; exported
safe notifications use an explicit bridge. The framework admits capture only to
the request's owning app and authorized observers; unrelated CEM apps cannot
install an arbitrary document listener to bypass that rule. Unmanaged script in
a hybrid same-document host is outside the CEM-only guard and requires the
appropriate stronger host boundary. Sensitive values remain outside public
event detail under the protected parameter contract below.

For ordinary keyboard commands the focused control supplies the executing app
and logical route. A host menu captures the content context at entry. A
selected-child binding pins the manager's selection instead. Neither binding
may switch targets after capture. See [keyboard ownership across apps](application.md#keyboard-ownership-across-apps).

Paused/offline/offloaded apps do not process business commands, shortcuts or
event-to-slice captures, including commands arriving through an export bridge.
Return unavailable at the manager; never queue the command to run on restart.
Only activation/start/close reach the stopped instance's management control
plane. Activation/start can prepare a runnable instance, but a subsequent business
command is a fresh admitted request, not a replay of the request that woke it.
Enclosing stopped apps also block descendant business admission. Save/clone of
sealed state can be manager bookkeeping without invoking the stopped app.

## Logical command route and permissions

The invoker supplies the request. The content target identifies the object of
the command. For a keyboard shortcut the focused placement supplies the initial
context. For a menu or toolbar, use the retained interaction context or an
explicit target binding. Resolve the placement through its registered owner
and walk the target's logical component owners up to its application. Projection
into a layout, slot or surface preserves this route. A presentation component
may participate through an explicitly admitted command-context binding.
A placement grant alone gives it no command rights. Missing or ambiguous
ownership makes the command unavailable.
[Placement ownership](../../../docs/cem-element-granted-placements-design.md#ownership-and-admission).

Windows offers two precedents. MFC gives an active view an opportunity to
handle a command and can then consult the document attached to the view.
WPF distinguishes the command source, target and binding and allows a focused
or explicit target. CEM additionally limits delivery to the app's admitted
participants. [MFC command routing](https://learn.microsoft.com/en-us/cpp/mfc/command-routing#standard-command-route)
and [WPF commanding](https://learn.microsoft.com/en-us/dotnet/desktop/wpf/advanced/commanding-overview).

| Permission | Who participates |
| --- | --- |
| Observe | Subscribed logical owners on the route receive read-only, capture-safe projections in their data islands. The invoker can track its own request. An external subscriber requires an explicit export. |
| Veto | Only guards declared for this command on the logical route may refuse execution. Run them synchronously from the owning app toward the target. The first refusal ends the request with a stable reason. Container restrictions apply at admission and at explicit bridge crossings. |
| Claim and execute | Search from target to app. The runtime binds the invocation once to the selected, eligible endpoint. Only that endpoint receives the executable request and its authorized parameters. With a valid route and no declaration, return `unhandled`. |
| Receive the result | The coordinator settles the invocation once. The original caller receives its permitted result while its binding is live, through the originating bridge for a cross-app call. Observers receive only subscribed status. The admitting host retains the write outcome after the editor closes. |

1. Use the same resolver for menu state, shortcut preflight and execution.
   Before consuming input, establish eligibility and pin the target, its logical
   owner and handler, the owning instance, and the command/schema revision.
2. Validate supplied parameters and collect missing values for that invocation.
   Before execution, freeze the resolved parameters (including repeated values)
   and revalidate the pinned route. A change of focus or physical layout alone
   keeps the target. Loss or change of its logical owner ends the request.
3. Check grants and the app and ancestor lifecycle gates, then run the declared
   guards. A guard can refuse but cannot edit the request or execute its effect.
   Immediately before starting the endpoint, recheck the instance generation and
   admission epoch, document identity/revision, guard and handler registrations,
   eligibility and semantic revision. A stale decision never transfers a claim.
4. Deliver the actionable `cem-command` once through the selected runtime
   binding. Project request, start and result into admitted subscriptions.
   Receiving a projection never executes or cancels the command.

Native click, keydown and HTML invoker events retain their browser behavior.
DOM capture follows physical ancestry even for a nonbubbling event. The runtime
therefore keeps actionable delivery and veto private to the logical route.
Calling `preventDefault()` on a native input can affect input preflight.
Calling it on an observer notification cannot veto an admitted invocation.
[DOM event dispatch](https://dom.spec.whatwg.org/#concept-event-dispatch)
and [HTML invoker commands](https://html.spec.whatwg.org/multipage/form-elements.html#the-button-element).

Activation-sensitive endpoints must connect directly to an admitted host service
in the originating gesture stack after the guards and final revalidation. They cannot
depend on asynchronous slice rendering, a worker or a promise completing. An
ordinary declarative handler may run asynchronously when no such activation is
needed; otherwise return `activation-unavailable` and provide a fresh explicit
gesture instead of silently retrying the gated API.

The host service receives the invocation's admitted app/provider identity and
uses the app's effective storage, location and other capabilities. It must not
select the containing page's ambient browser objects because the target shares
a document with the manager. The application contract owns provider binding;
command handlers consume the same local service names in either hosting form.

The runtime retains invocation ownership internally. Observer projections carry
immutable data and exclude executable claims, promises and DOM elements.
An arbitrary author-dispatched lookalike `cem-command` cannot create an admitted
invocation, replay its ID, or grant host permissions. Programmatic invocation
uses the same admitted request path and remains subject to host policy.

## Events and data-island reflection

Use camelCase detail fields and hyphenated event/attribute names, following
[component conventions](conventions.md#3-events).

| Event | Time and payload meaning |
| --- | --- |
| `cem-command` | Final validated request at the selected runtime endpoint. Subscribers receive a read-only projection. |
| `cem-command-start` | Accepted endpoint starts; noncancelable lifecycle notification |
| `cem-command-result` | One terminal outcome including cancellation before start or unhandled resolution. A still-admitted caller receives the allowed result and subscribed observers receive their permitted status. |
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
For command forms, apply [destination privacy from the first edit](#destination-privacy-from-the-first-edit)
before the runtime can capture a supplied value or keystroke.

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

## Nonmodal parameter forms

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
| `file.save-as` | Required MIME `contentType`. Desktop requires a private `filePath`. The browser accepts an optional private `suggestedName`. | Pin the host profile before collecting parameters. The browser adapter obtains the destination handle. See [Save As across hosts](#save-as-across-hosts). |

The app that owns the selected execution endpoint also owns parameter collection.
For a host menu invoking an exported child command, this is the child. Resolve
the form in that instance's effective, admitted resource view after applying
supplied values and defaults. Use the following precedence when a form is needed.

1. Use an explicit `command-dialog` binding, otherwise the definition's
   `parameterDialog` reference, when declared. Validate it in the owner's scope
   under the existing typed reference and grant rules.
2. Otherwise look up a **parameter-form resource with the same stable command ID**.
   A template or a `cem-element` declaration can supply it. For example, two
   editors can each supply their own form under `file.save-as`. Lookup uses the
   resource kind and ID within the owning app. Translated names, global DOM IDs,
   and the order of elements on the page play no part in selection.
3. If that lookup completes with no custom resource, generate a form from the
   pinned parameter schema. Use localized field names, descriptions and option
   labels for presentation. Use schema keys, types, cardinality, constraints
   and defaults for binding and validation. A bounded number such as
   `percent` can use a number field and a MIME enum can use a choice control.
   A destination requiring a picker needs an admitted picker capability.
   If the generator cannot collect a required value, return `unavailable`
   with `parameters-required` before opening the surface.

A pending lookup waits for resolution. A denied, ambiguous, broken or
schema-incompatible custom binding reports its failure. Generation applies
only on confirmed absence, so a required custom workflow cannot silently change.
Each invocation gets its own form session and draft. Both custom and generated
forms use the same nonmodal surface and schema validation.

The form provider declares one parameter form and explicit accepted
submitters. A form may be nested through slots, but its native form owner and
provider ownership must be unambiguous. An unrelated form inside task content
cannot resolve the invocation.

1. Validate supplied values and apply/validate declared defaults for omitted
   fields. Explicit values, including empty/null where permitted, take precedence
   over defaults. If all required values now exist and no explicit confirmation
   policy applies, execute without opening a dialog. Required confirmation is a
   separate policy from missing parameters.
2. When input is required, select the form as described above and seed its
   draft with the supplied values and applicable defaults.
3. Open `cem-dialog mode="nonmodal"` through the shared surface lifecycle
   (native `dialog.show()`). Reset session acceptance and stale `returnValue`.
   Only this invocation waits. The owner, container and siblings remain
   interactive in embedded and iframe hosts. Retain the original invocation,
   handler and content target even if focus or the manager's active app changes.
4. Only an accepted native form submission can resolve parameters. Apply native
   constraint validation, identify the submitting button and collect successful
   controls according to native form semantics. For a protected form, the private collector reads the owned successful controls without
   exposing a general `formdata` event containing the private entries. Validate
   and coerce the collected values against the command schema.
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
Give the nonmodal form an accessible title that identifies the task and, where
needed, the target document. Show field errors at the relevant controls.
Focus may leave while the draft is open. On close, restore an eligible
destination only if the form still owns focus. Preserve any newer deliberate
focus in this or another app. Reuse the existing
[nonmodal focus contract](dialog-contract.md#outside-focus-on-nonmodal-closure).

## Save As across hosts

`file.save-as` saves a pinned document revision to a chosen destination. The
owning app selects the host profile and pins its parameter schema before
resolving the same-ID form. The document comes from the retained command
context. The caller may supply admitted parameters or leave them for the
owning form to collect.

| Action | Input and authority | What counts as completion |
| --- | --- | --- |
| Desktop Save As | An allowed MIME `contentType` and private `filePath`. The granted file service validates the destination and overwrite policy. | The service confirms that it wrote the pinned revision. |
| Browser Save As | An allowed `contentType` and optional private `suggestedName`. The admitted adapter obtains a writable handle from the user's picker during an eligible gesture. | The adapter writes the revision and awaits successful stream close. Choosing a file only selects the destination. |
| Browser **Download a copy** | The user explicitly requests a download in the selected format, optionally with a suggested filename. | Report **Download requested** after handoff. The browser controls the eventual destination and completion. The editor keeps its existing save destination and dirty state. |

After confirmed Save As, the editor may adopt the chosen destination for future
saves and mark the written revision saved. Edits made since that revision stay
dirty. Only the guarded document service can retain the destination binding.
The shared format preference can update after success under its own grant.

In an embedded app, the host may provide a picker through an explicitly granted
adapter with an eligible user gesture. An unsupported picker or denied grant
makes Save As `unavailable`. If activation expires, offer a fresh gesture.
An identifiable user dismissal yields `cancelled`. A confirmed write or close
failure yields `failed`. If the service cannot establish whether the write
committed and the invocation must end, report `failed` with `outcome-unknown`
to authorized diagnostics and suppress automatic retry. A download is a separately labelled user choice.
[File picker requirements](https://wicg.github.io/file-system-access/#api-showsavefilepicker) ·
[File write and close](https://fs.spec.whatwg.org/#dom-filesystemfilehandle-createwritable) ·
[HTML downloads](https://html.spec.whatwg.org/multipage/links.html#downloading-resources)

### Destination privacy from the first edit

Classify the path, suggested filename and handle as private at admission,
including values supplied by the caller. Before enabling a custom or generated
form, bind its owned controls to an invocation-private draft. Exclude that form
from general event and `formdata` projections, form snapshots, validation mirrors
and event-to-slice capture. Both live capture and SSR/resume readers need this
rule. A collector that cannot protect the draft makes the invocation unavailable.

On an accepted submit, the admitted collector reads the owned successful
controls privately, validates them, freezes the values and passes an
invocation-scoped reference to the selected endpoint. Public validation can
identify the field and a safe error code. Keep raw values and handles out of
data islands, diagnostics, telemetry, shared preferences, SSR and checkpoints.
A locale switch preserves the live private draft. Cancellation, teardown or a
terminal outcome releases the draft and transient references. After a confirmed
save, the service may keep its separately authorized destination binding.

The current renderer constructs `FormData` for ordinary capture. Its resume
serializer can read control values directly. In Chromium and Firefox, a native
probe confirmed that constructing `FormData` before submit exposed the typed
test path and fired `formdata` even with autocomplete off. A protected
collector and capture exclusions are required runtime work.
[Form and validation capture](../../cem-elements/src/lib/cem-elements.ts)

## Version 1 Save As example

[Read the complete CEM-ML source](examples/file-save-as.v1.cem). The file uses
CEM-ML version 1. The application vocabulary, resource manifest and command
revision each have their own version. The records describe a proposed consumer.
The existing parser can read and round-trip the tree.
Each `resources` container groups messages for one language. The manifest
selects English by default, and the French group declares English as its fallback.

The desktop shell owns a French menu override and a named preferences store.
Its child editor owns the document, the command context and the nonmodal
parameter form. Selecting the menu item in the editor proceeds as follows.

1. Resolve `file.save-as` revision 1 against `note-context` and pin document
   `note-1`. The menu shows **Enregistrer le document sous…**. The action and
   form title still say **Enregistrer sous**, and the French description comes
   from the child’s French resources. Switching the editor to English gives **Save As…**.
   The Windows profile binds Ctrl+Shift+S to focused content. Its letter key
   matches ASCII `s` or `S` with the declared modifiers. When focus is in this
   editor, shortcut preflight uses the same logical route as the menu item.
2. Collect the missing `contentType` and `filePath` in the owning editor's
   same-ID form. The admitted preference can prefill the format. The user can
   choose plain text or Markdown. These choices submit `text/plain` and
   `text/markdown` in either language. Cancel ends this invocation without a
   write. If the custom form is omitted, the same two parameters and their
   localized labels drive the schema-generated form.
3. On submit, validate the values and recheck the pinned document and service
   grant. The desktop service writes the document using the requested format and
   private destination. On confirmed write, the editor can adopt that destination
   and mark the pinned revision saved. Success updates `lastContentType` when
   its separate store grant permits it. Failure to record the preference leaves the completed
   save successful. A live, admitted caller receives `invocationId` and `status` once.
   The draft stays in `editor-session`. The path is confined to the invocation's
   parameter form and admitted execution endpoint.

| Change to the example | Effective child context |
| --- | --- |
| Parent omits the French menu override | The child uses **Enregistrer sous…** from its French resources. |
| Parent explicitly supplies empty optional description text | The child shows no description for that entry. An empty required menu label fails override validation. |
| Host denies or revokes the optional preference binding | The editor can collect the format manually and save. It skips the preference read or write while denied. The shell's private store is inaccessible. |
| Host denies or revokes the execution grant before dispatch | The command is unavailable. Neither the file nor the preference is written. |

Both apps can use the private key `draft`. Only `editor-1` receives the grant
to read and write `store-17/lastContentType` under the local name
`save-preferences`. Native lexical scope suffices for local lookup once the host
has admitted the child's context. This self-contained manifest needs no module
map. If the locale resources move to separate files, native module-map resource entries
can resolve their specifiers. A map cannot express who may write a store or
which child receives a field override. Those checks belong to the proposed
grant and resource consumers. [Native scope and property inheritance](application.md#scope-syntax-and-property-inheritance-primary-candidate)

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

### Host-owned write outcomes and reconciliation

The application coordinator owns the invocation and its one terminal command
result. The host that admits a file write owns the record of what happened to
the file. Before dispatch, that host issues a fresh opaque `operationId` and
privately associates it with the original invocation and its caller. The host
retains enough of that association to settle the request if the executing
editor closes. Keep paths, handles, contents and arguments out of the journal.

If the editor closes after the write commits but before acknowledgment, the
file service can still supply evidence for that operation to the surviving
host. Closure ends the editor’s UI delivery. A still-admitted original caller,
such as the host File menu, can receive its permitted result once. Check its
pinned instance, generation, epoch and bridge before delivery. Closing,
reopening or cloning the editor cannot transfer its old subscription.

| Evidence the host can establish | Journal outcome | Command meaning |
| --- | --- | --- |
| The service confirms the pinned revision was written for this operation | `acknowledged` | Report `succeeded` if the invocation is still pending. |
| The service confirms this operation made no change | `not-applied` | Report `failed`, or `cancelled` for a known pre-commit cancellation. |
| The receipt is missing and the service cannot establish the outcome | `unknown` | Keep the request pending while awaiting evidence. If it must end, report `failed` with safe code `outcome-unknown`. |

A file’s existence, a timeout and an obsolete handler callback cannot establish
the outcome of a particular write. Only the trusted host reconciles the
operation, using independent service evidence and current journal authority.
Duplicate confirmation of the same outcome is idempotent. Reject contradictory
receipts and preserve the established outcome. An unknown write stays out of
automatic retry and leaves the document’s saved revision unchanged.
[Host journal contract](../../../docs/committed-checkpoints.md)

An authorized status lookup is a read of the host’s latest evidence. If an
invocation already ended with `outcome-unknown`, later confirmation can make
that lookup report **Saved**. The original command result stays final. A
reconnected editor needs fresh, scoped read permission and a host-recognized
correlation. The lookup exposes neither the destination nor execution authority.
The v1 example projects only `invocationId` and `status`. Callers needing to
distinguish an uncertain write from a confirmed failure use the status lookup
or a versioned projection admitting the safe diagnostic code.

The existing checkpoint journal lives in private host-session memory. After
loss of that host, reconciliation requires an independently retained authorized
receipt or a service that can answer for the operation ID. Report **Save could
not be confirmed** when that evidence is unavailable or expired.

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
  start and close controls remain in the surviving control plane. A recognized
  manager binding in the focused scope hands off one admitted semantic command.
  Unhandled input never falls through to another app. Native editing and
  browser-reserved bindings retain their ordinary behavior.
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
correlation. Browser tests must exercise logical routing from a focused placement, observer slices,
two nested editors/applications, native command coexistence, menu/task focus
relay, parameter forms, invalid/cancel submits, repeat and activation-sensitive
host calls. Test resource-bound actions and menu items as separate declarations.

Include selection changes while menus are open; removed targets while dialogs
wait; successful and failed Save; separate Copy invocations with identical
parameters; locale change during execution; sensitive-value redaction; no
execution on rerender/SSR/hydration; and accessible names, shortcuts, state,
focus, RTL/long labels, inherited themes and forced colors. These are future
acceptance cases, not fixtures or tests delivered by this design-only change.

Exercise explicit, same-ID and generated form selection. With a host menu and
two children exporting `file.save-as`, confirm that only the selected child's
form opens. Change active child and locale while editing, then submit or cancel.
Verify one result for the original invocation and zero effects on the other
child. Cover a missing custom form, duplicate or invalid bindings, unsupported
required types, target closure and preservation of outside focus.

Project A's editor into a layout with its own Save As handler and invoke A
from the host menu. Exercise an owner guard refusal and a successful save in
both light DOM and an iframe. Verify the layout gains zero observations, vetoes,
claims or results through placement. The caller gets one permitted status
without a private destination. A disabled nearest provider ends lookup. Changing
the target's logical owner during parameter entry prevents execution.

In both light DOM and iframe hosts, exercise custom and generated forms with a
recognizable destination. Type it, trigger validation, change locale, then
cancel. Check event projections, snapshots, diagnostics and checkpoint output
during editing and after teardown. On success only the file service receives
the destination for execution. Exercise picker denial, failed stream close,
download handoff and edits made after dispatch. Verify the reported outcome,
the document’s destination and its saved revision in each case.

Hold the service receipt until the write commits and the editor closes. Verify
one write, one host outcome and zero callbacks to the closed or replacement
editor. Exercise a still-admitted host-menu caller, duplicate and contradictory
receipts, confirmed failure, missing evidence, denied status lookup and host
loss. Reconnecting can read only its granted projection and never reissues Save.

Cross-app acceptance also requires exported-child Save targeting, denied upward
context writes, allowlisted parent services, sibling isolation, bridge-loop
refusal, same-ID/different-instance state, stopped-app refusal without replay,
and stale-generation rejection after pause/load/clone. MDI manager commands are
tested independently from business handlers with different focused and selected apps.
Verify private defaults, denied self-joining of a named group, admitted
per-family sharing without handler leakage, and equivalent command API outcomes
in light-DOM and iframe hosts. Guarded framework service calls must never fall
back to another app's browser/provider state.
