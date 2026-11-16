# Todo

This is the authoritative checklist for unfinished execution work. Product/module
sequencing lives in [roadmap.md](../roadmap.md); wishlist work lives in
[wishlist.md](wishlist.md).

Completed work and verification evidence are preserved in the
[2026-10-31 snapshot](archive/todo-snapshot-2026-10-31.md).
Earlier history remains linked from that snapshot. Historical checkbox states
do not reopen completed work; adopted contracts remain in their design documents.

The latest completion is [CLI admission for external stylesheet imports](archive/todo-completed-ssr-cli-resource-admission-2026-11-09.md).
The [completion index](archive/todo-completed-index-2026-11-09.md) links the
evidence for the retired checklist entries. Current execution is in the
[SSR follow-ups](#browser-and-cli-ssr-follow-ups).

Preserve accepted Studio functionality and presentation samples while addressing
the remaining shared-library and release diagnostics. Examples remain ordinary projects
with resources and configurations in the same workspace, without dedicated
example screens. Follow the [Studio milestone contract](cem-studio.md#next-delivery-milestone-user-owned-project-workspace).

Keep UI declarative in `cem-components` and Studio CEM-ML; reusable JavaScript
behavior belongs in `cem-elements`, and application JavaScript owns non-UI services
only. Alert the user before introducing a new shared JavaScript UI capability.
Apply the [canonical CEM theme principles](../packages/cem-theme/src/lib/tokens/index.md#core-cem-principles),
including inherited token ownership, accessibility and theme survivability.
Entered passwords or one-time codes continue to block recovery until explicitly
cleared by the user; automatic whole-Studio recovery remains disabled. Follow the
[credential privacy policy](committed-checkpoints.md#credential-privacy-policy-for-live-login-recovery).

Completed category indexes: [project builds and artifact previews](archive/todo-snapshot-2026-10-31.md#project-builds-and-artifact-previews),
[content-type/schema samples](archive/todo-snapshot-2026-10-31.md#studio-samples-supported-content-types-and-schemas)
and [Studio delivery](archive/todo-snapshot-2026-10-31.md#cem-studio-delivery-planning).

Preserve the [bounded project lifecycle](cem-studio.md#next-delivery-milestone-user-owned-project-workspace):
100 graph nodes / 1 MiB source and 20 history snapshots / 8 MiB. Authored ID
changes, aliases and expression-valued attributes remain source-only. Keep
credential recovery refusal, the aggregate 4 MiB checkpoint budget and separate
physical-device/spoken AT acceptance.

Amsterdam presentation delivery is accepted and archived. Use the SSR follow-ups
for the current native work and the conditional diagnostic items on recurrence.

## Studio project build command

- [x] Add a **Copy CLI** button on Studio's project page beside **Cancel build**.
      Copy the Bash command equivalent of **Build project** to the clipboard and,
      on the same click, display that command in a `<code>` section immediately
      after the buttons group. Use the current project's actual build configuration
      and shared command rendering, with correct shell quoting and working-folder
      context; keep the UI declarative through shared CEM controls.
  - [x] Verify copied/displayed command parity with project build inputs, correct
        button placement, repeated clicks, project/configuration changes,
        clipboard failure, keyboard access and theme behavior in focused fixtures.
      Verified 2026-11-13: 50 navigation/browser cases pass, including keyboard
      activation, six themes, unavailable/denied clipboard access and stale
      configuration/revision results. Native Bash checks match conversion/query
      stdout and both report/Login graph builds (4/2 generated artifacts);
      derived HTML preserves spaces, apostrophes, `$()` and backticks literally.
      Studio build, typecheck, lint, UI classification and packaged-output checks
      pass. See [the build command contract](cem-studio.md#project-builds-and-artifact-previews).

- [x] In Studio project options, use a radio-button group when there are **7 or
      fewer choices**, and retain the dropdown for **8 or more**. Preserve the
      selected value, labels, disabled states and shared declarative CEM controls;
      handle an empty choice set explicitly.
  - [x] Verify the 0/1/7/8-choice boundaries, project/configuration changes,
        selection preservation, keyboard/accessibility and theme behavior.
  - [x] Add opt-in shared `cem-select` radio-threshold support using `cem-radio`,
        with explicit placeholder markers, form/reset/disabled/required contracts,
        grouped and rich choices, dynamic 7↔8 transitions and unchanged listboxes.
        Cover the shared component in colocated stories/gallery/property controls
        before enabling the threshold throughout Studio's project forms.
  - [x] Preserve staged-form reset and complete Login recovery in worker/fallback
        fixtures under the existing 32,768-node witness and 4 MiB group limits;
        remove incidental formatting nodes rather than raising budgets/deadlines.
        Verify shared tree keyboard/selection behavior after removing repeated
        formatting-only nodes from its recursive row and Studio payload templates.
  - [x] Verify gallery navigation keyboard activation after the produced child
        publishes its update; retain the original toggle and focus assertions.
      Verified 2026-11-13: 37 select/radio/tree stories and all 251 Studio
      workflow/navigation/recovery cases pass. Complete Login recovery preserves
      credential refusal and three worker/fallback handoffs within the unchanged
      32,768-node witness and 4 MiB group limits. Lean rendered option projections
      retain the original canonical payload and omit unused radio/popup metadata;
      compact recursive/declarative markup removes incidental formatting nodes.
      Six-theme/inherited-token stories and a native forced-colors keyboard/form
      check pass. Navigation/content galleries, property pages and bundle pass
      from source and isolated installed packages; the disclosure fixture now
      waits for child publication without weakening toggle/focus assertions.
      Studio build, component catalog, typecheck, lint, declarative architecture, UI classification
      and packaged-output checks pass; existing lint warnings remain.

- [x] Extend Copy CLI to builds with renamed resolver addresses or Studio-only
      URI operands through shared native filesystem export/resolver mapping.
      Preserve stable identities, nested graph/resource resolution and original
      source bytes. Mapped commands use the matching materialized `project.cem`
      through the native CLI's `--resolver-project` option.
  - [x] Verify renamed inputs, nested graph references and explicit Studio URI
        operands against materialized project outputs before enabling copy.
  - [x] Add native filesystem-plan and CLI fixtures for immutable resource-to-path
        mappings, nested relative reads, output publication, literal quoting and
        unresolved/ambiguous addresses; retain exact source bytes and identities.
  - [x] Connect the planner through the native-generated CLI grammar and
        verify Copy CLI revision fencing, clipboard behavior and materialized
        project execution with renamed and explicit Studio-addressed resources.
      Verified 2026-11-13: 11 native CLI integration cases, three filesystem-plan
      cases, the record-preservation case and 13 command-invocation cases pass.
      The npm CLI suite passes 64/64; Studio navigation passes 51/51, with the
      five Copy CLI cases repeated after the final dependency-grouping change.
      Copied Bash commands match native argv for conversion, query and both
      report/Login graph builds (4/2 artifacts), with all materialized sources
      renamed to quoted filenames and their bytes unchanged. Native-only mapping
      arguments are refused by Node/browser WASM hosts before resource reads.
      An aggregate recovery run passed 58/59; the remaining complete Login
      worker case passes its isolated rerun with the original limits. Its fresh
      allocation transport error remains in the conditional diagnostic below.
      Native/Studio builds, generated command-grammar checks, native/npm/Studio
      lint, typecheck, UI classification and packaged-output verification pass;
      existing lint warnings remain. No runtime allocation policy was changed.

- [x] Add shared project-bootstrap configuration and Studio's guided questionnaire,
      with a native producer reusable by editor integrations. **New project** opens
      this flow. Use declarative CEM components to collect project type, resource
      hierarchy and transformation pipeline, then generate the project hierarchy
      and saved runnable configurations through the shared native producer.
      Include presets for a Markdown-based site, transformation pipeline, simple
      conversion, schema-validation project and a minimal custom project; allow
      users to review and adjust generated folders, resources and pipeline stages.
  - [x] Define one versioned bootstrap configuration/schema and native producer
        reusable by all three hosts, with the native CLI control transport and
        Studio's owned browser API; keep editor integrations thin and avoid
        separate scaffolding/semantic implementations.
    - [x] Add native fixtures for all five presets, stable resolver references,
          independent identities, schema/version/path failures and graph closure;
          execute generated commands against materialized projects.
    - [x] Verify the rebuilt browser runtime and
          Studio's owned native bridge without publishing partial projects.
      Verified 2026-11-13: six native producer cases and three native CLI
      integration cases pass, including all five presets materialized with
      original and moved paths (eight runnable commands). Fourteen native/WASM
      response comparisons match, and the emitted Draft 2020-12 JSON Schema
      accepts every preset. Both new Studio browser cases pass for generated
      builds, manifest round trips, invalid commands, cancellation before result
      delivery and retired bindings. The npm CLI suite passes 65/65; its full
      Chromium suite passes, including bootstrap authoring across two generation
      replacements, stale aliases and preparation invalidation. Generation
      performs no workspace writes; host publication and creation UI remain
      below. Runtime/package/Studio builds, lint, Studio typecheck, UI
      classification and both packaged-output checks pass; existing lint
      warnings remain.
  - [x] Build the declarative Studio questionnaire and review step over this
        producer; connect **New project** and publish through the existing
        validated repository transaction only after review/current-state checks.
    - [x] Add native planning fixtures for nested Markdown pages, shared layout,
          navigation, CSS/static publication, paired pipeline source/target trees,
          simple input/output and custom structures; verify builds and invalid
          path/count/collision refusals before rebuilding browser artifacts.
    - [x] Add Studio fixtures for choosing/editing structures, hierarchy/pipeline
          review, cancellation, failed creation, independent identities and
          production-control keyboard/theme behavior before commit and push.
    - [x] Add a shared native-surface fixture for replacing a modal dialog's
          focused step, preserving another focused control, the focus-target=none
          policy and nonmodal focus behavior; repair focus in cem-elements.
      Verified 2026-11-13: six producer, four planning and five native CLI
      integration cases pass. Nested sites build complete HTML with relative
      navigation, shared layouts and byte-exact CSS/static publication; paired
      pipelines, simple YAML/JSON conversion and multiple XHTML inputs build.
      All five new Studio browser cases and 59 navigation/recovery cases pass.
      Creation reviews native source/output trees and graph stages, publishes
      once, and refuses cancelled work, corrupt archives and failed storage.
      Seven shared native-surface stories pass, including deferred/disabled
      replacement controls without stealing surviving or nonmodal focus.
      Six theme modes remain usable; the locally served production questionnaire
      passes a forced-colors keyboard/modal check without browser errors.
      WASM, CLI, Studio and Playground builds, lint, typecheck, UI classification
      and packaged Studio output verification pass; existing lint warnings remain.
  - [x] Verify native/Studio preset/questionnaire transitions, pipeline and path validation,
        generated hierarchy/build parity, cancellation and failed-generation
        atomicity, independent project identities, accessibility and themes.
      VS Code and IntelliJ project-bootstrap adapters are deferred to the
      [roadmap](../roadmap.md#shared-project-bootstrap-and-guided-creation),
      including editor review, cancellation-safe publication and cross-host parity.

- [x] Add **Copy CLI** to the demo Playground, matching Studio's clipboard
      feedback and inline command display. Reuse the shared command grammar
      and literal shell serialization; verify that copied commands reproduce
      the selected conversion without saving the authored source.
  - [x] Add Playground fixtures for current source/format changes, literal Bash
        quoting, clipboard failure, inline code and native execution parity.
      Verified 2026-11-13: all 22 Playground/demo stories and 67 npm CLI tests
      pass. Copied commands use the shared grammar/serializer, literal `printf`
      input and `/dev/stdin` (Bash on Linux/macOS), with no saved source/output.
      Native execution succeeds for YAML→JSON, JSON→CEM-ML, CEM-ML→HTML,
      Markdown→HTML, CSS→CSS and JSON→YAML. The CEMT color profile is explicitly
      disabled for plain exports. Executable Node/WASM fixtures also verify
      piped YAML/JSON and raw CEM-ML HTML, exact shell bytes, Unicode, quotes,
      CRLF, empty/tilde input and refusal of NUL. Browser fixtures verify current
      format/source snapshots, safe code projection after the toolbar, retained
      rendering controls, source preservation and clipboard-denial fallback.

- [x] Assess adding an **Open in separate window** icon button (arrow pointing
      to the upper right) to Studio's **Output preview** heading. Determine
      whether the existing preview artifact URL and resource lifecycle can be
      reused, and record the effort and behavior for opening the current preview.
      Assessed 2026-11-13: the header control is small, but there is no standalone
      preview URL. Realm-local trees and frame-owned blobs are revoked with their
      current run/view. Recommend a read-only passive snapshot with an independent
      viewer-owned registration; shared snapshot/handoff/viewer support is medium
      scope. Interactive live/application detachment is a separate larger task.
      See [the assessment](cem-studio.md#separate-preview-window-assessment).

- [x] Implement **Open in separate window** for the current passive Output preview
      using the assessed snapshot contract, without rerunning or saving a build.
  - [x] Add shared bounded snapshot ownership and a one-use viewer handoff before
        Studio UI work; preserve exact output resources and existing isolation,
        limits, native CSS remapping and navigation. Give the viewer its own
        registration and cleanup, independent of Studio run/context replacement.
    - [x] Add shared snapshot ownership/admission and real cross-realm handoff
          fixtures before implementation: exact byte copies, independent leases,
          budgets, one receiver, replay/invalid-envelope refusal, cancellation,
          expiry, failed delivery and cleanup. Exercise existing artifact
          admission, relative CSS/images and navigation after source release.
      Implemented 2026-11-13: the shared store copies admitted artifact bytes or
      bounded passive text/document/data values, preserves selection metadata,
      charges retained payloads once and revokes owned resources on final release.
      Real BroadcastChannel handoff selects one receiver, validates its independent
      copy and commits ownership without transferring frame/runtime handles.
      Eight new browser stories plus 13 existing artifact/live-preview cases pass,
      including nested CSS/images, byte immutability, separate receiver realm,
      navigation after source release, URL revocation, quota/invalid payloads,
      cancellation before commit, expiry, replay and failed delivery. Source/pin
      selection integration and actual window close/popup policy remain below.
      Shared package and Studio builds, lint, packaged-output verification and
      a handoff through the locally served production Studio modules pass;
      existing lint warnings remain. No viewer or heading action is enabled yet.
  - [x] Add a trusted viewer with declarative CEM composition for artifact,
        document, text and data previews; defer live/application state migration.
    - [x] Add displayed-selection capture and declarative viewer fixtures before
          implementation: source/invocation/build precedence, pinned and stale
          outputs, unavailable/live/oversized refusals, isolated HTML, bounded
          tables, artifact navigation and cancellation/cleanup. Verify the
          packaged viewer across actual browsing contexts and offline reload.
      Implemented 2026-11-13: `preview.html` receives a one-use snapshot before
      component startup, clears its token from history and renders through shared
      media/data/feedback controls. The mounted workbench captures only settled
      displayed selection; the pane and capture share one resolver for source,
      invocation, project and pinned output precedence. Missing/live/download-only
      or oversized results are refused without rerunning commands. Original
      snapshot identities and stale/draft facts survive later source changes;
      unrelated editors cannot supply output revision metadata.
      Six viewer/capture browser cases and 52 navigation plus eight recovery
      cases pass, with the new viewer cases repeated from a fresh module cache.
      The production verifier passes all four modes offline, including quoted
      native data cells, source-window closure, isolated HTML, relative CSS/images,
      captured-tree navigation, token removal and refused reloads. Six themes,
      inherited typography, forced colors and receiver disposal are exercised.
      Build, lint, typecheck, UI classification and packaged-output checks pass;
      the viewer import map has a verified exact CSP hash. The launcher is next.
  - [x] Add a general `cem-workspace` preview-actions slot and the small contextual
        `cem-icon-button` with `north_east`, matching title/accessibility label and
        an explicit unavailable reason for unsupported or missing previews.
    - [x] Add shared workspace preview-actions retention/gallery fixtures before
          app adoption, then launcher/controller and real-window fixtures for
          blocked/no-response popups, repeated replacement, failed replacement,
          close-before/after-receipt, selection guards and snapshot cleanup, including rejected captures and a
          fresh smaller-output retry. Preserve viewer navigation during a pending
          failed replacement and show unavailable reasons while an older detached
          snapshot remains open.
    - [x] Verify compact shared icon-button/workspace declarations through their
          stories, add inherited native-control typography coverage and retain Login
          recovery within the original node limit.
    - [x] Verify popup/handshake failure, offline use, preserved Studio selection,
          keyboard/focus, six themes, inherited overrides and forced colors.
    - [x] Complete source/pinned/current selection, stale success, run replacement,
          repeated opening, parent/viewer close and resource-accounting fixtures
          through the actual header launcher and window lifecycle.

      Completed 2026-11-14: the shared workspace exposes `preview-actions` and
      Studio composes the compact contextual `north_east` icon button with its
      matching title/name. Disabled previews retain an explicit reason even when
      a previous detached snapshot stays open. Browser services own trusted
      `noopener` opening, a bounded readiness handshake, replacement and page-exit
      cleanup; all visible controls, status and bindings remain declarative.
      Repeated activation updates one viewer without building or saving. Failed
      replacement retains the prior snapshot and navigation; rejected captures
      release their leases and allow a fresh smaller-output retry. Parent closure
      after completed receipt preserves the independent viewer.
      Verification: 29 shared workspace/icon-button stories and 67 Studio browser
      cases pass across regression/focused runs. Six other recovery cases passed
      during the full recovery run; both complete Login worker/fallback cases
      pass in isolation within the unchanged 32,768-node/4 MiB limits and existing
      deadlines. Concurrent browser suites hit recovery deadlines; no deadline or
      limit was increased. Formatting-only icon-button/workspace nodes were
      removed, and native icon controls now inherit typography consistently.
      The packaged verifier passes all four passive modes offline and the actual
      header lifecycle under the local server's versioned base: keyboard/focus,
      preserved route/run, fresh build replacement, pinned/stale output, viewer
      reuse/close/reopen, forced colors, offline reopening and source closure.
      Build, typecheck, UI classification, catalog, declarative architecture and
      packaged asset/CSP checks pass. Studio lint is clean; shared-component lint
      passes with non-null-assertion warnings. Live/application detachment remains
      deferred as assessed.

- [x] Give `cem-menu-item` selected styling while its submenu is open. Derive
      the appearance from the shared submenu's actual expanded state, using
      existing CEM state tokens; clear it when the submenu closes or switches.
      Preserve menu roles and `aria-expanded` rather than adding `aria-selected`
      solely for styling.
  - [x] Add colocated stories/gallery coverage for nested submenu switching,
        keyboard and pointer opening, dismissal, disabled items, themes,
        inherited token overrides and forced colors.
  - [x] Verify that standalone expanded attributes do not imply an open submenu,
        that open ancestors retain their contour while focus enters descendants,
        and that pointer press, Escape/Tab/outside dismissal and disabling an open
        trigger clear only the appropriate state. Exercise the actual gallery.
  - [x] Keep the dropdown gallery’s filler clearance large enough for themed
        control/hit-area geometry, then verify menu, dropdown and menu-item source,
        built and packed-package galleries without relaxing their bounds checks.

      Completed 2026-11-14: enabled menu owners derive a persistent inset zebra
      contour from the managed `aria-haspopup="menu"`/`aria-expanded="true"`
      relationship. Open ancestors stay marked while focus moves into descendants;
      closing, switching or disabling an owner clears the contour. The outer
      keyboard-focus treatment stays independent, contextual hover/press colors
      remain, and no ARIA selection or pressed state is introduced. Forced colors
      use a `SelectedItem` boundary with canonical selected stroke/offset tokens.
      The declaration stays within the public selector-specificity ceiling.
      Thirteen colocated menu-item/menu/dropdown stories pass, including six
      themes, nested keyboard/pointer operation, held activation, disabled owners,
      owner color overrides and inherited geometry. The actual source, built and
      packed-package galleries pass for all three components; menu-item examples
      include projector and live submenu owners in all six theme rows. Visual
      inspection confirms open ancestors and the focused leaf remain distinct.
      Dropdown example filler spacing now reserves themed row/hit-area size plus
      inter-item gaps; narrow overlay bounds checks pass after layout settles.
      Declarative architecture, native style contract, state matrix (41 covered
      requirements), catalog/build and package verification pass. Component lint
      passes with non-null-assertion warnings; the gallery verifier lint is clean.

- [x] Use CSS anchor positioning with `position-area` for menus, tooltips and
      popups, including `position-try-fallbacks` for alternate placements when
      the preferred area overflows. Map the shared placement/anchor contract to
      declarative CSS; assess overflow fallbacks, viewport boundaries and supported browsers;
      keep any required compatibility behavior in `cem-elements`.
  - [x] Verify nested surfaces, scrolling/resizing, RTL/writing modes, keyboard
        focus, dismissal and anchor removal through shared fixtures and the
        production component galleries.
  - [x] Add placement-to-area/fallback mapping and reversible named-anchor
        ownership fixtures, including shared anchors, authored style restoration,
        fallback when unsupported or outside the native CSS policy, and native
        edge fitting, nested scrolling/resizing, RTL/vertical writing, dismissal
        and anchor-loss behavior in shared and shipped component cases. Verify
        that anchor style claims settle without a recurring geometry frame loop.
  - [x] Exercise compiled geometry in Chromium and Firefox, and document
        feature-detected browser support and any unavailable engine checks.
  - Completed 2026-11-14: declarative menu, dropdown and tooltip rules consume
    shared logical placement-to-area/fallback mappings. Reversible named-anchor
    leases preserve authored names, concurrent panels and external style edits;
    close/reset/disconnect restore their claims. CSS handles normal attachment
    and alternate areas; explicit boundaries, captured anchors, unsupported CSS,
    mixed-axis fallbacks and unsatisfied native fits retain shared compatibility
    geometry. Freeze-on-anchor-loss retains the last fitted rectangle. Existing
    paint, focus, ARIA, dismissal and theme token ownership are preserved.
    Verification: five focused unit tests, 34 interaction stories and six existing
    submenu/checkpoint cases pass. Compiled Chromium 148 and Firefox 150 pass ten
    direction/writing-mode combinations, edge fallback, unsupported-path and
    cleanup checks. Menu/dropdown/menu-item source, built and packed-package
    galleries pass themes, forced colors, reserved overlay bounds and scrolling;
    dialog/dialog-shell/tooltip source and packed galleries pass. Declarative,
    style-contract, state-matrix (41 requirements), catalog/package, typecheck and
    lint gates pass (existing non-null-assertion warnings; verifier lint clean).
    Studio rebuild and output verification pass (238 declared assets, 358 files).
    WebKit cannot launch because host libraries are missing; its engine check
    remains a documented verification limit. Browser detection and compatibility
    boundaries are recorded in the component reference.

- [x] Assess implementing dialogs through the popover mechanism, building on
      existing shared native-surface support. Define modal/nonmodal behavior,
      top-layer/backdrop handling, focus entry/return, background inertness,
      Escape/light dismissal and nested menu/tooltip interaction. Record the
      recommended design, accessibility implications and compatibility limits
      before changing `cem-dialog`.
  - [x] Add repeatable browser assessment checks for native modal versus
        popover inertness, outside activation, backdrop, focus return and
        cancelable versus forced dismissal; exercise nested surfaces and record
        Chromium/Firefox results and unavailable engine limits.
  - [x] Record the recommended presentation matrix, accessibility constraints,
        current implementation gaps and a bounded follow-up sequence in the
        dialog contract before changing production behavior.
    Completed 2026-11-14: the [dialog/popover assessment](../packages/cem-components/docs/dialog-contract.md#dialogpopover-assessment--2026-11-14)
    recommends retaining native modal `showModal()`, persistent nonmodal `show()`
    and explicit nonmodal popover presentations. No production dialog behavior
    or declarations changed. Browser probes verify focus/pointer inertness,
    distinct visibility, outside persistence, cancelable close requests,
    noncancelable auto hiding, `closedby="any"` light-dismiss veto, forced-close
    distinction, child/tooltip Escape and parent cleanup. Compiled workspace and
    isolated packed runtimes pass in Chromium 148.0.7778.96 and Firefox 150.0.2;
    source/packed galleries, bundle, themes and forced-color checks pass. The 29
    shared/production-dialog stories and three menu-to-task relay stories pass;
    verifier lint is clean. Observation probes exposed explicit nonmodal
    `closedby="none"` Escape bypass and Firefox's outside-focus restoration
    difference, also reproduced without the adapter. The close-policy repair is
    completed below; outside-focus restoration remains an implementation
    follow-up rather than a passing behavior assertion. Modal backdrop paint is
    currently user-agent-owned, without a canonical CEM scrim rule. WebKit lacks
    required host libraries; screen-reader, physical-device and native Back
    acceptance remain unverified. Standards and compatibility sources are linked
    in the assessment.

- [x] Honor explicit native dialog close policies in the shared nonmodal Escape
      adapter while retaining CEM's default Escape behavior. Add capability
      handling for unavailable `requestClose`/close-policy support without
      bypassing vetoes via forced close. Convert assessment observations into
      regression assertions and add colocated dialog/shell cases for `none`,
      `closerequest`, `any`, guard veto and explicit close requests.
  - [x] Add shared policy/native-keyboard and close-API compatibility fixtures,
        including native/CEM vetoes, missing/throwing APIs, command results,
        reentrant requests, stale sessions, pending-preparation policy and
        unsupported-policy refusal.
  - [x] Add colocated dialog and shell policy/fallback regressions and compiled
        Chromium/Firefox checks; preserve popover, form and focus behavior.
    Completed 2026-11-14: explicit native-dialog `none` suppresses shared Escape
    during preparation and while open, including case-insensitive enum matching.
    Default/invalid policies retain established Escape behavior; explicit close
    requests remain cancelable and popover modes stay independent. Missing
    `requestClose()` dispatches one cancelable owner event through the existing
    CEM guard and closes only after acceptance in the same connected session.
    Native/CEM vetoes, reentrancy and stale-session changes cannot authorize a
    fallback close. Request failures diagnose without retrying forced close;
    command results and retained return values survive. Missing `closedBy`
    permits equivalent native defaults (nonmodal `none`, modal `closerequest`),
    including Studio's modal policy; unsupported differences are refused before
    opening, including native modal commands. The core native close API is
    checked before admission. Shared policy changes are observed live.
    Verification: the two initial shared regressions fail before the fix; 37
    shared/component/relay stories and six existing submenu/checkpoint cases
    pass. Dialog and shell each exercise both modes, all three policies and
    present/missing request APIs. Compiled workspace/packed runtime probes pass
    in Chromium 148 and Firefox 150 with masked missing/throwing APIs, vetoes,
    default-policy compatibility and unsupported-policy refusal. Source/packed
    galleries, bundle, themes and forced colors pass. Typecheck, runtime build,
    declarative architecture and lint pass (existing non-null-assertion warnings;
    verifier lint clean). Studio rebuild/output verification pass. WebKit remains
    unavailable because host libraries are missing; no historical-browser or
    physical-device acceptance is inferred from feature masking. The
    [close-policy contract](../packages/cem-components/docs/dialog-contract.md#close-policy-and-request-compatibility)
    records the compatibility boundary and keeps outside-focus repair separate.
- [x] Preserve the outside focus destination when a persistent nonmodal dialog
      closes in Firefox. Use the unadapted/native comparison to bound the repair;
      preserve native modal restoration, explicit return references, Tab/outside
      navigation, removal and menu-to-task handoff. Add shared and colocated
      production regressions; convert the assessment observation to an assertion.
  - [x] Add shared regressions for native/fallback close, Tab/outside navigation,
        selection and scroll preservation, native removal/disposal, stale reopen,
        unavailable outside controls, explicit focus overrides and opt-out.
  - [x] Add colocated dialog/shell regressions and compiled Chromium/Firefox
        assertions against workspace and packed runtimes, retaining an unadapted
        browser reference and verifying modal/popover/menu-to-task behavior.
    Verified 2026-11-14: 41 shared/component stories and six submenu/checkpoint
    cases pass. Workspace and isolated packed runtimes pass 19 outside-focus
    cases each in Chromium 148 and Firefox 150, alongside presentation, policy,
    gallery, bundle, inherited-theme and forced-color checks. The adapter repairs
    only unexpected restoration to the native pre-open entry in the same live
    session; opt-out, unavailable destinations and subsequent workflow focus
    receive no stale focus grant. Typecheck, runtime build, declarative ownership
    and lint pass (existing non-null-assertion warnings). No paint or theme-token
    ownership changed. WebKit remains unavailable because host libraries are
    missing; historical browsers and physical assistive technology are unverified.
- [x] Use explicit manual nonmodal popovers for Studio dialogs, per the owner's
      2026-11-14 instruction: project questionnaire/create/clone/remove,
      incompatible command replacement and draft-navigation decisions. Preserve
      guarded persistence, cancellation, stable owners and focus return while
      making background controls available; retain native modal support in the
      shared components.
  - [x] Update mounted Studio and production-delivery fixtures for popover
        visibility, background focus, persistent outside activation, Escape,
        draft safety, questionnaire progression and project actions.
  - [x] Verify shared manual-task-popover step replacement retains owned focus
        without stealing a deliberate outside destination or ignoring focus opt-out.
  - [x] Keep confirmation dismissal and commit paths independently bounded by
        the existing test deadline, and refresh the delivery driver's legacy
        creation helper for the current questionnaire and radio choices.
  - [x] Align the delivery fixture's fieldset paint assertions with the canonical
        super-compact density: require the inherited `0px` container token and
        matching computed inset, preserve the `23px` override check, and retain
        five-theme contrast, border and forced-color assertions.
    Verified 2026-11-14: five questionnaire and seven focused workbench browser
    cases pass, including project actions, native hide/trusted Escape, guarded
    replacement and draft retention. The production workspace-delivery gate
    passes online creation/import, offline save/Apply/run, outside activation,
    Escape/focus/caret retention, export, reload, Trash/Restore, recovery,
    graph editing and Login/Supplier workflows. Theme and inherited/forced-color
    checks pass. Fieldset assertions now require the exact canonical
    [super-compact density](../packages/cem-theme/src/lib/tokens/cem-dimension.md#66-inherited-density-profiles)
    token and computed zero inset alongside the existing `23px` override;
    component paint and public token ownership are preserved. Studio build,
    typecheck, UI classification, output verification and lint pass. Shared
    native modality remains available; the component/browser limits above apply.
- [x] Give modal dialog backdrops a canonical theme-owned scrim definition and
      declarative consumption after auditing existing theme endpoints. Verify
      six themes, inherited overrides, forced colors and nested modal/overlay
      paint; keep nonmodal popovers nonblocking. Follow the source-table,
      generator/manifest and public CSS ownership contracts.
  - [x] Add canonical D0 scrim-color and D4 scrim-opacity source tables, generator
        previews/CSS, manifest coverage and correctly typed public token exports.
  - [x] Add colocated dialog/shell/datepicker regressions and compiled
        source/bundle/packed backdrop fixtures for six themes, inherited overrides,
        native/system colors, forced colors, nested modals and overlays, actual pixel paint,
        scroll containment and transparent nonmodal presentations; preserve
        native focus/dismissal.
    Verified 2026-11-14: all three native modal declarations consume D0
    `--cem-scrim-color` and D4 `--cem-scrim-opacity`, with no component defaults.
    Canonical source tables, generator previews/CSS and 530/530 manifest coverage
    pass; public DTCG color/number values, native system-color metadata and
    TypeScript exports are verified. All 24 colocated dialog/shell/datepicker
    stories pass. Chromium 148 and Firefox 150 pass workspace and isolated packed
    pixel checks across six themes, native light/dark and forced colors, inherited
    overrides, nested modal composition, overlays and undimmed nonmodal surfaces.
    Galleries, bundle, focus/dismissal regressions, style/declarative contracts,
    package checks, projector checks and lint pass (existing component non-null
    assertion warnings). Studio rebuild/output verification pass. An earlier
    tooltip surface-readiness timeout did not recur in isolated full gates;
    no timeout or worker-budget changes were made and no cause is attributed.
    Wheel probes match an unadapted native reference: Chromium contains both
    routes, while Firefox scrolls over backdrop and non-scrollable dialog content.
    Containment remains progressive enhancement without root scroll locks.
    WebKit and physical assistive technology remain unverified. See the
    [backdrop contract](../packages/cem-components/docs/dialog-contract.md#backdrop-and-theme-ownership).

- [x] Verify all CEM components using
      [`modern-web-guidance`](/home/suns/cem/.agents/skills/modern-web-guidance/SKILL.md).
      Search/retrieve the matching component and discipline guides; audit native
      semantics, accessibility, keyboard/focus behavior, dismissal, modern CSS
      and browser-support/fallback requirements. Record component-level evidence
      and actionable gaps, preserve declarative CEM implementation and canonical
      theme ownership, and rerun relevant stories and shipped galleries for fixes.
  - [x] Review all 59 declarations against relevant retrieved guides; publish a
        component-by-component matrix with source/story evidence, limitations
        and actionable findings rather than a blanket compliance claim.
  - [x] Verify the shared architecture/style/catalog gates and component stories;
        add focused regression checkitems for confirmed fixes before adding fixtures.
  - [x] Remove prohibited naming ARIA from passive chips; verify visible text and
        absence of an invented role/name while retaining named checkable buttons.
  - [x] Expose text previews as named, keyboard-scrollable regions; verify role
        queries, exact untrusted text, focus and retained-node identity in both
        worker and main-thread fallback stories and shipped galleries.

  Audit completed 2026-11-15: [59-component evidence matrix](../packages/cem-components/docs/modern-web-guidance-audit.md).
  Two naming/region fixes shipped; 411 baseline stories and 13 focused post-fix
  stories pass. Declarative/style/state/catalog, package, lint and source/packed
  media/navigation galleries pass. The first navigation run timed out at unchanged
  list readiness; the isolated rerun passed with unchanged limits. Nine findings
  remain below, including physical/WebKit/AT evidence; this closes the audit,
  not those findings or a general compliance claim.

- [ ] Resolve findings from the [CEM component web audit](../packages/cem-components/docs/modern-web-guidance-audit.md):
  - [x] MW-03: reconcile `cem-menu-item` roles with `cem-menu` native mode;
        verify native buttons/links, live mode changes, standalone items and
        nested menu/menuitem ownership in worker/fallback and packed delivery.
    - [x] Add source-backed worker/fallback regressions for implicit standalone
          and native-mode roles, trusted Tab/activation, mode cycles, nested
          owners, child rerenders and retained-control release/reparenting,
          including open submenu triggers and their generated ARIA relationships.
          Include parent-produced slice-driven mode changes, as used by the
          property questionnaire, alongside direct host attribute changes.
    - [x] Verify native/composite roles in source, bundled and isolated packed
          menus; preserve disabled handling, focus, themes and submenu behavior.
          Check exact authored theme inventories, including Presentation where
          supplied, rather than assuming every gallery has only five themes.
          Verify exact browser accessible names for submenu triggers; decorative
          generated arrows must not become part of the command name.
          Use explicit browser role/name locators for trusted pointer assertions;
          the current test driver's element-to-name conversion misparses CSS alt text.

    Completed 2026-11-15: native items no longer hard-code menu roles; shared
    composite ownership updates nested modes and releases retained controls.
    Native popup relationships and focus are restored correctly. Decorative
    arrow alternatives preserve exact names. All 29 focused worker/fallback,
    menu/dropdown/theme/task/relationship stories and source/packed/bundled menu
    delivery checks pass, including parent-produced mode changes. Runtime and
    component builds, declarative/style/state/catalog/package checks and lint
    pass (180 existing warnings). No timeout or worker-budget changes; physical
    AT/WebKit acceptance stays open. See the linked audit closure for evidence.

  - [x] MW-04: reconcile icon-button fallback naming with the missing-name
        diagnostic contract; verify icon-only, visible-slot, external label and
        empty-name cases without inventing action names.
    - [x] Remove the invented button name, forward external naming references,
          and verify named/default-slot/empty/disabled button and link cases.
    - [x] Add shared native-control missing-name checks and warning events;
          verify worker/fallback settlement, live referenced text/IDs, dedup,
          repair (including synchronous repair by warning listeners), reconnect and
          disposal without changing native activation.
    - [x] Verify source/packed/bundled controls, menus, theme/forced-color paint,
          property documentation and runtime/component publication contracts.
          Include a real external-label example in the release bundle and verify
          warning/repair behavior through the shipped property controls. Restore the
          existing control-role composition hook in both attribute inventories and
          a real named radio-menu example; verify its native checked owners. Cover
          native current-page and disabled-disclosure ARIA forwarding in the samples.
          Reconcile the full-suite content-family assertion with MW-01's passive
          chip text/unnamed-span contract; retain its visible-content assertion.

    Completed 2026-11-15: removed the invented icon command name, forwarded
    native naming references and added bounded shared warnings with live repair,
    reconnect and scope disposal. Source/packed/bundled icon and menu delivery
    checks pass, including inherited themes and forced colors. All 21 focused
    icon stories pass with final lifecycle checks; the repaired content-family
    file passes all six stories. Runtime build, declarative/style/state/catalog,
    playground/bundle/package checks and lint pass (180 existing warnings).
    No local token defaults, component behavior or timeout increases were added.
    See the audit's [MW-04 closure](../packages/cem-components/docs/modern-web-guidance-audit.md#mw-04-icon-control-naming-closure).

  - [ ] **Owner priority: preserve declarative components and generic runtime APIs.**
        Final clarification 2026-11-15: JavaScript is allowed in `cem-elements`
        for generic browser/HTML APIs and reusable declarative capabilities;
        production JavaScript is prohibited in `cem-components`. The earlier
        zero-browser-JavaScript interpretation, including removal of the generic
        runtime, is superseded. Shared framework code must not branch on specific
        CEM component tags or conceal component-specific controllers.
    - [x] Inventory all 59 canonical component declarations and explicit imported
          component dependencies. No production JS/TS files are authored in the
          component folders (colocated test stories excluded). All 59 currently
          consume the generic JavaScript runtime; 28 declare behavior capabilities
          and two more inherit them through composition. This dependency alone
          is not a violation. Retain the table below as a coupling-audit inventory.
    - [x] Keep the earlier native-submenu prototype reverted; it is not the
          implementation of the newly specified native-invoker/focus contract.
    - [x] Record the owner's native nested `popover="auto"` baseline and generic
          invoker return-focus requirement in the
          [interaction design](cem-interaction-design.md#owner-revision--native-popup-invocation-and-generic-focus-return-2026-11-15).
          Confirm/clarify the JS ownership boundary in the primary UI principle.
    - [x] Support native `popovertarget` and `commandfor` popover invocation for
          arbitrary native popup content and declarative CEM actions. Keep one
          browser-owned visibility path, stacking and light dismissal; the generic
          focus supplement must not call visibility methods or cancel activation.
    - [x] Migrate legacy popup/composite-menu visibility contracts to native
          invoker markup. Reconcile retained keyboard/typeahead, ARIA, geometry
          and semantic focus ownership without a second visibility controller;
          the generic focus foundation does not migrate those contracts itself.
      - [x] Add native-route assertions to the dropdown and recursive menu stories;
            preserve navigation/typeahead, one-level Escape, focus policy, form/link
            activation, disabled controls and retained identities on real popovers.
      - [x] Exercise native popup/menu rebinding, detached parent-item targets,
            task handoff, geometry/checkpoint recovery and scope cleanup in the
            existing framework fixtures; retain their original contracts except
            the obsolete absolute/non-top-layer visibility assumption.
      - [x] Add a native edge-placement geometry fixture proving rejected CSS
            anchors settle without repeated owned style writes, while queued
            author transforms still invalidate the geometry witness.
      - [x] Add an external native-button dropdown gallery example with retained
            input and complete the `trigger-for` inventory; verify Escape return,
            retained edits and authored hidden closure on a native submenu.
      - [x] Disambiguate trusted hover in the two-theme action contrast fixture
            by its original sample and verify the exact native button identity;
            retain every existing pointer, focus, contrast and animation assertion.
      - [x] Verify transient focus loss to the document body does not trigger a
            second popup/submenu close; retain actual outside-control and Tab
            dismissal in dropdown and empty-submenu stories.
      - [x] Verify a provider-backed action used as an external native popup
            invoker can be checkpointed; reject a forged/rebound native route
            through its original action guard, without granting IDs authority.
      - [x] Verify the migrated popup/menu source, isolated package and bundled
            galleries in Chromium and Firefox, including themes and native
            dismissal; retain the no-authored-component-JS publication gate.
    - [x] Add generic invoker return-focus handling in `cem-elements`: use native
          invoker/target relationships and the actual opening source, cover any
          popup content, and preserve correct native restoration. Do not branch
          on `cem-menu`, `cem-dropdown` or another component's tag/implementation.
    - [x] Add native HTML/standard-command verification for parent/child stacking,
          parent-item activation, outside cascade dismissal and one-level Escape;
          verify generic return focus, outside/Tab destinations, multiple invokers,
          canceled opens, rebinding/removal, cascades and stale native events.
          Native-only baseline pages must work without page scripts; the optional
          generic focus capability is framework JS, not component-authored code.
      - [x] Implement a generic native-invoker focus subscription for Document and
            ShadowRoot roots; install it with the CEM runtime, deduplicate shared
            source/packed subscriptions, release scoped subscriptions, and yield
            to existing semantic surface owners without component-tag checks.
      - [x] Add native HTML browser stories for both standard invocation routes,
            actual/multiple openers, nested Escape/explicit close, native stacking,
            outside/Tab focus, canceled opens, rapid reopen, source/target rebinding,
            removal, refcounted disposal, native restoration and owned-surface yield.
            Include runtime auto-installation and ancestor-scope disposal in an
            independent iframe document with no `ToggleEvent.source` getter,
            plus same-value and revoked/rebound routes.
      - [x] Add a declarative CEM action example and source/packed delivery checks
            for nested native popup focus return; verify a script-free HTML baseline
            with page JavaScript disabled and keep component production sources JS-free.
    - [ ] Audit each direct/composed capability below for component-specific
          coupling. Remove any such dependency by expressing the component
          declaratively over generic native APIs. Generic interaction-pattern
          capabilities are allowed; their names alone do not establish a violation.
      - [x] Record a [source-linked audit](cem-capability-coupling-audit.md) of all
            24 capability contracts and shared payload readers, separating
            native/part contracts from actual
            component-tag, private-class and theme-paint coupling.
      - [x] Replace choice-select private class and radio-tag lookup with owned
            `part` contracts and native radio inputs; leave row-height tokens in
            declaration CSS and expose only a generic visible-row count.
      - [x] Add independent choice fixtures with renamed host/radio tags and
            classes in worker and fallback modes; verify pointer/keyboard
            selection, FACE/reset, validation, listbox row geometry, mode-change
            focus return, newer outside-focus destinations and teardown. Migrate
            wrapper-validation fixtures and source-loaded form demo from the
            private class to the public control part, retaining demo parity.
      - [x] Add a focused choice delivery gate using the existing property,
            gallery and bundle assertions; verify source and isolated package
            archives in Chromium and Firefox with inherited/forced-color themes.
      - [ ] Replace component-tag payload readers in choice-options, time-input,
            native-list, tab-panels, workflow-panels and tree-view with explicit
            generic retained-payload contracts; preserve grammar diagnostics,
            original source identity, native options, hierarchy and recovery.

        Audit checkpoint 2026-11-16: all 24 capability boundaries are reviewed in
        the [audit report](cem-capability-coupling-audit.md). Seventeen use generic
        native/part contracts; seven still depend on component payload tags in
        six readers. Choice-control DOM lookup now uses owned parts/native radio
        inputs, and declaration CSS owns row-height tokens. Deferred radio focus
        awaits actual owned item renders and is fenced by the original connection,
        control and focus destination. The capability identity is now v3. The
        form demo and custom validation fixtures use the generic control part.
        All 27 focused browser stories (including the five-sample form demo),
        27 identity/form-source unit tests and four choice/form recovery stories
        pass. Runtime build/typecheck/lint/package and component declarative,
        state/style/catalog/playground/bundle/package checks pass; warnings remain.
        The focused delivery gate passes source, isolated package archives and
        bundle checks in Chromium 148.0.7778.96 and Firefox 150.0.2. Inherited
        themes, forced-color row geometry and actual Tab focus entry pass.
        No component JS, token defaults, timeout or worker-budget changes were
        introduced. Payload migrations remain open; this is not whole-stack
        coupling closure or a resolution of the earlier full-suite stalls.

    - [x] Keep semantic menu navigation/typeahead separate from popup visibility
          and generic focus return. When required behavior lacks a known generic
          declarative expression, stop and ask the owner for research instead of
          adding component-specific JavaScript anywhere in the stack.
    - [x] Verify the existing no-authored-component-JS publication gate and add
          evidence for generic runtime ownership. Preserve native identity,
          accessibility, keyboard/form contracts, theme tokens and lifecycle
          cleanup. The browser runtime and generic event/resource machinery are
          allowed and must not be removed merely because they are JavaScript.

    Native-invoker foundation verified 2026-11-15: the runtime build passes;
    five generic native-invoker stories and all 22 action stories pass. A focused
    25-story regression group also passes for surface references, task handoff,
    element-reference lifetimes, dialogs and tooltips. Native-invoker checks on
    source and isolated packed action property/gallery pages pass in Chromium
    148.0.7778.96 and Firefox
    150.0.2, including a native stacking/dismissal baseline with page JavaScript
    disabled. The full Chromium action gate passes all 13 gallery cards, existing
    form/keyboard/identity contracts, inherited themes, reduced motion and forced
    colors. Its stale property/gallery counts and the existing contextual-menu
    example's missing `control-role` coverage annotation were corrected without
    removing behavior assertions. Declarative, state/style/catalog, playground,
    bundle and package checks pass (59 declarations, 497 tokens, 329 packed
    files); lint has zero errors and 180 existing warnings. No production
    component JavaScript, local token defaults or timeout increases were added.
    This receipt covers the initial foundation. The subsequent popup/menu
    migration receipt follows; the broader coupling audit, existing whole-suite
    stalls and WebKit/assistive-technology evidence remain open below.

    Native popup/menu migration verified 2026-11-16: the focused regression group
    passes 51/51 stories (22 actions, eight menu/dropdown cases and 21 generic
    focus/reference/task/geometry cases). The selected checkpoint group passes
    all 23 recovery stories, including independent/typed/cross-runtime submenus,
    fresh-owner popup geometry and provider-action route revocation in worker
    and fallback modes. Native HTML owns activation, nesting, Escape and outside
    dismissal; arrow/typeahead, leaf/Tab/task semantics and authored focus policies
    remain generic runtime contracts. Compatibility `open`/`hidden` reflects
    actual native state, including unrelated modal entry closing auto-popovers.
    Geometry fitting now avoids repeated equivalent CSS writes and repeated
    rejected CSS alternatives without draining author mutations or weakening
    readiness witnesses. Popup/menu/action capability identities were renewed.
    Transient blur to the document body no longer requests a legacy close; actual
    outside-control focus and Tab still dismiss. The action contrast fixture now
    disambiguates its two intentional same-name samples and checks exact button
    identity; its pointer/contrast/animation assertions remain intact.
    Runtime build, typecheck, lint and package checks pass; component declarative,
    state-matrix, style, catalog, playground, bundle and package checks pass
    (59 declarations, 497 tokens, 329 component package files). Lint passes with
    warnings. No production component JS, token defaults or timeout increases
    were added. The focused popup/menu delivery gate passes source, isolated
    package archives and bundled pages in Chromium 148.0.7778.96 and Firefox
    150.0.2, including six themes, nested native focus return, retained external
    input edits and the script-free native invocation baseline. WebKit and
    assistive-technology evidence remain outside this receipt.

    Inventory scope: canonical `packages/cem-components/src/components/*/*.xhtml`
    declarations, root/template `capability` attributes, their explicit
    `{cem-element @tag=...}` dependencies and the shared capability registry in
    `packages/cem-elements/src/lib/cem-elements.ts`. The table names source owners,
    not copied/generated artifacts. “None declared” and “—” mean no explicit additional capability in this
    inventory; generic rendering, event/resource bindings and diagnostics still
    use the permitted browser runtime. This is a component inventory, not a
    completed audit of every application entry point or shared runtime module.

    | Component — generic runtime use is allowed | Direct JS behavior capability/source to audit | Additional JS behavior through composition to audit |
    | --- | --- | --- |
    | [cem-accordion](../packages/cem-components/src/components/cem-accordion/cem-accordion.xhtml) | None declared | [disclosure-panel](../packages/cem-elements/src/lib/disclosure-panel-capability.ts) |
    | [cem-action](../packages/cem-components/src/components/cem-action/cem-action.xhtml) | [action-command](../packages/cem-elements/src/lib/action-command-capability.ts) | — |
    | [cem-alert](../packages/cem-components/src/components/cem-alert/cem-alert.xhtml) | None declared | — |
    | [cem-app-bar](../packages/cem-components/src/components/cem-app-bar/cem-app-bar.xhtml) | None declared | — |
    | [cem-autocomplete](../packages/cem-components/src/components/cem-autocomplete/cem-autocomplete.xhtml) | [editable-choice](../packages/cem-elements/src/lib/editable-choice-capability.ts) | — |
    | [cem-avatar](../packages/cem-components/src/components/cem-avatar/cem-avatar.xhtml) | None declared | — |
    | [cem-badge](../packages/cem-components/src/components/cem-badge/cem-badge.xhtml) | None declared | — |
    | [cem-card](../packages/cem-components/src/components/cem-card/cem-card.xhtml) | None declared | — |
    | [cem-checkbox](../packages/cem-components/src/components/cem-checkbox/cem-checkbox.xhtml) | [checkable-control](../packages/cem-elements/src/lib/checkable-control.ts) | — |
    | [cem-chip](../packages/cem-components/src/components/cem-chip/cem-chip.xhtml) | None declared | — |
    | [cem-data-view](../packages/cem-components/src/components/cem-data-view/cem-data-view.xhtml) | None declared | — |
    | [cem-datepicker](../packages/cem-components/src/components/cem-datepicker/cem-datepicker.xhtml) | [calendar-input](../packages/cem-elements/src/lib/calendar-input-capability.ts) | — |
    | [cem-dialog](../packages/cem-components/src/components/cem-dialog/cem-dialog.xhtml) | [native-surface](../packages/cem-elements/src/lib/native-surface.ts) | — |
    | [cem-dialog-shell](../packages/cem-components/src/components/cem-dialog-shell/cem-dialog-shell.xhtml) | [native-surface](../packages/cem-elements/src/lib/native-surface.ts) | — |
    | [cem-divider](../packages/cem-components/src/components/cem-divider/cem-divider.xhtml) | None declared | — |
    | [cem-dropdown](../packages/cem-components/src/components/cem-dropdown/cem-dropdown.xhtml) | [popup](../packages/cem-elements/src/lib/popup-capability.ts) | — |
    | [cem-expansion](../packages/cem-components/src/components/cem-expansion/cem-expansion.xhtml) | [disclosure-panel](../packages/cem-elements/src/lib/disclosure-panel-capability.ts) | — |
    | [cem-field](../packages/cem-components/src/components/cem-field/cem-field.xhtml) | [form-control](../packages/cem-elements/src/lib/form-control-capability.ts) | — |
    | [cem-fieldset](../packages/cem-components/src/components/cem-fieldset/cem-fieldset.xhtml) | None declared | — |
    | [cem-grid](../packages/cem-components/src/components/cem-grid/cem-grid.xhtml) | None declared | — |
    | [cem-icon](../packages/cem-components/src/components/cem-icon/cem-icon.xhtml) | None declared | — |
    | [cem-icon-button](../packages/cem-components/src/components/cem-icon-button/cem-icon-button.xhtml) | None declared | — |
    | [cem-list](../packages/cem-components/src/components/cem-list/cem-list.xhtml) | [native-list](../packages/cem-elements/src/lib/native-list-capability.ts) | — |
    | [cem-media-preview](../packages/cem-components/src/components/cem-media-preview/cem-media-preview.xhtml) | [live-preview](../packages/cem-elements/src/lib/live-preview-capability.ts) | — |
    | [cem-menu](../packages/cem-components/src/components/cem-menu/cem-menu.xhtml) | [composite-menu](../packages/cem-elements/src/lib/composite-menu-capability.ts) | [action-control](../packages/cem-elements/src/lib/action-control-capability.ts) |
    | [cem-menu-item](../packages/cem-components/src/components/cem-menu-item/cem-menu-item.xhtml) | [action-control](../packages/cem-elements/src/lib/action-control-capability.ts) | — |
    | [cem-nav](../packages/cem-components/src/components/cem-nav/cem-nav.xhtml) | [navigation-landmark](../packages/cem-elements/src/lib/navigation-landmark-capability.ts) | — |
    | [cem-option](../packages/cem-components/src/components/cem-option/cem-option.xhtml) | None declared | — |
    | [cem-option-group](../packages/cem-components/src/components/cem-option-group/cem-option-group.xhtml) | None declared | — |
    | [cem-paginator](../packages/cem-components/src/components/cem-paginator/cem-paginator.xhtml) | [pagination](../packages/cem-elements/src/lib/pagination-capability.ts) | — |
    | [cem-progress](../packages/cem-components/src/components/cem-progress/cem-progress.xhtml) | None declared | — |
    | [cem-progress-spinner](../packages/cem-components/src/components/cem-progress-spinner/cem-progress-spinner.xhtml) | [progress-status](../packages/cem-elements/src/lib/progress-status-capability.ts) | — |
    | [cem-radio](../packages/cem-components/src/components/cem-radio/cem-radio.xhtml) | None declared | — |
    | [cem-recovery-status](../packages/cem-components/src/components/cem-recovery-status/cem-recovery-status.xhtml) | None declared | — |
    | [cem-select](../packages/cem-components/src/components/cem-select/cem-select.xhtml) | [choice-select](../packages/cem-elements/src/lib/choice-select-capability.ts) | — |
    | [cem-sheet](../packages/cem-components/src/components/cem-sheet/cem-sheet.xhtml) | None declared | — |
    | [cem-skeleton](../packages/cem-components/src/components/cem-skeleton/cem-skeleton.xhtml) | None declared | — |
    | [cem-slider](../packages/cem-components/src/components/cem-slider/cem-slider.xhtml) | [range-input](../packages/cem-elements/src/lib/range-input-capability.ts) | — |
    | [cem-sort-header](../packages/cem-components/src/components/cem-sort-header/cem-sort-header.xhtml) | [sort-action](../packages/cem-elements/src/lib/sort-action-capability.ts) | — |
    | [cem-source-editor](../packages/cem-components/src/components/cem-source-editor/cem-source-editor.xhtml) | [plaintext-editor](../packages/cem-elements/src/lib/plaintext-editor-capability.ts) | — |
    | [cem-stack](../packages/cem-components/src/components/cem-stack/cem-stack.xhtml) | None declared | — |
    | [cem-step](../packages/cem-components/src/components/cem-step/cem-step.xhtml) | None declared | — |
    | [cem-stepper](../packages/cem-components/src/components/cem-stepper/cem-stepper.xhtml) | [workflow-panels](../packages/cem-elements/src/lib/workflow-panels-capability.ts) | — |
    | [cem-suggestions](../packages/cem-components/src/components/cem-suggestions/cem-suggestions.xhtml) | [suggestions](../packages/cem-elements/src/lib/suggestions-capability.ts) | — |
    | [cem-surface](../packages/cem-components/src/components/cem-surface/cem-surface.xhtml) | None declared | — |
    | [cem-switch](../packages/cem-components/src/components/cem-switch/cem-switch.xhtml) | None declared | — |
    | [cem-tab](../packages/cem-components/src/components/cem-tab/cem-tab.xhtml) | None declared | — |
    | [cem-table](../packages/cem-components/src/components/cem-table/cem-table.xhtml) | None declared | — |
    | [cem-tabs](../packages/cem-components/src/components/cem-tabs/cem-tabs.xhtml) | [tab-panels](../packages/cem-elements/src/lib/tab-panels-capability.ts) | — |
    | [cem-text](../packages/cem-components/src/components/cem-text/cem-text.xhtml) | None declared | — |
    | [cem-text-field](../packages/cem-components/src/components/cem-text-field/cem-text-field.xhtml) | [form-control](../packages/cem-elements/src/lib/form-control-capability.ts) | — |
    | [cem-textarea](../packages/cem-components/src/components/cem-textarea/cem-textarea.xhtml) | [form-control](../packages/cem-elements/src/lib/form-control-capability.ts) | — |
    | [cem-theme-switch](../packages/cem-components/src/components/cem-theme-switch/cem-theme-switch.xhtml) | None declared | [action-control](../packages/cem-elements/src/lib/action-control-capability.ts), [composite-menu](../packages/cem-elements/src/lib/composite-menu-capability.ts), [popup](../packages/cem-elements/src/lib/popup-capability.ts) |
    | [cem-timepicker](../packages/cem-components/src/components/cem-timepicker/cem-timepicker.xhtml) | [time-input](../packages/cem-elements/src/lib/time-input-capability.ts) | — |
    | [cem-toast](../packages/cem-components/src/components/cem-toast/cem-toast.xhtml) | None declared | — |
    | [cem-tooltip](../packages/cem-components/src/components/cem-tooltip/cem-tooltip.xhtml) | [native-surface](../packages/cem-elements/src/lib/native-surface.ts) | — |
    | [cem-tree](../packages/cem-components/src/components/cem-tree/cem-tree.xhtml) | [tree-view](../packages/cem-elements/src/lib/tree-view-capability.ts) | — |
    | [cem-tree-item](../packages/cem-components/src/components/cem-tree-item/cem-tree-item.xhtml) | None declared | — |
    | [cem-workspace](../packages/cem-components/src/components/cem-workspace/cem-workspace.xhtml) | None declared | — |

  - [ ] Diagnose component whole-suite stalls between story files. The first
        all-component run passed 414/415 and exposed MW-01's stale passive-chip
        assertion, now repaired and verified in isolation. Two later full runs
        stopped progressing without a reported assertion failure; the verbose
        run stopped after 116 passes, including all six menu stories. Both were
        interrupted. Reproduce in fresh bounded file groups, retain per-file
        progress and distinguish runtime/worker cleanup from runner/host issues;
        do not increase timeouts or claim a fully green 415-test run. A shell
        wall-clock discontinuity was also observed during the verbose attempt;
        its relationship to the stall is unproven. The separate two-theme action
        contrast fixture's ambiguous trusted-hover locator was repaired during
        native-popup verification on 2026-11-16; its focused group now passes 51/51.
        That locator fix does not resolve or explain the earlier whole-suite stalls.

  - [ ] MW-05: define native table/sort-header composition and migration; preserve
        header/cell association, focus, row identity and result announcements.
  - [ ] MW-06: review tab wrapping and panel tab stops against the retrieved tabs
        guide; verify panels with/without focusable content and narrow/RTL layouts
        while preserving intentional manual activation and retained payloads.
  - [ ] MW-07: complete native form attribute/name forwarding and explicit label
        associations; verify FACE value ownership, external forms, reset,
        validation, autocomplete, enterkeyhint, autocorrect and textarea editing.
  - [ ] MW-08: reconcile tooltip naming/description deduplication and native hint
        behavior; verify equal-name hints, unnamed triggers, existing author ARIA,
        nested menus, Escape, hover persistence, touch and cleanup.
  - [ ] MW-09: evaluate native details/heading, switch and progress alternatives
        against existing public APIs; document decisions before changing owners.
  - [ ] MW-10: measure coarse-pointer targets, spacing exceptions, text spacing,
        200% text resize and 320px reflow across themes/densities; preserve the
        explicitly content-sized super-compact profile and canonical token owners.
  - [ ] MW-11: publish a per-feature browser support/refusal matrix; run WebKit
        on a supported host and physical keyboard/touch plus NVDA/VoiceOver checks.

## Amsterdam presentation samples: active Studio priority

Completed 2026-11-04. The [completed delivery archive](archive/todo-completed-amsterdam-delivery-2026-11-04.md#amsterdam-presentation-samples-active-studio-priority)
preserves ordinary sample identities, fixtures and verification. Preserve the
accepted Login/Supplier sources, independent copies and credential policy.

## Amsterdam presentation, setup and rehearsal

Completed on explicit owner confirmation, including rehearsal acceptance,
material freeze and archive. The [final handoff](presentation/declarative-amsterdam-2026-handoff.md)
and [completed actions](archive/todo-completed-amsterdam-delivery-2026-11-04.md#amsterdam-presentation-setup-and-rehearsal)
retain publication, slide/kit identities, verification and historical limitations.
Continue with the [browser and CLI SSR follow-ups](#browser-and-cli-ssr-follow-ups).

## Later work: shared foundations, diagnostics and release

All remaining topics below follow the Amsterdam delivery gates. Reuse their
completed native/worker/fallback foundations where the demonstration needs them;
do not make the presentation depend on finishing the reduced runtime, general
SSR pipeline, all optional modules or broad device acceptance. A concrete blocker
may be promoted with a link to its required checkpoint and a bounded regression.

## Native and release follow-ups

The [completion index](archive/todo-completed-index-2026-11-09.md#studio-and-release)
collects the completed native and Studio diagnostics. Use the original
assertions, deadlines and baseline evidence for any recurrence.

### Historical presentation-adjacent diagnostics

- [ ] Conditional allocation follow-up: if `worker-static-change`, `worker-method`
      or another rendering fresh allocation fails or times out again,
      retain the new per-worker/group phase trace and identify the stalled
      boundary before changing runtime behavior. The historical frozen 110/111
      result and the later `worker-method` 103/104 invocation remain unattributed.
      The isolated cases and final 104/104 invocation pass; a passing repeat is
      not a timing repair. Both media and text-storage/popup drivers now carry
      bounded phase capture. Preserve the original five-second deadline, complete-group
      revocation and static-preview refusal assertions. See the
      [diagnostic capture](archive/todo-completed-worker-allocation-diagnostics-2026-11-04.md)
      and [theme fixture verification](archive/todo-completed-theme-control-fixtures-2026-11-05.md#verification-and-allocation-follow-up).
      New evidence 2026-11-13: the navigation/recovery aggregate passed 58/59;
      complete live Login worker recovery failed at `allocate 2` with
      `CemProcessingWorkerTransportError` from the worker error callback. The
      sealed witness was 32,668 nodes, within the unchanged 32,768-node limit.
      All fallback cases and the isolated complete worker rerun pass (67.12 s).
      The transport failure remains unattributed; this rerun is not a timing
      repair, and no worker policy, deadline or checkpoint budget was changed.

## Declarative UI Architecture Correction

### Remaining declarative UI migration

The canonical component migration is complete. Keep these unresolved diagnostic
follow-ups separate from completed migration and checkpoint acceptance. Follow
the [declarative UI principle](declarative-ui-principle.md) and
[component development pattern](component-development-pattern.md).

- [ ] Conditional diagnostic follow-up: if slider property-page root readiness
      fails again, use the new failure capture to attribute it before changing
      behavior. One initial aggregate timeout remains unexplained; the unchanged
      source/packed gate, 20 reloads under load and final full gate pass. Passing
      repeats do not establish a cause or a specific slider fix.

- [ ] Conditional suggestions-controller follow-up: if
      `ProductionFieldsExternalSourcesAndTrustedAncestorResume` or
      `PlatformCloseCancelsPendingAndPreservesEditorRouting` fails again, retain
      the new story/preparation/row-map trace and attribute the actual boundary.
      Their October failures remain historically unattributed; the controlled
      fixture race above is not claimed as proof of their cause. Preserve all
      assertions, native authority checks and original deadlines.

- [ ] Conditional diagnostic follow-up: if datepicker/suggestions failures
      recur during concurrent browser runs, capture their focus, dialog and
      processing state before changing behavior. Running the canonical cohort
      alongside the package gate produced two 30-second play timeouts and two
      following focus/open failures (262/266); the standalone cohort passes
      266/266 with unchanged deadlines and assertions. No runtime cause is
      attributed. Preserve this distinction in subsequent acceptance reports.

### Native-surface delivery follow-ups

- [ ] On recurrence of the ARIA probe's draft-worker initialization timeout,
      use its failed-request path and initialization phase/counters to attribute
      the loading boundary before changing the runtime. One follow-up run
      recorded `ERR_NETWORK_CHANGED` but no failed path; retain that
      [failure](archive/aria-browser-relations-network-failure-2026-11-05.json)
      separately from the passing six-case matrix. Preserve the 30-second
      deadline and every DOM/AX assertion; passing repeats are not attribution.

- [ ] Verify both pinned [ARIA profiles](cem-element-aria-reference-profile.md)
      in the documented browser/assistive-technology matrix. Record actual
      browser/OS/AT versions, accessibility relation exposure, ordered/repeated
      details targets and error-message announcements. Keep untested combinations
      explicit and the draft profile opt-in. Single-relation accessibility APIs
      may expose only the first target; exported DOM sequences must stay intact.
      The repeatable browser probe and persistent manual fixtures above are
      ready. CDP evidence does not establish OS API mappings or spoken output;
      the documented Windows/macOS/Linux AT rows remain pending actual runs.

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

## Studio startup failure watch

- [ ] On recurrence, attribute the public presentation Site Studio startup variance observed on
      version `c0833c4c-6820-4014-a44a-65aba2901282`: the first combined public
      gate exceeded the unchanged 30-second startup deadline with no reported
      page/HTTP/worker errors. A separate fresh direct startup settled in
      23,370 ms. Retain the failed attempt and network timing capture; passing
      repeats alone do not establish the cause. Preserve the existing deadline
      and full navigation/offline acceptance assertions.
      The complete HTTP/2 public gate passes; retain the separate default search
      QUIC/network-change failures when investigating default-transport behavior.
      Follow-up 2026-11-05: bounded monotonic request/worker capture now retains
      the initial startup boundary in public reports. Fresh complete application
      workflows pass with default and HTTP/2 transport (9,017 / 6,883 ms initial
      view settlement); these repeats do not establish the old timeout's cause.
      [Capture audit](archive/todo-completed-studio-startup-capture-2026-11-05.md).

- [ ] On recurrence, attribute the historical aggregate fallback entry-module
      fetch failure before changing the loader or test lifecycle. The old cohort
      passed 273/274 and an isolated rerun passed; current observed and fresh-cache
      full cohorts each pass 275/275. Use the
      [script/network/frame capture](studio-module-load-diagnostics.md) to retain
      the actual failed request and owning frame, add a failing cause-specific
      regression, then repair and rerun the complete cohort. Passing repeats and
      a later HTTP 200 do not establish the historical cause. Preserve recovery
      assertions/deadlines, credential refusal, user storage and the 4 MiB budget.

## Browser failures observed during placement verification

Preserve meaningful assertions and existing story/global deadlines. Capture
failing lifecycle state before changing readiness or runtime behavior; passing
repeats alone do not attribute a historical failure.

- [ ] On recurrence, attribute the historical aggregate-only data-table/set-URL and
      native suggestions pointer-tap deadlines recorded on 2026-10-08. Establish
      isolated controls, then capture the failing step under load. The separate
      pan-opening readiness defect and provider story split are complete; the
      final aggregate passed 487/487. Retain every tap, pan-cancellation and
      outside-focus assertion. See the
      [archived investigation](archive/todo-snapshot-2026-10-10.md#browser-failures-observed-during-placement-verification).
      Follow-up 2026-11-05: unchanged related cohort passes 20/20 and all three
      critical journeys pass in isolation. Both later traced ten-file cohorts
      pass 35/35. Passive phase/failure capture now survives the original runner
      deadline, verified by a deliberately held 30-second control. No historical
      cause is inferred from these passing controls; use the actual failed
      phase/state on recurrence. [Audit](archive/todo-completed-placement-journey-audit-2026-11-05.md).

- [ ] Investigate runtime aggregate instability on recurrence. The historical
      action-cutover run passed 311/312 stories: `Payload Css Readiness And
      Hydration` expected one style but observed zero; its complete 12-story
      file passed in isolation. Later reader-admission fixture corrections do
      not attribute a post-settlement missing style. A full playground run on
      2026-10-10 also hit the gallery sample-readiness deadline once; all 19
      galleries passed in isolation and the complete source/packed rerun passed.
      Use the verifier's component, URL, missing-host and runtime diagnostics;
      preserve the readiness assertion and reproduce before changing behavior.

## Legacy Demo Case Coverage (after presentation delivery)

The static readiness inventory and reproduced scoped-CSS, external-src, retry,
NPM and string-demo fixture corrections are complete. Remaining work is traced
recurrence attribution, not wholesale replacement of frame waits. See the
[current boundary inventory](browser-stabilization-review.tmp.md#reconciliation-of-the-remaining-readiness-inventory).
These historical investigations remain behind presentation functionality.

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

- [ ] Evaluate graph-converter parity before expanding bootstrap conversion
      stages: native graph YAML-to-generic-CEM currently reports
      `cem.transform_runtime.convert_edge_unavailable`; XML-to-DOM projection
      selects `XmlDomProjectionConverter` but reports
      `cem.transform_runtime.convert_execution_unsupported`. Standalone YAML
      conversion works. Keep the verified XML-to-native-template pipeline
      preset usable and preserve the shared typed import boundary.

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.

- [ ] Implement the general whole-page DCE CLI compilation pipeline from the
      [whole-page SSR proposal](cem-ml-cli-ssr-proposal.md), using config-driven
      CEMT transform graphs and the shared native DCE lifecycle. Accept HTML,
      CEM-ML and opted-in `.cem.md` pages; keep source URI, deployed page/base URL
      and output destination distinct. Support scoped declaration/module
      resolution, canonical islands, scoped CSS, bounded nested settling and
      registered CEMT participants such as `cem-demo-element`. Emit deterministic
      HTML, hydration metadata, diagnostics and source maps with atomic
      single-page/directory fan-out. Add native, Node and browser fixtures for
      identity-matched hydration without an initial rerender, mismatch/privacy
      refusals and source/client parity, without executing arbitrary page
      JavaScript. Keep this behind the current Studio sample priorities.

### Browser and CLI SSR follow-ups

The native coordinator, readiness proof and named CLI exports are complete.
Browser adoption covers passive inline HTML, SVG and MathML, inert native
CEM source payloads, nested participants and resource-free scoped CSS.
Stateful groups with compatible inline declarations and HTML payloads support
owned scalar slices, local forms, projected controls and verified stylesheets.
The CLI publishes scoped CSS from explicitly mapped stylesheet imports and
accepts a single authored HTTP(S) document base. Direct and concrete config
page reads and native HTML parsing share the original SSR CPU budget.
The [SSR completion index](archive/todo-completed-index-2026-11-09.md#ssr)
links the corresponding design and verification records.

## Component application WASM profile (separate target)

After the Amsterdam delivery gates above, implement the
[profile design](cem-components-wasm-profile.md) and
[roadmap workstream](../roadmap.md#313-minimal-component-runtime-wasm-profile) as
an explicit bundle for `cem-elements` plus `cem-components`, leaving existing
full-runtime/CLI/Studio targets and defaults intact. Preserve every component
capability, shared UI ownership and the credential policy above. Establish fixed
size budgets from measurements before extraction is accepted. This remains a
separate component-library target.

Prioritize the app build/compiled-load boundary below over
broader benchmark and optional-authoring-module work. Start with shared compiled
UI + JSON for the shopping-cart app; the catalog remains the complete capability
inventory, not the minimum set loaded by every app.

Completed foundations: capability/size audit, compiled-template/JSON and CSS
prototypes, original island/input contracts, structural/type/dependency capsules,
versioned module/profile assembly, native CEMAPPD4 publication and mixed receivers,
actual generation census/retirement, and direct-query source/code/profile owners.
See the [native contracts](cem-native-schema-capsules.md),
[profile design](cem-components-wasm-profile.md) and
[completion history](archive/todo-snapshot-2026-11-01.md#component-application-wasm-profile-separate-target).
These establish native boundaries and bounded prototypes, not complete runtime
admission, production transport or linked-code omission.

The [native-provisioned byte endpoints](archive/todo-completed-query-endpoints-2026-11-01.md)
are complete for direct pure queries, including original native deployment owners,
independent pins, opaque per-heap tokens, native census and fresh worker/fallback
handoff. Default builds remain unchanged; application-specific bootstrap generation
and component admission remain gated.

The [rule/profile transport](archive/todo-completed-rule-profile-transport-2026-11-01.md)
now preserves complete custom sources, original code/profile owners and imported
rule leaves. Native CEMDP1 consumers bind their actual source fields/scopes. WASM
imported leaves remain guarded registrations until complete consumer installation;
they do not manufacture a datatype or infer a code grant from metadata.

Deferred execution gates, in order within this workstream:

- [ ] Assemble and measure the compiled-UI + JSON shopping-cart profile first.
  Add declarative shared GraphQL transport only if existing resource capabilities
  cannot express it; alert the user before implementing new shared JavaScript.
  Register the usable app as an ordinary Studio example project when complete,
  with real quantity/remove actions and build/preview/offline fixtures. No local
  UI JavaScript, fake action buttons or credential-policy bypass counts as done.
  - [ ] Connect the interactive selected application to an authorized Studio
        preview host and verify complete worker and fallback recovery and
        offline behavior.

- [ ] Extend the baseline with repeated cold/first-use latency, active-worker and
  aggregate host/native memory, all-capabilities/offline totals and clean npm
  consumer loading. Keep unavailable counters explicit. Preserve the matched
  before/after full-WASM acquisition evidence (worker two transfers to one;
  fallback one). Retain matched hashes/toolchains and fixed budget
  selection after the native boundary/prototype report, not an invented minimum.

  Browser-side fixture timing now supplements the original assertion wall time.
  The [readiness report and limits](cem-components-browser-readiness-timing.md)
  retain 29 passing collector/readiness tests, 40 original workflow observations
  and 120 complete PSS snapshots; hidden/incomplete controls and failed
  assertions cannot publish a result. Preserve historical archives and the
  separate metric definitions when extending these measurements.


  The [delivery/offline footprint audit](cem-components-offline-footprint.md)
  verifies all 59 canonical declaration assets separately from actual default
  and schema-assets Studio cache inventories. Five accounting/refusal fixtures
  and all 223/348/553 offline URL hashes pass. Cached response bodies total
  26.80/53.80/63.04 MiB; physical disk use, runtime memory and complete UI/format
  conformance are separate. Preserve original artifact hashes, scope limits,
  uncached files and the optional profile's duplicated native-body observation.

  - [ ] Measure cold loading and first use in clean npm worker/fallback consumers,
        checking asset closure and combined memory before selecting fixed budgets.
  - [ ] If the initial fallback/offline cart mutation or complete Studio recovery
        deadline recurs, use the original case/request/error and phase capture to
        attribute it before changing readiness or recovery deadlines.
  - [ ] On recurrence of the benchmark controls/fallback Apply-button visibility
        deadline, retain recorder state, page errors, native diagnostics and
        bounded markup to attribute it before changing behavior. The first smoke
        failure lacked this capture; later controls/full runs pass with the
        original deadline and do not establish its cause.

- [ ] Prove native call reachability and dynamic mode/provider/template closure
  alongside module extraction. Keep the conservative source matrix separate
  from linked-code omission evidence; map required source packs and compiled
  modules for each requested capability before moving its implementation.

- [ ] Define and prototype the versioned core/module ABI and capability catalog
  with native fixtures for one isolated optional operation and one importer
  feeding a retained component. Prove typed CEM-tree/source-owner handoff or
  original native reimport/recompile without cross-heap handles, JSON AST
  conversion or JavaScript parsing; retain types, namespaces/base URIs, ranges,
  diagnostics, cancellation and lease disposal. Keep a required parser in core
  until this boundary passes, and record the selected module-loading mechanism.
  Use editor paint/highlighting as the isolated-operation candidate and CSV
  feeding `cem-data-view` as the typed retained-data candidate from the audit.

- [ ] Split native dependency/registry/export roots behind a separate component
  profile; add proposed `cem_ql:build:wasm-components`. Keep existing full builds
  and rendering/admission/recovery guards unchanged. Add native fixtures and
  binary/import-graph evidence that optional compiled code and debug/tooling
  initialization are absent, rather than merely hiding their JavaScript exports.

- [ ] Add the reusable `cem-elements` module loader and proposed
  `cem-elements:build:components-runtime` target. Alert the user before new shared
  JavaScript capability implementation. Cover pinned ABI/version/hash/features,
  dependency closure, bounded deduplicated acquisition, worker/owned-fallback
  activation, exact-owner disposal and late/aborted/revoked publication. No
  Studio/component-local JavaScript loader workaround is allowed.

- [ ] Connect generated native requirements and existing declarative resource
  demand to module readiness/preload. Add fixtures for external/dynamic template
  languages, mode/resource changes, pending/failing demand, retry, concurrent
  consumers and source-editor selection/IME/focus preservation. Preserve current
  committed UI and exactly-once side effects; document explicit async readiness
  for this new entry point without silently changing existing synchronous APIs.

- [ ] Extract optional language/conversion/tooling modules one group at a time,
  with native and browser parity fixtures for each mapped component mode and
  original diagnostics/provenance. Reuse owned source packs for declarative
  schemas/formatters/colorizers; do not emit a separate WASM per source template
  or pull a monolithic full engine into every optional module.

- [ ] Add proposed `@epa-wg/cem-components:build:components-runtime` assembly and
  explicit package export/import-map wiring for declarations, matching shared
  core/loader/modules and theme dependencies. Add clean npm-consumer/bundler,
  standalone/subpath and full -> component -> full output/cache-isolation
  fixtures; assert one canonical core asset and no eager full-runtime dependency.

- [ ] Add offline/preload/update fixtures proving core-only installation fetches
  no optional modules or packs, explicit feature sets remain usable offline, and
  uncached offline demand fails without losing state or contacting an undeclared
  service. Preserve integrity, cache scope, native source authority and saved
  projects/drafts; never put credentials in public deployment caches.

- [ ] Enroll every optional endpoint, active work and retained owner in existing
  recovery groups. Add worker/fallback capture, drain, fresh-generation reimport,
  re-demand and lease/late-acknowledgment fixtures. Re-run entered-password/OTP
  refusal, require explicit clearing, keep automatic whole-Studio recovery off,
  and preserve the 4 MiB budget and all current deadlines.

- [ ] Run the complete catalog/mode, accessibility, theme/forced-color and shared
  component suites on full and component profiles without skipping capabilities
  or weakening assertions. Publish reproducible core/first-use/all-modules
  byte/latency/memory and duplication reports; enforce agreed fixed budgets,
  retained default builds and profile-specific optimizer/import/export checks.
  Document installation, demand, preload/offline choices and measured limits
  before advertising a minimum or considering a separate default-adoption step.
