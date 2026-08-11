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
installCemComponentPrimitives(runtime);
```

This registers the minimal primitive tags: `cem-action`, `cem-icon-button`, `cem-menu-item`, `cem-field`,
`cem-text-field`, `cem-textarea`, `cem-autocomplete`, `cem-select`, `cem-option`, `cem-option-group`, `cem-checkbox`,
`cem-radio`, `cem-switch`, `cem-surface`, `cem-text`,
`cem-icon`, `cem-stack`, `cem-grid`, `cem-divider`, `cem-list`, `cem-card`, `cem-table`, `cem-chip`, `cem-badge`, `cem-avatar`,
`cem-media-preview`, `cem-app-bar`, `cem-nav`, `cem-tabs`, `cem-dialog`, `cem-dialog-shell`, `cem-sheet`,
`cem-toast`, `cem-progress`, `cem-skeleton`, and `cem-alert`.

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

`cem-divider` owns a semantic or decorative separator track rather than a bare line. The D0 separator color is
derived from surface text at reduced salience, D5 supplies the hairline, D1 supplies relationship spacing and inset,
and D2 floors the complete line-plus-margins track at the coupling guard. Horizontal, vertical, inset, and decorative
forms remain non-focusable and event-neutral; forced colors restore the line to `CanvasText`.

## Build & Verify

```bash
yarn nx run @epa-wg/cem-components:verify
yarn nx run @epa-wg/cem-components:verify-primitives
yarn nx run @epa-wg/cem-components:verify-material-parity
yarn nx run @epa-wg/cem-components:verify-state-matrix
yarn nx run @epa-wg/cem-components:verify-style-contract
yarn nx run @epa-wg/cem-components:verify-input-indicator-forced-colors
yarn nx run @epa-wg/cem-components:verify-autocomplete-forced-colors
yarn nx run @epa-wg/cem-components:verify-navigation-hover-forced-colors
yarn nx run @epa-wg/cem-components:verify-content-hover-forced-colors
yarn nx run @epa-wg/cem-components:verify-divider-forced-colors
yarn nx run @epa-wg/cem-components:verify-package
yarn nx run @epa-wg/cem-components:test
yarn nx run @epa-wg/cem-components:build
yarn nx run @epa-wg/cem-components:build:styles
yarn nx run @epa-wg/cem-components:lint
```

`yarn build` at the repo root builds every package, including this one.

`yarn nx run @epa-wg/cem-components:verify` is the Phase 3.2 production-ready trigger. It runs the primitive manifest,
state-matrix audit, token-only style contract, package publication contract, and Node/Chromium browser coverage gates. The state-matrix audit keeps
every category/state requirement classified as browser-covered, static-only, or a gap and rejects stale test and
assertion references. Component-specific evidence lets a newly promoted owner
join a covered category state without displacing the browser evidence for its
existing owners. Intentional gaps remain visible in the generated JSON/Markdown
reports so the audit can select the next fixture without claiming that it
already exists. The style contract depends on `@epa-wg/cem-theme:build:tokens`,
and `@epa-wg/cem-theme:verify-package`, so the component gate checks current
generated tokens and the public theme stylesheet export.
The package verifier proves source/built CSS byte identity, the side-effect-free
JavaScript boundary, the built/packed autocomplete runtime, and exact dry-run
npm inclusion of one `dist/styles.css`.

`yarn nx run @epa-wg/cem-components:test` runs the Node unit test plus Chromium-backed component harness coverage.

## Fixture Surfaces

| Surface | Path |
| ------- | ---- |
| Primitive manifest gate | `tools/scripts/verify-cem-components-primitives.mjs` |
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
| Package publication gate | `scripts/verify-package.mjs` |
| Stylesheet copy | `scripts/copy-styles.mjs` |
| Primitive browser coverage | `src/lib/primitives.browser.spec.ts` |
| Autocomplete browser coverage | `src/lib/autocomplete.browser.spec.ts` |
| State and ARIA coverage | `src/lib/states.browser.spec.ts` |
| Workflow browser coverage | `src/lib/workflows.browser.spec.ts` |
| Workflow fixtures | `tests/workflows/` |
| Autocomplete contract fixture | `tests/autocomplete/contract.html` |
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
- Richer post-MVP controls such as split actions, sliders, date/time affordances, side-nav variants, breadcrumbs,
  pagination, and richer menu/dropdown families.
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
- [Autocomplete contract](./docs/autocomplete-contract.md) — accepted editable-combobox owner, form/events,
  keyboard/accessibility, token audit, focused fixture, forced-colors boundary, and assertion matrix.
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
