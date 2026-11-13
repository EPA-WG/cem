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

- [ ] Implement **Open in separate window** for the current passive Output preview
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
  - [ ] Add a general `cem-workspace` preview-actions slot and the small contextual
        `cem-icon-button` with `north_east`, matching title/accessibility label and
        an explicit unavailable reason for unsupported or missing previews.
    - [ ] Verify popup/handshake failure, offline use, preserved Studio selection,
          keyboard/focus, six themes, inherited overrides and forced colors.
    - [ ] Complete source/pinned/current selection, stale success, run replacement,
          repeated opening, parent/viewer close and resource-accounting fixtures
          through the actual header launcher and window lifecycle.

- [ ] Give `cem-menu-item` selected styling while its submenu is open. Derive
      the appearance from the shared submenu's actual expanded state, using
      existing CEM state tokens; clear it when the submenu closes or switches.
      Preserve menu roles and `aria-expanded` rather than adding `aria-selected`
      solely for styling.
  - [ ] Add colocated stories/gallery coverage for nested submenu switching,
        keyboard and pointer opening, dismissal, disabled items, themes,
        inherited token overrides and forced colors.

- [ ] Use CSS anchor positioning with `position-area` for menus, tooltips and
      popups, including `position-try-fallbacks` for alternate placements when
      the preferred area overflows. Map the shared placement/anchor contract to
      declarative CSS; assess overflow fallbacks, viewport boundaries and supported browsers;
      keep any required compatibility behavior in `cem-elements`.
  - [ ] Verify nested surfaces, scrolling/resizing, RTL/writing modes, keyboard
        focus, dismissal and anchor removal through shared fixtures and the
        production component galleries.

- [ ] Assess implementing dialogs through the popover mechanism, building on
      existing shared native-surface support. Define modal/nonmodal behavior,
      top-layer/backdrop handling, focus entry/return, background inertness,
      Escape/light dismissal and nested menu/tooltip interaction. Record the
      recommended design, accessibility implications and compatibility limits
      before changing `cem-dialog`.

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
