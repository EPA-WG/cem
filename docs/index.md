# CEM Documentation Index

This index links the active project, release, and token workflow documents.

## Project Planning

- [Roadmap](../roadmap.md) — product/module sequencing and major delivery phases.
- [Todo](./todo.md) — remaining execution tasks only.
- [CEM ML library plan](./cem-ml-library-plan.md) — Rust parser/runtime ownership for canonical CEM-ML plus XML/HTML
  parity responsibilities.
- [CEM ML acceptance criteria](./cem-ml-ac.md) — testable AC for the parser, schema, mutations, and plugin runtime.
- [CEM QL acceptance criteria](./cem-ql-ac.md) — testable AC for the CEM-ML query language consumed by templates and
  validation.
- [CEM QL stack design](./cem-ql-stack-design.md) — high-level design: pipeline layers, grammar, evaluator IR, type
  system, stdlib module layout, cost model, and binary artifact layout.
- [CEM QL implementation design](./cem-ql-stack-design-impl.md) — concrete Rust module map, surface AST, IR shapes,
  diagnostic table, and stdlib function tables.
- [CEM ML CLI feature summary](./cem-ml-cli-contract.md) — planned CLI capabilities, options, reports, diagnostics,
  fixture workflows, and exit codes.
- [CEM ML CLI plan](./cem-ml-cli-plan.md) — Rust `cem-ml` CLI implementation plan with `cem-ml` library separation.
- [CEM ML Phase 2 run-config audit](./cem-ml-phase2-run-config-audit.md) — current `cem_ml` and `cem_ml_cli`
  run-config/lifecycle surfaces mapped to the Phase 2 parser/runtime contract.
- [CEM ML parser/schema ADR](./cem-ml-parser-schema-adr.md) — Phase 1 parser engine, schema mirror, source-location,
  security, and WASM decisions.
- [CEM ML syntax](./cem-ml-syntax.md) — canonical `{name @attributes | content...}` syntax and XML convention
  parity examples.
- [CEM component MVP](./component-mvp.md) — first component list and state matrix.
- [`cem-element` design](./cem-element-design.md) — custom-element successor substrate, `<template>` data islands,
  event-to-data wiring, render loop, UI/processing split, migration path, and parity gates.
- [`cem-element` WASM proposal](./cem-element-wasm-proposal.md) — options for using `cem_ml` WASM, inline and URI
  declaration templates, module-map resolution, streaming source adapters, host runtime support, patch-frame streams,
  worker-pool scheduling, edge processing, and server-side rendering.
- [CEM-ML resource lifecycle](./cem-ml-resource-lifecycle.md) — portable resource and asset lifecycle states, AST stream
  revisions, renderability, diagnostics, hydration, and de-hydration for the CEM-ML stack.
- [`cem-element` external resource loading contract](./cem-element-src-loading-contract.md) — `src="#id"`,
  `src="url"`, `src="url#id"`, and `http-request url` loading, module-map treatment, content-type handling, CEM-ML
  lifecycle binding, artifact/AST stream handling, and security context.
- [CEM-ML UID and scoped CSS design](./cem-ml-uid-and-scoped-css-design.md) — generated identity requirements for
  scoped CSS, anonymous declarations, public-tag debug prefixes, deterministic UIDs, hydration, and validation gates.
- [CEM Elements HTTP request resource design](./cem-elements-http-request-design.md) — substrate-backed
  `<http-request>` resource slices, CEM-ML lifecycle binding, streaming content-type parsing, AST resource streams, and
  data source maps.
- [CEM UI Kit plan](./figma-ui-kit-plan.md) — Figma page, token, component, and QA mapping.
- [Token pipeline smoke](./token-pipeline-smoke.md) — full propagation check for a one-token source change.

## Release

- [NPM publishing workflow](./npm-publish.md) — release, publish, post-release Figma refresh, and branch sync steps.

## Theme And Token Pipeline

- [CEM theme package](../packages/cem-theme/README.md) — package-level build and test notes.
- [Token export architecture](../packages/cem-theme/docs/token-export.md) — DTCG export, Figma workflow, platform
  strategy, risks, and output contracts.
- [CEM tokens in Figma](../packages/cem-theme/docs/token-figma.md) — native Figma library variable model and UI checks.
- [Docs generation](../packages/cem-theme/docs/docs-generation.md) — markdown, XHTML, CSS, token, and docs pipeline.
- [HTML compile workflow](../packages/cem-theme/docs/html-compile.md) — package HTML compilation notes.

## Components

- [CEM components package](../packages/cem-components/README.md) — package-level build and test notes.
- [Component reference](../packages/cem-components/docs/component-reference.md) — MVP component semantics, token
  families, states, and accessibility notes.
- [Component conventions](../packages/cem-components/docs/conventions.md) — host API, attributes, events, forms,
  validation, loading, and progressive enhancement.
- [Light-DOM rendering rules](../packages/cem-components/docs/light-dom-rendering.md) — no shadow DOM, data-island
  isolation, slot projection, render lifecycle, and substrate compatibility.
- [Accessibility contract](../packages/cem-components/docs/accessibility.md) — names, ARIA, focus, keyboard behavior,
  live regions, unsafe content, and verification.
- [Component examples](../packages/cem-components/examples/README.md) — package-local workflow examples, separate from
  executable tests.

## Parser Runtime

- [`@epa-wg/cem-ml`](../packages/cem_ml/Cargo.toml) — active Rust parser/runtime library (Cargo crate `cem-ml`) for
  canonical CEM-ML plus XML/HTML parity inputs.
- [`@epa-wg/cem-ml-cli`](../packages/cem_ml_cli/Cargo.toml) — active Rust CLI (Cargo crate `cem-ml-cli`, binary
  `cem-ml`) for parsing, validation, reports, fixtures, and migration workflows.

## Native Outputs

Generated by `nx run @epa-wg/cem-theme:build:token-platforms` from the canonical token spine.

- [iOS Swift report](../packages/cem-theme/dist/lib/token-platforms/ios/ios-report.md) — `CEMTokens.swift` plus
  `CEMTokens.xcassets-hints.json` for asset-catalog wiring.
- [Android report](../packages/cem-theme/dist/lib/token-platforms/android/android-report.md) — `values/`,
  `values-night/`, and `compose/` outputs.
- [Per-mode JSON report](../packages/cem-theme/dist/lib/token-platforms/json/json-report.md) — flat
  `cem-tokens-{light,dark,contrast-light,contrast-dark,native}.json` for adapter experiments.

## Examples

- [Figma token workflow](../examples/figma/README.md) — native Figma library setup, validation, screenshot, and prompts.
- [Figma sample token application](../examples/figma/sample-token-application.md) — fixture for applying variables to
  a button and card.
- [Canonical CEM-ML fixture set](../examples/cem-ml/README.md)
- [Canonical CEM-ML login fixture](../examples/cem-ml/login.cem)
- [Canonical CEM-ML registration fixture](../examples/cem-ml/registration.cem)
- [Canonical CEM-ML profile fixture](../examples/cem-ml/profile.cem)
- [Canonical CEM-ML assets list fixture](../examples/cem-ml/assets-list.cem)
- [Canonical CEM-ML message thread fixture](../examples/cem-ml/message-thread.cem)
- [Semantic HTML login parity fixture](../examples/semantic/login.html)
- [Semantic HTML registration parity fixture](../examples/semantic/registration.html)
- [Semantic HTML profile parity fixture](../examples/semantic/profile.html)
- [Semantic HTML assets list parity fixture](../examples/semantic/assets-list.html)
- [Semantic HTML message thread parity fixture](../examples/semantic/message-thread.html)
