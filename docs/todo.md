# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).
Each item names the AC reference and design home so the closing change ship with a citation.

## Phase 2 — Implementation Tasks (`@epa-wg/cem-ml` / `@epa-wg/cem-ml-cli` / `@epa-wg/cem-ql`)

Acceptance criteria: [`cem-ml-ac.md`](cem-ml-ac.md), [`cem-ql-ac.md`](cem-ql-ac.md). Design homes:
[`cem-ml-stack-design.md`](cem-ml-stack-design.md), [`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md),
[`cem-ql-stack-design.md`](cem-ql-stack-design.md),
[`cem-ql-stack-design-impl.md`](cem-ql-stack-design-impl.md).

### Inline Schema And Mid-Document Schema Switch (AC-F-2, IMPL-FOLLOW-004)

Schema scoping is resolved at the AC level (`MEMORY.md` `project_schema_scoping.md`); parser/schema-frame lowering for
the in-document forms is now covered by fixtures and schema-machine tests.

- [x] Implement parser and schema-frame lowering for inline `{cem:schema @cem:name | ... }` declarations.
- [x] Implement `cem:schema-src` / `cem:schema-select` host-attribute switches.
- [x] Implement scope-chain `cem:name` shadowing per `cem-ml-stack-design.md §13.1`. Add fixtures covering shadowing,
      sibling isolation, and override boundaries.

### Plugin Runtime (AC-PL-1..AC-PL-20, IMPL-FOLLOW-006)

`packages/cem_ml/src/plugin/` module exists; descriptor, chain, lifecycle, and sandboxing per the resolved
host-trusted + Rust AST capability validator model (`MEMORY.md` `project_plugin_sandboxing.md`) need to be plumbed
through.

- [x] Implement plugin descriptor, chain composition, install/uninstall lifecycle, observe/mutate mode separation, and
      priority ordering.
- [x] Implement source-map stitching across plugin boundaries (host-frame inheritance + plugin-introduced frame).
- [x] Implement Rust AST capability validator at load time per the resolved sandboxing model.
- [x] Add plugin-budget enforcement against the scope policy. Emit `cem.plugin.budget_exceeded` on breach.
- [x] Plugin runtime AC verification tests: `tests/plugin_runtime.rs` already exists — extend it with descriptor /
      chain / source-map-stitching / sandbox cases.

### Scheduler Completion (AC-A-2..AC-A-7, AC-O-2, IMPL-FOLLOW-007)

`packages/cem_ml/src/scheduler/` module exists with six submodules. Worker pool, bounded queue, cancellation, and I/O
queue are implemented; deterministic trace report projection is still pending.

- [x] Implement per-scope thread pool and bounded queue per AC-A-4 / AC-A-5. Queue overflow emits
      `cem.scheduler.queue_full`; the diagnostic carries the overflowing scope id.
- [x] Implement end-to-end `AbortSignal` propagation per AC-A-7. Cancellation halts in-flight work at the next
      safe-point and surfaces `cem.scheduler.aborted` with the originating cancel-site source-map stack.
- [x] Implement external-resource I/O queue per AC-A-6 (separate from compute queue).
- [ ] Implement deterministic scheduling trace per AC-O-2. Trace projection is part of the report AST per AC-O-4.

### Registry Runtime Scoped Lookup (AC-R-1..AC-R-3, IMPL-FOLLOW-008)

`packages/cem_ml/src/registry/` module exists with three submodules. Scoped lookup and collision diagnostics still
pending.

- [ ] Implement scoped DCE / custom-element registry lookup with parent-scope fallback per AC-R-1 / AC-R-2.
- [ ] Implement collision detection across nested scopes per AC-R-3. Emit `cem.registry.collision` at the policy-
      controlled severity (default warning).
- [ ] Registry runtime AC verification tests: extend `tests/registry_runtime.rs` with inheritance / shadowing /
      collision cases.

### Content-Addressed Cache And Transport (AC-CC-1..AC-CC-9, Tier B)

Shared between `cem-ml` and `cem-ql`. The protocol is normative in `cem-ml-ac.md §14`; implementation is Tier B.

- [ ] Implement deterministic content hashing for parsed top-level artifacts (cem-ml documents, schemas, transform
      plans, cem-ql modules) per AC-CC-1. Hash scheme `cem-bin/1+blake3`.
- [ ] Implement portable binary serialization keyed by AC-CC-1 hash (AC-CC-2). Loader skips parsing when the hash
      matches an in-process or on-disk cache entry.
- [ ] Implement policy stamps (declared schema URIs, plugin imports, external reads, scope-policy fingerprint) per
      AC-CC-3. Mismatch path emits `cem.cc.policy_mismatch`.
- [ ] Implement `dev` / `prod` cache mode axis (AC-CC-4). Dev mode preserves source-map sidecars; prod mode omits them.
- [ ] Implement independently content-addressed source-map sidecars per AC-CC-5.
- [ ] Implement `CEM-Hash` / `If-CEM-Hash` HTTP transport protocol per AC-CC-6 / AC-CC-7.
- [ ] Bind cem-ql's `AC-QC-*` artifact path to the same loader (cem-ql-stack-design-impl.md §12).

## Phase 2 — CLI Fixture Parity And Validation Catalog

[`cem-ml-cli-plan.md`](cem-ml-cli-plan.md) Phase 12 / Phase 13.

- [ ] Build the fixture manifest pairing every `examples/cem-ml/*.cem` with its `examples/semantic/*.html` parity
      fixture. Wire `nx run cem_ml_cli:validate-fixtures` and `cem_ml_cli:e2e`.
- [ ] Add the cross-surface conversion fixtures CLI plan Phase 12 §6 — namespace bindings, comments / whitespace /
      doctypes / PIs / CDATA, anonymous typed scopes, rich-content enclosures, `$` expression nodes, attribute-value
      cem-ql spans, source-map frame preservation.
- [ ] Land the Tier A semantic-validation rule catalog per CLI plan Phase 13: accessible-name requirements, ARIA
      role/attribute compatibility, `id` / `for` / `aria-*` resolution, SVG-in-HTML accessibility boundaries, invalid
      component state combinations, required/forbidden state transitions, reference integrity, schema-owned
      open-content policy, unsafe-content rules.

## Phase 3 — Custom-Element Runtime Preparation (`@epa-wg/cem-components`)

Roadmap: [`../roadmap.md` §Phase 3](../roadmap.md). Component vocabulary: [`component-mvp.md`](component-mvp.md).
Start only when Phase 2 Tier A surfaces are stable enough to consume.

- [ ] Define base CEM custom-element conventions: naming, attributes, events, form participation, validation, loading
      states, progressive enhancement. Land in `packages/cem-components/docs/conventions.md`.
- [ ] Define light-DOM rendering rules and compatibility expectations with `@epa-wg/custom-element` (no shadow DOM).
- [ ] Define the accessibility contract: labels, descriptions, focus, keyboard behavior, roles, live regions.
- [ ] Build the test harness for DOM rendering, events, accessibility assertions, and visual snapshots.
- [ ] Implement minimal primitives: action, field, surface, text, icon, stack, grid, list, nav, dialog shell.

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
