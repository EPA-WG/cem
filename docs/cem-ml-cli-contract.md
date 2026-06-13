# `cem-ml-cli` Feature Summary

This document summarizes planned `cem-ml-cli` features: command capabilities, option
behavior, report fields, diagnostics, fixture workflows, and exit-code policy. It is an
input to implementation planning, not a compatibility contract with removed CLI behavior.

The default parser-backed input surface is canonical CEM-ML (`.cem`) using the
curly-brace syntax in [`cem-ml-syntax.md`](cem-ml-syntax.md). XML and HTML are
secondary parity input surfaces and should remain selectable anywhere parser-backed
input format is exposed.

## Major Requirement: Structural Data Lifecycle

`cem-ml` and `cem-ml-cli` MUST provide one structural data lifecycle over every supported
format:

1. **Validate** input bytes against the declared format identity.
2. **Load** the validated input into the internal CEM document AST / event model.
3. **Export** that internal representation into a declared external format.

The format identity is the pair of:

- **content type** — the wire/container syntax, for example `application/cem+xml`,
  `text/html`, `application/xml`, `application/xslt+xml`, or
  `text/custom-element-xslt`;
- **schema / namespace identity** — the structural vocabulary and validation contract
  active inside that content type.

The generic CEM-ML parser/event/AST pipeline is the internal spine. Namespace-specific
and content-type-specific behavior MUST be supplied by registered plugins/adapters that
can participate in these lifecycle stages:

- `load`: bytes + format identity → normalized events / CEM AST;
- `validate`: normalized events / CEM AST + schema identity → diagnostics/report;
- `export`: CEM AST + target format identity → bytes/projection + source map.

The CLI is a thin orchestration layer over this lifecycle. It MUST NOT grow separate
format-specific validation engines per command. Format-specific behavior belongs in
`cem-ml` plugins/adapters, and CLI flags only select the input and output identities.

Current implementation status:

- `parse`, `validate`, `check`, `inspect`, `convert`, and fixture flows already route
  through the `cem_ml::engine::CemMlEngine` trait.
- `--schema` and `--content-type` are carried in `EngineContext` and emitted in reports.
  `cem_ml::lifecycle::LifecycleRegistry` now owns built-in input content-type dispatch
  for parser-backed commands (`parse`, `validate`, `check`, `inspect`, `convert`,
  `trace`, `bench`, and fixture workflows) and CEM target export selection for
  `convert --to-content-type application/cem+xml`; schema-specific adapter selection
  is still pending.
- `validate` / `check` / `convert` route `custom-element-xslt` input through the first
  shared lifecycle adapter path, lowering legacy custom-element XSLT to canonical
  CEM-ML through `cem_ml::legacy_custom_element`; `convert --content-type
  custom-element-xslt --to-content-type application/cem+xml` selects canonical
  CEM-ML export from the declared target identity through the lifecycle registry.
- `schema` and `plugin` CLI command groups are reserved until the registry and plugin
  lifecycle surfaces are promoted from library internals to command-line workflows.

## Functional Surface

- Parse one input into structured output.
- Load supported inputs into the internal CEM event stream / AST through the adapter selected by content type + schema.
- Validate one or more inputs and emit human-readable or machine-readable diagnostics.
- Run CI-oriented checks with hard-violation behavior.
- Inspect parsed output as summary, tree, AST, events, diagnostics, or source-offset views.
- Convert/export supported inputs into declared external formats or debug projections through the same internal AST.
- Trace parser and validator work with deterministic text or JSON output.
- Benchmark parse and validate work with deterministic text or JSON reports.
- Validate the default semantic fixture set or explicitly provided fixture paths.
- Round trip fixtures through parser-backed projections until transform/render snapshots exist.
- Print help and version information.
- Reserve transform, schema, and plugin workflows until their subsystems are designed.

## Planned Option Behavior

- Fail level: `parse`, `validate`, `strict`.
- Input identity selection by content type and schema, with `--from-format cem|html|xml`
  retained only as a convenience alias while the registry matures.
- Output identity selection by content type and schema, with `--to-format cem|dom-json|ast|events`
  retained for current projections and debug layers.
- Output format selection for CEM-native, XML, JSON, text, HTML, Markdown, DOM JSON, AST, events, and tree-shaped
  output where relevant.
- Output destination handling for stdout and `--out`.
- Report destinations for JSON and Markdown reports, including directory destinations with default filenames.
- Schema, content-type, and base-URI recording even before full schema resolution exists.
- Quiet, verbose, and no-color terminal behavior.
- Zero-hard-violations check behavior.
- Source-offset preservation for conversion and parser projection workflows.
- Convert input/output format selection.
- Inspect view selection.
- Benchmark iterations, budget, profile, cold-cache, and JSON report options.
- Default canonical CEM-ML fixture paths and secondary semantic HTML parity fixture paths.

## Output Shapes

Diagnostics keep these fields where available:

- `uri`
- `line`
- `column`
- `byteOffset`
- `code`
- `severity`
- `message`
- optional `node`
- future `sourceMap`

Reports are rendered from the canonical AST-associated report tree. Report event nodes keep:

- source module state
- event sequence
- source-map stack at event time
- visible partial DOM/AST hierarchy at event time

Reports keep deterministic field names:

- `generatedAt`
- `inputs`
- `summary.inputCount`
- `summary.infoCount`
- `summary.warningCount`
- `summary.errorCount`
- `summary.fatalCount`
- `summary.hardViolationCount`
- `options.failLevel`
- `options.schema`
- `options.contentType`
- `options.baseUri`

The deterministic default timestamp for feature tests is `1970-01-01T00:00:00.000Z`.

## Report Ownership

- Fixture validation JSON: `packages/cem_ml_cli/dist/cem-ml.report.json`
- Fixture validation Markdown: `packages/cem_ml_cli/dist/cem-ml.report.md`
- Fixture roundtrip JSON: `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.json`
- Fixture roundtrip Markdown: `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.md`
- Bench JSON: `packages/cem_ml_cli/dist/cem-ml.bench.report.json`

JSON, XML, and CEM-native reports are structured projections. Text, Markdown, and HTML
reports are reference-implementation convenience projections.

## Exit Codes

- `0`: success
- `1`: parse, validation, strict-mode, or benchmark-budget failure
- `2`: CLI usage error, including reserved commands
- `3`: schema resolution error, reserved
- `4`: transform failure, reserved
- `5`: plugin failure, reserved
- `6`: I/O failure
- `7`: unexpected internal failure

## Verification Scope

Rust-side tests should assert functional behavior, option parsing, JSON/report fields, diagnostics, and exit codes.
