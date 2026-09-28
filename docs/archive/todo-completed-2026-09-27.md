# Completed work — 2026-09-27

This records follow-up work after the [TODO snapshot](todo-snapshot-2026-09-27.md).
The [active TODO](../todo.md) remains the authority for unfinished work.

- [x] Fixture: activate staged registry connections inside queued CSS/DOM
  publication; verify cleanup ordering, empty source groups, rejection and
  activation failure recovery.
  `QueuedConnectionActivation` exercises worker and fallback hosts. Publication
  adopts the connection after the DOM patch and before deferred old-load cleanup.
  Rejected candidates release preparations, reused active handles remain live,
  and activation failure or cleanup cancellation blocks the queue until recovery.
- [x] Update legacy viewer evidence checks to read completed fixture IDs from the
  archived checklist while continuing to check open IDs in the active TODO.

Validation: 585 unit tests and 304 browser cases in each of the default and
retained-CSS lanes passed. Typecheck and lint passed, with the two existing
non-null assertion warnings. All 31 existing open TODO items were preserved;
the active checklist contains no completed entries.

## Runtime payload stylesheet replacements

- [x] Fixture: retain active runtime instance CSS during replacement imports,
  failed loads and superseded updates; verify successful replacement, clearing
  and disconnect cleanup in both host modes.
  `PayloadReplacementPreservesActiveStyles` reproduced the removed live style
  node while an import was pending. Runtime replacements now use the staged
  installer, which retains active styles until a complete replacement succeeds.
  Initial installation retains its independent-source failure behavior.
  The fixture covers worker and fallback execution, node reuse, failed imports,
  superseded late results, empty payload CSS and disconnect cancellation.

Runtime CSS/DOM joint publication remains an open integration task.

Validation: 585 unit tests and 305 browser cases in each CSS lane passed,
along with typecheck and lint (two existing non-null assertion warnings).

## Native CSS migration closure

- [x] Close native CSS migration with these three delivery gates. Existing
      parsing, URL resolution, staging and queue helpers are prerequisites already
      implemented; do not expand them into separate completion milestones.
    - [x] Runtime fixture: publish retained declaration CSS, instance CSS and DOM
          through the per-host queue, adopt runtime state before cleanup, reject
          superseded loads and recover invalid DOM with a full render. Keep
          resource settlement outside the queue. Cover worker and fallback modes.
    - [x] Edge fixture: transport and retain changed stylesheet batches, validate
          them with the patch transaction, publish through the same ownership
          boundary and recover authoritatively. Verify cancellation, stale CAS,
          shared contexts and browser cleanup before removing the unchanged-CSS
          capability guard.
    - [x] Default cutover fixture: make retained CSS the browser default after
          SSR/hydration and component verification; retain an explicit legacy
          comparison lane and document the separately owned result-style path.
          Audit source profiles and diagnostic gates, document explicit
          compatibility boundaries, and close the native CSS migration.

Runtime DOM and processing-host updates now prepare declaration CSS, payload CSS
and DOM patches inside the per-host queue. They adopt connection, style and
render-plan state before cleanup, cancel superseded work, preserve live output
on CSS failure and retry invalid patches with a full render. Resource settlement
runs outside the queue. Hydration keeps server style nodes while imports load.
CSS source compilation uses its own artifact, so declaration startup cannot
prime the render cache with incomplete instance bindings.

Native Edge updates load a complete changed stylesheet batch before committing
retained state or exposing frames. Protocol v14 carries the content-addressed
batch; `publishEdgeCssDomUpdate` verifies it and publishes emitted CSS with the
DOM transaction without parsing CSS again. Fixtures cover changed and unchanged
inputs, held-import cancellation, competing CAS writes, failed imports, tampered
content, stale browser state, shared/private ownership, callback-induced recovery,
node reuse and teardown.

Native retained CSS is now the browser default. `retainedStylesheets: false` and
`cem-elements:test:legacy-css` provide explicit compatibility comparison. XSLT
result styles remain branch-local under their accepted contract. Unsupported CSS
syntax keeps the documented native diagnostics; automatic fragment substitution
remains in the wishlist. These are explicit boundaries, not further migration
prerequisites.

Validation:

- 585 runtime unit tests and 53 Edge unit tests passed.
- 306 browser cases passed in each CSS lane; all 10 SSR browser cases passed.
- Native retained-stylesheet/WASM verification, typecheck, lint and package
  verification passed. Lint retains two existing non-null assertion warnings.
- Rebuilt `cem-elements` before running component verification because the legacy
  suite consumes packaged output. Committed baseline `5f8e7172` and this cutover
  both produce 81 passing / 48 failing tests, with identical failing test names.
  No new component failures were introduced. The legacy component package gate
  remains red and is tracked under the remaining declarative UI migration.

The active TODO retains its 25 unrelated open items and no completed checkboxes.

## Site v3 XHTML deployment

    - [x] Implement v3 XHTML deployment before resuming the action cutover.
        - Native support complete: the v3 schema gate, 27 focused module-map
          tests, v1/v2/v3 CLI publication tests, cache-key fixture, TypeScript
          projections and WASM build pass. Generated examples document XHTML.
        - [x] Adopt typed v3 maps and exact module edges in Site, then prove
              static search/interactive XHTML delivery before registry removal.
            - [x] Fixture: verify Site catalog/search layouts query retained
                  generic-data trees, including rendered values and row counts;
                  retain one stable ID per heading without copying a second ID.
            - [x] Fixture: deploy the existing canonical select XHTML on both
                  routes; verify byte/digest identity, named-template loading,
                  scoped styles, interaction and complete declared module assets.

    - [x] Fixture: distinguish bounded document imports from small expression
          imports; accept a catalog-sized native tree with source provenance,
          reject document value/depth/byte overflow, and preserve string/CSS limits.
    - [x] Diagnose the restored Site build's token-browser failure:
          `cem.transform_template.adapter_failed` reports the 64-level / 4096-value
          JSON import limit before asset deployment. Resolve before end-to-end
          XHTML cutover verification; do not attribute it to the new resource.

Validation: 37 focused native import tests passed. The full `cem-site:verify`
gate passes, including deterministic builds, search, interactive behavior and
all 26 production routes. Both browser routes load the canonical select XHTML,
choose its second option and verify native scoped CSS. Static checks preserve
all catalog rows, component provenance and source/output asset digests.
The 65,536-value document profile fixes the token-catalog blocker while retaining
16 MiB and 64-level limits and the smaller string/CSS profiles. Site layouts now
read the retained generic-data tree. Completed items are removed from active TODO.
Site lint passes with one existing unused-variable warning in `search-runtime.js`.

## Action demo local preview

- [x] Load the legacy Material action page with page-level import maps for
  `cem-elements`, CEM-ML WASM and the repository-owned `cem-demo-element` helper.
  Use `<cem-element>` and explicit `custom-element-v0` template language markers,
  including the externally loaded icon template. Restore fallback label
  evaluation and place the Bend example inside its demo template.
- [x] Replace missing legacy stylesheet/header dependencies with the page's
  own layout and a demo navigation link. Keep external icon fonts as presentation
  assets; runtime JavaScript and WASM resolve locally.
- [x] Give action its own source/dist map pair in the native transform graph.
  Retain strict source-map validation for each Material page.
- [x] Verify all seven cards and 29 buttons, evaluated fallback/icon content,
  and the Bend interaction from source, dist and a clean installed archive.
  DevTools confirms the same results at the user's IDE URL:
  `http://localhost:63342/cem/packages/custom-element/material/components/action.html`.
  Runtime/demo modules and both WASM engines return HTTP 200; the console is clean.
- [x] Audit the packed inventory against the prior 189-file fingerprint.
  Its existing paths reproduce the prior digest exactly after removing 30 newly
  compiled native-CSS files and the two new action maps. Exclude the three
  browser-story-adapter outputs, then lock 218 files: the existing inventory,
  27 native-CSS runtime/declaration outputs and the two action maps.
- [x] Make the public adapter smoke test's scope syntax checks independent of
  emitter whitespace, preserving the exact declaration selector, implicit scope,
  computed colors and outside-instance isolation checks.

Validation: `yarn nx run @epa-wg/custom-element:lint` passes with its build,
reference-corpus, packed-archive, theme-runtime and browser-fixture dependencies.
The packed archive includes clean JavaScript/type consumer checks and Chromium
rendering. This closes local preview loading for the legacy Material reference;
the separate canonical `cem-components` action cutover remains active.

The component baseline investigation also adds a passing native
`host_boolean_attribute_presence_is_distinct_from_value_truthiness` fixture.
Its four cases distinguish attribute presence from empty/nonempty string values.
The unchanged legacy component suite remains 81 passing / 48 failing; the
[failure inventory](../cem-components-baseline-2026-09-27.md) records the findings.

## Action theme curvature

- [x] Load the generated CEM theme stylesheet through the action page's module
  map and declarative `cem-module-url` binding. Vendor the same generated CSS
  in the package; the archive now locks 219 files.
- [x] Replace unsupported legacy nested action selectors with declaration-owned
  native scoped rules. Bind the radius to `--cem-action-border-radius`, select
  local Bend tokens, and consume CEM control sizing. Prevent unintended label
  wrapping while retaining explicit multiline content.
- [x] Give the multiline round example an inert instance template with local
  shape-height and round-bend overrides. Verify radius equals half the height.
- [x] Verify computed default/sharp/round curvature, live Bend changes, and
  compact/forgiving sizing in source, dist and clean installed-package checks.

DevTools at the IDE preview confirms balanced round geometry of 20px/40px
(radius/height), compact 18px/36px, and forgiving 22px/44px. The multiline
example doubles both dimensions in each mode. Sharp is 0px and smooth is 8px
with the default theme. The preview has no console warnings or errors.
Validation: `yarn nx run @epa-wg/custom-element:lint`, including package,
reference-corpus and browser-fixture dependencies.

## Shared harness required-input repair

- [x] Reproduce the shared substrate harness's native validation failure in
  isolation: clearing the input incorrectly leaves `form.checkValidity()` true.
- [x] Correct the test-owned field declaration to bind `required` by presence,
  using the native semantics already pinned in the Rust fixture. Preserve the
  existing invalid-form assertion and reset, event, accessibility and visual checks.
- [x] Verify absent, empty, `"false"` and `"true"` required attributes, including
  live removal/reintroduction, native `validity.valueMissing`, and input identity.
- [x] Pass `yarn nx run @epa-wg/cem-components:verify-phase3-harness` and its
  lint/typecheck/build dependencies. Rerun the full component suite and compare
  full failure names against the recorded baseline: 82 passing / 47 failing,
  with exactly the harness failure resolved and no new failing test names.

The production component/runtime implementation is unchanged. Remaining
failures stay in the [baseline inventory](../cem-components-baseline-2026-09-27.md).
Next: the accepted canonical `cem-action` XHTML/colocated-story migration and
consumer cutover, preserving explicit submit/reset and default command behavior.

## Action preview states, variations and zebra

- [x] Bind native enabled hover/active and disabled paint to the corresponding
  theme action state pairs. Add an interaction card and verify held pointer,
  held/released Space, focus, unchanged geometry and disabled click suppression.
- [x] Forward disabled by native attribute presence, including empty and
  `"false"` values. A failing Rust fixture exposed legacy conversion folding
  `count(//attributes/@disabled)` to zero against the static template. Keep
  unmatched runtime data paths unresolved until rendering. Static document
  count/sum fixtures still pass; no component JavaScript workaround was added.
- [x] Compose zebra shadows on the focused button. The inherited root recipe
  had resolved all stripes to the inactive surface color before focus changed
  the local focus token. Compare all three actual stripes against local tokens
  across light, dark, contrast-light, contrast-dark and native themes, and retain
  a system-color outline in forced-colors mode.
- [x] Give every sample one consistent action variant and add a comparison
  matrix: primary, explicit, contextual, alternate and destructive (danger),
  each with sharp, smooth, round and disabled controls. Remove obsolete color
  names and the nonfunctional custom-color example. Verify each intent's
  default/hover/active/disabled colors against generated theme tokens.

Validation: `cargo test -p cem-ml legacy_custom_element::tests` passes all 74
tests. `yarn nx run @epa-wg/custom-element:lint` passes with all 12 dependencies,
including source/dist previews, clean installed-package Chromium checks and
219-file archive verification. The IDE preview has nine cards and 52 buttons;
DevTools confirms real Tab/Shift+Tab focus, distinct current zebra colors and
native disabled states after rebuilding and reloading WASM.

The canonical `cem-components` action cutover remains next. Its migration plan
now records the user's requested five theme intents as the accepted target.

## Canonical action cutover

`cem-action` now belongs to `cem-components/src/components/cem-action/` as a
canonical XHTML declaration with six colocated CSF Next stories. The legacy
registry entry and global styles are removed. Five intents, native hover/active,
keyboard zebra focus, bend geometry and explicit submit/reset forwarding live
in the declaration. Mixed legacy fixtures retain their unmigrated controls;
action unit assertions moved into the stories.

The Material gallery loads the canonical resource using its page-level module
map. Its compatibility URL stays available. The adapter archive contains 220
locked files, including the same XHTML asset. Workflow fixtures and Site's
search/interactive pages load action explicitly; Site deploys it through v3
resource maps alongside select.

Verification:

- Six action stories and the ordinary static Storybook build pass.
- Declarative inventory: 2 canonical components, 47 legacy components, 61 legacy
  authored JS/TS files. Declarative, style, state-matrix and primitive gates pass.
- Component build, typecheck, lint and packed-XHTML checks pass. Lint reports
  non-null assertion warnings in the new test file, with no errors.
- Adapter test and packed-consumer checks pass, including source/dist/installed
  gallery states, curvature, variations and keyboard zebra/forced colors.
- Site verification passes, including deterministic builds, deployed XHTML,
  interactive/search behavior and production routes.
- DevTools verified the IDE gallery loads the canonical XHTML, all nine cards
  render their 52 buttons, and the console contains no warnings or errors.

The accepted [component development pattern](../component-development-pattern.md)
keeps production XHTML separate from a companion property playground and source
view. The generated combined `#ID` release document is planned. These two new
artifacts remain explicit TODO work; this cutover does not claim they exist.

Final legacy comparison: **82 passing / 47 failing / 129 total**, with exactly
the same failed test names as the shared-harness repair baseline and no skipped
tests. The aggregate component gate remains red for those existing failures.

The broader runtime run reports **311 passing / 1 failing / 312 total**:
`Payload Css Readiness And Hydration` observed zero styles where one was expected.
Its complete 12-story file passes in isolation. An earlier concurrent run had
four other failures (HTTP/local-storage samples and stylesheet staging); all
seven stories in those three files passed on focused rerun, and the later full
run passed them. No runtime source changed in this cutover. The runtime aggregate
is not claimed green; the remaining instability is recorded in TODO.

## Action property playground and source view

The companion page is `packages/cem-components/playgrounds/cem-action.html`.
It uses the canonical action, canonical select and the existing shared field.
Seven public options update one stable native preview button: intent, bend,
label, type, disabled, loading and expanded. Hover, held pointer and keyboard
focus remain native interactions. The source-only viewer is a static sibling
of the reactive form and reads the canonical XHTML without executing a second
copy. The full gallery link is available at the top of the page.

Native import-map rewriting emits `dist/cem-action.html` and
`dist/cem-action-gallery.html`. The latter uses the existing Material source
and a byte-preserved icon resource; both pages load the canonical action.
Source, maps and build configuration ship with the package. The demo helper is
a direct dependency; the legacy adapter is not a package dependency. Public
exports expose both interactive pages. The legacy notice links the live page
in repository previews and its source in the standalone adapter archive.

The playground exposed a shared event binding gap: native controls supplied
`$target.value`, but form-associated custom controls did not. The runtime now
reads their live string value in direct bindings, aliases, default slice values
and serialized event targets. Two new DOM/CEM-ML stories failed before the fix
and pass after it, alongside five existing input-value stories. Non-form and
object-valued targets retain the explicit boundary.

Verification: runtime typecheck/lint pass; the component architecture and package
gates pass (79 packed files). `verify-playgrounds` exercises the repository and
five isolated npm archives, all properties, node identity, native states, source
bytes, a single action instance, the 52-button gallery and reciprocal links.
The adapter test and installed-consumer gates pass (220 locked files). DevTools
verified the IDE page renders its controls, preview and ready source viewer.

The combined `#ID` release document and select companion remain open TODO work.

The final full `cem-elements:test` run passes all 314 stories across 67 files,
including the six action stories and the new custom-control event cases. This
run also passes the previously intermittent stylesheet hydration case; its
separate investigation remains open because one passing run does not explain
the earlier failure.

## Select mouse selection in the action playground

A real mouse press blurred the combobox while the pointer was still held over
an option. Its focusout handler closed the popup before click could commit the
selection. Earlier immediate-click checks missed the interval between press
and release. The shared choice-select capability now prevents the option's
primary mousedown from moving focus, preserving the aria-activedescendant
combobox owner. Outside pointerdown still closes the popup, but its closing
render no longer takes focus back from the clicked control.

The colocated `NativeMouseSelection` story failed before the fix. It now holds
a trusted pointer for 250 ms over a rich option, asserts focus/open state and
no early event, verifies one input/change pair on release, and verifies outside
click focus. All eight select stories pass. Playground integration checks now
use a 180 ms native click delay for every option, in both source and isolated
package previews. No page-specific event handler was added.

Final source/isolated-archive playground checks and runtime typecheck/lint pass.
The IDE page also confirms the updated handler protects focus and selecting
Destructive updates the action variant and red theme paint.

## Visible radio options in the action playground

The six enumerated property controls now use labeled `cem-radio` groups:
intent, bend, button type, disabled, loading and expanded. All 18 options are
visible. The label remains a text field. Native radio names enforce one choice
per group, and declarative change bindings update the same action instance.
The playground no longer loads the select declaration.

Source and isolated-package browser checks pass for initial checked values,
all property changes, native arrow-key selection, exclusivity, persistence
after other edits, preview identity and the existing source/gallery checks.
The IDE browser confirms all six groups and their initial choices render.
The development pattern now calls for visible radio groups for mutually
exclusive enumerated properties.

## Action native border reset

The canonical action now sets `appearance: none` and `border: 0` for its
native button. This removes the browser's 2px outset border across all five
intents while preserving theme curvature, paint and zebra focus rings.

All six action stories and source/isolated-package playground checks pass.
DevTools confirms zero border width and no native appearance for each intent.
A separate browser check under forced colors confirms keyboard focus retains
the solid 3px system-color outline with no box shadow.

## Action loading motion and replay

`loading="true"` now paints each action intent's pending background/text pair
with a one-shot fade using D7 action duration and smooth easing. Pointer entry
and keyboard focus replay the fade while busy. Separate named CSS animations
preserve completed animations when an interaction ends, preventing exit from
restarting the fade. Static animation names and separate timing longhands use
the existing scoped CSS compiler's supported token syntax.

Disabled paint wins over pending; clearing loading immediately restores the
current interaction paint. Reduced motion suppresses animation while retaining
pending colors. Loading continues to reflect aria-busy without disabling the
button or changing its dimensions. The playground explains this below its preview.

All seven action stories pass, including all five intents, pointer re-entry,
keyboard focus, zebra, stable dimensions and disabled precedence. Source and
isolated-package playground checks verify animation start, loading toggles and
reduced motion. Runtime typecheck/lint pass. The IDE preview shows the description
and resolves the scoped animation name with the theme's 250ms duration.

## Canonical pending gradient restoration

This supersedes the one-shot action loading fade above. The historical
`custom-element-dist` Base Principles describes a 45-degree moving gradient;
its older consumer stylesheet implemented a separate 90-degree variant.
The canonical D0 contract now specifies the 45-degree pattern, binds its colors
to each intent's pending/active backgrounds, and retains pending text. D5 owns
the angle and square tile size; D7 owns a two-second pending cycle. Manifest
derivation and token exports include the five added tokens.

Generated theme CSS provides `.cem-pending`, and the color generator animates
all 25 intent/mode pending swatches. The canonical action owns the same recipe
in scoped CSS while enabled and loading. Hover and focus preserve its loop;
disabled and clearing loading remove it. Reduced motion freezes the gradient.
Forced colors suppress gradients/motion and use an inset system-color outline;
keyboard focus keeps its separate outer outline. The playground description
and component reference now describe the continuous pattern.

Theme build/export and all manifests pass (484/484 token coverage). All seven
action stories pass, covering intent paint, loop position across cycles,
interaction continuity, geometry and disabled precedence. Source and isolated
package browser checks pass for both action and theme generator pages, including
25 pending swatches, 50 gradient-endpoint text contrast checks at 4.5:1 or better,
real background movement, reduced motion and forced colors. The IDE preview
confirms a 45-degree gradient, two-second infinite duration and changing position.

## Disabled actions retain pending feedback

Pending action visuals now take precedence over disabled paint. A button with
both `loading="true"` and `disabled` retains the moving gradient, while native
disabled behavior still blocks activation and focus. Clearing loading leaves
the button disabled and restores disabled colors. The theme contract, component
reference and playground explanation now describe this combined state.

All eight action stories pass, including all five intents, unchanged animation
identity when disabling, blocked clicks and a submitted-form fixture that sets
loading and disabled together and prevents repeat submission. Source and isolated
package playground checks cover the combination, real movement, loading completion,
reduced motion and forced colors. The IDE preview confirms both attributes are
active and the background position continues advancing.

## Legacy action gallery attribute cleanup

Removed the nine-button alignment sample and its unused layout rule. The current
action has no align API; the historical declaration also did not consume its
declared align value. Removed redundant numeric label attributes from icon
examples and unsupported dot attributes on actions. The typical-use sample
retains label as an explicit fallback-text example. Icon-only actions now have
aria-label names, and alternate actions demonstrate ordinary child text with icons.

The gallery verification inventory is now eight samples and 43 buttons.
Source and isolated-package playground/gallery checks pass.

## Action variations matrix contains intent and shape

Removed the disabled column from the action variations matrix. It now compares
five intents across three shapes (15 actions). Disabled examples remain in the
Interaction states sample. The full gallery contains eight samples and 38 buttons.
Source and isolated-package playground/gallery verification passes.

## Legacy custom-element license alignment

At the user's direction, the current custom-element package now uses the
repository MIT license text and package metadata. All eight Material demo
footers identify MIT. Active package guidance is updated; historical snapshot
records retain the original license with an explicit current-license note.
Third-party vendor licenses are unchanged. Verified exact root/package license
text equality, MIT package metadata, all eight footer references and clean diff
formatting.

## Native host visibility convention

Canonical action and select now respect ordinary native hidden states despite
their inline-block host layout. The selector uses `:where` around the until-found
exclusion to stay within the scoped CSS specificity ceiling. Until-found values
remain browser-owned, including case-insensitive spelling; no internal control
attribute forwarding, invisible alias, animation or reveal polyfill was added.

Shared conventions explain hidden values, removing the attribute to reveal,
state/form preservation and the distinctions from visibility and aria-hidden.
The legacy invisible gallery sample is removed and the action reference links
to the shared guidance. The gallery now has seven samples and 36 buttons.

All 18 action/select stories pass, including initial/live hiding, zero layout
rectangles, native keyboard exclusion and re-entry, retained control identity,
action busy/label state, select value and both until-found spellings. Source and
isolated-package gallery checks pass, as do runtime typecheck/lint. IDE browser
inspection confirms hidden and hidden=false produce display:none, while mixed-case
until-found retains inline-block with native content-visibility:hidden.

## Action CSS redundancy cleanup

Removed the action's forced-colors background-image:none and box-shadow:none
declarations. Native forced-color adjustment already suppresses non-URL images
and shadows; explicit animation suppression and accessible outlines remain.
Consolidated the three identical bend-radius bindings into one :is selector,
preserving their specificity. The remaining private color mappings and native
attribute bindings are used. Nine action stories and source/isolated-package
playground checks pass, including reduced-motion and forced-colors coverage.

## Public CSS property ownership and theme adoption

- Recorded `--cem-<component>-*` as public API in component conventions, the
  theme index and CLAUDE.md. Conflicting definitions require an explicit
  theme/component ownership decision; an `internal` name is not an exemption.
- Adopted existing action intent/state theme endpoints directly, removing ten
  private aliases and component-local pending color bindings. All five intents
  retain hover, active, disabled, pending gradients and accessibility fallbacks.
- Promoted `--cem-select-popup-z-index: 1` into the D4 Markdown source table,
  generator, manifest and numeric DTCG export. Canonical select consumes it
  without a local default. The old private exception remains bounded to frozen
  legacy choice implementations until their migration.
- Removed redundant select focus/selection color aliases. Retained the five
  state/geometry calculations and all existing generated theme variables.
  Optional unused-code optimization is deferred to consumer builds in roadmap.md;
  CEM theme/component releases must not prune public tokens based on local usage.
- Added colocated browser checks for action container/instance overrides and
  restoration, select combined indicator states, and select popup stacking
  inheritance, override isolation and native pointer selection.
- Verification: 21 action/select browser stories pass; theme token coverage is
  485/485; the new DTCG property is a numeric `1`; theme Phase 13 verification,
  component style contract, package checks and action source/isolated-package
  playground checks pass. Pending gradient motion, reduced motion and forced
  colors remain covered by the playground verifier.
- Updated stale verifier expectations for the already-shipped action pending
  keyframes and D5 pending-pattern presentation table. An early source-copy
  mismatch and a browser startup during CSS regeneration were resolved by
  rerunning checks after source/build outputs settled.

## Lighter destructive pending stripes

- Added the canonical D0 `--cem-action-destructive-pending-stripe-background`
  token, mixing the active endpoint with 25% white in light schemes. Dark
  schemes retain the active endpoint for text contrast. Existing theme tokens
  and active action paint are preserved.
- Action and the theme generator pending preview consume the same stripe token.
  The light-theme stripe changes from approximately #7d070a to #9e4547 while
  the darker endpoint stays #6d0106; pending text contrast remains about 5.1:1.
- Verified 10 action stories, 486/486 token coverage, theme Phase 13, component
  style contract and source/isolated-package playgrounds including endpoint
  contrast, animation, reduced motion and forced colors. DevTools confirmed
  animation movement in the IDE preview, left on Destructive + Loading=True.

## Destructive pending dark-mode stripe separation

- Updated the canonical destructive pending-stripe token's dark branch to mix
  active background with 50% black. Dark and contrast-dark stripes now pair
  approximately #b61919 with #530a0a (about 2.22:1 endpoint separation), replacing
  the barely separated #b61919/#a61415 pair. The light branch is preserved.
- Added a five-mode disabled/loading story covering visible stripe separation,
  at least 4.5:1 text contrast at both endpoints and continuous animation.
  Branded story samples use a branded base so they do not inherit Storybook's
  native palette override.
- Verification: all 11 action stories, theme Phase 13 and source/isolated-package
  playground checks pass. DevTools confirmed the dark scheme and new gradient
  in the IDE action playground, left on contrast-dark + destructive + loading.


## Theme switch and nested custom-element ownership

- Added canonical `cem-theme-switch` XHTML and colocated stories. Visible
  Light/Dark/Native radios and a Contrast checkbox select the five existing
  themes. Native follows OS colors, disables Contrast, and remembers the
  contrast preference for returning to Light/Dark. Independent scopes use
  distinct radio-group names.
- Integrated the shared scope into the action playground and added its own
  companion playground/source view, page-level module maps, package export,
  native import-map build output and catalog documentation. Preview values,
  component identities and source displays survive theme changes. Canonical
  component inventory is now 3 declarative / 47 legacy.
- Fixed shared DOM ownership: full renders and worker patches preserve output
  and attributes created by registered custom elements, while updating/removing
  authored attributes and reconciling authored child inputs. Parent metadata
  updates stop at custom-element boundaries. Native and unregistered elements
  retain normal rendering behavior.
- Fixed parent slice/resource binding scans taking over nested component
  controls. Parent-owned host bindings remain active; nested output bindings
  retain their component owner. The normative lifecycle document records the
  ownership rule and the boundary for components that consume their inputs.
- Added six runtime browser regressions for full renders, worker patches,
  authored child updates/reordering/removal, dynamic text ranges and nested
  bindings. Source/package playground checks exercise all modes, native OS
  color changes, retained properties, source display, pending motion, reduced
  motion and forced colors.
- Verification: all 332 browser stories and 585 unit tests pass. Typecheck,
  lint (two existing warnings), declarative architecture, catalog, component
  style contract, package verification and source/isolated-package playground
  checks pass. DevTools confirms the IDE preview; it is left on contrast-dark
  with a destructive, disabled, loading action and a ready source viewer.
- The clean full browser run does not attribute or close the older intermittent
  startup investigations. The next component development step is the companion
  property playground/source view for `cem-select`, using this shared switch.


## Action dimensions and compact hit areas

- Recorded the shared dimension decision in `cem-dimension.md`, with controls,
  coupling and shape specifications carrying their respective contracts.
- Added Markdown-owned control height profiles: small 2.5rem, medium 3rem,
  large 4rem, x-large 6rem. Generated CSS exposes inherited `data-cem-size`
  profiles; public token exports retain all four new properties.
- Action's optional `size` selects the theme profile on its host. Omission or
  removal restores inherited geometry; content and flex/grid layout can grow
  the control. Typography, icon sizes and padding retain existing theme values.
- Native buttons reserve at least the coupling minimum in both axes. Compact
  requested geometry uses a transparent border with padding-box background
  clipping; profiles meeting the minimum have zero hit border. No extra
  interactive element or JavaScript behavior was introduced. Layout owners
  continue to provide the coupling guard between targets.
- Round geometry re-resolves at local size boundaries. Public host radius
  overrides remain effective, and zebra encloses the complete operable target.
- Added size radios and content/inherited/stretched examples to the playground.
  Source and isolated-package verification exercise every size and removal.
- Verification: 13 action browser stories pass, including size inheritance,
  instance overrides/removal, custom profile/radius values, compact/forgiving
  coupling, icon targets, stretch, native edge clicks and form submission,
  disabled behavior, pending gradients and keyboard zebra. Theme generation,
  token exports, all manifest checks and the Phase 13 generator/browser verifier
  pass. Component declarative/style/package checks, typecheck and source/package
  playgrounds pass. Lint passes with 48 existing non-null assertion warnings;
  fixed the existing unused style-read expression and documented the HTML-only
  playground dependency for Nx dependency checking.
- Next component development item: the `cem-select` companion property
  playground/source view with the shared theme switch and package preview checks.


## Undefined size and contrast action contours

- The playground's size radios now expose Undefined, Small, Medium, Large and
  X-large. Undefined removes `size` and restores inherited theme geometry.
- Fixed contrast-light/dark action tokens reusing ordinary branded fills.
  Canonical D0 state-table columns now collapse all action fills to the surface,
  use surface ink and subdue disabled ink. Existing tokens remain in the release.
- User decision: contrast loading also keeps a collapsed fill and uses an
  animated zebra contour. Ordinary light/dark/native pending gradients remain.
- Added Markdown-owned intent contour colors, pending contour image and D5
  contour visibility. Action uses existing stroke widths for resting, hover,
  active and pending feedback. The masked pseudo-element has no pointer behavior
  or layout footprint; outer keyboard zebra and native disabled behavior remain.
- Disabled loading retains the contour loop. Reduced motion freezes it; forced
  colors suppresses it and preserves the existing static system-color boundary.
  Theme preview utilities and displayed contrast formulas follow the same policy.
- Preserved visible slider hover/active thumb markers: these foreground colors
  no longer inherit collapsed action fills in contrast scopes.
- Registered selector-only size profiles and contour visibility in the theme
  presentation verifier's generator-only table inventory. D0/D5 manifest and
  export derivation include the new canonical source tables.
- Verification: all 14 action stories pass, covering both contrast modes and
  all five intents, text/contour contrast, pointer/keyboard states, disabled
  loading, theme restoration, unchanged geometry and the two-second repeating
  contour loop. Theme generation/exports and Phase 13 manifest, presentation,
  contrast, focus/target-size, reduced-motion and forced-colors checks pass.
  Component declarative/style/package checks, typecheck and lint pass (48
  existing non-null assertion warnings). Source and isolated-package playground
  checks cover Undefined/removal and both contrast modes, including reduced
  motion and forced colors.


## Action size roles and content compositions

- Recorded the size-role decision in D1 dimensions: optional inherited sizing,
  independent glyph/content layout, small inline controls, ordinary medium
  controls, prominent large controls, image/title tiles and hero compositions.
- Added the Markdown-owned `--cem-control-height-xx-large: 8rem` profile, generated
  inherited selectors and exported token. Action exposes the same `xx-large`
  value; all heights remain minimums, and labels/content can wrap and grow.
- Preserved conditional native hit borders only for requested geometry below
  the 3rem coupling target. Medium and larger defaults add no extra hit area;
  compact defaults and consumer overrides still receive the safety inset.
- Added working inline, text, medium/large icon, image-choice and hero-choice
  examples with a shared result announcement. Hero content stacks on narrow
  screens. Samples use one native command button each and no nested controls;
  native-link support for destination tiles remains a separate API addition.
- Added XX-large to the property radios. Sample graphics are self-contained
  authored SVG data URLs, so source and installed-package pages need no image
  service. Adjacent icon samples use the theme guard gap.
- Verification: 15 action browser stories pass, including profile inheritance,
  token overrides, both-axis targets, compact edge activation, wrapping long
  content and single activation from projected content. Theme build:tokens and
  a fresh Phase 13 verifier pass (manifest/presentation, shape, contrast,
  focus/target size, reduced motion and forced colors). Component declarative,
  style and package checks, typecheck and lint pass; lint retains 48 existing
  non-null assertion warnings. Source and isolated-package playground checks
  activate all samples, validate image loading, verify 360px layout/hero stacking,
  compact-only hit borders and icon guard spacing. Desktop composition inspected
  in Chromium.


## Selected actions and state-specific input indicators

Completed 2026-09-28, following the implementation in `c56d95c1`.

- Verified controlled action selection, attribute presence/removal, no automatic
  toggling, `aria-pressed`, keyboard focus, disabled/loading combinations and
  five theme modes. All 27 action/select stories pass in the browser runner.
- Added direct CSS verification to `verify-input-indicator-forced-colors` for
  ten native control owners across five modes: resting/hovered inputs and labels
  have no extra shadow; focus, checked/mixed, invalid and pending feedback has
  visible width and preserves geometry. Disabled controls suppress invalid and
  pending shadows. All ten controls retain pending/focus outlines in forced
  colors, and the existing forced-color keyboard/option assertions pass.
- The direct fixture exposed missing pending selectors for datepicker and
  timepicker native inputs. Added their pending width/color, forced-color outline
  and focused-pending precedence. All seven timepicker browser tests pass.
- Preserved the disabled native radio's resting forced-color treatment when
  checking hover suppression; a browser-provided outline need not be zero.
- Scoped playground icon spacing to the group containing `#size-icon`; the
  selected-choice example also uses `.icon-actions`. Source and isolated archive
  checks pass for action/theme-switch playgrounds, selected container ownership,
  responsive compositions, theme controls and pending theme gradients.
- Theme generation/manifests and Phase 13 checks pass. Theme and component package
  checks, component declarative/style/state-matrix/catalog gates, typecheck and
  lint pass; lint retains 48 existing warnings and no errors. Nx reused valid
  cached prerequisites where applicable. An initial overlapping build invocation
  raced while copying runtime output; sequential build-dependent checks passed.
- The legacy state file reports 3 passing / 18 failing. Compared all failed names
  with `cem-components-baseline-2026-09-27.md`: no additions or removals. The
  indicator test now stops at missing checked-state geometry because the legacy
  empty `checked` attribute is not reflected. Its component assertions remain
  intact; the direct CSS fixture verifies presentation without claiming to repair
  host reflection. The broader legacy package test gate remains open.

## Component galleries and module-URL settlement

Completed 2026-09-28.

- Routed the action playground to its owned CEM components gallery. Seven
  example cards explicitly cover all 20 implemented attributes, including size,
  selected/selectable, presence semantics and native form overrides. The verifier
  derives the inventory from the canonical declaration and checks examples.
- Added the select property playground, canonical source view, theme controls
  and dropdown/persistent/multiple examples. Exported and packaged both pages;
  source and isolated archive journeys pass.
- Kept selected options visible by scrolling their listbox. Mounting an offscreen
  select no longer scrolls the document; the new regression failed before the
  fix (621px document movement) and passes afterward.
- Captured partial module-URL startup settlement. Following the user's batching
  decision, the native resource path commits sibling results together, retaining
  stale-render/disconnection guards, payload identity and failure diagnostics.
  Four controlled resolver stories pass; six concurrent gallery probes retain
  both stylesheet links. Playground checks use the actual bootstrap runtime.
- Verification: 35 focused browser stories and 74 main runtime stories pass;
  source/package playgrounds, package checks, component declarative/style/state
  matrix/catalog gates, lint and typecheck pass. Lint retains the existing 48
  component and two runtime warnings. Legacy state-suite debt remains separate.
- Next: generate and verify the combined release XHTML bundle through the CEM
  AST pipeline, including fragment loading, dependencies and style ownership.

## Release XHTML bundle and retained source context

Completed 2026-09-28 after the user approved preserving source bases.

- Added shared native URL-chain validation, exposed through WASM. External
  fragment declarations apply the selected template's `xml:base` ancestor chain
  against the final fetched URL. Invalid metadata rejects registration. The
  effective base reaches existing resource/style identities while acquisition
  provenance retains the fetched document and fragment.
- Added a native CEM AST graph that collects canonical XHTML and exports
  `components.xhtml`, with copied declarations under source-base containers.
  Template text is unchanged. The package exports the bundle and includes its
  original sources, build inputs and companion playground.
- Native checks: three source-base tests and one CLI bundle fixture pass.
  Browser checks: all 77 stories in the main runtime and source-context files
  pass. Source and isolated-package playground journeys verify all three bundled
  components, unique IDs, exact template text, metadata, effective bases, one
  retained stylesheet per declaration, selection and duplicate rejection.
  Separate stories cover redirects, relative CSS imports, distinct fragment
  bases in one cached document and invalid metadata.
- Component/runtime lint, typecheck and package checks pass. Lint retains the
  existing 48 component and two runtime warnings. Overlapping verification
  initially raced on generated CEM-ML WASM package files; sequential rebuilding
  and verification passed. No build retry or wait-limit workaround was added.
- The accepted component development pattern is complete. Next: investigate the
  recorded legacy component state-suite failures before continuing migration.
