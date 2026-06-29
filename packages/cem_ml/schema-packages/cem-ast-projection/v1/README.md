# CEM AST Projection Schema

This package defines the semantic CEM AST projection layer:

- schema URL: `https://cem.dev/ns/projection/ast/1`
- primary content type: `application/vnd.cem.ast+cem-bin`
- debug/interchange view: `application/vnd.cem.ast+json`

The current JSON AST export keeps this schema identity but is treated as a view
over the semantic AST projection, not as the canonical runtime artifact.
