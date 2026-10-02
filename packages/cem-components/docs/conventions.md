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
- The primary source/layout/testing rule is
  [`declarative-ui-principle.md`](../../../docs/declarative-ui-principle.md).
  Every component is an XHTML CEM-ML declaration in its own `src/components/<cem-tag>/`
  folder. Its `<template id="<cem-tag>" type="text/cem-ml">` keeps the template
  reusable through `<declaration-url>#<cem-tag>` as part of the component
  structure. It contains an embedded CEM-ML `<style>` node using CEM UI `--cem-*` tokens
  and a colocated CSF Next `.stories.ts` test module. The story imports its own
  XHTML as raw source, uses the shared `cem-elements` loader, returns example
  HTML from `render`, and keeps unit assertions in async `play` functions using
  `storybook/test`. It is the only per-component TypeScript exception and must
  not contain production behavior. `cem-elements` installs each static embedded
  style once per declaration. The accepted CSS target uses native `@scope`, the
  produced tag for private rules, declaration-owned `scope="name"` for named
  shared rules, `slot="name"` for projected roots, and `part="name"` for
  component-owned internals. Per-instance CSS requires an explicit inert direct
  payload template. Generated render and data-island identity are never public
  CSS hooks. Therefore
  standalone component CSS and migrated selectors in global CSS are forbidden.
  If the required expression is missing or unreasonably verbose in CEM-ML, stop
  and implement a reusable declarative capability in `cem-elements` before
  continuing the component. The proven pattern is `src/components/cem-select/`.
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
  `disabled`, `required`, `readonly`, `busy`, `selected`, `expanded`, `selectable`,
  `collapsible`, `empty`, `checkable`, and `checked` MUST follow this rule. They MUST NOT be set to the string `"false"`
  to mean false; remove the attribute. The compatibility exception is
  `cem-action`'s existing `expanded="true|false"` forwarding to `aria-expanded`;
  it reports a disclosure state and is not a selection boolean.
- Enum attributes use a single hyphenated token: `variant="primary"`,
  `intent="destructive"`, `tone="quiet"`. The accepted token set per component is
  enumerated in that component's docs.
- A component MUST NOT repurpose a WHATWG global attribute. It SHOULD reuse the
  semantics of applicable standard attributes (`name`, `value`, `disabled`,
  `for`, `placeholder`, `autocomplete`). An attribute that WHATWG limits to a
  different element MAY be used as a documented autonomous-custom-element API
  only when the contract prevents that element from matching generated behavior.
  The CSS scope contract's declaration-owned `scope` is the sole current case:
  generated roots also require the direct CEM instance data island, so native
  `<th scope>` semantics remain untouched.

### Native visibility

`hidden` is a standard HTML global attribute on the component host, not a
component-specific state API. Put it on `<cem-action>`, `<cem-select>`, or another
component host to hide the whole component. Ordinary hidden content occupies no
layout space and its controls leave keyboard navigation and accessibility exposure.

- Use `hidden`, `hidden=""` or `hidden="hidden"` to hide. Remove the attribute to
  reveal; `hidden="false"` still hides. Do not bind a false string to mean visible.
- `hidden="until-found"` is a distinct, case-insensitive native state. Keep the
  browser's content-visibility and find/fragment reveal behavior; do not replace
  it with `display: none` or a CEM reveal handler. No polyfill is supplied.
- Component host layout CSS must respect ordinary hidden values, including before
  and after rendering. Canonical components use the scoped rule below so their
  default display declarations do not override native hiding.
- Do not forward the host attribute to internal controls or redeclare it with a
  component default. Hiding preserves the instance, control identities and values;
  it does not disable controls or remove them from form submission.
- `hidden` does not promise a fade animation. CSS `visibility: hidden` preserves
  layout space. `aria-hidden="true"` affects accessibility exposure without hiding
  content visually or preventing keyboard focus. These are different behaviors.
- `invisible` is not a CEM visibility API. New components and migrations use this
  shared convention; legacy `invisible` demonstrations are removed without an alias.

```css
:scope[hidden]:where(:not([hidden="until-found" i])) {
    display: none;
}
```

See the [HTML hidden specification](https://html.spec.whatwg.org/multipage/interaction.html#the-hidden-attribute).

### 1.3 CSS hooks

- Components expose state to CSS via reflected attributes (`data-state`,
  `aria-busy`, `aria-invalid`, etc.) and via CEM token CSS custom properties from
  `@epa-wg/cem-theme`. They MUST NOT expose state via private class names like
  `.cem-button-is-hover`.
- Component-internal CSS lives in the declaration's embedded static `<style>`.
  The accepted compiler target rewrites `:host` to `:where(:scope)` inside a
  native tag-rooted `@scope`; it MUST NOT translate `:host` to `&`. Named group
  rules require one matching declaration/style `scope`. Component-generated
  internals expose stable `part` tokens, and projected element roots retain
  their `slot` name. Library selectors stay at or below `0-2-1`, without IDs,
  manufactured specificity, generated layers, or `!important`. Do not target
  internal data-island/render identity or put component rules in the
  package-global stylesheet. The complete ownership, resolution matrix,
  instance payload, specificity, and fail-closed rules are normative in
  [CEM light-DOM CSS scoping rules](../../../docs/cem-ml-uid-and-scoped-css-design.md).
  Treat that contract as the implemented authoring and runtime baseline.
- Existing semantic CEM tokens have priority over raw or component-local values.
  If no token can express a requirement, stop and warn, then record the proposed
  exception in [`components-css-exceptions.md`](./components-css-exceptions.md)
  for analysis, categorization, and possible adoption into `@epa-wg/cem-theme`.
  The review queue is not a verifier allowlist.

### 1.4 Public CSS property ownership

`--cem-<component>-*` properties are public styling API. Containers may override
these properties through inheritance, and component styling may deliberately
bind them for an explicit variation. An `internal` name segment does not make a
property private or exempt it from this contract.

Theme-owned properties retain their canonical definitions in `cem-theme`.
Components consume existing semantic tokens directly; they MUST NOT duplicate
theme defaults or add equivalent aliases that mask inherited overrides.
Intent/state selection should select the appropriate theme endpoint rather than
redeclare its value on the component.

Theme variables are generated from canonical Markdown source tables. Do not
remove them because current CEM components do not reference them: consumer
components and inherited themes may depend on them. CEM theme and component
releases retain the full output selected by the manifest tier contract. Any
future unused-code optimization belongs to an opt-in consumer build, where the
consumer's complete usage is available; it must not prune CEM release artifacts.

When a component definition conflicts with or duplicates a theme definition,
STOP for an explicit ownership decision: adopt the theme definition, or adopt
the component definition into the theme and remove it from the component.
Do not resolve the conflict by silently renaming, creating a parallel token
family, or labeling the property internal. Missing semantic values still follow
the CSS exception review above; existing bounded exceptions are not permission
to create new public properties.

For action, the accepted decision is to consume
`--cem-action-<intent>-<state>-{background,text}` directly. Generic
`--cem-action-<state>-*` aliases are not introduced.
See the [theme ownership contract](../../cem-theme/src/lib/tokens/index.md#public-css-property-ownership).

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
- Declaration-owned `scope` is a public host styling-membership attribute, not a
  forwarded content attribute. It remains on the produced DCE host, and the
  runtime restores declaration state if an instance adds, mutates, or removes it.

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
| `data-state="empty"` | An explicit settled empty state is active. `cem-surface[empty]` mirrors it on the rendered section only when `busy` is absent; stacks and grids do not infer it from child count. |
| `aria-busy="true"` | Component or named region is mid-update. `cem-surface[busy]` pairs it with `data-state="loading"` and takes rendered-state precedence over `empty`; this does not disable or make descendants inert. |
| `aria-invalid="true"` | Form field failed validation. |
| `aria-disabled="true"` | Mirrors the `disabled` attribute on non-form components. |
| `aria-expanded="true"` | Disclosure or popover open. |
| `aria-selected="true"` | Native selectable-list option or navigation row currently selected. Passive lists and static table rows do not expose selected state. |
| `aria-pressed="true|false"` | Checkable chip mirrors `checked`; selectable action reports container-owned `selected`. Ordinary command buttons omit it. |

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

Form components (`cem-text-field`, `cem-select`, `cem-checkbox`, `cem-form`)
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

Components that own asynchronous work MUST treat it as a first-class state:

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

State-projecting components do not inherit that resource lifecycle.
Presence-only `cem-card[busy]` marks its stable named section as locally updating
content, while `cem-surface[busy]` applies the same exact markers to a stable
named whole-workflow layout. Both retain authored payload, create no slice,
timer, request, lifecycle event, live region, or inert subtree, and leave request,
cancellation, control disabling, status feedback, and outcome selection with the
application or workflow. Surface `busy` takes precedence over settled `empty`
during ordered transitions so it never exposes simultaneous or intermediate
rendered states. Initial loading uses visible authored text and optional
layout-preserving `cem-skeleton`/`cem-progress` composition; background refresh
SHOULD retain last-known content and placement. See the
[content loading](./content-loading-contract.md) and
[layout loading](./layout-loading-contract.md) contracts.

The seven input primitives use the same presence-only projection boundary at
their interactive control: explicit host `busy` produces `data-state="loading"` and
`aria-busy="true"` without starting work or changing value, focus, dimensions,
or interaction. D0 pending color and D5 pending stroke thickness compose the
anchor cue without relying on hue alone. See the
[input loading contract](./input-loading-contract.md).

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
  but new primitives MUST author directly against `<cem-element>`. The major
  `@epa-wg/custom-element` substrate adoption is deferred until after the Edge/SSR
  follow-up phase; after that adoption, `<custom-element>` remains the published tag
  and inherits the `cem-element` implementation.
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
  for declarative templating, attribute declarations, and `slice` events. Monorepo
  migration is deferred until after the Edge/SSR follow-up phase; treat as functional
  reference per [`CLAUDE.md`](../../../CLAUDE.md) §custom-element legacy info, not as
  a decision authority for component syntax.
- `~/aWork/custom-element-dist/src/material/` — material-style sample components
  used as the parity benchmark for the `<cem-element>` substrate (action,
  autocomplete, badge, dropdown, icon, icon-link, input, menu).

## Playground theme control

Use the canonical `cem-theme-switch` declaration as the theme scope around a
playground's visible content, including the preview and source viewer. Load its
XHTML through the page-level source/package module maps. Keep declaration and
stylesheet loaders outside the projected content. Do not add page-local theme
JavaScript or duplicate its controls.

The switch reflects `mode` (Light/Dark/Native), `contrast` (the remembered
preference) and the effective `data-theme`. Native disables and unchecks the
Contrast control; returning to Light/Dark restores the preference. `mode` defaults
to `native`; playgrounds may supply `mode="light"`. Native uses system colors
and permits the browser's light/dark preference. Give independent switch scopes
distinct radio-group `name` values (default `cem-theme-mode`).

The default slot owns the themed content. Theme changes must preserve projected
controls, their edited values and nested component identities. Keep each source
viewer separate from the component's reactive property form, as in the action
playground. Persistent storage and page-global DOM mutation are not part of this
component's contract.

## Shared label principle

`label` supplies visible fallback text. Projected payload replaces that fallback
and preserves its markup. Components whose default slot represents their label
use `{slot | {$label}}`, following `cem-action`. Components whose default payload
has another purpose use a named `label` slot. Accessibility-only names use
`aria-label` separately.

`cem-icon` follows this principle with an empty default label. `cem-icon-button`
still uses `label` for accessibility-only naming; aligning it is tracked in
[`docs/todo.md`](../../../docs/todo.md).
