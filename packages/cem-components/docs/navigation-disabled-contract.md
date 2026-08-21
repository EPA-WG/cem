# Navigation Disabled Contract

## Scope

This contract closes the Phase 4 `navigation:disabled` state for `cem-nav`. It
applies to the real rendered navigation owners defined by the hover, focus, and
active contracts. It does not make a whole navigation host inert and does not
introduce a host-level disabled or disclosure-disabled API. `cem-tabs` uses
native-disabled generated buttons under its dedicated
[tabs contract](./tabs-contract.md).

## Owner policy

Authors use native `disabled` whenever the navigation owner is a button that
must leave sequential focus. The browser then owns focus skipping, pointer and
keyboard activation suppression, form behavior, and programmatic `click()`.

`aria-disabled="true"` expresses an unavailable owner that should remain
discoverable. CEM deliberately preserves its authored tab stop and visible
focus indicator; it does not add, remove, or rewrite `tabindex`. Because ARIA
does not suppress behavior by itself, the owning `cem-nav` host suppresses
activation.

The behavior applies only to:

- direct `a[href]` and `button` children of `cem-nav > nav`;
- direct `a[href]` and `button` children of
  `cem-nav > nav > .cem-nav__content`.

Nested components and controls behind an additional authored wrapper retain
their own behavior contract. Structural nav/content/tablist wrappers are never
disabled owners.

## Activation boundary

Each navigation host installs one capture-phase behavior on itself. When an
accepted direct owner has exact `aria-disabled="true"`, it prevents the default
action and stops the activation before it reaches the target or subsequent
bubble-phase application listeners for:

- primary `click`, including trusted pointer release and programmatic
  `element.click()`;
- auxiliary click;
- `Enter` keydown and keyup on links and buttons; and
- Space keydown and keyup on native buttons.

Space on a link remains its native non-activation key and is not intercepted.
Tab and other non-activation keys remain available. The behavior creates no
slice, synthetic event, DOM replacement, ARIA mutation, or tabindex mutation.

Capture is required because a bubble listener on the host would run after an
authored target listener. Stopping at the owner host gives ARIA-disabled items
native-disabled-like activation absence while preserving their intentional
focus discoverability. An earlier ancestor capture listener can still observe
the event according to normal DOM dispatch order; it sees the event as
default-prevented after dispatch completes.

## Form boundary

A direct ARIA-disabled button is guarded even if authored as a submit button:
pointer, programmatic, Enter, and Space activation cannot submit its ancestor
form. The behavior neither registers `cem-nav` as a form-associated element nor
changes `FormData`. Authors should use `type="button"` for ordinary
navigation commands and native `disabled` when discoverability is not needed.

## Theme and cascade audit

D0 already provides required navigation disabled background/text semantics:

- `--cem-navigation-item-disabled-background`; and
- `--cem-navigation-item-disabled-text`.

No new theme endpoint or component CSS exception is needed. Normal component
CSS binds those tokens to the actual disabled owners. Enabled nav-button hover
and active selectors explicitly exclude `aria-disabled="true"`, and later
current-disabled selectors ensure disabled paint wins when `aria-current`
coexists.

Focus remains an independent outline channel for ARIA-disabled owners. Native
disabled buttons are skipped and cannot acquire focus paint. The disabled
state changes no geometry.

## Forced colors

In `forced-colors: active`, native- and ARIA-disabled owners use `Canvas` and
`GrayText`. An ARIA-disabled current link keeps disabled paint rather than its
current treatment, while its retained focus indicator uses
`CanvasText`. Native-disabled buttons remain outside the tab order.

The dedicated forced-colors fixture isolates system paint and focus behavior.
The upgraded component browser fixture proves the package-owned activation
behavior itself.

## Executable acceptance

The focused Chromium fixture proves:

- exact tab order with ARIA-disabled links/buttons retained and native
  disabled buttons skipped;
- visible focus, disabled tokens, current-state coexistence, and stable
  geometry/DOM/ARIA/runtime snapshots;
- trusted pointer and programmatic click cancellation before target and
  application listeners;
- Enter suppression on links/buttons, Space suppression on buttons, and native
  non-activating Space behavior on links;
- absence of navigation, form submit, FormData, input/change,
  and component lifecycle mutation; and
- retained owner identity and focus after every suppressed activation.

The forced-colors gate repeats disabled/current paint, focus order,
native-disabled skipping, restoration, wrapper isolation, and geometry checks.

## Failure conditions

The contract fails if an ARIA-disabled direct owner activates, a native-disabled
button enters the tab order, target/application bubble activation listeners run,
default navigation or form submission occurs, `tabindex` is rewritten,
current/ARIA/runtime state mutates, disabled paint loses to an enabled
state, forced colors lose system paint, geometry changes, behavior reaches an
unrelated nested control, or component CSS introduces a raw/local value.
