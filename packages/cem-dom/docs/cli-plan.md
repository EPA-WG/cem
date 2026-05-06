# `@epa-wg/cem-dom` Tier A CLI Implementation Plan

## Summary

Implement Tier A CLI behavior only: `parse`, `validate`, `check`, `fixture validate`, `help`, and `version`. Keep Tier
B/C commands documented/reserved, but do not implement them as working features. Preserve native Node TypeScript
execution, ESM output, Nx run-command targets, and native `node:test`.

## Implementation Status

- [x] Shared diagnostic normalization, fail-level helpers, and report helpers are implemented.
- [x] `parse`, `validate`, `check`, `fixture validate`, `help`, and `version` are implemented.
- [x] `parse --format ast|events|dom-json|json` is implemented for parser-backed machine-readable output.
- [x] `inspect` is implemented for parser-backed `summary`, `ast`, `diagnostics`, `source-offsets`, and `tree` views.
- [x] Tier B/C command names are reserved with usage failures instead of partially implemented behavior.
- [x] `validate-fixtures` delegates through the CLI.
- [x] Native `node:test` coverage was expanded for commands, reports, fail levels, usage errors, I/O errors, and
  reserved commands.
- [x] Nx verification passed for package targets: `typecheck`, `lint`, `test`, `build`, and `validate-fixtures`.
- [x] Root `yarn build` passed in the user's manual run after the `cem-dom` rebuild.
- [ ] Root `yarn build` remains blocked in this assistant execution environment by Nx daemon/plugin-worker startup
  failure before project targets run.
- [ ] Schema version compatibility is still deferred until schema loading exists.
- [ ] Real transform, conversion, schema, trace, bench, and plugin behavior remains Tier B/C deferred work.
- [ ] Advanced inspect views for scopes, schema bindings, plugins, and source maps remain deferred until those
  subsystems exist.

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
- `--format text|json|markdown|dom-json`
- `--out <file>`
- `--schema <uri-or-file>` accepted and recorded, but schema loading may be a documented no-op until schema
  implementation lands
- `--content-type <type>` accepted and recorded, no-op for now
- `--base-uri <uri>` used as URI prefix/source base when provided
- `--quiet`
- `--verbose`
- `--no-color`
- `--zero-hard-violations`

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
- Supports `--show summary|ast|diagnostics|source-offsets|tree`.
- Supports `--format text|json|tree`, with JSON defaults for non-summary/non-tree views.
- Writes output to stdout or `--out`.
- Does not inspect scopes, schema bindings, plugins, or source maps yet.

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
- `convert`
- `inspect`
- `schema emit`
- `schema sample`
- `fixture roundtrip`
- `schema replace`
- `trace`
- `bench`
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
6. Create output/report objects.
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
- `inspect --show summary|ast|diagnostics|source-offsets|tree`
- `inspect --out file`
- `validate <file>` text output
- `validate <file> --format json`
- `validate <file> --report-json dir --report-md dir`
- `validate <fileA> <fileB>` aggregate report
- `check <file> --zero-hard-violations`
- `fixture validate` default fixture set
- `fixture validate <file>`
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

**Status:** Completed. These command names are reserved; real behavior is not implemented.

Do not implement real behavior for:

- transforms
- conversion
- schema emit/sample/replace
- advanced inspect views for scopes, schema bindings, plugins, and source maps
- trace/bench
- plugin list/inspect/run
- source-map generation
- schema version resolution beyond accepting `--schema`

Only reserve command names and return usage text so future command names remain stable.
