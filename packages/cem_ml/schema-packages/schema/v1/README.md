# CEM Schema Definition Language Package

Status: initial source package

This package defines the CEM-ML schema declaration language used to describe
validation schemas for input content.

Owned schema URI:

```text
https://cem.dev/ns/schema/1
```

Primary content type:

```text
application/vnd.cem.schema+cem
```

Schema source files are ordinary CEM-ML documents using this namespace for the
schema-authoring vocabulary. The target schema being described is carried by the
`schema @namespace` attribute.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-schema.cem`](examples/basic-schema.cem) | Minimal schema definition with content type, element, and attribute declarations. | Pass |
| [`typed-resource-schema.cem`](examples/typed-resource-schema.cem) | Resource schema with `uses`, namespace claims, diagnostics, and open-content policy. | Pass |
| [`invalid-unclosed-schema.cem`](examples/invalid-unclosed-schema.cem) | Missing closing schema scope syntax diagnostic. | Fail with `cem.schema.unclosed_scope` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema+cem \
  --schema https://cem.dev/ns/schema/1 \
  packages/cem_ml/schema-packages/schema/v1/examples/basic-schema.cem
```
