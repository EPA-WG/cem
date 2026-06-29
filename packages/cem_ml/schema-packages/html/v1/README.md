# HTML schema package v1

This package defines the CEM schema identity for HTML resources.

- Schema URL: `https://cem.dev/ns/data/html/1`
- Primary content type: `text/html`
- DOM namespace: `http://www.w3.org/1999/xhtml`
- Source schema: `schema/html.cem`

HTML is not XML. The package models `text/html` as an HTML-parser-backed source format that can recover incomplete or non-normalized markup into a normalized DOM, preserving source identity where parser offsets are available.

XHTML remains a separate XML-backed package for `application/xhtml+xml`. SVG and MathML islands are foreign content delegated to their own schema packages.
