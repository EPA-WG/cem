# cem-menu: nested menu POC and requirements

## Required interaction and accessibility contract

The [WAI Menu and Menubar Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/)
is a requirement for `cem-menu`, including its keyboard interactions, roles,
states, properties, and orientation-specific behavior. Optional APG behaviors
remain optional; document the choices when implementing them. The POC is evidence
for implementation experiments, not an exception to this requirement or a claim
of accessibility conformance.

Required outcomes include:

- One composite focus model: Tab/Shift+Tab enter or leave the widget rather than
  visiting separate controls for each logical item; leaving closes open menus.
- Enter opens a submenu and focuses its first item, or activates a command and
  closes the menu. Retain Space activation as a supported project choice.
- Arrow behavior follows the APG for the actual menubar/menu orientation,
  including movement between levels.
- Escape closes the current menu and restores its invoking item. Whole-branch
  dismissal must not focus hidden descendants or overwrite newer outside focus.
- Expose named menu containers, correctly focused items, popup/expanded state,
  orientation, and applicable checked/disabled states. Implement the APG rules
  for disabled items and separators when those features are present.
- Support Home/End when arrow wrapping is absent. Typeahead remains optional.

The prototype uses roving tabindex on one native button per item. Its visible
label also supplies its accessible name.

## Arrow-key prototype

Open the [LTR sample](./css-menu-poc.html) or the [RTL sample](./css-menu-poc-rtl.html).
Each is a separate document with a horizontal menubar, a vertical menubar and
a standalone menu button. File contains a nested Export menu. Edit opens a
vertical panel from the horizontal bar and a horizontal panel from the vertical
bar. The RTL document uses Arabic labels.

| Focused level | Next / previous sibling | Enter child | Opposite cross-axis key |
| --- | --- | --- | --- |
| Horizontal LTR | Right / Left | Down | Up |
| Horizontal RTL | Left / Right | Down | Up |
| Vertical LTR | Down / Up | Right | Left |
| Vertical RTL | Down / Up | Left | Right |

In the horizontal LTR sample, press Down on Help. Focus moves to Edit with
its panel closed. A second Down opens Edit. In the vertical RTL sample the
same sequence uses Left. A cross-axis key on a disabled launcher keeps focus
there. In a popup, the forward key on a leaf moves to the next menubar branch.
In the standalone popup it keeps focus on the leaf. The reverse key in a
nested popup returns to its launcher. At the first level below a bar it
switches to the previous root branch.

The shared adapter handles all four arrows using the focused level’s orientation
and direction. Arrow entry opens a native popover with the launcher as its
source. Enter, Space and pointer input use native button activation. An enabled
leaf writes its visible label to the output. Tab or Shift+Tab exits from the
root launcher and dismisses the open panels. Each bar remembers its last
focused button. [Keyboard adapter](../../../../../examples/web/menu-keyboard/menu.js)

## Verification

From the workspace root run `node tools/scripts/verify-cem-menu-keyboard-poc.mjs`.
The Playwright check exercises both documents. It checks physical item order,
the focused button, open panel chain, roving Tab stop and key default handling.
It also counts trusted clicks from native button activation, checks that arrows
deliver zero clicks and exercises a disabled launcher with keyboard and pointer.
The verified engines were Chromium 148.0.7778.96 and Firefox 150.0.2.

## Remaining gaps

Spoken focus and state announcements still need screen-reader verification.
The production acceptance work also covers Home/End, optional typeahead,
zoom/reflow, visible focus, contrast and forced colors.
