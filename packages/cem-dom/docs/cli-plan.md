# `@epa-wg/cem-dom` Tier A CLI Implementation Plan

## Summary

Implement Tier A CLI behavior first: `parse`, `validate`, `check`, `fixture validate`, `help`, and `version`. Bounded
parser-backed Tier B/C slices may be implemented only when they use existing parser/validator APIs and are covered by
native tests. Keep broader Tier B/C behavior documented/reserved. Preserve native Node TypeScript execution, ESM output,
Nx run-command targets, and native `node:test`.

## Implementation Status

- [x] Shared diagnostic normalization, fail-level helpers, and report helpers are implemented.
- [x] `parse`, `validate`, `check`, `fixture validate`, `help`, and `version` are implemented.
- [x] `parse --format ast|events|dom-json|json` is implemented for parser-backed machine-readable output.
- [x] `inspect` is implemented for parser-backed `summary`, `ast`, `events`, `diagnostics`, `source-offsets`, and
  `tree` views.
- [x] `bench` is implemented for parser and validator timing with JSON reports and per-input budget checks.
- [x] `convert` is implemented for parser-backed HTML/XML input to `dom-json`, `ast`, and `events`.
- [x] `trace` is implemented for deterministic parser and validator trace output.
- [x] `fixture roundtrip` is implemented for deterministic parser projection checks and reports.
- [x] Remaining Tier B/C command names are reserved with usage failures instead of partially implemented behavior.
- [x] `validate-fixtures` delegates through the CLI.
- [x] Native `node:test` coverage was expanded for commands, reports, fail levels, usage errors, I/O errors, and
  reserved commands.
- [x] Nx verification passed for package targets: `typecheck`, `lint`, `test`, `build`, and `validate-fixtures`.
- [x] Root `yarn build` passed in the user's manual runs after `cem-dom` rebuilds.
- [ ] Schema version compatibility is still deferred until schema loading exists.
- [ ] Real transform, advanced conversion, schema, advanced trace, and plugin behavior remains Tier B/C deferred work.
- [ ] Transform/render fixture roundtrip snapshots remain deferred until the transform pipeline exists.
- [ ] Advanced inspect views for scopes, schema bindings, plugins, and source maps remain deferred until those
  subsystems exist.
- [ ] Transform benchmarking and real CPU/memory profiling remain deferred until those subsystems exist.

## 1. Public API And Types

**Status:** Completed for Tier A.

Update `packages/cem-dom/src/lib/cem-dom.ts` so CLI behavior is driven by shared library helpers, not ad hoc CLI logic.

Add or update exported types:

```ts
export type CemDiagnosticSeverity = 'info' | 'warning' | 'error' | 'fatal';
export type CemDomFailLevel = 'parse' | 'validate' | 'strict';

export interface CemDiagnostic {
    uri?: string;
    line?: number;
    column?: number;
    byteOffset?: number;
    code: string;
    severity: CemDiagnosticSeverity;
    message: string;
    node?: string;
}

export interface CemDomReport {
    generatedAt: string;
    inputs: CemDomReportInput[];
    summary: {
        inputCount: number;
        infoCount: number;
        warningCount: number;
        errorCount: number;
        fatalCount: number;
        hardViolationCount: number;
    };
}

export interface CemDomReportInput {
    uri: string;
    diagnostics: CemDiagnostic[];
}
```

Keep backward compatibility by preserving `location?: CemSourceLocation` internally if useful, but emitted CLI/report
diagnostics must include top-level `uri`, `line`, `column`, and `byteOffset`.

Add shared helpers:

- `normalizeDiagnostics(diagnostics, sourceName): CemDiagnostic[]`
- `hasFailingDiagnostics(diagnostics, failLevel): boolean`
- `createCemDomReport(inputs): CemDomReport`
- `formatDiagnostics(diagnostics, options?)`
- `formatReportMarkdown(report): string`

Failure policy:

- `parse`: fail only on `fatal`
- `validate`: fail on `error` or `fatal`
- `strict`: fail on `warning`, `error`, or `fatal`

## 2. CLI Commands And Options

**Status:** Completed for Tier A.

Refactor `packages/cem-dom/src/cli.ts` around a small native parser using `node:util.parseArgs`; do not add CLI
dependencies.

Global options supported for Tier A:

- `--fail-level parse|validate|strict`
- `--report-json <file-or-dir>`
- `--report-md <file-or-dir>`
- `--format text|json|markdown|dom-json|ast|events|tree`
- `--from-format html|xml`
- `--to-format dom-json|ast|events`
- `--out <file>`
- `--schema <uri-or-file>` accepted and recorded, but schema loading may be a documented no-op until schema
  implementation lands
- `--content-type <type>` accepted and recorded, no-op for now
- `--base-uri <uri>` used as URI prefix/source base when provided
- `--quiet`
- `--verbose`
- `--no-color`
- `--zero-hard-violations`
- `--preserve-source-offsets`

Reject unknown options with exit code `2`.

### `cem-dom parse <input>`

- Default `--format dom-json`.
- Reads exactly one file.
- Writes parse document JSON to stdout or `--out`.
- Includes diagnostics in JSON.
- Supports `--format dom-json|json|ast|events`.
- Default `--fail-level parse`.
- Exit `1` only if fail-level rules fail.

### `cem-dom inspect <input>`

- Parser-backed Tier B slice now implemented.
- Default `--show summary`.
- Supports `--show summary|ast|events|diagnostics|source-offsets|tree`.
- Supports `--format text|json|tree`, with JSON defaults for non-summary/non-tree views.
- Writes output to stdout or `--out`.
- Does not inspect scopes, schema bindings, plugins, or source maps yet.

### `cem-dom bench <input...>`

- Parser/validator benchmark slice now implemented.
- Reads one or more files.
- Supports `--iterations <n>`; default is `10`.
- Supports `--budget-ms <n>` as a per-input average budget; exits `1` when exceeded.
- Supports `--format text|json`.
- Supports `--report-json <file-or-dir>`.
- Accepts `--profile cpu|memory` and `--cold-cache`; `--profile` is recorded in the report, while real profiler
  integration remains deferred.
- Does not benchmark transforms yet.

### `cem-dom convert <input>`

- Parser-backed Tier B slice now implemented.
- Reads exactly one HTML/XML-ish file.
- Supports `--from-format html|xml`; default is `html`.
- Supports `--to-format dom-json|ast|events`; default is `dom-json`.
- Accepts `--format dom-json|json|ast|events` as an output alias only when `--to-format` is not provided.
- Writes the converted representation to stdout or `--out`.
- Omits parser node `location` objects by default; `--preserve-source-offsets` retains them.
- Includes normalized diagnostics in `dom-json` and `ast` outputs and diagnostic events in `events` output.
- Default `--fail-level parse`.
- Exit `1` only if fail-level rules fail.
- Does not convert CEM-native input, AST/events input, schema versions, light-DOM/custom-element markup, comments, source
  maps, or rendered HTML/XML yet.

### `cem-dom trace <input>`

- Parser/validator-backed Tier C slice now implemented.
- Reads exactly one file.
- Supports `--format json|text`; default is `json`.
- Writes deterministic trace output to stdout or `--out`.
- Emits input, parse, and validate events with stable event indexes.
- JSON output includes `generatedAt`, `uri`, `sourceBytes`, summary counts, parse diagnostics, validation diagnostics,
  and event records.
- Text output summarizes input bytes, parse counts, validation counts, and event lines.
- Default `--fail-level parse`.
- `--fail-level validate|strict` applies to validation diagnostics.
- Does not trace transforms, plugins, source maps, scheduling, worker pools, or interpreter stages yet.

### `cem-dom validate <input...>`

- Reads one or more files.
- Runs validation.
- Default `--format text`.
- Supports `--format text|json|markdown`.
- Supports `--report-json` and `--report-md`.
- Default `--fail-level validate`.
- Exit `1` if diagnostics fail the selected fail level.

### `cem-dom check <input...>`

- CI alias for parse + validate + report.
- Default `--format text`.
- Default `--fail-level validate`.
- `--zero-hard-violations` means any `error` or `fatal` fails, regardless of output format.
- Supports report outputs exactly like `validate`.

### `cem-dom fixture validate [input...]`

- If no inputs are provided, use the five existing semantic fixtures:
  `examples/semantic/assets-list.html`, `login.html`, `message-thread.html`, `profile.html`, `registration.html`.
- If inputs are provided, validate those paths.
- Default reports:
  - `packages/cem-dom/dist/cem-dom.report.json`
  - `packages/cem-dom/dist/cem-dom.report.md`
- Default `--fail-level validate`.
- Exit `1` on hard violations.

### `cem-dom fixture roundtrip [input...]`

- Parser-backed Tier B slice now implemented.
- If no inputs are provided, use the five existing semantic fixtures:
  `examples/semantic/assets-list.html`, `login.html`, `message-thread.html`, `profile.html`, `registration.html`.
- If inputs are provided, roundtrip those paths.
- Performs parse -> validate -> parser-output projection.
- Supports `--to-format dom-json|ast|events`; default is `dom-json`.
- Supports `--format text|json|markdown`; default is `text`.
- Supports `--out`, `--report-json`, and `--report-md`.
- Default reports:
  - `packages/cem-dom/dist/cem-dom.roundtrip.report.json`
  - `packages/cem-dom/dist/cem-dom.roundtrip.report.md`
- Report inputs include source bytes, node counts, projected output bytes, projected output SHA-256, and diagnostics.
- Default `--fail-level validate`.
- Exit `1` when any input fails the selected fail level.
- Does not transform, render, snapshot light-DOM custom-element markup, or emit source maps yet.

### `help`, `--help`, `-h`

- Print top-level command list, Tier A commands, and note that Tier B/C commands are reserved.

### `version`, `--version`, `-v`

- Print package version.

### Reserved commands

For now, return exit code `2` with:

```txt
Command "<name>" is reserved for a future Tier B/C CLI release.
```

Reserved commands:

- `transform`
- `schema emit`
- `schema sample`
- `schema replace`
- `plugin *`

## 3. Data Flow And File Layout

**Status:** Completed with `src/lib/cli-options.ts`, `src/lib/fail-level.ts`, and `src/lib/reports.ts`.

Create a CLI support module to keep `cli.ts` small:

- `src/cli.ts`: process entrypoint and `runCemDomCli`
- `src/lib/cli-options.ts`: parse argv, command routing types
- `src/lib/reports.ts`: report creation, JSON/Markdown writing
- `src/lib/fail-level.ts`: fail-level evaluation

Flow:

1. Parse argv.
2. Resolve input paths relative to current working directory.
3. Read files as UTF-8.
4. Run `parseCemDom` or `validateCemDom`.
5. Normalize diagnostics with URI/source fields.
6. Create output/report objects; `convert` formats parser output as `dom-json`, `ast`, or `events`, `trace` emits
   parser/validator trace events, and `fixture roundtrip` records deterministic parser projection hashes.
7. Write stdout, `--out`, `--report-json`, and/or `--report-md`.
8. Return stable exit code.

Report write behavior:

- If report destination is a directory or multiple inputs are used, write one aggregate report named
  `cem-dom.report.json` / `cem-dom.report.md` inside that directory.
- If destination has `.json` or `.md`, write exactly that file.
- Create parent directories with `mkdir({ recursive: true })`.
- I/O failures return exit code `6`.

## 4. Edge Cases

**Status:** Completed for Tier A.

Handle explicitly:

- Missing required input: exit `2`.
- Unknown command/option: exit `2`.
- Invalid `--fail-level`: exit `2`.
- Invalid `--format`: exit `2`.
- Invalid `--from-format` or `--to-format`: exit `2`.
- `convert` with both `--format` and `--to-format`: exit `2`.
- File read failure: exit `6`.
- Report write failure: exit `6`.
- Unexpected thrown error: exit `7`.
- Validation warnings with `--fail-level strict`: exit `1`.
- Multiple files with duplicate basenames: aggregate report still keys by full normalized path.
- `--quiet`: suppress success text, but not errors.
- `--out` with multi-input `validate`/`check`: reject with exit `2`; use reports instead.

## 5. Nx Targets

**Status:** Completed. The `validate-fixtures` target now delegates to `node packages/cem-dom/src/cli.ts fixture validate`.

Keep existing targets, but update commands if needed:

- `build`: unchanged, `tsc --build tsconfig.lib.json`
- `typecheck`: unchanged
- `lint`: unchanged
- `test`: include all CLI/lib tests under `src/**/*.test.ts`
- `validate-fixtures`: replace script-only logic or delegate to CLI:
  `node packages/cem-dom/src/cli.ts fixture validate`

Do not introduce `tsx`, `ts-node`, Jest, Vitest, Babel, or CLI dependencies.

## 6. Tests

**Status:** Completed for Tier A.

Use native `node:test`.

Add/update CLI tests for:

- `help`, `--help`, `version`, `--version`
- `parse <file>` default JSON output
- `parse --out file`
- `parse --format ast`
- `parse --format events`
- `parse --fail-level strict` fails on warnings
- `inspect --show summary|ast|events|diagnostics|source-offsets|tree`
- `inspect --out file`
- `convert --from-format html --to-format ast`
- `convert --format json`
- `convert --from-format xml --to-format events --out file`
- `convert --preserve-source-offsets`
- invalid convert format options
- `trace <file>` default JSON output
- `trace --format text --out file`
- `trace --fail-level strict` fails on validation warnings
- invalid trace format option
- `bench <file> --iterations <n>`
- `bench --format json`
- `bench --report-json dir`
- `bench --budget-ms <n>`
- invalid bench options
- `validate <file>` text output
- `validate <file> --format json`
- `validate <file> --report-json dir --report-md dir`
- `validate <fileA> <fileB>` aggregate report
- `check <file> --zero-hard-violations`
- `fixture validate` default fixture set
- `fixture validate <file>`
- `fixture roundtrip` default fixture set
- `fixture roundtrip <file> --to-format ast --format json`
- `fixture roundtrip --format markdown --out file`
- `fixture roundtrip --fail-level strict` fails on validation warnings
- invalid fixture roundtrip format option
- unknown command, unknown option, invalid fail level, missing input
- unreadable file returns exit `6`
- reserved Tier B/C command returns exit `2`

Update library tests for:

- diagnostic normalization includes `uri`, `line`, `column`, `byteOffset`
- fail-level helper behavior for parse/validate/strict
- report summary counts
- Markdown report deterministic output shape

Verification commands:

```bash
yarn nx run @epa-wg/cem-dom:typecheck
yarn nx run @epa-wg/cem-dom:lint
yarn nx run @epa-wg/cem-dom:test
yarn nx run @epa-wg/cem-dom:build
yarn nx run @epa-wg/cem-dom:validate-fixtures
node packages/cem-dom/src/cli.ts check examples/semantic/login.html --report-json /tmp/cem-dom-report.json
```

## 7. Deferred Tier B/C

**Status:** Partially completed. Parser-backed `inspect`, `convert`, and `fixture roundtrip`, plus parser/validator-
backed `trace` and `bench` slices, are implemented. The remaining command names are reserved; advanced behavior is not
implemented.

Do not implement real behavior for:

- transforms
- advanced conversion beyond HTML/XML input to `dom-json`, `ast`, and `events`
- schema emit/sample/replace
- advanced inspect views for scopes, schema bindings, plugins, and source maps
- advanced trace stages for transforms, plugins, source maps, scheduling, worker pools, and interpreters
- transform/render fixture roundtrip snapshots
- transform benchmarking and CPU/memory profiler integration
- plugin list/inspect/run
- source-map generation
- schema version resolution beyond accepting `--schema`

Only reserve remaining command names and return usage text so future command names remain stable.
