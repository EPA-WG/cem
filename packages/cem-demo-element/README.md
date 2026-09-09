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
an import map. The WASM runtime is loaded lazily when HTML or CEM-ML source
needs semantic formatting or live rendering. See [`demo/index.html`](./demo/index.html) and
[`demo/advanced.html`](./demo/advanced.html) for local workspace examples.

## Source contract

- With no template, the element presents its authored body and keeps that body
  as the live HTML demo.
- With a `<template>`, the template body is the displayed source and is cloned
  into the live demo.
- A `<template slot="source">` explicitly selects the source.
- `source` accepts a string or DOM node and can also be set as an attribute.
- `src` fetches source; fetched HTML is also rendered as the live demo, with
  URL-valued attributes rebased to the fetched document. `type="auto"` detects
  HTML, CSS, JavaScript/JSON, CEM-ML, or plain text from the response media type
  and URL extension.
- `type="cem-ml"` renders the source to live HTML through WASM. Invalid input
  is not injected and its structured diagnostics appear in the `status` region.

> **Language support:** The set of languages available for source formatting
> comes from the CEM-ML formatters; `cem-demo-element` does not define a
> separate language set.

HTML, CEM-ML, and CSS use lossless semantic roles produced by CEM-ML. The default
source theme derives those roles from CEM action and emotional tokens: bold tag
names use `--cem-action-primary-active-background`; bold attribute/variable
identifiers use `--cem-action-destructive-pending-background`; keywords use
`--cem-action-primary-pending-background`; strings use
`--cem-action-contextual-pending-background`; and errors use
`--cem-action-destructive-hover-background`. Comments use `calm-x`, punctuation
uses a subdued `conservative-x`, and numbers retain `creativity-x`.
CSS declaration properties, ordinary values, and functions use separate
IDE-inspired red/light-blue, blue/peach, and gold pairs. CSS custom-property
definitions and `var()` references retain the bold variable role. The CSS
parser records selector, property, custom-property, value, and function context
on its lossless AST events; the source view does not infer those roles from
token spelling alone.
Highlighted source keeps classes on the enclosing `code.cem-source-code` only.
Its compact native vocabulary is `b` for tag/name, `var` for
attribute/variable, `dfn` for CSS property, `data` for CSS value, `kbd` for CSS
function, `strong` for keyword, `i` for string, `u` for number, `small` for
comment, `samp` for source text, and `mark` for error; punctuation and raw
source remain unwrapped. The semantic elements retain useful visual
distinctions when CEM theme variables are unavailable.
Override `--cem-color-syntax-punctuation`, `--cem-color-syntax-name`,
`--cem-color-syntax-attribute`, `--cem-color-syntax-property`,
`--cem-color-syntax-value`, `--cem-color-syntax-function`,
`--cem-color-syntax-string`, or `--cem-color-syntax-comment` on an individual
`cem-demo-element` to theme it.
The complete role and parser-event matrix, including native/light/dark and both
contrast modes, is available in
[`demo/syntax-coloring.html`](./demo/syntax-coloring.html).

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
