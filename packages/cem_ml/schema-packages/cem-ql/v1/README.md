# CEM-QL Query Resource Schema Package

Status: schema, examples, formatter, and colorizer package frame

This package defines registry identity for CEM-QL query source modules and
compiled query artifacts.

CEM-QL source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/vnd.cem.query+cem-ql`
content type are parsed by the `cem-ql` crate.

## Syntax Baseline

CEM-QL authoring is Rust-first: comparison, arithmetic, boolean, cast, block,
and binding forms use Rust-style spelling such as `==`, `/`, `%`, `&&`, `||`,
`!`, `expr as Type`, `{ let name = value; body }`, and `declare let name =
value`. XPath, XQuery, XSLT, JQ, and Python are functional parity references
only; their path, variable, operator, and clause syntax is not the canonical
CEM-QL surface.

The `-` operator is canonical for both numeric subtraction and stream
difference. The parser records one Rust-style token; type checking and IR
lowering resolve numeric operands to subtraction and stream or collection
operands to set difference. Mixed numeric/stream operands are validation
errors, and `seq:difference(a, b)` remains only a named helper alias.

## Owned Identities

- Schema URI: `https://cem.dev/ns/query/cem-ql/1`
- Primary source content type: `application/vnd.cem.query+cem-ql`
- Authoring alias: `text/cem-ql`
- Compiled artifact alias: `application/vnd.cem.query-artifact+cem-bin`
- Legacy/internal cache aliases: `cem-ql/1`, `cem-ql/module`

## Standards And CEM Policy Matrix

| Area | External contract | CEM policy |
| ---- | ----------------- | ---------- |
| Media type | CEM-QL is CEM-owned query syntax, not an IANA-registered media type. | `application/vnd.cem.query+cem-ql` is the primary source content type. `text/cem-ql` is accepted as an authoring alias only. |
| Charset | CEM-QL source is plain text. | Direct parsing requires valid UTF-8. Other encodings require an explicit converter before CEM-QL validation or formatting. |
| Line endings | No external CEM-QL standard requires CRLF. | Formatter output defaults to LF for deterministic repository fixtures. Use generic formatter option `lineEnding=crlf` only when a downstream transport requires it, or `lineEnding=preserve` for source-preserving preview flows. |
| Syntax family | XPath, XQuery, XSLT, JQ, and Python are parity references, not source compatibility promises. | Rust-style operator and binding spelling is canonical. Compatibility spellings are parser facts that map to diagnostics and replacement hints. |
| Module identity | Query modules must have stable importable identities. | Source modules require `module "..."`; compiled artifacts carry source URI, module URI, policy stamps, import closure, and optional source maps. |
| Execution boundary | Query evaluation may read host resources or imported modules. | Validation and formatting do not execute arbitrary resource reads. Runtime execution must go through resolver policy, import policy, and host capability checks. |

Primary references for syntax behavior are the Rust `cem-ql` parser, lexer, and
evaluator crates in this workspace. XPath, XQuery, XSLT, JQ, and Python
documents are treated as functional parity material only when designing helper
coverage; they are not normative CEM-QL grammar references.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, `md`, and `none`.

The CEM-QL formatter/colorizer artifacts keep their public entrypoints as
`cem-ql.format-tree` and `cem-ql.color-tree`, then delegate to package-owned
private helpers for Rust-first token metadata. The formatter emits formatted
CEM-tree writer-token nodes for `compact`, `pretty`, and `tabular`; the generic
writer is the stage that turns those nodes into terminal bytes or HTML. The
current CEM-QL profiles are source-preserving token-stream layouts, so the
profiles are deterministic but intentionally close in visible output until
AST-aware layout rules are added. Canonical operators are grouped under
`cem-ql.operator.*` roles before color roles are applied. Deprecated
XPath/XQuery/Python spellings such as `div`, `mod`, `and`, `or`, `not`,
`then`, `return`, `some`, `every`, `True`, `False`, `None`, and `lambda` are
modeled as diagnostics, not alternate canonical syntax.

## Formatter Presentation Profiles

CEM-QL formatting has two audiences: source-preserving review of checked-in
query modules and future AST-aware canonicalization. The current formatter
profiles are source-preserving token-stream profiles:

- `compact`: the default profile. Today it preserves source token text and
  normalizes line endings according to the generic `lineEnding` option.
- `pretty`: currently an intentional source-preserving alias for `compact`.
  It exists as the public profile that will own readable indentation and
  wrapping once AST-aware layout rules are implemented.
- `tabular`: currently an intentional source-preserving alias for `compact`
  except for profile metadata. It exists as the public profile that will own
  vertically scannable declarations, operator groups, and import/declaration
  summaries once AST-aware layout rules are implemented.

Generic formatter options are unprefixed:

- `lineEnding=lf|crlf|preserve`: output line-ending policy. `lf` is the default
  used by repository fixtures, `crlf` is available for transport-specific
  output, and `preserve` keeps source whitespace exactly where token source
  preservation metadata is available.

No CEM-QL-specific formatter options are currently implemented. Future CEM-QL
formatter options should use the `cemQl.` namespace only when they express
query-specific layout or diagnostic presentation semantics.

Colorizer profiles apply semantic roles to the formatted CEM tree:

- `terminal`: ANSI-capable terminal output through the generic writer;
- `html`: escaped HTML spans with CEM syntax role classes/styles;
- `md`: Markdown-safe role output when a Markdown writer profile is selected;
- `none`: no styled output, preserving formatted text only.

## Query Safety Boundary

CEM-QL source validation and formatting are passive. They parse, tokenize, and
render source text; they must not execute host resource reads, network imports,
query evaluation, template execution, or policy-sensitive resolver actions.

Runtime evaluation is a separate boundary. Evaluators that resolve imports,
call `read(...)`, inspect CEM-ML resources, invoke template helpers, or access
host state must run under explicit resolver policy and host capability
controls. Formatting and preview generation must remain safe for untrusted
query text and must not treat source text as executable HTML.

## Formatter And Preview SDLC

CEM-QL formatter/colorizer changes follow the same lifecycle as CSV and other
schema-package output support:

1. Add or update the smallest CEM-QL fixture that exposes the behavior.
2. Declare the fixture in `package.cem` with expected result and diagnostics.
3. Add focused Rust or CLI tests for parser/token facts, formatter output
   bytes, HTML/terminal output, and output-stage metadata.
4. Update README command examples and SVG previews under `examples/previews/`
   when visible output changes.
5. Run the CEM-QL package verify target, which validates the package,
   validates manifest-declared examples, and checks README SVG preview drift.

Tracked but not complete:

- AST-aware `pretty` and `tabular` layout rules beyond source-preserving token
  streams;
- schema-owned diagnostic policy execution from parse facts rather than bridge
  logic selecting some `cem.ql.*` codes directly;
- examples for duplicate import/declaration, unresolved import, type errors,
  and compiled artifact/cache validation. Alias content type, line-ending
  policy, comments/whitespace, invalid UTF-8, and token byte-range preservation
  now have package examples and focused conversion coverage.

## Resource Model

The schema describes the query resource model used by loaders and caches:

- query modules declare a module URI;
- imports bind other module URIs through explicit aliases;
- declarations define immutable `declare let` bindings and functions;
- expressions are compiled to typed evaluator IR;
- compiled artifacts carry hash, mode, policy stamps, import closure, and
  optional source-map sidecars.

## Design: Schema-Owned Parsing And Validation

The target is schema-owned CEM-QL semantics, not a claim that CEM-QL bytes are
parsed without host code. CEM-QL is not CEM-ML syntax, so a native lexer/parser
still supplies byte-accurate facts. The package boundary is that those facts are
projected as data, and `schema/cem-ql.cem` owns which facts become diagnostics,
including codes, severities, and structured details.

### Current Boundary

The current direct conversion bridge parses CEM-QL source through the `cem-ql`
crate, projects lexer tokens into a CEM tree subject, and routes that subject
through package-owned formatter/colorizer CEMT assets before the generic writer
emits terminal or HTML bytes. The bridge still constructs some diagnostics
directly, including invalid UTF-8, missing module URI, parser diagnostics, and
legacy syntax replacement facts.

The schema now declares a schema-facing report shape for source identity,
encoding status, token streams, parse facts, and fact-bound diagnostics. That
makes the intended ownership inspectable even while the runtime migration is
still partly native.

### Schema-Facing Parse Report

The parser primitive should project a stable report shape that the schema,
formatter, colorizer, and writer can preserve:

- `source`: URI, content type, normalized media type, byte length, and detected
  line-ending style;
- `encoding-report`: declared charset, normalized charset, decoder status, and
  first invalid byte when present;
- `token-stream`: ordered tokens with token kind, role, lexeme, cooked value,
  operator role, legacy spelling, replacement hint, byte range, line/column
  range, and source id;
- `parse-facts`: non-diagnostic facts such as `invalid-utf8`, `parse-error`,
  `module-uri-missing`, `legacy-syntax`, `import-alias-duplicate`,
  `declaration-duplicate`, `unresolved-import`, `type-error`,
  `artifact-hash-mismatch`, `policy-mismatch`, and `source-map-unavailable`;
- formatter/colorizer metadata: source maps and output spans on writer-token
  nodes so HTML, terminal, and plain text output preserve source provenance.

The schema owns the mapping from facts to diagnostics. For example,
`parseFacts.kind = "module-uri-missing"` maps to
`cem.ql.module_uri_missing`, while `parseFacts.kind = "legacy-syntax"` maps to
the appropriate `cem.ql.*` replacement diagnostic declared in
`schema/cem-ql.cem`.

### Remaining Migration Plan

1. Move all CEM-QL source validation to produce neutral parse facts first.
2. Make schema-declared `@fact-kind`, `@diagnostic`, and `@behavior` bindings
   decide emitted `cem.ql.*` codes and severities.
3. Keep direct source-output conversion registered at the engine/context layer
   so direct API users and CLI users share the same parser/output path.
4. Expand examples so every runtime-owned parser, resolver, type, policy, and
   artifact condition has a schema-owned fixture.
5. Add mutation-style contract tests that prove changing the schema bindings
   changes diagnostics without editing Rust bridge logic.

### Verification Gates

The CEM-QL package is not complete until these gates pass:

- direct `RealCemMlEngine::convert` and CLI `convert` both use the registered
  CEM-QL source-output path;
- package examples validate through the manifest-derived schema-owned example
  harness;
- source ranges for parser/token diagnostics survive through CLI JSON and
  formatted HTML output;
- CEMT formatter/colorizer assets consume the schema-facing CEM-QL token/report
  model rather than reparsing source bytes;
- `yarn nx run cem_ml_schema_package_cem_ql_v1:verify` fails on README/SVG
  drift, formatter/colorizer drift, and schema example drift.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests and the package-local `verify` target.

<!--
AI maintenance: when changing any command example below, its referenced CEM-QL
fixture, formatter/colorizer assets, CLI report shape, HTML wrapper, role-color
mapping, or CEM-QL presentation output, refresh the matching SVG previews with
`node packages/cem_ml/schema-packages/cem-ql/v1/scripts/verify-previews.mjs --update`
and commit the preview changes in the same change. If the visible output is
unchanged, state that explicitly in the change notes.
-->

| Example                                                                 | Purpose                                                                                           | Expected result                       |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------- |
| [`basic-query.cemql`](examples/basic-query.cemql)                       | Minimal query module with a module URI, immutable binding declaration, and expression.            | Pass                                  |
| [`module-query.cemql`](examples/module-query.cemql)                     | Query module with import, immutable binding declaration, function declaration, and conditional expression. | Pass                                  |
| [`operators-and-control.cemql`](examples/operators-and-control.cemql)   | Arithmetic, comparisons, boolean short-circuit operators, type tests/casts, set operators, block `let`, and `if`/`else`. | Pass                                  |
| [`collections-and-pipelines.cemql`](examples/collections-and-pipelines.cemql) | Record streams, dot pipelines, current-item projection, `for` mapping, and `any`/`all` helpers. | Pass                                  |
| [`stdlib-data-helpers.cemql`](examples/stdlib-data-helpers.cemql)       | Sequence helpers, string helpers, number helpers, date/time helpers, and report helper calls.     | Pass                                  |
| [`host-resource-helpers.cemql`](examples/host-resource-helpers.cemql)   | State helpers, template helpers, CEM-ML helpers, content-type constants, `read(...)`, and JSON-array resource boundary shape. | Pass                                  |
| [`invalid-parse.cemql`](examples/invalid-parse.cemql)                   | Incomplete expression rejected by the CEM-QL parser.                                              | Fail with `cem.ql.parse_error`        |
| [`invalid-missing-module.cemql`](examples/invalid-missing-module.cemql) | Query source missing the required module URI declaration.                                         | Fail with `cem.ql.module_uri_missing` |
| [`invalid-old-syntax.cemql`](examples/invalid-old-syntax.cemql)         | XPath boolean spelling rejected with a Rust-first replacement diagnostic.                         | Fail with `cem.ql.use_rust_boolean_ops` |

The current parser has record literals and stream sequence literals; it does
not define a separate `[...]` array literal. Arrays enter CEM-QL through host
and resource boundaries such as JSON `read(...)` or WASM bindings, and are
therefore represented in the host/resource helper example rather than as a
new surface syntax.

Validate an example explicitly against this schema from a built CLI binary:

```bash
dist/target/cem_ml_cli/debug/cem-ml validate \
  --format json \
  --content-type application/vnd.cem.query+cem-ql \
  --schema https://cem.dev/ns/query/cem-ql/1 \
  packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql
```

![Preview of the CEM-QL validation JSON report](examples/previews/basic-query-validate.svg)

Convert the same example with the package-owned tabular formatter and terminal
colorizer:

```bash
dist/target/cem_ml_cli/debug/cem-ml convert \
  packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql \
  --content-type application/vnd.cem.query+cem-ql \
  --schema https://cem.dev/ns/query/cem-ql/1 \
  --to-content-type application/vnd.cem.query+cem-ql \
  --to-schema https://cem.dev/ns/query/cem-ql/1 \
  --cemt-formatter cem-ql.format-tree \
  --cemt-formatter-profile tabular \
  --cemt-colorizer cem-ql.color-tree \
  --cemt-color-profile terminal \
  --output-color-type ansi-256
```

![Preview of the CEM-QL tabular formatter with terminal colors](examples/previews/basic-query-tabular-terminal.svg)

Convert the same formatted/colorized tree to HTML:

```bash
dist/target/cem_ml_cli/debug/cem-ml convert \
  packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql \
  --content-type application/vnd.cem.query+cem-ql \
  --schema https://cem.dev/ns/query/cem-ql/1 \
  --to-content-type text/html \
  --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter cem-ql.format-tree \
  --cemt-formatter-profile tabular \
  --cemt-colorizer cem-ql.color-tree \
  --cemt-color-profile html \
  --output-color-type html-css-vars
```

![Preview of the CEM-QL tabular formatter with HTML colors](examples/previews/basic-query-tabular-html.svg)
