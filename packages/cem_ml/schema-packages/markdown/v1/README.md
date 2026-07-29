# Markdown Resource Schema Package

Status: schema, examples, formatter, and colorizer package frame

This package defines registry identity for generic Markdown resources.

Markdown source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `text/markdown` content type are
parsed by a Markdown parser or adapter.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/markdown/1`
- Primary content type: `text/markdown`
- Preferred extensions: `.md`, `.markdown`

RFC 7763 registers `text/markdown` with required `charset` and optional
`variant` parameters. RFC 7764 defines Markdown variant registration and local
storage guidance.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

## Resource Model

The schema describes Markdown resources as a document model:

- documents preserve encoding, optional charset, optional variant, and source
  identity;
- blocks preserve order, nesting, headings, lists, code fences, and block HTML;
- inline nodes preserve text, emphasis, links, images, breaks, and inline HTML;
- link references preserve labels, destinations, and titles;
- embedded HTML is explicit and must carry a trust policy before execution or
  rendering as trusted HTML;
- source offsets are preserved when the parser exposes them.

Variant-specific parsers can refine this model through converter profiles
instead of changing the generic Markdown schema identity.

## Verification

The package-local `cem_ml_schema_package_markdown_v1:verify` target checks:

- manifest validation against the schema-package schema;
- manifest-index coverage for all declared examples;
- embedded formatter/colorizer artifact catalog registration;
- formatter/colorizer CEMT body execution over `markdown-document` and
  `cem-tree` boundaries;
- Markdown source validation behavior for valid CommonMark, missing charset,
  unknown variant, embedded HTML rejection, and unsupported UTF-8;
- CLI validation behavior for the same Markdown source cases;
- README embedded HTML snippet drift for rendered Markdown examples, plus
  absence of Markdown README preview SVG artifacts.

## Release Behavior

Validation currently recognizes UTF-8 Markdown text, checks the `charset` and
`variant` content-type parameters, enables GFM parser options for GFM variants,
and rejects embedded HTML unless a future policy permits it. Markdown source
validation now opens a typed Markdown AST/event stream with source metadata,
encoding facts, variant facts, parser events, embedded-HTML facts, and source
maps before projecting diagnostics. Package-owned formatter/colorizer bodies
now consume the `markdown-document` subject and pass formatted/colored
`cem-tree` artifacts to the writer. The current formatter is an event-stream
writer, not a full Markdown block/inline reflow engine. Trusted HTML rendering
modes and variant-specific extension models are not part of the current release
contract.

Markdown documents can also export to `text/html` through the typed Markdown
AST stream. The current HTML export covers common block and inline Markdown and
supports fenced `cem-ml svg` blocks as trusted package examples: the fenced
CEM-ML is parsed and rendered as inline SVG markup before the generated HTML is
written as the README preview snippet.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Passing Markdown examples are converted by the
CLI to browser HTML and written to
`dist/cem_ml/schema-packages/markdown/v1/examples/<example-name>.md.html`.
The README embeds that generated file content directly as a fenced `html`
snippet. README preview SVGs are not generated for this package.

<details>
<summary>basic-document</summary>

- Source: [`examples/basic-document.md`](./examples/basic-document.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, Markdown AST to HTML, README html snippet`
- HTML output: `dist/cem_ml/schema-packages/markdown/v1/examples/basic-document.md.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/basic-document.md,contentType=text/markdown; charset=utf-8; variant=CommonMark,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile none --out \
  dist/cem_ml/schema-packages/markdown/v1/examples/basic-document.md.html
```

</details>

```html
<h1>CEM Markdown Example</h1>
<p>This document has <strong>strong</strong> text, <em>emphasis</em>, and a link to
<a href="https://cem.dev/">cem.dev</a>.</p>
<ul>
<li>Preserve source identity.</li>
<li>Keep parser diagnostics schema-owned.</li>
</ul>
```

<details>
<summary>gfm-worklog</summary>

- Source: [`examples/gfm-worklog.md`](./examples/gfm-worklog.md)
- Content type: `text/markdown; charset=utf-8; variant=GFM`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, Markdown AST to HTML, README html snippet`
- HTML output: `dist/cem_ml/schema-packages/markdown/v1/examples/gfm-worklog.md.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/gfm-worklog.md,contentType=text/markdown; charset=utf-8; variant=GFM,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile none --out \
  dist/cem_ml/schema-packages/markdown/v1/examples/gfm-worklog.md.html
```

</details>

```html
<h1>Worklog</h1>
<table>
<thead>
<tr>
<th>Task</th>
<th>Status</th>
</tr>
</thead>
<tr>
<td>Schema validation</td>
<td>Done</td>
</tr>
<tr>
<td>Markdown examples</td>
<td>In review</td>
</tr>
</table>
<ul>
<li>
<input type="checkbox" disabled checked> Add parser-backed validation.</li>
<li>
<input type="checkbox" disabled> Connect converter profiles.</li>
</ul>
<blockquote>
<p>Keep embedded HTML behind an explicit policy.</p>
</blockquote>
```

<details>
<summary>markdown-html-svg</summary>

- Source: [`examples/markdown1.md`](./examples/markdown1.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, Markdown AST to HTML, README html snippet`
- HTML output: `dist/cem_ml/schema-packages/markdown/v1/examples/markdown1.md.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/markdown1.md,contentType=text/markdown; charset=utf-8; variant=CommonMark,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile none --out \
  dist/cem_ml/schema-packages/markdown/v1/examples/markdown1.md.html
```

</details>

```html
<h1>Browser Markdown</h1>
<p>Markdown defines content that is normally expressed in the browser as HTML.</p>
<p>The next block embeds an SVG authored as CEM-ML and exports it as inline HTML
SVG markup.</p>
<svg viewBox="0 0 160 80" xmlns="http://www.w3.org/2000/svg">
<title>CEM-ML inline SVG</title>
<path d="M20 40h120M80 14v52M48 24l32 16-32 16">
</path>
</svg>
<p>The generated HTML keeps the SVG inline, so a browser can render it without a
separate image file.</p>
```

<details>
<summary>invalid-embedded-html</summary>

- Source: [`examples/invalid-embedded-html.md`](./examples/invalid-embedded-html.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `fail`
- Expected diagnostics: `cem.markdown.embedded_html_rejected`
- Preview renderer: `CLI validate; no HTML preview for expected-fail example`
- HTML output: not generated for expected-fail examples

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type 'text/markdown; charset=utf-8; variant=CommonMark' --schema \
  https://cem.dev/ns/data/markdown/1 \
  packages/cem_ml/schema-packages/markdown/v1/examples/invalid-embedded-html.md
```

</details>

<details>
<summary>unknown-variant</summary>

- Source: [`examples/unknown-variant.md`](./examples/unknown-variant.md)
- Content type: `text/markdown; charset=utf-8; variant=CustomWiki`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Expected diagnostics: `cem.markdown.unknown_variant`
- Preview renderer: `CLI convert, Markdown AST to HTML, README html snippet`
- HTML output: `dist/cem_ml/schema-packages/markdown/v1/examples/unknown-variant.md.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/unknown-variant.md,contentType=text/markdown; charset=utf-8; variant=CustomWiki,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/html --to-schema https://cem.dev/ns/data/html/1 \
  --cemt-formatter-profile tabular --cemt-color-profile none --out \
  dist/cem_ml/schema-packages/markdown/v1/examples/unknown-variant.md.html
```

</details>

```html
<h1>Custom Variant</h1>
<p>This content is valid Markdown, but its content type declares an unregistered
variant parameter in the validation example.</p>
```
