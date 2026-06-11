# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).
Each item names the AC reference and design home so the closing change ship with a citation.

## Active — Evolutionary Architecture of the Authoring/Rendering Model

Design home: [`content-type-switch.md`](content-type-switch.md) (BRD). The goal is an
evolutionary architecture in which **surface syntax**, **underlying logic**, and **content
model** evolve independently via guided, incremental change; content-type/syntax/namespace
switching plus versioning are the seams. Legacy XSLT coexistence is the current-focus use case,
not the goal. These open questions must be resolved before the model is committed. Adopting
**semantic versioning across all axes** (BRD §6.5) resolves OQ-1 and OQ-4 and narrows
OQ-2/5/6/7; separating the two **switching surfaces** (BRD §6.8 — host `<template>`/`<script>`
ingestion vs. interior namespace-scoped selection) resolves OQ-3's architecture (residual:
AC-P-6 detailing) and narrows OQ-8 (now resolved to **boundary-contract scope**, BRD §5).
Critical review added two further commitment blockers:
namespace metadata authority/trust/cache identity (OQ-9) and external legacy-version mapping
for non-SemVer standards such as XSLT (OQ-10).

- [x] OQ-1 Evolution axes: ratify the independent dimensions — surface syntax, underlying
      logic/semantics, and content/data model — and state which compatibility rules apply per
      axis. (BRD §6.1; design: [`cem-ml-syntax.md`](cem-ml-syntax.md)) Resolved: SemVer-per-axis
      in BRD §6.5 (BR-VC-5) ratifies the axes and assigns each its own SemVer compatibility rule.
- [x] OQ-2 Fitness functions. Resolved: **CI-blocking + FFDD** (BRD BR-FF-1/2/3) — all fitness
      functions are blocking pipeline gates on every change, and every governed-contract change
      adds/extends its guard. Catalog (8), most reusing existing gates:
  - [x] FF-1 Backward-render: prior-generation fixtures still render.
        (`cem_ml_cli:validate-fixtures`, `cem-elements:verify`) — **active** in the FF-gate map.
  - [x] FF-2 Negotiation determinism: same-MAJOR loads, unsupported MAJOR rejects, per axis.
        (`cem_ml_cli:e2e`, `cem_ml:test`; AC-P-V-5/AC-F-8) — **active** (evidence
        `version-negotiation/core-major-forgiving.cem` + `version_negotiation_fixtures.rs`: forgiving
        same-MAJOR load + unsupported-MAJOR / future-MINOR reject; core-namespace axis at Tier A).
  - [x] FF-3 Isolation: no region interpreted by another content type's processor.
        (`cem_ml_cli:e2e`; AC-P-V-3) — **active** (evidence `schema-scoping/sibling-isolation.cem`).
  - [x] FF-4 Mode-disposition: unknown optional → ignore/degrade/reject per app/build-SSR/dev;
        must-understand rejects. (`cem-elements:test:unit`/`cem-elements:test`; BR-VC-9 + AC-P-6.7) —
        **active.** Scoped (per the framing reconciliation) to the BR-VC-9 run-mode disposition over
        unknown OPTIONAL features per governed contract: `cem-elements/src/lib/disposition.ts`
        (RunMode app/build-SSR/dev × presentation/data-security class → reject/degrade, BR-VC-8
        must-understand override, `ingestContractVersion`). Applied at **both** data/security ingest
        seams — snapshot hydration (`adoptServerRenderedInstance`) and edge-render-state
        (`readEdgeRenderStateContents`) — with a configurable `runMode` (default `application`).
        Evidence: `disposition.spec.ts` + `projection.disposition.spec.ts` (56 unit tests) +
        the `SsrHydrationRejectsUnsupportedSnapshotVersion` Storybook story (61 total). The
        parser-side AC-P-V-6 is split out below.
  - [x] AC-P-V-6 (parser) unresolved-namespace disposition — **done** (cem_ml). The literal
        AC-P-V-6 verifier: a `cem_ml` region whose namespace resolves to no metadata, no schema, and
        no rule yields reject/allow/ignore strictly per the effective scope policy + run mode
        (AC-P-6.7's parser subject). Decision-core `packages/cem_ml/src/schema/disposition.rs`
        (RunMode × NamespaceClass → reject/allow/ignore + `is_unresolved_namespace` predicate +
        `KNOWN_NAMESPACES`; 10 unit tests). Wired into the schema machine
        (`apply_unresolved_namespace_disposition` at element close, run mode via
        `CemSchemaMachine::with_run_mode`, default `Application`): emits
        `cem.schema.unresolved_namespace` (reject, Error) / `…_allowed` / `…_ignored` (Info report
        events). Verified by `tests/namespace_disposition_fixtures.rs` +
        `examples/cem-ml/namespace-disposition/unknown-namespace.cem` (4 tests across modes; runs
        under `cem_ml:test`). Tier A realizes the decision as a diagnostic — deeper drop/foreign-DOM
        materialization and the scope-policy override source are future work.
  - [x] FF-5 Removal gate: zero in-repo consumers of a deprecated form + external window.
        Landed: registry `tools/fitness/deprecated-forms.json`, shared `tools/fitness/lib.mjs`,
        scanner `tools/scripts/ff-deprecated-form-scan.mjs`, Nx target
        `@epa-wg/cem:fitness-removal-scan` (green; forbidden-form fail path verified), and wired
        into the CI gate (`.github/workflows/ci.yml` "Run fitness-function gates", always-run).
        Forbidden XSLT-engine patterns + `custom-element-v0`/`cem-ml-v0` deprecated forms tracked;
        the `verify-package-baseline.mjs` dist guard is retained (FF-5 covers source-wide).
        (scope: [`fitness-functions.md`](fitness-functions.md))
  - [x] FF-6 SemVer-presence: every governed contract declares a version axis. Landed: registry
        `tools/fitness/governed-contracts.json`, scanner `tools/scripts/ff-semver-presence.mjs`,
        Nx target `@epa-wg/cem:fitness-semver-presence` (green; fail path verified), CI-wired
        alongside FF-5. All nine `required` contracts resolve real SemVers (custom-element 0.0.39,
        cem-elements 0.0.14, cem_ml/cem_ql/cem_ml_cli 0.1.0, data-snapshot/token-outputs/
        patch-transport/edge-render-state 1.0.0); **zero `pending-version` gaps remain — FF-6 fully
        closed.** (scope: [`fitness-functions.md`](fitness-functions.md))
  - [x] FF-7 XSLT capability-gating: unsupported XSLT version rejects; region isolated +
        version-pinned across CEM-ML MAJOR. (`cem_ml:test`; AC-P-V-4/V-7) — **active.** Scoped to
        AC-P-6.8 XSLT region **dispatch** (the gate's guards). Decision-core
        `packages/cem_ml/src/schema/xslt.rs` (`XSL_NAMESPACE`, `parse_xslt_version`, version-pinning
        `resolve_xslt_dispatch` against a CEM-owned `ADAPTER_LINE` independent of the CEM-ML core
        version → AC-P-V-4; `xslt_region_outcome`). Machine wiring (`with_xslt_dispatch` opt-in,
        `emit_xslt_dispatch`, region-isolation guard): an opted-in `xsl:` region opens an isolated,
        version-pinned handoff (descendants not interpreted), missing/malformed `@version` rejects,
        and without opt-in the region falls to the AC-P-V-6 unknown-namespace default (AC-P-V-7).
        Verified by `tests/xslt_dispatch_fixtures.rs` + `examples/cem-ml/xslt-dispatch/*.cem`. XSLT
        **execution** capability-gating (AC-P-6.9 — running a transform; rejecting an unimplemented
        executable version) stays a deferred Tier-C wishlist, not asserted by this gate.
  - [x] FF-8 Source-map continuity across dispatch boundaries.
        (`cem_ml_cli:validate-fixtures`; AC-P-V-2) — **active** (evidence
        `namespace-rebinding/default-html-svg-html.cem`).
- [x] OQ-3 Switching granularity. Architecture resolved (BRD §6.8): the
      whole-`<template>`/`<script>` `lang`/`type` routing is the HTML→CEM-ML host-ingestion
      boundary (owned by the HTML parser + cem-element, an instance of the BR-CT-4 content-type
      handoff); interior switching is namespace-scoped, selected directly or indirectly from
      resolved namespace metadata — the two layers compose rather than compete. Resolved: the
      detailing landed in [`cem-ml-ac.md`](cem-ml-ac.md) as AC-P-6.1–6.9 (interior indirect path)
      plus the embedded `xsl:` content type (AC-P-6.8/6.9) and the G-NVDL-CORE/FULL gate split.
  - [x] Draft the AC-P-6 promotion. Landed in
        [`cem-ml-ac-p6-nvdl-promotion.md`](cem-ml-ac-p6-nvdl-promotion.md): expands AC-P-6 into
        AC-P-6.1–6.9 (namespace-metadata dispatch, direct/indirect selection, host-vs-interior
        two-layer boundary, isolation, per-namespace SemVer, Layer-5 handoff/source-map
        continuity, unknown-namespace policy) plus the embedded `xsl:` content type, a proposed
        G-NVDL-CORE Tier-B split vs G-NVDL-FULL Tier C, and verification AC-P-V-2..V-8.
  - [x] Decide D-1..D-6: all resolved — D-1 G-NVDL-CORE Tier-B split; D-2 mode-selected (OQ-6);
        D-3 engine-implemented XSLT 3/4 (OQ-7); D-4 composed metadata chain (OQ-9); D-5
        refine-only direct/indirect; D-6 native-request + adapter-SemVer (OQ-10).
  - [x] Fold the draft into [`cem-ml-ac.md`](cem-ml-ac.md). Landed: AC-P-6.1–6.9 + the
        direct/indirect conflict rule (§1 Parser), verification AC-P-V-2..V-8, the §16.4 split
        into G-NVDL-CORE (Tier B) / G-NVDL-FULL (Tier C), and the §16.1 graph + Tier B/C
        descriptions + gate-id-list updates. The promotion draft is retained as rationale.
- [x] OQ-4 Version-negotiation policy: ratify the cross-axis compatibility policy — forgiving
      vs strict boundaries, who decides, how incompatible majors degrade, and how multiple
      coexisting versions of one content kind behave. (BRD §6.5; [`cem-ml-ac.md`](cem-ml-ac.md)
      AC-F-8, AC-V-9..V-13) Resolved: BRD §6.5 (BR-VC-6) — same-MAJOR forgiving / cross-MAJOR
      strict, declared by the document and decided by the processor, with per-region coexistence.
- [x] OQ-5 Migration pattern: adopt parallel-change (expand → migrate → contract). Resolved:
      **required + gated** (BRD BR-EV-5/7) — parallel-change is mandatory (shall) for breaking
      changes to governed contracts; expand-phase additions stay optional (not must-understand)
      until contract; the contract/removal phase is gated on a fitness function proving zero
      in-repo consumers plus a published ≥1-MINOR deprecation window for external consumers. The
      removal gate is itself an OQ-2 fitness function.
- [x] OQ-6 Forward compatibility: how a processor handles an unknown newer feature. Resolved:
      **mode-selected disposition** (BRD §6.5 BR-VC-8/9) — the engine supports all three, chosen
      by run mode: application run = per-contract (tolerant on presentation, strict on
      data/security); build/SSR = strict everywhere; dev/debug = tolerant everywhere.
      must-understand ⇒ reject in every mode; degrade to a producer fallback when present, else
      ignore/reject per mode. This also resolves **D-2** (AC-P-6.7 unknown-namespace default
      follows the same mode model). Residual = the must-understand marker mechanism (lands in the
      AC-P-6 promotion, not the BRD).
- [x] OQ-7 Legacy retirement criteria. Resolved (reframed): **only the deprecated browser-native
      XSLT 1.0 dependency retires**, via the BR-EV-5/7 parallel-change + gated removal. XSLT as a
      language does **not** retire — XSLT 3.0/4.0 is a supported peer language **implemented by
      the CEM-ML engine** (BR-CO-5), capability-gated and version-negotiated (BR-VC-6/8); CEM-ML
      stays primary. This also resolves **D-3** (execution = engine-implemented XSLT 3/4; the
      browser-1.0 bridge is the retiring escape). Residual = cem-theme CSS-generator conversion
      (Phase 3.6 `verify:phase13`) and engine XSLT 3/4 coverage are capability/roadmap work.
      (BRD §6.7; [`custom-element-template-migration-options.md`](custom-element-template-migration-options.md))
- [x] OQ-8 Scope of evolution: decide which dimensions the model governs. Resolved:
      **boundary-contract scope** (BRD §5, BR-EV-6) — govern every contract that crosses a
      process/trust boundary, persists across versions, or is externally consumed; mechanisms
      and in-memory single-process internals are out. Host-surface ingestion
      (`<template>`/`<script>`) is an adapter boundary, not a governed dimension (BRD §6.8).
      Residual: the two currently un-versioned governed contracts — the data/snapshot
      (`datadom`) contract and the design-token outputs — still need their own SemVer axis,
      tracked with the OQ-2 fitness-function worklist.
- [x] OQ-9 Namespace metadata authority. Resolved (=D-4): **composed, local-first chain** —
      inline descriptor → workspace registry → package manifests → external registry (explicit
      opt-in, G-EXT/AC-A-6 gated). Offline-deterministic by default; pinned via committed
      registry + lockfile; resolved `{contentType, schemaUri, schemaVersion}` + source enters
      AC-CC-1/AC-CC-3; reuses existing module-map resolver hooks. (draft:
      [`cem-ml-ac-p6-nvdl-promotion.md`](cem-ml-ac-p6-nvdl-promotion.md) D-4)
- [x] OQ-10 External-standard version mapping. Resolved (=D-6): **native request + adapter
      SemVer** — the document's native version (`xsl:stylesheet/@version`) is the requested
      version, resolved against a CEM-owned XSLT adapter SemVer line tracking the engine's
      implemented profile (BR-CO-5); the version-stable namespace URI is not a version source;
      unimplemented versions reject deterministically (BR-VC-8). (draft:
      [`cem-ml-ac-p6-nvdl-promotion.md`](cem-ml-ac-p6-nvdl-promotion.md) D-6)

### What's left — execution (all OQs and decisions resolved)

The BRD ([`content-type-switch.md`](content-type-switch.md)) and the AC-P-6 promotion are folded
into [`cem-ml-ac.md`](cem-ml-ac.md); the remaining items are implementation, with no open decisions:

- [x] Fold the AC-P-6 promotion into [`cem-ml-ac.md`](cem-ml-ac.md) — AC-P-6.1–6.9, AC-P-V-2..V-8,
      the §16.4 G-NVDL-CORE/FULL split, and the §16.1 graph + tier/gate-list updates.
- [x] Implement the eight fitness functions FF-1..FF-8 (OQ-2) as CI-blocking gates. **All 8 active**
      (`@epa-wg/cem:fitness-gate-map`: 8 active / 0 tracked / 0 errors). Net-new scanners FF-5
      (removal-scan) + FF-6 (SemVer-presence); the **FF-gate map** framework
      (`tools/fitness/fitness-gates.json` + `tools/scripts/ff-gate-run.mjs` + Nx
      `@epa-wg/cem:fitness-gate-map`, CI-wired) names all 8 FFs and verifies the FF→backing→CI
      mapping. CI invokes `cem_ml_cli:validate-fixtures` + `cem_ml_cli:e2e` and runs `cem_ml:test` +
      `cem-elements:test{,:unit}` via `nx affected -t test test:unit`. Per-FF acceptance landed
      slice-by-slice: FF-2 AC-P-V-5 version-negotiation corpus; FF-4 the BR-VC-9 / AC-P-6.7 run-mode
      disposition over contract optional features (`cem-elements/src/lib/disposition.ts`, both
      data/security ingest seams, 56 unit tests + a hydration-reject story); FF-7 AC-P-6.8 XSLT region
      **dispatch** (`cem_ml/src/schema/xslt.rs` + machine wiring; isolated version-pinned handoff on
      opt-in, AC-P-V-6 default without — AC-P-V-4/V-7). **Deferred (Tier-C wishlist, not gated):**
      AC-P-6.9 XSLT *execution* capability-gating. **Also landed:** the parser-side **AC-P-V-6**
      unresolved-namespace disposition (cem_ml), the literal verifier FF-4's contract disposition does
      not cover (see the FF-4 catalog entry above).
- [x] Add a SemVer axis to the two un-versioned governed contracts. Landed: `SNAPSHOT_SCHEMA_VERSION`
      = 1.0.0 on `DataIslandSnapshot` (`cem-elements.ts` — optional/additive expand-phase field per
      BR-EV-5, stamped at `createSnapshot` and carried through edge export) and `TOKENS_SCHEMA_VERSION`
      = 1.0.0 stamped as `$version` on `cem.tokens.json` / `cem.voice.tokens.json` (`export-tokens.mjs`);
      both flipped to `required` in FF-6 (green; typecheck + 59 Storybook tests pass). FF-6 then
      surfaced two further gaps — `patch-transport` and `edge-render-state` — now **also landed**
      (`RENDER_ENGINE_VERSION` on the `begin` `PatchFrame`; `EDGE_RENDER_STATE_VERSION` as
      `EdgeRenderStateRecord.schemaVersion`, part of the etag identity) and promoted to `required`.
      FF-6 is now fully closed: 9/9 `required`, 0 pending.
- [x] Convert the `cem-theme` CSS generators to CEM-ML+CEM-QL (Option B) and rerun
      `@epa-wg/cem-theme:verify:phase13` — **DONE: all 10 generators converted, `verify:phase13`
      fully green** (manifest coverage + browser capture for 10 specs, theme-mode resolution ×5,
      forced-colors, reduced-motion, shape, a11y/contrast, cross-spec). The live browser-XSLT-1.0
      runtime is retired from the generators. Landed slice-by-slice per the migration doc:
  - [x] Slice 1 — cem-ql `for-each` iteration. Landed in
        [`render.rs`](../packages/cem_ql/src/render.rs): `TemplateNode::ForEach`, `compile_for_each`
        (scoped compile-time `@as` declaration), render-time per-item rebind of `$as` via
        `policy_bindings`, and the `cem.ql.render.for_each_missing_select` diagnostic. Accepts
        `cem:for-each`/bare `for-each` with `@select` + optional `@as` (default `item`); flattens
        like the conditionals and flows through the WASM boundary automatically (no API change).
        Coverage: 3 tests in [`template_render.rs`](../packages/cem_ql/tests/template_render.rs)
        (atomic iteration, record-row `$row.field` access, missing-select diagnostic); full cem-ql
        suite green.
  - [x] Slice 2 — cem-ql functional navigation primitives (the "navigable data-document" engine
        side). Finding: XPath axis evaluation is **deliberately unwired** in cem-ql
        ([`eval.rs`](../packages/cem_ql/src/eval.rs): "host AST axis evaluation is not wired yet"),
        consistent with the C2.4 "functional parity, not an XPath engine" direction. The functional
        toolkit is otherwise present — Record field access (`datadom.x.y`), the slice-1 `for-each`,
        and stdlib `first`/`last`/`nth`/`string` — so the only missing primitive was
        `normalize_space`. Added `str:normalize_space` (XSLT normalize-space parity) in
        [`strings.rs`](../packages/cem_ql/src/stdlib/strings.rs) +
        [`pipeline.rs`](../packages/cem_ql/src/eval/pipeline.rs), with coverage in
        [`stdlib_runtime.rs`](../packages/cem_ql/tests/stdlib_runtime.rs) (registry count 48→49);
        full cem-ql suite green. Re-scope: token-XHTML→structured-records **shaping** is a host/data
        concern, folded into slice 3 — not an engine feature.
  - [x] Slice 3 — DOM→`datadom` bridge. Landed
        [`data-document.ts`](../packages/cem-elements/src/lib/data-document.ts): `normalizeSpace`,
        `domToRecord` (generic element→record), `tableToRows` (tbody rows → `{td1,td2,…}` records,
        whitespace-normalized), and `followingTable` + `tokenTableRows` (native `querySelector` +
        `nextElementSibling`, replacing the legacy `*[@id]/following-sibling::table[1]/tbody/tr`).
        **No XHTML parser** — the browser already parsed the token doc (`http-request.js`
        `DOMParser`, BR-PH-3). Browser test: the `DataDocumentDomBridge` story (cem-elements `test`
        green, 60 stories). The output shape (`{tdN}` row records) is already navigable per the
        slice-1 for-each record test. Remaining for slice 4: feed the rows into
        `datadom.slices.<name>` per generator.
  - [x] Slice 4 — rewrite each generator to `type="cem-ml; version=0.0"` CEM-ML/CEM-QL and route it
        through the substrate. **All 10 converted** (whole-file: preview tables + CSS body, legacy
        runtime removed); each passes `validate-manifest --hard` + a browser probe.
    - [x] Engine prerequisite A: `cem:for-each` iterates the **members** of a selected `Item::Array`.
          The slice-1 for-each was proven against native multi-item sequences, but the WASM JSON
          boundary delivers `datadom.slices.<name>` (a JSON array of row objects) as a single
          `Item::Array`, so for-each iterated once over the whole array. Fix in
          [`render.rs`](../packages/cem_ql/src/render.rs) `evaluate_select` (flatten one array level
          — legacy XSLT node-set iteration parity).
    - [x] Engine prerequisite B: `RichContent` (triple-backtick) now renders as verbatim text in
          [`render.rs`](../packages/cem_ql/src/render.rs) `compile_node`, so a generator can emit
          output containing literal `{`/`}` (CSS rule blocks `:root { … }`) that would otherwise
          collide with cem-ml's structural braces. Pair rich-content scaffold with sibling
          `{cem:for-each …}` for the dynamic lines. Both prerequisites covered by
          `render_template_for_each_iterates_array_item_members` +
          `render_template_rich_content_emits_literal_braces_around_for_each` in
          [`template_render.rs`](../packages/cem_ql/tests/template_render.rs) (25 render tests, full
          cem-ql suite green); WASM rebuilt.
    - [x] Build wiring + bootstrap: new
          [`cem-css-generator.js`](../packages/cem-theme/src/lib/css-generators/cem-css-generator.js)
          bootstrap (Option B) fetches the token doc, shapes `datadom.slices.<key>` via the slice-3
          DOM→datadom bridge (`tokenTableRows`), renders the `<template type="cem-ml; version=0.0">` through
          the cem-elements runtime-support `renderCemMlTemplate` (cem_ql WASM), materializes the
          plan into `<main>`, and feeds `<cem-css-loader>`. No live browser XSLT.
          [`compile-html.mjs`](../tools/scripts/compile-html.mjs) now stages the cem-elements +
          cem_ql WASM trees into `dist/vendor` unconditionally (`stageSubstrateRuntime`), decoupled
          from the dropped legacy `custom-element.js` reference.
    - [x] **All 10 generators converted** (whole-file: preview tables + CSS body, legacy runtime
          removed), each `validate-manifest --hard` green + browser-probed: `cem-controls`,
          `cem-coupling`, `cem-layering`, `cem-timing`, `cem-stroke`, `cem-dimension`, `cem-shape`,
          `cem-breakpoints` (conditional `@media` ranges via `cem:choose`/`when`/`otherwise`),
          `cem-voice-fonts-typography`, and `cem-colors`. Engine surface used: for-each over array
          slices, rich-content braces, `@media`/theme-class blocks + CSS nesting, `cem:if`/`cem:choose`
          with **bare-name** `@test` comparisons. For `cem-colors` (the hardest) three cem-ql
          additions landed: `str:replace` (the `[emotion]` placeholder substitution), `record_field`
          one-level array flattening (so `datadom.slices.hue.td1` projects across rows), and the
          cross-table join uses set-intersection `&` for sequence membership (the `=`/`!=` operators
          compare only the first item, not existentially). Gotchas: slice keys must avoid cem-ql
          builtin step names (incl. `read`); AVT attributes take one `{$…}` span; `str:concat(seq,sep)`
          is string-join (concatenate two strings with `str:concat((a, b))`); shape's cross-spec
          preview vars come from `<link>`ing the generated CSS. 60 cem-elements stories + full cem-ql
          suite green vs rebuilt WASM.
  - [x] Slice 5 — `@epa-wg/cem-theme:verify:phase13` **fully green** (manifest coverage + browser
        capture for 10 specs, theme-mode resolution ×5, forced-colors, reduced-motion, D3 shape,
        a11y/contrast, cross-spec semantic checks).
- [ ] **Wishlist (future — NOT in the immediate release timeline):** engine XSLT 3.0/4.0 execution
      behind G-NVDL-FULL (AC-P-6.9). The architecture keeps the capability-gated seam — XSLT is a
      peer content type and an unimplemented version rejects deterministically (BR-CO-5/BR-VC-8) —
      so the engine can add XSLT 3/4 later without breaking content. Building the XSLT 3/4 engine
      itself is deferred beyond the immediate release.

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
- [x] Implement the `<cem-element>` runtime in execution slices from
      [`cem-element-design.md` §3–§5](cem-element-design.md): slices A, B, C1, C1.5, C2 (C2.1–C2.6),
      D, E all landed; `cem-elements:verify` green.
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
  - [x] Runtime slice C2: lower canonical inline CEM-ML declaration templates through the `cem_ml`
        WASM/runtime-support boundary into the same render-plan shape. Canonical-subset CEM-ML now renders through
        the `cem_ql` WASM boundary (C2.1–C2.3) with functional `/datadom` data-document selection + `??` (C2.4) and
        `cem:if`/`cem:choose` conditionals, `<data>`/`<option>` payloads, and declarative slot projection (C2.5), and
        the C2.6 verification gate (`nx run cem-elements:verify`) is wired and green. The C1.5 TypeScript adapter is
        removed (C2.6): the WASM render plan drops `<attribute>`/`<slice>` declarations so declaration-bearing
        canonical templates render through WASM, attribute observation moved to a per-instance `MutationObserver`
        (Option A) so `observedAttributes` is no longer a synchronous blocker, declaration diagnostics come from
        the async WASM compile boundary, and `module-url` resource slices now resolve through the shared resource
        resolver hook.
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
          render token, reusing `materializeRenderPlan` and the slice-E frame attributes. The temporary C1.5 adapter
          was fully removed in C2.6 after declaration diagnostics/defaults moved to WASM and attribute observation no
          longer required synchronous declaration metadata. `cem_ql` `render.rs` now carries real per-node author-byte-exact
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
          the temporary C1.5 TS adapter has since been removed in C2.6.
    - [x] C2.5: Add material-parity constructs that depend on full data-bound rendering: `if`/`choose`/`when`,
          `<data>`, `<option>`, `module-url`, and declarative slots. Landed the in-scope constructs — conditionals
          (+ malformed-conditional diagnostics), `<data>`/`<option>` payloads, declarative slot projection, and
          edge-case coverage. `module-url` is deferred to the `src`-loading slice (shared module-map resolver); see
          the sub-item below.
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
      - [x] `<data>` / `<option>` instance payloads — serialize the data-island payload into the snapshot and
            expose it under `/datadom`. Landed as normalized `SerializedPayloadChoice` extraction in
            [`packages/cem-elements/src/lib/cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts):
            ordered `payload.data` / `payload.options` arrays preserve transport identity, while
            `datadom.data.<value>` / `datadom.options.<value>` expose field-access records for current cem-ql
            functional parity. `<optgroup label>` is captured as `group`. Coverage: `DataOptionPayloadRenderLoop`.
        - [x] Review finding: the data-document contract should be unified before adding `<data>`/`<option>`.
              Started: the WASM path now passes a structured `datadom` object derived from `DataIslandSnapshot`
              (`attributes`, `dataset`, `payload`, `slots`, `slices`, `validationState`, `eventPayloads`) while keeping
              flat host bindings for `$name` compatibility, and `cem_ql` preserves explicit `datadom` bindings instead
              of rebuilding them. The TS fallback consumes primitive `datadom.*` paths flattened from the same
              snapshot for its temporary `${}` interpolation path. The concrete `<data>`/`<option>` payload mapping is
              now defined by value plus ordered arrays as above.
      - [x] `module-url` resource slices — resolved through the shared resource/module-map hook. Landed in
            [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts): rendered `<module-url slice src>`
            helpers are inert, removed from light-DOM output, resolved via `resolveModuleUrl(specifier, baseDocument)`
            (defaulting URL-like specifiers against `baseURI`; bare package specifiers require the host hook), cached,
            exposed under `datadom.slices.<slice>`, and trigger an async rerender when the resolved value changes.
            Coverage: `MaterialIconLinkParity` exercises bare `@scope` resource specifiers with an injected resolver;
            55 Storybook stories and `nx run cem-elements:verify` are green.
      - [x] Declarative slot projection — project the produced instance's payload into `<slot>` positions in the
            light DOM (named + unnamed, slot default content as fallback). Landed as a render-plan lowering step in
            [`packages/cem-elements/src/lib/projection.ts`](../packages/cem-elements/src/lib/projection.ts), with both
            DOM and WASM render plans lowered before live DOM materialization. Each `<slot>` is replaced by
            serialized data-island payload assigned to it (named slots match `slot="<name>"`; the default slot takes
            unslotted payload text/elements) or by its rendered fallback children when unfilled. The inert island
            remains the durable source across rerenders. Coverage: `SlotProjectionRenderLoop` and
            `SlotProjectionWasmRenderLoop`.
        - [x] Review finding: slot projection currently reads live island DOM after WASM materialization, so it is not
              reproducible in worker/SSR/edge hosts from the serialized snapshot. Started: slot payload serialization
              moved into `DataIslandSnapshot`, and browser projection now reads `DataIslandSnapshot.payload` instead
              of the live data-island template; the WASM data document exposes slottables under `datadom.payload` and
              `datadom.slots`. Lowering now happens from serialized payload in the render plan before materialization;
              no post-materialization DOM slot query/replacement is needed.
        - [x] Add C2.5 edge-case coverage: nested conditionals, repeated same-name slots, mixed unslotted elements plus
              text ordering, slot fallback cloning, rerender after payload mutation, and serialized-payload projection
              on the WASM path. Mixed default-slot text/element ordering, fallback cloning, and payload-mutation
              rerender are covered by `SlotProjectionRenderLoop`; serialized-payload projection on the WASM path by
              `SlotProjectionWasmRenderLoop`; nested `cem:if`/`cem:choose`/`cem:when`/`cem:otherwise` by
              `render_template_supports_nested_conditionals` in
              [`template_render.rs`](../packages/cem_ql/tests/template_render.rs) (first-match wins, skipped subtree,
              nested-if in `otherwise`); repeated same-name slots (first match consumes the payload, later same-name
              slots fall back) by the `SlotProjectionRepeatedNames` story. 38 stories + 18 render tests green.
    - [x] C2.6: Wire the verification gate through `cem_ml_cli:validate-fixtures`, `cem_ml_cli:e2e`, and Storybook
          parity stories before retiring the C1.5 fallback. Decision (2026-06-04): cem-element substrate templates use
          a vocabulary that is not Tier-A HTML/SVG and whose host bindings resolve only at render time, so the semantic
          `validate` gate intentionally rejects them; they ride a **structural parse + roundtrip** leg instead. Landed:
          substrate fixtures in [`examples/cem-elements/`](../examples/cem-elements/) (conditionals, data-document,
          slots/data/option) verified by [`tools/scripts/verify-cem-elements-substrate.mjs`](../tools/scripts/verify-cem-elements-substrate.mjs)
          (real `cem-ml convert cem->cem` structural success + canonical-roundtrip idempotence) through the new
          `nx run cem-elements:verify-substrate` target; and a composite `nx run cem-elements:verify` gate that runs
          `cem_ml_cli:validate-fixtures` + `cem_ml_cli:e2e` + `verify-substrate` + the Storybook `test` (39 stories) —
          all 7 gate tasks green.
      - [x] Remove the C1.5 fallback. Earlier C2.6 work narrowed it while `observedAttributes` still required
            synchronous declaration metadata; async host-attribute observation removed that blocker. The `cem_ql`
            render plan now drops top-level
            `<attribute>`/`<slice>` declaration nodes ([`render.rs`](../packages/cem_ql/src/render.rs)
            `is_top_level_declaration`), and the cem-elements WASM-eligibility gate no longer excludes declaration-bearing
            templates ([`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts) `wasmEligible`), so
            declaration-bearing canonical templates (`<attribute>`/`<slice>` + `{$…}` content) now render through WASM.
            Declaration diagnostics are sourced from async WASM compile (`compileCemMlTemplate`), attribute changes are
            observed by instance-level `MutationObserver`, and the obsolete `cem-ml-template.ts` adapter plus `${}`
            fallback stories were deleted. Coverage: `render_template_drops_top_level_attribute_and_slice_declarations` in
            [`template_render.rs`](../packages/cem_ql/tests/template_render.rs) and the `DeclaredAttributeWasmRenderLoop`
            story.
      - [x] Async attribute observation (Option A — chosen for reliability): replace synchronous
            `observedAttributes` / `attributeChangedCallback` with a per-instance `MutationObserver` over host
            attributes (and the inert data-island), so produced elements stay **defined synchronously** while host
            attribute changes schedule an **async** re-render that reads the live attributes fresh. Observing every
            attribute also re-renders on *undeclared* attributes (reachable via `datadom.attributes.<x>`) that
            `observedAttributes` could never observe. Landed in
            [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts) (`observeInstance`,
            `disconnectProducedInstance`, `isConnected` guard; re-entrancy is structurally precluded since render
          never mutates an observed target). Coverage: `AttributeObserverRerendersOnUndeclaredAttribute`; 39 stories
            green. This removed the hard synchronous blocker for full C1.5 removal.
      - [x] Full C1.5 removal: the last declaration-metadata uses moved off the TS scan, WASM render applies
            `<attribute>` defaults internally, declaration diagnostics come from async WASM compile, bespoke `${}`
            stories were removed/migrated to canonical `{$…}`, parser-specific `cem-element.cem_ml.*` story assertions
            were replaced by WASM tokenizer diagnostics, and `cem-ml-template.ts` was deleted.
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
- [x] Build the legacy parity feature inventory from the old `@epa-wg/custom-element` suite and docs
      (`~/aWork/custom-element/docs/{attributes,rendering}.md` plus legacy test files). Convert every in-scope
      behavior into a named `<cem-element>` Storybook parity story; record intentional CEM-ML/CEM-QL replacements as
      migration decisions. Landed in
      [`packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md):
      `/home/suns/aWork/custom-element` had docs/demos/implementation references but no dedicated test/spec files,
      so the matrix maps each legacy behavior to existing stories, new named stories
      (`LegacyAttributeDefaultsAndHostOverridesParity`, `LegacyDatadomAccessMigrationParity`,
      `LegacyNamedSlotPayloadParity`, `LegacySliceInputEventParity`, `LegacySrcDeclarationLoadingIsTrackedAsBlocked`,
      `LegacyBridgeTemplateParity`), or an explicit migration/blocker decision (`src`, inline no-`tag`,
      XSLT `for-each`/`variable`, scoped CSS, resource primitives). Storybook coverage is now 45 passing stories.
- [x] Land material parity stories for every component in `~/aWork/custom-element-dist/src/material/` (action,
      autocomplete, badge, dropdown, icon, icon-link, input, menu). Landed a named parity story per component in
      [`packages/cem-elements/src/lib/material-parity.stories.ts`](../packages/cem-elements/src/lib/material-parity.stories.ts)
      reproducing each component's characteristic in-scope behavior on the C2 substrate (attribute defaults, `/datadom`
      selection, `??`, `cem:if`/`cem:choose`, declarative slots, `<data>`/`<option>`, slice events) plus nested
      composition; the file header records the intentional CEM-ML/CEM-QL migration decisions (canonical `cem:if`/`{$…}`
      vs legacy `<choose>`/DOM `{$x}`; cem-ql functional `/datadom` vs XPath; page-global scoped styles; host-provided
      module-map hooks for bare `@scope` `src` / `module-url` specifiers; `<if><attribute>` forwarding; builtin-step
      selection-key collisions). 55 stories green. The cem-ml/cem-ql features, `src` loading, and `module-url` resource
      slices these needed landed in C2 + the `src`/resource slices above.
  - [x] `src` declaration loading — local `src="#id"` (same-document `<template id>` / declaration resolution).
        Landed in [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts) (`resolveSrcTemplate`,
        `parseSrcReference`, `templateFromTarget`): a `<cem-element src="#id" tag="…">` resolves the same-document
        template and registers the produced tag from it; external `src="./file#tag"` and missing local targets are
        diagnosed (`cem-element.src_external_not_implemented` / `src_local_target_missing`). Coverage:
        `LocalSrcDeclarationLoadingParity` and `ExternalSrcDeclarationLoadingIsTrackedAsBlocked`; 46 stories green.
  - [x] `src` external / module-map loading — async fetch + parse of an external declaration document, then register
        the produced tag from its `#fragment` template. Landed in
        [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts): `registerExternalDeclaration` loads the
        referenced document (cached per path via `loadSrcDocumentParsed`), `DOMParser`-parses it, resolves the
        `#fragment` through `templateFromTarget`, `importNode`s the `<template>`, and registers asynchronously; a host
        `loadSrcDocument(specifier, baseDocument)` option controls module-map resolution / fetching / scope policy
        (default resolves the path against the document base URL and `fetch`es; bare `@scope/pkg` specifiers require a
        host loader). A content-aware `templateSourceText` reads the cem-ml source from parsed templates' `.content`.
        Load failures / missing fragments are diagnosed (`src_load_failed` / `src_target_missing`). Coverage:
        `ExternalSrcDeclarationLoadingParity` (injected loader → fetch/parse/register/render) and
        `SrcDeclarationLoadingDiagnostics`; 47 stories green. NB the default `fetch` path is exercised in real browsers;
        the injectable `loadSrcDocument` is the tested boundary.
  - [x] `module-url` resource slices — inert resource helpers in rendered output resolve URL-like specifiers by default
        and bare package/module specifiers through a host `resolveModuleUrl` hook, write the resolved value into the
        named slice, and rerender CEM-ML templates through `datadom.slices.<slice>`. Coverage landed in
        `MaterialIconLinkParity`; 55 stories green and `nx run cem-elements:verify` passes.
  - [x] Per-component parity stories — a named story per component (icon, icon-link, menu, badge, action, dropdown,
        input, autocomplete) in
        [`material-parity.stories.ts`](../packages/cem-elements/src/lib/material-parity.stories.ts), each exercising the
        component's characteristic behavior (attribute defaults, `/datadom` selection, `cem:if`/`cem:choose`, slots,
        `<data>`/`<option>`, slice events, nested composition) with migration decisions recorded in the file header.
        Remaining caveats noted there: bare `@scope/pkg` module specifiers need host `loadSrcDocument` /
        `resolveModuleUrl` hooks, scoped styles render page-global, and richer `@test` expressions (string functions,
        equality) author through cem-ql functional selection.
- [x] Build a material parity inventory from `~/aWork/custom-element-dist/src/material/components/*.html` covering
      local/external `src`, hidden declarations, nested custom elements, declarative slots, scoped styles,
      `attribute select`, `if`/`choose` bridge constructs, namespaced `xhtml:*` elements, boolean attribute helper
      semantics, `module-url` resource slices, `data`/`option` payloads, slice events, and `slice-value`. Landed in
      [`packages/cem-elements/docs/material-parity-inventory.md`](../packages/cem-elements/docs/material-parity-inventory.md):
      per-component feature usage plus a 22-row feature→runtime-support matrix. Key finding — external/local `src`
      declaration loading is the hard blocker (all 8 components compose via `src` imports), and the
      conditional/expression/slot/`data`-payload features gate behind slice C2 (cem-ml/cem-ql).
- [x] Wire `cem-element` through `nx run cem_ml_cli:validate-fixtures` and `cem_ml_cli:e2e` so substrate templates
      ride the same Phase 2 verification. Landed via the C2.6 gate: `nx run cem-elements:verify` composes
      `cem_ml_cli:validate-fixtures` + `cem_ml_cli:e2e` with the `cem-elements:verify-substrate` structural
      parse+roundtrip leg over [`examples/cem-elements/`](../examples/cem-elements/) and the Storybook `test`.
- [x] Production-ready gate: parity (1)–(6) from [`cem-element-design.md` §7](cem-element-design.md). When green,
      the browser substrate is eligible for Phase 3.5 Edge/SSR follow-up; `@epa-wg/custom-element` adoption remains
      deferred until after that follow-up phase.
  - [x] (1) Functional parity story coverage for in-scope legacy behavior.
  - [x] (2) Template and data-island isolation stories.
  - [x] (3) Material parity stories for the eight legacy material components, including local/external `src`,
        declarative slots, data/option payloads, slice events, and `module-url` resource slices.
  - [x] (4) CEM-ML integration through `nx run cem-elements:verify`.
  - [x] (5) Performance: prove AC-N-1 first-paint budgets on the material parity fixtures under the `cem_ml:bench`
        discipline. Landed `examples/cem-elements/material-parity.cem`, a canonical substrate fixture covering the
        eight material component shapes plus slots, slices, data/option payloads, and `module-url`; and
        `ac_n_1_cem_element_material_parity_fixtures_under_budget` in
        [`packages/cem_ml/tests/perf_budgets.rs`](../packages/cem_ml/tests/perf_budgets.rs), which runs
        `examples/cem-elements/material-*.cem` through the same AC-N-1 budget harness as the base CEM/HTML fixtures.
  - [x] (6) A11y: end-to-end accessibility-contract assertions on the material parity fixtures. Landed in
        [`material-parity.stories.ts`](../packages/cem-elements/src/lib/material-parity.stories.ts): the material
        stories now assert accessible names, native implicit roles, host focus delegation, disclosure state
        mirroring, decorative resource images, and `aria-*` reference integrity across the eight material parity
        fixtures; `nx run cem-elements:test` / `nx run cem-elements:verify` are green.
- [x] Bridge support: `<template lang="custom-element-v0">` compat path for legacy authoring during the migration
      window; keep only if needed after the `@epa-wg/custom-element` substrate adoption. Landed as the
      package-private legacy projection path in [`projection.ts`](../packages/cem-elements/src/lib/projection.ts) and
      [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts): legacy-v0 templates reuse the DOM source
      reader, accept legacy `{name}` / `{$name}` / `{//path}` interpolation, bridge `if` / `choose` / `when` /
      `otherwise`, declaration attributes/slices, slots, and the same resource/slice event handling as the DOM path.
      Coverage: `LegacyBridgeTemplateParity`; unsupported XSLT-only constructs remain adoption-phase follow-up.
- [~] Legacy HTML+XSLT backward-compat via DOM→CEM-ML conversion (keep the test suite, demos, and material
      components working on the substrate engine — **no browser `XSLTProcessor`**, so the FF-5 forbidden gate
      stays green). Decision: legacy template DOM is parsed with HTML + XSLT namespaces and **transpiled to
      canonical CEM-ML**, then rendered on the same cem_ql WASM engine as migrated templates, so a legacy
      sample and its CEM-ML twin render identically. **Landed:** the converter
      [`legacy-xslt/convert.ts`](../packages/cem-elements/src/lib/legacy-xslt/convert.ts) (Tier 1/2 element +
      XPath-expression mapping, inline node-set `for-each` unrolling), the runtime `legacy-xslt` mode in
      [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts) (auto-detection, namespace-split
      XML re-parse, convert→WASM), the adapter routing in
      [`custom-element.js`](../packages/custom-element/custom-element.js) (`custom-element-xslt` default), the
      cem-ql XPath stdlib gap (`str:translate`/`substring`/`substring_before`/`substring_after`, `seq:count`,
      `cem:for-each` `$position`), and the engine fix binding declared attributes when unset. Coverage: twin
      stories in [`legacy-xslt-parity.stories.ts`](../packages/cem-elements/src/lib/legacy-xslt-parity.stories.ts)
      (legacy ⇆ CEM-ML identical DOM) + `convert.spec.ts`; `cem-elements:verify` + `@epa-wg/custom-element:verify`
      + FF-5 green. **Remaining:** copy the demo/stories/material modules into the repo and port the rest of the
      legacy story patterns (material components, `xslt-if`/`xslt-conditionals`) as twin stories. **Tier 3
      deferred** (standalone XSLT stylesheets: push-model `apply-templates`/`call-template`/`sort`, EXSLT
      `func:function`, `msxsl:script` — non-transpilable; emit a conversion diagnostic).

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
- [x] Build the test harness for DOM rendering, events, accessibility assertions, and visual snapshots. Landed in
      [`component-harness.ts`](../packages/cem-components/src/lib/testing/component-harness.ts) with browser-backed
      coverage in
      [`component-harness.browser.spec.ts`](../packages/cem-components/src/lib/testing/component-harness.browser.spec.ts):
      the harness asserts light-DOM output, component event bubbling/composition and JSON payloads, accessible names,
      ARIA/reference integrity, focus indicators, deterministic visual snapshots, and a Chromium screenshot smoke path.
      `nx run @epa-wg/cem-components:test/build/lint` are green.
- [x] Implement minimal primitives: action, field, surface, text, icon, stack, grid, list, nav, dialog shell. Landed as
      installable `<cem-element>` CEM-ML declarations in
      [`primitives.ts`](../packages/cem-components/src/lib/primitives.ts): `cem-action`, `cem-field`, `cem-surface`,
      `cem-text`, `cem-icon`, `cem-stack`, `cem-grid`, `cem-list`, `cem-nav`, and `cem-dialog-shell` register through
      `installCemComponentPrimitives(runtime)` without imperative component classes. Browser coverage in
      [`primitives.browser.spec.ts`](../packages/cem-components/src/lib/primitives.browser.spec.ts) verifies the
      rendered light-DOM output, labels, landmarks, dialog semantics, field label/help separation, and ARIA/reference
      integrity. `nx run @epa-wg/cem-components:test/build/lint` are green.

## Phase 3.5 — Edge/SSR Processing Follow-Up

Roadmap: [`../roadmap.md` §Phase 3.5](../roadmap.md). Starts after the Phase 3 browser substrate production-ready
gate is green.

- [x] Add SSR fixture that renders initial HTML plus hydration metadata from a serialized `DataIslandSnapshot`, then
      hydrates into the same client-side data-island and render-plan identity. Landed in
      [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts) and
      [`SsrHydrationFromSerializedSnapshot`](../packages/cem-elements/src/lib/cem-elements.stories.ts): SSR output
      carries a direct instance data-island, render-boundary comments, and
      `<script type="application/json" data-cem-hydration="snapshot">` metadata. The client runtime preserves the
      server render-plan artifact/data revision on first connect, keeps the payload data island, and then handles
      ordinary attribute invalidation. `nx run cem-elements:test/build/lint` are green.
- [x] Add edge-processing fixture using serialized data snapshot plus previous render-plan identity to produce a
      patch-frame stream without access to live browser DOM. Landed in
      [`projection.ts`](../packages/cem-elements/src/lib/projection.ts) and
      [`EdgePatchFramesFromSerializedSnapshot`](../packages/cem-elements/src/lib/cem-elements.stories.ts): the pure
      `diffRenderPlansToPatchFrames(previous, next)` helper emits `begin`, batched `ops`, and `commit` frames from
      serializable render plans only. The fixture proves a stable render-node text patch and commit identity from two
      serialized `DataIslandSnapshot` revisions, with `replaceScope` reserved for first-render/fallback cases.
      `nx run cem-elements:test/build/lint` are green.
- [x] Verify privacy/export policy for browser-to-edge snapshots: denied fields are omitted or redacted before
      leaving the browser context. Landed in
      [`cem-elements.ts`](../packages/cem-elements/src/lib/cem-elements.ts) and
      [`BrowserToEdgeSnapshotPrivacyPolicy`](../packages/cem-elements/src/lib/cem-elements.stories.ts): the
      browser-to-edge export helper fail-closes data-bearing fields to omission unless the effective policy allows
      them, supports explicit redaction to empty JSON-compatible payload/record shapes, stamps the exported snapshot
      with the effective privacy policy, and detaches allowed fields from later browser mutation. `nx run
      cem-elements:test/build/lint` are green.
- [x] Decide the first edge render-state storage model: content-addressed cache only, revisioned KV/document records,
      or both. Accepted the hybrid model: immutable content-addressed blobs for template artifacts, render plans,
      rendered HTML fragments, and policy-sanitized snapshot exports, plus small revisioned pointer records carrying
      `RenderRevision`, content addresses, scope/privacy policy stamps, and an ETag-like compare value. Persistent
      full snapshot storage remains opt-in by export policy, and live/browser-only state remains out of scope. Landed
      in [`cem-element-design.md`](cem-element-design.md),
      [`cem-element-wasm-proposal.md`](cem-element-wasm-proposal.md),
      [`projection.ts`](../packages/cem-elements/src/lib/projection.ts) (`EdgeRenderStateRecord`,
      `InMemoryEdgeRenderStateStore`, `readEdgeContent`, `readEdgeRenderStateContents`,
      `advanceEdgeRenderState`, `projectAndAdvanceEdgeRenderState`), and
      [`EdgeRenderStateHybridStorageModel`](../packages/cem-elements/src/lib/cem-elements.stories.ts), including
      content-addressed template artifact/render plan/snapshot export/HTML writes, ETag compare-and-swap,
      stale-write rejection, sanitized snapshot retrieval, and explicit null snapshot/empty HTML content addressing,
      store-backed patch-frame generation from the previous content-addressed render plan, projection from serialized
      source/snapshot input, first-render replacement frames, whole-record content verification with field-specific
      missing-content/content-address mismatch failures for every pointer field, and render-revision mismatch
      handling.

## Phase 3.6 — `@epa-wg/custom-element` Monorepo Adoption

Roadmap: [`../roadmap.md` §Phase 3.6](../roadmap.md). Starts after Phase 3.5 is green.

- [x] Set the migration scope and branch strategy for moving `@epa-wg/custom-element` from
      `~/aWork/custom-element/` into `packages/custom-element/`, including how published npm identity and source
      history will be preserved. Landed in
      [`custom-element-migration-scope.md`](custom-element-migration-scope.md): Phase 3.6 uses staged branches for
      scope, history import, package scaffolding, adapter implementation, downstream consumers, and release readiness;
      separates the local `0.0.37` history source from the workspace's consumed `0.0.39` package baseline; avoids
      importing generic legacy release tags without namespacing; and sets the first-pass in-scope/out-of-scope
      migration boundaries.
- [x] Capture the migration baseline before importing code: current npm version, published package name, entrypoints
      (`custom-element.js`, `http-request.js`, IDE metadata, demos/docs), license/readme files, side-effect module
      behavior, and every root/package reference to `node_modules/@epa-wg/custom-element`. Landed in
      [`custom-element-package-baseline.md`](custom-element-package-baseline.md): the workspace consumes
      `@epa-wg/custom-element@0.0.39`, while the local history checkout is `0.0.37`; `custom-element.js` contains
      the published `0.0.39` browser fixes that must be preserved; companion modules register custom elements as
      import side effects; `module-url.js` ships as a browser file but is not re-exported from `index.js`; and the
      root/package plus `cem-theme` node_modules references are inventoried for the consumer-rewire phase.
- [x] Import the legacy source into `packages/custom-element/` with history-preserving mechanics where practical.
      Keep the POC as a functional reference only; do not let its XSLT/XPath implementation become the new
      architecture decision point. Landed as a published `0.0.39` source snapshot in
      [`../packages/custom-element/`](../packages/custom-element/) with editor/cache directories omitted; added
      [`IMPORT.md`](../packages/custom-element/IMPORT.md) to record the local `0.0.37` history source, the published
      `0.0.39` behavior baseline, and the branch-level history-graft strategy from
      [`custom-element-migration-scope.md`](custom-element-migration-scope.md). A true non-squashed history graft
      remains a dedicated clean-branch/import-commit operation rather than a hidden mixed-change side effect.
- [x] Scaffold the workspace package as the future `@epa-wg/custom-element` publish unit: package name, Nx targets,
      TypeScript/build output, package exports, browser module paths, IDE assets, README/docs, and release metadata
      must preserve the published npm identity while allowing a next-major implementation. Landed in
      [`project.json`](../packages/custom-element/project.json) plus package-local
      [`scripts/build-package.mjs`](../packages/custom-element/scripts/build-package.mjs) and
      [`scripts/verify-package-baseline.mjs`](../packages/custom-element/scripts/verify-package-baseline.mjs):
      Nx now recognizes `@epa-wg/custom-element` with `build`, `test`, `lint`, and `verify` targets; `build` stages
      the publish-shaped package into `dist`; the verifier checks package identity, exports, browser entrypoints,
      IDE metadata, and import-time custom-element registrations; package scripts route build/test/lint through Nx;
      and the legacy `.gitignore` was tightened so imported demo baseline files remain visible while `dist` stays
      ignored.
- [x] Define the adapter boundary from `<custom-element>` to the `cem-element` substrate. The package must keep
      publishing `<custom-element>` as the public tag, but internally translate legacy declaration shape (`tag`,
      `src`, inline templates, data islands, slices, host attributes, and event-to-data wiring) into the same
      declaration/runtime records used by `packages/cem-elements`; it must not retain a separate parser/render engine.
      Landed in [`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md): `custom-element.js`
      remains the public adapter and side-effect registration surface, while declaration compilation,
      produced-element registration, payload/data-island capture, slices, diagnostics, and rendering delegate to a
      `CemElementRuntime` configured with `declarationTag: 'custom-element'`; legacy normalization is limited to
      translating `tag`, `src`, inline bridge templates, data islands, slices, host attributes, and resources into the
      substrate model, with no package-local XSLT/XPath renderer.
- [x] Decide the bridge-template policy for the next major using fixture evidence. Start by keeping
      `<template lang="custom-element-v0">` for the migration window, then explicitly keep, migrate, or drop each
      adoption-phase legacy gap from
      [`legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md): omitted `tag`,
      full XSLT-only `for-each`/`variable`, broad XPath functions, multiple slice events/targets, resource slices,
      and true scoped CSS behavior.
      Landed in [`custom-element-bridge-template-policy.md`](custom-element-bridge-template-policy.md):
      `custom-element-v0` stays as a fixture-bounded migration bridge into `CemElementRuntime`; omitted `tag`,
      XSLT-only loops/variables, broad XPath, multi-target slice wiring, and scoped selector rewriting migrate or drop
      instead of retaining an engine fork; `module-url` stays through `resolveModuleUrl`; companion resource modules
      move to the next package task.
- [x] Port or replace package companion modules and resource primitives deliberately. `http-request.js`, demo
      resource helpers, `local-storage`, `location-element`, and `module-url` compatibility should either become
      substrate-backed primitives, documented shims, or explicit non-goals for the next major.
      Landed in [`custom-element-companion-modules.md`](custom-element-companion-modules.md): preserve published
      companion files and import-time registrations; keep `module-url` as the only first-pass substrate-backed
      resource path through `resolveModuleUrl`; keep `http-request`, `local-storage`, and `location-element` as
      documented browser shims with explicit-event migration fixtures rather than implicit render primitives; defer
      broader resource primitive design until host policy, privacy/export, async ordering, and edge/SSR behavior are
      specified.
- [x] Rewire downstream consumers to the workspace package without breaking existing HTML generator workflows.
      Update root dependencies and `packages/cem-theme` script/docs references that currently load
      `node_modules/@epa-wg/custom-element/{custom-element.js,http-request.js}`; keep browser-served paths stable or
      document the new import path.
      Landed in [`custom-element-consumer-rewire.md`](custom-element-consumer-rewire.md): the root dependency now uses
      `workspace:^`, IDE web-types point at `packages/custom-element`, `cem-theme:build:html` cache inputs track the
      workspace package sources, and source HTML keeps the stable `node_modules/@epa-wg/custom-element/...` browser
      path that `compile-html` vendors into `dist/vendor`.
- [x] Add package-local verification fixtures for `@epa-wg/custom-element`: legacy docs/demo parity, migrated
      `<custom-element>` adapter behavior, companion module behavior, package export/import smoke tests, and
      release-pack artifact shape. Reuse the existing `cem-elements` Storybook parity stories as acceptance fixtures
      instead of duplicating behavior assertions.
  - [x] Add the first package-local browser smoke fixture. Landed in
        [`../packages/custom-element/test-fixtures/browser-smoke.html`](../packages/custom-element/test-fixtures/browser-smoke.html)
        and [`../packages/custom-element/scripts/verify-browser-fixtures.mjs`](../packages/custom-element/scripts/verify-browser-fixtures.mjs):
        `@epa-wg/custom-element:test` now verifies package export/import identity, import-time registrations for
        `custom-element`, `http-request`, `local-storage`, `location-element`, and `module-url`, plus one legacy
        declaration render through Chromium; the existing package baseline verifier continues checking root and dist
        release-pack shape.
  - [x] Add companion-module behavior assertions to the browser fixture: `http-request` fetches local JSON and records
        request/response data, `local-storage` follows same-tab live updates, `location-element` parses URL fields and
        repeated params, and standalone `module-url` resolves a relative browser module URL.
  - [x] Run the browser smoke fixture against both workspace source files and the built `dist/` package artifact so
        export/import, companion side effects, and the legacy render smoke are verified against release-pack output.
  - [x] Document the adapter runtime packaging gate before replacing `custom-element.js`. Landed in
        [`custom-element-adapter-runtime-packaging.md`](custom-element-adapter-runtime-packaging.md): a direct
        `CemElementRuntime` import would break plain browser module consumers unless the substrate runtime and
        `cem_ql` WASM assets are vendored or bundled behind stable package-relative URLs; the next implementation
        slice should stage that browser-ready runtime path first.
  - [x] Stage the first browser-ready substrate runtime payload in the custom-element dist package:
        `scripts/build-package.mjs` copies `cem-elements/dist` and `cem_ql/dist/wasm` under
        `dist/vendor/@epa-wg/`, and the package baseline verifier asserts the staged runtime JS plus WASM files are
        present before adapter implementation begins.
  - [x] Replace `custom-element.js` with the first `CemElementRuntime` adapter slice. The public `CustomElement`
        export and import-time `custom-element` registration are preserved; untyped inline templates are normalized to
        `lang="custom-element-v0"`; source imports use the workspace `cem-elements` build and dist imports are rewritten
        to the vendored runtime path; browser smoke fixtures now assert substrate data-island output.
  - [x] Preserve migration-window omitted-`tag` inline rendering for legacy generator workflows: the adapter now assigns
        a stable generated produced tag, registers it through `CemElementRuntime`, appends one inline produced instance,
        and covers that path in source/dist browser fixtures.
  - [x] Add adapter-regression guards to the package verifier: `custom-element.js` must keep import-time registration
        but must not contain `XSLTProcessor`, `createXsltFromDom`, or the legacy `DceElement` produced-class engine.
  - [x] Update `cem-theme` HTML compilation for adapter transitive runtime files: when `custom-element.js` is vendored,
        `compile-html.mjs` now also copies `cem-elements/dist` and `cem_ql/dist/wasm` into `dist/vendor/@epa-wg/`, and
        the `build:html` target depends on those runtime builds.
      Package-local verification is now green through `@epa-wg/custom-element:verify`: source and dist browser smoke
      fixtures prove adapter registration, legacy-v0 normalization, substrate data islands, companion modules,
      package export/import identity, release-pack runtime staging, and no legacy XSLT/render-engine regression. The
      full cross-package gate remains the next todo item.
- [x] Verify the migrated package against all required gates: legacy parity inventory, material parity inventory,
      Phase 3.5 Edge/SSR fixtures, `cem-elements:verify`, the new `custom-element` package build/test/lint targets,
      and any affected `cem-theme` HTML/token generator workflows. **All green (re-verified 2026-06-09):**
      `cem-elements:verify` (61 Storybook parity + Edge/SSR fixtures + cem_ml_cli e2e/validate-fixtures + test:unit),
      `@epa-wg/custom-element:verify` (build/test/lint + browser smoke), `@epa-wg/cem-theme:build:html`, and
      `@epa-wg/cem-theme:verify:phase13` (the prior XSLT blocker — now green after the Option-B generator conversion).
      The legacy + material parity inventories are the docs/stories landed in the sub-items above.
  - [x] Run the first migrated-package gate pass. Landed findings in
        [`custom-element-migrated-package-gate.md`](custom-element-migrated-package-gate.md): `cem-elements:verify`,
        `@epa-wg/custom-element:verify`, and `@epa-wg/cem-theme:build:html` pass; `@epa-wg/cem-theme:verify:phase13`
        fails because existing CSS generator templates still depend on full XSLT+XPath `<variable>`, `<for-each>`, and
        broad XPath evaluation, producing empty CSS under the substrate adapter.
  - [x] Update the migration plan to carry both valid template options. Landed in
        [`custom-element-template-migration-options.md`](custom-element-template-migration-options.md): Option A keeps
        XSLT+XPath with legacy HTML/XSLT default-namespace behavior; Option B converts the logic to CEM-ML+CEM-QL under
        `<template type="cem-ml; version=0.0">`; Option B is the recommended path for `cem-theme` CSS generation after
        conversion.
  - [x] Convert the `cem-theme` CSS generator workflow to CEM-ML+CEM-QL templates marked
        `<template type="cem-ml; version=0.0">` (Option B), then rerun `@epa-wg/cem-theme:verify:phase13`.
        **Done** — all 10 generators converted (see the Phase-3.6 Option-B slices under
        "What's left — execution"); `verify:phase13` fully green; the live browser-XSLT runtime is
        retired from the generators.
- [~] Publish-readiness pass for the next major: changelog, migration guide from external POC package to workspace
      package, bridge-window support matrix, breaking-change list, npm package contents check, and rollback plan for
      consumers that still depend on the old XSLT-only surface. **Analysis landed** in
      [`release-readiness-0.1.0.md`](release-readiness-0.1.0.md): target **0.1.0**, bridge policy
      **deprecate-now / remove-next-major** (FF-5-gated), changelog summary, breaking-change list, support matrix,
      rollback plan, and the npm-contents check. **Release topology** (corrected from `nx.json`): the fixed nx `cem`
      group bumps `cem-theme` + `cem-components` + `cem-elements` (+ private root) together; `custom-element` releases
      separately (own repo); `trang-native` independent. **npm-contents:** publish is from each package's clean
      `dist/` (custom-element packs 80 files self-contained; the source `files:["*"]` is a dev-manifest non-issue) —
      only stray is a 38 kB vendored `tsbuildinfo` (optional cleanup). **Remaining = maintainer publish actions**
      (§7 checklist): land 0.1.0 on the nx group + custom-element's own repo, `npm pack --dry-run` per `dist/`,
      regenerate `CHANGELOG.md`, add the legacy-deprecation README notice, then tag/publish.

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
