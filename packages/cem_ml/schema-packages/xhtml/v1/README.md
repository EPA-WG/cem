# XHTML Resource Schema Package

This package defines registry identity for XHTML resources.

XHTML source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/xhtml+xml` content type
are parsed as XML and validated as HTML vocabulary in the XHTML namespace.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/xhtml/1`
- Primary content type: `application/xhtml+xml`
- Document namespace: `http://www.w3.org/1999/xhtml`
- Preferred extensions: `.xhtml`, `.xht`

RFC 3236 registers `application/xhtml+xml` with optional `charset` and
`profile` parameters. IANA now records `profile` as deprecated because XHTML
profiles were obsoleted in HTML5.

## Resource Model

The schema describes XHTML resources as an XML-based HTML document model:

- documents reuse the generic XML schema and preserve XML declaration,
  charset, doctype, source identity, and byte offsets;
- the root element must be `html` in the XHTML namespace;
- `head` and `body` structure is explicit and ordered;
- metadata, flow, phrasing, and interactive content are modeled as XHTML
  vocabulary facets over XML elements;
- foreign content is explicit and must be handled by a registered schema
  package or converter profile;
- `text/html` remains a separate HTML serialization identity and is not claimed
  by this package.

Current runtime export still routes `application/xhtml+xml` through the HTML
adapter; this package records the future XHTML-specific schema identity and
validation surface.

## Validation Examples

Validate XHTML resources through the CLI with the schema URL and content type:

```bash
cem-ml validate --format json \
  --content-type application/xhtml+xml \
  --schema https://cem.dev/ns/data/xhtml/1 \
  packages/cem_ml/schema-packages/xhtml/v1/examples/basic-document.xhtml
```

Checked examples:

- [basic-document.xhtml](examples/basic-document.xhtml): a minimal XHTML
  document with `head` and `body`.
- [form-page.xhtml](examples/form-page.xhtml): a small XHTML form page using
  XML serialization for void elements.
- [invalid-missing-namespace.xhtml](examples/invalid-missing-namespace.xhtml):
  reports `cem.xhtml.namespace_missing`.
- [invalid-body-before-head.xhtml](examples/invalid-body-before-head.xhtml):
  reports `cem.xhtml.head_body_order`.
- [invalid-not-well-formed.xhtml](examples/invalid-not-well-formed.xhtml):
  reports `cem.xhtml.not_well_formed_xml`.
