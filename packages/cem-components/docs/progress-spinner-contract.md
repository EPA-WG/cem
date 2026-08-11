# Progress Spinner Contract

This contract fixes the first circular progress owner for
`@epa-wg/cem-components`. It is benchmarked against Angular Material
`v22.1.1`, but it uses CEM's consumer-semantic state and theme vocabulary.

## Owner and author vocabulary

`cem-progress-spinner` is a non-interactive circular progress indicator. It is
distinct from the native linear `cem-progress`; neither owner changes shape to
stand in for the other.

The public author surface is:

- `label`: accessible name, with `Progress` as the safe fallback;
- `value`: its presence selects determinate mode; its absence selects
  indeterminate mode;
- `max`: the determinate maximum, defaulting to `100`;
- `describedby`: optional ID reference for additional task context.

There is no separate `mode` attribute. Inferring mode from whether a value is
known prevents contradictory `mode="indeterminate" value="…"` states and
matches the existing CEM linear-progress vocabulary. There are no public
diameter, stroke-width, color, disabled, selected, current, focus, or animation
speed attributes; those concerns are either inapplicable or theme-owned.

## State and geometry contract

When `value` is absent, the owner renders `data-mode="indeterminate"` and a
fixed incomplete arc. When `value` is present, it renders
`data-mode="determinate"` and the arc represents the normalized value.

Normalization is render-only and never rewrites author attributes:

- a missing, non-finite, or non-positive `max` resolves to `100`;
- an empty or non-finite determinate `value` resolves to `0`;
- a finite value is clamped to the inclusive range `0..max`;
- adding, changing, or removing `value`/`max` updates the same owner in place.

The generated SVG uses a stable `0 0 100 100` view box and two circles with
`pathLength="100"`: a full remaining-range track and one indicator arc. D2c
owns outer diameter and track thickness. Mode/value changes may alter only
indicator dash geometry and animation; they must not change the host or SVG
border box, replace the SVG owner, or move neighboring layout.

## Event and keyboard contract

The spinner has no activation or editing behavior. It registers no pointer,
click, keyboard, input, or change handlers, has no tab stop, and dispatches no
component event. Pointer enter/leave, click, Enter, and Space therefore do not
change mode, value, DOM, ARIA, geometry, or runtime slices. Keyboard focus moves
between the surrounding authored controls and skips the spinner.

`disabled`, `selected`, `checked`, `current`, `hover`, `active`, and
`focus-visible` are deliberately not spinner states. Availability belongs to
the operation that started the work, and a progress indicator must not suppress
or mutate that operation. Applications remain responsible for `aria-busy` on
the affected region and for removing the spinner when work completes.

## Accessibility contract

The rendered visual owner has `role="progressbar"`, its accessible name comes
from `label`, and its SVG is `aria-hidden="true"` and non-focusable.

Determinate mode exposes normalized `aria-valuemin="0"`, `aria-valuemax`, and
`aria-valuenow`. Indeterminate mode keeps the implicit `0..100` range and omits
`aria-valuenow`; absence of a known value, rather than a live-region message,
communicates indeterminacy. The component does not add `aria-live`, `role=status`,
or `aria-busy` and does not announce every visual cycle.

## Theme-token audit

The pre-CSS audit found appropriate CEM categories but missing semantic
endpoints, so the theme is extended before the component stylesheet:

| Concern | Canonical owner | Binding |
|---|---|---|
| Remaining-range track | D0 color | `--cem-progress-track-color` |
| Current/working arc | D0 color | `--cem-progress-indicator-color` |
| Circular diameter | D2c control geometry | `--cem-progress-spinner-size` |
| Track/indicator thickness | D2c control geometry | `--cem-progress-track-thickness` |
| Repeated cycle period | D7 time | `--cem-duration-continuous-cycle` |
| Mechanical cycle easing | D7 time | `--cem-easing-uniform` |

The progress track is not a divider: it represents remaining range, not sibling
separation, so it does not reuse `--cem-separator-color`. D5 also does not own
the track thickness because the line is the component's data graphic rather
than a boundary, divider, focus, selection, or target stroke.

All required values now have canonical theme categories. No entry is added to
`components-css-exceptions.md`.

## Motion and reduced-motion contract

Determinate mode is static. Indeterminate mode repeats one neutral rotation
using `--cem-duration-continuous-cycle` and `--cem-easing-uniform`; the fixed arc
geometry remains sufficient to communicate unknown progress when motion is not
available.

Under `prefers-reduced-motion: reduce`, component CSS removes the animation
rather than accelerating it, shortening it, hiding the indicator, or changing
ARIA. The static incomplete arc, size, track thickness, colors, DOM, and
accessible semantics remain unchanged.

## Forced-colors boundary

Theme generation maps `--cem-progress-track-color` to `GrayText` and
`--cem-progress-indicator-color` to `Highlight` in native and
`forced-colors: active` modes. Component CSS keeps `forced-color-adjust: auto`,
does not opt out of system adjustment, and does not introduce a component-local
color fallback. Geometry continues to resolve from D2c tokens, while reduced
motion remains an independent media preference.

The forced-colors gate must prove the exact system-color pair, visible track and
indicator strokes, stable geometry, programmatic semantics, event neutrality,
and automatic color adjustment in both determinate and indeterminate modes.

## Focused fixture and assertion matrix

`tests/progress-spinner/contract.html` is declarative and script-free. The
focused browser suite and forced-colors gate cover:

| Surface | Required assertion |
|---|---|
| Public owner | exact `cem-progress-spinner` light-DOM owner and persistent SVG |
| Mode | missing `value` is indeterminate; present `value` is determinate |
| Normalization | default/invalid `max`, invalid value, lower clamp, and upper clamp |
| Accessibility | name, progressbar role, exact value attributes, hidden/non-focusable SVG |
| Live attributes | `value`/`max` updates reuse the owner and do not rewrite author input |
| Geometry | D2c size/thickness resolve exactly and remain stable across modes/values |
| Paint | both strokes resolve through the D0 progress token pair |
| Motion | only indeterminate mode animates; reduced motion retains a static arc |
| Keyboard | spinner is skipped between surrounding focusable controls |
| Events | pointer/click/keyboard probes dispatch no spinner-owned mutation event |
| Forced colors | `GrayText` track, `Highlight` indicator, and `forced-color-adjust:auto` |
