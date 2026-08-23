# Declarative UI Is the Primary Architecture

**Status: normative.** This is the primary authoring rule for
`@epa-wg/cem-components`, CEM Studio, and CEM Site. It takes precedence over
older examples, generated output, migration-era TypeScript registries, and
package documents that describe the current implementation rather than the
target architecture.

## The rule

`cem-elements` and the browser code inside its runtime are the imperative
substrate. Components and applications consume that substrate declaratively;
they do not reimplement it.

- Every `@epa-wg/cem-components` member MUST be a `<cem-element>` template
  authored in CEM-ML and stored as XHTML at
  `packages/cem-components/src/components/<cem-tag>/<cem-tag>.xhtml`.
- Its Storybook source MUST be colocated at
  `packages/cem-components/src/components/<cem-tag>/<cem-tag>.stories.xhtml`.
  Component unit scenarios and assertions belong in that declarative Storybook
  document. A separate component `*.spec.ts`, `*.test.ts`, browser-spec module,
  or fixture-owned unit suite is not an acceptable substitute.
- Its component-owned CSS MUST be embedded in the declaration's CEM-ML template
  as a `<style>` node (authored in CEM-ML as `{style |```...```}`). The
  `cem-elements` runtime scopes that style to the rendered component instance.
  The embedded CSS consumes CEM UI theme tokens through `var(--cem-*)`; it must
  not import or redefine the theme or use a non-CEM custom property as a
  visual-token substitute. A separate `<cem-tag>.css` file is forbidden.
- `@epa-wg/cem-components` MUST contain no authored JavaScript or TypeScript in
  its completed state: no component classes, behavior callbacks, DOM listeners,
  registration arrays, installers, test modules, Storybook CSF modules, or
  package-local build scripts. XHTML, CEM-ML, CSS, metadata, and documentation
  are the package's authored surfaces. Build and Storybook adapters belong to
  the owning framework/tooling project.
- CEM Studio and CEM Site MUST author visible UI, component composition,
  interaction bindings, and UI state projection in XHTML/CEM-ML. Application
  JavaScript may implement non-UI host services such as persistence, workers,
  routing transport, browser APIs, and engine adapters, but it MUST NOT create
  or replace visible controls, render UI with DOM APIs, or attach app-local UI
  behavior listeners.

The runtime JavaScript loaded by a browser does not make a component or app
imperatively authored. The ownership boundary does: reusable DOM behavior and
browser interaction machinery live in `cem-elements`; component and
application source stays declarative.

## Hard-stop capability rule

When CEM-ML cannot express required behavior, or expressing it would require
verbose repetition that obscures the component contract, STOP work on the
component or application UI. Do not add a JavaScript behavior as a workaround.

1. Describe the smallest reusable declarative capability that is missing.
2. Implement and test that capability in `cem-elements` (and in the native
   CEM-ML/CEM-QL engine first when it changes transformation semantics).
3. Expose it through a declarative element, attribute, resource, event binding,
   or CEM-ML construct.
4. Resume the component only after the framework capability and its executable
   evidence pass.

This is a decision gate, not a preference. A component PR that reaches this
boundary remains blocked; moving the missing behavior into Studio, Site, a
Storybook file, or another `cem-components` JavaScript module is not an option.

## Declarative Storybook contract

The Storybook integration is a `cem-elements` capability because Storybook's
native CSF input is JavaScript/TypeScript. Before the first component can use
the required no-JS layout, `cem-elements` MUST provide an XHTML story indexer
and loader that:

- discovers colocated `*.stories.xhtml` documents;
- renders their CEM-ML declarations and examples through the production
  `cem-elements` runtime;
- recognizes declarative `<cem-story>` cases and `<cem-test>` assertions;
- reports those assertions through the repository's Storybook test runner;
- loads `@epa-wg/cem-theme/styles.css` exactly once for the preview and lets the
  production `cem-elements` render path materialize each declaration's embedded,
  automatically scoped `<style>` node;
- exposes an accessible Storybook-owned **Theme** switcher composed with the
  production `cem-select` component and the exact modes `cem-theme-light`,
  `cem-theme-dark`, `cem-theme-contrast-light`,
  `cem-theme-contrast-dark`, and `cem-theme-native`;
- keeps the Storybook global as theme authority, defaults it to
  `cem-theme-native`, and applies the selected value as both the one theme class
  and `data-theme` value on the preview root while removing the previous mode;
- switches the currently rendered story without requiring the story to own,
  persist, or duplicate theme state, and without re-registering its component;
- runs every component's interaction, accessibility, and visual assertions in
  all five modes and proves its used `--cem-*` properties resolve; and
- preserves browser, accessibility, interaction, and visual evidence without a
  component-owned CSF or test module.

Until that adapter exists and is executable, adding or migrating a component is
hard-stopped. The adapter must be created in `cem-elements`; the component may
not fall back to a `.stories.ts` file.

## Existing migration debt

The current package predates this rule. Its monolithic
`CEM_COMPONENT_PRIMITIVES` registry, `*-behavior.ts` files, installers,
package-local scripts, separate browser specs, and 49 registered component tags
are legacy migration debt. They remain runnable only to avoid representing an
unfinished migration as a release-ready rewrite. They are not templates for new
work and the legacy inventories may only shrink.

`packages/cem-components/declarative-migration.json` records that baseline.
`yarn nx run @epa-wg/cem-components:verify-declarative` rejects new authored
JavaScript/TypeScript, drift in the frozen legacy code, new legacy registry
members, misplaced component sources, missing colocated XHTML stories or
embedded token-based styles, forbidden standalone component CSS, and a
component migration attempted before the `cem-elements` Storybook adapter and
Storybook-owned five-mode theme controller exist. The target state is zero
legacy component tags and zero authored code files.

The same migration must remove app-local UI rendering and behavior from Studio
and Site. Service/adaptor JavaScript may remain only behind the non-UI boundary
above.
