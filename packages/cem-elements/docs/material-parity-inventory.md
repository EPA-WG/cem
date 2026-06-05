# Material parity inventory

Scoping inventory for the Phase 3.1 material-parity work
([`../../../docs/todo.md` §3.1](../../../docs/todo.md)). It catalogs every authoring feature the legacy
`@epa-wg/custom-element` material components use, and records the current `<cem-element>` runtime support
status for each so the parity-story work (todo line 118) and the `cem_ml_cli` fixture wiring (todo line 124)
can be scoped against real gaps rather than guesses.

- **Source:** `~/aWork/custom-element-dist/src/material/components/*.html` (read-only POC reference).
- **Components in scope (8):** action, autocomplete, badge, dropdown, icon, icon-link, input, menu.
- **Method:** static read of each component HTML; line citations are `file:line` against the POC sources.
- **Runtime baseline:** `<cem-element>` runtime slices A, B, C1, C2.1–C2.6, D, E, production gate, and the
  `custom-element-v0` bridge subset.
  See [`../../../docs/cem-element-design.md`](../../../docs/cem-element-design.md).

## Support legend

| Mark | Meaning                                                                                          |
| ---- | ----------------------------------------------------------------------------------------------- |
| ✅   | Supported — the runtime renders this faithfully today (slices A/B/C1/C2/D/E).                    |
| 🟡   | Partial — renders, but the parity semantics are degraded or only coincidentally correct.        |
| ❌   | Not yet — unimplemented; the construct renders wrong, renders inert, or is rejected.             |

## Component → feature usage

Each component declares one or more `<custom-element>` definitions and consumes sibling components by tag.
The "Page chrome" rows (`index.html#nav-head`, `html-demo-element`, FontAwesome/Material icon fonts) are demo
scaffolding, not parity surface, and are excluded from the support matrix below.

| Component   | Declares (tag)                         | Imports by `src`                                                              | Notable authoring features used                                                                                  |
| ----------- | -------------------------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| icon        | `cem-icon` (`src="#cem-icon"`)         | `icon-link.html#cem-icon-link`                                               | `choose`/`when` with XPath tests, `<attribute>` (image/size/direction), `{$image}` attr interpolation, `module-url` slice (demo) |
| icon-link   | `cem-icon-link` (`src="#cem-icon-link"`)| —                                                                            | `choose`/`when`, `<attribute>` href/icon/color, `<slot>`, two `module-url` slices                               |
| menu        | `cem-menu` (inline, `hidden`)          | `icon-link.html#cem-icon-link`                                              | `<attribute>` direction/justify, unnamed `<slot name="">`, scoped `<style>` (×6)                                 |
| badge       | `cem-badge` (`src="#cem-badge"`)       | `icon-link.html#cem-badge-link`, `icon-link.html#cem-icon-link`, `icon.html#cem-icon` | `<attribute>` (text/dot/color/align/invisible), `if test="/datadom/slice/…"`, `slice` checkbox, `<slot>`        |
| action      | `cem-action` (`src="#cem-action"`)     | `icon-link.html#cem-icon-link`, `icon.html#cem-icon`                        | `if` on slice state, `slice`/`slice-event`/`slice-value` instances, `class="{//bend}"` XPath attr, nested `cem-icon`, `<slot>` |
| dropdown    | `cem-dropdown` (inline, `hidden`)      | `icon-link.html#cem-icon-link`, `menu.html#cem-menu`                        | `<attribute>` open/label, named + unnamed `<slot>`, `slice` checkbox, nested `cem-menu`                          |
| input       | `cem-input` (inline, `hidden`) + demo tags | `icon-link.html#cem-icon-link`, `icon.html#cem-icon`                     | `<attribute select="…?? …">`, `if`/`hasBoolAttribute()` attr forwarding, `xhtml:input`, named `<slot>`s, `<slice>` decls with events, `<option>` payloads, scoped `<style>` (×6) |
| autocomplete| `cem-autocomplete` (inline, `hidden`)  | `icon-link.html#cem-icon-link`, `input.html#cem-input`, `menu.html#cem-menu`| `<attribute select>`, named `<slot name="input">`/`<slot name="menu">`, `<data>` + `<option>` payloads, nested `cem-input`/`cem-menu` |

Import dependency order (leaves → composites): `icon`, `icon-link`, `menu` → `badge`, `action`, `dropdown`,
`input` → `autocomplete`. Every component loads its dependencies through external/local `src` declaration
references.

## Feature → runtime support matrix

This is the core scoping output. Each row is a distinct authoring feature, where it appears, and whether the
current `<cem-element>` runtime renders it faithfully.

| #  | Feature                                                  | Used by (representative `file:line`)                                              | Status | Gap / note                                                                                                      |
| -- | -------------------------------------------------------- | -------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| 1  | Inline `<template>` DOM-parity declaration              | all components                                                                   | ✅     | Slice C1.                                                                                                        |
| 2  | `<attribute name>` + default text value                 | `icon.html:48`, `badge.html:48`                                                  | ✅     | Slice B/C1; `name` + default text consumed.                                                                     |
| 3a | Attribute interpolation `{$x}`                          | `icon-link.html:92` (`src="{$icon}"`), `icon.html:82` (`class="icon {$image}"`) | ✅     | Slice C1; `{$name}` resolves in attribute values.                                                               |
| 3b | Text-content interpolation                              | `icon.html:80` (`<b>{$image}</b>`)                                              | 🟡     | **Syntax divergence:** legacy uses `{$x}` in text; cem-elements C1 text interpolation expects `${$x}` and renders legacy `{$x}` literally. Record as a CEM-ML migration decision. |
| 4  | Whole-expression attr dropped on null/false             | (runtime behavior)                                                               | ✅     | Slice C1.                                                                                                        |
| 5  | `<slice name>` + `slice`/`slice-event`/`slice-value`    | `action.html:150`, `input.html:246`                                             | ✅     | Slice D. Material uses it on instances (action) and as `<slice>` decls (input).                                 |
| 6  | Attribute-change rerender / data-island invalidation    | (runtime behavior)                                                               | ✅     | Slice D.                                                                                                         |
| 7  | External `src="./file.html#id"` declaration loading      | `action.html:104`, `autocomplete.html:38`                                        | ✅     | Async external declaration loading parses `url#id`, imports the target template, and registers the produced tag; bare package specifiers require host `loadSrcDocument`. |
| 8  | Local `src="#id"` same-document declaration             | `action.html:103`, `icon.html:89`                                                | ✅     | Same-document `#id` targets resolve to a `<template>` target or first template child.                           |
| 9  | `hidden` declaration host attribute                     | `action.html:103`, `dropdown.html:74`                                            | 🟡     | Our declarations don't render regardless; no `hidden` cosmetic on the produced-tag model. Behaviorally moot.    |
| 10 | `<attribute select="//xpath">` dynamic propagation      | `autocomplete.html:53`, `input.html:95`                                          | 🟡     | Legacy XPath `select` is a migration decision; canonical CEM-ML uses cem-ql expressions over `datadom.*`.       |
| 11 | XPath data model `/datadom/…`, `//attributes`, `//selected` | `input.html:220`, `autocomplete.html:53`                                     | ✅     | Functional parity is exposed as structured `datadom` records (`attributes`, `dataset`, `payload`, `slots`, `slices`, etc.), not an XPath engine. |
| 12 | `??` coalescing operator in `select`                    | `autocomplete.html:53`, `input.html:95`                                          | ✅     | Supported by cem-ql coalescing expressions in canonical CEM-ML.                                                |
| 13 | `if` conditional construct                              | `badge.html:192`, `input.html:207`                                              | 🟡     | Supported through canonical CEM-ML/cem-ql (`if` and `cem:if`) with `datadom.*` expressions; legacy XPath spellings still need migration/lowering. |
| 14 | `choose`/`when`/`otherwise` conditional                 | `icon.html:79`, `icon-link.html:91`                                              | 🟡     | Supported through canonical CEM-ML/cem-ql (`choose`/`when`/`otherwise` and `cem:*`) with diagnostics for malformed branches; legacy XPath spellings still need migration/lowering. |
| 15 | `hasBoolAttribute()` helper in expressions              | `input.html:224-228`                                                            | ❌     | No expression-function support.                                                                                  |
| 16 | `class="{//bend}"` XPath in attribute value             | `action.html:148`                                                               | ❌     | Only `{$name}` resolves; `{//xpath}` is not evaluated.                                                           |
| 17 | Namespaced `xhtml:*` elements                           | `input.html:218`                                                                | 🟡     | `readTemplateSource` flattens the xhtml namespace, so `xhtml:input` renders as `<input>` — coincidental parity.  |
| 18 | Declarative `<slot>` / named slots                      | `icon.html:85`, `input.html:206`, `autocomplete.html:85`                         | ✅     | Named/default slots lower from serialized payload in the render plan before light-DOM materialization, including DOM and WASM paths. |
| 19 | Scoped `<style>` inside a template                      | `input.html` (×6), `menu.html` (×6)                                              | 🟡     | Emitted as a literal, page-global `<style>` into light DOM; no scoping/containment.                             |
| 20 | Nested custom elements in render output                 | `action.html:127` (`cem-icon` in `cem-action`)                                   | ✅     | Nested produced tags upgrade when their declarations are registered, including through local/external `src`.     |
| 21 | `<data>` / `<option>` instance payloads                 | `autocomplete.html:112`, `input.html:275`                                        | ✅     | Captured inert into the data island and serialized into `datadom.data.<value>` / `datadom.options.<value>` plus ordered arrays. |
| 22 | `module-url` resource slices                            | `icon.html:221`, `icon-link.html:119-120`                                        | ✅     | Inert rendered helpers resolve URL-like specifiers by default or bare package specifiers through host `resolveModuleUrl`, then expose values under `datadom.slices.<slice>`. |

## Readiness conclusions

**Hard blockers removed:** local/external `src` declaration loading and `module-url` resource slices now have runtime
support and Storybook material-parity coverage. Bare `@scope/pkg` specifiers still require host resolver hooks
(`loadSrcDocument` / `resolveModuleUrl`) because browser import-map or bundler policy is host-owned.

**Remaining material-parity caveats:** legacy XPath authoring and XSLT-only constructs are migration decisions; the
canonical path uses CEM-ML plus cem-ql functional selection over `datadom.*`. Scoped styles still render as page-global
light-DOM styles, and boolean helper functions such as `hasBoolAttribute()` are not reproduced directly.

**Production gate:** the Storybook parity set is green for the covered runtime behaviors. AC-N-1 first-paint
performance proof and end-to-end accessibility-contract assertions are both wired into the Phase 3.1 gate in
`docs/todo.md`.

**Recommended sequencing implied by this inventory:**

1. Decide whether scoped-style containment and `hasBoolAttribute()` compatibility remain bridge/adoption work or move
   into the browser-substrate production gate.

This inventory satisfies the todo §3.1 "Build a material parity inventory" item and feeds the parity-story
(line 118) and `cem_ml_cli` fixture-wiring (line 124) items.
