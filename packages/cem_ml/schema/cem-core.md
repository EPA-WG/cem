# CEM Core Schema (`https://cem.dev/ns/core/1`)

**Status:** Tier A active schema covering the vocabulary used by the canonical
`examples/cem-ml/*.cem` fixtures. Cross-reference: component IDs in
[`../../../docs/component-mvp.md`](../../../docs/component-mvp.md); state names
in the same document's State Matrix.

This is the source-of-truth markdown for the Tier A CEM schema. The Rust
schema compiler (`packages/cem_ml/src/schema/vocab.rs`) currently constructs
the equivalent `CompiledSchema` programmatically; the markdown→compiled-IR
pipeline (`schema::compiler`) lands alongside the markdown→XHTML docs
pipeline in a follow-up.

Conventions:

- `cem:` is the active namespace prefix; the bound namespace URI is
  `https://cem.dev/ns/core/1`.
- All annotations are **schema-qualified attribute names** in the CEM
  namespace, not HTML `data-*` attributes (AC-S-9).
- An annotation attaches a CEM semantic role to a host element (HTML or
  SVG); the host element keeps its native DOM identity.
- Values that look like identifiers are case-sensitive enum values.

## Annotation Vocabulary

| Annotation       | Host hint               | Value shape           | Tier A enum (where applicable) |
| ---------------- | ----------------------- | --------------------- | ------------------------------ |
| `cem:screen`     | `main` / page landmark  | screen id (string)    | `login`, `registration`, `profile`, `assets`, `message-thread` |
| `cem:form`       | `form`                  | form id (string)      | `sign-in`, `registration`, `asset-filter`, `profile-preferences`, `message-reply` |
| `cem:action`     | `button` / interactive  | intent (enum)         | `primary`, `secondary` |
| `cem:badge`      | inline (`span`, `mark`) | tone (enum)           | `success`, `info`, `warning`, `error` |
| `cem:card`       | `section` / `article`   | card id (string)      | `identity`, `preferences`, `summary` |
| `cem:list`       | `ul` / `ol` / `dl`      | list id (string)      | `assets`, `results`, `notifications` |
| `cem:row`        | `li` / `tr`             | row id (string)       | `asset`, `result`, `notification` |
| `cem:thread`     | `ol` / `ul`             | thread id (string)    | `support`, `notifications` |
| `cem:message`    | `li`                    | role (enum)           | `sent`, `received` |

The string-valued annotations (`screen`, `form`, `card`, `list`, `row`,
`thread`) accept any kebab-case identifier today; the Tier A enum column
lists the values produced by the existing five fixtures, which the schema
compiler can also expose as `known_values` for autocomplete and tooling.

## State Attribute Set

State attributes attach to the same host element as the CEM annotation.
Allowed state names are drawn from the State Matrix in
[`component-mvp.md`](../../../docs/component-mvp.md):

`default`, `hover`, `focus-visible`, `active`, `selected`, `disabled`,
`invalid`, `required`, `loading`, `empty`.

| Annotation       | Allowed states                                                                       |
| ---------------- | ------------------------------------------------------------------------------------ |
| `cem:screen`     | `default`, `loading`, `empty`                                                        |
| `cem:form`       | `default`, `disabled`, `invalid`, `loading`                                          |
| `cem:action`     | `default`, `hover`, `focus-visible`, `active`, `disabled`, `loading`                 |
| `cem:badge`      | `default`                                                                            |
| `cem:card`       | `default`, `selected`, `loading`, `empty`                                            |
| `cem:list`       | `default`, `loading`, `empty`                                                        |
| `cem:row`        | `default`, `hover`, `focus-visible`, `selected`, `disabled`                          |
| `cem:thread`     | `default`, `loading`, `empty`                                                        |
| `cem:message`    | `default`                                                                            |

State is exposed via a `cem:state` attribute on the same host node, with the
value being a single state name or a space-separated list of state names.
For example: `<button cem:action="primary" cem:state="loading">`.

States that imply ARIA reflection (e.g. `disabled` → `aria-disabled`,
`invalid` → `aria-invalid`) are accessibility-mirror behavior owned by the
content-type transform layer, not the schema. The schema validates which
states a given annotation accepts; ARIA reflection is enforced in the
semantic-rule catalog.

## Schema-Owned Diagnostics (Tier A)

| Code                                      | Severity | Trigger |
| ----------------------------------------- | -------- | ------- |
| `cem.schema.unknown_annotation`           | Error    | An attribute in the `cem:` namespace is not in the table above. |
| `cem.schema.unknown_annotation_value`     | Error    | Annotation value is not in the Tier A enum for an enum-valued annotation (e.g. `cem:action=bogus`). |
| `cem.schema.disallowed_state`             | Error    | `cem:state="..."` carries a state name not in the State Matrix. |
| `cem.schema.state_not_allowed_for_role`   | Error    | A state is in the State Matrix but is not allowed for the active annotation per the table above. |
| `cem.schema.unsupported_constraint`       | Error    | A schema rule requires unbounded buffering or another non-streamable feature. None in Tier A; emitted at schema compile if a rule is later added that would require it. |

## Non-Streamable Features

Tier A intentionally ships only streamable rules: every diagnostic the schema
emits is decidable from the current `SchemaFrame` plus the inbound
`NormalizedEvent`. Specifically the following remain **out of scope** and
are gated at schema-compile time with `cem.schema.unsupported_constraint`:

- Attribute-order constraints between non-adjacent attributes (per
  `cem-ml-stack-design-impl.md` §3.4 last paragraph).
- Cross-document or full-document-scoped constraints requiring a fully
  materialized AST before any diagnostic can be emitted.
- Predicate validation that needs to compare child counts across the entire
  document rather than within the current scope's content state.

## Composition Notes

- The CEM annotation does not replace the host node's native DOM meaning;
  the `cem:` attribute layers semantic role + state on top of the HTML/SVG
  element.
- Multiple annotations may attach to one host node (e.g. a `<section>` may
  carry both `cem:card` and `cem:screen` when the document is constructed
  as a single-card screen); the schema compiler resolves precedence via
  `CemAnnotationKind` ordering in `packages/cem_ml/src/parser.rs`.
- Annotations may carry a free-form `id` string value. The compiler does
  not enforce uniqueness in Tier A; downstream semantic rules
  (`cem-ml-ac.md` §V) cover reference integrity.
