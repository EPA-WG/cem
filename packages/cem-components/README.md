# `@epa-wg/cem-components`

Declarative component primitives that consume the CEM theme. No shadow DOM — every component renders in the
light DOM, authored against the `<cem-element>` substrate from `@epa-wg/cem-elements` (functional successor to
`@epa-wg/custom-element`; design home: [`docs/cem-element-design.md`](../../docs/cem-element-design.md)).

> **Status: minimal primitives.** The package exports the Phase 3.2 browser test harness and the first installable
> primitive declaration set from the [component MVP](../../docs/component-mvp.md), registered through the
> production-ready `<cem-element>` substrate.

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

This registers the minimal primitive tags: `cem-action`, `cem-field`, `cem-surface`, `cem-text`, `cem-icon`,
`cem-stack`, `cem-grid`, `cem-list`, `cem-nav`, and `cem-dialog-shell`.

## Build & test

```bash
yarn build 
# or 
nx run @epa-wg/cem-components:build
nx run @epa-wg/cem-components:test
nx run @epa-wg/cem-components:lint
```

`yarn build` at the repo root builds every package, including this one.

`nx run @epa-wg/cem-components:test` runs the existing Node unit tests plus the Chromium-backed component harness
coverage.

## Key paths

| Purpose | Path |
| ------- | ---- |
| Package source | `src/` |
| Current shell entry | `src/lib/cem-components.ts` |
| Primitive declarations | `src/lib/primitives.ts` |
| Primitive browser coverage | `src/lib/primitives.browser.spec.ts` |
| Component test harness | `src/lib/testing/component-harness.ts` |
| Browser harness coverage | `src/lib/testing/component-harness.browser.spec.ts` |
| Built output | `dist/` |

## Component contracts

Phase 3 contract docs (landed; pre-implementation):

- [Conventions](./docs/conventions.md) — naming, attributes, events, form participation, validation, loading states,
  progressive enhancement.
- [Light-DOM rendering rules](./docs/light-dom-rendering.md) — `@epa-wg/custom-element` compatibility, no shadow DOM,
  inert data islands, declarative slot projection, host-attribute forwarding, render lifecycle.
- [Accessibility contract](./docs/accessibility.md) — accessible names, ARIA wiring, focus, keyboard patterns, live
  regions; mirrors the Tier A semantic-validation catalog enforced by `cem_ml`.

## Related docs

- [CEM component MVP](../../docs/component-mvp.md) — first component list and state matrix.
- [CEM ML library plan](../../docs/cem-ml-library-plan.md) — the active parser/runtime path consumes/produces the
  declarative markup these components will render.
- [CEM ML acceptance criteria](../../docs/cem-ml-ac.md) — testable AC for the parser/transform stack.
- [Roadmap](../../roadmap.md) — Phase 3 (custom-element runtime) and Phase 4 (component set) define this package's
  delivery sequencing.
- [Repository documentation index](../../docs/index.md) — full project map.
