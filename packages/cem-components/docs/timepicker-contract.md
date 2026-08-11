# Timepicker Contract

This contract fixes the first CEM time-of-day picker against the pinned Angular
Material `v22.1.1` behavior before component CSS is added. Angular is a
behavioral benchmark; CEM keeps a native form control and a string time-of-day
model instead of reproducing Angular date objects, directives, adapters, or
overlay services.

## Owner and author vocabulary

`cem-timepicker` owns one time-of-day choice popup around exactly one direct
native text input assigned to the `input` slot:

```html
<label for="meeting-time">Meeting time</label>
<cem-timepicker min="09:00" max="17:00" interval="30">
  <input id="meeting-time" slot="input" type="text" name="meeting-time" value="09:30">
  <button slot="toggle" type="button" aria-label="Choose meeting time">&#128339;</button>
</cem-timepicker>
```

The optional direct `button[slot="toggle"][type="button"]` is a native popup
activation owner. The input remains the exact focus, editable value, accessible
name, constraint, event, and form owner after light-DOM projection. The host is
not form-associated and exposes no competing `value`, `name`, or custom time
event. Authors MUST label the input through native `label[for]`, `aria-label`,
or `aria-labelledby` and MUST label an authored toggle.

Additional input or toggle owners, a non-text input, a submitting toggle,
`cem-option-group`, legacy `option`, or unrelated element payload is authoring
non-conformance and suppresses the popup. Direct `cem-option` children are the
only custom-choice vocabulary. Every option requires a unique canonical value;
its `label` attribute or collapsed text is the visible label. Interactive option
descendants remain forbidden by the shared choice-option contract.

Public host attributes are:

| Attribute | Contract |
| --- | --- |
| `min` | Earliest valid canonical time; defaults to `00:00`. |
| `max` | Latest valid canonical time; defaults to `23:59`. `min` after `max` is invalid authoring; overnight ranges are out of scope. |
| `interval` | Positive whole minutes used only to generate choices, default `30`, maximum `1440`. It does not impose step-mismatch validation on a typed value. |
| `disabled` | Presence-only global authority projected to the input and optional toggle; it closes and suppresses the popup. |
| `required` | Presence-only native required constraint projected to the input. |
| `invalid` | Presence-only application validation state projected as `aria-invalid` without inventing a second form owner. |

Input-level `disabled` and `required` are not independent public states. The
host owns those two projections so live removal is deterministic. Input
`name`, `form`, `value`, `defaultValue`, `autocomplete`, and authored accessible
relationships remain native and application-owned.

## Value, choices, and validation

The only accepted non-empty value shape is zero-padded 24-hour `HH:mm` from
`00:00` through `23:59`. That exact string is the edited, selected, and submitted
value; empty remains available for optional or required native validation.
Locale-specific or 12-hour option presentation is application-owned through
`cem-option label`, but selection still writes the canonical value to the input.
This slice intentionally has no date portion, time zone, seconds, Angular date
adapter, or datepicker integration.

With no authored `cem-option`, choices begin at normalized `min` and advance by
`interval` while they remain at or before `max`. A non-aligned maximum is not
invented as an extra choice. Direct options replace generation. Their value
must be canonical; authored `disabled` is honored, and values outside the host
range are rendered disabled. Input value, not an option's authored `selected`
attribute, is the selection authority.

The behavior uses the native input's custom-validity surface for malformed and
out-of-range non-empty values. Empty required validation remains browser-owned.
It updates `aria-invalid` for computed or authored invalidity and never moves
the value onto the host. Programmatic input-property writes remain silent until
the application dispatches the native event it owns. Native form reset restores
the input's `defaultValue`; the selected visual and validity then resynchronize
without emitting another event.

## Interaction and event contract

Clicking the enabled input or toggle opens the popup. The toggle's native click
remains singular and then returns focus to the combobox input so list navigation
has one focus owner. Outside pointer down or focus leaving the component closes
the popup. Disabled state suppresses every route.

With input focus, `ArrowDown` opens at the selected or first enabled choice and
`ArrowUp` opens at the selected or last enabled choice. Subsequent Up/Down moves
the active descendant, skipping disabled options. `Enter` commits the active
choice; `Escape` closes without changing the value; `Tab` closes and follows
native focus order. Home, End, horizontal arrows, character input, selection,
clipboard commands, and undo remain native text-editing keys.

Pointer down on an enabled option retains input focus and click commits it.
Commit writes the canonical value to the exact input, then dispatches one
bubbling `input` followed by one bubbling `change`, both targeted at that input.
Typing uses the browser's original input/change events; the behavior neither
cancels nor redispatches them. Opening, closing, hover, focus, active movement,
validation, and programmatic host changes emit no input, change, or custom
component event and do not mutate the input value.

## Accessibility contract

The authored input receives `role="combobox"`, `aria-autocomplete="list"`,
`aria-haspopup="listbox"`, stable `aria-controls`, truthful `aria-expanded`, and
an `aria-activedescendant` only while an enabled choice is active. DOM focus
remains on this input while a generated `role="listbox"` exposes stable
`role="option"`, `aria-selected`, and `aria-disabled` states. The optional
toggle receives the same popup relationship and expanded state but never
becomes the combobox or listbox focus owner.

The host and generated structural wrappers are not focusable or clickable.
Sequential focus reaches the input and optional native toggle in authored order
when enabled and skips both under host disabled. Popup options do not enter the
tab order. Active, selected, hovered, and disabled option meanings remain
independent; disabled paint and behavior win when states coexist.

## Popup and theme-token audit

The popup uses `popover="manual"` and the browser top layer. The exact input is
the CSS anchor; logical block placement and browser fallback keep the list near
the owner without JavaScript coordinates, portals, or numeric z-index. Popover
top-layer state is transient presentation truth. The host exposes a read/write
`expanded` property for application control, but automatic interaction does not
reflect a host attribute or emit a toggle event.

The pre-CSS audit found complete existing coverage:

| Visual role | Domain | Token |
| --- | --- | --- |
| Input default/hover/invalid/disabled/expanded indicator | D0/D5 | existing `--cem-input-indicator-*`, zebra, and stroke endpoints |
| Toggle command states | D0 | existing `--cem-action-contextual-*` endpoints |
| Popup and option default/hover/active/selected/disabled colors | D0 | shared `--cem-select-*` endpoints |
| Input/popup separation and inset | D1 | `--cem-gap-related`, `--cem-control-padding-*` |
| Control target and bounded popup rows | D2/D2c | `--cem-coupling-zone-min`, `--cem-list-row-height`, `--cem-list-popup-rows` |
| Popup shape | D3 | `--cem-bend-overlay` |
| Semantic overlay depth | D4 | `--cem-elevation-3` |
| Contour and focus | D5 | `--cem-stroke-standard`, `--cem-stroke-focus`, `--cem-stroke-indicator-offset` |
| UI text | D6 | `--cem-typography-ui-*` |

Native top-layer ordering supplies physical draw order, so the bounded
`CEM-CSS-002` positioned-flow adapter does not apply and this contract adds no
entry to `components-css-exceptions.md`. D7 is unused because no component
animation or transition is introduced.

## Forced-colors boundary

The input and toggle retain automatic platform adjustment. The popup resolves
to `Canvas`/`CanvasText`; active and hover resolve to `Highlight`/
`HighlightText`; selected retains a `SelectedItem` inset outline; disabled
resolves to `GrayText`; keyboard focus resolves to `CanvasText`. Geometry,
selected/active coexistence, top-layer placement, and option text remain visible
without shadow, authored hue, opacity, animation, or numeric z-index.

## Focused fixture and assertion matrix

`tests/timepicker/contract.html`, its browser spec, and the dedicated
forced-colors gate form the executable contract:

| Concern | Required evidence |
| --- | --- |
| Owners | exact direct native text input, optional native toggle, strict malformed-payload rejection |
| Value/forms | canonical `HH:mm`, exact input serialization, native default/reset ownership, no host form value |
| Choices | default/custom intervals, labels, range-disabled and authored-disabled options, input-owned selection |
| Validation | required, syntax, range, authored invalid, live constraints, native input validity anchor |
| Keyboard | open/navigation/skip/commit, Escape/no value change, Tab dismissal, native editing keys retained |
| Pointer/events | input/toggle open, option commit order and target, outside close, disabled suppression |
| State/stability | hover/focus-visible/expanded/active/selected/disabled coexistence with stable geometry, identity, and transient event/value/host state |
| Accessibility | native label, combobox/listbox references, active descendant, selected/disabled option semantics, exact tab owners |
| Forced colors | system input/popup/option/focus paint, stable geometry, top layer, no shadow/motion/z-index dependence |

## Benchmark references

- [Pinned Angular Material timepicker contract](https://github.com/angular/components/blob/0b67c3c38141049657b1167479accc80e455d2bd/src/material/timepicker/timepicker.md)
- [Pinned Angular Material timepicker input implementation](https://github.com/angular/components/blob/0b67c3c38141049657b1167479accc80e455d2bd/src/material/timepicker/timepicker-input.ts)
- [WAI-ARIA combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)
- [HTML time value model](https://html.spec.whatwg.org/multipage/input.html#time-state-(type=time))
