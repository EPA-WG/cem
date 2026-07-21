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

Current active slice: add a Phase 4 component state-matrix coverage audit/gate
that ties `docs/component-mvp.md` state expectations to executable primitive,
state, and workflow browser assertions.

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

- [x] Audit the existing `packages/custom-element` source, dist package,
      package identity, release config, Nx build/test/verify targets, legacy POC
      docs/demos, material bridge fixtures, and dependency path on
      `@epa-wg/cem-elements` against the Phase 3.6 deliverables and exit
      criteria from `roadmap.md`.
      The audit found the package already migrated in-repo with preserved npm
      identity, `<custom-element>` registration, source and dist browser smoke
      fixtures, material bridge conversion, and dependency on the
      `cem-elements` substrate. The weakest proof was package baseline coverage
      for the source/dist substrate import split and release package-root
      correctness.
- [x] Populate a focused implementation slice from the audit, prioritizing the
      first missing proof around published `<custom-element>` tag continuity,
      substrate inheritance, package fixture coverage, legacy bridge support, or
      release/package-root correctness. The selected slice was to make package
      baseline verification enforce substrate inheritance and release package
      root configuration.
- [x] Add or repair the focused `@epa-wg/custom-element` package fixture,
      adapter wiring, or verification script for the selected gap without
      regressing the green Phase 3.5 Edge/SSR gates.
      Tightened `verify-package-baseline.mjs` to assert source and dist runtime
      imports, `CemElementRuntime`/legacy bridge imports, verify/test command
      shape, and `packages/custom-element/dist` publish/version roots.
- [x] Verify the selected Phase 3.6 slice with focused target(s),
      `yarn nx run @epa-wg/custom-element:verify`, and the relevant
      `cem-elements` gate if substrate behavior is touched.

### Phase 4 CEM Component Set

- [x] Audit the existing `@epa-wg/cem-components` primitive inventory,
      `docs/component-mvp.md`, component docs, browser harness, workflow
      fixtures, style contract, and dependency path on `@epa-wg/cem-elements`
      against the Phase 4 deliverables and exit criteria from `roadmap.md`.
      The audit found all 32 MVP primitives present, browser tests covering
      primitive families, state/ARIA behavior, and five executable workflow
      fixtures, plus manifest and token-only style gates. The weakest proof was
      package-local examples drifting from the executable workflow fixtures:
      profile editor and discussion thread were missing, and no static gate tied
      workflow examples back to tested fixtures.
- [x] Populate a focused implementation slice from the audit, prioritizing the
      first missing proof around Material-style UI coverage, component state
      matrix, app workflow coverage, semantic docs, token usage, or accessibility
      behavior. The selected slice was workflow example parity for the five MVP
      workflow surfaces.
- [x] Add or repair the focused component primitive, workflow fixture, docs
      page, style contract, or verification script for the selected Phase 4 gap.
      Added missing `profile-editor`, `discussion-thread`, and `settings`
      examples, documented them in `examples/README.md`, and added
      `verify-cem-components-workflows.mjs` plus a `verify-workflows` Nx target.
- [x] Verify the selected Phase 4 slice with focused `@epa-wg/cem-components`
      target(s), `yarn nx run @epa-wg/cem-components:verify`, and any relevant
      `cem-elements` gate if substrate behavior is touched.
- [x] Extend auth workflow coverage from the current sign-in fixture to the
      Phase 4 MVP auth surface: registration, password reset, and
      required/invalid/loading form states built only from existing MVP
      primitives.
- [x] Add or repair focused auth workflow fixtures, browser assertions, examples,
      and workflow-example verification so those auth surfaces remain
      declarative, accessible, token-backed, and free of one-off controls.
      Added registration and password-reset fixtures/examples with required,
      invalid, readonly, disabled/loading, progress, alert, and help/error
      relationships covered by browser assertions and workflow parity
      verification.
- [x] Verify the auth workflow slice with the focused workflow target,
      `yarn nx run @epa-wg/cem-components:test`, and
      `yarn nx run @epa-wg/cem-components:verify`.
- [ ] Add a Phase 4 component state-matrix coverage audit/gate that maps
      `docs/component-mvp.md` category state requirements to the executable
      primitive, state, and workflow browser assertions.
- [ ] Populate the first missing state fixture or assertion from that audit,
      prioritizing selected, expanded, empty, and loading coverage across
      navigation, content, and layout workflows.
- [ ] Verify the state-matrix slice with focused `@epa-wg/cem-components`
      target(s), then `yarn nx run @epa-wg/cem-components:verify`.

### Next Work Item

Extend Phase 4 state-matrix coverage:

- derive a package-local coverage map from `docs/component-mvp.md` that lists the
  required category states and the current executable proofs in
  `states.browser.spec.ts`, `primitives.browser.spec.ts`,
  `workflows.browser.spec.ts`, and workflow fixtures;
- add a verifier that fails when a required category state has no maintained
  primitive or workflow assertion;
- use the verifier output to add the first missing focused fixture/assertion,
  with priority on selected, expanded, empty, and loading states for navigation,
  content, and layout workflows;
- verify with the focused touched target first, then
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
