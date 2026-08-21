# Tabs Contract

## Owner and author vocabulary

`cem-tabs` is the public local-view switching owner. Its only authored element
children are direct `cem-tab` payloads. Each tab requires a non-empty unique
`value` identity and `label`; presence-only `disabled` makes that tab
unavailable. `cem-tab` is otherwise inert: its default content becomes one
generated panel and it owns no focus, selection, event, form, route, loading,
or persistence behavior.

The host accepts `orientation="horizontal|vertical"` (horizontal by default)
and a zero-based `selected-index`. `selectedIndex` is a silent reflected
property. Missing selection defaults to the first enabled tab. Out-of-range or
disabled requests resolve to the nearest enabled tab by searching forward with
wrap, while the authored attribute remains unchanged. At least one enabled tab
is required.

Malformed direct vocabulary, missing or duplicate values/labels, an empty tab
set, or an all-disabled tab set produces no interactive tab surface and one
deterministic console warning per distinct payload. Nested `cem-tab` elements
are panel content, not additional tabs.

Generated tab and panel IDs remain stable for the lifetime of every retained
host value across live append, removal, label, and disabled changes. Selection
is positional: a dynamic payload change keeps the effective selected index when
possible, skips a newly disabled tab, and clamps after removal. Arbitrary
in-place reordering is not part of the v1 contract. Application routes and data
remain outside the owner.

## Selection, interaction, and event contract

Each tab is one exact generated native `button[type="button"][role="tab"]`.
The tablist and panel container are structural; they never become hover, focus,
activation, or event owners. Clicking an enabled non-current tab updates
`selected-index`, keeps focus on that tab, and emits one bubbling/composed
`cem-tab` event with serializable `{ value, index, previousIndex }` detail.
Native Enter and Space use the same click path. Current-tab activation,
disabled activation, pointer/focus movement, dynamic payload changes, and
programmatic attribute/property changes emit nothing.

Activation is deliberately manual. This matches the pinned Angular Material
keyboard contract and keeps focus browsing separate from potentially expensive
Studio panel changes. Exactly one enabled tab participates in the roving tab
order. Horizontal Left/Right and vertical Up/Down move focus, wrap, and skip
disabled tabs without selecting. Home/End focus the first/last enabled tab in
either orientation. The other-axis arrows remain untouched. Enter or Space
activates the focused tab. When focus leaves the complete owner, the selected
tab becomes the tablist's entry stop again.

Programmatic selection normally does not move focus. If a selection or dynamic
payload change would hide the panel that currently contains focus, focus moves
to the newly selected tab after rendering so focus is never retained in hidden
content. Selection preserves each still-authored panel subtree and its native
control state.

## Accessibility contract

The exact generated owners implement the WAI-ARIA tabs pattern:

- one labeled `role="tablist"` exposes exact
  `aria-orientation="horizontal|vertical"`;
- every generated native button has `role="tab"`, stable `aria-controls`, and
  exact `aria-selected="true|false"`;
- every generated `role="tabpanel"` has reciprocal `aria-labelledby`, stable
  identity, and `tabindex="0"`;
- only the selected panel is visible; all other panels use native `hidden`;
- disabled tabs use native `disabled`, remain visible, and have no tab stop;
  and
- exactly one enabled tab has `tabindex="0"`.

Tab labels are authored plain strings. Rich panel content remains authored CEM
payload and is never derived from the label or inserted as HTML. The component
does not own deletion, popup menus, lazy loading, label pagination, routing, or
navigation-link behavior.

## Theme-token audit

No new theme token is required. Tabs use the established semantic families:

| Visual role | Domain | Token family |
| --- | --- | --- |
| Default/hover/active/selected/selected-hover/selected-active/disabled paint | D0 | existing `--cem-navigation-item-*` endpoints |
| Tablist and panel spacing | D1 | existing related/group/block gaps and control padding |
| Minimum interactive target and separation | D2 | existing control and coupling endpoints |
| Tab shape | D3 | existing control bend |
| Selection/focus/panel geometry | D5 | existing divider and focus strokes |
| Tab and panel text | D6 | existing UI/body typography |

Selected and focus-visible states remain independent: selection uses current
navigation paint and a stable divider-width indicator; focus uses the external
focus outline. Hover and active paint do not change geometry or selection.
There is no component animation. In forced colors, selected maps to
`SelectedItem`/`SelectedItemText`, hover/active to
`Highlight`/`HighlightText`, disabled to `Canvas`/`GrayText`, the focus outline
and panel text to `CanvasText`, and the selected indicator to `Highlight`.

## Evidence matrix

Focused browser and forced-color gates must prove:

1. strict payload normalization, stable reciprocal IDs, one selected tab and
   visible panel, one roving tab stop, and inert `cem-tab` payloads;
2. click, Enter, and Space selection with exactly one serializable `cem-tab`
   event and silence for current/disabled/programmatic paths;
3. horizontal/vertical Arrow/Home/End focus movement, wrap, disabled skipping,
   manual activation, and selected-tab entry restoration;
4. programmatic selection, clamping, disabled/removal recovery, stable retained IDs,
   retained panel control state, and hidden-panel focus recovery;
5. exact ARIA roles, labels, orientations, controls/labelledby references,
   native hidden/disabled behavior, and malformed-authoring failure closure;
6. independent hover, focus-visible, active, selected, and disabled paint with
   stable geometry and no transient semantic mutation; and
7. forced-color mappings, reduced-motion behavior, token provenance, package
   behavior artifact, public detail type, and clean package consumption.

## Reference boundary

- Pinned Angular Material `22.1.1` tabs documentation at commit
  `0b67c3c38141049657b1167479accc80e455d2bd` defines arrow/Home/End focus
  movement and Space/Enter switching.
- The WAI-ARIA Authoring Practices tabs pattern defines the tablist/tab/tabpanel
  relationships, roving tab order, orientation behavior, manual activation,
  and focusable-panel guidance.
- The existing CEM navigation state, style, accessibility, and forced-color
  contracts remain authoritative for shared paint and input-state behavior.
