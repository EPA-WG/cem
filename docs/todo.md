# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).
Each item names the AC reference and design home so the closing change ship with a citation.

## Phase 3 — Custom-Element Runtime Preparation

Roadmap: [`../roadmap.md` §Phase 3](../roadmap.md). Component vocabulary: [`component-mvp.md`](component-mvp.md).
Start only when Phase 2 Tier A surfaces are stable enough to consume.

### 3.1 Substrate — `@epa-wg/cem-elements`

Design home: [`cem-element-design.md`](cem-element-design.md). Substrate work gates 3.2 primitive implementation.

- [x] Draft `cem-element` design: `<template>`-wrapped data island, cem-ml templates, cem-ql expressions, monorepo
      migration plan, parity criteria. Landed in [`cem-element-design.md`](cem-element-design.md).
- [ ] review/rewise the `cem-element` design.
- [ ] Migrate `@epa-wg/custom-element` from `~/aWork/custom-element/` into `packages/custom-element/`. Preserve
      published npm identity and history.
- [ ] Scaffold `packages/cem-elements/` (new package). Wire `nx run cem-elements:build/test/lint`.
- [ ] Implement the `<cem-element>` runtime: `<template>` discovery, cem-ml lowering, data-island event wiring,
      light-DOM render loop, source-map carry-through.
- [ ] Land legacy parity fixtures under `packages/cem-elements/tests/parity/legacy/` covering every behavior in
      `~/aWork/custom-element/docs/{attributes,rendering}.md`.
- [ ] Land material parity fixtures under `packages/cem-elements/tests/parity/material/` for every component in
      `~/aWork/custom-element-dist/src/material/` (action, autocomplete, badge, dropdown, icon, icon-link, input,
      menu).
- [ ] Wire `cem-element` through `nx run cem_ml_cli:validate-fixtures` and `cem_ml_cli:e2e` so substrate templates
      ride the same Phase 2 verification.
- [ ] Production-ready gate: parity (1)–(5) from [`cem-element-design.md` §7](cem-element-design.md). When green,
      fold `cem-element` into the next major of `@epa-wg/custom-element` and archive `@epa-wg/cem-elements`.
- [ ] Bridge support: `<template lang="custom-element-v0">` compat path for legacy authoring during the migration
      window; remove at the major cutover.

### 3.2 Primitives — `@epa-wg/cem-components`

Authored exclusively against `<cem-element>` (3.1). The contract docs below name `<cem-element>` and `cem-ql` as the
authoring surface.

- [x] Define base CEM custom-element conventions: naming, attributes, events, form participation, validation, loading
      states, progressive enhancement. Landed in
      [`packages/cem-components/docs/conventions.md`](../packages/cem-components/docs/conventions.md).
- [x] Define light-DOM rendering rules and compatibility expectations with `<cem-element>` (no shadow DOM). Landed in
      [`packages/cem-components/docs/light-dom-rendering.md`](../packages/cem-components/docs/light-dom-rendering.md).
- [x] Define the accessibility contract: labels, descriptions, focus, keyboard behavior, roles, live regions. Landed
      in [`packages/cem-components/docs/accessibility.md`](../packages/cem-components/docs/accessibility.md).
- [ ] Build the test harness for DOM rendering, events, accessibility assertions, and visual snapshots.
- [ ] Implement minimal primitives: action, field, surface, text, icon, stack, grid, list, nav, dialog shell.

## Phase 5 — Figma UI Kit Token Validation (`examples/figma`)

Roadmap: [`../roadmap.md` §Phase 5](../roadmap.md). Token export contract:
[`../packages/cem-theme/docs/token-export.md`](../packages/cem-theme/docs/token-export.md). Figma library workflow:
[`../packages/cem-theme/docs/token-figma.md`](../packages/cem-theme/docs/token-figma.md). These items moved from
Phase 1 because the validation is only meaningful against a populated Figma UI Kit.

- [ ] Validate native Figma library variables against the generated `figma/cem-*.tokens.json` files for every mode.
      Surface the validation in `nx run @epa-wg/cem-theme:test:figma` (new target) or extend the existing
      token-platform report. Block release when a mode disagrees with the canonical spine.
- [ ] Extend the token-change smoke test with the Figma propagation leg: change one canonical token, refresh the Figma
      mode files, and assert the UI Kit variables reflect the change without manual rework. Track gaps in
      `token-pipeline-smoke.md`. The non-Figma leg of the same smoke test lives under Phase 8.

## Phase 8 — Native Platform Packages (`@epa-wg/cem-theme` native outputs)

Roadmap: [`../roadmap.md` §Phase 8](../roadmap.md). Token export contract:
[`../packages/cem-theme/docs/token-export.md`](../packages/cem-theme/docs/token-export.md). These items moved from
Phase 1 because they validate Phase 8 native artifacts (iOS Swift, Android Kotlin/Compose) and are gated by the
available toolchains, not the Phase 1 token-spine work that already shipped.

- [ ] Compile generated Swift (`packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`) with a supported Xcode
      toolchain. Add the compile step as a release gate; fail loudly when symbols drift.
- [ ] Compile generated Kotlin/Compose (`packages/cem-theme/dist/lib/token-platforms/android/`) with the supported
      Gradle toolchain. Add the compile step as a release gate.
- [ ] Wire a token-change smoke test for the non-Figma propagation path: change one canonical token, regenerate CSS,
      JSON, Swift, and Android outputs, and assert every artifact moves coherently. Track gaps in
      `token-pipeline-smoke.md`. (The Figma propagation leg of the same smoke test lives in Phase 5.)
