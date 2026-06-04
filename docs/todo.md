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
        WASM/runtime-support boundary into the same render-plan shape. Canonical-subset CEM-ML now renders through
        the `cem_ql` WASM boundary (C2.1–C2.3) with functional `/datadom` data-document selection + `??` (C2.4) and
        `cem:if`/`cem:choose` conditionals + declarative slot projection (C2.5); the C1.5 TypeScript adapter remains
        the fallback for bespoke constructs and WASM-unavailable hosts. Remaining: the rest of C2.5
        (`<data>`/`<option>`, `module-url`) and the verification gate before retiring the C1.5 fallback (C2.6).
    - [x] C2.1: Add a `cem_ql` data-bound render boundary for canonical CEM-ML templates. Split the production
          shape into `compileTemplate(source, options) -> TemplateArtifact` and
          `renderTemplate(artifact, DataIslandSnapshot) -> RenderPlan`; internally this may reuse
          `CompileContext.policy_bindings`, but the public boundary names the browser data as host/data bindings.
          Start with `$variable` content and AVT interpolation only, preserving structured source-map frames. Landed
          as the Rust-side compile-once/render-many boundary in
          [`packages/cem_ql/src/render.rs`](../packages/cem_ql/src/render.rs) with coverage in
          [`packages/cem_ql/tests/template_render.rs`](../packages/cem_ql/tests/template_render.rs); the browser
          snapshot/WASM transport wrapper remains C2.2/C2.3.
    - [x] C2.2: Add the `cem_ql` WASM entrypoint and build tooling. Export version plus compile/render functions,
          pin the wasm-bindgen toolchain, and make the Nx `cem_ql:build:wasm` target emit JS bindings usable by
          Vite/Storybook. Landed with wasm exports for `version`, `compileTemplate`, `renderTemplate`,
          `renderTemplateSource`, and `disposeTemplate`; `nx run cem_ql:build:wasm` now emits
          `packages/cem_ql/dist/wasm/{cem_ql.js,cem_ql.d.ts,cem_ql_bg.wasm}` through the pinned
          `wasm-bindgen-cli 0.2.122` build script.
    - [x] C2.3: Replace the C1.5 TypeScript CEM-ML adapter in `@epa-wg/cem-elements` with an async runtime-support
          call into the `cem_ql` WASM render boundary, keeping `materializeRenderPlan`, diagnostics, frame
          attributes, and the TS adapter as a temporary fallback only. Landed as the extraction-ready host
          runtime-support layer
          [`packages/cem-elements/src/lib/internal/runtime-support/cem-ql-render.ts`](../packages/cem-elements/src/lib/internal/runtime-support/cem-ql-render.ts)
          (lazy main-thread WASM init, async `renderCemMlTemplate(source, data) -> RenderPlanNode[]+diagnostics`;
          worker-backed primary path deferred). `cem-elements.ts` routes canonical-subset CEM-ML (`{$x}` content,
          `{…}` AVT, no `<attribute>`/`<slice>` decls, no `${}` text) through the WASM boundary with a per-instance
          render token, reusing `materializeRenderPlan` and the slice-E frame attributes; the C1.5 adapter
          ([`cem-ml-template.ts`](../packages/cem-elements/src/lib/runtime-support/cem-ml-template.ts)) is now the
          fallback only (declaration diagnostics, declared-attribute/slice extraction, bespoke constructs deferred to
          C2.4/C2.5, and WASM-unavailable hosts). `cem_ql` `render.rs` now carries real per-node author-byte-exact
          source frames (was whole-document) and `api/wasm.rs` emits per-node `byteOffset`; `build:wasm` writes a
          `{"type":"module"}` ESM marker into `dist/wasm`. Proven by new Storybook stories
          `CemQlWasmRenderBoundary` (direct render-plan/frames/diagnostics mapping) and `CemQlWasmRenderLoopUpgrade`
          (lifecycle upgrade) in
          [`cem-elements.stories.ts`](../packages/cem-elements/src/lib/cem-elements.stories.ts); all 30 stories green.
    - [x] C2.4: Add the data-document evaluation slice for XPath, `select`, `/datadom`, and `??` by extending the
          evaluator context from the serialized `DataIslandSnapshot`. Per direction, this is **functional parity in
          cem-ql, not an XPath engine**: the `/datadom` data document is exposed as a cem-ql `Record` and navigated
          with native record/pipeline access (`datadom.attributes.<name>`, the functional equivalent of the legacy
          `/datadom/attributes/<name>` selection), so `select` is just a cem-ql expression over it. Landed in
          [`packages/cem_ql/src/render.rs`](../packages/cem_ql/src/render.rs) (`build_data_document` seeds
          `datadom.attributes.*` from the host bindings and binds `datadom`; flows through the WASM
          `renderTemplateSource` path automatically) and the `??` null/empty-sequence coalescing operator across
          [`lexer.rs`](../packages/cem_ql/src/lexer.rs) (`Coalesce` token), [`parser/pratt.rs`](../packages/cem_ql/src/parser/pratt.rs)
          (`PREC_COALESCE`), [`parser.rs`](../packages/cem_ql/src/parser.rs) (`BinaryOp::Coalesce`),
          [`types.rs`](../packages/cem_ql/src/types.rs) (union type), and [`eval.rs`](../packages/cem_ql/src/eval.rs)
          (short-circuit). Coverage: functional-parity tests in
          [`template_render.rs`](../packages/cem_ql/tests/template_render.rs) (select, coalesce-absent/present/chained)
          and cem-elements stories `CemQlDataDocumentBoundary` (direct `??`+`datadom` mapping) and
          `CemQlDataDocumentRenderLoop` (lifecycle select) in
          [`cem-elements.stories.ts`](../packages/cem-elements/src/lib/cem-elements.stories.ts); 32 stories green.
          Note: `datadom.dataset`/`datadom.slices` subtrees and structured snapshot passing are a natural extension;
          the C1.5 TS adapter still can't parse canonical cem-ql (`??` trips its declaration parse), resolved at C2.6.
    - [ ] C2.5: Add material-parity constructs that depend on full data-bound rendering: `if`/`choose`/`when`,
          `<data>`, `<option>`, `module-url`, and declarative slots.
      - [x] Conditional constructs `cem:if` / `cem:choose` / `cem:when` / `cem:otherwise` (also accepting the
            bare legacy `if`/`choose`/`when`/`otherwise` spellings): recognized in the `cem_ql` template compiler,
            evaluating the `@test` cem-ql expression (built on the C2.4 functional `/datadom`) to an effective
            boolean and flattening the selected branch's children into the render plan (the conditional emits no
            wrapper element). Landed in [`packages/cem_ql/src/render.rs`](../packages/cem_ql/src/render.rs)
            (`TemplateNode::If`/`Choose`, `render_into` flattening, `test_is_truthy`) with coverage in
            [`template_render.rs`](../packages/cem_ql/tests/template_render.rs) and the cem-elements
            `CemQlConditionalRenderLoop` story; flows through the WASM boundary automatically. 33 stories green.
        - [x] Review finding: malformed conditional structure must diagnose instead of silently rendering false/empty.
              Add compile diagnostics for missing `@test` on `cem:if`/`cem:when`, `@test` on `cem:otherwise`, non-branch
              direct children under `cem:choose`, and multiple `cem:otherwise` branches; keep valid conditional output
              unchanged. Landed in [`packages/cem_ql/src/render.rs`](../packages/cem_ql/src/render.rs) with regression
              coverage in [`template_render.rs`](../packages/cem_ql/tests/template_render.rs).
      - [ ] `<data>` / `<option>` instance payloads — serialize the data-island payload into the snapshot and
            expose it under `/datadom` (needs snapshot-payload serialization).
        - [ ] Review finding: the data-document contract should be unified before adding `<data>`/`<option>`.
              Define `DataIslandSnapshot -> TemplateData` mapping for `attributes`, `dataset`, `payload`, `slots`,
              `slices`, `validationState`, and `eventPayloads`, then make both the WASM path and TS fallback consume
              that same shape.
      - [ ] `module-url` resource slices — async resource resolution + slice exposure (overlaps the deferred
            `src`-loading slice and Phase 3.5).
      - [x] Declarative slot projection — project the produced instance's payload into `<slot>` positions in the
            light DOM (named + unnamed, slot default content as fallback). Landed as a materialize-time step in
            [`packages/cem-elements/src/lib/cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts)
            (`projectSlots`/`collectSlotPayload`, applied on both the sync and WASM render paths): each `<slot>` is
            replaced by clones of the data-island payload assigned to it (named slots match `slot="<name>"`; the
            default slot takes unslotted payload plus non-empty text) or its own fallback children when unfilled.
            Cloning keeps the inert island as the durable source across rerenders. Coverage: the
            `SlotProjectionRenderLoop` story; 34 stories green.
        - [ ] Review finding: slot projection currently reads live island DOM after WASM materialization, so it is not
              reproducible in worker/SSR/edge hosts from the serialized snapshot. Started: slot payload serialization
              moved into `DataIslandSnapshot`, and browser projection now reads `DataIslandSnapshot.payload` instead
              of the live data-island template. Remaining: expose slottables under `datadom.payload`/`datadom.slots`
              and lower `<slot>` from serialized payload before or during render-plan materialization; keep DOM
              projection only as a temporary browser fallback while the render-plan slot node is designed.
        - [ ] Add C2.5 edge-case coverage: nested conditionals, repeated same-name slots, mixed unslotted elements plus
              text ordering, slot fallback cloning, rerender after payload mutation, and serialized-payload projection
              on the WASM path. Started: mixed default-slot text/element ordering, fallback cloning, and payload
              mutation rerender are covered by `SlotProjectionRenderLoop`.
    - [ ] C2.6: Wire the verification gate through `cem_ml_cli:validate-fixtures`, `cem_ml_cli:e2e`, and Storybook
          parity stories before retiring the C1.5 fallback.
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
- [x] Add Storybook browser stories proving data-island isolation: declaration and instance template contents do not
      affect layout, selectors, form submission, accessibility, or visible UI directly. Landed in
      [`packages/cem-elements/src/lib/data-island-isolation.stories.ts`](../packages/cem-elements/src/lib/data-island-isolation.stories.ts):
      selectors do not pierce the island, bulky island content does not inflate layout, island controls stay out of
      form submission and the accessibility/focus tree, and the declaration host renders no visible content of its own.
- [ ] Build the legacy parity feature inventory from the old `@epa-wg/custom-element` suite and docs
      (`~/aWork/custom-element/docs/{attributes,rendering}.md` plus legacy test files). Convert every in-scope
      behavior into a named `<cem-element>` Storybook parity story; record intentional CEM-ML/CEM-QL replacements as
      migration decisions.
- [ ] Land material parity stories for every component in `~/aWork/custom-element-dist/src/material/` (action,
      autocomplete, badge, dropdown, icon, icon-link, input, menu).
- [x] Build a material parity inventory from `~/aWork/custom-element-dist/src/material/components/*.html` covering
      local/external `src`, hidden declarations, nested custom elements, declarative slots, scoped styles,
      `attribute select`, `if`/`choose` bridge constructs, namespaced `xhtml:*` elements, boolean attribute helper
      semantics, `module-url` resource slices, `data`/`option` payloads, slice events, and `slice-value`. Landed in
      [`packages/cem-elements/docs/material-parity-inventory.md`](../packages/cem-elements/docs/material-parity-inventory.md):
      per-component feature usage plus a 22-row feature→runtime-support matrix. Key finding — external/local `src`
      declaration loading is the hard blocker (all 8 components compose via `src` imports), and the
      conditional/expression/slot/`data`-payload features gate behind slice C2 (cem-ml/cem-ql).
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
