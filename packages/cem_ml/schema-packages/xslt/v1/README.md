# XSLT schema package v1

This package defines CEM schema identity for XSLT stylesheet resources and for the legacy custom-element XSLT compatibility markers.

- Schema URL: `https://cem.dev/ns/transform/xslt/1`
- Primary content type: `application/xslt+xml`
- Alias content types: `text/xsl`, `custom-element-xslt`, `text/custom-element-xslt`, `application/custom-element-xslt`, `text/x-custom-element-xslt`
- Document namespace: `http://www.w3.org/1999/XSL/Transform`
- Source schema: `schema/xslt.cem`

XSLT is XML-backed. The package registers the stylesheet identity and the compatibility aliases consumed by the existing CEM legacy custom-element adapter.

The current executable support is intentionally bounded: copied custom-element and XSLT parity templates can lower through the CEM-owned compatibility path, while full XSLT 3.0/4.0 execution remains capability-gated roadmap work. Browser-native `XSLTProcessor` execution is not part of this package contract.
