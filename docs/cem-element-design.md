# `cem-element` Design

**Status:** Design doc for the `<cem-element>` declarative custom-element substrate.
Pairs with the parser/runtime work in [`cem-ml-stack-design.md`](./cem-ml-stack-design.md),
the query/template surface in [`cem-ql-stack-design.md`](./cem-ql-stack-design.md), and the
component contracts in [`packages/cem-components/docs/`](../packages/cem-components/docs/).

This document is the source of truth for the `cem-element` substrate that
`@epa-wg/cem-components` builds on. It supersedes the
`<custom-element>` authoring tag from `@epa-wg/custom-element` while preserving the
declarative concept that POC introduced.

## 1. Goal

`cem-element` keeps the `@epa-wg/custom-element` concept — a declarative element that
holds a **data island**, wires DOM events to data-change updates, and re-renders the
host's light-DOM children from a template + data — and replaces the template engine
with CEM-native syntax:

- Template markup uses canonical **CEM-ML** (curly-brace) or its XML/HTML parity
  surface; both lower into the same event/AST model owned by `cem_ml`.
- Expressions inside templates and attribute-value spans use **CEM-QL**, replacing
  XPath as the data-access language.
- The data island is wrapped in a WHATWG `<template>` element so its contents sit in
  the inert `template.content` DocumentFragment and never reach the live render
  tree. Only the rendered output (driven from the data island) is visible.

`cem-element` is **not** a fork of `<custom-element>`. It is its functional successor.
The end state of this work is that `@epa-wg/custom-element` next major adopts
`cem-element` as its authoring tag and the legacy `<custom-element>` tag is removed.

## 2. Packages

| Package | Status | Role |
| --- | --- | --- |
| `@epa-wg/cem-elements` | Planned, this design | Houses the `<cem-element>` runtime and its declarative authoring surface. Plural ("elements") distinguishes the substrate from `@epa-wg/cem-components` (the primitive library that consumes it). |
| `@epa-wg/cem-components` | Phase 3, contract docs landed | Declarative component primitives (`cem-button`, `cem-text-field`, …) authored with `<cem-element>` and the conventions in [`packages/cem-components/docs/conventions.md`](../packages/cem-components/docs/conventions.md). |
| `@epa-wg/custom-element` | External today, scheduled for monorepo migration | Existing POC at `~/aWork/custom-element/`. Source moves into `packages/custom-element/`; future major adopts `cem-element` and retires the legacy `<custom-element>` tag. |
| `custom-element-dist` (reference) | External | Material-style sample components at `~/aWork/custom-element-dist/src/material/` (`action`, `autocomplete`, `badge`, `dropdown`, `icon`, `icon-link`, `input`, `menu`). Used as the parity benchmark for `cem-element` (see §7). |

## 3. Authoring surface

```html
<cem-element tag="cem-button">
  <template>
    {attribute @name="disabled"}
    {attribute @name="busy"}
    {attribute @name="label" | Save}

    {button
      @disabled={$disabled}
      @aria-busy={$busy}
      | ${$label}
    }
  </template>
</cem-element>
```

Or the XML/HTML parity form (lowered to the same AST):

```html
<cem-element tag="cem-button">
  <template>
    <attribute name="disabled" />
    <attribute name="busy" />
    <attribute name="label">Save</attribute>

    <button disabled="{$disabled}" aria-busy="{$busy}">${$label}</button>
  </template>
</cem-element>
```

### 3.1 The `<template>` wrapper IS the data island

- Every direct child of `<cem-element>` MUST sit inside a single WHATWG `<template>`
  element. The browser parks `<template>` content in `template.content` (a
  `DocumentFragment`) and does not render it, so:
  - inner text never bleeds into the live page;
  - inner elements never affect layout;
  - inner attributes never reach selectors;
  - the data island is **inert by default** without any author opt-in.
- The cem-element runtime reads `template.content` at upgrade time, lowers it to the
  same `NormalizedEvent` stream `cem_ml` already produces, and runs it through the
  configured schema/scope policy.
- Multiple top-level concerns (attribute declarations, slices, named render templates,
  inline styles, plugin descriptors) coexist inside the single `<template>` — they are
  distinguished by element name, not by sibling position.
- The host element's own children outside the `<template>` are the **author-supplied
  light-DOM input** (slots, default text). They remain visible exactly as the author
  wrote them and provide the progressive-enhancement fallback.

### 3.2 Template engine

| Concern | `<custom-element>` legacy | `<cem-element>` |
| --- | --- | --- |
| Template syntax | XSLT-shaped HTML with `<for-each>`, `<if>`, `<choose>` | CEM-ML curly surface or XML/HTML parity; `cem-ql` template embedding (AC-T-7) |
| Expression language | XPath 1.0, `$var` and `//path` | CEM-QL (see [`cem-ql-stack-design.md`](./cem-ql-stack-design.md)); `$var` for declared attributes, dotted/path forms for slices |
| Text interpolation | `{ … }` in text and attribute values | `{ $expr }` in attributes (AVT spans); `${ $expr }` in text. Bare `{ … }` text is rejected per `cem-ml-syntax.md` Tier A. |
| Attribute declarations | `<attribute name="…">default</attribute>` | Same shape, lowered to the same AST. Default text or `@select="{$expr}"` attribute. |
| Slices and slice events | `slice="x"` + `slice-event="…"` + `slice-value="{ … }"` | Same surface, but `slice-value` carries a CEM-QL expression. |
| Validation / open-content | Implicit per the POC engine | Schema-governed; the cem-element substrate participates in `cem_ml` scope policy and Tier A semantic-validation catalog. |

## 4. Runtime model

1. **Upgrade.** When the browser upgrades `<cem-element tag="X">`, the runtime:
   - looks up the single child `<template>`;
   - hands `template.content` to `cem_ml` for tokenization, schema scoping, and AST
     construction;
   - extracts declared attributes (becomes the host's `observedAttributes`);
   - extracts slices (becomes the data island's mutable state);
   - extracts the render template (a CEM AST projected to WHATWG light DOM via
     `cem_ml`'s `OutputTarget::LightDomCustomElements`, AC-I-6);
   - registers `tag="X"` with `customElements.define` if not already defined.
2. **Render.** On connect and on every data-island change, the runtime re-renders the
   host's light-DOM children from the cached AST + the current data-island state.
   The render path goes through the same `cem_ml::interpreter::light_dom` pipeline
   as the build-time transform, so dev/runtime output is byte-identical.
3. **Events.** Declarative `slice-event="…"` bindings install DOM listeners on the
   rendered children. Listener payloads write back to the data island, which
   triggers the next render. There are no JS event handlers in the authoring
   surface.
4. **Source maps.** Every rendered node carries the AC-P-7 source-map stack back to
   its position inside `template.content`, so dev tools can trace any node in the
   live DOM to its author byte offset.

## 5. Data-island isolation guarantees

The `<template>` wrapper exists to make the following true without author effort:

- **Render isolation.** No child of `<template>` participates in CSS selector
  matching, layout, painting, accessibility tree, or `getElementsByTagName` on the
  document.
- **Form isolation.** Form-associated descendants inside `<template>` are not part
  of the page's form data; only the rendered (cloned) form controls submit.
- **Mutation isolation.** Author writes to the data island go through the runtime's
  scope-policy mutation API (AC-M-*); direct DOM mutations of `template.content`
  are allowed (it is a real `DocumentFragment`) and trigger a render diff.
- **Polyfill story.** When the browser does not upgrade `cem-element` (no JS, JS
  failed, lazy load pending), the page still renders the author's light-DOM input
  exactly as written; nothing inside `<template>` was visible to begin with.

## 6. Compatibility & migration

### 6.1 `@epa-wg/custom-element` monorepo migration

- The package is migrated from its current home (`~/aWork/custom-element/`) into
  `packages/custom-element/` inside this monorepo. The migration preserves history
  and the published npm package identity.
- Until parity is reached (§7) the existing `<custom-element>` authoring tag remains
  the production surface. The package continues to publish from this monorepo.
- The next major of `@epa-wg/custom-element` ships `cem-element` as the canonical
  authoring tag. `<custom-element>` is removed in that major. Cem-components and
  external consumers cut over at that boundary.

### 6.2 Co-existence window

During the bridge period (between this design landing and the major cutover):

- Both tags MAY appear in the same document. They share `customElements` registry
  state; tag names MUST NOT collide.
- The `cem-element` runtime understands the legacy XSLT-shaped template body as a
  compat surface only when the body is annotated `lang="custom-element-v0"` on the
  `<template>` element. New code MUST use the CEM-ML surface.

### 6.3 Cem-components contract

`@epa-wg/cem-components` authors every primitive with `<cem-element>`. The contract
docs in [`packages/cem-components/docs/`](../packages/cem-components/docs/) name
`<cem-element>` as the authoring tag and `cem-ql` as the expression language. The
host-API, attribute, event, validation, focus, and a11y rules are independent of
which substrate hosts them and remain authoritative.

## 7. Production-ready criteria

`@epa-wg/cem-elements` is **production-ready** (and the bridge window closes) only
when **all** of the following hold:

1. **Functional parity with `<custom-element>`.** Every public behavior the POC
   documents (`~/aWork/custom-element/docs/attributes.md`,
   `~/aWork/custom-element/docs/rendering.md`) reproduces under `<cem-element>` with
   a one-to-one fixture in `packages/cem-elements/tests/parity/legacy/`.
2. **Material parity.** Every component in
   `~/aWork/custom-element-dist/src/material/` — `action.html`, `autocomplete.html`,
   `badge.html`, `dropdown.html`, `icon.html`, `icon-link.html`, `input.html`,
   `menu.html` — is rebuilt under `<cem-element>` with a paired fixture in
   `packages/cem-elements/tests/parity/material/`. The rendered DOM, accessibility
   tree, and keyboard behavior match the legacy versions on a documented browser
   matrix.
3. **Cem-ml integration.** All `<cem-element>` templates parse cleanly through
   `nx run cem_ml_cli:validate-fixtures` and round-trip through
   `nx run cem_ml_cli:e2e` cross-surface conversion. The Phase 2 semantic-validation
   catalog applies without exceptions.
4. **Performance.** AC-N-1 first-paint budgets hold on the material parity fixtures
   under the same `nx run cem_ml:bench` discipline.
5. **A11y.** The accessibility contract from
   [`packages/cem-components/docs/accessibility.md`](../packages/cem-components/docs/accessibility.md)
   is verified end-to-end on the material parity fixtures.

When (1)–(5) are green, `cem-element` is folded into the next major of
`@epa-wg/custom-element` and `@epa-wg/cem-elements` is archived.

## 8. References

- [`docs/cem-ml-syntax.md`](./cem-ml-syntax.md) — CEM-ML canonical curly surface.
- [`docs/cem-ml-ac.md`](./cem-ml-ac.md) — AC-F-2 (schema scoping), AC-F-5
  (reference slots), AC-I-6 (WHATWG DOM compliance), AC-M-* (mutation), AC-P-7
  (source-map stack), AC-T-1 / AC-T-7 (transform + template embedding).
- [`docs/cem-ql-ac.md`](./cem-ql-ac.md) — CEM-QL surface that backs template
  expressions and AVT spans.
- [`packages/cem-components/docs/conventions.md`](../packages/cem-components/docs/conventions.md),
  [`light-dom-rendering.md`](../packages/cem-components/docs/light-dom-rendering.md),
  [`accessibility.md`](../packages/cem-components/docs/accessibility.md) — the
  contract the substrate exists to enable.
- `~/aWork/custom-element/` — legacy POC, functional reference per
  [`CLAUDE.md`](../CLAUDE.md) §custom-element legacy info.
- `~/aWork/custom-element-dist/src/material/` — material parity benchmark.
