# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

Inventory and lock the Phase 6 CEM Site implementation boundary, then build the
root-wired documentation product from canonical repository sources. Live Figma
library and prototype updates are deferred to final Phases 10 and 11; Markdown
token specifications under `packages/cem-theme/src/lib/tokens/` remain canonical.

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

- [x] Complete Phase 3A/3B/3C substrate parity.
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

- [x] Author the Phase 3 primitive set exclusively on the accepted substrate.
    - [x] Extend the existing component test harness for substrate rendering,
          events, forms, accessibility, and visual snapshots.
        - [x] Add a Chromium action/field fixture that registers real CEM-ML
              declarations, proves render and re-render settlement, slice-driven
              events, native form data/reset/validity, accessibility assertions,
              a deterministic structural/visual baseline, and screenshot capture.
        - Completed 2026-08-19: `createSubstrateComponentHarness()` now owns
          runtime-aware declaration registration, rejects missing behavior
          identities and hard diagnostics, awaits declaration/render/re-render
          settlement, exposes data-island and native-form snapshots, and normalizes
          volatile runtime metadata out of reviewed visual baselines. The real
          action/field Chromium fixture proves light-DOM accessibility, custom
          event details, slice updates, native form data/reset/validity, focus,
          geometry/computed styles, and browser screenshot capture. The cached
          `verify-phase3-harness` target runs package lint/typecheck plus the focused
          fixture, and the full component test target sequences it before starting
          its own browser run.
    - [x] Wire action, field, surface, text, icon, stack, grid, list, nav, and
          dialog shell primitives through `<cem-element>` with no legacy runtime
          dependency.
        - [x] Give every behavior-backed primitive a stable versioned host identity
              and make primitive installation await declaration settlement before
              reporting registered/skipped tags or diagnostics.
        - [x] Extend the Chromium primitive fixture to prove the exact ten minimal
              Phase 3 tags register and render as accessible light DOM through the
              accepted substrate.
        - Completed 2026-08-19: all 16 behavior-backed declarations now supply
          stable versioned host identities, the public installer resolves only
          after every accepted declaration settles, and its ordered result reports
          only browser-registered tags plus complete diagnostics. The static gate
          verifies all 48 manifest entries, while the focused Chromium fixture's
          10 tests prove the exact minimal set renders accessible light DOM with
          inert data islands and no shadow or legacy runtime dependency.
    - [x] Run the substrate, component, CEM-ML fixture, and accessibility aggregate
          gates before closing Phase 3.
        - [x] Repair stable native-owner reconciliation for behavior-backed CEM-ML
              re-renders and migrate their browser fixtures to explicit runtime
              settlement. Truthful behavior registration currently exposes 38
              failing component assertions across native identity, focus, ARIA,
              interaction state, and nested workflow projection.
            - [x] Add a focused `cem-elements` patch-frame regression proving a
                  conditional child insertion preserves compatible native sibling
                  identity and focus while updating the surrounding render plan.
        - Completed 2026-08-19: render-engine patch transport `1.1.0` adds
          namespace-preserving child reconciliation so conditional insertions retain
          compatible native owners, focus, and state instead of replacing their
          parent. Retained render-plan directive values keep slice event bindings
          current after their visible attributes are consumed, and the component
          harness now awaits nested runtime settlement before assertions. The focused
          patch-frame unit and Storybook regressions pass; all 38 exposed component
          failures are repaired. Accepted evidence is 136/136 `cem-elements` unit
          tests, 120/120 Storybook Chromium tests, 125/125 component tests, the
          uncached 51-dependency `cem-elements:verify` aggregate, and the uncached
          41-dependency `@epa-wg/cem-components:verify` aggregate.

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final Phases 10 and 11, after Phase 9 release governance.

- [x] Move all remaining live Figma library and prototype updates behind the
      CEM Site, Studio, native-package, and release-governance phases.
      Completed 2026-08-19: `roadmap.md` now makes the pull-only Figma UI Kit and
      site demo final Phases 10 and 11, and this checklist activates Phase 6 while
      preserving completed credential-free Figma preparation at the end. No live
      Figma state changed; generated artifacts remain downstream projections of
      the canonical theme Markdown specifications.

- [x] Before activating Phase 3.5, split its six Storybook cases and supporting
      processing-boundary selections from the shared Phase 3A files so
      `verify-edge-ssr` has phase-specific evidence instead of relying on the broad
      browser and unit targets.
      Completed 2026-08-19: a dedicated Edge/SSR Storybook configuration now
      registers exactly the three hydration, edge-patch, privacy-export, and hybrid
      state cases (6/6), while a focused unit configuration owns the three
      structured-clone, default-deny/redaction, and host-value rejection cases plus
      the accepted hybrid storage and external host-envelope contracts, plus eleven
      Node-only initial SSR, edge-update, and browser-export boundary cases across
      four focused files (16/16). The default
      Phase 3 lanes exclude those deferred cases and pass at
      114/114 Storybook and 133/133 unit tests. The uncached five-dependency
      `cem-elements:verify-edge-ssr` aggregate passes without depending on either
      broad test target; lint and typecheck are also green.

## Phase 3.5 Checklist

- [x] Lock the external Edge/SSR host and render-state contract.
    - [x] Decide whether the existing
          `content-addressed-cache-with-revision-pointer-v1` model is the accepted
          roadmap "both" option, or whether cache-only or revisioned KV/document
          storage replaces it.
          Completed 2026-08-19: accepted the existing hybrid model without changing
          the `edge-render-state` 1.0.0 wire format. Immutable template artifacts,
          render plans, sanitized snapshots, and rendered HTML remain independently
          content-addressed and verified on read; one stable per-instance pointer
          record carries current addresses, revision/policy identity, and an ETag.
          Pointer writes use expected-ETag compare-and-swap, retain the current record
          on mismatch, and may leave unreachable immutable blobs for adapter-managed
          retention. The normative design and roadmap now reject cache-only and
          pointer-only storage for this version.
    - [x] Lock clone-safe host requests/results for initial SSR output, hydration
          metadata, previous render-plan identity, and streamed edge patch frames.
          Completed 2026-08-19: the public Edge/SSR profile now reuses the
          `cem-processing-host-v1` protocol version, monotonic job IDs, correlation,
          and diagnostic lifecycle without expanding browser-worker capabilities.
          `render-initial` returns owned-range HTML, versioned hydration metadata,
          and committed render state. `render-update` requires the previous state
          key, ETag, plan identity, and content address; it streams one progress
          envelope per patch frame and returns the next plan and pointer only at the
          terminal result. Compare-and-swap succeeds before `commit`, conflicts
          terminate without `commit`, failures are typed, and every nested value is
          checked for plain structured-clone transport. The host can accept serialized
          source, a compiled artifact transfer, or a content-addressed artifact, but
          cannot reconstruct policy-omitted snapshot fields.
- [x] Add a non-browser SSR host fixture that emits initial HTML plus hydration
      metadata from a serialized `DataIslandSnapshot` and validates template
      artifact identity, `RenderRevision`, source-map mode, and retained render-plan
      identity before hydration.
      Completed 2026-08-19: a Node-environment `render-initial` reference host now
      projects serialized template source and a complete sanitized snapshot without
      DOM globals, applies deterministic scope and dev/prod source-map policy,
      serializes escaped identity-bearing owned-range HTML, writes the hybrid state,
      and rereads the retained plan before returning `cem-ssr-hydration-v1` metadata.
      It rejects mismatched template/revision/source-map/scope identities before
      storage, fails closed when privacy policy omitted render-required fields,
      reports unresolved artifact forms without guessing, rejects unsafe raw HTML,
      and uses an atomic `ifAbsent` precondition so duplicate or concurrent initial
      renders cannot replace the existing pointer.
- [x] Add a DOM-free edge processing fixture that accepts a serialized snapshot plus
      previous render-plan identity and emits the same deterministic patch-frame
      stream as the browser reference runtime.
      Completed 2026-08-19: the Node-environment `render-update` reference host
      validates the state key, expected ETag, retained plan address, previous plan
      identity, and content-addressed template before projecting without DOM globals.
      Its async response stream emits the exact reference `begin`, `ops`, and
      `commit` patch frames only after compare-and-swap advancement and a verified
      state reread, followed by one terminal result. Focused evidence proves exact
      frame parity plus fail-closed missing-state, stale-ETag, address, identity, and
      unavailable-content outcomes; each failure emits one typed terminal response,
      no progress or commit, and leaves the pointer unchanged.
- [x] Prove privacy/export policy is applied before the serialized snapshot crosses
      the browser boundary, including omission and redaction cases in both host
      fixtures.
      Completed 2026-08-19: the public browser request factory accepts a raw
      `DataIslandSnapshot` plus export policy, applies default-deny or canonical
      redaction into an owned clone, removes the policy, and only then creates the
      structured-clone host envelope. Initial and update evidence proves omitted
      secrets never enter requests, redacted requests remain isolated from later
      browser mutations, and hosts retain or return exactly the exported snapshot
      without reconstructing denied fields. Omitted render-required input terminates
      as `privacy-policy-rejected`; initial storage remains absent and update storage
      and progress remain unchanged.
- [x] Run the Phase 3 browser reference gates and the opt-in `verify-edge-ssr`
      aggregate before closing Phase 3.5.
      Completed 2026-08-19: the uncached 51-dependent-task
      `cem-elements:verify` aggregate passed through Phase 3A, Phase 3B, and Phase
      3C with 114/114 browser cases, 133/133 unit cases, the clean 71-file npm
      package, CEM-ML fixture/e2e/benchmark coverage, parity inventories, demos,
      scheduling/cache evidence, and native template-artifact reload coverage.
      The separate uncached five-dependent-task `verify-edge-ssr` aggregate passed
      16/16 focused unit/host cases, 6/6 dedicated Storybook cases, and all four
      substrate fixtures. Neither lane depends on the other's broad test targets,
      so Phase 3.5 closes with the opt-in boundary intact.

## Phase 3.6 Checklist

- [x] Inventory the external `@epa-wg/custom-element` package before migration.
    - [x] Record `~/aWork/custom-element/` repository cleanliness, branches,
          remotes, tags, and history shape without mutating the external checkout.
    - [x] Record the published package identity, version, exports, packed files,
          build/test/release targets, and current fixture surface.
    - [x] Map runtime responsibilities to reusable `cem-element` substrate
          boundaries and identify package-specific public compatibility behavior.
    - [x] Add an explicit checklist item for every new migration or parity fixture
          discovered by the inventory before creating that fixture.
    - Completed 2026-08-19: recorded the clean but divergent source and distribution
      repositories, the npm `0.0.39` package/archive contract, the 88-story plus
      three-unit behavioral reference, and the runtime ownership map in
      [`custom-element-phase3.6-inventory.md`](custom-element-phase3.6-inventory.md).
      The audit also found that `packages/custom-element/` is an existing
      snapshot-based adapter with no imported external Git objects, so the next
      item must join two valuable histories rather than import into an empty path.
- [x] Lock the history-preserving import mechanics and monorepo package boundary
      from the inventory evidence before copying source.
    - [x] Select and document the Git history-import method, retained refs, and
          rollback/check procedure.
    - [x] Define imported source, generated-output exclusions, npm identity,
          package exports, and Nx ownership for `packages/custom-element/`.
    - Completed 2026-08-19: accepted and rehearsed the exact isolated
      `filter-branch` path rewrite plus tree-neutral three-parent `ours` join in
      [`custom-element-history-import-plan.md`](custom-element-history-import-plan.md).
      The rehearsal retained all 282 commits, produced the pinned rewritten
      main/develop/root hashes, namespaced all 32 real tags, kept the current
      monorepo tree byte-identical, and passed ancestry and Git-integrity checks.
      The boundary also locks the curated `dist/` publisher root, stable npm/tag
      surface, next-major helper policy, canonical source manifest, and resolved
      Nx build/test/release ownership. No external or product repository history
      was changed by the rehearsal.
- [x] Import `@epa-wg/custom-element` into `packages/custom-element/` with the
      accepted history and published npm identity intact.
    - Completed 2026-08-19: joined the path-prefixed external source graph through
      tree-neutral three-parent commit `dfe142be`, retaining all 282 commits,
      rewritten `main` and npm-`0.0.39` `develop` tips, 32 namespaced release tags,
      and two permanent source-tip tags without changing the active package tree.
      The Git database, parent topology, path prefix, ref targets, and first-parent
      tree identity pass the package-owned provenance gate.
- [x] Add the inventory-discovered migration and parity fixtures before claiming
      package adoption.
    - [x] Add a history-provenance gate for retained rewritten refs, namespaced
          tags, external-tip reachability, and connection to the existing adapter
          history.
        - Completed 2026-08-19: the uncached
          `@epa-wg/custom-element:verify-history` Nx target validates the immutable
          manifest's join/tree/parent identities, 282-commit graph, rewritten root
          and tips, exact 32+2 tag targets, package path prefix, current-HEAD
          reachability, and exclusion of the distribution repository graph.
    - [x] Add an external-reference manifest and verifier mapping all 88 browser
          stories and three real unit cases to accepted, package-adapter, or
          explicitly rejected bridge evidence.
        - Completed 2026-08-19: pinned the distribution `develop` tree and all 18
          contributing blob IDs in a CI-local manifest covering 87 exported
          Storybook cases, one import-map browser harness, and three real helper
          unit cases. The cached `@epa-wg/custom-element:verify-reference-corpus`
          Nx gate locks the 88+3 identities and category counts, requires every
          case to resolve to existing evidence or an explicit adapter requirement,
          and records 61 accepted, 29 package-adapter, and one rejected-bridge
          mapping without importing the distribution repository history.
    - [x] Extend the existing source/dist public-adapter browser fixture with
          multi-event/multi-slice updates, checkbox/radio coercion, form/custom
          validity, scoped-style containment, and DOM identity/focus on rerender.
        - Completed 2026-08-19: the shared source/dist browser fixture now drives
          the public settlement APIs through multi-event arithmetic, slice
          fan-out, explicit checkbox/radio values, live form-data mirrors,
          form/control custom validity, declaration and per-instance payload CSS
          containment, and retained identity/focus/selection across slice and
          host-attribute rerenders. The external-reference gate now distinguishes
          the 20 package-adapter cases proved by this matrix from nine still-open
          helper, upward-propagation, and exported-attribute policy cases.
    - [x] Add an actual packed-archive clean-consumer gate for intentional files,
          private/generated exclusions, root/subpath JS and type contracts, and
          browser loading from the packed artifact.
        - Completed 2026-08-19: replaced the broad package wildcard with a
          checked-in release-root allowlist and a 146-path SHA-256 inventory,
          stripped workspace scripts from the generated consumer manifest, and
          added cached `@epa-wg/custom-element:verify-packed-archive` ownership.
          The gate injects representative private/generated sentinels into a
          temporary copy of the actual `dist/` root, creates a real npm tarball,
          proves exact contents and dependency-free/private-vendor exports,
          installs it into a clean temporary consumer, compiles root and
          `./CustomElement` imports with TypeScript 6, and renders canonical
          CEM-ML from the installed archive in Chromium. Conditional `types`
          exports repair the root/subpath resolution defect found by that gate
          without changing either JavaScript target.
- [x] Rebuild the next-major `<custom-element>` implementation on the
      `cem-element` substrate without retaining a separate parser/render engine.
    - [x] Preserve and prove the accepted non-engine helper surface through the
          public source, built package, and clean archive consumer.
        - Completed 2026-08-19: restored and typed `cloneAs`, `deepEqual`,
          `mergeAttr`, `mix`, `obj2node`, `tagUid`, `xml2dom`, and `xmlString` on
          both public entry points. The shared browser smoke fixture proves their
          behavior against source and `dist/`, while the packed-archive gate
          compiles all eight imports in a clean TypeScript consumer. The locked
          external corpus now records 26 verified package-adapter cases and 3
          remaining policy cases without adding a parser or rendering path.
    - [x] Decide the next-major policy for upward attribute propagation and
          retained exported attributes before changing either behavior.
        - [x] Add source/dist public-adapter assertions for one-way declared,
              selected, and slice state plus both retired retention markers.
        - Completed 2026-08-19: explicitly retired implicit child-to-host
          propagation and the `dceExportedAttributes`/
          `dce-exported-attributes` retention allowlists. Host attributes remain
          author/script inputs unless an explicit substrate behavior owns a
          mutation, and `mergeAttr` now has one exact-set contract. The public
          source/dist browser gate proves the negative compatibility boundary;
          all 29 package-adapter cases are verified and none remain required.
- [x] Keep or retire `<template lang="custom-element-v0">` only after the explicit
      migration fixtures provide compatibility evidence.
    - [x] Add public source/dist evidence that an explicit deprecated selector is
          preserved and renders through the same substrate data-island path.
    - Completed 2026-08-19: retained the exact selector as deprecated through the
      next-major migration window. Twelve manifest-backed legacy/CEM-ML pairs,
      selector precedence/negative cases, and the public source/dist fixture prove
      conversion and rendering through the single CEM-ML/CEM-QL substrate; the
      browser XSLT engine and `custom-element-xslt` browser alias remain excluded.
      Removal now requires canonical replacements for retained demos, material
      components, and downstream generators plus the governed FF-5 exit gate.
      FF-5 stays blocking while narrowly allowlisting the two XSLT schema policy
      records whose text explicitly forbids browser `XSLTProcessor` delegation.
- [x] Run legacy, material, Edge/SSR, and custom-element package gates together
      before closing Phase 3.6 and retiring `@epa-wg/cem-elements` as the staging
      migration target.
    - [x] Add and execute one root `@epa-wg/cem:verify:phase3.6` Nx aggregate
          without merging the browser and isolated Edge/SSR lanes.
    - Completed 2026-08-19: the new root closure target passed with all 64
      dependencies, aggregating `cem-elements:verify`, the separately owned
      `cem-elements:verify-edge-ssr` lane, and
      `@epa-wg/custom-element:verify`. The browser lane passed 114/114 Storybook
      and 133/133 unit cases, legacy/material inventories, the clean 71-file
      substrate package, demos, CEMT pipeline, CEM-ML CLI/e2e/bench, and native
      template-artifact evidence. The isolated lane remained separate and passed
      6/6 Storybook plus 16/16 unit/host cases. The adopted custom-element package
      passed its 282-commit/32-version-tag/2-source-tip provenance gate, complete
      88-browser/3-unit reference corpus with all 29 adapter cases verified, and
      the locked 146-file packed-archive contract. The first aggregate run exposed
      a load-sensitive material-icon story wait: its 120-frame budget expired
      while 63 sibling tasks passed, but the same 114 cases passed in isolation.
      Replacing only element discovery's frame count with a bounded 10-second
      elapsed-time deadline made the focused suite and the full aggregate green;
      the final run restored 43 of 65 tasks from Nx cache. Phase 3.6 retires
      `@epa-wg/cem-elements` only as the staging migration target:
      `@epa-wg/custom-element` now adopts it, while `@epa-wg/cem-elements`
      remains the shared substrate and package rather than being deleted.

## Phase 6 Checklist

- [ ] Inventory the current root, package, generated-documentation, and example
      surfaces, then lock the CEM Site project and content-ownership boundary
      before scaffolding.
    - [ ] Map reusable authored Markdown, generated token/API reports, component
          examples, release notes, and existing browser entry points.
    - [ ] Record the evidence-based choice between `apps/cem-site` and a static
          docs application, including build, routing, deployment, and Nx target
          ownership.
    - [ ] Define authored-versus-generated content ownership so the site never
          becomes a second source for token, API, or component contracts.
- [ ] Create the root-wired CEM Site shell with stable routes and generated-doc
      ingestion from public package/report boundaries.
- [ ] Build the guides, token browser, component gallery, examples, API/reference,
      and release-notes surfaces without duplicating canonical source content.
- [ ] Add interactive token, component, CEM fixture, and native-output examples
      using the production CEM/custom-element implementation.
- [ ] Add search, stable deep links, root navigation, and optional Angular Material
      coverage comparison from the pinned parity evidence.
- [ ] Add cached Nx verification for site build, links, generated-content drift,
      accessibility, and clean deployment output before closing Phase 6.

## Later Non-Figma Phase Gates

Expand each gate into its task-level checklist when it becomes the immediate
goal. These gates deliberately keep the deferred Figma work from becoming active
before the non-Figma roadmap is complete.

- [ ] Complete Phase 6.5, the CEM Studio PWA and browser workbench, after the CEM
      Site contract is stable.
- [ ] Complete Phase 8 native platform package hardening and toolchain validation.
- [ ] Complete Phase 9 release, governance, and compatibility work for the code,
      docs, Studio, and native distribution surfaces.

## Phase 10 Checklist — Deferred Figma UI Kit

Phase 4 component names, variants, executable states, and accessibility semantics
are complete in the archived checklist, and the Phase 10 repository foundation
already owns the five-mode token gate and 48-primitive executable Figma inventory.
The remaining Phase 10 work is reviewed canvas work in the canonical CEM UI Kit
and must not start before Phase 9 is complete.

- [ ] Build and review the `02 Foundations` page from native CEM variables.
    - [x] Re-run the credential-free repository preflight and confirm the
          live-canvas ownership boundary before editing Figma.
        - Completed 2026-08-19: the resolved
          `@epa-wg/cem-components:verify-figma-inventory` Nx graph passed all
          eight tasks, with four restored from cache. It regenerated and
          validated 252 consistent variables across Light, Dark, Contrast Light,
          Contrast Dark, and Native, proved the representative token-propagation
          smoke, and verified all 48 public primitives (37 component sets, three
          components, four payloads, and four structural owners). The inventory
          remains truthful at zero reviewed and 48 planned canvas entries. The
          last recorded live-library review is still the 2026-04-30 230-variable
          revision, so the canonical file must be refreshed and its current
          revision captured before foundation construction. This environment has
          no approved live Figma reader/editor or credential; anonymous access is
          blocked. No canvas or publication claim was made.
    - [x] Repair the generated native Figma mode projection to use the current
          DTCG value shapes accepted by Figma Import mode before refreshing the
          live collection.
        - Completed 2026-08-19: Stage 4 now emits sRGB color objects, px
          dimension objects, and second-based duration objects while preserving
          aliases and finite number/string scalars. The offline validator rejects
          legacy scalar shapes, and the propagation smoke test compares the
          structured color values. `@epa-wg/cem-theme:test:figma` regenerated and
          validated 252 consistent tokens across all five modes, theme lint
          passed, and `@epa-wg/cem-components:verify-figma-inventory` passed its
          eight-task graph with the 48-entry inventory still at zero reviewed and
          48 planned. Shadow recipes and easing curves remained deliberately
          excluded until the following representation contract was adopted; no
          live Figma update was claimed.
    - [x] Adopt derived composite Effect Styles for canonical layering shadows
          and derived motion specimens for canonical easing curves; keep both
          families outside native Figma variable import.
        - [x] Add a checked-in `02 Foundations` composite-style and motion-review
              inventory plus deliberate-rejection fixture before live canvas
              construction.
        - [x] Add a credential-free Nx verifier that derives composite values
              from canonical tokens, rejects raw values in the inventory, and
              emits review evidence.
        - Completed 2026-08-19: canonical export now emits six real layering
          recipes as DTCG `shadow` composites and all eight easing tokens as
          `cubicBezier` arrays. Base remains the explicit string `none` because a
          valid DTCG shadow array must be non-empty; five semantic layer aliases
          preserve their owning rung's type. The checked-in 15-entry Foundations
          inventory defines six Effect Styles, one no-effect specimen, five alias
          annotations, and eight motion specimens without raw values. The new
          `@epa-wg/cem-theme:verify:figma-foundations` gate passed and emitted
          JSON/Markdown review reports, `build:token-platforms` retained all 445
          tokens across five modes, theme lint passed, and the nine-task
          `@epa-wg/cem-components:verify-figma-inventory` graph passed with eight
          tasks restored from cache. Native import remains at 252 consistent
          tokens per mode, the component inventory remains at zero reviewed and
          48 planned, and the Foundations inventory remains at zero reviewed and
          15 planned. No live canvas or publication claim was made.
    - [ ] Record the starting CEM UI Kit revision and confirm that the native
          `CEM Tokens` collection has the accepted Light, Dark, Contrast Light,
          Contrast Dark, and Native modes before editing the canvas.
        - [x] Add a checked-in native-library review evidence record and
              deliberate-rejection fixture for this manual checkpoint.
        - [x] Add a credential-free Nx verifier that keeps the refresh pending
              until a real starting revision and five-mode review are recorded.
        - Completed 2026-08-19: `native-library-review.json` now separates the
          historical 2026-04-30 live review, the current 252-variable generated
          contract, and a truthful pending refresh. Its fixture defines the
          `pending` -> `started` -> `reviewed` promotion procedure and rejection
          cases. The new cached
          `@epa-wg/cem-theme:verify:figma-native-review` target derives the 57
          COLOR, 112 FLOAT, and 83 STRING expectations from all five mode files,
          requires an explicitly recorded starting revision plus the exact
          collection/modes before accepting `started`, and emits JSON/Markdown
          reports. Theme
          lint and the nine-task `test:figma` graph passed; the ten-task
          `@epa-wg/cem-components:verify-figma-inventory` aggregate passed with
          nine tasks restored from cache. The refresh remains `pending`, the
          parent live checkpoint remains open, and no external Figma change was
          claimed.
    - [ ] Build color, typography, spacing, shape, stroke, layering, and motion
          guidance with variable bindings or approved composite text styles and
          no raw replacement values.
    - [ ] Review every foundation section in all five modes and record the Figma
          revision, evidence locations, and raw-value findings.
- [ ] Build and review the representative `03 Components` pilot for
      `cem-action`, `cem-text-field`, `cem-card`, `cem-nav`, and `cem-dialog`.
    - [ ] Keep variant dimensions independent, use component properties by
          semantic meaning, and test every owned state in all five modes.
    - [ ] Record the pilot fixture and review evidence before expanding to the
          remaining component inventory.
- [ ] Complete `03 Components` for every executable inventory entry, keeping
      inert payloads nested under their consuming visual owners.
- [ ] Populate `99 QA`, run the offline token/component gates, record the
      reviewed Figma revision and five-mode evidence, and publish the Phase 10
      library only after raw-value, detached-shape, state, and documentation
      checks pass.

## Phase 11 Checklist — Deferred Figma Site Demo

Phase 11 starts only after the Phase 10 UI Kit is reviewed and published.

- [ ] Build `04 Patterns` for auth, profile, assets, discussion, and settings
      entirely from library instances, then compose `05 Site Demo` from those
      patterns without detached one-off controls.
- [ ] Add matching CEM XML/HTML fixtures and a web implementation built from CEM
      components, with native iOS/Android token-usage notes.
- [ ] Record scenario tests, screenshots, and reviewed Figma evidence proving
      consistent tokens and component semantics across design and implementation.
