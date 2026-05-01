# Claude Code Instructions

## Project overview

CEM (Consumer-Experience Model) is a semantic design token framework using `@epa-wg/custom-element` for declarative web
components. No shadow DOM is used -- all content renders in the light DOM.

### Key paths

| Purpose                         | Path                                               |
| ------------------------------- | -------------------------------------------------- |
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

| Token                 | ... existing columns ... | tier     |
| --------------------- | ------------------------ | -------- |
| `--cem-example-token` | ...                      | required |
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

## Token export contract

Token specs remain canonical markdown. The token export pipeline reads compiled XHTML and generated CSS, then emits
cross-platform artifacts from `packages/cem-theme/scripts/export-tokens.mjs`.

Public beta outputs under `packages/cem-theme/dist/lib/tokens/`:

- `cem.tokens.json` — canonical DTCG-compatible visual tokens.
- `cem.voice.tokens.json` — voice/audio metadata, separate from visual outputs.
- `cem.tokens.ts` — TypeScript token names and metadata for docs/tests/autocomplete.
- `cem.tokens.report.{md,json}` — portability and skipped-token reports.

Experimental outputs:

- `figma/cem-*.tokens.json` — one read-only native Figma library source file per mode.
- `../token-platforms/json/cem-tokens-*.json` — resolved-per-mode flat JSON for adapter experiments.

Debug-only outputs:

- `cem.tokens.intermediate.json`
- `cem.tokens.resolved.json`

Consumers must not import debug artifacts. Prefer package export subpaths such as
`@epa-wg/cem-theme/tokens/cem.tokens.json` or `@epa-wg/cem-theme/tokens/cem.tokens.ts` instead of deep `dist/` paths.

Build relationships:

- `build:css` is independent.
- `build:tokens` depends on `build:css`.
- `build:token-platforms` depends on `build:tokens`.

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
getComputedStyle(el).getPropertyValue('--cem-palette-comfort');

// Resolved computed values
getComputedStyle(el).backgroundColor; // e.g. "rgb(0, 16, 16)"
getComputedStyle(el).colorScheme; // "light", "dark", or "normal"

// Injected stylesheet presence / content
document.querySelectorAll('style[data-cem-css-loader]');
document.querySelector('style[data-cem-css-loader]')?.textContent.includes('--cem-palette-comfort');

// DOM context
el.parentElement.tagName;
el.getRootNode().constructor.name; // "HTMLDocument" = light DOM, "ShadowRoot" = shadow DOM
el.closest('.cem-theme-dark');
```

<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

## General Guidelines for working with Nx

- For navigating/exploring the workspace, invoke the `nx-workspace` skill first - it has patterns for querying projects, targets, and dependencies
- When running tasks (for example build, lint, test, e2e, etc.), always prefer running the task through `nx` (i.e. `nx run`, `nx run-many`, `nx affected`) instead of using the underlying tooling directly
- Prefix nx commands with the workspace's package manager (e.g., `pnpm nx build`, `npm exec nx test`) - avoids using globally installed CLI
- You have access to the Nx MCP server and its tools, use them to help the user
- For Nx plugin best practices, check `node_modules/@nx/<plugin>/PLUGIN.md`. Not all plugins have this file - proceed without it if unavailable.
- NEVER guess CLI flags - always check nx_docs or `--help` first when unsure

## Scaffolding & Generators

- For scaffolding tasks (creating apps, libs, project structure, setup), ALWAYS invoke the `nx-generate` skill FIRST before exploring or calling MCP tools

## When to use nx_docs

- USE for: advanced config options, unfamiliar flags, migration guides, plugin configuration, edge cases
- DON'T USE for: basic generator syntax (`nx g @nx/react:app`), standard commands, things you already know
- The `nx-generate` skill handles generator discovery internally - don't call nx_docs just to look up generator syntax

<!-- nx configuration end-->
