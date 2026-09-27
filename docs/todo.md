# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/). The
[2026-09-27 snapshot](archive/todo-snapshot-2026-09-27.md) preserves completed
items and the context referenced by older progress notes. Later completions are
recorded in the [follow-up log](archive/todo-completed-2026-09-27.md).

## Immediate: Legacy Demo Case Coverage

The local `~/aWork/custom-element/demo/` comparison found that current-gallery
verification is not a one-to-one legacy case audit. In particular, anonymous
whole-file XSLT loading and XSLT selected by `file.xhtml#id` initially lacked
gallery cases; the existing `embedded-xsl` fixture actually contains CEM-ML.

- [ ] Stabilize browser startup waits under parallel Storybook load, after
      DATA-CELL-MATCH-1: the unchanged scoped-CSS and legacy icon-link stories
      intermittently exhaust their short frame/two-second waits, although
      isolated and earlier full runs pass. Prefer declaration/render readiness
      over incidental delays. Also diagnose the source-loaded cell-overrides
      stock sample's initial-warning timeout seen once in the full gallery;
      six focused standalone/source-loaded runs passed without reproducing it.
      Establish its cause before treating it as the same startup issue.
      2026-09-21: scoped-CSS, hex-grid and legacy parity helpers now wait on
      rendered output/style/image conditions within the 30-second story budget,
      replacing frame-count and two-second cutoffs. All 29 cases in the five
      affected story files pass together and in the full parallel suite. The
      table's invalid-input alert also needed its existing ten-second interaction
      budget under parallel load. Final full result: 179 passed, three separate
      CEM-QL/local-storage failures tracked below. Stock-warning checks pass in
      both focused gallery modes; its earlier intermittent cause remains open.
      After native storage migration, all 190 browser stories and the full
      gallery (28 standalone pages, 34 source-loaded documents) pass. The
      original intermittent stock-warning cause still needs investigation.
      Follow-up 2026-09-21: all 198 current stories and 24 concurrent startup
      probes (12 gallery-helper / 12 real demo-component, with three declaration
      release schedules) pass. No lost markers, missing mounts or diagnostics
      reproduce. The user selected [continued investigation](browser-stabilization-review.tmp.md#startup-follow-up-decision-continue-investigation);
      keep this item active before the authored-sample coverage inventory.
      Sequencing update 2026-09-23: the user approved beginning that inventory
      while historical timeout attribution remains open on traced recurrence.
      This supersedes the earlier stabilization-first order, not its open
      investigations. The inventory follow-up below records the accepted work.
      Follow-up 2026-09-24: the cell-overrides audit and real source-harness
      component pass their focused stories and all 31 standalone / 37
      source-loaded gallery documents. The referrer matrix's scalar-URL frame
      timeout recurred in both final aggregate runs and a dedicated full
      Storybook rerun (219/220 each), temporarily blocking aggregate sign-off.
      The specific referrer fixture below now attributes and corrects it.
  - [ ] Audit remaining frame-count readiness helpers and aggregate-count
        predicates in demo stories. Check the authored instance inventory and
        available lifecycle signals before migrating each affected fixture;
        keep historical stock-timeout attribution separate from fixture fixes.
        Prioritize the newly observed failures: external-src declaration loading
        waits 120 frames for its first button; NPM-version samples wait 200
        frames for the default selection; location samples wait 180 frames for
        both readers. Capture declaration/render/worker state before attributing
        these failures or changing waits. Normal coverage passes, but that alone
        does not establish their failure cause under combined load.
        Progress 2026-09-22: opt-in lifecycle and frame-wait traces reproduce
        external-src's 120-frame timeout during pending rendering after clean
        declaration settlement. Its render settles 49 ms later without errors;
        later assertions remain unverified. Propose lifecycle-based readiness
        within the existing story limit. NPM/location pass this repeat and still
        require captured failure state before attribution; see the attribution
        section linked above. No waits changed in this investigation.
        The hook-eligibility validation also observes two declaration-source
        retry stories failing their default 1,000 ms `waitFor` after declaration
        settlement. Their four-story isolated run and a subsequent full suite
        pass; capture render/worker state on recurrence before attributing the
        failure or migrating these waits. Do not change retry/cache behavior
        based on a missing paragraph at a polling deadline.
        Follow-up 2026-09-22: external-src and retry fixtures now await existing
        lifecycle settlement (completed item below). The broader audit remains
        open for NPM/location failure attribution and other aggregate/frame
        predicates; the thirteen-file inventory is recorded in the
        [readiness audit](browser-stabilization-review.tmp.md#declaration-and-retry-readiness-audit).
        Follow-up 2026-09-23: focused overlapping load reproduces NPM's
        premature startup wait twice. Awaiting existing initial settlement
        before the unchanged HTTP predicates passes the same workload, both
        204-test full suites and all 160 stock probes. Location passes under
        focused load; its failure and the broader inventory remain open. See
        [NPM/location evidence](browser-stabilization-review.tmp.md#npm-and-location-readiness-under-overlapping-load).
    - [ ] Capture HTTP's newly observed 300-frame initial article-count
          timeout with declaration/render/worker tracing before choosing its
          correction. It failed once in the storage diagnostic workload;
          that run did not observe HTTP lifecycle state or verify its later
          resource assertions.
          Follow-up 2026-09-23: two traced 16-file runs and a focused five-file
          startup workload pass 26/26, 26/26 and 15/15, with 96/96 stock probes.
          The timeout remains unattributed; no wait or runtime change is made.
          Permanent opt-in tracing records lifecycle state and per-card article
          counts on startup success/failure, rethrowing the original error.
          Resume attribution on a traced recurrence. The native storage
          diagnostic follow-up below is complete; see
          [HTTP observation](browser-stabilization-review.tmp.md#http-startup-observation-without-a-reproduced-failure).
      - [ ] On a traced HTTP failure, distinguish declaration/library startup,
            current rendering and future HTTP completion before changing the
            fixture. If startup polling is premature, use existing lifecycle
            settlement before checking each authored instance; preserve the
            resource assertions and 30-second story deadline.
## Declarative UI Architecture Correction

### Native CSS `@scope` migration

- [ ] Support scoped module maps inside `cem-element` styles through typed CSS
      adoption into the CEM AST.
    - Resolve CSS import expressions (`@import`), `url(...)` references and
          other CSS URL-bearing constructs through the module map from the
          closest scope, using the shared resolver and retained CSS AST.
          Completed in the opt-in retained runtime; default cutover is separate.
        - [ ] Connect native closure emission to stable effective stylesheet
              ownership, cache invalidation and browser lifecycle only after
              completing the remaining syntax profiles and diagnostic gates.
    - [ ] Fixture: support atomic native CSS replacement in streamed Edge updates
          when payloads or consuming contexts change, including cancellation,
          stale updates and browser ownership cleanup before removing the update
          capability guard.
        - [ ] Coordinate declaration and instance leases with Edge patch commits,
              including context markers shared by several declaration owners;
              extend replacement responses/state and verify stale transaction
              rejection before enabling changes to CSS inputs in the Edge host.
            - [ ] Bind prepared CSS publication to validated Edge patch transactions.
    - [ ] Fixture: trial retained CSS as the default browser runtime; audit
          Edge/SSR and hydration compatibility, run both browser lanes and
          component verification, and remove obsolete compiler paths only after
          their remaining result-style callers have an explicit replacement.

### Remaining declarative UI migration

- [ ] Migrate every legacy `cem-components` member into its own
      `src/components/<cem-tag>/<cem-tag>.xhtml` folder with embedded,
      once-per-declaration scope-contract CEM-token `<style>` and colocated
      CSF Next `<cem-tag>.stories.ts` `play` tests, moving missing reusable behavior into
      `cem-elements`, until both migration targets are zero.
    - [ ] Implement v3 XHTML deployment before resuming the action cutover.
        - Native support complete: the v3 schema gate, 27 focused module-map
          tests, v1/v2/v3 CLI publication tests, cache-key fixture, TypeScript
          projections and WASM build pass. Generated examples document XHTML.
        - [ ] Adopt typed v3 maps and exact module edges in Site, then prove
              static search/interactive XHTML delivery before registry removal.
    - [ ] Investigate the legacy component suite failures before claiming a
          green package gate: the cutover trial reports 80 passing / 49 failing;
          empty disabled state and collapsible navigation failures reproduce
          with committed legacy sources. Establish the remaining baseline and
          repair shared semantics or migrate owners without weakening tests.
    - [ ] Diagnose the restored Site build's token-browser failure:
          `cem.transform_template.adapter_failed` reports the 64-level / 4096-value
          JSON import limit before asset deployment. Resolve before end-to-end
          XHTML cutover verification; do not attribute it to the new resource.
    - [ ] Fixture: verify canonical action loading in workflow/demo and Site
          search/interactive consumers, including packaged XHTML asset delivery.
    - [ ] Fixture: cover explicit submit/reset, required-input validation,
          submitter name/value and form overrides, external form ownership,
          disabled suppression and cancellation, while preserving default
          non-submitting command behavior.
    - [ ] Fixture: migrate action unit coverage to colocated CSF Next plays,
          retaining exact state reflection, slot/fallback labels, native click,
          keyboard/disabled behavior, node identity, hover/active token pairs
          and declaration-owned style installation.
    - [ ] Remove the legacy action registry member/global styles, migrate
          affected consumers to canonical XHTML, and update package/catalog/
          style/state evidence before reducing the migration inventory.
- [ ] Migrate remaining Studio and Site visible DOM construction, UI listeners,
      and state projection to XHTML/CEM-ML, retaining JavaScript only for non-UI
      services and host adapters.

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final Phases 10 and 11, after Phase 9 release governance.

## Later Non-Figma Phase Gates

Expand each gate into its task-level checklist when it becomes the immediate
goal. These gates deliberately keep the deferred Figma work from becoming active
before the non-Figma roadmap is complete.

## Phase 10 Checklist — Deferred Figma UI Kit

Phase 4 component names, variants, executable states, and accessibility semantics
are complete in the archived checklist, and the Phase 10 repository foundation
already owns the five-mode token gate and 49-primitive executable Figma inventory.
The remaining Phase 10 work is reviewed canvas work in the canonical CEM UI Kit
and must not start before Phase 9 is complete.

- [ ] Build and review the `02 Foundations` page from native CEM variables.
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
