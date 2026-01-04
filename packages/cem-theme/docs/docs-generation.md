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

