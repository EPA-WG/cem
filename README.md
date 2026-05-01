# CEM - Consumer Semantic Material Theme and custom-element components library

A theme system and custom-element component library for building declarative, no-JavaScript web applications.

CEM reinterprets Google’s [Material Design Guidelines](https://material.io/design) through a consumer-first
lens—focusing on how users perceive and interact with interfaces, rather than how designers construct them.

It is implemented as a combination of:

* [AI instructions](packages/cem-theme/src/lib/tokens) AI instructions for generating and adapting themes
* CSS design tokens and stylesheets
* Web Components for use in fully declarative applications (no JS required)

The result is a system where consumer semantics drive UI behavior and appearance, enabling consistent, accessible, and
maintainable interfaces.

[![npm version](https://badge.fury.io/js/%40epa-wg%2Fcem-theme.svg)](https://badge.fury.io/js/%40epa-wg%2Fcem-theme)
[![Downloads](https://img.shields.io/npm/dm/@epa-wg/cem-theme.svg)](https://www.npmjs.com/package/@epa-wg/cem-theme)
[![License](https://img.shields.io/npm/l/@epa-wg/cem-theme.svg)](./LICENSE)

# Figma design library

The CEM UI Kit is the Figma-native design library for CEM tokens, foundations, components, patterns, and QA fixtures.
Its Tokens page contains the native Figma Variables collection and visual token demos generated from the same source
artifacts as the CSS generator pages.

- [CEM UI Kit Tokens page](https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit?node-id=2-24&t=QQwTKeMg0v9dTQ10-1)
- [Figma token workflow](packages/cem-theme/docs/token-figma.md)

# Project documentation

- [Documentation index](docs/index.md) — canonical map of every CEM doc, report, and example.
- [Roadmap](roadmap.md) — product/module sequencing and delivery phases.
- [Todo](docs/todo.md) — current execution checklist.
- [Token export architecture](packages/cem-theme/docs/token-export.md)
- [CEM DOM library acceptance criteria](docs/cem-dom-ac.md)
- [NPM publishing workflow](docs/npm-publish.md)

# Package map

| Package | Status | Purpose |
| ------- | ------ | ------- |
| [`@epa-wg/cem-theme`](packages/cem-theme/README.md) | published | Canonical token specs, generated CSS, DTCG JSON, TypeScript metadata, native (iOS/Android) outputs, and Figma library files. |
| [`@epa-wg/cem-components`](packages/cem-components/README.md) | shell | Declarative custom-element primitives that consume the theme. Component implementations land in Phase 3. |
| `@epa-wg/cem-dom` | planned (Phase 2) | Schema, parser, validator, and XSLT-style transforms for CEM semantic documents. See [acceptance criteria](docs/cem-dom-ac.md). |

# Use the token CSS

The generated CSS exposes every CEM token as a CSS custom property on `:root`. Drop it into any page and consume
tokens via `var(--cem-...)`.

| File | When to use |
| ---- | ----------- |
| `dist/lib/css/cem-combined.css` | Single concatenated file. One HTTP request — best for `<link>` and CDN delivery. |
| `dist/lib/css/cem.css` | `@import` index over per-spec files (`cem-colors.css`, `cem-dimension.css`, …). Best when a tool resolves `@import` and you want tree-shaking. |

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

## Via unpkg CDN (no install)

```html
<!-- pin a specific version -->
<link rel="stylesheet" href="https://unpkg.com/@epa-wg/cem-theme@0.0.9/dist/lib/css/cem-combined.css" />

<!-- or float to latest -->
<link rel="stylesheet" href="https://unpkg.com/@epa-wg/cem-theme@latest/dist/lib/css/cem-combined.css" />
```

The same paths work for individual specs, e.g.
`https://unpkg.com/@epa-wg/cem-theme@latest/dist/lib/css/cem-colors.css`.

# Quickstart

```bash
yarn install
yarn start                # serves docs/lib at http://localhost (dev server)
yarn build                # builds every package via Nx
yarn build:theme          # build just the theme package
yarn build:css            # regenerate token CSS only
yarn lint                 # lint every package
nx run @epa-wg/cem-theme:test
```

The dev server is required for the custom-element templates — they use `fetch()` and `<http-request>`, both of which
break under `file://`.

# Release

Releases follow [`docs/npm-publish.md`](docs/npm-publish.md). The release flow runs `yarn publish:prepare`, drives the
Nx release pipeline, and refreshes the Figma kit afterwards. Pass `--dry-run` to any release command to preview without
publishing.
