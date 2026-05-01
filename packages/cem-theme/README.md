# `@epa-wg/cem-theme`

Canonical CEM token specs, generated CSS, DTCG JSON, TypeScript metadata, native iOS/Android outputs, and Figma
library files. Token specs in `src/lib/tokens/*.md` are the single source of truth; everything in `dist/` is generated.

## Install

```bash
yarn add @epa-wg/cem-theme
```

## Primary exports

| Path | Contents |
| ---- | -------- |
| `@epa-wg/cem-theme` | Theme entry (TypeScript). |
| `@epa-wg/cem-theme/tokens/cem.tokens.json` | Canonical DTCG-compatible visual tokens. |
| `@epa-wg/cem-theme/tokens/cem.voice.tokens.json` | Voice/audio metadata, separate from visual outputs. |
| `@epa-wg/cem-theme/tokens/cem.tokens.ts` | Token names + metadata for docs/tests/autocomplete. |
| `@epa-wg/cem-theme/tokens/cem.tokens.report.json` | Portability and skipped-token report. |
| `@epa-wg/cem-theme/tokens/figma/*` | Native Figma library source files (one per mode). |

Consumers MUST NOT import debug artifacts (`cem.tokens.intermediate.json`, `cem.tokens.resolved.json`).

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

| Purpose | Path |
| ------- | ---- |
| Token specs (markdown source of truth) | `src/lib/tokens/*.md` |
| Token specs (built XHTML, used by validators) | `dist/lib/tokens/*.xhtml` |
| CSS generators | `src/lib/css-generators/*.html` |
| Generated CSS | `dist/lib/css/*.css` |
| Generated DTCG / TS / Figma | `dist/lib/tokens/` |
| iOS Swift output | `dist/lib/token-platforms/ios/` |
| Android Kotlin/XML output | `dist/lib/token-platforms/android/` |
| Per-mode flat JSON | `dist/lib/token-platforms/json/` |
| Pipeline scripts | `scripts/` |

## Related docs

- [Token export architecture](docs/token-export.md) — DTCG export, Figma workflow, platform strategy, output contracts.
- [CEM tokens in Figma](docs/token-figma.md) — native Figma library variable model and UI checks.
- [Docs generation](docs/docs-generation.md) — markdown → XHTML → CSS pipeline.
- [HTML compile workflow](docs/html-compile.md) — package HTML compilation notes.
- [Repository documentation index](../../docs/index.md) — full project map.
