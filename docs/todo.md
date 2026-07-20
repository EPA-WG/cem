# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Close the remaining Phase 2 gaps from [`../roadmap.md`](../roadmap.md) before
continuing Phase 3 substrate expansion. The parser/runtime report-consumer
slice and browser runtime proof are verified, but the full Phase 2 exit
criteria still need stable chunked binary handoff semantics, embedded-payload
handoff coverage, and web-host coordinate projection.

Current active slice: promote the binary projection handoff from sealed-root
debug output to the first stable multi-chunk DOM/AST/events contract.

### Phase 2 Completion Gaps

- [x] Prove the Phase 2 "rendered by the component runtime" exit criterion:
      feed one canonical fixture such as `examples/cem-ml/login.cem` through
      decode/tokenize/normalize/schema/AST/export, then into the
      `@epa-wg/cem-elements` browser runtime, and assert rendered light DOM,
      template/runtime identity, and source-map fidelity in an executable Nx
      target.
- [ ] Promote the DOM/AST/events binary projection handoff from single
      sealed-root chunks and the debug AST encoder to a stable multi-chunk
      contract: subtree chunk metadata, child links, replay-from-cache
      behavior, deterministic routing to multiple sinks, and CEM-QL/query
      access without JSON reserialization.
- [ ] Complete embedded-language handoff coverage for the Phase 2 exit
      criterion: style/script payloads, XML CDATA or schema-tagged text,
      JSON string subdocuments, CSF-like fields, and unsupported/future
      content types must either validate through explicit scoped handoff
      fixtures or emit actionable diagnostics with source-map bounds.
- [ ] Add web-host reporting coordinate projection on top of byte offsets:
      line/column and UTF-16 position derivation should be available to CLI,
      WASM, and browser/devtools consumers without storing non-byte
      coordinates as parser truth.
- [ ] Re-run the Phase 2 completion gate after those gaps close:
      focused Rust tests for the touched layer, `NX_DAEMON=false yarn nx run
      cem_ml:test`, `NX_DAEMON=false yarn nx run cem_ml_cli:test`,
      `NX_DAEMON=false yarn nx run cem_ml_cli:e2e`, and the focused
      `cem-elements` runtime target that proves the browser fixture.

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
- [ ] Verify with the focused `cem-elements` unit target first, then the
      substrate, Storybook, legacy/material parity, demo fixture, and full
      `cem-elements:verify` gates that cover the touched runtime path.

### Next Work Item

Promote the binary projection handoff beyond sealed-root chunks:

- audit `packages/cem_ml/src/projection.rs`,
  `packages/cem_ml/src/ast.rs`,
  `packages/cem_ml/src/ast/{format,encode,decode}.rs`, the
  `cem-{dom,ast,events}-projection` schema packages, and CLI binary export
  and validation tests to identify where the current debug encoder still
  collapses projection output to one root chunk;
- define the first stable multi-chunk contract for one projection kind,
  preserving subtree chunk metadata, child links, hash identity, source-map
  deltas, and compatibility with existing single-root consumers;
- add focused Rust tests for the projection stream and cache-replay behavior
  before wiring CLI/browser consumers, then rerun
  `NX_DAEMON=false NX_ISOLATE_PLUGINS=false yarn nx run cem_ml:test` and the
  relevant CLI validation/e2e targets if binary export surfaces change.

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
