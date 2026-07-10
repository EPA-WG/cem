# JSON Schema Resource Schema Package

Status: schema, examples, formatter, and colorizer package frame

This package defines registry identity for JSON Schema documents.

JSON Schema source is JSON text, not CEM-ML syntax. The schema package and this
manifest are authored in CEM-ML, but resources with the `application/schema+json`
content type are parsed as JSON and interpreted as JSON Schema vocabulary
documents.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/json-schema/1`
- Primary content type: `application/schema+json`
- Common file forms: `.schema.json`, `.jsonschema`

This package depends on the generic JSON package because JSON Schema documents
are JSON values first. It does not claim `application/json`; callers should use
`application/schema+json` or an explicit schema identity when the same bytes are
intended to be interpreted as a JSON Schema document.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

## Resource Model

The schema describes the JSON Schema document model used by registry loaders and
tooling:

- schemas declare a dialect such as Draft 2020-12 through `$schema`;
- schema resources may carry `$id`, anchors, and dynamic anchors;
- `$ref` and `$dynamicRef` edges are URI references resolved by the loader;
- vocabularies define keyword sets and whether support is required;
- validation, applicator, annotation, format, and unevaluated keywords are kept
  distinct so engines can report unsupported vocabulary precisely.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-schema.schema.json`](examples/basic-schema.schema.json) | Minimal Draft 2020-12 object schema with required properties. | Pass |
| [`catalog-schema.schema.json`](examples/catalog-schema.schema.json) | Draft 2020-12 schema with `$defs`, arrays, string constraints, and `$ref`. | Pass |
| [`invalid-unsupported-dialect.schema.json`](examples/invalid-unsupported-dialect.schema.json) | Schema declaring an unsupported pre-2020-12 dialect. | Fail with `cem.json_schema.unsupported_dialect` |
| [`invalid-parse.schema.json`](examples/invalid-parse.schema.json) | JSON syntax error in a schema resource. | Fail with `cem.json_schema.parse_error` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/schema+json \
  --schema https://cem.dev/ns/data/json-schema/1 \
  packages/cem_ml/schema-packages/json-schema/v1/examples/basic-schema.schema.json
```
