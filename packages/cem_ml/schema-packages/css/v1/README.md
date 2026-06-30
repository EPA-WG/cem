# CSS schema package v1

This package defines the CEM schema identity for CSS stylesheets and scoped style content.

- Schema URL: `https://cem.dev/ns/data/css/1`
- Primary content type: `text/css`
- Source schema: `schema/css.cem`

CSS source is not CEM-ML syntax. The package models stylesheet, style block, style attribute, rule, selector, declaration, and component-value structure for future parser and converter work.

Scoped style content is represented as metadata on style blocks and style attributes. The scope can point at an HTML, SVG, MathML, custom-element, or shadow-root host without changing the `text/css` content identity.

## Validation

Validate CSS resources through the CLI with the schema URL and content type:

```bash
cem-ml validate --format json \
  --content-type text/css \
  --schema https://cem.dev/ns/data/css/1 \
  packages/cem_ml/schema-packages/css/v1/examples/basic-stylesheet.css
```

The direct validator scans CSS syntax without fetching or executing anything.
It accepts stylesheet and declaration-list shaped scoped style content, reports
charset conflicts as warnings, rejects `@import` and external `url()` references
without an explicit resolver/sanitizer policy, and surfaces token/declaration
recovery diagnostics.

## Examples

- [basic-stylesheet.css](examples/basic-stylesheet.css): a complete stylesheet
  with `@charset`, custom properties, and ordinary rules.
- [scoped-component.css](examples/scoped-component.css): scoped component style
  content using layer and scope boundaries.
- [style-attribute.css](examples/style-attribute.css): declaration-list shaped
  style attribute content carried as `text/css`.
- [invalid-import.css](examples/invalid-import.css): rejected `@import`
  resolver access.
- [invalid-url.css](examples/invalid-url.css): rejected external `url()`
  reference.
- [invalid-token.css](examples/invalid-token.css): unclosed block token.
- [invalid-declaration.css](examples/invalid-declaration.css): warning-only
  recovered declaration.
- [encoding-conflict.css](examples/encoding-conflict.css): warning-only MIME
  charset and `@charset` conflict.
