# Documentation Generation Design

## Overview

This document describes the design for compiling Markdown documentation files (`.md`) to XHTML format as part of the
build process in the `@epa-wg/cem-theme` package.

## Goals

1. **Compile Markdown to XHTML** - Convert `src/**/*.md` files to `.xhtml` format
2. **Parallel Build Process** - Treat Markdown files as source files, similar to TypeScript
3. **Output Structure** - Match the directory structure of compiled TypeScript files in `dist/`
4. **Nx Integration** - Leverage Nx caching and dependency tracking
5. **Watch Mode Support** - Rebuild XHTML when Markdown files change

## Architecture

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

1. **Markdown to XHTML** - Token definitions in `src/lib/tokens/*.md` contain metadata in XML format and are transpiled
   to XHTML files in `dist/lib/tokens/`

2. **HTML Dist Compilation** - `src/**/*.html` is copied into `dist/` with links and scripts rewritten so generated
   files point at other `dist` files. Runtime files referenced from `node_modules` are copied into `dist/vendor/`.

3. **CSS Generation** - Each token file has a matching HTML generator in `dist/lib/css-generators/`. For example:
    - `cem-colors.md` → `cem-colors.html` generator
    - `cem-breakpoints.md` → `cem-breakpoints.html` generator
    - `cem-coupling.md` → `cem-coupling.html` generator
    - `cem-controls.md` → `cem-controls.html` generator

4. **CSS Extraction** - The `capture-xpath-text.mjs` script executes each dist HTML generator and saves the CSS content to
   the target path within `dist/lib/css/`

5. **Coverage report** - `scripts/generate-token-coverage.mjs` derives the coverage matrix from the same manifest and
   CSS analysis helpers used by validation, then writes `dist/lib/tokens/generated-token-coverage.md` and
   `dist/lib/tokens/generated-token-coverage.xhtml`.

### Example Flow

```
cem-colors.md  →  dist/lib/tokens/cem-colors.xhtml
cem-colors.html → dist/lib/css-generators/cem-colors.html → dist/lib/css/cem-colors.css
cem-controls.md → dist/lib/tokens/cem-controls.xhtml
cem-controls.html → dist/lib/css-generators/cem-controls.html → dist/lib/css/cem-controls.css
```

The dist generator HTML files load the transpiled XHTML token definitions using dist-relative URLs and use XPath/XSLT
transformations to produce the final CSS output.

## CSS Generator Contract

Every token CSS generator follows the same contract. Token specs remain the canonical source of token names and values;
generators transform those specs into CSS, but do not invent missing tokens or ownership decisions.

### Source tables

Generators read source data from the compiled XHTML token spec in `dist/lib/tokens/<name>.xhtml`. The expected shape is:

1. A stable `h6` heading ID, such as `###### cem-color-hue-variant`.
2. A table immediately following that heading.
3. A final `tier` column on token source tables.

Generator XPath reads the next table after the heading:

```xpath
$xhtml//*[@id='<token-id>']/following-sibling::xhtml:table[1]/xhtml:tbody
```

Free-form metadata blocks are not part of the generator contract.

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

1. Load the compiled source spec through `<cem-http-request>`.
2. Extract source rows through the h6-plus-table XPath contract.
3. Emit exactly one `<code data-generated-css>` block.
4. Reuse `cem-css-loader.js` and `cem-http-request.js`.
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

## Cross-Phase Verification

Run the full theme verification suite from the workspace root:

```bash
yarn nx run @epa-wg/cem-theme:verify:phase13
```

The target runs `packages/cem-theme/scripts/verify-phase13.mjs` after CSS generation. It covers manifest validation,
CSS parsing, browser-level generator capture, theme-mode resolution, forced-colors fallbacks, reduced-motion behavior,
accessibility smoke checks, cross-spec semantic checks, and default absence of adapter-only or deprecated tokens.
