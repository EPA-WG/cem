# CEM Figma Token Workflow

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

The Figma workflow is a native Figma library generated from token artifacts. Markdown token specs remain the source of
truth; Figma changes must be converted into spec edits before they enter the build.

## Native Figma Library Setup

1. Run the token export pipeline so `packages/cem-theme/dist/lib/tokens/figma/` contains:
    - `cem-light.tokens.json`
    - `cem-dark.tokens.json`
    - `cem-contrast-light.tokens.json`
    - `cem-contrast-dark.tokens.json`
    - `cem-native.tokens.json`
    - `cem-figma-report.md`
2. In the `CEM UI Kit`, open or create one native Figma variable collection named `CEM Tokens`.
3. Use Figma's native **Import mode** action to import each generated DTCG file as its matching mode:
    - `Light`
    - `Dark`
    - `Contrast Light`
    - `Contrast Dark`
    - `Native`
4. No token plugin or synchronization connector is required. Keep the library read-only from generated artifacts and
   do not treat Figma edits or exported modes as source changes.
5. Before sharing the collection, check `cem-figma-report.md` for excluded tokens, concrete alias fallbacks, warnings,
   and validation errors.

`native` mode values are Chromium-computed browser-reference values. They are not iOS or Android system color
equivalents.

Run the offline release gate after regenerating token artifacts:

```bash
yarn nx run @epa-wg/cem-theme:test:figma
```

The target validates the five generated mode files, representative native alias bindings, source CSS token metadata for
Figma WEB code syntax, the zero-error Figma report, and the checked-in native-library and Foundations evidence. It does
not call the Figma REST API; API-backed live validation remains governed by the REST API sync policy below.

## Native Library Review Evidence

[`native-library-review.json`](./native-library-review.json) separates the last
historical live review, current generated expectations, and the pending live
refresh. [`native-library-review-fixture.md`](./native-library-review-fixture.md)
defines the manual promotion procedure and deliberate rejection cases.

Run the focused gate with:

```bash
yarn nx run @epa-wg/cem-theme:verify:figma-native-review
```

Refresh status: `started`. Starting refresh revision:
[`01 Tokens` in the canonical CEM UI Kit](https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit?node-id=2-24).
The 2026-08-23 read-only live review confirmed the `CEM Tokens` collection and
the exact `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, and `Native` mode
order before any import or canvas edit. The collection remains at the historical
230-variable `@epa-wg/cem-theme 0.0.8` state; the generated contract is 252
variables. The live import and result fields remain null, so this checkpoint
does not assert that the external file was refreshed. The gate emits
`packages/cem-theme/dist/reports/cem-figma-native-review.{json,md}`.

## Foundations Composite Inventory

[`foundations-library.json`](./foundations-library.json) is the executable
layering and motion checklist for `02 Foundations`. The associated
[`foundations-library-fixture.md`](./foundations-library-fixture.md) defines the
manual review procedure and deliberate rejection cases.

Run the focused gate with:

```bash
yarn nx run @epa-wg/cem-theme:verify:figma-foundations
```

The gate derives six Effect Style values, one explicit no-effect specimen, five
semantic layer aliases, and eight motion curves from canonical tokens. It
rejects raw composite values in the inventory and emits
`packages/cem-theme/dist/reports/cem-foundations-report.{json,md}`. The report is
preparation evidence only; planned entries remain unreviewed until the live
canvas revision is recorded.

## Component Library Inventory

[`component-library.json`](./component-library.json) is the executable checklist
for the `03 Components` page. It accounts for every public `cem-*` primitive,
classifies visual owners, inert payloads, and structural compositions, and binds
Figma properties/states to public component and token evidence. The associated
[`component-library-fixture.md`](./component-library-fixture.md) defines the
manual review procedure and deliberate rejection cases.

Run the component-owned gate with:

```bash
yarn nx run @epa-wg/cem-components:verify-figma-inventory
```

The gate also runs the native Figma token target, rejects inventory drift, and
emits review reports under `packages/cem-components/dist/reports/`. A planned
locator is not a claim that the canvas asset has been built; only a reviewed
entry may carry an external Figma revision or node URL.

## Sample Application

Use [sample-token-application.md](./sample-token-application.md) as the local fixture for applying imported variables
to a button and card in the CEM UI Kit. Replace it with screenshots after the native Figma library variables have been
validated in the real file.

## Generated Artifact Validation

Offline validation run: 2026-08-19.

- Generated variable count: 252 per mode.
- Variable types: 57 color, 96 dimension, 6 duration, 5 font family, 10 number, and 78 string.
- Modes: `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, and `Native` with identical token paths and types.
- Import values use current DTCG objects for sRGB colors, px dimensions, and
  second-based durations; numeric and string token values remain scalars.
- Report errors: 0.

This is the current repository import surface. The last manual native-library
review below recorded 230 variables, so the live `CEM UI Kit` must be refreshed
and reviewed before Phase 5 publication; offline generation alone does not claim
that external update.

Native Import mode does not accept composite DTCG shadow or easing-curve types.
Their source tokens remain excluded from the mode files and listed in
`cem-figma-report.md`. Use only the approved derived Effect Styles and motion
specimens from the Foundations inventory; never copy their CSS values onto the
canvas.

## Native Library Validation

Last manual validation run: 2026-04-30.

Validated CEM UI Kit file:
https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit?node-id=2-24&t=QQwTKeMg0v9dTQ10-1

Result:

- `CEM Tokens` collection exists.
- Modes: `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, `Native`.
- Variable count: 230.
- Variable types: 42 color, 100 float, 88 string.
- Missing mode values: 0.
- Variable aliases present: 255 mode values.
- `01 Tokens` includes the `CEM Generator Token Demos` frame.

Screenshot:

![CEM Generator Token Demos](./screenshots/cem-generator-token-demos.png)

## Visual Parity Check

Validation run: 2026-04-30.

Web reference:

- Source: `packages/cem-theme/src/lib/css-generators/index.html`
- Screenshot: [cem-css-generators-index-web.png](./screenshots/cem-css-generators-index-web.png)
- Extracted rows: [cem-css-generators-index-web.json](./screenshots/cem-css-generators-index-web.json)

Figma reference:

- File: https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit?node-id=2-24&t=QQwTKeMg0v9dTQ10-1
- Frame: `CEM Generator Token Demos`
- Screenshot: [cem-generator-token-demos.png](./screenshots/cem-generator-token-demos.png)

Result:

- The web generator index exposes 10 generator categories: Breakpoints, Colors, Controls, Coupling, Dimension, Shape,
  Stroke, Layering, Voice / Fonts / Typography, and Timing.
- The Figma demo includes those same 10 categories plus a `Generator Index` section that documents the category source.
- The Figma category sections include the corresponding unpkg CDN source links in the section titles.
- Visual parity is category-level and token-demo-level, not pixel-level. The web page is a tabular generator index; the
  Figma page is a native design-library demo surface bound to CEM variables where Figma supports binding.

## REST API Sync Policy

Do not enable Figma REST API write/sync in local builds or CI until file import is stable and token governance exists.
If REST sync is added later, it must start as a manual script with:

- explicit Figma file id configuration
- scoped write token
- dry-run/report mode
- no default local build execution

CI REST sync may only be considered after manual sync is proven. It must be gated behind protected branch or release
workflows, required approval, secret-scoped tokens, generated diff/report artifacts, and rollback instructions.

## Developer Prompt: Refresh Native Figma Variables

Use this prompt when refreshing the native Figma Variables from generated files:

```text
Update the CEM UI Kit native Figma Variables through Figma's native Import mode action.
Keep one CEM Tokens collection. Use the generated DTCG files in dist/lib/tokens/figma/ as the only Figma input:
cem-light.tokens.json, cem-dark.tokens.json, cem-contrast-light.tokens.json, cem-contrast-dark.tokens.json,
and cem-native.tokens.json. Preserve read-only governance: Figma changes must become markdown spec edits, not
write-backs. Record the starting and reviewed evidence in examples/figma/native-library-review.json. Update
docs/todo.md, packages/cem-theme/docs/token-export.md, and examples/figma/README.md.
```

## Developer Prompt: Split Figma Collections

Use this prompt only if Figma collection limits or designer navigation justify splitting the one-collection workflow:

```text
Update the CEM token export Figma workflow to split the single CEM collection into dimension-specific collections
only if Figma collection limits or designer navigation justify it. Proposed collections: CEM Color, CEM Dimension,
CEM Typography, CEM Motion, and CEM Platform Notes. Keep markdown specs as source of truth, keep Figma read-only,
and document cross-collection alias handling. If aliases cannot be preserved safely across collections, duplicate
only resolved values and list the loss of alias semantics in cem-figma-report.md. Update docs/todo.md,
packages/cem-theme/docs/token-export.md, and examples/figma/README.md.
```
