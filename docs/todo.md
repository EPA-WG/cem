# Todo

This is the authoritative checklist for unfinished execution work. Product/module
sequencing lives in [roadmap.md](../roadmap.md); wishlist work lives in
[wishlist.md](wishlist.md).

Completed work and verification evidence are preserved in the
[2026-10-31 snapshot](archive/todo-snapshot-2026-10-31.md).
Earlier history remains linked from that snapshot. Historical checkbox states
do not reopen completed work; adopted contracts remain in their design documents.

Latest completion: [Amsterdam presentation delivery](archive/todo-completed-amsterdam-delivery-2026-11-04.md).
Previous native completion: [native-owned node/executable receiver frames](archive/todo-completed-native-receiver-frames-2026-11-01.md).
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

Execution priority (2026-11-04): Amsterdam presentation delivery is accepted and
archived. Continue the unfinished native/release follow-ups in their listed
order. Preserve the frozen delivery artifacts and existing runtime contracts;
completed capabilities are foundations, not tasks to reimplement.

## Amsterdam presentation samples: active Studio priority

Completed 2026-11-04. The [completed delivery archive](archive/todo-completed-amsterdam-delivery-2026-11-04.md#amsterdam-presentation-samples-active-studio-priority)
preserves ordinary sample identities, fixtures and verification. Preserve the
accepted Login/Supplier sources, independent copies and credential policy.

## Amsterdam presentation, setup and rehearsal

Completed on explicit owner confirmation, including rehearsal acceptance,
material freeze and archive. The [final handoff](presentation/declarative-amsterdam-2026-handoff.md)
and [completed actions](archive/todo-completed-amsterdam-delivery-2026-11-04.md#amsterdam-presentation-setup-and-rehearsal)
retain publication, slide/kit identities, verification and historical limitations.
Continue with the first unfinished native/release follow-up below.

## Later work: shared foundations, diagnostics and release

All remaining topics below follow the Amsterdam delivery gates. Reuse their
completed native/worker/fallback foundations where the demonstration needs them;
do not make the presentation depend on finishing the reduced runtime, general
SSR pipeline, all optional modules or broad device acceptance. A concrete blocker
may be promoted with a link to its required checkpoint and a bounded regression.

## Native and release follow-ups

Completed follow-up: [Playground native-editing diagnostic and regression repair](archive/todo-completed-playground-native-editing-2026-11-04.md).
The blanket zero-child assertion conflicted with the documented terminal caret
BR. Exact native DOM/source/selection/undo checks now pass with the existing
runtime: all 20 demo-element and ten canonical source-editor stories, lint and
typecheck pass. No runtime, declarations, styles or deadlines changed.

Presentation delivery is complete; these follow-ups can now proceed in order.
Keep original assertions, deadlines and baseline evidence for each repair.

Completed follow-up: [Node output URI binding](archive/todo-completed-node-output-paths-2026-11-04.md).
Shared Rust lowering now binds relative/absolute output paths to runtime URIs
without changing declared paths, config-base policy or authority checks. All
30 focused native checks, 61 Node tests, browser command-service verification,
WASM/CLI builds and lint pass. Refused CSV preserves prior output; repair
publishes. Forged destinations still reject before callbacks and invalid commit
URIs roll back. Frozen presentation delivery bytes remain unchanged.

Completed follow-up: [linked preparation source provenance](archive/todo-completed-linked-preparation-provenance-2026-11-04.md).
The fixture now distinguishes original lexical evidence from the URI-enriched
retained query view, requiring exact original frames/spans and the correct owner
boundary. Existing boundaries retain priority. All 51 linked lifecycle tests,
11 evidence checks and seven retained-provenance checks pass. The three previous
failures were a fixture expectation mismatch; native runtime, evidence-forgery
refusals, context/publication guards and budgets remain unchanged. The original
46/49 baseline remains in the [CSV verification archive](archive/todo-completed-csv-parser-contracts-2026-10-24.md#verification).

Completed follow-up: [optional Studio sample generation](archive/todo-completed-studio-sample-isolation-2026-11-04.md).
The canonical generator emits the optional graph into `generated/schema-assets/`
with correct input/output bases. Nx outputs no longer overlap the strict default
sample directory. Both native-CLI regressions and Studio lint pass: all 65 default
generated files remain unchanged, all 107 sample outputs match between profiles,
and unexpected generated files still fail. Default → optional → default preserves
both deployments. Frozen presentation delivery bytes remain unchanged.

Completed diagnostic capture: [worker allocation phase investigation](archive/todo-completed-worker-allocation-diagnostics-2026-11-04.md).
The timeout did not recur: the broader run's allocation completed in 279 ms and
static refusal passed. Bounded payload-free traces now distinguish readiness,
headroom and group admission; all 48 host/diagnostic units, 23 generation tests,
build and lint pass. At that checkpoint, the broader browser invocation passed
94/104, with ten theme-control count failures repaired in the follow-up below.
Historical timeout attribution remains conditional; no timing repair is claimed.

Completed follow-up: [canonical theme recovery control inventory](archive/todo-completed-theme-control-fixtures-2026-11-05.md).
The fixture now checks six exact choices plus one trigger per switch, accessible
labels/roles and original control/editor identities after handoff. Fresh
Presentation selection preserves independent storage and the edited draft.
All ten targeted theme stories, the final 104-case checkpoint/source-paint/
editor-provider cohort and lint pass. An earlier 103/104 invocation exposed a separate
`worker-method` allocation timeout; its original deadline and failed receipt
remain preserved. Bounded allocation tracing now also covers this driver.
Runtime/declarations, token ownership and frozen presentation bytes are unchanged.

Completed follow-up: [retained CLI schema typecheck inputs](archive/todo-completed-cli-typecheck-inputs-2026-11-05.md).
The CLI build retains its exact native-generated TypeScript source and restores
it with `dist/` from cache, without hashing derived source as authored input.
Standalone runtime typechecking now prepares the CLI dependency automatically.
The real forced typecheck and a 5/5-task cache replay pass after deleting that
source. Full CLI verification passes all 61 Node tests, Chromium acceptance,
cold/cache/version checks and strict publication types. Its in-memory consumer
compile preserves all 373 packaged runtime files; lint passes with its existing
warning. Native schema ownership, runtime/UI/recovery policy and frozen materials
remain unchanged.

Completed follow-up: [Studio release-verifier scheduling](archive/todo-completed-release-scheduling-2026-11-05.md).
Nx now runs protected Studio browser tasks and deployment writers alone within
one invocation while preserving unrelated concurrency. The real scheduler
regression rejects the old overlap and passes for 50 protected workloads at
parallelism four, requiring unrelated probes to overlap. At parallelism three,
source/deployed task intervals do not overlap: all five cold-source-paint cases
and deployed Login/Supplier workflows pass. Source tests pass 243/245, with both
complete live Login cases refusing checkpoint capture at the existing node cap;
both also fail unchanged in isolation. This is not a green full release. Original
30-second/five-second bounds, privacy assertions and 4 MiB budget remain intact;
historical overlapping 276/283 and isolated rerun evidence stays in the archive.
Independent Nx processes/raw commands are outside this scheduling boundary.

- [x] Add a real Nx scheduler regression using resolved Studio browser/writer
      metadata: reject protected-workload overlap at parallelism four while
      requiring unrelated build probes to overlap. Completed with the
      [scheduling repair](archive/todo-completed-release-scheduling-2026-11-05.md).

Completed follow-up: [Login checkpoint DOM headroom](archive/todo-completed-login-checkpoint-headroom-2026-11-05.md).
The navigation view now retains only consumed tree fields and stable route keys;
complete descriptors remain in the service for selection, listings and export.
The failing-first navigation retention regression and worker/fallback live Login
acceptance pass, retaining two privacy refusals, three recoveries and twelve edits
each. The complete source suite passes all 368 tests across nine files; lint,
typecheck and 336-file production determinism also pass. Existing witness coverage,
32,768-node/depth-64 caps, 4 MiB budget and original deadlines remain unchanged.
The earlier 243/245 and isolated 2/2 failures are preserved in the archives.
Frozen presentation delivery bytes remain unchanged.

- [x] Add bounded Login recovery phase node counts and a navigation view-retention
      regression preserving full service descriptors, public routes and retained
      editor/preview identity. See the
      [completed repair](archive/todo-completed-login-checkpoint-headroom-2026-11-05.md).

Completed follow-up: [native conversion fixture intent](archive/todo-completed-native-conversion-fixture-2026-11-05.md).
The `.busy` expression was already retained through a native owning value node;
the fixture incorrectly required a literal compatibility value. It now requires
the exact owning expression, original body/source span and braced normalized
event, while quoted CEM/XML/HTML values remain literal through canonical round
trips. All six conversion cases pass in both native feature profiles. The eight
related integration suites then passed 36/37 per profile; the legacy-binary
fixture follow-up below closes that separate baseline. Parser/runtime/codec, example bytes,
recovery contracts and frozen presentation delivery bytes remain unchanged.

- [x] Strengthen the cross-surface fixture's native ownership, exact `.busy`
      source/event evidence and quoted CEM/XML/HTML literal round trips, and
      reconcile conversion documentation with the native-slot contract. See the
      [completed correction](archive/todo-completed-native-conversion-fixture-2026-11-05.md).

Completed follow-up: [stripped WASM size investigation](archive/todo-completed-stripped-wasm-size-2026-11-05.md).
Fresh matched artifacts pass the unchanged strict artifact verifier: browser and
Node WASM are 24,544,191 stripped versus 24,860,843 default bytes, saving 316,652
bytes per target. Native CLI saves 18,290,384 bytes. Cargo features, linked named
functions, four omitted debugger exports, capability refusal, native symbols and
negative Rust imports independently verify omission while core controls remain.
The initial installed pair had mismatched command declarations and only default's
current optimization report. Historical 27,264,508/27,033,499-byte evidence remains
unattributed; no source/runtime/optimizer change or weaker assertion is claimed.
The [measurement receipt](stripped-wasm-profile-measurements.json) records hashes,
producer freshness and the tested source cohort. Its historical native-test
failure is resolved by the compatibility follow-up below; frozen delivery bytes remain unchanged.

Completed follow-up: [genuine legacy CEMB fixtures](archive/todo-completed-legacy-cemb-fixtures-2026-11-05.md).
Eight frozen binaries come from unchanged historical v2/v3/v4 native encoders,
with source/producer hashes and exact original field/source observations. Legacy
literals retain their values, ID bindings and empty native slots; references
retain pending/empty targets, duplicates, self/cyclic edges and original frames.
The stale modern-header relabeling positives are replaced by genuine bytes;
intentional malformed-version, integrity, structural-truncation and count-limit
refusals remain strict. All 58 cases across ten related suites pass in each native
feature profile. Decoder/runtime code, wire formats, budgets, published runtimes
and frozen delivery bytes remain unchanged. Full release-graph verification is
separate from this native cohort.

- [x] Generate a historical native v2/v3/v4 CEMB compatibility corpus, pin exact
      source/producer/hash evidence and replace relabeled-modern positive fixtures;
      retain complete source/graph and malformed-body refusal checks. See the
      [fixture documentation](../packages/cem_ml/tests/fixtures/cemb-legacy/README.md).


### Historical presentation-adjacent diagnostics

- [ ] Conditional allocation follow-up: if `worker-static-change`, `worker-method`
      or another rendering fresh allocation times out again,
      retain the new per-worker/group phase trace and identify the stalled
      boundary before changing runtime behavior. The historical frozen 110/111
      result and the later `worker-method` 103/104 invocation remain unattributed.
      The isolated cases and final 104/104 invocation pass; a passing repeat is
      not a timing repair. Both media and text-storage/popup drivers now carry
      bounded phase capture. Preserve the original five-second deadline, complete-group
      revocation and static-preview refusal assertions. See the
      [diagnostic capture](archive/todo-completed-worker-allocation-diagnostics-2026-11-04.md)
      and [theme fixture verification](archive/todo-completed-theme-control-fixtures-2026-11-05.md#verification-and-allocation-follow-up).

- [x] Diagnose and repair the held-worker DOM stylesheet readiness timeout.
      The fixture held the host's later artifact-reacquisition preflight after
      releasing initial adoption. Release now opens the gate, with exact replay
      input assertions and bounded, payload-free failure capture. All original
      readiness/hydration/reconnect/style/diagnostic checks and the 30-second
      deadline remain. See the [diagnosis and evidence](archive/todo-completed-held-css-worker-timeout-2026-11-05.md).

- [x] Verify disposal readiness before the held CSS reply, then drain that late
      native reply, require worker termination and retain empty render/style
      assertions. Final sequential stylesheet cohort passes **44/44** across six
      files; lint passes with zero errors and the existing 818 warnings. This
      changes fixture code only; production runtime and frozen delivery remain
      unchanged. The conditional fresh-allocation follow-up stays separate.

- [x] Audit and correct the five Studio project schema-definition errors.
      `id` now uses a string with its unchanged stable-ID regex; both path
      attributes retain `cemml:path` and the reserved-directory facet without
      invalid `/` and `../` prefix declarations. Native regressions compile the
      schema without diagnostics and preserve ID/path refusals and projections.
      All **18/18** native cases pass in both feature profiles; the package gate
      passes. See the [audit and evidence](archive/todo-completed-studio-strict-schema-audit-2026-11-05.md).

- [x] Fix the schema metadata ARIA warning using original-owner expanded
      namespaces. Metadata retains its vocabulary semantics; embedded HTML, SVG
      and MathML keep ARIA checks, and foreign/missing captures grant no exemption.
      Studio schema strict CLI validation now exits **0** with zero diagnostics;
      an invalid HTML role still exits **1** with its exact ARIA warning.
      All five new native cases pass in both feature profiles. See the
      [correction and evidence](archive/todo-completed-schema-metadata-aria-2026-11-05.md).

- [x] Add Node/browser WASM command-service regressions for the actual Studio
      schema and invalid HTML role control, then rebuild and verify the package.
      Both cases fail against the original runtime; the rebuilt full runtime
      suite passes **30/30**, complete XSLT checks pass, and package ABI/types,
      658 integrity records and 32 schema packages verify. Studio's **18/18**
      native project cases and package structure gate also pass. The broader
      **342/343** native validation result remains separate below.

- [x] Audit and correct the Markdown events-projection fixture's stale JSON
      `byteRange` assertion. Both typed CEM projections now require every native
      event's exact source URI/range, order, kind, tag and text, plus original
      hash-bound writer provenance. Unicode/CRLF, nested markup and inert CEM-like
      text are covered; original package admission, unsafe-source refusal and
      coloring/owner checks remain. The full native validation cohort passes
      **343/343**, and all **16/16** Markdown cases pass without default features.
      Production parser/projection/provenance code and frozen delivery remain
      unchanged. See the [fixture audit and evidence](archive/todo-completed-markdown-projection-fixture-2026-11-05.md).


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

- [x] Audit the two historical suggestions-controller cases with unchanged
      assertions/deadlines: both pass in the fresh focused run, and the unchanged
      controller suite passes 15/15. A controlled late-initial-preparation case
      reproduces a fixture ownership race: replacement controllers both use query
      revision `1`, allowing an old preparation to replace the admitted row map.
      Monotonic fixture preparation tokens now preserve the admitted binding
      while keeping native query revisions unchanged. Added bounded, payload-free
      failure capture and verified it with a temporary missing-map control.
      Final related browser cohort passes **38/38**; lint has zero errors and the
      existing 818 warnings. See the
      [audit and evidence](archive/todo-completed-suggestions-controller-audit-2026-11-05.md).

- [x] Add a controlled late-initial-preparation regression across trusted
      controller replacement. The native revision-`1` collision fails before the
      fixture correction, then preserves the exact admitted row map and opens
      normally for both production field providers in worker/fallback modes.

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

- [x] Add a repeatable Chromium accessibility-tree probe for both pinned ARIA
      profiles through actual worker/fallback native export. Compare single,
      ordered/repeated and reversed targets to literal HTML controls; retain DOM
      sequences and observe invalid-state relation exposure without inferring
      spoken announcements or OS accessibility mappings from CDP evidence.
      Provide persistent fixture URLs for actual manual browser/AT runs and
      verify that serve mode without recording unperformed AT acceptance.
      Completed 2026-11-05: six worker/fallback export cases and all twelve
      invalid-state AX comparisons pass in Chromium 148.0.7778.96 on Ubuntu
      26.04/WSL2. Native DOM retains all ordered/repeated targets; CDP matches
      literal HTML, exposing error-message targets only while invalid. Manual
      serve mode advertises twelve URLs; two complementary pages pass browser
      smoke checks without overwriting acceptance. Lint passes with zero errors
      and the existing 818 warnings. Frozen delivery remains unchanged.
      [Audit](archive/todo-completed-aria-browser-relations-2026-11-05.md).

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

- [x] Record the captured public search `ERR_QUIC_PROTOCOL_ERROR` and
      `ERR_NETWORK_CHANGED` failures on `c0833c4c…`, then verify the complete
      publication using an explicit HTTP/2 browser transport profile. Keep the
      default transport, all behavior assertions and deadlines unchanged; record
      the profile in the acceptance receipt and retain both default failures.
      Completed 2026-11-04: full public target passes under the explicit HTTP/2
      profile, with normal public TLS validation and all 1,206 hashes and browser
      suites. This establishes that profile's acceptance, not default transport
      reliability. [Readback](declarative-amsterdam-2026/site-public-c0833c4c-readback.json).

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

- [x] Add bounded, monotonic Studio startup/network/dedicated-worker capture
      to public application verification; preserve the 30-second readiness limit,
      transport defaults, TLS validation and full navigation/offline assertions.
      Verify capture on a controlled delayed response and compare fresh public
      default/HTTP2 startups against frozen publication identities.
      Completed 2026-11-05: 26/26 host/capture tests, including a real unchanged
      30-second held-worker response; Site lint passes. Both fresh full application
      workflows pass with normal public TLS and five accepted startup-file pins;
      frozen presentation bytes unchanged. No runtime or deployment change.
      [Archive](archive/todo-completed-studio-startup-capture-2026-11-05.md).

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

- [x] Audit the unchanged data-table/set-URL journeys and native pointer-tap
      case, then add bounded, passive phase/failure capture that survives the
      original story timeout. Verify it with a controlled failure and run the
      related cohort under load; keep all assertions and deadlines unchanged,
      and retain historical attribution separately if no failure recurs.
      Completed 2026-11-05: 3/3 capture units; controlled original runner timeout
      retains the last 32 records, discarded count and live fixture state. The
      control is removed; original assertion/readiness/native-action calls are
      unchanged. Final 35/35 browser plays, both lint targets (zero errors), and
      strict declarative verification pass. Frozen delivery remains unchanged.
      [Archive](archive/todo-completed-placement-journey-audit-2026-11-05.md).

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

- [x] SSR Phase A routing increment: add normalized `{page}` bindings to the
      shared native graph lowerer for `.html`, `.cem` and compound `.cem.md`
      filenames. Strip exactly one registered suffix, preserve legacy bindings
      and explicit source/content-type identities, and retain unknown-binding
      and duplicate-output refusals. Add failing native/provider and actual CLI
      fan-out/collision fixtures before implementation, plus Node/WASM service
      and executable regressions against the old packaged runtime, and a browser
      worker graph/publication fixture; do not claim SSR rendering
      or CEM-enabled Markdown admission from this routing-only increment.
      Completed 2026-11-05: native failing-first vectors/provider/preflight tests
      pass in both requested feature profiles; 19 graph-request, 34 CLI config,
      eight template-conformance and two new executable tests pass. Rebuilt WASM
      passes 30 runtime tests plus native XSLT contracts; Node passes 63/63 and
      browser workers commit all three exact routed exports. Both lint targets
      pass. Legacy bindings, refusal codes, budgets and frozen delivery remain.
      [Archive](archive/todo-completed-ssr-page-binding-2026-11-05.md).

- [x] SSR Phase A route preflight: retain distinct source URI, deployed page URL,
      effective document base and output URI. Validate page URLs and duplicate
      normalized routes before execution; keep output paths out of render identity
      and preserve explicit resolver/read-map authority and source-map ownership.
      Add failing native HTML-owner/base-selection, URL normalization/refusal,
      batch-collision and render-identity fixtures before implementation; compare
      static base URL results with Chromium without executing authored scripts.
      Cover resolver denial/read-map ownership, mapped/generated base provenance,
      URL identity changes and explicit refusal of unsupported HTML page profiles.
      Completed 2026-11-05: 14 native cases pass in default and actual stripped
      profiles; 16 HTML-owner, 19 graph and three XML Base regressions pass.
      Chromium 148 and Chrome 155 each pass 16 static URL observations, including
      explicit invalid-base refusal and a CSP-blocking control. WASM and lint
      checks pass; resolver/source-map ownership and frozen delivery remain.
      The route contract does not add an SSR command, artifact or hydration claim.
      [Archive](archive/todo-completed-ssr-route-preflight-2026-11-05.md).

- [x] SSR artifact contract: retain the existing native DOM projection owner in
      a typed SSR envelope with explicit lifecycle/adoption identity and bounded
      policy. Add native artifact/report/cache fixtures before wiring the shared
      page lifecycle; unrendered or partial output must not claim hydration readiness.
      Add failing native owner-retention/route-binding, bounded-policy/usage,
      partial/adoption refusal, explicit report and cache-input identity fixtures.
      Check provenance, native extension handoff and sensitivity to source,
      dependency/policy/version changes while excluding physical output paths.
      Completed 2026-11-05: nine failing-first cases expanded to 14 native tests,
      passing in default and actual stripped profiles. Fourteen route, five
      dependency-identity and 32 native artifact/exporter regressions pass; WASM,
      Clippy and focused formatting/lint gates pass. Native owners/provenance and
      frozen delivery remain; only unrendered/partial states are constructible.
      The command, coordinator, wire codec and settled adoption proof remain open.
      [Archive](archive/todo-completed-ssr-artifact-contract-2026-11-05.md).

- [x] SSR shared lifecycle preparation: prepare one inline declaration and one
      active autonomous DCE from the retained route/artifact contracts. Add
      failing native owner-retention, decoded-source/provenance, inert/foreign,
      ambiguity, unsupported/resume, cancellation and bound fixtures before
      implementation. Add browser text vectors, character-reference versus literal
      line-ending cases, unsupported entity decode refusals, customized built-in
      refusals and tokenizer/compiler diagnostic origin regressions. Reuse native
      CEM trees and the existing RetainedTemplate compiler owner; keep the page
      unchanged and return non-adoptable Partial(Incomplete).
      Completed 2026-11-05: 13 preparation tests pass in default and actual
      stripped profiles; 24 common-import, 33 SSR-contract, 28 renderer and five
      declaration-artifact cases pass (cohorts overlap). Five Chromium template
      text vectors, Clippy, focused lint and existing WASM template/CSS consumers
      pass. The optimized default core is 24,234,163 bytes under the unchanged
      25 MiB limit, with matching public import/export descriptors. Original
      owners/control/provenance and frozen delivery remain. No island capture,
      rendered output or adoption proof is claimed.
      [Archive](archive/todo-completed-ssr-inline-preparation-2026-11-05.md).

- [x] SSR first-connection island preparation for the admitted HTML profile:
      capture the prepared instance's original author payload and attributes into the canonical native data-island
      lifecycle/domain owners. Extract reusable pieces from existing native island
      admission where needed; do not enable the whole experimental component
      profile or invent a parallel island/data model. Add failing native topology,
      slot/order, ownership/source-map, cancellation and marked-island resume
      fixtures before implementation. Keep preparation non-adoptable.
      Add explicit failing fixtures for native canonical topology and original
      payload/attribute owners, live versus explicit inert capture, empty/mixed
      payloads, slot order, DOMStringMap keys, unsupported CEM/foreign/CSS inputs,
      sensitive attributes, bounds, cancellation/completed scopes, source/page
      provenance separation and output-independent instance identity. Compare
      supported capture shapes with browser DOM without authored script execution.
      Completed 2026-11-05: 12 failing-first cases expanded to 15 capture tests,
      passing in default and actual stripped profiles. Thirteen shared island
      tests and 13 earlier preparation cases pass in both profiles; the isolated
      component receiver passes 17 cases. Common import, provenance and renderer
      regressions pass, and Chromium matches ten passive capture observations.
      Clippy, focused lint and existing WASM template/CSS consumers pass; the
      optimized default core is 24,179,963 bytes under the unchanged 25 MiB limit,
      with matching public import/export descriptors. Original native owners,
      control, source provenance and frozen delivery remain. No page AST apply,
      declaration evaluation or adoption claim is made.
      [Archive](archive/todo-completed-ssr-island-preparation-2026-11-05.md).

- [x] SSR retained rendering and AST application (one passive HTML instance):
      use the prepared shared native island bindings and retained declaration owner to generate the existing
      render plan and apply it outside the canonical island using common
      namespace/ownership rules. Add native
      source-map, rendered-boundary and original-owner preservation fixtures;
      compare the single-inline-DCE result with the browser lifecycle.
      Add failing native generic AST transaction and single-DCE fixtures before
      implementation: exact source/IR owners, decoded untouched content, inert
      island/range order, text/attribute/slot projection, namespaces and source
      maps, empty output, sensitive/active output and host-update refusals, cancellation and bounded
      work. Compare supported output and payload topology with the browser runtime
      using explicit observation fixtures; keep partial output non-adoptable.
      Completed 2026-11-06: four generic transaction tests pass in default and
      actual stripped profiles; 17 native application cases pass in both. Earlier
      capture/preparation, artifact and renderer regressions pass. Seven browser
      observations match all payloads and six rendered cases; one explicitly
      records the existing browser slot-record/native-node ingress difference.
      Applied native ownership, maps, namespaces and memory-permit lifetimes are
      retained, with one render pass and `Partial(Incomplete)` only. Host writes,
      broader domain admission, settling, HTML export and adoption remain below.
      Clippy, focused lint and optimized WASM consumers pass. The default core is
      24,220,012 bytes under the unchanged 25 MiB limit, with matching named
      import/export descriptors. Frozen presentation checksum remains unchanged.
      [Archive](archive/todo-completed-ssr-render-application-2026-11-06.md).

- [x] SSR shared host/island commit: apply generated host defaults and selected
      attribute writes in one native transaction with canonical attributes and
      dataset updates. Add failing shared transaction fixtures before extending
      `cem.ssr.host_updates_unavailable`; preserve source authority and rollback.
      Add failing native host-commit fixtures for missing defaults versus supplied
      empty values, selected/repeated writes, same-pass dataset reads, ordered
      attributes, source maps, payload reference identity, atomic attribute/child
      replacement, sensitive/reserved/active names, bounds and original control.
      Extend explicit browser observations for supported host/island commits;
      keep non-adoptable state and expose existing browser divergences. Add a
      failing native pure-cache fixture retaining host-write source maps and
      rejecting older host-write cache records without that provenance.
      Completed 2026-11-06: 11 host-commit cases and seven generic AST transaction
      tests pass in default and actual stripped profiles; four native output-cache
      cases pass. Existing module defaults (6), template rendering (55), capture,
      preparation and isolated component receiver (17) pass. Source-owned mapped
      writes, canonical attributes/dataset and children publish atomically in one
      pass; original payload references and partial/non-adoptable status remain.
      Browser snapshot and slot ingress gaps remain explicitly tracked below.
      Focused lint, both native Clippy targets and optimized WASM consumers pass;
      default core is 24,209,768 bytes under the unchanged 25 MiB budget, with
      matching named descriptors. All 12 browser observation contracts pass with
      the five documented bridge differences. Frozen delivery is unchanged.
      [Archive](archive/todo-completed-ssr-host-island-commit-2026-11-06.md).

- [ ] Browser host/island commit bridge: stage renderer host writes and canonical
      attribute/dataset updates through the shared native transaction before
      browser publication; retain processing revision and rollback guards without
      scheduling recursive rendering. Extend failing native/browser commit
      fixtures; current browser snapshots can retain pre-write attributes.

- [ ] SSR browser retained-slot parity: replace the current browser slot metadata
      ingress with shared native retained-node input before claiming direct slot
      insertion parity or hydration. Add failing native/browser boundary fixtures;
      preserve the intentional refusal of ordinary record insertion.

- [ ] SSR additional source/domain admission: support CEM-ML source payloads
      through the shared native import boundary, foreign payload domains and
      instance CSS through the existing native closure/stylesheet owners. Keep
      explicit source authority, envelope identity and source maps; never relabel
      unsupported payloads as HTML. Add failing native adapter/admission fixtures
      before extending the current passive HTML profile.

- [ ] SSR coordinator and completion: admit scoped/resource closures and multiple
      participants, settle nested work under the original operation/budget policy,
      then validate declaration versions and prove complete islands, fresh
      identity/policy and valid output ranges.
      Only that native proof may enable adoption. Wire command lowering, exports
      and browser hydration/parity through the same core after those gates pass.


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
