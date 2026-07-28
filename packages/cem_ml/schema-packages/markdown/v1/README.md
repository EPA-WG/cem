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
- Markdown source validation behavior for valid CommonMark, missing charset,
  unknown variant, embedded HTML rejection, and unsupported UTF-8;
- CLI validation behavior for the same Markdown source cases;
- README SVG preview drift for every manifest example.

## Release Behavior

Validation currently recognizes UTF-8 Markdown text, checks the `charset` and
`variant` content-type parameters, enables GFM parser options for GFM variants,
and rejects embedded HTML unless a future policy permits it. Markdown source
validation is still a direct source-validation path; a typed Markdown lifecycle
AST stream, package-owned formatter/colorizer output bodies, trusted HTML
rendering modes, and variant-specific extension models are not part of the
current release contract.

## Tracked Incomplete Work

- Markdown validation still reports directly from the source validator instead
  of returning a typed Markdown lifecycle AST stream with neutral parse facts.
- Formatter/colorizer CEMT assets are currently package declarations with
  placeholder raw `json`/`tokens` boundaries.
- README previews intentionally use source snapshots until package-owned
  Markdown formatter/colorizer output can render examples directly.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the example content, not
the validation report. The target writes a preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.
Source snapshots are used only where the current CLI cannot yet render
the package formatter/colorizer path for that content identity.

<details>
<summary>basic-document</summary>

- Source: [`examples/basic-document.md`](./examples/basic-document.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/markdown/v1/examples/basic-document.md.html`

</details>

![Preview of Markdown Resource Schema Package basic-document example](examples/previews/basic-document.md.svg)

<details>
<summary>gfm-worklog</summary>

- Source: [`examples/gfm-worklog.md`](./examples/gfm-worklog.md)
- Content type: `text/markdown; charset=utf-8; variant=GFM`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/markdown/v1/examples/gfm-worklog.md.html`

</details>

![Preview of Markdown Resource Schema Package gfm-worklog example](examples/previews/gfm-worklog.md.svg)

<details>
<summary>invalid-embedded-html</summary>

- Source: [`examples/invalid-embedded-html.md`](./examples/invalid-embedded-html.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `fail`
- Expected diagnostics: `cem.markdown.embedded_html_rejected`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/markdown/v1/examples/invalid-embedded-html.md.html`

</details>

![Preview of Markdown Resource Schema Package invalid-embedded-html example](examples/previews/invalid-embedded-html.md.svg)

<details>
<summary>unknown-variant</summary>

- Source: [`examples/unknown-variant.md`](./examples/unknown-variant.md)
- Content type: `text/markdown; charset=utf-8; variant=CustomWiki`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Expected diagnostics: `cem.markdown.unknown_variant`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/markdown/v1/examples/unknown-variant.md.html`

</details>

![Preview of Markdown Resource Schema Package unknown-variant example](examples/previews/unknown-variant.md.svg)
