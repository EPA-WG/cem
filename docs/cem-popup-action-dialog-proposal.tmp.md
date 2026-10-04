# Actions, popups, menus, and dialogs: a fresh design study

**Design successor:** [CEM interaction design](cem-interaction-design.md) consolidates the adopted decisions and specifies the accepted naming and contracts. Promoted to accepted design on 2026-10-03, it is authoritative for future implementation. This file remains the research/discussion archive; its candidate syntax is superseded by the accepted design.

Temporary proposal — 2026-10-03. Derived from web standards, accessibility patterns, and external component APIs. Existing CEM component boundaries, implementations, names, and compatibility requirements are deliberately excluded from the design reasoning. All CEM markup below is hypothetical HTML API notation.

## Research findings

### Popup behavior and content semantics are independent

Open UI considered a dedicated popup element, CSS, JavaScript, and an HTML attribute. Its popover design chose an attribute that adds presentation behavior to an existing element rather than imposing a new content meaning. This is a useful architectural precedent: a menu can be popup-presented, but being a popup does not make arbitrary content a menu. [Open UI popover rationale](https://open-ui.org/components/popover.research.explainer/)

The current HTML popover model distinguishes auto, hint, and manual behavior. Native popovers are non-modal; their grouping/dismissal behavior varies by mode. Do not infer that every popup is exclusive, modal, or automatically dismissed. [HTML popover standard](https://html.spec.whatwg.org/multipage/popover.html)

**Design inference:** define presentation as an independent trait, applicable to a menu, dialog, description, or other content. Avoid a mandatory inheritance tree in which every menu and dialog is a kind of popup.

### Invocation differs from interest

Activation commits an action. Hover/focus can merely express interest. Open UI explicitly separates invoker commands from interest invokers; a preview opened through interest should not take over a surface already opened through committed activation. [Interest invoker design](https://open-ui.org/components/interest-invokers.explainer/)

**Design inference:** track why a surface is open. Pointer leave must not close a dialog deliberately opened by click, and a tooltip timer must not control a task dialog's lifetime.

### A trigger is not necessarily its anchor

Radix exposes separate Root, Trigger, Anchor, Content, and Close parts, including collision boundaries and force mounting. React Aria also supports a separate anchor, placement boundaries, and a target rectangle for pointer/text-selection positioning. These are implementation precedents, not browser requirements. [Radix Popover](https://www.radix-ui.com/primitives/docs/components/popover), [React Aria Popover](https://react-aria.adobe.com/Popover)

**Design inference:** distinguish the command source, attachment anchor, fitting boundary, and focus-return destination. “Owner” alone is too ambiguous.

### Similar appearances need different keyboard contracts

A command menu uses composite keyboard navigation. A navigation dropdown can instead be a disclosure containing ordinary links. A tooltip leaves focus on its trigger. A modal dialog contains focus; a non-modal task dialog allows interaction elsewhere. [Menu/menubar pattern](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/), [disclosure navigation](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/examples/disclosure-navigation/), [tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/), [modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)

**Design inference:** choose semantic interaction first, then presentation and placement. Do not make “open → focus first child” a universal rule.

## Independent design axes

| Axis | Choices | Question answered |
| --- | --- | --- |
| Meaning | Command menu, task dialog, description, disclosure, choices | What interaction is this? |
| Presentation | Inline, local floating, top layer | Where/how does it paint? |
| Invocation | Activate, interest, form/model request | Why did it open? |
| Modality | Modal, non-modal | Can the user interact elsewhere? |
| Attachment | Element, pointer, selection, centered context | What geometry locates it? |
| Lifetime | Light dismiss, explicit close, persistent | What ends the interaction? |
| Materialization | Eager, first-open retain, each-open dispose | When does content exist? |

These axes have constrained combinations. A tooltip is noninteractive and does not receive focus. A modal task dialog cannot silently acquire a menu's Tab-exit behavior. An inline disclosure normally leaves focus on its button. Semantic profiles should provide safe combinations rather than exposing every combination as equally valid.

## Four possible component models

### A. Compound interaction container

```html
<cem-popup kind="menu" placement="block-end start">
  <cem-action slot="trigger">Tools</cem-action>
  <cem-menu slot="content">…</cem-menu>
</cem-popup>
```

The compound owns the relationship and open session. Its content provides navigation semantics. The trigger is an ordinary reusable action inserted into the compound.

Strengths: simple local markup; relationship references can be generated; repeated row contexts are naturally local; submenu composition is readable. Weaknesses: independently placed launchers need an escape hatch; a single interaction container can become an oversized policy engine; putting a dialog inside must not create two open-state owners.

Best fit: local dropdowns, teaching bubbles, row actions. Define the container as a composition root, not the semantic parent of every dialog/menu.

### B. Content element with an inline launcher

```html
<cem-dialog mode="nonmodal" title="Edit invoice" trigger-label="Edit">
  <cem-action slot="trigger">Edit</cem-action>
  <template slot="body">…</template>
</cem-dialog>
```

The dialog retains its local attachment context, renders its launcher inline, and presents only the task body out of flow. An explicit launcher overrides a shorthand label. The task title and launcher name remain separate.

Strengths: very compact inline editing; natural item context; body declaration and materialization are colocated. Weaknesses: couples the task to one launcher; multiple launchers are awkward; “dialog contains button which contains dialog” is invalid produced structure. Launcher and native surface must be siblings.

Best fit: single local editor or help task. This can be authoring convenience over either A or C rather than its own runtime architecture.

### C. Targetable content with orthogonal presentation traits

```html
<cem-action commandfor="tools" command="toggle-popover">Tools</cem-action>
<cem-menu id="tools" popover="auto" placement="block-end start">…</cem-menu>

<cem-action commandfor="editor" command="--show">Edit</cem-action>
<cem-dialog id="editor" mode="nonmodal" title="Edit invoice">…</cem-dialog>
```

The action issues a command. The target implements its semantic interaction and optional floating presentation. There is no required popup wrapper. The shown command names illustrate native commands versus a custom runtime command.

Strengths: directly follows native invoker relationships; several actions can target one task; a menu can be persistent or floating; a dialog can be centered or attached without changing identity. Weaknesses: references and repeated-instance identity need rules; custom hosts must map commands to their actual native owners; relationship declaration can be verbose.

Best fit: a general component foundation, shared dialogs, context menus, toolbar commands. This is my preferred core based on the external evidence.

### D. Shared interaction/command descriptor

```html
<cem-interaction id="editor-session" target="editor" command="--show" />
<cem-action interaction="editor-session">Edit</cem-action>
<cem-dialog id="editor" title="Edit invoice">…</cem-dialog>
```

A separate descriptor connects command presentations, target, context, and lifecycle. Actions and dialogs are views participating in that interaction.

Strengths: a command can appear in toolbar, menu, and palette; supports permissions, shortcuts, and explicit form/model invocation; dependencies are visible. Weaknesses: adds indirection to trivial dropdowns; descriptor state can conflict with native visibility; global command identity is insufficient for repeated item sessions.

Best fit: large applications with reusable commands. Treat desired visibility as a request, and actual visibility as acknowledged target state. A checked action alone should not be the session owner.

### Comparison

| Criterion | A: compound | B: inline launcher | C: target + traits | D: descriptor |
| --- | --- | --- | --- | --- |
| Small local examples | Excellent | Excellent | Good | Verbose |
| Multiple launchers | Moderate | Weak | Excellent | Excellent |
| Native HTML correspondence | Moderate | Moderate | Strongest | Adapter needed |
| Shared commands/shortcuts | Moderate | Weak | Good | Strongest |
| Repeated item context | Local | Local | Explicit capture | Explicit session scope |
| No-script potential | With native lowering | With native lowering | Most direct | Limited |
| Main cost | Compound complexity | Launcher coupling | Reference contract | Indirection/state reconciliation |

**Proposal:** C as the foundation; A and B as optional conveniences; D only when shared command metadata is useful. This recommendation follows the web platform's separation of behavior and semantics, not an existing CEM architecture.

## Proposed responsibility distribution

```text
Action: native activation + command intent
         ↓ request, source, invocation context
Target interaction: semantic policy + actual open/closed state
         ├─ presentation: top layer / local / inline
         ├─ attachment: anchor + boundary + fitting
         ├─ lifecycle: prepare / open / close / dispose
         └─ relationship: parent session + return-focus destination
```

| Concern | Responsible concept |
| --- | --- |
| Enter/Space, click, disabled, submit/reset | Native control behind action |
| Operation name and context | Invocation |
| Accepted/rejected visibility transition | Target interaction |
| Menu arrows/typeahead/roving focus | Menu semantics |
| Task labeling, initial focus, completion/cancel | Dialog semantics |
| Hover/focus interest, description | Tooltip/preview semantics |
| Document inertness and modal containment | Native modal dialog |
| Geometry and collision handling | Attachment trait |
| DOM materialization and cleanup | Content lifecycle |
| Deepest Escape and chain dismissal | Logical session relationships |
| Checked/pressed business value | Model or genuine toggle control |

This distribution can be implemented through shared declarative runtime machinery. It does not require separate public elements for each internal concern.

## State and how actions reflect it

Keep three meanings distinct: **requested visibility**, **actual visibility**, and **business checked value**. Browser dismissal can change actual visibility without activating the launcher. Opening can fail or be canceled.

A normal popup launcher exposes expanded state; a true “Show inspector” toggle may expose pressed state. A menu checkbox represents an independent business setting using checked state. A visual check mark can reflect open state without making every launcher a checkbox.

Several launchers may reflect one target's actual visibility, but each invocation has a particular source and context. Define already-open behavior: focus existing content, reanchor, change item context, or toggle. Protect unsaved context before switching items. Commands should be idempotent where appropriate; one activation must not both toggle natively and toggle again through a runtime handler.

## Focus and relay scenarios

| Interaction | On opening | On Escape | On leaving/clicking outside |
| --- | --- | --- | --- |
| Command menu | Focus enabled item | Close nearest submenu to parent, root to launcher | Dismiss chain; preserve destination |
| Navigation disclosure | Usually retain button focus | Optional pattern-specific close | Ordinary link/Tab flow |
| Tooltip | Retain trigger focus | Hide; suppress immediate reopen | End interest after delay |
| Interactive floating task | Declared task focus | Close to session's return target | Explicit light-dismiss or persistent policy |
| Modal task | Meaningful initial focus | Request cancellation | Tab contained; outside close explicit |
| Combobox choices | May retain input focus | Close to input | Choice-specific behavior |

**Menu → dialog:** capture context and a stable workflow return target; close the menu without returning to its soon-hidden item; open/focus the dialog. On exit, return to the root menu launcher or workflow destination. No later menu cleanup may steal focus.

**Submenu → submenu:** keyboard opening moves focus; pointer hover may change visible branches without stealing keyboard focus. Use delays and pointer travel continuity. Escape closes one logical level, not every visible ancestor.

**Tooltip + dialog on one action:** hover/focus interest shows the description; committed activation ends/suppresses it and opens the task. Do not use one checked flag to represent both lifetimes.

**Form → dialog:** validate first when opening depends on valid input. Capture the submitter/context. Do not make an action both an ambiguous submit and popup toggle. Form reset and externally bound visibility have explicit reconciliation rules.

**Independent launchers → shared dialog:** use the invocation's item key/context; do not infer data from the dialog's DOM ancestry. Model opening without a physical launcher supplies an explicit fallback focus destination.

For outside activation and Tab, respect the new focus destination. Escape/cancellation may restore focus. Missing, disabled, or hidden invokers require a valid workflow fallback. Logical parent relationships must survive top-layer presentation. A popup inside a modal dialog should stay within its interactive DOM subtree.

## Native commands, CSS, and JavaScript boundaries

Native button commands include `toggle-popover`, `show-popover`, `hide-popover`, `show-modal`, `close`, and `request-close`. Custom names start with `--` and require a command handler. The current built-in table does not provide non-modal `.show()`. [HTML button commands](https://html.spec.whatwg.org/dev/form-elements.html#the-button-element)

```html
<button type="button" commandfor="task" command="show-modal">Open task</button>
<dialog id="task" aria-labelledby="task-title">
  <h2 id="task-title">Task</h2>
  <button type="button" commandfor="task" command="request-close">Cancel</button>
</dialog>

<button type="button" popovertarget="help">Help</button>
<dialog id="help" popover="auto" aria-labelledby="help-title">
  <h2 id="help-title">Help</h2>
  <button type="button" popovertarget="help" popovertargetaction="hide">Close</button>
</dialog>
```

The second task is non-modal popover presentation. Its visibility is popover state, not `dialog.open`. A persistent non-modal dialog instead uses `.show()` through a custom shared adapter. Do not assume dialog-method forms close a dialog shown only as a popover. Native open-dialog forms with `method="dialog"` close and return a result without server submission. [Dialog reference](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/dialog)

Native commands must address the produced native surface, not merely a custom wrapper containing it. Specify native-owner reference lowering, or use custom host commands. Zero authored component JavaScript can still use a shared runtime; zero executed JavaScript requires native pre-rendered markup. A lazy template cannot instantiate itself through CSS or commands.

Checkbox + CSS can show a region and checked indicator, but does not implement focus transfer/restoration, cancellation, or accessibility-state synchronization. Its activation semantics also differ from a button. Use it for actual checkbox-controlled regions. `details` supplies disclosure semantics where appropriate. [Disclosure pattern](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/)

Native interest invokers are worth supporting as an enhancement with capability detection. Do not equate an explainer's graduation with availability in every supported browser. Open UI's Openable API and Menu Elements remain proposals: useful design direction, not assumed production features. [Openable proposal](https://open-ui.org/components/openable.explainer/), [Menu Elements proposal](https://open-ui.org/components/menu.explainer/)

## Declarative placement

Use independent attributes/concepts for anchor, fitting boundary, preferred logical side/alignment, fallback order, and lost-anchor behavior:

```html
<cem-menu popover="auto"
  anchor="invoker"
  placement="inline-end start"
  boundary="editor-region"
  fallback="inline-start start, block-end start"
  overflow="shift resize"
  anchor-lost="close">…</cem-menu>
```

This is illustrative vocabulary. Pointer and text-selection rectangles should be valid anchors; a container or visible viewport is a boundary. Direction and writing mode determine logical sides. Preferred and actual placement must remain distinct so collision fitting can also orient arrows/animations.

CSS anchors and position-try fallbacks handle geometry relative to their defined containing block. They do not automatically implement an arbitrary container boundary for a top-layer element. Measured fitting may be needed for that case. Intersect permitted bounds with the visible viewport; resize oversized content, then fit; observe relevant scrolling/resizing and define mobile visual-viewport behavior. [CSS Anchor Positioning specification](https://drafts.csswg.org/css-anchor-position-1/)

Top-layer presentation does not require DOM reparenting. Prefer preserving ancestry for form ownership, styles, and accessibility references. Local floating presentation remains useful when clipping is intentional.

## Optional materialization

| Policy | First open | Close | Use |
| --- | --- | --- | --- |
| Eager | Already rendered | Retain | Small content and native no-script paths |
| Lazy retain | Instantiate once | Hide mounted body | Editors retaining DOM/form state |
| Lazy dispose | Instantiate each session | Dispose after completed close | Large repeated, stateless content |

Keep a stable lightweight target/native shell; heavy body may be an inert template. Template markup still occupies storage, but no active rendered controls exist until materialized. Remote resource loading and DOM materialization are separate policies.

Opening transaction: capture invocation → prepare content → reject stale work → open → focus according to semantics → publish actual state. Async preparation either keeps focus on the launcher until ready or opens a named loading task with a safe focus target.

Closing transaction: request/cancel decision → native hide/close → state reconciliation → focus return/handoff → exit presentation → disposal. Never detach an active native modal owner to rerender it. Dispose listeners/resources and reject late async results by session identity.

Repeated content needs unique target identities, explicit item context, and draft retention rules. Lazy disposal must not silently lose unfinished forms. Tooltip accessible descriptions should remain available independently of lazy visual bubbles.

## Proposed decision

Support all four authoring models as first-class choices for their scenarios. Unify their resolved interaction relationships rather than requiring one public component structure. CEM adds automatic wiring and convenient declarations where useful; native markup remains a complete option where it already provides the required behavior. Keep semantic profiles responsible for safe focus/dismissal combinations.

Before implementation, settle public reference syntax, native lowering, non-modal task persistence, checked-state projection, lazy form retention, and pointer/container geometry. Validate single activation, deepest Escape, menu-to-dialog handoff, no tooltip focus theft, canceled close, multiple launchers, removed invokers, repeated lazy identities, stale resources, RTL/vertical writing, and no-script native output.

The recommendation is an architectural inference from the linked evidence. Standards define native behavior; APG describes interaction patterns; external libraries demonstrate possible decompositions; emerging proposals are identified separately. Browser-version support remains an implementation validation task.


## Convenience syntax over explicit relationships

### Revised direction: four entrances, one relationship contract

All four variations serve valid scenarios. They need not be progressively discouraged in favor of one wrapper or target structure. CEM's contribution is automatic relationship wiring, sensible interaction defaults, and concise declarations. Native controls/content can participate directly; use CEM components when their semantics, paint, or convenience help.

The common resolved relationship is:

```text
invoker(s) → command → surface owner
                        ↳ semantic content
                        ↳ invocation context
                        ↳ anchor and fitting boundary
                        ↳ initial-focus and return-focus policy
                        ↳ materialization policy
```

This contract is conceptual, not a requirement to emit an extra controller element. Simple native examples need only valid native markup. Enhanced examples use shared runtime machinery for the capabilities the browser does not supply.

### 1. Generated trigger, default content

```html
<cem-popup kind="menu" trigger="Tools" placement="block-end start">
  <cem-menu>…</cem-menu>
</cem-popup>
```

Recommended meaning: `trigger="Tools"` is **literal label text for a generated native button**, not an element ID, command name, or selector. Unslotted children are content. The component allocates stable local identities and wires the generated button to the produced surface. Its generated button may share action appearance without requiring an additional `cem-action` host in the DOM.

Do not interpret lowercase `trigger="tools"` differently from `trigger="Tools"`. Localization and capitalization cannot determine reference semantics. Use a distinct reference attribute when targeting existing UI.

For a generated trigger, expose a small set of conveniences such as `trigger-aria-label`, `trigger-disabled`, and `trigger-variant` only if needed. Once configuration becomes substantial, use the explicit trigger slot. `trigger` is plain text, never HTML.

`kind="menu"` supplies menu-launcher defaults and validates the semantic content. It must not silently convert arbitrary links into command-menu items. Menu content owns navigation; the composition wires opening, naming, and dismissal. A navigation disclosure needs its own profile rather than an inferred command menu.

### 2. Explicit trigger, default content

```html
<cem-popup kind="menu" placement="block-end start">
  <cem-action slot="trigger" aria-label="Tools for this item">Tools</cem-action>
  <cem-menu>…</cem-menu>
</cem-popup>
```

The trigger slot replaces the generated button; it does not place the supplied action inside another button. Wiring reaches the supplied action's actual native interactive control.

Native controls are equally valid:

```html
<cem-popup kind="dialog" placement="block-end start">
  <button type="button" slot="trigger">Edit</button>
  <section aria-label="Edit item">…</section>
</cem-popup>
```

Here the popup creates the appropriate surface owner around ordinary content according to the declared dialog profile. A supplied native button does not need a CEM wrapper just to become an invoker.

Require one designated trigger root for this convenience form. Do not guess which nested button, link, or focusable descendant is the invoker. A CEM action must expose its control through a defined runtime contract. Multiple external launchers use explicit targeting rather than an ambiguous multi-trigger slot.

### 3. Inline dialog convenience

```html
<cem-dialog trigger="Edit" title="Edit invoice" mode="nonmodal">
  …task content…
</cem-dialog>
```

This is the same generated-trigger convention applied to a task element. It produces an inline trigger and a separate native surface, with default children as the body. A `slot="trigger"` control can replace the generated one. Without `trigger`, no launcher is generated: the dialog is available for external targeting or model requests.

A local inline dialog and an independent externally targeted dialog should have the same body, focus, and lifecycle semantics. Only the relationship declaration differs.

### 4. Existing external control

```html
<button id="tools-button" type="button">Tools</button>
<cem-popup kind="menu" trigger-for="tools-button" placement="block-end start">
  <cem-menu>…</cem-menu>
</cem-popup>
```

Proposed `trigger-for` is an ID reference to an existing native control or a component with an explicit control contract. Its name parallels native reference attributes and does not overload the label shorthand. It identifies the exact control; it is not an arbitrary CSS selector.

The popup wires that control to its generated native surface. The control keeps its DOM position and author-owned label. This form is useful when the content author wants to establish the relationship. The reverse, action-owned declaration is equally valid:

```html
<cem-action commandfor="editor" command="--show">Edit</cem-action>
<cem-dialog id="editor" title="Edit invoice" mode="nonmodal">…</cem-dialog>
```

These are alternative directions of declaration, not two independent activation listeners. If both specify the same relationship, resolve once. Conflicting target/command declarations are diagnostics, not silently combined behaviors.

### 5. Native explicit form

```html
<button type="button" commandfor="tools" command="toggle-popover">Tools</button>
<div id="tools" popover="auto">
  <cem-menu>…</cem-menu>
</div>
```

The browser owns generic presentation; the menu component supplies command-menu behavior. There is no requirement for `cem-popup`. This composition needs an explicit menu-launcher focus/navigation integration contract; native popover toggling alone does not implement menu opening focus or arrow navigation.

For ordinary navigation links, a native popover containing links may be sufficient without any CEM component. For a modal task, native `button` plus `dialog` can provide native opening. Convenience must not become a mandatory wrapper tax.

### 6. Shared interaction descriptor

```html
<cem-interaction id="edit-item" target="editor" command="--show" />
<cem-action interaction="edit-item">Edit</cem-action>
<cem-dialog id="editor" title="Edit invoice" mode="nonmodal">…</cem-dialog>
```

A descriptor supplies reusable defaults such as target, operation, permissions, and context mapping. Each physical action still supplies its own invocation source and item context. Shortcut/model invocation supplies a declared focus-return fallback because it may have no physical button.

The descriptor does not own a second copy of actual visibility. Actions observing it receive target state. A local trigger generated by `trigger` can participate in a descriptor, but competing command declarations require an explicit override rule.

### Surface ownership: wrapping versus adopting

Two valid content cases need different wiring:

- Ordinary content or a menu: the popup creates a native presentation owner and hosts the content within it.
- A dialog or native surface already owning presentation: the composition adopts that owner instead of creating another independently opening surface.

Declare a surface/control capability contract for adoption. Do not infer it solely from a tag name or from finding any descendant `dialog`. If several candidate surfaces exist, require an explicit designated surface or report ambiguity. Wrapping a dialog's body is valid; opening two separate owners for one interaction is not.

### Defaults and precedence

| Situation | Proposed rule |
| --- | --- |
| Default children | Content/body |
| Explicit `slot="trigger"` plus label shorthand | Explicit trigger wins; shorthand acts only as fallback |
| No trigger label, slot, or reference | Generate no trigger; permit external/model opening |
| Empty `trigger=""` | Diagnose unnamed generated control unless an accessible name is provided |
| `trigger-for` plus local trigger slot | Diagnose competing trigger sources |
| Content in default and named content slot | Diagnose ambiguity; do not silently concatenate |
| Explicit anchor | Overrides invoker-as-anchor default only |
| Explicit return-focus target | Overrides default restoration, without changing anchor |
| Explicit semantic content profile | Must agree with composition `kind`; conflict is diagnosed |
| Existing native command plus automatic wiring | One route executes; compatible relationship is adopted |
| Business checked value plus open projection | Keep separate unless author explicitly binds them |
| Repeated generated instance | Stable unique internal references scoped to that instance |

Canonicalize on `trigger` for launcher convenience. Avoid introducing `action` as a synonymous trigger attribute: an action can submit, navigate, or invoke a command without opening anything. If an `action` attribute exists elsewhere, reserve it for operation meaning.

For new APIs, distinguish initial-open configuration from observable actual state and command requests. A convenience declaration must not create multiple writable copies of open state.

### Presets and explicit escape hatches

A semantic preset may supply default command, initial focus, dismissal, naming relationships, and placement. Authors can override placement, anchor, boundary, focus-return destination, and materialization independently. Overrides that violate the semantic pattern, such as focusable tooltip content or disabling modal containment, should require choosing a different profile rather than defeating accessibility through a low-level switch.

Keep accessible task titles explicit; a trigger label can provide a simple default where appropriate, but “Edit” may not be a sufficient task title. Focus targets should be exact authored references or semantic policies, not arbitrary first-descendant queries.

For lazy bodies, use an explicit template plus a materialization policy. All authoring forms keep a stable target shell and share the same open/close transaction. Generated trigger IDs and references must be established before activation; no-script output requires eager native materialization or an alternative task link.

### Recommended public starting point

Start with `trigger` literal label, `slot="trigger"`, default body/content, and ordinary target-side native command relationships. Add `trigger-for` for content-owned wiring and a descriptor reference for reusable commands when those scenarios need it. Use explicit `anchor`, `boundary`, and focus-return configuration independently.

The acceptance criterion is equivalence: expanding a convenience declaration into its explicit native/control relationships must preserve activation, accessibility, focus, dismissal, placement, context, and lazy lifecycle. Convenience saves relationship markup; it does not introduce a different interaction model.

## Native browser capabilities are first-class

The proposal supports native browser capabilities directly, both inside CEM convenience compositions and as standalone HTML. CEM must preserve native activation, form behavior, semantics, and lifecycle rather than replacing them with simulated equivalents. Automatic wiring fills missing relationships; it must not duplicate a relationship the author already expressed natively.

| Browser capability | Supported use | CEM convenience responsibility |
| --- | --- | --- |
| Native `button` | Enter/Space/click activation; disabled behavior | Generate or accept the control; avoid duplicate keyboard activation |
| `commandfor` and `command` | Native popover and modal-dialog commands | Preserve explicit commands or lower convenience references to the actual native target |
| `popovertarget` and `popovertargetaction` | Declarative popover opening, hiding, and toggling | Accept these as complete native presentation wiring |
| `popover` | Native top-layer presentation and mode-specific dismissal | Add semantic content behavior only where needed |
| Native `dialog` | Modal opening, focus containment, inertness, close/cancel lifecycle | Delegate to the browser; augment declared workflow handoffs |
| Native form controls and `method="dialog"` | Validation, submit/reset, and results for native open dialogs | Preserve form ownership and distinguish submit/result from popup commands |
| `details` / `summary` | Native disclosure behavior | Permit direct use when disclosure semantics fit |
| CSS anchor positioning and position-try fallbacks | Declarative attachment and browser fitting | Use where supported and sufficient; add measured fitting only for unsupported requirements |
| `interestfor` | Native interest-driven presentation where supported | Optional enhancement with capability detection and a shared fallback |

For example, this explicitly wired native composition requires no popup component:

```html
<button type="button" popovertarget="tools-panel">Tools</button>
<div id="tools-panel" popover="auto">
  <a href="/settings">Settings</a>
</div>
```

A native modal task likewise requires no CEM wrapper:

```html
<button type="button" commandfor="native-task" command="show-modal">Edit</button>
<dialog id="native-task" aria-labelledby="native-task-title">
  <h2 id="native-task-title">Edit item</h2>
  <form method="dialog">
    <button value="cancel">Cancel</button>
    <button value="save">Done</button>
  </form>
</dialog>
```

These native routes work without JavaScript when the relevant browser features are supported and the markup is already rendered. The dialog form returns a result; it does not persist edits to a server. Adding CEM command-menu content, custom commands, lazy materialization, or workflow-specific behavior may require the shared runtime. Native popover presentation alone does not supply command-menu keyboard navigation.

Explicit native attributes take precedence over generated convenience wiring when compatible. Contradictory native and convenience declarations produce diagnostics. A native control cannot be wrapped in another generated interactive control, and one activation must execute exactly one presentation transition.

This is a proposed support contract, not a claim that these integrations have been implemented. Feature availability must be checked against supported browser versions. Native behavior follows the [HTML popover standard](https://html.spec.whatwg.org/multipage/popover.html), [button command contract](https://html.spec.whatwg.org/dev/form-elements.html#the-button-element), and [dialog lifecycle](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element).

## Implicit submenu composition through nested menus

A `cem-menu` convenience layer can interpret a directly nested `cem-menu` as a submenu, wiring its popup presentation without requiring an explicit `cem-popup` wrapper. This composition is recursive and supports arbitrary practical submenu depth.

```html
<!-- Proposed convenience syntax. -->
<cem-menu>
  <cem-action>Refresh</cem-action>

  <cem-menu trigger="Export">
    <cem-action>PDF</cem-action>

    <cem-menu trigger="Image">
      <cem-action>PNG</cem-action>
      <cem-action>JPEG</cem-action>
    </cem-menu>
  </cem-menu>
</cem-menu>
```

Each nested menu contributes two distinct parts: a launcher participating in its parent's menu navigation, and a child menu containing its own items. `trigger` supplies the generated launcher's literal label; default children supply the child menu content. The root's presentation is independent: it can be persistent, explicitly popup-hosted, or externally invoked.

An explicit trigger replaces the generated launcher:

```html
<cem-menu>
  <cem-menu placement="inline-end start">
    <cem-action slot="trigger">
      Export <span aria-hidden="true">▸</span>
    </cem-action>
    <cem-action>PDF</cem-action>
    <cem-action>CSV</cem-action>
  </cem-menu>
</cem-menu>
```

### Automatic wiring

The convenience layer establishes the parent item, child surface, stable identities, accessible naming, submenu relationship, expanded state, and attachment anchor. The trigger's actual control receives menu-item semantics and participates in the parent's roving navigation. The child menu owns navigation among its own items; parent item discovery must not collect descendants belonging to that child.

Direct menu nesting is an explicit structural signal, not a search for arbitrary descendant menus. A menu nested inside unrelated task content does not become a submenu merely because a menu exists somewhere above it. A nested menu without a local trigger or explicit parent-item relationship is ambiguous and must be diagnosed rather than producing an unnamed launcher. Native/disclosure navigation requires its own profile; this convenience describes command-menu semantics.

No extra `cem-popup` custom-element host is required. The menu declaration can produce or adopt a native child surface and configure shared popup behavior indirectly. If a child already declares compatible native popover presentation or an explicit surface relationship, adopt it rather than creating another owner or activation route.

### Recursive interaction rules

- Opening a child closes its open sibling branch, including that branch's descendants.
- Keyboard opening focuses the appropriate enabled child item. Pointer hover may open after a delay without stealing keyboard focus; pointer travel into the child preserves the branch.
- Escape closes the deepest eligible child and returns focus to its parent item. Root dismissal follows the root presentation policy.
- Submenu opening/closing arrows follow the menu keyboard profile, orientation, and direction; opening a child is distinct from activating a leaf.
- Leaf activation normally dismisses the complete transient chain. A leaf that opens a dialog uses the previously defined focus handoff rather than restoring focus to a hidden menu item.
- Outside activation and Tab dismiss the appropriate transient chain while preserving the user's destination. Persistent root menus remain present.
- Every level anchors to its own launcher and fits independently. Default logical placement follows parent orientation: beside a vertical menu item, below a horizontal menubar item. Explicit placement and fitting boundaries override these defaults.

### Explicit escape hatches and native support

Authors may supply `slot="trigger"`, placement, anchor, boundary, focus-return policy, and lazy materialization explicitly. Independently targeted submenus remain supported through a declared logical parent-item relationship. An explicit popup composition is another valid way to express the same resolved relationship.

Native popover presentation, top-layer painting, and native invoker attributes remain first-class. Compatible native wiring is preserved, and enhanced menu navigation coordinates with it. Native presentation alone does not implement roving focus, arrow navigation, sibling-branch policy, or menu-to-dialog handoff. Generated or supplied menu controls must execute only one opening transition per activation.

The equivalence requirement is the same as for other convenience forms: nested-menu syntax expands to the same parent-item → command → child-surface relationship as explicit popup composition. It changes authoring effort, not focus, dismissal, accessibility, placement, or content lifecycle semantics.

## Further convenience improvements

The next improvements should eliminate repeated relationship declarations, rather than grow a universal popup element with dozens of unrelated attributes. Examples below are candidate API notation, not final syntax or implementation claims.

### 1. Local names for reusable compositions

Generated triggers solve internal references, but several local actions targeting one local surface still require author-written IDs. Repeated rows make globally unique IDs particularly cumbersome.

Provide explicitly scoped interaction names:

```html
<cem-interactions>
  <cem-action target="editor" command="--show">Edit</cem-action>
  <cem-action target="editor" command="--show">Review</cem-action>
  <cem-dialog name="editor" title="Edit item">…</cem-dialog>
</cem-interactions>
```

Proposed `target` resolves an interaction name in the nearest declared scope. Native `commandfor` continues to mean a document/tree ID reference; never reinterpret it as a local name. The compiler/runtime generates native owner IDs and accessibility references behind the local relationship. Cross-scope relationships remain explicitly addressed using native IDs or another deliberately qualified reference mechanism.

The scope need not require a new wrapper tag: an opt-in attribute on an existing container could establish it. Ordinary semantic nesting should not silently create arbitrary name scopes. Duplicate names in one scope, missing targets, and ambiguous references are diagnostics. Dynamically produced repeated scopes retain stable identities throughout each instance's lifetime.

Benefit: reusable row/editor compositions without ID bookkeeping. Cost: another reference mechanism; introduce it only if generated internal wiring and normal native IDs are insufficient.

### 2. Infer a semantic profile from a designated content owner

```html
<cem-popup trigger="Tools">
  <cem-menu>…</cem-menu>
</cem-popup>
```

`kind="menu"` is redundant when a single designated semantic content owner declares a menu capability. Likewise, adopting an explicitly modal dialog can establish the dialog policy without repeating `kind="dialog"` on its composition.

Inference should use a declared semantic capability, not arbitrary descendant searches or visual appearance. One unambiguous owner permits inference; arbitrary content requires an explicit profile; multiple owners require an explicit designation. An authored profile that conflicts with the content is an error. Root menus do not become transient merely because they are menus: semantic kind and presentation remain independent.

Benefit: fewer repeated declarations while preserving the meaning of native or custom content.

### 3. Cascade submenu context, but not task state

Nested menus should inherit the parent's interaction environment: direction, collision boundary, placement gap, hover timing, and logical dismissal chain. Each child derives an orientation-appropriate default attachment and can override its geometry independently.

Do not inherit visibility, business checked values, item context replacement, or arbitrary dialog modality. A persistent root with transient submenus is normal. Every child owns its own actual state and materialization lifecycle.

This makes deeply nested menus concise without forcing authors to repeat `boundary`, placement policy, and hover timing at every level. It also makes RTL and vertical-writing behavior coherent across the chain.

### 4. Capture invocation context for shared task surfaces

```html
<cem-action target="editor" command="--show"
            context-key="invoice-42">Edit</cem-action>
```

A shared editor needs both a command and the item the command concerns. The convenience layer can capture an invocation envelope: source control, operation, declared context key, modality of input, logical origin chain, and return-focus fallback. Applications bind richer context through the established declarative data mechanism; do not introduce a second object-expression language just for popups.

When a dialog opens from a transient menu, derive its stable return destination from the originating chain. That should eliminate routine author-written focus-handoff handlers. An explicit workflow return target still wins. A pointer context menu similarly preserves the keyboard owner separately from its pointer anchor.

Context is captured at invocation, not inferred later from changing DOM ancestry. Showing a second item in a dirty editor needs a declared replace/confirm/reject policy; convenient context capture must not silently discard the first item.

### 5. A close affordance that uses the appropriate native route

```html
<cem-dialog trigger="Edit" title="Edit item" close-label="Close">
  …
</cem-dialog>
```

An optional generated close button can supply an accessible name and target the current surface automatically. A `slot="close"` control replaces it. Its default intent is a close request, preserving dirty-task cancellation policy rather than directly forcing closure.

For a native open dialog, lower that request to `request-close` where supported. For popover presentation, use the appropriate hide command, with any declared task guard handled by the shared runtime. Do not assume every surface has the same cancelable native close lifecycle. Direct force-close remains an explicit operation for authors who need it.

A close shortcut should also be available to a native button through an explicit relationship; generated chrome is optional. The convenience must not add an extra close button when the author already supplied one.

### 6. Native forms as dialog results, with automatic completion wiring

Instead of a new custom submit language, allow the author to use native forms and submit buttons in a task surface. Their native validation, submitter name/value, and `method="dialog"` result semantics remain intact where applicable.

For tasks that save asynchronously, provide one shared declarative completion policy: the accepted submit starts a service operation, pending state disables duplicate completion, success requests closure, and failure preserves the open task and entered values. This is an application-service integration convenience, not a rule that every submit closes a popup.

A menu action launching such a task still uses the same invocation context and focus-return relationship. Do not treat “submitted,” “saved,” and “closed” as interchangeable events. Simple native result dialogs require no asynchronous machinery.

### 7. Distinguish submenu expansion from command selection

A nested menu trigger normally expands a branch; a leaf normally executes a command. Default leaf activation closes the transient menu chain, but some command menus contain repeatable operations or checked settings that should remain available.

Provide a concise activation policy, such as proposed `dismiss="chain|self|none"`, with a menu-level default and a per-item override. Disabled controls never trigger dismissal. A submenu launcher expands its child rather than inheriting leaf dismissal.

A checked menu setting needs a declared selection model with checkbox semantics; mutually exclusive choices need a group/value model with radio semantics. Opening a submenu must not mutate those checked values. Reuse generic declarative selection/form capabilities where possible rather than making menu visibility own selection.

For ordinary actions, default dismissal follows accepted activation. Commands that require confirmation or await success can declare another timing policy. Avoid a generic “close on any bubbled click” implementation: clicks within a task form or unrelated projected content are not automatically menu selection.

### 8. Lazy content inferred from an explicit template

```html
<cem-dialog trigger="Edit" title="Edit item">
  <template slot="body">…expensive editor body…</template>
</cem-dialog>
```

An explicitly designated body template can opt into first-open materialization with retained content, avoiding a second redundant “lazy” switch. Eager child content remains eager. Disposal is explicitly requested because it affects form/draft state.

Use the same convention in a nested menu or popup body. Template presence is an author signal, not a reason to activate arbitrary descendant templates. A template and eager body supplied simultaneously require a clear designation or diagnostic.

Provide named loading and error content slots for asynchronous preparation, if supported. Their use does not require the author to write activation races or timers. A first-open retained body is distinct from resource caching and from a new invocation changing item context.

### 9. Placement presets describe relationships

Defaults can express the relationship rather than force every author to spell coordinates:

- Root menu launcher: below the control, aligned at logical start.
- Submenu: alongside the parent item, accounting for parent orientation.
- Tooltip: near the described control, with no focus transfer.
- Task dialog: centered in the declared context unless attachment is requested.
- Context menu: at captured pointer position, or owner geometry for keyboard invocation.

These are proposed design defaults, not universal browser defaults. Explicit logical placement, anchor, and boundary remain available. Pointer coordinates must not become the focus-return destination, and a keyboard-opened context menu must not reuse stale pointer coordinates.

### 10. Separate presentation state from meaningful public events

Automatic wiring should expose a small, consistent declarative event/state vocabulary so application consumers do not need to know whether the native owner is a dialog or a popover.

Useful distinctions are: open requested, actual opened, close requested/canceled, actual closed with a reason, and task completed with a result. Native events remain available; normalized notifications represent observed outcomes and must not initiate duplicate transitions.

Avoid creating a writable generic `checked` property to make event consumption easy. A visual open indicator observes actual state; business values and service completion remain independently represented. Native toggle notifications may coalesce, so committed state synchronization must not depend on receiving one notification per intermediate request.

### Adoption status and limits

| Convenience | Status | Reason |
| --- | --- | --- |
| Semantic-owner inference | Adopted | Removes redundant `kind` with a narrow rule |
| Submenu context inheritance | Adopted | Makes recursive composition practical |
| Invocation context and automatic return target | Adopted | Solves shared tasks and menu-to-dialog handoff |
| Close affordance / explicit close slot | Adopted | Removes routine dismissal wiring |
| Template-driven retained materialization | Adopted | Concise and predictable lifecycle signal |
| Leaf dismissal policy | Adopted | Separates branch expansion from command completion |
| Local interaction names | Adopted | Valuable in repeated compositions; adds reference complexity |
| Async form completion | Next | Valuable but needs a precise service/validation contract |
| Shared command descriptors | Scenario-driven | Best when commands have several presentations |
| Large families of trigger styling attributes | Avoid initially | An explicit trigger slot scales better |

Across these additions, the design rule is: infer from an explicit structural or semantic signal, preserve native declarations, offer a precise override, and diagnose ambiguity. Convenience should reduce repeated author work without hiding a consequential choice such as modality, unsaved-state disposal, or the item being edited.


## Adopted convenience contract — 2026-10-03

The following conveniences are accepted design decisions following user approval. This adoption supersedes their earlier candidate/priority wording. It does not assert implementation, turn this temporary document into a repository-wide normative specification, or authorize changing existing component behavior as part of this documentation update.

1. **Semantic-owner inference:** infer the interaction profile from one explicitly identifiable semantic owner. Explicit compatible profiles remain supported; ambiguous or conflicting owners produce diagnostics.
2. **Submenu context inheritance:** recursively nested menus inherit direction, collision boundaries, and interaction timing. Each submenu retains its own actual visibility and lifecycle. Placement derives from parent orientation and can be overridden.
3. **Invocation context and focus return:** capture the originating control, item context, and logical interaction chain. Derive a stable return target for menu-to-dialog handoffs; explicit workflow destinations override the derived target. Protect unsaved content during context replacement.
4. **Optional close affordance:** support `close-label` for a generated named control and `slot="close"` for an explicit control. Route dismissal through the appropriate native operation or shared close-request adapter, preserving cancellation policy and avoiding duplicate controls.
5. **Template-driven retained materialization:** an explicitly designated body template requests first-open materialization with retention. Eager content remains eager; disposal is explicit because it affects draft/form state.
6. **Leaf dismissal policy:** support a menu default and item override for chain/self/no dismissal. Branch launchers expand submenus; checked business values remain independent of visibility. Dismissal follows accepted activation rather than arbitrary bubbled clicks.
7. **Scoped interaction names:** support explicitly established local scopes and local target names for repeated compositions. Generate stable native IDs internally. Native `commandfor` remains an ordinary ID reference and is never reinterpreted as a scoped name.

These decisions complement the accepted generated-trigger, explicit-trigger-slot, default-content, external-trigger-reference, and implicit recursive submenu compositions. All four architecture variations remain supported for their respective scenarios.

The invariants are accepted with these conveniences: native controls and browser capabilities are first-class; explicit compatible wiring is preserved; each activation executes one transition; command intent and actual visibility are distinct; geometry anchors and focus-return destinations are independent; ambiguity is diagnosed; expanding shorthand into explicit relationships preserves behavior.

Exact public naming for scoped references, invocation data binding, and policy attributes still needs API specification. Illustrative names in this document are not a promise of shipped syntax. Async service/form completion and expanded command-descriptor features remain optional later work, separate from the seven adopted conveniences.
