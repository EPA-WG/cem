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
Validation enforces the implementation-specific contract: CEMT converters must
name a CEMT template identity, Rust converters must name a `rust-symbol`, each
converter must have exactly one `from` and `to` endpoint, planner cost must be
positive, and known endpoint schemas must own the declared content type.
Serializer converters may also declare output-contract metadata:
`output-syntax`, `encoding-category`, `formatter-profile`, `color-profile`, and
`parity`. For CEMT schema-output producers, this metadata plans the structured
pipeline as CEMT transform, CEM tree formatting, CEM tree coloring, then final
writer. A missing visual `color-profile` still means a semantic no-color CEM
tree color stage before the writer. Converter-local `parity-fixture` children
name package-relative inputs that paired CEMT/native producers must share, plus
optional input identity and expected diagnostic codes.
Artifact declarations can also describe runtime output-stage assets. For
formatter and colorizer CEMT artifacts, `content-type` and `schema` identify the
artifact source itself, while `target-content-type`, `target-schema`, and
`target-category` identify the CEM tree the artifact formats or colors.
`function-name` identifies the CEMT output function supplied by the asset, and
`function-profile` records the referenced CEMT declaration's own `@profile`
when present. `formatter-profile` or `color-profile` selects the stage profile
when multiple assets can serve the same target.
For local `package.cem` inputs, validation also reads the declared schema
source and checks that the manifest schema URI, content type claims, and
namespace URI claims match the referenced `schema/*.cem` file.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-package.cem`](examples/basic-package.cem) | Minimal `package.cem` manifest with schema, content type, and namespace registration. | Pass |
| [`converter-package.cem`](examples/converter-package.cem) | Package manifest with aliases and a CEMT converter declaration. | Pass |
| [`invalid-unclosed-package.cem`](examples/invalid-unclosed-package.cem) | Missing closing package scope syntax diagnostic. | Fail with `cem.schema.unclosed_scope` |
| [`invalid-missing-required-attribute.cem`](examples/invalid-missing-required-attribute.cem) | Manifest schema entry missing its required `source` attribute. | Fail with `cem.schema_model.missing_required_attribute` |
| [`invalid-converter-contract.cem`](examples/invalid-converter-contract.cem) | Converter declaration with missing CEMT template identity, missing target endpoint, invalid cost, and incompatible endpoint schema/content type. | Fail with `cem.schema_package.converter_template_missing`, `cem.schema_package.converter_endpoint_missing`, `cem.schema_package.converter_cost_invalid`, `cem.schema_package.converter_content_type_mismatch` |
| [`invalid-schema-metadata.cem`](examples/invalid-schema-metadata.cem) | Manifest schema metadata disagrees with the referenced schema source. | Fail with `cem.schema_package.schema_uri_mismatch`, `cem.schema_package.schema_content_type_mismatch`, `cem.schema_package.schema_namespace_mismatch` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema-package+cem \
  --schema https://cem.dev/ns/schema-package/1 \
  packages/cem_ml/schema-packages/schema-package/v1/examples/basic-package.cem
```
