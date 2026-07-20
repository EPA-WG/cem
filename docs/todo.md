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

Current active slice: audit Phase 3.6 `@epa-wg/custom-element` monorepo
adoption readiness against `roadmap.md`, starting from the existing
`packages/custom-element` package, its Nx gates, and its bridge fixtures.

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

- [x] Audit the existing `@epa-wg/cem-components` package structure, docs,
      component inventory, Storybook/test harness, and dependency path on
      `@epa-wg/cem-elements` against the Phase 3.2 deliverables and exit
      criteria from `roadmap.md`.
- [x] Populate a focused implementation slice from the audit: add
      primitive-family visual snapshot coverage, because the existing package
      already has the `@epa-wg/cem-elements` dependency, CEM-ML primitive
      declarations, DOM rendering tests, event/state tests, accessibility
      assertions, workflow fixtures, manifest verification, and token-only style
      verification, while visual snapshots are currently exercised only by the
      harness smoke test.
- [x] Add primitive-family visual snapshot coverage around the selected
      representative primitives: action controls, input controls, layout/content
      containers, navigation landmarks, and feedback/status surfaces.
- [x] Verify the selected primitive slice with focused
      `@epa-wg/cem-components` target(s), then
      `yarn nx run @epa-wg/cem-components:verify`, and the relevant
      `cem-elements` gate if substrate behavior is touched.

### Phase 3.5 Edge/SSR Processing Follow-Up

- [x] Audit the existing `@epa-wg/cem-elements` Edge/SSR processing APIs, unit
      fixtures, Storybook stories, privacy/export policy coverage, render-state
      storage helpers, and Nx verification gates against the Phase 3.5
      deliverables and exit criteria from `roadmap.md`.
      The audit found SSR hydration, edge patch-frame, privacy/export policy,
      render-state storage, and `verify-edge-ssr` coverage already present; the
      weakest proof was privacy/export default-deny and redaction behavior living
      only in a browser story instead of the fast processing-boundary unit suite.
- [x] Populate a focused implementation slice from the audit, prioritizing the
      first missing proof around SSR hydration metadata, edge patch-frame
      streaming without live DOM access, privacy/export policy enforcement, or
      render-state storage identity. The selected slice was a unit-level
      privacy/export policy fixture.
- [x] Add or repair the focused `cem-elements` fixture(s) for the selected gap
      while keeping browser worker and main-thread fallback semantics unchanged.
      Added a `processing-boundary.spec.ts` unit fixture that asserts default
      omission, explicit redaction, export policy stamp override, and detached
      exported host attributes before snapshots leave for edge hosts.
- [x] Verify the selected Phase 3.5 slice with focused unit/browser target(s),
      `yarn nx run cem-elements:verify-edge-ssr`, and
      `yarn nx run cem-elements:verify`.

### Phase 3.6 `@epa-wg/custom-element` Monorepo Adoption

- [ ] Audit the existing `packages/custom-element` source, dist package,
      package identity, release config, Nx build/test/verify targets, legacy POC
      docs/demos, material bridge fixtures, and dependency path on
      `@epa-wg/cem-elements` against the Phase 3.6 deliverables and exit
      criteria from `roadmap.md`.
- [ ] Populate a focused implementation slice from the audit, prioritizing the
      first missing proof around published `<custom-element>` tag continuity,
      substrate inheritance, package fixture coverage, legacy bridge support, or
      release/package-root correctness.
- [ ] Add or repair the focused `@epa-wg/custom-element` package fixture,
      adapter wiring, or verification script for the selected gap without
      regressing the green Phase 3.5 Edge/SSR gates.
- [ ] Verify the selected Phase 3.6 slice with focused target(s),
      `yarn nx run @epa-wg/custom-element:verify`, and the relevant
      `cem-elements` gate if substrate behavior is touched.

### Next Work Item

Audit Phase 3.6 `@epa-wg/custom-element` monorepo adoption readiness:

- compare `roadmap.md` Phase 3.6 and `docs/cem-element-design.md` sections 6.1
  and 6.2 against the current `packages/custom-element` package;
- inspect `packages/custom-element/package.json`,
  `packages/custom-element/project.json`, `packages/custom-element/custom-element.js`,
  `packages/custom-element/index.js`, `packages/custom-element/scripts/*.mjs`,
  `packages/custom-element/test-fixtures/`, and
  `packages/custom-element/material/` to map package identity, substrate
  inheritance, bridge support, and fixture coverage;
- confirm which migration work is already landed in-repo versus which Phase 3.6
  proofs remain, especially published `<custom-element>` tag continuity,
  next-major substrate adoption, package fixture coverage, and package-root
  release correctness;
- identify the first narrow missing or weak proof and add it to this checklist
  before implementing;
- verify with the focused touched target first, then
  `yarn nx run @epa-wg/custom-element:verify`, and
  `yarn nx run cem-elements:verify-edge-ssr` if substrate behavior is touched.

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
