# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

Begin
[`Phase 3 - Custom-Element Runtime`](../roadmap.md#phase-3---custom-element-runtime)
by locking the declaration/registration contract for the
`@epa-wg/cem-elements` substrate before extending its browser runtime.

Phase 2.6 is complete. Its checklist is archived in
[`archive/todo-completed-2026-08-18.md`](archive/todo-completed-2026-08-18.md),
and the protected `0.1.0-rc.2` release, remote-byte verification, lane isolation,
and same-run recovery evidence are recorded in
[`cem-ml-release-rehearsal-0.1.0-rc.2.md`](cem-ml-release-rehearsal-0.1.0-rc.2.md).

The Phase 3 WASM topology is already decided: Option B, one dedicated browser
worker, is primary; Option A, main-thread WASM, is the required fallback. The
remaining immediate ambiguity is the relationship between the logical scoped
CEM template registry and the browser custom-elements registry, plus the
uniqueness/collision rules for produced tag names. Do not implement registration
behavior until the first checklist item resolves that boundary.

## Phase 3 Checklist

- [ ] Lock the `<cem-element>` declaration and registration contract.
    - [ ] Decide whether CEM declarations use a scoped, inherited logical template
          registry while produced browser custom elements remain registered in the
          document's global `customElements` registry.
    - [ ] Decide which names are globally unique, which references may be
          scope-local, and how duplicate declarations, inherited shadowing, and
          collisions with legacy `<custom-element>` declarations fail.
    - [ ] Reconcile the decision with `AC-R-1` through `AC-R-3`, the reserved
          `cem-` component namespace, and the coexistence rule that browser tag
          names must not collide.
    - [ ] Promote the accepted rules into `docs/cem-element-design.md` and
          executable contract tests; remove the two blocking `TBD` statements.
    - Recommended direction: scope and inherit CEM template/declaration lookup,
      keep browser `customElements` registration global, require globally unique
      public produced tag names, and make same-scope duplicates or incompatible
      inherited shadowing deterministic errors. This preserves the scoped CEM
      model without pretending the default browser registry is scoped.

- [ ] Audit the existing Phase 3 substrate against the locked contract.
    - [ ] Classify current `cem-elements` declaration-shape, data-document,
          disposition, projection, processing-boundary, runtime-support, Storybook,
          legacy, material, and edge/SSR fixtures as implemented, partial,
          placeholder, or deferred.
    - [ ] Map every current resolved Nx target to the Phase 3A/3B/3C roadmap and
          move edge/SSR-only acceptance out of the Phase 3 browser gate where
          necessary.
    - [ ] Add an explicit todo checkitem before adding any new parity or browser
          fixture discovered by the audit.

- [ ] Implement the smallest tests-first Phase 3A browser vertical slice.
    - [ ] Register one inline `<cem-element>` declaration under the locked name
          rules and capture one produced instance's author payload into an inert
          WHATWG template data island.
    - [ ] Compile/render through the existing CEM-ML/CEM-QL WASM boundary using
          one dedicated worker, with the same semantic result through the required
          main-thread fallback.
    - [ ] Apply revision-checked patch frames on the main thread while preserving
          light-DOM identity, focus/selection state, and data-island isolation.
    - [ ] Prove the vertical slice in Rust-first contract tests, TypeScript unit
          tests, and one executable Storybook/browser fixture.

- [ ] Add URI declarations and the Phase 1 `<http-request>` resource slice.
    - [ ] Support declaration `src` for document-relative, fragment-only, absolute,
          and module-map identities under the host resolver and scope policy.
    - [ ] Add remote/local streaming, abort/stale-response protection, JSON/XML
          projections, and the fixture-backed `cem:for-each` flow.
    - [ ] Preserve the same artifact identity, worker/fallback semantics, source
          maps, diagnostics, and patch protocol as inline declarations.

- [ ] Complete Phase 3A/3B/3C substrate parity.
    - [ ] Prove legacy compatibility only through opt-in
          `lang="custom-element-v0"` fixtures.
    - [ ] Prove the full legacy and material parity inventories plus browser
          data-island isolation and accessibility gates.
    - [ ] Add the Phase 3B scope-policy worker pool, content-addressed cache, and
          deterministic scheduling traces behind the stable host API.
    - [ ] Add Phase 3C precompiled component-template artifacts without removing
          the source-driven runtime path.

- [ ] Author the Phase 3 primitive set exclusively on the accepted substrate.
    - [ ] Extend the existing component test harness for substrate rendering,
          events, forms, accessibility, and visual snapshots.
    - [ ] Wire action, field, surface, text, icon, stack, grid, list, nav, and
          dialog shell primitives through `<cem-element>` with no legacy runtime
          dependency.
    - [ ] Run the substrate, component, CEM-ML fixture, and accessibility aggregate
          gates before closing Phase 3.

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Figma UI Kit work remains Phase 5,
and Swift/Xcode plus Kotlin/Compose compile gates remain Phase 8.
