# Sort Header Contract

This contract fixes the sortable-column action for `@epa-wg/cem-components`.
It is benchmarked against Angular Material `v22.1.1` and the WAI-ARIA sortable
table pattern while retaining CEM's declarative state and theme vocabulary.

## Owner and author vocabulary

`cem-sort-header` owns one sortable column header. The generated
`div[role="columnheader"]` owns column and `aria-sort` semantics; its direct
native button owns hover, focus, active, disabled, pointer, and keyboard input.
The custom-element host and column-header wrapper are structural and never
receive interaction paint or focusability.

The public author surface is:

- `label`: visible column label and source for `Sort by …`; defaults to `Column`;
- `name`: stable application identity included in `cem-sort` detail;
- `direction`: `ascending` or `descending`; absence means no active sort;
- `disabled`: presence disables the native button and user activation.

Authored `direction="none"` and invalid values render as no direction without
rewriting the authored value. On the next user activation they enter the same
cycle as an absent value. There are no public start-direction, disable-clear,
color, indicator, active, hover, focus, selected, or current options in v1.

`cem-table` remains a passive labeled table wrapper. It does not reorder data,
own column definitions, announce application results, or become an interaction
owner merely because it contains sort headers.

## State and geometry contract

The fixed user cycle is none -> ascending -> descending -> none. Ascending or
descending user activation clears the `direction` attribute from other direct
sort-header peers in the nearest `cem-table`, including a previously sorted
disabled peer. A clear activation removes only the activated header's
attribute. Headers outside that nearest table and headers inside nested tables
are independent. Applications that author initial or programmatic direction
changes remain responsible for supplying at most one active direction per
table.

Direction is redundantly distinguishable through text characters and ARIA:

| Direction | Host attribute | `aria-sort` | Hidden visual mark |
|---|---|---|---|
| None | absent, `none`, or invalid | absent | `◇` |
| Ascending | `direction="ascending"` | `ascending` | `▲` |
| Descending | `direction="descending"` | `descending` | `▼` |

The mark is `aria-hidden="true"`; it never becomes the accessible name. Its
fixed D2c box and the button's full-width target remain stable across direction,
hover, focus-visible, active, and disabled states. Live direction changes reuse
the column-header, native button, label, and indicator nodes.

## Event and keyboard contract

The native button supplies pointer click, Enter, and Space activation. Each
trusted activation advances the cycle exactly once and dispatches one
non-cancellable `cem-sort` custom event with `bubbles: true`, `composed: true`,
and JSON-serializable detail:

```json
{
  "name": "created",
  "direction": "ascending",
  "previousDirection": "none"
}
```

Clearing reports `direction: "none"`. The component never emits synthetic
`click`, `input`, or `change` events. Programmatic attribute changes emit no
event. Disabled suppresses native pointer and keyboard activation, while
programmatic direction control remains available.

Pointer enter/leave, hover, focus movement, and held active input change only
transient paint. They do not mutate host attributes, ARIA, runtime slices,
sibling state, or event detail, and focus-visible remains visible while hover
coexists.

## Accessibility contract

Only an ascending or descending column-header owner exposes `aria-sort`, and
user activation keeps it on no more than one header in the nearest table. The
button has the visible label plus the explicit accessible action name
`Sort by <label>`. Native disabled semantics remove disabled headers from the
tab order and suppress activation.

The component does not create a live region. As with Angular Material's sort
change output, the application consumes `cem-sort`, reorders its rows, and then
updates a localized polite status message describing the resulting sort. This
keeps data ownership and result wording together and avoids announcing a visual
direction change before application data has actually changed.

## Theme-token audit

The pre-CSS audit found complete existing semantic coverage:

| Concern | Canonical owner | Binding |
|---|---|---|
| Default, hover, active, disabled paint | D0 contextual action | `--cem-action-contextual-*-background/text` |
| Label/mark relationship | D1 gap | `--cem-gap-related` |
| Target floor and table rhythm | D2/D2c geometry | `--cem-coupling-zone-min`, `--cem-table-row-height` |
| Control padding and mark box | D2c control geometry | `--cem-control-padding-*`, `--cem-icon-button-icon-size` |
| Shape | D3 bend | `--cem-bend-control` |
| Focus ring | D5 stroke plus zebra | `--cem-stroke-focus`, `--cem-stroke-indicator-offset`, `--cem-zebra-color-1` |
| Button text | D6 UI typography | `--cem-typography-ui-*` |

Sort direction is state communicated by `aria-sort` and distinct character
shapes, not a color category. No theme addition and no
`components-css-exceptions.md` entry are required.

## Forced-colors boundary

With forced colors active, the button uses `Canvas` / `CanvasText`, hover and
active use `Highlight` / `HighlightText`, disabled uses `Canvas` / `GrayText`,
and focus uses the same D5 width/offset with `CanvasText`. Automatic forced
color adjustment remains enabled. The three text indicators survive without
background images, masks, or color-only meaning, and all state geometry stays
fixed.

## Focused fixture and assertion matrix

`tests/sort-header/contract.html`, its browser spec, and the dedicated
forced-colors gate must prove:

1. exact table, column-header, direct native-button, accessible-name, hidden
   mark, and conditional `aria-sort` ownership;
2. pointer enter/leave, hover restoration, held active paint, focus-visible plus
   hover coexistence, disabled suppression, and stable geometry;
3. pointer, Enter, and Space cycling with one ordered `cem-sort` event each and
   no synthetic input/change path;
4. nearest-table sibling clearing, nested/other-table independence, live author
   changes, node reuse, invalid-value normalization, and no programmatic event;
5. application-owned row ordering and live announcement boundaries; and
6. exact normal/forced-color token resolution with character-distinct states
   and no CSS exception.

## Reference basis

- [Angular Material sort overview](https://material.angular.dev/components/sort/overview)
- [WAI-ARIA APG sortable table example](https://www.w3.org/WAI/ARIA/apg/patterns/table/examples/sortable-table/)
