# MathML Schema Package

Status: schema, typed XML lifecycle input/output adapter, examples, formatter,
and colorizer

This package defines registry identity and executable source handling for
standalone MathML resources. MathML source is XML, not CEM-ML syntax; the schema
and package manifest are authored in CEM-ML.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/mathml/1`
- Primary content type: `application/mathml+xml`
- Presentation alias: `application/mathml-presentation+xml`
- Content alias: `application/mathml-content+xml`
- Document namespace: `http://www.w3.org/1998/Math/MathML`
- Preferred extensions: `.mml`, `.mathml`

The primary media type selects the generic mixed profile. The presentation and
content aliases require at least one corresponding expression subtree. An
optional `profile=generic|presentation|content` MIME parameter overrides that
selection; unknown profile values produce a warning and retain the media-type
default.

## Resource Model

Standalone input is represented by `MathMlDocumentAst` over the generic typed
XML event stream. It preserves media type and profile, MIME parameters, XML
declaration and doctype lexemes, qualified element and attribute names,
presentation/content and semantics/annotation boundaries, foreign namespaces,
source ranges, source-map stacks, and detected line endings.

The root must be `math` in the MathML namespace. Presentation and content
expressions may coexist in the generic profile and may be paired through
`semantics` with `annotation` or `annotation-xml` material. MathML embedded in
HTML or XHTML remains part of the containing document lifecycle.

## Parser Facts And Diagnostics

The adapter emits neutral facts for XML parsing and encoding, namespace prefix
binding, attribute uniqueness, root and MathML namespace identity, selected
media profile, presentation/content expression presence, semantics and
annotation boundaries, accessibility text, external annotation references,
foreign content, DTD/entity safety, and source-map availability. Constraints in
`schema/mathml.cem` bind reportable facts to package diagnostics through
`mathml-report-fact`.

The generic XML parser owns well-formedness, encoding, namespaces, duplicate
attributes, DTD/entity detection, lexical events, and source ranges. MathML owns
root/namespace, media-profile, expression, annotation URI, accessibility, and
foreign-content policy. Complete MathML vocabulary, arity, operator, type, and
rendering validation remains outside this first lifecycle contract.

## Output Artifacts

The package owns `compact`, `pretty`, and `tabular` formatter wrappers plus
`terminal`, `html`, and `md` colorizer wrappers. The formatter consumes a
`mathml-document` and emits a package-owned CEM tree; the colorizer consumes
that tree before the shared XML text writer.

All profiles currently preserve source lexemes and MathML element/attribute
case. Their metadata records a `lexical-lossless-*` layout decision while mixed
presentation/content whitespace reflow is deferred. Same-schema output
preserves XML declaration, empty-element spelling, qualified names,
annotations, foreign content, detected line endings, and appends one final
newline when absent.

## Resolver And Entity Safety

The lifecycle registry selects `mathml` for any declared MathML media type, the
package schema URI, or a standalone MathML namespace identity. It does not fall
through to HTML, generic XML, or CEM. Local fragments and `data:` annotation
references remain lexical. Other `src` and `definitionURL` references produce a
policy warning and are never dereferenced without explicit resolver approval.

Foreign annotation namespaces require a registered schema or converter policy.
The inherited XML policy preserves doctype/entity lexemes but rejects DTD
declarations and non-built-in entities without filesystem or network access.

## Validation

Validate MathML resources through the CLI with the schema URI and content type:

```bash
cem-ml validate --format json \
  --content-type application/mathml+xml \
  --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/basic-presentation.mml
```

The direct validator parses MathML as XML, requires a `math` root in the MathML
namespace, recognizes the presentation and content media-type aliases, and
reports external annotation references as policy warnings.

## Verification

`yarn nx run cem_ml_schema_package_mathml_v1:verify` runs manifest validation,
complete example indexing, schema-derived fact tests, dedicated lifecycle
load/export coverage, exact same-schema engine and CLI conversion for all three
media types, executable formatter/colorizer profiles, schema-owned CLI example
validation, and README/SVG preview drift checks without source fallback.

## Release Behavior

Standalone MathML is parsed once into `MathMlDocumentAst` and validated from
schema-owned fact bindings. Same-schema conversion executes the package
formatter, optional colorizer, and XML text writer; metadata identifies
`mathml-lifecycle-output` and
`mathml-ast-stream-to-mathml-output-pipeline`. Cross-schema conversion requires
an explicit registered converter path.

## Tracked Incomplete Work

- Add complete MathML Core and Content MathML vocabulary, expression arity,
  operator, type, and profile conformance.
- Define mixed presentation/content whitespace and reflow semantics before
  formatter profiles alter lexical content.
- Add an explicit resolver capability model before external annotation or
  `definitionURL` resources can be loaded.
- Compose foreign HTML, SVG, OpenMath, and other annotation vocabularies through
  registered schema-package contracts without changing MathML serialization.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the rendered example
content or validation diagnostics for expected-fail examples. The target writes a
preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.

<details>
<summary>basic-presentation</summary>

- Source: [`examples/basic-presentation.mml`](./examples/basic-presentation.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/mathml/v1/examples/basic-presentation.mml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/mathml/v1/examples/basic-presentation.mml,contentType=application/mathml+xml,schema=https://cem.dev/ns/data/mathml/1 \
  --to-content-type application/mathml+xml --to-schema https://cem.dev/ns/data/mathml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of MathML Schema Package basic-presentation example](examples/previews/basic-presentation.mml.svg)

<details>
<summary>content-expression</summary>

- Source: [`examples/content-expression.mathml`](./examples/content-expression.mathml)
- Content type: `application/mathml-content+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/mathml/v1/examples/content-expression.mathml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/mathml/v1/examples/content-expression.mathml,contentType=application/mathml-content+xml,schema=https://cem.dev/ns/data/mathml/1 \
  --to-content-type application/mathml-content+xml --to-schema \
  https://cem.dev/ns/data/mathml/1 --cemt-formatter-profile tabular --cemt-color-profile \
  html
```

</details>

![Preview of MathML Schema Package content-expression example](examples/previews/content-expression.mathml.svg)

<details>
<summary>semantics-external-annotation</summary>

- Source: [`examples/semantics-external-annotation.mml`](./examples/semantics-external-annotation.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `pass`
- Expected diagnostics: `cem.mathml.external_annotation_rejected`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/mathml/v1/examples/semantics-external-annotation.mml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/mathml/v1/examples/semantics-external-annotation.mml,contentType=application/mathml+xml,schema=https://cem.dev/ns/data/mathml/1 \
  --to-content-type application/mathml+xml --to-schema https://cem.dev/ns/data/mathml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of MathML Schema Package semantics-external-annotation example](examples/previews/semantics-external-annotation.mml.svg)

<details>
<summary>invalid-missing-namespace</summary>

- Source: [`examples/invalid-missing-namespace.mml`](./examples/invalid-missing-namespace.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.namespace_missing`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/mathml/v1/examples/invalid-missing-namespace.mml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml+xml --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-missing-namespace.mml
```

</details>

![Preview of MathML Schema Package invalid-missing-namespace example](examples/previews/invalid-missing-namespace.mml.svg)

<details>
<summary>invalid-root-not-math</summary>

- Source: [`examples/invalid-root-not-math.mml`](./examples/invalid-root-not-math.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.root_not_math`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/mathml/v1/examples/invalid-root-not-math.mml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml+xml --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-root-not-math.mml
```

</details>

![Preview of MathML Schema Package invalid-root-not-math example](examples/previews/invalid-root-not-math.mml.svg)

<details>
<summary>invalid-content-profile-presentation-only</summary>

- Source: [`examples/invalid-content-profile-presentation-only.mml`](./examples/invalid-content-profile-presentation-only.mml)
- Content type: `application/mathml-content+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.malformed_expression`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/mathml/v1/examples/invalid-content-profile-presentation-only.mml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml-content+xml --schema \
  https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-content-profile-presentation-only.mml
```

</details>

![Preview of MathML Schema Package invalid-content-profile-presentation-only example](examples/previews/invalid-content-profile-presentation-only.mml.svg)

<details>
<summary>invalid-not-well-formed</summary>

- Source: [`examples/invalid-not-well-formed.mml`](./examples/invalid-not-well-formed.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.not_well_formed_xml`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/mathml/v1/examples/invalid-not-well-formed.mml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml+xml --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-not-well-formed.mml
```

</details>

![Preview of MathML Schema Package invalid-not-well-formed example](examples/previews/invalid-not-well-formed.mml.svg)
