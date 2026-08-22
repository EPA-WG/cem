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

- [x] Establish graph-native Markdown-to-HTML conversion before choosing or
      scaffolding the site application.
    - [x] Declare the Markdown-to-HTML edge in the Markdown schema package so
          conversion planning selects a named, typed converter rather than an
          export-time format guess.
    - [x] Add an explicit `convert` transform-graph node that preserves glob
          variants and produces chainable typed HTML artifacts.
    - [x] Reuse the Markdown lifecycle AST and CEM-ML output pipeline without a
          JSON intermediary, with diagnostics and provenance covered by focused
          Rust and CLI tests.
    - Completed 2026-08-19: the Markdown schema package now owns the ready
      `markdown-to-html-rust` edge, explicit graph `convert` nodes resolve that
      edge by typed source/target identity, and glob variants remain distinct
      through HTML export. The native Markdown AST feeds the renderer without a
      JSON bridge, while HTML artifacts and `.html.map` sidecars retain source
      frames and output spans. Focused parser, registry, schema-package, direct
      conversion, and graph CLI tests pass; the aggregate `cem_ml:test` suite
      passes, and all 517 `cem_ml_cli:test` library cases pass (515 in the
      restricted sandbox plus the two loopback HTTP cases with socket access).
- [x] Inventory the current root, package, generated-documentation, and example
      surfaces, then lock the CEM Site project and content-ownership boundary
      before scaffolding.
    - [x] Map reusable authored Markdown, generated token/API reports, component
          examples, release notes, and existing browser entry points.
    - [x] Record the evidence-based choice between `apps/cem-site` and a static
          docs application, including build, routing, deployment, and Nx target
          ownership.
    - [x] Define authored-versus-generated content ownership so the site never
          becomes a second source for token, API, or component contracts.
    - Completed 2026-08-19: [`cem-site-phase6-inventory.md`](cem-site-phase6-inventory.md)
      records the 41-project resolved Nx graph, the absence of a site/serve
      owner, the existing Storybook and two generated-doc targets, authored
      and generated content families, 142 inspected example assets, browser
      entry points, missing API/site gates, and an eight-rule ownership
      contract. Theme Markdown specifications remain canonical; generated
      and Figma data remain read-only projections. The accepted boundary is a
      dedicated static-output `apps/cem-site` Nx application whose cached build
      target invokes the CEM-ML CLI transformation graph. CEM-ML, not Vite or an
      asset-copy script, owns site content and web dependency assembly; Vite may
      remain an optional generated-output server/test harness. No application was
      scaffolded.
- [x] Make npm/browser dependency assembly a native CEM-ML web-build graph
      before scaffolding the site shell.
    - [x] Define a dedicated CEM-ML module-map content type and schema. Its
          authored `imports` entries are the complete JavaScript dependency
          manifest: source maps resolve explicit npm files (including paths in
          `node_modules`), destination maps declare browser-facing module URLs,
          and CEM-ML does not discover dependencies by parsing JavaScript.
    - [x] Lower dedicated source/destination module maps into ordinary typed
          JavaScript graph imports and exports, copy only declared source assets
          beside the exported app HTML, and project only destination URLs into
          the browser import map.
    - [x] Reject missing destination entries, undeclared/non-JavaScript assets,
          non-bare module specifiers, escaping output paths, and destination
          collisions with stable native diagnostics.
    - [x] Prove with Rust-native and CLI graph fixtures that declared npm assets
          are byte-preserved in the clean output, no `node_modules` path leaks
          into browser output, and undeclared JavaScript is not copied.
        - Completed 2026-08-19: schema package
          `cem_ml_schema_package_module_map_v1` owns
          `application/vnd.cem.module-map+json` and
          `https://cem.dev/ns/data/module-map/1`. Paired maps now lower each
          exact declared `.js`/`.mjs` source into an opaque `text/javascript`
          graph import and a byte-preserving export relative to the HTML
          destination, while the HTML import map receives only app-relative
          destination URLs. Native negative coverage rejects mismatched keys,
          relative/URL/prefix specifiers, non-JavaScript or escaping targets,
          and destination collisions. The CLI fixture copies the declared npm
          asset, omits an undeclared sibling, and leaves no `node_modules` path
          in the emitted HTML.
    - [x] Emit a deterministic module-asset read/dependency manifest with content
          digests so Nx cache inputs and provenance auditing cover every declared
          JavaScript resource.
        - [x] Add native fixtures for stable specifier/source-map/source/target/
              destination records, byte lengths, per-asset SHA-256 digests, and a
              host-neutral aggregate cache key.
        - [x] Carry the manifest through the common transform-graph response and
              CLI JSON/CEM/Markdown reports so native, WASM, and CLI hosts expose
              the same provenance evidence.
        - [x] Add a read-only CLI cache-key projection and prove an Nx runtime
              input consumes it without parsing module maps in JavaScript or
              performing transform output writes during hashing.
        - Completed 2026-08-20: graph lowering now hashes each declared module
          asset once and emits ordered provenance records through the common
          response and every CLI report projection. The aggregate SHA-256 uses
          host-neutral specifier, target, content-type, length, and content-digest
          fields while reports retain resolved source-map, source, and destination
          URIs. `--module-asset-cache-key` stops after lowering and performs no
          output writes. The module-map Nx project consumes that projection as a
          runtime input over a real `lit` dependency; dependency-output hashing
          ensures the native CLI exists before Nx evaluates the runtime key.
- [x] Create the root-wired CEM Site shell with stable routes and generated-doc
      ingestion from public package/report boundaries.
    - [x] Add a Rust-native graph fixture for Markdown → HTML → recovered DOM →
          CEMT layout composition, including escaped-text regression coverage,
          native artifact ownership, diagnostics, and source-map provenance.
    - [x] Execute the registered `Html5RecoveryConverter` in transform graphs
          and expose its typed DOM projection directly to CEMT without raw HTML,
          JSON, or an untyped string bridge.
    - [x] Scaffold `apps/cem-site` as a cached CEM-ML CLI application with an
          explicit Hugo-like publication graph, stable root/reference routes,
          and generated documentation consumed from an upstream Nx output.
    - [x] Verify the clean static output, route links, generated-doc ownership,
          native transform report/source maps, and Nx cache replay.
    - Completed 2026-08-20: transform graphs now execute the registered HTML5
      recovery converter and retain `HtmlDocumentAst` as a native DOM projection.
      CEMT receives a borrowed hierarchical query view over that owner, including
      native event/attribute ranges, without a raw HTML, JSON, or replacement-tree
      handoff. Rust unit and adapter suites pass, the CLI's 519 pre-existing cases
      passed in the aggregate run, and the corrected end-to-end native-layout
      fixture passes as the 520th case. `apps/cem-site` now publishes explicit
      root, guides, and generated CEM-ML reference routes through the native CLI;
      its Nx verification proves the exact clean output allowlist, internal links,
      upstream generated-doc ownership, graph report, CEMT source-map spans, and a
      four-of-four local cache replay. The shell intentionally has no JavaScript;
      later interactive work must use the dedicated module-map graph contract.
- [x] Build the guides, token browser, component gallery, examples, API/reference,
      and release-notes surfaces without duplicating canonical source content.
    - [x] Extend the checked-in publication graph with an explicit route/source
          allowlist for the first authored guides and package-owned references;
          exclude archives, active planning documents, debug projections, and
          directory-wide copies.
        - Completed 2026-08-20: `apps/cem-site/site.routes.json` now records the
          exact source, route/output, source kind, owning Nx project, upstream
          generation target, and graph import/export identities for five pages.
          The first external authored surfaces are the
          `@epa-wg/cem-ml-cli` browser/Node usage guide and the
          `@epa-wg/cem-ml` WASM runtime reference; the existing transform-config
          reference remains generated only by `cem_ml:build:docs`. Nx hashes the
          package-owned Markdown directly, while the verifier rejects duplicate
          routes, missing owners/targets, graph/report drift, directory-wide or
          excluded archive/planning/temporary/Figma/debug sources, unexpected
          outputs, unpublished internal links, and missing CEMT source maps.
    - [x] Add a static token-browser surface sourced from canonical theme Markdown
          and public `@epa-wg/cem-theme:build:tokens` outputs, keeping generated
          values read-only and excluding Figma and debug token projections.
        - Completed 2026-08-20: the theme target now emits the public beta
          `cem.tokens.catalog.json` package export from the same sorted metadata
          records as `cem.tokens.ts`. The native CEM-ML site graph imports that
          JSON directly and renders 487 visual/voice records at `/tokens/`
          through CEMT with no JavaScript. The route allowlist names all ten
          canonical Markdown specifications exactly; verification maps every
          catalog record to its source-table heading, enforces the generated
          owner/target relationship, rejects Figma and intermediate/resolved
          debug inputs, and checks clean output, links, and render provenance.
          The uncached theme/token/site build and the full site verifier pass;
          a subsequent verification replay restored all 10 tasks from local Nx
          cache, and the final verifier-only edit restored 9 of 10. Focused
          `cem-site:lint` and `@epa-wg/cem-theme:lint` targets pass.
    - [x] Add a component-gallery catalog from component semantics, inventory
          reports, and separately owned Storybook/example links without copying
          executable component fixtures into site-owned content.
        - Completed 2026-08-20: the cached public
          `@epa-wg/cem-components:build:catalog` target now validates the
          canonical MVP semantics, 48 executable primitive declarations,
          package reference/conventions/accessibility guidance, and the
          generated 40-of-40 covered state-matrix report, then emits the public
          beta `cem.components.catalog.json` package export deterministically.
          The native site graph renders all 48 records at `/components/` with
          category states, token families, 48 package reference links, eight
          exact package-owned example source links, and the separately owned
          local `cem-elements:storybook`/`build-storybook` commands and source
          link. It copies no declaration or example markup, ships no JavaScript,
          consumes no Figma projection, and records canonical versus generated
          evidence sources separately. Site verification and the 66-file npm
          package inventory pass; two uncached catalog generations have the same
          SHA-256 digest, and the final Nx site replay restores all 13 tasks from
          local cache.
    - [x] Add example, API/reference, and release-note routes from their current
          owners, using an explicit authored-reference policy where no generated
          API projection exists and never presenting an absent generator as one.
        - Completed 2026-08-20: the native publication graph now adds eight
          owner-sourced pages: canonical CEM-ML and component example indexes,
          the package-authored component reference, the workspace release notes,
          and component, element, theme, and custom-element release notes. Every
          allowlisted page declares a content role and relative-link policy;
          `authored-reference` requires authored input with no upstream target,
          while `generated-reference` requires a generated input and scheduled
          Nx target. The shared CEMT layout rewrites 40 repository-relative links
          to the established canonical `develop` source tree without changing
          site-root navigation or copying executable example markup. All 15 site
          routes build and pass the Nx site verifier.
    - [x] Prove every new route records its canonical owner/upstream Nx target and
          that publication remains deterministic from a clean output directory.
        - Completed 2026-08-20: site verification now reads the resolved Nx
          project graph, assigns every source to its unique deepest project root,
          rejects owner drift, and requires each generated route's upstream target
          to exist on that same owner and be scheduled by `cem-site:build`. The
          cached `cem-site:verify:determinism` target removes the output directory,
          runs the real native publication twice, rejects any unexpected output,
          and compares per-file SHA-256 records. Both clean builds emitted the
          same 31 files for 15 routes with aggregate digest
          `c0ec818f318d7743cb4045501c1336dbc60b296e4454332404cb030c6b4bbd95`.
          Site lint passes, and the final aggregate verification restored all 14
          tasks—including ownership and determinism—from local Nx cache.
- [x] Extend the dedicated module-map deployment contract to schema v2 so the
      production CEM/custom-element runtime can publish every explicit web
      dependency without an asset-copy exception.
    - [x] Add a versioned schema package whose paired `resources` entries declare
          exact logical resource names, source files, app-relative destinations,
          and content types while preserving v1 `imports` behavior.
    - [x] Add Rust-native fixtures proving CSS, relative JavaScript sidecars,
          workers, and WASM lower as opaque graph artifacts without JavaScript
          parsing, transitive discovery, or undeclared-resource copying.
    - [x] Add a byte-capable transform-graph artifact boundary and atomic CLI
          publication path that preserve binary resource bytes and identity.
    - [x] Extend deterministic resolved-read manifests and cache keys across
          JavaScript and resource entries, with stable validation for mismatched
          keys, unsupported types, escaping destinations, and collisions.
    - [x] Verify the v1 compatibility lane, v2 schema package, native and CLI
          suites, WASM/type projections, and Nx cache replay before activating
          the interactive site fixture.
        - Completed 2026-08-20: module-map v2 adds an exact, paired `resources`
          contract for JavaScript, module workers, CSS, and WASM while keeping
          v1 `imports` behavior unchanged. Resource reads lower as declared
          opaque graph artifacts, retain their content identity, and publish
          exact text or binary bytes through the CLI's atomic multi-output
          boundary; invalid UTF-8 WASM bytes are covered explicitly. Resolved-read
          manifests and cache keys include every declared import and resource,
          and validation rejects schema/key mismatches, unsupported media types,
          unsafe destinations, and import/resource collisions. The CEM-ML build,
          its 1,968-test aggregate, and the added lifecycle regression pass; the
          full CLI target passes 521 unit tests and all integration suites,
          including both schema-owned module-map versions. A repeat of the
          aggregate restored all 37 Nx tasks from local cache, and lint plus
          scoped formatting pass.
- [x] Add interactive token, component, CEM fixture, and native-output examples
      using the production CEM/custom-element implementation.
    - [x] Add a site-owned interactive example fixture and native CEMT route that
          reference canonical token and component identities without copying their
          package-owned declarations.
    - [x] Declare the site bootstrap, production custom-element runtime,
          component sidecars, theme/component CSS, processing worker, and CEM-QL
          WASM exclusively through a paired module-map v2 source/destination map.
    - [x] Exercise token filtering, production component light-DOM rendering, an
          inline CEM declaration, and a live native-output projection in the
          browser without a Vite or asset-copy path.
    - [x] Verify the exact deployed resource allowlist, module-map rewrite,
          undeclared-resource exclusion, browser interaction contract, clean
          deterministic output, and Nx cache replay.
        - Completed 2026-08-20: `/examples/interactive/` now fuses a site-owned
          JSON fixture with native CEMT and references canonical theme-token and
          component identities. A paired module-map v2 graph owns three browser
          imports and all 34 explicit JavaScript, CSS, worker, and WASM resources;
          every published byte is compared with its declared source, and static
          verification rejects undeclared imports, module URLs, source-only paths,
          and destination drift. Chromium proves token filtering, two native
          `cem-action` clicks, `cem-field` light DOM, inline `custom-element` CEM
          compilation, native output, the exact import map, and both stylesheets
          with no runtime errors. Import-map rewriting now retains the upstream
          CEMT source-map stack and rebases unchanged output spans; the added Rust
          regression passes within all 1,970 library tests, and the regenerated
          module-map README passes all 13 schema-package structure tests. The two
          clean site publications contain the same 70 files for 16 routes with
          aggregate digest
          `61e57b1e30a2d0c15fd81cc33d0d93490da1f32919af62709ab745243ea33bcf`.
          Aggregate verification serializes determinism before Chromium to avoid
          output replacement races, lint passes, and the repeat restored all 19
          Nx tasks from local cache.
- [x] Add search, stable deep links, and root navigation.
    - [x] Generate a deterministic search index from the route manifest through
          the native CEM-ML transformation graph.
    - [x] Give rendered headings stable fragment identifiers and verify every
          declared fragment against the published HTML.
    - [x] Publish a dedicated search route/runtime through a paired module map,
          link it from the shared navigation, and cover query/deep-link behavior.
    - [x] Compose the search controls and interaction from production
          `@epa-wg/cem-components` on the `cem-elements`-backed custom-element
          runtime; keep site code limited to search orchestration.
    - [x] Verify clean deterministic output and Nx cache replay for the completed
          search/deep-link/navigation slice.
        - Completed 2026-08-20: the route manifest now owns 16 searchable
          documents and selected heading metadata, and native CEMT renders the
          semantic `/search/` index without a JSON sidecar or post-build copy.
          The search field and action are production `cem-field` and `cem-action`
          controls installed through the shared `cem-elements`-backed
          custom-element runtime; site JavaScript only filters/ranks the rendered
          index and maintains the `q` URL state. The paired module map now owns 39
          exact JavaScript, CSS, worker, and WASM declarations for both interactive
          routes, and all primary navigation exposes Search and Interactive
          examples. Native CEMT gives all 145 headings across 17 routes unique
          deterministic IDs, while static verification rejects missing/duplicate
          fragments and proves every indexed heading's level and text. Chromium
          proves initial and live search, exact-heading ranking, the stable
          `Graph Semantics` deep link, CEM light-DOM controls, stylesheet/import-map
          scope, and zero runtime errors. Two clean builds contain the same 113
          files with aggregate digest
          `c38e634aa647055b2381a367de79c46db78c83c5333e0b18cc16543a922d4fa9`;
          the repeat aggregate restored all 20 Nx tasks from local cache, and
          lint plus scoped Prettier checks pass. The CEM Site/CEM Studio shared
          component-composition principle is now durable in `CLAUDE.md` and the
          roadmap.
- [x] Add a static Angular Material coverage comparison generated from the
      pinned parity evidence, without an Angular runtime dependency.
    - [x] Make the exact pinned parity inventory a validated, cached site-build
          input and retain every catalog row.
    - [x] Publish a semantic comparison route with benchmark provenance,
          covered/partial totals, CEM owners, states, accessibility, keyboard,
          evidence, and bounded notes.
    - [x] Add the route to primary navigation and the generated search index.
    - [x] Verify source/render parity, stable fragments, zero Angular runtime
          assets, deterministic output, and production-site checks through Nx.
        - Completed 2026-08-20: native CEMT now publishes
          `/components/angular-material/` directly from the package-owned parity
          inventory pinned to Angular Material `v22.1.1` at commit
          `0b67c3c38141049657b1167479accc80e455d2bd`. The route retains all 37
          catalog rows—17 covered and 20 partial—and renders every CEM owner,
          state, keyboard contract, accessibility contract, evidence locator,
          and scope note with stable row fragments. The site build hashes the
          inventory, parity documentation, and primitive inventory and schedules
          cached `@epa-wg/cem-components:verify-material-parity` validation.
          All primary navigation and the static search projection expose the
          route; native `seq:count` now renders the exact 17-document search and
          37-row coverage totals. Static verification rejects source/render
          drift, missing package owners, scripts, `node_modules`, and Angular
          runtime imports. The full 21-task `cem-site:verify` graph passed links,
          186 stable headings across 18 routes, Chromium search and interactive
          checks, and two clean 115-file builds with aggregate SHA-256
          `04d025e2544701282f3183165493f817f2a3630aca7980c27d5f1293a0753314`;
          the repeat graph restored all 21 tasks from local cache. Site lint and
          scoped Prettier checks pass.
- [x] Close Phase 6 with complete public-surface documentation and cached
      production verification.
    - [x] Derive the public-surface inventory from every non-private workspace
          `package.json` and every Cargo crate with `publish = true`, retaining
          each deployment or crate identity even when product families overlap.
    - [x] Add canonical package documentation and root-wired site routes for
          every uncovered public npm package and Cargo crate without copying
          package content into the site application.
    - [x] Add a cached full-route Chromium gate covering responses, runtime and
          console failures, landmarks, titles, heading order, names, fragment
          targets, focus visibility, and clean production output.
    - [x] Make the public-surface inventory, static source/render verification,
          deterministic build, browser accessibility crawl, search, and
          interactive checks one cached `cem-site:verify` closure graph.
        - [x] Make `cem_ql:build:wasm` hash its Rust toolchain, lockfile,
              transitive `cem-ml` sources, embedded schemas, and build wrapper;
              make the `cem-elements` and `custom-element` packaging targets
              hash dependency outputs; prove uncached and cached site
              publication copy identical WASM bytes.
    - [x] Prove uncached execution plus all-cache replay, update root/site
          documentation, and mark the Phase 6 exit criterion complete.
    - Completed 2026-08-20: publication metadata now drives an independently
      verified inventory of all 11 public surfaces—seven non-private npm
      packages and four Cargo crates with explicit `publish = true`. Native CEMT
      publishes the static `/packages/` index plus owner-matched canonical routes
      for `cem-elements`, `custom-element`, Trang Native, and all four Rust
      crates; the new crate documentation remains package-owned. The strict
      Markdown edge remains raw-HTML-free. The cached production gate served and
      crawled all 26 routes in Chromium, checked 253 stable headings, 868 named
      keyboard stops with visible focus, 78 internal fragment links, landmarks,
      responses, ARIA references, images, and runtime/console failures, and
      reported zero errors. Search covers all 25 non-search pages, and the
      interactive component/runtime checks remain green. The closure exposed and
      repaired a stale transitive WASM cache boundary: `cem_ql:build:wasm` now
      hashes its toolchain, lockfile, `cem-ml` sources, schemas, and wrapper,
      while both packaging consumers hash dependency outputs. Fresh and cached
      publication now retain the same 34,568,929-byte WASM with SHA-256
      `0fcc18011de323e1b145e0824e4e3c1bf9221c1c2671df2189f5bc216ed934ac`.
      Two clean builds contain the same 131 files with aggregate SHA-256
      `96f6dd801da1891c29ef569140d8f1b099da90dedb8c99c0127080265ba43b7b`;
      the final replay restored all 22 Nx tasks from local cache. Site lint,
      resolved Nx configuration, JSON parsing, diff checks, and scoped Prettier
      checks pass. The Phase 6 roadmap exit criterion is complete.

## Phase 6.5 Checklist — CEM Studio PWA And Browser Workbench

This checklist executes the accepted Phase 6.5 roadmap and
[`cem-studio.md`](./cem-studio.md) contract. CEM-ML remains the production
transformation-graph authority, while all visible Site and Studio functionality,
including search, is composed from `@epa-wg/cem-components` and
`@epa-wg/cem-elements` rather than app-local substitute controls.

- [x] Audit the resolved Studio-adjacent Nx projects, browser command surface,
      component inventory, persistence/PWA foundation, and production web build
      boundary before scaffolding.
    - Completed 2026-08-20: the
      [`Phase 6.5 boundary audit`](./cem-studio-phase6.5-boundary-audit.md)
      resolved 44 Nx projects and no existing Studio app; confirmed the public
      browser command service and checked native/Node/browser parity for nine
      portable operation kinds plus cancellation; mapped the 48-component,
      37-row Material-parity foundation; and found no existing IndexedDB,
      File System Access, service-worker, or install-manifest owner. It accepts
      `packages/cem-studio`, one exact CLI/runtime chain, Nx orchestration, and
      CEM-ML graph/module-map production assembly. Its schema/content decision
      point is closed by the following completed contract slice.
- [x] Accept the Studio portable project v1 schema/content identity and prove
      its CEM and normalized JSON projections before application persistence.
    - [x] Add valid, edge, and deliberate-rejection project fixtures first,
          covering stable ids, hierarchy, resource identities, relative paths,
          run-config references, revisions/hashes, and forbidden provider/UI
          state.
    - [x] Decide and register the canonical CEM content type, JSON projection,
          versioned namespace/schema identities, schema-package ownership, and
          `project.cem` directory/bundle rules.
    - [x] Prove CEM/JSON semantic round trips, deterministic normalization,
          forward-version rejection, logical `studio://` URI derivation, and
          validation before import writes.
    - Completed 2026-08-20: `studio-project/v1` now owns the canonical
      `application/vnd.cem.studio-project+cem` projection, normalized
      `application/vnd.cem.studio-project+json` projection,
      `https://cem.dev/ns/studio/project/1` schema identity, and
      `https://cem.dev/schema/studio/project.schema.json` artifact. Six
      manifest-indexed fixtures and ten native contract tests cover projection
      equality, deterministic round trips, exact namespace selection, normalized
      JSON shape, stable hierarchy/resource references, contained paths,
      `studio://` derivation, forbidden host/UI state, duplicate ids, and
      forward-version rejection. The package is embedded in the built-in
      registry, participates in all three CLI schema-package gates, and passes
      its cached Nx verify, the 13-test schema-package structure target, and the
      CEM-ML WASM build. The broader 1,970-test CEM-ML gate passed 1,969 tests;
      its unrelated debugger deadline timing test passed immediately when
      rerun alone through the same Nx target.
- [x] Create the publishable `@epa-wg/cem-studio` Nx application/package under
      `packages/cem-studio` with exact `@epa-wg/cem-ml-cli` versioning and one
      transitive `@epa-wg/cem-ml` runtime.
    - [x] Make the CEM-ML transformation graph and source/destination module
          maps own final app, component, worker, WASM, manifest, service-worker,
          style, and cache-inventory emission with no post-graph copy exception.
    - [x] Add cached build, lint, typecheck, package, resolved-dependency,
          deterministic-output, and clean-consumer verification targets.
    - Completed 2026-08-21: `@epa-wg/cem-studio@0.1.0` is a fixed-version
      CEM-ML platform member with an exact `@epa-wg/cem-ml-cli` dependency and
      one logical transitive path to `@epa-wg/cem-ml`. Its single authoritative
      module-map rewrite emits 53 declared app/component/worker/style/WASM
      assets, including the service worker, plus seven direct graph exports:
      HTML, manifest, icon, build metadata, cache inventory, and two dependency
      metadata resources. Generic
      `import @opaque=true` config support now publishes explicitly typed raw
      resources without parsing or schema claims and rejects non-direct or
      identity-changing graph use. The cached Studio `check` covers lint,
      typecheck, dependency resolution, exact-byte graph output, two-clean-build
      determinism, npm packing, and a temporary clean consumer; the verified
      archive contains all 60 static files and exactly one installed runtime.
      The service worker and cache inventory remain intentionally
      bootstrap-only until the later PWA lifecycle item.
- [x] Classify the initial Studio shell and workbench behavior against the
      pinned Angular Material inventory before composing it.
    - [x] Reuse completed general CEM controls within their proven contracts;
          finish missing general parity in `@epa-wg/cem-components` first.
    - [x] Reserve a future `/studio` export only for reusable explorer,
          editor-frame, diagnostics, preview, trace, or graph behavior with no
          general counterpart.
    - [x] Keep application routing, state, persistence, workers, updates, and
          search orchestration in Studio while rendering every visible control
          through CEM components/elements.
    - Completed 2026-08-21: the executable
      [`Studio UI classification`](./cem-studio-phase6.5-ui-classification.md)
      maps all 23 initial shell/workbench behaviors to the pinned Angular
      Material `22.1.1` inventory and current 49-component CEM surface. Five
      behaviors directly reuse general controls, eleven keep only application
      orchestration in Studio, and seven reserve named reusable `/studio`
      composites because their workbench capability has no Material catalog
      counterpart. The completed general `cem-tabs`/`cem-tab` prerequisite now
      opens both view-switching compositions, so all 23 classifications are
      open with no remaining general parity gate. The cached
      `verify:ui-classification` target verifies the Material/state inventories,
      exact benchmark, component owners, evidence, blockers, deferred partial
      capabilities, documentation coverage, and deterministic reports. Initial
      popup menus, radio groups, sidenav behavior, and snack-bar lifecycle are
      explicitly excluded in favor of proven visible actions, select, semantic
      grid, and persistent alert contracts.
- [x] Complete the general `cem-tabs` parity required by Studio pane navigation
      before composing the shell or Results view switchers.
    - [x] Accept the authored tab/panel vocabulary, stable relationship model,
          automatic/manual activation policy, selection event, and disabled and
          dynamic-child behavior in a component contract.
    - [x] Add failing browser tests for horizontal/vertical roving focus,
          Home/End and arrow keys, activation, focus-safe panel changes,
          programmatic selection, mutation, and accessibility semantics.
    - [x] Implement the general light-DOM component behavior, state/token and
          forced-color treatment, docs, package output, and clean-consumer
          evidence without Studio-specific state.
    - [x] Promote the pinned tabs row only after the focused tests, state matrix,
          Material parity, package verification, and aggregate component gate
          pass.
    - Completed 2026-08-21: `cem-tabs` now consumes strict inert `cem-tab`
      payloads and owns stable reciprocal tab/tabpanel IDs, horizontal and
      vertical roving focus, manual Enter/Space activation, native-disabled
      skipping, silent `selectedIndex` control, one serializable `cem-tab`
      event, persistent panel state, and focus-safe removal/disable recovery.
      Token-only normal and forced-color styling uses the shared navigation
      families, with no animation, numeric stacking, or Studio-owned state.
      The focused five-test tabs suite, 22-test shared state suite, dedicated
      forced-colors gate, package verifier, 40-row state matrix, 49-primitive
      catalog/Figma projection, and full 130-test browser aggregate pass. The
      pinned Material inventory is now 18 covered / 19 partial, and Studio's
      two view switchers are open general-component compositions. Arbitrary
      in-place payload reordering remains explicitly outside v1 because it
      requires a separate CEM projection-reconciliation contract; append,
      removal, label, and disabled changes are covered. The atomized Vitest
      target default now supplies the `env` object required by local Nx 22.7.0
      without replacing inferred commands or working directories. No live
      Figma asset was changed; `cem-tab` is recorded only as a planned inert
      payload in the repository-owned projection.
- [x] Implement the versioned IndexedDB project repository with migrations,
      atomic autosave, trash/restore, revision/hash conflicts, multi-tab
      coordination, quota diagnostics, and validated import/export.
    - Completed 2026-08-21: `@epa-wg/cem-elements` now owns clone-safe
      repository protocol v1 and its logical registry, while
      `@epa-wg/cem-studio` registers the private `studio-projects` IndexedDB
      implementation without exposing database, store, index, or transaction
      vocabulary to CEM-ML. Database v1 creates all 12 accepted stores and
      indexes; strict multi-store import, resource save, trash, and restore
      transactions enforce expected revisions, content-address source bytes by
      SHA-256, advance the durable change journal, and rebuild deterministic
      search documents. Import and export both require the injected CEM-ML
      Studio-project validator and recheck all declared resource hashes before
      returning or committing a bundle. `BroadcastChannel` is only a wake-up
      hint over the durable cursor, storage status normalizes quota/persistence
      and migration diagnostics, and seven real Chromium tests cover schema,
      atomicity, validation, conflict, search, trash/restore, multi-instance
      coordination, quota, and version failures. The aggregate Studio check
      passes lint, typecheck, browser tests, deterministic graph assembly (55
      declared assets / 62 files), package verification, and a clean consumer;
      the generic repository registry is also covered by the full 138-test
      `cem-elements` unit suite.
- [x] Project logical repository reads and storage health into CEM data slices
      through transient `repository-query` and `storage-status` resources, with
      abort/stale-result handling, durable-cursor subscription cleanup, and a
      proof that rendering can never execute repository mutations or request
      persistent storage.
    - [x] Accept and document the clone-safe resource envelopes, declaration
          attributes, read-only registry injection, and canonical CEM-ML
          processing-host lowering.
    - [x] Add focused processing-engine and real-browser fixtures for query and
          status lifecycles, superseded-request abort, stale-result rejection,
          live cursor refresh, disconnect cleanup, and the no-mutation boundary.
    - [x] Integrate the resources into the package build and aggregate browser
          verification without adding app-owned visible UI.
    - Completed 2026-08-21: canonical processing-host plans and the direct DOM
      fallback now lower both declarations into clone-safe
      `scheduled`/`loaded`/`failed` data-slice envelopes. The runtime consumes
      the frozen `CemRepositoryRegistry.readOnly()` capability, aborts
      superseded queries, rejects late revisions, refreshes query and status
      slices from durable cursor hints, and releases every query/subscription on
      replacement, disappearance, or disconnect. Focused engine/registry tests
      and a real Chromium lifecycle fixture prove JSON-parameter projection,
      storage-health reads, stale-result protection, and zero calls to
      `execute`; the package and Studio module maps include the reader runtime
      without introducing application-owned visible UI. The full
      `cem-elements:verify` integration gate passes all 59 Nx tasks, and the
      Studio aggregate passes all 37 tasks with deterministic 55-asset/62-file
      graph output, package verification, and a clean consumer.
- [ ] Build the installable PWA shell with semantic theme modes, a dedicated
      command worker, versioned app/runtime/sample caches, explicit update
      coordination, offline navigation, and recovery without project loss.
    - [x] Audit the graph-emitted static module chain before caching it and
          record the worker-safe deployment decision gate.
        - Completed 2026-08-21: module-map v2 byte-preserves declared JavaScript
          and rewrites only the page import map. The emitted CLI command worker
          still imports bare `@epa-wg/cem-ml/wasm`, which a module worker cannot
          resolve from the page map, while the browser client also imports
          `@epa-wg/cem-ml/runtime.json` outside the current JavaScript-only
          `imports` vocabulary. The Studio design records two valid resolutions
          and recommends a versioned, syntax-aware CEM-ML module-map extension
          over package-specific deployment-loader behavior. PWA implementation
          stops at this decision instead of caching a worker that the static
          output cannot start.
    - [x] Accept and implement the worker-safe module deployment contract, then
          prove the real graph-emitted CLI worker and bundled WASM command both
          online and offline without a production bundler.
        - [x] Add schema-owned module-map v3 source/destination examples and
              native valid/rejection fixtures first, covering JavaScript and
              JSON module entries, exact static/export/dynamic specifier
              rewrites, comments/string false positives, undeclared bare
              specifiers, mismatched rewrite edges, and unsafe destinations.
        - [x] Implement the v3 parser/lowering, exact declared-edge rewrite with
              byte preservation outside specifier spans, deterministic
              source/output digest evidence, schema
              registration, reports, and v1/v2 compatibility lane.
        - [x] Adopt v3 in Studio and add a static-output browser fixture that
              executes the real CLI worker and bundled WASM online and offline.
        - Completed 2026-08-21: schema-owned module-map v3 adds typed
          JavaScript/JSON imports and exact `moduleImports` edges, with native
          acceptance/rejection fixtures, v1/v2 compatibility, and deterministic
          source/output digest evidence in engine and CLI reports. Studio's
          paired v3 maps now own 57 assets, including the CLI worker, runtime
          JSON, WASM wrapper/binary, and complete component chain; CEM-ML emits
          64 static files and the destination map used by versioned
          shell/runtime caches. Real Chromium executes the CLI `version` command
          through the dedicated graph-emitted worker online, reloads with the
          network disabled, and executes it again from Cache Storage. No
          production bundler or package-specific deployment loader is involved.
    - [x] Compose the installable CEM-component shell, five semantic theme
          modes, evolve the accepted shell/runtime caches with a sample-cache
          policy, add explicit safe-update
          barrier, offline navigation, and IndexedDB project-survival test.
        - [x] Fixture: install the production CEM component primitives, render
              the shell with CEM controls only, and persist each of the five
              theme modes named by the repository theme Markdown.
        - [x] Fixture: expose browser-provided install readiness and block a
              waiting service-worker activation during active work or until a
              dirty project has been persisted.
        - [x] Fixture: precache separately versioned shell, runtime, and sample
              groups, then navigate to an application route with the network
              disabled.
        - [x] Fixture: import a project into IndexedDB, reload the offline app,
              and export the same project bytes from the surviving database.
        - Completed 2026-08-21: Studio now installs the production
          `cem-components` declarations through `cem-elements` and composes its
          visible controls exclusively from that set. The five semantic modes
          are the exact classes named by the theme Markdown and persist locally.
          Browser install readiness remains browser-owned. A user-visible
          update action releases a waiting worker only after active work is
          idle and dirty state persists successfully; persistence failure keeps
          the prior worker active. Cache inventory v2 owns separate versioned
          shell, runtime, and sample groups, with an empty graph-emitted sample
          catalog reserved for the next Feature Tour item. Real Chromium proves
          the 58-asset/66-file deterministic deployment, scope-safe deep-route
          fallback, exact IndexedDB project survival, and CLI worker/WASM
          execution both online and offline.
- [x] Generate an editable CEM-ML Feature Tour seed from actual schema-package
      examples and browser capabilities, then verify every advertised example
      and preserve user copies across seed upgrades.
    - [x] Fixture: generate exactly one manifest-declared passing example for
          every registered schema package when the browser capability manifest
          advertises `validate`, with deterministic identities and source
          hashes and no hand-copied example content.
    - [x] Fixture: validate the generated Studio-project manifest and every
          advertised source through native CEM-ML, then copy the original
          package-example bytes through a generated CEM-ML transformation
          graph into the versioned sample cache.
    - [x] Fixture: load and hash-check the graph-emitted seed in Chromium, use
          the real browser worker to validate every advertised example, and
          reject any catalog/runtime capability drift.
    - [x] Fixture: install an editable IndexedDB copy with an identity separate
          from the read-only seed, preserve an edited copy byte-for-byte across
          a simulated seed upgrade, and create a separately identified reset
          copy from the upgraded seed.
    - Completed 2026-08-21: the deterministic generator selects the first
      manifest-declared passing example from all 30 registered schema packages,
      records exact source hashes and browser capability identity, and discovers
      referenced local schema resources transitively. Its generated CEM-ML
      graph emits the 61-resource Studio project, run configurations, original
      example/dependency bytes, catalog, and 64-URL offline sample inventory.
      Native CEM-ML validates the project and all advertised sources. Real
      Chromium integrity-checks the seed, validates all 30 examples through one
      reusable browser command worker using the `cem-studio://` inline-resource
      resolver, and proves the cache online/offline. IndexedDB keeps the
      read-only seed identity separate from `feature-tour`, preserves edited or
      trashed copies across upgrades, and creates `feature-tour-2` on reset.
- [x] Deliver the first offline vertical slice: edit one Feature Tour CEM
      resource, persist and reload its exact revision, validate it through the
      browser worker, and navigate structured diagnostics, report data, and
      source-map provenance with CEM controls.
    - [x] Fixture: save edited CEM bytes with the expected project/resource
          revisions, reload the committed bundle, and prove exact bytes, hash,
          and monotonically advanced revisions before validation.
    - [x] Fixture: validate the committed revision through the real browser
          worker and retain its native structured diagnostics, report summary,
          execution identity, and source-map frames without a JSON reshaping
          boundary in CEM-ML.
    - [x] Fixture: compose editing, save/reload actions, validation status,
          result tabs, diagnostic navigation, report rows, and provenance
          navigation exclusively from production `cem-components` controls on
          the `cem-elements` runtime.
    - [x] Fixture: reload the edited project while offline, revalidate the
          exact persisted revision, and mark an in-flight result stale when the
          draft or durable revision advances before that result is presented.
    - Completed 2026-08-21: the first workbench opens the generated CEM-ML
      example in `cem-textarea`, saves through the IndexedDB optimistic-revision
      command, reloads and verifies the exact committed bytes/hash/revisions,
      and passes those durable project/resource revisions into the browser
      command ledger. Native validation diagnostics, report summary, execution
      identity, and origin-first source-map frames remain structured in the
      workbench. `cem-badge`, `cem-alert`, `cem-action`, `cem-tabs`, selectable
      `cem-list`, and `cem-table` own every visible interaction and status.
      Chromium proves invalid diagnostic/report/provenance navigation, exact
      offline reload and revalidation through the WASM worker, while a browser
      concurrency fixture proves results become stale when a newer draft lands
      during validation.
- [ ] Add parse and inspect projections plus the lossless bidirectional CLI
      Command view for copy, edit, transactional Apply, and current/existing/new
      page targets.
    - [x] Fixture: execute CEM-ML `parse` (`ast` and `events`) and every
          browser-capable `inspect` view against exact IndexedDB bytes with the
          durable project/resource revisions carried into the command ledger.
    - [x] Fixture: retain the native command result, execution identity,
          diagnostics, source maps, and target-native CEM-ML output artifact;
          render the CEM-ML bytes without routing them through a JSON AST/DOM
          handoff.
    - [x] Fixture: reproduce browser command-service CEM events presentation
          with the native engine, preserve legal tabs and line endings in CEM
          quoted attributes, and continue rejecting non-text control bytes.
    - [x] Fixture: select and run parse/inspect projections, show status and
          read-only output, and switch result panes exclusively with production
          `cem-components` controls on the shared `cem-elements` runtime.
    - [x] Fixture: prove real-worker parse/inspect output online and after an
          offline reload, and mark a projection stale when the draft or durable
          revision advances while it is running.
    - Completed 2026-08-21: the Feature Tour workbench executes target-native
      CEM-ML `parse` AST/events projections and all six browser `inspect` views
      through the Rust-owned command grammar and real dedicated worker. Each
      request reads the exact IndexedDB resource and dependency bytes, carries
      optimistic project/resource revisions into the command ledger, retains
      the native result/identity/diagnostics/source maps, verifies the published
      output bytes by length and SHA-256, and renders those CEM-ML bytes in
      `cem-select`, `cem-action`, `cem-alert`, `cem-tabs`, `cem-textarea`, and
      `cem-table` controls. Chromium proves the projection matrix, stale draft
      handling, and identical online/offline output. The typed CEM writer now
      preserves tabs and line endings in quoted event payloads while rejecting
      non-text controls. The parent remains open for the lossless bidirectional
      CLI Command view and transactional Apply target workflow.
- [ ] Add conversion, query, transformation, trace, and transformation-graph
      workbenches without duplicating engine semantics or component behavior.
- [ ] Add the opt-in File System Access provider with explicit permissions,
      retained provider bindings, external-change detection, conflict-safe
      write-back, and complete IndexedDB/import-export fallback.
- [ ] Close Phase 6.5 with bounded/sandboxed previews, source/result limits,
      accessibility and forced-color coverage, offline/update/security tests,
      package/install verification, dependency audit, and synchronized release
      evidence.

## Later Non-Figma Phase Gates

Expand each gate into its task-level checklist when it becomes the immediate
goal. These gates deliberately keep the deferred Figma work from becoming active
before the non-Figma roadmap is complete.

- [ ] Complete Phase 8 native platform package hardening and toolchain validation.
- [ ] Complete Phase 9 release, governance, and compatibility work for the code,
      docs, Studio, and native distribution surfaces.

## Phase 10 Checklist — Deferred Figma UI Kit

Phase 4 component names, variants, executable states, and accessibility semantics
are complete in the archived checklist, and the Phase 10 repository foundation
already owns the five-mode token gate and 49-primitive executable Figma inventory.
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
