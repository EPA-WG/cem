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

A live text `local-storage` resource supplies the page theme. It stores one full
identifier: `cem-theme-light`, `cem-theme-dark`, `cem-theme-contrast-light`,
`cem-theme-contrast-dark` or `cem-theme-native`. Missing, empty and invalid stored
values display light; reading them does not overwrite storage. The effective
value becomes the page host's `data-theme` and is passed through navigation to
the theme switch's controlled `value`, together with the same storage key.

The switch writes the preference when a choice is activated. Existing live
storage synchronization updates the page and any other open same-origin demo
pages. Reloads and navigation restore the saved preference. A custom key creates
a separate page preference. No page-local theme JavaScript or ancestor mutation
is needed.

The page owns viewport sizing, padding, palette background and text color. Its
default slot retains authored content, native form values and nested component
identities across theme changes. Keep source viewers separate from reactive
property forms, as usual.

Examples remain independent. Keep their explicit theme classes/`data-theme`
values and give example switches keys distinct from the page key. A switch
without an explicit key continues to use `cem-theme`, independently of the
default page key. The variation matrix keeps all five fixed themes while the
surrounding page changes. The shared theme generator resolves palettes and
state stripes at each explicit theme boundary, including native/system scopes.

`cem-gallery-nav` accepts `theme-storage-key` and `theme-value` from the page
owner. Used independently, it retains the theme switch's `cem-theme` storage
default and uncontrolled selection behavior. The theme menu uses the shared
viewport boundary so navigation choices remain onscreen near window edges.

Verification lives in the gallery-page browser stories and the whole-inventory
`verify-demo-imports` gate. The gate checks source and packaged page ownership,
navigation, restored values, live theme selection, independent storage/matrix
scopes and shared bundle acquisition.
