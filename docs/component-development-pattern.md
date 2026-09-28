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
- Visible radio groups for mutually exclusive enumerated options, so users
  can compare choices without opening a menu. Use shared CEM controls.
- Controls for every supported public option. For action, start with intent,
  bend, size, label, type, disabled, loading and selected.
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

## Material demo transition

When migrating a Material component from `custom-element` or `cem-elements` to
`cem-components`, mark its old demo page visibly as **Legacy Material demo** and
include “Legacy” in the page title. Link to the canonical CEM component. Once
its companion playground exists, link that page as the current interactive demo.
Keep the old URL usable during the transition and load the canonical definition
where the page still demonstrates the migrated component. Links must work from
both the repository preview and the packaged gallery; a repository source link
is suitable until the new playground has a published URL.

Apply the notice as each component migrates. Do not label an unmigrated page as
having a replacement that does not exist.

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
The action companion is implemented at
`packages/cem-components/playgrounds/cem-action.html`. Its property form uses
visible `cem-radio` groups for enumerated options and `cem-field` for the label. The source viewer is a static
sibling of the reactive form, so parent updates do not own its rendered regions.
The package publishes `dist/cem-action.html` and `dist/cem-action-gallery.html`;
the native build rewrites page-level import maps. Both pages load the canonical
action definition. Source files and build maps ship alongside them. No dependency on `custom-element` is added.

The select companion is implemented at
`packages/cem-components/playgrounds/cem-select.html` and ships as
`dist/cem-select.html`. Its form edits public options on one retained canonical
select, with grouped, single-listbox and multiple-listbox examples alongside a
source-only view. The action gallery is owned at
`packages/cem-components/playgrounds/cem-action-gallery.html`; its attribute
inventory and executable examples must match the canonical declaration.

The combined release document remains next work in [todo.md](todo.md).
The original Material action URL remains available and links the playground. In the standalone legacy adapter archive, that link opens the
repository source; the CEM component package includes both interactive pages.
