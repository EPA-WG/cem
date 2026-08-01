# CSS schema package v1

Status: typed lifecycle, schema validation, and executable output profiles

This package defines the CEM schema identity for CSS stylesheets and scoped style content.

- Schema URI: `https://cem.dev/ns/data/css/1`
- Primary content type: `text/css`
- Source schema: `schema/css.cem`

CSS source is not CEM-ML syntax. The `CssAdapter` loads it into a lossless
`CssDocumentAst` backed by `cssparser` token and nested component-value
recovery. Exact comments, whitespace, lexemes, byte ranges, source maps, MIME
parameters, encoding evidence, and line endings remain available to validators
and output profiles.

Scoped style content is represented as metadata on style blocks and style attributes. The scope can point at an HTML, SVG, MathML, custom-element, or shadow-root host without changing the `text/css` content identity.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.
Each public profile is an executable wrapper over a private CSS CEM-tree helper.
Compact is lexically lossless; pretty and tabular currently retain the same
conservative lexical boundaries rather than rewriting strings, comments,
custom properties, functions, or nested rules. Text output receives the common
default final newline.

## Validation

Validate CSS resources through the CLI with the schema URI and content type:

```bash
cem-ml validate --format json \
  --content-type text/css \
  --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/basic-stylesheet.css
```

Validation runs through the typed lifecycle and emits neutral parser and policy
facts. Executable contracts in `schema/css.cem` own diagnostic codes,
severities, and policy metadata. Entry modes cover full stylesheets,
declaration lists/style attributes, and scoped style blocks; callers can select
the latter modes with `mode=style-attribute` or `mode=scoped-style-block` MIME
parameters.

Parsing, validation, and formatting do not fetch `@import` resources, URLs,
fonts, or other external references and do not evaluate cascade or host-document
semantics. `@import` and external `url()` references require a future explicit
resolver or sanitizer capability; unknown at-rules, vendor syntax, and custom
property token streams are preserved verbatim.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the rendered example
content or validation diagnostics for expected-fail examples. The target writes a
preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.

<details>
<summary>basic-stylesheet</summary>

- Source: [`examples/basic-stylesheet.css`](./examples/basic-stylesheet.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/basic-stylesheet.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/basic-stylesheet.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of CSS schema package v1 basic-stylesheet example](examples/previews/basic-stylesheet.css.svg)

<details>
<summary>scoped-component</summary>

- Source: [`examples/scoped-component.css`](./examples/scoped-component.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/scoped-component.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/scoped-component.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of CSS schema package v1 scoped-component example](examples/previews/scoped-component.css.svg)

<details>
<summary>style-attribute</summary>

- Source: [`examples/style-attribute.css`](./examples/style-attribute.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/style-attribute.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/style-attribute.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of CSS schema package v1 style-attribute example](examples/previews/style-attribute.css.svg)

<details>
<summary>invalid-import</summary>

- Source: [`examples/invalid-import.css`](./examples/invalid-import.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `fail`
- Expected diagnostics: `cem.css.import_rejected`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/invalid-import.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/css --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/invalid-import.css
```

</details>

![Preview of CSS schema package v1 invalid-import example](examples/previews/invalid-import.css.svg)

<details>
<summary>invalid-url</summary>

- Source: [`examples/invalid-url.css`](./examples/invalid-url.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `fail`
- Expected diagnostics: `cem.css.url_rejected`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/invalid-url.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/css --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/invalid-url.css
```

</details>

![Preview of CSS schema package v1 invalid-url example](examples/previews/invalid-url.css.svg)

<details>
<summary>invalid-token</summary>

- Source: [`examples/invalid-token.css`](./examples/invalid-token.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `fail`
- Expected diagnostics: `cem.css.invalid_token`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/invalid-token.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/css --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/invalid-token.css
```

</details>

![Preview of CSS schema package v1 invalid-token example](examples/previews/invalid-token.css.svg)

<details>
<summary>invalid-declaration</summary>

- Source: [`examples/invalid-declaration.css`](./examples/invalid-declaration.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- Expected diagnostics: `cem.css.invalid_declaration`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/invalid-declaration.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/invalid-declaration.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of CSS schema package v1 invalid-declaration example](examples/previews/invalid-declaration.css.svg)

<details>
<summary>encoding-conflict</summary>

- Source: [`examples/encoding-conflict.css`](./examples/encoding-conflict.css)
- Content type: `text/css; charset=iso-8859-1`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- Expected diagnostics: `cem.css.encoding_conflict`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/css/v1/examples/encoding-conflict.css.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/css/v1/examples/encoding-conflict.css,contentType=text/css; charset=iso-8859-1,schema=https://cem.dev/ns/data/css/1' \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of CSS schema package v1 encoding-conflict example](examples/previews/encoding-conflict.css.svg)
