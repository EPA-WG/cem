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

Current active slice: promote the first executable browser substrate contract
for inline `<cem-element>` declarations into one focused runtime story/fixture.

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
- [ ] Promote the first executable browser substrate contract into one focused
      runtime story/fixture for inline `<cem-element>` declarations: exactly
      one inert declaration template, produced custom-element registration,
      captured instance data-island payload, visible light-DOM render
      ownership, and source-map fidelity labels on rendered nodes.
- [ ] Add focused unit/browser fixtures for declaration registration, payload
      capture, attribute invalidation, and data-island isolation using the
      existing `cem-elements` test harness.
- [ ] Verify with the focused `cem-elements` unit target first, then the
      Storybook browser target, substrate, legacy/material parity, demo
      fixture, and full `cem-elements:verify` gates that cover the touched
      runtime path.

### Next Work Item

Promote the first executable browser substrate contract into one focused
runtime story/fixture:

- consolidate the currently separate assertions from `InlineDeclarationShape`,
  `DataIslandCaptureAndRender`, `CemQlWasmRenderLoopUpgrade`, and the
  data-island isolation stories into one story that starts from an inline
  `<cem-element>` declaration with exactly one direct-child `<template>`;
- assert in that one story that the declaration template remains inert, the
  produced tag is registered, fallback payload is captured into
  `<template data-cem-island="instance">`, visible output is owned by light DOM,
  and rendered nodes carry template artifact, render-node, data revision, and
  source-map fidelity labels;
- add negative checks for missing inline templates, duplicate templates, and
  live declaration content near the same contract story so shape regressions are
  visible in the browser harness;
- keep `src` declarations, legacy `lang="custom-element-v0"` bridge fixtures,
  material parity, and Edge/SSR follow-up stories outside this focused contract;
- verify with `yarn nx run cem-elements:test:unit`,
  `yarn nx run cem-elements:test`, `yarn nx run cem-elements:verify-substrate`,
  `yarn nx run cem-elements:verify-demo-fixtures`, and then the full
  `yarn nx run cem-elements:verify` gate.

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
