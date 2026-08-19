# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

Extend the existing `@epa-wg/cem-components` test harness for substrate-backed
DOM rendering, events, forms, accessibility assertions, and visual snapshots so
the Phase 3 primitive set can be authored exclusively with `<cem-element>`.

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
    - [x] Prove the full legacy and material parity inventories plus browser
          data-island isolation and accessibility gates.
        - [x] Promote all 12 file-backed legacy/CEM fixture pairs into
              one-to-one executable Storybook browser cases.
              Completed 2026-08-18: each manifest pair is imported directly
              from its checked-in legacy and CEM HTML files by a named browser
              story. The 12 cases exercise registration, declaration-shape
              rejection, local/external sources, payload, attributes and
              invalidation, slots, slice events, datadom migration,
              conditionals, and the exact legacy bridge; Storybook Chromium
              passes all 109 tests.
        - [x] Promote all 8 material/CEM fixture pairs into one-to-one
              executable Storybook browser cases.
              Completed 2026-08-18: each checked-in material source pair now
              runs in isolated same-origin documents so both sides retain the
              real `cem-*` browser names. The legacy side uses the documented
              thin adapter and exact v0 opt-in without modifying fixture bytes;
              manifest-ordered dependencies, local/external declarations,
              module URLs, composition, slots, data payloads, and migrated
              slice interactions pass within Storybook Chromium 117/117.
        - [x] Prove raw declaration templates and captured data islands remain
              inert to layout, selectors, forms, accessibility, and visible text.
              Completed 2026-08-18: the browser isolation matrix places both
              raw sources inside a live form and proves query/tag collections,
              layout boxes, active styles, visible text, form ownership/data,
              focus, and document/accessibility exposure contain only the
              rendered projection. Worker, startup-fallback, and
              execution-fallback rerenders preserve the same island boundary;
              Storybook Chromium passes all 118 tests.
        - [x] Enforce the Phase 3 accessibility contract across the complete
              legacy and material browser inventories.
              Completed 2026-08-18: one cross-document browser audit now runs
              over all 12 legacy/CEM pairs and all 8 isolated material/CEM
              pairs (40 rendered sides), including every legacy post-mutation
              and post-event checkpoint. It enforces accessible names, native
              roles and focusability, single-tab-stop ownership, unique IDs,
              resolved label/ARIA references, valid reflected ARIA state, and
              image alternatives. Material action/disclosure cases additionally
              exercise native activation and reflected state; named authored
              inputs keep the unchanged legacy input/autocomplete fixture bytes
              conformant. Storybook Chromium passes all 118 tests.
        - [x] Include every parity fixture in the Phase 2 CLI validation,
              end-to-end, and benchmark aggregate gates.
              Completed 2026-08-18: the CLI now extracts both parity manifests,
              resolves external fragments, and lowers legacy templates through the
              Rust converter into 45 schema-profile inputs alongside the 10 Phase 2
              base fixtures. Validation reports 55 inputs with no errors, fatals, or
              hard violations; the CLI end-to-end gate passes, and the benchmark
              asserts all 40 source sides while retaining the AC-N-1 budget.
            - [x] Add package-owned pass/fail fixtures for the dedicated
                  `cem-element-template/v1` schema profile before routing the
                  parity manifests through it.
                  Completed 2026-08-18: the registered schema package owns its
                  manifest, schema, valid and invalid examples, generated README,
                  Nx verification target, Rust manifest-index test, and CLI
                  pass/fail validation coverage.
    - [x] Add the Phase 3B scope-policy worker pool, content-addressed cache, and
          deterministic scheduling traces behind the stable host API.
          Completed 2026-08-19: compatible logical roots share lazily allocated
          worker slots bounded by hardware concurrency and an eight-worker host cap,
          with a 64-operation per-slot queue, FIFO-per-root ordering, round-robin
          cross-root dispatch, pool-global job IDs, and preemptive cancel controls.
          Template compilations and render plans use 64-entry content-addressed LRU
          retention in both worker and main-thread modes; artifact aliases refresh
          before rendering and evicted plans safely degrade to `replaceScope`.
          Clone-safe sequence-only traces expose enqueue, dispatch, cancellation,
          overflow, and fallback decisions without affecting execution. Accepted
          evidence is typecheck/lint, 132/132 unit tests, 118/118 Storybook Chromium
          tests, the 62-file clean package probe, and the 48-task
          `verify:phase3b` aggregate.
    - [x] Add Phase 3C precompiled component-template artifacts without removing
          the source-driven runtime path.
          Completed 2026-08-19: deterministic `cem-template-artifact/1`
          MessagePack envelopes retain compiled CEM-ML/CEM-QL IR across native
          and WASM reloads and bind their source hash, host bindings, source-map
          mode, compiler version, and IR version. The browser transfer binds the
          active policy stamp, and all mismatches are rejected before rendering.
          The browser host supports opt-in registry reads and write-through while
          preserving source compilation as the warning-backed fallback. Two
          isolated engines prove registry
          miss/compile/store followed by byte-only import/render. Accepted
          evidence is the 4/4 native artifact fixture, all 204 `cem_ql` tests,
          135/135 unit tests, 119/119 Storybook Chromium tests, the 62-file clean
          package probe, and the uncached 50-dependency `verify:phase3c`
          aggregate.
        - [x] Add a native component-template artifact fixture covering binary
              compile/reload render parity plus hash, source, binding, mode, and
              policy rejection.

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
