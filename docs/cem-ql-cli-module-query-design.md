# CLI CEM-QL module-query mode

Status: implemented with the accepted first-delivery scope (2026-09-25).
Registered built-in `cem:` imports and local declarations are supported;
external module loading/linking remains deferred. The execution checklist is
[todo.md](todo.md).

## Problem and implementation

Previously the CEM-QL CLI bridge compiled every source as a standalone
expression, despite advertising module identities. It also discarded nonfatal
evaluator diagnostics. The bridge now selects a typed expression or module
artifact from the declared identity and preserves warnings in results and
reports. All 30 shared URL matrix cases execute through the CLI, including
imported aliases and user-function isolation. Native server/browser rendering
remains separate evidence from the legacy Edge/SSR transport host.

Relevant existing owners:

- [CLI query arguments](../packages/cem_ml_cli/src/cli.rs) and
  [dispatch](../packages/cem_ml_cli/src/dispatch.rs) own source acquisition and
  command/report behavior.
- [Common query contract](../packages/cem_ml/src/query.rs) owns language identity,
  native input/result ownership, execution controls and exporter selection.
- [CEM-QL bridge](../packages/cem_ml_transform_cem_ql/src/lib.rs) owns the compiled
  query artifact and native evaluator adapter.
- [CEM-QL APIs](../packages/cem_ql/src/api.rs) own module/expression compilation
  and evaluation; [import policy](../packages/cem_ql/src/resolve.rs) checks grants.
- The [schema package](../packages/cem_ml/schema-packages/cem-ql/v1/README.md)
  already defines distinct module and standalone-expression identities.

## Explicit mode selection

Keep the existing `query` command and its arguments. Select the source kind from
its declared media type and compatible schema, before parsing source text.
Do not add a second mode flag, infer a mode from file extensions, or retry an
expression parse as a module parse after failure.

| Source kind | Content type | Schema |
| --- | --- | --- |
| Standalone expression | `application/vnd.cem.query-expression+cem-ql` | `https://cem.dev/ns/query/cem-ql/1#expression` |
| Query module | `application/vnd.cem.query+cem-ql` | `https://cem.dev/ns/query/cem-ql/1` |
| Module authoring alias | `text/cem-ql` | `https://cem.dev/ns/query/cem-ql/1` |

An omitted schema is derived from the declared content type. A supplied schema
must match that source kind, rather than merely belonging to CEM-QL. Existing
expression invocations keep their behavior and rejection of module statements.
Module media types currently listed as accepted gain their advertised semantics;
callers using them for headerless expressions must use the expression identity.
Record this compatibility correction in CLI help and package documentation.

Module invocation:

```sh
cem-ml query catalog.xml --content-type application/xml \
  --query-file links.cemql \
  --query-content-type application/vnd.cem.query+cem-ql \
  --query-schema https://cem.dev/ns/query/cem-ql/1 \
  --output json
```

`links.cemql`:

```cem-ql
module "urn:example:links"
import "cem:stdlib/url" as u

declare let base = "https://example.test/docs/"
declare function local:link(path as string) {
    u:href(path, base)
}

local:link("guide")
```

The explicit JSON result contains an `any-uri` item whose value is
`https://example.test/docs/guide`.

## Module execution and typed ownership

- Apply the schema package's module-URI and declaration rules. Require an
  executable root expression for the CLI runner; a declarations-only library
  is not a CLI query entrypoint. Do not invent an implicit `main()` call or
  concatenate multiple top-level expressions into a result.
- Compile imports, immutable declarations, functions and the root expression
  through the existing native CEM-QL module compiler and IR evaluator. Preserve
  lexical scope, alias rules, duplicate diagnostics and user-name isolation.
- Keep `DATA` required in this first mode. Bind its retained native item stream
  as `input`, with the same query lifecycle ownership and context-item behavior
  as expression mode. Do not convert the data AST to JSON or introduce a new
  JavaScript object binding path. Local declarations follow the native compiler's
  lexical shadowing rules; they do not mutate the retained input owner.
- Keep named host bindings and host capabilities at their current explicit
  boundaries. Module mode does not enable unimplemented `--param` behavior,
  implicit `module_url()` capabilities, custom callbacks or ambient I/O.
- Represent the prepared source kind explicitly in the bridge's owned compiled
  artifact, retaining expression metadata for expressions and typed compiled
  module IR for modules. Preserve the declared identity through prepare,
  evaluation, result export and diagnostics; do not canonicalize both kinds to
  the expression anchor.
- Preserve source URI, exact source ranges, full embedding maps and input-owner
  lifetime. The root module URI is an authored module identity, not an implicit
  network fetch or a substitute for the query source URI.

## Accepted import scope

**Accepted first delivery:** registered built-in `cem:` imports, including
`cem:stdlib/url`, plus declarations local to the entry module. Unknown built-ins
and all external/plugin imports fail before evaluation through existing import
policy diagnostics. Do not enable a URI scheme merely to bypass validation.

This delivers the two blocked URL cases without claiming external module
linking. `ImportPolicy::resolve_import` can classify an authorized external URI;
it does not fetch, compile or link that module's declarations into an executable
closure. Existing policy acceptance is therefore insufficient implementation
proof for external imports.

**Deferred follow-up:** external query-library loading.
That requires a separate, reviewable import-closure contract: source-relative
resolution for inline and file-backed roots, lifecycle loader ownership, scheme
and resource-policy grants, source/hash identity, cycle/depth/byte limits,
deterministic alias/export linking, cancellation, retained source maps and
atomic failure before execution. It must use the shared import/load boundaries.
Do not add a CLI-local file/HTTP loader or concatenate imported source strings.

Decision D1: the user accepted built-in imports and local declarations on
2026-09-25. External loading requires a separate design before implementation.

## Results, reports and execution controls

Use the same native `ItemStream`, input owner and requested result exporters as
expression mode. Keep item types, order, duplicate entries, warnings and empty
results intact. JSON remains an explicit `--output json` export; do not introduce
an AST serialization or implicit JSON handoff to enable module execution.

Reuse the common scope policy, cancellation signal, result/work limits and
report path. Hard failures return nonzero and no partial result output. Preserve
original `cem.ql.*` diagnostics with source maps exactly once; separately test the
existing CLI `cem.ql.query_evaluation_failed` summary diagnostic rather than
mistaking it for a duplicate language diagnostic. Setter warnings remain
nonfatal and ordered.

The low-level module `compile()` currently reduces failures to a compact
`CompileError`. The executable-module `compile_module()` API retains structured native
diagnostics for the CLI. Do not reconstruct byte
ranges by parsing error messages, silently lose declaration/import diagnostics,
or add a second parser in the CLI. Test this at the native compiler/bridge layer
before adding CLI fixtures.

Compile each entry module once per prepared invocation. Any later reuse must
include source kind, parser/stdlib versions, import closure and policy identity.
The shared AST cache proposed at the end of [roadmap.md](../roadmap.md) remains
separate future work.

## Implementation sequence and acceptance

1. Resolve D1 and record the chosen scope in `todo.md`.
2. Add native tests for exact expression/module identity pairing, module shape,
   declaration/alias isolation, `input` ownership, structured diagnostics and
   denied imports. Add the typed compiled-artifact branch and shared evaluator
   wiring only after these cases fail for the intended reason.
3. Add file-backed and inline CLI module fixtures using explicit module headers.
   Retain negative tests showing expression mode rejects `module`/`import`/
   `declare`, and mismatched schema/media pairs never change parser mode.
4. Execute the shared URL matrix's two module rows through module mode; preserve
   all expression-row results, ordered warnings and the CLI failure envelope.
   Extend with local immutable bindings/functions, retained input access,
   missing entry expression, invalid UTF-8, duplicate declarations/imports,
   cancellation, budgets and the chosen external-import rejection behavior.
5. Generate supported examples, update query help/command-schema expectations
   where descriptions or advertised identities change, and rerun the native,
   CLI, Node WASM and browser URL gates.

This implementation will touch shared query/bridge/compiler modules. Run their
appropriate package suites at that point, in addition to focused URL checks.
No workspace-wide test task is required.
