# Suggestions acceptance evidence

The first profile attaches `cem-suggestions` to the original text input owned by
`cem-field` or `cem-text-field`. Run the colocated plays through the shared
Storybook browser target:

```sh
yarn nx run cem-elements:test -- packages/cem-components/src/components/cem-suggestions/cem-suggestions.stories.ts packages/cem-elements/src/lib/suggestions-controller.stories.ts packages/cem-elements/src/lib/manual-listbox.stories.ts packages/cem-elements/src/lib/surface-geometry.stories.ts packages/cem-elements/src/lib/native-surface.stories.ts
```

## Automated evidence

The matrix records observed behavior; it does not establish physical device,
operating-system IME or screen-reader interoperability. The current runner is
Playwright Chromium. Its mouse, touch and pen cases inject browser input through
[CDP input](https://chromedevtools.github.io/devtools-protocol/tot/Input/), including
the browser's compatibility mouse events and implicit touch capture. Coordinates
account for the test iframe's scaling. Native touch-scroll gestures
did not scroll even a plain overflow-box control in this environment, with both
desktop and mobile emulation. The automated matrix therefore claims touch
movement cancellation and wheel scrolling separately; physical touch-pan
verification remains pending. Composition cases use Chromium's composition/input
protocol; these events are not uniformly
trusted and do not represent an operating-system candidate window.

| Area | Evidence | Boundary |
| --- | --- | --- |
| Both field providers | Original input/help identity, one FormData owner, reset, stored values and value hints | Text inputs only |
| Independent grouped filtering | Prefix filtering, unavailable rows, independent controls and preserved description | Local bounded native sources |
| Publication changes | Source replacement expires preview, old rows cannot commit, disable/read-only and conflicting inputs suspend claims | A fresh interaction is required to reopen |
| Commit transaction | Beforeinput veto, one raw input/change pair, one public pair, coherent field value and FormData at callbacks | Native undo grouping is not promised |
| Pointer taps | Mouse/touch/pen browser input commits once and leaves the original editor focused | Chromium protocol-driven input |
| Pointer cancellation | Touch movement, cancel, pen drag, outside focus and source loss cannot commit a stale press; native wheel input scrolls | No row refocus after blur |
| Composition | Composing edits update submission, close preview, settle from editor text and fence terminal Enter; a new press can select afterward | Protocol-driven composition, not physical IME |
| Native surfaces | Shared listbox/geometry/native-surface regressions plus production-template host integration with both fields and external CEMB sources: modal containment, nested Escape/held repeats, close/reopen, expired lifetime rejection, fresh admission and teardown | Worker/fallback host hooks explicitly supply ancestor authority; local adjacency supplies none |
| Accessibility structure | Listbox/group names, option active-descendant, stored-value names, localized status, disabled rows and preserved field help | DOM contracts, not spoken-output evidence |

The row adapter cancels primary mouse pointerdown only on an admitted row. Touch
and pen pointerdown stay native. While an exact row press is armed, its
compatibility mousedown may suppress the focus default. Normal implicit capture
release after a completed touch pointerup keeps the press armed for its click;
capture loss before completion, cancellation, scrolling, drag or revision loss
invalidates it. A browser that moves focus before this suppression closes the
session and rejects the activation. It must never recover by refocusing.

Observer-driven row synchronization waits for the original parent's committed
CSS/DOM frame and fences callbacks by the active capability connection. Pending
or failed scalar readiness withdraws the prior presentation immediately, so a
later native close does not discard focused status feedback. This timing is
covered by the shared feedback play and concurrent source/installed-package
gallery verification. Explicit dismissal, blur, composition and authority loss
still clear qualifying feedback.

The host integration uses the production suggestions template under a unique
fixture tag and the unchanged production fields. It admits original external
CEMB owners through `nativeSuggestionsInputs` and exact endpoint/grant leases
through `suggestionsControllerInputs`. Captured parent lifetimes expire on
close, remain expired after reopen, and must be replaced explicitly by the host.
Source preparation settles before the deliberate opening key; a key issued
while pending is never replayed. The combined five-suite run passes 39 browser
plays. Physical release evidence below remains separate.

## Manual release record

Use the linked [property playground](../playgrounds/cem-suggestions.html) and
[gallery](../playgrounds/cem-suggestions-gallery.html) from source and an installed
package. Record browser/OS version, device/pointer, keyboard/IME or assistive
technology version, field type, exact steps and observed results. A blank record
is pending evidence. Do not infer a pass from the automated matrix.

| Pending run | Procedure and expected result |
| --- | --- |
| Physical touch and pen | On each supported browser/device, tap an enabled rich-label row on both fields. The stored value commits once, caret/focus stays in the same input, and no keyboard flicker occurs. Pan the list and cancel a gesture: scrolling stays native and neither commits. Test disabled rows, group headings, blank space and scrollbars. |
| Native IME | With a real candidate window (for example Japanese and Chinese IMEs), compose while a row is previewed. Terminal Enter/Escape/arrows belong to the IME; they neither select nor submit. Verify both possible final-input/composition-end orders, then keyup and a fresh deliberate selection. Record actual event order. |
| Screen reader | On supported browser/AT combinations, verify the field label/help, expanded state, group and active row names including stored-value hints, and localized pending/failure/empty/count announcements. Preview must not repeat unchanged status. Escape, blur and cancellation clear preview without losing the field's name/help. |
| Mobile viewport | Open the keyboard, rotate/change the visual viewport and scroll ancestors. The popup remains fitted to the original editor or closes when the anchor is unusable. It never obscures an independently focused destination or revives after dismissal. |
| Native editing/history | After commit, type, move the caret, select text and undo/redo. Record actual browser behavior and field/FormData/validity coherence. Do not require a single undo entry for scripted replacement under this first profile. |
| External/ancestor relationships | Prepare exact native endpoints and trusted ancestor lifetimes through host APIs. Verify nested Escape, parent dismissal, modal containment, placement revocation and fresh admission after resume. The simple local gallery cannot supply this authority by adjacency. |

Actionable pending runs and their scenarios remain next to the acceptance item
in [todo.md](../../../docs/todo.md). Additional editor types, full surface-provider
adoption, undoable replacement and platform close requests have separate design
items.

## Recording a physical run

Use one record per browser/device/field combination. Test both `cem-field` and
`cem-text-field` against the same options. Keep failures and inconclusive runs;
repeat a failed gesture on a plain native overflow box to distinguish runner or
platform limitations from the suggestions surface. A release run is complete
only when its actual observations are attached here or linked from this record.

```text
Date / tester:
Source revision / source or installed package:
Browser version / OS version:
Device / pointer / keyboard / IME / assistive technology version:
Field: cem-field | cem-text-field
Scenario from the pending-run table:
Steps / input event order (for IME):
Observed focus / caret / stored value / FormData:
Observed scroll / viewport fitting / spoken announcements:
Outcome: pass | fail | inconclusive
Evidence link / follow-up issue:
```

No physical device, operating-system IME or assistive-technology run has been
recorded for this revision. The template makes the remaining checks executable;
it does not close their todo item.
