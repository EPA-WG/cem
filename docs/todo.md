# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/). The
[2026-09-27 snapshot](archive/todo-snapshot-2026-09-27.md) preserves completed
items and the context referenced by older progress notes. Later completions are
recorded in the [follow-up log](archive/todo-completed-2026-09-27.md).

## Immediate: Destructive pending dark-mode stripes

- [x] Fixture: darken the destructive pending stripe in dark and contrast-dark themes; verify endpoint separation, readable text, animation and browser preview.

## Immediate: Destructive pending stripe contrast

- [x] Fixture: lighten destructive pending stripes through the canonical theme, preserve readable text and motion across modes, and preview in the browser.

## Immediate: Public CSS property ownership

- [x] Record public component CSS property ownership and the mandatory theme/component conflict decision.
- [x] Fixture: action consumes existing intent/state theme tokens directly; verify container inheritance, instance overrides, restoration and state behavior.
- [x] Audit select indicator aliases and popup stacking ownership: adopt existing theme indicators; promote public popup stacking into D4 (user decisions).
- [x] Fixture: generate/export --cem-select-popup-z-index from D4; verify inherited/instance popup stacking and pointer access.
- [x] Fixture: remove redundant select focus/selection color aliases and verify theme overrides across underline/outline, pending, invalid, disabled, focused and expanded states (21 action/select stories pass).
- [x] Preserve all Markdown-generated theme variables; retain select's five existing state/geometry calculations. Record optional unused-code optimization for consumer builds in the roadmap, never CEM release pruning.
- [x] Verify source and packaged playgrounds after theme ownership migration; record evidence.
- [x] Commit and push public CSS ownership and theme adoption changes.

## Immediate: Legacy Demo Case Coverage

- [x] Remove redundant action forced-color gradient/shadow resets and consolidate
      bend-radius binding; verify action states and source/package accessibility fallbacks.

- [x] Fixture: canonical action and select respect native host hidden states,
      initial/live visibility, keyboard exclusion, retained control/state and
      case-insensitive until-found semantics. Document the shared convention,
      remove the legacy invisible demo and verify source/package galleries.

- [x] Align the legacy custom-element package license and all eight Material
      demo footers with the repository MIT license; clarify historical records.
      Verified matching license text, package metadata and footer references.

- [x] Keep the action variations matrix limited to intent and shape; remove its
      disabled column, retain interaction-state examples, and verify both galleries.

- [x] Remove the unsupported action alignment demo and redundant icon-example
      labels; retain the fallback-label example and verify the source/package gallery.

- [x] Fixture: disabled loading actions retain pending animation while native
      disabled semantics block activation; verify all intents, submit workflow,
      loading completion and reduced-motion/forced-colors package previews.

- [x] Fixture: restore canonical pending gradients (45-degree tilt, seamless
      two-second loop) in theme specs, token generation, generator previews and
      cem-action. Verify intent colors, movement across cycles, disabled/loading
      toggles, reduced motion, forced colors, and source/package previews.

- [x] Fixture: action loading transitions once into intent pending colors;
      verify all five intents, live toggling, disabled precedence, stable geometry,
      hover/keyboard-focus replay, and reduced-motion behavior in the playground.

- [x] Reset the native button border for every `cem-action` intent; verify
      action stories and source/package playgrounds retain zebra focus and
      the forced-colors outline.

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

### Remaining declarative UI migration

- [x] Fixture: restore the action preview's native hover, active, keyboard-focus
      and disabled states with CEM tokens. Verify native disabled suppression,
      stable geometry, and state paint in source/dist/installed-package previews.
      Check each zebra stripe against the focused element's current colors;
      inherited root-resolved ring recipes must not erase the focus stripe.
    - [x] Fixture: give each action sample a consistent theme intent and add
          a matrix covering all five intents and sharp/smooth/round/disabled
          controls. Verify each intent's actual default/hover/active/disabled paint.
    - [x] Fixture: retain runtime host-attribute counts during legacy template
          conversion instead of folding them against the static source document.
          Preserve static document count/sum evaluation and verify native lowering
          before rebuilding WASM and checking disabled browser behavior.
      Validation: all 74 native converter tests and the custom-element lint gate
      pass. Source, dist and installed-package checks cover nine cards / 52
      buttons, all five intents, native input states, and zebra across five theme
      modes. DevTools confirms keyboard focus and native disabled controls.
      See [state and variation evidence](archive/todo-completed-2026-09-27.md#action-preview-states-variations-and-zebra).

- [x] Fixture: verify the legacy action demo's actual sharp, smooth and round
      corner radii against generated CEM Shape/Controls tokens, including live
      Bend changes and source/dist/installed-package theme delivery. Keep styles
      declaration-owned and preserve page-level module-map loading.
      Checks cover multiline shape overrides and compact/forgiving sizes.
      See [curvature evidence](archive/todo-completed-2026-09-27.md#action-theme-curvature).

- [ ] Migrate every legacy `cem-components` member into its own
      `src/components/<cem-tag>/<cem-tag>.xhtml` folder with embedded,
      once-per-declaration scope-contract CEM-token `<style>` and colocated
      CSF Next `<cem-tag>.stories.ts` `play` tests, moving missing reusable behavior into
      `cem-elements`, until both migration targets are zero.
    - [ ] Investigate the legacy component suite failures before claiming a
          green package gate: the cutover trial reports 80 passing / 49 failing;
          empty disabled state and collapsible navigation failures reproduce
          with committed legacy sources. Establish the remaining baseline and
          repair shared semantics or migrate owners without weakening tests.
          CSS closure comparison (2026-09-27): rebuilt commit `5f8e7172` and
          the native-CSS default both produce 81 passing / 48 failing tests,
          with the same 48 test names. No new component failures; the package
          gate remains red. Rebuild `cem-elements` before comparing this suite,
          because these tests import its packaged output.
          Follow-up 2026-09-27: the unchanged suite again reports 81 passing /
          48 failing. The [failure inventory](cem-components-baseline-2026-09-27.md)
          records each failed test, the boolean-presence mismatch, and the
          separate datepicker template-brace defect. Owner repairs remain open.
        - [x] Fixture: pin native template attribute-presence semantics for
              absent, empty, `"false"`, and `"true"` host values. Verify the
              explicit presence expression used by canonical components and
              distinguish it from legacy value/truthiness bindings.
        - [x] Fixture: repair the shared substrate harness's required-input
              declaration using explicit attribute presence. Preserve native
              form-validity/reset assertions and verify absent, empty,
              `"false"`, and `"true"` values plus live removal without replacing
              the native input. Compare the full legacy failure inventory.
              The full harness gate passes. The aggregate suite is now
              82 passing / 47 failing, with only this failure removed and no
              new failing test names. See the
              [repair evidence](archive/todo-completed-2026-09-27.md#shared-harness-required-input-repair).
    - [x] Fixture: load the legacy material action demo through page-level
          module maps and repository-owned runtime/demo modules, matching the
          `cem-elements` demo convention. Verify source and packaged asset
          loading and the IDE preview URL through DevTools when connected.
          Review the archive inventory against native CSS runtime additions,
          excluding browser-story adapters from the vendored package.
          Validation: source, dist and clean installed-package browser checks
          pass, as does the package lint target and its dependencies. DevTools
          verifies all seven cards, 29 buttons, successful local JS/WASM loads,
          no console errors, and the Bend interaction at the IDE preview URL.
          See [closure evidence](archive/todo-completed-2026-09-27.md#action-demo-local-preview).
        - [x] Fixture: preserve the adapter smoke test's native declaration and
              implicit-instance scope checks across CSS emitter whitespace,
              retaining all computed-style and isolation assertions.
    - [x] Fixture: verify canonical action loading in workflow/demo and Site
          search/interactive consumers, including packaged XHTML asset delivery.
    - [x] Fixture: cover explicit submit/reset, required-input validation,
          submitter name/value and form overrides, external form ownership,
          disabled suppression and cancellation, while preserving default
          non-submitting command behavior.
    - [x] Fixture: migrate action unit coverage to colocated CSF Next plays,
          retaining exact state reflection, slot/fallback labels, native click,
          keyboard/disabled behavior, node identity, hover/active token pairs
          and declaration-owned style installation.
    - [x] Remove the legacy action registry member/global styles, migrate
          affected consumers to canonical XHTML, and update package/catalog/
          style/state evidence before reducing the migration inventory.
          Completed: 2 canonical / 47 legacy components. See
          [cutover evidence](archive/todo-completed-2026-09-27.md#canonical-action-cutover).
- [ ] Investigate runtime suite instability: the action-cutover full run passed
      311/312 stories; `Payload Css Readiness And Hydration` expected one style
      but observed zero. Its complete 12-story file passed on isolated rerun.
      Preserve the readiness assertion and reproduce before changing behavior.
- [x] Mark the old Material action page as legacy, link its canonical CEM
      component, and document the same transition step for other Material pages.
- [x] Fixture: a held native mouse click on a select option keeps combobox
      focus and the popup open until release, then commits once; outside clicks
      close the popup without taking focus back. Verify every
      action playground choice with a real press/release delay.
- [x] Fixture: action playground enumerated options use visible, labeled radio
      groups; verify initial selection, mouse and keyboard changes, exclusivity
      and retained selection after other property edits in source/package previews.
- [ ] Implement the accepted [component development pattern](component-development-pattern.md)
      first for `cem-action`, then apply it to `cem-select` and future components.
    - [x] Fixture: event bindings read the live string value of form-associated
          custom controls through `$target.value`, value aliases, defaults and
          serialized event targets; retain native input behavior and reject
          non-form/object-valued targets.
    - [x] Fixture: companion property form drives one canonical action instance
          across intent, bend, label, type, disabled, loading and expanded;
          native hover/active/keyboard focus remain usable.
    - [x] Fixture: ship the full action gallery and its icon declaration with
          the component playground, using native import-map rewriting; verify
          isolated archive loading without the legacy package installed.
    - [x] Fixture: source-only view fetches the canonical XHTML without executing
          another declaration; link the full demo and verify IDE/module-map and
          packaged preview loading. Update the legacy Material page to link
          the new playground when available.
    - [ ] Fixture: apply the companion property playground/source-view pattern
          to `cem-select`, including source and isolated package preview checks.
    - [ ] Fixture: generate the release XHTML bundle through the CEM AST pipeline
          and compare individual versus `#ID` loading, relative dependencies,
          style ownership and duplicate registration; ship source and playground.
- [ ] Migrate remaining Studio and Site visible DOM construction, UI listeners,
      and state projection to XHTML/CEM-ML, retaining JavaScript only for non-UI
      services and host adapters.

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.
