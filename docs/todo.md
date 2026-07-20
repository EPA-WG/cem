# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Advance Phase 3 from [`../roadmap.md`](../roadmap.md): establish the
`@epa-wg/cem-elements` browser substrate for the `<cem-element>` declarative
authoring tag before expanding the component catalog. Focus first on the
runtime contracts that consume the Phase 2 parser/report spine: declaration
template loading, inert data-island isolation, light-DOM rendering, source-map
fidelity, and executable parity fixtures.

Current active slice: audit the existing `cem-elements` substrate targets and
lock the first runtime-facing contract gap around inline declaration templates
and instance data-island isolation.

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
- [x] Verify the layered runtime fixture with focused Rust tests and the Phase
      2 CLI/Nx targets that consume validate/load/export reports.

### Phase 3 Custom-Element Runtime

- [ ] Audit the existing `@epa-wg/cem-elements` substrate implementation,
      parity inventories, Storybook fixtures, and Nx verification targets
      against the Phase 3.1 exit criteria from `roadmap.md`.
- [ ] Define the first executable browser substrate contract for inline
      `<cem-element>` declarations: exactly one inert WHATWG declaration
      template, produced custom-element registration, captured instance
      data-island payload, visible light-DOM render ownership, and source-map
      fidelity labels.
- [ ] Add focused unit/browser fixtures for declaration registration, payload
      capture, attribute invalidation, and data-island isolation using the
      existing `cem-elements` test harness.
- [ ] Wire one Phase 2 CEM-ML/CEMT parser-output fixture into the
      `cem-elements` runtime verification path so parser reports and rendered
      light DOM are checked together.
- [ ] Verify with the focused `cem-elements` unit target first, then the
      substrate, Storybook, legacy/material parity, demo fixture, and full
      `cem-elements:verify` gates that cover the touched runtime path.

### Next Work Item

Start Phase 3 by auditing and tightening the browser substrate contract:

- inspect `packages/cem-elements/src/lib`, `packages/cem-elements/tests/parity`,
  `packages/cem-elements/docs`, and `tools/scripts/verify-cem-elements-*.mjs`
  against `docs/cem-element-design.md` and the Phase 3.1 roadmap exit criteria;
- run the baseline gates most likely to expose the first substrate gap:
  `NX_DAEMON=false yarn nx run cem-elements:test:unit`,
  `NX_DAEMON=false yarn nx run cem-elements:verify-substrate`, and the focused
  `verify-legacy-fixtures` / `verify-material-fixtures` targets as needed;
- implement the smallest missing contract around inline declaration template
  shape, inert declaration source, instance data-island capture, or light-DOM
  render ownership, then expand verification only to the touched runtime path.

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
