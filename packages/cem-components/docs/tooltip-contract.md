# Tooltip Contract

This contract fixes the first CEM tooltip owner against the pinned Angular
Material `v22.1.1` behavior before component CSS is added. Angular is a
behavioral benchmark; CEM keeps one authored native trigger and uses browser
Popover and CSS Anchor Positioning rather than reproducing an Angular directive,
overlay service, injection defaults, or gesture interception.

## Owner and author vocabulary

`cem-tooltip` owns one supplemental plain-text description and its transient
visual presentation. Its strict payload contains exactly one supported native
trigger assigned to the `trigger` slot:

```html
<cem-tooltip message="Save the current document" position="above">
  <button slot="trigger" type="button">Save</button>
</cem-tooltip>
```

The supported native trigger set is `a[href]`, `button`, `input`, `select`,
`textarea`, and `summary`. The trigger remains the exact pointer, focus,
keyboard, activation, accessible-name, and application-event owner. The host,
generated description, and visible tooltip never become trigger substitutes.
Missing/blank `message`, zero or multiple named triggers, or an unsupported
trigger is authoring non-conformance and is not silently reinterpreted.

Public attributes are:

| Attribute | Contract |
| --- | --- |
| `message` | Required trimmed plain-text supplemental description. Markup and interactive tooltip content are out of scope; use a non-modal dialog for interactive hover content. |
| `position` | `above`, `below`, `before`, or `after`; invalid/missing values normalize to `below`. Logical `before`/`after` follow writing direction. Browser position-try fallbacks may flip the requested side to keep the tooltip visible. |
| `show-delay` | Optional finite non-negative milliseconds before automatic or manual presentation; invalid/missing values default to `0`. |
| `hide-delay` | Optional finite non-negative milliseconds before pointer/blur dismissal; invalid/missing values default to `0`. Escape and disabled suppression are immediate. |
| `open` | Presence-only declarative manual API. Adding it requests presentation and removing it releases the manual request without moving focus or emitting a component event. |
| `disabled` | Presence-only global suppression. It cancels pending presentation and closes the popover without removing authored `open`; removing `disabled` restores an outstanding manual request. |

## State and lifecycle contract

Automatic visibility has two independent reasons: a hover-capable pointer is
over the trigger or tooltip, or keyboard focus is on the trigger. Releasing one
reason MUST NOT hide while the other remains. Moving the pointer from the
trigger into the tooltip retains presentation. Pointer leave and blur use the
normalized hide delay; a renewed reason cancels pending dismissal.

`Escape` dismisses immediately without moving focus. It suppresses automatic
reopening until the pointer and focus reasons have both ended. If `open` supplied
the manual reason, Escape removes that attribute so serialization and visible
state remain truthful. No click, input, change, or component-specific event is
synthesized by show, hide, focus, pointer, or manual control.

The visible element uses `popover="manual"`, so its browser top-layer state is
the transient presentation truth. `open` is only the application-owned manual
request; automatic hover/focus presentation does not mutate host attributes.
Rendering preserves the trigger node, its focus, attributes, listeners, and
geometry. Native `beforetoggle`/`toggle` events from the Popover API remain
browser-owned and are not redispatched.

## Touch boundary

CEM deliberately does not reproduce Angular Material's long-press gesture.
Touch `pointerdown`, `pointerup`, `pointercancel`, click, selection, dragging,
and scrolling are never canceled, delayed, or restyled with `touch-action` or
`user-select`. A touch tap therefore activates the authored trigger exactly
once and does not automatically show the tooltip. The persistent accessible
description remains available to assistive technology, and an application that
needs a visible touch explanation may use the same `open` API.

This avoids making supplemental tooltip text depend on a discoverability-poor
gesture or taking over native input/drag behavior. Essential instructions MUST
remain visible in the application rather than existing only in a tooltip.

## Accessibility contract

The behavior creates one stable visually hidden plain-text description and
appends only that ID to the trigger's existing `aria-describedby` token list.
The description remains in the DOM independently of visual presentation. The
visible top-layer copy has `role="tooltip"`, contains the same text, has no
focusable descendants, and never receives focus.

When `cem-tooltip` or its native trigger is disabled, the component removes only
its own description reference and suppresses visual presentation; all authored
description IDs remain intact. Disconnect and malformed payload cleanup follow
the same ownership rule. The trigger keeps its authored accessible name and
normal tab position. Tooltip content supplements rather than replaces its name.

## Positioning and platform boundary

The host scopes one CSS anchor name, the exact native trigger establishes that
anchor, and the manual popover uses logical `position-area` placement with
browser flip fallbacks. This gives the tooltip top-layer draw order, viewport
avoidance, writing-mode behavior, and clipping escape without a numeric z-index,
portal clone, or JavaScript layout coordinates. CEM does not expose pointer-origin
placement or a general overlay service in this slice.

Popover and CSS Anchor Positioning are the platform baseline for this component.
If a target browser lacks either primitive, the tooltip's persistent accessible
description remains valid but the package does not invent a second positioning
engine.

## Theme-token audit

The pre-CSS audit found complete existing coverage:

| Visual role | Domain | Token |
| --- | --- | --- |
| Inverse neutral surface/text | D0 color | `--cem-palette-comfort-x`, `--cem-palette-comfort-text-x` |
| Trigger-to-overlay gap / compact inset | D1 space | `--cem-gap-related`, `--cem-inset-control` |
| Overlay corner geometry | D3 shape | `--cem-bend-overlay` |
| Contextual top-layer depth | D4 layering | `--cem-elevation-3` |
| Contour and hidden-description geometry | D5 stroke | `--cem-stroke-boundary`, `--cem-stroke-standard`, `--cem-stroke-none` |
| Supplemental UI text | D6 typography | `--cem-typography-ui-*` |

D7 was audited but is intentionally unused: visibility has no component
animation, so reduced-motion mode has nothing to shorten or remove. `show-delay`
and `hide-delay` are author-controlled interaction policy, not theme motion
durations. Native top-layer ordering supplies the physical adapter that D4 does
not encode, so this contract adds no entry to `components-css-exceptions.md`.

## Forced-colors boundary

The tooltip surface resolves explicitly to `Canvas`, text and contour to
`CanvasText`, and semantic shadow is removed by the generated forced-color
layering contract. The panel retains its D1/D3/D5 geometry, top-layer placement,
plain-text content, and automatic color adjustment. Visibility never depends on
shadow, hue, opacity, animation, or pointer-only access.

## Focused fixture and assertion matrix

`tests/tooltip/contract.html`, its browser spec, and the dedicated forced-colors
gate form the executable contract:

| Concern | Required evidence |
| --- | --- |
| Owner vocabulary | exactly one named supported native trigger, stable generated IDs, malformed payload rejection |
| Accessibility | authored name retained, existing descriptions preserved, persistent hidden description appended, visible non-focusable `role=tooltip` copy |
| Pointer lifecycle | trusted enter/leave on the trigger, continued visibility over the tooltip, delayed release, no wrapper ownership |
| Keyboard lifecycle | focus shows without moving focus; Escape and blur dismiss; focus and pointer reasons coexist |
| Manual/disabled | live `open`, show/hide delay, host/native disabled suppression, silent programmatic control |
| Touch/activation | touch is uncanceled, automatic touch presentation is absent, and the trigger's native click remains singular |
| Stability | transient presentation retains trigger identity, attributes, geometry, focus, and application event counts |
| Position/top layer | logical requested side, browser fallback, clipping escape, and no z-index declaration |
| Forced colors/motion | `Canvas`/`CanvasText`, automatic adjustment, stable geometry, and no component animation |

## Benchmark references

- [Pinned Angular Material tooltip contract](https://github.com/angular/components/blob/0b67c3c38141049657b1167479accc80e455d2bd/src/material/tooltip/tooltip.md)
- [Pinned Angular Material tooltip implementation](https://github.com/angular/components/blob/0b67c3c38141049657b1167479accc80e455d2bd/src/material/tooltip/tooltip.ts)
- [WAI-ARIA tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/)
