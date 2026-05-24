# CEM Light-DOM Rendering Rules

**Status:** Phase 3, item 2 of [`docs/todo.md`](../../../docs/todo.md). Pairs with
[`conventions.md`](./conventions.md) and [`accessibility.md`](./accessibility.md).

Every component in `@epa-wg/cem-components` renders into the **light DOM**. There is no
shadow DOM, no closed roots, no internal CSS encapsulation. This document fixes the
rules so the same fixture renders identically across the `cem_ml` transform pipeline,
the `<cem-element>` substrate upgrade (from `@epa-wg/cem-elements`), and the
unupgraded fallback. Substrate design lives in
[`docs/cem-element-design.md`](../../../docs/cem-element-design.md).

## 1. No shadow DOM

- Components MUST NOT call `attachShadow`. The host element's children, after upgrade,
  are visible in the page's document tree; they participate in selector matching,
  global CSS, accessibility tree, form-data submission, and DOM queries from the
  outside.
- Author-supplied children (slotted content, default text) are PRESERVED in place
  when the component upgrades. The component decorates them; it does not replace
  them. This matches the `<cem-element>` substrate's default light-DOM behavior
  (inherited from `@epa-wg/custom-element` and refined by the inert
  `<template>`-wrapped data island; see
  [`docs/cem-element-design.md §3`](../../../docs/cem-element-design.md)) and
  preserves the progressive-enhancement fallback documented in
  [`conventions.md §7`](./conventions.md).
- A component MAY append additional light-DOM children for decoration (icons,
  carets, status indicators). Those decorative children:
  - MUST be unambiguously distinguishable from author content (a documented
    attribute marker is fine; a `data-cem-decorative` attribute is recommended);
  - MUST be safe to remove without breaking the component's contract;
  - MUST NOT carry the component's accessible name, value, or validation state.

## 2. Template engine: `<cem-element>`

Components are authored as `<cem-element>` declarations from `@epa-wg/cem-elements`
(see [`docs/cem-element-design.md`](../../../docs/cem-element-design.md)). The
substrate replaces `<custom-element>` and its XSLT/XPath template engine with the
CEM-native parser/runtime (cem-ml templates, cem-ql expressions). The rules below
are the ones the cem-components package commits to.

### 2.1 The `<template>` wrapper

- Every `<cem-element>` body MUST sit inside a single direct-child `<template>`
  element. This is the data island; the browser parks its content in
  `template.content` and never renders it directly. Only the rendered output
  (driven from the data island) is visible.
- A component MUST NOT place declarations, slices, or render templates outside the
  `<template>` wrapper. The substrate ignores anything outside the wrapper.
- For the bridge-window only, a legacy POC body may be opted into via
  `<template lang="custom-element-v0">`; the substrate then accepts the
  XSLT/XPath surface as a compatibility path. New primitives MUST NOT use this
  opt-in.

### 2.2 cem-ql interpolation

- `${ expr }` in **text nodes** is interpreted as a CEM-QL expression and resolved
  against the data island (declared attributes, slices). This replaces the legacy
  XPath `{ … }` text interpolation.
- `{ expr }` in **attribute values** is the attribute-value template (AVT) span,
  also resolved against the data island. Attribute AVT carries cem-ql expressions
  per cem-ml AC-T-7.
- Bare `{ … }` text interpolation outside the AVT context is rejected by the Tier A
  CEM-ML tokenizer (`cem-ml-syntax.md`); the substrate enforces the same rejection.
- CSS literals containing `{` / `}` (CSS blocks) MUST be wrapped in a `<text>`
  element so the engine does not parse the braces as expressions. The cem-theme
  build pipeline already enforces this; component authors MUST follow the same rule
  when they include inline `<style>` regions inside the `<template>`.

### 2.3 Attribute declarations

- Attributes the component reads MUST be declared with `<attribute name="..." />`
  children inside the `<template>` wrapper. The declaration set IS the WHATWG
  `observedAttributes` list. Undeclared attributes are forwarded as host attributes
  per [`conventions.md §2.2`](./conventions.md) but do not trigger re-render.
- Default values go in the declaration body (text content). A `select="…"`
  attribute (a cem-ql expression) MAY bind the declared attribute to a slice or
  another attribute, replacing the legacy XPath `select`. Side-effect defaults in
  setters are forbidden — see [`conventions.md §2.3`](./conventions.md).

### 2.4 Slices and events

- `slice="..."` exposes a data-island field that templates can bind to with the
  cem-ql `$<slice-name>` form (or `.slice-name` in dotted form).
- `slice-event="..."` declares the DOM event that updates the slice. This is the
  declarative path required by [`conventions.md §3.3`](./conventions.md).
- `slice-value="..."` carries the cem-ql expression evaluated to compute the new
  slice value. Side-effect imperative handlers are not part of the cem-components
  surface.

## 3. Host-attribute forwarding

When a component renders a wrapper light-DOM element (e.g. `cem-button` renders a
`<button>` underneath, `cem-text-field` renders an `<input>`), it MUST forward host
attributes to a single, documented target element:

| Host attribute | Forwarded to |
| --- | --- |
| `id`, `name` | Inner interactive element. The host MAY keep its own `id` separately for ARIA wiring. |
| `class`, `style` | Host element (stays on the custom element). |
| `lang`, `dir` | Host element. |
| `tabindex` | Inner interactive element when one exists, host otherwise. |
| `aria-*` | Forwarded per [`accessibility.md §4`](./accessibility.md). |
| `data-*` | Host element. Authors expect to query these on the custom-element tag. |

A component MUST document which inner element is the forwarding target. The forwarding
target MUST NOT change between upgraded and not-yet-upgraded states (so progressive
enhancement keeps the same `id`/`name` bindings).

## 4. WHATWG DOM compliance (AC-I-6)

`cem_ml`'s AC-I-6 makes WHATWG HTML DOM compliance a schema-driven transform. Rendered
component output MUST be valid WHATWG HTML:

- Tag closing rules follow WHATWG. Void elements (`<input>`, `<img>`, `<br>`) MUST NOT
  carry text content even if the template tolerates it.
- ID uniqueness within a document is the author's responsibility, but components
  MUST NOT generate duplicate IDs from a single template invocation. Decorative
  children that need IDs MUST use a per-instance suffix (`${host.id}-icon`).
- Whitespace around author children is preserved verbatim. Components MUST NOT
  collapse text nodes; the `cem_ml` transform AST already encodes the canonical
  whitespace and feeds it through.
- A component's rendered output, fed back through `cem-ml convert --from-format html
  --to-format cem`, MUST round-trip to a canonical CEM-ML form. This is the
  cross-surface conversion contract from Phase 2.

## 5. Style scoping

Because there is no shadow DOM, styles affect — and are affected by — the page. Rules:

- Component styles MUST be authored against the element selector (`cem-button {…}`),
  not a `.cem-button` class. The cem-theme package follows the same rule.
- Components MUST NOT inject global stylesheets at runtime. Theme CSS is loaded once
  by the page; per-component styles ship as static stylesheets imported by the
  author.
- Component-internal layout MUST use CEM tokens. The list of tokens that apply per
  component lives in [`component-mvp.md` §Component List](../../../docs/component-mvp.md).
- A component MAY set CSS custom properties on itself to relay an attribute value
  into CSS (`--cem-button-loading-opacity`). It MUST NOT mutate other elements'
  custom properties.

## 6. Slot semantics

`@epa-wg/custom-element` does not use shadow-DOM `<slot>`. Cem-components use
**named author children** instead:

- A component documents which child element names it consumes (e.g. `cem-button`
  consumes a leading `cem-icon` child as a "leading icon").
- Author children that don't match any documented role render in place, untouched.
- A `slot="..."` attribute on an author child is preserved verbatim for downstream
  tools (e.g. design-system inspectors). The component itself does not consult
  `slot` for layout decisions; layout comes from the documented child-name roles.

This keeps the cem-components contract aligned with WHATWG light-DOM rendering and
with the way `cem_ml`'s transform emits author content.

## 7. Render lifecycle and idempotence

- A render MUST be a pure function of: (a) declared attributes, (b) `datadom`
  state (slices, slots), and (c) the static template. Given the same inputs, two
  renders MUST produce byte-identical light-DOM output.
- Re-render MUST be safe to call repeatedly. The runtime detects no-op renders by
  diffing the produced DOM against the current children of the host.
- A component MUST guard against the recursive render loop documented in
  `~/aWork/custom-element/docs/attributes.md`: an attribute computed via `select`
  that triggers a parent attribute write which re-triggers the child. The cem-components
  package documents per-component which attributes are `select`-bound and avoids
  cyclic dependencies between them.

## 8. Compatibility expectations with `<cem-element>`

The components in this package commit to the following substrate behaviors:

| Behavior | Required commitment |
| --- | --- |
| Single direct-child `<template>` wrapping the data island | Yes; no declarations outside `<template>`. |
| `attribute` declarations as `observedAttributes` | Yes; defaults from text node or cem-ql `select="…"`. |
| `slice` data model | Yes; sole source of dynamic state inside a template. |
| `slice-event` declarative event bindings | Yes; no programmatic event handlers in components. |
| `${…}` cem-ql text interpolation and `{…}` AVT spans in attributes | Yes; XPath/XSLT interpolation is not used. |
| Bridge-window legacy authoring via `<template lang="custom-element-v0">` | NOT used by cem-components; new primitives go straight to cem-ql. |
| Light-DOM rendering with author children preserved | Yes; no `attachShadow`. |
| `<text>` wrapper to escape `{}` inside CSS blocks | Yes; CSS in templates follows this rule. |
| Shadow / closed / named-root rendering surfaces | NOT used by cem-components. |
| `customElements` registry interaction | Handled by `<cem-element>` runtime; components MUST NOT call `customElements.define` themselves. |

If `@epa-wg/cem-elements` adds a feature outside this list, components MAY adopt it
only when the AC or substrate-design entry exists in this repo's docs and a fixture
exists under `packages/cem-elements/tests/` or `examples/` to verify it. Substrate
parity targets — legacy `<custom-element>` plus
`~/aWork/custom-element-dist/src/material/` — are tracked by the production-ready
criteria in [`docs/cem-element-design.md §7`](../../../docs/cem-element-design.md).

## 9. AC and design references

- [`docs/cem-element-design.md`](../../../docs/cem-element-design.md) — `<cem-element>`
  substrate design, including the `<template>`-wrapped data island and the
  cem-ml/cem-ql template engine.
- AC-I-6 (WHATWG HTML DOM compliance as transform) — `docs/cem-ml-ac.md`.
- AC-F-5 (reference slots for `id`/`for`/`aria-*`) — host-attribute forwarding feeds it.
- AC-T-1 / AC-T-7 (transform contract and template embedding) — produces the markup
  cem-components consume; round-trip is required.
- [`docs/cem-ql-ac.md`](../../../docs/cem-ql-ac.md) — cem-ql expression contract for
  template text, AVT, and `select=` declarations.
- Cross-surface conversion fixtures (Phase 2) — round-trip target for rendered output.
- `@epa-wg/custom-element` POC (`~/aWork/custom-element/`) — functional reference,
  scheduled for monorepo migration to `packages/custom-element/`; not a syntax
  decision point per [`CLAUDE.md`](../../../CLAUDE.md).
- `~/aWork/custom-element-dist/src/material/` — material parity benchmark for
  `<cem-element>`.
