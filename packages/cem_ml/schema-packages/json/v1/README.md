# JSON Resource Schema Package

This package defines registry identity for generic JSON text resources.

JSON source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/json` content type are
parsed by a JSON parser or adapter.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/json/1`
- Primary content type: `application/json`
- Authoring/legacy alias: `text/json`

This package does not claim JSON Schema (`application/schema+json`) or
CEM-specific projection/vendor JSON types such as `application/vnd.cem.*+json`.
Those formats have their own schemas and converter rules.

## Resource Model

The schema describes JSON values as a lossless resource model:

- documents contain one root value;
- objects preserve member order and key source identity;
- arrays preserve item order;
- strings, numbers, booleans, and null preserve their JSON value kind;
- parsers should retain lexical/source-map information when available.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-object.json`](examples/basic-object.json) | Minimal object with string, boolean, and number values. | Pass |
| [`nested-data.json`](examples/nested-data.json) | Nested object/array document with scalar values and null. | Pass |
| [`invalid-trailing-comma.json`](examples/invalid-trailing-comma.json) | Object with a trailing comma rejected by the JSON parser. | Fail with `cem.json.parse_error` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/json \
  --schema https://cem.dev/ns/data/json/1 \
  packages/cem_ml/schema-packages/json/v1/examples/basic-object.json
```
