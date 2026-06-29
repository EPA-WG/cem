# Markdown Resource Schema Package

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
