# `cem-ml-cli` Implementation Plan

**Status:** Phase 1 parser/schema assessment complete. No Rust implementation is included in this document.

Phase 1 artifact: [`docs/cem-ml-parser-schema-adr.md`](./cem-ml-parser-schema-adr.md).

This plan defines the planned `cem-ml` CLI feature set for the Rust platform. Existing
CLI notes in [`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md) are idea inputs, not
decision criteria or compatibility requirements.

- App crate: `packages/cem_ml_cli`, Cargo package `cem-ml-cli`, binary `cem-ml`.
- Library crate: `packages/cem_ml`, Cargo package `cem-ml`, Rust crate `cem_ml`.

The goal is to provide useful parser/runtime CLI capabilities: command workflows, option
semantics, report fields, diagnostic fields, fail-level behavior, and exit codes.

## Explicit Scope

- Do not implement or design the streaming parser in this plan.
- Do not implement or design multithreading, worker pools, scheduler traces, or render-while-parsing.
- Do not implement parser internals yet, including canonical CEM-ML curly syntax,
  XML/HTML parity profiles, AST construction, or event production.
- Do front-load a separate Java XML stack, parser pattern, and schema pattern assessment before implementation.
- Keep `cem-ml-cli` thin. Shared behavior belongs in `cem-ml`.

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
    - bench report
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
    - `version`
    - `help`
2. Preserve common options:
    - `--fail-level parse|validate|strict`
    - `--format text|html|json|xml|cem|markdown|dom-json|ast|events|tree`
    - `--from-format cem|html|xml`
    - `--to-format cem|dom-json|ast|events`
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
    - Writes to stdout or `--out`.
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
    - Supported input formats: `cem`, `html`, `xml`.
    - Supported output formats: `dom-json`, `ast`, `events`.
    - Schema-version conversion, rendered HTML/XML output, comment-preservation behavior, and source maps remain deferred
      to the parser implementation.
7. `cem-ml trace <input>`
    - Supported structured formats: `json`, `xml`, `cem`.
    - Reference convenience formats: `text`, `html`.
    - Parser and validator trace records remain placeholder output shapes until parser implementation exists.
    - Scheduler, worker-pool, transform, plugin, and source-map traces remain deferred.
8. `cem-ml bench <input...>`
    - Supported formats: `text`, `json`.
    - Supports `--iterations`, `--budget-ms`, `--profile`, `--cold-cache`, and `--report-json`.
    - Benchmarking uses the engine boundary; parser performance work is deferred.
9. `cem-ml fixture roundtrip [input...]`
    - Defaults to the canonical CEM-ML fixtures and HTML parity fixtures.
    - Supports `--to-format cem|dom-json|ast|events`.
    - Transform/render snapshots remain deferred.

## Phase 7 - File I/O And Reports

1. Resolve inputs relative to cwd unless absolute.
2. Apply `--base-uri` to emitted diagnostic/report URIs using the documented path normalization policy.
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
4. Maintain a feature coverage matrix in the test module docs or test manifest with rows for:
    - command surface: `parse`, `validate`, `check`, `inspect`, `convert`, `trace`, `bench`, `fixture validate`,
      `fixture roundtrip`, `help`, `version`, and reserved `transform`, `schema`, and `plugin` workflows
    - option groups: fail level, output format, report destinations, output file, schema/content-type/base URI,
      quiet/verbose/no-color, zero hard violations, source-offset preservation, inspect views, benchmark controls, and
      fixture defaults
    - output shapes: diagnostics, report AST, CEM/XML/JSON report renderings, text/HTML convenience renderings, DOM
      JSON, AST, events, inspect views, trace output, bench output, and fixture roundtrip reports
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
