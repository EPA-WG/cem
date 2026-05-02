# `@epa-wg/cem-theme`

Canonical CEM token specs, generated CSS, DTCG JSON, TypeScript metadata, native iOS/Android outputs, and Figma
library files. [Token specs](./src/lib/tokens/index.md) in `src/lib/tokens/*.md` are the single source of truth; copied Markdown specs are published
under `dist/lib/tokens/*.md` so consumers and AI coding assistants can read version-matched semantic guidance from the
installed npm package. See the prompt below.

## Use the token CSS

The generated CSS exposes every CEM token as a CSS custom property on `:root`. Drop it into any page and consume
tokens via `var(--cem-...)`.

| File                            | When to use                                                                                                                                    |
|---------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------|
| `dist/lib/css/cem-combined.css` | Single concatenated file. One HTTP request — best for `<link>` and CDN delivery.                                                               |
| `dist/lib/css/cem.css`          | `@import` index over per-spec files (`cem-colors.css`, `cem-dimension.css`, …). Best when a tool resolves `@import` and you want tree-shaking. |

## Via the npm package

```bash
yarn add @epa-wg/cem-theme
```

```html
<link rel="stylesheet" href="node_modules/@epa-wg/cem-theme/dist/lib/css/cem-combined.css" />
```

```js
// Bundlers that handle CSS imports
import '@epa-wg/cem-theme/dist/lib/css/cem-combined.css';
```

### Prompt for applying CEM styling to an existing project

If `@epa-wg/cem-theme` is already installed as an npm dependency, use this prompt with a coding assistant:

```text
Apply CEM theme styling to this existing project using the installed `@epa-wg/cem-theme` package.

Before changing styles, read the installed package-local AI instructions:
`node_modules/@epa-wg/cem-theme/dist/lib/tokens/cem-theme-ai-instructions.md`.

Follow that file's read order, token-selection rules, stylesheet setup, theme scoping, and verification checklist.
Prefer these installed Markdown docs over GitHub because they match the installed npm package version. Do not infer CEM
semantics from generated CSS values alone.
```

## Via unpkg CDN (no install)

```html
<!-- pin a specific version -->
<link rel="stylesheet" href="https://unpkg.com/@epa-wg/cem-theme@0.0.9/dist/lib/css/cem-combined.css" />

<!-- or float to latest -->
<link rel="stylesheet" href="https://unpkg.com/@epa-wg/cem-theme@latest/dist/lib/css/cem-combined.css" />
```

The same paths work for individual specs, e.g.
`https://unpkg.com/@epa-wg/cem-theme@latest/dist/lib/css/cem-colors.css`.


## Primary exports

| Path                                              | Contents                                            |
|---------------------------------------------------|-----------------------------------------------------|
| `@epa-wg/cem-theme`                               | Theme entry (TypeScript).                           |
| `@epa-wg/cem-theme/tokens/cem.tokens.json`        | Canonical DTCG-compatible visual tokens.            |
| `@epa-wg/cem-theme/tokens/cem.voice.tokens.json`  | Voice/audio metadata, separate from visual outputs. |
| `@epa-wg/cem-theme/tokens/cem.tokens.ts`          | Token names + metadata for docs/tests/autocomplete. |
| `@epa-wg/cem-theme/tokens/cem.tokens.report.json` | Portability and skipped-token report.               |
| `@epa-wg/cem-theme/tokens/figma/*`                | Native Figma library source files (one per mode).   |

Consumers MUST NOT import debug artifacts (`cem.tokens.intermediate.json`, `cem.tokens.resolved.json`).

For AI-assisted styling work, start with `dist/lib/tokens/cem-theme-ai-instructions.md`. It points to the relevant
token specs in `dist/lib/tokens/*.md` and explains how to choose semantic tokens before checking exact names and values
in `cem.tokens.ts`, `cem.tokens.json`, or generated CSS.

## Build & test

```bash
yarn build:theme                                # full theme build via Nx
nx run @epa-wg/cem-theme:build:css              # token CSS only
nx run @epa-wg/cem-theme:build:tokens           # JSON / TS / Figma exports (depends on build:css)
nx run @epa-wg/cem-theme:build:token-platforms  # iOS Swift + Android XML/Compose + per-mode JSON
nx run @epa-wg/cem-theme:test
nx run @epa-wg/cem-theme:lint
```

`build:tokens` depends on `build:css`. `build:token-platforms` depends on `build:tokens`.

## Key paths

| Purpose                                       | Path                                |
|-----------------------------------------------|-------------------------------------|
| Token specs (markdown source of truth)        | `src/lib/tokens/*.md`               |
| AI styling instructions                       | `dist/lib/tokens/cem-theme-ai-instructions.md` |
| Token specs (published Markdown guidance)     | `dist/lib/tokens/*.md`              |
| Token specs (built XHTML, used by validators) | `dist/lib/tokens/*.xhtml`           |
| CSS generators                                | `src/lib/css-generators/*.html`     |
| Generated CSS                                 | `dist/lib/css/*.css`                |
| Generated DTCG / TS / Figma                   | `dist/lib/tokens/`                  |
| iOS Swift output                              | `dist/lib/token-platforms/ios/`     |
| Android Kotlin/XML output                     | `dist/lib/token-platforms/android/` |
| Per-mode flat JSON                            | `dist/lib/token-platforms/json/`    |
| Pipeline scripts                              | `scripts/`                          |

## Related docs

- [Token export architecture](docs/token-export.md) — DTCG export, Figma workflow, platform strategy, output contracts.
- [CEM tokens in Figma](docs/token-figma.md) — native Figma library variable model and UI checks.
- [Docs generation](docs/docs-generation.md) — markdown → XHTML → CSS pipeline.
- [HTML compile workflow](docs/html-compile.md) — package HTML compilation notes.
- [Repository documentation index](../../docs/index.md) — full project map.
