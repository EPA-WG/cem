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

Validation routes `image/svg+xml` through the SVG schema package and reuses the
XML event reader as the parser. Conversion/export lifecycle routing still uses
the XML adapter for standalone SVG resources. A bare SVG namespace in mixed HTML
remains an embedded-namespace hint for the HTML adapter.

## Validation

Validate SVG resources through the CLI with the schema URL and content type:

```bash
cem-ml validate --format json \
  --content-type image/svg+xml \
  --schema https://cem.dev/ns/data/svg/1 \
  packages/cem_ml/schema-packages/svg/v1/examples/basic-icon.svg
```

The direct validator parses SVG as XML, requires an `svg` root in the SVG
namespace, rejects scripts and external resource references unless an explicit
policy is available, and reports warning diagnostics for visible SVG roots that
do not provide accessible name material.

## Examples

- [basic-icon.svg](examples/basic-icon.svg): a minimal named icon.
- [bar-chart.svg](examples/bar-chart.svg): a small chart using internal
  definitions and fragment-only paint references.
- [unnamed-icon.svg](examples/unnamed-icon.svg): valid SVG with a warning for
  missing accessible name material.
- [invalid-missing-namespace.svg](examples/invalid-missing-namespace.svg):
  XML with an `svg` root that does not claim the SVG namespace.
- [invalid-script.svg](examples/invalid-script.svg): executable SVG script,
  rejected by default policy.
- [invalid-external-image.svg](examples/invalid-external-image.svg): external
  image reference without an explicit resolver policy.
