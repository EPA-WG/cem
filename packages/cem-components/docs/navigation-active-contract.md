# Navigation Active Contract

## Scope

This contract closes the Phase 4 `navigation:active` state for `cem-nav` and
`cem-tabs`. Active is the transient native `:active` interval while an enabled
navigation owner is being pressed. It is not current-page state, tab selection,
focus, hover, or a durable runtime slice.

## Interaction owners

Active paint belongs to the same real owners as navigation hover and focus:

- direct `a[href]` and enabled `button` children of `cem-nav > nav`;
- direct `a[href]` and enabled `button` children of
  `cem-nav > nav > .cem-nav__content`;
- the enabled native `.cem-nav__disclosure` button; and
- direct enabled `button[role="tab"]` children of the `cem-tabs` tablist.

Nav hosts, rendered nav/content containers, tab hosts, and tablists are
structural wrappers. They may match ancestor interaction pseudo-classes but
receive no navigation active declarations.

## Disclosure ownership decision

`navigation:active` owns only the disclosure button's held native pseudo-class
and paint. Before release, the button, `aria-expanded`, controlled visibility,
serializable runtime snapshot, and event history must remain unchanged.

On release, the already-accepted `navigation:expanded` contract remains the
canonical owner: the trusted click toggles the boolean `expanded` slice,
`aria-expanded`, and controlled visibility while retaining button identity and
focus. This active fixture verifies that authorized boundary without redefining
or duplicating the expanded transition.

## Token audit and adoption

The existing navigation family covered default, hover, current,
current-hover, and disabled paint but had no active semantics. Binding action
tokens directly would couple navigation theming to action intent and could not
preserve a distinct current/selected active treatment.

D0 therefore adds four independently themeable endpoints:

- `--cem-navigation-item-active-{background,text}`; and
- `--cem-navigation-item-current-active-{background,text}`.

Their default formulas align with contextual and primary action progression,
respectively, but components bind only the navigation endpoints. No raw value,
component-local variable, or CSS exception is required.

## Cascade and timing

Enabled unselected owners use the active pair while current links and selected
tabs use current-active. Active follows hover at equal specificity; the
current-active rule follows current-hover. Disabled paint remains later and
wins over every enabled state. Focus-visible stays on its independent outline
channel.

The held interval changes only `background-color` and `color`. It does not
change geometry, DOM/ARIA, focus paint, component runtime state, or event
history. Pointer release emits the native click, restores hover while the
pointer remains, and restores default/current after pointer leave.

Native keyboard behavior is preserved rather than synthesized:

- `Enter` activates links and buttons; Chromium completes that activation on
  keydown, so the fixture verifies the trusted click and restored state rather
  than inventing a durable held interval.
- `Space` does not activate links.
- Native buttons expose an observable Space-held `:active` interval and click
  on keyup. Selected tabs retain selection; disclosure release delegates to
  the expanded contract described above.

ARIA-disabled and native-disabled owners keep disabled paint even if an
inspection path makes them match `:active`. The later `navigation:disabled`
contract owns their complete activation and tab-stop policy.

## Forced colors

In `forced-colors: active`, enabled active owners map to `Highlight` and
`HighlightText`. High-contrast modes may intentionally collapse hover and
active fill while native input feedback and the independent focus outline
remain available. Disabled owners remain `Canvas` / `GrayText`; current and
selected resting owners remain `SelectedItem` / `SelectedItemText`.

The forced-colors gate uses real trusted pointer holds for links. Raw
Playwright/Chromium does not expose the native-button pseudo-state in this
headless forced-colors path, so it uses Chromium's inspection-only forced
pseudo-state for disclosure/tab paint. The ordinary browser fixture proves real
native-button pointer and Space holds.

## Executable acceptance

The focused Chromium fixture proves:

- trusted pointer down/hold/release across default/current links, disclosed
  content, disclosure, unselected/selected tabs, and disabled owners;
- exact active/current-active token resolution, observable change, and at
  least 4.5:1 text contrast;
- current, selected, expanded, hover, and focus-visible coexistence;
- no click or component mutation before release and exact native click timing;
- the authorized disclosure release transition without changing its owner;
- Enter parity, link Space suppression, and native-button Space holds;
- disabled visual suppression; and
- stable held geometry, DOM/ARIA, structural-wrapper paint, and serializable
  runtime state.

The forced-colors gate verifies the active system colors, focus coexistence,
disabled suppression, restoration, geometry, DOM/ARIA, wrapper isolation, and
pre-release event absence.

## Failure conditions

The contract fails if paint lands on a wrapper, an enabled owner lacks the
navigation active pair, current/selected meaning is lost, a disabled owner
acquires enabled paint, focus paint or held geometry changes, an event/runtime
mutation occurs before release, the disclosure changes before release or
outside its expanded contract afterward, forced colors lose system paint, or
normal component CSS uses a raw/local/unknown value.
