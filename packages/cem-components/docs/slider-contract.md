# Slider Contract

This contract fixes the first CEM single-value and range slider owner against
the pinned Angular Material `v22.1.1` behavior before component CSS is added.
Angular is a behavioral benchmark; CEM keeps native HTML range controls and
native form submission instead of reproducing Angular directives or callbacks.

## Owner and author vocabulary

`cem-slider` owns one horizontal value-range visual. Its authored payload is
strict and contains either one or two direct native inputs:

```html
<cem-slider min="0" max="100" step="1">
  <input type="range" data-cem-slider-thumb="single" name="volume" value="40" aria-label="Volume">
</cem-slider>

<cem-slider min="0" max="100" step="1">
  <input type="range" data-cem-slider-thumb="start" name="minimum" value="25" aria-label="Minimum">
  <input type="range" data-cem-slider-thumb="end" name="maximum" value="75" aria-label="Maximum">
</cem-slider>
```

One input MUST use `single`. Two inputs MUST contain exactly one `start` and one
`end`. The inputs remain the focus, pointer, keyboard, value, event, and form
owners after light-DOM projection. There is no `cem-slider-thumb` tag, tuple
value, component form value, or custom slider event. Additional or malformed
thumb payload is authoring non-conformance and is not silently reinterpreted.

Parent attributes are `min`, `max`, `step`, `disabled`, `discrete`, and
`show-tick-marks`. Finite `min` defaults to `0`; finite `max` must exceed the
normalized minimum and otherwise defaults to `min + 100`; finite positive
`step` defaults to `1`. `disabled` is the global authority and is projected to
both native inputs. Input-level `disabled` is not an independent public state.

`discrete` exposes a visual value label for every thumb. The visible text uses
authored `aria-valuetext` when present and otherwise the native value. Localized
or domain-specific display formatting remains application-owned through
`aria-valuetext`; CEM does not reproduce Angular's JavaScript `displayWith`
callback. `show-tick-marks` renders step-aligned visual ticks without creating
accessible nodes or changing the input step.

## State and range contract

Each native input receives the normalized parent `min`, `max`, and `step`.
Single mode uses the input value. Range mode guarantees `start <= end`; equal
values are valid. Native value normalization and step behavior remain browser
owned. If user input attempts to cross the peer, the changing native value is
clamped to the peer before the event reaches application listeners.

Programmatic parent attribute changes update bounds, disabled state, and visual
positions silently. Programmatic native `value` property changes retain HTML's
native silence; an application that wants observers notified dispatches the
native event it owns. Rendering preserves each input node, form owner, focus,
name, default value, and application listener.

Hover and active state belong only to the enabled input thumb under the pointer.
Focus-visible belongs only to the focused enabled input thumb. These transient
states change paint without changing host/input attributes, rendered structure,
values, events, or geometry. Disabled wins over hover, active, and focus paint.

## Event and keyboard contract

Native range behavior owns pointer dragging and the standard keyboard surface:
ArrowRight/ArrowUp increase, ArrowLeft/ArrowDown decrease, PageUp/PageDown make
larger changes, and Home/End move to bounds. In range mode the non-crossing
constraint applies to every route.

The exact native thumb emits its ordinary bubbling `input` events while its
value changes and `change` on commit. CEM neither cancels nor redispatches those
events, and emits no component-specific event. Therefore event `target`,
`isTrusted`, ordering, `name`, value, and native `FormData` serialization stay
browser-owned. Pointer enter/leave and keyboard focus never synthesize input or
change events.

## Accessibility contract

Every authored input remains a native horizontal slider with browser-exposed
`role=slider`, current value, normalized bounds, and step semantics. Authors
MUST give the single thumb an accessible name and MUST give range thumbs
distinct accessible names using native `<label>`, `aria-label`, or
`aria-labelledby`. Authored `aria-describedby` and `aria-valuetext` remain on
the same input. The generated visual track, tick marks, and value labels are
`aria-hidden` and never add tab stops or competing slider roles.

The host and generated structural wrappers are not focusable, clickable, or
hover owners. Sequential focus visits enabled inputs in authored order and
skips both when the host is disabled.

## Theme-token audit

The audit found no existing CEM theme family that can represent slider paint
without collapsing distinct semantics. A slider is value input, so its
remaining and active track are not `--cem-progress-*`; the track is not a D2
divider between sibling regions; and thumb hover/active is not a command-action
surface. Canonical theme semantics therefore precede component CSS:

| Visual role | Domain | Token |
| --- | --- | --- |
| Remaining track | D0 color | `--cem-slider-track-color` |
| Selected track | D0 color | `--cem-slider-active-track-color` |
| Default/hover/active thumb | D0 color | `--cem-slider-thumb-color`, `--cem-slider-thumb-hover-color`, `--cem-slider-thumb-active-color` |
| Disabled track/thumb | D0 color | `--cem-slider-disabled-track-color`, `--cem-slider-disabled-thumb-color` |
| Tick marks | D0 color | `--cem-slider-tick-color` |
| Track thickness / visible thumb | D2c control geometry | `--cem-slider-track-thickness`, `--cem-slider-thumb-size` |
| Minimum pointer target | D2 coupling | `--cem-coupling-zone-min` |
| Circular thumb shape | D3 shape | `--cem-bend-circle` |
| Keyboard focus | D5 stroke/zebra | `--cem-stroke-focus`, `--cem-stroke-indicator-offset`, `--cem-zebra-color-1` |
| Value-label separation/type | D1/D6 | `--cem-gap-related`, data typography |

Dynamic thumb/track percentages are normalized value data, not theme constants.
They may be written as private runtime CSS properties; component CSS must still
contain no public token invention, raw color, or spacing constants. This audit
adds no entry to `components-css-exceptions.md`.

## Forced-colors boundary

Generated theme CSS maps the remaining and disabled track/thumb to `GrayText`,
the active track and enabled hover/active thumb to `Highlight`, the resting
thumb and tick marks to `CanvasText`, and the focus outline to `CanvasText`.
Component CSS keeps automatic forced-color adjustment and does not use shadows,
alpha, or motion as the only signal. Tick geometry and the active/remaining
track split survive independent of authored hue.

## Focused fixture and assertion matrix

`tests/slider/contract.html`, its browser spec, and the dedicated forced-colors
gate form the executable contract:

| Concern | Required evidence |
| --- | --- |
| Owner vocabulary | exact single and start/end native inputs; no custom thumb owner |
| Bounds/range | defaults, normalized live parent bounds, browser step behavior, equal/non-crossing range values |
| Native events | trusted pointer/keyboard input/change target the exact input; no synthetic component event |
| Forms | independent authored names and values serialize through native `FormData` |
| Accessibility | distinct names, native slider semantics, hidden visuals, exact keyboard order |
| State paint | enabled thumb hover/active, focus-visible coexistence, disabled precedence and suppression |
| Stability | pointer enter/leave and focus retain dimensions, DOM identity, attributes, values, and event counts |
| Optional visuals | tick marks and discrete labels mirror normalized values without accessible duplication |
| Forced colors | system track/thumb/tick/focus colors and stable geometry with automatic adjustment |

## Benchmark references

- [Pinned Angular Material slider contract](https://github.com/angular/components/blob/v22.1.1/src/material/slider/slider.md)
- [WAI-ARIA slider pattern](https://www.w3.org/WAI/ARIA/apg/patterns/slider/)
