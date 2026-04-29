# CEM Token Export Plan

This plan describes how to expose CEM design tokens to Figma and application developers while keeping the existing
markdown token specs as the source of truth.

The important constraint is unchanged:

```text
packages/cem-theme/src/lib/tokens/*.md
  -> packages/cem-theme/dist/lib/tokens/*.xhtml
  -> derived exports
```

Generated CSS already follows this model. JSON, Figma imports, Style Dictionary inputs, iOS files, Android files, and
developer package formats must follow the same model and must never become parallel token sources.

## Goals

- Make tokens consumable in Figma Variables, design handoff, web apps, and native apps.
- Preserve CEM's semantic token contract instead of flattening everything into implementation names.
- Keep markdown tables, h6 table ids, tier columns, and manifest derivation as the canonical contract.
- Add derived exports that can be regenerated and validated in CI.
- Make unsupported platform values visible instead of silently lossy.
- Keep current CSS generation as the primary web runtime output.

## Current State

CEM token definitions live in markdown files under `packages/cem-theme/src/lib/tokens/*.md`. The build compiles those
docs into XHTML under `packages/cem-theme/dist/lib/tokens/*.xhtml`. CSS generators under
`packages/cem-theme/src/lib/css-generators/*.html` read the built XHTML tables and emit CSS into
`packages/cem-theme/dist/lib/css/*.css`.

Manifest validation already uses the same table ids as the generators through
`packages/cem-theme/scripts/manifest-utils.mjs`. That script knows the source table ids, derives cross-product token
sets such as action intents by state, and compares expected tokens to generated CSS. The export pipeline should reuse
that manifest path instead of inventing a second registry.

## Reference Model

Material Design 3's token guidance separates token usage into a layered system: reusable system tokens, component-level
application, and generated code/design-tool artifacts. CEM should follow the same broad pattern without adopting M3's
token names as the canonical contract.

For CEM this means:

- CEM markdown tables are the token authoring source.
- CEM CSS custom properties remain the stable web-facing token API.
- Design tools receive variables that mirror CEM semantics, not ad hoc designer-only names.
- Platform exports are generated from the same semantic token graph and may use platform-specific naming only as an
  output transform.
- M3, Angular Material, MUI, or other external systems are adapter layers, not replacements for CEM tokens.

References:

- Material Design 3 token usage: https://m3.material.io/foundations/design-tokens/how-to-use-tokens
- Style Dictionary: https://github.com/style-dictionary/style-dictionary and https://styledictionary.com/
- Figma Variables and DTCG import: https://help.figma.com/hc/en-us/articles/15343816063383-Modes-for-variables
- Figma Variables API: https://developers.figma.com/docs/rest-api/variables/
- Tokens Studio: https://tokens.studio/
- Design Tokens Community Group format: https://tr.designtokens.org/format/

## Consumers

### Figma

Figma needs variables for designers, component authors, and design review. It supports practical token types such as
color, number, string, boolean, dimension in pixels, font family, and duration in seconds. Figma also supports modes, so
light and dark values should be represented as modes when concrete values are available.

Figma should not be allowed to overwrite CEM markdown specs. If a designer changes a value in Figma, that change must be
converted into a markdown-table change request and reviewed like source code.

### Application Developers

Developers need stable token names, generated assets, and platform-native formats:

- Web: current CSS custom properties, optional JSON/TS maps for tooling.
- TypeScript: typed token names and metadata for component libraries, docs, tests, and IDE assistance.
- Android: XML resources and/or Kotlin Compose token objects.
- iOS: Swift constants, SwiftUI helpers, and optional asset catalogs for colors.
- Documentation: generated coverage, token metadata, and portability reports.

## Approach Comparison

| Approach                                  | Description                                                                                                                       | Pros                                                                                                                                             | Cons                                                                                                                       | Decision                                                       |
|-------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------|
| Keep CSS-only exports                     | Continue exposing only generated CSS custom properties.                                                                           | Existing pipeline is working; web runtime remains simple; no new dependency.                                                                     | Does not serve Figma, Android, iOS, or typed developer tooling; CSS expressions are hard for non-web platforms to consume. | Keep, but insufficient alone.                                  |
| Hand-author token JSON                    | Create JSON token files and feed them to Figma or Style Dictionary directly.                                                      | Easy to use with current token tools; many examples exist.                                                                                       | Creates a second source of truth; markdown and JSON will drift; weakens the existing manifest contract.                    | Reject.                                                        |
| Parse markdown directly                   | Write an exporter that reads `src/lib/tokens/*.md` and extracts tables.                                                           | Stays close to source files; avoids waiting for compiled XHTML.                                                                                  | Must duplicate markdown parsing, anchor generation, and table-shape assumptions already solved by the build.               | Avoid for MVP.                                                 |
| Extract from built XHTML                  | Reuse compiled XHTML tables and the same table ids used by CSS generators and manifest validation.                                | Aligns with current build; reuses proven source-table structure; keeps generated artifacts derived; supports validation against CSS.             | Requires build order and robust XHTML/table parsing.                                                                       | Recommended source extraction path.                            |
| Generate DTCG JSON, then Style Dictionary | Emit canonical Design Tokens Community Group style JSON from the XHTML extractor, then use Style Dictionary for platform outputs. | Clean derived intermediate; aligns with Figma import expectations; Style Dictionary handles platform transforms and formats; easy CI validation. | CSS-specific values need normalization, aliases, or skip reports; custom transforms will be needed for CEM metadata.       | Recommended architecture.                                      |
| Use Tokens Studio as primary workflow     | Import/export tokens through the Figma plugin and sync with GitHub.                                                               | Strong designer experience; supports themes, aliases, and Figma collaboration.                                                                   | Can become a competing editor; plugin conventions may shape CEM more than desired; governance and licensing need review.   | Optional bridge, not source of truth.                          |
| Direct Figma REST API sync                | Push variables into a Figma file/library from CI or a release script.                                                             | Repeatable and automatable; supports library publishing workflows.                                                                               | Enterprise/write-access requirements; operational complexity; failure modes around file ids, permissions, and publishing.  | Later phase after file exports are stable.                     |
| Custom platform generators only           | Build every platform output with bespoke scripts.                                                                                 | Full control over CEM semantics and edge cases.                                                                                                  | Rebuilds a lot of Style Dictionary capability; higher maintenance cost.                                                    | Use only for reports or formats Style Dictionary cannot model. |

## Recommended Architecture

### 1. Keep markdown as the normative source

No token values should be edited in generated JSON, Figma files, native files, or CSS. Every derived artifact should
include a generated-file header and source metadata pointing back to the markdown spec, source table id, and token name.

The source authoring contract remains:

- Token specs live in `packages/cem-theme/src/lib/tokens/*.md`.
- Source token tables are identified by h6 ids.
- Tier stays in the last table column.
- Cross-product token families are derived from the same tables as the CSS generator.
- Each spec's manifest index documents the table ids and derivation logic.

### 2. Add a canonical export layer

Create a script such as `packages/cem-theme/scripts/export-tokens.mjs` that runs after `build:docs` and before
downstream platform exports.

The script should:

- Read `packages/cem-theme/dist/lib/tokens/*.xhtml`.
- Reuse or extend `manifest-utils.mjs` so manifest token coverage and export token coverage share one derivation path.
- Extract token value, description, tier, source table id, spec, category, and any formula/raw-value metadata.
- Emit a canonical generated JSON file in a DTCG-compatible shape.
- Emit a skip/report file for values that cannot be safely represented in a target platform.

Suggested outputs:

```text
packages/cem-theme/dist/lib/tokens/cem.tokens.json
packages/cem-theme/dist/lib/tokens/cem.tokens.report.json
packages/cem-theme/dist/lib/tokens/cem.tokens.report.md
```

### 3. Use DTCG-compatible JSON as the interchange format

Use Design Tokens Community Group style keys (`$type`, `$value`, `$description`, `$extensions`) for the canonical
derived file. CEM-specific metadata should live under `$extensions.cem`.

Example shape:

```json
{
    "cem": {
        "palette": {
            "comfort": {
                "$type": "color",
                "$value": "{cem.color.blue.l}",
                "$description": "Primary comfort surface color.",
                "$extensions": {
                    "cem": {
                        "cssName": "--cem-palette-comfort",
                        "spec": "cem-colors",
                        "sourceTable": "cem-palette-emotion-shift",
                        "tier": "required",
                        "category": "d0-palette",
                        "rawValue": "light-dark(var(--cem-color-blue-l), var(--cem-color-blue-d))",
                        "portability": "alias"
                    }
                }
            }
        }
    }
}
```

The canonical JSON should not try to hide CSS-only behavior. Values should be classified:

| Portability      | Meaning                                                      | Example handling                                                                                |
|------------------|--------------------------------------------------------------|-------------------------------------------------------------------------------------------------|
| `literal`        | Directly portable value.                                     | `#d7e3ff`, `4px`, `200ms`, `400`.                                                               |
| `alias`          | References another token and can be represented as an alias. | Palette token points to a branded color token.                                                  |
| `mode`           | Different concrete values per theme mode.                    | Figma light/dark mode files or DTCG mode extension.                                             |
| `css-expression` | Requires CSS runtime evaluation.                             | `light-dark()`, `color-mix()`, `calc()`, `var()` chains that cannot be resolved for the target. |
| `platform-note`  | Semantically useful but not directly supported by a target.  | `forced-colors`, system colors, SSML emphasis, complex font stacks.                             |

### 4. Keep current CSS generation as the web runtime path

Do not replace `packages/cem-theme/src/lib/css-generators/*.html` with Style Dictionary in the first implementation.
The current CSS generators encode CEM-specific behavior such as theme selectors, forced colors, zebra defaults, and
formulaic action state output. Style Dictionary can later emit auxiliary CSS, but the existing CSS output remains the
authoritative web runtime artifact.

### 5. Add Style Dictionary as a downstream exporter

Once canonical JSON exists, add Style Dictionary for platform-specific files. The Style Dictionary config should read
only generated canonical JSON and write only generated dist files.

Candidate outputs:

```text
packages/cem-theme/dist/lib/token-platforms/css/cem-tokens.css
packages/cem-theme/dist/lib/token-platforms/js/cem-tokens.js
packages/cem-theme/dist/lib/token-platforms/ts/cem-tokens.d.ts
packages/cem-theme/dist/lib/token-platforms/android/cem_tokens.xml
packages/cem-theme/dist/lib/token-platforms/android/CemTokens.kt
packages/cem-theme/dist/lib/token-platforms/ios/CemTokens.swift
```

Use Style Dictionary where it is strong:

- Name transforms for platform conventions.
- Unit transforms such as px to dp/sp/pt when policy is explicit.
- Built-in and custom formats.
- Reference-preserving output when a target format supports references.
- Filtering by token type, tier, platform support, or CEM portability.

Use custom CEM code where Style Dictionary is not enough:

- Parsing XHTML token tables.
- Resolving CEM formulas and mode-specific values.
- Producing target skip reports.
- Preserving source table metadata.
- Validating generated outputs against the markdown-derived manifest.

## Figma Export Plan

Figma should receive generated files optimized for import into Variables. The initial implementation should use file
imports because they are low-risk and easy to review. Direct API sync can be added later.

### Collections and modes

Start with one CEM collection unless Figma's variable limits or usability require splitting:

```text
CEM
  Modes: light, dark
  Groups: color, palette, action, dimension, shape, stroke, layering, timing, typography, breakpoints
```

If contrast or native/system themes cannot be expressed concretely, export them as separate reports first rather than
creating misleading variables.

Potential later split:

```text
CEM Color
CEM Dimension
CEM Typography
CEM Motion
CEM Platform Notes
```

### Figma naming

Keep CSS custom property names in metadata and expose designer-friendly variable paths. Names must normalize uniquely
under Figma's slash-separated groups.

Example:

```text
--cem-action-primary-default-background
  -> action/primary/default/background

--cem-dimension-3
  -> dimension/3

--cem-palette-comfort-text
  -> palette/comfort/text
```

The exporter must validate duplicate normalized names and fail or report before import.

### Supported values

Figma import should include only values that can be represented accurately:

| CEM value class                   | Figma handling                                                                                    |
|-----------------------------------|---------------------------------------------------------------------------------------------------|
| Hex, rgb, hsl colors              | Export as color tokens.                                                                           |
| Aliased colors                    | Export as aliases where import supports DTCG references.                                          |
| Pixel dimensions                  | Export as dimension or number tokens.                                                             |
| Numeric weights, opacity, z-index | Export as number tokens.                                                                          |
| Font family single value          | Export as fontFamily/string.                                                                      |
| Duration                          | Convert ms to seconds for duration tokens.                                                        |
| Complex font stacks               | Export string only if useful; otherwise report.                                                   |
| `light-dark()`                    | Split into light/dark mode values when both sides can be resolved.                                |
| `color-mix()`                     | Resolve only when inputs and formula are supported by the exporter; otherwise report as CSS-only. |
| CSS system colors                 | Report as native/system-only values unless mapped by target policy.                               |
| Forced-colors behavior            | Report as CSS accessibility behavior, not Figma variables.                                        |

### Figma outputs

Suggested outputs:

```text
packages/cem-theme/dist/lib/tokens/figma/cem-light.tokens.json
packages/cem-theme/dist/lib/tokens/figma/cem-dark.tokens.json
packages/cem-theme/dist/lib/tokens/figma/cem-figma-report.md
```

Each mode file should contain the same token names and compatible token types so Figma can import them as modes in one
collection. Tokens missing from one mode should be excluded from all mode files and listed in the report.

### Figma governance

- Figma imports are generated artifacts.
- Designers may propose edits in Figma, but source changes must land as markdown table changes.
- A release checklist should include reimporting variables into a test file before publishing a library update.
- Direct Figma API sync should require explicit file id configuration and should never run by default in a local build.

## Developer Export Plan

### Web

Keep `dist/lib/css/*.css` as the supported runtime API. Add optional generated JSON/TS metadata for tooling:

- Token name union types.
- Category and tier metadata.
- Source spec and table id metadata.
- Portability and target support reports.

### TypeScript and JavaScript

Generate a small package-facing module from canonical JSON:

```ts
export type CemTokenName = '--cem-palette-comfort' | '--cem-action-primary-default-background';

export interface CemTokenMeta {
  name: CemTokenName;
  type: 'color' | 'dimension' | 'number' | 'duration' | 'fontFamily' | 'string';
  tier: 'required' | 'recommended' | 'optional' | 'adapter' | 'deprecated';
  spec: string;
  sourceTable: string;
}
```

The TS output is for tooling and docs, not a replacement for CSS in browser runtime styling.

### Android

Generate two levels:

- XML resources for portable color, dimension, integer, and duration values.
- Kotlin/Compose token objects for semantic names and values that Compose can consume directly.

Use reports for CSS-only values instead of guessing Android equivalents.

Policy decisions to document before implementation:

- Whether CSS px maps to Android dp by default.
- Whether typography size tokens map to sp.
- How elevation/layering tokens map if values are not simple dimensions.
- Whether light/dark colors are generated as resource qualifiers or Compose color schemes.

Recommended default: map spacing and shape px to dp, typography px/rem to sp only when the source token category is
typographic size, and keep other mappings explicit.

### iOS

Generate:

- Swift constants for dimensions, durations, z-index-like layering values, and semantic metadata.
- SwiftUI `Color` helpers for portable colors.
- Optional asset catalog colors only after light/dark mode resolution is reliable.

Policy decisions to document before implementation:

- Whether CSS px maps to iOS points.
- How dynamic type should interact with CEM typography tokens.
- Whether color aliases should remain semantic Swift properties or be resolved into concrete colors.

Recommended default: map CSS px to points for layout tokens, avoid dynamic type scaling unless the token spec explicitly
identifies typographic roles, and keep semantic color accessors stable.

## Implementation Phases

### Phase 1: Documentation and extraction contract

- Add this plan.
- Add an "Export contract" section to `packages/cem-theme/src/lib/tokens/index.md` after the manifest schema once the
  exporter exists.
- Inventory source tables and columns needed for value, description, type inference, and tier extraction.
- Decide the first set of exported token types: color, dimension, number, duration, fontFamily, string.

### Phase 2: Canonical JSON exporter

- Create `packages/cem-theme/scripts/export-tokens.mjs`.
- Refactor `manifest-utils.mjs` only as needed so token derivation can return source table id and row data.
- Add type inference and value normalization.
- Emit `dist/lib/tokens/cem.tokens.json`.
- Emit `dist/lib/tokens/cem.tokens.report.md`.
- Add validation that the canonical export covers the same required/recommended manifest tokens as generated CSS, except
  explicitly reported unsupported values.

### Phase 3: Figma file exports

- Generate light and dark DTCG JSON files for Figma import.
- Add duplicate-name validation after Figma slash-name normalization.
- Add same-token-same-type validation across mode files.
- Add a manual import checklist and screenshot/test fixture in docs.

### Phase 4: Style Dictionary platform exports

- Add Style Dictionary as a dev dependency.
- Add `packages/cem-theme/style-dictionary.config.mjs`.
- Generate CSS auxiliary output, JS/TS metadata, Android XML/Compose, and iOS Swift files.
- Add custom filters for tier, type, and portability.
- Add custom transforms for CEM name conversion and unit policy.

### Phase 5: Release and automation

- Wire exporter into `@epa-wg/cem-theme` build after CSS validation.
- Add CI checks for JSON validity, duplicate names, unsupported required tokens, and Style Dictionary output generation.
- Publish generated files in the package if they are intended for consumers.
- Consider direct Figma REST API sync only after file import has been used successfully.

## Validation Requirements

The exporter should fail hard for:

- Required token missing from canonical JSON.
- Duplicate canonical token identity.
- Duplicate Figma-normalized name within one collection.
- Same Figma variable name having different types across mode files.
- Invalid DTCG shape.
- Platform output referencing a token filtered out of that platform.
- Generated output not carrying a generated-file header.

The exporter may warn and report for:

- Optional tokens without portable target values.
- CSS expressions that cannot be resolved outside CSS.
- Native/system color values that are meaningful only in browser/native accessibility contexts.
- Deprecated tokens excluded from a target.
- Adapter tokens excluded from core outputs.

## Build Integration

Recommended target layout:

```json
{
    "build:tokens": {
        "dependsOn": [
            "build:docs"
        ],
        "executor": "nx:run-commands",
        "options": {
            "command": "node scripts/export-tokens.mjs",
            "cwd": "packages/cem-theme"
        },
        "outputs": [
            "{projectRoot}/dist/lib/tokens/cem.tokens.json",
            "{projectRoot}/dist/lib/tokens/cem.tokens.report.md",
            "{projectRoot}/dist/lib/tokens/figma"
        ]
    },
    "build:token-platforms": {
        "dependsOn": [
            "build:tokens"
        ],
        "executor": "nx:run-commands",
        "options": {
            "command": "node scripts/build-token-platforms.mjs",
            "cwd": "packages/cem-theme"
        },
        "outputs": [
            "{projectRoot}/dist/lib/token-platforms"
        ]
    }
}
```

`build:css` should not be blocked on native platform generation until those outputs are stable. A later release can make
`build` depend on both CSS and token platform exports.

## Versioning and Package Contract

The package should treat source markdown and generated CSS as the strongest compatibility contract. New derived outputs
should initially be marked experimental until import and native-platform behavior are proven.

Recommended stability labels:

- Stable: markdown specs, CSS custom properties, required token names.
- Beta: canonical DTCG JSON, TypeScript metadata.
- Experimental: Figma imports, Android XML/Compose, iOS Swift, direct API sync.

Breaking changes include:

- Removing or renaming required tokens.
- Changing token type in canonical JSON.
- Changing Figma-normalized names after a release.
- Changing platform unit mapping defaults.
- Moving a token between required and optional in a way that removes it from outputs.

## Open Decisions

These should be resolved before implementing native exports:

- Exact output path and package export map for generated JSON and TS metadata.
- Whether Figma gets one collection or separate collections by token dimension.
- Whether contrast/native themes become Figma modes or remain CSS-only reports.
- Whether px maps to dp/pt globally or per token category.
- Whether `color-mix()` values should be resolved during export or reported as CSS expressions.
- Whether Tokens Studio should be supported as an optional import target.
- Whether direct Figma REST API sync belongs in CI, release tooling, or a manual script.

## Recommended MVP

The safest first implementation is:

1. Generate canonical DTCG-compatible JSON from built XHTML.
2. Generate a portability report.
3. Generate Figma light/dark import files for literal and aliasable tokens only.
4. Generate TypeScript token metadata for developers.
5. Keep native Android/iOS exports behind a separate build target until unit and mode policies are validated.

This gives designers and developers useful exports quickly while preserving the current CEM source-of-truth model and
avoiding silent loss of CSS-only semantics.
