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
- [x] **Promote Storybook parity to release gates.** Keep Storybook as the visual/debug surface, but make the Nx release
      gate assert the file-backed fixtures directly so parity does not depend on manual story inspection.
      `cem-elements:verify` now depends on `verify-legacy-fixtures` and `verify-material-fixtures` alongside
      substrate, CLI, unit, and Storybook checks, so the release gate fails when either parity manifest drifts.
- [x] **Prove declaration/data-island inertness in browser tests.** Cover exactly-one inline declaration template,
      `src` vs inline-template exclusivity, rejection of live declaration content, host payload capture into
      `<template data-cem-island="instance">`, removal of fallback payload before render, and proof that raw declaration
      or instance data does not affect layout, selectors, form submission, accessibility tree, or visible UI directly.
      Browser Storybook coverage now includes selector, layout, form, focus/accessibility, declaration-render isolation,
      and declaration-shape guardrails for exactly-one inline template, `src` conflict, and live-content rejection.
- [x] **Harden produced-tag lifecycle behavior.** Cover tag validation, idempotent registration, duplicate declaration
      diagnostics, observed attribute extraction, declared defaults, host overrides, undeclared attribute invalidation,
      reconnect/disconnect behavior, nested produced elements, and deterministic rerender ordering.
      Produced declaration registration is now idempotent per declaration element and duplicate tags no longer replace
      the first compiled declaration. Browser coverage asserts duplicate diagnostics, reconnect/disconnect observation,
      nested produced output, declared defaults/host overrides, undeclared attribute invalidation, and latest-wins render
      ordering.
- [x] **Close the event-to-data render loop.** Cover `slice`, `slice-event`, `slice-value`, event payload capture,
      repeated input events, stale render avoidance, multi-instance isolation, and serialization of `data`, `option`,
      slot, dataset, attribute, slice, validation, and event payload state into the instance data island.
      Event payloads now serialize event type, target/currentTarget metadata, resolved slice value, and JSON-safe custom
      detail. Browser coverage asserts repeated input events, latest-render output, multi-instance isolation, and
      snapshot serialization for host attributes, dataset, payload data/options/slots, slices, validation state, and
      event payloads.
- [x] **Make URI/module resolution policy explicit.** Cover fragment-only `src`, document-relative `url#fragment`,
      external document loading, module-map/specifier hooks, `module-url` resource slices, resolver failure diagnostics,
      source identity, and cache invalidation when resolver or scope policy changes.
      Runtime coverage now exercises fragment-only `src`, document-relative external documents keyed by declaring
      document base URI, host `resolveModuleUrl` hooks, `module-url` slice/payload snapshots, resolver failure
      diagnostics, and per-runtime cache policy changes.
- [x] **Move the legacy bridge behind the shared engine boundary.** The inventory says the `custom-element-v0`
      converter still lives in `cem-elements` TypeScript. Move or wrap that compatibility compiler so browser runtime,
      CLI validation, SSR, and package gates can consume the same CEM engine path instead of a browser-only converter.
      `lang="custom-element-v0"` now routes as a deprecated alias of the shared `legacy-xslt` engine path; the runtime no
      longer imports or branches to the browser-only `projectLegacyTemplate` path, and a unit guard locks that boundary.
- [x] **Decide scoped-style containment.** `material-parity-inventory.md` currently marks scoped template styles as
      partial because they render as page-global light-DOM `<style>` elements. Decide whether containment is required
      for the Phase 3.1 production gate. If yes, implement and test the containment model; if no, document it as
      bridge/adoption work and make the gate assert the documented behavior.
      Decision: selector containment is bridge/adoption work, not a Phase 3.1 gate requirement. `MaterialScopedStylePolicy`
      now asserts that template styles are emitted into light DOM and apply page-globally.
- [x] **Define the host processing boundary in code.** Add or harden TypeScript runtime-support contracts for
      `DataIslandSnapshot`, `RenderRevision`, template artifact refs, source-map refs, render-plan identity, patch
      frames, and patch apply results. Add structured-clone safety tests proving no live `Node`, `Event`, function,
      class instance, `Map`, `Set`, `Date`, or browser handle crosses the processing boundary.
      `assertProcessingBoundaryValue` now guards edge content/state persistence, and unit coverage proves exported
      snapshots, render plans, patch frames, edge records, and edge content reads are plain structured-clone data while
      rejecting functions, class/browser handles, `Map`, `Set`, `Date`, and event-like objects.
- [x] **Wire the first WASM-backed template-processing path.** Select the minimal Phase 3 path from
      `cem-element-wasm-proposal.md`: compile inline/external CEM-ML or DOM-parity source, run CEM-QL expressions,
      produce light-DOM render plans or patch frames, return structured diagnostics, and keep DOM patch application on
      the main-thread UI adapter.
      `processCemMlTemplate` now owns the first explicit processing boundary: it compiles/renders canonical CEM-ML
      through `cem_ql` WASM, projects slots from structured payload data, returns render-plan identity plus optional
      patch frames, and leaves DOM materialization/application in the `<cem-element>` UI adapter.
- [x] **Track source-map fidelity through render output.** Cover `author-byte-exact` for external/raw CEM sources,
      `dom-canonical` for DOM-parsed inline XML/HTML parity, and `declaration-only` fallback diagnostics. Rendered nodes
      should carry enough source-map identity for fixture assertions and devtools reporting.
      Runtime-support diagnostics now carry `sourceMapRef`, browser diagnostics preserve engine byte frames, declaration
      shape errors use `declaration-only` fallback frames, and Storybook gates assert source fidelity on DOM-parity,
      CEM-ML render nodes, text render-plan nodes, and parser/render diagnostics.
- [x] **Integrate Phase 2 verification gates.** Ensure the Phase 3.1 gate runs the parser/runtime checks named in the
      roadmap against the parity fixtures: `yarn nx run cem_ml_cli:validate-fixtures`, `yarn nx run cem_ml_cli:e2e`,
      and `yarn nx run cem_ml:bench`, plus the existing `cem-elements:verify-substrate` path.
      `cem-elements:verify` now depends on all three Phase 2 gates plus substrate, file-backed parity fixtures, unit
      tests, and Storybook parity stories.
- [x] **Add accessibility and first-paint gates for material parity.** Use
      `packages/cem-components/docs/accessibility.md` as the contract. Assert keyboard/focus/label/live-region behavior
      where applicable, and add a deterministic first-paint/performance smoke so AC-N-1-style regressions are visible
      before Phase 3.2 primitives depend on the substrate.
      Material parity Storybook gates now assert accessible names, native-role focus ownership, ARIA reference integrity,
      and a deterministic first-paint smoke that mounts all eight material parity components under a frame budget.
- [x] **Document the production-ready trigger.** Update `packages/cem-elements/README.md` and the parity inventories
      with the final command set, fixture locations, known bridge/adoption deferrals, and the exact handoff condition
      for Phase 3.5 Edge/SSR and Phase 3.6 `@epa-wg/custom-element` adoption.
      `packages/cem-elements/README.md`, the legacy inventory, and the material inventory now name
      `yarn nx run cem-elements:verify` as the browser-substrate production-ready trigger, list the fixture surfaces,
      and separate the Phase 3.5 Edge/SSR handoff from later Phase 3.6 `@epa-wg/custom-element` adoption.

### Phase 3.2 — `@epa-wg/cem-components` Primitive Production Gate

Roadmap: [`../roadmap.md` §3.2](../roadmap.md#32-primitives--epa-wgcem-components). Contract homes:
[`../docs/component-mvp.md`](../docs/component-mvp.md),
[`../packages/cem-components/docs/component-reference.md`](../packages/cem-components/docs/component-reference.md),
[`../packages/cem-components/docs/conventions.md`](../packages/cem-components/docs/conventions.md),
[`../packages/cem-components/docs/light-dom-rendering.md`](../packages/cem-components/docs/light-dom-rendering.md), and
[`../packages/cem-components/docs/accessibility.md`](../packages/cem-components/docs/accessibility.md).

Goal: make `@epa-wg/cem-components` production-ready as the first primitive declaration set built exclusively on the
`<cem-element>` substrate. The package already has primitive declarations, examples, docs, and Chromium-backed browser
coverage; the remaining work is to turn those into a durable release gate and prove the MVP workflows, states,
accessibility behavior, and token-only styling contract.

Recommended execution order:

- [x] **Baseline the current primitive package gate.** Run and record the current status for
      `yarn nx run @epa-wg/cem-components:test`, and note the browser/unit coverage surface that already exists. Fix only
      harness drift in this slice; leave primitive behavior gaps as named checklist items below.
      Baseline run on 2026-06-17: passed. The target built the required theme, `cem-elements`, and WASM dependencies,
      then ran 3 files / 11 tests: `cem-components.spec.ts` in Node, plus Chromium-backed
      `component-harness.browser.spec.ts` and `primitives.browser.spec.ts`.
- [x] **Make the primitive manifest a release gate.** Add a package-owned verifier that proves
      `CEM_COMPONENT_PRIMITIVES` exactly covers `docs/component-mvp.md`, every primitive has a CEM-ML declaration, no
      declaration depends on legacy `<custom-element>`, and install results surface registration diagnostics
      deterministically.
      `@epa-wg/cem-components:verify-primitives` now checks the TypeScript manifest against the MVP table with the
      TypeScript compiler API, rejects legacy declaration wrappers, and is included in `@epa-wg/cem-components:verify`.
      Browser coverage also asserts deterministic first-install and reinstall results.
- [x] **Promote workflow fixtures into executable coverage.** Turn the auth form, profile editor, asset browser,
      discussion thread, and settings examples into package-owned browser fixtures that assert common static and form
      flows work with no app JavaScript beyond installing the primitives.
      `packages/cem-components/tests/workflows/` now contains five declarative HTML workflow fixtures, and
      `workflows.browser.spec.ts` imports them as raw fixtures, rejects scripts/inline handlers, renders through the
      installed primitive set, and asserts the key static/form outputs.
- [x] **Harden state and ARIA behavior across the MVP matrix.** Cover disabled, loading, selected, expanded, invalid,
      required, readonly, checked, indeterminate, and empty states where applicable; assert accessible names, reference
      integrity, live-region roles, keyboard focus, and event payload behavior through the component harness.
      `primitives.ts` now reflects state attributes onto the native semantic controls, interactive primitives capture
      `slice-event` payloads, and `states.browser.spec.ts` covers action/loading/focus state, form validity and
      boolean-control state, serialized event payloads, empty fallbacks, indeterminate progress, and live regions.
- [x] **Prove the token-only styling contract.** Add a deterministic style inspection gate that rejects new
      component-specific color/spacing literals and verifies primitive styles resolve through CEM theme token families.
      `@epa-wg/cem-components:verify-style-contract` now depends on `@epa-wg/cem-theme:build:tokens`, checks MVP
      token-family names against generated `cem.tokens.json` and combined CSS, rejects inline component styles and raw
      component CSS color/spacing literals, and is included in the aggregate component verify target.
- [x] **Document the primitive production-ready trigger.** Update `packages/cem-components/README.md` and
      `packages/cem-components/docs/component-reference.md` with the final command set, fixture locations, known
      deferrals, and the exact handoff condition for Phase 4 component expansion.
      README and component reference now name `yarn nx run @epa-wg/cem-components:verify` as the Phase 3.2 trigger,
      list the primitive, style, browser, state, and workflow fixture surfaces, identify Edge/SSR and
      `@epa-wg/custom-element` adoption as later phases, and define the Phase 4 handoff condition.

### Phase 3.5 — Edge/SSR Processing Follow-Up

Roadmap: [`../roadmap.md` §Phase 3.5](../roadmap.md#phase-35---edgessr-processing-follow-up). Design home:
[`cem-element-design.md`](cem-element-design.md) §4.1, §4.2, and §4.3.

Goal: prove that the `<cem-element>` processing layer can run outside the browser UI adapter through the same
serializable boundary. Browser-local worker and main-thread fallback behavior remain the reference semantics; this
phase adds fixtures and host helpers for SSR and edge processing without changing declaration syntax or DOM ownership.

Recommended execution order:

- [x] **Baseline the current processing-boundary surface.** Run and record the current status for
      `yarn nx run cem-elements:verify`, `yarn nx run @epa-wg/cem-components:verify`, and the narrow runtime-support
      tests that cover `DataIslandSnapshot`, `RenderRevision`, render plans, patch frames, and structured-clone
      assertions. Fix only harness drift in this slice; leave Edge/SSR gaps as named checklist items below.
      Baseline run on 2026-06-18: both aggregate commands passed. `cem-elements:verify` ran the substrate roundtrip,
      file-backed parity fixture verifiers, unit runtime-support tests, Storybook runtime stories, CLI fixture/e2e
      gates, and `cem_ml:bench`; `@epa-wg/cem-components:verify` ran primitive manifest, style contract, and browser
      workflow/state/primitive tests. Existing narrow coverage includes `processing-boundary.spec.ts` for
      structured-clone contracts and Storybook stories for SSR hydration, edge patch frames, privacy export, and hybrid
      render-state storage.
- [x] **Extract a host-neutral processing fixture API.** Add package-owned helpers that can render from serialized
      template source plus `DataIslandSnapshot` without constructing live DOM, custom elements, events, focus state, or
      form controls. The helper should expose the existing browser worker/main-thread processing result shape so Edge,
      SSR, and browser tests assert the same artifact identity, diagnostics, source-map mode, render-plan identity, and
      patch-frame contracts.
      `packages/cem-elements/src/lib/projection.ts` already exposes the host-neutral projection and edge helpers:
      `projectTemplate`, `diffRenderPlansToPatchFrames`, `advanceEdgeRenderState`, and
      `projectAndAdvanceEdgeRenderState`. `processing-boundary.spec.ts` proves their snapshots, render plans, patch
      frames, edge records, and content reads remain structured-clone-safe data.
- [x] **Add the SSR initial-render fixture.** Build a deterministic fixture that emits initial light DOM plus hydration
      metadata from a serialized `DataIslandSnapshot`: direct child
      `<script type="application/json" data-cem-hydration="snapshot">`, direct instance data-island `<template>`, and
      normal `<!--cem-render-start-->` / `<!--cem-render-end-->` boundaries. Assert first connect preserves matching
      server-rendered DOM and restores revision/render-plan identity before normal client invalidation resumes.
      `SsrHydrationFromSerializedSnapshot` covers serialized snapshot metadata, data island payload preservation,
      render boundary preservation, retained template artifact/data revision identity, visible slot projection, and
      client-side invalidation after hydration.
- [x] **Broaden hydration mismatch diagnostics.** Current coverage rejects unsupported newer snapshot schema versions
      through `SsrHydrationRejectsUnsupportedSnapshotVersion`. Add stale template artifact identity, stale
      `RenderRevision`, mismatched source-map mode, missing hydration snapshot, malformed JSON, missing render
      boundaries, and retained render-plan mismatch diagnostics. Fail closed with structured diagnostics and
      deterministic client fallback behavior.
      Additional coverage landed for partial SSR markup: metadata without render boundaries reports
      `cem-element.hydration_boundaries_missing`, and render boundaries without metadata reports
      `cem-element.hydration_metadata_missing`. Snapshot parse failures now distinguish missing JSON
      (`cem-element.hydration_snapshot_missing`), invalid JSON shape (`cem-element.hydration_snapshot_invalid`), and
      malformed JSON (`cem-element.hydration_json_invalid`). Retained render roots now reject missing template
      artifact/data revision identity, stale template artifacts, and stale data revisions before falling back to a
      fresh client render. Source-map-mode mismatch is split into a follow-up contract extension because the current
      `DataIslandSnapshot` schema does not serialize source-map mode.
- [ ] **Define the hydration source-map-mode contract extension.** Add a source-map mode/fidelity field to the
      serialized hydration contract, define compatibility rules for mismatches, and add an SSR rejection fixture once
      the field exists in `DataIslandSnapshot`.
- [x] **Add the pure edge-processing fixture.** Feed serialized template source, previous render-plan identity, and a
      policy-sanitized `DataIslandSnapshot` into the pure render-plan projection path. Assert
      `diffRenderPlansToPatchFrames(previous, next)` emits `begin` / batched `ops` / `commit` frames without live DOM
      access.
      `EdgePatchFramesFromSerializedSnapshot` covers serialized previous/next snapshots, stable render-node-id text
      patching, `begin` / `ops` / `commit` frame order, transaction identity, and retained next render-plan identity.
- [x] **Broaden edge diff boundary coverage.** Current coverage proves stable text patching, store-backed patch frames,
      first-render `replaceScope`, missing/corrupt previous render-plan failures, and render-revision mismatch
      failures. Add explicit coverage for stable attribute patches, template changes, root-count changes, unsupported
      structural deltas, and target mismatches falling back to constrained `replaceScope` frames.
      `EdgePatchFramesFromSerializedSnapshot` now covers stable attribute `setAttribute` patches, template artifact
      changes, root-count changes, unsupported structural replacements, and produced-tag target mismatches. Fallback
      cases emit constrained `replaceScope` frames with `fallback` reasons rather than relying on live DOM access.
- [x] **Implement fail-closed data export policy fixtures.** Prove that snapshots are local-only by default and that
      denied fields are omitted or redacted before leaving the browser context. Cover sensitive fields, transient input
      composition, focus/selection state, raw browser events, credentials, and policy-denied payload data.
      `BrowserToEdgeSnapshotPrivacyPolicy` covers default omission, allowed host attributes, redacted payload and
      validation state, omitted dataset/slices/event payloads, policy stamps, and detached exported snapshots.
- [x] **Decide and document first render-state storage.** Choose the Phase 3.5 storage model for edge processing:
      content-addressed cache only, revisioned KV/document records, or the hybrid model described in
      `cem-element-design.md`. Add fixture coverage for content-addressed artifacts, revision pointer records,
      scope/privacy policy stamps, and stale-write rejection.
      `EdgeRenderStateHybridStorageModel` locks the hybrid model:
      `content-addressed-cache-with-revision-pointer-v1`. It covers content-addressed render plans, template
      artifacts, sanitized snapshots, rendered HTML, revision pointer records, policy stamps, ETag stale-write
      rejection, missing/corrupt content failures, and store-backed edge advancement.
- [x] **Wire the Phase 3.5 release gate.** Add an aggregate Nx target that runs the SSR fixture, edge fixture,
      privacy/export policy fixtures, and existing browser substrate verification. Document the final command set and
      the handoff condition for Phase 3.6 `@epa-wg/custom-element` adoption.
      `yarn nx run cem-elements:verify-edge-ssr` now runs the Phase 3.5 aggregate through `verify-substrate`,
      `test:unit`, and `test`; `cem-elements:verify` depends on that gate. The command set, coverage, and Phase 3.6
      handoff condition are recorded in [`cem-elements-edge-ssr-gate.md`](cem-elements-edge-ssr-gate.md).

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
