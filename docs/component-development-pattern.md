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

## Component index (required)

`packages/cem-components/index.html` is the component listing. Every component
introduction or migration must add an entry linking to its **Full examples and
variation matrix** page:

```html
<li><a href="./playgrounds/cem-field-gallery.html">cem-field</a></li>
```

The visible link text must exactly match the component name (`cem-<name>`).
Keep entries alphabetical and point directly to `playgrounds/<tag>-gallery.html`.
The gallery links to the property playground, where the canonical source is
available. Update the index in the same change that introduces the gallery,
and verify that each link resolves.
Workflow examples may remain below the component listing.

## Companion property playground

Provide a separate page under `playgrounds/`, outside the canonical two-file
component folder. The page loads the production declaration through page-level
module maps, so IDE HTTP previews and packaged previews use the same convention.

The page contains:

- One live component instance and a property form using shared CEM controls.
- Visible radio groups for mutually exclusive enumerated options, so users
  can compare choices without opening a menu. Use shared CEM controls.
- Controls for every supported public option. For action, start with intent,
  bend, size, label, type, disabled, pending and selected.
- The fetched canonical XHTML source, displayed without executing a second copy
  (`cem-demo-element` supports `demo="false"`).
- A mandatory link labeled **Full examples and variation matrix** to
  `<tag>-gallery.html`, plus a link to the automated stories.

Keep option bindings declarative in XHTML/CEM-ML. The property form changes the
public attributes or projected content of the real component. Hover, active and
keyboard focus remain native interactions; the playground must not add fake
production state attributes. Explain how to exercise those states next to the
preview. A missing reusable control or binding capability is work for the shared
component/runtime, not page-local JavaScript behavior.

## Full examples and variation matrix (required)

Every component must have `playgrounds/<tag>-gallery.html`, linked from its
property playground with the exact label **Full examples and variation matrix**.
A component introduction or migration is incomplete without this page and link.
The gallery links back to the property playground and to its automated stories.
Immediately after the page heading, before navigation links or examples, include
a brief paragraph explaining what the component does and how it relates to
other components. Link related component names to their galleries when available;
describe shared behavior, complementary uses, or when to choose another control.

The gallery must import the canonical production declaration and include:

- An inventory of supported public attributes and slots, with examples of their
  values, defaults, boolean presence and relevant combinations.
- Useful live examples with inspectable source, including form integration,
  projected content and grouped choices where supported.
- A side-by-side variation matrix covering supported appearances and default,
  disabled, invalid, pending, readonly and selected states where supported.
  Cover all five theme modes through the shared theme switch or fixed theme rows.
- Instructions to exercise real hover, press, keyboard focus and expansion.
  Do not simulate these interactions with production attributes.

Use the existing component contract to distinguish implemented behavior from
future work. The gallery must not introduce a second component implementation
or page-local JavaScript UI behavior. Keep inspectable example templates in a
static page region; fixed theme rows provide comparison without reprojecting
those templates through a reactive parent. Ship both gallery source and built HTML,
export the built page, and verify the playground link, rendered examples and
variation matrix in repository and isolated-package previews. Large galleries
may configure the shared runtime processing queue through `processingPoolPolicy`;
the larger galleries use `queueSize: 512` for their simultaneous examples.
The gate must cover every canonical component so new introductions cannot omit
their gallery.

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
Preserve the original source, companion playground and required gallery in the distribution.
The bundle is a build artifact derived from canonical declarations through the
shared CEM AST transformation pipeline; never edit it by hand or maintain a
second set of templates. Verify individual and bundled loading, dependency URLs,
style ownership and duplicate-registration behavior before publishing it.

## Delivery status

The source/test split is implemented for all eight canonical components:
`cem-action`, `cem-select`, `cem-theme-switch`, `cem-icon`, `cem-icon-button`,
`cem-menu-item`, `cem-field` and `cem-text-field`.
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

The combined release document ships as `dist/components.xhtml`, exported through
`@epa-wg/cem-components/components.xhtml`. The native CEM AST graph in
`packages/cem-components/build/` derives it from all canonical XHTML declarations.
The bundle playground ships as `dist/cem-bundle.html`. Source and isolated-package
checks compare template text and metadata, source bases, stylesheet ownership and
duplicate registration. The approved [source-context decision](component-bundle-source-context.md)
uses `xml:base` on generated containers to preserve relative dependencies.
The original Material action URL remains available and links the playground. In the standalone legacy adapter archive, that link opens the
repository source; the CEM component package includes both interactive pages.

All seven canonical components now provide linked galleries. The playground
verification gate discovers canonical component folders and checks each gallery
in source and isolated-package previews.
