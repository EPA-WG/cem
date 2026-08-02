# SVG Resource Schema Package

Status: schema, typed XML lifecycle input/output adapter, examples, formatter,
and colorizer

This package defines registry identity and executable source handling for SVG
resources.

SVG source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `image/svg+xml` content type are
parsed as XML and validated as SVG vocabulary in the SVG namespace.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/svg/1`
- Primary content type: `image/svg+xml`
- Document namespace: `http://www.w3.org/2000/svg`
- Preferred extensions: `.svg`, `.svgz`

IANA registers `image/svg+xml` with optional `charset`. The `.svgz` extension is
treated as gzip-compressed SVG content with the same media type, not a separate
schema identity.

## Resource Model

The schema describes SVG resources as an XML-based graphics document model:

- documents reuse the generic XML schema and preserve charset, compression
  disposition, source identity, and byte offsets;
- the root element must be `svg` in the SVG namespace;
- viewport, geometry, paint, text, definition, filter, animation, style, and
  script nodes are modeled explicitly enough for validation and conversion;
- visible SVG accessibility hooks are explicit through title, desc, role, and
  ARIA name material;
- external resources, scripts, CSS, and foreign content require explicit
  policy before dereferencing or execution;
- foreign content must be handled by a registered schema package or converter
  profile.

Standalone SVG input is represented by `SvgDocumentAst`. It reuses the generic
typed XML event stream while preserving SVG content type/schema identity, MIME
parameters, XML declarations and doctypes, qualified element and attribute
names, XLink attributes, foreign-content boundaries, source ranges, source-map
stacks, and detected line endings. Embedded `<svg>` inside an HTML or XHTML
document remains part of that containing document's lifecycle.

## Parser Facts And Diagnostics

The adapter emits neutral facts for XML parse and encoding errors, namespace
binding and attribute uniqueness, root and SVG namespace identity, `viewBox`
shape, title and ARIA accessibility material, references, external resources,
scripts and event handlers, foreign content, DTD/entity safety, and source-map
availability. Constraints in `schema/svg.cem` bind reportable facts to
package-owned diagnostics through `svg-report-fact`; observed lexical structure
remains available as non-diagnostic facts.

The generic XML parser owns well-formedness, encoding, qualified-name,
namespace, duplicate-attribute, DTD, entity, and source-range mechanics. SVG
owns root/namespace, viewport, accessibility, URI, executable content, and
foreign-content policy. The current validator does not perform complete SVG
vocabulary, geometry, CSS, animation, filter, or path-data validation.

## Output Artifacts

The package owns `compact`, `pretty`, and `tabular` formatter wrappers plus
`terminal`, `html`, and `md` colorizer wrappers. The formatter consumes an
`svg-document` and emits a package-owned CEM tree; the colorizer consumes that
tree before the shared text writer.

The `compact` profile removes typed structural whitespace, `pretty` emits one
structural event per indented line, and `tabular` additionally aligns attributes
on lines one level below their element. Layout remains lexical inside mixed-text
elements, SVG text/style/script content, `foreignObject`, `xml:space="preserve"`
scopes, CDATA, and foreign namespaces. Same-schema output retains XML
declarations, doctype and entity lexemes, empty-element spelling, qualified
names, attribute case and quote style, XLink attributes, detected line endings,
and appends one final newline when absent.

Start and end markup is projected as delimiter, element-name, attribute-name,
equals, and attribute-value writer tokens. Generated indentation and line
endings remain unmapped; lexical tokens retain token-level source maps and are
rebased to output spans by the writer. Terminal, HTML, and Markdown color
profiles consume the same token roles and preserve identical visible SVG text.

## Resolver And Script Safety

The lifecycle registry selects the `svg` adapter for `image/svg+xml`,
`https://cem.dev/ns/data/svg/1`, or a standalone
`http://www.w3.org/2000/svg` document identity. It does not fall through to the
HTML, generic XML, or CEM adapters. Same-schema output uses XML serialization
and package-owned SVG CEMT assets; it never applies HTML parsing or
serialization behavior.

Local fragment and `data:` references are preserved without dereferencing.
Other `href`, `xlink:href`, `src`, and CSS `url(...)` references are rejected
until an explicit resolver policy exists. Script elements and event-handler
attributes are rejected until an execution policy exists. Foreign namespaces
require an explicit registered schema or converter policy. The inherited XML
policy preserves doctype/entity lexemes but rejects DTD declarations and
non-built-in entities without resolving filesystem or network resources.

## Validation

Validate SVG resources through the CLI with the schema URI and content type:

```bash
cem-ml validate --format json \
  --content-type image/svg+xml \
  --schema https://cem.dev/ns/data/svg/1 \
  packages/cem_ml/schema-packages/svg/v1/examples/basic-icon.svg
```

## Verification

`yarn nx run cem_ml_schema_package_svg_v1:verify` runs manifest validation,
complete example indexing, schema-derived fact tests, dedicated lifecycle
load/export coverage, exact same-schema engine and CLI conversion, executable
formatter/colorizer profile tests, schema-owned CLI example validation, and
README source-fence generation checks with no SVG fallback.

## Release Behavior

Standalone SVG input is parsed once into `SvgDocumentAst` through the generic
XML event model and validated from SVG schema fact bindings. Same-schema
conversion executes the package formatter, optional colorizer, and XML text
writer; output metadata identifies `svg-lifecycle-output` and
`svg-ast-stream-to-svg-output-pipeline`. Cross-schema conversion requires an
explicit registered converter path.

## Tracked Incomplete Work

- Add complete SVG vocabulary, geometry, path-data, CSS, animation, filter, and
  paint validation independently of XML well-formedness.
- Add explicit bounded DTD/entity, URI resolver, script, and external-resource
  policies before permitting resource access or execution.
- Compose foreign HTML, XHTML, MathML, and other namespace validation through
  explicit schema-package contracts without changing SVG serialization.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-icon</summary>

- Source: [`examples/basic-icon.svg`](./examples/basic-icon.svg)
- Content type: `image/svg+xml`
- Schema: `https://cem.dev/ns/data/svg/1`
- Expected result: `pass`
- README rendering: fenced `svg` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/svg/v1/examples/basic-icon.svg,contentType=image/svg+xml,schema=https://cem.dev/ns/data/svg/1 \
  --to-content-type image/svg+xml --to-schema https://cem.dev/ns/data/svg/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```svg
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" role="img" viewBox="0 0 24 24" width="24" height="24">
  <title>Download</title>
  <path d="M12 3v12"/>
  <path d="M7 10l5 5 5-5"/>
  <path d="M5 21h14"/>
</svg>
```

<details>
<summary>bar-chart</summary>

- Source: [`examples/bar-chart.svg`](./examples/bar-chart.svg)
- Content type: `image/svg+xml`
- Schema: `https://cem.dev/ns/data/svg/1`
- Expected result: `pass`
- README rendering: fenced `svg` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/svg/v1/examples/bar-chart.svg,contentType=image/svg+xml,schema=https://cem.dev/ns/data/svg/1 \
  --to-content-type image/svg+xml --to-schema https://cem.dev/ns/data/svg/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```svg
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" role="img" viewBox="0 0 320 180" width="320" height="180">
  <title>Quarterly Revenue</title>
  <desc>Three bars compare revenue for the first three quarters.</desc>
  <defs>
    <linearGradient id="bar-fill" x1="0" x2="0" y1="0" y2="1">
      <stop offset="0" stop-color="#4f7cff"/>
      <stop offset="1" stop-color="#1b4fd7"/>
    </linearGradient>
  </defs>
  <rect x="48" y="82" width="48" height="58" fill="url(#bar-fill)"/>
  <rect x="136" y="48" width="48" height="92" fill="url(#bar-fill)"/>
  <rect x="224" y="28" width="48" height="112" fill="url(#bar-fill)"/>
  <text x="48" y="164">Q1</text>
  <text x="136" y="164">Q2</text>
  <text x="224" y="164">Q3</text>
</svg>
```

<details>
<summary>unnamed-icon</summary>

- Source: [`examples/unnamed-icon.svg`](./examples/unnamed-icon.svg)
- Content type: `image/svg+xml`
- Schema: `https://cem.dev/ns/data/svg/1`
- Expected result: `pass`
- Expected diagnostics: `cem.svg.accessible_name_missing`
- README rendering: fenced `svg` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/svg/v1/examples/unnamed-icon.svg,contentType=image/svg+xml,schema=https://cem.dev/ns/data/svg/1 \
  --to-content-type image/svg+xml --to-schema https://cem.dev/ns/data/svg/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```svg
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
  <path d="M12 3v18"/>
  <path d="M3 12h18"/>
</svg>
```

<details>
<summary>invalid-missing-namespace</summary>

- Source: [`examples/invalid-missing-namespace.svg`](./examples/invalid-missing-namespace.svg)
- Content type: `image/svg+xml`
- Schema: `https://cem.dev/ns/data/svg/1`
- Expected result: `fail`
- Expected diagnostics: `cem.svg.namespace_missing`
- README rendering: fenced `svg` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type image/svg+xml --schema https://cem.dev/ns/data/svg/1 \
  packages/cem_ml/schema-packages/svg/v1/examples/invalid-missing-namespace.svg
```

</details>

```svg
<?xml version="1.0" encoding="UTF-8"?>
<svg role="img" viewBox="0 0 24 24" width="24" height="24">
  <title>Missing Namespace</title>
  <path d="M4 12h16"/>
</svg>
```

<details>
<summary>invalid-script</summary>

- Source: [`examples/invalid-script.svg`](./examples/invalid-script.svg)
- Content type: `image/svg+xml`
- Schema: `https://cem.dev/ns/data/svg/1`
- Expected result: `fail`
- Expected diagnostics: `cem.svg.script_rejected`
- README rendering: fenced `svg` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type image/svg+xml --schema https://cem.dev/ns/data/svg/1 \
  packages/cem_ml/schema-packages/svg/v1/examples/invalid-script.svg
```

</details>

```svg
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" role="img" viewBox="0 0 24 24">
  <title>Scripted SVG</title>
  <script>alert("blocked")</script>
</svg>
```

<details>
<summary>invalid-external-image</summary>

- Source: [`examples/invalid-external-image.svg`](./examples/invalid-external-image.svg)
- Content type: `image/svg+xml`
- Schema: `https://cem.dev/ns/data/svg/1`
- Expected result: `fail`
- Expected diagnostics: `cem.svg.external_resource_rejected`
- README rendering: fenced `svg` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type image/svg+xml --schema https://cem.dev/ns/data/svg/1 \
  packages/cem_ml/schema-packages/svg/v1/examples/invalid-external-image.svg
```

</details>

```svg
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" role="img" viewBox="0 0 64 64">
  <title>External Image</title>
  <image href="https://example.test/logo.png" width="64" height="64"/>
</svg>
```
