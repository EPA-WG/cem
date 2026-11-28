# Declarative keyboard navigation in CEM

Design adopted 2026-11-27.

Render **one native control per logical menu item**. Use a button for a command or submenu launcher and an anchor with an `href` for navigation. That control carries the item’s accessible name, role and focus. The component declares the menu hierarchy and orientation. The `composite-menu` adapter owns **all four arrows** at the focused level. The browser owns ordinary button activation by Enter, Space or pointer.

## What the prototype settles

The earlier radio/button experiment at `f02e46e2` exposed two focus destinations for one item. Selecting a sibling also changed a radio’s checked state. A menu needs independent focus and command state. For example, arrowing to **Sort by Date** lets the user inspect that choice. Activating it commits the sort and updates the checked item. [Earlier experiment](https://github.com/EPA-WG/cem/blob/f02e46e2/packages/cem-components/src/components/cem-menu/css-menu-poc.html) · [Menu and menubar pattern](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/)

The revised prototype uses labelled native buttons throughout. Both documents use the same adapter. Each demonstrates horizontal and vertical menubars, nested submenus and a standalone menu button. [LTR sample](../packages/cem-components/src/components/cem-menu/css-menu-poc.html) · [RTL sample](../packages/cem-components/src/components/cem-menu/css-menu-poc-rtl.html)

The production component and prototype share a browser acceptance matrix. The component also exercises inherited scope in forms and fieldsets. [LTR component sample](../packages/cem-components/playgrounds/cem-menu-keynav.html#ltr) · [RTL component sample](../packages/cem-components/playgrounds/cem-menu-keynav.html#rtl) · [Component contract](../packages/cem-elements/docs/composite-menu-contract.md)

## Authoring interface

Set **`keynav`** on the root or inherit it from an enclosing scope. A persistent root has menubar semantics in this profile, including when laid out vertically. A popup opened by a menu button has menu semantics. A submenu can inherit the enabled state through its registered launcher. Retain `direction="row|column"` for each level and inherited `dir` for reading direction. This lets a vertical menubar and a vertical popup share a layout while answering Right differently.

```html
<cem-menu keynav direction="row" aria-label="Document commands">
  <cem-menu-item label="File">
    <cem-menu slot="submenu" direction="column">
      <cem-menu-item label="Export">
        <cem-menu slot="submenu">
          <cem-menu-item label="PDF"></cem-menu-item>
        </cem-menu>
      </cem-menu-item>
      <cem-menu-item label="Save As…"></cem-menu-item>
    </cem-menu>
  </cem-menu-item>
  <cem-menu-item label="Edit">
    <cem-menu slot="submenu">
      <cem-menu-item label="Copy"></cem-menu-item>
    </cem-menu>
  </cem-menu-item>
</cem-menu>
```

The example shows navigation structure. A leaf’s command is bound separately at the normal activation endpoint. A second component can opt into the same pattern through its XHTML `cem-element` declaration with `capability="composite-menu"`. Its template supplies a direct `part="composite"` container, native buttons or links (or item hosts with a direct `part="control"`), and submenu relationships. The existing `parent-item` relationship covers a panel projected elsewhere. The shared adapter installs the delegated listener for either declaration. [Menu declaration](../packages/cem-components/src/components/cem-menu/cem-menu.xhtml) · [Item declaration](../packages/cem-components/src/components/cem-menu-item/cem-menu-item.xhtml)

### Scope, ownership and native controls

`keynav` is the only navigation setting authors need. Bare HTML `keynav`, `keynav=""` and `keynav="on"` enable navigation. `keynav="off"` opts a subtree out. Omission inherits the nearest explicit setting. With no setting in the application, controls use their native keyboard behavior. A declaration can set the default on its rendered instance host; template nodes and instance markup can supply a closer setting. XHTML spells a bare attribute as `keynav=""`.

Only the enabled state inherits. At each keydown the focused control’s nearest owning menu, fieldset or form supplies the rules. An explicitly enabled descendant can resume navigation within an opted-out subtree. A registered projected submenu inherits through its launcher. Scope lookup stops at the application boundary.

This rendered example uses one marker. The form, its fieldset and the sort menu all inherit it. Moving the marker to the fieldset limits navigation to that group. Setting `keynav="off"` on the fieldset while leaving the host enabled opts out the complete group, including the menu.

```html
<file-list-tools keynav>
  <form>
    <fieldset>
      <legend>File filters</legend>
      <label>Filter files <input type="text" name="filter"></label>
      <label>Minimum size <input type="range" name="size"></label>
      <button type="submit">Apply</button>
      <button type="reset">Reset</button>
      <cem-menu direction="row" aria-label="Sort files">
        <button type="button" role="menuitemradio" aria-checked="true">Name</button>
        <button type="button" role="menuitemradio" aria-checked="false">Date</button>
      </cem-menu>
    </fieldset>
  </form>
</file-list-tools>
```

| Focus or owner | Behavior in this example |
| --- | --- |
| Text input, textarea or editable content | Leave arrows and modified arrows to the browser for caret movement and text selection. Space types and Enter follows the control’s editing or submission behavior. |
| Slider, select, number input or native radio group | The focused control keeps its value and selection keys, including at the end of its range. No per-input opt-out is needed. |
| Form or fieldset action button | Plain Down or inline-forward moves to the next eligible action button in source order. Up or inline-backward moves to the previous one. In LTR, Right on **Apply** focuses **Reset**. At either end focus stays in the group. Enter and Space activate the focused button normally. |
| Sort menu button | The menu owns all four arrows. In horizontal LTR, Right on **Name** focuses **Date** and leaves **Name** checked. Native button activation requests a sort once. On confirmation the application moves the checked state to **Date**. |
| Tab or Shift+Tab | Forms and fieldsets retain their ordinary Tab sequence. The menu contributes one stop. Tab from an open menu dismisses its chain and lets the browser continue traversal. |

Form navigation is a CEM convenience for enabled, visible action buttons belonging directly to that form or fieldset. Skip controls owned by nested groups and menus. Preserve native behavior for other controls and modified shortcuts. An ordinary link keeps Enter navigation and Space scrolling. A registered menu link uses the menu’s Space activation rule. [Keyboard interface conventions](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/) · [HTML implicit submission](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#implicit-submission)

**Delegate once per participating host.** A bubbling `keydown` listener resolves the event origin and the innermost registered owner. An outer owner never retries a key left to native handling by an inner owner. Resolve the effective marker and focused control for each event so a moved control or changed attribute takes effect on the next key. Respect composition, previously canceled events and modified shortcuts. Cancel the browser default only for a binding that replaces it. The following sketch names the three decisions.

```js
host.addEventListener("keydown", event => {
  if (event.defaultPrevented || event.isComposing) return;
  const owner = navigationOwner(event.composedPath());
  if (owner?.host !== host || !effectiveKeynav(event.target, owner)) return;
  owner.handleKeydown(event);
});
```

Track focus from `focusin` and `focusout`, including pointer and programmatic moves. Remember the menu’s last live root item for Tab re-entry. Read native values on `input` or `change` and project menu choice state from the confirmed command result. Release listeners when the host disconnects. [UI Events focus events](https://www.w3.org/TR/uievents/#events-focusevent)

Acceptance includes the sequence in the table, a second independent instance and both marker placements. Toggle the fieldset off while **Apply** is focused and verify that its next Right is uncanceled. Explicitly enable the nested menu and verify that its arrows work while Apply and Reset remain opted out. Remove both overrides and verify that all three owners inherit again.

### How to describe a key

The menu capability owns the key bindings in `cem-elements`. Each row describes **where focus is**, **which key matches**, **who performs its default action**, and **where focus should end up**. Resolve `current item`, `child`, `launcher` and `adjacent root item` against the admitted menu relationships at event time. A few representative rows in design notation show the proposed granularity.

| Focus and condition | Key | Executor | Observable result |
| --- | --- | --- | --- |
| Root item in a horizontal LTR bar | Right | Focus adapter | Next root item receives focus. A closed branch stays closed. |
| Vertical menu item with a child, LTR | Right | Adapter requests open from the existing popup binding | First item in the confirmed open child receives focus. |
| Vertical menu leaf under a menubar, LTR | Right | Adapter switches branch | Next root item receives focus. Its child opens if present. |
| Enabled native submenu button | Enter or Space | Browser activates the button once | Popup binding confirms opening and applies entry focus once. |
| Item inside an open chain | Shift+Tab | Adapter dismisses the chain, browser traverses backward | Focus reaches the preceding outside stop. |

For example, the library could store rows as CEM-ML and compile them once when registering the capability. This XML projection illustrates a candidate resource shape for four transitions. The `key` values below are logical keys resolved from the level’s orientation and reading direction.

```xml
<keyboard-profile id="composite-menu">
  <binding in="popup" key="toward-child" item="launcher"
           action="open-child" focus="first-child" default="replace"/>
  <binding in="popup" key="toward-child" item="leaf" root="menubar"
           action="next-root-branch" focus="root-item" default="replace"/>
  <binding in="menubar" key="toward-child" item="leaf"
           action="next-sibling" focus="sibling" default="replace"/>
  <binding in="chain" key="tab-backward"
           action="dismiss-chain" focus="sequential-backward" default="preserve"/>
</keyboard-profile>
```

The vocabulary is finite. A binding can name a registered parent, sibling, child or launcher. It cannot supply an arbitrary CSS selector or call a method on a DOM node. Normalize the rows into one decision per reachable state during registration. Reject an ambiguous pair with its two source locations. An unmatched key passes through. A stay-put row retains focus and consumes the arrow to prevent page scrolling. This makes a missing behavior testable without publishing raw keyboard events to an application data island.

The component’s capability selects its bindings. The library owns the individual transitions, including exit keys and native activation. The resolver can be a pure function over key, modifiers, item kind, orientation, root kind and branch state. Its output describes the action and focus destination. The DOM adapter checks that the destination is still live before applying it.

## Menu behavior at the edges

The following are CEM profile decisions based on the WAI-ARIA patterns. The arrows below assume a horizontal LTR menubar with vertical submenus. [Menubar keyboard guidance](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/#keyboardinteraction) · [Menu button guidance](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/#keyboardinteraction)

| Input | Focus and popup behavior |
| --- | --- |
| Tab / Shift+Tab | Give the persistent bar one sequential stop. Enter at its first item on first visit and remember the last live item on re-entry. Popup items receive programmatic focus. Tab from any depth closes the chain in either direction. Suppress close-time restoration, use the logical root stop or menu button as the traversal origin where necessary, and let the browser find the next outside stop. This also applies when the panel is rendered elsewhere. |
| Left / Right on the bar | Move among root siblings with wrap. If the user is browsing an open branch, close that branch and open the adjacent item’s child if present, keeping focus on the new root item. |
| Down / Up on a root launcher | Open its child at the first / last item. On a persistent root leaf the CEM sibling fallback below applies. An external menu button supports the same optional entry keys. |
| Down / Up in a vertical popup | Move to the next / previous focusable sibling with wrap. Browsing a command or choice has no application side effect. |
| Right / Left in a vertical popup | Right on a launcher enters its child. Right on a leaf under File switches to the next menubar branch. Left from File’s first-level popup switches to the previous menubar branch. Left from Export’s deeper popup closes only Export and focuses its launcher. In a menu-button tree there is no adjacent bar branch. Right on a leaf stays put, and Escape provides the return from the first level. |
| Enter / Space | On an enabled command button, use its one native click. A submenu launcher opens its child. A completed leaf activation dismisses the chain. Enter on a link keeps native navigation. In menu mode, plain Space on an enabled link needs one guarded activation and suppression of scrolling. A task such as Save As takes focus through the existing menu-to-task relay. |
| Escape | Give the top native auto-popover the close request. On confirmed closure restore its live launcher once. Escape in a persistent bar with no open surface passes through to its invoking context. Outside pointer dismissal preserves focus already placed outside. |
| Home / End and printable input | Home/End focus the first/last eligible item at this level even with wrap enabled. Keep localized typeahead as a focus-only convenience. Clear the buffer on locale or owner change. Preserve IME composition, AltGraph and modified application shortcuts. |

### Arrow resolution by orientation and reading direction

Resolve each key at the menu level that owns focus. Read its declared row or column and its effective HTML directionality, including an inherited or locally overridden `dir`. Use the same direction for layout, submenu indicators and keyboard interpretation. An English item label in an RTL menu follows that menu’s arrow mapping. Recompute on a direction change while retaining the focused item. Popup positioning can flip at a viewport edge without changing the keys. [HTML directionality](https://www.w3.org/International/questions/qa-html-dir)

| Focused level | Next sibling | Previous sibling | Toward child | Opposite cross-axis key |
| --- | --- | --- | --- | --- |
| Horizontal, LTR | Right | Left | Down | Up |
| Horizontal, RTL | Left | Right | Down | Up |
| Vertical, LTR | Down | Up | Right | Left |
| Vertical, RTL | Down | Up | Left | Right |

Apply the following rules in order:

1. **Along the menu.** Move focus among siblings in the level’s declared order, including when the focused item owns a submenu. Keep the adjacent-branch behavior described above if the user is already browsing an open menubar branch.
2. **Across the menu on a launcher.** The toward-child key opens the submenu and focuses its first item. On a persistent root launcher the opposite key can open at the last item. A disabled launcher stays focused and cannot open its child or fall through to leaf navigation.
3. **Across the menu on a persistent root leaf.** Adopt the suggested CEM convenience: toward-child moves to the next root sibling. The opposite key moves to the previous sibling. Both are focus moves, with the same wrap policy as along-axis traversal. Landing on a launcher this way leaves its child closed. The APG specifies Down on a horizontal menubar launcher and leaves Down on a menubar leaf unspecified. The extra sibling move is a CEM profile choice. [APG keyboard interaction](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/#keyboardinteraction)
4. **Across the menu inside a popup.** A leaf under a menubar uses toward-child to reach the next **root branch**. A leaf in a standalone menu-button tree stays in place. The opposite key at a deeper level closes that level and returns to its launcher. At the first level below a menubar it browses the previous root branch. These routes take priority over the root-leaf convenience.

For example, in a horizontal **File ▾ · Help · Edit ▾** bar, Down on File enters File. Down on Help focuses Edit. A second Down enters Edit. In a vertical RTL bar with those same items, Left performs those three actions and Down always moves to the next sibling. Inside File’s vertical RTL popup, Left on a leaf browses the next root branch. None of these arrows executes a leaf command.

Acceptance should exercise the four rows with focus on both a leaf and a launcher, repeat with a disabled launcher, and distinguish a root leaf from a popup leaf under a menubar or a standalone menu button. Switch `dir` on the open level and confirm that the next horizontal key follows the new mapping while focus and submenu ownership remain intact.

**Focusable and invokable need separate tests.** In the menu adapter a visible disabled menu item can receive arrow focus and be announced as unavailable. Use a focusable control with `aria-disabled="true"` and guard pointer, keyboard and command activation. Skip hidden or inert items and separators. A panel containing only disabled items can therefore focus its first disabled item. A genuinely empty panel needs a named, programmatically focusable container and a working exit. Checked state belongs to the choice command. Navigation alone cannot commit a radio or checkbox choice. [Menu roles and states](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/#wai-ariaroles,states,andproperties)

## Focus and activation on the same control

The current root item has `tabindex="0"`. Other root items and all popup items have `tabindex="-1"`. Moving focus to a root item transfers the menubar’s single Tab stop. Arrow entry asks the popup to open with its button as the source, then focuses the requested first or last item. Normal button activation uses the native popover command. Escape closes the native popover. The adapter repairs focus if nested dismissal leaves it on the document body or inside the closing panel.

In Chromium and Firefox, keyboard and pointer activation delivered one trusted click per enabled button gesture. Arrow navigation delivered zero clicks, including when it opened a submenu. Forward and reverse Tab exited open branches, and re-entry returned to the remembered menubar item. [Prototype browser check](../tools/scripts/verify-cem-menu-keyboard-poc.mjs)

## Adapter lifecycle and verification

- Delegate `keydown` once on each rendered host during bubbling. Resolve ownership, the effective `keynav` setting and the focused control’s owning group before dispatch. Honor an already handled key, editable descendants, composition, and modified shortcuts. Repeats may move focus. Guard repeated Enter/Space against a second invocation. Only an owned transition that replaces a browser default should cancel that default. Native Tab traversal and native Escape dismissal still need theirs.
- Keep the active item, open branch and return destination in the menu session. Revalidate ownership after render, native toggle and async task entry. A removed launcher or stale close notification cannot restore into a hidden branch, another menu or another app. Release listeners and restore authored roles and tab stops when the mode changes. Hydration can restore presentation without replaying a key or click. [Menu-to-task relay](../packages/cem-elements/src/lib/menu-task-relay.ts)
- Extend the existing trusted-key stories with a trace of **focused element, open panels, default action and command count** after each step. Start with File → Export → PDF, then Left twice. Expect Export first, then the previous root branch. Repeat from a menu button, a projected panel, a vertical bar and an RTL bar. Test forward and reverse Tab with focusable controls immediately before and after the widget.
- Exercise native button activation, held Space, a disabled command, duplicate menus, and Save As opening a nonmodal form. Navigation and refusal produce zero commands. One accepted activation produces one command. Confirm that a late menu close preserves focus already in the form. [Existing browser stories](../packages/cem-components/src/components/cem-menu/cem-menu.stories.ts)
- For accessibility verification, inspect the accessibility tree and listen to a screen reader announce the focused item, its name, submenu expansion and disabled state. The project adopts the APG interaction pattern. WCAG evaluation also covers keyboard access, escape from the composite, focus order and visible focus. The prototype verification covers keyboard and pointer input. [WCAG keyboard](https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html) · [No keyboard trap](https://www.w3.org/WAI/WCAG22/Understanding/no-keyboard-trap.html) · [Focus order](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html) · [Focus visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html)
