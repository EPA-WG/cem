# CEM Schema

Vocabulary reference for CEM semantic HTML documents. CEM semantic markup uses
`data-cem-*` attributes to annotate standard HTML elements with structural and
interaction roles. The parser, validator, and transformer in `@epa-wg/cem-dom`
read these attributes as the canonical role signal.

## Namespace

All CEM semantic attributes share the `data-cem-` prefix. The attribute name
encodes the semantic role; the attribute value carries the identifier or variant.

| Attribute | Value type | Applied to |
| --------- | ---------- | ---------- |
| `data-cem-screen` | identifier string | `<main>` or landmark element |
| `data-cem-form` | identifier string | `<form>` |
| `data-cem-action` | variant string | `<button>` or `<a>` |
| `data-cem-card` | identifier string | `<section>` or `<article>` |
| `data-cem-badge` | variant string | `<span>` or inline element |
| `data-cem-list` | identifier string | `<ul>`, `<ol>` |
| `data-cem-row` | identifier string | `<li>` |
| `data-cem-thread` | identifier string | `<ol>` |
| `data-cem-message` | variant string | `<li>` |

### Identifier roles

These roles carry a caller-assigned string identifier as their value. The value
names the semantic instance (e.g. `data-cem-screen="login"`).

- `data-cem-screen` — top-level view container. One per document.
- `data-cem-form` — user-input form with a stated intent.
- `data-cem-card` — summary container for a named data subject.
- `data-cem-list` — homogeneous row list.
- `data-cem-row` — single item within a `data-cem-list`.
- `data-cem-thread` — ordered list of conversational messages.

### Variant roles

These roles carry a semantic variant as their value. The value comes from a
closed vocabulary defined below.

- `data-cem-action` — interactive trigger. Valid variants: `primary`, `secondary`,
  `destructive`, `quiet`.
- `data-cem-badge` — inline status label. Valid variants: `success`, `info`,
  `warning`, `error`.
- `data-cem-message` — message in a thread. Valid variants: `sent`, `received`.

## Document structure rules

A well-formed CEM document:

1. Has exactly one `data-cem-screen` element at the top of the content area.
2. Provides `aria-labelledby` or `aria-label` on `data-cem-screen`.
3. Places `data-cem-form` within a `data-cem-screen` or `data-cem-card`.
4. Uses `data-cem-action` only on `<button>` or `<a>` elements.
5. Uses `data-cem-badge` only on inline elements (`<span>`, `<em>`, etc.).
6. Pairs each `data-cem-list` with `data-cem-row` children.
7. Pairs `data-cem-thread` with `data-cem-message` children.

## State attributes

CEM reuses native HTML and ARIA attributes for interactive state rather than
introducing new `data-cem-*` state attributes. The table below maps each state
from the CEM component MVP to the corresponding HTML/ARIA expression.

| State | HTML/ARIA expression |
| ----- | -------------------- |
| default | (none) |
| hover | CSS `:hover` pseudo-class |
| focus-visible | CSS `:focus-visible` pseudo-class |
| active | CSS `:active` pseudo-class |
| selected | `aria-selected="true"` or CSS `:checked` |
| disabled | `disabled` attribute or `aria-disabled="true"` |
| invalid | `aria-invalid="true"` or CSS `:invalid` |
| required | `required` attribute or `aria-required="true"` |
| loading | `aria-busy="true"` |
| empty | (component-specific — no universal HTML mapping) |

## Accessible name requirements

The validator enforces accessible names on the following elements when present
in a CEM document:

| Element / role | Required accessible name source |
| -------------- | ------------------------------- |
| `data-cem-screen` | `aria-labelledby` or `aria-label` |
| `data-cem-action` | Text content, `aria-label`, or `aria-labelledby` |
| `<input>` (visible) | `<label for>`, wrapping `<label>`, `aria-label`, or `aria-labelledby` |
| `<textarea>` | `<label for>`, wrapping `<label>`, `aria-label`, or `aria-labelledby` |
| `<select>` | `<label for>`, wrapping `<label>`, `aria-label`, or `aria-labelledby` |

## Transform output

The `transform()` function maps CEM semantic markup to custom-element markup
using the following element name convention:

```
data-cem-{role} element   →   cem-{role} custom element
```

Identifier roles receive a `cem-id` attribute; variant roles receive a
`variant` attribute. All other attributes on the source element pass through
unchanged.

Example — `login.html` before and after transform:

```html
<!-- Before -->
<main data-cem-screen="login" aria-labelledby="login-title">
  <form data-cem-form="sign-in" method="post" action="/session">
    <button type="submit" data-cem-action="primary">Sign in</button>
  </form>
</main>

<!-- After -->
<cem-screen cem-id="login" aria-labelledby="login-title">
  <cem-form cem-id="sign-in" method="post" action="/session">
    <cem-action variant="primary" type="submit">Sign in</cem-action>
  </cem-form>
</cem-screen>
```

## Validation rules

| Rule code | Severity | Condition |
| --------- | -------- | --------- |
| `missing-screen-label` | warning | `data-cem-screen` has no `aria-labelledby` or `aria-label` |
| `broken-aria-ref` | error | `aria-labelledby`/`aria-describedby` value not found in document `id` map |
| `broken-for-ref` | error | `<label for>` value not found in document `id` map |
| `invalid-action-variant` | error | `data-cem-action` value not in `primary \| secondary \| destructive \| quiet` |
| `invalid-badge-variant` | error | `data-cem-badge` value not in `success \| info \| warning \| error` |
| `invalid-message-variant` | error | `data-cem-message` value not in `sent \| received` |
| `missing-accessible-name` | error | `data-cem-action` or visible form control has no accessible name |
| `unknown-cem-attr` | warning | `data-cem-*` attribute not in the known vocabulary |
