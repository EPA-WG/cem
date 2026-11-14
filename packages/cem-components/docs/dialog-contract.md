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
| `closedby` | Forwarded native close policy. |
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
explicit `popover` for top-layer nonmodal tasks. Replacing modal dialogs with
popovers would require recreating browser focus containment and background
inertness. It offers no benefit to the current shared-owner model. This is an
assessment and follow-up plan; it does not change the shipped declarations or
their default presentation.

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

Neither dialog declaration currently authors a `::backdrop` rule. In the tested
engines, modal backdrop color is the user-agent `rgba(0, 0, 0, 0.1)` and popover
backdrop color is transparent. A computed pseudo-element color on a local
nonmodal dialog does not establish that a backdrop is painted. Current modal
scrim paint is therefore browser-owned, rather than a verified CEM theme rule.
Before styling it, select or add a canonical theme-owned scrim definition through
the source-table/manifest process. Consume it from declarative modal-only CSS;
do not introduce component-local public defaults or reuse elevation as z-index.
Verify six themes, inherited overrides, nested backdrops and forced colors before
claiming a theme-consistent scrim. Nonmodal popovers should stay undimmed unless
an independently specified use case requires otherwise.

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

1. The shared nonmodal Escape adapter requests closure even with explicit
   `closedby="none"`. The unadapted native probe stays open in both engines.
   Make the adapter honor explicit close policy while preserving the established
   CEM default Escape behavior and explicit cancelable close requests.
2. Closing a persistent nonmodal task after outside focus leaves focus outside
   in Chromium but returns it to the launcher in Firefox. The probe intercepts
   the launcher's JavaScript `focus()` method and confirms zero shared return
   calls in this path; the unadapted native reference reproduces the same engine
   difference. Preserve the current outside destination without adding a second
   general focus owner.
3. Native close-policy support and request APIs have independent compatibility
   histories. The adapter currently calls `requestClose()` directly. Add bounded
   capability handling without bypassing a close veto through forced `close()`.
   Do not claim `closedby` behavior based only on dialog/popover availability.
4. An auto/hint hide cannot promise veto. Keep guarded tasks on modal/native
   request or manual-popover paths; any future declarative guard configuration
   must reject an incompatible auto/hint guarantee before opening.

The observation probes log policy deviations instead of accepting them as
contract assertions. Convert them to regression assertions when the shared fixes
land. The [dialog compatibility data](https://github.com/mdn/browser-compat-data/blob/main/html/elements/dialog.json)
and [API compatibility data](https://github.com/mdn/browser-compat-data/blob/main/api/HTMLDialogElement.json)
were checked during this assessment. Installed WebKit cannot launch because host
libraries are missing; Safari, assistive technology and physical Back/dismiss
gestures are unverified. No modal emulation fallback is recommended.

```bash
CEM_SURFACE_FIREFOX=1 yarn nx run @epa-wg/cem-components:verify-surfaces --excludeTaskDependencies --skipNxCache
```

Implement shared close-policy/capability handling first, outside-focus recovery
second and canonical scrim styling third. Add colocated production-dialog cases
for each change; retain browser-owned modality and the existing default API.
