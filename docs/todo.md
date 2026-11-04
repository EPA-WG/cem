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

- [x] Implement the `projector` palette in canonical CEM theme tables and generated
      CSS/exports, retaining classic light fills and pending backgrounds. Add
      computed-color fixtures for state contrast, pending stripes, inheritance,
      nested themes, forced colors and class/attribute selection. Expose
      “Presentation” in the shared theme switch and Studio persistence; extend
      their browser fixtures. Apply the canonical profile to the current native
      Amsterdam deck with revision/order/content guards, profile drift and
      idempotence fixtures, exact readback, PDF review and opening preservation.
      Verified 2026-11-02: 69 projector color/state checks, existing Phase 13 and
      density gates, nine shared-switch browser stories, 13 Studio shell cases,
      45 presentation/resource tests, Studio build/output and all affected lint
      targets pass. Public exports separate profiles from the five operational
      modes; Presentation persists as `cem-theme-projector`. The 30-page native
      deck/PDF is reviewed, nine source images are re-exported in 12 positions,
      and exact color/media readback plus a zero repeat plan preserve other
      content, metadata, notes, geometry and supplied artwork. Fresh native
      timing and zero-click playback confirm the unchanged 25.1-second opening;
      the browser audit now measures from navigation start rather than late DOM
      readiness. [Delivery](declarative-amsterdam-2026/deck-delivery.md#projector-palette-2026-11-02)
      and [receipt](declarative-amsterdam-2026/native-deck-projector-readback.json).
      Physical projector acceptance and full timed rehearsal remain separate.

- [x] Propose a reusable `projector` color variation for the Amsterdam
      presentation, centered on white backgrounds and black primary text.
      [Palette example](presentation/projector-slides-palette.svg) prepared
      2026-11-02 using W3C WAI/WCAG, Microsoft and projection research.
      Verified 16 ink/stroke surface pairs, all 14 emotional-palette text pairs,
      design references and the rendered SVG. Documented canonical token
      ownership, inherited profile boundaries, forced-colors precedence and
      native export acceptance. Revised to use classic CEM light with retained
      background fills/state progression and no contrast-mode zebra substitution;
      retained SVG colors and recorded pending-recipe validation requirements.
      The proposal document was removed after implementation; the canonical
      D0 contract and delivery record now own the implemented behavior and evidence.

- [x] Verify the integrated native opening and preserve the current brain artwork,
      speaker notes, palette and slide order. Add fixtures for group ownership,
      timing/trigger drift, missing targets and paired DNA resize orchestration.
      Fresh native export shows an existing automatic sequence completing at
      25.1 seconds, rather than the older 26-second reference; the 45-second
      opening slot is unchanged. The older slide must not overwrite it. Actual anonymous Google Slides playback
      passed with zero clicks, eight sampled frames, stage visibility/order and
      retained content at 30 seconds. Native revision stayed unchanged.
      Nine native-contract tests, 32 resource/notes/palette tests and Studio lint
      passed. [Current timeline and checks](declarative-amsterdam-2026/opening-animation.md),
      [receipt](declarative-amsterdam-2026/native-opening-readback.json).
      Full-session rehearsal and physical presenter controls remain separate.

- [x] Use the presentation plan to map the manifesto and platform pitch into
      the presentation website and slides, following the current presentation
      minutes. Use familiar side-by-side representations and visible outcomes;
      prioritize XML/XSLT developers, web UI developers and platform/integration
      owners. Keep the published activities and 60-minute slot; use broader
      pipeline and security examples as companions. Add coverage fixtures for
      omitted manifesto/pitch sections, stale slide IDs, changed public intervals,
      missing evidence and missing built website headings/resource links.
      Completed 2026-11-01: [story map](declarative-amsterdam-2026/story-map.md)
      covers 14 manifesto principles and seven pitch sections across existing
      stable slide IDs and six native-published Site sections. Preserve the
      60-minute slot and final six minutes of questions. Site build, focused
      built-route acceptance, 40 guard tests and Studio lint passed. No native
      slide rewrite or public-hosting claim; source/credential policies unchanged.

- [x] Repair the Site CLI/WASM-guide publication policies using existing native
      canonical-source rewriting. Add parent-path/fragment fixtures, native search
      heading checks, literal-code/escaped-page guards and exact shared-helper
      publication fixtures. Refresh the Amsterdam and five component-reference
      search anchors; include five existing transport/schema/declaration helpers
      in both runtime maps. Keep all selected headings and actual link targets.
      Verified 2026-11-01: five regression tests, source/asset/provenance verifier,
      two identical clean builds (303 files), 27-route production browser crawl,
      focused Amsterdam guide acceptance, interactive/search browser checks and
      lint passed. No new runtime behavior
      or app UI JavaScript; credential policy unchanged. Public hosting,
      installation acceptance and full-session rehearsal remain separate.

- [x] Fixture: guard participant release claims against HTTP-only and CLI-version
      smoke evidence, archive/platform mismatch, incomplete offline exercises and
      credential-policy drift. Preserve the dated local-kit receipt; do not
      promote it to public-host, fresh-install or other-platform acceptance.
      Verified 2026-11-03: 20 participant-kit tests and Studio lint pass; the
      verifier checks its report before writing it. The retained receipt pins
      the historical 417-file archive and exact tested host. No new full-browser
      run, public availability, installation or rehearsal claim.

- [x] Produce the presentation, speaker notes, diagrams and participant setup
      instructions. Check the proposed Node/Linux installation paths against
      actual available releases, and record requirements for any other platforms
      included in the session. Prepare resource entries for discussion #24;
      add verified links as materials become available. Use that stable URL
      throughout the deck. Add release-readiness fixtures rejecting promotion
      from HTTP reachability or CLI version smoke to full exercise acceptance,
      and retain exact tested archive identities and platform limits.
      The native deck/notes/diagrams retain their existing guarded readbacks;
      participant resource entries and discussion drafts are prepared.
      The [Node/Linux setup guide](declarative-amsterdam-2026/node-linux-setup.md)
      and [installation readback](declarative-amsterdam-2026/installation-readback.json)
      now pin downloaded Node and CLI assets and actual temporary-consumer runs.
      Public npm metadata returns 404 for both expected packages; both GitHub
      prerelease CLI hosts differ at the first frozen Login AST checkpoint.
      Keep the selected local Studio kit; no public CLI parity or other-platform
      acceptance is advertised. Public deployment and rehearsal stay separate.
      Verified 2026-11-03: the freshly downloaded official Node 24.16.0 runs
      the unchanged 417-file kit through all 23 selected offline Login/Supplier
      configurations, live interaction, failure/repair and draft/copy checks.
      The new report exactly matches the retained receipt; 45 presentation/resource
      guards and the resource verifier pass. This completes materials for the
      selected local route, not a public CLI release or clean-OS acceptance.

- [x] Fixture: make the demo verifier explicitly support production `/studio/`
      hosting without loosening local revision-path and worker-scope checks.
      Reject stale public inventory and changed critical asset bytes before
      browser acceptance; cover rewritten HTML and incorrect production paths.
      Verified 2026-11-03: 23 combined tests, verifier lint, local browser acceptance
      and a 12-file local HTTP control pass. Production mode refuses the actual
      stale public inventory; the public-path gate retains exact worker scope.

- [x] Fixture: add an explicit Amsterdam production profile to the full
      Login/Supplier verifier. Keep loopback-only acceptance as the default;
      require the exact HTTPS host/subtree and selected presentation suite for
      public runs. Cover foreign origins, credentials, query/fragment inputs,
      profile typos and trailing-slash normalization; retain target provenance
      in separate public receipts that record failures as well as success.
      Verified 2026-11-03: 22 participant/target tests and Studio lint pass;
      the actual public attempt records its startup failure in a separate receipt
      before executing any configuration. Local/default hosting remains guarded.

- [x] Fixture: reproduce the public cold-start's serialized validator/component
      initialization with deferred services and a controlled cold-WASM deployment
      run. Overlap independent default-profile startup, wait for both original
      owners before publication, and close the validator on peer failure. Preserve
      optional schema-assets ordering, failure diagnostics, the original 30-second
      deadline, credential refusal and all native generation authorities.
      Verified 2026-11-03: the worker trace captures all four serialized transfers;
      the controlled baseline misses 30 seconds (ready at 36.54 s), while the
      overlapping candidate is ready at 20.36 s with all four runtimes retained.
      All 19 startup/shell tests, typecheck, lint, graph output, general local demo
      checks and the complete 23-configuration offline Login/Supplier route pass.
      [Verification](declarative-amsterdam-2026/startup-overlap-verification.md).
      Owner uploaded the 905-file copy as `2ea68bcd`; inventory/bytes match.
      Its separate command-worker timeout is tracked below; public acceptance
      remains open.

- [x] Fixture: reproduce the deployed command worker's duplicate WASM download
      and 10-second startup failure. Reuse only the installed immutable compiled
      module across the trusted bootstrap; retain separate instances/memories,
      native admission/ABI checks, cancellation, the complete 10-second startup
      budget and 30-second Studio readiness limit. Reject invalid bootstrap code,
      verify cold network reuse and late-load cleanup, then repeat local gates.
      Align stale browser inventory assertions (29 registries) with the existing
      native/unit contract (36), retaining exact registry and ownership checks;
      the rebuilt WASM hash is unchanged.
      Verified 2026-11-03: full CLI browser suite, 57 Node/native-boundary tests,
      19 shell/startup tests, 59 resource guards, lint and typecheck pass. The
      controlled deployed-host baseline hits its original worker timeout; the
      candidate settles at 20.58 s with three WASM requests and one CEM-ML load.
      General demo checks and all 23 selected offline Login/Supplier configurations
      pass against the new 905-file `253278d1…` copy. Four WASM files are unchanged.
      [Verification and receipt](declarative-amsterdam-2026/startup-code-reuse-verification.md).
      Public functionality remains gated on upload and actual public acceptance.

- [x] Fixture: check a previously cached Studio command module across deployment
      refresh at the unchanged production scope/version. Require current asset
      bytes after the update flow and preserve an isolated saved project and
      draft; keep cold public acceptance separate from cached-client upgrades.
      Cover deterministic whole-deployment fingerprints and changed non-module
      assets, worker dependency updates, revision/scope cache isolation and
      old-worker offline reads. Keep every deployment byte graph-emitted and
      retain explicit update activation and optional-profile identity.
      Verified 2026-11-03: six revision/cache checks and two real browser upgrades
      pass. A native-graph revision dependency detects build changes; caches stay
      pinned to their deployment/scope and restart offline from their own metadata.
      Archived command bytes refresh, and the saved project and unsaved draft
      survive activation and offline reload. A dependency-only update also works
      while the primary worker bytes remain unchanged. Lint/typecheck pass.
      [Verification](declarative-amsterdam-2026/hosting-upgrade-verification.md).

- [x] Fixture: verify all 14 prepared discussion demo links on the fixed public
      host in isolated default-browser contexts. Assert URL, resource, tree and
      selected configuration independently; create no saved projects, retain
      failure receipts, and keep navigation separate from full workflow acceptance.
      Attempt every frozen link even if an earlier destination fails; record each
      readiness duration and failure without retries or timeout changes.
      Verified 2026-11-03: all 14 public destinations pass in fresh default
      Chromium contexts on inventory `4d21f9cd…` (906 files), with exact URL,
      resource/tree/configuration agreement and zero saved projects. Observed
      readiness is 23.952–28.555 s under the original gate. No transport overrides
      or warm-only substitutes. [Public verification](declarative-amsterdam-2026/public-demo-4d21f9cd-verification.md).

- [x] Fixture: measure first-load service-worker prefetch against native runtime
      acquisition with enabled service workers and a shared WASM bandwidth limit.
      Compare the deployed startup with registration after core initialization;
      change scheduling only if the controlled run reproduces and repairs the
      unchanged 30-second readiness failure. Preserve all runtime owners, the
      worker startup budget, offline installation and update/dirty-work policy.
      Verified 2026-11-03: the controlled deployed baseline misses readiness at
      30.418 s; the built candidate is ready at 26.171 s with the same aggregate
      3.2 MiB/s WASM budget, all native owners and original deadlines retained.
      General local checks, cached-client upgrades and all 23 selected offline
      configurations pass. The prepared 906-file copy is `4d21f9cd…`; only main
      startup and deployment revision differ from the uploaded `ee7cc90c…` copy.
      Public byte/routing checks confirm the prior hosting fix is deployed, but
      default startup/full acceptance remain open. The 14-link verifier correctly
      refuses mismatched public inventory; its navigation fixture stays open.
      [Verification and receipt](declarative-amsterdam-2026/startup-prefetch-verification.md).
      Public follow-up: the owner-deployed `4d21f9cd…` copy now passes general
      acceptance, all 23 selected workflows and all 14 demo links; see the final
      public receipt below.

- [x] Verify the functionality of every Amsterdam demo page selected for the
      presentation and participant links. Check the deployed source/edit,
      validation failure/repair, inspection/formatting, query/graph, live preview,
      navigation and linked assets in isolated browser contexts. Record exact
      URLs, build/release identities and failures; preserve saved projects,
      drafts and the credential privacy policy. Resolve presentation blockers
      before publishing or advertising the corresponding demo links.
      Local demo release build, inventory freshness and the shared demo browser
      verifier pass (904 files, four WASM files below 25 MiB); see
      [the local verification record](declarative-amsterdam-2026/demo-verification.md).
      The same local demo scope also passes the complete selected Login/Supplier
      walkthrough: 23 offline configurations, failure/repair, live view,
      provenance, independent copies and offline draft preservation.
      Public-route functionality remains open.
      Public check 2026-11-02: the 898-file deployment is older than the local
      904-file build; the actual playground still fails on the missing factory
      import. Refresh the public deployment and repeat browser acceptance.
      The deployed playground initialization/editor/Render regression now passes;
      its import map includes the command service's WASM factory dependency.
      Recheck 2026-11-03: refreshed local inventory is `19913ea5…` (904 files);
      general local browser acceptance passes. Public remains at `7d88a2d8…`
      (898 files), with 114 changed/missing inventory records and the same real
      playground import failure. The production verifier is now ready. Wrangler
      config exists, but this Linux environment has no login; Sasha will deploy
      from a signed-in machine. See the [readback](declarative-amsterdam-2026/public-demo-readback.json).
      Repeat public freshness, general browser and full Login/Supplier gates
      after the upload; keep this functionality item open until they pass.
      Post-upload `ab74c3a7` check: the 904-file public inventory and all 11
      critical asset byte checks match; isolated playground editing/Render passes.
      Studio cold startup still fails the original 30-second readiness deadline.
      A diagnostic is not ready at 32.215 seconds, then ready at 52.238 seconds;
      worker-side completion is not captured. Preserve separate QUIC/aborted-body
      failures and successful isolated transport controls without inferring a fix.
      Full public Login/Supplier acceptance stops before any configuration.
      [Version readback](declarative-amsterdam-2026/public-demo-ab74c3a7-readback.json).
      Attribute the selected cold-start bottleneck, repair the demonstrated cause,
      then rerun both public gates; no timeout extension or warm-only pass.
      The selected serialization bottleneck is now repaired and verified locally:
      independent default validator/component startup overlaps; optional package
      admission stays serial. `/home/suns/cem-bin` has 905 files, inventory
      `5deaa842…`, and passes general plus all selected offline workflows. Upload
      this copy through the owner's existing Wrangler login and rerun the exact
      public gates. Separate QUIC/aborted-transfer causes remain unattributed.
      Post-upload `2ea68bcd-f2ab-444f-a215-deda0fd9b624`: the 905-file inventory
      and all 11 critical asset byte checks match. Main WASM downloads overlap
      (about 12.4 s each), but Studio still fails readiness: the command worker
      reports its existing 10-second startup timeout, with no network failure
      recorded in this diagnostic. Its terminated worker transfer was not timed.
      [Readback](declarative-amsterdam-2026/public-demo-2ea68bcd-readback.json).
      Shared compiled-code reuse now removes the redundant command-worker
      acquisition while retaining independent instances/memory and the original budget.
      `/home/suns/cem-bin` is prepared at 905 files, inventory `253278d1…`;
      general local acceptance and the complete selected 23-configuration offline
      route pass. Upload these bytes through the owner's existing login, then
      repeat both public gates. Check existing cached clients separately from
      fresh-context acceptance while preserving saved projects and drafts.
      [Prepared-copy receipt](declarative-amsterdam-2026/startup-code-reuse-readback.json).
      Post-upload check: public now matches `253278d1…` (905 files) and all 11
      critical asset checks; no new version ID was supplied. Default Chromium
      still misses readiness; a trace measures both main WASM transfers at about
      43.6 s without JavaScript errors. An explicit HTTP/1.1 control reaches
      readiness at 25.287 s, then the full suite stops at duplicate shell
      declaration caused by literal-`@` asset redirects. No selected public
      configuration passes, and transport-control success is not default-browser
      acceptance. [Public readback](declarative-amsterdam-2026/public-demo-253278d1-readback.json).
      Cache-upgrade and module-identity fixes are now prepared locally; deploy
      the updated Worker entry/binding and 906-file copy, then repeat unchanged
      default public and selected-workflow gates. Preserve transport evidence.
      Final prepared inventory is `ee7cc90c…` (906 files, four unchanged WASM
      files). General local demo acceptance and all 23 selected offline
      Login/Supplier configurations pass at the new `6ddd7228329531b8` alias.
      [Prepared-copy receipt](declarative-amsterdam-2026/hosting-upgrade-readback.json).
      Completed public verification 2026-11-03: owner-deployed inventory
      `4d21f9cd…` (906 files) and all 11 critical bytes match before the browser
      gates; final inventory is unchanged. General public checks, all 23 selected
      offline configurations (20 Login/three Supplier) and all 14 frozen deep
      links pass in fresh default contexts. Retain earlier failures as history;
      playground HTTP reachability does not imply every playground interaction.
      [Final public receipt](declarative-amsterdam-2026/public-demo-4d21f9cd-verification.md).

- [x] Fixture: reproduce Cloudflare's literal-`@` asset redirect and duplicate
      shell-module installation. Verify the hosting adapter serves canonical
      asset bytes without redirecting the browser's module URL, preserving
      request semantics, CSP, the complete native deployment and Studio scope.
      Verified 2026-11-03: actual public HEAD is 307 for literal `@`, 200 for
      `%40`. With real Studio modules and unchanged CSP over fixture HTTPS, the
      baseline has five redirects and the same duplicate declaration; the
      adapter has zero redirects and reuses the installation. Both forwarding
      unit checks, lint and Wrangler 4.157.0 dry-run pass; `ASSETS` binding exists.
      [Verification](declarative-amsterdam-2026/hosting-upgrade-verification.md).

- [ ] Fixture: publish the existing native CEM Site artifact with a reproducible
      static inventory and verify the actual public host against those bytes.
      Crawl every allowed route, check Amsterdam guide/search fragments and
      interactive examples, preserve the existing Studio deployment and native
      UI/source ownership, and record the deployment identity and remaining limits.
      Include a deploy-only Cloudflare bundle. Cover the 43.5 MiB token source
      map with a lossless compressed transport at its unchanged public URL;
      verify HTTP decoding returns the exact native bytes, plus HEAD, cache and
      missing-asset handling. Do not drop source maps or weaken byte checks.
      Prepared 2026-11-03: separate `cem-amsterdam-2026` Wrangler config and
      deploy-only bundle; the owner will deploy from their signed-in machine.
      Two native builds match (303 files), all 27 local routes/86 fragments,
      interactive/search and focused Amsterdam checks pass. All 15 hosting guards,
      five canonical regressions and lint pass. Packaged Wrangler dry run and
      actual local Worker HTTP checks match all original bytes, including the
      large source map. Public URL/version and remote acceptance remain pending.
      [Deployment steps and receipt](declarative-amsterdam-2026/site-cloudflare-publication.md).

- [ ] Publish the reviewed Amsterdam presentation site and related demo pages
      after their functionality checks pass. Verify the actual public routes,
      deployed assets, cache freshness and presentation resource links. Record
      the deployment identity; a successful local build is not publication.
      The demo site is already published at
      https://green-morning-fd6e.trans-acf.workers.dev/ (confirmed by Sasha).
      Verify the local demo deployment first, then check the corresponding
      public routes; publication alone does not establish functional acceptance.
      Latest owner-reported version `2ea68bcd-f2ab-444f-a215-deda0fd9b624` serves
      the 905-file overlap correction but fails command-worker startup.
      Keep presentation Site publication and the
      unresolved public Studio cold-start/functionality gates separate.
      Subsequent owner-confirmed upload now matches the 905-file `253278d1…`
      inventory; no new version ID was supplied. It still fails default startup
      and the controlled shell check. The next 906-file `ee7cc90c…` copy and
      updated Wrangler Worker are prepared and locally accepted; publication
      does not close default public functionality, destination checks or rehearsal.
      Subsequent upload `4d21f9cd…` now passes public demo functionality and
      all 14 destinations. The separately reviewed presentation Site publication
      remains open; timed rehearsal is a separate delivery task.

- [x] Fixture: validate all discussion #24 slide/demo draft rows against frozen
      project/resource/configuration identities and recorded native slide IDs.
      Reject omitted cues, foreign/ambiguous URLs and weakened credential text;
      verify the selected deep links locally while the new public upload is pending.
      Preserve independent viewed-resource/command-input selection and keep local
      navigation, public functionality and actual posting as separate evidence.
      Verified 2026-11-03: all 14 links across 13 recorded slides pass local
      URL/resource/tree/configuration checks in separate fresh contexts (zero
      saved projects). Two footer pages return 200 and exact pinned source bytes;
      relative footer links are replaced. All 59 resource guards and Studio lint
      pass. [Receipt](declarative-amsterdam-2026/discussion-demo-links-verification.md).
      Public now serves the 905-file `253278d1…` copy but fails default startup
      and controlled shell inspection; this preparation does not complete public
      acceptance or post the comment.

- [x] Fixture: review the existing discussion #24 comment and complete the draft
      for all 30 recorded slides, including optional references. Preserve the
      verified 14 demo bindings and existing published examples; add meaningful
      references for conceptual slides, reject missing/duplicate/misbound slides
      and unreviewed destinations, verify new public sources, and keep posting
      separate from draft review.
      Completed 2026-11-03: live native deck order matches all 30 recorded IDs.
      The draft covers 30 slides in 31 rows: 14 unchanged accepted Studio links
      and 17 added reference slides, including all optional references. Original
      published examples are retained; both playgrounds and embedded XSLT pass
      browser checks. All 18 source links match pinned public bytes, seven
      original HTTP destinations return 200, and lint/all 65 guards pass.
      [Review and receipt](declarative-amsterdam-2026/discussion-all-slides-verification.md).

- [x] Append a resource comment to [GitHub discussion #24](https://github.com/EPA-WG/cem/discussions/24)
      mapping each demo-related slide number to its verified public demo-page
      link. Keep the stable native slide ID beside the current physical slide
      number in the preparation map, and use projects/resources/configurations
      for Studio links. Prepare the complete comment for review, verify every
      destination after publication, then post it as an explicit delivery action.
      Do not treat this TODO entry or a draft as an already-published comment.
      The [slide-to-demo comment draft](declarative-amsterdam-2026/discussion-resource-comment-draft.md)
      is prepared; public deployment freshness and operation checks remain open.
      The 14-row map now has frozen-identity guards, complete local deep-link
      navigation checks and pinned public footer links. The draft notice reflects
      the pending 906-file hosting/cache correction; public destination acceptance and
      the actual posting action remain open.
      Public destination and selected-workflow acceptance are now complete on
      `4d21f9cd…`; the draft notice records those results. Review the complete
      comment, including the historical pinned footer edition, before the separate
      explicit posting action. No comment has been sent.
      Full draft review is now complete for all 30 slides, with original published
      examples preserved and current verification footer pins. The existing
      comment was read but not updated; explicit delivery remains this open item.
      Delivered 2026-11-03 after the owner continued: updated the existing
      resource comment in place at `discussioncomment-18867103`, retaining the
      original examples and comment URL. Separate GitHub readback matches the
      published body exactly, with all 30 slides, 31 rendered rows and every
      prepared link. All 65 guards pass; public inventory remains `4d21f9cd…`.
      [Published comment](https://github.com/EPA-WG/cem/discussions/24#discussioncomment-18867103)
      and [delivery receipt](declarative-amsterdam-2026/discussion-resource-comment-delivery.json).

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
