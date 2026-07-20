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

Current active slice: route XSLT 1.0 custom-element compatibility validation
and conversion through the shared lifecycle adapter registry while preserving
scoped handoff diagnostics and source-map boundaries.

### Phase 2 Parser And Runtime

- [ ] Route XSLT 1.0 custom-element compatibility validation and conversion
      through the same lifecycle adapter registry, preserving scoped handoff
      diagnostics and source-map boundaries.
- [ ] Add or update fixtures for CEM-ML, HTML/XML parity, and
      custom-element-XSLT validate/load/export behavior across content-identity
      selection and enum aliases.
- [ ] Verify with focused Rust tests first, then run
      `NX_DAEMON=false yarn nx run cem_ml:test`,
      `NX_DAEMON=false yarn nx run cem_ml_cli:e2e`, and the relevant adapter
      parity target.

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
