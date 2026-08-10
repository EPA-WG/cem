# Content Hover Contract

## Scope

This contract closes the Phase 4 `content:hover` state by narrowing it to the
interactive owners that the package actually renders today. Hover is transient
pointer paint. It does not select a list option, toggle a chip, activate a
control, or create component runtime state.

## Interaction owners

The accepted owners are:

- `cem-list[selectable] > select.cem-list.cem-list--selectable`, the native
  single-select listbox composite; and
- `cem-chip[checkable] > button.cem-chip`, the native toggle button.

The `cem-list` and `cem-chip` hosts are structural boundaries, not hover paint
owners. Passive `cem-list > ul`, passive `cem-chip > span`, and the
`cem-table > .cem-table` wrapper are explicitly excluded. A table's projected
rows remain application-authored; this slice does not invent interactive row or
grid vocabulary. Native `<option>` hover remains user-agent-owned and is not
substituted with the custom `cem-select` option contract.

## Token audit and adoption

The pre-CSS audit found complete action-intent hover pairs, but checkable content
is not necessarily a command and selectable content must remain independently
themeable. The generated `--cem-select-option-*` pairs belong specifically to
the HTML-rendered options of the custom form `cem-select`; they do not describe
the native content-list composite. Control tokens provide geometry only.

D0 therefore owns ten required `--cem-content-interaction-*` endpoints: paired
background/text values for `default`, `hover`, `selected`, `selected-hover`,
and `disabled`. The distinct selected-hover pair retains checked meaning while
the pointer is present. Component CSS uses no raw normal-mode color or local
custom property. This is theme adoption, so no component CSS exception is
recorded.

## State and selector precedence

The normal-mode cascade is `default < hover < selected < selected-hover <
disabled`. A checked chip binds selected paint through
`aria-pressed="true"`. The selectable list's committed option remains native
selected content inside the composite; hovering the `<select>` must not change
its value, option selectedness, or exact `aria-selected` reflection.

Enabled hover is gated by `:enabled`. The current component authoring APIs do
not add a new host-level `disabled` attribute for `cem-list` or `cem-chip` in
this slice. The selector and fixtures exercise native-owner disabled suppression
without expanding that public behavior contract; disabled list options remain
option-owned.

Focus is independently owned by the native control in this slice. Hover does
not replace its visible outline or move focus. The tokenized ring and keyboard
order are defined by the
[content focus-visible contract](./content-focus-visible-contract.md).

## Forced colors

In `forced-colors: active`:

- resting controls use `Canvas` / `CanvasText`;
- checked chips use `SelectedItem` / `SelectedItemText`;
- enabled chip hover uses `Highlight` / `HighlightText`;
- disabled native owners use `Canvas` / `GrayText`; and
- the native selectable-list composite retains its platform `Canvas` surface
  and `CanvasText`, with its existing border recolored to `Highlight` on hover.

Chromium intentionally preserves native listbox surface painting in forced
colors instead of exposing the authored hover fill. Recoloring its existing
border provides system-controlled pointer feedback without replacing native
appearance, changing border geometry, opting out of forced-color adjustment, or
colliding with the independent focus outline. Selected option state remains
native and unchanged.

## Executable acceptance

The focused Chromium browser fixture proves:

- trusted pointer enter and leave on both enabled owners, disabled native
  owners, and the excluded passive list/chip/table elements;
- exact generated-token resolution and at least 4.5:1 hover text/fill contrast
  in normal mode;
- selected-option, unchecked-chip, and checked-chip coexistence and restoration;
- disabled suppression and focus-visible coexistence;
- stable owner and host dimensions, HTML/ARIA, and serializable runtime state;
  and
- absence of click, input, change, and component lifecycle mutation events.

The combined hover/focus forced-colors Chromium gate verifies the system mappings,
restoration, native listbox border treatment, selected/checked state, focus
coexistence, disabled suppression, passive exclusions, trusted pointer boundary
events, geometry, and DOM/ARIA isolation. Chromium's inspection-only forced
pseudo-state is used for deterministic paint inspection; real trusted pointer
movement supplies the boundary events, and the ordinary browser fixture proves
native `:hover` matching.

## Failure conditions

The contract fails if a host or passive wrapper receives content hover paint, a
disabled owner acquires enabled paint, selected/checked state is lost, focus
treatment changes, geometry or DOM/runtime state mutates, a required theme
endpoint is absent, or normal-mode component CSS introduces a raw/local value.
