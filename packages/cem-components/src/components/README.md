# Declarative component source root

Every new or migrated component has exactly one folder here:

```text
<cem-tag>/
├── <cem-tag>.xhtml
└── <cem-tag>.stories.ts
```

`<cem-tag>.xhtml` is the entire production component implementation: one
`<cem-element>` declaration with
`<template id="<cem-tag>" type="text/cem-ml">`. The matching stable template
ID allows reuse through `<declaration-url>#<cem-tag>` and remains part of the
component's XHTML structure. Its
component-owned CSS is embedded as `{style @type="text/css" |```...```}` and
uses CEM UI `var(--cem-*)` tokens. `cem-elements` installs it once under
the declaration, not in each instance. The accepted native `@scope` target uses
the produced tag for private rules, declaration-owned `scope="name"` for a
public shared group, `slot="name"` for projected roots, and `part="name"` for
component-owned internals. Per-instance styles require an explicit inert direct
payload template. Data-island and render identity are internal and must not be
styled. Do not add `<cem-tag>.css`, component JavaScript/TypeScript, a
registry entry, or selectors in the package's global stylesheet.

`<cem-tag>.stories.ts` is the development-only exception to the no-component-TS
rule. Use CSF Next `preview.meta` / `meta.story`, import the declaration from
`./<cem-tag>.xhtml?raw`, load it through `loadCemDeclaration`, return example
HTML strings from `render`, and keep every component unit assertion in an async
`play` function using `storybook/test`. It must not contain component behavior.

If CEM-ML is missing functionality or expressing the behavior becomes verbose,
stop. Add the smallest reusable declarative capability to `cem-elements`, test
it there, and resume the component only after it passes. A component- or
story-local JavaScript workaround is forbidden.

Use [`cem-select.xhtml`](./cem-select/cem-select.xhtml) and
[`cem-select.stories.ts`](./cem-select/cem-select.stories.ts) as the proven
pattern. Read the normative
[`declarative-ui-principle.md`](../../../../docs/declarative-ui-principle.md)
and the linked CSS scope contract before adding another component. The native
`@scope` behavior is implemented across browser, worker, Edge/SSR, and hydration
paths.
