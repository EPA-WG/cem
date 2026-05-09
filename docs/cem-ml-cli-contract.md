# `cem-ml-cli` Functional Contract

`cem-ml-cli` replaces the deprecated `cem-dom` CLI. This document preserves functional coverage, option behavior,
report fields, diagnostics, fixture workflows, and exit-code policy without requiring the old `cem-dom` command syntax.

## Functional Surface

- Parse one input into structured output.
- Validate one or more inputs and emit human-readable or machine-readable diagnostics.
- Run CI-oriented checks with hard-violation behavior.
- Inspect parsed output as summary, tree, AST, events, diagnostics, or source-offset views.
- Convert supported inputs into DOM JSON, AST, or event representations.
- Trace parser and validator work with deterministic text or JSON output.
- Benchmark parse and validate work with deterministic text or JSON reports.
- Validate the default semantic fixture set or explicitly provided fixture paths.
- Round trip fixtures through parser-backed projections until transform/render snapshots exist.
- Print help and version information.
- Reserve transform, schema, and plugin workflows until their subsystems are designed.

## Option Behavior To Preserve

- Fail level: `parse`, `validate`, `strict`.
- Output format selection for text, JSON, Markdown, DOM JSON, AST, events, and tree-shaped output where relevant.
- Output destination handling for stdout and `--out`.
- Report destinations for JSON and Markdown reports, including directory destinations with default filenames.
- Schema, content-type, and base-URI recording even before full schema resolution exists.
- Quiet, verbose, and no-color terminal behavior.
- Zero-hard-violations check behavior.
- Source-offset preservation for conversion and parser projection workflows.
- Convert input/output format selection.
- Inspect view selection.
- Benchmark iterations, budget, profile, cold-cache, and JSON report options.
- Default semantic fixture paths.

## Output Contracts

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

The deterministic default timestamp for contract tests is `1970-01-01T00:00:00.000Z`.

## Report Ownership

- Fixture validation JSON: `packages/cem_ml_cli/dist/cem-ml.report.json`
- Fixture validation Markdown: `packages/cem_ml_cli/dist/cem-ml.report.md`
- Fixture roundtrip JSON: `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.json`
- Fixture roundtrip Markdown: `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.md`
- Bench JSON: `packages/cem_ml_cli/dist/cem-ml.bench.report.json`

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
They should not copy the removed TypeScript implementation or require the old binary name.
