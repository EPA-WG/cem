# Accordion contract

[cem-accordion](../src/components/cem-accordion/cem-accordion.xhtml) is a declarative
vertical container for independent [cem-expansion](./expansion-contract.md) owners.
It adds no JavaScript group state or single-open behavior. Its named `role=group`
uses `label` (Sections by default), overridden by `aria-label`; its default slot
contains direct `cem-expansion layout="panel" indicator="start"` elements.

`fit="content"` uses natural height. `fit="fill"` requires a definite parent
height: closed sections retain their header size, open sections share remaining
space and each panel scrolls independently. With all sections closed, headers
stay at the top; a pane shorter than its headers scrolls the outer group so each
header remains reachable. DOM owners and panel contents survive disclosure.

Each expansion owns its own `expanded`, `disabled`, summary, panel and focus.
The summary is icon plus text phrasing content; `header-actions` contains sibling
context controls. Counts should precede actions. Native header Enter/Space toggles
only its section. Closing a section never selects a tree item or changes editor
ownership; a hidden focused descendant returns to its header.

The container consumes shared palette, typography and stroke tokens. Density and
control target geometry come from the canonical theme, including the precise-only
compact profile and coarse/hybrid fallback. Forced colors use Canvas/CanvasText.
No component-local public defaults or application CSS reach into child internals.

The [property playground](../playgrounds/cem-accordion.html),
[five-mode gallery](../playgrounds/cem-accordion-gallery.html) and
[colocated stories](../src/components/cem-accordion/cem-accordion.stories.ts) cover
independent opening, retained edits, action isolation, leading chevrons, bounded
panels, all-closed/short-pane behavior, accessible naming and collapse focus.
