# Feedback Expanded Contract

**Status:** Accepted Phase 4 contract; the generic rendered-attribute ownership
boundary is implemented and verified, and the focused component fixture has
landed as passive passing coverage plus four executable expected failures. The
shared feedback behavior is the next work item. This contract is promoted by
[`docs/todo.md`](../../../docs/todo.md). No feedback runtime behavior has landed
yet.

## Decision

The existing feedback primitives remain static surfaces by default. A new
presence-only `transient` attribute opts `cem-dialog`, `cem-dialog-shell`, or
`cem-sheet` into an open/closed lifecycle, and presence-only `expanded` is the
current open-state input in that mode.

`cem-dialog[transient]` and `cem-dialog-shell[transient]` share one modal
lifecycle behavior and render a native `<dialog>` owner. They are compatibility
aliases at the lifecycle boundary; they do not nest or implement competing focus
models. The behavior uses `showModal()` and `close()` so the browser owns the top
layer, background inertness, modal Tab containment, initial focus algorithm,
Escape close request, and normal focus restoration.

`cem-sheet[transient]` remains a non-modal labeled `<aside>`. It mirrors
`expanded` only through native visibility. It does not trap or move focus,
intercept Escape, make the document inert, or claim dialog semantics.

`expanded` on a feedback host is component state, not an ARIA relationship.
When an application-owned button opens either kind of surface, that button owns
`aria-expanded="true|false"` and `aria-controls` where applicable. Neither the
dialog, dialog shell, sheet, nor a structural wrapper receives
`aria-expanded`.

## Alternatives considered

| Shape | Decision | Reason |
| --- | --- | --- |
| Opt-in `transient` mode plus current `expanded` state | Accepted | Preserves existing static output while adding an explicit, testable lifecycle for the state matrix. |
| Make absence of `expanded` close every existing feedback surface | Rejected | Existing examples and tests render visible surfaces without an open-state attribute; changing the default would silently break current markup. |
| Native `<dialog>` for transient dialogs | Accepted | The HTML platform owns top-layer modality, inertness, focus entry/containment, close requests, and restoration without a component-specific trap. |
| ARIA `div[role="dialog"]` plus imperative inerting and Tab loops | Rejected | It duplicates browser behavior, expands the mutation surface, and makes nested/top-layer cleanup component-owned. |
| Treat `cem-dialog` as a surface nested inside `cem-dialog-shell` | Rejected for Phase 4 | Existing public markup uses both tags independently and supplies the same projected body shape. Requiring composition would be a separate migration. |
| Put `aria-expanded` on the feedback surface | Rejected | Expanded state describes the controlling disclosure trigger; a dialog or region is the controlled surface, not its own opener. |
| Make sheets modal or dialog-like | Rejected | The shipped component contract explicitly defines `cem-sheet` as non-modal and `role="region"`. |

The native-dialog choice follows the
[HTML dialog lifecycle](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element):
`showModal()` captures the previously focused element, moves the dialog into the
top layer, blocks the document, runs dialog focusing, and `close()` performs
cleanup and restoration. Initial focus, contained Tab navigation, Escape, and
return-focus expectations also align with the
[WAI-ARIA modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/).

## Author API

Static compatibility mode remains unchanged:

```html
<cem-dialog label="Embedded confirmation summary">
  <p>The current static feedback content remains visible.</p>
</cem-dialog>
```

Transient state is explicit:

```html
<button
  type="button"
  aria-controls="confirm-dialog"
  aria-expanded="true"
>
  Review changes
</button>

<cem-dialog
  id="confirm-dialog"
  label="Confirm changes"
  transient
  expanded
>
  <form method="dialog">
    <p>Apply these changes?</p>
    <button value="cancel">Cancel</button>
    <button value="confirm" autofocus>Apply</button>
  </form>
</cem-dialog>
```

| Attribute | Contract |
| --- | --- |
| `label` | Required non-empty accessible name on the rendered dialog or sheet owner. |
| `transient` | Presence opts into open/closed lifecycle behavior. Absence preserves the current static output byte-for-byte and ignores `expanded`. |
| `expanded` | Presence means open and absence means closed in transient mode. It uses WHATWG boolean presence semantics and is kept synchronized with component- or application-requested transitions. |

`transient="false"` and `expanded="false"` are present/true and are invalid
attempts to express false. Authors close a transient surface by removing
`expanded`, not by serializing the string `"false"`.

The component does not create an opener. Applications own opener text,
placement, activation, `aria-controls`, and reflected `aria-expanded`. The
component does not search for or mutate an external trigger.

## Static compatibility mode

Without `transient`, each primitive retains its current rendered shape:

- `cem-dialog` renders its existing `.cem-dialog` ARIA dialog wrapper;
- `cem-dialog-shell` renders its existing `.cem-dialog-shell` ARIA dialog
  wrapper;
- `cem-sheet` renders its existing labeled `.cem-sheet` region; and
- projected content, accessible names, classes, DOM order, visibility, and
  behavior remain unchanged.

The implementation MUST prove byte-equivalent passive output in the focused
fixture. Adding or removing `expanded` while `transient` is absent has no effect.
This compatibility mode does not acquire dismissal, trapping, focus movement,
or a component event as a side effect of this contract.

## Transient dialog lifecycle

In transient mode, both dialog tags follow the same algorithm:

1. Render one stable native `HTMLDialogElement` with the tag-specific class and
   the host `label` as its accessible name. Rely on native dialog semantics; do
   not add `role="dialog"`, `aria-modal`, or `tabindex`.
2. Keep the native `open` attribute browser-owned. The behavior synchronizes it
   through `showModal()` and `close()` and MUST NOT remove `open` directly from
   a modal dialog.
3. When `expanded` becomes present and the dialog is closed, capture the current
   active element for disconnect recovery and call `showModal()` after the
   rendered dialog is connected.
4. When `expanded` becomes absent and the dialog is open, call `close()`. This
   application-requested transition does not dispatch `cem-dismiss`.
5. When a native close request succeeds—Escape or an authored native dialog
   action—synchronize the host by removing `expanded`, then dispatch one
   `cem-dismiss` notification from the host.
6. If native `cancel` is prevented, leave the dialog open, retain `expanded`,
   and do not dispatch `cem-dismiss`.
7. Preserve the native dialog element and projected payload identity across
   open/closed transitions. Opening and closing MUST NOT recreate authored
   descendants or mutate their state.

The behavior MUST tolerate redundant requests. Calling the open path while
already modal and the close path while already closed are no-ops, not errors.
An `InvalidStateError` from an externally corrupted native `open` state is a
contract violation to surface in the focused test, not a reason to fall back to
an ARIA wrapper.

### Dismissal event

`cem-dismiss` is a post-close notification for a native/user-requested dialog
dismissal whose detail is JSON-serializable:

```ts
interface CemDialogDismissDetail {
  reason: 'cancel' | 'close';
  returnValue: string;
}
```

The event bubbles and composes and is not cancelable. Native `cancel` remains the
cancellable pre-close event; the behavior MUST respect `preventDefault()`.
Removing host `expanded` is already an application close command and therefore
does not echo a `cem-dismiss` notification.

An authored `<form method="dialog">`, native close command, or direct native
dialog `close()` reports `reason: "close"`. A successful Escape/close request
reports `reason: "cancel"`. The event does not infer business meaning such as
confirm, delete, or save; authors use the native dialog `returnValue`.

## Focus and keyboard boundary

- `showModal()` captures focus at open time, satisfying programmatic as well as
  pointer/keyboard opening. Do not capture focus at an opener click.
- Native dialog focusing chooses an authored `autofocus` target, otherwise the
  browser focus delegate, otherwise the dialog. The component does not inject
  `autofocus` or `tabindex` and does not reorder authored descendants.
- The browser owns modal Tab/Shift+Tab containment and document inertness. At a
  native Chromium boundary, `document.activeElement` may temporarily become
  `body` before the next sequential move re-enters the dialog; no outside page
  control may receive focus. The component does not override that platform
  boundary with a document-wide keydown handler, focus sentinels,
  `aria-hidden` sweep, or custom Tab loop.
- Escape follows native `cancel`/close behavior. Preventing `cancel` is the only
  Phase 4 modal-blocking override.
- Normal `close()` restores the element captured by the platform at open time.
  If the host disconnects while modal, the behavior closes and releases the
  native dialog; it restores the separately captured element only when that
  element is still connected and focus was left in the removed dialog.
- If the previous element no longer exists, the component does not guess a
  workflow target. The application owns a logical fallback.

Authors SHOULD include a visible native dismissal control. The acceptance
fixture MUST include one and MUST also cover an `autofocus` target so initial
focus is deterministic.

## Transient sheet lifecycle

`cem-sheet[transient]` retains one stable labeled `<aside role="region">`:

1. `expanded` present removes native `hidden`; `expanded` absent adds `hidden`.
2. The host, aside, and projected payload remain stable across transitions.
3. Opening or closing does not move or restore focus. A surviving external
   opener therefore retains focus unless the application intentionally moves it.
4. The sheet does not listen for Escape, outside pointer interaction, or focus
   leaving its subtree and does not dispatch `cem-dismiss` merely because the
   application removes `expanded`.
5. The sheet does not add `aria-modal`, dialog roles, inertness, a backdrop, or
   a focus trap.

An application may put controls inside the sheet, including its own close
button. That control updates application state and removes host `expanded`; it
does not become a component-owned trigger merely by being projected.

## Failure and disconnect behavior

- Disconnecting a transient modal MUST release the top layer and all component
  listeners. Reconnecting an instance reads its current `expanded` state fresh.
- A close task queued before disconnection MUST NOT emit a late `cem-dismiss`
  from a disconnected host.
- Re-rendering a label or other host attribute while open MUST preserve modal
  state, focused-descendant identity, and the captured restoration target.
- The behavior MUST remove native listeners from replaced owners and on host
  disconnect; duplicate connect/render hooks MUST NOT duplicate events.
- Multiple component instances keep independent state. One instance MUST NOT
  close, focus, or mutate another.

## Resolved substrate boundary: browser-owned attributes

The first red component fixture proved that the prior `cem-elements` DOM merge
could not preserve the native dialog lifecycle safely. With a transient dialog
open through `showModal()`, changing only the host `label` retained the same
`HTMLDialogElement` but produced these observed `open` mutations:

```text
"" -> null -> ""
```

The generic attribute synchronizer removed `open` because it was absent from the
render plan, then the behavior's rendered hook called `showModal()` again. The
dialog appeared open and still matched `:modal`, but the second call captured a
new restoration target. A later native close therefore focused `body` instead
of the opener captured by the original open transition.

This was the contract's explicit stop condition. The component MUST NOT mask it
by manually removing/re-adding `open`, rendering `open` before calling
`showModal()`, or installing a component-specific mutation observer. Those
approaches either violate the HTML dialog cleanup lifecycle, open a non-modal
dialog before the modal call, or reproduce the same restoration loss.

The accepted substrate follow-up is now implemented as a generic opt-in
preservation hook:

1. `RenderedFragmentMergeOptions.preserveElementAttribute` identifies an exact
   current attribute that is runtime/browser-owned for a current/desired
   element pair.
2. `CemProducedElementBehavior.preserveRenderedAttribute` exposes the predicate
   through the browser-only behavior boundary, and `CemElementRuntime` forwards
   it for the current produced instance without serializing it into render
   plans.
3. During attribute synchronization, skip removal only when the predicate owns
   that exact current attribute. Desired render-plan attributes remain
   authoritative, and unrelated undeclared attributes must still be removed.
4. Let the feedback behavior preserve only native `dialog[open]` while its
   transient modal is active. The normal `beforeRender` close path still calls
   `close()` before an authored state transition or owner replacement.

Direct render-plan and produced-element Chromium stories now prove that contract,
including zero `open` mutations during an unrelated label render, retained modal
and focused-descendant identity, original-opener restoration, desired-value
override, removal of unclaimed attributes, and native cleanup before authored
state change, owner replacement, and disconnect. No special case for `dialog`,
`open`, or CEM feedback components exists in the generic projection module. The
component fixture may now be retried using the fourth rule above.

## Executable acceptance

Before adding the fixture, `docs/todo.md` must contain an explicit actionable
implementation item. The implementation is complete only when focused Chromium
coverage proves:

- passive output remains byte-equivalent and ignores `expanded`;
- transient closed/open initialization and live host-attribute transitions;
- native modal/top-layer state for both dialog tags and native-hidden state for
  the sheet;
- `autofocus` initial focus, forward/reverse native Tab boundaries with outside
  controls inert, Escape dismissal, prevented cancel, native close return
  value, and focus restoration;
- non-modal sheet focus retention with no Escape interception or inert document;
- exact `expanded`, native `open`/`hidden`, and external trigger ARIA agreement;
- one serializable `cem-dismiss` per native dismissal and none for application
  close, prevented cancel, passive mode, sheet state changes, or disconnect;
- stable owner/payload identity, geometry while each state is stable, and no
  authored descendant state mutation; and
- listener/top-layer cleanup across close, disconnect, and reconnect.

The declarative fixture is
[`tests/feedback/expanded.html`](../tests/feedback/expanded.html), exercised by
[`feedback-expanded.browser.spec.ts`](../src/lib/feedback-expanded.browser.spec.ts).
Passive compatibility passes normally. Four behavior-dependent cases use
Vitest's executable expected-failure mode and currently fail at the absent
native-dialog and hidden-sheet boundaries. The behavior implementation must
remove those expected-failure modifiers and make every assertion pass; expected
failures do not count as state-matrix coverage.

This expanded-state slice does not add feedback focus paint. After its lifecycle
is executable, the separate feedback focus decision must identify actual native
focus owners and audit theme semantics before adding component CSS. A CSS
exception remains forbidden unless no appropriate theme category represents the
accepted visual treatment.
