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
  including movement between levels. Radio arrow behavior alone is insufficient.
- Escape closes the current menu and restores its invoking item. Whole-branch
  dismissal must not focus hidden descendants or overwrite newer outside focus.
- Expose named menu containers, correctly focused items, popup/expanded state,
  orientation, and applicable checked/disabled states. Implement the APG rules
  for disabled items and separators when those features are present.
- Support Home/End when arrow wrapping is absent. Typeahead remains optional.

These requirements do not prescribe radios, roving tabindex, or another specific
implementation mechanism. The chosen mechanism must produce the required
behavior and accessible semantics with conforming HTML.

## Current experiment

[`css-menu-poc.html`](./css-menu-poc.html) explores native radio-group navigation
combined with button commands and nested auto-popovers:

- Every label represents one logical item and contains a radio plus a button.
  Group `Q` is the top level; group `W` is inside the first popup. The deepest
  popup currently contains explanatory text and a Close button, not another
  radio group.
- Radios provide native same-group arrow movement and checked-state changes.
  Arrow-generated radio clicks are ignored by the activation bridge so merely
  moving between radios does not open a submenu.
- Label-text clicks select the radio, focus its button, and invoke it.
  Enter/Space on a radio also focus and invoke that item's button. Events from
  buttons retain native button activation.
- Buttons use `command`/`commandfor` to show or hide popovers. Leaf A/C buttons
  are placeholders without application commands.
- Each popup records its opening source. Its closed-toggle handler forwards
  focus through the source button's parent label to the associated radio.
  Guards skip disconnected sources, sources within closed ancestor popovers,
  and restoration that would replace focus on an outside control.
- The closed-toggle handler retains `preventDefault()` as an explicit POC
  choice. This does not make the toggle notification cancellable.

The current source includes `role="menubar"` on the outer `nav` and
`role="menuitem"` on labels. These attributes are experimental: they do not
supply keyboard behavior or complete the required accessible focus model.
The layout displays the top-level items vertically.

## Remaining gaps

| Area | Current POC | Required follow-up |
| --- | --- | --- |
| Tab navigation | Radios and buttons remain separate focus targets. | Implement the composite focus contract. |
| Cross-level arrows | Native radio arrows stay within a group. | Implement orientation-aware navigation between levels. |
| Command completion | Leaf buttons have no command or menu-closing action. | Verify real activation and dismissal. |
| Roles and focus | Roles are on labels; focus moves to descendant radios/buttons. Submenu containers lack `role="menu"`. | Align the exposed item with the actual focus model and verify the accessibility tree. |
| Names and state | Menubar/popups lack explicit accessible names; vertical menubar orientation and parent-item popup/expanded state are absent. | Complete and verify names, orientation, and state projection. |
| Label structure | A label contains both a radio and a button. | Resolve the HTML restriction against extra labelable descendants while preserving the experiment's activation and restoration intent. |
| List structure | HTML `menu` directly contains labels, dialogs, paragraphs, and buttons. | Use conforming `li` structure or a suitable alternative container. HTML `menu` alone does not establish an ARIA menu. |
| Container meaning | `nav` represents command items; `dialog popover` exposes dialog semantics. | Decide whether navigation/dialog semantics describe the final content. The dialog/popover combination itself is valid and nonmodal. |

## Verification

Prior Chromium 148 and Firefox 150 checks verified arrow selection without
activation, label-text and Enter/Space opening, one-level Escape restoration,
whole-branch closing, and preservation of outside focus. Those checks covered
the behavior bridge; they do not establish conformance of subsequently added
ARIA roles or spoken screen-reader output.

Before acceptance, verify the complete required pattern in Chrome and Firefox,
including Tab/Shift+Tab, orientation-specific arrows, reopening, all dismissal
paths, and focus after multiple levels close. Inspect names, roles, states, and
relationships in the accessibility tree and test with screen readers. Also
verify visible focus, zoom/reflow, target sizes, contrast, and forced colors.
Record optional behavior choices and remaining browser/assistive-technology
limits. Passing a browser interaction test alone is not an ADA compliance claim.
