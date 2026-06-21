# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Release Queue

1. Complete deferred `<cem-element>` demo parity now that the runtime/data-island pieces are in place.
   - Promote the new `packages/cem-elements/index.html` and `packages/cem-elements/demo/*.html` CEM-ML demos to the
     same functional coverage level as the copied `packages/custom-element/index.html` and `demo/*.html` pages.
   - Close the current runtime gaps surfaced by those demos: resource-backed slices for `http-request`,
     `local-storage`, and `location-element`; richer slice expressions and multi-event/multi-target bindings; form-data
     and validation-state capture; focus-preserving DOM merge; scoped style containment; and full demo-backed
     `cem:for-each` data feeds.
   - Implement `<http-request>` as a CEM Elements resource primitive so CEM-ML demos can fetch JSON/XML payloads,
     project response data into slices, and drive `cem:for-each` over response records.
   - Implement `<local-storage>` as a CEM Elements resource primitive with typed text/date/time/number/json coercion,
     initial slice hydration, live storage updates, and write-back from slice changes.
   - Implement `<location-element>` as a CEM Elements resource primitive for reading `window.location`/custom hrefs
     into structured slices, including attributes and query params, plus live location updates where supported.
   - Implement declarative URL writes used by the legacy `set-url` demos, reusing the `location-element` method/src
     model or its CEM-ML equivalent.
   - Implement the accepted UID and scoped CSS design in
     [`cem-ml-uid-and-scoped-css-design.md`](cem-ml-uid-and-scoped-css-design.md):
     - [x] add `uid-seed` declaration support, including explicit blank seed handling and host/default seed resolution;
     - [x] generate `scopeUid` values from encoded seed plus deterministic occurrence path, never from public tag name,
       worker index, runtime randomness, or execution order for persisted/SSR output;
     - [x] stamp generated scope identity on produced render host/root with `data-cem-scope` and keep public tag names only as
       optional debug prefixes;
     - [x] add occurrence-path planning for browser, SSR, and WASM worker-pool render paths, with counter ranges allowed
       only as an internal optimization that preserves occurrence-path-equivalent public IDs;
     - [x] split ephemeral browser runtime from explicit stable-seed output so runtime may use dynamic fallback seeds
       while persisted/SSR output can supply stable `uid-seed` values, occurrence paths, and validation/debug checks;
     - [x] add build/CLI/SSR host transform seed and source-hash fallback integration when no explicit `uid-seed` is
       supplied;
     - [x] implement validation/debug duplicate-ID diagnostics for generated `scopeUid` values in the same output
       scope, without auto-repairing repeatable output with dynamic disambiguators;
     - [ ] extend validation/debug duplicate-ID diagnostics to future generated anonymous custom-element names, stylesheet
       IDs, hydration/render-root IDs, and emitted artifact IDs;
     - [x] implement scoped CSS nesting wrapper output using `[data-cem-scope="..."] { ... }` where native nesting safely
       scopes authored CSS;
     - [x] rewrite scoped CSS `:host` to `&`, treat `:global` and `:root` as `:host` with debug/validation warnings, and
       leave `html`/`body` unchecked;
     - [x] rename scoped `@keyframes` and rewrite `animation-name` plus shorthand `animation` references;
     - [x] suppress scoped CSS `@import` with warning and add diagnostics for unsupported global CSS constructs such as
       `@font-face`, `@property`, `@counter-style`, `@font-palette-values`, `@page`, and unsupported `@namespace`;
     - [x] implement SSR hydration no-op behavior: retain server `data-cem-scope` and generated IDs, skip
       `connectedCallback` DOM updates for hydration-produced bodies, and trust only runtime-owned data-island
       evidence;
     - [x] add event rerender no-op protection for slice events that resolve to unchanged data;
     - [ ] add render-tree diff/no-op replacement protection so unchanged virtual render trees do not mutate browser DOM;
     - [ ] Low priority: design dynamic internal `<textarea>` content handling for SSR and DOM merge; consider an
       `<xsl:element name="textarea">`-style or equivalent CEM-ML templating form where SSR emits loader-friendly
       placeholder markup that can be converted to an actual textarea DOM node capable of merging dynamic parts.
     - [ ] add the design test matrix as executable gates: repeated builds, occurrence paths, parallel scheduling,
       same-tag separate scopes, same-seed collision diagnostics, blank seed, runtime fallback, SSR/browser parity,
       dynamic-data exceptions, hydration no-op, event no-op rerender, scoped CSS isolation, `:host`/`:global`/`:root`,
       keyframes, `@import`, unsupported CSS diagnostics, and public-safe seed examples.
   - Wire the demo parity checks into `yarn nx run cem-elements:verify` once the pages are executable release fixtures.

Completed release-gate phases are recorded in:

- Phase 3.1 `<cem-element>` browser substrate:
  [`../packages/cem-elements/README.md`](../packages/cem-elements/README.md),
  [`../packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md),
  and
  [`../packages/cem-elements/docs/material-parity-inventory.md`](../packages/cem-elements/docs/material-parity-inventory.md).
- Phase 3.2 `@epa-wg/cem-components` primitives:
  [`../packages/cem-components/README.md`](../packages/cem-components/README.md) and
  [`../packages/cem-components/docs/component-reference.md`](../packages/cem-components/docs/component-reference.md).
- Phase 3.5 Edge/SSR processing:
  [`cem-elements-edge-ssr-gate.md`](cem-elements-edge-ssr-gate.md).
- Phase 3.6 `@epa-wg/custom-element` monorepo adoption:
  [`custom-element-migration-scope.md`](custom-element-migration-scope.md),
  [`custom-element-package-baseline.md`](custom-element-package-baseline.md),
  [`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md),
  [`custom-element-consumer-rewire.md`](custom-element-consumer-rewire.md), and
  [`release-readiness-0.1.0.md`](release-readiness-0.1.0.md).

Current verification commands:

- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`
- `yarn nx run cem-elements:verify-edge-ssr`
- `yarn nx run @epa-wg/custom-element:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
