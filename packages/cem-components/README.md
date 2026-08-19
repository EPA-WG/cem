# `@epa-wg/cem-components`

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

Declarative component primitives that consume the CEM theme. No shadow DOM — every component renders in the
light DOM, authored against the `<cem-element>` substrate from `@epa-wg/cem-elements` (functional successor to
`@epa-wg/custom-element`; design home: [`docs/cem-element-design.md`](../../docs/cem-element-design.md)).

> **Status: Phase 3.2 production gate.** The package exports the browser test harness and installable primitive
> declaration set from the [component MVP](../../docs/component-mvp.md), registered through the production-ready
> `<cem-element>` substrate.

## Install

```bash
yarn add @epa-wg/cem-components
```

This package depends on `@epa-wg/cem-theme` and `@epa-wg/cem-elements`; install them alongside.

## Runtime install

```ts
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { installCemComponentPrimitives } from '@epa-wg/cem-components';

const runtime = new CemElementRuntime();
const result = await installCemComponentPrimitives(runtime);
```

The promise resolves after every accepted declaration settles. Inspect
`result.registered`, `result.skipped`, and `result.diagnostics` before rendering
application content. Behavior-backed primitives supply stable versioned host
identities as part of their registration contract.

This registers the minimal primitive tags: `cem-action`, `cem-icon-button`, `cem-menu-item`, `cem-field`,
`cem-text-field`, `cem-textarea`, `cem-autocomplete`, `cem-timepicker`, `cem-datepicker`, `cem-select`, `cem-option`, `cem-option-group`, `cem-checkbox`,
`cem-radio`, `cem-switch`, `cem-slider`, `cem-surface`, `cem-text`,
`cem-icon`, `cem-stack`, `cem-grid`, `cem-divider`, `cem-list`, `cem-card`, `cem-expansion`, `cem-table`, `cem-sort-header`, `cem-chip`, `cem-badge`, `cem-avatar`,
`cem-media-preview`, `cem-tree`, `cem-tree-item`, `cem-app-bar`, `cem-nav`, `cem-tabs`, `cem-stepper`, `cem-step`, `cem-paginator`, `cem-tooltip`, `cem-dialog`, `cem-dialog-shell`, `cem-sheet`,
`cem-toast`, `cem-progress`, `cem-progress-spinner`, `cem-skeleton`, and `cem-alert`.

## Stylesheet install

Load generated theme CSS first, then explicitly load the component bindings:

```ts
import '@epa-wg/cem-theme/styles.css';
import '@epa-wg/cem-components/styles.css';
```

The JavaScript entry does not import or inject CSS. The public stylesheet is
built byte-for-byte from `src/styles.css` and published only as
`dist/styles.css` through the `./styles.css` package export. It binds enabled
`cem-action` buttons to primary default/hover/active tokens and enabled
`cem-icon-button`/`cem-menu-item` buttons to contextual default/hover/active
tokens.

Input primitives use the shared CEM indicator stack. Field-like controls
default to an underline and binary controls to a whole-label outline; authors
can select `indicator="underline|outline"`. Advanced custom elements may set
`--cem-input-indicator-appearance` to one of the generated
`--cem-indicator-appearance-*` tokens. All stripe color and geometry values
remain theme-owned.

`cem-autocomplete` adds a form-associated editable combobox while keeping its
native text input as the only focus and text-entry owner. Its transient listbox
uses `aria-activedescendant`, canonical `cem-option`/`cem-option-group` payloads,
tokenized hover/selected/active/disabled paint, and a focused forced-colors
gate. Application rendering owns filtering and option replacement.

`cem-timepicker` adds a canonical `HH:mm` time-of-day picker while one direct
authored native text input retains accessible-name, value, validation, event,
and form ownership. It generates interval choices or consumes direct
`cem-option` labels/values, supports an optional native toggle, and keeps focus
on the input during listbox navigation. The Popover top layer and CSS Anchor
Positioning provide overlay placement without numeric z-index; existing input,
contextual-action, and select-option tokens cover normal and forced colors.

`cem-datepicker` adds a bounded single-date `YYYY-MM-DD` capability while one
direct authored native text input retains accessible-name, value, validation,
event, reset, and form ownership. Its optional native toggle opens a modal
native dialog with a localized six-week calendar grid, roving day focus, and an
explicit draft/Apply boundary. The dialog uses the top layer and CSS Anchor
Positioning; theme-owned input, contextual-action, content-interaction, and
current-indicator semantics keep hover, selected, today/current, disabled, and
focus-visible states independent in normal and forced colors.

Navigation links, disclosure buttons, and tabs consume the generated
`--cem-navigation-item-*` state family. Hover styling is applied to those native
owners rather than their nav/content/tablist wrappers, preserves current and
selected semantics, and suppresses enabled treatment for disabled owners.
Keyboard focus uses the existing D5 focus width/offset and zebra focus color on
the same native owners without changing geometry or replacing hover/current
paint. Held native activation uses distinct navigation active/current-active
pairs; the disclosure delegates its release-time toggle to the existing
expanded contract. Native-disabled buttons retain browser suppression;
ARIA-disabled direct owners remain focusable while component-owned capture
behavior blocks pointer, programmatic, Enter, and native-button Space
activation before target and application bubble listeners.

`cem-stepper` is a workflow-navigation owner rather than a tab substitute. It
consumes strict inert `cem-step` payloads, generates an ordered set of exact
native header buttons and persistent linked regions, and exposes silent
`selected-index` control plus one serializable `cem-step` activation event.
Horizontal/vertical roving focus, linear eligibility, optional steps, editable
return, application-authored completion/invalid facts, and native/ARIA-disabled
suppression are covered without scanning panel controls. Header interaction uses
navigation state tokens; canonical workflow status/connector endpoints keep
completion and error independent in normal and forced colors. No CSS exception
or component animation is required.

`cem-tree` is a generic expandable hierarchy rather than navigation or an
application data source. Strict recursive `cem-tree-item` payloads are inert;
the tree generates exact native button treeitems, stable owned groups, explicit
hierarchy metadata, roving visible-node focus, typeahead, and component-owned
expansion. Loading and optional selection remain application-authored, while
parent toggles and leaf activations emit separate serializable events. Generic
content-interaction tokens cover exact-owner normal states and system colors
cover forced colors without wrapper paint, component animation, or a CSS
exception.

`cem-divider` owns a semantic or decorative separator track rather than a bare line. The D0 separator color is
derived from surface text at reduced salience, D5 supplies the hairline, D1 supplies relationship spacing and inset,
and D2 floors the complete line-plus-margins track at the coupling guard. Horizontal, vertical, inset, and decorative
forms remain non-focusable and event-neutral; forced colors restore the line to `CanvasText`.

`cem-expansion` owns one independent disclosure panel rather than reusing navigation-specific `cem-nav`. Its native
header button is the sole hover/focus/active/disabled owner, the live `expanded` host attribute controls a persistent
ARIA-linked panel, and the default slot remains instantiated while collapsed. Contextual action and existing
palette/spacing/control/shape/focus tokens cover the full visual contract, including forced colors, without a CSS
exception.

`cem-progress-spinner` is the circular complement to linear `cem-progress`.
Presence of `value` selects determinate semantics; absence selects
indeterminate semantics, with normalized range values projected only to ARIA.
The persistent SVG consumes generated D0 progress colors and D2c geometry;
indeterminate motion consumes the D7 continuous-cycle/uniform pair and stops
under reduced motion while leaving a static arc. Forced colors use the generated
`GrayText`/`Highlight` mapping. The component has no focus, activation, disabled,
selection, current, or live-region behavior, and requires no CSS exception.

`cem-sort-header` composes with passive `cem-table` as a sortable-column action.
Its direct native button owns pointer, keyboard, focus, active, and disabled
interaction; the generated column header conditionally owns `aria-sort`.
Activation cycles none, ascending, descending, and none, clears an active peer
only in the nearest table, and emits serializable `cem-sort` detail. Applications
retain row ordering and localized polite announcement ownership. Existing
contextual-action and D1/D2/D2c/D3/D5/D6 semantics cover normal and forced-color
paint without a CSS exception.

`cem-paginator` owns a labeled paged-content navigation landmark while leaving
data loading and item/row rendering to the application. Its native select and
button owners expose a zero-based normalized page model, optional page-size and
first/last controls, first-visible-item preservation, localizable labels, one
serializable `cem-page` request event, focus-stable boundary suppression, and
an atomic polite range status. Existing contextual-action, select/indicator,
palette, D1/D2/D2c/D3/D5/D6 semantics cover normal and forced-color styling, so
no CSS exception is needed.

`cem-slider` owns one horizontal single-value or range visual while its authored
native range inputs remain the exact pointer, keyboard, focus, value, event,
accessible-name, and form owners. Parent `min`/`max`/`step`/`disabled` state is
normalized and projected without replacing the inputs; range thumbs cannot
cross; `discrete` labels and `show-tick-marks` remain hidden visual output.
Canonical D0 slider paint, D2c track/thumb geometry, D2 targets, and D5 focus
cover normal and forced colors without a component CSS exception.

`cem-tooltip` owns one persistent plain-text description and a separate
non-interactive top-layer presentation for exactly one named native trigger.
The trigger retains pointer, focus, keyboard, activation, and application-event
ownership. Independent hover/focus reasons, Escape, delay, declarative `open`,
disabled suppression, logical CSS Anchor Positioning, and viewport fallback are
covered without long-press interception or geometry/state mutation. Existing
D0/D1/D3/D4/D5/D6 semantics cover normal and forced colors without a component
CSS exception.

## Build & Verify

```bash
yarn nx run @epa-wg/cem-components:verify
yarn nx run @epa-wg/cem-components:verify-primitives
yarn nx run @epa-wg/cem-components:verify-figma-inventory
yarn nx run @epa-wg/cem-components:verify-material-parity
yarn nx run @epa-wg/cem-components:verify-state-matrix
yarn nx run @epa-wg/cem-components:verify-style-contract
yarn nx run @epa-wg/cem-components:verify-input-indicator-forced-colors
yarn nx run @epa-wg/cem-components:verify-autocomplete-forced-colors
yarn nx run @epa-wg/cem-components:verify-navigation-hover-forced-colors
yarn nx run @epa-wg/cem-components:verify-content-hover-forced-colors
yarn nx run @epa-wg/cem-components:verify-divider-forced-colors
yarn nx run @epa-wg/cem-components:verify-expansion-forced-colors
yarn nx run @epa-wg/cem-components:verify-progress-spinner-forced-colors
yarn nx run @epa-wg/cem-components:verify-sort-header-forced-colors
yarn nx run @epa-wg/cem-components:verify-paginator-forced-colors
yarn nx run @epa-wg/cem-components:verify-slider-forced-colors
yarn nx run @epa-wg/cem-components:verify-tooltip-forced-colors
yarn nx run @epa-wg/cem-components:verify-timepicker-forced-colors
yarn nx run @epa-wg/cem-components:verify-datepicker-forced-colors
yarn nx run @epa-wg/cem-components:verify-stepper-forced-colors
yarn nx run @epa-wg/cem-components:verify-tree-forced-colors
yarn nx run @epa-wg/cem-components:verify-package
yarn nx run @epa-wg/cem-components:test
yarn nx run @epa-wg/cem-components:build
yarn nx run @epa-wg/cem-components:build:styles
yarn nx run @epa-wg/cem-components:lint
```

`yarn build` at the repo root builds every package, including this one.

`yarn nx run @epa-wg/cem-components:verify` is the Phase 3.2 production-ready trigger. It runs the primitive manifest,
Figma component inventory, state-matrix audit, token-only style contract,
package publication contract, and Node/Chromium browser coverage gates. The Figma inventory accounts for every public
primitive, validates its component/payload/structural classification, public
properties, executable states, token families, docs, and review locator, and
depends on the native five-mode theme token gate. The state-matrix audit keeps
every category/state requirement classified as browser-covered, static-only, or a gap and rejects stale test and
assertion references. Component-specific evidence lets a newly promoted owner
join a covered category state without displacing the browser evidence for its
existing owners. Intentional gaps remain visible in the generated JSON/Markdown
reports so the audit can select the next fixture without claiming that it
already exists. The style contract depends on `@epa-wg/cem-theme:build:tokens`,
and `@epa-wg/cem-theme:verify-package`, so the component gate checks current
generated tokens and the public theme stylesheet export.
The package verifier proves source/built CSS byte identity, the side-effect-free
JavaScript boundary, the built/packed behavior modules (including autocomplete,
expansion, progress spinner, sort header, paginator, slider, tooltip, timepicker,
datepicker, stepper, and tree), and exact dry-run npm inclusion of one `dist/styles.css`.

`yarn nx run @epa-wg/cem-components:test` runs the Node unit test plus Chromium-backed component harness coverage.

## Fixture Surfaces

| Surface | Path |
| ------- | ---- |
| Primitive manifest gate | `tools/scripts/verify-cem-components-primitives.mjs` |
| Figma component inventory | `../../examples/figma/component-library.json` |
| Figma component review fixture | `../../examples/figma/component-library-fixture.md` |
| Figma component inventory gate | `tools/scripts/verify-cem-components-figma-inventory.mjs` |
| Angular Material parity inventory | `tests/angular-material-parity.json` |
| Angular Material parity gate | `tools/scripts/verify-cem-components-material-parity.mjs` |
| State-matrix inventory | `tests/state-matrix-coverage.json` |
| State-matrix audit gate | `tools/scripts/verify-cem-components-state-matrix.mjs` |
| Token-only style gate | `tools/scripts/verify-cem-components-styles.mjs` |
| Input indicator forced-colors gate | `scripts/verify-input-indicator-forced-colors.mjs` |
| Autocomplete forced-colors gate | `scripts/verify-autocomplete-forced-colors.mjs` |
| Navigation hover/focus/active/disabled forced-colors gate | `scripts/verify-navigation-hover-forced-colors.mjs` |
| Content hover/focus forced-colors gate | `scripts/verify-content-hover-forced-colors.mjs` |
| Divider forced-colors gate | `scripts/verify-divider-forced-colors.mjs` |
| Expansion forced-colors gate | `scripts/verify-expansion-forced-colors.mjs` |
| Progress-spinner forced-colors/reduced-motion gate | `scripts/verify-progress-spinner-forced-colors.mjs` |
| Sort-header forced-colors gate | `scripts/verify-sort-header-forced-colors.mjs` |
| Paginator forced-colors gate | `scripts/verify-paginator-forced-colors.mjs` |
| Slider forced-colors gate | `scripts/verify-slider-forced-colors.mjs` |
| Tooltip forced-colors gate | `scripts/verify-tooltip-forced-colors.mjs` |
| Timepicker forced-colors gate | `scripts/verify-timepicker-forced-colors.mjs` |
| Datepicker forced-colors gate | `scripts/verify-datepicker-forced-colors.mjs` |
| Stepper forced-colors gate | `scripts/verify-stepper-forced-colors.mjs` |
| Tree forced-colors gate | `scripts/verify-tree-forced-colors.mjs` |
| Package publication gate | `scripts/verify-package.mjs` |
| Stylesheet copy | `scripts/copy-styles.mjs` |
| Primitive browser coverage | `src/lib/primitives.browser.spec.ts` |
| Autocomplete browser coverage | `src/lib/autocomplete.browser.spec.ts` |
| Expansion browser coverage | `src/lib/expansion.browser.spec.ts` |
| Progress-spinner browser coverage | `src/lib/progress-spinner.browser.spec.ts` |
| Sort-header browser coverage | `src/lib/sort-header.browser.spec.ts` |
| Paginator browser coverage | `src/lib/paginator.browser.spec.ts` |
| Slider browser coverage | `src/lib/slider.browser.spec.ts` |
| Stepper browser coverage | `src/lib/stepper.browser.spec.ts` |
| Stepper declarative contract fixture | `tests/stepper/contract.html` |
| Tree browser coverage | `src/lib/tree.browser.spec.ts` |
| Tree declarative contract fixture | `tests/tree/contract.html` |
| Tooltip browser coverage | `src/lib/tooltip.browser.spec.ts` |
| Timepicker browser coverage | `src/lib/timepicker.browser.spec.ts` |
| Datepicker browser coverage | `src/lib/datepicker.browser.spec.ts` |
| State and ARIA coverage | `src/lib/states.browser.spec.ts` |
| Workflow browser coverage | `src/lib/workflows.browser.spec.ts` |
| Workflow fixtures | `tests/workflows/` |
| Autocomplete contract fixture | `tests/autocomplete/contract.html` |
| Expansion contract fixture | `tests/expansion/contract.html` |
| Progress-spinner contract fixture | `tests/progress-spinner/contract.html` |
| Sort-header contract fixture | `tests/sort-header/contract.html` |
| Paginator contract fixture | `tests/paginator/contract.html` |
| Slider contract fixture | `tests/slider/contract.html` |
| Tooltip contract fixture | `tests/tooltip/contract.html` |
| Timepicker contract fixture | `tests/timepicker/contract.html` |
| Datepicker contract fixture | `tests/datepicker/contract.html` |
| Package examples | `examples/` |

## Handoff Condition

Phase 4 component expansion can start from this package when `yarn nx run @epa-wg/cem-components:verify` passes on the
branch being promoted and the working tree contains no uncommitted gate changes. That command proves the current MVP
primitive list matches `docs/component-mvp.md`, renders through the light-DOM `<cem-element>` substrate, covers the first
workflow surfaces, keeps required state and ARIA evidence explicitly classified without hiding Phase 4 gaps, and does
not introduce component-specific color or spacing literals.

Known deferrals stay outside this trigger:

- Phase 3.5 Edge/SSR processing fixtures for serialized `DataIslandSnapshot` handoff.
- Phase 3.6 `@epa-wg/custom-element` monorepo adoption.
- Richer post-MVP controls such as split actions, date ranges, date/time-zone integration, date adapters, side-nav variants, breadcrumbs,
  and richer menu/dropdown families.
- Full application behaviors around dialog focus trapping, routed navigation, async loading, and data fetching.

## Key paths

| Purpose | Path |
| ------- | ---- |
| Package source | `src/` |
| Public stylesheet source | `src/styles.css` |
| Current shell entry | `src/lib/cem-components.ts` |
| Primitive declarations | `src/lib/primitives.ts` |
| Primitive browser coverage | `src/lib/primitives.browser.spec.ts` |
| State and ARIA browser coverage | `src/lib/states.browser.spec.ts` |
| Workflow browser coverage | `src/lib/workflows.browser.spec.ts` |
| Workflow fixtures | `tests/workflows/` |
| Component test harness | `src/lib/testing/component-harness.ts` |
| Browser harness coverage | `src/lib/testing/component-harness.browser.spec.ts` |
| Component reference | `docs/component-reference.md` |
| Package-local examples | `examples/` |
| Built output | `dist/` |

## Component Docs

- [Component reference](./docs/component-reference.md) — MVP component semantics, token families, states, and
  accessibility notes.
- [Angular Material parity inventory](./docs/angular-material-parity.md) — pinned catalog mappings, gaps, and accepted
  implementation sequencing.
- [Divider contract](./docs/divider-contract.md) — line-plus-margins geometry, semantic/decorative ownership, tokens,
  and forced-colors behavior.
- [Expansion contract](./docs/expansion-contract.md) — independent disclosure ownership, live state, native events,
  ARIA references, token audit, focused fixture, and forced-colors boundary.
- [Progress spinner contract](./docs/progress-spinner-contract.md) — circular mode/value semantics, stable SVG
  geometry, D0/D2c/D7 token audit, reduced motion, forced colors, and event-neutral fixture matrix.
- [Sort header contract](./docs/sort-header-contract.md) — sortable-column ownership, fixed direction cycle,
  native events, application data/announcement boundary, theme audit, and forced-colors matrix.
- [Paginator contract](./docs/paginator-contract.md) — normalized page/range ownership, native controls, request
  events, application data boundary, token audit, and normal/forced-colors assertion matrix.
- [Autocomplete contract](./docs/autocomplete-contract.md) — accepted editable-combobox owner, form/events,
  keyboard/accessibility, token audit, focused fixture, forced-colors boundary, and assertion matrix.
- [Timepicker contract](./docs/timepicker-contract.md) — direct native text-input ownership, canonical `HH:mm`,
  interval/custom choices, listbox interaction, validation, token audit, and top-layer forced-colors boundary.
- [Datepicker contract](./docs/datepicker-contract.md) — direct native text-input ownership, canonical `YYYY-MM-DD`,
  localized modal calendar interaction, explicit confirmation, token audit, and top-layer forced-colors boundary.
- [Stepper contract](./docs/stepper-contract.md) — inert step payloads, native workflow headers, persistent panels,
  linear/optional/editable rules, application-owned completion/validation, token audit, and forced-colors boundary.
- [Tree contract](./docs/tree-contract.md) — inert recursive payloads, native treeitems, stable hierarchy IDs,
  roving focus, application-owned selection/loading, token audit, and forced-colors boundary.
- [Conventions](./docs/conventions.md) — naming, attributes, events, form participation, validation, loading states,
  progressive enhancement.
- [Light-DOM rendering rules](./docs/light-dom-rendering.md) — `@epa-wg/custom-element` compatibility, no shadow DOM,
  inert data islands, declarative slot projection, host-attribute forwarding, render lifecycle.
- [Accessibility contract](./docs/accessibility.md) — accessible names, ARIA wiring, focus, keyboard patterns, live
  regions; mirrors the Tier A semantic-validation catalog enforced by `cem_ml`.
- [Selectable list contract](./docs/selectable-list-contract.md) — accepted Phase 4 single-select listbox ownership,
  declarative option payload, native interaction boundary, and executable acceptance criteria.
- [Input loading contract](./docs/input-loading-contract.md) — explicit presence-only busy projection, native state
  and ARIA markers, tokenized pending indicator, interaction boundaries, and executable acceptance criteria.
- [Content hover contract](./docs/content-hover-contract.md) — actual native content owners, passive exclusions,
  checked/selected coexistence, disabled suppression, and forced-colors mapping.
- [Content focus-visible contract](./docs/content-focus-visible-contract.md) — native keyboard order, tokenized
  external rings, state/hover coexistence, disabled skipping, and forced-colors mapping.
- [Navigation hover contract](./docs/navigation-hover-contract.md) — owner-only pointer paint, current/selected
  coexistence, disabled suppression, forced-colors mapping, and executable acceptance criteria.
- [Navigation focus-visible contract](./docs/navigation-focus-visible-contract.md) — keyboard order, native-disabled
  skipping, tokenized external rings, state coexistence, restoration, and forced-colors mapping.
- [Navigation active contract](./docs/navigation-active-contract.md) — trusted pointer and native keyboard holds,
  current/selected coexistence, disclosure release ownership, disabled suppression, and forced-colors mapping.
- [Navigation disabled contract](./docs/navigation-disabled-contract.md) — native versus ARIA-disabled focus policy,
  capture-phase activation suppression, form neutrality, state coexistence, and forced-colors mapping.
- [Stylesheet publication contract](./docs/stylesheet-publication-contract.md) — single-source CSS build, package
  export, cache, release, and npm-pack boundary.
- [Component CSS exceptions](./docs/components-css-exceptions.md) — token-first review queue for proposed values that
  cannot yet be represented by `@epa-wg/cem-theme`.

## Related docs

- [CEM component MVP](../../docs/component-mvp.md) — first component list and state matrix.
- [CEM component examples](./examples/README.md) — package-local workflow examples, separate from executable tests.
- [CEM ML library plan](../../docs/cem-ml-library-plan.md) — the active parser/runtime path consumes/produces the
  declarative markup these components will render.
- [CEM ML acceptance criteria](../../docs/cem-ml-ac.md) — testable AC for the parser/transform stack.
- [Roadmap](../../roadmap.md) — Phase 3 (custom-element runtime) and Phase 4 (component set) define this package's
  delivery sequencing.
- [Repository documentation index](../../docs/index.md) — full project map.
