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
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-document</summary>

- Source: [`examples/basic-document.html`](./examples/basic-document.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/html/v1/examples/basic-document.html,contentType=text/html,schema=https://cem.dev/ns/data/html/1 \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Basic HTML Document</title>
  </head>
  <body>
    <main>
      <h1>Welcome</h1>
      <p>Hello from HTML.</p>
    </main>
  </body>
</html>
```

<details>
<summary>fragment</summary>

- Source: [`examples/fragment.html`](./examples/fragment.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/html/v1/examples/fragment.html,contentType=text/html,schema=https://cem.dev/ns/data/html/1 \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```html
<article>
  <h2>Card</h2>
  <p>This fragment relies on HTML parser recovery.
</article>
```

<details>
<summary>svg-mathml-islands</summary>

- Source: [`examples/svg-mathml-islands.html`](./examples/svg-mathml-islands.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/html/v1/examples/svg-mathml-islands.html,contentType=text/html,schema=https://cem.dev/ns/data/html/1 \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>SVG and MathML Islands</title>
  </head>
  <body>
    <svg viewBox="0 0 24 24" role="img" aria-label="Plus">
      <title>Plus</title>
      <path d="M12 3v18"></path>
      <path d="M3 12h18"></path>
    </svg>
    <math display="inline" alttext="x plus one">
      <mrow>
        <mi>x</mi>
        <mo>+</mo>
        <mn>1</mn>
      </mrow>
    </math>
  </body>
</html>
```

<details>
<summary>invalid-script</summary>

- Source: [`examples/invalid-script.html`](./examples/invalid-script.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `fail`
- Expected diagnostics: `cem.html.script_rejected`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/html --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/invalid-script.html
```

</details>

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Script</title>
  </head>
  <body>
    <script>alert("blocked")</script>
  </body>
</html>
```

<details>
<summary>invalid-external-resource</summary>

- Source: [`examples/invalid-external-resource.html`](./examples/invalid-external-resource.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `fail`
- Expected diagnostics: `cem.html.external_resource_rejected`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/html --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/invalid-external-resource.html
```

</details>

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>External Resource</title>
  </head>
  <body>
    <img src="images/logo.png" alt="Logo">
  </body>
</html>
```

<details>
<summary>invalid-custom-element</summary>

- Source: [`examples/invalid-custom-element.html`](./examples/invalid-custom-element.html)
- Content type: `text/html`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `fail`
- Expected diagnostics: `cem.html.custom_element_name_invalid`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/html --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/invalid-custom-element.html
```

</details>

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Invalid Custom Element</title>
  </head>
  <body>
    <x->Broken custom element name</x->
  </body>
</html>
```

<details>
<summary>encoding-conflict</summary>

- Source: [`examples/encoding-conflict.html`](./examples/encoding-conflict.html)
- Content type: `text/html; charset=windows-1252`
- Schema: `https://cem.dev/ns/data/html/1`
- Expected result: `pass`
- Expected diagnostics: `cem.html.encoding_conflict`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/html/v1/examples/encoding-conflict.html,contentType=text/html; charset=windows-1252,schema=https://cem.dev/ns/data/html/1' \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Encoding Conflict</title>
  </head>
  <body>
    <p>The CLI content type declares a different charset.</p>
  </body>
</html>
```
