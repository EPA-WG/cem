# `@epa-wg/cem-components`

Declarative custom-element primitives that consume the CEM theme. No shadow DOM — every component renders in the
light DOM via [`@epa-wg/custom-element`](https://www.npmjs.com/package/@epa-wg/custom-element).

> **Status: shell.** The package currently re-exports the theme entry point. Component implementations land in
> Phase 3 of the [roadmap](../../roadmap.md), after the `@epa-wg/cem-dom` schema/parser/transform pipeline (Phase 2)
> is in place. The component surface is defined ahead of time in [component MVP](../../docs/component-mvp.md).

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

## Related docs

- [CEM component MVP](../../docs/component-mvp.md) — first component list and state matrix.
- [CEM DOM library plan](../../docs/dom-library-plan.md) — the upcoming `@epa-wg/cem-dom` consumes/produces the
  declarative markup these components will render.
- [CEM DOM acceptance criteria](../../docs/cem-dom-ac.md) — testable AC for the parser/transform stack.
- [Roadmap](../../roadmap.md) — Phase 3 (custom-element runtime) and Phase 4 (component set) define this package's
  delivery sequencing.
- [Repository documentation index](../../docs/index.md) — full project map.
