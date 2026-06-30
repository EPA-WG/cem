# CEM Transform Template Schema Package

Status: initial source package

This package defines CEMT (`.cemt`) resources. CEMT is the primary declarative
converter implementation language in the schema content registry design.

Owned schema URI:

```text
https://cem.dev/ns/transform/cem/1
```

Primary content type:

```text
application/vnd.cem.transform+cem
```

CEMT reuses the CEM-native template module language. Source and target content
identity is not embedded in `.cemt`; it is declared by `package.cem` converter
edges so the same template execution surface can participate in registry
planning.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-transform.cemt`](examples/basic-transform.cemt) | Minimal CEMT module with one template body. | Pass |
| [`module-transform.cemt`](examples/module-transform.cemt) | Converter template module with import metadata, params, nested output, and `with:*` data propagation. | Pass |
| [`invalid-missing-required-attribute.cemt`](examples/invalid-missing-required-attribute.cemt) | Template declaration missing the inherited required `name` attribute. | Fail with `cem.schema_model.missing_required_attribute` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.transform+cem \
  --schema https://cem.dev/ns/transform/cem/1 \
  packages/cem_ml/schema-packages/cem-transform/v1/examples/basic-transform.cemt
```
