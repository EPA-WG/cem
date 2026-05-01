# CEM DOM Library Plan

## Package Name

Use `@epa-wg/cem-dom` for the XML/HTML/XSLT DOM library.

Rationale:

- It stays under the existing `@epa-wg` package scope.
- It is distinct from `@epa-wg/cem-theme` and `@epa-wg/cem-components`.
- It describes the package responsibility directly: parse, normalize, query, validate, and transform CEM semantic DOM
  documents.
- It leaves room for future packages such as `@epa-wg/cem-fixtures` or `@epa-wg/cem-adapters` without overloading the
  DOM package.

## Initial Responsibility

`@epa-wg/cem-dom` should own:

- Typed semantic document nodes for screens, regions, forms, fields, lists, assets, profiles, and messages.
- Parser helpers for CEM XML/HTML into a normalized DOM model.
- Query helpers for semantic roles, state, validation messages, relationships, and labels.
- Validation reports for invalid state combinations, broken references, and missing accessible names.
- XSLT transform helpers from semantic fixtures into light-DOM custom-element markup.

## Non-Goals

- It does not own visual styling; generated token CSS remains in `@epa-wg/cem-theme`.
- It does not own component implementations; rendered custom elements remain in `@epa-wg/cem-components`.
- It does not create runtime client-side behavior for applications that are meant to stay declarative.
