# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Advance Phase 2 from [`../roadmap.md`](../roadmap.md): schema-defined parser
and document runtime. Focus first on the shared run configuration and
multi-document lifecycle layer that lets `cem_ml`, `cem_ml_cli`, WASM, and
future hosts validate, load, and export documents through the same root-scope
context.

Current active slice: enrich the executable Phase 2 layered runtime fixture so
parser-stage report entries carry root-scope input identity, source-map
boundary, and diagnostics through the scheduler-visible report path.

### Phase 2 Parser And Runtime

- [x] Route XSLT 1.0 custom-element compatibility validation and conversion
      through the same lifecycle adapter registry, preserving scoped handoff
      diagnostics and source-map boundaries.
- [x] Add remaining fixtures for CEM-ML, HTML/XML parity, and
      custom-element-XSLT validate/load/export behavior across content-identity
      selection and enum aliases. Start by expanding CLI-normalized
      `--input-spec` / `--output-spec` cases so content type, schema,
      namespace, root-scope identity, and legacy enum hints are exercised
      through the same normalized run path rather than only direct engine
      request construction.
- [x] Verify with focused Rust tests first, then run
      `NX_DAEMON=false yarn nx run cem_ml:test`,
      `NX_DAEMON=false yarn nx run cem_ml_cli:e2e`, and the relevant adapter
      parity target.
- [x] Define the executable layered parser/runtime contract for one semantic
      document fixture, covering byte source, tokenizer, normalized event
      stream, schema validation, AST/source-map construction, and export
      reporting through the same scheduler trace.
- [ ] Add fixture assertions that every parser-stage report entry carries the
      root-scope input identity, stage name, source-map boundary, and
      diagnostics needed by CLI, WASM, and future host runtimes.
- [ ] Verify the layered runtime fixture with focused Rust tests and the Phase
      2 CLI/Nx targets that consume validate/load/export reports.

### Next Work Item

Add parser-stage report-entry assertions for the layered runtime fixture:

- inventory the current gap exposed by
  `layered_runtime_fixture_connects_parser_stage_contract_through_reports`:
  trace entries have ordered parser stage names and scope IDs, validate/convert
  reports have input-prefixed load/export tasks and output identity, but no
  parser-stage report entry yet carries root-scope content identity,
  source-map boundary, and per-stage diagnostic codes together;
- extend the smallest existing report surface, preferably
  `reportAst.schedulerTrace.events` or an adjacent `reportAst` parser-stage
  detail, instead of adding a parallel trace path;
- update the fixture assertions so tokenize, normalize, schema, AST, and
  validate entries all expose the same root-scope input identity, a non-empty
  source-map boundary, and the diagnostics array needed by CLI, WASM, and
  future host runtimes.

## Current Verification Commands

- `yarn nx run @epa-wg/cem-theme:verify:phase13`
- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`
- `yarn nx run cem-elements:verify-edge-ssr`
- `yarn nx run @epa-wg/custom-element:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
