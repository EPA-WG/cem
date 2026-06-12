# `@epa-wg/custom-element` Adapter Runtime Packaging

This records the packaging gate discovered before replacing
`packages/custom-element/custom-element.js` with the `CemElementRuntime` adapter.
It extends [`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md).

## Problem

`custom-element.js` is a public browser entrypoint. Existing consumers load it
directly:

```html
<script type="module" src=".../@epa-wg/custom-element/custom-element.js"></script>
```

The adapter boundary says `custom-element.js` should delegate to
`CemElementRuntime`, but `CemElementRuntime` currently lives in the
`@epa-wg/cem-elements` package and imports the internal `cem_ql` WASM runtime
support. A direct source-level import would create one of two failures:

- a bare import such as `import { CemElementRuntime } from '@epa-wg/cem-elements'`
  fails in plain browser modules without an import map or bundler;
- a monorepo-relative import can work for local source URLs but does not produce a
  portable npm package artifact unless the runtime and WASM files are vendored or
  bundled with stable relative URLs.

Do not swap the adapter implementation until this packaging path is in place and
verified in browser fixtures.

## Recommended Path

Build a browser-ready substrate runtime bundle for `@epa-wg/custom-element`.

The package build should:

- keep `custom-element.js` as the public browser entrypoint;
- make `custom-element.js` import a package-local runtime module with a relative
  URL, not a bare package specifier;
- stage the required `@epa-wg/cem-elements` runtime code and `cem_ql` WASM assets
  into `dist/` with stable relative paths;
- keep source and dist browser smoke fixtures green;
- keep `cem-theme` source HTML imports unchanged so `compile-html` can continue
  vendoring `node_modules/@epa-wg/custom-element/custom-element.js`.

The exact build shape can be either:

| Option | Shape | Tradeoff |
| --- | --- | --- |
| Vendored modules | Copy `cem-elements/dist/**` and `cem_ql/dist/wasm/**` under `packages/custom-element/dist/vendor/` and rewrite adapter imports to relative vendor paths. | Simple and transparent, but package contents include internal runtime files. |
| Bundled JS plus WASM asset | Use the workspace bundler to emit one browser runtime JS module plus the WASM asset. | Cleaner package surface, but introduces bundle config and must preserve WASM URL loading. |
| Runtime-support package | Extract the shared runtime to a public browser-ready package, then import through documented CDN/import-map paths. | Architecturally clean later, but too large for the first adapter implementation. |

Choose vendored modules first unless the bundler path is already available with
minimal config. It is the least disruptive way to prove the adapter in Phase 3.6.

## Verification Requirements

Before replacing the legacy `custom-element.js` engine:

- `@epa-wg/custom-element:test` must load the adapter from workspace source and
  from `dist/` in Chromium;
- the source and dist fixtures must prove untyped legacy templates route through
  `custom-element-v0`;
- the fixture must assert the registered `custom-element` class is the exported
  `CustomElement` adapter class;
- rendered output must show substrate metadata such as
  `data-cem-template-artifact` or the data-island template marker;
- a static verifier must assert the adapter source no longer contains
  `XSLTProcessor`, `createXsltFromDom`, `DceElement`, or a package-local render
  loop;
- `cem-theme:build:html` must still copy the browser entry into
  `dist/vendor/@epa-wg/custom-element/` and emitted HTML must not reference
  `node_modules`.

## Follow-Up TODO

- Add a package-local runtime staging step to `scripts/build-package.mjs`.
- Add a source/dist browser fixture that imports the staged runtime path.
- Replace `custom-element.js` with the adapter once the runtime path is stable.
- Update `custom-element.d.ts` to expose the adapter helpers and remove stale
  XSLT-specific helper declarations.
