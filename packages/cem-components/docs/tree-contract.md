# Tree Contract

## Owner and author vocabulary

`cem-tree` is the public generic expandable-hierarchy owner. Its only authored
element children are direct `cem-tree-item` payloads, and each item may recursively
contain only more `cem-tree-item` payloads. Every item requires a non-empty `value`
and `label`; values are unique across the whole tree and may not contain
whitespace so they remain safe members of host token lists. `cem-tree-item` is
otherwise inert and owns no independent role, focus, event, selection, loading,
form, data-source, or fetching behavior.

The host requires a non-empty `label` and accepts presence-only `disabled`, a
space-separated `expanded-values`, `selection="none|single|multiple"` (`none` by
default), a space-separated `selected-values`, and a localizable `loading-label`
(`Loading` by default). `expandedValues` and `selectedValues` are silent reflected
string-array properties. Unknown or duplicate list members are ignored for
rendering without rewriting authored attributes. Single-selection mode projects
the first matching selected value in tree order; multiple-selection mode projects
every matching value. Selection mode `none` exposes no selection state.

An item accepts presence-only `expandable`, `disabled`, and `loading`. Nested
children imply expandability; explicit `expandable` represents an application
branch whose stable, initially empty group may receive children later. Loading
remains an application-authored fact and never starts a request, timer, observer,
or child synthesis. Malformed
recursive vocabulary, duplicate/whitespace-containing identities, missing labels,
an empty root set, or a missing tree label produces no interactive tree and one
deterministic warning per distinct payload/owner signature.

## Expansion, activation, and event contract

Every node renders as one exact native `button[type="button"][role="treeitem"]`.
Its surrounding item and `role="group"` elements are structural: they own layout
and hierarchy only, never hover, focus, activation, state paint, or component
events. A parent treeitem owns its stable sibling group through `aria-owns`,
including an explicit expandable branch whose children have not arrived yet.
Collapsing hides rather than destroys that group and its nodes.

Pointer click or native Enter/Space on an enabled parent toggles that value in
`expanded-values`, retains focus, and emits one bubbling/composed
`cem-tree-toggle` event with serializable `{ value, expanded }` detail after the
attribute changes. The same activation on an enabled leaf emits one bubbling/
composed `cem-tree-activate` event with `{ value }` detail and mutates no tree or
selection state. Application code decides whether leaf activation changes
`selected-values`, navigates, or performs another domain action.

Programmatic expanded/selected attribute or property changes are silent. Current
parent activation never emits leaf activation, current leaf activation never
changes selection, and disabled, pointer enter/leave, focus, typeahead, and plain
focus movement emit nothing. Loading does not disable a node; an application may
expand a loading branch, receive its toggle event, and replace or append payload
children while node/group IDs and surviving focus remain stable.

## Keyboard and focus contract

Exactly one visible enabled treeitem participates in the page tab order. Initial
entry prefers the first visible selected item and otherwise the first visible
enabled item. `ArrowDown`/`ArrowUp` move without wrapping through visible enabled
nodes. `Home`/`End` move to the first/last visible enabled node. Right Arrow opens
a closed parent without moving focus, moves from an open parent to its first
enabled child, and does nothing on a leaf. Left Arrow closes an open parent,
otherwise moves to the nearest enabled ancestor, and does nothing at a root.

Printable-character typeahead searches visible enabled labels from the node after
the current item, wrapping only the search. Rapid characters form one
case-insensitive prefix; after the buffer expires a new search begins. Focus
movement and typeahead never activate, select, expand, or emit. When programmatic
collapse hides the focused descendant, roving recovery moves to its nearest
visible enabled ancestor without stealing document focus unless focus was already
inside the tree. Host-disabled trees expose no tab stop; item-disabled nodes stay
perceivable through `aria-disabled="true"` but are skipped and suppress all input.

## Accessibility contract

The exact root exposes `role="tree"`, the required accessible name, and
`aria-multiselectable="true"` only in multiple-selection mode. Exact native node
buttons expose `role="treeitem"`, stable IDs, explicit `aria-level`,
`aria-posinset`, and `aria-setsize`, plus `aria-expanded` only for parents.
Generated sibling child containers expose stable `role="group"` IDs and are
referenced by the parent treeitem's `aria-owns`. Leaf nodes never claim expanded
state.

Single/multiple selection projects truthful `aria-selected="true|false"` on all
nodes without coupling focus to selection; selection-none projects no
`aria-selected`. Loading nodes expose `aria-busy="true"` and visible localizable
status text. Selected and expanded indicators remain visibly shape-distinct and
are hidden from accessibility APIs because the ARIA states already provide the
same facts. Button content supplies each treeitem's accessible name. Generic site
navigation remains a disclosure/nav concern; this composite is reserved for
interfaces that require full tree keyboard behavior.

## Theme-token audit

The pre-CSS audit selected generic interactive-content semantics rather than
navigation semantics. A tree may represent files, resources, taxonomy, settings,
or other data without moving location. The existing content family covered
default, hover, selected, selected-hover, and disabled, but lacked held-active
feedback for both unselected and selected owners. Four D0 endpoints are therefore
canonical before component CSS consumes them:

| Visual role | Domain | Token |
| --- | --- | --- |
| Default/hover/selected/selected-hover/disabled node paint | D0 | existing `--cem-content-interaction-*` endpoints |
| Held unselected activation | D0 | new `--cem-content-interaction-active-{background,text}` |
| Held selected activation | D0 | new `--cem-content-interaction-selected-active-{background,text}` |
| Root/group/indent spacing | D1 | existing related/group/block gaps and control padding |
| Node target and hierarchy safety | D2 | existing coupling zone/guard endpoints |
| Node shape | D3 | existing control bend |
| Focus and selected marker geometry | D5 | existing focus/indicator strokes |
| Label/loading text | D6 | existing UI/tag typography |

No tree-specific palette, separator, navigation, geometry, animation, or z-index
token is required. The structural group is not a divider and receives no state
paint. The completed content-interaction category represents every required state,
so this contract adds no component CSS exception.

## Forced-colors boundary

Default nodes resolve to `ButtonFace`/`ButtonText`; hover and held activation use
`Highlight`/`HighlightText`; selected nodes use
`SelectedItem`/`SelectedItemText`; selected hover/active also use explicit system
highlight paint; disabled nodes resolve to `ButtonFace`/`GrayText`; and keyboard
focus uses a `CanvasText` outline. Expansion, selected, and loading remain
independently exposed through marker/status geometry and ARIA states. Forced colors
do not change visible-node order, dimensions, expanded/selected values, focus,
events, or payload identity.

## Focused fixture and assertion matrix

`tests/tree/contract.html`, its browser spec, and the dedicated forced-colors gate
form the executable contract:

| Concern | Required evidence |
| --- | --- |
| Owners | strict recursive payload, exact native treeitems, structural stable groups, explicit levels/positions/sets, malformed rejection |
| Pointer/events | parent toggle/event, leaf activation/event, exact button enter/leave/active ownership, disabled silence |
| Keyboard | one roving visible tab stop, Up/Down, parent/child Left/Right, Home/End, typeahead, disabled skip, native Enter/Space |
| Application state | silent selected/expanded value control, single/multiple/none projection, loading/late children, no fetching or selection mutation |
| State/stability | expanded/selected/loading/hover/active/focus/disabled coexistence, persistent node/group IDs, stable transient geometry |
| Forced colors | system default/hover/active/selected/focus/disabled paint, redundant markers/status, no animation/z-index dependence |

## Benchmark references

- [Pinned Angular Material tree source](https://github.com/angular/components/blob/0b67c3c38141049657b1167479accc80e455d2bd/src/material/tree/tree.md)
- [WAI-ARIA Tree View Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/treeview/)
- [ARIA in HTML role allowances](https://www.w3.org/TR/html-aria/)
