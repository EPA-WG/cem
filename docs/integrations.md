# CEM-ML IDE and SDLC Integration Options

Status: research and design proposal, 2026-08-09. This document is not yet an
active implementation checklist; its ordered feature list is intended to be
promoted into the roadmap after the contracts are accepted.

## Outcome

CEM-ML should have one protocol-neutral Rust tooling service and several thin
hosts:

- `cem-ml lsp --stdio` for validation and code insight in VS Code, VS Code
  derivatives, and IntelliJ-based IDEs;
- `cem-ml dap --stdio` for query and transformation debugging where the IDE can
  host the Debug Adapter Protocol (DAP);
- small VS Code and IntelliJ plugins that start those servers and add the UI
  that LSP and DAP do not cover;
- a Chromium DevTools extension with a CEM-QL workbench and an Elements sidebar
  for live `cem-element` / `custom-element` inspection;
- one-shot CLI output for shell tasks, pre-commit hooks, and CI, with canonical
  JSON plus SARIF and CI-vendor projections.

LSP, DAP, CI formats, and browser panels should be projections of the same
document, diagnostic, query, transform, source-map, and execution-session
models. They must not each grow a separate CEM parser or policy engine.

The common protocols have different jobs. The
[Language Server Protocol (LSP)](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
standardizes JSON-RPC communication for editor language features. The
[Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
standardizes JSON communication between a debugger UI and a debugger adapter.
LSP is not a debugger protocol, and DAP is not a validation or completion
protocol.

## Goals

- Validate saved and unsaved sources against their effective schema and point
  at exact warning/error ranges.
- Preserve the full origin-first source-map trace when a finding or generated
  value crosses a parser, schema, query, or transform boundary.
- Provide context-sensitive hints at the cursor for CEM-ML, CEM-native schemas,
  CEMT, CEM-QL, and supported embedded content types.
- Debug a query or transformation with breakpoints, stepping, call-stack-like
  execution frames, scopes, variables, watches, and expression evaluation.
- Preview transformed output and navigate in both directions between source and
  generated artifacts.
- Inspect live `cem-element` and compatibility `custom-element` instances in
  Chromium, including their declaration, data snapshot, diagnostics, render
  plan, and source provenance.
- Use the same rule identities, severity policy, source ranges, and exit policy
  in editors, local automation, and CI/CD.
- Make VS Code and IntelliJ-based IDEs the first desktop integrations, with
  Chromium DevTools as the first live browser integration.

## Existing Foundation

This proposal can build on substantial existing behavior rather than wrapping
the CLI's human-readable output.

| Existing capability                                                                                                                              | Integration value                                                                                   | Remaining gap                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `Diagnostic` has URI, line, column, byte offset, stable code, severity, details, and `SourceMapStack`.                                           | Already enough to identify and classify most findings.                                              | A canonical primary _range_, related locations, document version, and source-id-to-URI catalog are needed.   |
| Diagnostic projection already computes UTF-16 offsets/columns from bytes.                                                                        | Matches the default coordinate system expected by LSP-style clients.                                | The server must still negotiate position encoding and consistently convert both start and end positions.     |
| Source-map frames are origin-first and retain byte ranges and transform kinds.                                                                   | Can power related locations, “show source trace,” generated-source navigation, and debugger frames. | Source identities and cross-file ranges need a stable external projection.                                   |
| JSON reports contain diagnostics, parser stages, transform reports, source-map references, and deterministic scheduler traces.                   | Strong basis for CI artifacts, preview metadata, and trace views.                                   | The report schema is batch-oriented and should not become the interactive request protocol.                  |
| `--observe-events PATH` emits parse/validate/transform JSONL events.                                                                             | Useful for logs, progress, trace viewers, and early experiments.                                    | It has no request/response, document revision, pause, or back-pressure contract.                             |
| `cem-ml query` supports CSS selector, CEM-QL, and XPath through explicit content-type/schema identity.                                           | One query boundary can back editor commands and debugger evaluation.                                | Interactive evaluation needs paused-frame context and lazy value handles.                                    |
| CEM-QL has parse, resolve, type-check, typed IR, evaluator, source maps, and policy-bound scopes.                                                | Supplies most semantic data needed by completions, hover, navigation, and debug evaluation.         | Public tooling queries and pausable evaluator hooks are not yet defined.                                     |
| CEM-QL and parts of CEM-ML expose WASM entry points and observability callbacks.                                                                 | Provides a practical in-browser path for a CEM-QL workbench.                                        | WASM and native CLI lifecycle/query/transform capabilities are not yet at full parity.                       |
| `cem-element` exposes `diagnosticsFor(...)` and `snapshotInstance(...)`, and rendered nodes carry stable render identities in development flows. | Provides useful live-inspection inputs for Chromium DevTools.                                       | A versioned, capability-limited, redacting DevTools bridge is needed.                                        |
| `tree-sitter-cem` and legacy `custom-element` VS Code custom-data / IntelliJ web-types assets exist.                                             | Useful syntax and compatibility inputs.                                                             | They should be generated from current schema packages and must not become a second semantic source of truth. |

The old `custom-element` Chrome plugin described in
[`packages/custom-element/README.md`](../packages/custom-element/README.md#chrome-devtools-plugin)
is a functional reference for selected-node, parent declaration, `datadom`, XML,
and stylesheet inspection. It is not a decision to retain the retired browser
XSLT execution model.

## Integration Architecture

```text
                         schema packages and workspace config
                                      |
                                      v
  source snapshots ---> protocol-neutral CEM tooling service
                       - document/version store
                       - schema and module resolution
                       - parse/validate/index/cache
                       - query/transform execution
                       - source and generated-artifact maps
                       - debug session controller
                           |          |          |
                  +--------+-----+  +-+------+  +----------------+
                  | LSP server   |  | DAP    |  | batch/report   |
                  | diagnostics  |  | server |  | projections    |
                  | code insight |  | debug  |  | JSON/SARIF/... |
                  +------+-------+  +---+----+  +--------+-------+
                         |              |                |
                  VS Code/IntelliJ  VS Code/IntelliJ   CLI and CI

  Chromium DevTools panel ---> browser tooling facade ---> WASM tooling core
                                   |                  or native-message bridge
                                   +--> inspected-page runtime bridge
```

The protocol-neutral service is important even if the first implementation is
inside the `cem-ml` binary. It prevents LSP types, DAP object handles, Chromium
extension messages, or a CI vendor's report fields from leaking into parser,
schema, and evaluator contracts.

### Canonical tooling requests

The shared service should eventually expose typed Rust requests equivalent to:

- open, replace, incrementally change, close, and validate a versioned document;
- resolve the effective content type, schema, namespace, module map, resolver
  policy, and budgets for a document position;
- complete, hover, get signature help, locate definitions/references, list
  symbols, format, rename, and compute code actions;
- run or preview a query/transformation against an explicit input snapshot;
- create, control, inspect, and dispose a debug session;
- export diagnostics and execution results through selected projections.

The service should accept bytes and document snapshots directly. An IDE must
not need to save an incomplete buffer to disk or create a temporary file before
the engine can analyze it.

### Diagnostic and location contract

Before an editor protocol is implemented, the canonical diagnostic should be
strengthened with:

- `documentUri` and optional `documentVersion`;
- `primaryRange` containing byte start/length and projected start/end
  coordinates;
- a declared coordinate encoding, with UTF-16 projection always available;
- `relatedLocations[]` for schema declarations, imports, references, and other
  relevant source-map frames;
- `sourceTrace[]` retaining the complete origin-first transform chain;
- stable `code`, severity, message, schema/policy identity, and optional fix
  identifiers;
- a source catalog mapping every `SourceId` to URI, content identity, content
  hash, and availability state.

The current top-level `line`, `column`, and `byteOffset` fields can remain as a
compatibility projection. They are not sufficient for editor underlines because
they do not provide a uniform end position.

LSP positions are zero-based and are negotiated as a character encoding; the
server must not expose raw UTF-8 byte columns as LSP character positions. The
existing line index's UTF-16 projections are therefore a valuable foundation,
not redundant metadata.

### Configuration and identity precedence

Editor runs should normalize configuration through the existing CEM-ML
`RunConfig` path. Tool-specific settings should name or extend a run config,
not reimplement schema selection. Effective document identity should be
resolved in this order:

1. explicit request or debug/run configuration identity;
2. explicit document/template/query declarations;
3. matching workspace run-config input or transformation association;
4. schema-package manifest and registered extension/content-type association;
5. an unambiguous built-in extension default;
6. otherwise a diagnostic asking for identity, never query-text heuristics.

The normalized result, including provenance, should be visible in hover,
inspection, traces, and debug scopes. This makes “why is this schema/query
language active?” answerable.

### Process and stream rules

- LSP and DAP use long-lived processes and framed protocol messages on standard
  input/output. Standard output is reserved for the protocol; logs go to
  standard error or a requested log file.
- Batch CLI commands may write one canonical JSON value or JSONL stream to
  standard output, but their machine mode must never contain progress text or
  ANSI escapes.
- Every asynchronous answer carries or is checked against the source document
  version. Results from a superseded edit are discarded.
- Expensive work supports cancellation, debounce, budgets, and bounded queues.
- URI/path normalization is shared by CLI, LSP, DAP, and CI. Clients do not
  repair paths independently.

## CLI-to-Editor Communication Options

| Option                                             | Advantages                                                                                                                                                                                                                     | Disadvantages                                                                                                                                            | Recommended use                                                                                                         |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Parse human terminal output with a problem matcher | Almost no implementation work; usable from an editor task. VS Code tasks can turn matched output into Problems.                                                                                                                | Fragile text parsing, usually one range only, no unsaved buffers, no source-map chain, process startup per run, and easy stdout/log corruption.          | Bootstrap only; do not make it a contract.                                                                              |
| One-shot CLI with canonical JSON                   | Simple, scriptable, language-neutral, testable, and useful in CI as well as editors.                                                                                                                                           | Still has startup cost; cannot naturally track buffer revisions, requests, cancellation, or interactive hints.                                           | Supported fallback for save-time validation and external-tool integrations. Add stdin bytes plus an explicit stdin URI. |
| Background `--watch` plus JSONL events             | Reuses the current observability direction; fast repeated validation; easy trace consumption.                                                                                                                                  | Notifications alone cannot serve completion/hover requests; reconnect, back-pressure, stale revisions, and request correlation become a custom protocol. | Watch tasks, trace viewers, and development diagnostics, not the main IDE API.                                          |
| Custom JSON-RPC daemon                             | Can expose every CEM-specific operation and share caches across tools.                                                                                                                                                         | Recreates capability negotiation, cancellation, document synchronization, and client libraries already provided by LSP/DAP; every IDE needs custom glue. | Internal protocol only if a shared daemon later coordinates LSP, DAP, and browser clients.                              |
| LSP over stdio                                     | Portable validation/code insight, incremental document synchronization, cancellation, capability negotiation, and broad editor reuse. VS Code explicitly recommends the language-server pattern for reusable language tooling. | Does not model debugger control, arbitrary rich UI, or all IDE-native refactorings. Client support differs by feature and version.                       | Primary editing protocol.                                                                                               |
| DAP over stdio                                     | Portable debugger UI for breakpoints, stepping, stack frames, scopes, variables, watches, and evaluation.                                                                                                                      | Requires a genuinely pausable execution engine. It does not provide linting, completion, or formatting.                                                  | Primary query/transform debugger protocol.                                                                              |
| In-process Rust library                            | Lowest latency, typed API, direct cache and AST access.                                                                                                                                                                        | Couples the host to Rust ABI/build/distribution choices; not practical for TypeScript or JVM plugins without another boundary.                           | Core implementation and native tests.                                                                                   |
| WASM library                                       | Runs in browser panels, VS Code web extensions, and isolated previews without installing a native executable.                                                                                                                  | Larger download, browser resource constraints, host resolver limitations, and current native/WASM feature gaps.                                          | Chromium workbench and optional web-editor mode.                                                                        |
| IDE-proprietary API                                | Best access to native UI, tasks, testing, inspections, refactoring, and debugger presentation.                                                                                                                                 | Duplicated VS Code/JetBrains implementation and faster API churn.                                                                                        | Thin adapters around the shared LSP/DAP/tooling service.                                                                |

VS Code's task system and
[problem matchers](https://code.visualstudio.com/docs/debugtest/tasks#_processing-task-output-with-problem-matchers)
are useful for a zero-install experiment, but they should be visibly described
as a lower-fidelity fallback.

## Validation and Linting

### LSP mapping

The LSP server should validate on open and after debounced changes, validate
immediately on save, and offer an explicit workspace-validation command. The
initial implementation can publish diagnostics; a later implementation can add
pull/workspace diagnostics where clients support them.

| CEM diagnostic data                                  | LSP projection                                                                                              |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Primary URI and UTF-16 range                         | `Diagnostic.range` in that document.                                                                        |
| `error` / `fatal`                                    | Error severity; retain fatal classification in `data`.                                                      |
| `warning`                                            | Warning severity.                                                                                           |
| `info`                                               | Information severity.                                                                                       |
| Stable diagnostic code                               | `code`; use `codeDescription` when stable documentation URLs exist.                                         |
| Other source-map frames                              | `relatedInformation` when the target URI/range is known.                                                    |
| Complete source-map stack, schema identity, fix data | Opaque `data` plus a `cem.showSourceTrace` command.                                                         |
| Deterministic fix                                    | LSP code action/workspace edit, only after the engine validates the edit against the same document version. |

Only the CEM language server should publish CEM semantic/schema diagnostics.
An IDE adapter must not also run a save-time CLI task by default, or users will
see duplicate findings.

### One-shot fallback

A reliable one-shot editor contract needs additions such as:

```text
cem-ml validate --stdin --stdin-uri file:///workspace/data.cem \
  --content-type application/cem+xml --schema ... \
  --format json --no-color
```

The exact flags remain a CLI design task. The behavioral requirements are more
important: read bytes from stdin, preserve a real logical URI, emit only one
versioned machine object, provide complete ranges, use documented exit codes,
and write logs only to stderr. Writing an unsaved buffer to a temporary path
loses base-URI and workspace identity and should not be the normal design.

## Cursor Hints and General Code Insight

VS Code distinguishes declarative syntax support from programmatic language
features; it uses TextMate grammar for the former and supports LSP-backed hover,
completion, definition, diagnostics, formatting, and refactoring for the latter.
See the official
[Language Extensions overview](https://code.visualstudio.com/api/language-extensions/overview)
and
[Language Server guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide).

The useful CEM feature set is broader than completion alone:

| Feature               | CEM behavior                                                                                                                                                                                                         | Likely protocol                                                          |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Completion            | Schema-valid elements, attributes, annotations, enum/state/role values, namespaces, imports, templates, params, variables, functions, types, fields, and query steps valid at the cursor. Rank required items first. | LSP completion.                                                          |
| Hover                 | Effective schema/content identity, declaration docs, type/cardinality, default/deprecation, current query language, namespace resolution, and source-map provenance.                                                 | LSP hover.                                                               |
| Signature help        | CEM-QL functions, templates, formatter/colorizer functions, and parameter lists.                                                                                                                                     | LSP signature help.                                                      |
| Inlay hints           | Inferred CEM-QL types/cardinality, implicit defaults, effective schema version, selected transform rule, and optional cost estimates. Keep these configurable.                                                       | LSP inlay hints.                                                         |
| Semantic highlighting | Schema names, query variables/functions/types, declarations/references, deprecated names, generated/tainted regions, and embedded-language boundaries.                                                               | LSP semantic tokens plus a basic host grammar.                           |
| Navigation            | Definition/references for schemas, modules, imports, templates, params, variables, IDs/ARIA/`for` references, state slots, package assets, and output source-map origins.                                            | LSP definition/references/document links; proprietary source-trace view. |
| Symbols and structure | Document/workspace symbols, folding, selection ranges, call/type hierarchy where meaningful.                                                                                                                         | LSP.                                                                     |
| Code actions          | Insert required members, qualify a namespace, choose schema identity, create a missing declaration, update deprecated syntax, apply safe conversions, and suppress/configure a rule.                                 | LSP code actions.                                                        |
| Rename                | Scope-aware rename of templates, params, variables, state slots, and resolvable references.                                                                                                                          | LSP rename after reference indexing is reliable.                         |
| Formatting            | Schema-owned formatter/profile, whole document and safe range formatting.                                                                                                                                            | LSP formatting; on-type formatting later.                                |
| Preview/run           | Run the query at cursor, preview a transform, reveal generated artifact, and compare source/output.                                                                                                                  | IDE command plus tooling request; CodeLens is optional sugar.            |

Completion and hover must work on error-recovered trees while a user is typing.
Batch-only “valid AST or no result” behavior will make the language server feel
broken even if final-file validation is correct.

## Query and Transformation Debugging

### Why DAP

DAP is designed around the UI requested here. A development tool asks for
threads, then stack frames, then scopes and variables, and can issue evaluate
requests. Its high-level, language-neutral model is a good fit for a CEM
execution engine once that engine can pause at deterministic safe points. The
[DAP overview](https://github.com/microsoft/debug-adapter-protocol/blob/main/overview.md)
describes this request flow, while VS Code's
[Debugger Extension guide](https://code.visualstudio.com/api/extension-guides/debugger-extension)
shows the resulting breakpoint, call-stack, variable, watch, inline-value, and
debug-console UI.

The existing scheduler trace is observational: it records what ran. It cannot
by itself stop execution, preserve a live frame, step, or lazily expand values.
A debugger therefore needs an execution controller in the query/transform
runtime, not only a DAP serializer over completed reports.

### Proposed execution model

Debug safe points should include:

- transform graph import/export and stage entry/exit;
- template/rule match, select, and body entry/exit;
- named-template/function entry and return;
- CEM-QL expression and pipeline-step entry/exit;
- loop/sequence item transitions;
- schema validation behavior entry when debugging validators;
- resource request, resolution, wait, completion, denial, and failure;
- diagnostic emission and output artifact creation.

The runtime should use stable execution ids independent of source-map frame ids
or internal AST allocation ids.

### DAP projection

| DAP concept          | CEM meaning                                                                                                                                                                                                                                    |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Debug session        | One normalized query or transform run, including input versions, policy, budgets, and resolver capabilities.                                                                                                                                   |
| Thread               | Initially one deterministic execution lane per input/transform branch. If parallel scheduling is exposed, each independently pausable lane may become a thread. Do not equate every historical scheduler event with a thread.                  |
| Stack frame          | Nested transform graph stage, template/function invocation, rule evaluation, query expression/pipeline step, or resource boundary. Frame source points to the authored query/template/schema; related data-source locations remain accessible. |
| Scopes               | Locals/parameters, current item and input, transform context, state/slices, namespaces and schema, resources/policy/budgets, and output preview.                                                                                               |
| Variable             | A typed CEM-QL item or engine value. Large sequences, trees, maps, and source-map stacks use lazy child handles and paging.                                                                                                                    |
| Evaluate             | Evaluate an expression in the selected paused frame for hover, watch, or debug console without mutating runtime state by default.                                                                                                              |
| Source breakpoint    | Authored `.cemt`, `.cem-ql`, schema behavior, or embedded expression location mapped to an executable safe point.                                                                                                                              |
| Function breakpoint  | Named template, function, rule, stage, or diagnostic code.                                                                                                                                                                                     |
| Exception breakpoint | Fatal/error diagnostics, resolver denial, budget exhaustion, or selected diagnostic-code patterns.                                                                                                                                             |
| Loaded source        | Imported query/template/schema sources and optional virtual generated IR/plan views.                                                                                                                                                           |

The first debugger should be read-only: no set-variable, hot reload, or
side-effecting evaluate. Mutation complicates determinism, source maps, policy,
and replay and can be added only after explicit semantics exist.

### Query-language selection

Language selection must remain identity-based, consistent with the existing
unified CLI:

1. An explicit debug setting (`queryContentType` and optional `querySchema`)
   wins if it is compatible with the selected transform/query host.
2. A file-backed query uses its declared resource identity.
3. An expression embedded in a transformation inherits the transformation
   module's associated `query_language`.
4. CEM-native transformation templates currently associate CEM-QL as their
   default query language.
5. A standalone query with no association still requires explicit identity;
   neither the CLI nor debugger guesses CSS selector, CEM-QL, or XPath from
   source text.

DAP's standard evaluate request does not carry a query-language field. The
debug session therefore stamps the effective evaluation language on every
frame. A CEM-specific extension request may explicitly switch languages only
when the frame's input model has a registered adapter for that language.

### Later debugger capabilities

- Conditional breakpoints and logpoints evaluated in the frame's language.
- Data breakpoints for state slots, slices, resource states, or selected output
  nodes.
- “Break on diagnostic code” and “run to transform stage.”
- Bidirectional source/output selection using generated source maps.
- Deterministic replay or reverse stepping. This is plausible because traces
  are deterministic, but it additionally requires snapshotting inputs,
  resolver responses, state, and evaluator checkpoints; it is not an MVP.
- Performance view using stage cost, queue wait, resource time, and budget
  consumption without conflating performance events with debugger frames.

## VS Code and Compatible Derivatives

The VS Code extension should be a thin distribution and UI layer:

- declare CEM file types, brackets/comments, a baseline TextMate grammar, and
  snippets so files remain usable while the server starts;
- locate/download or use a configured `cem-ml` binary, then start
  `cem-ml lsp --stdio`;
- contribute the `cem-ml` debug type and start `cem-ml dap --stdio`;
- contribute run/validate/preview tasks and debug configuration schema;
- provide native tree views for schema/package/dependency and source-trace
  trees;
- use a webview only for a transform preview, visual diff, or other UI that has
  no native representation.

VS Code's extension API explicitly supports both language-server clients and
debug adapters. Its
[debug architecture](https://code.visualstudio.com/api/extension-guides/debugger-extension#debugging-architecture-of-vs-code)
allows a debugger extension to be little more than a package and configuration
layer around a standalone adapter.

This design is likely to work in derivatives that retain the relevant VS Code
extension and protocol APIs, but support should be tested rather than promised
for every clone. Publish a compatibility matrix for VS Code, VSCodium, Cursor,
Windsurf, and any requested derivative, including remote/container and web
extension modes. Desktop/native and browser/WASM distributions may need
different extension entry points.

Advantages:

- best initial LSP and DAP host;
- one extension can package language, tasks, debug, previews, and settings;
- broad reuse by compatible derivatives.

Disadvantages:

- TypeScript client/distribution work remains even with Rust servers;
- remote, untrusted workspace, and web-extension modes impose different
  process/filesystem capabilities;
- rich webviews can easily duplicate native editor/debug UI and should stay
  limited.

## IntelliJ-Based IDEs

There are two viable levels of IntelliJ integration.

### LSP-first plugin

JetBrains exposes a public IntelliJ Platform LSP API for listed IntelliJ-based
IDEs, and plugins can customize and extend the LSP client. However, JetBrains
also states that canonical custom-language support has broader IDE integration
than LSP. The current
[IntelliJ LSP documentation](https://plugins.jetbrains.com/docs/intellij/language-server-protocol.html)
also notes that this integration is generally tied to commercial IDEs and is
not available in IntelliJ IDEA open-source builds or Android Studio (with a
specific PyCharm exception).

The first plugin should:

- register file types and start `cem-ml lsp --stdio` through the platform LSP
  API where available;
- package generated JetBrains web-types for `cem-element` / `custom-element`
  HTML authoring;
- add CEM run configurations for validate, query, transform, and preview;
- bridge `cem-ml dap` into the IntelliJ debugger UI through the platform's
  debugger APIs;
- provide tool windows only for source traces, transform previews, and package
  views not represented by LSP/DAP.

Advantages:

- shares the language server with VS Code;
- substantially smaller than a complete PSI language plugin;
- IntelliJ-native run configurations persist working directory, arguments,
  environment, and other execution settings, as documented by the
  [Run Configuration API](https://plugins.jetbrains.com/docs/intellij/run-configurations.html).

Disadvantages:

- available LSP features and customization vary by IntelliJ release;
- commercial-module requirements exclude important hosts;
- DAP is not the IntelliJ plugin contract, so a plugin still has to adapt the
  CEM debug session to IntelliJ's debugger model. The public
  [`com.intellij.modules.xdebugger` module](https://plugins.jetbrains.com/docs/intellij/plugin-compatibility.html#modules-specific-to-functionality)
  covers debug sessions, frames, breakpoints, source positions, and related UI,
  but creates JetBrains-specific implementation work.

### Full native language plugin

A later native plugin can build PSI/stubs/indexes, inspections, intention
actions, refactorings, and debugger integrations directly. IntelliJ also
supports an External Annotator specifically for turning external-tool results
into editor highlighting, according to its
[syntax and error-highlighting documentation](https://plugins.jetbrains.com/docs/intellij/syntax-highlighting-and-error-highlighting.html#external-annotator).

Advantages:

- works around missing LSP features and can provide first-class IntelliJ
  inspections/refactorings;
- can potentially support Community/open-source IDE builds.

Disadvantages:

- largest duplicated implementation and maintenance burden;
- a second parser/semantic model is a serious consistency risk;
- PSI indexes must still defer CEM semantics to the shared tooling service.

Recommendation: ship the LSP-first commercial-IDE plugin, retain generated
web-types, and evaluate a smaller Community-compatible External Annotator or
native client only after demand is measured. Do not start with a complete
parallel PSI implementation.

## Chromium DevTools

Chromium DevTools is a first-class integration target for two related but
different jobs.

### CEM-QL web workbench

A dedicated CEM panel should provide:

- a CEM-QL editor with diagnostics, completion, hover, formatting, and explicit
  query identity;
- selectable input: inspected document, selected `$0` element, owning
  `cem-element` instance, sanitized runtime snapshot, pasted data, or a declared
  resource;
- query results as a typed expandable tree plus terminal/CEM/JSON projections;
- transform input/template/config selection and side-by-side output preview;
- source-map trace and click-to-source navigation;
- saved query history scoped to the inspected origin/workspace, without saving
  captured sensitive values by default;
- later, a custom debugger view backed by the same debug-session controller.

The panel can reuse the current CEM-QL WASM query/template exports for an early
prototype. It should disclose when an operation needs native-only resolver,
filesystem, plugin, or lifecycle capabilities rather than silently returning a
different result.

### `cem-element` and `custom-element` live inspection

An Elements sidebar should update when the DOM selection changes and show:

- selected rendered node and stable render-node identity;
- owning component instance and declarative component definition;
- effective schema, content type, namespace, version, and policy stamps;
- template artifact, matched rule/stage, params, state/slices, sanitized
  `datadom`, and resource lifecycle states;
- declaration, render, hydration, and runtime diagnostics;
- source-to-output provenance and links to the declaration/template/query;
- a command to make the selected instance the CEM-QL workbench context;
- an event/render timeline and output diff in later phases.

Chrome's
[`chrome.devtools.panels` API](https://developer.chrome.com/docs/extensions/reference/api/devtools/panels)
can create a full extension panel and an Elements sidebar, receive selection
changes, and open a resource at a line/column. The
[`inspectedWindow` API](https://developer.chrome.com/docs/extensions/reference/api/devtools/inspectedWindow)
can inspect page resources and evaluate in the inspected page, but its direct
evaluation capability is powerful and should be used narrowly.

The `cem-element` runtime should expose a versioned development-only bridge
rather than making the extension scrape private JavaScript fields. The bridge
should be read-only by default, capability negotiated, bounded, and able to
redact snapshot fields according to the existing privacy/export policy.
Compatibility `custom-element` support can use a small adapter to that bridge.

### Browser-to-engine transport options

| Transport                                      | Advantages                                                                                          | Disadvantages                                                                                                                                                                   | Recommendation                                                                                                                                                                                     |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CEM-ML/CEM-QL WASM inside the DevTools panel   | No native installation or local port; works offline; naturally sandboxed; easiest CEM-QL web UI.    | Download/memory cost; resolver and filesystem restrictions; current native/WASM parity gaps; separate cache from CLI.                                                           | Default workbench engine for safe local inputs and live snapshots.                                                                                                                                 |
| Chrome native messaging host wrapping `cem-ml` | Full native CLI/schema-package/resolver behavior; private stdio channel; no listening network port. | Requires OS-specific host installation and manifest registration; extension permission; message size/protocol limits; cannot be called directly from a content script.          | Optional “native parity” mode after the WASM panel. Chrome documents the extension-to-native [messaging contract](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging). |
| Authenticated loopback HTTP/WebSocket daemon   | Streams events well and can serve multiple browser/editor clients.                                  | Port discovery, authentication, origin/CSRF protections, lifecycle cleanup, firewall prompts, and accidental network exposure.                                                  | Development/remote scenario only unless a robust daemon product is justified.                                                                                                                      |
| Execute the engine in the inspected page       | Direct access to live objects.                                                                      | Pollutes/affects the application, inherits page CSP/runtime constraints, exposes privileged operations to untrusted page data, and makes results application-version-dependent. | Do not use for the tooling engine. Limit page code to the inspected runtime bridge.                                                                                                                |

### CDP and source-map limits

The Chrome DevTools Protocol (CDP) Debugger domain has JavaScript call frames,
scopes, stepping, and `evaluateOnCallFrame`, but those represent the inspected
page's JavaScript runtime. The
[CDP Debugger domain](https://chromedevtools.github.io/devtools-protocol/1-3/Debugger/)
should be used only when correlating a CEM operation with actual page script
execution. Pretending that CEM template/query frames are JavaScript frames would
create misleading semantics and couple CEM debugging to Chrome.

Similarly, Chrome's built-in authored/deployed source-map UI understands
generated JavaScript and CSS maps. Chrome documents that
[JavaScript and CSS source maps](https://developer.chrome.com/docs/devtools/settings/preferences#sources)
populate authored sources and source links. CEM should emit standard maps when
an output really is generated JS/CSS, but CEM-to-DOM and multi-stage CEM source
traces need the CEM panel/sidebar. A DOM tree is not a generated text file to
which Source Map v3 can be losslessly applied.

### Chromium security boundary

- Treat inspected-page values, DOM strings, messages, and resource URLs as
  untrusted input.
- Do not expose arbitrary filesystem or resolver access to the page.
- Require explicit user action for native mode and remote/network resolution.
- Redact form, storage, event, and resource payloads by default; show the active
  redaction policy.
- Keep page bridge commands allowlisted, versioned, bounded, and read-only in
  the first release.
- Never render captured values through unsafe HTML in the extension panel.
- Release remote object handles, debug variable handles, and large snapshots
  when the selection/session changes.

## CI/CD and Other SDLC Integrations

### Output formats

| Format/integration                            | Advantages                                                                                                                                                                            | Disadvantages                                                                                            | Priority                                              |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Existing human text/CEM/Markdown/HTML reports | Good logs and downloadable artifacts; CEM report remains canonical within this project.                                                                                               | Poor machine interoperability.                                                                           | Keep.                                                 |
| Canonical CEM JSON report                     | Lossless CEM details, source-map stacks, traces, identities, and summary counts.                                                                                                      | CEM-specific consumers only.                                                                             | Keep and version.                                     |
| SARIF 2.1.0                                   | Standard static-analysis interchange with rules, physical/logical locations, related locations, fixes, fingerprints, and code flows. GitHub code scanning accepts a supported subset. | Verbose; vendor subsets differ; not appropriate for test cases or full transform artifacts.              | First new CI projection.                              |
| GitHub workflow annotations                   | Immediate inline warning/error ranges with no security product setup.                                                                                                                 | GitHub-specific, stdout escaping, limited source-trace richness, and not a durable interchange artifact. | Small adapter after SARIF.                            |
| GitLab Code Quality JSON                      | Merge-request and diff presentation with a small required schema.                                                                                                                     | GitLab-specific and less expressive than CEM JSON/SARIF.                                                 | Small adapter when GitLab support is requested.       |
| JUnit XML                                     | Widely displayed by CI test UIs; useful for schema-package examples, fixture parity, and transformation tests.                                                                        | A validation finding is not naturally a test case; poor range/source-map representation.                 | Use only for manifest examples/tests.                 |
| Generated artifact/source-map bundle          | Lets reviewers inspect outputs, previews, provenance, and diffs.                                                                                                                      | Can be large or leak source/sensitive runtime data.                                                      | Opt-in CI artifact with retention/redaction controls. |

[SARIF 2.1.0 plus errata](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html)
is the recommended neutral static-analysis format. A CEM diagnostic maps its
stable code to `ruleId`, origin to the primary physical location, structural
schema path to a logical location, other source-map frames to related locations
or a code flow, safe fixes to `fixes`, and stable identity to partial
fingerprints. GitHub specifically relies on consistent rule ids, paths, and
fingerprints to avoid duplicate alerts; see its
[SARIF support](https://docs.github.com/en/code-security/code-scanning/integrating-with-code-scanning/sarif-support-for-code-scanning).

For lighter GitHub Actions jobs, workflow commands support file, line, column,
end-line, and end-column annotations. See the official
[workflow-command syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#setting-a-warning-message).
GitLab's
[Code Quality report](https://docs.gitlab.com/ci/testing/code_quality/#code-quality-report-format)
requires description, check name, fingerprint, repository-relative path, start
line, and severity. Both should be derived from canonical diagnostics, not
implemented as alternate validators.

### CI behaviors

- Validate all declared files and schema-package examples in deterministic,
  network-denied mode unless a job explicitly grants resolver capabilities.
- Fail according to configured severity/fail level; report generation itself
  must not change validation semantics.
- Publish canonical JSON even when a vendor format is also emitted so CEM-only
  details remain available for diagnosis.
- Fingerprint a finding from stable code, normalized artifact/logical location,
  relevant origin span content, schema identity, and policy identity. Do not
  fingerprint only the line number.
- Support baselines and “new findings only” review without hiding the full
  report artifact.
- Support changed-file/affected-project selection, but revalidate dependents
  when schemas, manifests, module maps, templates, or imported resources change.
- Cache only with content hashes plus schema, module, resolver-policy, tool
  version, and normalized run-plan stamps.
- Upload transform previews, diffs, traces, and source-map sidecars only under
  explicit retention and sensitivity policy.
- Emit a software-readable tool/version/config summary so a local IDE run and a
  CI result can be reproduced.

### Additional SDLC uses

- Pre-commit validation of staged content through stdin while retaining the
  repository URI.
- Pull-request annotations and preview artifacts for transformed CEM/HTML/CSS.
- Schema-package conformance tests surfaced in IDE test explorers and JUnit CI
  reports.
- Migration code actions and batch conversion reports for schema/version
  upgrades.
- Dependency/import graphs for impact analysis and Nx affected selection.
- Release gates for unresolved references, unpinned schema/module identity,
  forbidden resolver capability, output drift, and missing source maps.
- Performance budgets for parse/validate/query/transform stages, with trace
  artifacts on regression.
- Documentation playgrounds using the WASM facade for safe validation, queries,
  and previews.
- Runtime incident capture from `cem-element` using a redacted snapshot, active
  identities, diagnostics, and deterministic trace rather than raw page state.
- Optional editor/MCP or agent tools implemented over the same tooling service;
  they should not receive broader resolver or evaluation authority than the
  user-facing integrations.

## Ordered Feature List for a Future Roadmap

The order below is dependency-driven. Each item should be promoted into
acceptance criteria and `docs/todo.md` only when implementation is authorized.

1. **Tooling data contract.** Version a protocol-neutral document snapshot,
   source catalog, full primary/related ranges, source trace, diagnostic fixes,
   query identity, and generated-artifact mapping. Add golden Unicode,
   multi-file, embedded-expression, and multi-frame source-map cases.
2. **Interactive engine boundary.** Add byte-backed versioned documents,
   cancellation, stale-result rejection, error-recovered parse access, schema
   and module cache keys, and typed tooling requests without depending on LSP
   or an IDE SDK.
3. **Machine CLI and CI foundation.** Add stdin plus logical URI, strict
   stdout/stderr separation, canonical versioned JSON, deterministic
   fingerprints, SARIF, and manifest-example JUnit. Keep vendor annotation
   adapters as projections.
4. **Baseline language packaging.** Generate file associations, syntax
   grammar/configuration, snippets, VS Code custom data, and IntelliJ web-types
   from schema packages. Define embedded CEM-QL/CEMT boundaries and prevent
   generated metadata drift.
5. **LSP validation MVP.** Implement initialize/shutdown, incremental document
   sync, configuration, cancellation, and exact diagnostics for open/change/
   save. Prove unsaved Unicode buffers and cross-file source traces in protocol
   tests.
6. **First desktop clients.** Ship a VS Code extension and IntelliJ LSP plugin
   that start the same server, select run config/schema packages, expose logs,
   and avoid duplicate validators. Publish tested IDE/version/clone support.
7. **LSP code insight.** Add schema-driven completion, hover, signature help,
   symbols, folding/selection ranges, semantic tokens, and configurable inlay
   hints for CEM-ML, schemas, CEMT, CEM-QL, and embedded expressions.
8. **Navigation and safe edits.** Add definition/references/document links,
   source-trace navigation, deterministic code actions, formatting, and then
   scope-aware rename. Validate edits against the requesting document version.
9. **Run, preview, and test workflows.** Add run-query-at-cursor, transform
   preview/diff, generated-artifact navigation, schema-package example tests,
   VS Code task/test UI, and IntelliJ run configurations/tool windows.
10. **Chromium read-only inspector.** Define the development-only
    `cem-element` tooling bridge and compatibility adapter; ship an Elements
    sidebar for owner/declaration/snapshot/diagnostics/render/source trace with
    default redaction.
11. **Chromium CEM-QL workbench.** Ship a WASM-backed panel for explicit-
    language queries and transform previews over selected/sanitized inputs;
    then add optional installed native-messaging mode with parity tests.
12. **Pausable debug runtime.** Design safe points, execution/frame ids,
    breakpoint binding, immutable frame snapshots, lazy values, evaluate
    contexts, resource waits, cancellation, and debugger security. Test this in
    Rust before any IDE adapter.
13. **DAP server.** Implement launch/attach as appropriate, breakpoints,
    threads, stack trace, scopes, variables, continue/step, evaluate, loaded
    sources, output, termination, and diagnostic exception filters over the
    debug runtime.
14. **Desktop debugger clients.** Package DAP for VS Code and bridge the same
    session into IntelliJ XDebugger/run configurations. Add source/output
    navigation and explicit query-language configuration.
15. **Chromium debugging.** Reuse the debug-session controller in the CEM panel
    for transform/query stepping and scopes. Correlate with CDP JavaScript only
    when actual page script execution is involved.
16. **Advanced workspace and delivery features.** Add full workspace indexing,
    dependency-aware incremental validation, baselines/new-findings policy,
    performance budgets, remote/container support, authenticated shared daemon
    only if justified, and deterministic replay/reverse-debug feasibility.

## Decisions to Preserve

- One semantic engine; protocol and IDE layers are projections.
- LSP for editing and DAP for debugging; do not stretch either beyond its job.
- Exact ranges and source identities are foundations, not post-MVP polish.
- Unsaved document bytes are first-class inputs.
- Query language is selected by content/schema/transform identity, never by
  guessing source syntax.
- Human terminal output is not a machine protocol.
- IDE plugins stay thin until a proven missing feature justifies proprietary
  implementation.
- Chromium gets a custom CEM panel/sidebar; CDP remains the JavaScript/page
  debugger and is not the CEM query runtime protocol.
- Browser inspection is read-only and redacted by default.
- CI vendor formats never become alternate diagnostic semantics.

## Research References

- [LSP 3.17 specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
- [VS Code Language Server Extension Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)
- [VS Code Debugger Extension Guide](https://code.visualstudio.com/api/extension-guides/debugger-extension)
- [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
- [IntelliJ Platform LSP API](https://plugins.jetbrains.com/docs/intellij/language-server-protocol.html)
- [IntelliJ syntax and external annotator APIs](https://plugins.jetbrains.com/docs/intellij/syntax-highlighting-and-error-highlighting.html)
- [Chrome DevTools panels API](https://developer.chrome.com/docs/extensions/reference/api/devtools/panels)
- [Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
- [Chrome DevTools Protocol Debugger domain](https://chromedevtools.github.io/devtools-protocol/1-3/Debugger/)
- [SARIF 2.1.0 plus errata](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html)
- [GitHub SARIF support](https://docs.github.com/en/code-security/code-scanning/integrating-with-code-scanning/sarif-support-for-code-scanning)
- [GitLab Code Quality report format](https://docs.gitlab.com/ci/testing/code_quality/#code-quality-report-format)
