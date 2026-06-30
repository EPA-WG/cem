# XML Resource Schema Package

This package defines registry identity for generic XML resources.

XML source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with XML content types are parsed by an XML
parser or adapter.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/xml/1`
- Primary content type: `application/xml`
- Alias content types: `text/xml`, `application/xml-external-parsed-entity`,
  `text/xml-external-parsed-entity`, `application/xml-dtd`
- Preferred extension: `.xml`

RFC 7303 standardizes generic XML media types and the `+xml` structured syntax
suffix.

## Resource Model

The schema describes XML resources as a namespace-aware document model:

- documents preserve XML declaration, MIME charset, XML version, standalone
  state, root element, optional doctype, and source identity;
- elements and attributes preserve qualified names, expanded namespace identity,
  lexical order, and source offsets when available;
- text, CDATA, comments, processing instructions, and entity references remain
  explicit nodes;
- DTD and external entity material is preserved as declarations but external
  resolution is rejected unless an explicit policy enables it;
- external parsed entities and XML DTD resources use the same schema identity
  with specialized top-level resource elements.

This package intentionally does not claim all media types ending in `+xml`.
Domain formats such as XHTML, SVG, MathML, XSLT, Atom, and RSS need their own
schema packages that can depend on the generic XML schema.

## Validation Examples

Validate XML resources through the CLI with the schema URL and content type:

```bash
cem-ml validate --format json \
  --content-type application/xml \
  --schema https://cem.dev/ns/data/xml/1 \
  packages/cem_ml/schema-packages/xml/v1/examples/basic-document.xml
```

Checked examples:

- [basic-document.xml](examples/basic-document.xml): a minimal XML document.
- [namespaced-document.xml](examples/namespaced-document.xml): default and
  prefixed namespace declarations.
- [invalid-mismatched-tag.xml](examples/invalid-mismatched-tag.xml): reports
  `cem.xml.parse_error`.
- [invalid-unbound-prefix.xml](examples/invalid-unbound-prefix.xml): reports
  `cem.xml.unbound_namespace_prefix`.
- [invalid-doctype.xml](examples/invalid-doctype.xml): reports
  `cem.xml.dtd_rejected` under the current no-DTD runtime policy.
