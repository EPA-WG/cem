# `@epa-wg/custom-element` Migrated Package Gate

This records the first Phase 3.6 cross-package verification pass after
`custom-element.js` moved to the `CemElementRuntime` adapter.

## Green Gates

The migrated browser substrate gates are green:

- `yarn nx run cem-elements:verify`
  - Storybook browser parity stories passed.
  - `cem_ml_cli:validate-fixtures` passed with zero hard violations.
  - `cem_ml_cli:e2e` passed.
  - `cem-elements:verify-substrate` passed the substrate fixture roundtrip set.
- `yarn nx run @epa-wg/custom-element:verify`
  - package build/test/lint/verify passed;
  - source and `dist/` browser smoke fixtures passed;
  - adapter registration, legacy-v0 normalization, omitted-`tag` inline rendering,
    companion module behavior, release-pack runtime staging, and no-XSLT regression
    guards passed.
- `yarn nx run @epa-wg/cem-theme:build:html`
  - HTML compilation passed;
  - emitted HTML has no `node_modules` runtime references;
  - `dist/vendor/@epa-wg/` includes `custom-element`, `cem-elements`, and `cem_ql`
    WASM runtime files.

## Blocking Gate

`yarn nx run @epa-wg/cem-theme:verify:phase13` still fails in the CSS generation
phase.

The generator pages now render enough for `capture-xpath-text.mjs` to find
`code[data-generated-css]`, but the captured CSS is empty. Manifest validation then
reports every token missing, starting with `cem-colors.css`.

Root cause: `packages/cem-theme/src/lib/css-generators/*.html` still use legacy
XSLT+XPath template constructs:

- `<variable>`;
- `<for-each>`;
- broad XPath selections over fetched token XHTML;
- XSLT conditionals and text helpers.

The Phase 3.6 migration now carries two explicit options:

- Option A: keep XSLT+XPath with the legacy HTML/XSLT default-namespace behavior;
- Option B: convert the XSLT+XPath logic to CEM-ML+CEM-QL and mark converted
  migration templates with `<template type="cem-ml-v0">`.

The planned CSS generation path is Option B after conversion. The current adapter
supports the fixture-bounded legacy-v0 surface, omitted-`tag` inline rendering, and
substrate-backed data islands, but it does not evaluate the full generator
templates yet.

## Next Task

Choose and execute the `cem-theme` CSS generator migration option before this gate
can close.

Acceptable paths:

- Option B, recommended: convert each CSS generator to
  `<template type="cem-ml-v0">` and CEM-QL, then use that CEM-ML version in the CSS
  generation phase;
- Option A: keep the legacy XSLT+XPath generator runtime with default-namespace
  behavior and document it as a named compatibility runtime;
- replace browser-template CSS extraction with a Node generator only if the
  converted CEM-ML/CEM-QL path is rejected.

Do not silently reintroduce `XSLTProcessor` into `custom-element.js`. If Option A
is selected, make it explicit and adjust verifier expectations accordingly.
