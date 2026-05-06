# `cem-dom` CLI Acceptance Criteria

**Status:** Tier A CLI implemented, with parser-backed Tier B/C slices. The current CLI implements `parse`, `validate`,
`check`, `inspect`, `bench`, `convert`, `trace`, `fixture validate`, `version`, and `help`. Remaining Tier B/C commands
are reserved, not implemented.
**Audience:** CEM maintainers, `@epa-wg/cem-dom` contributors, CI authors, and advanced validation or migration users.

This document turns the CLI proposal in [`cli-ideas.md`](./cli-ideas.md) into acceptance criteria for implementation
planning and tracks which criteria have landed.

## Goal

`cem-dom` is the command-line interface for turning CEM documents into trustworthy artifacts: it parses input, validates
it against schemas, transforms it into target DOM representations, emits stable reports and type artifacts, and gives
maintainers CI-friendly tools for migration, debugging, and release verification.

The CLI mental model is:

```txt
input -> parse -> validate -> interpret/transform -> output/report
```

## Tiers

- **Tier A - MVP:** `parse`, `validate`, `check`, `fixture validate`, `help`, and `version`.
- **Tier B - Productized workflows:** `transform`, `convert`, `inspect`, `schema emit`, `schema sample`, and
  `fixture roundtrip`.
- **Tier C - Experimental / full vision:** `schema replace`, `trace`, `bench`, plugin commands, advanced source-map
  stitching, Rust type-header emission, full content-type scope dispatch, and thread-pool debugging.

Tier B and Tier C commands MUST be documented as future or experimental until implemented and covered by tests.

## Implementation Status

Completed Tier A acceptance criteria:

- **CLI-C-1 through CLI-C-5:** `parse`, `validate`, `check`, `fixture validate`, `help`, and `version`.
- **CLI-C-6 partial:** parser-backed `inspect` is implemented for summary, AST, diagnostics, source offsets, and tree
  views. Parser-backed `convert` is implemented for HTML/XML input to DOM JSON, AST, and events.
- **CLI-C-7 partial / CLI-T-7 partial / CLI-T-8 partial:** parser/validator-backed `trace` is implemented with
  deterministic JSON/text output. Parser/validator-backed `bench` is implemented with iterations, JSON reports, and
  per-input budget checks.
- **CLI-C-8:** stable lowercase task-oriented command naming.
- **CLI-O-1, CLI-O-2, CLI-O-5:** fail levels, report destinations, and unknown-option usage failures.
- **CLI-O-3:** Tier A global options are accepted where relevant; `--schema`, `--content-type`, and `--base-uri` are
  recorded but schema/content-type loading is deferred.
- **CLI-F-1 through CLI-F-3 and CLI-F-6:** fail-level behavior and command defaults.
- **CLI-D-1, CLI-D-3, CLI-D-4, CLI-D-5, CLI-D-7:** normalized diagnostics and deterministic JSON/Markdown reports.
- **CLI-P-1 through CLI-P-6:** parse formats include DOM JSON, AST, and events; validate, check, multi-input
  validation, and hard-violation checks are implemented.
- **CLI-X-1 through CLI-X-3:** default and explicit fixture validation plus reports.
- **CLI-N-1 through CLI-N-5:** native Node, ESM, Nx, and test-runner constraints.

Deferred or partial criteria:

- **CLI-F-4 and CLI-F-5:** schema semver behavior is deferred until schema resolution exists.
- **CLI-D-2 and CLI-D-6:** scope metadata and source maps are deferred.
- **CLI-C-6 remaining:** transform, schema emit, schema sample, fixture roundtrip, and advanced convert behavior are
  reserved only.
- **CLI-C-7, CLI-X-4, CLI-T-\* except parser-backed inspect, parser-backed convert, parser/validator-backed trace, and
  parser/validator-backed bench:** Tier B/C behavior is reserved only.

## Command Surface

- **CLI-C-1 [A] MUST** expose `cem-dom parse <input>` for parsing one document into a machine-readable representation.
- **CLI-C-2 [A] MUST** expose `cem-dom validate <input...>` for validating one or more documents.
- **CLI-C-3 [A] MUST** expose `cem-dom check <input...>` as a CI-friendly parse + validate command.
- **CLI-C-4 [A] MUST** expose `cem-dom fixture validate <input...>` for validating semantic fixtures and mirroring the
  package-level `validate-fixtures` workflow.
- **CLI-C-5 [A] MUST** expose `cem-dom help`, `cem-dom --help`, `cem-dom version`, and `cem-dom --version`.
- **CLI-C-6 [B] SHOULD** expose `transform`, `convert`, `inspect`, `schema emit`, `schema sample`, and
  `fixture roundtrip` after the corresponding parser, schema, and transform APIs exist. Parser-backed `inspect` and
  parser-backed `convert` are implemented; schema/transform-backed behavior remains deferred.
- **CLI-C-7 [C] MAY** expose `schema replace`, `trace`, `bench`, and `plugin list|inspect|run` as experimental commands.
  Parser/validator-backed `trace` and `bench` are implemented; transform/plugin/scheduling trace events, transform
  benchmarking, and profiler integration remain deferred.
- **CLI-C-8 [A] MUST** keep command naming stable, lowercase, and task-oriented. Subcommands SHOULD group schema,
  fixture, and plugin workflows.

## Global Options

- **CLI-O-1 [A] MUST** define `--fail-level parse|validate|strict` consistently across parse, validate, check, and
  fixture validation workflows.
- **CLI-O-2 [A] MUST** support report destination options for validation/check workflows:
  `--report-json <file-or-dir>` and `--report-md <file-or-dir>`.
- **CLI-O-3 [A] SHOULD** support `--schema <uri-or-file>`, `--content-type <type>`, `--base-uri <uri>`,
  `--format <format>`, `--out <file-or-dir>`, `--quiet`, `--verbose`, and `--no-color` where relevant.
- Parser-backed `convert` additionally supports `--from-format html|xml`, `--to-format dom-json|ast|events`, and
  `--preserve-source-offsets`.
- **CLI-O-4 [B] SHOULD** support `--source-map`, `--config <file>`, and `--debug` for transform, convert, inspect, and
  plugin workflows.
- **CLI-O-5 [A] MUST** reject unknown options with exit code `2` and a usage message.

## Fail Levels

- **CLI-F-1 [A] MUST** define `parse` fail level as: exit non-zero only on fatal parse failure.
- **CLI-F-2 [A] MUST** define `validate` fail level as: exit non-zero on parse failure or hard validation violation.
- **CLI-F-3 [A] MUST** define `strict` fail level as: exit non-zero on parse failure, hard validation violation, or
  warning.
- **CLI-F-4 [A] MUST** treat compatible minor schema drift as warnings when the loaded schema can safely tolerate the
  document.
- **CLI-F-5 [A] MUST** treat major schema mismatch as a validation failure.
- **CLI-F-6 [A] MUST** document the default fail level for each command before implementation.

## Diagnostics And Reports

- **CLI-D-1 [A] MUST** emit diagnostics with enough structure for tools:
  `{ uri, line, column, byteOffset, code, severity, message }`.
- **CLI-D-2 [B] SHOULD** include scope metadata when available:
  `{ schemaUri, contentType, namespaceUri }`.
- **CLI-D-3 [A] MUST** support terminal output intended for humans.
- **CLI-D-4 [A] MUST** support machine-readable JSON reports for validation/check workflows.
- **CLI-D-5 [A] MUST** support human-readable Markdown reports for validation/check workflows.
- **CLI-D-6 [B] SHOULD** emit source maps for mutating transforms and conversions.
- **CLI-D-7 [A] MUST** make report output deterministic for unchanged inputs.

## Parse, Validate, And Check

- **CLI-P-1 [A] MUST** allow `parse` to emit at least one structured format suitable for tooling, such as AST or DOM
  JSON.
- **CLI-P-2 [A] SHOULD** support or document parse formats. `events`, `ast`, and `dom-json` are implemented; rendered
  `html` and `xml` serialization remain future conversion work.
- **CLI-P-3 [A] MUST** allow `validate` to check schema violations, broken references, missing accessible names, and
  unsafe inline content when the underlying validator supports those checks.
- **CLI-P-4 [A] MUST** allow `check` to combine parse, validate, fail-level handling, and report emission for CI.
- **CLI-P-5 [A] MUST** support multiple input files for `validate` and `check`.
- **CLI-P-6 [A] MUST** make `check --zero-hard-violations` fail if any input has a hard violation.

## Fixture Workflows

- **CLI-X-1 [A] MUST** allow `fixture validate` to validate the known semantic fixtures or an explicit fixture glob.
- **CLI-X-2 [A] MUST** write `*.report.json` and `*.report.md` outputs using the same report conventions as the package
  validation target.
- **CLI-X-3 [A] MUST** fail non-zero when fixture validation records hard violations.
- **CLI-X-4 [B] SHOULD** allow `fixture roundtrip` to run parse -> validate -> transform -> render/snapshot once the
  transform pipeline exists.

## Transform, Convert, Schema, Inspect, Trace, Bench, And Plugins

- **CLI-T-1 [B] SHOULD** allow `transform <input>` to apply a CEM-native or XSLT-equivalent transform and write the
  transformed output plus diagnostics.
- **CLI-T-2 [B] SHOULD** allow `convert <input>` to convert between CEM-native syntax, HTML, XML, AST/events, DOM JSON,
  and schema versions. Parser-backed conversion from HTML/XML input to `dom-json`, `ast`, and `events` is implemented;
  CEM-native input, AST/events input, schema-version conversion, rendered HTML/XML output, comments, and source maps
  remain deferred.
- **CLI-T-3 [B] SHOULD** allow `schema emit <schema>` to emit schema mirrors and type headers, including TypeScript and
  one XML schema mirror for MVP schema work.
- **CLI-T-4 [B] SHOULD** allow `schema sample <schema>` to generate `minimal`, `typical`, `maximal`, `edge`, and
  intentionally `invalid` examples.
- **CLI-T-5 [B] SHOULD** allow `inspect <input>` to show AST, scopes, schema bindings, diagnostics, source offsets,
  plugins, and source maps. AST, diagnostics, source offsets, summary, and tree views are implemented. Scope, schema
  binding, plugin, and source-map views remain deferred.
- **CLI-T-6 [C] MAY** allow `schema replace <input>` to replace or upgrade a schema-governed sub-document selected by
  scope URI, namespace URI, content type, XPath, or CEM selector.
- **CLI-T-7 [C] MAY** allow `trace <input>` to emit deterministic parser, validator, interpreter, transform, plugin,
  and scheduling traces. Parser and validator trace output is implemented; interpreter, transform, plugin, source-map,
  scheduling, worker-pool, and profiler trace output remains deferred.
- **CLI-T-8 [C] MAY** allow `bench <input...>` to benchmark parse, validate, and transform performance. Parse and
  validate benchmarking are implemented; transform benchmarking remains deferred.
- **CLI-T-9 [C] MAY** allow `plugin list`, `plugin inspect <plugin>`, and `plugin run <plugin> <input>`.
- **CLI-T-10 [C] MUST NOT** enable mutating plugin workflows without source-map and failure-behavior documentation.

## Exit Codes

- **CLI-E-1 [A] MUST** use `0` for success.
- **CLI-E-2 [A] MUST** use `1` for parse, validation, or strict-mode failure.
- **CLI-E-3 [A] MUST** use `2` for CLI usage errors.
- **CLI-E-4 [B] SHOULD** reserve `3` for schema resolution errors.
- **CLI-E-5 [B] SHOULD** reserve `4` for transform failures.
- **CLI-E-6 [C] MAY** reserve `5` for plugin failures.
- **CLI-E-7 [B] SHOULD** reserve `6` for I/O errors.
- **CLI-E-8 [B] SHOULD** reserve `7` for unexpected internal errors.

## Native Node And Nx Constraints

- **CLI-N-1 [A] MUST** run as an ESM package with `"type": "module"`.
- **CLI-N-2 [A] MUST** support direct TypeScript execution during development with native Node TypeScript support.
- **CLI-N-3 [A] MUST NOT** require `ts-node`, `tsx`, Babel, Jest, or Vitest for CLI development or CLI tests.
- **CLI-N-4 [A] MUST** use the native Node test runner for CLI tests.
- **CLI-N-5 [A] MUST** expose Nx targets through the workspace package manager for build, lint, typecheck, test, and
  fixture validation.

## Prompt For Itemized Implementation Plan

Use this prompt when asking an engineer or coding agent to create the next implementation plan:

```md
Create an itemized implementation plan for the `@epa-wg/cem-dom` CLI.

Read these files first:

- `packages/cem-dom/docs/cli-ac.md`
- `packages/cem-dom/docs/cli-ideas.md`
- `docs/cem-dom-ac.md`
- `packages/cem-dom/README.md`
- `packages/cem-dom/src/cli.ts`
- `packages/cem-dom/project.json`

Goal:
Implement the Tier A CLI acceptance criteria first, while preserving the package's native Node TypeScript development
model and Nx workspace conventions.

The plan must:

1. Separate Tier A MVP work from Tier B/C future work.
2. List each command to implement or update, including syntax, options, outputs, reports, and exit behavior.
3. Specify any public library API or type changes needed by the CLI.
4. Define diagnostic and report shapes before implementation.
5. Explain data flow from input -> parse -> validate -> output/report.
6. Identify edge cases and failure modes for file I/O, usage errors, validation failures, strict warnings, and report
   writes.
7. Specify test cases using the native Node test runner.
8. Specify Nx target changes and verification commands.
9. Avoid implementing Tier B/C commands as complete unless explicitly scoped; document them as reserved or
   experimental if needed.

Return a decision-complete plan that another engineer can implement without choosing command names, option names,
report paths, exit codes, or test scope.
```
