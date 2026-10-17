## Studio and engine commit verification (2026-10-16)

- [x] Complete shared icon-control installation and verify native theme selection, focus and storage.
- [x] Preserve command-view ownership of the operation-option slice and verify deployed typed forms; give the existing multi-step browser integration 30 seconds after measuring 17.9 seconds in isolation.
- [x] Repair compatibility packaging for full XHTML imports; verify seven authored samples, all five action intents and disabled transitions, painted geometry, and the locked archive inventory.
- [x] Revalidate native engines, CLI and Studio release gates; commit reviewed source while excluding generated screenshots and Vitest attachments.

Evidence: 2,090 CEM-ML unit tests, 124 CEM-QL unit tests, 13 native project tests and three srcset tests pass (two existing CEM-ML and four CEM-QL tests remain ignored). CLI and full Studio check targets pass, including all 94 Studio browser tests, 32 native and deployed Feature Tour examples, 171 declared deployment assets, 261 deterministic output files, clean consumers and synchronized release evidence. Deployed two-project acceptance covers typed Set/Reset forms, draft recovery, export, Trash/Restore, offline reload, resource rename/move, unchanged consumer bytes, revision guards, undo/redo and automated accessibility. The compatibility archive passes Chromium rendering, interaction/forced-color/geometry and JavaScript/type contracts with 384 locked files; changed scripts pass lint. Staging uses an explicit 205-file source allowlist and excludes all generated screenshots and Vitest attachments.

CEM audit: visible Studio UI remains composed from production declarative controls and theme-owned layout/paint tokens, with the operation selector's local state retained by its command view. Stable dataset IDs and resolver addresses remain independent from mutable display/export paths; path edits preserve consumer bytes. No app-local UI JavaScript, component-local token defaults, browser instrumentation prerequisites or consumer rewriting were added.

## Navigation controls and spacing (2026-10-16)

- [x] Fixture: cover navigation icon links/disclosure, exact aria-current forwarding and live aria-expanded updates, native navigation/disabled behavior, all-theme paint and inherited overrides, layout gaps/insets, matching control typography, keyboard focus and retained projection.
- [x] Add navigation-token paint and ARIA forwarding to cem-icon-button; compose the cem-nav disclosure from it and add shared token-driven layout/padding/gaps while retaining native content support.
- [x] Update bundle and navigation/icon-button property pages and galleries, navigation contracts and component reference; verify browser/package/build/lint and forced-color behavior, inspect the rendered result and record CEM audit.

Evidence: all 41 focused navigation/icon-button/theme-switch/gallery-page browser plays pass. The new controls/ARIA fixtures first reproduced the missing composed disclosure and lost aria-current values. Checks cover all five themes, current/hover/held-active/disabled paint, inherited color and spacing overrides, minimum hit targets, matching typography, trusted glyph clicks, Enter/Space/Tab behavior, retained projection and native-link compatibility. All ten source/package checks pass for the updated nav/icon-button property pages, galleries and bundle. Component build/catalog/package verification passes (53 catalog components, 299 packed files); component/script lint passes with the existing 121 component warnings. The navigation forced-colors gate passes with the composed disclosure. Final real-browser source/package bundle inspection verifies all five page modes, native fragment navigation, exact current-page ARIA, glyph/keyboard disclosure, padding/gaps, narrow LTR/RTL bounds and forced-color current paint; the updated component snapshot is saved with this task's outputs.

CEM audit: declarative shared icon-button composition; navigation uses existing --cem-navigation-item-* endpoints, D1 inset/gap and coupling guards, shared control hit geometry, and inherited nav typography. Native links and their discoverable ARIA-disabled activation guard remain supported; disabled icon links retain their existing URL/tab-stop contract. Explicit ARIA values remain owner-controlled. Compact native CSS nesting stays within existing import/specificity limits, with valid XHTML entity escaping. No new theme defaults/aliases, runtime behavior JavaScript, app-local UI code or gallery-only test attributes were introduced. Previously dirty files outside this task remain unchanged.

## Shared persisted gallery page theme (2026-10-16)

- [x] Implement declarative cem-gallery-page with normalized live page storage, page palette and controlled navigation theme selector; retain independent example keys and fixed theme scopes.
- [x] Migrate every source property/gallery/index/bundle page and publish the shared declaration with generated pages; preserve authored samples and navigation.
- [x] Fixture: cover defaults/saved/invalid themes, all five choices, live/cross-page synchronization, retained editors/forms and independent example themes/keys in worker and fallback runtimes.
- [x] Fixture and fix nested theme palette/state isolation in the shared theme generator so explicit example themes survive every outer page mode, including native.
- [x] Fixture and fix generic button value capture for clicks on nested icon/label content; initialize nested demo declarations through an inert page payload.
- [x] Fixture and use the existing viewport boundary for theme menus in page navigation, including real pointer activation near screen edges.
- [x] Extend whole-inventory source/package checks for theme changes, persistence and scope isolation; update gallery/package verification and docs, run build/lint/browser/package gates and record CEM audit.

Evidence: all 108 source playground/gallery/index/bundle pages and the component overview use the shared owner; all 479 authored sample blocks and 260 fixed theme roots remain byte-identical. All 52 focused browser plays pass, covering storage normalization/rebinding, all five choices, independent examples, retained forms/declarations, worker/fallback execution, nested theme paint/state isolation, descendant button values and viewport menu geometry. The whole source/package inventory passes all 218 page checks; final rebuilt source/package theme and bundle checks also pass. Real-browser checks confirm reload/navigation restoration, live cross-tab synchronization, retained edited preview fields, fixed matrix palettes, narrow navigation and forced-color selection. Theme/component builds, manifest validation, runtime/component/script lint (existing warnings only), runtime typecheck and npm package verification (299 packed files) pass. As in the earlier runtime work, typecheck temporarily materialized the CLI dependency's generated command-schema source from its existing schema and removed it afterward.

CEM audit: declarative page ownership, controlled navigation and existing live text storage; example keys and fixed theme roots stay independent. Canonical source tables own per-theme palette resets and bounded state stripes, preserving inherited brand colors and explicit root overrides. Existing popup viewport geometry and the generic runtime button-value decoder cover nested menu glyph clicks. No component/app UI JavaScript, component-local theme defaults, parallel token families or gallery-only test attributes were added. Unrelated previously dirty files remain unchanged.

## Shared icon alignment (2026-10-16)

- [x] Add shared align=start|center|end with centered defaults, logical inline alignment independent of row/column direction, and icon-button forwarding; start-align theme choices.
- [x] Fixture: verify live alignment and fallback, row/column and LTR/RTL geometry, button/link modes, retained projected content and theme menu selection/clearance.
- [x] Update both property playgrounds, variation galleries and contracts; verify browser stories, source/package demos, builds, lint and theme appearance; record CEM audit.

Evidence: all 35 icon/icon-button/theme-switch/menu/dropdown browser plays pass. Geometry checks cover default/empty/invalid values, live changes, row/column layouts in LTR/RTL, projected labels, icon-only/text-only content, button/link modes and retained controls. Build and lint pass (121 existing warnings). Ten source/package import/gallery checks pass; eight source/packaged icon demos additionally verify 24 live preview changes and all 64 alignment examples. Production-source theme inspection confirms start-aligned glyphs, automatic height and adjacent selection/focus clearance at desktop and narrow viewports, with forced-color selection preserved and scrolling only when viewport height is constrained. CEM audit: declarative shared attribute/CSS composition, contextual theme choices and existing inherited icon/action/coupling tokens; no new token defaults, runtime/component JavaScript or gallery test attributes. Other previously dirty file contents remain unchanged.

## Theme menu icon buttons (2026-10-16)

- [x] Replace theme choices with contextual icon buttons and separate sized Unicode glyphs from their labels.
- [x] Fixture: preserve menu radio semantics, selected indicators, keyboard navigation, persistence and menu clearance; cover icon-button role transitions.
- [x] Verify theme/icon-button browser stories, source/package gallery, build/lint and desktop/compact/forced-color menu appearance; record CEM audit.

Evidence: all 27 theme-switch/icon-button/menu/dropdown browser plays pass, including radio-role transitions, glyph sizing, all five saved themes, retained projection and keyboard/indicator clearance. Component build, lint (121 existing warnings) and source/packaged theme gallery import checks pass. Production-source browser inspection confirms 32px glyphs, automatic 360px menu height with all five choices visible at desktop and 328×420 viewports, adjacent selected/focused ring clearance and forced-color selection borders; a 240px-tall viewport correctly scrolls within its 232px panel. CEM audit: contextual intent and shared icon/action/coupling tokens are retained, with no token defaults, component behavior JavaScript or gallery test attributes. System uses the trigger’s half-disc glyph. Pre-existing dirty contents outside the seven intended files are unchanged.

## Automatic dropdown height (2026-10-16)

- [x] Fixture: reproduce the theme menu's unnecessary vertical scrolling and require all five items to fit when the viewport has room.
- [x] Remove the dropdown's fixed four-row cap, retain the shared viewport/boundary constraint, and document content-driven height.
- [x] Verify menu/dropdown/theme and viewport-boundary behavior, rebuild, lint, and inspect the gallery at normal and constrained viewport heights.

Evidence: the geometry fixture first reproduced 320px of menu content constrained to 192px. Removed both four-row limits from the dropdown source; the existing popup controller still owns viewport/boundary clamping. All 16 focused menu/dropdown/theme/surface-geometry browser plays, component build, source/package gallery acceptance and component lint pass (existing warnings only). Production-source geometry shows all five items at their natural 320px height with no overflow, including a 328×360 viewport; a 240px viewport correctly constrains the panel to 232px and permits scrolling. Selected/adjacent focus rings and forced-color outlines remain visible. The gallery preview was inspected; compact sizing was verified through an isolated fixture loading the same production declarations and theme stylesheet because a later cold-gallery probe timed out waiting for asynchronous theme CSS readiness. CEM audit: no row-count policy or new token/behavior added; existing shared menu spacing and viewport safety remain intact. Unrelated dirty-file contents are preserved.

## Template capability documentation (2026-10-16)

- [x] Document `{template @capability=...}` and equivalent HTML metadata in the form demo, runtime README and loading contract; demonstrate template-owned choice behavior in the existing form-input sample.
- [x] Correct obsolete fragment-import instructions and add component documentation caveats about metadata preservation, explicit overrides and component-specific capability names.
- [x] Fixture: verify all five form cases with the capability on the template, source-loaded and standalone; run focused unit/browser coverage, fixture acceptance and applicable lint/typecheck.

Evidence: the form introduction documents `{template @capability=...}` and equivalent HTML, registration timing, inheritance and explicit empty overrides. The existing fifth sample puts `choice-select` on its template while retaining all five legends and use cases. Runtime/loading docs and component contracts now explain source-owned defaults, copying metadata and the distinct form ownership of native-input wrappers; obsolete repeat-on-fragment guidance is removed. All 8 form source-contract unit checks, 7 form/source-capability browser plays, and standalone/source-loaded form fixture acceptance pass. Both lint targets and runtime typecheck pass; typecheck uses the existing transient CLI-schema preparation and removes it afterward. Twenty added local documentation links resolve. CEM audit: no component behavior, styles, tokens, new demo cases or test-only sample hooks added; form ownership and required intrinsic capabilities remain explicit. Unrelated dirty-file contents are preserved.

## Menu indicator spacing (2026-10-16)

- [x] Fixture: reproduce the theme menu's clipped selected zebra ring; verify popup-edge clearance and item separation with default and inherited spacing.
- [x] Add theme-owned inset and gap spacing to the shared menu composite, respecting the coupling guard; document the shared layout contract.
- [x] Verify menu/theme/dropdown interactions, inherited overrides and forced colors; inspect the updated menu and record the CEM audit.

Evidence: the original popup had zero inset/gap and clipped its selected 6px zebra ring. The shared composite now consumes the control inset and related gap, floored at one coupling guard around the content and two between items (8px/16px by default). A new geometry play reproduces both edge clipping and overlapping adjacent selection/focus rings before the fix, then verifies default spacing, inherited 20px/18px overrides and the safety floor under zero spacing overrides. All 17 menu, menu-item, dropdown and theme-switch plays pass, including native keyboard checks. Source/package theme-switch gallery acceptance, component build and lint pass (existing warnings only). The real gallery's selected ring, last-item scrolling, adjacent keyboard focus and forced-color outlines fit inside their available space; an updated screenshot was inspected. CEM audit: canonical D1/D2 tokens retain ownership and inheritance; no new tokens, component-local defaults, app behavior or test-only gallery attributes were introduced.

## Source-owned declaration capabilities (2026-10-16)

- [x] Fixture: verify capability inheritance from selected templates and their owning declarations, fragment isolation, explicit overrides, invalid capabilities and host-behavior conflicts; exercise real field submission, validation and reset.
- [x] Retain selected-source capability metadata before registration and remove repeated capabilities from the theme-switch gallery imports.
- [x] Run focused registration/form browser regressions, gallery checks, build, lint and typecheck; record the CEM audit.

Evidence: six source-capability browser plays cover actual field/text-field sources, prefix-mapped and relative imports, local/inline templates, custom declaration names, fragment selection and isolation, importer/template/owner precedence, and existing diagnostics. All 28 focused loading/form/registration plays and both source/package theme-switch gallery checks pass. Runtime and component builds, both lint targets and runtime typecheck pass; typecheck uses the existing temporary CLI-schema preparation, removed afterward. The fixture first reproduced the missing behavior before the runtime change. Capability metadata is retained on template clones without modifying cached sources or importer attributes; URL resolution and shared-document fetching remain unchanged. CEM audit: source declarations own reusable form/progress behavior; existing explicit overrides remain supported. No component CSS, tokens, app-local behavior or test-only gallery markup added. Pre-existing dirty-file contents are preserved outside the three intentionally edited files.

## Direct imports across demo pages (2026-10-16)

- [x] Replace resolver-slice declaration imports throughout component playgrounds and shared navigation; consolidate library folder mappings in source and package maps.
- [x] Update declaration-loading examples in the runtime module-URL demo while retaining URL controls used for visible links, images and referrers.
- [x] Fixture: check every component demo's source/packaged registration and gallery navigation; run runtime demo/unit regression coverage, builds and lint, then commit only this migration.

Evidence: 112 demo/navigation files adopt direct source imports, replacing 5,135 indirect declaration references; all six source/package import maps use component folder mappings. Component sample markup, declaration inventories and capabilities are preserved. The new `@epa-wg/cem-components:verify-demo-imports` gate passes all 218 source/generated pages, validates document fragments and page-owned registration, renders gallery navigation and requires one shared bundle-document fetch. All 41 focused browser plays, 32 unit checks, the native prefix-map fixture, and the module-URL standalone/source-loaded acceptance pass. Runtime, playground and bundle builds, architecture checks, both lint targets, verifier lint and runtime typecheck pass; typecheck uses the existing temporary generated CLI-schema preparation and removes it afterward. Existing component-owned duplicate-registration diagnostics were reproduced before and after migration; the verifier checks every page-owned diagnostic and nested acquisition/selection failure without treating those dependency repeats as page import failures. CEM audit: no component behavior, CSS, theme tokens or test-only sample markup added. Unrelated theme, Studio and cleanup work remains outside this commit.

## Direct cem-element module sources (2026-10-16)

- [x] Fixture: verify full XHTML prefix/exact mappings with the shared native resolver, then browser declaration loading with fragments, contextual maps, host resolvers, failed resolution and retries.
- [x] Reuse the cem-module-url resolver for bare declaration sources; split fragments first and retain relative/local source loading and host-loader compatibility.
- [x] Pilot explicit mapped XHTML sources in the theme-switch gallery; verify registration, interactions and document fetches before broader convention adoption.
- [x] Design follow-up: adopt the direct full-path convention across demo pages and shared gallery navigation, authorized on 2026-10-16; preserve component-owned relative declarations.

Evidence: 16 native module-resolution tests, 15 focused unit checks and 30 browser plays pass, including nine new declaration-source plays and the existing URL-control settlement, source retry, relative/nested source, URI policy, module/referrer demo and theme-switch cases. The real source gallery registers all 18 direct imports without declaration diagnostics or browser errors and switches through all five themes. Runtime build and lint pass (16 existing lint warnings). Workspace typecheck passes with the CLI dependency's generated command-schema source temporarily materialized from its existing generated schema, then removed; the unprepared typecheck target still encounters that dependency's missing transient source. The shared navigation keeps its legacy icon-button alias during this pilot. CEM audit: no gallery sample, component behavior, CSS or theme-token changes; existing capabilities and relative dependencies are preserved. Resolution failures are evicted only for declaration imports, preserving URL-control failure settlement across rerenders.

## Shared persistent theme menu (2026-10-15)

- [x] Replace theme radios with contextual sun/moon action and shared dropdown/menu; persist selection with declarative local-storage and default to light.
- [x] Cover selection styling, keyboard dismissal, reload persistence, independent scopes and retained projected controls in shared stories and Studio browser checks.
- [x] Update Studio composition, component galleries/reference/accessibility docs; verify inherited tokens, forced colors and rebuild deployment.

CEM audit: declarative shared action/dropdown/menu composition; contextual intent for trigger and all choices, selected zebra through native radio-menu semantics, existing coupling/typography/layering tokens, no component-local token defaults or app UI JavaScript. Added generic button-value capture and corrected focus transitions/return in shared runtime. Verified 36 shared stories, the final 9 theme/dropdown/reference-lifecycle checks, 11 Studio shell tests, lint/typecheck, component catalog/playground builds and static deployment output. Regular Google Chrome verified all five themes across actual reloads, mobile layout, forced-color selection and inherited contextual overrides. Tooltip: “Change theme”.

## Studio vertical rhythm (2026-10-15)

- [x] Add browser regression coverage for installed shared layout components, theme-driven section/block rhythm and inherited spacing overrides. Reproduced the unregistered cem-stack failure before the fix.
- [x] Register shared stack/grid/text components and compose Studio shell, forms and workbench sections with semantic D1 gaps; remove browser-default paragraph margins through shared text composition.
- [x] Verify Chrome layout at desktop/mobile widths, theme/spacing modes and forced colors; rebuild the running Studio deployment. Passed 30 combinations (390/1280px, all five themes, normal/dense/sparse), forced colors and inherited token overrides. Shell/workbench coverage: 66 tests verified, including the targeted dialog rerun after replacing its paragraph selector with shared text. Lint/typecheck and deployment output pass. CEM audit: shared declarative stack/grid/text own D1 section/block/related gaps and D6 typography; no local spacing constants, token overrides, CSS or JavaScript UI rendering. Existing production components retain coupling and theme behavior.

## Remove retired browser research implementation (2026-10-15)

- [x] Remove browser observation/replay hosts, native evidence/preparation/rebinding research, WASM exports and probe fixtures. Ordinary HTML srcset parsing/validation remains supported.
- [x] Remove retired public exports, build inputs and documentation; clean generated artifacts. CLI/WASM rebuilt without research symbols; stale packaged copies and temporary Chromium archive removed, plus approximately 7.3 GiB of obsolete compiled research tests/probes.
- [x] Verify retained native project/command behavior, browser worker execution and Studio build. Native CEM-ML: 2,089 unit tests, 13 project tests and 3 srcset parser tests pass (2 pre-existing ignored tests). Regular Google Chrome CLI worker/command suite and CLI/Studio lint pass; 50 native renderer tests pass (2 pre-existing ignored tests); Studio typecheck, deterministic deployment, 32 native Feature Tour examples and deployed two-project lifecycle/accessibility acceptance pass in regular Google Chrome. Rename/move command execution retains unchanged consumer content. CEM audit: existing native dataset ownership and declarative UI remain intact; no UI styling or theme-token changes.

## Studio project dataset identity correction (2026-10-15)

- [x] Verify native resource references survive path/name changes, serialization and reload; reject ambiguous references. (13 native project tests pass; the research-only SVG tests were subsequently removed with their implementation.)
- [x] Replace Studio rename rebinding with a project dataset metadata edit; preserve consumer bytes and IDs.
- [x] Verify rename, move, collision, stale revisions, undo/reload and command execution through unchanged references (repository/workbench tests and deployed lifecycle in regular Google Chrome).
- [x] Retire custom Chromium checkout, private origin-provider research, build integration and obsolete Studio requirements. Removed approximately 54 GiB of checkout/build/depot-tools files; the temporary patch/notes archive was also deleted during the research-code removal.
- [x] Verify Studio command-service entry point excludes live DOM/paint observation modules from deployment (171 declared assets; 261 deterministic output files).
- [x] Run native and Studio regression checks and record filesystem reclamation and CEM audit. Passed: 13 native project tests, 76 Studio browser tests (repository/workbench/file-system provider), Studio lint/typecheck, deterministic deployment and deployed two-project lifecycle/accessibility acceptance. CEM audit: native dataset identity owns references; existing declarative UI projects committed metadata; no consumer rewriting, application DOM-rendering path, new styling or theme-token changes. The custom browser checkout and private probe integration are removed.

# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [roadmap.md](../roadmap.md), and future
wishlist work lives in [wishlist.md](wishlist.md).

Completed datatype/reference contracts, native-surface capabilities, component
migrations and verification evidence are preserved in the
[2026-10-10 snapshot](archive/todo-snapshot-2026-10-10.md). Its checkbox states are
historical; only the unfinished actions below govern execution. Adopted contracts
remain in their design documents. Earlier history is linked from the snapshot.

Current priority (2026-10-12): **CEM Studio — user-owned project workspace**.
The shared tree-to-host command bridge is complete. Next prove shared library
navigation workflows, then deliver create/import/open/edit/save/reopen/export
with guarded navigation.
The [milestone contract](cem-studio.md#next-delivery-milestone-user-owned-project-workspace)
defines scope and acceptance; its ordered implementation checklist is below.
Studio/Site declarative UI migration is complete. All 54 CEM components are
canonical and the legacy registry/installer is retired.
Use application needs to polish the shared library against Angular Material
functionality: implement useful missing controls in `cem-components` first,
with reusable JavaScript behavior in `cem-elements` and CEM theme styling in
the declarative component. Application JavaScript owns non-UI services only.
Presentation preparation and resume context remain preserved below.

Standing requirement: CEM theme principles govern every design decision and
completion review. Check semantic intent, bounded variation, accessibility,
cross-dimension harmony, theme survivability and theme-owned inherited tokens
against the [canonical principles](../packages/cem-theme/src/lib/tokens/index.md#core-cem-principles).
Passing a token-name check alone does not establish semantic conformance.

## Declarative UI Architecture Correction

### Remaining declarative UI migration

Current inventory: **54 canonical / 0 legacy components / 0 legacy authored
code files**. Follow the [declarative UI principle](declarative-ui-principle.md)
and [component development pattern](component-development-pattern.md).

- [x] **Migrate cem-datepicker**, starting with a native
      fixture for its recorded template-brace defect. Preserve keyboard, value,
      form and reset contracts in colocated stories and source/packed galleries;
      add generic runtime capability where declaration support is missing.
  - [x] Fixture: compile and render the canonical datepicker module natively,
        covering named day/week templates, blank edge cells and boolean output;
        retain the recorded brace regression before component cutover.
  - [x] Fixture and implement shared calendar-input behavior for exact native
        owners, bounded Gregorian dates, modal draft/Apply focus and event order,
        composition guards, reset, reconnect and worker/fallback execution.
  - [x] Transfer all seven legacy assertions into colocated declaration stories;
        add boolean presence, boundary years and five-theme coverage.
  - [x] Fixture: consecutive parent renders must update a child's inert authored
        input payload during full reconciliation, retaining native owner identity,
        live/default value, labels, form names and component-owned output in both
        worker and fallback execution; fix the shared projection boundary.
  - [x] Ship scoped token CSS, property controls and five-theme gallery; verify
        source/packed/bundle consumers, forced colors and package gates, then
        retire the legacy owner and transfer state/parity evidence.
  - Evidence: native module regression, 19 declaration-registry unit checks,
    22 shared calendar/input/select/patch plays, 17 datepicker/autocomplete plays,
    22 existing workflow/primitive regressions and source/packed/bundle/forced-color
    integration pass. See the [migration record](cem-components-baseline-2026-09-27.md#datepicker-migration--2026-10-11)
    for the full-reconciliation payload fix and verification boundaries.
- [x] Migrate cem-timepicker and cem-slider, preserving native editor/range
      ownership, value, form, reset and interaction contracts.
  - [x] Fixture: transfer the seven timepicker and six slider legacy browser
        cases into colocated declaration stories without dropping assertions.
  - [x] Fixture: shared time-input and range-input capabilities must retain exact
        native owners, silent reset/reconnect, presence semantics, composition
        guards and strict malformed-payload rejection in worker and fallback.
  - [x] Fixture: scoped CSS must style a projected native input's browser thumb
        while still excluding projected element descendants and nested DCE
        interiors. Reproduce pointer hit-testing, then fix the native scope
        wrapper and compatibility emitter with native/browser regression checks.
  - [x] Fixture: a parent changing a slider between single and range modes must
        update conditional authored input payload, retain the existing input,
        and preserve native values/forms in worker and fallback execution.
  - [x] Implement shared capabilities and canonical scoped declarations; ship
        property pages, five-theme galleries and bundle examples, then verify
        source/packed consumers, forced colors, state/parity and package gates.
  - Evidence: all 13 transferred component plays, four shared worker/fallback
    plays, six native CSS regressions, 46 runtime unit checks and 22 existing
    workflow/primitive regressions pass. Source/packed/bundle, forced-color and
    package gates pass. See the [migration record](cem-components-baseline-2026-09-27.md#timepicker-and-slider-migration--2026-10-12)
    for the scoped native-thumb fix and verification boundaries.
- [x] Migrate navigation and interactive content owners, including cem-nav,
      cem-tabs, cem-stepper, cem-tree, cem-list and cem-chip; resolve their
      recorded presence/focus failures through the adopted owner contracts.
  - [x] Fixture: transfer every tabs, stepper and tree contract assertion to
        colocated stories; retain navigation/content presence, hover, focus,
        active, disclosure, selection and passive-mode assertions.
  - [x] Fixture and implement reusable tab-panel, workflow-panel, tree and
        navigation capabilities with exact owners, stable relationships,
        manual focus/activation, nested-owner isolation and reconnect coverage
        in worker and fallback execution.
  - [x] Fixture: presence-only collapsible/expanded, selectable and
        checkable/checked attributes must work with empty and false-valued
        attributes; live changes retain native focus and state without events.
  - [x] Ship all six canonical scoped declarations, property pages and
        five-theme galleries; retire legacy owners and transfer state/parity
        evidence, then verify source/packed/bundle, forced colors and regressions.
  - Evidence: 34 transferred component plays, four shared worker/fallback plays,
    19 declaration-contract unit checks and 17 remaining primitive/workflow
    regression cases pass. Source/packed/bundle, five forced-color gates,
    architecture, styles, state/parity, lint, typecheck and package checks pass.
    The three pre-existing legacy card/workflow loading/empty failures remain.
    See the [migration record](cem-components-baseline-2026-09-27.md#navigation-and-content-migration--2026-10-12).
- [x] Migrate `cem-surface`, `cem-text`, `cem-stack`, `cem-grid`, `cem-divider`
      and `cem-card` to canonical declarations, resolving loading/empty failures.
  - [x] Fixture: transfer the three existing card/surface state cases and every
        divider assertion to colocated stories before changing their behavior.
  - [x] Fixture: busy/empty/decorative presence includes empty and false-valued
        spellings; live changes retain owners, projected children, edited form
        values and focus in worker and fallback execution. Formatting containers
        stay passive and never infer or inherit workflow state.
  - [x] Fixture: verify stack gap variants and responsive auto/equal-track grid
        geometry using production declarations and bounded width changes.
  - [x] Define token-based scoped layout/typography/surface styling and stable
        public parts; preserve divider spacing, orientation and forced colors.
  - [x] Ship six property pages, five-theme galleries and bundle examples;
        retire legacy definitions/tests, transfer coverage evidence and verify
        source/packed delivery, state/parity, styles, types and regressions.
  - Evidence: all 21 layout/content plays and 34 navigation/content regressions
    pass together. Source/packed/bundle delivery, divider forced colors,
    architecture, styles, state/parity, package, lint and typechecks pass.
    See the [migration record](cem-components-baseline-2026-09-27.md#layout-and-content-migration--2026-10-12).
- [x] Audit the completed layout/content set against CEM theme principles.
  - [x] Fixture: reproduce the grid's misuse of a reading maximum as a layout
        minimum and the missing data numeral features; verify inherited theme
        overrides for all six owners across the five theme modes.
  - [x] Give intrinsic grid minimums a canonical D1 token; consume complete D6
        data typography, reading optical sizing and D4 work-layer surface separation
        without local defaults.
  - [x] Verify source-table generation/manifest, theme contrast and forced
        colors, component plays, source/packed delivery and document the audit.
  - Evidence: 61 component/regression plays pass, including six five-mode
    inheritance audits. Theme Phase 13 (source tables, manifests, generators,
    modes, forced colors, accessibility smoke and cross-spec checks), component
    source/packed/bundle delivery, styles, package, lint and fresh typechecks pass.
    The [audit](../packages/cem-components/docs/layout-content-contract.md#cem-principle-audit--2026-10-12)
    records corrected semantic mappings and the boundary of browser evidence.
- [x] Migrate disclosure/data owners: `cem-expansion`, `cem-table`,
      `cem-sort-header` and `cem-paginator`, resolving the 11 recorded failures.
  - [x] Fixture: transfer all 15 existing expansion/sort/paginator cases to
        colocated stories without dropping assertions; cover passive table
        semantics and application-owned data ordering.
  - [x] Fixture and implement generic disclosure, sort-action and pagination
        capabilities: exact owners, presence attributes, stable focused nodes,
        nested-table isolation, silent author changes, reconnect and both
        worker/fallback execution. Cover fractional invalid page sizes.
  - [x] Ship canonical scoped declarations, four property pages and five-theme
        galleries; verify source/packed/bundle consumers and forced colors,
        then retire legacy owners and transfer state/parity evidence.
  - [x] Fixture and audit inherited semantic tokens in five modes, target/focus
        geometry, data typography, accessibility and passive table ownership;
        record CEM principle conformance and refresh the remaining suite.
  - [x] Fixture: a stylesheet begin must reacquire its compiled source after
        bounded worker artifact-cache eviction. Reproduce the bundled catalog
        failure without enlarging the cache, then verify worker/fallback and
        existing generation cancellation/release contracts.
  - Evidence: all 15 transferred cases, the passive table case, four five-mode
    inheritance audits and three shared worker/fallback lifecycle plays pass.
    Source/packed/bundle delivery, all three forced-color contracts, styles and
    package gates pass. The audit enforced D2 target/guard floors; a reproduced
    worker cache-eviction defect is fixed with 68 passing runtime unit checks.
    See the [CEM principle audit](../packages/cem-components/docs/disclosure-data-contract.md).
- [x] Close the full component gate before the next migration set. The configured
      package gate and all 59 dependencies pass; the canonical cohort is
      **242/242 across 39 files**, and the remaining legacy suite is **23/23
      across five files**. All assertions and deadlines remain intact.
  - [x] Fixture: parent updates retain the explicit payload envelope's patch
        identity and reconcile its unwrapped inert source. Worker/fallback plays
        preserve edited input values and exact input/dialog owners while changing
        parent attributes and inserting/removing an optional toggle.
  - [x] Fixture: Escape from a hovered tooltip surface followed by authored
        `trigger-for` rebinding restores focus interest on the alternate launcher.
        Hidden surfaces release stale pointer interest even without pointerleave.
  - [x] Fixture: a newer native radio choice survives an older child publication.
        Capture from render request time; preserve the same native owners and
        update defaults without overwriting live user state. Cover full plans,
        worker patches, DOM fragments and worker/fallback runtimes. Fifty rapid
        navigation presence cycles pass, as do both affected property gates.
  - [x] Fixture: captured picker lifecycle failures retain their native owners.
        Native disabled availability, dialog close/focus and commit events now
        complete independently of async publication. Matching retained time
        choices update active/selected state before showing; changed choices
        await publication.
  - [x] Refresh the offline component/Figma inventory against all 53 current
        declarations and executable states. Parse primary reference tables
        separately from detailed attribute tables, retaining duplicate checks.
        All 53 external component reviews remain planned.
  - [x] Fix inferred browser-test task dependencies so runtime builds complete
        before tests import their artifacts. The configured full gate proves
        the ordering and all five browser/spec targets pass.
  - [x] Re-run the complete canonical cohort and configured package gate, plus
        102 runtime unit checks, focused regression plays, fresh types and lint.
        Lint retains 115 component / 15 runtime warnings, with no new warnings
        and zero errors. Record the CEM principle audit in the
        [aggregate repair evidence](cem-components-baseline-2026-09-27.md#aggregate-lifecycle-repairs--2026-10-12).
  - [x] Refresh offline Figma token metadata from all five actual mode files:
        272 variables (64 COLOR / 124 FLOAT / 84 STRING). Preserve the historical
        230-variable manual review and pending external refresh.
- [ ] Conditional diagnostic follow-up: if slider property-page root readiness
      fails again, use the new failure capture to attribute it before changing
      behavior. One initial aggregate timeout remains unexplained; the unchanged
      source/packed gate, 20 reloads under load and final full gate pass. Passing
      repeats do not establish a cause or a specific slider fix.
- [ ] Conditional diagnostic follow-up: if datepicker/suggestions failures
      recur during concurrent browser runs, capture their focus, dialog and
      processing state before changing behavior. Running the canonical cohort
      alongside the package gate produced two 30-second play timeouts and two
      following focus/open failures (262/266); the standalone cohort passes
      266/266 with unchanged deadlines and assertions. No runtime cause is
      attributed. Preserve this distinction in subsequent acceptance reports.
- [x] Migrate the feedback/status set: `cem-toast`, `cem-progress`,
      `cem-progress-spinner`, `cem-skeleton` and `cem-alert`.
  - [x] Fixture and implement reusable progress-status preparation in
        `cem-elements`: presence-selected mode, finite range normalization,
        clamping, retained owners, no input handlers/events, worker/fallback.
  - [x] Fixture and migrate all five declarations with colocated plays for
        native progress, live status/urgent feedback, hidden placeholders,
        projected content, retained identity and inherited theme overrides.
        Port every spinner assertion before retiring its legacy suite.
  - [x] Fixture and ship property controls, five-theme galleries, source,
        bundled and packed consumers, forced colors and reduced motion.
  - [x] Retire the five legacy declarations and spinner behavior/spec; update
        package/catalog/state/parity/Figma evidence, run aggregate verification
        and record the CEM principle audit and remaining limits.
        Final evidence: 266/266 canonical plays across 44 files; 25/25 focused
        feedback/runtime plays; 18/18 remaining legacy checks across four files;
        full 60-task package gate; final source/packed/bundle feedback gate;
        fresh types and lint (115 component / 15 runtime existing warnings, no
        new warnings or errors). See the [feedback/status contract and CEM audit](../packages/cem-components/docs/feedback-status-contract.md)
        and [verification evidence](cem-components-baseline-2026-09-27.md#feedbackstatus-migration--2026-10-12).
- [x] Finish every remaining legacy member as
      `src/components/<cem-tag>/<cem-tag>.xhtml`, with embedded once-per-declaration
      scoped CEM-token CSS and colocated CSF Next `.stories.ts` play tests.
      Move missing reusable behavior into `cem-elements`, ship property pages
      and five-theme galleries, and verify source, bundle and packed consumers.
      Reduce both legacy component and authored-code counts to zero.
  - [x] Fixture and migrate the nine passive/inert owners (`cem-option`,
        `cem-option-group`, `cem-badge`, `cem-avatar`, `cem-media-preview`,
        `cem-tree-item`, `cem-app-bar`, `cem-tab`, `cem-step`): payload consumption,
        accessible identity/media/landmark semantics, retained nodes, worker and
        fallback, inherited tokens and five-theme/forced-color delivery.
  - [x] Preserve all remaining workflow and substrate-harness assertions in
        colocated component plays or shared runtime tests before deleting the
        legacy suites. Fixture source, bundle and isolated-package consumers.
  - [x] Fixture and fix shared choice-list rendering when every option is
        disabled: an empty active-option ID must not become the selector `#`.
  - [x] Retire the registry, installer, unused option copy and package-owned
        tooling/configuration; update consumers and Nx gates. Enforce zero
        legacy components and zero authored code except colocated stories.
  - [x] Verify Studio XHTML resource loading and its static asset manifest;
        replace the stale 31-schema tour assertion with the canonical runtime
        schema count and retain the one-example-per-package invariant.
  - [x] Fixture: await the nested icon before capturing icon-button hover/active
        baselines. The aggregate failure captured incomplete icon markup; retain
        every exact markup, geometry, focus, event and runtime-state assertion.
  - [x] Run canonical stories after the full package delivery prerequisites,
        audit CEM principles, and record completion evidence and remaining limits.
  - Evidence: **318/318 plays across 54 files** (316 canonical / two shared
    substrate), all 55 package prerequisites, source/packed/bundle and forced
    colors, refreshed architecture/style/state/parity/Figma/package gates,
    fresh runtime types and lint pass. Lint retains 115 component / 15 runtime
    warnings, with no new warnings or errors. Studio's five shell tests, static
    output (98 assets / 188 files) and UI classification checks pass. Package
    publication contains 296 files and zero JavaScript/TypeScript. The
    [CEM principle audit](../packages/cem-components/docs/passive-payload-contract.md#cem-principle-audit)
    and [completion record](cem-components-baseline-2026-09-27.md#passivepayload-migration-and-legacy-retirement--2026-10-12)
    preserve exact ownership, semantic token mappings and remaining manual
    browser/device/AT limits. External Figma review remains 0 completed / 53
    planned; Studio's subsequent visible UI migration is recorded below.
- [x] Re-establish the remaining legacy suite baseline after layout/content
      cutover without weakening assertions: **27 passing / 11 failing / 38 total**
      across all eight remaining specs (five passing files, three failing).
      The original historical baseline remains in the
      [failure inventory](cem-components-baseline-2026-09-27.md).
- [x] Migrate remaining Studio and Site visible DOM construction, UI listeners
      and state projection to XHTML/CEM-ML, retaining JavaScript only for non-UI
      services and host adapters.
  - [x] Migrate Studio application shell markup, interaction bindings and state
        projection to XHTML/CEM-ML; reuse shared select/action/status components.
  - [x] Fixture and implement a generic `cem-elements` host-service view binding:
        explicit command allowlist, synchronous invocation for browser gestures,
        scalar event values, state updates, nested-view isolation, retained focus,
        errors and disposal in worker/fallback execution. No app-owned DOM handlers.
  - [x] Fixture the declarative shell with theme persistence, install/update and
        file-provider availability, native action invocation, retained workbench
        content, disposal and production module-map delivery. Preserve old checks.
        Rapid repeat activations must not duplicate pending host operations.
  - [x] Close Studio's deployed runtime module graph over the migrated shared
        capabilities; fixture every relative JavaScript import against declared
        output assets so omitted modules fail before browser startup.
  - [x] Add a focused deployed-shell gate for production CSS, token inheritance,
        forced colors and offline reload, while retaining the full worker/Feature
        Tour gate and recording its independent schema-validation failure.
  - [x] Mount declaration owners so native stylesheet ownership can publish;
        authorize generated style elements in Studio CSP, retain strict script,
        style-attribute and preview policies, and verify actual deployed styles.
  - [x] Native fixture: module-prelude selected attributes must reflect exactly
        like unwrapped declarations, preserve authored defaults and source data,
        and exclude body/named-template bindings; fix before shell theme cutover.
  - [x] Audit the shell against CEM principles, verify Studio build/browser gates,
        fixture deployed five-mode styling, inherited token overrides and forced
        colors, and record workbench and Site follow-up boundaries.
  - [x] Fixture supported shell action intents; replace retired quiet/secondary
        variants with contextual/alternate and mark confirmed replacement
        destructive under the canonical CEM action vocabulary.
  - [x] Update Studio's incompatible-replacement confirmation to canonical modal
        dialog opening, accessible heading and `cem-close`; retain the existing
        replacement/cancellation browser fixture and assert browser modality.
  - [x] Reconcile Feature Tour fixture counts with all 32 registered schema
        packages, including `cem-data-island`; preserve exact counts and byte/hash
        checks in output verification.
  - [x] Repair native validation of the existing `cem-data-island` complete-instance
        example, then rerun the full Studio Feature Tour/static-worker gate.
        The native command now reports zero errors/warnings; both the public
        engine and package-example path use the selected model's namespace claims.
    - [x] Add native fixtures for data-island adapter/model admission, the complete
          example, required attributes and kind values, unknown parts/foreign
          namespaces, and namespace claims without a ready selected model; retain
          source coordinates and HTML payload checks. Cover the schema-package
          example-validation path as well as the public engine command.
  - [x] Migrate the Studio workbench's source, command, result and diagnostic views
        and their interaction/state bindings. First finish any reusable control
        or composite in `cem-components`, with JavaScript capability ownership in
        `cem-elements`; replace remaining retired action intents during cutover.
    - [x] Fixture shared host-command native form snapshots in worker/fallback:
          successful scalar entries, repeated names, disabled fields, submitters,
          nested-view isolation, synchronous invocation and formdata-event revocation.
    - [x] Fixture and implement declarative form-control selection requests in
          worker/fallback: explicit focus, retained caret on unrelated renders,
          repeated navigation, invalid/disabled/composing requests and disposal.
    - [x] Add a canonical textarea navigation example and colocated fixture;
          document the reusable selection attributes in its contract/playgrounds
          and fixture the five new declarative property controls.
    - [x] Migrate source editing, status and operation controls to a CEM-ML view;
          fixture save/reload, native form selections, duplicate pending commands,
          diagnostic navigation, focus retention and deployed asset delivery.
          Verify all five themes, inherited input/action tokens and forced colors.
          Command and result cutovers are completed below.
    - [x] Migrate command editing, semantic changes, apply/run targeting and
          replacement confirmation. Add any missing shared native-surface close
          binding before removing application listeners; retain revision guards.
      - [x] Fixture shared host-command surface dismissal in worker/fallback:
            exact owner, nested-surface isolation, cancellation and disposal.
      - [x] Fixture retained command/target edits, semantic-change text,
            pending-command exclusion, native replacement dismissal and declaration
            delivery; verify themes, inherited tokens and forced colors for the
            migrated command view.
    - [x] Migrate result previews, execution/expected/trace/graph views and
          diagnostic/provenance navigation. Build any reusable viewer in the
          component library first; preserve preview isolation and exact output.
      - [x] Extend canonical media-preview with text, isolated document and
            message modes; fixture exact text, deny-all sandbox, retained owners,
            property controls, inherited tokens and forced colors. Keep media
            payload/caption compatibility and update its gallery/contract.
        - [x] Accept native formatter whitespace before fenced style content in
              the architecture scanner while retaining its CSS/token checks.
        - [x] Verify new media-preview controls and exact text/document modes in
              source and isolated package galleries, including forced colors.
      - [x] Fixture declarative workbench results and navigation: exact transfers,
            duplicate pending operations, stable previews/focus, expected/trace/
            graph/report values, empty states, disposal and production delivery.
        - [x] Normalize native trace events into bounded path/value records;
              fixture raw event shapes and verify deployed trace/graph row counts.
        - [x] Audit neighboring workbench array counts; fixture zero validation
              diagnostics and empty/multiple semantic changes in their labels.
  - [x] Replace Site's retired installer imports and migrate interactive examples
        and search UI through the shared declarations and host-service boundary.
    - [x] Load canonical component declarations through `cem-elements`; retire
          Site's primitive installer and legacy runtime aliases.
    - [x] Migrate token filtering, action examples and native output presentation
          to CEM-ML. Build any useful missing viewer/control in the library first.
    - [x] Migrate search fields, result lists, status and navigation bindings;
          keep index loading/matching and URL transport in non-UI services.
    - [x] Fixture Site interaction/search and static delivery, then audit all
          five themes, inherited overrides, keyboard focus and forced colors.
      - [x] Fixture canonical Site deployment: explicit runtime asset closure,
            standalone XHTML loading, scoped styles and no retired imports.
      - [x] Fixture token filtering, retained field focus/value, action count and
            exact literal rendered HTML in the shared media-preview component.
      - [x] Fixture search phrase ranking, empty/multiple/literal queries, native
            form submission, URL preservation and the complete no-JS index.
      - [x] Fixture production themes, inherited surface/action tokens, keyboard
            focus and forced colors; retain route/provenance/determinism gates.


Site migration evidence (2026-10-12):

- `examples-view.xhtml` and `search-view.xhtml` own visible composition, bindings
  and state presentation. Token filtering and counting use declarative slices;
  the views reuse canonical field, action, surface and media-preview components.
  Search matching and URL transport remain host services. The native build's
  complete semantic index supplies bootstrap control data and remains available
  without JavaScript; imported source JSON stays inside the native CEM boundary.
- Bootstrap snapshots precede component upgrade. Host updates publish fresh
  state values, preserving edited native owners and focus. The shared text preview
  displays exact browser-serialized greeting paragraph HTML, with hydration
  metadata outside the example output. No app-owned UI listeners, DOM generators,
  visible writes, CSS or component behavior modules remain in Site services.
- The production graph declares **96 assets per interactive route**, including
  the complete `cem-elements` module graph, worker/WASM, standalone component
  XHTML and both app views. Retired installer/runtime aliases are removed. Named
  action/select XHTML transport, selection and scoped CSS pass. Component catalog
  provenance now requires canonical declarations. Eight stale reference fragments
  and a runtime README heading-level gap are corrected at their owning sources.
- `cem-site:verify` passes its full dependency chain: examples, search, production
  and provenance gates; **26 routes**, **25 searchable documents**, **245 files**,
  and two byte-identical clean builds
  (`bb8a635d608aab72263925d6d8b5e617dda0c3eb28366644e2662ad2fcf0f4e3`).
  Checks retain exact/literal output, case/empty filters, edited child values,
  owner/focus retention, phrase ranking, empty/multiple/literal searches, native
  Enter submission, unrelated URL/history state, no-JS index and all fragment
  targets. Site lint passes with no warnings or errors.
- Both view conversions pass native tabular reparse and native runtime rendering;
  search also passes a compiled-template **25 → 1 → 25 → 1** result sequence.
  Standalone CLI schema diagnostics still flag runtime vocabulary/whitespace
  annotations; formatter success is not a claim of complete schema validation.
  Deployed runtime diagnostics are clean.
- CEM audit: named palette samples use shared surfaces with paired D0 foregrounds;
  filtering/count/result statuses retain text cues. Existing D1 inset, D2 coupling,
  D3 shape, D5 boundary/focus and D6 reading/script roles remain theme-owned.
  All five theme modes, inherited palette/inset/script-font/action-radius values,
  the action's coupled border geometry and forced-color indicators pass. No new
  tokens, local defaults or alternate palette families are introduced. Production
  keyboard and naming checks pass in Chromium; physical-device, other-browser and
  spoken AT acceptance remain open. Presentation context remains deferred.

Studio results/preview migration evidence (2026-10-12):

- `workbench-layout.xhtml` and `workbench-results.xhtml` own result/status,
  execution, expected, trace, graph, report and diagnostic/provenance composition.
  The static layout mounts independent source, command and result views; state
  updates retain child owners, focus and unchanged document `srcdoc`. Shared host
  commands invoke exact copy/download and source navigation, with synchronous
  pending guards and disposal. Workbench JavaScript now owns services and bounded
  data transport; its visible DOM generators and UI listeners are retired.
- Canonical `cem-media-preview` now supplies explicit text, document and message
  modes alongside its compatible media/caption slots. Text preserves literal
  values and whitespace. Documents retain a fixed empty sandbox, opaque origin,
  no-referrer policy and deny-all CSP. No new JavaScript capability was necessary.
  Studio's legacy preview renderer, CSP export and preview stylesheet are retired;
  byte budgets, UTF-8 validation, exact transfers and redaction remain services.
- Native trace events now become at most **100** path/value records. A 105-event
  fixture verifies first/last displayed values and the bound. Declarative empty
  states and counts select array members, fixing blank lists/tables and singleton
  container counts in metadata, validation and command labels. Record text keeps
  booleans/nulls and literal markup; truncation preserves Unicode code points.
- Verification: media-preview **8/8** colocated worker/fallback/theme/control
  plays; Studio shell **7/7**, workbench **15/15** and preview **5/5** browser
  fixtures. Component build, source/bundle/isolated-package passive galleries,
  Studio typecheck/lint and runtime typecheck pass. Component lint has no errors
  and retains **115** existing warnings. Native tabular reparse is byte-identical.
  The architecture scanner accepts native formatter spacing before CSS fences
  while retaining its token and ownership checks.
- Final deployed `verify:static-worker` passes: **32/32** Feature Tour examples,
  all **5** portable workbenches with rendered trace/graph counts matching service
  records, exact copy/download, production themes/overrides/forced colors,
  keyboard reachability, reduced motion, sandbox hardening and offline reload.
  Output has **168** declared assets / **258** files; two clean builds are
  byte-identical (`53dab1ee54f5beb5938b1ca0064dd14fad0854a6b1634fc5cecc983a3ac109d8`).
- CEM audit: native figure/preview semantics and canonical action intents carry
  meaning; modes and record/preview limits bound variation. D0 comfort/text,
  D1 inset/related gaps, D3 media bend, D5 boundary/focus/zebra and the complete
  D6 script typography role stay theme-owned and inherit container overrides.
  The text viewport's `60vb` cap is a structural scrolling limit; iframe height
  retains its native default. All five themes and forced colors pass; no new
  token defaults, aliases, motion or application styling are introduced. The
  isolated document remains outside the host theme. Automated evidence is
  Chromium; physical-device and spoken AT acceptance remain open below.
- Site's canonical loading, examples and search are now complete (see the Site
  evidence above). Studio delivery planning and browser/device/AT acceptance
  remain independent work; presentation and its resume context stay preserved.

Studio command/application migration evidence (2026-10-12):

- `workbench-command.xhtml` owns command editing/status, semantic-change tables,
  target selection, apply/run actions and replacement confirmation. It composes
  canonical textarea, text-field, select, action, tabs, table, alert and dialog
  declarations. Application DOM listeners, command markup generators and direct
  editor/target value writes are removed. Host services retain revision guards,
  publish drafts before asynchronous reads and exclude duplicate pending work;
  older grammar results cannot overwrite newer edits. View data excludes full
  command previews, execution records and committed result bytes.
- Shared `mountCemView` admits `cem-close` only from the bound surface itself.
  Nested dismissal cannot cancel an ancestor task, cancelled close requests do
  not deliver commands, and disposal revokes delivery. The application checks
  that confirmation remains pending before handling dismissal. Native dialog
  close and Escape preserve the repository's replacement authorization boundary.
- Native tabular output reparses byte-identically. Named templates and select
  variables avoid formatter recursion/fencing limits; the CLI retains legacy
  HTML-policy warnings for template vocabulary. Browser compilation passes.
  Semantic changes retain missing/string/JSON distinctions and truncate at 239
  Unicode characters plus an ellipsis without splitting surrogate pairs.
- Shared host-view worker/fallback plays **4/4**, workbench browser checks
  **13/13**, shell **7/7**, runtime/Studio types and lint pass (15 existing runtime
  warnings; no Studio warnings). Component build prerequisites pass. Fixtures
  cover retained editors/carets/modal owners, out-of-order validation, exact
  semantic text, duplicate copy/apply-run, native close/Escape and disposal.
- Full `verify:static-worker` passes: **32/32** native/browser schema examples,
  all five portable workbenches, **167 assets / 257 files**, two byte-identical
  clean builds and online/offline worker/WASM execution. Final build hash:
  `b41f904fa90e41b60a252cd048f788398e30022db7ef4b871d19f8c685865706`.
  Deployed command XHTML and shared host adapter match their source/build bytes.
- CEM audit: existing controls retain canonical D0/D1/D2/D2c/D3/D4/D5/D6/D7
  ownership; copy/apply are alternate, reset/cancel contextual, apply-run/use-new
  primary and replacement destructive. Required labels, textual status cues,
  modal focus and explicit replacement consent remain intact. No application CSS,
  component JavaScript, token defaults or parallel token families were added.
  Five deployed themes, inherited input-background/action-radius overrides,
  coupling borders, forced colors and reduced motion pass. Evidence is Chromium;
  physical-device and AT acceptance remain open. Result/preview/diagnostic
  cutover is now recorded above; Site is next and presentation stays deferred.

Studio workbench source/operation migration evidence (2026-10-12):

- `workbench-controls.xhtml` owns source editing, workbench selection, status,
  revision labels, save/reload and operation forms. The host forwards service
  data and commands through `mountCemView`; result bytes/graphs/command previews
  are excluded from this view's state. Native forms retain programmatic select
  values. A synchronous pending guard prevents duplicate service operations.
- Shared host commands capture frozen successful string form values, retain
  repeated names and submitter data, and reject files or controls from another
  view. Formdata-event disposal revokes delivery. Text selection lives in the
  shared form-control capability, with UTF-16 ranges, explicit focus and one
  consumption per request change; unrelated renders and reconnects do not replay
  navigation. The canonical textarea contract, property controls, inspectable
  gallery example and colocated stories demonstrate the same capability.
- Native tabular conversion/reparse passes. Workbench browser checks **12/12**,
  shell **7/7**, shared host/form-control worker/fallback plays **5/5**, and
  canonical textarea plays **12/12**, including all five new property controls.
  Runtime/Studio type checks and runtime/component/Studio lint pass; lint retains
  the existing 15 runtime / 115 component warnings. The component build passes
  its architecture/state/style gates and emits the updated property/gallery pages.
  No app/component-local interaction code or new theme CSS was added.
- Final `verify:static-worker` passes: **32/32** native/browser schema examples,
  all five portable workbenches, **166 assets / 256 files**, two byte-identical
  clean builds, and worker/WASM commands online/offline. The final build hash is
  `c2ed5d4bde3e39eedaef478c040bb812abbe308ac32ab0bf8665d7ddbc6b4854`.
- CEM audit: existing controls retain canonical D0/D1/D2/D2c/D3/D5/D6 ownership;
  save/run are primary, parse/inspect alternate and reload contextual. Live
  status retains text cues, native labels/forms/validation remain intact, and
  explicit selection respects disabled/composing editors. All five deployed
  themes, inherited input-background/action-radius overrides, coupling borders,
  forced colors and reduced motion pass. No defaults or parallel token families
  were introduced. Evidence is Chromium; physical-device and AT acceptance
  remain open. Command/application and result/preview/diagnostic cutovers are
  now recorded above, including retirement of the remaining legacy UI.

Studio shell migration evidence (2026-10-12):

- `shell.xhtml` owns markup, bindings, theme classes and status projection;
  `shell.js` supplies non-UI services through shared `mountCemView`. No new widget
  or component-local behavior was introduced. Canonical action intents replace
  shell quiet/secondary aliases; confirmed replacement uses destructive intent.
- Native module-prelude selection fixtures **6/6**, existing attribute render
  regressions **11/11**, shared host-view worker/fallback contracts **2/2**,
  shell browser fixtures **7/7**, workbench browser regressions **11/11**.
  Host operations retain trusted browser activation and reject duplicate pending
  operations; native owners, edits, caret and focus survive state projection.
- Studio output verification passes with **165 declared assets / 255 files**.
  Two clean graph builds are byte-identical. Relative import closure includes
  all migrated runtime capabilities. The focused
  `verify:shell-delivery` gate passes against production headers, service-worker
  control and offline reload. All 23 UI classifications still pass; Studio and
  runtime type checks and lint pass (15 existing runtime lint warnings).
- CEM audit: semantic action/status choices use existing components and canonical
  D0/D1/D2/D2c/D3/D5/D6 tokens; no defaults, private aliases or new token families.
  All five modes render actual component CSS. Inherited container inset and
  public action-radius overrides reach controls; the action retains its D2
  coupling border. Forced colors, non-color status cues and keyboard-reachable
  preview checks pass. This is Chromium evidence, not a new browser/AT parity claim.
- Declaration owners are now connected for native CSS publication. Studio CSP
  explicitly permits generated style elements; inline script, general eval,
  style attributes and the separate preview policy remain restricted. The
  complete `verify:static-worker` dependency chain now passes after the native
  data-island repair below. Workbench and Site migration, broader Material
  functionality and the preserved presentation backlog remain open.

Data-island validation repair evidence (2026-10-12):

- Registered the data-island CEM adapter identity and native structural model;
  bound its four declared diagnostics to the shared field-contract reporting
  behavior. The unchanged complete-instance example now has **0 errors and
  0 warnings**, down from 41 errors and 16 warnings.
- Namespace admission uses only the selected ready model's package claims.
  Unknown schemas, foreign namespaces and pending models retain rejection;
  original AST names/source maps remain intact. Both engine and package-example
  validation use captured lexical names. Embedded HTML retains its policy checks,
  including acceptance of the standard `slot` attribute.
- New native fixtures **6/6**, namespace disposition **4/4**, runtime validation
  **5/5**, completion **4/4**, and semantic rules **71/71**, including the new
  package-example fixture. Native tests cover missing attributes, invalid kinds,
  unknown parts, namespace admission, source coordinates and embedded HTML.
- Full Studio `verify:static-worker` passes on the final rebuilt artifacts:
  **32/32** Feature Tour examples, all **5** workbenches, **165** declared assets /
  **255** files, two byte-identical clean builds, and browser worker/WASM commands
  online and offline. Production theme modes, inherited overrides, forced colors,
  reduced motion, keyboard reachability and preview hardening all pass.
- CEM audit: this repair keeps validation in the shared native schema layer;
  it introduces no application UI behavior, token defaults or theme aliases.
  Validation remains bounded to package namespace claims and local-name structural
  contracts. Per-element domain pairing and execution of the named lifecycle
  policies (part order/cardinality, dataset consistency, host state and payload
  delegation) are not added by this repair. Browser evidence remains Chromium;
  physical-device and assistive-technology acceptance stays open below.

### Native-surface delivery follow-ups

- [ ] Verify both pinned [ARIA profiles](cem-element-aria-reference-profile.md)
      in the documented browser/assistive-technology matrix. Record actual
      browser/OS/AT versions, accessibility relation exposure, ordered/repeated
      details targets and error-message announcements. Keep untested combinations
      explicit and the draft profile opt-in. Single-relation accessibility APIs
      may expose only the first target; exported DOM sequences must stay intact.

## Autocomplete and suggestions design for CEM inputs

The shared controller, declarative field composition, native profiles and
host-integrated fixtures are implemented. Remaining acceptance uses the
[suggestions acceptance record](../packages/cem-components/docs/suggestions-acceptance.md)
and its repeatable device procedures; automated input does not establish
physical-device or spoken assistive-technology coverage.

- [ ] Complete physical touch/pen, real keyboard/IME, screen-reader and mobile
      viewport runs for both field providers. Record browser/OS/device/AT versions
      and exact outcomes. Verify tap preserves the editor/caret, pan scrolls,
      terminal IME keys neither select nor submit, and active-row/status
      announcements preserve the field's label/help. Run the plain overflow-box
      touch baseline: the automated gesture did not scroll it in this environment,
      and a wheel-scroll pass does not substitute for touch-pan evidence.
- [ ] Record supported-browser native datalist selection, filtering,
      accessibility and device evidence before claiming additional-profile
      acceptance. Demonstrate numeric option selection stores/submits `1` rather
      than its label One; cover email/URL sanitization, range/step mismatches,
      source withdrawal, physical IME and mobile keyboard/AT. Headed Chromium
      and Firefox probes recorded value/FormData and withdrawal, but did not
      establish visible picker selection; numeric Arrow Down/Enter produced `0`.
      WebKit probes remain blocked by missing host libraries. A connected list
      alone is not selection evidence; unsupported native UI preserves ordinary
      editing without a custom popup fallback.
- [ ] Record supported-device Back/dismiss-gesture evidence before advertising
      platform close support. Use the host-enabled acceptance procedure; the
      ordinary local gallery supplies no platform-close authority. Verify one
      accepted request dismisses the admitted child, preserves text and parent
      state, and releases the watcher so normal navigation resumes. Existing
      Chromium protocol evidence does not establish physical Back/gesture support.
- [ ] **Only when requested:** design field-owned label/submission conversion,
      multiline/email-multiple token completion and full surface-provider
      adoption. Carry explicit value/provenance, selection/history, native
      semantics and relationship grants into each separate contract. Labels must
      not rewrite numeric values; token edits must preserve caret, separators and
      composition; one provider owns each adopted surface.

## Browser failures observed during placement verification

Preserve meaningful assertions and existing story/global deadlines. Capture
failing lifecycle state before changing readiness or runtime behavior; passing
repeats alone do not attribute a historical failure.

- [ ] Fixture: attribute the historical aggregate-only data-table/set-URL and
      native suggestions pointer-tap deadlines recorded on 2026-10-08. Establish
      isolated controls, then capture the failing step under load. The separate
      pan-opening readiness defect and provider story split are complete; the
      final aggregate passed 487/487. Retain every tap, pan-cancellation and
      outside-focus assertion. See the
      [archived investigation](archive/todo-snapshot-2026-10-10.md#browser-failures-observed-during-placement-verification).
- [ ] Investigate runtime aggregate instability on recurrence. The historical
      action-cutover run passed 311/312 stories: `Payload Css Readiness And
      Hydration` expected one style but observed zero; its complete 12-story
      file passed in isolation. Later reader-admission fixture corrections do
      not attribute a post-settlement missing style. A full playground run on
      2026-10-10 also hit the gallery sample-readiness deadline once; all 19
      galleries passed in isolation and the complete source/packed rerun passed.
      Use the verifier's component, URL, missing-host and runtime diagnostics;
      preserve the readiness assertion and reproduce before changing behavior.

## Immediate: Legacy Demo Case Coverage

The static readiness inventory and reproduced scoped-CSS, external-src, retry,
NPM and string-demo fixture corrections are complete. Remaining work is traced
recurrence attribution, not wholesale replacement of frame waits. See the
[current boundary inventory](browser-stabilization-review.tmp.md#reconciliation-of-the-remaining-readiness-inventory).
The approved sequencing permits component migration while these historical
investigations remain open.

- [ ] Attribute the source-loaded cell-overrides stock sample's historical
      initial-warning timeout on traced recurrence. Retain its diagnostic target
      and current deadline. After refreshing packaged WASM, all 32 concurrent
      helper/real-component probes passed on 2026-10-08; warning publication took
      5.65–9.35 seconds under overlapping load without reproducing the failure.
- [ ] Capture declaration/render/worker state on recurrence of the location
      samples' 180-frame wait for both readers. Focused overlapping load passes
      without attributing the historical failure. Preserve exact per-card output
      and resource predicates; inspect authored instances and available lifecycle
      signals before changing the fixture. See the
      [NPM/location evidence](browser-stabilization-review.tmp.md#npm-and-location-readiness-under-overlapping-load).
- [ ] Capture HTTP's historical 300-frame initial article-count timeout using
      the existing opt-in lifecycle/per-card tracing, which rethrows the original
      error. Distinguish declaration/library startup, current rendering and
      future HTTP completion before choosing a correction. If startup polling
      is premature, await existing lifecycle settlement before checking each
      authored instance; preserve resource assertions and the 30-second story
      deadline. Traced follow-up workloads pass without reproducing the failure;
      see the [HTTP observation](browser-stabilization-review.tmp.md#http-startup-observation-without-a-reproduced-failure).

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.

## Deferred work: Declarative Amsterdam 2026 CEM presentation

- [x] Include `packages/cem-components/playgrounds` and its assets in the demo
      copy; the component index linked to pages previously omitted from deployment.
      Refreshed `/home/suns/cem-bin`; verified 20 gallery links, 30 action-gallery
      import-map targets, and rendered action buttons without missing resources.

- [x] Export CSV to XML through the retained typed data tree. Verified namespace
      declarations, escaped cells, and browser coloring. All 55 native conversion
      tests and 18 browser tests passed, plus lint/typecheck. Refreshed the packaged
      demo and verified CSV → XML on ports 8767 and 8768.

- [x] Append YML to JSON and CSV to CEM-ML playground examples. Verified URL format selection and colored conversion output for both examples.
- [x] Add CSV-to-CEM-ML export through the shared typed import and CEMT writer.
      Bind the imported data namespace for valid CEM-ML output; preserve string
      cells and header policy. Four native CSV tests, 18 Chromium stories, lint
      and typecheck passed. Verified colored output on both source and packaged
      sites. `yarn copy:demo` refreshed `/home/suns/cem-bin`; fixed the elements
      build to avoid recursively compiling temporary CLI-generated sources.

- [x] Refresh the packaged demo site with `yarn copy:demo`. Fixed the components
      build to consume Nx-built dependency declarations instead of recursively
      compiling the CLI's removed temporary command-schema source. Copied 470
      files and generated 7 in `/home/suns/cem-bin`; Chromium verified the
      homepage link, all four playground examples, and reciprocal gallery links
      against the packaged shared WASM runtime without browser/request errors.

- [x] Append the Markdown cem-ml playground example; verify Markdown and embedded CEM-ML SVG render together and the shared link restores the source. Build and 18 Chromium tests passed; served-page check confirmed link restoration and inline SVG rendering.

- [x] Add playground title/description fields and named Hello HTML, Hello
      CEM-ML and login CEM-ML links. Include metadata in shared URLs and append
      each copied link to the list; verify preset rendering and URL restoration.
      All three preset links restore and render. Copied links retain metadata,
      source and formats; clipboard fallback also appends the link. Login forms
      render outside the playground controls form. Build, lint, typecheck,
      Storybook build and 18 Chromium tests passed; served-page checks covered
      clipboard success, URL restoration, append order and mobile layout.

- [x] Add a CEM demo playground with editable, colored source, format selection,
      native WASM parsing/conversion, an inline HTML or colored text preview, and
      shareable source/input/output URL parameters. Cover editing, conversion,
      diagnostics and URL round trips with browser interaction checks.
      Delivered `packages/cem-demo-element/demo/playground.html`: Auto input,
      12 textual formats, HTML default output, and clipboard fallback. Build,
      lint, typecheck, Storybook build and all 16 Chromium tests passed. Served
      page checks confirmed URL restore, clipboard success, native multiline
      editing, colors and mobile layout. Unsupported conversions show native
      diagnostics; Auto is a syntax hint with an explicit format override.

Presentation preparation started 2026-10-10 and was deferred at the user's
request on 2026-10-11. Keep its remaining actions for a later explicit resumption;
`continue` currently selects component implementation above. Read the
[presentation handoff](presentation/declarative-amsterdam-2026-handoff.md) after
clearing chat context; it records the starting material, implementation
checkpoint and return path. The existing
[session proposal](declarative.amsterdam.2026.md) is the starting brief. The
[working presentation document](declarative-amsterdam-2026/presentation.md)
records the 2026-10-11 program/discussion review, current agenda and
[presentation minutes](declarative-amsterdam-2026/presentation.md#presentation-minutes).
The user requested adoption-focused framing on October 10 and alignment with
the public agenda on October 11. Use the pitch to explain why the promised
document activities matter; preserve the login walkthrough, platform subjects,
and six proposal intervals. Correspondence and release checks remain open.
The [presentation plan](declarative-amsterdam-2026/presentation-plan.md) owns
ideas, preliminary design, and adoption planning; the
[platform pitch](declarative-amsterdam-2026/cem-platform-pitch.md) owns the
audience-facing message. This file remains the sole execution checklist.

- [x] Write the CEM manifesto and platform pitch around audience outcomes,
      declarative embeddable applications, cross-language data neutrality and
      evolutionary architecture. Ground the security story in slide 6 of the
      supplied DCE deck; distinguish current controls from complete vendor
      isolation. Record schema inventory, demonstrations and claim evidence.
      Saved in [manifesto](declarative-amsterdam-2026/cem-manifesto.md), [platform pitch](declarative-amsterdam-2026/cem-platform-pitch.md)
      and [evidence record](declarative-amsterdam-2026/platform-evidence.md).
      Documentation/source audit only; new demonstrations and benchmarks remain
      unimplemented and unverified.
- [x] Review EPA-concept and DWA as idea sources, not implementation guides.
      Incorporated independent ownership/releases, explicit effects, multiple
      presentation contexts and a proposed version-adapter proof into the
      manifesto/pitch; recorded source links and non-adopted assumptions in the
      evidence record. No implementation contracts or fixtures changed.
- [x] Compare the pitch opening with the official
      [Java EE introduction](https://docs.oracle.com/javaee/7/tutorial/overview.htm).
      Rewrite the opening and spoken pitch around the target market, delivery
      and integration costs, security/performance goals, and CEM's distinctive
      embeddability and independent evolution proposition.
- [x] Broaden the pitch beyond web applications to data processing services and
      pipelines with multiple sources and outputs. Align the manifesto, CLI
      and library use cases, audience priorities, and proposed demonstration;
      record the supported engine/graph entry points and host responsibilities.
- [x] Ground the enterprise transformation-service pitch in the Intel/SOA and
      appliance precedent. Explain format and location independence through
      adapter/resolver boundaries; add a proposed location-change proof and
      state extensible, customizable connectors as a platform capability,
      separately from format adapters and appliance comparisons.
- [x] Reframe security around runtime insulation of each vendor's application,
      pipeline stage, and connector. Promote the six layers (Memory, Storage,
      CPU, DOM, CSS, Network) to a main pitch promise; distinguish declarations,
      host grants, and enforced boundaries. Explain nested safeguards and
      budgets, with per-layer evidence and observable demonstration criteria.
- [x] Replace the engine's implementation-starting-point paragraph with the
      full SDLC proposition: source/type selection, schema reuse, extension, or creation,
      module reuse/development, composition, debugging, testing, packaging,
      browser/server/CI execution, operation, and evolution. Connect these
      stages through shared contracts and diagnostics; align the proposed demo.
- [x] Separate the internal presentation plan from the audience-facing platform
      pitch. Preserve audience strategy, proof design, adoption criteria and
      preparation requirements in the plan; rewrite the pitch as public copy
      and update document references to reflect their distinct purposes.
- [x] Tighten the full platform pitch to the user's concise, easy-to-scan style.
      Preserve its message and capability boundaries; record this style in the
      presentation plan for slides, website, and spoken copy.
- [x] Align schema choices across the pitch, manifesto, plan, and evidence:
      pick, extend, or create your own schemas. Retain all three in future copy.
- [x] Commit and push the existing preparation documents before agenda changes
      (`33e18c99` on `origin/develop`). Recheck the official program and discussion
      #24, then map their subjects into the current 60-minute run. Preserve all
      six intervals, the login lifecycle, formatting/reporting and provenance,
      and named platform modules. Reconcile plan, index, and handoff references.
      Source and documentation review on 2026-10-11 passed; validated 11 segments,
      60 minutes, all six proposal boundaries, and 292 local links/anchors.
      Demonstrations, releases, slides, and rehearsal remain unverified.
- [x] Adopt [discussion #24](https://github.com/EPA-WG/cem/discussions/24) as
      the stable resource link for slides, QR codes, handouts, and the website.
      Record Studio as not ready; keep it in the platform overview as work in
      progress and add its access link when available. Slide drafting can proceed.
- [x] Generate a slide-code preview from the existing login source with the
      native CLI's tabular formatter and the existing CEM HTML colorer. Saved
      the [preview workflow](declarative-amsterdam-2026/code-preview.md), standalone
      HTML, full screenshot, and code-only image. Chromium confirms exact source
      insertion, 61 colored tokens, and no clipping. Direct CLI HTML output
      currently stays plain; the preview uses the syntax demo's CEM WASM colorer.
      Studio is not required. Sasha approved the formatting and colors on
      2026-10-11 for the Google Slides presentation.
- [x] Create the native Google Slides deck with editable narrative text,
      diagrams, timed speaker notes and the approved code colors. Generate and
      verify presentation-only login, query and diagnostic assets from existing
      examples; record exact commands and any demo gaps. Import the local deck,
      inspect its rendered slides, and save the delivery link and generation files.
      Delivered 2026-10-11: 24 main slides, four reference slides, 28 speaker
      notes and a native-deck PDF. See [deck delivery](declarative-amsterdam-2026/deck-delivery.md).
      Verified schema failure/repair and equivalent query source spans; full
      interactive tutorial, participant installation and rehearsal remain open.
- [ ] Apply the canonical CEM theme light palette throughout the slide deck.
      Keep physical slide 2 (the original title slide) in dark mode as contrast.
      Use CEM dark mode for terminal output samples. Preserve semantic syntax
      colors and verify their contrast. The new opening teaser uses light mode.
- [x] Test native animation import for the opening: 2-second fades at 0, 12
      and 24 seconds. The converted one-slide deck retains editable groups,
      all three fades and exact delays in its exported animation timeline.
      Static rendering passed. See [opening animation](declarative-amsterdam-2026/opening-animation.md).
- [ ] Copy the prepared animated opening into the main deck and verify the
      26-second sequence in Slideshow. Import currently requires one starting
      click; try After previous in the Motion panel for automatic slide-entry
      playback, then recheck timing. The live deck remains unchanged.
- [x] Synchronize Sasha's native slide additions and ordering: preserve the DCE
      promise diagram and embedding edits, renumber every physical slide,
      reconcile timing and documentation, and verify the rendered deck.
      Reviewed 2026-10-11: 30 slides, 26 timed plus four optional references;
      all rendered pages checked. The 60-minute run and six-minute Q&A remain.
      See the [native order](declarative-amsterdam-2026/native-slide-order.md).
- [x] Develop the opening thought → DNA → CEM metaphor in the user-edited
      first slide, add speaker talking points, and verify its native rendering.
      Saved [opening idea and notes](declarative-amsterdam-2026/opening-teaser.md).
      Used canonical CEM light colors, kept slide 2 dark, and retained the
      60-minute slot with 25 main slides plus four reference slides.
- [ ] Use the presentation plan to map the manifesto and platform pitch into
      the presentation website and slides, following the current presentation
      minutes. Use familiar side-by-side representations and visible outcomes;
      prioritize XML/XSLT developers, web UI developers and platform/integration
      owners. Keep the published activities and 60-minute slot; use broader
      pipeline and security examples as companions.

- [ ] Verify the selected exercise against current CEM implementation and
      resolve remaining delivery requirements. Published title, slot, audience,
      activities, and working minutes are recorded in the presentation document.
      Gmail history, session-specific submission requirements, participant
      releases, and the complete demonstration path remain unresolved.
- [x] Develop the timed narrative and slide outline around one document's
      validation, inspection, source-linked diagnostics, conversion,
      query/transformation and light-DOM rendering. Connect the exercise to the
      wider CEM platform and distinguish implemented capabilities from plans.
      Initial outline completed October 10; revised October 11 into the current
      minutes, preserving public commitments with adoption-focused framing.
- [ ] Prepare a reproducible tutorial and presenter demo with source files,
      exact commands, expected results, an intentional error and repair, and a
      presenter-led fallback. Reuse current examples and native capabilities;
      add an explicit fixture action here before any new fixture work.
- [ ] Produce the presentation, speaker notes, diagrams and participant setup
      instructions. Check the proposed Node/Linux installation paths against
      actual available releases, and record requirements for any other platforms
      included in the session. Prepare resource entries for discussion #24;
      add verified links as materials become available. Use that stable URL
      throughout the deck; Studio remains a future participant route.
- [ ] Rehearse the timed walkthrough and verify every shown command, example,
      output and demo path. Check slide readability and the fallback flow;
      record the tested versions and remaining limitations with the materials.
- [ ] Finalize the presentation and supporting files, record their locations
      and verification in the handoff, and archive completed presentation actions.
      Then continue with **CEM Studio** below. An outline alone does not complete
      this set.

## CEM Studio delivery planning

Studio/Site migration is complete above. Keep shared component functionality ahead
of app composition; presentation preparation remains deferred.

- [x] Review CEM Studio's current implementation against the
      [Studio contract](cem-studio.md) and
      [package status](../packages/cem-studio/README.md). Define its next delivery
      milestone and add the remaining implementation and verification steps here.
  - Reviewed 2026-10-12: repository, search, import/export, command Apply and
    file-provider services exist, but startup/workbench descriptors remain
    Feature Tour-bound. Import changes the provider controller's project without
    opening it in the workbench. At review, the shared host bridge lacked tree activation
    value delivery. These are source-review findings, not new runtime test results.
    Definition checks: milestone links/anchors and changed-file whitespace pass;
    implementation and browser acceptance remain open below.

### Next milestone: user-owned project workspace

Deliver the [bounded project lifecycle](cem-studio.md#next-delivery-milestone-user-owned-project-workspace)
in dependency order. Each fixture step precedes its implementation; existing
Feature Tour, revision, native-command and offline evidence remains mandatory.

- [x] **Shared event-to-command bridge first.**
  - [x] Fixture: bind `cem-tree-activate` declaratively to a host service and
        deliver only its validated string value from the exact component owner.
        Cover keyboard/pointer activation, malformed detail, undeclared events,
        nested views/owners, disabled/cancelled events, revocation and disposal;
        retain native action gesture-stack and form/close regressions.
  - [x] Implement the bounded bridge in `cem-elements`, document its scalar
        request/trust contract, and pass focused shared-runtime browser stories.
        Do not add a Studio tree event listener or arbitrary event-detail handoff.
  - Evidence (2026-10-12): both new canonical-tree worker/fallback fixtures first
    failed on missing delivery, then passed with the shared bridge. The combined
    host-view, navigation-content, form-control and canonical tree suite passes
    **19 Chromium plays**, including native pointer/Enter/Space, focus without
    selection, exact owner isolation, malformed payload rejection, capture-time
    disposal, synchronous native action delivery and truthful custom-event trust.
    `cem-elements:build` and `verify-package` pass (276 packed files, clean import);
    `cem-elements:lint` passes with 15 existing warnings outside the edited files.
    Component `verify-tree-forced-colors` and `verify-gallery-navigation` pass,
    including source/isolated-package properties, bundles and five-theme galleries.
  - CEM review: activation remains a semantic request; shared native owners retain
    hierarchy, keyboard/focus and application-owned selection. The bridge permits
    one documented scalar event, adds no styling/token defaults or visual variants,
    and preserves existing tree theme/forced-color behavior. Shared guarded-dialog
    workflow evidence is recorded below; broader device/browser/AT acceptance remains open.
- [x] **Prove shared project-navigation controls before app adoption.**
  - [x] Fixture: reconciliation must preserve an explicit surface focus return
        and never rebind a removed control's render identity to another produced
        instance. Fix focus-snapshot ownership in the shared DOM projection.
  - [x] Fixture: removing a conditional dialog returns focus to its retained
        entry destination only when it still owned focus; cover explicit opt-out,
        removed destinations and focus moved elsewhere. Keep this lifecycle in
        the shared native-surface capability.
  - [x] Fixture: settling a mounted view must include descendant renders caused
        by ancestor payload updates, so retained tree selection and editor values
        are current before host callers continue. Fix the shared settlement order.
  - [x] Fixture: library workflows for list/action selection, tree expand versus
        leaf activation, admitted versus cancelled selection, Save/Discard/Cancel
        dialog focus return, retained tabs/panes and empty/loading/error states.
        Keep all reusable interaction in shared runtime capabilities.
  - [x] Close any discovered control gaps in `cem-elements` and canonical
        `cem-components` declarations first. Update contracts, colocated plays,
        property/full galleries, source/packed delivery and Studio's control
        classification; retain the pinned Material subset boundaries.
  - [x] Verify semantic intents, five themes, forced colors, inherited token
        overrides, compact layout and keyboard ownership without app CSS patches.
  - Evidence (2026-10-12): the shipped
    [project-navigation declaration](../packages/cem-components/playgrounds/project-navigation.xhtml)
    and [contract](../packages/cem-components/docs/project-navigation-contract.md)
    compose passive lists/actions, a named resource region, tree, retained tabs,
    textarea and conditional modal decisions. Worker/fallback plays cover dirty
    navigation, delayed/failed Save, Discard, Cancel, native Escape, focus without
    selection, retained caret and explicit empty/loading/error states.
  - Shared fixes: mounted-view settlement now waits for ancestors before their
    descendants; native-surface v6 retains a source-free dialog's focused entry
    and restores eligible conditional removal; DOM focus reconciliation respects
    an explicit transfer and cannot borrow another produced owner's control ID.
    Focus opt-out, missing destinations, moved focus and same-owner replacement
    have direct regression evidence. A named resource region isolates transient
    status content from the retained workbench. No Studio UI behavior was added.
  - Verification: **41 distinct Chromium plays** pass across host-view,
    navigation-content, native-surface, projection focus, DOM merge, form-control
    and canonical tree/dialog/tabs suites; **19 declaration-shape unit tests**
    pass, including the corrected existing progress-status capability inventory.
    `cem-elements:build`, `verify-package` (276 files, clean import) and `lint`
    pass; lint retains 15 existing warnings and adds none. Component declarative,
    catalog/state-matrix prerequisites and `verify-gallery-navigation` pass from
    source and isolated package archives, including the exported workflow,
    properties, bundles and five-theme galleries. Studio UI classification passes
    for all 23 rows; relevant rows now link this shared evidence.
  - CEM review: Save/Discard/Cancel retain primary/destructive/contextual intent;
    the composition adds no CSS, token defaults or variant families. Its native
    controls own keyboard/focus while scalar services own admission. Both delivery
    paths verify a 360px viewport, retained selection, inherited spacing and focus
    token overrides, all five themes and system-color focus under forced colors
    with reduced motion. The pinned Material subset boundaries are unchanged.
    Production project transactions, stale completions and broader device/browser/AT
    acceptance remain in their following checklist items. Presentation work stays
    deferred with its existing resume context preserved.
- [x] **Generalize the project session and workbench.**
  - [x] First deliver the session admission service before navigation wiring.
        Fixture: independent source Save/Discard/Cancel and command
        Apply/Discard/Cancel, both dirty, explicit replacement confirmation,
        persistence conflict/failure, editing during Save/Apply, duplicate requests,
        cancelled/failed candidate opening and late results. Defer discards until
        selection commits; saving must not depend on native validation succeeding.
        Keep each candidate workbench isolated until its durable identity is admitted.
        Include delayed native previews/projections so background results cannot
        replace a newer command draft or an applied baseline, and source persistence
        stays independent of native preview or validation failure; recheck and
        Apply the retained command after native preview recovers; recheck admission
        when both drafts are reverted while a decision is pending.
        Verify graph-emitted and isolated-package session exports, including an
        empty session without UI side effects; reject mismatched project/page/resource
        candidates and allow retry after an initial open failure.
  - [x] Refresh Studio's dependency fixture for the migrated shared library:
        distinguish published dependency routes from workspace demo/story routes,
        assert the exact CLI and component-gallery runtime paths, and retain the
        isolated consumer's single installed runtime check. Pack the gallery's
        demo dependency with Yarn's normal workspace-protocol conversion. Refresh
        the import-map CSP hash for the added session export.
        Fixture: required installed files must match the verified deployment/types;
        include emitted build outputs directly in the package cache inputs so a
        source change cannot reuse a stale tarball.
  - Admission slice evidence (2026-10-12): `createCemStudioProjectSession` is
    exported from the root and `/project-session`, with typed snapshots and
    transition decisions. It admits isolated workbenches only after exact
    identity checks and independent draft decisions. Discard consent is deferred
    until successful admission; failed/cancelled opens and newer edits retain
    drafts. Cancelled transactions may finish durably but cannot navigate.
    `saveSource()` separates persistence from native validation, Apply establishes
    its committed command baseline, and late previews/projections retain newer
    edits. The contract is in `docs/cem-studio.md#project-session-admission-service`.
  - Verification: `@epa-wg/cem-studio:verify:consumer` and its prerequisite gates
    pass. All 33 workbench Chromium cases pass (18 admission cases), alongside
    repository, provider, native Feature Tour, preview and shell regressions;
    lint, typecheck and all 23 UI classifications pass. Deployment verifies 169
    declared assets / 259 files, repeat-build determinism and native worker/WASM
    operation online and offline. The isolated consumer verifies both session
    exports, an empty session, current archive bytes/types and one installed
    runtime. The dependency audit asserts exactly three published and nine
    workspace routes. A failing archive-byte fixture exposed and verified the
    fix for the package target's missing direct build-output cache dependency.
  - CEM review: this slice adds service orchestration only, with no visible DOM,
    UI listeners, CSS, token defaults or variation families. Save/Apply,
    Discard/Cancel, conflict and pending states remain distinct scalar data for
    shared CEM controls. Emitted shell/workbench checks pass all five themes,
    inherited token overrides, forced colors/reduced motion and offline reload.
    This initial service slice preceded the resolver/dialog integration recorded
    below. Physical device/AT acceptance and presentation work retain their
    existing deferred context.
  - [x] Fixture: two non-seed projects, nested entries, empty project, missing
        selection, supported/unsupported resources and a command-created page;
        resolve project-owned commands/dependencies without a seed catalog.
        Cover native command-created page reopening/execution, input identity
        independent of resource order, exact binary downloads, and deployed
        project selection with optional Feature Tour expectation metadata.
  - [x] Shared select prerequisite: a changed nonempty `value-request` token
        reapplies the authored value even when it is unchanged. Fixture: preserve
        selection without a new token, reset without synthetic events, retain
        the focused control and cancel stale popup preview. Advance the shared
        capability identity with this contract. Studio uses it to
        restore admitted selection after rejected navigation; keep a stable
        CEM stack around conditional guard surfaces.
  - [x] Integrate the admission service with project-owned workbenches and shared
        native dialogs. Fixture: source Save/Discard/Cancel and command
        Apply/Discard/Cancel, both drafts dirty, cancelled dialog, failed save,
        stale Apply, duplicate requests and late completions during navigation.
        Keep drafts on failure; commit selection only after guarded admission.
        Exercise mounted native dialog Save/Apply failures, successful persistence,
        Escape focus return, and cancellation while a candidate opens.
  - [x] Select project/page/resource by durable identity, retain exact revision
        and stale-result authority, and adapt Feature Tour metadata as optional
        sample data. Implement draft/transition guards before wiring navigation;
        keep selected identity/preferences outside portable exports.
  - Resolver/navigation evidence (2026-10-12): project workbenches load durable
    page/resource identities, exact revisions and declared dependencies without
    requiring a seed catalog. Native CLI command resources are decoded through
    the shared grammar, select their input independently of resource order and
    execute exact persisted bytes. Empty and read-only resources remain selectable;
    original binary bytes are downloadable. Optional sample expectations and
    legacy portable scenarios retain Feature Tour compatibility.
    Startup now uses the guarded session adapter and project-record selector.
    Source and command decisions, incompatible replacement, failure, cancellation
    and late candidate disposal preserve the admitted workbench. Nested page labels
    retain parent context; refreshed records expose newly created command pages.
  - Shared prerequisite: `choice-select` v2 adds explicit `value-request` replay
    without synthetic input/change events or stale popup commitment. Studio's
    stable `cem-stack` composition preserves the launcher while conditional
    dialogs appear, and cancellation restores both admitted value and focus.
    Replacement has one modal owner during guarded Apply.
  - Verification: all 40 Studio workbench Chromium cases pass, including seven
    project-record/mounted-navigation cases; all 13 select stories and 19
    declaration/capability unit cases pass. Native deployment checks pass all 32
    schema-package examples, portable operations and online/offline recovery.
    `@epa-wg/cem-studio:verify:consumer` passes all 48 prerequisite/consumer
    tasks: lint/typecheck, all 23 UI classifications, 169 declared deployment
    assets / 259 files, deterministic builds, exact dependency routes, current
    installed archive bytes/types and a single installed runtime. The consumer
    verifies both new workbench factories as well as the session exports.
    Explicit Parse prepares the command editor for enabled-state theme checks;
    no load-time command is inferred to satisfy an old fixture.
  - CEM review: shared select/dialog/action/editor/status controls own interaction
    and presentation. Studio adds repository/session orchestration and CEM-ML
    composition, with no application UI listeners, styling, token defaults or
    parallel variation families. Five themes, inherited token overrides, system
    colors, reduced motion and keyboard/focus behavior pass deployed checks.
    Home, retained explorer/search panes, local selection recovery and full
    provider coordination remain below; broader device/AT acceptance and the
    deferred presentation resume context are unchanged.
- [x] **Deliver Home, project creation and import.**
  - [x] Fixture: deployed bundled starter passes native validation and exports one
        source resource; empty Home and unavailable storage remain readable.
  - [x] Fixture: mounted shared Home controls create/list/open projects through draft
        guards; cancelled pickers, duplicate archives, invalid names and failed writes
        retain the active draft and repository contents.
  - [x] Fixture: list/open existing projects, create from a validated bundled
        starter with a fresh project identity, and import a distinct archive;
        duplicate identities, invalid input and quota failure leave existing data
        unchanged. Confirm that an imported project becomes selectable/openable.
  - [x] Compose Home from shared fields, lists, actions and status controls;
        use existing validated repository transactions and native format boundaries.
        Present empty/storage/error states and preserve existing user copies at boot.
  - Implemented: persistent declarative Home, repository-owned project choices,
    UUID starter creation, create-only archive import, explicit refresh and guarded
    Open project. Successful creation/import leaves the current draft in place.
    Browser picker cancellation and file-read failures settle without hanging.
  - CEM review: existing shared card, select, text-field, action and alert controls
    own semantics and styling; no app UI listeners, local CSS, private token defaults
    or new behavior were needed. Deployed theme/inherited-override, forced-color
    and offline checks pass. Broader device/AT acceptance remains below.
  - Fixtures: 42 Chromium workbench cases pass; mounted Home covers empty/storage
    states, create/import/open, duplicate/invalid/quota failures and cancellation.
    The deployed native starter exports one source and no inferred command pages.
  - Final evidence: `yarn nx run @epa-wg/cem-studio:verify:consumer` passes all
    48 prerequisite/consumer tasks, including lint/types, all UI classifications,
    deterministic 259-file output, native worker/online/offline regressions and a
    clean seven-package consumer with one runtime copy. `git diff --check` passes.
- [x] **Deliver explorer, search and retained workbench panes.**
  - [x] Fixture: shared tab host events deliver only validated owner values; repeated
        diagnostic/result navigation reveals the retained source pane before applying
        its caret/focus request, without leaking nested tab events.
  - [x] Shared hash fixture: version binary-bearing content addresses, digest actual
        repeated artifact bytes once per call, preserve nonbinary v1 addresses,
        detect byte mutations and make cloned/shared buffers content-equivalent.
  - [x] Shared scale fixture: render a 128-resource tree without serialization
        failure; preserve every identity and expansion/selection semantics.
  - [x] Shared prerequisite fixture: bind a host service to an authored declarative
        child without replacing it; retain identity/caret through parent renders,
        reject foreign/wrong-tag/already-owned targets and revoke disposed commands.
        Prove worker/fallback behavior before Studio retained-pane adoption.
  - [x] Fixture: mounted recursive tree uses stable whitespace-safe keys, separates
        parent expansion/focus from admission, retains committed selection on Cancel,
        and preserves source/command nodes and caret across retained pane switches.
  - [x] Fixture: bounded deterministic repository search admits exact result identities,
        rejects stale completions, reports empty/error states, and refreshes newly
        applied command pages without losing drafts. Verify compact/expanded viewports.
  - [x] Fixture: nested keyboard navigation, stable opaque entity keys,
        parent expansion without navigation, focus without selection, deterministic
        search/result activation, and caret/selection retention across pane changes.
  - [x] Compose canonical tree, local-view tabs and existing editor/result views;
        bind all navigation through the project session. Reuse the bounded search
        service and project-owned command Apply/current/existing/new page paths.
        Initial compact and expanded layouts both use retained local-view panes.
  - Implemented: recursive repository explorer, stable encoded record keys, guarded
    tree/search admission, deterministic 50-result search with stale-completion
    rejection, and navigation refresh after durable workbench Save/Apply. Shared
    tabs retain Source/command and Results at compact and expanded widths;
    diagnostic/provenance actions reveal Source before requesting caret/focus.
  - Shared prerequisites: host views bind authored direct children with revocable
    service ownership; validated own `cem-tab` values cross the host bridge.
    Binary-bearing content addresses use `stable-json-fnv1a64-binary-v2`, hashing
    actual artifact bytes once per buffer per call. Nonbinary v1 keys are unchanged;
    old binary v1 cache entries require fresh publication, not silent reinterpretation.
    Native typed artifacts and their validation/authority remain intact.
  - Layer evidence: 45 Studio Chromium workbench cases, 8 worker/fallback host-view
    stories, 9 shared tree stories (including 128-resource expansion), 5 projection/
    content-address unit cases, 49 processing engine/host unit cases and 58 edge/SSR
    unit cases pass. Shared lint passes with 15 existing warnings and no new warnings.
  - CEM review: canonical tree/content and tabs/navigation semantics are preserved,
    including transparent resting tree paint, independent focus/selection, retained
    node identity and service-admitted navigation. No application CSS, private token
    defaults or application UI listeners were added. Deployed checks cover all five
    themes, inherited tree/control overrides, forced colors/reduced motion, retained
    editor/caret at 390px and 1280px, and offline reload. Broader device/AT acceptance
    and resizable simultaneous panes remain separately open.
  - Final delivery evidence: `yarn nx run @epa-wg/cem-studio:verify:consumer`
    passes all 48 tasks, including lint/types, 23 UI classifications, deterministic
    170-asset/260-file output, native worker/Feature Tour regressions and a clean
    seven-package consumer with one runtime copy. `git diff --check` passes.
- [x] **Finish recovery and active-project provider coordination.**
  - [x] Fixture: recover only admitted identities; missing/trashed records fall back,
        cancelled navigation keeps the committed identity and exact saved bytes.
  - [x] Fixture: repository invalidation refreshes explorer/search/Trash without
        replacing drafts; stale revisions fail Save and restore preserves bytes.
  - [x] Fixture: provider status and picker completion cannot retarget a changed
        active project; cancellation/denial retain archive fallback access.
  - [x] Fixture: update activation blocks unresolved source/command drafts,
        navigation/provider work and edits made during asynchronous persistence.
  - [x] Fixture: exact bytes and last committed selection after offline reload,
        multi-tab revision conflict, missing/trashed selection recovery,
        export/trash/restore and search invalidation, active provider target,
        picker cancellation/denial, archive fallback and safe-update persistence.
  - [x] Coordinate the session, shell/update and provider services; display
        conflicts without overwriting drafts, keep browser permissions explicit,
        and expose durable-save versus validation outcomes separately. Explicit
        Save/Apply is this slice's durability boundary; autosave remains later work.
  - Admission stores only local project/page/resource identity; reload reopens
    IndexedDB bytes. Missing/trashed identities fall back with a recovery alert.
    Repository notifications refresh navigation/search without replacing drafts;
    Home Trash/Restore remains reversible and revision-checked. Trashed resource
    writes are rejected until restore; stale saves retain their draft and conflict.
  - Provider actions capture the active target; late picker/status results cannot
    retarget the session. Cancellation and denial retain explicit archive fallback.
    Update activation waits for Home/navigation/provider work and both draft kinds;
    Save/Apply or reset must resolve drafts before activation. No implicit command
    Apply or draft autosave was added.
  - Build-order fix: shell delivery now depends on determinism, just like the
    full worker delivery check, so it cannot read output during a clean rebuild.
  - Evidence: 46 workbench and 9 shell browser cases pass, including actual Home
    Trash/Restore actions, multi-instance change delivery, retained drafts, revision
    conflicts, byte-exact export/recovery and stale provider completion. Deployed
    output reopens a non-default second project offline with the matching provider
    target, then preserves native Feature Tour/command regressions.
  - CEM audit: shared select/action/alert semantics, destructive Trash intent and
    named status text; no app-owned UI listeners, CSS or private theme defaults.
    Emitted checks retain five themes, inherited overrides, forced colors,
    reduced motion, keyboard behavior and compact/expanded layouts. Complete
    two-project UI lifecycle and physical browser/device/AT acceptance remain open
    below/above; draft autosave and structural editing remain subsequent work.
  - Final verification: `yarn nx run @epa-wg/cem-studio:verify:consumer` passes
    all 48 tasks (41 cached), including lint/types, 23 UI classifications,
    deterministic 170-asset/260-file output and the isolated seven-package consumer
    with one runtime copy. Final output hash:
    `3f189ae4094f0c9dd7e7ed3afb06f5c78db8c5877d41f52225ae72924b014bab`.
    `git diff --check` passes.
- [x] **Close deployed acceptance and CEM principle review.**
  - [x] Fixture: drive two user-owned projects through native CEM controls for
        create/import, source save, command-page Apply/run, guarded navigation,
        export, Trash/Restore and offline reopen with exact persisted bytes.
  - [x] Fixture: constrain shared `cem-select` long resource labels to a narrow
        container without losing accessible names, option access or return focus.
  - [x] Fixture: inspect the emitted Chromium accessibility tree, unique IDREFs,
        required control names, modal keyboard containment/return, retained caret,
        visible keyboard focus and 320/390/1280px reflow; retain five-theme,
        inherited-override, forced-color and reduced-motion delivery checks.
  - [x] Fixture: run the complete two-project lifecycle from graph-emitted Studio
        output online and offline, including command-created pages and retained
        input/focus; preserve Feature Tour native-command/artifact regressions.
  - [x] Include all new declarations/assets in the deployment graph, verify the
        isolated package consumer, run focused layer checks and Studio `check`,
        then record exact evidence and limits. Complete the semantic/token,
        five-theme/forced-color, keyboard and compact-layout audit before closure.
  - Deployed evidence: the dedicated `verify:workspace-delivery` gate drives the
    real file picker/download and CEM controls across two user-owned projects.
    Source Save/validation, native command-page Apply/search/run, cancelled and
    discarded navigation, exact UTF-8 backup, Trash/Restore, offline Save/Apply/run
    and reload all pass. The first project's saved bytes remain unchanged.
  - Chromium 148.0.7778.96 on Linux exposes 46 named interactive controls with
    unique IDs and valid label/help/control references. Modal Tab may visit browser
    chrome but never the inert background; Escape restores the select launcher
    while editor identity, draft and caret remain intact. Keyboard zebra focus
    is visible; page width equals 320/390/1280px at each tested viewport.
  - Shared fix: `cem-select` grid/control minimum sizing now permits its existing
    ellipsis to contain long selected resource labels. The new 240px fixture first
    reproduced 799px overflow, then passes with all 14 select stories, preserving
    option access, accessible naming and return focus. No application CSS or UI
    behavior workaround, theme defaults or parallel tokens were introduced.
  - CEM review: semantic action/status roles, bounded shared-control geometry,
    public token ownership and inherited theme behavior remain intact. Five-theme,
    inherited-override, forced-color and reduced-motion delivery gates remain in
    the aggregate check. `workspace-delivery.json` records the browser and scope;
    release evidence requires and references its lifecycle/accessibility results.
  - Limit: this closes automated Chromium deployed acceptance only. It does not
    claim WCAG certification, physical touch/pen/IME or spoken screen-reader
    acceptance; the independent device/browser/AT checklists stay open above.
  - Final evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (32 cached), including 46 workbench cases, 9 shell cases, 32 native Feature Tour
    samples, 23 UI classifications, deterministic 170-asset/260-file output,
    clean seven-package consumer with one runtime copy and release evidence.
    The separate shared select suite passes all 14 stories. Output hash:
    `08cf4e2d5ccecd14e86d576b84c1f2dcd2032c7691e2b457cc589f4f56aba667`.
    `git diff --check` passes. Draft recovery/autosave is the next implementation
    milestone; structural editing follows. Presentation context remains deferred.

- [x] **Local draft autosave and recovery** — journal source and command text by
  admitted project/page/resource identity; never Save, Apply, or Run implicitly.
  - [x] Add browser fixtures for reload recovery, exact Unicode/empty drafts,
    baseline conflicts, corrupt/quota storage, discarded navigation, cancelled
    admission, and independent project journals; verify no repository mutation.
  - [x] Provide shared CEM alert/action recovery status, backup download and
    explicit removal of blocked recovery records; retain failed writes in memory.
  - [x] Verify deployed reload recovery and conflict controls, Studio checks and
    CEM principle review.

  - Evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (35 cached), including 51 workbench cases and deployed offline recovery,
    backup download and conflict clearing. Release evidence requires
    `workspace-delivery.json`'s `draftRecovery` flag. Clean seven-package consumer
    retains one runtime copy; 170 assets / 260 files remain deterministic.
    SHA-256: `6f0d6e559bbec8773f27236d75eaa6e71ddac4bd1d8349968c2c8b2d2782eb2d`.
  - CEM review: existing declarative alerts/actions express status and destructive
    clearing with accessible names and native focus. No app CSS, tokens, DOM
    listeners or component-local behavior were added. Inherited theme, five-theme,
    forced-color, reduced-motion and deployed reflow checks pass in the aggregate.
  - Bounded contract: immediate local recovery is separate from project Save/Apply
    and exports; records are capped at 1,000,000 UTF-16 code units. Storage failures
    expose current in-memory backups. Conflicts do not merge automatically;
    simultaneous cross-tab writes are not transactional. Pending targets without
    command-text changes and backup import/merge remain outside this slice.
    Physical device/spoken AT acceptance and deferred presentation context remain
    independently open. `git diff --check` passes.

- [x] **Structural project editing and undo** — native-validated, revision-checked
  project/page rename, subproject creation, subtree copy/move, sibling reorder,
  resource creation/copy; bounded session undo/redo without implicit Save/Apply.
  - [x] Add repository browser fixtures for order/export round trips, copy byte
    identity, hierarchy rejection, stale revisions, Trash and monotonic undo.
  - [x] Add workbench fixtures for clean/dirty admission, history isolation and
    divergent revisions; add deployed shared-control editing/undo acceptance.
  - [x] Extend shared host-view fixtures with a field named `id`; read form
    identity through attributes to prevent native named-control clobbering.
  - [x] Verify aggregate Studio gates and record CEM principle review and limits.

  - Evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (41 cached), including 53 workbench cases, 15 repository cases and the deployed
    offline structure/edit/undo/redo/reload scenario. All 8 shared host-view stories
    pass with an `id`-named field in worker and fallback paths. Release evidence
    now requires `workspace-delivery.json`'s `structuralEditing` flag.
  - The new replace path exposed an existing IndexedDB key-cursor deletion bug;
    replacement now uses value cursors and rejects cursor callback failures.
    Form identity capture reads the attribute rather than a clobberable property.
  - Ordered native manifest arrays survive IndexedDB through private order
    metadata; portable exports gain no schema fields. Command Apply retains order.
    Undo/redo is capped at 20 snapshots / 8 MiB per project per session; divergent
    revisions disable replay. In-flight typing remains behind guarded admission.
  - CEM review: shared cards/fields/selects/textareas/alerts/actions own semantics,
    geometry, names and focus; no app CSS, public token overrides or UI listeners
    were introduced. Five-theme, inherited overrides, forced colors, reduced motion
    and deployed 320/390/1280px reflow gates pass. Clean seven-package consumer
    retains one runtime copy. Deterministic 170-asset/260-file SHA-256:
    `66cc9d77108471111a06b8ceeb443c03ee1f0c3317b8e5d9faf82508317e894a`.
    `git diff --check` passes. Physical-device/spoken AT acceptance and deferred
    presentation context remain open. Project resource rename now preserves native
    identity and resolver addresses; no authored-reference rebinding is required.

- [x] **Typed operation forms** — shared controls for native-metadata scalar,
  enum and boolean options on parse/inspect/convert/query/transform/trace drafts.
  Preserve unedited arguments and explicit Apply/Run; reject stale field submits.
  - [x] Add browser fixtures for metadata choices/defaults, literal quoting,
    unchanged repeated options, invalid choices, stale updates, no execution and
    packaged helper exports.
  - [x] Add shared tabs narrow-container coverage for wrapping labels without
    page overflow, retaining accessible selection and keyboard behavior.
  - [x] Verify deployed bounded option selection (one active editor), draft editing and explicit Apply/Run, plus
    aggregate Studio gates and CEM theme/accessibility review.

  - Evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (33 cached), including 55 workbench cases, deployed option selection/Set/Reset,
    unchanged project revision before Apply, explicit Run and offline lifecycle.
    All 7 shared tabs stories pass. Release evidence requires `typedOperationForms`;
    clean seven-package consumption verifies the public helpers and one runtime copy.
  - One option editor is rendered at a time. Rendering every option concurrently
    exceeded the shared scheduler's bounded queue and left result tables stale;
    bounded declarative composition fixes that without changing scheduler limits
    or weakening the original settled trace/graph row assertions.
  - CEM review: native metadata owns grammar/choices/defaults; shared CEM controls
    own names, focus, geometry and theme styling. UI selection is a declaration-local
    slice. Shared tabs wrap at narrow widths while preserving token-derived targets
    and keyboard order. No app CSS, token defaults or UI listeners were added.
    Inherited overrides, five-theme, forced-color, reduced-motion and deployed
    320/390/1280px reflow gates pass. Deterministic 170-asset/260-file SHA-256:
    `9c94c66eea46f1c6d5bb9e0be99c4575d6ae74788e64171b06195ee217c74149`.
  - Limits: forms edit prepared scalar/enum/boolean options on six operations;
    coupled/repeated options remain in the literal editor. Switching options drops
    unsubmitted field input; Set enters the guarded/recoverable command draft.
    Existing Apply support remains parse/inspect. Physical-device/spoken AT
    acceptance and deferred presentation context remain open. `git diff --check` passes.

- [x] Retire path-rebinding and browser paint-proof prerequisites for Studio
  tree operations. Project resources retain their stable IDs and resolver addresses;
  names and paths are dataset metadata. See the identity correction checklist above.
- [ ] **Grouped operation and graph authoring** — extend native-backed configuration
  editing and persistence, using shared CEM controls before app composition.
  - [x] Shared form-group foundation: ship declarative `cem-fieldset` before Studio adoption.
    - [x] Fixture: verify native legend/group naming, label/help projection, retained controls and unique description references; test fieldset disabling, native submission/reset and keyboard behavior with shared form controls, rendering the group in worker/fallback execution.
    - [x] Implement the shared XHTML declaration with inherited semantic spacing, typography and palette tokens; cover all five themes, inherited overrides and narrow/RTL layout in colocated stories, plus forced colors in source/package browser checks.
    - [x] Add property/full galleries, a bundled form-group example, catalog/index/accessibility documentation and source/isolated-package verification.
    - [x] Adopt it in Studio's existing operation-option form, retain command-view slice ownership and existing host services, and verify typed Set/Reset, stale guards and declaration-only UI composition.
    - Shared evidence: six colocated fieldset stories pass; the fieldset/field/select/action
      regression cohort passes 48 plays. Source/built import checks pass six pages;
      `verify-cem-component-playgrounds.mjs --fieldset-only` passes the property page,
      gallery and bundle from repository source and isolated package archives, including
      template metadata, original resource bases and duplicate style/registration checks.
      Build, package verification (304 files), declarative/state/catalog gates, Figma
      inventory (54 primitives, external reviews still planned), and Studio's 23 UI
      classifications pass. Component lint reports zero errors and 136 warnings;
      changed JavaScript/tooling lint is clean.
    - Studio evidence: `@epa-wg/cem-studio:check --parallel=1` passes all 51 tasks,
      including 95 browser tests (57 workbench), 32 native/deployed Feature Tour
      cases, deterministic 171-asset/261-file deployment, clean consumer/package
      checks, online/offline worker execution, two-project lifecycle, recovery and
      synchronized release evidence. The new mounted test proves native group help,
      pending disable, duplicate-activation prevention, retained group identity and
      typed Set/Reset without persistence or implicit Run. Two existing navigation
      cases timed out during concurrent browser verification; serial execution
      passed without increasing timeouts or changing assertions.
    - CEM audit: neutral comfort palette, boundary stroke, surface bend, container inset,
      related/group gaps and UI typography consume canonical D0/D1/D3/D5/D6 endpoints
      without local defaults or new token families. Inherited gap/inset/bend/font-size
      overrides (27px/23px/7px/19px), all five themes, narrow 300px groups, 360px pages,
      LTR/RTL and forced Canvas/CanvasText paint pass. Native legend/help references,
      retained values/focus/caret, first-legend exemption, nested disabling and explicit
      child state pass; nested controls retain interaction and focus ownership.
      No new production JavaScript behavior or runtime capability is needed.
    - Verification limits: physical-device and spoken AT acceptance remain open.
      The existing global `verify-cem-components-styles.mjs` audit stops on an XML-escaped
      nested `&amp;` selector in unchanged `cem-icon-button.xhtml`; its extractor/parser
      needs maintenance independently of this fieldset's browser theme evidence.
  - [x] Add native-backed atomic grouped option editing and persistence without implicit Apply/Run; retain unchanged arguments, stable resource identities and stale revision guards.
    - [x] Fixture: validate a whole option batch once, including coupled alternative replacement, literal/repeated-value retention, invalid/duplicate/unsupported edits and all-Keep no-ops.
    - [x] Fixture: reject stale/foreign or pending batches, retain the previous draft after preview failure, ignore superseded asynchronous results, and persist grouped changes only through explicit Apply and draft recovery.
    - [x] Compose bounded grouped editors declaratively with shared fieldsets/selects/fields/actions, native Keep/Set/Reset form capture and one visible editor; verify retained staging, native reset and pending disabling.
    - [x] Verify deployed grouped Set, one-transaction Apply, reload/offline persistence, clean package exports, existing single-option behavior and CEM theme/accessibility contracts before delivery.
    - Evidence: `@epa-wg/cem-studio:check --parallel=1` passes all 51 tasks (35
      cached), including 103 browser tests / 65 workbench cases. Ten focused
      acceptance cases cover the new batch/group contracts and existing field
      guards. The real command service persists both changed options in one Apply,
      reopens them with stable resource identities and leaves original resource
      records/bytes untouched. Deployed source/package delivery proves grouped Set,
      no revision change before Apply, online/offline reopening and recovery.
      Release evidence requires `groupedOperationForms`; clean consumer checks
      verify both new public metadata/batch helpers and one runtime copy.
    - CEM audit: one active editor, at most three grouped options; fieldsets,
      selects, fields, textareas, stacks and actions own native semantics, geometry,
      focus and theme paint. Labels, grouping, Keep/Set/Reset controls and bindings
      are CEM-ML. No app CSS, token defaults, DOM listeners or new runtime capability
      were added; JavaScript changes are non-UI command validation/orchestration.
      Native schema groups/choices/defaults remain authoritative. Shared five-theme,
      inherited-token, forced-color and deployed 320/390/1280px accessibility/reflow
      gates pass. Lint and staged whitespace checks pass. Deterministic deployment
      remains 171 assets / 261 files; SHA-256:
      `2f95434dd547a030b74826eff0415e17caa53054b0b253222c54bee32bd6128b`.
    - Bounds: Keep-only batches preserve exact command text; other edits use the
      shared canonical serializer, which may normalize quoting/order and materialize
      defaults. Apply retains its parse/inspect bounds. Graph/configuration authoring,
      physical-device/spoken AT acceptance and global style-audit maintenance remain
      independently open. Generated screenshots and Vitest attachments are excluded
      from the source commit.
  - [ ] Add bounded graph/configuration authoring through shared declarative controls and native validation; verify persistence, undo/recovery and offline execution.
- [ ] Maintain the global component style audit: decode canonical XHTML CSS text
  and support retained native nesting before asserting action state selectors.
  Preserve the semantic token/state checks; the current unchanged icon-button
  declaration fails extraction on an XML-escaped `&amp;` selector.

Continue with grouped operation/graph authoring using native project resource
identities. No renderer instrumentation or dependency rewriting is a prerequisite.
Physical-device/browser/AT acceptance remains independently open above.
