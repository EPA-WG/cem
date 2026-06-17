# `cem-ml-cli` Implementation Plan

**Status:** Parser-backed implementation exists for the current built-in CEM/HTML/XML surfaces. This document now
tracks the next design step: schema + content-type lifecycle dispatch and the remaining root-scope policy/budget
execution work after the immediate XSLT 1.0 adapter landed.

Phase 1 artifact: [`docs/cem-ml-parser-schema-adr.md`](./cem-ml-parser-schema-adr.md).

This plan defines the `cem-ml` CLI feature set for the Rust platform. The lifecycle requirement in
[`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md) is the active command-shape and format-identity source.

- App crate: `packages/cem_ml_cli`, Cargo package `cem-ml-cli`, binary `cem-ml`.
- Library crate: `packages/cem_ml`, Cargo package `cem-ml`, Rust crate `cem_ml`.

The goal is to provide useful parser/runtime CLI capabilities: command workflows, option
semantics, report fields, diagnostic fields, fail-level behavior, and exit codes.

## Immediate Goal - Structural Lifecycle And XSLT 1.0 Adapter

Promote the CLI/lib contract from fixed parser projections to the structural data
lifecycle defined in [`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md):

1. validate input bytes against a declared content type + schema identity;
2. load the validated structure into the internal CEM event stream / AST;
3. export that internal representation into a declared target content type + schema identity.

The first implementation target for this lifecycle is XSLT 1.0:

- `application/xslt+xml` / `text/xsl` / `text/custom-element-xslt` inputs are recognized
  as XSLT 1.0-family content types;
- the existing legacy custom-element XSLT 1.0 compatibility lowering
  (`cem_ml::legacy_custom_element`) becomes a registered input adapter instead of a
  one-off `convert` branch;
- CLI validation can run the XSLT adapter directly, producing diagnostics for unsupported
  or malformed XSLT 1.0 constructs before export;
- CLI conversion can load XSLT through the adapter and export canonical CEM-ML, light-DOM HTML, DOM JSON, AST, events,
  or later XML outputs through the same engine path.

This goal is separate from the deferred XSLT 3.0/4.0 execution engine. It covers the
XSLT 1.0 structural compatibility profile needed by copied custom-element templates and
the immediate CLI lifecycle contract.

### Design Changes

1. Add a `FormatIdentity` model in `cem-ml`:
    - `content_type: ContentType`
    - `schema: Option<SchemaIdentity>`
    - `base_uri: Option<String>`
    - optional namespace bindings / version pins when a parser discovers them.
2. Move format identity from global context toward per-input and per-output declarations:
    - `EngineInput` carries its input `FormatIdentity`;
    - `ConvertRequest` carries both source and target `FormatIdentity`;
    - `ValidateRequest` validates each input against its own identity.
3. Promote root-scope configuration to the shared API model:
    - every document root is scope `0` for that AST tree and carries the same
      parameters as internal scopes;
    - root input scope config includes default content type, schema identity, version
      pins, default namespace, named namespace map, module map / resolver identity,
      base URI, scope policy, and resource budgets;
    - output scope config mirrors the same identity fields so export adapters can
      choose content type, schema, namespace, version, and resolver behavior from the
      declared target, not from CLI-only flags.
4. Add a lifecycle adapter trait in `cem-ml`:
    - `matches(identity)`;
    - `load(bytes, identity) -> events / AST + diagnostics`;
    - `validate(ast/events, identity) -> diagnostics`;
    - `export(ast, identity) -> bytes/projection + source map`.
5. Back the lifecycle with a registry:
    - built-in adapters: CEM-ML, HTML parity, XML parity, legacy custom-element XSLT 1.0;
    - future adapters registered through the existing plugin descriptor/content-type model;
    - deterministic adapter selection errors when no adapter matches or more than one adapter matches.
6. Define one serializable run configuration shared by lib, WASM, and CLI:
    - Rust library and WASM APIs accept arrays of input specs and output specs;
    - CLI accepts a config file for the same shape, optimized for CI/build reproducibility;
    - CLI also accepts repeatable CSV option records for one-liners. CSV records must be
      parsed with deterministic escaping and map to the same input/output spec structs;
    - one run can contain multiple data sources and multiple outputs while preserving
      isolated root scopes per input.
   - config parsing belongs to `cem_ml`: hosts pass raw config bytes / raw spec-record
     strings plus config `FormatIdentity`, then consume the normalized `RunConfig`.
   **Current slice:** `cem_ml::run_config::RunConfig` and root `ScopeConfig` exist;
   `cem_ml::run_config::parse_run_config(bytes + FormatIdentity)` supports JSON config
   documents by content type; CLI accepts `--config`, `--config-content-type`,
   repeatable `--input-spec`, and repeatable `--output-spec`; WASM exposes JSON
   normalization and CSV spec parsing helpers over the same library parser. Input specs
   override global input identity for lifecycle dispatch, and the first output spec can
   select conversion target identity/destination. CEM core schema or namespace identity
   (`https://cem.dev/ns/core/1`) now selects the CEM adapter when no content type is
   present, and HTML/SVG namespace identity selects the HTML adapter when no content type
   or schema is present. XSLT namespace identity (`http://www.w3.org/1999/XSL/Transform`)
   selects the legacy custom-element XSLT compatibility adapter when no content type or
   schema is present, while explicit content type remains authoritative. Unsupported input
   and target identities emit deterministic lifecycle diagnostics with the declared
   content type, schema, and/or namespace while preserving the requested fallback input
   format or output projection. Config diagnostics for
   malformed JSON, unsupported config content type, duplicate input URIs, and unknown
   output input references fail before document parsing. `--observe-events` consumes the same
   normalized input list and lifecycle dispatch path as parser-backed commands, including
   `--input-spec` and `--config` inputs. Config-file convert execution fans out multiple
   `outputs[]` records, using `inputRef` or the sole configured input for each output.
   Normalized `RunConfig.scheduler` flows into the engine execution context, and the
   trace worker policy is derived from that scheduler config. Validate/check reports now
   embed a shared run-level scheduler trace with per-document scope IDs. Validate/check
   execute lifecycle loading and parser-backed validation through scheduler-dispatched
   per-document tasks, so the trace reflects actual document work instead of report
   projection only. Convert can now write explicit side reports from scheduler traces
   returned by engine convert execution while preserving content-primary stdout/`--out`
   behavior. Full input and output root-scope config reaches engine requests.
   Recognized root-scope scheduler policy and budget fields derive the per-scope
   worker policy for scheduled validate/check, trace, and convert execution; `parseMs`
   enforces a parser-backed pipeline wall-clock budget; `validateMs` and `checkMs`
   enforce scheduled per-input document work budgets; `convertMs` enforces
   input/output-scope convert work budgets; `traceMs`, `inspectMs`, `benchMs`,
   `fixtureValidateMs`, `fixtureRoundtripMs`, and `observeMs` enforce trace, inspect,
   benchmark, fixture, and observability workflow budgets; and effective `baseUri`
   values project relative report input and diagnostic URIs. Root-scope default and
   named namespace bindings seed
   schema validation's document-root namespace context, and recognized CEM-ML
   root-scope version pins resolve against the embedded document-format version.
   Input root-scope module maps provide the resolver base for
   relative schema `src` identities, load local JSON alias maps from paths and local
   `file://` URIs for schema-source specifier resolution, and normalize relative
   module-map paths against the config document path, including local `file://`
   config document bases. Config-file output destinations normalize relative paths
   against the config document path, including local `file://` config document bases.
   Configured, positional, and fixture-materialized input
   reads resolve local `file://` URIs, and primary output, side-report, and
   observability event writes resolve local `file://` destinations to filesystem paths.
   Run-config normalization validates root-scope module-map,
   namespace, and version-pin option shape before document parsing, while unreadable or
   malformed module maps, unknown future budget keys, and unsupported version-pin
   targets emit deterministic execution diagnostics instead of being silently ignored.
   Schema artifacts: the JSON `RunConfig` surface
   (`https://cem.dev/ns/cli/run-config/1`) has checked-in JSON Schema
   `packages/cem_ml/schema/cli/run-config.schema.json`
   (`https://cem.dev/schema/cli/run-config.schema.json`), and the CEM-native CLI
   transform graph config has checked-in schema artifact
   `packages/cem_ml/schema/cli/transform-config.md`
   (`https://cem.dev/ns/cli/transform-config/1`) for the `run` / `import` /
   `transform` / `export` graph syntax. The transform-config schema is separate from
   CEM core document schemas and from CEM-native template schemas. Generated/published
   distribution wiring copies the JSON schema into `packages/cem_ml/dist/cli/` during
   `cem_ml:build:docs`, keeps the transform config markdown/XHTML artifacts in the same
   CLI dist tree, and validates those artifacts through `cem_ml:test:cli-schema-artifacts`.
   Remote/custom module-map URI values, config document reads, configured and
   positional input reads, and fixture-materialized input reads use registered
   `EngineContext` resolvers when a host installs one. Default CLI behavior stays
   local-only, while `--resolver-read-map URI-PREFIX=DIR`, `--resolver-write-map
   URI-PREFIX=DIR`, and run-config `resolvers` entries explicitly map remote/custom
   URI prefixes to local filesystem roots. Primary output, side-report, and
   observability event writes use registered write resolvers when a host installs one;
   default CLI behavior still rejects remote/custom destinations instead of treating
   them as local paths unless a write map is registered.

   **Resolver implementation plan:** URI handling is being promoted into the shared
   `cem_ml::resolver` boundary instead of command-specific path checks.
    - Done: added `ResolvePurpose` (`config`, `input`, `template`, `moduleMap`, `output`,
      `report`, `observeEvents`), `ResolveDirection`, `ResolveRequest`,
      `ResolvedRead`, `ResolvedWrite`, `ResolverDiagnostic`, `ResourceResolver`,
      and `ResolverRegistry` in `cem_ml`.
    - Done: moved local path and local `file://` parsing into the library resolver
      module. The CLI, run-config normalization, and current real-engine
      module-map/materialized-input paths call the same local resolver code.
    - Done: threaded the resolver registry through non-serialized `EngineContext`.
      `RunConfig` remains serializable; host-only resolver objects live in runtime
      context only.
    - Done: converted module-map loading in `cem_ml::real` to registered resolver
      reads. The resolved module-map URI provides the base for relative schema `src`
      identities while command-facing diagnostics remain stable.
    - Done: config-document reads plus configured/positional input helpers use
      purpose-aware resolver reads when the runtime context provides a matching
      resolver. Existing fixture and benchmark materialized-input reads remain
      resolver-aware; config-backed fixture observability collection reuses the same
      helper path.
    - Done: transform request construction reads template resources through the dedicated `template` resolver purpose
      so template access can be authorized independently from data input access.
    - Done: empty fixture placeholder materialization for pre-engine observability and
      template-embedding diagnostics now uses the same input resolver path while
      preserving repo-relative fixture lookup for ordinary paths.
    - Done: converted output writes to resolver writes for `--out`,
      config/output-spec destinations, side reports, and `--observe-events`. Default
      CLI context keeps rejecting remote/custom writes until a host or CLI option
      registers a resolver for that scheme, purpose, and direction.
    - Done: added WASM `onResolveRead` / `onResolveWrite` callback registration plus
      `JsResourceResolver` and resolver-registry helpers for Rust-side WASM
      entrypoints.
    - Done: added optional CLI local mirror resolver registration through
      `--resolver-read-map`, `--resolver-write-map`, and run-config `resolvers`
      entries. The default CLI behavior stays local-only until a resolver is explicitly
      registered.
    - Keep security policy explicit: resolver registration is per scheme and purpose;
      read permission does not imply write permission, and remote output writes are not
      enabled by default.
7. Update CLI flags without breaking current debug workflows:
    - keep `--from-format` and `--to-format` as aliases for built-in identities;
    - keep `--content-type` as the input content type for `parse`, `validate`, `check`,
      `inspect`, and `convert`;
    - add explicit target identity flags for conversion, including `--to-content-type`,
      `--to-schema`, `--default-namespace`, and repeatable `--namespace PREFIX=URI`;
    - expose command-level root-scope defaults with `--default-namespace`,
      repeatable `--namespace PREFIX=URI`, `--module-map`, repeatable
      `--version-pin NAME=CONSTRAINT`, `--scope-policy`, and repeatable
      `--scope-budget NAME=VALUE`;
    - continue supporting `--schema` for input schema identity until split input/output schema
      flags land.
8. Replace the current XSLT special case in `RealCemMlEngine::convert` with the adapter
   registry path, then route `validate` through the same adapter so raw XSLT can be
   CLI-validated without a two-command convert-then-validate workaround.

### Run Configuration Shape

The API-level shape is an array model, not a set of global options:

```text
RunConfig {
  inputs: Vec<InputSpec>,
  outputs: Vec<OutputSpec>,
  scheduler: SchedulerConfig,
}

InputSpec {
  uri,
  bytes | stream,
  root_scope: ScopeConfig,
}

OutputSpec {
  input_ref,
  destination,
  root_scope: ScopeConfig,
}

ScopeConfig {
  default_content_type,
  schema,
  version_pins,
  default_namespace,
  namespaces,
  module_map,
  base_uri,
  policy,
  budgets,
}
```

The CLI config-file form should preserve this structure directly for implemented
parse/validate/convert workflows. A config file is the recommended interface for
build/CI validation because it is reviewable, reproducible, and can represent nested maps
without shell quoting hazards. The config document itself is parsed by `cem_ml` using its
declared content type. The CLI may infer `application/json` from `.json`, but it does not
own JSON semantics.

Transform graph configuration is CEM-ML-first, not JSON-first. Do not add JSON
`transforms[]` as the first graph config surface. The future transform config shape uses
nested CEM-ML nodes:

```cem
{run |
  {import @id="book" @src="inputs/*.xml" @content-type="application/xml" |
    {transform @id="base" @src="templates/book.xslt" @template-content-type="application/xslt+xml" |
      {export @out="book/chapters/{stem}.html" @content-type="text/html"}

      {transform @id="html" @src="templates/book2html.xslt" |
        {export @out="book/chapters/{stem}.html" @content-type="text/html"}
      }

      {transform @id="chart1" @src="illustrations/chart1.xslt" |
        {export @out="book/chapters/{stem}/img/chart1.svg" @content-type="image/svg+xml"}
      }

      {transform @id="chart2" @src="illustrations/chart2.xslt" |
        {export @out="book/chapters/{stem}/img/chart2.svg" @content-type="image/svg+xml"}
      }
    }
  }
}
```

Initial graph semantics:

- `import` creates initial artifact nodes from external resources.
- `transform` consumes its parent artifact by default and produces a child artifact.
- `export` consumes its parent artifact and writes it without mutating sibling branches.
- sibling nodes branch from the same parent artifact.
- `@id` names import/transform artifacts; IDs are unique across the graph.
- `@input` overrides the primary input artifact for a transform.
- `@with:*` adds named secondary artifacts for explicit cross-input joins.
- references must resolve to existing artifacts and must not create cycles.
- one import source match produces one artifact, and one transform produces one artifact,
  until explicit future split/group/reduce/flat-map transform kinds are designed.
- output paths use named bindings such as `{stem}`, `{path}`, and `{index}`; repeated
  resolved output paths are configuration errors unless an explicit overwrite policy is
  added later.
- CLI graph config dispatch currently expands local filesystem import globs and
  resolver-backed import globs with exactly one `*` in the file name and an
  optional single `**` directory segment for recursive descent. Match order is
  sorted and creates source bindings: `{src}`, `{path}`, `{dir}`, `{file}`,
  `{stem}`, `{ext}`, and `{index}`. One-to-one transform stages preserve those
  bindings for all sibling export branches. Unknown output bindings fail as
  config errors; duplicate resolved destinations fail before writes.

CSV CLI options are a convenience for small invocations, not the primary source of
truth. A future CLI should prefer repeatable records such as:

```bash
cem-ml validate \
  --input-spec 'uri=src/a.cem,contentType=application/cem+xml,schema=https://cem.dev/ns/core/1,defaultNs=https://cem.dev/ns/core/1,namespaces=html:https://www.w3.org/1999/xhtml|svg:http://www.w3.org/2000/svg,moduleMap=cem.modules.json'

cem-ml convert \
  --input-spec 'uri=src/a.cem,contentType=application/cem+xml,schema=https://cem.dev/ns/core/1' \
  --output-spec 'input=src/a.cem,dest=dist/a.cem,contentType=application/cem+xml,schema=https://cem.dev/ns/core/1'
```

`convert` is the implemented document-to-document command. The direct one-input shape remains:

```bash
cem-ml convert input.xml \
  --content-type application/xml \
  --to-content-type application/cem+xml \
  --out output.cem
```

`transform` is the data + template -> document command. The current CLI runtime supports the one-to-one CEM-native
template path and CEM-ML graph config dispatch for concrete import/template/export paths. XML+XSLT execution remains
deferred:

```bash
cem-ml transform data.xml \
  --data-content-type application/xml \
  --template view.xsl \
  --template-content-type application/xslt+xml \
  --to-content-type text/html \
  --out view.html
```

The transform shape also accepts `--data-schema URI-OR-FILE`, `--template-schema URI-OR-FILE`,
`--to-schema URI-OR-FILE`, shared context options, and `--report-json` / `--report-md`. The rendered document writes to
`--out` when provided, otherwise stdout. Diagnostics and warnings write to stderr unless a report destination is
provided. Single-transform reports include `reportAst.transform` with input, destination, output kind, source-map
presence, output-span count, and a `{destination}.map` `sourceMapRef` when the transform response has a source map and a
concrete output destination. Stdout transform output still reports source-map presence and output-span count, but omits
`sourceMapRef` because there is no adjacent file path for a sidecar.

The remaining CEM-native parity closure before XSLT adds named module invocation to the user-facing surfaces:

```bash
cem-ml transform data.cem \
  --data-content-type text/cem-ml \
  --template templates/page.cem \
  --template-schema https://cem.dev/ns/template/cem-native/1 \
  --template-entrypoint card \
  --param locale=fr-FR \
  --param title=Intro \
  --to-content-type text/html
```

One-line CLI params are repeatable `NAME=VALUE` records. Values enter the engine as strings and are normalized by the
CEM-native module declaration for the selected entrypoint: boolean/number/integer/array/object/json params parse the
string as the declared shape, nullable literal `null` becomes JSON null, and non-nullable `any`/`string` keep text such
as `null` as a string. The direct CLI stays one input, one template, one primary output; multiple outputs remain graph
branches and exports.

The same closure extends CEM-ML transform graph config with `transform @entrypoint` and child `param` records:

```cem
{@doc cem-ml 1}
{run |
  {import @id="book" @src="inputs/*.cem" @content-type="text/cem-ml" |
    {transform
      @id="html"
      @src="templates/page.cem"
      @template-schema="https://cem.dev/ns/template/cem-native/1"
      @entrypoint="card" |
      {param @name="locale" @value="fr-FR"}
      {param @name="title" @value="{stem}"}
      {export @out="book/chapters/{stem}.html" @content-type="text/html"}
    }
  }
}
```

Config `param @value` uses the same string-first normalization as `--param`; output/source bindings such as `{stem}`
are expanded by the CLI host before the engine request is built. CEM-native module execution applies declaration-driven
type/null/default validation after lowering. XSLT parity uses the same direct CLI and graph config surfaces for named
template entrypoints and string params, passing params as `xsl:with-param` values. `cem-ql-fragment` still requires the
implicit entrypoint and no params.

Graph configs use the dedicated CEM-ML transform-config schema identity
`https://cem.dev/ns/cli/transform-config/1` and can be run directly:

```bash
cem-ml transform --config graph.cem
```

Relative import/template/export paths are resolved against the config document path. Config graph exports write to their
`@out` destinations through the resolver layer; stdout remains reserved for direct transform output unless an export has
no destination.

Critical constraints:

- CSV cannot be parsed by naive comma splitting. Use a real CSV parser or a constrained
  key/value grammar with quoting/escaping tests.
- Namespace maps and module maps are structurally nested. For anything beyond simple
  one-liners, require or strongly recommend the config-file form.
- Global flags may remain as defaults, but per-input/per-output records must override
  them. This lets one CI invocation validate mixed content types and schemas safely.
- The scheduler/thread-pool context belongs to the run, not to one document. Work can be
  shared across documents, but scope policy, diagnostics, source-map stacks, and resource
  budget accounting remain per document/root scope.
- Config diagnostics happen before document parsing. Invalid config content type,
  malformed JSON, invalid module-map declarations, bad namespace maps, invalid version
  pins, or output references to unknown inputs must fail as configuration diagnostics
  instead of document-level validation noise.

## Explicit Scope

- For the lifecycle-dispatch increment, do not redesign the existing tokenizer, normalizer, AST builder, or
  validation-rule catalog.
- Do not implement the deferred XSLT 3.0/4.0 execution engine in this increment.
- Do not implement the shared multi-document scheduler/thread-pool as part of the current
  XSLT lifecycle adapter slice. It is a required run-configuration follow-up.
- Keep `cem-ml-cli` thin. Shared behavior belongs in `cem-ml`; CLI commands only select identities, wire I/O, and
  render reports/projections.

## Phase 0 - Feature Baseline

**Status:** Complete. The feature baseline is captured in this plan and summarized in
[`docs/cem-ml-cli-contract.md`](./cem-ml-cli-contract.md).

1. Confirm the platform outputs:
    - binary: `cem-ml`
    - default fixture report paths:
        - `packages/cem_ml_cli/dist/cem-ml.report.json`
        - `packages/cem_ml_cli/dist/cem-ml.report.md`
        - `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.json`
        - `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.md`
        - `packages/cem_ml_cli/dist/cem-ml.bench.report.json`
    - canonical fixture inputs live in `examples/cem-ml/*.cem`.
    - existing `examples/semantic/*.html` files remain secondary HTML parity fixtures.
2. Define exit codes:
    - `0`: success
    - `1`: parse, validation, strict-mode, or benchmark budget failure
    - `2`: CLI usage error
    - `3`: schema resolution error, reserved
    - `4`: transform failure
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
    - `report`: AST-associated report tree and CEM/XML/JSON renderers; text/HTML reference convenience renderers
    - `formats`: parse output format names and conversion output names
    - `fixture`: default fixture paths and fixture report path policy
    - `engine`: trait boundary for parse/validate/inspect/trace/bench inputs
    - `command`: I/O-independent command orchestration
    - `error`: usage, I/O, schema, transform, plugin, and internal error mapping
4. Use Rust type names with `CemMl` where a prefix is useful, but keep JSON field names compatible with the active
   feature documents.
5. Add serialization dependencies only when the implementation phase starts:
    - `serde`
    - `serde_json`
    - optional `thiserror`

## Phase 3 - Shared CLI Types In `cem-ml`

1. Define diagnostic types matching the documented JSON shape:
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
3. Define report models matching the documented CLI shape. The internal model is an AST-associated report tree, not a
   flat diagnostics array:
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
    - `reportAst.schedulerTrace.eventCount`
    - `reportAst.schedulerTrace.events[]`
    - optional `reportAst.transform`, with `input`, `destination`, `outputKind`, `hasSourceMap`, `outputSpanCount`,
      and optional `sourceMapRef`
    - optional `reportAst.transformGraph.exportCount`
    - optional `reportAst.transformGraph.exports[]`, with `exportId`, `input`, `destination`, `contentType`, `schema`,
      `outputKind`, `hasSourceMap`, `outputSpanCount`, optional `sourceMapRef`, and optional `collectionItems[]` for
      collection exports
    - event nodes with source module state, event sequence, source-map stack at event time, and visible partial
      hierarchy
4. Use a deterministic default timestamp for feature tests:
    - `1970-01-01T00:00:00.000Z`
5. Define command output models for:
    - DOM JSON
    - AST
    - events
    - inspect summary
    - trace report
    - benchmark report
    - fixture roundtrip report

These are data shapes only. Parser-filled content remains blocked until the parser decision phase is complete.

## Phase 4 - CLI Command Surface

1. Implement Clap command declarations for the planned functional surface, with binary name `cem-ml`:
    - `parse <input>`
    - `validate <input...>`
    - `check <input...>`
    - `inspect <input>`
    - `bench <input...>`
    - `convert <input>`
    - `trace <input>`
    - `fixture validate [input...]`
    - `fixture roundtrip [input...]`
    - `transform <data> --template <file>`
    - `version`
    - `help`
2. Preserve common options:
    - `--fail-level parse|validate|strict`
    - `--format text|html|json|xml|cem|markdown|dom-json|ast|events|tree`
    - `--from-format cem|html|xml`
    - `--to-format cem|html|dom-json|ast|events`
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
3. Reserve these commands with exit code `2` until their subsystem plans or implementations exist:
    - `transform <data> --template <file>` with `--data-content-type`, `--data-schema`,
      `--template-content-type`, `--template-schema`, `--to-content-type`, `--to-schema`, and `--out`
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
3. Provide a fake engine only for feature tests.
4. Do not add a real parser engine in this plan.
5. Do not mark parser-backed AC complete until the future parser implementation exists.
6. Keep the command orchestration complete enough that replacing the fake engine with the real engine does not require
   changing Clap definitions, output models, report writers, or exit-code logic.

## Phase 6 - Command Behavior

1. `cem-ml parse <input>`
    - Default format: `dom-json`.
    - Supported formats: `dom-json`, `json`, `ast`, `events`.
    - Default fail level: `parse`.
    - Writes primary output to stdout or `--out`.
    - Supports parse diagnostic side reports with `--report-json` and `--report-md`.
2. `cem-ml validate <input...>`
    - Supported structured formats: `json`, `xml`, `cem`.
    - Reference convenience formats: `text`, `html`, `markdown`.
    - Default fail level: `validate`.
    - Supports aggregate `--report-json` and `--report-md`.
3. `cem-ml check <input...>`
    - Same data flow as validate.
    - Supports `--zero-hard-violations`.
    - Default fail level: `validate`.
4. `cem-ml fixture validate [input...]`
    - Defaults to the canonical CEM-ML fixtures and HTML parity fixtures when no input is passed.
    - Writes default `cem-ml.report.json` and `cem-ml.report.md`.
5. `cem-ml inspect <input>`
    - Supported `--show`: `summary`, `ast`, `events`, `diagnostics`, `source-offsets`, `tree`.
    - Scope, schema-binding, plugin, and source-map views remain deferred.
6. `cem-ml convert <input>`
    - Converts a supported document in one declared document format into another declared document format.
    - Supported input formats: `cem`, `html`, `xml`.
    - Supported output formats: `cem`, `html`, `xml`, `dom-json`, `ast`, `events`.
    - `--to-content-type application/cem+xml` selects canonical CEM-ML export, and `--to-content-type text/html`
      / `application/xhtml+xml` selects light-DOM HTML export; `--to-content-type application/xml` / `text/xml`
      selects rendered XML output.
    - Schema-version conversion and broader target adapters remain deferred.
7. `cem-ml transform <data> --template <file>`
    - Applies a template/stylesheet to data and produces a document.
    - Supported options: `--data-content-type`, `--data-schema`, `--template`, `--template-content-type`,
      `--template-schema`, `--to-content-type`, `--to-schema`, `--out`, shared context options, `--report-json`, and
      `--report-md`.
    - Current CLI runtime supports the minimal one-to-one CEM-native path through the CEM-QL executable adapter and
      `--config` graph dispatch for concrete paths plus local and resolver-backed filename import globs, optional `**`
      recursive import glob segments, explicit `join @mode="collect"` aggregation, and source-binding
      `join @mode="group-by" @by="..."` aggregation, and same-binding
      `join @mode="match-by" @by="..." @with:...` aggregation, and positional
      `join @mode="zip" @with:...` aggregation.
      `RealCemMlEngine::transform_graph` executes CEM-native graph requests; XML+XSLT execution remains deferred.
    - Current config slice: `cem_ml::transform_config::parse_transform_graph_config` parses CEM-ML
      `run` / `import` / `join` / `transform` / `export` graph config and validates missing required operation
      attributes, duplicate IDs, unresolved refs, cycles, and wildcard output patterns. CLI dispatch lowers this config
      into `TransformGraphRequest`, resolving relative resource and export paths against the config document path and
      validating duplicate destinations after output bindings resolve.
    - Current engine API slice: `cem_ml::engine::TransformGraphRequest` and `TransformGraphResponse` model
      loaded import nodes, collect and source-binding group-by join nodes, template-backed transform stages, export
      nodes, graph dependencies, scheduler scope IDs, emitted artifacts, diagnostics, and scheduler trace. The default
      trait method still returns not implemented.
    - Current design/API slice:
        - Template identity dispatch now supports both XSLT template identities (`application/xslt+xml`, `text/xsl`,
          and legacy custom-element XSLT content types) and CEM-native template identities (`application/cem+xml`,
          `application/cem`, `text/cem`, `text/cem-ml`, and CEM core schema/namespace identity when no content type is
          present). `TransformTemplateKind` records the selected adapter class on request/stage models and graph config
          transform nodes; compilation/execution remains deferred.
        - CEM-native templates are insulated from the base CEM-ML language/AST as transform-template content-type
          adapters. `TransformTemplateAdapterRegistry` owns content-type/schema/namespace matching, has built-in
          CEM-native and XSLT adapters, and is carried by `EngineContext` so hosts can register newer CEM-native
          template iterations at runtime.
        - `TransformTemplateAdapter` now defines the execution-facing plugin boundary: adapters can compile a
          `TemplateInput` plus entrypoint/params and declared data binding names into an opaque compiled artifact, then
          render that artifact against a primary data artifact, optional secondary artifacts, and a target
          identity/scope. Static built-in adapters keep the current behavior by returning deterministic
          adapter-not-implemented errors until a runtime executor is registered.
        - Executable template adapters take precedence over selector-only adapters for the same identity, so a host can
          install an actual CEM-native executor without making the built-in selector capability ambiguous. Multiple
          executable matches remain ambiguous.
        - `cem_ml_transform_cem_ql` is the first executable CEM-native adapter crate. It sits above both `cem_ml` and
          `cem_ql`, registers a CEM-QL-backed template adapter, compiles CEM-ML fragments through
          `cem_ql::render::compile_template`, carries the compiled payload in the adapter artifact, and renders via
          `cem_ql::render::render_compiled_template`.
        - `RealCemMlEngine::transform` now runs the minimal one-to-one CEM-native engine path when an executable
          adapter is registered in `TransformRequest.context`: data is loaded through lifecycle, parsed to DOM JSON,
          passed as the primary transform data artifact, compiled by the selected adapter, rendered, and returned as the
          content-primary `TransformResponse.primary`.
        - The CLI host context registers the CEM-QL executable adapter so transform request construction and dispatch
          use the same adapter registry shape as programmatic hosts.
        - `TransformExecutionPolicy` records the first runtime contract:
          `runtimePhase`, `cardinality=one-to-one`, `duplicateDestinationPolicy=reject`,
          `failurePolicy=fail-fast`, and `outputPolicy=content-primary`. The runtime now accepts
          `cem-ql-fragment`, `cem-native-modules`, and `xslt-parity`; the fragment phase stays CEM-native and
          implicit-entrypoint only, the module phase is the declared CEM-native module phase, and the XSLT phase
          executes bounded XSLT 1.0 compatibility lowering through the registered CEM-QL adapter.
        - `TransformTemplateEntrypoint` and `params` exist on transform requests/stages so the API has a place for
          named-template/module execution. The `cem-ql-fragment` phase supports only the implicit entrypoint and rejects
          params; the `cem-native-modules` phase accepts validated public named entrypoints and declared params; the
          `xslt-parity` phase accepts named template entrypoints and string params through the compatibility adapter.
        - `TransformDiagnosticOrigin` reserves stable report/source-map origin categories for `config`, `import`,
          `template-load`, `template-compile`, `template-execution`, and `export`.
        - `validate_transform_request_runtime_contract` and
          `validate_transform_graph_runtime_contract` now enforce the supported runtime preflight:
          `cem-ql-fragment` is CEM-native, implicit-entrypoint, no-params only; `cem-native-modules` is the CEM-native
          declaration/module phase; `xslt-parity` is XSLT-only with named-template and string-param support. Graph
          runtime phase is carried per `TransformGraphStage`, while graph-wide execution controls remain on
          `TransformGraphRequest`; graph validation also checks known artifact refs, unique graph IDs, and duplicate
          output destination rejection.
        - CEM-native runtime order: first support pure CEM-QL evaluation plus CEM-ML fragments with embedded CEM-QL and
          one implicit entrypoint; then add native named templates/modules, explicit entrypoints, params,
          imports/includes, visibility, caching, and recursion/cycle limits; then expand XSLT parity using that native
          substrate.
        - Remaining CEM-native parity closure before XSLT:
            - Surface named entrypoints and params in the direct CLI via `--template-entrypoint NAME` and repeatable
              `--param NAME=VALUE`; keep string-first CLI param normalization at the module contract boundary.
            - Surface the same controls in CEM-ML graph config via `transform @entrypoint` and child
              `param @name @value` records; expand path/source bindings in `@value` before lowering to engine params.
            - Pass entrypoint/params through CLI one-liner and graph lowering into `TransformRequest` and
              `TransformGraphStage` without changing the engine API shape.
            - Prove resolver-backed imported CEM-native modules end-to-end from CLI one-liner and graph config,
              including relative import resolution, imported call diagnostics, stdout/default output behavior, report
              destinations, and source-map sidecars for configured exports.
            - Done: add separate CLI integration conformance fixtures in
              `packages/cem_ml_cli/tests/cem_native_module_conformance.rs` for the CEM-native module matrix: implicit
              entrypoint, public named entrypoint, private/missing entrypoint diagnostics,
              caller params/defaults/nulls/type coercion, same/imported calls, `@with:*` secondary inputs, nested
              imports, import cycles/depth, and recursion limits. These tests cover example-shaped login/profile/asset
              cases without turning `examples/` into the test fixture location.
            - Keep `include`, `@default-expr` / `@defaultExpr`, adapter extension buckets for unknown params, arbitrary
              template writes, and XSLT execution outside this closure.
          Exit criterion for starting XSLT parity: all supported CEM-native module semantics are available through the
          programmatic API, direct CLI, and CEM-ML graph config with stable diagnostics/reports and schema docs.
        - Native template/module contract for the next runtime/API slice:
            - Compile each template resource as an adapter-owned module. Module syntax and schema rules belong to the
              selected template content-type/schema adapter, not to the stable base CEM-ML document AST.
            - Treat the implicit entrypoint as the module default render entrypoint. Explicit named entrypoints select
              public exported templates and fail during compile/validation when missing or private.
            - Make declarations private by default; require explicit public visibility for cross-module use.
            - Make params immutable for each render call. Template defaults are allowed, caller params override by name,
              and unknown params are fatal unless an adapter declares an extension bucket. A param key with value `null`
              counts as provided; only omitted keys use defaults or fail required-param checks. Selected-entrypoint
              local and qualified param names are aliases, and providing both aliases is fatal.
            - Support v1 param `@type` values `any`, `string`, `boolean`, `number`, `integer`, `array`, `object`, and
              `json`. Omitted `@type` means `any`. Typed caller params are validated as JSON shapes before adapter
              compilation. Params are non-nullable by default; `@nullable="true"` allows explicit JSON `null` caller
              values and literal `@default="null"`. Explicit `null` remains provided for requiredness. Normalize
              string-valued caller params from CLI/config inputs at the module contract boundary: nullable literal
              `null` becomes JSON null, `boolean` accepts `true`/`false`, and
              `number`/`integer`/`array`/`object`/`json` parse JSON before validation; non-nullable `any`/`string` keep
              text such as `null` as a string. Keep `@default`
              literal: `any`/`string` are raw strings, `boolean` is `true`/`false`, and
              `number`/`integer`/`array`/`object`/`json` parse JSON. Reserve `@default-expr` / `@defaultExpr` for future
              expression defaults and reject it until expression context, resolver policy, and reporting are defined.
              Structural declaration values such as import aliases/URIs, template names, param names, call targets, and
              import identity fields trim surrounding whitespace; literal `@default` string values do not.
            - Keep portable data bindings explicit: primary artifact as `input`, named secondary artifacts under their
              graph labels, and params under their names. The CEM-QL data document also exposes those host bindings as
              top-level fields while retaining the legacy `datadom.attributes.*` projection. Direct primary-object
              convenience bindings are adapter compatibility behavior, not a portable template requirement.
            - Implement `import` before `include`. Imports load separate modules through the template resolver, isolate
              private declarations, expose public exports under an alias/namespace, and reject dependency cycles.
              Includes remain reserved until the import, cache, and cycle rules are proven.
            - Resolve relative imports against the importing template URI with resolver purpose `template`; report
              resolver, compile, and render failures through `template-load`, `template-compile`, and
              `template-execution` origins respectively.
            - Cache compiled modules by adapter ID, resolved URI, template identity, content hash, selected entrypoint,
              execution policy, and resolved dependency graph hash. Cache entries must not cross adapter or template
              schema versions.
            - Allow recursive named-template calls only with explicit runtime/scheduler limits; keep fail-fast as the
              default failure policy.
            - Keep output content-primary. Use graph branches and export nodes for multiple files/reports instead of
              arbitrary template writes.
        - Current native module API slice: `TransformTemplateCompileRequest` carries
          `TransformTemplateModuleOptions`; the stable model includes import declarations, entrypoint declarations with
          explicit visibility, param declarations with types/defaults/required flags, non-executing call-site records, module
          limits, module cache keys, and reserved diagnostic codes for private/missing entrypoints, unknown calls,
          unknown, missing required, or type-mismatched params, import cycles, recursion limits, and reserved includes. The real engine now
          parses/lowers v1 CEM-native template declarations into module options before adapter compilation, while leaving
          plain CEM fragment templates declaration-free. It validates named entrypoint requests against public
          declarations, rejects unknown caller params, validates typed caller/default param values, reports missing
          required params, and validates same-module and imported public `call` targets before adapter compilation. Compile requests also carry
          `TransformTemplateModulePreflight`, which the real engine builds before adapter compilation by reading declared
          imports through the template resolver, resolving relative imports against the importing template URI, carrying
          resolved module bytes/identity/content hashes with `parentUri` for non-root import edges, building the
          dependency graph cache-key input, rejecting duplicate aliases per importing module, rejecting reserved includes,
          enforcing import-depth limits, and rejecting import cycles. The CEM-QL executable adapter now compiles
          preflighted modules into its native payload and exposes import metadata on the compiled artifact. It dispatches
          validated same-module and imported module calls during render with the current data context. When an imported
          module's public entrypoint renders, unqualified calls resolve against that imported module's own named
          templates, and `call @from` resolves against that module's own import aliases rather than the root module. Call
          `@with:*` whole-expression attributes preserve their evaluated CEM-QL item stream for the invoked template,
          while literal and mixed attribute-value-template forms remain string bindings. Required params on same-module
          and imported calls are checked after `@with:*` bindings and defaults are applied; missing values report a
          `cem.transform_template.param_required` diagnostic at the call site. Typed call params are checked against the
          invoked template declaration using the same scalar/JSON shape names as caller params; mismatches report
          `cem.transform_template.param_type` at the call site. Whole-expression `@with:*` values that evaluate to
          explicit null count as provided and then follow nullability; whole-expression values that evaluate to an empty
          stream count as omitted. The adapter also preserves the selected entrypoint, caller params, and param
          declarations in its compiled payload; caller params override declaration defaults, omitted defaults are
          applied during same-module and imported renders, explicit `null` stays bound as a caller value, and named
          entrypoint-local params bind through their local names inside the invoked template. Qualified caller params
          such as `card.title` bind equivalently when `card` is selected, while duplicate local+qualified aliases are
          rejected before adapter compilation. The CEM-QL renderer declares the stable primary `input` binding
          at compile time and makes primary/secondary host bindings available as top-level fields on the synthesized data
          document while preserving `datadom.attributes.*` compatibility. Same-module recursive calls, including
          recursive calls inside an
          imported module, are bounded by the module recursion limit and report `cem.transform_template.recursion_limit`
          when exceeded.
        - The v1 CEM-native template declaration schema now has its own identity,
          `https://cem.dev/ns/template/cem-native/1`, and checked-in schema artifact
          `packages/cem_ml/schema/template/cem-native-template.md`. Its declaration vocabulary is `module`, `import`,
          `param`, `template`, `body`, and `call`; `param @type` defines the first JSON-shape type surface,
          `param @nullable` controls nullability, `param @default-expr` / `@defaultExpr` is reserved and fatal, and
          `include` remains intentionally absent/reserved. Built-in and CEM-QL
          executable template adapters recognize this schema identity while preserving legacy CEM core identity fallback
          for existing CEM-native template selection.
        - The CLI transform graph config has its own schema identity,
          `https://cem.dev/ns/cli/transform-config/1`, for `run` / `import` / `join` / `transform` / `export`; do not
          validate transform config as ordinary CEM core content or as a template document.
        - Resolver semantics for reading templates with the dedicated `template` resolver purpose and writing transform
          outputs with the existing local-only default plus registered resolver behavior. Primary one-line CLI output
          writes to `--out` or stdout; report side outputs write through the existing report resolver path.
          CLI graph config execution writes export artifacts to configured `@out` destinations through the resolver
          layer. Programmatic graph execution still returns artifacts in-memory.
        - CLI graph config source bindings are source-derived and immutable in this slice: `{src}`, `{path}`, `{dir}`,
          `{file}`, `{stem}`, `{ext}`, and `{index}`. Imports create bindings, one-to-one transforms preserve them, and
          exports consume them. Local and resolver-backed filename globs with exactly one `*` in the file name and an
          optional single `**` directory segment are expanded by the CLI host. Explicit `join @mode="collect"` nodes
          aggregate all artifacts from their primary input into one collection artifact and expose a downstream `{count}`
          binding. Source-binding `join @mode="group-by" @by="NAME"` nodes aggregate one collection artifact per
          distinct binding value and expose downstream `{key}`, `{count}`, and `{NAME}` bindings. Same-binding
          `join @mode="match-by" @by="NAME" @with:LABEL="NODE"` nodes aggregate one collection artifact per primary key,
          attach same-key named secondary artifacts, and expose downstream `{key}`, `{count}`, and `{NAME}` bindings.
          Missing secondary matches produce empty named secondary collections rather than fatal errors. Positional
          `join @mode="zip" @with:LABEL="NODE"` nodes aggregate one collection artifact per index across primary and
          named secondary artifact streams, expose downstream `{index}` and `{count}` bindings, and fail when any input
          stream has a different count. Resolver-backed globs require explicit list-capable resolvers, sort matches by
          resolved URI, and enforce a deterministic max-entry guard.
        - Report and source-map behavior for diagnostics that may originate from data, template compilation, template
          execution, or output export. Current CLI report destinations suppress diagnostic stderr and use
          `cem-ml.transform.report` as the default report basename. Single-transform reports include
          `reportAst.transform` metadata for the input, destination, output kind, source-map presence, output-span
          count, and sidecar refs when the response exposes a source map and has an output destination. Graph transform
          reports include `reportAst.transformGraph` export metadata for resolved export IDs, destinations, content
          identities, output kinds, source-map presence, output-span counts, sidecar refs, and collection-item
          provenance summaries when artifacts expose source-map fields and have export destinations. For concrete
          destinations, the CLI writes `{destination}.map` with the source-map JSON payload through the output resolver;
          graph export sidecars also carry export ID, input artifact ID, and destination metadata. Collection export
          sidecars retain per-item source maps and output spans instead of flattening multiple item stacks into one
          source-map stack. Collection primary JSON exports are a stable public projection with collection metadata and
          item payloads while omitting per-item `sourceMap` and `outputSpans` from the primary output.
        - Graph validation for duplicate IDs, unresolved refs, cycles, unsupported joins, unsupported cardinality
          changes, unknown output bindings, and duplicate resolved output destinations before writes.
      Execution is available through the programmatic engine API, the CLI one-liner, and CLI CEM-ML graph config dispatch
      when a host registers an executable adapter. `transform_graph` executes loaded in-memory graph requests and returns
      export artifacts; the CLI host writes configured graph exports through the resolver layer.
      First XSLT parity follow-up: `packages/cem_ml/tests/xslt_adapter_output_parity.rs` proves XSLT 1.0
      compatibility lowering renders the same light-DOM output as equivalent CEM sources for login/profile/asset-shaped
      cases. Executable parity now runs through direct `transform` and CEM-ML graph config, with CLI integration
      coverage in `packages/cem_ml_cli/tests/xslt_parity_transform.rs`. XSLT parity now accepts direct
      `--template-entrypoint` / `--param` and graph `transform @entrypoint` / child `param` records by lowering named
      entrypoints to bounded `xsl:call-template` execution with string `xsl:with-param` values. Missing selected XSLT
      entrypoints report fatal `cem.transform_template.call_unknown` diagnostics; graph config failures stop before
      writing export files or source-map sidecars. Unsupported XSLT constructs/functions stay fatal in executable
      parity, and direct/graph config failures stop before writing output/export files or source-map sidecars. The crate
      dependency cycle is avoided by keeping the concrete CEM-QL adapter in `cem_ml_transform_cem_ql` rather than in
      `cem_ml`. Mixed-runtime transform graphs now use per-stage runtime policy, with CLI coverage for CEM-native and
      XSLT stages in the same graph. Quoted XPath string literals in lowered XSLT value/param expressions now render as
      text while scalar variables that represent rewritten CEM-QL expressions still splice as expressions.
8. `cem-ml trace <input>`
    - Supported structured formats: `json`, `xml`, `cem`.
    - Reference convenience formats: `text`, `html`.
    - Parser and validator trace records remain placeholder output shapes until parser implementation exists.
    - Scheduler, worker-pool, transform, plugin, and source-map traces remain deferred.
9. `cem-ml bench <input...>`
    - Supported formats: `text`, `json`.
    - Supports `--iterations`, `--budget-ms`, `--profile`, `--cold-cache`, `--report-json`, and `--report-md`.
    - Benchmarking uses the engine boundary; parser performance work is deferred.
10. `cem-ml fixture roundtrip [input...]`
    - Defaults to the canonical CEM-ML fixtures and HTML parity fixtures.
    - Supports `--to-format cem|html|dom-json|ast|events`.
    - Transform/render snapshots remain deferred.

## Phase 7 - File I/O And Reports

1. Resolve inputs relative to cwd unless absolute.
2. `--base-uri` now applies to relative emitted diagnostic/report URIs.
3. Write parent directories recursively for `--out`, `--report-json`, and `--report-md`.
4. If a report destination has the expected file extension, write exactly that file.
5. If a report destination is a directory, write the default report filename inside it.
6. Treat validation-style commands (`validate`, `check`, `fixture validate`) as report-primary operations:
    - selected report output goes to `stdout` by default
    - explicit report targets write files instead
    - `stderr` is reserved for usage errors, I/O failures, unexpected internal failures, and non-report operational
      messages
7. Treat parse/convert/load/save-style commands as content-primary operations:
    - converted content or selected layer projection goes to `--out` when provided
    - otherwise it goes to `stdout`
    - reports are side outputs and are written only when report targets are requested
8. Additional layer outputs such as `events`, `tokens`, `input-dom`, `cem-ast`, or `report-ast` require explicit side
   output targets unless the CLI later defines a multiplexed container format.
9. Return exit code `6` for read or write failures.
10. Keep stdout empty when `--out` is used for the primary output.
11. Keep success text suppressed by `--quiet`, but still surface errors.
12. Generate report files by rendering the canonical report AST. JSON, XML, and CEM renderers are structured
   projections; text, Markdown, and HTML are reference convenience projections.

## Phase 8 - Tests

Concern: CLI feature coverage needs an explicit test matrix. The CLI plan owns this
coverage; the parser stack design only owns the layer outputs that feed CLI projections.

1. Add `cem-ml` unit tests for:
    - diagnostic normalization
    - fail-level evaluation
    - hard-violation detection
    - deterministic report summaries
    - report AST event sequence and event-time source-map hierarchy
    - JSON field naming
    - CEM/XML/JSON report rendering
    - text/HTML convenience report rendering
    - Markdown report formatting
2. Add `cem-ml` command tests with a fake engine for:
    - parse output formats
    - validate CEM, XML, JSON, text, HTML, Markdown, and report outputs
    - check `--zero-hard-violations`
    - fixture default path selection
    - inspect output modes
    - convert format validation
    - trace JSON/text shape
    - benchmark budget exit behavior
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
4. Maintain a feature coverage matrix in the test module docs or test manifest with rows for:
    - command surface: `parse`, `validate`, `check`, `inspect`, `convert`, `transform`, `trace`, `bench`,
      `fixture validate`, `fixture roundtrip`, `help`, `version`, and reserved `schema` and `plugin` workflows
    - option groups: fail level, output format, report destinations, output file, schema/content-type/base URI,
      default namespace and named namespace bindings, quiet/verbose/no-color, zero hard violations, source-offset
      preservation, inspect views, benchmark controls, and fixture defaults
    - output shapes: diagnostics, report AST, CEM/XML/JSON report renderings, text/HTML convenience renderings, DOM
      JSON, AST, events, inspect views, trace output, benchmark output, and fixture roundtrip reports
    - exit behavior: success, parser/validation failure, usage errors, reserved subsystem errors, I/O errors, and
      unexpected internal failures
    - parser-blocked cases: rows that assert routing and shape with the fake engine now, plus a future real-engine gate
      for semantic fixture validation
5. Do not assert real parsing behavior in this phase.
6. Keep parser-backed fixture success tests blocked until the parser implementation plan is approved.

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
3. Do not claim fixture validation is complete until `cem-ml fixture validate` validates
   `examples/cem-ml/*.cem` and `examples/semantic/*.html` with zero hard violations.
4. Keep `cem_ml_cli` dependent on `cem_ml` through Cargo, not by duplicating code.

## Phase 10 - Completion Gates

1. Feature gate:
    - help/version work
    - CLI usage errors match exit-code policy
    - report and diagnostic models match the documented CLI feature shapes
    - parser-backed commands are routed through `CemMlEngine`
    - CLI feature coverage matrix is present and every non-parser-blocked row has test coverage
    - fake-engine feature tests pass
2. Decision gate:
    - Java XML stack and schema/parser ADR is accepted
    - parser and schema mirror recommendations are recorded
    - security defaults are documented
3. Parser-enabled gate, future plan:
    - canonical CEM-ML tokenizer/parser implements the Tier A surface in
      [`cem-ml-syntax.md`](./cem-ml-syntax.md)
    - XML/HTML parity profiles lower into the same event model
    - real engine fills the existing `CemMlEngine` boundary
    - no CLI command or output-shape redesign is needed
    - fixture validation and parity comparison can be enabled as Nx targets
4. Feature-complete gate, future plan:
    - `cem-ml` CLI implements the command, option, report, diagnostic, fixture, trace,
      and benchmark features documented here and summarized in
      [`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md).

## Phase 11 - Parser/Tokenizer Implementation

**Status:** Future parser-enabled phase. This is the execution plan for making the
Tier A canonical CEM-ML surface executable.

1. Implement `cem_ml::tokenizer::cem` for the canonical curly syntax:
    - `{name @attributes | content...}` node scopes
    - optional `|` / relaxed content-boundary rules
    - `$` expression nodes and rejection of bare `{...}` text interpolation
    - anonymous typed scopes
    - directives (`@doc`, `@ns`, `@default`, `@schema`)
    - comments and rich-content enclosures from `cem-ml-syntax.md`
2. Implement schema-scoping syntax from `cem-ml-syntax.md`:
    - `@schema src="..."` prelude shorthand
    - inline `{cem:schema @cem:name="..." | ...}` declarations
    - self-closing and wrapping `{cem:schema @src=...}` / `@select=...` switches
    - host-node `@cem:schema-src` / `@cem:schema-select` attributes
    - scope-chain `cem:name` shadowing behavior
3. Lower CEM-native tokens into the shared `NormalizedEvent` model without going through
   HTML or XML token shapes.
4. Preserve source-map spans for node starts/ends, attributes, content boundaries,
   comments, rich content, and `$` expression bodies.
5. Implement tokenizer dispatch from `--from-format cem|html|xml`, file extension, and
   explicit content type, with `.cem` selecting canonical CEM-ML by default.
6. Keep HTML and XML tokenizer profiles as secondary parity paths that lower into the
   same event model as CEM-native input.
7. Wire the real parser engine into `CemMlEngine` while preserving the existing CLI
   command/output shapes.
8. Add parser diagnostics for syntax errors, unbound prefixes, unterminated scopes,
   invalid relaxed-boundary use, and invalid text interpolation.

Exit criteria: `cem_ml:test` has tokenizer and event-normalizer coverage for canonical
CEM-ML, HTML parity still routes through the same engine boundary, and CLI fake-engine
tests do not need command-shape changes.

## Phase 12 - Fixture Parity Tests

1. Maintain a fixture manifest pairing each canonical `examples/cem-ml/*.cem` file with
   its `examples/semantic/*.html` HTML parity fixture.
2. Add tokenizer fixtures for:
    - nested CEM-ML nodes
    - relaxed and explicit content boundaries
    - `$` expression nodes
    - attribute `{...}` cem-ql spans
    - schema-scoping forms and `cem:name` shadowing
    - repeated namespace binding names, including default namespace rebinding across
      unprefixed HTML/SVG subtrees
    - comments and rich-content enclosures
3. Add event-normalizer tests proving paired CEM-ML and HTML fixtures lower to the same
   schema event stream after content-type-specific trivia differences are accounted for.
4. Add validation fixtures proving the paired CEM-ML and HTML inputs produce the same
   hard-violation result and compatible diagnostics.
5. Add transform/roundtrip fixtures proving canonical CEM-ML snapshots are stable and
   rendered light-DOM custom-element output is unchanged by the source syntax.
6. Define exact lossless conversion rules for CEM-ML ↔ XML/HTML before enabling
   cross-surface conversion:
    - namespace bindings and default namespace changes
    - comments, whitespace, doctypes, processing instructions, CDATA/raw text, and
      content-type-specific trivia
    - anonymous typed scopes and schema/content-type switches
    - rich-content enclosures and raw/native content blocks
    - `$` expression nodes and attribute-value cem-ql spans
    - source-map frame preservation across both directions
7. Add XML convention parity fixtures when XML forms become executable; they must join
   the same manifest instead of creating a separate test path.
8. Enable Nx targets after the real engine exists:
    - `yarn nx run cem_ml_cli:validate-fixtures`
    - `yarn nx run cem_ml_cli:e2e`
    - `yarn nx run cem_ml_cli:bench`

## Phase 13 - Semantic Validation Rule Catalog

**Status:** Future validation phase. This phase turns AC-V-6 / AC-X-3 into concrete
schema-owned rule tables without making semantic validation an HTML/SVG-only subsystem.

1. Define the semantic-rule catalog shape in the compiled schema:
    - rule id
    - owning schema/content type
    - trigger layer
    - required AST/reference/source-map inputs
    - diagnostic code/severity defaults
    - policy override hooks
2. Build the first Tier A catalog for CEM UI projections over HTML/SVG/ARIA:
    - accessible-name requirements for rendered interactive and labeled nodes
    - ARIA role/attribute compatibility and reference integrity
    - `id` / `for` / `aria-*` reference-slot resolution
    - SVG-in-HTML accessibility boundaries such as `aria-hidden`, title/description, and focusability
3. Add generic CEM rules that are not HTML/SVG-specific:
    - invalid component state combinations
    - required/forbidden state transitions
    - template, slot, and schema-owned reference integrity
    - schema-owned open-content and unknown-name policy checks
4. Add unsafe-content rule tables for content-policy concerns:
    - inline script and event-handler policy
    - `javascript:` and unsafe URL-bearing attributes
    - `srcdoc`, imports, external entities/DTDs, and other policy-gated resource hooks
5. Keep later content types extensible through the same rule-registry model; CSS, JS,
   XML, JSON, plugin-loaded content, and future runtime content add rules instead of
   forking validation.
6. Add fixture expectations for canonical CEM-ML and HTML parity fixtures, including
   matching diagnostics where source syntax differs but semantic identity is the same.

## Phase 14 - Authoring Tooling

**Status:** Future tooling phase. This phase starts after the Phase 11 tokenizer and
Phase 12 conversion/parity rules are stable enough that tools do not encode a competing
grammar. Semantic diagnostics from Phase 13 feed editor/linter output when available.

1. Publish a machine-readable CEM-ML lexical grammar for editor integration and test it
   against the tokenizer fixtures from Phase 12.
2. Add syntax-highlighting support for canonical CEM-ML, including:
    - node starts/ends
    - attributes and namespaces
    - content markers
    - `$` expression scopes
    - rich-content enclosures
    - comments and diagnostics
3. Add a tree-sitter grammar or equivalent incremental parse grammar for editor use.
   It must round-trip with the canonical tokenizer on the shared fixture corpus.
4. Add formatter rules, including a Prettier-like profile, for:
    - stable indentation and line breaks
    - canonical `|` insertion policy
    - attribute ordering where schema permits it
    - quote and rich-content enclosure normalization
    - preservation of comments, whitespace-sensitive content, and source-map anchors
5. Add lint rules for unbound prefixes, invalid relaxed-boundary use, suspicious
   content-type switches, noncanonical but accepted delimiter choices, and forbidden
   bare `{...}` text interpolation.
6. Surface parser/schema diagnostics in editor-friendly shapes with byte offsets,
   line/column projections, quick-fix metadata where safe, and links back to source-map
   frames.
7. Add CLI entry points or subcommands only after the library contracts exist; the CLI
   remains a consumer of the tooling APIs, not the owner of the grammar.

## Deferred Work

The following remain deferred beyond the parser/tokenizer and fixture-parity phases
above:

- parser profiles beyond canonical CEM-ML, HTML parity, and XML parity
- full incremental/editor reparsing beyond the tooling grammar in Phase 14
- multithreading, worker pools, scheduler traces, and bounded queues
- schema emit/sample/replace implementation
- schema semver resolution behavior beyond accepting and recording `--schema`
- transform implementation
- plugin implementation
- source-map sidecar/export formats beyond the parser span preservation required in Phase 11
- WASM packaging beyond keeping the `cem-ml` crate boundary compatible
