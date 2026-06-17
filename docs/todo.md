# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked from the roadmap.

## Immediate Release Queue

### Phase 3.1 — `<cem-element>` Browser Substrate Production Gate

Roadmap: [`../roadmap.md` §Phase 3](../roadmap.md#phase-3---custom-element-runtime). Design homes:
[`cem-element-design.md`](cem-element-design.md), [`cem-element-wasm-proposal.md`](cem-element-wasm-proposal.md).
Current scoping inventories:
[`../packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md),
[`../packages/cem-elements/docs/material-parity-inventory.md`](../packages/cem-elements/docs/material-parity-inventory.md).

Goal: make `@epa-wg/cem-elements` production-ready as the browser substrate before Phase 3.2 primitives depend on it.
The package already has runtime slices, Storybook parity stories, and inventory docs; the remaining work is to turn that
coverage into durable fixtures, close the explicit material-parity gaps, and wire the roadmap exit gates.

Recommended execution order:

- [x] **Baseline the current substrate gate.** Run and record the current status for
      `yarn nx run cem-elements:test:unit`, `yarn nx run cem-elements:test`,
      `yarn nx run cem-elements:verify-substrate`, and `yarn nx run cem-elements:verify`. Fix only harness drift in this
      slice; leave behavior gaps as named checklist items below.
      Baseline run on 2026-06-17: all four commands passed. `test:unit` passed 3 files / 63 tests; `test` passed 4
      Storybook/Vitest files / 67 tests; `verify-substrate` parsed and roundtripped 4 CEM fixtures; `verify` passed the
      aggregate gate including `cem_ml_cli:validate-fixtures`, `cem_ml_cli:e2e`, substrate roundtrip, unit tests, and
      Storybook parity stories. Observed warnings were non-fatal baseline noise: Node `NO_COLOR`/`FORCE_COLOR`, Lit dev
      mode, fixture validation warnings with zero hard violations, and expected cross-surface conversion warnings.
- [x] **Create file-backed legacy parity fixtures.** Add `packages/cem-elements/tests/parity/legacy/` fixtures mapped to
      `packages/cem-elements/docs/legacy-parity-inventory.md`: declaration registration, inline template shape,
      local/external `src`, payload capture, attribute defaults/overrides, attribute invalidation, slots, slice events,
      datadom access migration, conditionals, and the `lang="custom-element-v0"` bridge.
      The fixture directory now contains a manifest plus paired legacy/CEM-ML files for every listed behavior, and
      `yarn nx run cem-elements:verify-legacy-fixtures` verifies manifest coverage and fixture shape. Runtime promotion
      remains the next checklist item.
- [x] **Create file-backed material parity fixtures.** Add `packages/cem-elements/tests/parity/material/` fixtures for
      the eight legacy material references: `action`, `autocomplete`, `badge`, `dropdown`, `icon`, `icon-link`, `input`,
      and `menu`. Keep the import dependency order explicit: `icon`, `icon-link`, `menu` before `badge`, `action`,
      `dropdown`, `input`, then `autocomplete`.
      The fixture directory now contains a manifest plus paired legacy/CEM-ML files for all eight references, and
      `yarn nx run cem-elements:verify-material-fixtures` verifies component coverage, import order, dependency
      markers, produced tags, and fixture shape.
- [ ] **Promote Storybook parity to release gates.** Keep Storybook as the visual/debug surface, but make the Nx release
      gate assert the file-backed fixtures directly so parity does not depend on manual story inspection.
- [ ] **Prove declaration/data-island inertness in browser tests.** Cover exactly-one inline declaration template,
      `src` vs inline-template exclusivity, rejection of live declaration content, host payload capture into
      `<template data-cem-island="instance">`, removal of fallback payload before render, and proof that raw declaration
      or instance data does not affect layout, selectors, form submission, accessibility tree, or visible UI directly.
- [ ] **Harden produced-tag lifecycle behavior.** Cover tag validation, idempotent registration, duplicate declaration
      diagnostics, observed attribute extraction, declared defaults, host overrides, undeclared attribute invalidation,
      reconnect/disconnect behavior, nested produced elements, and deterministic rerender ordering.
- [ ] **Close the event-to-data render loop.** Cover `slice`, `slice-event`, `slice-value`, event payload capture,
      repeated input events, stale render avoidance, multi-instance isolation, and serialization of `data`, `option`,
      slot, dataset, attribute, slice, validation, and event payload state into the instance data island.
- [ ] **Make URI/module resolution policy explicit.** Cover fragment-only `src`, document-relative `url#fragment`,
      external document loading, module-map/specifier hooks, `module-url` resource slices, resolver failure diagnostics,
      source identity, and cache invalidation when resolver or scope policy changes.
- [ ] **Move the legacy bridge behind the shared engine boundary.** The inventory says the `custom-element-v0`
      converter still lives in `cem-elements` TypeScript. Move or wrap that compatibility compiler so browser runtime,
      CLI validation, SSR, and package gates can consume the same CEM engine path instead of a browser-only converter.
- [ ] **Decide scoped-style containment.** `material-parity-inventory.md` currently marks scoped template styles as
      partial because they render as page-global light-DOM `<style>` elements. Decide whether containment is required
      for the Phase 3.1 production gate. If yes, implement and test the containment model; if no, document it as
      bridge/adoption work and make the gate assert the documented behavior.
- [ ] **Define the host processing boundary in code.** Add or harden TypeScript runtime-support contracts for
      `DataIslandSnapshot`, `RenderRevision`, template artifact refs, source-map refs, render-plan identity, patch
      frames, and patch apply results. Add structured-clone safety tests proving no live `Node`, `Event`, function,
      class instance, `Map`, `Set`, `Date`, or browser handle crosses the processing boundary.
- [ ] **Wire the first WASM-backed template-processing path.** Select the minimal Phase 3 path from
      `cem-element-wasm-proposal.md`: compile inline/external CEM-ML or DOM-parity source, run CEM-QL expressions,
      produce light-DOM render plans or patch frames, return structured diagnostics, and keep DOM patch application on
      the main-thread UI adapter.
- [ ] **Track source-map fidelity through render output.** Cover `author-byte-exact` for external/raw CEM sources,
      `dom-canonical` for DOM-parsed inline XML/HTML parity, and `declaration-only` fallback diagnostics. Rendered nodes
      should carry enough source-map identity for fixture assertions and devtools reporting.
- [ ] **Integrate Phase 2 verification gates.** Ensure the Phase 3.1 gate runs the parser/runtime checks named in the
      roadmap against the parity fixtures: `yarn nx run cem_ml_cli:validate-fixtures`, `yarn nx run cem_ml_cli:e2e`,
      and `yarn nx run cem_ml:bench`, plus the existing `cem-elements:verify-substrate` path.
- [ ] **Add accessibility and first-paint gates for material parity.** Use
      `packages/cem-components/docs/accessibility.md` as the contract. Assert keyboard/focus/label/live-region behavior
      where applicable, and add a deterministic first-paint/performance smoke so AC-N-1-style regressions are visible
      before Phase 3.2 primitives depend on the substrate.
- [ ] **Document the production-ready trigger.** Update `packages/cem-elements/README.md` and the parity inventories
      with the final command set, fixture locations, known bridge/adoption deferrals, and the exact handoff condition
      for Phase 3.5 Edge/SSR and Phase 3.6 `@epa-wg/custom-element` adoption.

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
