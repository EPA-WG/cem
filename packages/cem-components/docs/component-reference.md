# CEM Component Reference

**Status:** Phase 4 MVP reference for `@epa-wg/cem-components`.

This document describes all 53 canonical XHTML/CEM-ML components. It pairs with
[`conventions.md`](./conventions.md), [`accessibility.md`](./accessibility.md),
[`component-mvp.md`](../../../docs/component-mvp.md), and the
[passive/payload completion contract](./passive-payload-contract.md).
Components load from individual XHTML resources or the generated bundle through
`cem-elements`. The legacy JavaScript installer and package entry are retired.

**Capability caveat:** source templates and their owning declarations carry
intrinsic behavior metadata. Fragment imports inherit it, so ordinary consumers
need only `tag` and `src`. Preserve the metadata when copying a template;
explicit importer values override it, and `capability=""` suppresses it. A
component can still render while losing required runtime behavior when
the capability is removed or replaced. See the
[capability names and ownership rules](./conventions.md#41-intrinsic-capability-metadata)
and the [form demo](../../cem-elements/demo/form.html).

## Authoring Contract

- The normative source/layout/testing contract is the
  [declarative UI principle](../../../docs/declarative-ui-principle.md): one
  `src/components/<cem-tag>/<cem-tag>.xhtml` CEM-ML declaration whose
  `<template id="<cem-tag>" type="text/cem-ml">` is reusable as
  `<declaration-url>#<cem-tag>`, containing its
  declaration-owned `<style>` node with CEM UI `--cem-*` tokens, installed
  once per declaration, and one
  colocated CSF Next `<cem-tag>.stories.ts` module containing all component unit
  assertions in async `play` functions from `storybook/test`. Its `render`
  functions return HTML strings and it loads its own raw XHTML declaration
  through the shared Storybook loader. Standalone component CSS and production
  JavaScript/TypeScript elsewhere in the package is forbidden; the story is the only
  development-only TypeScript exception and cannot implement behavior.
- The accepted CSS target compiles declaration styles with native `@scope`.
  Private rules use the tag root; public shared groups use declaration-owned
  `scope="name"`; projected roots retain `slot="name"`; and component-owned
  internals expose `part="name"`. Per-instance CSS requires an explicit inert
  direct payload template. The complete target is normative in the
  [CEM light-DOM CSS scope contract](../../../docs/cem-ml-uid-and-scoped-css-design.md)
  but MUST NOT be assumed implemented until its active migration gate passes.
- If CEM-ML lacks a concise required capability, hard-stop the component and
  implement the reusable capability in `cem-elements` before resuming. A
  behavior module, installer callback, story-local behavior, or app-local
  workaround is forbidden.
- Components render in light DOM through the `<cem-element>` substrate. Authors provide semantic fallback content first;
  the upgraded output keeps the same visible meaning.
- Component state is expressed through host attributes and ARIA attributes, not private CSS classes. Use
  `disabled`, `aria-selected`, `aria-expanded`, `aria-busy`, `aria-invalid`, and `data-state` according to the state
  matrix in `docs/component-mvp.md`.
- Component styling must use CEM token families from `@epa-wg/cem-theme`. The token families listed below are the
  allowed styling surface for the MVP; no component-specific color or spacing literals should be introduced.
- Examples live under [`../examples`](../examples). New and migrated component
  tests live only inside their colocated CSF Next Storybook module. Use
  [`cem-select`](../src/components/cem-select/) as the proven pattern.

## Production Gate

The current migration baseline and the proven XHTML/CSF Next contract remain
verifiable when this command passes on the branch being promoted:

```bash
yarn nx run @epa-wg/cem-components:verify
```

The aggregate gate includes:

| Gate | Command | Coverage |
| --- | --- | --- |
| Declarative architecture | `yarn nx run @epa-wg/cem-components:verify-declarative` | Rejects new JavaScript/TypeScript outside the required story contract, new registry members, non-XHTML component implementations, missing embedded CEM-token `<style>` nodes, forbidden standalone/global component CSS, or a colocated story that does not use the raw-XHTML loader plus CSF Next HTML-string `render` and async `play` contract. Enforces the closed zero inventory for legacy components and authored code files other than stories. |
| Figma component inventory | `yarn nx run @epa-wg/cem-components:verify-figma-inventory` | Accounts for all 49 public primitives, validates component-set/component/payload/structural classification, public property traceability, exact executable states and token families, live documentation, planned/reviewed evidence rules, and the five-mode review fixture. It depends on the native Figma token gate. |
| Angular Material parity inventory | `yarn nx run @epa-wg/cem-components:verify-material-parity` | Pins the exact stable official catalog and requires every entry to remain visible until it is audited as a component mapping, cross-cutting behavior, partial mapping, or explicit gap. Barebone `cem-elements` compatibility fixtures cannot satisfy product UI parity evidence. |
| State matrix | `yarn nx run @epa-wg/cem-components:verify-state-matrix` | Resolves every category state to exact browser tests/assertions and supports verifier-checked component-specific evidence so a newly promoted owner does not replace existing evidence. |
| Public component catalog | `yarn nx run @epa-wg/cem-components:build:catalog` | Emits deterministic public beta JSON from the canonical MVP table, current migration inventory, this reference, conventions/accessibility guidance, and the verified state-matrix report. Related examples and Storybook remain source links/local commands rather than copied fixtures or site-owned builds. |
| Token-only style contract | `yarn nx run @epa-wg/cem-components:verify-style-contract` | Depends on current theme tokens and the verified public theme stylesheet export; checks exact action, content-interaction, navigation, tabs layout, paginator, feedback, slider, tooltip, and timepicker bindings, component selector scope, and rejects inline styles, unknown/non-CEM variables, and raw component color or spacing literals. |
| Divider forced colors | `yarn nx run @epa-wg/cem-components:verify-divider-forced-colors` | Proves `CanvasText` separator color, D5 thickness, D1 inset, and the complete D1/D2 line-plus-margins track in forced colors. |
| Expansion forced colors | `yarn nx run @epa-wg/cem-components:verify-expansion-forced-colors` | Proves header/panel system colors, contextual hover/active/disabled paint, D5 focus, D2 target size, stable geometry, and event/state isolation. |
| Sort-header forced colors | `yarn nx run @epa-wg/cem-components:verify-sort-header-forced-colors` | Proves character-distinct none/ascending/descending states, system hover/active/disabled colors, D5 focus coexistence, D2/D2c geometry, and transient-input state isolation. |
| Paginator forced colors | `yarn nx run @epa-wg/cem-components:verify-paginator-forced-colors` | Proves native select/action ownership, system hover/active/disabled colors, D5 focus coexistence, D2/D2c geometry, surviving character icons, and transient-input state isolation. |
| Slider forced colors | `yarn nx run @epa-wg/cem-components:verify-slider-forced-colors` | Proves native range-input ownership, system remaining/active/disabled track and thumb semantics, surviving ticks, D2/D2c geometry, D5 focus, and transient-input state isolation. |
| Tooltip forced colors | `yarn nx run @epa-wg/cem-components:verify-tooltip-forced-colors` | Proves exact trigger ownership, persistent description, source/installed rendering and `Canvas`/`CanvasText` surface paint through the shared native-surface verification gate. |
| Timepicker forced colors | `yarn nx run @epa-wg/cem-components:verify-timepicker-forced-colors` | Proves exact native input ownership, `Canvas`/`CanvasText` popup paint, system hover/active/selected/disabled coexistence, D5 focus, top-layer CSS anchor placement, stable geometry, and event/state isolation. |
| Timepicker/slider source/package integration | `yarn nx run @epa-wg/cem-components:verify-time-range` | Property controls, native value/form/reset ownership, five-theme galleries and bundle loading in source and isolated archives. |
| Datepicker source/package integration | `yarn nx run @epa-wg/cem-components:verify-datepicker` | Verifies property controls, native forms and reset, calendar focus/event order, five-theme gallery, forced colors and bundle loading from source and isolated package archives. |
| Datepicker forced colors | `yarn nx run @epa-wg/cem-components:verify-datepicker-forced-colors` | Proves exact native input ownership, modal/top-layer containment, `Canvas` dialog paint, `SelectedItem` selected paint, `Mark` current paint, `CanvasText` focus coexistence, disabled suppression, logical anchor placement, stable geometry, and event/state isolation. |
| Tabs forced colors | `yarn nx run @epa-wg/cem-components:verify-tabs-forced-colors` | Proves selected/default/disabled system paint, a surviving `Highlight` indicator, D2 target size, D5 focus, stable geometry and ARIA, and absence of animation or numeric stacking. |
| Stepper forced colors | `yarn nx run @epa-wg/cem-components:verify-stepper-forced-colors` | Proves exact native header ownership, current/hover/active/focus/completed/invalid/disabled coexistence, completed/remaining connector paint, D2/D5 geometry, and transient event/state isolation. |
| Tree forced colors | `yarn nx run @epa-wg/cem-components:verify-tree-forced-colors` | Proves exact native treeitem ownership, default/hover/active/selected/focus/disabled/loading system paint, visible state shapes, D2/D5 geometry, wrapper isolation, and transient event/state silence. |
| Input indicator forced colors | `yarn nx run @epa-wg/cem-components:verify-input-indicator-forced-colors` | Launches Chromium with forced colors active; proves component shadows collapse, field/binary hover uses `Highlight`, and keyboard focus traverses the original seven input owners with full `CanvasText` outlines. |
| Navigation/content source/packed/bundle | `yarn nx run @epa-wg/cem-components:verify-navigation` | Verifies six canonical property pages, five-theme galleries, native interaction and bundle loading in source and isolated archives. |
| Autocomplete source/packed/bundle | `yarn nx run @epa-wg/cem-components:verify-autocomplete` | Proves property controls, authoritative refresh, focus, forms, gallery binding and canonical bundle loading. |
| Autocomplete forced colors | `yarn nx run @epa-wg/cem-components:verify-autocomplete-forced-colors` | Proves popup draw order, input/option pointer ownership, system hover/active/selected/disabled colors, keyboard focus coexistence, stable geometry, and event/state isolation. |
| Navigation hover/focus/active/disabled forced colors | `yarn nx run @epa-wg/cem-components:verify-navigation-hover-forced-colors` | Launches Chromium with forced colors active; proves system hover/current/active/disabled colors, ARIA-disabled current/selected precedence, full keyboard traversal, focus coexistence, native-disabled skipping, restoration, and wrapper/state isolation. |
| Content hover/focus forced colors | `yarn nx run @epa-wg/cem-components:verify-content-hover-forced-colors` | Launches Chromium with forced colors active; proves exact content-owner keyboard order and `CanvasText` rings alongside checkable-chip system fills, native-listbox hover boundary color, selected/checked coexistence, disabled skipping, restoration, and passive wrapper isolation. |
| Native surfaces and forced colors | `yarn nx run @epa-wg/cem-components:verify-surfaces` | Verifies dialog, dialog-shell and tooltip source/installed property pages, five-theme galleries, release bundle, native focus and system-color paint. |
| Legacy stylesheet publication | `yarn nx run @epa-wg/cem-components:verify-package` | Preserves the frozen legacy component stylesheet byte-for-byte in `dist`, verifies the side-effect-free `./styles.css` compatibility export, and checks the dry-run npm file inventory. Migrated declaration CSS does not enter this artifact. |
| Phase 3 substrate harness | `yarn nx run @epa-wg/cem-components:verify-native-substrate` | Proves real CEM-ML action/field registration, render settlement and re-rendering, slice-driven events, native form data/reset/validity, light-DOM accessibility, a reviewed structural/computed-style baseline, and Chromium screenshot capture. |
| Legacy browser and unit behavior | `yarn nx run @epa-wg/cem-components:test` | Keeps the frozen Node/Chromium migration baseline executable. New/migrated component unit ownership is the colocated `.stories.ts` CSF Next module, not this suite. |

Current legacy evidence locations follow. They keep the pre-migration package
verifiable but are not valid locations or implementation patterns for new work:

| Purpose | Path |
| --- | --- |
| Declarative architecture and source layout | `../../../docs/declarative-ui-principle.md`, `../src/components/` |
| Frozen migration baseline | `../declarative-migration.json` |
| Angular Material parity inventory | `../tests/angular-material-parity.json` |
| Autocomplete behavior and state coverage | `../src/components/cem-autocomplete/cem-autocomplete.stories.ts` |
| Expansion behavior and state coverage | `../src/components/cem-expansion/cem-expansion.stories.ts` |
| Sort-header behavior and state coverage | `../src/components/cem-sort-header/cem-sort-header.stories.ts` |
| Paginator behavior and state coverage | `../src/components/cem-paginator/cem-paginator.stories.ts` |
| Slider behavior and state coverage | `../src/components/cem-slider/cem-slider.stories.ts` |
| Tooltip behavior and state coverage | `../src/components/cem-tooltip/cem-tooltip.stories.ts` |
| Timepicker behavior and state coverage | `../src/components/cem-timepicker/cem-timepicker.stories.ts` |
| Datepicker behavior and state coverage | `../src/components/cem-datepicker/cem-datepicker.stories.ts` |
| Tabs behavior and state coverage | `../src/components/cem-tabs/cem-tabs.stories.ts` |
| Stepper behavior and state coverage | `../src/components/cem-stepper/cem-stepper.stories.ts` |
| Tree behavior and state coverage | `../src/components/cem-tree/cem-tree.stories.ts` |
| State, ARIA, focus, and event payload coverage | `../src/components/<cem-tag>/<cem-tag>.stories.ts` |
| Sheet lifecycle and focus coverage | `../src/components/cem-sheet/cem-sheet.stories.ts` |
| Workflow coverage | Colocated `cem-card`, `cem-avatar`, `cem-app-bar` and `cem-sheet` stories |
| Declarative workflow fixtures | `../tests/workflows/` |
| Declarative autocomplete fixture | `../tests/autocomplete/contract.html` |
| Declarative expansion fixture | `../tests/expansion/contract.html` |
| Declarative sort-header fixture | `../tests/sort-header/contract.html` |
| Declarative paginator fixture | `../tests/paginator/contract.html` |
| Declarative slider fixture | `../tests/slider/contract.html` |
| Declarative tooltip fixture | `../playgrounds/cem-tooltip-gallery.html` |
| Declarative timepicker fixture | `../tests/timepicker/contract.html` |
| Declarative datepicker fixture | `../tests/datepicker/contract.html` |
| Declarative tabs fixture | `../tests/tabs/contract.html` |
| Declarative stepper fixture | `../tests/stepper/contract.html` |
| Declarative tree fixture | `../tests/tree/contract.html` |
| Style and manifest verifier scripts | `../../../tools/scripts/verify-cem-components-*.mjs` |
| Package stylesheet source | `../src/styles.css` |
| Package publication and forced-colors scripts | `../../../tools/cem-components/copy-styles.mjs`, `../../../tools/cem-components/verify-package.mjs`, `../../../tools/cem-components/verify-input-indicator-forced-colors.mjs`, `../../../tools/cem-components/verify-autocomplete-forced-colors.mjs`, `../../../tools/cem-components/verify-navigation-hover-forced-colors.mjs`, `../../../tools/cem-components/verify-content-hover-forced-colors.mjs`, `../../../tools/cem-components/verify-expansion-forced-colors.mjs`, `../../../tools/cem-components/verify-sort-header-forced-colors.mjs`, `../../../tools/cem-components/verify-paginator-forced-colors.mjs`, `../../../tools/cem-components/verify-slider-forced-colors.mjs`, `../../../tools/cem-components/verify-timepicker-forced-colors.mjs`, `../../../tools/cem-components/verify-datepicker-forced-colors.mjs`, `../../../tools/cem-components/verify-tabs-forced-colors.mjs`, `../../../tools/cem-components/verify-tree-forced-colors.mjs` |

The Phase 3 visual baseline is the inline Vitest snapshot produced by the
headless Chromium project used on Linux CI. It records normalized visible HTML,
geometry, and selected computed styles while removing runtime-generated
`data-cem-*` identity/source-map attributes and inert data-island templates.
Baseline updates therefore appear as reviewed source diffs. The accompanying
PNG capture is a browser-paint smoke assertion and is not stored as a second,
platform-sensitive golden file.

Handoff condition: Phase 4 component expansion can build on this primitive package after the aggregate verify gate is
green and the promoted branch has no uncommitted gate changes. The handoff covers the MVP primitive declaration set,
common static/form workflows, state and ARIA behavior, light-DOM rendering, event payload capture, and token-only style
constraints.

Known deferrals remain outside the Phase 3.2 trigger:

- Edge/SSR processing fixtures for serialized data-island snapshots are Phase 3.5.
- `@epa-wg/custom-element` monorepo adoption is Phase 3.6.
- Full application behaviors such as dialog focus trapping, routed navigation, async data loading, and resource
  primitives are follow-up runtime/application work.
- Post-MVP controls including split actions, date ranges, date/time-zone integration, adapters and alternate calendar views, side-nav variants, breadcrumbs, and richer
  Additional menu/dropdown variants remain future expansion work.

## Actions

Host visibility follows the [shared native visibility convention](./conventions.md#native-visibility).


| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-action` | Native button; defaults to command, supports explicit submit/reset. | Default slot or `label` fallback; `variant` selects primary, explicit, contextual, alternate or destructive intent. Optional `size="small|medium|large|x-large|xx-large"` overrides inherited minimum control height. Labels wrap; noninteractive image/title content can grow beyond the minimum. Native form attributes forward to the button. Presence-only `selected` and `selectable` expose container-owned selection with persistent zebra and native-button `aria-pressed`. | action, control, palette, bend, typography | Button text or `aria-label` must name the action. |
| `cem-icon-button` | Native icon command or navigation link. | `href` selects link mode; embedded `cem-icon` receives `image`, `size`, `direction`, `align`, `label`, and `aria-label`. Action-theme states include selected/selectable, pending and disabled. `variant="navigation"` adopts navigation state colors; explicit `aria-current` forwards to links and `aria-expanded`/`aria-controls` to buttons. | action, navigation, control, palette, stroke, bend | Use `aria-label` for icon-only controls; `label` also provides visible fallback text. |
| `cem-dropdown` | Triggered panel for arbitrary content and menus. | Native trigger in `slot="base"` and default panel slot. | action, palette, stroke, control, bend, typography | Native button fallback; expanded/controls relationships, Escape restoration and outside/Tab dismissal. |
| `cem-menu` | Persistent wrapping command/link collection; optional composite submenus. | Default slot accepts links, buttons and menu items. | action, palette, stroke, gap, inset, typography | Native Tab by default; opt-in menu mode owns roving focus and nested popup semantics. |
| `cem-menu-item` | Menu command row. | Default slot is command text. | action, palette, gap, inset, typography | Renders `role="menuitem"` and must be contained by a menu/list context in full menus. |

States: `default`, `hover`, `focus-visible`, `active`, `disabled`, `pending`, `selected`.
The canonical `cem-action` declaration embeds its scoped styles for all five
intents. `cem-icon-button` uses the same five variants in both button and link
state styles, including persistent selection and pending feedback. Its additional
`navigation` variant uses navigation colors for links and disclosure buttons.
The action variants use paired CEM action tokens for default, enabled native-button hover
and held native-button active states. Hover and active change only background/text color; they add
no ARIA or active runtime state and exclude disabled buttons through `:enabled`.
See the [action hover contract](./action-hover-contract.md) and
[action active contract](./action-active-contract.md).

Action consumes the theme's `--cem-action-<intent>-<state>-background` and
`--cem-action-<intent>-<state>-text` properties directly. Override these on a
container or an individual host; removing an override restores the inherited
theme value. The former private `--_cem-action-*` aliases are removed without
compatibility aliases. No generic `--cem-action-<state>-*` family is introduced.

For `cem-action` and `cem-icon-button`, `pending="true"` reflects `aria-busy="true"` and shows the
canonical 45-degree pending gradient with a seamless two-second loop. Colors
come from the current intent's pending/active background pair and pending text.
Destructive uses the theme's pending-stripe endpoint for a lighter stripe in
light schemes and a darker stripe in dark schemes; its active button state
remains unchanged.
Pending paint takes precedence over hover/active and disabled paint. A disabled
pending button keeps the gradient while native disabled semantics block activation
and focus. Clearing pending while disabled restores disabled colors. Hover/focus
preserve the continuous animation and zebra focus remains independent. Clearing
pending restores interaction paint. Reduced motion freezes
the gradient; forced colors use system colors and a static pending boundary.
Pending preserves dimensions and activation; use `disabled` to block activation.

## Inputs

`cem-suggestions` attaches a native suggestions list to one projected `cem-field`
or `cem-text-field`. Enable local composition through the embedding runtime's
`localSuggestions: true` policy. Use inert `editor` and `options` payload slots;
optional `option`/`group-label` templates use `type="text/cem-ml"` and receive
native `suggestion`/`group` parameters in their original namespace context.
The field owns value, name, constraints, visible label, help and form submission.
The attachment owns listbox/row IDs, group names, preview and status feedback.

The omitted profile, or explicit `profile="listbox"`, supports `filter="contains|prefix|none"`,
`filter-by="label|value|label value"` (omit with `none`), `require-selection`,
`selection-message`, `options-state="ready|pending|failed"`, `options-error`,
listbox `label`, and the five `*-message` feedback attributes shown in its
[gallery](../playgrounds/cem-suggestions-gallery.html). Custom labels cannot hide
the stored-value hint. Explicit native `options`/`editor-for` integration requires
host-admitted sources and relationships; it conflicts with the matching local
slot. Chromium input-protocol plays cover mouse/touch/pen taps, cancellation,
wheel scrolling and composition settlement on both fields. Physical touch-pan,
device IME, screen-reader interoperability and the broader composition matrix
remain in the [acceptance evidence and release record](./suggestions-acceptance.md).
The gallery maps [all seven audited source intents](../../../docs/cem-autocomplete-source-audit.md#adopted-gallery-coverage),
including native text fallback and explicit value/label edge cases. Its numeric
examples demonstrate ordinary editing only. The property playground includes
an explicit `profile="native-datalist"` example. This profile accepts text,
search, tel, url, single-email and number inputs, retains their native roles and
constraints, and consumes complete ungrouped local sources. It rejects custom
filtering, selection proof, label templates and email `multiple`. The browser
owns filtering and the picker; no custom popup or synthetic commit is installed.
Actual numeric picker selection and device/AT acceptance remain pending in the
acceptance record; typed numeric values are not picker-selection evidence.


`cem-field` and `cem-text-field` use canonical XHTML declarations and the shared
form-control capability. See the [field controls contract](./field-controls-contract.md)
for attribute presence, host form ownership, reset and scoped indicators.
Both single-line fields forward native range/step, length/pattern, editing hints
and `list` bindings; see that contract for the full shared inventory. Live public
value writes update submission/validity synchronously and preserve reset defaults.

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-field` | Generic labeled single-line field. | `name`, `value`, `type`, `placeholder`, `indicator`, `required-marker`, `busy`; named label/help/required-marker slots. | palette, stroke, bend, gap, typography | Label slot or `label` attribute must name the input. |
| `cem-text-field` | Single-line text entry. | `name`, `value`, `placeholder`, `indicator`, `required-marker`, `busy`; label/help/required-marker slots. | palette, stroke, bend, gap, typography | Label slot or `label` attribute must name the input. Help text must not become the accessible name. |
| `cem-textarea` | Multi-line text entry with native editing. | `name`, `form`, `value`, `rows`, `cols`, `minlength`, `maxlength`, `autocomplete`, `placeholder`, `indicator`, `required-marker`, `busy`, native boolean states and label/help/required-marker slots. | palette, stroke, bend, gap, typography | A labeled native textarea owns editing and validation; the form-associated host owns one submitted value and reset. Borderless across states. See the [textarea contract](./textarea-contract.md). |
| `cem-autocomplete` | Form-associated editable combobox with declarative suggestions. | Canonical direct `cem-option`/`cem-option-group`; all-native `option`/`optgroup` migration adapter; free-form or `require-selection`; `value`, `placeholder`, `autocomplete`, `indicator`, `busy`, `auto-active-first`, label/help slots. See the [autocomplete contract](./autocomplete-contract.md). | palette, select, stroke, bend, layering, control, typography | The native input owns focus and text entry; the transient listbox exposes options/groups while `aria-activedescendant` retains focus on the input. |
| `cem-suggestions` | Native datalist or a transient suggestions list for one editor. | `profile`, `editor-for`, `label`, `slot="editor"`, `slot="options"`, `slot="option"` and `slot="group-label"`; exact rich option payload and native editor association. | palette, select, stroke, bend, gap, typography | The editor owns focus, value and form submission; option labels preserve accessible distinctions. See the suggestions contract above. |
| `cem-timepicker` | Canonical time-of-day input with a transient choice popup. | Exactly one direct native text input in `slot="input"`; optional native `button[slot="toggle"]`; `min`, `max`, `interval`, `required`, `invalid`, `disabled`; generated choices or direct `cem-option`. See the [timepicker contract](./timepicker-contract.md). | action, palette, select, stroke, bend, layering, coupling, control, typography | The authored input retains native labeling, value, validation, events, and form submission while owning combobox focus and active-descendant navigation. |
| `cem-datepicker` | Canonical single-date input with a modal calendar. | Exactly one direct native text input in `slot="input"`; optional native `button[slot="toggle"]`; `min`, `max`, `required`, `invalid`, `disabled`, and `lang`. Values are empty or `YYYY-MM-DD`. See the [datepicker contract](./datepicker-contract.md). | action, palette, content, select, stroke, bend, layering, coupling, control, typography | The authored input retains native labeling, value, validation, events, reset, and form submission. A native modal dialog owns a labeled grid with one roving day, localized headers, and distinct selected/current/disabled semantics. |
| `cem-theme-switch` | Persistent contextual sun/moon theme action with dropdown menu. | `storage-key` (default `cem-theme`), legacy `name` key alias, initial `mode` (default `light`) and `contrast`, optional controlled `value` (full theme name), `theme-command` host-service binding; default slot retains themed content. | action, zebra, palette, layering, coupling | “Change theme” names the trigger and tooltip. Contextual choices expose the current selected action. Saved values survive reload; invalid values fall back to light. |
| `cem-select` | Form-associated custom single/multiple choice with HTML-rendered options. | Canonical direct `cem-option`/`cem-option-group`; all-native `option`/`optgroup` migration adapter; `multiple`, `size`, `indicator`, `busy`, label/help slots. See the [custom select contract](./select-contract.md). | palette, select, stroke, bend, layering, control, typography | Label slot or `label` attribute names the combobox/listbox; focus remains on the composite owner with `aria-activedescendant`. |
| `cem-option` | Canonical rich option payload consumed by `cem-select`, `cem-autocomplete`, and `cem-timepicker`. | Required `value`; optional `label`, `selected`, and `disabled`; static HTML descendants. Timepicker values use canonical `HH:mm`. | typography | Does not create a nested tab stop or interaction owner. |
| `cem-option-group` | Canonical labeled grouping payload consumed by `cem-select` and `cem-autocomplete`. | Required `label`; optional `disabled`; direct `cem-option` children. | typography | The consuming composite projects `role="group"` and its accessible label. |
| `cem-checkbox` | Native binary/mixed form choice. | Default slot replaces `label`; `name`, `value`, `form`, presence flags, native `indeterminate`, ARIA naming/help, `required-marker`, `indicator` and `busy`. See [checkable contract](./checkable-contract.md). | palette, stroke, control, bend, typography | Wrapping label must expose the visible text as the accessible name. |
| `cem-radio` | Mutually exclusive form choice. | Default slot replaces `label`; `name` and native form owner group radios; checked/default reset, presence flags, ARIA naming/help, `required-marker`, `indicator` and `busy`. | palette, stroke, control, typography | Radio group context should provide the set label. |
| `cem-switch` | Immediate boolean setting. | Default slot replaces `label`; native checkbox with `role="switch"`, `name`, `value`, `form`, checked/default reset, presence flags, ARIA naming/help, `required-marker`, `indicator` and `busy`. | palette, stroke, action, control, bend | Visible label must name the switch. |
| `cem-slider` | Horizontal single-value or range input. | One direct native range input marked `single`, or exact `start`/`end` inputs; parent `min`, `max`, `step`, `disabled`, `discrete`, `show-tick-marks`. See the [slider contract](./slider-contract.md). | slider, coupling, stroke, bend, gap, typography | Every input retains native slider semantics and requires an accessible name; range thumbs require distinct names. Generated visuals are hidden. |


Documentation page chrome uses the separate [cem-gallery-page declaration](gallery-page-contract.md).
Its `cem-playground-theme` preference controls the page; production theme-switch
examples and fixed variation themes keep their own scopes and storage keys.

States: `default`, `hover`, `focus-visible`, `disabled`, `loading`, `expanded`, `invalid`, `required`, `readonly`,
`checked`, `indeterminate`.

`cem-autocomplete` is covered across every applicable input state. It supports
free text by default and an explicit `require-selection` mode, contributes its
string value through `ElementInternals`, keeps focus on the native input during
listbox navigation, and accepts live declarative option replacement without
replacing that input or mutating the committed value. Filtering, ranking,
fetching, and debouncing remain application/CEM-QL concerns.

`cem-timepicker` is covered across its applicable input states while the direct
authored text input remains the form and event owner. Values are empty or
canonical `HH:mm`; typed syntax/range errors use native custom validity;
generated intervals or direct options supply listbox choices. Option commits
dispatch one input/change pair on that exact input. Date objects, seconds, time
zones, locale adapters, and datepicker integration remain out of scope.

`cem-datepicker` is covered across its applicable input states while the direct
authored text input remains the value, validation, event, reset, and form owner.
Calendar navigation changes only a draft; Apply emits one native input/change
pair when the canonical value changes, while Cancel, Escape, and backdrop
dismissal remain silent. The Angular Material parity row stays partial because
date ranges, adapters/filters, alternate year views, date/time-zone integration,
Date/Temporal values, and locale-specific text parsing remain out of scope.

`cem-slider` retains native range-input form/event ownership for single and
range values. It adds no tuple form value or custom event: each thumb serializes
under its authored name and emits its own native `input`/`change` sequence.
Parent bounds are normalized, range values cannot cross, and transient pointer
or focus paint does not mutate component state or geometry.

Presence-only host `busy` projects exact `data-state="loading"` and
`aria-busy="true"` markers to the same interactive control. It does not infer or
perform asynchronous work, disable editing, create runtime state, or replace
the control; label, value, focus, and dimensions remain stable. See the
[input loading contract](./input-loading-contract.md).

Extra input indicators are state-specific. Resting fields, closed selects and
unselected choices have no decorative shadow stack; native control boundaries
remain. Checkbox, radio and switch labels stay clickable but never own the
indicator. Their native inputs own focus, checked/mixed, invalid and pending
feedback, so the label text is not enclosed in a second box.

When needed, the stack composes an invalid/pending anchor, an independent
keyboard-focus stripe and an independent checked/mixed or open-popup stripe.
A normal anchor has zero width. Invalid controls activate the boundary width;
pending controls activate pending width; invalidity wins when both apply.
Disabled controls do not acquire an invalid/pending anchor, but retain their
native checked state and selection indicator. Pointer hover alone adds no extra
shadow; it can recolor an already active invalid/pending indicator.

Field-like controls default to `indicator="underline"`; checkbox, radio and
switch default to `indicator="outline"`. These options describe active state
feedback, not an always-visible decoration. Missing/unsupported values use the
family default. The inherited `--cem-input-indicator-appearance` adapter still
wins over the host attribute. Theme tokens remain generated and overridable.
In forced colors, shadows are removed and native state plus system-color
outlines preserve feedback. Focus and blur must preserve DOM, values and layout.

### Action selection and disclosure

`cem-action[selected]` represents a persistent choice and draws the selected
zebra independently of focus, hover, held `:active`, disabled and loading. Its
container owns selection; clicking never toggles the attribute automatically.
Use `selectable` on both selected and unselected members of a choice group.
`selected` implies selectable. Both attributes are presence-only: remove them
for false; `selected="false"` still means selected.

The native button exposes `aria-pressed="true"` when selected, `"false"` when
selectable but unselected, and no `aria-pressed` for an ordinary command. Do not
put `aria-selected` on an ordinary button; menus, tabs and list options retain
their own role-specific selection contracts. Selection is an outline channel,
not a required alternate fill. Combined selected/focus/pending cues must remain
visible without changing dimensions. Forced colors uses an inset selection
boundary and a separate focus/pending outline.

`expanded="true|false"` is retained as explicit forwarding to `aria-expanded`
for an action reporting an associated disclosure's state. It does not open a
panel or indicate selection, and is omitted from the standalone action
playground. The disclosure owner supplies the behavior and panel association;
`cem-action` forwards native invoker and relationship attributes to its exact button; see the command contract below.

### Action command invokers

The [accepted interaction design](../../../docs/cem-interaction-design.md) governs new action wiring. The declaration consumes the shared `action-command` capability in `cem-elements`; no component-local behavior owns visibility.

Native `commandfor`, `command`, `popovertarget`, `popovertargetaction`, and `interestfor` forward unchanged to the produced button. `aria-controls`, `aria-haspopup`, `aria-describedby`, and `aria-expanded` also forward; explicit `aria-expanded` takes precedence over the existing `expanded` shorthand. Native commands target native owners, never an implicitly selected descendant of a custom host.

`command-target="@name|#id"` resolves an explicit CEM relationship. `@name` resolves only in the nearest `interaction-scope` against `interaction-name`; missing/duplicate names are diagnostics, with no ancestor fallback. Built-in commands resolve a declared native owner (`part="surface"`) or a directly targeted native surface. Custom commands target the provider endpoint. Shared descriptor metadata may supply target/command/context defaults through `interaction`; surface and descriptor component lifecycles are delivered separately.

`context-key` is captured with the actual native source before browser command dispatch, available to shared surface capabilities through `getCemActionInvocation`. This transient metadata does not replace typed CEM context. The target owns popup/dialog state; button-selected business state is unchanged.

Conflicting native/CEM invocation routes are rejected rather than toggled twice. Missing targets rebind as the DOM changes. Submit/reset behavior remains native even when invalid surface wiring is diagnosed. `cem-interaction-error` bubbles from the invoker when no valid target exists. Existing controls and target IDs survive ordinary rerender.

This first delivery implements action-side wiring and native targets. Shared popup/dialog targets own session commands and their target-side lifecycle.

## Layout

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-surface` | Named section surface for grouped content and workflow regions. | Default slot projects authored content; `tone` selects visual treatment. Presence-only `busy` marks a pending whole-workflow update; presence-only `empty` marks its settled empty outcome. Busy takes rendered-state precedence during ordered transitions. | palette, stroke, bend, gap, inset | `label` names the section. Busy adds exact `data-state="loading"` and `aria-busy="true"`; empty adds exact `data-state="empty"`. Neither state adds live-region, inert, or focus semantics. |
| `cem-stack` | Generic single-axis layout container. | Default slot projects children; `gap="sm|md|lg"` selects related/default-stack/section spacing. The component does not infer or inherit loading or empty state. | gap, responsive | Adds no landmark or interaction semantics. |
| `cem-grid` | Generic responsive grid layout container. | Default slot projects children; `columns="auto|1|2|3|4"` and `gap="sm|md|lg"` select placement; auto uses responsive tracks at the layout minimum. The component does not infer or inherit loading or empty state. | gap, responsive | Adds no landmark or interaction semantics. |
| `cem-divider` | Visible sibling-separation track. | `orientation="horizontal|vertical"`; `spacing="related|group|block|section"`; presence-only `inset` and `decorative`. It owns no content, state, focus, or events. | separator, stroke, gap, inset, coupling | Semantic form exposes a non-focusable separator with exact orientation. Decorative form removes the role/orientation and sets `aria-hidden="true"`. |

These six layout/content owners use canonical XHTML, declaration-scoped token CSS
and stable public parts. See the [layout/content contract](layout-content-contract.md)
for defaults, typography, responsive tracks and retained content state.

States: `default`, `loading`, `empty`. In v1, both explicit layout states are
owned by `cem-surface`: `busy` projects pending loading and takes precedence over
the settled `empty` state. Stacks and grids remain formatting-only; dividers are
non-interactive separation tracks. `cem-divider` composes D0 color, D1 relationship
spacing/inset, the D2 guard floor, and D5 line geometry as specified by the
[divider contract](./divider-contract.md). See the
[layout loading](./layout-loading-contract.md) and
[layout empty](./layout-empty-contract.md) contracts.

## Navigation

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-app-bar` | Page or application banner. | `slot="title"` for title; default slot for global actions. | palette, stroke, gap, inset, typography | `label` names the banner when multiple landmarks exist. |
| `cem-nav` | Labeled navigation region with an optional region-wide disclosure. | Default slot accepts navigation-variant icon links and native links/actions. Shared padding and wrapping gaps use inset/gap tokens. Presence-only `collapsible` adds a navigation icon button with a native disclosure control; presence-only `expanded` sets its initial open state. Without `collapsible`, the existing passive landmark output is unchanged. | palette, navigation, gap, inset, typography | `label` names both the nav landmark and disclosure button. The button mirrors the current boolean state through `aria-expanded`; hidden content leaves the tab order and links retain native semantics. |
| `cem-tabs` | Labeled local-view switcher. | Direct `cem-tab` children require unique `value` and non-empty `label`; host accepts `selected-index` and `orientation`. See the [tabs contract](./tabs-contract.md). | palette, navigation, stroke, bend, gap, coupling, control, typography | A labeled tablist owns roving native tab buttons with manual activation and stable reciprocal persistent tabpanels; exactly one enabled tab and panel are selected. |
| `cem-tab` | Inert local-view payload consumed by `cem-tabs`. | `value`, `label`, optional `disabled`, and default panel content. | typography | Adds no independent role, focus, event, form, route, loading, or persistence owner. |
| `cem-stepper` | Linear or nonlinear workflow navigation. | Direct `cem-step` children require unique `value` and non-empty `label`; host accepts `selected-index`, `orientation`, `linear`, and `disabled`. Steps accept `completed`, `editable`, `optional`, `invalid`, and `disabled`. See the [stepper contract](./stepper-contract.md). | palette, navigation, workflow, stroke, bend, gap, coupling, control, typography | A labeled ordered list owns roving native header buttons with `aria-current="step"`; stable reciprocal regions retain authored panel state. Application facts control validation/completion. |
| `cem-step` | Inert step payload consumed by `cem-stepper`. | `value`, `label`, status attributes, and default panel content. | typography | Adds no independent role, focus, event, form, or validation owner. |
| `cem-paginator` | Labeled paged-content navigation with application-owned data. | `length`, zero-based `page-index`, `page-size`, whitespace-separated `page-size-options`, `show-first-last`, `hide-page-size`, `disabled`, `name`, `label`, and localizable control/range labels. See the [paginator contract](./paginator-contract.md). | action, palette, select, control, stroke, bend, gap, inset, typography | A native navigation landmark owns a labeled page-size select, named native actions, focus-stable `aria-disabled` boundaries, and one atomic polite range status. Applications consume `cem-page` to load/render data. |

States: `default`, `hover`, `focus-visible`, `active`, `disabled`, `selected`, `expanded`.
The public stylesheet implements hover and focus-visible on the actual nav
links/buttons, collapsible disclosure button, and tab buttons—not their
structural wrappers. Generated navigation-item pairs preserve
current-link/selected-tab meaning and suppress enabled hover styling for native
or ARIA-disabled owners. Existing D5 stroke/offset and zebra color semantics
provide an external focus ring that coexists with hover and selection without
changing geometry. Navigation active/current-active pairs provide native held
feedback; disclosure release remains owned by its expanded contract. Forced
colors use platform system colors. See the
[navigation hover contract](./navigation-hover-contract.md) and
[navigation focus-visible contract](./navigation-focus-visible-contract.md),
the [navigation active contract](./navigation-active-contract.md), and the
[navigation disabled contract](./navigation-disabled-contract.md). Native
disabled buttons leave the tab order and use browser suppression; `cem-tabs`
uses that native-disabled contract exclusively. Direct ARIA-disabled `cem-nav`
owners remain keyboard-discoverable while host capture behavior
prevents pointer, programmatic, Enter, and native-button Space activation
before target/application bubble listeners or default action.

`cem-paginator` uses its native select and action buttons as the only hover,
focus, active, and disabled owners; the landmark, range/actions group, and label
remain structural. Render-only normalization does not rewrite invalid or
out-of-range author attributes. Page-size changes preserve the first visible
item, successful user requests emit one serializable `cem-page`, and boundary
or globally disabled activation is suppressed. Existing theme semantics cover
normal and forced colors without a CSS exception.

## Content

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-text` | Token-scoped inline text and typography owner. | `variant` selects the semantic typography treatment; `text` or the default slot supplies content. | typography, palette | Preserves ordinary inline text semantics and adds no focus or interaction owner. |
| `cem-icon` | Passive icon with optional adjacent content. | `image` selects Unicode, image URL, Font Awesome or Material Icons; the default is `circle` when `image` is absent. Empty `image` hides the glyph. `size="normal|small|large"`, `direction="row|column"`, `align="start|center|end"` (default center); default slot supplies adjacent content. | icon, typography | The default slot overrides visible `label` fallback (empty by default), preserving markup. Without `aria-label`, the glyph is decorative. A nonempty `aria-label` provides one image role; slotted content stays independently accessible. See the [icon contract](./icon-contract.md). |
| `cem-card` | Summary surface and explicit content-loading boundary. | `slot="title"` for heading; default slot for body. Presence-only `busy` retains the authored payload and reflects loading state without starting resource work. | palette, stroke, bend, gap, inset | `label` names the section. Busy state adds exact `data-state="loading"` and `aria-busy="true"` without a live region, inert subtree, or focus move. |
| `cem-expansion` | One independent general-purpose disclosure panel. | `slot="summary"` or `label` names the native header; default slot supplies persistent panel content. Presence-only `expanded`, `disabled`, and `region`; `heading-level="1..6"` defaults to 3. See the [expansion contract](./expansion-contract.md). | action, palette, stroke, bend, gap, inset, coupling, control, typography | The header is the sole button owner inside a validated heading. Exact `aria-labelledby`, `aria-expanded`, `aria-controls`, and reciprocal panel references remain stable; collapsed content is hidden and disabled blocks only user toggling. |
| `cem-list` | Passive collection wrapper by default; native single-select listbox with `selectable`. | Passive mode projects `<li>` rows. Selectable mode consumes direct `cem-list-option` payload with required `value` and optional `selected`/`disabled`; parent `label`, `value`, and `size` configure the listbox. | palette, content, stroke, gap, typography | The list or listbox must be named. Selectable mode keeps focus, hover, and keyboard behavior on the native `<select>`, reflects exact option `aria-selected`, and does not participate in forms. |
| `cem-table` | Passive structured comparison table. | `label` (default Table); project ARIA rows/cells. | palette, stroke, gap, typography | A named `part=root` table projects rows without layout, sorting, selection, loading or focus behavior. Empty content supplies one No rows cell. See the [disclosure/data audit](./disclosure-data-contract.md). |
| `cem-sort-header` | Sortable-column action composed inside a table row. | `label` supplies visible text and the `Sort by …` action name; `name` identifies `cem-sort` detail; `direction="ascending|descending"` or absence supplies state; `disabled` suppresses native activation. See the [sort-header contract](./sort-header-contract.md). | action, control, stroke, bend, gap, coupling, typography | A generated `role="columnheader"` conditionally owns `aria-sort`; its direct native button owns focus and Space/Enter. The application consumes `cem-sort` to reorder rows and announce the localized result. |
| `cem-chip` | Compact label or filter toggle. | Default slot is visible label; `label` can provide a fuller name. Without `checkable`, renders a passive `<span>`. With `checkable`, renders a native toggle `<button>` and uses the presence-only `checked` attribute as its initial state. | palette, content, action, stroke, bend, inset, typography | Checkable chips mirror their boolean slice through `aria-pressed`; hover and keyboard focus retain checked meaning, while removable chips need a separate named remove action. |
| `cem-badge` | Status/count/severity label. | Default slot is text; `tone` maps to status styling. | palette, bend, inset, typography | Badge text must be visible or included in adjacent accessible text. |
| `cem-avatar` | Person or organization identity. | `label` names identity; `initials` fallback or projected media. | palette, bend, typography | Renders `role="img"` and requires a label. |
| `cem-media-preview` | Asset, literal text or isolated document preview. | `mode="media|text|document|message"` (media by default), `value`, `label`, presence-based `truncated`; media uses the default and `caption` slots. | palette, stroke, bend, gap, inset, script/UI typography | Media owns its alternative text. Text is exact and keyboard-scrollable; document uses an opaque sandbox and fixed deny-all CSP. See the [preview contract](./passive-payload-contract.md#explicit-output-preview-modes). |
| `cem-tree` | Generic expandable hierarchy with application-owned data. | Strict recursive `cem-tree-item` payloads require unique whitespace-free `value` and non-empty `label`; host accepts `expanded-values`, `selection`, `selected-values`, `disabled`, and `loading-label`. See the [tree contract](./tree-contract.md). | palette, content, control, stroke, bend, gap, coupling, typography | A labeled tree owns exact native button treeitems, stable owned groups, explicit levels/positions/set sizes, parent expansion, optional selection, loading status, and one visible enabled roving tab stop. |
| `cem-tree-item` | Inert recursive hierarchy payload consumed by `cem-tree`. | `value`, `label`, optional `expandable`, `disabled`, `loading`, and nested tree-item children. | typography | Adds no independent role, focus, event, selection, loading lifecycle, request, or navigation owner. |

States: `default`, `hover`, `focus-visible`, `selected`, `loading`, `empty`, `checked`.

`cem-expansion` additionally owns contextual `active`, `disabled`, and `expanded`
states for its native header without turning content disclosure into navigation or
selection. Multiple siblings are independent; exclusive accordion-group policy
is not part of this contract.

`cem-sort-header` additionally owns contextual `active`, `disabled`,
`ascending`, `descending`, and `none` states. Only its native button receives
interaction paint; `cem-table`, the host, and the column-header wrapper remain
structural. Existing theme semantics cover every binding, including forced
colors, so no CSS exception is recorded.

`cem-tree` additionally owns contextual `active`, `disabled`, and `expanded`
states on exact treeitem buttons. Selection and loading are projected application
facts; activation never mutates selection or starts resource work. Structural
item/group wrappers own hierarchy layout only. The completed generic content
interaction family and the dedicated forced-colors gate cover every state, so
no component CSS exception is recorded.

In v1, `content:loading` is owned only by `cem-card[busy]`; lists, tables, and
media previews neither infer nor inherit it. Authors retain last-known content
during refresh or provide visible loading text plus layout-preserving
`cem-skeleton` payload for initial loading. See the
[content loading contract](./content-loading-contract.md).

## Feedback

| Component | Semantics | Content and Attributes | Token Families | Required A11y |
| --- | --- | --- | --- | --- |
| `cem-tooltip` | Supplemental native tooltip description. | `message` or label slot; trigger slot, `trigger-for` or native interest; delays, `disabled` and logical `placement`. See [contract](./tooltip-contract.md). | palette, stroke, bend, gap, inset, layering, typography | The same non-focusable native Popover supplies the persistent accessible description; focus stays on the native trigger. |
| `cem-dialog` | Native nonmodal task by default; explicit modal mode. | Heading `label`/label slot; eager or template body; launcher/close conveniences, commands, focus and geometry references. See [contract](./dialog-contract.md). | palette, stroke, bend, gap, inset | Shared `native-surface` owns lifecycle; native dialog owns form behavior and modality. |
| `cem-dialog-shell` | Compatibility alias for the native dialog contract. | Same attributes, slots and shared capability as `cem-dialog`. | palette, stroke, bend, gap, inset | Stable native dialog with heading naming, explicit mode and shared focus restoration. |
| `cem-sheet` | Static non-modal task surface by default; application-controlled visible/hidden region with `transient`. | `label` supplies the visible heading and default region name; `slot="label"` replaces visible markup. `aria-label`/`aria-labelledby` name it separately, and `aria-describedby` supplies help. Default slot is retained body. In transient mode, presence-only `expanded` removes `hidden`. | palette, stroke, bend, gap, inset | Always remains a labeled `<aside role="region">`. It does not trap or move focus, intercept Escape, make the document inert, or dispatch dialog dismissal. |
| `cem-toast` | Transient status message. | Default slot is message text. | palette, stroke, inset, typography | Renders polite `role="status"` live region. |
| `cem-progress` | Determinate or indeterminate progress. | `value`, `max`, and `label`. | progress | Native progress must have an accessible name. |
| `cem-progress-spinner` | Non-interactive circular determinate or indeterminate progress. | `label` names the progressbar; presence of `value` selects determinate mode, absence selects indeterminate mode; `max` defaults to 100; `describedby` may reference task context. | progress, timing | Exposes normalized range values only when determinate, hides its SVG from assistive technology, has no tab stop or live region, and stops automatic rotation under reduced motion. |
| `cem-skeleton` | Loading placeholder. | `label` describes placeholder for author/debug context. | palette, control, bend, stroke | Rendered placeholder is `aria-hidden` and inert; pair with visible status when needed. |
| `cem-alert` | Inline feedback. | `tone` controls visual severity; `role` defaults to `status`. | palette, stroke, inset, typography | Use `role="alert"` for urgent warnings/errors only. |

States: `default`, `focus-visible`, `loading`, `expanded`, `invalid`, `indeterminate`.

The [tooltip contract](./tooltip-contract.md) defines native trigger ownership,
independent focus/hover interest, touch preservation, description cleanup and
shared placement. The same owner supplies description and visible presentation.

`cem-progress-spinner` is the distinct circular owner; linear `cem-progress`
does not change shape to satisfy it. Missing `value` means indeterminate, and
live `value`/`max` changes retain the same SVG and geometry. D0 progress colors,
D2c progress geometry, and D7 cycle/easing tokens cover its stylesheet. Forced
colors use `GrayText` for the remaining track and `Highlight` for the indicator;
reduced motion leaves a static incomplete arc. See the
[progress spinner contract](./progress-spinner-contract.md).

The [native dialog contract](./dialog-contract.md) defines commands, explicit
modal mode, cancelable closing, focus restoration and retained/disposed bodies.
The [feedback/status declaration contract](./feedback-status-contract.md) covers retained message owners, explicit urgency, native progress, shared circular preparation and inert skeletons.

The [feedback focus contract](./feedback-focus-visible-contract.md) covers
actual native focus owners and forced colors. Sheets remain separate regions.

### Action dimensions

`size` selects a theme minimum visible height: small `2.5rem`, medium `3rem`,
large `4rem`, x-large `6rem`. Omit/remove it to inherit container geometry,
including `data-cem-size`. Width and flex/grid stretch remain CSS layout concerns.
The native button reserves at least `--cem-coupling-zone-min` in both axes;
compact profiles use a transparent hit border and clip paint to the padding box.
This border is not a visible intent outline. Zebra marks the complete target.
Keep at least `--cem-coupling-guard-min` between neighboring targets.
See the [dimension decision](../../cem-theme/src/lib/tokens/cem-dimension.md#decision-optional-component-dimensions-and-compact-hit-areas).

### Action contrast modes

Contrast-light/dark collapse action fills to the surface and use surface ink.
Intent appears in the contour; hover and active increase its thickness without
changing layout. Keyboard zebra remains independent. Loading animates a masked
zebra contour, including while disabled, with a flat surface behind the label.
Reduced motion freezes that contour; forced colors uses a static system outline.
Ordinary light/dark/native modes retain pending background gradients. Both the
contrast state values and contour tokens are generated from canonical theme docs.

## Canonical command components

`cem-icon-button` and `cem-menu-item` load from their individual XHTML declarations
or `components.xhtml#<tag>`. Both expose their native button as `part="control"`;
icon-button also exposes the decorative glyph as `part="icon"`.

| Component | Implemented component attributes | Other explicit attributes |
| --- | --- | --- |
| `cem-icon-button` | `image`, `size`, `label`, `aria-label`, `href`, `type`, `name`, `value`, `form`, `formaction`, `formenctype`, `formmethod`, `formtarget`, `formnovalidate`, `direction`, `align="start|center|end"`, `variant`, `disabled` (presence), `selected`, `selectable` (presence), `pending="true|false"`; see [the full contract](icon-button-contract.md) | `hidden`, `class` |
| `cem-menu-item` | `label` (default-slot fallback, default Menu item), `disabled` (presence), `expanded` (standalone ARIA value), `href` (leaf link), `aria-label`; default and `submenu` slots | `hidden`, `class` |

Icon-button uses the action variants primary (default), explicit, contextual, alternate and destructive in both modes. It supports action bend classes and native button/form attributes. Native
clicks update the `pressed` (icon-button) or `selected` (menu-item) event slice;
these are event records, not persistent selected/toggle state. Menu navigation and
popup ownership remain the containing menu's responsibility. Default slot content
is supported by both controls. Boolean disabled presence includes `disabled="false"`.

Each component has a matching property playground under `playgrounds/`, covering
all listed attributes and showing its canonical source.


### Persistent and nested menus

`cem-menu` loads from `components/cem-menu/cem-menu.xhtml#cem-menu`. Its default
slot preserves links, buttons and `cem-menu-item` controls in a wrapping flex
container. `direction="row|column"` defaults to row and `justify="start|end"`
defaults to start. `aria-label` and `aria-labelledby` label the layout container;
`hidden` uses presence. `dir` inherits normally. Arbitrary content receives no
item paint.

The shared composite leaves `--cem-inset-control` around its items, floored at
`--cem-coupling-guard-min`. The `--cem-gap-related` between items is floored at
twice that guard, leaving room for both a selected item and its focused
neighbor's zebra indicators. This applies to row, column and nested popup menus,
including projected `cem-action` controls. Inherited spacing overrides apply
without changing each item's own control padding.

`keyboard="native|menu"` defaults to native. Native mode keeps ordinary Tab
navigation. Menu mode exposes a row menubar or column menu with one enabled
roving stop. Arrows follow orientation and RTL; Home/End and case-insensitive
typeahead skip disabled and hidden controls. Enter/Space retain native control
activation. Empty or all-disabled menus have no roving stop.

Place a nested `cem-menu slot="submenu"` directly inside a `cem-menu-item`.
Submenus require root menu mode, inherit that behavior, default to columns and
open below row triggers or beside column triggers, flipping/clamping to the
viewport. Owners remain buttons even when href is supplied. Leaf href items
render native anchors; disabled leaves omit href and suppress activation.
Opening focuses the panel's first enabled item. Escape closes the nearest panel
and restores the trigger. Tab, outside activation and leaf activation dismiss
open chains without a focus trap; only one sibling panel remains open. Runtime
IDs link controls, trigger names and panels. Roots remain persistent.

Shared `composite-menu` and `action-control` capabilities own browser behavior
in cem-elements. Dropdown composition supplies separately triggered root popups;
context menus, checkbox/radio items and lazy loading are deferred.

Legacy source audit: [Material menu](../../custom-element/material/components/menu.html)
(version in this repository) maps Default, Vertical, Horizontal, scoped and
inline styling, wrap menu and wrap menu right to the canonical gallery's
Default, Vertical, Horizontal, Scoped token overrides, Token overrides,
Wrapping and End alignment examples respectively. Customization now uses the
consumed contextual action and inset tokens. No external menu assets are needed.


### Dropdown and nested menu composition

`cem-dropdown` loads from `components/cem-dropdown/cem-dropdown.xhtml#cem-dropdown`.
`label` supplies fallback trigger text (Dropdown); `aria-label` names it separately.
The `base` slot overrides the fallback native button: supply a direct native button,
link or focusable trigger. With `trigger-for`, the declared external trigger owns activation and the fallback button is omitted. The default slot accepts arbitrary HTML or a menu.
`open="false"` closes; other values open, including the source's default visible
state. Use explicit `open="false"` for an initially closed interactive dropdown.
`disabled` and `hidden` use presence; `dir` and `class` inherit normally.

The panel height follows its content, including menu item spacing. There is no
fixed row-count cap. The shared popup controller constrains the panel to the
viewport or declared boundary; vertical scrolling appears only when the content
exceeds that available height.

The trigger toggles on native click/Enter/Space; ArrowDown opens and focuses the
first enabled panel control. Escape restores trigger focus. Tab, focus leaving
the component and outside activation dismiss without a trap. Geometry flips and
clamps within the viewport when opened or resized. Like the legacy source, the
dropdown uses a relative host and an absolute panel at `z-index: 1`, with square
top corners. Initially open panels retain the CSS position below their triggers,
including offscreen examples. Scrolling moves the panel
with its trigger, including beyond the viewport edge. The declaration consumes existing action, control,
bend, typography, focus and elevation tokens instead of defining
component-specific shadow/height/width aliases.

Place `cem-menu direction="column" keyboard="menu"` in the default slot.
Nested menu-item submenu slots keep their existing composition. Both capabilities
consume the shared popup controller. The nearest menu handles Escape first;
only when its submenu chain is closed does the outer dropdown handle Escape.
Leaf menu activation dispatches shared popup dismissal, closing the complete
chain. Popup wrappers and menu navigation retain separate ownership; no gallery
or component JavaScript implements their behavior.

Source audit: [dropdown 0.0.39](https://unpkg.com/@epa-wg/custom-element-dist@0.0.39/src/material/components/dropdown.html)
maps HTML in dropdown, Menu in dropdown and Hide dropdown dynamically to the
same-named canonical gallery examples. The source's external image is replaced
with the shipped custom-element SVG asset. The source's label becomes a native
button so its existing keyboard focus also supports keyboard activation.


External declaration aliases inherit their intrinsic source capabilities:
`composite-menu` for `cem-menu`, `action-control` for `cem-menu-item`, and `popup`
for `cem-dropdown`. Use `<cem-element tag="cem-menu" src="…#cem-menu">` without
repeating the capability. Preserve source metadata when copying templates;
explicit importer overrides can disable the keyboard or popup behavior described
here. The [metadata ownership rules](./conventions.md#41-intrinsic-capability-metadata)
apply to all three.

In a single-choice menu, `cem-action` and `cem-icon-button` with
`control-role="menuitemradio"` map their container-owned `selected` state to
`aria-checked` and the same selected zebra outline. Other selectable buttons
continue to expose `aria-pressed`. The theme switch uses icon buttons so its
Unicode sun/moon glyphs use `cem-icon` sizing separately from the choice labels.
Its choices use `align="start"`; both icon components support logical inline
alignment independently of row/column direction, with center as the default.
