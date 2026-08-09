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

## Production Gate

The primitive set is production-ready for Phase 4 expansion when this command passes on the branch being promoted:

```bash
yarn nx run @epa-wg/cem-components:verify
```

The aggregate gate includes:

| Gate | Command | Coverage |
| --- | --- | --- |
| Primitive manifest | `yarn nx run @epa-wg/cem-components:verify-primitives` | `CEM_COMPONENT_PRIMITIVES` exactly matches `docs/component-mvp.md`, uses CEM-ML declarations, and does not depend on legacy `<custom-element>` wrappers. |
| Token-only style contract | `yarn nx run @epa-wg/cem-components:verify-style-contract` | Depends on `@epa-wg/cem-theme:build:tokens`, checks MVP token families against generated theme tokens/CSS, and rejects inline component styles plus raw component color or spacing literals. |
| Browser and unit behavior | `yarn nx run @epa-wg/cem-components:test` | Runs the Node smoke test plus Chromium-backed harness, primitive, state/ARIA, and workflow specs. |

Executable fixture locations:

| Purpose | Path |
| --- | --- |
| Primitive declarations | `../src/lib/primitives.ts` |
| Primitive family coverage | `../src/lib/primitives.browser.spec.ts` |
| State, ARIA, focus, and event payload coverage | `../src/lib/states.browser.spec.ts` |
| Workflow fixture coverage | `../src/lib/workflows.browser.spec.ts` |
| Declarative workflow fixtures | `../tests/workflows/` |
| Component harness helpers | `../src/lib/testing/component-harness.ts` |
| Style and manifest verifier scripts | `../../../tools/scripts/verify-cem-components-*.mjs` |

Handoff condition: Phase 4 component expansion can build on this primitive package after the aggregate verify gate is
green and the promoted branch has no uncommitted gate changes. The handoff covers the MVP primitive declaration set,
common static/form workflows, state and ARIA behavior, light-DOM rendering, event payload capture, and token-only style
constraints.

Known deferrals remain outside the Phase 3.2 trigger:

- Edge/SSR processing fixtures for serialized data-island snapshots are Phase 3.5.
- `@epa-wg/custom-element` monorepo adoption is Phase 3.6.
- Full application behaviors such as dialog focus trapping, routed navigation, async data loading, and resource
  primitives are follow-up runtime/application work.
- Post-MVP controls including split actions, sliders, date/time controls, side-nav variants, breadcrumbs, pagination,
  and richer menu/dropdown families are Phase 4 expansion work.

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
| `cem-nav` | Labeled navigation region with an optional region-wide disclosure. | Default slot accepts links/actions. Presence-only `collapsible` adds a native disclosure button; presence-only `expanded` sets its initial open state. Without `collapsible`, the existing passive landmark output is unchanged. | palette, action, gap, inset, typography | `label` names both the nav landmark and disclosure button. The button mirrors the current boolean state through `aria-expanded`; hidden content leaves the tab order and links retain native semantics. |
| `cem-tabs` | Local view switcher. | Project tab buttons with `role="tab"` and `aria-selected`. | palette, action, stroke, gap, typography | Tablist must be named and exactly one active tab should be selected. |

States: `default`, `hover`, `focus-visible`, `active`, `disabled`, `selected`, `expanded`.

## Content

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-card` | Summary surface. | `slot="title"` for heading; default slot for body. | palette, stroke, bend, gap, inset | Label or visible title must name the region when used as a landmark-like summary. |
| `cem-list` | Passive collection wrapper by default; native single-select listbox with `selectable`. | Passive mode projects `<li>` rows. Selectable mode consumes direct `cem-list-option` payload with required `value` and optional `selected`/`disabled`; parent `label`, `value`, and `size` configure the listbox. | palette, stroke, gap, typography | The list or listbox must be named. Selectable mode keeps focus and keyboard behavior on the native `<select>`, reflects exact option `aria-selected`, and does not participate in forms. |
| `cem-table` | Structured comparison or data grid surface. | Project ARIA rows/cells. | palette, stroke, gap, typography | Renders `role="table"` and needs a label. |
| `cem-chip` | Compact label or filter toggle. | Default slot is visible label; `label` can provide a fuller name. Without `checkable`, renders a passive `<span>`. With `checkable`, renders a native toggle `<button>` and uses the presence-only `checked` attribute as its initial state. | palette, action, bend, inset, typography | Checkable chips mirror their boolean slice through `aria-pressed`; removable chips need a separate named remove action. |
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
