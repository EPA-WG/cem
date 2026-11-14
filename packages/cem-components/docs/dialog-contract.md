# Native dialog contract

`cem-dialog` and the compatibility alias `cem-dialog-shell` are canonical XHTML
CEM-ML declarations using the shared `native-surface` capability. Each produces
one stable native `dialog[part="surface"]`. It starts closed; the default is a
persistent nonmodal task. `mode="modal"` explicitly requests browser modality.

```html
<cem-dialog label="Edit profile" trigger="Edit" close-label="Done">
  <label>Name <input autofocus name="name"></label>
</cem-dialog>
```

| Authoring input | Meaning |
| --- | --- |
| `label`, `slot="label"` | Visible heading and default accessible name. Explicit `aria-labelledby` or `aria-label` overrides generated naming. |
| `trigger` | Text for a generated native launcher; `trigger-aria-label` supplies its optional accessible name and presence-only `trigger-disabled` disables it. |
| `slot="trigger"` | One native control or shared control provider replacing the generated launcher. |
| `trigger-for` | Typed CEM node reference, `#id`, bare ID or nearest-scope `@name` for an external launcher. Do not combine it with a local trigger. |
| `close-label`, `slot="close"` | Generated close button or one authored replacement. Close requests use the shared cancelable lifecycle. |
| Default slot | Eager task body. Native form controls retain their values, identity, focus, validation and events across ordinary rerenders. |
| `template[slot="body"]` | Inert body with `materialize="retain"` (default for a template), `dispose` or `eager`. A template body cannot be combined with an eager body. Use a direct inert outer template to author a nested body template in the host payload. |
| `mode` | `nonmodal` (default) uses `show()`; `modal` uses `showModal()`. Change it while closed. |
| `popover` | Optional `manual` or `auto` top-layer nonmodal presentation. Modal plus popover is rejected. |
| `default-open` | Presence requests the initial opening once per produced instance. It is not live open state. |
| `surface-id` | Optional ID on the native owner for native `commandfor` relationships. |
| `closedby` | Forwarded native close policy. Explicit `none` suppresses shared Escape; explicit close controls still request cancelable closure. Policies differing from the native presentation default require `closedBy` support. |
| `--cem-scrim-color` | Inherited D0 modal veil color: black in branded themes, system `CanvasText` in native/forced colors. |
| `--cem-scrim-opacity` | Inherited D4 modal veil opacity, default `0.32`; applies only to the native modal backdrop. |
| `focus-target`, `return-focus` | Shared typed reference or scoped string; `auto` uses native entry and the actual invoker; `none` suppresses shared focus work. Native browser focus behavior still applies. |
| `anchor`, `boundary`, `placement`, `fallback`, `overflow`, `anchor-lost` | Shared surface geometry. Dialogs default to centered placement and freeze established geometry on anchor loss. |
| `context-change` | `reject` (default), `replace`, or cancelable `request` for an already-open task invoked with different context. |

`cem-action` commands `--cem-show` and `--cem-hide`, native commands such as
`show-modal` and `request-close`, launcher activation and close controls all use
the shared surface lifecycle. Native `form method="dialog"` retains validation
and `returnValue`. The shared `cem-before-open` and `cem-before-close` events are
cancelable; `cem-open` and `cem-close` report transitions. The runtime owns
preparation, placement grants, invocation context, nested menu-to-task focus
relay and cleanup; the component contains no behavior script.

A heading ID and launcher ARIA references are shared runtime claims. Rerender
preserves these claims; disconnect/rebinding releases only owned values.
Keyboard focus enters an authored eligible target or the browser's dialog
fallback. Only the actual focused owner/control receives its own token outline;
forced colors use `Canvas`, `CanvasText` and `Highlight`.

A conditional `default-open` dialog without an explicit invoker retains the
focused entry control as its return destination. Removing the dialog after a
service decision returns focus only when it still owned focus and that control
remains available. Focus moved elsewhere, an unavailable destination, and
`return-focus="none"` suppress shared restoration. The
[project navigation workflow](project-navigation-contract.md) demonstrates delayed
Save, failed Save, Discard and Cancel with declarative state and shared controls.

## Close-policy and request compatibility

For a native dialog without popover presentation, explicit `closedby="none"`
suppresses the shared Escape handler during preparation and while open. Native
enum matching is case-insensitive; surrounding whitespace is not stripped.
Absent, empty and invalid values retain CEM's established nonmodal Escape
behavior. Modal Escape remains browser-owned. `closedby` does not govern popover
visibility: a manual popover task retains its shared Escape behavior.

Explicit CEM close controls/commands and shared `requestClose()` calls remain
cancelable requests even with `closedby="none"`. If the native method exists,
the adapter uses it. If it is unavailable, the adapter dispatches one cancelable
native-owner `cancel` event through the existing CEM guard bridge and calls
native `close()` only after acceptance in the same connected session. Either a
native cancel listener or `cem-before-close` may veto. Reentrant shared requests
are refused; reopening, replacing context or disconnecting during cancellation
cannot authorize closure of a newer session. Vetoes preserve `returnValue`;
fallback `request-close` commands retain an authored button value. A throwing
request API reports `interaction-close-request-failed` and leaves the task open,
without retrying through forced close.

The core native `close()` method is required before opening. Its absence reports
`interaction-close-api-unavailable`. Without a string-valued native `closedBy`
capability, nonmodal `none` and modal `closerequest` still work because they match
the presentation's native default. Other recognized explicit policies report
`interaction-close-policy-unsupported` and refuse opening before visibility or
focus entry, including modal commands without an authored mode. This avoids
pretending that keyboard handling alone implements all native dismissal channels.
Omit an unsupported policy to use CEM's default task behavior; popover tasks keep
their independent native mode. Feature
masking in modern Chromium/Firefox verifies these paths; it does not certify all
historical browser versions or polyfill command/popover APIs.

Native forced `close()` and successful `form method="dialog"` remain separate
completion paths. This compatibility change preserves native modality, the
stable owner, focus references, form validation and popover presentation.

## Outside focus on nonmodal closure

The shared owner captures the element focused immediately before a native
dialog opens and snapshots an eligible outside destination when closure starts.
Some native nonmodal closes return focus to the original entry even after the
user has moved outside. The adapter compensates only when focus actually lands
on that exact entry in the same session/generation and the outside destination
is still connected, visible, enabled and focusable. The single repair uses
`preventScroll` and preserves the outside editor's value and selection. Engines
that already keep outside focus receive no additional focus call.

The snapshot is transient browser ownership, never serialized checkpoint data.
Closing/reopening or disposal cannot grant an older session focus authority.
Another active destination is preserved; a close callback may choose a new
workflow target. Native modal return and popover presentation keep their own
focus routes. `return-focus="none"` opts out of the repair as well as ordinary
shared return; native browser focus behavior still applies. Explicit return
references remain effective when closing from within the task.

Task-step replacement keeps owned focus for modals and manual task popovers:
when the focused control disappears and focus falls to the body/owner, the
shared owner selects a surviving eligible internal target. Deliberate outside
focus, a surviving focused control and `focus-target="none"` suppress that repair.
Local nonmodal `show()` dialogs retain their existing focus policy.

## Migration from legacy dialogs

Remove `transient` and live `expanded`. Use explicit `mode="modal"` where modal
behavior is intended, a launcher/command for ongoing activation, and
`default-open` only for initial opening. Replace `cem-dismiss` consumers with
the shared lifecycle or native dialog events. Static `div[role="dialog"]`
wrappers are retired; use a region component for permanently inline content.
`cem-sheet` remains a separate canonical labeled region with its existing
`transient`/`expanded` API.

Colocated stories cover names, native forms, modal/nonmodal state, slots,
retention/disposal, external references, reconnect and native keyboard focus.
The property pages and five-theme galleries are included in source, release
bundle and installed-package verification through `verify-surfaces`.

## Dialog/popover assessment — 2026-11-14

Recommendation: retain the existing three presentation choices. Use native
`showModal()` for blocking tasks, `show()` for persistent local tasks and an
explicit `popover` for top-layer nonmodal tasks. Blocking tasks retain browser
focus containment and background inertness through native modality. Shared
declarations keep their default presentation. Following the owner's subsequent
2026-11-14 instruction, Studio explicitly selects `mode="nonmodal"
popover="manual"` for its task dialogs, making the workbench available while
retaining guarded decisions and persistent top-layer presentation.

### Presentation and dismissal matrix

| Choice | Native visibility | Background and stacking | Dismissal and result |
| --- | --- | --- | --- |
| Default nonmodal | `dialog.open` after `show()` | Local stacking; background remains usable | Persistent outside; CEM supplies Escape while focus is inside. Native requests/forms retain dialog results. |
| `mode="modal"` | `dialog.open` and `:modal` after `showModal()` | Browser top layer; effective background inertness and native keyboard containment | Native close requests and cancel event. Outside close requires declared `closedby="any"` support. |
| `popover="manual"` | `:popover-open`; `dialog.open` remains false | Top layer, without background inertness | Explicit shared close request or CEM Escape; no native outside dismissal. Use for a nonmodal task requiring a cancelable CEM close guard. |
| `popover="auto"` | `:popover-open`; `dialog.open` remains false | Top layer, without modality; native popover grouping applies | Native outside/close-request dismissal can hide it without a CEM guard. Appropriate only when unconditional hiding is acceptable. |

These native distinctions follow the [dialog lifecycle](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element)
and [popover modes](https://html.spec.whatwg.org/multipage/popover.html#the-popover-attribute).
Keep one visibility owner. Do not combine modal mode with `popover`, synthesize
modality by setting `open`/`aria-modal`, move the live owner into a portal, or
replace it during a body update. Close a popover through its popover lifecycle;
`dialog.close()` and `form method="dialog"` do not hide popover-only presentation.
`hint` remains a native tooltip/ephemeral option rather than a recommended task
dialog presentation.

Native `closedby` distinguishes `any`, `closerequest` and `none`. Its automatic
default allows close requests for modal dialogs and no automatic user dismissal
for nonmodal dialogs. Programmatic `requestClose()` deliberately bypasses this
policy while retaining the cancel event. Forced `close()` and successful native
dialog-method form submission are separate completion paths. A closing
`beforetoggle` event cannot veto hiding; use a supported close request for guards.
See [closedBy](https://developer.mozilla.org/en-US/docs/Web/API/HTMLDialogElement/closedBy)
and [requestClose](https://developer.mozilla.org/en-US/docs/Web/API/HTMLDialogElement/requestClose).

### Focus, nested surfaces and accessibility

Preserve eligible authored autofocus or an explicit internal `focus-target`.
Complex tasks can focus a heading with `tabindex="-1"`; do not add tabindex to
the native dialog. Provide a meaningful accessible name and a keyboard/touch
completion or cancel control. Only an actual modal presentation may claim
`aria-modal="true"`; a dimmed nonmodal popover remains nonmodal. Screen-reader
and physical-device acceptance remain separate from these browser probes.
These recommendations follow the [APG modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/).

Focus return belongs to the invocation's current, available destination. Explicit
close/Escape may restore the launcher or workflow target. Outside navigation and
Tab must preserve the destination the user reached. `return-focus="none"`
suppresses shared restoration; it does not disable browser restoration. Removal
must not focus a detached, hidden, disabled or inert launcher. Menu-to-task
handoff keeps the stable root/workflow destination instead of a hidden leaf.

Keep nested menus, popover tasks and tooltips in the modal owner's interactive
DOM subtree. Top-layer rendering does not change DOM ancestry or remove modal
inertness from unrelated nodes. Dismiss the active child before its parent;
child Escape returns to its own trigger while the parent stays modal. Tooltips
retain source focus and do not trap it. Closing a parent releases child surfaces,
timers and geometry without a late child focus return. Existing shared
menu-to-task, native-surface and tooltip owners remain the integration point.

### Backdrop and theme ownership

The [layering contract](../../cem-theme/src/lib/tokens/cem-layering.md#10-scrim)
assigns the modal Command layer the scrim and modality, including when a child
Overlay is physically above it. A top-layer `::backdrop` is a paint surface, not
an inertness or focus mechanism; see the [CSS top-layer model](https://drafts.csswg.org/css-position-4/#backdrop).
Its presence must not be used to infer modal semantics.

Both dialog declarations consume canonical `--cem-scrim-color` (D0) and
`--cem-scrim-opacity` (D4) directly in `[part~='surface']:modal::backdrop`.
The datepicker's native modal calendar uses the same endpoints through
`[part~='dialog']:modal::backdrop`; it also remains a native modal owner.
The source tables, generator previews, manifest coverage and public exports own
the defaults; neither component defines an equivalent local alias or fallback.
The veil is black at 32% opacity in branded light/dark, contrast and projector
themes. Native and forced-color themes use system `CanvasText` at the same
opacity, with automatic forced-color adjustment and the modal's system contour.
The veil is supplemental to native inertness and focus containment.

The pseudo-element inherits the originating dialog's custom properties; a
container can override either endpoint without changing the dialog's own opacity.
Each nested native modal paints its own veil in native top-layer order, so their
translucency composites. A child nonmodal overlay adds no veil. Nonmodal local,
manual and auto presentations stay undimmed; a computed local-dialog pseudo
color alone does not imply a painted backdrop.

Native modal surfaces use `overscroll-behavior: contain`; their backdrops use
`overflow: hidden` and the same containment. This is progressive enhancement:
unsupported scroll containment does not change native modality. The component
does not lock `html`/`body` scrolling or synthesize a focus trap. See the
[browser inheritance guidance](https://developer.chrome.com/blog/css-backdrop-inheritance)
for backdrop inheritance and the native profile limits below.

Compiled source/bundle and isolated package fixtures verify all three declarations
in Chromium and Firefox: six themes, native light/dark system schemes, forced
colors, inherited overrides, nested modal pixel composition, transparent child
overlays, nonmodal pixels and nested theme re-entry. Colocated plays also verify
inherited paint and native focus return. See
[`modal-backdrop-checks.mjs`](../../../tools/cem-components/modal-backdrop-checks.mjs).
Wheel probes compare against an unadapted native reference with the same CSS.
Chromium 148 contains the page's scroll over both backdrop and non-scrollable
dialog content. Firefox 150 scrolls in both routes, matching that native
reference. This engine limit remains progressive enhancement; the scrim,
native modality and foreground contour are still verified in both engines.

### Evidence and implementation gaps

The repeatable [presentation checks](../../../tools/cem-components/dialog-presentation-checks.mjs)
run against compiled workspace and isolated packed runtimes through the existing
surface gate. They assert effective inertness with actual focus and pointer
attempts, outside persistence, popover visibility, one close guard per request,
Escape return, noncancelable hiding and nested child/tooltip dismissal. Effective
inertness is tested by behavior, not by the reflected `inert` attribute.

Chromium 148.0.7778.96 and Firefox 150.0.2 agree on that core presentation matrix.
Both expose `requestClose` and `closedBy`. Their observations also identify work
that must precede a broader dialog change:

1. The assessment found shared nonmodal Escape bypassing explicit
   `closedby="none"`. Fixed on 2026-11-14 by the
   [close-policy compatibility contract](#close-policy-and-request-compatibility).
   The compiled observation is now a regression assertion in both engines.
2. Closing a persistent nonmodal task after outside focus leaves focus outside
   in Chromium but returns it to the launcher in Firefox. The probe intercepts
   the launcher's JavaScript `focus()` method and confirms zero shared return
   calls in this path; the unadapted native reference reproduces the same engine
   difference. Fixed on 2026-11-14 by the bounded
   [outside-focus repair](#outside-focus-on-nonmodal-closure). The unadapted native
   comparison remains evidence; adapted workspace and packed runtimes now assert
   outside focus, selection and scroll preservation in both engines.
3. Native close-policy support and request APIs have independent compatibility
   histories. Fixed on 2026-11-14 with cancelable request fallback and explicit
   unsupported-policy refusal, verified against masked capabilities in both
   compiled engines. `closedby` behavior is not inferred from dialog/popover
   availability.
4. An auto/hint hide cannot promise veto. Keep guarded tasks on modal/native
   request or manual-popover paths; any future declarative guard configuration
   must reject an incompatible auto/hint guarantee before opening.

Outside-focus preservation, close-policy and API compatibility are regression
assertions. Unadapted focus differences remain reference observations. The
[dialog compatibility data](https://github.com/mdn/browser-compat-data/blob/main/html/elements/dialog.json)
and [API compatibility data](https://github.com/mdn/browser-compat-data/blob/main/api/HTMLDialogElement.json)
were checked during this assessment. Installed WebKit cannot launch because host
libraries are missing; Safari, assistive technology and physical Back/dismiss
gestures are unverified. No modal emulation fallback is recommended.

```bash
CEM_SURFACE_FIREFOX=1 yarn nx run @epa-wg/cem-components:verify-surfaces --excludeTaskDependencies --skipNxCache
```

Shared close-policy/capability handling and outside-focus recovery are complete.
Canonical scrim styling is also complete. Continue with the component-wide
modern-web-guidance audit; retain browser-owned modality and the default API.
