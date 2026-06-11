# `@epa-wg/custom-element` Adapter Boundary

This records the Phase 3.6 boundary for moving the published
`@epa-wg/custom-element` package onto the `@epa-wg/cem-elements` substrate.
It should be read with
[`custom-element-package-baseline.md`](custom-element-package-baseline.md) and the
legacy parity inventory in
[`../packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md).

## Decision

`@epa-wg/custom-element` keeps the published `<custom-element>` declaration tag
and package entrypoints, but it does not keep the legacy XSLT/XPath parser and
render engine as an internal implementation.

The migrated package is a compatibility adapter over `CemElementRuntime`:

- the adapter exports a `CustomElement extends HTMLElement` class from
  `custom-element.js`;
- importing `custom-element.js` still defines the `custom-element` declaration tag
  as a browser side effect;
- the adapter owns only legacy declaration normalization and public package
  compatibility;
- declaration compilation, produced-element registration, instance payload
  capture, data-island snapshots, slices, host-attribute invalidation,
  diagnostics, and light-DOM rendering are delegated to the `cem-elements`
  runtime.

The first implementation should instantiate `CemElementRuntime` with
`declarationTag: 'custom-element'` and register the exported `CustomElement`
adapter class itself. Do not call `installCemElementRuntime()` for this package's
default browser side effect, because that helper defines an internal anonymous
declaration class and would make the exported `CustomElement` class diverge from
the element registered for `<custom-element>`.

## Public Surface To Preserve

The next-major package must preserve these browser-facing names unless a later
publish-readiness pass explicitly lists the break:

- package name: `@epa-wg/custom-element`;
- declaration tag: `<custom-element>`;
- `custom-element.js` default export and named `CustomElement` export;
- `index.js` default export of `CustomElement`;
- `index.js` re-exports for `custom-element.js`, `http-request.js`,
  `local-storage.js`, and `location-element.js`;
- import-time `customElements.define(...)` side effects for published browser
  modules;
- package metadata and entrypoints captured in the package baseline, including
  `browser`, `module`, `types`, `web-types`, and `exports`.

`module-url.js` remains a shipped browser file and side-effect registration for
now, but it is not added to `index.js` in this boundary decision because the
current package baseline does not re-export it.

## Runtime Handoff

The adapter should be shaped as a facade around one substrate runtime per host:

```js
import { CemElementRuntime } from '@epa-wg/cem-elements';

export const customElementRuntime = new CemElementRuntime({
  declarationTag: 'custom-element',
});

export class CustomElement extends HTMLElement {
  static observedAttributes = ['src', 'tag', 'hidden'];

  connectedCallback() {
    normalizeLegacyDeclaration(this);
    customElementRuntime.registerDeclaration(this);
  }
}

if (!customElements.get('custom-element')) {
  customElements.define('custom-element', CustomElement);
}
```

The implementation may expose a small host API later, but only as an adapter over
the same runtime:

- `installCustomElementRuntime(host, options)` to install the adapter class in a
  non-default window/document host;
- `getCustomElementRuntime(host)` or `customElementRuntime` for diagnostics and
  test synchronization;
- pass-through helpers for `diagnosticsFor(target)`,
  `whenDeclarationSettled(declaration)`, and `whenRenderSettled(instance)`.

The adapter must not create a parallel produced-element class, data document,
render loop, XSLT stylesheet, or XPath evaluator.

## Legacy Declaration Normalization

Normalization happens before `runtime.registerDeclaration(this)`. Its output must
look like a declaration the substrate already accepts.

### `tag`

`tag` is the produced custom-element name and maps directly to the substrate
declaration `tag`.

The legacy omitted-`tag` behavior auto-created a `dce-*` produced tag and inserted
an inline instance. The substrate currently requires `tag`; omitted `tag` remains
a bridge/adoption decision rather than part of this boundary. If it is preserved
for the migration window, it should be implemented by normalization that creates a
stable produced tag plus explicit instance markup, then delegates to the runtime.

### `src`

`src` maps directly to the substrate declaration `src`.

The adapter should preserve these legacy authoring forms through the runtime's
`loadSrcDocument` hook:

- local same-document fragments, for example `src="#my-template"`;
- relative or absolute external documents with fragments, for example
  `src="./forms.html#cem-input"`;
- host-resolved package/module specifiers where the consumer provides a loader.

The adapter should not reintroduce `XMLHttpRequest` loading or package-local URL
resolution logic. Fetching, module-map resolution, and scope policy belong to
`CemElementRuntimeOptions.loadSrcDocument`.

### Inline Templates

Inline `<template>` children are passed to the substrate. During the migration
window, legacy templates that do not declare `lang` or `type` are annotated as:

```html
<template lang="custom-element-xslt">...</template>
```

That routes old HTML+XSLT-shaped authoring through the legacy-XSLT conversion
path instead of the canonical CEM-ML parser. The conversion path parses the
template DOM, lowers the fixture-bounded XSLT/XPath subset to canonical CEM-ML,
and renders through `cem_ql` WASM. Templates that already declare
`lang="custom-element-xslt"`, `lang="custom-element-v0"`, `type="text/cem-ml"`,
or `type="application/cem-ml"` must be left unchanged. The `custom-element-v0`
bridge remains an explicit DOM-projection compatibility path; it is not the
default for untyped legacy templates.

### Data Islands And Payload

The legacy implementation builds a live XML `datadom` document from instance
attributes, dataset, payload children, slices, and validation state. The migrated
adapter must not preserve that live XML model.

Produced instances use the substrate data island:

- payload is captured into `<template data-cem-island="instance">`;
- live fallback payload is removed before first render;
- render input is the serializable `DataIslandSnapshot`;
- `datadom.*` access is provided by the substrate's structured snapshot mapping.

### Slices And Event-To-Data Wiring

Legacy slice attributes are carried through the legacy-v0 bridge where fixture
coverage already exists:

- `slice`;
- `slice-event`;
- `slice-value`;
- focused event/value update forms.

Broader legacy behavior, including multiple event names, multiple slice targets,
checkbox/radio coercion, and full XPath-style slice access, remains a bridge-policy
item. Any preserved behavior must update the same substrate slice state used by
`CemElementRuntime`; it must not maintain a second package-local event queue.

### Host Attributes

Host attributes are observed by produced instances through the substrate's
instance-level mutation observation and `DataIslandSnapshot.hostAttributes`.

The adapter may keep `CustomElement.observedAttributes = ['src', 'tag', 'hidden']`
for declaration compatibility, but produced-element attribute invalidation is not
owned by `@epa-wg/custom-element`.

### Resource Primitives

`module-url` resource slices should resolve through the substrate
`resolveModuleUrl` hook when they participate in rendering.

`http-request`, `local-storage`, and `location-element` remain companion modules in
this boundary. They may become substrate-backed primitives, documented shims, or
explicit non-goals in the later companion-module task.

## Diagnostics And Verification

The adapter should use substrate diagnostics as the authoritative result:

- declaration shape and `src` diagnostics come from `CemElementRuntime`;
- legacy normalization diagnostics should be recorded in the adapter only when the
  input cannot be translated into a substrate declaration;
- tests should wait on `whenDeclarationSettled()` and `whenRenderSettled()` instead
  of sleeping around asynchronous registration.

Package-local fixtures should prove at least:

- importing `custom-element.js` defines `custom-element`;
- the default and named exports are the same `CustomElement` class registered for
  the tag;
- inline legacy templates are annotated or routed to `custom-element-v0`;
- `src="#id"` and external `src="file#id"` delegate to the runtime loader;
- a declaration registers and renders a produced tag through the substrate;
- rendered instances expose substrate data-island metadata;
- no adapter path uses `XSLTProcessor`, `createXsltFromDom`, `DceElement`, or a
  package-local render loop.

## Non-Goals

This boundary does not:

- reimplement XPath or XSLT in `@epa-wg/custom-element`;
- define new semantics for `<cem-element>`;
- rewire downstream `cem-theme` consumers;
- replace companion modules;
- decide whether every legacy-v0 bridge gap is kept, migrated, or dropped;
- complete publish-readiness for the next major.

Those are the next Phase 3.6 tasks in [`todo.md`](todo.md).

## Recommended Engine Ownership

The published adapter should stay thin. Legacy HTML+XSLT semantics must not
become a second browser-only implementation hidden inside
`@epa-wg/custom-element`.

Current state:

- `@epa-wg/custom-element` normalizes untyped templates and delegates to
  `CemElementRuntime`;
- `cem_ml::legacy_custom_element` records the CEM-owned Tier 1/2 compatibility
  contract and Tier 3 handoff boundary;
- `cem-elements/src/lib/legacy-xslt/contract.ts` mirrors that compatibility
  contract for the current browser adapter and fixture gates;
- `cem-elements` owns the current TypeScript DOM-to-CEM-ML legacy converter
  implementation;
- `cem_ql` owns the canonical CEM-ML render boundary;
- `cem_ml` owns XSLT namespace dispatch/version-pinning only, not execution.

Recommended target:

- move the legacy HTML+XSLT compatibility compiler behind a CEM-owned engine
  boundary shared by browser runtime, CLI, SSR, and tests;
- keep the compiler output as canonical CEM-ML plus `cem_ql` expressions, not a
  live XSLT/XPath engine;
- treat copied demo/material files as executable compatibility fixtures with a
  per-file construct allowlist;
- keep standalone XSLT stylesheets and push-template constructs as an explicit
  Tier 3 handoff, not part of the material component bridge.

This preserves the current low-risk conversion strategy while making the
requirement "old custom-element syntax is supported by the CEM-ML engine" true
for non-browser surfaces as well.
