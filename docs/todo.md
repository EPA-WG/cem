# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

Complete the full legacy and material acceptance inventories on the stable Phase 3A
worker/fallback boundary, including browser data-island isolation and accessibility,
before Phase 3B scheduling or Phase 3C precompilation.

Phase 2.6 is complete. Its checklist is archived in
[`archive/todo-completed-2026-08-18.md`](archive/todo-completed-2026-08-18.md),
and the protected `0.1.0-rc.2` release, remote-byte verification, lane isolation,
and same-run recovery evidence are recorded in
[`cem-ml-release-rehearsal-0.1.0-rc.2.md`](cem-ml-release-rehearsal-0.1.0-rc.2.md).

The Phase 3 WASM topology is already decided: Option B, one dedicated browser
worker, is primary; Option A, main-thread WASM, is the required fallback. The
registration boundary is also locked: logical declaration lookup is scoped and
inherited, browser registration remains document-global, and only identical CEM
registration identities may reuse an inherited or existing definition.

## Phase 3 Checklist

- [x] Lock the `<cem-element>` declaration and registration contract.
    - [x] Decide whether CEM declarations use a scoped, inherited logical template
          registry while produced browser custom elements remain registered in the
          document's global `customElements` registry.
    - [x] Decide which names are globally unique, which references may be
          scope-local, and how duplicate declarations, inherited shadowing, and
          collisions with legacy `<custom-element>` declarations fail.
    - [x] Reconcile the decision with `AC-R-1` through `AC-R-3`, the reserved
          `cem-` component namespace, and the coexistence rule that browser tag
          names must not collide.
    - [x] Promote the accepted rules into `docs/cem-element-design.md` and
          executable contract tests; remove the two blocking `TBD` statements.
    - Completed 2026-08-18: adopted scoped/inherited logical lookup with
      document-global browser registration. Same-scope duplicates and incompatible
      inherited, legacy, foreign, or existing-CEM definitions fail before browser
      mutation; identical inherited or existing CEM registration identities reuse
      the one global definition. The pure decision core and 7 focused contract
      cases pass as part of all 94 `cem-elements:test:unit` tests.

- [x] Audit the existing Phase 3 substrate against the locked contract.
    - [x] Classify current `cem-elements` declaration-shape, data-document,
          disposition, projection, processing-boundary, runtime-support, Storybook,
          legacy, material, and edge/SSR fixtures as implemented, partial,
          placeholder, or deferred.
    - [x] Map every current resolved Nx target to the Phase 3A/3B/3C roadmap and
          move edge/SSR-only acceptance out of the Phase 3 browser gate where
          necessary.
    - [x] Repair or retire the inferred
          `test-ci--src/lib/cem-elements.declaration-shape.spec.ts` target, which
          currently selects the Storybook-only Vitest project and reports no unit
          test files; keep `cem-elements:test:unit` as the accepted unit gate until
          the resolved target topology is corrected.
    - [x] Restore the `cem-elements:lint` project baseline: it currently reports
          15 pre-existing module-boundary errors in the CEMT/Storybook sources and
          two unrelated warnings, while the registration-contract source and test
          lint clean in isolation.
    - [x] Add an explicit todo checkitem before adding any new parity or browser
          fixture discovered by the audit.
    - Completed 2026-08-18: classified the substrate and every resolved target in
      [`cem-elements-phase3-substrate-audit.md`](cem-elements-phase3-substrate-audit.md),
      retired the misconfigured inferred Vitest atomics only for `cem-elements`,
      restored a warning-free project lint gate, and separated the current
      `verify:phase3a` browser aggregate from opt-in `verify-edge-ssr` evidence.
      The audit added the focused registration-scope fixture checkitem below before
      creating any new fixture.

- [x] Lock the logical declaration-scope host API required by runtime registration.
    - [x] Decide how a host creates an explicit scope and supplies its optional
          parent: opaque runtime-owned scope objects, one default root per
          `Document`, explicit same-document host/parser parents, and no inference
          from arbitrary DOM ancestry.
    - [x] Associate both inline and external runtime declarations with the selected
          explicit scope or their document's default root.
    - [x] Define scope identity, document ownership, parent compatibility, lifetime,
          and disposal without conflating the scope with `scopePolicyStamp`.
    - [x] Define how identical inherited registrations reuse the parent declaration
          and document-global constructor while same-scope duplicates and
          incompatible shadows fail before browser mutation.
    - [x] Promote the accepted API and lifecycle into
          `docs/cem-element-design.md` and focused pure contract tests.
    - [x] Lock registration-identity derivation for `CemProducedElementBehavior`:
          require a non-empty host `behaviorIdentity` when behavior is supplied,
          include it with tag/source/language in the content address, and reject
          callback-source hashing or implicit object-identity reuse.
    - [x] Add the audit-identified browser registration-scope fixture after the
          API is locked; prove same-scope failure, identical inherited reuse, and
          incompatible inherited/browser collisions without adding unrelated parity
          fixtures.
    - Completed 2026-08-18: the runtime now selects explicit/default logical scopes
      for inline and external declarations, derives `cem-registration-v1` identities
      including required host behavior versions, invokes the pure decision core, and
      marks CEM-owned constructors before document-global definition. Four scope and
      nine registration cases pass within all 100 unit tests; the focused browser
      fixture passes within all 96 Storybook tests and proves same-scope rejection,
      inherited reuse, incompatible inherited/CEM/foreign browser collisions, and
      missing behavior identity without registry mutation.

- [x] Lock the Phase 3A processing-host and worker/fallback transition API.
    - [x] Decide whether the single dedicated worker is owned per logical root scope,
          per `CemElementRuntime`, or per browser `Document`. Recommended: per logical
          root scope so explicit child scopes share retained compatible artifacts and
          independent roots remain isolated.
    - [x] Define versioned structured-clone request/response envelopes with monotonic
          job IDs, full render revisions, diagnostics, retained artifact/plan handles,
          and explicit cancel/dispose messages.
    - [x] Define the worker construction seam for bundlers, CSP hosts, and browser
          tests. Recommended: an injectable module-worker factory with a package
          default, not a public worker instance or ambient global override.
    - [x] Define deterministic startup-failure and post-handshake execution-failure
          transitions to the same main-thread host interface, including which jobs
          may be retried and how duplicate commits are prevented.
    - [x] Promote the accepted host lifecycle and transition table into the design
          before creating the worker/fallback browser fixture.
    - Completed 2026-08-18: adopted one package-private host per logical root,
      `cem-processing-host-v1` clone-safe envelopes with monotonic IDs and retained
      handles, the shared compile/render-diff/cancel/dispose interface, and an
      injectable module-worker factory with a package default. The pure transition
      core retries compile and pre-commit render work exactly once through fallback,
      aborts begun transactions under a fresh `renderAttempt`, preserves committed
      jobs without replay, and suppresses late worker results; 9 focused cases pass
      within all 109 `cem-elements:test:unit` tests.

- [x] Implement the smallest tests-first Phase 3A browser vertical slice.
    - [x] Register one inline `<cem-element>` declaration under the locked name
          rules and capture one produced instance's author payload into an inert
          WHATWG template data island.
    - [x] Package the exact generated CEM-QL ESM/declaration/WASM assets beside the
          package-private runtime support, rewrite built imports package-locally,
          declare the assets as cached Nx outputs, and verify the npm archive
          inventory before adding the module-worker entry.
          Completed 2026-08-18: `cem-elements:verify-package` packs 53 files,
          verifies the exact 31,092,853-byte WASM artifact and local ESM imports,
          includes the processing engine/host/worker entries, excludes sources/build
          metadata, and imports the tarball from a clean temporary consumer; its
          build and verification targets restore from Nx cache.
    - [x] Compile/render through the existing CEM-ML/CEM-QL WASM boundary using
          one dedicated worker, with the same semantic result through the required
          main-thread fallback.
    - [x] Apply revision-checked patch frames on the main thread while preserving
          light-DOM identity, focus/selection state, and data-island isolation.
    - [x] Prove the vertical slice in Rust-first contract tests, TypeScript unit
          tests, and one executable Storybook/browser fixture.
    - Completed 2026-08-18: canonical inline CEM-ML now compiles/renders/diffs in
      one module worker per logical root, with retained artifact/render-plan handles
      and deterministic startup/execution fallback to the same main-thread engine.
      The main thread validates complete revisions and buffered transactions before
      DOM mutation, preserves render identity/focus/selection and behavior-owned
      attributes, retries target mismatch as a fresh `replaceScope` attempt, and
      leaves URI/resource and legacy declarations on their established paths. The
      accepted evidence is Rust-first parity 3/3, TypeScript unit 111/111, Storybook
      Chromium 97/97, typecheck/lint, and the 53-file clean-consumer package probe.

- [x] Add URI declarations and the Phase 1 `<http-request>` resource slice.
    - [x] Support declaration `src` for document-relative, fragment-only, absolute,
          and module-map identities under the host resolver and scope policy.
    - [x] Add remote/local streaming, abort/stale-response protection, JSON/XML
          projections, and the fixture-backed `cem:for-each` flow.
    - [x] Preserve the same artifact identity, worker/fallback semantics, source
          maps, diagnostics, and patch protocol as inline declarations.
    - Completed 2026-08-18: canonical fragment, document-relative, absolute, and
      module-map declarations now compile as clone-safe chunked text through the
      retained root-scope worker/fallback host. URI artifact cache identity includes
      source-ref, resolver, and scope-policy state, and imported resource controls use
      the imported source URL as their base. CEM-QL-rendered `<http-request>` nodes
      lower to explicit clone-safe controls before DOM diffing; the main thread retains
      resolver/policy, multi-chunk loader, abort/stale-revision, and patch-commit
      ownership. Template-visible HTTP states now use the portable lifecycle vocabulary,
      and executable JSON/XML projections drive the same `cem:for-each` path. Accepted
      evidence is the Rust-native resource-envelope test, TypeScript unit 113/113,
      Storybook Chromium 97/97, all 15 executable demo pages, typecheck/lint, and
      the 53-file clean-consumer package probe.

- [ ] Complete Phase 3A/3B/3C substrate parity.
    - [x] Wire superseded processing-host render jobs to the locked `cancel`
          operation while preserving revision checks and atomic patch recovery.
          Completed 2026-08-18: each canonical instance now retains its active
          render job ID and sends `cancel(reason: "superseded")` before a newer
          revision enters the root-scope host. Worker and main-thread modes share a
          bounded active/cancelled lifecycle, reject late cancelled results, accept
          only live targets, and forget terminal IDs. The existing worker browser
          fixture holds and releases an obsolete result late, then corrupts a patch
          target and proves only the fresh revision commits through atomic recovery;
          accepted evidence is unit 114/114 and Storybook Chromium 97/97.
    - [x] Prove legacy compatibility only through opt-in
          `lang="custom-element-v0"` fixtures.
          Completed 2026-08-18: the browser selector now enters the legacy converter
          only for the exact `lang="custom-element-v0"` annotation; explicit CEM-ML
          retains precedence, while untyped XSLT-shaped markup and the native
          `custom-element-xslt` engine identity stay on the DOM path. All 12 legacy
          fixture pairs carry the annotation and the inventory gate rejects any
          unannotated legacy template. Positive and negative browser evidence passes
          within Storybook Chromium 97/97, with selector coverage in unit 123/123;
          all 15 demos and the 56-file clean-consumer package probe also pass.
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

- [ ] Before activating Phase 3.5, split its six Storybook cases and supporting
      processing-boundary selections from the shared Phase 3A files so
      `verify-edge-ssr` has phase-specific evidence instead of relying on the broad
      browser and unit targets.
