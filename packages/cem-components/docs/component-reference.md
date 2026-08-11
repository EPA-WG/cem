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
| Angular Material parity inventory | `yarn nx run @epa-wg/cem-components:verify-material-parity` | Pins the exact stable official catalog and requires every entry to remain visible until it is audited as a component mapping, cross-cutting behavior, partial mapping, or explicit gap. Barebone `cem-elements` compatibility fixtures cannot satisfy product UI parity evidence. |
| State matrix | `yarn nx run @epa-wg/cem-components:verify-state-matrix` | Resolves every category state to exact browser tests/assertions and supports verifier-checked component-specific evidence so a newly promoted owner does not replace existing evidence. |
| Token-only style contract | `yarn nx run @epa-wg/cem-components:verify-style-contract` | Depends on current theme tokens and the verified public theme stylesheet export; checks exact action, content-interaction, navigation, paginator, feedback, slider, and tooltip bindings, component selector scope, and rejects inline styles, unknown/non-CEM variables, and raw component color or spacing literals. |
| Divider forced colors | `yarn nx run @epa-wg/cem-components:verify-divider-forced-colors` | Proves `CanvasText` separator color, D5 thickness, D1 inset, and the complete D1/D2 line-plus-margins track in forced colors. |
| Expansion forced colors | `yarn nx run @epa-wg/cem-components:verify-expansion-forced-colors` | Proves header/panel system colors, contextual hover/active/disabled paint, D5 focus, D2 target size, stable geometry, and event/state isolation. |
| Sort-header forced colors | `yarn nx run @epa-wg/cem-components:verify-sort-header-forced-colors` | Proves character-distinct none/ascending/descending states, system hover/active/disabled colors, D5 focus coexistence, D2/D2c geometry, and transient-input state isolation. |
| Paginator forced colors | `yarn nx run @epa-wg/cem-components:verify-paginator-forced-colors` | Proves native select/action ownership, system hover/active/disabled colors, D5 focus coexistence, D2/D2c geometry, surviving character icons, and transient-input state isolation. |
| Slider forced colors | `yarn nx run @epa-wg/cem-components:verify-slider-forced-colors` | Proves native range-input ownership, system remaining/active/disabled track and thumb semantics, surviving ticks, D2/D2c geometry, D5 focus, and transient-input state isolation. |
| Tooltip forced colors | `yarn nx run @epa-wg/cem-components:verify-tooltip-forced-colors` | Proves exact trigger ownership, persistent description, `Canvas`/`CanvasText` surface paint, top-layer CSS anchor placement and fallback, pointer/focus continuity, stable geometry, and event/state isolation. |
| Input indicator forced colors | `yarn nx run @epa-wg/cem-components:verify-input-indicator-forced-colors` | Launches Chromium with forced colors active; proves component shadows collapse, field/binary hover uses `Highlight`, and keyboard focus traverses the original seven input owners with full `CanvasText` outlines. |
| Autocomplete forced colors | `yarn nx run @epa-wg/cem-components:verify-autocomplete-forced-colors` | Proves popup draw order, input/option pointer ownership, system hover/active/selected/disabled colors, keyboard focus coexistence, stable geometry, and event/state isolation. |
| Navigation hover/focus/active/disabled forced colors | `yarn nx run @epa-wg/cem-components:verify-navigation-hover-forced-colors` | Launches Chromium with forced colors active; proves system hover/current/active/disabled colors, ARIA-disabled current/selected precedence, full keyboard traversal, focus coexistence, native-disabled skipping, restoration, and wrapper/state isolation. |
| Content hover/focus forced colors | `yarn nx run @epa-wg/cem-components:verify-content-hover-forced-colors` | Launches Chromium with forced colors active; proves exact content-owner keyboard order and `CanvasText` rings alongside checkable-chip system fills, native-listbox hover boundary color, selected/checked coexistence, disabled skipping, restoration, and passive wrapper isolation. |
| Feedback focus forced colors | `yarn nx run @epa-wg/cem-components:verify-feedback-focus-forced-colors` | Launches Chromium with forced colors active; proves both transient native-dialog fallback owners retain the D5 width/offset with `CanvasText` and automatic color adjustment while static wrappers, hosts, sheets, and authored descendants remain outside component focus paint. |
| Stylesheet publication | `yarn nx run @epa-wg/cem-components:verify-package` | Builds the canonical component stylesheet byte-for-byte into `dist`, verifies the side-effect-free `./styles.css` export, and checks the dry-run npm file inventory. |
| Browser and unit behavior | `yarn nx run @epa-wg/cem-components:test` | Runs the Node smoke test plus Chromium-backed harness, primitive, state/ARIA, and workflow specs. |

Executable fixture locations:

| Purpose | Path |
| --- | --- |
| Primitive declarations | `../src/lib/primitives.ts` |
| Angular Material parity inventory | `../tests/angular-material-parity.json` |
| Primitive family coverage | `../src/lib/primitives.browser.spec.ts` |
| Autocomplete behavior and state coverage | `../src/lib/autocomplete.browser.spec.ts` |
| Expansion behavior and state coverage | `../src/lib/expansion.browser.spec.ts` |
| Sort-header behavior and state coverage | `../src/lib/sort-header.browser.spec.ts` |
| Paginator behavior and state coverage | `../src/lib/paginator.browser.spec.ts` |
| Slider behavior and state coverage | `../src/lib/slider.browser.spec.ts` |
| Tooltip behavior and state coverage | `../src/lib/tooltip.browser.spec.ts` |
| State, ARIA, focus, and event payload coverage | `../src/lib/states.browser.spec.ts` |
| Feedback lifecycle and focus coverage | `../src/lib/feedback-expanded.browser.spec.ts` |
| Workflow fixture coverage | `../src/lib/workflows.browser.spec.ts` |
| Declarative workflow fixtures | `../tests/workflows/` |
| Declarative feedback fixture | `../tests/feedback/expanded.html` |
| Declarative autocomplete fixture | `../tests/autocomplete/contract.html` |
| Declarative expansion fixture | `../tests/expansion/contract.html` |
| Declarative sort-header fixture | `../tests/sort-header/contract.html` |
| Declarative paginator fixture | `../tests/paginator/contract.html` |
| Declarative slider fixture | `../tests/slider/contract.html` |
| Declarative tooltip fixture | `../tests/tooltip/contract.html` |
| Component harness helpers | `../src/lib/testing/component-harness.ts` |
| Style and manifest verifier scripts | `../../../tools/scripts/verify-cem-components-*.mjs` |
| Package stylesheet source | `../src/styles.css` |
| Package publication and forced-colors scripts | `../scripts/copy-styles.mjs`, `../scripts/verify-package.mjs`, `../scripts/verify-input-indicator-forced-colors.mjs`, `../scripts/verify-autocomplete-forced-colors.mjs`, `../scripts/verify-navigation-hover-forced-colors.mjs`, `../scripts/verify-content-hover-forced-colors.mjs`, `../scripts/verify-expansion-forced-colors.mjs`, `../scripts/verify-sort-header-forced-colors.mjs`, `../scripts/verify-paginator-forced-colors.mjs`, `../scripts/verify-slider-forced-colors.mjs`, `../scripts/verify-tooltip-forced-colors.mjs`, `../scripts/verify-feedback-focus-forced-colors.mjs` |

Handoff condition: Phase 4 component expansion can build on this primitive package after the aggregate verify gate is
green and the promoted branch has no uncommitted gate changes. The handoff covers the MVP primitive declaration set,
common static/form workflows, state and ARIA behavior, light-DOM rendering, event payload capture, and token-only style
constraints.

Known deferrals remain outside the Phase 3.2 trigger:

- Edge/SSR processing fixtures for serialized data-island snapshots are Phase 3.5.
- `@epa-wg/custom-element` monorepo adoption is Phase 3.6.
- Full application behaviors such as dialog focus trapping, routed navigation, async data loading, and resource
  primitives are follow-up runtime/application work.
- Post-MVP controls including split actions, date/time controls, side-nav variants, breadcrumbs, and richer
  menu/dropdown families are Phase 4 expansion work.

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
| `cem-autocomplete` | Form-associated editable combobox with declarative suggestions. | Canonical direct `cem-option`/`cem-option-group`; all-native `option`/`optgroup` migration adapter; free-form or `require-selection`; `value`, `placeholder`, `autocomplete`, `indicator`, `busy`, `auto-active-first`, label/help slots. See the [autocomplete contract](./autocomplete-contract.md). | select, input indicator, stroke, zebra, bend, layering, control, typography | The native input owns focus and text entry; the transient listbox exposes options/groups while `aria-activedescendant` retains focus on the input. |
| `cem-select` | Form-associated custom single/multiple choice with HTML-rendered options. | Canonical direct `cem-option`/`cem-option-group`; all-native `option`/`optgroup` migration adapter; `multiple`, `size`, `indicator`, `busy`, label/help slots. See the [custom select contract](./select-contract.md). | select, input indicator, stroke, zebra, bend, layering, control, typography | Label slot or `label` attribute names the combobox/listbox; focus remains on the composite owner with `aria-activedescendant`. |
| `cem-option` | Canonical rich option payload consumed by `cem-select` and `cem-autocomplete`. | Required `value`; optional `label`, `selected`, and `disabled`; static HTML descendants. | palette, typography | Does not create a nested tab stop or interaction owner. |
| `cem-option-group` | Canonical labeled grouping payload consumed by `cem-select` and `cem-autocomplete`. | Required `label`; optional `disabled`; direct `cem-option` children. | palette, typography | The consuming composite projects `role="group"` and its accessible label. |
| `cem-checkbox` | Binary form choice. | Default slot is label; `name` and `value` forward to native input; `indicator`; `busy`. | input indicator, stroke, zebra, control, bend, typography | Wrapping label must expose the visible text as the accessible name. |
| `cem-radio` | Mutually exclusive form choice. | Default slot is label; shared `name` groups radios; `indicator`; `busy`. | input indicator, stroke, zebra, control, typography | Radio group context should provide the set label. |
| `cem-switch` | Immediate boolean setting. | Default slot is label; renders checkbox with `role="switch"`; `indicator`; `busy`. | input indicator, stroke, zebra, action, control, bend | Visible label must name the switch. |
| `cem-slider` | Horizontal single-value or range input. | One direct native range input marked `single`, or exact `start`/`end` inputs; parent `min`, `max`, `step`, `disabled`, `discrete`, `show-tick-marks`. See the [slider contract](./slider-contract.md). | slider, coupling, stroke, bend, gap, typography | Every input retains native slider semantics and requires an accessible name; range thumbs require distinct names. Generated visuals are hidden. |

States: `default`, `hover`, `focus-visible`, `disabled`, `loading`, `expanded`, `invalid`, `required`, `readonly`,
`checked`, `indeterminate`.

`cem-autocomplete` is covered across every applicable input state. It supports
free text by default and an explicit `require-selection` mode, contributes its
string value through `ElementInternals`, keeps focus on the native input during
listbox navigation, and accepts live declarative option replacement without
replacing that input or mutating the committed value. Filtering, ranking,
fetching, and debouncing remain application/CEM-QL concerns.

`cem-slider` retains native range-input form/event ownership for single and
range values. It adds no tuple form value or custom event: each thumb serializes
under its authored name and emits its own native `input`/`change` sequence.
Parent bounds are normalized, range values cannot cross, and transient pointer
or focus paint does not mutate component state or geometry.

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
| `cem-divider` | Visible sibling-separation track. | `orientation="horizontal|vertical"`; `spacing="related|group|block|section"`; presence-only `inset` and `decorative`. It owns no content, state, focus, or events. | separator, stroke, gap, inset, coupling | Semantic form exposes a non-focusable separator with exact orientation. Decorative form removes the role/orientation and sets `aria-hidden="true"`. |

States: `default`, `loading`, `empty`. In v1, both explicit layout states are
owned by `cem-surface`: `busy` projects pending loading and takes precedence over
the settled `empty` state. Stacks and grids remain formatting-only; dividers are
non-interactive separation tracks. `cem-divider` composes D0 color, D1 relationship
spacing/inset, the D2 guard floor, and D5 line geometry as specified by the
[divider contract](./divider-contract.md). See the
[layout loading](./layout-loading-contract.md) and
[layout empty](./layout-empty-contract.md) contracts.

## Navigation

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-app-bar` | Page or application banner. | `slot="title"` for title; default slot for global actions. | palette, stroke, gap, inset, typography | `label` names the banner when multiple landmarks exist. |
| `cem-nav` | Labeled navigation region with an optional region-wide disclosure. | Default slot accepts links/actions. Presence-only `collapsible` adds a native disclosure button; presence-only `expanded` sets its initial open state. Without `collapsible`, the existing passive landmark output is unchanged. | palette, navigation, gap, inset, typography | `label` names both the nav landmark and disclosure button. The button mirrors the current boolean state through `aria-expanded`; hidden content leaves the tab order and links retain native semantics. |
| `cem-tabs` | Local view switcher. | Project tab buttons with `role="tab"` and `aria-selected`. | palette, navigation, stroke, gap, typography | Tablist must be named and exactly one active tab should be selected. |
| `cem-paginator` | Labeled paged-content navigation with application-owned data. | `length`, zero-based `page-index`, `page-size`, whitespace-separated `page-size-options`, `show-first-last`, `hide-page-size`, `disabled`, `name`, `label`, and localizable control/range labels. See the [paginator contract](./paginator-contract.md). | action, palette, select, control, stroke, bend, gap, inset, typography | A native navigation landmark owns a labeled page-size select, named native actions, focus-stable `aria-disabled` boundaries, and one atomic polite range status. Applications consume `cem-page` to load/render data. |

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
the [navigation active contract](./navigation-active-contract.md), and the
[navigation disabled contract](./navigation-disabled-contract.md). Native
disabled buttons leave the tab order and use browser suppression. Direct
ARIA-disabled owners remain keyboard-discoverable while host capture behavior
prevents pointer, programmatic, Enter, and native-button Space activation
before target/application bubble listeners or default action.

`cem-paginator` uses its native select and action buttons as the only hover,
focus, active, and disabled owners; the landmark, range/actions group, and label
remain structural. Render-only normalization does not rewrite invalid or
out-of-range author attributes. Page-size changes preserve the first visible
item, successful user requests emit one serializable `cem-page`, and boundary
or globally disabled activation is suppressed. Existing theme semantics cover
normal and forced colors without a CSS exception.

## Content

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-card` | Summary surface and explicit content-loading boundary. | `slot="title"` for heading; default slot for body. Presence-only `busy` retains the authored payload and reflects loading state without starting resource work. | palette, stroke, bend, gap, inset | `label` names the section. Busy state adds exact `data-state="loading"` and `aria-busy="true"` without a live region, inert subtree, or focus move. |
| `cem-expansion` | One independent general-purpose disclosure panel. | `slot="summary"` or `label` names the native header; default slot supplies persistent panel content. Presence-only `expanded`, `disabled`, and `region`; `heading-level="1..6"` defaults to 3. See the [expansion contract](./expansion-contract.md). | action, palette, stroke, bend, gap, inset, coupling, control, typography | The header is the sole button owner inside a validated heading. Exact `aria-labelledby`, `aria-expanded`, `aria-controls`, and reciprocal panel references remain stable; collapsed content is hidden and disabled blocks only user toggling. |
| `cem-list` | Passive collection wrapper by default; native single-select listbox with `selectable`. | Passive mode projects `<li>` rows. Selectable mode consumes direct `cem-list-option` payload with required `value` and optional `selected`/`disabled`; parent `label`, `value`, and `size` configure the listbox. | palette, content, stroke, gap, typography | The list or listbox must be named. Selectable mode keeps focus, hover, and keyboard behavior on the native `<select>`, reflects exact option `aria-selected`, and does not participate in forms. |
| `cem-table` | Structured comparison or data grid surface. | Project ARIA rows/cells. | palette, stroke, gap, typography | Renders `role="table"` and needs a label. |
| `cem-sort-header` | Sortable-column action composed inside a table row. | `label` supplies visible text and the `Sort by …` action name; `name` identifies `cem-sort` detail; `direction="ascending|descending"` or absence supplies state; `disabled` suppresses native activation. See the [sort-header contract](./sort-header-contract.md). | action, control, stroke, bend, gap, coupling, typography | A generated `role="columnheader"` conditionally owns `aria-sort`; its direct native button owns focus and Space/Enter. The application consumes `cem-sort` to reorder rows and announce the localized result. |
| `cem-chip` | Compact label or filter toggle. | Default slot is visible label; `label` can provide a fuller name. Without `checkable`, renders a passive `<span>`. With `checkable`, renders a native toggle `<button>` and uses the presence-only `checked` attribute as its initial state. | palette, content, action, stroke, bend, inset, typography | Checkable chips mirror their boolean slice through `aria-pressed`; hover and keyboard focus retain checked meaning, while removable chips need a separate named remove action. |
| `cem-badge` | Status/count/severity label. | Default slot is text; `tone` maps to status styling. | palette, bend, inset, typography | Badge text must be visible or included in adjacent accessible text. |
| `cem-avatar` | Person or organization identity. | `label` names identity; `initials` fallback or projected media. | palette, bend, typography | Renders `role="img"` and requires a label. |
| `cem-media-preview` | Asset thumbnail or object preview. | Project image/media; `slot="caption"` for caption. | palette, stroke, bend, gap | Media must carry its own accessible alternative text. |

States: `default`, `hover`, `focus-visible`, `selected`, `loading`, `empty`, `checked`.

`cem-expansion` additionally owns contextual `active`, `disabled`, and `expanded`
states for its native header without turning content disclosure into navigation or
selection. Multiple siblings are independent; exclusive accordion-group policy
is not part of this contract.

`cem-sort-header` additionally owns contextual `active`, `disabled`,
`ascending`, `descending`, and `none` states. Only its native button receives
interaction paint; `cem-table`, the host, and the column-header wrapper remain
structural. Existing theme semantics cover every binding, including forced
colors, so no CSS exception is recorded.

In v1, `content:loading` is owned only by `cem-card[busy]`; lists, tables, and
media previews neither infer nor inherit it. Authors retain last-known content
during refresh or provide visible loading text plus layout-preserving
`cem-skeleton` payload for initial loading. See the
[content loading contract](./content-loading-contract.md).

## Feedback

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-tooltip` | Supplemental plain-text description with a transient top-layer presentation. | Exactly one supported native trigger assigned to `slot="trigger"`; required `message`; logical `position`; optional `show-delay`, `hide-delay`, `open`, and `disabled`. See the [tooltip contract](./tooltip-contract.md). | palette, stroke, bend, gap, inset, layering, typography | Appends a stable hidden description to the trigger's existing `aria-describedby`; the separate non-focusable visual has `role="tooltip"`. Focus remains on the native trigger. |
| `cem-dialog` | Static dialog surface by default; native modal decision or task surface with `transient`. | `label` names the owner; default slot is body. Presence-only `transient` opts into lifecycle behavior and `expanded` supplies open state in that mode. | palette, stroke, bend, gap, inset | Static mode retains `div[role="dialog"][aria-modal="true"]`. Transient mode renders a native `<dialog>` and delegates modality, focus entry/containment, Escape, and restoration to the browser. The application-owned opener carries `aria-expanded` and `aria-controls`. |
| `cem-dialog-shell` | Compatibility dialog-shell alias sharing the `cem-dialog` lifecycle boundary. | `label`, default body slot, and presence-only `transient` / `expanded` follow `cem-dialog`. | palette, stroke, bend, gap, inset | Uses the same static ARIA-wrapper versus transient native-dialog split as `cem-dialog`; it does not add another focus model. |
| `cem-sheet` | Static non-modal task surface by default; application-controlled visible/hidden region with `transient`. | `label` names the region; default slot is body. In transient mode, presence-only `expanded` removes `hidden`. | palette, stroke, bend, gap, inset | Always remains a labeled `<aside role="region">`. It does not trap or move focus, intercept Escape, make the document inert, or dispatch dialog dismissal. |
| `cem-toast` | Transient status message. | Default slot is message text. | palette, action, stroke, gap, typography | Renders polite `role="status"` live region. |
| `cem-progress` | Determinate or indeterminate progress. | `value`, `max`, and `label`. | palette, action, control, typography | Native progress must have an accessible name. |
| `cem-progress-spinner` | Non-interactive circular determinate or indeterminate progress. | `label` names the progressbar; presence of `value` selects determinate mode, absence selects indeterminate mode; `max` defaults to 100; `describedby` may reference task context. | progress, timing | Exposes normalized range values only when determinate, hides its SVG from assistive technology, has no tab stop or live region, and stops automatic rotation under reduced motion. |
| `cem-skeleton` | Loading placeholder. | `label` describes placeholder for author/debug context. | palette, control, bend | Rendered placeholder is `aria-hidden`; pair with visible status when needed. |
| `cem-alert` | Inline feedback. | `tone` controls visual severity; `role` defaults to `status`. | palette, action, stroke, gap, typography | Use `role="alert"` for urgent warnings/errors only. |

States: `default`, `focus-visible`, `loading`, `expanded`, `invalid`, `indeterminate`.

`cem-tooltip` keeps the authored native trigger as the exact hover, focus,
keyboard, and activation owner. Hover and focus are independent visibility
reasons; pointer travel onto the tooltip retains presentation; Escape dismisses
without moving focus; touch remains native and does not auto-present. A stable
hidden description is always available unless the host or native trigger is
disabled. Existing inverse palette, D1 spacing, D3 shape, D4 elevation, D5
boundary, and D6 typography semantics cover the Popover/CSS Anchor Positioning
surface in normal and forced colors without a CSS exception.

`cem-progress-spinner` is the distinct circular owner; linear `cem-progress`
does not change shape to satisfy it. Missing `value` means indeterminate, and
live `value`/`max` changes retain the same SVG and geometry. D0 progress colors,
D2c progress geometry, and D7 cycle/easing tokens cover its stylesheet. Forced
colors use `GrayText` for the remaining track and `Highlight` for the indicator;
reduced motion leaves a static incomplete arc. See the
[progress spinner contract](./progress-spinner-contract.md).

The transient feedback lifecycle is defined by the
[feedback expanded contract](./feedback-expanded-contract.md). The separate
[feedback focus-visible contract](./feedback-focus-visible-contract.md) accepts
only a transient native dialog when it is itself the browser's focused fallback.
That owner receives the external D5 width/offset and zebra-color outline; forced
colors retain its dimensions with `CanvasText` and automatic color adjustment.
Eligible authored descendants retain their own focus styling. Static dialog
wrappers, feedback hosts, and sheets receive no `tabindex`, `:focus-within`, or
component focus paint, and focus never changes their geometry, DOM, ARIA, or
lifecycle state.
