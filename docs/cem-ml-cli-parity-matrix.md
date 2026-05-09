# `cem-ml` CLI Phase 0 Contract Lock

**Status:** Phase 0 complete. This document locks the functional contract for the Rust `cem-ml` CLI before parser,
schema, transform, or plugin implementation starts.

## Source Contract Pin

Pinned on 2026-05-07 against workspace revision `5c4149b1c421e7f8dcbbac62428c1612287f5878`.

Active contract source:

- [`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md) - normative functional CLI contract.
- [`cem-ml-ac.md`](./cem-ml-ac.md) - parser/runtime, schema, validation, transform, plugin, performance, and security ACs.
- This matrix — contract lock from functional requirements to Rust work items.

Existing Rust package boundary:

- App crate: `packages/cem_ml_cli`, Cargo package `cem-ml-cli`, binary `cem-ml`.
- Library crate: `packages/cem_ml`, Cargo package `cem-ml`, Rust crate `cem_ml`.
- Nx project targets already declared: `cem_ml:{build,test,lint,build:wasm}` and
  `cem_ml_cli:{build,test,lint,run}`.
- Active workspace projects: `@epa-wg/cem-components`, `cem_ml_cli`, `@epa-wg/cem-theme`, `cem_ml`, and `@epa-wg/cem`.

## Platform Configuration

| Item                              | Value                                                   |
| --------------------------------- | ------------------------------------------------------- |
| Binary                            | `cem-ml`                                                |
| App package/crate                 | Cargo package `cem-ml-cli` in `packages/cem_ml_cli`     |
| Shared library                    | Cargo package `cem-ml` in `packages/cem_ml`             |
| Fixture inputs                    | `examples/semantic/*.html`                              |
| Fixture validate JSON report      | `packages/cem_ml_cli/dist/cem-ml.report.json`           |
| Fixture validate Markdown report  | `packages/cem_ml_cli/dist/cem-ml.report.md`             |
| Fixture roundtrip JSON report     | `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.json` |
| Fixture roundtrip Markdown report | `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.md`   |
| Bench JSON report                 | `packages/cem_ml_cli/dist/cem-ml.bench.report.json`     |
| JSON field names                  | Defined in the active contract specification            |

Default fixture set remains:

- `examples/semantic/assets-list.html`
- `examples/semantic/login.html`
- `examples/semantic/message-thread.html`
- `examples/semantic/profile.html`
- `examples/semantic/registration.html`

## Exit Codes

| Exit code | Meaning                                                     | Rust status                                       |
| --------- | ----------------------------------------------------------- | ------------------------------------------------- |
| `0`       | Success                                                     | Preserve exactly.                                 |
| `1`       | Parse, validation, strict-mode, or benchmark-budget failure | Preserve exactly.                                 |
| `2`       | CLI usage error                                             | Preserve exactly. Reserved commands also use `2`. |
| `3`       | Schema resolution error                                     | Reserved until schema resolution exists.          |
| `4`       | Transform failure                                           | Reserved until transform exists.                  |
| `5`       | Plugin failure                                              | Reserved until plugins exist.                     |
| `6`       | I/O failure                                                 | Preserve exactly.                                 |
| `7`       | Unexpected internal failure                                 | Preserve exactly.                                 |

## Status Terms

- **Scaffolded only:** a Rust crate or binary exists, but no contract behavior is implemented.
- **Planned:** implement in `cem_ml` and expose through `cem_ml_cli` in later phases.
- **Reserved:** expose command names as usage failures until their subsystem plan exists.
- **Blocked by parser:** a real result cannot be produced until Phase 1 decisions and parser implementation exist.
- **Boundary first:** command and report plumbing can be implemented with a fake engine before the real parser.

## Command Surface Matrix

| AC id           | Rust command or library module                                                                         | Required output shape                                                                                 | Implementation status                                                     | Blocked-by-parser status                                                               |
| ------- | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| CLI-C-1         | `cem-ml parse <input>` via `cem_ml::command` and `cem_ml_cli`                                          | DOM JSON by default; `json`, `ast`, and `events` formats; stdout or `--out`; fail-level exit handling | Scaffolded only; planned Phases 4-6                                       | Boundary first; real content blocked                                                   |
| CLI-C-2         | `cem-ml validate <input...>` plus report models                                                        | Text, JSON, or Markdown validation output; optional aggregate JSON/Markdown reports                   | Planned Phases 4-7                                                        | Boundary first; real validation blocked                                                |
| CLI-C-3         | `cem-ml check <input...>`                                                                              | Same report shape as validate; CI exit behavior; `--zero-hard-violations`                             | Planned Phases 4-7                                                        | Boundary first; real validation blocked                                                |
| CLI-C-4         | `cem-ml fixture validate [input...]`                                                                   | Default semantic fixture validation and default `cem-ml.report.{json,md}` outputs                     | Planned Phases 4-7; fixture target deferred                               | Real fixture validation blocked                                                        |
| CLI-C-5         | `cem-ml help`, `cem-ml --help`, `cem-ml version`, `cem-ml --version`                                   | Human help text and version string                                                                    | Binary has only a custom Clap version flag; full contract planned Phase 4 | No                                                                                     |
| CLI-C-6         | `cem-ml convert`, `inspect`, `fixture roundtrip`; reserved `transform`, `schema emit`, `schema sample` | Parser-backed slices return deterministic output; schema/transform commands usage-fail as reserved    | Planned/reserved Phases 4-6                                               | Convert/inspect/roundtrip real content blocked; schema/transform blocked by subsystems |
| CLI-C-7         | `cem-ml trace`, `bench`; reserved `schema replace` and plugin commands                                 | Trace JSON/text and bench text/JSON/report shapes; plugin/schema commands usage-fail as reserved      | Planned/reserved Phases 4-6                                               | Trace/bench real parser metrics blocked; plugin/schema blocked by subsystems           |
| CLI-C-8         | Clap command declarations                                                                              | Stable lowercase task-oriented names and grouped subcommands                                          | Planned Phase 4                                                           | No                                                                                     |

## Global Options Matrix

| AC id           | Rust command or library module             | Required output shape                                                                                                                               | Implementation status        | Blocked-by-parser status                                |
| ------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------- |
| CLI-O-1         | `cem_ml::fail_level` and Clap enum         | `--fail-level` accepts `parse`, `validate`, or `strict` across parse, validate, check, fixtures, trace, convert, and roundtrip                      | Planned Phases 3-4           | No                                                      |
| CLI-O-2         | `cem_ml::report` and `cem_ml::fixture`     | `--report-json <file-or-dir>` and `--report-md <file-or-dir>` with default filenames when destination is a directory                                | Planned Phases 3, 7          | No                                                      |
| CLI-O-3         | Shared option structs in `cem_ml::command` | Record `--schema`, `--content-type`, `--base-uri`, `--format`, `--out`, `--quiet`, `--verbose`, `--no-color`; output shapes consume relevant values | Planned Phases 3-7           | Mostly no; schema semantics blocked by schema subsystem |
| CLI-O-4         | Reserved advanced options                  | `--source-map`, `--config`, `--debug` remain deferred for transform/convert/inspect/plugin workflows                                                | Deferred beyond current plan | Blocked by transform/plugin/source-map plans            |
| CLI-O-5         | Clap error mapping in `cem_ml_cli`         | Unknown options exit `2` with usage text                                                                                                            | Planned Phase 4              | No                                                      |

## Fail Level Matrix

| AC id           | Rust command or library module                 | Required output shape                                                                                                               | Implementation status                | Blocked-by-parser status    |
| ------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ | --------------------------- |
| CLI-F-1         | `cem_ml::fail_level`                           | `parse` fails only on `fatal` diagnostics                                                                                           | Planned Phase 3                      | No                          |
| CLI-F-2         | `cem_ml::fail_level`                           | `validate` fails on `error` or `fatal` diagnostics                                                                                  | Planned Phase 3                      | No                          |
| CLI-F-3         | `cem_ml::fail_level`                           | `strict` fails on `warning`, `error`, or `fatal` diagnostics                                                                        | Planned Phase 3                      | No                          |
| CLI-F-4         | `cem_ml::diagnostic` plus future schema module | Compatible minor schema drift emits warnings                                                                                        | Deferred until schema loading exists | Blocked by schema subsystem |
| CLI-F-5         | `cem_ml::diagnostic` plus future schema module | Major schema mismatch is a validation failure                                                                                       | Deferred until schema loading exists | Blocked by schema subsystem |
| CLI-F-6         | Clap declarations and command defaults         | Defaults: parse/convert/trace `parse`; validate/check/fixtures/roundtrip `validate`; bench budget failure independent of fail level | Planned Phases 4-6                   | No                          |

## Diagnostics And Reports Matrix

| AC id           | Rust command or library module                           | Required output shape                                                                                    | Implementation status        | Blocked-by-parser status              |
| ------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------- |
| CLI-D-1         | `cem_ml::diagnostic`                                     | Diagnostics emit `{ uri, line, column, byteOffset, code, severity, message }`; optional `node` preserved | Planned Phase 3              | Types no; real locations blocked      |
| CLI-D-2         | `cem_ml::diagnostic` future scope metadata               | Optional `{ schemaUri, contentType, namespaceUri }` when scope metadata exists                           | Deferred                     | Blocked by parser/schema scope design |
| CLI-D-3         | `cem_ml::diagnostic::format_text` and command formatting | Human terminal output for diagnostics, summaries, trace, bench, help, version                            | Planned Phases 3-6           | No for formatting; content blocked    |
| CLI-D-4         | `cem_ml::report`                                         | Deterministic JSON validation/check reports                                                              | Planned Phase 3              | No for model; content blocked         |
| CLI-D-5         | `cem_ml::report`                                         | Deterministic Markdown validation/check reports                                                          | Planned Phase 3              | No for renderer; content blocked      |
| CLI-D-6         | Future source-map module                                 | Source maps for mutating transforms/conversions                                                          | Deferred beyond current plan | Blocked by transform/source-map plans |
| CLI-D-7         | `cem_ml::report` and fixture reports                     | Stable `generatedAt` default `1970-01-01T00:00:00.000Z`; deterministic ordering and counts               | Planned Phases 3, 6-7        | No for model; content blocked         |

## Parse, Validate, And Check Matrix

| AC id           | Rust command or library module               | Required output shape                                                                                                               | Implementation status                          | Blocked-by-parser status                |
| ------- | -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- | --------------------------------------- |
| CLI-P-1         | `cem-ml parse` and output models             | At least one structured parse format; contract keeps `dom-json`, `ast`, and `events`                                                | Planned Phases 3-6                             | Real parse blocked                      |
| CLI-P-2         | `cem_ml::formats`                            | Document and enforce parse formats: `dom-json`, `json`, `ast`, `events`; rendered HTML/XML deferred                                 | Planned Phases 3-6                             | Real parse blocked                      |
| CLI-P-3         | `cem_ml::engine::CemMlEngine` validate path  | Validation diagnostics for schema violations, broken references, missing accessible names, and unsafe inline content when supported | Boundary planned Phase 5; real checks deferred | Real validation blocked                 |
| CLI-P-4         | `cem-ml check` command orchestration         | Combined parse, validate, fail-level, reports, and CI exit behavior                                                                 | Planned Phases 4-7                             | Boundary first; real validation blocked |
| CLI-P-5         | `cem_ml::command` multi-input validation     | Multiple input files for `validate` and `check`; aggregate report by URI                                                            | Planned Phases 4-7                             | No for orchestration; content blocked   |
| CLI-P-6         | `cem_ml::fail_level` and check orchestration | `check --zero-hard-violations` fails on any `error` or `fatal` diagnostic                                                           | Planned Phases 3, 6                            | No for evaluation; content blocked      |

## Fixture Workflow Matrix

| AC id           | Rust command or library module                  | Required output shape                                                                                                                                   | Implementation status                         | Blocked-by-parser status                                            |
| ------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------- |
| CLI-X-1         | `cem_ml::fixture` and `cem-ml fixture validate` | Default five semantic fixtures or explicit input paths                                                                                                  | Planned Phases 4, 6-7                         | No for path policy; real validation blocked                         |
| CLI-X-2         | `cem_ml::fixture` report writers                | Default `packages/cem_ml_cli/dist/cem-ml.report.json` and `.md`; same report conventions as package target                                              | Planned Phases 6-7                            | No for writers; content blocked                                     |
| CLI-X-3         | `cem_ml::fixture` fail handling                 | Non-zero when fixture validation records hard violations                                                                                                | Planned Phase 6                               | Real validation blocked                                             |
| CLI-X-4         | `cem-ml fixture roundtrip`                      | Parser projection report with target format, counts, output bytes, SHA-256, diagnostics, and JSON/Markdown reports; transform/render snapshots deferred | Planned Phase 6; transform snapshots deferred | Parser projection blocked; transform snapshots blocked by transform |

## Transform, Convert, Schema, Inspect, Trace, Bench, And Plugins Matrix

| AC id           | Rust command or library module           | Required output shape                                                                                                                        | Implementation status        | Blocked-by-parser status               |
| ------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------- |
| CLI-T-1         | Reserved `cem-ml transform <input>`      | Usage error `2` until transform plan exists; future transformed output plus diagnostics                                                      | Reserved Phase 4             | Blocked by transform subsystem         |
| CLI-T-2         | `cem-ml convert <input>`                 | HTML/XML to `dom-json`, `ast`, or `events`; stdout or `--out`; `--preserve-source-offsets`; rendered HTML/XML and schema conversion deferred | Planned Phase 6              | Real conversion blocked                |
| CLI-T-3         | Reserved `cem-ml schema emit <schema>`   | Usage error `2` until schema mirror plan exists; future schema mirrors/type headers                                                          | Reserved Phase 4             | Blocked by schema subsystem            |
| CLI-T-4         | Reserved `cem-ml schema sample <schema>` | Usage error `2`; future minimal/typical/maximal/edge/invalid examples                                                                        | Reserved Phase 4             | Blocked by schema subsystem            |
| CLI-T-5         | `cem-ml inspect <input>`                 | `summary`, `ast`, `events`, `diagnostics`, `source-offsets`, `tree`; text/json/tree output; advanced views deferred                          | Planned Phase 6              | Real inspect content blocked           |
| CLI-T-6         | Reserved `cem-ml schema replace <input>` | Usage error `2`; future scope-selected schema replacement                                                                                    | Reserved Phase 4             | Blocked by schema/transform subsystems |
| CLI-T-7         | `cem-ml trace <input>`                   | Deterministic parser/validator trace JSON/text with generatedAt, URI, source bytes, summaries, diagnostics, event indexes                    | Planned Phase 6              | Real trace content blocked             |
| CLI-T-8         | `cem-ml bench <input...>`                | Text/JSON benchmark report with iterations, profile, cold-cache, budget, totals, averages, per-input diagnostics, optional `--report-json`   | Planned Phase 6              | Real parser/validator timing blocked   |
| CLI-T-9         | Reserved `cem-ml` plugin commands        | Usage error `2` until plugin subsystem exists                                                                                                | Reserved Phase 4             | Blocked by plugin subsystem            |
| CLI-T-10        | Future plugin runtime guard              | No mutating plugin workflows until source-map and failure behavior are documented                                                            | Deferred beyond current plan | Blocked by plugin/source-map plans     |

## Exit Code Matrix

| AC id           | Rust command or library module                | Required output shape                                           | Implementation status | Blocked-by-parser status        |
| ------- | --------------------------------------------- | --------------------------------------------------------------- | --------------------- | ------------------------------- |
| CLI-E-1         | `cem_ml::error` and `cem_ml_cli` process exit | Exit `0` on success                                             | Planned Phases 2-4    | No                              |
| CLI-E-2         | `cem_ml::error` and fail-level evaluation     | Exit `1` for parse, validation, strict-mode, or budget failures | Planned Phases 3-6    | No for mapping; content blocked |
| CLI-E-3         | Clap and command validation                   | Exit `2` for CLI usage errors                                   | Planned Phase 4       | No                              |
| CLI-E-4         | Future schema error variant                   | Exit `3` reserved for schema resolution errors                  | Reserved Phase 2      | Blocked by schema subsystem     |
| CLI-E-5         | Future transform error variant                | Exit `4` reserved for transform failures                        | Reserved Phase 2      | Blocked by transform subsystem  |
| CLI-E-6         | Future plugin error variant                   | Exit `5` reserved for plugin failures                           | Reserved Phase 2      | Blocked by plugin subsystem     |
| CLI-E-7         | `cem_ml::error` and file I/O                  | Exit `6` for read/write failures                                | Planned Phases 2, 7   | No                              |
| CLI-E-8         | `cem_ml::error` and panic/internal handling   | Exit `7` for unexpected internal failures                       | Planned Phase 2       | No                              |

## Native Node And Nx Matrix

| AC id           | Rust command or library module  | Required output shape                                                                                                         | Implementation status                                                           | Blocked-by-parser status |
| ------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ------------------------ |
| CLI-N-1         | Rust/Cargo package metadata     | Node ESM package requirement does not apply; Rust replacement is Cargo workspace package metadata                             | Rust-specific replacement exists                                                | No                       |
| CLI-N-2         | Rust/Cargo development workflow | Native Node TypeScript execution does not apply; use Cargo through Nx targets                                                 | Rust-specific replacement exists                                                | No                       |
| CLI-N-3         | Rust/Cargo test workflow        | No `ts-node`, `tsx`, Babel, Jest, or Vitest for Rust CLI tests; use Cargo tests through Nx                                    | Rust-specific replacement exists                                                | No                       |
| CLI-N-4         | Rust/Cargo test workflow        | Native Node test runner does not apply; use Rust unit/integration tests                                                       | Rust-specific replacement planned Phase 8                                       | No                       |
| CLI-N-5         | Nx project targets              | Use workspace package manager and Nx targets for build, lint, test, run; add fixture validation only after real parser exists | Base targets declared; project discovery confirmed with `yarn nx show projects` | No                       |
