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
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-stylesheet</summary>

- Source: [`examples/basic-stylesheet.css`](./examples/basic-stylesheet.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/basic-stylesheet.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```css
@charset "utf-8";

:root {
  --space-2: 0.5rem;
  color-scheme: light dark;
}

body {
  margin: 0;
  font-family: system-ui, sans-serif;
}

.card {
  padding: var(--space-2);
  border: 1px solid currentColor;
}
```

<details>
<summary>scoped-component</summary>

- Source: [`examples/scoped-component.css`](./examples/scoped-component.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/scoped-component.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```css
@layer components {
  :host {
    display: block;
  }

  @scope (.profile-card) {
    .profile-card__title {
      font-weight: 700;
    }
  }
}
```

<details>
<summary>style-attribute</summary>

- Source: [`examples/style-attribute.css`](./examples/style-attribute.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/style-attribute.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```css
color: currentColor;
margin-inline: 0;
--card-gap: 0.75rem;
```

<details>
<summary>invalid-import</summary>

- Source: [`examples/invalid-import.css`](./examples/invalid-import.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `fail`
- Expected diagnostics: `cem.css.import_rejected`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/css --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/invalid-import.css
```

</details>

```css
@import "shared/theme.css";

.card {
  color: currentColor;
}
```

<details>
<summary>invalid-url</summary>

- Source: [`examples/invalid-url.css`](./examples/invalid-url.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `fail`
- Expected diagnostics: `cem.css.url_rejected`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/css --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/invalid-url.css
```

</details>

```css
.hero {
  background-image: url("images/hero.png");
  min-height: 20rem;
}
```

<details>
<summary>invalid-token</summary>

- Source: [`examples/invalid-token.css`](./examples/invalid-token.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `fail`
- Expected diagnostics: `cem.css.invalid_token`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type text/css --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/invalid-token.css
```

</details>

```css
.card {
  color: currentColor;
```

<details>
<summary>invalid-declaration</summary>

- Source: [`examples/invalid-declaration.css`](./examples/invalid-declaration.css)
- Content type: `text/css`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- Expected diagnostics: `cem.css.invalid_declaration`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/css/v1/examples/invalid-declaration.css,contentType=text/css,schema=https://cem.dev/ns/data/css/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```css
.card {
  color currentColor;
  padding: 1rem;
}
```

<details>
<summary>encoding-conflict</summary>

- Source: [`examples/encoding-conflict.css`](./examples/encoding-conflict.css)
- Content type: `text/css; charset=iso-8859-1`
- Schema: `https://cem.dev/ns/data/css/1`
- Expected result: `pass`
- Expected diagnostics: `cem.css.encoding_conflict`
- README rendering: fenced `css` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/css/v1/examples/encoding-conflict.css,contentType=text/css; charset=iso-8859-1,schema=https://cem.dev/ns/data/css/1' \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```css
@charset "utf-8";

.card {
  color: currentColor;
}
```
