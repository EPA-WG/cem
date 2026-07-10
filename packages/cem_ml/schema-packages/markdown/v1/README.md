# Markdown Resource Schema Package

Status: schema, examples, formatter, and colorizer package frame

This package defines registry identity for generic Markdown resources.

Markdown source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `text/markdown` content type are
parsed by a Markdown parser or adapter.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/markdown/1`
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

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-document.md`](examples/basic-document.md) | CommonMark headings, emphasis, links, and lists. | Pass |
| [`gfm-worklog.md`](examples/gfm-worklog.md) | GFM-style table and task list using the `variant=GFM` parser profile. | Pass |
| [`invalid-embedded-html.md`](examples/invalid-embedded-html.md) | Raw HTML rejected by the default embedded HTML policy. | Fail with `cem.markdown.embedded_html_rejected` |
| [`unknown-variant.md`](examples/unknown-variant.md) | Valid Markdown with an unregistered `variant` content-type parameter. | Pass with warning `cem.markdown.unknown_variant` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type 'text/markdown; charset=utf-8; variant=CommonMark' \
  --schema https://cem.dev/ns/data/markdown/1 \
  packages/cem_ml/schema-packages/markdown/v1/examples/basic-document.md
```
