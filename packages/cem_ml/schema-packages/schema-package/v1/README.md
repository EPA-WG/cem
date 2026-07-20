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
Validation enforces the manifest shape and implementation-specific contracts:
the package root must include schema and content-type children, exactly one
content-type child must be marked primary through a schema-declared CEM-ML
behavior function, CEMT converters must name a CEMT template identity, Rust
converters must name a `rust-symbol`, each converter must have exactly one
`from` and `to` endpoint, planner cost is validated by the schema-owned
integer `minInclusive` contract,
`explicit-only=true` cannot be paired with `implicit=true`, and known endpoint
schemas must own the declared content type.
Serializer converters may also declare output-contract metadata:
`output-syntax`, `encoding-category`, `formatter-profile`, `color-profile`, and
`parity`. For CEMT schema-output producers, this metadata plans the structured
pipeline as CEMT transform, CEM tree formatting, CEM tree coloring, then final
writer. A missing visual `color-profile` still means a semantic no-color CEM
tree color stage before the writer. Declaring a formatter or color profile also
requires `output-syntax` and `encoding-category` so the pipeline identity is
complete. When a CEMT converter declares
formatter/coloring output profiles, validation reads the referenced template
through the local package path or template resolver and compiles it as a
formatted CEM-tree producer before writer output is allowed. A CEMT converter
that only declares source/target identity, `output-syntax`, or
`encoding-category` is treated as metadata-only and does not get this executable
template contract check. Converter-local `parity-fixture` children name
package-relative inputs that paired CEMT/native producers must share, plus
optional input identity and expected diagnostic codes.
Artifact declarations can also describe runtime output-stage assets. For
formatter and colorizer CEMT artifacts, `content-type` and `schema` identify the
artifact source itself, while `target-content-type`, `target-schema`, and
`target-category` identify the CEM tree the artifact formats or colors.
`function-name` identifies the CEMT output function supplied by the asset, and
`function-profile` records the referenced CEMT declaration's own `@profile`
when present. `formatter-profile` or `color-profile` selects the stage profile
when multiple assets can serve the same target. Formatter artifacts must use
package-relative `.cemt` paths under `formatters/`; colorizer artifacts must use
package-relative `.cemt` paths under `colorizers/`. These directories sit beside
`schema/` inside the same `schema-packages/{schema-name}/{version}/` hierarchy,
so schema-owned formatting and coloring travel with the schema package instead
of a writer-local string filter.

The current shipped manifest surface remains lexical: artifacts declare
package-relative `path`, lexical `function-name`, optional lexical
`function-profile`, and stage profile selectors. The reference-normalization
target treats those fields as separate domains: `path` resolves through
document/artifact identity, `function-name` remains the authored exported
symbol, compiled CEMT declarations expose function identity records, and
profile fields use dotted profile-symbol semantics. Current validators may
project that structure internally while preserving the existing manifest field
names and diagnostic compatibility.
For local `package.cem` inputs, validation also reads the declared schema
source before registry admission. The first pass is pure declaration
consistency: manifest schema URI, content type claims, and namespace URI claims
must match the referenced `schema/*.cem` file without resolving the package
through the runtime catalog. After those checks pass, the validator may build an
isolated provisional descriptor for the current package and run registry-backed
endpoint, example, artifact, and namespace checks against built-ins plus that
overlay. The provisional descriptor is admitted to a host catalog only after all
required checks pass.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests. Representative failing examples assert stable
structured diagnostic details, including `behavior`, `checkKind`, `contract`,
severity, and source range data for the engine behavior contract they exercise.

| Example                                                                                           | Purpose                                                                                                                                                         | Expected result                                                                                                                                       |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`basic-package.cem`](examples/basic-package.cem)                                                 | Minimal `package.cem` manifest with schema, content type, and namespace registration.                                                                           | Pass                                                                                                                                                  |
| [`converter-package.cem`](examples/converter-package.cem)                                         | Package manifest with aliases and a CEMT converter declaration.                                                                                                 | Pass                                                                                                                                                  |
| [`invalid-unclosed-package.cem`](examples/invalid-unclosed-package.cem)                           | Missing closing package scope syntax diagnostic.                                                                                                                | Fail with `cem.schema.unclosed_scope`                                                                                                                 |
| [`invalid-missing-required-attribute.cem`](examples/invalid-missing-required-attribute.cem)       | Manifest schema entry missing its required `source` attribute and package root missing its required content-type child.                                         | Fail with `cem.schema_model.missing_required_attribute`, `cem.schema_package.package_check`                                                           |
| [`invalid-primary-content-type.cem`](examples/invalid-primary-content-type.cem)                   | Manifest declares more than one primary content type; the package-level diagnostic is produced by a schema-declared CEM-ML behavior function.                   | Fail with `cem.schema_package.content_type_conflict`                                                                                                  |
| [`invalid-primary-content-type-missing.cem`](examples/invalid-primary-content-type-missing.cem)   | Manifest declares content types but no primary content type; the same schema-declared CEM-ML behavior function enforces exact-one primary cardinality.          | Fail with `cem.schema_package.content_type_conflict`                                                                                                  |
| [`invalid-converter-contract.cem`](examples/invalid-converter-contract.cem)                       | Converter declaration with missing CEMT template identity, missing target endpoint, invalid cost, and incompatible endpoint schema/content type.                | Fail with `cem.schema_package.converter_check`, `cem.schema_model.invalid_attribute_datatype_param`                                                   |
| [`invalid-converter-runtime-constraints.cem`](examples/invalid-converter-runtime-constraints.cem) | Converter declarations with unknown implementation, invalid planner metadata, missing native fallback reason, invalid output metadata, and missing Rust symbol. | Fail with `cem.schema_model.invalid_attribute_type`, `cem.schema_model.invalid_attribute_value`, `cem.schema_package.converter_check`                 |
| [`invalid-converter-template-unreadable.cem`](examples/invalid-converter-template-unreadable.cem) | CEMT converter declares formatter/coloring output profiles, but its template path does not resolve to a readable source.                                        | Fail with `cem.schema_package.converter_check`                                                                                                        |
| [`invalid-converter-template-contract.cem`](examples/invalid-converter-template-contract.cem)     | CEMT converter declares formatter/coloring output profiles, but its template cannot compile as a formatted CEM-tree producer before writer output.              | Fail with `cem.schema_package.converter_check`                                                                                                        |
| [`invalid-artifact-contract.cem`](examples/invalid-artifact-contract.cem)                         | Formatter artifact metadata disagrees with the referenced CEMT function declaration.                                                                            | Fail with `cem.schema_package.artifact_check`                                                                                                         |
| [`invalid-artifact-layout.cem`](examples/invalid-artifact-layout.cem)                             | Formatter and colorizer artifacts point outside their schema-package stage directories.                                                                         | Fail with `cem.schema_package.artifact_check`                                                                                                         |
| [`invalid-artifact-source-unreadable.cem`](examples/invalid-artifact-source-unreadable.cem)       | Formatter artifact references a missing CEMT source file.                                                                                                       | Fail with `cem.schema_package.artifact_check`                                                                                                         |
| [`invalid-artifact-source-parse.cem`](examples/invalid-artifact-source-parse.cem)                 | Formatter artifact references a CEMT source file that cannot be parsed.                                                                                         | Fail with `cem.schema_package.artifact_check`                                                                                                         |
| [`invalid-artifact-function-missing.cem`](examples/invalid-artifact-function-missing.cem)         | Formatter artifact references a CEMT source file that does not declare the requested output function.                                                           | Fail with `cem.schema_package.artifact_check`                                                                                                         |
| [`invalid-schema-metadata.cem`](examples/invalid-schema-metadata.cem)                             | Manifest schema metadata disagrees with the referenced schema source.                                                                                           | Fail with `cem.schema_package.schema_uri_mismatch`, `cem.schema_package.schema_content_type_mismatch`, `cem.schema_package.schema_namespace_mismatch` |
| [`invalid-example-contract.cem`](examples/invalid-example-contract.cem)                           | Example metadata has an invalid expected result, incompatible schema/content type, and a failing example without expected diagnostics.                          | Fail with `cem.schema_model.invalid_attribute_value`, `cem.schema_package.example_check`                                                              |
| [`invalid-example-source-contract.cem`](examples/invalid-example-source-contract.cem)             | Example declarations cover unreadable source files, source validation result mismatches, and expected diagnostic mismatches.                                    | Fail with `cem.schema_package.example_check`                                                                                                          |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema-package+cem \
  --schema https://cem.dev/ns/schema-package/1 \
  packages/cem_ml/schema-packages/schema-package/v1/examples/basic-package.cem
```
