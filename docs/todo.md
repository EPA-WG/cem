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
Studio/Site declarative UI migration is complete. All 53 CEM components are
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

Current inventory: **53 canonical / 0 legacy components / 0 legacy authored
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
    presentation context remain open. Resource path rename/delete require an
    explicit authored-reference rebinding contract; current copies retain references.

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

- [x] **Safe resource-path rename/delete for data and parse/inspect projects** —
  atomic, revision-checked edits with native project validation, command input
  rebinding, guarded navigation and bounded session undo/redo.
  - [x] Add repository fixtures for command rebinding, exact untouched bytes,
    referenced deletion, unsupported dependency blockers, stale revisions and undo;
    extend workbench fixtures to prove dirty drafts block both path operations.
  - [x] Add deployed shared-control rename/delete, run-after-rebind and undo coverage;
    verify aggregate Studio gates and record the bounded CEM contract.

  - Evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (30 cached), including 18 repository and 55 workbench cases. Deployed acceptance
    renames the active source, blocks referenced deletion, deletes/restores an unused
    resource, executes the rebound command and reopens offline. Release evidence now
    requires `resourcePathEditing`; clean consumer retains one runtime copy.
  - Stable resource IDs keep manifest/page references intact. Only matching authored
    command inputs are reserialized and rehashed through the shared CLI contract;
    other bytes remain exact. Undo restores paths and command bytes at new revisions.
    All resource commands, including orphans, participate in dependency checks.
  - Boundaries: data/expected leaf assets and native parse/inspect command resources
    only. Other page kinds, executable resource roles, command formats, relative or
    unresolved inputs, resolver/configuration/output options and non-input project
    URI options block path edits. Embedded document text/hyperlinks are not rewritten.
    No cascade deletion, implicit Save/Apply/Run or unbounded history was introduced.
  - CEM review: existing shared select/field/textarea/actions own interaction and
    geometry; deletion has explicit destructive intent. Alerts expose blockers and
    bounded history. No app CSS, UI listeners or new token defaults were added.
    Inherited themes, forced colors, reduced motion, accessible names/focus and
    320/390/1280px reflow pass. Deterministic 170-asset/260-file SHA-256:
    `d6e3a8892dc1e54e1d03555d3b093945592c925734f9eeb44ff3f38a74a28499`.
    `git diff --check` passes. Broader dependency and physical-device/AT coverage
    remain explicit follow-ups; presentation context remains preserved.

- [ ] **Native dependency rebinding for configured workbenches** — define native
  discovery/rewrite contracts for validation, query, transformation and graph
  configurations (including relative references and resolver/module maps), then
  extend path-edit admission with native fixtures before Studio adoption.
  - [x] Add native run-config planning fixtures for owned input/schema-package
    references and output input links; exact no-op bytes; identity preservation;
    relative/base, resolver/module-map, unknown-field and output blockers; malformed
    configuration and rename collisions. Implement the bounded planner before host adoption.
  - Native foundation delivered in `run_config_rebind.rs`: typed reference roles,
    field-pointer blockers and optional replacement bytes; admitted exact absolute
    input/schema-package URLs and output input links. Native validation before/after
    rejects malformed configurations, dangling links and URI collisions. Unknown
    fields/non-object records and unresolved dependency mechanisms block all rewrites.
    Format/namespace identities retain their values; no-op source bytes remain exact.
  - Evidence: Rust tests were added first; `cargo test -p cem-ml --lib run_config`
    passes all 55 tests, including 6 new planner cases. `git diff --check` passes.
    CEM review: semantic dependency ownership remains native, variation is explicitly
    bounded, and the planner does not fetch, execute or mutate projects. This layer
    introduces no UI, CSS or token changes; existing shared-control behavior is not
    broadened. Browser/theme gates were not rerun for this unconnected native API.
  - Limits: this is one config's plan, not project-wide authorization or a complete
    deletion inventory. Worker transport and Studio adoption remain pending. Relative
    inputs, scope bases, resolver/module maps, formatter pipelines, outputs and graph
    dependencies need execution-aligned contracts before admission can expand.
  - [x] Add native graph dependency fixtures for execution-aligned relative references,
    distinct read/template/map/output roles, transitive scan requirements, opaque
    imports, glob blockers and invalid graph diagnostics; implement native discovery.
  - Graph discovery delivered in `transform_graph_dependencies.rs`: native CEM-ML
    parsing, acquired graph base and executor-owned relative resolution; immediate
    references retain node/attribute ownership, authored spelling and separate
    input/template/module-map/output roles. Explicit follow-ups cover document,
    template/map inspection, output ownership and pattern expansion. Opaque imports
    are leaves. Invalid graphs return diagnostics without partial inventories.
  - Evidence: fixtures first, then `cargo test -p cem-ml --lib transform_graph`
    passes 31 cases, including 5 new discovery fixtures and the existing provider,
    module-map and execution cases. Discovery's input/output fixture compares
    directly against manifest-provider acquisition. `git diff --check` passes.
  - CEM review: native semantic ownership and bounded contracts are preserved;
    node links/format identities/parameters are not rewritten as resource URLs.
    No app UI behavior, styles or theme defaults were introduced. This remains a
    native planning API; browser/theme gates were not rerun and Studio admission
    remains unchanged. Transitive closure and source-preserving graph edits remain
    open; this immediate inventory cannot authorize project rename/deletion.
  - [x] Add native graph rewrite fixtures for exact surrounding bytes, Unicode,
    quote styles, relative inputs, multiple matches, no-op bytes, output collisions,
    transitive blockers and ambiguous source slots; implement bounded opaque-input
    rewrites using native token ranges and revalidate the resulting graph.
  - Opaque-input source rewriting delivered in `transform_graph_rebind.rs`:
    executor-resolved matches bind to native AST/tokenizer attribute ranges, preserve
    surrounding source bytes and quote style, and turn rewritten relative inputs
    into absolute URLs. Reverse-order patches avoid offset drift. Native reparsing
    requires graph equality except for selected input values. No-op/blocked plans
    return no replacement. Graph relocation, conflicting outputs, unknown/duplicate
    attributes and transitive/pattern dependencies remain blocked.
  - Evidence: Rust fixtures first; `cargo test -p cem-ml --lib transform_graph`
    passes 36 tests, including 5 new rewrite cases and acquisition using only the
    rebound resource URI. Formatting and `git diff --check` pass.
  - CEM review: edits follow native semantic ownership rather than text matching;
    bounded admission preserves original source and explicit execution. No UI,
    CSS or token changes. Browser/theme gates were not rerun for this native-only
    candidate planner. Full transitive scans, host project validation/revision
    checks, worker/Studio adoption and deployed acceptance remain open.
  - [x] Add native source module-map dependency fixtures for v1/v2/v3 paths,
    map-relative resolution, declared cross-asset edges/cycles, missing or ambiguous
    targets and invalid formats; reuse execution's parser for discovery.
  - Source module-map discovery now shares execution's extracted parser and
    inventories v1/v2/v3 imports/resources with map-relative URIs and validated
    content types. V3 declared edges retain specifier/asset ownership and resolve
    target URIs; cycles remain explicit. Missing/ambiguous/non-module edge targets,
    undeclared formats and unknown top-level fields fail closed. Destination maps
    remain a distinct publication contract.
  - Evidence: fixtures first; `cargo test -p cem-ml --lib transform_graph` passes
    39 tests, including 3 new module-map cases and existing graph execution/provider
    tests. The extracted execution parser body is unchanged apart from its parameter
    and indentation. `git diff --check` passes.
  - CEM review: dependency semantics stay native, with bounded format admission and
    no app UI behavior/CSS/token changes. This inventories declared edges only;
    asset content verification, relative/dynamic JavaScript dependencies and native
    document/template inspection remain open. Empty edges do not prove a leaf.
    Browser/theme gates were not rerun; worker/Studio admission remains pending.
  - [x] Add native asset-verification fixtures for declared bare module edges,
    exact-byte hashes, missing/invalid assets, unused/undeclared imports and explicit
    relative/dynamic/template follow-ups; reuse execution's native source checks.
  - V3 declared-edge source verification now reuses execution's lexical module
    checks against immutable supplied assets. Evidence includes exact map/asset
    hashes, byte lengths and declared edges. Missing assets, conflicting content
    types, invalid UTF-8 and undeclared/unused imports fail without partial evidence.
    Non-bare imports, dynamic calls, template literals and non-JavaScript contents
    remain explicit follow-ups. Candidate bytes are discarded; sources stay exact.
  - Evidence: fixtures first; `cargo test -p cem-ml --lib transform_graph` passes
    42 cases including 3 new source-verification fixtures. `git diff --check` passes.
  - CEM review: native semantic ownership and bounded verification remain intact;
    no UI, CSS or theme defaults were added. This verifies declared bare imports,
    not JavaScript syntax or full runtime loading. Empty follow-ups never authorize
    rename/delete. Document/template inspection, runtime-load policy, snapshot/revision
    checks and worker/Studio acceptance remain pending. Browser/theme gates were
    not rerun for this native-only API.
  - [x] Add native template dependency-inspection fixtures for transitive relative
    imports, resolver provenance/hashes, missing imports, cycles/depth limits and
    dynamic import declarations; reuse runtime preflight without executing bodies.
  - Native template dependency inspection now reuses recursive runtime preflight
    through the caller's resolver/policy. Results retain import provenance, hashes,
    alias scopes and diagnostics; caller depth caps only tighten existing limits.
    Missing/cyclic/over-depth imports and expression-valued declarations produce
    no partial preflight. No template body, query or output is executed.
  - Evidence: fixtures first; 3 `template_dependency_inspection` cases and all
    42 `template_module` regression cases pass via `cargo test -p cem-ml --lib`.
    `git diff --check` passes. The relative-import fixture uses a supplied in-memory
    resolver and checks nested parent ownership plus acquired URI/policy/hash fields.
  - CEM review: static import ownership stays in native execution preflight instead
    of app parsing. No UI, CSS or token changes. Every visited body remains explicitly
    listed for analysis, including templates without imports; data-dependent document
    loads and general document dependencies remain open. Snapshot-only hosts must
    supply a snapshot resolver. Studio admission and browser/theme gates remain
    pending; the inspection key does not authorize rendering or mutation.
  - [x] Add native XPath dependency fixtures for literal/computed document loads,
    nested calls, namespace identity, higher-order/extension blockers and bounded
    traversal; attach findings to template inspections without evaluating expressions.
  - Native XPath resource-use inspection now traverses retained typed syntax and
    attaches findings to root/imported template slots. Standard resource functions
    are namespace-aware; literal URI candidates retain source ranges and resolution
    blockers, while computed/higher-order/extension/unsupported forms remain blocked.
    Iterative traversal has a 10,000-node per-slot budget. No expression or candidate
    document is evaluated/acquired; every template body still requires analysis.
  - Evidence: 8 `template_dep` native cases pass, including 4 XPath fixtures and a
    template integration fixture proving an unavailable document is only reported.
    `git diff --check` passes. CEM review: native semantic ownership, bounded work and
    explicit unresolved states are preserved; no app UI/CSS/token changes were made.
    Browser/theme checks remain pending for host adoption. General documents and
    non-XPath CEM/runtime loads remain outside this static inspection contract.
  - [x] Add native wire-contract, real-worker and repository fixtures for bounded
    planning, unchanged bytes on blockers, referenced deletion and exact-byte undo.
  - [x] Add a native binary graph-publication regression fixture and preserve
    retained primary bytes instead of publishing their descriptive JSON.
  - [x] Expose proven bounded run-config/opaque-graph plans through the worker and
    adopt them in atomic Studio edits, retaining unsupported-dependency blockers;
    verify deployed run-after-rebind, revision guards and exact-byte undo.
  - Studio now sends supplied configuration snapshots through the existing worker's
    native planner and commits matching run-config/opaque-graph bytes, hashes and
    revisions with resource paths in one guarded transaction. Unsupported roles,
    transitive/schema-package references, missing targets, configuration relocation,
    non-local resources and ambiguous/outside/query/fragment outputs remain blocked.
    Referenced deletion uses native inventories; undo restores exact original bytes
    while advancing revisions. No source-text search/replacement was introduced.
  - Deployed execution exposed an existing native graph-publication defect: retained
    opaque bytes were ignored in favor of their descriptive JSON. A non-UTF-8 fixture
    reproduced it before the one-line publication fix. Real-worker execution now
    rebinds an opaque graph, acquires only its renamed input and reads exact binary
    artifact bytes; deployed Studio additionally verifies exact SVG output and undo.
  - Evidence: 16 native `rebind` tests and 8 `command_execution` tests pass, including
    the binary regression. CLI lint and real-worker browser verification pass.
    `@epa-wg/cem-studio:check` passes all 51 tasks (25 cached), including 20 repository
    and 55 workbench tests, two deterministic clean builds, deployed lifecycle,
    release evidence and clean consumer verification. `nativeRebinding` is now a
    required release-evidence flag. `git diff --check` passes.
  - CEM review: all editing remains in the existing shared CEM resource controls;
    new JavaScript is worker/repository transport and transaction coordination.
    Native code owns dependency slots and source-byte edits. No UI behavior,
    component CSS, theme defaults or token aliases were added. Inherited overrides,
    forced colors, reflow, automated accessibility and offline acceptance pass.
    Physical device/browser/AT acceptance and broader dependency closure remain open.
  - [x] Add native configuration-base fixtures covering literal input/inputRef and
    schema-package keys, file-base module-map/output normalization, virtual bases
    and initial command input lookup without implicit config-relative joining.
  - [x] Add native relocation fixtures for preserved graph input/output bindings,
    same-directory no-op bytes, execution-provider parity, source preservation and
    blocked self/transitive/pattern/base-override cases; retain absolute-only run-config
    admission and test exact bytes plus unresolved relative keys.
  - [x] Define execution-aligned relative run-config bases and configuration
    relocation, starting with native fixtures before widening Studio admission.
    Contract: [relative bases and safe relocation](cem-studio.md#relative-configuration-bases-and-safe-relocation).
    Initial run-config keys remain literal; file-base module maps/destinations and
    graph acquisition-relative input/output keys keep their distinct native rules.
    Native relocation candidates retain exact admitted run-config bytes and preserve
    graph resolved bindings with native literal-range edits. Self/target references,
    ambiguous bases, patterns, unknown fields and unproven dependencies are blocked.
    Candidate plans do not authorize mutation or clear host dependency obligations.
  - Verification: seven configuration fixtures pass, plus native rebinding (21),
    transform-graph (45), run-config (58) and command preparation (8) suites.
    These filters overlap. Rebuilt WASM and the Nx CLI browser-worker regression
    pass, including existing native resource rebinding and opaque-byte publication.
    `git diff --check` passes. CEM review: execution semantics/source preservation stay
    native; no UI behavior, component styling or token ownership changed. Physical
    AT acceptance remains open. At this native-only milestone, Studio relocation
    remained blocked; the integration below enables the bounded contract.
  - [x] Add relocation control-wire, real-worker, repository and deployed fixtures
    covering explicit mode, binding preservation, command/page links, collision and
    revision guards, cancellation and exact-byte undo.
  - [x] Wire the bounded native relocation contract through the worker and Studio's
    atomic repository transaction; add real-worker and deployed fixtures for incoming
    command/page references, output ownership, stale revisions and exact-byte undo.
    Bound command invocation defaults and graph dependency closure to the existing
    scalar reporting options and native opaque-input subset. Relative run-config
    keys and configuration deletion remain blocked.
  - Integration result: the existing worker action accepts explicit native relocation
    mode, requiring its source to equal the acquired configuration URI. Studio updates
    command references, graph source slots, hashes, paths and revisions atomically;
    page IDs remain stable. A path plus byte change increments the resource once.
    Output collisions, blockers, cancellation and stale revisions leave storage
    unchanged. No-byte-edit configs retain exact bytes; undo restores exact graph,
    command and config bytes and paths while advancing revisions.
  - Verification: native rebinding/relocation tests (22), rebuilt WASM real-worker
    regression, CLI lint and the full Studio check pass (51 tasks; 16 cached).
    Repository tests pass 22 cases; workbench tests pass 55. Two clean builds match
    across 260 files (sha256
    `e49eafd0b30e0aa3b8433794e60aa47c11200d11f3ee8fbf683e326834bb751b`).
    Deployed relocation execution/undo, accessibility, inherited themes/forced colors,
    offline delivery, synchronized release evidence and clean-consumer packaging pass.
    `configurationRelocation` is required release evidence. `git diff --check` passes.
  - CEM review: native code owns configuration semantics and literal source edits;
    JavaScript changes remain worker/repository services using the shared CLI parser.
    Existing shared CEM controls retain UI ownership; no component CSS, token aliases
    or theme defaults were introduced. Physical device/browser/AT acceptance remains
    independently open.
  - [x] Add native source-map rewrite ownership fixtures for v3 asset rebinding,
    map relocation, declared cycles, exact asset bytes/hashes, no-op map bytes,
    duplicate/unknown fields, collisions and unresolved source mechanisms.
  - [x] Implement a snapshot-bound native source-map candidate planner that changes
    owned path slots only and re-verifies declared edges after the proposed edit;
    retain explicit runtime/document follow-ups and keep Studio admission bounded.
    Delivered `source_module_map_rebind::plan_source_module_map_edit`: source-map v3
    asset rebinding/map relocation owns only path fields. It reuses execution's
    declared-edge verifier before/after, preserving identities, specifiers, cycles,
    content types and asset bytes/hashes. Duplicate owners, unknown fields, metadata
    conflicts, missing assets, collisions and known unresolved mechanisms fail without
    a partial plan. No-op maps retain exact bytes; changed explicit JSON controls may
    be reformatted. Every asset still requires runtime/document analysis; hosts own
    metadata/revision guards, incoming references and atomic commit/undo. No source-map
    worker or Studio admission was added.
    Contract: [native source-map rewrite ownership](cem-studio.md#native-source-map-rewrite-ownership).
  - Verification: six new native fixtures pass; related module-map (34), rebinding
    (28) and transform-graph (45) suites pass (overlapping filters). Rebuilt WASM and
    the Nx CLI browser-worker regression pass. `git diff --check` passes.
    CEM review: native code owns semantics and explicit control-document edits; no
    UI, JavaScript document interpreter, component CSS or theme defaults changed.
    Physical AT acceptance and complete transitive dependency closure remain open.
  - [x] Add native non-XPath CEM-QL dependency fixtures for resource reads versus
    content parsing, module URL resolution, pipelines, nested branches, dynamic calls,
    bounded inspection and retained template expression provenance.
  - [x] Inspect compiled CEM-QL/template expression IR without evaluation; preserve
    resource candidates and source maps while retaining unresolved body/policy work.
  - [x] Add retained-document dependency fixtures for namespace-owned HTML/SVG
    references, decoded values/source ownership, navigation versus resource candidates,
    foreign vocabularies, embedded runtime/CSS work and bounded traversal.
  - [x] Reuse native HTML/SVG direct-resource policy classification in a retained-tree
    inventory without URI guessing, fetching or document-object handoff.
  - Delivered bounded native inventories for retained HTML/SVG attributes and compiled
    CEM-QL/template expressions. Candidates preserve native ownership and source
    provenance; resolved calls distinguish acquisition, URL resolution and content
    parsing. Budget exhaustion, dynamic behavior and unresolved base/policy work
    stay explicit. Every result retains `body_analysis_required`; Studio/worker
    source-map edit admission remains unchanged.
    Contract: [native reference inventory](cem-studio.md#native-document-and-expression-reference-inventory).
    CEM review: native semantics and retained data ownership, with no app-local
    document interpreter, UI behavior, styling or theme ownership change. Theme and
    forced-color checks are not applicable to these native-only additions;
    physical AT acceptance remains open.
  - Verification: eight expression-inspection and four document-inventory fixtures
    pass; full CEM-QL library suite passes (132 passed, four ignored), as do 307
    native validation tests. CEM-QL WASM build and rebuilt CEM-ML/CLI Chromium
    worker regression pass through Nx. `git diff --check` passes.
  - [x] Add preflight-integration fixtures for hash-bound body ownership, imported
    expression slots, bounded traversal, rejected preflight and explicit resolver context.
  - [x] Consume immutable native-preflight bodies in CEM-QL, verify all byte hashes
    before inspection, and inventory compiled template slots under explicit limits.
    Direct literal module-URL resolution uses the supplied execution capability and
    current context; it preserves native policy evidence and never guesses a base.
    Six fixtures cover root/import ownership, stale/failed preflight, bounded work,
    explicit context/referrer forwarding and policy denial without acquisition.
    Contract: [preflight body inventory](cem-studio.md#native-preflight-body-inventory-and-explicit-resolution-context).
    CEM review: native semantics, no UI/component/theme changes. Authored templates
    are not rendered document trees; output reference ownership, stylesheet/module-map
    contexts and runtime closure remain open. No Studio/worker admission expansion.
  - Verification: six new integration fixtures pass; the full CEM-QL library suite
    passes (138 passed, four ignored). Nx CEM-QL WASM build and `git diff --check`
    pass. No browser-facing API or UI changed in this step.
  - [x] Add native stylesheet-dependency fixtures for retained document ownership,
    decoded inline CSS, explicit final URL bases, module-map policy, unresolved bases,
    bounded traversal and preflight stylesheet provenance.
  - [x] Connect retained document CSS and preflight stylesheet owners to the shared
    native CSS resolver without fetching, rendering or claiming runtime closure.
    Delivered native inline CSS inventory for retained HTML/SVG documents, preserving
    document owners and CSS-local maps. Resolution uses the explicit final document
    URL and execution capability; truncated scans and unresolved bases block calls.
    Preflight bodies retain stylesheet trees/module maps and resolve exact sheet
    occurrences against their final body URI. Native policy failures remain evidence;
    no imports are fetched, no template is rendered and no project edit is admitted.
    Contract: [stylesheet dependency ownership](cem-studio.md#document-and-template-stylesheet-dependency-ownership).
    CEM review: shared native import/resolution, no UI/component/theme changes;
    inherited theme and forced-color behavior are unaffected. Generated-output edit
    ownership, document base chains, SVG presentation CSS and runtime closure stay open.
  - Verification: five new stylesheet-ownership fixtures pass; full CEM-QL library
    suite passes (139 passed, four ignored). Related CSS import-closure (23) and
    resource-resolution (four) suites pass. Nx CEM-QL WASM and rebuilt CEM-ML/CLI
    Chromium worker checks pass; `git diff --check` passes.
  - [x] Add native XML reference-provenance fixtures for nested/absolute bases,
    decoded values, source selection ownership, malformed bases and bounded scans;
    test exact native source binding and rejection of changed or absent provenance.
  - [x] Resolve XML-imported reference and inline-CSS candidates using import-owned
    base chains while retaining source keys and explicit edit/policy blockers.
    Delivered XML-specific native reference provenance and inline-CSS base resolution.
    Both reuse imported per-node base chains; HTML base behavior stays blocked.
    Native source binding requires matching source key/URI, attribute name and value;
    missing or changed provenance fails. Source correspondence is not a lexical edit
    range or mutation authorization. No Studio/worker admission was widened.
    Contract: [XML base/source correspondence](cem-studio.md#xml-base-chains-and-native-source-correspondence).
    CEM review: shared native import and retained source ownership; UI, accessibility
    behavior and theme token ownership are unchanged. Generated-output edit ownership,
    safe rewriting, HTML base rules and runtime closure remain open.
  - Verification: four new XML/source-binding fixtures pass within 73 XML-related
    native tests; four existing document-stylesheet regressions also pass. Rebuilt
    CEM-ML WASM/CLI and the Nx Chromium worker regression pass. `git diff --check`
    passes. Physical AT acceptance remains independently open.
  - [x] Add XML lexical-edit fixtures for quote/entity preservation, exact no-ops,
    source-key staleness, inherited bases and unchanged neighboring references.
  - [x] Plan one source-bound XML reference value edit at the native import boundary,
    reimport and verify targets, retaining explicit mutation/runtime blockers.
    Delivered native import-owned lexical value planning with source-key selection,
    exact no-op preservation, XML escaping and reimported before/after target checks.
    A retained-occurrence entry point requires native source correspondence first.
    Four fixtures cover lexical preservation, inherited bases, no-ops, stale keys,
    malformed bases, bounds and occurrence binding. Candidates keep runtime/shared
    occurrence/host-transaction blockers; no worker or Studio admission was widened.
    Contract: [native XML value edits](cem-studio.md#native-xml-reference-value-edit-candidates).
    CEM review: syntax stays at native import; UI, theme and accessibility behavior
    are unaffected. Shared/generated occurrence ownership and runtime closure remain open.
  - Verification: all 77 XML-related tests pass, including four new edit-planning
    fixtures. Rebuilt CEM-ML WASM/CLI and the Nx Chromium worker regression pass;
    `git diff --check` passes. Physical AT acceptance remains independently open.
  - [x] Add native shared-occurrence transaction fixtures for single/source-wide
    scope, stale source/output revisions, output membership/owners and policy drift.
  - [x] Prepare immutable XML edit evidence bound to an explicit occurrence scope
    and revalidate the host read set without authorizing incomplete dependency edits.
    Delivered explicit single-occurrence/source-wide scope over a supplied native
    output set. Private prepared evidence retains original/candidate bytes and all
    output owners. Revalidation rejects source byte/revision, policy, output set,
    output revision and native-owner drift; order-only changes are accepted.
    Unknown provenance, stale source correspondence and opaque references block
    preparation. The host still owns output-set completeness and atomic admission;
    runtime closure remains required. No Studio/worker edit admission was added.
    Contract: [shared occurrences and transaction evidence](cem-studio.md#shared-xml-occurrences-and-prepared-transaction-evidence).
    CEM review: native semantics and retained owners; no UI/accessibility/theme change.
  - Verification: all 81 XML-related tests pass, including four transaction fixtures;
    the focused transaction suite passes after final duplicate-node hardening.
    Rebuilt CEM-ML WASM/CLI and the Nx Chromium worker regression pass, as does
    `git diff --check`. Physical AT acceptance remains independently open.
  - [x] Add native HTML base-resolution fixtures for first-href ordering, explicit
    fallback contexts, policy denial, source ownership and incomplete scans;
    compare the shared bounded fixture corpus with Chromium document.baseURI.
  - [x] Implement bounded UTF-8 HTML snapshot base resolution over retained CEM
    trees, retaining selected source and host policy evidence without widening
    dependency completeness or Studio edit admission.
    Delivered explicit ordinary/srcdoc/about:blank fallback contexts, first-href
    selection, standard URL resolution and host policy decisions. Results retain
    native owners/source maps/keys; incomplete scans, unevaluated values, ambiguous
    template content and unsupported encodings fail. No general text/html importer
    or live frozen-base history reconstruction is claimed.
    Contract: [bounded native HTML base resolution](cem-studio.md#bounded-native-html-base-resolution).
    Chromium 148.0.7778.96 differs on invalid-first-href and empty-href-with-fragment;
    the parity gate asserts exact version/values and records divergence explicitly.
    Those cases remain outside execution/edit admission. CEM review: native semantic
    ownership and bounded host policy; no UI/accessibility/theme changes, no app-local
    document interpreter. Theme/forced-color checks do not apply to this native API;
    physical AT acceptance remains independently open.
  - Verification: nine HTML-base-related native library tests and both integration
    tests pass, including the shared 24-case corpus and unevaluated-attribute
    regression. Rebuilt CEM-ML WASM/CLI Chromium checks and CLI lint pass through Nx;
    the browser report records 22 matches and two exact, version-pinned divergences.
    `git diff --check` passes. No dependency-completeness or edit grant was added.
  - [x] Define browser compatibility admission for the measured invalid/empty-href
    base divergences and bind fallback/policy to actual freeze-time evidence before
    consuming HTML base results in browser reference or stylesheet inventories.
    Delivered explicit standard-agreement and pinned Chromium profiles, with private
    preparation/confirmation evidence retaining original fallback, policy decision,
    native source owner and execution epoch. Browser observations must match the
    selected profile; read-set validation rejects owner, revision, epoch, selected
    slot, browser, observed-URL or policy drift without recomputing the freeze.
    The two known static divergences are admitted only as explicit base evidence.
    A newly measured stale fallback after CSP denial/removal/history is rejected;
    it is outside this bounded compatibility profile. General resource/edit admission
    remains blocked on host observation completeness, runtime closure and transactions.
    Contract: [captured HTML base evidence](cem-studio.md#captured-html-base-evidence-and-browser-compatibility).
    CEM review: explicit bounded compatibility, native semantic/source ownership;
    no UI, accessibility, theme-token or inherited-style changes. Physical AT remains open.
  - [x] Add native HTML freeze-evidence fixtures for explicit compatibility profiles,
    captured fallback/policy, stale document/epoch/owner/selection rejection and
    missing observations; verify actual history, href and CSP transitions in Chromium.
  - Verification: seven native freeze-evidence tests and both HTML base integration
    tests pass. The Nx browser gate confirms all 24 static observations under the
    explicit Chromium profile, retaining the two standard divergences. Eight actual
    browser transitions verify five captures, two stale read-set rejections and one
    unsupported stale-fallback rejection. Rebuilt CEM-ML WASM/CLI, Chromium worker
    regression, CLI lint and `git diff --check` pass.
  - [x] Add native execution-host fixtures for complete bounded slot correspondence,
    host-owned epochs, same-value freezes, stale/foreign evidence and revocation;
    verify the shared browser adapter against real DOM mutation and history events.
  - [x] Connect captured HTML base evidence to a bounded exclusive execution-host
    adapter with native/DOM document-slot correspondence and controlled
    revision/epoch invalidation. Delivered a native retained session, WASM predicate
    bridge and shared `@epa-wg/cem-ml-cli/browser` adapter. Complete document
    correspondence precedes policy; explicit href re-freezing, synchronous
    invalidation, pending mutation records and read-time identity checks prevent
    stale admission. Inert template content stays outside document topology.
    Unowned DOM changes, failed confirmation, callback reentry/errors and document
    replacement revoke the session. No JavaScript AST reconstruction was added.
    The initial profile requires exclusive UTF-8 HTTP(S) context ownership;
    arbitrary page writers, unreported same-task history A-B-A, embedded-document
    hosts and a general native HTML snapshot producer remain outside this scope.
    A newly measured Chromium same-value href/CSP mismatch is rejected explicitly,
    without broadening compatibility. Resource closure and transaction admission
    remain blocked. Contract: [exclusive HTML base execution host](cem-studio.md#exclusive-html-base-execution-host).
  - Verification: five native host tests, seven freeze-evidence tests and both
    base integration tests pass. The rebuilt Nx WASM/browser gate passes 17 new
    packaged host cases, the 24 static base confirmations, eight historical
    transitions and existing worker/command regressions. CLI lint and
    `git diff --check` pass. CEM review: native semantic/source ownership and shared
    bounded browser services; no UI, token, inherited-style or accessibility change.
    Theme/forced-color checks are inapplicable; physical AT acceptance stays open.
  - [x] Add native HTML execution-inventory fixtures for confirmed base consumption,
    retained document/CSS source owners, resolver denial and drift, bounds, and
    stale/foreign host evidence; verify shared Chromium reference/CSS candidates.
  - [x] Consume validated execution-host evidence in native reference and
    stylesheet inventories, preserving the exclusive-context boundary and
    revalidating the same retained owner before consumption.
    Delivered private `HtmlExecutionInventory` ownership through the native host's
    `inspect_dependencies`/`validate_dependencies` APIs. Direct references use the
    confirmed execution base; inline CSS retains the actual resolver capability,
    mapping/denial evidence, document source frames and native CSS owners. Full
    scans precede resolution; stale/foreign/replaced read sets, missing resolver
    fingerprints and resolver/DOM drift prevent publication or consumption.
    Generic/XML stylesheet guards remain unchanged. Candidate coverage, resource
    policy, stylesheet execution history, runtime closure and edit transactions
    remain explicit blockers; no browser/worker edit admission was added.
    Contract: [HTML execution inventories](cem-studio.md#html-execution-reference-and-stylesheet-inventories).
  - Verification: all seven new inventory tests, 14 existing base/host integration
    tests and 210 document regression tests pass. The rebuilt Nx WASM/Chromium gate
    passes 18 host cases, including native/browser image, navigation and inline-CSS
    URL parity, plus 24 static base confirmations, eight transition cases and
    existing worker/command checks. CLI lint and `git diff --check` pass.
    CEM review: native semantics/source ownership, bounded shared host services and
    explicit authority; no UI, accessibility, theme-token or inherited-style change.
    Theme/forced-color verification is inapplicable; physical AT remains open.
  - [x] Integrate retained document/expression inventories with native template
    preflight and execution-owned base/resolver policy before producing edit evidence.
    - [x] Add native preparation fixtures covering retained preflight imports and
      expression owners, distinct document/template/execution bases, explicit
      referrers and denied resolutions, stale/replaced authorities, callback drift,
      missing body contexts and bounded expression/stylesheet analysis without
      runtime acquisition.
    Delivered the private native `HtmlTemplateDependencyPreparation`: construction
    runs execution's native preflight once, retains imported bytes/hashes and compiled
    expression/CSS owners, and joins them with a current HTML inventory. Every body
    requires its explicit execution capability; complete bounded scans precede
    resolution. Native root/engine/capability identity and resolver fingerprints are
    checked with the HTML host around callbacks and at later consumption. Template
    CSS, document references and expression calls preserve their distinct bases;
    denied mappings retain native policy evidence. Public preflight DTOs cannot mint
    this preparation. Runtime reads and rendering are never executed by inspection.
    Template/output correspondence, effective module maps, acquisition freshness,
    document coverage/policy, runtime closure and transaction admission remain
    blockers. This native API does not widen browser/worker or Studio edit admission.
    Contract: [native template/HTML preparation](cem-studio.md#native-template-and-html-dependency-preparation).
    Verification: seven native integration tests and 15 existing dependency inspection
    regressions pass; `cem_ql:build:wasm` succeeds. New Rust files pass `rustfmt --check`
    and `git diff --check` is clean. No browser behavior or admission path changed.
    CEM review: shared native semantics, explicit authority and retained sources;
    no app-local parser, UI behavior, token, inherited-style or accessibility change.
    Theme/forced-color checks are inapplicable; physical AT remains independently open.
  - [ ] Complete execution-aligned general document references, non-XPath template
    loads and runtime-load closure for source-map assets; combine those findings
    with native map rewrite ownership before widening configured-project admission.
    - [x] Bind template/output correspondence, effective body contexts/module maps
      and acquisition freshness to native execution and immutable snapshot records.
      Use those records to discharge preparation blockers before edit evidence
      for the bounded snapshot XHTML profiles below.
      - [x] Add native snapshot/execution fixtures for immutable declared imports,
        policy denial/substitution, missing/file resources without fallback I/O,
        source and output owner replacement, shared-context expression execution,
        bounds/cancellation, resolver drift and rejected unrecorded body maps/reads.
      - [x] Capture a bounded native XHTML execution from an immutable template
        snapshot and bind its exact output to the HTML dependency preparation.
        Delivered immutable native source snapshots and sealed snapshot preflight,
        preserving exact URI/revision/type/bytes and native import policy. Missing
        resources never fall back to filesystem, network or registered resolvers.
        Source/byte/import-edge/cumulative-byte bounds and cancellation apply.
        The native execution record compiles captured imports, renders scalar
        bindings with the actual shared resolution capability, writes XHTML and
        imports that output through CEM-ML. It retains original render spans and
        exact output/source owners. Dependency preparation reuses the same native
        preflight and body inventories without reacquisition or recompilation.
        Equal reimported output, another execution/snapshot/engine/capability,
        stale host evidence and resolver drift cannot replace retained authority.
        Only output correspondence, snapshot acquisition freshness and output
        document inspection blockers are discharged. Authored maps (including
        nested unapplied maps), runtime reads, computed/pipeline resolution,
        unknown calls and XPath are rejected by this initial execution profile.
        General runtime/resource coverage and transaction admission remain open;
        output XML coordinates are not template-source rewrite locations.
        Contract: [recorded snapshot XHTML execution](cem-studio.md#recorded-snapshot-xhtml-execution).
        Verification: six snapshot fixtures, four execution fixtures, 11 native
        preflight regressions, seven HTML preparation regressions and 139 CEM-QL
        library tests pass; four existing library tests remain ignored.
        `cem_ql:build:wasm`, new-file `rustfmt --check` and `git diff --check` pass.
        CEM review: shared native semantics, explicit source/context ownership and
        bounded inputs; no app-local parsing or JSON AST handoff. No UI, token,
        accessibility or inherited-style change; theme/forced-color checks are
        inapplicable and physical AT remains independently open.
      - [x] Record per-body execution context switching and authored module-map
        installation, including imported bodies, before admitting those maps in
        snapshot execution and discharging effective-context blockers.
        - [x] Add native fixtures for derived context ownership, inherited map and
          scheme restrictions, root/import/grandchild expression contexts, caller
          restoration, CSS precedence, equal repeated imports, ambiguous import
          ancestry, match/hook/conditional/iteration expressions, unsupported
          custom resolvers, derived-policy drift, map budgets and nested unapplied maps.
        Delivered `execute_snapshot_xhtml_scoped`: immutable native child contexts
        install each body's compiled top-level map at its final snapshot URI,
        inheriting declared import ancestry and host restrictions. Non-CSS mappings
        remain outer-first; CSS remains closest-first with ancestor restrictions.
        The closure compiler attaches exact body capabilities to native expressions;
        calls, matches, hooks and control expressions preserve lexical context and
        restore the caller after evaluation. Source labels and authored markers
        cannot select authority; serialized artifacts omit native capabilities.
        Dependency preparation reuses these owners, and consumption checks every
        derived fingerprint as well as the host's fingerprint. The combined view
        discharges effective map/context blockers. Equal repeated imports under
        one parent work when preflight accepts them; conflicting ancestry and
        nested unapplied maps are rejected. Custom resolvers require explicit native
        derivation support. Declared imports remain captured preflight acquisitions.
        Runtime/resource coverage, stylesheet history and atomic edit admission
        remain blocked; no browser/worker or Studio edit admission was widened.
        Contract: [recorded lexical body contexts](cem-studio.md#recorded-lexical-body-contexts-and-module-maps).
        Verification: two derivation fixtures, all 11 execution cases (seven new),
        15 native resolver regressions, 139 CEM-QL library tests and 56 related
        renderer/preparation/artifact-reload tests pass; four existing library
        tests remain ignored. `cem_ql:build:wasm`, new-file `rustfmt --check` and
        `git diff --check` pass. CEM review: shared native semantics, explicit
        source/context ownership and bounded variation; no app-local parser or
        JSON AST handoff. UI, public tokens, accessibility and inherited styles
        are unaffected; theme/forced-color checks are inapplicable and physical
        AT acceptance remains independently open.
    - [x] Inventory HTML `srcset` candidates through shared native retained-source
      parsing, with descriptor validation, source locations, bounded work and
      execution-owned base resolution before discharging that coverage blocker.
      - [x] Add native parser/import/inventory fixtures for URL commas, ASCII
        whitespace, density/width/height descriptors, recovery, numeric limits,
        decoded entity/Unicode offsets, namespace/owner guards, budget exhaustion,
        current/stale execution bases, shared validation policy and Chromium
        candidate parity.
      Delivered a shared bounded importer for retained HTML `img@srcset`,
      `source@srcset` and `link@imagesrcset`. It preserves decoded UTF-8 candidate
      and descriptor ranges, exact source owners and structured recovery/numeric
      diagnostics. The existing HTML policy validator reuses the same parser.
      The execution inventory resolves candidates with its confirmed base;
      imported XML bases and module mappings do not override it. Work limits and
      unsupported numeric profiles reject publication before CSS callbacks;
      existing DOM/host/capability checks reject stale consumption. Supported
      complete lists discharge their parsing/base requirements. Resource policy,
      responsive-source selection/load history, general coverage and edit
      admission remain explicit; decoded offsets do not authorize source rewrites.
      Contract: [retained HTML image-source candidates](cem-studio.md#retained-html-image-source-candidates).
      Verification: five parser/import fixtures, nine HTML inventory cases,
      16 HTML validation tests, 210 document-focused regressions and 18 CEM-QL
      preparation/execution tests pass. The Nx `@epa-wg/cem-ml-cli:test:browser`
      gate rebuilt WASM and passes, including seven shared srcset/currentSrc
      cases in Chromium 148.0.7778.96 and the existing base/worker checks.
      New-file `rustfmt --check` and `git diff --check` pass. CEM review:
      source authority and parsing remain shared native capabilities with bounded
      inputs; no app-local parser or JSON AST handoff. UI, public tokens,
      accessibility and inherited styles are unaffected; theme/forced-color
      checks are inapplicable and physical AT acceptance remains independently open.
    - [x] Capture bounded native CSS `@import` dependency closure from immutable
      acquired sources, retaining final URL bases, conditions, resolver policy,
      source owners, cycle/budget diagnostics and freshness before discharging
      import coverage. Keep stylesheet installation/load history separate.
      - [x] Add native snapshot/HTML fixtures for redirected and repeated imports,
        retained conditions/source ranges/revisions, missing sources without I/O,
        alias conflicts, MIME/integrity/import and leaf policy denial,
        cycle/depth/sheet/byte/root bounds, cancellation, resolver drift,
        equal-source owner substitution and stale
        HTML hosts; rerun existing import/emission regressions and WASM build.
      Delivered immutable `CssSourceSnapshot` responses and sealed native capture
      through the existing `CssImportClosure` loader. Exact acquisition owners,
      revisions, final URL bases, conditions and source ranges survive repeated
      import occurrences. Missing responses never fall back to I/O. Shared MIME,
      integrity, redirect policy, placement and cycle checks remain authoritative;
      response/sheet/depth/root limits, cancellation and resolver drift prevent
      publication. Equal replacement snapshots, roots or capability labels cannot
      restore authority. Hosts supply the current snapshot again at consumption.
      HTML execution integration binds the exact retained inline roots, shares
      limits across them and revalidates DOM/host/policy around consumption.
      Its combined view discharges only those roots' import coverage; ordinary
      resource denial, conditions, installation/history, runtime coverage and
      atomic admission stay explicit. External stylesheet and template-body
      integration follow; no Studio/browser-worker edit admission is widened.
      Contract: [immutable CSS import snapshot capture](cem-studio.md#immutable-css-import-snapshot-capture).
      Verification: six snapshot fixtures, all 11 HTML inventory cases (two new),
      23 existing import-closure cases, four resource-resolution cases and nine
      resource-emission cases pass (53 total). `cem_ql:build:wasm`, new-file
      `rustfmt --check` and `git diff --check` pass. CEM review: native shared
      import/loading semantics, explicit retained authority and bounded inputs;
      no app-local parser, JSON AST handoff, UI behavior or token change. Inherited
      styles and accessibility are unaffected; theme/forced-color checks are
      inapplicable and physical AT acceptance remains independently open.
    - [x] Bind captured CSS import snapshots to recorded template/body execution
      and dependency preparation, preserving each body's effective capability,
      root/source correspondence and acquisition freshness before discharging
      template stylesheet import coverage.
      - [x] Add native binding fixtures for root/imported-body map precedence,
        redirected nested imports and source ownership, distinct HTML/template
        bases, repeated occurrences and aggregate budgets, missing/denied CSS,
        empty stylesheet sets, snapshot/execution replacement, cancellation,
        host/derived-policy drift and retained runtime/history/edit blockers;
        verify native dependency regressions and the WASM build.
      Delivered `ExecutedHtmlTemplatePreparation::prepare_css_imports`, retaining
      exact execution/preparation owners, native body stylesheet roots and their
      recorded effective capabilities. Body CSS uses the final template URI;
      document CSS uses the confirmed HTML base; redirected imports retain their
      final response bases and the owning body's mapping/policy context. Repeated
      occurrences, conditions, source maps and acquisition revisions survive.
      All root bounds precede resolution, with shared sheet/root-node/response-byte
      budgets across document and body closures. Missing/denied imports, partial
      captures and cancellation cannot publish. Consumption rejects replacement
      snapshots/executions/engines/capabilities and stale host evidence. Every body
      fingerprint is checked, including empty stylesheet sets; observed policy
      drift stays invalid after restoration, including drift during DOM callbacks.
      The combined view discharges only stylesheet ownership/import coverage;
      original inventory blockers remain intact. Resource denials, conditions,
      installation/history, runtime coverage and atomic edit admission remain
      explicit. No template reacquisition, CSS installation or browser/Studio edit
      admission was added; CSS-local ranges remain distinct from source edit ranges.
      Contract: [recorded template CSS import binding](cem-studio.md#recorded-template-css-import-binding).
      Verification: seven new binding fixtures, 31 CEM-QL execution/preparation/
      retained-stylesheet regressions, seven native dependency inspection tests
      and 17 CSS snapshot/HTML inventory cases pass (62 total).
      `cem_ql:build:wasm`, new-file `rustfmt --check` and `git diff --check` pass.
      CEM review: shared native import/loading semantics, retained source/context
      authority and bounded inputs; no app-local parser, JSON AST handoff or UI
      behavior change. Public tokens, accessibility and inherited styles are
      unaffected; theme/forced-color checks are inapplicable and physical AT
      acceptance remains independently open.
    - [x] Capture external HTML link stylesheet roots from execution-owned link
      occurrences and immutable response snapshots, preserving link eligibility,
      confirmed href/base ownership, final URL policy, conditions and bounded
      import closure. Keep stylesheet installation/load history separate.
      - [x] Add native fixtures for link rel/type/namespace/inert eligibility,
        confirmed bases, repeated/redirected roots, source ownership, integrity,
        missing/denied responses, aggregate bounds and stale captures.
      Delivered exact retained link occurrences and immutable external response
      roots through `HtmlExecutionCssImports`. Confirmed HTML bases determine
      hrefs; native policy may restrict but cannot remap them. Shared response
      admission enforces MIME, integrity and byte bounds before native import;
      nested imports retain final URL bases. Link metadata, source owners,
      revisions and repeated occurrences survive, including disabled/alternate
      candidates. Inline, external and body roots share all capture budgets.
      Missing acquisitions never fall back to I/O; failed/incomplete captures
      cannot publish. Combined preparations discharge external closure coverage
      while retaining transport, runtime, installation and edit requirements.
      Contract: [external stylesheet snapshot roots](cem-studio.md#external-stylesheet-snapshot-roots).
    - [x] Bind stylesheet selection, installation and load history to native
      execution evidence for document, body and external-link roots before
      discharging their conditions/execution-history blockers.
      - [x] Add native execution-host fixtures for complete root/import observations,
        load/source and installation identity, conditional/disabled selection,
        epochs, foreign evidence, callback failure/drift and bounded work; verify
        actual stylesheet observations in Chromium.
      Delivered exclusive `StylesheetExecutionHost` epochs and sealed complete
      observations, bound to HTML and recorded-template CSS preparations. Native
      probes retain exact source owners, root/import conditions, body scopes and
      selection metadata. Actual execution adapters must verify loaded sources,
      installation identities and condition outcomes; source descriptors alone
      are insufficient. Inactive imports cannot contradict their ancestors;
      active occurrences require load/install evidence. Bounds, callbacks,
      source/owner substitution and environment drift reject or revoke capture.
      Observed drift stays invalid after restoration, including empty HTML CSS
      forests. Combined results discharge only installation/import-condition
      history; inner-rule/computed-resource runtime coverage and edits stay blocked.
      Production browser/worker observation wiring follows this native contract.
      Contract: [exclusive stylesheet execution observations](cem-studio.md#exclusive-stylesheet-execution-observations).
    - [x] Inventory SVG presentation URL references through shared native CSS
      import and execution-owned bases, preserving source and policy evidence.
      - [x] Add SVG fixtures for paint/filter/clip/mask/marker/cursor values,
        CSS escapes and decoded sources, local fragments, namespace and SMIL
        guards, malformed values, bounds, native ownership and Chromium parity.
      Delivered retained `fill`, `stroke`, `filter`, `clip-path`, `mask`, marker
      endpoint and cursor values through the shared CSS import boundary. Native
      syntax owns escapes, URL candidates and diagnostic offsets; no downstream
      CSS string parser was added. Declaration injection, top-level separators,
      `!important`, malformed syntax and bounded-work exhaustion reject analysis.
      Exact document/attribute/CSS owners survive, with decoded-value coordinates
      kept separate from edit locations. HTML execution uses the confirmed base;
      explicit XML uses import-owned bases. Denials remain visible, local fragments
      remain local dependencies and SMIL/computed SVG behavior remains blocked.
      Contract: [retained SVG presentation URL values](cem-studio.md#retained-svg-presentation-url-values).
      Verification for these three slices: 68 native CSS/import/HTML/SVG tests,
      211 document regressions and 26 CEM-QL preparation/execution tests pass
      (305 total). The Chromium fixture checks five stylesheet occurrences,
      actual loaded bytes/URLs, disabled/alternate roots, disable/restore/media
      transitions, SVG paint/fragment parity and two native stale rejections.
      `cem_ql:build:wasm`, CLI lint, new-file `rustfmt --check` and
      `git diff --check` pass.
      The rebuilt Nx `@epa-wg/cem-ml-cli:test:browser` gate passes the new
      stylesheet/SVG fixture plus 24 base confirmations, eight freeze transitions,
      18 execution-host cases and existing worker/command regressions.
      CEM review: shared native import/loading semantics, bounded variation and
      explicit execution/source ownership; no app-local behavior/parser or JSON
      AST handoff. Public tokens, inherited overrides and accessibility remain
      unchanged. Theme/forced-color checks are inapplicable; physical AT remains
      independently open. Browser transport policy and atomic edit admission
      remain separate requirements.
    - [x] Connect the exclusive stylesheet observer's bounded HTML profile to
      the shared browser/worker runtime, including CSSOM, load, selection,
      environment and lifecycle invalidation before exposing execution evidence.
      - [x] Add native source-probe fixtures for exact inline/external/import
        ownership, import order, UTF-8 bounds, ordinary element titles and
        unsupported observation profiles.
      - [x] Add packaged Chromium fixtures for CSSOM/source mismatch, controlled
        and unowned mutations, load/selection/media transitions, lifecycle
        revocation and worker-pool close/replacement invalidation.
      Delivered a bounded shared `BrowserStylesheetHost` through the confirmed
      HTML base host and `BrowserWorkerPool`. Native WASM sessions retain exact
      document/CSS/acquisition owners and sealed observations. Browser predicates
      compare installed CSSOM with native-owned text parsed by the browser,
      including final URLs, import order, media/supports and installation identity.
      No source-key echo, app-local parser, network fallback or JSON AST handoff
      can substitute. Ordinary element `title` attributes remain distinct from
      stylesheet-set titles, preserving inline CSS on labeled elements.
      Synchronous change boundaries invalidate before CSSOM/selection/environment
      writes, including A-B-A. Load/error/media/viewport events, DOM/base changes,
      abort, disposal and worker failure/close/replacement invalidate or revoke
      consumption; all failed/replacing generations must recover before new
      pool-bound sessions are admitted. Read summaries cannot grant edits or
      transfer native execution evidence to workers.
      Contract: [shared browser stylesheet observation host](cem-studio.md#shared-browser-stylesheet-observation-host).
      Verification: four new native source-probe fixtures plus 26 ML host/CSS
      regressions and eight recorded-template CSS tests pass (38 native tests).
      The rebuilt Nx `@epa-wg/cem-ml-cli:test:browser` gate passes 18 new shared
      stylesheet-host cases and existing base, stylesheet/SVG, worker and command
      checks. The gate rebuilds the ML WASM runtime and TypeScript host. CLI lint,
      focused Rust formatting and `git diff --check` pass.
      The initial profile covers UTF-8 inline styles/attributes, untitled external
      sheets and loaded readable imports. Titled sets, unloaded imports,
      shadow/adopted sheets and compiled body installation correspondence reject
      that initial profile; the following slice extends its bounded coverage.
      Exclusive ownership is required: observers cannot reconstruct uncontrolled synchronous CSSOM A-B-A.
      CEM review: semantic/native source authority and bounded shared capability
      first; no application/component behavior or style changes. Public tokens,
      inherited overrides and accessibility are unaffected. Theme/forced-color
      checks are inapplicable; physical AT remains independently open. Computed
      resource/runtime closure and atomic edit admission remain blocked.
    - [x] Extend the shared browser observer to recorded template/body stylesheet
      installations with exact compiled-source correspondence; then add bounded
      titled/alternate selection and unloaded-import evidence where observable.
      - [x] Add native compiled-installation fixtures for exact emitted-source/
        scope ownership, complete/empty recorded body coverage, diagnostics, shared
        consumer release/replacement/source cancellation, bounded probes and
        stale evidence.
      - [x] Add shared Chromium fixtures for compiled private/shared/instance
        installations, source/CSSOM tampering, selection changes, release and
        lifecycle invalidation, titled sets, observable unloaded imports and
        rejection of CSSOM-omitted conditional imports.
      Delivered sealed native compiled-installation sets and recorded-body plans,
      using the exact captured import closures, native emitter and admitted scopes.
      Complete ordered body coverage, empty-body policies and diagnostic-free
      emissions are required. Per-consumer leases retain incoming closures even
      on cache hits, so release, replacement, source cancellation and disposal
      cannot be hidden by another consumer's cached emission. Imported rules share
      their actual flattened output installation instead of invented source IDs.
      The realm-local QL WASM session and shared browser/pool observer verify
      exact emitted CSSOM plus style/sheet/parent identity, with controlled
      selection, mutation and lifecycle invalidation. Recorded-body compilation
      remains a native API; whole-output correspondence and worker-resident
      preparation integration remain separate below. No serialized seal or AST
      handoff is admitted.
      HTML observations now admit explicitly disabled titled/alternate sets and
      genuinely unloaded inactive imports with immutable source snapshots. The
      Chromium fixture covers a pending media import, its unloaded descendant,
      and its subsequent real load/invalidation. Direct capture of active titled sets rejects:
      Chromium can expose a false disabled flag while an alternate stays inactive.
      Unsupported supports imports omitted from CSSOM also reject instead of
      becoming inferred unloaded evidence. These compatibility limits remain
      actionable follow-ups; no source-only fallback is introduced.
      Contract: [compiled installation and selection evidence](cem-studio.md#compiled-template-and-body-stylesheet-installations).
      Verification: 28 QL and 19 ML native tests pass (47 total). The rebuilt Nx
      browser gate passes 20 new compiled-installation cases and nine new
      selection/import cases, plus 18 existing shared stylesheet cases and all
      HTML-base, stylesheet/SVG, worker and command regressions.
      `cem_ql:test:retained-stylesheets` passes its native and WASM lifecycle
      regressions. Both WASM rebuilds, CLI lint, focused Rust formatting and
      `git diff --check` pass.
      CEM review: native source/policy ownership and bounded shared capabilities
      remain authoritative; private/shared/instance scoping is checked through
      actual computed styles. No component/app behavior, public token defaults,
      inherited overrides, accessibility or theme ownership changes. Forced-color
      checks are inapplicable; physical AT remains independently open. Inner
      conditional/computed resource loads and atomic edit admission stay blocked.
    - [x] Define browser-owned active titled-set transitions and HTML/load
      recapture. Chromium can expose `disabled=false` for an inactive alternate;
      CSSOM flags alone must not certify activation.
      - [x] Add native selection-plan fixtures for exact titles, disabled and
        inline boundaries, retained owners, fresh epochs and source/policy drift.
      - [x] Add Chromium transition fixtures for actual application, repeated
        selection, multiple links, pending reads, load failure, cancellation,
        timeout, replacement and HTML/source recapture rejection.

      Implemented asynchronous `html.selectStylesheets(...)` and its pool-bound
      counterpart. Native commands select exact titles and preserve persistent
      and authored-disabled sheets. The host invalidates before explicit link
      enable/reload, waits for every selected link's trusted load and fresh sheet,
      then recaptures HTML and verifies complete CSSOM/source correspondence.
      Native rebinding retains the original tree, response snapshot, host,
      base and policy and requires a newer HTML epoch. Reads and competing writes
      reject while pending. Same-title/A-B-A transitions cannot revive evidence;
      errors, timeout, abort, DOM/source/policy drift, disposal and worker generation
      changes fail closed. Activation receipts remain private to the browser host.
      Bounds: active selected roots must be external links, all roots must already
      be loaded, and CSSOM must be readable. Inline titled activation remains
      unsupported. Empty title explicitly disables every titled root.
      Verification: 23 focused ML native tests pass, including four new selection
      tests. Rebuilt ML/QL WASM and the full Nx browser gate pass 19 new active-set
      cases plus existing shared/compiled/selection/import, HTML-base, SVG,
      worker and command regressions. CLI lint, focused Rust formatting and
      `git diff --check` pass.
      CEM review: native semantic/source ownership and bounded shared services
      remain authoritative. Actual computed-style checks preserve persistent
      rules while selecting all intended links/imports. No UI behavior, public
      token defaults, inherited overrides, accessibility or theme ownership changes;
      theme/forced-color checks are inapplicable and physical AT stays independent.
      Whole-output correspondence, remaining resource closure and transaction
      admission retain their blockers.
    - [x] Join native recorded-body installation evidence with complete rendered
      output correspondence before worker-owned preparation/transaction consumption;
      preserve native owners and never transfer seals through result DTOs.
      - [x] Add native whole-output fixtures for ordered private/shared
        placement and instance-scope rejection, emitted source spans, HTML delivery,
        exact HTML ownership, missing/changed DOM or CSS, cancellation, policy
        drift, release and bounded failure.
      - [x] Add Chromium parity for the native composed output, including real
        scoped application and complete DOM/source checks; keep fixture exports
        separate from native consumption authority.

      Implemented `RecordedBodyStylesheetInstallations::compose_output` and
      sealed `RecordedTemplateOutputEvidence`. Composition reuses the recorded
      native render and exact emitted fragments, preserves source frames and
      ordered private/shared declaration placement, and retains the original
      execution/source owners. Exact composed-tree capture joins full HTML
      topology/identity/value checks with document CSS observations and compiled
      installation identities; independent caller assertions cannot join the two.
      Explicit XML/HTML delivery stays separate from authority, including correct
      empty-template serialization. Equal bytes, changed slots or CSSOM, callbacks,
      policy drift, cancellation, release and stale HTML epochs reject consumption.
      Empty stylesheet sets still retain body policies. Instance payloads preserve
      their separate contract; declaration styles cannot adopt instance scope.
      Verification: 33 focused QL native tests pass (seven new cases), eight
      live-native Chromium cases pass, and the complete Nx browser gate and QL
      WASM build pass with all prior HTML-base, SVG, stylesheet, worker and command
      regressions. CLI lint, focused Rust formatting and `git diff --check` pass.
      The Chromium fixture retains native owners in one Rust process throughout
      scalar DOM/CSS callbacks; JSON test controls never transfer a native seal.
      CEM review: shared native ownership, bounded variation and existing CEM
      private/shared scope rules remain authoritative. Real computed-style checks
      use the required produced-host instance-island structure and verify scope
      containment. No UI behavior, public token defaults, inherited overrides,
      accessibility or theme ownership changes; theme/forced-color checks are
      inapplicable and physical AT remains independent. Runtime resource closure
      and atomic host transaction admission remain blocked.
    - [x] Add native worker-owned preparation handles for joined output records,
      with generation, release and cancellation invalidation. Keep native owners
      in their execution realm and never reconstruct seals from result DTOs.
      - [x] Add native registry fixtures for exact owner/generation binding,
        bounded retention, release/close/drop, cancellation during capture/read,
        failed-read revocation, replacement recovery and empty-style policies.
      - [x] Route the live-native Chromium fixture through retained preparation
        handles and verify lifecycle revocation alongside DOM/CSS drift.

      Implemented `RecordedPreparationWorker`, which retains the complete native
      output evidence and owns both observation hosts behind private native
      handles. Each read revalidates the DOM/CSS record, checks lifetimes around
      every scalar callback, and returns diagnostic blockers only. Failed reads
      and callback unwinding remove the record permanently. Release, abort,
      close/drop and consecutive worker generation replacement revoke all
      applicable handles; equal addresses/IDs and old lifetime controls cannot
      substitute for owner identity or act on a replacement generation.
      Retention is bounded to 1–256 live records, capacity rejects before
      callbacks, and revoked entries are reclaimed before new preparation.
      The host must revoke before controlled same-value writes or termination.
      Native sources/policies and empty stylesheet contexts remain authoritative.
      Verification: 45 focused QL native tests pass (12 new lifecycle tests).
      All 15 live-native Chromium output/preparation cases pass, including seven
      new lifecycle cases and existing real scoped-style/DOM/CSS checks. The full
      Nx browser gate passes HTML-base, SVG, stylesheet selection/installation,
      worker-pool and command regressions. QL WASM build, CLI lint, focused Rust
      formatting and `git diff --check` pass. Remaining resource closure and
      atomic transaction admission retain their blockers.
      CEM review: the shared native service preserves semantic/source ownership
      and bounded variation; no AST or seal crosses a result DTO. Existing
      private/shared scope rules, public token defaults, inherited overrides,
      accessibility and theme ownership are unchanged. Theme/forced-color checks
      are inapplicable; physical AT acceptance remains independent.
    - [x] Expose native preparation requests through the production browser/worker
      runtime with a bounded observation bridge and exclusive DOM ownership.
      Wire cancellation, release, host failure and replacement before publishing
      handles; keep native owners in one realm and diagnostic results separate
      from evidence. Use the synchronous DOM observation realm, with native
      preparation ownership bound to the worker pool's dependency lifetime.
      - [x] Add native staged-session fixtures for bounded source ingress,
        original/composed delivery, capture ordering, policy denial, cancellation
        and permanent failure; retain source owners throughout every stage.
      - [x] Add real WASM/browser fixtures through the shared production host,
        covering snapshot import flattening, complete DOM/CSS checks, exclusive
        ownership, disposal/reentry,
        pool closure/replacement, fresh recovery and incomplete-resource blockers.
      - [x] Bind imported stylesheet resolver-context markers to explicitly
        selected native produced hosts; fixture scope/island checks, missing or
        conflicting bindings, source preservation and actual imported-rule effect.
      - [x] Verify delayed initial inline-style load notifications preserve a
        record only after full revalidation; later/synthetic loads still revoke.

      Implemented the native `RecordedOutputSession` staged source/delivery
      contract, its QL WASM adapter, and the shared browser
      `captureBrowserRecordedOutput` / `pool.captureRecordedOutput` APIs.
      Source-acquisition envelopes enter native import once (4 MiB, at most 256
      sources/choices and 10,000 observed nodes); original/composed delivery,
      native base freezes and final capture retain the same semantic/source owners.
      Scalar predicates reuse shared DOM correspondence; HTML-base and whole-output
      hosts share exclusive document ownership. Callback/reentry/DOM/CSSOM/policy
      failures, disposal, abort, pool failure and generation replacement revoke
      evidence. Native cancellation shares the original execution signal.
      Context-sensitive styles require explicit native host paths; composition
      checks private tags/shared membership and instance-island structure, then
      binds native resolver-context markers without changing original sources.
      Missing, foreign or conflicting contexts reject. Initial trusted inline
      style load notifications preserve a capture only after full native reread;
      later/synthetic loads and errors revoke it.
      Diagnostic blocker summaries and HTML delivery never carry a native seal.
      Initial production scope: standard-agreement UTF-8 HTTP(S) output, immutable
      lexical contexts, and flattened untitled inline stylesheet installations
      without outer media, shadow/adopted sheets or nonempty inert templates.
      The native owner remains in the synchronous DOM observation realm; this
      does not move seals into dedicated workers or their result messages.
      Verification: 54 focused QL native tests pass (six new staged-session and
      three host-binding cases). All 22 production Chromium cases pass, including
      actual imported CSS application, delayed initial loads, ownership/reentry,
      cancellation and pool replacement/recovery. The full Nx browser gate passes
      all prior live-native output, HTML-base, stylesheet/SVG, worker and command
      regressions. TypeScript/WASM builds, CLI lint, focused Rust formatting and
      `git diff --check` pass.
      CEM review: native source ownership and shared services remain authoritative,
      with bounded inputs and existing private/shared scope semantics. No component
      behavior, public token defaults, inherited overrides, accessibility or theme
      ownership changes; theme/forced-color checks are inapplicable and physical AT
      acceptance remains independent. Resource closure and atomic admission stay
      blocked.
    - [x] Extend whole-output preparation observations to external/titled and
      outer-media document stylesheets, reusing native acquisition/selection
      records and controlled recapture without relaxing complete DOM ownership.
      - [x] Add native staged source/selection fixtures for external, titled and
        outer-media roots, missing acquisition, and fail-closed lifecycle.
      - [x] Add production Chromium fixtures for acquired imports, title selection,
        media observations, failed/pending loads, cancellation and fresh recapture.

      Implemented `captureBrowserRecordedOutputAsync` and the pool equivalent:
      exclusive ownership spans initial external loads, native title commands,
      fresh selected-link installations and final native capture. The HTML
      installer stays synchronous. A bounded whole deadline (1–30000 ms, default
      5000), cancellation, errors, policy/DOM drift and pool lifetime changes
      prevent publication. External roots and imported occurrences reuse native
      acquired-source projections and the shared CSSOM observer, including final
      URLs, source comparison, import conditions and outer media. Media listeners
      and orientation/viewport changes revoke evidence. Exact external titles
      activate eligible roots; an empty title disables all titled roots, including
      inline ones. Commands preserve persistent sheets and never activate
      authored-disabled roots. Captured records cannot be reused for selection:
      controlled changes revoke first and fresh sessions perform complete
      delivery/base/source/DOM validation.

      Verification: 57 focused native tests pass (three new staged acquisition/
      selection cases). All 37 production Chromium preparation cases pass,
      including actual imported CSS effect, active preferred/alternate titles,
      disabled titles, fresh recapture, screen/print media, source mismatch,
      missing acquisition, load failure, timeout, abort and pending pool closure.
      The full Nx browser gate passes all existing stylesheet/selection,
      compiled/live-native output, HTML-base, worker and command regressions.
      TypeScript and WASM builds, CLI lint, focused Rust formatting and
      `git diff --check` pass.
      Limits: readable UTF-8 external CSSOM and loaded roots remain required;
      active inline titled sheets, shadow/adopted styles and nonempty inert
      template correspondence remain unsupported. Native owners/seals remain in
      the DOM observation realm. Remaining runtime resources and atomic host
      transaction admission retain their blockers.
      CEM review: shared native acquisition/selection contracts remain authoritative;
      bounded inputs and exclusive ownership preserve semantic intent and
      lifecycle guarantees. Native private/shared declaration scoping remains
      verified by real browser effects. No component behavior, public token
      definitions, inherited overrides, accessibility or theme ownership changed;
      theme/forced-color checks are inapplicable to these observation services.
      Physical browser/device/AT acceptance stays independently open.
    - [x] Record bounded responsive image-source selection and acquisition.
      - [x] Fixture native image candidate ownership, picture ancestry, selected URL,
        acquisition bounds, cancellation and permanent stale-record rejection.
      - [x] Fixture production browser srcset/picture/sizes selection, decoded
        loading, bounded acquisition, mutation, failure and pool cancellation.

      Implemented shared native `ImagePlan`/`ImageRecord` ownership and the
      staged QL/WASM byte ingress. Candidate analysis reuses the native retained
      srcset importer and confirmed execution base, preserving img/picture
      ownership and excluding inert templates. Live selected URLs, completion
      and natural dimensions are checked before/after acquisition and every
      preparation read. Async browser capture waits for decoding, streams distinct
      same-origin raster responses within a shared 4 MiB/deadline budget, and
      retains bytes plus SHA-256 revision and response metadata natively.
      `imageAcquisitions` is a diagnostic count, never an execution grant.
      Source/DOM/media/DPR drift, errors, timeout, abort, pool closure and native
      output invalidation revoke records. Callbacks use the existing reentry and
      deferred-disposal guards.
      Verification: 62 focused native tests pass (four new ML image cases and one
      new QL staged-lifetime case). All 52 production Chromium preparation cases
      pass, including 15 image cases for src/density/width/picture selection,
      decoding, media invalidation, byte bounds, redirect rejection, acquisition
      failure, timeout, mutation, abort, pool closure and synchronous rejection.
      The full Nx browser regression gate, TypeScript/WASM builds, CLI lint,
      focused Rust formatting and `git diff --check` pass.

      This bounded profile records a separate reacquisition, not proof of the
      browser's original response bytes or the winning occurrence when candidates
      share a URL. `image-browser-response-identity-required` and existing runtime
      closure blockers remain. Redirects, cross-origin/custom request policies,
      SVG images, preloads and unresolved lazy/failed loads stay outside this
      profile; the next item closes response identity before broader admission.
      CEM review: the shared native importer, source ownership, bounded variation
      and existing private/shared scoping remain authoritative. No component
      behavior, public token defaults, inherited overrides, accessible names or
      theme/forced-color behavior is changed by the observation service; visual
      theme checks are inapplicable. Physical browser/device/AT acceptance remains
      independent.
    - [x] Bind selected image responses and disambiguate repeated source
      occurrences within the explicit new-delivery replay profile below.
      - [x] Fixture retained candidate occurrences and reject ambiguous selected URLs,
        including equivalent relative URLs, picture sources and src fallbacks.
      - [x] Fixture Chromium duplicate selection rejection and changed reacquisition
        bytes without promoting them to original-response evidence.

      Native candidates now retain owner/attribute/ordinal and parsed descriptor
      ranges before URL deduplication. The bounded currentSrc profile rejects a
      selected URL with multiple authored occurrences, including src fallbacks
      and resolved relative-URL aliases; it does not guess browser source choice.
      Verification: all 64 focused native tests (six ML image, 43 QL template CSS,
      11 execution-record and four installation cases), all 56 production Chromium
      preparation cases, the full Nx browser regression gate, CLI lint, builds,
      focused rustfmt and diff checks pass. The browser fixture now waits for initial
      iframe layout before capture; earlier runs hit delayed viewport invalidation.
      Native tests assert the ambiguity diagnostic; browser tests assert rejection
      because disposal can mask that diagnostic with cancellation.
      Observation-only modes retain their original-response identity blocker:
      reacquired hashes, sizes, dimensions and validators cannot recover a missing
      body observation. The explicit replay profile below binds consumers for a
      new delivery; it never claims historical proof for unchanged authored URLs.
      CEM review: retained native source ownership and bounded variation govern
      rejection; no app/component workaround, public token or theme behavior
      changes. Visual/theme/forced-color checks do not apply to this service.
    - [x] Capture image bodies before browser delivery through an opt-in shared
      transport and bind the delivery ledger to native execution cancellation.
      - [x] Fixture native receipt bounds, duplicate-body consistency, seal ordering,
        selected-response joins and cancellation before acknowledgement.
      - [x] Fixture production service-worker delivery, selected-body ingress,
        failures, abort/disposal and rejection of missing delivery receipts.

      Implemented execution-owned `ImageDeliveryCapture`, staged QL/WASM ingress
      and an opt-in shared service-worker transport. The worker buffers a direct
      same-origin raster response and waits for synchronous native acceptance
      before releasing that exact decoded body. The native ledger computes
      SHA-256 and enforces 256 responses/4 MiB, same-origin ownership, consistency,
      stage ordering and execution cancellation. Selected images join sealed
      receipts without a second fetch; missing receipts reject. Diagnostic
      `imageDeliveries` does not grant transaction admission. Worker registration
      remains explicitly application-owned, with 32 bounded client sessions and
      the existing maximum 30-second preparation deadline.
      Verification: 68 focused native tests pass (eight ML image, 45 QL template
      CSS, 11 execution-record and four installation cases). All 68 production
      Chromium preparation cases and the full Nx browser regression gate pass.
      New cases cover pre-delivery acknowledgement, a single network fetch shared
      across image consumers, empty documents, missing controller/receipts,
      redirects, byte limits, abort/pool closure, controller invalidation and
      synchronous/native rejection without pixel delivery. CLI/WASM builds, lint,
      focused rustfmt and diff checks pass. Server fixtures distinguish first and
      second network requests directly, since service-worker fetch destination
      metadata differs from an image-loader request.
      Limits: available-image caches may bypass interception, so receipts prove
      intercepted deliveries, not every img consumer's actual historical response.
      The plain service-worker mode keeps `image-browser-response-identity-required`;
      the separate replay profile below supplies cache-safe per-consumer binding. Redirects, cross-origin/custom policies, SVG and broader
      runtime loads stay outside the profile. Original HTTP header/wire encoding
      identity is not asserted by reconstructed decoded-body responses.
      CEM audit: shared transport and native retained source/lifetime owners govern
      the feature; no app/component UI workaround, public token defaults, inherited
      override, accessibility or theme behavior changed. Visual/forced-color
      checks are inapplicable to this resource service. Deferred presentation and
      physical browser/device/AT work remain preserved.
    - [x] Bind selected image consumers through an explicit native replay URL profile.
      - [x] Fixture native candidate-to-route provenance, descriptor-preserving
        replacement, source ownership, bounds and independent execution nonces.
      - [x] Fixture Chromium cache reuse, duplicate source candidates, replay
        receipts, stale/foreign routes and lifetime invalidation.

      Implemented `service-worker-replay`: a trusted worker generates a fresh
      execution nonce, native `ImageReplayPlan` retains candidate/source provenance,
      and native render-plan composition replaces URL spans with at most 256
      same-origin reserved routes. The composed output retains the replay owner.
      The adapter installs native route pairs before delivery; selected aliases
      join native receipts and immutable per-route bodies, so same-execution cache
      reuse cannot substitute an older authored-URL response. Duplicate srcset and
      picture source URLs retain distinct occurrence identities. Stale/foreign
      reserved routes reject without network fallback. Only the image identity
      blocker is removed after the replay receipt join; broader admission remains
      blocked. Unchanged-URL modes preserve their prior limitations.
      Verification: 71 focused native tests pass (10 ML image, 46 QL template CSS,
      11 execution-record and four installation tests). All 73 production Chromium
      preparation cases and the full Nx browser gate pass. Replay fixtures warm
      the source cache, serve different subsequent bytes, cover duplicate density
      and picture candidates plus shared-source consumers, and require fresh
      routes on the next execution. Builds, lint, rustfmt and diff checks pass.
      CEM review: native source ownership, bounded variation and shared transport
      remain authoritative; no component behavior or public token definitions
      change. Replay is explicitly a new delivery and changes reflected URLs;
      URL-attribute CSS selectors and cache-dependent image choices may differ.
      Callers needing unchanged URL reflection must use observation-only modes.
      This is not a historical-capture claim or a theme override. No styling was
      added, so visual/theme/forced-color checks are inapplicable; physical
      browser/device/AT acceptance stays independent.
    - [x] Add native fixtures and a bounded computed SVG presentation observation
      contract: retain source owners, reject unstable or foreign observations, and
      permanently revoke changed evidence without claiming fetched-resource identity.
      `SvgComputedHost` retains exact document/attribute/CSS owners, compares two
      capture passes, checks environment revisions and revokes on validation failure.
      Limits: 256 unique slots from one document and 4096 bytes per value/revision.
      Eight new native fixtures plus five retained SVG and five stylesheet execution
      tests pass; focused rustfmt and diff checks pass. Browser integration is pending;
      winning-source, acquired-body and general-runtime coverage blockers remain.
      CEM audit: shared native ownership and bounded inputs are preserved. No UI,
      public tokens, inherited theme values or scoped CSS are changed; visual and
      forced-color checks do not apply to this native evidence-only increment.
    - [x] Add native session fixtures and wire computed SVG observations into the shared browser/native preparation
      lifetime with exact element correspondence, style/environment invalidation,
      before/after read validation and Chromium cascade/owner-transition fixtures.
      Include native namespace-consumption and Chromium SVG declaration parity
      fixtures, rejecting nonmatching namespace declarations.
      Native sessions retain source owners and computed evidence through both
      browser capture paths; reads validate before and after native whole-output
      checks. Static SVG observation rejects animations/SMIL and revokes on owner,
      CSSOM, input-state, preference and session/pool lifetime changes. SVG XMLNS
      delivery equivalence is limited to the exact SVG namespace on SVG roots.
      Verification: 62 native tests pass (nine computed SVG, five SVG import and
      48 QL template CSS tests); all 84 production Chromium preparation cases and
      the full Nx browser gate pass. Builds, lint, focused rustfmt and diff checks
      pass. Eleven new browser cases cover scoped cascade overrides, both capture
      paths, pool closure, owner/CSSOM/focus/animation changes, SMIL rejection,
      preference changes, namespace mismatch and post-read value changes.
      CEM audit: shared native evidence and scoped stylesheet ownership remain
      authoritative; no component-local behavior, public tokens or theme defaults
      change. The preference fixture verifies invalidation, not visual forced-color
      acceptance. Winning-source/resource-body evidence and general runtime closure
      remain blocked; physical browser/device/AT acceptance remains independent.
    - [x] Define and implement bounded local SVG paint-server dependency traversal, starting with
      native fixtures for fragment ownership, fallback, cycles and external edges.
      Keep computed values separate from winning-source and acquired-body evidence.
      `SvgPaintPlan` retains the original HTML inventory and native presentation/CSS
      owners. Fill/stroke fragment candidates resolve through document-wide IDs;
      missing, duplicate, wrong-kind, encoded and non-fragment references remain
      explicit. Gradient/pattern template links retain exact href owners and modern
      href precedence, terminate cycles and require document/base agreement before
      following fragment hrefs. Fallback syntax remains in the retained property;
      no rendered fallback, effective inheritance or resource consumption is inferred.
      Construction caps roots at 256, template depth at 64 and work at one million
      units; exhaustion publishes no partial plan. Eleven new native fixtures cover
      these boundaries, source ownership and stale inventory validation.
      Verification: all 42 focused native tests across seven SVG/stylesheet suites
      pass, plus rustfmt and diff checks. Browser/WASM integration is pending; this
      native-only step does not change browser behavior or admission.
      CEM audit: shared native source ownership, bounded variation and inherited
      theme semantics remain intact. Paint-server inheritance is not replaced with
      consumer-side styling. No UI, public tokens or visual styling change, so
      visual/forced-color checks are inapplicable; physical AT acceptance remains
      independent. Contract: docs/cem-studio.md#local-svg-paint-server-dependency-plans.
    - [x] Add native session lifecycle fixtures, bind local paint-server plans to browser preparation lifetimes and add
      Chromium parity fixtures for fragment targets, template href/base behavior,
      duplicates, cycles and target mutation before consuming these candidates.
      Plans now retain the exact inventory passed into whole-output preparation
      and remain private to the native session. Publication follows validated
      capture; every read validates the original document/base/resolver/style read
      set before exposing bounded root/link counts and unresolved-edge diagnostics.
      Failed capture, target/link/base mutation, cancellation and pool closure
      release the plan. No AST, graph or evidence seal crosses the WASM boundary.
      Verification: 75 focused native tests pass (50 QL preparation/template CSS,
      11 paint traversal, nine computed SVG and five SVG import). All 97 production
      Chromium preparation cases, including 13 new paint cases, and the full Nx
      browser gate pass. CLI/WASM builds, lint, focused rustfmt and diff checks pass.
      Chromium parity covers candidate lookup, template href and computed
      fragment/fallback serialization; duplicates deliberately remain ambiguous
      despite browser first-match lookup. This is not pixel/rendered-paint parity
      or acquired-body evidence. Rendering, content-dependency and admission
      blockers remain. CEM audit: shared native ownership and bounded variation
      are preserved; no component behavior, public tokens, inherited styles or
      visual/theme defaults change. Visual/forced-color and physical AT acceptance
      remain independent of these candidate/lifetime checks.
    - [x] Define retained paint-content dependencies and inherited-style provenance
      for gradient stops and pattern content, starting with native fixtures; preserve
      rendering and acquired-body blockers until the relevant evidence is joined.
      Added `SvgPaintContentPlan`, retaining the native paint plan and distinct
      candidate servers, source-order content nodes, direct SVG gradient stops,
      direct URI attribute owners and lexical source ancestry. Own/template content
      stays separate; no effective content, clone inheritance or computed value is
      inferred. Bounds: 256 servers, 4096 node memberships, 128 ancestors per node
      and a caller work budget capped at one million units, with atomic failure.
      Eight new fixtures cover ancestry, template cycles/shared roots, stop ownership,
      pattern content/URI provenance, unknown targets, external template boundaries,
      budget exhaustion and stale inventory validation. All 33 focused native SVG
      tests pass, plus rustfmt and diff checks. Browser/WASM integration is pending;
      the existing browser preparation path is unchanged by this native-only step.
      CEM audit: source ownership and inherited-style semantics stay authoritative;
      consumer styles are not substituted for paint-server ancestry. No component
      behavior, public tokens or theme defaults change. Visual/forced-color and
      physical AT acceptance remain independent. Contract:
      docs/cem-studio.md#retained-svg-paint-content-and-source-ancestry.
    - [x] Bind retained paint-content plans to browser preparation, with native
      lifecycle and Chromium fixtures for stop/pattern/ancestor-style mutations;
      keep clone-style and rendered-resource evidence separate.
      Fixture scope: native atomic publication/failed capture/stale read/close;
      Chromium stop value restoration and replacement, pattern text restoration
      and child reattachment, ancestor style/class restoration and pool closure.
      Completed: content plans share the native traversal/inventory and publish
      only after full preparation validation; failed capture, stale reads and
      closure release both plans. Browser summaries expose bounded server/node
      counts only. All 50 native preparation and 33 focused SVG tests pass;
      WASM/CLI builds, CLI lint and the full Chromium gate pass, including 104
      production preparation cases (seven new content cases). Rustfmt and diff
      checks pass. CEM audit: lexical source ownership, bounded variation and
      inherited theme provenance remain native; no component behavior, theme
      defaults or public tokens change. Clone-style evaluation, rendered-resource
      evidence and physical/forced-color/AT acceptance remain separate.
    - [x] Define cloned paint-template content style provenance, with native
      fixtures distinguishing source ancestry from instantiated inheritance.
      Fixture scope: shared source/distinct hosts, own/descriptive-only/indirect
      content, pattern instance parents, cycles, unresolved/empty targets,
      namespace-sensitive descriptive elements, atomic bounds and stale inventory.
      Completed: native SvgPaintStylePlan retains the original content plan,
      separates source ancestry from receiving-host inheritance, records per-host
      selection/link provenance and prospective instance parent identities.
      Own content stops selection; descriptive-only chains select source children;
      per-host cycle checks preserve valid content on cyclic template graphs.
      Bounds: 256 hosts, 64 selection edges, 4096 aggregate instance memberships,
      one-million-unit work cap. Nine new fixtures and all 42 focused native SVG
      tests pass; focused rustfmt and diff checks pass. CEM audit: original source
      and theme ownership remain intact, shared templates preserve distinct host
      inheritance, and no copied computed style or consumer ancestry substitutes
      for instance evaluation. No component behavior or public tokens change.
      Browser integration, style evaluation and rendered-resource evidence remain
      pending; visual/forced-color and physical AT acceptance remain independent.
      Contract: docs/cem-studio.md#cloned-paint-template-style-provenance.
    - [x] Bind clone-style provenance plans to browser preparation and verify
      mutation lifetimes and SVG 2/Chromium compatibility before treating source
      observations as instance evidence.
      Fixture scope: native atomic publication/revocation of style plans;
      Chromium source/host style and template-link mutations, pool closure,
      and isolated SVG raster probes for own, inherited and descriptive content.
      Completed: the native session constructs the style plan from its retained
      content plan before publication and releases it with failed/stale/closed
      preparation. Only bounded host/instance counts cross the WASM boundary.
      51 native preparation and 42 focused SVG tests pass; WASM/CLI builds, lint,
      rustfmt/diff checks and the full Chromium gate pass, including 115 production
      preparation cases (11 new). An overlapping build initially collided on
      generated declarations; the sequential browser rerun passed.
      Chromium 148 compatibility evidence: inherited gradient/pattern content
      uses source color; own children use host color; descriptive-only gradients
      inherit content while descriptive-only patterns render transparent.
      Isolated raster probes are regression evidence, not execution receipts.
      SVG 2 structural provenance remains separate from browser behavior;
      compatibility/style-evaluation/resource blockers stay explicit.
      CEM audit: source/theme ownership and bounded host contexts are preserved;
      consumer styles never replace paint-server ancestry. No UI behavior, theme
      defaults or tokens change. Forced-color/visual and physical AT acceptance
      remain independent.
    - [x] Define an explicit clone-style browser compatibility profile covering
      inherited colors and descriptive-only patterns before evaluating instances.
      Fixture scope: exact browser identity; six named solid-currentColor probes;
      source/host color context checks; duplicate, missing and mismatched probes;
      unknown versions and preservation of evaluation/resource blockers.
      Completed: native SvgPaintCloneProfile pins Chromium 148.0.7778.96 with
      no default, alias or version fallback. It validates exactly six named
      reports, including source/host computed colors and pixel channels; missing,
      duplicate, extra or mismatched reports fail without latched acceptance.
      This fixed-size compatibility catalog does not generalize source-color
      behavior into arbitrary CSS rules, change the SVG 2 structural plan, or
      grant execution authority. Six new fixtures and all 48 focused native SVG
      tests pass; rustfmt/diff checks pass. Browser behavior is unchanged in this
      native-only step; the previously verified 115-case gate remains prior
      integration evidence, not a new run. CEM audit: original source/theme
      ownership and bounded variation are preserved; no component behavior,
      token defaults or source dependency pruning changes. Live profile binding,
      instance evaluation and rendered-resource evidence remain pending.
      Contract: docs/cem-studio.md#explicit-clone-style-browser-compatibility-profile.
    - [x] Wire explicit clone-profile selection and complete native probe report
      validation into browser preparation; reject unknown identities/mismatches
      while preserving source, instance and execution-evidence boundaries.
      Fixture scope: native explicit selection, absent/unknown/repeated profile,
      mismatched probe and stale-session rejection; browser actual six-case
      reports, malformed scalar ingress, sync/async capture and mutation/pool
      invalidation without converting compatibility reports into execution seals.
      Completed: optional explicit cloneProfile name/callback feeds six native
      requests through a twelve-byte scalar report boundary. The session checks
      its exact browser identity and complete probe set before capture; unknown,
      repeated, out-of-order, malformed and mismatched confirmations fail closed.
      Quiet-state checks bracket callbacks. Both capture paths publish only the
      svgCloneProfileConfirmed diagnostic; original lifetime validation remains
      authoritative and stale/closed preparations clear confirmation.
      All 53 native preparation and 48 focused SVG tests pass; WASM/CLI builds,
      CLI lint, rustfmt/diff checks and the full Chromium gate pass, including
      124 production preparation cases (nine new report-binding cases).
      CEM audit: bounded native source/theme ownership remains intact; reports
      never replace retained instance provenance or authorize copied computed
      styles. No component behavior, defaults or tokens change. Compatibility,
      style-evaluation and execution-evidence blockers remain; actual instance
      styles and consumed resources still need separate evidence. Physical
      device/AT and visual/forced-color acceptance remain independent.
    - [x] Evaluate the first bounded paint-instance style subset with retained
      declaration/inheritance provenance under the selected profile; full-cascade
      winning-source evidence remains a separate requirement.
      Fixture scope: explicit named/hex colors, currentColor declaration ancestry,
      own versus inherited flat gradient/pattern content, descriptive-pattern
      suppression, unsupported cascade/content/value rejection, work bounds and
      original inventory lifetime.
      Completed: native BoundedPaintColors retains the style plan and selected
      profile, derives flat stop-color/fill values from six named colors,
      three/six-digit hex and currentColor, and records paint/color declaration
      IDs plus consulted ancestors. Own and single-hop same-kind templates are
      bounded; a title-only pattern host is recorded as suppressed without
      dropping source dependencies. Competing CSS/dynamic content, unresolved
      values, missing inherited color and unsupported shapes fail atomically.
      All 57 focused native SVG tests pass, including nine new evaluator fixtures;
      focused rustfmt and diff checks pass. No browser/WASM integration changed
      in this native-only step. CEM audit: semantic source/theme ownership stays
      native, consumer ancestry never substitutes for paint-server ancestry, and
      no defaults or tokens change. These are authored-color derivations, not
      certified cascade winners or rendered pixels; full-cascade, forced-color,
      execution and resource evidence remain open.
      Contract: docs/cem-studio.md#bounded-paint-instance-color-derivations.
    - [x] Bind bounded color derivations to browser preparation and add Chromium
      parity/mutation fixtures for literal colors and currentColor provenance.
      Fixture scope: explicit evaluator opt-in requiring confirmed profile,
      atomic failure and revocation; native-to-CSSOM color comparison; Chromium
      literal/currentColor/transparent and suppressed-pattern raster parity,
      declaration/ancestor mutations and unsupported author-cascade rejection.
      Completed: explicit boundedPaintColors opt-in requires confirmed clone
      profile. Native derivations retain the same style/content/inventory before
      publication; unsupported evaluation fails capture. WASM validates source
      fill/stop-color CSSOM against native values after capture and around reads.
      Mismatch, invalidation and closure release derivations; only bounded color
      and suppressed-host counts cross the summary boundary.
      All 55 native preparation and 57 focused SVG tests pass; WASM/CLI builds,
      lint, rustfmt/diff checks and the full Chromium gate pass, including 140
      production preparation cases (16 new). Raster fixtures verify literals,
      hex, source currentColor, forwarding/local overrides, transparent paint,
      patterns and descriptive suppression. Restored source/host/declaration
      mutations, replacement and pool closure revoke preparation.
      CEM audit: inherited/source ownership stays native; consumer ancestry and
      copied CSSOM values do not replace declarations. No component behavior,
      public tokens or theme defaults change. Source CSSOM agreement and isolated
      raster parity do not establish full-cascade winners, browser-private clone
      identity or consumed bytes. Those blockers remain; general forced-color,
      physical-device and AT acceptance remain independent.
    - [x] Add bounded inline color declarations and priority over presentation
      attributes, with native fixtures for duplicates, important priority,
      currentColor inheritance, retained CSS winner provenance and rejected
      unsupported declarations; verify Chromium parity and style mutations.
      Completed: evaluator consumes existing retained inline CSS trees for SVG
      color/fill/stop-color. Inline declarations beat presentation attributes;
      importance precedes source order among duplicates. Inheritance consults
      local winners before ancestors, without carrying ancestor importance to
      children. Winners retain document attribute, CSS tree/declaration identity,
      source order and importance; CSSOM validation uses an explicit property.
      Unsupported declarations fail atomically, even when losing. Total inline
      declarations are bounded at 4096 under the existing work budget.
      All 55 preparation and 63 focused SVG tests pass, including six new native
      fixtures. WASM/CLI builds, CLI lint, rustfmt/diff checks and full Chromium
      gate pass, including 149 preparation cases (nine new inline cases).
      CSSOM/raster checks cover overrides, duplicates, importance, inline-only
      paint and inheritance; restored style/priority mutations revoke preparation.
      CEM audit: original declaration/theme ownership and inherited overrides
      remain native; no copied CSSOM value substitutes for winner provenance.
      No component behavior, theme defaults or public tokens change. Stylesheet
      cascade, user/UA/environment overrides, general forced-color/AT acceptance
      and rendered-resource evidence remain open.
      Contract: docs/cem-studio.md#inline-paint-color-declarations-and-priority.
    - [x] Define bounded stylesheet matching and cascade precedence with retained
      selector/declaration provenance.
      Completed design: SVG style elements, single-compound type/universal/ID/
      class selector lists, matched-branch specificity and unlayered author
      priority. Native tagged origins distinguish presentation/inline/rule
      winners; document order, bounds, atomic rejection and browser installation/
      mutation obligations are explicit. Reviewed against retained CSS import
      shape, current color evaluator and W3C selector/cascade contracts.
      Documentation-only: runtime still rejects style elements; no new fixture
      or browser acceptance is claimed. CEM audit preserves source/theme
      ownership, bounded variation and inherited overrides; no defaults/tokens
      change. Full-cascade, forced-color and execution/resource blockers remain.
      Contract: docs/cem-studio.md#bounded-stylesheet-matching-and-paint-cascade-contract.
    - [x] Implement the bounded native stylesheet matcher and tagged winner
      origins, starting with fixtures for compound/list matching, namespace/case/
      escapes, matched specificity, importance/inline/presentation priority,
      cross-sheet order, inheritance, source identity, rejected syntax and bounds.
      Completed: native retained CSS compounds/lists match SVG names, IDs and
      ASCII-whitespace class tokens. Matched specificity, importance, inline
      attachment and document/rule/declaration order determine local winners.
      Tagged origins retain document owner, CSS tree/declaration, matching
      selectors and rule priority; inheritance preserves source/host identity.
      Unsupported rules/properties/values fail atomically, including losers and
      nonmatches. Aggregate sheet/rule/selector/declaration caps share the work
      budget; fixture inputs span small sheets to test beyond per-import caps.
      All 72 focused native SVG tests pass (nine new cascade fixtures), plus all
      56 preparation tests including a new explicit browser installation gate.
      WASM/CLI builds, focused rustfmt/diff checks and the full Chromium gate
      pass, including all 149 existing preparation cases. Browser preparation
      still rejects stylesheet colors pending installation/mutation binding.
      CEM audit: retained semantic/source/theme ownership and inherited overrides
      remain intact; no UI behavior, public tokens or component defaults change.
      Full-cascade, forced-color/AT, clone-execution and resource blockers remain.
    - [x] Bind stylesheet color plans to installed browser preparation; add
      Chromium CSSOM/raster parity and restored class/ID/rule/priority/disablement/
      replacement mutations, cancellation and worker/pool lifetime fixtures.
      Cover SVG-native default media through namespace-correct CSSOM parsing.
      Completed: native capture/read joins every admitted sheet (including
      nonmatching rules) to the same retained CSS tree, source key, exact document
      owner and installed enabled/applicable root. Missing/inactive or changed
      observations fail atomically and revoke the color plan with preparation.
      Shared CSSOM comparison preserves SVG namespace and absent/empty media;
      comparison-node cleanup includes SVG roots. No selector/winner evidence
      leaves the native owner or is reconstructed from browser values.
      All 57 native preparation and 72 focused SVG tests pass. Chromium verifies
      172 preparation cases (23 new): gradient/pattern CSSOM and raster parity,
      specificity/list/importance/inline/document-order/inheritance selection,
      DOM changes, controlled CSSOM change-and-restore, raw CSSOM drift, disabled
      nonmatching sheets, tampering, unsupported media, abort and pool closure.
      WASM/CLI builds, CLI lint, focused rustfmt/diff checks and the complete
      Chromium regression gate pass.
      CEM audit: inherited overrides and original semantic/source/theme ownership
      stay native; no UI behavior, defaults or public tokens change. Unreported
      CSSOM history, full-cascade/forced-color handling, hidden clone identity,
      actual rendered-resource bytes and physical-device/AT acceptance remain
      separate. CSSOM restoration requires the existing exclusive-owner change
      boundary; final equality alone cannot establish mutation history.
    - [ ] Extend instance-style evaluation beyond the bounded author-rule subset
      with retained cascade/winning-declaration evidence; do not infer winners
      from computed values or compatibility reports.
    - [x] Bind bounded computed SVG fill/stroke URL references to immutable
      acquired SVG snapshots with native fixtures for CSS/reference provenance,
      fragment/body identity, shared bodies, missing/conflicting/unused responses,
      rejected values/MIME/redirects/XML, bounds, cancellation, stale/foreign
      computed owners and change-and-restore revocation. This first native step
      must retain browser-response identity and consumed-resource blockers.
      Completed: native SvgResourceRecord validates its original computed host,
      imports observed CSS values through shared retained CSS, and joins one
      absolute HTTP(S) URL per fill/stroke slot to immutable acquired SVG entity
      bytes. Fragment/slot/source identities survive body deduplication. Bodies
      retain native digests and shared-import SVG trees; missing/duplicate/unused
      responses, redirects, unsupported values/MIME/XML and bounds fail atomically.
      Validation drift, cancellation and close release bindings irreversibly.
      All 80 focused native SVG tests pass (eight new acquisition fixtures),
      alongside all 57 preparation tests. WASM/CLI builds, focused rustfmt/diff
      checks and the full Chromium regression gate pass, including all 172
      existing preparation cases. CEM audit preserves native semantic/
      source/theme ownership and bounded variation; no UI/default/token changes.
      This is an acquired-snapshot join, not a browser delivery receipt or proof
      of painting. Preparation integration, response identity, fragment targets,
      transitive resources, broader CSS selection and general closure stay open.
      Contract: docs/cem-studio.md#computed-svg-url-bindings-to-acquired-snapshots.
    - [x] Add native preparation fixtures for acquired SVG snapshot binding,
      stage/repetition rejection, acquisition cancellation, computed drift and
      enclosing preparation failure; retain response/consumption blockers.
      Completed: native RecordedOutputSession retains the resource record and
      original computed evidence, checks independent acquisition cancellation
      around staged operations, revalidates bindings through validate_svg, and
      releases them on close or validation failure. Counts publish only after
      capture. All 60 preparation tests (three new lifecycle fixtures) and all
      80 focused SVG tests pass. WASM/CLI builds, focused rustfmt/diff checks
      and the full Chromium gate pass (172 existing preparation cases).
      CEM audit preserves native source/theme
      ownership and bounded variation; no UI/default/public-token changes.
      Browser ingress and response identity remain open; this native stage does
      not claim consumed bytes or complete preparation/transaction admission.
    - [x] Add explicit WASM/browser SVG snapshot ingress and Chromium fixtures
      for sync/async capture, immutable copies, cancellation, mutations, bounds,
      malformed inputs and worker lifetime; keep consumption blockers.
      Completed: explicit svgSnapshots ingress copies bounded raw entity bytes
      to WASM before installers run, pins acquisition cancellation, binds after
      computed capture and validates around reads. Native bytes/trees/evidence
      remain private; only svgResourceBindings joins diagnostic summaries.
      Fifteen new Chromium fixtures cover both capture paths, copied buffers,
      abort, restored DOM mutation, worker closure, invalid/empty/duplicate/MIME
      inputs, bounds and unopened/repeated/closed native ingress. All 60 native
      preparation tests and all 187 Chromium preparation cases pass, with the
      full browser gate, WASM/CLI builds, lint and diff checks. CEM audit keeps
      semantic/source/theme ownership native and variation bounded; no UI,
      token/default changes or broader forced-color/device/AT claims.
      Supplied snapshots do not attest to browser responses or painting;
      response identity, consumption and broader closure blockers stay open.
    - [x] Define native SVG delivery evidence with fixtures for exact body joins,
      missing/changed/duplicate receipts, bounds, closed/stale owners and transport
      cancellation. Keep browser response identity blocked until transport wiring.
      Completed: SvgDeliveryCapture owns bounded direct-response inputs and
      rejects duplicate receipts, redirects, invalid MIME, noncanonical URLs,
      credentials, empty/oversized bodies and excess receipts irreversibly.
      SvgResourceRecord consumes the ledger once, joins exact bytes to existing
      retained bodies, validates computed ownership around the join and keeps
      transport cancellation live. Missing/extra/changed receipts and stale
      owners revoke bindings. All 84 focused SVG tests (four new delivery
      fixtures) and all 60 preparation tests pass. WASM/CLI builds, focused
      rustfmt/diff checks and the full Chromium gate pass (187 existing
      preparation cases). CEM audit preserves native
      semantic/source/theme ownership and bounded variation, with no UI or
      public token/default changes. No adapter supplies these native receipts
      yet; browser response identity and consumption remain blocked.
    - [x] Wire bounded SVG image-response transport to native delivery binding;
      add native session and Chromium exact-byte, missing-response, changed-body,
      cancellation and duplicate-request fixtures. Preserve paint-consumption limits.
      Completed: explicit async svgDelivery service-worker mode pins snapshot
      URLs, accepts only same-origin image-destination direct SVG responses,
      awaits native ledger acceptance before release and seals before joining
      exact bytes to snapshots. Native preparation owns the join; only a
      delivered-binding count is public. Direct SVG image load/error listeners
      cover Chromium's non-document load path and clean up on every exit.
      All 61 preparation tests (one new session fixture), six new Chromium
      delivery cases and all 193 preparation browser cases pass; full browser
      regressions, WASM/CLI builds, lint and focused rustfmt/diff checks pass.
      Native duplicate-receipt tests remain the ambiguity rejection contract.
      CEM audit retains native semantic/source/theme ownership and bounded
      variation, with no component defaults, UI or public token changes.
      Exact same-session SVG image response delivery is verified, but cached
      or independent fill/stroke selection is not. Response-identity and
      paint-consumption blockers remain until selection-bound replay and
      paint compatibility are established; no broad device/AT claims.
    - [x] Verify selection-bound SVG replay feasibility with real Chromium
      document pixels, local paint controls, fresh aliases and warm-cache image
      consumers and retargeting. Record the compatibility boundary without
      enabling replay-based paint consumption claims.
      Completed: the pinned Chromium 148.0.7778.96 compatibility gate checks
      twelve live-document screenshot/request cases: gradient and pattern local
      controls, fresh URLs, warm image-cache reuse, fresh aliases, and fresh/alias
      variants without fallback paint. External cases have no local definitions.
      Green alias versus magenta cached-source paint, blue image pixels and four
      retargeting checks distinguish actual consumer selection. Requests use the
      image destination; warm reuse and retargeting issue no second request.
      All twelve probes, all 194 preparation cases (including rejection of
      unimplemented SVG replay admission), the full browser gate, lint and diff
      checks pass. The helper is included in Nx target inputs; observations are
      saved under svgReplayCompatibility in the platform-parity browser report.
      CEM audit: fixture colors only, no UI/public-token/default changes; authored
      source/theme ownership and existing execution blockers remain intact.
      These are compatibility observations, not native replay plans or execution
      seals. Production source/alias/fragment ownership and receipt binding remain
      open; no broader CSS, cross-origin, device or AT acceptance is claimed.
    - [ ] Integrate computed SVG byte bindings with preparation/acquisition
      lifetime and verify browser response identity before claiming consumption.
    - [ ] Bind computed CSS/SVG selections to acquired resource bodies before
      claiming rendered-resource closure.
    - [ ] Record remaining computed CSS/SVG and general runtime resource loads,
      before claiming complete execution dependency closure.
  - [ ] Establish execution-owned lookup provenance for relative run-config inputs,
    including invocation/default origins, before widening relocation admission.
- [ ] **Grouped operation and graph authoring** — extend native-backed configuration
  editing and persistence, using shared CEM controls before app composition.

Next, implement bounded native SVG replay plans retaining authored consumer,
source URL and fragment provenance, then bind fresh aliases to browser delivery
and revocation. The pinned paint compatibility profile is verified; it does not
remove response-identity/consumption blockers or authorize replay by itself. Extend fragment/embedded SVG dependencies and broader CSS selections
afterward. General runtime loads and broader cascade evidence remain required
before complete dependency closure or wider transaction admission.
Selected image response/candidate binding is available through explicit native
replay; unchanged-URL observation deliberately retains its historical-identity
blocker.
Connect the prepared native read set to host transaction admission only after complete
dependency evidence. Combine that dependency closure with map rewrite ownership before
widening Studio admission.
Relative run-config input provenance and grouped operation/graph authoring follow.
Expanded/resizable panes and larger-data controls must follow the same library-first
rule. Physical device/browser/AT
acceptance remains independently open above. Presentation preparation stays deferred
and its existing checklist/resume context is preserved.
