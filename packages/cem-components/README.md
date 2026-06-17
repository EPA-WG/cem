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
`cem-text-field`, `cem-textarea`, `cem-select`, `cem-checkbox`, `cem-radio`, `cem-switch`, `cem-surface`, `cem-text`,
`cem-icon`, `cem-stack`, `cem-grid`, `cem-list`, `cem-card`, `cem-table`, `cem-chip`, `cem-badge`, `cem-avatar`,
`cem-media-preview`, `cem-app-bar`, `cem-nav`, `cem-tabs`, `cem-dialog`, `cem-dialog-shell`, `cem-sheet`,
`cem-toast`, `cem-progress`, `cem-skeleton`, and `cem-alert`.

## Build & Verify

```bash
yarn nx run @epa-wg/cem-components:verify
yarn nx run @epa-wg/cem-components:verify-primitives
yarn nx run @epa-wg/cem-components:verify-style-contract
yarn nx run @epa-wg/cem-components:test
yarn nx run @epa-wg/cem-components:build
yarn nx run @epa-wg/cem-components:lint
```

`yarn build` at the repo root builds every package, including this one.

`yarn nx run @epa-wg/cem-components:verify` is the Phase 3.2 production-ready trigger. It runs the primitive manifest
gate, the token-only style contract gate, and the Node/Chromium browser coverage. The style contract depends on
`@epa-wg/cem-theme:build:tokens`, so the component gate checks against current generated theme token artifacts.

`yarn nx run @epa-wg/cem-components:test` runs the Node unit test plus Chromium-backed component harness coverage.

## Fixture Surfaces

| Surface | Path |
| ------- | ---- |
| Primitive manifest gate | `tools/scripts/verify-cem-components-primitives.mjs` |
| Token-only style gate | `tools/scripts/verify-cem-components-styles.mjs` |
| Primitive browser coverage | `src/lib/primitives.browser.spec.ts` |
| State and ARIA coverage | `src/lib/states.browser.spec.ts` |
| Workflow browser coverage | `src/lib/workflows.browser.spec.ts` |
| Workflow fixtures | `tests/workflows/` |
| Package examples | `examples/` |

## Handoff Condition

Phase 4 component expansion can start from this package when `yarn nx run @epa-wg/cem-components:verify` passes on the
branch being promoted and the working tree contains no uncommitted gate changes. That command proves the current MVP
primitive list matches `docs/component-mvp.md`, renders through the light-DOM `<cem-element>` substrate, covers the first
workflow surfaces, reflects required state and ARIA behavior, and does not introduce component-specific color or spacing
literals.

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
- [Conventions](./docs/conventions.md) — naming, attributes, events, form participation, validation, loading states,
  progressive enhancement.
- [Light-DOM rendering rules](./docs/light-dom-rendering.md) — `@epa-wg/custom-element` compatibility, no shadow DOM,
  inert data islands, declarative slot projection, host-attribute forwarding, render lifecycle.
- [Accessibility contract](./docs/accessibility.md) — accessible names, ARIA wiring, focus, keyboard patterns, live
  regions; mirrors the Tier A semantic-validation catalog enforced by `cem_ml`.

## Related docs

- [CEM component MVP](../../docs/component-mvp.md) — first component list and state matrix.
- [CEM component examples](./examples/README.md) — package-local workflow examples, separate from executable tests.
- [CEM ML library plan](../../docs/cem-ml-library-plan.md) — the active parser/runtime path consumes/produces the
  declarative markup these components will render.
- [CEM ML acceptance criteria](../../docs/cem-ml-ac.md) — testable AC for the parser/transform stack.
- [Roadmap](../../roadmap.md) — Phase 3 (custom-element runtime) and Phase 4 (component set) define this package's
  delivery sequencing.
- [Repository documentation index](../../docs/index.md) — full project map.
