# Todo

This is the authoritative checklist for unfinished execution work. Product/module
sequencing lives in [roadmap.md](../roadmap.md); wishlist work lives in
[wishlist.md](wishlist.md).

Completed work and verification evidence are preserved in the
[2026-10-23 snapshot](archive/todo-snapshot-2026-10-23.md), including the native
schema-asset prototype, Studio examples, editor, previews and recovery work.
Earlier history remains linked from that snapshot. Historical checkbox states
do not reopen completed work; adopted contracts remain in their design documents.

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

Completed [compact Studio layout and shared runtime work](archive/todo-completed-compact-studio-2026-10-23.md) is archived. The [font-relative tree spacing follow-up](archive/todo-completed-compact-studio-2026-10-23.md#font-relative-tree-spacing-follow-up) records the IDE reference refinement. The [page-level super-compact follow-up](archive/todo-completed-compact-studio-2026-10-23.md#page-level-super-compact-theme-follow-up) records header controls, mode boundaries and verification.

The [explicit content-sized super-compact policy](archive/todo-completed-compact-studio-2026-10-23.md#explicit-content-sized-super-compact-policy) supersedes its earlier pointer-gated geometry.

Completed [editor filename de-duplication](archive/todo-completed-compact-studio-2026-10-23.md#studio-editor-filename-follow-up) covers editable files and read-only sample/output views.

Completed [graph parameter-map authoring and Studio controls](archive/todo-completed-graph-parameters-2026-10-23.md) is archived.

Shared package transports, project/resource-driven Studio demand, opt-in
embedded-body exclusion, isolated profile measurements, editor responsiveness and
trusted optional-format adoption are complete. Prioritize the functional
content-type and schema samples below.

Completed [shared popup surfaces and indicator clearance](archive/todo-completed-compact-studio-2026-10-23.md#shared-popup-surfaces-and-indicator-clearance) is archived.

Completed [immediate persisted theme updates](archive/todo-completed-compact-studio-2026-10-23.md#immediate-persisted-theme-updates) covers storage notification independence and nested button value capture, including the earlier repeated Studio theme-change failure.

## Project builds and artifact previews

Completed [Studio accordion delivery check](archive/todo-completed-compact-studio-2026-10-23.md#studio-accordion-delivery-check-2026-10-24)
confirms the layout source is unchanged, restores the stopped local preview, and
verifies styled accordion/collapse behavior across a stale-worker reload with
saved projects and unsaved drafts retained.

Completed [canonical source viewer repair](archive/todo-completed-canonical-source-2026-10-24.md) covers explicit XHTML endpoints, source/package viewer checks and the complete nine-gallery compact Studio gate.

Completed [gallery lifecycle and checkpoint-limit repairs](archive/todo-completed-gallery-imports-2026-10-24.md) covers delayed connection callbacks, inventory refusal versus physical retirement, and the targeted source/package import gate.

The optional native schema-asset prototype and
[owned package/formatter consumers](archive/todo-completed-owned-schema-inputs-2026-10-24.md)
are complete; the default release retains its complete embedded bodies.
The [shared acquisition and offline delivery contract](archive/todo-completed-schema-acquisition-2026-10-24.md)
is implemented, including persistent activation, coordinated host integration,
opt-in deployment and body exclusion. Measure delivery and runtime costs before
claiming binary, headroom or checkpoint savings. Prototype implementation,
measurements and verification are in the
[archived evidence](archive/todo-snapshot-2026-10-23.md#project-builds-and-artifact-previews).

Completed [native owned-package activation](archive/todo-completed-native-package-activation-2026-10-24.md) covers atomic publication, source ownership, ordinary engine consumers and strict body admission. Completed WASM/service wiring and declarative demand are recorded below.

Completed [native CSV parser contracts](archive/todo-completed-csv-parser-contracts-2026-10-24.md) enable owned CSV package activation and ordinary import/validation with checked fact bindings, original source ownership and separate structural-validation readiness.

Completed [persistent WASM package activation and declarative demand](archive/todo-completed-persistent-wasm-packages-2026-10-24.md) covers native owner inventory, ordinary command/import contexts, explicit admission versus full resource-validation phases, existing HTTP-resource binding and distinct-heap recompilation. No native owners or retirement authority cross status/checkpoint records.

Completed [opt-in Studio schema-asset deployment](archive/todo-completed-studio-schema-profile-2026-10-24.md)
covers native profile emission, authored URL expectations, both factory pins,
complete offline delivery, cache isolation and independent source preparation.
Explicit host admission and coordinated publication remain required; the normal
bootstrap does not activate packages implicitly.

Completed [shared package resource handoff and owned invocation transport](archive/todo-completed-package-resource-handoff-2026-10-24.md)
covers original physical identities, checked release, fresh compilation, complete
binding, policy/credential refusals, cleanup and consecutive offline handoffs.

Completed [command-service worker package transport](archive/todo-completed-command-worker-packages-2026-10-24.md)
covers pinned original-byte delivery, exact worker ownership, trusted startup
grants, asynchronous checked release, native empty-inventory confirmation,
consecutive offline handoffs, credential refusal and late/unknown-outcome guards.


Completed [rendering-worker and owned-fallback package adapters](archive/todo-completed-rendering-package-adapters-2026-10-24.md)
covers exact physical bindings, original configuration and pool/root isolation,
checked release, fresh publication, pressure/revocation guards and consecutive
offline handoffs with native imports and template compilation.


Completed [shared Studio package demand](archive/todo-completed-studio-package-demand-2026-10-24.md)
covers explicit declarative demand across invocation, command, rendering and owned
helper heaps; lazy preview admission; complete original recovery cohorts;
single-pass fresh import; closed-runtime cleanup; and offline Studio acceptance.
The default complete deployment, credential refusal and disabled automatic
whole-Studio recovery are preserved.

Completed [opt-in embedded source-body exclusion](archive/todo-completed-external-schema-bodies-2026-10-24.md)
covers five optional packages, a complete native identity catalog, missing-source
and alias refusal, deferred conversion registration, actual binary-body omission,
and online/offline Studio formatter/recovery acceptance. The default deployment
and credential policy remain unchanged; gzip delivery is currently larger.

Completed [profile delivery, latency and native memory measurements](archive/todo-completed-schema-profile-measurements-2026-10-24.md)
cover 30 fresh-browser samples and the [recorded method/results](schema-asset-profile-measurements.md).
The optional profile adds delivery/first-use cost and CSV capacity despite saving
256 KiB of initial/paint linear capacity. Keep the complete default; the demand
selection follow-up is complete below.

Completed [project/resource-driven optional package demand](archive/todo-completed-resource-package-demand-2026-10-24.md)
covers native metadata planning, effective export selection, selected-resource
admission, bounded shared preparation, complete physical publication despite
cancellation/host closure, offline acceptance and 30 fresh-browser measurements.
HTML startup downloads no optional packages; the first dependent command pays
activation. The frozen-baseline editor timeouts are addressed below.

Completed [shared host-view settlement](archive/todo-completed-host-view-settlement-2026-10-24.md)
covers the plaintext draft profile, bounded DOM-order discovery, asynchronous
parent payloads, inserted/replaced owners and disposal. Both original desktop
editor cases and all 140 Studio workbench/navigation/recovery tests pass with
unchanged deadlines, focus/selection/scroll assertions and credential policy.

Completed [optional Studio format demand and owned source Parse](archive/todo-completed-optional-format-demand-2026-10-24.md)
covers trusted grants for Markdown, SCSS, XSLT and Relax NG, retained native parser
contracts, strict typed-source Parse, source provenance and online/offline
activation/recovery acceptance. The default retains its complete embedded catalog
and the credential policy stays unchanged; the optional profile's supported
operation limits stay explicit.

## Studio samples: supported content types and schemas

Add an Examples subfolder named **Content types and schemas**, organized into
Data, Documents and UI, Queries and transformations, and Schema authoring.
Reuse the existing manifest-generated **Schema examples** resources and authored
projects where possible. Entries remain ordinary cloneable projects with resources
and configurations; collection folders are discovery metadata. Prioritize these
functional samples before unrelated diagnostic/helper acceptance.

Completed [content-type/schema capability inventory and checked sample mappings](archive/todo-completed-content-type-inventory-2026-10-24.md)
are archived. Use the [audited operation and sample map](studio-content-types.md)
when adding the folders and functional projects below; registered aliases alone
do not establish operation support.

Completed [nested catalog folders and contextual project discovery](archive/todo-completed-content-type-catalog-2026-10-24.md)
are archived. Content types and schemas now reuses existing ordinary projects;
continue with the missing functional workflows below.

Completed [JSON/YAML report workflows and shared import repair](archive/todo-completed-json-yaml-report-2026-10-24.md)
cover native configurations, three report artifacts, intentional syntax failure
and repair, shared previews, independent saved copies and offline rebuilding.
Source validation remains distinct from application-schema instance validation.

Completed [CSV inventory, portable validation configurations and owned string imports](archive/todo-completed-csv-inventory-2026-10-24.md)
cover native header/quote diagnostics and repairs, typed HTML/JSON reports, shared
previews, independent saved copies and offline saved header-policy execution.
Both default and optional deployments pass the CSV workflow; standard header-mode
and row-width diagnostics retain their warning severity.

Completed [XML/XHTML workflows and query namespace context](archive/todo-completed-xml-xhtml-2026-10-24.md)
cover Supplier validation/selection/repair, the ordinary XHTML document project,
shared retained import, explicit query identities and saved/offline preview acceptance.

Completed [Markdown article/site workflows and passive native import](archive/todo-completed-markdown-site-2026-10-25.md)
cover article inspection/query/preview, typed metadata repair, complete site assets,
native source-range navigation and independent saved/offline copies. Authored HTML
policy, shared declarative controls and credential privacy remain unchanged.

Completed [HTML/CEM card and shared component workflows](archive/todo-completed-html-cem-card-2026-10-25.md)
cover semantic comparison, production card/slot composition, native HTML builds,
source inspection and policy repair, saved/offline previews, explicit worker
Inspect formats and shared projected slice-event ownership. Existing Login
contracts, declaration authority and credential privacy remain unchanged.

- [ ] Add SVG and MathML projects: inspect and validate a vector diagram and a mathematical expression, then preview supported output using shared preview components.
- [ ] Add CSS, SCSS and CSS-selector projects: inspect/format a stylesheet, demonstrate supported variable/nesting transformations and select document elements. State native subset limits and include scoped-style examples where supported.
- [ ] Add CEM-QL and XPath projects: standalone expressions, selecting/filtering/joining data, parameter maps and inspecting native results with source provenance.
- [ ] Add CEM transformation and XSLT projects: transform a source document into HTML or data artifacts, compare supported approaches and inspect a multi-step transformation graph with explicit entry artifacts.
- [ ] Add schema-authoring projects for native CEM schemas, JSON Schema and RELAX NG: define a small domain contract, validate passing/failing schema sources and repair the reported violations. Demonstrate CEM instance validation separately; JSON Schema and RELAX NG currently provide source validation rather than general instance validators. Expose supported schema subsets and declaration/reference behavior.
- [ ] Add module-map and portable configuration projects: resolve relative and mapped resource dependencies, inspect a CLI command/run configuration and build a portable Studio project. Cover the supported schema versions without implying schema identity alone grants execution support.
- [ ] Add native DOM/AST/event projection projects: explain import versus structural inspection, show source locations and compare projections using the shared CEM-ML presentation path. Include any additional supported schema families found by the inventory audit.
- [ ] Give each project a README with its typical use, content types/schema identities, supported configurations, expected artifacts, provenance and deliberate failure/repair steps where applicable. Reuse shared table, editor and preview controls; add no sample-specific UI JavaScript.
- [ ] Add native fixtures for each sample's authored configurations, expected artifacts and intentional diagnostics, plus Studio catalog/navigation/clone/build/preview and offline-inventory coverage. Preserve saved projects/drafts and test that original examples remain unchanged after editing clones.

## Native and release follow-ups

These remain explicit follow-ups after the functional Studio sample catalog.
Keep the original admission, provenance, privacy and verification bounds.

- [ ] Design lossless typed CEM-ML inspection of source strings containing both single and double quotes, including embedded rich-content fences. The canonical `cem-card.xhtml` XML AST currently reaches the deliberate native attribute-string encoder refusal; keep that guard and original source bytes until an admitted representation exists. Add native round-trip fixtures before changing the writer/tokenizer contract. The HTML/CEM card project explicitly requests JSON for this one AST export; it is a presentation boundary, never an internal AST handoff.

- [ ] Adopt the existing element-template source identity for remaining live-view examples such as Login. Its current generic CEM Parse reports hard unknown runtime-binding diagnostics despite exiting successfully; the new card sample uses the correct template identity. Verify source validation, editor behavior, live rendering, saved/offline copies and credential/recovery contracts before changing existing example metadata; preserve diagnostic severity and old saved-project identities.

- [ ] Repair JSON/YAML source Parse identity and diagnostics: JSON `parse --format ast` currently exits successfully with CEM bare-brace tokenizer errors, while clean `inspect --show ast` presents an imported CEM generic-data tree. Distinguish source-format metadata from projection identity and retain original source start offsets; do not replace native parsing with application JavaScript.

- [ ] Separate constructed XML document URI from source provenance URI. `uri_accessors_share_native_metadata_and_enforce_optional_node_arguments` expects `data:document_uri(data:parse("<r/>", "xml"))` to be empty, while the unchanged pre-CSV-fix runtime returns `data:parse/source`. Preserve original source locations and the existing assertion when repairing the accessor.

- [ ] Exclude generated schema-asset WASM glue from Studio's authored-source lint scope. The broad package lint currently reports 238 errors and five warnings in generated `cem_ql.js` / `cem_ql_factory.js`; linting Studio `src` and `scripts` passes. Preserve lint rules for authored code.

- [ ] Pass the admitted original XSLT parser contract into CEM-QL bundle compilation, import discovery and name resolution. Preserve contextless optional-profile missing-schema refusal and complete-profile bounded execution; cover original-owned grammar/diagnostic rules, module limits, fresh runtime replay and source provenance before enabling optional-profile executable XSLT graphs.

- [ ] Make full schema-package artifact/example validation prefer its supplied resource reader over filesystem reads for file-like canonical URIs. Preserve full validation's diagnostics and legacy filesystem behavior when no reader is supplied; keep this separate from immutable owned-pack admission and strict runtime artifact consumption.

- [ ] Diagnose the three existing linked package preparation source-map failures at `schema_package_lifecycle/attribute_datatypes.rs:473`: pretyped handoff, callback context/publication guards and checked preparation replacement retain an additional `SourceBoundary` frame. All three reproduce on unchanged `eca54d20`; preserve source provenance and intended assertions when resolving the mismatch. The broader cohort passes 46/49 cases; see the [CSV contract verification](archive/todo-completed-csv-parser-contracts-2026-10-24.md#verification).

- [ ] Diagnose the intermittent worker-static-change fresh-allocation timeout in StaticPreviewRefusals observed in the frozen 111-case credential-policy cohort (110 pass); the unchanged isolated case passes. Preserve the original five-second allocation deadline and static-presentation refusal assertions.

- [ ] Repair standalone runtime typecheck preparation for the transitive CLI generated schema: retain or restore the exact native-generated TypeScript input through the CLI build/cache contract, so `tsc --build --emitDeclarationOnly` works without explicit schema preparation. Keep this separate from semantic editor/widget recovery.

- [ ] Stabilize concurrent release-verifier scheduling around the existing 30-second cold Login source-paint case; retain its assertions and deadline. The unchanged complete release passes with `--parallel=1`.

- [ ] Repair the existing native conversion baselines independently of recovery: `cross_surface_conversion_rules::cem_fixture_preserves_anonymous_scope_expression_avt_comments_and_rich_content` loses the expected `{.busy}` value in both default and stripped tests, and stripped CLI `convert_cemt_profile_ambiguity_reports_and_explicit_selectors_resolve` expects raw CEM where the current renderer returns a `<pre>` wrapper. Preserve the intended native semantics and assertions when resolving the mismatch.

- [ ] Investigate the existing stripped WASM binary-size gate: current stripped output is 27,264,508 bytes versus 27,033,499 default bytes, despite the stripped capability contract rejecting debug control. Establish actual debugger-code omission and the correct size baseline without weakening the gate to hide retention.

## Amsterdam presentation samples: active Studio priority

The Login, Supplier-pipeline and ordinary Markdown + CEM-ML site projects are
implemented in the shared workspace. Preserve projects/resources/configurations,
source provenance and independent copies. The
[required tutorial checkpoints](declarative-amsterdam-2026/presentation.md#required-tutorial-checkpoints),
[Login demonstration](declarative-amsterdam-2026/presentation-plan.md#required-login-demonstration-and-companion-examples)
and [sample audit](presentation/declarative-amsterdam-2026-handoff.md#studio-sample-audit-2026-10-17)
remain the scope references. Completed sample work is in the
[archive](archive/todo-snapshot-2026-10-23.md#amsterdam-presentation-samples-active-studio-priority).
These remaining diagnostics/helper actions do not lead the functionality queue.

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

## Deferred work: Declarative Amsterdam 2026 CEM presentation

Presentation preparation started 2026-10-10 and was deferred at the user's
request on 2026-10-11. On 2026-10-17 the user prioritized Studio functionality
and availability of the presentation's sample set. The active sample checklist
above takes precedence over device acceptance and helper work; the remaining
deck, website, release and rehearsal actions here remain preserved. Read the
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

- [ ] Apply the canonical CEM theme light palette throughout the slide deck.
      Keep physical slide 2 (the original title slide) in dark mode as contrast.
      Use CEM dark mode for terminal output samples. Preserve semantic syntax
      colors and verify their contrast. The new opening teaser uses light mode.

- [ ] Copy the prepared animated opening into the main deck and verify the
      26-second sequence in Slideshow. Import currently requires one starting
      click; try After previous in the Motion panel for automatic slide-entry
      playback, then recheck timing. The live deck remains unchanged.

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

Completed workspace, source-editor, node and edge authoring work remains in the
[prior snapshot](archive/todo-snapshot-2026-10-23.md#cem-studio-delivery-planning).
Native parameter maps, shared form controls and their verification are in the
[graph authoring completion record](archive/todo-completed-graph-parameters-2026-10-23.md).

Continue with persistent native package activation, opt-in deployment/body
exclusion and measurements under [Project builds and artifact previews](#project-builds-and-artifact-previews).
Preserve the [bounded project lifecycle](cem-studio.md#next-delivery-milestone-user-owned-project-workspace):
100 graph nodes / 1 MiB source and 20 history snapshots / 8 MiB. Authored ID
changes, aliases and expression-valued attributes remain source-only. Keep
credential recovery refusal, the aggregate 4 MiB checkpoint budget and separate
physical-device/spoken AT acceptance.

## Component gallery source-link maintenance (after Studio presentation work)

- [ ] Migrate remaining extensionless canonical-source `cem-module-url` references to full declaration paths under the folder import-map convention. Add source/isolated-package fixtures without assuming a template has only id/type attributes. The current artifact-tree increment repairs and verifies cem-media-preview; a filtered passive-component gate keeps that verification scoped while this older bulk maintenance remains separate.

- [x] Halve normal tree row padding through theme-owned `--cem-data-tree-padding-factor` (normal/dense 0.5, sparse 1). Verified `yarn nx run @epa-wg/cem-theme:build:css` and Chromium gallery styles across all five themes: normal 4px/6px (58px rows), sparse 8px/12px (66px rows), condensed unchanged. Ancestor sparse, nested normal reset and inherited control-padding override verified. Coupling minima and color/forced-color rules remain unchanged; forced-colors emulation was not separately run.

- [x] Group tree marker and icon in a zero-gap `part="leading"` span. Updated shared disclosure hit testing for nested markers and existing story assertions for the wrapper and zero gap. Verified `cem-elements:build`, `@epa-wg/cem-components:verify-declarative`, and Chromium gallery measurements (normal/sparse/condensed gap 0, row heights unchanged); marker click emits toggle and icon click emits activation.
