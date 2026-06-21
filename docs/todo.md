# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Release Queue

1. Complete deferred `<cem-element>` demo parity now that the runtime/data-island pieces are in place.
   - Promote the new `packages/cem-elements/index.html` and `packages/cem-elements/demo/*.html` CEM-ML demos to the
     same functional coverage level as the copied `packages/custom-element/index.html` and `demo/*.html` pages.
   - Close the current runtime gaps surfaced by those demos: focus-preserving DOM merge; scoped style containment; and
     full demo-backed `cem:for-each` data feeds.
   - Implement Phase 1 of `<http-request>` as a CEM Elements resource primitive per
     [`cem-elements-http-request-design.md`](cem-elements-http-request-design.md) so CEM-ML demos can fetch JSON/XML
     payloads, project response data into slices, and drive `cem:for-each` over response records:
     - [x] Add `http-request` declaration parsing and lowering from the legacy HTML spelling and CEM-ML resource form.
     - [x] Add resource-specific resolver and loader runtime hooks instead of overloading `resolveModuleUrl`.
     - [x] Resolve `@url` through the scoped URL/module-map pipeline, including unresolved bare-specifier diagnostics.
     - [x] Enforce Phase 1 policy defaults: `GET`/`HEAD` only, bounded response size/request timeout/redirects,
       host-controlled direct network access, and unsupported content-type diagnostics.
     - [x] Implement the resource slot envelope under `datadom.slices.<slice>` with pending/headers/complete/error/aborted
       states, request metadata, response metadata, diagnostics, resource revision, and no live host objects.
     - [x] Parse JSON responses into a CEM-QL-navigable AST/projection with content-type and parse diagnostics.
     - [x] Add Phase 1 source-id hooks to JSON response projections and diagnostics.
     - [x] Parse XML/XHTML responses into the same resource slot contract with source-id/diagnostic hooks.
     - [x] Trigger async rerender on resource completion while aborting stale requests by revision id and relying on
       render-tree no-op protection for unchanged output.
     - [x] Add an explicit resource-settled runtime/test hook so fixtures do not wait with timing sleeps.
     - [x] Add a local JSON fixture that proves `cem:for-each` over response records in demo parity pages.
     - [x] Add a local XML fixture that proves `cem:for-each` over response records in demo parity pages.
     - [x] Confirm `packages/cem-elements/index.html` does not need duplicate resource-backed `<http-request>` coverage for
       Phase 1 because the dedicated `demo/http-request.html` fixture now covers JSON/XML resource records.
     - [x] Keep the published standalone `http-request.js` companion shim smoke-tested as compatibility surface.
     - [ ] Defer progressive AST streaming, full browser source-map sidecars, cache identity, SSR preload/revalidation, and
       legacy broad XPath rewrite compatibility to later phases of the design.
   - [x] Implement `<local-storage>` as a CEM Elements resource primitive with typed text/date/time/number/json coercion,
     initial slice hydration, live storage updates, and write-back from slice changes.
   - [x] Implement `<location-element>` as a CEM Elements resource primitive for reading `window.location`/custom hrefs
     into structured slices, including attributes and query params, plus live location updates where supported.
   - [x] Implement declarative URL writes used by the legacy `set-url` demos, reusing the `location-element` method/src
     model or its CEM-ML equivalent.
   - [x] Implement richer slice expressions and multi-event/multi-target bindings for legacy data-slices parity:
     whitespace-separated event lists, `init`, `//slice` lookup, event/target aliases, numeric `+`/`-`, `concat(...)`,
     and `slice="a|b"` fan-out.
   - [x] Implement form-data extraction and validation-state capture for rendered forms, projecting them through
     `datadom.formData`, `datadom.validationState`, and a legacy-style mirror under `datadom.slices.<form>.formData`.
   - [x] Implement custom-validity expression application for rendered forms and controls, including validation-message
     propagation compatible with the legacy form demos.
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
     - [x] add render-tree diff/no-op replacement protection so unchanged virtual render trees do not mutate browser DOM;
     - [x] implement property-first sequential DOM merge for materialized render-plan fragments, including
       `node.cemRenderNodeId`, SSR/debug marker mirroring from `data-cem-render-node-id`, retained element/text/comment
       updates, temporary same-parent render-id lookahead for reordered elements, and nested CEM-owned subtree
       preservation;
     - [x] extend DOM merge from fragment parity to direct render-plan patch application with explicit comment-range
       dynamic regions and transaction-level `replaceScope` recovery diagnostics;
     - [x] extend direct render-plan patching to runtime directive setup for `slice-event` and `module-url`, then retire
       the materialized-fragment preprocessing fallback for rerenders that contain runtime directives;
     - [x] add the design test matrix as executable gates: repeated builds, occurrence paths, parallel scheduling,
       same-tag separate scopes, same-seed collision diagnostics, blank seed, runtime fallback, SSR/browser parity,
       dynamic-data exceptions, hydration no-op, event no-op rerender, scoped CSS isolation, `:host`/`:global`/`:root`,
       keyframes, `@import`, unsupported CSS diagnostics, and public-safe seed examples.
     - [ ] Deferred: implement and cross-browser validate dynamic internal `<textarea>` handling using an invisible
       child-node merge model plus explicit `.value` projection; include SSR loader conversion from a
       loader-friendly `<xsl:element name="textarea">`-style or equivalent CEM-ML placeholder form.
   - [x] Wire the demo parity checks into `yarn nx run cem-elements:verify` once the pages are executable release
     fixtures.

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
