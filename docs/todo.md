# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- Adopt the schema package content registry design as the active CEM-ML
  conversion goal:
  [`cem-ml-schema-content-registry-design.md`](cem-ml-schema-content-registry-design.md).
  Use the temporary transition plan in
  [`../packages/cem_ml/docs/schema-content-registry-transition.tmp.md`](../packages/cem_ml/docs/schema-content-registry-transition.tmp.md)
  to migrate the current runtime toward the design.

## Schema Package Implementation List

Implement schema packages for these content families:

- [x] CEM-ML generic document/content model.
- [x] CEM-ML schema definition.
- [x] CEM-ML schema package manifest (`application/vnd.cem.schema-package+cem`, `package.cem`).
- [x] create schema registry
- [x] CEM-ML template.
- [x] CEM-ML transform template (`application/vnd.cem.transform+cem`, `.cemt`).
- [x] use schema registry with transforms for parser/AST stream loading
- [x] CEM-QL module/query resources.
- [x] JSON.
- [x] JSON+JSON schema
- [x] CEM projection artifacts: DOM, AST, and events with primary CEM
      binary/stream encodings and optional JSON debug projections.
- [x] Define semantic DOM/AST/events projection schemas and migrate current
      registry-owned JSON projection exports
      (`https://cem.dev/ns/projection/dom-json/1`,
      `https://cem.dev/ns/projection/ast/1`,
      `https://cem.dev/ns/projection/events/1`) to optional debug/interchange
      views over primary CEM binary/stream artifacts.
- [x] Implement canonical CEM binary/chunk export adapters for
      `application/vnd.cem.dom+cem-bin`, `application/vnd.cem.ast+cem-bin`,
      and `application/vnd.cem.events+cem-bin`.
- [x] Add raw-byte CLI/file output for CEM binary artifacts.
- [x] Add native byte response APIs for CEM binary artifacts.
- [x] Remove JSON envelope dependency from internal binary projection routing;
      keep primary JSON metadata-only and full chunk envelopes
      compatibility/debug-only.
- [x] Implement parallel and multicast-capable projection stream routing over
      sealed CEM binary chunks.
- [x] YAML/YML.
- [ ] CSV.
- [ ] Markdown/MD markup.
- [ ] XML.
- [ ] XHTML.
- [ ] SVG.
- [ ] MathML.
- [ ] XSLT/XSL legacy/custom-element compatibility.
- [ ] HTML.
- [ ] CSS/scoped style content.
# [] custom schema creation instructions
# [] believes schema + registry
stop for sync up with author
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
