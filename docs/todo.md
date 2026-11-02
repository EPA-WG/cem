# Todo

This is the authoritative checklist for unfinished execution work. Product/module
sequencing lives in [roadmap.md](../roadmap.md); wishlist work lives in
[wishlist.md](wishlist.md).

Completed work and verification evidence are preserved in the
[2026-10-31 snapshot](archive/todo-snapshot-2026-10-31.md).
Earlier history remains linked from that snapshot. Historical checkbox states
do not reopen completed work; adopted contracts remain in their design documents.

Latest completion: [native-owned node/executable receiver frames](archive/todo-completed-native-receiver-frames-2026-11-01.md).
The [reduced native input and island frames](archive/todo-completed-application-frames-2026-11-01.md)
remain the preceding stage.
The [worker/fallback application checkpoint handoff](archive/todo-completed-application-handoff-2026-11-01.md)
remains the preceding stage.
The [native-provisioned application installation](archive/todo-completed-application-endpoints-2026-11-01.md)
remains the preceding stage.
The [original application source handoff and opt-in WASM contracts](archive/todo-completed-application-source-handoff-2026-11-01.md)
remain the preceding stage.
The [imported/native-rule and custom-profile transport](archive/todo-completed-rule-profile-transport-2026-11-01.md)
remains the preceding stage.
The [native-provisioned query endpoints and TODO cleanup](archive/todo-completed-query-endpoints-2026-11-01.md)
remain the preceding stage.
The [source-backed query execution owners](archive/todo-completed-query-execution-owners-2026-11-01.md)
remain the preceding stage.
The [shared query transport/checkpoint handoff](archive/todo-completed-query-resource-handoff-2026-11-01.md)
and [native query owner adapters](archive/todo-completed-native-query-owner-adapters-2026-10-31.md)
are also complete. The [pre-cleanup snapshot](archive/todo-snapshot-2026-11-01.md)
preserves the previous nested checklist; its completed foundations are not new work.

Prioritize Studio functionality and presentation samples before unrelated helper
work and physical-device/browser/AT acceptance. Examples remain ordinary projects
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

Execution priority (2026-11-01): deliver the Amsterdam Login demonstration,
its required Studio functionality, participant/presenter route and rehearsal.
Use the existing full runtime. Promote shared engine/component work only when a
reproduced gap blocks a named presentation checkpoint; link that dependency to
the affected item. Minimal-WASM extraction, packaging, portable native input
handoff and unrelated diagnostics remain later work. Existing capabilities and
completed verification are reusable foundations, not tasks to reimplement.

## Amsterdam presentation samples: active Studio priority

The Login, Supplier-pipeline and ordinary Markdown + CEM-ML site projects are
implemented in the shared workspace. Preserve projects/resources/configurations,
source provenance and independent copies. The
[required tutorial checkpoints](declarative-amsterdam-2026/presentation.md#required-tutorial-checkpoints),
[Login demonstration](declarative-amsterdam-2026/presentation-plan.md#required-login-demonstration-and-companion-examples)
and [sample audit](presentation/declarative-amsterdam-2026-handoff.md#studio-sample-audit-2026-10-17)
remain the scope references. Completed sample work is in the
[archive](archive/todo-snapshot-2026-10-23.md#amsterdam-presentation-samples-active-studio-priority).
The required feature list is present in those references. Start with the core
Login journey below; existing implemented samples remain ordinary projects.
A historical diagnostic becomes presentation work only after reproduction on
this selected path. The relevant diagnostic records are preserved under later
native/release follow-ups.

- [x] Record the ordered [Login walkthrough](declarative-amsterdam-2026/login-walkthrough.md)
  with resource/configuration IDs, baseline, expected results and boundaries;
  correct the README live-view source contract to match the project manifest.

- [x] Freeze the ordinary Login project archive, authored manifest, native checkpoint
  outputs and failure/repair diagnostics; add replay and drift-refusal fixtures
  for altered inputs, commands, outputs and privacy policy. Keep explicit source
  revision, shared runtime ownership and disposable-credential boundaries.
  Verified 2026-11-01: 21 exact native replays, 12 freeze/drift/privacy tests
  and Studio lint passed. References are checked-in native outputs; capture and
  replay never read a browser session or export entered credentials.

- [x] Verify and freeze the required Login journey against the current Studio
  project and full runtime: open the README/source, edit a label or heading,
  introduce the syntax-valid schema violation, follow the original diagnostic,
  save the repair, inspect/format/report, compare the agreed HTML/CEM-ML fields,
  run the prepared XPath/CEM-QL expression/template/graph, and show the live
  light-DOM result. Record exact project/configuration/resource identities,
  source revision, commands and expected outputs. Preserve known reference
  warnings and comparison limits. Implement concrete gaps exposed by this path
  first, in shared libraries; add an explicit fixture action before new tests.
  Use the required tutorial checkpoints rather than adding another feature tour.
  Frozen presenter baseline: `c5e5b892`, Login 2.8.1, 35 resources and 21 native
  checkpoint results (20 configurations plus saved repair). The
  [archive and replay instructions](declarative-amsterdam-2026/login-presenter/README.md)
  preserve exact outputs, original diagnostics and all known warning limits.
  Deployed interaction/navigation acceptance passed at that revision; native
  replay and drift/privacy refusal tests verify the frozen inputs and results.

- [x] Extend deployed Login acceptance with URL/tree selection and offline unsaved
  draft restoration fixtures; verify persisted bytes, independent clone roots,
  original sample isolation and diagnostic editor selection without clearing user data.
  Include the shared compiled-declaration transport helper in both deployment
  module maps and require its presence in output verification.
  Verified 2026-11-01: Studio build, output (219 assets/334 files), lint and
  `verify:login-navigation` passed all selected Login/Supplier offline workflows.
  See the [acceptance record](declarative-amsterdam-2026/login-walkthrough.md#deployed-navigation-and-persistence-acceptance).

- [x] Package and verify a portable local Login participant kit: copy the complete
  Studio deployment plus frozen references, inventory original bytes and provide
  a Node-only static launcher. Add production-hosting and changed/missing-file
  refusal fixtures, then extract outside the workspace and run the Login offline
  acceptance through the actual launcher. Record tested host/tool versions and
  keep public release/host availability separate.
  Verified 2026-11-01: 386-file kit extracted outside the workspace, Node
  24.16.0/Linux x64/Chromium 148.0.7778.96; five launcher tests, lint and all
  23 selected offline Login/Supplier configurations passed.
  [Handout](declarative-amsterdam-2026/participant-setup.md) and
  [dated verification](declarative-amsterdam-2026/participant-kit-verification.md).

- [x] Prepare the reproducible presenter/participant route for that Login
  project: verify Studio sample selection, independent editable copies,
  project/resource/configuration URL selection, source/template save and
  validation, build artifact and live previews, reload/offline reopening and
  draft preservation. Freeze source files, exact commands, expected results,
  intentional failure/repair and saved checkpoint outputs. Keep a local
  presenter fallback and verify every advertised access/install route before
  putting it in setup instructions or discussion #24. Preserve entered-password/
  OTP recovery refusal; exercise explicit clearing only with disposable demo
  credentials. This is acceptance for the selected presentation revision, not
  a reopening of completed workspace implementation.
  Selected delivery is the verified local-transfer kit, with the frozen native
  outputs as fallback. Public hosting/registry installation and untested OSes
  are not advertised; verify them separately if selected for participant delivery.

- [x] Freeze Supplier presenter fixtures for all six configurations, XML repair,
  zero-stock output, invalid numeric stock with no new artifacts, and restored
  stock. Reuse shared native project helpers; test drift, all output hashes and
  repair independence. Verify the advertised Amsterdam collection ID in the
  participant route and correct any mismatch before rebuilding the kit.
  Verified 2026-11-01: ten exact Supplier checkpoints, seven Supplier tests,
  12 Login tests/21 unchanged native replays and lint passed. The refreshed
  417-file kit passed offline acceptance and exact root-entry URL checks.
  [Supplier freeze](declarative-amsterdam-2026/supplier-presenter/README.md);
  [refresh evidence](declarative-amsterdam-2026/participant-kit-verification.md#supplier-refresh-and-advertised-entry-urls--2026-11-01).

- [x] After the core Login route is ready, select and verify only the companion
  projects actually shown: Supplier pipeline XML/CSV matching and HTML/JSON/YAML
  outputs with failure/repair; optionally the ordinary Markdown + CEM-ML folder
  site build and preview. Keep project copies, source provenance and offline
  inputs independent. Record expected artifacts and the demonstrated host/profile;
  omit an unready optional variant rather than delaying a required Login stage.
  Selected companion: Supplier availability outputs and stock failure/repair,
  after the required Login journey. XML source/query/repair snapshots remain
  reference checkpoints. The Markdown site remains optional reference material
  outside this frozen live route; no additional example screen is introduced.

## Amsterdam presentation, setup and rehearsal

Presentation preparation was deferred on 2026-10-11 and Studio/sample delivery
was prioritized on 2026-10-17. The 2026-11-01 instruction now makes the required
presentation path and its delivery materials the leading execution work. Complete
the Login functionality/route above before optional companions and unrelated
runtime work. Slides may proceed against the verified CLI/browser route; public
Studio availability must be verified separately before it is advertised.

Read the [presentation handoff](presentation/declarative-amsterdam-2026-handoff.md)
when returning to this work. The [session proposal](declarative.amsterdam.2026.md),
[working presentation document](declarative-amsterdam-2026/presentation.md),
[presentation minutes](declarative-amsterdam-2026/presentation.md#presentation-minutes)
and [presentation plan](declarative-amsterdam-2026/presentation-plan.md) retain
subject, timing, required demonstration and adoption scope. Resolve outstanding
organizer/delivery requirements through the existing evidence and authorized
channels; private agreements remain unverified. This file owns execution order.

- [x] Prepare participant resource links and current presenter cues keyed to
  recorded native slide IDs. Verify public pinned source links, keep the kit a
  local-transfer artifact, and prepare a reviewable discussion resource block
  without posting it. Add fixtures rejecting stale collection/resource/config IDs,
  broken fallback links, timing drift and weakened credential policy.
  [Resource index](declarative-amsterdam-2026/participant-resources.md),
  [discussion draft](declarative-amsterdam-2026/discussion-24-resources.md) and
  [current presenter notes](declarative-amsterdam-2026/presenter-notes.md) are ready.
  Three pinned source pages and the hub were read publicly on 2026-11-01.
  Local verification checks 12 cues, 30 recorded slide IDs, 26 timed slides and
  3600 planned seconds. Eleven guard tests and Studio lint passed on 2026-11-01;
  measured rehearsal and native-deck readback remain open.

- [x] Synchronize the reviewed operational presenter cues into the native deck
  speaker-note fields by stable slide ID, preserving Sasha's slide content/order.
  Add guards for order drift, missing/duplicate notes owners, preservation of
  existing notes/content, idempotence and resource-link readback.
  Read back the updated notes and resource links; keep the recorded timing and
  privacy boundaries. Do not reimport the initial PowerPoint over native edits.
  Verified 2026-11-01: 30 revision-guarded notes-only insertions, exact current
  blocks/resource-link/original-note readback and unchanged visible slide structure.
  Zero repeat insertions planned; signed image download URLs are excluded from
  structural comparison. [Receipt](declarative-amsterdam-2026/native-deck-notes-readback.json)
  records stable IDs and block hashes. Palette/animation and measured rehearsal
  remain separate; 21 resource/notes guard tests and Studio lint passed. No
  discussion message was posted.

- [x] Apply the canonical CEM theme light palette throughout the slide deck.
      Keep physical slide 2 (the original title slide) in dark mode as contrast.
      Add palette fixtures for canonical token drift, slide/order/text/notes
      preservation, exact color-only readback, dark-title exceptions and contrast.
      Use CEM dark mode for terminal output samples. Preserve semantic syntax
      colors and verify their contrast. The new opening teaser uses light mode.
      Verified 2026-11-01: color-only native readback preserves title/notes/text/
      images/geometry/order; all 30 final PDF pages visually reviewed. Narrative
      palette pairs exceed 4.5:1; nine unchanged syntax-image source documents
      measure at least 5.837:1. Two hyperlink colors fixed in one repair pass.
      Zero structural errors, one existing lifecycle-arrow warning; 32 guard
      tests and Studio lint passed. [Receipt](declarative-amsterdam-2026/native-deck-palette-readback.json).
      Opening playback and measured rehearsal remain separate.

- [ ] Copy the prepared animated opening into the main deck and verify the
      26-second sequence in Slideshow. Import currently requires one starting
      click; try After previous in the Motion panel for automatic slide-entry
      playback, then recheck timing. This animation has not yet been copied into
      the live deck; notes and palette updates are complete.

- [ ] Use the presentation plan to map the manifesto and platform pitch into
      the presentation website and slides, following the current presentation
      minutes. Use familiar side-by-side representations and visible outcomes;
      prioritize XML/XSLT developers, web UI developers and platform/integration
      owners. Keep the published activities and 60-minute slot; use broader
      pipeline and security examples as companions.

- [ ] Produce the presentation, speaker notes, diagrams and participant setup
      instructions. Check the proposed Node/Linux installation paths against
      actual available releases, and record requirements for any other platforms
      included in the session. Prepare resource entries for discussion #24;
      add verified links as materials become available. Use that stable URL
      throughout the deck. The verified local Studio kit and current repository
      notes and native-deck note readback are available; public hosting/installation
      and measured rehearsal remain open.

- [ ] Rehearse the timed walkthrough and verify every shown command, example,
      output and demo path. Check slide readability and the fallback flow;
      record the tested versions and remaining limitations with the materials.

- [ ] Finalize the presentation and supporting files, record their locations
      and verification in the handoff, and archive completed presentation actions.
      Return to the next open presentation item above. An outline alone does not
      complete this set.

## Later work: shared foundations, diagnostics and release

All remaining topics below follow the Amsterdam delivery gates. Reuse their
completed native/worker/fallback foundations where the demonstration needs them;
do not make the presentation depend on finishing the reduced runtime, general
SSR pipeline, all optional modules or broad device acceptance. A concrete blocker
may be promoted with a link to its required checkpoint and a bounded regression.

## Native and release follow-ups

These investigations remain deferred unless one is reproduced as a blocker of
an active presentation checkpoint. Keep original assertions, deadlines and
baseline evidence when promoting a specific repair.

- [ ] Diagnose relative Node `--out` destination canonicalization before common command-service URI admission. `createNodeCommandService({ cwd })` plus `parse inventory.csv --out inspection.cem` currently refuses `runPlan.outputs.resolvedDestination=inspection.cem` as `cem.command_service.uri_invalid` before reaching Parse; an explicit `file:` destination works and preserves refused output. Add lowering/Node/executable fixtures first, determine the supported argv-path contract and retain strict prepared-URI/publication authority checks. Keep this separate from native Parse refusal policy.

- [ ] Diagnose the three existing linked package preparation source-map failures at `schema_package_lifecycle/attribute_datatypes.rs:473`: pretyped handoff, callback context/publication guards and checked preparation replacement retain an additional `SourceBoundary` frame. All three reproduce on unchanged `eca54d20`; preserve source provenance and intended assertions when resolving the mismatch. The broader cohort passes 46/49 cases; see the [CSV contract verification](archive/todo-completed-csv-parser-contracts-2026-10-24.md#verification).

- [ ] Isolate the optional Studio sample graph from the ordinary Feature Tour generated directory. `build-schema-assets-profile.mjs` leaves `generated/feature-tour/feature-tour.schema-assets.cem`, causing the subsequent strict default `generate-feature-tour.mjs --check` file-set assertion to fail. Add default → optional → default verification fixtures, move the optional build input into its own generated domain with correct relative resource bases, and retain exact default file-set checking. Removing only that untracked build scratch file permits default verification; saved projects/drafts are unrelated.

- [ ] Diagnose the intermittent worker-static-change fresh-allocation timeout in StaticPreviewRefusals observed in the frozen 111-case credential-policy cohort (110 pass); the unchanged isolated case passes. Preserve the original five-second allocation deadline and static-presentation refusal assertions.

- [ ] Repair standalone runtime typecheck preparation for the transitive CLI generated schema: retain or restore the exact native-generated TypeScript input through the CLI build/cache contract, so `tsc --build --emitDeclarationOnly` works without explicit schema preparation. Keep this separate from semantic editor/widget recovery.

- [ ] Stabilize concurrent release-verifier scheduling around the existing 30-second cold Login source-paint case; retain its assertions and deadline. The unchanged complete release passes with `--parallel=1`. The 283-case Studio source invocation overlapping deployed browser gates passes 276 cases with seven deadline failures; all seven pass in an isolated Nx rerun, including complete live Login recovery on worker/fallback. Pending-build cancellation then passes three isolated runs after public navigation-readiness ordering is repaired. Keep the original five-second resume deadline, test bounds, privacy assertions and 4 MiB checkpoint budget; do not treat overlapping execution as a green full-cohort invocation. The typed control-source acceptance adds a concurrent fallback `cem.checkpoint.resume.timeout` at resume 0 (worker passes); its unchanged worker/fallback rerun passes 2/2 in isolation. Preserve this separate scheduling evidence; see the [control-source verification](archive/todo-completed-control-source-adapters-2026-10-27.md#verification).

- [ ] Repair the existing native conversion baseline independently of recovery: `cross_surface_conversion_rules::cem_fixture_preserves_anonymous_scope_expression_avt_comments_and_rich_content` loses the expected `{.busy}` value in both default and stripped tests. Preserve the intended native semantics and assertion when resolving the mismatch. The separate CEMT profile/writer mismatch is completed above.

- [ ] Investigate the existing stripped WASM binary-size gate: current stripped output is 27,264,508 bytes versus 27,033,499 default bytes, despite the stripped capability contract rejecting debug control. Establish actual debugger-code omission and the correct size baseline without weakening the gate to hide retention.


### Historical presentation-adjacent diagnostics

- [ ] Diagnose the existing `DOM Stylesheet Adoption / Readiness And Native Diagnostics` held-worker timeout without weakening readiness assertions. The stylesheet ownership work reproduces the 30-second timeout with the unchanged `6f1c8dca` registry as well as the repair, both alone and in the sequential browser group; the other two DOM-adoption cases pass. Keep this separate from completed replacement/collection ownership checks.

- [ ] Audit the five pre-existing strict schema-definition diagnostics in the Studio project package (`id` pattern datatype and `path`/`reference-path` prefix facets); preserve its native portable project contract. The explicit-entry work verifies the same baseline diagnostics before/after and introduces no additional one.


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

- [ ] Diagnose the isolated suggestions-controller baseline failures reproduced on 2026-10-20 against committed HEAD: ProductionFieldsExternalSourcesAndTrustedAncestorResume (child visibility) and PlatformCloseCancelsPendingAndPreservesEditorRouting (missing committed native row map). Keep assertions and deadlines intact; distinguish these from checkpoint capture acceptance.

- [ ] Conditional diagnostic follow-up: if datepicker/suggestions failures
      recur during concurrent browser runs, capture their focus, dialog and
      processing state before changing behavior. Running the canonical cohort
      alongside the package gate produced two 30-second play timeouts and two
      following focus/open failures (262/266); the standalone cohort passes
      266/266 with unchanged deadlines and assertions. No runtime cause is
      attributed. Preserve this distinction in subsequent acceptance reports.

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


## Studio startup failure watch

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


## Component gallery source-link maintenance (after Studio presentation work)

- [ ] Migrate remaining extensionless canonical-source `cem-module-url` references to full declaration paths under the folder import-map convention. Add source/isolated-package fixtures without assuming a template has only id/type attributes. The current artifact-tree increment repairs and verifies cem-media-preview; a filtered passive-component gate keeps that verification scoped while this older bulk maintenance remains separate.

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

- [ ] Add portable native receiver input/context checkpoint handoff before active
  input recovery. Capture original sources, scopes and input-owner contracts with
  privacy/budget guards; require fresh original input owners and consumed-write
  suppression on resume. Keep source-only capture blocked while input owners
  remain live. Add mutable-input, stale-scope and foreign-input worker/fallback
  checkpoint fixtures; immutable deployment fixture draining is not this proof.
  - [ ] Add original input/context capsule and recovery-resource fixtures for
    changed input and referenced data, fresh scoped bindings, native seal expiry,
    foreign recovery packets, credential refusal and shared aggregate budgets.

- [ ] Extend native-owned frames with explicit typed-only producers and remaining
  child/field instance contracts. Reuse original admission/profile authorities,
  native schemas and source references; do not infer grants from type labels or
  publish pending schemas as ready. Add native/WASM fixtures for input producer
  expiry, original source/scope ownership, richer topology and unavailable cases.

- [ ] Bind compiled declarations, retained CSS and shared behavior/provider/source
  ownership to that ABI. Prove worker/fallback edits, credential refusal and
  generation handoff without substituting the full tooling bootstrap.

- [ ] Generate app-build dependencies and GraphQL schema/operation contracts,
  including variable/response validation and compact runtime input checks. Map
  compiled function/module requirements to transitive native implementations and
  the selected registry; keep dynamic dispatch/provider/source gaps explicit.
  Deployment manifests describe requirements, not grants. Retain native JSON
  decoding and validation for API/user input arriving after deployment.

- [ ] Bind generated registry/artifact/input contracts to the selected application
  build and specialize remaining execution/type/source roots. Audit linked symbols
  and optional-module handoff before claiming unused language/schema/function
  implementations are absent; source inventories and fingerprints are not proof.

- [ ] Assemble and measure the compiled-UI + JSON shopping-cart profile first.
  Add declarative shared GraphQL transport only if existing resource capabilities
  cannot express it; alert the user before implementing new shared JavaScript.
  Register the usable app as an ordinary Studio example project when complete,
  with real quantity/remove actions and build/preview/offline fixtures. No local
  UI JavaScript, fake action buttons or credential-policy bypass counts as done.

- [ ] Extend the baseline with repeated cold/first-use latency, active-worker and
  aggregate host/native memory, all-capabilities/offline totals and clean npm
  consumer loading. Keep unavailable counters explicit. Diagnose duplicate full
  WASM acquisition observed in local no-store worker runs (two transfers of one
  unique binary; fallback one). Retain matched hashes/toolchains and fixed budget
  selection after the native boundary/prototype report, not an invented minimum.

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
