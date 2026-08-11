# Stepper Contract

## Owner and author vocabulary

`cem-stepper` is the public workflow-navigation owner. Its only authored element
children are direct `cem-step` payloads. Each step requires a non-empty `value`
identity and `label`; values are unique within the owner. `cem-step` is otherwise
inert: it contributes its default content to one generated panel and owns no
focus, selection, event, form value, or validation behavior by itself.

The host accepts `orientation="horizontal|vertical"` (horizontal by default),
presence-only `linear` and `disabled`, and a zero-based `selected-index`.
`selectedIndex` is a silent reflected property. Missing selection defaults to
zero; out-of-range values clamp to the nearest existing step. Selection may
remain on a step that later becomes disabled, but disabled paint and activation
suppression win and roving focus moves to an enabled header.

A step accepts presence-only `completed`, `editable`, `optional`, `invalid`, and
`disabled`. Completion and invalidity are application-authored facts; the
component never scans controls, forms, validity, values, or panel text. In a
linear owner, a forward destination is eligible only when every enabled earlier
step is completed or optional and none of those steps is invalid. Disabled
earlier steps are unavailable and omitted from the eligibility chain. An
invalid optional step still blocks forward navigation. A completed earlier step
can be revisited only when it is also `editable`. Nonlinear mode removes the
forward eligibility rule but retains disabled suppression and editable return.

Malformed direct vocabulary, missing/duplicate identities, missing labels, or
an empty step set produces no interactive step surface and one deterministic
console warning per distinct payload. Nested `cem-step` elements are panel
content, not additional workflow steps.

## Selection, interaction, and event contract

Each step header is one exact generated native `button[type="button"]`. The
ordered-list item and connector are structural and never become hover, focus,
activation, or event owners. Clicking an eligible non-current header updates
`selected-index`, keeps focus on that button, and emits one bubbling/composed
`cem-step` event with serializable `{ value, index, previousIndex }` detail.
Native Enter and Space use the same click path. Current-step activation,
ineligible linear/editable activation, disabled activation, pointer entry/exit,
focus movement, and programmatic attribute/property changes emit nothing.

Exactly one enabled header participates in the roving tab order. Horizontal
Left/Right and vertical Up/Down move focus, wrap, and skip disabled steps without
selecting. Home/End focus the first/last enabled header in either orientation.
The other-axis arrows remain untouched. Focus movement may land on an
ineligible header so applications can expose its label and state; Enter/Space
still obey activation eligibility. Host-disabled owners expose no tab stop.

All generated panels remain present and ARIA-linked for the lifetime of the
current payload. Only the selected panel is visible. Selection does not destroy
authored panel state or move controls between panels. User-authored Next/Back
actions remain ordinary panel content: applications update completion/invalid
facts and then set `selectedIndex`, so application validation is never inferred
or duplicated.

## Accessibility contract

The exact root is a labeled workflow region containing an `ol` of native header
buttons and one stable `role="region"` panel per step. The current button exposes
`aria-current="step"`; it does not claim tab semantics or `aria-selected`.
Every button has stable `aria-controls`, every panel has reciprocal
`aria-labelledby`, and hidden panels use the native `hidden` state.

Visible status copy announces `Complete`, `Optional`, or `Error` without relying
on color or iconography. Invalid headers also expose `aria-invalid="true"`.
Native `disabled` owns authored step and host-disabled suppression. Linear or
editable ineligibility uses focusable `aria-disabled="true"`, allowing roving
focus to expose the unavailable step's label/status while capture behavior
suppresses every activation path. The ordered list preserves step order, while
the visible number/complete/error marker is redundant and hidden from
accessibility APIs. Generic `tablist`/`tab`/`tabpanel` roles are not used: this
owner represents current workflow position, completion, eligibility, and
validation rather than interchangeable local views.

## Theme-token audit

The pre-CSS audit found workflow status/connector meaning missing while every
other category already exists:

| Visual role | Domain | Token |
| --- | --- | --- |
| Header default/hover/active/current/current-hover/current-active/disabled | D0 | existing `--cem-navigation-item-*` endpoints |
| Completed and invalid status indicators | D0 | new `--cem-workflow-step-{completed,invalid}-indicator-color` endpoints |
| Remaining and completed connectors | D0 | new `--cem-workflow-connector-{default,completed}-color` endpoints |
| Header/list/panel spacing | D1 | existing related/group/block gaps and control padding |
| Header target and process separation | D2 | existing coupling zone/guard endpoints |
| Header shape | D3 | existing control bend |
| Connector, focus, and indicator geometry | D5 | existing hair/divider/focus/indicator strokes |
| Header, status, and panel text | D6 | existing UI/tag typography |

The connector represents workflow progress, not separation between sibling
regions, so `--cem-separator-color` is not semantically valid. The new D0 family
is canonical before component CSS consumes it. Existing dimensions cover all
geometry and text; this contract adds no CSS exception. D7 is unused because no
component animation or transition is introduced.

## Forced-colors boundary

Default/header text and remaining connectors resolve to `CanvasText`/`GrayText`;
hover and held activation resolve to `Highlight`/`HighlightText`; the current
step resolves to `SelectedItem`/`SelectedItemText`; completion resolves to
`Highlight`; invalidity resolves to `Mark`; disabled steps resolve to `GrayText`;
and keyboard focus resolves to `CanvasText`. Current, completion/error, and
focus remain independently visible through fill, explicit text/marker, and
outline geometry. Forced colors retain native button behavior and remove no
semantic cue. Geometry, selection, events, and host state remain unchanged.

## Focused fixture and assertion matrix

`tests/stepper/contract.html`, its browser spec, and the dedicated forced-colors
gate form the executable contract:

| Concern | Required evidence |
| --- | --- |
| Owners | strict direct `cem-step` payloads, exact native headers, ordered list, stable linked regions, malformed rejection |
| Pointer/events | one eligible click commit/event, current/ineligible/disabled silence, trusted enter/leave on the button only |
| Keyboard | one roving tab stop, orientation axes, wrap, disabled skip, Home/End, native Enter/Space activation |
| Workflow | nonlinear selection, linear forward gate, completed/optional/invalid rules, editable return, application-owned facts |
| State/stability | current/completed/invalid/disabled/focus/hover coexistence, persistent panel content, stable transient geometry and state |
| Programmatic | silent reflected `selectedIndex`, clamping, live payload/state updates, no inferred validation |
| Forced colors | system current/hover/active/focus/status/disabled paint, progress connector distinction, no animation/z-index dependence |

## Benchmark references

- [Pinned Angular Material stepper contract](https://github.com/angular/components/blob/0b67c3c38141049657b1167479accc80e455d2bd/src/material/stepper/stepper.md)
- [WAI-ARIA `aria-current="step"` definition](https://www.w3.org/TR/wai-aria-1.2/#aria-current)
- [WAI-ARIA Authoring Practices roving tabindex guidance](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/#kbd_roving_tabindex)
