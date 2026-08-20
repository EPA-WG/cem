# CEM Tokens in Figma

**Status:** Current workflow for the CEM UI Kit.
**Audience:** CEM maintainers, design-system reviewers, and Figma library maintainers.

## Purpose

CEM tokens are authored in markdown specs and exported as generated JSON artifacts. Figma must consume those generated
artifacts as a read-only projection of the code source of truth.

The `CEM UI Kit` is a Figma-native design library. Its token layer is native Figma Variables generated from CEM token
artifacts, not plugin-managed token sets and not designer-authored token data.

The library file is:

```text
CEM UI Kit
https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD
```

The Tokens page is the primary review surface for native variables and generated token demos:

```text
01 Tokens
https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit?node-id=2-24&t=QQwTKeMg0v9dTQ10-1
```

Use this page to review the `CEM Tokens` variable collection, mode behavior, and the generator-category visual demos
that reflect values from the published CSS variable generator pages.

## Source files

Use the Figma-specific generated token files, not the aggregate `cem.tokens.json`, when populating Figma.

Local generated files:

```text
packages/cem-theme/dist/lib/tokens/figma/cem-light.tokens.json
packages/cem-theme/dist/lib/tokens/figma/cem-dark.tokens.json
packages/cem-theme/dist/lib/tokens/figma/cem-contrast-light.tokens.json
packages/cem-theme/dist/lib/tokens/figma/cem-contrast-dark.tokens.json
packages/cem-theme/dist/lib/tokens/figma/cem-native.tokens.json
```

Pinned npm CDN URLs for `@epa-wg/cem-theme`:

```text
https://unpkg.com/@epa-wg/cem-theme/dist/lib/tokens/figma/cem-light.tokens.json
https://unpkg.com/@epa-wg/cem-theme/dist/lib/tokens/figma/cem-dark.tokens.json
https://unpkg.com/@epa-wg/cem-theme/dist/lib/tokens/figma/cem-contrast-light.tokens.json
https://unpkg.com/@epa-wg/cem-theme/dist/lib/tokens/figma/cem-contrast-dark.tokens.json
https://unpkg.com/@epa-wg/cem-theme/dist/lib/tokens/figma/cem-native.tokens.json
```

Do not use `@latest` for the production Figma library. Pin the package version and update intentionally when a new
token release is approved.

## Direct Figma variable model

The CEM UI Kit uses one native Figma variable collection with five modes:

```text
Collection: CEM Tokens

Modes:
- Light
- Dark
- Contrast Light
- Contrast Dark
- Native
```

Each mode is populated from the matching generated token file:

| Figma mode      | Source file                           |
| --------------- | ------------------------------------- |
| Light           | `cem-light.tokens.json`               |
| Dark            | `cem-dark.tokens.json`                |
| Contrast Light  | `cem-contrast-light.tokens.json`      |
| Contrast Dark   | `cem-contrast-dark.tokens.json`       |
| Native          | `cem-native.tokens.json`              |

Populate or refresh the collection with Figma's native **Import mode** action.
The generated files use the DTCG JSON format accepted by Figma, so the canonical
workflow requires no token plugin or bidirectional synchronization connector.
Figma's current import and mode behavior is documented in
[Modes for variables](https://help.figma.com/hc/en-us/articles/15343816063383-Modes-for-variables).

Token names stay slash-based in Figma:

```text
cem/color/blue/xl
cem/palette/comfort
cem/zebra/color/0
cem/dim/small
cem/gap/block
cem/duration/action
```

## Type mapping

The generated Figma files in 0.1.1 contain matching 252-token sets per mode.
Some JSON nodes are both a token and a group parent; native import must capture
the parent token and continue walking child groups.

```text
252 tokens per mode
57 color
96 dimension
6 duration
5 fontFamily
10 number
78 string
```

Map token types to Figma variable types as follows:

| DTCG token type | Figma variable type | Generated `$value` shape | Notes |
| --- | --- | --- | --- |
| `color` | `COLOR` | `{ colorSpace: "srgb", components: [...], alpha, hex }` | Resolve CSS colors and emit the current DTCG color object. |
| `dimension` | `FLOAT` | `{ value: 16, unit: "px" }` | Resolve px-compatible CSS dimensions and convert rem values to px. |
| `number` | `FLOAT` | Numeric scalar | Preserve finite unitless values. |
| `duration` | `FLOAT` | `{ value: 0.25, unit: "s" }` | Normalize milliseconds to seconds; Figma imports the duration as a number variable. |
| `fontFamily` | `STRING` | Family-name string | Preserve the first family name; text styles can be generated separately later. |
| `string` | `STRING` | String scalar | Preserve platform notes, OpenType settings, and other string values. |

Aliases such as `{cem.palette.comfort}` become Figma variable aliases when the source and target variable types match.
If the target type does not match, preserve the value as a string and list it in the import report.

Figma's native Import mode does not currently accept DTCG `shadow` or
`cubicBezier` tokens. Layering and easing families therefore remain excluded
from the five mode files and visible in `cem-figma-report.md`. The approved UI
Kit projection is governed by `examples/figma/foundations-library.json`:

- six canonical shadow recipes become derived `CEM/Layering/*` Effect Styles;
- `cem/elevation/0` remains an explicit no-effect specimen because DTCG shadow
  arrays cannot be empty;
- five semantic `cem/layer/*` aliases annotate their owning rung rather than
  duplicating Effect Styles; and
- eight canonical `cubicBezier` tokens become derived motion specimens.

Run `yarn nx run @epa-wg/cem-theme:verify:figma-foundations` to validate the
raw-value-free inventory and emit the composite review report before editing the
canvas.

## Native Figma library workflow

The CEM theme deliverable for Figma is the Figma-native design library: native Figma Variables in the `CEM UI Kit`
file, imported from the `figma/cem-*.tokens.json` artifacts.

The generated JSON files are read-only inputs from code. A refresh must keep the same five generated files mapped to
the five `CEM Tokens` modes:

- `cem-light`
- `cem-dark`
- `cem-contrast-light`
- `cem-contrast-dark`
- `cem-native`

Before importing, run
`yarn nx run @epa-wg/cem-theme:verify:figma-native-review` and follow
`examples/figma/native-library-review-fixture.md`. Record the live starting
revision and confirmed collection/modes by promoting the checked-in record from
`pending` to `started`; promote it to `reviewed` only after all five imports and
live result checks are complete. The generated report is preparation evidence,
not proof that the external file changed.

Two-way write-back from Figma to Git or npm is outside the CEM theme deliverable and must stay disabled unless the
markdown source-of-truth workflow changes.

## What not to import

Do not use these files as the primary Figma UI Kit import:

```text
packages/cem-theme/dist/lib/tokens/cem.tokens.json
packages/cem-theme/dist/lib/tokens/cem.tokens.intermediate.json
packages/cem-theme/dist/lib/tokens/cem.tokens.resolved.json
packages/cem-theme/dist/lib/tokens/cem.voice.tokens.json
```

`cem.tokens.json` is the canonical aggregate artifact for code and downstream token tooling. The `figma/cem-*.tokens.json`
files are the Figma propagation surface because they are already split by CEM's Figma mode axis.

`cem.voice.tokens.json` is intentionally separate. Voice/audio metadata is not part of the visual Figma variable
collection.

## Import validation

After importing or regenerating the Figma variables:

1. Confirm `CEM Tokens` has exactly five modes.
2. Confirm each mode has the same token names and compatible variable types.
3. Confirm aliases resolve across the collection.
4. Apply the smoke fixture in `examples/figma/sample-token-application.md`.
5. Switch the collection mode through `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, and `Native`.

The smoke fixture should keep the same variable bindings while values change with the active mode.

## Checking Tokens in the Figma UI

Use this manual check after direct variable creation in the `CEM UI Kit` file.

### Open the variable collection

1. Open the `CEM UI Kit` Figma file or the direct
   [Tokens page](https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit?node-id=2-24&t=QQwTKeMg0v9dTQ10-1).
2. In the left sidebar, switch to the `01 Tokens` page.
3. Open Figma variables from the canvas toolbar: `Local variables` / `Variables`.
4. Select the `CEM Tokens` collection.

Confirm:

- The collection name is `CEM Tokens`.
- The collection has exactly five modes:
  - `Light`
  - `Dark`
  - `Contrast Light`
  - `Contrast Dark`
  - `Native`
- The variable groups are slash-delimited under `cem/...`.

### Spot-check values by mode

In the `CEM Tokens` collection, search for these variables and switch across all five modes:

| Variable | Expected check |
| -------- | -------------- |
| `cem/palette/comfort` | Changes from light surface to dark/native surface values by mode. |
| `cem/palette/comfort/text` | Inverts against `cem/palette/comfort`. |
| `cem/palette/calm` | Changes between light, dark, contrast, and native values. |
| `cem/dim/medium` | Stays `16` across all modes. |
| `cem/gap/block` | Is an alias to `cem/dim/medium`. |
| `cem/bend/control` | Is an alias through the bend chain. |
| `cem/zebra/color/0` | Is an alias to `cem/palette/comfort`. |

### Check aliases

For variables with alias values, the value cell should show a linked variable reference rather than a copied literal
value.

Check these aliases:

```text
cem/zebra/color/0 -> cem/palette/comfort
cem/gap/block -> cem/dim/medium
cem/layout/stack/gap -> cem/gap/block
cem/bend/smooth -> cem/dim/x/small
cem/bend/control -> cem/bend
```

String variables that point at numeric tokens cannot become native Figma aliases because Figma aliases must match
variable type. These are preserved as string token references. Check that the value is still a token reference string:

```text
cem/typography/ui/font/size = {cem.typography.size.m}
cem/typography/brand/font/size = {cem.typography.size.xxl}
```

### Check code syntax

Select a variable and inspect its code syntax. WEB syntax must use the CSS `var()` wrapper:

```text
cem/palette/comfort -> var(--cem-palette-comfort)
cem/dim/medium -> var(--cem-dim-medium)
cem/bend/control -> var(--cem-bend-control)
```

### Smoke-test bindings on canvas

Create a temporary frame on `99 QA` or use the fixture in `examples/figma/sample-token-application.md`.

1. Create a frame named `CEM token smoke`.
2. Bind the frame fill to `cem/palette/comfort`.
3. Add text and bind its fill to `cem/palette/comfort/text`.
4. Create a small button frame:
   - Fill: `cem/palette/calm`
   - Corner radius: `cem/bend/control`
   - Horizontal/vertical padding or gap: `cem/inset/control`
   - Stroke width: `cem/stroke/boundary`
5. Change the frame's explicit variable mode for `CEM Tokens`.

Confirm the same variable names stay bound while the rendered values change between `Light`, `Dark`,
`Contrast Light`, `Contrast Dark`, and `Native`.
