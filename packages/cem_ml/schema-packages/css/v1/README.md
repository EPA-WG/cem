# CSS schema package v1

This package defines the CEM schema identity for CSS stylesheets and scoped style content.

- Schema URL: `https://cem.dev/ns/data/css/1`
- Primary content type: `text/css`
- Source schema: `schema/css.cem`

CSS source is not CEM-ML syntax. The package models stylesheet, style block, style attribute, rule, selector, declaration, and component-value structure for future parser and converter work.

Scoped style content is represented as metadata on style blocks and style attributes. The scope can point at an HTML, SVG, MathML, custom-element, or shadow-root host without changing the `text/css` content identity.
