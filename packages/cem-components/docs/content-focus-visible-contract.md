# Content Focus-Visible Contract

## Scope

This contract closes the Phase 4 `content:focus-visible` state for the
interactive content controls the package renders today. Keyboard focus adds an
external indicator to an existing native owner. It does not select a list
option, toggle a chip, activate a control, or create component runtime state.

## Interaction owners

Focus paint belongs to the same native controls as content hover:

- `cem-list[selectable] > select.cem-list.cem-list--selectable`; and
- `cem-chip[checkable] > button.cem-chip`.

The `cem-list` and `cem-chip` hosts are structural boundaries. Passive
`cem-list > ul`, passive `cem-chip > span`, `cem-table > .cem-table`, and
application-authored table rows remain outside this contract. They do not gain
`tabindex`, focus handlers, or focus paint.

## Token audit

The existing theme catalog fully represents the requirement. D5 owns keyboard
focus thickness and external placement through `--cem-stroke-focus` and
`--cem-stroke-indicator-offset`. The zebra focus category owns the mode-aware
ring color through `--cem-zebra-color-1`.

These endpoints are independent from the D0 content-interaction default,
hover, selected, selected-hover, and disabled fill/text pairs. No new D0 token,
raw component value, local custom property, or CSS exception is required.

## Selector and state behavior

The public stylesheet applies the ring only when an accepted native owner
matches `:enabled:focus-visible`. Native-disabled owners cannot acquire the
ring and remain outside sequential keyboard order. This slice does not add a
host-level `disabled` API to `cem-list` or `cem-chip`.

The indicator uses `outline`, not border, padding, shadow, transform, generated
content, or motion, so focus does not alter owner or host geometry. The
selectable list's committed native option remains selected while the composite
is focused. Checked and unchecked chips retain their D0 fill/text semantics.
Pointer hover may replace those fill/text values without replacing the focus
ring, and leaving hover restores the focused default or selected paint.

Focus order remains native: selectable list composite, then authored checkable
chips in document order. Passive content and disabled native owners are
skipped. The component adds no focus, keyboard, selection, or restoration
handler.

## Forced colors

In `forced-colors: active`, focused content owners retain the D5 focus width and
offset while the outline maps to `CanvasText`. Checked rest paint continues to
use `SelectedItem` / `SelectedItemText`; enabled chip hover continues to use
`Highlight` / `HighlightText`; and the focused selectable list may use its
independent `Highlight` hover border without replacing the `CanvasText` ring.
Disabled and passive content remain unfocused.

## Executable acceptance

The focused Chromium state fixture proves:

- exact Tab traversal through the selectable-list composite, unchecked chip,
  and checked chip before the end sentinel;
- disabled native-owner and passive-content skipping;
- exact D5 width/offset and zebra focus-color resolution;
- selected-option, checked-chip, and hover coexistence and restoration;
- stable owner/host dimensions, HTML/ARIA, and serializable runtime state;
- host and passive-wrapper isolation; and
- absence of click, input, change, and component lifecycle mutation events.

The combined content hover/focus forced-colors gate repeats the keyboard order
and verifies `CanvasText`, tokenized width/offset, checked and hover system
paint, listbox hover-border coexistence, native-disabled skipping, restoration,
geometry, DOM/ARIA, wrapper isolation, and event/state absence.

## Failure conditions

The contract fails if focus paint lands on a host or passive wrapper, a
native-disabled owner enters the keyboard order, selected/checked state changes,
hover replaces the focus ring, geometry or DOM/runtime state mutates, a mutation
event fires, forced colors lose the system outline, or normal component CSS
uses an unknown/raw/local value.
