# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).
Each item names the AC reference and design home so the closing change ship with a citation.

## Phase 2 — Implementation Tasks (`@epa-wg/cem-ml` / `@epa-wg/cem-ml-cli` / `@epa-wg/cem-ql`)

Acceptance criteria: [`cem-ml-ac.md`](cem-ml-ac.md), [`cem-ql-ac.md`](cem-ql-ac.md). Design homes:
[`cem-ml-stack-design.md`](cem-ml-stack-design.md), [`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md),
[`cem-ql-stack-design.md`](cem-ql-stack-design.md),
[`cem-ql-stack-design-impl.md`](cem-ql-stack-design-impl.md).

### Schema Artifact Emission (AC-S-2..AC-S-6, IMPL-FOLLOW-001)

The only remaining "Non-Match" row in [`cem-ac-design-revalidation.md`](cem-ac-design-revalidation.md): the design still
lacks compiler output detail, and the implementation has no emitter for any release artifact.

- [x] Design and document the schema-compiler output module (location: `packages/cem_ml/src/schema/compiler/`).
      Cover the output struct shapes, file ownership, byte-stability rules, and verification fixtures. Update
      `cem-ml-stack-design.md` and `cem-ml-stack-design-impl.md` with the new section.
      **Closed (2026-05-19):** Design landed in `cem-ml-stack-design.md` §13.2 and
      `cem-ml-stack-design-impl.md` §3.4.2. All open questions (OQ-SC-3, OQ-SC-5, OQ-SC-6, OQ-SC-7, OQ-SC-8)
      resolved the same day — see `cem-ml-stack-design.md` §13.2.9 and
      [`cem-ml-schema-compiler-open-questions.md`](cem-ml-schema-compiler-open-questions.md) (decision archive).
      Emitter PR work is unblocked; see the implementation items below.
- [x] Extend `CompiledSchema` to the `cem-ml-stack-design-impl.md` §3.4 shape before emitter work (AC-S-7, AC-S-8).
      Add `SchemaVersionIdentity`, `CemNativeSchemaSource`, `StructuralSchemaIr`, `SemanticRule`,
      `OpenContentPolicy`, and transform-plan metadata; wire `CompiledSchema::cem_core()` to populate the richer IR.
      This is the first schema-artifact implementation PR, and no emitter PR lands before it.
      **Closed (2026-05-19):** Types live in `packages/cem_ml/src/schema/ir.rs` per the §4 module map;
      `schema/vocab.rs` re-exports for back-compat; `SchemaVersionIdentity` in `schema.rs` replaced with the §3.4
      shape (uri / embedded_version / constraint / match_rule / fingerprint_input). `cem_core()` populates
      all 15 `OpenContentDefaults` branches, one `SchemaStateDef` per annotation, and a `SemanticRule` entry
      for each `RuleRegistry::with_tier_a_rules` registration. Coverage: `schema::ir::tests` (13 tests);
      full workspace `yarn nx run cem_ml:test` green.
- [x] Emit RELAX NG XML mirror (`*.rng`) and RELAX NG compact (`*.rnc`) from `CompiledSchema` (AC-S-2). Add round-trip
      fixtures that read the emitted mirror back through an external validator.
      **Closed (2026-05-20):** `packages/cem_ml/src/schema/compiler/` lands the §3.4.2 module layout
      (`mod.rs`, `output.rs`, `emitter.rs`, `byte_stability.rs`, `error.rs`, `rng_xml.rs`, `rng_compact.rs`).
      `SchemaCompiler::emit_all` produces both artifacts under `core/<version>/cem-core.{rng,rnc}` via the
      shared `DeterministicWriter` (UTF-8, LF, no trailing whitespace, blake3 hash sink). Inline unit tests cover
      byte stability, deterministic ordering, namespace-tail derivation, header policy (OQ-SC-8), enum vs.
      free-form annotations, pass-through host attributes, unknown active-CEM namespace rejection, annotation-scoped
      state lists, and the cem:state matrix. AC-S-2 parity fixtures `tests/schema_emit/rng_xml_parity.rs` and
      `tests/schema_emit/rng_compact_roundtrip.rs` spawn `xmllint --relaxng` / Trang when available and skip with an
      info record under `CEM_ML_SCHEMA_PARITY_SKIP=1` or when the external tools are absent (OQ-SC-5 escape hatch).
      Non-streamable constraints now raise `EmitError::UnsupportedConstraint` before emitter output. `blake3 = "1"`
      added to `Cargo.toml`. `yarn nx run cem_ml:test` green.
- [x] Emit TypeScript `.d.ts` headers from `CompiledSchema` (AC-S-3, AC-S-6). Structural by default; `Validated<T>`
      wrapper opt-in per `MEMORY.md` (`project_ts_emit_strategy.md`).
      **Closed (2026-05-20):** `packages/cem_ml/src/schema/compiler/ts_dts.rs` lands the AC-S-3 / AC-S-6 emitter.
      Header per OQ-SC-8 (URI + version only, no hash). `asValidated` / `tryValidated` are re-exported from
      `@epa-wg/cem-ml/wasm` per OQ-SC-6; local `Validated<T>` imports the WASM brand and intersects it with a
      per-schema-version `unique symbol` brand so `Validated<Badge@1.0.0>` is not assignable to
      `Validated<Badge@2.0.0>` while still flowing as `Badge` / `HTMLElement`. Structural-by-default callers can drop
      the brand block via `include_validated_brand = false`. One `export interface {Pascal(annotation)} extends
      HTMLElement` per cem-core/1 annotation with a `readonly cem{Annotation}?` field (literal-union for enum-typed,
      `string` for free-form) and a `readonly cemState?` field reflecting `AnnotationDef.allowed_states`.
      Per-version on-disk path `core/1.0.0/cem-core.d.ts` (OQ-SC-7). Inline tests cover byte stability, header policy,
      brand-block gating, every annotation interface, enum vs free-form value typing, per-annotation state union,
      kebab→Pascal naming, and the LF/no-trailing-whitespace invariants. `tests/schema_emit/ts_dts_structural.rs` and
      `tests/schema_emit/ts_dts_validated_brand.rs` compile real `tsc --noEmit` fixtures. `yarn nx run cem_ml:test`
      green.
- [x] Emit Rust `.rs` headers from `CompiledSchema` (AC-S-4). Verify the generated module compiles with `cargo check`.
      **Closed (2026-05-20):** `packages/cem_ml/src/schema/compiler/rust_hdr.rs` ships per OQ-SC-3 (Tier A code,
      Tier B gate): `emit_all` invokes it only when `CompilerOptions.emit_rust = true` (default `false`). Output
      is a `pub mod schema { ... }` block with `SCHEMA_URI` / `EMBEDDED_VERSION` consts, one
      `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub enum {Pascal(annotation)}` per enum-typed annotation
      (`Action`, `Badge`, `Message` for cem-core/1), and a `pub enum CemState` carrying the schema-wide state
      matrix. Free-form annotations emit no enum (call-site type is `&str`). Header policy per OQ-SC-8 (URI +
      version, no hash); per-version path `core/1.0.0/cem-core.rs` per OQ-SC-7. 16 inline tests cover byte
      stability, header policy, per-annotation enum shapes, no-enum-for-free-form, CemState variants, brace
      balance, and the LF / no-trailing-whitespace invariants. The implementation intentionally does not emit
      host-bound structs in the Tier A gated subset; that surface remains a Tier B expansion. Verification fixture
      `tests/schema_emit/rust_hdr_compiles.rs` writes a stub `cem_ml_schema_stub` crate, includes the emitted
      `.rs`, and spawns `cargo check --offline`; gated by `CEM_ML_EMIT_RUST=1` per OQ-SC-3 (skipped with `info`
      in Tier A CI; exercised locally — `cargo check` succeeds against the emitted module). Full
      `yarn nx run cem_ml:test` green.
- [x] Publish schemas under stable URIs (AC-S-5). Define the URI scheme, byte-stability fixture, and the publication
      workflow (manifest, version, hash sidecar).
      **Closed (2026-05-21):** `packages/cem_ml/src/schema/compiler/uri_publish.rs` lands the AC-S-5 surface.
      `emit_manifest_artifact` projects `PublicationManifest` into a byte-stable `manifest.json` through the shared
      `DeterministicWriter` — encoding fixed by the new `cem-ml-stack-design.md` §13.2.11 (§13.2.3 wire order;
      `artifacts` keyed by `ArtifactKind` ordinal in kebab-case; `content_hash` as `{scheme}:{hex}`); `emit_all`
      appends it as the final artifact. `SchemaCompiler::write_to_disk` now writes the §13.2.5 tree — every artifact, a
      `<path>.hash` sidecar (body `cem-bin/1+blake3:<hex>\n`), and `manifest.json` last — each through a
      temp-then-rename adapter. `parse_schema_uri` / `resolve_uri` implement AC-V-10 URI-tail matching against
      published manifests and return a `UriResolution` carrying the fired match rule for an AC-V-13
      `cem.v.semver_resolved` event. Per the resolved semver decision (design §13.2.6; AC-V-10 unversioned bullet
      clarified), pre-release embedded versions are excluded from `unconstrained` / `major` / `major-minor` / `full`
      matches — reachable only through a `prerelease-exact` URI. Verification fixtures
      `tests/schema_emit/byte_stability.rs` (AC-S-2 — emit twice, byte- and hash-identical across every artifact incl.
      the manifest; LF/encoding invariants) and `tests/schema_emit/uri_manifest_resolution.rs` (AC-S-5 / AC-V-10 — the
      full resolution table plus `write_to_disk` on-disk-tree and sidecar-digest checks), with 13 inline `uri_publish`
      unit tests. `cem_ml::loader` — the document-loading caller named in §13.2.6 — does not exist yet; `resolve_uri`
      is the runnable AC-S-5 resolution surface per impl §3.4.2.1. The last `cem-ac-design-revalidation.md` "Non-Match"
      row is now resolved. `yarn nx run cem_ml:test` green (347 lib + 20 schema_emit).
- [ ] Add `nx run cem_ml:build:schema-artifacts` Nx target that runs all four emitters and writes outputs under
      `packages/cem_ml/dist/lib/schema/`.

### CEM-QL Tier A Implementation (`packages/cem_ql`)

Two design docs landed; no Rust crate exists yet. Crate boundary, module map, and AC mapping live in
[`cem-ql-stack-design-impl.md`](cem-ql-stack-design-impl.md) §3.

- [ ] Bootstrap `packages/cem_ql` crate (`Cargo.toml`, `project.json`, `src/lib.rs` with layered-contract import test
      per impl design §3.2). Nx targets: `build`, `test`, `lint`, `build:wasm`, `bench`, `test:xpath-parity`,
      `test:fixtures`. Reuse `cem_ml::benchmark::BenchmarkBudget` for the bench target.
- [ ] Implement L1 lexer (`cem_ql::lexer`) per impl §4 — token table, DFA-style scanner, `&&` / `||` reserved-form
      diagnostic emission. Tests: `parser_recovery.rs` covers lexer surface.
- [ ] Implement L2 parser (`cem_ql::parser`) per impl §5 — hand-written recursive descent for module/declare/import,
      Pratt expression sub-grammar, three-point recovery synchronization (statement, pipeline step, bracket).
- [ ] Implement L3 name resolver (`cem_ql::resolve`) per impl §6 — `BindingSet` chain, per-scope stdlib overlay,
      reserved-scheme guard, resolution-trace events for AC-QV-V-1.
- [ ] Implement L4 type checker (`cem_ml::types`) per impl §7 — bidirectional inference, structural subtype walk,
      cross-type comparison warning, strict-default + dev-profile failure contract (AC-QT-3).
- [ ] Implement L5 IR lowerer (`cem_ql::ir`) per impl §9 — typed `IrNode` enum, closure-detachment pass.
- [ ] Implement L6 evaluator (`cem_ql::eval`) per impl §10 — pull-based `ItemStream`, pipeline iterator chains,
      streaming `Union` and bounded-buffer `Intersect`/`Difference`/`SymmetricDifference`, budget charging
      (AC-QR-1 / AC-QR-2).
- [ ] Implement Tier A stdlib modules per impl §11: `cem:stdlib/sequence`, `strings`, `numbers`, `datetime`, `dom`,
      `report`, `state`, `template`, `cemml`. Each function listed in the §11 tables.
- [ ] Diagnostic table (`cem_ql::diagnostics`) per impl §8 — all Tier A codes plumbed through `cem_ml::report`.
- [ ] Verification scripts per AC §13:
      - [ ] `cem_ql:test` — unit coverage for L1..L6 + stdlib.
      - [ ] `cem_ql:test:xpath-parity` — XPath 3.1 conformance subset (AC-QX-1).
      - [ ] `cem_ql:test:fixtures` — Tier A query corpus against canonical CEM-ML fixtures.
      - [ ] `cem_ql:bench` — selector benchmark sharing `cem_ml::benchmark::BenchmarkBudget`.
      - [ ] AC-QV-V-1 — three-case scope-inheritance/overlay test.
      - [ ] AC-QO-V-1 — set-operator identity test (node/structured, strict-typed atom, explicit-conversion uniformity,
            cross-type comparison warning).
      - [ ] AC-QI-V-1 — import-gating test (one case per scheme tier).
- [ ] Wire `cem_ml_cli` to invoke cem-ql for `select=` / `match=` / `test=` template attributes and `{$ … }` content
      expressions per AC-T-7.

### Observability Public API And `byte_offset` Projection (AC-P-3, AC-O-1, IMPL-FOLLOW-003)

`onParseEvent` / `onValidate` / `onTransform` already exist (`packages/cem_ml/src/observability.rs`). `byte_offset` is
on `Diagnostic`. What's missing is the cross-host public surface and projection coverage.

- [ ] Define the WASM-callable observer surface in `packages/cem_ml/src/api/wasm.rs` (new module). Use `wasm-bindgen`
      to expose `onParseEvent` / `onValidate` / `onTransform` registration to JS callers (AC-C-1 browser/Node parity).
- [ ] Confirm `byte_offset` is the canonical top-level projection on every report node — not just `Diagnostic`. Add
      AC-P-3 verification fixtures that drive each layer's event through serialization and assert the field is present.
- [ ] Document the public observer payload schema in `cem-ml-stack-design-impl.md §3.12`; add JSON schema fixtures
      under `packages/cem_ml/schema/observability/`.

### Document-Format Directive `@doc` Hardening (AC-F-8, IMPL-FOLLOW-001B)

`@doc cem-ml 1` parsing is implemented. The SemVer constraint resolution, required-directive enforcement, and
AC-F-V-6 diagnostic coverage are not yet wired through.

- [ ] Add SemVer constraint resolution for `@doc cem-ml <constraint>` per AC-F-8. Reject unsupported majors with a hard
      diagnostic.
- [ ] Enforce required top-level `@doc` directive on every CEM-ML root document; emit `cem.doc.missing_directive` on
      omission. Embedded fragments inherit format identity per AC-F-8.
- [ ] Land AC-F-V-6 diagnostic fixtures covering missing, malformed, and version-mismatched directives.

### Tokenizer Lowering Test Coverage (AC-F-9, AC-P-1, AC-P-8, IMPL-FOLLOW-001A)

Canonical curly tokenizer exists; concrete lowering coverage per the AC list is incomplete.

- [ ] Add tokenizer-lowering tests for: `{name @attributes | content...}`, `$` expression nodes, anonymous typed
      scopes, comments (line and block), rich-content enclosures, and rejection of bare `{...}` text interpolation.
      File location: `packages/cem_ml/tests/tokenizer_lowering.rs` (new).

### Inline Schema And Mid-Document Schema Switch (AC-F-2, IMPL-FOLLOW-004)

Schema scoping is resolved at the AC level (`MEMORY.md` `project_schema_scoping.md`); parser/schema-frame lowering for
the in-document forms still needs implementation.

- [ ] Implement parser and schema-frame lowering for inline `{cem:schema @cem:name | ... }` declarations.
- [ ] Implement `cem:schema-src` / `cem:schema-select` host-attribute switches.
- [ ] Implement scope-chain `cem:name` shadowing per `cem-ml-stack-design.md §13.1`. Add fixtures covering shadowing,
      sibling isolation, and override boundaries.

### Plugin Runtime (AC-PL-1..AC-PL-20, IMPL-FOLLOW-006)

`packages/cem_ml/src/plugin/` module exists; descriptor, chain, lifecycle, and sandboxing per the resolved
host-trusted + Rust AST capability validator model (`MEMORY.md` `project_plugin_sandboxing.md`) need to be plumbed
through.

- [ ] Implement plugin descriptor, chain composition, install/uninstall lifecycle, observe/mutate mode separation, and
      priority ordering.
- [ ] Implement source-map stitching across plugin boundaries (host-frame inheritance + plugin-introduced frame).
- [ ] Implement Rust AST capability validator at load time per the resolved sandboxing model.
- [ ] Add plugin-budget enforcement against the scope policy. Emit `cem.plugin.budget_exceeded` on breach.
- [ ] Plugin runtime AC verification tests: `tests/plugin_runtime.rs` already exists — extend it with descriptor /
      chain / source-map-stitching / sandbox cases.

### Scheduler Completion (AC-A-2..AC-A-7, AC-O-2, IMPL-FOLLOW-007)

`packages/cem_ml/src/scheduler/` module exists with six submodules. Worker pool / bounded queue / cancellation /
deterministic trace still pending.

- [ ] Implement per-scope thread pool and bounded queue per AC-A-4 / AC-A-5. Queue overflow emits
      `cem.scheduler.queue_full`; the diagnostic carries the overflowing scope id.
- [ ] Implement end-to-end `AbortSignal` propagation per AC-A-7. Cancellation halts in-flight work at the next
      safe-point and surfaces `cem.scheduler.aborted` with the originating cancel-site source-map stack.
- [ ] Implement external-resource I/O queue per AC-A-6 (separate from compute queue).
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

## Phase 2 — Documentation And AC Cleanup

[`cem-ml-syntax-alignment-report.md`](cem-ml-syntax-alignment-report.md) has four outstanding items.

- [ ] **M-1** Add a `Template Elements` subsection to `cem-ml-syntax.md` showing the Tier A CEM-ML forms for
      `cem:value-of`, `cem:for-each`, `cem:if`, `cem:choose` / `cem:when`, and `cem:variable`. Cross-reference AC-T-7
      and `cem-ql-ac.md`.
- [ ] **M-2** Add AC text in `cem-ml-ac.md` covering CEM-ML, XML, and HTML comment delimiters; whether comments are
      AST-preserved by default; and the policy hook that can strip or retain them. Update AC-P-9.
- [ ] **M-3** Add tier tags (`[A]` / `[B]` / `[C]`) to syntax sections and table rows in `cem-ml-syntax.md`. Keep them
      in sync with `cem-ml-ac.md`.
- [ ] **M-4** Decide on Unicode-profile delimiters: either land them in an AC with tier tags and tokenizer tests, or
      mark them explicitly experimental / non-AC syntax in `cem-ml-syntax.md`.

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

## Recently Closed

- AC-N-* perf benchmark harness and policy. Budget ownership in `cem_ml::benchmark::BenchmarkBudget`, CI tolerance via
  `CEM_ML_PERF_TOLERANCE`, 10 MB and depth-200 proof fixtures in `packages/cem_ml/tests/perf_budgets.rs`, Nx entry
  point `yarn nx run cem_ml:bench`. Documented in `cem-ml-stack-design.md §17`.
- AC-C-* compatibility / distribution gates. Support matrix, crate surface, CLI boundary, release-check sequence
  documented in `cem-ml-stack-design.md §18`.
- CEM-QL stack design. `cem-ql-stack-design.md` (high-level: pipeline layers, grammar, evaluator IR, type system,
  stdlib module layout, cost model, binary artifact layout) and `cem-ql-stack-design-impl.md` (concrete Rust module
  map, surface AST, IR shapes, diagnostic table, stdlib function tables).
