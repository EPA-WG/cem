# Claude Code Instructions

## Project overview

CEM (Consumer-Experience Model) is a semantic design token framework using `@epa-wg/custom-element` for declarative web
components. No shadow DOM is used -- all content renders in the light DOM.

### Key paths

| Purpose                         | Path                                               |
|---------------------------------|----------------------------------------------------|
| Token specs (markdown)          | `packages/cem-theme/src/lib/tokens/*.md`           |
| Token specs (built XHTML)       | `packages/cem-theme/dist/lib/tokens/*.xhtml`       |
| CSS generators (HTML templates) | `packages/cem-theme/src/lib/css-generators/*.html` |
| Generated CSS                   | `packages/cem-theme/dist/lib/css/*.css`            |

### Build

```bash
yarn build          # build everything
yarn build:css      # generate CSS only
yarn build:theme    # build theme package
```

## Token manifest contract

Tier is encoded as a `tier` column on each **source table** — the same h6+table the generator reads. No separate
manifest table is maintained.

```markdown
###### {spec-id}-{category}
| Token | ... existing columns ... | tier |
|---|---|---|
| `--cem-example-token` | ... | required |
```

Rules:
- Append `tier` as the **last** column to preserve existing generator column indices.
- `tier` ∈ `required` / `recommended` / `optional` / `adapter` / `deprecated`.
- For cross-product token groups (intent × state), add `tier` to the **state** table only.
- Each spec ends with a `Token manifest index` section listing source tables and validator derivation logic.
- Generators emit ONLY tokens declared in source tables. Required always; recommended by default; others behind flags.
- The manifest validator reads the source tables directly (same XPath as generators).

See `packages/cem-theme/src/lib/tokens/index.md §Token Manifest Schema` for the full schema reference.
See `packages/cem-theme/src/lib/tokens/cem-colors.md §14.3` for the worked example.

## Dev server

```bash
yarn start                                                                      # opens dist/lib/tokens/index.xhtml
yarn start packages/cem-theme/src/lib/css-generators/cem-colors.html            # opens a specific file
PORT=8080 yarn start packages/cem-theme/dist/lib/tokens/cem-colors.xhtml        # custom port
```

The server serves from filesystem root so all relative paths in HTML resolve correctly. Files must be served over
HTTP -- `file://` protocol breaks `fetch()` / `<http-request>` in the custom-element templates.

## Debugging DOM and CSS with headless browser

Use `tools/scripts/debug-cem.mjs` (Playwright, same Chromium as the build pipeline):

```bash
node tools/scripts/debug-cem.mjs packages/cem-theme/src/lib/css-generators/cem-colors.html
node tools/scripts/debug-cem.mjs packages/cem-theme/dist/lib/tokens/cem-colors.xhtml
```

Edit the `page.evaluate()` block inside the script for each investigation. Browser console and page errors are
forwarded to stderr automatically.

### Common `page.evaluate()` patterns

```js
// CSS variable on :root or any element
getComputedStyle(el).getPropertyValue('--cem-palette-comfort')

// Resolved computed values
getComputedStyle(el).backgroundColor   // e.g. "rgb(0, 16, 16)"
getComputedStyle(el).colorScheme       // "light", "dark", or "normal"

// Injected stylesheet presence / content
document.querySelectorAll('style[data-cem-css-loader]')
document.querySelector('style[data-cem-css-loader]')?.textContent.includes('--cem-palette-comfort')

// DOM context
el.parentElement.tagName
el.getRootNode().constructor.name      // "HTMLDocument" = light DOM, "ShadowRoot" = shadow DOM
el.closest('.cem-theme-dark')
```
