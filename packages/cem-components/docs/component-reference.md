# CEM Component Reference

**Status:** Phase 4 MVP reference for `@epa-wg/cem-components`.

This document describes the installable MVP declaration set registered by
`installCemComponentPrimitives()`. It pairs with [`conventions.md`](./conventions.md),
[`light-dom-rendering.md`](./light-dom-rendering.md), [`accessibility.md`](./accessibility.md), and
[`../../../docs/component-mvp.md`](../../../docs/component-mvp.md).

## Authoring Contract

- Components render in light DOM through the `<cem-element>` substrate. Authors provide semantic fallback content first;
  the upgraded output keeps the same visible meaning.
- Component state is expressed through host attributes and ARIA attributes, not private CSS classes. Use
  `disabled`, `aria-selected`, `aria-expanded`, `aria-busy`, `aria-invalid`, and `data-state` according to the state
  matrix in `docs/component-mvp.md`.
- Component styling must use CEM token families from `@epa-wg/cem-theme`. The token families listed below are the
  allowed styling surface for the MVP; no component-specific color or spacing literals should be introduced.
- Examples live under [`../examples`](../examples). Tests stay separate in `src/lib/*.spec.ts` and cover the same
  workflow-shaped cases with package-owned fixtures.

## Actions

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-action` | Native command button. | Default slot is the visible label. `variant` selects visual treatment. | action, control, palette, bend, typography | Button text or `aria-label` must name the action. |
| `cem-icon-button` | Native icon-only command button. | `name` selects icon text; `label` provides the accessible name. | action, control, palette, stroke, bend | `label` is required because icon text is hidden from assistive tech. |
| `cem-menu-item` | Menu command row. | Default slot is command text. | action, palette, gap, inset, typography | Renders `role="menuitem"` and must be contained by a menu/list context in full menus. |

States: `default`, `hover`, `focus-visible`, `active`, `disabled`, `loading`.

## Inputs

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-text-field` | Single-line text entry. | `name`, `value`, `placeholder`; `slot="label"` and `slot="help"`. | palette, stroke, bend, gap, typography | Label slot or `label` attribute must name the input. Help text must not become the accessible name. |
| `cem-textarea` | Multi-line text entry. | `name`, `value`, `placeholder`; `slot="label"` and `slot="help"`. | palette, stroke, bend, gap, typography | Same label and help rules as text field. |
| `cem-select` | Native single-value choice. | Project `<option>` children; `slot="label"` names the control. | palette, stroke, bend, control, typography | Label slot or `label` attribute must name the select. |
| `cem-checkbox` | Binary form choice. | Default slot is label; `name` and `value` forward to native input. | palette, stroke, control, bend, typography | Wrapping label must expose the visible text as the accessible name. |
| `cem-radio` | Mutually exclusive form choice. | Default slot is label; shared `name` groups radios. | palette, stroke, control, typography | Radio group context should provide the set label. |
| `cem-switch` | Immediate boolean setting. | Default slot is label; renders checkbox with `role="switch"`. | palette, stroke, action, control, bend | Visible label must name the switch. |

States: `default`, `hover`, `focus-visible`, `disabled`, `loading`, `expanded`, `invalid`, `required`, `readonly`,
`checked`, `indeterminate`.

## Navigation

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-app-bar` | Page or application banner. | `slot="title"` for title; default slot for global actions. | palette, stroke, gap, inset, typography | `label` names the banner when multiple landmarks exist. |
| `cem-nav` | Labeled navigation region. | Default slot accepts links/actions. | palette, action, gap, inset, typography | `label` or `aria-label` must name the nav landmark. |
| `cem-tabs` | Local view switcher. | Project tab buttons with `role="tab"` and `aria-selected`. | palette, action, stroke, gap, typography | Tablist must be named and exactly one active tab should be selected. |

States: `default`, `hover`, `focus-visible`, `active`, `disabled`, `selected`, `expanded`.

## Content

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-card` | Summary surface. | `slot="title"` for heading; default slot for body. | palette, stroke, bend, gap, inset | Label or visible title must name the region when used as a landmark-like summary. |
| `cem-list` | Ordered or unordered collection wrapper. | Project `<li>` rows; `label` names the list. | palette, stroke, gap, typography | List name should describe the collection. |
| `cem-table` | Structured comparison or data grid surface. | Project ARIA rows/cells. | palette, stroke, gap, typography | Renders `role="table"` and needs a label. |
| `cem-chip` | Compact filter/token label. | Default slot is visible label; `label` can provide fuller name. | palette, action, bend, inset, typography | Removable chips need a separate named remove action. |
| `cem-badge` | Status/count/severity label. | Default slot is text; `tone` maps to status styling. | palette, bend, inset, typography | Badge text must be visible or included in adjacent accessible text. |
| `cem-avatar` | Person or organization identity. | `label` names identity; `initials` fallback or projected media. | palette, bend, typography | Renders `role="img"` and requires a label. |
| `cem-media-preview` | Asset thumbnail or object preview. | Project image/media; `slot="caption"` for caption. | palette, stroke, bend, gap | Media must carry its own accessible alternative text. |

States: `default`, `hover`, `focus-visible`, `selected`, `loading`, `empty`, `checked`.

## Feedback

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-dialog` | Modal decision or task surface. | `label` names dialog; default slot is body. | palette, stroke, bend, gap, inset | Renders `role="dialog"` and `aria-modal="true"`; full focus trapping is follow-up runtime behavior. |
| `cem-sheet` | Non-modal task surface. | `label` names region; default slot is body. | palette, stroke, bend, gap, inset | Renders labeled `role="region"`. |
| `cem-toast` | Transient status message. | Default slot is message text. | palette, action, stroke, gap, typography | Renders polite `role="status"` live region. |
| `cem-progress` | Determinate or indeterminate progress. | `value`, `max`, and `label`. | palette, action, control, typography | Native progress must have an accessible name. |
| `cem-skeleton` | Loading placeholder. | `label` describes placeholder for author/debug context. | palette, control, bend | Rendered placeholder is `aria-hidden`; pair with visible status when needed. |
| `cem-alert` | Inline feedback. | `tone` controls visual severity; `role` defaults to `status`. | palette, action, stroke, gap, typography | Use `role="alert"` for urgent warnings/errors only. |

States: `default`, `focus-visible`, `loading`, `expanded`, `invalid`.
