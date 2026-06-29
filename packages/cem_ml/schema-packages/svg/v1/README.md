# SVG Resource Schema Package

This package defines registry identity for SVG resources.

SVG source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `image/svg+xml` content type are
parsed as XML and validated as SVG vocabulary in the SVG namespace.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/svg/1`
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

Current lifecycle routing keeps `image/svg+xml` on the XML adapter. A bare SVG
namespace in mixed HTML remains an embedded-namespace hint for the HTML adapter.
