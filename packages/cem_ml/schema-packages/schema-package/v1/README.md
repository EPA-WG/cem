# CEM Schema Package Metadata Package

Status: initial source package

This package defines `package.cem`, the metadata manifest found at:

```text
schema-packages/{schema-name}/{version}/package.cem
```

Owned schema URI:

```text
https://cem.dev/ns/schema-package/1
```

Primary content type:

```text
application/vnd.cem.schema-package+cem
```

The package metadata schema is separate from the schema definition language. It
describes package registration metadata, while `https://cem.dev/ns/schema/1`
describes validation schemas for input content.

Converter declarations are registry-owned metadata in `package.cem`. A
converter can declare a Rust implementation hook or CEMT template, source and
target content identities, fallback hook, readiness, and planner `cost`.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-package.cem`](examples/basic-package.cem) | Minimal `package.cem` manifest with schema, content type, and namespace registration. | Pass |
| [`converter-package.cem`](examples/converter-package.cem) | Package manifest with aliases and a CEMT converter declaration. | Pass |
| [`invalid-unclosed-package.cem`](examples/invalid-unclosed-package.cem) | Missing closing package scope syntax diagnostic. | Fail with `cem.schema.unclosed_scope` |
| [`invalid-missing-required-attribute.cem`](examples/invalid-missing-required-attribute.cem) | Manifest schema entry missing its required `source` attribute. | Fail with `cem.schema_model.missing_required_attribute` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema-package+cem \
  --schema https://cem.dev/ns/schema-package/1 \
  packages/cem_ml/schema-packages/schema-package/v1/examples/basic-package.cem
```
