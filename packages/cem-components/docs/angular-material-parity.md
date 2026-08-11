# Angular Material Parity Inventory

**Status:** All pinned product mappings are audited. The accepted `autocomplete`,
`divider`, `expansion`, `progress-spinner`, `sort`, `paginator`, `slider`, and
`tooltip` priorities are covered;
selecting another implementation gap is a separate product decision tracked in
[`docs/todo.md`](../../../docs/todo.md).

## Benchmark

The M4 comparison baseline is the latest stable official Angular Material
release available when this inventory was captured:

| Field | Pinned value |
| --- | --- |
| Product release | Angular Material `22.1.1` |
| Git tag | `v22.1.1` |
| Git commit | `0b67c3c38141049657b1167479accc80e455d2bd` |
| Capture date | 2026-08-10 |
| Public catalog | <https://material.angular.dev/components/categories> |
| Tagged catalog source | <https://github.com/angular/components/blob/v22.1.1/docs/src/app/shared/documentation-items/documentation-items.ts> |
| Catalog entries | 37 |

The tagged `DOCS.components` list is the reproducible source of entry IDs and
display names. Pre-releases are not silently adopted. Updating the benchmark
requires a deliberate tag/commit change plus an exact inventory reconciliation.

## Package boundary

`@epa-wg/cem-elements` is the barebone layer: it supplies `<cem-element>` as the
declarative base plus browser/API primitives such as URL and HTTP resource
access. Its eight legacy Material fixtures prove template/runtime authoring
compatibility for the old component samples; they are not the product catalog
and cannot satisfy an Angular Material parity row.

`@epa-wg/cem-components` is the Material-superset UI layer. It builds on
`cem-elements`, supplies public UI components, and owns Consumer Semantic Theme
styling, state, keyboard, accessibility, forced-colors, and workflow evidence.
Only evidence rooted in this product layer can promote a catalog row to covered.

## Executable inventory

[`angular-material-parity.json`](../tests/angular-material-parity.json) contains
one row for each pinned official entry in official order. New rows start as
`unreviewed` with a null mapping. This is intentional: matching names or an
existing custom-element tag do not establish behavioral parity.

The mapping audit may use these states:

- `unreviewed`: no product mapping decision has been accepted;
- `gap`: required Material-facing behavior has no accepted CEM owner;
- `partial`: an accepted CEM component or cross-cutting behavior covers only a
  documented subset; and
- `covered`: the accepted mapping has exact state, keyboard, accessibility, and
  executable product-layer evidence.

Every reviewed row records one mapping kind (`component`, `behavior`, or
`gap`), exact owners, required states, keyboard behavior, accessibility
semantics, evidence, and notes. Component owners must be public
`CEM_COMPONENT_PRIMITIVES` tags. Behavior owners use a `behavior:` identity.
Evidence cannot point only at `packages/cem-elements` compatibility fixtures.

The audit compares behavioral capability, not Angular directive, service, or
TypeScript API compatibility. `covered` means that the accepted CEM semantic
owner has executable state, keyboard, and accessibility evidence for the
capability. `partial` means a real owner exists but the row names at least one
Material-facing behavior that owner does not support. `gap` means no semantically
appropriate public owner exists; a similarly shaped layout primitive is not
substituted for the missing component.

## Audit result

| Classification | Count | Catalog entries |
| --- | ---: | --- |
| Covered | 14 | autocomplete, card, checkbox, dialog, divider, expansion, input, paginator, progress-spinner, select, slide-toggle, slider, sort, tooltip |
| Partial | 19 | badge, bottom-sheet, button, button-toggle, chips, core, form-field, grid-list, icon, list, menu, progress-bar, radio, ripple, sidenav, snack-bar, table, tabs, toolbar |
| Gap | 4 | datepicker, stepper, timepicker, tree |

Each row in the executable inventory explains its boundary. In particular,
`cem-grid` is not treated as a grid-list, `cem-tabs` is not treated as a stepper,
`cem-progress` is not treated as a circular spinner, and navigation-specific
disclosure is not treated as a general expansion panel.

## Completed implementation priorities

`autocomplete` was accepted as the first gap to close. It has high reuse of the
existing input, option, listbox, form, popup, and forced-colors foundations while
requiring a distinct editable-combobox owner rather than expanding
selection-only `cem-select` semantics.

The [autocomplete contract](./autocomplete-contract.md) fixes the owner, author
vocabulary, value/form/event model, application-owned filtering boundary,
keyboard and accessibility behavior, theme-token coverage, focused fixture,
forced-colors behavior, and assertion matrix before runtime or CSS work.

The row is now covered by the public `cem-autocomplete`, `cem-option`, and
`cem-option-group` owners; the focused browser suite; the component-specific
state-matrix evidence; the token-only style contract; the dedicated
forced-colors gate; and npm package verification of the emitted autocomplete
runtime artifact. No successor priority is implied by this promotion.

`divider` was accepted next after recovering the Consumer Semantic Theme
boundary between spacing and visible separation. The
[divider contract](./divider-contract.md) defines `cem-divider` as the complete
line-plus-margins track: D0 supplies reduced-salience color and `CanvasText` in
forced colors, D1 supplies relationship spacing and inset, D2 floors the track
at the coupling guard, and canonical D5 supplies the line geometry. Semantic,
decorative, horizontal, vertical, and inset behavior is covered without adding
focus, keyboard, event, or state ownership.

`expansion` then closed the false substitution between navigation disclosure
and an independent content panel. The [expansion contract](./expansion-contract.md)
assigns one native header button, a persistent ARIA-linked panel, live expanded
state, disabled suppression, contextual-action paint, stable geometry, and
forced-colors behavior to the public `cem-expansion` owner.

`progress-spinner` adds the distinct circular progress owner without changing
linear `cem-progress`. The
[progress spinner contract](./progress-spinner-contract.md) makes value presence
the determinate/indeterminate boundary, normalizes values without rewriting
author attributes, keeps a stable light-DOM SVG, and stops automatic repetition
under reduced motion. The pre-CSS audit added canonical D0 progress colors, D2c
size/thickness, and D7 continuous-cycle timing; forced colors map the track and
indicator to `GrayText` and `Highlight`. No component CSS exception was needed.

`sort` adds the independent `cem-sort-header` owner without changing passive
`cem-table` into a data source. The [sort-header contract](./sort-header-contract.md)
puts column semantics and conditional `aria-sort` on the generated column
header, all interaction on its direct native button, and row ordering plus
localized announcement on the application consuming `cem-sort`. The fixed
none -> ascending -> descending -> none cycle coordinates only peers in the
nearest table. Existing contextual-action and D1/D2/D2c/D3/D5/D6 semantics
cover the normal and forced-color contract, so no theme addition or CSS
exception was needed.

`paginator` adds the independent `cem-paginator` owner without turning passive
`cem-table` or `cem-list` into a data source. The
[paginator contract](./paginator-contract.md) fixes a zero-based normalized page
model, native page-size and navigation controls, first-visible-item preservation,
focus-stable boundary suppression, localizable labels, an atomic polite range,
and one serializable `cem-page` request event. Data loading, item/row rendering,
and result announcements remain application-owned. Existing contextual-action,
select/input-indicator, palette, D1/D2/D2c/D3/D5/D6 semantics cover normal and
forced colors, so no theme addition or CSS exception was needed.

`slider` adds one `cem-slider` visual owner while authored native range inputs
retain exact thumb interaction, accessible-name, event, and independent form
serialization ownership. The [slider contract](./slider-contract.md) fixes the
strict single/start/end vocabulary, normalized parent bounds, non-crossing
range behavior, optional ticks/value labels, native keyboard/event surface, and
application-owned value formatting. The pre-CSS audit added canonical D0 slider
paint and D2c visual geometry; D2/D3/D5/D6 already cover targets, shape, focus,
and labels. No component CSS exception was needed.

`tooltip` adds one `cem-tooltip` description/presentation owner while its exact
authored native trigger retains pointer, focus, keyboard, activation, and event
ownership. The [tooltip contract](./tooltip-contract.md) fixes the strict named
trigger/message vocabulary, persistent `aria-describedby` copy, independent
hover/focus reasons, delay, declarative manual `open`, Escape and disabled
suppression, native touch boundary, and logical CSS Anchor Positioning in the
Popover top layer. Existing D0/D1/D3/D4/D5/D6 semantics cover normal and
forced-color paint; there is no animation and no component CSS exception.

Run the invariant with:

```bash
yarn nx run @epa-wg/cem-components:verify-material-parity
```

The gate verifies the exact 37-entry pin, every audited mapping, and completion
of the accepted implementation priority. It reports fourteen covered rows,
nineteen partial rows, four gaps, no remaining audit, and no next
implementation until a new gap is deliberately selected and contracted.
