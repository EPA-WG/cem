# CEM interaction design: actions, menus, popups, and dialogs

**Status:** Accepted design, promoted by user instruction on 2026-10-03. The public naming, defaults, relationships, and behavior contracts below are authoritative for future implementation of this interaction model. This document supersedes candidate naming in the [research proposal](cem-popup-action-dialog-proposal.tmp.md). Acceptance does not claim implementation or change currently shipped APIs; compatibility and migration require a separate plan.

**Scope:** authoring API, native lowering, relationship resolution, semantics, focus, geometry, lifecycle, diagnostics, and verification. Implementation and migration sequencing are subsequent tasks. Architecture derives from fresh research and accepted discussion, not existing CEM separation.

## 1. Invariants and component roles

An action requests an operation; the surface owns actual visibility. Business checked values are separate. Invoker, target, anchor, fitting boundary, and return-focus destination are distinct relationships. Semantics and presentation are independent, with validated combinations.

All four forms remain first-class: compound popup, inline dialog trigger, independent targeting, shared descriptor. Native markup is first-class and requires no CEM wrapper. Convenience expands to equivalent explicit relationships, with one transition per activation. Recursive menus wire their own submenu presentation.

| Concept | Responsibility |
| --- | --- |
| `cem-action` | Native control-backed command, submit/reset, or navigation presentation; invoker is its actual control |
| `cem-popup` | Optional local composition creating/adopting a surface and wiring its trigger |
| `cem-menu` | Command composite, persistent or popup-presented; direct nested menus form submenus |
| `cem-dialog` | Named task content with modal or non-modal presentation |
| `cem-tooltip` | Noninteractive description with interest-driven visual presentation |
| `cem-interaction` | Inert shared target/command/context descriptor; never visibility owner |
| Surface | Exact native popover element, dialog, or inline region |
| Semantic owner | Exact declared menu/dialog/tooltip/disclosure content provider |
| Session | Interaction lifetime containing source, context, logical parent, and return destination |

Ordinary navigation links retain native navigation/disclosure semantics; a popup does not make them command-menu items. Shared browser behavior lives in `cem-elements`, consumed declaratively through XHTML/CEM-ML. Native owners and identity survive rerender; an active modal owner is never detached merely to update its body.

Examples are HTML API illustrations, not executable fixtures or CEM-ML conversions.

## 2. Public naming

### Relationships and commands

| Attribute | Applies to | Meaning |
| --- | --- | --- |
| `trigger` | Popup/dialog/menu | Literal plain-text generated button label |
| `trigger-for` | Popup/dialog/menu | One unprefixed element ID naming an external invoker/control provider |
| `interaction-scope` | Participating container | Presence establishes local registry |
| `interaction-name` | Surface/descriptor | Unique name in nearest explicit registry |
| `surface-id` | Surface provider | Explicit ID on the produced native owner, for native external invokers |
| `command-target` | CEM action/descriptor | CEM reference: `@local-name` or `#element-id` |
| `command` | CEM action/descriptor | Native built-in or namespaced custom command |
| `interaction` | CEM action | `@name` or `#id` naming shared descriptor |
| `context-key` | CEM action/descriptor | Opaque item key captured at invocation; declaratively bindable |
| `parent-item` | Independent submenu | `#id` naming parent control/provider |

Do not use `action` as a trigger alias. Do not overload native link `target`, form `name`, or stylesheet `scope`. Values are references/plain data, not selectors or a new expression language. Rich invocation context uses native CEM data bindings.

Native `commandfor`, `popovertarget`, `popovertargetaction`, `interestfor`, and `form` preserve browser syntax and are forwarded unchanged to the actual control. A native built-in command targeting a custom host is not automatically redirected to a descendant owner: use `command-target` for CEM resolution or target an actual native owner.

### Surface properties

| Attribute | Values | Rule/default |
| --- | --- | --- |
| `kind` | `menu`, `dialog`, `tooltip`, `disclosure`, `none` | Popup-only profile; infer from one designated semantic owner; arbitrary content requires explicit kind |
| `mode` | `modal`, `nonmodal` | Dialog-only; new design default nonmodal |
| `orientation` | `vertical`, `horizontal` | Menu-only; default vertical, nested menus default vertical independently |
| `show-delay` | Nonnegative integer milliseconds | Pointer interest only: submenu 150, tooltip 500 |
| `hide-delay` | Nonnegative integer milliseconds | Pointer interest only: submenu/tooltip 100 |
| `label` | Plain text | Accessible name; native naming/heading precedence below |
| `default-open` | Presence boolean | Initial-open request once, not ongoing writable state |
| `presentation` | `inline`, `local`, `top-layer` | Derived from semantic/native owner |
| `anchor` | `invoker`, `pointer`, `selection`, `#id` | Relationship-derived; centered task has no required anchor |
| `placement` | Logical side + alignment, or `center` | Grammar in section 9 |
| `boundary` | `viewport`, `#id` | Viewport; child menus inherit explicit parent boundary |
| `fallback` | Comma-separated placements | Ordered opposite-side profile default |
| `overflow` | Tokens `flip shift resize`, or `hide` | Default flip shift resize; hide cannot mix with others |
| `anchor-lost` | `close`, `freeze` | Attached transient: close; persistent task: freeze |
| `focus-target` | `auto`, `none`, `#id` | Semantic default; explicit target inside surface |
| `return-focus` | `auto`, `none`, `#id` | Reason-aware auto restoration |
| `materialize` | `eager`, `retain`, `dispose` | Eager children: eager; designated template: retain |
| `context-change` | `reject`, `replace`, `request` | Reject different item in open task by default |
| `close-label` | Plain text | Optional generated named close control |
| `dismiss` | `chain`, `self`, `none` | Menu leaf policy; transient leaves default chain |

Proposal aliases are replaced: `title` → `label` for task naming; `target` → `command-target`; local `name` → `interaction-name`; `lazy-retain` → `materialize="retain"`; `--show` → `--cem-show`. HTML `title` retains advisory tooltip meaning. Native popover modes and dialog `closedby` remain supported browser configuration on their native owners.

### Slots and produced parts

| Hook | Contract |
| --- | --- |
| Default slot | Eager body/content |
| `slot="trigger"` | One explicit invoker/provider replacing generated button |
| `slot="body"` | One designated root/template; exclusive with meaningful default body |
| `slot="surface"` | Explicit native surface or semantic surface provider to adopt |
| `slot="heading"` | Dialog heading with stable ID naming the owner |
| `slot="close"` | Explicit close control replacing generated close button |
| `part="trigger"`, `part="surface"`, `part="body"`, `part="heading"`, `part="close"` | Component-owned nodes only, when needed |

Projected roots keep their slot hooks; no wrappers solely to manufacture parts. Generated trigger customization is limited to `trigger-aria-label` and presence-only `trigger-disabled`; richer requirements use an explicit control. Loading/error visual slots are deferred.

## 3. Resolution and precedence

`@name` resolves only within the nearest explicit `interaction-scope`, without ancestor/document fallback. `#id` uses element-ID resolution within the same DOM tree; no iframe crossing. Names are case-sensitive nonempty tokens of letters/digits/underscore/hyphen/dot, unique across surfaces and descriptors in each registry. Native IDs retain normal ID rules.

Scopes are explicitly participating containers, not every menu/dialog. Forward references may resolve during a connection batch; unresolved targets are diagnosed after the batch settles and cannot execute surface commands. Dynamic removal invalidates bindings and cleans up sessions. Repeated scopes allocate stable unique native IDs through rerender/resume.

| Combination | Resolution |
| --- | --- |
| Label shorthand + trigger slot | Slot replaces generated control |
| External trigger reference + local trigger | Conflict |
| No trigger | Generate none; independent/model invocation allowed |
| Empty trigger label | Require nonempty trigger-aria-label or diagnose |
| Default body + explicit body | Conflict except formatting whitespace/comments |
| One designated semantic owner | Infer profile |
| Explicit kind + conflicting owner | Diagnose; never force role rewrite |
| Compatible native + convenience relationship | Adopt one native route |
| Native route + command-target on same control | Reject dual routing; choose one |
| Descriptor defaults + explicit action command/context | Action overrides; explicit target must match or diagnose |
| Explicit anchor/return target | Override only that relationship |

## 4. Four authoring forms

### Compound popup

```html
<cem-popup trigger="Tools" placement="block-end start">
  <cem-menu>
    <cem-action>Refresh</cem-action>
    <cem-action>Inspect</cem-action>
  </cem-menu>
</cem-popup>
```

Menu semantics are inferred; no kind repetition. The popup supplies trigger/surface, the menu supplies navigation. An existing dialog provider is adopted, not wrapped in a second opening owner.

### Inline dialog

```html
<cem-dialog trigger="Edit" label="Edit invoice" close-label="Close">
  <template slot="body">…editor fields…</template>
</cem-dialog>
```

Trigger and native dialog are siblings; body materializes on first open and remains retained. Default nonmodal means persistent native `.show()`, not outside-dismissal popover behavior.

### Independent local target

```html
<section interaction-scope>
  <cem-action command-target="@editor" command="--cem-show"
              context-key="invoice-42">Edit</cem-action>
  <cem-action command-target="@editor" command="--cem-show"
              context-key="invoice-42">Review</cem-action>
  <cem-dialog interaction-name="editor" label="Edit invoice" close-label="Close">
    …editor fields…
  </cem-dialog>
</section>
```

Multiple launchers observe actual state; accepted invocation determines source/context. Different context cannot replace an open task under default reject policy.

### Shared descriptor

```html
<section interaction-scope>
  <cem-interaction interaction-name="edit" command-target="@editor"
                   command="--cem-show"></cem-interaction>
  <cem-action interaction="@edit" context-key="invoice-42">Edit</cem-action>
  <cem-dialog interaction-name="editor" label="Edit invoice">…</cem-dialog>
</section>
```

Descriptor is inert metadata. Descriptor-to-descriptor targeting is invalid. Shortcuts, permissions, and asynchronous saving are deferred rather than implicit descriptor behavior.

## 5. Native owners and command lowering

A CEM surface provider exposes exactly one stable native owner; an action provider exposes exactly one stable interactive control. Ordinary authored native controls/surfaces participate directly. Ambiguous control/owner discovery requires explicit designation, not a first-descendant guess.

| Command | Contract |
| --- | --- |
| Native popover commands | Require compatible native popover owner; preserve browser default |
| `show-modal` | Require compatible native dialog and declared modal profile |
| `close`, `request-close` | Native dialog lifecycle, preserving forced/request distinction |
| `--cem-show` | Semantic show request; idempotent, focuses existing task on explicit repeated activation |
| `--cem-toggle` | Resolve against actual state; closing passes through request policy |
| `--cem-request-close` | Policy-aware cancellation request |
| `--cem-close` | Explicit forced closure bypassing task guard |

With `command-target`, built-ins resolve/lower to the actual native owner ID; custom CEM commands target the stable provider endpoint. Generated triggers choose a native route when it preserves full policy, otherwise a shared adapter. Generated close controls use native request-close for ordinary native dialog cancellation, with adapters for other policies.

Native `commandfor` is forwarded unchanged and is never rewritten to descendant IDs. `surface-id` exposes a predictable authored ID on the exact native owner without moving the provider host ID. It must be unique within the tree; an adopted native owner with a conflicting ID is diagnosed rather than renamed. Built-in commands targeting unsuitable hosts produce a diagnostic for participating CEM relationships. Unknown application custom commands remain available and are not interpreted as popup requests without a registered provider.

Choose one route before activation. Supported native behavior executes through the browser. A fallback prevents/intercepts that route and performs the equivalent once. No click listener toggles again after native default. Native Enter/Space remain native; menu arrows supply separate navigation behavior.

Native presentation hosting CEM menus records the actual invoker and applies menu entry focus once. Popover commands alone do not implement menu navigation. An integration cannot assume the currently focused element is the invoker of every pointer opening.

Standalone native HTML remains complete where its semantics suffice:

```html
<button type="button" popovertarget="links">Links</button>
<div id="links" popover="auto"><a href="/settings">Settings</a></div>

<button type="button" commandfor="confirm" command="show-modal">Confirm</button>
<dialog id="confirm" aria-labelledby="confirm-heading">
  <h2 id="confirm-heading">Confirm change</h2>
  <form method="dialog"><button value="cancel">Cancel</button></form>
</dialog>
```

The built-in command set/custom namespace follows the [HTML button contract](https://html.spec.whatwg.org/dev/form-elements.html#the-button-element). Popover state/modes follow the [popover standard](https://html.spec.whatwg.org/multipage/popover.html); native cancel/close/form results follow [dialog lifecycle](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element).

Native controls can also target a CEM-produced owner explicitly:

```html
<button type="button" commandfor="editor-surface" command="show-modal">Edit</button>
<cem-dialog surface-id="editor-surface" mode="modal" label="Edit invoice"
            close-label="Close">…editor fields…</cem-dialog>
```

Here the browser command names the produced native dialog; the CEM host is not the native command target. Eager server rendering can emit the owner before scripts run. Client-only declarations require the runtime to produce it first.
## 6. Profiles and accessible naming

| Profile | Default owner/presentation | Initial focus | Closing defaults |
| --- | --- | --- | --- |
| Persistent root menu | Semantic menu in flow | No connection-time movement | Root remains; transient children close |
| Popup menu/submenu | Native auto popover with semantic menu | Enabled item on keyboard opening; pointer policy below | Escape, outside, Tab, accepted leaf command |
| Nonmodal dialog | Native `.show()` dialog, local | Meaningful task target | Explicit cancel/close, persistent outside |
| Modal dialog | Native `.showModal()` dialog, top layer | Meaningful task target | Native close request; outside close only declared |
| Nonmodal popover dialog | Native dialog with authored popover mode | Meaningful task target | Native mode-specific dismissal |
| Tooltip | Nonfocusable description visual | Retain invoker | Interest end/Escape |
| Disclosure | Inline/local region or authored native details | Retain control | Toggle, profile-specific Escape |
| `none` popup | Generic nonmodal popover | Retain source unless autofocus/explicit focus-target | Native popover mode |

Dialog shown as popover is governed by popover visibility, not dialog.open. Do not close it through dialog.close or assume method=dialog hides popover-only presentation. Modal profile cannot also declare popover or disable modal containment. Nonmodal Escape is supplied explicitly through supported closedby=closerequest on the native owner or a shared adapter; native show alone is not assumed to provide Escape cancellation.

A guarded task cannot promise veto of native auto/hint-popover hiding. Reject that combination; use a manual explicit-close surface or native dialog request lifecycle. Direct native forced close and dialog-method results are not retroactively vetoed. Manual-popover command-menu composition can request menu navigation/dismissal through the runtime, but is not a way to override browser auto-popover grouping rules.

Name precedence: compatible aria-labelledby, compatible aria-label, designated heading, label. Conflicting declarations diagnose rather than overwrite. Generated trigger text may name a menu but never silently becomes a task title. Tooltips preserve existing aria-describedby tokens and keep description content stable independently of visual materialization. Interactive tooltip content is invalid; use a task/preview profile.

CEM owners forward authored native autofocus, closedby, role, naming, and popover attributes only to their applicable native owner. Invalid combinations are reported. A static region must not claim aria-modal=true without actual modality.

## 7. Recursive menus

```html
<cem-menu trigger="Tools">
  <cem-action>Refresh</cem-action>
  <cem-menu trigger="Export">
    <cem-action>PDF</cem-action>
    <cem-menu trigger="Image">
      <cem-action>PNG</cem-action>
      <cem-action>JPEG</cem-action>
    </cem-menu>
  </cem-menu>
</cem-menu>
```

Root with local trigger defaults to popup presentation; without trigger/popup declaration it is persistent. An explicit compatible presentation overrides this default. Persistent horizontal command roots expose menubar semantics; vertical roots and popup children expose menu semantics with their declared orientation. A direct nested menu forms a submenu: its trigger belongs to parent navigation, its body owns child navigation. It needs a local trigger or explicit parent-item. Arbitrary descendant menus inside unrelated task content do not become submenus.

An independent child uses parent-item="#export-control". Parent control's menu must be unambiguous. Each item owns at most one submenu; cycles/multiple parents are invalid. The logical declaration supplies the relationship, not a second activation route. Keep descendants of a popup inside a modal dialog within that dialog's interactive DOM subtree.

Child defaults inherit boundary, direction, and hover timing. show-delay/hide-delay affect pointer interest only; keyboard activation has no delay. Touch activation remains explicit, not synthetic hover. Parent orientation supplies attachment. Opening closes sibling branches and descendants. Keyboard opening enters first enabled item, or last for explicit reverse-entry. Click opening enters first enabled item; hover opening never steals keyboard focus. Pointer/focus interests are tracked separately; a delayed pointer branch switch cannot invalidate a keyboard-active branch without an explicit transition. Pending timers cancel on dismissal; pointer travel across the trigger/panel gap preserves the branch.

Escape closes deepest child once to parent item; root Escape restores launcher. Tab leaves composite and dismisses transient branches without restoring over destination. Leaves use dismiss=chain/self/none; only accepted leaf activation dismisses, not arbitrary bubbled clicks. Chain closes transient ancestors while persistent root remains. Native auto-popover mode may inherently hide related descendants/siblings; dismiss=self/none cannot promise to defeat those browser rules.

Expanded submenu state uses aria-haspopup=menu/aria-expanded, not business aria-checked. Parent item enumeration excludes descendants owned by child menus. Disabled/hidden items do not activate; empty/all-disabled branches do not invent item focus stops. Menu navigation follows [APG menu/menubar](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/) and [menu-button entry](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/), including orientation-specific opening and closing arrows.

## 8. Invocation, focus, and state

Capture source/context/logical origin before hiding ancestors. Menu-to-dialog handoff suppresses menu restoration and performs task focus once; dialog exit returns to stable root launcher/workflow destination, not hidden child item. Outside clicks/Tab preserve destination; Escape/cancel restore when valid. Keep browser restoration where correct, compensating only for exceptional handoffs.

focus-target=auto selects eligible authored autofocus first, then semantic target. A complex task may focus an internal heading with tabindex=-1; never add tabindex to native dialog itself. Explicit focus target must be connected/visible/enabled and inside surface; invalid target diagnoses and uses safe semantic fallback. A modal none override cannot prevent valid native focus entry. Named static modal entry follows [APG dialog guidance](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/).

return-focus=auto selects eligible invocation control, stable ancestor launcher, or declared workflow fallback. Do not invent focus on hidden controls or arbitrary body if none exists. Programmatic opening supplies a fallback when restoration matters. Pointer geometry is never a focus destination.

An invocation contains transient source/control references, command, context key, native CEM context reference, input kind, logical parent, and return destination. These browser references are not portable data snapshots. Same-key re-show is idempotent; different-key open uses context-change. reject leaves existing task intact; replace explicitly permits rebinding according to the application's data/draft contract; request emits a cancelable context-change decision.

For request, cancellation rejects; synchronous non-cancellation accepts, matching normal cancelable-event semantics. An asynchronous decision must prevent the request and later reissue an explicit accepted replacement; silence/time passing is never asynchronous approval. Default reject protects tasks without requiring application-local UI handlers.

State machine:

```text
closed → preparing → open → closing → closed
             ↓         ↑
           closed    canceled close stays open
```

preparing may have no visible owner; closing may retain an exit animation after native hide. Actual visibility comes from :popover-open, native dialog.open, or inline region state, not animation phase. Failure returns to closed; launchers do not become expanded for invisible preparation.

Read-only public data-state=closed/preparing/open/closing exposes phase on participating CEM surface hosts; exact action controls expose derived ARIA. Not a writable state API. default-open initializes once per new instance and does not replay on rerender/resume. Models issue commands through generic declarative bindings, not mutate derived attributes.

Accepted re-show from a different source updates invoker/return destination and optional anchor; context changes still obey policy. Anchor must not silently move merely because another non-activating control observes open state. Tooltip interest and committed activation are separate causes; pointer leave cannot close an activated task.

## 9. Placement and native fitting

placement grammar: center or `<side> <align>`, side = block-start/block-end/inline-start/inline-end, align = start/center/end. Alignment follows the perpendicular logical axis. No physical aliases initially; inherited direction and writing mode determine mapping.

Defaults: root menu block-end start; vertical child inline-end start; horizontal menubar child block-end start; tooltip block-start center; task dialog center. Centered content has no required anchor. Attached content defaults invoker. Explicit pointer/selection uses geometry captured by invocation adapters; keyboard context-menu invocation derives owner geometry instead of stale pointer coordinates. Missing explicit pointer/selection geometry diagnoses and rejects opening.

fallback lists full placements in order. flip tries alternatives; shift moves within bounds; resize constrains inline/block size and allows appropriate internal scrolling. Oversized content resizes before final shift. hide is an explicit overflow concealment policy only for non-task/nonfocused transient presentation; it cannot report a successfully entered task that remains invisible.

Use native CSS anchors and position-try where their containing block expresses the requirement. For arbitrary container-boundary fitting of top-layer content, intersect measured boundary with visual viewport and use shared measured geometry when required. Observe relevant resize/scroll and visual viewport changes. Top-layer painting does not require DOM reparenting. CSS fitting details follow [CSS Anchor Positioning](https://drafts.csswg.org/css-anchor-position-1/).

Stable scoped owner identities prevent cross-anchoring repeated instances. Expose known actual placement through read-only data-placement on native owner using the same grammar; do not mutate preference. Arrows/animations are optional. Paint remains declaration-owned, using theme-owned CEM tokens; this design adds no component token defaults.

presentation=local does not provide top-layer clipping escape; presentation=inline does not imply floating geometry. Reject incompatible modes such as modal local presentation. Persistent nonmodal native show is local; top-layer nonmodal task requires popover presentation explicitly.

## 10. Materialization, teardown, and SSR

A designated direct template[slot=body] defaults materialize=retain. Eager children default eager. Explicit eager can materialize a template before interaction; dispose recreates it per session. retain/dispose without a template is invalid; do not implicitly destroy live authored children.

Keep target shell/native owner stable, mounting body only. Opening captures context/session identity, resolves materialization/resources, checks freshness, opens native owner, establishes semantic focus, and synchronizes actual state. Async preparation retains launcher focus and busy state; loading visual surfaces are deferred. Close during preparation invalidates the request; late results cannot reopen it.

Close ordering: request/cancel decision → native hide/close → actual-state/ARIA reconciliation → focus restoration/handoff → exit animation end → eligible body disposal. Nested sessions clean up first. Disconnect cleans active native owners and releases registries, listeners, observers, and resources. Disposal cannot silently lose form drafts; opting into dispose is explicit, and retained application draft state is separately owned.

Portable data/context/resources stay in native CEM lifecycle channels. Session metadata is runtime machinery, not a replacement DOM/JSON data store. Hydration retains semantic state and stable identities but never serializes live browser references. Reconnect resolves relationships and reads native state before acting. Modal resume needs a browser reopen transaction, not merely setting open (which is nonmodal).

No-script support requires eager pre-rendered native markup with IDs. SSR convenience lowering emits native relationships when the selected profile is natively expressible. Custom commands, unlowered local names, and lazy templates do not execute without runtime JavaScript. Essential tasks provide native eager or normal-link fallbacks. No component-authored JavaScript is distinct from no runtime JavaScript.

The shared implementation must respect the [durable CEM instance lifecycle](cem-element-lifecycle-principle.md) and [declarative UI boundary](declarative-ui-principle.md). Those constrain implementation ownership, not the design's semantic decomposition.

## 11. Events, cancellation, and native forms

| Event | Meaning | Cancelable |
| --- | --- | --- |
| cem-before-open | Participating request before preparation/opening | Yes when route can be intercepted |
| cem-open | Exact native owner became visible | No |
| cem-before-close | Policy-aware close request; native cancel bridge | Request paths only |
| cem-close | Exact owner became hidden/closed, with reason | No |
| cem-context-change | Different-key request under request policy | Yes |
| cem-interaction-error | Failed resolution/preparation/open | No |

Events bubble from the semantic surface provider, or native owner for wrapper-free enhanced composition. Resolution errors without a valid target bubble from the invoker provider instead. Local synchronous policy listeners receive request events before generic observation. A composition adopting another provider does not duplicate outcome events from its wrapper. The native source/control is included even when content is logically elsewhere.

Native events remain available. CEM observation never replays a command or fabricates cancelability where the browser provides none. Toggle coalescing must not delay actual-state/ARIA synchronization. Idempotent requests do not emit new open outcomes. If a native rapid transition cannot be observed independently, report the actual observed state rather than inventing intermediate events.

Metadata names: source, command, contextKey, reason, sessionId. Browser references are transient metadata only; richer native CEM data stays typed in its channel. Initial reasons: activate, escape, outside, tab, select, close-control, command, form, ancestor-close, anchor-lost, disconnect. Unknown native reason is native, not guessed. Dialog native returnValue may accompany form/close results; closing is not application save success.

Supported CEM close requests bridge native dialog cancel exactly once. Guards use shared declarative cancellation capabilities, not component/application-local listener workarounds. Forced native close and auto-popover hide are not promised vetoable. Every task must have an accessible keyboard/touch exit/completion route, even if close-label is omitted.

Native submit/reset controls retain form ownership, validation, and result behavior. Reject them as automatically wired popup launchers: validation-gated opening uses an accepted form event through generic declarative binding. method=dialog returns a native result without server persistence and does not hide popover-only dialog presentation. Async save-then-close orchestration is deferred; no rule says every submit closes.

## 12. Diagnostics

Use stable names with source location and related declarations:

- interaction-reference-missing, interaction-name-duplicate, interaction-reference-conflict.
- interaction-trigger-ambiguous, interaction-control-unsupported, interaction-surface-ambiguous.
- interaction-profile-conflict, interaction-command-incompatible, interaction-dual-route.
- interaction-submenu-cycle, interaction-submenu-parent-ambiguous.
- interaction-body-conflict, interaction-materialization-invalid.
- interaction-focus-target-invalid, interaction-anchor-unavailable.
- interaction-context-rejected, interaction-open-failed.

Do not silently change semantic kind, overwrite authored roles, or bind the first eligible descendant. Disable only an invalid surface-command relationship, preserving unrelated valid native form/link behavior. Generated invalid controls must not imply an operable popup. Unknown custom application commands are not automatically errors.

## 13. Acceptance matrix and delivery scope

All seven adopted conveniences are specified: semantic inference; inherited submenu context; invocation/focus-return capture; close affordance; template-retained body; leaf dismissal; local names.

| Area | Required evidence |
| --- | --- |
| Native route | Commands/popover targets execute once; independent native HTML needs no wrappers |
| Convenience equivalence | Generated/slotted/external/local-name/descriptor forms resolve equivalent relationships |
| Menu hierarchy | Three levels, sibling replacement, disabled/empty children, deepest Escape, Tab, leaf policy |
| Focus relay | Menu-to-dialog enters once, restores stable launcher, preserves outside destination |
| Native dialogs | Modal inertness/containment; nonmodal persistence; popover visibility distinction; cancel versus force |
| Context | Same-key repeat; different-key reject/replace/request; repeated scope isolation |
| Body lifecycle | Retain identity; disposal ordering; preparation cancellation; stale-result rejection |
| Rerender/resume | Stable native owner; disconnect cleanup; no replay of default-open |
| Geometry | RTL/vertical writing, container bounds, pointer/selection, mobile viewport, lost anchors |
| Accessibility | Names on exact owner/control; independent checked/expanded values; stable tooltip description |
| Forms | Native validation/results; no accidental submit or implicit save |
| No script | Eager lowered native output; essential task fallback |

Implementation adds actionable fixture checklist items before creating fixtures, follows native tests-first when transformation semantics change, uses colocated component story plays, and runs appropriate Nx checks. This documentation task creates no fixtures and requires no implementation test runs.

Deferred: async service completion, loading/error visual slots, global shortcut/permission management, general rich preview profile, animated arrows, checkbox/radio authoring beyond generic selection capabilities. Existing API migration/compatibility needs a separate inventory and transition plan; it does not dictate this design.

## 14. Accepted decisions

Public names and contracts in this document are accepted. The accepted defaults are: persistent nonmodal dialog; strict nearest-scope names with explicit @/# references; body-template retention with disposal only when requested; recursive command menus without popup hosts.

Future implementation must follow these names and contracts. Changes to the accepted design must be explicit document revisions rather than implementation-local deviations. Design acceptance remains distinct from implementing these APIs or changing currently shipped behavior. Standards sources define native behavior; CEM defaults and extra relationships are design decisions. Browser support must be checked against the supported version matrix before implementation, especially invoker commands, interest invokers, dialog closedby, and CSS anchors.
