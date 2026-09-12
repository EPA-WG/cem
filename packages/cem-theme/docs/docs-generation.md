# Documentation Generation Design

## Overview

This document describes the CEM-ML transform graph that compiles Markdown documentation files (`.md`) to XHTML as part
of the `@epa-wg/cem-theme` build.

## Goals

1. **Compile Markdown to XHTML** - Convert `src/**/*.md` files to `.xhtml` format
2. **Typed native pipeline** - Use schema-owned Markdown, HTML, DOM, and XML conversions rather than an npm renderer
3. **Output Structure** - Match the source directory structure in `dist/`
4. **Nx Integration** - Leverage Nx caching and dependency tracking
5. **Traceability** - Emit a source-map sidecar for every XHTML document

## Architecture

`src/docs.cem` owns the graph. Each Markdown input is parsed with the GFM schema, converted to HTML, recovered into a
native DOM projection, and passed through the shared `tools/cemt/markdown-xhtml-page.cemt` layout. A final DOM-to-XML
conversion makes HTML void elements and boolean attributes well-formed for the `.xhtml` artifact. The graph also copies
the source Markdown and colocated static assets. `build:docs` first builds the native CLI, then executes this graph.

### Source and Output Structure

```
packages/cem-theme/
├── src/
│   ├── lib/
│   │   ├── cem-theme.ts          →dist/lib/cem-theme.js
│   │   └── README.md             → dist/lib/README.xhtml
│   ├── components/
│   │   ├── button.ts             → dist/components/button.js
│   │   └── button.md             → dist/components/button.xhtml
│   └── index.ts                  → dist/index.js
├── dist/
│   ├── lib/
│   │   ├── cem-theme.js
│   │   ├── cem-theme.d.ts
│   │   └── README.xhtml          → Generated from src/lib/README.md
│   └── components/
│       ├── button.js
│       ├── button.d.ts
│       └── button.xhtml          → Generated from src/components/button.md
└── docs/
    └── docs-generation.md        (this file)
```

## CSS Generation Flow

Design tokens defined in Markdown files are transformed into CSS through a multi-stage pipeline.

### Source and Output Structure

```
packages/cem-theme/
├── src/lib/
│   ├── tokens/                     → Source: metadata in XML format
│   │   ├── cem-colors.md
│   │   ├── cem-breakpoints.md
│   │   ├── cem-dimension.md
│   │   ├── cem-coupling.md
│   │   ├── cem-controls.md
│   │   └── ...
│   └── css-generators/             → Generators: XHTML with CSS generation logic
│       ├── cem-colors.html
│       ├── cem-breakpoints.html
│       ├── cem-coupling.html
│       ├── cem-controls.html
│       └── ...
├── dist/lib/
│   ├── tokens/                     → Transpiled XHTML from Markdown
│   │   ├── cem-colors.xhtml
│   │   └── ...
│   ├── css-generators/             → Dist-safe generator HTML and helper scripts
│   │   ├── cem-colors.html
│   │   └── ...
│   └── css/                        → Generated CSS output
│       ├── cem-colors.css
│       └── ...
└── tools/scripts/
    └── capture-xpath-text.mjs      → Script that executes generators
```

### Pipeline Stages

1. **Markdown to XHTML** - The native CEM-ML graph converts token definitions in `src/lib/tokens/*.md` to XHTML files
   in `dist/lib/tokens/`, preserving stable heading IDs, images, link titles, and Markdown-to-XHTML links.

2. **HTML Dist Compilation** - `src/**/*.html` is copied into `dist/` with links and scripts rewritten so generated
   files point at other `dist` files. Runtime files referenced from `node_modules` are copied into `dist/vendor/`.

3. **CSS Generation** - Each token file has a matching HTML generator in `dist/lib/css-generators/`. Its
   `template[type="cem-ml; version=0.0"]` declares the source tables in `data-slices`; the browser bootstrap projects
   those XHTML tables into `datadom.slices` and renders the preview and CSS through CEM-ML/CEM-QL. For example:
    - `cem-colors.md` → `cem-colors.html` generator
    - `cem-breakpoints.md` → `cem-breakpoints.html` generator
    - `cem-coupling.md` → `cem-coupling.html` generator
    - `cem-controls.md` → `cem-controls.html` generator

4. **CSS Extraction** - The `capture-xpath-text.mjs` script executes each dist HTML generator and saves the CSS content to
   the target path within `dist/lib/css/`

5. **Coverage report** - `scripts/generate-token-coverage.mjs` derives and writes the Markdown coverage matrix. The
   `docs/generated-token-coverage.cem` graph renders that Markdown with the same native XHTML layout as source docs.

### Example Flow

```
cem-colors.md  →  dist/lib/tokens/cem-colors.xhtml
cem-colors.html → dist/lib/css-generators/cem-colors.html → dist/lib/css/cem-colors.css
cem-controls.md → dist/lib/tokens/cem-controls.xhtml
cem-controls.html → dist/lib/css-generators/cem-controls.html → dist/lib/css/cem-controls.css
```

The dist generator HTML files load the transpiled XHTML token definitions using dist-relative URLs. The browser DOM
locates each stable heading and following table, preserves its column order as `td1`, `td2`, and so on, and passes those
records to CEM-ML/CEM-QL. Generator pages do not use live XPath/XSLT.

## CSS Generator Contract

Every token CSS generator follows the same contract. Token specs remain the canonical source of token names and values;
generators transform those specs into CSS, but do not invent missing tokens or ownership decisions.

### Source tables

Generators read source data from the compiled XHTML token spec in `dist/lib/tokens/<name>.xhtml`. The expected shape is:

1. A stable `h6` heading ID, such as `###### cem-color-hue-variant`.
2. A table immediately following that heading.
3. A final `tier` column on token source tables.

The DOM-to-datadom bridge reads the next table after the heading, which is equivalent to the former XPath contract:

```xpath
$xhtml//*[@id='<token-id>']/following-sibling::xhtml:table[1]/xhtml:tbody
```

Free-form metadata blocks are not part of the generator contract.

### Presentation completeness

Generator pages are executable documentation as well as CSS producers. A migration may change the template language,
but must not silently reduce the Markdown-driven preview. Every configured source table must contribute all of its
protocol fields either to visible cells, derived previews/tooltips, or generated CSS.

`cem-colors.html` has specialized views whose mappings are part of this contract:

| Markdown source | Required presentation |
|-----------------|-----------------------|
| `cem-color-hue-variant` | Token and hue cells; Value drives both the visible value and computed swatch; Label is visible; Intended use is the swatch tooltip |
| `cem-color-native` | One row per system color with light and dark previews and Description |
| `cem-palette-emotion-shift` | Base and `-x` rows with light, dark, native-light, and native-dark previews |
| `cem-theme-mode` × `cem-action-intent-emotion` × `cem-action-state-color` | One state-by-intent color matrix per declared theme mode |
| `cem-theme-mode` × `cem-zebra-mode-mapping` | One six-state zebra-ring matrix per declared theme mode |
| `cem-zebra-tokens` | Root zebra custom properties emitted from the Markdown formulas and consumed by the matrices |
| Additional color token tables | One visible row per source row with every source column represented |

Preview-only Swatch and Ring cells are derived views; they are not extra Markdown columns. Tests compare the rendered
page to the current compiled XHTML, not to a frozen release snapshot, so additions to canonical Markdown flow through
without weakening the protocol.

The same rule applies to every generator. Simple tables preserve the established `0.0.14` presentation; newer source
groups remain additive. A configured source that is not duplicated as a human table must be declared generator-only and
must still be proven to drive the generated CSS.

The build also inventories every Markdown document under `src/`, including the six documents that do not produce CSS.
Each must have a compiled XHTML page and a byte-identical published Markdown copy at the matching path under `dist/`.

| Generator | Human presentation sources | Generator-only sources |
|-----------|----------------------------|------------------------|
| `cem-breakpoints` | width basis, height basis, and container-query basis | active breakpoint names and width/height range helpers |
| `cem-controls` | control, progress, slider, and coupling-mode geometry | none |
| `cem-coupling` | safety minimums and halo overrides | none |
| `cem-dimension` | dimension scale, gap/inset endpoints, layout, and density overrides | reading and data rhythm |
| `cem-layering` | rungs, required/optional semantic endpoints, and forced-colors values | none |
| `cem-shape` | basis, semantic endpoints, patterns, and sharp/round overrides | action-component binding |
| `cem-stroke` | basis, appearance, semantic endpoints, zebra/rings, and forced-colors values | none |
| `cem-timing` | durations, easings, and reduced-motion values | none |
| `cem-voice-fonts-typography` | fontography, typography primitives, voice channels, and semantic roles | dark and contrast ink overrides |

`verify-phase13.mjs` resolves this inventory from each generator's current `data-slices`, compares every visible row and
preview with the compiled XHTML, and checks generator-only token names and values in the emitted CSS. The published
`0.0.14` package is useful as a one-time migration comparison, but it is intentionally not a permanent snapshot.

See [CEM-ML/CEM-QL generator simplification analysis](./cem-ml-generator-simplification.md) for the reuse boundary and
the language/runtime audit prompted by this restoration.

### Emission tiers

| Tier          | Default generator behavior                                          |
|---------------|---------------------------------------------------------------------|
| `required`    | Emit unconditionally; missing output is a validation failure        |
| `recommended` | Emit by default; adapters may opt out                               |
| `optional`    | Emit only when the spec supplies a real value                       |
| `adapter`     | Emit only behind an explicit opt-in flag                            |
| `deprecated`  | Emit only behind an explicit legacy flag and keep marked deprecated |

For cross-product token groups, such as intent x state action tokens, the tier column belongs on the source table axis
that determines the tier.

### Generator implementation pattern

Each generator HTML mirrors the existing `cem-colors.html` pattern:

1. Declare the compiled source spec with `data-token-url` and stable `data-slices` mappings.
2. Let `cem-css-generator.js` fetch the spec and project h6-plus-table rows into `datadom.slices`.
3. Render preview tables and exactly one `<code data-generated-css>` block through CEM-ML/CEM-QL.
4. Reuse `cem-css-loader.js` to apply the generated CSS to the live previews.
5. Emit only tokens declared by the canonical token spec manifest.

`capture-xpath-text.mjs` captures `//code[@data-generated-css]` into `dist/lib/css/*.css`; duplicate generated-code
blocks create duplicate or stale output and must be avoided.

### Breakpoints and conditions

CSS custom properties cannot drive `@media` or `@container` conditions. Breakpoint output is split into:

1. CSS custom properties for runtime, JavaScript, and build-tool reference.
2. Literal `@media` helper rules for stylesheet consumption.
3. Optional build-time aliases only when a later build step expands them.

Do not emit production `@custom-media` rules unless a consuming build step expands them first.

## CSS Validation

`build:css` validates generator output with `scripts/validate-manifest.mjs`. The validator checks:

1. Manifest coverage: generated CSS contains the expected token set.
2. No extra default tokens outside the manifest emission set.
3. No placeholders, empty stubs, unresolved attribute-template fragments, or unbalanced braces.
4. CSS parser validity.

`validate-manifest.mjs --hard` exits non-zero on violations.

`generate-token-coverage.mjs` uses the same manifest derivation and CSS definition analysis to produce
`dist/lib/tokens/generated-token-coverage.xhtml`; the token index links to this generated report instead of carrying a
hand-maintained matrix.

## Token Export Pipeline

The CSS pipeline is the web runtime output. The token export pipeline is the cross-platform artifact output. It reads
the same compiled token XHTML and generated CSS, then emits canonical JSON, a flat visual/voice catalog, Figma files,
TypeScript metadata, reports, and post-MVP platform files.

Primary design and checklist:

- [token-export.md](./token-export.md) — architecture, output contracts, Figma workflow, platform strategy, and risks.
- [../../docs/todo.md](../../docs/todo.md) — implementation checklist and phase gates.

Important target relationships:

1. `build:css` remains independent and produces `dist/lib/css/*.css`.
2. `build:tokens` depends on `build:css` because it resolves values through browser-computed CSS.
3. `build:token-platforms` depends on `build:tokens` and currently emits resolved-per-mode flat JSON under
   `dist/lib/token-platforms/json/`.

Generated debug artifacts, `cem.tokens.intermediate.json` and `cem.tokens.resolved.json`, are not public package
contracts. Consumers should use `cem.tokens.json`, `cem.tokens.catalog.json`, `cem.voice.tokens.json`,
`cem.tokens.ts`, Figma files, or platform outputs instead. `cem.tokens.catalog.json` and `cem.tokens.ts` are emitted
from the same sorted metadata records so data and typed consumers see one projection contract.

## Cross-Phase Verification

Run the full theme verification suite from the workspace root:

```bash
yarn nx run @epa-wg/cem-theme:verify:phase13
```

The target runs `packages/cem-theme/scripts/verify-phase13.mjs` after CSS generation. It covers manifest validation,
CSS parsing, browser-level generator capture, theme-mode resolution, forced-colors fallbacks, reduced-motion behavior,
accessibility smoke checks, cross-spec semantic checks, and default absence of adapter-only or deprecated tokens.
