# `@epa-wg/custom-element` Consumer Rewire

This records the Phase 3.6 consumer rewire from the external npm package to the
workspace package in [`../packages/custom-element/`](../packages/custom-element/).

## Decision

The workspace dependency now resolves `@epa-wg/custom-element` through the local
workspace package. Browser source paths that already point at
`node_modules/@epa-wg/custom-element/...` stay stable because Yarn uses the
`node-modules` linker and exposes workspace packages through `node_modules`.

This keeps existing generator HTML source working while making local
`packages/custom-element` edits participate in workspace builds.

## Changes

- Root `package.json` depends on `@epa-wg/custom-element` via `workspace:^`.
- Root IDE `web-types` paths point at `packages/custom-element/ide/...` instead of
  the external package install.
- `packages/cem-theme:build:html` cache inputs track
  `packages/custom-element/custom-element.js` and
  `packages/custom-element/http-request.js`.
- Source HTML imports under `packages/cem-theme/src/` still use
  `node_modules/@epa-wg/custom-element/...` so the dev-server path remains
  stable.
- `tools/scripts/compile-html.mjs` still vendors those browser files into
  `packages/cem-theme/dist/vendor/@epa-wg/custom-element/`, so emitted HTML does
  not load from `node_modules`.
- `yarn install` relinked `node_modules/@epa-wg/custom-element` to
  `packages/custom-element` and removed the npm `0.0.39` lockfile resolution.

## Follow-Up

- Package-local adapter fixtures should later verify that the workspace source and
  vendored `cem-theme` runtime files agree.
- Publish readiness should revisit whether `@epa-wg/custom-element` joins the Nx
  release group or stays manually released for the next-major adoption.
