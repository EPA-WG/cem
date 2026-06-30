# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- [ ] Implement schema package loading and input-file validation for supported
      schemas, in schema package creation order. Definition of done for each
      schema: resolve content type to schema URL, load the schema package,
      select an explicit lifecycle parser/adaptor, validate source bytes
      against the schema-owned document model, surface diagnostics through
      `validate`/`check`, and add focused Rust coverage.
  - [x] Establish schema-owned validation examples and a reusable CLI fixture
        harness before implementing per-schema validators.
        For each schema package, add a few popular real-world use cases as
        checked-in example files under
        `packages/cem_ml/schema-packages/{schema-name}/v1/examples/`.
        Link those files from that schema package's `README.md`, document the
        matching CLI validation command, and use the same example files in CLI
        validation tests for that file type.
  - [x] Organize CLI validation coverage so schema sub-projects can reuse it:
        keep CLI argument-plumbing tests in `packages/cem_ml_cli/src/dispatch.rs`,
        move schema example validation into a table-driven integration test
        such as `packages/cem_ml_cli/tests/schema_validation_examples.rs`, and
        have that suite read schema-owned examples instead of duplicating inline
        `write_fixture` strings.
  - [x] Define the example fixture contract: each schema starts with at least
        valid basic, valid realistic/nested, and invalid diagnostic examples;
        every example declares expected content type, schema URL, validation
        command, expected pass/fail result, and expected diagnostic codes when
        failing.
  - [x] CEM-ML generic document/content model (`application/cem`).
  - [x] CEM-ML schema definition
        (`application/vnd.cem.schema+cem`).
  - [x] CEM-ML schema package manifest
        (`application/vnd.cem.schema-package+cem`, `package.cem`).
  - [x] CEM-ML native template
        (`application/vnd.cem.template+cem`).
  - [x] CEM-ML transform template
        (`application/vnd.cem.transform+cem`, `.cemt`).
  - [x] CEM-QL module/query resources
        (`application/vnd.cem.query+cem-ql`, `text/cem-ql`).
  - [x] JSON (`application/json`, `text/json`).
  - [x] JSON Schema (`application/schema+json`).
  - [ ] CEM DOM projection
        (`application/vnd.cem.dom+cem-bin`,
        `application/vnd.cem.dom+json` debug view).
  - [ ] CEM AST projection
        (`application/vnd.cem.ast+cem-bin`,
        `application/vnd.cem.ast+json` debug view).
  - [ ] CEM events projection
        (`application/vnd.cem.events+cem-bin`,
        `application/vnd.cem.events+json` debug view).
  - [ ] YAML/YML (`application/yaml`, compatibility aliases).
  - [ ] CSV (`text/csv`).
  - [ ] Markdown/MD markup (`text/markdown`).
  - [ ] XML (`application/xml`, XML aliases).
  - [ ] XHTML (`application/xhtml+xml`).
  - [ ] SVG (`image/svg+xml`).
  - [ ] MathML (`application/mathml+xml`, presentation/content aliases).
  - [ ] XSLT/XSL legacy/custom-element compatibility
        (`application/xslt+xml`, `text/xsl`, custom-element aliases).
  - [ ] HTML (`text/html`).
  - [ ] CSS/scoped style content (`text/css`).

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
- [x] CSV.
- [x] Markdown/MD markup.
- [x] XML.
- [x] XHTML.
- [x] SVG.
- [x] MathML.
- [x] XSLT/XSL legacy/custom-element compatibility.
- [x] HTML.
- [x] CSS/scoped style content.
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
