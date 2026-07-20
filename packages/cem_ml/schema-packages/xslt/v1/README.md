# XSLT schema package v1

This package defines CEM schema identity for XSLT stylesheet resources and for the legacy custom-element XSLT compatibility markers.

- Schema URI: `https://cem.dev/ns/transform/xslt/1`
- Primary content type: `application/xslt+xml`
- Alias content types: `text/xsl`, `custom-element-xslt`, `text/custom-element-xslt`, `application/custom-element-xslt`, `text/x-custom-element-xslt`
- Document namespace: `http://www.w3.org/1999/XSL/Transform`
- Source schema: `schema/xslt.cem`

XSLT is XML-backed. Direct validation routes the XSLT media type and
compatibility aliases through this schema package and reuses the XML event
reader as the parser. The package registers the stylesheet identity and the
compatibility aliases consumed by the existing CEM legacy custom-element
adapter.

The current executable support is intentionally bounded: copied custom-element and XSLT parity templates can lower through the CEM-owned compatibility path, while full XSLT 3.0/4.0 execution remains capability-gated roadmap work. Browser-native `XSLTProcessor` execution is not part of this package contract.

## Validation

Validate XSLT stylesheet resources through the CLI with the schema URI and
content type:

```bash
cem-ml validate --format json \
  --content-type application/xslt+xml \
  --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl
```

The direct validator parses standard XSLT as XML, requires an `xsl:stylesheet`
or `xsl:transform` root in the XSLT namespace, validates the root `version`
attribute, requires at least one top-level `xsl:template`, rejects external URI
constructs such as `xsl:include`, and reports warning diagnostics for extension
constructs outside the bounded legacy compatibility profile. The
`custom-element-xslt` aliases also accept legacy custom-element fragment
resources when the source is not a stylesheet root.

## Examples

- [basic-stylesheet.xsl](examples/basic-stylesheet.xsl): a minimal XSLT
  stylesheet using the primary media type.
- [named-template.xslt](examples/named-template.xslt): a stylesheet with a
  default template and named template entrypoint.
- [legacy-custom-element-stylesheet.xsl](examples/legacy-custom-element-stylesheet.xsl):
  a bounded legacy custom-element compatibility stylesheet.
- [legacy-custom-element-fragment.html](examples/legacy-custom-element-fragment.html):
  a legacy custom-element fragment validated through the custom-element alias.
- [unsupported-extension-warning.xsl](examples/unsupported-extension-warning.xsl):
  valid XML/XSLT that reports a warning for an unsupported extension construct.
- [invalid-missing-namespace.xsl](examples/invalid-missing-namespace.xsl):
  stylesheet-shaped XML without the XSLT namespace.
- [invalid-missing-version.xsl](examples/invalid-missing-version.xsl):
  XSLT root missing its required version.
- [invalid-external-include.xsl](examples/invalid-external-include.xsl):
  stylesheet with an external include URI and no resolver policy.
- [invalid-missing-entrypoint.xsl](examples/invalid-missing-entrypoint.xsl):
  stylesheet with no top-level template.
- [invalid-not-well-formed.xsl](examples/invalid-not-well-formed.xsl):
  malformed XML.
