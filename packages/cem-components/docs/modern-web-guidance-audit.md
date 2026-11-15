# CEM component web audit — 2026-11-15

Source baseline: `ffea182d3681d3d3d163259c789d627d98a9a835` on `develop`, plus
the two fixes recorded below. Scope: all **59** canonical declarations and their
colocated stories. This is a source/contract and automated-browser audit, not a
WCAG conformance declaration or physical assistive-technology acceptance.

## Method and authority

Applied the repository's [modern-web-guidance skill](../../../.agents/skills/modern-web-guidance/SKILL.md),
version `2026_11_09-5bcc021b`: searched `build accessible form controls` and
`navigate composite widgets`, listed the catalog once for the broad audit, then
retrieved the guides below. Reviewed rendered-owner markup, scoped CSS and
relevant shared capability/contract boundaries. Ran all colocated stories and
the shared declarative, style, state-matrix and publication gates. The matrix
links directly to each declaration and executable story module; a passing story
count means the existing assertions passed, not that every guide recommendation
has a test. Open findings are in [TODO](../../../docs/todo.md).

Guides reviewed:

- Disciplines: `html`, `css`, `accessibility`, `color`, `forms`, `web-components`,
  `security`, `css-layout`, `design-token-reactivity`, `motion`, `performance`.
- Patterns: `checkbox`, `switch`, `slider`, `progress`, `progress-ring`, `tabs`,
  `accordion`, `menu`, `app-menus`, `interest-triggered-tooltips`,
  `responsive-table`, `sortable-table`, `persistent-toast-notifications`, `icons`,
  `search-hidden-content`, `dialog`, `viewport-aware-dropdowns`.

The governing implementation constraints remain [declarative CEM](../../../docs/declarative-ui-principle.md),
[component conventions](conventions.md), [accessibility](accessibility.md),
[native retained CSS](../../../docs/scoped-css-module-maps.md), and
[canonical theme ownership](../../cem-theme/src/lib/tokens/index.md#core-cem-principles).
Guidance is applied to the actual native/ARIA owner, including runtime-applied
attributes; inspecting only the custom-element host gives misleading results.
For example, text fields use a form-associated host through shared
`form-control`, while checkables and authored picker/range inputs retain native
form ownership. Light DOM is intentional; no Shadow DOM conversion is needed.

## Findings and decisions

| ID | Priority / state | Evidence, implication and next action |
| --- | --- | --- |
| MW-01 | Fixed | Passive `cem-chip` emitted `aria-label` on an otherwise generic span. Removed that unsupported naming attribute without inventing a role or tab stop; visible slot/fallback text remains. Checkable chips retain their named native toggle button. The `accessibility` guide and [WAI-ARIA generic role](https://www.w3.org/TR/wai-aria-1.2/#generic) prohibit author naming on this role. |
| MW-02 | Fixed | Text-mode `cem-media-preview` had a focusable, scrollable `pre` with an accessible-name attribute but no nameable region role. Added `role="region"`; worker/fallback stories query the named role, preserve exact hostile literal text, and retain focus/node identity after label changes. Text mode also handles plain output, so it does not falsely mark every payload as computer code. The `html` guide requires a named region for scrollable code blocks. |
| MW-03 | P1 / open | `cem-menu-item` authors `role="menuitem"` on both button and link. `composite-menu-capability.ts` captures that as the original role and restores it in native mode, although its container becomes `role="group"` and Tab navigation remains ordinary. Native activation is covered by `NativeActivationAndBoundaries`, but that story does not assert the roles. Reconcile native/standalone versus composite role ownership; add mode-transition and nested-ownership regressions before changing defaults. See `menu` and `accessibility`. |
| MW-04 | P1 / open | `cem-icon-button` defaults its button name to “Icon action”, as explicitly documented in its contract. This conflicts with the general accessibility contract's missing-name diagnostic/no-invention rule and does not identify the actual command. Decide the compatible naming/diagnostic behavior in shared runtime, then cover visible slot text, explicit/external labels and empty names. Do not silently delete a public fallback or introduce component-local JS. See `icons` and `accessibility`. |
| MW-05 | P2 / design gap | `cem-table` uses an ARIA table wrapper and caller-supplied ARIA rows/cells; `cem-sort-header` produces an ARIA columnheader with a native button and an action name. `responsive-table`/`sortable-table` prefer native table/caption/th ownership and a stable column name. `cem-data-view` already provides a native table, caption and scoped headers. Define a compatible native table/sort composition rather than inserting `th` under a `div` or claiming a sort-header event reorders data. Applications still own row order and completed-result announcements. |
| MW-06 | P2 / design gap | `cem-tabs` wraps its horizontal tablist and gives every selected panel a tab stop, even when its payload already has focusable content. `tabs` recommends single-row scrolling and conditional panel focusability. Manual activation is intentional for potentially expensive Studio views and stays valid. Resolve wrapping/tab-stop contracts through shared `tab-panels`; preserve stable IDs, retained controls, disabled behavior and the selected/focused distinction. |
| MW-07 | P2 / API gap | `cem-field`, `cem-text-field` and `cem-textarea` use wrapping labels and shared FACE submission/validity/reset. Their declarations do not forward all native editing/naming attributes: for example host `aria-labelledby`/`aria-label`, `enterkeyhint`, `autocorrect`, and textarea `inputmode`/`spellcheck` are absent. The forms/accessibility guides also favor explicit label/control IDs for voice tools. Define supported forwarding centrally and test actual native owners plus host external-form/validity behavior; do not create duplicate submitted inputs. Existing wrapping labels remain meaningful. |
| MW-08 | P2 / behavior gap | `native-surface.ts` appends the tooltip ID to `aria-describedby` without comparing the hint to the trigger's name or respecting an existing author description as exclusive. The current explicit manual-popover controller provides hover retention, Escape suppression, touch non-interception and cleanup, but differs from the retrieved native hint/interest pattern. Reconcile duplicate descriptions, unnamed triggers and hint composition in the shared controller; verify nested menus and feature detection before changing the popover kind. |
| MW-09 | P3 / evaluation | `cem-expansion` uses a native button inside an ARIA heading, not native details/summary or an h1–h6; `cem-switch` is a native checkbox with switch semantics but no native `switch` appearance attribute; `cem-progress-spinner` is an ARIA progressbar with hidden SVG. These have existing semantic/state contracts and are not automatically accessibility failures. Evaluate the guides' native alternatives, feature-detected fallback and API/retention implications before changing them. |
| MW-10 | P2 / verification gap | Canonical super-compact density intentionally sets the coupling floor and block padding to zero on every input modality. That is not proof of the guides' 24px target/spacing exception or larger touch-target recommendations. Measure actual adjacent targets, overflow, 200% text sizing, text spacing and 320px reflow in all themes/densities. Preserve this explicit owner policy; any new density or spacing decision belongs to canonical tokens, not local component defaults. |
| MW-11 | P2 / verification gap | Native scoped CSS and host `hidden="until-found"` have explicit support boundaries; the latter intentionally has no polyfill. Shared popup geometry already has native anchor positioning and compatibility fitting. Do not infer browser support from a guide's summary or a successful Chromium run. Publish a per-feature support/refusal matrix; verify WebKit on a supported host, physical keyboard/touch, NVDA and VoiceOver. Browser DOM/story assertions do not establish spoken behavior, contrast across every combination, or production LCP/INP. |

The table lists confirmed source/contract gaps separately from optional native
modernizations and missing acceptance evidence. It does not treat all deviations
from a preferred guide implementation as WCAG failures.

### Applying guidance within the existing contracts

- Keep shared composite keyboard controllers as the current owner. Adopting
  `focusgroup` would require feature detection and retiring duplicate roving
  listeners; adding the attribute alone is not a fix.
- Keep ordinary `hidden` on mutually exclusive panels until a shared
  `beforematch`/selection contract is designed. Host `hidden="until-found"`
  already follows the explicit native convention; do not silently introduce a
  reveal handler or polyfill that contradicts it.
- Keep `cem-toast`'s persistent inline-status scope. A queued, timed top-layer
  notification service with history, dismissal and focus return is separate
  functionality, not an attribute-only audit correction.
- Retain Google icon-font support requested by the owner. A CSS mask/token icon
  engine is an optional alternative, not a reason to remove working font paths.
- Preserve Studio's owner-requested radio-choice threshold of seven; the
  forms guide's smaller rule of thumb does not supersede that choice.
- Measure consumer performance before adding containment, lazy rendering or
  preload hints indiscriminately. Retained payload identity, visible media and
  accessible content must survive optimizations. Optional consumer builds may
  reduce dependencies; canonical theme/component release artifacts keep all
  public tokens and declarations.
- Preview sandbox/CSP/resource admission stays in the shared capability and
  application boundary. This audit does not grant ambient network or script
  access or claim deployment headers are owned by component markup.
- Dialogs keep native modal/nonmodal owners and the shared compatibility paths
  already required by the dialog contract. Prior modal-backdrop work recorded
  Chromium/Firefox pixel and native-reference wheel comparisons. That evidence
  is historical; this audit reruns their colocated stories, not those pixel
  probes. Native scroll containment remains progressive enhancement without
  root scroll locks; WebKit acceptance is still open.

## Component matrix

Every row also uses the `html`, `css`, `accessibility`, `color` and
`web-components` discipline review. “No additional finding” means none found in
this bounded source review; MW-10/MW-11 remain shared acceptance limits.
Story counts below total **411** across **59** passing files.

<!-- Rows name the exact source and story owners rather than generated copies. -->

| Component / declaration | Story evidence | Owner and guide-specific review | Result / remaining work |
| --- | --- | --- | --- |
| [cem-accordion](../src/components/cem-accordion/cem-accordion.xhtml) | [4 stories](../src/components/cem-accordion/cem-accordion.stories.ts) | Named group of expansion owners. Guides: accordion; css-layout. | Native-button keyboard ownership; no group arrow interception. MW-09. |
| [cem-action](../src/components/cem-action/cem-action.xhtml) | [21 stories](../src/components/cem-action/cem-action.stories.ts) | Native button; submit/command/popover attributes. Guides: forms; menu. | Type defaults to button; selection/menu-radio states remain distinct. No additional finding. |
| [cem-alert](../src/components/cem-alert/cem-alert.xhtml) | [4 stories](../src/components/cem-alert/cem-alert.stories.ts) | Status by default; explicit urgent alert. Guides: persistent-toast-notifications. | Tone does not implicitly escalate urgency; projected text stays literal. No additional finding. |
| [cem-app-bar](../src/components/cem-app-bar/cem-app-bar.xhtml) | [7 stories](../src/components/cem-app-bar/cem-app-bar.stories.ts) | Named header/banner; native projected controls. Guides: css-layout. | Global banner intent requires appropriate application placement/uniqueness. MW-11. |
| [cem-autocomplete](../src/components/cem-autocomplete/cem-autocomplete.xhtml) | [9 stories](../src/components/cem-autocomplete/cem-autocomplete.stories.ts) | Native text input/combobox; FACE host; grouped listbox. Guides: forms. | Expanded/controls/active-descendant state is shared editable-choice owned. MW-07/MW-11 acceptance applies. |
| [cem-avatar](../src/components/cem-avatar/cem-avatar.xhtml) | [5 stories](../src/components/cem-avatar/cem-avatar.stories.ts) | Named image role with initials/projected media. Guides: icons; performance. | Author supplies identity and media alternatives; no activation or live region. No additional finding. |
| [cem-badge](../src/components/cem-badge/cem-badge.xhtml) | [4 stories](../src/components/cem-badge/cem-badge.stories.ts) | Passive text span. Guides: css-layout. | No focus or live region; visible wording supplements tone. No additional finding. |
| [cem-card](../src/components/cem-card/cem-card.xhtml) | [8 stories](../src/components/cem-card/cem-card.stories.ts) | Optional named section; projected task controls. Guides: css-layout. | Busy state does not turn the whole surface into a live announcement. No additional finding. |
| [cem-checkbox](../src/components/cem-checkbox/cem-checkbox.xhtml) | [5 stories](../src/components/cem-checkbox/cem-checkbox.stories.ts) | Wrapping label and native checkbox. Guides: checkbox; forms. | Shared indeterminate property; native form/reset, no duplicate FACE value. MW-07 label-association review. |
| [cem-chip](../src/components/cem-chip/cem-chip.xhtml) | [1 stories](../src/components/cem-chip/cem-chip.stories.ts) | Passive span or native pressed button. Guides: forms. | MW-01 fixed; passive text/role/tab-stop regression and unchanged checkable behavior. |
| [cem-data-view](../src/components/cem-data-view/cem-data-view.xhtml) | [4 stories](../src/components/cem-data-view/cem-data-view.stories.ts) | Named scroll region; native table/caption/th. Guides: responsive-table; security. | Bounded typed data parsing; header scope preserved. MW-05 documents contrast with cem-table. |
| [cem-datepicker](../src/components/cem-datepicker/cem-datepicker.xhtml) | [10 stories](../src/components/cem-datepicker/cem-datepicker.stories.ts) | Retained authored input; native dialog/calendar grid. Guides: forms; css-layout. | Shared calendar/surface focus and ARIA owners; native modal scrim contract. MW-11. |
| [cem-dialog](../src/components/cem-dialog/cem-dialog.xhtml) | [9 stories](../src/components/cem-dialog/cem-dialog.stories.ts) | Native dialog; modal, local or popover state. Guides: css-layout; security. | Shared surface owns modality/dismissal/focus; modal and popover states are exclusive. MW-11. |
| [cem-dialog-shell](../src/components/cem-dialog-shell/cem-dialog-shell.xhtml) | [5 stories](../src/components/cem-dialog-shell/cem-dialog-shell.stories.ts) | Native dialog; projected trigger/body owners. Guides: css-layout. | Same shared surface lifecycle and modal-only scrim ownership. MW-11. |
| [cem-divider](../src/components/cem-divider/cem-divider.xhtml) | [6 stories](../src/components/cem-divider/cem-divider.stories.ts) | Separator role or hidden decoration. Guides: css-layout. | Orientation explicit; decorative path omits separator semantics. No additional finding. |
| [cem-dropdown](../src/components/cem-dropdown/cem-dropdown.xhtml) | [2 stories](../src/components/cem-dropdown/cem-dropdown.stories.ts) | Native trigger plus shared popup owner. Guides: menu; css-layout. | Geometry and dismissal shared; arbitrary popup content is not automatically a menu. MW-11. |
| [cem-expansion](../src/components/cem-expansion/cem-expansion.xhtml) | [6 stories](../src/components/cem-expansion/cem-expansion.stories.ts) | Native button; ARIA heading and retained panel. Guides: accordion; search-hidden-content. | No extra arrows; optional panel region avoids landmark proliferation. MW-09. |
| [cem-field](../src/components/cem-field/cem-field.xhtml) | [7 stories](../src/components/cem-field/cem-field.stories.ts) | Native input with wrapping label; FACE host. Guides: forms. | Internal form="" avoids duplicate submission; shared validity/reset. MW-07. |
| [cem-fieldset](../src/components/cem-fieldset/cem-fieldset.xhtml) | [6 stories](../src/components/cem-fieldset/cem-fieldset.stories.ts) | Native fieldset/legend and retained controls. Guides: forms. | Native disabled inheritance and legend naming. MW-07 explicit label association applies to children. |
| [cem-grid](../src/components/cem-grid/cem-grid.xhtml) | [4 stories](../src/components/cem-grid/cem-grid.stories.ts) | Structural grid wrapper. Guides: css-layout. | No invented landmark/focus semantics; token spacing and intrinsic tracks. MW-10 reflow measurement. |
| [cem-icon](../src/components/cem-icon/cem-icon.xhtml) | [7 stories](../src/components/cem-icon/cem-icon.stories.ts) | Decorative hidden glyph or named image role. Guides: icons; performance. | Unicode, image, font and Google ligature paths; caller owns font loading. No mandatory icon-engine rewrite. |
| [cem-icon-button](../src/components/cem-icon-button/cem-icon-button.xhtml) | [19 stories](../src/components/cem-icon-button/cem-icon-button.stories.ts) | Native button or anchor; embedded icon. Guides: icons; forms; menu. | Disabled link removes href/tab stop; button type is explicit. MW-04. |
| [cem-list](../src/components/cem-list/cem-list.xhtml) | [6 stories](../src/components/cem-list/cem-list.stories.ts) | Native selectable select or passive ul. Guides: forms. | Native options/change ownership; passive list keeps list semantics without a reset. No additional finding. |
| [cem-media-preview](../src/components/cem-media-preview/cem-media-preview.xhtml) | [12 stories](../src/components/cem-media-preview/cem-media-preview.stories.ts) | Named figure; text region or sandboxed titled iframe. Guides: html; security; performance. | MW-02 fixed; exact text remains inert; fixed document CSP and explicit live resource admission preserved. |
| [cem-menu](../src/components/cem-menu/cem-menu.xhtml) | [4 stories](../src/components/cem-menu/cem-menu.stories.ts) | Group in native mode; menu/menubar in composite mode. Guides: menu; app-menus. | Shared roving/typeahead and submenu geometry; no second focusgroup system added. MW-03/MW-11. |
| [cem-menu-item](../src/components/cem-menu-item/cem-menu-item.xhtml) | [7 stories](../src/components/cem-menu-item/cem-menu-item.stories.ts) | Native button or anchor with menuitem role. Guides: menu; app-menus. | Submenu trigger and leaf navigation have distinct paths. MW-03 native-mode role mismatch. |
| [cem-nav](../src/components/cem-nav/cem-nav.xhtml) | [10 stories](../src/components/cem-nav/cem-nav.stories.ts) | Named nav; optional native disclosure button. Guides: menu; accordion. | Navigation remains ordinary links rather than menu roles; expanded/hidden mirrored. No additional finding. |
| [cem-option](../src/components/cem-option/cem-option.xhtml) | [5 stories](../src/components/cem-option/cem-option.stories.ts) | Passive choice payload wrapper. Guides: forms. | Consuming select/autocomplete/time controller owns option semantics, state and focus. No additional finding. |
| [cem-option-group](../src/components/cem-option-group/cem-option-group.xhtml) | [7 stories](../src/components/cem-option-group/cem-option-group.stories.ts) | Passive group payload wrapper. Guides: forms. | Consuming controller supplies group name/role and inherited disabled state. No additional finding. |
| [cem-paginator](../src/components/cem-paginator/cem-paginator.xhtml) | [7 stories](../src/components/cem-paginator/cem-paginator.stories.ts) | Named nav; select and native action buttons; status. Guides: forms. | Native keyboard, range status and boundary-disabled state remain separate. MW-10 target measurement. |
| [cem-progress](../src/components/cem-progress/cem-progress.xhtml) | [4 stories](../src/components/cem-progress/cem-progress.stories.ts) | Named native progress. Guides: progress. | Indeterminate omits value; explicit zero retained. No additional finding. |
| [cem-progress-spinner](../src/components/cem-progress-spinner/cem-progress-spinner.xhtml) | [7 stories](../src/components/cem-progress-spinner/cem-progress-spinner.stories.ts) | Named ARIA progressbar; decorative SVG. Guides: progress-ring; motion. | Normalized determinate state, forced colors and static reduced-motion arc. MW-09 native alternative. |
| [cem-radio](../src/components/cem-radio/cem-radio.xhtml) | [4 stories](../src/components/cem-radio/cem-radio.stories.ts) | Wrapping label and native radio. Guides: forms; checkbox. | Browser group/form/reset semantics and disabled-peer skipping. MW-07 label association. |
| [cem-recovery-status](../src/components/cem-recovery-status/cem-recovery-status.xhtml) | [4 stories](../src/components/cem-recovery-status/cem-recovery-status.stories.ts) | Native details/summary; explanatory status text. Guides: accordion; security. | Disclosure reports observations, never authorizes recovery or reads credentials. No additional finding. |
| [cem-select](../src/components/cem-select/cem-select.xhtml) | [20 stories](../src/components/cem-select/cem-select.stories.ts) | FACE host; button combobox/listbox or native radio group. Guides: forms. | Choice capability owns value, active IDs and groups; no duplicate form values. MW-07/MW-11 acceptance. |
| [cem-sheet](../src/components/cem-sheet/cem-sheet.xhtml) | [4 stories](../src/components/cem-sheet/cem-sheet.stories.ts) | Named region/aside with heading. Guides: css-layout. | Inline feedback surface, not an implicit modal/focus trap; application owns opener. No additional finding. |
| [cem-skeleton](../src/components/cem-skeleton/cem-skeleton.xhtml) | [5 stories](../src/components/cem-skeleton/cem-skeleton.stories.ts) | Hidden, inert placeholder span. Guides: motion. | No accessible fake content or focus target; no authored animation. No additional finding. |
| [cem-slider](../src/components/cem-slider/cem-slider.xhtml) | [7 stories](../src/components/cem-slider/cem-slider.stories.ts) | Retained native range input(s); hidden visual track. Guides: slider; forms. | Shared range preparation; browser keyboard/form ownership, separate engine thumb styles. MW-10/MW-11. |
| [cem-sort-header](../src/components/cem-sort-header/cem-sort-header.xhtml) | [6 stories](../src/components/cem-sort-header/cem-sort-header.stories.ts) | ARIA columnheader containing native button. Guides: sortable-table. | Single sorted peer after activation; application owns row order/status. MW-05 native composition/name review. |
| [cem-source-editor](../src/components/cem-source-editor/cem-source-editor.xhtml) | [10 stories](../src/components/cem-source-editor/cem-source-editor.stories.ts) | Focusable plaintext-only editing pre with textbox role. Guides: forms; security. | Shared plaintext-editor owns edits/highlighting; read-only/disabled explicit. MW-11 physical AT/editing verification. |
| [cem-stack](../src/components/cem-stack/cem-stack.xhtml) | [4 stories](../src/components/cem-stack/cem-stack.stories.ts) | Structural flex wrapper. Guides: css-layout. | Layout alone adds no role, focus or live region. MW-10 narrow-content measurement. |
| [cem-step](../src/components/cem-step/cem-step.xhtml) | [5 stories](../src/components/cem-step/cem-step.stories.ts) | Passive retained step payload. Guides: tabs; forms. | Stepper owns workflow actions/regions; no independent step action. No additional finding. |
| [cem-stepper](../src/components/cem-stepper/cem-stepper.xhtml) | [7 stories](../src/components/cem-stepper/cem-stepper.stories.ts) | Named section, ordered steps, native buttons and regions. Guides: tabs; forms. | Current step differs from tab selection; manual workflow availability is explicit. MW-10/MW-11. |
| [cem-suggestions](../src/components/cem-suggestions/cem-suggestions.xhtml) | [13 stories](../src/components/cem-suggestions/cem-suggestions.stories.ts) | Retained editor; datalist or popover listbox; polite status. Guides: forms; security. | Shared suggestions owner supplies relationships and keyboard; native datalist limits remain explicit. MW-11. |
| [cem-surface](../src/components/cem-surface/cem-surface.xhtml) | [5 stories](../src/components/cem-surface/cem-surface.stories.ts) | Optional named section with busy/empty state. Guides: css-layout. | No whole-surface live region or synthetic focus. No additional finding. |
| [cem-switch](../src/components/cem-switch/cem-switch.xhtml) | [3 stories](../src/components/cem-switch/cem-switch.stories.ts) | Wrapping label and native checkbox with switch role. Guides: switch; forms. | Native checked/form/keyboard state retained. MW-09 native appearance evaluation; MW-07 labels. |
| [cem-tab](../src/components/cem-tab/cem-tab.xhtml) | [5 stories](../src/components/cem-tab/cem-tab.stories.ts) | Passive retained panel payload. Guides: tabs. | Tabs owns button/panel relationship and visibility. No additional finding beyond MW-06 consumer. |
| [cem-table](../src/components/cem-table/cem-table.xhtml) | [3 stories](../src/components/cem-table/cem-table.stories.ts) | Named ARIA table; projected rows/cells. Guides: responsive-table; sortable-table. | Passive table has no sorting controller; caller owns row semantics. MW-05. |
| [cem-tabs](../src/components/cem-tabs/cem-tabs.xhtml) | [7 stories](../src/components/cem-tabs/cem-tabs.stories.ts) | Tablist; native tab buttons and reciprocal panels. Guides: tabs; search-hidden-content. | Manual activation, stable IDs and retained payload controls are intentional. MW-06. |
| [cem-text](../src/components/cem-text/cem-text.xhtml) | [5 stories](../src/components/cem-text/cem-text.stories.ts) | Passive text span; visual variants. Guides: css-layout. | Heading variant changes typography only; authors must supply a real surrounding heading when needed. |
| [cem-text-field](../src/components/cem-text-field/cem-text-field.xhtml) | [7 stories](../src/components/cem-text-field/cem-text-field.stories.ts) | Native input with wrapping label; FACE host. Guides: forms. | Value slice and shared form owner; host name is not a native input name. MW-07. |
| [cem-textarea](../src/components/cem-textarea/cem-textarea.xhtml) | [13 stories](../src/components/cem-textarea/cem-textarea.stories.ts) | Native textarea with wrapping label; FACE host. Guides: forms. | Native rows/length/readonly and shared value/reset ownership. MW-07 editing attributes/labels. |
| [cem-theme-switch](../src/components/cem-theme-switch/cem-theme-switch.xhtml) | [9 stories](../src/components/cem-theme-switch/cem-theme-switch.stories.ts) | Named native icon trigger and radio menu items. Guides: menu; color; design-token-reactivity. | Six theme choices, including Presentation; storage/selection stays declarative. MW-11 composition verification. |
| [cem-timepicker](../src/components/cem-timepicker/cem-timepicker.xhtml) | [7 stories](../src/components/cem-timepicker/cem-timepicker.stories.ts) | Retained native input; popover listbox. Guides: forms. | Shared time-input sets popup name from the input and owns active options. MW-11. |
| [cem-toast](../src/components/cem-toast/cem-toast.xhtml) | [4 stories](../src/components/cem-toast/cem-toast.stories.ts) | Persistent inline polite status. Guides: persistent-toast-notifications. | Current contract is inline, without queue/timer/overlay; do not claim full overlay-toast implementation. |
| [cem-tooltip](../src/components/cem-tooltip/cem-tooltip.xhtml) | [4 stories](../src/components/cem-tooltip/cem-tooltip.stories.ts) | Noninteractive manual popover with tooltip role. Guides: interest-triggered-tooltips. | Persistent description, hoverability and Escape owned by native-surface. MW-08. |
| [cem-tree](../src/components/cem-tree/cem-tree.xhtml) | [13 stories](../src/components/cem-tree/cem-tree.stories.ts) | Named tree; native buttons exposed as treeitems. Guides: accessibility. | Stable hierarchy/position/selection/expanded state; shared tree-view keyboard. MW-10/MW-11. |
| [cem-tree-item](../src/components/cem-tree-item/cem-tree-item.xhtml) | [5 stories](../src/components/cem-tree-item/cem-tree-item.stories.ts) | Passive recursive tree payload. Guides: accessibility. | Tree owns levels, groups, focus and expansion. Standalone wrapper invents none. No additional finding. |
| [cem-workspace](../src/components/cem-workspace/cem-workspace.xhtml) | [10 stories](../src/components/cem-workspace/cem-workspace.stories.ts) | Named section with four headed regions. Guides: css-layout; performance. | Native/projected editor/actions remain owners; application chooses useful region labels. MW-10/MW-11. |

## Verification performed for this audit

Known Nx targets ran with `NX_DAEMON=false`; prerequisites were already built.
`--excludeTaskDependencies` avoided rebuilding/cleaning the shared runtime while
browser suites were running. No operation timeout, worker budget, checkpoint
limit, theme definition or dependency policy changed.

| Check | Result and scope |
| --- | --- |
| `yarn nx run cem-elements:test --excludeTaskDependencies --args='packages/cem-components/src/components --no-file-parallelism'` | Baseline before the two fixes: 59 files, 411 tests passed. |
| Same target restricted to `cem-chip/cem-chip.stories.ts` and `cem-media-preview/cem-media-preview.stories.ts` | After the fixes: 2 files, 13 tests passed, including worker/fallback exact text, named-role lookup and retained focus/node checks. |
| `yarn nx run-many -t verify-declarative verify-style-contract verify-state-matrix -p @epa-wg/cem-components --excludeTaskDependencies` | Passed; 59 canonical primitives, 497 generated theme tokens, 9 style-gate regression tests. The state-matrix gate is a coverage/accounting check, not a substitute for browser assertions. |
| `yarn nx run-many -t build:catalog build:playgrounds build:bundle -p @epa-wg/cem-components --excludeTaskDependencies` | Passed; catalog includes all 59 declarations and 8 source-linked examples; bundle refreshed from canonical declarations. |
| `yarn nx run @epa-wg/cem-components:verify-passive --excludeTaskDependencies --args='--passive-component=cem-media-preview'` | Passed: workspace and isolated packed property pages, five-theme galleries, bundle, literal text, named text region, visible focus and forced colors. |
| `yarn nx run @epa-wg/cem-components:verify-navigation --excludeTaskDependencies` | Passed on isolated rerun: workspace and isolated packed nav/tabs/stepper/tree/list/chip galleries, properties, project-navigation workflow and bundle. Added passive-chip text/role/name/tab-stop checks use existing public samples. |
| `yarn nx run @epa-wg/cem-components:verify-package --excludeTaskDependencies` | Passed: 329 packed files, all 59 canonical declaration/property/gallery exports, public catalog, one dist stylesheet and no retired runtime copies. |
| `yarn nx run @epa-wg/cem-components:lint --excludeTaskDependencies` | Passed with 0 errors and 180 existing non-null assertion warnings. |

The first navigation gate timed out at the unchanged `cem-list` gallery's
render-readiness predicate after 30 seconds; its subsequent diagnostic returned
an empty missing-sample list. The isolated rerun passed without a code or timeout
change. Nx reported the task as flaky. This is recorded as an unresolved
intermittent readiness observation, not attributed to these attribute changes.

No full all-family gallery rerun, WebKit run, public deployment, physical-device
or spoken assistive-technology acceptance is claimed. Theme inheritance and
forced-color checks exercise the existing contracts; they do not prove every
possible consumer combination. MW-03 through MW-11 remain actionable follow-ups.
