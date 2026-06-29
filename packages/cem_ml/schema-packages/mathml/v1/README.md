# MathML schema package v1

This package defines the CEM schema identity for MathML resources.

- Schema URL: `https://cem.dev/ns/data/mathml/1`
- Primary content type: `application/mathml+xml`
- Alias content types: `application/mathml-presentation+xml`, `application/mathml-content+xml`
- Document namespace: `http://www.w3.org/1998/Math/MathML`
- Source schema: `schema/mathml.cem`

MathML is XML-backed. Standalone MathML package identity is handled by the XML lifecycle adapter, while MathML embedded in HTML can be selected by namespace through the HTML adapter when no explicit content type or package schema URL is present.

The schema keeps the presentation, content, semantics, annotation, source-map, and accessibility-related fields explicit so later converters can normalize MathML without losing source identity.
