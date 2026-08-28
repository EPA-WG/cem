# Declarative UI Is the Primary Architecture

**Status: normative.** This is the primary authoring rule for
`@epa-wg/cem-components`, CEM Studio, and CEM Site. It takes precedence over
legacy examples, generated output, migration-era TypeScript registries, and
package documents that only describe old implementation debt.

## The rule

`cem-elements` and the browser JavaScript inside its runtime are the imperative
UI substrate. Components and applications consume that substrate declaratively;
they do not reimplement it locally. The runtime plus CEM-ML must be sufficient
for no-JavaScript component and application UI authoring.

- Every `@epa-wg/cem-components` member MUST be a `<cem-element>` declaration
  authored in CEM-ML and stored at
  `packages/cem-components/src/components/<cem-tag>/<cem-tag>.xhtml`.
  Its declaration template MUST be
  `<template id="<cem-tag>" type="text/cem-ml">`; the stable matching ID keeps
  the template addressable for reuse as `<declaration-url>#<cem-tag>` while the
  template remains part of the component's XHTML structure.
- Every component MUST have exactly one colocated Storybook source at
  `packages/cem-components/src/components/<cem-tag>/<cem-tag>.stories.ts`.
  This development-only CSF Next module is the sole TypeScript exception in a
  component folder; it is test/tooling source, never component implementation or
  production behavior.
- Component-owned CSS MUST be embedded in the XHTML declaration's CEM-ML
  template as a `<style>` node, authored as `{style @type="text/css"
  |```...```}`. `cem-elements` extracts it as a once-per-declaration artifact
  rather than cloning it into instance DOM. The accepted CSS target uses native
  `@scope`: the produced tag owns private rules, declaration-owned
  `scope="name"` is the public named-shared surface, `slot="name"` marks a
  projected root, and `part="name"` marks component-owned internals. An
  instance stylesheet is valid only inside an explicit inert direct `<template>`
  payload. Data-island and `data-cem-render-scope` identity remain internal and
  MUST NOT be selected by authored CSS. Component CSS consumes
  CEM UI tokens through `var(--cem-*)`, does not import or redefine the theme,
  and does not use non-CEM properties as visual-token substitutes. A standalone
  `<cem-tag>.css` file is forbidden and global `src/styles.css` must not contain
  selectors for a migrated component. The normative ownership, scope-resolution,
  specificity, lifecycle, and diagnostics rules are in the
  [CEM light-DOM CSS scoping contract](./cem-ml-uid-and-scoped-css-design.md).
  That contract is the implemented runtime and authoring baseline.
- `@epa-wg/cem-components` MUST contain no authored JavaScript or TypeScript
  except the required colocated `.stories.ts` files: no component classes,
  behavior callbacks, DOM listeners, registration entries, installers, or
  package-local tooling scripts. A story may contain only Storybook setup,
  rendered example strings, and test code.
- CEM Studio and CEM Site MUST author visible UI, component composition,
  interaction bindings, and UI state projection in XHTML/CEM-ML using production
  `cem-components`. Application JavaScript may implement non-UI host services
  such as persistence, workers, routing transport, browser APIs, and engine
  adapters, but it MUST NOT create or mutate visible UI or attach app-local UI
  behavior.

The presence of framework JavaScript in a browser does not make a component or
application imperatively authored. Ownership is the boundary: reusable DOM
behavior and browser interaction machinery live in `cem-elements`; component
and application UI source stays declarative.

## Hard-stop capability rule

When CEM-ML cannot express required behavior, or the expression would require
verbose repetition that obscures the component contract, STOP work on the
component or application UI. Do not add a local JavaScript behavior workaround.

1. Describe the smallest reusable declarative capability that is missing.
2. Implement and test it in `cem-elements` (and in the native CEM-ML/CEM-QL
   engine first when it changes transformation semantics).
3. Expose it through a declarative element, attribute, capability, resource,
   event binding, or CEM-ML construct.
4. Resume component or app UI work only after that framework capability and its
   executable evidence pass.

This is a decision gate, not a preference. Moving missing behavior into Studio,
Site, a story, or another `cem-components` JavaScript/TypeScript module is not an
option.

## Storybook and component-unit-test contract

Storybook's native authoring surface is TypeScript, so the colocated
`<cem-tag>.stories.ts` file is an explicit development-only exception to the
no-component-JavaScript rule. It MUST:

- import `expect`, `userEvent`, `within`, or other required test helpers from
  `storybook/test`;
- use CSF Next `preview.meta(...)` and `meta.story(...)`;
- import its own `./<cem-tag>.xhtml?raw` declaration and load it through the
  shared `cem-elements` Storybook declaration loader;
- return the example HTML body from `render`, for example
  `render: () => '<cem-select>Hello World</cem-select>'`;
- keep component unit assertions in asynchronous `play` functions; and
- contain no component implementation logic or behavior substitute.

Separate component `*.spec.ts`, `*.test.ts`, browser-spec modules, and
fixture-owned unit suites are forbidden for a new or migrated component.
Storybook loads the public `@epa-wg/cem-theme/styles.css`; individual stories do
not import theme CSS or own global theme state.

The proven reference is
[`cem-select.xhtml`](../packages/cem-components/src/components/cem-select/cem-select.xhtml)
with its colocated
[`cem-select.stories.ts`](../packages/cem-components/src/components/cem-select/cem-select.stories.ts).
Its reusable form, choice, keyboard, and interaction machinery is the
`choice-select` capability in `cem-elements`, not component-local code.

## Existing migration debt and enforcement

The package predates this rule. Its remaining monolithic
`CEM_COMPONENT_PRIMITIVES` registry, `*-behavior.ts` files, global component
selectors, installers, and separate browser specs are compatibility debt. They
remain runnable while migrating but are not templates for new work; their
inventories may only shrink.

`packages/cem-components/declarative-migration.json` records that baseline.
`yarn nx run @epa-wg/cem-components:verify-declarative` rejects new legacy code
or registry members, changes to the remaining frozen legacy behavior files,
invalid component folders, component implementation outside XHTML/CEM-ML,
standalone component CSS, migrated selectors in global CSS, component behavior
left in TypeScript, or a colocated story that does not follow the CSF Next
loader/render/`play` contract. The target state is zero legacy component tags
and zero legacy authored code files other than the required stories.

Studio and Site have the same migration direction: app-local visible DOM
construction, UI listeners, and state projection must disappear. JavaScript may
remain only behind the non-UI service boundary above.
