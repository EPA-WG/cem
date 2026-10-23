# `@epa-wg/cem-components`

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

Declarative component primitives that consume the CEM theme. No shadow DOM — every component renders in the
light DOM, authored against the `<cem-element>` substrate from `@epa-wg/cem-elements` (functional successor to
`@epa-wg/custom-element`; design home: [`docs/cem-element-design.md`](../../docs/cem-element-design.md)).

> **Primary architecture:** every component is a CEM-ML `<cem-element>`
> declaration at `src/components/<cem-tag>/<cem-tag>.xhtml`, whose
> `<template id="<cem-tag>" type="text/cem-ml">` remains addressable as
> `<declaration-url>#<cem-tag>`, with CEM-token CSS
> embedded as a static `<style>` node that `cem-elements` installs once per
> declaration, and Storybook cases/unit
> assertions in the colocated CSF Next `<cem-tag>.stories.ts` file's async
> `play` functions using `storybook/test`. Story `render` functions return HTML
> strings. The story is development-only and must not implement component
> behavior. Do not add standalone component CSS or migrated component selectors
> to global CSS. This package contains zero authored JavaScript/TypeScript
> except the required colocated `.stories.ts` modules. If CEM-ML cannot express a requirement concisely,
> component work hard-stops until the reusable capability is implemented in
> `cem-elements`. See the normative
> [declarative UI principle](../../docs/declarative-ui-principle.md).

> **Migration complete:** all 54 components use canonical XHTML and colocated
> stories. The package contains no legacy registry, installer, behavior module,
> separate TypeScript suite or package-owned tooling. The closed zero inventory
> is enforced by [`declarative-migration.json`](./declarative-migration.json).

The first proven implementation of this contract is
[`cem-select.xhtml`](./src/components/cem-select/cem-select.xhtml) with
[`cem-select.stories.ts`](./src/components/cem-select/cem-select.stories.ts).
Its choice/form/keyboard behavior is a reusable `cem-elements` capability; the
component owns no JavaScript and its CSS is embedded in the XHTML declaration.
That CSS is absent from instance DOM. The accepted native `@scope` target uses
the tag as its private root, declaration-owned `scope="name"` for public shared
membership, `slot="name"` for projected roots, and `part="name"` for stable
component internals. Data-island and render identity are not styling APIs. See
the normative
[CEM light-DOM CSS scoping rules](../../docs/cem-ml-uid-and-scoped-css-design.md)
for ownership, scope resolution, specificity, lifecycle, and diagnostics.
That target is not current runtime behavior until its active migration gate is
complete.

The shared [`cem-source-editor` code editor](./playgrounds/cem-source-editor-gallery.html)
has a [property playground](./playgrounds/cem-source-editor.html) and is used by
the [`cem-demo-element` playground](../cem-demo-element/demo/playground.html).
Browse the [components gallery](./playgrounds/index.html) for related controls.

## Install

```bash
yarn add @epa-wg/cem-components
```

This package depends on `@epa-wg/cem-theme` and `@epa-wg/cem-elements`; install them alongside.

## Public component catalog

`@epa-wg/cem-components/catalog/cem.components.catalog.json` is the public beta
structured catalog for documentation and other data consumers. It contains all
verified component semantics, token families, category state coverage, and
source links to separately owned examples and local Storybook configuration.
It contains no executable declaration or example markup and is not a runtime
component bundle.

## Declaration loading

Load an individual `@epa-wg/cem-components/components/<tag>` resource, or
`@epa-wg/cem-components/components.xhtml#<tag>` from the generated bundle,
through the shared `cem-elements` loader. The package has no JavaScript entry
point. Runtime APIs and event-detail types come from `@epa-wg/cem-elements`.
Page-level import maps resolve these public resource subpaths; the shipped
property pages demonstrate both source and installed-package loading.

**Capability caveat:** imports inherit the selected template's intrinsic
capability or its immediate owning declaration's metadata; consumers normally
need only `tag` and `src`. Preserve that metadata when copying a template.
An explicit importer value overrides it, and `capability=""` disables inheritance,
which can remove required runtime behavior. See the
[component-specific capability names](docs/conventions.md#41-intrinsic-capability-metadata)
and the [form demo](../cem-elements/demo/form.html).

```html
<cem-element tag="cem-badge" src="./components.xhtml#cem-badge"></cem-element>
<cem-badge label="Ready" tone="success"></cem-badge>
```

The final passive and inert payload owners are `cem-option`, `cem-option-group`,
`cem-badge`, `cem-avatar`, `cem-media-preview`, `cem-tree-item`, `cem-app-bar`,
`cem-tab` and `cem-step`. See their [contract and CEM audit](docs/passive-payload-contract.md).

The foundational layout/content owners also load directly from canonical XHTML,
without a capability or component JavaScript:

| Component | Property playground |
| --- | --- |
| `cem-surface` | [Surface](playgrounds/cem-surface.html) |
| `cem-text` | [Text](playgrounds/cem-text.html) |
| `cem-stack` | [Stack](playgrounds/cem-stack.html) |
| `cem-grid` | [Grid](playgrounds/cem-grid.html) |
| `cem-divider` | [Divider](playgrounds/cem-divider.html) |
| `cem-card` | [Card](playgrounds/cem-card.html) |

Each page links to a five-theme gallery. Their `part="root"` owners remain stable
through attribute changes; cards also expose `part="header"` and `part="body"`.
Busy, empty and decorative are presence attributes, including `="false"`.
The surface/card state markers preserve native focus and edited content. Stack
and grid remain passive formatting containers. See the
[layout/content contract](docs/layout-content-contract.md) and
[divider contract](docs/divider-contract.md).

The navigation/content owners now load from canonical XHTML declarations:

| Component | Shared capability | Property playground |
| --- | --- | --- |
| `cem-nav` | `navigation-landmark` | [Navigation](playgrounds/cem-nav.html) |
| `cem-tabs` | `tab-panels` | [Tabs](playgrounds/cem-tabs.html) |
| `cem-stepper` | `workflow-panels` | [Stepper](playgrounds/cem-stepper.html) |
| `cem-tree` | `tree-view` | [Tree](playgrounds/cem-tree.html) |
| `cem-list` | `native-list` | [List](playgrounds/cem-list.html) |
| `cem-chip` | None; native button and declarative slice bindings | [Chip](playgrounds/cem-chip.html) |

Each declaration owns its scoped token CSS and colocated executable stories.
The hosts are not form-associated. Disclosure and chip toggles retain their
native button identity; presence-only attributes accept empty and false-valued
spellings. Tabs/stepper retain panel controls and recover focus when the focused
panel is hidden. Tree collapse recovers to a visible ancestor without stealing
focus from outside. `CemTabDetail`, `CemStepDetail`, `CemTreeToggleDetail` and
`CemTreeActivateDetail` remain public type exports.

Load canonical `cem-timepicker` with `capability="time-input"` and `cem-slider`
with `capability="range-input"`. Both keep their authored inputs as native value,
event and form owners; the host is not form-associated. The property pages and
five-theme galleries are [timepicker](playgrounds/cem-timepicker.html) and
[slider](playgrounds/cem-slider.html). Slider accepts named `single`, `start` and
`end` slots as well as the existing `data-cem-slider-thumb` vocabulary.

Load canonical `cem-datepicker` with `capability="calendar-input"`. Its authored
native text input owns the canonical Gregorian date, labels, validation, events,
forms and reset. The shared capability supplies bounded calendar navigation and
a silent modal draft/Apply lifecycle; the declaration owns scoped token CSS.
See the [datepicker contract](docs/datepicker-contract.md),
[property playground](playgrounds/cem-datepicker.html) and
[five-theme gallery](playgrounds/cem-datepicker-gallery.html).

Load canonical `cem-autocomplete` with `capability="editable-choice"`. It keeps
native text input focus, authoritative option payloads and one form-associated
submitted value, including the distinction between option identity and label.
See the [autocomplete contract](docs/autocomplete-contract.md),
[property playground](playgrounds/cem-autocomplete.html) and
[gallery](playgrounds/cem-autocomplete-gallery.html).

Load canonical `cem-checkbox`, `cem-radio` and `cem-switch` declarations
explicitly. Their native inputs own checked state, radio groups, forms and
reset; checkbox adds the shared `checkable-control` capability for mixed state.
See the [checkable contract](docs/checkable-contract.md), linked property
playgrounds and five-theme galleries for names, presence flags and native keys.

## Theme and compatibility stylesheet

Component CSS is installed once from each canonical declaration. The public
`styles.css` export remains as an empty comment-only compatibility asset.

Load generated theme CSS first, then explicitly load the component bindings:

```ts
import '@epa-wg/cem-theme/styles.css';
import '@epa-wg/cem-components/styles.css';
```

The JavaScript entry does not import or inject CSS. The public stylesheet is
built byte-for-byte from `src/styles.css` and published only as
`dist/styles.css` through the `./styles.css` package export. It binds enabled
`cem-action` buttons to primary default/hover/active tokens and enabled
`cem-icon-button`/`cem-menu-item` buttons to contextual default/hover/active
tokens.

Input primitives use the shared CEM indicator stack. Field-like controls
default to an underline and binary controls to a whole-label outline; authors
can select `indicator="underline|outline"`. Advanced custom elements may set
`--cem-input-indicator-appearance` to one of the generated
`--cem-indicator-appearance-*` tokens. All stripe color and geometry values
remain theme-owned.

`cem-autocomplete` adds a form-associated editable combobox while keeping its
native text input as the only focus and text-entry owner. Its transient listbox
uses `aria-activedescendant`, canonical `cem-option`/`cem-option-group` payloads,
tokenized hover/selected/active/disabled paint, and a focused forced-colors
gate. Application rendering owns filtering and option replacement.

`cem-timepicker` adds a canonical `HH:mm` time-of-day picker while one direct
authored native text input retains accessible-name, value, validation, event,
and form ownership. It generates interval choices or consumes direct
`cem-option` labels/values, supports an optional native toggle, and keeps focus
on the input during listbox navigation. The Popover top layer and CSS Anchor
Positioning provide overlay placement without numeric z-index; existing input,
contextual-action, and select-option tokens cover normal and forced colors.

`cem-datepicker` adds a bounded single-date `YYYY-MM-DD` capability while one
direct authored native text input retains accessible-name, value, validation,
event, reset, and form ownership. Its optional native toggle opens a modal
native dialog with a localized six-week calendar grid, roving day focus, and an
explicit draft/Apply boundary. The dialog uses the top layer and CSS Anchor
Positioning; theme-owned input, contextual-action, content-interaction, and
current-indicator semantics keep hover, selected, today/current, disabled, and
focus-visible states independent in normal and forced colors.

Navigation links, disclosure buttons, and tabs consume the generated
`--cem-navigation-item-*` state family. Hover styling is applied to those native
owners rather than their nav/content/tablist wrappers, preserves current and
selected semantics, and suppresses enabled treatment for disabled owners.
Keyboard focus uses the existing D5 focus width/offset and zebra focus color on
the same native owners without changing geometry or replacing hover/current
paint. Held native activation uses distinct navigation active/current-active
pairs; the disclosure delegates its release-time toggle to the existing
expanded contract. Native-disabled buttons retain browser suppression.
`cem-tabs` generates only native-disabled unavailable tab buttons under its
dedicated manual-activation contract. ARIA-disabled direct `cem-nav` owners
remain focusable while component-owned capture behavior blocks pointer,
programmatic, Enter, and native-button Space activation before target and
application bubble listeners.

`cem-stepper` is a workflow-navigation owner rather than a tab substitute. It
consumes strict inert `cem-step` payloads, generates an ordered set of exact
native header buttons and persistent linked regions, and exposes silent
`selected-index` control plus one serializable `cem-step` activation event.
Horizontal/vertical roving focus, linear eligibility, optional steps, editable
return, application-authored completion/invalid facts, and native/ARIA-disabled
suppression are covered without scanning panel controls. Header interaction uses
navigation state tokens; canonical workflow status/connector endpoints keep
completion and error independent in normal and forced colors. No CSS exception
or component animation is required.

`cem-tree` is a generic expandable hierarchy rather than navigation or an
application data source. Strict recursive `cem-tree-item` payloads are inert;
the tree generates exact native button treeitems, stable owned groups, explicit
hierarchy metadata, roving visible-node focus, typeahead, and component-owned
expansion. Loading and optional selection remain application-authored, while
parent toggles and leaf activations emit separate serializable events. Generic
content-interaction tokens cover exact-owner normal states and system colors
cover forced colors without wrapper paint, component animation, or a CSS
exception.

`cem-divider` owns a semantic or decorative separator track rather than a bare line. The D0 separator color is
derived from surface text at reduced salience, D5 supplies the hairline, D1 supplies relationship spacing and inset,
and D2 floors the complete line-plus-margins track at the coupling guard. Horizontal, vertical, inset, and decorative
forms remain non-focusable and event-neutral; forced colors restore the line to `CanvasText`.

`cem-accordion` groups independent disclosure panels and can share the available height among multiple open sections. It preserves panel DOM owners while each `cem-expansion` owns disclosure and focus. See the [accordion contract](docs/accordion-contract.md), [property playground](playgrounds/cem-accordion.html) and [gallery](playgrounds/cem-accordion-gallery.html).

`cem-data-view` presents bounded native typed data in a canonical native HTML table, retaining structural output for unsupported shapes. It complements the passive [cem-table projection contract](docs/disclosure-data-contract.md) with caption and scoped native column headers. It shares the native import boundary and never parses JSON/CSV in JavaScript. See its [property playground](playgrounds/cem-data-view.html) and [gallery](playgrounds/cem-data-view-gallery.html).

`cem-expansion` owns one independent disclosure panel rather than reusing navigation-specific `cem-nav`. Its native
header button is the sole hover/focus/active/disabled owner, the live `expanded` host attribute controls a persistent
ARIA-linked panel, and the default slot remains instantiated while collapsed. Contextual action and existing
palette/spacing/control/shape/focus tokens cover the full visual contract, including forced colors, without a CSS
exception.

`cem-progress-spinner` is the circular complement to linear `cem-progress`.
Presence of `value` selects determinate semantics; absence selects
indeterminate semantics, with normalized range values projected only to ARIA.
The persistent SVG consumes generated D0 progress colors and D2c geometry;
indeterminate motion consumes the D7 continuous-cycle/uniform pair and stops
under reduced motion while leaving a static arc. Forced colors use the generated
`GrayText`/`Highlight` mapping. The component has no focus, activation, disabled,
selection, current, or live-region behavior, and requires no CSS exception.

`cem-sort-header` composes with passive `cem-table` as a sortable-column action.
Its direct native button owns pointer, keyboard, focus, active, and disabled
interaction; the generated column header conditionally owns `aria-sort`.
Activation cycles none, ascending, descending, and none, clears an active peer
only in the nearest table, and emits serializable `cem-sort` detail. Applications
retain row ordering and localized polite announcement ownership. Existing
contextual-action and D1/D2/D2c/D3/D5/D6 semantics cover normal and forced-color
paint without a CSS exception.

`cem-paginator` owns a labeled paged-content navigation landmark while leaving
data loading and item/row rendering to the application. Its native select and
button owners expose a zero-based normalized page model, optional page-size and
first/last controls, first-visible-item preservation, localizable labels, one
serializable `cem-page` request event, focus-stable boundary suppression, and
an atomic polite range status. Existing contextual-action, select/indicator,
palette, D1/D2/D2c/D3/D5/D6 semantics cover normal and forced-color styling, so
no CSS exception is needed.

`cem-slider` owns one horizontal single-value or range visual while its authored
native range inputs remain the exact pointer, keyboard, focus, value, event,
accessible-name, and form owners. Parent `min`/`max`/`step`/`disabled` state is
normalized and projected without replacing the inputs; range thumbs cannot
cross; `discrete` labels and `show-tick-marks` remain hidden visual output.
Canonical D0 slider paint, D2c track/thumb geometry, D2 targets, and D5 focus
cover normal and forced colors without a component CSS exception.

`cem-dialog`, `cem-dialog-shell` and `cem-tooltip` are canonical declarations
using shared `native-surface`. Dialogs default to persistent nonmodal tasks,
with explicit modal mode, native forms, commands, heading naming and retained
or disposed bodies. Tooltips preserve native triggers and persistent descriptions
with independent focus/hover interest. See the [dialog](docs/dialog-contract.md)
and [tooltip](docs/tooltip-contract.md) contracts for migration from legacy APIs.
`verify-surfaces` checks source/installed galleries, property pages, release
bundle, keyboard behavior and forced colors.

`cem-fieldset` groups related controls through a native fieldset and legend,
with projected labels/help, stable description references and presence-only
group disabling. Its XHTML declaration needs no JavaScript capability and leaves
values, validation and form ownership with the nested controls. See the
[fieldset contract](docs/fieldset-contract.md), [property playground](playgrounds/cem-fieldset.html)
and [gallery](playgrounds/cem-fieldset-gallery.html). The gallery integration gate
accepts `--fieldset-only` for source and isolated-package form, theme and bundle checks.

## Build & Verify

```bash
yarn nx run @epa-wg/cem-components:verify
yarn nx run @epa-wg/cem-components:verify-declarative
yarn nx run @epa-wg/cem-components:verify-passive
yarn nx run @epa-wg/cem-components:verify-native-substrate
yarn nx run @epa-wg/cem-components:verify-figma-inventory
yarn nx run @epa-wg/cem-components:verify-material-parity
yarn nx run @epa-wg/cem-components:verify-state-matrix
yarn nx run @epa-wg/cem-components:verify-style-contract
yarn nx run @epa-wg/cem-components:verify-input-indicator-forced-colors
yarn nx run @epa-wg/cem-components:verify-autocomplete-forced-colors
yarn nx run @epa-wg/cem-components:verify-navigation-hover-forced-colors
yarn nx run @epa-wg/cem-components:verify-content-hover-forced-colors
yarn nx run @epa-wg/cem-components:verify-divider-forced-colors
yarn nx run @epa-wg/cem-components:verify-expansion-forced-colors
yarn nx run @epa-wg/cem-components:verify-progress-spinner-forced-colors
yarn nx run @epa-wg/cem-components:verify-sort-header-forced-colors
yarn nx run @epa-wg/cem-components:verify-paginator-forced-colors
yarn nx run @epa-wg/cem-components:verify-slider-forced-colors
yarn nx run @epa-wg/cem-components:verify-tooltip-forced-colors
yarn nx run @epa-wg/cem-components:verify-timepicker-forced-colors
yarn nx run @epa-wg/cem-components:verify-datepicker
yarn nx run @epa-wg/cem-components:verify-time-range
yarn nx run @epa-wg/cem-components:verify-navigation
yarn nx run @epa-wg/cem-components:verify-datepicker-forced-colors
yarn nx run @epa-wg/cem-components:verify-stepper-forced-colors
yarn nx run @epa-wg/cem-components:verify-tree-forced-colors
yarn nx run @epa-wg/cem-components:verify-package
yarn nx run @epa-wg/cem-components:test
yarn nx run @epa-wg/cem-components:build
yarn nx run @epa-wg/cem-components:build:styles
yarn nx run @epa-wg/cem-components:build:catalog
yarn nx run @epa-wg/cem-components:lint
```

`yarn build` at the repo root builds every package, including this one.

`verify-declarative` runs first and enforces the primary architecture: new
JavaScript/TypeScript outside the required story contract is rejected, and
every migrated/new component must use its own XHTML folder with embedded,
once-per-declaration low-specificity CEM-token styles and colocated CSF Next Storybook `play`
tests. Reusable behavior lives in `cem-elements`; there are no legacy exceptions. The gate
also rejects a story that does not import its own raw declaration,
use the shared lazy `cem-elements` loader, return HTML from `render`, or import
test helpers from `storybook/test`.

`yarn nx run @epa-wg/cem-components:verify` runs the architecture,
Figma component inventory, state-matrix audit, token-only style contract,
package publication and browser delivery gates, then runs the full Chromium story cohort after those gates finish. The Figma inventory accounts for every public
primitive, validates its component/payload/structural classification, public
properties, executable states, token families, docs, and review locator, and
depends on the native five-mode theme token gate. The state-matrix audit keeps
every category/state requirement classified as browser-covered, static-only, or a gap and rejects stale test and
assertion references. Component-specific evidence lets a newly promoted owner
join a covered category state without displacing the browser evidence for its
existing owners. Intentional gaps remain visible in the generated JSON/Markdown
reports so the audit can select the next fixture without claiming that it
already exists. The style contract depends on `@epa-wg/cem-theme:build:tokens`,
and `@epa-wg/cem-theme:verify-package`, so the component gate checks current
generated tokens and the public theme stylesheet export.
The style gate reads declaration template text through the native XML/XPath
boundary and collects embedded CSS through the native CEM-ML compiler. It handles
XML entities, CDATA, multiple static style blocks and retained CSS nesting before
checking the existing semantic token and state contracts. The target builds the
shared CLI/compiler dependencies and runs regression fixtures, including rejected
state/token/geometry mutations. This is a static source audit; rendered theme,
inheritance and forced-color checks remain separate browser gates.
The package verifier proves source/built CSS byte identity, the side-effect-free
asset-only boundary, all 54 canonical declarations, zero packaged JavaScript/TypeScript, and exact dry-run npm inclusion of one `dist/styles.css`.

`yarn nx run @epa-wg/cem-components:test` runs every colocated component play through the shared Chromium Storybook runner. Run this cohort separately from the package browser gates.

## Fixture Surfaces

| Surface | Path |
| ------- | ---- |
| Figma component inventory | `../../examples/figma/component-library.json` |
| Figma component review fixture | `../../examples/figma/component-library-fixture.md` |
| Figma component inventory gate | `tools/scripts/verify-cem-components-figma-inventory.mjs` |
| Angular Material parity inventory | `tests/angular-material-parity.json` |
| Angular Material parity gate | `tools/scripts/verify-cem-components-material-parity.mjs` |
| State-matrix inventory | `tests/state-matrix-coverage.json` |
| State-matrix audit gate | `tools/scripts/verify-cem-components-state-matrix.mjs` |
| Token-only style gate | `tools/scripts/verify-cem-components-styles.mjs` |
| Input indicator forced-colors gate | `../../tools/cem-components/verify-input-indicator-forced-colors.mjs` |
| Autocomplete forced-colors gate | `../../tools/cem-components/verify-autocomplete-forced-colors.mjs` |
| Navigation hover/focus/active/disabled forced-colors gate | `../../tools/cem-components/verify-navigation-hover-forced-colors.mjs` |
| Content hover/focus forced-colors gate | `../../tools/cem-components/verify-content-hover-forced-colors.mjs` |
| Divider forced-colors gate | `../../tools/cem-components/verify-divider-forced-colors.mjs` |
| Expansion forced-colors gate | `../../tools/cem-components/verify-expansion-forced-colors.mjs` |
| Progress-spinner forced-colors/reduced-motion gate | `../../tools/cem-components/verify-progress-spinner-forced-colors.mjs` |
| Sort-header forced-colors gate | `../../tools/cem-components/verify-sort-header-forced-colors.mjs` |
| Paginator forced-colors gate | `../../tools/cem-components/verify-paginator-forced-colors.mjs` |
| Slider forced-colors gate | `../../tools/cem-components/verify-slider-forced-colors.mjs` |
| Tooltip forced-colors gate | `../../tools/scripts/verify-cem-component-playgrounds.mjs --surfaces-only` |
| Timepicker forced-colors gate | `../../tools/cem-components/verify-timepicker-forced-colors.mjs` |
| Datepicker forced-colors gate | `../../tools/cem-components/verify-datepicker-forced-colors.mjs` |
| Stepper forced-colors gate | `../../tools/cem-components/verify-stepper-forced-colors.mjs` |
| Tree forced-colors gate | `../../tools/cem-components/verify-tree-forced-colors.mjs` |
| Package publication gate | `../../tools/cem-components/verify-package.mjs` |
| Stylesheet copy | `../../tools/cem-components/copy-styles.mjs` |
| Autocomplete browser coverage | `src/components/cem-autocomplete/cem-autocomplete.stories.ts` |
| Expansion browser coverage | `src/components/cem-expansion/cem-expansion.stories.ts` |
| Progress-spinner browser coverage | `src/components/cem-progress-spinner/cem-progress-spinner.stories.ts` |
| Sort-header browser coverage | `src/components/cem-sort-header/cem-sort-header.stories.ts` |
| Paginator browser coverage | `src/components/cem-paginator/cem-paginator.stories.ts` |
| Slider browser coverage | `src/components/cem-slider/cem-slider.stories.ts` |
| Stepper browser coverage | `src/components/cem-stepper/cem-stepper.stories.ts` |
| Stepper declarative contract fixture | `tests/stepper/contract.html` |
| Tree browser coverage | `src/components/cem-tree/cem-tree.stories.ts` |
| Tree declarative contract fixture | `tests/tree/contract.html` |
| Tooltip browser coverage | `src/components/cem-tooltip/cem-tooltip.stories.ts` |
| Timepicker browser coverage | `src/components/cem-timepicker/cem-timepicker.stories.ts` |
| Datepicker browser coverage | `src/components/cem-datepicker/cem-datepicker.stories.ts` |
| State and ARIA coverage | `src/components/<cem-tag>/<cem-tag>.stories.ts` |
| Workflow coverage | Colocated `cem-card`, `cem-avatar`, `cem-app-bar` and `cem-sheet` stories |
| Workflow fixtures | `tests/workflows/` |
| Autocomplete contract fixture | `tests/autocomplete/contract.html` |
| Expansion contract fixture | `tests/expansion/contract.html` |
| Progress-spinner contract fixture | `tests/progress-spinner/contract.html` |
| Sort-header contract fixture | `tests/sort-header/contract.html` |
| Paginator contract fixture | `tests/paginator/contract.html` |
| Slider contract fixture | `tests/slider/contract.html` |
| Tooltip contract fixture | `playgrounds/cem-tooltip-gallery.html` |
| Timepicker contract fixture | `tests/timepicker/contract.html` |
| Datepicker contract fixture | `tests/datepicker/contract.html` |
| Package examples | `examples/` |

## Handoff Condition

The XHTML declaration loader and CSF Next test pattern are proven by
`cem-select`. Every next component must follow that same folder, embedded-style,
raw-declaration loader, HTML-string `render`, and async `play` contract. If its
behavior cannot be expressed concisely, migration is hard-stopped until the
missing reusable capability exists in `cem-elements` and its tests pass.

Known deferrals stay outside this trigger:

- Phase 3.5 Edge/SSR processing fixtures for serialized `DataIslandSnapshot` handoff.
- Phase 3.6 `@epa-wg/custom-element` monorepo adoption.
- Richer post-MVP controls such as split actions, date ranges, date/time-zone integration, date adapters, side-nav variants, breadcrumbs,
  and richer menu/dropdown families.
- Full application behaviors around dialog focus trapping, routed navigation, async loading, and data fetching.

## Key paths

| Purpose | Path |
| ------- | ---- |
| Normative architecture | `../../docs/declarative-ui-principle.md` |
| Declarative component source | `src/components/<cem-tag>/<cem-tag>.xhtml` |
| Colocated Storybook/unit source | `src/components/<cem-tag>/<cem-tag>.stories.ts` |
| Proven declarative example | `src/components/cem-select/` |
| Migration baseline | `declarative-migration.json` |
| Declarative architecture gate | `../../tools/scripts/verify-cem-components-declarative.mjs` |
| Public stylesheet source | `src/styles.css` |
| Public component catalog source | `../../tools/cem-components/build-component-catalog.mjs` |
| Public component catalog output | `dist/catalog/cem.components.catalog.json` |
| State-matrix catalog evidence | `dist/reports/component-state-matrix.{json,md}` |
| Colocated state and ARIA browser coverage | `src/components/<cem-tag>/<cem-tag>.stories.ts` |
| Workflow coverage | Colocated `cem-card`, `cem-avatar`, `cem-app-bar` and `cem-sheet` stories |
| Workflow fixtures | `tests/workflows/` |
| Component reference | `docs/component-reference.md` |
| Package-local examples | `examples/` |
| Built output | `dist/` |

## Component Docs

- [Component reference](./docs/component-reference.md) — MVP component semantics, token families, states, and
  accessibility notes.
- [Angular Material parity inventory](./docs/angular-material-parity.md) — pinned catalog mappings, gaps, and accepted
  implementation sequencing.
- [Divider contract](./docs/divider-contract.md) — line-plus-margins geometry, semantic/decorative ownership, tokens,
  and forced-colors behavior.
- [Expansion contract](./docs/expansion-contract.md) — independent disclosure ownership, live state, native events,
  ARIA references, token audit, focused fixture, and forced-colors boundary.
- [Progress spinner contract](./docs/progress-spinner-contract.md) — circular mode/value semantics, stable SVG
  geometry, D0/D2c/D7 token audit, reduced motion, forced colors, and event-neutral fixture matrix.
- [Sort header contract](./docs/sort-header-contract.md) — sortable-column ownership, fixed direction cycle,
  native events, application data/announcement boundary, theme audit, and forced-colors matrix.
- [Paginator contract](./docs/paginator-contract.md) — normalized page/range ownership, native controls, request
  events, application data boundary, token audit, and normal/forced-colors assertion matrix.
- [Autocomplete contract](./docs/autocomplete-contract.md) — accepted editable-combobox owner, form/events,
  keyboard/accessibility, token audit, focused fixture, forced-colors boundary, and assertion matrix.
- [Timepicker contract](./docs/timepicker-contract.md) — direct native text-input ownership, canonical `HH:mm`,
  interval/custom choices, listbox interaction, validation, token audit, and top-layer forced-colors boundary.
- [Datepicker contract](./docs/datepicker-contract.md) — direct native text-input ownership, canonical `YYYY-MM-DD`,
  localized modal calendar interaction, explicit confirmation, token audit, and top-layer forced-colors boundary.
- [Stepper contract](./docs/stepper-contract.md) — inert step payloads, native workflow headers, persistent panels,
  linear/optional/editable rules, application-owned completion/validation, token audit, and forced-colors boundary.
- [Tree contract](./docs/tree-contract.md) — inert recursive payloads, native treeitems, stable hierarchy IDs,
  roving focus, application-owned selection/loading, token audit, and forced-colors boundary.
- [Conventions](./docs/conventions.md) — naming, attributes, events, form participation, validation, loading states,
  progressive enhancement.
- [Light-DOM rendering rules](./docs/light-dom-rendering.md) — `@epa-wg/custom-element` compatibility, no shadow DOM,
  inert data islands, declarative slot projection, host-attribute forwarding, render lifecycle.
- [Accessibility contract](./docs/accessibility.md) — accessible names, ARIA wiring, focus, keyboard patterns, live
  regions; mirrors the Tier A semantic-validation catalog enforced by `cem_ml`.
- [Selectable list contract](./docs/selectable-list-contract.md) — accepted Phase 4 single-select listbox ownership,
  declarative option payload, native interaction boundary, and executable acceptance criteria.
- [Field controls](./docs/field-controls-contract.md) — canonical field declarations, form ownership and reset.
- [Input loading contract](./docs/input-loading-contract.md) — explicit presence-only busy projection, native state
  and ARIA markers, tokenized pending indicator, interaction boundaries, and executable acceptance criteria.
- [Content hover contract](./docs/content-hover-contract.md) — actual native content owners, passive exclusions,
  checked/selected coexistence, disabled suppression, and forced-colors mapping.
- [Content focus-visible contract](./docs/content-focus-visible-contract.md) — native keyboard order, tokenized
  external rings, state/hover coexistence, disabled skipping, and forced-colors mapping.
- [Navigation hover contract](./docs/navigation-hover-contract.md) — owner-only pointer paint, current/selected
  coexistence, disabled suppression, forced-colors mapping, and executable acceptance criteria.
- [Navigation focus-visible contract](./docs/navigation-focus-visible-contract.md) — keyboard order, native-disabled
  skipping, tokenized external rings, state coexistence, restoration, and forced-colors mapping.
- [Navigation active contract](./docs/navigation-active-contract.md) — trusted pointer and native keyboard holds,
  current/selected coexistence, disclosure release ownership, disabled suppression, and forced-colors mapping.
- [Navigation disabled contract](./docs/navigation-disabled-contract.md) — native versus ARIA-disabled focus policy,
  capture-phase activation suppression, form neutrality, state coexistence, and forced-colors mapping.
- [Stylesheet publication contract](./docs/stylesheet-publication-contract.md) — single-source CSS build, package
  export, cache, release, and npm-pack boundary.
- [Component CSS exceptions](./docs/components-css-exceptions.md) — token-first review queue for proposed values that
  cannot yet be represented by `@epa-wg/cem-theme`.

## Related docs

- [CEM component MVP](../../docs/component-mvp.md) — first component list and state matrix.
- [CEM component examples](./examples/README.md) — package-local workflow examples, separate from executable tests.
- [CEM ML library plan](../../docs/cem-ml-library-plan.md) — the active parser/runtime path consumes/produces the
  declarative markup these components will render.
- [CEM ML acceptance criteria](../../docs/cem-ml-ac.md) — testable AC for the parser/transform stack.
- [Roadmap](../../roadmap.md) — Phase 3 (custom-element runtime) and Phase 4 (component set) define this package's
  delivery sequencing.
- [Repository documentation index](../../docs/index.md) — full project map.

## Source and development pages

The accepted [component development pattern](../../docs/component-development-pattern.md) keeps canonical XHTML
separate from a companion property playground and source view. A generated
`#ID` bundle is generated as release output. `cem-action`, `cem-select`,
`cem-theme-switch`, `cem-icon`, `cem-icon-button`, `cem-menu-item`, `cem-field`, and
`cem-text-field` and `cem-textarea` use the canonical XHTML and colocated story structure.

## Action property playground

Open [`playgrounds/cem-action.html`](playgrounds/cem-action.html) through an HTTP
server (including the IDE preview). Visible radio groups edit intent, bend,
size, type, disabled, pending and selected; a text field edits the label. Each group
uses native exclusive selection and arrow-key navigation. Hover, held pointer/Space
and keyboard focus use native browser states. The source-only panel fetches the
canonical XHTML; the full gallery link opens the component-owned
[attribute gallery](playgrounds/cem-action-gallery.html). It covers all 20
explicitly implemented attributes, including selected/selectable presence,
size profiles, disclosure/visibility and native form overrides.

`yarn nx run @epa-wg/cem-components:build` generates `dist/cem-action.html` and
`dist/cem-action-gallery.html` with package-relative module maps. Serve installed
packages over HTTP and open the first page. Public exports are
`@epa-wg/cem-components/playgrounds/cem-action.html` and
`@epa-wg/cem-components/playgrounds/cem-action-gallery.html`. The demo helper is
a package dependency; the legacy adapter package is not required. The gallery
is authored in this package and uses the same canonical action definition.

`yarn nx run @epa-wg/cem-components:verify-playgrounds` tests real interactions
and source display in the repository and in isolated npm package archives.
Use `yarn nx run @epa-wg/cem-components:verify-playgrounds --args=--icons-only`
for the icon and icon-button property playgrounds, their full galleries, and
the release bundle in both locations. This focused mode reuses the full gate's
checks; failures in unrelated component galleries remain part of the full gate.
Use `yarn nx run @epa-wg/cem-components:verify-playgrounds --args=--compact-studio-only`
for accordion and native data-view property interactions, the nine affected
component galleries, and the release bundle in both source and isolated npm
archives. The complete gate also runs the new property checks; requested
gallery tags must exist, and the package catalog must contain every canonical
declaration exactly once.

## Select property playground

Open [`playgrounds/cem-select.html`](playgrounds/cem-select.html) for one live
select, its canonical source and grouped/single/multiple examples. Shared CEM
controls edit label, placeholder, name, value, autocomplete,
indicator, row count, multiple, disabled, required, busy and invalid state.
Changes to selection remain native interactions; the last committed value is
announced below the preview. Form validity and submitted values remain owned by
the production select.

The build exports `dist/cem-select.html` as
`@epa-wg/cem-components/playgrounds/cem-select.html`. The playground verifier
checks properties, form values, retained host identity, examples, source-only
rendering and narrow layout in source and isolated-package previews.

## Navigation controls

`cem-nav` composes its disclosure from `cem-icon-button variant="navigation"`.
Use the same variant with `href` and `align="start"` for padded navigation links;
explicit `aria-current="page"` reaches the native anchor. The nav owns container
inset and wrapping gaps, while icon buttons own control padding and hit targets.
All navigation states retain their dedicated theme colors. Native links remain
supported. See the [navigation contract](docs/navigation-disclosure-contract.md)
and [icon button contract](docs/icon-button-contract.md).

## Playground page theme

All property playgrounds, full galleries, the playground index and release-bundle
page use the shared `cem-gallery-page` declaration. It renders navigation and a
production `cem-theme-switch`, restores the shared `cem-playground-theme` key,
and applies the effective `data-theme` to the whole page. Choices are Light,
Dark, Contrast light, Contrast dark and System. Theme changes preserve edited
preview properties and source displays, and persist across navigation/reloads.

```html
<cem-gallery-page heading="cem-icon: Property playground">
  <template>
    <header>…</header>
    <main>…</main>
  </template>
</cem-gallery-page>
```

Load `@epa-wg/cem-components/playgrounds/cem-gallery-page.xhtml#cem-gallery-page`
through the page's module map. Fixed example themes and example storage keys
remain independent from the page preference. See the
[shared page contract](docs/gallery-page-contract.md) and
[playground convention](docs/conventions.md#playground-theme-control).

The standalone theme-switch property playground remains exported at
`@epa-wg/cem-components/playgrounds/cem-theme-switch.html`.

## Combined XHTML release bundle

The package exports `@epa-wg/cem-components/components.xhtml` alongside individual
component definitions. Load a template by its unique ID:

```html
<cem-element tag="cem-action" src="./components.xhtml#cem-action"></cem-element>
<cem-element tag="cem-select" capability="choice-select"
             src="./components.xhtml#cem-select"></cem-element>
```

Resolve the document URL through your module map or package deployment. The
native CEM AST build derives the bundle from canonical XHTML and preserves the
original resource base with `xml:base`. It includes action, select and theme
switch. All 54 canonical components ship in this declaration bundle.

The [bundle playground](playgrounds/cem-bundle.html) is exported at
`@epa-wg/cem-components/playgrounds/cem-bundle.html`. Canonical source,
playgrounds, and [build inputs](build/README.md) remain in the package.
`yarn nx run @epa-wg/cem-components:build:bundle` regenerates the artifact;
`verify-playgrounds` checks source and isolated-package loading.

## Required component galleries

Every canonical component has a **Full examples and variation matrix** link
on its property playground. The gallery provides live examples with source and
side-by-side theme/state comparisons. Both pages ship under the package
`playgrounds/` exports. See the [demo protocol](../../docs/component-development-pattern.md#full-examples-and-variation-matrix-required).

The [icon-button contract](docs/icon-button-contract.md) includes legacy icon-link
navigation, image and font icons, slotted labels, and disabled links.

### Standalone icon

`cem-icon` now ships as a canonical XHTML declaration. Load it explicitly through
`components/cem-icon` or the release bundle; all registration uses the shared runtime loader. It supports the legacy source formats and size/direction
variants while preserving the `name` alias and decorative/labeled semantics.
See the [icon contract](./docs/icon-contract.md) and
[Full examples and variation matrix](./playgrounds/cem-icon-gallery.html).

### Multiline text entry

`cem-textarea` is a canonical XHTML component with the shared form-control
capability. Load its declaration explicitly or use the release bundle; it is
registered through the shared runtime loader. See the
[textarea contract](./docs/textarea-contract.md) and
[Full examples and variation matrix](./playgrounds/cem-textarea-gallery.html).

Disclosure/data owners now use canonical declarations and shared runtime capabilities.
See the [CEM principle audit](docs/disclosure-data-contract.md), their colocated
stories, and each component’s property playground and five-theme gallery.

The feedback/status owners now load from canonical XHTML. Toast and alert retain
polite/explicit urgent message semantics; skeleton payload is decorative and inert.
Linear progress retains its native owner, while circular progress uses the shared
`progress-status` preparation capability. See the [feedback/status contract](docs/feedback-status-contract.md)
and the five component property pages and galleries.
