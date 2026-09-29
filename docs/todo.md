# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/). The
[2026-09-27 snapshot](archive/todo-snapshot-2026-09-27.md) preserves completed
items and the context referenced by older progress notes. Later completions are
recorded in the [follow-up log](archive/todo-completed-2026-09-27.md).

## Completed: Rename action loading to pending

- [x] Rename the canonical `cem-action` attribute and action state to `pending`;
      update theme/component docs, examples, playgrounds, stories and state coverage.
- [x] Verify all 17 action stories, declarative architecture, state matrix and catalog.
- [x] Verify source and installed-package playgrounds and published package contents.

All 17 action stories, catalog/state-matrix checks, package build and source/installed
playgrounds pass. The public action attribute is now `pending="true"`.

## Completed: Extra-light red destructive loading stripe

- [x] Use the palette extra-light red (`--cem-color-red-xl`, `#ffb4ab`) for
      the destructive light contrast outline; preserve dark mode and normal fills.
- [x] Fixture: verify the exact extra-light endpoint and retained motion,
      then run browser/package checks, commit and push.

All 17 action stories pass, plus source/installed-package playgrounds with
visible motion, reduced-motion and forced-colors checks. Package, style, material
parity and lint pass (no errors; 48 existing warnings).

## Completed: Brighter light contrast loading outlines

- [x] Decision: use 40% white for light contrast outline stripes only; preserve
      ordinary fills and dark mode (user approved option 1).
- [x] Fixture: every light contrast intent uses the brighter endpoint, with
      stripe separation above 3:1; normal fills and dark contours retain their colors.
- [x] Update theme recipes and documentation, verify, commit and push.

All 17 action stories pass, including the explicit light-outline exception and
restoration of ordinary fills after switching themes. Source and installed-package
playgrounds pass visible-frame motion, reduced-motion and forced-colors checks.
Package, style, material parity and lint pass (48 existing warnings, no errors).

## Completed: Sharper loading gradient and light contrast motion

- [x] Apply the approved 20% hold / 60% blend / 20% hold pattern to all
      action loading gradients and theme previews, preserving endpoints/timing.
- [x] Fixture: verify solid color holds and actual painted motion for every
      intent in light/dark contrast playgrounds; investigate reported static light mode.
- [x] Verify browser/package checks, commit and push.

Chromium showed moving pixels for all five intents in both contrast modes before
and after the change; a stopped light-mode animation was not reproduced. The new
solid color holds make stripe boundaries sharper without changing the palette,
text contrast, timing or reduced-motion behavior. All 17 action stories and
source/installed-package playground checks pass, including actual frame comparisons,
plus package, style, material parity and lint (no errors; 48 existing warnings).

## Completed: Stronger loading stripe contrast

- [x] Fixture: assert stripe separation above 1.8:1 and text contrast of at least
      4.5:1 for every intent in light/dark, retaining normal/contrast parity.
- [x] Add coherent theme-owned stripe endpoints across intents, update action
      and theme preview, verify accessibility fallbacks, commit and push.

Measured light/dark stripe separation: 1.83:1–2.72:1; minimum text contrast:
4.58:1. All 17 action stories pass, including gradient midpoint contrast and
normal/contrast parity. Source/package playgrounds, theme preview, package, style,
material parity and declarative checks pass. Lint has no errors (48 existing
warnings). Native system-palette stripe endpoints retain their previous behavior.

## Completed: Intent-colored pending animation

- [x] Fixture: verify every action intent has matching loading colors and motion
      in light/contrast-light and dark/contrast-dark, including live switching.
- [x] Update theme recipes, canonical action and generated preview; verify, commit
      and push. Preserve contrast contour geometry and accessibility fallbacks.

Verification: all 17 canonical action stories; source and installed-package
playgrounds (including reduced motion and forced colors); package, style contract,
material parity and declarative architecture gates. Existing per-control normal
fill color overrides remain covered.

## Completed: Shared demo property layout

- [x] Put property fieldsets in a wrapping flex container and stack their options
      vertically using shared `cem-components/demo.css` classes in every property
      playground; include the shared stylesheet in published packages.

## Completed: Canonical field controls

- [x] Fixture: both fields remain borderless at rest, hover, focus, readonly,
      invalid, busy and disabled, including themes and forced colors; hover
      retains a visible theme-owned indicator.

- [x] Audit canonical prototypes for both fields; identify the form-reset contract gap.
- [x] Decision: approve an opt-in shared form-control capability in `cem-elements`
      before resuming migration. See [the migration gate](field-controls-migration-gate.md).
      User approved the recommended shared capability.
- [x] Fixture: prove shared form reset, one submission owner, native validity,
      fieldset disabling and state restoration before component migration.
- [x] Fixture: preserve native Enter submission, default-submit cancellation,
      disabled submitters and multiple-control suppression in the shared capability.
- [x] Implement and verify the shared form-control capability.
- [x] Migrate `cem-field` and `cem-text-field` with explicit boolean-presence
      semantics, full attribute coverage and canonical source playgrounds.
- [x] Fixture: verify field attributes, boolean presence, input/value events, form reset,
      retained busy state, scoped indicator paint and source/package playgrounds.
- [x] Compare remaining legacy state failures after the migration.

Ten canonical field stories and six shared form/generic-input stories pass.
Primitive tests pass (9/9). Legacy state tests retain the 4-pass/15-fail baseline;
workflow tests pass 13/14, with the remaining failure at legacy checkbox required
presence. Field assertions pass. Source/package playgrounds, package, style,
state-matrix and forced-color checks pass. See the
[field contract](../packages/cem-components/docs/field-controls-contract.md).

## Completed: Required component galleries

- [x] Require a full examples and variation matrix gallery for every component,
      linked from its property playground and shipped in the package.
- [x] Add missing galleries for all six introduced canonical components.
- [x] Fixture: verify select outline styles compile without diagnostics and the
      gallery applies the outline appearance token.
- [x] Fixture: verify every canonical playground links its gallery, renders real
      examples and theme/state variations, and works from source and package archives.

All seven canonical galleries pass source and isolated-package browser checks.
Package verification passes (119 packed files), and all 12 select stories pass.
The select outline selector now stays within the scoped-CSS specificity limit.

## Completed: Component demo index

- [x] Define `packages/cem-components/index.html` as the required component listing
      in the demo protocol, with exact component names linking to galleries.
- [x] List all seven canonical components alphabetically and verify their labels
      and local gallery targets.

## Completed: Gallery introductions and index destinations

- [x] Point component index entries directly to their full examples galleries.
- [x] Require and add a brief component description and relationships paragraph
      immediately after each gallery heading, with links to related galleries.
- [x] Verify all seven index entries, introduction placement, related links,
      and generated gallery pages.

## Next: Canonical textarea

- [ ] Migrate `cem-textarea` using the shared form-control capability, with scoped
      indicator CSS, a canonical source/package playground and its required gallery.
- [ ] Fixture: cover multiline editing, reset, boolean presence, validation,
      retained focus/selection and forced colors; compare remaining legacy failures.

## Completed: Native radio selection feedback

- [x] Decision: radios use their native dot for selection; checkbox and switch
      styling stays unchanged. Radio focus, invalid and pending outlines follow
      the circular native control (user approved).
- [x] Fixture: checked radios have no extra selection shadow; circular focus,
      invalid and pending feedback remains visible across themes and forced colors.
- [x] Update shared styles and contracts, verify and commit/push.

## Completed: Canonical icon-button and menu-item migration

- [x] Migrate `cem-icon-button` and `cem-menu-item` into per-component XHTML,
      using explicit disabled presence and removing their legacy ownership.
- [x] Fixture: colocated stories for every implemented attribute, native
      pointer/keyboard activation and disabled behavior; companion playgrounds.
- [x] Compare the complete legacy state failure inventory after migration;
      verify source/package output, commit and push.

Verification and the remaining 4-pass/15-fail legacy state inventory are recorded
in [command component migration](command-component-migration.md).

## Completed: Batch renders after event bursts

- [x] Decision: let bubbling and related synchronous events finish before one
      asynchronous render of their accumulated state (user instruction).
- [x] Fixture: nested/bubbling events, microtask follow-ups, host attributes and
      behavior updates produce one render; retain latest event metadata and
      make `whenRenderSettled` wait for the queued work.
- [x] Fixture: disconnected queued instances do not render; reconnected
      instances render current state and stale in-flight results stay rejected.
- [x] Update runtime scheduling and the repeated-click contract, verify and
      commit/push. Continue owner migration after this shared capability.

The [legacy investigation](legacy-action-state-investigation.md) remains the
owner-failure baseline. Event payloads must be captured while `currentTarget` is
available; batching concerns rendering, not discarding later events.

All 348 runtime browser stories and 585 unit tests pass. Source/package
playgrounds pass; the legacy state baseline is unchanged. See
[event render batching](event-render-batching.md).

## Completed: Release XHTML bundle and source context

- [x] Fixture: characterize relative module URLs in individual and combined
      fragment sources, including retained `xml:base`.
- [x] Decision: preserve original source-base metadata in the shared loader
      (user approved the recommended approach).
- [x] Fixture: native source-base chain validation and browser fragment parity,
      invalid metadata, source identity, dependencies and styles.

The generated `components.xhtml` bundle and its playground now ship alongside
canonical source. The shared loader preserves original resource bases through
`xml:base`, with native validation. Source and isolated-package checks pass;
see [delivery evidence](component-bundle-source-context.md).

## Completed: Component galleries and module-URL startup settlement

- [x] Decision: batch sibling module-URL results before rendering, matching
      the DOM fallback (approved by the user on 2026-09-28).
- [x] Fixture: reproduce partial URL publication with controlled resolver
      completion; batch results while retaining stale-render/disconnection
      guards, successful siblings and failure diagnostics.
- [x] Verify the owned action attribute gallery and select companion from
      source and isolated packages, including concurrent gallery startup.

The action gallery covers all 20 explicitly implemented attributes. The select
companion and listbox scrolling fix are complete. Verification and remaining
scope are recorded in the [completion log](archive/todo-completed-2026-09-27.md#component-galleries-and-module-url-settlement)
and [startup review](browser-stabilization-review.tmp.md#gallery-module-url-settlement-recurrence).
The release XHTML bundle is also complete; remaining migration work is below.

## Completed: Selected actions and state-specific input indicators

- [x] Replace blanket input shadows with state-specific native-control feedback; revise shared/theme contracts.
- [x] Restore controlled selected action state, zebra and accessible selectable-button semantics.
- [x] Replace Expanded in the playground with Selected and add a container-owned choice example.
- [x] Fixture: resting inputs/labels, focus, checked/mixed, invalid, pending and forced-colors feedback.
    - [x] Verify native control CSS states directly across theme modes so legacy
          boolean-binding failures cannot hide feedback regressions; preserve
          the separate component behavior assertions and baseline repair task.
- [x] Fixture: selected action presence/removal, no automatic toggling, accessibility and combined states across modes.
- [x] Verify source/package playgrounds, theme and component gates; record completion and commit/push.

Browser and package verification now runs in the unrestricted session. Native
CSS checks cover all ten controls across five theme modes and forced colors;
source and isolated-package playgrounds pass. The follow-up fixes missing
pending feedback on date/time inputs and scopes the icon-spacing fixture to its
intended sample. No theme token definitions were removed.

The legacy state suite remains 3 passing / 18 failing, with exactly the same
failed test names as the documented baseline. Its boolean-binding failures
still block later component assertions; direct CSS coverage does not close that
separate repair task. See [completion evidence](archive/todo-completed-2026-09-27.md#selected-actions-and-state-specific-input-indicators).

## Completed: Action size roles and content compositions

- [x] Document size roles, independent content layout, native navigation semantics and compact-only hit expansion in the dimensions specification.
- [x] Add the theme-owned xx-large minimum profile and allow action labels to wrap within constrained layouts.
- [x] Fixture: cover xx-large inheritance/overrides, compact-only hit borders and growing image/title content in action stories.
- [x] Add working inline, text, icon, image-choice and hero-choice playground samples; verify responsive source and installed-package pages.
- [x] Run theme and component checks and record evidence.

Verification: 15 action stories, theme generation and fresh Phase 13 checks,
component package/style/declarative checks, typecheck, lint and responsive
source/isolated-package playgrounds pass. See [completion evidence](archive/todo-completed-2026-09-27.md#action-size-roles-and-content-compositions).

## Completed: Playground undefined size and contrast contract

- [x] Fixture: Size radios expose Undefined, small, medium, large and x-large; Undefined removes the attribute in source and packaged playgrounds.
- [x] Resolve pending-gradient versus contrast-outline policy: user selected animated loading zebra with collapsed fills.
- [x] Generate contrast action fills/ink and implement zebra state feedback from the canonical theme.
- [x] Fixture: verify contrast-light/dark default, hover, active, focus, disabled and loading, including theme switching and accessible state feedback.

Verification: 14 action stories, theme generator/manifest checks, component
package/style/declarative checks, typecheck, lint and source/isolated-package
playgrounds pass. See [completion evidence](archive/todo-completed-2026-09-27.md#undefined-size-and-contrast-action-contours).

## Completed: Action dimensions and compact hit areas

- [x] Document optional theme size profiles and conditional native hit areas in the dimensions, controls and coupling specifications.
- [x] Generate reusable size profiles and preserve their public token exports.
- [x] Apply inherited/explicit sizes to action and reserve compact hit areas in layout.
- [x] Fixture: verify size inheritance/removal, both-axis targets, compact edge activation, disabled/form behavior, spacing, stretching, bends and pending/focus paint.
- [x] Add size radios and layout examples to the property playground; verify source and installed-package pages.
- [x] Finish theme generator verification, record evidence, commit and push.

Verification: 13 action browser stories, source/isolated-package playgrounds,
component package/style checks and typecheck pass. Lint passes with existing
non-null assertion warnings. Theme generation, token exports, manifest validation
and the Phase 13 browser verifier pass. See [completion evidence](archive/todo-completed-2026-09-27.md#action-dimensions-and-compact-hit-areas).

## Completed: Theme switch component and playground integration

- [x] Add a reusable `cem-theme-switch` component with visible Light / Dark /
      Native choices and a separate Contrast toggle. Author a canonical XHTML
      definition with colocated CSF stories, following the shared component
      authoring and public CSS ownership rules.
- [x] Native disables the Contrast toggle; the operating system owns native
      contrast. Light and Dark map to the existing contrast variants.
- [x] Add the shared theme switch to component playground pages, starting with
      the action playground, and make it part of the pattern for future
      playgrounds. Apply the selected theme to the page, including its live
      preview, while preserving the component's current property selections.
- [x] Fixture: verify mouse and keyboard operation, accessible labels and state,
      all supported mode/contrast combinations, retained preview state, and
      source/packaged playground loading through page-level module maps.

### Theme switch integration finding

- [x] Adopt generic nested custom-element ownership: preserve self-rendered
      output while updating authored attributes and child inputs. Components
      that consume those inputs expose subsequent updates through their API.
- [x] Fixture: preserve custom-element output and self-added attributes during
      full renders and worker patches; verify authored input updates/removals
      and nested slice/resource binding ownership.
- [x] Verify source/installed-package playgrounds after the runtime fix, then
      commit and push the theme-switch integration.

Verification: 332 browser stories and 585 runtime unit tests pass; component
package, style, source/installed playground checks, typecheck and lint pass.
See [completion evidence](archive/todo-completed-2026-09-27.md#theme-switch-and-nested-custom-element-ownership).

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
    - [x] Fixture: a pointer click on button content followed by a Space click
          keeps the slice value but refreshes native target metadata; confirm
          that holding Space does not publish another click.
    - [x] Fixture: isolate the icon-button/menu-item disabled bindings in the
          unchanged state suite, compare failed test names, and characterize
          repeated click payloads before changing the active-state contract.
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
- [x] Implement the accepted [component development pattern](component-development-pattern.md)
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
    - [x] Fixture: route the action playground gallery link to the owned
          cem-components gallery in source and isolated-package previews.
    - [x] Fixture: cover every explicitly implemented action attribute in the
          owned gallery, including size, selected/selectable and native form
          overrides; verify inventory and behavior in source/package previews.
    - [x] Fixture: apply the companion property playground/source-view pattern
          to `cem-select`, including source and isolated package preview checks.
        - [x] Fixture: keep the page scroll position when persistent select
              examples mount; reveal active options within their own listbox
              during initial rendering and keyboard navigation.
    - [x] Fixture: characterize individual/bundled resource bases; record the
          [source-context decision](component-bundle-source-context.md) above.
    - [x] Fixture: generate the release XHTML bundle through the CEM AST pipeline
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
