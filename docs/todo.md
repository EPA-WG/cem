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

Current active slice: fill the remaining lifecycle fixture matrix for
content-identity selection and compatibility aliases after the
`custom-element-xslt-compat` adapter profile promotion.

### Phase 2 Parser And Runtime

- [x] Route XSLT 1.0 custom-element compatibility validation and conversion
      through the same lifecycle adapter registry, preserving scoped handoff
      diagnostics and source-map boundaries.
- [ ] Add remaining fixtures for CEM-ML, HTML/XML parity, and
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

### Next Work Item

Build the remaining fixture matrix around normalized CLI/run-config inputs:

- prove CEM-ML, HTML, XML, XHTML, SVG, MathML, and XSLT identities select the
  intended lifecycle adapters from content type first, then schema, then
  namespace;
- cover alias fallback behavior for `--from-format` / `--to-format` alongside
  explicit content identity, including unsupported explicit identity
  diagnostics;
- add report assertions that lifecycle adapter/profile details survive validate
  and convert reports for `custom-element-xslt-compat`;
- prefer CLI fixtures backed by `RunConfig`/`--input-spec` over only direct
  `EngineInput` unit tests, because the remaining risk is normalization drift.

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
