# `@epa-wg/cem-components`

Declarative component primitives that consume the CEM theme. No shadow DOM — every component renders in the
light DOM, authored against the `<cem-element>` substrate from `@epa-wg/cem-elements` (functional successor to
`@epa-wg/custom-element`; design home: [`docs/cem-element-design.md`](../../docs/cem-element-design.md)).

> **Status: shell.** The package currently re-exports the theme entry point. Component implementations land in
> Phase 3.2 of the [roadmap](../../roadmap.md), after the `cem-ml` / `cem-ml-cli` schema/parser/transform pipeline
> (Phase 2) and the `<cem-element>` substrate (Phase 3.1) are in place. The component surface is defined ahead of time
> in [component MVP](../../docs/component-mvp.md).

## Install

```bash
yarn add @epa-wg/cem-components
```

This package depends on `@epa-wg/cem-theme`; install it alongside.

## Build & test

```bash
yarn build 
# or 
nx run @epa-wg/cem-components:build
nx run @epa-wg/cem-components:test
nx run @epa-wg/cem-components:lint
```

`yarn build` at the repo root builds every package, including this one.

## Key paths

| Purpose | Path |
| ------- | ---- |
| Package source | `src/` |
| Current shell entry | `src/lib/cem-components.ts` |
| Built output | `dist/` |

## Component contracts

Phase 3 contract docs (landed; pre-implementation):

- [Conventions](./docs/conventions.md) — naming, attributes, events, form participation, validation, loading states,
  progressive enhancement.
- [Light-DOM rendering rules](./docs/light-dom-rendering.md) — `@epa-wg/custom-element` compatibility, no shadow DOM,
  host-attribute forwarding, render lifecycle.
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
