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

Current active slice: verify the executable Phase 2 layered runtime fixture
through the focused Rust tests and the CLI/Nx targets that consume
validate/load/export reports.

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
- [x] Add fixture assertions that every parser-stage report entry carries the
      root-scope input identity, stage name, source-map boundary, and
      diagnostics needed by CLI, WASM, and future host runtimes.
- [ ] Verify the layered runtime fixture with focused Rust tests and the Phase
      2 CLI/Nx targets that consume validate/load/export reports.

### Next Work Item

Verify the layered runtime fixture across the Phase 2 report consumers:

- keep the focused Rust gates green:
  `cargo test -p cem-ml trace_response_embeds_scheduler_projection_in_report_ast`,
  `cargo test -p cem-ml cli_report_json_schema_artifact_matches_contract`, and
  `cargo test -p cem-ml-cli layered_runtime_fixture_connects_parser_stage_contract_through_reports`;
- run the Phase 2 Nx consumers for the report path, starting with
  `NX_DAEMON=false yarn nx run cem_ml:test` and then the relevant
  `cem_ml_cli` test/e2e targets that consume validate/load/export reports;
- triage or fix the currently unrelated `cem_ml_cli:test` failures in legacy
  convert/transform helper cases before marking the verification item closed.

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
