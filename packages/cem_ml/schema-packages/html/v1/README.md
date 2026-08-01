# HTML schema package v1

Status: typed lifecycle, schema-bound validation, and executable output profiles

This package defines the CEM schema identity for HTML resources.

- Schema URI: `https://cem.dev/ns/data/html/1`
- Primary content type: `text/html`
- DOM namespaces:
    - HTML: `http://www.w3.org/1999/xhtml`
    - SVG: `http://www.w3.org/2000/svg`
    - MathML: `http://www.w3.org/1998/Math/MathML`
- Source schema: `schema/html.cem`

HTML is not XML. The package loads `text/html` into a dedicated typed event AST.
It preserves lexical tag and attribute spelling, comments, doctype evidence,
raw-text and RCDATA bodies, MIME parameters, source ranges, source maps, and
document-versus-fragment mode. Recoverable mismatched nesting remains HTML
recovery evidence instead of becoming an XML well-formedness error.

XHTML remains a separate XML-backed package for `application/xhtml+xml`. In
`text/html`, HTML, SVG, and MathML are all parser-default DOM namespaces: SVG
and MathML tags switch into their own namespaces while remaining associated with
their registered schema packages.

## Lifecycle

The native HTML tokenizer feeds `HtmlDocumentAst` directly. Validation and
same-schema conversion consume that typed stream without reparsing the source as
CEM-ML or XML. HTML namespace semantics are tracked separately from lexical
spelling, including SVG and MathML islands and HTML integration points.

XHTML remains owned by the XML-backed `application/xhtml+xml` package. It does
not enter this lifecycle.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

All six public wrappers and their private helpers execute through the package
CEMT pipeline. Current formatter profiles are lexically lossless: they retain
the event lexemes and source maps while recording profile-specific layout
decisions. The text writer adds one final newline. Colorizers attach syntax
roles for terminal, HTML-class, and Markdown-span consumers.

## Validation

Validate HTML resources through the CLI with the schema URI and content type:

```bash
cem-ml validate --format json \
  --content-type text/html \
  --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/basic-document.html
```

Schema constraints bind native facts through `html-report-fact`. Reports include
the owning schema/package, constraint and policy, fact kind/value, source range,
and source map. The package reports malformed tokens, invalid doctypes and
quirks mode, encoding conflicts, duplicate attributes, recovered nesting,
scripts, event-handler attributes, external references, invalid custom-element
names, and unregistered foreign content.

## Safety and Limitations

Validation never fetches external resources or executes scripts. Script,
event-handler, and external-resource facts are rejected by the built-in package
policy.

The typed decoder currently accepts UTF-8 source bytes. MIME and `<meta charset>`
evidence is preserved and conflicts are reported, but legacy byte encodings are
not transcoded. Recovery covers the native tokenizer/event model rather than a
complete browser tree-construction implementation. Formatter profiles therefore
preserve source lexemes instead of claiming canonical browser DOM serialization.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the rendered example
content or validation diagnostics for expected-fail examples. The target writes a
preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.

<details>
<summary>basic-document</summary>

- Source: [`examples/basic-document.html`](./examples/basic-document.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/html/v1/examples/basic-document.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/html/v1/examples/basic-document.html,contentType=text/html,schema=https://cem.dev/ns/data/html/1 \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of HTML schema package v1 basic-document example](examples/previews/basic-document.html.svg)

<details>
<summary>fragment</summary>

- Source: [`examples/fragment.html`](./examples/fragment.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/html/v1/examples/fragment.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/html/v1/examples/fragment.html,contentType=text/html,schema=https://cem.dev/ns/data/html/1 \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of HTML schema package v1 fragment example](examples/previews/fragment.html.svg)

<details>
<summary>svg-mathml-islands</summary>

- Source: [`examples/svg-mathml-islands.html`](./examples/svg-mathml-islands.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/html/v1/examples/svg-mathml-islands.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/html/v1/examples/svg-mathml-islands.html,contentType=text/html,schema=https://cem.dev/ns/data/html/1 \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of HTML schema package v1 svg-mathml-islands example](examples/previews/svg-mathml-islands.html.svg)

<details>
<summary>invalid-script</summary>

- Source: [`examples/invalid-script.html`](./examples/invalid-script.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `fail`
- Expected diagnostics: `cem.html.script_rejected`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/html/v1/examples/invalid-script.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/html --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/invalid-script.html
```

</details>

![Preview of HTML schema package v1 invalid-script example](examples/previews/invalid-script.html.svg)

<details>
<summary>invalid-external-resource</summary>

- Source: [`examples/invalid-external-resource.html`](./examples/invalid-external-resource.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `fail`
- Expected diagnostics: `cem.html.external_resource_rejected`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/html/v1/examples/invalid-external-resource.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/html --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/invalid-external-resource.html
```

</details>

![Preview of HTML schema package v1 invalid-external-resource example](examples/previews/invalid-external-resource.html.svg)

<details>
<summary>invalid-custom-element</summary>

- Source: [`examples/invalid-custom-element.html`](./examples/invalid-custom-element.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `fail`
- Expected diagnostics: `cem.html.custom_element_name_invalid`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/html/v1/examples/invalid-custom-element.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/html --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/invalid-custom-element.html
```

</details>

![Preview of HTML schema package v1 invalid-custom-element example](examples/previews/invalid-custom-element.html.svg)

<details>
<summary>encoding-conflict</summary>

- Source: [`examples/encoding-conflict.html`](./examples/encoding-conflict.html)
- Content type: `text/html; charset=windows-1252`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- Expected diagnostics: `cem.html.encoding_conflict`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/html/v1/examples/encoding-conflict.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/html/v1/examples/encoding-conflict.html,contentType=text/html; charset=windows-1252,schema=https://cem.dev/ns/data/html/1' \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of HTML schema package v1 encoding-conflict example](examples/previews/encoding-conflict.html.svg)
