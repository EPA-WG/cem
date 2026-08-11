# Divider Contract

**Status:** Accepted
**Accepted:** August 10, 2026

`cem-divider` is the public, non-interactive owner for visible separation between sibling regions. It is a Consumer
Semantic Theme divider track—a D5 line plus D1 relationship margins, with D2 supplying the minimum safe track extent.
It is not a generic structural wrapper, a focus/selection indicator, or a replacement for authored document hierarchy.

## Owner and author vocabulary

The public owner is `cem-divider`.

- `orientation="horizontal|vertical"` selects the logical line axis. Missing or unsupported values resolve to
  `horizontal`.
- `spacing="related|group|block|section"` selects the D1 relationship space represented by the complete track.
  Missing or unsupported values resolve to `group`.
- Presence-only `inset` moves the line's logical start edge by `--cem-inset-container`; it does not change the
  cross-axis track extent.
- Presence-only `decorative` removes separator semantics and exposes the rendered line with `aria-hidden="true"`.

The component has no content slot, value, disabled state, selection, lifecycle, or behavior object.

## Geometry contract

The rendered `.cem-divider` is the line owner. Its host supplies the cross-axis margins. For the selected D1 endpoint:

```text
track extent = max(D1 relationship gap, D2 coupling guard minimum)
each margin = (track extent - D5 divider stroke) / 2
```

The line consumes `--cem-stroke-divider`; the complete margin box therefore equals the selected relationship space
unless D2 raises it to `--cem-coupling-guard-min`. A 1px hairline without those margins is not a conforming CEM
divider. Horizontal and vertical implementations transpose logical block/inline axes without changing the formula.

The standard inset is along the line axis and consumes `--cem-inset-container`. It never introduces a
divider-specific inset token. Pointer movement, click dispatch, orientation, inset, and decorative semantics must not
mutate DOM, ARIA, geometry, component state, or application events.

## Event and keyboard contract

`cem-divider` is not focusable and owns no keyboard or pointer interaction. It synthesizes no `click`, `input`,
`change`, or custom state event; ordinary trusted pointer events aimed at its painted line remain browser input and
must not mutate the component. Authored interaction on surrounding controls remains application-owned.

## Accessibility contract

The semantic form renders `role="separator"` with exact `aria-orientation="horizontal|vertical"`. It has no accessible
name and no `tabindex`; non-focusable ARIA separator semantics describe structure without creating a widget.

The decorative form renders no separator role or orientation and sets `aria-hidden="true"`. Authors use it when the
same division is already expressed by native document structure or when announcement would be redundant.

## Theme-token audit

| Concern | Dimension | Endpoint |
| --- | --- | --- |
| Reduced-salience sibling line | D0 color | `--cem-separator-color` |
| Relationship extent | D1 space | `--cem-gap-related`, `--cem-gap-group`, `--cem-gap-block`, `--cem-gap-section` |
| Leading inset | D1 space | `--cem-inset-container` |
| Minimum interactive clearance | D2 coupling | `--cem-coupling-guard-min` |
| Line thickness | D5 stroke | `--cem-stroke-divider` |

Every concern is represented by an accepted theme category, so this component requires no entry in
`components-css-exceptions.md` and no public component-local CSS token.

## Forced-colors boundary

In `forced-colors: active`, the line resolves through D0 `--cem-separator-color` to `CanvasText`. D5 thickness, the D1
inset, and the D1/D2 track calculation remain unchanged. The component does not use backgrounds, shadows, gradients,
or `forced-color-adjust: none` to simulate the line.

## Focused fixture and assertion matrix

| Surface | Assertions |
| --- | --- |
| Semantic horizontal | Exact separator role/orientation, D0 color, D5 thickness, default D1 group track, D2 floor |
| Semantic vertical | Exact vertical orientation, transposed line/margins, nonzero stretched line extent |
| Inset | Logical-start offset resolves from `--cem-inset-container` without changing cross-axis track geometry |
| Relationship spacing | Related/group/block/section variants resolve to their D1 endpoints with the D2 minimum |
| Decorative | `aria-hidden="true"`, no role/orientation/name/tab stop |
| Input neutrality | Pointer enter/leave and click cause no DOM, ARIA, state, geometry, or application-event mutation |
| Forced colors | `CanvasText` line with the same D5 width, D1 inset, and D1/D2 margin-box extent |

The browser fixture and dedicated forced-colors gate are aggregated by the package verification target before the
Angular Material divider row can be promoted from `gap` to `covered`.
