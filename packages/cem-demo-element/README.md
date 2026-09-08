# `cem-demo-element`

`<cem-demo-element>` presents source beside a live HTML or CEM-ML example. It is
the offline-first replacement for `html-demo-element`: its element code, source
presentation, CSS, and demos have no CDN dependency.

CEM-ML examples are rendered by the same Rust light-DOM renderer used by the
CEM-ML platform. The WASM response includes diagnostics and output-to-source
spans so the public renderer can also serve CEM Studio rather than becoming a
demo-only implementation.

## Use

Map the CEM-ML WASM package to a local installation and load the element:

```html
<script type="importmap">
{
    "imports": {
        "@epa-wg/cem-ml/wasm": "./node_modules/@epa-wg/cem-ml/dist/wasm/browser/cem_ml.js"
    }
}
</script>
<script type="module" src="./node_modules/@epa-wg/cem-demo-element/dist/index.js"></script>

<cem-demo-element legend="HTML example">
    <template><button type="button">Hello</button></template>
</cem-demo-element>

<cem-demo-element legend="CEM-ML example" type="cem-ml">
    <template>{button @type=button | Hello}</template>
</cem-demo-element>
```

Bundlers can resolve `@epa-wg/cem-ml/wasm` from the package dependency without
an import map. The WASM runtime is loaded lazily, so HTML-only examples do not
initialize it. See [`demo/index.html`](./demo/index.html) and
[`demo/advanced.html`](./demo/advanced.html) for local workspace examples.

## Source contract

- With no template, the element presents its authored body and keeps that body
  as the live HTML demo.
- With a `<template>`, the template body is the displayed source and is cloned
  into the live demo.
- A `<template slot="source">` explicitly selects the source.
- `source` accepts a string or DOM node and can also be set as an attribute.
- `src` fetches source; `type="auto"` detects HTML, CSS, JavaScript/JSON,
  CEM-ML, or plain text from the response media type and URL extension.
- `type="cem-ml"` renders the source to live HTML through WASM. Invalid input
  is not injected and its structured diagnostics appear in the `status` region.

> **Language support:** The set of languages available for source formatting
> comes from the CEM-ML formatters; `cem-demo-element` does not define a
> separate language set.

The named light-DOM regions are `legend`, `description`, `text`, `demo`,
`source`, and `status`. Existing regions are reused, which permits custom order
and wrapper markup while retaining straightforward document styling. In
particular, `slot="source"` selects the input even when another template is
present, `demo` and `text` are replaced with the live and presented results,
and authored `legend` and `description` regions control where those headings
appear. The complete arrangement is demonstrated in
[`demo/index.html`](./demo/index.html).

`updateComplete` resolves after the current fetch or CEM-ML render. The element
emits `cem-demo-render` on success and `cem-demo-error` on failure. Its
`data-state` is `loading`, `ready`, or `error`.

The rendered live output and markup-valued `legend`/`description` are trusted
author content. Do not pass untrusted fetched source to a live HTML or CEM-ML
demo without sanitizing it first.

## Tests

Storybook Chromium interaction stories are this package's unit tests. They
cover inline and template HTML, explicit regions, programmatic and fetched
source, type detection, the Rust/WASM CEM-ML boundary, diagnostics, and the
absence of external CDN resources:

```sh
yarn nx run @epa-wg/cem-demo-element:test
```
