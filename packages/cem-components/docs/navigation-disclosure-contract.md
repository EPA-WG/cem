# Navigation Disclosure Contract

**Status:** Accepted Phase 4 design; implementation pending. This contract is
promoted by [`docs/todo.md`](../../../docs/todo.md) and governs the next
`navigation:expanded` implementation slice.

## Decision

`cem-nav` keeps its current labeled navigation landmark and projected content by
default. The presence-only `collapsible` attribute opts the entire landmark into
one disclosure controlled by a native `<button type="button">` inside the rendered
`<nav>`. The button owns the current `aria-expanded="true|false"` state, and a
stable sibling content container owns the projected navigation payload and the
matching `hidden` state.

This is a region-wide disclosure, suitable for a compact or responsive navigation
surface. Parent-scoped navigation groups, nested submenus, menu/menubar roles, and
application routing behavior are outside v1.

## Alternatives considered

| Shape | Decision | Reason |
| --- | --- | --- |
| `cem-nav[collapsible]` with a native button | Accepted | Preserves the 32-component MVP, uses the existing default slot, exposes exact `aria-expanded` evidence, and needs only existing declarative boolean slices. |
| Parent-scoped `cem-nav-group` payload | Deferred | Multiple independent groups require new recursive group vocabulary, labels, nesting rules, and payload projection beyond the current `cem-nav` contract. |
| Native `<details>/<summary>` | Rejected for this slice | Native details correctly owns open/closed behavior, but its expanded accessibility state is implicit. The current [ARIA in HTML `summary` rules](https://www.w3.org/TR/html-aria/#el-summary) do not allow authors to add `aria-expanded` to a conforming summary-for-details, so it cannot also provide the explicit reflected evidence required by this package contract. |
| Menu button or menubar | Rejected | Ordinary site navigation links do not need the composite focus and keyboard contract implied by `menu`, `menubar`, and `menuitem` roles. |

The accepted shape follows the [WAI-ARIA disclosure pattern](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/)
and its [navigation disclosure example](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/examples/disclosure-navigation/):
the controlling element is a button, it exposes exact expanded state, and the
navigation links retain their native link semantics rather than becoming a menu
widget.

## Author API

```html
<cem-nav label="Primary navigation" collapsible expanded>
  <a href="/overview">Overview</a>
  <a href="/assets">Assets</a>
</cem-nav>
```

| Attribute | Contract |
| --- | --- |
| `label` | Required non-empty accessible name for both the navigation landmark and its disclosure button. The button text is plain text derived from this value. |
| `collapsible` | Presence enables the region-wide disclosure. Absence preserves the existing passive projection byte-for-byte. |
| `expanded` | Optional initial state for collapsible mode. Presence starts open; absence starts closed. It is ignored in passive mode. |

`collapsible` and `expanded` use WHATWG boolean presence semantics. The strings
`collapsible="false"` and `expanded="false"` still mean present/true and are
invalid authoring attempts to express false.

The default payload remains ordinary navigation content: links, lists of links,
and related non-composite actions. No `cem-nav-group` child vocabulary is accepted
in v1. The disclosure button cannot contain author-projected controls or rich
interactive content because its label comes only from the `label` attribute.

## Normalization algorithm

1. If `collapsible` is absent, render the existing named `<nav>` and project the
   default slot unchanged. Do not add a button, `aria-expanded`, or hidden wrapper.
2. If `collapsible` is present, keep the same named `<nav>` landmark and render a
   native `type="button"` disclosure control followed by one stable content
   container containing the existing default-slot projection.
3. Resolve current state as the serializable `expanded` slice when it exists;
   otherwise use the authored `expanded` attribute. An absent attribute therefore
   initializes the disclosure as closed.
4. Materialize `aria-expanded="true"` on the button and omit `hidden` from the
   content container when open. Materialize `aria-expanded="false"` and the native
   `hidden` attribute when closed. Never serialize a boolean HTML attribute as the
   string `"false"`.
5. A native button `click` writes the opposite boolean to the `expanded` slice.
   Once present, the slice is authoritative for subsequent renders and is recorded
   in the normal serializable event payload. Do not dispatch a component-specific
   event when the native click already carries the transition.
6. Keep the trigger and controlled container structurally stable across renders so
   light-DOM diffing can retain button focus. The projected payload remains in the
   data island and controlled container; closing hides it rather than replacing it
   with alternate content.

The v1 contract does not add `aria-controls`. The disclosure pattern treats it as
optional, and omitting it avoids inventing a component-local ID contract before
stable per-instance author-facing IDs are defined.

## Accessibility and interaction

- The `<nav>` retains its implicit navigation landmark and accessible `label` in
  both modes. `aria-expanded` belongs to the focusable button, never the landmark.
- The button relies on native activation: pointer click, `Enter`, and `Space`
  toggle it. No imperative keyboard handler, roving `tabindex`, arrow-key loop,
  `Escape` handler, or focus transfer is introduced.
- Focus remains on the disclosure button after a user toggle. When open, normal
  `Tab` order reaches the projected links; when closed, native `hidden` removes
  them from rendering and sequential focus navigation.
- Navigation links keep native link roles. Do not add `role="menu"`,
  `role="menubar"`, `role="menuitem"`, `aria-haspopup`, or composite-menu
  keyboard behavior.
- Before upgrade, authored links remain readable fallback content. Collapsing is an
  enhanced behavior applied only after the declarative component upgrades.

## Lifecycle and ownership boundaries

`cem-nav` is the only Phase 4 owner of `navigation:expanded`:

- `cem-app-bar` remains a banner containing title/context and global actions. Any
  disclosure action inside it owns its own action state; the banner does not.
- `cem-tabs` owns selected tab/panel state and the tab keyboard pattern. Expanded
  navigation groups are not tab panels.
- `cem-nav[collapsible]` remains a Navigation component and does not participate in
  forms or contribute data to `FormData`.
- Multiple independently collapsible groups, nested navigation trees, submenu
  relationships, and menu/menubar composite focus require a separate accepted
  contract. They MUST NOT be approximated by nesting disclosure buttons or adding
  partial menu roles in this slice.

The implementation MUST use existing CEM-ML/CEM-QL conditional rendering and a
declarative boolean `slice-event` transition. If the red fixture shows that stable
hidden content, native button activation, focus retention, or boolean payloads
require new substrate behavior, stop and promote that capability as a separate
decision rather than adding component-specific imperative code.

## Executable acceptance

The implementation slice is complete only when a focused browser test proves:

- passive `cem-nav` output remains unchanged;
- `collapsible` adds exactly one named native button inside the same named
  navigation landmark;
- absent/present `expanded` initializes closed/open state respectively;
- pointer, `Enter`, and `Space` activation toggle exact button
  `aria-expanded="false|true"` and matching content visibility;
- focus stays on the button while open content enters normal tab order and closed
  content leaves it;
- the boolean `expanded` slice and serialized click payload match the rendered
  state; and
- no menu roles, form participation, parent-scoped group vocabulary, or
  component-specific keyboard handlers are introduced.
