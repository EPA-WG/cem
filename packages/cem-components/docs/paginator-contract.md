# Paginator Contract

This contract fixes the paged-content navigation owner for
`@epa-wg/cem-components`. It is benchmarked against Angular Material
`v22.1.1` while using CEM's declarative state, event, accessibility, and
Consumer Semantic Theme vocabulary.

## Owner and author vocabulary

`cem-paginator` owns one labeled pagination navigation region, its page-size
control, requested range status, and first/previous/next/last actions. It does
not own a `cem-table`, `cem-list`, row payload, data source, network request,
loading state, or rendered-result lifecycle.

The public author surface is:

- `label`: navigation landmark name, defaulting to `Pagination`;
- `name`: stable application identity included in `cem-page` detail;
- `length`: total item count, defaulting to `0`;
- `page-index`: zero-based requested page index, defaulting to `0`;
- `page-size`: positive item count per page, defaulting to `50`;
- `page-size-options`: ASCII-whitespace-separated positive integer choices;
- `show-first-last`: presence renders first/last actions in addition to
  previous/next;
- `hide-page-size`: presence omits the page-size label and select;
- `disabled`: presence disables every user control;
- `items-per-page-label`, `first-page-label`, `previous-page-label`,
  `next-page-label`, `last-page-label`, and `of-label`: localizable visible or
  accessible text with English defaults.

The page index is intentionally zero-based to match the pinned Material
benchmark and its event model. Visible range numbers remain one-based. Public
color, density, shape, icon, and focus options are theme concerns and are not
attributes.

## State and range contract

Numeric normalization is render-only until a user action and never rewrites
invalid authored values by itself:

- `length` is a finite non-negative integer; invalid input resolves to `0`;
- `page-size` is a finite positive integer; invalid input resolves to `50`;
- page count is `ceil(length / pageSize)`;
- `page-index` is a finite non-negative integer clamped to the last available
  page, with empty content fixed at index `0`;
- page-size options discard invalid/non-positive values, de-duplicate and sort
  numerically, and always include the normalized current page size.

The range status is `0 – 0 of 0` when empty. Otherwise it is
`start – end of length`, where start is one-based and end never exceeds length.
Live author changes update the same navigation owner, select, range status, and
surviving action buttons without emitting events.

First and previous are unavailable at index `0`; next and last are unavailable
at the final page or when empty. Boundary actions use
`aria-disabled="true"` and `tabindex="-1"`, and capture-phase suppression keeps
pointer, keyboard, and programmatic clicks from reaching target/application
bubble listeners. When an enabled action moves onto a boundary, its persistent button
retains focus even though it leaves subsequent sequential tab order. Global
`disabled` additionally uses native `disabled` on every rendered control.

Changing page size preserves the previous page's first visible item:

`newPageIndex = floor(previousPageIndex * previousPageSize / newPageSize)`.

The result is clamped to the new final page. A user page-size choice writes
normalized `page-size` and `page-index` host attributes once per transition;
navigation actions write normalized `page-index`. Geometry and control identity
remain stable across page and range changes.

## Event and keyboard contract

Native Tab navigation reaches the enabled page-size select and action buttons
in DOM order. Native button Enter/Space and pointer click each navigate once.
The native select owns Arrow/Home/End selection behavior and emits its normal
trusted `input`/`change` events; the component listens only to `change` to
commit one page-size transition.

Every successful user transition emits one non-cancellable `cem-page` custom
event with `bubbles: true`, `composed: true`, and JSON-serializable detail:

```json
{
  "name": "records",
  "pageIndex": 2,
  "previousPageIndex": 1,
  "pageSize": 25,
  "length": 120
}
```

No-op boundary activation, reselecting the current page size, disabled input,
and programmatic attribute changes emit no `cem-page`. The component does not
synthesize `click`, `input`, or `change` events. Applications consume
`cem-page` to load or project the requested data and may reflect server-adjusted
pagination values back through attributes without creating an event loop.

## Accessibility contract

The direct rendered owner is a native `nav` named by `label`. This is a semantic
superset of Material's labeled control group and follows landmark guidance for
pagination controls. Identical top/bottom paginator instances may use the same
label when they perform the same navigation.

The visible page-size label natively labels its select. Each action is a native
button with a localizable `aria-label`; its character icon is
`aria-hidden="true"`. Boundary availability is programmatically exposed and
activation-suppressed without replacing the focused owner.

The range is an atomic polite `role="status"`. It announces the paginator's
requested numeric range after a successful user transition. Applications own
separate loading, success, empty, and error messages for the actual data result
and must not duplicate the range text in another live region.

## Theme-token audit

The pre-CSS audit found complete existing semantic coverage:

| Concern | Canonical owner | Binding |
|---|---|---|
| Navigation surface and text | D0 palette | `--cem-palette-comfort`, `--cem-palette-comfort-text` |
| Action states | D0 contextual action | `--cem-action-contextual-*-background/text` |
| Page-size field and boundary | D0 select/input indicator | `--cem-select-popup-*`, `--cem-input-indicator-anchor-*` |
| Relationships and surface inset | D1 | `--cem-gap-related`, `--cem-gap-group`, `--cem-inset-container` |
| Target floors and padding | D2/D2c | `--cem-coupling-zone-min`, `--cem-control-*`, `--cem-icon-button-*` |
| Control shape | D3 | `--cem-bend-control`, `--cem-bend-field` |
| Boundary and focus strokes | D5 | `--cem-stroke-boundary`, `--cem-stroke-focus`, `--cem-stroke-indicator-offset` |
| Labels and range text | D6 UI typography | `--cem-typography-ui-*` |

No new theme semantic and no `components-css-exceptions.md` entry are
required. The page-size control is in-flow, so it does not require popup
stacking or the bounded `CEM-CSS-002` exception.

## Forced-colors boundary

With forced colors active, the navigation surface and enabled select use
`Canvas` / `CanvasText`. Action default uses the same pair; hover and active use
`Highlight` / `HighlightText`; disabled controls use `Canvas` / `GrayText`.
The page-size boundary and D5 focus ring use `CanvasText`, hover uses
`Highlight`, and disabled uses `GrayText`. Automatic forced-color adjustment
stays enabled. Text character icons survive without masks, images, or
color-only meaning, and all transient states preserve geometry.

## Focused fixture and assertion matrix

`tests/paginator/contract.html`, its browser spec, and the dedicated
forced-colors gate must prove:

1. exact native landmark, select/label, range status, button/label/icon, and
   optional-control ownership;
2. numeric and option normalization without programmatic attribute rewriting or
   events;
3. pointer, Enter, Space, native select, event ordering/detail, first-item
   preservation, and application-owned data rendering;
4. focus-stable boundary transitions, initial boundary skipping, global
   disabled suppression, and silent live author control;
5. pointer enter/leave, hover/focus-visible/active/disabled coexistence, stable
   node/geometry/state, and zero transient mutation; and
6. exact normal/forced-color token resolution with surviving character icons
   and no CSS exception.

## Reference basis

- [Angular Material paginator overview](https://material.angular.dev/components/paginator/overview)
- [Pinned Angular Material paginator contract](https://github.com/angular/components/blob/v22.1.1/src/material/paginator/paginator.md)
- [WAI-ARIA APG landmark regions](https://www.w3.org/WAI/ARIA/apg/practices/landmark-regions/)
