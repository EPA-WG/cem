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
