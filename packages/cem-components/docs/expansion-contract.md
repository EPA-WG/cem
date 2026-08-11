# Expansion Contract

**Status:** Accepted
**Accepted:** August 10, 2026

`cem-expansion` is the public owner for one general-purpose disclosure panel. It is content disclosure, not
navigation, selection, a menu, or a workflow step, so `cem-nav` and `cem-tabs` remain outside this contract. Multiple
`cem-expansion` siblings operate independently. Exclusive single-open coordination and a public accordion-group owner
are deliberately outside this parity slice.

## Owner and author vocabulary

The public owner is `cem-expansion`.

- `label` supplies the fallback header name.
- `slot="summary"` supplies visible header content and its accessible name. It may contain phrasing content but no
  nested interactive control because the native header button is the sole activation owner.
- The default slot supplies panel content. Content remains instantiated while collapsed so disclosure does not become
  a resource-loading or lifecycle boundary.
- Presence-only `expanded` is the live public state. Pointer or keyboard activation toggles the host attribute;
  application code may add or remove the same attribute programmatically.
- Presence-only `disabled` disables only user activation. It does not collapse an expanded panel, remove its content,
  or prevent application code from changing `expanded`.
- `heading-level="1|2|3|4|5|6"` sets the programmatic heading level; missing or unsupported values resolve to `3`.
- Presence-only `region` opts the panel into `role="region"`. Without it, `aria-controls` still provides the exact
  disclosure relationship without creating unnecessary landmarks.

The component owns no value, form association, selection, required/invalid/loading state, arrow-key navigation, or
exclusive group policy.

## State and geometry contract

The rendered `.cem-expansion__header` native button is the only hover, focus, active, disabled, and activation owner.
The `.cem-expansion__heading` wrapper supplies heading semantics only and receives no interaction paint. The rendered
`.cem-expansion__panel` is the revealed content surface; `hidden` removes it from layout and the tab sequence while
collapsed.

The header keeps one stable D2 target box across default, hover, focus-visible, active, disabled, and expanded states.
Its disclosure indicator occupies a fixed D2c icon box, so changing the glyph cannot move header text. D5 focus is an
external outline and does not alter border or layout geometry. Expanded state changes only the expected disclosure
attributes, indicator text, and panel visibility; it does not replace the header, panel, or authored payload nodes.

## Event and keyboard contract

The native `button[type="button"]` owns activation. `Enter` and `Space` use browser button behavior; `Tab` and
`Shift+Tab` follow the document tab sequence. Pointer click uses the same path. The component adds no arrow, Home, End,
or roving-focus behavior.

An uncanceled header `click` toggles the live `expanded` host attribute. The render mirrors that state to
`aria-expanded` and `hidden`. The component does not synthesize `click`, `input`, `change`, or a custom toggle event.
Programmatic attribute changes likewise emit no event. A disabled native header is not focusable and suppresses
pointer and keyboard activation while preserving its current expanded/collapsed state.

## Accessibility contract

The header is the only child of an element with `role="heading"` and a validated `aria-level`. It is a native button
with an exact `aria-labelledby` reference to the persistent summary ID, `aria-expanded="true|false"`, and
`aria-controls` referencing the persistent panel ID.
The panel has `aria-labelledby` referencing the persistent header ID. Presence-only `region` adds `role="region"` only
when the author has determined that the extra landmark helps rather than proliferates regions.

Collapsed content is `hidden`, so nested controls leave the tab sequence. Expanded content returns to its authored
order. The summary slot must not contain buttons, links, or other interactive descendants.

## Theme-token audit

| Concern | Dimension/category | Endpoint |
| --- | --- | --- |
| Header default/hover/active/disabled paint | D0 contextual action | `--cem-action-contextual-{default,hover,active,disabled}-{background,text}` |
| Panel surface and readable content | D0 palette | `--cem-palette-comfort`, `--cem-palette-comfort-text` |
| Header/panel relationship and panel inset | D1 space | `--cem-gap-related`, `--cem-inset-container` |
| Operable header floor | D2 coupling | `--cem-coupling-zone-min` |
| Header and indicator geometry | D2c controls | `--cem-control-padding-{x,y}`, `--cem-icon-button-icon-size` |
| Surface/control bend | D3 shape | `--cem-bend-control`, `--cem-bend-surface` |
| Keyboard focus | D5 stroke | `--cem-stroke-focus`, `--cem-stroke-indicator-offset`, `--cem-zebra-color-1` |
| UI label typography | D6 typography | `--cem-typography-ui-*` |

Every visual concern is already represented by an accepted Consumer Semantic Theme category. No new theme token and
no entry in `components-css-exceptions.md` are required.

## Forced-colors boundary

In `forced-colors: active`, the default header and panel use `Canvas`/`CanvasText`; hover and active use
`Highlight`/`HighlightText`; disabled uses `Canvas`/`GrayText`; and focus-visible uses the D5 width and offset with a
`CanvasText` outline. Expanded state remains visible through the disclosure glyph and programmatic state instead of a
color-only treatment. Forced colors do not alter target size, padding, inset, focus geometry, DOM identity, or event
behavior.

## Focused fixture and assertion matrix

| Surface | Assertions |
| --- | --- |
| Collapsed panel | Exact heading/button semantics, accessible name, persistent ID references, false expansion state, hidden panel |
| Expanded panel | Live host attribute, true expansion state, visible content, unchanged owner/payload identity |
| Pointer and keyboard | Trusted click, Enter, and Space share one toggle path; Tab follows native order; no synthetic state events |
| Disabled | Native disabled owner has no tab stop and suppresses pointer/keyboard toggling without collapsing programmatic expanded state |
| Programmatic control | Adding/removing `expanded` updates ARIA and visibility without emitting application events |
| Transient visual states | Hover, focus-visible, active, and disabled resolve accepted tokens without changing header geometry or expansion state |
| Region/heading options | Validated heading level, optional region landmark, exact reciprocal ARIA references |
| Forced colors | System-color mappings, D2 target floor, D2c padding/icon box, D5 focus, stable geometry, and expanded coexistence |

The declarative fixture, focused browser test, and dedicated forced-colors target must pass in the aggregate package
gate before the Angular Material expansion row can be promoted from `gap` to `covered`.
