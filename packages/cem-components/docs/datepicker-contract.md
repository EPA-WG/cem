# Datepicker Contract

This contract fixes the first CEM calendar date picker against the pinned
Angular Material `v22.1.1` catalog before component CSS is added. Angular and
the WAI-ARIA date-picker combobox are behavioral benchmarks; CEM keeps one
native form control and a canonical string date instead of reproducing Angular
directives, date objects, adapters, services, or selection strategies.

## Owner and author vocabulary

`cem-datepicker` owns one single-date calendar around exactly one direct native
text input assigned to the `input` slot:

```html
<label for="arrival-date">Arrival date</label>
<cem-datepicker min="2026-08-01" max="2026-12-31" required>
  <input
    id="arrival-date"
    slot="input"
    type="text"
    name="arrival-date"
    value="2026-08-11"
    aria-describedby="arrival-format"
  >
  <button slot="toggle" type="button" aria-label="Choose arrival date">Choose</button>
</cem-datepicker>
<span id="arrival-format">Use YYYY-MM-DD.</span>
```

The optional direct `button[slot="toggle"][type="button"]` is a native dialog
activation owner. The input remains the exact editable value, accessible name,
constraint, event, and form owner after light-DOM projection. The host is not
form-associated and exposes no competing `value`, `name`, JavaScript `Date`, or
custom date event. Authors MUST label the input through native `label[for]`,
`aria-label`, or `aria-labelledby`, SHOULD describe the canonical input format,
and MUST label an authored toggle.

Additional input or toggle owners, a non-text input, a submitting toggle, or
any unrelated element payload is authoring non-conformance and suppresses the
calendar. The calendar grid, heading, weekday labels, previous/next controls,
and Cancel/Apply actions are generated structural and native-interaction owners;
authors do not replace them in this first contract.

Public host attributes are:

| Attribute | Contract |
| --- | --- |
| `min` | Earliest selectable canonical date; defaults to `0001-01-01`. |
| `max` | Latest selectable canonical date; defaults to `9999-12-31`. `min` after `max` is invalid authoring. |
| `disabled` | Presence-only global authority projected to the input and optional toggle; it closes and suppresses the dialog. |
| `required` | Presence-only native required constraint projected to the input. |
| `invalid` | Presence-only application validation state projected as `aria-invalid` without inventing a second form owner. |
| `lang` | Optional locale source for month, weekday, and full-date labels; otherwise normal inherited language and runtime locale fallback apply. |

Input-level `disabled` and `required` are not independent public states. Input
`name`, `form`, `value`, `defaultValue`, `autocomplete`, and authored accessible
relationships remain native and application-owned.

## Value, calendar, and validation

The only accepted non-empty value shape is a real zero-padded Gregorian date
`YYYY-MM-DD` with a four-digit year from `0001` through `9999`. That exact string
is edited, selected, and submitted; empty remains available for optional or
required native validation. Calendar labels use `Intl.DateTimeFormat` with UTC
date parts so localization cannot change or timezone-shift the canonical value.
The locale's week origin orders the seven weekday columns when available, with
Sunday as the compatibility fallback.

The selected input value determines the opening month when it is canonical and
within bounds. Otherwise the picker opens at today's local calendar date,
clamped to the accepted range. The month view renders complete weeks, including
adjacent-month dates, and marks out-of-range dates disabled. Previous/next month
controls are disabled when the destination month contains no selectable date.
Today is current contextual information, not selection: its D0 indicator remains
visible when another date is selected, drafted, hovered, or focused.

The native input's custom-validity surface reports malformed and out-of-range
non-empty values. Empty required validation remains browser-owned. The behavior
updates `aria-invalid` for computed or authored invalidity and never moves the
value onto the host. Programmatic input-property writes remain silent until the
application dispatches the native event it owns. Native form reset restores the
input's `defaultValue`; selection, validity, and the next opening month then
resynchronize without another event.

This slice deliberately excludes ranges, comparison/preview ranges, `step`,
date filters, alternate year or multi-year views, date/time composition,
timezones, JavaScript `Date`/Temporal values, and locale parsing/format adapters.
The Angular Material datepicker row therefore advances from gap to partial, not
covered.

## Interaction and event contract

Clicking the enabled toggle, pressing `ArrowDown`, or pressing `Alt+ArrowDown`
on the enabled input opens the generated native modal dialog. Ordinary input
click, horizontal arrows, character input, selection, clipboard commands, and
undo remain native text editing. Opening moves focus to the selected enabled day
or the clamped current date and keeps exactly one calendar day in the roving tab
order. The native modal boundary contains sequential focus.

Inside the grid, Left/Right moves one day, Up/Down moves one week, and Home/End
moves to the first/last day of the locale-ordered week. PageUp/PageDown keeps the
day number where possible in the previous/next month; Shift or Alt with PageUp/
PageDown moves by a year. Movement crosses displayed months and clamps at
`min`/`max`. Disabled dates never become the draft or receive activation.

Pointer click, Space, or Enter on an enabled day changes only the dialog draft
selection and keeps the input value/event state untouched. Apply writes the
canonical draft to the exact input, closes the dialog, returns focus to the
input, then dispatches one bubbling `input` followed by one bubbling `change`,
both targeted at that input. Cancel, Escape, backdrop activation, focus/
navigation movement, month changes, validation, and programmatic host changes
close or update presentation without committing a value or emitting an input,
change, or custom component event. Reopening starts from the committed input
value, so a cancelled draft is never retained.

The host exposes a silent read/write `expanded` property for application
control. It neither reflects an attribute nor emits a lifecycle event.

## Accessibility contract

The authored input receives `role="combobox"`, `aria-autocomplete="none"`,
`aria-haspopup="dialog"`, stable `aria-controls`, and truthful `aria-expanded`.
The optional toggle receives the same popup relationship and expanded state but
does not become the combobox or form owner.

The generated native `dialog` has a stable accessible name and contains a
polite month heading, exact previous/next native buttons, a `role="grid"`
labelled by that heading, seven localized `role="columnheader"` labels, complete
`role="row"` weeks, and native day buttons exposed as `role="gridcell"` with
roving `tabindex`, `aria-selected`, `aria-current="date"`, and `aria-disabled`
where applicable. Cancel and Apply are native buttons. The host and visual
wrappers are neither focusable nor clickable.

Focus, current date, draft/committed selection, pointer hover, and disabled
meaning are independent. Disabled semantics and behavior win when states
coexist. Native modal Escape and inert-background behavior are retained; close
restores focus to the input.

## Dialog and theme-token audit

The calendar uses the native modal-dialog top layer. The exact input is its CSS
anchor; logical block placement and browser fallback keep the dialog near the
owner without portals, JavaScript coordinates, or numeric z-index.

The pre-CSS audit found one missing D0 meaning and otherwise complete coverage:

| Visual role | Domain | Token |
| --- | --- | --- |
| Input default/hover/invalid/disabled/expanded indicator | D0/D5 | existing `--cem-input-indicator-*`, zebra, and stroke endpoints |
| Toggle, month navigation, Cancel, and Apply states | D0 | existing `--cem-action-contextual-*` endpoints |
| Dialog surface and contour | D0 | shared `--cem-select-popup-*` endpoints |
| Day default/hover/selected/selected-hover/disabled | D0 | existing `--cem-content-interaction-*` endpoints |
| Today's independent current-date indicator | D0 | new `--cem-content-interaction-current-indicator-color` endpoint |
| Owner/dialog/action/grid spacing | D1 | `--cem-gap-related`, `--cem-gap-group`, `--cem-control-padding-*` |
| Operable targets and grid geometry | D2/D2c | `--cem-coupling-zone-min`, `--cem-control-height` |
| Dialog and cell shape | D3 | `--cem-bend-overlay`, `--cem-bend-control` |
| Semantic modal depth | D4 | `--cem-elevation-3` |
| Contour, today, selection, and focus | D5 | existing boundary/selected/focus strokes and indicator offset |
| Month, weekday, day, and action text | D6 | existing UI/data typography endpoints |

The new D0 endpoint is added to the canonical theme token table and generated
outputs before component CSS. Native top-layer ordering supplies physical draw
order, so `CEM-CSS-002` does not apply and this contract adds no exception entry.
D7 is unused because no component animation or transition is introduced.

## Forced-colors boundary

The input and native actions retain automatic platform adjustment. The dialog
resolves to `Canvas`/`CanvasText`; enabled day hover/active resolves to
`Highlight`/`HighlightText`; selection retains `SelectedItem`; today retains
`Mark`; disabled resolves to `GrayText`; keyboard focus resolves to
`CanvasText`. Geometry, today/selection/focus coexistence, dialog modality, and
day text remain visible without shadow, authored hue, opacity, animation, or
numeric z-index.

## Focused fixture and assertion matrix

`tests/datepicker/contract.html`, its browser spec, and the dedicated
forced-colors gate form the executable contract:

| Concern | Required evidence |
| --- | --- |
| Owners | exact direct native text input, optional native toggle, generated native dialog/actions, strict malformed-payload rejection |
| Value/forms | canonical real `YYYY-MM-DD`, exact input serialization, native default/reset ownership, no host form value |
| Calendar | selected/today opening month, locale heading/week order, adjacent-month dates, bounded month controls |
| Validation | required, syntax, real-date and range errors, authored invalid, live constraints, native input validity anchor |
| Keyboard | input open, roving grid movement, locale Home/End, month/year Page movement, draft, Apply/Cancel, Escape |
| Pointer/events | toggle open, day draft, Apply commit order/target, Cancel/backdrop silence, disabled suppression |
| Focus/modality | native modal containment, exact active day, one roving gridcell, close restoration, inert background |
| State/stability | hover/focus/current/selected/disabled coexistence with stable input/day geometry, identity, and transient input/event/host state |
| Forced colors | system input/dialog/action/day paint, current/selection/focus coexistence, top layer, no shadow/motion/z-index dependence |

## Benchmark references

- [Pinned Angular Material datepicker contract](https://github.com/angular/components/blob/0b67c3c38141049657b1167479accc80e455d2bd/src/material/datepicker/datepicker.md)
- [WAI-ARIA date-picker combobox example](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-datepicker/)
- [HTML date value model](https://html.spec.whatwg.org/multipage/input.html#date-state-(type=date))
