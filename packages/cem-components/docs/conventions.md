# CEM Component Conventions

**Status:** Phase 3, item 1 of [`docs/todo.md`](../../../docs/todo.md). Pairs with
[`light-dom-rendering.md`](./light-dom-rendering.md) and [`accessibility.md`](./accessibility.md).
Component vocabulary and state matrix live in [`docs/component-mvp.md`](../../../docs/component-mvp.md).

These conventions govern every component in `@epa-wg/cem-components`. They define the
host-API contract that downstream packages (theme, ML transform output, Figma kit) can
rely on, and they are the contract that the Phase 3 test harness (item 4) and primitive
set (item 5) MUST satisfy.

## Scope

- This document is normative for `@epa-wg/cem-components`.
- Components are declarative custom elements rendered into the **light DOM**. No shadow
  DOM. Templates are authored against the `<cem-element>` substrate from
  `@epa-wg/cem-elements` (design home:
  [`docs/cem-element-design.md`](../../../docs/cem-element-design.md)). `<cem-element>`
  is the functional successor to `<custom-element>` from `@epa-wg/custom-element`;
  during the bridge window it accepts the legacy surface under
  `<template lang="custom-element-v0">`. New code MUST use the cem-ml/cem-ql surface.
  Rendering rules live in [`light-dom-rendering.md`](./light-dom-rendering.md).
- Accessibility behaviors are normative under [`accessibility.md`](./accessibility.md).
  This document handles the host-API surface only; a11y wording stays there.

## 1. Naming

### 1.1 Element names

- All component tag names use the `cem-` prefix: `cem-button`, `cem-text-field`,
  `cem-app-shell`, `cem-message-thread`. The prefix is the WHATWG custom-element
  reserved namespace for this package and MUST NOT be reused by adapters.
- Multi-word names use a single hyphen between words and stay lowercase.
- Composite components SHOULD use a parent-child naming pattern when the children are
  declarative slots, not separate widgets: `cem-data-list` + `cem-data-list-row`,
  `cem-message-thread` + `cem-message-thread-item`.
- A component never aliases another component name. There is at most one element name
  per row in [`component-mvp.md` §Component List](../../../docs/component-mvp.md).

### 1.2 Attribute names

- Attributes are lowercase, hyphen-separated, and align with WHATWG conventions
  (`aria-*`, `data-*`, `name`, `value`, `disabled`).
- Boolean attributes use the WHATWG presence convention: present = true, absent = false.
  `disabled`, `required`, `readonly`, `busy`, `selected`, `expanded` MUST follow this
  rule. They MUST NOT be set to the string `"false"` to mean false; remove the attribute.
- Enum attributes use a single hyphenated token: `variant="primary"`,
  `intent="destructive"`, `tone="quiet"`. The accepted token set per component is
  enumerated in that component's docs.
- A component MUST NOT introduce a new attribute name that already exists in WHATWG
  HTML with different semantics. Reuse the existing attribute (`name`, `value`,
  `disabled`, `for`, `placeholder`, `autocomplete`) and inherit its semantics.

### 1.3 CSS hooks

- Components expose state to CSS via reflected attributes (`data-state`,
  `aria-busy`, `aria-invalid`, etc.) and via CEM token CSS custom properties from
  `@epa-wg/cem-theme`. They MUST NOT expose state via private class names like
  `.cem-button-is-hover`.
- Component-internal CSS lives in the same light-DOM stylesheet as the page, so all
  rules MUST be scoped via the element selector (`cem-button`, `cem-button[disabled]`)
  rather than a global class.

## 2. Attributes & Properties

### 2.1 Attribute is the source of truth

Components are declarative: the rendered output is a pure function of attributes,
captured payload, slices, and the static template. The attribute is canonical. A
property setter, when provided, MUST mirror through to the attribute so that the
source-of-truth invariant holds across reads, serialization, and re-hydration. This
mirrors the contract `@epa-wg/custom-element` already provides for
`attribute name="..."` declarations.

### 2.2 Declared vs. host attributes

- A **declared attribute** is one the component reads (`name`, `value`, `variant`,
  `busy`). It is named in the component's template `attribute` declarations and is
  documented in that component's reference.
- A **host attribute** is any attribute the framework supplies and the component
  forwards verbatim (`id`, `class`, `style`, `data-*`, `slot`, `lang`, `dir`,
  `tabindex`, `aria-*`). Components MUST forward host attributes onto the rendered
  light-DOM children when the attribute has a clear target; rules are in
  [`light-dom-rendering.md §3`](./light-dom-rendering.md).
- A component MUST NOT silently swallow `data-*` attributes. They reach the host
  element and remain queryable by the parent application.

### 2.3 Default values

- Defaults are declared in the template via `attribute` default text, not via setter
  side effects. This keeps re-renders deterministic and source-mappable.
- A default value MUST be valid input for the same attribute setter; an attribute
  that defaults to `"medium"` MUST accept `"medium"` as an explicit user-provided
  value with the same result.

### 2.4 Reflected state

Components reflect interaction and validation state to attributes so CSS, queries,
and ARIA computations can observe it:

| Reflected attribute | Set when |
| --- | --- |
| `data-state="loading"` | Async operation pending (per AC-V-6 loading state). |
| `data-state="empty"` | List/thread/asset component has zero items. |
| `aria-busy="true"` | Component is mid-update and not safe to interact with. |
| `aria-invalid="true"` | Form field failed validation. |
| `aria-disabled="true"` | Mirrors the `disabled` attribute on non-form components. |
| `aria-expanded="true"` | Disclosure or popover open. |
| `aria-selected="true"` | List/nav row currently selected. |

These are the **only** attributes a component is allowed to set on itself for state.
A component MUST NOT set `class` to track state.

## 3. Events

### 3.1 Event names

- All component-defined events use the `cem-` prefix and lower-kebab-case:
  `cem-change`, `cem-submit`, `cem-select`, `cem-dismiss`, `cem-loaded`,
  `cem-error`.
- A component MUST reuse the matching WHATWG event when one exists with the same
  semantics: `input`, `change`, `focus`, `blur`, `submit` (when participating in a
  form), `click`. The `cem-` prefix is reserved for events whose payload or semantics
  differ from the WHATWG counterpart.
- Events bubble and compose by default (`bubbles: true, composed: true`) so they
  cross declarative-component boundaries cleanly. They MAY be cancellable; cancellation
  semantics are documented per component.

### 3.2 Event payload

- The payload is a plain object on `event.detail`. It MUST be JSON-serializable so
  CEM AST and DOM-stream snapshots can round-trip it.
- The payload MUST include enough state to drive a declarative re-render of the
  caller. For `cem-change`, payload is `{ name, value, valid }`. For `cem-select`,
  payload is `{ value, index }`. For `cem-error`, payload is
  `{ code, message, severity }` matching the CEM diagnostic shape.
- Payload field names follow camelCase (matches `event.detail` convention).

### 3.3 No imperative-only events

Every event a component dispatches MUST be observable from a declarative
`<cem-element>` `slice-event="..."` binding, and during the bridge window from the
compatible `@epa-wg/custom-element` binding. If a behavior cannot be expressed
through a documented event, it does not belong in a component.

## 4. Form Participation

Form components (`cem-text-field`, `cem-select-field`, `cem-checkbox`, `cem-form`)
MUST participate in WHATWG form-associated custom elements:

- `static formAssociated = true` so the component shows up in the implicit
  `HTMLFormElement.elements` collection.
- `name` and `value` attributes follow WHATWG semantics. The component contributes
  its `value` to `FormData` on submit.
- The component MUST expose `validity` (a `ValidityState`-shaped object) and forward
  it through `ElementInternals` when the runtime supports it.
- A component without a `name` attribute does NOT contribute to `FormData`. This is
  the documented opt-out path.
- `disabled` and `readonly` follow WHATWG semantics: disabled fields do not submit
  and do not receive focus; readonly fields submit and receive focus but reject
  edits.
- Reset: components MUST listen for the host form's `reset` and restore their
  template-declared default value, not the runtime DOM value at the time of
  submit.

## 5. Validation

### 5.1 Surface

Components surface validation through three coordinated channels:

1. The `validity` object (programmatic / `ElementInternals`).
2. `aria-invalid` reflected attribute (a11y assistive tech).
3. A `cem-invalid` / `cem-valid` event pair with payload `{ name, value, code, message }`.

These channels MUST agree at all times. A component is never `aria-invalid="true"`
without a corresponding `validity` failure and a `cem-invalid` event having been
dispatched (or about to be dispatched in the same microtask).

### 5.2 Diagnostic shape

Component validation diagnostics use the same shape as `cem_ml` diagnostics:
`{ code, severity, message, node?, uri?, line?, column?, byteOffset? }`. The
`code` MUST be a stable string under the `cem.component.*` namespace, e.g.
`cem.component.required_missing`, `cem.component.value_out_of_range`. Severity is
`info | warning | error | fatal`.

### 5.3 Error message authority

Components do NOT hard-code user-facing strings. The error message comes from one of:

- A slotted `<cem-error-message for="...">` child the author provided.
- A schema-owned message resolved by `cem_ml` semantic validation (catalog landed in
  Phase 2). When `cem_ml` reports a hard violation on a form field, the component
  reflects it instead of inventing a parallel message.
- The browser's built-in `validationMessage` as the documented fallback.

## 6. Loading States

Components MUST treat asynchronous work as a first-class state:

- While the work is pending: set `data-state="loading"` and `aria-busy="true"`.
  Preserve layout dimensions; do not collapse to zero size.
- On success: clear both attributes and dispatch the component's success event
  (`cem-loaded`, `cem-change`, `cem-submit`, etc. as appropriate).
- On failure: clear `data-state="loading"` but set `aria-invalid="true"` (for form
  components) or `data-state="error"` (for non-form components), and dispatch
  `cem-error` with the diagnostic payload from §5.2.
- On cancellation (`AbortSignal` aborted): clear `data-state="loading"`, do not set
  invalid/error state, and dispatch `cem-cancel` with `{ name, reason }`.

The `cem_ml` async API (AC-A-1..AC-A-7) is the source of truth for cancellation
semantics. Components MUST accept an `AbortSignal` via property when they perform
async work directly.

## 7. Progressive Enhancement

Every component MUST degrade gracefully when its custom element is not upgraded yet
(JS not loaded, polyfill blocked, or transform/render is server-side only):

- The light-DOM children that the author wrote are the **fallback rendering** before
  upgrade. The page MUST remain readable, navigable, and form-submittable without
  the upgrade running.
- During upgrade, the substrate captures author-supplied children into the
  component instance's `<template data-cem-island="instance">` and replaces the
  visible content with the rendered projection. The raw payload remains associated
  with the component scope as data and MUST NOT affect layout, selectors, form
  submission, or accessibility directly after upgrade.
- A `cem-` prefix on an element is a signal to the styling layer that the element
  exists; cem-theme CSS uses element selectors (`cem-button { … }`) so unstyled
  fallback still picks up theme tokens.
- A component MAY render additional decorative children (icons, separators) only
  when upgraded. Those children MUST NOT carry semantic content; semantic content
  comes from the author's light-DOM input.
- Form components, when not upgraded, behave as their nearest WHATWG analogue:
  `cem-text-field` renders a working `<input>` from its author children;
  `cem-button` renders a working `<button>`. This is the form-submission fallback
  expected by the Phase 3 accessibility contract.

## 8. Compatibility expectations

- `@epa-wg/cem-elements` (the `<cem-element>` substrate) is the host runtime.
  Components rely on its declarative template, data island, and slice-event binding.
  Templates use cem-ml syntax with cem-ql expressions; declaration data and upgraded
  instance payload sit inside `<template>` data islands so they are inert to the
  browser rendering engine.
  Imperative state machines that bypass the declarative slice surface are forbidden
  in this package.
- The legacy `<custom-element>` surface from `@epa-wg/custom-element` remains
  consumable through the bridge-window compat (see Scope and cem-element-design §6.2)
  but new primitives MUST author directly against `<cem-element>`. After the major
  substrate adoption, `<custom-element>` remains the published
  `@epa-wg/custom-element` tag and inherits the `cem-element` implementation.
- Tokens come from `@epa-wg/cem-theme`. Components MUST NOT define their own color
  or spacing literals; they reference CEM token CSS custom properties.
- AST-to-light-DOM transforms are owned by `cem_ml` and produce output that already
  conforms to these conventions. Components are the consumer side of that contract.

## 9. AC and design references

- [`docs/cem-element-design.md`](../../../docs/cem-element-design.md) — `<cem-element>`
  substrate design (data island, template engine, migration plan, parity criteria).
- [`docs/cem-ml-ac.md`](../../../docs/cem-ml-ac.md) — AC-F-5 (reference slots),
  AC-V-6 (validation diagnostics, loading state, accessible names), AC-I-6 (WHATWG
  HTML DOM compliance as a transform), AC-A-1..AC-A-7 (async + cancellation).
- [`docs/cem-ql-ac.md`](../../../docs/cem-ql-ac.md) — CEM-QL surface, the expression
  language used inside `<cem-element>` templates.
- [`docs/component-mvp.md`](../../../docs/component-mvp.md) — component list and
  state matrix; this document refines the host-API contract for every row.
- [`docs/roadmap.md`](../../../roadmap.md) §Phase 3 — runtime preparation goals,
  split into 3.1 substrate (`@epa-wg/cem-elements`) and 3.2 primitives.
- `@epa-wg/custom-element` POC (`~/aWork/custom-element/`) — functional reference
  for declarative templating, attribute declarations, and `slice` events. Scheduled
  for monorepo migration to `packages/custom-element/`; treat as functional
  reference per [`CLAUDE.md`](../../../CLAUDE.md) §custom-element legacy info, not
  as a decision authority for component syntax.
- `~/aWork/custom-element-dist/src/material/` — material-style sample components
  used as the parity benchmark for the `<cem-element>` substrate (action,
  autocomplete, badge, dropdown, icon, icon-link, input, menu).
