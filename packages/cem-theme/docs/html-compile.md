# HTML Dist Compilation

## Overview

The theme build compiles every `src/**/*.html` file into a matching `dist/**/*.html` file. The emitted HTML is rewritten
so it can run from the package `dist` folder without loading scripts from `node_modules`.

The source HTML remains development-oriented. Only the copied `dist` HTML receives rewritten URLs.

## Goals

1. **Mirror source HTML** - Copy `src/**/*.html` to the same relative path under `dist/`.
2. **Use dist-relative links** - Rewrite references to built token docs, generated CSS, local helper scripts, and package
   runtime files so each emitted HTML file points at files in `dist`.
3. **Avoid node_modules at runtime** - Copy only referenced third-party runtime files into `dist/vendor/`.
4. **Keep generator execution stable** - Run CSS extraction from `dist/lib/css-generators/*.html`, not from source HTML.

## Output Structure

```
packages/cem-theme/
├── src/lib/
│   ├── css-generators/
│   │   ├── cem-colors.html
│   │   ├── cem-http-request.js
│   │   └── cem-css-loader.js
│   └── theme-editor/
│       └── theme-editor.html
└── dist/
    ├── lib/
    │   ├── css-generators/
    │   │   ├── cem-colors.html
    │   │   ├── cem-http-request.js
    │   │   └── cem-css-loader.js
    │   ├── theme-editor/
    │   │   └── theme-editor.html
    │   ├── tokens/
    │   │   └── cem-colors.xhtml
    │   └── css/
    │       └── cem-colors.css
    └── vendor/
        └── @epa-wg/custom-element/
            ├── custom-element.js
            └── http-request.js
```

## Rewriting Rules

- Source files keep their original URLs.
- Emitted HTML uses relative URLs from the emitted file location.
- URLs that resolve into `packages/cem-theme/dist/` are rewritten to the matching relative dist path.
- URLs that resolve into repo `node_modules/` are copied to `packages/cem-theme/dist/vendor/` and rewritten to the
  copied file.
- URLs that resolve to local source JavaScript are copied to the matching path under `dist/` and rewritten if needed.
- Relative links between source HTML files are rewritten to their mirrored `dist` locations, which usually preserves the
  same visible URL.
- Remote URLs, data URLs, hash-only links, and other protocol URLs are left unchanged.
- Query strings and hash fragments are preserved after rewriting.

## Examples

From `src/lib/css-generators/cem-colors.html` to `dist/lib/css-generators/cem-colors.html`:

```html
<http-request url="../../../dist/lib/tokens/cem-colors.xhtml"></http-request>
```

becomes:

```html
<http-request url="../tokens/cem-colors.xhtml"></http-request>
```

Runtime scripts:

```html
<script src="../../../../../node_modules/@epa-wg/custom-element/custom-element.js" type="module"></script>
```

becomes:

```html
<script src="../../vendor/@epa-wg/custom-element/custom-element.js" type="module"></script>
```

From `src/lib/theme-editor/theme-editor.html` to `dist/lib/theme-editor/theme-editor.html`:

```css
@import "../../../dist/lib/css/cem-colors.css";
```

becomes:

```css
@import "../css/cem-colors.css";
```

## Build Flow

1. `build:docs` compiles Markdown token specs into `dist/lib/tokens/*.xhtml`.
2. `build:html` runs `tools/scripts/compile-html.mjs`, copies HTML into `dist`, rewrites URLs, and copies referenced
   runtime files into `dist/vendor`.
3. `build:css` executes `dist/lib/css-generators/*.html` and captures `code[data-generated-css]` into
   `dist/lib/css/*.css`.
4. Manifest validation checks generated CSS against the built token XHTML files.

## Validation

- Run `yarn build:theme`.
- Confirm no emitted HTML references `node_modules`.
- Open or debug `packages/cem-theme/dist/lib/css-generators/cem-colors.html` over HTTP; direct `file://` loading is not
  supported because the generators use `fetch()`.
