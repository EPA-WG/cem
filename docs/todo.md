# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).
Each item names the AC reference and design home so the closing change ship with a citation.

## Phase 3 — Custom-Element Runtime Preparation

Roadmap: [`../roadmap.md` §Phase 3](../roadmap.md). Component vocabulary: [`component-mvp.md`](component-mvp.md).
Start only when Phase 2 Tier A surfaces are stable enough to consume.

### 3.1 Substrate — `@epa-wg/cem-elements`

Design home: [`cem-element-design.md`](cem-element-design.md). WASM proposal:
[`cem-element-wasm-proposal.md`](cem-element-wasm-proposal.md). Substrate work gates 3.2 primitive implementation.

- [x] Draft `cem-element` design: `<template>`-wrapped data island, cem-ml templates, cem-ql expressions, monorepo
      migration plan, parity criteria. Landed in [`cem-element-design.md`](cem-element-design.md).
- [x] Review and revise the `cem-element` design against the legacy data-island lifecycle, instance payload capture,
      and material parity requirements.
- [x] Draft `cem-element` WASM integration proposal: inline and URI declaration templates, module-map resolution,
      remote source streaming, local parser streaming, reusable host runtime support, patch-frame streams,
      worker-pool options, edge/SSR processing topologies, and runtime fallback strategy. Landed in
      [`cem-element-wasm-proposal.md`](cem-element-wasm-proposal.md).
- [x] Lock Phase 3 MVP topology from [`cem-element-wasm-proposal.md`](cem-element-wasm-proposal.md): primary
      worker-backed WASM with stream inputs, main-thread WASM fallback, and edge/SSR/threaded/precompiled/service-worker
      paths deferred unless explicitly promoted. Landed in [`cem-element-design.md` §4.3](cem-element-design.md).
- [x] Decide URI declaration syntax: URI lives on `<cem-element src="…">`, matching the legacy
      `<custom-element src="…">` shape. Both `<template src="…">` and
      `<cem-element template-src="…">` are rejected. Landed in
      [`cem-element-design.md` §3.2](cem-element-design.md) and
      [`cem-element-wasm-proposal.md` §2.2–2.3](cem-element-wasm-proposal.md).
- [x] Define the JS/WASM artifact wire format for Phase 3: structured-clone objects, JSON, transferable
      `ArrayBuffer`/binary AST, or a hybrid. Document which shapes cross worker boundaries for template artifacts,
      render plans, diagnostics, and source maps. Resolved as the hybrid Option D format designed for later Option C
      binary payload migration. Landed in [`cem-element-design.md` §4.2](cem-element-design.md) and
      [`cem-element-wasm-proposal.md` §14](cem-element-wasm-proposal.md).
- [x] Define the patch transport contract: `PatchFrame`, `DomPatchOp`, `DomPatchPlan`, render sequence handling,
      stale-frame dropping, abort/commit frames, and the host-neutral `PatchApplier` interface. Resolved with
      `RenderRevision` ordering, stable render-node-id ops, constrained `replaceScope` fallback, buffered
      transactions, indexed op batches, and a host-neutral `PatchApplier`. Landed in
      [`cem-element-design.md` §4.2](cem-element-design.md) and
      [`cem-element-wasm-proposal.md` §8](cem-element-wasm-proposal.md).
- [x] Define the serializable processing boundary for UI/worker/edge/SSR hosts: `DataIslandSnapshot`, render-plan
      identity, scope policy stamp, resolver identity, cache identity, patch-frame transport, and privacy rules for
      data leaving the browser. Landed in [`cem-element-design.md` §4.2](cem-element-design.md).
- [x] Decide the initial worker-pool default: single worker first or small pool by scope policy. Document fallback
      behavior when workers or `SharedArrayBuffer` are unavailable. Resolved as one dedicated worker by default for
      Phase 3A, with scope-policy worker pools deferred to Phase 3B and main-thread WASM fallback when workers fail
      or are unavailable. `SharedArrayBuffer` is optional; threaded WASM falls back to non-threaded worker message
      passing, then main-thread WASM. Landed in [`cem-element-design.md` §4.3](cem-element-design.md) and
      [`cem-element-wasm-proposal.md` §9/§14](cem-element-wasm-proposal.md).
- [x] Define Phase 3 cache identity fields for template artifacts and render plans: source hash, URL/specifier,
      resolver identity, scope policy stamp, `cem_ml` version, `cem_ql` version, and dev/prod source-map mode.
      Resolved as the two-level Option C identity: portable `TemplateArtifactPayloadKey` plus host-specific
      `TemplateArtifactIdentity`, with render plans keyed by template artifact identity, `RenderRevision`, render
      engine version, and source-map mode. Landed in [`cem-element-design.md` §4.2](cem-element-design.md) and
      [`cem-element-wasm-proposal.md` §14](cem-element-wasm-proposal.md).
- [x] Set the accepted source-map fidelity for DOM-parsed inline XML/HTML parity templates where original browser
      source bytes are unrecoverable. Resolved with `SourceMapFidelity` markers:
      `author-byte-exact`, `dom-canonical`, and `declaration-only`. DOM-parsed inline XML/HTML parity may pass with
      `dom-canonical`; exact author bytes remain required for external, fetched, and raw text sources.
      `declaration-only` is fallback-only. Landed in [`cem-element-design.md` §4.2](cem-element-design.md) and
      [`cem-element-wasm-proposal.md` §4.2/§14](cem-element-wasm-proposal.md).
- [x] Decide host runtime support packaging: internal module inside `@epa-wg/cem-elements` first, or separate
      reusable package/module for `<custom-element>`, docs/playgrounds, tests, SSR, and edge hosts. Resolved as
      Option D: internal `@epa-wg/cem-elements/internal/runtime-support` for Phase 3A, authored for later extraction
      to reserved package name `@epa-wg/cem-runtime-support` after material parity, worker/cache/source-map fixture
      coverage, the Edge/SSR follow-up phase, and adoption-phase `<custom-element>` consumption. Landed in
      [`cem-element-wasm-proposal.md` §6/§14](cem-element-wasm-proposal.md).
- [x] Decide Phase 3 edge/SSR scope: design-only boundary, verification fixtures, or hard runtime deliverable.
      Resolved: Phase 3 keeps only the serializable boundary and topology notes. Edge/SSR fixtures, SSR bootstrap,
      edge patch streams, privacy/export verification, and render-state storage decisions move to the separate
      Phase 3.5 follow-up. Landed in [`cem-element-wasm-proposal.md` §7.3/§11/§14](cem-element-wasm-proposal.md)
      and [`../roadmap.md` §Phase 3.5](../roadmap.md).
- [x] Decide whether service-worker template/artifact registry is Phase 3 scope or deferred until after component
      parity. Resolved as Option C: Phase 3 defines service-worker-compatible artifact identity,
      namespace/version metadata, and optional registry hooks, but the concrete service-worker registry is deferred
      until after component parity. Landed in [`cem-element-design.md` §4.2](cem-element-design.md) and
      [`cem-element-wasm-proposal.md` §10/§11/§13/§14](cem-element-wasm-proposal.md).
- [x] Scaffold `packages/cem-elements/` (new package). Wire `nx run cem-elements:build/test/lint`.
- [ ] Implement the `<cem-element>` runtime in execution slices from
      [`cem-element-design.md` §3–§5](cem-element-design.md):
  - [x] Runtime slice A: define `<cem-element>`, validate inline declaration shape, reject `src`+inline template
        conflicts, and register produced custom-element tags from `tag`.
  - [x] Runtime slice B: initialize produced instances, create/reuse
        `<template data-cem-island="instance">`, capture host attributes/dataset/fallback payload, and remove raw
        fallback payload before first render.
  - [x] Runtime slice C1: lower inline XML/HTML parity declaration templates through the available DOM
        parser/projection boundary and install a minimal light-DOM render loop. The package-private
        `projection.ts` boundary reads parsed template content into serializable source records, projects against a
        `DataIslandSnapshot`-based input, and materializes the render plan back into light DOM.
  - [x] Runtime slice C1.5: add package-private canonical CEM-ML subset lowering for `{name @attr=value | ...}`
        templates so the browser runtime can exercise the same render-plan path before the final WASM API exists.
  - [ ] Runtime slice C2: lower canonical inline CEM-ML declaration templates through the `cem_ml`
        WASM/runtime-support boundary into the same render-plan shape. Current runtime recognizes CEM-ML and
        renders the supported C1.5 subset through the temporary TypeScript adapter; the production `cem_ml`
        WASM/runtime-support boundary remains open.
  - [x] Runtime slice D: wire attribute changes and declarative data-island/event updates to render invalidation.
        Observed declaration attributes rerender produced instances; rendered `slice`/`slice-event`/`slice-value`
        bindings update package-local slice state and rerender through the same `DataIslandSnapshot` path; inert
        data-island template content is observed for mutation-driven invalidation.
  - [x] Runtime slice E: carry source-map/render identity metadata through rendered nodes and expose diagnostics for
        declaration, parsing, and render failures. Rendered elements carry render-node id, template artifact id, data
        revision, and source-fidelity/source-frame attributes. DOM parity templates use `dom-canonical`; temporary
        raw-text CEM-ML subset templates use `author-byte-exact`. Declaration parse diagnostics and render failures
        are exposed through `diagnosticsFor`.
- [x] Add Storybook as the primary browser/runtime test runner for `packages/cem-elements`, with Nx targets for
      interactive Storybook and CI Storybook Test execution through `@storybook/addon-vitest`.
- [ ] Add Storybook browser stories proving data-island isolation: declaration and instance template contents do not
      affect layout, selectors, form submission, accessibility, or visible UI directly.
- [ ] Build the legacy parity feature inventory from the old `@epa-wg/custom-element` suite and docs
      (`~/aWork/custom-element/docs/{attributes,rendering}.md` plus legacy test files). Convert every in-scope
      behavior into a named `<cem-element>` Storybook parity story; record intentional CEM-ML/CEM-QL replacements as
      migration decisions.
- [ ] Land material parity stories for every component in `~/aWork/custom-element-dist/src/material/` (action,
      autocomplete, badge, dropdown, icon, icon-link, input, menu).
- [ ] Build a material parity inventory from `~/aWork/custom-element-dist/src/material/components/*.html` covering
      local/external `src`, hidden declarations, nested custom elements, declarative slots, scoped styles,
      `attribute select`, `if`/`choose` bridge constructs, namespaced `xhtml:*` elements, boolean attribute helper
      semantics, `module-url` resource slices, `data`/`option` payloads, slice events, and `slice-value`.
- [ ] Wire `cem-element` through `nx run cem_ml_cli:validate-fixtures` and `cem_ml_cli:e2e` so substrate templates
      ride the same Phase 2 verification.
- [ ] Production-ready gate: parity (1)–(6) from [`cem-element-design.md` §7](cem-element-design.md). When green,
      the browser substrate is eligible for Phase 3.5 Edge/SSR follow-up; `@epa-wg/custom-element` adoption remains
      deferred until after that follow-up phase.
- [ ] Bridge support: `<template lang="custom-element-v0">` compat path for legacy authoring during the migration
      window; keep only if needed after the `@epa-wg/custom-element` substrate adoption.

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

## Phase 3.5 — Edge/SSR Processing Follow-Up

Roadmap: [`../roadmap.md` §Phase 3.5](../roadmap.md). Starts after the Phase 3 browser substrate production-ready
gate is green.

- [ ] Add SSR fixture that renders initial HTML plus hydration metadata from a serialized `DataIslandSnapshot`, then
      hydrates into the same client-side data-island and render-plan identity.
- [ ] Add edge-processing fixture using serialized data snapshot plus previous render-plan identity to produce a
      patch-frame stream without access to live browser DOM.
- [ ] Verify privacy/export policy for browser-to-edge snapshots: denied fields are omitted or redacted before
      leaving the browser context.
- [ ] Decide the first edge render-state storage model: content-addressed cache only, revisioned KV/document records,
      or both.

## Phase 3.6 — `@epa-wg/custom-element` Monorepo Adoption

Roadmap: [`../roadmap.md` §Phase 3.6](../roadmap.md). Starts after Phase 3.5 is green.

- [ ] Migrate `@epa-wg/custom-element` from `~/aWork/custom-element/` into `packages/custom-element/`. Preserve
      published npm identity and history.
- [ ] Make the next major of `@epa-wg/custom-element` keep publishing `<custom-element>` with an implementation that
      inherits the `cem-element` substrate.
- [ ] Verify the migrated package against legacy parity, material parity, and Phase 3.5 Edge/SSR follow-up fixtures.

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
