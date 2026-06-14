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

## Major Requirement: Root Scope And Run Configuration

The document root is the root scope for the whole AST tree. It MUST use the same
scope model as any internal AST scope: descendants inherit root parameters until an
inner scope explicitly overrides them, and diagnostics/source maps must be able to
report which effective scope parameters were active.

Root-scope input parameters MUST be available through the Rust library API, the WASM
API, and the CLI. The shared model must include at least:

- default content type;
- schema identity and version pins;
- default namespace binding;
- named namespace bindings;
- module map / resolver identity;
- base URI;
- scope policy and resource budget hooks needed by validation, module resolution, and
  plugin/adaptor dispatch.

The input → internal AST → output chain MUST accept output scope options as well.
Output identity is not just an output format enum: outputs can declare content type,
schema identity, namespace bindings, version pins, base URI, and module-map/resolver
identity where the target adapter needs them.

Configuration surfaces:

- Rust library and WASM APIs MUST accept structured arrays of input specs and output
  specs so one run can validate or transform multiple sources without collapsing their
  per-document root scopes.
- Config parsing, validation, normalization, and defaulting MUST be implemented in
  `cem_ml`. The CLI, WASM adapter, and Rust callers provide raw config bytes or raw
  spec-record strings plus a declared config content type; they must not carry separate
  config semantics.
- Config files are structural data too: config bytes + config content type + config
  schema/namespace identity are parsed through the CEM-ML-owned config lifecycle before
  document parsing starts. JSON is the first supported config content type; CEM-native,
  YAML, or CSV config documents can be added later as content-type adapters that produce
  the same normalized `RunConfig`.
- CLI MUST support a config-file surface for reproducible CI/build use. The file is the
  preferred shape for multi-source runs, module maps, namespace maps, and multiple
  outputs.
- CLI MAY also support repeatable CSV option values for concise one-liners. CSV is a
  convenience projection of the same input/output spec arrays, not a separate behavior
  path. It must be parsed with deterministic escaping rules rather than ad-hoc comma
  splitting.

Multi-document build/CI runs MUST share one scheduler/thread-pool run context across
documents where safe, while keeping per-document root scopes, diagnostics, source maps,
and resource policy accounting isolated. This is required for validating or transforming
many data sources in one CLI invocation without paying one full runtime setup cost per
document.

Current implementation status:

- `parse`, `validate`, `check`, `inspect`, `convert`, and fixture flows already route
  through the `cem_ml::engine::CemMlEngine` trait.
- `cem_ml::run_config::RunConfig` defines the shared structured shape for input specs,
  output specs, root scope configuration, and scheduler configuration. `cem_ml` owns
  `parse_run_config(bytes + FormatIdentity)`, plus repeatable CSV `InputSpec` /
  `OutputSpec` record parsing. CLI accepts `--config`, `--config-content-type`,
  repeatable `--input-spec`, and repeatable `--output-spec` by delegating parsing to
  `cem_ml`; WASM exposes helpers over the same library parser. This is the first
  execution slice: input specs override global input content-type/schema/base URI during
  lifecycle dispatch, and the first output spec can select conversion target content
  type plus destination. Config diagnostics for malformed JSON, unsupported config
  content type, duplicate input URIs, and unknown output input references fail before
  document parsing. `--observe-events` uses the same normalized input list and
  lifecycle dispatch path as parser-backed commands, including `--input-spec` and
  `--config` inputs. Config-file convert execution fans out multiple `outputs[]`
  records, using `inputRef` or the sole configured input for each output. Normalized
  `RunConfig.scheduler` flows into the engine execution context, and the trace worker
  policy is derived from that scheduler config. Validate/check reports embed a shared
  run-level scheduler trace with per-document scope IDs. Validate/check execute
  lifecycle loading and parser-backed validation through scheduler-dispatched
  per-document tasks, so the trace reflects actual document work instead of report
  projection only. Convert can write explicit side reports from scheduler traces returned
  by engine convert execution while preserving content-primary stdout/`--out` behavior.
  Full input and output root-scope config reaches engine requests. Recognized
  root-scope scheduler policy and budget fields derive the per-scope worker policy for
  scheduled validate/check, trace, and convert execution; `parseMs` enforces a
  parser-backed pipeline wall-clock budget; `validateMs` and `checkMs` enforce
  scheduled per-input document work budgets; `convertMs` enforces input/output-scope
  convert work budgets; `traceMs`, `inspectMs`, `benchMs`, `fixtureValidateMs`,
  `fixtureRoundtripMs`, and `observeMs` enforce trace, inspect, benchmark, fixture,
  and observability workflow budgets; and effective `baseUri` values project relative
  report input and diagnostic URIs.
  Root-scope default and named namespace bindings seed schema validation's
  document-root namespace context. Recognized CEM-ML root-scope version pins resolve
  against the embedded document-format version. Input root-scope module maps provide
  the resolver base for relative schema `src` identities, load local JSON alias maps
  for schema-source specifier resolution, and normalize relative module-map paths
  against the config document path, including local `file://` config document bases.
  Config-file output destinations normalize relative paths against the config document
  path, including local `file://` config document bases.
  Run-config normalization validates root-scope module-map, namespace, and version-pin
  option shape before document parsing, while unreadable or malformed module maps,
  unknown future budget keys, and unsupported version-pin targets emit deterministic
  execution diagnostics instead of being silently ignored. Remote/custom module-map
  URI values now emit an explicit unsupported-resolver diagnostic instead of being
  treated as local paths, and CLI file-write paths reject remote/custom URI
  destinations explicitly. Real remote/custom module-map resolver semantics and real
  remote/custom output resolver semantics remain pending.
- `--schema` and `--content-type` are carried in `EngineContext` and emitted in reports.
  `cem_ml::lifecycle::LifecycleRegistry` now owns built-in input content-type dispatch
  for parser-backed commands (`parse`, `validate`, `check`, `inspect`, `convert`,
  `trace`, `bench`, and fixture workflows). CEM core schema identity
  (`https://cem.dev/ns/core/1`) selects the CEM adapter when no content type is present,
  while explicit content type remains authoritative. CEM/HTML target export selection is
  registry-owned for `convert --to-content-type application/cem+xml`,
  `convert --to-schema https://cem.dev/ns/core/1`, and
  `convert --to-content-type text/html` / `application/xhtml+xml`; unsupported target
  schemas emit a deterministic lifecycle diagnostic while preserving the requested
  fallback output projection. Remaining non-CEM schema/namespace-specific export
  adapters are still pending.
- Root-scope configuration is not complete yet. Current execution uses run-config identity
  fields for lifecycle dispatch and conversion output selection, recognized scheduler
  policy/budget fields for scheduled worker policy, `parseMs` for parser-backed
  wall-clock enforcement, `validateMs` / `checkMs` for scheduled per-input document
  work budgets, `convertMs` for input/output-scope convert work budgets, and
  `traceMs` / `inspectMs` / `benchMs` / `fixtureValidateMs` / `fixtureRoundtripMs` /
  `observeMs` for trace, inspect, benchmark, fixture, and observability workflow
  budgets.
  Effective `baseUri` values project relative report input and diagnostic URIs.
  Default and named namespace maps seed schema validation's root namespace context.
  Recognized CEM-ML version pins resolve through the document-format
  version resolver. Input module maps resolve relative schema-source identities and
  local JSON aliases, including config-relative module-map paths, local `file://`
  module-map URIs, and local `file://` config document bases. Config-file output
  destinations normalize relative paths against the config document path, including
  local `file://` config document bases. Configured, positional, and fixture-materialized
  input reads resolve local `file://` URIs, and primary output, side-report, and
  observability event writes resolve local `file://` destinations to filesystem paths
  and reject remote/custom URI destinations until a resolver is implemented. Remote/custom
  module-map URI values emit an unsupported-resolver diagnostic until a resolver is
  implemented. Real remote/custom module-map resolver semantics and real remote/custom
  output resolver semantics remain planned requirements.
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
- Output identity selection by content type and schema, with `--to-format cem|html|dom-json|ast|events`
  retained for current projections and debug layers.
- Root-scope configuration for inputs and outputs: default content type, schema,
  version pins, default namespace, named namespaces, module map / resolver identity,
  base URI, scope policy, and resource budgets.
- Multi-source configuration via config file, plus repeatable CSV option records for
  CLI one-liners. Config files are preferred for CI/build reproducibility.
- Config-file content type via `--config-content-type`, inferred from extension when
  omitted for known config formats.
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
- Benchmark JSON (`cem-ml bench`): `packages/cem_ml_cli/dist/cem-ml.bench.report.json`

JSON, XML, and CEM-native reports are structured projections. Text, Markdown, and HTML
reports are reference-implementation convenience projections.

## Exit Codes

- `0`: success
- `1`: parse, validation, strict-mode, or benchmark budget failure
- `2`: CLI usage error, including reserved commands
- `3`: schema resolution error, reserved
- `4`: transform failure, reserved
- `5`: plugin failure, reserved
- `6`: I/O failure
- `7`: unexpected internal failure

## Verification Scope

Rust-side tests should assert functional behavior, option parsing, JSON/report fields, diagnostics, and exit codes.
