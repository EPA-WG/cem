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

Current active slice: add primitive-family visual snapshot coverage for
`@epa-wg/cem-components` using the existing browser harness, starting with the
action, input, layout/content, navigation, and feedback families already covered
by DOM/ARIA tests.

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
- [ ] Add primitive-family visual snapshot coverage around the selected
      representative primitives: action controls, input controls, layout/content
      containers, navigation landmarks, and feedback/status surfaces.
- [ ] Verify the selected primitive slice with focused
      `@epa-wg/cem-components` target(s), then
      `yarn nx run @epa-wg/cem-components:verify`, and the relevant
      `cem-elements` gate if substrate behavior is touched.

### Next Work Item

Add primitive-family visual snapshot coverage in `@epa-wg/cem-components`:

- reuse `captureVisualSnapshot()` from
  `packages/cem-components/src/lib/testing/component-harness.ts` rather than
  adding a new screenshot system;
- add a focused browser spec, or extend `primitives.browser.spec.ts`, with one
  deterministic fixture per family: action controls, input controls,
  layout/content containers, navigation landmarks, and feedback/status surfaces;
- assert stable rendered HTML/text, non-zero dimensions, expected display/style
  properties, and token-resolved computed styles where the current theme CSS is
  available;
- keep behavior assertions in the existing DOM/ARIA/state/workflow specs and use
  this slice only for visual regression shape;
- verify with `yarn nx run @epa-wg/cem-components:test-ci--src/lib/primitives.browser.spec.ts`
  if the coverage lands there, then
  `yarn nx run @epa-wg/cem-components:test`, and finally
  `yarn nx run @epa-wg/cem-components:verify`.

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
