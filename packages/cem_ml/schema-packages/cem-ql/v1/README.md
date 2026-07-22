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

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

The CEM-QL formatter/colorizer artifacts keep their public entrypoints as
`cem-ql.format-tree` and `cem-ql.color-tree`, then delegate to package-owned
private helpers for Rust-first token metadata. Canonical operators are grouped
under `cem-ql.operator.*` roles before color roles are applied. Deprecated
XPath/XQuery/Python spellings such as `div`, `mod`, `and`, `or`, `not`,
`then`, `return`, `some`, `every`, `True`, `False`, `None`, and `lambda` are
modeled as diagnostics, not alternate canonical syntax.

## Resource Model

The schema describes the query resource model used by loaders and caches:

- query modules declare a module URI;
- imports bind other module URIs through explicit aliases;
- declarations define immutable `declare let` bindings and functions;
- expressions are compiled to typed evaluator IR;
- compiled artifacts carry hash, mode, policy stamps, import closure, and
  optional source-map sidecars.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

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

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.query+cem-ql \
  --schema https://cem.dev/ns/query/cem-ql/1 \
  packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql
```
