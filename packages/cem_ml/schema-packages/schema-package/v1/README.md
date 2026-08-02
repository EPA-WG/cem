# CEM Schema Package Metadata Package

Status: current implemented surface for the schema-package manifest package.
Reference-normalization target design lives in
[`../../../../../docs/cem-ml-reference-normalization-design.md`](../../../../../docs/cem-ml-reference-normalization-design.md),
with lookup and comparison vocabulary in
[`../../../../../docs/cem-ml-reference-vocabulary-design.md`](../../../../../docs/cem-ml-reference-vocabulary-design.md)
and
[`../../../../../docs/cem-ml-reference-comparison-design.md`](../../../../../docs/cem-ml-reference-comparison-design.md).

This package defines `package.cem`, the metadata manifest found at:

```text
schema-packages/{schema-name}/{version}/package.cem
```

Owned schema URI:

```text
https://cem.dev/ns/schema-package/1
```

Schema source:

```text
schema/schema-package.cem
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
The target declarative check sequence keeps source readability and CEMT parse
validity in explicit resource behaviors, then uses read-only
`schema:cemt-output-function` inspection to select output declaration metadata
by resolved artifact identity plus lexical function name. The selected
declaration is normalized as `schema:function-identity`; artifact contract
checks compare its kind, target content type, target schema, target category,
optional profile, and subject metadata against manifest fields without
executing the CEMT function body.

Compatibility projection is part of the migration contract. Current CLI/report
output may keep existing diagnostic codes and broad value buckets while
structured metadata records target operand bindings, lookup key provenance,
normalized values, and per-item or per-operand reasons. The compatibility
projection must not collapse schema identity to URI-only equality or treat
namespace claims as schema identity.

For local `package.cem` inputs, validation also reads the declared schema
source before registry admission. The first pass is pure declaration
consistency: manifest schema URI, content type claims, and namespace URI claims
must match the referenced `schema/*.cem` file without resolving the package
through the runtime catalog. After those checks pass, the validator may build an
isolated provisional descriptor for the current package and run registry-backed
endpoint, example, artifact, and namespace checks against built-ins plus that
overlay. The provisional descriptor records complete schema identity, package
id/version, manifest and schema source artifact identity, declared content-type
and namespace claims, descriptor origin, registry layer, match rule, and source
ranges when available. It is admitted to a host catalog only after all required
checks pass.

## Folder Contract

`package.cem` is the manifest-owned index for this folder. It declares the
schema URI and source file, the primary content type, namespace claims, schema
package manifest constraints, and every validation example under `examples/`.

`project.json` owns the package-local Nx library
`cem_ml_schema_package_schema_package_v1`. Its `verify` target validates
`package.cem` through the CLI at the parse failure boundary and tracks
`README.md`, `schema/**/*.cem`, `formatters/**/*.cemt`,
`colorizers/**/*.cemt`, `converters/**/*.cemt`, and `examples/**/*` as package
inputs. Full semantic schema-package validation remains in the final
registry/package gate.

Example metadata is intentionally manifest-owned. This package does not require
checked-in `.example.cem` sidecars because `package.cem` already records the
example path, content type, schema URI, expected pass/fail result, and expected
diagnostic codes.

## CEMT Output Status

The schema-package metadata package currently declares no runtime converter
edges in its own manifest and no package-owned formatter or colorizer CEMT
artifacts. The schema-package structure audit therefore reports the baseline
formatter/colorizer profiles as alignment gaps, not hard errors.

CEMT files under `examples/converters/`, `examples/formatters/`, and
`examples/transforms/` are validation fixtures, not registered package output
assets. They exercise converter and artifact contract failures for package
metadata validation. Until schema-package-specific formatter and colorizer
assets are authored, schema-package examples rely on the generic CEM-ML output
path rather than a schema-package-specific output pipeline.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-package</summary>

- Source: [`examples/basic-package.cem`](./examples/basic-package.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/basic-package.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="note" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}

    {namespace @prefix="note" @uri="https://example.test/ns/note/1"}
}
```

<details>
<summary>converter-package</summary>

- Source: [`examples/converter-package.cem`](./examples/converter-package.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/converter-package.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="note-html" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note-html/1"
        @source="schema/note-html.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}
    {content-type @value="text/html" @alias=true}

    {namespace @prefix="note" @uri="https://example.test/ns/note-html/1"}

    {converter
        @id="note-to-html"
        @implementation="cemt"
        @template="templates/note-to-html.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1"
        @streamable=true
        @lossiness="lossless"
        @output-syntax="html"
        @encoding-category="html-document"
        @parity="parse-equivalent"
        @cost=100 |
        {from @content-type="application/vnd.example.note+cem" @schema="https://example.test/ns/note-html/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}
```

<details>
<summary>invalid-unclosed-package</summary>

- Source: [`examples/invalid-unclosed-package.cem`](./examples/invalid-unclosed-package.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.ast.unclosed_scope`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-unclosed-package.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="broken" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/broken/1"
        @source="schema/broken.cem"
```

<details>
<summary>invalid-missing-required-attribute</summary>

- Source: [`examples/invalid-missing-required-attribute.cem`](./examples/invalid-missing-required-attribute.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_model.missing_required_attribute`, `cem.schema_package.package_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-missing-required-attribute.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="broken" @version="1.0.0" |
    {schema @uri="https://example.test/ns/broken/1"}
}
```

<details>
<summary>invalid-primary-content-type</summary>

- Source: [`examples/invalid-primary-content-type.cem`](./examples/invalid-primary-content-type.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.content_type_conflict`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-primary-content-type.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="broken-primary" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/broken-primary/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.broken-primary+cem" @primary=true}
    {content-type @value="application/vnd.example.broken-primary-alt+cem" @primary=true}

    {namespace @prefix="broken" @uri="https://example.test/ns/broken-primary/1"}
}
```

<details>
<summary>invalid-primary-content-type-missing</summary>

- Source: [`examples/invalid-primary-content-type-missing.cem`](./examples/invalid-primary-content-type-missing.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.content_type_conflict`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-primary-content-type-missing.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="missing-primary" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/missing-primary/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.missing-primary+cem" @alias=true}
    {content-type @value="application/vnd.example.missing-primary-secondary+cem" @primary=false}

    {namespace @prefix="missing" @uri="https://example.test/ns/missing-primary/1"}
}
```

<details>
<summary>invalid-converter-contract</summary>

- Source: [`examples/invalid-converter-contract.cem`](./examples/invalid-converter-contract.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.converter_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-contract.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-converter" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/bad-converter/1"
        @source="schema/bad-converter.cem"
    }

    {content-type @value="application/vnd.example.bad-converter+cem" @primary=true}

    {converter
        @id="bad-to-html"
        @implementation="cemt"
        @template-content-type="text/cem-ml"
        @cost=0 |
        {from @content-type="text/html" @schema="https://cem.dev/ns/data/xml/1"}
    }
}
```

<details>
<summary>invalid-converter-runtime-constraints</summary>

- Source: [`examples/invalid-converter-runtime-constraints.cem`](./examples/invalid-converter-runtime-constraints.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.converter_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-runtime-constraints.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-converter-runtime-constraints" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}

    {namespace @prefix="note" @uri="https://example.test/ns/note/1"}

    {converter
        @id="unknown-implementation"
        @implementation="python" |
        {from @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }

    {converter
        @id="bad-cemt-planner-state"
        @implementation="cemt"
        @template="converters/missing.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://example.test/ns/not-cemt/1"
        @rust-symbol="fallback_bad_cemt_planner_state"
        @streamable="sometimes"
        @implicit=true
        @explicit-only=true
        @readiness="later"
        @lossiness="hand-wave"
        @output-syntax="pixels"
        @formatter-profile="compact"
        @parity="same-enough"
        @cost=1 |
        {from @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }

    {converter
        @id="rust-without-symbol"
        @implementation="rust" |
        {from @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }

    {converter
        @id="rust-with-template"
        @implementation="rust"
        @rust-symbol="convert_rust_with_template"
        @template="converters/unexpected.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1"
        @template-entrypoint="main" |
        {from @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}
```

<details>
<summary>invalid-converter-template-contract</summary>

- Source: [`examples/invalid-converter-template-contract.cem`](./examples/invalid-converter-template-contract.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.converter_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-template-contract.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-converter-template" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note-html/1"
        @source="schema/note-html.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}
    {content-type @value="text/html" @alias=true}

    {namespace @prefix="note" @uri="https://example.test/ns/note-html/1"}

    {converter
        @id="note-to-html-bad-template"
        @implementation="cemt"
        @template="converters/invalid-output-pipeline.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1"
        @template-entrypoint="main"
        @streamable=true
        @lossiness="lossless"
        @output-syntax="html"
        @encoding-category="html-document"
        @formatter-profile="compact"
        @color-profile="classes"
        @parity="parse-equivalent"
        @cost=100 |
        {from @content-type="application/vnd.example.note+cem" @schema="https://example.test/ns/note-html/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}
```

<details>
<summary>invalid-converter-template-unreadable</summary>

- Source: [`examples/invalid-converter-template-unreadable.cem`](./examples/invalid-converter-template-unreadable.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.converter_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-template-unreadable.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-converter-template-source" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note-html/1"
        @source="schema/note-html.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}
    {content-type @value="text/html" @alias=true}

    {namespace @prefix="note" @uri="https://example.test/ns/note-html/1"}

    {converter
        @id="note-to-html-missing-template"
        @implementation="cemt"
        @template="converters/missing-output-pipeline.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1"
        @template-entrypoint="main"
        @streamable=true
        @lossiness="lossless"
        @output-syntax="html"
        @encoding-category="html-document"
        @formatter-profile="compact"
        @color-profile="classes"
        @parity="parse-equivalent"
        @cost=100 |
        {from @content-type="application/vnd.example.note+cem" @schema="https://example.test/ns/note-html/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}
```

<details>
<summary>invalid-artifact-contract</summary>

- Source: [`examples/invalid-artifact-contract.cem`](./examples/invalid-artifact-contract.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.artifact_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-contract.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-artifact" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/bad-artifact/1"
        @source="schema/bad-artifact.cem"
    }

    {content-type @value="application/vnd.example.bad-artifact+cem" @primary=true}

    {artifact
        @kind="formatter"
        @path="formatters/invalid-artifact-contract.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="wrong-tree"
        @function-name="bad.format"
        @formatter-profile="compact"
    }
}
```

<details>
<summary>invalid-artifact-layout</summary>

- Source: [`examples/invalid-artifact-layout.cem`](./examples/invalid-artifact-layout.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.artifact_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-layout.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="note-bad-artifact-layout" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}

    {namespace @prefix="note" @uri="https://example.test/ns/note/1"}

    {artifact
        @kind="formatter"
        @path="transforms/invalid-artifact-layout-format.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="bad.layout.format"
        @formatter-profile="compact"
    }

    {artifact
        @kind="colorizer"
        @path="formatters/invalid-artifact-layout-color.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="bad.layout.color"
        @function-profile="classes"
        @color-profile="classes"
    }
}
```

<details>
<summary>invalid-artifact-source-unreadable</summary>

- Source: [`examples/invalid-artifact-source-unreadable.cem`](./examples/invalid-artifact-source-unreadable.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.artifact_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-source-unreadable.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-artifact-source-unreadable" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}

    {artifact
        @kind="formatter"
        @path="formatters/missing.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="bad.missing"
        @formatter-profile="compact"
    }
}
```

<details>
<summary>invalid-artifact-source-parse</summary>

- Source: [`examples/invalid-artifact-source-parse.cem`](./examples/invalid-artifact-source-parse.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.artifact_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-source-parse.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-artifact-source-parse" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}

    {artifact
        @kind="formatter"
        @path="formatters/invalid-artifact-source-parse.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="bad.invalid"
        @formatter-profile="compact"
    }
}
```

<details>
<summary>invalid-artifact-function-missing</summary>

- Source: [`examples/invalid-artifact-function-missing.cem`](./examples/invalid-artifact-function-missing.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.artifact_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-function-missing.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-artifact-function-missing" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}

    {artifact
        @kind="formatter"
        @path="formatters/missing-function.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="bad.missing"
        @formatter-profile="compact"
    }
}
```

<details>
<summary>invalid-schema-metadata</summary>

- Source: [`examples/invalid-schema-metadata.cem`](./examples/invalid-schema-metadata.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.schema_uri_mismatch`, `cem.schema_package.schema_content_type_mismatch`, `cem.schema_package.schema_namespace_mismatch`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-metadata.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="broken-schema" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/broken-schema/1"
        @source="schema/invalid-schema-metadata.cem"
    }

    {content-type @value="application/vnd.example.broken+cem" @primary=true}

    {namespace @prefix="broken" @uri="https://example.test/ns/broken-schema/1"}
}
```

<details>
<summary>invalid-schema-source-unreadable</summary>

- Source: [`examples/invalid-schema-source-unreadable.cem`](./examples/invalid-schema-source-unreadable.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.schema_source_unreadable`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-source-unreadable.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="broken-schema-source-unreadable" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/broken-schema-source-unreadable/1"
        @source="schema/missing-schema-source.cem"
    }

    {content-type @value="application/vnd.example.broken-schema-source-unreadable+cem" @primary=true}

    {namespace @prefix="broken" @uri="https://example.test/ns/broken-schema-source-unreadable/1"}
}
```

<details>
<summary>invalid-schema-source-invalid</summary>

- Source: [`examples/invalid-schema-source-invalid.cem`](./examples/invalid-schema-source-invalid.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.schema_source_invalid`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-source-invalid.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="broken-schema-source-invalid" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/broken-schema-source-invalid/1"
        @source="schema/invalid-schema-source.cem"
    }

    {content-type @value="application/vnd.example.broken-schema-source-invalid+cem" @primary=true}

    {namespace @prefix="broken" @uri="https://example.test/ns/broken-schema-source-invalid/1"}
}
```

<details>
<summary>invalid-example-contract</summary>

- Source: [`examples/invalid-example-contract.cem`](./examples/invalid-example-contract.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.example_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-example-contract.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-examples" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/bad-examples/1"
        @source="schema/bad-examples.cem"
    }

    {content-type @value="application/vnd.example.bad-examples+cem" @primary=true}

    {example
        @id="wrong-result"
        @path="examples/wrong-result.html"
        @content-type="text/html"
        @schema="https://cem.dev/ns/data/html/1"
        @expected-result="maybe"
    }

    {example
        @id="wrong-content-type"
        @path="examples/wrong-content-type.html"
        @content-type="text/html"
        @schema="https://cem.dev/ns/data/xml/1"
        @expected-result="pass"
    }

    {example
        @id="missing-diagnostics"
        @path="examples/missing-diagnostics.html"
        @content-type="text/html"
        @schema="https://cem.dev/ns/data/html/1"
        @expected-result="fail"
    }
}
```

<details>
<summary>invalid-example-source-contract</summary>

- Source: [`examples/invalid-example-source-contract.cem`](./examples/invalid-example-source-contract.cem)
- Content type: `application/vnd.cem.schema-package+cem`
- Schema: `https://cem.dev/ns/schema-package/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_package.example_check`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-example-source-contract.cem,contentType=application/vnd.cem.schema-package+cem,schema=https://cem.dev/ns/schema-package/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="bad-example-source" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}

    {example
        @id="missing-source"
        @path="schema/missing-example-source.cem"
        @content-type="application/vnd.cem.schema-package+cem"
        @schema="https://cem.dev/ns/schema-package/1"
        @expected-result="pass"
    }

    {example
        @id="expected-pass-but-invalid"
        @path="schema/invalid-example-source.cem"
        @content-type="application/vnd.cem.schema-package+cem"
        @schema="https://cem.dev/ns/schema-package/1"
        @expected-result="pass"
    }

    {example
        @id="wrong-expected-diagnostic"
        @path="schema/invalid-example-source.cem"
        @content-type="application/vnd.cem.schema-package+cem"
        @schema="https://cem.dev/ns/schema-package/1"
        @expected-result="fail"
        @expected-diagnostics="cem.schema_model.invalid_child_element"
    }
}
```
