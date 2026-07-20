# MathML schema package v1

This package defines the CEM schema identity for MathML resources.

- Schema URI: `https://cem.dev/ns/data/mathml/1`
- Primary content type: `application/mathml+xml`
- Alias content types: `application/mathml-presentation+xml`, `application/mathml-content+xml`
- Document namespace: `http://www.w3.org/1998/Math/MathML`
- Source schema: `schema/mathml.cem`

MathML is XML-backed. Direct validation routes MathML content types through
this schema package and reuses the XML event reader as the parser. Conversion
and export lifecycle routing still uses the XML adapter for standalone MathML
resources, while MathML embedded in HTML can be selected by namespace through
the HTML adapter when no explicit content type or package schema URI is present.

The schema keeps the presentation, content, semantics, annotation, source-map, and accessibility-related fields explicit so later converters can normalize MathML without losing source identity.

## Validation

Validate MathML resources through the CLI with the schema URI and content type:

```bash
cem-ml validate --format json \
  --content-type application/mathml+xml \
  --schema https://cem.dev/ns/data/mathml/1 \
  packages/cem_ml/schema-packages/mathml/v1/examples/basic-presentation.mml
```

The direct validator parses MathML as XML, requires a `math` root in the MathML
namespace, recognizes the presentation and content media-type aliases, and
reports external annotation `src` values as policy warnings unless a loader
policy is supplied by a later conversion layer.

## Examples

- [basic-presentation.mml](examples/basic-presentation.mml): a minimal
  presentation MathML expression.
- [content-expression.mathml](examples/content-expression.mathml): a content
  MathML expression for the content media-type alias.
- [semantics-external-annotation.mml](examples/semantics-external-annotation.mml):
  valid MathML that reports a warning for an external annotation source.
- [invalid-missing-namespace.mml](examples/invalid-missing-namespace.mml):
  XML with a `math` root that does not claim the MathML namespace.
- [invalid-root-not-math.mml](examples/invalid-root-not-math.mml): MathML
  namespace content with the wrong document root.
- [invalid-content-profile-presentation-only.mml](examples/invalid-content-profile-presentation-only.mml):
  presentation-only MathML checked as `application/mathml-content+xml`.
- [invalid-not-well-formed.mml](examples/invalid-not-well-formed.mml):
  malformed XML.
