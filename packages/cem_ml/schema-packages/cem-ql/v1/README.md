# CEM-QL Query Resource Schema Package

Status: schema, examples, formatter, and colorizer package frame

This package defines registry identity for CEM-QL query source modules and
compiled query artifacts.

CEM-QL source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/vnd.cem.query+cem-ql`
content type are parsed by the `cem-ql` crate.

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

## Resource Model

The schema describes the query resource model used by loaders and caches:

- query modules declare a module URI;
- imports bind other module URIs through explicit aliases;
- declarations define variables and functions;
- expressions are compiled to typed evaluator IR;
- compiled artifacts carry hash, mode, policy stamps, import closure, and
  optional source-map sidecars.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example                                                                 | Purpose                                                                                           | Expected result                       |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------- |
| [`basic-query.cemql`](examples/basic-query.cemql)                       | Minimal query module with a module URI, variable declaration, and expression.                     | Pass                                  |
| [`module-query.cemql`](examples/module-query.cemql)                     | Query module with import, variable declaration, function declaration, and conditional expression. | Pass                                  |
| [`invalid-parse.cemql`](examples/invalid-parse.cemql)                   | Incomplete expression rejected by the CEM-QL parser.                                              | Fail with `cem.ql.parse_error`        |
| [`invalid-missing-module.cemql`](examples/invalid-missing-module.cemql) | Query source missing the required module URI declaration.                                         | Fail with `cem.ql.module_uri_missing` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.query+cem-ql \
  --schema https://cem.dev/ns/query/cem-ql/1 \
  packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql
```
