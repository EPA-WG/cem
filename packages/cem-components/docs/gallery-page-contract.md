# Shared gallery page

`playgrounds/cem-gallery-page.xhtml#cem-gallery-page` owns documentation page
chrome. It is a shared playground declaration, separate from the production
component catalog. Source and packaged demos load the same declaration through
`@epa-wg/cem-components/playgrounds/cem-gallery-page.xhtml`.

```html
<cem-element tag="cem-gallery-page"
  src="@epa-wg/cem-components/playgrounds/cem-gallery-page.xhtml#cem-gallery-page">
</cem-element>
<cem-gallery-page heading="cem-icon: Property playground">
  <template>
    <header>…</header>
    <main>…</main>
  </template>
</cem-gallery-page>
```

The example assumes the page's playground import-map prefix. Keep declarations
and runtime setup outside the wrapper. Load the theme and component CSS as native
stylesheet links in the document head, before UI initialization; source and
packaged pages share the same package-relative CSS paths. Place all visible page
content, including the variation matrix, inside one direct inert `<template>`
payload. This lets nested anonymous declarations and source viewers initialize
after page ownership is established. The page composes one
`cem-gallery-nav`; do not add another navigation or theme-switch wrapper.

| Attribute | Meaning | Default |
| --- | --- | --- |
| `heading` | Navigation heading | `Components` |
| `index-href` | Component playground index link, relative to the document | `./index.html` |
| `storage-key` | Shared page preference key | `cem-playground-theme` |
| `data-cem-density` | Inherited page chrome geometry: `normal`, `compact`, `super-compact` | `super-compact` |
| `data-cem-spacing` | Inherited page spacing: `normal`, `dense`, `sparse` | `normal` |

A live text `local-storage` resource supplies the page theme. It stores one full
identifier: `cem-theme-light`, `cem-theme-dark`, `cem-theme-contrast-light`,
`cem-theme-contrast-dark`, `cem-theme-projector` or `cem-theme-native`. Missing,
empty and invalid stored values display light; reading them does not overwrite storage. The effective
value becomes the page host's `data-theme` and is passed through navigation to
the theme switch's controlled `value`, together with the same storage key.

The switch writes the preference when a choice is activated. Existing live
storage synchronization updates the page immediately and any other open same-origin demo
pages. Runtime-owned storage writes notify same-document live consumers directly,
even if the embedding host prevents or replaces storage-method interception.
Interception remains available for external same-document writers; native
`storage` events synchronize other documents. Reloads and navigation restore the saved preference. A custom key creates
a separate page preference. No page-local theme JavaScript or ancestor mutation
is needed.

The page owns viewport sizing, padding, palette background and text color. Its
host exposes density and spacing alongside `data-theme`. The explicit
super-compact profile uses zero coupling and generic control height minima and
zero control padding, so navigation buttons follow their content instead of a
fixed 48px height. It applies without an input media gate, including coarse,
hybrid and unknown pointer capabilities. Tree/table rows retain `1lh` minima.
Explicit control size profiles and inherited consumer overrides still apply.
Normal geometry and the precise-input-only compact profile keep their existing
behavior. Density and spacing are authored preferences, independent of the
stored light/contrast theme.

The `part="content"` wrapper establishes `data-cem-density="normal"` and
`data-cem-spacing="normal"` for examples. Normal/sparse samples and ordinary
property forms keep their own geometry while page chrome is condensed. Samples
can opt into `compact` or `super-compact` themselves; nested `normal` boundaries
restore canonical geometry. No example storage keys are changed.

The page's default slot retains authored content, native form values and nested component
identities across theme changes. Keep source viewers separate from reactive
property forms, as usual.

Examples remain independent. Keep their explicit theme classes/`data-theme`
values and give example switches keys distinct from the page key. A switch
without an explicit key continues to use `cem-theme`, independently of the
default page key. Presentation uses the canonical Projector palette: a white
page canvas with black narrative text and a light color scheme. Its full identifier is stored
and restored in the same way as the other choices.

The variation matrix keeps all five fixed operational themes while the
surrounding page changes. The shared theme generator resolves palettes and
state stripes at each explicit theme boundary, including native/system scopes.

Every gallery heading names its component and page purpose. The shared header
shows a visible Theme label beside the accessible Change theme button, including
at narrow viewport widths. At widths up to 40rem the heading takes its own row,
keeping long component names readable and both navigation controls visible.

`cem-gallery-nav` accepts `theme-storage-key` and `theme-value` from the page
owner. Used independently, it retains the theme switch's `cem-theme` storage
default and uncontrolled selection behavior. The theme menu uses the shared
viewport boundary so navigation choices remain onscreen near window edges.

Verification lives in the gallery-page browser stories and the whole-inventory
`verify-demo-imports` gate. The gate checks source and packaged page ownership,
navigation, restored values, live theme selection, independent storage/matrix
scopes and shared bundle acquisition.

The inventory gate runs pages sequentially by default. Large galleries each load
many independent WASM-backed examples; concurrent pages can delay header renders
past the existing deadlines on constrained hosts. `--concurrency=2` or
`--concurrency=3` opts into parallel pages where resources permit. Assertions and
render deadlines remain identical in every mode.

## Header review — 2026-11-04

All 59 component gallery headings identify their component and the full examples
and variation matrix. Autocomplete and suggestions now follow the same heading
format as the other galleries. The shared Theme label and responsive title row
apply to every page using this chrome, including property playgrounds.

All 118 source/packaged gallery pages pass the sequential inventory gate.
The eight shared page stories cover all six selector choices, retained form
values, independent example scopes and storage. Inventory checks additionally
exercise Presentation selection, the white canvas, reload restoration and
content-specific visible headings in both source and packaged pages. At 320px
and 1440px, the icon-button header and theme switch fit onscreen and keyboard
navigation reaches both controls.

The [delivery readback](../../../docs/declarative-amsterdam-2026/component-gallery-header-readback.json)
records gallery coverage and the separately prepared demo update. Its 906-file
inventory retains the verified demo release and changes four authored header/title
assets. Extract `dist/releases/cem-demo-gallery-update.tgz` and deploy from that
folder with `npx wrangler@4 deploy --config wrangler.jsonc`. This updates
`green-morning-fd6e`; the presentation Site is already verified separately.
The updated gallery chrome awaits owner deployment and public readback.
