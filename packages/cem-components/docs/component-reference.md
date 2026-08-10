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
| Token-only style contract | `yarn nx run @epa-wg/cem-components:verify-style-contract` | Depends on current theme tokens and the verified public theme stylesheet export; checks exact action bindings and component selector scope, and rejects inline styles, unknown/non-CEM variables, and raw component color or spacing literals. |
| Input indicator forced colors | `yarn nx run @epa-wg/cem-components:verify-input-indicator-forced-colors` | Launches Chromium with forced colors active; proves component shadows collapse, field/binary hover uses `Highlight`, and keyboard focus traverses all seven input owners with full `CanvasText` outlines. |
| Navigation hover/focus/active forced colors | `yarn nx run @epa-wg/cem-components:verify-navigation-hover-forced-colors` | Launches Chromium with forced colors active; proves system hover/current/active/disabled colors, full keyboard traversal, focus coexistence, native-disabled skipping, restoration, and wrapper/state isolation. |
| Stylesheet publication | `yarn nx run @epa-wg/cem-components:verify-package` | Builds the canonical component stylesheet byte-for-byte into `dist`, verifies the side-effect-free `./styles.css` export, and checks the dry-run npm file inventory. |
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
| Package stylesheet source | `../src/styles.css` |
| Package publication and forced-colors scripts | `../scripts/copy-styles.mjs`, `../scripts/verify-package.mjs`, `../scripts/verify-input-indicator-forced-colors.mjs` |

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
The public component stylesheet implements `default`, enabled native-button
`hover`, and held native-button `active` for all three primitives with paired
CEM action tokens. Hover and active change only background/text color; they add
no ARIA or active runtime state and exclude disabled buttons through `:enabled`.
See the [action hover contract](./action-hover-contract.md) and
[action active contract](./action-active-contract.md).

## Inputs

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-field` | Generic labeled single-line field. | `name`, `value`, `type`, `placeholder`, `indicator`, `busy`; named label/help slots. | input indicator, stroke, zebra, bend, gap, typography | Label slot or `label` attribute must name the input. |
| `cem-text-field` | Single-line text entry. | `name`, `value`, `placeholder`, `indicator`, `busy`; `slot="label"` and `slot="help"`. | input indicator, stroke, zebra, bend, gap, typography | Label slot or `label` attribute must name the input. Help text must not become the accessible name. |
| `cem-textarea` | Multi-line text entry. | `name`, `value`, `placeholder`, `indicator`, `busy`; `slot="label"` and `slot="help"`. | input indicator, stroke, zebra, bend, gap, typography | Same label and help rules as text field. |
| `cem-select` | Form-associated custom single/multiple choice with HTML-rendered options. | Canonical direct `cem-option`/`cem-option-group`; all-native `option`/`optgroup` migration adapter; `multiple`, `size`, `indicator`, `busy`, label/help slots. See the [custom select contract](./select-contract.md). | select, input indicator, stroke, zebra, bend, layering, control, typography | Label slot or `label` attribute names the combobox/listbox; focus remains on the composite owner with `aria-activedescendant`. |
| `cem-option` | Canonical rich option payload consumed by `cem-select`. | Required `value`; optional `label`, `selected`, and `disabled`; static HTML descendants. | palette, typography | Does not create a nested tab stop or interaction owner. |
| `cem-option-group` | Canonical labeled grouping payload consumed by `cem-select`. | Required `label`; optional `disabled`; direct `cem-option` children. | palette, typography | The select projects `role="group"` and its accessible label. |
| `cem-checkbox` | Binary form choice. | Default slot is label; `name` and `value` forward to native input; `indicator`; `busy`. | input indicator, stroke, zebra, control, bend, typography | Wrapping label must expose the visible text as the accessible name. |
| `cem-radio` | Mutually exclusive form choice. | Default slot is label; shared `name` groups radios; `indicator`; `busy`. | input indicator, stroke, zebra, control, typography | Radio group context should provide the set label. |
| `cem-switch` | Immediate boolean setting. | Default slot is label; renders checkbox with `role="switch"`; `indicator`; `busy`. | input indicator, stroke, zebra, action, control, bend | Visible label must name the switch. |

States: `default`, `hover`, `focus-visible`, `disabled`, `loading`, `expanded`, `invalid`, `required`, `readonly`,
`checked`, `indeterminate`.

Presence-only host `busy` projects exact `data-state="loading"` and
`aria-busy="true"` markers to the same interactive control. It does not infer or
perform asynchronous work, disable editing, create runtime state, or replace
the control; label, value, focus, and dimensions remain stable. See the
[input loading contract](./input-loading-contract.md).

The input indicator is one three-stripe stack: anchor/state is always present,
focus is independent, and checked/indeterminate selection is independent.
Invalidity changes the anchor color rather than adding geometry, so invalid,
focus, and selection remain simultaneously legible without defining a fourth
role. Field-like controls default to `indicator="underline"`; checkbox, radio,
and switch default to `indicator="outline"`. Either family accepts the other
value, and a missing or unsupported value falls back to its family default.

Advanced custom elements can set the inherited
`--cem-input-indicator-appearance` adapter to
`var(--cem-indicator-appearance-underline)` or
`var(--cem-indicator-appearance-outline)`; this adapter wins over the host
attribute. The stripe colors and widths remain generated CEM theme tokens and
are not component-local customization endpoints. Pending also consumes the
generated `--cem-stroke-pending` endpoint so it changes thickness as well as
color. In forced colors, shadows are removed: hover uses `Highlight`, while
pending and focus use full `CanvasText` outlines at their semantic widths.
The state fixture drives real Tab navigation through every enabled input owner,
proves disabled controls are skipped, and requires focus/blur to preserve DOM,
ARIA, values, runtime snapshots, and layout geometry.

## Layout

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-surface` | Named section surface for grouped content and workflow regions. | Default slot projects authored content; `tone` selects visual treatment. Presence-only `busy` marks a pending whole-workflow update; presence-only `empty` marks its settled empty outcome. Busy takes rendered-state precedence during ordered transitions. | palette, stroke, bend, gap, inset | `label` names the section. Busy adds exact `data-state="loading"` and `aria-busy="true"`; empty adds exact `data-state="empty"`. Neither state adds live-region, inert, or focus semantics. |
| `cem-stack` | Generic single-axis layout container. | Default slot projects children; `gap` selects spacing. The component does not infer or inherit loading or empty state. | gap, responsive | Adds no landmark or interaction semantics. |
| `cem-grid` | Generic responsive grid layout container. | Default slot projects children; `columns` and `gap` select placement. The component does not infer or inherit loading or empty state. | gap, responsive | Adds no landmark or interaction semantics. |

States: `default`, `loading`, `empty`. In v1, both explicit layout states are
owned by `cem-surface`: `busy` projects pending loading and takes precedence over
the settled `empty` state. Stacks and grids remain formatting-only. See the
[layout loading](./layout-loading-contract.md) and
[layout empty](./layout-empty-contract.md) contracts.

## Navigation

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-app-bar` | Page or application banner. | `slot="title"` for title; default slot for global actions. | palette, stroke, gap, inset, typography | `label` names the banner when multiple landmarks exist. |
| `cem-nav` | Labeled navigation region with an optional region-wide disclosure. | Default slot accepts links/actions. Presence-only `collapsible` adds a native disclosure button; presence-only `expanded` sets its initial open state. Without `collapsible`, the existing passive landmark output is unchanged. | palette, navigation, gap, inset, typography | `label` names both the nav landmark and disclosure button. The button mirrors the current boolean state through `aria-expanded`; hidden content leaves the tab order and links retain native semantics. |
| `cem-tabs` | Local view switcher. | Project tab buttons with `role="tab"` and `aria-selected`. | palette, navigation, stroke, gap, typography | Tablist must be named and exactly one active tab should be selected. |

States: `default`, `hover`, `focus-visible`, `active`, `disabled`, `selected`, `expanded`.
The public stylesheet implements hover and focus-visible on the actual nav
links/buttons, collapsible disclosure button, and tab buttons—not their
structural wrappers. Generated navigation-item pairs preserve
current-link/selected-tab meaning and suppress enabled hover styling for native
or ARIA-disabled owners. Existing D5 stroke/offset and zebra color semantics
provide an external focus ring that coexists with hover and selection without
changing geometry. Navigation active/current-active pairs provide native held
feedback; disclosure release remains owned by its expanded contract. Forced
colors use platform system colors. See the
[navigation hover contract](./navigation-hover-contract.md) and
[navigation focus-visible contract](./navigation-focus-visible-contract.md),
plus the [navigation active contract](./navigation-active-contract.md).

## Content

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-card` | Summary surface and explicit content-loading boundary. | `slot="title"` for heading; default slot for body. Presence-only `busy` retains the authored payload and reflects loading state without starting resource work. | palette, stroke, bend, gap, inset | `label` names the section. Busy state adds exact `data-state="loading"` and `aria-busy="true"` without a live region, inert subtree, or focus move. |
| `cem-list` | Passive collection wrapper by default; native single-select listbox with `selectable`. | Passive mode projects `<li>` rows. Selectable mode consumes direct `cem-list-option` payload with required `value` and optional `selected`/`disabled`; parent `label`, `value`, and `size` configure the listbox. | palette, stroke, gap, typography | The list or listbox must be named. Selectable mode keeps focus and keyboard behavior on the native `<select>`, reflects exact option `aria-selected`, and does not participate in forms. |
| `cem-table` | Structured comparison or data grid surface. | Project ARIA rows/cells. | palette, stroke, gap, typography | Renders `role="table"` and needs a label. |
| `cem-chip` | Compact label or filter toggle. | Default slot is visible label; `label` can provide a fuller name. Without `checkable`, renders a passive `<span>`. With `checkable`, renders a native toggle `<button>` and uses the presence-only `checked` attribute as its initial state. | palette, action, bend, inset, typography | Checkable chips mirror their boolean slice through `aria-pressed`; removable chips need a separate named remove action. |
| `cem-badge` | Status/count/severity label. | Default slot is text; `tone` maps to status styling. | palette, bend, inset, typography | Badge text must be visible or included in adjacent accessible text. |
| `cem-avatar` | Person or organization identity. | `label` names identity; `initials` fallback or projected media. | palette, bend, typography | Renders `role="img"` and requires a label. |
| `cem-media-preview` | Asset thumbnail or object preview. | Project image/media; `slot="caption"` for caption. | palette, stroke, bend, gap | Media must carry its own accessible alternative text. |

States: `default`, `hover`, `focus-visible`, `selected`, `loading`, `empty`, `checked`.

In v1, `content:loading` is owned only by `cem-card[busy]`; lists, tables, and
media previews neither infer nor inherit it. Authors retain last-known content
during refresh or provide visible loading text plus layout-preserving
`cem-skeleton` payload for initial loading. See the
[content loading contract](./content-loading-contract.md).

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
