# CEM - Consumer Semantic Material Theme and custom-element components library

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

A theme system and custom-element Material component library for building declarative, no-JavaScript web applications.

CEM reinterprets Google’s [Material Design Guidelines](https://material.io/design) through a consumer-first
lens—focusing on how users perceive and interact with interfaces, rather than how designers construct them.

It is implemented as a combination of:

- [AI instructions](packages/cem-theme/src/lib/tokens) AI instructions for generating and adapting themes
- [CSS design tokens and stylesheets](https://unpkg.com/@epa-wg/cem-theme/dist/lib/css-generators/index.html)
- Web Components for use in fully declarative applications (no JS required)

[CEM POC - custom-element](https://unpkg.com/@epa-wg/custom-element/index.html) |
[CEM elements lib POC](https://unpkg.com/@epa-wg/custom-element-dist/src/material/components.html)

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
- [Completed todo history](docs/archive/todo-completed.md) — archived execution
  rationale and verification evidence.
- [`cem-element` design](docs/cem-element-design.md) — successor substrate for `@epa-wg/custom-element`: data
  islands, event wiring, render loop, follow-up adoption sequencing, and parity gates.
- [Angular Material parity inventory](packages/cem-components/docs/angular-material-parity.md) — version-pinned
  product UI benchmark for the styled `cem-components` layer, distinct from barebone `cem-elements` runtime parity.
- [Token export architecture](packages/cem-theme/docs/token-export.md)
- [CEM ML parser/runtime acceptance criteria](docs/cem-ml-ac.md)
- [CEM ML CLI feature summary](docs/cem-ml-cli-contract.md)
- [NPM publishing workflow](docs/npm-publish.md)

# Package map

The component stack has two deliberately separate layers:

- `@epa-wg/cem-elements` is the barebone layer. It supplies `<cem-element>` as
  the declarative component base and browser/API primitives such as URL and HTTP
  resource access. It does not own Material UI parity or Consumer Semantic Theme
  styling.
- `@epa-wg/cem-components` is the Material-superset UI layer built on
  `cem-elements`. It owns public UI components plus their Consumer Semantic
  Theme styling, state, keyboard, accessibility, forced-colors, and workflow
  contracts.

Runtime/template compatibility fixtures in `cem-elements` therefore do not, by
themselves, establish product-component parity in `cem-components`.

Publication metadata is authoritative. The CEM Site's
[`/packages/`](apps/cem-site/README.md) projection and verification gate derive
the same inventory independently, so a newly publishable package or crate must
gain an owner-matched documentation route in the same change.

## Public npm packages

| Package                                                       | Purpose                                                                                                              |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| [`@epa-wg/cem-components`](packages/cem-components/README.md) | Material-superset declarative UI components, behaviors, accessibility contracts, and public catalog.                 |
| [`@epa-wg/cem-elements`](packages/cem-elements/README.md)     | Barebone `<cem-element>` substrate, processing host, and browser resource primitives.                                |
| [`@epa-wg/cem-ml-cli`](packages/cem-ml-cli-npm/README.md)     | Synchronized browser and Node CLI deployment with bounded worker hosts and a main-thread fallback.                   |
| [`@epa-wg/cem-ml`](packages/cem-ml-npm/README.md)             | Low-level synchronized CEM-ML WASM runtime, loaders, declarations, schema assets, and integrity metadata.            |
| [`@epa-wg/cem-theme`](packages/cem-theme/README.md)           | Canonical token specs plus generated CSS, DTCG JSON, TypeScript metadata, platform outputs, and adapter projections. |
| [`@epa-wg/custom-element`](packages/custom-element/README.md) | Production declarative custom-element compatibility package retained beside the successor `cem-elements` substrate.  |
| [`@epa-wg/trang-native`](packages/trang-native/README.md)     | Self-contained native Trang deployment used for RELAX NG parity without a consumer JRE.                              |

## Public Cargo crates

| Crate                                                                   | Purpose                                                                                                  |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| [`cem-ml`](packages/cem_ml/README.md)                                   | Shared parser, validation, lifecycle, transformation, reporting, scheduling, and host-service semantics. |
| [`cem-ml-cli`](packages/cem_ml_cli/README.md)                           | Native Rust command host and `cem-ml` executable.                                                        |
| [`cem-ml-transform-cem-ql`](packages/cem_ml_transform_cem_ql/README.md) | Adapter between the CEM-ML transform lifecycle and CEM-QL compilation/rendering.                         |
| [`cem-ql`](packages/cem_ql/README.md)                                   | Query-language compiler, evaluator, template renderer, artifact runtime, and WASM boundary.              |

# Use the token CSS

The generated CSS exposes every CEM token as a CSS custom property on `:root`. Drop it into any page and consume
tokens via `var(--cem-...)`.

| File                            | When to use                                                                                                                                    |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
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

## Prompt for applying CEM styling to an existing project

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
<link rel="stylesheet" href="https://unpkg.com/@epa-wg/cem-theme@latest/dist/lib/css/cem-combined.css" />

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
