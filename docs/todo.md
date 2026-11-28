# Todo

This is the authoritative checklist for unfinished execution work. Product/module
sequencing lives in [roadmap.md](../roadmap.md); wishlist work lives in
[wishlist.md](wishlist.md).

Completed work and verification evidence are preserved in the
[2026-10-31 snapshot](archive/todo-snapshot-2026-10-31.md).
Earlier history remains linked from that snapshot. Historical checkbox states
do not reopen completed work; adopted contracts remain in their design documents.

Completed application/command/localization designs, native menu experiments,
Studio work and component audit fixes are preserved in the
[2026-11-23 snapshot](archive/todo-snapshot-2026-11-23.md).
Its unchecked entries are historical; this file remains the active checklist.
The [virtual application design revision](archive/todo-completed-virtual-application-design-2026-11-23.md)
extends those proposals with embedded contexts, management and portable state.
The [insulation-first revision](archive/todo-completed-application-insulation-design-2026-11-24.md)
records native scope candidates, guarded browser APIs and `embed-page` parity.
The [command parameter form revision](../packages/cem-components/docs/command.md#nonmodal-parameter-forms)
resolves proposal review item 1. Chromium and Firefox native dialog checks
confirmed that the host could take focus, a sibling could be edited and a draft
could be submitted in both light DOM and an iframe. The two-editor runtime test exercises both same-ID forms and generated forms.
The [command checklist](#resource-backed-command-verification) covers the remaining selection and validation cases.
The [first display and lazy feature decision](../packages/cem-components/docs/application.md#first-display-and-lazy-features)
resolves review item 2 at proposal level. The acceptance scenario covers
first paint before hydration and independent feature loading, denial and disposal.
The [logical command route](../packages/cem-components/docs/command.md#logical-command-route-and-permissions)
records the adopted routing decision for review item 3. Projected-editor acceptance covers
observer and guard permissions, one executable claim, and private result delivery.
The [keyboard ownership decision](../packages/cem-components/docs/application.md#keyboard-ownership-across-apps)
resolves review item 4. Native focus delivery was checked in Chromium and Firefox
in light DOM and an iframe. Cross-app command routing still needs runtime coverage.
The [versioned Save As example](../packages/cem-components/docs/command.md#version-1-save-as-example)
resolves review item 5. Its `resources` containers group the English and French
entries. Native CEM-ML round-trip verification covers the example.
The [Save As profiles and destination privacy rules](../packages/cem-components/docs/command.md#save-as-across-hosts)
resolve review item 6. Native Chromium and Firefox probes confirmed capture of
an unsubmitted path. The two-editor runtime test also checks protected destination capture.
The [host-owned write outcome](../packages/cem-components/docs/command.md#host-owned-write-outcomes-and-reconciliation)
resolves review item 7. A checkpoint-coordinator probe covered owner replacement,
stale acknowledgment and trusted reconciliation. The two-editor runtime test covers late receipt delivery.
The command checklist tracks the remaining reconciliation and status-access cases.
The [persistence namespace](../packages/cem-components/docs/application.md#private-persistence-namespaces)
and [shared storage contract](../packages/cem-components/docs/application.md#shared-storage-access-and-commit-order)
resolve review item 8. The proposal covers hydration, scoped grants and commit
ordering. Browser probes cover the IndexedDB primitive. The CEM provider and
its acceptance scenarios remain implementation work.
The [locale publication contract](../packages/cem-components/docs/localization.md#locale-publication-and-command-dispatch)
resolves review item 9. It defines the acknowledgment set, dispatch and failure
rules, and immediate revocation. The race scenarios require runtime acceptance.
The [per-message localization](../packages/cem-components/docs/localization.md#per-message-language-and-formatting)
and [resource admission rules](../packages/cem-components/docs/localization.md#missing-resources-and-admission)
resolve review item 10. Native ICU, server and browser probes checked the
fallback plural and formatted label. CEM and spoken accessibility checks remain.
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

## Component web audit and runtime coupling

- [ ] Resolve findings from the [CEM component web audit](../packages/cem-components/docs/modern-web-guidance-audit.md):
  - [ ] **Owner priority: preserve declarative components and generic runtime APIs.**
        Final clarification 2026-11-15: JavaScript is allowed in `cem-elements`
        for generic browser/HTML APIs and reusable declarative capabilities;
        production JavaScript is prohibited in `cem-components`. The earlier
        zero-browser-JavaScript interpretation, including removal of the generic
        runtime, is superseded. Shared framework code must not branch on specific
        CEM component tags or conceal component-specific controllers.
    - [ ] Audit each direct/composed capability for component-specific
          coupling. Remove any such dependency by expressing the component
          declaratively over generic native APIs. Generic interaction-pattern
          capabilities are allowed; their names alone do not establish a violation.
          Completed foundation checks and the 59-component inventory are in the
          [snapshot](archive/todo-snapshot-2026-11-23.md#studio-project-build-command);
          use the [coupling audit](cem-capability-coupling-audit.md) for reader status.
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

## Menu keyboard navigation

- [x] Run the prototype’s keyboard and activation cases against the [LTR](../packages/cem-components/playgrounds/cem-menu-keynav.html) and [RTL](../packages/cem-components/playgrounds/cem-menu-keynav-rtl.html) production menus. Chromium and Firefox passed horizontal, vertical, standalone, branch-preview activation, disabled-item and Tab cases, plus inherited `keynav` and native form keys. Reproduce with `node tools/scripts/verify-cem-menu-keyboard-poc.mjs --component` after building `cem-elements`.

- [x] Exercise the radio/button prototype in separate LTR and RTL documents, each with horizontal and vertical menus. Verify native sibling moves, root leaf fallbacks, nested popup entry/return and branch switching in Chromium and Firefox.
- [x] Use one native control per item in the LTR and RTL samples. Verify adapter-owned arrows, one Tab stop per menubar and native activation with `node tools/scripts/verify-cem-menu-keyboard-poc.mjs`.

## CEM application acceptance

- [x] **Run the two-editor Save As scenario through the CEM runtime** in both
  light DOM and iframe hosts. Give editors A and B distinct drafts. From A,
  open the host File menu and invoke `file.save-as`. Move focus to B and edit
  it while A’s nonmodal form stays open. Exercise these cases in each host.
  [Command design and verification](../packages/cem-components/docs/command.md#implementation-and-verification)

  - Enter a recognizable test destination in A and switch the UI to French.
    Verify the form preserves the input and focus, and shortcuts become
    available only when the controls and binding registry agree. Submit and
    confirm exactly one write of A’s pinned revision. B keeps its edits.
    Verify the raw destination stays out of event projections, snapshots,
    diagnostics and shared storage during editing and after the form closes.
  - Revoke Save while its old label is visible. Try the menu and shortcut,
    then deliver a late locale acknowledgment. Verify zero writes.
  - In a successful run, hold the service receipt after the write commits.
    Close A, reopen it, then deliver the receipt. Verify one host outcome,
    one write in total and zero callbacks to the closed or replacement editor.

  Verified in Chromium and Firefox with `nx run cem-elements:test:two-editor-save-as`.
  Each successful case checks the contents of the temporary file written by the service.

- [ ] **Listen to “Open 1.234 files” with a screen reader.** Use a French UI,
  an English fallback button label and German (`de-DE`) number formatting.
  Focus the button and record what is heard. Verify the listener understands
  the action and the integer 1,234. Flag a decimal reading. Move to the next
  French control and check the return to French. Repeat a locale switch with
  a nonmodal form open and check that the user keeps their place without a
  repeat announcement of the whole form. Record the browser, screen reader,
  synthesizer, installed voices and automatic language-switching setting.
  [Per-message language and formatting](../packages/cem-components/docs/localization.md#per-message-language-and-formatting)

- [ ] **Verify general virtual-application isolation and lifecycle.** Exercise the guarded
  storage, URL and provider APIs in light DOM and iframe hosts against the `embed-page`
  behavioral reference. Cover named grants, read-only and concurrent shared writes,
  lifecycle fences, portable save/load/clone and physical offload. The two-editor Save As
  scenario above covers a narrower path. [Application acceptance](../packages/cem-components/docs/application.md#acceptance-and-limits)

  - [x] Verify private draft keys and per-app navigation in Chromium and Firefox, in light DOM
    and same-origin iframe hosts. The [browser check](../packages/cem-elements/tests/application-isolation.mjs)
    exercises the application storage service before any element is registered, including
    separate local/session values and refusal of a detached app’s write. An app service and
    a declarative control see each other’s writes. Two editors with the same container-local
    ID retain separate drafts and URLs. Reopening in the other host type in reverse order
    restores each draft. A live cross-tab reader sees only its partition’s update.
    Run `nx run cem-elements:test:application-isolation`.
  - [x] Verify durable named shared stores at the `cem-application` boundary in Chromium
    and Firefox, in light DOM and same-origin iframe hosts. The [browser check](../packages/cem-elements/tests/application-shared-stores.mjs)
    exercises whole-store and key-limited read/write and read-only grants. Editors in one
    host and across tabs race on an expected version. One write commits and one conflicts.
    It also checks unconditional writes, clear and deletion ordering, and a downgrade
    while a write is queued. Run `nx run cem-elements:test:application-isolation`.
  - [x] Verify session-scoped named stores in Chromium and Firefox in light DOM and
    same-origin iframe hosts. The [browser check](../packages/cem-elements/tests/application-shared-stores.mjs)
    reloads two tabs and reissues their grants. Each recovers its own values and versions.
    A fresh tab starts empty. An opener-created tab can clear its initial copy independently.
    The check also covers competing writers, a read-only recipient and a durable store
    with the same owner and ID. Run `nx run cem-elements:test:application-isolation`.
  - [x] Verify write-only named-store grants in Chromium and Firefox, in light DOM and
    same-origin iframes. For durable and session stores, the [browser check](../packages/cem-elements/tests/application-shared-stores.mjs)
    compares the writer’s completion for inserts, overwrites, removals and no-ops while
    a separately granted reader checks the values and versions. Reads, enumeration, clear
    and mutations with a version token are refused. Run `nx run cem-elements:test:application-isolation`.
    [Unit checks for early denial and provider error privacy](../packages/cem-elements/src/lib/internal/runtime-support/shared-store.spec.ts)
  - [x] Verify named-store subscriptions in Chromium and Firefox in light DOM and
    same-origin iframe hosts. The [browser check](../packages/cem-elements/tests/named-store-subscriptions.mjs)
    covers opt-in grants, filtered keys and clears, durable cross-tab commit order,
    session-only delivery and suppression of writer-self and no-op events. It revokes
    a grant during delivery, replaces a live grant and checks unsubscribe. It also
    exercises version 1 store migration, recovery beyond the journal window and a native
    session clear. Run `nx run cem-elements:test:application-isolation`.
  - The two-editor Save As gate also covers grant revocation and receipts after editor
    replacement. Private session lifetime across tabs, ancestor stop fences, portable
    save/load/clone and physical offload remain open.
    [Implementation order](../packages/cem-components/docs/application.md#missing-capabilities-and-implementation-order)

- [ ] **Complete localized resource delivery.** Pin the versioned message vocabulary and
  the declarative binding for `locate(lang)` and resource lookup. Verify equivalent
  CEM-ML, JSON, YAML and mapped CSV lookup, fallback and stale catalog handling, and
  locale/shortcut publication during hydration and in stopped apps. Record the spoken
  fallback-label result in the existing check above. [Localization acceptance](../packages/cem-components/docs/localization.md#caching-ssr-and-implementation-acceptance)

### Resource-backed command verification

The completed two-editor scenario above covers the desktop write, grant revocation during
French locale publication, and a receipt arriving after editor replacement. Extend that
fixture where a case needs two editors. The following checks cover the rest of the
[command design](../packages/cem-components/docs/command.md).

- [x] **Validate command definitions and invocation identity in native tests.** Exercise schema
  changes, scalar and native parameter profiles, locale-neutral values, duplicate command IDs,
  handler eligibility, claim and cancellation order, stale contexts and result correlation.
  Replay an invocation ID and verify its effect occurs at most once. Issue two Copy commands
  with identical parameters and distinct invocation IDs and verify each runs once.
  Run `cargo test -p cem-ml --test resource_command`. The native coordinator reads the Save As
  manifest and retains native parameters through the endpoint call. The tests also cover
  terminal results delivered out of order and revocation during a guard.
- [ ] **Verify routing from a projected editor in light DOM and iframe hosts.** Give the layout
  its own Save As handler and invoke the editor through the host menu. Exercise a successful
  claim, an owner guard refusal and a disabled nearest provider. Assert the layout receives
  zero observations, vetoes, claims or results through placement. Check the admitted observer
  slices and the caller’s permitted status. Change the selection with the menu open, then
  change the target’s logical owner during parameter entry and verify the stale route cannot execute.
- [ ] **Verify resource-bound controls and keyboard dispatch.** Exercise the action, icon button
  and menu item declarations independently with `command-id`. Check native `command` and
  `commandfor` coexistence, shortcut repeats, host calls that require user activation, menu-to-task
  focus transfer and focus return. Rerender, server-render and hydrate a bound control and
  verify that each produces zero invocations.
- [ ] **Complete nonmodal parameter-form coverage in both hosts.** Exercise explicit dialog
  overrides, same-ID lookup in the owning child and schema fallback when a custom form is
  missing. Cover duplicate or invalid bindings, unsupported required types, invalid submission
  and target closure. Change the active child and locale while editing. On submit or cancel,
  verify one result for the pinned invocation, zero effects on its sibling and preservation
  of focus when it has moved outside the form.
- [ ] **Verify destination privacy and Save As outcomes across host profiles.** In both hosts,
  enter a recognizable destination in custom and generated forms, trigger validation, switch
  locale and cancel. Inspect event projections, snapshots, diagnostics, shared storage and
  checkpoint output during editing and after teardown. On success verify that only the file
  service receives the destination for execution. Cover picker denial, failed stream close,
  download handoff and edits after dispatch. Assert the reported outcome, document destination
  and saved revision for each case.
- [ ] **Complete host receipt and reconciliation coverage.** Keep a host-menu caller admitted
  after the editor closes and check its permitted result. Exercise duplicate and contradictory
  receipts, confirmed failure, missing evidence, denied status lookup and loss of the host.
  Verify that reconnecting exposes only the granted status and never reissues the write.
- [ ] **Verify cross-application command permissions and lifecycle.** Exercise exported-child
  targeting, denied upward context writes, allowlisted parent services, sibling isolation,
  bridge-loop refusal and independent state for two instances exporting the same ID. Check
  stopped-app refusal without replay and stale generations after pause, load and clone. Run
  manager commands separately from business commands with focus and active selection in
  different apps. Test private defaults, denied self-joining and admitted per-family sharing.
  Compare light DOM and iframe outcomes, including that sharing exposes no extra handlers and
  a refused framework service cannot fall back to another app’s provider or browser state.
- [ ] **Verify accessible command presentation during locale changes.** Change locale during
  execution and check the accessible name, effective shortcut, enabled/checked/busy state and
  focus. Exercise RTL, long labels, inherited themes and forced colors. Record the spoken
  fallback-label result in the existing “Open 1.234 files” check above.

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
      [whole-page SSR design](cem-ml-cli-ssr-design.md), using config-driven
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
