# Component source and development pattern

Accepted for all `cem-components` members.

## One production definition

Author each component in `src/components/<tag>/<tag>.xhtml`, with a
`<cem-element tag="<tag>">` containing `<template id="<tag>" type="text/cem-ml">`.
Keep component styles and behavior in that declaration. Consumers load
`<declaration-url>#<tag>`. The definition has no visible showcase or development
controls. Its colocated `<tag>.stories.ts` owns automated component tests.

This separation lets applications import the component without downloading or
executing its documentation UI. A development page still provides a single
place to edit, inspect and exercise that same source.

## Companion property playground

Provide a separate page under `playgrounds/`, outside the canonical two-file
component folder. The page loads the production declaration through page-level
module maps, so IDE HTTP previews and packaged previews use the same convention.

The page contains:

- One live component instance and a property form using shared CEM controls.
- Controls for every supported public option. For action, start with intent,
  bend, label, type, disabled, loading and expanded.
- The fetched canonical XHTML source, displayed without executing a second copy
  (`cem-demo-element` supports `demo="false"`).
- A link to the full examples and variation matrix, plus the automated stories.

Keep option bindings declarative in XHTML/CEM-ML. The property form changes the
public attributes or projected content of the real component. Hover, active and
keyboard focus remain native interactions; the playground must not add fake
production state attributes. Explain how to exercise those states next to the
preview. A missing reusable control or binding capability is work for the shared
component/runtime, not page-local JavaScript behavior.

The full demo remains useful for comparing samples side by side. It imports the
same definition and does not own another implementation.

## Release output

Generate a combined XHTML document with unique template IDs, allowing consumers
to load `components.xhtml#cem-action`, alongside the individual definitions.
Preserve the original source and companion playground in the distribution.
The bundle is a build artifact derived from canonical declarations through the
shared CEM AST transformation pipeline; never edit it by hand or maintain a
second set of templates. Verify individual and bundled loading, dependency URLs,
style ownership and duplicate-registration behavior before publishing it.

## Delivery status

The source/test split is implemented for `cem-select` and `cem-action`.
The companion property playground and combined release document are accepted
next work, tracked in [todo.md](todo.md). Existing action examples already load
the canonical declaration. They remain at the compatibility gallery URL.
