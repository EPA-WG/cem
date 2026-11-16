# Capability coupling audit — 2026-11-16

Baseline: `3080c117` on `develop`, plus the choice-control correction below.
Authority: [declarative UI principle](declarative-ui-principle.md). Framework
JavaScript is permitted; component-specific runtime dispatch is not.

## Scope and method

Reviewed all 24 entries in `CEM_DECLARATIVE_CAPABILITIES`, covering the 28
direct capability consumers and the two additional composed consumers in the
[59-component inventory](todo.md). Checked source imports, DOM owner selection,
retained payload discriminators, CSS writes and shared normalization helpers.
There are no imports of component implementation code in these capability
modules. The table records the contract boundary, not a claim that every behavior
has been independently reverified in every browser.

A native element, role, owned `part`/`slot`, or explicit reusable attribute is a
generic contract. Exact component tags and private presentation classes used to
select behavior are coupling. Generated ID prefixes, diagnostics and documented
event names are not element discriminators. For example, `host-view.ts` forwards
the `cem-tab` event's value; it does not select a `cem-tab` element. The
`cem-source-*` CSS Highlight names are a shared syntax-role protocol, not a
source-editor tag or class lookup. The range input's legacy
`data-cem-slider-thumb` alias is an explicit payload attribute; its preferred
contract is the native input's `slot`.

## Results

Source paths below are under
[`packages/cem-elements/src/lib`](../packages/cem-elements/src/lib/).
Seventeen capability boundaries use generic native/part contracts. Seven still
depend on component payload tags, including two through `choice-options.ts`.
The choice-control DOM and row-height coupling is corrected in this change;
the payload migrations remain open in [TODO](todo.md).

| Capability / source | Boundary reviewed | Result |
| --- | --- | --- |
| [live-preview](../packages/cem-elements/src/lib/live-preview-capability.ts) | Owned iframe/root parts, sandbox policy, retained preview service and explicit resource envelope | Generic |
| [progress-status](../packages/cem-elements/src/lib/progress-status-capability.ts) | Numeric progress attributes and derived slices | Generic |
| [disclosure-panel](../packages/cem-elements/src/lib/disclosure-panel-capability.ts) | Native button, heading/panel/root parts and ARIA relationships | Generic |
| [sort-action](../packages/cem-elements/src/lib/sort-action-capability.ts) | Native button, columnheader role and nearest table/grid ownership | Generic; native-table composition remains MW-05 |
| [pagination](../packages/cem-elements/src/lib/pagination-capability.ts) | Native nav/button/select owners, range-action and page-size parts | Generic |
| [native-list](../packages/cem-elements/src/lib/native-list-capability.ts) | Native select/ul output; input filters `cem-list-option` | **Open payload coupling**, including a legacy tag absent from the canonical component catalog |
| [navigation-landmark](../packages/cem-elements/src/lib/navigation-landmark-capability.ts) | Direct native nav/link/button owners, content part and disabled semantics | Generic |
| [tab-panels](../packages/cem-elements/src/lib/tab-panels-capability.ts) | Native buttons, list/panel parts; input accepts only `cem-tab` | **Open payload coupling** |
| [workflow-panels](../packages/cem-elements/src/lib/workflow-panels-capability.ts) | Native buttons, steps/panels parts; input accepts only `cem-step` | **Open payload coupling** |
| [tree-view](../packages/cem-elements/src/lib/tree-view-capability.ts) | Native node buttons/tree roles; recursive input requires `cem-tree-item` | **Open payload coupling** |
| [plaintext-editor](../packages/cem-elements/src/lib/plaintext-editor-capability.ts) | Native contenteditable textbox, Selection/Range, CSS Highlights and syntax roles | Generic |
| [time-input](../packages/cem-elements/src/lib/time-input-capability.ts) | Native input/button slots and popover; input requires `cem-option` and the shared choice reader | **Open payload coupling** |
| [range-input](../packages/cem-elements/src/lib/range-input-capability.ts) | Native range inputs, thumb slots, bounds and dimensionless thumb-position geometry | Generic; legacy attribute alias recorded above |
| [calendar-input](../packages/cem-elements/src/lib/calendar-input-capability.ts) | Native input/button slots, dialog/grid and date arithmetic | Generic |
| [editable-choice](../packages/cem-elements/src/lib/editable-choice-capability.ts) | Native input/combobox and option parts; shared choice reader recognizes component tags | **Open payload coupling** |
| [checkable-control](../packages/cem-elements/src/lib/checkable-control.ts) | Native checkbox indeterminate property, native radio/form semantics and control part | Generic |
| [suggestions](../packages/cem-elements/src/lib/suggestions-capability.ts) | Native datalist/editor references, retained source/provider and placement contracts | Generic |
| [native-surface](../packages/cem-elements/src/lib/native-surface.ts) | Native dialog/popover, standard dialog/tooltip roles, preparation/focus/resource policies | Generic; surface kind is a role policy, not component-tag dispatch |
| [form-control](../packages/cem-elements/src/lib/form-control-capability.ts) | Owned native input/textarea, FACE, validation, editing and datalist claims | Generic |
| [popup](../packages/cem-elements/src/lib/popup-capability.ts) | Native invoker/popup binding, base/popup parts and typed references | Generic |
| [action-control](../packages/cem-elements/src/lib/action-control-capability.ts) | Native button/link, control part and submenu slot | Generic |
| [action-command](../packages/cem-elements/src/lib/action-command-capability.ts) | Native invoker/target routes, control/surface parts and retained task invocation | Generic |
| [composite-menu](../packages/cem-elements/src/lib/composite-menu-capability.ts) | Composite parts, native buttons/links, submenu slot/parent references and shared popover binding | Generic |
| [choice-select](../packages/cem-elements/src/lib/choice-select-capability.ts) | FACE and choice semantics; private classes, `cem-radio` queries and theme-based inline height removed | **DOM coupling corrected; shared payload coupling remains** |

## Choice-control correction

`choice-select` now finds one owned `part="control"`, either directly on the
host or directly inside its `part="root"`. Dropdown activation checks containment
by that owner. Adaptive radio focus uses actual native `input[type="radio"]`
checked/disabled state inside that control, independent of any wrapper tag.
When inputs are not materialized yet, focus follows the original owned item
renders, checks the current lifecycle and retains any newer outside destination.

Listbox geometry projects only the normalized dimensionless
`--_cem-choice-visible-rows` count. The declaration computes block size using
the existing theme-owned `--cem-list-row-height`; framework JavaScript no longer
contains that visual token or writes a component-specific block size. This is
runtime state, not a new theme token or a component-local default. The generic
capability's behavior identity is renewed to `cem-elements-choice-select-v3`.

The independent framework fixtures rename both the host and radio tags and all
private classes, retain the production declaration's parts, and run worker and
fallback paths. They exercise trusted activation, keyboard selection, validation,
FACE/reset, inherited row-height geometry, mode-change focus and scope teardown.
The existing wrapper-validation fixtures and the framework's source-loaded form
example use `part="control"` as well.

## Verification

The four new independent choice stories first fail on the original class/tag
lookups. They now pass in worker and fallback modes, including newer outside
focus and reconnects. All 20 canonical select stories and both wrapper-validation
stories pass; the source-loaded form story passes all five authored examples.
All 27 declaration-identity/form-source unit tests and four selected choice/form
checkpoint stories pass. Runtime build/typecheck/lint/package and component
architecture/style/state/catalog/playground/bundle/package gates pass. Lint has
existing warnings. No timeout or worker-budget changes were made.

The focused delivery gate is
`yarn nx run @epa-wg/cem-components:verify-playgrounds --excludeTaskDependencies --args='--choices-only'`.
It reuses the property, gallery and bundle checks and additionally verifies
row-count ownership and forced-color geometry/focus. Source, isolated package
archives and bundled pages pass in Chromium 148.0.7778.96 and Firefox 150.0.2.
Focus-ring assertions use actual Tab entry rather than assuming that a modifier
key after programmatic focus changes the browser's focus-visible modality.
The completed checklist is in [TODO](todo.md).

## Remaining migration

Replace the six component-aware readers, rather than merely renaming their
string literals: [choice-options](../packages/cem-elements/src/lib/choice-options.ts),
time-input, native-list, tab-panels, workflow-panels and tree-view. The
[versioned retained-payload item/group contract](cem-retained-payload-contract.md)
is now available before the first model normalization in source and compiled
declarations. Reader adoption remains open and must preserve native option/optgroup compatibility, strict
authoring diagnostics, recursive topology, original source maps/owners and
checkpoint behavior. Components should declare their vocabulary through that
generic contract. A configurable selector must not grant a DOM node data or
recovery authority. This audit does not mark those migrations complete.

## Guidance and limits

The `web-components` guide informed the native-owner review: “wrap the real
`<button>` rather than re-implementing it.” CEM intentionally uses light DOM;
the guide's Shadow DOM recommendations do not change the repository's normative
architecture. No new shadow tree, custom native-input replacement, component
JavaScript or component callback registry was introduced.

The audit does not establish that every application entry point or every runtime
helper is free of coupling. Physical assistive-technology acceptance and the
separate whole-suite stall investigation remain open.
