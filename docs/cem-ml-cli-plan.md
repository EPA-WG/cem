# `cem-ml-cli` Implementation Plan

**Status:** Phase 1 parser/schema assessment complete. No Rust implementation is included in this document.

Phase 0 artifact: [`docs/cem-ml-cli-parity-matrix.md`](./cem-ml-cli-parity-matrix.md).
Phase 1 artifact: [`docs/cem-ml-parser-schema-adr.md`](./cem-ml-parser-schema-adr.md).

This plan maps the completed `cem-dom` CLI contract onto the Rust platform:

- App crate: `packages/cem_ml_cli`, Cargo package `cem-ml-cli`, binary `cem-ml`.
- Library crate: `packages/cem_ml`, Cargo package `cem-ml`, Rust crate `cem_ml`.
- Acceptance source: `packages/cem-dom/docs/cli-ac.md` and `packages/cem-dom/docs/cli-plan.md`.

Where the `cem-dom` CLI AC names the executable or package-owned output files, map the platform name from
`cem-dom` to `cem-ml`. Otherwise preserve command names, options, report fields, diagnostic fields, fail-level behavior,
and exit codes.

## Explicit Scope

- Do not implement or design the streaming parser in this plan.
- Do not implement or design multithreading, worker pools, scheduler traces, or render-while-parsing.
- Do not implement parser internals yet, including XML, HTML, CEM-native syntax, AST construction, or event production.
- Do front-load a separate Java XML stack, parser pattern, and schema pattern assessment before implementation.
- Keep `cem-ml-cli` thin. Shared behavior belongs in `cem-ml`.

## Phase 0 - Contract Lock

**Status:** Complete. See [`docs/cem-ml-cli-parity-matrix.md`](./cem-ml-cli-parity-matrix.md).

1. Read and pin the source contract:
    - `packages/cem-dom/docs/cli-ac.md`
    - `packages/cem-dom/docs/cli-plan.md`
    - `docs/cem-dom-ac.md`
    - `packages/cem-dom/src/cli.ts`
    - `packages/cem-dom/src/lib/*.ts`
2. Create a parity matrix with columns:
    - `cem-dom` AC id
    - Rust command or library module
    - required output shape
    - implementation status
    - blocked-by-parser status
3. Confirm the platform remaps:
    - binary: `cem-dom` -> `cem-ml`
    - default fixture report paths:
        - `packages/cem_ml_cli/dist/cem-ml.report.json`
        - `packages/cem_ml_cli/dist/cem-ml.report.md`
        - `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.json`
        - `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.md`
        - `packages/cem_ml_cli/dist/cem-ml.bench.report.json`
    - fixture inputs remain `examples/semantic/*.html`.
4. Preserve exit codes:
    - `0`: success
    - `1`: parse, validation, strict-mode, or benchmark-budget failure
    - `2`: CLI usage error
    - `3`: schema resolution error, reserved
    - `4`: transform failure, reserved
    - `5`: plugin failure, reserved
    - `6`: I/O failure
    - `7`: unexpected internal failure

## Phase 1 - Java XML, Parser, And Schema Pattern Assessment

**Status:** Complete. See [`docs/cem-ml-parser-schema-adr.md`](./cem-ml-parser-schema-adr.md).

This phase must finish before parser-backed implementation begins. It is research and decision work only.

1. Inventory Java XML stack patterns relevant to CEM:
    - JAXP DOM, SAX, and StAX API boundaries.
    - Xerces-style XML diagnostics, source locations, entity handling, and namespace behavior.
    - Saxon-style XPath/XSLT boundaries and URI resolution.
    - Jing/Trang-style RELAX NG validation and schema conversion.
    - Validator.nu and HTML5 parser behavior for HTML inputs.
    - XML catalog and schema resolver patterns.
2. Inventory Rust ecosystem candidates without selecting or integrating one yet:
    - XML token/event readers.
    - HTML5 parser crates.
    - DOM tree materialization crates.
    - RELAX NG, XSD, XPath, and XSLT options or gaps.
    - libxml2 or Java-process interop risks if pure Rust coverage is insufficient.
3. Compare the stacks against CEM requirements:
    - deterministic diagnostics with `{ uri, line, column, byteOffset, code, severity, message }`
    - namespace and schema URI behavior
    - secure defaults for untrusted input
    - stable event model suitable for future streaming
    - schema mirror generation path, especially XSD vs RELAX NG
    - WASM feasibility for the `cem-ml` crate
4. Record decisions in an ADR before implementation:
    - parser engine recommendation
    - schema mirror recommendation
    - source-location strategy
    - security defaults
    - unresolved gaps and follow-up plan

Exit criteria: an ADR exists and no parser code has been added.

## Phase 2 - Rust Crate Boundary

1. Move all reusable behavior into `cem-ml`.
2. Keep `cem-ml-cli/src/main.rs` limited to:
    - Clap argument parsing
    - process cwd/workspace/package root detection
    - stdout/stderr writing
    - process exit code handling
3. Define `cem-ml` modules:
    - `diagnostic`: severity, source location, diagnostic structs, formatting
    - `fail_level`: parse, validate, strict evaluation
    - `report`: deterministic report models and Markdown/JSON rendering
    - `formats`: parse output format names and conversion output names
    - `fixture`: default fixture paths and fixture report path policy
    - `engine`: trait boundary for parse/validate/inspect/trace/bench inputs
    - `command`: I/O-independent command orchestration
    - `error`: usage, I/O, schema, transform, plugin, and internal error mapping
4. Use Rust type names with `CemMl` where a prefix is useful, but keep JSON field names compatible with `cem-dom`.
5. Add serialization dependencies only when the implementation phase starts:
    - `serde`
    - `serde_json`
    - optional `thiserror`

## Phase 3 - Shared Contract Types In `cem-ml`

1. Define diagnostic types matching the `cem-dom` JSON shape:
    - `uri`
    - `line`
    - `column`
    - `byteOffset`
    - `code`
    - `severity`
    - `message`
    - optional `node`
2. Define fail levels:
    - `parse`: fail only on `fatal`
    - `validate`: fail on `error` or `fatal`
    - `strict`: fail on `warning`, `error`, or `fatal`
3. Define report models matching `cem-dom`:
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
4. Use a deterministic default timestamp for contract tests:
    - `1970-01-01T00:00:00.000Z`
5. Define command output models for:
    - DOM JSON
    - AST
    - events
    - inspect summary
    - trace report
    - bench report
    - fixture roundtrip report

These are data contracts only. Parser-filled content remains blocked until the parser decision phase is complete.

## Phase 4 - CLI Command Surface

1. Implement Clap command declarations for the same surface as `cem-dom`, with binary name `cem-ml`:
    - `parse <input>`
    - `validate <input...>`
    - `check <input...>`
    - `inspect <input>`
    - `bench <input...>`
    - `convert <input>`
    - `trace <input>`
    - `fixture validate [input...]`
    - `fixture roundtrip [input...]`
    - `version`
    - `help`
2. Preserve common options:
    - `--fail-level parse|validate|strict`
    - `--format text|json|markdown|dom-json|ast|events|tree`
    - `--from-format html|xml`
    - `--to-format dom-json|ast|events`
    - `--show summary|ast|events|diagnostics|source-offsets|tree`
    - `--iterations <n>`
    - `--budget-ms <n>`
    - `--profile cpu|memory`
    - `--cold-cache`
    - `--preserve-source-offsets`
    - `--out <file>`
    - `--report-json <file-or-dir>`
    - `--report-md <file-or-dir>`
    - `--schema <uri-or-file>`
    - `--content-type <type>`
    - `--base-uri <uri>`
    - `--zero-hard-violations`
    - `--quiet`
    - `--verbose`
    - `--no-color`
3. Reserve these commands with exit code `2` until their subsystem plans exist:
    - `transform`
    - `schema emit`
    - `schema sample`
    - `schema replace`
    - `plugin list`
    - `plugin inspect`
    - `plugin run`
4. Reject unknown commands, unknown options, invalid enum values, missing inputs, and incompatible option combinations
   with exit code `2`.

## Phase 5 - Engine Boundary Without Parser Implementation

1. Define a `CemMlEngine` trait in `cem-ml`.
2. Route parser-backed commands through that trait:
    - `parse`
    - `validate`
    - `check`
    - `inspect`
    - `convert`
    - `trace`
    - `bench`
    - `fixture validate`
    - `fixture roundtrip`
3. Provide a fake engine only for contract tests.
4. Do not add a real parser engine in this plan.
5. Do not mark parser-backed AC complete until the future parser implementation exists.
6. Keep the command orchestration complete enough that replacing the fake engine with the real engine does not require
   changing Clap definitions, output models, report writers, or exit-code logic.

## Phase 6 - Command Behavior Contracts

1. `cem-ml parse <input>`
    - Default format: `dom-json`.
    - Supported formats: `dom-json`, `json`, `ast`, `events`.
    - Default fail level: `parse`.
    - Writes to stdout or `--out`.
2. `cem-ml validate <input...>`
    - Supported formats: `text`, `json`, `markdown`.
    - Default fail level: `validate`.
    - Supports aggregate `--report-json` and `--report-md`.
3. `cem-ml check <input...>`
    - Same data flow as validate.
    - Supports `--zero-hard-violations`.
    - Default fail level: `validate`.
4. `cem-ml fixture validate [input...]`
    - Defaults to the five semantic fixtures when no input is passed.
    - Writes default `cem-ml.report.json` and `cem-ml.report.md`.
5. `cem-ml inspect <input>`
    - Supported `--show`: `summary`, `ast`, `events`, `diagnostics`, `source-offsets`, `tree`.
    - Scope, schema-binding, plugin, and source-map views remain deferred.
6. `cem-ml convert <input>`
    - Supported input formats: `html`, `xml`.
    - Supported output formats: `dom-json`, `ast`, `events`.
    - CEM-native input, schema-version conversion, rendered HTML/XML output, comments, and source maps remain deferred.
7. `cem-ml trace <input>`
    - Supported formats: `json`, `text`.
    - Parser and validator trace records are contract-only until parser implementation exists.
    - Scheduler, worker-pool, transform, plugin, and source-map traces remain deferred.
8. `cem-ml bench <input...>`
    - Supported formats: `text`, `json`.
    - Supports `--iterations`, `--budget-ms`, `--profile`, `--cold-cache`, and `--report-json`.
    - Benchmarking uses the engine boundary; parser performance work is deferred.
9. `cem-ml fixture roundtrip [input...]`
    - Defaults to the five semantic fixtures.
    - Supports `--to-format dom-json|ast|events`.
    - Transform/render snapshots remain deferred.

## Phase 7 - File I/O And Reports

1. Resolve inputs relative to cwd unless absolute.
2. Apply `--base-uri` to emitted diagnostic/report URIs using the same path normalization policy as `cem-dom`.
3. Write parent directories recursively for `--out`, `--report-json`, and `--report-md`.
4. If a report destination has the expected file extension, write exactly that file.
5. If a report destination is a directory, write the default report filename inside it.
6. Return exit code `6` for read or write failures.
7. Keep stdout empty when `--out` is used.
8. Keep success text suppressed by `--quiet`, but still surface errors.

## Phase 8 - Tests

1. Add `cem-ml` unit tests for:
    - diagnostic normalization
    - fail-level evaluation
    - hard-violation detection
    - deterministic report summaries
    - JSON field naming
    - Markdown report formatting
2. Add `cem-ml` command tests with a fake engine for:
    - parse output formats
    - validate text, JSON, Markdown, and report outputs
    - check `--zero-hard-violations`
    - fixture default path selection
    - inspect output modes
    - convert format validation
    - trace JSON/text shape
    - bench budget exit behavior
    - fixture roundtrip report shape
3. Add `cem-ml-cli` integration tests for:
    - help and version
    - unknown command
    - unknown option
    - invalid fail level
    - invalid format
    - missing required input
    - reserved commands
    - file read failure exit `6`
4. Do not assert real parsing behavior in this phase.
5. Keep parser-backed fixture success tests blocked until the parser implementation plan is approved.

## Phase 9 - Nx And Cargo Verification

Use Nx through the workspace package manager.

1. Existing project targets to preserve:
    - `yarn nx run cem_ml:build`
    - `yarn nx run cem_ml:test`
    - `yarn nx run cem_ml:lint`
    - `yarn nx run cem_ml_cli:build`
    - `yarn nx run cem_ml_cli:test`
    - `yarn nx run cem_ml_cli:lint`
    - `yarn nx run cem_ml_cli:run`
2. Add `cem_ml_cli:validate-fixtures` only after a real parser engine exists.
3. Do not claim parity with `@epa-wg/cem-dom:validate-fixtures` until `cem-ml fixture validate` validates
   `examples/semantic/*.html` with zero hard violations.
4. Keep `cem_ml_cli` dependent on `cem_ml` through Cargo, not by duplicating code.

## Phase 10 - Completion Gates

1. Contract gate:
    - help/version work
    - CLI usage errors match exit-code policy
    - report and diagnostic models match `cem-dom`
    - parser-backed commands are routed through `CemMlEngine`
    - fake-engine contract tests pass
2. Decision gate:
    - Java XML stack and schema/parser ADR is accepted
    - parser and schema mirror recommendations are recorded
    - security defaults are documented
3. Parser-enabled gate, future plan:
    - real engine fills the existing `CemMlEngine` boundary
    - no CLI command or output-shape redesign is needed
    - fixture validation can be enabled as an Nx target
4. Parity gate, future plan:
    - `cem-ml` CLI reaches the same acceptance status as current `cem-dom` CLI, subject only to platform naming.

## Deferred Work

The following are intentionally outside this plan:

- streaming parser implementation
- parser algorithm selection or parser crate integration
- multithreading, worker pools, scheduler traces, and bounded queues
- schema emit/sample/replace implementation
- schema semver resolution behavior beyond accepting and recording `--schema`
- transform implementation
- plugin implementation
- source maps
- WASM packaging beyond keeping the `cem-ml` crate boundary compatible
