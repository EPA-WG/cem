# Markdown Resource Schema Package

Status: schema, examples, formatter, colorizer, and Markdown-to-HTML converter package

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

The package declares CEMT formatter and colorizer artifacts plus the
`markdown-to-html-rust` typed converter edge in `package.cem`.
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
- exact language-tagged Markdown source-fence drift for every example, plus
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

Markdown documents can also convert to `text/html` through the typed Markdown
AST stream. Direct CLI conversion and transform-graph `convert` nodes select
the schema-owned `markdown-to-html-rust` edge; graph conversion produces a
typed HTML artifact that later graph stages can consume. The current HTML
conversion covers common block and inline Markdown and
supports fenced `cem-ml svg` blocks as trusted package examples: the fenced
CEM-ML is parsed and rendered as inline SVG markup before the generated HTML is
written as conversion output. The package README still quotes the original
Markdown source rather than substituting rendered HTML or an SVG snapshot.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-document</summary>

- Source: [`examples/basic-document.md`](./examples/basic-document.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- README rendering: fenced `markdown` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/basic-document.md,contentType=text/markdown; charset=utf-8; variant=CommonMark,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/markdown --to-schema https://cem.dev/ns/data/markdown/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```markdown
# CEM Markdown Example

This document has **strong** text, _emphasis_, and a link to
[cem.dev](https://cem.dev/).

- Preserve source identity.
- Keep parser diagnostics schema-owned.
```

<details>
<summary>gfm-worklog</summary>

- Source: [`examples/gfm-worklog.md`](./examples/gfm-worklog.md)
- Content type: `text/markdown; charset=utf-8; variant=GFM`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- README rendering: fenced `markdown` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/gfm-worklog.md,contentType=text/markdown; charset=utf-8; variant=GFM,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/markdown --to-schema https://cem.dev/ns/data/markdown/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```markdown
# Worklog

| Task | Status |
| --- | --- |
| Schema validation | Done |
| Markdown examples | In review |

- [x] Add parser-backed validation.
- [ ] Connect converter profiles.

> Keep embedded HTML behind an explicit policy.
```

<details>
<summary>markdown-html-svg</summary>

- Source: [`examples/markdown1.md`](./examples/markdown1.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- README rendering: fenced `markdown` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/markdown1.md,contentType=text/markdown; charset=utf-8; variant=CommonMark,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/markdown --to-schema https://cem.dev/ns/data/markdown/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

````markdown
# Browser Markdown

Markdown defines content that is normally expressed in the browser as HTML.

The next block embeds an SVG authored as CEM-ML and exports it as inline HTML
SVG markup.

```cem-ml svg
@doc cem-ml 1
{svg @xmlns="http://www.w3.org/2000/svg" @viewBox="0 0 160 80" |
    {title | CEM-ML inline SVG}
    {path @d="M20 40h120M80 14v52M48 24l32 16-32 16"}
}
```

The generated HTML keeps the SVG inline, so a browser can render it without a
separate image file.
````

<details>
<summary>invalid-embedded-html</summary>

- Source: [`examples/invalid-embedded-html.md`](./examples/invalid-embedded-html.md)
- Content type: `text/markdown; charset=utf-8; variant=CommonMark`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `fail`
- Expected diagnostics: `cem.markdown.embedded_html_rejected`
- README rendering: fenced `markdown` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type 'text/markdown; charset=utf-8; variant=CommonMark' --schema \
  https://cem.dev/ns/data/markdown/1 \
  packages/cem_ml/schema-packages/markdown/v1/examples/invalid-embedded-html.md
```

</details>

```markdown
# Unsafe Markdown

<script>alert('x')</script>
```

<details>
<summary>unknown-variant</summary>

- Source: [`examples/unknown-variant.md`](./examples/unknown-variant.md)
- Content type: `text/markdown; charset=utf-8; variant=CustomWiki`
- Schema: `https://cem.dev/ns/data/markdown/1`
- Expected result: `pass`
- Expected diagnostics: `cem.markdown.unknown_variant`
- README rendering: fenced `markdown` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/markdown/v1/examples/unknown-variant.md,contentType=text/markdown; charset=utf-8; variant=CustomWiki,schema=https://cem.dev/ns/data/markdown/1' \
  --to-content-type text/markdown --to-schema https://cem.dev/ns/data/markdown/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```markdown
# Custom Variant

This content is valid Markdown, but its content type declares an unregistered
variant parameter in the validation example.
```
