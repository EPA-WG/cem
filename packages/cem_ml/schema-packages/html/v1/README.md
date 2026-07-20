# HTML schema package v1

Status: schema, examples, formatter, colorizer, and converter package frame

This package defines the CEM schema identity for HTML resources.

- Schema URI: `https://cem.dev/ns/data/html/1`
- Primary content type: `text/html`
- DOM namespaces:
    - HTML: `http://www.w3.org/1999/xhtml`
    - SVG: `http://www.w3.org/2000/svg`
    - MathML: `http://www.w3.org/1998/Math/MathML`
- Source schema: `schema/html.cem`

HTML is not XML. The package models `text/html` as an HTML-parser-backed source format that can recover incomplete or non-normalized markup into a normalized DOM, preserving source identity where parser offsets are available.

XHTML remains a separate XML-backed package for `application/xhtml+xml`. In
`text/html`, HTML, SVG, and MathML are all parser-default DOM namespaces: SVG
and MathML tags switch into their own namespaces while remaining associated with
their registered schema packages.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

## Validation

Validate HTML resources through the CLI with the schema URI and content type:

```bash
cem-ml validate --format json \
  --content-type text/html \
  --schema https://cem.dev/ns/data/html/1 \
  packages/cem_ml/schema-packages/html/v1/examples/basic-document.html
```

The direct validator treats incomplete and non-normalized HTML as parser-backed
input, accepts SVG and MathML namespace islands, rejects executable script and
external resource access without explicit policy, reports invalid custom-element
names, and preserves parser recovery as diagnostics instead of requiring XML
well-formedness.

## Examples

- [basic-document.html](examples/basic-document.html): a complete HTML document.
- [fragment.html](examples/fragment.html): an incomplete fragment accepted by
  the HTML parser contract.
- [svg-mathml-islands.html](examples/svg-mathml-islands.html): HTML with SVG
  and MathML tags using their parser-default namespaces.
- [invalid-script.html](examples/invalid-script.html): executable script
  rejected by default policy.
- [invalid-external-resource.html](examples/invalid-external-resource.html):
  external resource access without resolver policy.
- [invalid-custom-element.html](examples/invalid-custom-element.html): invalid
  custom-element name.
- [encoding-conflict.html](examples/encoding-conflict.html): warning-only MIME
  charset and meta charset conflict.
