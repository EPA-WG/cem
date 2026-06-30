# CEM-Native Template Schema Package

Status: initial source package

This package defines the CEM-native template module language used by template
adapters. It is a schema package for authored template modules, not for CLI
transform graph configuration.

Owned schema URI:

```text
https://cem.dev/ns/template/cem-native/1
```

Primary content type:

```text
application/vnd.cem.template+cem
```

Current runtime aliases are also declared so callers can keep using generic
CEM-ML source content types with an explicit template schema.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-template.cem`](examples/basic-template.cem) | Minimal module with one template and body. | Pass |
| [`module-template.cem`](examples/module-template.cem) | Module with import metadata, params, nested template output, and a template call with `with:*` data propagation. | Pass |
| [`invalid-missing-required-attribute.cem`](examples/invalid-missing-required-attribute.cem) | Template declaration missing its required `name` attribute. | Fail with `cem.schema_model.missing_required_attribute` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.template+cem \
  --schema https://cem.dev/ns/template/cem-native/1 \
  packages/cem_ml/schema-packages/cem-native-template/v1/examples/basic-template.cem
```
