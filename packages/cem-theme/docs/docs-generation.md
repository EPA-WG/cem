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
│   │   └── ...
│   └── css-generators/             → Generators: XHTML with CSS generation logic
│       ├── cem-colors.html
│       ├── cem-breakpoints.html
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

4. **CSS Extraction** - The `capture-xpath-text.mjs` script executes each dist HTML generator and saves the CSS content to
   the target path within `dist/lib/css/`

### Example Flow

```
cem-colors.md  →  dist/lib/tokens/cem-colors.xhtml
cem-colors.html → dist/lib/css-generators/cem-colors.html → dist/lib/css/cem-colors.css
```

The dist generator HTML files load the transpiled XHTML token definitions using dist-relative URLs and use XPath/XSLT
transformations to produce the final CSS output.
