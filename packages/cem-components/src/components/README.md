# Declarative component source root

Every non-legacy component has exactly one folder here:

```text
<cem-tag>/
├── <cem-tag>.xhtml
└── <cem-tag>.stories.xhtml
```

The first file is a `<cem-element>` declaration with a
`<template type="text/cem-ml">`. Its component-owned CSS is embedded there as a
`<style>` node, authored in CEM-ML as `{style |```...```}`, and consumes CEM UI
theme tokens through `var(--cem-*)`; `cem-elements` scopes it automatically to
the rendered component instance. Do not add `<cem-tag>.css`. The story file owns
the component's declarative Storybook cases and unit assertions. Neither the
folder nor the package may add JavaScript or TypeScript.

Storybook loads the public CEM theme and component CSS, owns the accessible
theme switcher through production `cem-select`, and applies the selected one of
the five canonical theme modes to the preview root. Individual stories do not
own or persist theme state.

Read [`../../../../docs/declarative-ui-principle.md`](../../../../docs/declarative-ui-principle.md)
before adding a component. Component work is hard-stopped until
`cem-elements` supplies the declarative XHTML Storybook adapter named in
[`../../declarative-migration.json`](../../declarative-migration.json).
