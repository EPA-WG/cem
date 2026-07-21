# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Phase 2 exit criteria from [`../roadmap.md`](../roadmap.md) are closed: the
parser/runtime report-consumer slice, browser runtime proof, stable chunked
binary handoff, embedded-payload handoff coverage, and web-host coordinate
projection have all passed their focused and gate verification. Continue with
Phase 3 substrate expansion through the schema-package registry surface.

Current active slice: make `csv/v1` the proving slice for schema-owned source
semantics and CEMT-owned output stages before returning to the wider built-in
schema-package folder alignment.

### Immediate Execution Phase: CSV Schema And CEMT Ownership

- [x] Move CSV parsing and validation ownership into
      `packages/cem_ml/schema-packages/csv/v1/schema/csv.cem`: model row,
      field, quote, field-count, charset, and `header` parameter contracts as
      schema-declared constraints/diagnostics instead of package-specific Rust
      diagnostics.
- [x] Convert `csv/v1` to an Nx library with `*.cemt` sources tracked for
      caching; CLI tests should depend on the package target and invoke it
      through Nx.
- [x] Replace the current Rust-backed CSV validation path in
      `packages/cem_ml_cli/src/dispatch.rs` with a generic schema-package
      validation path that executes the CSV schema contracts and keeps CLI
      diagnostics byte/source-map aware.
- [ ] Add focused package examples and tests proving valid CSV, quoted fields,
      ragged rows, unclosed quotes, invalid quote escapes, unsupported charset,
      US-ASCII byte mismatch, and invalid `header` metadata are all driven by
      schema-owned contracts.
- [ ] Make CSV formatter profiles `compact`, `pretty`, and `tabular`
      executable through the CEMT assets in
      `packages/cem_ml/schema-packages/csv/v1/formatters/`, not Rust string
      formatting.
- [ ] Make CSV colorizer profiles `terminal`, `html`, and `md` executable
      through the CEMT assets in
      `packages/cem_ml/schema-packages/csv/v1/colorizers/`, including
      source-map-safe token/color metadata for writer output.
- [ ] Verify the CSV ownership slice with focused schema-package tests, the CLI
      schema-owned example validation, the CEMT pipeline fixture, and
      `yarn nx run cem_ml:test` before resuming sibling package alignment.
- [ ] Convert `schema-packages/*/v1` folders to Nx libraries with `*.cemt`
      sources tracked for caching; CLI tests should depend on package targets
      and invoke them through Nx.
Implementation gaps to close during this slice:

- [x] Define the schema-facing CSV parse-report nodes and fact vocabulary in
      `schema/csv.cem`, including the exact source metadata, row, field,
      parse-fact, and source-map fields exposed by the host parser behavior.
- [x] Add a generic host behavior hook for CSV parse fact extraction that
      returns neutral facts and never chooses `cem.csv.*` diagnostic codes or
      severities.
- [x] Teach the schema validation runtime how a non-CEM source such as
      `text/csv` enters schema-owned contract evaluation without first becoming
      a CEM AST document.
- [x] Add behavior/constraint bindings that map CSV parse facts to package-owned
      diagnostics, starting with one narrow fact-to-diagnostic path before
      moving every CSV diagnostic.
- [x] Add mutation-style tests proving a changed `schema/csv.cem` diagnostic or
      behavior binding changes CSV validation output, so the ownership boundary
      is testable.
- [x] Preserve CLI JSON compatibility while moving diagnostic provenance into
      schema-owned structured details (`contract`, `behavior`, `factKind`,
      source range, media type, row/field indices, expected/actual values).

Started implementation:

- [x] Migrated the `text/csv; header=...` validation path to a neutral
      `invalid-header-parameter` parse fact, with `cem.csv.invalid_header_parameter`
      code and severity read from `csv/v1/schema/csv.cem`.
- [x] Added schema-owned diagnostic provenance for that first path in CLI JSON
      details (`contract`, `behavior`, `factKind`, media type, expected, actual,
      and byte length).
- [x] Moved the remaining current CSV diagnostics out of
      `packages/cem_ml_cli/src/dispatch.rs`: unsupported charset, UTF-8 decode
      failures, US-ASCII byte mismatch, invalid quote escapes, unclosed quotes,
      ragged rows, and parser fallback errors now flow through neutral parse
      facts and schema-declared diagnostic bindings in
      `packages/cem_ml/src/validation/csv.rs`.
- [x] Reduced schema-validation CLI test runtime by exposing in-process CLI
      dispatch for integration tests, grouping schema-owned example validation
      by content type/schema/result, splitting schema-package-heavy cases into
      focused package/contract tests, and making recursive schema-package
      manifest self-validation an explicit ignored check.
- [x] Added schema-package contract validation caching for built-in registry
      reuse, package resource reads, and CEMT module parse results so repeated
      formatter/colorizer artifact checks avoid duplicated work inside a rule
      run.
- [x] Registered `packages/cem_ml/schema-packages/csv/v1` as the first
      cacheable schema-package Nx library, tracking its manifest, schema,
      formatter/colorizer CEMT assets, and examples, with `cem_ml_cli:test`
      depending on the package `verify` target through Nx.
- [x] Added a generic `cem_ml::validation::schema_package_source` entry point
      for schema-package-owned non-CEM sources and routed CLI CSV validation
      through it, preserving schema-owned contract/fact details and byte,
      line, and column source-map fields.

### Immediate Execution Phase: Schema Package Folder Alignment

Observed package state:

- every built-in package already has `package.cem`, `README.md`, `schema/`, and
  `examples/`;
- `cem-ml/v1/schema/cem-ml-generic.cem` and
  `schema/v1/schema/cem-schema.cem` are the only schema-source filename
  exceptions to the README shape `schema/{package-id}.cem`;
- `schema`, `schema-package`, `cem-ast-projection`,
  `cem-events-projection`, and `cem-dom-projection` lack package-owned
  formatter/colorizer stage directories or baseline manifest artifacts;
- no package currently has `examples/*.example.cem` sidecars; example contracts
  are held in manifest metadata, so the execution pass must either generate the
  sidecars or codify manifest metadata as the accepted README-compatible
  representation before per-package cleanup starts.

Contract gates for this slice:

- [ ] Add a schema-package structure audit that walks every
      `schema-packages/{package-id}/v1` folder and reports `package.cem`,
      `README.md`, manifest schema source, `examples/`, package-owned CEMT
      artifact paths, baseline formatter profiles
      `compact`/`pretty`/`tabular`, baseline colorizer profiles
      `terminal`/`html`/`md`, and converter template paths only when the
      manifest declares CEMT converters.
- [ ] Decide and encode the example-reference representation: generate
      checked-in `examples/*.example.cem` sidecars from current manifest example
      entries, or update the audit to explicitly accept manifest-owned example
      entries as the equivalent reference document described by the README.
- [ ] Add focused checks for the two schema-source filename exceptions so
      `cem-ml-generic.cem` and `cem-schema.cem` are intentional, or rename them
      and update their manifests/readmes to the literal `schema/{package-id}.cem`
      shape.
- [ ] Keep converter endpoint checks as a final registry pass because current
      manifests contain cross-package edges (`cem-ml` to projections, `xml` to
      DOM projection, and DOM projection back to HTML/XML) that should not force
      a false per-folder dependency cycle.

Dependency-ordered package checklist:

- [ ] `cem-ml/v1` (root CEM-ML syntax): align the schema-source filename
      decision, keep the existing formatter/colorizer helper artifacts as the
      canonical baseline, normalize example references, and leave projection
      converter endpoint validation for the final registry pass.
- [ ] `schema/v1` (uses `cem-ml`): align the schema-source filename decision,
      add/register package-owned formatter and colorizer stages or a documented
      bootstrap exception, and normalize its schema-definition examples.
- [ ] `schema-package/v1` (uses `schema`): add/register package-owned formatter
      and colorizer stages or a documented bootstrap exception, then normalize
      the flat and nested manifest examples under `examples/`.
- [ ] `cem-native-template/v1` (uses `schema`, `cem-ml`): verify the existing
      formatter/colorizer baseline artifacts, then normalize manifest/readme
      example references.
- [ ] `cem-transform/v1` (uses `schema`, `cem-native-template`): verify the
      existing CEMT formatter/colorizer baseline artifacts, then normalize the
      formatter/coloring pipeline examples and expected diagnostics.
- [ ] `cem-ql/v1` (uses `schema`): verify output-stage artifacts for the CEM-QL
      source/parser boundary and normalize query examples.
- [ ] `cem-ast-projection/v1` (uses `schema`, `cem-ml`): add/register
      formatter/colorizer stage assets or an explicit binary-projection
      exception, then normalize binary and JSON debug examples.
- [ ] `cem-events-projection/v1` (uses `schema`, `cem-ml`): add/register
      formatter/colorizer stage assets or an explicit binary-projection
      exception, then normalize binary and JSON debug examples.
- [ ] `json/v1` (uses `schema`): verify existing formatter/colorizer baseline
      artifacts and normalize JSON examples.
- [ ] `json-schema/v1` (uses `json`): verify existing formatter/colorizer
      baseline artifacts and normalize JSON Schema examples.
- [ ] `yaml/v1` (uses `schema`, `cem-ml`): verify existing
      formatter/colorizer baseline artifacts and normalize YAML examples.
- [ ] `csv/v1` (uses `schema`, `cem-ml`): finish the active CSV schema/CEMT
      ownership slice above, keep the README-shaped formatter/colorizer asset
      layout, and normalize CSV examples.
- [ ] `markdown/v1` (uses `schema`, `cem-ml`): verify existing
      formatter/colorizer baseline artifacts and normalize Markdown examples.
- [ ] `xml/v1` (uses `schema`, `cem-ml`): verify existing formatter/colorizer
      baseline artifacts, normalize XML examples, and defer the XML-to-DOM
      converter endpoint check to the final registry pass.
- [ ] `relax-ng/v1` (uses `xml`): verify existing formatter/colorizer baseline
      artifacts and normalize XML/compact-syntax examples.
- [ ] `xhtml/v1` (uses `xml`): verify existing formatter/colorizer baseline
      artifacts and normalize XHTML examples.
- [ ] `svg/v1` (uses `xml`): verify existing formatter/colorizer baseline
      artifacts and normalize SVG examples.
- [ ] `mathml/v1` (uses `xml`): verify existing formatter/colorizer baseline
      artifacts and normalize MathML examples.
- [ ] `xslt/v1` (uses `xml`): verify existing formatter/colorizer baseline
      artifacts and normalize XSLT plus legacy custom-element compatibility
      examples.
- [ ] `html/v1` (uses `svg`, `mathml`, `schema`, `cem-ml`): verify existing
      formatter/colorizer baseline artifacts and normalize HTML examples,
      including SVG/MathML island coverage.
- [ ] `cem-dom-projection/v1` (uses `schema`, `cem-ml`; converter endpoints
      target `html` and `xml`): add/register package-owned formatter/colorizer
      stage assets or an explicit binary-projection exception, verify
      `converters/dom-to-html.cemt` and `converters/dom-to-xml.cemt` stay under
      `converters/`, and normalize binary/JSON debug examples.
- [ ] `css/v1` (uses `html`, `svg`, `mathml`, `schema`, `cem-ml`): verify
      existing formatter/colorizer baseline artifacts and normalize stylesheet,
      scoped-style, and style-attribute examples.
- [ ] Run the final registry/package validation gate after the dependency
      checklist is green:
      `yarn nx run cem_ml:test:cli-schema-artifacts`,
      `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture`,
      `yarn nx run cem_ml_cli:validate-converter-parity`,
      `yarn nx run cem_ml_cli:e2e`, then `yarn nx run cem_ml:test`.

### Deferred: Phase 3 Custom-Element Runtime

- [ ] Resume Phase 3 custom-element runtime substrate expansion after the
      schema-package folder contract slice is closed.

### Deferred: Phase 4 CEM Component Set

- [ ] Add a Phase 4 component state-matrix coverage audit/gate that maps
      `docs/component-mvp.md` category state requirements to the executable
      primitive, state, and workflow browser assertions.
- [ ] Populate the first missing state fixture or assertion from that audit,
      prioritizing selected, expanded, empty, and loading coverage across
      navigation, content, and layout workflows.
- [ ] Verify the state-matrix slice with focused `@epa-wg/cem-components`
      target(s), then `yarn nx run @epa-wg/cem-components:verify`.

### Next Work Item

Continue the active CSV schema/CEMT ownership slice:

1. Promote the new `cem_ml::validation::csv::validate_csv_source_bytes` path
   behind the generic schema-package source-validation dispatcher, so
   `text/csv` enters validation through schema URI/content-type resolution
   instead of a CSV branch in `dispatch.rs`.
2. Add the missing package-level CSV examples for unsupported charset,
   US-ASCII byte mismatch, invalid UTF-8, and invalid quote escape, then wire
   their expected diagnostics through `package.cem` so schema-owned example
   validation covers every parse fact kind.
3. Make the CSV formatter CEMT profiles executable in dependency order:
   `compact` first as the minimal source-to-string proof, then `pretty`, then
   `tabular` with row/field alignment and source-map-safe field spans.
4. Make the CSV colorizer CEMT profiles executable after formatter output is
   stable: `terminal`, then `html`, then `md`, preserving token/color metadata
   without reparsing bytes in Rust.
5. Verify with focused CSV tests, CLI schema-owned examples, the CEMT pipeline
   fixture, and `yarn nx run cem_ml:test`.

## Current Verification Commands

- `yarn nx run cem_ml:test:cli-schema-artifacts`
- `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture`
- `yarn nx run cem_ml_cli:validate-converter-parity`
- `yarn nx run cem_ml_cli:e2e`
- `yarn nx run cem_ml:test`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
