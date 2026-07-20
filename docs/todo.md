# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Phase 2 exit criteria from [`../roadmap.md`](../roadmap.md) are closed: the
parser/runtime report-consumer slice, browser runtime proof, stable chunked
binary handoff, embedded-payload handoff coverage, and web-host coordinate
projection have all passed their focused and gate verification. Continue with
Phase 3 substrate expansion.

Current active slice: audit the existing `@epa-wg/cem-components` package
against Phase 3.2 primitives deliverables, exit criteria, docs, component
inventory, and verification targets before selecting the first implementation
slice.

### Phase 3 Custom-Element Runtime

- [x] Audit the existing `@epa-wg/cem-elements` substrate implementation,
      parity inventories, Storybook fixtures, and Nx verification targets
      against the Phase 3.1 exit criteria from `roadmap.md`.
- [x] Tighten inline declaration shape validation to match
      `docs/cem-element-design.md`: inline declarations require exactly one
      direct-child WHATWG `<template>`, `src` declarations must not include an
      inline template, live declaration content is rejected, and the current
      implicit CEM-ML fallback path is removed or converted into an explicit
      diagnostic.
- [x] Promote the first executable browser substrate contract into one focused
      runtime story/fixture for inline `<cem-element>` declarations: exactly
      one inert declaration template, produced custom-element registration,
      captured instance data-island payload, visible light-DOM render
      ownership, and source-map fidelity labels on rendered nodes.
- [x] Add focused unit/browser fixtures for declaration registration, payload
      capture, attribute invalidation, and data-island isolation using the
      existing `cem-elements` test harness.
- [x] Verify with the focused `cem-elements` unit target first, then the
      Storybook browser target, substrate, legacy/material parity, demo
      fixture, and full `cem-elements:verify` gates that cover the touched
      runtime path.

### Phase 3.2 CEM Primitives

- [ ] Audit the existing `@epa-wg/cem-components` package structure, docs,
      component inventory, Storybook/test harness, and dependency path on
      `@epa-wg/cem-elements` against the Phase 3.2 deliverables and exit
      criteria from `roadmap.md`.
- [ ] Populate a focused implementation slice from the audit: the minimal
      primitive or harness gap that blocks declarative no-JS component usage
      before expanding the catalog.
- [ ] Add or repair focused tests for DOM rendering, events, accessibility
      assertions, and visual snapshots around the selected primitive slice.
- [ ] Verify the selected primitive slice with focused
      `@epa-wg/cem-components` target(s), then
      `yarn nx run @epa-wg/cem-components:verify`, and the relevant
      `cem-elements` gate if substrate behavior is touched.

### Next Work Item

Audit Phase 3.2 primitives readiness in `@epa-wg/cem-components` before
starting component implementation:

- inspect `packages/cem-components/project.json`, source files, stories/tests,
  docs, and package dependencies to identify what already exists;
- map the current component inventory against the roadmap's minimal primitives:
  action, field, surface, text, icon, stack, grid, list, nav, and dialog shell;
- check whether existing primitives are authored exclusively with
  `<cem-element>` and keep the no-shadow-DOM/light-DOM rendering contract;
- identify the first narrow primitive or harness gap that should be implemented
  next, avoiding broad catalog work until the audit proves the target;
- update `docs/todo.md` with the resulting focused slice and recommended Nx
  verification commands.

## Current Verification Commands

- `yarn nx run @epa-wg/cem-theme:verify:phase13`
- `yarn nx run cem-elements:test:unit`
- `yarn nx run cem-elements:test`
- `yarn nx run cem-elements:verify-substrate`
- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`
- `yarn nx run cem-elements:verify-edge-ssr`
- `yarn nx run @epa-wg/custom-element:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
