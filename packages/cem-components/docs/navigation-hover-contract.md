# Navigation Hover Contract

## Scope

This contract closes the Phase 4 `navigation:hover` state for `cem-nav` and
`cem-tabs`. Hover is optional pointer feedback on an existing navigation owner.
It does not select a tab, change the current page, expand navigation, activate a
command, or create component runtime state.

## Interaction owners

The owners are the rendered interactive elements, never their structural
containers:

- direct `a[href]` and `button` children of `cem-nav > nav`;
- direct `a[href]` and `button` children of
  `cem-nav > nav > .cem-nav__content`, including disclosed navigation content;
- the native `.cem-nav__disclosure` button, which is a direct nav child; and
- direct native `button[role="tab"]` children of the `cem-tabs` tablist.

`cem-nav`, its rendered `nav`, `.cem-nav__content`, `cem-tabs`, and its rendered
tablist may match `:hover` because a descendant is pointer-designated. They are
structural wrappers, not paint owners, and receive no navigation-item state
declarations.

The contract does not reach through arbitrary nested components. A nested
component retains its own state family unless its public contract explicitly
adopts navigation-item semantics.

## Token audit and adoption

Before component CSS, the generated catalog was audited for both navigation and
action endpoints. D0 exposed complete action-intent hover pairs, but those
tokens mean “do something” and cannot independently theme links/tabs or preserve
current/selected meaning. No generated navigation category existed.

D0 now owns ten required `--cem-navigation-item-*` endpoints: paired
`background` and `text` values for `default`, `hover`, `current`,
`current-hover`, and `disabled`. Current links (`aria-current` except the
explicit value `false`) and selected tabs (`aria-selected="true"`) share the
current semantics. Disabled wins over both current and hover. The component CSS
uses no raw normal-mode color and no component-local custom property.

This is theme adoption, so no entry is added to the component CSS exception
queue.

## Selector and state precedence

The public component stylesheet binds only `background-color` and `color` on
the accepted owners. Enabled hover requires:

- links without `aria-disabled="true"`;
- native buttons matching `:enabled` and without `aria-disabled="true"`; and
- the real owner itself matching `:hover`.

The normal-mode cascade is `default < hover < current < current-hover <
disabled`. The distinct current-hover pair makes pointer feedback observable
without discarding current/selected semantics. Focus-visible uses the native
owner's independent, tokenized outline channel; hover must not replace it. See
the [navigation focus-visible contract](./navigation-focus-visible-contract.md).

ARIA-disabled links and tabs are presentation-suppressed by this slice. Their
activation and tab-stop policy remains the responsibility of the later
`navigation:disabled` behavior contract; this hover slice does not synthesize
that behavior.

## Forced colors

In `forced-colors: active`, the same owner selectors map:

- current/selected to `SelectedItem` / `SelectedItemText`;
- enabled hover, including current/selected hover, to `Highlight` /
  `HighlightText`; and
- disabled to `Canvas` / `GrayText`.

The rules leave focus outlines intact and do not add border, padding, size,
shadow, transform, generated content, or motion. Structural wrappers remain
unpainted.

## Executable acceptance

The focused Chromium state fixture proves:

- trusted pointer enter and leave for enabled links, the native disclosure
  button, tabs, and disabled owners;
- exact generated-token resolution and 4.5:1 hover text/fill contrast;
- current-link and selected-tab coexistence, restoration, and disabled
  suppression;
- focus-visible coexistence on every enabled owner;
- stable owner, wrapper, and host geometry and HTML/ARIA;
- stable serializable runtime snapshots; and
- absence of click, input, change, and component lifecycle mutation events.

The dedicated Chromium forced-colors gate verifies the system-color mapping,
restoration, focus coexistence, disabled suppression, trusted pointer boundary
events, geometry, DOM/ARIA, and wrapper isolation. Raw Playwright/Chromium does
not expose the native-button `:hover` pseudo-state in this headless path even
though it emits trusted pointer boundary events; the forced-colors gate uses
Chromium's inspection-only forced pseudo-state for button paint checks. The
ordinary component browser fixture independently proves real native-button
`:hover` behavior.

## Failure conditions

The contract fails if a structural wrapper receives the treatment, a disabled
owner acquires enabled hover colors, current/selected state is lost, focus paint
changes, geometry or DOM/runtime state mutates, a required token is absent, or
normal-mode component CSS introduces a raw/local value.
