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

## Field Contract Requirement

The schema definition language owns field contracts for every schema-declared
construct. A schema must be able to declare required, optional, and forbidden
fields or attributes; accepted children; value types and vocabularies; defaults;
dependent fields; mutually exclusive groups; conditional rules; open-content
policy; and the diagnostic contract for failed checks.

This follows the same separation of concerns used by established schema systems:
RELAX NG patterns own structure, XSD owns complex types and attribute use,
JSON Schema owns `properties`, `required`, `dependentRequired`, and
`if`/`then`, and SHACL owns shape constraints. In CEM, the `.cem` schema source
is the authority. Rust validators compile and evaluate the schema declarations
and perform operational checks, but they must not be the source of
package-specific required-field lists or conditional field rules.

Field-check diagnostics should identify the contract family and carry structured
details such as target, check kind, expected fields, missing fields, invalid
fields, forbidden fields, and actual values. They should not require one
diagnostic code per individual metadata or schema field.

Field contracts can be gated by value selectors such as `when-attribute` plus
`when-values`, and by presence selectors such as `when-present-attributes`.
Use presence selectors for dependent-required rules such as "when this
attribute is present, require these other attributes".
Use `forbidden-attribute-values` for value-specific exclusions, such as a
schema-owned mutual exclusion where one attribute value makes another attribute
value invalid while leaving other values legal.
Use `required-children` plus `max-one-children` for exact-one child occurrence
contracts, such as schema package converter `from`/`to` endpoints.
Use `path-layout-attributes` with `path-layout-prefix` and
`path-layout-extension` for package-relative path layout contracts, such as
formatter artifacts under `formatters/` and colorizer artifacts under
`colorizers/`.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-schema.cem`](examples/basic-schema.cem) | Minimal schema definition with content type, element, and attribute declarations. | Pass |
| [`typed-resource-schema.cem`](examples/typed-resource-schema.cem) | Resource schema with `uses`, namespace claims, diagnostics, and open-content policy. | Pass |
| [`invalid-unclosed-schema.cem`](examples/invalid-unclosed-schema.cem) | Missing closing schema scope syntax diagnostic. | Fail with `cem.schema.unclosed_scope` |
| [`invalid-missing-required-attribute.cem`](examples/invalid-missing-required-attribute.cem) | Schema declaration missing its required `namespace` attribute. | Fail with `cem.schema_model.missing_required_attribute` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema+cem \
  --schema https://cem.dev/ns/schema/1 \
  packages/cem_ml/schema-packages/schema/v1/examples/basic-schema.cem
```
