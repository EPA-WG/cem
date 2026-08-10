# Navigation Focus-Visible Contract

## Scope

This contract closes the Phase 4 `navigation:focus-visible` state for
`cem-nav` and `cem-tabs`. Keyboard focus adds an external indicator to an
existing navigation owner. It does not navigate, select a tab, toggle a
disclosure, activate a command, or create component runtime state.

## Interaction owners

Focus paint belongs to the same rendered native owners as navigation hover:

- direct `a[href]` and enabled `button` children of `cem-nav > nav`;
- direct `a[href]` and enabled `button` children of
  `cem-nav > nav > .cem-nav__content`;
- the enabled native `.cem-nav__disclosure` button; and
- direct enabled `button[role="tab"]` children of the `cem-tabs` tablist.

`cem-nav`, its rendered `nav`, `.cem-nav__content`, `cem-tabs`, and its
rendered tablist are structural containers. They may match `:focus-within`, but
they receive no navigation focus declarations.

## Token audit and adoption

The generated theme catalog already exposes the required semantics. D5 owns
the keyboard-focus thickness and external placement through
`--cem-stroke-focus` and `--cem-stroke-indicator-offset`. The zebra focus
category owns the mode-aware ring color through `--cem-zebra-color-1`.

These endpoints describe focus independently of navigation default, hover,
current, selected, and disabled fill/text semantics. No new navigation token,
raw component value, local custom property, or CSS exception is required.

## Selector and state behavior

The public stylesheet applies the ring only when the actual native owner
matches `:focus-visible`. It uses `button:enabled` so native-disabled buttons
cannot acquire focus paint. The ring uses `outline`, not border, padding,
shadow, transform, or generated content, and therefore does not alter owner or
container geometry.

Focus is an independent paint channel. A current link and selected tab retain
their current fill/text pair while focused. Pointer hover can replace that
fill/text pair with the corresponding hover pair without changing the focus
ring. Leaving hover restores the focused default/current pair; moving focus
restores the previous owner's unfocused paint. Held native activation likewise
uses the independent fill/text treatment defined by the
[navigation active contract](./navigation-active-contract.md) without replacing
the ring.

An authored `aria-disabled="true"` link or enabled button remains in the tab
order and intentionally keeps its focus indicator while component behavior
suppresses activation. Native-disabled buttons are skipped and never match the
focus selector. See the
[navigation disabled contract](./navigation-disabled-contract.md).

## Forced colors

In `forced-colors: active`, focused navigation owners retain the D5 focus width
and offset while the outline color maps to `CanvasText`. Existing current,
selected, hover, and disabled system-color mappings remain independent.
Structural wrappers remain unpainted.

## Executable acceptance

The focused Chromium state fixture proves:

- real Tab traversal through direct nav links, the disclosure button,
  disclosed content, and tab buttons;
- native-disabled navigation controls are skipped;
- exact resolution of the D5 width/offset and zebra focus-color tokens;
- current-link, selected-tab, disclosure, and hover coexistence;
- restoration after hover leave and after focus moves;
- stable owner, wrapper, and host geometry and DOM/ARIA;
- stable serializable component runtime snapshots; and
- absence of click, input, change, and component lifecycle mutation events.

The navigation forced-colors Chromium gate repeats the full keyboard order,
including focusable ARIA-disabled link and selected-tab boundaries, and
verifies `CanvasText`, tokenized width/offset, native-disabled skipping,
restoration, geometry, DOM/ARIA, wrapper isolation, and event/state absence.

## Failure conditions

The contract fails if focus paint lands on a structural wrapper, a
native-disabled button enters the keyboard order, current/selected/expanded
state changes, hover replaces the focus ring, focus or restoration changes
geometry/DOM/runtime state, a mutation event fires, forced colors lose the
system outline, or normal component CSS uses an unknown/raw/local value.
