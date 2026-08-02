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

The profiles preserve MathML element and attribute case while applying distinct
deterministic structural layouts. `compact` removes structural whitespace,
`pretty` places structural events on depth-indented lines, and `tabular` also
places each attribute on its own continuation line. Mathematical token elements
(`mi`, `mn`, `mo`, `mtext`, and `ms`), annotation payloads, CDATA, direct mixed
text, `xml:space="preserve"`, and foreign namespaces remain lexical islands.

Start and end tags are projected into mapped delimiter, element-name,
attribute-name, equals, and attribute-value tokens. Generated indentation and
line endings remain unmapped. Same-schema output preserves XML declaration,
empty-element spelling, qualified names, sensitive annotations and foreign
content, honors configured indentation and line endings, and appends one final
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
validation, and README source-fence generation checks without SVG fallback.

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
- Add an explicit resolver capability model before external annotation or
  `definitionURL` resources can be loaded.
- Compose foreign HTML, SVG, OpenMath, and other annotation vocabularies through
  registered schema-package contracts without changing MathML serialization.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-presentation</summary>

- Source: [`examples/basic-presentation.mml`](./examples/basic-presentation.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `pass`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/mathml/v1/examples/basic-presentation.mml,contentType=application/mathml+xml,schema=https://cem.dev/ns/data/mathml/1 \
  --to-content-type application/mathml+xml --to-schema https://cem.dev/ns/data/mathml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<math xmlns="http://www.w3.org/1998/Math/MathML" display="inline" alttext="x plus one">
  <mrow>
    <mi>x</mi>
    <mo>+</mo>
    <mn>1</mn>
  </mrow>
</math>
```

<details>
<summary>content-expression</summary>

- Source: [`examples/content-expression.mathml`](./examples/content-expression.mathml)
- Content type: `application/mathml-content+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `pass`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/mathml/v1/examples/content-expression.mathml,contentType=application/mathml-content+xml,schema=https://cem.dev/ns/data/mathml/1 \
  --to-content-type application/mathml-content+xml --to-schema \
  https://cem.dev/ns/data/mathml/1 --cemt-formatter-profile tabular --cemt-color-profile \
  html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<math xmlns="http://www.w3.org/1998/Math/MathML" display="block">
  <apply>
    <plus/>
    <ci>x</ci>
    <cn>1</cn>
  </apply>
</math>
```

<details>
<summary>semantics-external-annotation</summary>

- Source: [`examples/semantics-external-annotation.mml`](./examples/semantics-external-annotation.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `pass`
- Expected diagnostics: `cem.mathml.external_annotation_rejected`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/mathml/v1/examples/semantics-external-annotation.mml,contentType=application/mathml+xml,schema=https://cem.dev/ns/data/mathml/1 \
  --to-content-type application/mathml+xml --to-schema https://cem.dev/ns/data/mathml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<math xmlns="http://www.w3.org/1998/Math/MathML" alttext="x squared">
  <semantics>
    <msup>
      <mi>x</mi>
      <mn>2</mn>
    </msup>
    <annotation encoding="application/json" src="formula.json"/>
  </semantics>
</math>
```

<details>
<summary>invalid-missing-namespace</summary>

- Source: [`examples/invalid-missing-namespace.mml`](./examples/invalid-missing-namespace.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.namespace_missing`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml+xml --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-missing-namespace.mml
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<math display="inline">
  <mi>x</mi>
</math>
```

<details>
<summary>invalid-root-not-math</summary>

- Source: [`examples/invalid-root-not-math.mml`](./examples/invalid-root-not-math.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.root_not_math`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml+xml --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-root-not-math.mml
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<mrow xmlns="http://www.w3.org/1998/Math/MathML">
  <mi>x</mi>
</mrow>
```

<details>
<summary>invalid-content-profile-presentation-only</summary>

- Source: [`examples/invalid-content-profile-presentation-only.mml`](./examples/invalid-content-profile-presentation-only.mml)
- Content type: `application/mathml-content+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.malformed_expression`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml-content+xml --schema \
  https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-content-profile-presentation-only.mml
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<math xmlns="http://www.w3.org/1998/Math/MathML">
  <mrow>
    <mi>x</mi>
    <mo>+</mo>
    <mn>1</mn>
  </mrow>
</math>
```

<details>
<summary>invalid-not-well-formed</summary>

- Source: [`examples/invalid-not-well-formed.mml`](./examples/invalid-not-well-formed.mml)
- Content type: `application/mathml+xml`
- Schema: `https://cem.dev/ns/data/mathml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.mathml.not_well_formed_xml`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/mathml+xml --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/invalid-not-well-formed.mml
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<math xmlns="http://www.w3.org/1998/Math/MathML">
  <mrow>
    <mi>x</mrow>
</math>
```
