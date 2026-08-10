# Feedback Focus-Visible Contract

**Status:** Accepted Phase 4 implementation contract with the focused Chromium
fixture landed. Native ownership/lifecycle cases pass normally, and one
executable expected failure now isolates the missing D5/zebra dialog-owner
outline before component CSS or forced-colors work. It is promoted by
[`docs/todo.md`](../../../docs/todo.md).

## Scope

This contract closes `feedback:focus-visible` without creating a second focus
model for feedback surfaces. The component paints focus only when the browser
uses a transient native dialog itself as the modal focus fallback. Focus on an
authored descendant remains owned by that descendant and its component or user
agent styling.

`cem-sheet` stays non-modal and focus-neutral. It can contain focusable authored
content, but the sheet host and region do not become focus owners merely because
focus is inside them.

## Accepted owners

The only component-owned focus-paint owners are:

- `cem-dialog[transient] > dialog.cem-dialog:focus-visible`; and
- `cem-dialog-shell[transient] > dialog.cem-dialog-shell:focus-visible`.

Both selectors address the stable native owner created by the accepted
[feedback expanded contract](./feedback-expanded-contract.md). A closed dialog
cannot receive focus, so the selector does not duplicate lifecycle state with
an `[expanded]` condition.

The following are explicitly outside the contract:

- static `div[role="dialog"]` compatibility owners;
- `cem-dialog`, `cem-dialog-shell`, and `cem-sheet` hosts;
- `cem-sheet > aside.cem-sheet` in either lifecycle mode;
- authored buttons, links, fields, and other projected descendants; and
- `:focus-within`, injected `tabindex`, focus sentinels, or focus event classes.

The component stylesheet MUST NOT reach through the feedback owner to restyle
arbitrary authored descendants. Authors may use CEM action/input primitives or
their own accessible focus treatment inside a dialog or sheet.

## Platform boundary

Native dialog focusing first considers eligible autofocus/focusable authored
content. When no eligible descendant exists, Chromium focuses the dialog owner
itself without an authored `tabindex`. A pre-contract keyboard-modality spike
used a focused opener and a dialog whose only authored control was disabled and
marked `autofocus`; after `showModal()`:

- `document.activeElement` was the native dialog;
- the dialog matched both `:modal` and `:focus-visible`;
- the disabled control was skipped; and
- native `close()` restored the opener.

If a supported browser cannot expose its native dialog fallback through
`:focus-visible`, the implementation must stop at that platform gap. It must
not compensate by making a structural wrapper focusable or painting
`:focus-within`.

## Token audit and paint

The generated theme already represents the complete focus indicator:

- D5 `--cem-stroke-focus` owns keyboard-focus thickness;
- D5 `--cem-stroke-indicator-offset` owns external placement; and
- `--cem-zebra-color-1` owns the mode-aware focus color.

The normal-mode rule uses an external `outline` with those three endpoints.
It adds no fill, border, padding, shadow, transform, motion, local custom
property, raw component value, or dialog-specific theme semantic. No component
CSS exception is required.

The outline must not alter dialog or host geometry. It coexists with native
modal/top-layer state and must not mutate `expanded`, `open`, return values,
focus order, authored state, or component events.

## Keyboard and lifecycle behavior

Executable coverage must distinguish two native paths:

1. With an eligible authored autofocus/focus target, focus lands on that
   descendant. The dialog owner does not match `:focus-visible`, and the
   component does not add a wrapper ring.
2. With no eligible authored target, the browser focuses the native dialog
   owner. Under keyboard modality, the owner matches `:focus-visible` and
   receives the D5/zebra outline.

Forward and reverse Tab navigation from the fallback owner remain native and
must not reach inert outside controls while modal. Disabled authored controls
are skipped. Successful Escape/native close restores the original opener and
removes the ring with focus; prevented cancel retains both modal and focused
owner state.

A transient sheet retains external opener focus when shown. When keyboard focus
moves to an authored sheet control, that control—not the aside or host—matches
`:focus-visible`. Sheet visibility does not intercept Escape or add focus
restoration behavior.

## Forced colors

Under `forced-colors: active`, the accepted dialog owners retain the D5 width
and offset and use `CanvasText` for the outline. `forced-color-adjust` remains
`auto`. The implementation must not use box-shadow as the only indicator or
force platform colors on authored descendants.

## Executable acceptance

Before the matrix row can move to covered, focused Chromium evidence must prove:

- exact keyboard focus entry for authored-target and dialog-fallback cases on
  both dialog aliases;
- exact `:focus-visible` ownership and absence on hosts, static wrappers,
  sheets, and non-focused authored descendants;
- disabled autofocus skipping and native forward/reverse modal Tab boundaries;
- opener restoration after native close plus retained owner focus/ring after a
  prevented cancel;
- a focus-neutral transient sheet whose authored control owns its own focus;
- exact D5/zebra computed values, stable owner/host geometry and DOM/ARIA, and
  no expanded/open/input/event mutation caused by focus paint;
- normal-mode restoration after blur/close; and
- forced-colors `CanvasText`, tokenized width/offset, wrapper/descendant
  isolation, and `forced-color-adjust: auto`.

The style verifier must lock the exact direct-owner selectors and reject
structural, descendant-wide, raw-value, or unknown-token alternatives. Only
after the focused browser, forced-colors, style, documentation, and aggregate
gates pass may `feedback:focus-visible` become covered.

The focused cases live in
[`feedback-expanded.browser.spec.ts`](../src/lib/feedback-expanded.browser.spec.ts)
and reuse the declarative
[`tests/feedback/expanded.html`](../tests/feedback/expanded.html) fixture. Two
ordinary tests prove authored-descendant versus native-fallback ownership and
the full lifecycle boundary above. The paint case currently uses executable
expected-failure mode: its promoted red run observed the UA's `1px auto`
outline at `0px` for both aliases instead of the required tokenized `3px solid`
outline at `2px` and zebra focus color.
