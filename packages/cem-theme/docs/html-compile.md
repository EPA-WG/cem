# HTML Compilation Design

## Overview

This document describes the design for compiling HTML sources (`src/**/*.html`) into `dist/` and rewriting
`node_modules` dependencies so each HTML file references local copies in `dist/`. Only the files actually referenced by
HTML are copied, not entire packages.

## Goals

1. **Compile HTML** - Treat `src/**/*.html` as build inputs and emit to matching paths in `dist/`
2. **Rewrite Dependencies** - Replace `node_modules` URLs with `dist/` local equivalents
3. **Copy Used Files Only** - Copy only the referenced assets, not full package directories
4. **Deterministic Output** - Stable output paths for caching and reproducible builds
5. **Watch Mode Friendly** - Rebuild and recopy only affected HTML and assets

## Source and Output Structure

```
packages/cem-theme/
├── src/
│   ├── pages/
│   │   ├── index.html          → dist/pages/index.html
│   │   └── docs.html           → dist/pages/docs.html
│   └── assets/
│       └── logo.svg            → dist/assets/logo.svg
├── dist/
│   ├── pages/
│   │   ├── index.html
│   │   └── docs.html
│   └── vendor/
│       ├── lit/
│       │   └── core.min.js     (copied on demand)
│       └── dayjs/
│           └── dayjs.min.js    (copied on demand)
└── docs/
    └── html-compile.md         (this file)
```

## Dependency Model

### What counts as a dependency

Dependencies are detected from HTML attributes that load external files:

- `script[src]`
- `link[href]` for stylesheets
- `img[src]`, `source[src]`, `source[srcset]`
- `video[src]`, `audio[src]`
- `use[href]`, `use[xlink:href]` (inline SVG references)

Only URLs that start with `node_modules/` (or `/node_modules/`) are rewritten and copied. Other relative or absolute
references are left intact.

### Output location for dependencies

Copy dependencies into `dist/vendor/<package>/...` while preserving the package-internal path. Example:

- `node_modules/lit/core.min.js` → `dist/vendor/lit/core.min.js`
- `node_modules/dayjs/plugin/utc.js` → `dist/vendor/dayjs/plugin/utc.js`

The HTML references are rewritten to `./../vendor/...` or `/vendor/...` depending on the project’s URL strategy.
To keep output deterministic, use a consistent rule (recommended: absolute `/vendor/...`).

## Build Flow

1. **Discover HTML sources**
   - Use a glob for `src/**/*.html`
2. **Parse and rewrite HTML**
   - Parse HTML and collect dependency URLs
   - For each dependency URL under `node_modules`:
     - Resolve to an on-disk path
     - Compute destination path under `dist/vendor/<package>`
     - Rewrite the URL in HTML
3. **Copy assets**
   - Copy each resolved dependency file to its `dist/vendor` location
   - Copy only files actually referenced (no package directory copies)
4. **Emit HTML**
   - Write rewritten HTML to `dist/` in the same relative path as `src/`

## Rewriting Rules

- Preserve the HTML directory structure from `src/` to `dist/`
- Replace `node_modules/<pkg>/<path>` with `/vendor/<pkg>/<path>`
- Replace `/node_modules/<pkg>/<path>` with `/vendor/<pkg>/<path>`
- Do not rewrite URLs that are:
  - Remote (`http://`, `https://`)
  - Data URLs (`data:`)
  - Local relative paths (e.g., `./styles.css`, `../assets/logo.svg`)

## Example

**Input HTML** (`src/pages/index.html`)

```html
<link rel="stylesheet" href="node_modules/prismjs/themes/prism.css" />
<script type="module" src="/node_modules/lit/core.min.js"></script>
<img src="./assets/logo.svg" />
```

**Output HTML** (`dist/pages/index.html`)

```html
<link rel="stylesheet" href="/vendor/prismjs/themes/prism.css" />
<script type="module" src="/vendor/lit/core.min.js"></script>
<img src="./assets/logo.svg" />
```

**Copied files**

- `node_modules/prismjs/themes/prism.css` → `dist/vendor/prismjs/themes/prism.css`
- `node_modules/lit/core.min.js` → `dist/vendor/lit/core.min.js`

## Caching and Incremental Builds

- Track HTML file → dependency list to avoid recopying unchanged assets
- On change to an HTML file, update only the assets it references
- On change to a dependency file, recopy and update dependent HTML if needed

## Edge Cases

- **Scoped packages**: `node_modules/@scope/pkg/file.js` → `dist/vendor/@scope/pkg/file.js`
- **Query strings**: `node_modules/pkg/file.js?module` → resolve to file path without the query for copying, but preserve
  the query when rewriting
- **Srcset**: split by commas, rewrite each URL portion independently

## Validation

- Ensure all rewritten URLs point to files that exist in `dist/vendor`
- Ensure no `node_modules` references remain in emitted HTML

## Implementation Checklist

- Identify HTML sources with a `src/**/*.html` glob
- Parse HTML and extract dependency URLs from relevant attributes
- Normalize URLs, strip query/hash for filesystem resolution, keep them for output
- Map `node_modules` paths to `dist/vendor` destinations (handle scoped packages)
- Rewrite HTML URLs to `/vendor/...` paths
- Copy each referenced file into `dist/vendor`, skipping duplicates
- Emit rewritten HTML into `dist/` with original relative structure
- Add incremental caching keyed by HTML file and dependency file mtime
